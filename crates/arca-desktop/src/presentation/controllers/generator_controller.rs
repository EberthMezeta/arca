//! Generador de contraseñas: opciones guardadas por equipo, generación y uso dentro del editor.

use std::collections::BTreeSet;
use std::rc::Rc;

use slint::{ComponentHandle, SharedString};
use vault_core::domain::password_generation::password_generation_options::{
    CharacterSet, DEFAULT_PASSPHRASE_WORD_COUNT, DEFAULT_PASSWORD_LENGTH, PasswordGenerationOptions,
};
use vault_core::domain::password_strength::PasswordStrength;
use vault_core::domain::secret_text::SecretText;

use crate::platform::ui_preferences_store::{GenerationStrategy, GeneratorSettings};
use crate::presentation::app_context::{AppContext, AppEvent};
use crate::presentation::controllers::clipboard_feedback::{copy_with_feedback, report_error};
use crate::ui_error::UiError;
use crate::{AppWindow, Generator, ItemEditor};

/// Orden de los conjuntos tal como los numera la interfaz (`toggle-character-set`).
const CHARACTER_SET_ORDER: [CharacterSet; 4] = [CharacterSet::Uppercase, CharacterSet::Lowercase, CharacterSet::Digits, CharacterSet::Symbols];

pub struct GeneratorController {
    context: Rc<AppContext>,
}

fn generation_options_from(generator_settings: &GeneratorSettings) -> PasswordGenerationOptions {
    match generator_settings.active_strategy {
        GenerationStrategy::RandomCharacters => {
            let random_characters = &generator_settings.random_characters;
            let selected_sets = [
                random_characters.includes_uppercase,
                random_characters.includes_lowercase,
                random_characters.includes_digits,
                random_characters.includes_symbols,
            ];
            PasswordGenerationOptions::RandomCharacters {
                password_length: random_characters.password_length,
                character_sets: CHARACTER_SET_ORDER
                    .into_iter()
                    .zip(selected_sets)
                    .filter_map(|(character_set, is_selected)| is_selected.then_some(character_set))
                    .collect::<BTreeSet<_>>(),
                should_exclude_ambiguous_characters: random_characters.should_exclude_ambiguous_characters,
            }
        }
        GenerationStrategy::Passphrase => PasswordGenerationOptions::Passphrase {
            word_count: generator_settings.passphrase.word_count,
            word_separator: generator_settings.passphrase.word_separator.clone(),
            should_capitalize_words: generator_settings.passphrase.should_capitalize_words,
            should_append_number: generator_settings.passphrase.should_append_number,
        },
    }
}

impl GeneratorController {
    pub fn install(context: &Rc<AppContext>, app_window: &AppWindow) -> Rc<Self> {
        let controller = Rc::new(Self { context: Rc::clone(context) });
        let generator = app_window.global::<Generator>();

        let settings_controller = Rc::clone(&controller);
        generator.on_settings_changed(move || {
            settings_controller.store_settings_from_window();
            settings_controller.regenerate();
        });
        let toggling_controller = Rc::clone(&controller);
        generator.on_toggle_character_set(move |set_position| toggling_controller.toggle_character_set(set_position));
        let regenerating_controller = Rc::clone(&controller);
        generator.on_regenerate(move || regenerating_controller.regenerate());
        let copying_controller = Rc::clone(&controller);
        generator.on_copy_generated(move || copying_controller.copy_generated());
        let using_controller = Rc::clone(&controller);
        generator.on_use_generated(move || using_controller.use_generated());

        let event_controller = Rc::clone(&controller);
        context.events.subscribe(move |app_event| match app_event {
            AppEvent::SessionUnlocked => event_controller.regenerate(),
            AppEvent::SessionLocked => event_controller.publish_password(String::new()),
            _ => {}
        });

        controller.publish_settings(&context.ui_preferences().generator_settings);
        controller
    }

    fn publish_settings(&self, generator_settings: &GeneratorSettings) {
        let Some(app_window) = self.context.window() else {
            return;
        };
        let generator = app_window.global::<Generator>();
        let random_characters = &generator_settings.random_characters;
        let passphrase = &generator_settings.passphrase;
        generator.set_is_passphrase(generator_settings.active_strategy == GenerationStrategy::Passphrase);
        generator.set_password_length(i32::try_from(random_characters.password_length).unwrap_or(i32::MAX));
        generator.set_includes_uppercase(random_characters.includes_uppercase);
        generator.set_includes_lowercase(random_characters.includes_lowercase);
        generator.set_includes_digits(random_characters.includes_digits);
        generator.set_includes_symbols(random_characters.includes_symbols);
        generator.set_excludes_ambiguous(random_characters.should_exclude_ambiguous_characters);
        generator.set_word_count(i32::try_from(passphrase.word_count).unwrap_or(i32::MAX));
        generator.set_word_separator(passphrase.word_separator.clone().into());
        generator.set_capitalizes_words(passphrase.should_capitalize_words);
        generator.set_appends_number(passphrase.should_append_number);
    }

