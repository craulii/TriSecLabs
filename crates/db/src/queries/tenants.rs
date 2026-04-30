use common::AppError;
use sqlx::PgPool;
use uuid::Uuid;

pub struct TenantRow {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
}

pub async fn find_by_slug(pool: &PgPool, slug: &str) -> Result<Option<TenantRow>, AppError> {
    sqlx::query_as!(
        TenantRow,
        "SELECT id, slug, name FROM tenants WHERE slug = $1",
        slug
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))
}

pub async fn create(pool: &PgPool, slug: &str, name: &str) -> Result<TenantRow, AppError> {
    sqlx::query_as!(
        TenantRow,
        "INSERT INTO tenants (slug, name) VALUES ($1, $2) RETURNING id, slug, name",
        slug,
        name
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))
}
