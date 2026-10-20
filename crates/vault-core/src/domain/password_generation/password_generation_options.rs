use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::domain::password_generation::PasswordGenerationStrategy;
use crate::domain::password_generation::passphrase_strategy::PassphraseStrategy;
use crate::domain::password_generation::random_characters_strategy::RandomCharactersStrategy;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CharacterSet {
    Uppercase,
    Lowercase,
    Digits,
    Symbols,
}

const AMBIGUOUS_CHARACTERS: &str = "Il1O0o";

impl CharacterSet {
    fn all_characters(self) -> &'static str {
        match self {
            Self::Uppercase => "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            Self::Lowercase => "abcdefghijklmnopqrstuvwxyz",
            Self::Digits => "0123456789",
            Self::Symbols => "!@#$%^&*()-_=+[]{};:,.?/~",
        }
    }

    pub fn allowed_characters(self, should_exclude_ambiguous: bool) -> Vec<char> {
        self.all_characters()
            .chars()
            .filter(|candidate_character| !should_exclude_ambiguous || !AMBIGUOUS_CHARACTERS.contains(*candidate_character))
            .collect()
    }
}

/// Opciones que envía la UI; actúan como fábrica de la estrategia correspondiente.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "strategy", rename_all = "camelCase")]
pub enum PasswordGenerationOptions {
    #[serde(rename_all = "camelCase")]
    RandomCharacters {
        password_length: usize,
        character_sets: BTreeSet<CharacterSet>,
        should_exclude_ambiguous_characters: bool,
    },
    #[serde(rename_all = "camelCase")]
    Passphrase {
        word_count: usize,
        word_separator: String,
        should_capitalize_words: bool,
        should_append_number: bool,
    },
}

impl Default for PasswordGenerationOptions {
    fn default() -> Self {
        Self::RandomCharacters {
            password_length: 20,
            character_sets: BTreeSet::from([
                CharacterSet::Uppercase,
                CharacterSet::Lowercase,
                CharacterSet::Digits,
                CharacterSet::Symbols,
            ]),
            should_exclude_ambiguous_characters: false,
        }
    }
}

impl PasswordGenerationOptions {
    pub fn into_strategy(self) -> Box<dyn PasswordGenerationStrategy> {
        match self {
            Self::RandomCharacters { password_length, character_sets, should_exclude_ambiguous_characters } => {
                Box::new(RandomCharactersStrategy {
                    password_length,
                    character_sets,
                    should_exclude_ambiguous_characters,
                })
            }
            Self::Passphrase { word_count, word_separator, should_capitalize_words, should_append_number } => {
                Box::new(PassphraseStrategy { word_count, word_separator, should_capitalize_words, should_append_number })
            }
        }
    }
}
