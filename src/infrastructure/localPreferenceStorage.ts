// Preferencias de interfaz por equipo (idioma, tema). Nunca datos de la bóveda.
// localStorage puede no estar disponible: toda lectura/escritura está protegida.

/** Prefijo de las claves cuando la app se llamaba RustVault (`rustvault.theme`…). */
const LEGACY_PREFERENCE_PREFIX = "rustvault.";
const PREFERENCE_PREFIX = "arca.";

/** Si la clave nueva no existe, adopta (y borra) la que guardó la versión anterior. */
function adoptLegacyPreference(preferenceKey: string): string | null {
  if (!preferenceKey.startsWith(PREFERENCE_PREFIX)) return null;
  const legacyPreferenceKey = LEGACY_PREFERENCE_PREFIX + preferenceKey.slice(PREFERENCE_PREFIX.length);
  const legacyPreferenceValue = window.localStorage.getItem(legacyPreferenceKey);
  if (legacyPreferenceValue === null) return null;
  window.localStorage.setItem(preferenceKey, legacyPreferenceValue);
  window.localStorage.removeItem(legacyPreferenceKey);
  return legacyPreferenceValue;
}

export function readLocalPreference(preferenceKey: string): string | null {
  try {
    return window.localStorage.getItem(preferenceKey) ?? adoptLegacyPreference(preferenceKey);
  } catch {
    return null;
  }
}

export function writeLocalPreference(preferenceKey: string, preferenceValue: string): void {
  try {
    window.localStorage.setItem(preferenceKey, preferenceValue);
  } catch {
    // Sin almacenamiento local la app sigue funcionando con los valores por defecto.
  }
}
