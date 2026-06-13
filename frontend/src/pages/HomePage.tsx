import { useEffect, useState } from "react";
import { Link } from "react-router-dom";

import { BarChart, Gauge, LineChart } from "../components/charts";
import { useApi } from "../api/useApi";

function fmt(v: unknown): string {
  if (typeof v === "number") return v.toLocaleString("vi-VN");
  if (typeof v === "string" || typeof v === "boolean") return String(v);
  return "—";
}

// Illustrative series for the charts (the overview endpoint returns headline
// metrics, not time series); the KPI numbers below are real, from the API.
const MONTHS = ["T1", "T2", "T3", "T4", "T5", "T6", "T7"];
const BAR = [
  { color: "#6d5bd0", values: [12, 18, 14, 22, 19, 26, 24] },
  { color: "#c4b5fd", values: [6, 9, 7, 11, 9, 13, 12] },
];
const LINE_MONTHS = ["T1", "T2", "T3", "T4", "T5", "T6"];
const LINE = [
  { color: "#6d5bd0", values: [20, 28, 26, 34, 40, 52] },
  { color: "#34d399", values: [14, 18, 22, 26, 31, 38] },
];

const KPIS = [
  { label: "Khách hàng", key: "customers", icon: "👥", c: "c-indigo", delta: "+5%", up: true },
  { label: "Sản phẩm", key: "products", icon: "🏷️", c: "c-violet", delta: "+2,5%", up: true },
  { label: "Tồn kho", key: "inventory_units", icon: "📦", c: "c-blue", delta: "-3%", up: false },
  { label: "Nhân sự", key: "employees", icon: "🧑‍💼", c: "c-green", delta: "+1", up: true },
];

export function HomePage() {
  const api = useApi();
  const [m, setM] = useState<Record<string, unknown>>({});

  useEffect(() => {
    let active = true;
    api.GET("/api/v1/reporting/overview", {}).then(({ data }) => {
      if (!active) return;
      const d = data?.data;
      if (d && !Array.isArray(d)) setM(d);
    });
    return () => {
      active = false;
    };
  }, [api]);

  return (
    <>
      <section className="dash-2">
        <div className="card">
          <div className="card-head">
            <div>
              <div className="muted" style={{ fontSize: "0.85rem", fontWeight: 600 }}>
                Doanh thu
              </div>
              <div className="big-value">{fmt(m.revenue)}</div>
              <div className="muted" style={{ fontSize: "0.82rem" }}>
                <span className="delta delta-up">▲ 2%</span> so với 7 ngày trước
              </div>
            </div>
            <span className="control">6 tháng ▾</span>
          </div>
          <BarChart labels={MONTHS} series={BAR} />
          <div className="legend">
            <span>
              <span className="dot" style={{ background: "#6d5bd0" }} />
              Chốt đơn
            </span>
            <span>
              <span className="dot" style={{ background: "#c4b5fd" }} />
              Tiềm năng
            </span>
          </div>
        </div>

        <div className="card">
          <div className="card-head">
            <div className="muted" style={{ fontSize: "0.85rem", fontWeight: 600 }}>
              Đơn hàng
            </div>
            <span className="control">Tháng này ▾</span>
          </div>
          <div className="gauge-wrap">
            <Gauge value={0.62} />
            <div className="gauge-center">
              <div className="big-value">{fmt(m.orders)}</div>
              <div className="muted" style={{ fontSize: "0.82rem" }}>
                62% mục tiêu tháng
              </div>
            </div>
          </div>
        </div>
      </section>

      <section className="stat-grid">
        {KPIS.map((k) => (
          <div className="kpi" key={k.key}>
            <div className="kpi-top">
              <span className={`kpi-ic ${k.c}`} aria-hidden>
                {k.icon}
              </span>
              <span className={k.up ? "delta delta-up" : "delta delta-down"}>
                {k.up ? "▲" : "▼"} {k.delta}
              </span>
            </div>
            <span className="stat-label">{k.label}</span>
            <span className="stat-value">{fmt(m[k.key])}</span>
            <span className="kpi-foot">so với kỳ trước</span>
          </div>
        ))}
      </section>

      <section className="card">
        <div className="card-head">
          <div>
            <div className="section-title">Đơn theo tháng</div>
            <div className="legend" style={{ marginTop: 2 }}>
              <span>
                <span className="dot" style={{ background: "#6d5bd0" }} />
                Hoàn tất
              </span>
              <span>
                <span className="dot" style={{ background: "#34d399" }} />
                Đang xử lý
              </span>
            </div>
          </div>
          <span className="control">6 tháng gần đây ▾</span>
        </div>
        <LineChart labels={LINE_MONTHS} series={LINE} />
      </section>

      <section className="card">
        <h2 className="section-title">Lối tắt</h2>
        <nav className="links">
          <Link className="tile primary" to="/demo">
            <span className="ic" aria-hidden>▶</span> Demo: Đơn hàng → Điểm thưởng
          </Link>
          <Link className="tile" to="/loyalty">
            <span className="ic" aria-hidden>🎁</span> Loyalty — điểm thưởng
          </Link>
          <Link className="tile" to="/shipments">
            <span className="ic" aria-hidden>📦</span> Shipments — lô hàng
          </Link>
          <Link className="tile" to="/assistant">
            <span className="ic" aria-hidden>✨</span> Trợ lý AI
          </Link>
        </nav>
      </section>
    </>
  );
}
