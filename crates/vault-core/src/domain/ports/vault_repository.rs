use crate::domain::vault_error::VaultResult;

/// Persistencia del archivo cifrado de la bóveda (bytes opacos).
pub trait VaultRepository: Send + Sync {
    fn vault_exists(&self) -> bool;
    fn read_vault_bytes(&self) -> VaultResult<Vec<u8>>;
    /// Guarda la versión nueva conservando la anterior como copia de seguridad.
    fn write_vault_bytes(&self, encrypted_vault_bytes: &[u8]) -> VaultResult<()>;
    /// Guarda la versión nueva sin dejar copia de la anterior (ni antes ni después).
    /// Tras rotar la clave, cualquier copia anterior se abriría con la contraseña vieja.
    fn write_vault_bytes_discarding_backups(&self, encrypted_vault_bytes: &[u8]) -> VaultResult<()>;
}
