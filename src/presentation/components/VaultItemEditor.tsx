import { Plus, Save, Star, Trash2, X } from "lucide-react";
import { type ComponentType, type FormEvent, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { useErrorMessage } from "../../application/hooks/useErrorMessage";
import { useKeyboardShortcuts } from "../../application/hooks/useKeyboardShortcuts";
import { useToastStore } from "../../application/stores/toastStore";
import { type ItemEditorState, useVaultWorkspaceStore } from "../../application/stores/vaultWorkspaceStore";
import type { CommandError, CustomFieldDraft, Folder, ItemContent, ItemContentType, SaveVaultItemRequest } from "../../domain/vaultTypes";
import { vaultApiClient } from "../../infrastructure/vaultApiClient";
import { SealedSecretInput, SealedSecretRevealProvider } from "../editors/SealedSecretInput";
import { type ContentEditorProps, definitionForContentType, ItemTypeBadge } from "../registries/itemContentTypeRegistry";
import { Button } from "./primitives/Button";
import { FormField } from "./primitives/FormField";
import { IconButton } from "./primitives/IconButton";
import { SelectInput } from "./primitives/SelectInput";
import { Spinner } from "./primitives/Spinner";
import { TextArea, TextInput } from "./primitives/TextInput";

type OpenEditorState = Exclude<ItemEditorState, { editorMode: "closed" }>;

function createEmptyDraft(contentType: ItemContentType, preselectedFolderId: string | null): SaveVaultItemRequest {
  return {
    itemId: null,
    title: "",
    folderId: preselectedFolderId,
    tagNames: [],
    isFavorite: false,
    notes: "",
    customFields: [],
    itemContent: definitionForContentType(contentType).createEmptyContent(),
  };
}

/** Carga el borrador inicial según el modo (crear / editar) sin bifurcaciones repartidas. */
const DRAFT_LOADERS: { [Mode in OpenEditorState["editorMode"]]: (editorState: Extract<OpenEditorState, { editorMode: Mode }>, preselectedFolderId: string | null) => Promise<SaveVaultItemRequest> } = {
  creating: async (editorState, preselectedFolderId) => createEmptyDraft(editorState.contentType, preselectedFolderId),
  editing: async (editorState) => vaultApiClient.getVaultItemForEditing(editorState.itemId),
};

function loadDraft(editorState: OpenEditorState, preselectedFolderId: string | null): Promise<SaveVaultItemRequest> {
  const draftLoader = DRAFT_LOADERS[editorState.editorMode] as (state: OpenEditorState, folderId: string | null) => Promise<SaveVaultItemRequest>;
  return draftLoader(editorState, preselectedFolderId);
}

function parseTagNames(rawTagText: string): string[] {
  return rawTagText
    .split(",")
    .map((tagName) => tagName.trim().replace(/^#/, ""))
    .filter(Boolean);
}

export function VaultItemEditor({ editorState }: { editorState: OpenEditorState }) {
  const { t: translate } = useTranslation();
  const itemFilter = useVaultWorkspaceStore((workspaceState) => workspaceState.itemFilter);
  const closeItemEditor = useVaultWorkspaceStore((workspaceState) => workspaceState.closeItemEditor);
  const selectItem = useVaultWorkspaceStore((workspaceState) => workspaceState.selectItem);
  const notifyVaultDataChanged = useVaultWorkspaceStore((workspaceState) => workspaceState.notifyVaultDataChanged);
  const showToast = useToastStore((toastState) => toastState.showToast);
  const describeError = useErrorMessage();

  const [itemDraft, setItemDraft] = useState<SaveVaultItemRequest | null>(null);
  const [tagText, setTagText] = useState("");
  const [availableFolders, setAvailableFolders] = useState<Folder[]>([]);
  const [saveErrorText, setSaveErrorText] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);

  const preselectedFolderId = itemFilter.folderScope.scope === "specificFolder" ? itemFilter.folderScope.folderId : null;

  useEffect(() => {
    let isStillRelevant = true;
    void Promise.all([loadDraft(editorState, preselectedFolderId), vaultApiClient.listFolders()]).then(([initialDraft, folders]) => {
      if (!isStillRelevant) return;
      setItemDraft(initialDraft);
      setTagText(initialDraft.tagNames.join(", "));
      setAvailableFolders(folders);
    });
    return () => {
      isStillRelevant = false;
    };
    // Solo al abrir el editor: cambiar de carpeta en la barra lateral no debe resetear el borrador.
  }, [editorState]);

  const saveDraft = async (formEvent?: FormEvent) => {
    formEvent?.preventDefault();
    if (!itemDraft || isSaving) return;
    setIsSaving(true);
    setSaveErrorText(null);
    try {
      const savedItem = await vaultApiClient.saveVaultItem({ ...itemDraft, tagNames: parseTagNames(tagText) });
      notifyVaultDataChanged();
      selectItem(savedItem.itemId);
      showToast({ message: translate("editor.saved", { itemTitle: savedItem.title }), tone: "success" });
    } catch (commandError) {
      setSaveErrorText(describeError(commandError as CommandError));
    } finally {
      setIsSaving(false);
    }
  };

  useKeyboardShortcuts({ "mod+s": () => void saveDraft(), escape: closeItemEditor });

  if (!itemDraft) {
    return (
      <div className="flex h-full items-center justify-center text-muted">
        <Spinner />
      </div>
    );
  }

  const contentType = itemDraft.itemContent.contentType;
  const contentTypeDefinition = definitionForContentType(contentType);
  const ContentEditor = contentTypeDefinition.ContentEditor as ComponentType<ContentEditorProps<ItemContentType>> | null;
  const updateDraft = (draftChanges: Partial<SaveVaultItemRequest>) => setItemDraft({ ...itemDraft, ...draftChanges });
  const folderOptions = [
    { optionValue: "", optionLabel: translate("navigation.noFolder") },
    ...availableFolders.map((folder) => ({ optionValue: folder.folderId, optionLabel: folder.folderName })),
  ];
  const headerTitleKey = editorState.editorMode === "creating" ? "editor.newItemTitle" : "editor.editItemTitle";

  return (
    <form onSubmit={saveDraft} className="flex h-full flex-col">
      <header className="flex items-center gap-3 border-b border-border bg-surface px-6 py-4">
        <ItemTypeBadge contentType={contentType} sizeClassName="size-11" />
        <div className="min-w-0 flex-1">
          <h2 className="text-lg font-semibold">{translate(headerTitleKey, { itemType: translate(contentTypeDefinition.singularLabelKey) })}</h2>
          <p className="text-xs text-muted">{translate("editor.shortcutsHint")}</p>
        </div>
        <IconButton
          accessibleLabel={translate(itemDraft.isFavorite ? "actions.unfavorite" : "actions.favorite")}
          icon={<Star className={`size-4 ${itemDraft.isFavorite ? "fill-warning text-warning" : ""}`} />}
          onClick={() => updateDraft({ isFavorite: !itemDraft.isFavorite })}
        />
        <Button leadingIcon={<X className="size-4" />} onClick={closeItemEditor}>
          {translate("actions.cancel")}
        </Button>
        <Button type="submit" buttonVariant="primary" isBusy={isSaving} leadingIcon={<Save className="size-4" />}>
          {translate("actions.save")}
        </Button>
      </header>

      <SealedSecretRevealProvider itemId={itemDraft.itemId}>
        <div className="flex-1 overflow-y-auto">
          <div className="mx-auto flex max-w-2xl flex-col gap-6 p-6">
            {saveErrorText && (
              <p role="alert" className="rounded-lg bg-danger-soft px-4 py-3 text-sm text-danger">
                {saveErrorText}
              </p>
            )}

            <FormField fieldLabel={translate("fields.title")}>
              {(fieldControlId) => (
                <TextInput
                  id={fieldControlId}
                  autoFocus
                  required
                  maxLength={200}
                  placeholder={translate("editor.titlePlaceholder")}
                  value={itemDraft.title}
                  onChange={(changeEvent) => updateDraft({ title: changeEvent.target.value })}
                />
              )}
            </FormField>

            {ContentEditor && (
              <ContentEditor
                itemContent={itemDraft.itemContent}
                onContentChange={(itemContent: ItemContent) => updateDraft({ itemContent })}
              />
            )}

            <div className="grid grid-cols-2 gap-4">
              <FormField fieldLabel={translate("fields.folder")}>
                {(fieldControlId) => (
                  <SelectInput
                    id={fieldControlId}
                    selectedValue={itemDraft.folderId ?? ""}
                    selectOptions={folderOptions}
                    onValueChange={(selectedFolderId) => updateDraft({ folderId: selectedFolderId || null })}
                  />
                )}
              </FormField>
              <FormField fieldLabel={translate("fields.tags")} hintText={translate("editor.tagsHint")}>
                {(fieldControlId) => (
                  <TextInput
                    id={fieldControlId}
                    placeholder={translate("editor.tagsPlaceholder")}
                    value={tagText}
                    onChange={(changeEvent) => setTagText(changeEvent.target.value)}
                  />
                )}
              </FormField>
            </div>

            <FormField fieldLabel={translate("fields.notes")}>
              {(fieldControlId) => (
                <TextArea
                  id={fieldControlId}
                  rows={contentType === "secureNote" ? 10 : 4}
                  value={itemDraft.notes}
                  onChange={(changeEvent) => updateDraft({ notes: changeEvent.target.value })}
                />
              )}
            </FormField>

            <CustomFieldsEditor customFields={itemDraft.customFields} onCustomFieldsChange={(customFields) => updateDraft({ customFields })} />
          </div>
        </div>
      </SealedSecretRevealProvider>
    </form>
  );
}

interface CustomFieldsEditorProps {
  customFields: CustomFieldDraft[];
  onCustomFieldsChange: (customFields: CustomFieldDraft[]) => void;
}

function CustomFieldsEditor({ customFields, onCustomFieldsChange }: CustomFieldsEditorProps) {
  const { t: translate } = useTranslation();
  const updateCustomField = (fieldPosition: number, fieldChanges: Partial<CustomFieldDraft>) =>
    onCustomFieldsChange(
      customFields.map((customField, existingPosition) => (existingPosition === fieldPosition ? { ...customField, ...fieldChanges } : customField)),
    );

  return (
    <section className="flex flex-col gap-3">
      <h3 className="text-xs font-semibold tracking-wide text-muted uppercase">{translate("editor.customFields")}</h3>
      {customFields.map((customField, fieldPosition) => {
        const updateFieldValue = (fieldValue: string) => updateCustomField(fieldPosition, { fieldValue });
        return (
          <div key={fieldPosition} className="flex items-center gap-2">
            <TextInput
              className="w-40"
              aria-label={translate("editor.customFieldLabel")}
              placeholder={translate("editor.customFieldLabel")}
              value={customField.fieldLabel}
              onChange={(changeEvent) => updateCustomField(fieldPosition, { fieldLabel: changeEvent.target.value })}
            />
            <div className="flex-1">
              {customField.isHidden ? (
                <SealedSecretInput
                  aria-label={translate("editor.customFieldValue")}
                  placeholder={translate("editor.customFieldValue")}
                  value={customField.fieldValue}
                  revealableField={
                    customField.sourceFieldIndex === null ? undefined : { fieldKind: "customField", fieldIndex: customField.sourceFieldIndex }
                  }
                  onValueChange={updateFieldValue}
                />
              ) : (
                <TextInput
                  aria-label={translate("editor.customFieldValue")}
                  placeholder={translate("editor.customFieldValue")}
                  value={customField.fieldValue}
                  onChange={(changeEvent) => updateFieldValue(changeEvent.target.value)}
                />
              )}
            </div>
            <label className="flex items-center gap-1.5 text-xs text-muted">
              <input
                type="checkbox"
                className="accent-[var(--accent)]"
                checked={customField.isHidden}
                onChange={(changeEvent) => updateCustomField(fieldPosition, { isHidden: changeEvent.target.checked })}
              />
              {translate("editor.hiddenField")}
            </label>
            <IconButton
              accessibleLabel={translate("actions.remove")}
              icon={<Trash2 className="size-4" />}
              onClick={() => onCustomFieldsChange(customFields.filter((_, existingPosition) => existingPosition !== fieldPosition))}
            />
          </div>
        );
      })}
      <button
        type="button"
        className="flex items-center gap-1 self-start text-xs font-semibold text-accent hover:underline"
        onClick={() => onCustomFieldsChange([...customFields, { fieldLabel: "", fieldValue: "", isHidden: false, sourceFieldIndex: null }])}
      >
        <Plus className="size-3.5" /> {translate("editor.addCustomField")}
      </button>
    </section>
  );
}
