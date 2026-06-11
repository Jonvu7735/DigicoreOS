//! Deterministic, no-network `InsightGenerator`.
//!
//! Stands in for a real LLM while this environment has no model credentials. It
//! applies simple heuristics over the context so the full pipeline
//! (event/HTTP -> generate -> persist -> publish) is exercisable end to end. A
//! real adapter (Claude API + embeddings) replaces this behind the same port.

use async_trait::async_trait;

use crate::domain::insights::ports::{GeneratedInsight, GenerationRequest, InsightGenerator};
use crate::domain::shared::error::DomainResult;

pub struct StubInsightGenerator;

#[async_trait]
impl InsightGenerator for StubInsightGenerator {
    async fn generate(&self, request: &GenerationRequest) -> DomainResult<GeneratedInsight> {
        let ctx = &request.context;

        // Heuristic classification from the context shape.
        let generated = if let Some(stype) = ctx.get("snapshot_type").and_then(|v| v.as_str()) {
            GeneratedInsight {
                category: "snapshot_digest".into(),
                summary: format!("A {stype} report snapshot is available; review it for trends."),
            }
        } else if let Some(total) = ctx.get("total_paid").and_then(|v| v.as_i64()) {
            let count = ctx
                .get("payment_count")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            if count == 0 {
                GeneratedInsight {
                    category: "sales_anomaly".into(),
                    summary: "No payments recorded for this period — investigate stalled sales."
                        .into(),
                }
            } else {
                GeneratedInsight {
                    category: "sales_summary".into(),
                    summary: format!("{count} payment(s) totalling {total} minor units."),
                }
            }
        } else {
            GeneratedInsight {
                category: request
                    .category_hint
                    .clone()
                    .unwrap_or_else(|| "general".into()),
                summary: "Insight generated from the provided context.".into(),
            }
        };
        Ok(generated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn gen(ctx: serde_json::Value, hint: Option<&str>) -> GeneratedInsight {
        StubInsightGenerator
            .generate(&GenerationRequest {
                category_hint: hint.map(|h| h.to_string()),
                context: ctx,
            })
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn classifies_zero_sales_as_anomaly() {
        let g = gen(
            serde_json::json!({ "total_paid": 0, "payment_count": 0 }),
            None,
        )
        .await;
        assert_eq!(g.category, "sales_anomaly");
    }

    #[tokio::test]
    async fn summarises_nonzero_sales() {
        let g = gen(
            serde_json::json!({ "total_paid": 5000, "payment_count": 2 }),
            None,
        )
        .await;
        assert_eq!(g.category, "sales_summary");
        assert!(g.summary.contains('2'));
    }

    #[tokio::test]
    async fn snapshot_context_is_a_digest() {
        let g = gen(serde_json::json!({ "snapshot_type": "sales" }), Some("x")).await;
        assert_eq!(g.category, "snapshot_digest");
    }

    #[tokio::test]
    async fn falls_back_to_hint() {
        let g = gen(serde_json::json!({ "foo": "bar" }), Some("churn_risk")).await;
        assert_eq!(g.category, "churn_risk");
    }
}
