use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::folder::Folder;
use crate::domain::vault_error::{VaultError, VaultResult};
use crate::domain::vault_item::VaultItem;
use crate::domain::vault_preferences::VaultPreferences;

/// Todo lo que se guarda cifrado dentro del archivo `.vault`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VaultContents {
    pub vault_items: Vec<VaultItem>,
    pub folders: Vec<Folder>,
    pub preferences: VaultPreferences,
}

impl VaultContents {
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
