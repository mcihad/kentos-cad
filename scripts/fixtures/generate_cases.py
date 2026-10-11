#!/usr/bin/env python3
"""Independent reference of the Geometri üretimi tools (docs/adr/0219), written from the ADR alone, no KentOS code.

Writes fixtures/processing/v1/generate.kcad (the drawing) and fixtures/processing/v1/generate.json (the tools run on it,
in the processing cases' format, fixtures/processing/README.md).

How it computes, independently of the core's ways:

1. Voronoi cells exactly by fractions: the frame cut by every other site's bisector (no triangulation), rounded once;
   cross-checked with GEOS (shapely's voronoi_polygons, each cell cut to the frame).
2. Delaunay triangles exactly on integers (the coordinates times 2³⁴): every triple whose circle holds no other site;
   cross-checked with GEOS's delaunay_triangles.
3. Convex hulls by Andrew's chain on those integers; boxes by the points' extremes; rotated boxes by every hull edge at
   50 digits (cross-checked with GEOS's oriented_envelope's area); circles exactly by Welzl's algorithm on fractions
   (cross-checked with GEOS's minimum_bounding_radius).
4. Concave hulls by the ADR's rule (§5) over the exact triangulation, the lengths √(dx² + dy²) in float64 as the ADR
   says; cross-checked with GEOS's concave_hull without holes (GEOS takes hole seeds smallest first, §5).
5. Poles by polylabel written from its description (§6) with the ADR's order of equal cells, float64 as the ADR says;
   cross-checked with GEOS's maximum_inscribed_circle within the precision.
6. Curves as the ADR says (§4): an arc's points at the chord step that keeps 0.1 mm, its extremes along the axes.
"""

import heapq
import json
import math
import random
import sys
from fractions import Fraction as F
from itertools import combinations
from pathlib import Path

import mpmath as mp
import shapely
from shapely.geometry import MultiPoint, Point, Polygon

sys.path.insert(0, str(Path(__file__).resolve().parent))
from layer_ops_cases import new_id  # noqa: E402
from numeric_display import shown  # noqa: E402

mp.mp.dps = 50

ROOT = Path(__file__).resolve().parents[2]
PROC_DIR = ROOT / "fixtures" / "processing" / "v1"
E, N = 487000, 4420000
TOL = 1e-8
# Floats near the drawing's coordinates are integers times 2⁻³⁴ (or a power of two of it).
SCALE = 2**34
CURVE_TOL = 1e-4


def P(x, y):
    return {"x": E + x, "y": N + y}


def rect(x0, y0, x1, y1):
    return [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]


def xy(p):
    return (p["x"], p["y"])


def out(p):
    return {"x": float(p[0]), "y": float(p[1])}


# ── Exact arithmetic ───────────────────────────────────────────────────────


def ints(p):
    """A float point's coordinates as integers (times 2³⁴), exactly."""
    x, y = F(p[0]) * SCALE, F(p[1]) * SCALE
    assert x.denominator == 1 and y.denominator == 1, p
    return (int(x), int(y))


def orient(a, b, c):
    return (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])


def in_circle(a, b, c, d):
    """Positive when d is strictly inside the circle through a, b, c (counter-clockwise)."""
    rows = []
    for p in (a, b, c):
        dx, dy = p[0] - d[0], p[1] - d[1]
        rows.append((dx, dy, dx * dx + dy * dy))
    (a1, a2, a3), (b1, b2, b3), (c1, c2, c3) = rows
    return a1 * (b2 * c3 - b3 * c2) - a2 * (b1 * c3 - b3 * c1) + a3 * (b1 * c2 - b2 * c1)


def distinct(points):
    """Each place once, in the order of its first point; and for each place its first point's number."""
    seen, places, first = {}, [], []
    for i, p in enumerate(points):
        k = (p[0] + 0.0, p[1] + 0.0)
        if k not in seen:
            seen[k] = len(places)
            places.append(p)
            first.append(i)
    return places, first


def normalise(ring):
    """A ring without repeated corners, from its lowest corner (the westmost of equal ones), its turn as it is."""
    r = []
    for p in ring:
        if not r or r[-1] != p:
            r.append(p)
    while len(r) > 1 and r[0] == r[-1]:
        r.pop()
    k = min(range(len(r)), key=lambda i: (r[i][1], r[i][0]))
    return r[k:] + r[:k]


# ── Voronoi (§2) ──────────────────────────────────────────────────────────


def frame_of(places, margin):
    """The frame as the ADR computes it in float64: the box grown by the larger side times the margin (a fraction)."""
    xs, ys = [p[0] for p in places], [p[1] for p in places]
    x0, x1, y0, y1 = min(xs), max(xs), min(ys), max(ys)
    grow = max(x1 - x0, y1 - y0) * margin
    return (x0 - grow, y0 - grow, x1 + grow, y1 + grow)


def cut_q(ring, p, q):
    """The ring cut to the side of the bisector of p and q nearer to p, exactly."""
    mx, my = (p[0] + q[0]) / 2, (p[1] + q[1]) / 2
    nx, ny = q[0] - p[0], q[1] - p[1]

    def side(v):
        return (v[0] - mx) * nx + (v[1] - my) * ny

    res = []
    for i in range(len(ring)):
        a, b = ring[i], ring[(i + 1) % len(ring)]
        sa, sb = side(a), side(b)
        if sa <= 0:
            res.append(a)
        if (sa < 0 < sb) or (sb < 0 < sa):
            t = sa / (sa - sb)
            res.append((a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t))
    return res


def voronoi_cells(places, frame):
    """Every place's cell exactly: the frame cut by every other place's bisector, then rounded."""
    fq = [F(v) for v in frame]
    pq = [(F(p[0]), F(p[1])) for p in places]
    cells = []
    for i, p in enumerate(pq):
        ring = [(fq[0], fq[1]), (fq[2], fq[1]), (fq[2], fq[3]), (fq[0], fq[3])]
        for j, q in enumerate(pq):
            if j != i:
                ring = cut_q(ring, p, q)
        ring = normalise(ring)
        area = sum(ring[k][0] * ring[(k + 1) % len(ring)][1] - ring[(k + 1) % len(ring)][0] * ring[k][1] for k in range(len(ring)))
        cells.append([(float(x), float(y)) for x, y in ring] if len(ring) >= 3 and area > 0 else None)
    return cells


