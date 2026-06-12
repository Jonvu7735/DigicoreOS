import { type FormEvent, useState } from "react";
import { Link } from "react-router-dom";

import type { components } from "../api/schema";
import { useApi } from "../api/useApi";

type AiResponse = components["schemas"]["AiResponse"];

export function AssistantPage() {
  const api = useApi();
  const [query, setQuery] = useState("");
  const [answer, setAnswer] = useState<AiResponse | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function onAsk(event: FormEvent) {
    event.preventDefault();
    const q = query.trim();
    if (!q) return;
    setBusy(true);
    setError(null);
    setAnswer(null);
    const { data, error: err } = await api.POST("/api/v1/ai/query", {
      body: { query: q },
    });
    setBusy(false);
    if (err || !data) {
      setError("Trợ lý không phản hồi được.");
      return;
    }
    setAnswer(data);
  }

  return (
    <main className="card wide">
      <header className="row">
        <h1>Trợ lý AI</h1>
        <Link className="ghost-link" to="/">
          ← Trang chủ
        </Link>
      </header>
      <p className="muted">
        Hỏi trợ lý về nền tảng. Khi chưa cấu hình model, hệ thống trả lời bằng
        engine mẫu (deterministic).
      </p>

      <form onSubmit={onAsk} className="create-row">
        <input
          placeholder="Hỏi gì đó… (VD: cách đặt chỗ lô hàng?)"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          required
          style={{ flex: "1 1 320px" }}
        />
        <button type="submit" disabled={busy}>
          {busy ? "…" : "Hỏi"}
        </button>
      </form>
      {error && <p className="error">{error}</p>}

      {answer && (
        <div className="done-box">
          <p style={{ margin: 0, whiteSpace: "pre-wrap" }}>{answer.answer}</p>
          {answer.model && (
            <span className="pill" style={{ alignSelf: "flex-start" }}>
              {answer.model}
            </span>
          )}
        </div>
      )}
    </main>
  );
}
