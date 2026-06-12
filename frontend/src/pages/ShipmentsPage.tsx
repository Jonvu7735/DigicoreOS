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

  function onBooked(updated: Shipment) {
    setShipments((prev) => prev.map((s) => (s.id === updated.id ? updated : s)));
  }

  return (
    <main className="card wide">
      <header className="row">
        <h1>Shipments</h1>
        <Link className="ghost-link" to="/">
          ← Trang chủ
        </Link>
      </header>
      <p className="muted">Lô hàng xuất khẩu (Trade-export).</p>

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
              <ShipmentRow key={s.id ?? i} shipment={s} onBooked={onBooked} />
            ))}
          </tbody>
        </table>
      )}
    </main>
  );
}

function ShipmentRow({
  shipment,
  onBooked,
}: {
  shipment: Shipment;
  onBooked: (updated: Shipment) => void;
}) {
  const api = useApi();
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  async function book() {
    if (!shipment.id) return;
    setBusy(true);
    setErr(null);
    const { data, error } = await api.POST(
      "/api/v1/trade-export/shipments/{shipment_id}/book",
      { params: { path: { shipment_id: shipment.id } } },
    );
    setBusy(false);
    if (error || !data) {
      setErr("Không đặt được chỗ");
      return;
    }
    onBooked(data);
  }

  return (
    <tr>
      <td className="mono">{shipment.reference ?? "—"}</td>
      <td>{shipment.destination_country}</td>
      <td>{shipment.incoterm}</td>
      <td>
        <span className="pill">{shipment.status ?? "—"}</span>
      </td>
      <td className="redeem">
        {shipment.status === "DRAFT" && (
          <button disabled={busy} onClick={book}>
            {busy ? "…" : "Đặt chỗ"}
          </button>
        )}
        {err && <span className="error">{err}</span>}
      </td>
    </tr>
  );
}
