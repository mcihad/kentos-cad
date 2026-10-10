#!/usr/bin/env python3
"""Mekânsal ve öznitelik sorgusu (docs/adr/0200): the shared cases, written from the ADR without KentOS code.

Two files come out of it:

- fixtures/spatial-query/v1/cases.json: the core's rules. Relations between pairs of objects (Kesişen, İçeren, İçinde
  kalan, Merkezi içinde) and their least distance; the objects' centres (`$merkez_y`, `$merkez_x`); numbers read by the
  decimal rule; the figures (sum, mean, least, greatest, standard deviation) and one statistic per group; Özet
  istatistik's tables; join keys and plans.
- fixtures/processing/v1/queries.json (with queries.kcad and malikler.csv): the five tools run on a drawing, in the
  processing cases' format (fixtures/processing/README.md).

Geometry: straight edges with exact fractions (orientation, crossing, point in ring by crossing number, distances as
squares, the square root taken last); a circle exactly (|p − c|² against r²; a segment's nearest and farthest distance
from the centre). “On the boundary” and “meet” are within 1 mm. Numbers: Python's Fraction and Decimal; the mean rounded
half to even with `round` on a Fraction; the deviation in floats, two passes in order, formatted half to even.

    python3 scripts/fixtures/spatial_query_cases.py          # write
    python3 scripts/fixtures/spatial_query_cases.py --check  # compare
"""

import copy
import json
import math
import sys
from fractions import Fraction as F
from functools import cmp_to_key
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from point_editor_cases import natural_cmp  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
CORE_OUT = ROOT / "fixtures" / "spatial-query" / "v1" / "cases.json"
PROC_DIR = ROOT / "fixtures" / "processing" / "v1"
SOURCE = "scripts/fixtures/spatial_query_cases.py (docs/adr/0200)"

TOL = F(1, 1000)
E, N = 487000, 4420000


def P(x, y):
    return (F(E) + F(x), F(N) + F(y))


def jp(p):
    """A point as the contract writes it."""
    return {"x": num(p[0]), "y": num(p[1])}


def num(v):
    """A fraction as a JSON number: an integer when it is one."""
    v = F(v)
    return int(v) if v.denominator == 1 else float(v)


# ── Shapes ────────────────────────────────────────────────────────────
#
# A shape here: {"areas": [(outer, [holes])], "paths": [[p…]], "points": [p…], "circles": [(c, r)], "center": p,
# "json": the contract's entity}. A circle is an area of its own (a disk).


def ring_area2(ring):
    """Twice the signed area."""
    s = F(0)
    for i in range(len(ring)):
        (x0, y0), (x1, y1) = ring[i], ring[(i + 1) % len(ring)]
        s += x0 * y1 - x1 * y0
    return s


def ring_centroid(ring):
    """Area and first moments of a ring (signed)."""
    a = F(0)
    mx = F(0)
    my = F(0)
    for i in range(len(ring)):
        (x0, y0), (x1, y1) = ring[i], ring[(i + 1) % len(ring)]
        c = x0 * y1 - x1 * y0
        a += c / 2
        mx += (x0 + x1) * c / 6
        my += (y0 + y1) * c / 6
    return a, mx, my


def polygon(name, outer, holes=(), parts=()):
    """An area: its outer ring, holes, and more parts (each (outer, holes))."""
    all_parts = [(list(outer), [list(h) for h in holes])] + [(list(o), [list(h) for h in hs]) for o, hs in parts]
    total = F(0)
    sx = F(0)
    sy = F(0)
    for o, hs in all_parts:
        a, mx, my = ring_centroid(o)
        sign = 1 if a >= 0 else -1
        pa, px, py = a * sign, mx * sign, my * sign
        for h in hs:
            a2, mx2, my2 = ring_centroid(h)
            s2 = 1 if a2 >= 0 else -1
            pa -= a2 * s2
            px -= mx2 * s2
            py -= my2 * s2
        total += pa
        sx += px
        sy += py
    center = (sx / total, sy / total)
    entity = {"kind": "polygon", "pts": [jp(p) for p in outer]}
    if holes:
        entity["holes"] = [{"pts": [jp(p) for p in h]} for h in holes]
    if parts:
        entity["parts"] = [{"pts": [jp(p) for p in o], **({"holes": [{"pts": [jp(p) for p in h]} for h in hs]} if hs else {})} for o, hs in parts]
    return name, {"areas": all_parts, "paths": [], "points": [], "circles": [], "center": center, "json": entity}


def square(name, x0, y0, x1, y1, holes=()):
    return polygon(name, [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)], holes)


def line(name, a, b):
    mid = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
    return name, {"areas": [], "paths": [[a, b]], "points": [], "circles": [], "center": mid, "json": {"kind": "line", "a": jp(a), "b": jp(b)}}


def polyline(name, pts):
    # Its centre is its placement point: the middle vertex (`pts[n / 2]`, rounded down).
    return name, {"areas": [], "paths": [list(pts)], "points": [], "circles": [], "center": pts[len(pts) // 2], "json": {"kind": "polyline", "pts": [jp(p) for p in pts]}}


def point(name, p, more=()):
    pts = [p] + list(more)
    center = (sum(q[0] for q in pts) / len(pts), sum(q[1] for q in pts) / len(pts))
    entity = {"kind": "point", "p": jp(p)}
    if more:
        entity["parts"] = [{"p": jp(q)} for q in more]
    return name, {"areas": [], "paths": [], "points": pts, "circles": [], "center": center, "json": entity}


def circle(name, c, r):
    return name, {"areas": [], "paths": [], "points": [], "circles": [(c, F(r))], "center": c, "json": {"kind": "circle", "c": jp(c), "r": num(r)}}


def text(name, p, words):
    return name, {"areas": [], "paths": [], "points": [p], "circles": [], "center": p, "json": {"kind": "text", "p": jp(p), "text": words, "height": 2, "rotation": 0}}


# ── Exact geometry ────────────────────────────────────────────────────

def orient(a, b, c):
    v = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    return (v > 0) - (v < 0)


def on_seg_exact(a, b, p):
    return orient(a, b, p) == 0 and min(a[0], b[0]) <= p[0] <= max(a[0], b[0]) and min(a[1], b[1]) <= p[1] <= max(a[1], b[1])


def segs_cross(a, b, c, d):
    o1, o2, o3, o4 = orient(a, b, c), orient(a, b, d), orient(c, d, a), orient(c, d, b)
    if o1 != o2 and o3 != o4:
        return True
    return (o1 == 0 and on_seg_exact(a, b, c)) or (o2 == 0 and on_seg_exact(a, b, d)) or (o3 == 0 and on_seg_exact(c, d, a)) or (o4 == 0 and on_seg_exact(c, d, b))


def d2_point_seg(p, a, b):
    """Squared distance from p to segment ab (exact)."""
    dx, dy = b[0] - a[0], b[1] - a[1]
    L = dx * dx + dy * dy
    if L == 0:
        return (p[0] - a[0]) ** 2 + (p[1] - a[1]) ** 2
    t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / L
    t = max(F(0), min(F(1), t))
    q = (a[0] + t * dx, a[1] + t * dy)
    return (p[0] - q[0]) ** 2 + (p[1] - q[1]) ** 2


def d2_seg_seg(a, b, c, d):
    if segs_cross(a, b, c, d):
        return F(0)
    return min(d2_point_seg(a, c, d), d2_point_seg(b, c, d), d2_point_seg(c, a, b), d2_point_seg(d, a, b))


def ring_segs(ring):
    return [(ring[i], ring[(i + 1) % len(ring)]) for i in range(len(ring))]


def in_ring(ring, p):
    """Crossing number, exact (a point on the ring is decided before)."""
    inside = False
    for a, b in ring_segs(ring):
        if (a[1] > p[1]) != (b[1] > p[1]):
            x = a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1])
            if x > p[0]:
                inside = not inside
    return inside


