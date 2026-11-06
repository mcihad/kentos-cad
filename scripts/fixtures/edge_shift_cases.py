#!/usr/bin/env python3
"""Paralel kaydır (docs/adr/0191): the shared cases, written from the ADR's
rules without KentOS code, with exact fractions (the target area's root
with mpmath at 50 digits).

- The edge's line moves by d along its normal (an area: away from its
  inside, so a positive d grows it; a polyline: the right of its way), its
  two vertices go to where the moved line meets the neighbouring edges'
  lines; an open polyline's free end moves square to the edge.
- Refused: a neighbour parallel to the edge, a neighbour or the edge
  shrinking to nothing or turning round.
- A target area: A(d) = A0 + a1·d + a2·d², the root with the smaller |d|.

Arcs (docs/adr/1024, plan/map-cad.md MAP-06), at 50 digits: when the edge or
a neighbour is an arc, an arc is its own circle (centre on the chord's left
at chord/2·cot(θ/2), θ = 4·atan(bulge), radius chord/(2·|sin(θ/2)|)). The
moved edge is its line moved d along the unit normal, or its circle with the
radius grown by d where the normal at its middle points away from the centre
(shrunk otherwise; refused at zero). Each vertex goes to where the moved
edge's curve meets its neighbour's curve (its line, or its circle kept with
its far vertex), the meeting point nearer the old vertex; a free end goes
square (radially for an arc). An arc's new bulge is tan(θ′/4), θ′ the sweep
from its start to its end turning as before, refused when it is gone or
jumps half a turn; a straight neighbour must not pass its far vertex, a
straight edge must not turn round. A target area: bracketed by doubling
|A0 − target|/length outward from 0, solved with mpmath's own root finder.

The core runs the cases natively and through WASM
(crates/shared/geometry-core/tests/all/edge_shift.rs,
apps/web/src/tools/edgeShift.wasm.test.ts).

    python3 scripts/fixtures/edge_shift_cases.py          # write
    python3 scripts/fixtures/edge_shift_cases.py --check  # compare
"""

import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

import mpmath as mp

mp.mp.dps = 50
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "edge-shift" / "v1" / "cases.json"
SOURCE = "scripts/fixtures/edge_shift_cases.py (docs/adr/0191)"

CURVED = "Kaydırılan kenar ya da komşusu yay; paralel kaydırma düz kenarlarla yapılır."
PARALLEL = "Komşu kenar kaydırılan kenara paralel; köşe bulunamıyor."
PASSED = "Kenar bu uzaklıkta komşusunu aşıyor; daha kısa bir uzaklık yazın."
NO_EDGE = "Bu nesnede böyle bir kenar yok."
NO_AREA = "Bu alana kenarı kaydırarak ulaşılamıyor."
NOT_AREA = "Hedef alan yalnız alanlarda yazılır."
MULTI = "Çok parçalı nesnenin kenarı kaydırılmaz; önce Parçalara ayır."
NOT_PATH = "Paralel kaydır alanın ya da çoklu çizginin düz kenarında çalışır; böyle bir kenara tıklayın."


def pt(x, y):
    return {"x": x, "y": y}


def polygon(pts, holes=None, bulges=None):
    e = {"id": 1, "layerId": "a", "attrs": {}, "kind": "polygon", "pts": [pt(*p) for p in pts]}
    if bulges:
        e["bulges"] = bulges
    if holes:
        e["holes"] = [{"pts": [pt(*p) for p in h]} for h in holes]
    return e


def polyline(pts, bulges=None):
    e = {"id": 1, "layerId": "a", "attrs": {}, "kind": "polyline", "pts": [pt(*p) for p in pts]}
    if bulges:
        e["bulges"] = bulges
    return e


def frac_pts(ps):
    return [(F(p["x"]), F(p["y"])) for p in ps]


def M(x):
    """A fraction at 50 digits."""
    return mp.mpf(x.numerator) / x.denominator if isinstance(x, F) else mp.mpf(x)


