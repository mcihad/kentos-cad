#!/usr/bin/env python3
"""Independent reference of the map services' tiles (docs/adr/0208 §3).

Writes fixtures/services/v1/tiles.json from PROJ (pyproj) and the ADR's rules, no KentOS code:

- a view of the project (its box and screen pixels a unit) in a grid's system: the box its 5 × 5 points
  fall in, its centre, and a screen pixel's side at the centre (the mean of a step east and a step north);
- the level whose pixel is nearest the screen's on a log scale (on a tie the finer) within the service's
  levels; the tiles of that level the box meets, the nearest the centre first (the squared distance of the
  tiles' middles, then row, then column);
- a tile's mesh: its box's (n + 1) × (n + 1) nodes row by row from the top left, in the project's system;
- Bing's quadkeys.

The project's systems are TUREF's TM zones (GRS80; TUREF to WGS 84 is a null transformation, EPSG:5261) and
Web Mercator; the grids are Web Mercator's square (WebMercatorQuad) and the geographic one (WorldCRS84Quad).
The geometry core (crates/shared/geometry-core/tests/all/tiles.rs) must give the same: the boxes, centres and
nodes within 1e-6 m (1e-11 degrees), the pixel (their differences) within twice that, the levels and the tiles
exactly.

    python3 scripts/fixtures/service_tiles_cases.py [--check]
"""

import argparse
import json
import math
import sys
from pathlib import Path

from pyproj import Transformer

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/services/v1/tiles.json"

HALF = 20037508.342789244

SYSTEMS = {
    5254: {"kind": "tm", "datum": "TUREF", "centralMeridian": 30, "scaleFactor": 1, "falseEasting": 500000, "falseNorthing": 0},
    5255: {"kind": "tm", "datum": "TUREF", "centralMeridian": 33, "scaleFactor": 1, "falseEasting": 500000, "falseNorthing": 0},
    3857: {"kind": "mercator"},
    4326: {"kind": "geographic", "datum": "WGS84"},
}


def tm(lon0):
    return f"+proj=tmerc +lat_0=0 +lon_0={lon0} +k=1 +x_0=500000 +y_0=0 +ellps=GRS80"


def pipeline(src, dst):
    """A PROJ pipeline from `src` to `dst` (the null datum shift between TUREF and WGS 84 is no step)."""
    if src == dst:
        return None
    steps = []
    if src in (5254, 5255):
        steps.append(f"+step +inv {tm(SYSTEMS[src]['centralMeridian'])}")
    elif src == 3857:
        steps.append("+step +inv +proj=webmerc +ellps=WGS84")
    elif src == 4326:
        steps.append("+step +proj=unitconvert +xy_in=deg +xy_out=rad")
    if dst in (5254, 5255):
        steps.append(f"+step {tm(SYSTEMS[dst]['centralMeridian'])}")
    elif dst == 3857:
        steps.append("+step +proj=webmerc +ellps=WGS84")
    elif dst == 4326:
        steps.append("+step +proj=unitconvert +xy_in=rad +xy_out=deg")
    return Transformer.from_pipeline("+proj=pipeline " + " ".join(steps))


def mover(src, dst):
    t = pipeline(src, dst)
    if t is None:
        return lambda x, y: (x, y)
    return lambda x, y: tuple(float(v) for v in t.transform(x, y))


def grid(kind, tile, top):
    """The grid's matrices: (resolution, x0, y0, cols, rows) for levels 0..top."""
    if kind == "webMercator":
        res0, corner, across, down = 2 * HALF / tile, (-HALF, HALF), 1, 1
    else:
        res0, corner, across, down = 180.0 / tile, (-180.0, 90.0), 2, 1
    return [(res0 / 2 ** z, corner[0], corner[1], across << z, down << z) for z in range(top + 1)]


def view_in(view, px_per_unit, move):
    xs, ys = [], []
    for j in range(5):
        for i in range(5):
            x = view[0] + (view[2] - view[0]) * i / 4
            y = view[1] + (view[3] - view[1]) * j / 4
            gx, gy = move(x, y)
            xs.append(gx)
            ys.append(gy)
    cx, cy = (view[0] + view[2]) / 2, (view[1] + view[3]) / 2
    step = 1 / px_per_unit
    c = move(cx, cy)
    e = move(cx + step, cy)
    n = move(cx, cy + step)
    upp = (math.hypot(e[0] - c[0], e[1] - c[1]) + math.hypot(n[0] - c[0], n[1] - c[1])) / 2
    return [min(xs), min(ys), max(xs), max(ys)], list(c), upp


def level_for(matrices, upp, lo, hi):
    best, gap = lo, math.inf
    for z in range(lo, hi + 1):
        d = abs(math.log(matrices[z][0]) - math.log(upp))
        if d <= gap:
            best, gap = z, d
    return best


