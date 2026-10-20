use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use vault_core::application::vault_service::VaultService;

use crate::command_error::{CommandError, CommandResult};
use crate::composition_root::ComposedApplication;

pub struct AppState {
    vault_service: Mutex<VaultService>,
    pub vault_file_path: PathBuf,
}

impl AppState {
    pub fn new(composed_application: ComposedApplication) -> Self {
        Self {
            vault_service: Mutex::new(composed_application.vault_service),
            vault_file_path: composed_application.vault_file_path,
        }
    }

    pub fn lock_vault_service(&self) -> CommandResult<MutexGuard<'_, VaultService>> {
        self.vault_service.lock().map_err(|_| CommandError::internal())
    }

    /// Bloquea si el usuario activó "bloquear al minimizar". Devuelve `true` si bloqueó.
    pub fn lock_if_minimize_policy_applies(&self) -> bool {
        let Ok(mut vault_service) = self.vault_service.lock() else {
            return false;
        };
        let should_lock = vault_service
            .preferences()
            .is_ok_and(|vault_preferences| vault_preferences.should_lock_on_window_minimize);
        if should_lock {
            vault_service.lock_vault();
        }
        should_lock
    }

    pub fn lock_if_idle(&self) -> bool {
        self.vault_service.lock().is_ok_and(|mut vault_service| vault_service.lock_if_idle())
    }
}
