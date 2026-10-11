#!/usr/bin/env python3
"""Independent reference of the Örnekleme tools (docs/adr/0220), written from the ADR alone, no KentOS code.

Writes fixtures/processing/v1/sampling.kcad (the drawing) and fixtures/processing/v1/sampling.json (the tools run on
it, in the processing cases' format, fixtures/processing/README.md).

How it computes, independently of the core's ways:

1. The grids by the ADR's lattice formulas (§2) in float64, the same operations in the same order; the cells kept with
   the box by the separating axis test as the ADR words it; Nesnelere değenler as GEOS's distance (shapely) within
   1 mm, every cell checked to be clearly in or out (no distance near 1 mm).
2. Whether a point is in or on an area exactly, on fractions: within 1 µm of an edge (the exact distance), else the
   crossing number; every decided point checked to be clearly on one side of 1 µm.
3. The seeded numbers by the ADR's rule (§4) on Python's integers: SplitMix64, xoshiro256**, the top 53 bits.
4. Points along paths and their bearings and elevations by the ADR's rule (§5), arcs from their bulges by the
   bulge's definition; random points on paths the same way.
5. Pieces by the ADR's rule (§6): halves clipped by GEOS (shapely's intersection), paths cut on the line here, the
   cut's place from the ring's vertex nearest the middle.
"""

import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

import shapely
from shapely.geometry import LineString, Point, Polygon, box

sys.path.insert(0, str(Path(__file__).resolve().parent))
from layer_ops_cases import new_id  # noqa: E402
from numeric_display import shown  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
PROC_DIR = ROOT / "fixtures" / "processing" / "v1"
E, N = 487000, 4420000
TOL = 1e-8
ON = 1e-6  # the overlay's boundary (§3)
NEAR = 1e-3  # Konuma göre seç's Kesişen (§2)
MOST = 1_000_000
MASK = (1 << 64) - 1


def P(x, y):
    return {"x": E + x, "y": N + y}


def xy(p):
    return (p["x"], p["y"])


def out(p):
    return {"x": float(p[0]), "y": float(p[1])}


# ── The seeded numbers (§4) ────────────────────────────────────────────────


def rotl(x, k):
    return ((x << k) | (x >> (64 - k))) & MASK


class Sequence:
    def __init__(self, seed):
        x = seed & MASK
        s = []
        for _ in range(4):
            x = (x + 0x9E3779B97F4A7C15) & MASK
            z = x
            z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & MASK
            z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & MASK
            s.append(z ^ (z >> 31))
        self.s = s

    def next_u64(self):
        s = self.s
        result = (rotl((s[1] * 5) & MASK, 7) * 9) & MASK
        t = (s[1] << 17) & MASK
        s[2] ^= s[0]
        s[3] ^= s[1]
        s[1] ^= s[2]
        s[0] ^= s[3]
        s[2] ^= t
        s[3] = rotl(s[3], 45)
        return result

    def unit(self):
        return (self.next_u64() >> 11) / 9007199254740992.0


def at_lattice(seed, i, j):
    return Sequence(seed ^ (((i & MASK) * 0x9E3779B97F4A7C15) & MASK) ^ (((j & MASK) * 0xC2B2AE3D27D4EB4F) & MASK))


# ── Areas and paths ────────────────────────────────────────────────────────


def bulge_arc(a, b, bulge):
    """The arc a bulge makes on a chord (its definition: the bulge is the tangent of a quarter of the swept angle)."""
    if bulge == 0:
        return None
    sweep = 4 * math.atan(bulge)
    dx, dy = b[0] - a[0], b[1] - a[1]
    chord = math.hypot(dx, dy)
    r = chord / (2 * math.sin(abs(sweep) / 2))
    # The centre lies off the chord's middle, left of a→b for a counter-clockwise arc.
    mx, my = (a[0] + b[0]) / 2, (a[1] + b[1]) / 2
    h = r * math.cos(sweep / 2)
    s = 1 if sweep > 0 else -1
    cx, cy = mx - s * h * dy / chord, my + s * h * dx / chord
    a0 = math.atan2(a[1] - cy, a[0] - cx)
    return (cx, cy, r, a0, sweep)


def edges_of(pts, bulges, closed):
    n = len(pts)
    count = n if closed else n - 1
    res = []
    for i in range(count):
        a, b = pts[i], pts[(i + 1) % n]
        bg = bulges[i] if bulges and i < len(bulges) else 0
        arc = bulge_arc(a, b, bg)
        res.append(("arc", arc, a, b) if arc else ("seg", None, a, b))
    return res


def edge_len(e):
    if e[0] == "seg":
        return math.hypot(e[3][0] - e[2][0], e[3][1] - e[2][1])
    _, (cx, cy, r, a0, sweep), _, _ = e
    return r * abs(sweep)


def edge_at(e, t):
    if e[0] == "seg":
        a, b = e[2], e[3]
        return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)
    _, (cx, cy, r, a0, sweep), _, _ = e
    th = a0 + sweep * t
    return (cx + math.cos(th) * r, cy + math.sin(th) * r)


def edge_dir(e, t):
    if e[0] == "seg":
        a, b = e[2], e[3]
        dx, dy = b[0] - a[0], b[1] - a[1]
        length = math.hypot(dx, dy)
        return (dx / length, dy / length) if length > 0 else (0.0, 0.0)
    _, (cx, cy, r, a0, sweep), _, _ = e
    th = a0 + sweep * t
    w = 1 if sweep > 0 else -1
    return (-math.sin(th) * w, math.cos(th) * w)


def bearing_of(d):
    b = math.atan2(d[0], d[1])
    return b + 2 * math.pi if b < 0 else b


class Walk:
    """A path walked by length (§5): its edges, the length at each edge's start, its vertices' elevations."""

    def __init__(self, pts, bulges, closed, zs=None):
        self.edges = edges_of(pts, bulges, closed)
        self.closed = closed
        self.zs = zs or []
        self.cum, s = [], 0.0
        for e in self.edges:
            self.cum.append(s)
            s += edge_len(e)
        self.length = s

    def z(self, i):
        if not self.zs:
            return None
        return self.zs[i % len(self.zs)] if self.closed else (self.zs[i] if i < len(self.zs) else None)

    def at(self, s):
        i = max(0, sum(1 for c in self.cum if c <= s) - 1)
        e = self.edges[i]
        length = edge_len(e)
        t = min(1.0, max(0.0, (s - self.cum[i]) / length)) if length > 0 else 0.0
        if t == 0.0:
            z = self.z(i)
        elif t == 1.0:
            z = self.z(i + 1)
        else:
            za, zb = self.z(i), self.z(i + 1)
            z = za + (zb - za) * t if za is not None and zb is not None else None
        return edge_at(e, t), z, edge_dir(e, t)


