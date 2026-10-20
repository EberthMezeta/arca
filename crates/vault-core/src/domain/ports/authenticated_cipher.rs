use zeroize::Zeroizing;

use crate::domain::encryption_key::EncryptionKey;
use crate::domain::vault_error::VaultResult;

/// Resultado de un cifrado autenticado: nonce aleatorio + texto cifrado con etiqueta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedBox {
    pub nonce: Vec<u8>,
    pub ciphertext_with_tag: Vec<u8>,
}

/// Cifrado autenticado con datos asociados (AEAD).
pub trait AuthenticatedCipher: Send + Sync {
    fn nonce_length_bytes(&self) -> usize;

    fn seal(&self, encryption_key: &EncryptionKey, plaintext: &[u8], associated_data: &[u8]) -> VaultResult<SealedBox>;

    fn open(
        &self,
        encryption_key: &EncryptionKey,
        sealed_box: &SealedBox,
        associated_data: &[u8],
    ) -> VaultResult<Zeroizing<Vec<u8>>>;
}
