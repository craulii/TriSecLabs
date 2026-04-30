-- Vulnerabilidades estructuradas (lifecycle completo) y puertos expuestos.
-- Complementan scan_findings: findings = output raw del scanner,
-- vulnerabilities = entidad de negocio con ciclo de vida y deduplicación.

-- ─── Exposed Ports ──────────────────────────────────────────────────────────

CREATE TYPE port_protocol AS ENUM ('tcp', 'udp', 'sctp');
CREATE TYPE port_state    AS ENUM ('open', 'filtered', 'closed');

CREATE TABLE exposed_ports (
    id            UUID          PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id     UUID          NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    target_id     UUID          NOT NULL REFERENCES scan_targets(id) ON DELETE CASCADE,
    port          INTEGER       NOT NULL CHECK (port BETWEEN 1 AND 65535),
    protocol      port_protocol NOT NULL DEFAULT 'tcp',
    state         port_state    NOT NULL DEFAULT 'open',
    service       TEXT,
    banner        TEXT,
    product       TEXT,
    version       TEXT,
    extra_info    TEXT,
    is_active     BOOLEAN       NOT NULL DEFAULT true,
    first_seen_at TIMESTAMPTZ   NOT NULL DEFAULT now(),
    last_seen_at  TIMESTAMPTZ   NOT NULL DEFAULT now(),
    metadata      JSONB         NOT NULL DEFAULT '{}',
    created_at    TIMESTAMPTZ   NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ   NOT NULL DEFAULT now(),

    UNIQUE (tenant_id, target_id, port, protocol)
);

CREATE INDEX idx_ports_tenant_target ON exposed_ports (tenant_id, target_id)
    WHERE is_active = true;
CREATE INDEX idx_ports_service ON exposed_ports (tenant_id, service)
    WHERE is_active = true;

ALTER TABLE exposed_ports ENABLE ROW LEVEL SECURITY;
ALTER TABLE exposed_ports FORCE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation ON exposed_ports
    AS PERMISSIVE FOR ALL TO app_user
    USING  (tenant_id = current_setting('app.tenant_id', true)::uuid)
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true)::uuid);

-- ─── Vulnerabilities ────────────────────────────────────────────────────────

CREATE TYPE vuln_status AS ENUM (
    'open',
    'in_progress',
    'mitigated',
    'resolved',
    'accepted',
    'false_positive'
);

CREATE TYPE vuln_source AS ENUM (
    'nmap',
    'nessus',
    'openvas',
    'manual',
    'llm_analysis',
    'osint'
);

CREATE TABLE vulnerabilities (
    id               UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id        UUID         NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    target_id        UUID         NOT NULL REFERENCES scan_targets(id) ON DELETE CASCADE,
    -- Nullable: hay vulns sin puerto asociado (config débil, OSINT, etc.)
    port_id          UUID         REFERENCES exposed_ports(id) ON DELETE SET NULL,

    -- Hash determinístico SHA256(target_id || port? || cve_id? || title)
    -- Calculado en la aplicación para deduplicar re-escaneos.
    fingerprint      TEXT         NOT NULL,

    title            TEXT         NOT NULL,
    description      TEXT,
    severity         risk_level   NOT NULL,
    cvss_score       NUMERIC(4,1) CHECK (cvss_score BETWEEN 0.0 AND 10.0),
    cvss_vector      TEXT,
    cve_id           TEXT,
    cwe_id           TEXT,

    status           vuln_status  NOT NULL DEFAULT 'open',
    source           vuln_source  NOT NULL DEFAULT 'manual',

    -- Output crudo del scanner (nmap XML snippet, nessus plugin output, etc.)
    evidence         JSONB        NOT NULL DEFAULT '{}',

    -- Gestión del ciclo de vida
    remediation_note TEXT,
    accepted_by      UUID         REFERENCES users(id) ON DELETE SET NULL,
    accepted_at      TIMESTAMPTZ,

    -- Tracking temporal
    first_seen_at    TIMESTAMPTZ  NOT NULL DEFAULT now(),
    last_seen_at     TIMESTAMPTZ  NOT NULL DEFAULT now(),
    resolved_at      TIMESTAMPTZ,

    created_at       TIMESTAMPTZ  NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ  NOT NULL DEFAULT now(),

    -- Misma vulnerabilidad en el mismo target no se duplica.
    -- ON CONFLICT actualiza last_seen_at y evidence.
    UNIQUE (tenant_id, target_id, fingerprint)
);

CREATE INDEX idx_vulns_open_by_target ON vulnerabilities (tenant_id, target_id, severity)
    WHERE status = 'open';
CREATE INDEX idx_vulns_cve ON vulnerabilities (cve_id)
    WHERE cve_id IS NOT NULL;
CREATE INDEX idx_vulns_status_updated ON vulnerabilities (tenant_id, status, updated_at DESC);

ALTER TABLE vulnerabilities ENABLE ROW LEVEL SECURITY;
ALTER TABLE vulnerabilities FORCE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation ON vulnerabilities
    AS PERMISSIVE FOR ALL TO app_user
    USING  (tenant_id = current_setting('app.tenant_id', true)::uuid)
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true)::uuid);
