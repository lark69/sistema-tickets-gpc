import { BadgeCheck, ClipboardList, Gauge, Package, Settings } from "lucide-react";
import type { AppRoute } from "../../types";

interface TopBarProps {
  activeRoute: AppRoute;
  onNavigate: (route: AppRoute) => void;
}

export function TopBar({ activeRoute, onNavigate }: TopBarProps) {
  return (
    <header className="topbar">
      <div className="topbar-brand">
        <span className="brand-symbol">GPC</span>
        <span>Sistema de Tickets GPC</span>
      </div>
      <nav className="topbar-nav" aria-label="Navegação principal">
        <button
          type="button"
          className={activeRoute === "home" ? "active" : ""}
          onClick={() => onNavigate("home")}
        >
          <Gauge size={18} />
          <span>PDV</span>
        </button>
        <button
          type="button"
          className={activeRoute === "dashboard" ? "active" : ""}
          onClick={() => onNavigate("dashboard")}
        >
          <Package size={18} />
          <span>Produtos</span>
        </button>
        <button
          type="button"
          className={activeRoute === "logs" ? "active" : ""}
          onClick={() => onNavigate("logs")}
        >
          <ClipboardList size={18} />
          <span>Logs</span>
        </button>
        <button
          type="button"
          className={activeRoute === "verify-ticket" ? "active" : ""}
          onClick={() => onNavigate("verify-ticket")}
        >
          <BadgeCheck size={18} />
          <span>Verificar</span>
        </button>
        <button
          type="button"
          className={activeRoute === "settings" ? "active" : ""}
          onClick={() => onNavigate("settings")}
        >
          <Settings size={18} />
          <span>Configurações</span>
        </button>
      </nav>
    </header>
  );
}