def walks_of(e):
    k = e["kind"]
    if k == "line":
        zs = [e.get("za"), e.get("zb")] if e.get("za") is not None or e.get("zb") is not None else None
        return [Walk([xy(e["a"]), xy(e["b"])], None, False, zs)]
    if k == "polyline":
        ws = [Walk([xy(p) for p in e["pts"]], e.get("bulges"), False, e.get("zs"))]
        ws += [Walk([xy(p) for p in q["pts"]], q.get("bulges"), False, q.get("zs")) for q in e.get("parts", [])]
        return ws
    if k == "polygon":
        ws = [Walk([xy(p) for p in e["pts"]], e.get("bulges"), True, e.get("zs"))]
        ws += [Walk([xy(p) for p in h["pts"]], h.get("bulges"), True, h.get("zs")) for h in e.get("holes", [])]
        return ws
    if k == "arc":
        c, r = xy(e["c"]), e["r"]
        sweep = (e["a1"] - e["a0"]) % (2 * math.pi) or 2 * math.pi
        w = Walk([], None, False)
        w.edges = [("arc", (c[0], c[1], r, e["a0"], sweep), None, None)]
        w.cum, w.length, w.zs = [0.0], r * abs(sweep), []
        return [w]
    if k == "circle":
        c, r = xy(e["c"]), e["r"]
        w = Walk([], None, True)
        w.edges = [("arc", (c[0], c[1], r, 0.0, 2 * math.pi), None, None)]
        w.cum, w.length, w.zs = [0.0], r * 2 * math.pi, []
        return [w]
    return []


def rings_of(e):
    """An area's rings as float points (outer first, then holes); straight edges only."""
    assert e["kind"] == "polygon" and not any(e.get("bulges") or []), e["id"]
    return [[xy(p) for p in e["pts"]]] + [[xy(p) for p in h["pts"]] for h in e.get("holes", [])]


def exact_place(p, rings):
    """In or on (within 1 µm) the area, exactly: on fractions; refused near the 1 µm line."""
    px, py = F(p[0]), F(p[1])
    tol2 = F(ON) * F(ON)
    best = None
    for ring in rings:
        n = len(ring)
        for i in range(n):
            ax, ay = F(ring[i][0]), F(ring[i][1])
            bx, by = F(ring[(i + 1) % n][0]), F(ring[(i + 1) % n][1])
            dx, dy = bx - ax, by - ay
            l2 = dx * dx + dy * dy
            t = ((px - ax) * dx + (py - ay) * dy) / l2 if l2 else F(0)
            t = min(F(1), max(F(0), t))
            qx, qy = ax + t * dx, ay + t * dy
            d2 = (px - qx) ** 2 + (py - qy) ** 2
            best = d2 if best is None else min(best, d2)
    assert not (F(81, 100) * tol2 < best < F(121, 100) * tol2), f"{p} 1 µm sınırına çok yakın"
    if best <= tol2:
        return True
    inside = []
    for ring in rings:
        n, odd = len(ring), False
        for i in range(n):
            ax, ay = F(ring[i][0]), F(ring[i][1])
            bx, by = F(ring[(i + 1) % n][0]), F(ring[(i + 1) % n][1])
            if (ay > py) != (by > py):
                x = ax + (py - ay) * (bx - ax) / (by - ay)
                if px < x:
                    odd = not odd
        inside.append(odd)
    return inside[0] and not any(inside[1:])


def area_box(rings):
    pts = [p for r in rings for p in r]
    return (min(p[0] for p in pts), min(p[1] for p in pts), max(p[0] for p in pts), max(p[1] for p in pts))


# ── The drawing ────────────────────────────────────────────────────────────


def layer(i, name, color, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25}, "children": []}


LAYERS = [
    layer("cizim", "Çizim", "fg"),
    layer("parsel", "Parsel", "#E5484D"),
    layer("mahalle", "Mahalle", "#3E63DD"),
    layer("nokta", "Nokta", "#0090FF"),
    layer("yol", "Yol", "#AB4ABA"),
    layer("diger", "Diğer", "#F76B15"),
    layer("ince", "İnce", "#12A594"),
    layer("buyuk", "Büyük", "#8E4EC6"),
    layer("kilitli", "Arşiv", "#8E8C99", locked=True),
]


def ent(i, kind, lay, attrs, **geom):
    e = {"kind": kind, "id": i, "layerId": lay, "attrs": attrs}
    e.update(geom)
    return e


def star(n, r0, r1, cx, cy):
    return [(round(cx + (r1 if i % 2 == 0 else r0) * math.cos(math.pi * i / n), 3),
             round(cy + (r1 if i % 2 == 0 else r0) * math.sin(math.pi * i / n), 3)) for i in range(2 * n)]


STAR = star(20, 40.0, 100.0, 450.0, 100.0)
HOLE = star(30, 20.0, 60.0, 455.0, 325.0)[::-1]
ZIG = [(350.0 + 6.0 * i, round(450.0 + (0.0 if i % 2 == 0 else 8.0) + 0.37 * i, 2)) for i in range(31)]

