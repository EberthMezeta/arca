use std::sync::OnceLock;

use crate::domain::secret_text::SecretText;
use crate::domain::vault_error::{VaultError, VaultResult};
use crate::domain::secure_random::random_index_below;
use crate::domain::password_generation::PasswordGenerationStrategy;

/// Lista larga de la EFF: 7776 palabras ≈ 12,9 bits de entropía por palabra.
const EFF_LARGE_WORDLIST_SOURCE: &str = include_str!("../../../resources/eff_large_wordlist.txt");

pub const MINIMUM_WORD_COUNT: usize = 3;
pub const MAXIMUM_WORD_COUNT: usize = 20;

fn eff_wordlist() -> &'static [&'static str] {
    static PARSED_WORDLIST: OnceLock<Vec<&'static str>> = OnceLock::new();
    PARSED_WORDLIST.get_or_init(|| {
        EFF_LARGE_WORDLIST_SOURCE
            .lines()
            .filter_map(|wordlist_line| wordlist_line.split('\t').nth(1))
            .collect()
    })
}

pub struct PassphraseStrategy {
    pub word_count: usize,
    pub word_separator: String,
    pub should_capitalize_words: bool,
    pub should_append_number: bool,
}

impl PassphraseStrategy {
    fn format_word(&self, word: &str) -> String {
        if !self.should_capitalize_words {
            return word.to_string();
        }
        let mut word_characters = word.chars();
        word_characters
            .next()
            .map(|first_character| first_character.to_uppercase().chain(word_characters).collect())
            .unwrap_or_default()
    }
}

impl PasswordGenerationStrategy for PassphraseStrategy {
    fn generate_password(&self) -> VaultResult<SecretText> {
        if !(MINIMUM_WORD_COUNT..=MAXIMUM_WORD_COUNT).contains(&self.word_count) {
            return Err(VaultError::invalid_input("wordCountOutOfRange"));
        }

        let wordlist = eff_wordlist();
        let mut chosen_words = Vec::with_capacity(self.word_count);
        for _ in 0..self.word_count {
            chosen_words.push(self.format_word(wordlist[random_index_below(wordlist.len())?]));
        }
        if self.should_append_number {
            let appended_digit = random_index_below(10)?;
            let last_word_position = chosen_words.len() - 1;
            chosen_words[last_word_position].push_str(&appended_digit.to_string());
        }

        Ok(SecretText::new(chosen_words.join(&self.word_separator)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wordlist_has_all_eff_words() {
        assert_eq!(eff_wordlist().len(), 7776);
    }

    #[test]
    fn passphrase_has_requested_word_count() {
        let passphrase_strategy = PassphraseStrategy {
            word_count: 5,
            word_separator: "-".into(),
            should_capitalize_words: true,
            should_append_number: true,
        };

        let generated_passphrase = passphrase_strategy.generate_password().expect("generar");
        let passphrase_words: Vec<&str> = generated_passphrase.expose_secret().split('-').collect();

        assert_eq!(passphrase_words.len(), 5);
        assert!(passphrase_words.iter().all(|word| word.starts_with(|first: char| first.is_uppercase())));
        assert!(passphrase_words[4].ends_with(|last: char| last.is_ascii_digit()));
    }
}
