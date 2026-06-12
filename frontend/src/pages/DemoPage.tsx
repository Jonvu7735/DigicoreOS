import { useState } from "react";
import { Link } from "react-router-dom";

import { useApi } from "../api/useApi";

type Step = "idle" | "running" | "done" | "error";

/**
 * One-click cross-service demo: create a customer (CRM) + a product (ERP) + an
 * order (ERP). Creating the order emits `OrderCreated`, which flows over NATS to
 * retail-svc, which accrues loyalty points — so the event backbone is provable
 * live, on screen, via the Loyalty page.
 */
export function DemoPage() {
  const api = useApi();
  const [step, setStep] = useState<Step>("idle");
  const [log, setLog] = useState<string[]>([]);
  const [summary, setSummary] = useState<{ name: string; total: number } | null>(
    null,
  );

  async function run() {
    setStep("running");
    setLog([]);
    setSummary(null);
    const add = (line: string) => setLog((l) => [...l, line]);
    const tag = Date.now().toString().slice(-6);

    const name = `Demo Khách ${tag}`;
    const customer = await api.POST("/api/v1/crm/customers", {
      body: { name, segment: "demo" },
    });
    const customerId = customer.data?.id;
    if (customer.error || !customerId) {
      add("✗ Tạo khách hàng thất bại — cần quyền crm_customer_create.");
      setStep("error");
      return;
    }
    add(`✓ Khách hàng: ${name}`);

    const price = 250_000; // minor units
    const product = await api.POST("/api/v1/erp/products", {
      body: { sku: `DEMO-${tag}`, name: `Sản phẩm demo ${tag}`, price, currency: "USD" },
    });
    const productId = product.data?.id;
    if (product.error || !productId) {
      add("✗ Tạo sản phẩm thất bại — cần quyền erp_product_create (MANAGER+).");
      setStep("error");
      return;
    }
    add(`✓ Sản phẩm: DEMO-${tag}`);

    const order = await api.POST("/api/v1/erp/orders", {
      body: {
        customer_id: customerId,
        currency: "USD",
        lines: [{ product_id: productId, quantity: 1, unit_price: price }],
      },
    });
    if (order.error || !order.data?.id) {
      add("✗ Tạo đơn hàng thất bại — cần quyền erp_order_create.");
      setStep("error");
      return;
    }
    const total = order.data.total_amount ?? price;
    add(`✓ Đơn hàng tạo xong — tổng ${total} → phát OrderCreated`);
    setSummary({ name, total });
    setStep("done");
  }

  const running = step === "running";

  return (
    <main className="card wide">
      <header className="row">
        <h1>Demo: Đơn hàng → Điểm thưởng</h1>
        <Link className="ghost-link" to="/">
          ← Trang chủ
        </Link>
      </header>
      <p className="muted">
        Một cú nhấp tạo khách hàng + sản phẩm + đơn hàng. Khi đơn được tạo, sự kiện{" "}
        <code>OrderCreated</code> chạy qua NATS và retail-svc tự cộng điểm thưởng —
        kiểm chứng event backbone ngay trên màn hình.
      </p>

      <button onClick={run} disabled={running}>
        {running ? "Đang chạy…" : "Tạo đơn hàng demo"}
      </button>

      {log.length > 0 && (
        <ul className="log">
          {log.map((line, i) => (
            <li key={i}>{line}</li>
          ))}
        </ul>
      )}

      {step === "done" && summary && (
        <div className="done-box">
          <p>
            Xong! Điểm thưởng cho <strong>{summary.name}</strong> (tổng{" "}
            {summary.total}) sẽ xuất hiện trong giây lát — event là bất đồng bộ.
          </p>
          <Link className="tile" to="/loyalty">
            Xem Loyalty →
          </Link>
        </div>
      )}
    </main>
  );
}
