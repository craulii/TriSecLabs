mod middleware;
mod routes;
mod state;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::http::{header, Method};
use axum::Router;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use db::create_pool;
use llm::LlmClient;
use state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let jwt_secret   = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let llm_base_url = std::env::var("LLM_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".into());
    let llm_model    = std::env::var("LLM_MODEL").unwrap_or_else(|_| "mistral-7b-instruct".into());
    let host         = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port         = std::env::var("PORT").unwrap_or_else(|_| "3000".into());

    // En Docker: STATIC_DIR=web/build. En dev local se sirve via Vite (:5173).
    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "apps/web/build".into());

    let db = create_pool(&database_url)
        .await
        .expect("failed to connect to PostgreSQL");

    sqlx::migrate!("../../migrations")
        .run(&db)
        .await
        .expect("failed to run migrations");

    info!("migrations applied");

    let llm   = LlmClient::new(llm_base_url, llm_model);
    let state = AppState {
        db,
        llm,
        jwt_secret,
        login_throttle: Arc::new(Mutex::new(HashMap::new())),
    };

    // CORS: orígenes fijos — SvelteKit dev + Tauri WebView (dos URL schemes por plataforma).
    // Headers y métodos explícitos: no se aceptan headers arbitrarios de otros orígenes.
    let cors = CorsLayer::new()
        .allow_origin([
            "http://localhost:5173".parse().unwrap(),   // Vite dev
            "tauri://localhost".parse().unwrap(),        // Tauri WebView (macOS/Linux)
            "https://tauri.localhost".parse().unwrap(),  // Tauri WebView (Windows)
        ])
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT]);

    let app = Router::new()
        .merge(routes::public_router(state.clone()))
        .merge(routes::protected_router(state.clone()))
        .nest_service(
            "/",
            ServeDir::new(&static_dir)
                .fallback(ServeFile::new(format!("{static_dir}/index.html"))),
        )
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http());

    let addr = format!("{host}:{port}");
    info!(address = %addr, "server listening");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app)
        .await
        .expect("server error");
}
