//! Conversión de un elemento (enmascarado) en la petición de guardado que lo deja igual.
//! Los marcadores enmascarados sin tocar recuperan su valor real al guardar (ver `save_vault_item`).

use vault_core::application::save_vault_item::{CustomFieldDraft, SaveVaultItemRequest};
use vault_core::domain::vault_item::VaultItem;

pub fn save_request_from_item(vault_item: &VaultItem) -> SaveVaultItemRequest {
    SaveVaultItemRequest {
        item_id: Some(vault_item.item_id),
        title: vault_item.title.clone(),
        folder_id: vault_item.folder_id,
        tag_names: vault_item.tag_names.clone(),
        is_favorite: vault_item.is_favorite,
        notes: vault_item.notes.clone(),
        custom_fields: vault_item
            .custom_fields
            .iter()
            .enumerate()
            .map(|(field_index, custom_field)| CustomFieldDraft {
                field_label: custom_field.field_label.clone(),
                field_value: custom_field.field_value.clone(),
                is_hidden: custom_field.is_hidden,
                source_field_index: Some(field_index),
            })
            .collect(),
        item_content: vault_item.item_content.clone(),
    }
}