def ring_area(pts, bulges=()):
    """The signed area of a ring (counter-clockwise positive): exact for
    straight edges; an arc edge (bulge b, sweep 4·atan b, as DXF's) adds its
    segment r²/2·(sweep − sin sweep), at 50 digits."""
    n = len(pts)
    a = sum(pts[i][0] * pts[(i + 1) % n][1] - pts[(i + 1) % n][0] * pts[i][1] for i in range(n)) / 2
    if not any(bulges):
        return a
    total = M(a)
    for i, b in enumerate(bulges):
        if b == 0:
            continue
        p, q = pts[i], pts[(i + 1) % n]
        chord = mp.sqrt(M((q[0] - p[0]) ** 2 + (q[1] - p[1]) ** 2))
        sweep = 4 * mp.atan(M(F(b)))
        r = chord / (2 * abs(mp.sin(sweep / 2)))
        total += r * r / 2 * (sweep - mp.sin(sweep))
    return total


def area_of(rings, bulge_sets):
    """An area's size: its outer ring's less its holes'."""
    sizes = [abs(ring_area(r, b)) for r, b in zip(rings, bulge_sets)]
    if any(not isinstance(v, F) for v in sizes):
        sizes = [M(v) for v in sizes]
    return sizes[0] - sum(sizes[1:])


def meet(p, d, q, e):
    """Where the line p + t·d meets the line q + u·e; None when parallel."""
    cross = d[0] * e[1] - d[1] * e[0]
    # Parallel: the sine of the angle between them at most 1e-12.
    if cross * cross <= F(1, 10**24) * (d[0] ** 2 + d[1] ** 2) * (e[0] ** 2 + e[1] ** 2):
        return None
    t = ((q[0] - p[0]) * e[1] - (q[1] - p[1]) * e[0]) / cross
    return (p[0] + d[0] * t, p[1] + d[1] * t)


def shift(e, ring, edge, dist, check=True):
    """The shape's rings and the ring moved, or a refusal: first what stops
    the edge whatever the distance (its kind, the edge, an arc, a parallel
    neighbour), then what the distance does (a neighbour or the edge passed)."""
    if e.get("parts"):
        return MULTI
    if e["kind"] not in ("polygon", "polyline"):
        return NOT_PATH
    closed = e["kind"] == "polygon"
    if closed:
        rings = [frac_pts(e["pts"])] + [frac_pts(h["pts"]) for h in e.get("holes", [])]
        bulge_sets = [e.get("bulges") or []] + [h.get("bulges") or [] for h in e.get("holes", [])]
    else:
        rings = [frac_pts(e["pts"])]
        bulge_sets = [e.get("bulges") or []]
    if not (0 <= ring < len(rings)):
        return NO_EDGE
    pts, bulges = rings[ring], bulge_sets[ring]
    n = len(pts)
    edges = n if closed else n - 1
    if not (0 <= edge < edges):
        return NO_EDGE
    bulge = lambda i: (bulges[i] if i < len(bulges) else 0)  # noqa: E731
    i, j = edge, (edge + 1) % n
    if bulge(i) != 0:
        return CURVED
    a, b = pts[i], pts[j]
    t = (b[0] - a[0], b[1] - a[1])
    # The normal: an area's away from its inside, a polyline's right.
    right = (t[1], -t[0])
    if closed:
        ccw = ring_area(pts, bulges) > 0
        inside_left = ccw if ring == 0 else not ccw
        n_dir = right if inside_left else (-right[0], -right[1])
    else:
        n_dir = right
    length = t[0] ** 2 + t[1] ** 2
    # n_dir is as long as the edge; the move is dist along the unit normal.
    if length.denominator == 1 and math.isqrt(int(length)) ** 2 == int(length):
        scale = F(dist) / math.isqrt(int(length))
    else:
        # An irrational length: at 50 digits, then back to fractions for the exact steps.
        scale = F(str(mp.mpf(F(dist).numerator) / F(dist).denominator / mp.sqrt(mp.mpf(length.numerator) / length.denominator)))
    move = (n_dir[0] * scale, n_dir[1] * scale)
    a2, b2 = (a[0] + move[0], a[1] + move[1]), (b[0] + move[0], b[1] + move[1])
    # Each end: where the moved line meets its neighbour's line, or a free end square.
    ends = []
    for vertex, moved_to, nb_edge, nb_vertex in ((a, a2, (i - 1) % n, (i - 1) % n), (b, b2, j, (j + 1) % n)):
        free = not closed and ((vertex is a and i == 0) or (vertex is b and j == n - 1))
        if free:
            ends.append((moved_to, None))
            continue
        if bulge(nb_edge) != 0:
            return CURVED
        other = pts[nb_vertex]
        x = meet(moved_to, t, other, (vertex[0] - other[0], vertex[1] - other[1]))
        if x is None:
            return PARALLEL
        ends.append((x, other))
    new = list(pts)
    new[i], new[j] = ends[0][0], ends[1][0]
    if check:
        for (x, other), vertex in zip(ends, (a, b)):
            if other is None:
                continue
            old, now = (vertex[0] - other[0], vertex[1] - other[1]), (x[0] - other[0], x[1] - other[1])
            if old[0] * now[0] + old[1] * now[1] <= 0:
                return PASSED
        moved_edge = (new[j][0] - new[i][0], new[j][1] - new[i][1])
        if moved_edge[0] * t[0] + moved_edge[1] * t[1] <= 0:
            return PASSED
    return rings, bulge_sets, ring, new


