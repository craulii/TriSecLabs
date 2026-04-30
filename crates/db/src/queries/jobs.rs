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
        "UPDATE jobs SET status = 'done', progress = 100, updated_at = now() WHERE id = $1",
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

/// Reencola un job para reintento con backoff. Restaura status='pending' y
/// retrasa la próxima ejecución `run_after`. El contador `attempts` ya fue
/// incrementado por `dequeue_next`.
pub async fn requeue_with_delay(
    pool: &PgPool,
    job_id: Uuid,
    delay_secs: i64,
    last_error: &str,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"UPDATE jobs
           SET status = 'pending',
               error = $2,
               run_after = now() + ($3 || ' seconds')::interval,
               updated_at = now()
           WHERE id = $1"#,
        job_id,
        last_error,
        delay_secs.to_string(),
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}

pub struct LatestJobRow {
    pub id:           Uuid,
    pub status:       String,
    pub attempts:     i32,
    pub error:        Option<String>,
    pub progress:     Option<i16>,
    pub current_step: Option<String>,
    pub stats_json:   serde_json::Value,
    pub created_at:   chrono::DateTime<chrono::Utc>,
    pub updated_at:   chrono::DateTime<chrono::Utc>,
}

pub async fn latest_scan_for_target(
    pool: &PgPool,
    tenant_id: Uuid,
    target_id: Uuid,
) -> Result<Option<LatestJobRow>, AppError> {
    sqlx::query_as!(
        LatestJobRow,
        r#"SELECT id, status::text AS "status!", attempts, error,
                  progress, current_step,
                  stats_json AS "stats_json!",
                  created_at, updated_at
           FROM jobs
           WHERE tenant_id = $1
             AND job_type = 'scan'
             AND payload->>'target_id' = $2
           ORDER BY created_at DESC
           LIMIT 1"#,
        tenant_id,
        target_id.to_string(),
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))
}

pub struct JobFullRow {
    pub id:           Uuid,
    pub tenant_id:    Uuid,
    pub job_type:     String,
    pub payload:      serde_json::Value,
    pub status:       String,
    pub attempts:     i32,
    pub max_attempts: i32,
    pub error:        Option<String>,
    pub progress:     Option<i16>,
    pub current_step: Option<String>,
    pub stats_json:   serde_json::Value,
    pub created_at:   chrono::DateTime<chrono::Utc>,
    pub updated_at:   chrono::DateTime<chrono::Utc>,
}

/// Lookup por id con filtro tenant_id. Aunque jobs no tiene RLS, filtramos
/// explícitamente para evitar leaks cross-tenant en endpoints de lectura.
pub async fn get_by_id(
    pool: &PgPool,
    tenant_id: Uuid,
    job_id: Uuid,
) -> Result<Option<JobFullRow>, AppError> {
    sqlx::query_as!(
        JobFullRow,
        r#"SELECT id, tenant_id,
                  job_type::text AS "job_type!",
                  payload,
                  status::text   AS "status!",
                  attempts, max_attempts, error,
                  progress, current_step,
                  stats_json AS "stats_json!",
                  created_at, updated_at
           FROM jobs
           WHERE id = $1 AND tenant_id = $2"#,
        job_id,
        tenant_id,
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))
}

/// Actualiza progreso/etapa/stats del job. Llamado desde el worker durante
/// la ejecución. Sin RLS — usa el pool privilegiado del worker.
/// `stats_merge` se aplica con el operador `||` de jsonb (merge superficial:
/// reemplaza claves de primer nivel).
pub async fn update_progress(
    pool: &PgPool,
    job_id: Uuid,
    progress: Option<i16>,
    current_step: Option<&str>,
    stats_merge: Option<&serde_json::Value>,
) -> Result<(), AppError> {
    let stats = stats_merge.cloned().unwrap_or_else(|| serde_json::json!({}));
    sqlx::query!(
        r#"UPDATE jobs
           SET progress     = COALESCE($2, progress),
               current_step = COALESCE($3, current_step),
               stats_json   = COALESCE(stats_json, '{}'::jsonb) || $4,
               updated_at   = now()
           WHERE id = $1"#,
        job_id,
        progress,
        current_step,
        stats,
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