def boundary_segs(s):
    out = []
    for o, hs in s["areas"]:
        out += ring_segs(o)
        for h in hs:
            out += ring_segs(h)
    return out


def all_segs(s):
    out = boundary_segs(s)
    for path in s["paths"]:
        out += [(path[i], path[i + 1]) for i in range(len(path) - 1)]
    return out


def strictly_in_areas(s, p):
    """Inside a polygon area (outer, out of every hole) or a disk, the boundary aside."""
    for o, hs in s["areas"]:
        if in_ring(o, p) and not any(in_ring(h, p) for h in hs):
            return True
    for c, r in s["circles"]:
        if (p[0] - c[0]) ** 2 + (p[1] - c[1]) ** 2 < r * r:
            return True
    return False


def circle_gap(c, r, p):
    """| |p − c| − r |, as a float."""
    return abs(math.sqrt(float((p[0] - c[0]) ** 2 + (p[1] - c[1]) ** 2)) - float(r))


def on_area_boundary(s, p):
    if any(d2_point_seg(p, a, b) <= TOL * TOL for a, b in boundary_segs(s)):
        return True
    return any(circle_gap(c, r, p) <= float(TOL) for c, r in s["circles"])


def in_or_on(s, p):
    return strictly_in_areas(s, p) or on_area_boundary(s, p)


def meets_point(s, p):
    if strictly_in_areas(s, p):
        return True
    if any(d2_point_seg(p, a, b) <= TOL * TOL for a, b in all_segs(s)):
        return True
    if any(circle_gap(c, r, p) <= float(TOL) for c, r in s["circles"]):
        return True
    return any((p[0] - q[0]) ** 2 + (p[1] - q[1]) ** 2 <= TOL * TOL for q in s["points"])


def seg_circle_meet(a, b, c, r):
    """A segment and a circle's line meet (within 1 mm): its nearest point not beyond, its farthest not within."""
    near = math.sqrt(float(d2_point_seg(c, a, b)))
    far = math.sqrt(float(max((a[0] - c[0]) ** 2 + (a[1] - c[1]) ** 2, (b[0] - c[0]) ** 2 + (b[1] - c[1]) ** 2)))
    return near <= float(r) + float(TOL) and far >= float(r) - float(TOL)


def starts(s):
    out = [o[0] for o, _ in s["areas"]] + [path[0] for path in s["paths"]]
    out += [(c[0] + r, c[1]) for c, r in s["circles"]]
    return out


def intersects(s, t):
    for a, b in all_segs(s):
        for c, d in all_segs(t):
            if d2_seg_seg(a, b, c, d) <= TOL * TOL:
                return True
        for c, r in t["circles"]:
            if seg_circle_meet(a, b, c, r):
                return True
    for c, r in s["circles"]:
        for a, b in all_segs(t):
            if seg_circle_meet(a, b, c, r):
                return True
        for c2, r2 in t["circles"]:
            dd = math.sqrt(float((c[0] - c2[0]) ** 2 + (c[1] - c2[1]) ** 2))
            if abs(float(r) - float(r2)) - float(TOL) <= dd <= float(r + r2) + float(TOL):
                return True
    if any(meets_point(t, p) for p in s["points"]) or any(meets_point(s, p) for p in t["points"]):
        return True
    return any(strictly_in_areas(t, p) for p in starts(s)) or any(strictly_in_areas(s, p) for p in starts(t))


def seg_pieces(a, b, cuts):
    """The segment ab cut where it crosses the segments `cuts`: the pieces' middles."""
    ts = {F(0), F(1)}
    for c, d in cuts:
        den = (b[0] - a[0]) * (d[1] - c[1]) - (b[1] - a[1]) * (d[0] - c[0])
        if den == 0:
            continue
        t = ((c[0] - a[0]) * (d[1] - c[1]) - (c[1] - a[1]) * (d[0] - c[0])) / den
        u = ((c[0] - a[0]) * (b[1] - a[1]) - (c[1] - a[1]) * (b[0] - a[0])) / den
        if 0 <= t <= 1 and 0 <= u <= 1:
            ts.add(t)
    ts = sorted(ts)
    return [(a[0] + (ts[i] + ts[i + 1]) / 2 * (b[0] - a[0]), a[1] + (ts[i] + ts[i + 1]) / 2 * (b[1] - a[1])) for i in range(len(ts) - 1)]


def contains(s, t):
    """`s`'s areas hold `t` wholly (in or on), no hole of `s` inside `t`'s area."""
    if not s["areas"] and not s["circles"]:
        return False
    if not (t["areas"] or t["paths"] or t["points"] or t["circles"]):
        return False
    if not all(in_or_on(s, p) for p in t["points"]):
        return False
    cuts = boundary_segs(s)
    for a, b in all_segs(t):
        if not (in_or_on(s, a) and in_or_on(s, b)):
            return False
        for m in seg_pieces(a, b, cuts):
            if not in_or_on(s, m):
                return False
        # A disk's edge: a segment is inside a disk when its ends are (convex).
    for c, r in t["circles"]:
        # A disk inside: its centre inside and the boundary no nearer than its radius (less 1 mm).
        if not strictly_in_areas(s, c):
            return False
        segs = boundary_segs(s)
        if segs and min(math.sqrt(float(d2_point_seg(c, a, b))) for a, b in segs) < float(r) - float(TOL):
            return False
        for c2, r2 in s["circles"]:
            if math.sqrt(float((c[0] - c2[0]) ** 2 + (c[1] - c2[1]) ** 2)) + float(r) > float(r2) + float(TOL):
                return False
    for o, hs in s["areas"]:
        for h in hs:
            p = h[0]
            if strictly_in_areas(t, p) and not on_area_boundary(t, p):
                return False
    return True


