import { ArrowLeft, ArrowRight, KeyRound, Lock, ShieldCheck, TriangleAlert } from "lucide-react";
import { type ComponentType, type FormEvent, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { useDebouncedValue } from "../../application/hooks/useDebouncedValue";
import { useErrorMessage } from "../../application/hooks/useErrorMessage";
import { useSessionStore } from "../../application/stores/sessionStore";
import type { CommandError, MasterPasswordViolation, PasswordStrength } from "../../domain/vaultTypes";
import { vaultApiClient } from "../../infrastructure/vaultApiClient";
import { PasswordStrengthMeter } from "../components/PasswordStrengthMeter";
import { Button } from "../components/primitives/Button";
import { FormField } from "../components/primitives/FormField";
import { PasswordInput } from "../components/primitives/PasswordInput";

const MINIMUM_MASTER_PASSWORD_LENGTH = 12;
const MINIMUM_MASTER_PASSWORD_SCORE = 3;

interface SetupStepProps {
  goToNextStep: () => void;
  goToPreviousStep: () => void;
}

function WelcomeStep({ goToNextStep }: SetupStepProps) {
  const { t: translate } = useTranslation();
  const reassuranceMessages = [
    { messageIcon: <Lock className="size-4" />, messageKey: "setup.pointLocal" },
    { messageIcon: <ShieldCheck className="size-4" />, messageKey: "setup.pointEncryption" },
    { messageIcon: <KeyRound className="size-4" />, messageKey: "setup.pointMasterPassword" },
  ];
  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-2xl font-semibold">{translate("setup.welcomeTitle")}</h1>
        <p className="mt-2 text-sm text-muted">{translate("setup.welcomeText")}</p>
      </div>
      <ul className="flex flex-col gap-3">
        {reassuranceMessages.map(({ messageIcon, messageKey }) => (
          <li key={messageKey} className="flex items-start gap-3 text-sm">
            <span className="mt-0.5 text-accent">{messageIcon}</span>
            {translate(messageKey)}
          </li>
        ))}
      </ul>
      <Button buttonVariant="primary" className="self-end" onClick={goToNextStep}>
        {translate("setup.getStarted")} <ArrowRight className="size-4" />
      </Button>
    </div>
  );
}

function WarningStep({ goToNextStep, goToPreviousStep }: SetupStepProps) {
  const { t: translate } = useTranslation();
  const [hasUnderstood, setHasUnderstood] = useState(false);
  return (
    <div className="flex flex-col gap-6">
      <div className="flex gap-3 rounded-xl bg-warning-soft p-4 text-warning">
        <TriangleAlert className="size-5 shrink-0" />
        <div className="flex flex-col gap-1 text-sm">
          <strong>{translate("setup.noRecoveryTitle")}</strong>
          <span className="text-text">{translate("setup.noRecoveryText")}</span>
        </div>
      </div>
      <div>
        <h2 className="font-semibold">{translate("setup.tipsTitle")}</h2>
        <ul className="mt-2 list-disc pl-5 text-sm text-muted">
          <li>{translate("setup.tipPassphrase")}</li>
          <li>{translate("setup.tipUnique")}</li>
          <li>{translate("setup.tipWriteDown")}</li>
        </ul>
      </div>
      <label className="flex cursor-pointer items-center gap-2 text-sm font-medium">
        <input type="checkbox" className="accent-[var(--accent)]" checked={hasUnderstood} onChange={(changeEvent) => setHasUnderstood(changeEvent.target.checked)} />
        {translate("setup.understood")}
      </label>
      <div className="flex justify-between">
        <Button buttonVariant="ghost" leadingIcon={<ArrowLeft className="size-4" />} onClick={goToPreviousStep}>
          {translate("actions.back")}
        </Button>
        <Button buttonVariant="primary" disabled={!hasUnderstood} onClick={goToNextStep}>
          {translate("actions.continue")} <ArrowRight className="size-4" />
        </Button>
      </div>
    </div>
  );
}

function describeViolation(violation: MasterPasswordViolation, translate: (key: string, values?: Record<string, unknown>) => string) {
  const violationMessages: Record<MasterPasswordViolation["kind"], () => string> = {
    tooShort: () => translate("setup.violationTooShort", { count: MINIMUM_MASTER_PASSWORD_LENGTH }),
    tooWeak: () => translate("setup.violationTooWeak"),
  };
  return violationMessages[violation.kind]();
}