def area_after(e, ring, edge, dist):
    rings, bulge_sets, r, new = shift(e, ring, edge, dist, check=False)
    rings = list(rings)
    rings[r] = new
    return area_of(rings, bulge_sets)


def for_area(e, ring, edge, target):
    if e["kind"] != "polygon":
        return NOT_AREA
    if not e.get("parts") and involves_arcs(e, ring, edge):
        return arc_for_area(e, ring, edge, target)
    got = shift(e, ring, edge, 0, check=False)
    if isinstance(got, str):
        return got
    a0 = area_after(e, ring, edge, 0)
    ap, am = area_after(e, ring, edge, 1), area_after(e, ring, edge, -1)
    # A(d) is quadratic in d: its coefficients from three exact values.
    a0, ap, am = M(a0), M(ap), M(am)
    a2 = (ap + am - 2 * a0) / 2
    a1 = (ap - am) / 2
    c = a0 - M(F(target))
    disc = a1 * a1 - 4 * a2 * c
    if disc < 0 or a1 == 0:
        return NO_AREA
    # The root nearer 0, in the form that keeps its digits when a2 is small.
    d = float(-2 * c / (a1 + mp.sign(a1) * mp.sqrt(disc)))
    # The distance found must be one the edge can move by.
    got = shift(e, ring, edge, d)
    return got if isinstance(got, str) else d


def arc_of(p, q, b):
    """An arc segment's circle and signed sweep, at 50 digits; None when straight."""
    if b == 0:
        return None
    th = 4 * mp.atan(M(F(b)))
    px, py, qx, qy = M(p[0]), M(p[1]), M(q[0]), M(q[1])
    dx, dy = qx - px, qy - py
    chord = mp.sqrt(dx * dx + dy * dy)
    h = chord / 2 * mp.cot(th / 2)
    c = ((px + qx) / 2 - dy / chord * h, (py + qy) / 2 + dx / chord * h)
    r = chord / (2 * abs(mp.sin(th / 2)))
    return c, r, th


def mpt(p):
    return (M(p[0]), M(p[1]))


def mdist(p, q):
    return mp.sqrt((p[0] - q[0]) ** 2 + (p[1] - q[1]) ** 2)


def curve(p, q, b):
    a = arc_of(p, q, b)
    if a:
        return ("circle", a[0], a[1])
    P, Q = mpt(p), mpt(q)
    L = mdist(P, Q)
    return ("line", P, ((Q[0] - P[0]) / L, (Q[1] - P[1]) / L))


def meets(u, v, near):
    """The meeting point of two curves nearer `near`; None when they miss (or two lines are parallel)."""
    pick = lambda xs: min(xs, key=lambda x: mdist(x, near)) if xs else None  # noqa: E731
    if u[0] == "circle" and v[0] == "line":
        u, v = v, u
    if u[0] == "line" and v[0] == "line":
        (p, d), (q, e) = u[1:], v[1:]
        x = d[0] * e[1] - d[1] * e[0]
        if abs(x) <= mp.mpf("1e-12"):
            return None
        t = ((q[0] - p[0]) * e[1] - (q[1] - p[1]) * e[0]) / x
        return (p[0] + d[0] * t, p[1] + d[1] * t)
    if u[0] == "line":
        (p, d), (c, r) = u[1:], v[1:]
        wx, wy = p[0] - c[0], p[1] - c[1]
        bb = d[0] * wx + d[1] * wy
        disc = bb * bb - (wx * wx + wy * wy - r * r)
        if disc < 0:
            return None
        s = mp.sqrt(disc)
        return pick([(p[0] + d[0] * t, p[1] + d[1] * t) for t in (-bb - s, -bb + s)])
    (c1, r1), (c2, r2) = u[1:], v[1:]
    k = mdist(c1, c2)
    if k == 0 or k > r1 + r2 or k < abs(r1 - r2):
        return None
    a = (r1 * r1 - r2 * r2 + k * k) / (2 * k)
    h = mp.sqrt(max(r1 * r1 - a * a, 0))
    ex, ey = (c2[0] - c1[0]) / k, (c2[1] - c1[1]) / k
    mx, my = c1[0] + ex * a, c1[1] + ey * a
    return pick([(mx - ey * h, my + ex * h), (mx + ey * h, my - ex * h)])