def distance(s, t):
    if intersects(s, t):
        return 0.0
    best = math.inf
    segs_s, segs_t = all_segs(s), all_segs(t)
    for p in s["points"]:
        for q in t["points"]:
            best = min(best, math.sqrt(float((p[0] - q[0]) ** 2 + (p[1] - q[1]) ** 2)))
        for a, b in segs_t:
            best = min(best, math.sqrt(float(d2_point_seg(p, a, b))))
        for c, r in t["circles"]:
            best = min(best, circle_gap(c, r, p))
    for q in t["points"]:
        for a, b in segs_s:
            best = min(best, math.sqrt(float(d2_point_seg(q, a, b))))
        for c, r in s["circles"]:
            best = min(best, circle_gap(c, r, q))
    for a, b in segs_s:
        for c, d in segs_t:
            best = min(best, math.sqrt(float(d2_seg_seg(a, b, c, d))))
    for c, r in s["circles"]:
        for a, b in segs_t:
            best = min(best, math.sqrt(float(d2_point_seg(c, a, b))) - float(r))
    for c, r in t["circles"]:
        for a, b in segs_s:
            best = min(best, math.sqrt(float(d2_point_seg(c, a, b))) - float(r))
    return best


def center_in(s, t):
    return in_or_on(t, s["center"])


# ── The core's shapes and pairs ───────────────────────────────────────

SHAPES = dict([
    square("A1", 0, 0, 10, 10),
    square("A2", 2, 2, 4, 4),
    square("A3", 10, 0, 15, 5),
    square("A4", 20, 0, 25, 5),
    square("A5", 30, 0, 50, 20, holes=[[P(35, 5), P(45, 5), P(45, 15), P(35, 15)]]),
    square("A6", 37, 7, 43, 13),
    square("A7", 31, 1, 34, 4),
    square("A8", 33, 3, 47, 17),
    polygon("L", [P(60, 0), P(70, 0), P(70, 4), P(64, 4), P(64, 10), P(60, 10)]),
    polygon("M", [P(80, 0), P(84, 0), P(84, 4), P(80, 4)], parts=[([P(90, 0), P(94, 0), P(94, 4), P(90, 4)], [])]),
    line("L1", P(-5, 5), P(5, 5)),
    line("L2", P(1, 1), P(9, 9)),
    line("L3", P(10, 2), P(10, 8)),
    polyline("L4", [P(-5, -5), P(-5, 20), P(5, 20)]),
    point("Pt1", P(5, 5)),
    point("Pt2", P(10, 5)),
    point("Pt3", P(12, 12)),
    point("Pt4", P(10, F(50005, 10000))),
    point("MP1", P(1, 1), [P(20, 20)]),
    circle("C1", P(40, 30), 5),
    circle("C2", P(5, 5), 3),
    circle("C3", P(12, 5), 3),
    text("T1", P(3, 7), "Ağaç"),
])

PAIRS = [
    ("A1", "A2"), ("A2", "A1"), ("A1", "A3"), ("A1", "A4"), ("A5", "A6"), ("A6", "A5"), ("A7", "A5"), ("A8", "A5"),
    ("A5", "A8"), ("L1", "A1"), ("L2", "A1"), ("L3", "A1"), ("L4", "A1"), ("Pt1", "A1"), ("Pt2", "A1"), ("Pt3", "A1"),
    ("Pt4", "A1"), ("MP1", "A1"), ("C1", "A5"), ("C2", "A1"), ("C3", "A1"), ("C3", "A3"), ("C2", "A3"), ("T1", "A1"),
    ("A1", "Pt1"), ("A1", "T1"), ("L1", "L2"), ("Pt2", "Pt4"), ("A2", "L"), ("M", "A1"), ("Pt1", "C2"), ("C2", "Pt1"),
]


def relation_row(a, b):
    s, t = SHAPES[a], SHAPES[b]
    return {
        "a": a,
        "b": b,
        "intersects": intersects(s, t),
        "contains": contains(s, t),
        "within": contains(t, s),
        "centerIn": center_in(s, t),
        "distance": distance(s, t),
    }


# ── Numbers ───────────────────────────────────────────────────────────

JS_SPACE = " \t\n\v\f\r                 　﻿"


def read_number(t):
    """(mantissa, scale) by the decimal rule (ADR 0199 §1), or None."""
    t = t.strip(JS_SPACE)
    neg = t.startswith("-")
    if neg or t.startswith("+"):
        t = t[1:]
    seps = [i for i, c in enumerate(t) if c in ".,"]
    if len(seps) > 1:
        return None
    if seps:
        i = seps[0]
        whole, frac = t[:i], t[i + 1:]
    else:
        whole, frac = t, ""
    if not whole and not frac:
        return None
    if not all(c in "0123456789" for c in whole + frac):
        return None
    whole = whole.lstrip("0")
    if len(whole) + len(frac) > 30:
        return None
    digits = whole + frac
    m = int(digits) if digits else 0
    return (-m if neg else m, len(frac))


def dec_text(m, scale):
    neg = m < 0
    s = str(abs(m)).rjust(scale + 1, "0")
    body = s if scale == 0 else s[:-scale] + "." + s[-scale:]
    return ("-" + body) if neg else body


def figures(values, scale=None):
    nums, skipped = [], 0
    for v in values:
        if v is None or v.strip(JS_SPACE) == "":
            continue
        d = read_number(v)
        if d is None:
            skipped += 1
        else:
            nums.append(d)
    out = {"read": len(nums), "skipped": skipped, "sum": None, "mean": None, "min": None, "max": None, "std": None}
    if not nums:
        return out
    top = max(s for _, s in nums)
    total = sum(F(m, 10 ** s) for m, s in nums)
    out["sum"] = dec_text(int(total * 10 ** top), top)
    scale_out = top + 2 if scale is None else scale
    mean = total / len(nums)
    out["mean"] = dec_text(round(mean * 10 ** scale_out), scale_out)
    lo = min(nums, key=lambda d: F(d[0], 10 ** d[1]))
    hi = max(nums, key=lambda d: F(d[0], 10 ** d[1]))
    out["min"] = dec_text(*lo)
    out["max"] = dec_text(*hi)
    if len(nums) >= 2:
        xs = [float(dec_text(m, s)) for m, s in nums]
        mu = 0.0
        for x in xs:
            mu += x
        mu = mu / len(xs)
        ss = 0.0
        for x in xs:
            ss += (x - mu) * (x - mu)
        std = math.sqrt(ss / (len(xs) - 1))
        t = format(std, f".{scale_out}f")
        if t.startswith("-") and set(t[1:]) <= set("0."):
            t = t[1:]
        out["std"] = t
    return out


def statistic(values, stat, scale=None):
    given = [v for v in values if v is not None and v.strip(JS_SPACE) != ""]
    if stat == "count":
        return {"value": str(len(values)), "skipped": 0}
    if stat == "first":
        return {"value": given[0] if given else None, "skipped": 0}
    f = figures(values, scale)
    return {"value": f[stat], "skipped": f["skipped"]}


