# TriSecLabs — Documentación Técnica

> Documentación completa para desarrolladores. Cubre arquitectura, esquema de base de datos, API, frontend y despliegue.

---

## Tabla de contenidos

1. [Visión general](#1-visión-general)
2. [Stack tecnológico](#2-stack-tecnológico)
3. [Estructura del monorepo](#3-estructura-del-monorepo)
4. [Base de datos — Esquema completo](#4-base-de-datos--esquema-completo)
5. [Multi-tenancy y Row Level Security](#5-multi-tenancy-y-row-level-security)
6. [Backend — Crates Rust](#6-backend--crates-rust)
7. [API REST — Referencia completa](#7-api-rest--referencia-completa)
8. [Autenticación y JWT](#8-autenticación-y-jwt)
9. [Cola de trabajos asíncronos](#9-cola-de-trabajos-asíncronos)
10. [Workers — Procesamiento en background](#10-workers--procesamiento-en-background)
11. [Integración LLM](#11-integración-llm)
12. [Frontend SvelteKit](#12-frontend-sveltekit)
13. [Seguridad Tauri (app de escritorio)](#13-seguridad-tauri-app-de-escritorio)
14. [Variables de entorno](#14-variables-de-entorno)
15. [Desarrollo local](#15-desarrollo-local)
16. [Despliegue con Docker](#16-despliegue-con-docker)

---

## 1. Visión general

TriSecLabs es una plataforma CTEM (**Continuous Threat Exposure Management**) multi-tenant. Permite a organizaciones monitorear continuamente su superficie de ataque externa: dominios, rangos de IP, proveedores y entidades corporativas.

El sistema escanea activos, detecta vulnerabilidades, calcula scores de riesgo y genera informes ejecutivos en lenguaje natural usando un LLM local (privacidad garantizada, sin datos a la nube).

**Flujo principal:**
```
Usuario crea asset → Lanza scan → Worker procesa → Vulnerabilidades detectadas
→ Worker analysis calcula risk score → LLM genera informe en español
→ Dashboard muestra métricas y tendencias históricas
```

---

## 2. Stack tecnológico

| Capa | Tecnología | Versión |
|---|---|---|
| Frontend | SvelteKit 5 + Svelte 5 (runes) | ^2.5 / ^5.0 |
| Estilos | Tailwind CSS v4 (plugin Vite) | ^4.0 |
| Gráficos | Apache ECharts | ^5.5 |
| Backend API | Axum (Rust async web framework) | ^0.7 |
| Runtime async | Tokio | ^1 |
| Base de datos | PostgreSQL 16 | — |
| ORM/queries | sqlx (compile-time checked) | ^0.8 |
| Autenticación | JWT HS256 (jsonwebtoken crate) | ^9 |
| Hashing | bcrypt cost=12 | ^0.15 |
| App desktop | Tauri v2 | ^2.0 |
| LLM | llama.cpp (OpenAI-compat API) | — |
| Modelo por defecto | Mistral-7B-Instruct | — |
| Contenedores | Docker + Docker Compose | — |
| Proxy reverso | Traefik | v3 |

---

## 3. Estructura del monorepo

```
TriSecLabs/
├── apps/
│   ├── web/                    SvelteKit SPA (adapter-static, SSR desactivado)
│   │   ├── src/
│   │   │   ├── routes/         Páginas de la aplicación
│   │   │   │   ├── +layout.ts          Guard de autenticación global
│   │   │   │   ├── +layout.svelte      Layout raíz (tema + toasts)
│   │   │   │   ├── login/              Página de inicio de sesión
│   │   │   │   └── (app)/              Rutas protegidas (requieren JWT)
│   │   │   │       ├── +layout.svelte  Layout con Sidebar + Topbar
│   │   │   │       ├── dashboard/      Dashboard KPIs + timeline
│   │   │   │       ├── assets/         Lista de activos escaneados
│   │   │   │       ├── assets/[id]/    Detalle de activo (ports + vulns)
│   │   │   │       ├── vulnerabilities/Lista global de vulnerabilidades
│   │   │   │       ├── reports/        Módulo informes LLM (placeholder)
│   │   │   │       └── settings/
│   │   │   │           ├── tenants/    Gestión de tenants (placeholder)
│   │   │   │           └── users/      Gestión de usuarios (placeholder)
│   │   │   └── lib/
│   │   │       ├── api/        Clientes HTTP tipados por módulo
│   │   │       ├── components/ Componentes reutilizables
│   │   │       ├── stores/     Estado global (auth, ui, tenant)
│   │   │       └── types/      Interfaces TypeScript espejo de modelos Rust
│   │   ├── package.json
│   │   └── vite.config.ts      Proxy /api → :3000 en desarrollo
│   │
│   └── desktop/                Tauri v2 — envuelve la web app
│       ├── src/
│       │   ├── lib.rs          Setup Tauri: lanza sidecar Axum, monitorea proceso
│       │   └── main.rs         Entry point (llama lib::run)
│       ├── capabilities/
│       │   └── default.json    Permisos mínimos Tauri (shell spawn únicamente)
│       └── tauri.conf.json     Config Tauri: CSP, sidecar, seguridad
│
├── crates/
│   ├── shared/     (package: common)   Tipos compartidos sin dependencias runtime
│   ├── db/                            Capa de datos: pool, queries, RLS helpers
│   ├── llm/                           Cliente HTTP para llama.cpp
│   ├── backend/    (package: server)  Binario Axum: API HTTP + serving estático
│   └── workers/    (package: worker)  Binario de procesamiento en background
│
├── migrations/     Migraciones SQL numeradas (aplicadas al arranque del server)
├── infra/          Config Traefik + tuning PostgreSQL
├── docs/           Esta documentación
├── Cargo.toml      Workspace Rust
├── docker-compose.yml
└── dev.sh          Script para levantar el entorno local completo
```

---

## 4. Base de datos — Esquema completo

### 4.1 Tablas principales

#### `tenants` — Organizaciones clientes
```sql
id         UUID        PK, gen_random_uuid()
slug       TEXT        UNIQUE — identificador en URL ("demo", "acme-corp")
name       TEXT        Nombre legible ("Acme Corporation")
created_at TIMESTAMPTZ
updated_at TIMESTAMPTZ
```

#### `users` — Usuarios del sistema
```sql
id            UUID     PK
tenant_id     UUID     FK → tenants (CASCADE DELETE)
email         TEXT     UNIQUE por tenant
password_hash TEXT     bcrypt cost=12
role          ENUM     'admin' | 'analyst'
active        BOOLEAN  Soft-delete / desactivación
created_at    TIMESTAMPTZ
updated_at    TIMESTAMPTZ
```
Índice único: `(tenant_id, email)`

#### `scan_targets` — Activos bajo monitoreo
```sql
id              UUID     PK
tenant_id       UUID     FK → tenants
kind            ENUM     'domain' | 'ip_range' | 'vendor' | 'organization'
name            TEXT     Nombre descriptivo ("Web principal")
value           TEXT     Valor técnico ("example.com", "192.168.0.0/24")
risk_score      SMALLINT 0–100, calculado por analysis worker
risk_level      ENUM     'critical'|'high'|'medium'|'low'|'info'
metadata        JSONB    Metadatos adicionales libres
last_scanned_at TIMESTAMPTZ
created_at / updated_at TIMESTAMPTZ
```
Índice único: `(tenant_id, kind, value)` — no duplica activos.

#### `exposed_ports` — Puertos expuestos detectados por nmap
```sql
id            UUID     PK
tenant_id     UUID     FK → tenants
target_id     UUID     FK → scan_targets
port          INTEGER  CHECK 1–65535
protocol      ENUM     'tcp' | 'udp' | 'sctp'
state         ENUM     'open' | 'filtered' | 'closed'
service       TEXT     Nombre del servicio ("http", "ssh")
product       TEXT     Software detectado ("Apache httpd")
version       TEXT     Versión ("2.4.51")
banner        TEXT     Banner del servicio
is_active     BOOLEAN  false = puerto cerrado en último scan
first_seen_at / last_seen_at TIMESTAMPTZ
```
Índice único: `(tenant_id, target_id, port, protocol)`

#### `vulnerabilities` — Vulnerabilidades con ciclo de vida completo
```sql
id               UUID     PK
tenant_id        UUID     FK → tenants
target_id        UUID     FK → scan_targets
port_id          UUID?    FK → exposed_ports (nullable)
fingerprint      TEXT     SHA256 determinístico para deduplicación
title            TEXT     Título de la vulnerabilidad
description      TEXT?
severity         ENUM     'critical'|'high'|'medium'|'low'|'info'
cvss_score       NUMERIC(4,1) 0.0–10.0
cvss_vector      TEXT?    Vector CVSS ("AV:N/AC:L/...")
cve_id           TEXT?    "CVE-2024-1234"
cwe_id           TEXT?    "CWE-79"
status           ENUM     'open'|'in_progress'|'mitigated'|'resolved'|'accepted'|'false_positive'
source           ENUM     'nmap'|'nessus'|'openvas'|'manual'|'llm_analysis'|'osint'
evidence         JSONB    Output crudo del scanner
remediation_note TEXT?    Nota del analista al actualizar estado
first_seen_at / last_seen_at / resolved_at TIMESTAMPTZ
```
Índice único: `(tenant_id, target_id, fingerprint)` — deduplicación entre scans.

#### `jobs` — Cola de trabajos asíncronos
```sql
id           UUID      PK
tenant_id    UUID      FK → tenants
job_type     ENUM      'scan' | 'analysis' | 'llm_report'
payload      JSONB     Datos del trabajo (target_id, etc.)
status       ENUM      'pending' | 'running' | 'done' | 'failed'
attempts     INT       Intentos realizados
max_attempts INT       Máximo (default 3)
error        TEXT?     Mensaje de error si failed
run_after    TIMESTAMPTZ Para reintentos con delay
progress     SMALLINT? 0..100, NULL si aún no aplica
current_step TEXT?     Etapa actual del job (ScanStage)
stats_json   JSONB     Datos en vivo (puertos descubiertos, log, notas)
```
**Sin RLS** — los workers acceden con conexión privilegiada.

Índice parcial `idx_jobs_running_recent` sobre `updated_at` para optimizar el polling SSE de jobs en ejecución.

#### `metrics` — Series temporales de métricas
```sql
id           UUID     PK
tenant_id    UUID     FK → tenants
target_id    UUID?    FK → scan_targets (null = métrica del tenant)
kind         ENUM     'risk_score'|'vuln_count_critical'|...|'scan_coverage'
value        NUMERIC  Valor numérico
period_start / period_end TIMESTAMPTZ
computed_at  TIMESTAMPTZ
```
Índice único: `(tenant_id, target_id, kind, period_start)`

#### `reports` — Informes generados por LLM
```sql
id           UUID     PK
tenant_id    UUID     FK → tenants
target_id    UUID     FK → scan_targets
content      TEXT     Texto completo del informe (Markdown)
model_used   TEXT     Modelo LLM usado ("mistral-7b-instruct")
generated_at TIMESTAMPTZ
```

#### `scan_findings` — Hallazgos raw del scanner
```sql
id          UUID     PK
tenant_id   UUID     FK → tenants
target_id   UUID     FK → scan_targets
title       TEXT
description TEXT?
severity    ENUM     risk_level
cve_id      TEXT?
cvss_score  NUMERIC(4,1)
raw_data    JSONB    Output original del scanner
found_at    TIMESTAMPTZ
```

### 4.2 Tipos ENUM PostgreSQL

| Enum | Valores |
|---|---|
| `user_role` | admin, analyst |
| `target_kind` | domain, ip_range, vendor, organization |
| `risk_level` | critical, high, medium, low, info |
| `job_status` | pending, running, done, failed |
| `job_type` | scan, analysis, llm_report |
| `port_protocol` | tcp, udp, sctp |
| `port_state` | open, filtered, closed |
| `vuln_status` | open, in_progress, mitigated, resolved, accepted, false_positive |
| `vuln_source` | nmap, nessus, openvas, manual, llm_analysis, osint |
| `metric_kind` | risk_score, vuln_count_critical, vuln_count_high, vuln_count_medium, vuln_count_low, exposed_port_count, mean_time_to_remediate, scan_coverage |

---

## 5. Multi-tenancy y Row Level Security

**Todas las tablas** (excepto `jobs`) tienen RLS habilitado con la siguiente política:

```sql
CREATE POLICY tenant_isolation ON <tabla>
    AS PERMISSIVE FOR ALL TO app_user
    USING (tenant_id = current_setting('app.tenant_id', true)::uuid);
```

### Cómo funciona en práctica

El parámetro de sesión `app.tenant_id` se establece antes de cada query usando los helpers de `crates/db/src/rls.rs`:

**`with_tenant_conn`** — Para lecturas o operaciones mixtas:
```rust
with_tenant_conn(&pool, tenant_id, |conn| Box::pin(async move {
    queries::scan_targets::list(conn).await
})).await?
```
Internamente ejecuta `SELECT set_config('app.tenant_id', $1, false)` y al finalizar hace `RESET app.tenant_id` para evitar leakage al pool de conexiones.

**`with_tenant`** — Para escrituras transaccionales:
```rust
with_tenant(&pool, tenant_id, |tx| Box::pin(async move {
    queries::scan_targets::create(tx, tenant_id, kind, name, value).await
})).await?
```
Usa `set_config(..., true)` (equivalente a `SET LOCAL`), que PostgreSQL resetea automáticamente al terminar la transacción.

> **Regla crítica:** Nunca ejecutar queries en tablas con RLS sin establecer `app.tenant_id` primero. El resultado sería 0 filas (RLS bloquea silenciosamente), no un error.

### Por qué `set_config` en lugar de `SET`

PostgreSQL no acepta parámetros vinculados (`$1`) en la sentencia `SET`. La solución correcta es:
```sql
SELECT set_config('app.tenant_id', $1, false)  -- con bind parameter ✓
SET app.tenant_id = $1                          -- INVÁLIDO ✗
```

---

## 6. Backend — Crates Rust

### `crates/shared` (package: `common`)

Sin dependencias de runtime. Contiene:

**`src/tenant.rs`**
- `TenantId` — newtype sobre `Uuid` para seguridad de tipos en compilación
- `TenantContext` — contexto inyectado por middleware en cada request: `{ tenant_id, user_id, user_role }`
- `UserRole` — enum `Admin | Analyst` con métodos `is_admin()`, `can_write()`

**`src/error.rs`** — `AppError` unificado:
| Variante | HTTP | Descripción |
|---|---|---|
| `Database(String)` | 500 | Error de sqlx |
| `NotFound` | 404 | Recurso no existe |
| `Unauthorized(String)` | 401 | Sin token o token inválido |
| `Forbidden` | 403 | Rol insuficiente |
| `InvalidInput(String)` | 422 | Datos de entrada inválidos |
| `Llm(String)` | 502 | Error del servidor LLM |
| `Internal(String)` | 500 | Error inesperado |

**`src/models.rs`** — Structs compartidos entre crates:
- `Tenant`, `User`, `ScanTarget`, `ScanFinding`, `Report`
- Payload types para jobs: `ScanPayload { target_id }`, `AnalysisPayload { target_id }`, `LlmReportPayload { target_id }`

---

### `crates/db` (package: `db`)

Abstracción de datos sobre sqlx. Todas las queries usan `query_as!` o `query!` con verificación en tiempo de compilación (requiere DB activa o cache `.sqlx/`).

**`src/pool.rs` — `create_pool(url) → PgPool`**
- Crea pool con 20 conexiones máximas, 2 mínimas, timeout 5s

**`src/rls.rs`**
- `with_tenant_conn(pool, tenant_id, closure)` — lectura con RLS
- `with_tenant(pool, tenant_id, closure)` — escritura transaccional con RLS

**`src/queries/auth.rs`**
- `find_user_by_tenant_and_email(pool, slug, email)` — JOIN users+tenants, devuelve hash para verificación

**`src/queries/scan_targets.rs`**
- `list(conn)` → `Vec<ScanTargetRow>` — todos los targets del tenant, ORDER BY created_at DESC, LIMIT 200
- `find_by_id(conn, id)` → `Option<ScanTargetRow>`
- `create(tx, tenant_id, kind, name, value)` → `ScanTargetRow`
- `update_risk(conn, id, score, level)` — actualiza risk_score y risk_level tras analysis

**`src/queries/vulnerabilities.rs`**
- `list_for_target(conn, target_id)` → ordenadas por severidad y first_seen_at
- `list_global(conn, page, limit, severity?, status?)` → QueryBuilder dinámico para filtros opcionales
- `update_status(tx, id, status, note?)` → actualiza estado y resolved_at si es "resolved"

**`src/queries/ports.rs`**
- `list_for_target(conn, target_id)` → puertos activos, ORDER BY port

**`src/queries/jobs.rs`**
- `dequeue_next(pool)` → `Option<JobRow>` — SELECT FOR UPDATE SKIP LOCKED, atómico
- `mark_done(pool, job_id)` — estado → 'done'
- `mark_failed(pool, job_id, error)` — estado → 'failed' con mensaje
- `enqueue(pool, tenant_id, job_type, payload)` → `Uuid` del job creado

**`src/queries/metrics.rs`**
- `history(conn, target_id, kind, limit)` → últimos N puntos de la métrica

**`src/queries/tenants.rs`**
- `find_by_slug(pool, slug)` → para validar tenant en login
- `create(pool, slug, name)` → nuevo tenant

---

### `crates/llm` (package: `llm`)

**`src/client.rs` — `LlmClient`**

Cliente HTTP para servidor llama.cpp (API compatible OpenAI):

```rust
pub struct LlmClient {
    base_url: String,
    model:    String,
    client:   reqwest::Client,  // timeout 120s
}

impl LlmClient {
    pub fn new(base_url: String, model: String) -> Self
    pub async fn complete(&self, system: &str, user: &str) -> Result<String, AppError>
}
```

- Endpoint: `POST {base_url}/v1/chat/completions`
- Temperatura: `0.3` (determinismo razonable)
- `max_tokens`: 2048
- Timeout: 120 segundos (los modelos locales son lentos)

---

### `crates/backend` (package: `server`)

Binario Axum que sirve la API y el SPA estático.

**`src/state.rs` — `AppState`**
```rust
pub struct AppState {
    pub db:             PgPool,
    pub llm:            LlmClient,
    pub jwt_secret:     String,
    pub login_throttle: Arc<Mutex<HashMap<String, (u32, Instant)>>>,
}
```
`login_throttle` es un mapa en memoria: clave `"tenant_slug:email"` → `(intentos, timestamp_primer_intento)`.

**`src/main.rs`** — arranque del servidor:
1. Inicializa `tracing-subscriber` con `RUST_LOG`
2. Carga env vars (`DATABASE_URL`, `JWT_SECRET`, etc.)
3. Crea pool PostgreSQL y aplica migraciones
4. Configura CORS con orígenes explícitos
5. Monta router: público (`/api/auth`) + protegido (`/api/*`) + archivos estáticos
6. Escucha en `HOST:PORT` (default `127.0.0.1:3000`)

**`src/middleware/auth.rs` — Middleware JWT**

Ejecuta antes de cada handler protegido:
1. Extrae `Authorization: Bearer <token>`
2. Verifica firma HS256, expiración, `iss = "triseclabs"`, `aud = "triseclabs-api"`
3. Decodifica claims: `sub` (user_id), `tenant` (tenant_id), `role`
4. Inyecta `TenantContext` en las extensiones del request
5. Retorna `401` en cualquier fallo de validación

---

## 7. API REST — Referencia completa

### Autenticación

**Sin middleware JWT:**

| Método | Ruta | Body | Respuesta |
|---|---|---|---|
| `POST` | `/api/auth/login` | `{tenant_slug, email, password}` | `{token, user_id, tenant_id, role}` |

**Con `Authorization: Bearer <token>`:**

### Activos (scan targets)

| Método | Ruta | Descripción |
|---|---|---|
| `GET` | `/api/vendors` | Lista todos los activos del tenant |
| `POST` | `/api/vendors` | Crea activo: `{kind, name, value}` |
| `GET` | `/api/vendors/:id` | Obtiene activo por ID |
| `PATCH` | `/api/vendors/:id` | Actualiza activo: `{name, value}` |
| `DELETE` | `/api/vendors/:id` | Elimina activo (cascada en puertos y vulns) |
| `POST` | `/api/vendors/:id/scan` | Encola job de tipo `scan`. Devuelve `{job_id}` |
| `POST` | `/api/vendors/:id/analyze` | Encola job de tipo `llm_report` |
| `GET` | `/api/vendors/:id/job` | Último scan job del target (snapshot completo) |

### Jobs

| Método | Ruta | Descripción |
|---|---|---|
| `GET` | `/api/jobs/:id` | Snapshot del job. Incluye `progress`, `stage`, `discovered_ports`, `log`, `note`, `error` |
| `GET` | `/api/jobs/:id/stream` | **SSE**: stream de eventos `JobProgressEvent` hasta status terminal. Auth dual: `Authorization: Bearer` o `?token=` (necesario porque `EventSource` no soporta headers custom) |

### Puertos

| Método | Ruta | Descripción |
|---|---|---|
| `GET` | `/api/targets/:id/ports` | Lista puertos expuestos activos del activo |

### Vulnerabilidades

| Método | Ruta | Descripción |
|---|---|---|
| `GET` | `/api/targets/:id/vulnerabilities` | Vulnerabilidades del activo, orden por severidad |
| `GET` | `/api/vulnerabilities` | Lista global con filtros: `?page=1&limit=25&severity=high&status=open` |
| `PATCH` | `/api/vulnerabilities/:id/status` | `{status, note?}` — actualiza estado |

### Métricas

| Método | Ruta | Descripción |
|---|---|---|
| `GET` | `/api/targets/:id/metrics/:kind` | Historial de métrica: `?limit=30` |

**Kinds disponibles:** `risk_score`, `vuln_count_critical`, `vuln_count_high`, `vuln_count_medium`, `vuln_count_low`, `exposed_port_count`, `mean_time_to_remediate`, `scan_coverage`

### Respuestas de error

```json
{ "error": "mensaje descriptivo" }
```

| Código | Causa |
|---|---|
| 400 | Cuerpo JSON inválido o campo faltante |
| 401 | Token ausente, expirado o inválido |
| 403 | Rol insuficiente |
| 404 | Recurso no encontrado |
| 422 | Validación de entrada fallida |
| 429 | Rate limiting: demasiados intentos de login |
| 500 | Error interno (ver logs del servidor) |

---

## 8. Autenticación y JWT

### Flujo de login

```
Cliente → POST /api/auth/login {tenant_slug, email, password}
  ↓
1. Verificar rate limit (máx 5 intentos / 15 min por cuenta)
2. Buscar tenant por slug
3. Buscar usuario por tenant_id + email
4. Si usuario no existe: bcrypt.verify(password, DUMMY_HASH) ← timing-safe
5. bcrypt.verify(password, user.password_hash)
6. Si falla: AppError::Unauthorized
7. Si ok: generar JWT, resetear contador de intentos
8. Devolver {token, user_id, tenant_id, role}
```

### Estructura del JWT

```json
{
  "sub":    "uuid del usuario",
  "tenant": "uuid del tenant",
  "role":   "admin" | "analyst",
  "iss":    "triseclabs",
  "aud":    "triseclabs-api",
  "iat":    1234567890,
  "exp":    1234567890  // iat + 8 horas
}
```

### Protecciones implementadas

- **Rate limiting en memoria:** `Arc<Mutex<HashMap<"slug:email", (intentos, Instant)>>>` — 5 intentos máximo, ventana de 15 minutos, sin dependencias externas.
- **Timing-safe:** Si el usuario no existe, se ejecuta `bcrypt::verify` con un hash dummy para que el tiempo de respuesta sea idéntico al caso de contraseña incorrecta. Evita enumeración de usuarios por timing.
- **Claims `iss` + `aud`:** El middleware valida emisor y audiencia además de la firma.

---

## 9. Cola de trabajos asíncronos

### Diseño

Sin Redis ni mensaje brokers externos. PostgreSQL actúa como cola mediante:

```sql
UPDATE jobs SET status = 'running', attempts = attempts + 1
WHERE id = (
    SELECT id FROM jobs
    WHERE status = 'pending' AND run_after <= now() AND attempts < max_attempts
    ORDER BY created_at
    FOR UPDATE SKIP LOCKED
    LIMIT 1
)
RETURNING ...
```

`SKIP LOCKED` garantiza que dos workers concurrentes nunca tomen el mismo job.

### Ciclo de vida de un job

```
pending → running → done
                 ↘ failed (si attempts >= max_attempts)
```

En caso de error, el job queda en estado `failed` con el mensaje de error. No hay reintento automático en la implementación actual (se puede implementar actualizando `run_after` en lugar de `mark_failed`).

### Tipos de payload (JSONB)

```rust
ScanPayload      { target_id: Uuid }
AnalysisPayload  { target_id: Uuid }
LlmReportPayload { target_id: Uuid }
```

---

## 10. Workers — Procesamiento en background

El binario `worker` (`crates/workers`) corre un loop de polling:

```
loop:
  1. dequeue_next(pool) → Option<JobRow>
  2. Si None: sleep(2s), continuar
  3. Deserializar payload según job_type
  4. Llamar handler correspondiente
  5. mark_done o mark_failed según resultado
```

El worker es **single-threaded por diseño** — las llamadas al LLM son secuenciales para no saturar la GPU/CPU del modelo local.

### Handler: `scan`

**Archivo:** `crates/workers/src/handlers/scan.rs`

Pipeline completo con progreso en vivo:

1. **Validación de input** (`validate_target_value`):
   - `domain` → regex DNS (RFC 1123)
   - `ip_range` → `IpAddr::parse` o `IpNetwork::parse` si contiene `/`
   - `vendor` / `organization` → `AppError::InvalidInput` (no escaneable directamente)
2. **Etapa `validating`** persistida en `jobs.current_step`.
3. **Spawn nmap** con flags:
   ```
   -sV --script=vulners --script-args=mincvss=5.0 -T4 --open
   -v --stats-every 3s -oX $HOME/triseclabs-tmp/scan-<job_id>.xml
   ```
   El XML va a archivo (no stdout) porque con `-oX -` nmap omite las líneas legibles de progreso. El path está en `$HOME` porque el snap de nmap no puede escribir en `/tmp`.
4. **Loop `tokio::select!`**: lee stdout y stderr línea a línea, parsea con regex:
   - `Initiating Ping Scan / Connect Scan / Service scan / Script scanning` → cambio de etapa
   - `About X% done` → progress dentro del rango de la etapa actual
   - `Discovered open port N/proto on IP` → push a `discovered_ports`
   - Throttle: persiste en DB cada 1s máximo
   - **Keepalive cada 5s**: avanza progress lentamente cuando nmap está silencioso (NSE pre-scan, DNS resolution)
5. **Detección de fallos silenciosos** (post-exit, aunque exit=0):
   - `Failed to resolve` → `InvalidInput("DNS no resuelve...")`
   - `Host seems down` → `InvalidInput("Host no responde a ping...")`
   - `0 hosts up` → `InvalidInput("Ningún host respondió...")`
6. **Etapa `persisting`**: parseo del XML, transacción que upsertea `exposed_ports` (ON CONFLICT por `(tenant_id, target_id, port, protocol)`) y `vulnerabilities` (ON CONFLICT por fingerprint SHA256). Marca como `is_active=false` los puertos no vistos en este scan.
7. **Encola automáticamente un job `analysis`** tras completar.

**Etapas (`ScanStage` en `crates/shared/src/models.rs`)**:

| Stage | Rango progress | Detectado por |
|---|---|---|
| `validating` | 0–2 | Pre-spawn |
| `starting` | 2–5 | `NSE: Loaded`, pre-scan |
| `host_discovery` | 5–10 | `Initiating Ping Scan`, DNS resolution |
| `port_scan` | 10–40 | `Initiating Connect/SYN Scan` + `% done` |
| `service_detection` | 40–70 | `Initiating Service scan` |
| `vulners` | 70–90 | `Script scanning` |
| `persisting` | 90–99 | Post-XML parse |
| `done` / `failed` | 100 / – | `mark_done` / `mark_failed` |

**Retry inteligente** (en dispatcher de `crates/workers/src/main.rs`):
- `AppError::InvalidInput` → `mark_failed` inmediato, sin reintento.
- `AppError::Internal` con `attempts < max_attempts` → `requeue_with_delay(60s)`.

### Real-time progress (SSE)

**Endpoint:** `GET /api/jobs/:id/stream`  (`crates/backend/src/routes/stream.rs`)

- **Auth dual**: header `Authorization: Bearer` o query param `?token=` (necesario porque `EventSource` no acepta headers custom).
- Polling de `jobs::get_by_id` cada 800ms con `futures::stream::unfold`.
- Emite `JobProgressEvent` solo cuando cambia `updated_at` o se alcanza status terminal.
- Cierre limpio al recibir `done` / `failed` + `take_until` con timeout duro de 15min.
- Keep-alive 15s.

**Snapshot fallback:** `GET /api/jobs/:id` devuelve el mismo `JobProgressEvent` para clientes que no soportan SSE (Tauri WebView en algunas plataformas).

**Tipo compartido (`crates/shared/src/models.rs`):**

```rust
pub struct JobProgressEvent {
    pub id:               Uuid,
    pub status:           String,            // pending|running|done|failed
    pub stage:            Option<ScanStage>,
    pub progress:         Option<i16>,        // 0..100
    pub discovered_ports: Vec<DiscoveredPort>,
    pub log:              Vec<String>,
    pub error:            Option<String>,
    pub note:             Option<String>,    // "Sin puertos abiertos detectados", etc.
    pub updated_at:       DateTime<Utc>,
}
```

### Handler: `analysis`

**Archivo:** `crates/workers/src/handlers/analysis.rs`

1. Carga todas las vulnerabilidades del target
2. Cuenta por severidad: `n_critical`, `n_high`, `n_medium`, `n_low`
3. Calcula risk score (0–100):
   ```
   score = min(100,
     n_critical * 25 +
     n_high     * 10 +
     n_medium   *  4 +
     n_low      *  1
   )
   ```
4. Mapea score a risk_level:
   - ≥ 75 → `critical`
   - ≥ 50 → `high`
   - ≥ 25 → `medium`
   - ≥ 10 → `low`
   - < 10 → `info`
5. Llama `scan_targets::update_risk(conn, target_id, score, level)`
6. Inserta punto en tabla `metrics` (series temporales)

### Handler: `llm_report`

**Archivo:** `crates/workers/src/handlers/llm_report.rs`

1. Carga target + top 20 vulnerabilidades (ordenadas por severidad)
2. Construye prompt del sistema en español:
   > "Eres un analista de ciberseguridad experto. Redacta informes ejecutivos claros, precisos y accionables."
3. Construye prompt del usuario con contexto del target y lista de hallazgos
4. Llama `LlmClient::complete(system, user)` → espera hasta 120s
5. Guarda informe en tabla `reports` con `model_used`

---

## 11. Integración LLM

### Servidor compatible

Cualquier servidor que implemente la API de chat completions de OpenAI:
- **llama.cpp** con `--server` (recomendado)
- **Ollama** (compatible)
- **LM Studio** (compatible)
- OpenAI API real (cambiando `LLM_BASE_URL`)

### Configuración

```env
LLM_BASE_URL=http://localhost:8080    # URL del servidor llama.cpp
LLM_MODEL=mistral-7b-instruct         # Nombre del modelo a usar
```

### Levantar en Docker

```bash
docker compose --profile llm up -d
```

Descarga y sirve el modelo automáticamente via la imagen de llama.cpp incluida en `docker-compose.yml`.

---

## 12. Frontend SvelteKit

### Configuración

- **`adapter-static`:** genera un SPA estático (`apps/web/build/`)
- **SSR desactivado:** todo se ejecuta en el cliente
- **Proxy en dev:** Vite redirige `/api` → `http://localhost:3000`
- **Tipo de módulo ES:** `"type": "module"` en `package.json`

### Rutas

| Ruta | Archivo | Descripción |
|---|---|---|
| `/login` | `routes/login/+page.svelte` | Formulario de inicio de sesión |
| `/dashboard` | `routes/(app)/dashboard/+page.svelte` | KPIs, timeline, crear activos |
| `/assets` | `routes/(app)/assets/+page.svelte` | Lista de activos con acciones |
| `/assets/[id]` | `routes/(app)/assets/[id]/+page.svelte` | Detalle: gauge, ports, vulns |
| `/vulnerabilities` | `routes/(app)/vulnerabilities/+page.svelte` | Lista global con filtros |
| `/reports` | `routes/(app)/reports/+page.svelte` | Informes LLM (placeholder) |
| `/settings/tenants` | `routes/(app)/settings/tenants/+page.svelte` | Gestión tenants (placeholder) |
| `/settings/users` | `routes/(app)/settings/users/+page.svelte` | Gestión usuarios (placeholder) |

**Guard de autenticación:** `routes/+layout.ts` — redirige a `/login` si no hay token válido, y de `/login` a `/dashboard` si ya hay sesión.

### Stores (`src/lib/stores/`)

**`auth.ts` — Estado de autenticación**
```typescript
interface AuthState {
  token:    string | null
  userId:   string | null
  tenantId: string | null
  role:     'admin' | 'analyst' | null
}
```
- En **Tauri:** token solo en memoria (nunca persiste)
- En **browser:** `sessionStorage` con clave `tsl_session`
- Detecta runtime Tauri via `window.__TAURI_INTERNALS__`
- `auth.login(response)` — guarda estado
- `auth.logout()` — limpia estado y redirige a `/login`
- Derived stores: `isAuthenticated`, `isAdmin`

**`ui.ts` — Estado de UI**
- `theme` — `'dark' | 'light'`, persiste en `localStorage['tsl_theme']`, detecta preferencia del sistema, aplica `data-theme` attribute en `<html>`
- `sidebarCollapsed` — estado del sidebar colapsado/expandido
- `toasts` — sistema de notificaciones con tipos `info | success | warning | error`

**`tenant.ts`**
- `activeTenantId` — derived de `auth.tenantId`

### Clientes API (`src/lib/api/`)

Todos los clientes usan el helper base en `client.ts`:

**`client.ts`** — `api.get/post/patch/delete`
- Base URL: `VITE_API_BASE ?? '/api'`
- Inyecta `Authorization: Bearer` automáticamente
- En 401: llama `auth.logout()` y redirige a `/login`
- Parsea `{ error: string }` del body para mensajes de error

**`auth.ts`** — `authApi.login(tenant, email, password)`

**`targets.ts`** — `targetsApi`
- `list()`, `get(id)`, `create({kind, name, value})`
- `update(id, {name, value})`, `delete(id)`
- `enqueueScan(id)` → `{job_id}`, `enqueueLlmReport(id)`
- `getLatestJob(id)` → `LatestJobSummary` (status, progress, current_step, stats_json, error)
- `listPorts(id)`

**`jobs.ts`** — `jobsApi`
- `getSnapshot(id)` → `JobProgressEvent` (snapshot puntual)
- `stream(id, onEvent, onError)` → `EventSource` que emite `JobProgressEvent` hasta status terminal. El caller debe llamar `.close()` en cleanup. Usado por `ScanLiveDrawer`.

**`vulnerabilities.ts`** — `vulnsApi`
- `listForTarget(targetId)`
- `list({page, limit, severity?, status?})`
- `updateStatus(id, status, note?)`

**`metrics.ts`** — `metricsApi`
- `history(targetId, kind, limit)` — N puntos más recientes
- `latest(targetId)` — carga en paralelo 4 métricas KPI del target

### Componentes clave (`src/lib/components/`)

**Layout:**
- `Sidebar.svelte` — navegación lateral con iconos SVG inline, activos/admin, colapso animado
- `Topbar.svelte` — toggle sidebar, cambio de tema, indicador de rol

**Common:**
- `DataTable.svelte` — tabla genérica con sorting, skeleton loading, slots para celdas y acciones
- `Pagination.svelte` — paginación con límite configurable
- `RiskBadge.svelte` — badge de color para `RiskLevel` y `VulnStatus`
- `ProgressBar.svelte` — barra de progreso determinada/indeterminada con label y ARIA

**Iconos (`src/lib/icons/`):** SVG inline lucide-style con prop `size`/`strokeWidth`:
`Play`, `Loader` (spin), `Bot`, `Pencil`, `Trash`, `Search`, `X`, `AlertCircle`, `Activity`, `Check`, `ChevronUp`/`Down`/`UpDown`. Reemplazan emojis por iconografía profesional consistente.

**Scan en vivo (`src/lib/components/scan/`):**
- `ScanLiveDrawer.svelte` — drawer 440px que abre al lanzar un scan. Conecta `EventSource` al endpoint SSE, muestra:
  - ProgressBar con etapa actual y porcentaje
  - Timeline vertical de las 7 etapas (`done` / `active` / `pending`)
  - Tabla de puertos descubiertos en tiempo real (port, proto, service)
  - Log tail con últimas 10–12 líneas de nmap
  - Sección de resumen al completar: mensaje claro para 0 puertos, error legible para fallos
  - Auto-cierra `EventSource` al recibir `done`/`failed`; cleanup en `$effect` y `onDestroy`

**Charts (ECharts via dynamic import en `onMount`):**
- `EChart.svelte` — wrapper reactivo para ECharts, destruye instancia en `onDestroy`
- `RiskGauge.svelte` — gauge 0–100 para risk score
- `VulnTimeline.svelte` — líneas temporales por severidad

### Sistema de temas y design tokens

CSS custom properties en `app.css`, aplicadas via `data-theme` en `<html>`. Base font-size **16px** (`--font-size-base`).

**Paleta:**

| Variable | Dark | Light |
|---|---|---|
| `--bg-base` | `#0f172a` | `#f8fafc` |
| `--bg-surface` | `#1e293b` | `#ffffff` |
| `--bg-elevated` | `#334155` | `#f1f5f9` |
| `--border` | `#475569` | `#e2e8f0` |
| `--text-primary` | `#f1f5f9` | `#0f172a` |
| `--text-secondary` | `#94a3b8` | `#475569` |
| `--accent` | `#3b82f6` | `#2563eb` |

Severidad: `--sev-critical` `#ef4444`, `--sev-high` `#f97316`, `--sev-medium` `#f59e0b`, `--sev-low` `#22c55e`, `--sev-info` `#94a3b8`.

**Tokens de sistema:**

- **Spacing** (escala 4px): `--space-1` (0.25rem) … `--space-16` (4rem)
- **Typography**: `--font-size-xs` (12px) → `--font-size-2xl` (30px); `--line-height-tight/base/loose`; `--font-weight-regular/medium/semibold/bold`
- **Radius**: `--radius-sm/md/lg/xl/pill`
- **Shadows**: `--shadow-sm/md/lg`
- **Layout**: `--max-content-width: 1280px`

---

## 13. Seguridad Tauri (app de escritorio)

### CSP activo

`apps/desktop/tauri.conf.json`:
```json
"security": {
  "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self' http://localhost:3000; img-src 'self' data:; font-src 'self'"
}
```
`connect-src` limita fetch al sidecar local. `unsafe-inline` en styles es necesario para Tailwind v4 runtime.

### Capabilities mínimas

`apps/desktop/capabilities/default.json` — solo permisos necesarios:
```json
{
  "permissions": ["core:default", "shell:allow-spawn", "shell:allow-execute"]
}
```
Sin `shell:allow-open` → no se pueden abrir URLs externas desde JS en el WebView.

### Monitoreo del sidecar

`apps/desktop/src/lib.rs` — el proceso Axum se spawna al inicio y se guarda el `CommandChild` en el managed state de Tauri. Un task asíncrono monitorea stdout/stderr y loguea si el proceso termina inesperadamente.

---

## 14. Variables de entorno

| Variable | Obligatoria | Default | Descripción |
|---|---|---|---|
| `DATABASE_URL` | ✓ | — | `postgres://user:pass@host:port/db` |
| `JWT_SECRET` | ✓ | — | Secret HS256 (mínimo 32 chars en prod) |
| `HOST` | — | `127.0.0.1` | IP de escucha del servidor |
| `PORT` | — | `3000` | Puerto del servidor |
| `STATIC_DIR` | — | `apps/web/build` | Directorio del build SvelteKit |
| `LLM_BASE_URL` | — | `http://localhost:8080` | URL del servidor llama.cpp |
| `LLM_MODEL` | — | `mistral-7b-instruct` | Nombre del modelo |
| `RUST_LOG` | — | `info` | Nivel de logging (`info,triseclabs=debug`) |
| `SQLX_OFFLINE` | — | `false` | `true` para builds sin DB (usa cache `.sqlx/`) |

---

## 15. Desarrollo local

### Prerrequisitos

- Rust (stable, `rustup`)
- Node.js 20+
- PostgreSQL 16 (local o Docker)

### Inicio rápido con `dev.sh`

```bash
cd ~/projects/TriSecLabs
./dev.sh
```

El script:
1. Inicia PostgreSQL (`pg_ctl start -D ~/postgres/data`) si no está corriendo, y verifica que `pg_isready` responda en `:5433`.
2. Carga `.env` (falla si no existe).
3. Compila `server` y `worker` (`cargo build` incremental).
4. **Mata cualquier instancia previa de server/worker con `pkill`** — evita correr binarios viejos tras un cambio de código.
5. Lanza el server con `nohup` y espera hasta 15s a que responda en `:3000` (healthcheck contra `/api/auth/login`).
6. Lanza el worker; verifica que no muera al arrancar.
7. `exec npm run dev` — Vite en primer plano. Server y worker (disowned) sobreviven al `Ctrl+C` final.

Logs: `/tmp/triseclabs-server.log` y `/tmp/triseclabs-worker.log`.

Accede a `http://localhost:5173` — credenciales: `admin@demo.com` / `admin123` (tenant: `demo`).

### Setup manual

```bash
# 1. PostgreSQL
pg_ctl start -D ~/postgres/data -l ~/postgres/postgres.log

# 2. Axum server (auto-aplica migraciones)
set -a && source .env && set +a
cargo run -p server

# 3. Frontend (otra terminal)
cd apps/web
npm install
npm run dev
```

### Regenerar cache sqlx (tras cambios de schema)

```bash
cargo sqlx prepare --workspace    # requiere DB activa
git add .sqlx/
```

---

## 16. Despliegue con Docker

### Prerrequisito único

Antes del primer build, el directorio `.sqlx/` debe existir y estar commiteado:
```bash
cargo sqlx prepare --workspace   # con DB activa
```
El Dockerfile usa `SQLX_OFFLINE=true` — sin este directorio el build falla.

### Build y arranque

```bash
cp .env.docker .env.docker.local
# Editar POSTGRES_PASSWORD y JWT_SECRET en .env.docker.local

docker compose --env-file .env.docker.local build
docker compose --env-file .env.docker.local up -d

# Con HTTPS via Cloudflare Tunnel:
docker compose --env-file .env.docker.local --profile tunnel up -d
```

### Producción (Hetzner)

```bash
docker compose -f docker-compose.yml -f docker-compose.prod.yml --env-file .env.prod up -d
```

### Servicios Docker

| Servicio | Descripción |
|---|---|
| `postgres` | PostgreSQL 16 con tuning en `infra/` |
| `server` | Binario Axum (API + SPA estático) |
| `worker` | Worker de jobs en background |
| `llm` (profile) | llama.cpp con modelo Mistral-7B |
| `tunnel` (profile) | Cloudflare Tunnel para HTTPS |

---

*Documentación generada para TriSecLabs v0.1.0*
