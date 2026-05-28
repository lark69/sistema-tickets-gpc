import type { InputHTMLAttributes, ReactNode, TextareaHTMLAttributes } from "react";

interface BaseProps {
  label: string;
  error?: string;
  hint?: string;
  icon?: ReactNode;
}

interface InputProps extends BaseProps, InputHTMLAttributes<HTMLInputElement> {
  multiline?: false;
}

interface TextareaProps extends BaseProps, TextareaHTMLAttributes<HTMLTextAreaElement> {
  multiline: true;
}

type TextInputProps = InputProps | TextareaProps;

export function TextInput(props: TextInputProps) {
  const { label, error, hint, icon, multiline, className = "", ...fieldProps } = props;

  return (
    <label className={`field ${className}`}>
      <span className="field-label">{label}</span>
      <span className={`field-control ${error ? "field-control-error" : ""}`}>
        {icon ? <span className="field-icon">{icon}</span> : null}
        {multiline ? (
          <textarea {...(fieldProps as TextareaHTMLAttributes<HTMLTextAreaElement>)} />
        ) : (
          <input {...(fieldProps as InputHTMLAttributes<HTMLInputElement>)} />
        )}
      </span>
      {error ? <span className="field-error">{error}</span> : null}
      {!error && hint ? <span className="field-hint">{hint}</span> : null}
    </label>
  );
}
