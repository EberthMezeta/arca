import { Download, History, MousePointerClick, Paperclip, Pencil, RotateCcw, Star, Trash2, X } from "lucide-react";
import { type ComponentType, type ReactNode, useState } from "react";
import { useTranslation } from "react-i18next";

import { useAsyncData } from "../../application/hooks/useAsyncData";
import { useErrorMessage } from "../../application/hooks/useErrorMessage";
import { useToastStore } from "../../application/stores/toastStore";
import { useVaultWorkspaceStore } from "../../application/stores/vaultWorkspaceStore";
import type { CommandError, ItemContentType, VaultItem } from "../../domain/vaultTypes";
import { vaultApiClient } from "../../infrastructure/vaultApiClient";
import { type ContentDetailsProps, definitionForContentType, ItemTypeBadge } from "../registries/itemContentTypeRegistry";
import { PlainDetailRow, SecretDetailRow } from "./DetailRows";
import { Button } from "./primitives/Button";
import { ConfirmDialog } from "./primitives/ConfirmDialog";
import { EmptyState } from "./primitives/EmptyState";
import { IconButton } from "./primitives/IconButton";
import { Spinner } from "./primitives/Spinner";

export function formatFileSize(sizeBytes: number): string {
  const sizeUnits = ["B", "KB", "MB", "GB"];
  const unitIndex = Math.min(sizeUnits.length - 1, Math.floor(Math.log(Math.max(sizeBytes, 1)) / Math.log(1024)));
  return `${(sizeBytes / 1024 ** unitIndex).toFixed(unitIndex === 0 ? 0 : 1)} ${sizeUnits[unitIndex]}`;
}

function DetailSection({ sectionTitle, children }: { sectionTitle?: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-1.5">
      {sectionTitle && <h3 className="px-1 text-xs font-semibold tracking-wide text-muted uppercase">{sectionTitle}</h3>}
      <div className="divide-y divide-border overflow-hidden rounded-xl border border-border bg-surface">{children}</div>
    </section>
  );
}

/** Renderiza el detalle específico del tipo a través del registro. */
function TypeSpecificDetails({ vaultItem }: { vaultItem: VaultItem }) {
  const { ContentDetails } = definitionForContentType(vaultItem.itemContent.contentType);
  if (!ContentDetails) return null;
  const TypedContentDetails = ContentDetails as ComponentType<ContentDetailsProps<ItemContentType>>;
  return <TypedContentDetails itemId={vaultItem.itemId} itemContent={vaultItem.itemContent} />;
}

