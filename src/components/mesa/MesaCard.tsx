import type { Mesa } from "../../types";

interface MesaCardProps {
  mesa: Mesa;
  now: number;
  onClick: (mesa: Mesa) => void;
}

export function MesaCard({ mesa, now, onClick }: MesaCardProps) {
  const elapsed = mesa.tempoInicio ? formatElapsed(now - mesa.tempoInicio) : null;

  return (
    <button
      type="button"
      className={`mesa-card mesa-card-${mesa.status}`}
      onClick={() => onClick(mesa)}
    >
      <span className="mesa-status-dot" />
      <strong>{String(mesa.numero).padStart(2, "0")}</strong>
      <span>{mesa.status === "ativa" ? "Ativa" : "Livre"}</span>
      {elapsed && mesa.status === "ativa" ? <em>{elapsed}</em> : null}
    </button>
  );
}

function formatElapsed(durationMillis: number): string {
  const totalSeconds = Math.max(0, Math.floor(durationMillis / 1000));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}
