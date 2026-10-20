//! Raíz de composición: el único sitio que conoce las implementaciones concretas.

use std::fs::File;
use std::path::PathBuf;

use tauri::{AppHandle, Manager};
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

use crate::adapters::tauri_secret_clipboard::TauriSecretClipboard;
use crate::single_instance_lock::{self, InstanceLockOutcome};

const VAULT_FILE_NAME: &str = "personal.vault";
const ATTACHMENTS_DIRECTORY_NAME: &str = "personal.vault.d";
const UNLOCK_THROTTLE_FILE_NAME: &str = "unlock-throttle.json";
pub const MAXIMUM_ATTACHMENT_SIZE_BYTES: u64 = 50 * 1024 * 1024;
/// Ningún export real de un gestor de contraseñas se acerca a esto.
pub const MAXIMUM_IMPORT_FILE_SIZE_BYTES: u64 = 64 * 1024 * 1024;

pub struct ComposedApplication {
    pub vault_service: VaultService,
    pub vault_file_path: PathBuf,
    pub single_instance_lock: File,
}

pub enum StartupOutcome {
    Ready(Box<ComposedApplication>),
    AnotherInstanceIsRunning,
}

pub fn compose_application(app_handle: &AppHandle) -> tauri::Result<StartupOutcome> {
    let application_data_directory = app_handle.path().app_data_dir()?;
    // También contiene la caché de WebKit: nadie más debe poder listar ni leer este directorio.
    create_owner_only_directory(&application_data_directory).map_err(|storage_error| {
        tauri::Error::Io(std::io::Error::other(format!("No se pudo proteger {}: {storage_error:?}", application_data_directory.display())))
    })?;
    let InstanceLockOutcome::Acquired(single_instance_lock) = single_instance_lock::acquire(&application_data_directory)? else {
        return Ok(StartupOutcome::AnotherInstanceIsRunning);
    };
    let vault_file_path = application_data_directory.join(VAULT_FILE_NAME);

    let vault_dependencies = VaultDependencies {
        vault_repository: Box::new(AtomicFileVaultRepository::new(&vault_file_path)),
        attachment_storage: Box::new(DirectoryAttachmentStorage::new(
            application_data_directory.join(ATTACHMENTS_DIRECTORY_NAME),
        )),
        key_derivation_function: Box::new(Argon2idKeyDerivation),
        authenticated_cipher: Box::new(XChaCha20Poly1305Cipher),
        secret_clipboard: Box::new(TauriSecretClipboard::new(app_handle.clone())),
        unlock_throttle_store: Box::new(JsonFileUnlockThrottleStore::new(
            application_data_directory.join(UNLOCK_THROTTLE_FILE_NAME),
        )),
        clock: Box::new(SystemClock),
        transfer_format_registry: build_default_transfer_format_registry(),
        key_derivation_policy: KeyDerivationPolicy::default(),
        maximum_attachment_size_bytes: MAXIMUM_ATTACHMENT_SIZE_BYTES,
    };

    Ok(StartupOutcome::Ready(Box::new(ComposedApplication {
        vault_service: VaultService::new(vault_dependencies),
        vault_file_path,
        single_instance_lock,
    })))
}
