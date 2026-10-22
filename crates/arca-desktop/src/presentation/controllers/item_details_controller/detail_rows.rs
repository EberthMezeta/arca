//! Filas del panel de detalle: de dónde sale cada valor, y cómo se revela o se copia.

use std::rc::Rc;

use slint::Timer;
use vault_core::domain::item_content::ItemContent;
use vault_core::domain::secret_text::SecretText;
use vault_core::domain::vault_item::{RevealableField, VaultItem};

use super::{ItemDetailsController, REVEALED_SECRET_VISIBLE_DURATION};
use crate::DetailRowKind;
use crate::presentation::controllers::clipboard_feedback::copy_with_feedback;
use crate::presentation::item_kinds::definition_for;

/// De dónde sale el valor de una fila al copiarla y si puede revelarse.
#[derive(Clone)]
pub(super) enum RowValueSource {
    PlainText(String),
    Field(RevealableField),
    Totp,
}

#[derive(Clone)]
pub(super) struct DetailRowSpec {
    pub(super) kind: DetailRowKind,
    pub(super) label: String,
    pub(super) displayed_value: String,
    pub(super) value_source: RowValueSource,
}

impl ItemDetailsController {
    pub(super) fn build_main_rows(&self, vault_item: &VaultItem) -> Vec<DetailRowSpec> {
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

    // ── Revelar y copiar ──────────────────────────────────────────────────────

    fn row_spec(&self, row_index: i32) -> Option<DetailRowSpec> {
        let row_index = usize::try_from(row_index).ok()?;
        let details_state = self.details_state.borrow();
        details_state.main_rows.iter().chain(details_state.custom_rows.iter()).nth(row_index).cloned()
    }

    pub(super) fn toggle_reveal(self: &Rc<Self>, row_index: i32) {
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

    pub(super) fn copy_row(&self, row_index: i32) {
        let (Some(item_id), Some(row_spec)) = (self.displayed_item_id(), self.row_spec(row_index)) else {
            return;
        };
        copy_with_feedback(&self.context, &row_spec.label, |vault_service| match row_spec.value_source {
            RowValueSource::PlainText(plain_text) => vault_service.copy_secret_to_clipboard(SecretText::new(plain_text)),
            RowValueSource::Field(revealable_field) => vault_service.copy_item_field_to_clipboard(item_id, revealable_field),
            RowValueSource::Totp => vault_service.copy_totp_code_to_clipboard(item_id),
        });
    }
}

pub(super) fn build_custom_rows(vault_item: &VaultItem) -> Vec<DetailRowSpec> {
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

