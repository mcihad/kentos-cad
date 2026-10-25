"""Stil alışverişinin profilinden (catalog/v1/style-exchange.json) okunur belge (docs/style/exchange.md), docs/adr/0635 §8.

Profilin doğruluğunu biçimlerin kendi testleri denetler (kentos_formats::style_exchange::check); bu betik yalnız onu
tabloya döker. `--check` belge güncel değilse düşer, yoksa yazar.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PROFILE = ROOT / "catalog/v1/style-exchange.json"
OUT = ROOT / "docs/style/exchange.md"
FORMATS = [("sld", "SLD 1.1"), ("qml", "QML (QGIS)"), ("maplibre", "MapLibre")]
WORDS = {"both": "İki yönde", "write": "Yazılır, başka türle geri gelir", "lost": "Taşınmaz"}
GEOMETRY = {"point": "nokta", "line": "çizgi", "area": "alan"}
SYMBOL = {"fill": "dolgu", "line": "çizgi", "marker": "işaret"}


def cell(row, f):
    text = WORDS[row[f]]
    note = row.get("notes", {}).get(f)
    return f"{text} ({note})" if note else text


def table(rows, kind_word, extra):
    lines = [f"| {kind_word} | {extra[0]} | " + " | ".join(n for _, n in FORMATS) + " |",
             "|---|---|" + "---|" * len(FORMATS)]
    for r in rows:
        lines.append(f"| {r['name']} (`{r['kind']}`) | {extra[1](r)} | " + " | ".join(cell(r, f) for f, _ in FORMATS)
                     + " |")
    return "\n".join(lines)


def render():
    p = json.loads(PROFILE.read_text(encoding="utf-8"))
    parts = [
        "# Stil alışverişinin profili",
        "",
        "Bu belge `catalog/v1/style-exchange.json`'dan `scripts/style/exchange.py` ile üretilir; elle düzenlemeyin. "
        "Her satırın örneği kendi biçiminin testinde yazılıp geri okunur, profil kodla aynı değilse test düşer "
        "([ADR 0635](../adr/0635-style-fields-and-exchange.md) §8). Taşınmayan her şey içe ve dışa aktarmanın "
        "raporunda adıyla söylenir.",
        "",
        "## İşleyiciler",
        "",
        table(p["renderers"], "İşleyici", ("Örnek", lambda r: GEOMETRY.get(r["geometry"], r["geometry"]))),
        "",
        "## Sembol katmanları",
        "",
        table(p["layers"], "Sembol katmanı", ("Simge", lambda r: SYMBOL.get(r["symbol"], r["symbol"]))),
        "",
    ]
    return "\n".join(parts)


def main():
    text = render()
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print("docs/style/exchange.md güncel değil: python3 scripts/style/exchange.py", file=sys.stderr)
            return 1
        print("ok")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
