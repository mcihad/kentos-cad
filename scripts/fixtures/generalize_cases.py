#!/usr/bin/env python3
"""Independent reference of the Genelleştirme tools (docs/adr/0221), written from the ADR alone, no KentOS code.

Writes fixtures/processing/v1/generalize.kcad (the drawing) and fixtures/processing/v1/generalize.json (the tools run
on it, in the processing cases' format, fixtures/processing/README.md).

How it computes, independently of the core's ways:

1. Yumuşat, Dik açılandır, Bina sadeleştir and the coverage's edges by the ADR's rules (§2–§4, §8) in float64, the same
   operations in the same order; a vertex in a corner's triangle decided on fractions.
2. Küçük parçaları ayıkla's areas by the shoelace formula (straight rings).
3. Alanları kümele's results: their areas worked out by hand for each case (rectangles, quarter-disc fillets, a circular
   segment), checked against GEOS's closing (shapely, 256 segments a quarter); which objects a result holds from
   GEOS's interior points.
4. Küçükleri komşuya kat's shared boundaries and unions by GEOS (straight edges: exact lengths).
"""

import heapq
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

import shapely
from shapely.geometry import Polygon
from shapely.ops import unary_union

sys.path.insert(0, str(Path(__file__).resolve().parent))
from layer_ops_cases import new_id  # noqa: E402
from numeric_display import shown  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
PROC_DIR = ROOT / "fixtures" / "processing" / "v1"
E, N = 487000, 4420000
TOL = 1e-8
NEAR = 1e-3  # the shared boundary's tolerance (ADR 0215 §2.4)
FRAC_1_SQRT_2 = 0.7071067811865476


def P(x, y):
    return {"x": E + x, "y": N + y}


def xy(p):
    return (p["x"], p["y"])


def out(p):
    return {"x": float(p[0]), "y": float(p[1])}


# ── Tracks: a path's vertices, its edges' bulges, its elevations ──────────


class Track:
    def __init__(self, pts, bulges, zs, closed):
        self.pts = pts
        self.bulges = bulges
        self.zs = zs
        self.closed = closed

    def copy(self):
        return Track(list(self.pts), list(self.bulges), list(self.zs), self.closed)

    def edges(self):
        n = len(self.pts)
        if n < 2:
            return 0
        return n if self.closed else n - 1

    def on_arc(self, i):
        n = len(self.pts)
        if i > 0:
            into = self.bulges[i - 1] != 0.0
        else:
            into = self.closed and n > 1 and self.bulges[n - 1] != 0.0
        outgoing = (self.closed or i + 1 < n) and self.bulges[i] != 0.0
        return into or outgoing

    def push(self, p, bulge, z):
        if self.pts and self.pts[-1] == p:
            self.bulges[-1] = bulge
            return
        self.pts.append(p)
        self.bulges.append(bulge)
        self.zs.append(z)

    def close_up(self):
        if self.closed and len(self.pts) > 1 and self.pts[-1] == self.pts[0]:
            self.pts.pop()
            self.bulges.pop()
            self.zs.pop()


def track(src, closed):
    pts = [xy(p) for p in src["pts"]]
    n = len(pts)
    b = [float(x) for x in (src.get("bulges") or [])]
    b = (b + [0.0] * n)[:n]
    if not closed and n:
        b[n - 1] = 0.0
    zs = src.get("zs") or []
    z = [zs[i] if i < len(zs) else None for i in range(n)]
    return Track(pts, b, z, closed)


def tracks_of(e):
    if e["kind"] == "polyline":
        return [track(e, False)] + [track(q, False) for q in e.get("parts") or []]
    if e["kind"] == "polygon":
        out_ = [track(e, True)] + [track(h, True) for h in e.get("holes") or []]
        for q in e.get("parts") or []:
            out_.append(track(q, True))
            out_ += [track(h, True) for h in q.get("holes") or []]
        return out_
    return []


def put(dst, t):
    dst["pts"] = [out(p) for p in t.pts]
    if any(b != 0.0 for b in t.bulges):
        dst["bulges"] = [float(b) for b in t.bulges]
    if any(z is not None for z in t.zs):
        dst["zs"] = list(t.zs)


def shape_like(e, tracks, layer, attrs):
    """The object `e` with its paths replaced (the same parts and holes), as written."""
    o = {"kind": e["kind"], "layerId": layer, "attrs": attrs}
    it = iter(tracks)
    put(o, next(it))
    if e["kind"] == "polygon":
        if e.get("holes"):
            o["holes"] = []
            for _ in e["holes"]:
                h = {}
                put(h, next(it))
                o["holes"].append(h)
        if e.get("parts"):
            o["parts"] = []
            for q in e["parts"]:
                part = {}
                put(part, next(it))
                if q.get("holes"):
                    part["holes"] = []
                    for _ in q["holes"]:
                        h = {}
                        put(h, next(it))
                        part["holes"].append(h)
                o["parts"].append(part)
    elif e.get("parts"):
        o["parts"] = []
        for _ in e["parts"]:
            part = {}
            put(part, next(it))
            o["parts"].append(part)
    return o


def polygon_of(parts, layer, attrs):
    """Parts (an outer ring and its holes) as a polygon, the first part's ring first."""
    (outer, holes), rest = parts[0], parts[1:]
    o = {"kind": "polygon", "layerId": layer, "attrs": attrs}
    put(o, outer)
    if holes:
        o["holes"] = []
        for h in holes:
            d = {}
            put(d, h)
            o["holes"].append(d)
    if rest:
        o["parts"] = []
        for (ring, hs) in rest:
            d = {}
            put(d, ring)
            if hs:
                d["holes"] = []
                for h in hs:
                    dh = {}
                    put(dh, h)
                    d["holes"].append(dh)
            o["parts"].append(d)
    return o


def parts_of(e):
    """An area object's tracks by part: (outer ring, its holes)."""
    out_ = [(track(e, True), [track(h, True) for h in e.get("holes") or []])]
    for q in e.get("parts") or []:
        out_.append((track(q, True), [track(h, True) for h in q.get("holes") or []]))
    return out_


def vertices_of(e):
    n = len(e["pts"]) + sum(len(h["pts"]) for h in e.get("holes") or [])
    for q in e.get("parts") or []:
        n += len(q["pts"]) + sum(len(h["pts"]) for h in q.get("holes") or [])
    return n


# ── Yumuşat (§2) ───────────────────────────────────────────────────────────


def turn(t, i):
    n = len(t.pts)
    p, c, q = t.pts[(i - 1) % n], t.pts[i], t.pts[(i + 1) % n]
    ux, uy, wx, wy = c[0] - p[0], c[1] - p[1], q[0] - c[0], q[1] - c[1]
    if (ux == 0.0 and uy == 0.0) or (wx == 0.0 and wy == 0.0):
        return 0.0
    return abs(math.atan2(ux * wy - uy * wx, ux * wx + uy * wy))


def along(za, zb, f):
    if za is None or zb is None:
        return None
    return za + f * (zb - za)


