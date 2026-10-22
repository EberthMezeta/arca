//! Panel de detalle: secretos enmascarados que se revelan o copian bajo demanda, código TOTP,
//! favoritos, papelera y adjuntos.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use slint::{ComponentHandle, ModelRc, SharedString, Timer, TimerMode, VecModel};
use uuid::Uuid;
use vault_core::domain::item_content::ItemContent;
use vault_core::domain::secret_text::SecretText;
use vault_core::domain::vault_item::{RevealableField, VaultItem};
use vault_core::infrastructure::storage::owner_only_files::write_owner_only_file;

use crate::composition_root::MAXIMUM_ATTACHMENT_SIZE_BYTES;
use crate::platform::bounded_file_reader::{BoundedReadError, read_file_with_size_limit};
use crate::platform::file_dialogs::{ask_file_to_open, ask_file_to_save};
use crate::presentation::app_context::{AppContext, AppEvent, EditorMode};
use crate::presentation::background_task::{run_in_background, run_with_vault_in_background, spawn_ui_task};
use crate::presentation::controllers::clipboard_feedback::{copy_with_feedback, report_error};
use crate::presentation::controllers::dialog_controller::{ConfirmRequest, PromptRequest};
use crate::presentation::controllers::toast_controller::ToastRequest;
use crate::presentation::date_formatting::{format_date_time, format_file_size};
use crate::presentation::error_messages::describe_error;
use crate::presentation::item_drafts::save_request_from_item;
use crate::presentation::item_kinds::definition_for;
use crate::ui_error::{UiError, UiResult};
use vault_core::domain::vault_error::VaultError;
use crate::{AppWindow, AttachmentRow, DetailRow, DetailRowKind, ItemDetails, ToastTone};

/// Un secreto revelado vuelve a ocultarse solo, para que no quede a la vista indefinidamente.
const REVEALED_SECRET_VISIBLE_DURATION: Duration = Duration::from_secs(30);
const TOTP_REFRESH_INTERVAL: Duration = Duration::from_secs(1);
const UNDOABLE_TOAST_DURATION: Duration = Duration::from_secs(6);

/// De dónde sale el valor de una fila al copiarla y si puede revelarse.
#[derive(Clone)]
enum RowValueSource {
    PlainText(String),
    Field(RevealableField),
    Totp,
}

type UndoAction = Box<dyn FnOnce(&AppContext) -> UiResult<()>>;

