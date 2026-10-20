use uuid::Uuid;

use crate::domain::vault_error::VaultResult;

/// Guarda blobs de adjuntos YA cifrados. No conoce claves ni contenido.
pub trait AttachmentStorage: Send + Sync {
    fn write_encrypted_attachment(&self, attachment_id: Uuid, encrypted_bytes: &[u8]) -> VaultResult<()>;
    fn read_encrypted_attachment(&self, attachment_id: Uuid) -> VaultResult<Vec<u8>>;
    fn delete_encrypted_attachment(&self, attachment_id: Uuid) -> VaultResult<()>;
}
