//! Ajustes: idioma y tema (por equipo), preferencias de seguridad (en la bóveda),
//! contraseña maestra, importar/exportar y ubicación del archivo.

use std::cell::RefCell;
use std::rc::Rc;

use chrono::Local;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use vault_core::application::import_export_vault::{ExportedFile, ImportLabels, parse_import_file};
use vault_core::domain::ports::vault_exporter::ExportFormat;
use vault_core::domain::ports::vault_importer::ImportFormat;
use vault_core::domain::secret_text::SecretText;
use vault_core::domain::vault_preferences::VaultPreferences;
use vault_core::infrastructure::storage::owner_only_files::write_owner_only_file;

use crate::composition_root::MAXIMUM_IMPORT_FILE_SIZE_BYTES;
use crate::platform::bounded_file_reader::read_user_file;
use crate::platform::file_dialogs::{ask_file_to_open, ask_file_to_save};
use crate::platform::system_color_scheme;
use crate::platform::ui_preferences_store::ThemePreference;
use crate::presentation::app_context::{AppContext, AppEvent};
use crate::presentation::background_task::{run_in_background, run_with_vault_in_background, spawn_ui_task};
use crate::presentation::controllers::clipboard_feedback::report_error;
use crate::presentation::error_messages::describe_error;
use crate::presentation::translator::SUPPORTED_LANGUAGES;
use crate::ui_error::{UiError, UiResult};
use crate::{AppWindow, Settings, Theme, ThemeChoice, VaultSection};

const AUTO_LOCK_SECONDS_CHOICES: [u64; 6] = [60, 300, 900, 1800, 3600, 14400];
const CLIPBOARD_CLEAR_SECONDS_CHOICES: [u64; 4] = [10, 30, 60, 120];
const PASSWORD_HISTORY_LIMIT_CHOICES: [u64; 5] = [0, 5, 10, 20, 50];

const THEME_CHOICES: [(ThemePreference, ThemeChoice, &str); 3] = [
    (ThemePreference::System, ThemeChoice::System, "settings.theme.system"),
    (ThemePreference::Light, ThemeChoice::Light, "settings.theme.light"),
    (ThemePreference::Dark, ThemeChoice::Dark, "settings.theme.dark"),
];

/// Unidades de tiempo de mayor a menor: se usa la primera que cabe.
const DURATION_UNITS: [(u64, &str); 3] = [(3600, "settings.hours"), (60, "settings.minutes"), (1, "settings.seconds")];

struct TransferFormatDescription<Format> {
    transfer_format: Format,
    label_key: &'static str,
    hint_key: &'static str,
}

const IMPORT_FORMATS: [TransferFormatDescription<ImportFormat>; 4] = [
    TransferFormatDescription {
        transfer_format: ImportFormat::BitwardenJson,
        label_key: "transfer.formats.bitwardenJson",
        hint_key: "transfer.importHints.bitwardenJson",
    },
    TransferFormatDescription {
        transfer_format: ImportFormat::BitwardenCsv,
        label_key: "transfer.formats.bitwardenCsv",
        hint_key: "transfer.importHints.bitwardenCsv",
    },
    TransferFormatDescription {
        transfer_format: ImportFormat::BrowserCsv,
        label_key: "transfer.formats.browserCsv",
        hint_key: "transfer.importHints.browserCsv",
    },
    TransferFormatDescription {
        transfer_format: ImportFormat::EncryptedVaultExport,
        label_key: "transfer.formats.encryptedVaultExport",
        hint_key: "transfer.importHints.encryptedVaultExport",
    },
];

const EXPORT_FORMATS: [TransferFormatDescription<ExportFormat>; 3] = [
    TransferFormatDescription {
        transfer_format: ExportFormat::EncryptedVaultExport,
        label_key: "transfer.formats.encryptedVaultExport",
        hint_key: "",
    },
    TransferFormatDescription {
        transfer_format: ExportFormat::BitwardenJson,
        label_key: "transfer.formats.bitwardenJson",
        hint_key: "",
    },
    TransferFormatDescription {
        transfer_format: ExportFormat::BitwardenCsv,
        label_key: "transfer.formats.bitwardenCsv",
        hint_key: "",
    },
];

