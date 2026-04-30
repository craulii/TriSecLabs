use common::AppError;
use sqlx::PgPool;
use uuid::Uuid;

pub struct JobRow {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub job_type: String,
    pub payload: serde_json::Value,
    pub status: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub error: Option<String>,
}

/// Dequeue atómico: marca el job como 'running' y lo retorna.
/// SKIP LOCKED garantiza que múltiples workers no tomen el mismo job.
/// Los jobs NO tienen RLS — el worker usa el pool directamente.
pub async fn dequeue_next(pool: &PgPool) -> Result<Option<JobRow>, AppError> {
    sqlx::query_as!(
        JobRow,
        r#"UPDATE jobs
           SET status = 'running'::job_status,
               attempts = attempts + 1,
               updated_at = now()
           WHERE id = (
               SELECT id FROM jobs
               WHERE status = 'pending'::job_status
                 AND run_after <= now()
                 AND attempts < max_attempts
               ORDER BY created_at
               FOR UPDATE SKIP LOCKED
               LIMIT 1
           )
           RETURNING
               id,
               tenant_id,
               job_type::text AS "job_type!",
               payload,
               status::text   AS "status!",
               attempts,
               max_attempts,
               error"#
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))
}

pub async fn mark_done(pool: &PgPool, job_id: Uuid) -> Result<(), AppError> {
    sqlx::query!(
        "UPDATE jobs SET status = 'done', updated_at = now() WHERE id = $1",
        job_id
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}

pub async fn mark_failed(pool: &PgPool, job_id: Uuid, error: &str) -> Result<(), AppError> {
    sqlx::query!(
        "UPDATE jobs SET status = 'failed', error = $2, updated_at = now() WHERE id = $1",
        job_id,
        error
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}

pub async fn enqueue(
    pool: &PgPool,
    tenant_id: Uuid,
    job_type: &str,
    payload: &serde_json::Value,
) -> Result<Uuid, AppError> {
    let row = sqlx::query!(
        r#"INSERT INTO jobs (tenant_id, job_type, payload)
           VALUES ($1, $2::text::job_type, $3)
           RETURNING id"#,
        tenant_id,
        job_type,
        payload
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(row.id)
}
