use std::sync::Arc;

use crate::application::transfer_format_registry::TransferFormatRegistry;
use crate::domain::key_derivation_parameters::KeyDerivationParameters;
use crate::domain::ports::attachment_storage::AttachmentStorage;
use crate::domain::ports::authenticated_cipher::AuthenticatedCipher;
use crate::domain::ports::clock::Clock;
use crate::domain::ports::key_derivation_function::KeyDerivationFunction;
use crate::domain::ports::secret_clipboard::SecretClipboard;
use crate::domain::ports::unlock_throttle_store::UnlockThrottleStore;
use crate::domain::ports::vault_repository::VaultRepository;
use crate::domain::ports::vault_revision_store::VaultRevisionStore;

pub struct KeyDerivationPolicy {
    pub parameters_for_new_vaults: KeyDerivationParameters,
    pub minimum_accepted_parameters: KeyDerivationParameters,
}

impl Default for KeyDerivationPolicy {
    fn default() -> Self {
        Self {
            parameters_for_new_vaults: KeyDerivationParameters::RECOMMENDED,
            minimum_accepted_parameters: KeyDerivationParameters::MINIMUM_ACCEPTED,
        }
    }
}

impl KeyDerivationPolicy {
    pub fn accepts(&self, candidate_parameters: &KeyDerivationParameters) -> bool {
        candidate_parameters.is_within(&self.minimum_accepted_parameters, &KeyDerivationParameters::MAXIMUM_ACCEPTED)
    }
}

/// Todos los puertos que necesitan los casos de uso, inyectados desde la raíz de composición.
pub struct VaultDependencies {
    pub vault_repository: Box<dyn VaultRepository>,
    pub attachment_storage: Box<dyn AttachmentStorage>,
    pub key_derivation_function: Box<dyn KeyDerivationFunction>,
    pub authenticated_cipher: Box<dyn AuthenticatedCipher>,
    pub secret_clipboard: Box<dyn SecretClipboard>,
    pub unlock_throttle_store: Box<dyn UnlockThrottleStore>,
    pub vault_revision_store: Box<dyn VaultRevisionStore>,
    pub clock: Box<dyn Clock>,
    pub transfer_format_registry: Arc<TransferFormatRegistry>,
    pub key_derivation_policy: KeyDerivationPolicy,
    pub maximum_attachment_size_bytes: u64,
}
