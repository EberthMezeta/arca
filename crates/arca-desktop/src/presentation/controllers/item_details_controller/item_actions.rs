//! Acciones sobre el elemento abierto: favorito, papelera (con «Deshacer») y borrado definitivo.

use std::rc::Rc;
use std::time::Duration;

use super::ItemDetailsController;
use crate::ToastTone;
use crate::presentation::app_commands::ItemCommand;
use crate::presentation::app_context::AppEvent;
use crate::presentation::controllers::clipboard_feedback::report_error;
use crate::presentation::controllers::dialog_controller::ConfirmRequest;
use crate::presentation::controllers::toast_controller::ToastRequest;

const UNDOABLE_TOAST_DURATION: Duration = Duration::from_secs(6);

impl ItemDetailsController {
    /// Ejecuta el comando, avisa de los cambios y muestra el resultado. Con `offers_undo`, el
    /// aviso lleva «Deshacer», que ejecuta la acción inversa del propio comando.
    pub(super) fn run_item_command(
        &self,
        item_command: ItemCommand,
        success_message: Option<String>,
        offers_undo: bool,
    ) {
        if let Err(ui_error) = item_command.execute(&self.context) {
            return report_error(&self.context, &ui_error);
        }
        self.context.events.publish(AppEvent::VaultDataChanged);
        let Some(success_message) = success_message else {
            return;
        };
        let Some(undo_command) = item_command.inverse().filter(|_| offers_undo) else {
            return self.context.toasts.success(success_message);
        };
        let undoing_context = Rc::clone(&self.context);
        self.context.toasts.show(
            ToastRequest::new(success_message, ToastTone::Success).lasting(UNDOABLE_TOAST_DURATION).with_action(
                self.context.text("actions.undo"),
                move || match undo_command.execute(&undoing_context) {
                    Ok(()) => undoing_context.events.publish(AppEvent::VaultDataChanged),
                    Err(ui_error) => report_error(&undoing_context, &ui_error),
                },
            ),
        );
    }

    pub(super) fn toggle_favorite(&self) {
        if let Some(item_id) = self.displayed_item_id() {
            self.run_item_command(ItemCommand::ToggleFavorite(item_id), None, false);
        }
    }

    pub(super) fn move_to_trash(&self) {
        let Some((item_id, item_title)) = self
            .details_state
            .borrow()
            .displayed_item
            .as_ref()
            .map(|vault_item| (vault_item.item_id, vault_item.title.clone()))
        else {
            return;
        };
        let success_message = self.context.text_with("details.movedToTrash", &[("itemTitle", &item_title)]);
        self.run_item_command(ItemCommand::MoveToTrash(item_id), Some(success_message), true);
    }

    pub(super) fn restore(&self) {
        if let Some(item_id) = self.displayed_item_id() {
            self.run_item_command(
                ItemCommand::RestoreFromTrash(item_id),
                Some(self.context.text("details.restored")),
                false,
            );
        }
    }

    pub(super) fn confirm_permanent_deletion(self: &Rc<Self>) {
        let Some((item_id, item_title)) = self
            .details_state
            .borrow()
            .displayed_item
            .as_ref()
            .map(|vault_item| (vault_item.item_id, vault_item.title.clone()))
        else {
            return;
        };
        let controller = Rc::clone(self);
        self.context.dialogs.confirm(
            ConfirmRequest::new(
                self.context.text("details.deleteForeverTitle"),
                self.context.text_with("details.deleteForeverText", &[("itemTitle", &item_title)]),
                self.context.text("actions.deleteForever"),
            )
            .destructive()
            .on_confirm(move || {
                let success_message = controller.context.text("details.deletedForever");
                controller.run_item_command(ItemCommand::DeletePermanently(item_id), Some(success_message), false);
            }),
        );
    }
}
