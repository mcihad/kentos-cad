#!/usr/bin/env python3
"""Nokta hesaplayıcı ekleri (docs/adr/0188): the shared cases, written from the
ADR's rules without KentOS code, at 50 digits with mpmath.

- stations: a path's point at a distance from its start and an offset square
  to it (the travel direction's right positive); the start is the first
  vertex, or the last one when walked from the end; a distance outside
  [0, length] gives none (Obje üzerinde nokta, Km ve sapma).
- readings: where a point stands against a path: the distance of its nearest
  point from the start and its signed offset.
- km: the km notation read (`k+mmm.mmm` or metres) and written.
- slope: a slope distance and a percent slope as the horizontal distance and
  the rise (Mesafe ve eğim).
- bisector: the point at a distance along the inner bisector of an angle,
  and a point's nearest place on it (Açıortay).

Paths: a line, a polyline (straight and bulged segments: bulge = tan(θ/4),
counter-clockwise positive), a closed polygon, a circle (from its east
point, counter-clockwise), an arc (counter-clockwise from a0 to a1), an
ellipse (from t0, the parameter rising; its own arc length by quadrature)
and a fit-point curve (centripetal Catmull–Rom, the Barry–Goldman pyramid,
reflected end points; its own arc length by quadrature). The core walks a
curve along chords within 0.1 mm of it (docs/adr/0149 §5.3), so a curve's
case holds to 0.1 mm (`tolerance`), the others to 10⁻⁷ m.

The core runs the cases natively and through WASM
(crates/shared/geometry-core/tests/all/point_calc.rs,
apps/web/src/tools/pointCalcExtras.wasm.test.ts).

    python3 scripts/fixtures/point_calc_cases.py          # write
    python3 scripts/fixtures/point_calc_cases.py --check  # compare
"""

import json
import sys
from decimal import ROUND_HALF_UP, Decimal
from pathlib import Path

import mpmath as mp

mp.mp.dps = 50
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "point-calc" / "v1" / "cases.json"
SOURCE = "scripts/fixtures/point_calc_cases.py (docs/adr/0188)"
PI = mp.pi


def pt(x, y):
    return {"x": x, "y": y}


def line(a, b):
    return {"id": 1, "layerId": "a", "attrs": {}, "kind": "line", "a": pt(*a), "b": pt(*b)}


def polyline(pts, bulges=None):
    e = {"id": 1, "layerId": "a", "attrs": {}, "kind": "polyline", "pts": [pt(*p) for p in pts]}
    if bulges:
        e["bulges"] = bulges
    return e


def polygon(pts, holes=None, parts=None):
    e = {"id": 1, "layerId": "a", "attrs": {}, "kind": "polygon", "pts": [pt(*p) for p in pts]}
    if holes:
        e["holes"] = [{"pts": [pt(*p) for p in h]} for h in holes]
    if parts:
        e["parts"] = [{"pts": [pt(*p) for p in q]} for q in parts]
    return e


def text(p, s):
    return {"id": 1, "layerId": "a", "attrs": {}, "kind": "text", "p": pt(*p), "text": s, "height": 2.5}


def circle(c, r):
    return {"id": 1, "layerId": "a", "attrs": {}, "kind": "circle", "c": pt(*c), "r": r}


def arc(c, r, a0, a1):
    return {"id": 1, "layerId": "a", "attrs": {}, "kind": "arc", "c": pt(*c), "r": r, "a0": a0, "a1": a1}


def ellipse(c, major, ratio, t0, t1):
    return {"id": 1, "layerId": "a", "attrs": {}, "kind": "ellipse", "c": pt(*c), "major": pt(*major), "ratio": ratio, "t0": t0, "t1": t1}


def spline(pts, closed=False):
    return {"id": 1, "layerId": "a", "attrs": {}, "kind": "spline", "pts": [pt(*q) for q in pts], "closed": closed}


def point(p):
    return {"id": 1, "layerId": "a", "attrs": {}, "kind": "point", "p": pt(*p)}


