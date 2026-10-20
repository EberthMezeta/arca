use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
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
    fn debug_output_never_reveals_the_secret() {
        let master_password = SecretText::new("correct horse battery staple");

        let debug_output = format!("{master_password:?}");

        assert!(!debug_output.contains("horse"));
    }
}
