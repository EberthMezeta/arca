//! Lo que hay que saber de un formato de import/export antes de usarlo (pedir contraseña,
//! avisar de que va en claro). Fuente única para el núcleo y para la interfaz.

use crate::domain::ports::vault_exporter::ExportFormat;
use crate::domain::ports::vault_importer::ImportFormat;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferFormatTraits {
    pub requires_file_password: bool,
    pub is_plaintext: bool,
}

const ENCRYPTED_FILE: TransferFormatTraits = TransferFormatTraits { requires_file_password: true, is_plaintext: false };
const PLAINTEXT_FILE: TransferFormatTraits = TransferFormatTraits { requires_file_password: false, is_plaintext: true };

impl ImportFormat {
    pub const fn traits(self) -> TransferFormatTraits {
        match self {
            Self::EncryptedVaultExport => ENCRYPTED_FILE,
            Self::BitwardenJson | Self::BitwardenCsv | Self::BrowserCsv => PLAINTEXT_FILE,
        }
    }
}

impl ExportFormat {
    pub const fn traits(self) -> TransferFormatTraits {
        match self {
            Self::EncryptedVaultExport => ENCRYPTED_FILE,
            Self::BitwardenJson | Self::BitwardenCsv => PLAINTEXT_FILE,
        }
    }
}
