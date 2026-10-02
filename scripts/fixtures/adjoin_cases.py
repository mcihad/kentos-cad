#!/usr/bin/env python3
"""Independent reference of Bitişik alan and the overlap control (docs/adr/0162).

Writes fixtures/adjoin/v1/cases.json from the ADR's rules alone, with
Python's standard library (and mpmath for the arc cases), no KentOS code.
The geometry core (`ops::adjoin`, crates/shared/geometry-core/tests/adjoin.rs)
and the web through its WASM (apps/web/src/model/ops/adjoin.test.ts) must
give the same: a vertex that is an input vertex bit for bit, any other
within 1e-9 m; bulges within 1e-12; areas within 1e-9 relative.

avoid(area, neighbours) (§2): the neighbours whose inside meets the new
area's inside, by their places, and the new area less them. When none does,
the area itself, as given; a neighbour that only touches it cuts nothing
and adds no corner.

fill(path, neighbours) (§3): the bounded faces of the arrangement of the
path and the neighbours' rings that lie outside every neighbour and whose
own outer walk runs along the path on a piece shared with another face;
their union, a neighbour inside it a hole.

Worked out on exact rationals (straight edges):

1. The arrangement: every segment is cut where another meets it; pieces
   lying on top of each other are one; a piece knows whether the path runs
   along it.
2. Walks: every piece both ways, each walk keeping its face on the left (at
   a vertex the walk turns into the piece next clockwise from the way back).
   A walk of positive area is a bounded face; any other, the outline of a
   connected group, belongs to the smallest face holding a point a hair to
   the left of its longest piece that is no spike, or to the outside.
3. A face is inside an area (its outer ring less its holes) when a point a
   hair to the left of its walk's longest piece that is no spike is.
4. The result: every directed piece whose face is kept and whose way back's
   face is not, chained keeping the kept faces on the left (at a vertex the
   next result piece clockwise from the way back), split where a ring comes
   back to a vertex; a vertex where the ring runs straight on is dropped
   unless it is an input vertex (a corner of the area, the path or a
   neighbour). A ring of area at most its perimeter × 1e-6 is dust.
5. Counter-clockwise rings are outer rings; clockwise ones holes of the
   smallest outer ring holding a point a hair to their left.

The arc cases (a circle among the neighbours) are worked out by hand from
the same rules: their crossing points are whole numbers, their bulges
tan(θ/4) and their areas (the corners' polygon plus each arc's segment)
come from 50-digit mpmath.

Random cases (jittered parcel blocks about (487000, 4420000), a new area
across them, a path round a missing parcel) keep every vertex, crossing
and piece at least 1e-4 m from any other it does not lie exactly on: the
core joins what is within 1 µm, the rationals never do.

Output, whatever order the core finds things in: areas sorted by their
outer ring's lowest vertex (x, then y), then its next; each ring starting
at its lowest vertex and running as the core runs it (outer rings
counter-clockwise, holes clockwise); holes by their first vertex.
"""

import argparse
import json
import math
import random
import sys
from fractions import Fraction as F
from functools import cmp_to_key
from pathlib import Path

import mpmath as mp

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "adjoin" / "v1" / "cases.json"
DUST = 1e-6
CLEAR = F(1, 10**4)
E0, N0 = 487000, 4420000
mp.mp.dps = 50


# ── Exact plane ─────────────────────────────────────────────────────────────


def sub(a, b):
    return (a[0] - b[0], a[1] - b[1])


def cross(a, b):
    return a[0] * b[1] - a[1] * b[0]


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1]


def meet(p, q, r, s):
    """The points where segments pq and rs meet: none, one, or the ends of their overlap."""
    d1, d2 = sub(q, p), sub(s, r)
    den = cross(d1, d2)
    if den != 0:
        t = cross(sub(r, p), d2) / den
        u = cross(sub(r, p), d1) / den
        if 0 <= t <= 1 and 0 <= u <= 1:
            return [(p[0] + d1[0] * t, p[1] + d1[1] * t)]
        return []
    if cross(sub(r, p), d1) != 0:
        return []
    out = []
    for a, b, c in ((p, q, r), (p, q, s), (r, s, p), (r, s, q)):
        ab = sub(b, a)
        if dot(ab, ab) == 0:
            continue
        t = dot(sub(c, a), ab) / dot(ab, ab)
        if 0 <= t <= 1:
            out.append(c)
    return out


