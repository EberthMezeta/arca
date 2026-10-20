import { useTranslation } from "react-i18next";

import type { ItemId } from "../../domain/vaultTypes";
import { PlainDetailRow, SecretDetailRow } from "../components/DetailRows";
import type { ContentFieldDescriptor } from "./contentFieldDescriptors";

interface DescriptorDrivenDetailsProps<Content> {
  itemId: ItemId;
  itemContent: Content;
  fieldDescriptors: ContentFieldDescriptor<Content>[];
}

/** Muestra solo los campos con valor; los secretos llegan enmascarados y se revelan bajo demanda. */
export function DescriptorDrivenDetails<Content>({ itemId, itemContent, fieldDescriptors }: DescriptorDrivenDetailsProps<Content>) {
  const { t: translate } = useTranslation();
  return (
    <>
      {fieldDescriptors
        .map((fieldDescriptor) => ({ fieldDescriptor, fieldValue: String(itemContent[fieldDescriptor.fieldKey] ?? "") }))
        .filter(({ fieldValue }) => fieldValue.trim() !== "")
        .map(({ fieldDescriptor, fieldValue }) =>
          fieldDescriptor.revealableField ? (
            <SecretDetailRow
              key={fieldDescriptor.fieldKey}
              itemId={itemId}
              rowLabel={translate(fieldDescriptor.labelKey)}
              revealableField={fieldDescriptor.revealableField}
              maskedValue={fieldValue}
            />
          ) : (
            <PlainDetailRow key={fieldDescriptor.fieldKey} itemId={itemId} rowLabel={translate(fieldDescriptor.labelKey)} displayedValue={fieldValue} />
          ),
        )}
    </>
  );
}