def geos_voronoi_check(places, frame, cells):
    """GEOS's cells, each cut to the frame, against the exact ones (within a micrometre)."""
    box = shapely.box(*frame)
    got = shapely.voronoi_polygons(MultiPoint(places), extend_to=box)
    by_site = {}
    for g in got.geoms:
        g = g.intersection(box)
        for i, p in enumerate(places):
            if g.covers(Point(p)):
                by_site.setdefault(i, g)
    for i, c in enumerate(cells):
        if c is None:
            continue
        g = by_site[i]
        assert abs(g.area - Polygon(c).area) < 1e-6, (i, g.area, Polygon(c).area)
        assert Polygon(c).symmetric_difference(g).area < 1e-6, i


# ── Delaunay (§3) ─────────────────────────────────────────────────────────


def delaunay(places):
    """The Delaunay triangles exactly: every counter-clockwise triple whose circle holds no other place."""
    q = [ints(p) for p in places]
    tris = []
    for i, j, k in combinations(range(len(q)), 3):
        o = orient(q[i], q[j], q[k])
        if o == 0:
            continue
        if o < 0:
            j, k = k, j
        if all(in_circle(q[i], q[j], q[k], q[m]) <= 0 for m in range(len(q)) if m not in (i, j, k)):
            tris.append((i, j, k))
    return tris


def geos_delaunay_check(places, tris):
    got = shapely.delaunay_triangles(MultiPoint(places))
    want = {frozenset(map(tuple, (places[i] for i in t))) for t in tris}
    have = {frozenset(tuple(c) for c in g.exterior.coords[:3]) for g in got.geoms}
    assert want == have, "GEOS'un üçgenleri başka"


# ── Hulls, boxes, circles (§4) ─────────────────────────────────────────────


def hull(points):
    """The convex hull, counter-clockwise from its lowest point (the westmost of equal ones), exact turns."""
    pts = sorted(set((p[0] + 0.0, p[1] + 0.0) for p in points), key=lambda p: (p[1], p[0]))
    if len(pts) < 3:
        return pts
    q = {p: ints(p) for p in pts}

    def chain(seq):
        res = []
        for p in seq:
            while len(res) >= 2 and orient(q[res[-2]], q[res[-1]], q[p]) <= 0:
                res.pop()
            res.append(p)
        return res

    lower, upper = chain(pts), chain(reversed(pts))
    h = lower[:-1] + upper[:-1]
    return h if len(h) >= 3 else [pts[0], pts[-1]]


def ring_area_q(ring):
    """A ring's area exactly (counter-clockwise positive)."""
    rq = [(F(x), F(y)) for x, y in ring]
    return sum(rq[k][0] * rq[(k + 1) % len(rq)][1] - rq[(k + 1) % len(rq)][0] * rq[k][1] for k in range(len(rq))) / 2


def axis_box(points):
    xs, ys = [p[0] for p in points], [p[1] for p in points]
    x0, x1, y0, y1 = min(xs), max(xs), min(ys), max(ys)
    if not (x1 > x0 and y1 > y0):
        return None
    return {"ring": [(x0, y0), (x1, y0), (x1, y1), (x0, y1)], "width": x1 - x0, "height": y1 - y0}


def half_bearing(ux, uy):
    t = mp.atan2(ux, uy)
    if t < 0:
        t += mp.pi
    if t >= mp.pi:
        t -= mp.pi
    return t


def rotated_box(points):
    """The least rotated box: every hull edge at 50 digits; of equal areas the first edge (from the lowest point)."""
    h = hull(points)
    if len(h) < 3:
        return None
    hm = [(mp.mpf(p[0]), mp.mpf(p[1])) for p in h]
    best = None
    for i in range(len(hm)):
        a, b = hm[i], hm[(i + 1) % len(hm)]
        length = mp.hypot(b[0] - a[0], b[1] - a[1])
        u = ((b[0] - a[0]) / length, (b[1] - a[1]) / length)
        w = (-u[1], u[0])
        s = [(p[0] - a[0]) * u[0] + (p[1] - a[1]) * u[1] for p in hm]
        t = [(p[0] - a[0]) * w[0] + (p[1] - a[1]) * w[1] for p in hm]
        s0, s1, t1 = min(s), max(s), max(t)
        area = (s1 - s0) * t1
        if best is not None and area >= best[0]:
            continue

        def at(sv, tv):
            return (float(a[0] + sv * u[0] + tv * w[0]), float(a[1] + sv * u[1] + tv * w[1]))

        ring = normalise([at(s0, 0), at(s1, 0), at(s1, t1), at(s0, t1)])
        along, across = s1 - s0, t1
        long_dir = u if along >= across else w
        long_side, short_side = (along, across) if along >= across else (across, along)
        best = (area, {"ring": ring, "length": float(long_side), "width": float(short_side), "bearing": float(half_bearing(*long_dir))})
    got = shapely.oriented_envelope(MultiPoint(h)).area
    assert abs(got - float(best[0])) <= 1e-6 * got, ("GEOS'un döndürülmüş kutusu başka", got, float(best[0]))
    return best[1]


def circle3(a, b, c):
    """The exact centre of the circle through a, b, c (fractions)."""
    d = 2 * (a[0] * (b[1] - c[1]) + b[0] * (c[1] - a[1]) + c[0] * (a[1] - b[1]))
    if d == 0:
        return None
    a2, b2, c2 = a[0] ** 2 + a[1] ** 2, b[0] ** 2 + b[1] ** 2, c[0] ** 2 + c[1] ** 2
    ux = (a2 * (b[1] - c[1]) + b2 * (c[1] - a[1]) + c2 * (a[1] - b[1])) / d
    uy = (a2 * (c[0] - b[0]) + b2 * (a[0] - c[0]) + c2 * (b[0] - a[0])) / d
    return (ux, uy)


