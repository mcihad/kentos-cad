#!/usr/bin/env bash
# The desktop's cloud interface against a real kentosd on the development
# database (docs/adr/0041): builds kentosd and runs the ignored live test
# (apps/desktop/src/cloud/live_run.rs), which starts and stops the server
# itself to take the connection away. The development accounts' password
# comes from .env.local and is never printed. Screenshots go to
# .run/shots/bulut-*.png; copies and drafts to .run/cloud-live/.
#
#   apps/desktop/scripts/cloud-live.sh
set -euo pipefail
cd "$(dirname "$0")/../../.."
if [ ! -f .env.local ]; then
  echo ".env.local yok: önce yerel veritabanını kurun (make db-setup)." >&2
  exit 1
fi
mkdir -p .run
cargo build -q -p kentos-api
KENTOS_DEV_PASSWORD="$(sed -n 's/^KENTOS_DEV_PASSWORD=//p' .env.local)" \
KENTOS_SNAPSHOT_BACKEND="${KENTOS_SNAPSHOT_BACKEND:-wgpu}" \
  cargo test -q -p kentos-desktop --bin kentos-cad cloud_live -- --ignored --nocapture
