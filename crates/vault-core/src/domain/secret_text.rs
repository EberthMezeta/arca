use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use unicode_normalization::{UnicodeNormalization, is_nfc};
use zeroize::Zeroizing;

/// Texto sensible (contraseñas, números de tarjeta, notas...).
///
/// - Se borra de memoria al liberarse (`Zeroizing`).
/// - `Debug` nunca muestra el contenido, así no se filtra en logs.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct SecretText(Zeroizing<String>);

impl SecretText {
    pub fn new(plain_text: impl Into<String>) -> Self {
        Self(Zeroizing::new(plain_text.into()))
    }

    pub fn expose_secret(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn character_count(&self) -> usize {
        self.0.chars().count()
    }

    /// Forma NFC del texto: "é" escrito como un carácter o como "e" + tilde combinante
    /// (lo habitual en macOS) dan los mismos bytes, y la contraseña abre la bóveda en cualquier sistema.
    #[must_use]
    pub fn nfc_normalized(&self) -> Self {
        // NFC puede alargar el texto como mucho al triple: reservar evita realocar
        // (cada realocación dejaría una copia del secreto sin borrar).
        let mut normalized_text = Zeroizing::new(String::with_capacity(self.0.len() * 3));
        normalized_text.extend(self.0.nfc());
        Self(normalized_text)
    }

    /// Formas a probar al derivar una clave a partir de una contraseña: primero la NFC (con la
    /// que se crean las bóvedas y copias nuevas) y, si difiere, la original tal cual, para
    /// seguir abriendo lo creado antes de normalizar.
    pub fn key_derivation_candidates(&self) -> Vec<Self> {
        if is_nfc(&self.0) {
            return vec![self.clone()];
        }
        vec![self.nfc_normalized(), self.clone()]
    }
}

impl From<&str> for SecretText {
    fn from(plain_text: &str) -> Self {
        Self::new(plain_text)
    }
}

impl From<String> for SecretText {
    fn from(plain_text: String) -> Self {
        Self::new(plain_text)
    }
}

impl fmt::Debug for SecretText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretText(***)")
    }
}

impl Serialize for SecretText {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for SecretText {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let plain_text = String::deserialize(deserializer)?;
        Ok(Self::new(plain_text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composed_and_decomposed_accents_normalize_to_the_same_text() {
        let composed_password = SecretText::new("contrase\u{f1}a-caf\u{e9}");
        let decomposed_password = SecretText::new("contrasen\u{303}a-cafe\u{301}");

        assert_ne!(composed_password, decomposed_password);
        assert_eq!(composed_password.nfc_normalized(), decomposed_password.nfc_normalized());
    }

    #[test]
    fn decomposed_password_is_tried_normalized_first_then_as_typed() {
        let decomposed_password = SecretText::new("cafe\u{301}");

        let candidates = decomposed_password.key_derivation_candidates();

        assert_eq!(candidates, vec![SecretText::new("caf\u{e9}"), decomposed_password]);
        assert_eq!(SecretText::new("ascii-only").key_derivation_candidates().len(), 1);
    }

    #[test]
    fn debug_output_never_reveals_the_secret() {
        let master_password = SecretText::new("correct horse battery staple");

        let debug_output = format!("{master_password:?}");

        assert!(!debug_output.contains("horse"));
    }
}