def sweep_between(c, p, q, old):
    """The sweep from p to q about c turning as `old` does, in (0, 2π) that way; None when gone or jumped."""
    ux, uy, vx, vy = p[0] - c[0], p[1] - c[1], q[0] - c[0], q[1] - c[1]
    a = mp.atan2(ux * vy - uy * vx, ux * vx + uy * vy)
    if old > 0 and a <= 0:
        a += 2 * mp.pi
    elif old < 0 and a >= 0:
        a -= 2 * mp.pi
    if a == 0 or abs(a - old) >= mp.pi:
        return None
    return a


def arc_shift(e, ring, edge, dist):
    """The arc rule: (rings, bulge_sets, ring, new points, new bulges) or a refusal."""
    closed = e["kind"] == "polygon"
    rings = [frac_pts(e["pts"])] + ([frac_pts(h["pts"]) for h in e.get("holes", [])] if closed else [])
    bulge_sets = [list(e.get("bulges") or [])] + ([list(h.get("bulges") or []) for h in e.get("holes", [])] if closed else [])
    pts, bulges = rings[ring], bulge_sets[ring]
    n = len(pts)
    bl = lambda k: (bulges[k] if k < len(bulges) else 0)  # noqa: E731
    i, j = edge, (edge + 1) % n
    a, b = pts[i], pts[j]
    # The unit normal: an area's away from its inside, a polyline's right; an arc's at its middle.
    own = arc_of(a, b, bl(i))
    if own:
        c, r, th = own
        mid_ang = mp.atan2(M(a[1]) - c[1], M(a[0]) - c[0]) + th / 2
        v = (mp.cos(mid_ang), mp.sin(mid_ang))
        along = (-v[1], v[0]) if th > 0 else (v[1], -v[0])
    else:
        A, B = mpt(a), mpt(b)
        L = mdist(A, B)
        along = ((B[0] - A[0]) / L, (B[1] - A[1]) / L)
    right = (along[1], -along[0])
    if closed:
        inside_left = (ring_area(pts, bulges) > 0) if ring == 0 else not (ring_area(pts, bulges) > 0)
        normal = right if inside_left else (-right[0], -right[1])
    else:
        normal = right
    D = M(F(dist)) if not isinstance(dist, mp.mpf) else dist
    if own:
        c, r, th = own
        mid = (c[0] + r * mp.cos(mid_ang), c[1] + r * mp.sin(mid_ang))
        away = 1 if normal[0] * (mid[0] - c[0]) + normal[1] * (mid[1] - c[1]) > 0 else -1
        r2 = r + away * D
        if r2 <= 0:
            return PASSED
        moved = ("circle", c, r2)
    else:
        A = mpt(a)
        moved = ("line", (A[0] + normal[0] * D, A[1] + normal[1] * D), along)
    new = [mpt(p) for p in pts]
    newb = [mp.mpf(0) if bl(k) == 0 else 4 * mp.atan(M(F(bl(k)))) for k in range(n)]  # sweeps for now
    for v_ix, before in ((i, True), (j, False)):
        at = mpt(pts[v_ix])
        if before:
            nb = ((v_ix - 1) % n, (v_ix - 1) % n) if (closed or v_ix > 0) else None
        else:
            nb = (v_ix, (v_ix + 1) % n) if (closed or v_ix + 1 < n) else None
        if nb is None:
            if moved[0] == "line":
                new[v_ix] = (at[0] + normal[0] * D, at[1] + normal[1] * D)
            else:
                c, r2 = moved[1], moved[2]
                w = (at[0] - c[0], at[1] - c[1])
                l = mp.sqrt(w[0] ** 2 + w[1] ** 2)
                new[v_ix] = (c[0] + w[0] / l * r2, c[1] + w[1] / l * r2)
            continue
        seg, far = nb
        s0, s1 = (pts[far], pts[v_ix]) if before else (pts[v_ix], pts[far])
        nbc = curve(s0, s1, bl(seg))
        if moved[0] == "line" and nbc[0] == "line":
            d, e2 = moved[2], nbc[2]
            if abs(d[0] * e2[1] - d[1] * e2[0]) <= mp.mpf("1e-12"):
                return PARALLEL
        to = meets(moved, nbc, at)
        if to is None:
            return PASSED
        if nbc[0] == "line":
            fp = mpt(pts[far])
            if (to[0] - fp[0]) * (at[0] - fp[0]) + (to[1] - fp[1]) * (at[1] - fp[1]) <= 0:
                return PASSED
        else:
            old = 4 * mp.atan(M(F(bl(seg))))
            p0, q0 = (mpt(pts[far]), to) if before else (to, mpt(pts[far]))
            sw = sweep_between(nbc[1], p0, q0, old)
            if sw is None:
                return PASSED
            newb[seg] = sw
        new[v_ix] = to
    if moved[0] == "line":
        if (new[j][0] - new[i][0]) * moved[2][0] + (new[j][1] - new[i][1]) * moved[2][1] <= 0:
            return PASSED
    else:
        sw = sweep_between(moved[1], new[i], new[j], own[2])
        if sw is None:
            return PASSED
        newb[i] = sw
    bulges_out = [mp.tan(x / 4) if x != 0 else mp.mpf(0) for x in newb]
    return rings, bulge_sets, ring, new, bulges_out


