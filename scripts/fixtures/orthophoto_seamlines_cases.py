#!/usr/bin/env python3
"""The shared cases of `cad.orthophoto.seamlines` (Mozaik birleşme çizgisi; plan/map-cad.md MAP-12, docs/adr/4700), without
KentOS code.

    python3 scripts/fixtures/orthophoto_seamlines_cases.py           # writes fixtures/commands/v1/cad.orthophoto.seamlines.json
    python3 scripts/fixtures/orthophoto_seamlines_cases.py --check   # writes nothing; compares

The rules, by hand (Pan, Wang, Li, Li 2009, the area Voronoi diagram with overlap; Chon, Kim, Lin 2010, the minimax
seam; the ADR's definitions), computed here at 40 decimal digits where the geometry is exact:
- footprints: the rasters' four corners (GDAL's affine, pixel corners (0, h), (w, h), (w, 0), (0, 0)) or the areas
  given; repeated corners and corners on a straight edge dropped, counter-clockwise, convex;
- the overlap: the first clipped by the second's half-planes (a point on a line is inside), started at its southernmost
  corner (the westernmost of equals); an edge on the second's boundary only is the first's end (the second image
  stops there, inside the first), on the first's only the second's end, on both shared; each end one run;
- f(p) = distance to the first's end − to the second's (a point's distance to an image's own part, inside the overlap);
- the ends: after the first's run (counter-clockwise) a corner of the second's run (the boundaries cross there), or on
  the shared run between, the first point (from the first's side) where f reaches 0; the same after the second's run;
- the Voronoi seam: here the chord from the start to the end is scanned densely (1000 lines across it) for the pairs
  of nearest elements (an edge when the foot falls inside it, else a corner; equals to the edge); every change of pair
  is found by bisection on the chord to 40 digits, the seam's point there its turn. A piece whose pair is two edges,
  two corners or a corner on the other's edge line is straight; between a corner and an edge it is a parabola, whose
  points are the roots on the lines across the chord at k/n (n = the chord's length / step, a whole number within
  10⁻⁹ or rounded up, at least 2). KentOS finds the turns by intersecting the pieces' lines instead;
- the polygons: the first's ring from the end round its own side (counter-clockwise) to the start, then the seam;
  the second's from the start round its side to the end, then the seam back;
- radiometry: the synthetic images of orthophoto_compare_cases.py (their pixel values computed there, written there
  byte by byte); a pixel of the first is read when its centre lies in the overlap and the second can be read there:
  its own pixel when the centres coincide (10⁻⁶ pixel), else Keys' cubic convolution of its 16 pixels round the point
  (rows of columns, as written there); its difference the largest of the bands' absolute differences, a cell's (c × c
  pixels from the image's pixel (0, 0)) the largest of its pixels', searched when all its pixels are read, its centre
  within the corridor of the Voronoi seam and no pixel centre in an avoided area; from the searched cell nearest the
  start to the one nearest the end (squared distance, the first in row order of equals): the minimax path, then the
  least-cost path under it (a move: its length 1 or √2 × (1 + the mean of its cells' differences)); queues by
  (key, row, column), a cell lowered only strictly; the seam is the start, the path's turning cells' centres, the end.
"""

import copy
import json
import math
import sys
from decimal import Decimal, getcontext
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import orthophoto_compare_cases as oc  # noqa: E402  the synthetic GeoTIFFs and their pixels

OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.orthophoto.seamlines.json"
X0, Y0 = oc.X0, oc.Y0
getcontext().prec = 40
SAME_D = Decimal("1e-9")
ON_D = Decimal("1e-7")
SAME = {"revision": "same", "canUndo": False}
STEP = "Mozaik elemanları"
LAYER, LAYER_NAME = "mozaik-elemanlari", "Mozaik elemanları"


def uid(n):
    return f"0192f5c0-4700-7000-8000-{n:012x}"


# ── Exact plane helpers ───────────────────────────────────────────────────────────────────────────────────────────
def D(v):
    return Decimal(v)


def sub(a, b):
    return (a[0] - b[0], a[1] - b[1])


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1]


def cross(a, b):
    return a[0] * b[1] - a[1] * b[0]


def norm(a):
    return (a[0] * a[0] + a[1] * a[1]).sqrt()


def lerp(a, b, t):
    return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)


def orient(a, b, c):
    s = cross(sub(b, a), sub(c, a))
    return (s > 0) - (s < 0)


def seg_dist(p, a, b):
    ab = sub(b, a)
    l2 = dot(ab, ab)
    t = dot(sub(p, a), ab) / l2
    if t <= 0:
        return norm(sub(p, a))
    if t >= 1:
        return norm(sub(p, b))
    return abs(cross(ab, sub(p, a))) / l2.sqrt()


def poly_dist(p, line):
    return min(seg_dist(p, line[k], line[k + 1]) for k in range(len(line) - 1))


def area(r):
    o = r[0]
    return sum((cross(sub(r[i], o), sub(r[i + 1], o)) for i in range(1, len(r) - 1)), D(0)) / 2


def deduped(r):
    out = []
    for p in r:
        if not out or norm(sub(p, out[-1])) > SAME_D:
            out.append(p)
    while len(out) > 1 and norm(sub(out[0], out[-1])) <= SAME_D:
        out.pop()
    return out


def straightened(r):
    r = list(r)
    while len(r) >= 3:
        n = len(r)
        for i in range(n):
            a, b, c = r[i - 1], r[i], r[(i + 1) % n]
            ac = sub(c, a)
            if orient(a, b, c) == 0 or (norm(ac) > SAME_D and abs(cross(ac, sub(b, a))) / norm(ac) <= SAME_D):
                del r[i]
                break
        else:
            return r
    return r


class Fault(Exception):
    pass


def ready(ring, k):
    r = straightened(deduped([(D(x), D(y)) for x, y in ring]))
    if len(r) < 3 or not abs(area(r)) > D("1e-12"):
        raise Fault(("degenerate", k))
    if area(r) < 0:
        r.reverse()
    n = len(r)
    if any(orient(r[i - 1], r[i], r[(i + 1) % n]) < 0 for i in range(n)):
        raise Fault(("convex", k))
    return r


