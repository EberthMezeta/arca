//! Cambio de la contraseña maestra.

use std::rc::Rc;

use slint::{ComponentHandle, SharedString};
use vault_core::domain::secret_text::SecretText;

use crate::presentation::background_task::{run_with_vault_in_background, spawn_ui_task};
use crate::presentation::error_messages::describe_error;
use crate::Settings;

use super::SettingsController;
use vault_core::application::vault_service::VaultSession;

impl SettingsController {
    pub(super) fn change_master_password(self: &Rc<Self>) {
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
}
