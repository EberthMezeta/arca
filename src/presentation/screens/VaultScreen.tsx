import * as Tooltip from "@radix-ui/react-tooltip";
import { type ComponentType, useRef, useState } from "react";

import { useKeyboardShortcuts } from "../../application/hooks/useKeyboardShortcuts";
import { useUserActivityReporter } from "../../application/hooks/useUserActivityReporter";
import { useVaultOverview } from "../../application/hooks/useVaultData";
import { useSessionStore } from "../../application/stores/sessionStore";
import { type ItemEditorState, type VaultSection, useVaultWorkspaceStore } from "../../application/stores/vaultWorkspaceStore";
import { CommandPalette } from "../components/CommandPalette";
import { VaultItemDetails } from "../components/VaultItemDetails";
import { VaultItemEditor } from "../components/VaultItemEditor";
import { VaultItemList } from "../components/VaultItemList";
import { VaultSidebar } from "../components/VaultSidebar";
import { GeneratorView } from "../views/GeneratorView";
import { PasswordHealthView } from "../views/PasswordHealthView";
import { SettingsView } from "../views/SettingsView";

/** Panel derecho del espacio de elementos: detalle o editor según el estado. */
function ItemPanel({ itemEditorState }: { itemEditorState: ItemEditorState }) {
  if (itemEditorState.editorMode === "closed") return <VaultItemDetails />;
  return <VaultItemEditor key={JSON.stringify(itemEditorState)} editorState={itemEditorState} />;
}

function ItemsWorkspace() {
  const searchInputRef = useRef<HTMLInputElement>(null);
  const itemEditorState = useVaultWorkspaceStore((workspaceState) => workspaceState.itemEditorState);
  const openSection = useVaultWorkspaceStore((workspaceState) => workspaceState.openSection);
  const { loadedData: vaultOverview } = useVaultOverview();

  useKeyboardShortcuts({ "mod+f": () => searchInputRef.current?.focus() });

  return (
    <div className="grid h-full grid-cols-[minmax(260px,340px)_1fr]">
      <VaultItemList ref={searchInputRef} vaultOverview={vaultOverview} onRequestImport={() => openSection("settings")} />
      <div className="min-w-0 bg-canvas">
        <ItemPanel itemEditorState={itemEditorState} />
      </div>
    </div>
  );
}

/** Registro de secciones: cada entrada de la barra lateral abre su vista. */
const SECTION_VIEWS: Record<VaultSection, ComponentType> = {
  items: ItemsWorkspace,
  generator: GeneratorView,
  health: PasswordHealthView,
  settings: SettingsView,
};

export function VaultScreen() {
  const activeSection = useVaultWorkspaceStore((workspaceState) => workspaceState.activeSection);
  const startCreatingItem = useVaultWorkspaceStore((workspaceState) => workspaceState.startCreatingItem);
  const openSection = useVaultWorkspaceStore((workspaceState) => workspaceState.openSection);
  const lockVaultNow = useSessionStore((sessionState) => sessionState.lockVaultNow);
  const { loadedData: vaultOverview } = useVaultOverview();
  const [isCommandPaletteOpen, setIsCommandPaletteOpen] = useState(false);

  useUserActivityReporter();
  useKeyboardShortcuts({
    "mod+k": () => setIsCommandPaletteOpen(true),
    "mod+n": () => startCreatingItem("login"),
    "mod+l": () => void lockVaultNow(),
    "mod+f": () => openSection("items"),
  });

  const ActiveSectionView = SECTION_VIEWS[activeSection];

  return (
    <Tooltip.Provider delayDuration={400}>
      <div className="grid h-full grid-cols-[230px_1fr]">
        <aside className="min-h-0 border-r border-border">
          <VaultSidebar vaultOverview={vaultOverview} />
        </aside>
        <main className="min-h-0 min-w-0">
          <ActiveSectionView />
        </main>
      </div>
      <CommandPalette isOpen={isCommandPaletteOpen} onOpenChange={setIsCommandPaletteOpen} />
    </Tooltip.Provider>
  );
}
