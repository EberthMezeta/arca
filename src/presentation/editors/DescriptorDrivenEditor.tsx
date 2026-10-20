import { useTranslation } from "react-i18next";

import { FormField } from "../components/primitives/FormField";
import { PasswordInput } from "../components/primitives/PasswordInput";
import { TextInput } from "../components/primitives/TextInput";
import type { ContentFieldDescriptor } from "./contentFieldDescriptors";

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
        const InputComponent = fieldDescriptor.revealableField ? PasswordInput : TextInput;
        return (
          <div key={fieldDescriptor.fieldKey} className={fieldDescriptor.gridColumnSpan === 2 ? "col-span-2" : ""}>
            <FormField fieldLabel={translate(fieldDescriptor.labelKey)}>
              {(fieldControlId) => (
                <InputComponent
                  id={fieldControlId}
                  inputMode={fieldDescriptor.inputMode}
                  value={String(itemContent[fieldDescriptor.fieldKey] ?? "")}
                  onChange={(changeEvent) => onContentChange({ ...itemContent, [fieldDescriptor.fieldKey]: changeEvent.target.value })}
                />
              )}
            </FormField>
          </div>
        );
      })}
    </div>
  );
}
