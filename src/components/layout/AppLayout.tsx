import type { ReactNode } from "react";
import type { AppRoute, LocalUser } from "../../types";
import { TopBar } from "./TopBar";

interface AppLayoutProps {
  route: AppRoute;
  children: ReactNode;
  showUsers?: boolean;
  allowedMenuRoutes?: AppRoute[];
  currentUser?: LocalUser | null;
  onSwitchUser?: () => void;
  onNavigate: (route: AppRoute) => void;
}

export function AppLayout({
  route,
  children,
  showUsers = true,
  allowedMenuRoutes,
  currentUser,
  onSwitchUser,
  onNavigate
}: AppLayoutProps) {
  return (
    <div className="app-shell">
      <TopBar
        activeRoute={route}
        showUsers={showUsers}
        allowedMenuRoutes={allowedMenuRoutes}
        currentUser={currentUser}
        onSwitchUser={onSwitchUser}
        onNavigate={onNavigate}
      />
      <main className="app-main">{children}</main>
    </div>
  );
}
