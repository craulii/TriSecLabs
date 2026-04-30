-- Activos/proveedores bajo análisis CTEM.
-- Un scan_target es cualquier entidad externa que se monitorea
-- (dominio, IP, proveedor, organización).

CREATE TYPE target_kind AS ENUM ('domain', 'ip_range', 'vendor', 'organization');
CREATE TYPE risk_level AS ENUM ('critical', 'high', 'medium', 'low', 'info');

CREATE TABLE scan_targets (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id   UUID        NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    kind        target_kind NOT NULL,
    name        TEXT        NOT NULL,
    value       TEXT        NOT NULL,  -- el dominio, CIDR, o nombre del vendor
    risk_score  SMALLINT,              -- 0-100, calculado por analysis worker
    risk_level  risk_level,
    metadata    JSONB       NOT NULL DEFAULT '{}',
    last_scanned_at TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (tenant_id, kind, value)
);

CREATE INDEX idx_scan_targets_tenant ON scan_targets (tenant_id);
CREATE INDEX idx_scan_targets_risk ON scan_targets (tenant_id, risk_level)
    WHERE risk_level IN ('critical', 'high');

ALTER TABLE scan_targets ENABLE ROW LEVEL SECURITY;
ALTER TABLE scan_targets FORCE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation ON scan_targets
    AS PERMISSIVE FOR ALL
    TO app_user
    USING (tenant_id = current_setting('app.tenant_id', true)::uuid);

-- Resultados individuales de cada scan
CREATE TABLE scan_findings (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID        NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    target_id       UUID        NOT NULL REFERENCES scan_targets(id) ON DELETE CASCADE,
    title           TEXT        NOT NULL,
    description     TEXT,
    severity        risk_level  NOT NULL,
    cve_id          TEXT,
    cvss_score      NUMERIC(4,1),
    raw_data        JSONB       NOT NULL DEFAULT '{}',
    found_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_findings_target ON scan_findings (tenant_id, target_id, severity);

ALTER TABLE scan_findings ENABLE ROW LEVEL SECURITY;
ALTER TABLE scan_findings FORCE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation ON scan_findings
    AS PERMISSIVE FOR ALL
    TO app_user
    USING (tenant_id = current_setting('app.tenant_id', true)::uuid);

-- Informes LLM generados sobre targets
CREATE TABLE reports (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id   UUID        NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    target_id   UUID        NOT NULL REFERENCES scan_targets(id) ON DELETE CASCADE,
    content     TEXT        NOT NULL,
    model_used  TEXT        NOT NULL,
    generated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE reports ENABLE ROW LEVEL SECURITY;
ALTER TABLE reports FORCE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation ON reports
    AS PERMISSIVE FOR ALL
    TO app_user
    USING (tenant_id = current_setting('app.tenant_id', true)::uuid);