def summarize(groups, values, grouped):
    columns = (["Grup"] if grouped else []) + ["Nesne", "Değer", "Toplam", "Ortalama", "En az", "En çok", "Std. sapma"]
    total = figures(values)

    def row(name, count, f):
        cells = [] if name is None else [name]
        return cells + [str(count), str(f["read"]), f["sum"] or "", f["mean"] or "", f["min"] or "", f["max"] or "", f["std"] or ""]

    if not grouped:
        return {"columns": columns, "rows": [row(None, len(values), total)], "skipped": total["skipped"]}
    names, members, empty = [], {}, []
    for g, v in zip(groups, values):
        k = (g or "").strip(JS_SPACE)
        if not k:
            empty.append(v)
            continue
        if k not in members:
            names.append(k)
            members[k] = []
        members[k].append(v)
    reads = [read_number(v) for v in values if v is not None and v.strip(JS_SPACE) != ""]
    scale = max([d[1] for d in reads if d is not None], default=0) + 2
    rows = []
    for k in sorted(names, key=cmp_to_key(natural_cmp)):
        rows.append(row(k, len(members[k]), figures(members[k], scale)))
    if empty:
        rows.append(row("(boş)", len(empty), figures(empty, scale)))
    rows.append(row("Toplam", len(values), total))
    return {"columns": columns, "rows": rows, "skipped": total["skipped"]}


def canonical_number(m, s):
    t = dec_text(m, s)
    if "." in t:
        t = t.rstrip("0").rstrip(".")
        if t == "-0":
            t = "0"
    return t


def join_key(t):
    t = t.strip(JS_SPACE)
    if not t:
        return None
    d = read_number(t)
    return "#" + canonical_number(*d) if d is not None else "$" + t


def join_plan(targets, sources):
    first, repeated, repeated_rows = {}, set(), 0
    for i, s in enumerate(sources):
        k = join_key(s) if s is not None else None
        if k is None:
            continue
        if k in first:
            repeated.add(k)
            repeated_rows += 1
        else:
            first[k] = i
    matches = []
    for t in targets:
        k = join_key(t) if t is not None else None
        matches.append(first.get(k) if k is not None else None)
    used = {m for m in matches if m is not None}
    keyed = sum(1 for s in sources if s is not None and join_key(s) is not None)
    return {"matches": matches, "repeated": len(repeated), "unused": keyed - len(used) - repeated_rows}


NUMBERS = [" 1,50 ", "-0.0", "+007", "1e3", "1.2.3", "", ",5", "5,", "-", "12345678901234567890123456789012", "١٢", "0001.000", "-3"]

FIGURES = [
    ([" 0.1", "0,2", "x", None], None),
    (["100", "25,5", "70", "36"], None),
    (["100", "25,5", "70", "36"], 1),
    (["0.125"], 2),
    (["0.135"], 2),
    (["1", "2"], 0),
    (["1", "4"], 0),
    (["-1.5", "-2.5"], 0),
    (["7"], None),
    (["a", "b"], None),
    (["1000000000000000000000.5", "1000000000000000000000.25"], None),
]

STATISTICS = [
    ([None, "3", "x", "4"], "count", None),
    ([None, " ", "Konut", "Ticaret"], "first", None),
    (["70", "25,5"], "mean", 1),
    (["70", "25,5"], "mean", 0),
    (["70", "x", "25,5"], "sum", None),
    (["70", "25,5"], "min", None),
    (["70", "25,5"], "max", None),
    ([], "sum", None),
]

SUMMARIES = [
    (["2", "1", "2", ""], ["70", "100", "25,5", "36"], True),
    (["Konut", "konut", "İşyeri", None, "Arsa 10", "Arsa 2"], ["1", "2", "x", "4", "5", "6"], True),
    ([None, None, None], ["600", "750", "450"], False),
]

KEYS = [" 007 ", "7", "1.50", "1,5", "7a", "", "  ", "101/5", "-0", "0"]

JOINS = [
    (["7", "8", None], ["007", "9", "7"]),
    (["1", "2", "3", "4"], ["1", "2", "002", "9"]),
    (["101/5", "101/6"], ["101/5", " 101/5", "101/7"]),
]


def build_core():
    centers = []
    for name in ["A1", "A5", "L", "M", "L1", "L4", "MP1", "C1", "T1"]:
        c = SHAPES[name]["center"]
        centers.append({"shape": name, "center": [num(c[0]), num(c[1])]})
    return {
        "format": "kentos.spatial-query-cases",
        "version": 1,
        "source": SOURCE,
        "tolerance": num(TOL),
        "shapes": {k: v["json"] for k, v in SHAPES.items()},
        "relations": [relation_row(a, b) for a, b in PAIRS],
        "centers": centers,
        "numbers": [{"text": t, "number": (dec_text(*read_number(t)) if read_number(t) is not None else None)} for t in NUMBERS],
        "figures": [{"values": v, "scale": s, "expect": figures(v, s)} for v, s in FIGURES],
        # The core writes no value when there is none (an absent field, not null).
        "statistics": [{"values": v, "stat": st, "scale": s, "expect": {k: x for k, x in statistic(v, st, s).items() if x is not None}} for v, st, s in STATISTICS],
        "summaries": [{"groups": g, "values": v, "grouped": gr, "expect": summarize(g, v, gr)} for g, v, gr in SUMMARIES],
        "keys": [{"text": t, "key": join_key(t)} for t in KEYS],
        "joins": [{"targets": t, "sources": s, "expect": join_plan(t, s)} for t, s in JOINS],
    }


# ── The processing cases' drawing ─────────────────────────────────────

def style(color, weight=0.25, **more):
    return {"color": color, "lineType": "continuous", "lineWeight": weight, **more}


def layer(i, name, st, locked=False, fields=None):
    out = {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True, "style": st, "children": []}
    if fields:
        out["fields"] = fields
    return out


PARCEL_FIELDS = [
    {"name": "Ada", "kind": "text"},
    {"name": "Parsel", "kind": "integer"},
    {"name": "Ağaç sayısı", "kind": "integer"},
    {"name": "Taban toplamı", "kind": "decimal", "scale": 2},
    {"name": "Taban ortalaması", "kind": "decimal", "scale": 1},
]

LAYERS = [
    layer("cizim", "Çizim", style("fg")),
    layer("parsel", "Parsel", style("#E5484D", 0.35, fill="#E5484D1F"), fields=PARCEL_FIELDS),
    layer("yapi", "Yapı", style("#8C9AAA")),
    layer("agac", "Ağaç", style("#30A46C", point={"symbol": "ring", "size": 6})),
    layer("yol", "Yol", style("fg", 0.5)),
    layer("kayit", "Tapu kayıtları", style("#3E63DD", point={"symbol": "cross", "size": 6})),
    layer("kilitli", "Kilitli parseller", style("fg-dim"), locked=True),
]


def ent(i, kind, layer_id, attrs=None, label=None, **geom):
    out = {"kind": kind, "id": i, "layerId": layer_id}
    if label is not None:
        out["label"] = label
    out["attrs"] = attrs or {}
    out.update(geom)
    return out


def rect_pts(x0, y0, x1, y1):
    return [jp(P(x0, y0)), jp(P(x1, y0)), jp(P(x1, y1)), jp(P(x0, y1))]


