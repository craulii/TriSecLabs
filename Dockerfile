# =============================================================================
# STAGE 1: builder
# Construye el SPA de SvelteKit y compila los binarios Axum (server) y worker.
# Sin Leptos/WASM: stack reemplazado por SvelteKit 2 + Svelte 5.
# =============================================================================
FROM rust:1.78-bookworm AS builder

# pkg-config, libssl-dev: openssl-sys (dep transitivo de reqwest/sqlx)
# ca-certificates, curl: TLS en npm y healthcheck
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

# Node.js 20 LTS: requerido para `npm run build` del SvelteKit SPA.
# nodesource garantiza la versión exacta sin interferir con la imagen Rust base.
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# ── Rust: pre-fetch de dependencias ──────────────────────────────────────────
# Solo manifiestos primero. Este layer se invalida cuando cambia Cargo.lock o
# un Cargo.toml — no cuando cambia el código fuente. Ahorra ~10 min en rebuilds.
COPY Cargo.toml Cargo.lock ./
COPY crates/shared/Cargo.toml  crates/shared/Cargo.toml
COPY crates/db/Cargo.toml      crates/db/Cargo.toml
COPY crates/llm/Cargo.toml     crates/llm/Cargo.toml
COPY crates/backend/Cargo.toml crates/backend/Cargo.toml
COPY crates/workers/Cargo.toml crates/workers/Cargo.toml
# apps/desktop es workspace member (Tauri); se incluye para que cargo resuelva
# el workspace completo. No se compila con -p server/-p worker.
COPY apps/desktop/Cargo.toml   apps/desktop/Cargo.toml

# Stubs mínimos: cargo necesita archivos fuente para compilar,
# pero `cargo fetch` solo los necesita para resolver el grafo.
RUN mkdir -p crates/{shared,db,llm}/src \
             crates/backend/src crates/workers/src \
             apps/desktop/src migrations && \
    for d in shared db llm; do printf 'pub fn _s(){}' > crates/$d/src/lib.rs; done && \
    printf 'fn main(){}' > crates/backend/src/main.rs && \
    printf 'fn main(){}' > crates/workers/src/main.rs && \
    printf 'pub fn _s(){}' > apps/desktop/src/lib.rs
RUN cargo fetch

# ── Node: pre-install de dependencias ────────────────────────────────────────
# Solo package.json; se invalida cuando cambian deps del frontend, no el código.
COPY apps/web/package.json apps/web/
RUN cd apps/web && npm install

# ── SvelteKit: build del SPA ─────────────────────────────────────────────────
# Copiar fuentes después de npm install para que node_modules no se sobreescriba
# (ver .dockerignore: apps/web/node_modules excluido del build context).
COPY apps/web/ apps/web/
RUN cd apps/web && npm run build
# Output: apps/web/build/  ← directorio estático servido por ServeDir en Axum

# ── Rust: código fuente real ──────────────────────────────────────────────────
# Este COPY invalida los layers de compilación cuando cambia código Rust.
COPY crates/     crates/
COPY migrations/ migrations/

# SQLX_OFFLINE=true: usa el cache .sqlx/ commiteado en lugar de conectarse a DB.
# Prerrequisito: ejecutar `cargo sqlx prepare --workspace` con DB activa
# y commitear .sqlx/ al repositorio antes del primer docker build.
COPY .sqlx/ .sqlx/
ENV SQLX_OFFLINE=true

# Binario Axum: sirve la API REST + SvelteKit SPA
# Output: target/release/server
RUN cargo build --release -p server

# Binario worker: poll loop de background jobs
# Output: target/release/worker
RUN cargo build --release -p worker

# =============================================================================
# STAGE 2: server-runtime
# Imagen mínima: binario Axum + SvelteKit build estático.
# DATABASE_URL y JWT_SECRET son obligatorios — el servidor panica si faltan.
# =============================================================================
FROM debian:bookworm-slim AS server-runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/target/release/server ./server
# SvelteKit SPA: servido por ServeDir("/") con fallback a index.html (SPA routing)
COPY --from=builder /build/apps/web/build         ./web/build

# STATIC_DIR: ruta relativa al CWD (/app) del directorio SvelteKit compilado.
# Coincide con el default de main.rs cuando la var está presente.
# DATABASE_URL y JWT_SECRET NO tienen default: el proceso panica si faltan.
ENV HOST=0.0.0.0 \
    PORT=3000 \
    LLM_BASE_URL=http://llama:8080 \
    LLM_MODEL=mistral-7b-instruct \
    RUST_LOG=info \
    STATIC_DIR=web/build

EXPOSE 3000

HEALTHCHECK --interval=10s --timeout=5s --start-period=30s --retries=5 \
    CMD curl -f http://localhost:3000/ || exit 1

CMD ["./server"]

# =============================================================================
# STAGE 3: worker-runtime
# Imagen mínima: solo el binario worker.
# No necesita assets estáticos — solo conectividad a postgres y llama.
# DATABASE_URL es obligatorio — el proceso panica si falta.
# =============================================================================
FROM debian:bookworm-slim AS worker-runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/target/release/worker ./worker

ENV LLM_BASE_URL=http://llama:8080 \
    LLM_MODEL=mistral-7b-instruct \
    RUST_LOG=info

CMD ["./worker"]
