use std::path::Path;

use serde::Serialize;
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::application::vault_dependencies::VaultDependencies;
use crate::application::vault_session::UnlockedVault;
use crate::domain::encryption_key::EncryptionKey;
use crate::domain::key_hierarchy::derive_single_attachment_key;
use crate::domain::vault_contents::VaultContents;
use crate::domain::vault_error::{VaultError, VaultResult};
use crate::domain::vault_file::vault_file_format::{decode_standalone_sealed_box, encode_standalone_sealed_box};
use crate::domain::vault_item::{AttachmentReference, VaultItem};

pub struct DecryptedAttachment {
    pub file_name: String,
    pub file_bytes: Zeroizing<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentSummary {
    pub attachment_id: Uuid,
    pub file_name: String,
    pub size_bytes: u64,
}

pub fn add_attachment(
    vault_dependencies: &VaultDependencies,
    unlocked_vault: &mut UnlockedVault,
    item_id: Uuid,
    raw_file_name: &str,
    file_bytes: &[u8],
) -> VaultResult<AttachmentReference> {
    let size_bytes = file_bytes.len() as u64;
    if size_bytes > vault_dependencies.maximum_attachment_size_bytes {
        return Err(VaultError::AttachmentTooLarge {
            maximum_size_bytes: vault_dependencies.maximum_attachment_size_bytes,
        });
    }
    unlocked_vault.vault_contents.find_item(item_id)?;

    let attachment_reference = AttachmentReference {
        attachment_id: Uuid::new_v4(),
        file_name: sanitize_file_name(raw_file_name)?,
        size_bytes,
        added_at: vault_dependencies.clock.now(),
    };
    seal_and_store_attachment(
        vault_dependencies,
        &unlocked_vault.vault_subkeys.attachments_root_key,
        attachment_reference.attachment_id,
        file_bytes,
    )?;

    let persist_result = unlocked_vault.modify_and_persist(vault_dependencies, |vault_contents| {
        vault_contents.find_item_mut(item_id)?.attachments.push(attachment_reference.clone());
        Ok(())
    });
    if persist_result.is_err() {
        let _ = vault_dependencies.attachment_storage.delete_encrypted_attachment(attachment_reference.attachment_id);
    }
    persist_result.map(|()| attachment_reference)
}

pub fn read_attachment(
    vault_dependencies: &VaultDependencies,
    unlocked_vault: &UnlockedVault,
    item_id: Uuid,
    attachment_id: Uuid,
) -> VaultResult<DecryptedAttachment> {
    let attachment_reference = find_attachment_reference(unlocked_vault.vault_contents.find_item(item_id)?, attachment_id)?;
    let file_bytes =
        load_and_open_attachment(vault_dependencies, &unlocked_vault.vault_subkeys.attachments_root_key, attachment_id)?;
    Ok(DecryptedAttachment { file_name: attachment_reference.file_name.clone(), file_bytes })
}

/// Resultado de volver a cifrar los adjuntos con una clave nueva: cada adjunto se copia
/// con un identificador nuevo, así los originales siguen intactos hasta que la bóveda
/// nueva esté guardada (un corte a mitad no pierde nada).
pub struct AttachmentKeyRotation {
    previous_attachment_ids: Vec<Uuid>,
    reencrypted_attachment_ids: Vec<Uuid>,
}

impl AttachmentKeyRotation {
    /// La bóveda nueva ya está en disco: los originales (clave antigua) sobran.
    pub fn discard_previous_copies(&self, vault_dependencies: &VaultDependencies) {
        delete_attachment_blobs(vault_dependencies, &self.previous_attachment_ids);
    }

