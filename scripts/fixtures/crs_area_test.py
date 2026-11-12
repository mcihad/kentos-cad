"""A point against EPSG's area of use as the CRS notes read it (docs/adr/3700, 4900), for the CRS case generators. No
KentOS code.

A registry system whose extent's polygon is carried (fixtures/crs/v1/area-polygons.json, the EPSG Dataset's, unchanged)
holds a point inside its rings (even-odd: a ray to the east crosses an odd number of edges) or within 1e-9° of an edge;
any other area is its box with every edge 1e-9° out (a box west of its east crossing the antimeridian).
"""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EDGE = 1e-9
_EXTENTS = json.loads((ROOT / "fixtures/crs/v1/area-polygons.json").read_text("utf-8"))["extents"]
POLYGONS = {s: x["rings"] for x in _EXTENTS for s in x["srids"]}
# docs/adr/5300: an EPSG extent's polygon by its code (a datum operation's area).
EXTENTS = {x["code"]: x["rings"] for x in _EXTENTS}


def box_holds(b, lon, lat):
    w, s, e, n = b[0] - EDGE, b[1] - EDGE, b[2] + EDGE, b[3] + EDGE
    along = (w <= lon <= e) if w <= e else (lon >= w or lon <= e)
    return along and s <= lat <= n


def rings_hold(rings, x, y):
    inside = False
    for r in rings:
        for (x1, y1), (x2, y2) in zip(r, r[1:]):
            if (y1 > y) != (y2 > y) and x < x1 + (y - y1) * (x2 - x1) / (y2 - y1):
                inside = not inside
    if inside:
        return True
    for r in rings:
        for (x1, y1), (x2, y2) in zip(r, r[1:]):
            vx, vy = x2 - x1, y2 - y1
            l2 = vx * vx + vy * vy
            t = 0.0 if l2 == 0 else max(0.0, min(1.0, ((x - x1) * vx + (y - y1) * vy) / l2))
            if (x - x1 - vx * t) ** 2 + (y - y1 - vy * t) ** 2 <= EDGE * EDGE:
                return True
    return False


def extent_of(code, box):
    """docs/adr/5300: a datum operation's area: its extent's polygon where carried, else its box."""
    return ("rings", EXTENTS[code]) if code in EXTENTS else ("box", box)


def area_of(srid, box):
    """A registry system's area: its polygon where carried, else its box."""
    return ("rings", POLYGONS[srid]) if srid in POLYGONS else ("box", box)


def holds(area, lon, lat):
    kind, v = area
    return rings_hold(v, lon, lat) if kind == "rings" else box_holds(v, lon, lat)
