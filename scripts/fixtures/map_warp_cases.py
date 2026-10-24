#!/usr/bin/env python3
"""Independent reference of pictures, rasters and point clouds drawn on the map's system (docs/adr/0630).

    .run/geo-py/bin/python scripts/fixtures/map_warp_cases.py           # writes fixtures/map-warp/v1/cases.json
    .run/geo-py/bin/python scripts/fixtures/map_warp_cases.py --check   # compares

A TUREF / TM36 project (EPSG:5256) seen in Web Mercator, WGS 84 degrees, TUREF / TM30, WGS 84 / UTM 36N and ED50 /
TM30, through PROJ (pyproj) and the EPSG paths crs_transform_cases.py names; no KentOS code. Five kinds of case, each
the rule of docs/adr/0630 §1–§3 written again here:

- `grid`: a raster tile's quad meshed onto the map. Its cells across are the fewest powers of two whose midpoints stray
  from the mean of the moved corners by no more than the tolerance, a quarter less at each halving; the nodes the
  quad's grid moved by PROJ. Beside them the drawn mesh's real stray: each cell's two triangles sampled 6 × 6 inside,
  PROJ's point against the point the flat triangle draws.
- `mesh`: a picture's triangles halved as the same rule asks: the level, the small triangles' count and the first
  ones' corners.
- `linear`: a point's centre moved and the map's linear part there (central differences over a step), which gives
  the view's pixels a project's metre; how far the linear part alone would stray at a node's corners (17 cm on a 2 km
  node in Web Mercator: why a node is not moved by it alone).
- `node`: a point cloud node's square meshed at a tenth of the map's tolerance, its points taken through their cell
  bilinearly: the cells and the bilinear points' real stray against PROJ.
- `box`: the box of the project's system a map's box shows: its edges walked at 16 points each and its middle taken
  back by PROJ.

The project crate's map_warp_tests read these and must give the cells and levels exactly, the points within 1e-6 m
(1e-11°) of PROJ's. KentOS and PROJ differ by at most 0.1 mm on TUREF (docs/adr/0168 Doğrulama): the comparisons
keep 1e-3 m and 1e-8°. The meshes' real stray stays below the tolerance in every case, at most about half of it (the
midpoint rule is the quadratic term; the centroid's and the edges' are both taken); the script fails above 1.5 ×.
"""

import argparse
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import crs_transform_cases as ct  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "map-warp" / "v1" / "cases.json"
PROJECT = 5256
MAPS = [3857, 4326, 5254, 32636, 2320]
# The scenes' place: E 487 200, N 4 420 600 in TM36.
E, N = 487_200.0, 4_420_600.0


def registry():
    reg = json.loads(ct.REGISTRY.read_text(encoding="utf-8"))
    return {e["srid"]: e for e in reg["systems"] if e["kind"] != "local"}


class Map:
    """The project's system seen in the map's, through PROJ."""

    def __init__(self, entries, srid):
        self.srid = srid
        self.degrees = entries[srid]["kind"] == "geographic"
        self.fwd, _, _ = ct.transformer(entries[PROJECT], entries[srid])
        self.inv, _, _ = ct.transformer(entries[srid], entries[PROJECT])
        # The map's own tolerance (docs/adr/0583 §3): about 5 cm in either's units.
        self.tolerance = 5e-7 if self.degrees else 0.05

    def forward(self, p):
        x, y = self.fwd.transform(p[0], p[1])
        return (x, y) if math.isfinite(x) and math.isfinite(y) else None

    def inverse(self, p):
        x, y = self.inv.transform(p[0], p[1])
        return (x, y) if math.isfinite(x) and math.isfinite(y) else None


def mean(ps):
    return (sum(p[0] for p in ps) / len(ps), sum(p[1] for p in ps) / len(ps))


def mesh_level(m, tris, tolerance, most):
    """The halvings the rule asks: the worst stray of an edge's or a triangle's middle, a quarter less each time."""
    worst = 0.0
    for t in tris:
        moved = [m.forward(p) for p in t]
        if any(q is None for q in moved):
            continue

        def stray(ps, ms):
            q = m.forward(mean(ps))
            if q is None:
                return 0.0
            c = mean(ms)
            return math.hypot(q[0] - c[0], q[1] - c[1])

        for i, j in ((0, 1), (1, 2), (2, 0)):
            worst = max(worst, stray([t[i], t[j]], [moved[i], moved[j]]))
        worst = max(worst, stray(t, moved))
    level, w = 0, worst
    while level < most and w > tolerance:
        w /= 4.0
        level += 1
    return level, worst


