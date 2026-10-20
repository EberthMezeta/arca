import { Folder as FolderIcon, FolderPlus, Hash, Layers, Lock, Pencil, Settings, ShieldCheck, Star, Trash2, Wand2, X } from "lucide-react";
import { type ReactNode, useState } from "react";
import { useTranslation } from "react-i18next";

import { useErrorMessage } from "../../application/hooks/useErrorMessage";
import { useSessionStore } from "../../application/stores/sessionStore";
import { useToastStore } from "../../application/stores/toastStore";
import { type VaultSection, useVaultWorkspaceStore } from "../../application/stores/vaultWorkspaceStore";
import type { CommandError, FolderOverview, VaultItemFilter, VaultOverview } from "../../domain/vaultTypes";
import { vaultApiClient } from "../../infrastructure/vaultApiClient";
import { ITEM_CONTENT_TYPE_REGISTRY, ITEM_CONTENT_TYPES } from "../registries/itemContentTypeRegistry";
import { ConfirmDialog } from "./primitives/ConfirmDialog";
import { IconButton } from "./primitives/IconButton";
import { TextPromptDialog } from "./primitives/TextPromptDialog";

interface NavigationEntry {
  entryKey: string;
  entryIcon: ReactNode;
  entryLabel: string;
  itemCount?: number;
  isActive: boolean;
  activateEntry: () => void;
  trailingActions?: ReactNode;
}

function NavigationButton({ navigationEntry }: { navigationEntry: NavigationEntry }) {
  return (
    <li className="group relative">
      <button
        type="button"
        aria-current={navigationEntry.isActive ? "page" : undefined}
        onClick={navigationEntry.activateEntry}
        className={`flex w-full items-center gap-2.5 rounded-lg px-2.5 py-1.5 text-left text-sm transition-colors ${navigationEntry.isActive ? "bg-accent-soft font-medium text-accent" : "text-text hover:bg-surface-hover"}`}
      >
        <span className={navigationEntry.isActive ? "text-accent" : "text-muted"}>{navigationEntry.entryIcon}</span>
        <span className="flex-1 truncate">{navigationEntry.entryLabel}</span>
        {navigationEntry.itemCount !== undefined && (
          <span className={`text-xs text-muted tabular-nums ${navigationEntry.trailingActions ? "group-focus-within:invisible group-hover:invisible" : ""}`}>
            {navigationEntry.itemCount}
          </span>
        )}
      </button>
      {navigationEntry.trailingActions && (
        <div className="absolute inset-y-0 right-1 hidden items-center group-focus-within:flex group-hover:flex">{navigationEntry.trailingActions}</div>
      )}
    </li>
  );
}

function NavigationGroup({ groupTitle, groupAction, navigationEntries }: { groupTitle?: string; groupAction?: ReactNode; navigationEntries: NavigationEntry[] }) {
  if (navigationEntries.length === 0 && !groupAction) return null;
  return (
    <div className="flex flex-col gap-0.5">
      {groupTitle && (
        <div className="flex h-7 items-center justify-between px-2.5">
          <h3 className="text-[11px] font-semibold tracking-wider text-muted uppercase">{groupTitle}</h3>
          {groupAction}
        </div>
      )}
      <ul className="flex flex-col gap-0.5">
        {navigationEntries.map((navigationEntry) => (
          <NavigationButton key={navigationEntry.entryKey} navigationEntry={navigationEntry} />
        ))}
      </ul>
    </div>
  );
}

type FolderDialogState = { dialogKind: "none" } | { dialogKind: "create" } | { dialogKind: "rename"; folder: FolderOverview } | { dialogKind: "delete"; folder: FolderOverview };

const ICON_CLASS = "size-4";

