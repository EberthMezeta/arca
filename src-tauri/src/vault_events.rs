use serde::Serialize;
use tauri::{AppHandle, Emitter};

pub const VAULT_LOCKED_EVENT: &str = "vault-locked";
pub const CLIPBOARD_CLEARED_EVENT: &str = "clipboard-cleared";

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VaultLockReason {
    IdleTimeout,
    WindowMinimized,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct VaultLockedPayload {
    lock_reason: VaultLockReason,
}

/// Patrón Observer: el backend avisa y la UI reacciona (vuelve a la pantalla de desbloqueo).
pub fn emit_vault_locked(app_handle: &AppHandle, lock_reason: VaultLockReason) {
    let _ = app_handle.emit(VAULT_LOCKED_EVENT, VaultLockedPayload { lock_reason });
}

pub fn emit_clipboard_cleared(app_handle: &AppHandle) {
    let _ = app_handle.emit(CLIPBOARD_CLEARED_EVENT, ());
}
