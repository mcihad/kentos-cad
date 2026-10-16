#!/usr/bin/env python3
"""Places on objects and how they follow edits (docs/adr/0249), written from the ADR without KentOS code.

The ADR's rules are written here again:
- a place's point: a point's node (part 0 its own, k its k-th extra point), a line's start, end and middle, a circle's,
  arc's or ellipse's centre, an arc's or elliptical arc's start and end, a path's vertex (part, ring, index) and the
  middle of its edge (the chord's middle, or the arc's when the edge bulges: the chord's middle moved by half the
  bulge times the chord turned clockwise);
- following an edit: the object gone breaks (object_gone); another family breaks (kind_changed; polyline and polygon
  are one family); a place the object did not have breaks (no_such_place); with the same structure (a path's place:
  its part and ring have as many vertices and close as before; a node: as many points; any other place, as a line's
  middle or an arc's start, has no structure) the place stays and its point is
  read from the new object; else a vertex (or node) is looked for by its exact coordinates among the new object's
  vertices: once rebinds, never breaks (vertex_gone), more than once breaks (ambiguous); an edge's middle needs its two
  ends found once each as consecutive vertices of one ring, else vertex_gone (ambiguous when an end is).

Writes fixtures/subref/v1/cases.json; `--check` says whether the file is what this script writes. Vertices and
chord middles of dyadic coordinates are exact; trigonometric points (arcs, ellipses) are compared within 1e-9.
"""

import argparse
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/subref/v1/cases.json"


def v(x, y):
    return {"x": x, "y": y}


def pts(*xy):
    return [v(x, y) for x, y in xy]


# ---- the rules -------------------------------------------------------------------------------------------------

def family(e):
    k = e["kind"]
    return {"point": 0, "line": 1, "polyline": 2, "polygon": 2, "circle": 3, "arc": 4, "ellipse": 5}.get(k, 6)


def ring(e, part, r):
    """(points, bulges, closed) of a path's ring, or None."""
    if e["kind"] not in ("polyline", "polygon"):
        return None
    closed = e["kind"] == "polygon"
    if part == 0:
        p, b, holes = e["pts"], e.get("bulges"), e.get("holes")
    else:
        parts = e.get("parts") or []
        if part - 1 >= len(parts):
            return None
        q = parts[part - 1]
        p, b, holes = q["pts"], q.get("bulges"), q.get("holes")
    if r == 0:
        return p, b, closed
    holes = holes or []
    if r - 1 >= len(holes):
        return None
    h = holes[r - 1]
    return h["pts"], h.get("bulges"), True


def bulge(b, i):
    return b[i] if b and i < len(b) else 0.0


def edge_mid(a, b, k):
    return v((a["x"] + b["x"]) / 2 + (b["y"] - a["y"]) * k / 2, (a["y"] + b["y"]) / 2 - (b["x"] - a["x"]) * k / 2)


def point(e, pl):
    k, kind = e["kind"], pl["kind"]
    if kind == "along":
        return along_point(e, pl)
    # A text's, an insert's and a table's insertion point (docs/adr/0272 §1).
    if kind == "node" and k in ("text", "insert", "table"):
        return e["p"] if pl["part"] == 0 else None
    if kind == "node" and k == "point":
        if pl["part"] == 0:
            return e["p"]
        parts = e.get("parts") or []
        return parts[pl["part"] - 1]["p"] if pl["part"] - 1 < len(parts) else None
    if k == "line":
        if kind == "start":
            return e["a"]
        if kind == "end":
            return e["b"]
        if kind == "mid" and (pl["part"], pl["ring"], pl["index"]) == (0, 0, 0):
            return v((e["a"]["x"] + e["b"]["x"]) / 2, (e["a"]["y"] + e["b"]["y"]) / 2)
        return None
    if kind == "center" and k in ("circle", "arc", "ellipse"):
        return e["c"]
    if k == "arc" and kind in ("start", "end"):
        t = e["a0"] if kind == "start" else e["a1"]
        return v(e["c"]["x"] + e["r"] * math.cos(t), e["c"]["y"] + e["r"] * math.sin(t))
    if k == "ellipse" and kind in ("start", "end"):
        t = e["t0"] if kind == "start" else e["t1"]
        mj = e["major"]
        mn = (-mj["y"] * e["ratio"], mj["x"] * e["ratio"])
        return v(e["c"]["x"] + mj["x"] * math.cos(t) + mn[0] * math.sin(t),
                 e["c"]["y"] + mj["y"] * math.cos(t) + mn[1] * math.sin(t))
    if kind == "vertex":
        r = ring(e, pl["part"], pl["ring"])
        return r[0][pl["index"]] if r and pl["index"] < len(r[0]) else None
    if kind == "mid":
        r = ring(e, pl["part"], pl["ring"])
        if not r:
            return None
        p, b, closed = r
        n, i = len(p), pl["index"]
        if i >= n or (not closed and i + 1 >= n):
            return None
        return edge_mid(p[i], p[(i + 1) % n], bulge(b, i))
    return None


# ---- places along an edge or a curve (docs/adr/0263 §1) ----------------------------------------------------------

def lerp(a, b, t):
    return v(a["x"] + (b["x"] - a["x"]) * t, a["y"] + (b["y"] - a["y"]) * t)


def bulge_circle(a, b, k):
    """(centre, radius, start angle, signed sweep) of a bulged edge: centre on the chord's left normal at
    chord·(1−k²)/(4k), radius chord·(1+k²)/(4|k|), sweep 4·atan(k)."""
    dx, dy = b["x"] - a["x"], b["y"] - a["y"]
    chord = math.hypot(dx, dy)
    if abs(k) <= 1e-12 or chord < 1e-12:
        return None
    s = (1 - k * k) / (4 * k)
    c = v((a["x"] + b["x"]) / 2 - dy * s, (a["y"] + b["y"]) / 2 + dx * s)
    r = chord * (1 + k * k) / (4 * abs(k))
    return c, r, math.atan2(a["y"] - c["y"], a["x"] - c["x"]), 4 * math.atan(k)


def on_circle(c, r, a):
    return v(c["x"] + math.cos(a) * r, c["y"] + math.sin(a) * r)


def ccw(a0, a1):
    """The counter-clockwise sweep from a0 to a1 in (0, 2π]."""
    s = (a1 - a0) % (2 * math.pi)
    return 2 * math.pi if s < 1e-12 else s


def edge_of(e, part, r, i):
    """(a, b, bulge) of a path's edge from vertex i, or None."""
    rg = ring(e, part, r)
    if not rg:
        return None
    p, b, closed = rg
    n = len(p)
    if i >= n or (not closed and i + 1 >= n):
        return None
    return p[i], p[(i + 1) % n], bulge(b, i)


def ellipse_at(e, t):
    mj = e["major"]
    mn = (-mj["y"] * e["ratio"], mj["x"] * e["ratio"])
    return v(e["c"]["x"] + mj["x"] * math.cos(t) + mn[0] * math.sin(t),
             e["c"]["y"] + mj["y"] * math.cos(t) + mn[1] * math.sin(t))


