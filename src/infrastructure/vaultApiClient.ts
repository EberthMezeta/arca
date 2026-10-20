// Único punto del frontend que habla con el backend (`invoke`).
// El resto de la app depende de este objeto, nunca de Tauri directamente.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type {
  AttachmentReference,
  ClipboardCopyResult,
  CommandError,
  ExportFormat,
  Folder,
  FolderId,
  ImportFormat,
  ImportSummary,
  ItemId,
  PasswordGenerationOptions,
  PasswordHealthReport,
  PasswordStrength,
  RevealableField,
  SaveVaultItemRequest,
  TotpCode,
  VaultItem,
  VaultItemFilter,
  VaultItemSummary,
  VaultLockReason,
  VaultOverview,
  VaultPreferences,
  VaultStatus,
} from "../domain/vaultTypes";

export function isCommandError(unknownError: unknown): unknownError is CommandError {
  return typeof unknownError === "object" && unknownError !== null && "errorCode" in unknownError;
}

function toCommandError(unknownError: unknown): CommandError {
  if (isCommandError(unknownError)) return unknownError;
  return { errorCode: "internal", technicalDetail: String(unknownError) };
}

async function callBackend<Result>(commandName: string, commandArguments?: Record<string, unknown>): Promise<Result> {
  try {
    return await invoke<Result>(commandName, commandArguments);
  } catch (unknownError) {
    throw toCommandError(unknownError);
  }
}

export const vaultApiClient = {
  // Sesión
  getVaultStatus: () => callBackend<VaultStatus>("get_vault_status"),
  createVault: (masterPassword: string) => callBackend<void>("create_vault", { masterPassword }),
  unlockVault: (masterPassword: string) => callBackend<void>("unlock_vault", { masterPassword }),
  lockVault: () => callBackend<void>("lock_vault"),
  registerUserActivity: () => callBackend<void>("register_user_activity"),
  changeMasterPassword: (currentMasterPassword: string, newMasterPassword: string) =>
    callBackend<void>("change_master_password", { currentMasterPassword, newMasterPassword }),
  getVaultLocation: () => callBackend<string>("get_vault_location"),

  // Elementos
  searchVaultItems: (itemFilter: VaultItemFilter) =>
    callBackend<VaultItemSummary[]>("search_vault_items", { itemFilter }),
  getVaultOverview: () => callBackend<VaultOverview>("get_vault_overview"),
  getVaultItemDetails: (itemId: ItemId) => callBackend<VaultItem>("get_vault_item_details", { itemId }),
  getVaultItemForEditing: (itemId: ItemId) => callBackend<VaultItem>("get_vault_item", { itemId }),
  saveVaultItem: (saveRequest: SaveVaultItemRequest) => callBackend<VaultItem>("save_vault_item", { saveRequest }),
  moveItemToTrash: (itemId: ItemId) => callBackend<void>("move_item_to_trash", { itemId }),
  restoreItemFromTrash: (itemId: ItemId) => callBackend<void>("restore_item_from_trash", { itemId }),
  deleteItemPermanently: (itemId: ItemId) => callBackend<void>("delete_item_permanently", { itemId }),
  emptyTrash: () => callBackend<number>("empty_trash"),
  revealItemField: (itemId: ItemId, requestedField: RevealableField) =>
    callBackend<string>("reveal_item_field", { itemId, requestedField }),
  getTotpCode: (itemId: ItemId) => callBackend<TotpCode>("get_totp_code", { itemId }),
  getPasswordHealthReport: () => callBackend<PasswordHealthReport>("get_password_health_report"),

  // Carpetas
  listFolders: () => callBackend<Folder[]>("list_folders"),
  createFolder: (folderName: string) => callBackend<Folder>("create_folder", { folderName }),
  renameFolder: (folderId: FolderId, folderName: string) => callBackend<Folder>("rename_folder", { folderId, folderName }),
  deleteFolder: (folderId: FolderId) => callBackend<void>("delete_folder", { folderId }),

  // Portapapeles
  copyItemField: (itemId: ItemId, requestedField: RevealableField) =>
    callBackend<ClipboardCopyResult>("copy_item_field", { itemId, requestedField }),
  copyTotpCode: (itemId: ItemId) => callBackend<ClipboardCopyResult>("copy_totp_code", { itemId }),
  copyTextSecret: (secretValue: string) => callBackend<ClipboardCopyResult>("copy_text_secret", { secretValue }),

  // Generador
  generatePassword: (generationOptions: PasswordGenerationOptions) =>
    callBackend<string>("generate_password", { generationOptions }),
  estimatePasswordStrength: (candidatePassword: string) =>
    callBackend<PasswordStrength>("estimate_password_strength", { candidatePassword }),

  // Adjuntos (los diálogos de archivo se abren desde Rust)
  addAttachmentFromFile: (itemId: ItemId, dialogTitle: string) =>
    callBackend<AttachmentReference | null>("add_attachment_from_file", { itemId, dialogTitle }),
  saveAttachmentToFile: (itemId: ItemId, attachmentId: string, dialogTitle: string) =>
    callBackend<boolean>("save_attachment_to_file", { itemId, attachmentId, dialogTitle }),
  deleteAttachment: (itemId: ItemId, attachmentId: string) =>
    callBackend<void>("delete_attachment", { itemId, attachmentId }),

  // Importar / exportar
  importVaultFile: (importFormat: ImportFormat, filePassword: string | null, dialogTitle: string) =>
    callBackend<ImportSummary | null>("import_vault_file", { importFormat, filePassword, dialogTitle }),
  exportVaultToFile: (exportFormat: ExportFormat, filePassword: string | null, dialogTitle: string) =>
    callBackend<boolean>("export_vault_to_file", { exportFormat, filePassword, dialogTitle }),

  // Preferencias
  getVaultPreferences: () => callBackend<VaultPreferences>("get_vault_preferences"),
  updateVaultPreferences: (newPreferences: VaultPreferences) =>
    callBackend<VaultPreferences>("update_vault_preferences", { newPreferences }),

  // Eventos (patrón Observer)
  onVaultLocked: (handleVaultLocked: (lockReason: VaultLockReason) => void): Promise<UnlistenFn> =>
    listen<{ lockReason: VaultLockReason }>("vault-locked", (lockedEvent) =>
      handleVaultLocked(lockedEvent.payload.lockReason),
    ),
  onClipboardCleared: (handleClipboardCleared: () => void): Promise<UnlistenFn> =>
    listen("clipboard-cleared", () => handleClipboardCleared()),
};

export type VaultApiClient = typeof vaultApiClient;
