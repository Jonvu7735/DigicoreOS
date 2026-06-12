import { useAuth } from "../auth/useAuth";

export function HomePage() {
  const { user, logout } = useAuth();
  return (
    <main className="card">
      <header className="row">
        <h1>DigicoreOS</h1>
        <button className="ghost" onClick={logout}>
          Đăng xuất
        </button>
      </header>
      <p>Đăng nhập thành công.</p>
      <p className="muted">User: {user?.id}</p>
      <p className="muted">
        Tenant: {user?.tenant_id} · Vai trò: {user?.roles.join(", ") || "—"}
      </p>
      <nav className="links">
        <span className="muted">Sắp có:</span>
        <span className="pill">Loyalty (Retail)</span>
        <span className="pill">Shipments (Trade-export)</span>
      </nav>
    </main>
  );
}