ENTITIES = [
    ent(1, "polygon", "parsel", {"Ada": "101", "Parsel": "1", "Tapu alanı": "600.00"}, "1", pts=rect_pts(0, 0, 20, 30)),
    ent(2, "polygon", "parsel", {"Ada": "101", "Parsel": "2", "Tapu alanı": "750.00"}, "2", pts=rect_pts(20, 0, 45, 30)),
    ent(3, "polygon", "parsel", {"Ada": "101", "Parsel": "3", "Tapu alanı": "450,25"}, "3", pts=rect_pts(45, 0, 60, 30)),
    ent(4, "polygon", "parsel", {"Ada": "102", "Parsel": "4", "Tapu alanı": "1100"}, "4", pts=rect_pts(60, 0, 100, 30),
        holes=[{"pts": rect_pts(70, 10, 80, 20)}]),
    ent(5, "polygon", "yapi", {"Taban alanı": "70", "Kat": "2"}, pts=rect_pts(5, 5, 15, 12)),
    ent(6, "polygon", "yapi", {"Taban alanı": "100", "Kat": "1"}, pts=rect_pts(25, 5, 35, 15)),
    ent(7, "polygon", "yapi", {"Taban alanı": "25,5", "Kat": "2"}, pts=rect_pts(40, 20, 50, 25)),
    ent(8, "polygon", "yapi", {"Taban alanı": "36", "Kat": ""}, pts=rect_pts(72, 12, 78, 18)),
    ent(9, "polygon", "yapi", {"Taban alanı": "bilinmiyor", "Kat": "1"}, pts=rect_pts(85, 2, 95, 8)),
    ent(10, "point", "agac", {"Tür": "Çam"}, p=jp(P(5, 20))),
    ent(11, "point", "agac", {"Tür": "Meşe"}, p=jp(P(10, 25))),
    ent(12, "point", "agac", {"Tür": "Çam"}, p=jp(P(30, 25))),
    ent(13, "point", "agac", {"Tür": "Ihlamur"}, p=jp(P(45, 10))),
    ent(14, "point", "agac", {"Tür": "Çam"}, p=jp(P(75, 15))),
    ent(15, "point", "agac", {"Tür": "Meşe"}, p=jp(P(90, 22))),
    ent(16, "line", "yol", {"Ad": "Cumhuriyet Caddesi"}, a=jp(P(-10, -5)), b=jp(P(110, -5))),
    ent(17, "polyline", "yol", {"Ad": "Okul Sokağı"}, pts=[jp(P(52, -10)), jp(P(52, 40))]),
    ent(18, "point", "kayit", {"Parsel": "1", "Malik": "Ayşe Yılmaz"}, p=jp(P(10, 15))),
    ent(19, "point", "kayit", {"Parsel": "2", "Malik": "Hasan Demir"}, p=jp(P(32, 20))),
    ent(20, "point", "kayit", {"Parsel": "002", "Malik": "Tekrar Kayıt"}, p=jp(P(33, 20))),
    ent(21, "point", "kayit", {"Parsel": "9", "Malik": "Kayıtsız"}, p=jp(P(120, 15))),
    ent(22, "polygon", "kilitli", {"Ada": "103", "Parsel": "5"}, "5", pts=rect_pts(0, 40, 20, 60)),
]

DOCUMENT = {
    "format": "kentos.document",
    "version": 1,
    "name": "İşlem durumları: sorgular",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                 "workspace": "gis", "drawingFont": "barlow"},
    "origin": {"x": E, "y": N},
    "layers": LAYERS,
    "activeLayer": "cizim",
    "entities": ENTITIES,
    "styles": {"items": [], "categories": []},
}

CSV_TEXT = "Parsel;Malik;Hisse\n1;Ayşe Yılmaz;1/2\n3;Mehmet Kaya;1\n03;Tekrar Kayıt;1\n5;Kimse;1\n"
CSV_ROWS = [line.split(";") for line in CSV_TEXT.strip("\n").split("\n")]


# ── Shapes of the drawing, for the reference's own relations ──────────

def shape_of(e):
    k = e["kind"]
    pt = lambda q: (F(q["x"]), F(q["y"]))  # noqa: E731
    if k == "polygon":
        holes = [[pt(q) for q in h["pts"]] for h in e.get("holes", [])]
        return polygon(str(e["id"]), [pt(q) for q in e["pts"]], holes)[1]
    if k == "line":
        return line(str(e["id"]), pt(e["a"]), pt(e["b"]))[1]
    if k == "polyline":
        return polyline(str(e["id"]), [pt(q) for q in e["pts"]])[1]
    if k == "point":
        return point(str(e["id"]), pt(e["p"]))[1]
    raise ValueError(k)


BY_ID = {e["id"]: e for e in ENTITIES}
SHAPE_OF = {e["id"]: shape_of(e) for e in ENTITIES}
LOCKED = {lay["id"] for lay in LAYERS if lay["locked"]}


def on_layer(layer_id):
    return [e["id"] for e in ENTITIES if e["layerId"] == layer_id]


def related(i, j, relation, within=0):
    s, t = SHAPE_OF[i], SHAPE_OF[j]
    if relation == "intersects":
        return intersects(s, t)
    if relation == "contains":
        return contains(s, t)
    if relation == "within":
        return contains(t, s)
    if relation == "near":
        return distance(s, t) <= within + 1e-9 * max(abs(within), 1)
    if relation == "centerIn":
        return center_in(s, t)
    raise ValueError(relation)


RELATION_LABEL = {
    "intersects": "Kesişen",
    "contains": "İçeren",
    "within": "İçinde kalan",
    "disjoint": "Ayrık",
    "near": "Uzaklıkta",
    "centerIn": "Merkezi içinde",
}

STAT_LABEL = {"count": "sayı", "sum": "toplam", "mean": "ortalama", "min": "en az", "max": "en çok", "first": "ilk değer"}


def canonical_for(layer_id, name, value):
    """A value as the layer's field takes it (ADR 0199 §1, the kinds these cases use); unchanged without a field."""
    fields = next((lay.get("fields") for lay in LAYERS if lay["id"] == layer_id), None) or []
    f = next((f for f in fields if f["name"] == name), None)
    if f is None or value is None:
        return value
    d = read_number(value)
    if f["kind"] == "integer":
        assert d is not None and d[1] == 0, value
        return str(d[0])
    if f["kind"] == "decimal":
        assert d is not None and d[1] <= f["scale"], (value, f)
        return dec_text(*d)
    return value


def field_scale(layer_id, name):
    fields = next((lay.get("fields") for lay in LAYERS if lay["id"] == layer_id), None) or []
    f = next((f for f in fields if f["name"] == name), None)
    if f is None:
        return None
    return 0 if f["kind"] == "integer" else f.get("scale") if f["kind"] == "decimal" else None


def updated_rows(changes):
    """The processing cases' `updated`: id, every attribute after, the label when it has one."""
    out = []
    for i in sorted(changes):
        e = BY_ID[i]
        row = {"id": i, "attrs": changes[i]}
        if "label" in e:
            row["label"] = e["label"]
        out.append(row)
    return out


