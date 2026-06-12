import { type FormEvent, useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";

import type { components } from "../api/schema";
import { useApi } from "../api/useApi";

type Shipment = components["schemas"]["Shipment"];
type CargoLine = components["schemas"]["CargoLine"];

export function ShipmentDetailPage() {
  const api = useApi();
  const { id = "" } = useParams();
  const [shipment, setShipment] = useState<Shipment | null>(null);
  const [lines, setLines] = useState<CargoLine[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Add-cargo form
  const [description, setDescription] = useState("");
  const [hsCode, setHsCode] = useState("");
  const [quantity, setQuantity] = useState("");
  const [unit, setUnit] = useState("CTN");
  const [weight, setWeight] = useState("");
  const [adding, setAdding] = useState(false);
  const [addErr, setAddErr] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    Promise.all([
      api.GET("/api/v1/trade-export/shipments/{shipment_id}", {
        params: { path: { shipment_id: id } },
      }),
      api.GET("/api/v1/trade-export/shipments/{shipment_id}/cargo", {
        params: { path: { shipment_id: id } },
      }),
    ]).then(([ship, cargo]) => {
      if (!active) return;
      if (ship.error || !ship.data) setError("Không tải được lô hàng.");
      else setShipment(ship.data);
      if (!cargo.error && cargo.data) setLines(cargo.data);
      setLoading(false);
    });
    return () => {
      active = false;
    };
  }, [api, id]);

  async function onAdd(event: FormEvent) {
    event.preventDefault();
    const qty = Number.parseInt(quantity, 10);
    if (!description.trim() || !Number.isInteger(qty) || qty < 1) {
      setAddErr("Cần mô tả và số lượng ≥ 1.");
      return;
    }
    setAdding(true);
    setAddErr(null);
    const net = weight.trim() === "" ? null : Number.parseFloat(weight);
    const { data, error: err } = await api.POST(
      "/api/v1/trade-export/shipments/{shipment_id}/cargo",
      {
        params: { path: { shipment_id: id } },
        body: {
          description: description.trim(),
          hs_code: hsCode.trim() || null,
          quantity: qty,
          unit: unit.trim().toUpperCase() || "PCS",
          net_weight_kg: net !== null && Number.isFinite(net) ? net : null,
        },
      },
    );
    setAdding(false);
    if (err || !data) {
      setAddErr("Thêm dòng hàng thất bại.");
      return;
    }
    setLines((prev) => [...prev, data]);
    setDescription("");
    setHsCode("");
    setQuantity("");
    setWeight("");
  }

  const editable =
    shipment?.status === "DRAFT" || shipment?.status === "BOOKED";
  const totalQty = lines.reduce((sum, l) => sum + (l.quantity ?? 0), 0);
  const totalWeight = lines.reduce((sum, l) => sum + (l.net_weight_kg ?? 0), 0);

  return (
    <main className="card wide">
      <header className="row">
        <h1>{shipment?.reference ?? "Lô hàng"}</h1>
        <Link className="ghost-link" to="/shipments">
          ← Danh sách
        </Link>
      </header>

      {loading && <p className="muted">Đang tải…</p>}
      {error && <p className="error">{error}</p>}

      {shipment && (
        <>
          <p className="muted">
            Đến <strong>{shipment.destination_country}</strong> ·{" "}
            {shipment.incoterm} ·{" "}
            <span className="pill">{shipment.status}</span>
            {shipment.order_id ? ` · đơn ${shipment.order_id}` : ""}
          </p>

          <h2 style={{ fontSize: "1rem", marginBottom: 0 }}>Dòng hàng</h2>
          {lines.length === 0 && (
            <p className="muted">Chưa có dòng hàng nào.</p>
          )}
          {lines.length > 0 && (
            <table>
              <thead>
                <tr>
                  <th>Mô tả</th>
                  <th>Mã HS</th>
                  <th className="num">SL</th>
                  <th>ĐVT</th>
                  <th className="num">Net (kg)</th>
                </tr>
              </thead>
              <tbody>
                {lines.map((l, i) => (
                  <tr key={l.id ?? i}>
                    <td>{l.description}</td>
                    <td className="mono">{l.hs_code ?? "—"}</td>
                    <td className="num">{l.quantity}</td>
                    <td>{l.unit}</td>
                    <td className="num">
                      {l.net_weight_kg == null ? "—" : l.net_weight_kg}
                    </td>
                  </tr>
                ))}
              </tbody>
              <tfoot>
                <tr>
                  <td className="muted">Tổng</td>
                  <td></td>
                  <td className="num">{totalQty}</td>
                  <td></td>
                  <td className="num">
                    {totalWeight > 0 ? totalWeight.toFixed(2) : "—"}
                  </td>
                </tr>
              </tfoot>
            </table>
          )}

          {editable && (
            <form onSubmit={onAdd} className="create-row">
              <input
                placeholder="Mô tả hàng"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                required
                style={{ flex: "2 1 180px" }}
              />
              <input
                placeholder="Mã HS"
                value={hsCode}
                onChange={(e) => setHsCode(e.target.value)}
              />
              <input
                placeholder="SL"
                type="number"
                min={1}
                value={quantity}
                onChange={(e) => setQuantity(e.target.value)}
                required
              />
              <input
                placeholder="ĐVT"
                value={unit}
                onChange={(e) => setUnit(e.target.value)}
                required
              />
              <input
                placeholder="Net kg"
                type="number"
                min={0}
                step="0.01"
                value={weight}
                onChange={(e) => setWeight(e.target.value)}
              />
              <button type="submit" disabled={adding}>
                {adding ? "…" : "Thêm"}
              </button>
            </form>
          )}
          {addErr && <p className="error">{addErr}</p>}
          {!editable && !loading && (
            <p className="muted">
              Lô hàng đã {shipment.status === "DISPATCHED" ? "gửi đi" : "huỷ"} —
              không thể sửa dòng hàng.
            </p>
          )}
        </>
      )}
    </main>
  );
}
