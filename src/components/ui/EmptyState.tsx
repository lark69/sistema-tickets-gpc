import type { ReactNode } from "react";

interface EmptyStateProps {
  title: string;
  action?: ReactNode;
}

export function EmptyState({ title, action }: EmptyStateProps) {
  return (
    <section className="empty-state">
      <div className="empty-state-mark">GPC</div>
      <h2>{title}</h2>
      {action ? <div className="empty-state-action">{action}</div> : null}
    </section>
  );
}
