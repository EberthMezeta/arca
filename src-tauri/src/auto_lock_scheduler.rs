use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::app_state::AppState;
use crate::vault_events::{VaultLockReason, emit_vault_locked};

const IDLE_CHECK_INTERVAL: Duration = Duration::from_secs(5);

/// Hilo en segundo plano que bloquea la bóveda tras el tiempo de inactividad configurado.
pub fn start(app_handle: AppHandle) {
    thread::spawn(move || {
        loop {
            thread::sleep(IDLE_CHECK_INTERVAL);
            let app_state = app_handle.state::<AppState>();
            if app_state.lock_if_idle() {
                emit_vault_locked(&app_handle, VaultLockReason::IdleTimeout);
            }
        }
    });
}
