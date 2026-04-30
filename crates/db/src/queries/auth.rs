use common::AppError;
use sqlx::PgPool;
use uuid::Uuid;

pub struct UserAuthRow {
    pub user_id:       Uuid,
    pub tenant_id:     Uuid,
    pub email:         String,
    pub role:          String,
    pub password_hash: String,
}

/// Busca un usuario por tenant slug + email.
/// No usa RLS (no hay tenant_id de sesión aún — es el login).
pub async fn find_user_by_tenant_and_email(
    pool: &PgPool,
    tenant_slug: &str,
    email: &str,
) -> Result<Option<UserAuthRow>, AppError> {
    sqlx::query_as!(
        UserAuthRow,
        r#"SELECT u.id            AS user_id,
                  u.tenant_id,
                  u.email,
                  u.role::text       AS "role!",
                  u.password_hash
           FROM users u
           JOIN tenants t ON t.id = u.tenant_id
           WHERE t.slug  = $1
             AND u.email = $2
             AND u.active = true"#,
        tenant_slug,
        email,
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))
}