def grid_cells(m, corners, tolerance, most):
    level, worst = mesh_level(m, [[corners[0], corners[1], corners[2]], [corners[0], corners[2], corners[3]]],
                              tolerance, int(math.log2(most)))
    return 1 << level, worst


def grid_point(corners, u, v):
    p0, p1, p2, p3 = corners
    l = (p0[0] + (p3[0] - p0[0]) * v, p0[1] + (p3[1] - p0[1]) * v)
    r = (p1[0] + (p2[0] - p1[0]) * v, p1[1] + (p2[1] - p1[1]) * v)
    return (l[0] + (r[0] - l[0]) * u, l[1] + (r[1] - l[1]) * u)


def grid(m, corners, n):
    return [m.forward(grid_point(corners, i / n, j / n)) for j in range(n + 1) for i in range(n + 1)]


def grid_stray(m, corners, n, nodes):
    """The drawn mesh's real stray: each cell's triangles (p0 p1 p2, p0 p2 p3) sampled inside against PROJ."""
    worst = 0.0
    k = 6
    for j in range(n):
        for i in range(n):
            q = [nodes[j * (n + 1) + i], nodes[j * (n + 1) + i + 1], nodes[(j + 1) * (n + 1) + i + 1],
                 nodes[(j + 1) * (n + 1) + i]]
            for a in range(k + 1):
                for b in range(k + 1):
                    fu, fv = a / k, b / k
                    # The triangle the sample lies in and its barycentric weights (u along, v down).
                    if fu >= fv:
                        w = (1 - fu, fu - fv, fv)
                        tri = (q[0], q[1], q[2])
                    else:
                        w = (1 - fv, fu, fv - fu)
                        tri = (q[0], q[2], q[3])
                    flat = (sum(w[t] * tri[t][0] for t in range(3)), sum(w[t] * tri[t][1] for t in range(3)))
                    true = m.forward(grid_point(corners, (i + fu) / n, (j + fv) / n))
                    worst = max(worst, math.hypot(true[0] - flat[0], true[1] - flat[1]))
    return worst


def mesh(m, t, level):
    """The triangle halved `level` times, the small triangles' corners (project, map), as the core walks them."""
    n = 1 << level
    a, b, c = t

    def node(i, j):
        u, v = i / n, j / n
        p = (a[0] + (b[0] - a[0]) * u + (c[0] - a[0]) * v, a[1] + (b[1] - a[1]) * u + (c[1] - a[1]) * v)
        return p, m.forward(p)

    out = []
    below = [node(i, 0) for i in range(n + 1)]
    for j in range(n):
        above = [node(i, j + 1) for i in range(n - j)]
        for i in range(n - j):
            out.append((below[i], below[i + 1], above[i]))
            if i + 1 < n - j:
                out.append((below[i + 1], above[i + 1], above[i]))
        below = above
    return out


def linear_at(m, p, h):
    e, w = m.forward((p[0] + h, p[1])), m.forward((p[0] - h, p[1]))
    n, s = m.forward((p[0], p[1] + h)), m.forward((p[0], p[1] - h))
    k = 2 * h
    return m.forward(p), [(e[0] - w[0]) / k, (n[0] - s[0]) / k, (e[1] - w[1]) / k, (n[1] - s[1]) / k]


def box_back(m, b):
    steps = 16
    pts = []
    for k in range(steps + 1):
        f = k / steps
        x, y = b[0] + (b[2] - b[0]) * f, b[1] + (b[3] - b[1]) * f
        pts += [(x, b[1]), (x, b[3]), (b[0], y), (b[2], y)]
    pts.append(((b[0] + b[2]) / 2, (b[1] + b[3]) / 2))
    back = [m.inverse(p) for p in pts]
    back = [p for p in back if p is not None]
    return [min(p[0] for p in back), min(p[1] for p in back), max(p[0] for p in back), max(p[1] for p in back)]


