//! Adjuntos del elemento abierto: añadir, guardar una copia descifrada (con reautenticación) y quitar.

use std::rc::Rc;

use slint::ComponentHandle;
use uuid::Uuid;
use vault_core::domain::secret_text::SecretText;
use vault_core::domain::vault_error::VaultError;
use vault_core::infrastructure::storage::owner_only_files::write_owner_only_file;

use super::ItemDetailsController;
use crate::ItemDetails;
use crate::composition_root::MAXIMUM_ATTACHMENT_SIZE_BYTES;
use crate::platform::bounded_file_reader::{BoundedReadError, read_file_with_size_limit};
use crate::platform::file_dialogs::{ask_file_to_open, ask_file_to_save};
use crate::presentation::app_context::AppEvent;
use crate::presentation::background_task::{run_in_background, run_with_vault_in_background, spawn_ui_task};
use crate::presentation::controllers::clipboard_feedback::report_error;
use crate::presentation::controllers::dialog_controller::PromptRequest;
use crate::presentation::error_messages::describe_error;
use crate::ui_error::UiError;

impl ItemDetailsController {
    fn set_adding_attachment(&self, is_adding_attachment: bool) {
        self.is_adding_attachment.set(is_adding_attachment);
        if let Some(app_window) = self.context.window() {
            app_window.global::<ItemDetails>().set_is_adding_attachment(is_adding_attachment);
        }
    }

    pub(super) fn add_attachment(self: &Rc<Self>) {
        let Some(item_id) = self.displayed_item_id() else {
            return;
        };
        if self.is_adding_attachment.get() {
            return;
        }
        self.set_adding_attachment(true);
        let controller = Rc::clone(self);
        spawn_ui_task(async move {
            let dialog_title = controller.context.text("attachments.chooseFile");
            let Some(chosen_path) = ask_file_to_open(&dialog_title, None).await else {
                return controller.set_adding_attachment(false);
            };
            let file_name = chosen_path.file_name().map(|file_name| file_name.to_string_lossy().into_owned()).unwrap_or_default();
            let attaching_file_name = file_name.clone();
            let attach_result = run_with_vault_in_background(&controller.context.shared_vault, move |vault_service| {
                let file_bytes = read_file_with_size_limit(&chosen_path, MAXIMUM_ATTACHMENT_SIZE_BYTES).map_err(|read_error| match read_error {
                    BoundedReadError::TooLarge => UiError::from(VaultError::AttachmentTooLarge { maximum_size_bytes: MAXIMUM_ATTACHMENT_SIZE_BYTES }),
                    BoundedReadError::Unreadable(io_error) => UiError::file_access(io_error),
                })?;
                vault_service.add_attachment(item_id, &attaching_file_name, &file_bytes)?;
                Ok(())
            })
            .await;
            controller.set_adding_attachment(false);
            match attach_result {
                Ok(()) => {
                    controller.context.events.publish(AppEvent::VaultDataChanged);
                    controller.context.toasts.success(controller.context.text_with("attachments.added", &[("fileName", &file_name)]));
                }
                Err(ui_error) => report_error(&controller.context, &ui_error),
            }
        });
    }

    fn attachment_at(&self, attachment_position: i32) -> Option<(Uuid, Uuid, String)> {
        let attachment_position = usize::try_from(attachment_position).ok()?;
        let details_state = self.details_state.borrow();
        let vault_item = details_state.displayed_item.as_ref()?;
        let attachment = vault_item.attachments.get(attachment_position)?;
        Some((vault_item.item_id, attachment.attachment_id, attachment.file_name.clone()))
    }

    /// Guardar un adjunto descifrado lo saca del cifrado: antes se pide la contraseña maestra.
    pub(super) fn ask_to_save_attachment(self: &Rc<Self>, attachment_position: i32) {
        let Some((item_id, attachment_id, _)) = self.attachment_at(attachment_position) else {
            return;
        };
        let controller = Rc::clone(self);
        self.context.dialogs.ask_for_text(PromptRequest {
            title: self.context.text("attachments.saveCopy"),
            label: self.context.text("attachments.masterPassword"),
            hint: self.context.text("attachments.masterPasswordHint"),
            confirm_label: self.context.text("attachments.saveConfirm"),
            initial_value: String::new(),
            is_secret: true,
            on_submit: Rc::new(move |master_password: SecretText| controller.save_attachment(item_id, attachment_id, master_password)),
        });
    }

    fn save_attachment(self: &Rc<Self>, item_id: Uuid, attachment_id: Uuid, master_password: SecretText) {
        self.context.dialogs.set_prompt_busy(true);
        let controller = Rc::clone(self);
        spawn_ui_task(async move {
            let decryption_result = run_with_vault_in_background(&controller.context.shared_vault, move |vault_service| {
                Ok(vault_service.read_attachment(&master_password, item_id, attachment_id)?)
            })
            .await;
            let decrypted_attachment = match decryption_result {
                Ok(decrypted_attachment) => decrypted_attachment,
                Err(ui_error) => {
                    let error_message = describe_error(&controller.context.translator.borrow(), &ui_error);
                    return controller.context.dialogs.show_prompt_error(error_message);
                }
            };
            controller.context.dialogs.close_prompt();

            let dialog_title = controller.context.text("attachments.saveCopy");
            let Some(destination_path) = ask_file_to_save(&dialog_title, &decrypted_attachment.file_name).await else {
                return;
            };
            let write_result = run_in_background(move || {
                write_owner_only_file(&destination_path, &decrypted_attachment.file_bytes).map_err(UiError::file_write)
            })
            .await;
            match write_result {
                Ok(()) => controller.context.toasts.success(controller.context.text("attachments.saved")),
                Err(ui_error) => report_error(&controller.context, &ui_error),
            }
        });
    }

    pub(super) fn delete_attachment(&self, attachment_position: i32) {
        let Some((item_id, attachment_id, _)) = self.attachment_at(attachment_position) else {
            return;
        };
        self.run_item_action(move |context| Ok(context.shared_vault.lock_service()?.delete_attachment(item_id, attachment_id)?), None, None);
    }
}