def arc_area(rings, bulge_sets, ring, new, newb):
    """An area's size after an arc shift, at 50 digits (the segment rule of `ring_area` on mp values)."""
    def signed(pts, bs):
        n = len(pts)
        a = sum(pts[k][0] * pts[(k + 1) % n][1] - pts[(k + 1) % n][0] * pts[k][1] for k in range(n)) / 2
        for k, b in enumerate(bs):
            if b == 0:
                continue
            p, q = pts[k], pts[(k + 1) % n]
            chord = mp.sqrt((q[0] - p[0]) ** 2 + (q[1] - p[1]) ** 2)
            sw = 4 * mp.atan(b)
            r = chord / (2 * abs(mp.sin(sw / 2)))
            a += r * r / 2 * (sw - mp.sin(sw))
        return a
    sizes = []
    for k, (r, bs) in enumerate(zip(rings, bulge_sets)):
        if k == ring:
            sizes.append(abs(signed(new, newb)))
        else:
            sizes.append(abs(signed([mpt(p) for p in r], [M(F(x)) for x in bs] if bs else [0] * len(r))))
    return sizes[0] - sum(sizes[1:])


def involves_arcs(e, ring, edge):
    closed = e["kind"] == "polygon"
    bs = [e.get("bulges") or []] + [h.get("bulges") or [] for h in e.get("holes", [])]
    pts = [e["pts"]] + [h["pts"] for h in e.get("holes", [])]
    if ring >= len(pts):
        return False
    b, n = bs[ring], len(pts[ring])
    if not (0 <= edge < (n if closed else n - 1)):
        return False
    bl = lambda k: (b[k] if k < len(b) else 0)  # noqa: E731
    near = [edge] + ([(edge - 1) % n] if (closed or edge > 0) else []) + ([(edge + 1) % n] if (closed or edge + 2 < n) else [])
    return any(bl(k) != 0 for k in near)


def arc_for_area(e, ring, edge, target):
    got = arc_shift(e, ring, edge, 0)
    if isinstance(got, str):
        return got
    a0 = arc_area(*got)
    f0 = a0 - M(F(target))
    pts, bs = e["pts"] if ring == 0 else e["holes"][ring - 1]["pts"], (e.get("bulges") or []) if ring == 0 else (e["holes"][ring - 1].get("bulges") or [])
    n = len(pts)
    p, q = (pts[edge]["x"], pts[edge]["y"]), (pts[(edge + 1) % n]["x"], pts[(edge + 1) % n]["y"])
    arc = arc_of(p, q, bs[edge] if edge < len(bs) else 0)
    length = arc[1] * abs(arc[2]) if arc else mdist(mpt(p), mpt(q))

    def f(d):
        g = arc_shift(e, ring, edge, d)
        return g if isinstance(g, str) else arc_area(*g) - M(F(target))

    direction = -1 if f0 > 0 else 1
    guess = abs(f0) / length
    lo = mp.mpf(0)
    for k in range(64):
        d = direction * guess * 2 ** k
        v = f(d)
        if isinstance(v, str):
            return v
        if (v > 0) != (f0 > 0):
            root = mp.findroot(lambda x: f(x), (lo, d), solver="illinois")
            return float(root)
        lo = d
    return NO_AREA


