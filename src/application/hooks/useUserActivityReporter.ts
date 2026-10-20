import { useEffect } from "react";

import { vaultApiClient } from "../../infrastructure/vaultApiClient";

const USER_ACTIVITY_EVENTS = ["keydown", "pointerdown", "pointermove", "wheel"] as const;
const MINIMUM_MILLISECONDS_BETWEEN_REPORTS = 15_000;

/** Avisa al backend de que hay actividad (retrasa el auto-bloqueo) sin saturarlo. */
export function useUserActivityReporter(): void {
  useEffect(() => {
    let lastReportTimestamp = 0;
    const reportActivity = () => {
      const now = Date.now();
      if (now - lastReportTimestamp < MINIMUM_MILLISECONDS_BETWEEN_REPORTS) return;
      lastReportTimestamp = now;
      vaultApiClient.registerUserActivity().catch(() => undefined);
    };

    USER_ACTIVITY_EVENTS.forEach((eventName) => window.addEventListener(eventName, reportActivity, { passive: true }));
    return () => USER_ACTIVITY_EVENTS.forEach((eventName) => window.removeEventListener(eventName, reportActivity));
  }, []);
}
