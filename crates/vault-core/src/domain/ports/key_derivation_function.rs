use crate::domain::encryption_key::EncryptionKey;
use crate::domain::key_derivation_parameters::KeyDerivationParameters;
use crate::domain::secret_text::SecretText;
use crate::domain::vault_error::VaultResult;

/// Deriva la clave que envuelve la clave de la bóveda a partir de la contraseña maestra.
pub trait KeyDerivationFunction: Send + Sync {
    fn derive_key_encryption_key(
        &self,
        master_password: &SecretText,
        salt: &[u8],
        derivation_parameters: &KeyDerivationParameters,
    ) -> VaultResult<EncryptionKey>;
}
