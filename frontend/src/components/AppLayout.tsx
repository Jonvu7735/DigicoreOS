import type { ReactNode } from "react";
import { Link, NavLink, Outlet } from "react-router-dom";

import { useAuth } from "../auth/useAuth";
import {
  IconBell,
  IconBolt,
  IconBox,
  IconGift,
  IconGrid,
  IconPlay,
  IconSearch,
  IconSparkles,
} from "./icons";

type Item = { to: string; label: string; icon: ReactNode; end?: boolean };

const GROUPS: { title: string; items: Item[] }[] = [
  {
    title: "Menu",
    items: [{ to: "/", label: "Tổng quan", icon: <IconGrid />, end: true }],
  },
  {
    title: "Nghiệp vụ",
    items: [
      { to: "/loyalty", label: "Loyalty", icon: <IconGift /> },
      { to: "/shipments", label: "Shipments", icon: <IconBox /> },
    ],
  },
  {
    title: "Công cụ",
    items: [
      { to: "/assistant", label: "Trợ lý AI", icon: <IconSparkles /> },
      { to: "/demo", label: "Demo", icon: <IconPlay /> },
    ],
  },
];

function initials(id?: string): string {
  if (!id) return "DC";
  const parts = id.replace(/[^a-zA-Z0-9]+/g, " ").trim().split(" ");
  return (parts[0]?.[0] ?? "D").concat(parts[1]?.[0] ?? "").toUpperCase();
}

export function AppLayout() {
  const { user, logout } = useAuth();

  return (
    <div className="shell">
      <aside className="sidebar">
        <div className="ws">
          <span className="avatar" aria-hidden>
            {initials(user?.tenant_id)}
          </span>
          <span className="ws-meta">
            <strong>DigicoreOS</strong>
            <span>{user?.tenant_id ?? "tenant"} · Pro</span>
          </span>
        </div>

        <div className="side-search">
          <IconSearch size={16} />
          Tìm kiếm…
        </div>

        {GROUPS.map((group) => (
          <div key={group.title}>
            <div className="side-group-label">{group.title}</div>
            <nav className="side-nav">
              {group.items.map((item) => (
                <NavLink
                  key={item.to}
                  to={item.to}
                  end={item.end}
                  className={({ isActive }) =>
                    isActive ? "side-link active" : "side-link"
                  }
                >
                  <span className="ic">{item.icon}</span>
                  {item.label}
                </NavLink>
              ))}
            </nav>
          </div>
        ))}

        <Link className="upgrade-card" to="/demo">
          <span className="uc-ic" aria-hidden>
            <IconBolt size={18} />
          </span>
          <p>Chạy demo Đơn hàng → Điểm thưởng để xem event backbone hoạt động.</p>
          <span className="uc-btn">Chạy demo →</span>
        </Link>

        <div className="side-foot">
          <div className="user-card">
            <span className="avatar" aria-hidden>
              {initials(user?.id)}
            </span>
            <span className="user-meta">
              <strong>{user?.id ?? "—"}</strong>
              {user?.roles.join(", ") || "—"}
            </span>
          </div>
          <button className="ghost" onClick={logout}>
            Đăng xuất
          </button>
        </div>
      </aside>

      <div className="content">
        <header className="topbar">
          <span className="topbar-search">
            <IconSearch size={16} />
            Tìm nhanh…
          </span>
          <span className="spacer" />
          <button className="icon-btn" aria-label="Thông báo">
            <IconBell size={19} />
          </button>
          <span className="avatar-sm" aria-hidden>
            {initials(user?.id)}
          </span>
        </header>
        <main className="container">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
