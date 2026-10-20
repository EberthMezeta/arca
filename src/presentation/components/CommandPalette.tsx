import * as Dialog from "@radix-ui/react-dialog";
import { Lock, Search, Settings, ShieldCheck, Wand2 } from "lucide-react";
import { type KeyboardEvent, type ReactNode, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { useAsyncData } from "../../application/hooks/useAsyncData";
import { useSessionStore } from "../../application/stores/sessionStore";
import { DEFAULT_ITEM_FILTER, useVaultWorkspaceStore } from "../../application/stores/vaultWorkspaceStore";
import { vaultApiClient } from "../../infrastructure/vaultApiClient";
import { ITEM_CONTENT_TYPE_REGISTRY, ITEM_CONTENT_TYPES, ItemTypeBadge } from "../registries/itemContentTypeRegistry";

interface PaletteCommand {
  commandKey: string;
  commandLabel: string;
  commandIcon: ReactNode;
  commandHint?: string;
  runCommand: () => void;
}

const MAXIMUM_ITEM_RESULTS = 6;

export function CommandPalette({ isOpen, onOpenChange }: { isOpen: boolean; onOpenChange: (isOpen: boolean) => void }) {
  const { t: translate } = useTranslation();
  const [queryText, setQueryText] = useState("");
  const [highlightedPosition, setHighlightedPosition] = useState(0);
  const startCreatingItem = useVaultWorkspaceStore((workspaceState) => workspaceState.startCreatingItem);
  const openSection = useVaultWorkspaceStore((workspaceState) => workspaceState.openSection);
  const applyNavigationFilter = useVaultWorkspaceStore((workspaceState) => workspaceState.applyNavigationFilter);
  const selectItem = useVaultWorkspaceStore((workspaceState) => workspaceState.selectItem);
  const lockVaultNow = useSessionStore((sessionState) => sessionState.lockVaultNow);

  const { loadedData: matchingItems } = useAsyncData(
    () => (isOpen && queryText.trim() ? vaultApiClient.searchVaultItems({ ...DEFAULT_ITEM_FILTER, searchText: queryText }) : Promise.resolve([])),
    [isOpen, queryText],
  );

  useEffect(() => {
    if (!isOpen) setQueryText("");
  }, [isOpen]);

  const staticCommands: PaletteCommand[] = useMemo(
    () => [
      ...ITEM_CONTENT_TYPES.map((contentType) => ({
        commandKey: `new-${contentType}`,
        commandLabel: translate("palette.newItem", { itemType: translate(ITEM_CONTENT_TYPE_REGISTRY[contentType].singularLabelKey) }),
        commandIcon: <ItemTypeBadge contentType={contentType} sizeClassName="size-6" />,
        commandHint: contentType === "login" ? "Ctrl N" : undefined,
        runCommand: () => startCreatingItem(contentType),
      })),
      { commandKey: "generator", commandLabel: translate("navigation.generator"), commandIcon: <Wand2 className="size-4" />, runCommand: () => openSection("generator") },
      { commandKey: "health", commandLabel: translate("navigation.health"), commandIcon: <ShieldCheck className="size-4" />, runCommand: () => openSection("health") },
      { commandKey: "settings", commandLabel: translate("navigation.settings"), commandIcon: <Settings className="size-4" />, runCommand: () => openSection("settings") },
      { commandKey: "lock", commandLabel: translate("actions.lock"), commandIcon: <Lock className="size-4" />, commandHint: "Ctrl L", runCommand: () => void lockVaultNow() },
    ],
    [lockVaultNow, openSection, startCreatingItem, translate],
  );

  const normalizedQuery = queryText.trim().toLowerCase();
  const itemCommands: PaletteCommand[] = (matchingItems ?? []).slice(0, MAXIMUM_ITEM_RESULTS).map((itemSummary) => ({
    commandKey: itemSummary.itemId,
    commandLabel: itemSummary.title,
    commandHint: itemSummary.subtitle,
    commandIcon: <ItemTypeBadge contentType={itemSummary.contentType} sizeClassName="size-6" />,
    runCommand: () => {
      applyNavigationFilter({});
      selectItem(itemSummary.itemId);
    },
  }));
  const visibleCommands = [
    ...itemCommands,
    ...staticCommands.filter((paletteCommand) => paletteCommand.commandLabel.toLowerCase().includes(normalizedQuery)),
  ];

  useEffect(() => setHighlightedPosition(0), [queryText, matchingItems]);

  const executeCommand = (paletteCommand: PaletteCommand | undefined) => {
    if (!paletteCommand) return;
    onOpenChange(false);
    paletteCommand.runCommand();
  };

  const handleKeyDown = (keyboardEvent: KeyboardEvent) => {
    const keyHandlers: Record<string, () => void> = {
      ArrowDown: () => setHighlightedPosition((position) => Math.min(visibleCommands.length - 1, position + 1)),
      ArrowUp: () => setHighlightedPosition((position) => Math.max(0, position - 1)),
      Enter: () => executeCommand(visibleCommands[highlightedPosition]),
    };
    const keyHandler = keyHandlers[keyboardEvent.key];
    if (!keyHandler) return;
    keyboardEvent.preventDefault();
    keyHandler();
  };

  return (
    <Dialog.Root open={isOpen} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 z-40 bg-black/30" />
        <Dialog.Content
          onKeyDown={handleKeyDown}
          className="fixed top-[15vh] left-1/2 z-50 w-[calc(100vw-32px)] max-w-lg -translate-x-1/2 overflow-hidden rounded-2xl border border-border bg-surface shadow-2xl focus:outline-none"
        >
          <Dialog.Title className="sr-only">{translate("palette.title")}</Dialog.Title>
          <Dialog.Description className="sr-only">{translate("palette.description")}</Dialog.Description>
          <div className="flex items-center gap-3 border-b border-border px-4">
            <Search className="size-4 text-muted" />
            <input
              autoFocus
              value={queryText}
              onChange={(changeEvent) => setQueryText(changeEvent.target.value)}
              placeholder={translate("palette.placeholder")}
              aria-label={translate("palette.placeholder")}
              className="h-12 flex-1 bg-transparent text-sm focus:outline-none"
            />
            <kbd className="rounded border border-border px-1.5 text-[10px] text-muted">Esc</kbd>
          </div>
          <ul role="listbox" className="max-h-80 overflow-y-auto p-1.5">
            {visibleCommands.length === 0 && <li className="px-3 py-6 text-center text-sm text-muted">{translate("palette.noResults")}</li>}
            {visibleCommands.map((paletteCommand, commandPosition) => (
              <li
                key={paletteCommand.commandKey}
                role="option"
                aria-selected={commandPosition === highlightedPosition}
                onPointerMove={() => setHighlightedPosition(commandPosition)}
                onClick={() => executeCommand(paletteCommand)}
                className={`flex cursor-pointer items-center gap-3 rounded-lg px-3 py-2 text-sm ${commandPosition === highlightedPosition ? "bg-accent-soft" : ""}`}
              >
                <span className="flex size-6 items-center justify-center text-muted">{paletteCommand.commandIcon}</span>
                <span className="flex-1 truncate">{paletteCommand.commandLabel}</span>
                {paletteCommand.commandHint && <span className="truncate text-xs text-muted">{paletteCommand.commandHint}</span>}
              </li>
            ))}
          </ul>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
