use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::item_content::ItemContent;
use crate::domain::secret_text::SecretText;
use crate::domain::vault_error::{VaultError, VaultResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultItem {
    pub item_id: Uuid,
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
    pub custom_fields: Vec<CustomField>,
    pub item_content: ItemContent,
    #[serde(default)]
    pub password_history: Vec<PasswordHistoryEntry>,
    #[serde(default)]
    pub attachments: Vec<AttachmentReference>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub password_changed_at: Option<DateTime<Utc>>,
    /// Si tiene valor, el elemento está en la papelera (permite "deshacer").
    #[serde(default)]
    pub moved_to_trash_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomField {
    pub field_label: String,
    pub field_value: SecretText,
    pub is_hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordHistoryEntry {
    pub previous_password: SecretText,
    pub replaced_at: DateTime<Utc>,
}

/// Referencia a un adjunto cifrado. La clave de cada adjunto se deriva
/// (HKDF) a partir de la clave de adjuntos y de `attachment_id`, por eso aquí no hay material de clave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentReference {
    pub attachment_id: Uuid,
    pub file_name: String,
    pub size_bytes: u64,
    pub added_at: DateTime<Utc>,
}

/// Campos cuyo valor sensible puede revelarse o copiarse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "fieldKind", rename_all = "camelCase")]
pub enum RevealableField {
    Username,
    Password,
    Notes,
    CardNumber,
    CardSecurityCode,
    IdentityDocumentNumber,
    #[serde(rename_all = "camelCase")]
    CustomField { field_index: usize },
}

/// Sustituye a cada secreto no vacío en la vista de detalle.
pub const REDACTED_SECRET_PLACEHOLDER: &str = "••••••••";

fn redact_secret(secret_value: &mut SecretText) {
    if !secret_value.is_empty() {
        *secret_value = SecretText::new(REDACTED_SECRET_PLACEHOLDER);
    }
}

impl VaultItem {
    /// Copia para la vista de detalle: los secretos se enmascaran y solo se obtienen con `reveal_field`.
    /// Las notas se mantienen visibles (igual que en Bitwarden).
    #[must_use]
    pub fn redacted_copy(&self) -> Self {
        let mut redacted_item = self.clone();
        match &mut redacted_item.item_content {
            ItemContent::Login(login_content) => {
                redact_secret(&mut login_content.password);
                login_content.totp_secret.iter_mut().for_each(redact_secret);
            }
            ItemContent::PaymentCard(card_content) => {
                redact_secret(&mut card_content.card_number);
                redact_secret(&mut card_content.security_code);
            }
            ItemContent::Identity(identity_content) => redact_secret(&mut identity_content.identity_document_number),
            ItemContent::SecureNote(_) => {}
        }
        redacted_item
            .custom_fields
            .iter_mut()
            .filter(|custom_field| custom_field.is_hidden)
            .for_each(|custom_field| redact_secret(&mut custom_field.field_value));
        redacted_item
            .password_history
            .iter_mut()
            .for_each(|history_entry| redact_secret(&mut history_entry.previous_password));
        redacted_item
    }

    pub fn is_in_trash(&self) -> bool {
        self.moved_to_trash_at.is_some()
    }

    pub fn reveal_field(&self, requested_field: RevealableField) -> VaultResult<SecretText> {
        let revealed_value = match (requested_field, &self.item_content) {
            (RevealableField::Notes, _) => Some(self.notes.clone()),
            (RevealableField::CustomField { field_index }, _) => {
                self.custom_fields.get(field_index).map(|custom_field| custom_field.field_value.clone())
            }
            (RevealableField::Username, ItemContent::Login(login_content)) => {
                Some(SecretText::new(login_content.username.clone()))
            }
            (RevealableField::Password, ItemContent::Login(login_content)) => Some(login_content.password.clone()),
            (RevealableField::CardNumber, ItemContent::PaymentCard(card_content)) => {
                Some(card_content.card_number.clone())
            }
            (RevealableField::CardSecurityCode, ItemContent::PaymentCard(card_content)) => {
                Some(card_content.security_code.clone())
            }
            (RevealableField::IdentityDocumentNumber, ItemContent::Identity(identity_content)) => {
                Some(identity_content.identity_document_number.clone())
            }
            _ => None,
        };

        revealed_value.filter(|value| !value.is_empty()).ok_or(VaultError::FieldNotAvailable)
    }

    /// Guarda la contraseña anterior en el historial si ha cambiado, respetando el límite.
    pub fn record_password_change(
        &mut self,
        previous_password: Option<&SecretText>,
        changed_at: DateTime<Utc>,
        password_history_limit: usize,
    ) {
        let current_password = self.item_content.login_password();
        if current_password == previous_password {
            return;
        }

        self.password_changed_at = Some(changed_at);
        let Some(previous_password) = previous_password else {
            return;
        };

        self.password_history.insert(
            0,
            PasswordHistoryEntry { previous_password: previous_password.clone(), replaced_at: changed_at },
        );
        self.password_history.truncate(password_history_limit);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::item_content::LoginContent;

    fn login_item_with_password(password: &str) -> VaultItem {
        let now = Utc::now();
        VaultItem {
            item_id: Uuid::new_v4(),
            title: "Correo".into(),
            folder_id: None,
            tag_names: vec![],
            is_favorite: false,
            notes: SecretText::default(),
            custom_fields: vec![],
            item_content: ItemContent::Login(LoginContent {
                username: "ana".into(),
                password: SecretText::new(password),
                ..LoginContent::default()
            }),
            password_history: vec![],
            attachments: vec![],
            created_at: now,
            updated_at: now,
            password_changed_at: None,
            moved_to_trash_at: None,
        }
    }

    #[test]
    fn changing_password_keeps_previous_one_in_history() {
        let mut vault_item = login_item_with_password("nueva");

        vault_item.record_password_change(Some(&SecretText::new("antigua")), Utc::now(), 5);

        assert_eq!(vault_item.password_history.len(), 1);
        assert_eq!(vault_item.password_history[0].previous_password.expose_secret(), "antigua");
    }

    #[test]
    fn unchanged_password_does_not_touch_history() {
        let mut vault_item = login_item_with_password("igual");

        vault_item.record_password_change(Some(&SecretText::new("igual")), Utc::now(), 5);

        assert_eq!(vault_item.password_history, Vec::new());
    }

    #[test]
    fn history_is_truncated_to_limit() {
        let mut vault_item = login_item_with_password("v3");
        vault_item.record_password_change(Some(&SecretText::new("v1")), Utc::now(), 1);
        vault_item.item_content = ItemContent::Login(LoginContent {
            password: SecretText::new("v4"),
            ..LoginContent::default()
        });

        vault_item.record_password_change(Some(&SecretText::new("v3")), Utc::now(), 1);

        assert_eq!(vault_item.password_history.len(), 1);
        assert_eq!(vault_item.password_history[0].previous_password.expose_secret(), "v3");
    }

    #[test]
    fn redacted_copy_hides_password_but_keeps_username() {
        let vault_item = login_item_with_password("muy-secreta");

        let redacted_item = vault_item.redacted_copy();

        let ItemContent::Login(login_content) = &redacted_item.item_content else {
            panic!("se esperaba un login");
        };
        assert_eq!(login_content.password.expose_secret(), REDACTED_SECRET_PLACEHOLDER);
        assert_eq!(login_content.username, "ana");
    }

    #[test]
    fn revealing_card_number_on_login_is_rejected() {
        let vault_item = login_item_with_password("secreta");

        let reveal_result = vault_item.reveal_field(RevealableField::CardNumber);

        assert!(matches!(reveal_result, Err(VaultError::FieldNotAvailable)));
    }
}
