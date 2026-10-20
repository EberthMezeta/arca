pub mod passphrase_strategy;
pub mod password_generation_options;
pub mod random_characters_strategy;

use crate::domain::secret_text::SecretText;
use crate::domain::vault_error::VaultResult;

/// Patrón Strategy: cada forma de generar contraseñas implementa este trait.
pub trait PasswordGenerationStrategy {
    fn generate_password(&self) -> VaultResult<SecretText>;
}
