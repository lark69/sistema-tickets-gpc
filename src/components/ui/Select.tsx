import type { SelectHTMLAttributes } from "react";

interface SelectOption {
  value: string;
  label: string;
}

interface SelectProps extends SelectHTMLAttributes<HTMLSelectElement> {
  label: string;
  options: SelectOption[];
  hint?: string;
}

export function Select({ label, options, hint, className = "", ...props }: SelectProps) {
  return (
    <label className={`field ${className}`}>
      <span className="field-label">{label}</span>
      <span className="field-control">
        <select {...props}>
          {options.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      </span>
      {hint ? <span className="field-hint">{hint}</span> : null}
    </label>
  );
}
