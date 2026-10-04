#!/bin/bash
# Rebuild times after real edits (docs/adr/0170): a private function is added to a file, the build is timed, the edit is
# taken back and built again untimed, so every step starts warm. Run from the repository root, one heavy process at a
# time (ADR 0001):
#
#     nice -n 10 bash scripts/perf/rebuild-times.sh [rust|wasm|all]
#
# rust: the native test binaries and the desktop app after an edit to geometry-core, formats, interaction and the desktop;
# wasm: the web's WASM packages after a formats and a geometry-core edit, in the shipped profile (`--release`) and in the
# dev one, each in a block of its own (a package's stamp holds one profile at a time).
set -u
cd "$(git rev-parse --show-toplevel)" || exit 1
what=${1:-all}

probe() { printf '\n#[allow(dead_code)]\nfn bench_probe_%s() -> u32 {\n    7\n}\n' "$(date +%s%N)" >> "$1"; }

step() {
  local name=$1 file=$2
  shift 2
  if ! git diff --quiet -- "$file"; then
    echo "$file has changes of its own; it is not edited." >&2
    return
  fi
  probe "$file"
  local s rc e
  s=$(date +%s)
  "$@" > /dev/null 2>&1
  rc=$?
  e=$(($(date +%s) - s))
  git checkout -q -- "$file"
  "$@" > /dev/null 2>&1
  echo "$name: $e s (exit $rc)"
}

if [ "$what" = rust ] || [ "$what" = all ]; then
  cargo test -q -p kentos-formats -p kentos-interaction -p kentos-desktop --no-run > /dev/null 2>&1
  cargo build -q -p kentos-desktop > /dev/null 2>&1
  step "geometry-core → interaction testleri" crates/shared/geometry-core/src/display.rs cargo test -q -p kentos-interaction --no-run
  step "formats → formats testleri" crates/shared/formats/src/field/write.rs cargo test -q -p kentos-formats --no-run
  step "interaction → masaüstü testleri" crates/native/interaction/src/gnss.rs cargo test -q -p kentos-desktop --no-run
  step "masaüstü → masaüstü testleri" apps/desktop/src/exchange/gnss_import.rs cargo test -q -p kentos-desktop --no-run
  step "masaüstü → masaüstü uygulaması" apps/desktop/src/exchange/gnss_import.rs cargo build -q -p kentos-desktop
fi

if [ "$what" = wasm ] || [ "$what" = all ]; then
  for mode in --release ""; do
    label=$([ -n "$mode" ] && echo "wasm (gönderilen)" || echo "wasm-dev")
    node scripts/wasm/ensure.mjs $mode > /dev/null 2>&1
    step "$label | formats (3 paket)" crates/shared/formats/src/field/write.rs node scripts/wasm/ensure.mjs $mode
    step "$label | geometry-core (6 paket)" crates/shared/geometry-core/src/display.rs node scripts/wasm/ensure.mjs $mode
  done
  # Leave the dev packages in place for the dev server and the tests.
  node scripts/wasm/ensure.mjs > /dev/null 2>&1
fi
