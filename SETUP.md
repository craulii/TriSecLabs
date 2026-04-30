# SETUP.md — TriSecLabs desde cero

Guía de bootstrapping completa. Sin saltos. Comandos reales.

---

## 0. Prerrequisitos del sistema

### 0.1 Rust

```bash
# Instalar rustup (gestor de toolchain)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

# Verificar
rustc --version   # rustc 1.78+
cargo --version
```

### 0.2 Node.js 20+

```bash
# Via nvm (recomendado — permite gestionar versiones)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.7/install.sh | bash
source ~/.bashrc   # o ~/.zshrc

nvm install 20
nvm use 20
node --version    # v20.x
npm --version     # 10.x
```

### 0.3 Docker + Docker Compose v2

```bash
# Ubuntu/Debian
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER
newgrp docker

# Verificar
docker --version           # 25+
docker compose version     # 2.x (compose integrado, no docker-compose)
```

### 0.4 Dependencias de sistema para Tauri v2

```bash
# Ubuntu 22.04 / Debian 12
sudo apt-get update && sudo apt-get install -y \
    libwebkit2gtk-4.1-dev \
    build-essential \
    curl \
    wget \
    file \
    libssl-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    libgtk-3-dev

# macOS: solo Xcode Command Line Tools
xcode-select --install

# Windows: Visual Studio C++ build tools (desde vs_buildtools.exe)
```

### 0.5 cargo-watch (hot-reload Axum en dev)

```bash
cargo install cargo-watch
```

---

## 1. Crear el monorepo

```bash
# Directorio raíz del proyecto
mkdir -p triseclabs
cd triseclabs

# Estructura de crates Rust
mkdir -p crates/shared/src \
         crates/db/src/queries \
         crates/llm/src \
         crates/backend/src/middleware \
         crates/backend/src/routes \
         crates/workers/src/handlers

# Estructura de apps
mkdir -p apps/desktop/src \
         migrations \
         infra/traefik/dynamic \
         infra/postgres

# .gitignore desde el inicio
cat > .gitignore << 'EOF'
/target/
.env.docker.local
.env.prod
.env.local
/models/
/infra/traefik/acme/
node_modules/
apps/web/build/
apps/web/.svelte-kit/
EOF
```

---

## 2. Inicializar el workspace Cargo

```bash
# Workspace raíz — sin [package], solo [workspace]
cat > Cargo.toml << 'EOF'
[workspace]
resolver = "2"
members = [
    "crates/shared",
    "crates/db",
    "crates/llm",
    "crates/backend",
    "crates/workers",
    "apps/desktop",
]

[workspace.dependencies]
tokio        = { version = "1", features = ["full"] }
axum         = { version = "0.7", features = ["macros"] }
tower        = { version = "0.5" }
tower-http   = { version = "0.6", features = ["fs", "compression-gzip", "trace", "cors"] }
sqlx         = { version = "0.8", features = ["runtime-tokio-rustls","postgres","uuid","chrono","json","migrate"] }
serde        = { version = "1", features = ["derive"] }
serde_json   = "1"
jsonwebtoken = "9"
bcrypt       = "0.15"
uuid         = { version = "1", features = ["v4", "serde"] }
chrono       = { version = "0.4", features = ["serde"] }
thiserror    = "2"
anyhow       = "1"
tracing      = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
reqwest      = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
EOF

# Verificar que cargo lo lee (no compila nada, solo parsea)
cargo metadata --no-deps --format-version 1 > /dev/null && echo "workspace OK"
```

---

## 3. Levantar PostgreSQL con Docker

```bash
# Crear .env para desarrollo local (no Docker networking)
cat > .env << 'EOF'
DATABASE_URL=postgres://app_user:dev_password@localhost:5432/triseclabs
JWT_SECRET=dev_secret_min_32_chars_change_in_prod
LLM_BASE_URL=http://localhost:8080
LLM_MODEL=mistral-7b-instruct
HOST=127.0.0.1
PORT=3000
RUST_LOG=info,server=debug,worker=debug
SQLX_OFFLINE=false
STATIC_DIR=apps/web/build
EOF

# Levantar SOLO postgres (aislado, sin construir los binarios)
docker run --name triseclabs-pg \
  -e POSTGRES_DB=triseclabs \
  -e POSTGRES_USER=app_user \
  -e POSTGRES_PASSWORD=dev_password \
  -p 127.0.0.1:5432:5432 \
  -d postgres:16-alpine

# Esperar a que esté listo (normalmente 3-5 segundos)
until docker exec triseclabs-pg pg_isready -U app_user -d triseclabs; do
  sleep 1
done
echo "PostgreSQL listo"
```