def case_location(cid, title, values, selection=None, inputs=None, refs=None, relation="intersects", within=0, mode="new"):
    hits = []
    for i in inputs:
        found = any(related(i, j, "intersects" if relation == "disjoint" else relation, within) for j in refs if j != i)
        if found != (relation == "disjoint"):
            hits.append(i)
    current = list(selection or [])
    hit = set(hits)
    if mode == "new":
        sel = hits
    elif mode == "add":
        sel = current + [i for i in hits if i not in current]
    elif mode == "remove":
        sel = [i for i in current if i not in hit]
    else:
        sel = [i for i in current if i in hit]
    n = len(dict.fromkeys(sel))
    c = {
        "id": cid,
        "title": title,
        "document": "queries.kcad",
        "run": {"tool": "selection.byLocation"},
        "values": values,
        "expect": {
            "status": "ok",
            "summary": f"{len(hits)} / {len(inputs)} nesne “{RELATION_LABEL[relation]}” ilişkisini sağladı; seçimde {n} nesne var.",
            "undo": None,
            "selection": sel,
            "outputs": {"count": len(hits), "matched": hits},
        },
    }
    if selection is not None:
        c["selection"] = selection
    return c


def case_inside(cid, title, values, targets, sources, relation, stat, field, output, log=None):
    changes = {}
    written = 0
    skipped = 0
    for t in targets:
        members = [s for s in sources if s != t and related(s, t, relation)]
        vals = [BY_ID[s]["attrs"].get(field) if field else None for s in members]
        scale = field_scale(BY_ID[t]["layerId"], output) if stat == "mean" else None
        r = statistic(vals, stat, scale)
        skipped += r["skipped"]
        attrs = dict(BY_ID[t]["attrs"])
        value = canonical_for(BY_ID[t]["layerId"], output, r["value"])
        if value is None:
            if output not in attrs:
                continue
            del attrs[output]
        else:
            if attrs.get(output) == value:
                continue
            attrs[output] = value
        changes[t] = attrs
        written += 1
    lines = list(log or [])
    if skipped:
        lines.append({"level": "warn", "text": f"{skipped} değer sayı olarak okunamadığı için atlandı."})
    expect = {
        "status": "ok",
        "summary": f"{written} hedef nesneye “{output}” yazıldı ({STAT_LABEL[stat]}).",
        **({"log": lines} if lines else {}),
        "undo": "İçindekinden bilgi al",
        "updated": updated_rows(changes),
        "outputs": {"count": written, "changed": sorted(changes)},
    }
    return {"id": cid, "title": title, "document": "queries.kcad", "run": {"tool": "attributes.fromInside"}, "values": values, "expect": expect}


def case_enclosing(cid, title, values, targets, sources, field, output):
    changes = {}
    none = many = 0
    for t in targets:
        holders = [s for s in sources if s != t and related(t, s, "centerIn")]
        if not holders:
            none += 1
            continue
        if len(holders) > 1:
            many += 1
        value = BY_ID[holders[0]]["attrs"].get(field)
        attrs = dict(BY_ID[t]["attrs"])
        value = canonical_for(BY_ID[t]["layerId"], output, value)
        if value is None:
            if output not in attrs:
                continue
            del attrs[output]
        else:
            if attrs.get(output) == value:
                continue
            attrs[output] = value
        changes[t] = attrs
    lines = []
    if many:
        lines.append({"level": "warn", "text": f"{many} nesnenin merkezi birden çok alanın içinde; çizim sırasıyla ilki alındı."})
    if none:
        lines.append({"level": "warn", "text": f"{none} nesnenin merkezi hiçbir alanın içinde değil; değişmedi."})
    expect = {
        "status": "ok",
        "summary": f"{len(changes)} nesneye “{output}” yazıldı.",
        **({"log": lines} if lines else {}),
        "undo": "Çevreleyenden bilgi al",
        "updated": updated_rows(changes),
        "outputs": {"count": len(changes), "changed": sorted(changes)},
    }
    return {"id": cid, "title": title, "document": "queries.kcad", "run": {"tool": "attributes.fromEnclosing"}, "values": values, "expect": expect}


def case_summary(cid, title, values, objects, field, group):
    groups = [BY_ID[i]["attrs"].get(group) for i in objects] if group else [None] * len(objects)
    vals = [BY_ID[i]["attrs"].get(field) for i in objects]
    s = summarize(groups, vals, bool(group))
    n_groups = len(s["rows"]) - 1 if group else 0
    read = sum(1 for v in vals if v is not None and v.strip(JS_SPACE) != "" and read_number(v) is not None)
    summary = f"{len(objects)} nesnede “{field}”: {read} değer okundu" + (f", {n_groups} grup." if group else ".")
    lines = []
    if s["skipped"]:
        lines.append({"level": "warn", "text": f"{s['skipped']} değer sayı olarak okunamadığı için atlandı."})
    expect = {
        "status": "ok",
        "summary": summary,
        **({"log": lines} if lines else {}),
        "undo": None,
        "outputs": {"count": len(objects), "table": {"columns": s["columns"], "rows": s["rows"]}},
    }
    return {"id": cid, "title": title, "document": "queries.kcad", "run": {"tool": "statistics.summary"}, "values": values, "expect": expect}


def case_join(cid, title, values, targets, target_key, rows, source_key, fields, prefix, only_empty):
    header = rows[0]
    if source_key not in header:
        raise ValueError(source_key)
    k = header.index(source_key)
    body = rows[1:]
    take = [i for i, h in enumerate(header) if i != k] if not fields else [header.index(f) for f in fields]
    plan = join_plan([BY_ID[t]["attrs"].get(target_key) for t in targets], [r[k] if k < len(r) else None for r in body])
    changes = {}
    for t, m in zip(targets, plan["matches"]):
        if m is None:
            continue
        attrs = dict(BY_ID[t]["attrs"])
        changed = False
        for i in take:
            name = prefix + header[i]
            value = body[m][i] if i < len(body[m]) else ""
            value = canonical_for(BY_ID[t]["layerId"], name, value) if value != "" else value
            if only_empty and attrs.get(name, "").strip(JS_SPACE) != "":
                continue
            if value == "":
                if name in attrs:
                    del attrs[name]
                    changed = True
                continue
            if attrs.get(name) != value:
                attrs[name] = value
                changed = True
        if changed:
            changes[t] = attrs
    matched = sum(1 for m in plan["matches"] if m is not None)
    unmatched = len(targets) - matched
    lines = []
    if plan["repeated"]:
        lines.append({"level": "warn", "text": f"{plan['repeated']} anahtar kaynakta birden çok kez var; ilk satırları alındı."})
    if unmatched:
        lines.append({"level": "info", "text": f"{unmatched} hedef nesnenin kaynakta eşi yok."})
    if plan["unused"]:
        lines.append({"level": "info", "text": f"{plan['unused']} kaynak satırı hiçbir hedefle eşleşmedi."})
    expect = {
        "status": "ok",
        "summary": f"{matched} nesne eşleşti; {len(changes)} nesnede {len(take)} alan aktarıldı.",
        **({"log": lines} if lines else {}),
        "undo": "Anahtarla birleştir",
        "updated": updated_rows(changes),
        "outputs": {"count": matched, "changed": sorted(changes)},
    }
    return {"id": cid, "title": title, "document": "queries.kcad", "run": {"tool": "attributes.joinByField"}, "values": values, "expect": expect}


