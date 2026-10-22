use std::fs;
use std::path::PathBuf;

use crate::domain::ports::vault_revision_store::VaultRevisionStore;
use crate::domain::vault_error::{VaultError, VaultResult};
use crate::domain::vault_revision::VaultRevision;
use crate::infrastructure::storage::owner_only_files::write_owner_only_file;

/// Guarda la última revisión vista en un JSON junto a la bóveda. No es secreto ni está firmado:
/// quien pueda escribir en el disco puede retroceder también este archivo (ver «Límites conocidos»).
pub struct JsonFileVaultRevisionStore {
    revision_file_path: PathBuf,
}

impl JsonFileVaultRevisionStore {
    pub fn new(revision_file_path: impl Into<PathBuf>) -> Self {
        Self { revision_file_path: revision_file_path.into() }
    }
}

impl VaultRevisionStore for JsonFileVaultRevisionStore {
    /// Un archivo ausente o ilegible equivale a "nada visto todavía".
    fn load_last_seen_revision(&self) -> Option<VaultRevision> {
        let serialized_revision = fs::read(&self.revision_file_path).ok()?;
        serde_json::from_slice(&serialized_revision).ok()
    }

    fn record_revision(&self, vault_revision: &VaultRevision) -> VaultResult<()> {
        let serialized_revision = serde_json::to_vec(vault_revision).map_err(VaultError::storage)?;
        write_owner_only_file(&self.revision_file_path, &serialized_revision)
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    #[test]
    fn recorded_revision_is_loaded_back() {
        let temporary_directory = tempfile::tempdir().expect("directorio temporal");
        let revision_store = JsonFileVaultRevisionStore::new(temporary_directory.path().join("vault-revision.json"));
        let vault_revision = VaultRevision { vault_id: Uuid::new_v4(), revision_number: 42 };

        assert_eq!(revision_store.load_last_seen_revision(), None);
        revision_store.record_revision(&vault_revision).expect("guardar");

        assert_eq!(revision_store.load_last_seen_revision(), Some(vault_revision));
    }
}
