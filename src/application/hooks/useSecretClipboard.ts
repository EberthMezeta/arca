import { useCallback } from "react";
import { useTranslation } from "react-i18next";

import type { ClipboardCopyResult, CommandError, ItemId, RevealableField } from "../../domain/vaultTypes";
import { vaultApiClient } from "../../infrastructure/vaultApiClient";
import { useToastStore } from "../stores/toastStore";
import { useErrorMessage } from "./useErrorMessage";

/** Copia secretos desde Rust y muestra "Copiado · se borrará en N s". */
export function useSecretClipboard() {
  const { t: translate } = useTranslation();
  const showToast = useToastStore((toastState) => toastState.showToast);
  const describeError = useErrorMessage();

  const runCopy = useCallback(
    async (copyOperation: () => Promise<ClipboardCopyResult>, copiedLabel: string) => {
      try {
        const { clearAfterSeconds } = await copyOperation();
        showToast({
          message: translate("clipboard.copied", { copiedLabel, clearAfterSeconds }),
          tone: "success",
          countdownSeconds: clearAfterSeconds,
          durationMilliseconds: 2500,
        });
      } catch (commandError) {
        showToast({ message: describeError(commandError as CommandError), tone: "error" });
      }
    },
    [describeError, showToast, translate],
  );

  return {
    copyItemField: (itemId: ItemId, requestedField: RevealableField, copiedLabel: string) =>
      runCopy(() => vaultApiClient.copyItemField(itemId, requestedField), copiedLabel),
    copyTotpCode: (itemId: ItemId) => runCopy(() => vaultApiClient.copyTotpCode(itemId), translate("fields.totpCode")),
    copyTextSecret: (secretValue: string, copiedLabel: string) =>
      runCopy(() => vaultApiClient.copyTextSecret(secretValue), copiedLabel),
  };
}