def smooth_step(t, o, max_turn):
    n = len(t.pts)
    fix = []
    for i in range(n):
        if not t.closed and (i == 0 or i + 1 == n):
            fix.append(True)
        else:
            fix.append(t.on_arc(i) or turn(t, i) > max_turn)
    back = 1.0 - o
    r = Track([], [], [], t.closed)
    for i in range(t.edges()):
        j = (i + 1) % n
        a, c = t.pts[i], t.pts[j]
        if t.bulges[i] != 0.0:
            r.push(a, t.bulges[i], t.zs[i])
            continue
        if fix[i]:
            r.push(a, 0.0, t.zs[i])
        else:
            r.push((a[0] + o * (c[0] - a[0]), a[1] + o * (c[1] - a[1])), 0.0, along(t.zs[i], t.zs[j], o))
        if fix[j]:
            r.push(c, 0.0, t.zs[j])
        else:
            r.push((a[0] + back * (c[0] - a[0]), a[1] + back * (c[1] - a[1])), 0.0, along(t.zs[i], t.zs[j], back))
    if not t.closed and n > 0:
        r.push(t.pts[n - 1], 0.0, t.zs[n - 1])
    r.close_up()
    return r


def smooth_track(t, iterations, offset, max_turn):
    cur = t.copy()
    for _ in range(iterations):
        if len(cur.pts) < 3:
            break
        cur = smooth_step(cur, offset, max_turn)
    return cur


# ── The coverage's edges (§8) ──────────────────────────────────────────────


def key(p):
    return (p[0] + 0.0, p[1] + 0.0)


def turned(b):
    return 0.0 - b


def segment(a, b, bulge):
    ka, kb = key(a), key(b)
    if ka <= kb:
        return ((ka, kb, bulge + 0.0), True)
    return ((kb, ka, turned(bulge)), False)


class Coverage:
    def __init__(self, edges, uses, rings):
        self.edges = edges
        self.uses = uses
        self.rings = rings


def build_coverage(rings):
    segs = []
    for r in rings:
        n = len(r.pts)
        segs.append([segment(r.pts[i], r.pts[(i + 1) % n], r.bulges[i]) for i in range(n)])
    sets, touching = {}, {}
    for ri, lst in enumerate(segs):
        for (s, _) in lst:
            st = sets.setdefault(s, [])
            if not st or st[-1] != ri:
                st.append(ri)
            for end in (s[0], s[1]):
                at = touching.setdefault(end, [])
                if s not in at:
                    at.append(s)
    edges, uses, seats, out_rings = [], [], {}, []
    for ri, r in enumerate(rings):
        n = len(r.pts)
        lst = segs[ri]

        def node(i):
            degree = len(touching.get(key(r.pts[i]), []))
            return degree != 2 or sets[lst[(i - 1) % n][0]] != sets[lst[i][0]]

        nodes = [i for i in range(n) if node(i)]
        refs = []
        if not nodes:
            s, al = lst[0]
            seat = seats.get(s)
            if seat is not None and edges[seat[0]].closed:
                uses[seat[0]] += 1
                refs.append((seat[0], al == seat[2]))
            else:
                eid = len(edges)
                for i, (s2, al2) in enumerate(lst):
                    seats[s2] = (eid, i, al2)
                edges.append(r.copy())
                uses.append(1)
                refs.append((eid, True))
            out_rings.append(refs)
            continue
        for k, start in enumerate(nodes):
            end = nodes[(k + 1) % len(nodes)]
            count = end - start if end > start else end + n - start
            s, al = lst[start]
            seat = seats.get(s)
            found = None
            if seat is not None:
                e = edges[seat[0]]
                ok_ = (not e.closed) and e.edges() == count and (seat[1] == 0 if al == seat[2] else seat[1] + 1 == count)
                if ok_:
                    found = seat
            if found is not None:
                uses[found[0]] += 1
                refs.append((found[0], al == found[2]))
            else:
                eid = len(edges)
                t = Track([], [], [], False)
                for j in range(count + 1):
                    i = (start + j) % n
                    t.pts.append(r.pts[i])
                    t.bulges.append(r.bulges[i] if j < count else 0.0)
                    t.zs.append(r.zs[i])
                    if j < count:
                        s2, al2 = lst[i]
                        if s2 not in seats:
                            seats[s2] = (eid, j, al2)
                edges.append(t)
                uses.append(1)
                refs.append((eid, True))
        out_rings.append(refs)
    return Coverage(edges, uses, out_rings)


def reversed_track(t):
    n = len(t.pts)
    r = Track([], [], [], t.closed)
    if n == 0:
        return r
    if t.closed:
        for j in range(n):
            i = (n - j) % n
            frm = (n - j - 1) % n
            r.pts.append(t.pts[i])
            r.zs.append(t.zs[i])
            r.bulges.append(turned(t.bulges[frm]))
    else:
        for j in range(n):
            i = n - 1 - j
            r.pts.append(t.pts[i])
            r.zs.append(t.zs[i])
            r.bulges.append(turned(t.bulges[i - 1]) if i > 0 else 0.0)
    return r


def coverage_ring(c, edges, k):
    r = Track([], [], [], True)
    for (e, fwd) in c.rings[k]:
        t = edges[e].copy() if fwd else reversed_track(edges[e])
        if t.closed:
            return t
        for i in range(len(t.pts) - 1):
            r.pts.append(t.pts[i])
            r.bulges.append(t.bulges[i])
            r.zs.append(t.zs[i])
    return r


def orient(a, b, c):
    """The sign of (b − a) × (c − a), exactly."""
    ax, ay, bx, by, cx, cy = (F(v) for v in (*a, *b, *c))
    d = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax)
    return (d > 0) - (d < 0)


def in_triangle(a, b, c, v):
    o1, o2, o3 = orient(a, b, v), orient(b, c, v), orient(c, a, v)
    return (o1 >= 0 and o2 >= 0 and o3 >= 0) or (o1 <= 0 and o2 <= 0 and o3 <= 0)


def corner_area(p, k, s):
    ax, ay, bx, by = s[0] - p[0], s[1] - p[1], k[0] - p[0], k[1] - p[1]
    return abs(ax * by - ay * bx) / 2.0


