//! Editor de elementos. El formulario vive en el global `ItemEditor` (enlaces bidireccionales);
//! este controlador lo rellena al abrir y lo lee al guardar. Los secretos llegan enmascarados.

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use uuid::Uuid;
use vault_core::application::save_vault_item::{CustomFieldDraft, SaveVaultItemRequest, SubmittedSecret};
use vault_core::domain::folder::Folder;
use vault_core::domain::item_content::{ItemContent, ItemContentType};
use vault_core::domain::secret_text::SecretText;
use vault_core::domain::vault_item::{REDACTED_SECRET_PLACEHOLDER, RevealableField};
use vault_core::application::vault_service::VaultItems;

use crate::presentation::app_context::{AppContext, AppEvent, EditorMode};
use crate::presentation::error_messages::describe_error;
use crate::presentation::item_drafts::save_request_from_item;
use crate::presentation::item_kinds::{ContentFieldDescriptor, definition_for};
use crate::ui_error::UiResult;
use crate::{AppWindow, CustomFieldRow, DescriptorField, ItemEditor, SealedField, SealedSecret, WebsiteRow};

/// El campo sigue mostrando el marcador con el que llegó enmascarado: el usuario no lo tocó.
fn is_untouched_mask(field_text: &str) -> bool {
    field_text == REDACTED_SECRET_PLACEHOLDER
}

/// Lo que el formulario no puede guardar por sí mismo.
struct OpenDraft {
    item_id: Option<Uuid>,
    item_content: ItemContent,
    folder_ids: Vec<Option<Uuid>>,
    /// Posición de cada campo personalizado en el elemento guardado (`None` si es nuevo).
    custom_field_sources: Vec<Option<usize>>,
    website_rows: Rc<VecModel<WebsiteRow>>,
    descriptor_fields: Rc<VecModel<DescriptorField>>,
    custom_fields: Rc<VecModel<CustomFieldRow>>,
}

pub struct ItemEditorController {
    context: Rc<AppContext>,
    open_draft: RefCell<Option<OpenDraft>>,
}

fn parse_tag_names(raw_tag_text: &str) -> Vec<String> {
    raw_tag_text
        .split(',')
        .map(|tag_name| tag_name.trim().trim_start_matches('#').to_string())
        .filter(|tag_name| !tag_name.is_empty())
        .collect()
}

/// Rejilla de 2 columnas: los campos anchos ocupan una fila; los estrechos se emparejan.
fn layout_descriptor_fields(field_descriptors: &[ContentFieldDescriptor], item_content: &ItemContent, context: &AppContext) -> Vec<DescriptorField> {
    let mut grid_row = 0;
    let mut column = 0;
    field_descriptors
        .iter()
        .map(|field_descriptor| {
            if field_descriptor.is_wide && column == 1 {
                grid_row += 1;
                column = 0;
            }
            let descriptor_field = DescriptorField {
                label: context.text(field_descriptor.label_key).into(),
                value: (field_descriptor.read_value)(item_content).into(),
                is_secret: field_descriptor.revealable_field.is_some(),
                is_wide: field_descriptor.is_wide,
                grid_row,
                column,
            };
            let row_is_complete = field_descriptor.is_wide || column == 1;
            column = i32::from(!row_is_complete);
            grid_row += i32::from(row_is_complete);
            descriptor_field
        })
        .collect()
}

impl ItemEditorController {
    pub fn install(context: &Rc<AppContext>, app_window: &AppWindow) -> Rc<Self> {
        let controller = Rc::new(Self { context: Rc::clone(context), open_draft: RefCell::new(None) });
        controller.bind_callbacks(app_window);

        let event_controller = Rc::clone(&controller);
        context.events.subscribe(move |app_event| match app_event {
            AppEvent::EditorChanged => event_controller.sync_with_editor_mode(),
            AppEvent::SessionLocked => event_controller.close(),
            _ => {}
        });
        controller
    }

