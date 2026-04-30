mod handlers;

use common::TenantId;
use db::{create_pool, queries::jobs};
use llm::LlmClient;
use std::time::Duration;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// WorkerState — inmutable, cloneable, compartido entre iteraciones del loop.
#[derive(Clone)]
struct WorkerState {
    pool: db::PgPool,
    llm: LlmClient,
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

    // Poll loop — LLM jobs son secuenciales (concurrencia=1) por diseño.
    // Los jobs scan/analysis se podrían paralelizar en el futuro con un semáforo.
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

                let result = dispatch(&state, &job.job_type, tenant_id, &job.payload).await;

                match result {
                    Ok(()) => {
                        if let Err(e) = jobs::mark_done(&state.pool, job.id).await {
                            error!(job_id = %job.id, error = %e, "failed to mark job done");
                        }
                        info!(job_id = %job.id, "job completed");
                    }
                    Err(e) => {
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

async fn dispatch(
    state: &WorkerState,
    job_type: &str,
    tenant_id: TenantId,
    payload: &serde_json::Value,
) -> Result<(), common::AppError> {
    match job_type {
        "scan" => handlers::scan::handle(&state.pool, tenant_id, payload).await,
        "analysis" => handlers::analysis::handle(&state.pool, tenant_id, payload).await,
        "llm_report" => handlers::llm_report::handle(&state.pool, &state.llm, tenant_id, payload).await,
        other => Err(common::AppError::internal(format!("unknown job type: {other}"))),
    }
}