def min_circle(points):
    """The least enclosing circle exactly (Welzl's algorithm on fractions; any order finds the one circle)."""
    p = [(F(x), F(y)) for x, y in set((q[0] + 0.0, q[1] + 0.0) for q in points)]
    random.Random(1).shuffle(p)

    def d2(a, b):
        return (a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2

    c, r2 = p[0], F(0)
    for i in range(1, len(p)):
        if d2(c, p[i]) <= r2:
            continue
        c, r2 = p[i], F(0)
        for j in range(i):
            if d2(c, p[j]) <= r2:
                continue
            c = ((p[i][0] + p[j][0]) / 2, (p[i][1] + p[j][1]) / 2)
            r2 = d2(c, p[i])
            for k in range(j):
                if d2(c, p[k]) <= r2:
                    continue
                c = circle3(p[i], p[j], p[k])
                r2 = d2(c, p[i])
    r = float(mp.sqrt(mp.mpf(r2.numerator) / r2.denominator))
    got = shapely.minimum_bounding_radius(MultiPoint([(float(x), float(y)) for x, y in p]))
    assert abs(got - r) <= 1e-7, ("GEOS'un dairesi başka", got, r)
    return (float(c[0]), float(c[1])), r


# ── Curves as points (§4) ─────────────────────────────────────────────────


def arc_points(c, r, a0, sweep, tol):
    """The ADR's chord step (each chord's sagitta at most tol, at least 8 a turn, at most 4096 chords), ends included."""
    sw = abs(sweep)
    if sw == 0 or r <= 0:
        n = 1
    else:
        step = math.pi / 4 if tol >= r else min(2 * math.acos(1 - tol / r), math.pi / 4)
        n = max(1, min(4096, math.ceil(sw / step)))
    return [(c[0] + r * math.cos(a0 + sweep * i / n), c[1] + r * math.sin(a0 + sweep * i / n)) for i in range(n + 1)]


def on_sweep(t, a0, sweep):
    if abs(sweep) >= 2 * math.pi:
        return True
    if sweep >= 0:
        return (t - a0) % (2 * math.pi) <= sweep
    return (a0 - t) % (2 * math.pi) <= -sweep


def extremes(c, r, a0, sweep):
    at = [(0.0, (c[0] + r, c[1])), (math.pi / 2, (c[0], c[1] + r)), (math.pi, (c[0] - r, c[1])), (3 * math.pi / 2, (c[0], c[1] - r))]
    return [p for t, p in at if on_sweep(t, a0, sweep)]


def bulge_arc(a, b, bulge):
    dx, dy = b[0] - a[0], b[1] - a[1]
    chord = math.hypot(dx, dy)
    k = (1 - bulge * bulge) / (4 * bulge)
    c = ((a[0] + b[0]) / 2 - dy * k, (a[1] + b[1]) / 2 + dx * k)
    r = (chord * (1 + bulge * bulge)) / (4 * abs(bulge))
    return c, r, math.atan2(a[1] - c[1], a[0] - c[0]), 4 * math.atan(bulge)


def ring_points(pts, bulges, closed):
    """A ring's or a path's vertices, its arcs' inner points and extremes."""
    res = []
    n = len(pts)
    edges = n if closed else n - 1
    for i in range(n):
        res.append(pts[i])
        b = bulges[i] if bulges and i < len(bulges) else 0
        if i < edges and b:
            c, r, a0, sw = bulge_arc(pts[i], pts[(i + 1) % n], b)
            inner = arc_points(c, r, a0, sw, CURVE_TOL)
            res.extend(inner[1:-1])
            res.extend(extremes(c, r, a0, sw))
    return res


def outline(e):
    """An object's points (§4)."""
    k = e["kind"]
    if k == "point":
        return [xy(e["p"])] + [xy(q["p"]) for q in e.get("parts", [])]
    if k == "line":
        return [xy(e["a"]), xy(e["b"])]
    if k in ("polygon", "polyline"):
        res = []
        for part in [e, *e.get("parts", [])]:
            res.extend(ring_points([xy(p) for p in part["pts"]], part.get("bulges"), k == "polygon"))
            for h in part.get("holes", []):
                res.extend(ring_points([xy(p) for p in h["pts"]], h.get("bulges"), True))
        return res
    if k == "circle":
        c = xy(e["c"])
        return arc_points(c, e["r"], 0.0, 2 * math.pi, CURVE_TOL) + extremes(c, e["r"], 0.0, 2 * math.pi)
    if k == "arc":
        c = xy(e["c"])
        sw = (e["a1"] - e["a0"]) % (2 * math.pi) or 2 * math.pi
        return arc_points(c, e["r"], e["a0"], sw, CURVE_TOL) + extremes(c, e["r"], e["a0"], sw)
    raise ValueError(k)


def curved(e):
    k = e["kind"]
    if k in ("circle", "arc", "ellipse", "spline"):
        return True
    if k in ("polygon", "polyline"):
        return any(b for part in [e, *e.get("parts", [])] for r in [part, *part.get("holes", [])] for b in (r.get("bulges") or []))
    return False


# ── İçbükey zarf (§5) ─────────────────────────────────────────────────────


def length(a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    return math.sqrt(dx * dx + dy * dy)


def concave(points, ratio, holes):
    """The ADR's wearing away over the exact triangulation; the rings outer first, holes by their lowest corner."""
    places, _ = distinct(points)
    tris = delaunay(places)
    if not tris:
        return None
    pt = places
    # Edges to their triangles.
    owner = {}
    for t, (a, b, c) in enumerate(tris):
        for u, v in ((a, b), (b, c), (c, a)):
            owner[(u, v)] = t
    alive = [True] * len(tris)
    hull_vertices = set()
    for t in tris:
        for u, v in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])):
            if (v, u) not in owner:
                hull_vertices.update((u, v))
    around = {}
    for t, tri in enumerate(tris):
        for v in tri:
            around.setdefault(v, []).append(t)

    def across(t, i):
        a, b = tris[t][i], tris[t][(i + 1) % 3]
        o = owner.get((b, a))
        return o if o is not None and alive[o] else None

    def count(t):
        return sum(across(t, i) is not None for i in range(3))

    def interior(v):
        return v not in hull_vertices and all(alive[t] for t in around[v])

    def connecting(t):
        has = [across(t, i) is not None for i in range(3)]
        for i in range(3):
            if has[i] and has[(i + 1) % 3]:
                return not interior(tris[t][(i + 1) % 3])
        return False

    def touches(t):
        for k in range(3):
            if across(t, k) is not None and across(t, (k + 2) % 3) is not None and not interior(tris[t][k]):
                return True
        return False

    def area(t):
        a, b, c = (pt[v] for v in tris[t])
        return abs((c[0] - a[0]) * (b[1] - a[1]) - (b[0] - a[0]) * (c[1] - a[1])) / 2

    def border(t):
        return sum(length(pt[tris[t][i]], pt[tris[t][(i + 1) % 3]]) for i in range(3) if across(t, i) is None)

    lens = [length(pt[t[i]], pt[t[(i + 1) % 3]]) for t in tris for i in range(3)]
    if ratio <= 0:
        target = 0.0
    elif ratio >= 1:
        target = 2 * max(lens)
    else:
        target = ratio * (max(lens) - min(lens)) + min(lens)
    size = [max(length(pt[t[i]], pt[t[(i + 1) % 3]]) for i in range(3)) for t in tris]

    def push(q, t):
        if alive[t] and count(t) == 2:
            size[t] = border(t)
            heapq.heappush(q, (-size[t], -area(t), t))

    def remove(q, t):
        near = [across(t, i) for i in range(3)]
        alive[t] = False
        for n in near:
            if n is not None:
                push(q, n)

    q = []
    for t in range(len(tris)):
        push(q, t)
    while q:
        s, _, t = heapq.heappop(q)
        if -s < target:
            break
        if not alive[t] or count(t) != 2 or connecting(t):
            continue
        remove(q, t)
    if holes:
        seeds = [t for t in range(len(tris)) if alive[t] and size[t] >= target and count(t) == 3 and not touches(t)]
        seeds.sort(key=lambda t: (-size[t], -area(t), t))
        for seed in seeds:
            if not alive[seed] or count(seed) != 3 or touches(seed):
                continue
            q = []
            remove(q, seed)
            while q:
                s, _, t = heapq.heappop(q)
                if -s < target:
                    break
                if not alive[t] or count(t) != 2 or touches(t):
                    continue
                remove(q, t)
    nxt = {}
    for t in range(len(tris)):
        if not alive[t]:
            continue
        for i in range(3):
            if across(t, i) is None:
                nxt.setdefault(tris[t][i], tris[t][(i + 1) % 3])
    rings, seen = [], set()
    for s in sorted(nxt):
        if s in seen:
            continue
        ring, v = [], s
        while v not in seen and v in nxt:
            seen.add(v)
            ring.append(pt[v])
            v = nxt[v]
        if len(ring) >= 3:
            rings.append(normalise(ring))
    outer = [r for r in rings if ring_area_q(r) > 0]
    inner = sorted((r for r in rings if ring_area_q(r) < 0), key=lambda r: (r[0][1], r[0][0]))
    return outer + inner