# ── Expressions that look at other layers (docs/adr/0214 §2.6–§2.8) ──────────
# The tools run them with the drawing's layers: İfadeyle seç and Öznitelik hesapla. A layer is named as the tree
# shows it; the relations are the ones above, an aggregate's numbers kentos.statistics/1's.

def layer_named(name):
    return next(lay["id"] for lay in LAYERS if lay["name"] == name)


def number_key(v):
    """A value as keys and groups compare it: numbers (and text that reads as one) by value, text as it is."""
    if v is None or v == "":
        return None
    d = read_number(v)
    return ("n", d[0] / 10 ** d[1]) if d is not None and "," not in v else ("t", v)


def case_expr_select(cid, title, layer_id, condition, test):
    inputs = on_layer(layer_id)
    hits = [i for i in inputs if test(i)]
    expect = {
        "status": "ok",
        "summary": f"{len(hits)} / {len(inputs)} nesne koşulu sağladı; seçimde {len(hits)} nesne var.",
        "undo": None,
        "selection": hits,
        "outputs": {"count": len(hits), "matched": hits},
    }
    return {"id": cid, "title": title, "document": "queries.kcad", "run": {"tool": "selection.byExpression"},
            "values": {"input": {"scope": "layer", "layerId": layer_id}, "condition": condition}, "expect": expect}


def case_expr_calculate(cid, title, layer_id, field, value, compute):
    inputs = on_layer(layer_id)
    changes, empty = {}, 0
    for i in inputs:
        v = compute(i)
        if v is None:
            empty += 1
            continue
        v = canonical_for(layer_id, field, v)
        attrs = dict(BY_ID[i]["attrs"])
        attrs[field] = v
        changes[i] = attrs
    notes = [f"{empty} nesnede sonuç boş olduğu için dokunulmadı"] if empty else []
    expect = {
        "status": "ok",
        "summary": f"{len(changes)} nesnede “{field}” yazıldı" + "".join(f"; {n}" for n in notes) + ".",
        "undo": "Öznitelik hesapla",
        "updated": updated_rows(changes),
        "outputs": {"count": len(changes), "changed": sorted(changes)},
    }
    return {"id": cid, "title": title, "document": "queries.kcad", "run": {"tool": "attributes.calculate"},
            "values": {"input": {"scope": "layer", "layerId": layer_id}, "field": field, "value": value, "where": "",
                       "empty": "keep", "label": False}, "expect": expect}


def nearest(i, layer_id):
    """The nearest object of a layer to `i` (another than it): the first in the layer's order among equals."""
    others = [j for j in on_layer(layer_id) if j != i]
    return min(others, key=lambda j: distance(SHAPE_OF[i], SHAPE_OF[j])) if others else None


def world_cases():
    roads, trees = on_layer("yol"), on_layer("agac")
    parcels = on_layer("parsel")
    kat = {i: number_key(BY_ID[i]["attrs"].get("Kat")) for i in on_layer("yapi")}
    return [
        case_expr_select("expr-intersects", "İfadeyle seç, başka katmanla: Yol'la kesişen parseller (Konuma göre seç'in Kesişen'i gibi)",
                         "parsel", "kesişir('Yol')", lambda i: any(related(i, r, "intersects") for r in roads)),
        case_expr_calculate("expr-tree-count", "Öznitelik hesapla, kesişen sayısı: parseldeki ağaçlar; sınırdaki ağaç iki parselde, delikteki hiçbirinde; tam sayı alanına",
                            "parsel", "Ağaç sayısı", "kesişen_sayısı('Ağaç')",
                            lambda i: str(sum(1 for t in trees if related(i, t, "intersects")))),
        case_expr_calculate("expr-nearest-road", "Öznitelik hesapla, en yakın: her ağaca en yakın yolun adı",
                            "agac", "Yol", "en_yakın('Yol', Ad)", lambda i: BY_ID[nearest(i, "yol")]["attrs"]["Ad"]),
        case_expr_calculate("expr-group-count", "Öznitelik hesapla, katmanın toplaması: aynı kattaki yapı sayısı; boş kat kendi grubu",
                            "yapi", "Aynı kattaki", "say($id, Kat)",
                            lambda i: str(sum(1 for j in kat if kat[j] == kat[i]))),
        case_expr_calculate("expr-from-layer", "Öznitelik hesapla, başka katmandan: tapu kaydının parselinin adası; 002 ile 2 aynı anahtar, eşi olmayana dokunulmaz",
                            "kayit", "Ada", "katmandan('Parsel', Ada, 'Parsel', Parsel)",
                            lambda i: next((BY_ID[p]["attrs"]["Ada"] for p in parcels
                                            if number_key(BY_ID[p]["attrs"].get("Parsel")) == number_key(BY_ID[i]["attrs"].get("Parsel"))), None)),
        case_expr_calculate("expr-variable", "Öznitelik hesapla, projenin değişkeniyle: @proje_adi çizimin adıdır",
                            "parsel", "Proje", "@proje_adi || ' / ' || Parsel",
                            lambda i: f"{DOCUMENT['name']} / {BY_ID[i]['attrs']['Parsel']}"),
    ]


def layer_rows(layer_id, key_field, fields):
    """A layer's objects as a join's rows: the key's column, then the fields."""
    header = [key_field] + fields
    rows = [header]
    for i in on_layer(layer_id):
        rows.append([BY_ID[i]["attrs"].get(h, "") for h in header])
    return rows


