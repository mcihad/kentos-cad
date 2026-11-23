#!/usr/bin/env bash
# The forms' acceptance in the cloud with three test accounts (docs/adr/9194): a throwaway database with this build's
# migrations and the development accounts (apps/api/examples/e2e_database.rs), kentosd on a free port, the desktop's
# live test (apps/desktop/src/shell/cloud/live_forms.rs), then the cloud portal on a free port and its pictures of the
# project's page for the three accounts (apps/portal/scripts/forms-shots.mjs). At the end the server and the portal
# stop and the database is dropped.
#
# Needs a PostgreSQL with PostGIS as the server tests do (docs/kilavuz/02-calistirma.md): KENTOS_TEST_ADMIN_URL,
# KENTOS_DATABASE_OWNER_URL and KENTOS_DATABASE_URL (the application role's; its database name is not used), and
# Google Chrome for the portal's pictures. The accounts' password is made here, kept in .run/cloud-forms/password (600)
# and never printed. The pictures go to .run/gui-evidence/kullanim-9194/ (never in Git).
set -euo pipefail
cd "$(dirname "$0")/../../.."
: "${KENTOS_TEST_ADMIN_URL:?KENTOS_TEST_ADMIN_URL gerekli: PostGIS sunucusunun yönetici bağlantısı}"
: "${KENTOS_DATABASE_OWNER_URL:?KENTOS_DATABASE_OWNER_URL gerekli: şemanın sahibi rolün bağlantısı}"
: "${KENTOS_DATABASE_URL:?KENTOS_DATABASE_URL gerekli: uygulama rolünün bağlantısı}"
export KENTOS_TEST_DB=required
DIR=.run/cloud-forms
EVIDENCE=.run/gui-evidence/kullanim-9194
mkdir -p "$DIR"
rm -rf "$EVIDENCE" "$DIR/project.json" "$DIR/devices"
# The application role's address without its database: the scratch database's name goes after it.
APP_BASE="${KENTOS_DATABASE_URL%/*}"
free_port() { python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])'; }

# 1. The build, before anything waits on it: the server, the scratch database's maker, the desktop's tests.
scripts/dev/svc.sh heavy cargo build -p kentos-api --bin kentosd --example e2e_database
RUST_MIN_STACK=67108864 scripts/dev/svc.sh heavy cargo test -p kentos-desktop --lib --no-run

# 2. The scratch database (dropped when its standard input closes) and the server.
(umask 077 && python3 -c 'import secrets; print(secrets.token_urlsafe(18))' > "$DIR/password")
export KENTOS_DEV_PASSWORD="$(cat "$DIR/password")"
rm -f "$DIR/hold" "$DIR/db"
mkfifo "$DIR/hold"
target/debug/examples/e2e_database < "$DIR/hold" > "$DIR/db" 2> "$DIR/db.log" &
DB_PID=$!
exec 3> "$DIR/hold"
PIDS=()
cleanup() {
  for p in "${PIDS[@]:-}"; do [ -n "$p" ] && kill "$p" 2>/dev/null || true; done
  exec 3>&- || true
  wait "$DB_PID" 2>/dev/null || true
  rm -f "$DIR/hold"
}
trap cleanup EXIT
for _ in $(seq 1 120); do [ -s "$DIR/db" ] && break; sleep 0.5; done
NAME="$(head -1 "$DIR/db")"
case "$NAME" in
  kentos_cad_test_*) ;;
  *) echo "geçici veritabanı açılamadı:" >&2; cat "$DIR/db.log" >&2; exit 1 ;;
esac
API_PORT="$(free_port)"
KENTOS_API_PORT="$API_PORT" KENTOS_DATABASE_URL="$APP_BASE/$NAME" KENTOS_LOG=warn target/debug/kentosd serve > "$DIR/api.log" 2>&1 &
PIDS+=($!)
API="http://127.0.0.1:$API_PORT"
for _ in $(seq 1 150); do curl -fs "$API/v1/health" > /dev/null && break; sleep 0.2; done
curl -fs "$API/v1/health" > /dev/null || { echo "sunucu açılmadı:" >&2; cat "$DIR/api.log" >&2; exit 1; }
echo "geçici veritabanı $NAME, sunucu $API"

# 3. The desktop with the three accounts.
KENTOS_E2E_SERVER="$API" KENTOS_E2E_PASSWORD="$KENTOS_DEV_PASSWORD" RUST_MIN_STACK=67108864 \
  scripts/dev/svc.sh heavy cargo test -p kentos-desktop --lib cloud::live_forms::forms_roles_live -- --ignored --nocapture \
  | tee "$DIR/desktop.log"
grep -q "test result: ok. 1 passed" "$DIR/desktop.log" || { echo "masaüstü kabulü koşmadı" >&2; exit 1; }

# 4. The cloud portal against the same server, and its pictures.
PORTAL_PORT="$(free_port)"
(cd apps/portal && PORTAL_PORT="$PORTAL_PORT" KENTOS_API_PORT="$API_PORT" exec node_modules/.bin/vite --host 127.0.0.1 --logLevel warn) > "$DIR/portal.log" 2>&1 &
PIDS+=($!)
PORTAL="http://127.0.0.1:$PORTAL_PORT/"
for _ in $(seq 1 150); do curl -fs "$PORTAL" > /dev/null && break; sleep 0.2; done
PORTAL_URL="$PORTAL" PORTAL_DEMO_PASSWORD="$KENTOS_DEV_PASSWORD" FORMS_PROJECT="$DIR/project.json" \
  node apps/portal/scripts/forms-shots.mjs
echo "kabul tamam; resimler $EVIDENCE"