function CreatePasswordStep({ goToPreviousStep }: SetupStepProps) {
  const { t: translate } = useTranslation();
  const markVaultUnlocked = useSessionStore((sessionState) => sessionState.markVaultUnlocked);
  const describeError = useErrorMessage();
  const [masterPassword, setMasterPassword] = useState("");
  const [confirmedMasterPassword, setConfirmedMasterPassword] = useState("");
  const [passwordStrength, setPasswordStrength] = useState<PasswordStrength | null>(null);
  const [creationErrorText, setCreationErrorText] = useState<string | null>(null);
  const [isCreating, setIsCreating] = useState(false);
  const debouncedMasterPassword = useDebouncedValue(masterPassword, 200);

  useEffect(() => {
    if (!debouncedMasterPassword) return setPasswordStrength(null);
    vaultApiClient.estimatePasswordStrength(debouncedMasterPassword).then(setPasswordStrength).catch(() => setPasswordStrength(null));
  }, [debouncedMasterPassword]);

  const pendingViolations: MasterPasswordViolation[] = [
    ...(masterPassword.length < MINIMUM_MASTER_PASSWORD_LENGTH ? [{ kind: "tooShort" as const, minimumCharacterCount: MINIMUM_MASTER_PASSWORD_LENGTH }] : []),
    ...((passwordStrength?.score ?? 0) < MINIMUM_MASTER_PASSWORD_SCORE ? [{ kind: "tooWeak" as const, minimumStrengthScore: MINIMUM_MASTER_PASSWORD_SCORE }] : []),
  ];
  const doPasswordsMatch = masterPassword === confirmedMasterPassword;
  const canCreateVault = pendingViolations.length === 0 && doPasswordsMatch && confirmedMasterPassword.length > 0;

  const createVault = async (formEvent: FormEvent) => {
    formEvent.preventDefault();
    if (!canCreateVault) return;
    setIsCreating(true);
    setCreationErrorText(null);
    try {
      await vaultApiClient.createVault(masterPassword);
      markVaultUnlocked();
    } catch (commandError) {
      setCreationErrorText(describeError(commandError as CommandError));
      setIsCreating(false);
    }
  };

  return (
    <form onSubmit={createVault} className="flex flex-col gap-5">
      <div>
        <h2 className="text-xl font-semibold">{translate("setup.createTitle")}</h2>
        <p className="mt-1 text-sm text-muted">{translate("setup.createText")}</p>
      </div>
      <FormField fieldLabel={translate("setup.masterPassword")}>
        {(fieldControlId) => (
          <div className="flex flex-col gap-2">
            <PasswordInput id={fieldControlId} autoFocus value={masterPassword} onChange={(changeEvent) => setMasterPassword(changeEvent.target.value)} />
            <PasswordStrengthMeter passwordStrength={passwordStrength} />
          </div>
        )}
      </FormField>
      {masterPassword && pendingViolations.length > 0 && (
        <ul className="flex flex-col gap-1 text-xs text-muted">
          {pendingViolations.map((violation) => (
            <li key={violation.kind}>• {describeViolation(violation, translate)}</li>
          ))}
        </ul>
      )}
      <FormField fieldLabel={translate("setup.confirmMasterPassword")} errorText={confirmedMasterPassword && !doPasswordsMatch ? translate("setup.passwordsDoNotMatch") : null}>
        {(fieldControlId) => (
          <PasswordInput id={fieldControlId} value={confirmedMasterPassword} onChange={(changeEvent) => setConfirmedMasterPassword(changeEvent.target.value)} />
        )}
      </FormField>
      {creationErrorText && (
        <p role="alert" className="text-sm text-danger">
          {creationErrorText}
        </p>
      )}
      <div className="flex justify-between">
        <Button buttonVariant="ghost" leadingIcon={<ArrowLeft className="size-4" />} onClick={goToPreviousStep} disabled={isCreating}>
          {translate("actions.back")}
        </Button>
        <Button type="submit" buttonVariant="primary" isBusy={isCreating} disabled={!canCreateVault}>
          {translate(isCreating ? "setup.creating" : "setup.createVault")}
        </Button>
      </div>
    </form>
  );
}

/** Pasos del asistente en orden: añadir uno es añadirlo a la lista. */
const SETUP_STEPS: ComponentType<SetupStepProps>[] = [WelcomeStep, WarningStep, CreatePasswordStep];

export function SetupScreen() {
  const { t: translate } = useTranslation();
  const [currentStepPosition, setCurrentStepPosition] = useState(0);
  const CurrentStep = SETUP_STEPS[currentStepPosition] ?? WelcomeStep;

  return (
    <main className="flex h-full items-center justify-center p-6">
      <div className="w-full max-w-md">
        <div className="mb-6 flex items-center justify-between">
          <span className="flex size-10 items-center justify-center rounded-xl bg-accent text-accent-contrast">
            <ShieldCheck className="size-5" />
          </span>
          <ol className="flex gap-1.5" aria-label={translate("setup.progress", { current: currentStepPosition + 1, total: SETUP_STEPS.length })}>
            {SETUP_STEPS.map((_, stepPosition) => (
              <li key={stepPosition} className={`h-1.5 w-8 rounded-full ${stepPosition <= currentStepPosition ? "bg-accent" : "bg-surface-hover"}`} />
            ))}
          </ol>
        </div>
        <div className="rounded-2xl border border-border bg-surface p-7 shadow-sm">
          <CurrentStep
            goToNextStep={() => setCurrentStepPosition((stepPosition) => Math.min(SETUP_STEPS.length - 1, stepPosition + 1))}
            goToPreviousStep={() => setCurrentStepPosition((stepPosition) => Math.max(0, stepPosition - 1))}
          />
        </div>
      </div>
    </main>
  );
}
