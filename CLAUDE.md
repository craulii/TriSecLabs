# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

TriSecLabs is a multi-tenant CTEM (Continuous Threat Exposure Management) platform. It scans external assets (domains, IP ranges, vendors, organizations), scores their risk, and generates executive security reports using a local LLM.

## Monorepo layout

```
/
├── apps/
│   ├── web/          SvelteKit SPA (adapter-static, SSR off)
│   └── desktop/      Tauri v2 wrapper — spawns Axum sidecar
├── crates/
│   ├── shared/       Types: TenantId, AppError, domain models (package name = "common")
│   ├── db/           sqlx queries, pool, RLS helpers
│   ├── llm/          LlmClient wrapping llama.cpp OpenAI-compat endpoint
│   ├── backend/      Axum binary (package name = "server")
│   └── workers/      Background job binary (package name = "worker")
├── migrations/       PostgreSQL migrations (applied at server startup)
└── infra/            Traefik config, PostgreSQL tuning
```

## Commands

### Backend development

```bash
# PostgreSQL (required for everything)
docker compose up -d postgres

# Axum API server (hot-reload via cargo-watch)
cargo run -p server

# Background worker
cargo run -p worker

# LLM server (optional — needed for llm_report jobs)
docker compose --profile llm up -d
```

### Frontend development

```bash
cd apps/web
npm install
npm run dev          # Vite dev server on :5173 (proxies /api → :3000)
npm run check        # svelte-check type check
npm run build        # SPA bundle → apps/web/build/
```

### Desktop (Tauri v2)

```bash
# Requires Rust + cargo + @tauri-apps/cli v2
cd apps/desktop
npm install
npm run tauri:dev    # Opens Tauri window pointing at :5173
npm run tauri:build  # Bundles desktop app + Axum sidecar
```

### Build & check (Rust)

```bash
cargo check -p server        # Fast type check
cargo check -p worker
cargo clippy --workspace
cargo sqlx prepare --workspace  # Regenerate .sqlx/ offline cache (requires live DB)
```

### Docker (local)

```bash
# Prerrequisito: .sqlx/ debe existir y estar commiteado antes del primer build.
# El builder usa SQLX_OFFLINE=true — sin este directorio el build falla.
cargo sqlx prepare --workspace   # ejecutar una vez con DB activa

cp .env.docker .env.docker.local   # Edit POSTGRES_PASSWORD + JWT_SECRET
docker compose --env-file .env.docker.local build
docker compose --env-file .env.docker.local up -d
docker compose --env-file .env.docker.local --profile tunnel up -d  # HTTPS via cloudflared
```

### Docker (Hetzner)

```bash
docker compose -f docker-compose.yml -f docker-compose.prod.yml --env-file .env.prod up -d
```

## Architecture

### Crate responsibilities

| Crate | Package | Role |
|---|---|---|
| `crates/shared` | `common` | `TenantId`, `AppError`, domain models. No sqlx — no runtime deps. |
| `crates/db` | `db` | sqlx queries, pool (`create_pool`), RLS helpers (`with_tenant_conn`, `with_tenant`). |
| `crates/llm` | `llm` | `LlmClient` — wraps llama.cpp `/v1/chat/completions`. Timeout 120s. |
| `crates/backend` | `server` | Axum binary. Routes, JWT middleware, CORS, `ServeDir` for SvelteKit build. |
| `crates/workers` | `worker` | Poll loop: `SELECT FOR UPDATE SKIP LOCKED`, dispatches `scan`/`analysis`/`llm_report`. |
| `apps/desktop` | `triseclabs-desktop` | Tauri v2 binary. Spawns `server` sidecar on startup via `tauri-plugin-shell`. |

### Multi-tenancy via PostgreSQL RLS

Every table except `jobs` has Row Level Security. Policy: `tenant_id = current_setting('app.tenant_id', true)::uuid`.

**Rule: never query a tenant-isolated table without setting `app.tenant_id` first.**

Use helpers in `crates/db/src/rls.rs`:
- `with_tenant_conn(pool, tenant_id, |conn| ...)` — reads or mixed ops. Resets `app.tenant_id` on completion (prevents pool leakage).
- `with_tenant(pool, tenant_id, |tx| ...)` — transactional writes. Uses `SET LOCAL` (auto-reset at transaction end).

`jobs` has no RLS. The worker uses the pool directly, then calls `with_tenant_conn` internally for tenant data.

### Request auth flow

1. `POST /api/auth/login` (public) → validates tenant_slug + email + bcrypt password → returns JWT (HS256, 8h).
2. All other `/api/*` routes: `crates/backend/src/middleware/auth.rs` extracts `Authorization: Bearer <token>`, validates JWT, injects `TenantContext { tenant_id, user_id, user_role }` into Axum extensions.
3. Handlers extract `Extension(ctx): Extension<TenantContext>` and pass `ctx.tenant_id` to `with_tenant_conn`.

### API surface (`crates/backend/src/routes/api.rs`)

| Method | Path | Handler |
|---|---|---|
| `POST` | `/api/auth/login` | `routes/auth.rs` — public, no middleware |
| `GET` | `/api/vendors` | List scan targets |
| `POST` | `/api/vendors` | Create scan target |
| `GET` | `/api/vendors/:id` | Get single target |
| `POST` | `/api/vendors/:id/scan` | Enqueue `scan` job |
| `POST` | `/api/vendors/:id/analyze` | Enqueue `llm_report` job |
| `GET` | `/api/targets/:id/ports` | List exposed ports |
| `GET` | `/api/targets/:id/vulnerabilities` | List vulns for target |
| `GET` | `/api/targets/:id/metrics/:kind` | Metric history (30 pts default) |
| `GET` | `/api/vulnerabilities` | Global vuln list (paginated, filters) |
| `PATCH` | `/api/vulnerabilities/:id/status` | Update vuln status |

### Static serving

`crates/backend/src/main.rs` mounts `ServeDir::new(STATIC_DIR).fallback(ServeFile::new(".../index.html"))` at `/`. `STATIC_DIR` defaults to `apps/web/build` (dev) or `/app/web/build` (Docker via env var). The SPA fallback enables client-side routing.

### Job queue

PostgreSQL `SELECT FOR UPDATE SKIP LOCKED` — no Redis. Three job types: `scan`, `analysis`, `llm_report`. Worker is single-threaded by design (LLM calls are sequential). `attempts` increments on each try; job marked `failed` after `max_attempts` (default 3).

### CORS

`CorsLayer` in `main.rs` allows:
- `http://localhost:5173` — Vite dev
- `tauri://localhost` — Tauri WebView (macOS/Linux)
- `https://tauri.localhost` — Tauri WebView (Windows)

### Tauri desktop mode

`apps/desktop/src/lib.rs` spawns `server` (the Axum binary) as a sidecar on startup via `tauri-plugin-shell`. `tauri.conf.json` `externalBin` points to `../../target/release/server`. Frontend always calls `http://localhost:3000` — no code difference between web and desktop.

### LLM integration

`LlmClient` in `crates/llm` calls llama.cpp's `/v1/chat/completions`. Default model: `mistral-7b-instruct`. Timeout 120s. LLM reports generated in Spanish (system prompt in `workers/src/handlers/llm_report.rs`).

### Frontend (apps/web)

SvelteKit 5 (Svelte 5 runes), `adapter-static`, `ssr: false`. All routing is client-side. Auth guard in `+layout.ts` redirects unauthenticated users to `/login`. Stores: `auth` (sessionStorage JWT), `ui` (theme/sidebar/toasts). ECharts via dynamic import in `onMount` (SSR-safe).
