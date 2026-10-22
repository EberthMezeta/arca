//! Diálogos modales reutilizables: pedir un texto (o un secreto) y confirmar una acción.
//! Quien abre el diálogo decide qué hacer con la respuesta; este servicio solo gestiona su ciclo de vida.

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, SharedString};
use vault_core::domain::secret_text::SecretText;

use crate::{AppWindow, Dialogs};

/// Diálogo que pide un texto. Se construye con lo obligatorio y se ajusta con métodos (Builder).
pub struct PromptRequest {
    title: String,
    label: String,
    hint: String,
    confirm_label: String,
    initial_value: String,
    is_secret: bool,
    /// Recibe el valor como `SecretText` (puede ser una contraseña). Debe cerrar el diálogo o mostrar un error.
    on_submit: Rc<dyn Fn(SecretText)>,
}

impl PromptRequest {
    pub fn new(title: String, label: String, confirm_label: String, on_submit: impl Fn(SecretText) + 'static) -> Self {
        Self {
            title,
            label,
            hint: String::new(),
            confirm_label,
            initial_value: String::new(),
            is_secret: false,
            on_submit: Rc::new(on_submit),
        }
    }

    #[must_use]
    pub fn hint(mut self, hint: String) -> Self {
        self.hint = hint;
        self
    }

    #[must_use]
    pub fn initial_value(mut self, initial_value: String) -> Self {
        self.initial_value = initial_value;
        self
    }

    /// El campo se muestra enmascarado (contraseñas).
    #[must_use]
    pub fn secret(mut self) -> Self {
        self.is_secret = true;
        self
    }
}

/// Diálogo de confirmación. Sin `on_confirm`, solo informa (el botón cierra).
pub struct ConfirmRequest {
    title: String,
    text: String,
    confirm_label: String,
    is_destructive: bool,
    on_confirm: Box<dyn FnOnce()>,
}

impl ConfirmRequest {
    pub fn new(title: String, text: String, confirm_label: String) -> Self {
        Self { title, text, confirm_label, is_destructive: false, on_confirm: Box::new(|| {}) }
    }

    /// El botón de confirmar se pinta como peligroso (borrar).
    #[must_use]
    pub fn destructive(mut self) -> Self {
        self.is_destructive = true;
        self
    }

    #[must_use]
    pub fn on_confirm(mut self, on_confirm: impl FnOnce() + 'static) -> Self {
        self.on_confirm = Box::new(on_confirm);
        self
    }
}

#[derive(Default)]
struct PendingDialogs {
    prompt_submit_handler: Option<Rc<dyn Fn(SecretText)>>,
    confirm_handler: Option<Box<dyn FnOnce()>>,
}

#[derive(Clone)]
pub struct DialogService {
    window: slint::Weak<AppWindow>,
    pending_dialogs: Rc<RefCell<PendingDialogs>>,
}

impl DialogService {
    pub fn new(window: slint::Weak<AppWindow>) -> Self {
        Self { window, pending_dialogs: Rc::default() }
    }

    pub fn install(&self, app_window: &AppWindow) {
        let dialogs = app_window.global::<Dialogs>();

        let submitting_service = self.clone();
        dialogs.on_prompt_submitted(move |submitted_value| {
            let submit_handler = submitting_service.pending_dialogs.borrow().prompt_submit_handler.clone();
            submitting_service.with_dialogs(|dialogs| {
                dialogs.set_prompt_value(SharedString::new());
                dialogs.set_prompt_error(SharedString::new());
            });
            if let Some(submit_handler) = submit_handler {
                submit_handler(SecretText::new(submitted_value.as_str()));
            }
        });
        let cancelling_service = self.clone();
        dialogs.on_prompt_cancelled(move || cancelling_service.close_prompt());

        let confirming_service = self.clone();
        dialogs.on_confirmed(move || {
            let confirm_handler = confirming_service.pending_dialogs.borrow_mut().confirm_handler.take();
            confirming_service.close_confirmation();
            if let Some(confirm_handler) = confirm_handler {
                confirm_handler();
            }
        });
        let dismissing_service = self.clone();
        dialogs.on_confirm_cancelled(move || dismissing_service.close_confirmation());
    }

    fn with_dialogs(&self, apply: impl FnOnce(&Dialogs<'_>)) {
        if let Some(app_window) = self.window.upgrade() {
            apply(&app_window.global::<Dialogs>());
        }
    }

    pub fn ask_for_text(&self, prompt_request: PromptRequest) {
        self.pending_dialogs.borrow_mut().prompt_submit_handler = Some(prompt_request.on_submit);
        self.with_dialogs(|dialogs| {
            dialogs.set_prompt_title(prompt_request.title.into());
            dialogs.set_prompt_label(prompt_request.label.into());
            dialogs.set_prompt_hint(prompt_request.hint.into());
            dialogs.set_prompt_confirm_label(prompt_request.confirm_label.into());
            dialogs.set_prompt_value(prompt_request.initial_value.into());
            dialogs.set_prompt_is_secret(prompt_request.is_secret);
            dialogs.set_prompt_error(SharedString::new());
            dialogs.set_prompt_busy(false);
            dialogs.set_prompt_open(true);
        });
    }

    pub fn set_prompt_busy(&self, is_busy: bool) {
        self.with_dialogs(|dialogs| dialogs.set_prompt_busy(is_busy));
    }

    pub fn show_prompt_error(&self, error_message: String) {
        self.with_dialogs(|dialogs| {
            dialogs.set_prompt_busy(false);
            dialogs.set_prompt_error(error_message.into());
        });
    }

    pub fn close_prompt(&self) {
        self.pending_dialogs.borrow_mut().prompt_submit_handler = None;
        self.with_dialogs(|dialogs| {
            dialogs.set_prompt_open(false);
            dialogs.set_prompt_busy(false);
            dialogs.set_prompt_value(SharedString::new());
            dialogs.set_prompt_error(SharedString::new());
        });
    }

    pub fn confirm(&self, confirm_request: ConfirmRequest) {
        self.pending_dialogs.borrow_mut().confirm_handler = Some(confirm_request.on_confirm);
        self.with_dialogs(|dialogs| {
            dialogs.set_confirm_title(confirm_request.title.into());
            dialogs.set_confirm_text(confirm_request.text.into());
            dialogs.set_confirm_label(confirm_request.confirm_label.into());
            dialogs.set_confirm_destructive(confirm_request.is_destructive);
            dialogs.set_confirm_open(true);
        });
    }

    fn close_confirmation(&self) {
        self.pending_dialogs.borrow_mut().confirm_handler = None;
        self.with_dialogs(|dialogs| dialogs.set_confirm_open(false));
    }

    /// Al bloquear la bóveda no puede quedar ningún diálogo abierto sobre la pantalla de desbloqueo.
    pub fn close_all(&self) {
        self.close_prompt();
        self.close_confirmation();
    }
}
