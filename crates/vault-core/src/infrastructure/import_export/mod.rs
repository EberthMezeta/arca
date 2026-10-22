pub mod bitwarden_json_format;
pub mod bitwarden_json_transfer;
pub mod csv_transfer;
pub mod encrypted_vault_export;

use crate::application::transfer_format_registry::TransferFormatRegistry;
use crate::infrastructure::import_export::encrypted_vault_export::ExportCryptography;

/// Registro con todos los formatos soportados (raíz de composición).
pub fn build_default_transfer_format_registry() -> TransferFormatRegistry {
    build_transfer_format_registry(&ExportCryptography::default())
}

/// Igual, con la criptografía de la copia cifrada inyectada (p. ej. la política de la app o
/// un Argon2id barato en tests).
pub fn build_transfer_format_registry(export_cryptography: &ExportCryptography) -> TransferFormatRegistry {
    TransferFormatRegistry::default()
        .with_importer(Box::new(bitwarden_json_transfer::BitwardenJsonImporter))
        .with_importer(Box::new(csv_transfer::BitwardenCsvImporter))
        .with_importer(Box::new(csv_transfer::BrowserCsvImporter))
        .with_importer(Box::new(encrypted_vault_export::EncryptedVaultExportImporter {
            export_cryptography: export_cryptography.clone(),
        }))
        .with_exporter(Box::new(bitwarden_json_transfer::BitwardenJsonExporter))
        .with_exporter(Box::new(csv_transfer::BitwardenCsvExporter))
        .with_exporter(Box::new(encrypted_vault_export::EncryptedVaultExportExporter {
            export_cryptography: export_cryptography.clone(),
        }))
}
