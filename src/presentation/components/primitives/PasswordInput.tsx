import { Eye, EyeOff } from "lucide-react";
import { forwardRef, type InputHTMLAttributes, useState } from "react";
import { useTranslation } from "react-i18next";

import { INPUT_BASE_CLASSES } from "./TextInput";

interface PasswordInputProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "type"> {
  isMonospace?: boolean;
}

export const PasswordInput = forwardRef<HTMLInputElement, PasswordInputProps>(function PasswordInput(
  { className = "", isMonospace = true, ...inputAttributes },
  forwardedRef,
) {
  const { t: translate } = useTranslation();
  const [isPasswordVisible, setIsPasswordVisible] = useState(false);
  const visibilityLabel = translate(isPasswordVisible ? "actions.hide" : "actions.show");

  return (
    <div className="relative">
      <input
        ref={forwardedRef}
        type={isPasswordVisible ? "text" : "password"}
        spellCheck={false}
        autoComplete="off"
        autoCapitalize="off"
        className={`h-9 pr-10 ${isMonospace ? "font-mono" : ""} ${INPUT_BASE_CLASSES} ${className}`}
        {...inputAttributes}
      />
      <button
        type="button"
        aria-label={visibilityLabel}
        title={visibilityLabel}
        onClick={() => setIsPasswordVisible((currentVisibility) => !currentVisibility)}
        className="absolute inset-y-0 right-0 flex w-9 items-center justify-center text-muted hover:text-text"
      >
        {isPasswordVisible ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
      </button>
    </div>
  );
});
