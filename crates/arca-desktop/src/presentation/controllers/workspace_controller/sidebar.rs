//! Barra lateral: secciones, filtros, carpetas y etiquetas con sus recuentos.

use std::collections::HashMap;

use slint::{ComponentHandle, ModelRc, VecModel};
use uuid::Uuid;
use vault_core::application::search_vault_items::{FolderScope, VaultItemFilter, VaultOverview};

use crate::presentation::app_context::WorkspaceState;
use crate::presentation::item_kinds::ITEM_KIND_DEFINITIONS;
use crate::{NavEntry, NavIcon, Sidebar, VaultSection};

use super::{NavigationTarget, WorkspaceController};

/// Biblioteca, tipos, carpetas, etiquetas, papelera y herramientas (un modelo de Slint por grupo).
const NAVIGATION_GROUP_COUNT: usize = 6;

/// Una entrada de la barra lateral antes de convertirse en `NavEntry` (la clave se asigna al publicarla).
struct NavigationEntry {
    label: String,
    icon: NavIcon,
    item_count: Option<usize>,
    is_active: bool,
    folder_id: Option<Uuid>,
    target: NavigationTarget,
}

fn is_plain_filter(item_filter: &VaultItemFilter) -> bool {
    !item_filter.should_show_trash
        && !item_filter.should_show_only_favorites
        && item_filter.content_type.is_none()
        && item_filter.tag_name.is_none()
}

impl WorkspaceController {
    pub(super) fn refresh_sidebar(&self) {
        self.context.with_window(|app_window| {
            let overview = self.context.shared_vault.lock_service().and_then(|vault_service| Ok(vault_service.vault_overview()?)).ok();
            let workspace_state = self.context.navigator.snapshot();
            let entry_groups = self.build_navigation_groups(overview.as_ref(), &workspace_state);

            let mut navigation_targets = HashMap::new();
            let mut group_models = entry_groups.into_iter().enumerate().map(|(group_position, navigation_entries)| {
                let nav_entries: Vec<NavEntry> = navigation_entries
                    .into_iter()
                    .enumerate()
                    .map(|(entry_position, navigation_entry)| {
                        let entry_key = format!("{group_position}:{entry_position}");
                        navigation_targets.insert(entry_key.clone(), navigation_entry.target);
                        NavEntry {
                            entry_key: entry_key.into(),
                            label: navigation_entry.label.into(),
                            icon: navigation_entry.icon,
                            item_count: navigation_entry.item_count.map(|item_count| item_count.to_string()).unwrap_or_default().into(),
                            is_active: navigation_entry.is_active,
                            folder_id: navigation_entry.folder_id.map(|folder_id| folder_id.to_string()).unwrap_or_default().into(),
                        }
                    })
                    .collect();
                ModelRc::new(VecModel::from(nav_entries))
            });

            let sidebar = app_window.global::<Sidebar>();
            let mut next_group = || group_models.next().unwrap_or_default();
            sidebar.set_library_entries(next_group());
            sidebar.set_type_entries(next_group());
            sidebar.set_folder_entries(next_group());
            sidebar.set_tag_entries(next_group());
            sidebar.set_trash_entries(next_group());
            sidebar.set_tool_entries(next_group());

            let mut view_state = self.view_state.borrow_mut();
            view_state.navigation_targets = navigation_targets;
            view_state.overview = overview;
        });
    }