    fn bind_callbacks(self: &Rc<Self>, app_window: &AppWindow) {
        app_window.global::<SealedSecret>().set_placeholder(REDACTED_SECRET_PLACEHOLDER.into());
        let item_editor = app_window.global::<ItemEditor>();
        let saving_controller = Rc::clone(self);
        item_editor.on_save(move || saving_controller.save());
        let cancelling_controller = Rc::clone(self);
        item_editor.on_cancel(move || cancelling_controller.context.navigator.close_item_editor());
        let revealing_controller = Rc::clone(self);
        item_editor.on_reveal_sealed(move |sealed_field, field_position| {
            if let Ok(field_position) = usize::try_from(field_position) {
                revealing_controller.reveal_sealed(sealed_field, field_position);
            }
        });
        let website_controller = Rc::clone(self);
        item_editor.on_add_website(move || {
            website_controller.with_draft(|open_draft| open_draft.website_rows.push(WebsiteRow { url: SharedString::new() }));
        });
        let removing_website_controller = Rc::clone(self);
        item_editor.on_remove_website(move |row_position| {
            removing_website_controller.with_draft(|open_draft| {
                if let Ok(row_position) = usize::try_from(row_position)
                    && row_position < open_draft.website_rows.row_count()
                {
                    open_draft.website_rows.remove(row_position);
                }
            });
        });
        let field_controller = Rc::clone(self);
        item_editor.on_add_custom_field(move || {
            field_controller.with_draft(|open_draft| {
                open_draft.custom_fields.push(CustomFieldRow { label: SharedString::new(), value: SharedString::new(), is_hidden: false });
                open_draft.custom_field_sources.push(None);
            });
        });
        let removing_field_controller = Rc::clone(self);
        item_editor.on_remove_custom_field(move |row_position| {
            removing_field_controller.with_draft(|open_draft| {
                if let Ok(row_position) = usize::try_from(row_position)
                    && row_position < open_draft.custom_fields.row_count()
                {
                    open_draft.custom_fields.remove(row_position);
                    open_draft.custom_field_sources.remove(row_position);
                }
            });
        });
    }

    fn with_draft(&self, apply: impl FnOnce(&mut OpenDraft)) {
        if let Some(open_draft) = self.open_draft.borrow_mut().as_mut() {
            apply(open_draft);
        }
    }

    fn sync_with_editor_mode(&self) {
        match self.context.navigator.snapshot().editor_mode {
            EditorMode::Closed => self.close(),
            EditorMode::Creating(content_type) => self.open_for_creation(content_type),
            EditorMode::Editing(item_id) => self.open_for_editing(item_id),
        }
    }

    fn open_for_creation(&self, content_type: ItemContentType) {
        let empty_request = SaveVaultItemRequest {
            item_id: None,
            title: String::new(),
            folder_id: self.context.navigator.preselected_folder_id(),
            tag_names: Vec::new(),
            is_favorite: false,
            notes: SecretText::default(),
            custom_fields: Vec::new(),
            item_content: (definition_for(content_type).create_empty_content)(),
            unchanged_secrets: Vec::new(),
        };
        self.open_draft_request(empty_request, "editor.newItemTitle");
    }

    fn open_for_editing(&self, item_id: Uuid) {
        let loaded_item = self.context.shared_vault.lock_service().and_then(|vault_service| Ok(vault_service.get_item_details(item_id)?));
        match loaded_item {
            Ok(vault_item) => self.open_draft_request(save_request_from_item(&vault_item), "editor.editItemTitle"),
            Err(_) => self.context.navigator.close_item_editor(),
        }
    }

