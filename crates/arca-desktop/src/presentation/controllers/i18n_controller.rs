//! Servicios transversales que la interfaz consulta en sus enlaces: textos traducidos,
//! fortaleza de contraseñas y actividad del usuario (retrasa el auto-bloqueo).

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use slint::{ComponentHandle, SharedString};
use vault_core::domain::master_password_policy::{MINIMUM_MASTER_PASSWORD_CHARACTER_COUNT, MINIMUM_MASTER_PASSWORD_STRENGTH_SCORE};
use vault_core::domain::password_strength::PasswordStrength;
use vault_core::domain::secret_text::SecretText;

use crate::presentation::app_context::{AppContext, AppEvent};
use crate::{AppWindow, I18n, PasswordStrengths, UserActivity};

const MINIMUM_TIME_BETWEEN_ACTIVITY_REPORTS: Duration = Duration::from_secs(15);

pub fn install(context: &Rc<AppContext>, app_window: &AppWindow) {
    install_translations(context, app_window);
    install_password_strengths(app_window);
    install_activity_reporter(context, app_window);
}

fn install_translations(context: &Rc<AppContext>, app_window: &AppWindow) {
    let i18n = app_window.global::<I18n>();
    let lookup_context = Rc::clone(context);
    i18n.on_lookup(move |translation_key| lookup_context.text(&translation_key).into());
    let count_context = Rc::clone(context);
    i18n.on_lookup_count(move |translation_key, count| count_context.count_text(&translation_key, i64::from(count), &[]).into());
    let with_context = Rc::clone(context);
    i18n.on_lookup_with(move |translation_key, placeholder, value| {
        with_context.text_with(&translation_key, &[(placeholder.as_str(), value.as_str())]).into()
    });
    let with_two_context = Rc::clone(context);
    i18n.on_lookup_with_two(move |translation_key, first_placeholder, first_value, second_placeholder, second_value| {
        with_two_context
            .text_with(
                &translation_key,
                &[(first_placeholder.as_str(), first_value.as_str()), (second_placeholder.as_str(), second_value.as_str())],
            )
            .into()
    });

    // Cambiar `revision` obliga a la interfaz a recalcular todas las cadenas.
    let window = app_window.as_weak();
    context.events.subscribe(move |app_event| {
        if app_event != AppEvent::LanguageChanged {
            return;
        }
        if let Some(app_window) = window.upgrade() {
            let i18n = app_window.global::<I18n>();
            i18n.set_revision(i18n.get_revision() + 1);
        }
    });
}

fn install_password_strengths(app_window: &AppWindow) {
    let password_strengths = app_window.global::<PasswordStrengths>();
    password_strengths.set_master_minimum_length(i32::try_from(MINIMUM_MASTER_PASSWORD_CHARACTER_COUNT).unwrap_or(i32::MAX));
    password_strengths.set_master_minimum_score(i32::from(MINIMUM_MASTER_PASSWORD_STRENGTH_SCORE));
    password_strengths.on_score_of(|candidate_password: SharedString| {
        if candidate_password.is_empty() {
            return -1;
        }
        i32::from(PasswordStrength::estimate(&SecretText::new(candidate_password.as_str())).score)
    });
}

/// Se limita la frecuencia y se omite si Argon2id tiene la bóveda ocupada: nunca congela la ventana.
fn install_activity_reporter(context: &Rc<AppContext>, app_window: &AppWindow) {
    let reporting_context = Rc::clone(context);
    let last_report_time: Cell<Option<Instant>> = Cell::new(None);
    app_window.global::<UserActivity>().on_reported(move || {
        let is_too_soon = last_report_time.get().is_some_and(|last_report| last_report.elapsed() < MINIMUM_TIME_BETWEEN_ACTIVITY_REPORTS);
        if is_too_soon {
            return;
        }
        let Some(mut vault_service) = reporting_context.shared_vault.try_lock_service() else {
            return;
        };
        let _ = vault_service.register_user_activity();
        last_report_time.set(Some(Instant::now()));
    });
}