def area2(ring):
    return sum(cross(ring[i], ring[(i + 1) % len(ring)]) for i in range(len(ring)))


def fl(p):
    return (float(p[0]), float(p[1]))


def perimeter(ring):
    return sum(math.dist(fl(ring[i]), fl(ring[(i + 1) % len(ring)])) for i in range(len(ring)))


def inside(p, ring):
    """Strictly inside a ring, exactly (p is off its edges)."""
    x, y = p
    c = False
    n = len(ring)
    for i in range(n):
        (x1, y1), (x2, y2) = ring[i], ring[(i + 1) % n]
        if (y1 > y) != (y2 > y):
            xi = x1 + (y - y1) * (x2 - x1) / (y2 - y1)
            if xi > x:
                c = not c
    return c


def in_area(p, rings):
    """Inside an area: its outer ring less its holes."""
    return inside(p, rings[0]) and not any(inside(p, h) for h in rings[1:])


def half(d):
    return 0 if (d[1] > 0 or (d[1] == 0 and d[0] > 0)) else 1


def turn_order(a, b):
    """Counter-clockwise from east: by half plane, then by turn."""
    ha, hb = half(a), half(b)
    if ha != hb:
        return ha - hb
    c = cross(a, b)
    return -1 if c > 0 else 1 if c < 0 else 0


def hair_left(a, b):
    """A point a hair to the left of segment ab's middle."""
    a, b = fl(a), fl(b)
    m = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
    ln = math.hypot(b[0] - a[0], b[1] - a[1])
    eps = min(ln * 1e-4, 1e-4)
    return (F(m[0] - (b[1] - a[1]) / ln * eps), F(m[1] + (b[0] - a[0]) / ln * eps))


def longest(pieces):
    return max(pieces, key=lambda h: (dot(sub(h[1], h[0]), sub(h[1], h[0])), h))


def probe(walk):
    """A hair to the left of the walk's longest piece that is no spike (any, if all are)."""
    own = set(walk)
    plain = [h for h in walk if (h[1], h[0]) not in own] or walk
    return hair_left(*longest(plain))


def split(walk):
    seen = {}
    for i, h in enumerate(walk):
        if h[0] in seen:
            j = seen[h[0]]
            return split(walk[j:i]) + split(walk[:j] + walk[i:])
        seen[h[0]] = i
    return [walk] if len(walk) >= 2 else []


def straightened(pts, inputs):
    pts = list(pts)
    changed = True
    while changed and len(pts) > 2:
        changed = False
        for i in range(len(pts)):
            a, v, b = pts[i - 1], pts[i], pts[(i + 1) % len(pts)]
            if v not in inputs and cross(sub(v, a), sub(b, v)) == 0 and dot(sub(v, a), sub(b, v)) > 0:
                del pts[i]
                changed = True
                break
    return pts


# ── The arrangement and its faces ───────────────────────────────────────────


