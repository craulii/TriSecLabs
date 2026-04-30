use common::{AppError, TenantId};
use db::{queries::scan_targets, with_tenant_conn, PgPool};
use serde_json::Value;
use tracing::info;
use uuid::Uuid;

pub async fn handle(pool: &PgPool, tenant_id: TenantId, payload: &Value) -> Result<(), AppError> {
    let target_id: Uuid = payload["target_id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| AppError::InvalidInput("missing target_id".into()))?;

    info!(%tenant_id, %target_id, "computing risk score");

    let (critical, high, medium, low) = with_tenant_conn(pool, tenant_id, |conn| {
        Box::pin(async move {
            let counts = sqlx::query!(
                r#"SELECT
                    COUNT(*) FILTER (WHERE severity = 'critical') AS critical,
                    COUNT(*) FILTER (WHERE severity = 'high')     AS high,
                    COUNT(*) FILTER (WHERE severity = 'medium')   AS medium,
                    COUNT(*) FILTER (WHERE severity = 'low')      AS low
                   FROM vulnerabilities
                   WHERE target_id = $1
                     AND status NOT IN ('false_positive'::vuln_status, 'accepted'::vuln_status)"#,
                target_id
            )
            .fetch_one(conn)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

            Ok((
                counts.critical.unwrap_or(0),
                counts.high.unwrap_or(0),
                counts.medium.unwrap_or(0),
                counts.low.unwrap_or(0),
            ))
        })
    })
    .await?;

    let score = (critical * 25 + high * 10 + medium * 4 + low).min(100) as i16;

    let risk_level = match score {
        75..=100 => "critical",
        50..=74  => "high",
        25..=49  => "medium",
        10..=24  => "low",
        _        => "info",
    };

    with_tenant_conn(pool, tenant_id, |conn| {
        Box::pin(async move {
            scan_targets::update_risk(conn, target_id, score, risk_level).await
        })
    })
    .await?;

    info!(%target_id, score, risk_level, "analysis completed");
    Ok(())
}
