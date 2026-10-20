use std::thread;
use std::time::{Duration, SystemTime};

use tauri::{AppHandle, Manager};

use crate::app_state::AppState;
use crate::system_lock_watcher::{SuspensionDetector, lock_and_notify};
use crate::vault_events::{VaultLockReason, emit_vault_locked};

const IDLE_CHECK_INTERVAL: Duration = Duration::from_secs(5);

/// Hilo en segundo plano que bloquea la bóveda tras el tiempo de inactividad configurado
/// o al volver de una suspensión del equipo.
pub fn start(app_handle: AppHandle) {
    thread::spawn(move || {
        let mut suspension_detector = SuspensionDetector::new(IDLE_CHECK_INTERVAL, SystemTime::now());
        loop {
            thread::sleep(IDLE_CHECK_INTERVAL);
            if suspension_detector.detect_suspension(SystemTime::now()) {
                lock_and_notify(&app_handle, VaultLockReason::SystemSuspended);
                continue;
            }
            let app_state = app_handle.state::<AppState>();
            if app_state.lock_if_idle() {
                emit_vault_locked(&app_handle, VaultLockReason::IdleTimeout);
            }
        }
    });
}