def clip(subject, clipper):
    out = list(subject)
    for i in range(len(clipper)):
        a, b = clipper[i], clipper[(i + 1) % len(clipper)]
        inp, out = out, []
        for j in range(len(inp)):
            p, q = inp[j], inp[(j + 1) % len(inp)]
            sp, sq = orient(a, b, p), orient(a, b, q)
            if sp >= 0:
                out.append(p)
            if (sp > 0 > sq) or (sp < 0 < sq):
                ab = sub(b, a)
                cp, cq = cross(ab, sub(p, a)), cross(ab, sub(q, a))
                out.append(lerp(p, q, cp / (cp - cq)))
        if not out:
            break
    return straightened(deduped(out))


def on_ring(ring, u, v):
    mid = lerp(u, v, D("0.5"))
    n = len(ring)
    return any(all(seg_dist(x, ring[i], ring[(i + 1) % n]) <= ON_D for x in (u, v, mid)) for i in range(n))


# ── The footprints' overlap, ends and seam ───────────────────────────────────────────────────────────────────────
class Seam:
    def __init__(self, f1, f2):
        self.a, self.b = ready(f1, 0), ready(f2, 1)
        o = clip(self.a, self.b)
        if len(o) < 3 or area(o) <= D("1e-6"):
            raise Fault(("apart",))
        k = min(range(len(o)), key=lambda i: (o[i][1], o[i][0]))
        o = o[k:] + o[:k]
        self.o = o
        n = len(o)
        kinds = []
        for i in range(n):
            u, v = o[i], o[(i + 1) % n]
            on1, on2 = on_ring(self.a, u, v), on_ring(self.b, u, v)
            kinds.append("shared" if on1 and on2 else "first" if on2 else "second")
        self.kinds = kinds
        runs = []
        start = next((i for i in range(n) if kinds[i] != kinds[i - 1]), None)
        if start is None:
            runs = [(kinds[0], 0, n - 1)]
        else:
            k = 0
            while k < n:
                i = (start + k) % n
                e = k
                while e + 1 < n and kinds[(start + e + 1) % n] == kinds[i]:
                    e += 1
                runs.append((kinds[i], i, (start + e) % n))
                k = e + 1
        count = {kind: sum(1 for r in runs if r[0] == kind) for kind in ("first", "second")}
        if count["first"] == 0:
            raise Fault(("inside", 0))
        if count["second"] == 0:
            raise Fault(("inside", 1))
        if count["first"] > 1 or count["second"] > 1:
            raise Fault(("crossed",))
        r1 = next(r for r in runs if r[0] == "first")
        r2 = next(r for r in runs if r[0] == "second")

        def line(r):
            pts, i = [o[r[1]]], r[1]
            while True:
                pts.append(o[(i + 1) % n])
                if i == r[2]:
                    return pts
                i = (i + 1) % n

        self.g1, self.g2 = line(r1), line(r2)
        ends = []
        for r, stop, sign in ((r1, r2[1], 1), (r2, r1[1], -1)):
            s = (r[2] + 1) % n
            if s == stop:
                ends.append((o[s], True))
                continue
            chain, i = [o[s]], s
            while i != stop:
                i = (i + 1) % n
                chain.append(o[i])
            ends.append((self.root_along(chain, sign), False))
        (self.s, self.sc), (self.t, self.tc) = ends

    def f(self, p):
        return poly_dist(p, self.g1) - poly_dist(p, self.g2)

    def root_along(self, chain, sign):
        for k in range(len(chain) - 1):
            a, b = chain[k], chain[k + 1]
            if sign * self.f(b) >= 0:
                lo, hi = D(0), D(1)
                for _ in range(140):
                    mid = (lo + hi) / 2
                    if sign * self.f(lerp(a, b, mid)) >= 0:
                        hi = mid
                    else:
                        lo = mid
                return lerp(a, b, (lo + hi) / 2)
        return chain[-1]

    def across(self, s):
        chord = sub(self.t, self.s)
        L = norm(chord)
        nu = (-chord[1] / L, chord[0] / L)
        c = lerp(self.s, self.t, s)
        lo, hi = None, None
        n = len(self.o)
        for i in range(n):
            u, v = self.o[i], self.o[(i + 1) % n]
            e = sub(v, u)
            a, b = cross(e, sub(c, u)), cross(e, nu)
            if b > 0:
                lo = -a / b if lo is None else max(lo, -a / b)
            elif b < 0:
                hi = -a / b if hi is None else min(hi, -a / b)
        return c, nu, lo, hi

    def root(self, s):
        c, nu, lo, hi = self.across(D(s))
        at = lambda t: (c[0] + nu[0] * t, c[1] + nu[1] * t)  # noqa: E731
        pos, neg = lo, hi
        assert self.f(at(pos)) >= 0 >= self.f(at(neg)), s
        for _ in range(120):
            mid = (pos + neg) / 2
            if self.f(at(mid)) >= 0:
                pos = mid
            else:
                neg = mid
        return at((pos + neg) / 2)

    def pair(self, p):
        return nearest(self.g1, p), nearest(self.g2, p)

    def curved(self, pair):
        """A corner and an edge whose line misses the corner: a parabola."""
        (k1, i1), (k2, i2) = pair
        if k1 == k2:
            return False
        corner, (a, b) = (self.g1[i1], (self.g2[i2], self.g2[i2 + 1])) if k1 == "corner" else \
            (self.g2[i2], (self.g1[i1], self.g1[i1 + 1]))
        ab = sub(b, a)
        return abs(cross(ab, sub(corner, a))) / norm(ab) > SAME_D

    def voronoi(self, step):
        L = float(norm(sub(self.t, self.s)))
        q = L / step
        n = max(2, int(round(q)) if abs(q - round(q)) <= 1e-9 else math.ceil(q))
        # The pieces, by a dense scan.
        m = 1000
        samples = [(u, self.pair(self.root(u))) for u in [1e-6] + [i / m for i in range(1, m)] + [1 - 1e-6]]
        turns = []
        for (ua, pa), (ub, pb) in zip(samples, samples[1:]):
            if pa != pb:
                turns.extend(self.transitions(D(ua), pa, D(ub), pb))
        curved = []
        for k in range(1, n):
            s = k / n
            p = self.root(s)
            if self.curved(self.pair(p)):
                curved.append((D(s), p))
        pts = sorted([(self.param(p), p) for p in turns] + curved, key=lambda x: x[0])
        seam = [self.s] + [p for _, p in pts] + [self.t]
        out = []
        for p in seam:
            if not out or norm(sub(p, out[-1])) > SAME_D:
                out.append(p)
        return out

    def transitions(self, ua, pa, ub, pb):
        """The turns between two scanned lines of different pairs, by bisection on the chord."""
        lo, hi = ua, ub
        for _ in range(100):
            mid = (lo + hi) / 2
            pm = self.pair(self.root(mid))
            if pm == pa:
                lo = mid
            elif pm == pb:
                hi = mid
            else:
                return self.transitions(ua, pa, mid, pm) + self.transitions(mid, pm, ub, pb)
        return [self.root((lo + hi) / 2)]

    def param(self, p):
        chord = sub(self.t, self.s)
        return dot(sub(p, self.s), chord) / dot(chord, chord)

    def rings(self, seam):
        inner = seam[1:-1]

        def arc(ring, frm, to):
            a, b = position(ring, frm), position(ring, to)
            n = len(ring)
            span = b - a if b > a else b - a + n
            out, k = [frm], int(math.floor(a)) + 1
            while k < a + span - D("1e-12"):
                out.append(ring[k % n])
                k += 1
            out.append(to)
            return out

        first = arc(self.a, self.t, self.s) + inner
        second = arc(self.b, self.s, self.t) + list(reversed(inner))
        return deduped(first), deduped(second)

    def in_overlap(self, p):
        n = len(self.o)
        return all(orient(self.o[i], self.o[(i + 1) % n], p) >= 0 for i in range(n))


