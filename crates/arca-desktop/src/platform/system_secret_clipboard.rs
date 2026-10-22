use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use arboard::Clipboard;
#[cfg(target_os = "macos")]
use arboard::SetExtApple;
#[cfg(all(unix, not(any(target_os = "macos", target_os = "android", target_os = "emscripten"))))]
use arboard::SetExtLinux;
#[cfg(windows)]
use arboard::SetExtWindows;
use vault_core::domain::ports::secret_clipboard::SecretClipboard;
use vault_core::domain::secret_text::SecretText;
use vault_core::domain::vault_error::{VaultError, VaultResult};
use zeroize::Zeroizing;

use crate::vault_events::{VaultEvent, VaultEventSink};

/// Último secreto copiado y cuándo hay que vaciarlo. Una copia nueva sustituye el plazo de la
/// anterior, así que nunca se borra antes de tiempo lo que se copió después.
#[derive(Default)]
struct TrackedClipboardSecret {
    copied_secret: Option<SecretText>,
    clear_deadline: Option<Instant>,
}

/// Una única instancia de `arboard` viva durante toda la app: en X11, soltar la última
/// entrega el contenido al gestor del portapapeles (que lo guardaría en su historial).
#[derive(Default)]
struct ClipboardState {
    system_clipboard: Option<Clipboard>,
    tracked_secret: TrackedClipboardSecret,
}

impl ClipboardState {
    fn system_clipboard(&mut self) -> VaultResult<&mut Clipboard> {
        if self.system_clipboard.is_none() {
            self.system_clipboard = Some(Clipboard::new().map_err(VaultError::storage)?);
        }
        self.system_clipboard.as_mut().ok_or_else(|| VaultError::storage("portapapeles no disponible"))
    }

    /// Vacía el portapapeles solo si sigue conteniendo el secreto que copiamos.
    fn clear_tracked_secret(&mut self) -> bool {
        self.tracked_secret.clear_deadline = None;
        let Some(copied_secret) = self.tracked_secret.copied_secret.take() else {
            return false;
        };
        let Ok(system_clipboard) = self.system_clipboard() else {
            return false;
        };
        let current_clipboard_text = Zeroizing::new(system_clipboard.get_text().unwrap_or_default());
        if current_clipboard_text.as_str() != copied_secret.expose_secret() {
            return false;
        }
        system_clipboard.clear().is_ok()
    }
}

/// Copia al portapapeles del sistema marcando el contenido como sensible, para que los
/// gestores de historial del escritorio no lo guarden (`x-kde-passwordManagerHint` en Linux,
/// `ExcludeClipboardContentFromMonitorProcessing` en Windows, `org.nspasteboard.ConcealedType`
/// en macOS). Se vacía pasado `clear_after`, al bloquear la bóveda y al cerrar la app,
/// siempre que siga conteniendo el mismo secreto (no borra lo que el usuario copió después).
pub struct SystemSecretClipboard {
    vault_event_sink: VaultEventSink,
    clipboard_state: Arc<(Mutex<ClipboardState>, Condvar)>,
}

impl SystemSecretClipboard {
    /// Un único hilo vacía el portapapeles cuando vence el plazo (no uno por cada copia).
    pub fn new(vault_event_sink: VaultEventSink) -> Self {
        let clipboard_state = Arc::new((Mutex::new(ClipboardState::default()), Condvar::new()));
        let scheduler_state = Arc::clone(&clipboard_state);
        let scheduler_event_sink = Arc::clone(&vault_event_sink);
        thread::Builder::new()
            .name("clipboard-clear".into())
            .spawn(move || run_clear_scheduler(&scheduler_state, &scheduler_event_sink))
            .ok();
        Self { vault_event_sink, clipboard_state }
    }

    fn lock_state(&self) -> MutexGuard<'_, ClipboardState> {
        self.clipboard_state.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Espera al plazo del secreto copiado (o a que una copia nueva lo cambie) y lo vacía al vencer.
fn run_clear_scheduler(clipboard_state: &(Mutex<ClipboardState>, Condvar), vault_event_sink: &VaultEventSink) {
    let (state_mutex, deadline_changed) = clipboard_state;
    let mut state = state_mutex.lock().unwrap_or_else(PoisonError::into_inner);
    loop {
        let Some(clear_deadline) = state.tracked_secret.clear_deadline else {
            state = deadline_changed.wait(state).unwrap_or_else(PoisonError::into_inner);
            continue;
        };
        let now = Instant::now();
        if now < clear_deadline {
            state =
                deadline_changed.wait_timeout(state, clear_deadline - now).unwrap_or_else(PoisonError::into_inner).0;
            continue;
        }
        if state.clear_tracked_secret() {
            vault_event_sink(VaultEvent::ClipboardCleared);
        }
    }
}

impl SecretClipboard for SystemSecretClipboard {
    fn copy_with_automatic_clear(&self, secret_value: SecretText, clear_after: Duration) -> VaultResult<()> {
        let mut clipboard_state = self.lock_state();
        clipboard_state
            .system_clipboard()?
            .set()
            .exclude_from_history()
            .text(secret_value.expose_secret())
            .map_err(VaultError::storage)?;
        clipboard_state.tracked_secret.copied_secret = Some(secret_value);
        clipboard_state.tracked_secret.clear_deadline = Some(Instant::now() + clear_after);
        self.clipboard_state.1.notify_one();
        Ok(())
    }

    /// Se llama al bloquear: aunque el mutex esté envenenado, el secreto no debe quedarse copiado.
    fn clear_copied_secret(&self) {
        let mut clipboard_state = self.lock_state();
        if clipboard_state.clear_tracked_secret() {
            (self.vault_event_sink)(VaultEvent::ClipboardCleared);
        }
    }
}
