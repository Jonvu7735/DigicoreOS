import type { ReactNode } from "react";
import { useEffect, useState } from "react";

import { BarChart, Gauge, LineChart } from "../components/charts";
import {
  IconBriefcase,
  IconCube,
  IconTag,
  IconUsers,
} from "../components/icons";
import { useApi } from "../api/useApi";

function fmt(v: unknown): string {
  if (typeof v === "number") return v.toLocaleString("vi-VN");
  if (typeof v === "string" || typeof v === "boolean") return String(v);
  return "—";
}

// Illustrative chart series (overview returns headline metrics, not time
// series); the KPI numbers are real, from the API.
const MONTHS = ["T1", "T2", "T3", "T4", "T5", "T6", "T7"];
const C1 = "#3c50e0";
const C2 = "#80caee";
const BAR = [
  { color: C1, values: [12, 18, 14, 22, 19, 26, 24] },
  { color: C2, values: [6, 9, 7, 11, 9, 13, 12] },
];
const LINE_MONTHS = ["T1", "T2", "T3", "T4", "T5", "T6"];
const LINE = [
  { color: C1, values: [20, 28, 26, 34, 40, 52] },
  { color: C2, values: [14, 18, 22, 26, 31, 38] },
];

type Kpi = {
  label: string;
  key: string;
  icon: ReactNode;
  c: string;
  delta: string;
  up: boolean;
};
const KPIS: Kpi[] = [
  { label: "Khách hàng", key: "customers", icon: <IconUsers />, c: "c-indigo", delta: "+5%", up: true },
  { label: "Sản phẩm", key: "products", icon: <IconTag />, c: "c-violet", delta: "+2,5%", up: true },
  { label: "Tồn kho", key: "inventory_units", icon: <IconCube />, c: "c-blue", delta: "-3%", up: false },
  { label: "Nhân sự", key: "employees", icon: <IconBriefcase />, c: "c-green", delta: "+1", up: true },
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
      <section className="page-head row">
        <div>
          <h1>Tổng quan</h1>
          <p className="subtitle">Số liệu nền tảng theo thời gian thực.</p>
        </div>
        <span className="control">6 tháng gần đây ▾</span>
      </section>

      <section className="stat-grid">
        {KPIS.map((k) => (
          <div className="kpi" key={k.key}>
            <div className="kpi-top">
              <span className={`kpi-ic ${k.c}`}>{k.icon}</span>
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

      <section className="dash-2">
        <div className="card">
          <div className="card-head">
            <div>
              <div className="section-title">Doanh thu</div>
              <div className="big-value">{fmt(m.revenue)}</div>
            </div>
            <span className="delta delta-up">▲ 2% / 7 ngày</span>
          </div>
          <BarChart labels={MONTHS} series={BAR} />
          <div className="legend">
            <span>
              <span className="dot" style={{ background: C1 }} />
              Chốt đơn
            </span>
            <span>
              <span className="dot" style={{ background: C2 }} />
              Tiềm năng
            </span>
          </div>
        </div>

        <div className="card">
          <div className="card-head">
            <div className="section-title">Đơn hàng</div>
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

      <section className="card">
        <div className="card-head">
          <div>
            <div className="section-title">Đơn theo tháng</div>
            <div className="legend" style={{ marginTop: 2 }}>
              <span>
                <span className="dot" style={{ background: C1 }} />
                Hoàn tất
              </span>
              <span>
                <span className="dot" style={{ background: C2 }} />
                Đang xử lý
              </span>
            </div>
          </div>
        </div>
        <LineChart labels={LINE_MONTHS} series={LINE} />
      </section>
    </>
  );
}