def position(ring, p):
    n = len(ring)
    best = None
    for i in range(n):
        a, b = ring[i], ring[(i + 1) % n]
        if norm(sub(p, a)) <= SAME_D:
            return D(i)
        d = seg_dist(p, a, b)
        if best is None or d < best[0]:
            ab = sub(b, a)
            t = min(max(dot(sub(p, a), ab) / dot(ab, ab), D(0)), D(1))
            best = (d, D((i + 1) % n) if norm(sub(p, b)) <= SAME_D else D(i) + t)
    return best[1]


def nearest(g, p):
    best = None
    for k in range(len(g) - 1):
        a, b = g[k], g[k + 1]
        ab = sub(b, a)
        t = dot(sub(p, a), ab) / dot(ab, ab)
        if t <= 0:
            cand = (norm(sub(p, a)), ("corner", k))
        elif t >= 1:
            cand = (norm(sub(p, b)), ("corner", k + 1))
        else:
            cand = (abs(cross(ab, sub(p, a))) / norm(ab), ("edge", k))
        if best is None:
            best = cand
            continue
        tie = abs(cand[0] - best[0]) <= D("1e-12") * max(D(1), best[0])
        if (cand[0] < best[0] and not tie) or (tie and cand[1][0] == "edge" and best[1][0] == "corner"):
            best = cand
    return best[1]


def simple(ring):
    n = len(ring)
    for i in range(n):
        for j in range(i + 1, n):
            if j == i + 1 or (i == 0 and j == n - 1):
                continue
            a, b, c, d = ring[i], ring[(i + 1) % n], ring[j], ring[(j + 1) % n]
            o1, o2, o3, o4 = orient(a, b, c), orient(a, b, d), orient(c, d, a), orient(c, d, b)
            if o1 * o2 < 0 and o3 * o4 < 0:
                return False
            if (o1 == 0 and seg_dist(c, a, b) <= SAME_D) or (o2 == 0 and seg_dist(d, a, b) <= SAME_D) or \
                    (o3 == 0 and seg_dist(a, c, d) <= SAME_D) or (o4 == 0 and seg_dist(b, c, d) <= SAME_D):
                return False
    return True


def fl(p):
    return [float(p[0]), float(p[1])]


# ── The drawing ───────────────────────────────────────────────────────────────────────────────────────────────────
def layer(id_, name, locked=False):
    lay = oc.layer(id_, name)
    lay["locked"] = locked
    return lay


def rect(x0, y0, x1, y1):
    return [(X0 + x0, Y0 + y0), (X0 + x1, Y0 + y0), (X0 + x1, Y0 + y1), (X0 + x0, Y0 + y1)]


def area_ent(ring, layer_="ayak-izi", **kw):
    return dict({"kind": "polygon", "layerId": layer_, "attrs": {}, "pts": [{"x": x, "y": y} for x, y in ring]}, **kw)


def raster_ent(name, **kw):
    if name in oc.RESAMPLED:
        r = oc.resampled_raster(name)
    else:
        r = oc.seam_raster(name, {"dikis-a.tif": oc.A0}.get(name, oc.B0))
    r.update(kw)
    return r


def setup(first="dikis-a.tif", second="dikis-b.tif", extra=(), layers=(), first_kw=None, second_kw=None):
    ents = [dict(raster_ent(first, **(first_kw or {})), id=1), dict(raster_ent(second, **(second_kw or {})), id=2)]
    for e in extra:
        ents.append(dict(e, id=len(ents) + 1))
    return {"format": "kentos.document", "version": 2, "name": "Mozaik birleşme çizgisi",
            "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000.0, "workspace": "cad", "cadWork": "map"},
            "origin": {"x": X0, "y": Y0},
            "layers": [layer("0", "0"), layer("altlik", "Altlık"), layer("ayak-izi", "Ayak izleri")] + list(layers),
            "activeLayer": "0", "entities": ents, "uids": [uid(i + 1) for i in range(len(ents))],
            "styles": {"items": [], "categories": []}}


