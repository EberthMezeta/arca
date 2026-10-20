import { Copy, KeyRound, Paperclip, Plus, Search, ShieldCheck, Star, Trash2, Upload } from "lucide-react";
import { forwardRef, type KeyboardEvent, type ReactNode, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { useSecretClipboard } from "../../application/hooks/useSecretClipboard";
import { useFilteredVaultItems } from "../../application/hooks/useVaultData";
import { useVaultWorkspaceStore } from "../../application/stores/vaultWorkspaceStore";
import type { ItemContentType, VaultItemFilter, VaultItemSummary, VaultOverview } from "../../domain/vaultTypes";
import { ITEM_CONTENT_TYPE_REGISTRY, ITEM_CONTENT_TYPES, ItemTypeBadge } from "../registries/itemContentTypeRegistry";
import { Button } from "./primitives/Button";
import { EmptyState } from "./primitives/EmptyState";
import { IconButton } from "./primitives/IconButton";
import { Spinner } from "./primitives/Spinner";

type Translate = (translationKey: string, interpolation?: Record<string, unknown>) => string;

/** Título de la lista según el filtro activo (orden de prioridad declarativo). */
function describeActiveFilter(itemFilter: VaultItemFilter, vaultOverview: VaultOverview | null, translate: Translate): string {
  const folderScope = itemFilter.folderScope;
  const titleCandidates: Array<[boolean, () => string]> = [
    [itemFilter.shouldShowTrash, () => translate("navigation.trash")],
    [itemFilter.shouldShowOnlyFavorites, () => translate("navigation.favorites")],
    [itemFilter.contentType !== null, () => translate(ITEM_CONTENT_TYPE_REGISTRY[itemFilter.contentType as ItemContentType].pluralLabelKey)],
    [itemFilter.tagName !== null, () => `#${itemFilter.tagName}`],
    [folderScope.scope === "withoutFolder", () => translate("navigation.noFolder")],
    [
      folderScope.scope === "specificFolder",
      () =>
        vaultOverview?.folders.find(
          (folderOverview) => folderScope.scope === "specificFolder" && folderOverview.folderId === folderScope.folderId,
        )?.folderName ?? "",
    ],
  ];
  const matchingCandidate = titleCandidates.find(([isApplicable]) => isApplicable);
  return matchingCandidate ? matchingCandidate[1]() : translate("navigation.allItems");
}

interface VaultItemListProps {
  vaultOverview: VaultOverview | null;
  onRequestImport: () => void;
}

export const VaultItemList = forwardRef<HTMLInputElement, VaultItemListProps>(function VaultItemList(
  { vaultOverview, onRequestImport },
  searchInputRef,
) {
  const { t: translate } = useTranslation();
  const itemFilter = useVaultWorkspaceStore((workspaceState) => workspaceState.itemFilter);
  const selectedItemId = useVaultWorkspaceStore((workspaceState) => workspaceState.selectedItemId);
  const changeSearchText = useVaultWorkspaceStore((workspaceState) => workspaceState.changeSearchText);
  const selectItem = useVaultWorkspaceStore((workspaceState) => workspaceState.selectItem);
  const startCreatingItem = useVaultWorkspaceStore((workspaceState) => workspaceState.startCreatingItem);
  const { loadedData: itemSummaries, isLoading } = useFilteredVaultItems();
  const [isNewItemMenuOpen, setIsNewItemMenuOpen] = useState(false);
  const listContainerRef = useRef<HTMLUListElement>(null);

  const visibleSummaries = itemSummaries ?? [];
  const isVaultCompletelyEmpty = vaultOverview?.totalItemCount === 0 && !itemFilter.shouldShowTrash;

  const listDisplayState = resolveListDisplayState({
    isLoadingFirstTime: isLoading && !itemSummaries,
    isVaultCompletelyEmpty,
    hasVisibleItems: visibleSummaries.length > 0,
  });

  /** Qué se dibuja en cada estado de la lista (patrón State + tabla de renderizado). */
  const LIST_CONTENT_RENDERERS: Record<ListDisplayState, () => ReactNode> = {
    loading: () => (
      <div className="flex justify-center p-8 text-muted">
        <Spinner />
      </div>
    ),
    vaultEmpty: () => (
      <EmptyState
        illustrationIcon={<ShieldCheck className="size-7" />}
        headline={translate("emptyStates.vaultEmptyTitle")}
        explanation={translate("emptyStates.vaultEmptyText")}
        callToAction={
          <>
            <Button buttonVariant="primary" leadingIcon={<KeyRound className="size-4" />} onClick={() => startCreatingItem("login")}>
              {translate("emptyStates.addFirstLogin")}
            </Button>
            <Button leadingIcon={<Upload className="size-4" />} onClick={onRequestImport}>
              {translate("emptyStates.importFromBitwarden")}
            </Button>
          </>
        }
      />
    ),
    noMatches: () => <NoMatchesState itemFilter={itemFilter} />,
    itemList: () => (
      <ul ref={listContainerRef} role="listbox" aria-label={translate("navigation.itemList")} onKeyDown={handleListKeyDown} className="flex flex-col p-1.5">
        {visibleSummaries.map((itemSummary) => (
          <VaultItemRow
            key={itemSummary.itemId}
            itemSummary={itemSummary}
            isSelected={itemSummary.itemId === selectedItemId}
            onSelect={() => selectItem(itemSummary.itemId)}
          />
        ))}
      </ul>
    ),
  };

  /** Flechas arriba/abajo recorren la lista sin soltar el teclado. */
  const handleListKeyDown = (keyboardEvent: KeyboardEvent) => {
    const keyOffsets: Record<string, number> = { ArrowDown: 1, ArrowUp: -1 };
    const selectionOffset = keyOffsets[keyboardEvent.key];
    if (selectionOffset === undefined || visibleSummaries.length === 0) return;
    keyboardEvent.preventDefault();
    const currentPosition = visibleSummaries.findIndex((itemSummary) => itemSummary.itemId === selectedItemId);
    const nextPosition = Math.min(visibleSummaries.length - 1, Math.max(0, currentPosition + selectionOffset));
    selectItem(visibleSummaries[nextPosition]?.itemId ?? null);
  };

  useEffect(() => {
    listContainerRef.current?.querySelector<HTMLElement>("[aria-selected='true']")?.scrollIntoView({ block: "nearest" });
  }, [selectedItemId]);

  return (
    <section className="flex h-full min-w-0 flex-col border-r border-border bg-surface" aria-label={translate("navigation.itemList")}>
      <header className="flex flex-col gap-3 border-b border-border p-3">
        <div className="flex items-center justify-between gap-2">
          <h2 className="truncate text-base font-semibold">{describeActiveFilter(itemFilter, vaultOverview, translate)}</h2>
          <div className="relative">
            <Button buttonVariant="primary" leadingIcon={<Plus className="size-4" />} onClick={() => setIsNewItemMenuOpen((isOpen) => !isOpen)} aria-haspopup="menu" aria-expanded={isNewItemMenuOpen}>
              {translate("actions.new")}
            </Button>
            {isNewItemMenuOpen && (
              <NewItemMenu
                onSelectType={(contentType) => {
                  setIsNewItemMenuOpen(false);
                  startCreatingItem(contentType);
                }}
                onDismiss={() => setIsNewItemMenuOpen(false)}
              />
            )}
          </div>
        </div>
        <label className="relative">
          <span className="sr-only">{translate("search.label")}</span>
          <Search className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted" />
          <input
            ref={searchInputRef}
            type="search"
            value={itemFilter.searchText ?? ""}
            onChange={(changeEvent) => changeSearchText(changeEvent.target.value)}
            onKeyDown={handleListKeyDown}
            placeholder={translate("search.placeholder")}
            className="h-9 w-full rounded-lg border border-transparent bg-surface-muted pr-3 pl-9 text-sm placeholder:text-muted focus:border-accent focus:bg-surface focus:outline-none"
          />
        </label>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto">{LIST_CONTENT_RENDERERS[listDisplayState]()}</div>

      {visibleSummaries.length > 0 && (
        <footer className="border-t border-border px-4 py-2 text-xs text-muted">
          {translate("search.resultCount", { count: visibleSummaries.length })}
        </footer>
      )}
    </section>
  );
});

type ListDisplayState = "loading" | "vaultEmpty" | "noMatches" | "itemList";

interface ListDisplayConditions {
  isLoadingFirstTime: boolean;
  isVaultCompletelyEmpty: boolean;
  hasVisibleItems: boolean;
}

/** Reglas en orden de prioridad: la primera que se cumple decide el estado. */
const LIST_DISPLAY_RULES: Array<[ListDisplayState, (conditions: ListDisplayConditions) => boolean]> = [
  ["loading", (conditions) => conditions.isLoadingFirstTime],
  ["vaultEmpty", (conditions) => conditions.isVaultCompletelyEmpty],
  ["itemList", (conditions) => conditions.hasVisibleItems],
];

function resolveListDisplayState(conditions: ListDisplayConditions): ListDisplayState {
  return LIST_DISPLAY_RULES.find(([, ruleApplies]) => ruleApplies(conditions))?.[0] ?? "noMatches";
}

function NoMatchesState({ itemFilter }: { itemFilter: VaultItemFilter }) {
  const { t: translate } = useTranslation();
  const hasActiveSearch = Boolean(itemFilter.searchText);
  const headlineRules: Array<[boolean, string]> = [
    [hasActiveSearch, "emptyStates.noResultsTitle"],
    [itemFilter.shouldShowTrash, "emptyStates.trashEmptyTitle"],
  ];
  const headlineKey = headlineRules.find(([isApplicable]) => isApplicable)?.[1] ?? "emptyStates.sectionEmptyTitle";
  return (
    <EmptyState
      illustrationIcon={itemFilter.shouldShowTrash ? <Trash2 className="size-7" /> : <Search className="size-7" />}
      headline={translate(headlineKey)}
      explanation={hasActiveSearch ? translate("emptyStates.noResultsText", { searchText: itemFilter.searchText }) : undefined}
    />
  );
}

function NewItemMenu({ onSelectType, onDismiss }: { onSelectType: (contentType: ItemContentType) => void; onDismiss: () => void }) {
  const { t: translate } = useTranslation();
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    menuRef.current?.querySelector<HTMLButtonElement>("button")?.focus();
    const dismissOnOutsideClick = (pointerEvent: PointerEvent) => {
      if (!menuRef.current?.contains(pointerEvent.target as Node)) onDismiss();
    };
    window.addEventListener("pointerdown", dismissOnOutsideClick);
    return () => window.removeEventListener("pointerdown", dismissOnOutsideClick);
  }, [onDismiss]);

  return (
    <div
      ref={menuRef}
      role="menu"
      onKeyDown={(keyboardEvent) => keyboardEvent.key === "Escape" && onDismiss()}
      className="absolute top-full right-0 z-30 mt-1 w-52 rounded-xl border border-border bg-surface p-1 shadow-xl"
    >
      {ITEM_CONTENT_TYPES.map((contentType) => (
        <button
          key={contentType}
          type="button"
          role="menuitem"
          onClick={() => onSelectType(contentType)}
          className="flex w-full items-center gap-2.5 rounded-lg px-2 py-1.5 text-left text-sm hover:bg-surface-hover focus:bg-surface-hover focus:outline-none"
        >
          <ItemTypeBadge contentType={contentType} sizeClassName="size-7" />
          {translate(ITEM_CONTENT_TYPE_REGISTRY[contentType].singularLabelKey)}
        </button>
      ))}
    </div>
  );
}

