#!/usr/bin/env python3
"""Independent reference for a region's least width (docs/adr/8110): written from the ADR's definition with Shapely
(GEOS), without KentOS code. It writes `fixtures/geometry/v1/least-width.json`; `cargo test -p kentos-geometry-core
--test all least_width` reads the cases and runs `geom::neck::least_width` over them.

The definition: the least width is the length of the shortest chord across the region, with both ends on one of its
rings, that cuts it in two so that each side's largest inscribed circle is at least as wide as the chord is long; a
region with no such chord is as wide as its own largest inscribed circle.

Here by brute force: chords square into the region from points every 2 mm along every edge, and from every point of
every ring in every quarter degree, toward every other point and to the nearest point of every edge; each ray runs to
the first ring it meets (Shapely's intersection with the boundary). Chords are tried shortest first; a chord is cut
with `shapely.ops.split` and each side's circle is `shapely.maximum_inscribed_circle`.

    uv run --offline --no-project --python 3.11 --with shapely==2.2.0 python -I scripts/fixtures/least_width_cases.py
    ... --check  # compare
"""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path

import shapely
from shapely.geometry import LineString, LinearRing, Point, Polygon
from shapely.ops import split

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/geometry/v1/least-width.json"
STEP = 0.002
DEGREES = 0.25
TOL = 1e-6


def circle(poly: Polygon) -> float:
    """The diameter of the largest circle inside ``poly``."""
    return 2.0 * shapely.maximum_inscribed_circle(poly, tolerance=1e-7).length


def first_hit(poly: Polygon, rings: list[LinearRing], o: tuple[float, float], u: tuple[float, float], far: float):
    """The nearest point past ``o`` where the ray along ``u`` meets the boundary, and the index of its ring."""
    ray = LineString([o, (o[0] + u[0] * far, o[1] + u[1] * far)])
    hit = ray.intersection(poly.boundary)
    pts = []
    for g in getattr(hit, "geoms", [hit]):
        if g.is_empty:
            continue
        pts += list(g.coords)
    best = None
    for x, y in pts:
        d = math.hypot(x - o[0], y - o[1])
        if d > 1e-9 and (best is None or d < best[0]):
            best = (d, (x, y))
    if best is None:
        return None
    q = best[1]
    ring = min(range(len(rings)), key=lambda i: rings[i].distance(Point(q)))
    return q, ring


def ring_of(rings: list[LinearRing], p: tuple[float, float]) -> int:
    return min(range(len(rings)), key=lambda i: rings[i].distance(Point(p)))


def chords(poly: Polygon) -> list[tuple[float, tuple[float, float], tuple[float, float]]]:
    """Every chord tried: its length and ends, both on one ring, its inside within the region."""
    rings = [poly.exterior] + list(poly.interiors)
    x0, y0, x1, y1 = poly.bounds
    far = 4.0 * math.hypot(x1 - x0, y1 - y0)
    out = []

    def add(o, u):
        step = Point(o[0] + u[0] * 1e-7, o[1] + u[1] * 1e-7)
        if not poly.contains(step):
            return
        h = first_hit(poly, rings, o, u, far)
        if h is None:
            return
        q, rq = h
        if rq != ring_of(rings, o):
            return
        mid = Point((o[0] + q[0]) / 2, (o[1] + q[1]) / 2)
        if not poly.buffer(1e-9).contains(mid):
            return
        out.append((math.hypot(q[0] - o[0], q[1] - o[1]), o, q))

    points = []
    edges = []
    for r in rings:
        c = list(r.coords)[:-1]
        points += c
        edges += [(c[i], c[(i + 1) % len(c)]) for i in range(len(c))]
    for a, b in edges:
        ex, ey = b[0] - a[0], b[1] - a[1]
        length = math.hypot(ex, ey)
        if length == 0:
            continue
        k = max(1, math.ceil(length / STEP))
        for j in range(k):
            t = (j + 0.5) / k
            o = (a[0] + ex * t, a[1] + ey * t)
            for n in ((-ey / length, ex / length), (ey / length, -ex / length)):
                add(o, n)
    for v in points:
        dirs = [(math.cos(math.radians(a * DEGREES)), math.sin(math.radians(a * DEGREES)))
                for a in range(int(360 / DEGREES))]
        for w in points:
            d = math.hypot(w[0] - v[0], w[1] - v[1])
            if d > 1e-9:
                dirs.append(((w[0] - v[0]) / d, (w[1] - v[1]) / d))
        for a, b in edges:
            ex, ey = b[0] - a[0], b[1] - a[1]
            ee = ex * ex + ey * ey
            t = 0.0 if ee == 0 else min(1.0, max(0.0, ((v[0] - a[0]) * ex + (v[1] - a[1]) * ey) / ee))
            f = (a[0] + ex * t - v[0], a[1] + ey * t - v[1])
            d = math.hypot(*f)
            if d > 1e-9:
                dirs.append((f[0] / d, f[1] / d))
        for u in dirs:
            add(v, u)
    return out


