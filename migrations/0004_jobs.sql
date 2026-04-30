-- Cola de trabajos asíncronos usando PostgreSQL.
-- Pattern: SELECT FOR UPDATE SKIP LOCKED para dequeue atómico sin Redis.

CREATE TYPE job_status AS ENUM ('pending', 'running', 'done', 'failed');
CREATE TYPE job_type   AS ENUM ('scan', 'analysis', 'llm_report');

CREATE TABLE jobs (
    id           UUID       PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id    UUID       NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    job_type     job_type   NOT NULL,
    payload      JSONB      NOT NULL,
    status       job_status NOT NULL DEFAULT 'pending',
    attempts     INT        NOT NULL DEFAULT 0,
    max_attempts INT        NOT NULL DEFAULT 3,
    error        TEXT,
    run_after    TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Índice parcial solo sobre jobs pendientes: el worker solo necesita estos
CREATE INDEX idx_jobs_pending
    ON jobs (run_after, created_at)
    WHERE status = 'pending';

-- Rate limiting por tenant (opcional, extensible)
CREATE TABLE job_rate_limits (
    tenant_id   UUID      NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    job_type    job_type  NOT NULL,
    max_per_hour INT      NOT NULL DEFAULT 10,
    PRIMARY KEY (tenant_id, job_type)
);

-- Los jobs no tienen RLS: son gestionados por el worker que usa una conexión
-- privilegiada sin tenant_id seteado. El tenant_id en el payload garantiza
-- que los workers acceden a los datos correctos via with_tenant().