class Arrangement:
    def __init__(self, segs, inputs):
        """`segs`: (a, b, on the path); `inputs`: every input vertex."""
        pieces = {}
        for i, (a, b, path) in enumerate(segs):
            d = sub(b, a)
            dd = dot(d, d)
            if dd == 0:
                continue
            cuts = {a, b}
            for j, (c, e, _) in enumerate(segs):
                if i != j and c != e:
                    cuts.update(meet(a, b, c, e))
            order = sorted(cuts, key=lambda p: dot(sub(p, a), d) / dd)
            for p, r in zip(order, order[1:]):
                if p != r:
                    k = (min(p, r), max(p, r))
                    pieces[k] = pieces.get(k, False) or path
        self.pieces = pieces
        self.inputs = inputs
        around = {}
        for p, r in pieces:
            around.setdefault(p, []).append(r)
            around.setdefault(r, []).append(p)
        for v, ws in around.items():
            ws.sort(key=cmp_to_key(lambda w1, w2, v=v: turn_order(sub(w1, v), sub(w2, v))))
        self.around = around
        seen = set()
        walks = []
        for p, r in sorted(pieces):
            for start in ((p, r), (r, p)):
                if start in seen:
                    continue
                walk = []
                h = start
                while h not in seen:
                    seen.add(h)
                    walk.append(h)
                    v, w = h
                    ws = around[w]
                    h = (w, ws[(ws.index(v) - 1) % len(ws)])
                walks.append(walk)
        self.walks = walks
        self.walk_of = {h: k for k, wk in enumerate(walks) for h in wk}
        twice = [sum(cross(v, w) for v, w in wk) for wk in walks]
        faces = sorted((k for k in range(len(walks)) if twice[k] > 0), key=lambda k: twice[k])
        self.faces = faces
        self.face_of = {}
        for k in range(len(walks)):
            if twice[k] > 0:
                self.face_of[k] = k
            else:
                p = probe(walks[k])
                self.face_of[k] = next((f for f in faces if inside(p, [h[0] for h in walks[f]])), None)

    def face(self, h):
        return self.face_of[self.walk_of[h]]

    def probe(self, f):
        return probe(self.walks[f])

    def on_path(self, h):
        return self.pieces[(min(h), max(h))]

    def boundary(self, kept):
        """The areas the kept faces make (step 4 and 5)."""
        bset = {h for wk in self.walks for h in wk if self.face(h) in kept and self.face((h[1], h[0])) not in kept}
        used = set()
        loops = []
        for start in sorted(bset):
            if start in used:
                continue
            walk = []
            h = start
            while h not in used:
                used.add(h)
                walk.append(h)
                v, w = h
                ws = self.around[w]
                k = ws.index(v)
                h = next((w, ws[(k - s) % len(ws)]) for s in range(1, len(ws) + 1) if (w, ws[(k - s) % len(ws)]) in bset)
            loops.extend(split(walk))
        rings = []
        for lp in loops:
            pts = straightened([h[0] for h in lp], self.inputs)
            if len(pts) < 3 or abs(float(area2(pts))) / 2 <= perimeter(pts) * DUST:
                continue
            rings.append(pts)
        outers = sorted((r for r in rings if area2(r) > 0), key=area2)
        areas = [{"outer": o, "holes": []} for o in outers]
        for hole in (r for r in rings if area2(r) < 0):
            p = hair_left(hole[0], hole[1])
            k = next((i for i, o in enumerate(outers) if inside(p, o)), None)
            if k is not None:
                areas[k]["holes"].append(hole)
        return areas


# ── The two operations ──────────────────────────────────────────────────────


def q(p):
    return (F(p["x"]), F(p["y"]))


def rings_of(area):
    return [[q(p) for p in area["outer"]["pts"]]] + [[q(p) for p in h["pts"]] for h in area.get("holes", [])]


def ring_segs(ring, path=False):
    return [(ring[i], ring[(i + 1) % len(ring)], path) for i in range(len(ring))]


def arrangement_of(mine, theirs):
    segs = [s for r in mine for s in ring_segs(r)]
    inputs = {v for r in mine for v in r}
    for obj in theirs:
        for part in obj:
            for r in part:
                segs += ring_segs(r)
                inputs.update(r)
    return Arrangement(segs, inputs)