def along_point(e, pl):
    t = pl["t"]
    if not (0 <= t <= 1):
        return None
    k = e["kind"]
    whole = (pl["part"], pl["ring"], pl["index"]) == (0, 0, 0)
    if k == "line":
        return lerp(e["a"], e["b"], t) if whole else None
    if k == "arc":
        return on_circle(e["c"], e["r"], e["a0"] + t * ccw(e["a0"], e["a1"])) if whole else None
    if k == "circle":
        return on_circle(e["c"], e["r"], t * 2 * math.pi) if whole else None
    if k == "ellipse":
        return ellipse_at(e, e["t0"] + t * ccw(e["t0"], e["t1"])) if whole else None
    edge = edge_of(e, pl["part"], pl["ring"], pl["index"])
    if not edge:
        return None
    a, b, kb = edge
    arc = bulge_circle(a, b, kb)
    if not arc:
        return lerp(a, b, t)
    c, r, a0, sw = arc
    return on_circle(c, r, a0 + t * sw)


def tol(at):
    return max(1e-9, 1e-14 * max(abs(at["x"]), abs(at["y"])))


def clamp01(t):
    return min(1.0, max(0.0, t))


def t_on_segment(a, b, at):
    dx, dy = b["x"] - a["x"], b["y"] - a["y"]
    ll = dx * dx + dy * dy
    if ll == 0:
        return None
    t = ((at["x"] - a["x"]) * dx + (at["y"] - a["y"]) * dy) / ll
    q = lerp(a, b, clamp01(t))
    if math.hypot(q["x"] - at["x"], q["y"] - at["y"]) > tol(at):
        return None
    return clamp01(t)


def t_on_arc(c, r, a0, sw, at):
    """The fraction of a signed sweep from a0 at which `at` lies on the circle, or None off it."""
    if abs(math.hypot(at["x"] - c["x"], at["y"] - c["y"]) - r) > tol(at):
        return None
    th = math.atan2(at["y"] - c["y"], at["x"] - c["x"])
    d = (th - a0) % (2 * math.pi) if sw > 0 else (a0 - th) % (2 * math.pi)
    span = abs(sw)
    if d > span + 1e-9:
        if d >= 2 * math.pi - 1e-9:
            return 0.0
        return None
    return clamp01(d / span)


def along_at(e, at):
    """The first edge or curve of `e` that `at` lies on, as an along place (docs/adr/0263 §1)."""
    k = e["kind"]
    z = {"kind": "along", "part": 0, "ring": 0, "index": 0}
    if k == "line":
        t = t_on_segment(e["a"], e["b"], at)
        return None if t is None else dict(z, t=t)
    if k == "arc":
        t = t_on_arc(e["c"], e["r"], e["a0"], ccw(e["a0"], e["a1"]), at)
        return None if t is None else dict(z, t=t)
    if k == "circle":
        t = t_on_arc(e["c"], e["r"], 0.0, 2 * math.pi, at)
        return None if t is None else dict(z, t=t % 1.0)
    if k == "ellipse":
        mj = e["major"]
        mn = (-mj["y"] * e["ratio"], mj["x"] * e["ratio"])
        det = mj["x"] * mn[1] - mj["y"] * mn[0]
        dx, dy = at["x"] - e["c"]["x"], at["y"] - e["c"]["y"]
        ux, uy = (mn[1] * dx - mn[0] * dy) / det, (-mj["y"] * dx + mj["x"] * dy) / det
        p = math.atan2(uy, ux) % (2 * math.pi)
        q = ellipse_at(e, p)
        if math.hypot(q["x"] - at["x"], q["y"] - at["y"]) > tol(at):
            return None
        sw = ccw(e["t0"], e["t1"])
        d = (p - e["t0"]) % (2 * math.pi)
        if d > sw + 1e-9:
            return dict(z, t=0.0) if d >= 2 * math.pi - 1e-9 else None
        return dict(z, t=clamp01(d / sw))
    for pl, _ in vertices(e):
        if pl["kind"] != "vertex":
            continue
        edge = edge_of(e, pl["part"], pl["ring"], pl["index"])
        if not edge:
            continue
        a, b, kb = edge
        arc = bulge_circle(a, b, kb)
        t = t_on_segment(a, b, at) if not arc else t_on_arc(*arc, at)
        if t is not None:
            return {"kind": "along", "part": pl["part"], "ring": pl["ring"], "index": pl["index"], "t": t}
    return None


# ---- ranges (docs/adr/0263 §2) -----------------------------------------------------------------------------------

def edge_length(a, b, kb, t0, t1):
    arc = bulge_circle(a, b, kb)
    if not arc:
        return math.hypot(b["x"] - a["x"], b["y"] - a["y"]) * (t1 - t0)
    _, r, _, sw = arc
    return r * abs(sw) * (t1 - t0)


def ellipse_piece(e, s0, s1):
    """The length of an ellipse's curve from parameter s0 to s1: ∫|C′(t)|dt by adaptive Simpson to 1e-13 (a different
    rule from the core's Gauss–Legendre)."""
    mj = e["major"]
    mn = (-mj["y"] * e["ratio"], mj["x"] * e["ratio"])

    def speed(t):
        return math.hypot(-mj["x"] * math.sin(t) + mn[0] * math.cos(t), -mj["y"] * math.sin(t) + mn[1] * math.cos(t))

    def simpson(a, b, fa, fm, fb):
        return (b - a) / 6 * (fa + 4 * fm + fb)

    def go(a, b, fa, fm, fb, whole, depth):
        m = (a + b) / 2
        lm, rm = speed((a + m) / 2), speed((m + b) / 2)
        left, right = simpson(a, m, fa, lm, fm), simpson(m, b, fm, rm, fb)
        if depth > 40 or abs(left + right - whole) <= 1e-14 * 15:
            return left + right + (left + right - whole) / 15
        return go(a, m, fa, lm, fm, left, depth + 1) + go(m, b, fm, rm, fb, right, depth + 1)

    fa, fm, fb = speed(s0), speed((s0 + s1) / 2), speed(s1)
    return go(s0, s1, fa, fm, fb, simpson(s0, s1, fa, fm, fb), 0)


def range_length(e, a, b):
    """The length from along place a to along place b in the object's direction, or None."""
    if a["kind"] != "along" or b["kind"] != "along" or point(e, a) is None or point(e, b) is None:
        return None
    if (a["part"], a["ring"]) != (b["part"], b["ring"]):
        return None
    k = e["kind"]
    ka, kb = (a["index"], a["t"]), (b["index"], b["t"])
    if k in ("line", "arc", "ellipse") and not (k == "ellipse" and ccw(e["t0"], e["t1"]) >= 2 * math.pi):
        if kb < ka:
            return None
        if k == "line":
            return math.hypot(e["b"]["x"] - e["a"]["x"], e["b"]["y"] - e["a"]["y"]) * (b["t"] - a["t"])
        if k == "arc":
            return e["r"] * ccw(e["a0"], e["a1"]) * (b["t"] - a["t"])
        sw = ccw(e["t0"], e["t1"])
        return ellipse_piece(e, e["t0"] + a["t"] * sw, e["t0"] + b["t"] * sw)
    if k == "circle":
        d = b["t"] - a["t"]
        return e["r"] * 2 * math.pi * (d if d >= 0 else d + 1)
    if k == "ellipse":
        d = b["t"] - a["t"]
        s0 = e["t0"] + a["t"] * 2 * math.pi
        return ellipse_piece(e, s0, s0 + (d if d >= 0 else d + 1) * 2 * math.pi)
    p, bl, closed = ring(e, a["part"], a["ring"])
    n = len(p)
    edges = n if closed else n - 1
    if kb < ka and not closed:
        return None
    total = 0.0
    i, t = ka
    # Whole edges and pieces from a to b, around the ring when b comes before a.
    wrapped = kb < ka
    while True:
        last = (i == kb[0]) and (not wrapped)
        x0, x1 = p[i], p[(i + 1) % n]
        end = kb[1] if last else 1.0
        total += edge_length(x0, x1, bulge(bl, i), t, end)
        if last:
            return total
        i, t = (i + 1) % edges, 0.0
        if i == 0 and wrapped:
            wrapped = False


