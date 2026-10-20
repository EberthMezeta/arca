//! La app se llamaba `RustVault` y guardaba sus datos en otra carpeta (`dev.rustvault.desktop`).
//! La primera vez que Arca arranca, mueve allí la bóveda; si no, parecería que la bóveda desapareció.

use std::fs;
use std::io;
use std::path::Path;

use crate::single_instance_lock::{self, InstanceLockOutcome};

const LEGACY_APPLICATION_IDENTIFIER: &str = "dev.rustvault.desktop";
const LEGACY_SINGLE_INSTANCE_LOCK_FILE_NAME: &str = "rustvault.lock";
const VAULT_FILE_NAME: &str = "personal.vault";

/// Lo que se traslada: la bóveda, su copia, los adjuntos cifrados, el retardo de desbloqueo
/// y el `localStorage` del `WebView` (idioma, tema y opciones del generador). Las cachés de `WebKit` no.
const MIGRATED_ENTRY_NAMES: [&str; 5] =
    ["personal.vault", "personal.vault.bak", "personal.vault.d", "unlock-throttle.json", "localstorage"];

pub enum MigrationOutcome {
    NothingToMigrate,
    Migrated,
    LegacyAppStillRunning,
}

/// Solo migra si la carpeta antigua tiene bóveda y la nueva todavía no: nunca sobrescribe nada.
/// Un fallo a medias se devuelve como error para no mostrar la pantalla de "crear bóveda"
/// a alguien que sí tiene una.
pub fn migrate_legacy_data(application_data_directory: &Path) -> io::Result<MigrationOutcome> {
    let Some(data_root_directory) = application_data_directory.parent() else {
        return Ok(MigrationOutcome::NothingToMigrate);
    };
    let legacy_data_directory = data_root_directory.join(LEGACY_APPLICATION_IDENTIFIER);
    let has_legacy_vault = legacy_data_directory.join(VAULT_FILE_NAME).is_file();
    let already_has_vault = application_data_directory.join(VAULT_FILE_NAME).exists();
    if !has_legacy_vault || already_has_vault {
        return Ok(MigrationOutcome::NothingToMigrate);
    }

    let legacy_instance_lock = single_instance_lock::acquire(&legacy_data_directory, LEGACY_SINGLE_INSTANCE_LOCK_FILE_NAME)?;
    let InstanceLockOutcome::Acquired(_legacy_lock_held_during_migration) = legacy_instance_lock else {
        return Ok(MigrationOutcome::LegacyAppStillRunning);
    };

    for entry_name in MIGRATED_ENTRY_NAMES {
        let legacy_entry_path = legacy_data_directory.join(entry_name);
        let migrated_entry_path = application_data_directory.join(entry_name);
        if !legacy_entry_path.exists() || migrated_entry_path.exists() {
            continue;
        }
        fs::rename(&legacy_entry_path, &migrated_entry_path)?;
    }
    Ok(MigrationOutcome::Migrated)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// Simula `~/.local/share` con la carpeta antigua y la nueva.
    fn data_root_for(test_name: &str) -> (PathBuf, PathBuf, PathBuf) {
        let data_root_directory = std::env::temp_dir().join(format!("arca-migration-{test_name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&data_root_directory);
        let legacy_data_directory = data_root_directory.join(LEGACY_APPLICATION_IDENTIFIER);
        let application_data_directory = data_root_directory.join("dev.arca.desktop");
        fs::create_dir_all(&legacy_data_directory).expect("carpeta antigua");
        fs::create_dir_all(&application_data_directory).expect("carpeta nueva");
        (data_root_directory, legacy_data_directory, application_data_directory)
    }

    #[test]
    fn legacy_vault_and_attachments_are_moved_but_caches_stay() {
        let (data_root_directory, legacy_data_directory, application_data_directory) = data_root_for("mueve");
        fs::write(legacy_data_directory.join("personal.vault"), b"boveda").expect("escribir");
        fs::create_dir_all(legacy_data_directory.join("personal.vault.d")).expect("adjuntos");
        fs::write(legacy_data_directory.join("personal.vault.d/adjunto.bin"), b"cifrado").expect("escribir");
        fs::create_dir_all(legacy_data_directory.join("WebKitCache")).expect("cache");

        let migration_outcome = migrate_legacy_data(&application_data_directory).expect("migrar");

        assert!(matches!(migration_outcome, MigrationOutcome::Migrated));
        assert_eq!(fs::read(application_data_directory.join("personal.vault")).expect("leer"), b"boveda");
        assert!(application_data_directory.join("personal.vault.d/adjunto.bin").is_file());
        assert!(!legacy_data_directory.join("personal.vault").exists());
        assert!(legacy_data_directory.join("WebKitCache").exists());
        let _ = fs::remove_dir_all(data_root_directory);
    }

    #[test]
    fn existing_vault_in_the_new_location_is_never_overwritten() {
        let (data_root_directory, legacy_data_directory, application_data_directory) = data_root_for("respeta");
        fs::write(legacy_data_directory.join("personal.vault"), b"antigua").expect("escribir");
        fs::write(application_data_directory.join("personal.vault"), b"nueva").expect("escribir");

        let migration_outcome = migrate_legacy_data(&application_data_directory).expect("migrar");

        assert!(matches!(migration_outcome, MigrationOutcome::NothingToMigrate));
        assert_eq!(fs::read(application_data_directory.join("personal.vault")).expect("leer"), b"nueva");
        assert!(legacy_data_directory.join("personal.vault").exists());
        let _ = fs::remove_dir_all(data_root_directory);
    }

    #[test]
    fn running_legacy_app_blocks_the_migration() {
        let (data_root_directory, legacy_data_directory, application_data_directory) = data_root_for("abierta");
        fs::write(legacy_data_directory.join("personal.vault"), b"boveda").expect("escribir");
        let _legacy_app_lock =
            single_instance_lock::acquire(&legacy_data_directory, LEGACY_SINGLE_INSTANCE_LOCK_FILE_NAME).expect("bloqueo");

        let migration_outcome = migrate_legacy_data(&application_data_directory).expect("migrar");

        assert!(matches!(migration_outcome, MigrationOutcome::LegacyAppStillRunning));
        assert!(legacy_data_directory.join("personal.vault").exists());
        let _ = fs::remove_dir_all(data_root_directory);
    }
}
