//! Endpoint SSE para progreso en vivo de jobs.
//!
//! Difiere del resto de la API porque `EventSource` no permite headers custom,
//! por lo que el JWT se acepta vía query param `?token=...` además del header.

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::sse::{Event, KeepAlive, Sse},
    routing::get,
    Router,
};
use common::models::{DiscoveredPort, JobProgressEvent, ScanStage};
use common::tenant::{TenantContext, UserRole};
use common::TenantId;
use db::queries::jobs::{self, JobFullRow};
use futures::stream::{self, Stream, StreamExt};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use std::convert::Infallible;
use std::time::Duration;
use uuid::Uuid;

use crate::middleware::auth::JwtClaims;
use crate::state::AppState;

const POLL_INTERVAL: Duration = Duration::from_millis(800);
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);
const HARD_TIMEOUT: Duration = Duration::from_secs(900);

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/jobs/:id/stream", get(stream_job))
        .with_state(state)
}

#[derive(Deserialize)]
struct TokenQuery {
    token: Option<String>,
}

/// Resuelve el `TenantContext` a partir del header `Authorization` o, en su
/// defecto, del query param `?token=` (necesario para `EventSource`).
fn resolve_ctx(
    headers: &HeaderMap,
    query_token: Option<&str>,
    secret: &str,
) -> Result<TenantContext, StatusCode> {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::to_string)
        .or_else(|| query_token.map(str::to_string))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let key = DecodingKey::from_secret(secret.as_bytes());
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    validation.set_issuer(&["triseclabs"]);
    validation.set_audience(&["triseclabs-api"]);

    let claims = decode::<JwtClaims>(&token, &key, &validation)
        .map_err(|_| StatusCode::UNAUTHORIZED)?
        .claims;

    let tenant_id = TenantId::try_from(claims.tenant.as_str())
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let user_role = match claims.role.as_str() {
        "admin" => UserRole::Admin,
        _ => UserRole::Analyst,
    };

    Ok(TenantContext { tenant_id, user_id, user_role })
}

async fn stream_job(
    State(state): State<AppState>,
    Path(job_id): Path<Uuid>,
    Query(q): Query<TokenQuery>,
    headers: HeaderMap,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let ctx = resolve_ctx(&headers, q.token.as_deref(), &state.jwt_secret)?;
    let tenant_id = *ctx.tenant_id.as_uuid();

    let initial = jobs::get_by_id(&state.db, tenant_id, job_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    struct St {
        pool:             db::PgPool,
        tenant_id:        Uuid,
        job_id:           Uuid,
        last_updated:     chrono::DateTime<chrono::Utc>,
        emitted_initial:  bool,
        terminal_emitted: bool,
        initial_event:    Option<JobProgressEvent>,
    }

    let initial_event = row_to_event(&initial);
    let st = St {
        pool:             state.db.clone(),
        tenant_id,
        job_id,
        last_updated:     initial.updated_at,
        emitted_initial:  false,
        terminal_emitted: false,
        initial_event:    Some(initial_event),
    };

    // unfold genera (Option<event>, st) iterativamente; None termina el stream.
    let event_stream = stream::unfold(st, |mut st| async move {
        if st.terminal_emitted {
            return None;
        }

        // Primer evento: snapshot inicial sin esperar al tick.
        if !st.emitted_initial {
            st.emitted_initial = true;
            let ev = st.initial_event.take().unwrap();
            if is_terminal_status(&ev.status) {
                st.terminal_emitted = true;
            }
            return Some((ev, st));
        }

        // Polling: espera POLL_INTERVAL y consulta hasta detectar cambio o terminal.
        loop {
            tokio::time::sleep(POLL_INTERVAL).await;
            let row = match jobs::get_by_id(&st.pool, st.tenant_id, st.job_id).await {
                Ok(Some(r)) => r,
                Ok(None) => return None,
                Err(_)   => return None,
            };

            let changed = row.updated_at > st.last_updated;
            let is_terminal = is_terminal_status(&row.status);

            if changed || is_terminal {
                st.last_updated = row.updated_at;
                let ev = row_to_event(&row);
                if is_terminal {
                    st.terminal_emitted = true;
                }
                return Some((ev, st));
            }
        }
    });

    let sse_stream = event_stream
        .map(|ev| {
            let json = serde_json::to_string(&ev).unwrap_or_else(|_| "{}".into());
            Ok::<_, Infallible>(Event::default().data(json))
        })
        .take_until(tokio::time::sleep(HARD_TIMEOUT));

    Ok(Sse::new(sse_stream).keep_alive(
        KeepAlive::new()
            .interval(KEEPALIVE_INTERVAL)
            .text("ping"),
    ))
}

fn is_terminal_status(s: &str) -> bool {
    matches!(s, "done" | "failed")
}

/// Convierte la fila DB en el evento que consume el frontend.
pub fn row_to_event(row: &JobFullRow) -> JobProgressEvent {
    let stage = row.current_step.as_deref().and_then(parse_stage);

    let discovered_ports = row
        .stats_json
        .get("discovered_ports")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    let port = v.get("port")?.as_i64()? as i32;
                    let protocol = v.get("protocol")?.as_str()?.to_string();
                    let service = v
                        .get("service")
                        .and_then(|s| s.as_str())
                        .map(str::to_string);
                    Some(DiscoveredPort { port, protocol, service })
                })
                .collect()
        })
        .unwrap_or_default();

    let log = row
        .stats_json
        .get("log_tail")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    let note = row
        .stats_json
        .get("note")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    JobProgressEvent {
        id:               row.id,
        status:           row.status.clone(),
        stage,
        progress:         row.progress,
        discovered_ports,
        log,
        error:            row.error.clone(),
        note,
        updated_at:       row.updated_at,
    }
}

fn parse_stage(s: &str) -> Option<ScanStage> {
    Some(match s {
        "validating"        => ScanStage::Validating,
        "starting"          => ScanStage::Starting,
        "host_discovery"    => ScanStage::HostDiscovery,
        "port_scan"         => ScanStage::PortScan,
        "service_detection" => ScanStage::ServiceDetection,
        "vulners"           => ScanStage::Vulners,
        "persisting"        => ScanStage::Persisting,
        "done"              => ScanStage::Done,
        "failed"            => ScanStage::Failed,
        _                   => return None,
    })
}

