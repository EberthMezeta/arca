use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zeroize::Zeroize;

use crate::domain::folder::Folder;
use crate::domain::vault_error::{VaultError, VaultResult};
use crate::domain::vault_item::VaultItem;
use crate::domain::vault_preferences::VaultPreferences;
use crate::domain::vault_revision::VaultRevision;

/// Todo lo que se guarda cifrado dentro del archivo `.vault`.
/// (`default` va por campo y no en el struct: con `Drop` no se puede usar `..Default::default()`.)
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultContents {
    #[serde(default)]
    pub vault_items: Vec<VaultItem>,
    #[serde(default)]
    pub folders: Vec<Folder>,
    #[serde(default)]
    pub preferences: VaultPreferences,
    /// `None` en bóvedas guardadas antes de existir la detección de rollback; se asigna al guardar.
    #[serde(default)]
    pub vault_id: Option<Uuid>,
    /// Sube en cada guardado (ver `vault_revision`).
    #[serde(default)]
    pub revision_number: u64,
}

/// Los secretos (`SecretText`) ya se borran solos; aquí se borra el resto del texto descifrado
/// (títulos, usuarios, URLs, datos personales, nombres de carpetas y adjuntos) al bloquear o al
/// sustituir una copia por otra. Best-effort: las copias que ya salieron hacia la UI no se alcanzan.
impl Drop for VaultContents {
    fn drop(&mut self) {
        self.vault_items.iter_mut().for_each(VaultItem::zeroize_plain_text);
        self.folders.iter_mut().for_each(|folder| folder.folder_name.zeroize());
    }
}

impl VaultContents {
    pub fn revision(&self) -> Option<VaultRevision> {
        self.vault_id.map(|vault_id| VaultRevision { vault_id, revision_number: self.revision_number })
    }

    /// Antes de cada guardado: la nueva versión queda por delante de todas las anteriores.
    pub fn advance_revision(&mut self) -> VaultRevision {
        let vault_id = *self.vault_id.get_or_insert_with(Uuid::new_v4);
        self.revision_number = self.revision_number.saturating_add(1);
        VaultRevision { vault_id, revision_number: self.revision_number }
    }

    pub fn find_item(&self, item_id: Uuid) -> VaultResult<&VaultItem> {
        self.vault_items
            .iter()
            .find(|vault_item| vault_item.item_id == item_id)
            .ok_or(VaultError::ItemNotFound { item_id })
    }

    pub fn find_item_mut(&mut self, item_id: Uuid) -> VaultResult<&mut VaultItem> {
        self.vault_items
            .iter_mut()
            .find(|vault_item| vault_item.item_id == item_id)
            .ok_or(VaultError::ItemNotFound { item_id })
    }

    pub fn remove_item(&mut self, item_id: Uuid) -> VaultResult<VaultItem> {
        let item_position = self
            .vault_items
            .iter()
            .position(|vault_item| vault_item.item_id == item_id)
            .ok_or(VaultError::ItemNotFound { item_id })?;
        Ok(self.vault_items.remove(item_position))
    }

    pub fn ensure_folder_exists(&self, folder_id: Uuid) -> VaultResult<()> {
        self.folders
            .iter()
            .any(|folder| folder.folder_id == folder_id)
            .then_some(())
            .ok_or(VaultError::FolderNotFound { folder_id })
    }

    pub fn find_folder_mut(&mut self, folder_id: Uuid) -> VaultResult<&mut Folder> {
        self.folders
            .iter_mut()
            .find(|folder| folder.folder_id == folder_id)
            .ok_or(VaultError::FolderNotFound { folder_id })
    }

    pub fn find_folder_by_name(&self, folder_name: &str) -> Option<&Folder> {
        self.folders
            .iter()
            .find(|folder| folder.folder_name.eq_ignore_ascii_case(folder_name))
    }

    pub fn active_items(&self) -> impl Iterator<Item = &VaultItem> {
        self.vault_items.iter().filter(|vault_item| !vault_item.is_in_trash())
    }
}
