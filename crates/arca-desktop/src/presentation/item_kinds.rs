//! Fuente única de verdad por tipo de elemento: textos, icono, contenido vacío y campos descriptivos.
//! Añadir un tipo es añadir una entrada (el `match` exhaustivo de `definition_for` obliga a ello).

use vault_core::domain::item_content::{
    IdentityContent, ItemContent, ItemContentType, LoginContent, PaymentCardContent, SecureNoteContent,
};
use vault_core::domain::secret_text::SecretText;
use vault_core::domain::vault_item::RevealableField;

use crate::{ItemKind, NavIcon};

pub struct ItemKindDefinition {
    pub content_type: ItemContentType,
    pub ui_kind: ItemKind,
    pub nav_icon: NavIcon,
    pub singular_label_key: &'static str,
    pub plural_label_key: &'static str,
    pub create_empty_content: fn() -> ItemContent,
    /// Campos que se editan y muestran con la rejilla genérica (tarjeta, identidad).
    pub descriptor_fields: &'static [ContentFieldDescriptor],
}

/// Campo de texto descrito de forma declarativa: sirve a la vez para el editor y para la vista de detalle.
pub struct ContentFieldDescriptor {
    pub label_key: &'static str,
    pub read_value: fn(&ItemContent) -> String,
    pub write_value: fn(&mut ItemContent, String),
    /// `Some` si es un secreto: llega enmascarado y se revela bajo demanda.
    pub revealable_field: Option<RevealableField>,
    pub is_wide: bool,
}

/// Genera un descriptor para un campo de `$content_variant`. Los secretos (`SecretText`) se marcan con `secret`.
macro_rules! content_field {
    ($content_variant:ident, $field_name:ident, $label_key:literal, wide: $is_wide:literal) => {
        ContentFieldDescriptor {
            label_key: $label_key,
            read_value: |item_content| match item_content {
                ItemContent::$content_variant(typed_content) => typed_content.$field_name.clone(),
                _ => String::new(),
            },
            write_value: |item_content, new_value| {
                if let ItemContent::$content_variant(typed_content) = item_content {
                    typed_content.$field_name = new_value;
                }
            },
            revealable_field: None,
            is_wide: $is_wide,
        }
    };
    ($content_variant:ident, $field_name:ident, $label_key:literal, wide: $is_wide:literal, secret: $revealable_field:expr) => {
        ContentFieldDescriptor {
            label_key: $label_key,
            read_value: |item_content| match item_content {
                ItemContent::$content_variant(typed_content) => typed_content.$field_name.expose_secret().to_string(),
                _ => String::new(),
            },
            write_value: |item_content, new_value| {
                if let ItemContent::$content_variant(typed_content) = item_content {
                    typed_content.$field_name = SecretText::new(new_value);
                }
            },
            revealable_field: Some($revealable_field),
            is_wide: $is_wide,
        }
    };
}

const PAYMENT_CARD_FIELDS: [ContentFieldDescriptor; 6] = [
    content_field!(PaymentCard, cardholder_name, "fields.cardholderName", wide: true),
    content_field!(PaymentCard, card_brand, "fields.cardBrand", wide: true),
    content_field!(PaymentCard, card_number, "fields.cardNumber", wide: true, secret: RevealableField::CardNumber),
    content_field!(PaymentCard, expiration_month, "fields.expirationMonth", wide: false),
    content_field!(PaymentCard, expiration_year, "fields.expirationYear", wide: false),
    content_field!(PaymentCard, security_code, "fields.securityCode", wide: false, secret: RevealableField::CardSecurityCode),
];

const IDENTITY_FIELDS: [ContentFieldDescriptor; 11] = [
    content_field!(Identity, honorific_title, "fields.honorificTitle", wide: false),
    content_field!(Identity, first_name, "fields.firstName", wide: false),
    content_field!(Identity, last_name, "fields.lastName", wide: true),
    content_field!(Identity, email_address, "fields.emailAddress", wide: false),
    content_field!(Identity, phone_number, "fields.phoneNumber", wide: false),
    content_field!(Identity, company_name, "fields.companyName", wide: true),
    content_field!(Identity, street_address, "fields.streetAddress", wide: true),
    content_field!(Identity, city, "fields.city", wide: false),
    content_field!(Identity, postal_code, "fields.postalCode", wide: false),
    content_field!(Identity, country, "fields.country", wide: true),
    content_field!(
        Identity,
        identity_document_number,
        "fields.identityDocumentNumber",
        wide: true,
        secret: RevealableField::IdentityDocumentNumber
    ),
];

