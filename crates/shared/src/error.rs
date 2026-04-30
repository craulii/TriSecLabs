use thiserror::Error;

/// Error unificado de la aplicación.
/// `Database` toma String para que `common` no dependa de sqlx
/// (sqlx no compila a WASM y `app` se compila para ambos targets).
#[derive(Debug, Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("unauthorized")]
    Unauthorized,

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl AppError {
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}

