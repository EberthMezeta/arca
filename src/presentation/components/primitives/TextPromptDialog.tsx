import { type FormEvent, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { useErrorMessage } from "../../../application/hooks/useErrorMessage";
import type { CommandError } from "../../../domain/vaultTypes";
import { Button } from "./Button";
import { FormField } from "./FormField";
import { ModalDialog } from "./ModalDialog";
import { PasswordInput } from "./PasswordInput";
import { TextInput } from "./TextInput";

interface TextPromptDialogProps {
  isOpen: boolean;
  onOpenChange: (isOpen: boolean) => void;
  dialogTitle: string;
  fieldLabel: string;
  hintText?: string;
  /** Contraseña: campo enmascarado, sin recortar espacios ni límite de longitud. */
  isSecretValue?: boolean;
  initialValue?: string;
  confirmLabel: string;
  onSubmitValue: (submittedValue: string) => Promise<void>;
}

/** Pide un texto corto (p. ej. nombre de carpeta o la contraseña maestra) con validación en el backend. */
export function TextPromptDialog({
  isOpen,
  onOpenChange,
  dialogTitle,
  fieldLabel,
  hintText,
  isSecretValue = false,
  initialValue = "",
  confirmLabel,
  onSubmitValue,
}: TextPromptDialogProps) {
  const { t: translate } = useTranslation();
  const [enteredValue, setEnteredValue] = useState(initialValue);
  const [submitErrorText, setSubmitErrorText] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const describeError = useErrorMessage();

  // También al cerrar: una contraseña no debe quedarse en el estado del diálogo.
  useEffect(() => {
    setEnteredValue(isOpen ? initialValue : "");
    setSubmitErrorText(null);
  }, [isOpen, initialValue]);

  const submittedValue = isSecretValue ? enteredValue : enteredValue.trim();

  const handleSubmit = async (formEvent: FormEvent) => {
    formEvent.preventDefault();
    if (!submittedValue) return;
    setIsSubmitting(true);
    try {
      await onSubmitValue(submittedValue);
      onOpenChange(false);
    } catch (submitError) {
      setSubmitErrorText(describeError(submitError as CommandError));
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <ModalDialog isOpen={isOpen} onOpenChange={onOpenChange} dialogTitle={dialogTitle}>
      <form onSubmit={handleSubmit} className="flex flex-col gap-5">
        <FormField fieldLabel={fieldLabel} hintText={hintText} errorText={submitErrorText}>
          {(fieldControlId) =>
            isSecretValue ? (
              <PasswordInput
                id={fieldControlId}
                autoFocus
                value={enteredValue}
                onChange={(changeEvent) => setEnteredValue(changeEvent.target.value)}
              />
            ) : (
              <TextInput
                id={fieldControlId}
                autoFocus
                value={enteredValue}
                maxLength={100}
                onChange={(changeEvent) => setEnteredValue(changeEvent.target.value)}
              />
            )
          }
        </FormField>
        <div className="flex justify-end gap-2">
          <Button onClick={() => onOpenChange(false)}>{translate("actions.cancel")}</Button>
          <Button type="submit" buttonVariant="primary" isBusy={isSubmitting} disabled={!submittedValue}>
            {confirmLabel}
          </Button>
        </div>
      </form>
    </ModalDialog>
  );
}
