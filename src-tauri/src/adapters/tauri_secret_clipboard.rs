use std::thread;
use std::time::Duration;

use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;
use vault_core::domain::ports::secret_clipboard::SecretClipboard;
use vault_core::domain::secret_text::SecretText;
use vault_core::domain::vault_error::{VaultError, VaultResult};
use zeroize::Zeroizing;

use crate::vault_events::emit_clipboard_cleared;

/// Copia al portapapeles del sistema y lo vacía pasado `clear_after`,
/// solo si sigue conteniendo el mismo secreto (no borra lo que el usuario copió después).
pub struct TauriSecretClipboard {
    app_handle: AppHandle,
}

impl TauriSecretClipboard {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

impl SecretClipboard for TauriSecretClipboard {
    fn copy_with_automatic_clear(&self, secret_value: SecretText, clear_after: Duration) -> VaultResult<()> {
        self.app_handle
            .clipboard()
            .write_text(secret_value.expose_secret())
            .map_err(VaultError::storage)?;

        let app_handle = self.app_handle.clone();
        thread::spawn(move || {
            thread::sleep(clear_after);
            let current_clipboard_text = Zeroizing::new(app_handle.clipboard().read_text().unwrap_or_default());
            if current_clipboard_text.as_str() != secret_value.expose_secret() {
                return;
            }
            if app_handle.clipboard().clear().is_ok() {
                emit_clipboard_cleared(&app_handle);
            }
        });
        Ok(())
    }
}
