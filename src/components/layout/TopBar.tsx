import { BadgeCheck, Gauge, Home, Settings } from "lucide-react";
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
          <Home size={18} />
          <span>Home</span>
        </button>
        <button
          type="button"
          className={activeRoute === "dashboard" ? "active" : ""}
          onClick={() => onNavigate("dashboard")}
        >
          <Gauge size={18} />
          <span>Dashboard</span>
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
