//! Panel de detalle: secretos enmascarados que se revelan o copian bajo demanda, código TOTP,
//! favoritos, papelera y adjuntos.

mod attachments;
mod detail_rows;
mod item_actions;
mod totp;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use slint::{ComponentHandle, ModelRc, SharedString, Timer, VecModel};
use uuid::Uuid;
use vault_core::domain::item_content::ItemContent;
use vault_core::domain::secret_text::SecretText;
use vault_core::domain::vault_item::VaultItem;

use crate::presentation::app_context::{AppContext, AppEvent, EditorMode};
use crate::presentation::date_formatting::{format_date_time, format_file_size};
use crate::presentation::item_kinds::definition_for;
use crate::{AppWindow, AttachmentRow, DetailRow, ItemDetails};
use detail_rows::{DetailRowSpec, build_custom_rows};
use vault_core::application::vault_service::VaultItems;

/// Un secreto revelado vuelve a ocultarse solo, para que no quede a la vista indefinidamente.
const REVEALED_SECRET_VISIBLE_DURATION: Duration = Duration::from_secs(30);

#[derive(Default)]
struct DetailsState {
    displayed_item: Option<VaultItem>,
    main_rows: Vec<DetailRowSpec>,
    custom_rows: Vec<DetailRowSpec>,
    revealed_values: HashMap<usize, SecretText>,
}

pub struct ItemDetailsController {
    context: Rc<AppContext>,
    details_state: RefCell<DetailsState>,
    totp_timer: Timer,
    /// Distingue revelados sucesivos: el temporizador de uno anterior no oculta uno más reciente.
    reveal_generation: Cell<u64>,
    is_adding_attachment: Cell<bool>,
}

impl ItemDetailsController {
    pub fn install(context: &Rc<AppContext>, app_window: &AppWindow) -> Rc<Self> {
        let controller = Rc::new(Self {
            context: Rc::clone(context),
            details_state: RefCell::default(),
            totp_timer: Timer::default(),
            reveal_generation: Cell::new(0),
            is_adding_attachment: Cell::new(false),
        });
        controller.bind_callbacks(app_window);

        let event_controller = Rc::clone(&controller);
        context.events.subscribe(move |app_event| match app_event {
            AppEvent::SelectionChanged | AppEvent::EditorChanged | AppEvent::VaultDataChanged | AppEvent::LanguageChanged => {
                event_controller.refresh();
            }
            AppEvent::SessionLocked => event_controller.clear(),
            _ => {}
        });
        controller
    }

    fn bind_callbacks(self: &Rc<Self>, app_window: &AppWindow) {
        let item_details = app_window.global::<ItemDetails>();
        let revealing_controller = Rc::clone(self);
        item_details.on_toggle_reveal(move |row_index| revealing_controller.toggle_reveal(row_index));
        let copying_controller = Rc::clone(self);
        item_details.on_copy_row(move |row_index| copying_controller.copy_row(row_index));
        let favorite_controller = Rc::clone(self);
        item_details.on_toggle_favorite(move || favorite_controller.toggle_favorite());
        let trashing_controller = Rc::clone(self);
        item_details.on_move_to_trash(move || trashing_controller.move_to_trash());
        let restoring_controller = Rc::clone(self);
        item_details.on_restore(move || restoring_controller.restore());
        let deleting_controller = Rc::clone(self);
        item_details.on_delete_forever(move || deleting_controller.confirm_permanent_deletion());
        let editing_controller = Rc::clone(self);
        item_details.on_edit(move || {
            if let Some(item_id) = editing_controller.displayed_item_id() {
                editing_controller.context.navigator.start_editing_item(item_id);
            }
        });
        let attaching_controller = Rc::clone(self);
        item_details.on_add_attachment(move || attaching_controller.add_attachment());
        let saving_controller = Rc::clone(self);
        item_details.on_save_attachment(move |attachment_position| saving_controller.ask_to_save_attachment(attachment_position));
        let removing_controller = Rc::clone(self);
        item_details.on_delete_attachment(move |attachment_position| removing_controller.delete_attachment(attachment_position));
    }

    fn displayed_item_id(&self) -> Option<Uuid> {
        self.details_state.borrow().displayed_item.as_ref().map(|vault_item| vault_item.item_id)
    }

    // ── Carga y presentación ──────────────────────────────────────────────────

