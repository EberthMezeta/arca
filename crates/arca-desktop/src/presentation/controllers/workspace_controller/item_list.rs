//! Lista de elementos: filtro activo, selección y copiado rápido desde una fila.


use slint::{ComponentHandle, ModelRc, VecModel};
use uuid::Uuid;
use vault_core::application::search_vault_items::{FolderScope, VaultItemFilter, VaultItemSummary, VaultOverview};
use vault_core::domain::item_content::ItemContentType;
use vault_core::domain::vault_item::RevealableField;

use crate::presentation::app_context::EditorMode;
use crate::presentation::controllers::clipboard_feedback::copy_with_feedback;
use crate::presentation::item_kinds::definition_for;
use crate::{ItemPanel, ItemRow, ListState, Workspace};

use super::WorkspaceController;

impl WorkspaceController {
    pub(super) fn refresh_list(&self) {
        self.context.with_window(|app_window| {
            let workspace_state = self.context.navigator.snapshot();
            let search_result = self.context.shared_vault.lock_service().and_then(|vault_service| {
                let summaries = vault_service.search_items(&workspace_state.item_filter)?;
                let overview = vault_service.vault_overview()?;
                Ok((summaries, overview))
            });
            let Ok((visible_summaries, overview)) = search_result else {
                return;
            };

            let workspace = app_window.global::<Workspace>();
            workspace.set_section(workspace_state.section);
            workspace.set_list_title(self.describe_active_filter(&workspace_state.item_filter, &overview).into());
            let is_vault_completely_empty = overview.total_item_count == 0 && !workspace_state.item_filter.should_show_trash;
            let list_state = match (is_vault_completely_empty, visible_summaries.is_empty()) {
                (true, _) => ListState::VaultEmpty,
                (false, false) => ListState::ItemList,
                (false, true) => ListState::NoMatches,
            };
            workspace.set_list_state(list_state);
            self.publish_no_matches_texts(&workspace, &workspace_state.item_filter);

            let item_rows: Vec<ItemRow> = visible_summaries
                .iter()
                .map(|summary| ItemRow {
                    title: summary.title.clone().into(),
                    subtitle: summary.subtitle.clone().into(),
                    kind: definition_for(summary.content_type).ui_kind,
                    is_favorite: summary.is_favorite,
                    has_attachments: summary.attachment_count > 0,
                    can_copy_username: summary.content_type == ItemContentType::Login && !summary.subtitle.is_empty(),
                    can_copy_password: summary.has_password,
                })
                .collect();
            workspace.set_item_rows(ModelRc::new(VecModel::from(item_rows)));
            let result_count = i64::try_from(visible_summaries.len()).unwrap_or(i64::MAX);
            workspace.set_result_count(self.context.count_text("search.resultCount", result_count, &[]).into());

            self.view_state.borrow_mut().visible_summaries = visible_summaries;
            self.refresh_selection();
        });
    }

    /// Título de la lista según el filtro activo (orden de prioridad declarativo).
    fn describe_active_filter(&self, item_filter: &VaultItemFilter, overview: &VaultOverview) -> String {
        let folder_name = |folder_id: Uuid| {
            overview
                .folders
                .iter()
                .find(|folder_overview| folder_overview.folder_id == folder_id)
                .map(|folder_overview| folder_overview.folder_name.clone())
                .unwrap_or_default()
        };
        if item_filter.should_show_trash {
            return self.context.text("navigation.trash");
        }
        if item_filter.should_show_only_favorites {
            return self.context.text("navigation.favorites");
        }
        if let Some(content_type) = item_filter.content_type {
            return self.context.text(definition_for(content_type).plural_label_key);
        }
        if let Some(tag_name) = &item_filter.tag_name {
            return format!("#{tag_name}");
        }
        match item_filter.folder_scope {
            FolderScope::WithoutFolder => self.context.text("navigation.noFolder"),
            FolderScope::SpecificFolder { folder_id } => folder_name(folder_id),
            FolderScope::AllFolders => self.context.text("navigation.allItems"),
        }
    }

    fn publish_no_matches_texts(&self, workspace: &Workspace<'_>, item_filter: &VaultItemFilter) {
        let search_text = item_filter.search_text.clone().unwrap_or_default();
        let headline_key = match (search_text.is_empty(), item_filter.should_show_trash) {
            (false, _) => "emptyStates.noResultsTitle",
            (true, true) => "emptyStates.trashEmptyTitle",
            (true, false) => "emptyStates.sectionEmptyTitle",
        };
        let explanation = if search_text.is_empty() {
            String::new()
        } else {
            self.context.text_with("emptyStates.noResultsText", &[("searchText", &search_text)])
        };
        workspace.set_no_matches_headline(self.context.text(headline_key).into());
        workspace.set_no_matches_text(explanation.into());
        workspace.set_no_matches_is_trash(item_filter.should_show_trash);
    }

    pub(super) fn refresh_selection(&self) {
        self.context.with_window(|app_window| {
            let selected_item_id = self.context.navigator.snapshot().selected_item_id;
            let selected_position = self
                .view_state
                .borrow()
                .visible_summaries
                .iter()
                .position(|summary| Some(summary.item_id) == selected_item_id)
                .and_then(|position| i32::try_from(position).ok())
                .unwrap_or(-1);
            app_window.global::<Workspace>().set_selected_row(selected_position);
        });
    }

    pub(super) fn refresh_item_panel(&self) {
        self.context.with_window(|app_window| {
            let workspace_state = self.context.navigator.snapshot();
            let item_panel = match (workspace_state.editor_mode, workspace_state.selected_item_id) {
                (EditorMode::Creating(_) | EditorMode::Editing(_), _) => ItemPanel::Editor,
                (EditorMode::Closed, Some(_)) => ItemPanel::Details,
                (EditorMode::Closed, None) => ItemPanel::NothingSelected,
            };
            app_window.global::<Workspace>().set_item_panel(item_panel);
        });
    }

    fn summary_at(&self, row_position: i32) -> Option<VaultItemSummary> {
        let row_position = usize::try_from(row_position).ok()?;
        self.view_state.borrow().visible_summaries.get(row_position).cloned()
    }

    pub(super) fn select_row(&self, row_position: i32) {
        if let Some(summary) = self.summary_at(row_position) {
            self.context.navigator.select_item(Some(summary.item_id));
        }
    }

    /// Flechas arriba/abajo: se queda en los extremos en vez de dar la vuelta.
    pub(super) fn move_selection(&self, selection_offset: i32) {
        let (visible_count, current_position) = {
            let view_state = self.view_state.borrow();
            let selected_item_id = self.context.navigator.snapshot().selected_item_id;
            let current_position =
                view_state.visible_summaries.iter().position(|summary| Some(summary.item_id) == selected_item_id);
            (view_state.visible_summaries.len(), current_position)
        };
        if visible_count == 0 {
            return;
        }
        let last_position = i32::try_from(visible_count - 1).unwrap_or(i32::MAX);
        let next_position = current_position
            .and_then(|position| i32::try_from(position).ok())
            .map_or(0, |position| (position + selection_offset).clamp(0, last_position));
        self.select_row(next_position);
    }

    pub(super) fn copy_row_field(&self, row_position: i32, label_key: &str, requested_field: RevealableField) {
        let Some(summary) = self.summary_at(row_position) else {
            return;
        };
        copy_with_feedback(&self.context, &self.context.text(label_key), |vault_service| {
            vault_service.copy_item_field_to_clipboard(summary.item_id, requested_field)
        });
    }
}
