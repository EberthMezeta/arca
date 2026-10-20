import { create } from "zustand";

import type { ItemContentType, ItemId, VaultItemFilter } from "../../domain/vaultTypes";

export type VaultSection = "items" | "generator" | "health" | "settings";

export type ItemEditorState =
  | { editorMode: "closed" }
  | { editorMode: "creating"; contentType: ItemContentType }
  | { editorMode: "editing"; itemId: ItemId };

export const DEFAULT_ITEM_FILTER: VaultItemFilter = {
  searchText: null,
  folderScope: { scope: "allFolders" },
  tagName: null,
  contentType: null,
  shouldShowOnlyFavorites: false,
  shouldShowTrash: false,
};

interface VaultWorkspaceState {
  activeSection: VaultSection;
  itemFilter: VaultItemFilter;
  selectedItemId: ItemId | null;
  itemEditorState: ItemEditorState;
  /** Se incrementa tras cada cambio para que las vistas vuelvan a pedir datos. */
  vaultDataRevision: number;
  openSection: (activeSection: VaultSection) => void;
  applyNavigationFilter: (filterChanges: Partial<VaultItemFilter>) => void;
  changeSearchText: (searchText: string) => void;
  selectItem: (itemId: ItemId | null) => void;
  startCreatingItem: (contentType: ItemContentType) => void;
  startEditingItem: (itemId: ItemId) => void;
  closeItemEditor: () => void;
  notifyVaultDataChanged: () => void;
  resetWorkspace: () => void;
}

export const useVaultWorkspaceStore = create<VaultWorkspaceState>((setState) => ({
  activeSection: "items",
  itemFilter: DEFAULT_ITEM_FILTER,
  selectedItemId: null,
  itemEditorState: { editorMode: "closed" },
  vaultDataRevision: 0,

  openSection: (activeSection) => setState({ activeSection }),

  /** Al cambiar de carpeta/tipo/etiqueta se conserva la búsqueda escrita. */
  applyNavigationFilter: (filterChanges) =>
    setState((currentState) => ({
      activeSection: "items",
      itemFilter: { ...DEFAULT_ITEM_FILTER, searchText: currentState.itemFilter.searchText, ...filterChanges },
      selectedItemId: null,
      itemEditorState: { editorMode: "closed" },
    })),

  changeSearchText: (searchText) =>
    setState((currentState) => ({ itemFilter: { ...currentState.itemFilter, searchText: searchText || null } })),

  selectItem: (selectedItemId) => setState({ selectedItemId, itemEditorState: { editorMode: "closed" } }),

  startCreatingItem: (contentType) =>
    setState({ activeSection: "items", itemEditorState: { editorMode: "creating", contentType } }),

  startEditingItem: (itemId) =>
    setState({ activeSection: "items", selectedItemId: itemId, itemEditorState: { editorMode: "editing", itemId } }),

  closeItemEditor: () => setState({ itemEditorState: { editorMode: "closed" } }),

  notifyVaultDataChanged: () => setState((currentState) => ({ vaultDataRevision: currentState.vaultDataRevision + 1 })),

  resetWorkspace: () =>
    setState({
      activeSection: "items",
      itemFilter: DEFAULT_ITEM_FILTER,
      selectedItemId: null,
      itemEditorState: { editorMode: "closed" },
    }),
}));
