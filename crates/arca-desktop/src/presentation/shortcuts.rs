//! Tabla de atajos de teclado: qué tecla lanza cada `AppCommand` y cómo se muestra.
//! La interfaz solo pasa la tecla pulsada; añadir un atajo es añadir una fila.

use vault_core::domain::item_content::ItemContentType;

use crate::presentation::app_commands::AppCommand;

pub struct ControlKeyShortcut {
    /// Tecla en minúsculas tal como la entrega Slint en `KeyEvent.text`.
    pub key: &'static str,
    pub command: AppCommand,
    pub label: &'static str,
}

pub const CONTROL_KEY_SHORTCUTS: &[ControlKeyShortcut] = &[
    ControlKeyShortcut { key: "k", command: AppCommand::OpenCommandPalette, label: "Ctrl K" },
    ControlKeyShortcut { key: "n", command: AppCommand::CreateItem(ItemContentType::Login), label: "Ctrl N" },
    ControlKeyShortcut { key: "l", command: AppCommand::LockVault, label: "Ctrl L" },
    ControlKeyShortcut { key: "f", command: AppCommand::FocusSearch, label: "Ctrl F" },
    ControlKeyShortcut { key: "s", command: AppCommand::SaveOpenEditor, label: "Ctrl S" },
];

pub fn command_for_control_key(key: &str) -> Option<AppCommand> {
    CONTROL_KEY_SHORTCUTS.iter().find(|entry| entry.key == key).map(|entry| entry.command)
}

/// Etiqueta para mostrar junto a un comando («Ctrl N»), o vacía si no tiene tecla.
pub fn label_for(command: AppCommand) -> &'static str {
    CONTROL_KEY_SHORTCUTS.iter().find(|entry| entry.command == command).map_or("", |entry| entry.label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_and_command_appears_once() {
        for (position, entry) in CONTROL_KEY_SHORTCUTS.iter().enumerate() {
            let later_entries = &CONTROL_KEY_SHORTCUTS[position + 1..];
            assert!(later_entries.iter().all(|later| later.key != entry.key && later.command != entry.command));
        }
    }

    #[test]
    fn looks_up_keys_and_labels() {
        assert_eq!(command_for_control_key("k"), Some(AppCommand::OpenCommandPalette));
        assert_eq!(command_for_control_key("x"), None);
        assert_eq!(label_for(AppCommand::LockVault), "Ctrl L");
        assert_eq!(label_for(AppCommand::CancelOpenEditor), "");
    }
}
