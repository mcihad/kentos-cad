#!/usr/bin/env python3
"""Independent reference of the box transform (docs/adr/0504 §2, plan/gis.md CBS-07).

    python3 scripts/fixtures/crs_bounds_cases.py           # writes fixtures/geodesy/v1/bounds.json
    python3 scripts/fixtures/crs_bounds_cases.py --check   # compares

A box of one system as a box of another, from PROJ's own `transform_bounds` (pyproj, `densify_pts`), no KentOS code:
each edge sampled at `densify` inner points besides its corners, the moved points' extent; a target in degrees whose
edges cross the antimeridian answers min x > max x. The datum shifts are the EPSG operations crs_transform_cases.py
names. The geometry core (crates/shared/geometry-core/tests/all/crs.rs) must give the same box: grid coordinates
within 1e-6 m, degrees within 1e-11. Two corner points alone would miss the bulge of a TM zone's edge: the cases with
densify 0 and 21 give different boxes, and both are checked.
"""

import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import crs_transform_cases as ct  # noqa: E402
from pyproj import CRS, Transformer  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "geodesy" / "v1" / "bounds.json"

# A TM zone on the antimeridian (UTM 60's edge, WGS 84), beside the registry's systems.
DATELINE = {"kind": "tm", "datum": "WGS84", "centralMeridian": 180, "scaleFactor": 0.9996, "falseEasting": 500000,
            "falseNorthing": 0}
DATELINE_PROJ = "+proj=tmerc +lat_0=0 +lon_0=180 +k=0.9996 +x_0=500000 +y_0=0 +datum=WGS84 +units=m +no_defs"


def build():
    registry = json.loads(ct.REGISTRY.read_text(encoding="utf-8"))
    entries = {e["srid"]: e for e in registry["systems"] if e["kind"] != "local"}
    cases = []

    def add(name, a, b, box, densify):
        src, dst = entries[a], entries[b]
        t, _, _ = ct.transformer(src, dst)
        if densify == 0:
            # PROJ asks for at least two inner points toward degrees: the corners' extent by its point transform.
            xs, ys = t.transform([box[0], box[2], box[2], box[0]], [box[1], box[1], box[3], box[3]])
            got = (min(xs), min(ys), max(xs), max(ys))
        else:
            got = t.transform_bounds(*box, densify_pts=densify)
        cases.append({"name": name, "from": ct.system(src), "to": ct.system(dst), "fromSrid": a, "toSrid": b,
                      "bounds": list(box), "densify": densify, "expect": list(got)})

    # Ankara's TM33 sheet: a 20 km square.
    ankara = (480000.0, 4410000.0, 500000.0, 4430000.0)
    add("TM33 → WGS 84, Ankara, köşeler", 5255, 4326, ankara, 0)
    add("TM33 → WGS 84, Ankara, sık", 5255, 4326, ankara, 21)
    # A whole zone wide: the edges bend, the corners alone miss the box.
    zone = (330000.0, 4000000.0, 670000.0, 4700000.0)
    add("TM33 → WGS 84, bütün dilim, köşeler", 5255, 4326, zone, 0)
    add("TM33 → WGS 84, bütün dilim, sık", 5255, 4326, zone, 21)
    add("WGS 84 → TM30, İstanbul", 4326, 5254, (28.5, 40.8, 29.5, 41.3), 21)
    add("WGS 84 → Web Mercator, Türkiye", 4326, 3857, (26.0, 36.0, 45.0, 42.0), 21)
    add("Web Mercator → TM30, İstanbul", 3857, 5254, (3200000.0, 4980000.0, 3260000.0, 5030000.0), 21)
    add("ED50 UTM 36 → TUREF TM33, Ankara", 23036, 5255, ankara, 21)
    add("TM30 → UTM 35, sınırda", 5254, 32635, (380000.0, 4300000.0, 520000.0, 4500000.0), 21)

    # The antimeridian: a box on UTM 60's edge, its longitudes on both sides of 180°.
    t = Transformer.from_crs(CRS.from_proj4(DATELINE_PROJ), CRS.from_epsg(4326), always_xy=True)
    box = (400000.0, 5500000.0, 600000.0, 5600000.0)
    got = t.transform_bounds(*box, densify_pts=21)
    cases.append({"name": "180° boylamında dilim → WGS 84: kutu sarar", "from": DATELINE,
                  "to": ct.system(entries[4326]), "fromSrid": None, "toSrid": 4326, "bounds": list(box),
                  "densify": 21, "expect": list(got)})
    return {"format": "kentos.crs-bounds", "version": 1, "proj": __import__("pyproj").proj_version_str,
            "cases": cases}


def main():
    data = build()
    corners = next(c for c in data["cases"] if c["name"].endswith("bütün dilim, köşeler"))["expect"]
    dense = next(c for c in data["cases"] if c["name"].endswith("bütün dilim, sık"))["expect"]
    assert corners != dense, "the dense box must differ from the corners' box"
    wrap = data["cases"][-1]["expect"]
    assert wrap[0] > wrap[2], "the antimeridian box wraps"
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"kutu dönüşümü durumları tutarlı: {len(data['cases'])} durum")
        return 0
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(data['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
