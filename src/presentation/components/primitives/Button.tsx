import type { ButtonHTMLAttributes, ReactNode } from "react";

import { Spinner } from "./Spinner";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";

const BUTTON_VARIANT_CLASSES: Record<ButtonVariant, string> = {
  primary: "bg-accent text-accent-contrast hover:bg-accent-hover",
  secondary: "bg-surface border border-border text-text hover:bg-surface-hover",
  ghost: "text-text hover:bg-surface-hover",
  danger: "bg-danger text-white hover:opacity-90",
};

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  buttonVariant?: ButtonVariant;
  leadingIcon?: ReactNode;
  isBusy?: boolean;
}

export function Button({
  buttonVariant = "secondary",
  leadingIcon,
  isBusy = false,
  className = "",
  disabled,
  children,
  type = "button",
  ...buttonAttributes
}: ButtonProps) {
  return (
    <button
      type={type}
      disabled={disabled || isBusy}
      className={`inline-flex h-9 items-center justify-center gap-2 rounded-lg px-3.5 text-sm font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50 ${BUTTON_VARIANT_CLASSES[buttonVariant]} ${className}`}
      {...buttonAttributes}
    >
      {isBusy ? <Spinner /> : leadingIcon}
      {children}
    </button>
  );
}
