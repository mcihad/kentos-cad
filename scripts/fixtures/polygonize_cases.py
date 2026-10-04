#!/usr/bin/env python3
"""Independent reference of Toplu alan (docs/adr/0151).

Writes fixtures/polygonize/v1/cases.json from the rules alone, with Python's
standard library and no KentOS code. The geometry core
(`ops::polygonize::polygonize`, crates/shared/geometry-core/tests/all/polygonize.rs)
and the web through its WASM must give the same, points within 1e-8 m.

Input: the line work in the drawing's order (lines, polylines, areas with
holes; straight edges only), labels ({at, value}) and Adalar. Arcs,
circles, ellipses and curves are the core's own tests' business: its face
walk is the one Tarama and İçine tıklayarak alan already use.

Worked out on exact rationals (every input float is a fraction exactly):

1. The arrangement: every segment is cut where another meets it; pieces
   lying on top of each other are one.
2. Walks: every piece both ways, each walk keeping its face on the left (at
   a vertex the walk turns into the piece next clockwise from the way back).
3. A walk loses its spikes (a piece and its way back, also round the walk's
   seam), is split where it comes back to a vertex (each loop on its own),
   and where it runs straight on through a vertex that is no input vertex,
   the two pieces are one edge. A loop of under three corners, or of area
   at most its perimeter × 1e-6 (dust), is dropped.
4. Loops running counter-clockwise are regions; clockwise ones are the
   outlines of groups. A group is a hole of the smallest region whose outer
   ring holds a point a hair to the left of its outline (outside the group);
   the outline of the whole drawing's line work is no one's hole. With
   Adalar, each region has its groups as holes; without, none.
5. A label within 1e-6 m of any region's or group's edge is on a boundary;
   any other goes to the smallest region whose outer ring holds it.
6. A region is an input area's when their outer rings, and with Adalar
   their holes, have the same corners in the same order (whichever corner
   each starts at, either way round), corners being the vertices where a
   ring turns. The first such area in the drawing's order is named.
7. A free end: an end of a line or an open polyline that lies within
   1e-6 m of no other object's edge and of no other vertex of its own path.

Output, so that it can be compared whatever order the core finds things in:
regions sorted by area, then by their lowest corner; each ring starting at
its lowest vertex (x, then y) and running as the core runs it (regions
counter-clockwise, holes clockwise); holes by their first vertex; free ends
sorted. Labels and areas are named by their places in the input.

Hand cases first (each at the origin and at TM coordinates), then random
networks about (487000, 4420000).
"""

import argparse
import json
import math
import random
import sys
from fractions import Fraction as F
from functools import cmp_to_key
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "polygonize" / "v1" / "cases.json"
NEAR = 1e-6
DUST = 1e-6
E0, N0 = 487000.0, 4420000.0


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


def perimeter(ring):
    return sum(math.dist(fl(ring[i]), fl(ring[(i + 1) % len(ring)])) for i in range(len(ring)))


def fl(p):
    return (float(p[0]), float(p[1]))


def dist_seg(p, a, b):
    """Distance from p to segment ab, in floats (the 1 µm rules)."""
    (px, py), (ax, ay), (bx, by) = fl(p), fl(a), fl(b)
    dx, dy = bx - ax, by - ay
    ll = dx * dx + dy * dy
    t = 0.0 if ll == 0 else max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / ll))
    return math.hypot(px - (ax + dx * t), py - (ay + dy * t))


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


def half(d):
    return 0 if (d[1] > 0 or (d[1] == 0 and d[0] > 0)) else 1


def turn_order(a, b):
    """Counter-clockwise from east: by half plane, then by turn."""
    ha, hb = half(a), half(b)
    if ha != hb:
        return ha - hb
    c = cross(a, b)
    return -1 if c > 0 else 1 if c < 0 else 0


# ── Regions ─────────────────────────────────────────────────────────────────


def loops(segments, inputs):
    """The cleaned loops of the arrangement of `segments` (exact points)."""
    pieces = set()
    for i, (a, b) in enumerate(segments):
        d = sub(b, a)
        dd = dot(d, d)
        if dd == 0:
            continue
        cuts = {a, b}
        for j, (c, e) in enumerate(segments):
            if i != j:
                cuts.update(meet(a, b, c, e))
        order = sorted(cuts, key=lambda p: dot(sub(p, a), d) / dd)
        for p, q in zip(order, order[1:]):
            if p != q:
                pieces.add((min(p, q), max(p, q)))
    around = {}
    for p, q in pieces:
        around.setdefault(p, []).append(q)
        around.setdefault(q, []).append(p)
    for v, ws in around.items():
        ws.sort(key=cmp_to_key(lambda w1, w2, v=v: turn_order(sub(w1, v), sub(w2, v))))
    seen = set()
    out = []
    for p, q in sorted(pieces):
        for start in ((p, q), (q, p)):
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
            for loop in split(spikeless(walk)):
                pts = straightened([h[0] for h in loop], inputs)
                if len(pts) < 3:
                    continue
                if abs(float(area2(pts)) / 2) <= perimeter(pts) * DUST:
                    continue
                out.append(pts)
    return out


