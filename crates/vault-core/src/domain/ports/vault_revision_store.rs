use crate::domain::vault_error::VaultResult;
use crate::domain::vault_revision::VaultRevision;

/// Guarda fuera de la bóveda la última revisión vista, para notar si el archivo retrocede.
pub trait VaultRevisionStore: Send + Sync {
    fn load_last_seen_revision(&self) -> Option<VaultRevision>;
    fn record_revision(&self, vault_revision: &VaultRevision) -> VaultResult<()>;
}