def corners(r):
    """A raster's four corners, GDAL's affine: pixel corners (0, h), (w, h), (w, 0), (0, 0)."""
    x0, a, b, y0, c, d = r["affine"]
    w, h = r["width"], r["height"]
    return [(x0 + a * i + b * j, y0 + c * i + d * j) for i, j in ((0, h), (w, h), (w, 0), (0, 0))]


# ── Radiometry ───────────────────────────────────────────────────────────────────────────────────────────────────
ROWS = {}


def pixels(name):
    if name not in ROWS:
        if name in oc.IMAGES:
            origin, shift, flat, change = oc.IMAGES[name]
            ROWS[name] = oc.image(origin, shift, flat, change)
        else:
            t, size, shift = oc.RESAMPLED[name]
            ROWS[name] = oc.image_at(t, size, shift)
    return ROWS[name]


def pixel_of(r, x, y):
    x0, a, b, y0, c, d = r["affine"]
    det = a * d - b * c
    dx, dy = x - x0, y - y0
    return (d * dx - b * dy) / det, (a * dy - c * dx) / det


def ground(r, i, j):
    x0, a, b, y0, c, d = r["affine"]
    return x0 + a * i + b * j, y0 + c * i + d * j


def in_area(rings, p):
    """Inside the outer ring and outside its holes (crossings of a ray to the east)."""
    def inside(ring):
        x, y, n, c = p[0], p[1], len(ring), False
        for i in range(n):
            (ax, ay), (bx, by) = ring[i], ring[(i + 1) % n]
            if (ay > y) != (by > y) and x < ax + (y - ay) * (bx - ax) / (by - ay):
                c = not c
        return c
    return inside(rings[0]) and not any(inside(h) for h in rings[1:])


def radiometry(seam, r1, r2, cell=1, corridor=None, avoid=(), voronoi=None):
    rows1, rows2 = pixels(r1["file"].split("/")[-1]), pixels(r2["file"].split("/")[-1])
    w1, h1, w2, h2 = r1["width"], r1["height"], r2["width"], r2["height"]
    gw, gh = -(-w1 // cell), -(-h1 // cell)
    vor = [fl(p) for p in voronoi]
    diff = {}
    total, searched = 0.0, 0

    def second_at(u, v):
        cu, cv = u - 0.5, v - 0.5
        ru, rv = oc_round(cu), oc_round(cv)
        if abs(cu - ru) <= 1e-6 and abs(cv - rv) <= 1e-6:
            i, j = int(ru), int(rv)
            return None if i < 0 or j < 0 or i >= w2 or j >= h2 else float(rows2[j][i])
        fu, fv = math.floor(cu), math.floor(cv)
        iu, iv = int(fu), int(fv)
        if iu - 1 < 0 or iv - 1 < 0 or iu + 2 >= w2 or iv + 2 >= h2:
            return None
        tx, ty = cu - fu, cv - fv
        s = 0.0
        for nn in range(-1, 3):
            row = 0.0
            for mm in range(-1, 3):
                row += float(rows2[iv + nn][iu + mm]) * oc.keys(tx - mm)
            s += row * oc.keys(ty - nn)
        return s

    for gj in range(gh):
        for gi in range(gw):
            worst = 0.0
            ok = True
            for dj in range(cell):
                for di in range(cell):
                    i, j = gi * cell + di, gj * cell + dj
                    if i >= w1 or j >= h1:
                        ok = False
                        break
                    cx, cy = ground(r1, i + 0.5, j + 0.5)
                    if not seam.in_overlap((D(cx), D(cy))) or any(in_area(a, (cx, cy)) for a in avoid):
                        ok = False
                        break
                    u, v = pixel_of(r2, cx, cy)
                    b = second_at(u, v)
                    if b is None:
                        ok = False
                        break
                    worst = max(worst, abs(float(rows1[j][i]) - b))
                if not ok:
                    break
            if not ok:
                continue
            centre = ground(r1, gi * cell + cell / 2, gj * cell + cell / 2)
            if corridor is not None and min(math.dist(centre, q) if False else seg_d(centre, vor[k], vor[k + 1])
                                            for k in range(len(vor) - 1)) > corridor:
                continue
            diff[(gj, gi)] = worst
            total += worst
            searched += 1
    assert searched, "no cell"
    centre_of = lambda key: ground(r1, key[1] * cell + cell / 2, key[0] * cell + cell / 2)  # noqa: E731
    s, t = fl(seam.s), fl(seam.t)

    def nearest_cell(p):
        best = None
        for key in sorted(diff):
            q = centre_of(key)
            dx, dy = q[0] - p[0], q[1] - p[1]
            dd = dx * dx + dy * dy
            if best is None or dd < best[0]:
                best = (dd, key)
        return best[1]

    frm, to = nearest_cell(s), nearest_cell(t)
    path, bottleneck = grid_path(diff, frm, to)
    if path is None:
        return None
    mean = sum(diff[k] for k in path) / len(path)
    turning = turns(path)
    pts = [seam.s] + [(D(x), D(y)) for x, y in (centre_of(k) for k in turning)] + [seam.t]
    a = r1["affine"]
    size = math.sqrt(abs(a[1] * a[5] - a[2] * a[4])) * cell
    return pts, {"bottleneck": bottleneck, "mean": mean, "searchedMean": total / searched, "cells": len(path),
                 "searched": searched, "size": size}


def oc_round(x):
    """Rust's f64::round: halves away from zero."""
    return math.floor(x + 0.5) if x >= 0 else -math.floor(-x + 0.5)


def seg_d(p, a, b):
    ab = (b[0] - a[0], b[1] - a[1])
    l2 = ab[0] * ab[0] + ab[1] * ab[1]
    t = ((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1]) / l2
    if t <= 0:
        return math.hypot(p[0] - a[0], p[1] - a[1])
    if t >= 1:
        return math.hypot(p[0] - b[0], p[1] - b[1])
    return abs(ab[0] * (p[1] - a[1]) - ab[1] * (p[0] - a[0])) / math.sqrt(l2)


MOVES = [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)]


