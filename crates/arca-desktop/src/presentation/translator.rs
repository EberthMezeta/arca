//! Traducciones con el mismo formato que usaba i18next (los JSON no cambiaron):
//! claves con puntos, interpolación `{{nombre}}` y plurales con sufijos `_zero`, `_one`, `_other`.

use std::collections::HashMap;

use serde_json::Value;

pub struct SupportedLanguage {
    pub language_code: &'static str,
    pub native_name: &'static str,
    catalog_source: &'static str,
}

/// Registro de idiomas: añadir uno es añadir su JSON y una entrada aquí.
pub const SUPPORTED_LANGUAGES: [SupportedLanguage; 2] = [
    SupportedLanguage {
        language_code: "es",
        native_name: "Español",
        catalog_source: include_str!("../../i18n/es.json"),
    },
    SupportedLanguage {
        language_code: "en",
        native_name: "English",
        catalog_source: include_str!("../../i18n/en.json"),
    },
];

const FALLBACK_LANGUAGE_CODE: &str = "en";

type TranslationCatalog = HashMap<String, String>;

pub struct Translator {
    active_language_code: &'static str,
    catalogs_by_language: HashMap<&'static str, TranslationCatalog>,
}

impl Translator {
    pub fn new(requested_language_code: Option<&str>) -> Self {
        let catalogs_by_language = SUPPORTED_LANGUAGES
            .iter()
            .map(|supported_language| {
                (supported_language.language_code, flatten_catalog(supported_language.catalog_source))
            })
            .collect();
        let mut translator = Self { active_language_code: FALLBACK_LANGUAGE_CODE, catalogs_by_language };
        translator.change_language(requested_language_code.unwrap_or(&system_language_code()));
        translator
    }

    pub fn active_language_code(&self) -> &'static str {
        self.active_language_code
    }

    /// Un código desconocido deja el idioma de reserva (inglés).
    pub fn change_language(&mut self, requested_language_code: &str) {
        self.active_language_code = SUPPORTED_LANGUAGES
            .iter()
            .find(|supported_language| supported_language.language_code == requested_language_code)
            .map_or(FALLBACK_LANGUAGE_CODE, |supported_language| supported_language.language_code);
    }

    pub fn find_text(&self, translation_key: &str) -> Option<&str> {
        [self.active_language_code, FALLBACK_LANGUAGE_CODE]
            .into_iter()
            .find_map(|language_code| self.catalogs_by_language.get(language_code)?.get(translation_key))
            .map(String::as_str)
    }

    /// Una clave sin traducción se muestra tal cual (como i18next): delata el fallo sin romper la UI.
    pub fn text(&self, translation_key: &str) -> String {
        self.find_text(translation_key).unwrap_or(translation_key).to_string()
    }

    pub fn text_with(&self, translation_key: &str, interpolated_values: &[(&str, &str)]) -> String {
        interpolate(&self.text(translation_key), interpolated_values)
    }

    /// Elige la forma plural y sustituye `{{count}}` además de los valores indicados.
    pub fn count_text(&self, translation_key: &str, count: i64, interpolated_values: &[(&str, &str)]) -> String {
        let count_text = count.to_string();
        let mut all_values = vec![("count", count_text.as_str())];
        all_values.extend_from_slice(interpolated_values);
        let plural_key = self.plural_key(translation_key, count);
        interpolate(&self.text(&plural_key), &all_values)
    }

    /// `_zero` solo si existe; español e inglés comparten la regla "uno / el resto".
    fn plural_key(&self, translation_key: &str, count: i64) -> String {
        let zero_key = format!("{translation_key}_zero");
        if count == 0 && self.find_text(&zero_key).is_some() {
            return zero_key;
        }
        let plural_suffix = if count == 1 { "one" } else { "other" };
        format!("{translation_key}_{plural_suffix}")
    }
}

fn interpolate(template_text: &str, interpolated_values: &[(&str, &str)]) -> String {
    interpolated_values.iter().fold(template_text.to_string(), |partial_text, (placeholder_name, placeholder_value)| {
        partial_text.replace(&format!("{{{{{placeholder_name}}}}}"), placeholder_value)
    })
}

/// `{"a": {"b": "x"}}` → `{"a.b": "x"}`.
fn flatten_catalog(catalog_source: &str) -> TranslationCatalog {
    let mut flattened_catalog = TranslationCatalog::new();
    if let Ok(catalog_root) = serde_json::from_str::<Value>(catalog_source) {
        flatten_into(&mut flattened_catalog, "", &catalog_root);
    }
    flattened_catalog
}

fn flatten_into(flattened_catalog: &mut TranslationCatalog, key_prefix: &str, catalog_node: &Value) {
    match catalog_node {
        Value::Object(child_nodes) => child_nodes.iter().for_each(|(child_key, child_node)| {
            let child_path =
                if key_prefix.is_empty() { child_key.clone() } else { format!("{key_prefix}.{child_key}") };
            flatten_into(flattened_catalog, &child_path, child_node);
        }),
        Value::String(translated_text) => {
            flattened_catalog.insert(key_prefix.to_string(), translated_text.clone());
        }
        _ => {}
    }
}

fn system_language_code() -> String {
    sys_locale::get_locale().map_or_else(
        || FALLBACK_LANGUAGE_CODE.to_string(),
        |system_locale| system_locale.chars().take(2).collect::<String>().to_lowercase(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_keys_are_translated_in_the_active_language() {
        let translator = Translator::new(Some("es"));

        assert_eq!(translator.text("actions.save"), "Guardar");
        assert_eq!(translator.text("settings.theme.dark"), "Oscuro");
    }

    #[test]
    fn placeholders_are_interpolated() {
        let translator = Translator::new(Some("en"));

        let interpolated_text = translator.text_with("details.movedToTrash", &[("itemTitle", "Mail")]);

        assert!(interpolated_text.contains("Mail"));
        assert!(!interpolated_text.contains("{{"));
    }

    #[test]
    fn plural_forms_follow_the_count_and_zero_is_optional() {
        let translator = Translator::new(Some("es"));

        assert_eq!(translator.count_text("search.resultCount", 1, &[]), "1 elemento");
        assert_eq!(translator.count_text("search.resultCount", 3, &[]), "3 elementos");
        assert_eq!(translator.count_text("search.resultCount", 0, &[]), "0 elementos");
        assert_eq!(translator.count_text("settings.historyEntries", 0, &[]), "Desactivado");
    }

    #[test]
    fn unknown_language_and_missing_keys_degrade_gracefully() {
        let translator = Translator::new(Some("tlh"));

        assert_eq!(translator.active_language_code(), "en");
        assert_eq!(translator.text("no.such.key"), "no.such.key");
    }

    #[test]
    fn every_language_has_the_same_keys_as_the_fallback() {
        let fallback_catalog = flatten_catalog(SUPPORTED_LANGUAGES[1].catalog_source);
        for supported_language in &SUPPORTED_LANGUAGES {
            let language_catalog = flatten_catalog(supported_language.catalog_source);
            let missing_keys: Vec<&String> =
                fallback_catalog.keys().filter(|fallback_key| !language_catalog.contains_key(*fallback_key)).collect();
            assert!(missing_keys.is_empty(), "{} no traduce {missing_keys:?}", supported_language.language_code);
        }
    }
}
