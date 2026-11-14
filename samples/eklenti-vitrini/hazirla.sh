#!/usr/bin/env bash
# Eklenti Vitrini'ni denemeye hazırlar: eklentiyi paketler, bir katalog klasörüne koyar ve kataloğun dizinini yazar.
#
#   samples/eklenti-vitrini/hazirla.sh HEDEF
#
# HEDEF/katalog/vitrin.kek ve HEDEF/katalog/index.json yazılır. KentOS GIS'i bu kataloğu görecek biçimde açın
# (KENTOS_EXTENSIONS_CATALOG=HEDEF/katalog), Eklentiler › Bulunabilir › Eklenti Vitrini › Gözden geçir ve kur.
# Kurulumu bilgisayarın kendi eklentilerinden ayrı tutmak için KENTOS_EXTENSIONS_DIR=HEDEF/eklentiler de verin.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
out=${1:?"kullanım: $0 HEDEF"}
mkdir -p "$out/katalog"
cargo build -q -p kentos-cli --manifest-path "$root/Cargo.toml"
"$root/target/debug/kentos" kek pack "$here/eklenti" -o "$out/katalog/vitrin.kek"
"$root/target/debug/kentos" kek index "$out/katalog" --name "Eklenti Vitrini"
cat <<SAY

Hazır. KentOS GIS'i örnek çizimle açmak için:
  KENTOS_EXTENSIONS_CATALOG=$out/katalog KENTOS_EXTENSIONS_DIR=$out/eklentiler \\
    cargo run -p kentos-desktop --bin kentos-cad -- --modul cbs $here/vitrin.kcad
SAY
