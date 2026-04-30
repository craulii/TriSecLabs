use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::TenantId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: TenantId,
    pub slug: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub email: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    Domain,
    IpRange,
    Vendor,
    Organization,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanTarget {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub kind: TargetKind,
    pub name: String,
    pub value: String,
    pub risk_score: Option<i16>,
    pub risk_level: Option<RiskLevel>,
    pub last_scanned_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanFinding {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub target_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub severity: RiskLevel,
    pub cve_id: Option<String>,
    pub cvss_score: Option<f32>,
    pub found_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub target_id: Uuid,
    pub content: String,
    pub model_used: String,
    pub generated_at: DateTime<Utc>,
}

// Payloads para jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanPayload {
    pub target_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisPayload {
    pub target_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmReportPayload {
    pub target_id: Uuid,
}
