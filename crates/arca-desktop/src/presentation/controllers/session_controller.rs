//! Ciclo de vida de la sesión: crear la bóveda, desbloquear, bloquear (a mano o por eventos del sistema).

use std::rc::Rc;
use std::time::Duration;

use slint::{ComponentHandle, SharedString, Timer, TimerMode};
use vault_core::domain::secret_text::SecretText;

use crate::presentation::app_context::{AppContext, AppEvent};
use crate::presentation::controllers::toast_controller::ToastRequest;
use crate::presentation::background_task::{run_with_vault_in_background, spawn_ui_task};
use crate::presentation::error_messages::describe_error;
use crate::presentation::vault_event_bridge;
use crate::vault_events::{VaultEvent, VaultLockReason};
use crate::{AppScreen, AppWindow, Session, ToastTone};

const MINIMIZE_CHECK_INTERVAL: Duration = Duration::from_millis(500);
const CLIPBOARD_CLEARED_TOAST_DURATION: Duration = Duration::from_secs(2);
const DEFAULT_UNLOCK_SUBTITLE_KEY: &str = "unlock.subtitle";

fn unlock_subtitle_key(lock_reason: VaultLockReason) -> &'static str {
    match lock_reason {
        VaultLockReason::IdleTimeout => "unlock.lockedByInactivity",
        VaultLockReason::WindowMinimized => "unlock.lockedByMinimize",
        VaultLockReason::ScreenLocked => "unlock.lockedByScreenLock",
        VaultLockReason::SystemSuspended => "unlock.lockedBySuspend",
    }
}

pub struct SessionController {
    context: Rc<AppContext>,
    retry_countdown_timer: Timer,
    minimize_watch_timer: Timer,
}

impl SessionController {
    pub fn install(context: &Rc<AppContext>, app_window: &AppWindow) -> Rc<Self> {
        let controller = Rc::new(Self {
            context: Rc::clone(context),
            retry_countdown_timer: Timer::default(),
            minimize_watch_timer: Timer::default(),
        });

        let session = app_window.global::<Session>();
        let unlocking_controller = Rc::clone(&controller);
        session.on_unlock(move || unlocking_controller.unlock());
        let creating_controller = Rc::clone(&controller);
        session.on_create_vault(move || creating_controller.create_vault());
        let locking_controller = Rc::clone(&controller);
        session.on_lock_now(move || locking_controller.lock_now());

        let event_controller = Rc::clone(&controller);
        context.events.subscribe(move |app_event| {
            if app_event == AppEvent::LockRequested {
                event_controller.lock_now();
            }
        });
        let system_event_controller = Rc::clone(&controller);
        vault_event_bridge::install_ui_thread_handler(move |vault_event| system_event_controller.handle_vault_event(vault_event));

        controller.watch_window_minimize();
        controller.show_initial_screen();
        controller
    }