def avoid(area, neighbours):
    """The neighbours that overlap the area are found among them all; only those cut it (one that just
    touches it adds no corner)."""
    mine = rings_of(area)
    theirs = [[rings_of(part) for part in obj] for obj in neighbours]
    arr = arrangement_of(mine, theirs)
    overlapped = set()
    for f in arr.faces:
        p = arr.probe(f)
        if in_area(p, mine):
            overlapped.update(k for k, obj in enumerate(theirs) if any(in_area(p, part) for part in obj))
    if not overlapped:
        return [{"outer": mine[0], "holes": mine[1:]}], []
    cutters = [theirs[k] for k in sorted(overlapped)]
    arr = arrangement_of(mine, cutters)
    kept = set()
    for f in arr.faces:
        p = arr.probe(f)
        if in_area(p, mine) and not any(in_area(p, part) for obj in cutters for part in obj):
            kept.add(f)
    return arr.boundary(kept), sorted(overlapped)


def fill(path, neighbours):
    pts = [q(p) for p in path["pts"]]
    return fill_with(pts, [[rings_of(part) for part in obj] for obj in neighbours])


def fill_with(pts, theirs):
    segs = [(pts[i], pts[i + 1], True) for i in range(len(pts) - 1)]
    inputs = set(pts)
    for obj in theirs:
        for part in obj:
            for r in part:
                segs += ring_segs(r)
                inputs.update(r)
    arr = Arrangement(segs, inputs)
    kept = set()
    for f in arr.faces:
        p = arr.probe(f)
        if any(in_area(p, part) for obj in theirs for part in obj):
            continue
        if any(arr.on_path(h) and arr.face((h[1], h[0])) != f for h in arr.walks[f]):
            kept.add(f)
    return arr.boundary(kept)


# ── Written form ────────────────────────────────────────────────────────────


def lowest_first(ring):
    k = min(range(len(ring)), key=lambda i: ring[i])
    return ring[k:] + ring[:k]


def net_area(a):
    return (area2(a["outer"]) + sum(area2(h) for h in a["holes"])) / 2


def written(a):
    return {
        "outer": {"pts": [list(fl(p)) for p in lowest_first(a["outer"])]},
        "holes": sorted(({"pts": [list(fl(p)) for p in lowest_first(h)]} for h in a["holes"]), key=lambda h: h["pts"][0]),
        "area": float(net_area(a)),
    }


def sorted_areas(areas):
    return sorted(areas, key=lambda a: (a["outer"]["pts"][0], a["outer"]["pts"][1]))


def P(x, y):
    return {"x": x, "y": y}


def AREA(pts, holes=()):
    return {"outer": {"pts": [P(*p) for p in pts]}, "holes": [{"pts": [P(*p) for p in h]} for h in holes]}


def RECT(x0, y0, x1, y1):
    return AREA([(x0, y0), (x1, y0), (x1, y1), (x0, y1)])


def PATH(*pts):
    return {"pts": [P(*p) for p in pts]}


def CIRCLE(cx, cy, r):
    """A circle as the area tools take it: two half circles from (cx + r, cy)."""
    return {"outer": {"pts": [P(cx + r, cy), P(cx - r, cy)], "bulges": [1, 1]}, "holes": []}


# ── Hand cases ──────────────────────────────────────────────────────────────