def least(rings_xy: list[list[tuple[float, float]]]) -> dict:
    poly = Polygon(rings_xy[0], rings_xy[1:])
    assert poly.is_valid, shapely.validation.explain_validity(poly)
    full = circle(poly)
    tried = sorted((c for c in chords(poly) if c[0] <= full + 1e-6), key=lambda c: c[0])
    for length, p, q in tried:
        # The chord a hair longer at both ends, so that it crosses the boundary it ends on.
        dx, dy = (q[0] - p[0]) / length, (q[1] - p[1]) / length
        cut = LineString([(p[0] - dx * 1e-9, p[1] - dy * 1e-9), (q[0] + dx * 1e-9, q[1] + dy * 1e-9)])
        parts = [g for g in split(poly, cut).geoms if g.area > 1e-12]
        if len(parts) < 2:
            continue
        if all(circle(g) >= length - TOL for g in parts):
            return {"width": round(length, 7), "neck": True}
    return {"width": round(full, 7), "neck": False}


def rect(x0, y0, x1, y1):
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]


def moved(ring, dx, dy):
    return [(x + dx, y + dy) for x, y in ring]


def turned(ring, deg):
    a = math.radians(deg)
    c, s = math.cos(a), math.sin(a)
    return [(x * c - y * s, x * s + y * c) for x, y in ring]


def arc(cx, cy, r, a0, a1, n):
    return [(cx + r * math.cos(math.radians(a0 + (a1 - a0) * i / n)), cy + r * math.sin(math.radians(a0 + (a1 - a0) * i / n)))
            for i in range(n + 1)]


