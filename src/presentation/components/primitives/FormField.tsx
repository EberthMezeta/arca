import { type ReactNode, useId } from "react";

interface FormFieldProps {
  fieldLabel: string;
  hintText?: string;
  errorText?: string | null;
  children: (fieldControlId: string) => ReactNode;
}

/** Etiqueta + control + ayuda/error, enlazados para lectores de pantalla. */
export function FormField({ fieldLabel, hintText, errorText, children }: FormFieldProps) {
  const fieldControlId = useId();
  return (
    <div className="flex flex-col gap-1.5">
      <label htmlFor={fieldControlId} className="text-xs font-semibold tracking-wide text-muted uppercase">
        {fieldLabel}
      </label>
      {children(fieldControlId)}
      {errorText ? (
        <p role="alert" className="text-xs text-danger">
          {errorText}
        </p>
      ) : (
        hintText && <p className="text-xs text-muted">{hintText}</p>
      )}
    </div>
  );
}
