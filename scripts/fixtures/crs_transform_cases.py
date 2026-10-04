#!/usr/bin/env python3
"""Independent reference of the coordinate transforms (docs/adr/0167).

Writes fixtures/geodesy/v1/transform.json from PROJ (pyproj) and the ADR's
rules, no KentOS code: points between the registry's systems
(fixtures/crs/v1/registry.json) the way PROJ transforms them, each datum
shift by the EPSG operation the ADR names (PROJ's default for Türkiye):

- TUREF and WGS 84: TUREF to WGS 84 (1), EPSG:5261, a null transformation, 1 m;
- ED50 and WGS 84: ED50 to WGS 84 (30), EPSG:1784, 2 m;
- ED50 and TUREF: ED50 to ETRS89 (9) and TUREF to ETRS89 (1) reversed,
  EPSG:1783 + EPSG:5260, 2.1 m;
- the same datum: the projection alone, exact.

The geometry core (crates/shared/geometry-core/tests/all/crs.rs) and the web
through its WASM (apps/web/src/geo/transform.test.ts) must give the same:
grid points within 1e-6 m, latitudes and longitudes within 1e-11 degrees
(about a micrometre), the accuracy and the operations exactly.

Degrees, minutes and seconds are written by the ADR's rule with exact
decimals: the value rounded to the last second's place (halves away from
zero), carried into minutes and degrees, two figures before the seconds'
point, K/G for a latitude, D/B for a longitude; decimal degrees likewise.
Typed angles are read by the grammar of §4.
"""

import argparse
import json
import re
import sys
from decimal import ROUND_HALF_UP, Decimal
from pathlib import Path

from pyproj import CRS, Transformer
from pyproj.transformer import TransformerGroup

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "geodesy" / "v1" / "transform.json"
REGISTRY = ROOT / "fixtures" / "crs" / "v1" / "registry.json"

# The datum shifts PROJ takes for each pair, by its operations' names (§3).
PATHS = {
    frozenset(["TUREF", "WGS84"]): ("TUREF to WGS 84 (1)", 1.0, "EPSG:5261"),
    frozenset(["ED50", "WGS84"]): ("ED50 to WGS 84 (30)", 2.0, "EPSG:1784"),
    frozenset(["ED50", "TUREF"]): ("ED50 to ETRS89 (9)", 2.1, "EPSG:1783 + EPSG:5260"),
}

# Places across Türkiye (latitude, longitude, TUREF): provinces' centres and the zones' edges.
PLACES = [
    ("İstanbul", 41.0082, 28.9784),
    ("Ankara", 39.9334, 32.8597),
    ("Van", 38.5012, 43.3729),
    ("Antalya", 36.8969, 30.7133),
    ("Edirne", 41.6818, 26.5623),
    ("Sinop", 42.0231, 35.1531),
    ("Hakkari", 37.5744, 43.7408),
    ("TM30'un doğu kenarı", 39.5, 31.5),
    ("TM30'un batı kenarı", 39.5, 28.5),
    ("UTM 36'nın batı kenarı", 38.0, 30.0),
]


def system(entry):
    """The core's System for a registry entry; None for the local one."""
    if entry["kind"] == "local":
        return None
    if entry["kind"] == "geographic":
        return {"kind": "geographic", "datum": entry["datum"]}
    if entry.get("projection") == "Pseudo-Mercator":
        return {"kind": "mercator"}
    return {"kind": "tm", "datum": entry["datum"], "centralMeridian": entry["centralMeridian"],
            "scaleFactor": entry["scaleFactor"], "falseEasting": entry["falseEasting"],
            "falseNorthing": entry["falseNorthing"]}


def datum(entry):
    return "WGS84" if entry.get("projection") == "Pseudo-Mercator" else entry["datum"]


def transformer(src, dst):
    """PROJ's transformer from `src` to `dst` (x east or longitude first): the projection alone within a datum, the
    named EPSG path across datums."""
    a, b = datum(src), datum(dst)
    if a == b:
        return Transformer.from_crs(f"EPSG:{src['srid']}", f"EPSG:{dst['srid']}", always_xy=True), 0.0, ""
    name, accuracy, via = PATHS[frozenset([a, b])]
    group = TransformerGroup(f"EPSG:{src['srid']}", f"EPSG:{dst['srid']}", always_xy=True)
    for t in group.transformers:
        # The path whose first datum step is the named one (or its inverse).
        if name in t.description:
            return t, accuracy, via
    raise SystemExit(f"PROJ has no '{name}' path from {src['name']} to {dst['name']}")


def into(entry, lat, lon):
    """The place in the entry's own coordinates (from TUREF latitude and longitude, through PROJ)."""
    src = {"srid": 5252, "kind": "geographic", "datum": "TUREF", "name": "TUREF"}
    t, _, _ = transformer(src, entry)
    x, y = t.transform(lon, lat)
    return [x, y]


# ── Degrees, minutes and seconds ───────────────────────────────────────────────


def hemisphere(deg, latitude):
    if latitude:
        return "G" if deg < 0 else "K"
    return "B" if deg < 0 else "D"