def simplify_coverage(c, tol, keep_boundary):
    limit = tol * tol
    first, points = [], []
    for e in c.edges:
        first.append(len(points))
        points.extend(e.pts)
    gone = [False] * len(points)
    twos = [(e.pts[0], e.pts[1]) for e in c.edges if not e.closed and len(e.pts) == 2 and e.bulges[0] == 0.0]

    def in_corner(a, b, cc):
        lo_x, hi_x = min(a[0], b[0], cc[0]), max(a[0], b[0], cc[0])
        lo_y, hi_y = min(a[1], b[1], cc[1]), max(a[1], b[1], cc[1])
        for g, v in enumerate(points):
            if gone[g] or v == a or v == b or v == cc:
                continue
            if not (lo_x <= v[0] <= hi_x and lo_y <= v[1] <= hi_y):
                continue
            if in_triangle(a, b, cc, v):
                return True
        return False

    for k, e in enumerate(c.edges):
        if keep_boundary and c.uses[k] < 2:
            continue
        n = len(e.pts)
        base = first[k]
        fixed = [(e.closed and i == 0) or (not e.closed and (i == 0 or i + 1 == n)) or e.on_arc(i) for i in range(n)]
        prev = [(i - 1) % n for i in range(n)]
        nxt = [(i + 1) % n for i in range(n)]
        left = n
        loop_edge = (not e.closed) and n > 1 and e.pts[0] == e.pts[n - 1]
        least = 3 if e.closed else (4 if loop_edge else 2)

        def area_of(i):
            return corner_area(e.pts[prev[i]], e.pts[i], e.pts[nxt[i]])

        heap = [(area_of(i), i) for i in range(n) if not fixed[i]]
        heapq.heapify(heap)
        while heap:
            area, at = heapq.heappop(heap)
            if gone[base + at] or area != area_of(at):
                continue
            if area >= limit or left <= least:
                break
            p, s = prev[at], nxt[at]
            a, b, cc = e.pts[p], e.pts[at], e.pts[s]
            if in_corner(a, b, cc):
                continue
            if not e.closed and left == 3 and any((u == a and v == cc) or (u == cc and v == a) for (u, v) in twos):
                continue
            gone[base + at] = True
            left -= 1
            nxt[p] = s
            prev[s] = p
            for j in (p, s):
                if not fixed[j]:
                    heapq.heappush(heap, (area_of(j), j))
        if not e.closed and left == 2:
            twos.append((e.pts[0], e.pts[n - 1]))
    result = []
    for k, e in enumerate(c.edges):
        base = first[k]
        n = len(e.pts)
        t = Track([], [], [], e.closed)
        for i in range(n):
            if gone[base + i]:
                continue
            if e.closed or i + 1 < n:
                bulge = 0.0 if gone[base + (i + 1) % n] else e.bulges[i]
            else:
                bulge = 0.0
            t.pts.append(e.pts[i])
            t.bulges.append(bulge)
            t.zs.append(e.zs[i])
        result.append(t)
    return result


def coverage_of(objects, tracks):
    at, rings = [], []
    for k, (e, lst) in enumerate(zip(objects, tracks)):
        if e["kind"] != "polygon":
            continue
        for i, t in enumerate(lst):
            if t.closed:
                at.append((k, i))
                rings.append(t)
    return build_coverage(rings), at


# ── Dik açılandır (§3) ─────────────────────────────────────────────────────

SQUARE = 1e-8


def length(x, y):
    return math.sqrt(x * x + y * y)


def square_track(t, threshold, iterations):
    n = len(t.pts)
    if n < 3:
        return t.copy(), False
    lower = math.cos((90.0 - threshold) * math.pi / 180.0)
    upper = math.cos(threshold * math.pi / 180.0)

    def takes(d):
        return lower > abs(d) or abs(d) > upper

    moves = [(t.closed or (0 < i < n - 1)) and not t.on_arc(i) for i in range(n)]

    def nb(pts, i):
        return pts[(i - 1) % n], pts[i], pts[(i + 1) % n]

    def motion(a, b, c):
        px, py, qx, qy = a[0] - b[0], a[1] - b[1], c[0] - b[0], c[1] - b[1]
        lp, lq = length(px, py), length(qx, qy)
        if lp == 0.0 or lq == 0.0:
            return (0.0, 0.0)
        scale = 2.0 * min(lp, lq)
        pnx, pny, qnx, qny = px / lp, py / lp, qx / lq, qy / lq
        d = pnx * qnx + pny * qny
        if not takes(d):
            return (0.0, 0.0)
        if d < -FRAC_1_SQRT_2:
            d += 1.0
        vx, vy = pnx + qnx, pny + qny
        lv = length(vx, vy)
        if lv == 0.0:
            return (0.0, 0.0)
        f = 0.1 * d * scale
        return (vx / lv * f, vy / lv * f)

    def squareness(pts):
        total = 0.0
        for i in range(n):
            if not moves[i]:
                continue
            a, b, c = nb(pts, i)
            px, py, qx, qy = a[0] - b[0], a[1] - b[1], c[0] - b[0], c[1] - b[1]
            lp, lq = length(px, py), length(qx, qy)
            if lp > 0.0:
                px /= lp
                py /= lp
            if lq > 0.0:
                qx /= lq
                qy /= lq
            d = px * qx + py * qy
            if not takes(d):
                continue
            total += 2.0 * min(abs(d - 1.0), min(abs(d), abs(d + 1.0)))
        return total

    best = list(t.pts)
    best_score = squareness(t.pts)
    moved = False
    cur = list(t.pts)
    for _ in range(iterations):
        if best_score < SQUARE:
            break
        steps = [motion(*nb(cur, i)) if moves[i] else (0.0, 0.0) for i in range(n)]
        cur = [(p[0] + m[0], p[1] + m[1]) for p, m in zip(cur, steps)]
        score = squareness(cur)
        if score < best_score:
            best = list(cur)
            best_score = score
            moved = True
    r = t.copy()
    r.pts = best
    return r, moved


# ── Bina sadeleştir (§4) ───────────────────────────────────────────────────


def rebuilt(pts, bulges, first_, frm, count):
    n = len(pts)
    np_ = list(first_)
    nb_ = [0.0] * len(first_)
    for k in range(count):
        i = (frm + k) % n
        np_.append(pts[i])
        nb_.append(bulges[i])
    nb_[-1] = 0.0
    return np_, nb_


def remove_edge(pts, bulges, i, min_edge, parallel, orthogonal):
    n = len(pts)
    if n < 4:
        return None
    ai, bi, b2 = (i - 1) % n, (i + 1) % n, (i + 2) % n
    if bulges[ai] != 0.0 or bulges[i] != 0.0 or bulges[bi] != 0.0:
        return None
    a_, a, b, b_ = pts[ai], pts[i], pts[bi], pts[b2]
    d1x, d1y, d2x, d2y = a_[0] - a[0], a_[1] - a[1], b_[0] - b[0], b_[1] - b[1]
    if (d1x == 0.0 and d1y == 0.0) or (d2x == 0.0 and d2y == 0.0):
        return None
    alpha = math.atan2(d2y, d2x) - math.atan2(d1y, d1x)
    while alpha > math.pi:
        alpha -= 2.0 * math.pi
    while alpha <= -math.pi:
        alpha += 2.0 * math.pi
    tn = abs(alpha)
    if math.pi / 2.0 - orthogonal <= tn <= math.pi / 2.0 + orthogonal:
        den = d1x * d2y - d1y * d2x
        if den == 0.0:
            return None
        wx, wy = b[0] - a[0], b[1] - a[1]
        t = (wx * d2y - wy * d2x) / den
        c = (a[0] + t * d1x, a[1] + t * d1y)
        return rebuilt(pts, bulges, [c], b2, n - 2)
    if n < 5 or bulges[(ai - 1) % n] != 0.0 or bulges[b2] != 0.0:
        return None
    if tn < parallel:
        dd = d2x * d2x + d2y * d2y
        t = ((a_[0] - b[0]) * d2x + (a_[1] - b[1]) * d2y) / dd
        ca = (b[0] + t * d2x, b[1] + t * d2y) if 0.0 < t < 1.0 else None
        ee = d1x * d1x + d1y * d1y
        u = ((b_[0] - a[0]) * d1x + (b_[1] - a[1]) * d1y) / ee
        cb = (a[0] + u * d1x, a[1] + u * d1y) if 0.0 < u < 1.0 else None
        if ca is not None and cb is None:
            pair = [ca, b_]
        elif ca is None and cb is not None:
            pair = [a_, cb]
        elif ca is not None and cb is not None:
            pair = [ca, b_] if length(d1x, d1y) <= length(d2x, d2y) else [a_, cb]
        else:
            return None
        return rebuilt(pts, bulges, pair, b2 + 1, n - 4)
    if tn > math.pi - parallel:
        ux, uy = (a[0] - a_[0]) + (b_[0] - b[0]), (a[1] - a_[1]) + (b_[1] - b[1])
        lu = length(ux, uy)
        if lu == 0.0:
            return None
        ux, uy = ux / lu, uy / lu
        m = ((a_[0] + b_[0]) / 2.0, (a_[1] + b_[1]) / 2.0)

        def foot(p):
            s = (p[0] - m[0]) * ux + (p[1] - m[1]) * uy
            return (m[0] + s * ux, m[1] + s * uy)

        return rebuilt(pts, bulges, [foot(a_), foot(b_)], b2 + 1, n - 4)
    return None


