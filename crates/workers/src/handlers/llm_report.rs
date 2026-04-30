use common::{AppError, TenantId};
use db::{queries::scan_targets, with_tenant_conn, PgPool};
use llm::LlmClient;
use serde_json::Value;
use tracing::info;
use uuid::Uuid;

/// Worker LLM: genera informe en lenguaje natural.
/// Concurrencia máxima: 1 (gestionada por el poll loop del worker main).
pub async fn handle(
    pool: &PgPool,
    llm: &LlmClient,
    tenant_id: TenantId,
    payload: &Value,
) -> Result<(), AppError> {
    let target_id: Uuid = payload["target_id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| AppError::InvalidInput("missing target_id".into()))?;

    info!(%tenant_id, %target_id, "generating LLM report");

    let (target, findings) = with_tenant_conn(pool, tenant_id, |conn| {
        Box::pin(async move {
            let target = scan_targets::find_by_id(conn, target_id)
                .await?
                .ok_or_else(|| AppError::not_found(format!("target {target_id}")))?;

            let findings = sqlx::query!(
                r#"SELECT title, severity::text AS "severity!", cve_id,
                          cvss_score::float4 AS cvss_score
                   FROM scan_findings
                   WHERE target_id = $1
                   ORDER BY severity DESC
                   LIMIT 20"#,
                target_id
            )
            .fetch_all(conn)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

            Ok((target, findings))
        })
    })
    .await?;

    let findings_text = findings
        .iter()
        .map(|f| {
            format!(
                "- [{}] {} {}",
                f.severity,
                f.title,
                f.cve_id.as_deref().unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let system = "Eres un experto en ciberseguridad B2B. \
                  Analiza los hallazgos de seguridad y genera un informe ejecutivo conciso \
                  en español. Incluye: resumen ejecutivo, principales riesgos y recomendaciones \
                  priorizadas. Máximo 500 palabras.";

    let user_prompt = format!(
        "Vendor/Target: {} ({})\nRisk Score: {}/100\n\nHallazgos:\n{}",
        target.name,
        target.kind,
        target.risk_score.unwrap_or(0),
        if findings_text.is_empty() {
            "Sin hallazgos registrados.".into()
        } else {
            findings_text
        }
    );

    let report_content = llm.complete(system, &user_prompt).await?;

    with_tenant_conn(pool, tenant_id, |conn| {
        let content = report_content.clone();
        let tenant_uuid = *tenant_id.as_uuid();
        Box::pin(async move {
            sqlx::query!(
                "INSERT INTO reports (tenant_id, target_id, content, model_used)
                 VALUES ($1, $2, $3, 'llama.cpp')",
                tenant_uuid,
                target_id,
                content
            )
            .execute(conn)
            .await
            .map_err(|e| AppError::Database(e.to_string()))
        })
    })
    .await?;

    info!(%target_id, "LLM report saved");
    Ok(())
}