    fn open_draft_request(&self, draft_request: SaveVaultItemRequest, header_key: &str) {
        self.context.with_window(|app_window| {
            let folders: Vec<Folder> =
                self.context.shared_vault.lock_service().and_then(|vault_service| Ok(vault_service.list_folders()?)).unwrap_or_default();
            let kind_definition = definition_for(draft_request.item_content.content_type());
            let item_editor = app_window.global::<ItemEditor>();

            let item_type_label = self.context.text(kind_definition.singular_label_key);
            item_editor.set_header_title(self.context.text_with(header_key, &[("itemType", &item_type_label)]).into());
            item_editor.set_kind(kind_definition.ui_kind);
            item_editor.set_is_saving(false);
            item_editor.set_save_error(SharedString::new());
            item_editor.set_can_reveal_sealed(draft_request.item_id.is_some());
            item_editor.set_is_generator_open(false);
            item_editor.set_title(draft_request.title.clone().into());
            item_editor.set_is_favorite(draft_request.is_favorite);
            item_editor.set_notes(draft_request.notes.expose_secret().into());
            item_editor.set_tags_text(draft_request.tag_names.join(", ").into());

            let (username, password, totp_secret, website_urls) = match &draft_request.item_content {
                ItemContent::Login(login_content) => (
                    login_content.username.clone(),
                    login_content.password.expose_secret().to_string(),
                    login_content.totp_secret.as_ref().map(|totp_secret| totp_secret.expose_secret().to_string()).unwrap_or_default(),
                    login_content.website_urls.clone(),
                ),
                _ => Default::default(),
            };
            item_editor.set_username(username.into());
            item_editor.set_password(password.into());
            item_editor.set_totp_secret(totp_secret.into());
            let website_rows = Rc::new(VecModel::from(
                website_urls.into_iter().map(|website_url| WebsiteRow { url: website_url.into() }).collect::<Vec<_>>(),
            ));
            if website_rows.row_count() == 0 {
                website_rows.push(WebsiteRow { url: SharedString::new() });
            }
            item_editor.set_website_rows(ModelRc::from(Rc::clone(&website_rows)));

            let descriptor_fields = Rc::new(VecModel::from(layout_descriptor_fields(
                kind_definition.descriptor_fields,
                &draft_request.item_content,
                &self.context,
            )));
            item_editor.set_descriptor_fields(ModelRc::from(Rc::clone(&descriptor_fields)));

            let custom_fields = Rc::new(VecModel::from(
                draft_request
                    .custom_fields
                    .iter()
                    .map(|field_draft| CustomFieldRow {
                        label: field_draft.field_label.clone().into(),
                        value: match &field_draft.field_value {
                            SubmittedSecret::Unchanged => REDACTED_SECRET_PLACEHOLDER.into(),
                            SubmittedSecret::Replaced(field_value) => field_value.expose_secret().into(),
                        },
                        is_hidden: field_draft.is_hidden,
                    })
                    .collect::<Vec<_>>(),
            ));
            item_editor.set_custom_fields(ModelRc::from(Rc::clone(&custom_fields)));

            let mut folder_names = vec![SharedString::from(self.context.text("navigation.noFolder"))];
            folder_names.extend(folders.iter().map(|folder| SharedString::from(folder.folder_name.as_str())));
            let mut folder_ids = vec![None];
            folder_ids.extend(folders.iter().map(|folder| Some(folder.folder_id)));
            let folder_position = folder_ids.iter().position(|folder_id| *folder_id == draft_request.folder_id).unwrap_or(0);
            item_editor.set_folder_names(ModelRc::new(VecModel::from(folder_names)));
            item_editor.set_folder_index(i32::try_from(folder_position).unwrap_or(0));
            item_editor.set_focus_title_requests(item_editor.get_focus_title_requests() + 1);

            *self.open_draft.borrow_mut() = Some(OpenDraft {
                item_id: draft_request.item_id,
                custom_field_sources: draft_request.custom_fields.iter().map(|field_draft| field_draft.source_field_index).collect(),
                item_content: draft_request.item_content,
                folder_ids,
                website_rows,
                descriptor_fields,
                custom_fields,
            });
        });
    }

    /// Los valores reales no deben quedarse en el formulario al cerrar el editor.
    fn close(&self) {
        *self.open_draft.borrow_mut() = None;
        self.context.with_window(|app_window| {
            let item_editor = app_window.global::<ItemEditor>();
            item_editor.set_password(SharedString::new());
            item_editor.set_totp_secret(SharedString::new());
            item_editor.set_notes(SharedString::new());
            item_editor.set_descriptor_fields(ModelRc::default());
            item_editor.set_custom_fields(ModelRc::default());
            item_editor.set_is_generator_open(false);
        });
    }

    /// Sustituye el marcador enmascarado por el valor real (el usuario pulsó el ojo).
    fn reveal_sealed(&self, sealed_field: SealedField, field_position: usize) {
        self.context.with_window(|app_window| {
            let item_editor = app_window.global::<ItemEditor>();
            let open_draft = self.open_draft.borrow();
            let Some(open_draft) = open_draft.as_ref() else {
                return;
            };
            let Some(item_id) = open_draft.item_id else {
                return;
            };
            let kind_definition = definition_for(open_draft.item_content.content_type());
            let requested_field = match sealed_field {
                SealedField::Password => Some(RevealableField::Password),
                SealedField::TotpSecret => Some(RevealableField::TotpSecret),
                SealedField::Descriptor => kind_definition.descriptor_fields.get(field_position).and_then(|descriptor| descriptor.revealable_field),
                SealedField::Custom => open_draft
                    .custom_field_sources
                    .get(field_position)
                    .copied()
                    .flatten()
                    .map(|field_index| RevealableField::CustomField { field_index }),
            };
            let Some(requested_field) = requested_field else {
                return;
            };
            let Ok(revealed_value) =
                self.context.shared_vault.lock_service().and_then(|vault_service| Ok(vault_service.reveal_item_field(item_id, requested_field)?))
            else {
                return;
            };
            let revealed_text = SharedString::from(revealed_value.expose_secret());
            match sealed_field {
                SealedField::Password => item_editor.set_password(revealed_text),
                SealedField::TotpSecret => item_editor.set_totp_secret(revealed_text),
                SealedField::Descriptor => {
                    if let Some(mut descriptor_field) = open_draft.descriptor_fields.row_data(field_position) {
                        descriptor_field.value = revealed_text;
                        open_draft.descriptor_fields.set_row_data(field_position, descriptor_field);
                    }
                }
                SealedField::Custom => {
                    if let Some(mut custom_field) = open_draft.custom_fields.row_data(field_position) {
                        custom_field.value = revealed_text;
                        open_draft.custom_fields.set_row_data(field_position, custom_field);
                    }
                }
            }
        });
    }

