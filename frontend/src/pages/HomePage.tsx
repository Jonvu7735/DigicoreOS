import { useEffect, useState } from "react";
import { Link } from "react-router-dom";

import { useApi } from "../api/useApi";
import { useAuth } from "../auth/useAuth";

/** Vietnamese labels for known overview metric keys; unknown keys fall back to
 * a prettified version of the key itself. */
const LABELS: Record<string, string> = {
  revenue: "Doanh thu",
  total_revenue: "Doanh thu",
  orders: "Đơn hàng",
  order_count: "Đơn hàng",
  total_orders: "Đơn hàng",
  customers: "Khách hàng",
  total_customers: "Khách hàng",
  products: "Sản phẩm",
  invoices: "Hoá đơn",
  inventory_units: "Tồn kho",
  stock_on_hand: "Tồn kho",
  employees: "Nhân sự",
  headcount: "Nhân sự",
  deals: "Cơ hội",
  shipments: "Lô hàng",
  loyalty_points: "Điểm thưởng",
  points: "Điểm thưởng",
};

function label(key: string): string {
  if (LABELS[key]) return LABELS[key];
  const s = key.replace(/[_-]+/g, " ");
  return s.charAt(0).toUpperCase() + s.slice(1);
}

function formatValue(v: unknown): string {
  if (typeof v === "number") return v.toLocaleString("vi-VN");
  if (typeof v === "string" || typeof v === "boolean") return String(v);
  return "—";
}

export function HomePage() {
  const api = useApi();
  const { user } = useAuth();
  const [metrics, setMetrics] = useState<Record<string, unknown>>({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    api.GET("/api/v1/reporting/overview", {}).then(({ data, error: err }) => {
      if (!active) return;
      if (err || !data) {
        setError("Chưa tải được số liệu tổng quan.");
      } else {
        const d = data.data;
        setMetrics(d && !Array.isArray(d) ? d : {});
      }
      setLoading(false);
    });
    return () => {
      active = false;
    };
  }, [api]);

  const entries = Object.entries(metrics);

  return (
    <>
      <section className="page-head">
        <h1>Tổng quan</h1>
        <p className="subtitle">
          Chào {user?.id ?? "bạn"} · tenant <strong>{user?.tenant_id}</strong> ·
          vai trò {user?.roles.join(", ") || "—"}
        </p>
      </section>

      {loading && <p className="muted">Đang tải số liệu…</p>}
      {error && <p className="muted">{error}</p>}

      {entries.length > 0 && (
        <section className="stat-grid">
          {entries.map(([key, value]) => (
            <div className="stat" key={key}>
              <span className="stat-label">{label(key)}</span>
              <span className="stat-value">{formatValue(value)}</span>
            </div>
          ))}
        </section>
      )}

      <section className="card">
        <h2 className="section-title">Lối tắt</h2>
        <nav className="links">
          <Link className="tile primary" to="/demo">
            ▶ Demo: Đơn hàng → Điểm thưởng
          </Link>
          <Link className="tile" to="/loyalty">
            Loyalty (Retail) — tài khoản & điểm thưởng
          </Link>
          <Link className="tile" to="/shipments">
            Shipments (Trade-export) — lô hàng xuất khẩu
          </Link>
          <Link className="tile" to="/assistant">
            Trợ lý AI — hỏi đáp nghiệp vụ
          </Link>
        </nav>
      </section>
    </>
  );
}
