use uuid::Uuid;

use crate::domain::folder::Folder;
use crate::domain::vault_contents::VaultContents;
use crate::domain::vault_error::{VaultError, VaultResult};

pub(crate) const MAXIMUM_FOLDER_NAME_CHARACTER_COUNT: usize = 100;

fn validated_folder_name(
    vault_contents: &VaultContents,
    raw_folder_name: &str,
    folder_id_being_renamed: Option<Uuid>,
) -> VaultResult<String> {
    let folder_name = raw_folder_name.trim();
    if folder_name.is_empty() {
        return Err(VaultError::invalid_input("folderNameRequired"));
    }
    if folder_name.chars().count() > MAXIMUM_FOLDER_NAME_CHARACTER_COUNT {
        return Err(VaultError::invalid_input("folderNameTooLong"));
    }
    let conflicting_folder = vault_contents
        .find_folder_by_name(folder_name)
        .filter(|existing_folder| Some(existing_folder.folder_id) != folder_id_being_renamed);
    if conflicting_folder.is_some() {
        return Err(VaultError::invalid_input("folderNameDuplicated"));
    }
    Ok(folder_name.to_string())
}

pub fn create_folder(vault_contents: &mut VaultContents, raw_folder_name: &str) -> VaultResult<Folder> {
    let folder_name = validated_folder_name(vault_contents, raw_folder_name, None)?;
    let new_folder = Folder { folder_id: Uuid::new_v4(), folder_name };
    vault_contents.folders.push(new_folder.clone());
    Ok(new_folder)
}

pub fn rename_folder(
    vault_contents: &mut VaultContents,
    folder_id: Uuid,
    raw_folder_name: &str,
) -> VaultResult<Folder> {
    let folder_name = validated_folder_name(vault_contents, raw_folder_name, Some(folder_id))?;
    let folder = vault_contents.find_folder_mut(folder_id)?;
    folder.folder_name = folder_name;
    Ok(folder.clone())
}

/// Los elementos de la carpeta no se borran: quedan "sin carpeta".
pub fn delete_folder(vault_contents: &mut VaultContents, folder_id: Uuid) -> VaultResult<()> {
    vault_contents.ensure_folder_exists(folder_id)?;
    vault_contents.folders.retain(|folder| folder.folder_id != folder_id);
    vault_contents
        .vault_items
        .iter_mut()
        .filter(|vault_item| vault_item.folder_id == Some(folder_id))
        .for_each(|vault_item| vault_item.folder_id = None);
    Ok(())
}
