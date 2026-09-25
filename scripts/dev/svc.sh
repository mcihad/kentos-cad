#!/usr/bin/env bash
# Background services and the heavy-work lock for the root Makefile.
#
# A service runs in its own session and process group (setsid), with its PID
# in .run/<name>.pid and its output in .run/<name>.log. `stop` ends the whole
# group (pnpm → node → vite → esbuild, or kentosd): INT as Ctrl+C would, TERM
# after 3 s, KILL after 10 s, so nothing is left listening on a port.
#
# `heavy` runs a build, test or measurement under the shared lock
# /tmp/kentos-heavy.lock, one at a time on this machine (CLAUDE.md §2): a
# second one waits instead of starving the desktop.
#
#   scripts/dev/svc.sh start <name> <command…>   start in the background
#   scripts/dev/svc.sh stop <name…>              stop (no name: every service)
#   scripts/dev/svc.sh running <name>            exit 0 when it runs
#   scripts/dev/svc.sh wait <name> <url> [s]     wait until the URL answers
#   scripts/dev/svc.sh status                    what runs, since when
#   scripts/dev/svc.sh heavy <command…>          run under the lock
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUN="$ROOT/.run"
LOCK="${KENTOS_HEAVY_LOCK:-/tmp/kentos-heavy.lock}"
mkdir -p "$RUN"

pid_of() { cat "$RUN/$1.pid" 2>/dev/null; }

# The group outlives its leader (vite may still be closing), so ask for the group.
running() {
  local pid
  pid=$(pid_of "$1") || return 1
  [ -n "$pid" ] && pgrep -g "$pid" >/dev/null 2>&1
}

start() {
  local name=$1
  shift
  if running "$name"; then
    echo "• $name zaten çalışıyor (PID $(pid_of "$name")); yeniden başlatmak için: make stop-$name"
    return 0
  fi
  {
    echo "── $(date '+%F %T') başlatıldı: $*"
  } >"$RUN/$name.log"
  # Not a group leader here (no job control in a script), so setsid does not
  # fork: the PID below is the new group's id and `exec` keeps it.
  setsid bash -c 'exec "$@"' _ "$@" >>"$RUN/$name.log" 2>&1 </dev/null &
  echo $! >"$RUN/$name.pid"
  echo "• $name başladı (PID $!, günlük .run/$name.log)"
}

stop_one() {
  local name=$1 pid
  pid=$(pid_of "$name") || { echo "• $name çalışmıyor"; return 0; }
  if ! pgrep -g "$pid" >/dev/null 2>&1; then
    rm -f "$RUN/$name.pid"
    echo "• $name çalışmıyordu"
    return 0
  fi
  # INT first, as Ctrl+C would: Vite's dev server leaves at once on INT but
  # hangs in its graceful close on TERM. TERM after 3 s, KILL after 10 s.
  kill -INT -- "-$pid" 2>/dev/null || true
  for i in $(seq 1 50); do
    pgrep -g "$pid" >/dev/null 2>&1 || break
    [ "$i" -eq 15 ] && kill -TERM -- "-$pid" 2>/dev/null
    sleep 0.2
  done
  if pgrep -g "$pid" >/dev/null 2>&1; then
    kill -KILL -- "-$pid" 2>/dev/null || true
    echo "• $name 10 sn içinde kapanmadı, zorla durduruldu"
  else
    echo "• $name durduruldu"
  fi
  rm -f "$RUN/$name.pid"
}

stop() {
  local names=("$@") f
  if [ ${#names[@]} -eq 0 ]; then
    for f in "$RUN"/*.pid; do
      [ -e "$f" ] && names+=("$(basename "$f" .pid)")
    done
  fi
  [ ${#names[@]} -eq 0 ] && { echo "• çalışan servis yok"; return 0; }
  for n in "${names[@]}"; do stop_one "$n"; done
}

wait_url() {
  local name=$1 url=$2 seconds=${3:-60} i
  for i in $(seq 1 $((seconds * 5))); do
    if curl -fsS -o /dev/null --max-time 2 "$url" 2>/dev/null; then
      echo "• $name hazır: $url"
      return 0
    fi
    if ! running "$name"; then
      echo "• $name başlarken kapandı; günlüğün sonu (.run/$name.log):" >&2
      tail -n 25 "$RUN/$name.log" >&2
      return 1
    fi
    [ "$i" -eq 25 ] && echo "  $name bekleniyor… (ilk derleme sürebilir; günlük .run/$name.log)"
    sleep 0.2
  done
  echo "• $name $seconds sn içinde yanıt vermedi; günlük .run/$name.log" >&2
  return 1
}

status() {
  local f name pid any=0
  for f in "$RUN"/*.pid; do
    [ -e "$f" ] || continue
    any=1
    name=$(basename "$f" .pid)
    pid=$(cat "$f")
    if pgrep -g "$pid" >/dev/null 2>&1; then
      echo "  ● $name  çalışıyor  PID $pid, $(ps -o etime= -p "$pid" 2>/dev/null | tr -d ' ' || echo '?') süredir"
    else
      echo "  ○ $name  durmuş (eski PID $pid; make stop temizler)"
    fi
  done
  [ "$any" -eq 1 ] || echo "  (çalışan servis yok)"
}

heavy() {
  if ! command -v flock >/dev/null 2>&1; then
    exec "$@"
  fi
  exec 9>"$LOCK"
  if ! flock -n 9; then
    echo "• başka bir ağır iş sürüyor (kilit $LOCK); bitmesi bekleniyor…"
    flock -w 1800 9 || { echo "• kilit 30 dk içinde boşalmadı; vazgeçildi" >&2; exit 1; }
  fi
  nice -n 10 "$@"
}

cmd=${1:-}
shift || true
case "$cmd" in
  start) start "$@" ;;
  stop) stop "$@" ;;
  running) running "$1" ;;
  wait) wait_url "$@" ;;
  status) status ;;
  heavy) heavy "$@" ;;
  *)
    sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//'
    exit 2
    ;;
esac
