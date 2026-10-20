import { forwardRef, type InputHTMLAttributes, type TextareaHTMLAttributes } from "react";

export const INPUT_BASE_CLASSES =
  "w-full rounded-lg border border-border bg-surface px-3 text-sm text-text placeholder:text-muted/70 focus:border-accent focus:outline-none focus:ring-2 focus:ring-[var(--focus-ring)]";

export const TextInput = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(function TextInput(
  { className = "", ...inputAttributes },
  forwardedRef,
) {
  return (
    <input
      ref={forwardedRef}
      spellCheck={false}
      autoComplete="off"
      className={`h-9 ${INPUT_BASE_CLASSES} ${className}`}
      {...inputAttributes}
    />
  );
});

export function TextArea({ className = "", ...textAreaAttributes }: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return <textarea className={`min-h-24 py-2 ${INPUT_BASE_CLASSES} ${className}`} {...textAreaAttributes} />;
}