def geos_concave_check(points, ratio, holes, rings):
    """GEOS's hull against ours (GEOS takes hole seeds smallest first: the same here, each region one hole)."""
    got = shapely.concave_hull(MultiPoint(points), ratio=ratio, allow_holes=holes)
    want = Polygon(rings[0], rings[1:])
    assert got.symmetric_difference(want).area < 1e-6, ("GEOS'un içbükey zarfı başka", got.area, want.area)


# ── Ulaşılmazlık kutbu (§6) ──────────────────────────────────────────────


def seg_d2(p, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    x, y = a
    if dx != 0 or dy != 0:
        t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)
        if t > 1:
            x, y = b
        elif t > 0:
            x += dx * t
            y += dy * t
    ex, ey = p[0] - x, p[1] - y
    return ex * ex + ey * ey


def signed_distance(p, rings):
    least, inside = math.inf, False
    for ring in rings:
        n = len(ring)
        if n < 2:
            continue
        j = n - 1
        for i in range(n):
            a, b = ring[i], ring[j]
            least = min(least, seg_d2(p, a, b))
            if n >= 3 and (a[1] > p[1]) != (b[1] > p[1]) and p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]:
                inside = not inside
            j = i
    d = math.sqrt(least)
    return d if inside else -d


def centroid(ring):
    area = x = y = 0.0
    n = len(ring)
    j = n - 1
    for i in range(n):
        a, b = ring[i], ring[j]
        f = a[0] * b[1] - b[0] * a[1]
        x += (a[0] + b[0]) * f
        y += (a[1] + b[1]) * f
        area += f * 3
        j = i
    return ring[0] if area == 0 else (x / area, y / area)


def polylabel(rings, precision):
    """Cells of a square grid halved while their potential (distance plus half their diagonal) can pass the best;
    the greatest potential first, of equal ones the earliest made."""
    outer = rings[0]
    x0, y0 = min(p[0] for p in outer), min(p[1] for p in outer)
    x1, y1 = max(p[0] for p in outer), max(p[1] for p in outer)
    w, h = x1 - x0, y1 - y0
    size = min(w, h)
    if not size > 0:
        return None
    seq = [0]

    def make(c, half):
        seq[0] += 1
        d = signed_distance(c, rings)
        return (d + half * math.sqrt(2), seq[0], c, half, d)

    heap = []
    half = size / 2
    x = x0
    while x < x1:
        y = y0
        while y < y1:
            cell = make((x + half, y + half), half)
            heapq.heappush(heap, (-cell[0], cell[1], cell))
            y += size
        x += size
    best = make(centroid(outer), 0.0)
    middle = make((x0 + w / 2, y0 + h / 2), 0.0)
    if middle[4] > best[4]:
        best = middle
    while heap:
        _, _, cell = heapq.heappop(heap)
        if cell[4] > best[4]:
            best = cell
        if cell[0] - best[4] <= precision:
            continue
        hh = cell[3] / 2
        for sx, sy in ((-1, -1), (1, -1), (-1, 1), (1, 1)):
            child = make((cell[2][0] + sx * hh, cell[2][1] + sy * hh), hh)
            if child[0] > best[4]:
                heapq.heappush(heap, (-child[0], child[1], child))
    return (best[2], best[4]) if best[4] > 0 else None


def area_rings(e):
    """An area's parts as rings (outer first, holes), arcs as points within 0.1 mm."""
    if e["kind"] != "polygon":
        raise ValueError(e["kind"])
    parts = []
    for part in [e, *e.get("parts", [])]:
        rings = [arc_ring([xy(p) for p in part["pts"]], part.get("bulges"))]
        for h in part.get("holes", []):
            rings.append(arc_ring([xy(p) for p in h["pts"]], h.get("bulges")))
        parts.append(rings)
    return parts


def arc_ring(pts, bulges):
    res = []
    n = len(pts)
    for i in range(n):
        res.append(pts[i])
        b = bulges[i] if bulges and i < len(bulges) else 0
        if b:
            c, r, a0, sw = bulge_arc(pts[i], pts[(i + 1) % n], b)
            res.extend(arc_points(c, r, a0, sw, CURVE_TOL)[1:-1])
    return res


