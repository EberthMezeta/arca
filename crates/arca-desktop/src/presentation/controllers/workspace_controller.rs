//! Barra lateral, lista de elementos, selección y atajos de teclado globales.

mod item_list;
mod sidebar;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, SharedString};
use vault_core::application::search_vault_items::{VaultItemFilter, VaultItemSummary, VaultOverview};
use vault_core::domain::vault_item::RevealableField;

use crate::presentation::app_context::{AppContext, AppEvent};
use crate::presentation::item_kinds::definition_for_ui_kind;
use crate::presentation::app_commands::AppCommand;
use crate::presentation::shortcuts::{self, command_for_control_key};
use crate::{AppWindow, ItemPanel, ListState, Sidebar, VaultSection, Workspace};

#[derive(Clone)]
enum NavigationTarget {
    Filter(VaultItemFilter),
    Section(VaultSection),
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
        sidebar.set_lock_shortcut_label(shortcuts::label_for(AppCommand::LockVault).into());
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
        workspace.on_escape_pressed(move || AppCommand::CancelOpenEditor.execute(&shortcut_controller.context));
        let control_key_controller = Rc::clone(self);
        workspace.on_control_key_shortcut(move |key| {
            command_for_control_key(&key).is_some_and(|command| command.execute(&control_key_controller.context))
        });
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

    fn clear(&self) {
        *self.view_state.borrow_mut() = WorkspaceViewState::default();
        self.context.with_window(|app_window| {
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
        });
    }
}
