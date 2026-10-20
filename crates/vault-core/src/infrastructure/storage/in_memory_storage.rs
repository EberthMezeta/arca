//! Adaptadores en memoria para tests de casos de uso.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use uuid::Uuid;

use crate::domain::ports::attachment_storage::AttachmentStorage;
use crate::domain::ports::secret_clipboard::SecretClipboard;
use crate::domain::ports::unlock_throttle_store::UnlockThrottleStore;
use crate::domain::ports::vault_repository::VaultRepository;
use crate::domain::secret_text::SecretText;
use crate::domain::unlock_attempt_throttle::UnlockAttemptThrottle;
use crate::domain::vault_error::{VaultError, VaultResult};

#[derive(Default)]
pub struct InMemoryVaultRepository {
    pub stored_vault_bytes: Mutex<Option<Vec<u8>>>,
    pub backup_vault_bytes: Mutex<Option<Vec<u8>>>,
}

impl VaultRepository for InMemoryVaultRepository {
    fn vault_exists(&self) -> bool {
        self.stored_vault_bytes.lock().expect("mutex").is_some()
    }

    fn read_vault_bytes(&self) -> VaultResult<Vec<u8>> {
        self.stored_vault_bytes.lock().expect("mutex").clone().ok_or(VaultError::VaultNotFound)
    }

    fn write_vault_bytes(&self, encrypted_vault_bytes: &[u8]) -> VaultResult<()> {
        let mut stored_vault_bytes = self.stored_vault_bytes.lock().expect("mutex");
        *self.backup_vault_bytes.lock().expect("mutex") = stored_vault_bytes.take();
        *stored_vault_bytes = Some(encrypted_vault_bytes.to_vec());
        Ok(())
    }

    fn write_vault_bytes_discarding_backups(&self, encrypted_vault_bytes: &[u8]) -> VaultResult<()> {
        *self.backup_vault_bytes.lock().expect("mutex") = None;
        *self.stored_vault_bytes.lock().expect("mutex") = Some(encrypted_vault_bytes.to_vec());
        Ok(())
    }
}

#[derive(Default)]
pub struct InMemoryUnlockThrottleStore {
    pub stored_throttle: Mutex<Option<UnlockAttemptThrottle>>,
}

impl UnlockThrottleStore for InMemoryUnlockThrottleStore {
    fn load_throttle(&self) -> Option<UnlockAttemptThrottle> {
        self.stored_throttle.lock().expect("mutex").clone()
    }

    fn save_throttle(&self, unlock_attempt_throttle: &UnlockAttemptThrottle) -> VaultResult<()> {
        *self.stored_throttle.lock().expect("mutex") = Some(unlock_attempt_throttle.clone());
        Ok(())
    }
}

#[derive(Default)]
pub struct InMemoryAttachmentStorage {
    pub encrypted_attachments: Mutex<HashMap<Uuid, Vec<u8>>>,
}

impl AttachmentStorage for InMemoryAttachmentStorage {
    fn write_encrypted_attachment(&self, attachment_id: Uuid, encrypted_bytes: &[u8]) -> VaultResult<()> {
        self.encrypted_attachments.lock().expect("mutex").insert(attachment_id, encrypted_bytes.to_vec());
        Ok(())
    }

    fn read_encrypted_attachment(&self, attachment_id: Uuid) -> VaultResult<Vec<u8>> {
        self.encrypted_attachments
            .lock()
            .expect("mutex")
            .get(&attachment_id)
            .cloned()
            .ok_or(VaultError::AttachmentNotFound { attachment_id })
    }

    fn delete_encrypted_attachment(&self, attachment_id: Uuid) -> VaultResult<()> {
        self.encrypted_attachments.lock().expect("mutex").remove(&attachment_id);
        Ok(())
    }
}

#[derive(Default)]
pub struct FakeSecretClipboard {
    pub last_copied_value: Mutex<Option<(SecretText, Duration)>>,
}

impl SecretClipboard for FakeSecretClipboard {
    fn copy_with_automatic_clear(&self, secret_value: SecretText, clear_after: Duration) -> VaultResult<()> {
        *self.last_copied_value.lock().expect("mutex") = Some((secret_value, clear_after));
        Ok(())
    }

    fn clear_copied_secret(&self) {
        *self.last_copied_value.lock().expect("mutex") = None;
    }
}

/// Permite que el test conserve un `Arc` para inspeccionar el estado tras inyectarlo.
impl<T: VaultRepository> VaultRepository for std::sync::Arc<T> {
    fn vault_exists(&self) -> bool {
        self.as_ref().vault_exists()
    }

    fn read_vault_bytes(&self) -> VaultResult<Vec<u8>> {
        self.as_ref().read_vault_bytes()
    }

    fn write_vault_bytes(&self, encrypted_vault_bytes: &[u8]) -> VaultResult<()> {
        self.as_ref().write_vault_bytes(encrypted_vault_bytes)
    }

    fn write_vault_bytes_discarding_backups(&self, encrypted_vault_bytes: &[u8]) -> VaultResult<()> {
        self.as_ref().write_vault_bytes_discarding_backups(encrypted_vault_bytes)
    }
}

impl<T: SecretClipboard> SecretClipboard for std::sync::Arc<T> {
    fn copy_with_automatic_clear(&self, secret_value: SecretText, clear_after: Duration) -> VaultResult<()> {
        self.as_ref().copy_with_automatic_clear(secret_value, clear_after)
    }

    fn clear_copied_secret(&self) {
        self.as_ref().clear_copied_secret();
    }
}

impl<T: AttachmentStorage> AttachmentStorage for std::sync::Arc<T> {
    fn write_encrypted_attachment(&self, attachment_id: Uuid, encrypted_bytes: &[u8]) -> VaultResult<()> {
        self.as_ref().write_encrypted_attachment(attachment_id, encrypted_bytes)
    }

    fn read_encrypted_attachment(&self, attachment_id: Uuid) -> VaultResult<Vec<u8>> {
        self.as_ref().read_encrypted_attachment(attachment_id)
    }

    fn delete_encrypted_attachment(&self, attachment_id: Uuid) -> VaultResult<()> {
        self.as_ref().delete_encrypted_attachment(attachment_id)
    }
}

impl<T: UnlockThrottleStore> UnlockThrottleStore for std::sync::Arc<T> {
    fn load_throttle(&self) -> Option<UnlockAttemptThrottle> {
        self.as_ref().load_throttle()
    }

    fn save_throttle(&self, unlock_attempt_throttle: &UnlockAttemptThrottle) -> VaultResult<()> {
        self.as_ref().save_throttle(unlock_attempt_throttle)
    }
}
