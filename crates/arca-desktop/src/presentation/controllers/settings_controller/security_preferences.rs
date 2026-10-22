//! Preferencias de seguridad guardadas en la bóveda: bloqueo automático, portapapeles e historial.

use slint::ComponentHandle;
use vault_core::domain::vault_preferences::VaultPreferences;

use crate::presentation::controllers::clipboard_feedback::report_error;
use crate::ui_error::UiResult;
use crate::{Settings, VaultSection};

use super::{SettingsController, to_string_model};
use vault_core::application::vault_service::VaultSession;

const AUTO_LOCK_SECONDS_CHOICES: [u64; 6] = [60, 300, 900, 1800, 3600, 14400];
const CLIPBOARD_CLEAR_SECONDS_CHOICES: [u64; 4] = [10, 30, 60, 120];
const PASSWORD_HISTORY_LIMIT_CHOICES: [u64; 5] = [0, 5, 10, 20, 50];

/// Unidades de tiempo de mayor a menor: se usa la primera que cabe.
const DURATION_UNITS: [(u64, &str); 3] = [(3600, "settings.hours"), (60, "settings.minutes"), (1, "settings.seconds")];

/// Opciones visibles de cada desplegable de seguridad. Si el valor guardado no es una de las
/// opciones habituales (p. ej. importado), se añade para no perderlo al mostrarlo.
#[derive(Default)]
pub(super) struct PreferenceChoices {
    loaded_preferences: Option<VaultPreferences>,
    auto_lock_seconds: Vec<u64>,
    clipboard_clear_seconds: Vec<u64>,
    history_limits: Vec<u64>,
}

fn choices_including(standard_choices: &[u64], current_value: u64) -> Vec<u64> {
    let mut choices = standard_choices.to_vec();
    if !choices.contains(&current_value) {
        choices.push(current_value);
        choices.sort_unstable();
    }
    choices
}

fn position_of(choices: &[u64], value: u64) -> i32 {
    choices.iter().position(|choice| *choice == value).and_then(|position| i32::try_from(position).ok()).unwrap_or(0)
}

fn choice_at(choices: &[u64], position: i32) -> Option<u64> {
    usize::try_from(position).ok().and_then(|position| choices.get(position).copied())
}

impl SettingsController {
    fn format_duration(&self, total_seconds: u64) -> String {
        let (unit_seconds, unit_key) = DURATION_UNITS
            .iter()
            .find(|(unit_seconds, _)| total_seconds >= *unit_seconds)
            .copied()
            .unwrap_or((1, "settings.seconds"));
        self.context.count_text(unit_key, i64::try_from(total_seconds / unit_seconds).unwrap_or(0), &[])
    }

    /// Solo con la sección abierta: las preferencias viven dentro de la bóveda.
    pub(super) fn refresh_preferences(&self) {
        if self.context.navigator.snapshot().section != VaultSection::Settings {
            return;
        }
        let Ok(vault_preferences) =
            self.context.shared_vault.lock_service().and_then(|vault_service| Ok(vault_service.preferences()?))
        else {
            return;
        };
        let history_limit = u64::try_from(vault_preferences.password_history_limit).unwrap_or(0);
        let preference_choices = PreferenceChoices {
            auto_lock_seconds: choices_including(
                &AUTO_LOCK_SECONDS_CHOICES,
                vault_preferences.auto_lock_after_idle_seconds,
            ),
            clipboard_clear_seconds: choices_including(
                &CLIPBOARD_CLEAR_SECONDS_CHOICES,
                vault_preferences.clipboard_clear_after_seconds,
            ),
            history_limits: choices_including(&PASSWORD_HISTORY_LIMIT_CHOICES, history_limit),
            loaded_preferences: Some(vault_preferences.clone()),
        };
        let auto_lock_labels =
            preference_choices.auto_lock_seconds.iter().map(|seconds| self.format_duration(*seconds)).collect();
        let clipboard_labels =
            preference_choices.clipboard_clear_seconds.iter().map(|seconds| self.format_duration(*seconds)).collect();
        let history_labels = preference_choices
            .history_limits
            .iter()
            .map(|limit| self.context.count_text("settings.historyEntries", i64::try_from(*limit).unwrap_or(0), &[]))
            .collect();
        self.with_settings(|settings| {
            settings.set_auto_lock_labels(to_string_model(auto_lock_labels));
            settings.set_auto_lock_index(position_of(
                &preference_choices.auto_lock_seconds,
                vault_preferences.auto_lock_after_idle_seconds,
            ));
            settings.set_clipboard_clear_labels(to_string_model(clipboard_labels));
            settings.set_clipboard_clear_index(position_of(
                &preference_choices.clipboard_clear_seconds,
                vault_preferences.clipboard_clear_after_seconds,
            ));
            settings.set_history_limit_labels(to_string_model(history_labels));
            settings.set_history_limit_index(position_of(&preference_choices.history_limits, history_limit));
            settings.set_locks_on_minimize(vault_preferences.should_lock_on_window_minimize);
            settings.set_preferences_loaded(true);
        });
        *self.preference_choices.borrow_mut() = preference_choices;
    }

    pub(super) fn save_preferences(&self) {
        self.context.with_window(|app_window| {
            let settings = app_window.global::<Settings>();
            let updated_preferences = {
                let preference_choices = self.preference_choices.borrow();
                let Some(loaded_preferences) = preference_choices.loaded_preferences.clone() else {
                    return;
                };
                VaultPreferences {
                    auto_lock_after_idle_seconds: choice_at(
                        &preference_choices.auto_lock_seconds,
                        settings.get_auto_lock_index(),
                    )
                    .unwrap_or(loaded_preferences.auto_lock_after_idle_seconds),
                    clipboard_clear_after_seconds: choice_at(
                        &preference_choices.clipboard_clear_seconds,
                        settings.get_clipboard_clear_index(),
                    )
                    .unwrap_or(loaded_preferences.clipboard_clear_after_seconds),
                    password_history_limit: choice_at(
                        &preference_choices.history_limits,
                        settings.get_history_limit_index(),
                    )
                    .and_then(|limit| usize::try_from(limit).ok())
                    .unwrap_or(loaded_preferences.password_history_limit),
                    should_lock_on_window_minimize: settings.get_locks_on_minimize(),
                }
            };
            let update_result: UiResult<_> = self
                .context
                .shared_vault
                .lock_service()
                .and_then(|mut vault_service| Ok(vault_service.update_preferences(updated_preferences)?));
            match update_result {
                Ok(_) => self.report_success("settings.saved"),
                Err(ui_error) => report_error(&self.context, &ui_error),
            }
            // Tanto si se guardó como si no, la vista refleja lo que realmente hay en la bóveda.
            self.refresh_preferences();
        });
    }
}
