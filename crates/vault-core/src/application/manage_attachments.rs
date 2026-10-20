use std::path::Path;

use serde::Serialize;
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::application::vault_dependencies::VaultDependencies;
use crate::application::vault_session::UnlockedVault;
use crate::domain::key_hierarchy::derive_single_attachment_key;
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
        file_name: sanitize_file_name(raw_file_name),
        size_bytes,
        added_at: vault_dependencies.clock.now(),
    };
    let attachment_key =
        derive_single_attachment_key(&unlocked_vault.vault_subkeys.attachments_root_key, attachment_reference.attachment_id)?;
    let encrypted_attachment = vault_dependencies.authenticated_cipher.seal(
        &attachment_key,
        file_bytes,
        attachment_reference.attachment_id.as_bytes(),
    )?;
    vault_dependencies.attachment_storage.write_encrypted_attachment(
        attachment_reference.attachment_id,
        &encode_standalone_sealed_box(&encrypted_attachment)?,
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
    let encoded_attachment = vault_dependencies.attachment_storage.read_encrypted_attachment(attachment_id)?;
    let encrypted_attachment = decode_standalone_sealed_box(&encoded_attachment)?;
    let attachment_key = derive_single_attachment_key(&unlocked_vault.vault_subkeys.attachments_root_key, attachment_id)?;

    let file_bytes = vault_dependencies
        .authenticated_cipher
        .open(&attachment_key, &encrypted_attachment, attachment_id.as_bytes())
        .map_err(|_| VaultError::corrupted_file("adjunto dañado"))?;
    Ok(DecryptedAttachment { file_name: attachment_reference.file_name.clone(), file_bytes })
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
    removed_items
        .iter()
        .flat_map(|removed_item| &removed_item.attachments)
        .for_each(|attachment| {
            let _ = vault_dependencies.attachment_storage.delete_encrypted_attachment(attachment.attachment_id);
        });
}

fn find_attachment_reference(vault_item: &VaultItem, attachment_id: Uuid) -> VaultResult<&AttachmentReference> {
    vault_item
        .attachments
        .iter()
        .find(|attachment| attachment.attachment_id == attachment_id)
        .ok_or(VaultError::AttachmentNotFound { attachment_id })
}

/// Se queda solo con el nombre (sin rutas) para evitar path traversal al guardar.
fn sanitize_file_name(raw_file_name: &str) -> String {
    Path::new(raw_file_name)
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .map(str::trim)
        .filter(|file_name| !file_name.is_empty())
        .unwrap_or("adjunto")
        .to_string()
}