ENTITIES = [
    # An ada of parcels: one edge on y = 90 and a hole on round coordinates (points on them count, §3).
    ent(1, "polygon", "parsel", {"Ada": "120", "Parsel": "1"}, pts=[P(12.3, 8.7), P(61.9, 4.2), P(66.4, 41.5), P(15.8, 47.3)]),
    ent(2, "polygon", "parsel", {"Ada": "120", "Parsel": "2"}, pts=[P(61.9, 4.2), P(118.6, 6.9), P(121.2, 38.8), P(66.4, 41.5)]),
    ent(3, "polygon", "parsel", {"Ada": "120", "Parsel": "3"}, pts=[P(15.8, 47.3), P(66.4, 41.5), P(70, 90), P(20, 90)]),
    ent(4, "polygon", "parsel", {"Ada": "120", "Parsel": "4"}, pts=[P(66.4, 41.5), P(121.2, 38.8), P(125, 90), P(70, 90)],
        holes=[{"pts": [P(85, 55), P(85, 75), P(105, 75), P(105, 55)]}]),
    ent(5, "polygon", "parsel", {"Ada": "121", "Parsel": "5"},
        pts=[P(125.4, 6.2), P(170.5, 9.1), P(168.2, 52.3), P(147.7, 50.9), P(148.9, 28.4), P(123.6, 27.05)]),
    # A neighbourhood's boundary.
    ent(11, "polygon", "mahalle", {"Ad": "Yenimahalle"},
        pts=[P(-20.3, -30.6), P(150.2, -45.1), P(260.7, 10.4), P(240.1, 140.3), P(120.6, 170.2), P(10.4, 150.7), P(-35.2, 60.8)]),
    # Wells.
    *[ent(20 + k, "point", "nokta", {"No": f"K{k}"}, p=P(x, y)) for k, (x, y) in enumerate(
        [(30.5, 120.25), (75.2, 130.8), (140.4, 110.1), (200.7, 60.3), (230.1, 120.6), (180.3, 150.2), (95.5, 160.4),
         (10.2, 100.9), (55.7, 52.3), (160.0, 80.0)], start=1)],
    # Roads: with elevations, of two parts, a line, one with an arc.
    ent(31, "polyline", "yol", {"Ad": "Atatürk Cd."}, pts=[P(0, 200), P(40.5, 205.3), P(85.2, 198.6), P(130, 210.4)],
        zs=[100.0, 101.5, 99.8, 102.25]),
    ent(32, "polyline", "yol", {"Ad": "Çevre yolu"}, pts=[P(150, 190), P(190, 196.5)],
        parts=[{"pts": [P(200, 200), P(240.3, 230.7), P(260, 205)]}]),
    ent(33, "line", "yol", {"Ad": "Bağlantı"}, a=P(10, 230), b=P(70, 250), za=98.0, zb=104.0),
    ent(34, "polyline", "yol", {"Ad": "Kıvrım"}, pts=[P(90, 230), P(120, 230), P(150, 245)], bulges=[0.35, 0]),
    # Other kinds.
    ent(41, "circle", "diger", {"Ad": "Havuz"}, c=P(300, 50), r=18),
    ent(42, "arc", "diger", {"Ad": "Kavis"}, c=P(300, 110), r=15, a0=0.4, a1=2.6),
    ent(43, "polygon", "diger", {"Ad": "Kemer"}, pts=[P(280, 150), P(320, 150), P(320, 175), P(280, 175)], bulges=[0, 0, 0.3, 0]),
    # A sliver: its box a hundred metres across, itself two tenths of a millimetre thick.
    ent(51, "polygon", "ince", {"Ad": "Şerit"}, pts=[P(300, 200), P(400, 300), P(400, 300.0002), P(300, 200.0002)]),
    # Large objects to cut.
    ent(61, "polygon", "buyuk", {"Ad": "Yıldız"}, pts=[P(x, y) for x, y in STAR]),
    ent(62, "polygon", "buyuk", {"Ad": "Avlu"}, pts=[P(350, 230), P(560, 230), P(560, 420), P(350, 420)],
        holes=[{"pts": [P(x, y) for x, y in HOLE]}]),
    ent(63, "polyline", "buyuk", {"Ad": "Zikzak"}, pts=[P(x, y) for x, y in ZIG], zs=[round(50 + 0.5 * i, 2) for i in range(31)]),
    ent(71, "polygon", "kilitli", {"Parsel": "9"}, pts=[P(180, 10), P(215, 12), P(212, 45), P(178, 41)]),
]

DOCUMENT = {
    "format": "kentos.document",
    "version": 1,
    "name": "İşlem durumları: örnekleme",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                 "workspace": "gis", "drawingFont": "barlow"},
    "origin": {"x": E, "y": N},
    "layers": LAYERS,
    "activeLayer": "cizim",
    "entities": ENTITIES,
    "styles": {"items": [], "categories": []},
}


def by_id(i):
    return next(e for e in ENTITIES if e["id"] == i)


def objects(lay):
    return [e for e in ENTITIES if e["layerId"] == lay]


def L(i):
    return {"scope": "layer", "layerId": i}


SELECTION = {"scope": "selection"}

OUTPUT = {
    "sample.grid": ("Ağ", "#8E4EC6", "Ağ oluştur"),
    "sample.regular": ("Düzenli noktalar", "#3E63DD", "Düzenli noktalar"),
    "sample.random": ("Rastgele noktalar", "#E54666", "Rastgele noktalar"),
    "sample.alongLines": ("Hat noktaları", "#F76B15", "Hat boyunca nokta"),
    "sample.subdivide": ("Alt bölümler", "#30A46C", "Alt bölümlere ayır"),
}


