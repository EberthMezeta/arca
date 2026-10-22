//! Estado compartido por los controladores. Vive en el hilo de la interfaz (`Rc`, no `Arc`).

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::rc::Rc;

use slint::ComponentHandle;
use uuid::Uuid;
use vault_core::application::search_vault_items::{FolderScope, VaultItemFilter};
use vault_core::domain::item_content::ItemContentType;

use crate::platform::ui_preferences_store::{UiPreferences, UiPreferencesStore};
use crate::presentation::controllers::dialog_controller::DialogService;
use crate::presentation::controllers::toast_controller::ToastCenter;
use crate::presentation::translator::Translator;
use crate::shared_vault::SharedVault;
use crate::{AppWindow, VaultSection};

/// Lo que puede cambiar en la app y a lo que reaccionan los controladores (patrón Observer).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEvent {
    SessionUnlocked,
    SessionLocked,
    SectionChanged,
    FilterChanged,
    SelectionChanged,
    EditorChanged,
    VaultDataChanged,
    LanguageChanged,
    LockRequested,
    CommandPaletteRequested,
}

type EventSubscriber = Box<dyn Fn(AppEvent)>;

/// Los eventos publicados mientras se reparte otro se encolan: así ningún controlador
/// se vuelve a ejecutar mientras aún tiene su estado prestado.
#[derive(Default)]
pub struct EventBus {
    subscribers: RefCell<Vec<EventSubscriber>>,
    pending_events: RefCell<VecDeque<AppEvent>>,
    is_dispatching: Cell<bool>,
}

impl EventBus {
    pub fn subscribe(&self, subscriber: impl Fn(AppEvent) + 'static) {
        self.subscribers.borrow_mut().push(Box::new(subscriber));
    }

    pub fn publish(&self, app_event: AppEvent) {
        self.pending_events.borrow_mut().push_back(app_event);
        if self.is_dispatching.replace(true) {
            return;
        }
        loop {
            let next_event = self.pending_events.borrow_mut().pop_front();
            let Some(next_event) = next_event else {
                break;
            };
            self.subscribers.borrow().iter().for_each(|subscriber| subscriber(next_event));
        }
        self.is_dispatching.set(false);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    Closed,
    Creating(ItemContentType),
    Editing(Uuid),
}

#[derive(Debug, Clone)]
pub struct WorkspaceState {
    pub section: VaultSection,
    pub item_filter: VaultItemFilter,
    pub selected_item_id: Option<Uuid>,
    pub editor_mode: EditorMode,
}

impl Default for WorkspaceState {
    fn default() -> Self {
        Self {
            section: VaultSection::Items,
            item_filter: VaultItemFilter::default(),
            selected_item_id: None,
            editor_mode: EditorMode::Closed,
        }
    }
}

/// Única puerta para cambiar la navegación: actualiza el estado y avisa a quien le interese.
pub struct WorkspaceNavigator {
    workspace_state: RefCell<WorkspaceState>,
    events: Rc<EventBus>,
}

impl WorkspaceNavigator {
    fn new(events: Rc<EventBus>) -> Self {
        Self { workspace_state: RefCell::new(WorkspaceState::default()), events }
    }

    pub fn snapshot(&self) -> WorkspaceState {
        self.workspace_state.borrow().clone()
    }

    fn update(&self, change: impl FnOnce(&mut WorkspaceState), published_events: &[AppEvent]) {
        change(&mut self.workspace_state.borrow_mut());
        for app_event in published_events {
            self.events.publish(*app_event);
        }
    }

    pub fn open_section(&self, section: VaultSection) {
        self.update(|workspace_state| workspace_state.section = section, &[AppEvent::SectionChanged]);
    }

    /// Al cambiar de carpeta/tipo/etiqueta se conserva la búsqueda escrita.
    pub fn apply_navigation_filter(&self, item_filter: VaultItemFilter) {
        self.update(
            |workspace_state| {
                let search_text = workspace_state.item_filter.search_text.take();
                *workspace_state = WorkspaceState {
                    item_filter: VaultItemFilter { search_text, ..item_filter },
                    ..WorkspaceState::default()
                };
            },
            &[AppEvent::SectionChanged, AppEvent::FilterChanged, AppEvent::SelectionChanged, AppEvent::EditorChanged],
        );
    }

