use std::sync::OnceLock;

use crate::domain::password_generation::PasswordGenerationStrategy;
use crate::domain::secret_text::SecretText;
use crate::domain::secure_random::random_index_below;
use crate::domain::vault_error::{VaultError, VaultResult};

/// Lista larga de la EFF: 7776 palabras ≈ 12,9 bits de entropía por palabra.
const EFF_LARGE_WORDLIST_SOURCE: &str = include_str!("../../../resources/eff_large_wordlist.txt");

pub const MINIMUM_WORD_COUNT: usize = 3;
pub const MAXIMUM_WORD_COUNT: usize = 20;
const MAXIMUM_SEPARATOR_CHARACTER_COUNT: usize = 5;

fn eff_wordlist() -> &'static [&'static str] {
    static PARSED_WORDLIST: OnceLock<Vec<&'static str>> = OnceLock::new();
    PARSED_WORDLIST.get_or_init(|| {
        EFF_LARGE_WORDLIST_SOURCE.lines().filter_map(|wordlist_line| wordlist_line.split('\t').nth(1)).collect()
    })
}

pub struct PassphraseStrategy {
    pub word_count: usize,
    pub word_separator: String,
    pub should_capitalize_words: bool,
    pub should_append_number: bool,
}

impl PassphraseStrategy {
    fn append_word(&self, passphrase: &mut String, word: &str) {
        let mut word_characters = word.chars();
        let Some(first_character) = word_characters.next() else {
            return;
        };
        if self.should_capitalize_words {
            passphrase.extend(first_character.to_uppercase());
        } else {
            passphrase.push(first_character);
        }
        passphrase.extend(word_characters);
    }
}

impl PasswordGenerationStrategy for PassphraseStrategy {
    /// La frase se escribe en un único `String` reservado de antemano con su tamaño máximo,
    /// para que crecer no deje copias de la frase en memoria liberada sin borrar.
    fn generate_password(&self) -> VaultResult<SecretText> {
        if !(MINIMUM_WORD_COUNT..=MAXIMUM_WORD_COUNT).contains(&self.word_count) {
            return Err(VaultError::invalid_input("wordCountOutOfRange"));
        }
        if self.word_separator.chars().count() > MAXIMUM_SEPARATOR_CHARACTER_COUNT {
            return Err(VaultError::invalid_input("wordSeparatorTooLong"));
        }

        let wordlist = eff_wordlist();
        let chosen_word_indexes =
            (0..self.word_count).map(|_| random_index_below(wordlist.len())).collect::<VaultResult<Vec<usize>>>()?;
        let appended_digit = self.should_append_number.then(|| random_index_below(10)).transpose()?;

        let longest_word_byte_count = wordlist.iter().map(|word| word.len()).max().unwrap_or_default();
        let maximum_byte_count = self.word_count * (longest_word_byte_count + self.word_separator.len()) + 1;
        let mut passphrase = String::with_capacity(maximum_byte_count);
        for (word_position, word_index) in chosen_word_indexes.iter().enumerate() {
            if word_position > 0 {
                passphrase.push_str(&self.word_separator);
            }
            self.append_word(&mut passphrase, wordlist[*word_index]);
        }
        if let Some(appended_digit) = appended_digit.and_then(|digit| char::from_digit(u32::try_from(digit).ok()?, 10))
        {
            passphrase.push(appended_digit);
        }
        Ok(SecretText::new(passphrase))
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

    #[test]
    fn overly_long_separator_is_rejected() {
        let passphrase_strategy = PassphraseStrategy {
            word_count: 5,
            word_separator: "-".repeat(1_000),
            should_capitalize_words: false,
            should_append_number: false,
        };

        assert!(passphrase_strategy.generate_password().is_err());
    }
}
