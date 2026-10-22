//! Preferencias de interfaz por equipo (idioma, tema, opciones del generador). Nunca datos de la bóveda:
//! van en un JSON aparte, sin cifrar, porque se necesitan antes de desbloquear.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use vault_core::domain::password_generation::password_generation_options::{
    DEFAULT_PASSPHRASE_SEPARATOR, DEFAULT_PASSPHRASE_WORD_COUNT, DEFAULT_PASSWORD_LENGTH,
};
use vault_core::infrastructure::storage::owner_only_files::write_owner_only_file;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GenerationStrategy {
    #[default]
    RandomCharacters,
    Passphrase,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RandomCharactersSettings {
    pub password_length: usize,
    pub includes_uppercase: bool,
    pub includes_lowercase: bool,
    pub includes_digits: bool,
    pub includes_symbols: bool,
    pub should_exclude_ambiguous_characters: bool,
}

impl Default for RandomCharactersSettings {
    fn default() -> Self {
        Self {
            password_length: DEFAULT_PASSWORD_LENGTH,
            includes_uppercase: true,
            includes_lowercase: true,
            includes_digits: true,
            includes_symbols: true,
            should_exclude_ambiguous_characters: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PassphraseSettings {
    pub word_count: usize,
    pub word_separator: String,
    pub should_capitalize_words: bool,
    pub should_append_number: bool,
}

impl Default for PassphraseSettings {
    fn default() -> Self {
        Self {
            word_count: DEFAULT_PASSPHRASE_WORD_COUNT,
            word_separator: DEFAULT_PASSPHRASE_SEPARATOR.into(),
            should_capitalize_words: true,
            should_append_number: true,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GeneratorSettings {
    pub active_strategy: GenerationStrategy,
    pub random_characters: RandomCharactersSettings,
    pub passphrase: PassphraseSettings,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UiPreferences {
    /// `None`: se usa el idioma del sistema.
    pub language_code: Option<String>,
    pub theme_preference: ThemePreference,
    pub generator_settings: GeneratorSettings,
}

pub struct UiPreferencesStore {
    preferences_file_path: PathBuf,
}

impl UiPreferencesStore {
    pub fn new(preferences_file_path: PathBuf) -> Self {
        Self { preferences_file_path }
    }

    /// Un archivo ausente o dañado no impide arrancar: se usan los valores por defecto.
    pub fn load(&self) -> UiPreferences {
        fs::read(&self.preferences_file_path)
            .ok()
            .and_then(|file_bytes| serde_json::from_slice(&file_bytes).ok())
            .unwrap_or_default()
    }

    /// Best-effort: perder el idioma o el tema elegidos no justifica interrumpir al usuario.
    pub fn save(&self, ui_preferences: &UiPreferences) {
        let Ok(serialized_preferences) = serde_json::to_vec_pretty(ui_preferences) else {
            return;
        };
        let _ = write_owner_only_file(&self.preferences_file_path, &serialized_preferences);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_or_corrupted_file_falls_back_to_defaults() {
        let preferences_file_path = std::env::temp_dir().join(format!("arca-ui-prefs-{}.json", std::process::id()));
        std::fs::write(&preferences_file_path, b"{ no es json").expect("escribir");

        let loaded_preferences = UiPreferencesStore::new(preferences_file_path.clone()).load();

        assert_eq!(loaded_preferences, UiPreferences::default());
        let _ = std::fs::remove_file(preferences_file_path);
    }

    #[test]
    fn saved_preferences_are_loaded_back() {
        let preferences_file_path =
            std::env::temp_dir().join(format!("arca-ui-prefs-roundtrip-{}.json", std::process::id()));
        let preferences_store = UiPreferencesStore::new(preferences_file_path.clone());
        let chosen_preferences = UiPreferences {
            language_code: Some("es".into()),
            theme_preference: ThemePreference::Dark,
            generator_settings: GeneratorSettings {
                active_strategy: GenerationStrategy::Passphrase,
                ..GeneratorSettings::default()
            },
        };

        preferences_store.save(&chosen_preferences);

        assert_eq!(preferences_store.load(), chosen_preferences);
        let _ = std::fs::remove_file(preferences_file_path);
    }
}