export function VaultSidebar({ vaultOverview }: { vaultOverview: VaultOverview | null }) {
  const { t: translate } = useTranslation();
  const activeSection = useVaultWorkspaceStore((workspaceState) => workspaceState.activeSection);
  const itemFilter = useVaultWorkspaceStore((workspaceState) => workspaceState.itemFilter);
  const applyNavigationFilter = useVaultWorkspaceStore((workspaceState) => workspaceState.applyNavigationFilter);
  const openSection = useVaultWorkspaceStore((workspaceState) => workspaceState.openSection);
  const notifyVaultDataChanged = useVaultWorkspaceStore((workspaceState) => workspaceState.notifyVaultDataChanged);
  const lockVaultNow = useSessionStore((sessionState) => sessionState.lockVaultNow);
  const showToast = useToastStore((toastState) => toastState.showToast);
  const describeError = useErrorMessage();
  const [folderDialogState, setFolderDialogState] = useState<FolderDialogState>({ dialogKind: "none" });

  const isItemsSection = activeSection === "items";
  const isFilterActive = (filterMatcher: (currentFilter: VaultItemFilter) => boolean) => isItemsSection && filterMatcher(itemFilter);
  const isPlainFilter = (currentFilter: VaultItemFilter) =>
    !currentFilter.shouldShowTrash && !currentFilter.shouldShowOnlyFavorites && currentFilter.contentType === null && currentFilter.tagName === null;

  const libraryEntries: NavigationEntry[] = [
    {
      entryKey: "all",
      entryIcon: <Layers className={ICON_CLASS} />,
      entryLabel: translate("navigation.allItems"),
      itemCount: vaultOverview?.totalItemCount,
      isActive: isFilterActive((currentFilter) => isPlainFilter(currentFilter) && currentFilter.folderScope.scope === "allFolders"),
      activateEntry: () => applyNavigationFilter({}),
    },
    {
      entryKey: "favorites",
      entryIcon: <Star className={ICON_CLASS} />,
      entryLabel: translate("navigation.favorites"),
      itemCount: vaultOverview?.favoriteItemCount,
      isActive: isFilterActive((currentFilter) => currentFilter.shouldShowOnlyFavorites),
      activateEntry: () => applyNavigationFilter({ shouldShowOnlyFavorites: true }),
    },
  ];

  const typeEntries: NavigationEntry[] = ITEM_CONTENT_TYPES.map((contentType) => {
    const { typeIcon: TypeIcon, pluralLabelKey } = ITEM_CONTENT_TYPE_REGISTRY[contentType];
    return {
      entryKey: contentType,
      entryIcon: <TypeIcon className={ICON_CLASS} />,
      entryLabel: translate(pluralLabelKey),
      itemCount: vaultOverview?.contentTypeItemCounts[contentType] ?? 0,
      isActive: isFilterActive((currentFilter) => currentFilter.contentType === contentType),
      activateEntry: () => applyNavigationFilter({ contentType }),
    };
  });

  const folderEntries: NavigationEntry[] = (vaultOverview?.folders ?? []).map((folderOverview) => ({
    entryKey: folderOverview.folderId,
    entryIcon: <FolderIcon className={ICON_CLASS} />,
    entryLabel: folderOverview.folderName,
    itemCount: folderOverview.itemCount,
    isActive: isFilterActive(
      (currentFilter) =>
        isPlainFilter(currentFilter) && currentFilter.folderScope.scope === "specificFolder" && currentFilter.folderScope.folderId === folderOverview.folderId,
    ),
    activateEntry: () => applyNavigationFilter({ folderScope: { scope: "specificFolder", folderId: folderOverview.folderId } }),
    trailingActions: (
      <>
        <IconButton
          className="size-6"
          accessibleLabel={translate("folders.rename")}
          icon={<Pencil className="size-3.5" />}
          onClick={() => setFolderDialogState({ dialogKind: "rename", folder: folderOverview })}
        />
        <IconButton
          className="size-6"
          accessibleLabel={translate("folders.delete")}
          icon={<X className="size-3.5" />}
          onClick={() => setFolderDialogState({ dialogKind: "delete", folder: folderOverview })}
        />
      </>
    ),
  }));
  if ((vaultOverview?.itemsWithoutFolderCount ?? 0) > 0 && folderEntries.length > 0) {
    folderEntries.push({
      entryKey: "without-folder",
      entryIcon: <FolderIcon className={`${ICON_CLASS} opacity-50`} />,
      entryLabel: translate("navigation.noFolder"),
      itemCount: vaultOverview?.itemsWithoutFolderCount,
      isActive: isFilterActive((currentFilter) => isPlainFilter(currentFilter) && currentFilter.folderScope.scope === "withoutFolder"),
      activateEntry: () => applyNavigationFilter({ folderScope: { scope: "withoutFolder" } }),
    });
  }

  const tagEntries: NavigationEntry[] = Object.entries(vaultOverview?.tagItemCounts ?? {}).map(([tagName, itemCount]) => ({
    entryKey: `tag-${tagName}`,
    entryIcon: <Hash className={ICON_CLASS} />,
    entryLabel: tagName,
    itemCount,
    isActive: isFilterActive((currentFilter) => currentFilter.tagName === tagName),
    activateEntry: () => applyNavigationFilter({ tagName }),
  }));

  const sectionEntry = (section: VaultSection, entryIcon: ReactNode, labelKey: string): NavigationEntry => ({
    entryKey: section,
    entryIcon,
    entryLabel: translate(labelKey),
    isActive: activeSection === section,
    activateEntry: () => openSection(section),
  });
  const toolEntries: NavigationEntry[] = [
    sectionEntry("generator", <Wand2 className={ICON_CLASS} />, "navigation.generator"),
    sectionEntry("health", <ShieldCheck className={ICON_CLASS} />, "navigation.health"),
    sectionEntry("settings", <Settings className={ICON_CLASS} />, "navigation.settings"),
  ];

  const trashEntries: NavigationEntry[] = [
    {
      entryKey: "trash",
      entryIcon: <Trash2 className={ICON_CLASS} />,
      entryLabel: translate("navigation.trash"),
      itemCount: vaultOverview?.trashItemCount,
      isActive: isFilterActive((currentFilter) => currentFilter.shouldShowTrash),
      activateEntry: () => applyNavigationFilter({ shouldShowTrash: true }),
    },
  ];

  const runFolderAction = async (folderAction: () => Promise<unknown>) => {
    await folderAction();
    notifyVaultDataChanged();
  };

  return (
    <nav aria-label={translate("navigation.label")} className="flex h-full flex-col bg-canvas">
      <div className="flex items-center gap-2.5 px-4 pt-4 pb-3">
        <span className="flex size-8 items-center justify-center rounded-lg bg-accent text-accent-contrast">
          <ShieldCheck className="size-4.5" />
        </span>
        <span className="text-base font-semibold">RustVault</span>
      </div>

      <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-2 pb-3">
        <NavigationGroup navigationEntries={libraryEntries} />
        <NavigationGroup groupTitle={translate("navigation.types")} navigationEntries={typeEntries} />
        <NavigationGroup
          groupTitle={translate("navigation.folders")}
          groupAction={
            <IconButton
              className="size-6"
              accessibleLabel={translate("folders.create")}
              icon={<FolderPlus className="size-3.5" />}
              onClick={() => setFolderDialogState({ dialogKind: "create" })}
            />
          }
          navigationEntries={folderEntries}
        />
        <NavigationGroup groupTitle={translate("navigation.tags")} navigationEntries={tagEntries} />
        <NavigationGroup navigationEntries={trashEntries} />
        <NavigationGroup groupTitle={translate("navigation.tools")} navigationEntries={toolEntries} />
      </div>

      <div className="border-t border-border p-2">
        <button
          type="button"
          onClick={() => void lockVaultNow()}
          className="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm font-medium hover:bg-surface-hover"
        >
          <Lock className="size-4 text-muted" />
          <span className="flex-1 text-left">{translate("actions.lock")}</span>
          <kbd className="rounded border border-border px-1.5 text-[10px] text-muted">Ctrl L</kbd>
        </button>
      </div>

      <TextPromptDialog
        isOpen={folderDialogState.dialogKind === "create" || folderDialogState.dialogKind === "rename"}
        onOpenChange={(isOpen) => !isOpen && setFolderDialogState({ dialogKind: "none" })}
        dialogTitle={translate(folderDialogState.dialogKind === "rename" ? "folders.rename" : "folders.create")}
        fieldLabel={translate("folders.name")}
        initialValue={folderDialogState.dialogKind === "rename" ? folderDialogState.folder.folderName : ""}
        confirmLabel={translate(folderDialogState.dialogKind === "rename" ? "actions.save" : "actions.create")}
        onSubmitValue={(folderName) =>
          runFolderAction(() =>
            folderDialogState.dialogKind === "rename"
              ? vaultApiClient.renameFolder(folderDialogState.folder.folderId, folderName)
              : vaultApiClient.createFolder(folderName),
          )
        }
      />
      <ConfirmDialog
        isOpen={folderDialogState.dialogKind === "delete"}
        onOpenChange={(isOpen) => !isOpen && setFolderDialogState({ dialogKind: "none" })}
        dialogTitle={translate("folders.deleteTitle")}
        dialogDescription={translate("folders.deleteText", {
          folderName: folderDialogState.dialogKind === "delete" ? folderDialogState.folder.folderName : "",
        })}
        confirmLabel={translate("folders.delete")}
        isDestructive
        onConfirm={() => {
          if (folderDialogState.dialogKind !== "delete") return;
          const deletedFolderId = folderDialogState.folder.folderId;
          runFolderAction(() => vaultApiClient.deleteFolder(deletedFolderId))
            .then(() => applyNavigationFilter({}))
            .catch((commandError: CommandError) => showToast({ message: describeError(commandError), tone: "error" }));
        }}
      />
    </nav>
  );
}
