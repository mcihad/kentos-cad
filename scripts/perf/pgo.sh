#!/usr/bin/env bash
# Sample-based profile-guided optimisation of kentos-cad (docs/adr/8055 §3): the release program built once with line
# tables and symbols, its CPU's branch records (LBR) taken while it opens and draws drawings and plays the usage
# traces, turned into an LLVM sample profile, and the release built again with `-Cprofile-sample-use` (Rust 1.100).
# Every step writes under .run/pgo and its own target folder: the release program in target/release is not touched.
#
#   scripts/perf/pgo.sh build              the profiling build (target/pgo)
#   scripts/perf/pgo.sh record FILE…       perf record of `snapshot` of each FILE and of the usage traces
#   scripts/perf/pgo.sh profile            llvm-profgen per record, merged into .run/pgo/kentos-cad.prof
#   scripts/perf/pgo.sh use                the release built with the profile (target/pgo-use)
#   scripts/perf/pgo.sh measure BIN FILE…  the same work with BIN, RUNS times each (median wall seconds)
#
# Needs perf with user profiling allowed (kernel.perf_event_paranoid ≤ 2) and llvm-profgen / llvm-profdata
# (LLVM's; PGO_LLVM_SUFFIX=-21 picks /usr/bin/llvm-profgen-21). Run under scripts/dev/svc.sh heavy.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT="$ROOT/.run/pgo"
SUFFIX="${PGO_LLVM_SUFFIX:--21}"
RUNS="${RUNS:-5}"
# The usage traces played as work: drawing, snapping, blocks, parcels, roads (fixtures/interaction/v1/usage-*.json).
TRACES="${PGO_TRACES:-usage-parcel usage-blocks usage-snap usage-junctions usage-line-parts usage-semantic-tools}"
mkdir -p "$OUT"
export CARGO_INCREMENTAL=0

# Line tables map the samples back to the code; the symbols stay for llvm-profgen. A package's `strip` in Cargo.toml
# is not overridden by an environment variable, a `--config` profile is (config files' profiles come first).
PROFILING=(--config 'profile.release.debug="line-tables-only"' --config 'profile.release.package.kentos-desktop.strip="none"')

# The work: each drawing opened and drawn without a window, then each usage trace played.
work() {
  local bin=$1
  shift
  local i=0
  for file in "$@"; do
    i=$((i + 1))
    KENTOS_SNAPSHOT_BACKEND=wgpu "$bin" snapshot "$OUT/shot-$i.png" "$file" >/dev/null 2>&1
  done
  for trace in $TRACES; do
    "$bin" kullan "$trace" >/dev/null 2>&1
  done
}

case "${1:-}" in
  build)
    cargo build --release -p kentos-desktop --target-dir "$ROOT/target/pgo" "${PROFILING[@]}"
    ;;
  record)
    shift
    [ "$(cat /proc/sys/kernel/perf_event_paranoid)" -le 2 ] ||
      { echo "perf kapalı: sudo sysctl -w kernel.perf_event_paranoid=2 (docs/adr/8055 §3)" >&2; exit 2; }
    bin="$ROOT/target/pgo/release/kentos-cad"
    rm -f "$OUT"/*.data
    i=0
    for file in "$@"; do
      i=$((i + 1))
      KENTOS_SNAPSHOT_BACKEND=wgpu perf record -q -e cycles:u -j any,u -o "$OUT/snapshot-$i.data" -- \
        "$bin" snapshot "$OUT/shot-$i.png" "$file" >/dev/null 2>&1
    done
    for trace in $TRACES; do
      perf record -q -e cycles:u -j any,u -o "$OUT/$trace.data" -- "$bin" kullan "$trace" >/dev/null 2>&1
    done
    ls -la "$OUT"/*.data
    ;;
  profile)
    bin="$ROOT/target/pgo/release/kentos-cad"
    rm -f "$OUT"/*.prof
    for data in "$OUT"/*.data; do
      "llvm-profgen$SUFFIX" --binary="$bin" --perfdata="$data" --output="${data%.data}.prof"
    done
    mapfile -t profiles < <(find "$OUT" -maxdepth 1 -name '*.prof' ! -name kentos-cad.prof | sort)
    "llvm-profdata$SUFFIX" merge --sample --extbinary -o "$OUT/kentos-cad.prof" "${profiles[@]}"
    ls -la "$OUT/kentos-cad.prof"
    ;;
  use)
    [ -s "$OUT/kentos-cad.prof" ] || { echo "önce: pgo.sh profile" >&2; exit 2; }
    # The profile's loader reads the code's lines: line tables in the build, stripped from the program as in release.
    RUSTFLAGS="-Cprofile-sample-use=$OUT/kentos-cad.prof" \
      cargo build --release -p kentos-desktop --target-dir "$ROOT/target/pgo-use" --config 'profile.release.debug="line-tables-only"'
    ;;
  measure)
    shift
    bin=$1
    shift
    for _ in $(seq "$RUNS"); do
      s=$(date +%s.%N)
      work "$bin" "$@"
      e=$(date +%s.%N)
      LC_NUMERIC=C printf '%.2f\n' "$(echo "$e - $s" | bc)"
    done | sort -n | awk '{a[NR]=$1} END {printf "median %.2f s (%d runs; min %.2f, max %.2f)\n", a[int((NR+1)/2)], NR, a[1], a[NR]}'
    ;;
  *)
    sed -n '2,13p' "$0"
    exit 2
    ;;
esac
