//! Raíz de composición: el único sitio que conoce las implementaciones concretas.

use std::fs::File;
use std::io;
use std::path::PathBuf;

use vault_core::application::vault_service::VaultService;
use vault_core::infrastructure::storage::atomic_file_vault_repository::AtomicFileVaultRepository;
use vault_core::infrastructure::storage::directory_attachment_storage::DirectoryAttachmentStorage;
use vault_core::infrastructure::storage::json_file_unlock_throttle_store::JsonFileUnlockThrottleStore;
use vault_core::infrastructure::storage::json_file_vault_revision_store::JsonFileVaultRevisionStore;
use vault_core::infrastructure::storage::owner_only_files::create_owner_only_directory;
use vault_core::infrastructure::vault_dependencies_builder::{VaultDependenciesBuilder, VaultStorage};

use crate::platform::application_paths::application_data_directory;
use crate::platform::data_files::{
    ATTACHMENTS_DIRECTORY_NAME, SINGLE_INSTANCE_LOCK_FILE_NAME, UI_PREFERENCES_FILE_NAME, UNLOCK_THROTTLE_FILE_NAME,
    VAULT_FILE_NAME, VAULT_REVISION_FILE_NAME,
};
use crate::platform::legacy_data_migration::{MigrationOutcome, migrate_legacy_data};
use crate::platform::single_instance_lock::{self, InstanceLockOutcome};
use crate::platform::system_secret_clipboard::SystemSecretClipboard;
use crate::platform::ui_preferences_store::UiPreferencesStore;
use crate::shared_vault::SharedVault;
use crate::vault_events::VaultEventSink;

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

    let vault_storage = VaultStorage {
        vault_repository: Box::new(AtomicFileVaultRepository::new(&vault_file_path)),
        attachment_storage: Box::new(DirectoryAttachmentStorage::new(
            application_data_directory.join(ATTACHMENTS_DIRECTORY_NAME),
            MAXIMUM_ATTACHMENT_SIZE_BYTES,
        )),
        unlock_throttle_store: Box::new(JsonFileUnlockThrottleStore::new(
            application_data_directory.join(UNLOCK_THROTTLE_FILE_NAME),
        )),
        vault_revision_store: Box::new(JsonFileVaultRevisionStore::new(
            application_data_directory.join(VAULT_REVISION_FILE_NAME),
        )),
    };
    let secret_clipboard = Box::new(SystemSecretClipboard::new(vault_event_sink));
    let vault_dependencies =
        VaultDependenciesBuilder::new(vault_storage, secret_clipboard, MAXIMUM_ATTACHMENT_SIZE_BYTES).build();

    Ok(StartupOutcome::Ready(Box::new(ComposedApplication {
        shared_vault: SharedVault::new(VaultService::new(vault_dependencies)),
        vault_file_path,
        ui_preferences_store: UiPreferencesStore::new(application_data_directory.join(UI_PREFERENCES_FILE_NAME)),
        single_instance_lock,
    })))
}
