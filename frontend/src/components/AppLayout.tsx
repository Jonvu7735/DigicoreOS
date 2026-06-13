import { Link, NavLink, Outlet } from "react-router-dom";

import { useAuth } from "../auth/useAuth";

const NAV = [
  { to: "/", label: "Tổng quan", end: true },
  { to: "/loyalty", label: "Loyalty" },
  { to: "/shipments", label: "Shipments" },
  { to: "/assistant", label: "Trợ lý AI" },
  { to: "/demo", label: "Demo" },
];

/** Persistent app shell: top navbar (brand + nav + identity/logout) over a
 * centered content container. Wraps every authenticated route. */
export function AppLayout() {
  const { user, logout } = useAuth();
  return (
    <div className="app">
      <header className="navbar">
        <div className="navbar-inner">
          <Link to="/" className="brand">
            <span className="brand-dot" aria-hidden />
            DigicoreOS
          </Link>
          <nav className="nav-links">
            {NAV.map((item) => (
              <NavLink
                key={item.to}
                to={item.to}
                end={item.end}
                className={({ isActive }) =>
                  isActive ? "nav-link active" : "nav-link"
                }
              >
                {item.label}
              </NavLink>
            ))}
          </nav>
          <div className="nav-right">
            <span className="who">
              <strong>{user?.id ?? "—"}</strong>
              {user?.tenant_id} · {user?.roles.join(", ") || "—"}
            </span>
            <button className="ghost" onClick={logout}>
              Đăng xuất
            </button>
          </div>
        </div>
      </header>
      <main className="container">
        <Outlet />
      </main>
    </div>
  );
}