def simplify_rings(rings, min_edge, parallel, orthogonal):
    done = 0
    while True:
        short = []
        for k, (pts, bulges) in enumerate(rings):
            n = len(pts)
            for i in range(n):
                if bulges[i] != 0.0:
                    continue
                a, b = pts[i], pts[(i + 1) % n]
                ln = length(b[0] - a[0], b[1] - a[1])
                if ln < min_edge:
                    short.append((ln, k, i))
        short.sort()
        edited = None
        for (_, k, i) in short:
            r = remove_edge(rings[k][0], rings[k][1], i, min_edge, parallel, orthogonal)
            if r is not None:
                edited = (k, r)
                break
        if edited is None:
            return done
        rings[edited[0]] = edited[1]
        done += 1


def shoelace(pts):
    s = 0.0
    n = len(pts)
    for i in range(n):
        a, b = pts[i], pts[(i + 1) % n]
        s += a[0] * b[1] - b[0] * a[1]
    return s / 2.0


def shapely_polygon(e):
    """A straight-edged polygon object as shapely sees it (parts and holes)."""
    polys = [Polygon([xy(p) for p in e["pts"]], [[xy(p) for p in h["pts"]] for h in e.get("holes") or []])]
    for q in e.get("parts") or []:
        polys.append(Polygon([xy(p) for p in q["pts"]], [[xy(p) for p in h["pts"]] for h in q.get("holes") or []]))
    return polys[0] if len(polys) == 1 else shapely.MultiPolygon(polys)


# ── The drawing ────────────────────────────────────────────────────────────


def layer(i, name, color, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25}, "children": []}


LAYERS = [
    layer("cizim", "Çizim", "fg"),
    layer("bina", "Bina", "#E5484D"),
    layer("yamuk", "Yamuk", "#3E63DD"),
    layer("yol", "Yol", "#AB4ABA"),
    layer("parsel", "Parsel", "#0090FF"),
    layer("ada", "Ada", "#12A594"),
    layer("kadastro", "Kadastro", "#F76B15"),
    layer("yapi", "Yapı", "#8E4EC6"),
    layer("kilitli", "Arşiv", "#8E8C99", locked=True),
]


def ent(i, kind, lay, attrs, **geom):
    e = {"kind": kind, "id": i, "layerId": lay, "attrs": attrs}
    e.update(geom)
    return e


def poly(i, lay, attrs, pts, **more):
    return ent(i, "polygon", lay, attrs, pts=[P(x, y) for x, y in pts], **more)


def ring_of(pts):
    return {"pts": [P(x, y) for x, y in pts]}


def rect(x0, y0, x1, y1):
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]


ENTITIES = [
    # Buildings: a step, a finger with sides of unequal length, a cut corner, a yard with a step, an arc beside a short edge.
    poly(1, "bina", {"Ad": "Basamak"}, [(0, 0), (20, 0), (20, 10), (10, 10), (10, 10.8), (0, 10.8)]),
    poly(2, "bina", {"Ad": "Çıkıntı"}, [(30, 0), (50, 0), (50, 5), (54, 5), (54, 6.5), (51, 6.5), (51, 12), (30, 12)]),
    poly(3, "bina", {"Ad": "Pah"}, [(60, 0), (80, 0), (80, 8), (78, 10), (60, 10)]),
    poly(4, "bina", {"Ad": "Avlulu"}, rect(90, 0, 120, 20),
         holes=[ring_of([(100, 5), (100, 15), (110, 15), (110, 10), (110.8, 10), (110.8, 5)])]),
    poly(5, "bina", {"Ad": "Kemerli"}, [(130, 0), (150, 0), (150, 10), (149, 10), (130, 10)], bulges=[0, 0, 0, 0.25, 0]),
    # Nearly square buildings, one nearly straight corner, a 45° corner that stays, a wall, an arch.
    poly(11, "yamuk", {"Ad": "Yamuk"}, [(0, 30), (20, 30.4), (19.6, 40.3), (-0.3, 39.8)]),
    poly(12, "yamuk", {"Ad": "Düzleşen"}, [(30, 30), (40, 30.1), (50, 30.05), (50.2, 40), (30, 40.1)]),
    poly(13, "yamuk", {"Ad": "Eğik köşe"}, [(60, 30.2), (75, 30), (80, 35), (80, 45), (60.3, 45)]),
    ent(14, "polyline", "yamuk", {"Ad": "Duvar"}, pts=[P(90, 30), P(100, 30.3), P(100.2, 40)]),
    poly(15, "yamuk", {"Ad": "Kemer"}, [(110, 30), (125, 30.2), (125, 40), (110, 40.3)], bulges=[0, 0, 0.25, 0]),
    # Lines to smooth: a creek with elevations, a road with an arc, a pond.
    ent(21, "polyline", "yol", {"Ad": "Dere"}, pts=[P(0, 60), P(10, 65), P(20, 58), P(30, 66), P(40, 60)],
        zs=[10.0, 11.0, 12.0, 13.0, 14.0]),
    ent(22, "polyline", "yol", {"Ad": "Kavisli"}, pts=[P(50, 60), P(60, 60), P(70, 65), P(80, 65)], bulges=[0, 0.3, 0, 0]),
    poly(23, "yol", {"Ad": "Gölet"}, [(90, 55), (110, 55), (115, 65), (100, 72), (88, 64)]),
]

