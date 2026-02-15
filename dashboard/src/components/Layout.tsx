import { NavLink, Outlet } from "react-router-dom";
import {
  LayoutDashboard,
  ScrollText,
  Shield,
  Settings,
  BarChart3,
  Bot,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { Separator } from "@/components/ui/separator";

const navItems = [
  { to: "/", icon: LayoutDashboard, label: "Dashboard" },
  { to: "/logs", icon: ScrollText, label: "Audit Logs" },
  { to: "/rules", icon: Shield, label: "WAF Rules" },
  { to: "/bots", icon: Bot, label: "Bot Detection" },
  { to: "/config", icon: Settings, label: "Configuration" },
  { to: "/metrics", icon: BarChart3, label: "Metrics" },
];

export function Layout() {
  return (
    <div className="flex h-screen">
      {/* Sidebar */}
      <aside className="w-64 shrink-0 border-r bg-sidebar text-sidebar-foreground flex flex-col">
        <div className="p-4">
          <div className="flex items-center gap-2">
            <Shield className="h-6 w-6 text-sidebar-primary" />
            <h1 className="text-lg font-bold">Layer7 WAF</h1>
          </div>
          <p className="text-xs text-muted-foreground mt-1">Web Application Firewall</p>
        </div>
        <Separator />
        <nav className="flex-1 p-2 space-y-1">
          {navItems.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              end={item.to === "/"}
              className={({ isActive }) =>
                cn(
                  "flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
                  isActive
                    ? "bg-sidebar-accent text-sidebar-accent-foreground"
                    : "text-muted-foreground hover:bg-sidebar-accent/50 hover:text-sidebar-foreground"
                )
              }
            >
              <item.icon className="h-4 w-4" />
              {item.label}
            </NavLink>
          ))}
        </nav>
        <Separator />
        <div className="p-4 text-xs text-muted-foreground">
          v0.1.0
        </div>
      </aside>

      {/* Main content */}
      <main className="flex-1 overflow-auto">
        <div className="p-6">
          <Outlet />
        </div>
      </main>
    </div>
  );
}