def avoid_hand():
    """(name, area, neighbours), at the origin."""
    c = []
    c.append(("bir komşu: taşan kısım kesilir", RECT(0, 0, 20, 10), [[RECT(15, -5, 30, 15)]]))
    c.append(("biri kırpar, öbürü yalnız köşesiyle değer", RECT(0, 0, 20, 10), [[RECT(15, -5, 30, 15)], [AREA([(5, 10), (8, 15), (2, 15)])]]))
    c.append(("iki komşu, iki yandan", RECT(0, 0, 30, 10), [[RECT(-5, -5, 5, 15)], [RECT(25, 2, 40, 8)]]))
    c.append(("kenarı ortak, örtüşmüyor: olduğu gibi", RECT(0, 0, 20, 10), [[RECT(20, 0, 30, 10)]]))
    c.append(("yalnız köşede değer: olduğu gibi", RECT(0, 0, 20, 10), [[RECT(20, 10, 30, 20)]]))
    c.append(("komşu kenarın bir parçasına oturur: olduğu gibi", RECT(0, 0, 20, 10), [[AREA([(5, 10), (15, 10), (10, 15)])]]))
    c.append(("komşunun köşesi kenarında: olduğu gibi", RECT(0, 0, 20, 10), [[AREA([(10, 10), (15, 15), (5, 15)])]]))
    c.append(("tümüyle komşunun içinde: boş", RECT(2, 2, 8, 8), [[RECT(0, 0, 10, 10)]]))
    c.append(("komşuyu içine alır: delik", RECT(0, 0, 30, 20), [[RECT(10, 5, 20, 15)]]))
    c.append(("komşu ortadan böler: iki parça", RECT(0, 0, 30, 10), [[RECT(12, -5, 18, 15)]]))
    c.append(("kenar üst üste biner, sonra komşuya girer", AREA([(0, 0), (20, 0), (20, 10), (0, 10)]), [[AREA([(10, 0), (30, 0), (30, 10), (14, 10), (14, 4), (10, 4)])]]))
    c.append(("delikli komşu: delikteki kısım kalır", RECT(0, 0, 30, 10), [[AREA([(10, -10), (40, -10), (40, 20), (10, 20)], [[(15, 2), (15, 8), (25, 8), (25, 2)]])]]))
    c.append(("çok parçalı komşu bir kez sayılır", RECT(0, 0, 40, 10), [[RECT(5, 5, 10, 15), RECT(30, -5, 35, 5)]]))
    c.append(("eğik kenarlar, kesişim köşeleri", AREA([(0, 0), (24, 3), (20, 17), (2, 13)]), [[AREA([(10, -6), (31, 5), (14, 22)])], [AREA([(-8, 8), (5, 4), (6, 20)])]]))
    c.append(("delikli yeni alan, komşu deliğe uzanır", AREA([(0, 0), (40, 0), (40, 30), (0, 30)], [[(10, 10), (10, 20), (20, 20), (20, 10)]]), [[RECT(15, 12, 50, 18)]]))
    c.append(("komşular üst üste", RECT(0, 0, 30, 10), [[RECT(10, -5, 20, 5)], [RECT(15, 0, 25, 15)]]))
    return c


def fill_hand():
    """(name, path, neighbours), at the origin."""
    c = []
    left, right, bottom = RECT(0, 0, 10, 10), RECT(20, 0, 30, 10), RECT(0, -10, 30, 0)
    c.append(("iki komşu arası boşluk", PATH((5, 5), (15, 12), (25, 5)), [[left], [right], [bottom]]))
    c.append(("uçlar komşuların sınırında", PATH((10, 6), (15, 12), (20, 6)), [[left], [right], [bottom]]))
    c.append(("yol ters yönde", PATH((25, 5), (15, 12), (5, 5)), [[left], [right], [bottom]]))
    c.append(("kapanmayan yol: bölge yok", PATH((5, 5), (15, 12), (18, 20)), [[left], [right], [bottom]]))
    c.append(("yol bir komşuya girip çıkar", PATH((5, 5), (5, 14), (22, 14), (22, 8), (26, 8), (26, 14), (35, 14), (35, 5), (28, 5)), [[left], [right], [bottom]]))
    c.append(("boşluktaki ada delik olur", PATH((5, 5), (15, 12), (25, 5)), [[left], [right], [bottom], [RECT(13, 2, 17, 4)]]))
    c.append(("iki cep: çok parçalı", PATH((5, 5), (5, 14), (15, 14), (15, 4), (16, 4), (16, 14), (25, 14), (25, 5)), [[RECT(0, 0, 10, 10)], [RECT(12, 0, 19, 8)], [RECT(20, 0, 30, 10)], [RECT(0, -10, 30, 0)]]))
    c.append(("yol komşunun kenarı boyunca da gider", PATH((5, 5), (10, 8), (10, 12), (20, 12), (20, 8), (25, 5)), [[left], [right], [bottom]]))
    c.append(("kendini kesen yol, komşusuz", PATH((0, 0), (10, 10), (10, 0), (0, 10)), []))
    c.append(("derin girinti: dipte uzak komşu", PATH((3, 18), (15, 22), (27, 18)), [[RECT(0, 10, 10, 20)], [RECT(20, 10, 30, 20)], [RECT(0, -40, 30, -30)], [RECT(0, -30, 6, 10)], [RECT(24, -30, 30, 10)]]))
    c.append(("komşunun deliğini kesen yol: iki yanı da dolar", PATH((10, 15), (17, 22), (25, 15)), [[AREA([(0, 0), (40, 0), (40, 30), (0, 30)], [[(10, 5), (10, 25), (25, 25), (25, 5)]])]]))
    c.append(("sınırdan sarkan yol bölge saymaz", PATH((15, 0), (15, 5)), [[left], [right], [bottom], [RECT(10, 9, 20, 15)]]))
    return c


