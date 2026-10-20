import { useTranslation } from "react-i18next";

import { Button } from "./Button";
import { ModalDialog } from "./ModalDialog";

interface ConfirmDialogProps {
  isOpen: boolean;
  onOpenChange: (isOpen: boolean) => void;
  dialogTitle: string;
  dialogDescription: string;
  confirmLabel: string;
  isDestructive?: boolean;
  onConfirm: () => void;
}

export function ConfirmDialog({
  isOpen,
  onOpenChange,
  dialogTitle,
  dialogDescription,
  confirmLabel,
  isDestructive = false,
  onConfirm,
}: ConfirmDialogProps) {
  const { t: translate } = useTranslation();
  return (
    <ModalDialog isOpen={isOpen} onOpenChange={onOpenChange} dialogTitle={dialogTitle} dialogDescription={dialogDescription}>
      <div className="flex justify-end gap-2">
        <Button onClick={() => onOpenChange(false)}>{translate("actions.cancel")}</Button>
        <Button
          buttonVariant={isDestructive ? "danger" : "primary"}
          onClick={() => {
            onConfirm();
            onOpenChange(false);
          }}
        >
          {confirmLabel}
        </Button>
      </div>
    </ModalDialog>
  );
}
