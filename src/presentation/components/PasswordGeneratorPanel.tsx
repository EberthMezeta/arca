import { Check, Copy, RefreshCw } from "lucide-react";
import { type ComponentType, useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { useSecretClipboard } from "../../application/hooks/useSecretClipboard";
import type { CharacterSet, PasswordGenerationOptions, PasswordStrength } from "../../domain/vaultTypes";
import { readLocalPreference, writeLocalPreference } from "../../infrastructure/localPreferenceStorage";
import { vaultApiClient } from "../../infrastructure/vaultApiClient";
import { Button } from "./primitives/Button";
import { IconButton } from "./primitives/IconButton";
import { TextInput } from "./primitives/TextInput";
import { ToggleSwitch } from "./primitives/ToggleSwitch";
import { PasswordStrengthMeter } from "./PasswordStrengthMeter";

type GenerationStrategy = PasswordGenerationOptions["strategy"];
type OptionsOf<Strategy extends GenerationStrategy> = Extract<PasswordGenerationOptions, { strategy: Strategy }>;

const DEFAULT_OPTIONS_BY_STRATEGY: { [Strategy in GenerationStrategy]: OptionsOf<Strategy> } = {
  randomCharacters: {
    strategy: "randomCharacters",
    passwordLength: 20,
    characterSets: ["uppercase", "lowercase", "digits", "symbols"],
    shouldExcludeAmbiguousCharacters: false,
  },
  passphrase: {
    strategy: "passphrase",
    wordCount: 5,
    wordSeparator: "-",
    shouldCapitalizeWords: true,
    shouldAppendNumber: true,
  },
};

const GENERATOR_OPTIONS_PREFERENCE_KEY = "arca.generatorOptions";

/** Las opciones del generador no son secretas: se recuerdan entre sesiones. */
function readStoredGeneratorOptions(): typeof DEFAULT_OPTIONS_BY_STRATEGY & { activeStrategy: GenerationStrategy } {
  const fallbackOptions = { ...DEFAULT_OPTIONS_BY_STRATEGY, activeStrategy: "randomCharacters" as GenerationStrategy };
  try {
    const storedOptions = readLocalPreference(GENERATOR_OPTIONS_PREFERENCE_KEY);
    return storedOptions ? { ...fallbackOptions, ...JSON.parse(storedOptions) } : fallbackOptions;
  } catch {
    return fallbackOptions;
  }
}

interface StrategyOptionsEditorProps<Strategy extends GenerationStrategy> {
  generationOptions: OptionsOf<Strategy>;
  onOptionsChange: (generationOptions: OptionsOf<Strategy>) => void;
}

const CHARACTER_SET_LABEL_KEYS: Record<CharacterSet, string> = {
  uppercase: "generator.uppercase",
  lowercase: "generator.lowercase",
  digits: "generator.digits",
  symbols: "generator.symbols",
};

function RandomCharactersOptionsEditor({ generationOptions, onOptionsChange }: StrategyOptionsEditorProps<"randomCharacters">) {
  const { t: translate } = useTranslation();
  const toggleCharacterSet = (characterSet: CharacterSet) => {
    const isEnabled = generationOptions.characterSets.includes(characterSet);
    const updatedCharacterSets = isEnabled
      ? generationOptions.characterSets.filter((enabledSet) => enabledSet !== characterSet)
      : [...generationOptions.characterSets, characterSet];
    if (updatedCharacterSets.length === 0) return;
    onOptionsChange({ ...generationOptions, characterSets: updatedCharacterSets });
  };

  return (
    <div className="flex flex-col gap-4">
      <LengthSlider
        sliderLabel={translate("generator.length")}
        currentValue={generationOptions.passwordLength}
        minimumValue={8}
        maximumValue={64}
        onValueChange={(passwordLength) => onOptionsChange({ ...generationOptions, passwordLength })}
      />
      <div className="grid grid-cols-2 gap-2">
        {(Object.keys(CHARACTER_SET_LABEL_KEYS) as CharacterSet[]).map((characterSet) => (
          <label key={characterSet} className="flex cursor-pointer items-center gap-2 rounded-lg border border-border px-3 py-2 text-sm hover:bg-surface-hover">
            <input
              type="checkbox"
              className="accent-[var(--accent)]"
              checked={generationOptions.characterSets.includes(characterSet)}
              onChange={() => toggleCharacterSet(characterSet)}
            />
            {translate(CHARACTER_SET_LABEL_KEYS[characterSet])}
          </label>
        ))}
      </div>
      <ToggleSwitch
        accessibleLabel={translate("generator.excludeAmbiguous")}
        descriptionText={translate("generator.excludeAmbiguousHint")}
        isChecked={generationOptions.shouldExcludeAmbiguousCharacters}
        onCheckedChange={(shouldExcludeAmbiguousCharacters) => onOptionsChange({ ...generationOptions, shouldExcludeAmbiguousCharacters })}
      />
    </div>
  );
}

function PassphraseOptionsEditor({ generationOptions, onOptionsChange }: StrategyOptionsEditorProps<"passphrase">) {
  const { t: translate } = useTranslation();
  return (
    <div className="flex flex-col gap-4">
      <LengthSlider
        sliderLabel={translate("generator.wordCount")}
        currentValue={generationOptions.wordCount}
        minimumValue={3}
        maximumValue={12}
        onValueChange={(wordCount) => onOptionsChange({ ...generationOptions, wordCount })}
      />
      <label className="flex items-center justify-between gap-4 text-sm font-medium">
        {translate("generator.separator")}
        <TextInput
          className="w-16 text-center font-mono"
          maxLength={3}
          value={generationOptions.wordSeparator}
          onChange={(changeEvent) => onOptionsChange({ ...generationOptions, wordSeparator: changeEvent.target.value })}
        />
      </label>
      <ToggleSwitch
        accessibleLabel={translate("generator.capitalize")}
        isChecked={generationOptions.shouldCapitalizeWords}
        onCheckedChange={(shouldCapitalizeWords) => onOptionsChange({ ...generationOptions, shouldCapitalizeWords })}
      />
      <ToggleSwitch
        accessibleLabel={translate("generator.appendNumber")}
        isChecked={generationOptions.shouldAppendNumber}
        onCheckedChange={(shouldAppendNumber) => onOptionsChange({ ...generationOptions, shouldAppendNumber })}
      />
    </div>
  );
}

interface LengthSliderProps {
  sliderLabel: string;
  currentValue: number;
  minimumValue: number;
  maximumValue: number;
  onValueChange: (selectedValue: number) => void;
}

function LengthSlider({ sliderLabel, currentValue, minimumValue, maximumValue, onValueChange }: LengthSliderProps) {
  return (
    <label className="flex flex-col gap-2 text-sm font-medium">
      <span className="flex justify-between">
        {sliderLabel}
        <span className="font-mono text-accent tabular-nums">{currentValue}</span>
      </span>
      <input
        type="range"
        min={minimumValue}
        max={maximumValue}
        value={currentValue}
        className="accent-[var(--accent)]"
        onChange={(changeEvent) => onValueChange(Number(changeEvent.target.value))}
      />
    </label>
  );
}

/** Registro de estrategias: añadir una nueva solo requiere una entrada aquí. */
const STRATEGY_EDITORS: { [Strategy in GenerationStrategy]: { labelKey: string; OptionsEditor: ComponentType<StrategyOptionsEditorProps<Strategy>> } } = {
  randomCharacters: { labelKey: "generator.randomCharacters", OptionsEditor: RandomCharactersOptionsEditor },
  passphrase: { labelKey: "generator.passphrase", OptionsEditor: PassphraseOptionsEditor },
};

interface PasswordGeneratorPanelProps {
  onUseGeneratedPassword?: (generatedPassword: string) => void;
}

export function PasswordGeneratorPanel({ onUseGeneratedPassword }: PasswordGeneratorPanelProps) {
  const { t: translate } = useTranslation();
  const { copyTextSecret } = useSecretClipboard();
  const [storedOptions, setStoredOptions] = useState(readStoredGeneratorOptions);
  const [generatedPassword, setGeneratedPassword] = useState("");
  const [passwordStrength, setPasswordStrength] = useState<PasswordStrength | null>(null);

  const activeStrategy = storedOptions.activeStrategy;
  const activeOptions = storedOptions[activeStrategy];

  const regeneratePassword = useCallback(async () => {
    const freshPassword = await vaultApiClient.generatePassword(activeOptions).catch(() => "");
    setGeneratedPassword(freshPassword);
    setPasswordStrength(freshPassword ? await vaultApiClient.estimatePasswordStrength(freshPassword).catch(() => null) : null);
  }, [activeOptions]);

  useEffect(() => {
    void regeneratePassword();
    writeLocalPreference(GENERATOR_OPTIONS_PREFERENCE_KEY, JSON.stringify(storedOptions));
  }, [regeneratePassword, storedOptions]);

  const updateActiveOptions = (updatedOptions: PasswordGenerationOptions) =>
    setStoredOptions((currentOptions) => ({ ...currentOptions, [updatedOptions.strategy]: updatedOptions }));

  const { OptionsEditor } = STRATEGY_EDITORS[activeStrategy] as {
    OptionsEditor: ComponentType<StrategyOptionsEditorProps<typeof activeStrategy>>;
  };

  return (
    <div className="flex flex-col gap-5">
      <div className="rounded-xl border border-border bg-surface-muted p-4">
        <output
          data-selectable
          aria-label={translate("generator.generatedPassword")}
          className="block min-h-12 font-mono text-lg leading-relaxed break-all"
        >
          {generatedPassword}
        </output>
        <div className="mt-3 flex items-center gap-2">
          <div className="flex-1">
            <PasswordStrengthMeter passwordStrength={passwordStrength} />
          </div>
          <IconButton accessibleLabel={translate("generator.regenerate")} icon={<RefreshCw className="size-4" />} onClick={regeneratePassword} />
          <IconButton
            accessibleLabel={translate("actions.copy")}
            icon={<Copy className="size-4" />}
            onClick={() => copyTextSecret(generatedPassword, translate("fields.password"))}
          />
        </div>
      </div>

      <div role="tablist" className="grid grid-cols-2 gap-1 rounded-lg bg-surface-muted p-1">
        {(Object.keys(STRATEGY_EDITORS) as GenerationStrategy[]).map((generationStrategy) => (
          <button
            key={generationStrategy}
            type="button"
            role="tab"
            aria-selected={generationStrategy === activeStrategy}
            onClick={() => setStoredOptions((currentOptions) => ({ ...currentOptions, activeStrategy: generationStrategy }))}
            className={`rounded-md py-1.5 text-sm font-medium transition-colors ${generationStrategy === activeStrategy ? "bg-surface shadow-sm" : "text-muted hover:text-text"}`}
          >
            {translate(STRATEGY_EDITORS[generationStrategy].labelKey)}
          </button>
        ))}
      </div>

      <OptionsEditor generationOptions={activeOptions} onOptionsChange={updateActiveOptions} />

      {onUseGeneratedPassword && (
        <Button buttonVariant="primary" leadingIcon={<Check className="size-4" />} onClick={() => onUseGeneratedPassword(generatedPassword)}>
          {translate("generator.useThisPassword")}
        </Button>
      )}
    </div>
  );
}
