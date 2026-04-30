use common::{AppError, TenantId};
use sqlx::{PgConnection, PgPool};
use std::future::Future;
use std::pin::Pin;

/// Alias para futures boxeados con lifetime — patrón necesario para closures
/// async que reciben `&mut PgConnection` (el compilador no puede inferir el tipo
/// del future sin boxearlo cuando la referencia tiene un lifetime arbitrario).
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Ejecuta `f` sobre una conexión del pool con `app.tenant_id` configurado.
/// Al finalizar (ok o error) resetea el setting para que la conexión pueda
/// devolverse al pool sin riesgo de leakage.
///
/// Uso:
/// ```rust,ignore
/// with_tenant_conn(pool, tenant_id, |conn| Box::pin(async move {
///     queries::scan_targets::list(conn).await
/// })).await?;
/// ```
pub async fn with_tenant_conn<F, T>(
    pool: &PgPool,
    tenant_id: TenantId,
    f: F,
) -> Result<T, AppError>
where
    F: for<'c> FnOnce(&'c mut PgConnection) -> BoxFuture<'c, Result<T, AppError>>,
    T: Send,
{
    let mut conn = pool.acquire().await.map_err(|e| AppError::Database(e.to_string()))?;

    sqlx::query("SELECT set_config('app.tenant_id', $1, false)")
        .bind(tenant_id.to_string())
        .execute(&mut *conn)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let result = f(&mut conn).await;

    // Siempre limpiar, incluso en error — previene leakage al pool
    let _ = sqlx::query("RESET app.tenant_id")
        .execute(&mut *conn)
        .await;

    result
}

/// Versión transaccional: usa `SET LOCAL` (equivalente a `set_config(..., true)`)
/// que PostgreSQL resetea automáticamente al terminar la transacción.
/// Úsala cuando necesites atomicidad de escritura.
///
/// El closure debe retornar `(T, Transaction)` para que `with_tenant` pueda
/// hacer commit después. Si el closure retorna Err, la transacción hace rollback.
pub async fn with_tenant<F, T>(
    pool: &PgPool,
    tenant_id: TenantId,
    f: F,
) -> Result<T, AppError>
where
    F: for<'t> FnOnce(
        &'t mut sqlx::Transaction<'static, sqlx::Postgres>,
    ) -> BoxFuture<'t, Result<T, AppError>>,
    T: Send,
{
    let mut tx = pool.begin().await.map_err(|e| AppError::Database(e.to_string()))?;

    // set_config(name, value, is_local=true) == SET LOCAL — válido solo en la transacción
    sqlx::query("SELECT set_config('app.tenant_id', $1, true)")
        .bind(tenant_id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let result = f(&mut tx).await;

    match result {
        Ok(value) => {
            tx.commit().await.map_err(|e| AppError::Database(e.to_string()))?;
            Ok(value)
        }
        Err(e) => {
            // rollback automático al drop, pero lo hacemos explícito
            let _ = tx.rollback().await;
            Err(e)
        }
    }
}