    #[allow(clippy::too_many_lines)]
    /// Grupos en el orden en que los publica `refresh_sidebar`: biblioteca, tipos, carpetas, etiquetas, papelera, herramientas.
    fn build_navigation_groups(
        &self,
        overview: Option<&VaultOverview>,
        workspace_state: &WorkspaceState,
    ) -> [Vec<NavigationEntry>; NAVIGATION_GROUP_COUNT] {
        let item_filter = &workspace_state.item_filter;
        let is_items_section = workspace_state.section == VaultSection::Items;
        let is_active = |matches_filter: bool| is_items_section && matches_filter;
        let filter_entry = |label: String, icon: NavIcon, item_count: Option<usize>, matches_filter: bool, target_filter: VaultItemFilter| {
            NavigationEntry {
                label,
                icon,
                item_count,
                is_active: is_active(matches_filter),
                folder_id: None,
                target: NavigationTarget::Filter(target_filter),
            }
        };

        let library_entries = vec![
            filter_entry(
                self.context.text("navigation.allItems"),
                NavIcon::Layers,
                overview.map(|overview| overview.total_item_count),
                is_plain_filter(item_filter) && item_filter.folder_scope == FolderScope::AllFolders,
                VaultItemFilter::default(),
            ),
            filter_entry(
                self.context.text("navigation.favorites"),
                NavIcon::Star,
                overview.map(|overview| overview.favorite_item_count),
                item_filter.should_show_only_favorites,
                VaultItemFilter { should_show_only_favorites: true, ..VaultItemFilter::default() },
            ),
        ];

        let type_entries = ITEM_KIND_DEFINITIONS
            .iter()
            .map(|definition| {
                let item_count = overview
                    .map(|overview| overview.content_type_item_counts.get(&definition.content_type).copied().unwrap_or(0));
                filter_entry(
                    self.context.text(definition.plural_label_key),
                    definition.nav_icon,
                    item_count,
                    item_filter.content_type == Some(definition.content_type),
                    VaultItemFilter { content_type: Some(definition.content_type), ..VaultItemFilter::default() },
                )
            })
            .collect();

        let folder_overviews = overview.map(|overview| overview.folders.as_slice()).unwrap_or_default();
        let mut folder_entries: Vec<NavigationEntry> = folder_overviews
            .iter()
            .map(|folder_overview| {
                let folder_scope = FolderScope::SpecificFolder { folder_id: folder_overview.folder_id };
                NavigationEntry {
                    folder_id: Some(folder_overview.folder_id),
                    ..filter_entry(
                        folder_overview.folder_name.clone(),
                        NavIcon::Folder,
                        Some(folder_overview.item_count),
                        is_plain_filter(item_filter) && item_filter.folder_scope == folder_scope,
                        VaultItemFilter { folder_scope, ..VaultItemFilter::default() },
                    )
                }
            })
            .collect();
        let items_without_folder_count = overview.map_or(0, |overview| overview.items_without_folder_count);
        if items_without_folder_count > 0 && !folder_entries.is_empty() {
            folder_entries.push(filter_entry(
                self.context.text("navigation.noFolder"),
                NavIcon::FolderMuted,
                Some(items_without_folder_count),
                is_plain_filter(item_filter) && item_filter.folder_scope == FolderScope::WithoutFolder,
                VaultItemFilter { folder_scope: FolderScope::WithoutFolder, ..VaultItemFilter::default() },
            ));
        }

        let tag_entries = overview
            .map(|overview| overview.tag_item_counts.iter().collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
            .map(|(tag_name, item_count)| {
                filter_entry(
                    tag_name.clone(),
                    NavIcon::Hash,
                    Some(*item_count),
                    item_filter.tag_name.as_deref() == Some(tag_name.as_str()),
                    VaultItemFilter { tag_name: Some(tag_name.clone()), ..VaultItemFilter::default() },
                )
            })
            .collect();

        let trash_entries = vec![filter_entry(
            self.context.text("navigation.trash"),
            NavIcon::Trash,
            overview.map(|overview| overview.trash_item_count),
            item_filter.should_show_trash,
            VaultItemFilter { should_show_trash: true, ..VaultItemFilter::default() },
        )];

        let tool_entries = [
            (VaultSection::Generator, NavIcon::Generator, "navigation.generator"),
            (VaultSection::Health, NavIcon::Health, "navigation.health"),
            (VaultSection::Settings, NavIcon::Settings, "navigation.settings"),
        ]
        .into_iter()
        .map(|(section, icon, label_key)| NavigationEntry {
            label: self.context.text(label_key),
            icon,
            item_count: None,
            is_active: workspace_state.section == section,
            folder_id: None,
            target: NavigationTarget::Section(section),
        })
        .collect();

        [library_entries, type_entries, folder_entries, tag_entries, trash_entries, tool_entries]
    }

    pub(super) fn activate_entry(&self, entry_key: &str) {
        let navigation_target = self.view_state.borrow().navigation_targets.get(entry_key).cloned();
        match navigation_target {
            Some(NavigationTarget::Filter(item_filter)) => self.context.navigator.apply_navigation_filter(item_filter),
            Some(NavigationTarget::Section(section)) => self.context.navigator.open_section(section),
            None => {}
        }
    }

    pub(super) fn folder_name(&self, folder_id: Uuid) -> String {
        self.view_state
            .borrow()
            .overview
            .as_ref()
            .and_then(|overview| overview.folders.iter().find(|folder_overview| folder_overview.folder_id == folder_id))
            .map(|folder_overview| folder_overview.folder_name.clone())
            .unwrap_or_default()
    }

    pub(super) fn rename_folder(&self, folder_id_text: &str) {
        let Ok(folder_id) = Uuid::parse_str(folder_id_text) else {
            return;
        };
        crate::presentation::controllers::folder_dialogs::ask_folder_rename(&self.context, folder_id, self.folder_name(folder_id));
    }

    pub(super) fn delete_folder(&self, folder_id_text: &str) {
        let Ok(folder_id) = Uuid::parse_str(folder_id_text) else {
            return;
        };
        crate::presentation::controllers::folder_dialogs::confirm_folder_deletion(&self.context, folder_id, &self.folder_name(folder_id));
    }
}
