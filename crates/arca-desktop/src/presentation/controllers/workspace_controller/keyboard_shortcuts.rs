//! Atajos de teclado globales (la tabla de teclas está en `presentation::shortcuts`).

use std::time::Duration;

use slint::{ComponentHandle, Timer};
use vault_core::domain::item_content::ItemContentType;

use crate::presentation::app_context::{AppEvent, EditorMode};
use crate::{ItemEditor, Shortcut, VaultSection, Workspace};

use super::WorkspaceController;

/// Tras cambiar de sección hay que esperar a que exista el campo de búsqueda para enfocarlo.
const FOCUS_AFTER_SECTION_CHANGE_DELAY: Duration = Duration::from_millis(50);

impl WorkspaceController {
    // Los métodos generados por Slint no son lo bastante genéricos en el tiempo de vida para pasarlos sin cierre.
    #[allow(clippy::redundant_closure_for_method_calls)]
    pub(super) fn run_shortcut(&self, shortcut: Shortcut) -> bool {
        let editor_is_open = self.context.navigator.snapshot().editor_mode != EditorMode::Closed;
        match shortcut {
            Shortcut::Palette => self.context.events.publish(AppEvent::CommandPaletteRequested),
            Shortcut::NewLogin => self.context.navigator.start_creating_item(ItemContentType::Login),
            Shortcut::Lock => self.context.events.publish(AppEvent::LockRequested),
            Shortcut::Find => self.focus_search(),
            Shortcut::Save if editor_is_open => self.invoke_editor(|item_editor| item_editor.invoke_save()),
            Shortcut::Escape if editor_is_open => self.invoke_editor(|item_editor| item_editor.invoke_cancel()),
            Shortcut::Save | Shortcut::Escape => return false,
        }
        true
    }

    fn invoke_editor(&self, editor_action: impl FnOnce(&ItemEditor<'_>)) {
        if let Some(app_window) = self.context.window() {
            editor_action(&app_window.global::<ItemEditor>());
        }
    }

    fn focus_search(&self) {
        let was_items_section = self.context.navigator.snapshot().section == VaultSection::Items;
        self.context.navigator.open_section(VaultSection::Items);
        let window = self.context.window().map(|app_window| app_window.as_weak());
        let request_focus = move || {
            let Some(app_window) = window.as_ref().and_then(slint::Weak::upgrade) else {
                return;
            };
            let workspace = app_window.global::<Workspace>();
            workspace.set_focus_search_requests(workspace.get_focus_search_requests() + 1);
        };
        if was_items_section {
            request_focus();
            return;
        }
        Timer::single_shot(FOCUS_AFTER_SECTION_CHANGE_DELAY, request_focus);
    }
}
