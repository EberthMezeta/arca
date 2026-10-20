use crate::domain::vault_error::VaultResult;

/// Persistencia del archivo cifrado de la bóveda (bytes opacos).
pub trait VaultRepository: Send + Sync {
    fn vault_exists(&self) -> bool;
    fn read_vault_bytes(&self) -> VaultResult<Vec<u8>>;
    fn write_vault_bytes(&self, encrypted_vault_bytes: &[u8]) -> VaultResult<()>;
}
