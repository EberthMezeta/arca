use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, Utc};

use crate::application::vault_dependencies::{KeyDerivationPolicy, VaultDependencies};
use crate::application::vault_service::VaultService;
use crate::domain::key_derivation_parameters::KeyDerivationParameters;
use crate::domain::ports::clock::Clock;
use crate::infrastructure::crypto::argon2id_key_derivation::Argon2idKeyDerivation;
use crate::infrastructure::crypto::xchacha20_poly1305_cipher::XChaCha20Poly1305Cipher;
use crate::infrastructure::import_export::build_default_transfer_format_registry;
use crate::infrastructure::storage::in_memory_storage::{
    FakeSecretClipboard, InMemoryAttachmentStorage, InMemoryUnlockThrottleStore, InMemoryVaultRepository,
    InMemoryVaultRevisionStore,
};

pub const STRONG_MASTER_PASSWORD: &str = "glacier-mantis-orbit-velvet-27";

/// Reloj controlable; los clones comparten la misma hora.
#[derive(Clone)]
pub struct FakeClock {
    current_time: Arc<Mutex<DateTime<Utc>>>,
}

impl FakeClock {
    pub fn starting_now() -> Self {
        Self { current_time: Arc::new(Mutex::new(Utc::now())) }
    }

    pub fn advance_by(&self, elapsed_time: Duration) {
        *self.current_time.lock().expect("mutex") += elapsed_time;
    }
}

impl Clock for FakeClock {
    fn now(&self) -> DateTime<Utc> {
        *self.current_time.lock().expect("mutex")
    }
}

/// Argon2id barato: los tests prueban la lógica, no el coste.
const CHEAP_TEST_KEY_DERIVATION_PARAMETERS: KeyDerivationParameters =
    KeyDerivationParameters { memory_cost_kibibytes: 64, iteration_count: 1, parallelism_degree: 1 };

pub struct TestVaultHarness {
    pub vault_service: VaultService,
    pub fake_clock: FakeClock,
    pub fake_clipboard: Arc<FakeSecretClipboard>,
    pub vault_repository: Arc<InMemoryVaultRepository>,
    pub attachment_storage: Arc<InMemoryAttachmentStorage>,
    pub unlock_throttle_store: Arc<InMemoryUnlockThrottleStore>,
    pub vault_revision_store: Arc<InMemoryVaultRevisionStore>,
}

/// Adaptadores compartidos para poder "reiniciar la app" con el mismo disco simulado.
#[derive(Clone, Default)]
pub struct SharedTestStorage {
    pub vault_repository: Arc<InMemoryVaultRepository>,
    pub attachment_storage: Arc<InMemoryAttachmentStorage>,
    pub unlock_throttle_store: Arc<InMemoryUnlockThrottleStore>,
    pub vault_revision_store: Arc<InMemoryVaultRevisionStore>,
}

pub fn build_test_vault_service() -> TestVaultHarness {
    build_test_vault_service_on(SharedTestStorage::default(), FakeClock::starting_now())
}

pub fn build_test_vault_service_on(shared_storage: SharedTestStorage, fake_clock: FakeClock) -> TestVaultHarness {
    let fake_clipboard = Arc::new(FakeSecretClipboard::default());
    let vault_dependencies = build_test_vault_dependencies(&shared_storage, &fake_clock, &fake_clipboard);
    let SharedTestStorage { vault_repository, attachment_storage, unlock_throttle_store, vault_revision_store } = shared_storage;

    TestVaultHarness {
        vault_service: VaultService::new(vault_dependencies),
        fake_clock,
        fake_clipboard,
        vault_repository,
        attachment_storage,
        unlock_throttle_store,
        vault_revision_store,
    }
}

/// Las dependencias sin la fachada: para preparar en el "disco" bóvedas que la fachada no dejaría crear.
pub fn build_test_vault_dependencies(
    shared_storage: &SharedTestStorage,
    fake_clock: &FakeClock,
    fake_clipboard: &Arc<FakeSecretClipboard>,
) -> VaultDependencies {
    let SharedTestStorage { vault_repository, attachment_storage, unlock_throttle_store, vault_revision_store } = shared_storage;
    VaultDependencies {
        vault_repository: Box::new(Arc::clone(vault_repository)),
        attachment_storage: Box::new(Arc::clone(attachment_storage)),
        key_derivation_function: Box::new(Argon2idKeyDerivation),
        authenticated_cipher: Box::new(XChaCha20Poly1305Cipher),
        secret_clipboard: Box::new(Arc::clone(fake_clipboard)),
        unlock_throttle_store: Box::new(Arc::clone(unlock_throttle_store)),
        vault_revision_store: Box::new(Arc::clone(vault_revision_store)),
        clock: Box::new(fake_clock.clone()),
        transfer_format_registry: build_default_transfer_format_registry(),
        key_derivation_policy: KeyDerivationPolicy {
            parameters_for_new_vaults: CHEAP_TEST_KEY_DERIVATION_PARAMETERS,
            minimum_accepted_parameters: CHEAP_TEST_KEY_DERIVATION_PARAMETERS,
        },
        maximum_attachment_size_bytes: 1024,
    }
}
