import { useEffect, useRef } from "react";

/** Mapa de combinación → acción, p. ej. `{ "mod+k": abrirPaleta, "escape": cerrar }`. */
export type KeyboardShortcutMap = Record<string, (keyboardEvent: KeyboardEvent) => void>;

function describeKeyCombination(keyboardEvent: KeyboardEvent): string {
  const pressedModifiers = [
    keyboardEvent.ctrlKey || keyboardEvent.metaKey ? "mod" : null,
    keyboardEvent.shiftKey ? "shift" : null,
    keyboardEvent.altKey ? "alt" : null,
  ].filter(Boolean);
  return [...pressedModifiers, keyboardEvent.key.toLowerCase()].join("+");
}

/** Con un diálogo modal abierto, los atajos globales se desactivan (el diálogo gestiona su teclado). */
function isModalDialogOpen(): boolean {
  return document.querySelector("[role='dialog'][data-state='open']") !== null;
}

export function useKeyboardShortcuts(keyboardShortcutMap: KeyboardShortcutMap, isEnabled = true): void {
  const latestShortcutMap = useRef(keyboardShortcutMap);
  latestShortcutMap.current = keyboardShortcutMap;

  useEffect(() => {
    if (!isEnabled) return;
    const handleKeyDown = (keyboardEvent: KeyboardEvent) => {
      const shortcutAction = latestShortcutMap.current[describeKeyCombination(keyboardEvent)];
      if (!shortcutAction || keyboardEvent.defaultPrevented || isModalDialogOpen()) return;
      keyboardEvent.preventDefault();
      shortcutAction(keyboardEvent);
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isEnabled]);
}
