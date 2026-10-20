use serde::{Deserialize, Serialize};

use crate::domain::secret_text::SecretText;
use crate::domain::vault_contents::VaultContents;
use crate::domain::vault_error::VaultResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportFormat {
    EncryptedVaultExport,
    BitwardenJson,
    BitwardenCsv,
}

pub trait VaultExporter: Send + Sync {
    fn export_format(&self) -> ExportFormat;

    fn requires_file_password(&self) -> bool;

    fn suggested_file_extension(&self) -> &'static str;

    fn serialize_vault(&self, vault_contents: &VaultContents, file_password: Option<&SecretText>) -> VaultResult<Vec<u8>>;
}
