#!/bin/sh
# The disk-full drill (docs/adr/0739 §3): kentos-application's `disk_full` test with the server's object store on a
# small tmpfs of a private mount namespace, so a write fails as on a full disk (ENOSPC). No root: an unprivileged
# user namespace mounts it (`unshare -rm`). The database is the test database (`.env.local`); the backup step needs
# `pg_dump` (`KENTOS_PG_DUMP`).
#
#   scripts/ops/disk_full_drill.sh [MB]     # the file system's size, default 24
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
size=${1:-24}
bin=$(cd "$root" && cargo test -q -p kentos-application --test disk_full --no-run --message-format=json 2>/dev/null |
	python3 -c '
import json, sys
for line in sys.stdin:
    try:
        m = json.loads(line)
    except ValueError:
        continue
    if m.get("reason") == "compiler-artifact" and m.get("executable"):
        print(m["executable"])
' | tail -1)
if [ -z "$bin" ]; then
	echo "disk_full testi derlenemedi: cargo test -p kentos-application --test disk_full --no-run" >&2
	exit 1
fi
dir=$(mktemp -d "${TMPDIR:-/tmp}/kentos-dolu-XXXXXX")
trap 'rmdir "$dir" 2>/dev/null || true' EXIT
unshare -rm sh -c "mount -t tmpfs -o size=${size}m kentos-dolu '$dir' && cd '$root' &&
	KENTOS_FULL_DIR='$dir' KENTOS_TEST_DB=required '$bin' --ignored --nocapture --test-threads=1"
