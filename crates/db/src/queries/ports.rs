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
