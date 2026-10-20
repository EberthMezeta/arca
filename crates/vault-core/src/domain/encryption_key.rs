use std::fmt;

use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::domain::vault_error::VaultResult;

pub const ENCRYPTION_KEY_LENGTH_BYTES: usize = 32;

/// Clave simétrica de 256 bits. Se sobrescribe con ceros al liberarse
/// y nunca se serializa ni se muestra.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct EncryptionKey([u8; ENCRYPTION_KEY_LENGTH_BYTES]);

impl EncryptionKey {
    /// Escribe el material de clave directamente dentro de la clave, sin arrays intermedios
    /// que quedarían en la pila sin borrar. Si `fill_key_bytes` falla, la clave a medio
    /// rellenar se borra al soltarse (`ZeroizeOnDrop`).
    pub fn fill_in_place(
        fill_key_bytes: impl FnOnce(&mut [u8; ENCRYPTION_KEY_LENGTH_BYTES]) -> VaultResult<()>,
    ) -> VaultResult<Self> {
        let mut encryption_key = Self([0u8; ENCRYPTION_KEY_LENGTH_BYTES]);
        fill_key_bytes(&mut encryption_key.0)?;
        Ok(encryption_key)
    }

    /// `None` si `key_bytes` no mide exactamente lo que una clave.
    pub fn copy_from_slice(key_bytes: &[u8]) -> Option<Self> {
        if key_bytes.len() != ENCRYPTION_KEY_LENGTH_BYTES {
            return None;
        }
        Self::fill_in_place(|key_slot| {
            key_slot.copy_from_slice(key_bytes);
            Ok(())
        })
        .ok()
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