interface VaultItemRowProps {
  itemSummary: VaultItemSummary;
  isSelected: boolean;
  onSelect: () => void;
}

function VaultItemRow({ itemSummary, isSelected, onSelect }: VaultItemRowProps) {
  const { t: translate } = useTranslation();
  const { copyItemField } = useSecretClipboard();
  const isLogin = itemSummary.contentType === "login";

  return (
    <li
      role="option"
      aria-selected={isSelected}
      tabIndex={isSelected ? 0 : -1}
      onClick={onSelect}
      onKeyDown={(keyboardEvent) => keyboardEvent.key === "Enter" && onSelect()}
      className={`group flex cursor-pointer items-center gap-3 rounded-lg px-2.5 py-2 ${isSelected ? "bg-accent-soft" : "hover:bg-surface-hover"}`}
    >
      <ItemTypeBadge contentType={itemSummary.contentType} />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-1.5">
          <span className="truncate font-medium">{itemSummary.title}</span>
          {itemSummary.isFavorite && <Star className="size-3.5 shrink-0 fill-warning text-warning" aria-label={translate("fields.favorite")} />}
          {itemSummary.attachmentCount > 0 && <Paperclip className="size-3.5 shrink-0 text-muted" aria-label={translate("attachments.title")} />}
        </div>
        <div className="truncate text-xs text-muted">{itemSummary.subtitle || " "}</div>
      </div>
      {isLogin && !itemSummary.isInTrash && (
        <div className="flex opacity-0 transition-opacity group-focus-within:opacity-100 group-hover:opacity-100">
          {itemSummary.subtitle && (
            <IconButton
              accessibleLabel={translate("actions.copyUsername")}
              icon={<Copy className="size-3.5" />}
              onClick={(clickEvent) => {
                clickEvent.stopPropagation();
                void copyItemField(itemSummary.itemId, { fieldKind: "username" }, translate("fields.username"));
              }}
            />
          )}
          {itemSummary.hasPassword && (
            <IconButton
              accessibleLabel={translate("actions.copyPassword")}
              icon={<KeyRound className="size-3.5" />}
              onClick={(clickEvent) => {
                clickEvent.stopPropagation();
                void copyItemField(itemSummary.itemId, { fieldKind: "password" }, translate("fields.password"));
              }}
            />
          )}
        </div>
      )}
    </li>
  );
}
