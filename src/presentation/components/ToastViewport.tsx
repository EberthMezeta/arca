import { CircleAlert, CircleCheck, Info, X } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { type ToastTone, useToastStore } from "../../application/stores/toastStore";

const TOAST_TONE_ICONS: Record<ToastTone, ReactNode> = {
  success: <CircleCheck className="size-4 text-success" />,
  info: <Info className="size-4 text-accent" />,
  error: <CircleAlert className="size-4 text-danger" />,
};

export function ToastViewport() {
  const { t: translate } = useTranslation();
  const visibleToasts = useToastStore((toastState) => toastState.visibleToasts);
  const dismissToast = useToastStore((toastState) => toastState.dismissToast);

  return (
    <div aria-live="polite" className="pointer-events-none fixed right-4 bottom-4 z-50 flex w-80 flex-col gap-2">
      {visibleToasts.map((visibleToast) => (
        <div
          key={visibleToast.toastId}
          role={visibleToast.tone === "error" ? "alert" : "status"}
          className="pointer-events-auto overflow-hidden rounded-xl border border-border bg-surface shadow-xl"
        >
          <div className="flex items-start gap-2.5 p-3">
            <span className="mt-0.5">{TOAST_TONE_ICONS[visibleToast.tone]}</span>
            <p className="flex-1 text-sm">{visibleToast.message}</p>
            {visibleToast.toastAction && (
              <button
                type="button"
                className="text-sm font-semibold text-accent hover:underline"
                onClick={() => {
                  visibleToast.toastAction?.runAction();
                  dismissToast(visibleToast.toastId);
                }}
              >
                {visibleToast.toastAction.actionLabel}
              </button>
            )}
            <button
              type="button"
              aria-label={translate("actions.close")}
              className="text-muted hover:text-text"
              onClick={() => dismissToast(visibleToast.toastId)}
            >
              <X className="size-4" />
            </button>
          </div>
          {visibleToast.countdownSeconds !== null && (
            <div
              className="countdown-bar h-0.5 bg-accent"
              style={{ animationDuration: `${visibleToast.countdownSeconds}s` }}
            />
          )}
        </div>
      ))}
    </div>
  );
}
