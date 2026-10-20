pub mod bitwarden_json_format;
pub mod bitwarden_json_transfer;
pub mod csv_transfer;
pub mod encrypted_vault_export;

use crate::application::transfer_format_registry::TransferFormatRegistry;

/// Registro con todos los formatos soportados (raíz de composición).
pub fn build_default_transfer_format_registry() -> TransferFormatRegistry {
    TransferFormatRegistry::default()
        .with_importer(Box::new(bitwarden_json_transfer::BitwardenJsonImporter))
        .with_importer(Box::new(csv_transfer::BitwardenCsvImporter))
        .with_importer(Box::new(csv_transfer::BrowserCsvImporter))
        .with_importer(Box::new(encrypted_vault_export::EncryptedVaultExportImporter))
        .with_exporter(Box::new(bitwarden_json_transfer::BitwardenJsonExporter))
        .with_exporter(Box::new(csv_transfer::BitwardenCsvExporter))
        .with_exporter(Box::new(encrypted_vault_export::EncryptedVaultExportExporter))
}