/// Opciones visibles de cada desplegable de seguridad. Si el valor guardado no es una de las
/// opciones habituales (p. ej. importado), se añade para no perderlo al mostrarlo.
#[derive(Default)]
struct PreferenceChoices {
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

fn to_string_model(labels: Vec<String>) -> ModelRc<SharedString> {
    ModelRc::new(VecModel::from(labels.into_iter().map(SharedString::from).collect::<Vec<_>>()))
}

pub struct SettingsController {
    context: Rc<AppContext>,
    preference_choices: RefCell<PreferenceChoices>,
    /// Exportación ya generada que espera a que el usuario elija dónde guardarla. Puede ir en
    /// texto plano: si la bóveda se bloquea mientras tanto, se descarta (y se borra de memoria).
    pending_export: RefCell<Option<ExportedFile>>,
}

/// Con el tema «Sistema», la interfaz cambia en cuanto el sistema pasa a claro u oscuro.
fn follow_system_color_scheme(app_window: &AppWindow) {
    let window_handle = app_window.as_weak();
    system_color_scheme::watch_changes(move |system_is_dark| {
        let _ = window_handle.upgrade_in_event_loop(move |app_window| app_window.global::<Theme>().set_system_is_dark(system_is_dark));
    });
}

impl SettingsController {
    pub fn install(context: &Rc<AppContext>, app_window: &AppWindow) -> Rc<Self> {
        let controller =
            Rc::new(Self { context: Rc::clone(context), preference_choices: RefCell::default(), pending_export: RefCell::default() });
        controller.bind_callbacks(app_window);

        let event_controller = Rc::clone(&controller);
        context.events.subscribe(move |app_event| match app_event {
            AppEvent::SectionChanged => event_controller.refresh_preferences(),
            AppEvent::LanguageChanged => {
                event_controller.publish_static_options();
                event_controller.refresh_preferences();
            }
            AppEvent::SessionLocked => event_controller.clear_secrets(),
            _ => {}
        });

        controller.apply_theme(context.ui_preferences().theme_preference);
        follow_system_color_scheme(app_window);
        controller.publish_static_options();
        app_window.global::<Settings>().set_vault_location(context.vault_file_path.display().to_string().into());
        controller.select_export_format(0);
        controller.select_import_format(0);
        controller
    }

    fn bind_callbacks(self: &Rc<Self>, app_window: &AppWindow) {
        let settings = app_window.global::<Settings>();
        let language_controller = Rc::clone(self);
        settings.on_language_selected(move |language_position| language_controller.select_language(language_position));
        let theme_controller = Rc::clone(self);
        settings.on_theme_selected(move |theme_position| theme_controller.select_theme(theme_position));
        let preferences_controller = Rc::clone(self);
        settings.on_preferences_changed(move || preferences_controller.save_preferences());
        let password_controller = Rc::clone(self);
        settings.on_change_master_password(move || password_controller.change_master_password());
        let import_format_controller = Rc::clone(self);
        settings.on_import_format_selected(move |format_position| import_format_controller.select_import_format(format_position));
        let import_controller = Rc::clone(self);
        settings.on_import_file(move || import_controller.import_file());
        let export_format_controller = Rc::clone(self);
        settings.on_export_format_selected(move |format_position| export_format_controller.select_export_format(format_position));
        let export_controller = Rc::clone(self);
        settings.on_export_file(move || export_controller.export_file());
    }

