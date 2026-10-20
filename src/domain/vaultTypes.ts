// Tipos espejo de los DTO serializados por el backend Rust (serde, camelCase).

export type ItemId = string;
export type FolderId = string;
export type AttachmentId = string;
export type IsoDateTime = string;

export interface VaultStatus {
  vaultExists: boolean;
  isUnlocked: boolean;
  unlockRetryAfterSeconds: number | null;
}

export type ItemContentType = "login" | "secureNote" | "paymentCard" | "identity";

export interface LoginContent {
  contentType: "login";
  username: string;
  password: string;
  websiteUrls: string[];
  totpSecret: string | null;
}

export interface SecureNoteContent {
  contentType: "secureNote";
}

export interface PaymentCardContent {
  contentType: "paymentCard";
  cardholderName: string;
  cardBrand: string;
  cardNumber: string;
  expirationMonth: string;
  expirationYear: string;
  securityCode: string;
}

export interface IdentityContent {
  contentType: "identity";
  honorificTitle: string;
  firstName: string;
  lastName: string;
  emailAddress: string;
  phoneNumber: string;
  streetAddress: string;
  city: string;
  postalCode: string;
  country: string;
  companyName: string;
  identityDocumentNumber: string;
}

/** Unión discriminada: TypeScript obliga a tratar cada tipo de contenido. */
export type ItemContent = LoginContent | SecureNoteContent | PaymentCardContent | IdentityContent;

export type ItemContentOf<T extends ItemContentType> = Extract<ItemContent, { contentType: T }>;

export interface CustomField {
  fieldLabel: string;
  fieldValue: string;
  isHidden: boolean;
}

/** Campo personalizado en el editor. `sourceFieldIndex` es su posición en el elemento guardado
 *  (null si es nuevo): así el backend recupera un valor oculto enmascarado aunque se borren otros campos. */
export interface CustomFieldDraft extends CustomField {
  sourceFieldIndex: number | null;
}

export interface PasswordHistoryEntry {
  previousPassword: string;
  replacedAt: IsoDateTime;
}

export interface AttachmentReference {
  attachmentId: AttachmentId;
  fileName: string;
  sizeBytes: number;
  addedAt: IsoDateTime;
}

export interface VaultItem {
  itemId: ItemId;
  title: string;
  folderId: FolderId | null;
  tagNames: string[];
  isFavorite: boolean;
  notes: string;
  customFields: CustomField[];
  itemContent: ItemContent;
  passwordHistory: PasswordHistoryEntry[];
  attachments: AttachmentReference[];
  createdAt: IsoDateTime;
  updatedAt: IsoDateTime;
  passwordChangedAt: IsoDateTime | null;
  movedToTrashAt: IsoDateTime | null;
}

export interface SaveVaultItemRequest {
  itemId: ItemId | null;
  title: string;
  folderId: FolderId | null;
  tagNames: string[];
  isFavorite: boolean;
  notes: string;
  customFields: CustomFieldDraft[];
  itemContent: ItemContent;
}

export interface VaultItemSummary {
  itemId: ItemId;
  title: string;
  subtitle: string;
  contentType: ItemContentType;
  folderId: FolderId | null;
  tagNames: string[];
  isFavorite: boolean;
  hasPassword: boolean;
  hasTotp: boolean;
  primaryWebsiteUrl: string | null;
  attachmentCount: number;
  updatedAt: IsoDateTime;
  isInTrash: boolean;
}

export type FolderScope =
  | { scope: "allFolders" }
  | { scope: "withoutFolder" }
  | { scope: "specificFolder"; folderId: FolderId };

export interface VaultItemFilter {
  searchText: string | null;
  folderScope: FolderScope;
  tagName: string | null;
  contentType: ItemContentType | null;
  shouldShowOnlyFavorites: boolean;
  shouldShowTrash: boolean;
}

export interface Folder {
  folderId: FolderId;
  folderName: string;
}

export interface FolderOverview extends Folder {
  itemCount: number;
}