def cases() -> list[dict]:
    L = [(0, 0), (10, 0), (10, 1.8), (1.8, 1.8), (1.8, 10), (0, 10)]
    T = [(0, 0), (20, 0), (20, 1.6), (10.8, 1.6), (10.8, 10), (9.2, 10), (9.2, 1.6), (0, 1.6)]
    X = [(-0.75, -10), (0.75, -10), (0.75, -0.75), (10, -0.75), (10, 0.75), (0.75, 0.75), (0.75, 10), (-0.75, 10),
         (-0.75, 0.75), (-10, 0.75), (-10, -0.75), (-0.75, -0.75)]
    out = [
        ("koridor", "20 m'lik 2,00 m koridor: duvarları arası", [rect(0, 0, 20, 2)]),
        ("kisa-oda", "3,00 × 2,00 oda: boğazı yok, dairesi kadar", [rect(0, 0, 3, 2)]),
        ("kare", "4 × 4 oda: boğazı yok", [rect(0, 0, 4, 4)]),
        ("L", "L koridor, kollar 1,80 m (en büyük daire 2,11 m)", [L]),
        ("L-esit-olmayan", "L koridor, kollar 1,60 m ve 2,20 m", [[(0, 0), (12, 0), (12, 1.6), (2.2, 1.6), (2.2, 12),
                                                               (0, 12)]]),
        ("T", "T koridor, kollar 1,60 m (en büyük daire 2,00 m)", [T]),
        ("arti", "artı koridor, kollar 1,50 m (en büyük daire 2,12 m)", [X]),
        ("kapi-girintisi", "2,00 m koridorda 1,00 m geniş 0,15 m derin kapı girintisi boğaz değil",
         [[(0, 0), (9.5, 0), (9.5, -0.15), (10.5, -0.15), (10.5, 0), (20, 0), (20, 2), (0, 2)]]),
        ("derin-nis", "1,00 m geniş 1,20 m derin niş: genişliği kadar derin kol",
         [[(0, 0), (9.5, 0), (9.5, -1.2), (10.5, -1.2), (10.5, 0), (20, 0), (20, 2), (0, 2)]]),
        ("ayak", "2,40 m koridorda 0,40 m çıkıntı: yanındaki 2,00 m",
         [[(0, 0), (9.7, 0), (9.7, 0.4), (10.3, 0.4), (10.3, 0), (20, 0), (20, 2.4), (0, 2.4)]]),
        ("bel", "iki ucu 2,50 m, ortası 1,50 m", [[(0, 0), (10, 0), (10, 2.5), (5, 1.5), (0, 2.5)]]),
        ("cikmaz-daralan", "kapalı ucu 1,50 m, öbür ucu 2,50 m: boğaz değil, dairesi kadar",
         [[(0, 0), (10, 0), (10, 2.5), (0, 1.5)]]),
        ("lobili", "5 × 5 holden çıkan 15 m'lik 1,20 m koridor",
         [[(0, 0), (5, 0), (5, 1.9), (20, 1.9), (20, 3.1), (5, 3.1), (5, 5), (0, 5)]]),
        ("capraz-koseler", "3,00 m koridorda iki yandan karşılıklı kaydırılmış çıkıntı: köşeden köşeye",
         [[(0, 0), (8, 0), (8, 1.2), (9, 1.2), (9, 0), (20, 0), (20, 3), (10.5, 3), (10.5, 1.8), (9.5, 1.8),
           (9.5, 3), (0, 3)]]),
        ("yay", "iki 4 × 4 hol arasında iç yarıçapı 5 m, dışı 7 m çeyrek yay koridor (64 parçalı)",
         [[(8, -4), (8, 0)] + arc(0, 0, 7, 0, 90, 64) + [(0, 8), (-4, 8), (-4, 4), (0, 4)] + arc(0, 0, 5, 90, 0, 64)
          + [(4, 0), (4, -4)]]),
        ("uzakta", "L koridor, projeksiyon düzleminde uzakta", [moved(L, 4_400_000.0, 520_000.0)]),
        ("ters-yon", "T koridor saat yönünde çizilmiş", [list(reversed(T))]),
        ("dondurulmus", "2,00 m koridor 30° döndürülmüş", [turned(rect(0, 0, 20, 2), 30)]),
        ("pahli-L", "köşesi 45° pahlı L koridor, kollar 1,80 m",
         [[(0, 0.6), (0.6, 0), (10, 0), (10, 1.8), (1.8, 1.8), (1.8, 10), (0, 10)]]),
        ("Z", "Z koridor, kollar 1,40 m",
         [[(0, 0), (8, 0), (8, 6), (14, 6), (14, 7.4), (6.6, 7.4), (6.6, 1.4), (0, 1.4)]]),
        ("iki-girinti-lobi", "1,80 m koridor, iki kapı girintisi, sonunda 4 × 4 hol",
         [[(0, 0), (4, 0), (4, -0.2), (5, -0.2), (5, 0), (9, 0), (9, -0.2), (10, -0.2), (10, 0), (16, 0), (16, -1.1),
           (20, -1.1), (20, 2.9), (16, 2.9), (16, 1.8), (0, 1.8)]]),
        ("eğik-bel", "iki yanı eğik bel: ortası 1,60 m", [[(0, 0), (6, 0.5), (12, 0), (12, 3), (6, 2.1), (0, 3)]]),
        ("kolonlu", "3,00 m koridorun ortasında serbest kolon: yanındaki aralık ölçülmez",
         [rect(0, 0, 20, 3), rect(9.8, 1.3, 10.2, 1.7)]),
        ("U-ayak", "içi 0,30 m geniş 0,50 m derin U biçimli serbest ayağın cebi: genişliği kadar derin",
         [rect(0, 0, 20, 3), [(9.5, 1.0), (10.5, 1.0), (10.5, 2.0), (10.15, 2.0), (10.15, 1.5), (9.85, 1.5),
                              (9.85, 2.0), (9.5, 2.0)]]),
    ]
    result = []
    for name, why, rings in out:
        rings = [[(float(x), float(y)) for x, y in r] for r in rings]
        result.append({"id": name, "why": why, "rings": [[list(p) for p in r] for r in rings],
                       "expect": least(rings)})
        print(name, result[-1]["expect"], file=sys.stderr)
    return result


def main() -> None:
    text = json.dumps({"version": 1, "about": __doc__.split("\n\n")[1].replace("\n", " "), "cases": cases()},
                      ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"{OUT} is out of date; run the script to rewrite it")
        print("up to date")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)}: {len(json.loads(text)['cases'])} cases")


if __name__ == "__main__":
    main()
