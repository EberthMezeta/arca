//! Fachada: único punto de entrada para la capa de presentación. Se reparte en traits por tema
//! (`VaultSession`, `VaultItems`, `VaultClipboard`, `VaultAttachments`, `VaultTransfer`,
//! `PasswordTools`): cada controlador importa solo los que usa.
//! Cada método delega en un caso de uso; aquí solo se gestiona el estado de sesión.

mod session;
mod items;
mod clipboard;
mod attachments;
mod transfer;
mod password_tools;

pub use attachments::VaultAttachments;
pub use clipboard::VaultClipboard;
pub use items::VaultItems;
pub use password_tools::PasswordTools;
pub use session::VaultSession;
pub use transfer::VaultTransfer;


use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::application::throttled_master_password_attempt::run_throttled;
use crate::domain::unlock_attempt_throttle::UnlockAttemptThrottle;
use crate::application::vault_dependencies::VaultDependencies;
use crate::application::vault_key_wrapping::unwrap_vault_key;
use crate::application::vault_session::{ActivityTimestamp, UnlockedVault, VaultSessionState};
use crate::domain::ports::clock::Clock;
use crate::domain::secret_text::SecretText;
use crate::domain::vault_contents::VaultContents;
use crate::domain::vault_error::VaultResult;
use crate::domain::master_password_policy::MasterPasswordViolation;
use crate::domain::vault_revision::VaultRollback;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatus {
    pub vault_exists: bool,
    pub is_unlocked: bool,
    pub unlock_retry_after_seconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnlockReport {
    pub detected_rollback: Option<VaultRollback>,
    /// Vacío si la maestra cumple la política actual.
    pub master_password_violations: Vec<MasterPasswordViolation>,
}

pub struct VaultService {
    vault_dependencies: VaultDependencies,
    session_state: VaultSessionState,
    unlock_attempt_throttle: UnlockAttemptThrottle,
}

impl VaultService {
    pub fn new(vault_dependencies: VaultDependencies) -> Self {
        let unlock_attempt_throttle = vault_dependencies.unlock_throttle_store.load_throttle().unwrap_or_default();
        Self { vault_dependencies, session_state: VaultSessionState::Locked, unlock_attempt_throttle }
    }

    /// Descarta claves y secretos de memoria y vacía el portapapeles si aún tiene un secreto nuestro.
    fn lock_session(&mut self) {
        self.session_state.lock();
        self.vault_dependencies.secret_clipboard.clear_copied_secret();
    }

    /// Acceso de escritura: además cuenta como actividad del usuario (retrasa el auto-bloqueo).
    fn unlocked_vault_for_change<'session>(
        session_state: &'session mut VaultSessionState,
        clock: &dyn Clock,
    ) -> VaultResult<&'session mut UnlockedVault> {
        let unlocked_vault = session_state.require_unlocked_mut()?;
        unlocked_vault.last_activity = ActivityTimestamp::now(clock);
        Ok(unlocked_vault)
    }

    fn now(&self) -> DateTime<Utc> {
        self.vault_dependencies.clock.now()
    }

    /// Aplica un cambio a la bóveda desbloqueada y lo guarda cifrado (cuenta como actividad).
    fn mutate<Output>(
        &mut self,
        apply_mutation: impl FnOnce(&mut VaultContents) -> VaultResult<Output>,
    ) -> VaultResult<Output> {
        let unlocked_vault =
            Self::unlocked_vault_for_change(&mut self.session_state, self.vault_dependencies.clock.as_ref())?;
        unlocked_vault.modify_and_persist(&self.vault_dependencies, apply_mutation)
    }

    /// Como `mutate`, sin dejar `.bak`: para borrados definitivos.
    fn mutate_discarding_backups<Output>(
        &mut self,
        apply_mutation: impl FnOnce(&mut VaultContents) -> VaultResult<Output>,
    ) -> VaultResult<Output> {
        let unlocked_vault =
            Self::unlocked_vault_for_change(&mut self.session_state, self.vault_dependencies.clock.as_ref())?;
        unlocked_vault.modify_and_persist_discarding_backups(&self.vault_dependencies, apply_mutation)
    }

    /// Vuelve a pedir la contraseña maestra (con retardo) antes de sacar algo del cifrado.
    fn reauthenticate(&mut self, master_password: &SecretText) -> VaultResult<()> {
        let unlocked_vault = self.session_state.require_unlocked()?;
        run_throttled(&self.vault_dependencies, &mut self.unlock_attempt_throttle, || {
            unwrap_vault_key(&self.vault_dependencies, master_password, &unlocked_vault.vault_header)
        })?;
        Ok(())
    }
}
