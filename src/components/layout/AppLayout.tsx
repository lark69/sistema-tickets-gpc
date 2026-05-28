import type { ReactNode } from "react";
import type { AppRoute } from "../../types";
import { TopBar } from "./TopBar";

interface AppLayoutProps {
  route: AppRoute;
  children: ReactNode;
  onNavigate: (route: AppRoute) => void;
}

export function AppLayout({ route, children, onNavigate }: AppLayoutProps) {
  return (
    <div className="app-shell">
      <TopBar activeRoute={route} onNavigate={onNavigate} />
      <main className="app-main">{children}</main>
    </div>
  );
}
