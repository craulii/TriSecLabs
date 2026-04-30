use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Newtype sobre Uuid para identificar tenants.
/// Fuerza uso explícito — no se puede pasar un Uuid genérico donde se espera TenantId.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TenantId(pub Uuid);

impl TenantId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for TenantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<Uuid> for TenantId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl TryFrom<&str> for TenantId {
    type Error = uuid::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

/// Contexto de tenant inyectado por el middleware de Axum en cada request.
#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant_id: TenantId,
    pub user_id: Uuid,
    pub user_role: UserRole,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserRole {
    Analyst,
    Admin,
}

impl UserRole {
    pub fn is_admin(&self) -> bool {
        matches!(self, UserRole::Admin)
    }
}
