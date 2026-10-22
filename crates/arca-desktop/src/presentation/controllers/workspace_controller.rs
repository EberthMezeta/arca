//! Barra lateral, lista de elementos, selección y atajos de teclado globales.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use slint::{ComponentHandle, ModelRc, SharedString, Timer, VecModel};
use uuid::Uuid;
use vault_core::application::search_vault_items::{FolderScope, VaultItemFilter, VaultItemSummary, VaultOverview};
use vault_core::domain::item_content::ItemContentType;
use vault_core::domain::vault_item::RevealableField;

use crate::presentation::app_context::{AppContext, AppEvent, EditorMode, WorkspaceState};
use crate::presentation::controllers::clipboard_feedback::copy_with_feedback;
use crate::presentation::item_kinds::{ITEM_KIND_DEFINITIONS, definition_for, definition_for_ui_kind};
use crate::{AppWindow, ItemEditor, ItemPanel, ItemRow, ListState, NavEntry, NavIcon, Sidebar, VaultSection, Workspace};

/// Tras cambiar de sección hay que esperar a que exista el campo de búsqueda para enfocarlo.
const FOCUS_AFTER_SECTION_CHANGE_DELAY: Duration = Duration::from_millis(50);

#[derive(Clone)]
enum NavigationTarget {
    Filter(VaultItemFilter),
    Section(VaultSection),
}

/// Una entrada de la barra lateral antes de convertirse en `NavEntry` (la clave se asigna al publicarla).
struct NavigationEntry {
    label: String,
    icon: NavIcon,
    item_count: Option<usize>,
    is_active: bool,
    folder_id: Option<Uuid>,
    target: NavigationTarget,
}

#[derive(Default)]
struct WorkspaceViewState {
    navigation_targets: HashMap<String, NavigationTarget>,
    visible_summaries: Vec<VaultItemSummary>,
    overview: Option<VaultOverview>,
}

pub struct WorkspaceController {
    context: Rc<AppContext>,
    view_state: RefCell<WorkspaceViewState>,
}

fn is_plain_filter(item_filter: &VaultItemFilter) -> bool {
    !item_filter.should_show_trash
        && !item_filter.should_show_only_favorites
        && item_filter.content_type.is_none()
        && item_filter.tag_name.is_none()
}

impl WorkspaceController {
    pub fn install(context: &Rc<AppContext>, app_window: &AppWindow) -> Rc<Self> {
        let controller = Rc::new(Self { context: Rc::clone(context), view_state: RefCell::default() });
        controller.bind_sidebar(app_window);
        controller.bind_workspace(app_window);

        let event_controller = Rc::clone(&controller);
        context.events.subscribe(move |app_event| event_controller.handle_event(app_event));
        controller
    }

    fn bind_sidebar(self: &Rc<Self>, app_window: &AppWindow) {
        let sidebar = app_window.global::<Sidebar>();
        let activating_controller = Rc::clone(self);
        sidebar.on_activate(move |entry_key| activating_controller.activate_entry(&entry_key));
        let creating_controller = Rc::clone(self);
        sidebar.on_create_folder(move || super::folder_dialogs::ask_new_folder(&creating_controller.context));
        let renaming_controller = Rc::clone(self);
        sidebar.on_rename_folder(move |folder_id| renaming_controller.rename_folder(&folder_id));
        let deleting_controller = Rc::clone(self);
        sidebar.on_delete_folder(move |folder_id| deleting_controller.delete_folder(&folder_id));
    }

    fn bind_workspace(self: &Rc<Self>, app_window: &AppWindow) {
        let workspace = app_window.global::<Workspace>();
        let search_controller = Rc::clone(self);
        workspace.on_search_edited(move |search_text| search_controller.context.navigator.change_search_text(&search_text));
        let selecting_controller = Rc::clone(self);
        workspace.on_select_row(move |row_position| selecting_controller.select_row(row_position));
        let closing_controller = Rc::clone(self);
        workspace.on_close_item(move || closing_controller.context.navigator.select_item(None));
        let moving_controller = Rc::clone(self);
        workspace.on_move_selection(move |selection_offset| moving_controller.move_selection(selection_offset));
        let username_controller = Rc::clone(self);
        workspace.on_copy_username(move |row_position| {
            username_controller.copy_row_field(row_position, "fields.username", RevealableField::Username);
        });
        let password_controller = Rc::clone(self);
        workspace.on_copy_password(move |row_position| {
            password_controller.copy_row_field(row_position, "fields.password", RevealableField::Password);
        });
        let creating_controller = Rc::clone(self);
        workspace.on_create_item(move |ui_kind| {
            creating_controller.context.navigator.start_creating_item(definition_for_ui_kind(ui_kind).content_type);
        });
        let section_controller = Rc::clone(self);
        workspace.on_open_section(move |section| section_controller.context.navigator.open_section(section));
        let import_controller = Rc::clone(self);
        workspace.on_request_import(move || import_controller.context.navigator.open_section(VaultSection::Settings));
        let shortcut_controller = Rc::clone(self);
        workspace.on_shortcut(move |shortcut_name| shortcut_controller.run_shortcut(&shortcut_name));
    }

    fn handle_event(&self, app_event: AppEvent) {
        match app_event {
            AppEvent::SessionUnlocked | AppEvent::VaultDataChanged | AppEvent::LanguageChanged => self.refresh_everything(),
            AppEvent::SectionChanged | AppEvent::FilterChanged => {
                self.refresh_list();
                self.refresh_sidebar();
            }
            AppEvent::SelectionChanged | AppEvent::EditorChanged => {
                self.refresh_selection();
                self.refresh_item_panel();
            }
            AppEvent::SessionLocked => self.clear(),
            AppEvent::LockRequested | AppEvent::CommandPaletteRequested => {}
        }
    }