def pole(e, precision):
    if e["kind"] == "circle":
        return (xy(e["c"]), e["r"]) if e["r"] > 0 else None
    best = None
    for rings in area_rings(e):
        got = polylabel(rings, precision)
        if got and (best is None or got[1] > best[1]):
            best = got
        if got:
            poly = Polygon(rings[0], rings[1:])
            mic = shapely.maximum_inscribed_circle(poly, precision / 100)
            r = mic.length
            assert r - precision - 1e-9 <= got[1] <= r + precision / 100 + 1e-9, ("GEOS'un kutbu başka", got[1], r)
    return best


# ── The drawing ──────────────────────────────────────────────────────────


def layer(i, name, color, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25}, "children": []}


LAYERS = [
    layer("cizim", "Çizim", "fg"),
    layer("nokta", "Nokta", "#0090FF"),
    layer("hat", "Hat", "#AB4ABA"),
    layer("parsel", "Parsel", "#E5484D"),
    layer("diger", "Diğer", "#F76B15"),
    layer("kume", "Küme", "#12A594"),
    layer("halka", "Halka", "#3E63DD"),
    layer("kilitli", "Arşiv", "#8E8C99", locked=True),
]


def ent(i, kind, lay, attrs, **geom):
    e = {"kind": kind, "id": i, "layerId": lay, "attrs": attrs}
    e.update(geom)
    return e


# Survey points in general position (no three on a line, no four on a circle, checked exactly below), two kinds, the
# first nine with elevations; a multi-point object of three; one more in N3's place.
SITES = [(229.51, 22.31), (216.74, 1.8), (228.8, 31.02), (242.32, 34.28), (220.72, 42.7), (236.29, 38.67),
         (228.32, 1.18), (213.81, 40.03), (214.52, 8.77), (253.8, 32.05), (236.6, 0.73), (215.45, 18.8),
         (221.03, 33.0), (218.27, 38.23), (230.54, 30.97), (241.35, 3.09), (252.22, 15.39), (221.91, 14.41),
         (252.47, 36.36), (213.13, 5.28), (213.81, 27.94)]
# A C of points (open to the east) and an O round an empty middle, in general position.
C_SHAPE = [(324.55, 42.91), (316.4, 34.69), (314.38, 40.21), (339.5, 3.79), (314.82, 15.14), (316.95, 45.07),
           (339.27, 5.9), (333.71, 10.95), (320.03, 3.8), (344.59, 39.76), (315.41, 38.93), (335.81, 45.16),
           (326.11, 43.92), (328.0, 2.65), (317.59, 19.62), (312.53, 15.08), (338.17, 43.02), (319.25, 44.21),
           (310.2, 25.36), (329.88, 43.65), (325.2, 44.38), (335.6, 39.49), (341.48, 6.55), (307.32, 21.09),
           (337.6, 9.64), (345.8, 7.03), (334.51, 47.09), (343.94, 44.29), (314.73, 18.4), (324.0, 36.72),
           (316.79, 7.07), (315.85, 13.78), (315.56, 12.23), (313.55, 32.8), (314.41, 39.21), (315.4, 14.95),
           (311.0, 21.24), (318.02, 18.13), (329.06, 8.13), (308.29, 16.22)]
O_SHAPE = [(417.59, 34.65), (415.97, 15.66), (386.75, 38.29), (393.64, 3.66), (385.31, 39.26), (421.86, 20.11),
           (389.23, 35.59), (382.09, 15.1), (388.0, 37.43), (393.34, 40.79), (382.81, 11.05), (416.97, 20.15),
           (405.59, 4.27), (394.91, 8.54), (383.66, 11.99), (400.88, 2.05), (384.96, 23.31), (397.94, 3.26),
           (390.58, 44.04), (421.3, 17.29), (397.9, 3.88), (413.28, 43.38), (417.75, 29.25), (385.5, 37.85),
           (419.18, 20.7), (405.32, 4.47), (420.98, 16.74), (415.93, 22.77), (403.25, 5.57), (421.88, 18.4),
           (401.86, 47.03), (422.39, 26.9), (400.79, 5.94), (418.73, 34.42), (414.32, 35.65), (389.19, 10.02),
           (411.68, 38.21), (400.36, 47.67), (389.01, 42.67), (409.12, 12.26), (406.58, 5.59), (405.15, 9.72),
           (418.44, 36.67), (386.7, 16.24), (398.43, 4.73), (379.06, 27.18), (397.59, 42.9), (381.83, 18.73)]

ENTITIES = []
for k, (x, y) in enumerate(SITES[:18], start=1):
    e = ent(k, "point", "nokta", {"No": f"N{k}", "Tür": "A" if k % 3 else "B"}, p=P(x, y))
    if k <= 9:
        e["z"] = round(100 + 1.25 * k, 2)
    ENTITIES.append(e)
ENTITIES.append(ent(19, "point", "nokta", {"No": "Ç", "Tür": "B"}, p=P(*SITES[18]), parts=[{"p": P(*SITES[19])}, {"p": P(*SITES[20])}]))
ENTITIES.append(ent(20, "point", "nokta", {"No": "N3b", "Tür": "A"}, p=P(*SITES[2])))
ENTITIES += [
    # Three points exactly on one line.
    ent(31, "point", "hat", {"No": "H1"}, p=P(10, 60)),
    ent(32, "point", "hat", {"No": "H2"}, p=P(20, 65)),
    ent(33, "point", "hat", {"No": "H3"}, p=P(30, 70)),
    # Parcels: a square, an L, a triangle, one with a hole, a two-part one, a turned rectangle, one with an arc on top.
    ent(41, "polygon", "parsel", {"Ada": "101", "Parsel": "1"}, pts=rect(0, 0, 20, 20)),
    ent(42, "polygon", "parsel", {"Ada": "101", "Parsel": "2"}, pts=[P(20, 0), P(50, 0), P(50, 4), P(24, 4), P(24, 30), P(20, 30)]),
    ent(43, "polygon", "parsel", {"Ada": "102", "Parsel": "3"}, pts=[P(60, 0), P(90, 3), P(72, 25)]),
    ent(44, "polygon", "parsel", {"Ada": "102", "Parsel": "4"}, pts=rect(100, 0, 130, 30), zs=[10, 11, 12, 13],
        holes=[{"pts": rect(110, 10, 120, 20)}]),
    ent(45, "polygon", "parsel", {"Ada": "103", "Parsel": "5"}, pts=rect(140, 0, 150, 10), parts=[{"pts": rect(160, 0, 180, 12)}]),
    ent(46, "polygon", "parsel", {"Ada": "103", "Parsel": "6"}, pts=[P(0, 40), P(18, 49), P(13, 59), P(-5, 50)]),
    ent(47, "polygon", "parsel", {"Ada": "104", "Parsel": "7"}, pts=rect(30, 40, 50, 55), bulges=[0, 0, 0.25, 0]),
    # Other kinds: a circle, an arc, a polyline, a line, a point.
    ent(51, "circle", "diger", {"Ad": "Kuyu"}, c=P(80, 50), r=6),
    ent(52, "arc", "diger", {"Ad": "Kavis"}, c=P(100, 50), r=8, a0=0.3, a1=2.2),
    ent(53, "polyline", "diger", {"Ad": "Yol"}, pts=[P(60, 70), P(75, 62), P(90, 72)]),
    ent(54, "line", "diger", {"Ad": "Hat"}, a=P(100, 65), b=P(115, 75)),
    ent(55, "point", "diger", {"Ad": "Direk"}, p=P(120, 60)),
]
for k, (x, y) in enumerate(C_SHAPE, start=1):
    ENTITIES.append(ent(60 + k, "point", "kume", {"No": f"K{k}"}, p=P(x, y)))
