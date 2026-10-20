use std::fmt;

use zeroize::{Zeroize, ZeroizeOnDrop};

pub const ENCRYPTION_KEY_LENGTH_BYTES: usize = 32;

/// Clave simétrica de 256 bits. Se sobrescribe con ceros al liberarse
/// y nunca se serializa ni se muestra.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct EncryptionKey([u8; ENCRYPTION_KEY_LENGTH_BYTES]);

impl EncryptionKey {
    pub fn from_bytes(key_bytes: [u8; ENCRYPTION_KEY_LENGTH_BYTES]) -> Self {
        Self(key_bytes)
    }

    pub fn as_bytes(&self) -> &[u8; ENCRYPTION_KEY_LENGTH_BYTES] {
        &self.0
    }
}

impl fmt::Debug for EncryptionKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("EncryptionKey(***)")
    }
}
