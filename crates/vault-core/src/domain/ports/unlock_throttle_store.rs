use crate::domain::unlock_attempt_throttle::UnlockAttemptThrottle;
use crate::domain::vault_error::VaultResult;

/// Guarda el estado del retardo entre intentos de desbloqueo para que
/// cerrar y volver a abrir la app no lo reinicie.
pub trait UnlockThrottleStore: Send + Sync {
    fn load_throttle(&self) -> Option<UnlockAttemptThrottle>;
    fn save_throttle(&self, unlock_attempt_throttle: &UnlockAttemptThrottle) -> VaultResult<()>;
}