def moved_area(a, de, dn):
    def mv(v):
        if isinstance(v, dict) and set(v) == {"x", "y"}:
            return P(v["x"] + de, v["y"] + dn)
        if isinstance(v, dict):
            return {k: mv(x) for k, x in v.items()}
        if isinstance(v, list):
            return [mv(x) for x in v]
        return v

    return mv(a)


# ── Arc cases, by hand ──────────────────────────────────────────────────────


def seg_area(r, sweep):
    """Signed area between an arc of radius r and its chord (counter-clockwise positive)."""
    return r * r / 2 * (sweep - mp.sin(sweep))


def to_mp(v):
    return mp.mpf(v.numerator) / v.denominator if isinstance(v, F) else mp.mpf(v)


def shoelace(pts):
    return sum(pts[i][0] * pts[(i + 1) % len(pts)][1] - pts[(i + 1) % len(pts)][0] * pts[i][1] for i in range(len(pts))) / 2


def arc_written(pts, sweeps, r):
    """A ring of corners and sweeps (None: straight), already lowest first and counter-clockwise."""
    area = shoelace([(to_mp(x), to_mp(y)) for x, y in pts]) + sum(seg_area(r, s) for s in sweeps if s is not None)
    return {
        "outer": {
            "pts": [[float(x), float(y)] for x, y in pts],
            "bulges": [0.0 if s is None else float(mp.tan(s / 4)) for s in sweeps],
        },
        "holes": [],
        "area": float(area),
    }


def arc_cases(de, dn):
    """The square less a circle on its edge, and a gap closed by a circle; (de, dn) moves them."""
    m = lambda x, y: (x + de, y + dn)
    # 1. Square (0,0)–(10,10) less the circle of radius 3 about (10, 5): the circle meets x = 10 at
    # (10, 2) and (10, 8); its left half bounds the result, cut at its own corner (7, 5), each quarter
    # clockwise (−π/2).
    q90 = -mp.pi / 2
    avoid_arc = (
        "daire komşu kenarda: yarım daire kesilir",
        AREA([m(0, 0), m(10, 0), m(10, 10), m(0, 10)]),
        [[CIRCLE(*m(10, 5), 3)]],
        {
            "areas": [arc_written([m(0, 0), m(10, 0), m(10, 2), m(7, 5), m(10, 8), m(10, 10), m(0, 10)], [None, None, q90, q90, None, None, None], 3)],
            "overlapped": [0],
        },
    )
    # 2. A gap between a parcel (0,1)–(10,10), a strip below it (y ≤ 1) and the circle of radius 5 about
    # (20, 5), closed by a path from inside the parcel to the circle's centre. The strip meets the circle
    # at (17, 1); the path's last leg meets it at (16, 8) and the parcel's edge x = 10 at (10, 29/3). The
    # circle's arc from (17, 1) to (16, 8) runs clockwise round its centre through its corner (15, 5):
    # −acos(3/5), then −acos(4/5).
    s1, s2 = -mp.acos(mp.mpf(3) / 5), -mp.acos(mp.mpf(4) / 5)
    fill_arc = (
        "daire komşuyla kapanan boşluk",
        PATH(m(6, 7), m(12, 11), m(20, 5)),
        [[AREA([m(0, 1), m(10, 1), m(10, 10), m(0, 10)])], [AREA([m(0, -5), m(30, -5), m(30, 1), m(0, 1)])], [CIRCLE(*m(20, 5), 5)]],
        {
            "areas": [arc_written([m(10, 1), m(17, 1), m(15, 5), m(16, 8), m(12, 11), (10 + de, F(29, 3) + dn)], [None, s1, s2, None, None, None], 5)],
        },
    )
    return avoid_arc, fill_arc


