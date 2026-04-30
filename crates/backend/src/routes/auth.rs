use std::time::Instant;

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{middleware::auth::JwtClaims, state::AppState};

// 5 intentos por cuenta dentro de una ventana de 15 minutos.
const MAX_ATTEMPTS: u32  = 5;
const WINDOW_SECS:  u64  = 900;

// Hash dummy de costo 12 para simular bcrypt cuando el usuario no existe.
// Previene enumeración de cuentas válidas por tiempo de respuesta.
const DUMMY_HASH: &str =
    "$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj/RK.s5vxou";

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/login", post(login))
        .with_state(state)
}

#[derive(Deserialize)]
struct LoginRequest {
    tenant_slug: String,
    email:       String,
    password:    String,
}

#[derive(Serialize)]
struct LoginResponse {
    token:     String,
    user_id:   Uuid,
    tenant_id: Uuid,
    role:      String,
}

async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> impl IntoResponse {
    use db::queries::auth as auth_queries;

    let throttle_key = format!(
        "{}:{}",
        body.tenant_slug.to_lowercase(),
        body.email.to_lowercase()
    );

    // Rate limiting por cuenta — sin awaits dentro del bloque (Mutex sync).
    {
        let mut map = state.login_throttle.lock().unwrap();
        // GC lazy: eliminar entradas cuya ventana ya expiró.
        map.retain(|_, (_, start)| start.elapsed().as_secs() < WINDOW_SECS);
        let entry = map.entry(throttle_key.clone()).or_insert((0, Instant::now()));
        if entry.0 >= MAX_ATTEMPTS {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                Json(serde_json::json!({
                    "error": "too many login attempts, try again in 15 minutes"
                })),
            ).into_response();
        }
        entry.0 += 1;
    }

    let user_opt = match auth_queries::find_user_by_tenant_and_email(
        &state.db,
        &body.tenant_slug,
        &body.email,
    )
    .await
    {
        Ok(opt)  => opt,
        Err(e)   => return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ).into_response(),
    };

    // Siempre ejecutar bcrypt, exista o no el usuario.
    // Si el usuario no existe, verificar contra DUMMY_HASH (mismo costo=12).
    // Esto previene enumeración de cuentas válidas por diferencia de tiempo.
    let hash_to_check = user_opt
        .as_ref()
        .map(|u| u.password_hash.as_str())
        .unwrap_or(DUMMY_HASH);

    let password_ok = bcrypt::verify(&body.password, hash_to_check).unwrap_or(false);

    if user_opt.is_none() || !password_ok {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "invalid credentials" })),
        ).into_response();
    }

    let user_row = user_opt.unwrap();

    // Login exitoso: resetear contador de intentos para esta cuenta.
    {
        let mut map = state.login_throttle.lock().unwrap();
        map.remove(&throttle_key);
    }

    let now = Utc::now();
    let exp = (now + Duration::hours(8)).timestamp() as usize;
    let iat = now.timestamp() as usize;

    let claims = JwtClaims {
        sub:    user_row.user_id.to_string(),
        tenant: user_row.tenant_id.to_string(),
        role:   user_row.role.clone(),
        iss:    "triseclabs".to_string(),
        aud:    "triseclabs-api".to_string(),
        iat,
        exp,
    };

    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .expect("JWT encoding failed");

    (StatusCode::OK, Json(LoginResponse {
        token,
        user_id:   user_row.user_id,
        tenant_id: user_row.tenant_id,
        role:      user_row.role,
    })).into_response()
}
