import { useTranslation } from "react-i18next";

import { FormField } from "../components/primitives/FormField";
import { TextInput } from "../components/primitives/TextInput";
import type { ContentFieldDescriptor } from "./contentFieldDescriptors";
import { SealedSecretInput } from "./SealedSecretInput";

interface DescriptorDrivenEditorProps<Content> {
  itemContent: Content;
  fieldDescriptors: ContentFieldDescriptor<Content>[];
  onContentChange: (itemContent: Content) => void;
}

/** Genera el formulario a partir de los descriptores (los secretos usan campo oculto). */
export function DescriptorDrivenEditor<Content>({ itemContent, fieldDescriptors, onContentChange }: DescriptorDrivenEditorProps<Content>) {
  const { t: translate } = useTranslation();
  return (
    <div className="grid grid-cols-2 gap-4">
      {fieldDescriptors.map((fieldDescriptor) => {
        const fieldValue = String(itemContent[fieldDescriptor.fieldKey] ?? "");
        const updateFieldValue = (newFieldValue: string) => onContentChange({ ...itemContent, [fieldDescriptor.fieldKey]: newFieldValue });
        return (
          <div key={fieldDescriptor.fieldKey} className={fieldDescriptor.gridColumnSpan === 2 ? "col-span-2" : ""}>
            <FormField fieldLabel={translate(fieldDescriptor.labelKey)}>
              {(fieldControlId) =>
                fieldDescriptor.revealableField ? (
                  <SealedSecretInput
                    id={fieldControlId}
                    inputMode={fieldDescriptor.inputMode}
                    value={fieldValue}
                    revealableField={fieldDescriptor.revealableField}
                    onValueChange={updateFieldValue}
                  />
                ) : (
                  <TextInput
                    id={fieldControlId}
                    inputMode={fieldDescriptor.inputMode}
                    value={fieldValue}
                    onChange={(changeEvent) => updateFieldValue(changeEvent.target.value)}
                  />
                )
              }
            </FormField>
          </div>
        );
      })}
    </div>
  );
}
