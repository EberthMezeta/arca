//! Generar contraseñas y estimar su fortaleza (no necesitan la bóveda abierta).

use crate::domain::password_generation::password_generation_options::PasswordGenerationOptions;
use crate::domain::password_strength::PasswordStrength;
use crate::domain::secret_text::SecretText;
use crate::domain::vault_error::VaultResult;

use super::VaultService;

pub trait PasswordTools {
    fn generate_password(&self, generation_options: PasswordGenerationOptions) -> VaultResult<SecretText>;

    fn estimate_password_strength(&self, candidate_password: &SecretText) -> PasswordStrength;
}

impl PasswordTools for VaultService {
    fn generate_password(&self, generation_options: PasswordGenerationOptions) -> VaultResult<SecretText> {
        generation_options.into_strategy().generate_password()
    }

    fn estimate_password_strength(&self, candidate_password: &SecretText) -> PasswordStrength {
        PasswordStrength::estimate(candidate_password)
    }
}
