#!/usr/bin/env python3
"""Writes fixtures/road/v1/cutout.json: the shared cases of `kentos_road::cutout` (plan/road.md Y12.3d, docs/adr/0910
§6), their expectations by shapely (GEOS), without KentOS code: the plan area the terrain keeps (the union of its
triangles less the union of the corridors' triangles) and the area it loses. The Rust test also checks that every
triangle it gives lies on the terrain (its centroid's height the terrain's there) and none overlaps another.

    uv run --no-project --python 3.11 --with shapely --with numpy --with triangle python3 scripts/fixtures/road_cutout_cases.py [--check]
"""

import json
import math
import random
import sys
from pathlib import Path

import numpy as np
import triangle
from shapely.geometry import Polygon
from shapely.ops import unary_union

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "cutout.json"


def grid(n, size, x0=0.0, y0=0.0, z=lambda x, y: 100 + 0.05 * x + 0.02 * y):
    vs = [[x0 + size * i / n, y0 + size * j / n, 0.0] for j in range(n + 1) for i in range(n + 1)]
    for v in vs:
        v[2] = round(z(v[0], v[1]), 6)
    ts = []
    for j in range(n):
        for i in range(n):
            a = j * (n + 1) + i
            ts += [[a, a + 1, a + n + 2], [a, a + n + 2, a + n + 1]]
    return vs, ts


def band(points, width, z=200.0):
    """A corridor's triangles along a polyline: two a segment, `width` wide."""
    out = []
    for (x0, y0), (x1, y1) in zip(points, points[1:]):
        dx, dy = x1 - x0, y1 - y0
        L = math.hypot(dx, dy)
        nx, ny = -dy / L * width / 2, dx / L * width / 2
        a, b, c, d = [x0 + nx, y0 + ny, z], [x0 - nx, y0 - ny, z], [x1 - nx, y1 - ny, z], [x1 + nx, y1 + ny, z]
        out += [[b, c, d], [b, d, a]]
    return out


def case(name, terrain, footprint):
    vs, ts = terrain
    land = unary_union([Polygon([vs[k][:2] for k in t]) for t in ts if Polygon([vs[k][:2] for k in t]).area > 0])
    foot = unary_union([Polygon([p[:2] for p in t]) for t in footprint if Polygon([p[:2] for p in t]).area > 0]) \
        if footprint else Polygon()
    kept = land.difference(foot).area
    return {"name": name, "vertices": vs, "triangles": ts, "footprint": footprint,
            "keptArea": kept, "lostArea": land.area - kept}


def random_terrain(seed, n, size, x0=0.0, y0=0.0):
    rng = random.Random(seed)
    pts = [[x0 + rng.random() * size, y0 + rng.random() * size] for _ in range(n)]
    t = triangle.triangulate({"vertices": np.array(pts)}, "")
    vs = [[p[0], p[1], round(100 + 0.03 * (p[0] - x0) + 4 * math.sin((p[1] - y0) / 40), 6)] for p in pts]
    return vs, t["triangles"].tolist()


def cases():
    g = grid(10, 100.0)
    ring = band([(20, 20), (80, 20), (80, 80), (20, 80), (20, 20)], 8)
    return [
        case("Çapraz şerit", g, band([(-5, 10), (105, 70)], 10)),
        case("İki şerit kesişir", g, band([(-5, 50), (105, 50)], 10) + band([(50, -5), (50, 105)], 10)),
        case("Kapalı halka: ortası kalır", g, ring),
        case("Arazi üçgeniyle aynı üçgen", g, [[[0, 0, 5], [10, 0, 5], [10, 10, 5]]]),
        case("Izgaraya oturan şerit", g, band([(0, 45), (100, 45)], 10)),
        case("Dışarıda", g, band([(200, 200), (300, 210)], 10)),
        case("Hepsini örter", g, [[[-10, -10, 0], [120, -10, 0], [120, 120, 0]], [[-10, -10, 0], [120, 120, 0], [-10, 120, 0]]]),
        case("Kenara değen", g, [[[100, 20, 0], [130, 20, 0], [130, 60, 0]]]),
        case("TM koordinatında kıvrılan yol", grid(12, 240.0, 512000.0, 4401000.0, lambda x, y: 1200 + 0.02 * (x - 512000)),
             band([(512010.5, 4401020.25), (512100.75, 4401090.5), (512150.0, 4401200.125), (512230.5, 4401230.0)], 12.5)),
        case("Rastgele arazi, kıvrılan yol", random_terrain(5, 300, 400.0),
             band([(-10, 40 + 30 * math.sin(k / 3)) if k == 0 else (k * 20.0, 200 + 60 * math.sin(k / 3)) for k in range(22)], 14)),
        case("Boş iz", g, []),
    ]


def main():
    data = {"format": "kentos.road-cutout-cases", "version": 1,
            "note": "scripts/fixtures/road_cutout_cases.py yazar; alanlar shapely (GEOS) ile, KentOS kodu olmadan.",
            "cases": cases()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text(encoding="utf-8") != text:
            raise SystemExit(f"{OUT} güncel değil: python3 {Path(__file__).name}")
        print(f"{OUT.name}: {len(data['cases'])} durum güncel")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.name}: {len(data['cases'])} durum")


if __name__ == "__main__":
    main()
