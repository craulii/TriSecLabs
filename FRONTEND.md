# FRONTEND.md

Documentación de arquitectura y decisiones del frontend TriSecLabs.

---

## Stack

| Capa | Tecnología | Motivo |
|---|---|---|
| Framework | SvelteKit 2 + Svelte 5 | Runes API nativa, compilación a JS puro, sin runtime pesado |
| Build mode | `adapter-static` + `ssr: false` | SPA pura — compatible con Tauri (no hay Node en desktop) |
| CSS | Tailwind v4 | Plugin Vite nativo, sin PostCSS config |
| Gráficos | Apache ECharts 5 | Mejor soporte de gauge/stacked bar para B2B; import dinámico en `onMount` para evitar SSR |
| Desktop | Tauri v2 | Wrapper ligero (~10 MB) vs Electron (~80 MB); sidecar Axum |
| API | Axum REST (puerto 3000) | Misma API que consume la web; proxy Vite en dev |

---

## Estructura de directorios

```
frontend/src/
├── app.css                   ← design tokens CSS custom properties (dark/light)
├── app.html                  ← HTML shell
├── lib/
│   ├── api/                  ← clientes HTTP por recurso (client.ts base, auth/targets/vulns/metrics)
│   ├── stores/               ← auth (sessionStorage), tenant, ui (theme/sidebar/toasts)
│   ├── types/models.ts       ← espejo exacto de los tipos Rust (actualizar en sync)
│   └── components/
│       ├── charts/           ← EChart.svelte (base), RiskGauge, VulnTimeline
│       ├── common/           ← DataTable (genérico T), Pagination, RiskBadge
│       └── layout/           ← Sidebar, Topbar
└── routes/
    ├── +layout.ts            ← auth guard SPA (redirect /login si no hay token)
    ├── +layout.svelte        ← root: theme init + toast stack
    ├── login/+page.svelte    ← form tenant_slug + email + password
    └── (app)/                ← grupo protegido con Sidebar + Topbar
        ├── +layout.svelte
        ├── dashboard/        ← KPI cards, VulnTimeline tenant, tabla targets
        ├── assets/           ← lista targets; /[id] detalle con gauge+ports+vulns
        └── vulnerabilities/  ← tabla global con filtros severity/status
```

---

## Auth flow

1. `+layout.ts` corre en cada navegación (SPA). Si no hay token en `auth` store → `redirect('/login')`.
2. Login form llama `POST /api/auth/login` con `{ tenant_slug, email, password }`.
3. Axum devuelve `{ token, user_id, tenant_id, role }`.
4. `auth.login(...)` persiste en `sessionStorage` (clave `tsl_session`).
5. `api/client.ts` adjunta `Authorization: Bearer <token>` en cada request; en 401 llama `auth.logout()` + `goto('/login')`.

**Upgrade path a httpOnly cookie**: implementar endpoint `POST /auth/refresh` en Axum + `Set-Cookie: HttpOnly; SameSite=Strict`. Eliminar sessionStorage, ajustar `client.ts` para enviar `credentials: 'include'`.

---

## Design tokens

Definidos en `app.css`. Aplicar siempre con `var(--token)`, nunca con clases Tailwind de color directas. Esto garantiza que el toggle dark/light funcione sin reescribir estilos.

Tokens principales: `--bg-base`, `--bg-surface`, `--bg-elevated`, `--border`, `--text-primary`, `--text-secondary`, `--text-muted`, `--accent`, `--sev-critical/high/medium/low/info`, `--status-open/resolved/...`

---

## ECharts — patrón de uso

```svelte
<!-- Nunca importar echarts directamente en un componente de ruta -->
<EChart {option} height="280px" />
```

`EChart.svelte` gestiona: import dinámico en `onMount`, registro de temas `tsl-dark`/`tsl-light`, `ResizeObserver`, `$effect` para re-render reactivo, `onDestroy` para dispose.

Para añadir un nuevo gráfico: crear `MyChart.svelte` que construya un `EChartsOption` como `$derived(...)` y lo pase a `<EChart>`.

---

## DataTable — patrón de uso

```svelte
<DataTable rows={data} columns={cols} loading={loading}>
  {#snippet cell({ row, col })}
    <!-- render custom por columna -->
  {/snippet}
  {#snippet rowActions({ row })}
    <!-- botones por fila -->
  {/snippet}
</DataTable>
```

Sorting client-side incluido. Para paginación server-side, usar con `<Pagination>` externo y recargar `rows` al cambiar de página.

---

## Tauri v2 — integración desktop

Archivo clave: `src-tauri/tauri.conf.json` define `externalBin: ["../server-binary"]`.

En `src-tauri/src/lib.rs`, el `setup` hace spawn del sidecar Axum al arrancar la ventana. El frontend siempre apunta a `http://localhost:3000` — no hay diferencia entre web y desktop.

```bash
# Compilar desktop (requiere Rust + cargo + tauri-cli v2)
npm run tauri build

# Dev desktop con HMR
npm run tauri dev
```

Para incluir el binario `server-binary` en el bundle desktop: compilar `crates/server` y copiar el output en `frontend/server-binary` antes de `npm run tauri build`.

---

## Pendiente (backend Axum)

Para que el frontend web funcione en producción sin Leptos:

1. En `crates/server/src/main.rs`: reemplazar el handler Leptos por `ServeDir::new("../frontend/dist").fallback(ServeFile::new("../frontend/dist/index.html"))` (SPA fallback).
2. Eliminar crates `app` + `leptos`/`leptos_axum` del workspace si no se van a usar.
3. Agregar `CorsLayer` de `tower-http` con `allow_origin(["tauri://localhost", "http://localhost:5173"])` para desktop/dev.
4. El Dockerfile deberá copiar `frontend/dist/` al container de runtime del servidor.
