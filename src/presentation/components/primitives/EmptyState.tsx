import type { ReactNode } from "react";

interface EmptyStateProps {
  illustrationIcon: ReactNode;
  headline: string;
  explanation?: string;
  callToAction?: ReactNode;
}

export function EmptyState({ illustrationIcon, headline, explanation, callToAction }: EmptyStateProps) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 p-8 text-center">
      <div className="flex size-14 items-center justify-center rounded-2xl bg-surface-muted text-muted">{illustrationIcon}</div>
      <h3 className="text-base font-semibold">{headline}</h3>
      {explanation && <p className="max-w-sm text-sm text-muted">{explanation}</p>}
      {callToAction && <div className="mt-2 flex flex-wrap justify-center gap-2">{callToAction}</div>}
    </div>
  );
}
