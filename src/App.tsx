import { ShieldCheck } from "lucide-react";
import { type ComponentType, useEffect } from "react";
import { useTranslation } from "react-i18next";

import { type AppScreen, useSessionStore } from "./application/stores/sessionStore";
import { useToastStore } from "./application/stores/toastStore";
import { useVaultWorkspaceStore } from "./application/stores/vaultWorkspaceStore";
import { vaultApiClient } from "./infrastructure/vaultApiClient";
import { Spinner } from "./presentation/components/primitives/Spinner";
import { ToastViewport } from "./presentation/components/ToastViewport";
import { SetupScreen } from "./presentation/screens/SetupScreen";
import { UnlockScreen } from "./presentation/screens/UnlockScreen";
import { VaultScreen } from "./presentation/screens/VaultScreen";

function LoadingScreen() {
  return (
    <main className="flex h-full flex-col items-center justify-center gap-4 text-muted">
      <ShieldCheck className="size-10 text-accent" />
      <Spinner />
    </main>
  );
}

/** Patrón State en la UI: cada estado de la sesión tiene su pantalla. */
const SCREEN_COMPONENTS: Record<AppScreen, ComponentType> = {
  loading: LoadingScreen,
  setup: SetupScreen,
  unlock: UnlockScreen,
  vault: VaultScreen,
};

export function App() {
  const { t: translate } = useTranslation();
  const appScreen = useSessionStore((sessionState) => sessionState.appScreen);
  const refreshVaultStatus = useSessionStore((sessionState) => sessionState.refreshVaultStatus);
  const markVaultLocked = useSessionStore((sessionState) => sessionState.markVaultLocked);
  const resetWorkspace = useVaultWorkspaceStore((workspaceState) => workspaceState.resetWorkspace);
  const showToast = useToastStore((toastState) => toastState.showToast);

  useEffect(() => {
    void refreshVaultStatus();
  }, [refreshVaultStatus]);

  useEffect(() => {
    const vaultLockedSubscription = vaultApiClient.onVaultLocked((lockReason) => {
      resetWorkspace();
      markVaultLocked(lockReason);
    });
    const clipboardClearedSubscription = vaultApiClient.onClipboardCleared(() =>
      showToast({ message: translate("clipboard.cleared"), tone: "info", durationMilliseconds: 2000 }),
    );
    return () => {
      void vaultLockedSubscription.then((unsubscribe) => unsubscribe());
      void clipboardClearedSubscription.then((unsubscribe) => unsubscribe());
    };
  }, [markVaultLocked, resetWorkspace, showToast, translate]);

  useEffect(() => {
    if (appScreen !== "vault") resetWorkspace();
  }, [appScreen, resetWorkspace]);

  const ActiveScreen = SCREEN_COMPONENTS[appScreen];
  return (
    <>
      <ActiveScreen />
      <ToastViewport />
    </>
  );
}
