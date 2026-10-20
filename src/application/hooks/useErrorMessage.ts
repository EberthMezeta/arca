import { useCallback } from "react";
import { useTranslation } from "react-i18next";

import type { CommandError } from "../../domain/vaultTypes";

type Translate = ReturnType<typeof useTranslation>["t"];

/** Errores cuyo mensaje depende de un código de detalle; el resto usa `errors.<errorCode>`. */
const DETAILED_ERROR_TRANSLATORS: Partial<Record<string, (commandError: CommandError, translate: Translate) => string>> = {
  invalidInput: (commandError, translate) =>
    translate(`errors.invalidInputReasons.${commandError.technicalDetail}`, { defaultValue: translate("errors.invalidInput") }),
};

/** Traduce un `CommandError` a un mensaje legible para la persona usuaria. */
export function useErrorMessage(): (commandError: CommandError) => string {
  const { t: translate } = useTranslation();
  return useCallback(
    (commandError: CommandError) => {
      const detailedTranslator = DETAILED_ERROR_TRANSLATORS[commandError.errorCode];
      if (detailedTranslator) return detailedTranslator(commandError, translate);
      return translate(`errors.${commandError.errorCode}`, {
        defaultValue: translate("errors.internal"),
        retryAfterSeconds: commandError.retryAfterSeconds,
        technicalDetail: commandError.technicalDetail,
      });
    },
    [translate],
  );
}