# ── Random cases ────────────────────────────────────────────────────────────


def clean(segs):
    """No vertex, crossing or piece end within 1e-4 m of a segment or point it does not lie on exactly."""
    pts = set()
    for i, (a, b, _) in enumerate(segs):
        pts.update((a, b))
        for j in range(i + 1, len(segs)):
            pts.update(meet(a, b, segs[j][0], segs[j][1]))
    pts = list(pts)
    for i, p in enumerate(pts):
        for o in pts[i + 1:]:
            d = sub(p, o)
            if dot(d, d) < CLEAR * CLEAR:
                return False
        for a, b, _ in segs:
            d = sub(b, a)
            dd = dot(d, d)
            t = dot(sub(p, a), d) / dd
            if t < 0 or t > 1:
                continue
            c = (a[0] + d[0] * t, a[1] + d[1] * t)
            e = sub(p, c)
            if 0 < dot(e, e) < CLEAR * CLEAR:
                return False
    return True


def segs_of(neighbours, extra):
    out = [s for obj in neighbours for part in obj for r in rings_of(part) for s in ring_segs(r)]
    return out + extra


def block(rnd, cols, rows):
    """A jittered block of parcels about (487000, 4420000): their corner grid and the parcels."""
    w, h = rnd.uniform(15, 30), rnd.uniform(15, 25)
    x0, y0 = E0 + rnd.uniform(-100, 100), N0 + rnd.uniform(-100, 100)
    r3 = lambda v: round(v, 3)
    grid = [[(r3(x0 + i * w + (rnd.uniform(-3, 3) if 0 < i < cols else 0)), r3(y0 + j * h + (rnd.uniform(-3, 3) if 0 < j < rows else 0))) for j in range(rows + 1)] for i in range(cols + 1)]
    parcels = {}
    for i in range(cols):
        for j in range(rows):
            parcels[(i, j)] = AREA([grid[i][j], grid[i + 1][j], grid[i + 1][j + 1], grid[i][j + 1]])
    return grid, parcels, w, h


def avoid_random(seed):
    rnd = random.Random(seed)
    while True:
        grid, parcels, w, h = block(rnd, rnd.randint(2, 4), rnd.randint(2, 3))
        cols, rows = len(grid) - 1, len(grid[0]) - 1
        # The new area: a 4- to 6-cornered shape over a corner of the block, partly outside it.
        cx, cy = grid[rnd.randint(0, cols)][rnd.randint(0, rows)]
        n = rnd.randint(4, 6)
        rad = rnd.uniform(0.6, 1.2) * min(w, h)
        pts = []
        for k in range(n):
            a = 2 * math.pi * k / n + rnd.uniform(-0.3, 0.3)
            rr = rad * rnd.uniform(0.6, 1.0)
            pts.append((round(cx + rr * math.cos(a), 3), round(cy + rr * math.sin(a), 3)))
        area = AREA(pts)
        keys = sorted(parcels)
        rnd.shuffle(keys)
        neighbours = [[parcels[k]] for k in keys[: rnd.randint(1, len(keys))]]
        if clean(segs_of(neighbours, ring_segs([q(P(*p)) for p in pts]))):
            return (f"rastgele kırpma {seed}", area, neighbours)


