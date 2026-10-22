//! Esquema de color del sistema (claro / oscuro) y sus cambios en vivo.
//!
//! winit no lo informa en Wayland (GNOME, COSMIC…): en Linux se lee del portal de escritorio
//! XDG, que es lo que consultan GTK y Qt. En otras plataformas basta con la `Palette` de Slint.
//!
//! El portal rechaza las llamadas de un proceso no volcable (`PR_SET_DUMPABLE = 0`): no puede abrir
//! `/proc/<pid>/root` para identificarlo. Por eso el esquema inicial se lee antes del endurecimiento
//! del proceso; las señales de cambio no pasan por esa comprobación y se escuchan después.

/// `Some(true)` si el sistema prefiere oscuro; `None` si no lo dice. Llamar antes de `process_hardening::apply`.
pub fn read_current() -> Option<bool> {
    #[cfg(target_os = "linux")]
    return xdg_portal::read_current();
    #[cfg(not(target_os = "linux"))]
    None
}

/// Se llama con `true` si el sistema pasa a oscuro. Puede llamarse desde otro hilo.
pub fn watch_changes(on_scheme_changed: impl Fn(bool) + Send + 'static) {
    #[cfg(target_os = "linux")]
    xdg_portal::watch_changes(on_scheme_changed);
    #[cfg(not(target_os = "linux"))]
    let _ = on_scheme_changed;
}

#[cfg(target_os = "linux")]
mod xdg_portal {
    use std::thread;
    use std::time::Duration;

    use dbus::arg::Variant;
    use dbus::blocking::Connection;
    use dbus::message::MatchRule;

    const PORTAL_SERVICE: &str = "org.freedesktop.portal.Desktop";
    const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
    const SETTINGS_INTERFACE: &str = "org.freedesktop.portal.Settings";
    const APPEARANCE_NAMESPACE: &str = "org.freedesktop.appearance";
    const COLOR_SCHEME_KEY: &str = "color-scheme";
    const PORTAL_CALL_TIMEOUT: Duration = Duration::from_secs(2);
    const SIGNAL_POLL_INTERVAL: Duration = Duration::from_mins(1);

    /// Valores del portal: 0 sin preferencia, 1 oscuro, 2 claro.
    /// Sin preferencia se deja el esquema que detectó Slint.
    fn prefers_dark(portal_value: u32) -> Option<bool> {
        match portal_value {
            1 => Some(true),
            2 => Some(false),
            _ => None,
        }
    }

    pub fn read_current() -> Option<bool> {
        let session_bus = Connection::new_session().ok()?;
        read_color_scheme(&session_bus).and_then(prefers_dark)
    }

    pub fn watch_changes(on_scheme_changed: impl Fn(bool) + Send + 'static) {
        thread::spawn(move || listen_to_portal(on_scheme_changed));
    }

    /// `ReadOne` es la llamada actual; los portales antiguos solo tienen `Read` (variante dentro de variante).
    fn read_color_scheme(session_bus: &Connection) -> Option<u32> {
        let portal = session_bus.with_proxy(PORTAL_SERVICE, PORTAL_PATH, PORTAL_CALL_TIMEOUT);
        let arguments = (APPEARANCE_NAMESPACE, COLOR_SCHEME_KEY);
        if let Ok((Variant(portal_value),)) = portal.method_call::<(Variant<u32>,), _, _, _>(SETTINGS_INTERFACE, "ReadOne", arguments) {
            return Some(portal_value);
        }
        let (Variant(Variant(portal_value)),): (Variant<Variant<u32>>,) = portal.method_call(SETTINGS_INTERFACE, "Read", arguments).ok()?;
        Some(portal_value)
    }

    /// Sin bus de sesión o sin portal no hay nada que escuchar: se queda el esquema inicial.
    fn listen_to_portal(on_scheme_changed: impl Fn(bool) + Send + 'static) {
        let Ok(session_bus) = Connection::new_session() else {
            return;
        };
        let _ = session_bus.add_match(
            MatchRule::new_signal(SETTINGS_INTERFACE, "SettingChanged"),
            move |(namespace, key, Variant(portal_value)): (String, String, Variant<u32>), _, _| {
                let is_color_scheme = namespace == APPEARANCE_NAMESPACE && key == COLOR_SCHEME_KEY;
                if let Some(is_dark) = prefers_dark(portal_value).filter(|_| is_color_scheme) {
                    on_scheme_changed(is_dark);
                }
                true
            },
        );
        while session_bus.process(SIGNAL_POLL_INTERVAL).is_ok() {}
    }
}