    fn refresh_everything(&self) {
        self.refresh_list();
        self.refresh_sidebar();
        self.refresh_item_panel();
    }

    // ── Barra lateral ─────────────────────────────────────────────────────────

    fn refresh_sidebar(&self) {
        let Some(app_window) = self.context.window() else {
            return;
        };
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
    }

    #[allow(clippy::too_many_lines)]
    /// Grupos en el orden en que los publica `refresh_sidebar`: biblioteca, tipos, carpetas, etiquetas, papelera, herramientas.
    fn build_navigation_groups(&self, overview: Option<&VaultOverview>, workspace_state: &WorkspaceState) -> [Vec<NavigationEntry>; 6] {
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

    fn activate_entry(&self, entry_key: &str) {
        let navigation_target = self.view_state.borrow().navigation_targets.get(entry_key).cloned();
        match navigation_target {
            Some(NavigationTarget::Filter(item_filter)) => self.context.navigator.apply_navigation_filter(item_filter),
            Some(NavigationTarget::Section(section)) => self.context.navigator.open_section(section),
            None => {}
        }
    }

    fn folder_name(&self, folder_id: Uuid) -> String {
        self.view_state
            .borrow()
            .overview
            .as_ref()
            .and_then(|overview| overview.folders.iter().find(|folder_overview| folder_overview.folder_id == folder_id))
            .map(|folder_overview| folder_overview.folder_name.clone())
            .unwrap_or_default()
    }

    fn rename_folder(&self, folder_id_text: &str) {
        let Ok(folder_id) = Uuid::parse_str(folder_id_text) else {
            return;
        };
        super::folder_dialogs::ask_folder_rename(&self.context, folder_id, self.folder_name(folder_id));
    }

    fn delete_folder(&self, folder_id_text: &str) {
        let Ok(folder_id) = Uuid::parse_str(folder_id_text) else {
            return;
        };
        super::folder_dialogs::confirm_folder_deletion(&self.context, folder_id, &self.folder_name(folder_id));
    }

    // ── Lista ─────────────────────────────────────────────────────────────────

    fn refresh_list(&self) {
        let Some(app_window) = self.context.window() else {
            return;
        };
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

    fn refresh_selection(&self) {
        let Some(app_window) = self.context.window() else {
            return;
        };
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
    }

    fn refresh_item_panel(&self) {
        let Some(app_window) = self.context.window() else {
            return;
        };
        let workspace_state = self.context.navigator.snapshot();
        let item_panel = match (workspace_state.editor_mode, workspace_state.selected_item_id) {
            (EditorMode::Creating(_) | EditorMode::Editing(_), _) => ItemPanel::Editor,
            (EditorMode::Closed, Some(_)) => ItemPanel::Details,
            (EditorMode::Closed, None) => ItemPanel::NothingSelected,
        };
        app_window.global::<Workspace>().set_item_panel(item_panel);
    }

    fn summary_at(&self, row_position: i32) -> Option<VaultItemSummary> {
        let row_position = usize::try_from(row_position).ok()?;
        self.view_state.borrow().visible_summaries.get(row_position).cloned()
    }

    fn select_row(&self, row_position: i32) {
        if let Some(summary) = self.summary_at(row_position) {
            self.context.navigator.select_item(Some(summary.item_id));
        }
    }

    /// Flechas arriba/abajo: se queda en los extremos en vez de dar la vuelta.
    fn move_selection(&self, selection_offset: i32) {
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

    fn copy_row_field(&self, row_position: i32, label_key: &str, requested_field: RevealableField) {
        let Some(summary) = self.summary_at(row_position) else {
            return;
        };
        copy_with_feedback(&self.context, &self.context.text(label_key), |vault_service| {
            vault_service.copy_item_field_to_clipboard(summary.item_id, requested_field)
        });
    }

    // ── Atajos ────────────────────────────────────────────────────────────────

    // Los métodos generados por Slint no son lo bastante genéricos en el tiempo de vida para pasarlos sin cierre.
    #[allow(clippy::redundant_closure_for_method_calls)]
    fn run_shortcut(&self, shortcut_name: &str) -> bool {
        let editor_is_open = self.context.navigator.snapshot().editor_mode != EditorMode::Closed;
        match shortcut_name {
            "palette" => self.context.events.publish(AppEvent::CommandPaletteRequested),
            "new" => self.context.navigator.start_creating_item(ItemContentType::Login),
            "lock" => self.context.events.publish(AppEvent::LockRequested),
            "find" => self.focus_search(),
            "save" if editor_is_open => self.invoke_editor(|item_editor| item_editor.invoke_save()),
            "escape" if editor_is_open => self.invoke_editor(|item_editor| item_editor.invoke_cancel()),
            _ => return false,
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

    fn clear(&self) {
        *self.view_state.borrow_mut() = WorkspaceViewState::default();
        let Some(app_window) = self.context.window() else {
            return;
        };
        let workspace = app_window.global::<Workspace>();
        workspace.set_item_rows(ModelRc::default());
        workspace.set_search_text(SharedString::new());
        workspace.set_selected_row(-1);
        workspace.set_list_state(ListState::Loading);
        workspace.set_item_panel(ItemPanel::NothingSelected);
        workspace.set_section(VaultSection::Items);
        let sidebar = app_window.global::<Sidebar>();
        for clear_group in [
            Sidebar::set_library_entries,
            Sidebar::set_type_entries,
            Sidebar::set_folder_entries,
            Sidebar::set_tag_entries,
            Sidebar::set_trash_entries,
        ] {
            clear_group(&sidebar, ModelRc::default());
        }
    }
}
