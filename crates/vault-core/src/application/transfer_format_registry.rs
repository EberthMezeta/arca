use std::collections::HashMap;

use crate::domain::ports::vault_exporter::{ExportFormat, VaultExporter};
use crate::domain::ports::vault_importer::{ImportFormat, VaultImporter};
use crate::domain::vault_error::{VaultError, VaultResult};

/// Patrón Registry: cada formato se registra una vez; no hay `match` por formato en los casos de uso.
#[derive(Default)]
pub struct TransferFormatRegistry {
    importers_by_format: HashMap<ImportFormat, Box<dyn VaultImporter>>,
    exporters_by_format: HashMap<ExportFormat, Box<dyn VaultExporter>>,
}

impl TransferFormatRegistry {
    #[must_use]
    pub fn with_importer(mut self, vault_importer: Box<dyn VaultImporter>) -> Self {
        self.importers_by_format.insert(vault_importer.import_format(), vault_importer);
        self
    }

    #[must_use]
    pub fn with_exporter(mut self, vault_exporter: Box<dyn VaultExporter>) -> Self {
        self.exporters_by_format.insert(vault_exporter.export_format(), vault_exporter);
        self
    }

    pub fn importer_for(&self, import_format: ImportFormat) -> VaultResult<&dyn VaultImporter> {
        self.importers_by_format.get(&import_format).map(AsRef::as_ref).ok_or(VaultError::UnsupportedTransferFormat)
    }

    pub fn exporter_for(&self, export_format: ExportFormat) -> VaultResult<&dyn VaultExporter> {
        self.exporters_by_format.get(&export_format).map(AsRef::as_ref).ok_or(VaultError::UnsupportedTransferFormat)
    }
}