def follow_range(a, b, before, after):
    fa, fb = follow(a, before, after), follow(b, before, after)
    if "broken" in fa:
        return fa
    if "broken" in fb:
        return fb
    if (fa["at"]["part"], fa["at"]["ring"]) != (fb["at"]["part"], fb["at"]["ring"]):
        return {"broken": "vertex_gone"}
    return {"from": fa["at"], "to": fb["at"]}


def vertices(e):
    out = []
    if e["kind"] == "point":
        out.append(({"kind": "node", "part": 0}, e["p"]))
        for k, q in enumerate(e.get("parts") or []):
            out.append(({"kind": "node", "part": k + 1}, q["p"]))
        return out
    if e["kind"] not in ("polyline", "polygon"):
        return out
    for part in range(1 + len(e.get("parts") or [])):
        r = 0
        while (rg := ring(e, part, r)) is not None:
            for i, p in enumerate(rg[0]):
                out.append(({"kind": "vertex", "part": part, "ring": r, "index": i}, p))
            r += 1
    return out


def same_structure(b, a, pl):
    # Only a path's rings and a multi-point's points have a structure; a line's middle has none.
    if pl["kind"] in ("vertex", "mid", "along") and b["kind"] in ("polyline", "polygon"):
        rb, ra = ring(b, pl["part"], pl["ring"]), ring(a, pl["part"], pl["ring"])
        return bool(rb and ra and len(rb[0]) == len(ra[0]) and rb[2] == ra[2])
    if pl["kind"] == "node":
        def count(e):
            return 1 + len(e.get("parts") or []) if e["kind"] == "point" else 0
        return count(b) == count(a)
    return True


def found(e, want):
    hits = [pl for pl, p in vertices(e) if p["x"] == want["x"] and p["y"] == want["y"]]
    if len(hits) == 1:
        return hits[0], None
    return None, ("vertex_gone" if not hits else "ambiguous")


def follow(pl, before, after):
    if after is None:
        return {"broken": "object_gone"}
    if family(before) != family(after):
        return {"broken": "kind_changed"}
    was = point(before, pl)
    if was is None:
        return {"broken": "no_such_place"}
    if same_structure(before, after, pl):
        p = point(after, pl)
        return {"at": pl, "point": p} if p is not None else {"broken": "no_such_place"}
    if pl["kind"] in ("vertex", "node"):
        to, why = found(after, was)
        return {"at": to, "point": was} if to else {"broken": why}
    if pl["kind"] in ("mid", "along"):
        p, _, _ = ring(before, pl["part"], pl["ring"])
        n = len(p)
        a, b = p[pl["index"]], p[(pl["index"] + 1) % n]
        (ta, wa), (tb, wb) = found(after, a), found(after, b)
        if not ta or not tb:
            return {"broken": "ambiguous" if "ambiguous" in (wa, wb) else "vertex_gone"}
        if (ta["part"], ta["ring"]) != (tb["part"], tb["ring"]):
            return {"broken": "vertex_gone"}
        np_, _, closed = ring(after, ta["part"], ta["ring"])
        m = len(np_)
        nxt = ta["index"] + 1 if ta["index"] + 1 < m else (0 if closed else None)
        if nxt != tb["index"]:
            return {"broken": "vertex_gone"}
        to = {"kind": pl["kind"], "part": ta["part"], "ring": ta["ring"], "index": ta["index"]}
        if pl["kind"] == "along":
            to["t"] = pl["t"]
        return {"at": to, "point": point(after, to)}
    p = point(after, pl)
    return {"at": pl, "point": p} if p is not None else {"broken": "no_such_place"}


def place_at(e, snap, at):
    """The place a snap of `snap` stood on at `at`: the first candidate (the object's order) within a nanometre."""
    cands = []
    if snap == "endpoint":
        cands = [place("start"), place("end")] + [pl for pl, _ in vertices(e)]
    elif snap == "midpoint":
        cands = [place("mid", 0, 0, 0)] + [place("mid", pl["part"], pl["ring"], pl["index"])
                                          for pl, _ in vertices(e) if pl["kind"] == "vertex"]
    elif snap == "center":
        cands = [place("center")]
    elif snap == "node":
        cands = [pl for pl, _ in vertices(e)]
    elif snap == "insertion":
        cands = [{"kind": "node", "part": 0}]
    elif snap in ("nearest", "perpendicular", "tangent", "quadrant"):
        return along_at(e, at)
    for pl in cands:
        p = point(e, pl)
        if p is not None and abs(p["x"] - at["x"]) <= 1e-9 and abs(p["y"] - at["y"]) <= 1e-9:
            return pl
    return None


# ---- pieces inside blocks (docs/adr/0263 §3) ---------------------------------------------------------------------

MAX_DEPTH = 16


def placement(ins):
    """x' = p + scale·R(rotation)·M·x, M mirroring in the x axis: (a, b, c, d, e, f) with x' = a·x + c·y + e and
    y' = b·x + d·y + f."""
    s, r, k = ins.get("scale", 1.0), ins.get("rotation", 0.0), -1.0 if ins.get("mirror") else 1.0
    co, si = math.cos(r), math.sin(r)
    return (s * co, s * si, -s * k * si, s * k * co, ins["p"]["x"], ins["p"]["y"])


def compose(m, n):
    """m after n."""
    a, b, c, d, e, f = m
    A, B, C, D, E, F = n
    return (a * A + c * B, b * A + d * B, a * C + c * D, b * C + d * D, a * E + c * F + e, b * E + d * F + f)


def shift(dx, dy):
    return (1.0, 0.0, 0.0, 1.0, dx, dy)


def apply(m, q):
    a, b, c, d, e, f = m
    return v(a * q["x"] + c * q["y"] + e, b * q["x"] + d * q["y"] + f)


def placed(e, m):
    out = json.loads(json.dumps(e))
    det = m[0] * m[3] - m[1] * m[2]
    for key in ("a", "b", "c", "p"):
        if key in out:
            out[key] = apply(m, out[key])
    if "pts" in out:
        out["pts"] = [apply(m, q) for q in out["pts"]]
        if det < 0 and out.get("bulges"):
            out["bulges"] = [-x for x in out["bulges"]]
    if "r" in out:
        out["r"] = out["r"] * math.sqrt(abs(det))
    return out


