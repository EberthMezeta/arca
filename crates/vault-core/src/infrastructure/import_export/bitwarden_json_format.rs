//! Estructura del JSON *no cifrado* que exporta Bitwarden, y su traducción al dominio.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::item_content::{IdentityContent, ItemContent, LoginContent, PaymentCardContent, SecureNoteContent};
use crate::domain::secret_text::SecretText;
use crate::domain::vault_item::{CustomField, PasswordHistoryEntry, VaultItem};

pub const BITWARDEN_LOGIN_TYPE: u8 = 1;
pub const BITWARDEN_SECURE_NOTE_TYPE: u8 = 2;
pub const BITWARDEN_CARD_TYPE: u8 = 3;
pub const BITWARDEN_IDENTITY_TYPE: u8 = 4;
const BITWARDEN_HIDDEN_FIELD_TYPE: u8 = 1;
const BITWARDEN_TEXT_FIELD_TYPE: u8 = 0;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BitwardenExportDocument {
    #[serde(default)]
    pub encrypted: bool,
    #[serde(default)]
    pub folders: Vec<BitwardenFolder>,
    #[serde(default)]
    pub items: Vec<BitwardenItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BitwardenFolder {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BitwardenItem {
    pub id: Option<String>,
    pub folder_id: Option<String>,
    #[serde(rename = "type")]
    pub item_type: u8,
    pub name: String,
    pub notes: Option<SecretText>,
    pub favorite: bool,
    pub fields: Vec<BitwardenField>,
    pub login: Option<BitwardenLogin>,
    pub secure_note: Option<BitwardenSecureNote>,
    pub card: Option<BitwardenCard>,
    pub identity: Option<BitwardenIdentity>,
    pub password_history: Option<Vec<BitwardenPasswordHistoryEntry>>,
    pub revision_date: Option<DateTime<Utc>>,
    pub creation_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BitwardenField {
    pub name: Option<String>,
    pub value: Option<SecretText>,
    #[serde(rename = "type")]
    pub field_type: u8,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BitwardenLogin {
    pub uris: Option<Vec<BitwardenUri>>,
    pub username: Option<String>,
    pub password: Option<SecretText>,
    pub totp: Option<SecretText>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BitwardenUri {
    pub uri: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BitwardenSecureNote {
    #[serde(rename = "type")]
    pub note_type: u8,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BitwardenCard {
    pub cardholder_name: Option<String>,
    pub brand: Option<String>,
    pub number: Option<SecretText>,
    pub exp_month: Option<String>,
    pub exp_year: Option<String>,
    pub code: Option<SecretText>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BitwardenIdentity {
    pub title: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub address1: Option<String>,
    pub city: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub company: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub passport_number: Option<SecretText>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BitwardenPasswordHistoryEntry {
    pub last_used_date: Option<DateTime<Utc>>,
    pub password: Option<SecretText>,
}

// ── Bitwarden → dominio ──────────────────────────────────────────────────────

impl BitwardenItem {
    /// `None` para tipos que no soportamos (p. ej. claves SSH); se omiten al importar.
    fn to_item_content(&self) -> Option<ItemContent> {
        match self.item_type {
            BITWARDEN_LOGIN_TYPE => Some(ItemContent::Login(self.login.as_ref().map(login_content_from).unwrap_or_default())),
            BITWARDEN_SECURE_NOTE_TYPE => Some(ItemContent::SecureNote(SecureNoteContent {})),
            BITWARDEN_CARD_TYPE => Some(ItemContent::PaymentCard(self.card.as_ref().map(card_content_from).unwrap_or_default())),
            BITWARDEN_IDENTITY_TYPE => {
                Some(ItemContent::Identity(self.identity.as_ref().map(identity_content_from).unwrap_or_default()))
            }
            _ => None,
        }
    }

    pub fn to_vault_item(&self, folder_id: Option<Uuid>, now: DateTime<Utc>) -> Option<VaultItem> {
        let item_content = self.to_item_content()?;
        let has_password = item_content.login_password().is_some();
        let updated_at = self.revision_date.unwrap_or(now);
        Some(VaultItem {
            item_id: Uuid::new_v4(),
            title: non_empty_or(&self.name, "Sin título"),
            folder_id,
            tag_names: Vec::new(),
            is_favorite: self.favorite,
            notes: self.notes.clone().unwrap_or_default(),
            custom_fields: self.fields.iter().map(custom_field_from).collect(),
            item_content,
            password_history: self
                .password_history
                .iter()
                .flatten()
                .filter_map(|history_entry| password_history_entry_from(history_entry, updated_at))
                .collect(),
            attachments: Vec::new(),
            created_at: self.creation_date.unwrap_or(updated_at),
            updated_at,
            password_changed_at: has_password.then_some(updated_at),
            moved_to_trash_at: None,
        })
    }
}

fn non_empty_or(candidate_text: &str, fallback_text: &str) -> String {
    let trimmed_text = candidate_text.trim();
    if trimmed_text.is_empty() {
        return fallback_text.to_string();
    }
    trimmed_text.to_string()
}

fn login_content_from(bitwarden_login: &BitwardenLogin) -> LoginContent {
    LoginContent {
        username: bitwarden_login.username.clone().unwrap_or_default(),
        password: bitwarden_login.password.clone().unwrap_or_default(),
        website_urls: bitwarden_login
            .uris
            .iter()
            .flatten()
            .filter_map(|bitwarden_uri| bitwarden_uri.uri.clone())
            .collect(),
        totp_secret: bitwarden_login.totp.clone().filter(|totp_secret| !totp_secret.is_empty()),
    }
}

fn card_content_from(bitwarden_card: &BitwardenCard) -> PaymentCardContent {
    PaymentCardContent {
        cardholder_name: bitwarden_card.cardholder_name.clone().unwrap_or_default(),
        card_brand: bitwarden_card.brand.clone().unwrap_or_default(),
        card_number: bitwarden_card.number.clone().unwrap_or_default(),
        expiration_month: bitwarden_card.exp_month.clone().unwrap_or_default(),
        expiration_year: bitwarden_card.exp_year.clone().unwrap_or_default(),
        security_code: bitwarden_card.code.clone().unwrap_or_default(),
    }
}

fn identity_content_from(bitwarden_identity: &BitwardenIdentity) -> IdentityContent {
    IdentityContent {
        honorific_title: bitwarden_identity.title.clone().unwrap_or_default(),
        first_name: bitwarden_identity.first_name.clone().unwrap_or_default(),
        last_name: bitwarden_identity.last_name.clone().unwrap_or_default(),
        email_address: bitwarden_identity.email.clone().unwrap_or_default(),
        phone_number: bitwarden_identity.phone.clone().unwrap_or_default(),
        street_address: bitwarden_identity.address1.clone().unwrap_or_default(),
        city: bitwarden_identity.city.clone().unwrap_or_default(),
        postal_code: bitwarden_identity.postal_code.clone().unwrap_or_default(),
        country: bitwarden_identity.country.clone().unwrap_or_default(),
        company_name: bitwarden_identity.company.clone().unwrap_or_default(),
        identity_document_number: bitwarden_identity.passport_number.clone().unwrap_or_default(),
    }
}

fn custom_field_from(bitwarden_field: &BitwardenField) -> CustomField {
    CustomField {
        field_label: bitwarden_field.name.clone().unwrap_or_default(),
        field_value: bitwarden_field.value.clone().unwrap_or_default(),
        is_hidden: bitwarden_field.field_type == BITWARDEN_HIDDEN_FIELD_TYPE,
    }
}

fn password_history_entry_from(
    bitwarden_entry: &BitwardenPasswordHistoryEntry,
    fallback_date: DateTime<Utc>,
) -> Option<PasswordHistoryEntry> {
    Some(PasswordHistoryEntry {
        previous_password: bitwarden_entry.password.clone()?,
        replaced_at: bitwarden_entry.last_used_date.unwrap_or(fallback_date),
    })
}

// ── Dominio → Bitwarden ──────────────────────────────────────────────────────

impl From<&VaultItem> for BitwardenItem {
    fn from(vault_item: &VaultItem) -> Self {
        let mut bitwarden_item = Self {
            id: Some(vault_item.item_id.to_string()),
            folder_id: vault_item.folder_id.map(|folder_id| folder_id.to_string()),
            name: vault_item.title.clone(),
            notes: Some(vault_item.notes.clone()).filter(|notes| !notes.is_empty()),
            favorite: vault_item.is_favorite,
            fields: vault_item.custom_fields.iter().map(bitwarden_field_from).collect(),
            password_history: Some(vault_item.password_history.iter().map(bitwarden_history_entry_from).collect()),
            revision_date: Some(vault_item.updated_at),
            creation_date: Some(vault_item.created_at),
            ..Self::default()
        };
        fill_type_specific_content(&mut bitwarden_item, &vault_item.item_content);
        bitwarden_item
    }
}

fn fill_type_specific_content(bitwarden_item: &mut BitwardenItem, item_content: &ItemContent) {
    match item_content {
        ItemContent::Login(login_content) => {
            bitwarden_item.item_type = BITWARDEN_LOGIN_TYPE;
            bitwarden_item.login = Some(BitwardenLogin {
                uris: Some(
                    login_content
                        .website_urls
                        .iter()
                        .map(|website_url| BitwardenUri { uri: Some(website_url.clone()) })
                        .collect(),
                ),
                username: Some(login_content.username.clone()),
                password: Some(login_content.password.clone()),
                totp: login_content.totp_secret.clone(),
            });
        }
        ItemContent::SecureNote(_) => {
            bitwarden_item.item_type = BITWARDEN_SECURE_NOTE_TYPE;
            bitwarden_item.secure_note = Some(BitwardenSecureNote { note_type: 0 });
        }
        ItemContent::PaymentCard(card_content) => {
            bitwarden_item.item_type = BITWARDEN_CARD_TYPE;
            bitwarden_item.card = Some(BitwardenCard {
                cardholder_name: Some(card_content.cardholder_name.clone()),
                brand: Some(card_content.card_brand.clone()),
                number: Some(card_content.card_number.clone()),
                exp_month: Some(card_content.expiration_month.clone()),
                exp_year: Some(card_content.expiration_year.clone()),
                code: Some(card_content.security_code.clone()),
            });
        }
        ItemContent::Identity(identity_content) => {
            bitwarden_item.item_type = BITWARDEN_IDENTITY_TYPE;
            bitwarden_item.identity = Some(BitwardenIdentity {
                title: Some(identity_content.honorific_title.clone()),
                first_name: Some(identity_content.first_name.clone()),
                last_name: Some(identity_content.last_name.clone()),
                address1: Some(identity_content.street_address.clone()),
                city: Some(identity_content.city.clone()),
                postal_code: Some(identity_content.postal_code.clone()),
                country: Some(identity_content.country.clone()),
                company: Some(identity_content.company_name.clone()),
                email: Some(identity_content.email_address.clone()),
                phone: Some(identity_content.phone_number.clone()),
                passport_number: Some(identity_content.identity_document_number.clone()),
            });
        }
    }
}

fn bitwarden_field_from(custom_field: &CustomField) -> BitwardenField {
    BitwardenField {
        name: Some(custom_field.field_label.clone()),
        value: Some(custom_field.field_value.clone()),
        field_type: if custom_field.is_hidden { BITWARDEN_HIDDEN_FIELD_TYPE } else { BITWARDEN_TEXT_FIELD_TYPE },
    }
}

fn bitwarden_history_entry_from(history_entry: &PasswordHistoryEntry) -> BitwardenPasswordHistoryEntry {
    BitwardenPasswordHistoryEntry {
        last_used_date: Some(history_entry.replaced_at),
        password: Some(history_entry.previous_password.clone()),
    }
}