> **Nota sobre RLS en desarrollo**: `POSTGRES_USER=app_user` crea este rol como superuser
> en el container Docker. Los superusers de PostgreSQL bypasan RLS por defecto.
> Las políticas RLS son efectivas en producción donde `app_user` es un rol sin privilegios.
> En dev, la invariante de tenant isolation la mantiene la aplicación via `with_tenant_conn`.

---

## 4. Inicializar los crates Rust (shared, db, llm, backend, workers)

Copiar los archivos del monorepo o crearlos desde cero. Con el repo ya creado:

```bash
# Verificar que los Cargo.toml existen en cada crate
ls crates/shared/Cargo.toml \
   crates/db/Cargo.toml \
   crates/llm/Cargo.toml \
   crates/backend/Cargo.toml \
   crates/workers/Cargo.toml

# Primer check — descarga dependencias (puede tardar 2-5 min la primera vez)
cargo check -p server 2>&1 | tail -5
```

Si algún crate falta, los Cargo.toml mínimos son:

```bash
# crates/shared/Cargo.toml
cat > crates/shared/Cargo.toml << 'EOF'
[package]
name    = "common"
version = "0.1.0"
edition = "2021"

[dependencies]
serde     = { workspace = true }
uuid      = { workspace = true }
chrono    = { workspace = true }
thiserror = { workspace = true }
EOF

# crates/db/Cargo.toml
cat > crates/db/Cargo.toml << 'EOF'
[package]
name    = "db"
version = "0.1.0"
edition = "2021"

[dependencies]
common     = { path = "../shared" }
sqlx       = { workspace = true }
tokio      = { workspace = true }
uuid       = { workspace = true }
chrono     = { workspace = true }
serde      = { workspace = true }
serde_json = { workspace = true }
thiserror  = { workspace = true }
tracing    = { workspace = true }
EOF
```

---

## 5. Ejecutar migraciones

```bash
# Instalar sqlx-cli (herramienta de migraciones)
cargo install sqlx-cli --no-default-features --features rustls,postgres

# Exportar DATABASE_URL para que sqlx la use
export DATABASE_URL=postgres://app_user:dev_password@localhost:5432/triseclabs

# Ejecutar todas las migraciones (0001–0007)
sqlx migrate run --source migrations/

# Verificar
sqlx migrate info --source migrations/
```

Salida esperada:
```
20240101000001/installed 0001_tenants
20240101000002/installed 0002_users
20240101000003/installed 0003_scan_targets
20240101000004/installed 0004_jobs
20240101000005/installed 0005_vulnerabilities_ports
20240101000006/installed 0006_metrics
20240101000007/installed 0007_dev_seed
```

Verificar que el seed funcionó:
```bash
docker exec triseclabs-pg psql -U app_user -d triseclabs \
  -c "SELECT t.slug, u.email, u.role FROM users u JOIN tenants t ON t.id = u.tenant_id;"
```

Salida esperada:
```
 slug | email          | role
------+----------------+-------
 demo | admin@demo.com | admin
```

---

## 6. Generar cache sqlx offline (necesario para compilar sin DB activa)

```bash
# Con la DB viva (paso 3 completado), generar el cache
cargo sqlx prepare --workspace

# Verificar que se creó
ls .sqlx/

# Esto debe ir al control de versiones
# En el .gitignore NO debe estar .sqlx/
git add .sqlx/ 2>/dev/null || echo ".sqlx/ generado, agregar al repo"
```

---

## 7. Compilar y levantar el servidor Axum

```bash
# Primer build completo (4-10 min dependiendo del hardware)
cargo build -p server

# Verificar que compiló
ls target/debug/server

# Levantar con hot-reload (recarga en cada cambio en crates/)
cargo watch -x "run -p server" -w crates/

# O sin hot-reload:
cargo run -p server
```

