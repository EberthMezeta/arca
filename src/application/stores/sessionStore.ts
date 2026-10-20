import { create } from "zustand";

import type { VaultLockReason } from "../../domain/vaultTypes";
import { vaultApiClient } from "../../infrastructure/vaultApiClient";

export type AppScreen = "loading" | "setup" | "unlock" | "vault";

interface SessionState {
  appScreen: AppScreen;
  lastLockReason: VaultLockReason | null;
  refreshVaultStatus: () => Promise<void>;
  markVaultUnlocked: () => void;
  markVaultLocked: (lockReason: VaultLockReason | null) => void;
  lockVaultNow: () => Promise<void>;
}

export const useSessionStore = create<SessionState>((setState) => ({
  appScreen: "loading",
  lastLockReason: null,

  refreshVaultStatus: async () => {
    const vaultStatus = await vaultApiClient.getVaultStatus();
    const nextScreen: AppScreen = !vaultStatus.vaultExists ? "setup" : vaultStatus.isUnlocked ? "vault" : "unlock";
    setState({ appScreen: nextScreen });
  },

  markVaultUnlocked: () => setState({ appScreen: "vault", lastLockReason: null }),

  markVaultLocked: (lockReason) => setState({ appScreen: "unlock", lastLockReason: lockReason }),

  lockVaultNow: async () => {
    await vaultApiClient.lockVault();
    setState({ appScreen: "unlock", lastLockReason: null });
  },
}));