    fn build_save_request(&self, item_editor: &ItemEditor<'_>) -> Option<SaveVaultItemRequest> {
        let open_draft = self.open_draft.borrow();
        let open_draft = open_draft.as_ref()?;
        let mut item_content = open_draft.item_content.clone();
        if let ItemContent::Login(login_content) = &mut item_content {
            login_content.username = item_editor.get_username().to_string();
            login_content.password = SecretText::new(item_editor.get_password().as_str());
            let totp_secret = item_editor.get_totp_secret();
            login_content.totp_secret = (!totp_secret.trim().is_empty()).then(|| SecretText::new(totp_secret.trim()));
            login_content.website_urls = open_draft.website_rows.iter().map(|website_row| website_row.url.trim().to_string()).collect();
        }
        definition_for(item_content.content_type())
            .descriptor_fields
            .iter()
            .zip(open_draft.descriptor_fields.iter())
            .for_each(|(field_descriptor, descriptor_field)| {
                (field_descriptor.write_value)(&mut item_content, descriptor_field.value.to_string());
            });
        let is_existing_item = open_draft.item_id.is_some();
        let unchanged_secrets = item_content
            .masked_secrets_mut()
            .into_iter()
            .filter(|(_, submitted_secret)| is_existing_item && is_untouched_mask(submitted_secret.expose_secret()))
            .map(|(masked_field, _)| masked_field)
            .collect();
        let folder_position = usize::try_from(item_editor.get_folder_index()).unwrap_or(0);
        Some(SaveVaultItemRequest {
            item_id: open_draft.item_id,
            title: item_editor.get_title().to_string(),
            folder_id: open_draft.folder_ids.get(folder_position).copied().flatten(),
            tag_names: parse_tag_names(&item_editor.get_tags_text()),
            is_favorite: item_editor.get_is_favorite(),
            notes: SecretText::new(item_editor.get_notes().as_str()),
            custom_fields: open_draft
                .custom_fields
                .iter()
                .zip(open_draft.custom_field_sources.iter())
                .map(|(custom_field, source_field_index)| CustomFieldDraft {
                    field_label: custom_field.label.to_string(),
                    field_value: if source_field_index.is_some() && is_untouched_mask(&custom_field.value) {
                        SubmittedSecret::Unchanged
                    } else {
                        SubmittedSecret::Replaced(SecretText::new(custom_field.value.as_str()))
                    },
                    is_hidden: custom_field.is_hidden,
                    source_field_index: *source_field_index,
                })
                .collect(),
            item_content,
            unchanged_secrets,
        })
    }

    fn save(&self) {
        self.context.with_window(|app_window| {
            let item_editor = app_window.global::<ItemEditor>();
            if item_editor.get_is_saving() {
                return;
            }
            let Some(save_request) = self.build_save_request(&item_editor) else {
                return;
            };
            item_editor.set_is_saving(true);
            item_editor.set_save_error(SharedString::new());
            let save_result: UiResult<_> =
                self.context.shared_vault.lock_service().and_then(|mut vault_service| Ok(vault_service.save_item(save_request)?));
            item_editor.set_is_saving(false);
            match save_result {
                Ok(saved_item) => {
                    self.context.navigator.select_item(Some(saved_item.item_id));
                    self.context.events.publish(AppEvent::VaultDataChanged);
                    self.context.toasts.success(self.context.text_with("editor.saved", &[("itemTitle", &saved_item.title)]));
                }
                Err(ui_error) => {
                    item_editor.set_save_error(describe_error(&self.context.translator.borrow(), &ui_error).into());
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_are_split_trimmed_and_stripped_of_hashes() {
        assert_eq!(parse_tag_names(" #work, banking,, "), vec!["work".to_string(), "banking".to_string()]);
    }
}
