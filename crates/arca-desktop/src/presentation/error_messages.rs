//! Traduce un `UiError` a un mensaje legible para la persona usuaria.

use crate::presentation::translator::Translator;
use crate::ui_error::UiError;

type DetailedErrorDescriber = fn(&Translator, &UiError) -> Option<String>;

/// Errores cuyo mensaje depende de un código de detalle; el resto usa `errors.<código>`.
const DETAILED_ERROR_DESCRIBERS: [(&str, DetailedErrorDescriber); 1] = [("invalidInput", describe_invalid_input)];

fn describe_invalid_input(translator: &Translator, ui_error: &UiError) -> Option<String> {
    let reason_code = ui_error.technical_detail.as_deref()?;
    translator.find_text(&format!("errors.invalidInputReasons.{reason_code}")).map(str::to_string)
}

pub fn describe_error(translator: &Translator, ui_error: &UiError) -> String {
    let detailed_message = DETAILED_ERROR_DESCRIBERS
        .iter()
        .find(|(error_code, _)| *error_code == ui_error.error_code)
        .and_then(|(_, describe_detail)| describe_detail(translator, ui_error));
    if let Some(detailed_message) = detailed_message {
        return detailed_message;
    }
    let error_key = format!("errors.{}", ui_error.error_code);
    let Some(message_template) = translator.find_text(&error_key) else {
        return translator.text("errors.internal");
    };
    let retry_after_seconds = ui_error.retry_after_seconds.map(|seconds| seconds.to_string()).unwrap_or_default();
    let technical_detail = ui_error.technical_detail.clone().unwrap_or_default();
    message_template
        .replace("{{retryAfterSeconds}}", &retry_after_seconds)
        .replace("{{technicalDetail}}", &technical_detail)
}

#[cfg(test)]
mod tests {
    use vault_core::domain::vault_error::VaultError;

    use super::*;

    #[test]
    fn known_codes_use_their_translation_with_details() {
        let translator = Translator::new(Some("en"));

        let blocked_message = describe_error(
            &translator,
            &UiError::from(VaultError::UnlockTemporarilyBlocked { retry_after_seconds: 7 }),
        );

        assert_eq!(blocked_message, "Too many attempts. Wait 7 s.");
    }

    #[test]
    fn invalid_input_uses_the_specific_reason_or_falls_back() {
        let translator = Translator::new(Some("en"));

        let specific_message = describe_error(&translator, &UiError::from(VaultError::invalid_input("titleRequired")));
        let generic_message = describe_error(&translator, &UiError::from(VaultError::invalid_input("unknownReason")));

        assert_eq!(specific_message, "A name is required.");
        assert_eq!(generic_message, "Please check the data you entered.");
    }
}
