use serde::{Deserialize, Serialize};

use crate::domain::vault_error::{VaultError, VaultResult};

/// Preferencias de seguridad guardadas (cifradas) dentro de la propia bóveda.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VaultPreferences {
    pub auto_lock_after_idle_seconds: u64,
    pub clipboard_clear_after_seconds: u64,
    pub password_history_limit: usize,
    pub should_lock_on_window_minimize: bool,
}

impl Default for VaultPreferences {
    fn default() -> Self {
        Self {
            auto_lock_after_idle_seconds: 5 * 60,
            clipboard_clear_after_seconds: 30,
            password_history_limit: 10,
            should_lock_on_window_minimize: false,
        }
    }
}

const MINIMUM_AUTO_LOCK_SECONDS: u64 = 30;
const MAXIMUM_AUTO_LOCK_SECONDS: u64 = 24 * 60 * 60;
const MINIMUM_CLIPBOARD_CLEAR_SECONDS: u64 = 5;
const MAXIMUM_CLIPBOARD_CLEAR_SECONDS: u64 = 10 * 60;
const MAXIMUM_PASSWORD_HISTORY_LIMIT: usize = 100;

impl VaultPreferences {
    pub fn validate(&self) -> VaultResult<()> {
        if !(MINIMUM_AUTO_LOCK_SECONDS..=MAXIMUM_AUTO_LOCK_SECONDS).contains(&self.auto_lock_after_idle_seconds) {
            return Err(VaultError::invalid_input("autoLockOutOfRange"));
        }
        if !(MINIMUM_CLIPBOARD_CLEAR_SECONDS..=MAXIMUM_CLIPBOARD_CLEAR_SECONDS)
            .contains(&self.clipboard_clear_after_seconds)
        {
            return Err(VaultError::invalid_input("clipboardClearOutOfRange"));
        }
        if self.password_history_limit > MAXIMUM_PASSWORD_HISTORY_LIMIT {
            return Err(VaultError::invalid_input("historyLimitOutOfRange"));
        }
        Ok(())
    }
}
