use std::collections::HashMap;

use chrono::Utc;
use uuid::Uuid;

use crate::domain::folder::Folder;
use crate::domain::ports::vault_exporter::{ExportFormat, VaultExporter};
use crate::domain::ports::vault_importer::{ImportFormat, ImportedVaultData, VaultImporter};
use crate::domain::secret_text::SecretText;
use crate::domain::vault_contents::VaultContents;
use crate::domain::vault_error::{VaultError, VaultResult};
use crate::infrastructure::import_export::bitwarden_json_format::{BitwardenExportDocument, BitwardenFolder, BitwardenItem};

pub struct BitwardenJsonImporter;

impl VaultImporter for BitwardenJsonImporter {
    fn import_format(&self) -> ImportFormat {
        ImportFormat::BitwardenJson
    }

    fn parse_file(&self, file_bytes: &[u8], _file_password: Option<&SecretText>) -> VaultResult<ImportedVaultData> {
        let export_document: BitwardenExportDocument =
            serde_json::from_slice(file_bytes).map_err(VaultError::import_failed)?;
        if export_document.encrypted {
            return Err(VaultError::import_failed(
                "los exports cifrados de Bitwarden no se pueden leer; exporta en JSON sin cifrar",
            ));
        }

        let folder_id_by_source_id: HashMap<&str, Uuid> = export_document
            .folders
            .iter()
            .map(|bitwarden_folder| (bitwarden_folder.id.as_str(), Uuid::new_v4()))
            .collect();
        let folders = export_document
            .folders
            .iter()
            .filter_map(|bitwarden_folder| {
                folder_id_by_source_id.get(bitwarden_folder.id.as_str()).map(|folder_id| Folder {
                    folder_id: *folder_id,
                    folder_name: bitwarden_folder.name.clone(),
                })
            })
            .collect();

        let now = Utc::now();
        let vault_items = export_document
            .items
            .iter()
            .filter_map(|bitwarden_item| {
                let folder_id = bitwarden_item
                    .folder_id
                    .as_deref()
                    .and_then(|source_folder_id| folder_id_by_source_id.get(source_folder_id).copied());
                bitwarden_item.to_vault_item(folder_id, now)
            })
            .collect();
        Ok(ImportedVaultData { folders, vault_items })
    }
}

/// Export en claro compatible con Bitwarden. La UI debe advertir antes de usarlo.
pub struct BitwardenJsonExporter;

impl VaultExporter for BitwardenJsonExporter {
    fn export_format(&self) -> ExportFormat {
        ExportFormat::BitwardenJson
    }

    fn requires_file_password(&self) -> bool {
        false
    }

    fn suggested_file_extension(&self) -> &'static str {
        "json"
    }

    fn serialize_vault(&self, vault_contents: &VaultContents, _file_password: Option<&SecretText>) -> VaultResult<Vec<u8>> {
        let export_document = BitwardenExportDocument {
            encrypted: false,
            folders: vault_contents
                .folders
                .iter()
                .map(|folder| BitwardenFolder { id: folder.folder_id.to_string(), name: folder.folder_name.clone() })
                .collect(),
            items: vault_contents.vault_items.iter().map(BitwardenItem::from).collect(),
        };
        serde_json::to_vec_pretty(&export_document).map_err(VaultError::storage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::vault_preferences::VaultPreferences;
    use crate::domain::item_content::ItemContent;

    const BITWARDEN_SAMPLE_EXPORT: &str = r#"{
      "encrypted": false,
      "folders": [{ "id": "f-1", "name": "Trabajo" }],
      "items": [
        { "id": "i-1", "folderId": "f-1", "type": 1, "name": "GitHub", "notes": null, "favorite": true,
          "fields": [{ "name": "PIN", "value": "1234", "type": 1 }],
          "login": { "uris": [{ "match": null, "uri": "https://github.com" }], "username": "ana", "password": "s3cr3t", "totp": null } },
        { "id": "i-2", "folderId": null, "type": 2, "name": "Nota", "notes": "texto", "favorite": false, "secureNote": { "type": 0 } },
        { "id": "i-3", "folderId": null, "type": 5, "name": "Clave SSH" }
      ]
    }"#;

    #[test]
    fn imports_logins_notes_and_folders_skipping_unknown_types() {
        let imported_vault_data =
            BitwardenJsonImporter.parse_file(BITWARDEN_SAMPLE_EXPORT.as_bytes(), None).expect("importar");

        assert_eq!(imported_vault_data.folders.len(), 1);
        assert_eq!(imported_vault_data.vault_items.len(), 2);
        let github_item = &imported_vault_data.vault_items[0];
        assert_eq!(github_item.folder_id, Some(imported_vault_data.folders[0].folder_id));
        assert!(github_item.is_favorite);
        assert!(github_item.custom_fields[0].is_hidden);
        let ItemContent::Login(login_content) = &github_item.item_content else {
            panic!("se esperaba un login");
        };
        assert_eq!(login_content.password.expose_secret(), "s3cr3t");
        assert_eq!(login_content.website_urls, vec!["https://github.com".to_string()]);
    }

    #[test]
    fn exported_json_can_be_imported_again() {
        let imported_vault_data =
            BitwardenJsonImporter.parse_file(BITWARDEN_SAMPLE_EXPORT.as_bytes(), None).expect("importar");
        let vault_contents = VaultContents {
            vault_items: imported_vault_data.vault_items,
            folders: imported_vault_data.folders,
            preferences: VaultPreferences::default(),
            vault_id: None,
            revision_number: 0,
        };

        let exported_bytes = BitwardenJsonExporter.serialize_vault(&vault_contents, None).expect("exportar");
        let reimported_vault_data = BitwardenJsonImporter.parse_file(&exported_bytes, None).expect("reimportar");

        assert_eq!(reimported_vault_data.vault_items.len(), 2);
        assert_eq!(reimported_vault_data.vault_items[0].item_content, vault_contents.vault_items[0].item_content);
    }

    #[test]
    fn encrypted_bitwarden_export_is_rejected_with_a_clear_reason() {
        let import_result = BitwardenJsonImporter.parse_file(br#"{"encrypted": true}"#, None);

        assert!(matches!(import_result, Err(VaultError::ImportFailed { .. })));
    }
}
