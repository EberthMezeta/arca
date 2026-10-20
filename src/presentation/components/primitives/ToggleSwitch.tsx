interface ToggleSwitchProps {
  isChecked: boolean;
  onCheckedChange: (isChecked: boolean) => void;
  accessibleLabel: string;
  descriptionText?: string;
}

export function ToggleSwitch({ isChecked, onCheckedChange, accessibleLabel, descriptionText }: ToggleSwitchProps) {
  return (
    <label className="flex cursor-pointer items-start justify-between gap-4">
      <span className="flex flex-col gap-0.5">
        <span className="text-sm font-medium">{accessibleLabel}</span>
        {descriptionText && <span className="text-xs text-muted">{descriptionText}</span>}
      </span>
      <button
        type="button"
        role="switch"
        aria-checked={isChecked}
        aria-label={accessibleLabel}
        onClick={() => onCheckedChange(!isChecked)}
        className={`relative mt-0.5 inline-flex h-5 w-9 shrink-0 rounded-full transition-colors ${isChecked ? "bg-accent" : "bg-surface-hover"}`}
      >
        <span
          className={`absolute top-0.5 size-4 rounded-full bg-white shadow transition-transform ${isChecked ? "translate-x-4.5" : "translate-x-0.5"}`}
        />
      </button>
    </label>
  );
}
