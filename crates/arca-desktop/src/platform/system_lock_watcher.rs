//! Bloquea la bóveda cuando el equipo se suspende o se bloquea la pantalla.
//!
//! - En todas las plataformas: `SuspensionDetector` nota la suspensión por el salto del reloj
//!   de pared entre dos comprobaciones del planificador de auto-bloqueo.
//! - En Linux, además, se escuchan las señales de D-Bus del bloqueo de pantalla y de logind,
//!   que avisan al momento (antes de dormir y aunque el bloqueo no vaya seguido de suspensión).

use std::time::{Duration, SystemTime};

use crate::shared_vault::VaultLocker;

/// Margen sobre el intervalo esperado antes de considerar que hubo una suspensión.
const SUSPENSION_GAP_TOLERANCE: Duration = Duration::from_secs(30);

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

pub fn start(vault_locker: &VaultLocker) {
    #[cfg(target_os = "linux")]
    linux_session_signals::start(vault_locker);
    #[cfg(not(target_os = "linux"))]
    let _ = vault_locker;
}

#[cfg(target_os = "linux")]
mod linux_session_signals {
    use std::thread;
    use std::time::Duration;

    use dbus::Path;
    use dbus::blocking::Connection;
    use dbus::message::MatchRule;
    use crate::shared_vault::VaultLocker;
    use crate::vault_events::VaultLockReason;

    const SIGNAL_POLL_INTERVAL: Duration = Duration::from_secs(60);
    const LOGIN_MANAGER_SERVICE: &str = "org.freedesktop.login1";
    const LOGIN_MANAGER_PATH: &str = "/org/freedesktop/login1";
    const LOGIN_MANAGER_INTERFACE: &str = "org.freedesktop.login1.Manager";
    const LOGIN_MANAGER_CALL_TIMEOUT: Duration = Duration::from_secs(5);

    /// Bloqueo de pantalla del escritorio (estándar freedesktop y GNOME): `ActiveChanged(bool)`.
    const SCREENSAVER_INTERFACES: [&str; 2] = ["org.freedesktop.ScreenSaver", "org.gnome.ScreenSaver"];

    pub fn start(vault_locker: &VaultLocker) {
        let session_bus_locker = vault_locker.clone();
        thread::spawn(move || listen_to_session_bus(&session_bus_locker));
        let system_bus_locker = vault_locker.clone();
        thread::spawn(move || listen_to_system_bus(&system_bus_locker));
    }

    /// Sin bus de sesión (p. ej. sesión remota mínima) no hay nada que escuchar: se ignora.
    fn listen_to_session_bus(vault_locker: &VaultLocker) {
        let Ok(session_bus) = Connection::new_session() else {
            return;
        };
        for screensaver_interface in SCREENSAVER_INTERFACES {
            let signal_locker = vault_locker.clone();
            let _ = session_bus.add_match(
                MatchRule::new_signal(screensaver_interface, "ActiveChanged"),
                move |(is_screen_locked,): (bool,), _, _| {
                    if is_screen_locked {
                        signal_locker.lock_and_notify(VaultLockReason::ScreenLocked);
                    }
                    true
                },
            );
        }
        process_signals_forever(&session_bus);
    }

    /// Sesión de logind a la que pertenece este proceso. Así solo cuenta el bloqueo de
    /// NUESTRA sesión: otro usuario del equipo no puede bloquearnos la bóveda con el suyo.
    fn own_login_session(system_bus: &Connection) -> Option<Path<'static>> {
        let login_manager = system_bus.with_proxy(LOGIN_MANAGER_SERVICE, LOGIN_MANAGER_PATH, LOGIN_MANAGER_CALL_TIMEOUT);
        let (session_path,): (Path<'static>,) =
            login_manager.method_call(LOGIN_MANAGER_INTERFACE, "GetSessionByPID", (std::process::id(),)).ok()?;
        Some(session_path)
    }

    /// logind: `Session.Lock` (p. ej. `loginctl lock-session`, COSMIC, KDE) y
    /// `Manager.PrepareForSleep(true)` justo antes de suspender o hibernar.
    fn listen_to_system_bus(vault_locker: &VaultLocker) {
        let Ok(system_bus) = Connection::new_system() else {
            return;
        };
        let session_lock_rule = MatchRule::new_signal("org.freedesktop.login1.Session", "Lock");
        // Sin sesión propia (p. ej. lanzada desde un servicio) se escucha cualquiera: mejor bloquear de más.
        let session_lock_rule = match own_login_session(&system_bus) {
            Some(session_path) => session_lock_rule.with_path(session_path),
            None => session_lock_rule,
        };
        let screen_lock_locker = vault_locker.clone();
        let _ = system_bus.add_match(
            session_lock_rule,
            move |(): (), _, _| {
                screen_lock_locker.lock_and_notify(VaultLockReason::ScreenLocked);
                true
            },
        );
        let sleep_locker = vault_locker.clone();
        let _ = system_bus.add_match(
            MatchRule::new_signal(LOGIN_MANAGER_INTERFACE, "PrepareForSleep"),
            move |(is_going_to_sleep,): (bool,), _, _| {
                if is_going_to_sleep {
                    sleep_locker.lock_and_notify(VaultLockReason::SystemSuspended);
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