def piece(defs, ins, path):
    """The object a block path names in an insert, placed in the drawing; None when there is none."""
    if not path or len(path) > MAX_DEPTH:
        return None
    m, cur = (1.0, 0.0, 0.0, 1.0, 0.0, 0.0), ins
    for depth, i in enumerate(path):
        d = defs.get(cur["block"])
        if d is None:
            return None
        m = compose(m, compose(placement(cur), shift(-d["base"]["x"], -d["base"]["y"])))
        if i >= len(d["entities"]):
            return None
        e = d["entities"][i]
        if depth < len(path) - 1:
            if e["kind"] != "insert":
                return None
            cur = e
        else:
            return None if e["kind"] == "insert" else placed(e, m)
    return None


def paths(defs, block, depth=0):
    """Every piece's path of a definition, in the definition's order (a nested insert's pieces where it stands)."""
    d = defs.get(block)
    if d is None or depth >= MAX_DEPTH:
        return []
    out = []
    for i, e in enumerate(d["entities"]):
        if e["kind"] == "insert":
            out.extend([i] + q for q in paths(defs, e["block"], depth + 1))
        else:
            out.append([i])
    return out


def path_in(defs_b, ins_b, defs_a, ins_a, path):
    """A block path made a path through the insert after (ADR 0315 §1): each step's object found again by its local
    id when that id is once in both definitions, else kept at its place."""
    def once(entities, i):
        found = [k for k, e in enumerate(entities) if e.get("id") == i]
        return found[0] if len(found) == 1 else None
    out, was, now = [], ins_b, ins_a
    for i in path:
        old, new = defs_b.get(was.get("block")), defs_a.get(now.get("block"))
        if old is None or new is None or i >= len(old["entities"]):
            return None
        e = old["entities"][i]
        j = i
        if "id" in e and once(old["entities"], e["id"]) == i:
            k = once(new["entities"], e["id"])
            j = i if k is None else k
        if j >= len(new["entities"]):
            return None
        out.append(j)
        was, now = e, new["entities"][j]
    return out


def follow_in(pl, path, defs_b, ins_b, defs_a, ins_a):
    if ins_a is None:
        return {"broken": "object_gone"}
    if ins_a["kind"] != "insert":
        return {"broken": "kind_changed"}
    pb = piece(defs_b, ins_b, path)
    if pb is None:
        return {"broken": "no_such_place"}
    moved = path_in(defs_b, ins_b, defs_a, ins_a, path) or path
    pa = piece(defs_a, ins_a, moved)
    if pa is None:
        return {"broken": "piece_gone"}
    out = follow(pl, pb, pa)
    if moved != path and "broken" not in out:
        out["path"] = moved
    return out


def place_in(defs, ins, snap, at):
    for path in paths(defs, ins["block"]):
        e = piece(defs, ins, path)
        if e is None:
            continue
        pl = place_at(e, snap, at)
        if pl is not None:
            return {"path": path, "place": pl}
    return None


# ---- the sub-object a click falls on (docs/adr/0276) ---------------------------------------------------------

def closest_seg(a, b, at):
    dx, dy = b["x"] - a["x"], b["y"] - a["y"]
    ll = dx * dx + dy * dy
    t = 0.0 if ll == 0 else clamp01(((at["x"] - a["x"]) * dx + (at["y"] - a["y"]) * dy) / ll)
    q = lerp(a, b, t)
    return q, math.hypot(q["x"] - at["x"], q["y"] - at["y"])


def closest_arc(c, r, a0, sw, at):
    """The nearest point of an arc from a0 sweeping `sw` (signed): the foot on the circle when it lies on the arc,
    else the nearer end."""
    th = math.atan2(at["y"] - c["y"], at["x"] - c["x"])
    d = (th - a0) % (2 * math.pi) if sw > 0 else (a0 - th) % (2 * math.pi)
    if d <= abs(sw):
        q = on_circle(c, r, th)
        return q, abs(math.hypot(at["x"] - c["x"], at["y"] - c["y"]) - r)
    ends = [on_circle(c, r, a0), on_circle(c, r, a0 + sw)]
    return min(((q, math.hypot(q["x"] - at["x"], q["y"] - at["y"])) for q in ends), key=lambda x: x[1])


def edges_near(e, at):
    """Every edge's and curve's nearest point to `at`, with the distance."""
    k = e["kind"]
    if k == "line":
        return [closest_seg(e["a"], e["b"], at)]
    if k == "arc":
        return [closest_arc(e["c"], e["r"], e["a0"], ccw(e["a0"], e["a1"]), at)]
    if k == "circle":
        return [closest_arc(e["c"], e["r"], 0.0, 2 * math.pi, at)]
    out = []
    for pl, _ in vertices(e):
        if pl["kind"] != "vertex":
            continue
        edge = edge_of(e, pl["part"], pl["ring"], pl["index"])
        if not edge:
            continue
        a, b, kb = edge
        arc = bulge_circle(a, b, kb)
        out.append(closest_seg(a, b, at) if not arc else closest_arc(*arc, at))
    return out


def point_places(e):
    """An object's points a click takes before its edges: its ends, its own point, its vertices."""
    out = []
    for pl in (place("start"), place("end"), {"kind": "node", "part": 0}):
        q = point(e, pl)
        if q is not None:
            out.append((pl, q))
    return out + vertices(e)


def near_on(e, at, tl):
    best = None
    for pl, q in point_places(e):
        d = math.hypot(q["x"] - at["x"], q["y"] - at["y"])
        if d <= tl and (best is None or d < best[1]):
            best = (pl, d)
    edge = None
    for q, d in edges_near(e, at):
        if d <= tl and (edge is None or d < edge[1]):
            edge = (q, d)
    return best, edge


def place_near(e, at, tl):
    """The sub-object a click at `at` falls on within `tl`: the nearest of its points, else the nearest point of its
    edges as an along place; None when nothing is within `tl`."""
    best, edge = near_on(e, at, tl)
    if best:
        return best[0]
    return along_at(e, edge[0]) if edge else None


def place_near_in(defs, ins, at, tl):
    """`place_near` inside an insert: its own point and the pieces' points, then the pieces' edges; the piece's path."""
    own, _ = near_on(ins, at, tl)
    pbest = ([], own[0], own[1]) if own and own[0] == {"kind": "node", "part": 0} else None
    ebest = None
    for path in paths(defs, ins["block"]):
        e = piece(defs, ins, path)
        if e is None:
            continue
        b, ed = near_on(e, at, tl)
        if b and (pbest is None or b[1] < pbest[2]):
            pbest = (path, b[0], b[1])
        if ed and (ebest is None or ed[1] < ebest[3]):
            ebest = (path, e, ed[0], ed[1])
    if pbest:
        return {"path": pbest[0], "place": pbest[1]}
    if ebest:
        pl = along_at(ebest[1], ebest[2])
        return None if pl is None else {"path": ebest[0], "place": pl}
    return None


# ---- the cases -------------------------------------------------------------------------------------------------

def place(kind, part=None, ring_=None, index=None):
    d = {"kind": kind}
    if kind == "node":
        d["part"] = part
    if kind in ("vertex", "mid"):
        d.update({"part": part, "ring": ring_, "index": index})
    return d


