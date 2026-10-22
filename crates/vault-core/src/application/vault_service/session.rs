//! Sesión: crear, abrir y bloquear la bóveda, cambiar la maestra y sus preferencias.

use crate::application::throttled_master_password_attempt::run_throttled;
use crate::application::vault_session::VaultSessionState;
use crate::application::{change_master_password, create_vault, unlock_vault};
use crate::domain::secret_text::SecretText;
use crate::domain::vault_error::VaultResult;
use crate::domain::vault_preferences::VaultPreferences;

use super::{UnlockReport, VaultService, VaultStatus};

pub trait VaultSession {
    fn vault_status(&self) -> VaultStatus;

    fn create_vault(&mut self, master_password: &SecretText) -> VaultResult<()>;

    /// Desbloquea e informa de lo que conviene avisar: rollback (ver `vault_revision`) o una
    /// maestra que ya no cumpliría la política (bóvedas creadas con una política anterior).
    fn unlock_vault(&mut self, master_password: &SecretText) -> VaultResult<UnlockReport>;

    fn lock_vault(&mut self);

    fn register_user_activity(&mut self) -> VaultResult<()>;

    /// Lo llama periódicamente el planificador de auto-bloqueo. Devuelve `true` si ha bloqueado.
    fn lock_if_idle(&mut self) -> bool;

    fn change_master_password(
        &mut self,
        current_master_password: &SecretText,
        new_master_password: &SecretText,
    ) -> VaultResult<()>;

    fn preferences(&self) -> VaultResult<VaultPreferences>;

    fn update_preferences(&mut self, new_preferences: VaultPreferences) -> VaultResult<VaultPreferences>;
}

impl VaultSession for VaultService {
    fn vault_status(&self) -> VaultStatus {
        VaultStatus {
            vault_exists: self.vault_dependencies.vault_repository.vault_exists(),
            is_unlocked: self.session_state.is_unlocked(),
            unlock_retry_after_seconds: self.unlock_attempt_throttle.seconds_until_next_attempt(self.now()),
        }
    }

    fn create_vault(&mut self, master_password: &SecretText) -> VaultResult<()> {
        let unlocked_vault = create_vault::execute(&self.vault_dependencies, master_password)?;
        self.session_state = VaultSessionState::Unlocked(Box::new(unlocked_vault));
        Ok(())
    }

    fn unlock_vault(&mut self, master_password: &SecretText) -> VaultResult<UnlockReport> {
        let unlock_outcome =
            unlock_vault::execute(&self.vault_dependencies, &mut self.unlock_attempt_throttle, master_password)?;
        self.session_state = VaultSessionState::Unlocked(Box::new(unlock_outcome.unlocked_vault));
        Ok(UnlockReport {
            detected_rollback: unlock_outcome.detected_rollback,
            master_password_violations: self.vault_dependencies.master_password_policy.find_violations(master_password),
        })
    }

    fn lock_vault(&mut self) {
        self.lock_session();
    }

    fn register_user_activity(&mut self) -> VaultResult<()> {
        Self::unlocked_vault_for_change(&mut self.session_state, self.vault_dependencies.clock.as_ref()).map(|_| ())
    }

    fn lock_if_idle(&mut self) -> bool {
        let Ok(unlocked_vault) = self.session_state.require_unlocked() else {
            return false;
        };
        let idle_seconds = unlocked_vault.last_activity.idle_seconds(self.vault_dependencies.clock.as_ref());
        let is_idle_too_long = idle_seconds >= unlocked_vault.vault_contents.preferences.auto_lock_after_idle_seconds;
        if is_idle_too_long {
            self.lock_session();
        }
        is_idle_too_long
    }

    fn change_master_password(
        &mut self,
        current_master_password: &SecretText,
        new_master_password: &SecretText,
    ) -> VaultResult<()> {
        let unlocked_vault =
            Self::unlocked_vault_for_change(&mut self.session_state, self.vault_dependencies.clock.as_ref())?;
        run_throttled(&self.vault_dependencies, &mut self.unlock_attempt_throttle, || {
            change_master_password::execute(
                &self.vault_dependencies,
                unlocked_vault,
                current_master_password,
                new_master_password,
            )
        })
    }

    fn preferences(&self) -> VaultResult<VaultPreferences> {
        Ok(self.session_state.require_unlocked()?.vault_contents.preferences.clone())
    }

    fn update_preferences(&mut self, new_preferences: VaultPreferences) -> VaultResult<VaultPreferences> {
        new_preferences.validate()?;
        self.mutate(|vault_contents| {
            vault_contents.preferences = new_preferences.clone();
            Ok(new_preferences)
        })
    }
}