Salida esperada:
```
2024-01-01T00:00:00Z  INFO migrations applied
2024-01-01T00:00:00Z  INFO server listening address=127.0.0.1:3000
```

Verificar el endpoint de login:
```bash
curl -s -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"tenant_slug":"demo","email":"admin@demo.com","password":"admin123"}' | jq .
```

Respuesta esperada:
```json
{
  "token": "eyJ...",
  "user_id": "uuid-here",
  "tenant_id": "uuid-here",
  "role": "admin"
}
```

Verificar endpoint protegido con el token:
```bash
TOKEN=$(curl -s -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"tenant_slug":"demo","email":"admin@demo.com","password":"admin123"}' | jq -r .token)

curl -s http://localhost:3000/api/vendors \
  -H "Authorization: Bearer $TOKEN" | jq .
```

---

## 8. Inicializar SvelteKit

```bash
# Crear el proyecto SvelteKit en apps/web/
# (si apps/web ya existe con los archivos del repo, saltar este paso)
npm create svelte@latest apps/web
# Opciones:
#   ✓ Skeleton project
#   ✓ TypeScript
#   ✓ No ESLint/Prettier (opcionales, agregar después)
```

Si el proyecto YA tiene `apps/web/` con todos los archivos:

```bash
cd apps/web
npm install
```

Verificar que package.json tiene las dependencias correctas:
```bash
cat apps/web/package.json | jq '.dependencies, .devDependencies'
```

Debe incluir: `echarts`, `@tauri-apps/api`, `@sveltejs/adapter-static`, `tailwindcss`.

Instalar dependencias faltantes si aplica:
```bash
cd apps/web
npm install echarts @tauri-apps/api @tauri-apps/plugin-shell
npm install -D @sveltejs/adapter-static @tailwindcss/vite tailwindcss
```

### Verificar svelte.config.js

```bash
cat apps/web/svelte.config.js
```

Debe tener `adapter-static` con `fallback: 'index.html'`. Si no:

```js
// apps/web/svelte.config.js
import adapter from '@sveltejs/adapter-static';

export default {
  kit: {
    adapter: adapter({ pages: 'build', assets: 'build', fallback: 'index.html' }),
  },
};
```

---

## 9. Conectar frontend con backend — primera carga de datos en UI

```bash
# Terminal 1: servidor Axum
cargo run -p server

# Terminal 2: Vite dev server
cd apps/web
npm run dev
```

Abrir `http://localhost:5173` en el browser.

Debe redirigir a `/login`. Credenciales: `demo` / `admin@demo.com` / `admin123`.

Si el login falla con 401, verificar:
```bash
# ¿El seed se aplicó?
docker exec triseclabs-pg psql -U app_user -d triseclabs \
  -c "SELECT count(*) FROM users;"

# ¿El servidor Axum está corriendo?
curl -I http://localhost:3000/api/auth/login

# ¿El proxy de Vite está activo?
# En apps/web/vite.config.ts debe existir:
#   proxy: { '/api': { target: 'http://localhost:3000' } }
```

Tras el login exitoso, el dashboard en `/dashboard` carga los targets vía `GET /api/vendors`.

---

## 10. Levantar el worker en background

```bash
# Terminal 3: worker de jobs
cargo run -p worker
```

Salida esperada:
```
2024-01-01T00:00:00Z  INFO worker started, polling for jobs
```

El worker encola un scan desde la UI: Dashboard → fila de target → botón "Scan".
En los logs del worker verás:
```
INFO processing job job_id=... job_type=scan
INFO scan completed target_id=...
```

---

## 11. Inicializar Tauri v2

### 11.1 Instalar Tauri CLI v2

```bash
cd apps/desktop
npm install
# @tauri-apps/cli v2 ya está en package.json como devDependency
npx tauri --version   # 2.x
```

### 11.2 Verificar la configuración

```bash
cat apps/desktop/tauri.conf.json
```

Debe tener:
```json
{
  "build": {
    "frontendDist": "../web/build",
    "devUrl": "http://localhost:5173",
    "beforeBuildCommand": "cd ../web && npm run build"
  }
}
```

