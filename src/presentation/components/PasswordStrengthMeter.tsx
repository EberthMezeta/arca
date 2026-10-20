import { useTranslation } from "react-i18next";

import type { PasswordStrength } from "../../domain/vaultTypes";

interface StrengthLevelAppearance {
  labelKey: string;
  barColorClassName: string;
  textColorClassName: string;
}

/** Apariencia por puntuación zxcvbn (0-4): tabla en lugar de if/else. */
const STRENGTH_LEVEL_APPEARANCES: Record<PasswordStrength["score"], StrengthLevelAppearance> = {
  0: { labelKey: "strength.veryWeak", barColorClassName: "bg-danger", textColorClassName: "text-danger" },
  1: { labelKey: "strength.weak", barColorClassName: "bg-danger", textColorClassName: "text-danger" },
  2: { labelKey: "strength.fair", barColorClassName: "bg-warning", textColorClassName: "text-warning" },
  3: { labelKey: "strength.strong", barColorClassName: "bg-success", textColorClassName: "text-success" },
  4: { labelKey: "strength.veryStrong", barColorClassName: "bg-success", textColorClassName: "text-success" },
};

const STRENGTH_SEGMENT_COUNT = 4;

export function PasswordStrengthMeter({ passwordStrength }: { passwordStrength: PasswordStrength | null }) {
  const { t: translate } = useTranslation();
  if (!passwordStrength) return <div className="h-5" />;

  const strengthAppearance = STRENGTH_LEVEL_APPEARANCES[passwordStrength.score];
  const filledSegmentCount = Math.max(1, passwordStrength.score);

  return (
    <div className="flex items-center gap-3" aria-live="polite">
      <div className="flex flex-1 gap-1" aria-hidden="true">
        {Array.from({ length: STRENGTH_SEGMENT_COUNT }, (_, segmentIndex) => (
          <span
            key={segmentIndex}
            className={`h-1.5 flex-1 rounded-full transition-colors ${segmentIndex < filledSegmentCount ? strengthAppearance.barColorClassName : "bg-surface-hover"}`}
          />
        ))}
      </div>
      <span className={`text-xs font-semibold ${strengthAppearance.textColorClassName}`}>
        {translate(strengthAppearance.labelKey)}
      </span>
    </div>
  );
}