export function VaultItemDetails() {
  const { t: translate, i18n } = useTranslation();
  const selectedItemId = useVaultWorkspaceStore((workspaceState) => workspaceState.selectedItemId);
  const vaultDataRevision = useVaultWorkspaceStore((workspaceState) => workspaceState.vaultDataRevision);
  const { loadedData: vaultItem, isLoading } = useAsyncData(
    () => (selectedItemId ? vaultApiClient.getVaultItemDetails(selectedItemId) : Promise.resolve(null)),
    [selectedItemId, vaultDataRevision],
  );

  if (!selectedItemId) {
    return (
      <EmptyState
        illustrationIcon={<MousePointerClick className="size-7" />}
        headline={translate("emptyStates.nothingSelectedTitle")}
        explanation={translate("emptyStates.nothingSelectedText")}
      />
    );
  }
  if (!vaultItem || vaultItem.itemId !== selectedItemId) {
    return <div className="flex h-full items-center justify-center text-muted">{isLoading && <Spinner />}</div>;
  }

  const formatDateTime = (isoDateTime: string) =>
    new Intl.DateTimeFormat(i18n.language, { dateStyle: "medium", timeStyle: "short" }).format(new Date(isoDateTime));
  const visibleCustomFields = vaultItem.customFields
    .map((customField, fieldIndex) => ({ customField, fieldIndex }))
    .filter(({ customField }) => customField.fieldLabel || customField.fieldValue);

  return (
    <div className="flex h-full flex-col">
      <ItemDetailsHeader vaultItem={vaultItem} />
      <div className="flex-1 overflow-y-auto">
        <div className="mx-auto flex max-w-2xl flex-col gap-5 p-6">
          <DetailSection>
            <TypeSpecificDetails vaultItem={vaultItem} />
            {vaultItem.notes && vaultItem.itemContent.contentType === "secureNote" && (
              <div className="px-4 py-3 text-sm whitespace-pre-wrap" data-selectable>
                {vaultItem.notes}
              </div>
            )}
          </DetailSection>

          {visibleCustomFields.length > 0 && (
            <DetailSection sectionTitle={translate("editor.customFields")}>
              {visibleCustomFields.map(({ customField, fieldIndex }) =>
                customField.isHidden ? (
                  <SecretDetailRow
                    key={fieldIndex}
                    itemId={vaultItem.itemId}
                    rowLabel={customField.fieldLabel}
                    revealableField={{ fieldKind: "customField", fieldIndex }}
                    maskedValue={customField.fieldValue}
                  />
                ) : (
                  <PlainDetailRow key={fieldIndex} itemId={vaultItem.itemId} rowLabel={customField.fieldLabel} displayedValue={customField.fieldValue} />
                ),
              )}
            </DetailSection>
          )}

          {vaultItem.notes && vaultItem.itemContent.contentType !== "secureNote" && (
            <DetailSection sectionTitle={translate("fields.notes")}>
              <div className="px-4 py-3 text-sm whitespace-pre-wrap" data-selectable>
                {vaultItem.notes}
              </div>
            </DetailSection>
          )}

          <AttachmentsSection vaultItem={vaultItem} />

          <div className="flex flex-col gap-1 px-1 text-xs text-muted">
            {vaultItem.tagNames.length > 0 && (
              <div className="mb-2 flex flex-wrap gap-1.5">
                {vaultItem.tagNames.map((tagName) => (
                  <span key={tagName} className="rounded-full bg-surface-muted px-2 py-0.5 text-xs font-medium text-text">
                    #{tagName}
                  </span>
                ))}
              </div>
            )}
            <span>{translate("details.updatedAt", { dateTime: formatDateTime(vaultItem.updatedAt) })}</span>
            <span>{translate("details.createdAt", { dateTime: formatDateTime(vaultItem.createdAt) })}</span>
            {vaultItem.passwordHistory.length > 0 && (
              <span className="flex items-center gap-1">
                <History className="size-3.5" />
                {translate("details.passwordHistoryCount", { count: vaultItem.passwordHistory.length })}
              </span>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

function ItemDetailsHeader({ vaultItem }: { vaultItem: VaultItem }) {
  const { t: translate } = useTranslation();
  const startEditingItem = useVaultWorkspaceStore((workspaceState) => workspaceState.startEditingItem);
  const selectItem = useVaultWorkspaceStore((workspaceState) => workspaceState.selectItem);
  const notifyVaultDataChanged = useVaultWorkspaceStore((workspaceState) => workspaceState.notifyVaultDataChanged);
  const showToast = useToastStore((toastState) => toastState.showToast);
  const describeError = useErrorMessage();
  const [isDeleteConfirmationOpen, setIsDeleteConfirmationOpen] = useState(false);
  const isInTrash = vaultItem.movedToTrashAt !== null;

  const runItemAction = async (itemAction: () => Promise<void>, successMessage?: string, undoAction?: () => Promise<void>) => {
    try {
      await itemAction();
      notifyVaultDataChanged();
      if (!successMessage) return;
      showToast({
        message: successMessage,
        tone: "success",
        durationMilliseconds: undoAction ? 6000 : 3000,
        toastAction: undoAction
          ? { actionLabel: translate("actions.undo"), runAction: () => void undoAction().then(notifyVaultDataChanged) }
          : null,
      });
    } catch (commandError) {
      showToast({ message: describeError(commandError as CommandError), tone: "error" });
    }
  };

  const toggleFavorite = () =>
    runItemAction(async () => {
      const editableDraft = await vaultApiClient.getVaultItemForEditing(vaultItem.itemId);
      await vaultApiClient.saveVaultItem({ ...editableDraft, isFavorite: !editableDraft.isFavorite });
    });

  const moveToTrash = () =>
    runItemAction(
      async () => {
        await vaultApiClient.moveItemToTrash(vaultItem.itemId);
        selectItem(null);
      },
      translate("details.movedToTrash", { itemTitle: vaultItem.title }),
      () => vaultApiClient.restoreItemFromTrash(vaultItem.itemId),
    );

  return (
    <header className="flex items-center gap-3 border-b border-border bg-surface px-6 py-4">
      <ItemTypeBadge contentType={vaultItem.itemContent.contentType} sizeClassName="size-11" />
      <div className="min-w-0 flex-1">
        <h2 className="truncate text-lg font-semibold" data-selectable>
          {vaultItem.title}
        </h2>
        <p className="text-xs text-muted">{translate(definitionForContentType(vaultItem.itemContent.contentType).singularLabelKey)}</p>
      </div>
      {isInTrash ? (
        <>
          <Button
            leadingIcon={<RotateCcw className="size-4" />}
            onClick={() => runItemAction(() => vaultApiClient.restoreItemFromTrash(vaultItem.itemId), translate("details.restored"))}
          >
            {translate("actions.restore")}
          </Button>
          <Button buttonVariant="danger" leadingIcon={<Trash2 className="size-4" />} onClick={() => setIsDeleteConfirmationOpen(true)}>
            {translate("actions.deleteForever")}
          </Button>
        </>
      ) : (
        <>
          <IconButton
            accessibleLabel={translate(vaultItem.isFavorite ? "actions.unfavorite" : "actions.favorite")}
            icon={<Star className={`size-4 ${vaultItem.isFavorite ? "fill-warning text-warning" : ""}`} />}
            onClick={toggleFavorite}
          />
          <IconButton accessibleLabel={translate("actions.moveToTrash")} icon={<Trash2 className="size-4" />} onClick={moveToTrash} />
          <Button buttonVariant="primary" leadingIcon={<Pencil className="size-4" />} onClick={() => startEditingItem(vaultItem.itemId)}>
            {translate("actions.edit")}
          </Button>
        </>
      )}
      <ConfirmDialog
        isOpen={isDeleteConfirmationOpen}
        onOpenChange={setIsDeleteConfirmationOpen}
        dialogTitle={translate("details.deleteForeverTitle")}
        dialogDescription={translate("details.deleteForeverText", { itemTitle: vaultItem.title })}
        confirmLabel={translate("actions.deleteForever")}
        isDestructive
        onConfirm={() =>
          void runItemAction(async () => {
            await vaultApiClient.deleteItemPermanently(vaultItem.itemId);
            selectItem(null);
          }, translate("details.deletedForever"))
        }
      />
    </header>
  );
}

function AttachmentsSection({ vaultItem }: { vaultItem: VaultItem }) {
  const { t: translate } = useTranslation();
  const notifyVaultDataChanged = useVaultWorkspaceStore((workspaceState) => workspaceState.notifyVaultDataChanged);
  const showToast = useToastStore((toastState) => toastState.showToast);
  const describeError = useErrorMessage();
  const [isAddingAttachment, setIsAddingAttachment] = useState(false);
  const isInTrash = vaultItem.movedToTrashAt !== null;

  const reportError = (commandError: unknown) => showToast({ message: describeError(commandError as CommandError), tone: "error" });

  const addAttachment = async () => {
    setIsAddingAttachment(true);
    try {
      const addedAttachment = await vaultApiClient.addAttachmentFromFile(vaultItem.itemId, translate("attachments.chooseFile"));
      if (!addedAttachment) return;
      notifyVaultDataChanged();
      showToast({ message: translate("attachments.added", { fileName: addedAttachment.fileName }), tone: "success" });
    } catch (commandError) {
      reportError(commandError);
    } finally {
      setIsAddingAttachment(false);
    }
  };

  if (vaultItem.attachments.length === 0 && isInTrash) return null;

  return (
    <section className="flex flex-col gap-1.5">
      <div className="flex items-center justify-between px-1">
        <h3 className="text-xs font-semibold tracking-wide text-muted uppercase">{translate("attachments.title")}</h3>
        {!isInTrash && (
          <button
            type="button"
            disabled={isAddingAttachment}
            onClick={addAttachment}
            className="flex items-center gap-1 text-xs font-semibold text-accent hover:underline disabled:opacity-50"
          >
            <Paperclip className="size-3.5" /> {translate("attachments.add")}
          </button>
        )}
      </div>
      {vaultItem.attachments.length === 0 ? (
        <p className="rounded-xl border border-dashed border-border px-4 py-3 text-xs text-muted">{translate("attachments.none")}</p>
      ) : (
        <ul className="divide-y divide-border overflow-hidden rounded-xl border border-border bg-surface">
          {vaultItem.attachments.map((attachment) => (
            <li key={attachment.attachmentId} className="flex items-center gap-3 px-4 py-2.5">
              <Paperclip className="size-4 text-muted" />
              <div className="min-w-0 flex-1">
                <div className="truncate text-sm">{attachment.fileName}</div>
                <div className="text-xs text-muted">{formatFileSize(attachment.sizeBytes)}</div>
              </div>
              <IconButton
                accessibleLabel={translate("attachments.saveCopy")}
                icon={<Download className="size-4" />}
                onClick={() =>
                  vaultApiClient
                    .saveAttachmentToFile(vaultItem.itemId, attachment.attachmentId, translate("attachments.saveCopy"))
                    .then((wasSaved) => wasSaved && showToast({ message: translate("attachments.saved"), tone: "success" }))
                    .catch(reportError)
                }
              />
              {!isInTrash && (
                <IconButton
                  accessibleLabel={translate("actions.remove")}
                  icon={<X className="size-4" />}
                  onClick={() =>
                    vaultApiClient.deleteAttachment(vaultItem.itemId, attachment.attachmentId).then(notifyVaultDataChanged).catch(reportError)
                  }
                />
              )}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
