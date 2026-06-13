import { Link, NavLink, Outlet, useLocation } from "react-router-dom";

import { useAuth } from "../auth/useAuth";

type Item = { to: string; label: string; icon: string; end?: boolean; count?: string };

const GROUPS: { title: string; items: Item[] }[] = [
  {
    title: "Menu",
    items: [{ to: "/", label: "Tổng quan", icon: "▦", end: true }],
  },
  {
    title: "Nghiệp vụ",
    items: [
      { to: "/loyalty", label: "Loyalty", icon: "🎁" },
      { to: "/shipments", label: "Shipments", icon: "📦" },
    ],
  },
  {
    title: "Công cụ",
    items: [
      { to: "/assistant", label: "Trợ lý AI", icon: "✨" },
      { to: "/demo", label: "Demo", icon: "▶" },
    ],
  },
];

const TITLES: Record<string, string> = {
  "/": "Tổng quan",
  "/loyalty": "Loyalty",
  "/shipments": "Shipments",
  "/assistant": "Trợ lý AI",
  "/demo": "Demo",
};

function pageTitle(path: string): string {
  if (TITLES[path]) return TITLES[path];
  if (path.startsWith("/loyalty")) return "Loyalty";
  if (path.startsWith("/shipments")) return "Shipments";
  return "DigicoreOS";
}

function initials(id?: string): string {
  if (!id) return "DC";
  const parts = id.replace(/[^a-zA-Z0-9]+/g, " ").trim().split(" ");
  return (parts[0]?.[0] ?? "D").concat(parts[1]?.[0] ?? "").toUpperCase();
}

export function AppLayout() {
  const { user, logout } = useAuth();
  const { pathname } = useLocation();

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
          <span className="chev" aria-hidden>
            ⌄
          </span>
        </div>

        <div className="side-search">🔍 Tìm kiếm…</div>

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
                  <span className="ic" aria-hidden>
                    {item.icon}
                  </span>
                  {item.label}
                  {item.count && <span className="count">{item.count}</span>}
                </NavLink>
              ))}
            </nav>
          </div>
        ))}

        <Link className="upgrade-card" to="/demo">
          <span className="uc-ic" aria-hidden>
            ⚡
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
          <span className="page-title">{pageTitle(pathname)}</span>
          <span className="spacer" />
          <span className="topbar-search">🔍 Tìm nhanh…</span>
          <div className="top-actions">
            <button className="btn-export">⤓ Xuất báo cáo</button>
            <span className="avatar-sm" aria-hidden>
              {initials(user?.id)}
            </span>
          </div>
        </header>
        <main className="container">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