    /// No se pudo guardar la bóveda nueva: se descartan las copias recién creadas.
    pub fn discard_reencrypted_copies(&self, vault_dependencies: &VaultDependencies) {
        delete_attachment_blobs(vault_dependencies, &self.reencrypted_attachment_ids);
    }
}

/// Vuelve a cifrar todos los adjuntos (incluidos los de la papelera) con `new_attachments_root_key`
/// y actualiza sus referencias en `vault_contents`.
pub fn reencrypt_attachments_with_new_key(
    vault_dependencies: &VaultDependencies,
    previous_attachments_root_key: &EncryptionKey,
    new_attachments_root_key: &EncryptionKey,
    vault_contents: &mut VaultContents,
) -> VaultResult<AttachmentKeyRotation> {
    let mut attachment_key_rotation =
        AttachmentKeyRotation { previous_attachment_ids: Vec::new(), reencrypted_attachment_ids: Vec::new() };
    let all_attachment_references = vault_contents.vault_items.iter_mut().flat_map(|vault_item| vault_item.attachments.iter_mut());

    for attachment_reference in all_attachment_references {
        let reencryption_result = reencrypt_single_attachment(
            vault_dependencies,
            previous_attachments_root_key,
            new_attachments_root_key,
            attachment_reference.attachment_id,
        );
        let reencrypted_attachment_id = match reencryption_result {
            Ok(reencrypted_attachment_id) => reencrypted_attachment_id,
            Err(reencryption_error) => {
                attachment_key_rotation.discard_reencrypted_copies(vault_dependencies);
                return Err(reencryption_error);
            }
        };
        attachment_key_rotation.previous_attachment_ids.push(attachment_reference.attachment_id);
        attachment_key_rotation.reencrypted_attachment_ids.push(reencrypted_attachment_id);
        attachment_reference.attachment_id = reencrypted_attachment_id;
    }
    Ok(attachment_key_rotation)
}

fn reencrypt_single_attachment(
    vault_dependencies: &VaultDependencies,
    previous_attachments_root_key: &EncryptionKey,
    new_attachments_root_key: &EncryptionKey,
    previous_attachment_id: Uuid,
) -> VaultResult<Uuid> {
    let file_bytes = load_and_open_attachment(vault_dependencies, previous_attachments_root_key, previous_attachment_id)?;
    let reencrypted_attachment_id = Uuid::new_v4();
    seal_and_store_attachment(vault_dependencies, new_attachments_root_key, reencrypted_attachment_id, &file_bytes)?;
    Ok(reencrypted_attachment_id)
}

fn seal_and_store_attachment(
    vault_dependencies: &VaultDependencies,
    attachments_root_key: &EncryptionKey,
    attachment_id: Uuid,
    file_bytes: &[u8],
) -> VaultResult<()> {
    let attachment_key = derive_single_attachment_key(attachments_root_key, attachment_id)?;
    let encrypted_attachment =
        vault_dependencies.authenticated_cipher.seal(&attachment_key, file_bytes, attachment_id.as_bytes())?;
    vault_dependencies
        .attachment_storage
        .write_encrypted_attachment(attachment_id, &encode_standalone_sealed_box(&encrypted_attachment)?)
}

fn load_and_open_attachment(
    vault_dependencies: &VaultDependencies,
    attachments_root_key: &EncryptionKey,
    attachment_id: Uuid,
) -> VaultResult<Zeroizing<Vec<u8>>> {
    let encoded_attachment = vault_dependencies.attachment_storage.read_encrypted_attachment(attachment_id)?;
    let encrypted_attachment = decode_standalone_sealed_box(&encoded_attachment)?;
    let attachment_key = derive_single_attachment_key(attachments_root_key, attachment_id)?;
    vault_dependencies
        .authenticated_cipher
        .open(&attachment_key, &encrypted_attachment, attachment_id.as_bytes())
        .map_err(|_| VaultError::corrupted_file("adjunto dañado"))
}

fn delete_attachment_blobs(vault_dependencies: &VaultDependencies, attachment_ids: &[Uuid]) {
    for attachment_id in attachment_ids {
        let _ = vault_dependencies.attachment_storage.delete_encrypted_attachment(*attachment_id);
    }
}

pub fn delete_attachment(
    vault_dependencies: &VaultDependencies,
    unlocked_vault: &mut UnlockedVault,
    item_id: Uuid,
    attachment_id: Uuid,
) -> VaultResult<()> {
    unlocked_vault.modify_and_persist(vault_dependencies, |vault_contents| {
        let vault_item = vault_contents.find_item_mut(item_id)?;
        find_attachment_reference(vault_item, attachment_id)?;
        vault_item.attachments.retain(|attachment| attachment.attachment_id != attachment_id);
        Ok(())
    })?;
    vault_dependencies.attachment_storage.delete_encrypted_attachment(attachment_id)
}

/// Borra los blobs cifrados de elementos eliminados definitivamente (best-effort).
pub fn delete_attachments_of_items(vault_dependencies: &VaultDependencies, removed_items: &[VaultItem]) {
    let removed_attachment_ids: Vec<Uuid> = removed_items
        .iter()
        .flat_map(|removed_item| &removed_item.attachments)
        .map(|attachment| attachment.attachment_id)
        .collect();
    delete_attachment_blobs(vault_dependencies, &removed_attachment_ids);
}

fn find_attachment_reference(vault_item: &VaultItem, attachment_id: Uuid) -> VaultResult<&AttachmentReference> {
    vault_item
        .attachments
        .iter()
        .find(|attachment| attachment.attachment_id == attachment_id)
        .ok_or(VaultError::AttachmentNotFound { attachment_id })
}

/// Se queda solo con el nombre (sin rutas) para evitar path traversal al guardar. Sin nombre
/// utilizable es un error (el núcleo no inventa uno: tendría que elegir idioma).
fn sanitize_file_name(raw_file_name: &str) -> VaultResult<String> {
    Path::new(raw_file_name)
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .map(str::trim)
        .filter(|file_name| !file_name.is_empty())
        .map(str::to_string)
        .ok_or(VaultError::invalid_input("attachmentNameRequired"))
}