def grid_path(diff, frm, to):
    import heapq
    best = {frm: diff[frm]}
    heap = [(diff[frm], frm)]
    while heap:
        key, at = heapq.heappop(heap)
        if key > best[at]:
            continue
        if at == to:
            break
        for dr, dc in MOVES:
            q = (at[0] + dr, at[1] + dc)
            if q not in diff:
                continue
            cand = max(key, diff[q])
            if cand < best.get(q, math.inf):
                best[q] = cand
                heapq.heappush(heap, (cand, q))
    if to not in best:
        return None, None
    bottleneck = best[to]
    cost, prev = {frm: 0.0}, {}
    heap = [(0.0, frm)]
    while heap:
        key, at = heapq.heappop(heap)
        if key > cost[at]:
            continue
        if at == to:
            break
        for dr, dc in MOVES:
            q = (at[0] + dr, at[1] + dc)
            if q not in diff or diff[q] > bottleneck:
                continue
            step = math.sqrt(2) if dr and dc else 1.0
            cand = key + step * (1.0 + (diff[at] + diff[q]) / 2.0)
            if cand < cost.get(q, math.inf):
                cost[q] = cand
                prev[q] = at
                heapq.heappush(heap, (cand, q))
    path, at = [to], to
    while at != frm:
        at = prev[at]
        path.append(at)
    return path[::-1], bottleneck


def turns(path):
    if len(path) <= 2:
        return list(path)
    step = lambda a, b: (b[0] - a[0], b[1] - a[1])  # noqa: E731
    out = [path[0]]
    for k in range(1, len(path) - 1):
        if step(path[k - 1], path[k]) != step(path[k], path[k + 1]):
            out.append(path[k])
    return out + [path[-1]]


# ── The answer ───────────────────────────────────────────────────────────────────────────────────────────────────
def two(v):
    """Rust's {:.2}, held off a half (the reference and KentOS reach it by other roundings)."""
    assert abs(abs(v) * 100 % 1 - 0.5) > 1e-6, v
    return f"{v:.2f}"


def one(v):
    assert abs(abs(v) * 10 % 1 - 0.5) > 1e-6, v
    return f"{v:.1f}"


def built(s, inp, layer_=LAYER, opened=True):
    """The command's answer for the drawing `s` and input `inp`."""
    ents = s["entities"]
    r1 = next(e for e in ents if s["uids"][e["id"] - 1] == inp["first"].strip())
    r2 = next(e for e in ents if s["uids"][e["id"] - 1] == inp["second"].strip())
    if "footprints" in inp:
        f1, f2 = [[(p["x"], p["y"]) for p in next(e for e in ents if s["uids"][e["id"] - 1] == u)["pts"]]
                  for u in inp["footprints"]]
    else:
        f1, f2 = corners(r1), corners(r2)
    seam = Seam(f1, f2)
    vor = seam.voronoi(inp.get("step", 1.0))
    method = inp.get("method", "voronoi")
    difference = None
    if method == "radiometry":
        avoid = []
        if "avoid" in inp:
            for e in ents:
                if e["layerId"] == inp["avoid"] and e["kind"] == "polygon":
                    avoid.append([[(p["x"], p["y"]) for p in e["pts"]]])
        got = radiometry(seam, r1, r2, inp.get("cell", 1), inp.get("corridor"), avoid, vor)
        line, difference = got
    else:
        line = vor
    ring1, ring2 = seam.rings(line)
    assert simple(ring1) and simple(ring2)
    length = sum(float(norm(sub(line[k + 1], line[k]))) for k in range(len(line) - 1))
    ov = float(area(seam.o))
    a1, a2 = float(area(ring1)), float(area(ring2))
    kept = [a1 - (float(area(seam.a)) - ov), a2 - (float(area(seam.b)) - ov)]
    word = "eşit uzaklık (Voronoi)" if method == "voronoi" else "radyometrik fark"
    notes = [f"Mozaik elemanları ({word}): örtüşme {two(ov)} m², birleşme çizgisi {two(length)} m; birinci görüntü örtüşmenin "
             f"{two(kept[0])} m²'sini, ikinci {two(kept[1])} m²'sini kullanır."]
    notes.append({(True, True): "Birleşme çizgisinin uçları iki ayak izinin sınırlarının kesiştiği yerlerde.",
                  (False, False): "Birleşme çizgisinin uçları iki ayak izinin ortak sınırında, iki görüntünün kendi "
                                  "bölgelerinden eşit uzakta."}.get((seam.sc, seam.tc),
                 "Birleşme çizgisinin bir ucu sınırların kesiştiği yerde, öbürü ortak sınırda iki görüntünün kendi "
                 "bölgelerinden eşit uzakta."))
    if difference:
        notes.append(f"Radyometrik fark: birleşme çizgisi boyunca en büyük {one(difference['bottleneck'])}, ortalama "
                     f"{one(difference['mean'])}; aranan {difference['searched']} hücrenin ortalaması "
                     f"{one(difference['searchedMean'])}; hücre {two(difference['size'])} m.")
        notes.append("BÖHHBÜY m. 67(6): ortak alanda radyometrik düzeltme yapılmalıdır. Radyometrik fark düzeltilmiş "
                     "görüntülerde anlamlıdır (KentOS'un önerisi: çizgiyi düzeltmeden sonra kurun).")
    if "avoid" in inp:
        notes.append("Geçilmeyecek alanların piksellerini içeren hücre aranmadı: birleşme çizgisi onları kesmez.")
    notes.append("Mozaik elemanı görüntünün mozaikte kullanılan bölümüdür (TUCBS Ortogörüntü); BÖHHBÜY birleşme çizgisinin "
                 "yerine kural koymaz, kayıklık m. 67(6) ile ayrıca denetlenir (Ortofoto karşılaştırması › Kayıklık ölçümü).")
    out = {"method": method, "overlap": [fl(p) for p in seam.o], "overlapArea": ov, "seam": [fl(p) for p in line],
           "length": length,
           "ends": [{"at": fl(seam.s), "kind": "crossing" if seam.sc else "shared"},
                    {"at": fl(seam.t), "kind": "crossing" if seam.tc else "shared"}],
           "polygons": [{"image": inp["first"].strip(), "ring": [fl(p) for p in ring1], "area": a1, "kept": kept[0]},
                        {"image": inp["second"].strip(), "ring": [fl(p) for p in ring2], "area": a2, "kept": kept[1]}]}
    if difference:
        out["difference"] = difference
    out["layer"] = layer_
    if opened:
        out["opened"] = True
    out["notes"] = notes
    out["revision"] = "$current"
    return out, (ring1, ring2), {"voronoi": "eşit uzaklık (Voronoi)", "radiometry": "radyometrik fark"}[method]


