use std::collections::BTreeSet;

use crate::domain::password_generation::PasswordGenerationStrategy;
use crate::domain::password_generation::password_generation_options::CharacterSet;
use crate::domain::secret_text::SecretText;
use crate::domain::secure_random::{random_index_below, shuffle_securely};
use crate::domain::vault_error::{VaultError, VaultResult};

pub const MINIMUM_PASSWORD_LENGTH: usize = 8;
pub const MAXIMUM_PASSWORD_LENGTH: usize = 128;

pub struct RandomCharactersStrategy {
    pub password_length: usize,
    pub character_sets: BTreeSet<CharacterSet>,
    pub should_exclude_ambiguous_characters: bool,
}

impl RandomCharactersStrategy {
    fn validate(&self) -> VaultResult<()> {
        if !(MINIMUM_PASSWORD_LENGTH..=MAXIMUM_PASSWORD_LENGTH).contains(&self.password_length) {
            return Err(VaultError::invalid_input("passwordLengthOutOfRange"));
        }
        if self.character_sets.is_empty() {
            return Err(VaultError::invalid_input("characterSetRequired"));
        }
        Ok(())
    }
}

impl PasswordGenerationStrategy for RandomCharactersStrategy {
    /// Garantiza al menos un carácter de cada conjunto elegido y luego mezcla.
    fn generate_password(&self) -> VaultResult<SecretText> {
        self.validate()?;

        let character_pools: Vec<Vec<char>> = self
            .character_sets
            .iter()
            .map(|character_set| character_set.allowed_characters(self.should_exclude_ambiguous_characters))
            .collect();
        let combined_pool: Vec<char> = character_pools.iter().flatten().copied().collect();

        let mut password_characters = Vec::with_capacity(self.password_length);
        for character_pool in &character_pools {
            password_characters.push(character_pool[random_index_below(character_pool.len())?]);
        }
        while password_characters.len() < self.password_length {
            password_characters.push(combined_pool[random_index_below(combined_pool.len())?]);
        }
        shuffle_securely(&mut password_characters)?;

        let generated_password = SecretText::new(password_characters.iter().collect::<String>());
        password_characters.fill('\0');
        Ok(generated_password)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strategy_with(password_length: usize, character_sets: &[CharacterSet]) -> RandomCharactersStrategy {
        RandomCharactersStrategy {
            password_length,
            character_sets: character_sets.iter().copied().collect(),
            should_exclude_ambiguous_characters: true,
        }
    }

    #[test]
    fn generated_password_has_requested_length_and_every_set() {
        let all_sets = [CharacterSet::Uppercase, CharacterSet::Lowercase, CharacterSet::Digits, CharacterSet::Symbols];

        for _ in 0..50 {
            let generated_password = strategy_with(12, &all_sets).generate_password().expect("generar");
            let password_text = generated_password.expose_secret();

            assert_eq!(password_text.chars().count(), 12);
            assert!(password_text.chars().any(|character| character.is_ascii_uppercase()));
            assert!(password_text.chars().any(|character| character.is_ascii_lowercase()));
            assert!(password_text.chars().any(|character| character.is_ascii_digit()));
            assert!(password_text.chars().any(|character| !character.is_ascii_alphanumeric()));
            assert!(!password_text.contains(['I', 'l', '1', 'O', '0', 'o']));
        }
    }

    #[test]
    fn empty_character_sets_are_rejected() {
        assert!(strategy_with(16, &[]).generate_password().is_err());
    }

    #[test]
    fn too_short_length_is_rejected() {
        assert!(strategy_with(4, &[CharacterSet::Lowercase]).generate_password().is_err());
    }
}
