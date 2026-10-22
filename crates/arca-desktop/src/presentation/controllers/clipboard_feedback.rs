//! Copia secretos desde Rust y muestra "Copiado · se borrará en N s".

use std::time::Duration;

use vault_core::application::vault_service::VaultService;
use vault_core::domain::vault_error::VaultResult;

use crate::ToastTone;
use crate::presentation::app_context::AppContext;
use crate::presentation::controllers::toast_controller::ToastRequest;
use crate::presentation::error_messages::describe_error;
use crate::ui_error::{UiError, UiResult};
use vault_core::application::vault_service::VaultSession;

const COPIED_TOAST_DURATION: Duration = Duration::from_millis(2500);

pub fn copy_with_feedback(
    context: &AppContext,
    copied_label: &str,
    copy_operation: impl FnOnce(&mut VaultService) -> VaultResult<()>,
) {
    let copy_result: UiResult<u64> = context.shared_vault.lock_service().and_then(|mut vault_service| {
        copy_operation(&mut vault_service)?;
        Ok(vault_service.preferences()?.clipboard_clear_after_seconds)
    });
    match copy_result {
        Ok(clear_after_seconds) => {
            let message = context.text_with(
                "clipboard.copied",
                &[("copiedLabel", copied_label), ("clearAfterSeconds", &clear_after_seconds.to_string())],
            );
            context.toasts.show(
                ToastRequest::new(message, ToastTone::Success)
                    .lasting(COPIED_TOAST_DURATION)
                    .with_countdown(clear_after_seconds),
            );
        }
        Err(ui_error) => report_error(context, &ui_error),
    }
}

pub fn report_error(context: &AppContext, ui_error: &UiError) {
    context.toasts.error(describe_error(&context.translator.borrow(), ui_error));
}