def fill_random(seed):
    rnd = random.Random(seed)
    while True:
        grid, parcels, w, h = block(rnd, rnd.randint(3, 4), rnd.randint(2, 3))
        cols, rows = len(grid) - 1, len(grid[0]) - 1
        # A parcel on the top row is missing; the path closes it from the parcels beside it, over the top.
        i = rnd.randint(1, cols - 2)
        gone = (i, rows - 1)
        neighbours = [[parcels[k]] for k in sorted(parcels) if k != gone]
        rnd.shuffle(neighbours)
        (ax, ay), (bx, by) = grid[i][rows], grid[i + 1][rows]
        top = max(ay, by) + rnd.uniform(3, 8)
        pts = [
            (round(ax - rnd.uniform(2, 0.4 * w), 3), round(ay - rnd.uniform(2, 0.4 * h), 3)),
            (round(ax + rnd.uniform(-1, 1), 3), round(top, 3)),
        ]
        if rnd.random() < 0.5:
            pts.append((round((ax + bx) / 2 + rnd.uniform(-2, 2), 3), round(top + rnd.uniform(0, 4), 3)))
        pts += [
            (round(bx + rnd.uniform(-1, 1), 3), round(top + rnd.uniform(-1, 1), 3)),
            (round(bx + rnd.uniform(2, 0.4 * w), 3), round(by - rnd.uniform(2, 0.4 * h), 3)),
        ]
        path = PATH(*pts)
        qp = [q(P(*p)) for p in pts]
        if clean(segs_of(neighbours, [(qp[k], qp[k + 1], True) for k in range(len(qp) - 1)])):
            return (f"rastgele boşluk {seed}", path, neighbours)


# ── Build ───────────────────────────────────────────────────────────────────


def avoid_case(name, area, neighbours):
    areas, overlapped = avoid(area, neighbours)
    return {"name": name, "area": area, "neighbours": neighbours, "expect": {"areas": sorted_areas([written(a) for a in areas]), "overlapped": overlapped}}


def fill_case(name, path, neighbours):
    return {"name": name, "path": path, "neighbours": neighbours, "expect": {"areas": sorted_areas([written(a) for a in fill(path, neighbours)])}}


def build():
    avoids, fills = [], []
    for name, area, neighbours in avoid_hand():
        avoids.append(avoid_case(name, area, neighbours))
        avoids.append(avoid_case(name + ", TM koordinatlarında", moved_area(area, E0, N0), moved_area(neighbours, E0, N0)))
    for name, path, neighbours in fill_hand():
        fills.append(fill_case(name, path, neighbours))
        fills.append(fill_case(name + ", TM koordinatlarında", moved_area(path, E0, N0), moved_area(neighbours, E0, N0)))
    for de, dn, suffix in ((0, 0, ""), (E0, N0, ", TM koordinatlarında")):
        (an, aa, anb, aex), (fn, fp, fnb, fex) = arc_cases(de, dn)
        avoids.append({"name": an + suffix, "area": aa, "neighbours": anb, "expect": aex})
        fills.append({"name": fn + suffix, "path": fp, "neighbours": fnb, "expect": fex})
    for seed in range(1, 21):
        avoids.append(avoid_case(*avoid_random(seed)))
        fills.append(fill_case(*fill_random(seed)))
    return {
        "format": "kentos.adjoin",
        "version": 1,
        "source": "scripts/fixtures/adjoin_cases.py (docs/adr/0162 §2, §3, §5)",
        "avoid": avoids,
        "fill": fills,
    }


def jsonable(v):
    if isinstance(v, F):
        return float(v)
    if isinstance(v, dict):
        return {k: jsonable(x) for k, x in v.items()}
    if isinstance(v, (list, tuple)):
        return [jsonable(x) for x in v]
    return v


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--check", action="store_true", help="compare with the written file instead of writing it")
    args = ap.parse_args()
    text = json.dumps(jsonable(build()), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if OUT.read_text() != text:
            print(f"{OUT} güncel değil: python3 scripts/fixtures/adjoin_cases.py", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT}: güncel")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    d = json.loads(text)
    print(f"{OUT}: {len(d['avoid'])} kırpma, {len(d['fill'])} doldurma")


if __name__ == "__main__":
    main()
