use common::AppError;
use sqlx::{PgConnection, Transaction, Postgres};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct VulnerabilityRow {
    pub id:               Uuid,
    pub target_id:        Uuid,
    pub port_id:          Option<Uuid>,
    pub fingerprint:      String,
    pub title:            String,
    pub description:      Option<String>,
    pub severity:         String,
    pub cvss_score:       Option<f64>,
    pub cvss_vector:      Option<String>,
    pub cve_id:           Option<String>,
    pub cwe_id:           Option<String>,
    pub status:           String,
    pub source:           String,
    pub evidence:         serde_json::Value,
    pub remediation_note: Option<String>,
    pub first_seen_at:    DateTime<Utc>,
    pub last_seen_at:     DateTime<Utc>,
    pub resolved_at:      Option<DateTime<Utc>>,
}

pub async fn list_for_target(
    conn: &mut PgConnection,
    target_id: Uuid,
) -> Result<Vec<VulnerabilityRow>, AppError> {
    sqlx::query_as!(
        VulnerabilityRow,
        r#"SELECT
               id, target_id, port_id, fingerprint, title, description,
               severity::text AS "severity!", cvss_score::float8 AS cvss_score, cvss_vector,
               cve_id, cwe_id, status::text AS "status!", source::text AS "source!",
               evidence, remediation_note, first_seen_at, last_seen_at, resolved_at
           FROM vulnerabilities
           WHERE target_id = $1
           ORDER BY
               CASE severity
                   WHEN 'critical' THEN 1 WHEN 'high' THEN 2
                   WHEN 'medium'   THEN 3 WHEN 'low'  THEN 4 ELSE 5
               END,
               first_seen_at DESC"#,
        target_id,
    )
    .fetch_all(conn)
    .await
    .map_err(|e| AppError::Database(e.to_string()))
}

pub async fn list_global(
    conn: &mut PgConnection,
    page: i64,
    limit: i64,
    severity: Option<&str>,
    status: Option<&str>,
) -> Result<Vec<VulnerabilityRow>, AppError> {
    // sqlx no soporta filtros opcionales con query_as! directamente.
    // Construimos la query dinámicamente con QueryBuilder.
    let mut qb = sqlx::QueryBuilder::new(
        r#"SELECT
               id, target_id, port_id, fingerprint, title, description,
               severity::text, cvss_score::float8 AS cvss_score, cvss_vector,
               cve_id, cwe_id, status::text, source::text,
               evidence, remediation_note, first_seen_at, last_seen_at, resolved_at
           FROM vulnerabilities WHERE 1=1"#,
    );

    if let Some(s) = severity {
        qb.push(" AND severity::text = ").push_bind(s);
    }
    if let Some(s) = status {
        qb.push(" AND status::text = ").push_bind(s);
    }

    qb.push(" ORDER BY first_seen_at DESC LIMIT ")
      .push_bind(limit)
      .push(" OFFSET ")
      .push_bind((page - 1) * limit);

    qb.build_query_as::<VulnerabilityRow>()
        .fetch_all(conn)
        .await
        .map_err(|e| AppError::Database(e.to_string()))
}

pub async fn update_status(
    tx: &mut Transaction<'static, Postgres>,
    id: Uuid,
    status: &str,
    note: Option<&str>,
) -> Result<(), AppError> {
    let resolved = if status == "resolved" {
        Some(chrono::Utc::now())
    } else {
        None
    };

    sqlx::query!(
        r#"UPDATE vulnerabilities
           SET status = $2::text::vuln_status,
               remediation_note = COALESCE($3, remediation_note),
               resolved_at = $4,
               updated_at = now()
           WHERE id = $1"#,
        id,
        status,
        note,
        resolved,
    )
    .execute(&mut **tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}
