import { type FormEvent, useEffect, useState } from "react";
import { Link } from "react-router-dom";

import type { components } from "../api/schema";
import { useApi } from "../api/useApi";

type Shipment = components["schemas"]["Shipment"];

export function ShipmentsPage() {
  const api = useApi();
  const [shipments, setShipments] = useState<Shipment[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Create form
  const [dest, setDest] = useState("");
  const [incoterm, setIncoterm] = useState("FOB");
  const [orderId, setOrderId] = useState("");
  const [creating, setCreating] = useState(false);
  const [createErr, setCreateErr] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    api
      .GET("/api/v1/trade-export/shipments", {
        params: { query: { page: 1, page_size: 50 } },
      })
      .then(({ data, error: err }) => {
        if (!active) return;
        if (err || !data) setError("Không tải được danh sách lô hàng.");
        else setShipments(data.items ?? []);
        setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [api]);

  async function onCreate(event: FormEvent) {
    event.preventDefault();
    setCreating(true);
    setCreateErr(null);
    const { data, error: err } = await api.POST(
      "/api/v1/trade-export/shipments",
      {
        body: {
          destination_country: dest.trim().toUpperCase(),
          incoterm: incoterm.trim().toUpperCase(),
          order_id: orderId.trim() || null,
        },
      },
    );
    setCreating(false);
    if (err || !data) {
      setCreateErr("Tạo lô hàng thất bại.");
      return;
    }
    setShipments((prev) => [data, ...prev]);
    setDest("");
    setOrderId("");
  }

  function onUpdated(updated: Shipment) {
    setShipments((prev) => prev.map((s) => (s.id === updated.id ? updated : s)));
  }

  const countOf = (st: string) =>
    shipments.filter((s) => s.status === st).length;

  return (
    <>
      <section className="page-head">
        <h1>Shipments</h1>
        <p className="subtitle">Lô hàng xuất khẩu (Trade-export).</p>
      </section>

      <section className="stat-grid">
        <div className="stat">
          <span className="stat-label">Tổng lô hàng</span>
          <span className="stat-value">{shipments.length}</span>
        </div>
        <div className="stat">
          <span className="stat-label">Nháp / Đặt chỗ</span>
          <span className="stat-value">
            {countOf("DRAFT") + countOf("BOOKED")}
          </span>
        </div>
        <div className="stat">
          <span className="stat-label">Đã gửi đi</span>
          <span className="stat-value">{countOf("DISPATCHED")}</span>
        </div>
        <div className="stat">
          <span className="stat-label">Đã huỷ</span>
          <span className="stat-value">{countOf("CANCELLED")}</span>
        </div>
      </section>

      <section className="card">
      <form onSubmit={onCreate} className="create-row">
        <input
          placeholder="Nước đến (VD: VN)"
          value={dest}
          onChange={(e) => setDest(e.target.value)}
          required
          maxLength={2}
        />
        <input
          placeholder="Incoterm (VD: FOB)"
          value={incoterm}
          onChange={(e) => setIncoterm(e.target.value)}
          required
        />
        <input
          placeholder="Order ID (tuỳ chọn)"
          value={orderId}
          onChange={(e) => setOrderId(e.target.value)}
        />
        <button type="submit" disabled={creating}>
          {creating ? "…" : "Tạo lô"}
        </button>
      </form>
      {createErr && <p className="error">{createErr}</p>}

      {loading && <p className="muted">Đang tải…</p>}
      {error && <p className="error">{error}</p>}
      {!loading && !error && shipments.length === 0 && (
        <p className="muted">Chưa có lô hàng nào.</p>
      )}

      {shipments.length > 0 && (
        <table>
          <thead>
            <tr>
              <th>Mã</th>
              <th>Đến</th>
              <th>Incoterm</th>
              <th>Trạng thái</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {shipments.map((s, i) => (
              <ShipmentRow key={s.id ?? i} shipment={s} onUpdated={onUpdated} />
            ))}
          </tbody>
        </table>
      )}
      </section>
    </>
  );
}

function ShipmentRow({
  shipment,
  onUpdated,
}: {
  shipment: Shipment;
  onUpdated: (updated: Shipment) => void;
}) {
  const api = useApi();
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  async function act(action: "book" | "dispatch" | "cancel") {
    const id = shipment.id;
    if (!id) return;
    setBusy(true);
    setErr(null);
    const res =
      action === "book"
        ? await api.POST("/api/v1/trade-export/shipments/{shipment_id}/book", {
            params: { path: { shipment_id: id } },
          })
        : action === "dispatch"
          ? await api.POST("/api/v1/trade-export/shipments/{shipment_id}/dispatch", {
              params: { path: { shipment_id: id } },
            })
          : await api.POST("/api/v1/trade-export/shipments/{shipment_id}/cancel", {
              params: { path: { shipment_id: id } },
            });
    setBusy(false);
    if (res.error || !res.data) {
      setErr("Thao tác thất bại");
      return;
    }
    onUpdated(res.data);
  }

  const status = shipment.status;
  return (
    <tr>
      <td className="mono">
        {shipment.id ? (
          <Link className="ghost-link" to={`/shipments/${shipment.id}`}>
            {shipment.reference ?? "—"}
          </Link>
        ) : (
          (shipment.reference ?? "—")
        )}
      </td>
      <td>{shipment.destination_country}</td>
      <td>{shipment.incoterm}</td>
      <td>
        <span className="pill">{status ?? "—"}</span>
      </td>
      <td className="redeem">
        {status === "DRAFT" && (
          <button disabled={busy} onClick={() => act("book")}>
            {busy ? "…" : "Đặt chỗ"}
          </button>
        )}
        {status === "BOOKED" && (
          <button disabled={busy} onClick={() => act("dispatch")}>
            {busy ? "…" : "Gửi đi"}
          </button>
        )}
        {(status === "DRAFT" || status === "BOOKED") && (
          <button className="ghost" disabled={busy} onClick={() => act("cancel")}>
            Huỷ
          </button>
        )}
        {err && <span className="error">{err}</span>}
      </td>
    </tr>
  );
}
