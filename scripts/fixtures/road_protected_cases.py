#!/usr/bin/env python3
"""The shared cases of a footprint against protected areas (docs/adr/0952, plan/road.md Y46; ROAD-GRADE-06), by GEOS
(shapely) without KentOS code: a triangle's overlap with an area (outer ring less holes) and a segment's length inside
it, by GEOS's own polygon overlay where KentOS clips the ring to the triangle and cuts the segment at the edges.

    uv run --no-project --python 3.11 --with shapely python3 scripts/fixtures/road_protected_cases.py [--check]
"""

import json
import sys
from pathlib import Path

from shapely.geometry import LineString, Polygon

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "protected.json"


def area_of(a):
    return Polygon(a["outer"], a["holes"])


def cases():
    out = []
    square = {"outer": [[0, 0], [10, 0], [10, 10], [0, 10]], "holes": []}
    holed = {"outer": [[0, 0], [10, 0], [10, 10], [0, 10]], "holes": [[[3, 3], [7, 3], [7, 7], [3, 7]]]}
    ell = {"outer": [[0, 0], [10, 0], [10, 3], [3, 3], [3, 10], [0, 10]], "holes": []}
    yoo = {"outer": [[0, 0], [12, 0], [12, 10], [9, 10], [9, 3], [3, 3], [3, 10], [0, 10]], "holes": []}
    cw = {"outer": [[0, 0], [0, 10], [10, 10], [10, 0]], "holes": []}
    tris = [
        [[2, 2], [8, 2], [5, 8]],
        [[-5, -5], [15, -5], [5, 20]],
        [[20, 20], [30, 20], [25, 30]],
        [[5, 5], [15, 5], [5, 15]],
        [[-2, 4], [14, 4], [6, 12]],
        [[1, 1], [9, 1], [5, 2]],
        [[8, 8], [5, 2], [2, 8]],
        [[0, 0], [10, 0], [0, 10]],
    ]
    segs = [
        [[-5, 5], [15, 5]],
        [[5, -5], [5, 15]],
        [[-5, 8], [15, 8]],
        [[2, 2], [8, 8]],
        [[20, 20], [30, 30]],
        [[-5, 0], [15, 0]],
        [[0, 0], [10, 10]],
        [[1.5, -1], [10.5, 11]],
    ]
    for name, a in [("Kare", square), ("Delikli kare", holed), ("L biçimi", ell), ("U biçimi", yoo),
                    ("Saat yönünde kare", cw)]:
        poly = area_of(a)
        out.append({"name": name, "area": a, "size": poly.area,
                    "triangles": [{"triangle": t, "covered": Polygon(t).intersection(poly).area} for t in tris],
                    "segments": [{"segment": s, "reach": LineString(s).intersection(poly).length} for s in segs]})
    return out


def main():
    data = {"cases": cases()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            sys.exit(f"{OUT} is not what {Path(__file__).name} writes")
        print(f"{OUT}: {len(data['cases'])} cases")
        return
    OUT.write_text(text)
    print(f"wrote {OUT}: {len(data['cases'])} cases")
    for c in data["cases"]:
        print(f"  {c['name'][:20]:20} {c['size']} {[round(t['covered'], 3) for t in c['triangles']]} "
              f"{[round(s['reach'], 3) for s in c['segments']]}")


if __name__ == "__main__":
    main()
