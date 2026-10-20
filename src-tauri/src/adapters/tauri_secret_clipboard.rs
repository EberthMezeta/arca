use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use arboard::Clipboard;
#[cfg(target_os = "macos")]
use arboard::SetExtApple;
#[cfg(all(unix, not(any(target_os = "macos", target_os = "android", target_os = "emscripten"))))]
use arboard::SetExtLinux;
#[cfg(windows)]
use arboard::SetExtWindows;
use tauri::AppHandle;
use vault_core::domain::ports::secret_clipboard::SecretClipboard;
use vault_core::domain::secret_text::SecretText;
use vault_core::domain::vault_error::{VaultError, VaultResult};
use zeroize::Zeroizing;

use crate::vault_events::emit_clipboard_cleared;

/// Último secreto copiado. `copy_generation` distingue copias sucesivas sin comparar secretos:
/// el temporizador de una copia anterior no debe borrar una más reciente.
#[derive(Default)]
struct TrackedClipboardSecret {
    copied_secret: Option<SecretText>,
    copy_generation: u64,
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
pub struct TauriSecretClipboard {
    app_handle: AppHandle,
    clipboard_state: Arc<Mutex<ClipboardState>>,
}

impl TauriSecretClipboard {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle, clipboard_state: Arc::new(Mutex::new(ClipboardState::default())) }
    }
}

impl SecretClipboard for TauriSecretClipboard {
    fn copy_with_automatic_clear(&self, secret_value: SecretText, clear_after: Duration) -> VaultResult<()> {
        let mut clipboard_state = self.clipboard_state.lock().map_err(|_| VaultError::storage("portapapeles bloqueado"))?;
        clipboard_state
            .system_clipboard()?
            .set()
            .exclude_from_history()
            .text(secret_value.expose_secret())
            .map_err(VaultError::storage)?;
        clipboard_state.tracked_secret.copy_generation += 1;
        clipboard_state.tracked_secret.copied_secret = Some(secret_value);
        let copy_generation = clipboard_state.tracked_secret.copy_generation;
        drop(clipboard_state);

        let clipboard_state = Arc::clone(&self.clipboard_state);
        let app_handle = self.app_handle.clone();
        thread::spawn(move || {
            thread::sleep(clear_after);
            let Ok(mut clipboard_state) = clipboard_state.lock() else {
                return;
            };
            if clipboard_state.tracked_secret.copy_generation != copy_generation {
                return;
            }
            if clipboard_state.clear_tracked_secret() {
                emit_clipboard_cleared(&app_handle);
            }
        });
        Ok(())
    }

    fn clear_copied_secret(&self) {
        let Ok(mut clipboard_state) = self.clipboard_state.lock() else {
            return;
        };
        if clipboard_state.clear_tracked_secret() {
            emit_clipboard_cleared(&self.app_handle);
        }
    }
}
