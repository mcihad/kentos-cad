#!/usr/bin/env bash
# The desktop's Sunucu işleri tab against a real kentosd (docs/adr/0547): runs the ignored live test
# (apps/desktop/src/cloud/live_run.rs, `jobs_live`) on the database the environment names. It never touches
# `.env.local`'s development database unless that is what is configured: for a throwaway one, give the URLs by
# environment (KENTOS_DATABASE_URL, KENTOS_DATABASE_OWNER_URL), migrate it (`kentosd migrate`) and seed it
# (`kentosd dev-seed`, with KENTOS_ENV_FILE naming a scratch file for the generated password) first. The password
# comes from KENTOS_DEV_PASSWORD or the env file and is never printed. Screenshots go to .run/shots/bulut-isler-*.png.
#
#   KENTOS_ENV_FILE=/tmp/jobs.env KENTOS_BLOB_DIR=/tmp/jobs-blobs apps/desktop/scripts/jobs-live.sh
set -euo pipefail
cd "$(dirname "$0")/../../.."
mkdir -p .run
cargo build -q -p kentos-api
env_file="${KENTOS_ENV_FILE:-.env.local}"
if [ -z "${KENTOS_DEV_PASSWORD:-}" ] && [ -f "$env_file" ]; then
  KENTOS_DEV_PASSWORD="$(sed -n 's/^KENTOS_DEV_PASSWORD=//p' "$env_file")"
  export KENTOS_DEV_PASSWORD
fi
KENTOS_SNAPSHOT_BACKEND="${KENTOS_SNAPSHOT_BACKEND:-wgpu}" \
  cargo test -q -p kentos-desktop --bin kentos-cad jobs_live -- --ignored --nocapture
