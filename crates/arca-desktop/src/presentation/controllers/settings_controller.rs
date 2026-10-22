//! Ajustes: idioma y tema (por equipo), preferencias de seguridad (en la bóveda),
//! contraseña maestra, importar/exportar y ubicación del archivo.

mod appearance;
mod master_password;
mod security_preferences;
mod transfer;

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use vault_core::application::import_export_vault::ExportedFile;

use crate::presentation::app_context::{AppContext, AppEvent};
use crate::presentation::translator::SUPPORTED_LANGUAGES;
use crate::{AppWindow, Settings};
use appearance::{THEME_CHOICES, follow_system_color_scheme};
use security_preferences::PreferenceChoices;
use transfer::{EXPORT_FORMATS, IMPORT_FORMATS};

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