    fn store_settings_from_window(&self) {
        let Some(app_window) = self.context.window() else {
            return;
        };
        let generator = app_window.global::<Generator>();
        self.context.update_ui_preferences(|ui_preferences| {
            let generator_settings = &mut ui_preferences.generator_settings;
            generator_settings.active_strategy =
                if generator.get_is_passphrase() { GenerationStrategy::Passphrase } else { GenerationStrategy::RandomCharacters };
            let random_characters = &mut generator_settings.random_characters;
            random_characters.password_length = usize::try_from(generator.get_password_length()).unwrap_or(DEFAULT_PASSWORD_LENGTH);
            random_characters.should_exclude_ambiguous_characters = generator.get_excludes_ambiguous();
            let passphrase = &mut generator_settings.passphrase;
            passphrase.word_count = usize::try_from(generator.get_word_count()).unwrap_or(DEFAULT_PASSPHRASE_WORD_COUNT);
            passphrase.word_separator = generator.get_word_separator().to_string();
            passphrase.should_capitalize_words = generator.get_capitalizes_words();
            passphrase.should_append_number = generator.get_appends_number();
        });
    }

    /// Siempre queda al menos un conjunto de caracteres marcado.
    fn toggle_character_set(&self, set_position: i32) {
        let mut generator_settings = self.context.ui_preferences().generator_settings;
        let random_characters = &mut generator_settings.random_characters;
        let mut selected_sets = [
            random_characters.includes_uppercase,
            random_characters.includes_lowercase,
            random_characters.includes_digits,
            random_characters.includes_symbols,
        ];
        let Some(toggled_set) = usize::try_from(set_position).ok().filter(|position| *position < selected_sets.len()) else {
            return;
        };
        selected_sets[toggled_set] = !selected_sets[toggled_set];
        if !selected_sets.contains(&true) {
            return;
        }
        [
            random_characters.includes_uppercase,
            random_characters.includes_lowercase,
            random_characters.includes_digits,
            random_characters.includes_symbols,
        ] = selected_sets;
        self.context.update_ui_preferences(|ui_preferences| ui_preferences.generator_settings = generator_settings.clone());
        self.publish_settings(&generator_settings);
        self.regenerate();
    }

    fn regenerate(&self) {
        let generation_options = generation_options_from(&self.context.ui_preferences().generator_settings);
        match generation_options.into_strategy().generate_password() {
            Ok(generated_password) => self.publish_password(generated_password.expose_secret().to_string()),
            Err(vault_error) => report_error(&self.context, &UiError::from(vault_error)),
        }
    }

    fn publish_password(&self, generated_password: String) {
        let Some(app_window) = self.context.window() else {
            return;
        };
        let strength_score = if generated_password.is_empty() {
            -1
        } else {
            i32::from(PasswordStrength::estimate(&SecretText::new(generated_password.as_str())).score)
        };
        let generator = app_window.global::<Generator>();
        generator.set_generated_password(SharedString::from(generated_password));
        generator.set_strength_score(strength_score);
    }

    fn generated_password(&self) -> Option<SecretText> {
        let app_window = self.context.window()?;
        let generated_password = app_window.global::<Generator>().get_generated_password();
        (!generated_password.is_empty()).then(|| SecretText::new(generated_password.as_str()))
    }

    fn copy_generated(&self) {
        let Some(generated_password) = self.generated_password() else {
            return;
        };
        copy_with_feedback(&self.context, &self.context.text("fields.password"), |vault_service| {
            vault_service.copy_secret_to_clipboard(generated_password)
        });
    }

    /// Pasa la contraseña al editor y deja otra preparada para la próxima vez.
    fn use_generated(&self) {
        let (Some(app_window), Some(generated_password)) = (self.context.window(), self.generated_password()) else {
            return;
        };
        let item_editor = app_window.global::<ItemEditor>();
        item_editor.set_password(generated_password.expose_secret().into());
        item_editor.set_is_generator_open(false);
        self.regenerate();
    }
}
