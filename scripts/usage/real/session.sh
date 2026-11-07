#!/bin/bash
# Real window: kentos-cad in an invisible X screen (Xvfb), a scenario played with real mouse and keyboard
# events (XTEST), pictures taken from that screen (scripts/usage/real/README.md, docs/adr/3000).
#   scripts/usage/real/session.sh SCENARIO.txt OUT_DIR [app arguments, e.g. --modul mimari]
# The app runs in a fresh home of its own with a copy of the owner's settings: the owner's ~/.config, recovery
# copies and recent files are never written.
set -u
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
HERE=$ROOT/scripts/usage/real
SCENARIO=$(realpath "$1"); OUT=$(realpath -m "$2"); shift 2
mkdir -p "$OUT"
SCREEN=${KENTOS_REAL_DISPLAY:-:91}
# KENTOS_REAL_SIZE: the screen, 1440x900 unless given (1100x650 for the narrow acceptance, plan §6).
Xvfb "$SCREEN" -screen 0 "${KENTOS_REAL_SIZE:-1440x900}x24" -nolisten tcp > "$OUT/xvfb.log" 2>&1 &
XVFB=$!
sleep 1
export DISPLAY=$SCREEN
unset WAYLAND_DISPLAY
# Xvfb has no DRI3: the software Vulkan (lavapipe) presents without it.
export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json WGPU_BACKEND=vulkan
HOME_DIR=$(mktemp -d)
mkdir -p "$HOME_DIR/.config/kentos-cad" "$HOME_DIR/.local/share" "$HOME_DIR/.cache"
SETTINGS=${KENTOS_REAL_SETTINGS:-$HOME/.config/kentos-cad}
for f in ayarlar.json yerlesim-mimari.json pencereler.json; do
  [ -f "$SETTINGS/$f" ] && cp "$SETTINGS/$f" "$HOME_DIR/.config/kentos-cad/"
done
export XDG_CONFIG_HOME=$HOME_DIR/.config XDG_DATA_HOME=$HOME_DIR/.local/share XDG_CACHE_HOME=$HOME_DIR/.cache
cd "$ROOT"
"${KENTOS_BIN:-$ROOT/target/debug/kentos-cad}" "$@" > "$OUT/app.log" 2>&1 &
APP=$!
sleep "${KENTOS_REAL_START:-12}"
uv run --no-project --quiet --with python-xlib==0.33 --with pillow==13.0.0 python3 "$HERE/driver.py" "$SCENARIO" "$OUT" \
  > "$OUT/driver.log" 2>&1
STATUS=$?
echo "driver exit=$STATUS" >> "$OUT/driver.log"
kill $APP 2>/dev/null; sleep 1; kill $XVFB 2>/dev/null
rm -rf "$HOME_DIR"
python3 "$HERE/count.py" "$SCENARIO" | tee "$OUT/sayac.txt"
exit $STATUS
