// Tipos que espejo exacto de los modelos Rust en crates/common/src/models.rs
// Actualizar aquí cuando cambie el schema.

export type RiskLevel   = 'critical' | 'high' | 'medium' | 'low' | 'info';
export type VulnStatus  = 'open' | 'in_progress' | 'mitigated' | 'resolved' | 'accepted' | 'false_positive';
export type VulnSource  = 'nmap' | 'nessus' | 'openvas' | 'manual' | 'llm_analysis' | 'osint';
export type TargetKind  = 'domain' | 'ip_range' | 'vendor' | 'organization';
export type MetricKind  =
  | 'risk_score' | 'vuln_count_critical' | 'vuln_count_high'
  | 'vuln_count_medium' | 'vuln_count_low' | 'exposed_port_count'
  | 'mean_time_to_remediate' | 'scan_coverage';

export interface Tenant {
  id:         string;
  slug:       string;
  name:       string;
  created_at: string;
}

export interface User {
  id:         string;
  tenant_id:  string;
  email:      string;
  role:       'admin' | 'analyst';
  created_at: string;
}

export interface ScanTarget {
  id:             string;
  tenant_id:      string;
  kind:           TargetKind;
  name:           string;
  value:          string;
  risk_score:     number | null;
  risk_level:     RiskLevel | null;
  last_scanned_at: string | null;
  created_at:     string;
}

export interface Vulnerability {
  id:           string;
  tenant_id:    string;
  target_id:    string;
  port_id:      string | null;
  fingerprint:  string;
  title:        string;
  description:  string | null;
  severity:     RiskLevel;
  cvss_score:   number | null;
  cvss_vector:  string | null;
  cve_id:       string | null;
  cwe_id:       string | null;
  status:       VulnStatus;
  source:       VulnSource;
  evidence:     Record<string, unknown>;
  remediation_note: string | null;
  first_seen_at: string;
  last_seen_at:  string;
  resolved_at:   string | null;
}

export interface ExposedPort {
  id:           string;
  tenant_id:    string;
  target_id:    string;
  port:         number;
  protocol:     string;
  state:        string;
  service:      string | null;
  banner:       string | null;
  product:      string | null;
  version:      string | null;
  is_active:    boolean;
  first_seen_at: string;
  last_seen_at:  string;
}

export interface Metric {
  kind:         MetricKind;
  value:        number;
  period_start: string;
  computed_at:  string;
}

export interface Report {
  id:           string;
  tenant_id:    string;
  target_id:    string;
  content:      string;
  model_used:   string;
  generated_at: string;
}

export interface Job {
  id:          string;
  tenant_id:   string;
  job_type:    'scan' | 'analysis' | 'llm_report';
  status:      'pending' | 'running' | 'done' | 'failed';
  attempts:    number;
  error:       string | null;
  created_at:  string;
}

// ─── Respuestas de la API ────────────────────────────────────────────────────

export interface LoginResponse {
  token:     string;
  user_id:   string;
  tenant_id: string;
  role:      'admin' | 'analyst';
}

export interface JobAccepted {
  job_id: string;
}

export interface PaginatedResponse<T> {
  data:  T[];
  total: number;
  page:  number;
  limit: number;
}
