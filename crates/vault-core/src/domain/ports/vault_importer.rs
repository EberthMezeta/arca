use serde::{Deserialize, Serialize};

use crate::domain::folder::Folder;
use crate::domain::secret_text::SecretText;
use crate::domain::vault_error::VaultResult;
use crate::domain::vault_item::VaultItem;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportFormat {
    BitwardenJson,
    BitwardenCsv,
    BrowserCsv,
    EncryptedVaultExport,
}

/// Elementos y carpetas leídos de un archivo externo, con identificadores nuevos.
#[derive(Debug, Default)]
pub struct ImportedVaultData {
    pub folders: Vec<Folder>,
    pub vault_items: Vec<VaultItem>,
}

pub trait VaultImporter: Send + Sync {
    fn import_format(&self) -> ImportFormat;

    fn parse_file(&self, file_bytes: &[u8], file_password: Option<&SecretText>) -> VaultResult<ImportedVaultData>;
}
