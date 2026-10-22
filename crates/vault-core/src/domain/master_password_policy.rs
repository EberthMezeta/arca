use serde::Serialize;

use crate::domain::password_strength::PasswordStrength;
use crate::domain::secret_text::SecretText;

/// Motivo concreto por el que una contraseña maestra no es aceptable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum MasterPasswordViolation {
    #[serde(rename_all = "camelCase")]
    TooShort { minimum_character_count: usize },
    #[serde(rename_all = "camelCase")]
    TooWeak { minimum_strength_score: u8 },
}

/// Regla individual de la política (patrón Strategy + lista de reglas en vez de `if`s encadenados).
trait MasterPasswordRule: Send + Sync {
    fn find_violation(&self, candidate_password: &SecretText) -> Option<MasterPasswordViolation>;
}

struct MinimumLengthRule {
    minimum_character_count: usize,
}

impl MasterPasswordRule for MinimumLengthRule {
    fn find_violation(&self, candidate_password: &SecretText) -> Option<MasterPasswordViolation> {
        let is_long_enough = candidate_password.character_count() >= self.minimum_character_count;
        (!is_long_enough)
            .then_some(MasterPasswordViolation::TooShort { minimum_character_count: self.minimum_character_count })
    }
}

struct MinimumStrengthRule {
    minimum_strength_score: u8,
}

impl MasterPasswordRule for MinimumStrengthRule {
    fn find_violation(&self, candidate_password: &SecretText) -> Option<MasterPasswordViolation> {
        let strength = PasswordStrength::estimate(candidate_password);
        let is_strong_enough = strength.score >= self.minimum_strength_score;
        (!is_strong_enough)
            .then_some(MasterPasswordViolation::TooWeak { minimum_strength_score: self.minimum_strength_score })
    }
}

pub const MINIMUM_MASTER_PASSWORD_CHARACTER_COUNT: usize = 12;
/// 4 = más de 10^10 intentos según zxcvbn. Con 3 (10^8) bastan horas con varias GPU
/// contra una copia del archivo, aun con Argon2id a 256 MiB.
pub const MINIMUM_MASTER_PASSWORD_STRENGTH_SCORE: u8 = 4;

pub struct MasterPasswordPolicy {
    rules: Vec<Box<dyn MasterPasswordRule>>,
}

impl Default for MasterPasswordPolicy {
    fn default() -> Self {
        Self {
            rules: vec![
                Box::new(MinimumLengthRule { minimum_character_count: MINIMUM_MASTER_PASSWORD_CHARACTER_COUNT }),
                Box::new(MinimumStrengthRule { minimum_strength_score: MINIMUM_MASTER_PASSWORD_STRENGTH_SCORE }),
            ],
        }
    }
}

impl MasterPasswordPolicy {
    pub fn find_violations(&self, candidate_password: &SecretText) -> Vec<MasterPasswordViolation> {
        self.rules.iter().filter_map(|rule| rule.find_violation(candidate_password)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_and_common_password_reports_both_violations() {
        let violations = MasterPasswordPolicy::default().find_violations(&SecretText::new("password"));

        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn long_but_predictable_passwords_are_rejected() {
        for predictable_password in ["Barcelona1992!", "Hola123456789!", "arcaarca2024!", "contraseñasegura"] {
            let violations = MasterPasswordPolicy::default().find_violations(&SecretText::new(predictable_password));

            assert_eq!(
                violations,
                vec![MasterPasswordViolation::TooWeak { minimum_strength_score: 4 }],
                "{predictable_password}"
            );
        }
    }

    #[test]
    fn long_random_passphrase_is_accepted() {
        let candidate_password = SecretText::new("glacier-mantis-orbit-velvet-27");

        let violations = MasterPasswordPolicy::default().find_violations(&candidate_password);

        assert_eq!(violations, Vec::new());
    }
}