# ── Paths as edges: ("seg", a, b) or ("arc", c, r, a0, sweep) ───────────────

def m(v):
    return mp.mpf(v)


def bulge_edge(a, b, bulge):
    if bulge == 0:
        return ("seg", a, b)
    theta = 4 * mp.atan(m(bulge))
    dx, dy = b[0] - a[0], b[1] - a[1]
    chord = mp.sqrt(dx * dx + dy * dy)
    r = chord / (2 * abs(mp.sin(theta / 2)))
    # The centre sits on the chord's perpendicular: left of a→b for a counter-clockwise arc under a half turn.
    mx, my = (a[0] + b[0]) / 2, (a[1] + b[1]) / 2
    h = mp.sqrt(max(r * r - (chord / 2) ** 2, 0))
    ux, uy = -dy / chord, dx / chord  # the chord's left
    side = 1 if (theta > 0) == (abs(theta) < PI) else -1
    if abs(abs(theta) - PI) < mp.mpf("1e-40"):
        side = 0
    c = (mx + side * h * ux, my + side * h * uy)
    a0 = mp.atan2(a[1] - c[1], a[0] - c[0])
    return ("arc", c, r, a0, theta)


def edges_of(e):
    """The route (docs/adr/0188 §1): an area's outer ring alone; none for a multi-part object or another kind."""
    k = e["kind"]
    if e.get("parts") or k not in ("line", "polyline", "polygon", "circle", "arc"):
        return [], False
    P = lambda v: (m(v["x"]), m(v["y"]))
    if k == "line":
        return [("seg", P(e["a"]), P(e["b"]))], False
    if k in ("polyline", "polygon"):
        pts = [P(v) for v in e["pts"]]
        closed = k == "polygon"
        n = len(pts)
        bulges = e.get("bulges") or []
        count = n if closed else n - 1
        return [bulge_edge(pts[i], pts[(i + 1) % n], bulges[i] if i < len(bulges) else 0) for i in range(count)], closed
    if k == "circle":
        return [("arc", P(e["c"]), m(e["r"]), m(0), 2 * PI)], True
    if k == "arc":
        sweep = (m(e["a1"]) - m(e["a0"])) % (2 * PI)
        if sweep == 0:
            sweep = 2 * PI
        return [("arc", P(e["c"]), m(e["r"]), m(e["a0"]), sweep)], False
    return [], False


def length(ed):
    if ed[0] == "seg":
        _, a, b = ed
        return mp.sqrt((b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2)
    _, c, r, a0, sweep = ed
    return r * abs(sweep)


def at_edge(ed, t):
    """The point and the unit travel direction at fraction t of an edge."""
    if ed[0] == "seg":
        _, a, b = ed
        l = length(ed)
        return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t), ((b[0] - a[0]) / l, (b[1] - a[1]) / l)
    _, c, r, a0, sweep = ed
    ang = a0 + sweep * t
    sgn = 1 if sweep > 0 else -1
    return (c[0] + r * mp.cos(ang), c[1] + r * mp.sin(ang)), (-mp.sin(ang) * sgn, mp.cos(ang) * sgn)


def reversed_edge(ed):
    if ed[0] == "seg":
        _, a, b = ed
        return ("seg", b, a)
    _, c, r, a0, sweep = ed
    return ("arc", c, r, a0 + sweep, -sweep)


def walk(e, from_end):
    edges, closed = edges_of(e)
    if from_end:
        edges = [reversed_edge(ed) for ed in reversed(edges)]
    return edges


def station(e, from_end, s, offset):
    edges = walk(e, from_end)
    if not edges:
        return {"point": None, "length": 0.0}
    total = sum(length(ed) for ed in edges)
    s, offset = m(s), m(offset)
    if s < 0 or s > total:
        return {"point": None, "length": float(total)}
    _, closed = edges_of(e)
    if closed and s == total:
        # Round a closed path, its length is its start again: the first edge's square.
        s = m(0)
    acc = m(0)
    for i, ed in enumerate(edges):
        l = length(ed)
        # At a vertex the next edge's direction (the last edge holds the end).
        if s < acc + l or i == len(edges) - 1:
            p, t = at_edge(ed, (s - acc) / l)
            right = (t[1], -t[0])
            return {"point": [float(p[0] + offset * right[0]), float(p[1] + offset * right[1])], "length": float(total)}
        acc += l


