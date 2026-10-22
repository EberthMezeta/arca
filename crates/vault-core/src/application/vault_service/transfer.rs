//! Importar y exportar.

use std::sync::Arc;


use crate::application::import_export_vault::{self, ExportedFile, ImportLabels, ImportSummary};
use crate::application::transfer_format_registry::TransferFormatRegistry;
use crate::domain::ports::vault_exporter::ExportFormat;
use crate::domain::ports::vault_importer::{ImportFormat, ImportedVaultData};
use crate::domain::secret_text::SecretText;
use crate::domain::vault_error::VaultResult;

use super::VaultService;

pub trait VaultTransfer {
    /// Los formatos de import/export. Se comparten para poder leer un archivo externo sin tener
    /// tomada la bóveda: una copia `.arca` puede exigir un Argon2id largo, y mientras tanto el
    /// bloqueo (pantalla, suspensión, inactividad) tiene que poder entrar.
    fn transfer_format_registry(&self) -> Arc<TransferFormatRegistry>;

    /// Fusiona en la bóveda lo leído con `import_export_vault::parse_import_file`.
    fn merge_imported_data(
        &mut self,
        imported_vault_data: ImportedVaultData,
        import_labels: &ImportLabels,
    ) -> VaultResult<ImportSummary>;

    /// Leer y fusionar en un paso (con la bóveda tomada todo el rato): cómodo en tests.
    fn import_file(
        &mut self,
        import_format: ImportFormat,
        file_bytes: &[u8],
        file_password: Option<&SecretText>,
        import_labels: &ImportLabels,
    ) -> VaultResult<ImportSummary>;

    /// Exportar saca toda la bóveda del cifrado: exige la contraseña maestra (con retardo),
    /// para que alguien con acceso breve a la app desbloqueada no pueda llevársela.
    fn export_vault(
        &mut self,
        master_password: &SecretText,
        export_format: ExportFormat,
        file_password: Option<&SecretText>,
    ) -> VaultResult<ExportedFile>;
}

impl VaultTransfer for VaultService {
    fn transfer_format_registry(&self) -> Arc<TransferFormatRegistry> {
        Arc::clone(&self.vault_dependencies.transfer_format_registry)
    }

    fn merge_imported_data(
        &mut self,
        imported_vault_data: ImportedVaultData,
        import_labels: &ImportLabels,
    ) -> VaultResult<ImportSummary> {
        self.mutate(|vault_contents| Ok(import_export_vault::merge_imported_data(vault_contents, imported_vault_data, import_labels)))
    }

    fn import_file(
        &mut self,
        import_format: ImportFormat,
        file_bytes: &[u8],
        file_password: Option<&SecretText>,
        import_labels: &ImportLabels,
    ) -> VaultResult<ImportSummary> {
        let imported_vault_data = import_export_vault::parse_import_file(
            &self.vault_dependencies.transfer_format_registry,
            import_format,
            file_bytes,
            file_password,
        )?;
        self.merge_imported_data(imported_vault_data, import_labels)
    }

    fn export_vault(
        &mut self,
        master_password: &SecretText,
        export_format: ExportFormat,
        file_password: Option<&SecretText>,
    ) -> VaultResult<ExportedFile> {
        self.reauthenticate(master_password)?;
        let unlocked_vault = self.session_state.require_unlocked()?;
        import_export_vault::export_vault(&self.vault_dependencies, &unlocked_vault.vault_contents, export_format, file_password)
    }
}
