use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::Response,
    Extension,
};
use common::tenant::{TenantContext, UserRole};
use common::TenantId;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub:    String,   // user_id
    pub tenant: String,   // tenant_id
    pub role:   String,   // "admin" | "analyst"
    pub iss:    String,   // "triseclabs"
    pub aud:    String,   // "triseclabs-api"
    pub iat:    usize,
    pub exp:    usize,
}

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = extract_bearer_token(request.headers())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let claims = verify_token(&token, &state.jwt_secret)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let tenant_id = TenantId::try_from(claims.tenant.as_str())
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let user_role = match claims.role.as_str() {
        "admin" => UserRole::Admin,
        _ => UserRole::Analyst,
    };

    let ctx = TenantContext { tenant_id, user_id, user_role };
    request.extensions_mut().insert(ctx);

    Ok(next.run(request).await)
}

fn extract_bearer_token(headers: &axum::http::HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    value.strip_prefix("Bearer ").map(String::from)
}

fn verify_token(token: &str, secret: &str) -> Result<JwtClaims, jsonwebtoken::errors::Error> {
    let key = DecodingKey::from_secret(secret.as_bytes());
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    validation.set_issuer(&["triseclabs"]);
    validation.set_audience(&["triseclabs-api"]);
    let data = decode::<JwtClaims>(token, &key, &validation)?;
    Ok(data.claims)
}
