use std::collections::HashMap;

use serde::Serialize;
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::application::manage_folders::MAXIMUM_FOLDER_NAME_CHARACTER_COUNT;
use crate::application::save_vault_item::MAXIMUM_TITLE_CHARACTER_COUNT;
use crate::application::transfer_format_registry::TransferFormatRegistry;
use crate::application::vault_dependencies::VaultDependencies;
use crate::domain::folder::Folder;
use crate::domain::item_content::ItemContent;
use crate::domain::ports::vault_exporter::ExportFormat;
use crate::domain::ports::vault_importer::{ImportFormat, ImportedVaultData};
use crate::domain::secret_text::SecretText;
use crate::domain::totp_configuration::TotpConfiguration;
use crate::domain::vault_contents::VaultContents;
use crate::domain::vault_error::{VaultError, VaultResult};
use crate::domain::vault_item::{CustomField, VaultItem};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub imported_item_count: usize,
    pub created_folder_count: usize,
}

pub struct ExportedFile {
    /// Puede ir en texto plano (CSV, JSON): se borra al soltarse.
    pub file_bytes: Zeroizing<Vec<u8>>,
    pub suggested_file_extension: &'static str,
}

/// No necesita la bóveda: se puede llamar sin tenerla tomada (ver `VaultService::transfer_format_registry`).
pub fn parse_import_file(
    transfer_format_registry: &TransferFormatRegistry,
    import_format: ImportFormat,
    file_bytes: &[u8],
    file_password: Option<&SecretText>,
) -> VaultResult<ImportedVaultData> {
    transfer_format_registry.importer_for(import_format)?.parse_file(file_bytes, file_password)
}

const UNTITLED_ITEM_TITLE: &str = "Sin título";
const INVALID_TOTP_FIELD_LABEL: &str = "TOTP (no válido)";

/// Recorta a `maximum_character_count` caracteres (no bytes, para no partir un carácter UTF-8).
fn truncated_to_characters(raw_text: &str, maximum_character_count: usize) -> String {
    raw_text.trim().chars().take(maximum_character_count).collect()
}

/// Lo importado viene de terceros: se le aplican las mismas reglas que al guardar desde el
/// editor, para que luego se pueda editar sin errores. Un TOTP no válido no se pierde: pasa a
/// un campo oculto (si se quedara, el elemento no se podría volver a guardar sin borrarlo).
fn normalize_imported_item(imported_item: &mut VaultItem) {
    imported_item.title = truncated_to_characters(&imported_item.title, MAXIMUM_TITLE_CHARACTER_COUNT);
    if imported_item.title.is_empty() {
        imported_item.title = UNTITLED_ITEM_TITLE.to_string();
    }

    let ItemContent::Login(login_content) = &mut imported_item.item_content else {
        return;
    };
    let invalid_totp_secret = login_content
        .totp_secret
        .take_if(|totp_secret| !totp_secret.is_empty() && TotpConfiguration::parse(totp_secret).is_err());
    let Some(invalid_totp_secret) = invalid_totp_secret else {
        return;
    };
    imported_item.custom_fields.push(CustomField {
        field_label: INVALID_TOTP_FIELD_LABEL.to_string(),
        field_value: invalid_totp_secret,
        is_hidden: true,
    });
}

/// Fusiona lo importado: las carpetas con el mismo nombre se reutilizan; las que no tienen
/// nombre no se crean (sus elementos quedan sin carpeta).
pub fn merge_imported_data(vault_contents: &mut VaultContents, imported_vault_data: ImportedVaultData) -> ImportSummary {
    let mut folder_id_translation: HashMap<Uuid, Uuid> = HashMap::new();
    let mut created_folder_count = 0;

    for imported_folder in imported_vault_data.folders {
        let folder_name = truncated_to_characters(&imported_folder.folder_name, MAXIMUM_FOLDER_NAME_CHARACTER_COUNT);
        if folder_name.is_empty() {
            continue;
        }
        let existing_folder_id = vault_contents.find_folder_by_name(&folder_name).map(|existing_folder| existing_folder.folder_id);
        let target_folder_id = existing_folder_id.unwrap_or_else(|| {
            created_folder_count += 1;
            let new_folder = Folder { folder_id: Uuid::new_v4(), folder_name };
            let new_folder_id = new_folder.folder_id;
            vault_contents.folders.push(new_folder);
            new_folder_id
        });
        folder_id_translation.insert(imported_folder.folder_id, target_folder_id);
    }

    let imported_item_count = imported_vault_data.vault_items.len();
    for mut imported_item in imported_vault_data.vault_items {
        normalize_imported_item(&mut imported_item);
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
        vault_id: None,
        revision_number: 0,
    };
    Ok(ExportedFile {
        file_bytes: vault_exporter.serialize_vault(&exportable_contents, usable_file_password)?,
        suggested_file_extension: vault_exporter.suggested_file_extension(),
    })
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::domain::item_content::LoginContent;

    fn imported_login(title: &str, totp_secret: &str) -> VaultItem {
        let now = Utc::now();
        VaultItem {
            item_id: Uuid::new_v4(),
            title: title.to_string(),
            folder_id: None,
            tag_names: vec![],
            is_favorite: false,
            notes: SecretText::default(),
            custom_fields: vec![],
            item_content: ItemContent::Login(LoginContent {
                totp_secret: Some(SecretText::new(totp_secret)),
                ..LoginContent::default()
            }),
            password_history: vec![],
            attachments: vec![],
            created_at: now,
            updated_at: now,
            password_changed_at: None,
            moved_to_trash_at: None,
        }
    }

    #[test]
    fn imported_items_follow_the_editor_rules() {
        let mut vault_contents = VaultContents::default();
        let imported_vault_data = ImportedVaultData {
            folders: vec![Folder { folder_id: Uuid::new_v4(), folder_name: "   ".into() }],
            vault_items: vec![imported_login("   ", "¡no es base32!"), imported_login(&"x".repeat(500), "JBSWY3DPEHPK3PXP")],
        };

        let import_summary = merge_imported_data(&mut vault_contents, imported_vault_data);

        assert_eq!(import_summary.created_folder_count, 0);
        let untitled_item = &vault_contents.vault_items[0];
        assert_eq!(untitled_item.title, UNTITLED_ITEM_TITLE);
        assert_eq!(untitled_item.item_content.totp_secret(), None);
        assert_eq!(untitled_item.custom_fields[0].field_value.expose_secret(), "¡no es base32!");
        assert!(untitled_item.custom_fields[0].is_hidden);
        let long_title_item = &vault_contents.vault_items[1];
        assert_eq!(long_title_item.title.chars().count(), MAXIMUM_TITLE_CHARACTER_COUNT);
        assert!(long_title_item.item_content.totp_secret().is_some());
    }
}