def ok(name, s, op="plan", note=None, layer_=LAYER, opened=True, **inp):
    inp = dict({"first": uid(1), "second": uid(2)}, **inp)
    out, rings, word = built(s, inp, layer_, opened)
    case = {"name": name, "setup": s}
    if note:
        case["note"] = note
    if op != "execute":
        result = {"status": "completed", "output": None if op == "validate" else out, "warnings": []}
        case["steps"] = [{"op": op, "input": inp, "result": result, "expect": SAME, "within": 1e-7}]
        return case, out
    written = copy.deepcopy(out)
    for p in written["polygons"]:
        p["uid"] = "$any"
    first_slot = len(s["entities"]) + 1
    fields = {str(first_slot + k): {"layerId": layer_, "attrs": {"Görüntü": inp[("first", "second")[k]].strip(),
                                                                "Yöntem": word},
                                    "pts": [{"x": float(p[0]), "y": float(p[1])} for p in rings[k]]} for k in (0, 1)}
    expect = {"revision": "changed", "canUndo": True, "fields": fields}
    if opened:
        expect["nodes"] = {LAYER: {"name": LAYER_NAME, "parent": None}}
    case["steps"] = [
        {"op": "execute", "input": inp, "result": {"status": "completed", "output": written, "warnings": []},
         "expect": expect, "within": 1e-7},
        {"op": "undo", "returns": STEP, "expect": {"ids": [e["id"] for e in s["entities"]]}},
    ]
    return case, out


def refused(name, s, inp, code, message, path_):
    inp = dict({"first": uid(1), "second": uid(2)}, **inp)
    error = {"code": code, "message": message}
    if path_:
        error["path"] = path_
    return {"name": name, "setup": s,
            "steps": [{"op": "execute", "input": inp, "result": {"status": "failed", "error": error}, "expect": SAME}]}


