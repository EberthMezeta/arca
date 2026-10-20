import { Copy, Eye, EyeOff } from "lucide-react";
import { type ReactNode, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { useSecretClipboard } from "../../application/hooks/useSecretClipboard";
import { useTotpCode } from "../../application/hooks/useTotpCode";
import type { ItemId, RevealableField } from "../../domain/vaultTypes";
import { vaultApiClient } from "../../infrastructure/vaultApiClient";
import { IconButton } from "./primitives/IconButton";

function DetailRowFrame({ rowLabel, children, rowActions }: { rowLabel: string; children: ReactNode; rowActions?: ReactNode }) {
  return (
    <div className="group flex items-center gap-3 px-4 py-2.5">
      <div className="min-w-0 flex-1">
        <div className="text-xs text-muted">{rowLabel}</div>
        <div className="truncate text-sm" data-selectable>
          {children}
        </div>
      </div>
      <div className="flex shrink-0 gap-0.5 opacity-70 group-focus-within:opacity-100 group-hover:opacity-100">{rowActions}</div>
    </div>
  );
}

interface PlainDetailRowProps {
  rowLabel: string;
  displayedValue: string;
  itemId: ItemId;
  copyableField?: RevealableField;
}

export function PlainDetailRow({ rowLabel, displayedValue, itemId, copyableField }: PlainDetailRowProps) {
  const { t: translate } = useTranslation();
  const { copyItemField, copyTextSecret } = useSecretClipboard();
  const copyDisplayedValue = () =>
    copyableField ? copyItemField(itemId, copyableField, rowLabel) : copyTextSecret(displayedValue, rowLabel);

  return (
    <DetailRowFrame
      rowLabel={rowLabel}
      rowActions={
        <IconButton accessibleLabel={translate("actions.copy")} icon={<Copy className="size-4" />} onClick={copyDisplayedValue} />
      }
    >
      {displayedValue}
    </DetailRowFrame>
  );
}

interface SecretDetailRowProps {
  rowLabel: string;
  itemId: ItemId;
  revealableField: RevealableField;
  maskedValue: string;
}

/** Muestra el secreto enmascarado; solo lo pide al backend al pulsar "mostrar". */
export function SecretDetailRow({ rowLabel, itemId, revealableField, maskedValue }: SecretDetailRowProps) {
  const { t: translate } = useTranslation();
  const { copyItemField } = useSecretClipboard();
  const [revealedValue, setRevealedValue] = useState<string | null>(null);

  useEffect(() => setRevealedValue(null), [itemId]);

  const toggleReveal = async () => {
    if (revealedValue !== null) {
      setRevealedValue(null);
      return;
    }
    setRevealedValue(await vaultApiClient.revealItemField(itemId, revealableField).catch(() => null));
  };

  return (
    <DetailRowFrame
      rowLabel={rowLabel}
      rowActions={
        <>
          <IconButton
            accessibleLabel={translate(revealedValue === null ? "actions.show" : "actions.hide")}
            icon={revealedValue === null ? <Eye className="size-4" /> : <EyeOff className="size-4" />}
            onClick={toggleReveal}
          />
          <IconButton
            accessibleLabel={translate("actions.copy")}
            icon={<Copy className="size-4" />}
            onClick={() => copyItemField(itemId, revealableField, rowLabel)}
          />
        </>
      }
    >
      <span className="font-mono tracking-wide">{revealedValue ?? maskedValue}</span>
    </DetailRowFrame>
  );
}

const TOTP_RING_RADIUS = 9;
const TOTP_RING_CIRCUMFERENCE = 2 * Math.PI * TOTP_RING_RADIUS;

export function TotpDetailRow({ itemId }: { itemId: ItemId }) {
  const { t: translate } = useTranslation();
  const { copyTotpCode } = useSecretClipboard();
  const totpCode = useTotpCode(itemId, true);
  if (!totpCode) return null;

  const remainingFraction = totpCode.secondsUntilRefresh / totpCode.periodSeconds;
  const isAboutToExpire = totpCode.secondsUntilRefresh <= 5;
  const formattedCode = totpCode.currentCode.replace(/^(\d{3})(\d+)$/, "$1 $2");

  return (
    <DetailRowFrame
      rowLabel={translate("fields.totpCode")}
      rowActions={
        <IconButton accessibleLabel={translate("actions.copy")} icon={<Copy className="size-4" />} onClick={() => copyTotpCode(itemId)} />
      }
    >
      <span className="flex items-center gap-2.5">
        <span className={`font-mono text-base font-semibold tracking-widest ${isAboutToExpire ? "text-danger" : "text-accent"}`}>
          {formattedCode}
        </span>
        <svg viewBox="0 0 24 24" className="size-5 -rotate-90" aria-label={translate("totp.secondsLeft", { secondsLeft: totpCode.secondsUntilRefresh })}>
          <circle cx="12" cy="12" r={TOTP_RING_RADIUS} fill="none" strokeWidth="3" className="stroke-surface-hover" />
          <circle
            cx="12"
            cy="12"
            r={TOTP_RING_RADIUS}
            fill="none"
            strokeWidth="3"
            strokeLinecap="round"
            strokeDasharray={TOTP_RING_CIRCUMFERENCE}
            strokeDashoffset={TOTP_RING_CIRCUMFERENCE * (1 - remainingFraction)}
            className={`transition-[stroke-dashoffset] duration-1000 ease-linear ${isAboutToExpire ? "stroke-danger" : "stroke-accent"}`}
          />
        </svg>
        <span className="text-xs text-muted tabular-nums">{totpCode.secondsUntilRefresh}s</span>
      </span>
    </DetailRowFrame>
  );
}