LINE = {"kind": "line", "a": v(0, 0), "b": v(4, 0)}
ROOM = {"kind": "polygon", "pts": pts((0, 0), (4, 0), (4, 3), (0, 3))}
WALL = {"kind": "polyline", "pts": pts((0, 0), (2, 0), (2, 2), (5, 2))}
ARCED = {"kind": "polyline", "pts": pts((0, 0), (2, 0), (4, 0)), "bulges": [0.5, 0.0]}
HOLED = {"kind": "polygon", "pts": pts((0, 0), (10, 0), (10, 10), (0, 10)),
         "holes": [{"pts": pts((2, 2), (2, 4), (4, 4), (4, 2))}]}
PARTS = {"kind": "polygon", "pts": pts((0, 0), (2, 0), (2, 2), (0, 2)),
         "parts": [{"pts": pts((5, 0), (7, 0), (7, 2), (5, 2))}]}
CIRCLE = {"kind": "circle", "c": v(1, 1), "r": 2}
ARC = {"kind": "arc", "c": v(0, 0), "r": 2, "a0": 0.0, "a1": math.pi / 2}
ELLIPSE = {"kind": "ellipse", "c": v(1, 2), "major": v(3, 0), "ratio": 0.5, "t0": 0.25, "t1": 2.5}
POINTS = {"kind": "point", "p": v(1, 1), "parts": [{"p": v(2, 2)}, {"p": v(3, 3)}]}


def moved(e, dx, dy):
    def m(p):
        return v(p["x"] + dx, p["y"] + dy)
    out = json.loads(json.dumps(e))
    for k in ("a", "b", "c", "p"):
        if k in out:
            out[k] = m(out[k])
    if "pts" in out:
        out["pts"] = [m(p) for p in out["pts"]]
    for h in out.get("holes") or []:
        h["pts"] = [m(p) for p in h["pts"]]
    for q in out.get("parts") or []:
        if "pts" in q:
            q["pts"] = [m(p) for p in q["pts"]]
            for h in q.get("holes") or []:
                h["pts"] = [m(p) for p in h["pts"]]
        if "p" in q:
            q["p"] = m(q["p"])
    return out


def with_pts(e, *xy, **extra):
    out = json.loads(json.dumps(e))
    out["pts"] = pts(*xy)
    out.update(extra)
    return out


TEXT = {"kind": "text", "p": v(5, 6), "text": "Bina", "height": 1.0}
INSERT = {"kind": "insert", "block": "vana", "p": v(7, 8)}

POINT_CASES = [
    ("Yazının ekleme noktası", TEXT, {"kind": "node", "part": 0}),
    ("Yerleştirmenin ekleme noktası", INSERT, {"kind": "node", "part": 0}),
    ("Yazının ikinci düğümü yok", TEXT, {"kind": "node", "part": 1}),
    ("Çizginin başı", LINE, place("start")), ("Çizginin sonu", LINE, place("end")),
    ("Çizginin ortası", LINE, place("mid", 0, 0, 0)),
    ("Alanın köşesi", ROOM, place("vertex", 0, 0, 2)),
    ("Alanın kapanan kenarının ortası", ROOM, place("mid", 0, 0, 3)),
    ("Açık yolun son kenarı yok", WALL, place("mid", 0, 0, 3)),
    ("Yaylı kenarın ortası", ARCED, place("mid", 0, 0, 0)),
    ("Deliğin köşesi", HOLED, place("vertex", 0, 1, 2)),
    ("İkinci parçanın köşesi", PARTS, place("vertex", 1, 0, 1)),
    ("Olmayan parça", PARTS, place("vertex", 2, 0, 0)),
    ("Dairenin merkezi", CIRCLE, place("center")),
    ("Dairenin başı yok", CIRCLE, place("start")),
    ("Yayın başı", ARC, place("start")), ("Yayın sonu", ARC, place("end")),
    ("Elipsin başı", ELLIPSE, place("start")), ("Elipsin sonu", ELLIPSE, place("end")),
    ("Çok noktalının ikinci noktası", POINTS, place("node", 2)),
]

FOLLOW_CASES = [
    ("Taşınan çizgi: baş izler", place("start"), LINE, moved(LINE, 1.5, 2)),
    ("Taşınan çizgi: orta izler", place("mid", 0, 0, 0), LINE, moved(LINE, 1.5, 2)),
    ("Silinen çizgi", place("end"), LINE, None),
    ("Çizgi yaya döndü", place("start"), LINE, {"kind": "arc", "c": v(2, 0), "r": 2, "a0": math.pi, "a1": 0.0}),
    ("Köşesi tutamaçla çekilen alan: köşe izler", place("vertex", 0, 0, 2), ROOM,
     with_pts(ROOM, (0, 0), (4, 0), (5, 3.5), (0, 3))),
    ("Önüne köşe eklenen köşe yeni sırasına bağlanır", place("vertex", 0, 0, 2), ROOM,
     with_pts(ROOM, (0, 0), (2, -0.5), (4, 0), (4, 3), (0, 3))),
    ("Arkasına köşe eklenen köşe yerinde", place("vertex", 0, 0, 1), ROOM,
     with_pts(ROOM, (0, 0), (4, 0), (4, 3), (2, 3.5), (0, 3))),
    ("Silinen köşe", place("vertex", 0, 0, 2), ROOM, with_pts(ROOM, (0, 0), (4, 0), (0, 3))),
    ("Köşe hem taşındı hem köşe eklendi", place("vertex", 0, 0, 2), ROOM,
     with_pts(ROOM, (0, 0), (2, -0.5), (4, 0), (4.5, 3), (0, 3))),
    ("Kenarına köşe eklenen kenarın ortası kopar", place("mid", 0, 0, 1), ROOM,
     with_pts(ROOM, (0, 0), (4, 0), (4.5, 1.5), (4, 3), (0, 3))),
    ("Başka kenara köşe eklenince kenarın ortası yeni sırasında", place("mid", 0, 0, 2), ROOM,
     with_pts(ROOM, (0, 0), (2, -0.5), (4, 0), (4, 3), (0, 3))),
    ("Çoklu çizgi alana çevrildi: köşe bulunur", place("vertex", 0, 0, 3), WALL, dict(WALL, kind="polygon")),
    ("Çoklu çizgi alana çevrildi: kenar ortası", place("mid", 0, 0, 1), WALL, dict(WALL, kind="polygon")),
    ("Yayın bükümü değişti: kenar ortası izler", place("mid", 0, 0, 0), ARCED, dict(ARCED, bulges=[-0.25, 0.0])),
    ("Delik eklenen alanın dış köşesi yerinde", place("vertex", 0, 0, 2), ROOM,
     dict(ROOM, holes=[{"pts": pts((1, 1), (1, 2), (2, 2), (2, 1))}])),
    ("Deliği silinen alanın delik köşesi kopar", place("vertex", 0, 1, 0), HOLED, dict(HOLED, holes=None)),
    ("Parçaları ayrılan alan: ikinci parçanın köşesi bulunamaz", place("vertex", 1, 0, 2), PARTS,
     dict(PARTS, parts=None)),
    ("Parçası eklenen alanda ortak köşe iki kez: belirsiz", place("vertex", 0, 0, 1),
     with_pts(ROOM, (0, 0), (4, 0), (4, 3)),
     dict(with_pts(ROOM, (0, 0), (4, 0), (4, 3), (0, 3)), parts=[{"pts": pts((4, 0), (6, 0), (6, 2))}])),
    ("Taşınan dairenin merkezi", place("center"), CIRCLE, moved(CIRCLE, -1, 0.5)),
    ("Daire yaya kırıldı", place("center"), CIRCLE, {"kind": "arc", "c": v(1, 1), "r": 2, "a0": 0.0, "a1": 3.0}),
    ("Budanan yayın başı yeni başında", place("start"), ARC, dict(ARC, a0=math.pi / 6)),
    ("Elipsin sonu", place("end"), ELLIPSE, dict(ELLIPSE, t1=3.0)),
    ("Taşınan çok noktalı: düğüm izler", place("node", 1), POINTS, moved(POINTS, 1, 0)),
    ("Noktası silinen çok noktalı: düğüm yeni sırasında", place("node", 2), POINTS,
     dict(POINTS, parts=[{"p": v(3, 3)}])),
    ("Olmayan köşe", place("vertex", 0, 0, 9), ROOM, ROOM),
]

