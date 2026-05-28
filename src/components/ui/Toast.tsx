import { CheckCircle2, Info, TriangleAlert, X } from "lucide-react";
import type { ToastState } from "../../types";

interface ToastProps {
  toast: ToastState | null;
  onDismiss: () => void;
}

export function Toast({ toast, onDismiss }: ToastProps) {
  if (!toast) {
    return null;
  }

  const Icon = toast.tone === "success" ? CheckCircle2 : toast.tone === "error" ? TriangleAlert : Info;

  return (
    <div className={`toast toast-${toast.tone}`} role="status">
      <Icon size={18} />
      <span>{toast.message}</span>
      <button type="button" className="toast-close" aria-label="Dispensar" onClick={onDismiss}>
        <X size={16} />
      </button>
    </div>
  );
}