#[derive(Clone)]
struct DetailRowSpec {
    kind: DetailRowKind,
    label: String,
    displayed_value: String,
    value_source: RowValueSource,
}

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
            details_state.custom_rows = Self::build_custom_rows(&vault_item);
            details_state.displayed_item = Some(vault_item);
        }
        self.publish_item();
        self.restart_totp_refresh();
    }

    fn build_main_rows(&self, vault_item: &VaultItem) -> Vec<DetailRowSpec> {
        let plain_row = |label_key: &str, displayed_value: String, value_source: RowValueSource| DetailRowSpec {
            kind: DetailRowKind::Plain,
            label: self.context.text(label_key),
            displayed_value,
            value_source,
        };
        let secret_row = |label_key: &str, masked_value: String, revealable_field: RevealableField| DetailRowSpec {
            kind: DetailRowKind::Secret,
            label: self.context.text(label_key),
            displayed_value: masked_value,
            value_source: RowValueSource::Field(revealable_field),
        };

        let ItemContent::Login(login_content) = &vault_item.item_content else {
            return definition_for(vault_item.item_content.content_type())
                .descriptor_fields
                .iter()
                .map(|field_descriptor| (field_descriptor, (field_descriptor.read_value)(&vault_item.item_content)))
                .filter(|(_, field_value)| !field_value.trim().is_empty())
                .map(|(field_descriptor, field_value)| match field_descriptor.revealable_field {
                    Some(revealable_field) => secret_row(field_descriptor.label_key, field_value, revealable_field),
                    None => plain_row(field_descriptor.label_key, field_value.clone(), RowValueSource::PlainText(field_value)),
                })
                .collect();
        };

        let mut login_rows = Vec::new();
        if !login_content.username.is_empty() {
            login_rows.push(plain_row("fields.username", login_content.username.clone(), RowValueSource::Field(RevealableField::Username)));
        }
        if !login_content.password.is_empty() {
            login_rows.push(secret_row("fields.password", login_content.password.expose_secret().to_string(), RevealableField::Password));
        }
        if vault_item.item_content.totp_secret().is_some() {
            login_rows.push(DetailRowSpec {
                kind: DetailRowKind::Totp,
                label: self.context.text("fields.totpCode"),
                displayed_value: String::new(),
                value_source: RowValueSource::Totp,
            });
        }
        login_rows.extend(
            login_content
                .website_urls
                .iter()
                .map(|website_url| plain_row("fields.website", website_url.clone(), RowValueSource::PlainText(website_url.clone()))),
        );
        login_rows
    }

    fn build_custom_rows(vault_item: &VaultItem) -> Vec<DetailRowSpec> {
        vault_item
            .custom_fields
            .iter()
            .enumerate()
            .filter(|(_, custom_field)| !custom_field.field_label.is_empty() || !custom_field.field_value.is_empty())
            .map(|(field_index, custom_field)| DetailRowSpec {
                kind: if custom_field.is_hidden { DetailRowKind::Secret } else { DetailRowKind::Plain },
                label: custom_field.field_label.clone(),
                displayed_value: custom_field.field_value.expose_secret().to_string(),
                value_source: RowValueSource::Field(RevealableField::CustomField { field_index }),
            })
            .collect()
    }

    fn publish_item(&self) {
        let Some(app_window) = self.context.window() else {
            return;
        };
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
    }

    fn clear(&self) {
        self.totp_timer.stop();
        *self.details_state.borrow_mut() = DetailsState::default();
        let Some(app_window) = self.context.window() else {
            return;
        };
        let item_details = app_window.global::<ItemDetails>();
        item_details.set_main_rows(ModelRc::default());
        item_details.set_custom_rows(ModelRc::default());
        item_details.set_notes(SharedString::new());
        item_details.set_totp_code(SharedString::new());
    }

    // ── TOTP ──────────────────────────────────────────────────────────────────

    fn restart_totp_refresh(self: &Rc<Self>) {
        let has_totp = self.details_state.borrow().main_rows.iter().any(|row_spec| matches!(row_spec.value_source, RowValueSource::Totp));
        if !has_totp {
            self.totp_timer.stop();
            return;
        }
        self.refresh_totp_code();
        let controller = Rc::downgrade(self);
        self.totp_timer.start(TimerMode::Repeated, TOTP_REFRESH_INTERVAL, move || {
            if let Some(controller) = controller.upgrade() {
                controller.refresh_totp_code();
            }
        });
    }

    /// Si Argon2id tiene la bóveda ocupada se salta un segundo en vez de congelar la ventana.
    fn refresh_totp_code(&self) {
        let Some(item_id) = self.displayed_item_id() else {
            return;
        };
        let Some(totp_code) = self.context.shared_vault.try_lock_service().and_then(|vault_service| vault_service.current_totp_code(item_id).ok())
        else {
            return;
        };
        let Some(app_window) = self.context.window() else {
            return;
        };
        let item_details = app_window.global::<ItemDetails>();
        item_details.set_totp_code(group_totp_digits(&totp_code.current_code).into());
        item_details.set_totp_seconds_left(i32::try_from(totp_code.seconds_until_refresh).unwrap_or(0));
        item_details.set_totp_period_seconds(i32::try_from(totp_code.period_seconds).unwrap_or(30));
    }

    // ── Revelar y copiar ──────────────────────────────────────────────────────

    fn row_spec(&self, row_index: i32) -> Option<DetailRowSpec> {
        let row_index = usize::try_from(row_index).ok()?;
        let details_state = self.details_state.borrow();
        details_state.main_rows.iter().chain(details_state.custom_rows.iter()).nth(row_index).cloned()
    }

    fn toggle_reveal(self: &Rc<Self>, row_index: i32) {
        let (Some(item_id), Some(row_spec), Ok(row_position)) = (self.displayed_item_id(), self.row_spec(row_index), usize::try_from(row_index)) else {
            return;
        };
        let RowValueSource::Field(revealable_field) = row_spec.value_source else {
            return;
        };
        if self.details_state.borrow_mut().revealed_values.remove(&row_position).is_some() {
            self.publish_item();
            return;
        }
        let revealed_value =
            self.context.shared_vault.lock_service().and_then(|vault_service| Ok(vault_service.reveal_item_field(item_id, revealable_field)?));
        let Ok(revealed_value) = revealed_value else {
            return;
        };
        self.details_state.borrow_mut().revealed_values.insert(row_position, revealed_value);
        self.publish_item();

        let reveal_generation = self.reveal_generation.get() + 1;
        self.reveal_generation.set(reveal_generation);
        let controller = Rc::downgrade(self);
        Timer::single_shot(REVEALED_SECRET_VISIBLE_DURATION, move || {
            let Some(controller) = controller.upgrade() else {
                return;
            };
            if controller.reveal_generation.get() != reveal_generation {
                return;
            }
            controller.details_state.borrow_mut().revealed_values.clear();
            controller.publish_item();
        });
    }

    fn copy_row(&self, row_index: i32) {
        let (Some(item_id), Some(row_spec)) = (self.displayed_item_id(), self.row_spec(row_index)) else {
            return;
        };
        copy_with_feedback(&self.context, &row_spec.label, |vault_service| match row_spec.value_source {
            RowValueSource::PlainText(plain_text) => vault_service.copy_secret_to_clipboard(SecretText::new(plain_text)),
            RowValueSource::Field(revealable_field) => vault_service.copy_item_field_to_clipboard(item_id, revealable_field),
            RowValueSource::Totp => vault_service.copy_totp_code_to_clipboard(item_id),
        });
    }

    // ── Acciones sobre el elemento ────────────────────────────────────────────

    /// Ejecuta una acción, avisa de los cambios y muestra el resultado (con "Deshacer" opcional).
    fn run_item_action(
        &self,
        item_action: impl FnOnce(&AppContext) -> UiResult<()>,
        success_message: Option<String>,
        undo_action: Option<UndoAction>,
    ) {
        if let Err(ui_error) = item_action(&self.context) {
            return report_error(&self.context, &ui_error);
        }
        self.context.events.publish(AppEvent::VaultDataChanged);
        let Some(success_message) = success_message else {
            return;
        };
        let Some(undo_action) = undo_action else {
            return self.context.toasts.success(success_message);
        };
        let undoing_context = Rc::clone(&self.context);
        self.context.toasts.show(
            ToastRequest::new(success_message, ToastTone::Success)
                .lasting(UNDOABLE_TOAST_DURATION)
                .with_action(self.context.text("actions.undo"), move || match undo_action(&undoing_context) {
                    Ok(()) => undoing_context.events.publish(AppEvent::VaultDataChanged),
                    Err(ui_error) => report_error(&undoing_context, &ui_error),
                }),
        );
    }

    fn toggle_favorite(&self) {
        let Some(item_id) = self.displayed_item_id() else {
            return;
        };
        self.run_item_action(
            move |context| {
                let mut vault_service = context.shared_vault.lock_service()?;
                let mut save_request = save_request_from_item(&vault_service.get_item_details(item_id)?);
                save_request.is_favorite = !save_request.is_favorite;
                vault_service.save_item(save_request)?;
                Ok(())
            },
            None,
            None,
        );
    }

    fn move_to_trash(&self) {
        let Some((item_id, item_title)) =
            self.details_state.borrow().displayed_item.as_ref().map(|vault_item| (vault_item.item_id, vault_item.title.clone()))
        else {
            return;
        };
        self.run_item_action(
            move |context| {
                context.shared_vault.lock_service()?.move_item_to_trash(item_id)?;
                context.navigator.select_item(None);
                Ok(())
            },
            Some(self.context.text_with("details.movedToTrash", &[("itemTitle", &item_title)])),
            Some(Box::new(move |context| Ok(context.shared_vault.lock_service()?.restore_item_from_trash(item_id)?))),
        );
    }

    fn restore(&self) {
        let Some(item_id) = self.displayed_item_id() else {
            return;
        };
        self.run_item_action(
            move |context| Ok(context.shared_vault.lock_service()?.restore_item_from_trash(item_id)?),
            Some(self.context.text("details.restored")),
            None,
        );
    }

    fn confirm_permanent_deletion(self: &Rc<Self>) {
        let Some((item_id, item_title)) =
            self.details_state.borrow().displayed_item.as_ref().map(|vault_item| (vault_item.item_id, vault_item.title.clone()))
        else {
            return;
        };
        let controller = Rc::clone(self);
        self.context.dialogs.confirm(ConfirmRequest {
            title: self.context.text("details.deleteForeverTitle"),
            text: self.context.text_with("details.deleteForeverText", &[("itemTitle", &item_title)]),
            confirm_label: self.context.text("actions.deleteForever"),
            is_destructive: true,
            on_confirm: Box::new(move || {
                controller.run_item_action(
                    move |context| {
                        context.shared_vault.lock_service()?.delete_item_permanently(item_id)?;
                        context.navigator.select_item(None);
                        Ok(())
                    },
                    Some(controller.context.text("details.deletedForever")),
                    None,
                );
            }),
        });
    }

    // ── Adjuntos ──────────────────────────────────────────────────────────────

    fn set_adding_attachment(&self, is_adding_attachment: bool) {
        self.is_adding_attachment.set(is_adding_attachment);
        if let Some(app_window) = self.context.window() {
            app_window.global::<ItemDetails>().set_is_adding_attachment(is_adding_attachment);
        }
    }

    fn add_attachment(self: &Rc<Self>) {
        let Some(item_id) = self.displayed_item_id() else {
            return;
        };
        if self.is_adding_attachment.get() {
            return;
        }
        self.set_adding_attachment(true);
        let controller = Rc::clone(self);
        spawn_ui_task(async move {
            let dialog_title = controller.context.text("attachments.chooseFile");
            let Some(chosen_path) = ask_file_to_open(&dialog_title, None).await else {
                return controller.set_adding_attachment(false);
            };
            let file_name = chosen_path.file_name().map(|file_name| file_name.to_string_lossy().into_owned()).unwrap_or_default();
            let attaching_file_name = file_name.clone();
            let attach_result = run_with_vault_in_background(&controller.context.shared_vault, move |vault_service| {
                let file_bytes = read_file_with_size_limit(&chosen_path, MAXIMUM_ATTACHMENT_SIZE_BYTES).map_err(|read_error| match read_error {
                    BoundedReadError::TooLarge => UiError::from(VaultError::AttachmentTooLarge { maximum_size_bytes: MAXIMUM_ATTACHMENT_SIZE_BYTES }),
                    BoundedReadError::Unreadable(io_error) => UiError::file_access(io_error),
                })?;
                vault_service.add_attachment(item_id, &attaching_file_name, &file_bytes)?;
                Ok(())
            })
            .await;
            controller.set_adding_attachment(false);
            match attach_result {
                Ok(()) => {
                    controller.context.events.publish(AppEvent::VaultDataChanged);
                    controller.context.toasts.success(controller.context.text_with("attachments.added", &[("fileName", &file_name)]));
                }
                Err(ui_error) => report_error(&controller.context, &ui_error),
            }
        });
    }

    fn attachment_at(&self, attachment_position: i32) -> Option<(Uuid, Uuid, String)> {
        let attachment_position = usize::try_from(attachment_position).ok()?;
        let details_state = self.details_state.borrow();
        let vault_item = details_state.displayed_item.as_ref()?;
        let attachment = vault_item.attachments.get(attachment_position)?;
        Some((vault_item.item_id, attachment.attachment_id, attachment.file_name.clone()))
    }

    /// Guardar un adjunto descifrado lo saca del cifrado: antes se pide la contraseña maestra.
    fn ask_to_save_attachment(self: &Rc<Self>, attachment_position: i32) {
        let Some((item_id, attachment_id, _)) = self.attachment_at(attachment_position) else {
            return;
        };
        let controller = Rc::clone(self);
        self.context.dialogs.ask_for_text(PromptRequest {
            title: self.context.text("attachments.saveCopy"),
            label: self.context.text("attachments.masterPassword"),
            hint: self.context.text("attachments.masterPasswordHint"),
            confirm_label: self.context.text("attachments.saveConfirm"),
            initial_value: String::new(),
            is_secret: true,
            on_submit: Rc::new(move |master_password: SecretText| controller.save_attachment(item_id, attachment_id, master_password)),
        });
    }

    fn save_attachment(self: &Rc<Self>, item_id: Uuid, attachment_id: Uuid, master_password: SecretText) {
        self.context.dialogs.set_prompt_busy(true);
        let controller = Rc::clone(self);
        spawn_ui_task(async move {
            let decryption_result = run_with_vault_in_background(&controller.context.shared_vault, move |vault_service| {
                Ok(vault_service.read_attachment(&master_password, item_id, attachment_id)?)
            })
            .await;
            let decrypted_attachment = match decryption_result {
                Ok(decrypted_attachment) => decrypted_attachment,
                Err(ui_error) => {
                    let error_message = describe_error(&controller.context.translator.borrow(), &ui_error);
                    return controller.context.dialogs.show_prompt_error(error_message);
                }
            };
            controller.context.dialogs.close_prompt();

            let dialog_title = controller.context.text("attachments.saveCopy");
            let Some(destination_path) = ask_file_to_save(&dialog_title, &decrypted_attachment.file_name).await else {
                return;
            };
            let write_result = run_in_background(move || {
                write_owner_only_file(&destination_path, &decrypted_attachment.file_bytes).map_err(UiError::file_write)
            })
            .await;
            match write_result {
                Ok(()) => controller.context.toasts.success(controller.context.text("attachments.saved")),
                Err(ui_error) => report_error(&controller.context, &ui_error),
            }
        });
    }

    fn delete_attachment(&self, attachment_position: i32) {
        let Some((item_id, attachment_id, _)) = self.attachment_at(attachment_position) else {
            return;
        };
        self.run_item_action(move |context| Ok(context.shared_vault.lock_service()?.delete_attachment(item_id, attachment_id)?), None, None);
    }
}

/// "123456" → "123 456": más fácil de leer y teclear.
fn group_totp_digits(totp_code: &str) -> String {
    let split_position = totp_code.len() / 2;
    if totp_code.len() < 6 || !totp_code.is_char_boundary(split_position) {
        return totp_code.to_string();
    }
    format!("{} {}", &totp_code[..split_position], &totp_code[split_position..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn totp_codes_are_split_in_two_groups() {
        assert_eq!(group_totp_digits("123456"), "123 456");
        assert_eq!(group_totp_digits("12345678"), "1234 5678");
        assert_eq!(group_totp_digits("1234"), "1234");
    }
}
