// Preferencias de interfaz por equipo (idioma, tema). Nunca datos de la bóveda.
// localStorage puede no estar disponible: toda lectura/escritura está protegida.

export function readLocalPreference(preferenceKey: string): string | null {
  try {
    return window.localStorage.getItem(preferenceKey);
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