SNAP_CASES = [
    ("Ekleme noktası: yazının", TEXT, "insertion", v(5, 6)),
    ("Ekleme noktası: yerleştirmenin", INSERT, "insertion", v(7, 8)),
    ("Uç nokta: çizginin sonu", LINE, "endpoint", v(4, 0)),
    ("Uç nokta: alanın köşesi", ROOM, "endpoint", v(4, 3)),
    ("Uç nokta: deliğin köşesi", HOLED, "endpoint", v(4, 2)),
    ("Uç nokta: yayın sonu", ARC, "endpoint", v(0.0, 2.0)),
    ("Orta nokta: kapanan kenar", ROOM, "midpoint", v(0, 1.5)),
    ("Orta nokta: çizgi", LINE, "midpoint", v(2, 0)),
    ("Orta nokta: yaylı kenar", ARCED, "midpoint", v(1, -0.5)),
    ("Merkez: daire", CIRCLE, "center", v(1, 1)),
    ("Nokta: çok noktalının ikincisi", POINTS, "node", v(3, 3)),
    ("Yakın: çizginin çeyreği", LINE, "nearest", v(1, 0)),
    ("Uç nokta ama orada köşe yok", ROOM, "endpoint", v(2, 0)),
    ("Yakın: alanın kapanan kenarı", ROOM, "nearest", v(0, 0.75)),
    ("Dik: yolun ikinci kenarı", WALL, "perpendicular", v(2, 0.5)),
    ("Yakın: yaylı kenarın üstü", ARCED, "nearest", v(1, -0.5)),
    ("Yakın: yolun dışında", WALL, "nearest", v(3, 3)),
    ("Çeyrek: dairenin kuzeyi", CIRCLE, "quadrant", v(1, 3)),
    ("Teğet: yayın üstü", ARC, "tangent", v(2 * math.cos(math.pi / 6), 2 * math.sin(math.pi / 6))),
    ("Yakın: elipsin üstü", ELLIPSE, "nearest", ellipse_at(ELLIPSE, 1.0)),
    ("Yakın: deliğin kenarı", HOLED, "nearest", v(3, 4)),
    ("Kesişim: yer değil", LINE, "intersection", v(2, 0)),
]

ALONG = {"kind": "along"}


def along(part, ring_, index, t):
    return {"kind": "along", "part": part, "ring": ring_, "index": index, "t": t}


ALONG_POINT_CASES = [
    ("Çizginin çeyreği", LINE, along(0, 0, 0, 0.25)),
    ("Çizgide olmayan kenar", LINE, along(0, 0, 1, 0.5)),
    ("Alanın kapanan kenarında", ROOM, along(0, 0, 3, 0.5)),
    ("Açık yolun son kenarı yok", WALL, along(0, 0, 3, 0.5)),
    ("Yaylı kenarın yarısı", ARCED, along(0, 0, 0, 0.5)),
    ("Yaylı kenarın çeyreği", ARCED, along(0, 0, 0, 0.25)),
    ("Deliğin kenarı", HOLED, along(0, 1, 1, 0.5)),
    ("İkinci parçanın kenarı", PARTS, along(1, 0, 2, 0.75)),
    ("Yayın üçte biri", ARC, along(0, 0, 0, 1 / 3)),
    ("Dairenin çeyreği", CIRCLE, along(0, 0, 0, 0.25)),
    ("Elipsin yarısı", ELLIPSE, along(0, 0, 0, 0.5)),
    ("Aralık dışı t", LINE, along(0, 0, 0, 1.5)),
]

ALONG_FOLLOW_CASES = [
    ("Taşınan çizgide oran kalır", along(0, 0, 0, 0.25), LINE, moved(LINE, 1.5, 2)),
    ("Uzatılan çizgide oran kalır", along(0, 0, 0, 0.25), LINE, dict(LINE, b=v(8, 0))),
    ("Köşesi çekilen alanın kenarı", along(0, 0, 1, 0.5), ROOM, with_pts(ROOM, (0, 0), (4, 0), (5, 3.5), (0, 3))),
    ("Önüne köşe eklenen kenar yeni sırasında", along(0, 0, 2, 0.25), ROOM,
     with_pts(ROOM, (0, 0), (2, -0.5), (4, 0), (4, 3), (0, 3))),
    ("Bölünen kenar kopar", along(0, 0, 1, 0.5), ROOM, with_pts(ROOM, (0, 0), (4, 0), (4.5, 1.5), (4, 3), (0, 3))),
    ("Budanan yayda oran kalır", along(0, 0, 0, 0.5), ARC, dict(ARC, a0=math.pi / 6)),
    ("Büyüyen dairede oran kalır", along(0, 0, 0, 0.125), CIRCLE, dict(CIRCLE, r=3)),
    ("Silinen yay", along(0, 0, 0, 0.5), ARC, None),
    ("Elips yaya döndü", along(0, 0, 0, 0.5), ELLIPSE, ARC),
]

RANGE_CASES = [
    ("Çizginin ortadaki yarısı", LINE, along(0, 0, 0, 0.25), along(0, 0, 0, 0.75)),
    ("Çizgide ters sıra", LINE, along(0, 0, 0, 0.75), along(0, 0, 0, 0.25)),
    ("Yolun iki kenarından", WALL, along(0, 0, 0, 0.5), along(0, 0, 2, 0.5)),
    ("Kapalı alanda dolanan", ROOM, along(0, 0, 3, 0.5), along(0, 0, 0, 0.5)),
    ("Açık yolda ters sıra", WALL, along(0, 0, 2, 0.5), along(0, 0, 0, 0.5)),
    ("Yaylı kenarla", ARCED, along(0, 0, 0, 0.0), along(0, 0, 1, 0.5)),
    ("Yayın parçası", ARC, along(0, 0, 0, 0.25), along(0, 0, 0, 0.75)),
    ("Dairede dolanan", CIRCLE, along(0, 0, 0, 0.75), along(0, 0, 0, 0.25)),
    ("Elips yayının parçası", ELLIPSE, along(0, 0, 0, 0.0), along(0, 0, 0, 1.0)),
    ("Farklı halkalar", HOLED, along(0, 0, 0, 0.5), along(0, 1, 0, 0.5)),
    ("Köşe yer değil", WALL, place("vertex", 0, 0, 1), along(0, 0, 2, 0.5)),
]