pub const ITEM_KIND_DEFINITIONS: [ItemKindDefinition; 4] = [
    ItemKindDefinition {
        content_type: ItemContentType::Login,
        ui_kind: ItemKind::Login,
        nav_icon: NavIcon::Login,
        singular_label_key: "itemTypes.login",
        plural_label_key: "itemTypes.logins",
        create_empty_content: || ItemContent::Login(LoginContent { website_urls: vec![String::new()], ..LoginContent::default() }),
        descriptor_fields: &[],
    },
    ItemKindDefinition {
        content_type: ItemContentType::SecureNote,
        ui_kind: ItemKind::SecureNote,
        nav_icon: NavIcon::SecureNote,
        singular_label_key: "itemTypes.secureNote",
        plural_label_key: "itemTypes.secureNotes",
        create_empty_content: || ItemContent::SecureNote(SecureNoteContent {}),
        descriptor_fields: &[],
    },
    ItemKindDefinition {
        content_type: ItemContentType::PaymentCard,
        ui_kind: ItemKind::PaymentCard,
        nav_icon: NavIcon::PaymentCard,
        singular_label_key: "itemTypes.paymentCard",
        plural_label_key: "itemTypes.paymentCards",
        create_empty_content: || ItemContent::PaymentCard(PaymentCardContent::default()),
        descriptor_fields: &PAYMENT_CARD_FIELDS,
    },
    ItemKindDefinition {
        content_type: ItemContentType::Identity,
        ui_kind: ItemKind::Identity,
        nav_icon: NavIcon::Identity,
        singular_label_key: "itemTypes.identity",
        plural_label_key: "itemTypes.identities",
        create_empty_content: || ItemContent::Identity(IdentityContent::default()),
        descriptor_fields: &IDENTITY_FIELDS,
    },
];

/// Búsqueda por tipo (no por posición): reordenar la tabla no puede devolver otra definición.
pub fn definition_for(content_type: ItemContentType) -> &'static ItemKindDefinition {
    ITEM_KIND_DEFINITIONS
        .iter()
        .find(|definition| definition.content_type == content_type)
        .unwrap_or(&ITEM_KIND_DEFINITIONS[0])
}

pub fn definition_for_ui_kind(ui_kind: ItemKind) -> &'static ItemKindDefinition {
    ITEM_KIND_DEFINITIONS
        .iter()
        .find(|definition| definition.ui_kind == ui_kind)
        .unwrap_or(&ITEM_KIND_DEFINITIONS[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lo que antes garantizaba el `match` de `definition_for`: cada tipo tiene su entrada.
    #[test]
    fn every_content_type_has_a_definition() {
        let all_content_types = [ItemContentType::Login, ItemContentType::SecureNote, ItemContentType::PaymentCard, ItemContentType::Identity];
        for content_type in all_content_types {
            // Sin comodín: si se añade un tipo, este `match` deja de compilar hasta incluirlo arriba.
            match content_type {
                ItemContentType::Login | ItemContentType::SecureNote | ItemContentType::PaymentCard | ItemContentType::Identity => {}
            }
            assert!(ITEM_KIND_DEFINITIONS.iter().any(|definition| definition.content_type == content_type), "{content_type:?}");
        }
    }

    #[test]
    fn every_content_type_maps_to_its_own_definition() {
        for definition in &ITEM_KIND_DEFINITIONS {
            assert_eq!(definition_for(definition.content_type).content_type, definition.content_type);
            assert_eq!((definition.create_empty_content)().content_type(), definition.content_type);
        }
    }

    #[test]
    fn descriptors_read_back_what_they_write() {
        let mut card_content = (definition_for(ItemContentType::PaymentCard).create_empty_content)();
        for field_descriptor in &PAYMENT_CARD_FIELDS {
            (field_descriptor.write_value)(&mut card_content, field_descriptor.label_key.to_string());
            assert_eq!((field_descriptor.read_value)(&card_content), field_descriptor.label_key);
        }
    }
}
