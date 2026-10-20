import { create } from "zustand";

import { readLocalPreference, writeLocalPreference } from "../../infrastructure/localPreferenceStorage";

export type ThemePreference = "system" | "light" | "dark";

const THEME_PREFERENCE_KEY = "arca.theme";
const THEME_PREFERENCES: readonly ThemePreference[] = ["system", "light", "dark"];

function readStoredThemePreference(): ThemePreference {
  const storedTheme = readLocalPreference(THEME_PREFERENCE_KEY);
  return THEME_PREFERENCES.find((themePreference) => themePreference === storedTheme) ?? "system";
}

/** "system" quita el atributo para que mande `prefers-color-scheme`. */
function applyThemeToDocument(themePreference: ThemePreference): void {
  const rootElement = document.documentElement;
  if (themePreference === "system") {
    rootElement.removeAttribute("data-theme");
    return;
  }
  rootElement.setAttribute("data-theme", themePreference);
}

interface ThemeState {
  themePreference: ThemePreference;
  changeThemePreference: (themePreference: ThemePreference) => void;
}

export const useThemeStore = create<ThemeState>((setState) => {
  const initialThemePreference = readStoredThemePreference();
  applyThemeToDocument(initialThemePreference);
  return {
    themePreference: initialThemePreference,
    changeThemePreference: (themePreference) => {
      writeLocalPreference(THEME_PREFERENCE_KEY, themePreference);
      applyThemeToDocument(themePreference);
      setState({ themePreference });
    },
  };
});

export { THEME_PREFERENCES };
