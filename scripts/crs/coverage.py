#!/usr/bin/env python3
"""KentOS's own coordinate systems, written out (docs/adr/0630 §4).

    python3 scripts/crs/coverage.py           # writes docs/crs/coverage.md
    python3 scripts/crs/coverage.py --check   # fails when the page is not what the registry and the core say

KentOS transforms coordinates with its own Rust engine (geometry-core's `crs`), not PROJ: no C library rides with the
desktop, the headless host, Python or MCP. This page says what that engine covers, from the registry every program
reads (fixtures/crs/v1/registry.json) and the core's kinds of system (crates/shared/geometry-core/src/crs.rs, read
here so a new kind fails the check until the page says it), and what it refuses by name. The registry's systems are
checked against PROJ elsewhere (crs_transform_cases.py, crs_sweep.py, map_warp_cases.py).
"""

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
REGISTRY = ROOT / "fixtures" / "crs" / "v1" / "registry.json"
CORE = ROOT / "crates" / "shared" / "geometry-core" / "src" / "crs.rs"
DATUMS = ROOT / "crates" / "shared" / "geometry-core" / "src" / "crs" / "datum.rs"
OUT = ROOT / "docs" / "crs" / "coverage.md"

# The core's kinds of system and what each is, in words; the check fails when the core has another.
KINDS = {
    "Geographic": "Coğrafi: boylam ve enlem, derece; datumun elipsoidinde.",
    "Tm": "Transverse Mercator: her başlangıç meridyeni, ölçek çarpanı, sabit doğu ve kuzey, başlangıç enlemi "
          "(3°, 6° dilimleri ve UTM bunun özel hâlleri).",
    "Mercator": "Web Mercator (Pseudo-Mercator, EPSG:3857): küresel Mercator, WGS 84 üstünde.",
    "Local": "Yerel: bir taban sistemin düzlemine ortak noktalardan oturtulmuş dört parametreli (benzerlik) ya da afin "
             "düzlem (ADR 0168).",
}
DATUM_WORDS = {
    "Turef": "TUREF (ITRF96, GRS80)",
    "Ed50": "ED50 (International 1924)",
    "Wgs84": "WGS 84",
    "Custom": "Projenin kendi datumu: elipsoidi ve WGS 84'e 7 parametre (konum vektörü ya da koordinat çerçevesi "
              "dönüklüğü), isteğe bağlı NTv2 ızgarası (ADR 0168)",
}
REFUSED = [
    "Başka izdüşüm yöntemleri: Lambert konik (LCC), Albers, stereografik, eğik Mercator, Cassini, Krovak, kutupsal "
    "izdüşümler. Böyle bir sistemin dosyası “koordinat sistemi tanınmadı” denerek reddedilir ya da sistemsiz okunur "
    "ve sistem kullanıcıya sorulur; benzer bir sistem tahmin edilmez.",
    "KentOS'un kaydında olmayan EPSG kodları (`crs_unknown`); WKT ya da PROJ metni bu yöntemlerin ve datumların "
    "dışındaysa okunamaz denir (`crs_text_unreadable`).",
    "Düşey datum, geoit ve epok dönüşümleri: kayıtlarıyla taşınır, dönüşümleri adıyla reddedilir (ADR 0519).",
    "Genel PROJ bağdaştırıcısı yok: PROJ bir C kütüphanesidir ve sahibin onayı olmadan bağımlılık olarak eklenmez. "
    "Gerekirse ayrı bir karar konusudur; o zamana dek kapsam bu sayfadır.",
]


def kinds_in_core():
    text = CORE.read_text(encoding="utf-8")
    m = re.search(r"pub enum System \{(.*?)\n\}", text, re.S)
    if not m:
        raise SystemExit("crs.rs: the System enum is not where it was")
    return re.findall(r"^    ([A-Z][A-Za-z]*) \{", m.group(1), re.M)


def datums_in_core():
    text = DATUMS.read_text(encoding="utf-8")
    m = re.search(r"pub enum Datum \{(.*?)\n\}", text, re.S)
    return re.findall(r"^    ([A-Z][A-Za-z0-9]*)", m.group(1), re.M)


def page():
    reg = json.loads(REGISTRY.read_text(encoding="utf-8"))
    kinds, datums = kinds_in_core(), datums_in_core()
    if set(kinds) != set(KINDS):
        raise SystemExit(f"the core's kinds of system are {kinds}: say the new one in KINDS")
    if set(datums) != set(DATUM_WORDS):
        raise SystemExit(f"the core's datums are {datums}: say the new one in DATUM_WORDS")
    lines = [
        "# KentOS'un koordinat sistemleri",
        "",
        "Bu sayfa `scripts/crs/coverage.py` ile kayıttan ve çekirdekten üretilir; elle düzenlemeyin "
        "([ADR 0630](../adr/0630-map-system-pictures-and-crs-coverage.md) §4).",
        "",
        "KentOS koordinatları kendi Rust motoruyla (geometry-core'un `crs`'i) dönüştürür; PROJ kullanmaz. Masaüstü, "
        "başsız sunucu, Python ve MCP aynı motoru ve aynı kaydı kullanır. Kayıttaki sistemler PROJ'la bağımsız olarak "
        "denetlenir (`crs_transform_cases.py`, `crs_sweep.py`, `map_warp_cases.py`).",
        "",
        "## Sistem türleri",
        "",
    ]
    lines += [f"- **{k}**: {KINDS[k]}" for k in kinds]
    lines += ["", "## Datumlar", ""]
    lines += [f"- {DATUM_WORDS[d]}" for d in datums]
    lines += [
        "",
        "Datumlar arası yollar EPSG'nin adlı dönüşümleridir (TUREF–WGS 84: EPSG:5261; ED50–WGS 84: EPSG:1784; "
        "ED50–TUREF: EPSG:1783 + EPSG:5260); projenin datum seçimleri bunların yerine geçebilir.",
        "",
        f"## Kayıttaki sistemler ({len(reg['systems'])})",
        "",
        "| EPSG | Ad | Tür | Datum | Yöntem | Eksenler | Kullanım alanı |",
        "|---|---|---|---|---|---|---|",
    ]
    for e in reg["systems"]:
        method = e.get("projection", e.get("method")) or "—"
        lines.append(f"| {e['srid']} | {e['name']} | {e['kind']} | {e['datum']} | {method} | {e.get('axisOrder', '—')} "
                     f"| {e.get('area', '—')} |")
    lines += ["", "## Kapsam dışı: açıkça reddedilenler", ""]
    lines += [f"- {r}" for r in REFUSED]
    lines += [
        "",
        "## Harita sistemi",
        "",
        "Projenin harita sistemi (ADR 0583 §3) bu kayıttaki her sistem olabilir. Vektörler stil motorunda, rasterler, "
        "resimler ve nokta bulutları çizerken ağla (ADR 0630) harita sistemine taşınır; ölçü ve yazma projenin "
        "sisteminde kalır.",
        "",
    ]
    return "\n".join(lines)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    a = ap.parse_args()
    text = page()
    if a.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            raise SystemExit(f"{OUT.relative_to(ROOT)} is stale: run scripts/crs/coverage.py and read the diff")
        print(f"{OUT.relative_to(ROOT)} güncel")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı")


if __name__ == "__main__":
    sys.exit(main())
