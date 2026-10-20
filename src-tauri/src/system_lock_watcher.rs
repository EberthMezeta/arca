//! Bloquea la bóveda cuando el equipo se suspende o se bloquea la pantalla.
//!
//! - En todas las plataformas: `SuspensionDetector` nota la suspensión por el salto del reloj
//!   de pared entre dos comprobaciones del planificador de auto-bloqueo.
//! - En Linux, además, se escuchan las señales de D-Bus del bloqueo de pantalla y de logind,
//!   que avisan al momento (antes de dormir y aunque el bloqueo no vaya seguido de suspensión).

use std::time::{Duration, SystemTime};

use tauri::{AppHandle, Manager};

use crate::app_state::AppState;
use crate::vault_events::{VaultLockReason, emit_vault_locked};

/// Margen sobre el intervalo esperado antes de considerar que hubo una suspensión.
const SUSPENSION_GAP_TOLERANCE: Duration = Duration::from_secs(30);

pub fn lock_and_notify(app_handle: &AppHandle, lock_reason: VaultLockReason) {
    let app_state = app_handle.state::<AppState>();
    if app_state.lock_if_unlocked() {
        emit_vault_locked(app_handle, lock_reason);
    }
}

/// Mientras el equipo duerme, el reloj monotónico (el de `thread::sleep`) se detiene pero
/// el de pared sigue avanzando: un hueco mucho mayor que el intervalo esperado delata la suspensión.
/// Un ajuste manual del reloj hacia delante también bloquea, lo cual es inofensivo.
pub struct SuspensionDetector {
    expected_check_interval: Duration,
    last_check_at: SystemTime,
}

impl SuspensionDetector {
    pub fn new(expected_check_interval: Duration, now: SystemTime) -> Self {
        Self { expected_check_interval, last_check_at: now }
    }

    pub fn detect_suspension(&mut self, now: SystemTime) -> bool {
        let elapsed_wall_clock_time = now.duration_since(self.last_check_at).unwrap_or_default();
        self.last_check_at = now;
        elapsed_wall_clock_time > self.expected_check_interval + SUSPENSION_GAP_TOLERANCE
    }
}

pub fn start(app_handle: &AppHandle) {
    #[cfg(target_os = "linux")]
    linux_session_signals::start(app_handle);
    #[cfg(not(target_os = "linux"))]
    let _ = app_handle;
}

#[cfg(target_os = "linux")]
mod linux_session_signals {
    use std::thread;
    use std::time::Duration;

    use dbus::blocking::Connection;
    use dbus::message::MatchRule;
    use tauri::AppHandle;

    use super::lock_and_notify;
    use crate::vault_events::VaultLockReason;

    const SIGNAL_POLL_INTERVAL: Duration = Duration::from_secs(60);

    /// Bloqueo de pantalla del escritorio (estándar freedesktop y GNOME): `ActiveChanged(bool)`.
    const SCREENSAVER_INTERFACES: [&str; 2] = ["org.freedesktop.ScreenSaver", "org.gnome.ScreenSaver"];

    pub fn start(app_handle: &AppHandle) {
        let session_bus_app_handle = app_handle.clone();
        thread::spawn(move || listen_to_session_bus(&session_bus_app_handle));
        let system_bus_app_handle = app_handle.clone();
        thread::spawn(move || listen_to_system_bus(&system_bus_app_handle));
    }

    /// Sin bus de sesión (p. ej. sesión remota mínima) no hay nada que escuchar: se ignora.
    fn listen_to_session_bus(app_handle: &AppHandle) {
        let Ok(session_bus) = Connection::new_session() else {
            return;
        };
        for screensaver_interface in SCREENSAVER_INTERFACES {
            let signal_app_handle = app_handle.clone();
            let _ = session_bus.add_match(
                MatchRule::new_signal(screensaver_interface, "ActiveChanged"),
                move |(is_screen_locked,): (bool,), _, _| {
                    if is_screen_locked {
                        lock_and_notify(&signal_app_handle, VaultLockReason::ScreenLocked);
                    }
                    true
                },
            );
        }
        process_signals_forever(&session_bus);
    }

    /// logind: `Session.Lock` (p. ej. `loginctl lock-session`, COSMIC, KDE) y
    /// `Manager.PrepareForSleep(true)` justo antes de suspender o hibernar.
    fn listen_to_system_bus(app_handle: &AppHandle) {
        let Ok(system_bus) = Connection::new_system() else {
            return;
        };
        let lock_app_handle = app_handle.clone();
        let _ = system_bus.add_match(
            MatchRule::new_signal("org.freedesktop.login1.Session", "Lock"),
            move |(): (), _, _| {
                lock_and_notify(&lock_app_handle, VaultLockReason::ScreenLocked);
                true
            },
        );
        let sleep_app_handle = app_handle.clone();
        let _ = system_bus.add_match(
            MatchRule::new_signal("org.freedesktop.login1.Manager", "PrepareForSleep"),
            move |(is_going_to_sleep,): (bool,), _, _| {
                if is_going_to_sleep {
                    lock_and_notify(&sleep_app_handle, VaultLockReason::SystemSuspended);
                }
                true
            },
        );
        process_signals_forever(&system_bus);
    }

    fn process_signals_forever(bus_connection: &Connection) {
        while bus_connection.process(SIGNAL_POLL_INTERVAL).is_ok() {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHECK_INTERVAL: Duration = Duration::from_secs(5);

    #[test]
    fn regular_checks_are_not_a_suspension() {
        let start_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let mut suspension_detector = SuspensionDetector::new(CHECK_INTERVAL, start_time);

        assert!(!suspension_detector.detect_suspension(start_time + Duration::from_secs(5)));
        assert!(!suspension_detector.detect_suspension(start_time + Duration::from_secs(12)));
    }

    #[test]
    fn large_wall_clock_gap_means_the_system_slept() {
        let start_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let mut suspension_detector = SuspensionDetector::new(CHECK_INTERVAL, start_time);

        assert!(suspension_detector.detect_suspension(start_time + Duration::from_mins(45)));
    }

    #[test]
    fn clock_moving_backwards_is_not_a_suspension() {
        let start_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let mut suspension_detector = SuspensionDetector::new(CHECK_INTERVAL, start_time);

        assert!(!suspension_detector.detect_suspension(start_time - Duration::from_secs(3600)));
    }
}
