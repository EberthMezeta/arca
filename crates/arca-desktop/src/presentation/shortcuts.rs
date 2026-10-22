//! Tabla de atajos de teclado: qué tecla dispara cada `Shortcut` y cómo se muestra.
//! La interfaz solo pasa la tecla pulsada; añadir un atajo es añadir una fila.

use crate::Shortcut;

pub struct ControlKeyShortcut {
    /// Tecla en minúsculas tal como la entrega Slint en `KeyEvent.text`.
    pub key: &'static str,
    pub shortcut: Shortcut,
    pub label: &'static str,
}

pub const CONTROL_KEY_SHORTCUTS: &[ControlKeyShortcut] = &[
    ControlKeyShortcut { key: "k", shortcut: Shortcut::Palette, label: "Ctrl K" },
    ControlKeyShortcut { key: "n", shortcut: Shortcut::NewLogin, label: "Ctrl N" },
    ControlKeyShortcut { key: "l", shortcut: Shortcut::Lock, label: "Ctrl L" },
    ControlKeyShortcut { key: "f", shortcut: Shortcut::Find, label: "Ctrl F" },
    ControlKeyShortcut { key: "s", shortcut: Shortcut::Save, label: "Ctrl S" },
];

pub fn shortcut_for_control_key(key: &str) -> Option<Shortcut> {
    CONTROL_KEY_SHORTCUTS.iter().find(|entry| entry.key == key).map(|entry| entry.shortcut)
}

/// Etiqueta para mostrar junto a un comando («Ctrl N»), o vacía si no tiene tecla.
pub fn label_for(shortcut: Shortcut) -> &'static str {
    CONTROL_KEY_SHORTCUTS.iter().find(|entry| entry.shortcut == shortcut).map_or("", |entry| entry.label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_and_shortcut_appears_once() {
        for (position, entry) in CONTROL_KEY_SHORTCUTS.iter().enumerate() {
            let later_entries = &CONTROL_KEY_SHORTCUTS[position + 1..];
            assert!(later_entries.iter().all(|later| later.key != entry.key && later.shortcut != entry.shortcut));
        }
    }

    #[test]
    fn looks_up_keys_and_labels() {
        assert_eq!(shortcut_for_control_key("k"), Some(Shortcut::Palette));
        assert_eq!(shortcut_for_control_key("x"), None);
        assert_eq!(label_for(Shortcut::Lock), "Ctrl L");
        assert_eq!(label_for(Shortcut::Escape), "");
    }
}
