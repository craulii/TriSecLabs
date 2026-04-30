use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch, post},
    Json, Router,
};
use common::tenant::TenantContext;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        // Targets (alias legacy: /vendors)
        .route("/vendors",              get(list_targets).post(create_target))
        .route("/vendors/:id",          get(get_target).patch(update_target).delete(delete_target))
        .route("/vendors/:id/scan",     post(enqueue_scan))
        .route("/vendors/:id/analyze",  post(enqueue_llm_report))
        .route("/vendors/:id/job",      get(get_latest_job))
        // Jobs (lectura puntual)
        .route("/jobs/:id",             get(get_job_snapshot))
        // Ports & vulns bajo /targets (más semánticamente correcto)
        .route("/targets/:id/ports",           get(list_ports))
        .route("/targets/:id/vulnerabilities", get(list_vulns_for_target))
        .route("/targets/:id/metrics/:kind",   get(get_metrics))
        // Global vulnerabilities
        .route("/vulnerabilities",        get(list_vulns_global))
        .route("/vulnerabilities/:id/status", patch(update_vuln_status))
        .with_state(state)
}

// ─── Response types ───────────────────────────────────────────────────────────

#[derive(Serialize)]
struct JobAccepted {
    job_id: Uuid,
}

// ─── Targets ─────────────────────────────────────────────────────────────────

async fn list_targets(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
) -> impl IntoResponse {
    use db::{queries::scan_targets, with_tenant_conn};

    match with_tenant_conn(&state.db, ctx.tenant_id, |conn| {
        Box::pin(async move { scan_targets::list(conn).await })
    })
    .await
    {
        Ok(rows) => (StatusCode::OK, Json(rows)).into_response(),
        Err(e)   => err500(e),
    }
}

async fn get_target(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    use db::{queries::scan_targets, with_tenant_conn};

    match with_tenant_conn(&state.db, ctx.tenant_id, |conn| {
        Box::pin(async move { scan_targets::find_by_id(conn, id).await })
    })
    .await
    {
        Ok(Some(t)) => (StatusCode::OK, Json(t)).into_response(),
        Ok(None)    => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "not found" }))).into_response(),
        Err(e)      => err500(e),
    }
}

#[derive(Deserialize)]
struct CreateTargetBody {
    kind:  String,
    name:  String,
    value: String,
}

async fn create_target(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Json(body): Json<CreateTargetBody>,
) -> impl IntoResponse {
    use db::{queries::scan_targets, with_tenant};

    match with_tenant(&state.db, ctx.tenant_id, |tx| {
        Box::pin(async move {
            scan_targets::create(tx, ctx.tenant_id, &body.kind, &body.name, &body.value).await
        })
    })
    .await
    {
        Ok(t)  => (StatusCode::CREATED, Json(t)).into_response(),
        Err(e) => err500(e),
    }
}

#[derive(Deserialize)]
struct UpdateTargetBody {
    name:  String,
    value: String,
}

async fn update_target(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateTargetBody>,
) -> impl IntoResponse {
    use db::{queries::scan_targets, with_tenant};

    match with_tenant(&state.db, ctx.tenant_id, |tx| {
        Box::pin(async move {
            scan_targets::update(tx, id, &body.name, &body.value).await
        })
    })
    .await
    {
        Ok(t)  => (StatusCode::OK, Json(t)).into_response(),
        Err(e) => err500(e),
    }
}

async fn delete_target(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    use db::{queries::scan_targets, with_tenant_conn};

    match with_tenant_conn(&state.db, ctx.tenant_id, |conn| {
        Box::pin(async move { scan_targets::delete(conn, id).await })
    })
    .await
    {
        Ok(true)  => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "not found" }))).into_response(),
        Err(e)    => err500(e),
    }
}

async fn get_latest_job(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(target_id): Path<Uuid>,
) -> impl IntoResponse {
    use db::queries::jobs;

    match jobs::latest_scan_for_target(&state.db, *ctx.tenant_id.as_uuid(), target_id).await {
        Ok(Some(j)) => (StatusCode::OK, Json(serde_json::json!({
            "id":           j.id,
            "status":       j.status,
            "attempts":     j.attempts,
            "error":        j.error,
            "progress":     j.progress,
            "current_step": j.current_step,
            "stats_json":   j.stats_json,
            "created_at":   j.created_at,
            "updated_at":   j.updated_at,
        }))).into_response(),
        Ok(None)    => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "no jobs" }))).into_response(),
        Err(e)      => err500(e),
    }
}

