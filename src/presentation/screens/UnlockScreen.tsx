import { LockKeyhole, ShieldCheck } from "lucide-react";
import { type FormEvent, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { useErrorMessage } from "../../application/hooks/useErrorMessage";
import { useSessionStore } from "../../application/stores/sessionStore";
import type { CommandError, VaultLockReason } from "../../domain/vaultTypes";
import { vaultApiClient } from "../../infrastructure/vaultApiClient";
import { Button } from "../components/primitives/Button";
import { PasswordInput } from "../components/primitives/PasswordInput";

const LOCK_REASON_MESSAGE_KEYS: Record<VaultLockReason, string> = {
  idleTimeout: "unlock.lockedByInactivity",
  windowMinimized: "unlock.lockedByMinimize",
};

export function UnlockScreen() {
  const { t: translate } = useTranslation();
  const lastLockReason = useSessionStore((sessionState) => sessionState.lastLockReason);
  const markVaultUnlocked = useSessionStore((sessionState) => sessionState.markVaultUnlocked);
  const describeError = useErrorMessage();
  const [masterPassword, setMasterPassword] = useState("");
  const [unlockErrorText, setUnlockErrorText] = useState<string | null>(null);
  const [isUnlocking, setIsUnlocking] = useState(false);
  const [retryCountdownSeconds, setRetryCountdownSeconds] = useState(0);

  useEffect(() => {
    if (retryCountdownSeconds <= 0) return;
    const countdownTimeoutId = window.setTimeout(() => setRetryCountdownSeconds((secondsLeft) => secondsLeft - 1), 1000);
    return () => window.clearTimeout(countdownTimeoutId);
  }, [retryCountdownSeconds]);

  const unlockVault = async (formEvent: FormEvent) => {
    formEvent.preventDefault();
    if (!masterPassword || isUnlocking || retryCountdownSeconds > 0) return;
    setIsUnlocking(true);
    setUnlockErrorText(null);
    try {
      await vaultApiClient.unlockVault(masterPassword);
      setMasterPassword("");
      markVaultUnlocked();
    } catch (unknownError) {
      const commandError = unknownError as CommandError;
      setUnlockErrorText(describeError(commandError));
      setRetryCountdownSeconds(commandError.retryAfterSeconds ?? 0);
      setIsUnlocking(false);
    }
  };

  const isWaitingForRetry = retryCountdownSeconds > 0;

  return (
    <main className="flex h-full items-center justify-center p-6">
      <form onSubmit={unlockVault} className="flex w-full max-w-sm flex-col items-center gap-6">
        <span className="flex size-16 items-center justify-center rounded-2xl bg-accent text-accent-contrast shadow-lg">
          <ShieldCheck className="size-8" />
        </span>
        <div className="text-center">
          <h1 className="text-2xl font-semibold">{translate("unlock.title")}</h1>
          <p className="mt-1 text-sm text-muted">{translate(lastLockReason ? LOCK_REASON_MESSAGE_KEYS[lastLockReason] : "unlock.subtitle")}</p>
        </div>
        <div className="flex w-full flex-col gap-3">
          <label htmlFor="master-password" className="sr-only">
            {translate("setup.masterPassword")}
          </label>
          <PasswordInput
            id="master-password"
            autoFocus
            placeholder={translate("unlock.placeholder")}
            value={masterPassword}
            disabled={isUnlocking}
            aria-invalid={unlockErrorText !== null}
            aria-describedby={unlockErrorText ? "unlock-error" : undefined}
            onChange={(changeEvent) => setMasterPassword(changeEvent.target.value)}
            className="h-11 text-base"
          />
          {unlockErrorText && (
            <p id="unlock-error" role="alert" className="text-center text-sm text-danger">
              {isWaitingForRetry ? translate("errors.unlockTemporarilyBlocked", { retryAfterSeconds: retryCountdownSeconds }) : unlockErrorText}
            </p>
          )}
          <Button
            type="submit"
            buttonVariant="primary"
            className="h-11 text-base"
            isBusy={isUnlocking}
            disabled={!masterPassword || isWaitingForRetry}
            leadingIcon={<LockKeyhole className="size-4" />}
          >
            {translate(isUnlocking ? "unlock.unlocking" : "unlock.submit")}
          </Button>
        </div>
      </form>
    </main>
  );
}
