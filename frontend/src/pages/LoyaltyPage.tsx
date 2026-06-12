import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";

import type { components } from "../api/schema";
import { useApi } from "../api/useApi";

type LoyaltyAccount = components["schemas"]["LoyaltyAccount"];

export function LoyaltyPage() {
  const api = useApi();
  const [accounts, setAccounts] = useState<LoyaltyAccount[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Load on mount (and if the client changes). State is only set in the async
  // resolution, not synchronously in the effect (react-hooks/set-state-in-effect).
  useEffect(() => {
    let active = true;
    api
      .GET("/api/v1/retail/loyalty", {
        params: { query: { page: 1, page_size: 50 } },
      })
      .then(({ data, error: err }) => {
        if (!active) return;
        if (err || !data) {
          setError("Không tải được danh sách điểm thưởng.");
        } else {
          setAccounts(data.items ?? []);
        }
        setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [api]);

  const onRedeemed = useCallback((updated: LoyaltyAccount) => {
    setAccounts((prev) =>
      prev.map((a) => (a.customer_id === updated.customer_id ? updated : a)),
    );
  }, []);

  return (
    <main className="card wide">
      <header className="row">
        <h1>Loyalty</h1>
        <Link className="ghost-link" to="/">
          ← Trang chủ
        </Link>
      </header>
      <p className="muted">Tài khoản điểm thưởng của khách hàng (Retail).</p>

      {loading && <p className="muted">Đang tải…</p>}
      {error && <p className="error">{error}</p>}
      {!loading && !error && accounts.length === 0 && (
        <p className="muted">Chưa có tài khoản điểm thưởng nào.</p>
      )}

      {accounts.length > 0 && (
        <table>
          <thead>
            <tr>
              <th>Khách hàng</th>
              <th>Hạng</th>
              <th className="num">Điểm</th>
              <th className="num">Chi tiêu</th>
              <th>Đổi điểm</th>
            </tr>
          </thead>
          <tbody>
            {accounts.map((a, i) => (
              <LoyaltyRow
                key={a.customer_id ?? i}
                account={a}
                onRedeemed={onRedeemed}
              />
            ))}
          </tbody>
        </table>
      )}
    </main>
  );
}

function LoyaltyRow({
  account,
  onRedeemed,
}: {
  account: LoyaltyAccount;
  onRedeemed: (updated: LoyaltyAccount) => void;
}) {
  const api = useApi();
  const [points, setPoints] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  async function redeem() {
    const n = Number(points);
    if (!Number.isInteger(n) || n < 1) {
      setErr("Số điểm không hợp lệ");
      return;
    }
    setBusy(true);
    setErr(null);
    const { data, error } = await api.POST(
      "/api/v1/retail/loyalty/{customer_id}/redeem",
      {
        params: { path: { customer_id: account.customer_id ?? "" } },
        body: { points: n },
      },
    );
    setBusy(false);
    if (error || !data) {
      setErr("Đổi điểm thất bại (đủ điểm?)");
      return;
    }
    setPoints("");
    onRedeemed(data);
  }

  return (
    <tr>
      <td className="mono">
        {account.customer_id ? (
          <Link className="ghost-link" to={`/loyalty/${account.customer_id}`}>
            {account.customer_id}
          </Link>
        ) : (
          "—"
        )}
      </td>
      <td>
        <span className="pill">{account.tier ?? "—"}</span>
      </td>
      <td className="num">{account.points_balance ?? 0}</td>
      <td className="num">{account.lifetime_spend ?? 0}</td>
      <td className="redeem">
        <input
          type="number"
          min={1}
          value={points}
          onChange={(e) => setPoints(e.target.value)}
          placeholder="điểm"
        />
        <button disabled={busy} onClick={redeem}>
          {busy ? "…" : "Đổi"}
        </button>
        {err && <span className="error">{err}</span>}
      </td>
    </tr>
  );
}