def spikeless(walk):
    st = []
    for h in walk:
        if st and st[-1] == (h[1], h[0]):
            st.pop()
        else:
            st.append(h)
    lo, hi = 0, len(st)
    while hi - lo >= 2 and st[lo] == (st[hi - 1][1], st[hi - 1][0]):
        lo += 1
        hi -= 1
    return st[lo:hi]


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


def probe(ring):
    """A point a hair to the left of a ring's first edge's middle: outside a group's outline, in the
    region around it (any edge does: a group's outline borders one region)."""
    a, b = fl(ring[0]), fl(ring[1])
    m = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
    ln = math.hypot(b[0] - a[0], b[1] - a[1])
    eps = min(ln * 1e-4, 1e-4)
    return (F(m[0] - (b[1] - a[1]) / ln * eps), F(m[1] + (b[0] - a[0]) / ln * eps))


def corners(ring):
    """The vertices where a ring turns (a repeated vertex is none)."""
    out = []
    n = len(ring)
    for i in range(n):
        a, v, b = ring[i - 1], ring[i], ring[(i + 1) % n]
        u, w = sub(v, a), sub(b, v)
        if dot(u, u) == 0 or dot(w, w) == 0:
            continue
        if cross(u, w) == 0 and dot(u, w) > 0:
            continue
        out.append(v)
    return out


def same_ring(a, b):
    n = len(a)
    if n != len(b) or n == 0:
        return False
    for k in range(n):
        if all(a[i] == b[(i + k) % n] for i in range(n)):
            return True
        if all(a[i] == b[(k - i) % n] for i in range(n)):
            return True
    return False


def same_holes(a, b):
    if len(a) != len(b):
        return False
    used = [False] * len(b)
    for h in a:
        j = next((j for j in range(len(b)) if not used[j] and same_ring(h, b[j])), None)
        if j is None:
            return False
        used[j] = True
    return True


# ── The case ────────────────────────────────────────────────────────────────


def P(x, y):
    return {"x": x, "y": y}


def segments_of(lines):
    """Every object's segments (exact), its vertices and, for an open path, its ends."""
    out = []
    for e in lines:
        q = lambda p: (F(p["x"]), F(p["y"]))
        if e["kind"] == "line":
            pts = [q(e["a"]), q(e["b"])]
            out.append({"segs": [(pts[0], pts[1])], "verts": pts, "ends": [pts[0], pts[1]], "rings": None})
        elif e["kind"] == "polyline":
            pts = [q(p) for p in e["pts"]]
            out.append({"segs": list(zip(pts, pts[1:])), "verts": pts, "ends": [pts[0], pts[-1]], "rings": None})
        elif e["kind"] == "polygon":
            rings = [[q(p) for p in e["pts"]]] + [[q(p) for p in h["pts"]] for h in e.get("holes", [])]
            segs = [(r[i], r[(i + 1) % len(r)]) for r in rings for i in range(len(r))]
            out.append({"segs": segs, "verts": [v for r in rings for v in r], "ends": [], "rings": rings})
        else:
            raise ValueError(e["kind"])
    return out


