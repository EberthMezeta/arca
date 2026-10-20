use thiserror::Error;
use uuid::Uuid;

use crate::domain::master_password_policy::MasterPasswordViolation;

/// Errores de negocio. Los mensajes son para logs; la UI traduce por `error_code()`.
#[derive(Debug, Error)]
pub enum VaultError {
    #[error("ya existe una bóveda en esta ubicación")]
    VaultAlreadyExists,
    #[error("no se encontró ninguna bóveda")]
    VaultNotFound,
    #[error("la bóveda está bloqueada")]
    VaultLocked,
    #[error("contraseña maestra incorrecta o archivo dañado")]
    InvalidMasterPasswordOrCorruptedVault,
    #[error("demasiados intentos; reintenta en {retry_after_seconds} s")]
    UnlockTemporarilyBlocked { retry_after_seconds: u64 },
    #[error("el archivo de la bóveda está dañado: {reason}")]
    CorruptedVaultFile { reason: String },
    #[error("versión de bóveda no soportada: {found_version}")]
    UnsupportedVaultVersion { found_version: u16 },
    #[error("la contraseña maestra no cumple la política")]
    WeakMasterPassword { violations: Vec<MasterPasswordViolation> },
    #[error("parámetros de derivación de clave por debajo del mínimo seguro")]
    InsecureKeyDerivationParameters,
    #[error("elemento no encontrado: {item_id}")]
    ItemNotFound { item_id: Uuid },
    #[error("carpeta no encontrada: {folder_id}")]
    FolderNotFound { folder_id: Uuid },
    #[error("adjunto no encontrado: {attachment_id}")]
    AttachmentNotFound { attachment_id: Uuid },
    #[error("el adjunto supera el máximo de {maximum_size_bytes} bytes")]
    AttachmentTooLarge { maximum_size_bytes: u64 },
    #[error("este elemento no tiene ese campo")]
    FieldNotAvailable,
    #[error("secreto TOTP inválido")]
    InvalidTotpSecret,
    /// `reason_code` es un identificador estable (camelCase) que la UI traduce.
    #[error("datos de entrada inválidos: {reason_code}")]
    InvalidInput { reason_code: &'static str },
    #[error("formato de importación/exportación no soportado")]
    UnsupportedTransferFormat,
    #[error("este formato requiere una contraseña de archivo")]
    TransferPasswordRequired,
    #[error("no se pudo importar el archivo: {reason}")]
    ImportFailed { reason: String },
    #[error("error de almacenamiento: {reason}")]
    Storage { reason: String },
    #[error("error criptográfico")]
    Cryptography,
}

#[allow(clippy::needless_pass_by_value)]
impl VaultError {
    /// Código estable para que el frontend muestre un mensaje traducido.
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::VaultAlreadyExists => "vaultAlreadyExists",
            Self::VaultNotFound => "vaultNotFound",
            Self::VaultLocked => "vaultLocked",
            Self::InvalidMasterPasswordOrCorruptedVault => "invalidMasterPassword",
            Self::UnlockTemporarilyBlocked { .. } => "unlockTemporarilyBlocked",
            Self::CorruptedVaultFile { .. } => "corruptedVaultFile",
            Self::UnsupportedVaultVersion { .. } => "unsupportedVaultVersion",
            Self::WeakMasterPassword { .. } => "weakMasterPassword",
            Self::InsecureKeyDerivationParameters => "insecureKeyDerivationParameters",
            Self::ItemNotFound { .. } => "itemNotFound",
            Self::FolderNotFound { .. } => "folderNotFound",
            Self::AttachmentNotFound { .. } => "attachmentNotFound",
            Self::AttachmentTooLarge { .. } => "attachmentTooLarge",
            Self::FieldNotAvailable => "fieldNotAvailable",
            Self::InvalidTotpSecret => "invalidTotpSecret",
            Self::InvalidInput { .. } => "invalidInput",
            Self::UnsupportedTransferFormat => "unsupportedTransferFormat",
            Self::TransferPasswordRequired => "transferPasswordRequired",
            Self::ImportFailed { .. } => "importFailed",
            Self::Storage { .. } => "storage",
            Self::Cryptography => "cryptography",
        }
    }

    pub fn storage(reason: impl ToString) -> Self {
        Self::Storage { reason: reason.to_string() }
    }

    pub fn corrupted_file(reason: impl ToString) -> Self {
        Self::CorruptedVaultFile { reason: reason.to_string() }
    }

    pub fn invalid_input(reason_code: &'static str) -> Self {
        Self::InvalidInput { reason_code }
    }

    pub fn import_failed(reason: impl ToString) -> Self {
        Self::ImportFailed { reason: reason.to_string() }
    }
}

pub type VaultResult<T> = Result<T, VaultError>;