def dms(deg, latitude, decimals):
    """The ADR's DMS text, with exact decimals (the double's own value)."""
    v = abs(Decimal(deg))
    q = Decimal(1).scaleb(-decimals)
    units = (v * 3600 / q).quantize(Decimal(1), rounding=ROUND_HALF_UP)
    per_degree = Decimal(3600) / q
    per_minute = Decimal(60) / q
    d = int(units // per_degree)
    m = int((units - d * per_degree) // per_minute)
    s = (units - d * per_degree - m * per_minute) * q
    seconds = f"{s:.{decimals}f}"
    if s < 10:
        seconds = "0" + seconds
    return f"{d}°{m:02d}′{seconds}″{hemisphere(deg, latitude)}"


def dd(deg, decimals, latitude):
    v = abs(Decimal(deg)).quantize(Decimal(1).scaleb(-decimals), rounding=ROUND_HALF_UP)
    return f"{v:.{decimals}f}°{hemisphere(deg, latitude)}"


ANGLE = re.compile(r"^\s*([+-]?)\s*(.*?)\s*([KGDBNSEWkgdbnsew]?)\s*$")


def parse(text):
    """§4's grammar: decimal degrees, degrees and minutes, degrees minutes seconds; spaces or ° ′ ″ ' "; a closing
    hemisphere letter; minutes and seconds below 60, a fraction only on the last part."""
    m = ANGLE.match(text)
    if not m:
        return None
    sign, body, letter = m.groups()
    parts = [p for p in re.split(r"[\s°′″'\"]+", body) if p]
    if not parts or len(parts) > 3:
        return None
    if not all(re.fullmatch(r"\d+(\.\d+)?", p) for p in parts):
        return None
    if any("." in p for p in parts[:-1]):
        return None
    values = [Decimal(p) for p in parts]
    if any(v >= 60 for v in values[1:]):
        return None
    value = values[0] + (values[1] / 60 if len(values) > 1 else 0) + (values[2] / 3600 if len(values) > 2 else 0)
    negative = (sign == "-") != (letter.upper() in ("G", "B", "S", "W"))
    return float(-value if negative else value)


# ── The cases ─────────────────────────────────────────────────────────────────


def build():
    registry = json.loads(REGISTRY.read_text(encoding="utf-8"))
    entries = {e["srid"]: e for e in registry["systems"] if e["kind"] != "local"}
    pairs = [
        (5254, 2320), (2320, 5254), (5254, 4326), (4326, 5254), (5254, 5252), (5252, 5254),
        (2320, 4326), (4326, 2320), (5254, 32635), (32636, 5254), (23036, 5256), (5256, 23036),
        (5257, 5256), (5253, 5254), (4326, 3857), (3857, 5254), (5252, 2320), (32637, 4326),
    ]
    places_for = {
        5253: ["Edirne", "İstanbul"], 5254: ["İstanbul", "TM30'un doğu kenarı", "TM30'un batı kenarı", "Antalya"],
        5255: ["Sinop"], 5256: ["Ankara", "Sinop"], 5257: ["Van"], 5252: ["Ankara", "Hakkari"],
        2320: ["İstanbul", "Antalya"], 4326: ["Van", "Ankara"], 32635: ["Edirne"], 32636: ["Ankara", "UTM 36'nın batı kenarı"],
        23036: ["Ankara"], 3857: ["İstanbul"], 32637: ["Van"],
    }
    by_name = {n: (lat, lon) for n, lat, lon in PLACES}
    cases = []
    for a, b in pairs:
        src, dst = entries[a], entries[b]
        t, accuracy, via = transformer(src, dst)
        for name in places_for[a]:
            lat, lon = by_name[name]
            p = into(src, lat, lon)
            x, y = t.transform(p[0], p[1])
            cases.append({"name": f"{src['name']} → {dst['name']}, {name}", "from": system(src), "to": system(dst),
                          "fromSrid": a, "toSrid": b, "p": p, "expect": {"point": [x, y], "accuracy": accuracy,
                                                                        "via": via}})
    angles = [40.7534293, 29.99999999, -0.5, 0.0, 36.0000138889, 43.37290001, -29.5, 89.9999999]
    formats = []
    for v in angles:
        for lat in (True, False):
            for dec in (2, 4):
                formats.append({"deg": v, "latitude": lat, "decimals": dec, "dms": dms(v, lat, dec)})
            formats.append({"deg": v, "latitude": lat, "decimals": 7, "dd": dd(v, 7, lat)})
    texts = ["40 45 12.3456", "40°45'12.3456\"", "40°45′12.3456″K", "40.7534293", "-29.5", "29.5 B", "29.5W", "40 45.5",
             "40 60", "40.5 30", "", "abc", "40 45 12 3", "+36", "36 00 00.05 D", "4o 45", "40 45 60", ".5", "5."]
    reads = [{"text": t, "expect": parse(t)} for t in texts]
    return {"format": "kentos.crs-transform", "version": 1,
            "proj": __import__("pyproj").proj_version_str, "transform": cases, "format_": formats, "parse": reads}


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="fail when the fixture is not what PROJ and the rules give")
    args = ap.parse_args()
    data = build()
    data["formatting"] = data.pop("format_")
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} PROJ'dan ve kurallardan çıkan değil; yeniden yazın: "
                  f"python3 {Path(__file__).relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} PROJ'dan ve kurallardan çıkanla aynı.")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(data['transform'])} dönüşüm, {len(data['formatting'])} yazılış, "
          f"{len(data['parse'])} okunuş.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
