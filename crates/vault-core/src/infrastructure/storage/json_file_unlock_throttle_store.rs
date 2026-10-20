use std::fs;
use std::path::PathBuf;

use crate::domain::ports::unlock_throttle_store::UnlockThrottleStore;
use crate::domain::unlock_attempt_throttle::UnlockAttemptThrottle;
use crate::domain::vault_error::{VaultError, VaultResult};
use crate::infrastructure::storage::owner_only_files::write_owner_only_file;

/// Guarda el contador de intentos fallidos en un JSON junto a la bóveda.
/// No es secreto: solo evita que reiniciar la app borre el retardo.
/// (Un atacante con acceso al disco puede copiar la bóveda y atacarla fuera de la app;
/// contra eso protege Argon2id, no este retardo.)
pub struct JsonFileUnlockThrottleStore {
    throttle_file_path: PathBuf,
}

impl JsonFileUnlockThrottleStore {
    pub fn new(throttle_file_path: impl Into<PathBuf>) -> Self {
        Self { throttle_file_path: throttle_file_path.into() }
    }
}

impl UnlockThrottleStore for JsonFileUnlockThrottleStore {
    /// Un archivo ausente o ilegible equivale a "sin intentos fallidos".
    fn load_throttle(&self) -> Option<UnlockAttemptThrottle> {
        let serialized_throttle = fs::read(&self.throttle_file_path).ok()?;
        serde_json::from_slice(&serialized_throttle).ok()
    }

    fn save_throttle(&self, unlock_attempt_throttle: &UnlockAttemptThrottle) -> VaultResult<()> {
        let serialized_throttle = serde_json::to_vec(unlock_attempt_throttle).map_err(VaultError::storage)?;
        write_owner_only_file(&self.throttle_file_path, &serialized_throttle)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    #[test]
    fn saved_throttle_is_loaded_back() {
        let temporary_directory = tempfile::tempdir().expect("directorio temporal");
        let throttle_store = JsonFileUnlockThrottleStore::new(temporary_directory.path().join("unlock-throttle.json"));
        let now = Utc::now();
        let mut unlock_attempt_throttle = UnlockAttemptThrottle::default();
        (0..5).for_each(|_| unlock_attempt_throttle.register_failed_attempt(now));

        throttle_store.save_throttle(&unlock_attempt_throttle).expect("guardar");

        assert_eq!(throttle_store.load_throttle(), Some(unlock_attempt_throttle));
    }

    #[test]
    fn missing_or_corrupted_file_means_no_throttle() {
        let temporary_directory = tempfile::tempdir().expect("directorio temporal");
        let throttle_file_path = temporary_directory.path().join("unlock-throttle.json");
        let throttle_store = JsonFileUnlockThrottleStore::new(&throttle_file_path);

        assert_eq!(throttle_store.load_throttle(), None);

        fs::write(&throttle_file_path, b"{no es json").expect("escribir");
        assert_eq!(throttle_store.load_throttle(), None);
    }
}