def tiles(matrices, z, tile, bbox, center):
    res, x0, y0, cols, rows = matrices[z]
    span = res * tile
    c1 = max(0, math.floor((bbox[0] - x0) / span))
    c2 = min(cols - 1, math.floor((bbox[2] - x0) / span))
    r1 = max(0, math.floor((y0 - bbox[3]) / span))
    r2 = min(rows - 1, math.floor((y0 - bbox[1]) / span))
    out = [(z, c, r) for r in range(r1, r2 + 1) for c in range(c1, c2 + 1)]
    assert len(out) <= 400, "a case for the exact comparison has at most 400 tiles"

    def key(t):
        _, c, r = t
        mx = x0 + (c + 0.5) * span
        my = y0 - (r + 0.5) * span
        return ((mx - center[0]) ** 2 + (my - center[1]) ** 2, r, c)

    return [list(t) for t in sorted(out, key=key)], [c1, r1, c2, r2]


def quadkey(z, c, r):
    out = ""
    for i in range(z, 0, -1):
        mask = 1 << (i - 1)
        out += str((1 if c & mask else 0) + (2 if r & mask else 0))
    return out


# Views: the project, the grid, the box and pixels a unit (a 1440 px wide drawing area).
VIEWS = [
    ("Kızılay, 1:500 civarı, TM33 üzerinde Web Mercator", 5255, "webMercator", 256, 0, 19, [487266.0, 4420575.0, 487786.0, 4420915.0]),
    ("Kızılay, 1:5000, TM33 üzerinde Web Mercator", 5255, "webMercator", 256, 0, 19, [485000.0, 4419000.0, 490200.0, 4422400.0]),
    ("Ankara, 1:50 000, TM33 üzerinde Web Mercator", 5255, "webMercator", 256, 0, 19, [460000.0, 4400000.0, 512000.0, 4434000.0]),
    ("Türkiye, TM33 üzerinde Web Mercator (512'lik karolar)", 5255, "webMercator", 512, 0, 22, [-300000.0, 3950000.0, 1300000.0, 4700000.0]),
    ("Bursa, TM30 üzerinde coğrafi ızgara", 5254, "geographic", 256, 0, 18, [480000.0, 4430000.0, 520000.0, 4460000.0]),
    ("Kızılay, Web Mercator projesi (aynı sistem)", 3857, "webMercator", 256, 0, 19, [3657300.0, 4854000.0, 3658000.0, 4854500.0]),
    ("Kızılay, katları 10–16 ile sınırlı servis", 5255, "webMercator", 256, 10, 16, [487266.0, 4420575.0, 487786.0, 4420915.0]),
]


def build():
    views = []
    for title, project, kind, tile, lo, hi, view in VIEWS:
        service = 3857 if kind == "webMercator" else 4326
        px = 1440 / (view[2] - view[0])
        move = mover(project, service)
        bbox, center, upp = view_in(view, px, move)
        matrices = grid(kind, tile, max(hi, 22))
        z = level_for(matrices, upp, lo, hi)
        found, rng = tiles(matrices, z, tile, bbox, center)
        views.append({
            "title": title,
            "project": project,
            "grid": {"kind": kind, "tile": tile, "max": max(hi, 22)},
            "levels": [lo, hi],
            "view": view,
            "pxPerUnit": px,
            "expect": {"bbox": bbox, "center": center, "unitsPerPx": upp, "level": z, "tiles": found, "range": rng},
        })
    meshes = []
    # The first tile of three views (the nearest their centres), divided as the apps divide them.
    for (title, project, kind, tile, _, _, _), v, n in zip([VIEWS[0], VIEWS[2], VIEWS[4]], [views[0], views[2], views[4]], [4, 8, 4]):
        z, c, r = v["expect"]["tiles"][0]
        service = 3857 if kind == "webMercator" else 4326
        res, x0, y0, _, _ = grid(kind, tile, 22)[z]
        span = res * tile
        bx1, by2 = x0 + c * span, y0 - r * span
        bx2, by1 = bx1 + span, by2 - span
        move = mover(service, project)
        nodes = []
        for j in range(n + 1):
            y = by2 - (by2 - by1) * j / n
            for i in range(n + 1):
                x = bx1 + (bx2 - bx1) * i / n
                nodes.extend(move(x, y))
        meshes.append({"title": title, "project": project, "grid": {"kind": kind, "tile": tile, "max": 22}, "tile": [z, c, r], "n": n, "nodes": nodes})
    keys = [{"tile": list(t), "key": quadkey(*t)} for t in [(0, 0, 0), (1, 1, 0), (3, 3, 5)] + [tuple(v["expect"]["tiles"][0]) for v in views]]
    return {
        "format": "kentos.service-tiles",
        "version": 1,
        "source": "pyproj " + __import__("pyproj").__version__ + ", PROJ " + __import__("pyproj").proj_version_str,
        "systems": {str(k): v for k, v in SYSTEMS.items()},
        "views": views,
        "meshes": meshes,
        "quadkeys": keys,
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true", help="compare with the file instead of writing it")
    args = ap.parse_args()
    data = build()
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if args.check:
        old = OUT.read_text(encoding="utf-8") if OUT.exists() else ""
        if json.loads(old or "null") != json.loads(text):
            print(f"{OUT.relative_to(ROOT)} is not what this script writes: run it without --check and read the difference.")
            return 1
        print(f"{OUT.relative_to(ROOT)} matches")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} written")
    return 0


if __name__ == "__main__":
    sys.exit(main())
