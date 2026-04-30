mod handlers;

use common::{AppError, TenantId};
use db::{create_pool, queries::jobs};
use llm::LlmClient;
use std::time::Duration;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use uuid::Uuid;

/// WorkerState — inmutable, cloneable, compartido entre iteraciones del loop.
#[derive(Clone)]
struct WorkerState {
    pool: db::PgPool,
    llm:  LlmClient,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let llm_base_url = std::env::var("LLM_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:8080".into());
    let llm_model = std::env::var("LLM_MODEL")
        .unwrap_or_else(|_| "mistral-7b-instruct".into());

    let pool = create_pool(&database_url)
        .await
        .expect("failed to connect to PostgreSQL");

    let state = WorkerState {
        pool,
        llm: LlmClient::new(llm_base_url, llm_model),
    };

    info!("worker started, polling for jobs");

    loop {
        match jobs::dequeue_next(&state.pool).await {
            Err(e) => {
                error!(error = %e, "dequeue error, backing off");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            Ok(None) => {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            Ok(Some(job)) => {
                info!(job_id = %job.id, job_type = %job.job_type, "processing job");

                let tenant_id = TenantId::from(job.tenant_id);
                let result = dispatch(&state, &job.job_type, job.id, tenant_id, &job.payload).await;

                match result {
                    Ok(()) => {
                        if let Err(e) = jobs::mark_done(&state.pool, job.id).await {
                            error!(job_id = %job.id, error = %e, "failed to mark job done");
                        }
                        info!(job_id = %job.id, "job completed");
                    }
                    Err(e) => {
                        let attempts_left = job.max_attempts.saturating_sub(job.attempts);
                        let should_retry = matches!(&e, AppError::Internal(_)) && attempts_left > 0;

                        if should_retry {
                            warn!(job_id = %job.id, attempts_left, error = %e, "transient error, requeuing");
                            if let Err(re) = jobs::requeue_with_delay(&state.pool, job.id, 60, &e.to_string()).await {
                                error!(job_id = %job.id, error = %re, "failed to requeue, marking failed");
                                let _ = jobs::mark_failed(&state.pool, job.id, &e.to_string()).await;
                            }
                        } else {
                            warn!(job_id = %job.id, error = %e, "job failed");
                            if let Err(mark_err) = jobs::mark_failed(&state.pool, job.id, &e.to_string()).await {
                                error!(job_id = %job.id, error = %mark_err, "failed to mark job failed");
                            }
                        }
                    }
                }
            }
        }
    }
}

async fn dispatch(
    state: &WorkerState,
    job_type: &str,
    job_id: Uuid,
    tenant_id: TenantId,
    payload: &serde_json::Value,
) -> Result<(), AppError> {
    match job_type {
        "scan"       => handlers::scan::handle(&state.pool, job_id, tenant_id, payload).await,
        "analysis"   => handlers::analysis::handle(&state.pool, tenant_id, payload).await,
        "llm_report" => handlers::llm_report::handle(&state.pool, &state.llm, tenant_id, payload).await,
        other        => Err(AppError::internal(format!("unknown job type: {other}"))),
    }
}
