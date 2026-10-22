//! Builder de `VaultDependencies`: el almacenamiento y el portapapeles son obligatorios; el resto
//! tiene los valores de producción (Argon2id, XChaCha20-Poly1305, reloj del sistema, política
//! por defecto) y quien construye solo cambia lo que le interesa (p. ej. los tests).

use std::sync::Arc;

use crate::application::vault_dependencies::{KeyDerivationPolicy, VaultDependencies};
use crate::domain::master_password_policy::MasterPasswordPolicy;
use crate::domain::ports::attachment_storage::AttachmentStorage;
use crate::domain::ports::clock::{Clock, SystemClock};
use crate::domain::ports::secret_clipboard::SecretClipboard;
use crate::domain::ports::unlock_throttle_store::UnlockThrottleStore;
use crate::domain::ports::vault_repository::VaultRepository;
use crate::domain::ports::vault_revision_store::VaultRevisionStore;
use crate::infrastructure::crypto::argon2id_key_derivation::Argon2idKeyDerivation;
use crate::infrastructure::crypto::xchacha20_poly1305_cipher::XChaCha20Poly1305Cipher;
use crate::infrastructure::import_export::build_transfer_format_registry;
use crate::infrastructure::import_export::encrypted_vault_export::ExportCryptography;

/// Lo que va a disco además de la bóveda: siempre lo elige quien construye.
pub struct VaultStorage {
    pub vault_repository: Box<dyn VaultRepository>,
    pub attachment_storage: Box<dyn AttachmentStorage>,
    pub unlock_throttle_store: Box<dyn UnlockThrottleStore>,
    pub vault_revision_store: Box<dyn VaultRevisionStore>,
}

pub struct VaultDependenciesBuilder {
    storage: VaultStorage,
    secret_clipboard: Box<dyn SecretClipboard>,
    clock: Box<dyn Clock>,
    key_derivation_policy: KeyDerivationPolicy,
    master_password_policy: Arc<MasterPasswordPolicy>,
    maximum_attachment_size_bytes: u64,
}

impl VaultDependenciesBuilder {
    pub fn new(
        storage: VaultStorage,
        secret_clipboard: Box<dyn SecretClipboard>,
        maximum_attachment_size_bytes: u64,
    ) -> Self {
        Self {
            storage,
            secret_clipboard,
            clock: Box::new(SystemClock),
            key_derivation_policy: KeyDerivationPolicy::default(),
            master_password_policy: Arc::new(MasterPasswordPolicy::default()),
            maximum_attachment_size_bytes,
        }
    }

    #[must_use]
    pub fn clock(mut self, clock: Box<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Coste de Argon2id para la bóveda y, con los mismos valores, para las copias `.arca`.
    #[must_use]
    pub fn key_derivation_policy(mut self, key_derivation_policy: KeyDerivationPolicy) -> Self {
        self.key_derivation_policy = key_derivation_policy;
        self
    }

    #[must_use]
    pub fn master_password_policy(mut self, master_password_policy: MasterPasswordPolicy) -> Self {
        self.master_password_policy = Arc::new(master_password_policy);
        self
    }

    pub fn build(self) -> VaultDependencies {
        let export_cryptography = ExportCryptography {
            key_derivation_function: Arc::new(Argon2idKeyDerivation),
            authenticated_cipher: Arc::new(XChaCha20Poly1305Cipher),
            parameters_for_new_exports: self.key_derivation_policy.parameters_for_new_vaults,
            minimum_accepted_parameters: self.key_derivation_policy.minimum_accepted_parameters,
            file_password_policy: Arc::clone(&self.master_password_policy),
        };
        let VaultStorage { vault_repository, attachment_storage, unlock_throttle_store, vault_revision_store } =
            self.storage;
        VaultDependencies {
            vault_repository,
            attachment_storage,
            key_derivation_function: Box::new(Argon2idKeyDerivation),
            authenticated_cipher: Box::new(XChaCha20Poly1305Cipher),
            secret_clipboard: self.secret_clipboard,
            unlock_throttle_store,
            vault_revision_store,
            clock: self.clock,
            transfer_format_registry: Arc::new(build_transfer_format_registry(&export_cryptography)),
            key_derivation_policy: self.key_derivation_policy,
            master_password_policy: self.master_password_policy,
            maximum_attachment_size_bytes: self.maximum_attachment_size_bytes,
        }
    }
}