export interface VaultOverview {
  totalItemCount: number;
  favoriteItemCount: number;
  trashItemCount: number;
  itemsWithoutFolderCount: number;
  folders: FolderOverview[];
  tagItemCounts: Record<string, number>;
  contentTypeItemCounts: Partial<Record<ItemContentType, number>>;
}

export type RevealableField =
  | { fieldKind: "username" }
  | { fieldKind: "password" }
  | { fieldKind: "notes" }
  | { fieldKind: "cardNumber" }
  | { fieldKind: "cardSecurityCode" }
  | { fieldKind: "identityDocumentNumber" }
  | { fieldKind: "totpSecret" }
  | { fieldKind: "customField"; fieldIndex: number };

export interface TotpCode {
  currentCode: string;
  secondsUntilRefresh: number;
  periodSeconds: number;
}

export type CharacterSet = "uppercase" | "lowercase" | "digits" | "symbols";

export type PasswordGenerationOptions =
  | {
      strategy: "randomCharacters";
      passwordLength: number;
      characterSets: CharacterSet[];
      shouldExcludeAmbiguousCharacters: boolean;
    }
  | {
      strategy: "passphrase";
      wordCount: number;
      wordSeparator: string;
      shouldCapitalizeWords: boolean;
      shouldAppendNumber: boolean;
    };

export interface PasswordStrength {
  score: 0 | 1 | 2 | 3 | 4;
  estimatedGuessesLog10: number;
  warningMessage: string | null;
  improvementSuggestions: string[];
}

export type PasswordHealthIssue =
  | { issueKind: "weakPassword"; strengthScore: number }
  | { issueKind: "reusedPassword"; reuseCount: number }
  | { issueKind: "oldPassword"; ageInDays: number };

export interface PasswordHealthFinding {
  itemId: ItemId;
  itemTitle: string;
  issue: PasswordHealthIssue;
}

export interface PasswordHealthReport {
  analyzedLoginCount: number;
  healthyLoginCount: number;
  overallHealthScore: number;
  findings: PasswordHealthFinding[];
}

export interface VaultPreferences {
  autoLockAfterIdleSeconds: number;
  clipboardClearAfterSeconds: number;
  passwordHistoryLimit: number;
  shouldLockOnWindowMinimize: boolean;
}

export type ImportFormat = "bitwardenJson" | "bitwardenCsv" | "browserCsv" | "encryptedVaultExport";
export type ExportFormat = "encryptedVaultExport" | "bitwardenJson" | "bitwardenCsv";

export interface ImportSummary {
  importedItemCount: number;
  createdFolderCount: number;
}

export interface ClipboardCopyResult {
  clearAfterSeconds: number;
}

export type MasterPasswordViolation =
  | { kind: "tooShort"; minimumCharacterCount: number }
  | { kind: "tooWeak"; minimumStrengthScore: number };

export interface CommandError {
  errorCode: string;
  retryAfterSeconds?: number;
  masterPasswordViolations?: MasterPasswordViolation[];
  technicalDetail?: string;
}

export type VaultLockReason = "idleTimeout" | "windowMinimized" | "screenLocked" | "systemSuspended";

/** Mismo valor que `REDACTED_SECRET_PLACEHOLDER` en Rust. */
export const REDACTED_SECRET_PLACEHOLDER = "••••••••";

/** Un secreto que la UI recibió enmascarado y aún no se ha revelado ni editado. */
export function isSealedSecret(secretValue: string | null | undefined): boolean {
  return secretValue === REDACTED_SECRET_PLACEHOLDER;
}

/**
 * Convierte un elemento (con secretos enmascarados) en el borrador que se envía al guardar.
 * Los secretos se quedan enmascarados: el backend conserva el valor guardado si vuelven sin tocar.
 */
export function toEditableDraft(vaultItem: VaultItem): SaveVaultItemRequest {
  return {
    itemId: vaultItem.itemId,
    title: vaultItem.title,
    folderId: vaultItem.folderId,
    tagNames: vaultItem.tagNames,
    isFavorite: vaultItem.isFavorite,
    notes: vaultItem.notes,
    customFields: vaultItem.customFields.map((customField, fieldPosition) => ({ ...customField, sourceFieldIndex: fieldPosition })),
    itemContent: vaultItem.itemContent,
  };
}
