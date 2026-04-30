use common::{AppError, TenantId};
use db::{queries::scan_targets, with_tenant_conn, PgPool};
use serde_json::Value;
use tracing::info;
use uuid::Uuid;

pub async fn handle(pool: &PgPool, tenant_id: TenantId, payload: &Value) -> Result<(), AppError> {
    let target_id: Uuid = payload["target_id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| AppError::InvalidInput("missing target_id in payload".into()))?;

    info!(%tenant_id, %target_id, "starting scan");

    let target = with_tenant_conn(pool, tenant_id, |conn| {
        Box::pin(async move { scan_targets::find_by_id(conn, target_id).await })
    })
    .await?
    .ok_or_else(|| AppError::not_found(format!("target {target_id}")))?;

    // TODO: integrar fuentes externas (shodan, nvd, etc.)
    with_tenant_conn(pool, tenant_id, |conn| {
        Box::pin(async move {
            sqlx::query!(
                "UPDATE scan_targets SET last_scanned_at = now(), updated_at = now() WHERE id = $1",
                target_id
            )
            .execute(conn)
            .await
            .map_err(|e| AppError::Database(e.to_string()))
        })
    })
    .await?;

    info!(%target_id, name = %target.name, "scan completed");
    Ok(())
}