def build_processing():
    parcels = [i for i in on_layer("parsel")]
    buildings = on_layer("yapi")
    trees = on_layer("agac")
    roads = on_layer("yol")
    lay = lambda i: {"scope": "layer", "layerId": i}  # noqa: E731
    cases = [
        case_location("location-intersects", "Konuma göre seç, Kesişen: Okul Sokağı'nın geçtiği parsel",
                      {"input": lay("parsel"), "relation": "intersects", "reference": lay("yol")}, inputs=parcels, refs=roads),
        case_location("location-within", "İçinde kalan: parsellerin içindeki yapılar; iki parseli aşan ve parselin deliğindeki yapı seçilmez",
                      {"input": lay("yapi"), "relation": "within", "reference": lay("parsel")}, inputs=buildings, refs=parcels, relation="within"),
        case_location("location-contains", "İçeren: bir yapıyı bütünüyle içeren parseller",
                      {"input": lay("parsel"), "relation": "contains", "reference": lay("yapi")}, inputs=parcels, refs=buildings, relation="contains"),
        case_location("location-disjoint", "Ayrık: yollarla kesişmeyen parseller (cadde 5 m güneyde, dokunmaz)",
                      {"input": lay("parsel"), "relation": "disjoint", "reference": lay("yol")}, inputs=parcels, refs=roads, relation="disjoint"),
        case_location("location-near", "Uzaklıkta, 7 m: sokağa tam 7 m'deki ağaç sayılır",
                      {"input": lay("agac"), "relation": "near", "reference": lay("yol"), "distance": 7}, inputs=trees, refs=roads, relation="near", within=7),
        case_location("location-center", "Merkezi içinde: merkezi iki parselin sınırında olan yapı da, deliğin içindeki değil",
                      {"input": lay("yapi"), "relation": "centerIn", "reference": lay("parsel")}, inputs=buildings, refs=parcels, relation="centerIn"),
        case_location("location-remove", "Seçimden çıkar: seçili üç parselden yolla kesişen çıkar",
                      {"input": lay("parsel"), "relation": "intersects", "reference": lay("yol"), "mode": "remove"}, selection=[1, 2, 3],
                      inputs=parcels, refs=roads, mode="remove"),
        {
            "id": "refuse-location-reference",
            "title": "Başvuru seçili nesnelerken seçim boşsa çalışmaz",
            "document": "queries.kcad",
            "run": {"tool": "selection.byLocation"},
            "values": {"input": lay("parsel"), "relation": "intersects", "reference": {"scope": "selection"}},
            "expect": {"status": "invalid", "issues": [{"param": "reference", "message": "“Başvuru nesneleri”: seçili nesneler arasında uygun nesne yok. Önce nesneleri seçin ya da kapsamı değiştirin."}]},
        },
        case_inside("inside-count", "İçindekinden bilgi al, Sayı: parsellerin ağaçları; sınırdaki ağaç iki parselde, delikteki hiçbirinde",
                    {"target": lay("parsel"), "source": lay("agac"), "relation": "within", "stat": "count", "output": "Ağaç sayısı"},
                    parcels, trees, "within", "count", None, "Ağaç sayısı"),
        case_inside("inside-sum", "Toplam, Kesişen: yapıların taban alanları (virgüllü değer okunur, sayı olmayan atlanır); alanı ondalık 2 basamak",
                    {"target": lay("parsel"), "source": lay("yapi"), "relation": "intersects", "stat": "sum", "field": "Taban alanı", "output": "Taban toplamı"},
                    parcels, buildings, "intersects", "sum", "Taban alanı", "Taban toplamı"),
        case_inside("inside-mean", "Ortalama, Merkezi içinde: alan olmayan yazılacak alanda girdilerin basamağı + 2",
                    {"target": lay("parsel"), "source": lay("yapi"), "relation": "centerIn", "stat": "mean", "field": "Taban alanı", "output": "Ortalama taban"},
                    parcels, buildings, "centerIn", "mean", "Taban alanı", "Ortalama taban"),
        case_inside("inside-mean-field", "Ortalama ondalık 1 basamaklı alana yarım çifte yuvarlanır (62,75 → 62,8)",
                    {"target": lay("parsel"), "source": lay("yapi"), "relation": "intersects", "stat": "mean", "field": "Taban alanı", "output": "Taban ortalaması"},
                    parcels, buildings, "intersects", "mean", "Taban alanı", "Taban ortalaması"),
        case_inside("inside-first", "İlk değer: ağaçların türü, çizim sırasıyla ilki",
                    {"target": lay("parsel"), "source": lay("agac"), "relation": "within", "stat": "first", "field": "Tür", "output": "İlk ağaç"},
                    parcels, trees, "within", "first", "Tür", "İlk ağaç"),
        case_enclosing("enclosing-parcel", "Çevreleyenden bilgi al: yapıların parsel numarası; sınırdaki yapı ilk parselden, delikteki değişmez",
                       {"target": lay("yapi"), "source": lay("parsel"), "field": "Parsel", "output": ""},
                       buildings, parcels, "Parsel", "Parsel"),
        case_summary("summary-grouped", "Özet istatistik, Kat'a göre: taban alanları; boş grup sonda, Toplam satırı",
                     {"input": lay("yapi"), "field": "Taban alanı", "group": "Kat"}, buildings, "Taban alanı", "Kat"),
        case_summary("summary-plain", "Özet istatistik, gruplamasız: parsellerin tapu alanı",
                     {"input": lay("parsel"), "field": "Tapu alanı", "group": ""}, parcels, "Tapu alanı", ""),
        case_join("join-layer", "Anahtarla birleştir, katmandan: tapu kayıtlarının maliki Parsel'le; 002 ile 2 aynı anahtar",
                  {"target": lay("parsel"), "targetKey": "Parsel", "sourceKind": "layer", "source": lay("kayit"), "sourceKey": "", "fields": "Malik"},
                  parcels, "Parsel", layer_rows("kayit", "Parsel", ["Malik"]), "Parsel", ["Malik"], "", False),
        case_join("join-file", "Anahtarla birleştir, CSV'den: önekle bütün sütunlar; 03 ile 3 aynı anahtar",
                  {"target": lay("parsel"), "targetKey": "Parsel", "sourceKind": "file", "file": "malikler.csv", "sourceKey": "", "fields": "", "prefix": "Tapu "},
                  parcels, "Parsel", CSV_ROWS, "Parsel", [], "Tapu ", False),
        {
            "id": "refuse-join-column",
            "title": "Dosyada kaynak anahtar sütunu yoksa çalışmaz, sütunları söyler",
            "document": "queries.kcad",
            "run": {"tool": "attributes.joinByField"},
            "values": {"target": lay("parsel"), "targetKey": "Parsel", "sourceKind": "file", "file": "malikler.csv", "sourceKey": "Ada", "fields": ""},
            "expect": {"status": "error", "message": "Kaynakta “Ada” alanı yok; alanları: Parsel, Malik, Hisse."},
        },
        *world_cases(),
        {
            "id": "refuse-field-rule",
            "title": "Öznitelik hesapla, katmanın tam sayı alanına sayı olmayan değer: çalıştırma reddedilir, hiçbir şey değişmez",
            "document": "queries.kcad",
            "run": {"tool": "attributes.calculate"},
            "values": {"input": lay("parsel"), "field": "Ağaç sayısı", "value": "'çok'", "where": "", "empty": "keep", "label": False},
            "expect": {"status": "error", "message": "Öznitelik yazılamadı (#1): “Ağaç sayısı” alanı tam sayı ister; “çok” verildi. Rakamlarla, ondalıksız yazın."},
        },
    ]
    return {
        "format": "kentos.processing-cases",
        "version": 1,
        "tolerance": 1e-9,
        "documents": {},
        "files": {"malikler.csv": "malikler.csv"},
        "cases": cases,
    }


def main():
    core = json.dumps(build_core(), ensure_ascii=False, indent=1) + "\n"
    proc = json.dumps(build_processing(), ensure_ascii=False, indent=1) + "\n"
    doc = json.dumps(DOCUMENT, ensure_ascii=False, indent=1) + "\n"
    outs = [(CORE_OUT, core), (PROC_DIR / "queries.json", proc), (PROC_DIR / "queries.kcad", doc), (PROC_DIR / "malikler.csv", CSV_TEXT)]
    if "--check" in sys.argv[1:]:
        bad = [p for p, t in outs if not p.exists() or p.read_text() != t]
        for p in bad:
            print(f"{p.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
        if bad:
            return 1
        print("sorgu durumları tutarlı: fixtures/spatial-query/v1/cases.json, fixtures/processing/v1/queries.*")
        return 0
    for p, t in outs:
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(t)
        print(f"yazıldı: {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
