//! Paleta de comandos (Ctrl K): busca elementos y ejecuta acciones.

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use vault_core::application::search_vault_items::VaultItemFilter;
use vault_core::domain::item_content::ItemContentType;

use crate::presentation::app_context::{AppContext, AppEvent};
use crate::presentation::item_kinds::{ITEM_KIND_DEFINITIONS, definition_for};
use crate::{AppWindow, CommandPalette, NavIcon, PaletteEntry, VaultSection};

const MAXIMUM_ITEM_RESULTS: usize = 6;

type PaletteAction = Rc<dyn Fn(&AppContext)>;

struct PaletteCommand {
    entry: PaletteEntry,
    run_command: PaletteAction,
}

pub struct CommandPaletteController {
    context: Rc<AppContext>,
    visible_commands: RefCell<Vec<PaletteCommand>>,
}

impl CommandPaletteController {
    pub fn install(context: &Rc<AppContext>, app_window: &AppWindow) -> Rc<Self> {
        let controller = Rc::new(Self { context: Rc::clone(context), visible_commands: RefCell::default() });
        let command_palette = app_window.global::<CommandPalette>();
        let query_controller = Rc::clone(&controller);
        command_palette.on_query_edited(move |query_text| query_controller.show_commands_for(&query_text));
        let executing_controller = Rc::clone(&controller);
        command_palette.on_execute(move |entry_position| executing_controller.execute(entry_position));
        let closing_controller = Rc::clone(&controller);
        command_palette.on_close(move || closing_controller.close());

        let event_controller = Rc::clone(&controller);
        context.events.subscribe(move |app_event| match app_event {
            AppEvent::CommandPaletteRequested => event_controller.open(),
            AppEvent::SessionLocked => event_controller.close(),
            _ => {}
        });
        controller
    }

    fn open(&self) {
        let Some(app_window) = self.context.window() else {
            return;
        };
        let command_palette = app_window.global::<CommandPalette>();
        command_palette.set_query(SharedString::new());
        self.show_commands_for("");
        command_palette.set_is_open(true);
    }

    fn close(&self) {
        self.visible_commands.borrow_mut().clear();
        if let Some(app_window) = self.context.window() {
            app_window.global::<CommandPalette>().set_is_open(false);
        }
    }

    fn static_commands(&self) -> Vec<PaletteCommand> {
        let command = |label: String, icon: NavIcon, shortcut: &str, run_command: PaletteAction| PaletteCommand {
            entry: PaletteEntry { label: label.into(), hint: SharedString::new(), icon, shortcut: shortcut.into() },
            run_command,
        };
        let mut static_commands: Vec<PaletteCommand> = ITEM_KIND_DEFINITIONS
            .iter()
            .map(|definition| {
                let content_type = definition.content_type;
                let item_type_label = self.context.text(definition.singular_label_key);
                command(
                    self.context.text_with("palette.newItem", &[("itemType", &item_type_label)]),
                    definition.nav_icon,
                    if content_type == ItemContentType::Login { "Ctrl N" } else { "" },
                    Rc::new(move |context: &AppContext| context.navigator.start_creating_item(content_type)),
                )
            })
            .collect();
        static_commands.extend(
            [
                (VaultSection::Generator, NavIcon::Generator, "navigation.generator"),
                (VaultSection::Health, NavIcon::Health, "navigation.health"),
                (VaultSection::Settings, NavIcon::Settings, "navigation.settings"),
            ]
            .into_iter()
            .map(|(section, icon, label_key)| {
                command(self.context.text(label_key), icon, "", Rc::new(move |context: &AppContext| context.navigator.open_section(section)))
            }),
        );
        static_commands.push(command(
            self.context.text("actions.lock"),
            NavIcon::Lock,
            "Ctrl L",
            Rc::new(|context: &AppContext| context.events.publish(AppEvent::LockRequested)),
        ));
        static_commands
    }

    fn item_commands(&self, query_text: &str) -> Vec<PaletteCommand> {
        if query_text.trim().is_empty() {
            return Vec::new();
        }
        let item_filter = VaultItemFilter { search_text: Some(query_text.to_string()), ..VaultItemFilter::default() };
        let matching_items =
            self.context.shared_vault.lock_service().and_then(|vault_service| Ok(vault_service.search_items(&item_filter)?)).unwrap_or_default();
        matching_items
            .into_iter()
            .take(MAXIMUM_ITEM_RESULTS)
            .map(|item_summary| {
                let item_id = item_summary.item_id;
                PaletteCommand {
                    entry: PaletteEntry {
                        label: item_summary.title.into(),
                        hint: item_summary.subtitle.into(),
                        icon: definition_for(item_summary.content_type).nav_icon,
                        shortcut: SharedString::new(),
                    },
                    run_command: Rc::new(move |context: &AppContext| {
                        context.navigator.apply_navigation_filter(VaultItemFilter::default());
                        context.navigator.select_item(Some(item_id));
                    }),
                }
            })
            .collect()
    }

    fn show_commands_for(&self, query_text: &str) {
        let normalized_query = query_text.trim().to_lowercase();
        let mut visible_commands = self.item_commands(query_text);
        visible_commands.extend(
            self.static_commands().into_iter().filter(|command| command.entry.label.to_lowercase().contains(&normalized_query)),
        );
        let palette_entries: Vec<PaletteEntry> = visible_commands.iter().map(|command| command.entry.clone()).collect();
        *self.visible_commands.borrow_mut() = visible_commands;
        if let Some(app_window) = self.context.window() {
            let command_palette = app_window.global::<CommandPalette>();
            command_palette.set_entries(ModelRc::new(VecModel::from(palette_entries)));
            command_palette.set_highlighted(0);
        }
    }

    fn execute(&self, entry_position: i32) {
        let run_command = usize::try_from(entry_position)
            .ok()
            .and_then(|entry_position| self.visible_commands.borrow().get(entry_position).map(|command| Rc::clone(&command.run_command)));
        let Some(run_command) = run_command else {
            return;
        };
        self.close();
        run_command(&self.context);
    }
}