def rnd(v, degrees):
    return round(v, 13 if degrees else 7)


def build():
    entries = registry()
    maps = {s: Map(entries, s) for s in MAPS}
    cases = []

    # Rasters' tiles: a photograph's (100 m), a valley's elevation model's coarse level (5 km), a province's (80 km),
    # each turned a little as an affine raster's are, at half a device pixel of a view drawing them about 1:1.
    tiles = [
        ("ortofoto karosu (100 m)", 100.0, 0.0, 0.05),
        ("yükseklik modelinin kaba karosu (5 km)", 5_000.0, 0.02, 2.5),
        ("il karosu (80 km)", 80_000.0, -0.01, 40.0),
    ]
    for srid, m in maps.items():
        for name, side, turn, metres in tiles:
            c, s = math.cos(turn), math.sin(turn)
            x0, y0 = E - side / 2, N + side / 2
            # Along the row east, then down south, as a raster's pixels run.
            corners = [(x0, y0), (x0 + side * c, y0 + side * s), (x0 + side * c + side * s, y0 + side * s - side * c),
                       (x0 + side * s, y0 - side * c)]
            # The tolerance in the map's units: the metres in degrees on a geographic map.
            tolerance = metres / 111_320.0 if m.degrees else metres
            n, worst = grid_cells(m, corners, tolerance, 32)
            nodes = grid(m, corners, n)
            stray = grid_stray(m, corners, n, nodes)
            assert stray <= 1.5 * tolerance or n == 32, (name, srid, stray, tolerance)
            # Not near a halving's threshold: a 0.1 mm difference between PROJ and KentOS must not move it.
            k = int(math.log2(n))
            assert abs(worst / 4 ** k / tolerance - 1) > 0.01 and (k == 0 or abs(worst / 4 ** (k - 1) / tolerance - 1) > 0.01)
            cases.append({"name": f"{name} → EPSG:{srid}", "kind": "grid", "map": srid,
                          "corners": [[x, y] for x, y in corners], "tolerance": tolerance, "most": 32,
                          "cells": n, "nodes": [[rnd(p[0], m.degrees), rnd(p[1], m.degrees)] for p in nodes],
                          "stray": stray})

    # Pictures' triangles: a site photograph's frame (60 × 40 m) and a scanned sheet's (6 × 4 km), at the map's own
    # tolerance; the small triangles at most 200 000.
    frames = [("şantiye fotoğrafı (60 × 40 m)", 60.0, 40.0), ("taranmış pafta (6 × 4 km)", 6_000.0, 4_000.0)]
    for srid, m in maps.items():
        for name, w, h in frames:
            r = [(E, N), (E + w, N), (E + w, N + h), (E, N + h)]
            tris = [[r[0], r[1], r[2]], [r[0], r[2], r[3]]]
            most = 0
            while most < 8 and len(tris) << (2 * (most + 1)) <= 200_000:
                most += 1
            level, worst = mesh_level(m, tris, m.tolerance, most)
            assert abs(worst / 4 ** level / m.tolerance - 1) > 0.01
            small = mesh(m, tris[0], level)
            cases.append({"name": f"{name} → EPSG:{srid}", "kind": "mesh", "map": srid,
                          "triangles": [[[x, y] for x, y in t] for t in tris], "tolerance": m.tolerance,
                          "most": most, "level": level, "count": 2 * 4 ** level,
                          "first": [[[rnd(c[1][0], m.degrees), rnd(c[1][1], m.degrees)] for c in tri]
                                    for tri in small[:4]]})

    # Point cloud nodes: a leaf (20 m) and a root (2 km), their centre and linear part, the stray at the corners.
    nodes = [("yaprak düğüm (20 m)", 10.0), ("kök düğüm (2 km)", 1_000.0)]
    for srid, m in maps.items():
        for name, half in nodes:
            p = (E + 13.0, N - 7.0)
            centre, j = linear_at(m, p, half)
            stray = 0.0
            for dx, dy in ((half, half), (-half, half), (half, -half), (-half, -half)):
                q = m.forward((p[0] + dx, p[1] + dy))
                lin = (centre[0] + j[0] * dx + j[1] * dy, centre[1] + j[2] * dx + j[3] * dy)
                stray = max(stray, math.hypot(q[0] - lin[0], q[1] - lin[1]))
            cases.append({"name": f"{name} → EPSG:{srid}", "kind": "linear", "map": srid, "p": list(p), "h": half,
                          "centre": [rnd(centre[0], m.degrees), rnd(centre[1], m.degrees)],
                          "j": [round(v, 12) for v in j], "stray": stray})

    # Point cloud nodes' grids: the node's square meshed at a tenth of the map's tolerance, each point taken through
    # its cell bilinearly (docs/adr/0630 §3); the stray of the bilinear points against PROJ's.
    for srid, m in maps.items():
        for name, half in nodes:
            c = (E + 13.0, N - 7.0)
            corners = [(c[0] - half, c[1] + half), (c[0] + half, c[1] + half), (c[0] + half, c[1] - half),
                       (c[0] - half, c[1] - half)]
            tolerance = m.tolerance / 10.0
            n, worst = grid_cells(m, corners, tolerance, 64)
            k = int(math.log2(n))
            assert abs(worst / 4 ** k / tolerance - 1) > 0.01 and (k == 0 or abs(worst / 4 ** (k - 1) / tolerance - 1) > 0.01)
            g = grid(m, corners, n)
            stray = 0.0
            for j in range(n):
                for i in range(n):
                    q = [g[j * (n + 1) + i], g[j * (n + 1) + i + 1], g[(j + 1) * (n + 1) + i],
                         g[(j + 1) * (n + 1) + i + 1]]
                    for a in range(7):
                        for b in range(7):
                            fu, fv = a / 6, b / 6
                            top = (q[0][0] + (q[1][0] - q[0][0]) * fu, q[0][1] + (q[1][1] - q[0][1]) * fu)
                            low = (q[2][0] + (q[3][0] - q[2][0]) * fu, q[2][1] + (q[3][1] - q[2][1]) * fu)
                            flat = (top[0] + (low[0] - top[0]) * fv, top[1] + (low[1] - top[1]) * fv)
                            true = m.forward(grid_point(corners, (i + fu) / n, (j + fv) / n))
                            stray = max(stray, math.hypot(true[0] - flat[0], true[1] - flat[1]))
            assert stray <= 1.5 * tolerance, (name, srid, stray, tolerance)
            cases.append({"name": f"bulut {name} → EPSG:{srid}", "kind": "node", "map": srid,
                          "corners": [[x, y] for x, y in corners], "tolerance": tolerance, "most": 64, "cells": n,
                          "stray": stray})

    # The boxes a view of the map shows: around the scenes' place, 2 km and 200 km wide.
    for srid, m in maps.items():
        for name, half in (("görünüm (2 km)", 1_000.0), ("görünüm (200 km)", 100_000.0)):
            c = m.forward((E, N))
            d = half / 111_320.0 if m.degrees else half
            b = [c[0] - d, c[1] - d * 0.6, c[0] + d, c[1] + d * 0.6]
            cases.append({"name": f"{name} → EPSG:{srid}", "kind": "box", "map": srid, "box": b,
                          "back": [round(v, 6) for v in box_back(m, b)]})
    return {"format": "kentos.map-warp-cases", "version": 1, "project": PROJECT, "cases": cases}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    a = ap.parse_args()
    got = build()
    text = json.dumps(got, ensure_ascii=False, indent=1) + "\n"
    kinds = {}
    for c in got["cases"]:
        kinds[c["kind"]] = kinds.get(c["kind"], 0) + 1
    worst = max(c["stray"] / c["tolerance"] for c in got["cases"] if c["kind"] in ("grid", "node"))
    if a.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            raise SystemExit(f"{OUT.relative_to(ROOT)} is not what PROJ gives: run without --check and read the diff")
        print(f"{len(got['cases'])} durum {kinds} PROJ'la aynı; ağın en büyük sapması toleransın {worst:.3f} katı")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)}: {len(got['cases'])} durum {kinds}; ağın en büyük sapması toleransın {worst:.3f} katı")


if __name__ == "__main__":
    main()