def nearest_on(ed, p):
    """The fraction of an edge nearest p and its distance."""
    if ed[0] == "seg":
        _, a, b = ed
        dx, dy = b[0] - a[0], b[1] - a[1]
        t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)
        t = min(m(1), max(m(0), t))
        q = (a[0] + dx * t, a[1] + dy * t)
        return t, mp.sqrt((p[0] - q[0]) ** 2 + (p[1] - q[1]) ** 2)
    _, c, r, a0, sweep = ed
    ang = mp.atan2(p[1] - c[1], p[0] - c[0])
    # Along the sweep from a0, in its direction.
    rel = ((ang - a0) * (1 if sweep > 0 else -1)) % (2 * PI)
    if rel <= abs(sweep):
        t = rel / abs(sweep)
    else:
        ends = []
        for tt in (m(0), m(1)):
            q, _ = at_edge(ed, tt)
            ends.append((mp.sqrt((p[0] - q[0]) ** 2 + (p[1] - q[1]) ** 2), tt))
        t = min(ends)[1]
    q, _ = at_edge(ed, t)
    return t, mp.sqrt((p[0] - q[0]) ** 2 + (p[1] - q[1]) ** 2)


def reading(e, from_end, at):
    edges = walk(e, from_end)
    p = (m(at[0]), m(at[1]))
    total = sum(length(ed) for ed in edges)
    best = None
    acc = m(0)
    for ed in edges:
        t, d = nearest_on(ed, p)
        if best is None or d < best[0]:
            best = (d, acc + t * length(ed), ed, t)
        acc += length(ed)
    _, s, ed, t = best
    q, tan = at_edge(ed, t)
    right = (tan[1], -tan[0])
    side = (p[0] - q[0]) * right[0] + (p[1] - q[1]) * right[1]
    away = mp.sqrt((p[0] - q[0]) ** 2 + (p[1] - q[1]) ** 2)
    # How far from the way, its right positive.
    offset = -away if side < 0 else away
    return {"s": float(s), "offset": float(offset), "length": float(total)}


# ── Curves: a parameter, its point, its derivative and the arc length ─────

def ellipse_curve(e):
    c, mj, ratio = (m(e["c"]["x"]), m(e["c"]["y"])), (m(e["major"]["x"]), m(e["major"]["y"])), m(e["ratio"])
    mn = (-mj[1] * ratio, mj[0] * ratio)
    t0 = m(e["t0"])
    sweep = (m(e["t1"]) - t0) % (2 * PI)
    if sweep == 0:
        sweep = 2 * PI

    def at(t):
        return (c[0] + mj[0] * mp.cos(t) + mn[0] * mp.sin(t), c[1] + mj[1] * mp.cos(t) + mn[1] * mp.sin(t))

    def d(t):
        return (-mj[0] * mp.sin(t) + mn[0] * mp.cos(t), -mj[1] * mp.sin(t) + mn[1] * mp.cos(t))

    # One piece, the parameter from t0 over the sweep.
    return [(t0, t0 + sweep, at, d)], sweep >= 2 * PI