def cases():
    out = []
    base = setup()

    # Voronoi on the rasters' extents: two strips side by side.
    c, o = ok("iki şerit yan yana: birleşme çizgisi örtüşmenin ortasından, uçları ortak kenarlarda", base)
    assert o["seam"] == [[X0 + 515.0, Y0 + 108.0], [X0 + 515.0, Y0 + 120.0]], o["seam"]
    assert [e["kind"] for e in o["ends"]] == ["shared", "shared"]
    out.append(c)
    out.append(ok("yazılır: iki mozaik elemanı Mozaik elemanları katmanına, katman aynı adımda açılır; geri alınır", base,
                  op="execute")[0])
    out.append(ok("doğrulama", base, op="validate")[0])
    with_layer = setup(layers=[layer("ortofoto-mozaik", "Ortofoto mozaiği")])
    out.append(ok("verilen katmana yazılır, katman açılmaz", with_layer, op="execute", layer_="ortofoto-mozaik", opened=False,
                  layer="ortofoto-mozaik")[0])

    # Given footprints.
    def given(f1, f2, extra=(), **kw):
        return setup(extra=[area_ent(f1), area_ent(f2)] + list(extra), **kw)

    stag = given(rect(500, 108, 520, 120), rect(510, 105, 530, 117))
    fps = {"footprints": [uid(3), uid(4)]}
    c, o = ok("kaydırılmış şeritler: birleşme çizgisi kesişmelerden köşelerin açıortaylarıyla (Pan ve ark.'nın AEFC'si)", stag, **fps)
    want = [[X0 + 510.0, Y0 + 108.0], [X0 + 514.5, Y0 + 112.5], [X0 + 515.5, Y0 + 112.5], [X0 + 520.0, Y0 + 117.0]]
    assert all(abs(a - b) < 1e-9 for p, q in zip(o["seam"], want) for a, b in zip(p, q)) and len(o["seam"]) == 4, o["seam"]
    out.append(c)
    out.append(ok("aralık 50 m: düz kesimler aralıktan bağımsız, aynı çizgi", stag, step=50.0, **fps)[0])
    mixed = given(rect(500, 108, 520, 120), rect(510, 105, 530, 120))
    c, o = ok("bir ucu kesişmede, öbürü ortak kenarda: önce açıortay, sonra orta çizgi", mixed, **fps)
    want = [[X0 + 510.0, Y0 + 108.0], [X0 + 515.0, Y0 + 113.0], [X0 + 515.0, Y0 + 120.0]]
    assert all(abs(a - b) < 1e-9 for p, q in zip(o["seam"], want) for a, b in zip(p, q)) and len(o["seam"]) == 3, o["seam"]
    out.append(c)
    # The second's north-west corner cut at 30°: at its corner (516, 120) the overlap's angle is 150°, and above the
    # corner's normal the seam is the parabola of that corner and the first's east edge (Pan et al.'s vertex case).
    slant = [(X0 + 510, Y0 + 108), (X0 + 530, Y0 + 108), (X0 + 530, Y0 + 120), (X0 + 516, Y0 + 120),
             (X0 + 510, Y0 + 120 - 6 * math.tan(math.radians(30)))]
    curved = given(rect(500, 108, 520, 120), slant)
    c, o = ok("eğri kesim: köşe ile kenar arasında parabol, noktaları aralıkla", curved, **fps)
    assert len(o["seam"]) >= 5 and abs(o["ends"][1]["at"][0] - (X0 + 518.0)) < 1e-9, o["seam"]
    out.append(c)
    c, o2 = ok("eğri kesim, aralık 0,25 m: parabolde daha çok nokta", curved, step=0.25, **fps)
    assert len(o2["seam"]) > len(o["seam"]), (o["seam"], o2["seam"])
    out.append(c)
    turned = setup(second="dikis-e.tif")
    c, o = ok("dönük ikinci görüntü (9°): sınırları iki yerde kesişir, çizgi üç açıortay kesimi", turned)
    assert [e["kind"] for e in o["ends"]] == ["crossing", "crossing"], o["ends"]
    out.append(c)
    cw = [(X0 + 510, Y0 + 108), (X0 + 510, Y0 + 120), (X0 + 530, Y0 + 120), (X0 + 530, Y0 + 114),
          (X0 + 530, Y0 + 108), (X0 + 510, Y0 + 108)]
    polyline_fp = setup(extra=[area_ent(rect(500, 108, 520, 120)),
                               {"kind": "polyline", "layerId": "ayak-izi", "attrs": {}, "closed": True,
                                "pts": [{"x": x, "y": y} for x, y in cw]}])
    out.append(ok("ayak izi saat yönünde, kapanış noktalı, düz kenarda köşeli kapalı çoklu çizgi: aynı çizgi",
                  polyline_fp, **fps)[0])

    # Radiometry on the synthetic images (footprints a hair inside, the ends off the pixel corners).
    rad = given(rect(500, 108, 519.97, 120), rect(510, 108, 530, 120))
    c, o = ok("radyometrik fark: en büyük farkı en küçük yol, değişen ve dokusuz yerlerin dışından", rad, method="radiometry", **fps)
    out.append(c)
    out.append(ok("radyometrik fark, hücre 2 piksel", rad, method="radiometry", cell=2, **fps)[0])
    out.append(ok("radyometrik fark, eşit uzaklık çizgisinden en çok 1 m", rad, method="radiometry", corridor=1.0, **fps)[0])
    bina = area_ent(rect(514.4, 114.4, 515.6, 115.6), layer_="bina")
    rad_avoid = given(rect(500, 108, 519.97, 120), rect(510, 108, 530, 120), extra=[bina],
                      layers=[layer("bina", "Binalar")])
    out.append(ok("radyometrik fark, binalardan geçmeden: birleşme çizgisi binayı kesmez", rad_avoid, method="radiometry",
                  avoid="bina", **fps)[0])
    out.append(ok("radyometrik fark yazılır ve geri alınır", rad, op="execute", method="radiometry", **fps)[0])
    rad_d = given(rect(500, 108, 519.97, 120), rect(510, 108, 529.99, 120), second="dikis-d.tif")
    out.append(ok("radyometrik fark, ikinci görüntü 8 cm: Keys kübik evrişimiyle birincinin piksel merkezinde okunur", rad_d,
                  method="radiometry", cell=2, **fps)[0])

    # Refusals.
    out.append(refused("aynı ortofoto iki kez", base, {"second": f" {uid(1)}"}, "invalid_input",
                       "Aynı ortofoto iki kez verildi; birleşme çizgisi iki ayrı görüntü arasında kurulur.", "second"))
    out.append(refused("adım 0", base, {"step": 0.0}, "invalid_input", "Adım 0'dan büyük olmalı (metre).", "step"))
    out.append(refused("hücre Voronoi'de verilmez", base, {"cell": 2}, "invalid_input",
                       "Hücre yalnız radyometrik fark yönteminde verilir.", "cell"))
    out.append(refused("hücre 65", base, {"method": "radiometry", "cell": 65}, "invalid_input",
                       "Hücre 1 ile 64 piksel arasında olmalı.", "cell"))
    out.append(refused("arama bandı Voronoi'de verilmez", base, {"corridor": 1.0}, "invalid_input",
                       "Arama bandı yalnız radyometrik fark yönteminde verilir.", "corridor"))
    out.append(refused("arama bandı 0", base, {"method": "radiometry", "corridor": 0.0}, "invalid_input",
                       "Arama bandı 0'dan büyük olmalı (metre); bütün örtüşme için boş bırakın.", "corridor"))
    out.append(refused("geçilmeyecek alanlar Voronoi'de verilmez", base, {"avoid": "bina"}, "invalid_input",
                       "Geçilmeyecek alanlar yalnız radyometrik fark yönteminde verilir.", "avoid"))
    out.append(refused("aynı ayak izi iki kez", stag, {"footprints": [uid(3), f"{uid(3)} "]}, "invalid_input",
                       "Aynı ayak izi iki kez verildi; her görüntünün kendi ayak izini verin.", "footprints"))
    out.append(refused("ayak izi çizimde yok", stag, {"footprints": [uid(99), uid(4)]}, "invalid_input",
                       f"“{uid(99)}” kimlikli ayak izi çizimde yok.", "footprints[0]"))
    out.append(refused("ayak izi raster", stag, {"footprints": [uid(3), uid(1)]}, "invalid_input",
                       "Ayak izi bir alan ya da kapalı bir çoklu çizgi olmalı.", "footprints[1]"))
    holed = given(rect(500, 108, 520, 120), rect(510, 108, 530, 120))
    holed["entities"][3]["holes"] = [{"pts": [{"x": X0 + 521, "y": Y0 + 110}, {"x": X0 + 522, "y": Y0 + 110},
                                              {"x": X0 + 522, "y": Y0 + 111}]}]
    out.append(refused("delikli ayak izi", holed, fps, "invalid_input",
                       "Ayak izi tek parçalı ve deliksiz bir alan olmalı.", "footprints[1]"))
    arc = given(rect(500, 108, 520, 120), rect(510, 108, 530, 120))
    arc["entities"][3]["bulges"] = [0.0, 0.2, 0.0, 0.0]
    out.append(refused("yaylı ayak izi", arc, fps, "invalid_input",
                       "Ayak izinin kenarları düz olmalı; yaylı ayak izi kullanılmaz.", "footprints[1]"))
    notch = [(X0 + 500, Y0 + 108), (X0 + 520, Y0 + 108), (X0 + 512, Y0 + 113), (X0 + 520, Y0 + 120),
             (X0 + 500, Y0 + 120)]
    out.append(refused("dışbükey olmayan ayak izi", given(notch, rect(510, 108, 530, 120)), fps, "invalid_input",
                       "Birinci ayak izi dışbükey değil; birleşme çizgisi dışbükey ayak izleri arasında "
                       "kurulur. İçbükey ayak izini parçalara bölün.", "footprints"))
    out.append(refused("örtüşmeyen görüntüler", given(rect(500, 108, 520, 120), rect(520, 108, 540, 120)), fps,
                       "invalid_input", "İki görüntü örtüşmüyor (ya da yalnız değiyor); birleşme çizgisi ancak örtüşmede kurulur.",
                       "footprints"))
    out.append(refused("ikinci birincinin içinde", given(rect(500, 108, 520, 120), rect(505, 110, 515, 118)), fps,
                       "invalid_input", "İkinci görüntü öbürünün içinde kalıyor: öbürü onu bütünüyle örtüyor, "
                       "aralarında birleşme çizgisi yok. Mozaikte üstte kalanı seçin.", "footprints"))
    out.append(refused("artı gibi kesişen ayak izleri", given(rect(500, 110, 530, 118), rect(510, 100, 520, 128)), fps,
                       "invalid_input", "Örtüşmenin sınırı iki görüntü arasında ikiden çok kez el değiştiriyor "
                       "(ayak izleri artı gibi kesişiyor); birleşme çizgisi tek çizgiyle kurulamaz.", "footprints"))
    out.append(refused("birinci raster değil", stag, {"first": uid(3)}, "not_a_raster",
                       f"“{uid(3)}” bir raster değil; ortofotoyu seçin.", "first"))
    out.append(refused("kimlik UUID değil", base, {"first": "x"}, "invalid_uid", "“x” geçerli bir nesne kimliği değil.",
                       "first"))
    out.append(refused("çizimde olmayan raster", base, {"second": uid(77)}, "entity_not_found",
                       f"“{uid(77)}” kimlikli nesne çizimde yok.", "second"))
    out.append(refused("ikinci raster projenin sisteminde değil", setup(second_kw={"srid": 5256}), {}, "crs_mismatch",
                       "Ortofotonun koordinat sistemi EPSG:5256, projeninki EPSG:5254; birleşme çizgisi aynı sistemdeki "
                       "görüntüler arasında kurulur. Önce projeye çevirin (Raster ekle › Projenin sistemine çevir ve "
                       "ekle).", "second"))
    out.append(refused("verilen katman yok", base, {"layer": "yok"}, "layer_not_found", "“yok” katmanı çizimde yok.",
                       "layer"))
    out.append(refused("katman kilitli", setup(layers=[layer(LAYER, LAYER_NAME, locked=True)]), {}, "layer_locked",
                       f"“{LAYER_NAME}” katmanı kilitli; mozaik elemanları bu katmana yazılır. Kilidi Katmanlar "
                       "panelinden açın.", "layer"))
    out.append(refused("geçilmeyecek alanların katmanı yok", rad, dict(fps, method="radiometry", avoid="yok"), "layer_not_found",
                       "Geçilmeyecek alanların katmanı “yok” çizimde yok.", "avoid"))
    wall = area_ent(rect(509.0, 113.0, 521.0, 114.0), layer_="bina")
    walled = given(rect(500, 108, 519.97, 120), rect(510, 108, 530, 120), extra=[wall],
                   layers=[layer("bina", "Binalar")])
    out.append(refused("geçilmeyecek alan örtüşmeyi boydan boya kesiyor: yol yok", walled,
                       dict(fps, method="radiometry", avoid="bina"), "invalid_input",
                       "Birleşme çizgisinin iki ucu arasında aranacak hücrelerden geçen yol yok (geçilmeyecek alanlar ya da "
                       "arama bandı yolu kesiyor); arama bandını genişletin ya da geçilmeyecek alanları gözden geçirin.", "avoid"))
    missing = setup(first_kw={"file": "$fixture:orthophoto/v1/files/yok.tif"})
    out.append(refused("radyometrik farkta görüntünün dosyası yok", missing, {"method": "radiometry"}, "file_not_found",
                       "Ortofotonun dosyası “$fixture:orthophoto/v1/files/yok.tif” bulunamadı.", "first"))
    out.append(refused("sürüm yazısı geçersiz", base, {"expectedRevision": "abc"}, "invalid_revision",
                       "Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü "
                       "belgeden ya da komutun planından alın.", "expectedRevision"))
    out.append({"name": "beklenen sürüm eski: çatışma, hiçbir şey yazılmaz", "setup": base, "steps": [
        {"op": "execute", "input": {"first": uid(1), "second": uid(2), "expectedRevision": "7"},
         "result": {"status": "conflict", "error": {
             "code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey "
                                                     "yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
             "path": "expectedRevision", "revision": "$current"}}, "expect": SAME}]})
    return out


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.orthophoto.seamlines", "commandVersion": 1,
           "title": "Mozaik birleşme çizgisi",
           "note": "ADR 4700. Bağımsız başvuru scripts/fixtures/orthophoto_seamlines_cases.py: Pan ve ark. 2009'un "
                   "örtüşmeli alan Voronoi diyagramı 40 basamakla, birleşme çizgisinin kesimleri yoğun taramayla bulunup dönüm "
                   "noktaları ikiye bölmeyle; radyometri (Chon ve ark. 2010) orthophoto_compare_cases.py'nin sentetik "
                   "görüntülerinin piksel değerleriyle; KentOS kodu yok.",
           "setup": setup(), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok_ = OUT.exists() and OUT.read_text("utf-8") == text
        print("Mozaik birleşme çizgisi ortak durumları güncel." if ok_ else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok_ else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
