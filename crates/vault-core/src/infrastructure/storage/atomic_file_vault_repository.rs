use std::fs::{self, File};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::domain::ports::vault_repository::VaultRepository;
use crate::domain::vault_error::{VaultError, VaultResult};
use crate::infrastructure::storage::owner_only_files::{create_owner_only_directory, write_owner_only_file};

/// Guarda la bóveda en disco de forma atómica:
/// copia de seguridad `.bak` → escribe `.tmp` + fsync → `rename` sobre el original.
/// Un corte de luz nunca deja un archivo a medio escribir.
pub struct AtomicFileVaultRepository {
    vault_file_path: PathBuf,
}

impl AtomicFileVaultRepository {
    pub fn new(vault_file_path: impl Into<PathBuf>) -> Self {
        Self { vault_file_path: vault_file_path.into() }
    }

    pub fn vault_file_path(&self) -> &Path {
        &self.vault_file_path
    }

    fn sibling_path_with_suffix(&self, suffix: &str) -> PathBuf {
        let mut sibling_path = self.vault_file_path.clone().into_os_string();
        sibling_path.push(suffix);
        PathBuf::from(sibling_path)
    }
}

impl VaultRepository for AtomicFileVaultRepository {
    fn vault_exists(&self) -> bool {
        self.vault_file_path.is_file()
    }

    fn read_vault_bytes(&self) -> VaultResult<Vec<u8>> {
        if !self.vault_exists() {
            return Err(VaultError::VaultNotFound);
        }
        fs::read(&self.vault_file_path).map_err(VaultError::storage)
    }

    fn write_vault_bytes(&self, encrypted_vault_bytes: &[u8]) -> VaultResult<()> {
        if let Some(parent_directory) = self.vault_file_path.parent() {
            create_owner_only_directory(parent_directory)?;
        }
        if self.vault_exists() {
            let previous_vault_bytes = fs::read(&self.vault_file_path).map_err(VaultError::storage)?;
            write_owner_only_file(&self.sibling_path_with_suffix(".bak"), &previous_vault_bytes)?;
        }

        let temporary_file_path = self.sibling_path_with_suffix(".tmp");
        write_owner_only_file(&temporary_file_path, encrypted_vault_bytes)?;
        fs::rename(&temporary_file_path, &self.vault_file_path).map_err(VaultError::storage)?;
        sync_parent_directory(&self.vault_file_path);
        Ok(())
    }

    fn discard_backup(&self) -> VaultResult<()> {
        match fs::remove_file(self.sibling_path_with_suffix(".bak")) {
            Ok(()) => Ok(()),
            Err(remove_error) if remove_error.kind() == ErrorKind::NotFound => Ok(()),
            Err(remove_error) => Err(VaultError::storage(remove_error)),
        }
    }
}

/// Best-effort: en algunos sistemas no se puede abrir un directorio para fsync.
fn sync_parent_directory(file_path: &Path) {
    let Some(parent_directory) = file_path.parent() else {
        return;
    };
    if let Ok(directory_handle) = File::open(parent_directory) {
        let _ = directory_handle.sync_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overwriting_keeps_previous_version_as_backup() {
        let temporary_directory = tempfile::tempdir().expect("directorio temporal");
        let vault_file_path = temporary_directory.path().join("personal.vault");
        let vault_repository = AtomicFileVaultRepository::new(&vault_file_path);

        vault_repository.write_vault_bytes(b"version-1").expect("escribir v1");
        vault_repository.write_vault_bytes(b"version-2").expect("escribir v2");

        assert_eq!(vault_repository.read_vault_bytes().expect("leer"), b"version-2");
        let backup_bytes = fs::read(temporary_directory.path().join("personal.vault.bak")).expect("leer bak");
        assert_eq!(backup_bytes, b"version-1");
    }

    #[test]
    fn discarding_backup_removes_previous_version() {
        let temporary_directory = tempfile::tempdir().expect("directorio temporal");
        let vault_file_path = temporary_directory.path().join("personal.vault");
        let vault_repository = AtomicFileVaultRepository::new(&vault_file_path);
        vault_repository.write_vault_bytes(b"version-1").expect("escribir v1");
        vault_repository.write_vault_bytes(b"version-2").expect("escribir v2");

        vault_repository.discard_backup().expect("descartar");

        assert!(!temporary_directory.path().join("personal.vault.bak").exists());
        assert_eq!(vault_repository.read_vault_bytes().expect("leer"), b"version-2");
        vault_repository.discard_backup().expect("descartar sin copia no falla");
    }

    #[test]
    fn missing_vault_is_reported() {
        let temporary_directory = tempfile::tempdir().expect("directorio temporal");
        let vault_repository = AtomicFileVaultRepository::new(temporary_directory.path().join("nada.vault"));

        assert!(matches!(vault_repository.read_vault_bytes(), Err(VaultError::VaultNotFound)));
    }
}
