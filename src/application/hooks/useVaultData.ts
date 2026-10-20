import type { VaultItemSummary, VaultOverview } from "../../domain/vaultTypes";
import { vaultApiClient } from "../../infrastructure/vaultApiClient";
import { useVaultWorkspaceStore } from "../stores/vaultWorkspaceStore";
import { type AsyncDataState, useAsyncData } from "./useAsyncData";
import { useDebouncedValue } from "./useDebouncedValue";

const SEARCH_DEBOUNCE_MILLISECONDS = 120;

export function useFilteredVaultItems(): AsyncDataState<VaultItemSummary[]> {
  const itemFilter = useVaultWorkspaceStore((workspaceState) => workspaceState.itemFilter);
  const vaultDataRevision = useVaultWorkspaceStore((workspaceState) => workspaceState.vaultDataRevision);
  const debouncedItemFilter = useDebouncedValue(itemFilter, SEARCH_DEBOUNCE_MILLISECONDS);

  return useAsyncData(() => vaultApiClient.searchVaultItems(debouncedItemFilter), [debouncedItemFilter, vaultDataRevision]);
}

export function useVaultOverview(): AsyncDataState<VaultOverview> {
  const vaultDataRevision = useVaultWorkspaceStore((workspaceState) => workspaceState.vaultDataRevision);
  return useAsyncData(() => vaultApiClient.getVaultOverview(), [vaultDataRevision]);
}
