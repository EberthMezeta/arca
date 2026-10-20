//! Escritura de archivos y directorios accesibles solo por el usuario actual.
//! Se usa para todo lo que la app deja en disco: bóveda, adjuntos cifrados,
//! exportaciones (que pueden ir en texto plano) y adjuntos descifrados.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::domain::vault_error::{VaultError, VaultResult};

#[cfg(unix)]
const OWNER_ONLY_FILE_MODE: u32 = 0o600;
#[cfg(unix)]
const OWNER_ONLY_DIRECTORY_MODE: u32 = 0o700;

/// Escribe y sincroniza con permisos `0600`. Si el archivo ya existía con permisos
/// más abiertos (p. ej. una exportación anterior), se restringen antes de escribir.
/// En sistemas de archivos sin permisos Unix (FAT32 / exFAT de un USB) el `chmod` falla
/// con `EPERM`: ahí los permisos no significan nada, así que se escribe igualmente.
pub fn write_owner_only_file(file_path: &Path, file_bytes: &[u8]) -> VaultResult<()> {
    let mut open_options = OpenOptions::new();
    open_options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        open_options.mode(OWNER_ONLY_FILE_MODE);
    }

    let mut output_file = open_options.open(file_path).map_err(VaultError::storage)?;
    restrict_to_owner_if_supported(&output_file);
    output_file.write_all(file_bytes).map_err(VaultError::storage)?;
    output_file.sync_all().map_err(VaultError::storage)
}

#[cfg(unix)]
fn restrict_to_owner_if_supported(open_file: &fs::File) {
    use std::os::unix::fs::PermissionsExt;
    let _ = open_file.set_permissions(fs::Permissions::from_mode(OWNER_ONLY_FILE_MODE));
}

#[cfg(not(unix))]
fn restrict_to_owner_if_supported(_open_file: &fs::File) {}

/// Crea el directorio (y sus padres) y lo deja en `0700`: nadie más puede listar qué hay dentro.
pub fn create_owner_only_directory(directory_path: &Path) -> VaultResult<()> {
    fs::create_dir_all(directory_path).map_err(VaultError::storage)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(directory_path, fs::Permissions::from_mode(OWNER_ONLY_DIRECTORY_MODE))
            .map_err(VaultError::storage)?;
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    fn permission_bits(path: &Path) -> u32 {
        fs::metadata(path).expect("metadatos").permissions().mode() & 0o777
    }

    #[test]
    fn new_file_is_readable_only_by_owner() {
        let temporary_directory = tempfile::tempdir().expect("directorio temporal");
        let file_path = temporary_directory.path().join("export.csv");

        write_owner_only_file(&file_path, b"secreto").expect("escribir");

        assert_eq!(permission_bits(&file_path), 0o600);
    }

    #[test]
    fn existing_world_readable_file_gets_restricted() {
        let temporary_directory = tempfile::tempdir().expect("directorio temporal");
        let file_path = temporary_directory.path().join("export.csv");
        fs::write(&file_path, b"antiguo").expect("escribir");
        fs::set_permissions(&file_path, fs::Permissions::from_mode(0o644)).expect("permisos");

        write_owner_only_file(&file_path, b"nuevo").expect("sobrescribir");

        assert_eq!(permission_bits(&file_path), 0o600);
        assert_eq!(fs::read(&file_path).expect("leer"), b"nuevo");
    }

    #[test]
    fn directory_is_restricted_even_if_it_already_existed() {
        let temporary_directory = tempfile::tempdir().expect("directorio temporal");
        let data_directory = temporary_directory.path().join("datos");
        fs::create_dir(&data_directory).expect("crear");
        fs::set_permissions(&data_directory, fs::Permissions::from_mode(0o775)).expect("permisos");

        create_owner_only_directory(&data_directory).expect("restringir");

        assert_eq!(permission_bits(&data_directory), 0o700);
    }
}