    fn with_settings(&self, apply: impl FnOnce(&Settings<'_>)) {
        if let Some(app_window) = self.context.window() {
            apply(&app_window.global::<Settings>());
        }
    }

    fn report_success(&self, translation_key: &str) {
        self.context.toasts.success(self.context.text(translation_key));
    }

    /// Opciones que solo dependen del idioma (no de la bóveda).
    fn publish_static_options(&self) {
        let active_language_code = self.context.translator.borrow().active_language_code();
        let theme_preference = self.context.ui_preferences().theme_preference;
        let language_names = SUPPORTED_LANGUAGES.iter().map(|language| language.native_name.to_string()).collect();
        let theme_names = THEME_CHOICES.iter().map(|(_, _, label_key)| self.context.text(label_key)).collect();
        let import_names = IMPORT_FORMATS.iter().map(|description| self.context.text(description.label_key)).collect();
        let export_names = EXPORT_FORMATS.iter().map(|description| self.context.text(description.label_key)).collect();
        let language_position = SUPPORTED_LANGUAGES.iter().position(|language| language.language_code == active_language_code).unwrap_or(0);
        let theme_position = THEME_CHOICES.iter().position(|(preference, _, _)| *preference == theme_preference).unwrap_or(0);
        self.with_settings(|settings| {
            settings.set_language_names(to_string_model(language_names));
            settings.set_language_index(i32::try_from(language_position).unwrap_or(0));
            settings.set_theme_names(to_string_model(theme_names));
            settings.set_theme_index(i32::try_from(theme_position).unwrap_or(0));
            settings.set_import_format_names(to_string_model(import_names));
            settings.set_export_format_names(to_string_model(export_names));
            let import_position = usize::try_from(settings.get_import_format_index()).unwrap_or(0);
            let import_hint = IMPORT_FORMATS.get(import_position).map(|description| self.context.text(description.hint_key)).unwrap_or_default();
            settings.set_import_hint(import_hint.into());
        });
    }

    // ── Apariencia ────────────────────────────────────────────────────────────

    fn select_language(&self, language_position: i32) {
        let Some(language) = usize::try_from(language_position).ok().and_then(|position| SUPPORTED_LANGUAGES.get(position)) else {
            return;
        };
        self.context.translator.borrow_mut().change_language(language.language_code);
        self.context.update_ui_preferences(|ui_preferences| ui_preferences.language_code = Some(language.language_code.to_string()));
        self.context.events.publish(AppEvent::LanguageChanged);
    }

    fn select_theme(&self, theme_position: i32) {
        let Some((theme_preference, _, _)) = usize::try_from(theme_position).ok().and_then(|position| THEME_CHOICES.get(position)) else {
            return;
        };
        self.context.update_ui_preferences(|ui_preferences| ui_preferences.theme_preference = *theme_preference);
        self.apply_theme(*theme_preference);
    }

    fn apply_theme(&self, theme_preference: ThemePreference) {
        let theme_choice = THEME_CHOICES
            .iter()
            .find(|(preference, _, _)| *preference == theme_preference)
            .map_or(ThemeChoice::System, |(_, theme_choice, _)| *theme_choice);
        if let Some(app_window) = self.context.window() {
            app_window.global::<Theme>().set_choice(theme_choice);
        }
    }

    // ── Seguridad ─────────────────────────────────────────────────────────────

    fn format_duration(&self, total_seconds: u64) -> String {
        let (unit_seconds, unit_key) =
            DURATION_UNITS.iter().find(|(unit_seconds, _)| total_seconds >= *unit_seconds).copied().unwrap_or((1, "settings.seconds"));
        self.context.count_text(unit_key, i64::try_from(total_seconds / unit_seconds).unwrap_or(0), &[])
    }

    /// Solo con la sección abierta: las preferencias viven dentro de la bóveda.
    fn refresh_preferences(&self) {
        if self.context.navigator.snapshot().section != VaultSection::Settings {
            return;
        }
        let Ok(vault_preferences) = self.context.shared_vault.lock_service().and_then(|vault_service| Ok(vault_service.preferences()?)) else {
            return;
        };
        let history_limit = u64::try_from(vault_preferences.password_history_limit).unwrap_or(0);
        let preference_choices = PreferenceChoices {
            auto_lock_seconds: choices_including(&AUTO_LOCK_SECONDS_CHOICES, vault_preferences.auto_lock_after_idle_seconds),
            clipboard_clear_seconds: choices_including(&CLIPBOARD_CLEAR_SECONDS_CHOICES, vault_preferences.clipboard_clear_after_seconds),
            history_limits: choices_including(&PASSWORD_HISTORY_LIMIT_CHOICES, history_limit),
            loaded_preferences: Some(vault_preferences.clone()),
        };
        let auto_lock_labels = preference_choices.auto_lock_seconds.iter().map(|seconds| self.format_duration(*seconds)).collect();
        let clipboard_labels = preference_choices.clipboard_clear_seconds.iter().map(|seconds| self.format_duration(*seconds)).collect();
        let history_labels = preference_choices
            .history_limits
            .iter()
            .map(|limit| self.context.count_text("settings.historyEntries", i64::try_from(*limit).unwrap_or(0), &[]))
            .collect();
        self.with_settings(|settings| {
            settings.set_auto_lock_labels(to_string_model(auto_lock_labels));
            settings.set_auto_lock_index(position_of(&preference_choices.auto_lock_seconds, vault_preferences.auto_lock_after_idle_seconds));
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

    fn save_preferences(&self) {
        self.context.with_window(|app_window| {
            let settings = app_window.global::<Settings>();
            let updated_preferences = {
                let preference_choices = self.preference_choices.borrow();
                let Some(loaded_preferences) = preference_choices.loaded_preferences.clone() else {
                    return;
                };
                VaultPreferences {
                    auto_lock_after_idle_seconds: choice_at(&preference_choices.auto_lock_seconds, settings.get_auto_lock_index())
                        .unwrap_or(loaded_preferences.auto_lock_after_idle_seconds),
                    clipboard_clear_after_seconds: choice_at(&preference_choices.clipboard_clear_seconds, settings.get_clipboard_clear_index())
                        .unwrap_or(loaded_preferences.clipboard_clear_after_seconds),
                    password_history_limit: choice_at(&preference_choices.history_limits, settings.get_history_limit_index())
                        .and_then(|limit| usize::try_from(limit).ok())
                        .unwrap_or(loaded_preferences.password_history_limit),
                    should_lock_on_window_minimize: settings.get_locks_on_minimize(),
                }
            };
            let update_result: UiResult<_> =
                self.context.shared_vault.lock_service().and_then(|mut vault_service| Ok(vault_service.update_preferences(updated_preferences)?));
            match update_result {
                Ok(_) => self.report_success("settings.saved"),
                Err(ui_error) => report_error(&self.context, &ui_error),
            }
            // Tanto si se guardó como si no, la vista refleja lo que realmente hay en la bóveda.
            self.refresh_preferences();
        });
    }

    // ── Contraseña maestra ────────────────────────────────────────────────────

    fn change_master_password(self: &Rc<Self>) {
        self.context.with_window(|app_window| {
            let settings = app_window.global::<Settings>();
            let passwords_match = settings.get_new_master_password() == settings.get_confirmed_master_password();
            if settings.get_is_changing_master_password() || !passwords_match {
                return;
            }
            let current_master_password = SecretText::new(settings.get_current_master_password().as_str());
            let new_master_password = SecretText::new(settings.get_new_master_password().as_str());
            if current_master_password.is_empty() || new_master_password.is_empty() {
                return;
            }
            settings.set_current_master_password(SharedString::new());
            settings.set_is_changing_master_password(true);
            settings.set_master_password_error(SharedString::new());

            let controller = Rc::clone(self);
            spawn_ui_task(async move {
                let change_result = run_with_vault_in_background(&controller.context.shared_vault, move |vault_service| {
                    Ok(vault_service.change_master_password(&current_master_password, &new_master_password)?)
                })
                .await;
                let error_message = change_result.as_ref().err().map(|ui_error| describe_error(&controller.context.translator.borrow(), ui_error));
                controller.with_settings(|settings| {
                    settings.set_is_changing_master_password(false);
                    if let Some(error_message) = &error_message {
                        settings.set_master_password_error(error_message.clone().into());
                    } else {
                        settings.set_new_master_password(SharedString::new());
                        settings.set_confirmed_master_password(SharedString::new());
                    }
                });
                if error_message.is_none() {
                    controller.report_success("settings.masterPasswordChanged");
                }
            });
        });
    }

    // ── Importar ──────────────────────────────────────────────────────────────

    fn select_import_format(&self, format_position: i32) {
        let Some(description) = usize::try_from(format_position).ok().and_then(|position| IMPORT_FORMATS.get(position)) else {
            return;
        };
        self.with_settings(|settings| {
            settings.set_import_format_index(format_position);
            settings.set_import_hint(self.context.text(description.hint_key).into());
            settings.set_import_requires_password(description.transfer_format.traits().requires_file_password);
        });
    }

    fn import_file(self: &Rc<Self>) {
        self.context.with_window(|app_window| {
            let settings = app_window.global::<Settings>();
            let Some(description) = usize::try_from(settings.get_import_format_index()).ok().and_then(|position| IMPORT_FORMATS.get(position))
            else {
                return;
            };
            if settings.get_is_importing() {
                return;
            }
            let import_format = description.transfer_format;
            let file_password = import_format
                .traits()
                .requires_file_password
                .then(|| SecretText::new(settings.get_import_file_password().as_str()));
            settings.set_is_importing(true);

            // Lo que la importación escribe en la bóveda, en el idioma de la interfaz.
            let import_labels = ImportLabels {
                untitled_item_title: self.context.text("transfer.untitledItem"),
                invalid_totp_field_label: self.context.text("transfer.invalidTotpField"),
            };
            let controller = Rc::clone(self);
            spawn_ui_task(async move {
                let dialog_title = controller.context.text("transfer.chooseImportFile");
                let Some(chosen_path) = ask_file_to_open(&dialog_title, None).await else {
                    return controller.with_settings(|settings| settings.set_is_importing(false));
                };
                // Leer y descifrar el archivo (una copia `.arca` puede pedir un Argon2id de decenas de
                // segundos) se hace sin tomar la bóveda: así el bloqueo nunca tiene que esperar.
                let transfer_formats = controller.context.shared_vault.lock_service().map(|vault_service| vault_service.transfer_format_registry());
                let parse_result = match transfer_formats {
                    Ok(transfer_formats) => {
                        run_in_background(move || {
                            let file_bytes = read_user_file(&chosen_path, MAXIMUM_IMPORT_FILE_SIZE_BYTES, UiError::import_file_too_large())?;
                            Ok(parse_import_file(&transfer_formats, import_format, &file_bytes, file_password.as_ref())?)
                        })
                        .await
                    }
                    Err(ui_error) => Err(ui_error),
                };
                let import_result = match parse_result {
                    Ok(imported_vault_data) => {
                        run_with_vault_in_background(&controller.context.shared_vault, move |vault_service| {
                            Ok(vault_service.merge_imported_data(imported_vault_data, &import_labels)?)
                        })
                        .await
                    }
                    Err(ui_error) => Err(ui_error),
                };
                controller.with_settings(|settings| settings.set_is_importing(false));
                match import_result {
                    Ok(import_summary) => {
                        controller.with_settings(|settings| settings.set_import_file_password(SharedString::new()));
                        controller.context.events.publish(AppEvent::VaultDataChanged);
                        let imported_count = i64::try_from(import_summary.imported_item_count).unwrap_or(i64::MAX);
                        let folder_count = import_summary.created_folder_count.to_string();
                        controller.context.toasts.success(controller.context.count_text(
                            "transfer.imported",
                            imported_count,
                            &[("folderCount", &folder_count)],
                        ));
                    }
                    Err(ui_error) => report_error(&controller.context, &ui_error),
                }
            });
        });
    }

    // ── Exportar ──────────────────────────────────────────────────────────────

    fn select_export_format(&self, format_position: i32) {
        let Some(description) = usize::try_from(format_position).ok().and_then(|position| EXPORT_FORMATS.get(position)) else {
            return;
        };
        self.with_settings(|settings| {
            settings.set_export_format_index(format_position);
            settings.set_export_requires_password(description.transfer_format.traits().requires_file_password);
            settings.set_export_is_plaintext(description.transfer_format.traits().is_plaintext);
            settings.set_plaintext_risk_acknowledged(false);
        });
    }

    /// Primero se comprueba la contraseña maestra (y se cifra); luego se pregunta dónde guardar.
    fn export_file(self: &Rc<Self>) {
        self.context.with_window(|app_window| {
            let settings = app_window.global::<Settings>();
            let Some(description) = usize::try_from(settings.get_export_format_index()).ok().and_then(|position| EXPORT_FORMATS.get(position))
            else {
                return;
            };
            if settings.get_is_exporting() {
                return;
            }
            let export_format = description.transfer_format;
            let master_password = SecretText::new(settings.get_export_master_password().as_str());
            let file_password = export_format
                .traits()
                .requires_file_password
                .then(|| SecretText::new(settings.get_export_file_password().as_str()));
            settings.set_export_master_password(SharedString::new());
            settings.set_is_exporting(true);

            let controller = Rc::clone(self);
            spawn_ui_task(async move {
                let export_result = run_with_vault_in_background(&controller.context.shared_vault, move |vault_service| {
                    Ok(vault_service.export_vault(&master_password, export_format, file_password.as_ref())?)
                })
                .await;
                let exported_file = match export_result {
                    Ok(exported_file) => exported_file,
                    Err(ui_error) => {
                        controller.with_settings(|settings| settings.set_is_exporting(false));
                        return report_error(&controller.context, &ui_error);
                    }
                };
                let suggested_file_name =
                    format!("arca-export-{}.{}", Local::now().format("%Y-%m-%d"), exported_file.suggested_file_extension);
                *controller.pending_export.borrow_mut() = Some(exported_file);
                let dialog_title = controller.context.text("transfer.chooseExportFile");
                let destination_path = ask_file_to_save(&dialog_title, &suggested_file_name).await;
                let pending_export = controller.pending_export.borrow_mut().take();
                let (Some(destination_path), Some(exported_file)) = (destination_path, pending_export) else {
                    return controller.with_settings(|settings| settings.set_is_exporting(false));
                };
                let write_result = run_in_background(move || {
                    write_owner_only_file(&destination_path, &exported_file.file_bytes).map_err(UiError::file_write)
                })
                .await;
                controller.with_settings(|settings| settings.set_is_exporting(false));
                match write_result {
                    Ok(()) => {
                        controller.with_settings(|settings| {
                            settings.set_export_file_password(SharedString::new());
                            settings.set_export_confirmed_password(SharedString::new());
                            settings.set_plaintext_risk_acknowledged(false);
                        });
                        controller.report_success("transfer.exported");
                    }
                    Err(ui_error) => report_error(&controller.context, &ui_error),
                }
            });
        });
    }

    /// Al bloquear no puede quedar ninguna contraseña escrita en los formularios.
    fn clear_secrets(&self) {
        self.pending_export.borrow_mut().take();
        self.with_settings(|settings| {
            for clear_field in [
                Settings::set_current_master_password,
                Settings::set_new_master_password,
                Settings::set_confirmed_master_password,
                Settings::set_import_file_password,
                Settings::set_export_file_password,
                Settings::set_export_confirmed_password,
                Settings::set_export_master_password,
            ] {
                clear_field(settings, SharedString::new());
            }
            settings.set_master_password_error(SharedString::new());
            settings.set_preferences_loaded(false);
        });
    }
}
