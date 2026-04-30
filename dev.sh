#!/usr/bin/env bash
set -e

REPO="$(cd "$(dirname "$0")" && pwd)"
export PATH="/usr/lib/postgresql/16/bin:$PATH"

# PostgreSQL
if ! pg_ctl status -D ~/postgres/data &>/dev/null; then
    echo "[pg] iniciando..."
    pg_ctl start -D ~/postgres/data -l ~/postgres/postgres.log
    sleep 2
else
    echo "[pg] ya está corriendo"
fi

# Axum server
set -a && source "$REPO/.env" && set +a
if ! pgrep -f "target/debug/server" &>/dev/null; then
    echo "[api] compilando..."
    cargo build -p server 2>&1 | tail -3
    echo "[api] iniciando en :3000"
    "$REPO/target/debug/server" >> /tmp/triseclabs-server.log 2>&1 &
    sleep 2
else
    echo "[api] ya está corriendo"
fi

# Frontend
echo "[web] iniciando en :5173"
cd "$REPO/apps/web"
npm run dev
