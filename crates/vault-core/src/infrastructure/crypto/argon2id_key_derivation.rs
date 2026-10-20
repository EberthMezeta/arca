use argon2::{Algorithm, Argon2, Params, Version};

use crate::domain::encryption_key::{ENCRYPTION_KEY_LENGTH_BYTES, EncryptionKey};
use crate::domain::key_derivation_parameters::KeyDerivationParameters;
use crate::domain::ports::key_derivation_function::KeyDerivationFunction;
use crate::domain::secret_text::SecretText;
use crate::domain::vault_error::{VaultError, VaultResult};

pub struct Argon2idKeyDerivation;

impl KeyDerivationFunction for Argon2idKeyDerivation {
    fn derive_key_encryption_key(
        &self,
        master_password: &SecretText,
        salt: &[u8],
        derivation_parameters: &KeyDerivationParameters,
    ) -> VaultResult<EncryptionKey> {
        let argon2_parameters = Params::new(
            derivation_parameters.memory_cost_kibibytes,
            derivation_parameters.iteration_count,
            derivation_parameters.parallelism_degree,
            Some(ENCRYPTION_KEY_LENGTH_BYTES),
        )
        .map_err(|_| VaultError::InsecureKeyDerivationParameters)?;
        let argon2id = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon2_parameters);

        EncryptionKey::fill_in_place(|key_slot| {
            argon2id
                .hash_password_into(master_password.expose_secret().as_bytes(), salt, key_slot)
                .map_err(|_| VaultError::Cryptography)
        })
    }
}
