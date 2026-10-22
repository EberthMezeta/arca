use serde::Serialize;

use crate::domain::secret_text::SecretText;

/// Palabras que zxcvbn no conoce (su diccionario es inglés) y que cualquiera probaría primero
/// contra una bóveda de esta app.
const CONTEXT_WORDS: &[&str] = &[
    "arca",
    "rustvault",
    "vault",
    "boveda",
    "bóveda",
    "contraseña",
    "contrasena",
    "clave",
    "maestra",
    "segura",
    "secreta",
    "password",
];

/// Estimación de fortaleza (0 = muy débil, 4 = muy fuerte) basada en zxcvbn.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordStrength {
    pub score: u8,
    pub estimated_guesses_log10: f64,
    pub warning_message: Option<String>,
    pub improvement_suggestions: Vec<String>,
}

impl PasswordStrength {
    pub fn estimate(candidate_password: &SecretText) -> Self {
        let estimation = zxcvbn::zxcvbn(candidate_password.expose_secret(), CONTEXT_WORDS);
        let feedback = estimation.feedback();

        Self {
            score: estimation.score() as u8,
            estimated_guesses_log10: estimation.guesses_log10(),
            #[allow(clippy::redundant_closure_for_method_calls)]
            warning_message: feedback.and_then(|feedback| feedback.warning()).map(|warning| warning.to_string()),
            improvement_suggestions: feedback
                .map(|feedback| feedback.suggestions().iter().map(ToString::to_string).collect())
                .unwrap_or_default(),
        }
    }
}
