use serde::Serialize;
use vault_core::domain::master_password_policy::MasterPasswordViolation;
use vault_core::domain::vault_error::VaultError;

/// Error serializable para la UI: `error_code` se traduce con i18n en el frontend.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub error_code: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub master_password_violations: Vec<MasterPasswordViolation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub technical_detail: Option<String>,
}

pub type CommandResult<T> = Result<T, CommandError>;

impl CommandError {
    fn with_code(error_code: &'static str) -> Self {
        Self { error_code, retry_after_seconds: None, master_password_violations: Vec::new(), technical_detail: None }
    }

    pub fn internal() -> Self {
        Self::with_code("internal")
    }

    pub fn file_access(file_error: impl ToString) -> Self {
        Self { technical_detail: Some(file_error.to_string()), ..Self::with_code("fileAccess") }
    }
}

impl From<VaultError> for CommandError {
    fn from(vault_error: VaultError) -> Self {
        let mut command_error = Self::with_code(vault_error.error_code());
        match vault_error {
            VaultError::UnlockTemporarilyBlocked { retry_after_seconds } => {
                command_error.retry_after_seconds = Some(retry_after_seconds);
            }
            VaultError::WeakMasterPassword { violations } => command_error.master_password_violations = violations,
            VaultError::ImportFailed { reason } => command_error.technical_detail = Some(reason),
            VaultError::InvalidInput { reason_code } => command_error.technical_detail = Some(reason_code.to_string()),
            _ => {}
        }
        command_error
    }
}