def out_layer(tool):
    name, color, _ = OUTPUT[tool]
    return {"id": new_id(name, set()), "name": name,
            "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25, "fill": f"{color}26"}}


def case(cid, title, tool, values, expect, selection=None):
    c = {"id": cid, "title": title, "document": "sampling.kcad", "run": {"tool": tool}, "values": values, "expect": expect}
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


def refused(message):
    return {"status": "error", "message": message}


def warn(text):
    return {"level": "warn", "text": text}


def info(text):
    return {"level": "info", "text": text}


def length_text(v):
    return shown(v, 3)


def bearing_text(t):
    return shown((t * 200) / math.pi, 4)


def point_obj(lay, attrs, p, z=None):
    o = {"kind": "point", "layerId": lay, "attrs": attrs, "p": out(p)}
    if z is not None:
        o["z"] = z
    return o


def curved_note(inputs):
    c = sum(1 for e in inputs if e["kind"] in ("ellipse", "spline"))
    return [warn(f"{c} nesnenin eğrileri 0,1 mm'lik kirişlerle alındı.")] if c else []


# ── Ağ oluştur (§2) ────────────────────────────────────────────────────────

TOO_BIG_SCAN = "Ağ çok büyük: 4 000 000'dan çok hücre taranırdı; aralığı büyütün ya da kapsamı küçültün."
TOO_BIG = "Ağ çok büyük: 1 000 000'dan çok parça yazılırdı; aralığı büyütün ya da kapsamı küçültün."


def entity_box(e):
    k = e["kind"]
    if k == "point":
        pts = [xy(e["p"])] + [xy(q["p"]) for q in e.get("parts", [])]
        return (min(p[0] for p in pts), min(p[1] for p in pts), max(p[0] for p in pts), max(p[1] for p in pts))
    if k == "circle":
        c, r = xy(e["c"]), e["r"]
        return (c[0] - r, c[1] - r, c[0] + r, c[1] + r)
    if k == "line":
        pts = [xy(e["a"]), xy(e["b"])]
        return (min(p[0] for p in pts), min(p[1] for p in pts), max(p[0] for p in pts), max(p[1] for p in pts))
    pts = []
    for w in walks_of(e):
        for ed in w.edges:
            if ed[0] == "seg":
                pts += [ed[2], ed[3]]
            else:
                cx, cy, r, a0, sweep = ed[1]
                pts += [edge_at(ed, 0.0), edge_at(ed, 1.0)]
                for q in range(-8, 9):
                    th = q * math.pi / 2
                    lo, hi = (a0, a0 + sweep) if sweep > 0 else (a0 + sweep, a0)
                    if lo < th < hi:
                        pts.append((cx + r * math.cos(th), cy + r * math.sin(th)))
    return (min(p[0] for p in pts), min(p[1] for p in pts), max(p[0] for p in pts), max(p[1] for p in pts))


def extent(inputs):
    boxes = [entity_box(e) for e in inputs]
    return (min(b[0] for b in boxes), min(b[1] for b in boxes), max(b[2] for b in boxes), max(b[3] for b in boxes))


def start(lo, step, snap):
    return math.floor(lo / step) * step if snap else lo


def steps(fr, hi, step):
    n = math.ceil((hi - fr) / step)
    return n if n >= 1 else 1


def meets_box(ring, b):
    x0, y0 = min(p[0] for p in ring), min(p[1] for p in ring)
    x1, y1 = max(p[0] for p in ring), max(p[1] for p in ring)
    if not (x0 < b[2] and b[0] < x1 and y0 < b[3] and b[1] < y1):
        return False
    corners = [(b[0], b[1]), (b[2], b[1]), (b[2], b[3]), (b[0], b[3])]
    n = len(ring)
    for i in range(n):
        a, c = ring[i], ring[(i + 1) % n]
        nx, ny = c[1] - a[1], a[0] - c[0]
        if all((q[0] - a[0]) * nx + (q[1] - a[1]) * ny >= 0.0 for q in corners):
            return False
    return True


def lattice_cells(b, kind, dx, dy, side, snap, keep):
    """The grid's pieces as (row, col, points); 'scan' or 'big' when refused (counted before anything is built)."""
    if kind in ("rect", "lines"):
        ox, oy = start(b[0], dx, snap), start(b[1], dy, snap)
        nc, nr = steps(ox, b[2], dx), steps(oy, b[3], dy)
        if kind == "rect" and keep == "all" and nc * nr > MOST:
            return "big"
        if kind == "rect" and nc * nr > 4_000_000:
            return "scan"
        if kind == "lines":
            if nc + nr + 2 > MOST:
                return "big"
            east, north = ox + nc * dx, oy + nr * dy
            res = [(r, None, [(ox, oy + (nr + 1 - r) * dy), (east, oy + (nr + 1 - r) * dy)]) for r in range(1, nr + 2)]
            res += [(None, c, [(ox + (c - 1) * dx, oy), (ox + (c - 1) * dx, north)]) for c in range(1, nc + 2)]
            return res
        res = []
        for r in range(1, nr + 1):
            ya, yb = oy + (nr - r) * dy, oy + (nr - r + 1) * dy
            for c in range(1, nc + 1):
                xa, xb = ox + (c - 1) * dx, ox + c * dx
                res.append((r, c, [(xa, ya), (xb, ya), (xb, yb), (xa, yb)]))
        return res
    a = side
    u, h = a / 2.0, (a * math.sqrt(3.0)) / 2.0
    res = []
    if kind == "hexagon":
        ox, oy = (start(b[0], 3.0 * a, True), start(b[1], 2.0 * h, True)) if snap else (b[0], b[1])
        c0 = math.floor((b[0] - ox) / (3.0 * u)) - 1
        c1 = math.ceil((b[2] - ox) / (3.0 * u))
        r0 = math.floor((b[1] - oy) / (2.0 * h)) - 1
        r1 = math.ceil((b[3] - oy) / (2.0 * h))
        if (c1 - c0 + 1) * (r1 - r0 + 1) > 4_000_000:
            return "scan"

        def at(k, m):
            return (ox + k * u, oy + m * h)

        for r in range(r1, r0 - 1, -1):
            for c in range(c0, c1 + 1):
                kx, ky = 3 * c + 2, 2 * r + 1 + c % 2
                ring = [at(kx + 1, ky - 1), at(kx + 2, ky), at(kx + 1, ky + 1), at(kx - 1, ky + 1), at(kx - 2, ky), at(kx - 1, ky - 1)]
                if meets_box(ring, b):
                    res.append((r1 - r + 1, c - c0 + 1, ring))
        return res
    ox, oy = (start(b[0], a, True), start(b[1], 2.0 * h, True)) if snap else (b[0], b[1])
    j0 = math.floor((b[0] - ox) / a) - 1
    j1 = math.ceil((b[2] - ox) / a)
    r0 = math.floor((b[1] - oy) / h)
    r1 = max(math.ceil((b[3] - oy) / h) - 1, r0)

    def at(k, m):
        return (ox + k * u, oy + m * h)

    for r in range(r1, r0 - 1, -1):
        shift = r % 2
        for j in range(j0, j1 + 1):
            x = 2 * j + shift
            up = [at(x, r), at(x + 2, r), at(x + 1, r + 1)]
            down = [at(x + 2, r), at(x + 3, r + 1), at(x + 1, r + 1)]
            for k, ring in ((0, up), (1, down)):
                if meets_box(ring, b):
                    res.append((r1 - r + 1, 2 * (j - j0) + k + 1, ring))
    return res


def shapely_of(e):
    """An object as GEOS takes it for Kesişen; arcs as chords within a micrometre."""
    k = e["kind"]
    if k == "point":
        return Point(xy(e["p"]))
    if k == "line":
        return LineString([xy(e["a"]), xy(e["b"])])
    if k == "polygon":
        walks = walks_of(e)
        rings = []
        for w in walks:
            pts = []
            for ed in w.edges:
                if ed[0] == "seg":
                    pts.append(ed[2])
                else:
                    n = 2000
                    pts += [edge_at(ed, i / n) for i in range(n)]
            rings.append(pts)
        return Polygon(rings[0], rings[1:])
    parts = []
    for w in walks_of(e):
        pts = []
        for ed in w.edges:
            if ed[0] == "seg":
                pts += [ed[2], ed[3]]
            else:
                pts += [edge_at(ed, i / 2000) for i in range(2001)]
        parts.append(LineString(pts))
    return shapely.union_all(parts)


def grid_case(tool, inputs, kind, dx, dy, side, snap, keep):
    b = extent(inputs)
    got = lattice_cells(b, kind, dx, dy, side, snap, keep)
    if got == "scan":
        return refused(TOO_BIG_SCAN)
    if got == "big" or (keep == "all" and len(got) > MOST):
        return refused(TOO_BIG)
    if keep == "meeting" and kind != "lines":
        geoms = [shapely_of(e) for e in inputs]
        kept = []
        for row, col, ring in got:
            cell = Polygon(ring)
            d = min(cell.distance(g) for g in geoms)
            assert not (0.5e-3 < d < 2e-3), f"hücre {row},{col} nesneye 1 mm kadar yakın"
            if d <= NEAR:
                kept.append((row, col, ring))
        got = kept
    lay = out_layer(tool)["id"]
    added = []
    for row, col, pts in got:
        if kind == "lines":
            attrs = {"Satır": str(row), "Kuzey": length_text(pts[0][1])} if row else {"Sütun": str(col), "Doğu": length_text(pts[0][0])}
            added.append({"kind": "line", "layerId": lay, "attrs": attrs, "a": out(pts[0]), "b": out(pts[1])})
        else:
            added.append({"kind": "polygon", "layerId": lay, "attrs": {"Satır": str(row), "Sütun": str(col)}, "pts": [out(p) for p in pts]})
    what = "çizgi" if kind == "lines" else "hücre"
    return ok(tool, added, f"{len(added)} {what} yazıldı.", [], len(added))


# ── Düzenli noktalar (§3) ──────────────────────────────────────────────────


def regular_case(tool, inputs, dx, dy, snap, shift, seed):
    rings = [rings_of(e) for e in inputs]
    boxes = [area_box(r) for r in rings]
    allb = (min(b[0] for b in boxes), min(b[1] for b in boxes), max(b[2] for b in boxes), max(b[3] for b in boxes))
    ox, oy, half = (0.0, 0.0, 0.0) if snap else (allb[0], allb[1], 0.5)
    reach = shift if shift > 0 else 0.0

    def rng(lo, hi, o, step):
        return (math.floor((lo - reach - o) / step - half), math.ceil((hi + reach - o) / step - half))

    lay = out_layer(tool)["id"]
    added, empty = [], 0
    for e, rs, b in zip(inputs, rings, boxes):
        (i0, i1), (j0, j1) = rng(b[0], b[2], ox, dx), rng(b[1], b[3], oy, dy)
        before = len(added)
        for j in range(j1, j0 - 1, -1):
            for i in range(i0, i1 + 1):
                p = (ox + (i + half) * dx, oy + (j + half) * dy)
                if shift > 0:
                    q = at_lattice(seed, i, j)
                    ux, uy = q.unit(), q.unit()
                    p = (p[0] + (2.0 * ux - 1.0) * shift, p[1] + (2.0 * uy - 1.0) * shift)
                if exact_place(p, rs):
                    added.append(point_obj(lay, dict(e["attrs"]), p))
        if len(added) == before:
            empty += 1
    log = curved_note(inputs)
    if empty:
        log.append(info(f"{empty} alana nokta düşmedi."))
    return ok(tool, added, f"{len(inputs)} alana {len(added)} nokta yazıldı.", log, len(added))


# ── Rastgele noktalar (§4) ─────────────────────────────────────────────────

DRAWS = 10_000


def net_area(rings):
    def signed(r):
        return sum(r[i][0] * r[(i + 1) % len(r)][1] - r[(i + 1) % len(r)][0] * r[i][1] for i in range(len(r))) / 2

    return abs(signed(rings[0])) - sum(abs(signed(h)) for h in rings[1:])


def random_case(tool, inputs, counts, minimum, tries, seed, unread=0):
    seq = Sequence(seed)
    written = []
    d2 = minimum * minimum
    tries = max(1, tries) if minimum > 0 else 1
    lay = out_layer(tool)["id"]
    added, refused_n, thin, thin_lost = [], 0, 0, 0

    def close(p):
        return minimum > 0 and any((p[0] - q[0]) * (p[0] - q[0]) + (p[1] - q[1]) * (p[1] - q[1]) < d2 for q in written)

    for e, n in zip(inputs, counts):
        if e["kind"] == "polygon":
            rs = rings_of(e)
            x0, y0, x1, y1 = area_box(rs)
            w, h = x1 - x0, y1 - y0
            k = 0
            while k < n:
                k += 1
                placed = False
                gave_up = False
                for _ in range(tries):
                    found = None
                    for _ in range(DRAWS):
                        u = seq.unit()
                        v = seq.unit()
                        p = (x0 + u * w, y0 + v * h)
                        if exact_place(p, rs):
                            found = p
                            break
                    if found is None:
                        thin += 1
                        thin_lost += n - k + 1
                        gave_up = True
                        break
                    if close(found):
                        continue
                    written.append(found)
                    added.append(point_obj(lay, dict(e["attrs"]), found))
                    placed = True
                    break
                if gave_up:
                    break
                if not placed:
                    refused_n += 1
        else:
            walks = walks_of(e)
            total = 0.0
            for wk in walks:
                total += wk.length
            for _ in range(n):
                placed = False
                for _ in range(tries):
                    u = seq.unit()
                    rest, kk = u * total, len(walks) - 1
                    for k, wk in enumerate(walks):
                        if rest < wk.length or k == len(walks) - 1:
                            kk = k
                            break
                        rest -= wk.length
                    s = min(rest, walks[kk].length)
                    p, z, _ = walks[kk].at(s)
                    if close(p):
                        continue
                    written.append(p)
                    attrs = dict(e["attrs"])
                    attrs["Uzaklık"] = length_text(s)
                    if len(walks) > 1:
                        attrs["Parça"] = str(kk + 1)
                    added.append(point_obj(lay, attrs, p, z))
                    placed = True
                    break
                if not placed:
                    refused_n += 1
    log = []
    if unread:
        log.append(warn(f"{unread} nesnede nokta sayısı okunamadı; atlandı."))
    if refused_n:
        log.append(warn(f"{refused_n} nokta en az uzaklık sağlanamadığı için yazılmadı."))
    if thin:
        log.append(warn(f"{thin} alan kutusuna göre çok ince: {thin_lost} noktası 10 000 denemede bulunamadı."))
    log += curved_note(inputs)
    return ok(tool, added, f"{len(inputs)} nesneye {len(added)} nokta yazıldı.", log, len(added))


# ── Hat boyunca nokta (§5) ─────────────────────────────────────────────────


def distances(length, closed, mode, step, start_, end, parts, ends):
    eps = 1e-9 * max(1.0, length)
    res = []
    if mode == "interval":
        if step > 0:
            k = 0
            while True:
                s = start_ + k * step
                if not (s <= length - end + eps):
                    break
                res.append(min(s, length))
                k += 1
        if closed:
            res = [s for s in res if s <= length - eps]
        if ends:
            if not res or res[0] > eps:
                res.insert(0, 0.0)
            if not closed and (not res or res[-1] < length - eps):
                res.append(length)
    else:
        n = max(1, parts)
        if ends:
            res.append(0.0)
        res += [(length * k) / n for k in range(1, n)]
        if ends and not closed:
            res.append(length)
    return res


def along_case(tool, inputs, mode, step, start_, end, parts, ends):
    lay = out_layer(tool)["id"]
    added = []
    for e in inputs:
        walks = walks_of(e)
        for k, w in enumerate(walks):
            for s in distances(w.length, w.closed, mode, step, start_, end, parts, ends):
                p, z, d = w.at(s)
                attrs = dict(e["attrs"])
                attrs["Uzaklık"] = length_text(s)
                attrs["Semt"] = bearing_text(bearing_of(d))
                if len(walks) > 1:
                    attrs["Parça"] = str(k + 1)
                added.append(point_obj(lay, attrs, p, z))
    return ok(tool, added, f"{len(inputs)} nesneden {len(added)} nokta yazıldı.", curved_note(inputs), len(added))


# ── Alt bölümlere ayır (§6) ────────────────────────────────────────────────

DEPTH = 50


def signed_area(r):
    return sum(r[i][0] * r[(i + 1) % len(r)][1] - r[(i + 1) % len(r)][0] * r[i][1] for i in range(len(r))) / 2


def lowest(r):
    best = r[0]
    for p in r[1:]:
        if p[1] < best[1] or (p[1] == best[1] and p[0] < best[0]):
            best = p
    return best


def settled(r, ccw):
    r = list(r)
    if (signed_area(r) > 0) != ccw:
        r.reverse()
    k = r.index(lowest(r))
    return r[k:] + r[:k]


def poly_rings(g):
    """A shapely polygon's rings without the repeated closing point."""
    return [list(g.exterior.coords)[:-1]] + [list(i.coords)[:-1] for i in g.interiors]


def polygons_of(g):
    if g.is_empty:
        return []
    if g.geom_type == "Polygon":
        return [g] if g.area > 0 else []
    if hasattr(g, "geoms"):
        return [q for part in g.geoms for q in polygons_of(part)]
    return []


def area_pivot(rings, b, across_x):
    lo, hi = (b[0], b[2]) if across_x else (b[1], b[3])
    middle = (lo + hi) / 2.0
    n = sum(len(r) for r in rings)
    ring = rings[0]
    if len(rings) > 1 and n >= 2 * len(rings[0]):
        best, size = None, None
        for hring in rings[1:]:
            s = abs(signed_area(hring))
            if best is None or s > size or (s == size and (lowest(hring)[1], lowest(hring)[0]) < (lowest(best)[1], lowest(best)[0])):
                best, size = hring, s
        ring = best
    gap, c = math.inf, middle
    for p in ring:
        v = p[0] if across_x else p[1]
        g = abs(v - middle)
        if g < gap or (g == gap and v < c):
            gap, c = g, v
    if abs(c - lo) <= 1e-12 or abs(c - hi) <= 1e-12:
        return middle
    return c


def ring_box(pts):
    return (min(p[0] for p in pts), min(p[1] for p in pts), max(p[0] for p in pts), max(p[1] for p in pts))


def split_area(rings, depth, most, out_):
    n = sum(len(r) for r in rings)
    if n <= most or depth > DEPTH:
        out_.append(("area", rings))
        return
    b = ring_box(rings[0])
    w, h = b[2] - b[0], b[3] - b[1]
    across_x = w > h
    cut = area_pivot(rings, b, across_x)
    # A vertex within 1 µm of the line counts as on it (§6): none of the cases' rings is that near without being on it.
    k = 0 if across_x else 1
    assert not any(0 < abs(p[k] - cut) <= 1e-5 for r in rings for p in r), f"köşe kesim doğrusuna 1 µm kadar yakın: {cut}"
    g = Polygon(rings[0], rings[1:])
    halves = [(b[0], b[1], cut, b[3]), (cut, b[1], b[2], b[3])] if across_x else [(b[0], b[1], b[2], cut), (b[0], cut, b[2], b[3])]
    for hb in halves:
        for q in polygons_of(g.intersection(box(*hb))):
            split_area(poly_rings(q), depth + 1, most, out_)


def cut_seg(a, q, across_x, cut):
    ca, cb = (a[0], q[0]) if across_x else (a[1], q[1])
    pieces = []
    if (ca < cut < cb) or (ca > cut > cb):
        t = (cut - ca) / (cb - ca)
        p = (cut, a[1] + (q[1] - a[1]) * t) if across_x else (a[0] + (q[0] - a[0]) * t, cut)
        if math.hypot(p[0] - a[0], p[1] - a[1]) > ON:
            pieces.append((a, p))
            a = p
    pieces.append((a, q))
    return pieces


def chain(segs):
    runs = []
    for s in segs:
        if runs and math.hypot(runs[-1][-1][1][0] - s[0][0], runs[-1][-1][1][1] - s[0][1]) <= ON:
            runs[-1].append(s)
        else:
            runs.append([s])
    return runs


def split_path(segs, depth, most, out_):
    n = len(segs) + 1
    if n <= most or depth > DEPTH:
        out_.append(("path", segs))
        return
    pts = [segs[0][0]] + [s[1] for s in segs]
    b = ring_box(pts)
    w, h = b[2] - b[0], b[3] - b[1]
    across_x = w > h
    cut = ((b[0] + b[2]) / 2.0) if across_x else ((b[1] + b[3]) / 2.0)
    low, high = [], []
    for a, q in segs:
        for piece in cut_seg(a, q, across_x, cut):
            m = (piece[0][0] + (piece[1][0] - piece[0][0]) * 0.5, piece[0][1] + (piece[1][1] - piece[0][1]) * 0.5)
            (low if (m[0] if across_x else m[1]) <= cut else high).append(piece)
    for run in chain(low) + chain(high):
        split_path(run, depth + 1, most, out_)


def seg_t(p, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    l2 = dx * dx + dy * dy
    t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2
    return min(1.0, max(0.0, t)), math.hypot(p[0] - (a[0] + dx * t), p[1] - (a[1] + dy * t))


def height(p, src_pts, src_zs, closed):
    for v, z in zip(src_pts, src_zs):
        if z is not None and math.hypot(p[0] - v[0], p[1] - v[1]) <= ON:
            return z
    n = len(src_pts)
    for i in range(n if closed else n - 1):
        a, b = src_pts[i], src_pts[(i + 1) % n]
        za, zb = src_zs[i], src_zs[(i + 1) % n]
        t, d = seg_t(p, a, b)
        if d <= ON and za is not None and zb is not None:
            return za + (zb - za) * t
    return None


def subdivide_case(tool, inputs, most):
    lay = out_layer(tool)["id"]
    added, whole = [], 0
    for e in inputs:
        pieces = []
        if e["kind"] == "polygon":
            split_area(rings_of(e), 0, max(5, most), pieces)
        else:
            pts = [xy(p) for p in e["pts"]]
            split_path(list(zip(pts, pts[1:])), 0, max(5, most), pieces)
        if len(pieces) == 1:
            whole += 1
        placed = []
        for kind, g in pieces:
            if kind == "area":
                outer = settled(g[0], True)
                holes = sorted((settled(h, False) for h in g[1:]), key=lambda r: (r[0][1], r[0][0]))
                placed.append((ring_box(outer), kind, [outer] + holes))
            else:
                pts = [g[0][0]] + [s[1] for s in g]
                placed.append((ring_box(pts), kind, pts))
        placed.sort(key=lambda t: (t[0][0], t[0][1], t[0][2], t[0][3], (t[2][0] if t[1] == 'path' else t[2][0][0])[1],
                                   (t[2][0] if t[1] == 'path' else t[2][0][0])[0]))
        src = [xy(p) for p in e["pts"]]
        zs = e.get("zs")
        for k, (_, kind, g) in enumerate(placed, start=1):
            attrs = dict(e["attrs"])
            attrs["Parça"] = str(k)
            if kind == "area":
                o = {"kind": "polygon", "layerId": lay, "attrs": attrs, "pts": [out(p) for p in g[0]]}
                if len(g) > 1:
                    o["holes"] = [{"pts": [out(p) for p in h]} for h in g[1:]]
            else:
                o = {"kind": "polyline", "layerId": lay, "attrs": attrs, "pts": [out(p) for p in g]}
                if zs:
                    o["zs"] = [height(p, src, zs, False) for p in g]
            added.append(o)
    log = [info(f"{whole} nesne en çok köşeyi aşmadığı için tek parça yazıldı.")] if whole else []
    return ok(tool, added, f"{len(inputs)} nesneden {len(added)} parça yazıldı.", log, len(added))


# ── The cases ────────────────────────────────────────────────────────────


def counts_of(inputs, per):
    return [per(e) for e in inputs]


def build():
    parsel, yol, nokta = objects("parsel"), objects("yol"), objects("nokta")
    g, r, s, a, d = "sample.grid", "sample.regular", "sample.random", "sample.alongLines", "sample.subdivide"

    def grid(kind="rect", dx=100, dy=100, side=100, snap=True, keep="all"):
        return {"shape": kind, "dx": dx, "dy": dy, "side": side, "snap": snap, "keep": keep}

    density = [math.trunc(net_area(rings_of(e)) / 300) for e in parsel]
    for e in parsel:
        f = net_area(rings_of(e)) / 300
        assert 0.02 < f - math.trunc(f) < 0.98, (e["id"], f)
    cases = [
        case("grid-rect", "Ağ oluştur: 50 m'lik dikdörtgenler, parsellerin kutusunu kaplar, aralığın katlarında",
             g, {"input": L("parsel"), **grid(dx=50, dy=50)}, grid_case(g, parsel, "rect", 50, 50, 100, True, "all")),
        case("grid-rect-meeting", "20 m'lik dikdörtgenlerden parsellere değenler: köşesi çizgi üstündeki nokta iki yana",
             g, {"input": L("parsel"), **grid(dx=20, dy=20, keep="meeting")}, grid_case(g, parsel, "rect", 20, 20, 100, True, "meeting")),
        case("grid-rect-free", "Kuyuların kutusundan 30 × 20 m: koordinatlara oturmaz",
             g, {"input": L("nokta"), **grid(dx=30, dy=20, snap=False)}, grid_case(g, nokta, "rect", 30, 20, 100, False, "all")),
        case("grid-hex", "Mahalleye değen 15 m'lik altıgenler",
             g, {"input": L("mahalle"), **grid(kind="hexagon", side=15, keep="meeting")},
             grid_case(g, objects("mahalle"), "hexagon", 100, 100, 15, True, "meeting")),
        case("grid-hex-points", "Kuyuları içine alan 25 m'lik altıgenler",
             g, {"input": L("nokta"), **grid(kind="hexagon", side=25, keep="meeting")}, grid_case(g, nokta, "hexagon", 100, 100, 25, True, "meeting")),
        case("grid-tri", "Parsellerin kutusunu kaplayan 25 m'lik üçgenler",
             g, {"input": L("parsel"), **grid(kind="triangle", side=25)}, grid_case(g, parsel, "triangle", 100, 100, 25, True, "all")),
        case("grid-tri-meeting", "Yollara değen 20 m'lik üçgenler: yaylı yol da",
             g, {"input": L("yol"), **grid(kind="triangle", side=20, keep="meeting")}, grid_case(g, yol, "triangle", 100, 100, 20, True, "meeting")),
        case("grid-lines", "Mahallenin kutusunda 50 × 25 m'lik ağın çizgileri, koordinatlarıyla",
             g, {"input": L("mahalle"), **grid(kind="lines", dx=50, dy=25)}, grid_case(g, objects("mahalle"), "lines", 50, 25, 100, True, "all")),
        case("grid-too-big", "5 cm'lik dikdörtgenlerle mahalle: reddedilir",
             g, {"input": L("mahalle"), **grid(dx=0.05, dy=0.05)}, grid_case(g, objects("mahalle"), "rect", 0.05, 0.05, 100, True, "all")),
        case("grid-too-big-scan", "5 cm'lik altıgenlerle mahalle: taranacak hücre çok, reddedilir",
             g, {"input": L("mahalle"), **grid(kind="hexagon", side=0.05, keep="meeting")},
             grid_case(g, objects("mahalle"), "hexagon", 100, 100, 0.05, True, "meeting")),
        case("regular-snap", "Düzenli noktalar: 10 m'lik katlarda, sınırdaki ve delik kenarındaki noktalar da",
             r, {"input": L("parsel"), "dx": 10, "dy": 10, "snap": True, "shift": 0, "seed": 1}, regular_case(r, parsel, 10, 10, True, 0, 1)),
        case("regular-middles", "15 × 10 m'lik hücrelerin ortaları, alanların kutusundan",
             r, {"input": L("parsel"), "dx": 15, "dy": 10, "snap": False, "shift": 0, "seed": 1}, regular_case(r, parsel, 15, 10, False, 0, 1)),
        case("regular-shift", "5 m'lik katlar, 1,5 m'ye kadar tohumla kaydırılmış",
             r, {"input": SELECTION, "dx": 5, "dy": 5, "snap": True, "shift": 1.5, "seed": 7}, regular_case(r, [by_id(1)], 5, 5, True, 1.5, 7),
             selection=[1]),
        case("regular-sparse", "Alandan büyük aralık: noktası düşmeyen alan söylenir",
             r, {"input": L("parsel"), "dx": 50, "dy": 50, "snap": True, "shift": 0, "seed": 1}, regular_case(r, parsel, 50, 50, True, 0, 1)),
        case("regular-locked", "Kilitli katmanın alanı okunur, noktalar yeni katmana",
             r, {"input": L("kilitli"), "dx": 10, "dy": 10, "snap": True, "shift": 0, "seed": 1}, regular_case(r, objects("kilitli"), 10, 10, True, 0, 1)),
        case("random-parcels", "Rastgele noktalar: parsel başına beş, tohum 1",
             s, {"input": L("parsel"), "count": "5", "min": 0, "tries": 10, "seed": 1}, random_case(s, parsel, [5] * len(parsel), 0, 10, 1)),
        case("random-density", "Sayı ifadeden: 300 m²'ye bir nokta, tohum 2",
             s, {"input": L("parsel"), "count": "$alan / 300", "min": 0, "tries": 10, "seed": 2}, random_case(s, parsel, density, 0, 10, 2)),
        case("random-min", "Mahallede 150 nokta, en az 15 m arayla: sığmayanlar söylenir",
             s, {"input": L("mahalle"), "count": "150", "min": 15, "tries": 10, "seed": 3}, random_case(s, objects("mahalle"), [150], 15, 10, 3)),
        case("random-lines", "Yollarda altışar nokta: uzaklık, parça ve kot",
             s, {"input": L("yol"), "count": "6", "min": 0, "tries": 10, "seed": 4}, random_case(s, yol, [6] * len(yol), 0, 10, 4)),
        case("random-thin", "İki desimilimetrelik şerit kutusuna göre çok ince",
             s, {"input": L("ince"), "count": "3", "min": 0, "tries": 10, "seed": 5}, random_case(s, objects("ince"), [3], 0, 10, 5)),
        case("random-unread", "Sayısı okunamayan nesne atlanır",
             s, {"input": L("parsel"), "count": "Parsel + 'x'", "min": 0, "tries": 10, "seed": 1},
             random_case(s, parsel, [0] * len(parsel), 0, 10, 1, unread=len(parsel))),
        case("along-interval", "Hat boyunca nokta: 25 m'de bir, uçlar da; kotlar ve parçalar",
             a, {"input": L("yol"), "mode": "interval", "step": 25, "start": 0, "end": 0, "parts": 10, "ends": True},
             along_case(a, yol, "interval", 25, 0, 0, 10, True)),
        case("along-offsets", "20 m'de bir, baştan 5, sondan 7,5 m pay, uçsuz",
             a, {"input": L("yol"), "mode": "interval", "step": 20, "start": 5, "end": 7.5, "parts": 10, "ends": False},
             along_case(a, yol, "interval", 20, 5, 7.5, 10, False)),
        case("along-parts", "Parsellerin sınırları beşer eşit parçaya: kapalı yol, delik ikinci parça",
             a, {"input": L("parsel"), "mode": "parts", "step": 10, "start": 0, "end": 0, "parts": 5, "ends": True},
             along_case(a, parsel, "parts", 10, 0, 0, 5, True)),
        case("along-curves", "Daire, yay ve yaylı alanın sınırında 10 m'de bir",
             a, {"input": L("diger"), "mode": "interval", "step": 10, "start": 0, "end": 0, "parts": 10, "ends": True},
             along_case(a, objects("diger"), "interval", 10, 0, 0, 10, True)),
        case("subdivide-star", "Alt bölümlere ayır: 40 köşeli yıldız, en çok 8 köşe",
             d, {"input": SELECTION, "max": 8}, subdivide_case(d, [by_id(61)], 8), selection=[61]),
        case("subdivide-hole", "60 köşeli delikli avlu, en çok 12 köşe: kesimler deliğin köşelerinden",
             d, {"input": SELECTION, "max": 12}, subdivide_case(d, [by_id(62)], 12), selection=[62]),
        case("subdivide-line", "31 köşeli zikzak, en çok 6 köşe: kesimin kotları kenarlarından",
             d, {"input": SELECTION, "max": 6}, subdivide_case(d, [by_id(63)], 6), selection=[63]),
        case("subdivide-small", "En çok köşeyi aşmayan parseller tek parça",
             d, {"input": L("parsel"), "max": 256}, subdivide_case(d, parsel, 256)),
    ]
    return {"format": "kentos.processing-cases", "version": 1, "tolerance": TOL, "toleranceFields": ["z", "zs"],
            "documents": {}, "cases": cases}


def main():
    doc = json.dumps(DOCUMENT, ensure_ascii=False, indent=1) + "\n"
    proc = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    outs = [(PROC_DIR / "sampling.json", proc), (PROC_DIR / "sampling.kcad", doc)]
    if "--check" in sys.argv[1:]:
        bad = [p for p, t in outs if not p.exists() or p.read_text() != t]
        for p in bad:
            print(f"{p.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
        if bad:
            return 1
        print("örneklemenin durumları tutarlı: fixtures/processing/v1/sampling*")
        return 0
    for p, t in outs:
        p.write_text(t)
        print(f"yazıldı: {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
