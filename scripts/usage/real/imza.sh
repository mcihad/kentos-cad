#!/bin/bash
# Real window: KentOS İmza's PIN window (kentos-imza-agent) and its local service with the software test card, in an
# invisible X screen, driven with real mouse and keyboard events (XTEST; docs/adr/8000, docs/adr/9002 §6).
#   scripts/usage/real/imza.sh OUT_DIR [acik|koyu]
# Needs, built beforehand under the heavy lock:
#   cargo build -p kentos-imza-service --features mock-token
#   cargo build -p kentos-imza-agent
#   cargo build -p kentos-imza-security --features issue --example deneme_zarfi
# Everything runs in a fresh home of its own (all four XDG folders, the runtime folder, no session bus) on its own
# ports, so neither the owner's settings nor another e-signature install on this machine is read or touched.
set -u
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
HERE=$ROOT/scripts/usage/real
OUT=$(realpath -m "$1"); TEMA=${2:-acik}
mkdir -p "$OUT"
# One theme's run writes its own files; a second theme into the same folder keeps the first one's.
AKIS=$OUT/akis-$TEMA.txt
: > "$AKIS"
: > "$OUT/driver-$TEMA.log"
SCREEN=${KENTOS_REAL_DISPLAY:-:93}
PORT=${KENTOS_IMZA_REAL_PORT:-32177}
BIN=$ROOT/target/debug
for b in kentos-imza-service kentos-imza-agent examples/deneme_zarfi; do
  [ -x "$BIN/$b" ] || { echo "eksik: $BIN/$b (önce derleyin, betiğin başına bakın)"; exit 2; }
done

Xvfb "$SCREEN" -screen 0 "${KENTOS_REAL_SIZE:-1440x900}x24" -nolisten tcp > "$OUT/xvfb.log" 2>&1 &
XVFB=$!
sleep 1
export DISPLAY=$SCREEN
unset WAYLAND_DISPLAY
export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json WGPU_BACKEND=vulkan
HOME_DIR=$(mktemp -d)
mkdir -p "$HOME_DIR/.config" "$HOME_DIR/.local/share" "$HOME_DIR/.local/state" "$HOME_DIR/.cache" "$HOME_DIR/run"
chmod 700 "$HOME_DIR/run"
export HOME=$HOME_DIR XDG_CONFIG_HOME=$HOME_DIR/.config XDG_DATA_HOME=$HOME_DIR/.local/share \
  XDG_STATE_HOME=$HOME_DIR/.local/state XDG_CACHE_HOME=$HOME_DIR/.cache XDG_RUNTIME_DIR=$HOME_DIR/run
export DBUS_SESSION_BUS_ADDRESS="unix:path=$HOME_DIR/no-session-bus"
# The service reads only this run's (empty) config file, never another install's.
export KENTOS_IMZA_CONFIG=$HOME_DIR/imza.env
: > "$KENTOS_IMZA_CONFIG"

# The server's part: a key pair; the service trusts its public half.
ACIK=$("$BIN/examples/deneme_zarfi" anahtar "$HOME_DIR/ozel.txt")
URL=http://127.0.0.1:$PORT
KENTOS_IMZA_PORT=$PORT KENTOS_IMZA_TRUST_KEY=$ACIK KENTOS_IMZA_DENEME_KARTI=1 \
  "$BIN/kentos-imza-service" > "$OUT/servis-$TEMA.log" 2>&1 &
SERVIS=$!
KENTOS_IMZA_URL=$URL KENTOS_IMZA_TEMA=$TEMA "$BIN/kentos-imza-agent" > "$OUT/ajan-$TEMA.log" 2>&1 &
AJAN=$!

surucu() {  # one scenario file
  uv run --no-project --quiet --with python-xlib==0.33 --with pillow==13.0.0 python3 "$HERE/driver.py" "$1" "$OUT" \
    >> "$OUT/driver-$TEMA.log" 2>&1 || echo "driver: $1 başarısız" >> "$OUT/driver-$TEMA.log"
}
istek() {  # a signing request as the portal's page sends it: create, open, sign without a PIN (native)
  "$BIN/examples/deneme_zarfi" zarf "$HOME_DIR/ozel.txt" "$1" > "$HOME_DIR/$1.json"
  curl -s -X POST "$URL/api/v1/signing/batches" -H 'Content-Type: application/json' -H 'Origin: http://127.0.0.1:8797' \
    --data @"$HOME_DIR/$1.json" > "$OUT/$1-olustur.json"
  curl -s -X POST "$URL/api/v1/signing/batches/$1-batch/open" > /dev/null
  curl -s -X POST "$URL/api/v1/signing/batches/$1-batch/sign" -H 'Content-Type: application/json' -d '{}' \
    > "$OUT/$1-imzala.json"
}
durum() { curl -s "$URL/api/v1/signing/batches/$1-batch" | python3 -c 'import sys,json; print(json.load(sys.stdin)["status"])'; }

# The agent beats every 5 s; /sign refuses until the service has seen it.
for _ in $(seq 1 40); do
  curl -s "$URL/api/v1/health" | grep -q '"interactiveAgent":true' && break
  sleep 0.5
done
curl -s "$URL/api/v1/health" > "$OUT/health.json"
curl -s "$URL/api/v1/info" > "$OUT/info.json"

# The window where a 820×720 window sits, on a 1440×900 screen with no window manager.
W="fit KentOS İmza 310 90 820 720"
S=$HOME_DIR/sahne.txt
printf '%s\nwait 800\nshot 01-bekliyor-%s\n' "$W" "$TEMA" > "$S"; surucu "$S"

# 1) A request comes: the window comes to the front; a wrong PIN, then the right one; the card signs.
istek tx-1
# No click on the PIN field: it takes the focus by itself when the cards are read and again after a refused PIN.
printf '%s\n' "wait 2500" "$W" "wait 600" "shot 02-pin-istendi-$TEMA" "type 0000" "key Return" \
  "wait 1500" "shot 03-yanlis-pin-$TEMA" "type 1234" "key Return" "wait 1600" "shot 04-imzaliyor-$TEMA" \
  "wait 2600" "shot 05-teslim-bekleniyor-$TEMA" > "$S"; surucu "$S"
curl -s "$URL/api/v1/signing/batches/tx-1-batch/result" > "$OUT/tx-1-sonuc.json"
echo "tx-1 imzadan sonra: $(durum tx-1)" >> "$AKIS"
# 2) The portal's answer: accepted. The window says it and hides after 3 s.
curl -s -X POST "$URL/api/v1/signing/batches/tx-1-batch/acknowledge" -H 'Content-Type: application/json' \
  -d '{"accepted":true,"reason":null}' > "$OUT/tx-1-teslim.json"
printf '%s\n' "wait 700" "shot 06-tamamlandi-$TEMA" "wait 3500" "shot 07-gizlendi-$TEMA" > "$S"; surucu "$S"
echo "tx-1 teslimden sonra: $(durum tx-1)" >> "$AKIS"

# 3) Another request, and the user says Vazgeç: the request is cancelled at once (no 180 s wait).
istek tx-2
printf '%s\n' "wait 2500" "$W" "wait 600" "shot 08-ikinci-istek-$TEMA" "click 1008 778" "wait 1200" \
  "shot 09-vazgecildi-$TEMA" > "$S"; surucu "$S"
echo "tx-2 vazgeçince: $(durum tx-2)" >> "$AKIS"

kill $AJAN $SERVIS 2>/dev/null; sleep 1; kill $XVFB 2>/dev/null
rm -rf "$HOME_DIR"
cat "$AKIS"
