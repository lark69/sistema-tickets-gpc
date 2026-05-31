import {
  BadgeCheck,
  BarChart3,
  Boxes,
  ClipboardList,
  Gauge,
  LogOut,
  Menu,
  Package,
  Settings,
  Users,
  Wallet
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { AppRoute, LocalUser } from "../../types";

interface TopBarProps {
  activeRoute: AppRoute;
  showUsers?: boolean;
  allowedMenuRoutes?: AppRoute[];
  currentUser?: LocalUser | null;
  onSwitchUser?: () => void;
  onNavigate: (route: AppRoute) => void;
}

export function TopBar({
  activeRoute,
  showUsers = true,
  allowedMenuRoutes,
  currentUser,
  onSwitchUser,
  onNavigate
}: TopBarProps) {
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement | null>(null);
  const menuRoutes = useMemo(() => {
    const routes = [
      { route: "inventory" as const, label: "Estoque", icon: <Boxes size={18} /> },
      { route: "reports" as const, label: "Relatorios", icon: <BarChart3 size={18} /> },
      { route: "logs" as const, label: "Logs", icon: <ClipboardList size={18} /> },
      { route: "verify-ticket" as const, label: "Verificar", icon: <BadgeCheck size={18} /> },
      { route: "settings" as const, label: "Config", icon: <Settings size={18} /> },
      ...(showUsers ? [{ route: "users" as const, label: "Usuarios", icon: <Users size={18} /> }] : [])
    ];

    return allowedMenuRoutes
      ? routes.filter((item) => allowedMenuRoutes.includes(item.route))
      : routes;
  }, [allowedMenuRoutes, showUsers]);
  const menuActive = menuRoutes.some((item) => item.route === activeRoute);

  useEffect(() => {
    function handlePointerDown(event: MouseEvent) {
      if (!menuRef.current?.contains(event.target as Node)) {
        setMenuOpen(false);
      }
    }

    if (menuOpen) {
      window.addEventListener("mousedown", handlePointerDown);
    }

    return () => window.removeEventListener("mousedown", handlePointerDown);
  }, [menuOpen]);

  function navigate(route: AppRoute) {
    setMenuOpen(false);
    onNavigate(route);
  }

  return (
    <header className="topbar">
      <div className="topbar-brand">
        <span className="brand-symbol">PDV</span>
        <span>Portex PDV</span>
      </div>
      <nav className="topbar-nav" aria-label="Navegacao principal">
        <button type="button" className={activeRoute === "home" ? "active" : ""} onClick={() => navigate("home")}>
          <Gauge size={18} />
          <span>PDV</span>
        </button>
        <button type="button" className={activeRoute === "cash" ? "active" : ""} onClick={() => navigate("cash")}>
          <Wallet size={18} />
          <span>Caixa</span>
        </button>
        <button type="button" className={activeRoute === "dashboard" ? "active" : ""} onClick={() => navigate("dashboard")}>
          <Package size={18} />
          <span>Produtos</span>
        </button>
        <div className="topbar-menu-container" ref={menuRef}>
          <button
            type="button"
            className={menuActive || menuOpen ? "active" : ""}
            aria-haspopup="menu"
            aria-expanded={menuOpen}
            onClick={() => setMenuOpen((current) => !current)}
          >
            <Menu size={18} />
            <span>Menu</span>
          </button>
          {menuOpen ? (
            <div className="topbar-menu-panel" role="menu">
              {currentUser ? (
                <div className="topbar-menu-user">
                  <strong>{currentUser.username}</strong>
                  <span>{currentUser.role === "admin" ? "Administrador" : "Operador/Caixa"}</span>
                </div>
              ) : null}
              {menuRoutes.map((item) => (
                <button
                  key={item.route}
                  type="button"
                  role="menuitem"
                  className={activeRoute === item.route ? "active" : ""}
                  onClick={() => navigate(item.route)}
                >
                  {item.icon}
                  <span>{item.label}</span>
                </button>
              ))}
              {onSwitchUser ? (
                <button type="button" role="menuitem" onClick={onSwitchUser}>
                  <LogOut size={18} />
                  <span>Alternar usuario</span>
                </button>
              ) : null}
            </div>
          ) : null}
        </div>
      </nav>
    </header>
  );
}
