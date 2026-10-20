//! Jerarquía de claves: de la clave de bóveda (aleatoria) se derivan subclaves
//! independientes por propósito con HKDF-SHA256, para no reutilizar una misma clave en contextos distintos.

use hkdf::Hkdf;
use sha2::Sha256;
use uuid::Uuid;

use crate::domain::encryption_key::EncryptionKey;
use crate::domain::vault_error::{VaultError, VaultResult};

const VAULT_CONTENTS_KEY_PURPOSE: &[u8] = b"rustvault/v1/vault-contents";
const ATTACHMENTS_ROOT_KEY_PURPOSE: &[u8] = b"rustvault/v1/attachments-root";
const SINGLE_ATTACHMENT_KEY_PURPOSE: &[u8] = b"rustvault/v1/attachment/";

pub struct VaultSubkeys {
    pub vault_contents_key: EncryptionKey,
    pub attachments_root_key: EncryptionKey,
}

pub fn derive_vault_subkeys(vault_key: &EncryptionKey) -> VaultResult<VaultSubkeys> {
    Ok(VaultSubkeys {
        vault_contents_key: derive_subkey(vault_key, VAULT_CONTENTS_KEY_PURPOSE)?,
        attachments_root_key: derive_subkey(vault_key, ATTACHMENTS_ROOT_KEY_PURPOSE)?,
    })
}

/// Cada adjunto obtiene su propia clave única derivada de su identificador.
pub fn derive_single_attachment_key(attachments_root_key: &EncryptionKey, attachment_id: Uuid) -> VaultResult<EncryptionKey> {
    let purpose_with_identifier = [SINGLE_ATTACHMENT_KEY_PURPOSE, attachment_id.as_bytes().as_slice()].concat();
    derive_subkey(attachments_root_key, &purpose_with_identifier)
}

fn derive_subkey(parent_key: &EncryptionKey, purpose_label: &[u8]) -> VaultResult<EncryptionKey> {
    let key_derivation = Hkdf::<Sha256>::new(None, parent_key.as_bytes());
    EncryptionKey::fill_in_place(|key_slot| {
        key_derivation.expand(purpose_label, key_slot).map_err(|_| VaultError::Cryptography)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::secure_random::random_encryption_key;

    #[test]
    fn subkeys_are_distinct_from_each_other() {
        let vault_key = random_encryption_key().expect("clave");

        let vault_subkeys = derive_vault_subkeys(&vault_key).expect("subclaves");

        assert_ne!(vault_subkeys.vault_contents_key.as_bytes(), vault_subkeys.attachments_root_key.as_bytes());
        assert_ne!(vault_subkeys.vault_contents_key.as_bytes(), vault_key.as_bytes());
    }

    #[test]
    fn each_attachment_gets_its_own_key() {
        let attachments_root_key = random_encryption_key().expect("clave");

        let first_key = derive_single_attachment_key(&attachments_root_key, Uuid::new_v4()).expect("clave");
        let second_key = derive_single_attachment_key(&attachments_root_key, Uuid::new_v4()).expect("clave");

        assert_ne!(first_key.as_bytes(), second_key.as_bytes());
    }
}
