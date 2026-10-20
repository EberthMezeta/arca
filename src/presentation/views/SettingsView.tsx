import { Download, KeyRound, Languages, Lock, Palette, ShieldAlert, Upload } from "lucide-react";
import { type FormEvent, type ReactNode, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { useAsyncData } from "../../application/hooks/useAsyncData";
import { useDebouncedValue } from "../../application/hooks/useDebouncedValue";
import { useErrorMessage } from "../../application/hooks/useErrorMessage";
import { THEME_PREFERENCES, type ThemePreference, useThemeStore } from "../../application/stores/themeStore";
import { useToastStore } from "../../application/stores/toastStore";
import { useVaultWorkspaceStore } from "../../application/stores/vaultWorkspaceStore";
import type { CommandError, ExportFormat, ImportFormat, PasswordStrength, VaultPreferences } from "../../domain/vaultTypes";
import { changeLanguage, SUPPORTED_LANGUAGES, type SupportedLanguage } from "../../i18n";
import { vaultApiClient } from "../../infrastructure/vaultApiClient";
import { PasswordStrengthMeter } from "../components/PasswordStrengthMeter";
import { Button } from "../components/primitives/Button";
import { FormField } from "../components/primitives/FormField";
import { PasswordInput } from "../components/primitives/PasswordInput";
import { SelectInput } from "../components/primitives/SelectInput";
import { ToggleSwitch } from "../components/primitives/ToggleSwitch";

function SettingsSection({ sectionIcon, sectionTitle, sectionDescription, children }: { sectionIcon: ReactNode; sectionTitle: string; sectionDescription?: string; children: ReactNode }) {
  return (
    <section className="rounded-2xl border border-border bg-surface p-6">
      <header className="mb-5 flex items-start gap-3">
        <span className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-surface-muted text-muted">{sectionIcon}</span>
        <div>
          <h2 className="font-semibold">{sectionTitle}</h2>
          {sectionDescription && <p className="mt-0.5 text-sm text-muted">{sectionDescription}</p>}
        </div>
      </header>
      <div className="flex flex-col gap-5">{children}</div>
    </section>
  );
}

function useSettingsFeedback() {
  const showToast = useToastStore((toastState) => toastState.showToast);
  const describeError = useErrorMessage();
  return {
    reportSuccess: (message: string) => showToast({ message, tone: "success" }),
    reportError: (commandError: unknown) => showToast({ message: describeError(commandError as CommandError), tone: "error" }),
    describeError,
  };
}

// ── Apariencia ───────────────────────────────────────────────────────────────

function AppearanceSettings() {
  const { t: translate, i18n } = useTranslation();
  const themePreference = useThemeStore((themeState) => themeState.themePreference);
  const changeThemePreference = useThemeStore((themeState) => themeState.changeThemePreference);

  const languageOptions = (Object.keys(SUPPORTED_LANGUAGES) as SupportedLanguage[]).map((languageCode) => ({
    optionValue: languageCode,
    optionLabel: SUPPORTED_LANGUAGES[languageCode].nativeName,
  }));
  const themeOptions = THEME_PREFERENCES.map((themeOption) => ({ optionValue: themeOption, optionLabel: translate(`settings.theme.${themeOption}`) }));

  return (
    <SettingsSection sectionIcon={<Palette className="size-4.5" />} sectionTitle={translate("settings.appearanceTitle")}>
      <div className="grid grid-cols-2 gap-4">
        <FormField fieldLabel={translate("settings.language")}>
          {(fieldControlId) => (
            <SelectInput
              id={fieldControlId}
              selectedValue={i18n.language as SupportedLanguage}
              selectOptions={languageOptions}
              onValueChange={changeLanguage}
            />
          )}
        </FormField>
        <FormField fieldLabel={translate("settings.themeLabel")}>
          {(fieldControlId) => (
            <SelectInput<ThemePreference> id={fieldControlId} selectedValue={themePreference} selectOptions={themeOptions} onValueChange={changeThemePreference} />
          )}
        </FormField>
      </div>
    </SettingsSection>
  );
}

// ── Seguridad ────────────────────────────────────────────────────────────────

/** Unidades de tiempo de mayor a menor: se usa la primera que cabe. */
const DURATION_UNITS: Array<[number, string]> = [
  [3600, "settings.hours"],
  [60, "settings.minutes"],
  [1, "settings.seconds"],
];

const AUTO_LOCK_SECONDS_CHOICES = [60, 300, 900, 1800, 3600, 14400];
const CLIPBOARD_CLEAR_SECONDS_CHOICES = [10, 30, 60, 120];
const PASSWORD_HISTORY_LIMIT_CHOICES = [0, 5, 10, 20, 50];

function SecuritySettings() {
  const { t: translate } = useTranslation();
  const { reportSuccess, reportError } = useSettingsFeedback();
  const { loadedData: loadedPreferences } = useAsyncData(() => vaultApiClient.getVaultPreferences(), []);
  const [vaultPreferences, setVaultPreferences] = useState<VaultPreferences | null>(null);

  useEffect(() => setVaultPreferences(loadedPreferences), [loadedPreferences]);
  if (!vaultPreferences) return null;

  const savePreferenceChange = async (preferenceChanges: Partial<VaultPreferences>) => {
    const previousPreferences = vaultPreferences;
    const updatedPreferences = { ...vaultPreferences, ...preferenceChanges };
    setVaultPreferences(updatedPreferences);
    try {
      await vaultApiClient.updateVaultPreferences(updatedPreferences);
      reportSuccess(translate("settings.saved"));
    } catch (commandError) {
      setVaultPreferences(previousPreferences);
      reportError(commandError);
    }
  };

  const formatDuration = (totalSeconds: number) => {
    const [unitSeconds, unitTranslationKey] = DURATION_UNITS.find(([unitSeconds]) => totalSeconds >= unitSeconds) ?? [1, "settings.seconds"];
    return translate(unitTranslationKey, { count: totalSeconds / unitSeconds });
  };
  const toSelectOptions = (choices: number[], formatChoice: (choice: number) => string) =>
    choices.map((choice) => ({ optionValue: String(choice), optionLabel: formatChoice(choice) }));

  return (
    <SettingsSection sectionIcon={<Lock className="size-4.5" />} sectionTitle={translate("settings.securityTitle")} sectionDescription={translate("settings.securityDescription")}>
      <div className="grid grid-cols-2 gap-4">
        <FormField fieldLabel={translate("settings.autoLock")}>
          {(fieldControlId) => (
            <SelectInput
              id={fieldControlId}
              selectedValue={String(vaultPreferences.autoLockAfterIdleSeconds)}
              selectOptions={toSelectOptions(AUTO_LOCK_SECONDS_CHOICES, formatDuration)}
              onValueChange={(selectedSeconds) => void savePreferenceChange({ autoLockAfterIdleSeconds: Number(selectedSeconds) })}
            />
          )}
        </FormField>
        <FormField fieldLabel={translate("settings.clipboardClear")}>
          {(fieldControlId) => (
            <SelectInput
              id={fieldControlId}
              selectedValue={String(vaultPreferences.clipboardClearAfterSeconds)}
              selectOptions={toSelectOptions(CLIPBOARD_CLEAR_SECONDS_CHOICES, formatDuration)}
              onValueChange={(selectedSeconds) => void savePreferenceChange({ clipboardClearAfterSeconds: Number(selectedSeconds) })}
            />
          )}
        </FormField>
        <FormField fieldLabel={translate("settings.historyLimit")}>
          {(fieldControlId) => (
            <SelectInput
              id={fieldControlId}
              selectedValue={String(vaultPreferences.passwordHistoryLimit)}
              selectOptions={toSelectOptions(PASSWORD_HISTORY_LIMIT_CHOICES, (choice) => translate("settings.historyEntries", { count: choice }))}
              onValueChange={(selectedLimit) => void savePreferenceChange({ passwordHistoryLimit: Number(selectedLimit) })}
            />
          )}
        </FormField>
      </div>
      <ToggleSwitch
        accessibleLabel={translate("settings.lockOnMinimize")}
        descriptionText={translate("settings.lockOnMinimizeHint")}
        isChecked={vaultPreferences.shouldLockOnWindowMinimize}
        onCheckedChange={(shouldLockOnWindowMinimize) => void savePreferenceChange({ shouldLockOnWindowMinimize })}
      />
    </SettingsSection>
  );
}

// ── Contraseña maestra ───────────────────────────────────────────────────────

function MasterPasswordSettings() {
  const { t: translate } = useTranslation();
  const { reportSuccess, describeError } = useSettingsFeedback();
  const [currentMasterPassword, setCurrentMasterPassword] = useState("");
  const [newMasterPassword, setNewMasterPassword] = useState("");
  const [confirmedMasterPassword, setConfirmedMasterPassword] = useState("");
  const [newPasswordStrength, setNewPasswordStrength] = useState<PasswordStrength | null>(null);
  const [changeErrorText, setChangeErrorText] = useState<string | null>(null);
  const [isChanging, setIsChanging] = useState(false);
  const debouncedNewPassword = useDebouncedValue(newMasterPassword, 200);

  useEffect(() => {
    if (!debouncedNewPassword) return setNewPasswordStrength(null);
    vaultApiClient.estimatePasswordStrength(debouncedNewPassword).then(setNewPasswordStrength).catch(() => setNewPasswordStrength(null));
  }, [debouncedNewPassword]);

  const doPasswordsMatch = newMasterPassword === confirmedMasterPassword;
  const canSubmit = currentMasterPassword.length > 0 && newMasterPassword.length > 0 && doPasswordsMatch;

  const changeMasterPassword = async (formEvent: FormEvent) => {
    formEvent.preventDefault();
    if (!canSubmit) return;
    setIsChanging(true);
    setChangeErrorText(null);
    try {
      await vaultApiClient.changeMasterPassword(currentMasterPassword, newMasterPassword);
      setCurrentMasterPassword("");
      setNewMasterPassword("");
      setConfirmedMasterPassword("");
      reportSuccess(translate("settings.masterPasswordChanged"));
    } catch (commandError) {
      setChangeErrorText(describeError(commandError as CommandError));
    } finally {
      setIsChanging(false);
    }
  };

  return (
    <SettingsSection sectionIcon={<KeyRound className="size-4.5" />} sectionTitle={translate("settings.masterPasswordTitle")} sectionDescription={translate("settings.masterPasswordDescription")}>
      <form onSubmit={changeMasterPassword} className="flex flex-col gap-4">
        <FormField fieldLabel={translate("settings.currentMasterPassword")}>
          {(fieldControlId) => <PasswordInput id={fieldControlId} value={currentMasterPassword} onChange={(changeEvent) => setCurrentMasterPassword(changeEvent.target.value)} />}
        </FormField>
        <FormField fieldLabel={translate("settings.newMasterPassword")}>
          {(fieldControlId) => (
            <div className="flex flex-col gap-2">
              <PasswordInput id={fieldControlId} value={newMasterPassword} onChange={(changeEvent) => setNewMasterPassword(changeEvent.target.value)} />
              <PasswordStrengthMeter passwordStrength={newPasswordStrength} />
            </div>
          )}
        </FormField>
        <FormField fieldLabel={translate("settings.confirmMasterPassword")} errorText={confirmedMasterPassword && !doPasswordsMatch ? translate("setup.passwordsDoNotMatch") : null}>
          {(fieldControlId) => <PasswordInput id={fieldControlId} value={confirmedMasterPassword} onChange={(changeEvent) => setConfirmedMasterPassword(changeEvent.target.value)} />}
        </FormField>
        {changeErrorText && (
          <p role="alert" className="text-sm text-danger">
            {changeErrorText}
          </p>
        )}
        <Button type="submit" buttonVariant="primary" className="self-start" isBusy={isChanging} disabled={!canSubmit}>
          {translate("settings.changeMasterPassword")}
        </Button>
      </form>
    </SettingsSection>
  );
}

// ── Importar / exportar ──────────────────────────────────────────────────────

interface TransferFormatDescription {
  labelKey: string;
  requiresFilePassword: boolean;
  isPlaintext: boolean;
}

const IMPORT_FORMAT_DESCRIPTIONS: Record<ImportFormat, TransferFormatDescription> = {
  bitwardenJson: { labelKey: "transfer.formats.bitwardenJson", requiresFilePassword: false, isPlaintext: true },
  bitwardenCsv: { labelKey: "transfer.formats.bitwardenCsv", requiresFilePassword: false, isPlaintext: true },
  browserCsv: { labelKey: "transfer.formats.browserCsv", requiresFilePassword: false, isPlaintext: true },
  encryptedVaultExport: { labelKey: "transfer.formats.encryptedVaultExport", requiresFilePassword: true, isPlaintext: false },
};

const EXPORT_FORMAT_DESCRIPTIONS: Record<ExportFormat, TransferFormatDescription> = {
  encryptedVaultExport: { labelKey: "transfer.formats.encryptedVaultExport", requiresFilePassword: true, isPlaintext: false },
  bitwardenJson: { labelKey: "transfer.formats.bitwardenJson", requiresFilePassword: false, isPlaintext: true },
  bitwardenCsv: { labelKey: "transfer.formats.bitwardenCsv", requiresFilePassword: false, isPlaintext: true },
};

function formatOptionsFrom<Format extends string>(formatDescriptions: Record<Format, TransferFormatDescription>, translate: (key: string) => string) {
  return (Object.keys(formatDescriptions) as Format[]).map((transferFormat) => ({
    optionValue: transferFormat,
    optionLabel: translate(formatDescriptions[transferFormat].labelKey),
  }));
}

export function ImportSettings() {
  const { t: translate } = useTranslation();
  const { reportSuccess, reportError } = useSettingsFeedback();
  const notifyVaultDataChanged = useVaultWorkspaceStore((workspaceState) => workspaceState.notifyVaultDataChanged);
  const [importFormat, setImportFormat] = useState<ImportFormat>("bitwardenJson");
  const [filePassword, setFilePassword] = useState("");
  const [isImporting, setIsImporting] = useState(false);
  const formatDescription = IMPORT_FORMAT_DESCRIPTIONS[importFormat];

  const importFile = async () => {
    setIsImporting(true);
    try {
      const importSummary = await vaultApiClient.importVaultFile(importFormat, formatDescription.requiresFilePassword ? filePassword : null, translate("transfer.chooseImportFile"));
      if (!importSummary) return;
      notifyVaultDataChanged();
      setFilePassword("");
      reportSuccess(translate("transfer.imported", { count: importSummary.importedItemCount, folderCount: importSummary.createdFolderCount }));
    } catch (commandError) {
      reportError(commandError);
    } finally {
      setIsImporting(false);
    }
  };

  return (
    <SettingsSection sectionIcon={<Upload className="size-4.5" />} sectionTitle={translate("transfer.importTitle")} sectionDescription={translate("transfer.importDescription")}>
      <FormField fieldLabel={translate("transfer.format")} hintText={translate(`transfer.importHints.${importFormat}`)}>
        {(fieldControlId) => (
          <SelectInput id={fieldControlId} selectedValue={importFormat} selectOptions={formatOptionsFrom(IMPORT_FORMAT_DESCRIPTIONS, translate)} onValueChange={setImportFormat} />
        )}
      </FormField>
      {formatDescription.requiresFilePassword && (
        <FormField fieldLabel={translate("transfer.filePassword")}>
          {(fieldControlId) => <PasswordInput id={fieldControlId} value={filePassword} onChange={(changeEvent) => setFilePassword(changeEvent.target.value)} />}
        </FormField>
      )}
      <Button
        buttonVariant="primary"
        className="self-start"
        leadingIcon={<Upload className="size-4" />}
        isBusy={isImporting}
        disabled={formatDescription.requiresFilePassword && !filePassword}
        onClick={importFile}
      >
        {translate("transfer.chooseAndImport")}
      </Button>
    </SettingsSection>
  );
}

function ExportSettings() {
  const { t: translate } = useTranslation();
  const { reportSuccess, reportError } = useSettingsFeedback();
  const [exportFormat, setExportFormat] = useState<ExportFormat>("encryptedVaultExport");
  const [filePassword, setFilePassword] = useState("");
  const [confirmedFilePassword, setConfirmedFilePassword] = useState("");
  const [hasAcknowledgedPlaintextRisk, setHasAcknowledgedPlaintextRisk] = useState(false);
  const [isExporting, setIsExporting] = useState(false);
  const formatDescription = EXPORT_FORMAT_DESCRIPTIONS[exportFormat];

  useEffect(() => setHasAcknowledgedPlaintextRisk(false), [exportFormat]);

  const isPasswordReady = !formatDescription.requiresFilePassword || (filePassword.length > 0 && filePassword === confirmedFilePassword);
  const isRiskAcknowledged = !formatDescription.isPlaintext || hasAcknowledgedPlaintextRisk;

  const exportFile = async () => {
    setIsExporting(true);
    try {
      const wasSaved = await vaultApiClient.exportVaultToFile(exportFormat, formatDescription.requiresFilePassword ? filePassword : null, translate("transfer.chooseExportFile"));
      if (!wasSaved) return;
      setFilePassword("");
      setConfirmedFilePassword("");
      reportSuccess(translate("transfer.exported"));
    } catch (commandError) {
      reportError(commandError);
    } finally {
      setIsExporting(false);
    }
  };

  return (
    <SettingsSection sectionIcon={<Download className="size-4.5" />} sectionTitle={translate("transfer.exportTitle")} sectionDescription={translate("transfer.exportDescription")}>
      <FormField fieldLabel={translate("transfer.format")}>
        {(fieldControlId) => (
          <SelectInput id={fieldControlId} selectedValue={exportFormat} selectOptions={formatOptionsFrom(EXPORT_FORMAT_DESCRIPTIONS, translate)} onValueChange={setExportFormat} />
        )}
      </FormField>
      {formatDescription.requiresFilePassword && (
        <div className="grid grid-cols-2 gap-4">
          <FormField fieldLabel={translate("transfer.filePassword")} hintText={translate("transfer.filePasswordHint")}>
            {(fieldControlId) => <PasswordInput id={fieldControlId} value={filePassword} onChange={(changeEvent) => setFilePassword(changeEvent.target.value)} />}
          </FormField>
          <FormField fieldLabel={translate("transfer.confirmFilePassword")} errorText={confirmedFilePassword && filePassword !== confirmedFilePassword ? translate("setup.passwordsDoNotMatch") : null}>
            {(fieldControlId) => <PasswordInput id={fieldControlId} value={confirmedFilePassword} onChange={(changeEvent) => setConfirmedFilePassword(changeEvent.target.value)} />}
          </FormField>
        </div>
      )}
      {formatDescription.isPlaintext && (
        <div className="flex gap-3 rounded-xl bg-danger-soft p-4 text-sm text-danger">
          <ShieldAlert className="size-5 shrink-0" />
          <label className="flex cursor-pointer flex-col gap-2">
            <span>{translate("transfer.plaintextWarning")}</span>
            <span className="flex items-center gap-2 font-medium">
              <input type="checkbox" className="accent-[var(--danger)]" checked={hasAcknowledgedPlaintextRisk} onChange={(changeEvent) => setHasAcknowledgedPlaintextRisk(changeEvent.target.checked)} />
              {translate("transfer.plaintextAcknowledge")}
            </span>
          </label>
        </div>
      )}
      <Button
        buttonVariant={formatDescription.isPlaintext ? "danger" : "primary"}
        className="self-start"
        leadingIcon={<Download className="size-4" />}
        isBusy={isExporting}
        disabled={!isPasswordReady || !isRiskAcknowledged}
        onClick={exportFile}
      >
        {translate("transfer.exportAndSave")}
      </Button>
    </SettingsSection>
  );
}

// ── Acerca de ────────────────────────────────────────────────────────────────

function AboutSettings() {
  const { t: translate } = useTranslation();
  const { loadedData: vaultLocation } = useAsyncData(() => vaultApiClient.getVaultLocation(), []);
  return (
    <SettingsSection sectionIcon={<Languages className="size-4.5" />} sectionTitle={translate("settings.aboutTitle")}>
      <dl className="grid grid-cols-[auto_1fr] gap-x-6 gap-y-2 text-sm">
        <dt className="text-muted">{translate("settings.vaultLocation")}</dt>
        <dd className="font-mono text-xs break-all" data-selectable>
          {vaultLocation}
        </dd>
        <dt className="text-muted">{translate("settings.encryption")}</dt>
        <dd>XChaCha20-Poly1305 · Argon2id (256 MiB, 3, 4) · HKDF-SHA256</dd>
      </dl>
      <p className="text-xs text-muted">{translate("settings.backupAdvice")}</p>
    </SettingsSection>
  );
}

/** Orden de las secciones: lista declarativa, fácil de reordenar o ampliar. */
const SETTINGS_SECTIONS = [AppearanceSettings, SecuritySettings, MasterPasswordSettings, ImportSettings, ExportSettings, AboutSettings];

export function SettingsView() {
  const { t: translate } = useTranslation();
  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto flex max-w-2xl flex-col gap-5 p-8">
        <h1 className="text-2xl font-semibold">{translate("navigation.settings")}</h1>
        {SETTINGS_SECTIONS.map((SettingsSectionComponent, sectionPosition) => (
          <SettingsSectionComponent key={sectionPosition} />
        ))}
      </div>
    </div>
  );
}
