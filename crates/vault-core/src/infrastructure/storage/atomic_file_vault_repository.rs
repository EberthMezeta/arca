use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::domain::ports::vault_repository::VaultRepository;
use crate::domain::vault_error::{VaultError, VaultResult};

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
            fs::create_dir_all(parent_directory).map_err(VaultError::storage)?;
        }
        if self.vault_exists() {
            fs::copy(&self.vault_file_path, self.sibling_path_with_suffix(".bak")).map_err(VaultError::storage)?;
        }

        let temporary_file_path = self.sibling_path_with_suffix(".tmp");
        write_and_sync_owner_only_file(&temporary_file_path, encrypted_vault_bytes)?;
        fs::rename(&temporary_file_path, &self.vault_file_path).map_err(VaultError::storage)?;
        sync_parent_directory(&self.vault_file_path);
        Ok(())
    }
}

pub(crate) fn write_and_sync_owner_only_file(file_path: &Path, file_bytes: &[u8]) -> VaultResult<()> {
    let mut open_options = OpenOptions::new();
    open_options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        open_options.mode(0o600);
    }

    let mut output_file = open_options.open(file_path).map_err(VaultError::storage)?;
    output_file.write_all(file_bytes).map_err(VaultError::storage)?;
    output_file.sync_all().map_err(VaultError::storage)
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
    fn missing_vault_is_reported() {
        let temporary_directory = tempfile::tempdir().expect("directorio temporal");
        let vault_repository = AtomicFileVaultRepository::new(temporary_directory.path().join("nada.vault"));

        assert!(matches!(vault_repository.read_vault_bytes(), Err(VaultError::VaultNotFound)));
    }
}
