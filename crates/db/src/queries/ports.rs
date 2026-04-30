use common::AppError;
use sqlx::PgConnection;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ExposedPortRow {
    pub id:         Uuid,
    pub target_id:  Uuid,
    pub port:       i32,
    pub protocol:   String,
    pub state:      String,
    pub service:    Option<String>,
    pub product:    Option<String>,
    pub version:    Option<String>,
    pub banner:     Option<String>,
    pub is_active:  bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn upsert_port(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    target_id: Uuid,
    port: i32,
    protocol: &str,
    state: &str,
    service: Option<&str>,
    product: Option<&str>,
    version: Option<&str>,
) -> Result<Uuid, AppError> {
    let row = sqlx::query!(
        r#"INSERT INTO exposed_ports
               (tenant_id, target_id, port, protocol, state, service, product, version,
                is_active, last_seen_at, updated_at)
           VALUES ($1, $2, $3, $4::text::port_protocol, $5::text::port_state, $6, $7, $8, true, now(), now())
           ON CONFLICT (tenant_id, target_id, port, protocol)
           DO UPDATE SET
               state        = EXCLUDED.state,
               service      = EXCLUDED.service,
               product      = EXCLUDED.product,
               version      = EXCLUDED.version,
               is_active    = true,
               last_seen_at = now(),
               updated_at   = now()
           RETURNING id"#,
        tenant_id,
        target_id,
        port,
        protocol,
        state,
        service,
        product,
        version,
    )
    .fetch_one(conn)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(row.id)
}

pub async fn deactivate_stale(
    conn: &mut PgConnection,
    target_id: Uuid,
    seen_ids: &[Uuid],
) -> Result<(), AppError> {
    sqlx::query!(
        r#"UPDATE exposed_ports
           SET is_active = false, updated_at = now()
           WHERE target_id = $1
             AND is_active = true
             AND NOT (id = ANY($2))"#,
        target_id,
        seen_ids as &[Uuid],
    )
    .execute(conn)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

pub async fn list_for_target(
    conn: &mut PgConnection,
    target_id: Uuid,
) -> Result<Vec<ExposedPortRow>, AppError> {
    sqlx::query_as!(
        ExposedPortRow,
        r#"SELECT
               id, target_id, port, protocol::text AS "protocol!", state::text AS "state!",
               service, product, version, banner, is_active, created_at, updated_at
           FROM exposed_ports
           WHERE target_id = $1
             AND is_active = true
           ORDER BY port"#,
        target_id,
    )
    .fetch_all(conn)
    .await
    .map_err(|e| AppError::Database(e.to_string()))
}
