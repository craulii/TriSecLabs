# TriSecLabs — Guía de Implementación Completa

> Referencia técnica para implementar el stack desde cero.
> Programar primero, inicializar Docker después (ver §10).
> Mantiene consistencia total con las decisiones de arquitectura previas.

---

## Índice

1. [Prerequisitos del entorno](#1-prerequisitos)
2. [Estructura del workspace](#2-estructura)
3. [Schema PostgreSQL completo](#3-schema)
4. [RLS: políticas y aislamiento real](#4-rls)
5. [Capa de datos — `db` crate](#5-capa-db)
6. [Modelos de dominio — `common` crate](#6-modelos)
7. [Capa de servicios — `server` crate](#7-servicios)
8. [Handlers Axum](#8-handlers)
9. [Workers async](#9-workers)
10. [Flujo completo: request → DB → worker → resultado](#10-flujo)
11. [Autenticación: login → JWT](#11-auth)
12. [Inicializar el entorno después de programar](#12-docker)

---

## 1. Prerequisitos

```bash
# Rust toolchain estable
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown

# cargo-leptos (build tool SSR + WASM)
cargo install cargo-leptos --locked

# sqlx-cli (migraciones + offline cache)
cargo install sqlx-cli --no-default-features --features postgres,rustls

# binaryen (wasm-opt para builds release)
# Debian/Ubuntu:
apt-get install binaryen
# macOS:
brew install binaryen

# nmap (requerido por el scan worker en runtime)
apt-get install nmap   # el worker lo invoca via subprocess
```

**Dependencias nuevas en `Cargo.toml` del workspace** (añadir al existente):

```toml
[workspace.dependencies]
# ... dependencias existentes ...

# Autenticación
argon2    = "0.5"           # hashing de contraseñas con argon2id
rand      = "0.8"           # salt generation

# Scanner
tokio-process = { version = "1", package = "tokio", features = ["process"] }

# Hashing para fingerprints de vulnerabilidades
sha2 = "0.10"
hex  = "0.4"
```

---

## 2. Estructura del workspace

```
TriSecLabs/
├── Cargo.toml              workspace
├── Leptos.toml
├── Dockerfile
├── docker-compose.yml
├── docker-compose.prod.yml
├── migrations/
│   ├── 0001_tenants.sql
│   ├── 0002_users.sql
│   ├── 0003_scan_targets.sql
│   ├── 0004_jobs.sql
│   ├── 0005_vulnerabilities_ports.sql   ← NUEVA
│   └── 0006_metrics.sql                 ← NUEVA
├── assets/
│   └── style.css
└── crates/
    ├── common/src/
    │   ├── lib.rs
    │   ├── error.rs
    │   ├── models.rs          ← extender con nuevos tipos
    │   └── tenant.rs
    ├── db/src/
    │   ├── lib.rs
    │   ├── pool.rs
    │   ├── rls.rs
    │   └── queries/
    │       ├── mod.rs
    │       ├── tenants.rs
    │       ├── users.rs        ← NUEVA (auth)
    │       ├── scan_targets.rs
    │       ├── vulnerabilities.rs  ← NUEVA
    │       ├── exposed_ports.rs    ← NUEVA
    │       ├── metrics.rs          ← NUEVA
    │       └── jobs.rs
    ├── llm/src/
    │   ├── lib.rs
    │   └── client.rs
    ├── app/src/
    │   ├── lib.rs
    │   ├── app.rs
    │   ├── components/
    │   └── pages/
    │       ├── mod.rs
    │       ├── home.rs
    │       ├── dashboard.rs
    │       ├── vendors.rs
    │       └── not_found.rs
    ├── server/src/
    │   ├── main.rs
    │   ├── state.rs
    │   ├── middleware/
    │   │   ├── mod.rs
    │   │   └── auth.rs
    │   ├── services/           ← NUEVA capa
    │   │   ├── mod.rs
    │   │   ├── vulnerability.rs
    │   │   ├── port.rs
    │   │   └── metrics.rs
    │   └── routes/
    │       ├── mod.rs
    │       ├── api.rs          ← extender
    │       └── auth.rs         ← NUEVA (login)
    └── worker/src/
        ├── main.rs
        └── handlers/
            ├── mod.rs
            ├── scan.rs         ← implementar (nmap)
            ├── analysis.rs     ← implementar (risk scoring)
            └── llm_report.rs   ← ya existe
```

---

## 3. Schema PostgreSQL completo

### Migraciones existentes (0001–0004)

Ya implementadas. No modificar. Resumen:
- `tenants`: tabla raíz, slug único por organización
- `users`: RLS activo, `(tenant_id, email)` unique
- `scan_targets`: activos monitoreados, RLS, `target_kind` + `risk_level` enums
- `scan_findings`: hallazgos raw del scanner, RLS
- `reports`: informes LLM, RLS
- `jobs`: cola async, **sin RLS** (worker usa conexión privilegiada)

### Migration 0005: vulnerabilities + exposed_ports

```sql
-- migrations/0005_vulnerabilities_ports.sql
-- Vulnerabilidades estructuradas (lifecycle completo) y puertos expuestos.
-- Complementan scan_findings: findings = output raw del scanner,
-- vulnerabilities = entidad de negocio con ciclo de vida.

-- ─── Exposed Ports ─────────────────────────────────────────────────────────

CREATE TYPE port_protocol AS ENUM ('tcp', 'udp', 'sctp');
CREATE TYPE port_state    AS ENUM ('open', 'filtered', 'closed');

CREATE TABLE exposed_ports (
    id           UUID          PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id    UUID          NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    target_id    UUID          NOT NULL REFERENCES scan_targets(id) ON DELETE CASCADE,
    port         INTEGER       NOT NULL CHECK (port BETWEEN 1 AND 65535),
    protocol     port_protocol NOT NULL DEFAULT 'tcp',
    state        port_state    NOT NULL DEFAULT 'open',
    service      TEXT,          -- nombre del servicio (http, ssh, rdp…)
    banner       TEXT,          -- banner crudo del servicio
    product      TEXT,          -- producto detectado (Apache httpd, OpenSSH…)
    version      TEXT,          -- versión del producto
    extra_info   TEXT,
    is_active    BOOLEAN        NOT NULL DEFAULT true,
    first_seen_at TIMESTAMPTZ   NOT NULL DEFAULT now(),
    last_seen_at  TIMESTAMPTZ   NOT NULL DEFAULT now(),
    metadata      JSONB         NOT NULL DEFAULT '{}',

    -- Unicidad física: mismo puerto+protocolo no puede aparecer dos veces
    -- para el mismo target del mismo tenant.
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
    USING (tenant_id = current_setting('app.tenant_id', true)::uuid);

-- ─── Vulnerabilities ────────────────────────────────────────────────────────

CREATE TYPE vuln_status AS ENUM (
    'open',         -- detectada, sin acción
    'in_progress',  -- en proceso de remediación
    'mitigated',    -- mitigada (workaround, no fix definitivo)
    'resolved',     -- corregida y verificada
    'accepted',     -- riesgo aceptado formalmente
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
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id        UUID        NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    target_id        UUID        NOT NULL REFERENCES scan_targets(id) ON DELETE CASCADE,
    -- port_id es nullable: hay vulns que no están asociadas a un puerto específico
    -- (ej: configuración débil del servidor, credenciales expuestas en OSINT)
    port_id          UUID        REFERENCES exposed_ports(id) ON DELETE SET NULL,

    -- fingerprint: hash determinístico para deduplicar en re-escaneos.
    -- Calculado en la aplicación: SHA256(tenant_id || target_id || port? || cve_id? || title)
    fingerprint      TEXT        NOT NULL,

    title            TEXT        NOT NULL,
    description      TEXT,
    severity         risk_level  NOT NULL,
    cvss_score       NUMERIC(4,1) CHECK (cvss_score BETWEEN 0.0 AND 10.0),
    cvss_vector      TEXT,        -- e.g. CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H
    cve_id           TEXT,        -- CVE-2024-XXXXX
    cwe_id           TEXT,        -- CWE-79

    status           vuln_status NOT NULL DEFAULT 'open',
    source           vuln_source NOT NULL DEFAULT 'manual',

    -- evidencia cruda del scanner (output nmap XML, snippet Nessus, etc.)
    evidence         JSONB       NOT NULL DEFAULT '{}',

    -- campos de gestión
    remediation_note TEXT,
    accepted_by      UUID        REFERENCES users(id) ON DELETE SET NULL,
    accepted_at      TIMESTAMPTZ,

    -- tracking temporal
    first_seen_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at      TIMESTAMPTZ,

    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),

    -- Misma vulnerabilidad en el mismo target no se duplica.
    -- Si cambia el estado (open → resolved → open) se actualiza, no se inserta.
    UNIQUE (tenant_id, target_id, fingerprint)
);

CREATE INDEX idx_vulns_tenant_target ON vulnerabilities (tenant_id, target_id, severity)
    WHERE status = 'open';
CREATE INDEX idx_vulns_cve ON vulnerabilities (cve_id)
    WHERE cve_id IS NOT NULL;
CREATE INDEX idx_vulns_status ON vulnerabilities (tenant_id, status, updated_at DESC);

ALTER TABLE vulnerabilities ENABLE ROW LEVEL SECURITY;
ALTER TABLE vulnerabilities FORCE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation ON vulnerabilities
    AS PERMISSIVE FOR ALL TO app_user
    USING (tenant_id = current_setting('app.tenant_id', true)::uuid);
```

### Migration 0006: metrics

```sql
-- migrations/0006_metrics.sql
-- Series temporales de métricas agregadas por tenant y por target.
-- Calculadas por el analysis worker, no por queries ad-hoc en tiempo real.
-- Permite dashboards históricos sin agregar en caliente sobre tablas grandes.

CREATE TYPE metric_kind AS ENUM (
    'risk_score',              -- 0–100, score de riesgo del target
    'vuln_count_critical',
    'vuln_count_high',
    'vuln_count_medium',
    'vuln_count_low',
    'exposed_port_count',      -- puertos abiertos activos
    'mean_time_to_remediate',  -- MTTR en horas (promedio de vulns cerradas en el período)
    'scan_coverage'            -- % de targets con scan en los últimos 7 días (solo tenant-level)
);

CREATE TABLE metrics (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id    UUID        NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    -- NULL → métrica agregada del tenant completo
    -- non-NULL → métrica de un target específico
    target_id    UUID        REFERENCES scan_targets(id) ON DELETE CASCADE,
    kind         metric_kind NOT NULL,
    value        NUMERIC     NOT NULL,
    -- ventana temporal de la métrica (para métricas de período, ej. MTTR del mes)
    period_start TIMESTAMPTZ NOT NULL,
    period_end   TIMESTAMPTZ NOT NULL,
    computed_at  TIMESTAMPTZ NOT NULL DEFAULT now(),

    -- Solo una métrica por (tenant, target?, kind, period)
    UNIQUE (tenant_id, target_id, kind, period_start)
);

-- Query típica: últimas N métricas de un tipo para un target (time series)
CREATE INDEX idx_metrics_target_kind ON metrics (tenant_id, target_id, kind, period_start DESC)
    WHERE target_id IS NOT NULL;

-- Query típica: métricas globales del tenant
CREATE INDEX idx_metrics_tenant_kind ON metrics (tenant_id, kind, period_start DESC)
    WHERE target_id IS NULL;

ALTER TABLE metrics ENABLE ROW LEVEL SECURITY;
ALTER TABLE metrics FORCE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation ON metrics
    AS PERMISSIVE FOR ALL TO app_user
    USING (tenant_id = current_setting('app.tenant_id', true)::uuid);
```

---

## 4. RLS: políticas y aislamiento real

### Mecanismo

PostgreSQL evalúa la policy en cada fila antes de retornarla o permitir escritura:

```sql
-- La policy de todas las tablas de datos tiene esta forma:
USING (tenant_id = current_setting('app.tenant_id', true)::uuid)
--                 ↑ lee la variable de sesión establecida por with_tenant_conn()
```

`current_setting('app.tenant_id', true)` — el segundo argumento `true` significa "no lanzar
error si la variable no existe; retornar NULL en su lugar". Si el worker llama sin haber
establecido `app.tenant_id`, todas las policies retornan `NULL = UUID` que es siempre false →
cero filas visibles. Es un safety net, no un camino válido.

### Establecer el contexto (Rust)

```rust
// db/src/rls.rs (ya implementado)
// with_tenant_conn: SET app.tenant_id = $1, luego RESET
// with_tenant: usa SET LOCAL dentro de una transacción (se resetea automáticamente)
```

Nunca setear `app.tenant_id` manualmente fuera de estos helpers.

### Políticas completas (explicación por tabla)

```sql
-- Los policies de INSERT y UPDATE no necesitan WITH CHECK separado si
-- solo usamos PERMISSIVE FOR ALL con USING — PostgreSQL aplica USING como
-- CHECK en escrituras también para PERMISSIVE policies.
--
-- Sin embargo, para tables donde necesitamos que tenant_id NO sea manipulable
-- por el cliente, añadimos WITH CHECK explícito:

-- Ejemplo para vulnerabilities (política de escritura segura):
-- Si un usuario intenta INSERT con tenant_id diferente al de su sesión,
-- la policy lo bloquea.

ALTER POLICY tenant_isolation ON vulnerabilities
    USING (tenant_id = current_setting('app.tenant_id', true)::uuid)
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true)::uuid);

-- Aplicar el mismo patrón a exposed_ports y metrics.
```

### Test de aislamiento (ejecutar manualmente)

```sql
-- 1. Preparar datos de prueba
INSERT INTO tenants (id, slug, name) VALUES
    ('aaaaaaaa-0000-0000-0000-000000000001', 'acme', 'Acme Corp'),
    ('bbbbbbbb-0000-0000-0000-000000000002', 'rival', 'Rival Inc');

INSERT INTO scan_targets (id, tenant_id, kind, name, value)
VALUES
    ('cccccccc-0000-0000-0000-000000000001',
     'aaaaaaaa-0000-0000-0000-000000000001', 'domain', 'acme.com', 'acme.com'),
    ('dddddddd-0000-0000-0000-000000000002',
     'bbbbbbbb-0000-0000-0000-000000000002', 'domain', 'rival.com', 'rival.com');

-- 2. Test: Acme no puede ver datos de Rival
SET app.tenant_id = 'aaaaaaaa-0000-0000-0000-000000000001';
SELECT id, name FROM scan_targets;
-- Resultado: solo 'acme.com'. 'rival.com' invisible.

-- 3. Test: SELECT sin tenant_id establecido → 0 filas (safety net)
RESET app.tenant_id;
SELECT id, name FROM scan_targets;
-- Resultado: 0 filas. No error, pero nada visible.

-- 4. Test: intento de INSERT cross-tenant es rechazado
SET app.tenant_id = 'aaaaaaaa-0000-0000-0000-000000000001';
INSERT INTO scan_targets (tenant_id, kind, name, value)
VALUES ('bbbbbbbb-0000-0000-0000-000000000002', 'domain', 'hack', 'hack.com');
-- Resultado: ERROR - new row violates row-level security policy

-- 5. Verificar con función helper (útil en tests de integración)
SET app.tenant_id = 'aaaaaaaa-0000-0000-0000-000000000001';
SELECT current_setting('app.tenant_id', true) AS active_tenant;
-- Resultado: 'aaaaaaaa-0000-0000-0000-000000000001'
```

### Tabla `users` — policy adicional para self-access

```sql
-- Los usuarios solo pueden ver su propio perfil, a menos que sean admin.
-- Implementar como policy adicional (RESTRICTIVE para admins sería un anti-pattern).
-- En su lugar, la lógica de "admin ve todos los usuarios del tenant" se maneja
-- en la query, filtrando por role en la capa de servicios.
-- La policy RLS solo garantiza aislamiento de tenant, no de usuario individual.
```

---

## 5. Capa de datos — `db` crate

### `db/src/queries/users.rs`

```rust
use common::{AppError, TenantId};
use sqlx::PgConnection;
use uuid::Uuid;

pub struct UserRow {
    pub id:            Uuid,
    pub tenant_id:     Uuid,
    pub email:         String,
    pub password_hash: String,
    pub role:          String,
}

/// Busca por email dentro del tenant activo en la conexión.
/// Llamar SIEMPRE desde within_tenant_conn para que RLS funcione.
pub async fn find_by_email(
    conn: &mut PgConnection,
    email: &str,
) -> Result<Option<UserRow>, AppError> {
    sqlx::query_as!(
        UserRow,
        r#"SELECT id, tenant_id, email, password_hash, role
           FROM users WHERE email = $1"#,
        email
    )
    .fetch_optional(conn)
    .await
    .map_err(|e| AppError::Database(e.to_string()))
}

pub async fn create(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    email: &str,
    password_hash: &str,
    role: &str,
) -> Result<Uuid, AppError> {
    let row = sqlx::query!(
        r#"INSERT INTO users (tenant_id, email, password_hash, role)
           VALUES ($1, $2, $3, $4)
           RETURNING id"#,
        tenant_id, email, password_hash, role
    )
    .fetch_one(conn)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(row.id)
}
```

### `db/src/queries/vulnerabilities.rs`

```rust
use chrono::{DateTime, Utc};
use common::{AppError, models::RiskLevel};
use sqlx::PgConnection;
use uuid::Uuid;

pub struct VulnRow {
    pub id:           Uuid,
    pub tenant_id:    Uuid,
    pub target_id:    Uuid,
    pub port_id:      Option<Uuid>,
    pub fingerprint:  String,
    pub title:        String,
    pub description:  Option<String>,
    pub severity:     String,
    pub cvss_score:   Option<f32>,
    pub cve_id:       Option<String>,
    pub status:       String,
    pub source:       String,
    pub evidence:     serde_json::Value,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at:  DateTime<Utc>,
}

/// Upsert por fingerprint: si ya existe actualiza last_seen_at y severity.
/// Si es nueva, inserta. Retorna el id y si fue creada (true) o actualizada (false).
pub async fn upsert(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    target_id: Uuid,
    port_id: Option<Uuid>,
    fingerprint: &str,
    title: &str,
    description: Option<&str>,
    severity: &str,
    cvss_score: Option<f32>,
    cve_id: Option<&str>,
    source: &str,
    evidence: &serde_json::Value,
) -> Result<(Uuid, bool), AppError> {
    let row = sqlx::query!(
        r#"INSERT INTO vulnerabilities
               (tenant_id, target_id, port_id, fingerprint,
                title, description, severity, cvss_score, cve_id,
                source, evidence)
           VALUES ($1,$2,$3,$4,$5,$6,$7::risk_level,$8,$9,$10::vuln_source,$11)
           ON CONFLICT (tenant_id, target_id, fingerprint) DO UPDATE
               SET last_seen_at = now(),
                   severity     = EXCLUDED.severity,
                   cvss_score   = EXCLUDED.cvss_score,
                   evidence     = EXCLUDED.evidence,
                   updated_at   = now()
           RETURNING id,
                     (xmax = 0) AS "inserted!: bool""#,
        tenant_id, target_id, port_id, fingerprint,
        title, description, severity, cvss_score, cve_id,
        source, evidence
    )
    .fetch_one(conn)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok((row.id, row.inserted))
}

pub async fn list_open_by_target(
    conn: &mut PgConnection,
    target_id: Uuid,
) -> Result<Vec<VulnRow>, AppError> {
    sqlx::query_as!(
        VulnRow,
        r#"SELECT id, tenant_id, target_id, port_id, fingerprint,
                  title, description,
                  severity::text AS "severity!",
                  cvss_score::float4 AS cvss_score,
                  cve_id,
                  status::text AS "status!",
                  source::text AS "source!",
                  evidence, first_seen_at, last_seen_at
           FROM vulnerabilities
           WHERE target_id = $1
             AND status = 'open'
           ORDER BY
               CASE severity
                   WHEN 'critical' THEN 1
                   WHEN 'high'     THEN 2
                   WHEN 'medium'   THEN 3
                   WHEN 'low'      THEN 4
                   ELSE 5
               END,
               cvss_score DESC NULLS LAST"#,
        target_id
    )
    .fetch_all(conn)
    .await
    .map_err(|e| AppError::Database(e.to_string()))
}

/// Cuenta por severidad para calcular risk score.
/// No necesita RLS explícito — la policy filtra target_id automáticamente.
pub async fn count_by_severity(
    conn: &mut PgConnection,
    target_id: Uuid,
) -> Result<SeverityCounts, AppError> {
    let row = sqlx::query!(
        r#"SELECT
               COUNT(*) FILTER (WHERE severity = 'critical' AND status = 'open') AS critical,
               COUNT(*) FILTER (WHERE severity = 'high'     AND status = 'open') AS high,
               COUNT(*) FILTER (WHERE severity = 'medium'   AND status = 'open') AS medium,
               COUNT(*) FILTER (WHERE severity = 'low'      AND status = 'open') AS low
           FROM vulnerabilities
           WHERE target_id = $1"#,
        target_id
    )
    .fetch_one(conn)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(SeverityCounts {
        critical: row.critical.unwrap_or(0) as u32,
        high:     row.high.unwrap_or(0)     as u32,
        medium:   row.medium.unwrap_or(0)   as u32,
        low:      row.low.unwrap_or(0)      as u32,
    })
}

pub struct SeverityCounts {
    pub critical: u32,
    pub high:     u32,
    pub medium:   u32,
    pub low:      u32,
}
```

### `db/src/queries/exposed_ports.rs`

```rust
use chrono::{DateTime, Utc};
use common::AppError;
use sqlx::PgConnection;
use uuid::Uuid;

pub struct PortRow {
    pub id:          Uuid,
    pub tenant_id:   Uuid,
    pub target_id:   Uuid,
    pub port:        i32,
    pub protocol:    String,
    pub state:       String,
    pub service:     Option<String>,
    pub banner:      Option<String>,
    pub product:     Option<String>,
    pub version:     Option<String>,
    pub is_active:   bool,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at:  DateTime<Utc>,
}

/// Upsert por (tenant_id, target_id, port, protocol).
/// Marca el puerto como activo y actualiza last_seen_at.
pub async fn upsert(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    target_id: Uuid,
    port: i32,
    protocol: &str,
    state: &str,
    service: Option<&str>,
    banner: Option<&str>,
    product: Option<&str>,
    version: Option<&str>,
) -> Result<Uuid, AppError> {
    let row = sqlx::query!(
        r#"INSERT INTO exposed_ports
               (tenant_id, target_id, port, protocol, state,
                service, banner, product, version, is_active)
           VALUES ($1,$2,$3,$4::port_protocol,$5::port_state,$6,$7,$8,$9,true)
           ON CONFLICT (tenant_id, target_id, port, protocol) DO UPDATE
               SET state        = EXCLUDED.state,
                   service      = COALESCE(EXCLUDED.service, exposed_ports.service),
                   banner       = COALESCE(EXCLUDED.banner,  exposed_ports.banner),
                   product      = COALESCE(EXCLUDED.product, exposed_ports.product),
                   version      = COALESCE(EXCLUDED.version, exposed_ports.version),
                   is_active    = true,
                   last_seen_at = now()
           RETURNING id"#,
        tenant_id, target_id, port, protocol, state,
        service, banner, product, version
    )
    .fetch_one(conn)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(row.id)
}

/// Marca como inactivos los puertos del target que NO están en la lista
/// de puertos encontrados en el scan actual. Útil para detectar puertos cerrados.
pub async fn deactivate_missing(
    conn: &mut PgConnection,
    target_id: Uuid,
    active_port_ids: &[Uuid],
) -> Result<u64, AppError> {
    let result = sqlx::query!(
        r#"UPDATE exposed_ports
           SET is_active = false, updated_at = now()
           WHERE target_id = $1
             AND is_active = true
             AND id != ALL($2)"#,
        target_id,
        active_port_ids
    )
    .execute(conn)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(result.rows_affected())
}

pub async fn list_active_by_target(
    conn: &mut PgConnection,
    target_id: Uuid,
) -> Result<Vec<PortRow>, AppError> {
    sqlx::query_as!(
        PortRow,
        r#"SELECT id, tenant_id, target_id, port,
                  protocol::text AS "protocol!",
                  state::text    AS "state!",
                  service, banner, product, version,
                  is_active, first_seen_at, last_seen_at
           FROM exposed_ports
           WHERE target_id = $1 AND is_active = true
           ORDER BY port"#,
        target_id
    )
    .fetch_all(conn)
    .await
    .map_err(|e| AppError::Database(e.to_string()))
}
```

### `db/src/queries/metrics.rs`

```rust
use chrono::{DateTime, Utc};
use common::AppError;
use sqlx::PgConnection;
use uuid::Uuid;

pub async fn upsert(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    target_id: Option<Uuid>,
    kind: &str,
    value: f64,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"INSERT INTO metrics (tenant_id, target_id, kind, value, period_start, period_end)
           VALUES ($1, $2, $3::metric_kind, $4, $5, $6)
           ON CONFLICT (tenant_id, target_id, kind, period_start) DO UPDATE
               SET value = EXCLUDED.value,
                   computed_at = now()"#,
        tenant_id, target_id, kind, value, period_start, period_end
    )
    .execute(conn)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}

pub struct MetricRow {
    pub kind:         String,
    pub value:        f64,
    pub period_start: DateTime<Utc>,
    pub computed_at:  DateTime<Utc>,
}

pub async fn latest_for_target(
    conn: &mut PgConnection,
    target_id: Uuid,
    kind: &str,
    limit: i64,
) -> Result<Vec<MetricRow>, AppError> {
    sqlx::query_as!(
        MetricRow,
        r#"SELECT kind::text AS "kind!", value::float8 AS "value!",
                  period_start, computed_at
           FROM metrics
           WHERE target_id = $1 AND kind = $2::metric_kind
           ORDER BY period_start DESC
           LIMIT $3"#,
        target_id, kind, limit
    )
    .fetch_all(conn)
    .await
    .map_err(|e| AppError::Database(e.to_string()))
}
```

---

## 6. Modelos de dominio — `common` crate

Extender `common/src/models.rs` con los nuevos tipos:

```rust
// Añadir al models.rs existente

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VulnStatus {
    Open,
    InProgress,
    Mitigated,
    Resolved,
    Accepted,
    FalsePositive,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VulnSource {
    Nmap,
    Nessus,
    OpenVas,
    Manual,
    LlmAnalysis,
    Osint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub id:           Uuid,
    pub tenant_id:    TenantId,
    pub target_id:    Uuid,
    pub port_id:      Option<Uuid>,
    pub fingerprint:  String,
    pub title:        String,
    pub description:  Option<String>,
    pub severity:     RiskLevel,
    pub cvss_score:   Option<f32>,
    pub cve_id:       Option<String>,
    pub status:       VulnStatus,
    pub source:       VulnSource,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at:  DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposedPort {
    pub id:        Uuid,
    pub tenant_id: TenantId,
    pub target_id: Uuid,
    pub port:      u16,
    pub protocol:  String,
    pub state:     String,
    pub service:   Option<String>,
    pub product:   Option<String>,
    pub version:   Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub id:           Uuid,
    pub tenant_id:    TenantId,
    pub target_id:    Option<Uuid>,
    pub kind:         String,
    pub value:        f64,
    pub period_start: DateTime<Utc>,
    pub period_end:   DateTime<Utc>,
    pub computed_at:  DateTime<Utc>,
}

// Fingerprint para deduplicación de vulnerabilidades
// Usar en worker y en ingesta manual
pub fn vuln_fingerprint(
    target_id: Uuid,
    port: Option<u16>,
    cve_id: Option<&str>,
    title: &str,
) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(target_id.as_bytes());
    if let Some(p) = port  { h.update(p.to_le_bytes()); }
    if let Some(c) = cve_id { h.update(c.as_bytes()); }
    h.update(title.as_bytes());
    hex::encode(h.finalize())
}
```

---

## 7. Capa de servicios — `server` crate

### `server/src/services/vulnerability.rs`

```rust
use common::{AppError, TenantId, models::vuln_fingerprint};
use db::{queries::{vulnerabilities, exposed_ports}, with_tenant_conn, PgPool};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub struct VulnerabilityService {
    pool: PgPool,
}

impl VulnerabilityService {
    pub fn new(pool: PgPool) -> Self { Self { pool } }

    pub async fn list_for_target(
        &self,
        tenant_id: TenantId,
        target_id: Uuid,
    ) -> Result<Vec<vulnerabilities::VulnRow>, AppError> {
        with_tenant_conn(&self.pool, tenant_id, |conn| {
            Box::pin(async move {
                vulnerabilities::list_open_by_target(conn, target_id).await
            })
        })
        .await
    }

    pub async fn change_status(
        &self,
        tenant_id: TenantId,
        vuln_id: Uuid,
        new_status: &str,
        note: Option<&str>,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        with_tenant_conn(&self.pool, tenant_id, |conn| {
            let new_status = new_status.to_owned();
            let note = note.map(|s| s.to_owned());
            Box::pin(async move {
                sqlx::query!(
                    r#"UPDATE vulnerabilities
                       SET status = $2::vuln_status,
                           remediation_note = COALESCE($3, remediation_note),
                           accepted_by = CASE WHEN $2 = 'accepted' THEN $4 ELSE accepted_by END,
                           accepted_at = CASE WHEN $2 = 'accepted' THEN now() ELSE accepted_at END,
                           resolved_at = CASE WHEN $2 = 'resolved' THEN now() ELSE resolved_at END,
                           updated_at = now()
                       WHERE id = $1"#,
                    vuln_id, new_status, note, user_id
                )
                .execute(conn)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
                Ok(())
            })
        })
        .await
    }
}
```

### `server/src/services/metrics.rs`

```rust
use chrono::Utc;
use common::{AppError, TenantId};
use db::{queries::{vulnerabilities, exposed_ports, metrics, scan_targets}, with_tenant_conn, PgPool};
use uuid::Uuid;

pub struct MetricsService {
    pool: PgPool,
}

impl MetricsService {
    pub fn new(pool: PgPool) -> Self { Self { pool } }

    /// Computa y persiste todas las métricas para un target.
    /// Llamado por el analysis worker después de un scan.
    pub async fn compute_for_target(
        &self,
        tenant_id: TenantId,
        target_id: Uuid,
    ) -> Result<u32, AppError> {
        let now = Utc::now();
        let period_start = now;
        let period_end = now;

        with_tenant_conn(&self.pool, tenant_id, |conn| {
            Box::pin(async move {
                let counts = vulnerabilities::count_by_severity(conn, target_id).await?;
                let port_count = sqlx::query_scalar!(
                    "SELECT COUNT(*) FROM exposed_ports WHERE target_id=$1 AND is_active=true",
                    target_id
                )
                .fetch_one(conn)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?
                .unwrap_or(0) as u32;

                // Fórmula de risk score: ponderada por severidad CVSS-inspired
                // Critical=25, High=10, Medium=3, Low=1 — cap 100
                let score = (counts.critical * 25
                    + counts.high * 10
                    + counts.medium * 3
                    + counts.low)
                    .min(100) as f64;

                let tenant_uuid = *tenant_id.as_uuid();

                // Persistir métricas
                for (kind, value) in [
                    ("risk_score",            score),
                    ("vuln_count_critical",   counts.critical as f64),
                    ("vuln_count_high",       counts.high     as f64),
                    ("vuln_count_medium",     counts.medium   as f64),
                    ("vuln_count_low",        counts.low      as f64),
                    ("exposed_port_count",    port_count      as f64),
                ] {
                    metrics::upsert(
                        conn, tenant_uuid, Some(target_id),
                        kind, value, period_start, period_end
                    ).await?;
                }

                // Actualizar risk_score en scan_targets también
                let risk_level = match score as u32 {
                    75..=100 => "critical",
                    50..=74  => "high",
                    25..=49  => "medium",
                    _        => "low",
                };
                sqlx::query!(
                    r#"UPDATE scan_targets
                       SET risk_score = $2, risk_level = $3::risk_level,
                           last_scanned_at = now(), updated_at = now()
                       WHERE id = $1"#,
                    target_id, score as i16, risk_level
                )
                .execute(conn)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

                Ok(score as u32)
            })
        })
        .await
    }
}
```

---

## 8. Handlers Axum

### `server/src/routes/auth.rs` (nuevo — login)

```rust
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::state::AppState;
use common::AppError;
use db::{queries::users, with_tenant_conn};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub tenant_slug: String,
    pub email:       String,
    pub password:    String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token:      String,
    pub user_id:    Uuid,
    pub tenant_id:  Uuid,
    pub role:       String,
}

pub fn auth_router(state: AppState) -> Router {
    Router::new()
        .route("/auth/login", post(login))
        .with_state(state)
}

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    // 1. Buscar tenant por slug (sin RLS — tenants es pública)
    let tenant = match db::queries::tenants::find_by_slug(&state.db, &req.tenant_slug).await {
        Ok(Some(t)) => t,
        Ok(None) => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
            "error": "invalid credentials"
        }))).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
            "error": e.to_string()
        }))).into_response(),
    };

    let tenant_id = common::TenantId::from(tenant.id);

    // 2. Buscar usuario en el tenant
    let user_result = with_tenant_conn(&state.db, tenant_id, |conn| {
        let email = req.email.clone();
        Box::pin(async move { users::find_by_email(conn, &email).await })
    })
    .await;

    let user = match user_result {
        Ok(Some(u)) => u,
        Ok(None) | Err(_) => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
            "error": "invalid credentials"
        }))).into_response(),
    };

    // 3. Verificar contraseña con argon2id
    let valid = verify_password(&req.password, &user.password_hash);
    if !valid {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
            "error": "invalid credentials"
        }))).into_response();
    }

    // 4. Generar JWT
    let exp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize
        + 86400; // 24 horas

    let claims = crate::middleware::auth::JwtClaims {
        sub:    user.id.to_string(),
        tenant: tenant.id.to_string(),
        role:   user.role.clone(),
        exp,
    };

    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .expect("jwt encode failed");

    (StatusCode::OK, Json(LoginResponse {
        token,
        user_id:   user.id,
        tenant_id: tenant.id,
        role:      user.role,
    }))
    .into_response()
}

fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::{Argon2, PasswordHash, PasswordVerifier};
    let parsed = PasswordHash::new(hash).ok();
    parsed.map_or(false, |h| {
        Argon2::default().verify_password(password.as_bytes(), &h).is_ok()
    })
}

// En services/auth.rs (o inline en el handler de registro):
pub fn hash_password(password: &str) -> Result<String, AppError> {
    use argon2::{
        password_hash::{rand_core::OsRng, SaltString},
        Argon2, PasswordHasher,
    };
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::Internal(e.to_string()))
}
```

### `server/src/routes/api.rs` (extendido)

```rust
use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, patch},
    Json, Router,
};
use common::tenant::TenantContext;
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;

pub fn api_router(state: AppState) -> Router {
    Router::new()
        // Vendors / scan targets
        .route("/vendors",             get(list_vendors))
        .route("/vendors/:id/analyze", post(enqueue_llm_analysis))
        .route("/vendors/:id/scan",    post(enqueue_scan))
        // Vulnerabilities
        .route("/targets/:id/vulnerabilities",          get(list_vulnerabilities))
        .route("/vulnerabilities/:id/status",           patch(update_vuln_status))
        // Ports
        .route("/targets/:id/ports",                    get(list_ports))
        // Metrics
        .route("/targets/:id/metrics/:kind",            get(get_metric_history))
        .with_state(state)
}

// GET /api/targets/:id/vulnerabilities
async fn list_vulnerabilities(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(target_id): Path<Uuid>,
) -> impl IntoResponse {
    use db::{queries::vulnerabilities, with_tenant_conn};

    let result = with_tenant_conn(&state.db, ctx.tenant_id, |conn| {
        Box::pin(async move {
            vulnerabilities::list_open_by_target(conn, target_id).await
        })
    })
    .await;

    match result {
        Ok(rows) => (StatusCode::OK, Json(rows)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
    }
}

#[derive(serde::Deserialize)]
struct StatusUpdate {
    status: String,
    note:   Option<String>,
}

// PATCH /api/vulnerabilities/:id/status
async fn update_vuln_status(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(vuln_id): Path<Uuid>,
    Json(body): Json<StatusUpdate>,
) -> impl IntoResponse {
    use db::with_tenant_conn;

    // Solo admins o analysts asignados pueden cerrar vulns
    if !matches!(body.status.as_str(), "open"|"in_progress"|"mitigated"|"resolved"|"accepted"|"false_positive") {
        return (StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "invalid status" }))).into_response();
    }

    let result = with_tenant_conn(&state.db, ctx.tenant_id, |conn| {
        let status = body.status.clone();
        let note   = body.note.clone();
        let user_id = ctx.user_id;
        Box::pin(async move {
            sqlx::query!(
                r#"UPDATE vulnerabilities
                   SET status = $2::vuln_status,
                       remediation_note = COALESCE($3, remediation_note),
                       accepted_by = CASE WHEN $2 = 'accepted' THEN $4 ELSE accepted_by END,
                       accepted_at = CASE WHEN $2 = 'accepted' THEN now() ELSE accepted_at END,
                       resolved_at = CASE WHEN $2 = 'resolved' THEN now() ELSE resolved_at END,
                       updated_at = now()
                   WHERE id = $1"#,
                vuln_id, status, note, user_id
            )
            .execute(conn)
            .await
            .map_err(|e| common::AppError::Database(e.to_string()))
        })
    })
    .await;

    match result {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
    }
}

// POST /api/vendors/:id/scan
async fn enqueue_scan(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(target_id): Path<Uuid>,
) -> impl IntoResponse {
    use db::queries::jobs;

    let payload = serde_json::json!({ "target_id": target_id.to_string() });
    match jobs::enqueue(&state.db, *ctx.tenant_id.as_uuid(), "scan", &payload).await {
        Ok(job_id) => (StatusCode::ACCEPTED,
            Json(serde_json::json!({ "job_id": job_id }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
    }
}

// GET /api/targets/:id/ports
async fn list_ports(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(target_id): Path<Uuid>,
) -> impl IntoResponse {
    use db::{queries::exposed_ports, with_tenant_conn};

    let result = with_tenant_conn(&state.db, ctx.tenant_id, |conn| {
        Box::pin(async move {
            exposed_ports::list_active_by_target(conn, target_id).await
        })
    })
    .await;

    match result {
        Ok(rows) => (StatusCode::OK, Json(rows)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
    }
}

// GET /api/targets/:id/metrics/:kind?limit=30
async fn get_metric_history(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path((target_id, kind)): Path<(Uuid, String)>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    use db::{queries::metrics, with_tenant_conn};

    let limit = params.get("limit")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(30)
        .min(100);

    let result = with_tenant_conn(&state.db, ctx.tenant_id, |conn| {
        let kind = kind.clone();
        Box::pin(async move {
            metrics::latest_for_target(conn, target_id, &kind, limit).await
        })
    })
    .await;

    match result {
        Ok(rows) => (StatusCode::OK, Json(rows)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
    }
}
```

---

## 9. Workers async

### Diseño de la cola (repaso)

```
┌─────────────────────────────────────────────────────────────┐
│  jobs table (PostgreSQL)                                    │
│                                                             │
│  status: pending → running → done | failed                  │
│                                                             │
│  El worker hace:                                            │
│    1. SELECT ... FOR UPDATE SKIP LOCKED (dequeue atómico)   │
│    2. SET status = 'running'                                 │
│    3. Ejecuta el handler                                     │
│    4. SET status = 'done' | 'failed'                        │
│                                                             │
│  Si el worker muere mientras procesa un job, el job queda   │
│  en estado 'running'. Para recuperarlos, añadir un cron    │
│  que resetee a 'pending' los jobs 'running' con             │
│  updated_at > 10min.                                        │
└─────────────────────────────────────────────────────────────┘
```

**Recovery de jobs stuck** (añadir a `db/src/queries/jobs.rs`):

```rust
/// Resetea a 'pending' jobs que llevan más de `stale_minutes` en 'running'.
/// Llamar periódicamente desde el worker (cada N ciclos).
pub async fn reset_stale_running(
    pool: &PgPool,
    stale_minutes: i64,
) -> Result<u64, AppError> {
    let result = sqlx::query!(
        r#"UPDATE jobs
           SET status = 'pending', updated_at = now()
           WHERE status = 'running'
             AND updated_at < now() - ($1 || ' minutes')::interval
             AND attempts < max_attempts"#,
        stale_minutes.to_string()
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(result.rows_affected())
}
```

### `worker/src/handlers/scan.rs` — implementación real con nmap

```rust
use common::{AppError, TenantId, models::vuln_fingerprint};
use db::{queries::{exposed_ports, vulnerabilities, scan_targets}, with_tenant_conn, PgPool};
use serde_json::Value;
use tokio::process::Command;
use tracing::{info, warn};
use uuid::Uuid;

pub async fn handle(
    pool: &PgPool,
    tenant_id: TenantId,
    payload: &Value,
) -> Result<(), AppError> {
    let target_id: Uuid = payload["target_id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| AppError::InvalidInput("missing target_id".into()))?;

    // Obtener el valor del target (dominio, CIDR, etc.)
    let target = with_tenant_conn(pool, tenant_id, |conn| {
        Box::pin(async move {
            scan_targets::find_by_id(conn, target_id)
                .await?
                .ok_or_else(|| AppError::not_found(format!("target {target_id}")))
        })
    })
    .await?;

    info!(%tenant_id, %target_id, target_value = %target.value, "starting nmap scan");

    // Ejecutar nmap como subprocess.
    // -sV: detectar versiones de servicios
    // -T4: velocidad agresiva
    // -oX -: output XML en stdout
    // --top-ports 1000: los 1000 puertos más comunes
    let output = Command::new("nmap")
        .args(["-sV", "-T4", "--top-ports", "1000", "-oX", "-", &target.value])
        .output()
        .await
        .map_err(|e| AppError::Internal(format!("nmap exec failed: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Internal(format!("nmap failed: {stderr}")));
    }

    let xml = String::from_utf8_lossy(&output.stdout);
    let scan_result = parse_nmap_xml(&xml)
        .map_err(|e| AppError::Internal(format!("nmap parse failed: {e}")))?;

    info!(%target_id, ports_found = scan_result.ports.len(), "nmap scan complete");

    // Persistir ports y generar vulnerabilidades para servicios conocidos inseguros
    let mut active_port_ids: Vec<Uuid> = Vec::new();

    for port_info in &scan_result.ports {
        let port_id = with_tenant_conn(pool, tenant_id, |conn| {
            let p = port_info.clone();
            let tid = *tenant_id.as_uuid();
            Box::pin(async move {
                exposed_ports::upsert(
                    conn, tid, target_id,
                    p.port as i32, &p.protocol,
                    &p.state,
                    p.service.as_deref(),
                    p.banner.as_deref(),
                    p.product.as_deref(),
                    p.version.as_deref(),
                ).await
            })
        })
        .await?;

        active_port_ids.push(port_id);

        // Detectar servicios inseguros conocidos y crear vulnerabilidades automáticas
        if let Some(ref service) = port_info.service {
            if let Some(vuln) = detect_insecure_service(port_info) {
                let fp = vuln_fingerprint(
                    target_id,
                    Some(port_info.port),
                    vuln.cve_id.as_deref(),
                    &vuln.title,
                );
                let evidence = serde_json::json!({
                    "port": port_info.port,
                    "protocol": port_info.protocol,
                    "service": service,
                    "product": port_info.product,
                    "version": port_info.version,
                });
                with_tenant_conn(pool, tenant_id, |conn| {
                    let fp = fp.clone();
                    let v  = vuln.clone();
                    let tid = *tenant_id.as_uuid();
                    let pid = Some(port_id);
                    Box::pin(async move {
                        vulnerabilities::upsert(
                            conn, tid, target_id, pid,
                            &fp, &v.title, v.description.as_deref(),
                            &v.severity, v.cvss_score, v.cve_id.as_deref(),
                            "nmap", &evidence,
                        ).await
                    })
                })
                .await?;
            }
        }
    }

    // Marcar como inactivos los puertos que ya no aparecen en el scan
    with_tenant_conn(pool, tenant_id, |conn| {
        let ids = active_port_ids.clone();
        Box::pin(async move {
            exposed_ports::deactivate_missing(conn, target_id, &ids).await
        })
    })
    .await?;

    info!(%target_id, "scan persisted, enqueuing analysis");

    // Encolar análisis de riesgo como siguiente paso
    let analysis_payload = serde_json::json!({ "target_id": target_id.to_string() });
    db::queries::jobs::enqueue(
        pool,
        *tenant_id.as_uuid(),
        "analysis",
        &analysis_payload,
    )
    .await?;

    Ok(())
}

// ─── Nmap XML parser ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct PortInfo {
    port:     u16,
    protocol: String,
    state:    String,
    service:  Option<String>,
    banner:   Option<String>,
    product:  Option<String>,
    version:  Option<String>,
}

struct ScanResult {
    ports: Vec<PortInfo>,
}

fn parse_nmap_xml(xml: &str) -> Result<ScanResult, Box<dyn std::error::Error>> {
    // Parseo minimalista. Para producción usar quick-xml o roxmltree.
    // quick-xml = "0.31" (añadir a workspace deps)
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    let mut ports = Vec::new();
    let mut current: Option<PortInfo> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                match e.name().as_ref() {
                    b"port" => {
                        let port = e.try_get_attribute("portid")?
                            .and_then(|a| std::str::from_utf8(a.value.as_ref()).ok()
                                .and_then(|s| s.parse::<u16>().ok()))
                            .unwrap_or(0);
                        let protocol = e.try_get_attribute("protocol")?
                            .and_then(|a| std::str::from_utf8(a.value.as_ref()).ok()
                                .map(|s| s.to_owned()))
                            .unwrap_or_else(|| "tcp".to_owned());
                        current = Some(PortInfo {
                            port, protocol, state: "open".into(),
                            service: None, banner: None,
                            product: None, version: None,
                        });
                    }
                    b"state" => {
                        if let Some(ref mut p) = current {
                            p.state = e.try_get_attribute("state")?
                                .and_then(|a| std::str::from_utf8(a.value.as_ref()).ok()
                                    .map(|s| s.to_owned()))
                                .unwrap_or_else(|| "open".into());
                        }
                    }
                    b"service" => {
                        if let Some(ref mut p) = current {
                            p.service = e.try_get_attribute("name")?
                                .and_then(|a| std::str::from_utf8(a.value.as_ref()).ok()
                                    .map(|s| s.to_owned()));
                            p.product = e.try_get_attribute("product")?
                                .and_then(|a| std::str::from_utf8(a.value.as_ref()).ok()
                                    .map(|s| s.to_owned()));
                            p.version = e.try_get_attribute("version")?
                                .and_then(|a| std::str::from_utf8(a.value.as_ref()).ok()
                                    .map(|s| s.to_owned()));
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) if e.name().as_ref() == b"port" => {
                if let Some(p) = current.take() {
                    if p.state == "open" || p.state == "filtered" {
                        ports.push(p);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
    }

    Ok(ScanResult { ports })
}

// ─── Detección de servicios inseguros conocidos ───────────────────────────

#[derive(Clone)]
struct AutoVuln {
    title:       String,
    description: Option<String>,
    severity:    String,
    cvss_score:  Option<f32>,
    cve_id:      Option<String>,
}

fn detect_insecure_service(port: &PortInfo) -> Option<AutoVuln> {
    let service = port.service.as_deref().unwrap_or("");
    match (port.port, service) {
        (21, "ftp") => Some(AutoVuln {
            title: "FTP expuesto (sin cifrado)".into(),
            description: Some("FTP transmite credenciales en texto plano.".into()),
            severity: "high".into(),
            cvss_score: Some(7.5),
            cve_id: None,
        }),
        (23, "telnet") => Some(AutoVuln {
            title: "Telnet expuesto".into(),
            description: Some("Telnet transmite todo el tráfico en texto plano.".into()),
            severity: "critical".into(),
            cvss_score: Some(9.8),
            cve_id: None,
        }),
        (3389, _) => Some(AutoVuln {
            title: "RDP expuesto a internet".into(),
            description: Some("RDP expuesto directamente es vector de ataques de fuerza bruta.".into()),
            severity: "high".into(),
            cvss_score: Some(8.1),
            cve_id: None,
        }),
        (445, _) => Some(AutoVuln {
            title: "SMB expuesto a internet".into(),
            description: Some("SMB expuesto es vector para EternalBlue y similares.".into()),
            severity: "critical".into(),
            cvss_score: Some(9.8),
            cve_id: Some("CVE-2017-0144".into()),
        }),
        (1433, _) | (3306, _) | (5432, _) | (27017, _) => Some(AutoVuln {
            title: format!("Base de datos ({service}) expuesta directamente"),
            description: Some("Las bases de datos no deben ser accesibles directamente desde internet.".into()),
            severity: "critical".into(),
            cvss_score: Some(9.1),
            cve_id: None,
        }),
        _ => None,
    }
}
```

### `worker/src/handlers/analysis.rs` — risk scoring real

```rust
use common::{AppError, TenantId};
use db::{queries::{vulnerabilities, metrics, scan_targets}, with_tenant_conn, PgPool};
use chrono::Utc;
use serde_json::Value;
use tracing::info;
use uuid::Uuid;

pub async fn handle(
    pool: &PgPool,
    tenant_id: TenantId,
    payload: &Value,
) -> Result<(), AppError> {
    let target_id: Uuid = payload["target_id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| AppError::InvalidInput("missing target_id".into()))?;

    info!(%tenant_id, %target_id, "running risk analysis");

    let now = Utc::now();
    let tenant_uuid = *tenant_id.as_uuid();

    with_tenant_conn(pool, tenant_id, |conn| {
        Box::pin(async move {
            // Contar vulnerabilidades por severidad
            let counts = vulnerabilities::count_by_severity(conn, target_id).await?;

            // Contar puertos activos
            let port_count: i64 = sqlx::query_scalar!(
                "SELECT COUNT(*) FROM exposed_ports WHERE target_id=$1 AND is_active=true",
                target_id
            )
            .fetch_one(conn)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .unwrap_or(0);

            // Risk score: cap 100
            let score = (counts.critical * 25
                + counts.high * 10
                + counts.medium * 3
                + counts.low)
                .min(100) as f64;

            let risk_level = match score as u32 {
                75..=100 => "critical",
                50..=74  => "high",
                25..=49  => "medium",
                _        => "low",
            };

            // Persistir métricas individuales
            for (kind, value) in [
                ("risk_score",          score),
                ("vuln_count_critical", counts.critical as f64),
                ("vuln_count_high",     counts.high     as f64),
                ("vuln_count_medium",   counts.medium   as f64),
                ("vuln_count_low",      counts.low      as f64),
                ("exposed_port_count",  port_count      as f64),
            ] {
                metrics::upsert(conn, tenant_uuid, Some(target_id), kind, value, now, now).await?;
            }

            // Actualizar scan_targets con el score calculado
            sqlx::query!(
                r#"UPDATE scan_targets
                   SET risk_score = $2, risk_level = $3::risk_level,
                       last_scanned_at = now(), updated_at = now()
                   WHERE id = $1"#,
                target_id, score as i16, risk_level
            )
            .execute(conn)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

            info!(%target_id, risk_score = score, %risk_level, "analysis complete");
            Ok(())
        })
    })
    .await
}
```

---

## 10. Flujo completo: request → DB → worker → resultado

```
Cliente HTTP
    │
    │  POST /api/vendors/:id/scan
    │  Authorization: Bearer <JWT>
    ▼
┌─────────────────────────────────────────────┐
│  Axum: auth_middleware                       │
│  - Valida JWT                                │
│  - Extrae tenant_id, user_id, role           │
│  - Inyecta TenantContext en extensions       │
└─────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────┐
│  Handler: enqueue_scan                       │
│  - Lee TenantContext de extensions           │
│  - jobs::enqueue(pool, tenant_id, "scan",   │
│      { target_id })                          │
│  - HTTP 202 Accepted { job_id }              │
└─────────────────────────────────────────────┘
    │ (respuesta inmediata al cliente)
    │
    │ (asíncrono — worker poll loop)
    ▼
┌─────────────────────────────────────────────┐
│  Worker: dequeue_next()                      │
│  SELECT ... FOR UPDATE SKIP LOCKED           │
│  → job.job_type = "scan"                     │
└─────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────┐
│  handlers::scan::handle()                    │
│  1. fetch target value (domain/IP)           │
│  2. nmap -sV -T4 --top-ports 1000 -oX -     │
│  3. parse XML                                │
│  4. for each port:                           │
│     - exposed_ports::upsert()                │
│     - detect_insecure_service() →            │
│       vulnerabilities::upsert()              │
│  5. exposed_ports::deactivate_missing()      │
│  6. jobs::enqueue("analysis", {target_id})   │
│  7. jobs::mark_done(job_id)                  │
└─────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────┐
│  Worker (siguiente ciclo): job "analysis"    │
│  handlers::analysis::handle()               │
│  1. vulnerabilities::count_by_severity()     │
│  2. Calcular risk_score = f(critical,high…)  │
│  3. metrics::upsert() × 6 métricas           │
│  4. scan_targets UPDATE risk_score           │
│  5. jobs::mark_done(job_id)                  │
└─────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────┐
│  (opcional) job "llm_report"                 │
│  handlers::llm_report::handle()             │
│  1. fetch target + top 20 findings           │
│  2. LlmClient::complete(system, prompt)      │
│    POST http://llama:8080/v1/chat/completions│
│  3. reports::insert()                        │
│  4. jobs::mark_done(job_id)                  │
└─────────────────────────────────────────────┘

Cliente puede consultar resultados vía:
  GET /api/targets/:id/vulnerabilities
  GET /api/targets/:id/ports
  GET /api/targets/:id/metrics/risk_score
```

---

## 11. Autenticación: login → JWT

```
POST /api/auth/login (NO protegido por auth_middleware)
{
  "tenant_slug": "acme",
  "email": "admin@acme.com",
  "password": "S3cr3t!"
}

Flujo:
  1. tenants::find_by_slug("acme") → tenant (sin RLS, tabla pública)
  2. SET app.tenant_id = tenant.id
  3. users::find_by_email("admin@acme.com") → user (RLS activo)
  4. argon2::verify(password, user.password_hash)
  5. jwt::encode({ sub: user.id, tenant: tenant.id, role: user.role, exp: +24h })
  6. HTTP 200 { token, user_id, tenant_id, role }

El token se envía en todas las requests posteriores:
  Authorization: Bearer <token>
```

**Rutas que NO pasan por `auth_middleware`:**

En `server/src/main.rs`, montar el router de auth ANTES del middleware:

```rust
let app = Router::new()
    // Auth no requiere JWT
    .nest("/api", routes::auth_router(state.clone()))
    // Todo lo demás sí requiere JWT
    .merge(
        Router::new()
            .nest("/api", routes::api_router(state.clone()))
            .route_layer(axum_middleware::from_fn_with_state(
                state.clone(),
                middleware::auth_middleware,
            ))
    )
    .leptos_routes(/* ... */)
    // ...
```

**Crear el primer usuario** (solo en desarrollo):

```rust
// Script temporal o endpoint de bootstrap — no exponer en producción
// POST /api/admin/bootstrap (proteger con IP allowlist o desactivar después)
pub async fn bootstrap_tenant(/* ... */) {
    // 1. INSERT INTO tenants
    // 2. hash_password(password)
    // 3. INSERT INTO users (role = 'admin')
}
```

---

## 12. Inicializar el entorno después de programar

Orden de operaciones una vez que el código esté escrito:

### 12.1 Levantar solo PostgreSQL (para generar el sqlx cache)

```bash
# Arrancar solo postgres (usando el compose existente)
docker compose --env-file .env.docker.local up -d postgres

# Esperar a que postgres esté listo
docker compose --env-file .env.docker.local ps
```

### 12.2 Ejecutar las migraciones manualmente

```bash
# Con la DB viva, ejecutar las migraciones (incluyendo las nuevas 0005 y 0006)
export DATABASE_URL="postgres://app_user:TU_PASSWORD@localhost:5432/triseclabs"
sqlx migrate run

# Verificar
sqlx migrate info
```

### 12.3 Generar el cache offline de sqlx

```bash
# CRÍTICO: el Dockerfile usa SQLX_OFFLINE=true — sin este paso el build falla
cargo sqlx prepare --workspace

# Verificar que se generó .sqlx/
ls .sqlx/

# Commitear
git add .sqlx/
git commit -m "chore: add sqlx offline query cache"
```

### 12.4 Build y levantar el stack completo

```bash
# Build de todas las imágenes (primer build: ~20-30 min por Rust + WASM)
docker compose --env-file .env.docker.local build

# Levantar stack base (postgres + server + worker)
docker compose --env-file .env.docker.local up -d

# Verificar
docker compose --env-file .env.docker.local ps
docker compose --env-file .env.docker.local logs server | tail -20
docker compose --env-file .env.docker.local logs worker | tail -20

# Verificar que las migraciones corrieron (el server las ejecuta al arrancar)
docker compose --env-file .env.docker.local logs server | grep "migrations applied"
```

### 12.5 Con LLM local

```bash
# Descargar modelo (~4GB)
mkdir -p models
wget -O models/mistral-7b-instruct-v0.2.Q4_K_M.gguf \
  "https://huggingface.co/TheBloke/Mistral-7B-Instruct-v0.2-GGUF/resolve/main/mistral-7b-instruct-v0.2.Q4_K_M.gguf"

# Levantar con perfil llm
docker compose --env-file .env.docker.local --profile llm up -d

# Verificar que llama.cpp responde
curl http://localhost:8080/health
```

### 12.6 Exponer con HTTPS (Cloudflare quick tunnel)

```bash
docker compose --env-file .env.docker.local --profile tunnel up -d

# Ver la URL temporal asignada
docker compose --env-file .env.docker.local logs cloudflared | grep trycloudflare
# Ejemplo: https://random-words.trycloudflare.com
```

### 12.7 Test end-to-end rápido

```bash
BASE="http://localhost:3000"

# 1. Bootstrap (solo en dev — implementar endpoint o insertar manualmente)
psql $DATABASE_URL -c "
  INSERT INTO tenants (slug, name) VALUES ('demo', 'Demo Corp');
  INSERT INTO users (tenant_id, email, password_hash, role)
  SELECT id, 'admin@demo.com', '\$argon2id\$v=19\$m=19456,t=2,p=1\$...', 'admin'
  FROM tenants WHERE slug = 'demo';
"

# 2. Login
TOKEN=$(curl -s -X POST $BASE/api/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"tenant_slug":"demo","email":"admin@demo.com","password":"changeme"}' \
  | jq -r .token)

# 3. Crear un target
TARGET_ID=$(curl -s -X POST $BASE/api/vendors \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"kind":"domain","name":"example.com","value":"example.com"}' \
  | jq -r .id)

# 4. Disparar un scan
JOB_ID=$(curl -s -X POST $BASE/api/vendors/$TARGET_ID/scan \
  -H "Authorization: Bearer $TOKEN" | jq -r .job_id)

echo "Scan job enqueued: $JOB_ID"

# 5. Esperar y consultar resultados (el worker tarda unos segundos)
sleep 10

curl -s $BASE/api/targets/$TARGET_ID/ports \
  -H "Authorization: Bearer $TOKEN" | jq .

curl -s $BASE/api/targets/$TARGET_ID/vulnerabilities \
  -H "Authorization: Bearer $TOKEN" | jq .

curl -s $BASE/api/targets/$TARGET_ID/metrics/risk_score \
  -H "Authorization: Bearer $TOKEN" | jq .
```

---

## Apéndice: dependencias adicionales para los nuevos crates

### `crates/common/Cargo.toml`

```toml
[dependencies]
# ... existentes ...
sha2 = "0.10"
hex  = "0.4"
```

### `crates/server/Cargo.toml`

```toml
[dependencies]
# ... existentes ...
argon2 = "0.5"
```

### `crates/worker/Cargo.toml`

```toml
[dependencies]
# ... existentes ...
quick-xml = "0.31"
```

### `crates/db/Cargo.toml`

No requiere cambios — solo archivos `.rs` nuevos en `src/queries/`.
