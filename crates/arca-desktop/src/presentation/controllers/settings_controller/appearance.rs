//! Ajustes de apariencia: idioma y tema (por equipo, fuera de la bóveda).

use slint::ComponentHandle;

use crate::platform::system_color_scheme;
use crate::platform::ui_preferences_store::ThemePreference;
use crate::presentation::app_context::AppEvent;
use crate::presentation::translator::SUPPORTED_LANGUAGES;
use crate::{AppWindow, Theme, ThemeChoice};

use super::SettingsController;

pub(super) const THEME_CHOICES: [(ThemePreference, ThemeChoice, &str); 3] = [
    (ThemePreference::System, ThemeChoice::System, "settings.theme.system"),
    (ThemePreference::Light, ThemeChoice::Light, "settings.theme.light"),
    (ThemePreference::Dark, ThemeChoice::Dark, "settings.theme.dark"),
];

/// Con el tema «Sistema», la interfaz cambia en cuanto el sistema pasa a claro u oscuro.
pub(super) fn follow_system_color_scheme(app_window: &AppWindow) {
    let window_handle = app_window.as_weak();
    system_color_scheme::watch_changes(move |system_is_dark| {
        let _ = window_handle
            .upgrade_in_event_loop(move |app_window| app_window.global::<Theme>().set_system_is_dark(system_is_dark));
    });
}

impl SettingsController {
    pub(super) fn select_language(&self, language_position: i32) {
        let Some(language) =
            usize::try_from(language_position).ok().and_then(|position| SUPPORTED_LANGUAGES.get(position))
        else {
            return;
        };
        self.context.translator.borrow_mut().change_language(language.language_code);
        self.context.update_ui_preferences(|ui_preferences| {
            ui_preferences.language_code = Some(language.language_code.to_string());
        });
        self.context.events.publish(AppEvent::LanguageChanged);
    }

    pub(super) fn select_theme(&self, theme_position: i32) {
        let Some((theme_preference, _, _)) =
            usize::try_from(theme_position).ok().and_then(|position| THEME_CHOICES.get(position))
        else {
            return;
        };
        self.context.update_ui_preferences(|ui_preferences| ui_preferences.theme_preference = *theme_preference);
        self.apply_theme(*theme_preference);
    }

    pub(super) fn apply_theme(&self, theme_preference: ThemePreference) {
        let theme_choice = THEME_CHOICES
            .iter()
            .find(|(preference, _, _)| *preference == theme_preference)
            .map_or(ThemeChoice::System, |(_, theme_choice, _)| *theme_choice);
        if let Some(app_window) = self.context.window() {
            app_window.global::<Theme>().set_choice(theme_choice);
        }
    }
}
