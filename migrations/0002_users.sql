-- Usuarios del sistema. Cada usuario pertenece a exactamente un tenant.
-- El hash de contraseña usa argon2id (gestionado en la capa de aplicación).

CREATE TYPE user_role AS ENUM ('admin', 'analyst');

CREATE TABLE users (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id     UUID        NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    email         TEXT        NOT NULL,
    password_hash TEXT        NOT NULL,
    role          user_role   NOT NULL DEFAULT 'analyst',
    active        BOOLEAN     NOT NULL DEFAULT true,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (tenant_id, email)
);

ALTER TABLE users ENABLE ROW LEVEL SECURITY;
ALTER TABLE users FORCE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation ON users
    AS PERMISSIVE FOR ALL
    TO app_user
    USING (tenant_id = current_setting('app.tenant_id', true)::uuid);
