import i18next from "i18next";
import { initReactI18next } from "react-i18next";

import { readLocalPreference, writeLocalPreference } from "../infrastructure/localPreferenceStorage";
import englishTranslations from "./locales/en.json";
import spanishTranslations from "./locales/es.json";

export const SUPPORTED_LANGUAGES = {
  es: { nativeName: "Español", translations: spanishTranslations },
  en: { nativeName: "English", translations: englishTranslations },
} as const;

export type SupportedLanguage = keyof typeof SUPPORTED_LANGUAGES;

const LANGUAGE_PREFERENCE_KEY = "arca.language";

function isSupportedLanguage(candidateLanguage: string | null): candidateLanguage is SupportedLanguage {
  return candidateLanguage !== null && candidateLanguage in SUPPORTED_LANGUAGES;
}

function detectInitialLanguage(): SupportedLanguage {
  const storedLanguage = readLocalPreference(LANGUAGE_PREFERENCE_KEY);
  if (isSupportedLanguage(storedLanguage)) return storedLanguage;
  const browserLanguage = navigator.language.slice(0, 2);
  return isSupportedLanguage(browserLanguage) ? browserLanguage : "en";
}

const translationResources = Object.fromEntries(
  Object.entries(SUPPORTED_LANGUAGES).map(([languageCode, languageDefinition]) => [
    languageCode,
    { translation: languageDefinition.translations },
  ]),
);

void i18next.use(initReactI18next).init({
  resources: translationResources,
  lng: detectInitialLanguage(),
  fallbackLng: "en",
  interpolation: { escapeValue: false },
});

document.documentElement.lang = i18next.language;

export function changeLanguage(selectedLanguage: SupportedLanguage): void {
  writeLocalPreference(LANGUAGE_PREFERENCE_KEY, selectedLanguage);
  document.documentElement.lang = selectedLanguage;
  void i18next.changeLanguage(selectedLanguage);
}

export default i18next;
