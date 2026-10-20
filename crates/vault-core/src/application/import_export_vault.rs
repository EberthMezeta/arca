use std::collections::HashMap;

use serde::Serialize;
use uuid::Uuid;

use crate::application::vault_dependencies::VaultDependencies;
use crate::domain::folder::Folder;
use crate::domain::ports::vault_exporter::ExportFormat;
use crate::domain::ports::vault_importer::{ImportFormat, ImportedVaultData};
use crate::domain::secret_text::SecretText;
use crate::domain::vault_contents::VaultContents;
use crate::domain::vault_error::{VaultError, VaultResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub imported_item_count: usize,
    pub created_folder_count: usize,
}

pub struct ExportedFile {
    pub file_bytes: Vec<u8>,
    pub suggested_file_extension: &'static str,
}

pub fn parse_import_file(
    vault_dependencies: &VaultDependencies,
    import_format: ImportFormat,
    file_bytes: &[u8],
    file_password: Option<&SecretText>,
) -> VaultResult<ImportedVaultData> {
    vault_dependencies.transfer_format_registry.importer_for(import_format)?.parse_file(file_bytes, file_password)
}

/// Fusiona lo importado: las carpetas con el mismo nombre se reutilizan.
pub fn merge_imported_data(vault_contents: &mut VaultContents, imported_vault_data: ImportedVaultData) -> ImportSummary {
    let mut folder_id_translation: HashMap<Uuid, Uuid> = HashMap::new();
    let mut created_folder_count = 0;

    for imported_folder in imported_vault_data.folders {
        let existing_folder_id =
            vault_contents.find_folder_by_name(&imported_folder.folder_name).map(|existing_folder| existing_folder.folder_id);
        let target_folder_id = existing_folder_id.unwrap_or_else(|| {
            created_folder_count += 1;
            let new_folder = Folder { folder_id: Uuid::new_v4(), folder_name: imported_folder.folder_name.clone() };
            let new_folder_id = new_folder.folder_id;
            vault_contents.folders.push(new_folder);
            new_folder_id
        });
        folder_id_translation.insert(imported_folder.folder_id, target_folder_id);
    }

    let imported_item_count = imported_vault_data.vault_items.len();
    for mut imported_item in imported_vault_data.vault_items {
        imported_item.item_id = Uuid::new_v4();
        imported_item.attachments.clear();
        imported_item.folder_id =
            imported_item.folder_id.and_then(|source_folder_id| folder_id_translation.get(&source_folder_id).copied());
        vault_contents.vault_items.push(imported_item);
    }

    ImportSummary { imported_item_count, created_folder_count }
}

pub fn export_vault(
    vault_dependencies: &VaultDependencies,
    vault_contents: &VaultContents,
    export_format: ExportFormat,
    file_password: Option<&SecretText>,
) -> VaultResult<ExportedFile> {
    let vault_exporter = vault_dependencies.transfer_format_registry.exporter_for(export_format)?;
    let usable_file_password = file_password.filter(|file_password| !file_password.is_empty());
    if vault_exporter.requires_file_password() && usable_file_password.is_none() {
        return Err(VaultError::TransferPasswordRequired);
    }

    let exportable_contents = VaultContents {
        vault_items: vault_contents.active_items().cloned().collect(),
        folders: vault_contents.folders.clone(),
        preferences: vault_contents.preferences.clone(),
    };
    Ok(ExportedFile {
        file_bytes: vault_exporter.serialize_vault(&exportable_contents, usable_file_password)?,
        suggested_file_extension: vault_exporter.suggested_file_extension(),
    })
}
