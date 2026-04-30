#!/usr/bin/env bash
# dev.sh — levanta el stack local de TriSecLabs (postgres + api + worker + vite).
#
# Comportamiento:
#   1. Inicia postgres si no está corriendo.
#   2. Compila server y worker (cargo decide si rebuilds o no).
#   3. Mata cualquier instancia previa de server/worker para evitar correr código viejo.
#   4. Lanza server y espera a que responda en :3000 antes de continuar.
#   5. Lanza worker.
#   6. Inicia vite (npm run dev) en primer plano.
#
# Logs: /tmp/triseclabs-server.log y /tmp/triseclabs-worker.log

set -euo pipefail

REPO="$(cd "$(dirname "$0")" && pwd)"
export PATH="/usr/lib/postgresql/16/bin:$PATH"

cd "$REPO"

# ─── 1. PostgreSQL ──────────────────────────────────────────────────────────

if ! pg_ctl status -D "$HOME/postgres/data" &>/dev/null; then
    echo "[pg] iniciando..."
    pg_ctl start -D "$HOME/postgres/data" -l "$HOME/postgres/postgres.log"
    # Espera hasta 10s a que postgres acepte conexiones
    for _ in {1..10}; do
        pg_isready -h localhost -p 5433 &>/dev/null && break
        sleep 1
    done
else
    echo "[pg] ya está corriendo"
fi

if ! pg_isready -h localhost -p 5433 &>/dev/null; then
    echo "[pg] ERROR: postgres no responde en :5433"
    exit 1
fi

# ─── 2. Variables de entorno ────────────────────────────────────────────────

if [[ ! -f "$REPO/.env" ]]; then
    echo "[env] ERROR: $REPO/.env no existe (copia .env.example y ajusta)"
    exit 1
fi

set -a
source "$REPO/.env"
set +a

# ─── 3. Build server + worker ───────────────────────────────────────────────

echo "[build] compilando server + worker..."
cargo build -p server -p worker 2>&1 | tail -3

# ─── 4. Reiniciar server + worker para evitar correr binarios viejos ────────

echo "[stop] matando server/worker previos si existen..."
pkill -f "target/debug/server" 2>/dev/null || true
pkill -f "target/debug/worker" 2>/dev/null || true
sleep 1

mkdir -p "$REPO/target"

echo "[api] iniciando en :3000"
nohup "$REPO/target/debug/server" >> /tmp/triseclabs-server.log 2>&1 &
SERVER_PID=$!
disown $SERVER_PID

# Esperar a que el server responda (max 15s)
for i in {1..30}; do
    if curl -s -o /dev/null -w "%{http_code}" http://localhost:3000/api/auth/login -X POST \
        -H 'Content-Type: application/json' -d '{}' 2>/dev/null | grep -qE "^(400|422)$"; then
        echo "[api] listo (PID $SERVER_PID)"
        break
    fi
    if ! kill -0 $SERVER_PID 2>/dev/null; then
        echo "[api] ERROR: el server murió. Últimas líneas del log:"
        tail -20 /tmp/triseclabs-server.log
        exit 1
    fi
    sleep 0.5
    if [[ $i -eq 30 ]]; then
        echo "[api] WARN: el server no respondió en 15s; continuando de todos modos"
    fi
done

echo "[worker] iniciando..."
nohup "$REPO/target/debug/worker" >> /tmp/triseclabs-worker.log 2>&1 &
WORKER_PID=$!
disown $WORKER_PID
sleep 1

if ! kill -0 $WORKER_PID 2>/dev/null; then
    echo "[worker] ERROR: el worker murió. Últimas líneas del log:"
    tail -20 /tmp/triseclabs-worker.log
    exit 1
fi
echo "[worker] listo (PID $WORKER_PID)"

# ─── 5. Frontend ────────────────────────────────────────────────────────────

echo ""
echo "[web] http://localhost:5173"
echo "[api] http://localhost:3000"
echo "[demo] tenant=demo  email=admin@demo.com  password=admin123"
echo ""

cd "$REPO/apps/web"
exec npm run dev
