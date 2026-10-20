use crate::application::vault_dependencies::VaultDependencies;
use crate::domain::unlock_attempt_throttle::UnlockAttemptThrottle;
use crate::domain::vault_error::{VaultError, VaultResult};

/// Ejecuta una operación que comprueba la contraseña maestra (desbloquear, cambiarla,
/// exportar) bajo el mismo retardo exponencial: con la app abierta tampoco se puede
/// adivinar la contraseña a ritmo de Argon2id. Solo cuenta como fallo una contraseña incorrecta.
pub fn run_throttled<AttemptOutput>(
    vault_dependencies: &VaultDependencies,
    unlock_attempt_throttle: &mut UnlockAttemptThrottle,
    password_attempt: impl FnOnce() -> VaultResult<AttemptOutput>,
) -> VaultResult<AttemptOutput> {
    let now = vault_dependencies.clock.now();
    if let Some(retry_after_seconds) = unlock_attempt_throttle.seconds_until_next_attempt(now) {
        return Err(VaultError::UnlockTemporarilyBlocked { retry_after_seconds });
    }

    let attempt_result = password_attempt();
    match &attempt_result {
        Ok(_) => unlock_attempt_throttle.register_successful_attempt(),
        Err(VaultError::InvalidMasterPasswordOrCorruptedVault) => unlock_attempt_throttle.register_failed_attempt(now),
        Err(_) => return attempt_result,
    }
    // Best-effort: si no se puede guardar, el retardo sigue vigente en memoria.
    let _ = vault_dependencies.unlock_throttle_store.save_throttle(unlock_attempt_throttle);
    attempt_result
}
