//! Patrón Command: cada acción de la interfaz es un valor que se lanza igual desde la paleta,
//! un atajo de teclado o un botón. Las acciones sobre un elemento saben además deshacerse.

use std::time::Duration;

use slint::{ComponentHandle, Timer};
use uuid::Uuid;
use vault_core::application::search_vault_items::VaultItemFilter;
use vault_core::domain::item_content::ItemContentType;
use vault_core::application::vault_service::VaultAttachments;
use vault_core::application::vault_service::VaultItems;

use crate::presentation::app_context::{AppContext, AppEvent, EditorMode};
use crate::presentation::item_drafts::save_request_from_item;
use crate::ui_error::UiResult;
use crate::{ItemEditor, VaultSection, Workspace};

/// Tras cambiar de sección hay que esperar a que exista el campo de búsqueda para enfocarlo.
const FOCUS_AFTER_SECTION_CHANGE_DELAY: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppCommand {
    OpenCommandPalette,
    CreateItem(ItemContentType),
    OpenSection(VaultSection),
    OpenItem(Uuid),
    LockVault,
    FocusSearch,
    SaveOpenEditor,
    CancelOpenEditor,
}

impl AppCommand {
    /// Devuelve `false` si ahora no aplica (p. ej. guardar sin ningún editor abierto), para que
    /// la tecla siga su camino.
    pub fn execute(self, context: &AppContext) -> bool {
        match self {
            Self::OpenCommandPalette => context.events.publish(AppEvent::CommandPaletteRequested),
            Self::CreateItem(content_type) => context.navigator.start_creating_item(content_type),
            Self::OpenSection(section) => context.navigator.open_section(section),
            Self::OpenItem(item_id) => {
                context.navigator.apply_navigation_filter(VaultItemFilter::default());
                context.navigator.select_item(Some(item_id));
            }
            Self::LockVault => context.events.publish(AppEvent::LockRequested),
            Self::FocusSearch => focus_search(context),
            Self::SaveOpenEditor | Self::CancelOpenEditor => {
                if context.navigator.snapshot().editor_mode == EditorMode::Closed {
                    return false;
                }
                context.with_window(|app_window| {
                    let item_editor = app_window.global::<ItemEditor>();
                    if self == Self::SaveOpenEditor { item_editor.invoke_save() } else { item_editor.invoke_cancel() }
                });
            }
        }
        true
    }
}

fn focus_search(context: &AppContext) {
    let was_items_section = context.navigator.snapshot().section == VaultSection::Items;
    context.navigator.open_section(VaultSection::Items);
    let window = context.window().map(|app_window| app_window.as_weak());
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

/// Acción sobre un elemento guardado, con su inversa cuando se puede deshacer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemCommand {
    ToggleFavorite(Uuid),
    MoveToTrash(Uuid),
    RestoreFromTrash(Uuid),
    DeletePermanently(Uuid),
    DeleteAttachment { item_id: Uuid, attachment_id: Uuid },
}

impl ItemCommand {
    pub fn execute(self, context: &AppContext) -> UiResult<()> {
        let mut vault_service = context.shared_vault.lock_service()?;
        match self {
            Self::ToggleFavorite(item_id) => {
                let mut save_request = save_request_from_item(&vault_service.get_item_details(item_id)?);
                save_request.is_favorite = !save_request.is_favorite;
                vault_service.save_item(save_request)?;
            }
            Self::MoveToTrash(item_id) => {
                vault_service.move_item_to_trash(item_id)?;
                drop(vault_service);
                context.navigator.select_item(None);
            }
            Self::RestoreFromTrash(item_id) => vault_service.restore_item_from_trash(item_id)?,
            Self::DeleteAttachment { item_id, attachment_id } => vault_service.delete_attachment(item_id, attachment_id)?,
            Self::DeletePermanently(item_id) => {
                vault_service.delete_item_permanently(item_id)?;
                drop(vault_service);
                context.navigator.select_item(None);
            }
        }
        Ok(())
    }

    /// La acción que deshace esta, si existe (lo borrado para siempre no se puede recuperar).
    pub fn inverse(self) -> Option<Self> {
        match self {
            Self::ToggleFavorite(item_id) => Some(Self::ToggleFavorite(item_id)),
            Self::MoveToTrash(item_id) => Some(Self::RestoreFromTrash(item_id)),
            Self::RestoreFromTrash(item_id) => Some(Self::MoveToTrash(item_id)),
            Self::DeletePermanently(_) | Self::DeleteAttachment { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trash_and_restore_undo_each_other_but_deleting_cannot_be_undone() {
        let item_id = Uuid::new_v4();
        assert_eq!(ItemCommand::MoveToTrash(item_id).inverse(), Some(ItemCommand::RestoreFromTrash(item_id)));
        assert_eq!(ItemCommand::RestoreFromTrash(item_id).inverse(), Some(ItemCommand::MoveToTrash(item_id)));
        assert_eq!(ItemCommand::DeletePermanently(item_id).inverse(), None);
    }
}
