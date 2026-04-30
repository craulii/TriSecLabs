pub mod api;
pub mod auth;
pub mod stream;

use axum::{middleware as axum_middleware, Router};
use crate::state::AppState;

/// Router público: /api/auth/* (sin JWT middleware)
pub fn public_router(state: AppState) -> Router {
    Router::new()
        .nest("/api/auth", auth::router(state.clone()))
}

/// Router protegido: /api/* con JWT middleware aplicado
pub fn protected_router(state: AppState) -> Router {
    Router::new()
        .nest("/api", api::router(state.clone()))
        .route_layer(axum_middleware::from_fn_with_state(
            state,
            crate::middleware::auth_middleware,
        ))
}

/// Router de streams SSE: hace su propia validación JWT (header o query param)
/// porque `EventSource` no permite headers custom.
pub fn stream_router(state: AppState) -> Router {
    Router::new().nest("/api", stream::router(state))
}
