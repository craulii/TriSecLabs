# Changelog

Formato basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/).

## [Unreleased]

### Added — Scan en vivo con SSE

- **Nueva migración `0009_jobs_progress.sql`**: añade `progress SMALLINT`, `current_step TEXT`, `stats_json JSONB` a `jobs`, e índice parcial `idx_jobs_running_recent` para optimizar el polling SSE.
- **Endpoint `GET /api/jobs/:id`** (snapshot completo del job, autenticación normal).
- **Endpoint `GET /api/jobs/:id/stream`** (Server-Sent Events) en `crates/backend/src/routes/stream.rs`:
  - Auth dual: `Authorization: Bearer` o `?token=` (necesario porque `EventSource` no acepta headers custom).
  - Polling DB cada 800ms con `futures::stream::unfold`, emite `JobProgressEvent` solo cuando cambia.
  - Cierre automático al recibir `done`/`failed`, hard timeout 15min, keep-alive 15s.
- **Tipos compartidos** (`crates/shared/src/models.rs`): `ScanStage`, `DiscoveredPort`, `JobProgressEvent`.
- **DB queries** (`crates/db/src/queries/jobs.rs`):
  - `get_by_id(pool, tenant_id, job_id)` — filtro defensivo de tenant.
  - `update_progress(pool, job_id, progress, current_step, stats_merge)` — usado por el worker, merge `||` jsonb.
  - `requeue_with_delay(pool, job_id, delay_secs, last_error)` — para retry inteligente.
- **Frontend**:
  - `lib/api/jobs.ts` con `jobsApi.getSnapshot()` y `jobsApi.stream()`.
  - `lib/components/common/ProgressBar.svelte` — determinada/indeterminada con ARIA.
  - `lib/components/scan/ScanLiveDrawer.svelte` — drawer 440px con timeline de etapas, tabla de puertos descubiertos en vivo, log tail, mensaje de resumen al completar.
  - `lib/icons/` — 13 iconos SVG inline (`Play`, `Loader`, `Bot`, `Pencil`, `Trash`, `Search`, `X`, `AlertCircle`, `Activity`, `Check`, `ChevronUp/Down/UpDown`).

### Changed — Scan handler reescrito

- **Validación de input** (`validate_target_value`): regex DNS, `IpAddr`/`IpNetwork::parse`. Vendor/organization devuelven `InvalidInput` (no escaneable directamente, requiere OSINT).
- **`run_nmap_streaming`** reescrito en `crates/workers/src/handlers/scan.rs`:
  - Spawn nmap con `-v --stats-every 3s -oX <archivo>`. El XML va a `$HOME/triseclabs-tmp/scan-<job_id>.xml` porque el snap de nmap no puede escribir en `/tmp`.
  - Loop `tokio::select!` que lee stdout y stderr, parsea regex de etapas (`Initiating Connect Scan`, `% done`, `Discovered open port`), persiste progreso a DB con throttle 1s.
  - **Keepalive cada 5s**: avanza progress lentamente cuando nmap está silencioso (NSE pre-scan, DNS resolution).
  - **Detección de fallos silenciosos**: nmap retorna exit 0 incluso en `Failed to resolve` y `Host seems down`; los detectamos en stdout/stderr post-exit.
  - Timeout 300s → **600s** (vulners es lento).
- **Etapas tipadas**: `validating → starting → host_discovery → port_scan → service_detection → vulners → persisting → done`.
- **Retry inteligente** en dispatcher: `InvalidInput` falla inmediato, `Internal` reencola con `run_after = now() + 60s`.
- **`mark_done` setea `progress = 100`** automáticamente.

### Changed — UI rediseñada

- **Base font-size: 14px → 16px** (`apps/web/src/app.css`).
- **Tokens de diseño**: `--space-1..16`, `--font-size-xs..2xl`, `--line-height-*`, `--font-weight-*`, `--radius-sm..xl/pill`, `--shadow-sm/md/lg`, `--max-content-width: 1280px`.
- **Página `/assets`** reescrita:
  - Wrapper `max-width: var(--max-content-width)`.
  - Eliminado `setInterval` de polling (era memory leak — no había `onDestroy` cleanup).
  - Drawer en vivo abre al lanzar scan.
  - Botones con SVG (sin emojis).
  - Botón Scan deshabilitado en vendor/organization con tooltip explicativo.
  - Tipografía mayor en encabezados.

### Changed — `dev.sh` robustecido

- `set -euo pipefail`, fail-fast con mensajes claros.
- Verifica `pg_isready` después de iniciar postgres.
- **Mata server/worker previos siempre** antes de iniciar (evita correr binarios viejos tras un cambio).
- Espera healthcheck en `:3000` antes de continuar (max 15s).
- Verifica que el worker no muera al arrancar.
- Logs con PIDs y URLs visibles, `exec npm run dev` para mantener viva la sesión.

### Fixed

- nmap exit 0 con DNS fail / host down ya no se marca como `done` con 0 puertos en silencio: ahora falla con mensaje claro.
- Memory leak: `setInterval` en `assets/+page.svelte` que no se limpiaba al desmontar el componente.
- `parse_nmap_xml`: ahora usa el XML completo del archivo en lugar de stdout (que cuando es `-oX -` mezcla con líneas de progreso).

### API surface adicional

| Método | Ruta | Descripción |
|---|---|---|
| `PATCH` | `/api/vendors/:id` | Actualizar `name` y `value` |
| `DELETE` | `/api/vendors/:id` | Eliminar activo (cascada) |
| `GET` | `/api/vendors/:id/job` | Último scan job (snapshot, incluye progress) |
| `GET` | `/api/jobs/:id` | Snapshot del job |
| `GET` | `/api/jobs/:id/stream` | SSE stream de `JobProgressEvent` |

---

## [0.1.0] — 2026-04-30

Initial release. Plataforma CTEM multi-tenant en Rust + SvelteKit.

- Workspace Rust con `shared`, `db`, `llm`, `backend`, `workers`.
- PostgreSQL 16 con Row-Level Security en todas las tablas excepto `jobs`.
- Cola de trabajos basada en `SELECT FOR UPDATE SKIP LOCKED`.
- nmap como scanner activo con script `vulners` para detección de CVEs.
- LLM local vía llama.cpp (compatible OpenAI API).
- Frontend SvelteKit 5 + Svelte 5 runes, ECharts, adapter-static.
- Tauri v2 wrapper con sidecar Axum.
- Auth JWT HS256 + bcrypt, rate limiting en memoria.
