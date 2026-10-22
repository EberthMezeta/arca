use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::domain::secret_text::SecretText;
use crate::domain::vault_item::RevealableField;

/// Contenido específico de cada tipo de elemento.
///
/// Se usa un `enum` con `match` exhaustivo: añadir un tipo nuevo obliga al
/// compilador a revisar cada lugar que lo trata (sin `if item_type == ...`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "contentType", rename_all = "camelCase")]
pub enum ItemContent {
    Login(LoginContent),
    SecureNote(SecureNoteContent),
    PaymentCard(PaymentCardContent),
    Identity(IdentityContent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemContentType {
    Login,
    SecureNote,
    PaymentCard,
    Identity,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LoginContent {
    pub username: String,
    pub password: SecretText,
    pub website_urls: Vec<String>,
    pub totp_secret: Option<SecretText>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SecureNoteContent {}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PaymentCardContent {
    pub cardholder_name: String,
    pub card_brand: String,
    pub card_number: SecretText,
    pub expiration_month: String,
    pub expiration_year: String,
    pub security_code: SecretText,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct IdentityContent {
    pub honorific_title: String,
    pub first_name: String,
    pub last_name: String,
    pub email_address: String,
    pub phone_number: String,
    pub street_address: String,
    pub city: String,
    pub postal_code: String,
    pub country: String,
    pub company_name: String,
    pub identity_document_number: SecretText,
}

impl ItemContent {
    pub fn content_type(&self) -> ItemContentType {
        match self {
            Self::Login(_) => ItemContentType::Login,
            Self::SecureNote(_) => ItemContentType::SecureNote,
            Self::PaymentCard(_) => ItemContentType::PaymentCard,
            Self::Identity(_) => ItemContentType::Identity,
        }
    }

    /// Borra usuarios, URLs y datos personales (no son `SecretText`, pero tampoco deben
    /// quedar en memoria liberada).
    pub fn zeroize_plain_text(&mut self) {
        match self {
            Self::Login(login_content) => {
                login_content.username.zeroize();
                login_content.website_urls.zeroize();
            }
            Self::SecureNote(_) => {}
            Self::PaymentCard(card_content) => [
                &mut card_content.cardholder_name,
                &mut card_content.card_brand,
                &mut card_content.expiration_month,
                &mut card_content.expiration_year,
            ]
            .into_iter()
            .for_each(Zeroize::zeroize),
            Self::Identity(identity_content) => [
                &mut identity_content.honorific_title,
                &mut identity_content.first_name,
                &mut identity_content.last_name,
                &mut identity_content.email_address,
                &mut identity_content.phone_number,
                &mut identity_content.street_address,
                &mut identity_content.city,
                &mut identity_content.postal_code,
                &mut identity_content.country,
                &mut identity_content.company_name,
            ]
            .into_iter()
            .for_each(Zeroize::zeroize),
        }
    }

    /// Los secretos que se enmascaran al enviar el elemento a la interfaz, con su campo.
    /// Fuente única: la usan tanto el enmascarado (`VaultItem::redacted_copy`) como la
    /// restauración de lo que la interfaz no tocó (`save_vault_item`).
    pub fn masked_secrets_mut(&mut self) -> Vec<(RevealableField, &mut SecretText)> {
        match self {
            Self::Login(login_content) => {
                let mut masked_secrets = vec![(RevealableField::Password, &mut login_content.password)];
                if let Some(totp_secret) = login_content.totp_secret.as_mut() {
                    masked_secrets.push((RevealableField::TotpSecret, totp_secret));
                }
                masked_secrets
            }
            Self::SecureNote(_) => Vec::new(),
            Self::PaymentCard(card_content) => vec![
                (RevealableField::CardNumber, &mut card_content.card_number),
                (RevealableField::CardSecurityCode, &mut card_content.security_code),
            ],
            Self::Identity(identity_content) => {
                vec![(RevealableField::IdentityDocumentNumber, &mut identity_content.identity_document_number)]
            }
        }
    }

    pub fn login_password(&self) -> Option<&SecretText> {
        let Self::Login(login_content) = self else {
            return None;
        };
        Some(&login_content.password).filter(|password| !password.is_empty())
    }

    pub fn totp_secret(&self) -> Option<&SecretText> {
        let Self::Login(login_content) = self else {
            return None;
        };
        login_content.totp_secret.as_ref().filter(|totp_secret| !totp_secret.is_empty())
    }

    pub fn primary_website_url(&self) -> Option<&str> {
        let Self::Login(login_content) = self else {
            return None;
        };
        login_content.website_urls.first().map(String::as_str)
    }

    /// Texto secundario no sensible para la lista (usuario, últimos dígitos, email...).
    pub fn non_sensitive_subtitle(&self) -> String {
        match self {
            Self::Login(login_content) => login_content.username.clone(),
            Self::SecureNote(_) => String::new(),
            Self::PaymentCard(card_content) => masked_card_number(&card_content.card_number),
            Self::Identity(identity_content) => {
                format!("{} {}", identity_content.first_name, identity_content.last_name).trim().to_string()
            }
        }
    }

    /// Texto no sensible sobre el que se puede buscar.
    pub fn searchable_text(&self) -> String {
        match self {
            Self::Login(login_content) => {
                format!("{} {}", login_content.username, login_content.website_urls.join(" "))
            }
            Self::SecureNote(_) => String::new(),
            Self::PaymentCard(card_content) => {
                format!("{} {}", card_content.cardholder_name, card_content.card_brand)
            }
            Self::Identity(identity_content) => format!(
                "{} {} {} {}",
                identity_content.first_name,
                identity_content.last_name,
                identity_content.email_address,
                identity_content.company_name
            ),
        }
    }
}

fn masked_card_number(card_number: &SecretText) -> String {
    let digits: Vec<char> = card_number.expose_secret().chars().filter(char::is_ascii_digit).collect();
    if digits.len() < 4 {
        return String::new();
    }
    let last_four_digits: String = digits[digits.len() - 4..].iter().collect();
    format!("•••• {last_four_digits}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_subtitle_only_shows_last_four_digits() {
        let card_content = ItemContent::PaymentCard(PaymentCardContent {
            card_number: SecretText::new("4111 1111 1111 1234"),
            ..PaymentCardContent::default()
        });

        assert_eq!(card_content.non_sensitive_subtitle(), "•••• 1234");
    }
}
