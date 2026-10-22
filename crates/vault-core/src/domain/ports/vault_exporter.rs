use serde::{Deserialize, Serialize};

use zeroize::Zeroizing;

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

    fn suggested_file_extension(&self) -> &'static str;

    /// Puede ir en texto plano: se devuelve en un buffer que se borra al soltarse.
    fn serialize_vault(
        &self,
        vault_contents: &VaultContents,
        file_password: Option<&SecretText>,
    ) -> VaultResult<Zeroizing<Vec<u8>>>;
}