RANGE_FOLLOW_CASES = [
    ("Taşınan yolda aralık izler", WALL, along(0, 0, 0, 0.5), along(0, 0, 2, 0.5), moved(WALL, 1, 1)),
    ("Ucu kopan aralık kopar", ROOM, along(0, 0, 0, 0.5), along(0, 0, 1, 0.5),
     with_pts(ROOM, (0, 0), (4, 0), (4.5, 1.5), (4, 3), (0, 3))),
    ("Silinen nesne", LINE, along(0, 0, 0, 0.25), along(0, 0, 0, 0.75), None),
]

# Blocks: a door (a line and a leaf), a room that holds the door twice, a frame that holds the room.
DOOR = {"id": "kapi", "base": v(0, 0), "entities": [
    {"kind": "line", "a": v(0, 0), "b": v(1, 0)},
    {"kind": "polyline", "pts": pts((0, 0), (0, 1), (1, 1))},
    {"kind": "circle", "c": v(0.5, 0.5), "r": 0.25},
]}
ROOM_BLOCK = {"id": "oda", "base": v(1, 1), "entities": [
    {"kind": "polygon", "pts": pts((1, 1), (5, 1), (5, 4), (1, 4))},
    {"kind": "insert", "block": "kapi", "p": v(2, 1), "scale": 1.0, "rotation": 0.0},
    {"kind": "insert", "block": "kapi", "p": v(5, 2), "scale": 0.5, "rotation": math.pi / 2},
]}
FRAME = {"id": "cerceve", "base": v(0, 0), "entities": [
    {"kind": "line", "a": v(0, 0), "b": v(10, 0)},
    {"kind": "insert", "block": "oda", "p": v(3, 3), "scale": 2.0, "rotation": math.pi / 6, "mirror": True},
]}
DEFS = {d["id"]: d for d in (DOOR, ROOM_BLOCK, FRAME)}
DOOR_WIDE = dict(DOOR, entities=[{"kind": "line", "a": v(0, 0), "b": v(1.5, 0)}] + DOOR["entities"][1:])
DOOR_SHORT = dict(DOOR, entities=DOOR["entities"][:1])
DEFS_WIDE = dict(DEFS, kapi=DOOR_WIDE)
DEFS_SHORT = dict(DEFS, kapi=DOOR_SHORT)
DEFS_REBASED = dict(DEFS, oda=dict(ROOM_BLOCK, base=v(0, 0)))
# The door with its objects' local ids, and the same door after an object was put before them (ADR 0315 §1): the
# ids stay, the places move; and one whose ids are not once each.
DOOR_IDS = dict(DOOR, entities=[dict(e, id=k + 1) for k, e in enumerate(DOOR["entities"])])
DOOR_PUT_BEFORE = dict(DOOR, entities=[{"kind": "line", "id": 9, "a": v(0, -0.2), "b": v(1, -0.2)}]
                       + DOOR_IDS["entities"])
DOOR_TWICE = dict(DOOR, entities=[dict(e, id=1) for e in DOOR["entities"]])
DEFS_IDS = dict(DEFS, kapi=DOOR_IDS)
DEFS_PUT_BEFORE = dict(DEFS, kapi=DOOR_PUT_BEFORE)
DEFS_TWICE = dict(DEFS, kapi=DOOR_TWICE)
DEFS_TWICE_PUT = dict(DEFS, kapi=dict(DOOR, entities=[{"kind": "line", "id": 9, "a": v(0, -0.2), "b": v(1, -0.2)}]
                                       + DOOR_TWICE["entities"]))


def insert(block, x, y, scale=1.0, rotation=0.0, mirror=None):
    d = {"kind": "insert", "block": block, "p": v(x, y), "scale": scale, "rotation": rotation}
    if mirror is not None:
        d["mirror"] = mirror
    return d


ROOM_IN = insert("oda", 20, 10)
FRAME_IN = insert("cerceve", 100, 50, 1.0, math.pi / 2)

PIECE_CASES = [
    ("Odanın alanının köşesi", ROOM_IN, [0], place("vertex", 0, 0, 2)),
    ("Odadaki kapının çizgisinin sonu", ROOM_IN, [1, 0], place("end")),
    ("Dönük ve küçük kapının kanadının köşesi", ROOM_IN, [2, 1], place("vertex", 0, 0, 1)),
    ("Kapının çemberinin merkezi", ROOM_IN, [2, 2], place("center")),
    ("Aynalı odadaki kapının kanadı boyunca", FRAME_IN, [1, 1, 1], along(0, 0, 1, 0.5)),
    ("Aynalı odanın köşesi", FRAME_IN, [1, 0], place("vertex", 0, 0, 1)),
    ("Yerleştirme anılamaz", ROOM_IN, [1], place("start")),
    ("Olmayan sıra", ROOM_IN, [7], place("start")),
    ("Yerleştirme olmayanın içi", ROOM_IN, [0, 0], place("start")),
    ("Boş yol", ROOM_IN, [], place("start")),
]

PIECE_FOLLOW_CASES = [
    ("Taşınan odanın kapısı izler", [1, 0], place("end"), DEFS, ROOM_IN, DEFS, insert("oda", 25, 12)),
    ("Döndürülen odanın köşesi izler", [0], place("vertex", 0, 0, 2), DEFS, ROOM_IN, DEFS,
     insert("oda", 20, 10, 1.0, math.pi / 4)),
    ("Genişleyen kapı: yeni tanımda uç izler", [1, 0], place("end"), DEFS, ROOM_IN, DEFS_WIDE, ROOM_IN),
    ("Kanadı silinen kapı: parça yok", [1, 1], place("vertex", 0, 0, 1), DEFS, ROOM_IN, DEFS_SHORT, ROOM_IN),
    ("Taban noktası değişen oda", [0], place("vertex", 0, 0, 0), DEFS, ROOM_IN, DEFS_REBASED, ROOM_IN),
    ("Silinen yerleştirme", [1, 0], place("end"), DEFS, ROOM_IN, DEFS, None),
    ("Yerleştirme çizgiye döndü", [1, 0], place("end"), DEFS, ROOM_IN, DEFS,
     {"kind": "line", "a": v(0, 0), "b": v(1, 0)}),
    ("Aynalı çerçevede kanat boyunca", [1, 1, 1], along(0, 0, 1, 0.5), DEFS, FRAME_IN, DEFS,
     insert("cerceve", 90, 40, 1.0, math.pi / 2)),
    ("Kapının önüne nesne girdi: kanat yerel kimliğiyle bulunur, yol değişir", [1, 1], place("vertex", 0, 0, 1),
     DEFS_IDS, ROOM_IN, DEFS_PUT_BEFORE, ROOM_IN),
    ("Yerel kimlikleri tek olmayan kapı: yol yerinde kalır, önüne giren nesneyi anar", [1, 0], place("end"),
     DEFS_TWICE, ROOM_IN, DEFS_TWICE_PUT, ROOM_IN),
]

