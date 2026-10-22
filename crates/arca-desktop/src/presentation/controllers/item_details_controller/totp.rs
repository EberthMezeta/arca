//! Código TOTP del elemento abierto, refrescado cada segundo mientras se muestra.

use std::rc::Rc;
use std::time::Duration;

use slint::{ComponentHandle, TimerMode};

use super::ItemDetailsController;
use super::detail_rows::RowValueSource;
use crate::ItemDetails;

const TOTP_REFRESH_INTERVAL: Duration = Duration::from_secs(1);

impl ItemDetailsController {
    pub(super) fn restart_totp_refresh(self: &Rc<Self>) {
        let has_totp = self.details_state.borrow().main_rows.iter().any(|row_spec| matches!(row_spec.value_source, RowValueSource::Totp));
        if !has_totp {
            self.totp_timer.stop();
            return;
        }
        self.refresh_totp_code();
        let controller = Rc::downgrade(self);
        self.totp_timer.start(TimerMode::Repeated, TOTP_REFRESH_INTERVAL, move || {
            if let Some(controller) = controller.upgrade() {
                controller.refresh_totp_code();
            }
        });
    }

    /// Si Argon2id tiene la bóveda ocupada se salta un segundo en vez de congelar la ventana.
    fn refresh_totp_code(&self) {
        let Some(item_id) = self.displayed_item_id() else {
            return;
        };
        let Some(totp_code) = self.context.shared_vault.try_lock_service().and_then(|vault_service| vault_service.current_totp_code(item_id).ok())
        else {
            return;
        };
        self.context.with_window(|app_window| {
            let item_details = app_window.global::<ItemDetails>();
            item_details.set_totp_code(group_totp_digits(&totp_code.current_code).into());
            item_details.set_totp_seconds_left(i32::try_from(totp_code.seconds_until_refresh).unwrap_or(0));
            item_details.set_totp_period_seconds(i32::try_from(totp_code.period_seconds).unwrap_or(30));
        });
    }
}

/// "123456" → "123 456": más fácil de leer y teclear.
fn group_totp_digits(totp_code: &str) -> String {
    let split_position = totp_code.len() / 2;
    if totp_code.len() < 6 || !totp_code.is_char_boundary(split_position) {
        return totp_code.to_string();
    }
    format!("{} {}", &totp_code[..split_position], &totp_code[split_position..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn totp_codes_are_split_in_two_groups() {
        assert_eq!(group_totp_digits("123456"), "123 456");
        assert_eq!(group_totp_digits("12345678"), "1234 5678");
        assert_eq!(group_totp_digits("1234"), "1234");
    }
}