### 11.3 Arrancar en modo dev

```bash
# Terminal 1: Axum debe estar corriendo (paso 7)
# Terminal 2: Vite debe estar corriendo (cd apps/web && npm run dev)

# Terminal 3: Tauri dev
cd apps/desktop
npx tauri dev
```

Tauri abre una ventana nativa que carga `http://localhost:5173`. La app es idéntica al browser — misma SvelteKit SPA, misma API.

### 11.4 Build de producción desktop

```bash
# Construir SvelteKit primero
cd apps/web && npm run build

# Build Tauri (compila Rust + empaqueta)
cd ../desktop
npx tauri build
```

El binario final está en:
```
target/release/bundle/
├── deb/    (Linux)
├── AppImage/ (Linux)
├── dmg/    (macOS)
└── msi/    (Windows)
```

---

## 12. Verificación end-to-end

Con todo corriendo (Postgres + Axum server + Axum worker + Vite):

```bash
# 1. Login
TOKEN=$(curl -s -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"tenant_slug":"demo","email":"admin@demo.com","password":"admin123"}' \
  | jq -r .token)
echo "Token: ${TOKEN:0:40}..."

# 2. Listar targets
curl -s http://localhost:3000/api/vendors \
  -H "Authorization: Bearer $TOKEN" | jq '.[].name'

# 3. Crear target nuevo
NEW_ID=$(curl -s -X POST http://localhost:3000/api/vendors \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"kind":"domain","name":"Test Corp","value":"testcorp.com"}' \
  | jq -r .id)
echo "Nuevo target: $NEW_ID"

# 4. Encolar scan
curl -s -X POST "http://localhost:3000/api/vendors/$NEW_ID/scan" \
  -H "Authorization: Bearer $TOKEN" | jq .

# 5. Ver jobs procesados en logs del worker
```

---

## 13. Troubleshooting común

### `error: no such file or directory: migrations/`
El path de `sqlx::migrate!` es relativo al Cargo.toml del crate.
`crates/backend/src/main.rs` usa `sqlx::migrate!("../../migrations")` → sube dos niveles desde `crates/backend/` → llega a la raíz. Verificar que existe `migrations/` en la raíz.

### `ERROR: relation "users" does not exist`
Las migraciones no se ejecutaron. Correr `sqlx migrate run --source migrations/`.

### `401 Unauthorized` en todos los endpoints
Verificar que el header es exactamente `Authorization: Bearer <token>` (con espacio después de Bearer).

### `connection refused` al servidor Axum desde Vite
Verificar que el servidor Axum está corriendo en `127.0.0.1:3000` y que `vite.config.ts` tiene `proxy: { '/api': 'http://localhost:3000' }`.

### Tauri: `WebKit2Gtk not found`
```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev
# En Ubuntu 20.04 puede requerir:
sudo apt-get install -y libwebkit2gtk-4.0-dev
```

### `error: the lock file ... needs to be updated`
```bash
cargo update
```

### `sqlx: query not found in offline cache`
La query cambió pero el cache `.sqlx/` no se regeneró.
```bash
export DATABASE_URL=postgres://app_user:dev_password@localhost:5432/triseclabs
cargo sqlx prepare --workspace
```

### Puerto 5432 ocupado
```bash
# Ver qué usa el puerto
sudo lsof -i :5432
# Parar el container
docker stop triseclabs-pg
```

### Reset completo de la DB
```bash
docker stop triseclabs-pg && docker rm triseclabs-pg
# Volver al paso 3
```

---

## Resumen de procesos activos en dev

| Terminal | Proceso | Puerto |
|----------|---------|--------|
| T1 | `cargo watch -x "run -p server"` | `3000` |
| T2 | `cargo run -p worker` | — (poll loop) |
| T3 | `cd apps/web && npm run dev` | `5173` |
| T4 | `cd apps/desktop && npx tauri dev` | ventana nativa → `:5173` |
| Docker | `triseclabs-pg` | `5432` |

Para LLM (opcional):
```bash
docker compose --profile llm up -d   # llama.cpp en :8080
```
