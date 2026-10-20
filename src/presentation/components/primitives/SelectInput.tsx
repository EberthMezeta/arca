import type { SelectHTMLAttributes } from "react";

import { INPUT_BASE_CLASSES } from "./TextInput";

export interface SelectOption<Value extends string> {
  optionValue: Value;
  optionLabel: string;
}

interface SelectInputProps<Value extends string> extends Omit<SelectHTMLAttributes<HTMLSelectElement>, "onChange" | "value"> {
  selectedValue: Value;
  selectOptions: readonly SelectOption<Value>[];
  onValueChange: (selectedValue: Value) => void;
}

export function SelectInput<Value extends string>({
  selectedValue,
  selectOptions,
  onValueChange,
  className = "",
  ...selectAttributes
}: SelectInputProps<Value>) {
  return (
    <select
      value={selectedValue}
      onChange={(changeEvent) => onValueChange(changeEvent.target.value as Value)}
      className={`h-9 ${INPUT_BASE_CLASSES} ${className}`}
      {...selectAttributes}
    >
      {selectOptions.map((selectOption) => (
        <option key={selectOption.optionValue} value={selectOption.optionValue}>
          {selectOption.optionLabel}
        </option>
      ))}
    </select>
  );
}
