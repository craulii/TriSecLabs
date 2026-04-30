-- Series temporales de métricas agregadas por tenant y por target.
-- Calculadas por el analysis worker, no en caliente.
-- Permite dashboards históricos sin agregar sobre tablas grandes en cada request.

CREATE TYPE metric_kind AS ENUM (
    'risk_score',              -- 0–100
    'vuln_count_critical',
    'vuln_count_high',
    'vuln_count_medium',
    'vuln_count_low',
    'exposed_port_count',
    'mean_time_to_remediate',  -- MTTR en horas
    'scan_coverage'            -- % de targets escaneados en últimos 7d (solo tenant-level)
);

CREATE TABLE metrics (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id    UUID        NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    -- NULL = métrica del tenant agregada / non-NULL = métrica de un target
    target_id    UUID        REFERENCES scan_targets(id) ON DELETE CASCADE,
    kind         metric_kind NOT NULL,
    value        NUMERIC     NOT NULL,
    period_start TIMESTAMPTZ NOT NULL,
    period_end   TIMESTAMPTZ NOT NULL,
    computed_at  TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (tenant_id, target_id, kind, period_start)
);

-- Time series por target (query más frecuente)
CREATE INDEX idx_metrics_target_kind ON metrics (tenant_id, target_id, kind, period_start DESC)
    WHERE target_id IS NOT NULL;

-- Métricas globales del tenant
CREATE INDEX idx_metrics_tenant_kind ON metrics (tenant_id, kind, period_start DESC)
    WHERE target_id IS NULL;

ALTER TABLE metrics ENABLE ROW LEVEL SECURITY;
ALTER TABLE metrics FORCE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation ON metrics
    AS PERMISSIVE FOR ALL TO app_user
    USING  (tenant_id = current_setting('app.tenant_id', true)::uuid)
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true)::uuid);
