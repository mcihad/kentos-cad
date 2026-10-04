#!/usr/bin/env python3
"""Independent reference of Koordinat dönüştür's readings and writings (docs/adr/0167 §4).

Writes fixtures/crs/v1/convert.json from PROJ (pyproj) and the ADR's rules, no KentOS code: a point typed in one system
as the window reads it, taken to another the way PROJ takes it (the EPSG paths of crs_transform_cases.py), and written
as the window writes it.

- A projected system's point is two numbers, east first, named as the project's type names its axes (CBS: Y, X; CAD:
  X, Y), read as the Hesap windows read numbers (a comma for the point too); it is written with the project's length
  decimals by the display rule (numeric_display.py).
- A geographic system's point is the latitude, then the longitude, read by §4's grammar (decimal degrees, degrees and
  minutes, degrees minutes seconds with spaces or ° ′ ″ ' "; a closing hemisphere letter); a latitude beyond ±90° or
  a longitude beyond ±180° is no point. It is written in the user's notation: DMS with the seconds' 4 decimals, or
  DD with 7.
- How sure the values are: “±{accuracy} m, {operations}”, with “; resmî dönüşüm değil” when either side is ED50, or
  “kesin, yalnız projeksiyon” within one datum.
- The systems the project defines and its datum choices (docs/adr/0168 §9 3c) are taken the way crs_custom_cases.py's
  PROJ pipelines take them: the accuracy is the steps' sum (“doğruluğu bilinmiyor, …” when one is not written), what it
  rests on the operations' and the project's names, “resmî dönüşüm değil” when an EPSG operation of ED50 was used. A
  choice whose grid the device does not have leaves no value (`noGrid`), as does a datum of the project's with no way
  to WGS 84 (`noLink`).

The desktop (apps/desktop/src/calc/convert.rs) and the web (apps/web/src/ui/calc/convert.ts) must give exactly the
expected texts.
"""

import argparse
import json
import math
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import crs_custom_cases as custom  # noqa: E402  (the project's systems and choices as PROJ pipelines)
import crs_transform_cases as reference  # noqa: E402  (the EPSG paths, the angle grammar, DMS and DD)
import numeric_display  # noqa: E402  (the display rule)
from pyproj import Transformer  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "crs" / "v1" / "convert.json"

AXES = {"gis": ("Y", "X"), "cad": ("X", "Y")}


NUMBER = re.compile(r"^[-+]?(\d+(\.\d*)?|\.\d+)(e[-+]?\d+)?$", re.IGNORECASE)


def number(text):
    """A typed number as the Hesap windows read one: trimmed, its first comma a point, then the pattern above;
    none for anything else, an empty text too."""
    t = text.strip().replace(",", ".", 1)
    return float(t) if t and NUMBER.match(t) else None


def read(entry, a, b):
    """The point in the entry's own order (x east or longitude, y north or latitude), or why it is none."""
    if entry["kind"] == "geographic":
        lat, lon = reference.parse(a), reference.parse(b)
        if lat is None:
            return None, "a"
        if lon is None:
            return None, "b"
        if abs(lat) > 90 or abs(lon) > 180:
            return None, "range"
        return (lon, lat), None
    east, north = number(a), number(b)
    if east is None:
        return None, "a"
    if north is None:
        return None, "b"
    return (east, north), None


def write(entry, p, axes, decimals, notation):
    if entry["kind"] == "geographic":
        lon, lat = p
        if notation == "dd":
            return [["Enlem", reference.dd(lat, 7, True)], ["Boylam", reference.dd(lon, 7, False)]]
        return [["Enlem", reference.dms(lat, True, 4)], ["Boylam", reference.dms(lon, False, 4)]]
    east, north = AXES[axes]
    return [[east, numeric_display.shown(p[0], decimals)], [north, numeric_display.shown(p[1], decimals)]]


def accuracy(src, dst):
    _, acc, via = reference.transformer(src, dst)
    if not via:
        return "kesin, yalnız projeksiyon"
    unofficial = "; resmî dönüşüm değil" if "ED50" in (reference.datum(src), reference.datum(dst)) else ""
    shown = f"{acc:g}"
    return f"±{shown} m, {via}{unofficial}"


CASES = [
    # name, from, to, axes, notation, a, b
    ("TUREF TM30'dan ED50 TM30'a", 5254, 2320, "gis", "dms", "414120.512", "4540398.207"),
    ("aynısı, CAD'in eksenleriyle", 5254, 2320, "cad", "dms", "414120.512", "4540398.207"),
    ("TUREF TM30'dan WGS 84'e, DMS", 5254, 4326, "gis", "dms", "414120.512", "4540398.207"),
    ("TUREF TM30'dan WGS 84'e, DD", 5254, 4326, "gis", "dd", "414120.512", "4540398.207"),
    ("WGS 84'ten TUREF TM30'a, işaretli DMS", 4326, 5254, "gis", "dms", "40°59′38.0581″K", "28°58′45.7302″D"),
    ("WGS 84'ten TUREF TM30'a, boşluklu DMS", 4326, 5254, "gis", "dms", "40 59 38.0581", "28 58 45.7302"),
    ("WGS 84'ten TUREF TM30'a, ondalık derece", 4326, 5254, "gis", "dms", "40.9939050", "28.9793695"),
    ("ED50 UTM 36N'den TUREF TM36'ya", 23036, 5256, "gis", "dms", "486512.34", "4420187.52"),
    ("TUREF TM36'dan komşu dilim TM39'a", 5256, 5257, "gis", "dms", "486512.34", "4420187.52"),
    ("TUREF TM36'dan Pseudo-Mercator'a", 5256, 3857, "gis", "dms", "486512.34", "4420187.52"),
    ("TUREF TM36'dan TUREF coğrafiye, DD", 5256, 5252, "gis", "dd", "486512.34", "4420187.52"),
    ("enlem 90°'nin ötesinde", 4326, 5254, "gis", "dms", "95", "29"),
    ("boylam okunmuyor", 4326, 5254, "gis", "dms", "40", "yirmi dokuz"),
    ("virgüllü doğu", 5254, 2320, "gis", "dms", "414120,512", "4540398.207"),
    ("doğu okunmuyor", 5254, 2320, "gis", "dms", "414120.512 m", "4540398.207"),
    ("kuzey yok", 5254, 2320, "gis", "dms", "414120.512", ""),
]