for k, (x, y) in enumerate(O_SHAPE, start=1):
    ENTITIES.append(ent(120 + k, "point", "halka", {"No": f"O{k}"}, p=P(x, y)))
ENTITIES += [
    ent(111, "polygon", "kilitli", {"Parsel": "9"}, pts=[P(0, 80), P(15, 80), P(15, 90), P(0, 92)]),
    ent(112, "point", "kilitli", {"No": "A1"}, p=P(20, 85)),
]

DOCUMENT = {
    "format": "kentos.document",
    "version": 1,
    "name": "İşlem durumları: geometri üretimi",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                 "workspace": "gis", "drawingFont": "barlow"},
    "origin": {"x": E, "y": N},
    "layers": LAYERS,
    "activeLayer": "cizim",
    "entities": ENTITIES,
    "styles": {"items": [], "categories": []},
}


def general(points):
    q = [ints(p) for p in points]
    assert all(orient(a, b, c) != 0 for a, b, c in combinations(q, 3)), "üç nokta bir doğruda"
    for a, b, c, d in combinations(q, 4):
        if orient(a, b, c) < 0:
            b, c = c, b
        assert in_circle(a, b, c, d) != 0, "dört nokta bir çemberde"


# ── The cases ────────────────────────────────────────────────────────────

OUTPUT = {
    "generate.voronoi": ("Voronoi", "#3E63DD", "Voronoi alanları"),
    "generate.delaunay": ("Üçgenler", "#12A594", "Delaunay üçgenleri"),
    "generate.bounding": ("Sınırlayıcı", "#F76B15", "Sınırlayıcı geometri"),
    "generate.concaveHull": ("İçbükey zarf", "#8E4EC6", "İçbükey zarf"),
    "generate.pole": ("Kutuplar", "#E54666", "Ulaşılmazlık kutbu"),
}
WHAT = {"hull": "dışbükey zarf", "box": "sınır kutusu", "rotated": "döndürülmüş kutu", "circle": "daire"}


def by_id(i):
    return next(e for e in ENTITIES if e["id"] == i)


def objects(lay):
    return [e for e in ENTITIES if e["layerId"] == lay]


def sel(*ids):
    return [by_id(i) for i in ids]


def L(i):
    return {"scope": "layer", "layerId": i}