def catmull_curve(e):
    q = [(m(v["x"]), m(v["y"])) for v in e["pts"]]
    n, closed = len(q), e["closed"]

    def at_i(i):
        if closed:
            return q[i % n]
        if i < 0:
            return (2 * q[0][0] - q[1][0], 2 * q[0][1] - q[1][1])
        if i >= n:
            return (2 * q[n - 1][0] - q[n - 2][0], 2 * q[n - 1][1] - q[n - 2][1])
        return q[i]

    def knot(a, b):
        return mp.sqrt(mp.sqrt((b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2))

    pieces = []
    for i in range(n if closed else n - 1):
        p0, p1, p2, p3 = at_i(i - 1), at_i(i), at_i(i + 1), at_i(i + 2)
        k0 = m(0)
        k1 = k0 + knot(p0, p1)
        k2 = k1 + knot(p1, p2)
        k3 = k2 + knot(p2, p3)

        def make(p0=p0, p1=p1, p2=p2, p3=p3, k0=k0, k1=k1, k2=k2, k3=k3):
            def lerp(a, b, ta, tb, t):
                u, w = (tb - t) / (tb - ta), (t - ta) / (tb - ta)
                return (a[0] * u + b[0] * w, a[1] * u + b[1] * w)

            def dlerp(a, b, da, db, ta, tb, t):
                u, w = (tb - t) / (tb - ta), (t - ta) / (tb - ta)
                k = 1 / (tb - ta)
                return (da[0] * u + db[0] * w + (b[0] - a[0]) * k, da[1] * u + db[1] * w + (b[1] - a[1]) * k)

            def both(t):
                zero = (m(0), m(0))
                a1, a2, a3 = lerp(p0, p1, k0, k1, t), lerp(p1, p2, k1, k2, t), lerp(p2, p3, k2, k3, t)
                da1, da2, da3 = dlerp(p0, p1, zero, zero, k0, k1, t), dlerp(p1, p2, zero, zero, k1, k2, t), dlerp(p2, p3, zero, zero, k2, k3, t)
                b1, b2 = lerp(a1, a2, k0, k2, t), lerp(a2, a3, k1, k3, t)
                db1, db2 = dlerp(a1, a2, da1, da2, k0, k2, t), dlerp(a2, a3, da2, da3, k1, k3, t)
                return lerp(b1, b2, k1, k2, t), dlerp(b1, b2, db1, db2, k1, k2, t)

            return (k1, k2, lambda t: both(t)[0], lambda t: both(t)[1])

        pieces.append(make())
    return pieces, closed


def curve_of(e):
    return ellipse_curve(e) if e["kind"] == "ellipse" else catmull_curve(e)


def speed_of(d):
    return lambda t: mp.sqrt(d(t)[0] ** 2 + d(t)[1] ** 2)


def piece_length(piece, upto=None):
    a, b, _, d = piece
    return mp.quad(speed_of(d), [a, (a + b) / 2, b] if upto is None else [a, upto])


def curve_station(e, from_end, s, offset):
    pieces, closed = curve_of(e)
    lengths = [piece_length(pc) for pc in pieces]
    total = sum(lengths)
    s, offset = m(s), m(offset)
    if s < 0 or s > total:
        return {"point": None, "length": float(total)}
    if closed and s == total:
        s = m(0)
    walk = total - s if from_end else s
    acc = m(0)
    for i, (pc, l) in enumerate(zip(pieces, lengths)):
        if walk <= acc + l or i == len(pieces) - 1:
            a, b, at, d = pc
            want = walk - acc
            guess = a + (b - a) * want / l
            t = mp.findroot(lambda t: piece_length(pc, t) - want, guess, solver="newton", df=speed_of(d))
            p, dv = at(t), d(t)
            ln = mp.sqrt(dv[0] ** 2 + dv[1] ** 2)
            tan = (dv[0] / ln, dv[1] / ln)
            if from_end:
                tan = (-tan[0], -tan[1])
            right = (tan[1], -tan[0])
            return {"point": [float(p[0] + offset * right[0]), float(p[1] + offset * right[1])], "length": float(total)}
        acc += l


def curve_reading(e, from_end, at_p):
    pieces, closed = curve_of(e)
    lengths = [piece_length(pc) for pc in pieces]
    total = sum(lengths)
    p = (m(at_p[0]), m(at_p[1]))
    best = None
    acc = m(0)
    for pc, l in zip(pieces, lengths):
        a, b, at, d = pc
        # The nearest of fine samples, then the foot of the perpendicular.
        ts = [a + (b - a) * k / 64 for k in range(65)]
        t = min(ts, key=lambda t: (at(t)[0] - p[0]) ** 2 + (at(t)[1] - p[1]) ** 2)

        def foot(t):
            q, dv = at(t), d(t)
            return (q[0] - p[0]) * dv[0] + (q[1] - p[1]) * dv[1]

        if a < t < b:
            t = mp.findroot(foot, t)
        q = at(t)
        dist = mp.sqrt((q[0] - p[0]) ** 2 + (q[1] - p[1]) ** 2)
        if best is None or dist < best[0]:
            best = (dist, acc + piece_length(pc, t), pc, t)
        acc += l
    dist, sw, pc, t = best
    _, _, at, d = pc
    q, dv = at(t), d(t)
    side = (p[0] - q[0]) * dv[1] - (p[1] - q[1]) * dv[0]
    offset = -dist if side < 0 else dist
    if from_end:
        sw, offset = total - sw, -offset
    return {"s": float(sw), "offset": float(offset), "length": float(total)}


# ── Km ──────────────────────────────────────────────────────────────────────

def km_value(text):
    t = text.strip()
    import re
    m1 = re.fullmatch(r"([0-9]+)\+([0-9]+(?:\.[0-9]+)?)", t)
    if m1:
        metres = mp.mpf(m1.group(2))
        if metres >= 1000:
            return None
        return float(int(m1.group(1)) * 1000 + metres)
    m2 = re.fullmatch(r"-?[0-9]+(?:\.[0-9]+)?", t)
    return float(mp.mpf(t)) if m2 else None


def display_fixed(v, d):
    """The display rule (docs/adr/0149): the float's exact value rounded to seven decimals, then to d, halves away
    from zero; no minus sign on a zero."""
    x = Decimal(abs(v))
    if d < 7:
        x = x.quantize(Decimal("1e-7"), ROUND_HALF_UP)
    body = x.quantize(Decimal(1).scaleb(-d), ROUND_HALF_UP)
    text = f"{body:.{d}f}"
    return "-" + text if v < 0 and any(c in "123456789" for c in text) else text


def km_text(value, decimals):
    """k+mmm.ddd: the value written by the display rule, its whole metres parted at the thousands."""
    text = display_fixed(value, decimals)
    sign = "-" if text.startswith("-") else ""
    whole, _, frac = text.lstrip("-").partition(".")
    k, metres = divmod(int(whole), 1000)
    return f"{sign}{k}+{metres:03d}" + (f".{frac}" if frac else "")


# ── Mesafe ve eğim, Açıortay ────────────────────────────────────────────────

def slope(s, percent):
    s, e = m(s), m(percent)
    d = s / mp.sqrt(1 + (e / 100) ** 2)
    return {"horizontal": float(d), "rise": float(d * e / 100)}


def bisector_dir(k, a, b):
    ka = (m(a[0]) - k[0], m(a[1]) - k[1])
    kb = (m(b[0]) - k[0], m(b[1]) - k[1])
    la, lb = mp.sqrt(ka[0] ** 2 + ka[1] ** 2), mp.sqrt(kb[0] ** 2 + kb[1] ** 2)
    if la == 0 or lb == 0:
        return None
    ua, ub = (ka[0] / la, ka[1] / la), (kb[0] / lb, kb[1] / lb)
    w = (ua[0] + ub[0], ua[1] + ub[1])
    lw = mp.sqrt(w[0] ** 2 + w[1] ** 2)
    if lw < mp.mpf("1e-30"):
        # A straight angle: square to the first arm, to its left.
        return (-ua[1], ua[0])
    return (w[0] / lw, w[1] / lw)


def bisector(k, a, b, d):
    k = (m(k[0]), m(k[1]))
    u = bisector_dir(k, a, b)
    if u is None:
        return None
    return [float(k[0] + m(d) * u[0]), float(k[1] + m(d) * u[1])]


def bisector_nearest(k, a, b, p):
    k = (m(k[0]), m(k[1]))
    u = bisector_dir(k, a, b)
    if u is None:
        return None
    t = max(m(0), (m(p[0]) - k[0]) * u[0] + (m(p[1]) - k[1]) * u[1])
    return [float(k[0] + t * u[0]), float(k[1] + t * u[1])]


# ── The cases ───────────────────────────────────────────────────────────────

LINE = line((0, 0), (10, 0))
ELL = polyline([(0, 0), (10, 0), (10, 10)])
BULGED = polyline([(0, 0), (10, 0), (20, 0)], [1, 0])
SQUARE = polygon([(0, 0), (10, 0), (10, 10), (0, 10)])
HOLED = polygon([(0, 0), (10, 0), (10, 10), (0, 10)], holes=[[(4, 4), (6, 4), (6, 6), (4, 6)]])
CIRCLE = circle((0, 0), 5)
ARC = arc((0, 0), 10, 0, float(PI / 2))
ELLIPSE_ARC = ellipse((0, 0), (10, 0), 0.5, 0, float(PI))
ELLIPSE_FULL = ellipse((5, 5), (0, 8), 0.75, 0, float(2 * PI))
ELLIPSE_FAR = ellipse((487000.25, 4420000.75), (10.392304845413264, 6.0), 0.4, 0.3, 2.5)
CURVE = spline([(0, 0), (10, 5), (20, 0), (30, 8)])
CURVE_CLOSED = spline([(0, 0), (20, 0), (20, 15), (0, 15)], True)
CURVE_FAR = spline([(487100.5, 4420050.25), (487112.0, 4420058.5), (487125.75, 4420052.0), (487131.0, 4420040.5)])
FAR = polyline([(486990.125, 4419990.25), (487012.5, 4419995.75), (487030.0, 4420011.0)], [0, -0.3])

STATIONS = [
    ("a line from its start", LINE, False, 4, 0),
    ("a line, an offset to the right of its way", LINE, False, 4, 2),
    ("a line from its end: the right is the other side", LINE, True, 4, 2),
    ("a corner takes the next edge's square", ELL, False, 10, 1),
    ("on the second edge", ELL, False, 12, 1),
    ("the middle of a bulged half turn", BULGED, False, float(5 * PI / 2), 1),
    ("past the bulge on the straight", BULGED, False, float(5 * PI + 3), -2),
    ("a closed polygon's last edge", SQUARE, False, 35, 1),
    ("a closed polygon's length is its start, the first edge's square", SQUARE, False, 40, 1),
    ("a circle a quarter round, outward", CIRCLE, False, float(5 * PI / 2), 1),
    ("an arc's middle from its end", ARC, True, float(5 * PI / 2), 0.5),
    ("map coordinates, a bulge turning clockwise", FAR, False, 30.5, -1.25),
    ("an elliptical arc, outward of its way", ELLIPSE_ARC, False, 7, 2),
    ("an elliptical arc from its end", ELLIPSE_ARC, True, 3.5, -1),
    ("a full ellipse, round past its length's half", ELLIPSE_FULL, False, 30, 1.5),
    ("a turned ellipse at map coordinates from its end", ELLIPSE_FAR, True, 6.5, 3),
    ("a fit-point curve", CURVE, False, 17, 2.5),
    ("a closed fit-point curve", CURVE_CLOSED, False, 33, -2),
    ("a fit-point curve at map coordinates from its end", CURVE_FAR, True, 12.25, 4),
    ("past an ellipse's end: none", ELLIPSE_ARC, False, 40, 0),
    ("before the start: none", LINE, False, -0.5, 0),
    ("past the end: none", ELL, False, 20.5, 0),
    ("no path: a point", point((1, 1)), False, 0, 0),
    ("an area with a hole walks its outer ring", HOLED, False, 25, 1),
    ("a multi-part area has no route", polygon([(0, 0), (4, 0), (4, 4)], parts=[[(10, 0), (14, 0), (14, 4)]]), False, 1, 0),
    ("a text has no route", text((0, 0), "A"), False, 0, 0),
]

READINGS = [
    ("left of a line is a negative offset", LINE, False, (3, 2)),
    ("the same point from the line's end", LINE, True, (3, 2)),
    ("under a bulged half turn", BULGED, False, (5, -7)),
    ("beside a polygon's first edge", SQUARE, False, (5, -1)),
    ("map coordinates", FAR, False, (487020.0, 4420000.0)),
    ("outside a corner: the distance to it, on the outer side", ELL, False, (12, -2)),
    ("inside an elliptical arc", ELLIPSE_ARC, False, (3, 2)),
    ("beside a fit-point curve from its end", CURVE, True, (12, 1)),
]

KM_TEXTS = ["0+000", "1+250.5", "12+005.25", "250.5", "-3", "1+1000", "1+", "+250", "abc", " 3+125.125 ", "1+250,5", "1.5+250", "١+٢٥٠"]
KM_WRITES = [(1250.5, 3), (12.3454, 3), (999.9996, 3), (0, 2), (5000, 0), (12005.25, 2), (12.0005, 3), (-12.5, 3), (-0.0004, 3)]
SLOPES = [(100, 0), (100, 10), (50, -5), (10, 100)]
BISECTORS = [
    ("a right angle", (0, 0), (10, 0), (0, 10), 5),
    ("a straight angle: square to the first arm, left", (0, 0), (10, 0), (-10, 0), 5),
    ("arms the same way", (0, 0), (10, 0), (20, 0), 5),
    ("an obtuse angle at map coordinates", (487000.5, 4420000.25), (487010.5, 4420001.25), (486995.5, 4420008.25), 7.5),
    ("a corner on an arm: none", (0, 0), (0, 0), (0, 10), 5),
]
BISECTOR_CLICKS = [
    ("onto a right angle's bisector", (0, 0), (10, 0), (0, 10), (5, 1)),
    ("behind the corner: the corner", (0, 0), (10, 0), (0, 10), (-5, -5)),
]


def curved(e):
    return e["kind"] in ("ellipse", "spline")


def tolerance(e):
    """A curve is walked along chords within 0.1 mm of it (docs/adr/0149 §5.3)."""
    return 1e-4 if curved(e) else 1e-7


def cases():
    return {
        "format": "kentos.point-calc-cases",
        "version": 1,
        "source": SOURCE,
        "stations": [
            {"name": n, "shape": e, "fromEnd": f, "s": s, "offset": o, "tolerance": tolerance(e),
             "expect": (curve_station if curved(e) else station)(e, f, s, o)}
            for n, e, f, s, o in STATIONS
        ],
        "readings": [
            {"name": n, "shape": e, "fromEnd": f, "at": list(p), "tolerance": tolerance(e),
             "expect": (curve_reading if curved(e) else reading)(e, f, p)}
            for n, e, f, p in READINGS
        ],
        "km": [{"text": t, "value": km_value(t)} for t in KM_TEXTS],
        "kmText": [{"value": v, "decimals": d, "text": km_text(v, d)} for v, d in KM_WRITES],
        "slopes": [{"s": s, "percent": p, "expect": slope(s, p)} for s, p in SLOPES],
        "bisectors": [
            {"name": n, "k": list(k), "a": list(a), "b": list(b), "d": d, "expect": bisector(k, a, b, d)}
            for n, k, a, b, d in BISECTORS
        ],
        "bisectorClicks": [
            {"name": n, "k": list(k), "a": list(a), "b": list(b), "at": list(p), "expect": bisector_nearest(k, a, b, p)}
            for n, k, a, b, p in BISECTOR_CLICKS
        ],
    }


def main():
    text = json.dumps(cases(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            sys.exit("fixtures/point-calc/v1/cases.json güncel değil; yeniden yazmak için --check'siz çalıştırın.")
        print("fixtures/point-calc/v1/cases.json güncel.")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print("fixtures/point-calc/v1/cases.json yazıldı.")


if __name__ == "__main__":
    main()