def solve(lines, labels, islands):
    objs = segments_of(lines)
    segs = [s for o in objs for s in o["segs"]]
    inputs = {v for o in objs for v in o["verts"]}
    all_loops = loops(segs, inputs)
    regions = [r for r in all_loops if area2(r) > 0]
    groups = [r for r in all_loops if area2(r) < 0]
    regions.sort(key=lambda r: area2(r))
    hosts = []
    for g in groups:
        p = probe(g)
        hosts.append(next((k for k, r in enumerate(regions) if inside(p, r)), None))
    faces = []
    for k, r in enumerate(regions):
        holes = [g for g, h in zip(groups, hosts) if h == k] if islands else []
        faces.append({"outer": r, "holes": holes, "labels": [], "existing": None})
    on_boundary = []
    rings_all = regions + groups
    for i, l in enumerate(labels):
        p = (F(l["at"]["x"]), F(l["at"]["y"]))
        if any(dist_seg(p, r[j], r[(j + 1) % len(r)]) <= NEAR for r in rings_all for j in range(len(r))):
            on_boundary.append(i)
            continue
        k = next((k for k, r in enumerate(regions) if inside(p, r)), None)
        if k is not None:
            faces[k]["labels"].append(i)
    for f in faces:
        oc = corners(f["outer"])
        hc = [corners(h) for h in f["holes"]]
        for i, o in enumerate(objs):
            if o["rings"] is None:
                continue
            if same_ring(oc, corners(o["rings"][0])) and same_holes(hc, [corners(h) for h in o["rings"][1:]]):
                f["existing"] = i
                break
    free = []
    for i, o in enumerate(objs):
        for k, end in enumerate(o["ends"]):
            own = len(o["verts"]) - 1 if k else 0
            if any(j != own and math.dist(fl(v), fl(end)) <= NEAR for j, v in enumerate(o["verts"])):
                continue
            if any(dist_seg(end, a, b) <= NEAR for j, other in enumerate(objs) if j != i for a, b in other["segs"]):
                continue
            free.append(fl(end))
    return {
        "regions": sorted((written(f) for f in faces), key=region_key),
        "onBoundary": on_boundary,
        "freeEnds": [list(p) for p in sorted(free)],
    }


def lowest_first(ring):
    k = min(range(len(ring)), key=lambda i: ring[i])
    return ring[k:] + ring[:k]


def written(f):
    return {
        "outer": [list(fl(p)) for p in lowest_first(f["outer"])],
        "holes": sorted(([list(fl(p)) for p in lowest_first(h)] for h in f["holes"]), key=lambda h: h[0]),
        "labels": f["labels"],
        "existing": f["existing"],
    }


def region_key(r):
    """Regions by their lowest vertex, then the next: the order the tests sort the core's into."""
    return (r["outer"][0], r["outer"][1])


# ── Cases ───────────────────────────────────────────────────────────────────


def L(a, b):
    return {"kind": "line", "a": P(*a), "b": P(*b)}


def PL(*pts):
    return {"kind": "polyline", "pts": [P(*p) for p in pts]}


def AREA(pts, holes=()):
    out = {"kind": "polygon", "pts": [P(*p) for p in pts]}
    if holes:
        out["holes"] = [{"pts": [P(*p) for p in h]} for h in holes]
    return out


def LB(x, y, value):
    return {"at": P(x, y), "value": value}


