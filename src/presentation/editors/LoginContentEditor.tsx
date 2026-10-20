import { Plus, Trash2, Wand2 } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { useDebouncedValue } from "../../application/hooks/useDebouncedValue";
import type { LoginContent, PasswordStrength } from "../../domain/vaultTypes";
import { vaultApiClient } from "../../infrastructure/vaultApiClient";
import { PasswordGeneratorPanel } from "../components/PasswordGeneratorPanel";
import { PasswordStrengthMeter } from "../components/PasswordStrengthMeter";
import { Button } from "../components/primitives/Button";
import { FormField } from "../components/primitives/FormField";
import { IconButton } from "../components/primitives/IconButton";
import { ModalDialog } from "../components/primitives/ModalDialog";
import { PasswordInput } from "../components/primitives/PasswordInput";
import { TextInput } from "../components/primitives/TextInput";

interface LoginContentEditorProps {
  itemContent: LoginContent;
  onContentChange: (itemContent: LoginContent) => void;
}

export function LoginContentEditor({ itemContent, onContentChange }: LoginContentEditorProps) {
  const { t: translate } = useTranslation();
  const [isGeneratorOpen, setIsGeneratorOpen] = useState(false);
  const [passwordStrength, setPasswordStrength] = useState<PasswordStrength | null>(null);
  const debouncedPassword = useDebouncedValue(itemContent.password, 200);

  useEffect(() => {
    if (!debouncedPassword) {
      setPasswordStrength(null);
      return;
    }
    vaultApiClient.estimatePasswordStrength(debouncedPassword).then(setPasswordStrength).catch(() => setPasswordStrength(null));
  }, [debouncedPassword]);

  const websiteUrls = itemContent.websiteUrls.length > 0 ? itemContent.websiteUrls : [""];
  const updateWebsiteUrl = (urlPosition: number, websiteUrl: string) =>
    onContentChange({
      ...itemContent,
      websiteUrls: websiteUrls.map((existingUrl, existingPosition) => (existingPosition === urlPosition ? websiteUrl : existingUrl)),
    });
  const removeWebsiteUrl = (urlPosition: number) =>
    onContentChange({ ...itemContent, websiteUrls: websiteUrls.filter((_, existingPosition) => existingPosition !== urlPosition) });

  return (
    <div className="flex flex-col gap-4">
      <FormField fieldLabel={translate("fields.username")}>
        {(fieldControlId) => (
          <TextInput
            id={fieldControlId}
            value={itemContent.username}
            onChange={(changeEvent) => onContentChange({ ...itemContent, username: changeEvent.target.value })}
          />
        )}
      </FormField>

      <FormField fieldLabel={translate("fields.password")}>
        {(fieldControlId) => (
          <div className="flex flex-col gap-2">
            <div className="flex gap-2">
              <div className="flex-1">
                <PasswordInput
                  id={fieldControlId}
                  value={itemContent.password}
                  onChange={(changeEvent) => onContentChange({ ...itemContent, password: changeEvent.target.value })}
                />
              </div>
              <Button leadingIcon={<Wand2 className="size-4" />} onClick={() => setIsGeneratorOpen(true)}>
                {translate("generator.generate")}
              </Button>
            </div>
            <PasswordStrengthMeter passwordStrength={passwordStrength} />
          </div>
        )}
      </FormField>

      <FormField fieldLabel={translate("fields.websiteUrls")}>
        {(fieldControlId) => (
          <div className="flex flex-col gap-2">
            {websiteUrls.map((websiteUrl, urlPosition) => (
              <div key={urlPosition} className="flex gap-1">
                <TextInput
                  id={urlPosition === 0 ? fieldControlId : undefined}
                  inputMode="url"
                  placeholder="https://"
                  value={websiteUrl}
                  onChange={(changeEvent) => updateWebsiteUrl(urlPosition, changeEvent.target.value)}
                />
                {websiteUrls.length > 1 && (
                  <IconButton accessibleLabel={translate("actions.remove")} icon={<Trash2 className="size-4" />} onClick={() => removeWebsiteUrl(urlPosition)} />
                )}
              </div>
            ))}
            <button
              type="button"
              className="flex items-center gap-1 self-start text-xs font-semibold text-accent hover:underline"
              onClick={() => onContentChange({ ...itemContent, websiteUrls: [...websiteUrls, ""] })}
            >
              <Plus className="size-3.5" /> {translate("editor.addWebsite")}
            </button>
          </div>
        )}
      </FormField>

      <FormField fieldLabel={translate("fields.totpSecret")} hintText={translate("editor.totpHint")}>
        {(fieldControlId) => (
          <PasswordInput
            id={fieldControlId}
            placeholder="otpauth://… / JBSWY3DPEHPK3PXP"
            value={itemContent.totpSecret ?? ""}
            onChange={(changeEvent) => onContentChange({ ...itemContent, totpSecret: changeEvent.target.value || null })}
          />
        )}
      </FormField>

      <ModalDialog isOpen={isGeneratorOpen} onOpenChange={setIsGeneratorOpen} dialogTitle={translate("generator.title")}>
        <PasswordGeneratorPanel
          onUseGeneratedPassword={(generatedPassword) => {
            onContentChange({ ...itemContent, password: generatedPassword });
            setIsGeneratorOpen(false);
          }}
        />
      </ModalDialog>
    </div>
  );
}
