use std::thread;
use std::time::{Duration, SystemTime};

use crate::platform::system_lock_watcher::SuspensionDetector;
use crate::shared_vault::VaultLocker;
use crate::vault_events::VaultLockReason;

const IDLE_CHECK_INTERVAL: Duration = Duration::from_secs(5);

/// Hilo en segundo plano que bloquea la bóveda tras el tiempo de inactividad configurado
/// o al volver de una suspensión del equipo.
pub fn start(vault_locker: VaultLocker) {
    thread::spawn(move || {
        let mut suspension_detector = SuspensionDetector::new(IDLE_CHECK_INTERVAL, SystemTime::now());
        loop {
            thread::sleep(IDLE_CHECK_INTERVAL);
            if suspension_detector.detect_suspension(SystemTime::now()) {
                vault_locker.lock_and_notify(VaultLockReason::SystemSuspended);
                continue;
            }
            vault_locker.lock_if_idle_and_notify();
        }
    });
}
