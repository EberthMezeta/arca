//! Raíz de composición: el único sitio que conoce las implementaciones concretas.

use std::fs::File;
use std::io;
use std::path::PathBuf;

use vault_core::application::vault_dependencies::{KeyDerivationPolicy, VaultDependencies};
use vault_core::application::vault_service::VaultService;
use vault_core::domain::ports::clock::SystemClock;
use vault_core::infrastructure::crypto::argon2id_key_derivation::Argon2idKeyDerivation;
use vault_core::infrastructure::crypto::xchacha20_poly1305_cipher::XChaCha20Poly1305Cipher;
use vault_core::infrastructure::import_export::build_default_transfer_format_registry;
use vault_core::infrastructure::storage::atomic_file_vault_repository::AtomicFileVaultRepository;
use vault_core::infrastructure::storage::directory_attachment_storage::DirectoryAttachmentStorage;
use vault_core::infrastructure::storage::json_file_unlock_throttle_store::JsonFileUnlockThrottleStore;
use vault_core::infrastructure::storage::owner_only_files::create_owner_only_directory;

use crate::platform::application_paths::application_data_directory;
use crate::platform::legacy_data_migration::{MigrationOutcome, migrate_legacy_data};
use crate::platform::single_instance_lock::{self, InstanceLockOutcome, SINGLE_INSTANCE_LOCK_FILE_NAME};
use crate::platform::system_secret_clipboard::SystemSecretClipboard;
use crate::platform::ui_preferences_store::UiPreferencesStore;
use crate::shared_vault::SharedVault;
use crate::vault_events::VaultEventSink;

const VAULT_FILE_NAME: &str = "personal.vault";
const ATTACHMENTS_DIRECTORY_NAME: &str = "personal.vault.d";
const UNLOCK_THROTTLE_FILE_NAME: &str = "unlock-throttle.json";
const UI_PREFERENCES_FILE_NAME: &str = "ui-preferences.json";
pub const MAXIMUM_ATTACHMENT_SIZE_BYTES: u64 = 50 * 1024 * 1024;
/// Ningún export real de un gestor de contraseñas se acerca a esto.
pub const MAXIMUM_IMPORT_FILE_SIZE_BYTES: u64 = 64 * 1024 * 1024;

pub struct ComposedApplication {
    pub shared_vault: SharedVault,
    pub vault_file_path: PathBuf,
    pub ui_preferences_store: UiPreferencesStore,
    /// Se conserva para mantener el bloqueo de instancia única mientras la app esté abierta.
    pub single_instance_lock: File,
}

pub enum StartupOutcome {
    Ready(Box<ComposedApplication>),
    AnotherInstanceIsRunning,
}

pub fn compose_application(vault_event_sink: VaultEventSink) -> io::Result<StartupOutcome> {
    let application_data_directory = application_data_directory()?;
    create_owner_only_directory(&application_data_directory).map_err(|storage_error| {
        io::Error::other(format!("No se pudo proteger {}: {storage_error:?}", application_data_directory.display()))
    })?;
    let InstanceLockOutcome::Acquired(single_instance_lock) =
        single_instance_lock::acquire(&application_data_directory, SINGLE_INSTANCE_LOCK_FILE_NAME)?
    else {
        return Ok(StartupOutcome::AnotherInstanceIsRunning);
    };
    if let MigrationOutcome::LegacyAppStillRunning = migrate_legacy_data(&application_data_directory)? {
        return Ok(StartupOutcome::AnotherInstanceIsRunning);
    }
    let vault_file_path = application_data_directory.join(VAULT_FILE_NAME);

    let vault_dependencies = VaultDependencies {
        vault_repository: Box::new(AtomicFileVaultRepository::new(&vault_file_path)),
        attachment_storage: Box::new(DirectoryAttachmentStorage::new(
            application_data_directory.join(ATTACHMENTS_DIRECTORY_NAME),
        )),
        key_derivation_function: Box::new(Argon2idKeyDerivation),
        authenticated_cipher: Box::new(XChaCha20Poly1305Cipher),
        secret_clipboard: Box::new(SystemSecretClipboard::new(vault_event_sink)),
        unlock_throttle_store: Box::new(JsonFileUnlockThrottleStore::new(
            application_data_directory.join(UNLOCK_THROTTLE_FILE_NAME),
        )),
        clock: Box::new(SystemClock),
        transfer_format_registry: build_default_transfer_format_registry(),
        key_derivation_policy: KeyDerivationPolicy::default(),
        maximum_attachment_size_bytes: MAXIMUM_ATTACHMENT_SIZE_BYTES,
    };

    Ok(StartupOutcome::Ready(Box::new(ComposedApplication {
        shared_vault: SharedVault::new(VaultService::new(vault_dependencies)),
        vault_file_path,
        ui_preferences_store: UiPreferencesStore::new(application_data_directory.join(UI_PREFERENCES_FILE_NAME)),
        single_instance_lock,
    })))
}