# The project's systems and choices (ADR 0168): name, from, to, choices, axes, notation, a, b.
GRID_REGION = {"from": "ED50", "to": "TUREF", "name": "ED50 → TUREF: ızgara", "grid": {"id": "ab" * 32}}
CUSTOM_CASES = [
    ("şantiye sisteminden TUREF TM33'e: yalnız düzlem", custom.SITE, custom.TUREF_TM33, [], "gis", "dms", "1234.567",
     "-876.543"),
    ("TUREF TM33'ten şantiye sistemine, CAD'in eksenleriyle", custom.TUREF_TM33, custom.SITE, [], "cad", "dms",
     "512345.678", "4423456.789"),
    ("Bessel datumlu TM'den WGS 84'e, ondalık derece", custom.BESSEL_TM, custom.WGS84_GEO, [], "gis", "dd",
     "512345.678", "4423456.789"),
    ("TUREF TM30'dan ED50 TM30'a, projenin bölge parametreleriyle", custom.TUREF_TM30, custom.ED50_TM30,
     [custom.REGION], "gis", "dms", "412345.678", "4512345.678"),
    ("aynısı, doğruluğu yazılmamış parametrelerle", custom.TUREF_TM30, custom.ED50_TM30, [custom.REGION_UNKNOWN],
     "gis", "dms", "412345.678", "4512345.678"),
    ("aynısı, ızgarası bu cihazda olmayan seçimle", custom.TUREF_TM30, custom.ED50_TM30, [GRID_REGION], "gis", "dms",
     "412345.678", "4512345.678"),
    ("bağsız datumlu TM'den TUREF TM33'e", custom.LONE_TM, custom.TUREF_TM33, [], "gis", "dms", "512345.678",
     "4423456.789"),
]


def custom_accuracy(acc, via, unofficial):
    if not via:
        return "kesin, yalnız projeksiyon"
    sure = "doğruluğu bilinmiyor" if acc is None else f"±{acc:g} m"
    return f"{sure}, {via}" + ("; resmî dönüşüm değil" if unofficial else "")


def grid_between(src, dst, choices):
    """Whether the way between the two systems' registry datums is a grid choice (not loaded on the device)."""
    a, b = custom.datum_of(src), custom.datum_of(dst)
    return isinstance(a, str) and isinstance(b, str) and any(
        "grid" in c and {c["from"], c["to"]} == {a, b} for c in choices)


def build_custom():
    cases = []
    for name, src, dst, choices, axes, notation, a, b in CUSTOM_CASES:
        case = {"name": name, "from": src, "to": dst, "choices": choices, "axes": axes, "decimals": 3,
                "notation": notation, "input": [a, b]}
        p, why = read(src, a, b)
        if p is None:
            case["error"] = why
        elif grid_between(src, dst, choices):
            case["error"] = "noGrid"
        else:
            try:
                text, acc, via, unofficial = custom.pipeline(src, dst, choices)
            except custom.NoLink:
                case["error"] = "noLink"
            else:
                q = Transformer.from_pipeline(text).transform(*p)
                if not all(math.isfinite(v) for v in q):
                    case["error"] = "unreachable"
                else:
                    case["expect"] = {"values": write(dst, q, axes, 3, notation),
                                      "accuracy": custom_accuracy(acc, via, unofficial)}
        cases.append(case)
    return cases


def build():
    registry = json.loads(reference.REGISTRY.read_text(encoding="utf-8"))
    entries = {e["srid"]: e for e in registry["systems"] if e["kind"] != "local"}
    cases = []
    for name, a_srid, b_srid, axes, notation, a, b in CASES:
        src, dst = entries[a_srid], entries[b_srid]
        p, why = read(src, a, b)
        case = {"name": name, "from": reference.system(src), "to": reference.system(dst), "fromSrid": a_srid, "toSrid": b_srid,
                "axes": axes, "decimals": 3, "notation": notation, "input": [a, b]}
        if p is None:
            case["error"] = why
        else:
            t, _, _ = reference.transformer(src, dst)
            q = t.transform(*p)
            if not all(math.isfinite(v) for v in q):
                case["error"] = "unreachable"
            else:
                case["expect"] = {"values": write(dst, q, axes, 3, notation), "accuracy": accuracy(src, dst)}
        cases.append(case)
    cases += build_custom()
    return {"format": "kentos.crs-convert", "version": 1, "proj": __import__("pyproj").proj_version_str, "cases": cases}


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="fail when the fixture is not what PROJ and the rules give")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} PROJ'dan ve kurallardan çıkan değil; yeniden yazın: python3 {Path(__file__).relative_to(ROOT)}",
                  file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} PROJ'dan ve kurallardan çıkanla aynı.")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(json.loads(text)['cases'])} dönüştürme.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
