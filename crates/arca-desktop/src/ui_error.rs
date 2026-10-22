use vault_core::domain::master_password_policy::MasterPasswordViolation;
use vault_core::domain::vault_error::VaultError;

/// Error listo para mostrarse: `error_code` se traduce con `errors.<código>`.
#[derive(Debug)]
pub struct UiError {
    pub error_code: &'static str,
    pub retry_after_seconds: Option<u64>,
    pub master_password_violations: Vec<MasterPasswordViolation>,
    pub technical_detail: Option<String>,
}

pub type UiResult<T> = Result<T, UiError>;

impl UiError {
    fn with_code(error_code: &'static str) -> Self {
        Self { error_code, retry_after_seconds: None, master_password_violations: Vec::new(), technical_detail: None }
    }

    pub fn internal() -> Self {
        Self::with_code("internal")
    }

    pub fn file_access(file_error: impl ToString) -> Self {
        Self { technical_detail: Some(file_error.to_string()), ..Self::with_code("fileAccess") }
    }

    pub fn import_file_too_large() -> Self {
        Self::with_code("importFileTooLarge")
    }

    /// Fallo al escribir un archivo elegido por el usuario (se informa como acceso a archivo, no como error de la bóveda).
    pub fn file_write(storage_error: VaultError) -> Self {
        Self::file_access(format!("{storage_error:?}"))
    }
}

impl From<VaultError> for UiError {
    fn from(vault_error: VaultError) -> Self {
        let mut ui_error = Self::with_code(vault_error.error_code());
        match vault_error {
            VaultError::UnlockTemporarilyBlocked { retry_after_seconds } => {
                ui_error.retry_after_seconds = Some(retry_after_seconds);
            }
            VaultError::WeakMasterPassword { violations } => ui_error.master_password_violations = violations,
            VaultError::ImportFailed { reason } => ui_error.technical_detail = Some(reason),
            VaultError::InvalidInput { reason_code } => ui_error.technical_detail = Some(reason_code.to_string()),
            _ => {}
        }
        ui_error
    }
}
