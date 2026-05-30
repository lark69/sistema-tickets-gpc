import {
  BadgeCheck,
  BarChart3,
  Boxes,
  ClipboardList,
  Gauge,
  Package,
  Settings,
  Users,
  Wallet
} from "lucide-react";
import type { AppRoute } from "../../types";

interface TopBarProps {
  activeRoute: AppRoute;
  showUsers?: boolean;
  onNavigate: (route: AppRoute) => void;
}

export function TopBar({ activeRoute, showUsers = true, onNavigate }: TopBarProps) {
  return (
    <header className="topbar">
      <div className="topbar-brand">
        <span className="brand-symbol">PDV</span>
        <span>Portex PDV</span>
      </div>
      <nav className="topbar-nav" aria-label="Navegacao principal">
        <button type="button" className={activeRoute === "home" ? "active" : ""} onClick={() => onNavigate("home")}>
          <Gauge size={18} />
          <span>PDV</span>
        </button>
        <button type="button" className={activeRoute === "dashboard" ? "active" : ""} onClick={() => onNavigate("dashboard")}>
          <Package size={18} />
          <span>Produtos</span>
        </button>
        <button type="button" className={activeRoute === "cash" ? "active" : ""} onClick={() => onNavigate("cash")}>
          <Wallet size={18} />
          <span>Caixa</span>
        </button>
        <button type="button" className={activeRoute === "inventory" ? "active" : ""} onClick={() => onNavigate("inventory")}>
          <Boxes size={18} />
          <span>Estoque</span>
        </button>
        <button type="button" className={activeRoute === "reports" ? "active" : ""} onClick={() => onNavigate("reports")}>
          <BarChart3 size={18} />
          <span>Relatorios</span>
        </button>
        <button type="button" className={activeRoute === "logs" ? "active" : ""} onClick={() => onNavigate("logs")}>
          <ClipboardList size={18} />
          <span>Logs</span>
        </button>
        <button type="button" className={activeRoute === "verify-ticket" ? "active" : ""} onClick={() => onNavigate("verify-ticket")}>
          <BadgeCheck size={18} />
          <span>Verificar</span>
        </button>
        <button type="button" className={activeRoute === "settings" ? "active" : ""} onClick={() => onNavigate("settings")}>
          <Settings size={18} />
          <span>Config</span>
        </button>
        {showUsers ? (
          <button type="button" className={activeRoute === "users" ? "active" : ""} onClick={() => onNavigate("users")}>
            <Users size={18} />
            <span>Usuarios</span>
          </button>
        ) : null}
      </nav>
    </header>
  );
}