def hand():
    """(name, lines, labels, islands), at the origin."""
    cases = []
    # A block of four parcels drawn as lines: the outer frame, a cross through it; each parcel's number inside.
    block = [PL((0, 0), (40, 0), (40, 30), (0, 30), (0, 0)), L((20, 0), (20, 30)), L((0, 15), (40, 15))]
    nums = [LB(10, 7, "101/1"), LB(30, 7, "101/2"), LB(10, 22, "101/3"), LB(30, 22, "101/4")]
    cases.append(("dört parsellik ada, numaraları içinde", block, nums, True))
    # A divider that stops on the frame (a T junction: its end is an input vertex, kept on the frame's ring).
    cases.append(("T kavşağı: bölücünün ucu çerçevede, iki bölge", [PL((0, 0), (30, 0), (30, 20), (0, 20), (0, 0)), L((12, 0), (12, 20))], [LB(5, 10, "A"), LB(20, 10, "B")], True))
    # Crossing lines that run on past the frame: the stubs outside are dangles, the crossings no corners where straight.
    cases.append(("taşan çizgiler: dışarıdaki parçalar sarkan, kesişmeler düz geçilir", [L((-5, 0), (35, 0)), L((-5, 20), (35, 20)), L((0, -5), (0, 25)), L((30, -5), (30, 25))], [LB(15, 10, "tek")], True))
    # A dangle inside a parcel and a free end; a label on the dangle still goes to the parcel.
    cases.append(("sarkan çizgi ve boşta uç", [PL((0, 0), (20, 0), (20, 20), (0, 20), (0, 0)), L((5, 5), (12, 9))], [LB(8.5, 7, "üstünde"), LB(15, 15, "P")], True))
    # A bridge from a building to the parcel's frame: the building is a region; the bridge bounds nothing.
    cases.append(("köprü: yapıdan çerçeveye çizgi", [PL((0, 0), (30, 0), (30, 30), (0, 30), (0, 0)), PL((10, 10), (20, 10), (20, 20), (10, 20), (10, 10)), L((20, 15), (30, 15))], [LB(5, 5, "parsel"), LB(15, 15, "yapı")], True))
    # A building alone inside a parcel: a hole with Adalar, none without; the building's own region either way.
    island = [PL((0, 0), (30, 0), (30, 30), (0, 30), (0, 0)), PL((10, 10), (20, 10), (20, 20), (10, 20), (10, 10))]
    cases.append(("ada açık: yapı parselin deliği", island, [LB(5, 5, "parsel"), LB(15, 15, "yapı")], True))
    cases.append(("ada kapalı: parsel yapıyı da kapsar", island, [LB(5, 5, "parsel"), LB(15, 15, "yapı")], False))
    # Labels on a boundary (on the frame, on the divider), outside every region, two in one parcel, none in another.
    cases.append(("sınırda, dışarıda, iki etiketli ve etiketsiz", block, [LB(20, 7, "sınır"), LB(0, 10, "çerçeve"), LB(50, 50, "dışarı"), LB(10, 7, "1"), LB(12, 9, "2"), LB(30, 22, "4"), LB(20, 15, "köşe")], True))
    # An existing area among the lines: its region is named; the other region is new.
    cases.append(("var olan alan", [AREA([(0, 0), (20, 0), (20, 20), (0, 20)]), PL((20, 0), (40, 0), (40, 20), (20, 20))], [LB(10, 10, "eski"), LB(30, 10, "yeni")], True))
    # The existing area drawn the other way round and from another corner, with an extra vertex on an edge.
    cases.append(("var olan alan ters yönde, fazladan köşeyle", [AREA([(20, 20), (10, 20), (0, 20), (0, 0), (20, 0)]), PL((20, 0), (40, 0), (40, 20), (20, 20))], [], True))
    # An area with a hole: the hole's region is new; with Adalar the area's region is the area.
    holed = [AREA([(0, 0), (30, 0), (30, 30), (0, 30)], [[(10, 10), (10, 20), (20, 20), (20, 10)]])]
    cases.append(("delikli alan, ada açık: alan var olan", holed, [LB(5, 5, "dış"), LB(15, 15, "iç")], True))
    cases.append(("delikli alan, ada kapalı: dış bölge alanla aynı değil", holed, [LB(5, 5, "dış"), LB(15, 15, "iç")], False))
    # Two blocks side by side not touching: four regions, no holes; a gap closed by nothing leaves free ends.
    cases.append(("açık kalan çerçeve: bölge yok, iki uç boşta", [PL((0, 0), (20, 0), (20, 20), (0, 20), (0, 1))], [LB(10, 10, "açık")], True))
    # A polyline that ends on itself (a 6): its end touches its own vertex, not free; the loop is a region.
    cases.append(("kendi üstünde biten çoklu çizgi", [PL((0, -10), (0, 0), (10, 0), (10, 10), (0, 10), (0, 0))], [LB(5, 5, "döngü")], True))
    # Slanted parcels with crossings that turn (corners at crossings that are no input vertex).
    cases.append(("eğik çizgiler, köşe olan kesişmeler", [L((0, 0), (30, 12)), L((0, 12), (30, 0)), L((0, -2), (0, 14)), L((30, -2), (30, 14))], [LB(5, 6, "sol"), LB(25, 6, "sağ"), LB(15, 9, "üst"), LB(15, 3, "alt")], True))
    # Nested: a parcel, a building, a courtyard in the building.
    nest = [PL((0, 0), (40, 0), (40, 40), (0, 40), (0, 0)), PL((10, 10), (30, 10), (30, 30), (10, 30), (10, 10)), PL((15, 15), (25, 15), (25, 25), (15, 25), (15, 15))]
    cases.append(("iç içe üç bölge, ada açık", nest, [LB(5, 5, "parsel"), LB(12, 12, "yapı"), LB(20, 20, "avlu")], True))
    cases.append(("iç içe üç bölge, ada kapalı", nest, [LB(5, 5, "parsel"), LB(12, 12, "yapı"), LB(20, 20, "avlu")], False))
    return cases


def moved(case, de, dn):
    name, lines, labels, islands = case

    def mv(v):
        if isinstance(v, dict) and set(v) == {"x", "y"}:
            return P(v["x"] + de, v["y"] + dn)
        if isinstance(v, dict):
            return {k: mv(x) for k, x in v.items()}
        if isinstance(v, list):
            return [mv(x) for x in v]
        return v

    return (name + ", TM koordinatlarında", mv(lines), mv(labels), islands)


