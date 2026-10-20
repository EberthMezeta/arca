import { CalendarClock, Copy, ShieldAlert, ShieldCheck, TriangleAlert } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { useAsyncData } from "../../application/hooks/useAsyncData";
import { useVaultWorkspaceStore } from "../../application/stores/vaultWorkspaceStore";
import type { PasswordHealthFinding, PasswordHealthIssue } from "../../domain/vaultTypes";
import { vaultApiClient } from "../../infrastructure/vaultApiClient";
import { Button } from "../components/primitives/Button";
import { EmptyState } from "../components/primitives/EmptyState";
import { Spinner } from "../components/primitives/Spinner";

type IssueKind = PasswordHealthIssue["issueKind"];

interface IssueKindPresentation {
  issueIcon: ReactNode;
  titleKey: string;
  explanationKey: string;
  toneClassName: string;
  describeFinding: (issue: PasswordHealthIssue) => Record<string, number>;
}

/** Cómo se presenta cada tipo de problema (tabla en vez de condicionales). */
const ISSUE_KIND_PRESENTATIONS: Record<IssueKind, IssueKindPresentation> = {
  weakPassword: {
    issueIcon: <TriangleAlert className="size-5" />,
    titleKey: "health.weakTitle",
    explanationKey: "health.weakExplanation",
    toneClassName: "bg-danger-soft text-danger",
    describeFinding: () => ({}),
  },
  reusedPassword: {
    issueIcon: <Copy className="size-5" />,
    titleKey: "health.reusedTitle",
    explanationKey: "health.reusedExplanation",
    toneClassName: "bg-warning-soft text-warning",
    describeFinding: (issue) => ({ reuseCount: issue.issueKind === "reusedPassword" ? issue.reuseCount : 0 }),
  },
  oldPassword: {
    issueIcon: <CalendarClock className="size-5" />,
    titleKey: "health.oldTitle",
    explanationKey: "health.oldExplanation",
    toneClassName: "bg-accent-soft text-accent",
    describeFinding: (issue) => ({ ageInDays: issue.issueKind === "oldPassword" ? issue.ageInDays : 0 }),
  },
};

const FINDING_DETAIL_KEYS: Record<IssueKind, string> = {
  weakPassword: "health.weakDetail",
  reusedPassword: "health.reusedDetail",
  oldPassword: "health.oldDetail",
};

/** Umbrales de color de la puntuación global, de mayor a menor. */
const SCORE_TONE_THRESHOLDS: Array<[number, string]> = [
  [80, "text-success"],
  [50, "text-warning"],
];

function groupFindingsByIssueKind(healthFindings: PasswordHealthFinding[]): Record<IssueKind, PasswordHealthFinding[]> {
  const groupedFindings: Record<IssueKind, PasswordHealthFinding[]> = { weakPassword: [], reusedPassword: [], oldPassword: [] };
  healthFindings.forEach((healthFinding) => groupedFindings[healthFinding.issue.issueKind].push(healthFinding));
  return groupedFindings;
}

export function PasswordHealthView() {
  const { t: translate } = useTranslation();
  const vaultDataRevision = useVaultWorkspaceStore((workspaceState) => workspaceState.vaultDataRevision);
  const startEditingItem = useVaultWorkspaceStore((workspaceState) => workspaceState.startEditingItem);
  const { loadedData: healthReport, isLoading } = useAsyncData(() => vaultApiClient.getPasswordHealthReport(), [vaultDataRevision]);

  if (isLoading && !healthReport) {
    return (
      <div className="flex h-full items-center justify-center text-muted">
        <Spinner />
      </div>
    );
  }
  if (!healthReport || healthReport.analyzedLoginCount === 0) {
    return <EmptyState illustrationIcon={<ShieldCheck className="size-7" />} headline={translate("health.noLoginsTitle")} explanation={translate("health.noLoginsText")} />;
  }

  const groupedFindings = groupFindingsByIssueKind(healthReport.findings);
  const issueKinds = Object.keys(ISSUE_KIND_PRESENTATIONS) as IssueKind[];
  const scoreToneClassName =
    SCORE_TONE_THRESHOLDS.find(([minimumScore]) => healthReport.overallHealthScore >= minimumScore)?.[1] ?? "text-danger";

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto flex max-w-3xl flex-col gap-6 p-8">
        <header className="flex items-center gap-6 rounded-2xl border border-border bg-surface p-6">
          <div className={`text-5xl font-bold tabular-nums ${scoreToneClassName}`}>{healthReport.overallHealthScore}</div>
          <div>
            <h1 className="text-xl font-semibold">{translate("health.title")}</h1>
            <p className="mt-1 text-sm text-muted">
              {translate("health.summary", { healthyCount: healthReport.healthyLoginCount, totalCount: healthReport.analyzedLoginCount })}
            </p>
          </div>
        </header>

        <div className="grid grid-cols-3 gap-3">
          {issueKinds.map((issueKind) => (
            <div key={issueKind} className="rounded-xl border border-border bg-surface p-4">
              <span className={`flex size-9 items-center justify-center rounded-lg ${ISSUE_KIND_PRESENTATIONS[issueKind].toneClassName}`}>
                {ISSUE_KIND_PRESENTATIONS[issueKind].issueIcon}
              </span>
              <div className="mt-3 text-2xl font-semibold tabular-nums">{groupedFindings[issueKind].length}</div>
              <div className="text-sm text-muted">{translate(ISSUE_KIND_PRESENTATIONS[issueKind].titleKey)}</div>
            </div>
          ))}
        </div>

        {healthReport.findings.length === 0 && (
          <EmptyState illustrationIcon={<ShieldCheck className="size-7 text-success" />} headline={translate("health.allGoodTitle")} explanation={translate("health.allGoodText")} />
        )}

        {issueKinds
          .filter((issueKind) => groupedFindings[issueKind].length > 0)
          .map((issueKind) => (
            <section key={issueKind} className="flex flex-col gap-2">
              <div>
                <h2 className="flex items-center gap-2 font-semibold">
                  <ShieldAlert className="size-4 text-muted" /> {translate(ISSUE_KIND_PRESENTATIONS[issueKind].titleKey)}
                </h2>
                <p className="text-sm text-muted">{translate(ISSUE_KIND_PRESENTATIONS[issueKind].explanationKey)}</p>
              </div>
              <ul className="divide-y divide-border overflow-hidden rounded-xl border border-border bg-surface">
                {groupedFindings[issueKind].map((healthFinding) => (
                  <li key={`${issueKind}-${healthFinding.itemId}`} className="flex items-center gap-3 px-4 py-3">
                    <div className="min-w-0 flex-1">
                      <div className="truncate font-medium">{healthFinding.itemTitle}</div>
                      <div className="text-xs text-muted">
                        {translate(FINDING_DETAIL_KEYS[issueKind], ISSUE_KIND_PRESENTATIONS[issueKind].describeFinding(healthFinding.issue))}
                      </div>
                    </div>
                    <Button onClick={() => startEditingItem(healthFinding.itemId)}>{translate("health.changeNow")}</Button>
                  </li>
                ))}
              </ul>
            </section>
          ))}
      </div>
    </div>
  );
}
