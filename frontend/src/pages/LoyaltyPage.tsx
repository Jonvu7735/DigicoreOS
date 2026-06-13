import { type FormEvent, useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";

import type { components } from "../api/schema";
import { useApi } from "../api/useApi";
import { badgeClass } from "../components/badge";

type LoyaltyAccount = components["schemas"]["LoyaltyAccount"];
type LoyaltyRules = components["schemas"]["LoyaltyRules"];

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

  const totalPoints = accounts.reduce((s, a) => s + (a.points_balance ?? 0), 0);
  const totalSpend = accounts.reduce((s, a) => s + (a.lifetime_spend ?? 0), 0);

  return (
    <>
      <section className="page-head">
        <h1>Loyalty</h1>
        <p className="subtitle">
          Tài khoản điểm thưởng của khách hàng (Retail).
        </p>
      </section>

      <section className="stat-grid">
        <div className="stat">
          <span className="stat-label">Tài khoản</span>
          <span className="stat-value">{accounts.length}</span>
        </div>
        <div className="stat">
          <span className="stat-label">Tổng điểm</span>
          <span className="stat-value">
            {totalPoints.toLocaleString("vi-VN")}
          </span>
        </div>
        <div className="stat">
          <span className="stat-label">Tổng chi tiêu</span>
          <span className="stat-value">
            {totalSpend.toLocaleString("vi-VN")}
          </span>
        </div>
      </section>

      <section className="card">
        <RulesPanel />

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
      </section>
    </>
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
        <span className={badgeClass(account.tier)}>{account.tier ?? "—"}</span>
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

function RulesPanel() {
  const api = useApi();
  const [rules, setRules] = useState<LoyaltyRules | null>(null);
  const [editing, setEditing] = useState(false);
  const [saving, setSaving] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  // Draft fields (strings while editing)
  const [perPoint, setPerPoint] = useState("");
  const [silver, setSilver] = useState("");
  const [gold, setGold] = useState("");

  useEffect(() => {
    let active = true;
    api.GET("/api/v1/retail/loyalty/rules", {}).then(({ data }) => {
      if (active && data) setRules(data);
    });
    return () => {
      active = false;
    };
  }, [api]);

  function startEdit() {
    if (!rules) return;
    setPerPoint(String(rules.minor_per_point));
    setSilver(String(rules.silver_min));
    setGold(String(rules.gold_min));
    setErr(null);
    setEditing(true);
  }

  async function save(event: FormEvent) {
    event.preventDefault();
    const body = {
      minor_per_point: Number.parseInt(perPoint, 10),
      silver_min: Number.parseInt(silver, 10),
      gold_min: Number.parseInt(gold, 10),
    };
    if (body.minor_per_point < 1 || body.gold_min < body.silver_min) {
      setErr("Quy tắc không hợp lệ (điểm/đơn vị ≥ 1, Vàng ≥ Bạc).");
      return;
    }
    setSaving(true);
    setErr(null);
    const { data, error } = await api.PUT("/api/v1/retail/loyalty/rules", {
      body,
    });
    setSaving(false);
    if (error || !data) {
      setErr("Lưu quy tắc thất bại.");
      return;
    }
    setRules(data);
    setEditing(false);
  }

  if (!rules) return null;

  return (
    <div className="done-box">
      {!editing ? (
        <div className="row">
          <span className="muted">
            Quy tắc: <strong>1 điểm</strong> / {rules.minor_per_point}đ · Bạc ≥{" "}
            {rules.silver_min} · Vàng ≥ {rules.gold_min}
          </span>
          <button className="ghost" onClick={startEdit}>
            Sửa
          </button>
        </div>
      ) : (
        <form onSubmit={save} className="create-row">
          <input
            type="number"
            min={1}
            value={perPoint}
            onChange={(e) => setPerPoint(e.target.value)}
            placeholder="đ / điểm"
          />
          <input
            type="number"
            min={0}
            value={silver}
            onChange={(e) => setSilver(e.target.value)}
            placeholder="Bạc ≥"
          />
          <input
            type="number"
            min={0}
            value={gold}
            onChange={(e) => setGold(e.target.value)}
            placeholder="Vàng ≥"
          />
          <button type="submit" disabled={saving}>
            {saving ? "…" : "Lưu"}
          </button>
          <button
            type="button"
            className="ghost"
            onClick={() => setEditing(false)}
          >
            Huỷ
          </button>
        </form>
      )}
      {err && <p className="error">{err}</p>}
    </div>
  );
}
