pub mod api;
pub mod auth;

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