    fn with_session(&self, apply: impl FnOnce(&Session<'_>)) {
        if let Some(app_window) = self.context.window() {
            apply(&app_window.global::<Session>());
        }
    }

    fn show_initial_screen(self: &Rc<Self>) {
        let vault_status = match self.context.shared_vault.lock_service() {
            Ok(vault_service) => vault_service.vault_status(),
            Err(_) => return,
        };
        let initial_screen = match (vault_status.vault_exists, vault_status.is_unlocked) {
            (false, _) => AppScreen::Setup,
            (true, false) => AppScreen::Unlock,
            (true, true) => AppScreen::Vault,
        };
        self.with_session(|session| session.set_screen(initial_screen));
        if let Some(retry_after_seconds) = vault_status.unlock_retry_after_seconds {
            // Un error no vacío hace que la pantalla muestre la cuenta atrás.
            self.with_session(|session| session.set_unlock_error(" ".into()));
            self.start_retry_countdown(retry_after_seconds);
        }
    }

    fn unlock(self: &Rc<Self>) {
        let Some(app_window) = self.context.window() else {
            return;
        };
        let session = app_window.global::<Session>();
        let master_password = SecretText::new(session.get_unlock_password().as_str());
        session.set_unlock_password(SharedString::new());
        if master_password.is_empty() || session.get_is_unlocking() {
            return;
        }
        session.set_is_unlocking(true);
        session.set_unlock_error(SharedString::new());

        let controller = Rc::clone(self);
        spawn_ui_task(async move {
            let unlock_result = run_with_vault_in_background(&controller.context.shared_vault, move |vault_service| {
                Ok(vault_service.unlock_vault(&master_password)?)
            })
            .await;
            controller.with_session(|session| session.set_is_unlocking(false));
            match unlock_result {
                Ok(()) => controller.enter_vault(),
                Err(ui_error) => {
                    let error_message = describe_error(&controller.context.translator.borrow(), &ui_error);
                    controller.with_session(|session| {
                        session.set_unlock_error(error_message.into());
                        session.set_unlock_attempts(session.get_unlock_attempts() + 1);
                    });
                    controller.start_retry_countdown(ui_error.retry_after_seconds.unwrap_or(0));
                }
            }
        });
    }

    fn create_vault(self: &Rc<Self>) {
        let Some(app_window) = self.context.window() else {
            return;
        };
        let session = app_window.global::<Session>();
        if session.get_is_creating_vault() {
            return;
        }
        let master_password = SecretText::new(session.get_setup_password().as_str());
        let is_confirmed = session.get_setup_password() == session.get_setup_confirmation();
        if !is_confirmed || master_password.is_empty() {
            return;
        }
        session.set_is_creating_vault(true);
        session.set_setup_error(SharedString::new());

        let controller = Rc::clone(self);
        spawn_ui_task(async move {
            let creation_result = run_with_vault_in_background(&controller.context.shared_vault, move |vault_service| {
                Ok(vault_service.create_vault(&master_password)?)
            })
            .await;
            controller.with_session(|session| session.set_is_creating_vault(false));
            match creation_result {
                Ok(()) => {
                    controller.with_session(|session| {
                        session.set_setup_password(SharedString::new());
                        session.set_setup_confirmation(SharedString::new());
                        session.set_setup_step(0);
                    });
                    controller.enter_vault();
                }
                Err(ui_error) => {
                    let error_message = describe_error(&controller.context.translator.borrow(), &ui_error);
                    controller.with_session(|session| session.set_setup_error(error_message.into()));
                }
            }
        });
    }

    fn enter_vault(&self) {
        self.retry_countdown_timer.stop();
        self.with_session(|session| {
            session.set_unlock_error(SharedString::new());
            session.set_retry_countdown_seconds(0);
            session.set_unlock_subtitle_key(DEFAULT_UNLOCK_SUBTITLE_KEY.into());
            session.set_screen(AppScreen::Vault);
        });
        self.context.navigator.reset();
        self.context.events.publish(AppEvent::SessionUnlocked);
    }

    fn lock_now(&self) {
        self.context.shared_vault.lock_if_unlocked();
        self.show_locked_screen(DEFAULT_UNLOCK_SUBTITLE_KEY);
    }

    fn show_locked_screen(&self, subtitle_key: &str) {
        self.context.dialogs.close_all();
        self.context.navigator.reset();
        self.with_session(|session| {
            session.set_unlock_subtitle_key(subtitle_key.into());
            session.set_unlock_error(SharedString::new());
            session.set_screen(AppScreen::Unlock);
        });
        self.context.events.publish(AppEvent::SessionLocked);
    }

    fn handle_vault_event(&self, vault_event: VaultEvent) {
        match vault_event {
            VaultEvent::VaultLocked(lock_reason) => self.show_locked_screen(unlock_subtitle_key(lock_reason)),
            VaultEvent::ClipboardCleared => {
                let message = self.context.text("clipboard.cleared");
                self.context.toasts.show(
                    ToastRequest::new(message, ToastTone::Info).lasting(CLIPBOARD_CLEARED_TOAST_DURATION),
                );
            }
        }
    }

    /// Cuenta atrás visible tras demasiados intentos fallidos.
    fn start_retry_countdown(self: &Rc<Self>, retry_after_seconds: u64) {
        self.with_session(|session| session.set_retry_countdown_seconds(i32::try_from(retry_after_seconds).unwrap_or(i32::MAX)));
        if retry_after_seconds == 0 {
            self.retry_countdown_timer.stop();
            return;
        }
        let controller = Rc::downgrade(self);
        self.retry_countdown_timer.start(TimerMode::Repeated, Duration::from_secs(1), move || {
            let Some(controller) = controller.upgrade() else {
                return;
            };
            let mut seconds_left = 0;
            controller.with_session(|session| {
                seconds_left = (session.get_retry_countdown_seconds() - 1).max(0);
                session.set_retry_countdown_seconds(seconds_left);
            });
            if seconds_left == 0 {
                controller.retry_countdown_timer.stop();
            }
        });
    }

    /// Slint no avisa al minimizar: se comprueba periódicamente (barato y sin bloquear la ventana).
    fn watch_window_minimize(self: &Rc<Self>) {
        let controller = Rc::downgrade(self);
        self.minimize_watch_timer.start(TimerMode::Repeated, MINIMIZE_CHECK_INTERVAL, move || {
            let Some(controller) = controller.upgrade() else {
                return;
            };
            let Some(app_window) = controller.context.window() else {
                return;
            };
            let is_vault_open = app_window.global::<Session>().get_screen() == AppScreen::Vault;
            if !is_vault_open || !app_window.window().is_minimized() {
                return;
            }
            if controller.context.shared_vault.lock_if_minimize_policy_applies() {
                controller.show_locked_screen(unlock_subtitle_key(VaultLockReason::WindowMinimized));
            }
        });
    }
}