# A coverage of parcels: four around a node with wobbling shared sides, an island in a hole, a notch beside a shed, a lens.
PARCELS = [
    poly(31, "parsel", {"Parsel": "A"}, [(0, 80), (20, 79.8), (40, 80), (40.03, 90), (39.98, 100), (40, 110), (30, 110.03),
                                          (20, 109.98), (10, 110.04), (0, 110)], zs=[1.0, 1.5, 2.0, 2.2, 2.4, 2.6, 2.4, 2.2, 2.0, 1.0]),
    poly(32, "parsel", {"Parsel": "B"}, [(40, 80), (80, 80), (80.3, 95), (80, 110), (60, 110.4), (40, 110), (39.98, 100),
                                          (40.03, 90)]),
    poly(33, "parsel", {"Parsel": "C"}, [(0, 110), (10, 110.04), (20, 109.98), (30, 110.03), (40, 110), (40.2, 125), (40, 140),
                                          (20, 139.5), (0, 140)]),
    poly(34, "parsel", {"Parsel": "D"}, [(40, 110), (60, 110.4), (80, 110), (80, 140), (40, 140), (40.2, 125)],
         holes=[ring_of([(55, 120), (55, 130), (65, 130), (65, 120)])]),
    poly(35, "parsel", {"Parsel": "E"}, [(55, 120), (65, 120), (65, 130), (55, 130)]),
    poly(36, "parsel", {"Parsel": "F"}, [(17, 139.9), (23, 139.9), (23, 145), (17, 145)]),
    poly(37, "parsel", {"Parsel": "G"}, [(100, 80), (110, 79.5), (120, 80), (110, 80.6)]),
    poly(38, "parsel", {"Parsel": "H"}, [(100, 90), (100, 80), (110, 80.6), (120, 80), (120, 90)]),
    poly(39, "parsel", {"Parsel": "I"}, [(100, 70), (120, 70), (120, 80), (110, 79.5), (100, 80)]),
]

ISLANDS = [
    # Parts: a large one with two holes, a small one, a middle one; elevations on the large one's ring.
    poly(41, "ada", {"Ad": "Adalar"}, rect(0, 200, 50, 240), zs=[1.0, 2.0, 3.0, 4.0],
         holes=[ring_of([(5, 205), (5, 207), (7, 207), (7, 205)]), ring_of([(20, 210), (20, 218), (28, 218), (28, 210)])],
         parts=[ring_of(rect(60, 200, 63, 203)), ring_of(rect(70, 200, 80, 210))]),
    poly(42, "ada", {"Ad": "Kırıntılar"}, rect(100, 200, 105, 205), parts=[ring_of(rect(110, 200, 114, 204))]),
]

CADASTRE = [
    poly(51, "kadastro", {"Parsel": "K1"}, rect(0, 150, 40, 180)),
    poly(52, "kadastro", {"Parsel": "K2"}, rect(45, 155, 90, 185)),
    poly(53, "kadastro", {"Parsel": "K3"}, rect(100, 150, 140, 180)),
    poly(54, "kadastro", {"Parsel": "S1"}, rect(40, 150, 45, 170)),
    poly(55, "kadastro", {"Parsel": "S5"}, rect(146, 162, 150, 168)),
    poly(56, "kadastro", {"Parsel": "S6"}, rect(140, 160, 146, 170)),
    poly(57, "kadastro", {"Parsel": "S7"}, rect(160, 150, 163, 153)),
]

BUILDINGS = [
    poly(61, "yapi", {"Tür": "Konut"}, rect(0, 260, 10, 270)),
    poly(62, "yapi", {"Tür": "Konut"}, rect(12, 260, 22, 270)),
    poly(63, "yapi", {"Tür": "Konut"}, rect(40, 260, 50, 270)),
    poly(64, "yapi", {"Tür": "Konut"}, rect(52, 265, 62, 275)),
    poly(65, "yapi", {"Tür": "Konut"}, rect(100, 260, 110, 270)),
    poly(66, "yapi", {"Tür": "Ticari"}, [(120, 260), (140, 260), (140, 275), (132, 275), (132, 272), (128, 272), (128, 275),
                                         (120, 275)]),
    poly(67, "yapi", {"Tür": "Ticari"}, rect(112, 260, 118, 270)),
    poly(68, "yapi", {"Tür": "Konut"}, rect(170, 260, 175, 290)),
    poly(69, "yapi", {"Tür": "Konut"}, rect(177, 260, 193, 265)),
    poly(70, "yapi", {"Tür": "Konut"}, rect(195, 260, 200, 290)),
    poly(71, "yapi", {"Tür": "Konut"}, rect(177, 285, 193, 290)),
]

LOCKED = [poly(81, "kilitli", {"Ad": "Eski bina"}, [(220, 0), (240, 0.5), (239.7, 12), (219.8, 11.6)])]

ENTITIES += PARCELS + ISLANDS + CADASTRE + BUILDINGS + LOCKED

DOCUMENT = {
    "format": "kentos.document",
    "version": 1,
    "name": "İşlem durumları: genelleştirme",
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
    "generalize.smooth": ("Yumuşatılan", "#0090FF", "Yumuşat"),
    "generalize.orthogonalize": ("Dik açılandırılan", "#7C66DC", "Dik açılandır"),
    "generalize.building": ("Sadeleştirilen binalar", "#D6409F", "Bina sadeleştir"),
    "generalize.aggregate": ("Kümelenen alanlar", "#E5484D", "Alanları kümele"),
    "generalize.dropParts": ("Ayıklanan", "#12A594", "Küçük parçaları ayıkla"),
    "generalize.eliminate": ("Katılan", "#F76B15", "Küçükleri komşuya kat"),
    "generalize.coverageSimplify": ("Sadeleştirilen kaplama", "#3E63DD", "Ortak kenarları koruyarak sadeleştir"),
}


def lid(tool):
    return new_id(OUTPUT[tool][0], set())


