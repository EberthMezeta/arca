use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use zeroize::Zeroizing;

use crate::domain::encryption_key::EncryptionKey;
use crate::domain::ports::authenticated_cipher::{AuthenticatedCipher, SealedBox};
use crate::domain::vault_error::{VaultError, VaultResult};
use crate::domain::secure_random::random_bytes;

pub const XCHACHA20_NONCE_LENGTH_BYTES: usize = 24;

/// XChaCha20-Poly1305: nonce de 192 bits, seguro de generar al azar en cada cifrado.
pub struct XChaCha20Poly1305Cipher;

impl XChaCha20Poly1305Cipher {
    fn build_cipher(encryption_key: &EncryptionKey) -> VaultResult<XChaCha20Poly1305> {
        XChaCha20Poly1305::new_from_slice(encryption_key.as_bytes()).map_err(|_| VaultError::Cryptography)
    }
}

impl AuthenticatedCipher for XChaCha20Poly1305Cipher {
    fn nonce_length_bytes(&self) -> usize {
        XCHACHA20_NONCE_LENGTH_BYTES
    }

    fn seal(&self, encryption_key: &EncryptionKey, plaintext: &[u8], associated_data: &[u8]) -> VaultResult<SealedBox> {
        let cipher = Self::build_cipher(encryption_key)?;
        let nonce_bytes = random_bytes::<XCHACHA20_NONCE_LENGTH_BYTES>()?;
        let nonce = XNonce::from(nonce_bytes);

        let ciphertext_with_tag = cipher
            .encrypt(&nonce, Payload { msg: plaintext, aad: associated_data })
            .map_err(|_| VaultError::Cryptography)?;
        Ok(SealedBox { nonce: nonce_bytes.to_vec(), ciphertext_with_tag })
    }

    fn open(
        &self,
        encryption_key: &EncryptionKey,
        sealed_box: &SealedBox,
        associated_data: &[u8],
    ) -> VaultResult<Zeroizing<Vec<u8>>> {
        let cipher = Self::build_cipher(encryption_key)?;
        let nonce_bytes: [u8; XCHACHA20_NONCE_LENGTH_BYTES] =
            sealed_box.nonce.as_slice().try_into().map_err(|_| VaultError::Cryptography)?;
        let nonce = XNonce::from(nonce_bytes);

        cipher
            .decrypt(&nonce, Payload { msg: &sealed_box.ciphertext_with_tag, aad: associated_data })
            .map(Zeroizing::new)
            .map_err(|_| VaultError::Cryptography)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::secure_random::random_encryption_key;

    #[test]
    fn sealed_box_round_trips() {
        let encryption_key = random_encryption_key().expect("clave");
        let sealed_box = XChaCha20Poly1305Cipher.seal(&encryption_key, b"secreto", b"cabecera").expect("cifrar");

        let decrypted_plaintext = XChaCha20Poly1305Cipher.open(&encryption_key, &sealed_box, b"cabecera").expect("descifrar");

        assert_eq!(decrypted_plaintext.as_slice(), b"secreto");
    }

    #[test]
    fn tampered_associated_data_is_rejected() {
        let encryption_key = random_encryption_key().expect("clave");
        let sealed_box = XChaCha20Poly1305Cipher.seal(&encryption_key, b"secreto", b"cabecera").expect("cifrar");

        let open_result = XChaCha20Poly1305Cipher.open(&encryption_key, &sealed_box, b"cabecerA");

        assert!(open_result.is_err());
    }

    #[test]
    fn tampered_ciphertext_is_rejected() {
        let encryption_key = random_encryption_key().expect("clave");
        let mut sealed_box = XChaCha20Poly1305Cipher.seal(&encryption_key, b"secreto", b"").expect("cifrar");
        sealed_box.ciphertext_with_tag[0] ^= 0x01;

        assert!(XChaCha20Poly1305Cipher.open(&encryption_key, &sealed_box, b"").is_err());
    }

    #[test]
    fn every_seal_uses_a_fresh_nonce() {
        let encryption_key = random_encryption_key().expect("clave");

        let first_box = XChaCha20Poly1305Cipher.seal(&encryption_key, b"igual", b"").expect("cifrar");
        let second_box = XChaCha20Poly1305Cipher.seal(&encryption_key, b"igual", b"").expect("cifrar");

        assert_ne!(first_box.nonce, second_box.nonce);
    }
}