PIECE_SNAP_CASES = [
    ("Uç: odadaki kapının sonu", ROOM_IN, "endpoint", None, [1, 0], place("end")),
    ("Merkez: dönük kapının çemberi", ROOM_IN, "center", None, [2, 2], place("center")),
    ("Yakın: odanın alt kenarı", ROOM_IN, "nearest", v(22, 10), None, None),
    ("Uç ama orada parça yok", ROOM_IN, "endpoint", v(30, 30), None, None),
]

NEAR_TOL = 0.3
NEAR_CASES = [
    ("Çizginin sonunun yanı: uç", LINE, v(3.9, 0.1)),
    ("Kenarın yanı ama uç da yakın: uç önce", LINE, v(3.95, 0.0)),
    ("Çizginin üstü: kenar boyunca", LINE, v(2.2, 0.2)),
    ("Çizgiden uzak: yok", LINE, v(2.0, 1.0)),
    ("Alanın köşesinin yanı", ROOM, v(4.1, 2.9)),
    ("Alanın üst kenarı", ROOM, v(2.0, 3.1)),
    ("Alanın kapanan kenarı", ROOM, v(-0.1, 1.2)),
    ("Alanın içi: yok", ROOM, v(2.0, 1.5)),
    ("Deliğin kenarı", HOLED, v(3, 4.1)),
    ("Yaylı kenarın üstü", ARCED, v(1, -0.6)),
    ("Yolun dirseği", WALL, v(2.1, 2.1)),
    ("Yolun ikinci kenarı", WALL, v(2.15, 1.0)),
    ("Yayın başı", ARC, v(2.05, 0.1)),
    ("Yayın üstü", ARC, v(2.1 * math.cos(1.0), 2.1 * math.sin(1.0))),
    ("Dairenin üstü", CIRCLE, v(1 + 1.9 * math.cos(2.0), 1 + 1.9 * math.sin(2.0))),
    ("Dairenin merkezi tıklanmaz: yok", CIRCLE, v(1.0, 1.0)),
    ("Çok noktalının ikincisi", POINTS, v(2.1, 2.05)),
    ("Yazının ekleme noktası", TEXT, v(5.1, 6.1)),
    ("Yazının gövdesi: yok", TEXT, v(6.0, 6.4)),
]

PIECE_NEAR_CASES = [
    ("Odadaki kapının ucu", ROOM_IN, v(22.05, 10.05)),
    ("Odanın üst duvarı", ROOM_IN, v(22.0, 13.1)),
    ("Yerleştirmenin noktası, odanın köşesiyle aynı yerde: yerleştirmeninki", ROOM_IN, v(20.05, 10.05)),
    ("Çerçevede aynalı odanın köşesi", FRAME_IN, None),
    ("Bloğun hiçbir yerinin yakını değil: yok", ROOM_IN, v(40.0, 40.0)),
]


TRIG = {"Yayın başı", "Yayın sonu", "Elipsin başı", "Elipsin sonu", "Budanan yayın başı yeni başında", "Elipsin sonu",
        "Yaylı kenarın yarısı", "Yaylı kenarın çeyreği", "Yayın üçte biri", "Dairenin çeyreği", "Elipsin yarısı",
        "Budanan yayda oran kalır", "Büyüyen dairede oran kalır"}


def snap_at(defs, ins, path, pl):
    """A piece's place's point in the drawing, where a snap would stand."""
    return point(piece(defs, ins, path), pl)


def piece_snap(name, ins, snap, at, path, pl):
    if at is None:
        at = snap_at(DEFS, ins, path, pl)
    return {"name": name, "definitions": list(DEFS.values()), "insert": ins, "snap": snap, "at": at,
            "expect": place_in(DEFS, ins, snap, at)}


def build():
    points = []
    for name, e, pl in POINT_CASES:
        points.append({"name": name, "entity": e, "place": pl, "point": point(e, pl), "exact": name not in TRIG})
    follows = []
    for name, pl, before, after in FOLLOW_CASES:
        follows.append({"name": name, "place": pl, "before": before, "after": after,
                        "expect": follow(pl, before, after), "exact": name not in TRIG})
    for name, e, pl in ALONG_POINT_CASES:
        points.append({"name": name, "entity": e, "place": pl, "point": point(e, pl), "exact": name not in TRIG})
    for name, pl, before, after in ALONG_FOLLOW_CASES:
        follows.append({"name": name, "place": pl, "before": before, "after": after,
                        "expect": follow(pl, before, after), "exact": name not in TRIG})
    snaps = [{"name": n, "entity": e, "snap": s, "at": at, "place": place_at(e, s, at)} for n, e, s, at in SNAP_CASES]
    ranges = [{"name": n, "entity": e, "from": a, "to": b, "length": range_length(e, a, b)}
              for n, e, a, b in RANGE_CASES]
    range_follows = [{"name": n, "from": a, "to": b, "before": e, "after": after,
                      "expect": follow_range(a, b, e, after)} for n, e, a, b, after in RANGE_FOLLOW_CASES]
    pieces = [{"name": n, "definitions": list(DEFS.values()), "insert": ins, "path": path, "place": pl,
               "point": (lambda e: point(e, pl) if e else None)(piece(DEFS, ins, path))}
              for n, ins, path, pl in PIECE_CASES]
    piece_follows = []
    for n, path, pl, db, ib, da, ia in PIECE_FOLLOW_CASES:
        piece_follows.append({"name": n, "path": path, "place": pl, "before": {"definitions": list(db.values()),
                              "insert": ib}, "after": {"definitions": list(da.values()), "insert": ia},
                              "expect": follow_in(pl, path, db, ib, da, ia)})
    piece_snaps = [piece_snap(*c) for c in PIECE_SNAP_CASES]
    nears = [{"name": n, "entity": e, "at": at, "tolerance": NEAR_TOL, "place": place_near(e, at, NEAR_TOL)}
             for n, e, at in NEAR_CASES]
    piece_nears = []
    for n, ins, at in PIECE_NEAR_CASES:
        if at is None:
            # The framed room's corner (its vertex 2), a little off.
            q = point(piece(DEFS, ins, [1, 0]), place("vertex", 0, 0, 2))
            at = v(q["x"] + 0.05, q["y"] - 0.05)
        piece_nears.append({"name": n, "definitions": list(DEFS.values()), "insert": ins, "at": at,
                            "tolerance": NEAR_TOL, "expect": place_near_in(DEFS, ins, at, NEAR_TOL)})
    return {
        "format": "kentos.subref-cases",
        "version": 1,
        "source": "scripts/fixtures/subref_cases.py (docs/adr/0249, 0263)",
        "points": points,
        "follows": follows,
        "snaps": snaps,
        "ranges": ranges,
        "rangeFollows": range_follows,
        "pieces": pieces,
        "pieceFollows": piece_follows,
        "pieceSnaps": piece_snaps,
        "nears": nears,
        "pieceNears": piece_nears,
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    a = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if a.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil", file=sys.stderr)
            return 1
        b = build()
        print(f"alt nesne başvuruları tutarlı: {len(b['points'])} nokta, {len(b['follows'])} izleme, {len(b['snaps'])} kenet, "
              f"{len(b['ranges']) + len(b['rangeFollows'])} aralık, {len(b['pieces']) + len(b['pieceFollows']) + len(b['pieceSnaps'])} blok durumu, "
              f"{len(b['nears']) + len(b['pieceNears'])} alt nesne seçimi")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