def written(e, ring, edge, dist):
    if e.get("kind") in ("polygon", "polyline") and not e.get("parts") and involves_arcs(e, ring, edge):
        got = arc_shift(e, ring, edge, dist)
        if isinstance(got, str):
            return {"problem": got}
        rings, bulge_sets, r, new, newb = got
        all_rings = [[(M(p[0]), M(p[1])) for p in rr] for rr in rings]
        all_rings[r] = new
        out = {"pts": [[float(p[0]), float(p[1])] for p in all_rings[0]]}
        if len(all_rings) > 1:
            out["holes"] = [[[float(p[0]), float(p[1])] for p in h] for h in all_rings[1:]]
        bs_out = [float(x) for x in newb] if r == 0 else (e.get("bulges") or None)
        if bs_out:
            out["bulges"] = bs_out
        if r > 0:
            out["holeBulges"] = [float(x) for x in newb]
        if e["kind"] == "polygon":
            # The area of the geometry as it is stored: its vertices and bulges are f64s, and at map coordinates a
            # vertex's last bit moves the area by ~10⁻⁸ m² over a 30 m chord; the size is taken from the stored
            # (rounded) values, exactly, at 50 digits.
            stored = [(mp.mpf(float(p[0])), mp.mpf(float(p[1]))) for p in new]
            stored_b = [mp.mpf(float(x)) for x in newb]
            out["area"] = float(arc_area(rings, bulge_sets, r, stored, stored_b))
        return out
    got = shift(e, ring, edge, dist)
    if isinstance(got, str):
        return {"problem": got}
    rings, bulge_sets, r, new = got
    rings = list(rings)
    rings[r] = new
    out = {"pts": [[float(p[0]), float(p[1])] for p in rings[0]]}
    if len(rings) > 1:
        out["holes"] = [[[float(p[0]), float(p[1])] for p in h] for h in rings[1:]]
    if e.get("bulges"):
        out["bulges"] = e["bulges"]
    if e["kind"] == "polygon":
        out["area"] = float(area_of(rings, bulge_sets))
    return out


SQUARE = polygon([(0, 0), (10, 0), (10, 10), (0, 10)])
CW = polygon([(0, 0), (0, 10), (10, 10), (10, 0)])
TRAPEZE = polygon([(0, 0), (20, 0), (15, 10), (5, 10)])
HOLED = polygon([(0, 0), (20, 0), (20, 20), (0, 20)], holes=[[(5, 5), (15, 5), (15, 15), (5, 15)]])
OPEN = polyline([(0, 0), (10, 0), (10, 10), (20, 10)])
ARCED = polygon([(0, 0), (10, 0), (10, 10), (0, 10)], bulges=[0, 0.5, 0, 0])
FAR = polygon([(487000.25, 4420000.5), (487030.25, 4420000.5), (487030.25, 4420020.5), (487000.25, 4420020.5)])
PARALLEL_NEXT = polygon([(0, 0), (10, 0), (20, 0), (20, 10), (0, 10)])
ARC_TOP = polygon([(0, 0), (10, 0), (10, 10), (0, 10)], bulges=[0, 0, 0.25, 0])
KITE = polygon([(0, 0), (10, 0), (10, 2), (0, 10)])
SLANT = polygon([(0, 0), (12, 0), (14, 4), (0, 4)])
TWO_ARCS = polygon([(0, 0), (10, 0), (10, 10), (0, 10)], bulges=[0, 0.3, 0.2, 0])
FAR_ARC = polygon([(487000.25, 4420000.5), (487030.25, 4420000.5), (487030.25, 4420020.5), (487000.25, 4420020.5)],
                  bulges=[0, 0, 0.125, 0])
OPEN_ARC = polyline([(0, 0), (10, 0), (20, 10)], bulges=[0.3, 0])
HOLED_ARC = polygon([(0, 0), (20, 0), (20, 20), (0, 20)], holes=[[(5, 5), (15, 5), (15, 15), (5, 15)]])
HOLED_ARC["holes"][0]["bulges"] = [0, -0.2, 0, 0]
LINE = {"id": 1, "layerId": "a", "attrs": {}, "kind": "line", "a": pt(0, 0), "b": pt(10, 0)}
PARTS = dict(polygon([(0, 0), (10, 0), (10, 10), (0, 10)]), parts=[{"pts": [pt(20, 0), pt(30, 0), pt(30, 10), pt(20, 10)]}])

