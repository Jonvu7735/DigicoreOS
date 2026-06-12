import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";

import type { components } from "../api/schema";
import { useApi } from "../api/useApi";

type LoyaltyAccount = components["schemas"]["LoyaltyAccount"];
type LedgerEntry = components["schemas"]["PointsLedgerEntry"];

function fmtTime(iso?: string): string {
  if (!iso) return "";
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString();
}

export function LoyaltyDetailPage() {
  const api = useApi();
  const { customerId = "" } = useParams();
  const [account, setAccount] = useState<LoyaltyAccount | null>(null);
  const [ledger, setLedger] = useState<LedgerEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    Promise.all([
      api.GET("/api/v1/retail/loyalty/{customer_id}", {
        params: { path: { customer_id: customerId } },
      }),
      api.GET("/api/v1/retail/loyalty/{customer_id}/ledger", {
        params: {
          path: { customer_id: customerId },
          query: { page: 1, page_size: 100 },
        },
      }),
    ]).then(([acc, led]) => {
      if (!active) return;
      if (acc.error || !acc.data) setError("Không tải được tài khoản.");
      else setAccount(acc.data);
      if (!led.error && led.data) setLedger(led.data);
      setLoading(false);
    });
    return () => {
      active = false;
    };
  }, [api, customerId]);

  return (
    <main className="card wide">
      <header className="row">
        <h1 className="mono">{customerId}</h1>
        <Link className="ghost-link" to="/loyalty">
          ← Loyalty
        </Link>
      </header>

      {loading && <p className="muted">Đang tải…</p>}
      {error && <p className="error">{error}</p>}

      {account && (
        <>
          <p className="muted">
            <span className="pill">{account.tier}</span> ·{" "}
            <strong>{account.points_balance ?? 0}</strong> điểm · chi tiêu{" "}
            {account.lifetime_spend ?? 0}
          </p>

          <h2 style={{ fontSize: "1rem", marginBottom: 0 }}>Lịch sử điểm</h2>
          {ledger.length === 0 && (
            <p className="muted">Chưa có giao dịch điểm nào.</p>
          )}
          {ledger.length > 0 && (
            <table>
              <thead>
                <tr>
                  <th>Thời gian</th>
                  <th>Loại</th>
                  <th className="num">Điểm</th>
                  <th className="num">Số dư</th>
                  <th>Nguồn</th>
                </tr>
              </thead>
              <tbody>
                {ledger.map((e, i) => (
                  <tr key={e.id ?? i}>
                    <td className="muted">{fmtTime(e.at)}</td>
                    <td>
                      <span className="pill">
                        {e.kind === "EARN" ? "Tích" : "Đổi"}
                      </span>
                    </td>
                    <td className="num">
                      {e.kind === "EARN" ? "+" : "−"}
                      {e.points}
                    </td>
                    <td className="num">{e.balance_after}</td>
                    <td className="mono">{e.reason ?? "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </>
      )}
    </main>
  );
}