async fn get_job_snapshot(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(job_id): Path<Uuid>,
) -> impl IntoResponse {
    use db::queries::jobs;

    match jobs::get_by_id(&state.db, *ctx.tenant_id.as_uuid(), job_id).await {
        Ok(Some(j)) => (StatusCode::OK, Json(crate::routes::stream::row_to_event(&j))).into_response(),
        Ok(None)    => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "not found" }))).into_response(),
        Err(e)      => err500(e),
    }
}

async fn enqueue_scan(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    enqueue_job(&state, ctx, id, "scan").await
}

async fn enqueue_llm_report(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    enqueue_job(&state, ctx, id, "llm_report").await
}

async fn enqueue_job(
    state: &AppState,
    ctx: TenantContext,
    target_id: Uuid,
    job_type: &str,
) -> impl IntoResponse {
    use db::queries::jobs;
    let payload = serde_json::json!({ "target_id": target_id });
    match jobs::enqueue(&state.db, *ctx.tenant_id.as_uuid(), job_type, &payload).await {
        Ok(job_id) => (StatusCode::ACCEPTED, Json(JobAccepted { job_id })).into_response(),
        Err(e)     => err500(e),
    }
}

// ─── Ports ────────────────────────────────────────────────────────────────────

async fn list_ports(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(target_id): Path<Uuid>,
) -> impl IntoResponse {
    use db::{queries::ports, with_tenant_conn};

    match with_tenant_conn(&state.db, ctx.tenant_id, |conn| {
        Box::pin(async move { ports::list_for_target(conn, target_id).await })
    })
    .await
    {
        Ok(rows) => (StatusCode::OK, Json(rows)).into_response(),
        Err(e)   => err500(e),
    }
}

// ─── Vulnerabilities ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct VulnListParams {
    page:     Option<i64>,
    limit:    Option<i64>,
    severity: Option<String>,
    status:   Option<String>,
}

async fn list_vulns_for_target(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(target_id): Path<Uuid>,
) -> impl IntoResponse {
    use db::{queries::vulnerabilities, with_tenant_conn};

    match with_tenant_conn(&state.db, ctx.tenant_id, |conn| {
        Box::pin(async move { vulnerabilities::list_for_target(conn, target_id).await })
    })
    .await
    {
        Ok(rows) => (StatusCode::OK, Json(rows)).into_response(),
        Err(e)   => err500(e),
    }
}

async fn list_vulns_global(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Query(params): Query<VulnListParams>,
) -> impl IntoResponse {
    use db::{queries::vulnerabilities, with_tenant_conn};

    let page  = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(25).clamp(1, 100);

    match with_tenant_conn(&state.db, ctx.tenant_id, |conn| {
        let sev    = params.severity.clone();
        let status = params.status.clone();
        Box::pin(async move {
            vulnerabilities::list_global(conn, page, limit, sev.as_deref(), status.as_deref()).await
        })
    })
    .await
    {
        Ok(rows) => (StatusCode::OK, Json(rows)).into_response(),
        Err(e)   => err500(e),
    }
}

#[derive(Deserialize)]
struct UpdateStatusBody {
    status: String,
    note:   Option<String>,
}

async fn update_vuln_status(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateStatusBody>,
) -> impl IntoResponse {
    use db::{queries::vulnerabilities, with_tenant};

    match with_tenant(&state.db, ctx.tenant_id, |tx| {
        let status = body.status.clone();
        let note   = body.note.clone();
        Box::pin(async move {
            vulnerabilities::update_status(tx, id, &status, note.as_deref()).await
        })
    })
    .await
    {
        Ok(())  => StatusCode::NO_CONTENT.into_response(),
        Err(e)  => err500(e),
    }
}

// ─── Metrics ─────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct MetricsParams {
    limit: Option<i64>,
}

async fn get_metrics(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path((target_id, kind)): Path<(Uuid, String)>,
    Query(params): Query<MetricsParams>,
) -> impl IntoResponse {
    use db::{queries::metrics, with_tenant_conn};

    let limit = params.limit.unwrap_or(30).clamp(1, 90);

    match with_tenant_conn(&state.db, ctx.tenant_id, |conn| {
        let kind = kind.clone();
        Box::pin(async move {
            metrics::history(conn, target_id, &kind, limit).await
        })
    })
    .await
    {
        Ok(rows) => (StatusCode::OK, Json(rows)).into_response(),
        Err(e)   => err500(e),
    }
}

// ─── Error helper ─────────────────────────────────────────────────────────────

fn err500(e: common::AppError) -> axum::response::Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": e.to_string() })),
    )
        .into_response()
}