def out_layer(tool):
    name, color, _ = OUTPUT[tool]
    return {"id": lid(tool), "name": name,
            "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25, "fill": f"{color}26"}}


def case(cid, title, tool, values, expect, selection=None):
    c = {"id": cid, "title": title, "document": "generalize.kcad", "run": {"tool": tool}, "values": values, "expect": expect}
    if selection is not None:
        c["selection"] = selection
    return c


def ok(tool, added, summary, log=None, outputs=None, shapes=False):
    e = {"status": "ok", "summary": summary, "undo": OUTPUT[tool][2] if added else None}
    if added:
        e["layers"] = [out_layer(tool)]
        e["addedShapes" if shapes else "added"] = added
    if log:
        e["log"] = log
    if outputs is not None:
        e["outputs"] = outputs
    return e


def warn(text):
    return {"level": "warn", "text": text}


def heights_note(inputs):
    def held(src):
        return any(z is not None for z in (src.get("zs") or []))

    def elevated(e):
        if any(held(h) for h in e.get("holes") or []) or held(e):
            return True
        return any(held(q) or any(held(h) for h in q.get("holes") or []) for q in e.get("parts") or [])

    c = sum(1 for e in inputs if elevated(e))
    return [warn(f"{c} nesnenin kotları sonuca taşınmadı.")] if c else []


def fixed_text(v, d):
    return shown(v, d)


# ── The tools' cases ───────────────────────────────────────────────────────


def smooth_case(inputs, iterations, offset, max_angle, shared):
    tool = "generalize.smooth"
    max_turn = max_angle * math.pi / 180.0
    tracks = [tracks_of(e) for e in inputs]
    done = [[False] * len(t) for t in tracks]
    if shared:
        c, at = coverage_of(inputs, tracks)
        edges = [smooth_track(e, iterations, offset, max_turn) for e in c.edges]
        for k, (o, i) in enumerate(at):
            tracks[o][i] = coverage_ring(c, edges, k)
            done[o][i] = True
    added, before, after = [], 0, 0
    for e, lst, dn in zip(inputs, tracks, done):
        new = [t if d else smooth_track(t, iterations, offset, max_turn) for t, d in zip(lst, dn)]
        o = shape_like(e, new, lid(tool), dict(e["attrs"]))
        added.append(o)
        before += vertices_of(e)
        after += vertices_of(o)
    summary = f"{len(added)} nesne yazıldı; köşe sayısı {before} → {after}."
    return ok(tool, added, summary, outputs={"count": len(added)})


def square_case(inputs, threshold, iterations):
    tool = "generalize.orthogonalize"
    added, moved, worst = [], 0, 0.0
    for e in inputs:
        new, any_ = [], False
        for t in tracks_of(e):
            r, m = square_track(t, threshold, iterations)
            any_ = any_ or m
            for a, b in zip(t.pts, r.pts):
                worst = max(worst, length(b[0] - a[0], b[1] - a[1]))
            new.append(r)
        moved += 1 if any_ else 0
        added.append(shape_like(e, new, lid(tool), dict(e["attrs"])))
    if moved:
        summary = f"{len(added)} nesne yazıldı; {moved} nesnenin köşeleri dik açılandı, en büyük kayma {fixed_text(worst, 3)} m."
    else:
        summary = f"{len(added)} nesne yazıldı; dik açıya ya da düzlüğe yakın köşe yok."
    return ok(tool, added, summary, outputs={"count": moved})


BUILDING_COLUMNS = ["Nesne", "Köşe (önce)", "Köşe (sonra)", "Alan değişimi (m²)", "Alan değişimi (%)"]


def object_area(o):
    """A straight-edged polygon object's net area (shoelace)."""
    total = 0.0
    for (outer, holes) in parts_of(o):
        total += abs(shoelace(outer.pts)) - sum_holes(holes)
    return total


def sum_holes(holes):
    s = 0.0
    for h in holes:
        s += abs(shoelace(h.pts))
    return s


def building_case(inputs, min_edge, parallel_deg, orthogonal_deg):
    tool = "generalize.building"
    par, ortho = parallel_deg * math.pi / 180.0, orthogonal_deg * math.pi / 180.0
    added, rows, edits_total, changed, grew, invalid = [], [], 0, 0, 0, 0
    for e in inputs:
        parts = []
        edits = 0
        for (outer, holes) in parts_of(e):
            rings = [(list(t.pts), list(t.bulges)) for t in [outer] + holes]
            edits += simplify_rings(rings, min_edge, par, ortho)
            tr = [Track(p, b, [None] * len(p), True) for (p, b) in rings]
            parts.append((tr[0], tr[1:]))
        attrs = dict(e["attrs"])
        if edits == 0:
            added.append(shape_like(e, tracks_of(e), lid(tool), attrs))
            continue
        new = polygon_of(parts, lid(tool), attrs)
        old_area, new_area = object_area(e), object_area(new)
        if new_area > 2.0 * old_area:
            grew += 1
            added.append(shape_like(e, tracks_of(e), lid(tool), attrs))
            continue
        if not shapely_polygon(new).is_valid and shapely_polygon(e).is_valid:
            invalid += 1
            added.append(shape_like(e, tracks_of(e), lid(tool), attrs))
            continue
        added.append(new)
        changed += 1
        edits_total += edits
        change = new_area - old_area
        rows.append([f"#{e['id']}", str(vertices_of(e)), str(vertices_of(new)), fixed_text(change, 2),
                     fixed_text(change / old_area * 100.0, 2)])
    log = []
    if grew:
        log.append(warn(f"{grew} binanın sadeleşmişi iki katından büyük çıktı; olduğu gibi yazıldı."))
    if invalid:
        log.append(warn(f"{invalid} binanın sadeleşmişi geçersiz çıktı; olduğu gibi yazıldı."))
    if rows:
        summary = f"{len(added)} nesne yazıldı; {changed} binada {edits_total} kısa kenar atıldı."
    else:
        summary = f"{len(added)} nesne yazıldı; atılacak kısa kenar yok."
    outputs = {"count": edits_total}
    if rows:
        outputs["report"] = {"columns": BUILDING_COLUMNS, "rows": rows}
    return ok(tool, added, summary, log, outputs)


def parts_case(inputs, min_area, percent, parts_on, holes_on):
    tool = "generalize.dropParts"
    added, gone_parts, gone_holes = [], 0, 0
    for e in inputs:
        parts = parts_of(e)
        total = 0.0
        for (o, _) in parts:
            total += abs(shoelace(o.pts))

        def small(a):
            return a < min_area or (percent > 0.0 and a < total * percent / 100.0)

        net = [abs(shoelace(o.pts)) - sum_holes(hs) for (o, hs) in parts]
        keep = [(not parts_on) or not small(a) for a in net]
        if not any(keep):
            best = 0
            for i, a in enumerate(net):
                if a > net[best]:
                    best = i
            keep[best] = True
        kept, hg = [], 0
        for (o, hs), k in zip(parts, keep):
            if not k:
                continue
            kh = [h for h in hs if (not holes_on) or not small(abs(shoelace(h.pts)))]
            hg += len(hs) - len(kh)
            kept.append((o, kh))
        pg = len(parts) - len(kept)
        gone_parts += pg
        gone_holes += hg
        attrs = dict(e["attrs"])
        if pg == 0 and hg == 0:
            added.append(shape_like(e, tracks_of(e), lid(tool), attrs))
        else:
            added.append(polygon_of(kept, lid(tool), attrs))
    if gone_parts or gone_holes:
        summary = f"{len(added)} nesne yazıldı; {gone_parts} parça ve {gone_holes} delik atıldı."
    else:
        summary = f"{len(added)} nesne yazıldı; atılacak küçük parça ya da delik yok."
    return ok(tool, added, summary, outputs={"count": gone_parts + gone_holes})


COVERAGE_COLUMNS = ["Nesne", "Köşe (önce)", "Köşe (sonra)", "Alan değişimi (m²)", "Alan değişimi (%)"]


def coverage_case(inputs, tol, keep_boundary):
    tool = "generalize.coverageSimplify"
    tracks = [tracks_of(e) for e in inputs]
    c, at = coverage_of(inputs, tracks)
    edges = simplify_coverage(c, tol, keep_boundary)
    for k, (o, i) in enumerate(at):
        tracks[o][i] = coverage_ring(c, edges, k)
    added, rows, removed = [], [], 0
    for e, lst in zip(inputs, tracks):
        o = shape_like(e, lst, lid(tool), dict(e["attrs"]))
        added.append(o)
        b, a = vertices_of(e), vertices_of(o)
        if a != b:
            removed += b - a
            old_area, new_area = object_area(e), object_area(o)
            change = new_area - old_area
            rows.append([f"#{e['id']}", str(b), str(a), fixed_text(change, 2), fixed_text(change / old_area * 100.0, 2)])
    if rows:
        summary = f"{len(added)} nesne yazıldı; {len(rows)} nesnede {removed} köşe atıldı."
    else:
        summary = f"{len(added)} nesne yazıldı; atılacak köşe yok."
    outputs = {"count": removed}
    if rows:
        outputs["report"] = {"columns": COVERAGE_COLUMNS, "rows": rows}
    return ok(tool, added, summary, outputs=outputs)


# ── Alanları kümele (§5) ───────────────────────────────────────────────────

Q = 256  # segments a quarter in GEOS's buffers


def closing_check(inputs, distance, fill, area, parts=1, holes=0):
    """GEOS's closing beside a hand-worked area: the bridges (or every piece) of the closing's excess, joined."""
    polys = [shapely_polygon(e) for e in inputs]
    union = unary_union(polys)
    r = distance / 2.0
    closing = union.buffer(r, quad_segs=Q).buffer(-r, quad_segs=Q)
    extra = closing.difference(union)
    pieces = list(extra.geoms) if hasattr(extra, "geoms") else [extra]
    taken = [union]
    for p in pieces:
        if p.is_empty or p.area < 1e-6:
            continue
        touching = sum(1 for q in polys if p.boundary.intersection(q.boundary).length > 1e-6)
        if fill or touching >= 2:
            taken.append(p)
    result = unary_union(taken)
    geoms = list(result.geoms) if hasattr(result, "geoms") else [result]
    assert len(geoms) == parts, (len(geoms), parts)
    assert sum(len(g.interiors) for g in geoms) == holes
    assert abs(result.area - area) < 2e-4 * area, (result.area, area)
    return geoms


def members_of(geom, inputs):
    return [e for e in inputs if geom.buffer(1e-9).contains(shapely_polygon(e).representative_point())]


def aggregate_case(groups, distance, min_area=0.0, min_hole=0.0, fill=False, group_field=None):
    """`groups`: [(group value, [(members (ids), hand area, holes, hole area)])] in output order."""
    tool = "generalize.aggregate"
    added, dropped, n_inputs = [], 0, 0
    for value, results in groups:
        for (ids, area, holes, hole_area) in results:
            inputs = [by_id(i) for i in ids]
            n_inputs += len(ids)
            geoms = closing_check(inputs, distance, fill, area, 1, holes)
            assert sorted(e["id"] for e in members_of(geoms[0], inputs)) == sorted(ids)
            net = area
            kept_holes = holes
            if holes and hole_area < min_hole:
                net += hole_area
                kept_holes = 0
            if net < min_area:
                dropped += 1
                continue
            attrs = {"Nesne sayısı": str(len(ids))}
            if group_field:
                attrs[group_field] = value
            added.append({"kind": "polygon", "layerId": lid(tool), "attrs": attrs, "parts": 1, "holes": kept_holes,
                          "area": net})
    summary = f"{n_inputs} alandan {len(added)} kümelenmiş alan yazıldı."
    log = [warn(f"{dropped} kümelenmiş alan en küçük alandan küçük; yazılmadı.")] if dropped else []
    return ok(tool, added, summary, log, {"count": len(added)}, shapes=True)


# ── Küçükleri komşuya kat (§7) ─────────────────────────────────────────────

ELIMINATE_COLUMNS = ["Küçük alan", "Katıldığı", "Ortak sınır (m)", "Alan (m²)"]


def eliminate_case(inputs, min_area, largest):
    tool = "generalize.eliminate"
    geom = {e["id"]: shapely_polygon(e) for e in inputs}
    size = {e["id"]: geom[e["id"]].area for e in inputs}
    small = {i for i in size if size[i] < min_area}
    owner = {e["id"]: (None if e["id"] in small else e["id"]) for e in inputs}
    current = {i: geom[i] for i in size}
    order = sorted((i for i in size if i in small), key=lambda i: (size[i], [e["id"] for e in inputs].index(i)))
    index = {e["id"]: k for k, e in enumerate(inputs)}
    joined = []
    while True:
        any_ = False
        for s in order:
            if owner[s] is not None:
                continue
            owners = sorted({owner[e["id"]] for e in inputs if owner[e["id"]] is not None}, key=lambda o: index[o])
            best = None
            for o in owners:
                shared = geom[s].boundary.intersection(current[o].boundary).length
                if shared <= 0.0:
                    continue
                score = current[o].area if largest else shared
                if best is None or score > best[1]:
                    best = (o, score, shared)
            if best is None:
                continue
            o = best[0]
            current[o] = unary_union([current[o], geom[s]])
            owner[s] = o
            joined.append((s, o, best[2]))
            any_ = True
        if not any_:
            break
    changed = {o for (_, o, _) in joined}
    added = []
    for e in inputs:
        i = e["id"]
        if i in small and owner[i] is not None:
            continue
        g = current[i] if i in changed else geom[i]
        holes = len(g.interiors) if g.geom_type == "Polygon" else sum(len(p.interiors) for p in g.geoms)
        parts = 1 if g.geom_type == "Polygon" else len(g.geoms)
        added.append({"kind": "polygon", "layerId": lid(tool), "attrs": dict(e["attrs"]), "parts": parts, "holes": holes,
                      "area": g.area})
    left = [s for s in order if owner[s] is None]
    rows = [[f"#{s}", f"#{o}", fixed_text(sh, 3), fixed_text(size[s], 2)] for (s, o, sh) in joined]
    log = [warn(f"{len(left)} küçük alanın büyük komşusu yok; olduğu gibi yazıldı.")] if left else []
    if joined:
        summary = f"{len(added)} nesne yazıldı; {len(joined)} küçük alan komşusuna katıldı."
    else:
        summary = f"{len(added)} nesne yazıldı; katılacak küçük alan yok."
    outputs = {"count": len(joined)}
    if rows:
        outputs["report"] = {"columns": ELIMINATE_COLUMNS, "rows": rows}
    return ok(tool, added, summary, log, outputs, shapes=True)


# ── The cases ──────────────────────────────────────────────────────────────

PI = math.pi
# Worked by hand for the closing of radius 5 (§5): where a gap's two corners stand level, the closing's edge across it is
# the disc's arc through both, a segment short of the straight line; where one side goes on, the corner fills with the
# square of radius 5 less a quarter disc.
FILLET = 25.0 - 6.25 * PI


def SEG(chord):
    t = 2.0 * math.asin(chord / 10.0)
    return 12.5 * (t - math.sin(t))


def build():
    sm, sq, bu, ag, dp, el, cs = ("generalize.smooth", "generalize.orthogonalize", "generalize.building",
                                  "generalize.aggregate", "generalize.dropParts", "generalize.eliminate",
                                  "generalize.coverageSimplify")
    yol, yamuk, bina, parsel = objects("yol"), objects("yamuk"), objects("bina"), objects("parsel")
    # Two level corners at both ends; offset at both; level at one end, offset at the other (twice); a frame of four.
    pair = ((61, 62), 220.0 - 2.0 * SEG(2.0), 0, 0.0)
    offset = ((63, 64), 210.0 + 2.0 * FILLET, 0, 0.0)
    frame = ((68, 69, 70, 71), 500.0 - 4.0 * SEG(2.0) + 4.0 * FILLET, 1, 400.0 - 4.0 * FILLET)
    konut = [
        ("Konut", [pair, offset, ((65,), 100.0, 0, 0.0), frame]),
        ("Ticari", [((66, 67), 368.0 - SEG(2.0) + FILLET, 0, 0.0)]),
    ]
    together = [("", [pair, offset, ((65, 66, 67), 488.0 - 3.0 * SEG(2.0) + FILLET, 0, 0.0), frame])]
    cases = [
        case("smooth-lines", "Yumuşat: dere, yaylı yol ve gölet; üç yineleme, dörtte bir; kotlar oranla",
             sm, {"input": L("yol"), "iterations": 3, "offset": 0.25, "maxAngle": 180, "shared": False},
             smooth_case(yol, 3, 0.25, 180.0, False)),
        case("smooth-half", "Kesim oranı 0,5, bir yineleme: kenarların ortaları",
             sm, {"input": L("yol"), "iterations": 1, "offset": 0.5, "maxAngle": 180, "shared": False},
             smooth_case(yol, 1, 0.5, 180.0, False)),
        case("smooth-angle", "En büyük açı 60°: binaların dik köşeleri keskin kalır",
             sm, {"input": L("bina"), "iterations": 2, "offset": 0.25, "maxAngle": 60, "shared": False},
             smooth_case(bina, 2, 0.25, 60.0, False)),
        case("smooth-shared", "Ortak kenarları koru: parsellerin ortak kenarları bir kez yumuşar, ada ile delik birlikte",
             sm, {"input": L("parsel"), "iterations": 2, "offset": 0.25, "maxAngle": 180, "shared": True},
             smooth_case(parsel, 2, 0.25, 180.0, True)),
        case("square-default", "Dik açılandır: yamuk binalar, düzleşen köşe, 45° kalan köşe, duvar ve kemer",
             sq, {"input": L("yamuk"), "threshold": 15, "iterations": 1000}, square_case(yamuk, 15.0, 1000)),
        case("square-tight", "Açı eşiği 2°: eşikten uzak köşeler kıpırdamaz",
             sq, {"input": L("yamuk"), "threshold": 2, "iterations": 1000}, square_case(yamuk, 2.0, 1000)),
        case("square-locked", "Kilitli katmanın binası okunur, sonuç yeni katmana",
             sq, {"input": L("kilitli"), "threshold": 15, "iterations": 1000}, square_case(LOCKED, 15.0, 1000)),
        case("building-default", "Bina sadeleştir: basamak, çıkıntı, pah, avludaki basamak; yayın yanındaki kısa kenar kalır",
             bu, {"input": L("bina"), "minEdge": 3.5, "parallel": 20, "orthogonal": 20}, building_case(bina, 3.5, 20.0, 20.0)),
        case("building-none", "En kısa kenar 0,5 m: atılacak kenar yok",
             bu, {"input": L("bina"), "minEdge": 0.5, "parallel": 20, "orthogonal": 20}, building_case(bina, 0.5, 20.0, 20.0)),
        case("parts-area", "Küçük parçaları ayıkla: 20 m²'den küçük parça ve delikler; kotlar kalır",
             dp, {"input": L("ada"), "minArea": 20, "percent": 0, "parts": True, "holes": True},
             parts_case(objects("ada"), 20.0, 0.0, True, True)),
        case("parts-percent", "Yüzde 3: dış halkaların alanının yüzde üçünden küçükler",
             dp, {"input": L("ada"), "minArea": 0, "percent": 3, "parts": True, "holes": True},
             parts_case(objects("ada"), 0.0, 3.0, True, True)),
        case("parts-holes", "Yalnız delikler", dp, {"input": L("ada"), "minArea": 20, "percent": 0, "parts": False, "holes": True},
             parts_case(objects("ada"), 20.0, 0.0, False, True)),
        case("parts-all-small", "Bütün parçaları küçük alanın en büyük parçası kalır",
             dp, {"input": SELECTION, "minArea": 50, "percent": 0, "parts": True, "holes": True},
             parts_case([by_id(42)], 50.0, 0.0, True, True), selection=[42]),
        case("eliminate-longest", "Küçükleri komşuya kat: en uzun ortak sınır; küçüğe değen küçük ikinci geçişte, yalnız kalan söylenir",
             el, {"input": L("kadastro"), "minArea": 150, "into": "longest"}, eliminate_case(objects("kadastro"), 150.0, False)),
        case("eliminate-largest", "En büyük komşu", el, {"input": L("kadastro"), "minArea": 150, "into": "largest"},
             eliminate_case(objects("kadastro"), 150.0, True)),
        case("coverage-default", "Ortak kenarları koruyarak sadeleştir: 1 m; ortak kenarlar bir kez, ada ile delik birlikte; kotlar",
             cs, {"input": L("parsel"), "tolerance": 1, "keepBoundary": False}, coverage_case(parsel, 1.0, False)),
        case("coverage-boundary", "Sınırı koru, 5 m: dış sınır değişmez, ortak kenarlar sadeleşir",
             cs, {"input": L("parsel"), "tolerance": 5, "keepBoundary": True}, coverage_case(parsel, 5.0, True)),
        case("coverage-blocked", "5 m: üçgeninde başka köşe olan köşe ve iki kenarı aynı doğruya indirecek köşe kalır",
             cs, {"input": L("parsel"), "tolerance": 5, "keepBoundary": False}, coverage_case(parsel, 5.0, False)),
        case("aggregate-default", "Alanları kümele: 10 m; hizalı, kaydırılmış ve üçlü köprüler, avlulu çerçeve",
             ag, {"input": L("yapi"), "distance": 10, "minArea": 0, "minHole": 0, "fillNotches": False},
             aggregate_case(together, 10.0)),
        case("aggregate-group", "Türe göre gruplar: konut ve ticari ayrı kümelenir",
             ag, {"input": L("yapi"), "distance": 10, "minArea": 0, "minHole": 0, "fillNotches": False, "group": "Tür"},
             aggregate_case(konut, 10.0, group_field="Tür")),
        case("aggregate-small", "En küçük alan 150 m², en küçük boşluk 500 m²: tek konut yazılmaz, avlu dolar",
             ag, {"input": L("yapi"), "distance": 10, "minArea": 150, "minHole": 500, "fillNotches": False, "group": "Tür"},
             aggregate_case(konut, 10.0, min_area=150.0, min_hole=500.0, group_field="Tür")),
        case("aggregate-notch", "Girintileri doldur: tek binanın girintisi kapanışla dolar",
             ag, {"input": SELECTION, "distance": 10, "minArea": 0, "minHole": 0, "fillNotches": True},
             aggregate_case([("", [((66,), 300.0 - SEG(4.0), 0, 0.0)])], 10.0, fill=True), selection=[66]),
    ]
    return {"format": "kentos.processing-cases", "version": 1, "tolerance": TOL, "toleranceFields": ["z", "zs", "area"],
            "documents": {}, "cases": cases}


def main():
    doc = json.dumps(DOCUMENT, ensure_ascii=False, indent=1) + "\n"
    proc = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    outs = [(PROC_DIR / "generalize.json", proc), (PROC_DIR / "generalize.kcad", doc)]
    if "--check" in sys.argv[1:]:
        bad = [p for p, t in outs if not p.exists() or p.read_text() != t]
        for p in bad:
            print(f"{p.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
        if bad:
            return 1
        print("genelleştirmenin durumları tutarlı: fixtures/processing/v1/generalize*")
        return 0
    for p, t in outs:
        p.write_text(t)
        print(f"yazıldı: {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
