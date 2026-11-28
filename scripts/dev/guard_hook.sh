#!/usr/bin/env bash
# Claude Code'un Stop hook'u (docs/adr/9400 §4): oturum dururken derleme istemeyen bekçileri (`make guard`) koşar.
# Kırmızıysa durmayı bir kez engeller ve bekçilerin çıktısını oturuma geri verir.
#
# - HEAD ve çalışma ağacı son yeşil koşudan beri değişmediyse koşmaz (.run/guard.ok).
# - Hook bu duruşu zaten bir kez engellediyse (`stop_hook_active`) yeniden engellemez: döngü olmaz.
# - Hook'un kendi hatası oturumu durdurmaz: her yolda 0 ile çıkar, engeli JSON ile söyler.
set -u

root="$(cd "$(dirname "$0")/../.." && pwd)" || exit 0
cd "$root" || exit 0

input="$(cat 2>/dev/null || true)"
case "$input" in
*'"stop_hook_active":true'* | *'"stop_hook_active": true'*) exit 0 ;;
esac

key="$({
  git rev-parse HEAD
  git status --porcelain
  git diff HEAD
} 2>/dev/null | shasum | cut -d' ' -f1)"
mkdir -p .run
if [ -f .run/guard.ok ] && [ "$(cat .run/guard.ok)" = "$key" ]; then
  exit 0
fi

if out="$(make guard 2>&1)"; then
  printf '%s' "$key" >.run/guard.ok
  exit 0
fi

python3 - "$out" <<'PY'
import json
import sys

text = sys.argv[1]
print(json.dumps({
    "decision": "block",
    "reason": "make guard kırmızı. Düzeltin ya da nedenini kullanıcıya söyleyin:\n" + text[-6000:],
}, ensure_ascii=False))
PY
exit 0
