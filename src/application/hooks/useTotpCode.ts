import { useEffect, useState } from "react";

import type { ItemId, TotpCode } from "../../domain/vaultTypes";
import { vaultApiClient } from "../../infrastructure/vaultApiClient";

/** Cuenta atrás local cada segundo; solo pide un código nuevo al backend al cambiar de periodo. */
export function useTotpCode(itemId: ItemId, hasTotp: boolean): TotpCode | null {
  const [totpCode, setTotpCode] = useState<TotpCode | null>(null);

  useEffect(() => {
    if (!hasTotp) {
      setTotpCode(null);
      return;
    }
    let isStillMounted = true;
    let refreshTimeoutId = 0;

    const fetchCurrentCode = async () => {
      const freshCode = await vaultApiClient.getTotpCode(itemId).catch(() => null);
      if (!isStillMounted || !freshCode) return;
      setTotpCode(freshCode);
      refreshTimeoutId = window.setTimeout(fetchCurrentCode, freshCode.secondsUntilRefresh * 1000 + 50);
    };
    void fetchCurrentCode();

    const countdownIntervalId = window.setInterval(() => {
      setTotpCode((currentCode) =>
        currentCode ? { ...currentCode, secondsUntilRefresh: Math.max(0, currentCode.secondsUntilRefresh - 1) } : null,
      );
    }, 1000);

    return () => {
      isStillMounted = false;
      window.clearTimeout(refreshTimeoutId);
      window.clearInterval(countdownIntervalId);
    };
  }, [itemId, hasTotp]);

  return totpCode;
}
