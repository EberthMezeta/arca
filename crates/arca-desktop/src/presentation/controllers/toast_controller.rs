//! Avisos efímeros (toasts) en la esquina inferior derecha, con acción opcional ("Deshacer").

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use slint::{ComponentHandle, ModelRc, Timer, VecModel};

use crate::{AppWindow, ToastData, ToastTone, Toasts};

const DEFAULT_TOAST_DURATION: Duration = Duration::from_secs(4);
const MAXIMUM_VISIBLE_TOASTS: usize = 3;

pub struct ToastRequest {
    pub message: String,
    pub tone: ToastTone,
    pub duration: Duration,
    pub action: Option<ToastAction>,
    /// Si es mayor que cero, se muestra una barra que se vacía (p. ej. borrado del portapapeles).
    pub countdown_seconds: u64,
}

pub struct ToastAction {
    pub action_label: String,
    pub run_action: Box<dyn FnOnce()>,
}

impl ToastRequest {
    pub fn new(message: impl Into<String>, tone: ToastTone) -> Self {
        Self { message: message.into(), tone, duration: DEFAULT_TOAST_DURATION, action: None, countdown_seconds: 0 }
    }

    pub fn lasting(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    pub fn with_action(mut self, action_label: String, run_action: impl FnOnce() + 'static) -> Self {
        self.action = Some(ToastAction { action_label, run_action: Box::new(run_action) });
        self
    }

    pub fn with_countdown(mut self, countdown_seconds: u64) -> Self {
        self.countdown_seconds = countdown_seconds;
        self
    }
}

struct VisibleToast {
    toast_data: ToastData,
    run_action: Option<Box<dyn FnOnce()>>,
}

#[derive(Default)]
struct ToastState {
    visible_toasts: RefCell<Vec<VisibleToast>>,
    next_toast_id: Cell<i32>,
}

#[derive(Clone)]
pub struct ToastCenter {
    window: slint::Weak<AppWindow>,
    toast_state: Rc<ToastState>,
}

impl ToastCenter {
    pub fn new(window: slint::Weak<AppWindow>) -> Self {
        Self { window, toast_state: Rc::new(ToastState::default()) }
    }

    pub fn install(&self, app_window: &AppWindow) {
        let toasts = app_window.global::<Toasts>();
        let dismissing_center = self.clone();
        toasts.on_dismiss(move |toast_id| dismissing_center.dismiss(toast_id));
        let acting_center = self.clone();
        toasts.on_run_action(move |toast_id| {
            let run_action = acting_center.take_action(toast_id);
            acting_center.dismiss(toast_id);
            if let Some(run_action) = run_action {
                run_action();
            }
        });
    }

    pub fn show(&self, toast_request: ToastRequest) {
        let toast_id = self.toast_state.next_toast_id.get() + 1;
        self.toast_state.next_toast_id.set(toast_id);
        let (action_label, run_action) = toast_request
            .action
            .map_or((String::new(), None), |toast_action| (toast_action.action_label, Some(toast_action.run_action)));
        let toast_data = ToastData {
            toast_id,
            message: toast_request.message.into(),
            tone: toast_request.tone,
            action_label: action_label.into(),
            countdown_seconds: i32::try_from(toast_request.countdown_seconds).unwrap_or(i32::MAX),
        };
        {
            let mut visible_toasts = self.toast_state.visible_toasts.borrow_mut();
            visible_toasts.push(VisibleToast { toast_data, run_action });
            let overflow_count = visible_toasts.len().saturating_sub(MAXIMUM_VISIBLE_TOASTS);
            visible_toasts.drain(..overflow_count);
        }
        self.publish_to_window();
        let expiring_center = self.clone();
        Timer::single_shot(toast_request.duration, move || expiring_center.dismiss(toast_id));
    }

    pub fn success(&self, message: impl Into<String>) {
        self.show(ToastRequest::new(message, ToastTone::Success));
    }

    pub fn error(&self, message: impl Into<String>) {
        self.show(ToastRequest::new(message, ToastTone::Error));
    }

    fn dismiss(&self, toast_id: i32) {
        self.toast_state
            .visible_toasts
            .borrow_mut()
            .retain(|visible_toast| visible_toast.toast_data.toast_id != toast_id);
        self.publish_to_window();
    }

    fn take_action(&self, toast_id: i32) -> Option<Box<dyn FnOnce()>> {
        self.toast_state
            .visible_toasts
            .borrow_mut()
            .iter_mut()
            .find(|visible_toast| visible_toast.toast_data.toast_id == toast_id)
            .and_then(|visible_toast| visible_toast.run_action.take())
    }

    fn publish_to_window(&self) {
        let Some(app_window) = self.window.upgrade() else {
            return;
        };
        let toast_rows: Vec<ToastData> = self
            .toast_state
            .visible_toasts
            .borrow()
            .iter()
            .map(|visible_toast| visible_toast.toast_data.clone())
            .collect();
        app_window.global::<Toasts>().set_visible_toasts(ModelRc::new(VecModel::from(toast_rows)));
    }
}