def out_layer(tool):
    name, color, _ = OUTPUT[tool]
    return {"id": new_id(name, set()), "name": name,
            "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25, "fill": f"{color}26"}}


def case(cid, title, tool, values, expect, selection=None):
    c = {"id": cid, "title": title, "document": "generate.kcad", "run": {"tool": tool}, "values": values, "expect": expect}
    if selection is not None:
        c["selection"] = selection
    return c


def ok(tool, added, summary, log, count):
    e = {"status": "ok", "summary": summary, "undo": OUTPUT[tool][2] if added else None}
    if added:
        e["layers"] = [out_layer(tool)]
        e["added"] = added
    if log:
        e["log"] = log
    e["outputs"] = {"count": count}
    return e


def warn(text):
    return {"level": "warn", "text": text}


def has_elevation(e):
    k = e["kind"]
    if k == "point":
        return e.get("z") is not None or any(q.get("z") is not None for q in e.get("parts", []))
    if k == "line":
        return e.get("za") is not None or e.get("zb") is not None
    rings = [e, *e.get("holes", []), *e.get("parts", []), *[h for p in e.get("parts", []) for h in p.get("holes", [])]]
    return any(z is not None for r in rings for z in (r.get("zs") or []))


def polygon(ring, lay, attrs, holes=(), zs=None):
    o = {"kind": "polygon", "layerId": lay, "attrs": attrs, "pts": [out(p) for p in ring]}
    if holes:
        o["holes"] = [{"pts": [out(p) for p in h]} for h in holes]
    if zs is not None:
        o["zs"] = zs
    return o


def sites_of(inputs):
    """The tools' sites: each point of each object in order, with its object, its part (1, 2 … for a multi-point one)."""
    res = []
    for e in inputs:
        pts = [e] + e.get("parts", [])
        for k, q in enumerate(pts, start=1):
            res.append((xy(q["p"]), e, k if len(pts) > 1 else None, q.get("z")))
    return res


def length_text(v):
    return shown(v, 3)


def area_text(v):
    return shown(v, 2)


def bearing_text(t):
    return shown((t * 200) / math.pi, 4)


def voronoi(tool, inputs, margin):
    s = sites_of(inputs)
    places, first = distinct([p for p, *_ in s])
    if len(places) < 2:
        return {"status": "error", "message": "Voronoi alanları için en az iki ayrı yerde nokta gerekir."}
    frame = frame_of([p for p, *_ in s], margin / 100)
    cells = voronoi_cells(places, frame)
    geos_voronoi_check(places, frame, cells)
    lay = out_layer(tool)["id"]
    place_of = {}
    for k, i in enumerate(first):
        place_of[i] = k
    added, same, empty = [], 0, 0
    for i, (p, e, part, _) in enumerate(s):
        if i not in place_of:
            same += 1
            continue
        cell = cells[place_of[i]]
        if cell is None:
            empty += 1
            continue
        attrs = dict(e["attrs"])
        if part is not None:
            attrs["Parça"] = str(part)
        added.append(polygon(cell, lay, attrs))
    log = []
    if same:
        log.append(warn(f"{same} nokta başka bir noktayla aynı yerde; alanı yazılmadı."))
    heights = sum(1 for e in inputs if has_elevation(e))
    if heights:
        log.append(warn(f"{heights} nesnenin kotları sonuca taşınmadı."))
    if empty:
        log.append(warn(f"{empty} noktanın alanı çerçevenin içinde alan değil; yazılmadı."))
    return ok(tool, added, f"{len(s)} noktadan {len(added)} alan yazıldı.", log, len(added))


def name_of(e, field, place):
    """An object's name (docs/adr/0215 §2.3): its field's value, else its label, else its place in the input from 1."""
    v = e["attrs"].get(field, "").strip() if field else ""
    if v:
        return v
    return (e.get("label") or "").strip() or str(place + 1)


def delaunay_case(tool, inputs, field):
    s = sites_of(inputs)
    places, first = distinct([p for p, *_ in s])
    tris = delaunay(places)
    if len(places) < 3 or not tris:
        return {"status": "error", "message": "Üçgenleme için doğru üzerinde olmayan en az üç nokta gerekir."}
    geos_delaunay_check(places, tris)
    names = []
    for p, e, part, _ in s:
        n = name_of(e, field, inputs.index(e))
        names.append(f"{n}/{part}" if part is not None else n)
    lay = out_layer(tool)["id"]
    triples = []
    for t in tris:
        sid = [first[v] for v in t]
        k = min(range(3), key=lambda j: sid[j])
        triples.append(tuple(sid[k:] + sid[:k]))
    added = []
    for t in sorted(triples):
        zs = [s[i][3] for i in t]
        attrs = {f"Nokta {j + 1}": names[t[j]] for j in range(3)}
        added.append(polygon([s[i][0] for i in t], lay, attrs, zs=zs if any(z is not None for z in zs) else None))
    same = len(s) - len(places)
    log = [warn(f"{same} nokta başka bir noktayla aynı yerde; üçgenlere bir kez girdi.")] if same else []
    return ok(tool, added, f"{len(s)} noktadan {len(added)} üçgen yazıldı.", log, len(added))


def grouping(inputs, group, field):
    """The groups (each object, all, by a field's value in the order of its first object) and their attributes."""
    if group == "each":
        return [([e], dict(e["attrs"])) for e in inputs]
    if group == "field" and field:
        keys, members = [], {}
        for e in inputs:
            v = e["attrs"].get(field, "")
            if v not in members:
                keys.append(v)
                members[v] = []
            members[v].append(e)
        return [(members[k], {field: k, "Nesne sayısı": str(len(members[k]))}) for k in keys]
    return [(list(inputs), {"Nesne sayısı": str(len(inputs))})]


def grouped_summary(inputs, groups, group, k, what):
    if group == "each":
        return f"{len(inputs)} nesneden {k} {what} yazıldı."
    return f"{len(inputs)} nesne {len(groups)} grupta; {k} {what} yazıldı."


def common_log(inputs, empty, circles=True):
    log = []
    bent = sum(1 for e in inputs if curved(e) and (circles or e["kind"] != "circle"))
    if bent:
        log.append(warn(f"{bent} nesnenin yayları ya da eğrileri 0,1 mm içinde noktalarla alındı."))
    heights = sum(1 for e in inputs if has_elevation(e))
    if heights:
        log.append(warn(f"{heights} nesnenin kotları sonuca taşınmadı."))
    return log


def bounding(tool, inputs, kind, group, field=""):
    groups = grouping(inputs, group, field)
    lay = out_layer(tool)["id"]
    added, empty = [], 0
    for members, attrs in groups:
        pts = [p for e in members for p in outline(e)]
        attrs = dict(attrs)
        if kind == "hull":
            h = hull(pts)
            area = ring_area_q(h) if len(h) >= 3 else 0
            if area <= 0:
                empty += 1
                continue
            attrs["Zarf alanı"] = area_text(float(area))
            added.append(polygon(h, lay, attrs))
        elif kind == "box":
            b = axis_box(pts)
            if b is None:
                empty += 1
                continue
            attrs["Kutu genişliği"] = length_text(b["width"])
            attrs["Kutu yüksekliği"] = length_text(b["height"])
            added.append(polygon(b["ring"], lay, attrs))
        elif kind == "rotated":
            b = rotated_box(pts)
            if b is None:
                empty += 1
                continue
            attrs["Kutu uzunluğu"] = length_text(b["length"])
            attrs["Kutu genişliği"] = length_text(b["width"])
            attrs["Kutu semti"] = bearing_text(b["bearing"])
            added.append(polygon(b["ring"], lay, attrs))
        else:
            c, r = min_circle(pts)
            if not r > 0:
                empty += 1
                continue
            attrs["Daire yarıçapı"] = length_text(r)
            added.append({"kind": "circle", "layerId": lay, "attrs": attrs, "c": out(c), "r": r})
    log = common_log(inputs, empty)
    if empty:
        log.append(warn(f"{empty} sonuç alan değil (bir nokta ya da bir doğru); yazılmadı."))
    return ok(tool, added, grouped_summary(inputs, groups, group, len(added), WHAT[kind]), log, len(added))


def concave_case(tool, inputs, ratio, holes, group, field="", check=True):
    groups = grouping(inputs, group, field)
    lay = out_layer(tool)["id"]
    added, empty = [], 0
    for members, attrs in groups:
        pts = [p for e in members for p in outline(e)]
        rings = concave(pts, ratio, holes)
        net = sum(ring_area_q(r) for r in rings) if rings else 0
        if not rings or net <= 0:
            empty += 1
            continue
        if check:
            geos_concave_check(pts, ratio, holes, rings)
        attrs = dict(attrs)
        attrs["Zarf alanı"] = area_text(float(net))
        added.append(polygon(rings[0], lay, attrs, holes=rings[1:]))
    log = common_log(inputs, empty)
    if empty:
        log.append(warn(f"{empty} sonuç alan değil (bir nokta ya da bir doğru); yazılmadı."))
    return ok(tool, added, grouped_summary(inputs, groups, group, len(added), "içbükey zarf"), log, len(added))


def poles(tool, inputs, precision):
    lay = out_layer(tool)["id"]
    added, empty = [], 0
    for e in inputs:
        got = pole(e, precision)
        if got is None:
            empty += 1
            continue
        attrs = dict(e["attrs"])
        attrs["Uzaklık"] = length_text(got[1])
        added.append({"kind": "point", "layerId": lay, "attrs": attrs, "p": out(got[0])})
    log = common_log(inputs, empty, circles=False)
    if empty:
        log.append(warn(f"{empty} nesnenin alanı yok; kutbu yazılmadı."))
    return ok(tool, added, f"{len(inputs)} nesneden {len(added)} nokta yazıldı.", log, len(added))


def build():
    general([xy(e["p"]) for e in objects("nokta") if e["id"] != 20] + [xy(q["p"]) for q in by_id(19)["parts"]])
    general([xy(e["p"]) for e in objects("kume")])
    general([xy(e["p"]) for e in objects("halka")])
    nokta, kume, parsel, diger = objects("nokta"), objects("kume"), objects("parsel"), objects("diger")
    v, d, b, c, p = "generate.voronoi", "generate.delaunay", "generate.bounding", "generate.concaveHull", "generate.pole"
    cases = [
        case("voronoi-sites", "Voronoi alanları: noktaların çerçevesi %10 büyür; çok noktalı nesnenin her noktası, aynı yerdeki ikinci nokta, kotlar",
             v, {"input": L("nokta"), "margin": 10}, voronoi(v, nokta, 10)),
        case("voronoi-tight", "Genişletme 0: alanlar noktaların kutusunda, kutunun kenarındaki noktalar",
             v, {"input": {"scope": "selection"}, "margin": 0}, voronoi(v, sel(*range(1, 11)), 0), selection=list(range(1, 11))),
        case("voronoi-line", "Bir doğrudaki noktalar şeritler verir",
             v, {"input": L("hat"), "margin": 25}, voronoi(v, objects("hat"), 25)),
        case("voronoi-one", "Tek yerde nokta: reddedilir",
             v, {"input": {"scope": "selection"}, "margin": 10}, voronoi(v, sel(3, 20), 10), selection=[3, 20]),
        case("delaunay-named", "Delaunay üçgenleri: adlar No alanından, çok noktalı nesnenin noktaları ad/sıra, kotlar köşelerde",
             d, {"input": L("nokta"), "name": "No"}, delaunay_case(d, nokta, "No")),
        case("delaunay-plain", "Ad alanı yokken adlar girdideki sıralar",
             d, {"input": L("kume"), "name": ""}, delaunay_case(d, kume, "")),
        case("delaunay-line", "Bir doğrudaki noktalar üçgen vermez: reddedilir",
             d, {"input": L("hat"), "name": ""}, delaunay_case(d, objects("hat"), "")),
        case("bounding-hull", "Her parselin dışbükey zarfı: L, delik, iki parça, yay",
             b, {"input": L("parsel"), "kind": "hull", "group": "each", "field": ""}, bounding(b, parsel, "hull", "each")),
        case("bounding-box-all", "Bütün türlerin tek sınır kutusu: dairenin ve yayın uçları kesin",
             b, {"input": L("diger"), "kind": "box", "group": "all", "field": ""}, bounding(b, diger, "box", "all")),
        case("bounding-rotated-ada", "Adalara göre döndürülmüş kutular",
             b, {"input": L("parsel"), "kind": "rotated", "group": "field", "field": "Ada"}, bounding(b, parsel, "rotated", "field", "Ada")),
        case("bounding-circle", "Her nesnenin en küçük dairesi: daire kendisi, noktanın dairesi yok",
             b, {"input": L("diger"), "kind": "circle", "group": "each", "field": ""}, bounding(b, diger, "circle", "each")),
        case("bounding-locked", "Kilitli katmanın nesneleri okunur: zarflar yeni katmana",
             b, {"input": L("kilitli"), "kind": "hull", "group": "each", "field": ""}, bounding(b, objects("kilitli"), "hull", "each")),
        case("concave-c", "İçbükey zarf, oran 0,3: C biçimli küme",
             c, {"input": L("kume"), "ratio": 0.3, "holes": False, "group": "all", "field": ""}, concave_case(c, kume, 0.3, False, "all")),
        case("concave-holes", "İçbükey zarf, oran 0,5, delikler açık: halkanın ortası delik",
             c, {"input": L("halka"), "ratio": 0.5, "holes": True, "group": "all", "field": ""},
             concave_case(c, objects("halka"), 0.5, True, "all")),
        case("concave-filled", "Aynı halka delikler kapalıyken: ortası dolu",
             c, {"input": L("halka"), "ratio": 0.5, "holes": False, "group": "all", "field": ""},
             concave_case(c, objects("halka"), 0.5, False, "all")),
        case("concave-kinds", "Türlerine göre içbükey zarflar, oran 0,5",
             c, {"input": L("nokta"), "ratio": 0.5, "holes": False, "group": "field", "field": "Tür"}, concave_case(c, nokta, 0.5, False, "field", "Tür")),
        case("concave-line", "Bir doğrudaki noktalar alan vermez",
             c, {"input": L("hat"), "ratio": 0.3, "holes": False, "group": "all", "field": ""}, concave_case(c, objects("hat"), 0.3, False, "all")),
        case("pole-parcels", "Parsellerin ulaşılmazlık kutupları, 0,1 m: L, delik, iki parça, yay",
             p, {"input": L("parsel"), "precision": 0.1}, poles(p, parsel, 0.1)),
        case("pole-fine", "İnce L parselin kutbu, 1 mm",
             p, {"input": {"scope": "selection"}, "precision": 0.001}, poles(p, sel(42), 0.001), selection=[42]),
        case("pole-circle", "Dairenin kutbu merkezi",
             p, {"input": L("diger"), "precision": 0.1}, poles(p, [e for e in diger if e["kind"] == "circle"], 0.1)),
    ]
    return {"format": "kentos.processing-cases", "version": 1, "tolerance": TOL, "toleranceFields": ["r", "zs"],
            "documents": {}, "cases": cases}


def main():
    doc = json.dumps(DOCUMENT, ensure_ascii=False, indent=1) + "\n"
    proc = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    outs = [(PROC_DIR / "generate.json", proc), (PROC_DIR / "generate.kcad", doc)]
    if "--check" in sys.argv[1:]:
        bad = [p for p, t in outs if not p.exists() or p.read_text() != t]
        for p in bad:
            print(f"{p.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
        if bad:
            return 1
        print("geometri üretiminin durumları tutarlı: fixtures/processing/v1/generate*")
        return 0
    for p, t in outs:
        p.write_text(t)
        print(f"yazıldı: {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
