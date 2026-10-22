//! Crear, renombrar y borrar carpetas con los diálogos compartidos.

use std::rc::Rc;

use uuid::Uuid;
use vault_core::application::search_vault_items::VaultItemFilter;
use vault_core::domain::secret_text::SecretText;

use crate::presentation::app_context::{AppContext, AppEvent};
use crate::presentation::controllers::clipboard_feedback::report_error;
use crate::presentation::controllers::dialog_controller::{ConfirmRequest, PromptRequest};
use crate::presentation::error_messages::describe_error;
use crate::ui_error::UiResult;

type FolderOperation = Box<dyn Fn(&AppContext, &str) -> UiResult<()>>;

fn ask_folder_name(context: &Rc<AppContext>, title_key: &str, confirm_key: &str, initial_name: String, folder_operation: FolderOperation) {
    let submitting_context = Rc::clone(context);
    context.dialogs.ask_for_text(PromptRequest {
        title: context.text(title_key),
        label: context.text("folders.name"),
        hint: String::new(),
        confirm_label: context.text(confirm_key),
        initial_value: initial_name,
        is_secret: false,
        on_submit: Rc::new(move |folder_name: SecretText| {
            match folder_operation(&submitting_context, folder_name.expose_secret()) {
                Ok(()) => {
                    submitting_context.dialogs.close_prompt();
                    submitting_context.events.publish(AppEvent::VaultDataChanged);
                }
                Err(ui_error) => {
                    let error_message = describe_error(&submitting_context.translator.borrow(), &ui_error);
                    submitting_context.dialogs.show_prompt_error(error_message);
                }
            }
        }),
    });
}

pub fn ask_new_folder(context: &Rc<AppContext>) {
    ask_folder_name(
        context,
        "folders.create",
        "actions.create",
        String::new(),
        Box::new(|context, folder_name| {
            context.shared_vault.lock_service()?.create_folder(folder_name)?;
            Ok(())
        }),
    );
}

pub fn ask_folder_rename(context: &Rc<AppContext>, folder_id: Uuid, current_name: String) {
    ask_folder_name(
        context,
        "folders.rename",
        "actions.save",
        current_name,
        Box::new(move |context, folder_name| {
            context.shared_vault.lock_service()?.rename_folder(folder_id, folder_name)?;
            Ok(())
        }),
    );
}

pub fn confirm_folder_deletion(context: &Rc<AppContext>, folder_id: Uuid, folder_name: &str) {
    let deleting_context = Rc::clone(context);
    context.dialogs.confirm(ConfirmRequest {
        title: context.text("folders.deleteTitle"),
        text: context.text_with("folders.deleteText", &[("folderName", folder_name)]),
        confirm_label: context.text("folders.delete"),
        is_destructive: true,
        on_confirm: Box::new(move || {
            let deletion_result: UiResult<()> =
                deleting_context.shared_vault.lock_service().and_then(|mut vault_service| Ok(vault_service.delete_folder(folder_id)?));
            match deletion_result {
                Ok(()) => {
                    deleting_context.navigator.apply_navigation_filter(VaultItemFilter::default());
                    deleting_context.events.publish(AppEvent::VaultDataChanged);
                }
                Err(ui_error) => report_error(&deleting_context, &ui_error),
            }
        }),
    });
}