CASES = [
    ("a square's bottom edge out by 2", SQUARE, 0, 0, 2),
    ("a clockwise square: out is still out", CW, 0, 1, 2),
    ("a trapeze's top edge in by 4: the slanted sides shorten", TRAPEZE, 0, 2, -4),
    ("a hole's edge: out of the area is into the hole", HOLED, 1, 0, 1),
    ("an open polyline's middle edge to its right", OPEN, 0, 1, 3),
    ("an open polyline's first edge: its free end moves square", OPEN, 0, 0, -2),
    ("map coordinates", FAR, 0, 1, 0.75),
    ("a slanted edge of irrational length out by 1", SLANT, 0, 1, 1),
    ("an arc elsewhere on the ring stays", ARC_TOP, 0, 0, 1),
    # Arcs (docs/adr/1024): each arc stays an arc of its own circle.
    ("a curved edge out by 1: its circle grows, its straight neighbours slide", ARCED, 0, 1, 1),
    ("a curved edge in by 1", ARCED, 0, 1, -1),
    ("a straight edge whose next neighbour is an arc: the vertex slides on the arc's circle", ARCED, 0, 0, 1),
    ("a straight edge whose neighbour before it is an arc", ARCED, 0, 2, -1.5),
    ("a curved edge whose neighbour is an arc: two circles meet", TWO_ARCS, 0, 1, 0.5),
    ("a curved kerb in map coordinates out by 0.25 m", FAR_ARC, 0, 2, 0.25),
    ("an open polyline's curved first edge: its free end moves along the radius", OPEN_ARC, 0, 0, 1),
    ("a hole's curved edge", HOLED_ARC, 1, 1, 0.5),
    ("a curved edge shrunk past its centre is refused", ARCED, 0, 1, -40),
    ("a neighbour parallel to the edge is refused", PARALLEL_NEXT, 0, 0, 1),
    ("an edge passing its neighbour is refused", TRAPEZE, 0, 2, -12),
    ("no such edge", SQUARE, 0, 9, 1),
    ("no such ring", SQUARE, 1, 0, 1),
    ("a multi-part area is refused", PARTS, 0, 0, 1),
    ("a line is not an area's or a polyline's edge", LINE, 0, 0, 1),
]

TARGETS = [
    ("a square grown to 120 m²: its edge out by 2", SQUARE, 0, 0, 120),
    ("a trapeze to 180 m² by its top edge: a quadratic", TRAPEZE, 0, 2, 180),
    ("a trapeze to a size no shift of its top edge reaches", TRAPEZE, 0, 2, 100000),
    ("a target reached only past a neighbour is refused", KITE, 0, 0, 30),
    ("a slanted edge to 70 m²", SLANT, 0, 1, 70),
    ("an area with an arc elsewhere to 125 m²", ARC_TOP, 0, 0, 125),
    ("a curved edge to 140 m²", ARCED, 0, 1, 140),
    ("a straight edge with a curved neighbour to 100 m²", ARCED, 0, 0, 100),
    ("a curved kerb in map coordinates to 640 m²", FAR_ARC, 0, 2, 640),
    ("a polyline has no area", OPEN, 0, 1, 50),
]


def cases():
    return {
        "format": "kentos.edge-shift-cases",
        "version": 1,
        "source": SOURCE,
        "shifts": [{"name": n, "shape": e, "ring": r, "edge": k, "distance": d, "expect": written(e, r, k, d)} for n, e, r, k, d in CASES],
        "targets": [
            {"name": n, "shape": e, "ring": r, "edge": k, "target": t, "expect": (lambda v: {"problem": v} if isinstance(v, str) else {"distance": v})(for_area(e, r, k, t))}
            for n, e, r, k, t in TARGETS
        ],
    }


def main():
    text = json.dumps(cases(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            sys.exit("fixtures/edge-shift/v1/cases.json güncel değil; yeniden yazmak için --check'siz çalıştırın.")
        print("fixtures/edge-shift/v1/cases.json güncel.")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print("fixtures/edge-shift/v1/cases.json yazıldı.")


if __name__ == "__main__":
    main()
