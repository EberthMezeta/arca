import { createContext, type InputHTMLAttributes, type ReactNode, useContext } from "react";
import { useTranslation } from "react-i18next";

import { isSealedSecret, type ItemId, type RevealableField } from "../../domain/vaultTypes";
import { vaultApiClient } from "../../infrastructure/vaultApiClient";
import { PasswordInput } from "../components/primitives/PasswordInput";

type RevealSealedSecret = (revealableField: RevealableField) => Promise<string>;

/** Al editar un elemento existente, permite pedir al backend el valor real de un secreto enmascarado. */
const SealedSecretRevealContext = createContext<RevealSealedSecret | null>(null);

export function SealedSecretRevealProvider({ itemId, children }: { itemId: ItemId | null; children: ReactNode }) {
  const revealSealedSecret: RevealSealedSecret | null = itemId
    ? (revealableField) => vaultApiClient.revealItemField(itemId, revealableField)
    : null;
  return <SealedSecretRevealContext.Provider value={revealSealedSecret}>{children}</SealedSecretRevealContext.Provider>;
}

interface SealedSecretInputProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "type" | "value" | "onChange"> {
  value: string;
  onValueChange: (secretValue: string) => void;
  /** Campo con el que revelarlo; sin él (p. ej. un campo nuevo) no hay nada guardado que revelar. */
  revealableField?: RevealableField;
}

/**
 * Campo de secreto del editor. Mientras muestra el marcador enmascarado:
 * - el ojo pide el valor real al backend (solo entonces cruza a la UI);
 * - al enfocarlo se selecciona todo, así escribir reemplaza el secreto en vez de añadirse al marcador.
 * Si se guarda sin tocarlo, el backend conserva el valor guardado.
 */
export function SealedSecretInput({ value, onValueChange, revealableField, onFocus, ...inputAttributes }: SealedSecretInputProps) {
  const { t: translate } = useTranslation();
  const revealSealedSecret = useContext(SealedSecretRevealContext);
  const isSealed = isSealedSecret(value);

  const unsealBeforeReveal = async () => {
    if (!isSealed || !revealSealedSecret || !revealableField) return true;
    try {
      onValueChange(await revealSealedSecret(revealableField));
      return true;
    } catch {
      return false;
    }
  };

  return (
    <PasswordInput
      {...inputAttributes}
      value={value}
      title={isSealed ? translate("editor.sealedSecretHint") : inputAttributes.title}
      onChange={(changeEvent) => onValueChange(changeEvent.target.value)}
      onFocus={(focusEvent) => {
        if (isSealed) focusEvent.currentTarget.select();
        onFocus?.(focusEvent);
      }}
      onBeforeReveal={unsealBeforeReveal}
    />
  );
}
