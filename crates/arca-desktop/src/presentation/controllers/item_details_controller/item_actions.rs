//! Acciones sobre el elemento abierto: favorito, papelera (con «Deshacer») y borrado definitivo.

use std::rc::Rc;
use std::time::Duration;

use super::ItemDetailsController;
use crate::ToastTone;
use crate::presentation::app_context::{AppContext, AppEvent};
use crate::presentation::controllers::clipboard_feedback::report_error;
use crate::presentation::controllers::dialog_controller::ConfirmRequest;
use crate::presentation::controllers::toast_controller::ToastRequest;
use crate::presentation::item_drafts::save_request_from_item;
use crate::ui_error::UiResult;

const UNDOABLE_TOAST_DURATION: Duration = Duration::from_secs(6);

type UndoAction = Box<dyn FnOnce(&AppContext) -> UiResult<()>>;

impl ItemDetailsController {
    /// Ejecuta una acción, avisa de los cambios y muestra el resultado (con "Deshacer" opcional).
    pub(super) fn run_item_action(
        &self,
        item_action: impl FnOnce(&AppContext) -> UiResult<()>,
        success_message: Option<String>,
        undo_action: Option<UndoAction>,
    ) {
        if let Err(ui_error) = item_action(&self.context) {
            return report_error(&self.context, &ui_error);
        }
        self.context.events.publish(AppEvent::VaultDataChanged);
        let Some(success_message) = success_message else {
            return;
        };
        let Some(undo_action) = undo_action else {
            return self.context.toasts.success(success_message);
        };
        let undoing_context = Rc::clone(&self.context);
        self.context.toasts.show(
            ToastRequest::new(success_message, ToastTone::Success)
                .lasting(UNDOABLE_TOAST_DURATION)
                .with_action(self.context.text("actions.undo"), move || match undo_action(&undoing_context) {
                    Ok(()) => undoing_context.events.publish(AppEvent::VaultDataChanged),
                    Err(ui_error) => report_error(&undoing_context, &ui_error),
                }),
        );
    }

    pub(super) fn toggle_favorite(&self) {
        let Some(item_id) = self.displayed_item_id() else {
            return;
        };
        self.run_item_action(
            move |context| {
                let mut vault_service = context.shared_vault.lock_service()?;
                let mut save_request = save_request_from_item(&vault_service.get_item_details(item_id)?);
                save_request.is_favorite = !save_request.is_favorite;
                vault_service.save_item(save_request)?;
                Ok(())
            },
            None,
            None,
        );
    }

    pub(super) fn move_to_trash(&self) {
        let Some((item_id, item_title)) =
            self.details_state.borrow().displayed_item.as_ref().map(|vault_item| (vault_item.item_id, vault_item.title.clone()))
        else {
            return;
        };
        self.run_item_action(
            move |context| {
                context.shared_vault.lock_service()?.move_item_to_trash(item_id)?;
                context.navigator.select_item(None);
                Ok(())
            },
            Some(self.context.text_with("details.movedToTrash", &[("itemTitle", &item_title)])),
            Some(Box::new(move |context| Ok(context.shared_vault.lock_service()?.restore_item_from_trash(item_id)?))),
        );
    }

    pub(super) fn restore(&self) {
        let Some(item_id) = self.displayed_item_id() else {
            return;
        };
        self.run_item_action(
            move |context| Ok(context.shared_vault.lock_service()?.restore_item_from_trash(item_id)?),
            Some(self.context.text("details.restored")),
            None,
        );
    }

    pub(super) fn confirm_permanent_deletion(self: &Rc<Self>) {
        let Some((item_id, item_title)) =
            self.details_state.borrow().displayed_item.as_ref().map(|vault_item| (vault_item.item_id, vault_item.title.clone()))
        else {
            return;
        };
        let controller = Rc::clone(self);
        self.context.dialogs.confirm(ConfirmRequest {
            title: self.context.text("details.deleteForeverTitle"),
            text: self.context.text_with("details.deleteForeverText", &[("itemTitle", &item_title)]),
            confirm_label: self.context.text("actions.deleteForever"),
            is_destructive: true,
            on_confirm: Box::new(move || {
                controller.run_item_action(
                    move |context| {
                        context.shared_vault.lock_service()?.delete_item_permanently(item_id)?;
                        context.navigator.select_item(None);
                        Ok(())
                    },
                    Some(controller.context.text("details.deletedForever")),
                    None,
                );
            }),
        });
    }
}
