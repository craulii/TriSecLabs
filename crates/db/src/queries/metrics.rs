use common::AppError;
use sqlx::PgConnection;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct MetricRow {
    pub kind:         String,
    pub value:        f64,
    pub period_start: DateTime<Utc>,
}

/// Historial de una métrica para un target, limitado a `limit` períodos más recientes.
pub async fn history(
    conn: &mut PgConnection,
    target_id: Uuid,
    kind: &str,
    limit: i64,
) -> Result<Vec<MetricRow>, AppError> {
    sqlx::query_as!(
        MetricRow,
        r#"SELECT
               kind::text AS "kind!",
               value::float8 AS "value!",
               period_start
           FROM metrics
           WHERE target_id = $1
             AND kind::text = $2
           ORDER BY period_start DESC
           LIMIT $3"#,
        target_id,
        kind,
        limit,
    )
    .fetch_all(conn)
    .await
    .map_err(|e| AppError::Database(e.to_string()))
}
