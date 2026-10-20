use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::item_content::ItemContent;
use crate::domain::secret_text::SecretText;
use crate::domain::totp_configuration::TotpConfiguration;
use crate::domain::vault_contents::VaultContents;
use crate::domain::vault_error::{VaultError, VaultResult};
use crate::domain::vault_item::{CustomField, VaultItem, restore_sealed_content_secrets, restore_sealed_secret};

const MAXIMUM_TITLE_CHARACTER_COUNT: usize = 200;

/// Campo personalizado tal como lo edita la UI. `source_field_index` es su posición en el
/// elemento guardado (`None` si es nuevo): permite recuperar un valor oculto enmascarado
/// aunque el usuario haya borrado o añadido otros campos.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomFieldDraft {
    pub field_label: String,
    pub field_value: SecretText,
    pub is_hidden: bool,
    #[serde(default)]
    pub source_field_index: Option<usize>,
}

impl CustomFieldDraft {
    fn into_custom_field(self, stored_field: Option<&CustomField>) -> CustomField {
        let mut field_value = self.field_value;
        if let Some(stored_field) = stored_field {
            restore_sealed_secret(&mut field_value, &stored_field.field_value);
        }
        CustomField { field_label: self.field_label, field_value, is_hidden: self.is_hidden }
    }
}

/// Datos editables que envía la UI. Fechas, historial y adjuntos los gestiona el backend.
/// Los secretos que la UI recibió enmascarados y no tocó llegan como el marcador
/// `REDACTED_SECRET_PLACEHOLDER` y se sustituyen por el valor guardado.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveVaultItemRequest {
    pub item_id: Option<Uuid>,
    pub title: String,
    #[serde(default)]
    pub folder_id: Option<Uuid>,
    #[serde(default)]
    pub tag_names: Vec<String>,
    #[serde(default)]
    pub is_favorite: bool,
    #[serde(default)]
    pub notes: SecretText,
    #[serde(default)]
    pub custom_fields: Vec<CustomFieldDraft>,
    pub item_content: ItemContent,
}

pub fn execute(
    vault_contents: &mut VaultContents,
    mut save_request: SaveVaultItemRequest,
    now: DateTime<Utc>,
) -> VaultResult<VaultItem> {
    let stored_item = save_request.item_id.map(|existing_item_id| vault_contents.find_item(existing_item_id)).transpose()?;
    let custom_fields = unseal_submitted_secrets(&mut save_request, stored_item);
    let save_request = validate_and_normalize(vault_contents, save_request)?;
    match save_request.item_id {
        None => Ok(create_item(vault_contents, save_request, custom_fields, now)),
        Some(existing_item_id) => update_item(vault_contents, existing_item_id, save_request, custom_fields, now),
    }
}

/// Antes de validar (p. ej. el TOTP), los marcadores enmascarados recuperan su valor real.
fn unseal_submitted_secrets(save_request: &mut SaveVaultItemRequest, stored_item: Option<&VaultItem>) -> Vec<CustomField> {
    let custom_field_drafts = std::mem::take(&mut save_request.custom_fields);
    let Some(stored_item) = stored_item else {
        return custom_field_drafts.into_iter().map(|field_draft| field_draft.into_custom_field(None)).collect();
    };

    restore_sealed_content_secrets(&mut save_request.item_content, &stored_item.item_content);
    custom_field_drafts
        .into_iter()
        .map(|field_draft| {
            let stored_field =
                field_draft.source_field_index.and_then(|source_field_index| stored_item.custom_fields.get(source_field_index));
            field_draft.into_custom_field(stored_field)
        })
        .collect()
}

fn validate_and_normalize(
    vault_contents: &VaultContents,
    mut save_request: SaveVaultItemRequest,
) -> VaultResult<SaveVaultItemRequest> {
    save_request.title = save_request.title.trim().to_string();
    if save_request.title.is_empty() {
        return Err(VaultError::invalid_input("titleRequired"));
    }
    if save_request.title.chars().count() > MAXIMUM_TITLE_CHARACTER_COUNT {
        return Err(VaultError::invalid_input("titleTooLong"));
    }
    if let Some(folder_id) = save_request.folder_id {
        vault_contents.ensure_folder_exists(folder_id)?;
    }
    if let Some(totp_secret) = save_request.item_content.totp_secret() {
        TotpConfiguration::parse(totp_secret)?;
    }

    save_request.tag_names = normalize_tag_names(&save_request.tag_names);
    if let ItemContent::Login(login_content) = &mut save_request.item_content {
        login_content.website_urls.retain(|website_url| !website_url.trim().is_empty());
    }
    Ok(save_request)
}

fn normalize_tag_names(raw_tag_names: &[String]) -> Vec<String> {
    let unique_tag_names: BTreeSet<String> = raw_tag_names
        .iter()
        .map(|tag_name| tag_name.trim().to_string())
        .filter(|tag_name| !tag_name.is_empty())
        .collect();
    unique_tag_names.into_iter().collect()
}

fn create_item(
    vault_contents: &mut VaultContents,
    save_request: SaveVaultItemRequest,
    custom_fields: Vec<CustomField>,
    now: DateTime<Utc>,
) -> VaultItem {
    let has_password = save_request.item_content.login_password().is_some();
    let new_item = VaultItem {
        item_id: Uuid::new_v4(),
        title: save_request.title,
        folder_id: save_request.folder_id,
        tag_names: save_request.tag_names,
        is_favorite: save_request.is_favorite,
        notes: save_request.notes,
        custom_fields,
        item_content: save_request.item_content,
        password_history: Vec::new(),
        attachments: Vec::new(),
        created_at: now,
        updated_at: now,
        password_changed_at: has_password.then_some(now),
        moved_to_trash_at: None,
    };
    vault_contents.vault_items.push(new_item.clone());
    new_item
}

fn update_item(
    vault_contents: &mut VaultContents,
    existing_item_id: Uuid,
    save_request: SaveVaultItemRequest,
    custom_fields: Vec<CustomField>,
    now: DateTime<Utc>,
) -> VaultResult<VaultItem> {
    let password_history_limit = vault_contents.preferences.password_history_limit;
    let existing_item = vault_contents.find_item_mut(existing_item_id)?;
    let previous_password = existing_item.item_content.login_password().cloned();

    existing_item.title = save_request.title;
    existing_item.folder_id = save_request.folder_id;
    existing_item.tag_names = save_request.tag_names;
    existing_item.is_favorite = save_request.is_favorite;
    existing_item.notes = save_request.notes;
    existing_item.custom_fields = custom_fields;
    existing_item.item_content = save_request.item_content;
    existing_item.updated_at = now;
    existing_item.record_password_change(previous_password.as_ref(), now, password_history_limit);

    Ok(existing_item.clone())
}
