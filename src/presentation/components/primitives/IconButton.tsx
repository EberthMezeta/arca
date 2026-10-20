import * as Tooltip from "@radix-ui/react-tooltip";
import type { ButtonHTMLAttributes, ReactNode } from "react";

interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  accessibleLabel: string;
  icon: ReactNode;
  isActive?: boolean;
}

/** Botón solo con icono: siempre lleva etiqueta accesible y tooltip. */
export function IconButton({ accessibleLabel, icon, isActive = false, className = "", type = "button", ...buttonAttributes }: IconButtonProps) {
  return (
    <Tooltip.Root>
      <Tooltip.Trigger asChild>
        <button
          type={type}
          aria-label={accessibleLabel}
          className={`inline-flex size-8 shrink-0 items-center justify-center rounded-md text-muted transition-colors hover:bg-surface-hover hover:text-text disabled:opacity-40 ${isActive ? "text-accent" : ""} ${className}`}
          {...buttonAttributes}
        >
          {icon}
        </button>
      </Tooltip.Trigger>
      <Tooltip.Portal>
        <Tooltip.Content
          sideOffset={6}
          className="z-50 rounded-md bg-text px-2 py-1 text-xs font-medium text-canvas shadow-lg"
        >
          {accessibleLabel}
        </Tooltip.Content>
      </Tooltip.Portal>
    </Tooltip.Root>
  );
}
