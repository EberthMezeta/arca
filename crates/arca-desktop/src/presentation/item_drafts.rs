//! Conversión de un elemento (enmascarado) en la petición de guardado que lo deja igual:
//! todos sus secretos van como «sin tocar» y recuperan su valor real al guardar.

use vault_core::application::save_vault_item::{CustomFieldDraft, SaveVaultItemRequest, SubmittedSecret};
use vault_core::domain::vault_item::VaultItem;

pub fn save_request_from_item(vault_item: &VaultItem) -> SaveVaultItemRequest {
    let mut item_content = vault_item.item_content.clone();
    let unchanged_secrets =
        item_content.masked_secrets_mut().into_iter().map(|(masked_field, _)| masked_field).collect();
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
                // Los ocultos llegan enmascarados; los visibles, con su valor.
                field_value: if custom_field.is_hidden {
                    SubmittedSecret::Unchanged
                } else {
                    SubmittedSecret::Replaced(custom_field.field_value.clone())
                },
                is_hidden: custom_field.is_hidden,
                source_field_index: Some(field_index),
            })
            .collect(),
        item_content,
        unchanged_secrets,
    }
}
