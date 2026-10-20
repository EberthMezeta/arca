import * as Dialog from "@radix-ui/react-dialog";
import type { ReactNode } from "react";

interface ModalDialogProps {
  isOpen: boolean;
  onOpenChange: (isOpen: boolean) => void;
  dialogTitle: string;
  dialogDescription?: string;
  children: ReactNode;
  widthClassName?: string;
}

/** Diálogo accesible (foco atrapado, Escape para cerrar) basado en Radix. */
export function ModalDialog({
  isOpen,
  onOpenChange,
  dialogTitle,
  dialogDescription,
  children,
  widthClassName = "max-w-md",
}: ModalDialogProps) {
  return (
    <Dialog.Root open={isOpen} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 z-40 bg-black/40 backdrop-blur-[2px]" />
        <Dialog.Content
          className={`fixed top-1/2 left-1/2 z-50 w-[calc(100vw-32px)] -translate-x-1/2 -translate-y-1/2 rounded-2xl border border-border bg-surface p-6 shadow-2xl focus:outline-none ${widthClassName}`}
        >
          <Dialog.Title className="text-lg font-semibold">{dialogTitle}</Dialog.Title>
          {dialogDescription ? (
            <Dialog.Description className="mt-1.5 text-sm text-muted">{dialogDescription}</Dialog.Description>
          ) : (
            <Dialog.Description className="sr-only">{dialogTitle}</Dialog.Description>
          )}
          <div className="mt-5">{children}</div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