def network(seed):
    """A jittered street block: a grid of lines, some cut short or missing, dangles, a few buildings
    and existing areas, labels inside, on the lines and outside."""
    rnd = random.Random(seed)
    cols, rows = rnd.randint(2, 4), rnd.randint(2, 3)
    w, h = rnd.uniform(15, 30), rnd.uniform(15, 25)
    x0, y0 = E0 + rnd.uniform(-100, 100), N0 + rnd.uniform(-100, 100)
    r3 = lambda v: round(v, 3)
    xs = [r3(x0 + i * w + (rnd.uniform(-2, 2) if 0 < i < cols else 0)) for i in range(cols + 1)]
    ys = [r3(y0 + j * h + (rnd.uniform(-2, 2) if 0 < j < rows else 0)) for j in range(rows + 1)]
    lines = []
    for x in xs:
        lines.append(L((x, ys[0] - r3(rnd.uniform(0, 3))), (x, ys[-1] + r3(rnd.uniform(0, 3)))))
    for y in ys:
        lines.append(L((xs[0] - r3(rnd.uniform(0, 3)), y), (xs[-1] + r3(rnd.uniform(0, 3)), y)))
    # One inner line missing (two cells become one), sometimes.
    if rnd.random() < 0.5 and cols > 1:
        lines.pop(rnd.randint(1, cols - 1))
    # Dangles.
    for _ in range(rnd.randint(0, 3)):
        x, y = r3(rnd.uniform(xs[0], xs[-1])), r3(rnd.uniform(ys[0], ys[-1]))
        lines.append(L((x, y), (r3(x + rnd.uniform(-4, 4)), r3(y + rnd.uniform(-4, 4)))))
    # Buildings: closed polylines well inside a cell.
    for _ in range(rnd.randint(0, 2)):
        i, j = rnd.randrange(cols), rnd.randrange(rows)
        cx, cy = (xs[i] + xs[i + 1]) / 2, (ys[j] + ys[j + 1]) / 2
        s = rnd.uniform(1.5, 4)
        lines.append(PL((r3(cx - s), r3(cy - s)), (r3(cx + s), r3(cy - s)), (r3(cx + s), r3(cy + s)), (r3(cx - s), r3(cy + s)), (r3(cx - s), r3(cy - s))))
    islands = rnd.random() < 0.7
    labels = []
    for k in range(rnd.randint(3, 10)):
        x, y = r3(rnd.uniform(xs[0] - 5, xs[-1] + 5)), r3(rnd.uniform(ys[0] - 5, ys[-1] + 5))
        labels.append(LB(x, y, f"{100 + k}"))
    # A label exactly on a grid line.
    labels.append(LB(xs[0], r3((ys[0] + ys[1]) / 2), "sınırda"))
    lines_out = lines
    # Existing areas: a first pass's regions given back as areas, some of them: those whose corners are all
    # exact (the grid's own crossings and the input's vertices), so that the area adds no new point.
    exact = {(x, y) for x in xs for y in ys}
    for e in lines:
        for k in ("a", "b"):
            if k in e:
                exact.add((e[k]["x"], e[k]["y"]))
        for p in e.get("pts", []):
            exact.add((p["x"], p["y"]))
    first = solve(lines_out, [], islands)
    for reg in first["regions"]:
        rings = [reg["outer"], *reg["holes"]]
        if not all(tuple(p) in exact for r in rings for p in r):
            continue
        if rnd.random() < 0.3:
            pts = [tuple(p) for p in reg["outer"]]
            if rnd.random() < 0.5:
                pts = list(reversed(pts))
            holes = [[tuple(p) for p in hole] for hole in reg["holes"]]
            lines_out.append(AREA(pts, holes))
    rnd.shuffle(lines_out)
    return (f"rastgele ağ {seed}", lines_out, labels, islands)


def build():
    cases = []
    for c in hand():
        cases.append(c)
        cases.append(moved(c, E0, N0))
    for seed in range(1, 25):
        cases.append(network(seed))
    out = []
    for name, lines, labels, islands in cases:
        out.append({"name": name, "lines": lines, "labels": labels, "islands": islands, "expected": solve(lines, labels, islands)})
    return {"format": "kentos.polygonize-fixtures", "version": 1, "cases": out}


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--check", action="store_true", help="compare with the written file instead of writing it")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if OUT.read_text() != text:
            print(f"{OUT} güncel değil: python3 scripts/fixtures/polygonize_cases.py", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT}: güncel")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    print(f"{OUT}: {len(json.loads(text)['cases'])} durum")


if __name__ == "__main__":
    main()