    pub fn change_search_text(&self, search_text: &str) {
        let search_text = Some(search_text.to_string()).filter(|search_text| !search_text.is_empty());
        self.update(|workspace_state| workspace_state.item_filter.search_text = search_text, &[AppEvent::FilterChanged]);
    }

    pub fn select_item(&self, selected_item_id: Option<Uuid>) {
        self.update(
            |workspace_state| {
                workspace_state.selected_item_id = selected_item_id;
                workspace_state.editor_mode = EditorMode::Closed;
            },
            &[AppEvent::SelectionChanged, AppEvent::EditorChanged],
        );
    }

    pub fn start_creating_item(&self, content_type: ItemContentType) {
        self.update(
            |workspace_state| {
                workspace_state.section = VaultSection::Items;
                workspace_state.editor_mode = EditorMode::Creating(content_type);
            },
            &[AppEvent::SectionChanged, AppEvent::EditorChanged],
        );
    }

    pub fn start_editing_item(&self, item_id: Uuid) {
        self.update(
            |workspace_state| {
                workspace_state.section = VaultSection::Items;
                workspace_state.selected_item_id = Some(item_id);
                workspace_state.editor_mode = EditorMode::Editing(item_id);
            },
            &[AppEvent::SectionChanged, AppEvent::SelectionChanged, AppEvent::EditorChanged],
        );
    }

    pub fn close_item_editor(&self) {
        self.update(|workspace_state| workspace_state.editor_mode = EditorMode::Closed, &[AppEvent::EditorChanged]);
    }

    /// Al bloquear no debe quedar nada del espacio de trabajo anterior.
    pub fn reset(&self) {
        self.update(|workspace_state| *workspace_state = WorkspaceState::default(), &[]);
    }

    pub fn preselected_folder_id(&self) -> Option<Uuid> {
        match self.workspace_state.borrow().item_filter.folder_scope {
            FolderScope::SpecificFolder { folder_id } => Some(folder_id),
            FolderScope::AllFolders | FolderScope::WithoutFolder => None,
        }
    }
}

pub struct AppContext {
    window: slint::Weak<AppWindow>,
    pub shared_vault: SharedVault,
    pub vault_file_path: PathBuf,
    pub translator: RefCell<Translator>,
    ui_preferences_store: UiPreferencesStore,
    ui_preferences: RefCell<UiPreferences>,
    pub events: Rc<EventBus>,
    pub navigator: WorkspaceNavigator,
    pub toasts: ToastCenter,
    pub dialogs: DialogService,
}

impl AppContext {
    pub fn new(
        app_window: &AppWindow,
        shared_vault: SharedVault,
        vault_file_path: PathBuf,
        ui_preferences_store: UiPreferencesStore,
    ) -> Rc<Self> {
        let ui_preferences = ui_preferences_store.load();
        let translator = Translator::new(ui_preferences.language_code.as_deref());
        let events = Rc::new(EventBus::default());
        Rc::new(Self {
            window: app_window.as_weak(),
            shared_vault,
            vault_file_path,
            translator: RefCell::new(translator),
            ui_preferences_store,
            ui_preferences: RefCell::new(ui_preferences),
            navigator: WorkspaceNavigator::new(Rc::clone(&events)),
            events,
            toasts: ToastCenter::new(app_window.as_weak()),
            dialogs: DialogService::new(app_window.as_weak()),
        })
    }

    /// `None` solo mientras se cierra la ventana.
    pub fn window(&self) -> Option<AppWindow> {
        self.window.upgrade()
    }

    pub fn text(&self, translation_key: &str) -> String {
        self.translator.borrow().text(translation_key)
    }

    pub fn text_with(&self, translation_key: &str, interpolated_values: &[(&str, &str)]) -> String {
        self.translator.borrow().text_with(translation_key, interpolated_values)
    }

    pub fn count_text(&self, translation_key: &str, count: i64, interpolated_values: &[(&str, &str)]) -> String {
        self.translator.borrow().count_text(translation_key, count, interpolated_values)
    }

    pub fn ui_preferences(&self) -> UiPreferences {
        self.ui_preferences.borrow().clone()
    }

    pub fn update_ui_preferences(&self, change: impl FnOnce(&mut UiPreferences)) {
        change(&mut self.ui_preferences.borrow_mut());
        self.ui_preferences_store.save(&self.ui_preferences.borrow());
    }
}