    fn refresh(self: &Rc<Self>) {
        let workspace_state = self.context.navigator.snapshot();
        let (EditorMode::Closed, Some(selected_item_id)) = (workspace_state.editor_mode, workspace_state.selected_item_id) else {
            return self.clear();
        };
        let loaded_item = self.context.shared_vault.lock_service().and_then(|vault_service| Ok(vault_service.get_item_details(selected_item_id)?));
        let Ok(vault_item) = loaded_item else {
            return self.clear();
        };
        let is_same_item = self.displayed_item_id() == Some(selected_item_id);
        {
            let mut details_state = self.details_state.borrow_mut();
            if !is_same_item {
                details_state.revealed_values.clear();
            }
            details_state.main_rows = self.build_main_rows(&vault_item);
            details_state.custom_rows = build_custom_rows(&vault_item);
            details_state.displayed_item = Some(vault_item);
        }
        self.publish_item();
        self.restart_totp_refresh();
    }

    fn publish_item(&self) {
        self.context.with_window(|app_window| {
            let details_state = self.details_state.borrow();
            let Some(vault_item) = details_state.displayed_item.as_ref() else {
                return;
            };
            let item_details = app_window.global::<ItemDetails>();
            let kind_definition = definition_for(vault_item.item_content.content_type());
            let language_code = self.context.translator.borrow().active_language_code();

            let to_detail_rows = |row_specs: &[DetailRowSpec], first_row_index: usize| -> ModelRc<DetailRow> {
                let detail_rows: Vec<DetailRow> = row_specs
                    .iter()
                    .enumerate()
                    .map(|(position, row_spec)| {
                        let row_index = first_row_index + position;
                        let revealed_value = details_state.revealed_values.get(&row_index);
                        DetailRow {
                            row_index: i32::try_from(row_index).unwrap_or(i32::MAX),
                            kind: row_spec.kind,
                            label: row_spec.label.clone().into(),
                            value: revealed_value.map_or(row_spec.displayed_value.as_str(), SecretText::expose_secret).into(),
                            is_revealed: revealed_value.is_some(),
                        }
                    })
                    .collect();
                ModelRc::new(VecModel::from(detail_rows))
            };

            item_details.set_title(vault_item.title.clone().into());
            item_details.set_kind(kind_definition.ui_kind);
            item_details.set_kind_label(self.context.text(kind_definition.singular_label_key).into());
            item_details.set_is_favorite(vault_item.is_favorite);
            item_details.set_is_in_trash(vault_item.is_in_trash());
            item_details.set_main_rows(to_detail_rows(&details_state.main_rows, 0));
            item_details.set_custom_rows(to_detail_rows(&details_state.custom_rows, details_state.main_rows.len()));
            item_details.set_notes(vault_item.notes.expose_secret().into());
            item_details.set_notes_in_main_section(matches!(vault_item.item_content, ItemContent::SecureNote(_)));
            let attachment_rows: Vec<AttachmentRow> = vault_item
                .attachments
                .iter()
                .map(|attachment| AttachmentRow {
                    file_name: attachment.file_name.clone().into(),
                    size_text: format_file_size(attachment.size_bytes).into(),
                })
                .collect();
            item_details.set_attachments(ModelRc::new(VecModel::from(attachment_rows)));
            item_details.set_is_adding_attachment(self.is_adding_attachment.get());
            let tag_names: Vec<SharedString> = vault_item.tag_names.iter().map(SharedString::from).collect();
            item_details.set_tag_names(ModelRc::new(VecModel::from(tag_names)));
            item_details.set_updated_text(
                self.context.text_with("details.updatedAt", &[("dateTime", &format_date_time(language_code, vault_item.updated_at))]).into(),
            );
            item_details.set_created_text(
                self.context.text_with("details.createdAt", &[("dateTime", &format_date_time(language_code, vault_item.created_at))]).into(),
            );
            let history_count = i64::try_from(vault_item.password_history.len()).unwrap_or(i64::MAX);
            let history_text =
                if history_count > 0 { self.context.count_text("details.passwordHistoryCount", history_count, &[]) } else { String::new() };
            item_details.set_history_text(history_text.into());
        });
    }

    fn clear(&self) {
        self.totp_timer.stop();
        *self.details_state.borrow_mut() = DetailsState::default();
        self.context.with_window(|app_window| {
            let item_details = app_window.global::<ItemDetails>();
            item_details.set_main_rows(ModelRc::default());
            item_details.set_custom_rows(ModelRc::default());
            item_details.set_notes(SharedString::new());
            item_details.set_totp_code(SharedString::new());
        });
    }
}
