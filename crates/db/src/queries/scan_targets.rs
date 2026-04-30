use common::{AppError, TenantId};
use sqlx::{PgConnection, Transaction, Postgres};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ScanTargetRow {
    pub id:              Uuid,
    pub tenant_id:       Uuid,
    pub name:            String,
    pub value:           String,
    pub kind:            String,
    pub risk_score:      Option<i16>,
    pub risk_level:      Option<String>,
    pub last_scanned_at: Option<DateTime<Utc>>,
    pub created_at:      DateTime<Utc>,
}

/// Lista targets del tenant (RLS aplicado automáticamente).
pub async fn list(conn: &mut PgConnection) -> Result<Vec<ScanTargetRow>, AppError> {
    sqlx::query_as!(
        ScanTargetRow,
        r#"SELECT id, tenant_id, name, value, kind::text AS "kind!", risk_score,
                  risk_level::text AS "risk_level", last_scanned_at, created_at
           FROM scan_targets
           ORDER BY created_at DESC
           LIMIT 200"#
    )
    .fetch_all(conn)
    .await
    .map_err(|e| AppError::Database(e.to_string()))
}

pub async fn find_by_id(
    conn: &mut PgConnection,
    id: Uuid,
) -> Result<Option<ScanTargetRow>, AppError> {
    sqlx::query_as!(
        ScanTargetRow,
        r#"SELECT id, tenant_id, name, value, kind::text AS "kind!", risk_score,
                  risk_level::text AS "risk_level", last_scanned_at, created_at
           FROM scan_targets
           WHERE id = $1"#,
        id
    )
    .fetch_optional(conn)
    .await
    .map_err(|e| AppError::Database(e.to_string()))
}

pub async fn create(
    tx: &mut Transaction<'static, Postgres>,
    tenant_id: TenantId,
    kind: &str,
    name: &str,
    value: &str,
) -> Result<ScanTargetRow, AppError> {
    sqlx::query_as!(
        ScanTargetRow,
        r#"INSERT INTO scan_targets (tenant_id, kind, name, value)
           VALUES ($1, $2::text::target_kind, $3, $4)
           RETURNING id, tenant_id, name, value, kind::text AS "kind!", risk_score,
                     risk_level::text AS "risk_level", last_scanned_at, created_at"#,
        *tenant_id.as_uuid(),
        kind,
        name,
        value,
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))
}

pub async fn update(
    tx: &mut Transaction<'static, Postgres>,
    id: Uuid,
    name: &str,
    value: &str,
) -> Result<ScanTargetRow, AppError> {
    sqlx::query_as!(
        ScanTargetRow,
        r#"UPDATE scan_targets
           SET name = $2, value = $3, updated_at = now()
           WHERE id = $1
           RETURNING id, tenant_id, name, value, kind::text AS "kind!", risk_score,
                     risk_level::text AS "risk_level", last_scanned_at, created_at"#,
        id,
        name,
        value,
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))
}

pub async fn delete(conn: &mut PgConnection, id: Uuid) -> Result<bool, AppError> {
    let result = sqlx::query!(
        "DELETE FROM scan_targets WHERE id = $1",
        id
    )
    .execute(conn)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(result.rows_affected() > 0)
}

pub async fn update_risk(
    conn: &mut PgConnection,
    id: Uuid,
    risk_score: i16,
    risk_level: &str,
) -> Result<(), AppError> {
    sqlx::query!(
        "UPDATE scan_targets
         SET risk_score = $2, risk_level = $3::text::risk_level, updated_at = now()
         WHERE id = $1",
        id,
        risk_score,
        risk_level
    )
    .execute(conn)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}
