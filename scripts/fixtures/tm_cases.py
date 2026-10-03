#!/usr/bin/env python3
"""The transverse Mercator cases of the geometry core's forward projection (docs/adr/0165 §3).

`fixtures/geodesy/v1/tm-forward.json` holds points (latitude, longitude) and
where PROJ puts them on the grids of the registry's transverse Mercator
systems: TUREF's TM3 zones (GRS80, k = 1), ED50's (International 1924) and
the UTM zones (k = 0.9996). PROJ's `tmerc` (Poder and Engsager's Krüger
series) is the independent reference: no KentOS code. The Rust core
(crates/shared/geometry-core/tests/geodesy.rs) and the web through the WASM
module (apps/web/src/model/geom/geodesy.test.ts) read the same file and
agree with it to a micrometre.

    python3 scripts/fixtures/tm_cases.py           # writes the file
    python3 scripts/fixtures/tm_cases.py --check   # compares it with PROJ
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

from pyproj import Proj

ROOT = Path(__file__).resolve().parents[2]
FILE = ROOT / "fixtures" / "geodesy" / "v1" / "tm-forward.json"

ELLIPSOIDS = {"GRS80": (6378137.0, 298.257222101), "intl": (6378388.0, 297.0), "WGS84": (6378137.0, 298.257223563)}

# (name, central meridian, scale factor, false easting, ellipsoid)
GRIDS = [
    ("TUREF / TM27", 27, 1.0, 500000.0, "GRS80"),
    ("TUREF / TM30", 30, 1.0, 500000.0, "GRS80"),
    ("TUREF / TM33", 33, 1.0, 500000.0, "GRS80"),
    ("TUREF / TM36", 36, 1.0, 500000.0, "GRS80"),
    ("TUREF / TM39", 39, 1.0, 500000.0, "GRS80"),
    ("TUREF / TM42", 42, 1.0, 500000.0, "GRS80"),
    ("TUREF / TM45", 45, 1.0, 500000.0, "GRS80"),
    ("ED50 / TM30", 30, 1.0, 500000.0, "intl"),
    ("ED50 / UTM 35N", 27, 0.9996, 500000.0, "intl"),
    ("WGS 84 / UTM 36N", 33, 0.9996, 500000.0, "WGS84"),
]

# Province centres in each grid's zone, its edges (1.5° and 3° off the meridian),
# the equator and the far north; latitudes and longitudes in degrees.
OFFSETS = [(36.0, 0.0), (37.25, -1.5), (39.9, 1.5), (41.5, -0.75), (42.1, 3.0), (0.0, 0.5), (60.0, -2.0), (-12.5, 1.0)]


def proj_of(cm: float, k: float, fe: float, ellps: str) -> Proj:
    a, rf = ELLIPSOIDS[ellps]
    return Proj(f"+proj=tmerc +lat_0=0 +lon_0={cm} +k={k} +x_0={fe} +y_0=0 +a={a} +rf={rf} +units=m +no_defs")


def build() -> dict:
    cases = []
    for name, cm, k, fe, ellps in GRIDS:
        a, rf = ELLIPSOIDS[ellps]
        p = proj_of(cm, k, fe, ellps)
        for lat, off in OFFSETS:
            lon = cm + off
            east, north = p(lon, lat)
            cases.append({
                "grid": name,
                "tm": {"centralMeridian": cm, "scaleFactor": k, "falseEasting": fe, "falseNorthing": 0.0, "semiMajor": a, "inverseFlattening": rf},
                "lat": lat,
                "lon": lon,
                "east": east,
                "north": north,
            })
    return {
        "format": "kentos.tm-forward-cases",
        "version": 1,
        "note": "PROJ 9's tmerc (pyproj); within 1e-6 m of it is the same point. Written by scripts/fixtures/tm_cases.py.",
        "cases": cases,
    }


def main() -> int:
    data = build()
    if "--check" in sys.argv[1:]:
        have = json.loads(FILE.read_text(encoding="utf-8"))
        bad = [
            f"{w['grid']} {w['lat']},{w['lon']}"
            for h, w in zip(have["cases"], data["cases"])
            if abs(h["east"] - w["east"]) > 1e-9 or abs(h["north"] - w["north"]) > 1e-9 or h["tm"] != w["tm"]
        ]
        if len(have["cases"]) != len(data["cases"]) or bad:
            print(f"✗ {FILE.name}: {len(bad)} cases differ from PROJ: {bad[:5]}")
            return 1
        print(f"✓ {FILE.name}: {len(data['cases'])} cases as PROJ puts them")
        return 0
    FILE.write_text(json.dumps(data, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print(f"wrote {FILE.relative_to(ROOT)}: {len(data['cases'])} cases")
    return 0


if __name__ == "__main__":
    sys.exit(main())
