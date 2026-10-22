//! Diálogos modales reutilizables: pedir un texto (o un secreto) y confirmar una acción.
//! Quien abre el diálogo decide qué hacer con la respuesta; este servicio solo gestiona su ciclo de vida.

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, SharedString};
use vault_core::domain::secret_text::SecretText;

use crate::{AppWindow, Dialogs};

pub struct PromptRequest {
    pub title: String,
    pub label: String,
    pub hint: String,
    pub confirm_label: String,
    pub initial_value: String,
    pub is_secret: bool,
    /// Recibe el valor como `SecretText` (puede ser una contraseña). Debe cerrar el diálogo o mostrar un error.
    pub on_submit: Rc<dyn Fn(SecretText)>,
}

pub struct ConfirmRequest {
    pub title: String,
    pub text: String,
    pub confirm_label: String,
    pub is_destructive: bool,
    pub on_confirm: Box<dyn FnOnce()>,
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
