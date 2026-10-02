#!/usr/bin/env python3
"""Independent reference of Vektör oturtma's warping of objects (docs/adr/0156 §4–§5).

Writes fixtures/fit/v1/warp.json from the rules alone, with Python's standard
library and no KentOS code. Both platforms warp every case's objects with the
shared core (`ops::warp`, WASM `warpShapes`) and must give the same
geometry, the same elevations and the same counts, or refuse the same object.

A warp is the command's centred form (§6): `from`, `to` and its numbers;
f(p) = to + g(p − from), g the centred map, J its derivative.

- similarity {a, b}: g(x, y) = (a·x − b·y, b·x + a·y)
- affine {m: [a, b, c, d]}: g(x, y) = (a·x + c·y, b·x + d·y)
- projective {h: [a1, a2, a3, b1, b2, b3, c1, c2]}: w = c1·x + c2·y + 1,
  g(x, y) = ((a1·x + a2·y + a3)/w, (b1·x + b2·y + b3)/w)

The rules:

1. Beyond the horizon: a point where w ≤ 1e-9 (a vertex, a curve's sample,
   an anchor) refuses the whole warp, naming the object (its index).
2. A similarity (the similarity kind; an affine whose a = d and b = −c, or
   a = −d and b = c) moves every object as the modify tools do (not in these
   cases beyond a Helmert check of points, a circle, an arc and a text).
3. Otherwise:
   - points, lines, straight paths (polyline, polygon, holes, parts), hatch
     rings, a spline's fit points: every vertex by f; elevations stay;
   - an arc segment of a path (a bulge), and under a projective warp a
     circle, an arc and an ellipse: straight vertices along the curve (rule
     4); a path keeps its vertices and gains the new ones, an arc segment's
     new vertices taking its ends' elevations linearly by angle (when both
     ends have one); a full curve becomes a polyline whose last vertex is
     its first, an arc an open polyline;
   - under an affine warp a circle, an arc and an ellipse stay exact: the
     ellipse of the curve's image (rule 5);
   - a text (and a leader's note at its last point): at its anchor's J,
     its rotation is the image of its baseline, a half turn more when J
     mirrors; height · |det J| / |J·u|; width factor · |J·u|² / |det J|;
   - a block insert: its point by f; with s = √|det J| and θ the angle of
     J's first column, the similarity rule of the modify tools (scale·s;
     rotation θ ± rotation; mirrored when J mirrors);
   - a dimension: its points by f; with J at the middle of a and b, its
     offset and height · √|det J| (the offset's sign turns when J mirrors,
     an angular one's arms swap), a linear one's direction its image under J;
   - a hatch: its pattern's angle the image of its direction under J at the
     mean of its outer ring's vertices (degrees, from 0 up to 180), its
     spacing · √|det J|;
   - an xline or ray: its point by f, its direction J·dir made a unit; a ray
     that runs to the horizon (c·dir < 0 under a projective warp, rule 1 at
     the point where w reaches 0) is refused.
4. Straight vertices along a curve P(t), t from t0 to t1: the interval split
   into ⌈|t1 − t0| / (π/8)⌉ equal parts, each halved again while the image of
   its middle lies more than 0.1 mm from the chord of its ends' images (at
   most 30 times). A vertex is f of its parameter's point; the ends of an
   arc segment are its vertices' images.
5. An ellipse's image: the curve is c + M·cos t + N·sin t (a circle: M =
   (r, 0), N = (0, r); an ellipse: M its major, N = ratio · M turned a
   quarter); A = J·[M N]. From A·Aᵀ = [[p, q], [q, s]]: the major's angle
   φ = ½·atan2(2q, p − s), σ₁,₂² = (p + s)/2 ± √(((p − s)/2)² + q²); major
   σ₁·(cos φ, sin φ), ratio σ₂/σ₁. A point q of the image (from the centre)
   is at t = atan2((q·u₂)/σ₂, (q·u₁)/σ₁), u₁ = (cos φ, sin φ), u₂ its quarter
   turn. A full curve stays full (t from 0 to 2π); an arc goes from its
   start's image to its end's, the other way round when J mirrors.

What is compared: every object's kind and fields (coordinates within
1e-9 m, angles within 1e-12 rad, other numbers within 1e-12 of their size,
at least 1), its paths' elevations, and the counts: curves turned into
straight vertices, and texts, notes, blocks, dimensions and hatch patterns
kept their shape.
"""

import argparse
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "fit" / "v1" / "warp.json"
TOL = 1e-4
HORIZON = 1e-9
STEP = math.pi / 8
DEPTH = 30
TAU = 2 * math.pi


def P(x, y):
    return {"x": float(x), "y": float(y)}


def norm_angle(a):
    r = math.fmod(a, TAU)
    return r + TAU if r < 0 else r


def sweep(a0, a1):
    """CCW sweep from a0 to a1 in (0, 2π]; equal angles mean a full turn."""
    s = norm_angle(a1 - a0)
    return TAU if s < 1e-12 else s


class Beyond(Exception):
    pass


class Warp:
    def __init__(self, w):
        self.kind = w["kind"]
        self.fx, self.fy = w["from"]["x"], w["from"]["y"]
        self.tx, self.ty = w["to"]["x"], w["to"]["y"]
        if self.kind == "similarity":
            a, b = w["a"], w["b"]
            self.m = (a, b, -b, a)
        elif self.kind == "affine":
            self.m = tuple(w["m"])
        else:
            self.h = tuple(w["h"])

    def centred(self, x, y):
        """g at a centred point, or Beyond."""
        if self.kind == "projective":
            a1, a2, a3, b1, b2, b3, c1, c2 = self.h
            w = c1 * x + c2 * y + 1
            if w <= HORIZON:
                raise Beyond()
            return ((a1 * x + a2 * y + a3) / w, (b1 * x + b2 * y + b3) / w)
        a, b, c, d = self.m
        return (a * x + c * y, b * x + d * y)

    def g_of(self, p):
        return self.centred(p[0] - self.fx, p[1] - self.fy)

    def f(self, p):
        gx, gy = self.g_of(p)
        return (self.tx + gx, self.ty + gy)

    def jac(self, p):
        """J at p as (a, b, c, d): columns (a, b) and (c, d)."""
        if self.kind != "projective":
            return self.m
        a1, a2, a3, b1, b2, b3, c1, c2 = self.h
        x, y = p[0] - self.fx, p[1] - self.fy
        w = c1 * x + c2 * y + 1
        if w <= HORIZON:
            raise Beyond()
        n1, n2 = a1 * x + a2 * y + a3, b1 * x + b2 * y + b3
        return ((a1 * w - n1 * c1) / (w * w), (b1 * w - n2 * c1) / (w * w), (a2 * w - n1 * c2) / (w * w), (b2 * w - n2 * c2) / (w * w))

    def similar(self):
        if self.kind == "similarity":
            return True
        if self.kind != "affine":
            return False
        a, b, c, d = self.m
        return (a == d and b == -c) or (a == -d and b == c)


def det(j):
    return j[0] * j[3] - j[1] * j[2]


def lin(j, v):
    return (j[0] * v[0] + j[2] * v[1], j[1] * v[0] + j[3] * v[1])


def xy(p):
    return (p["x"], p["y"])


def pt(p):
    return {"x": p[0], "y": p[1]}


def densify(warp, point, t0, t1):
    """Rule 4: the parameters of the straight vertices from t0 to t1, both ends included."""
    n = max(1, math.ceil(abs(t1 - t0) / STEP))
    params = [t0 + (t1 - t0) * k / n for k in range(n + 1)]
    out = [t0]

    def chord_far(ta, tb):
        tm = (ta + tb) / 2
        a, m, b = warp.g_of(point(ta)), warp.g_of(point(tm)), warp.g_of(point(tb))
        dx, dy = b[0] - a[0], b[1] - a[1]
        length = math.hypot(dx, dy)
        if length == 0:
            return math.hypot(m[0] - a[0], m[1] - a[1]) > TOL
        return abs(dx * (m[1] - a[1]) - dy * (m[0] - a[0])) / length > TOL

    def split(ta, tb, depth):
        if depth < DEPTH and chord_far(ta, tb):
            tm = (ta + tb) / 2
            split(ta, tm, depth + 1)
            split(tm, tb, depth + 1)
        else:
            out.append(tb)

    for k in range(n):
        split(params[k], params[k + 1], 0)
    return out


def bulge_arc(a, b, bulge):
    dx, dy = b[0] - a[0], b[1] - a[1]
    chord = math.hypot(dx, dy)
    k = (1 - bulge * bulge) / (4 * bulge)
    c = ((a[0] + b[0]) / 2 - dy * k, (a[1] + b[1]) / 2 + dx * k)
    r = chord * (1 + bulge * bulge) / (4 * abs(bulge))
    return c, r, math.atan2(a[1] - c[1], a[0] - c[0]), 4 * math.atan(bulge)


def warp_ring(warp, pts, bulges, zs, closed):
    """A path's vertices and elevations warped; (pts, zs, densified)."""
    pts = [xy(p) for p in pts]
    n = len(pts)
    zs = list(zs) if zs is not None else [None] * n
    bulges = list(bulges) if bulges else [0.0] * n
    out, out_z, dense = [], [], False
    edges = n if closed else n - 1
    for i in range(n):
        out.append(warp.f(pts[i]))
        out_z.append(zs[i])
        if i >= edges:
            continue
        j = (i + 1) % n
        bg = bulges[i] if i < len(bulges) else 0.0
        if abs(bg) <= 1e-12 or math.hypot(pts[j][0] - pts[i][0], pts[j][1] - pts[i][1]) < 1e-12:
            continue
        dense = True
        c, r, a0, sw = bulge_arc(pts[i], pts[j], bg)
        ts = densify(warp, lambda t: (c[0] + r * math.cos(a0 + sw * t), c[1] + r * math.sin(a0 + sw * t)), 0.0, 1.0)
        for t in ts[1:-1]:
            out.append(warp.f((c[0] + r * math.cos(a0 + sw * t), c[1] + r * math.sin(a0 + sw * t))))
            za, zb = zs[i], zs[j]
            out_z.append(za + (zb - za) * t if za is not None and zb is not None else None)
    return [pt(p) for p in out], out_z, dense


def ellipse_of(j, mv, nv):
    """Rule 5: the image's major, ratio, and its axes for the parameters."""
    a, b, c, d = j
    # A = J·[M N]: its columns.
    c1 = (a * mv[0] + c * mv[1], b * mv[0] + d * mv[1])
    c2 = (a * nv[0] + c * nv[1], b * nv[0] + d * nv[1])
    p = c1[0] * c1[0] + c2[0] * c2[0]
    s = c1[1] * c1[1] + c2[1] * c2[1]
    q = c1[0] * c1[1] + c2[0] * c2[1]
    phi = 0.5 * math.atan2(2 * q, p - s)
    root = math.hypot((p - s) / 2, q)
    s1 = math.sqrt((p + s) / 2 + root)
    s2 = math.sqrt(max((p + s) / 2 - root, 0.0))
    u1 = (math.cos(phi), math.sin(phi))
    u2 = (-u1[1], u1[0])
    return (s1 * u1[0], s1 * u1[1]), s2 / s1, u1, u2, s1, s2


def param_on(q, u1, u2, s1, s2):
    return norm_angle(math.atan2((q[0] * u2[0] + q[1] * u2[1]) / s2, (q[0] * u1[0] + q[1] * u1[1]) / s1))


def text_rule(warp, anchor, height, rotation, width):
    j = warp.jac(anchor)
    rad = rotation * math.pi / 180
    lu = lin(j, (math.cos(rad), math.sin(rad)))
    length = math.hypot(lu[0], lu[1])
    dt = det(j)
    rot = math.atan2(lu[1], lu[0]) * 180 / math.pi
    if dt < 0:
        rot += 180
    rot = math.fmod(math.fmod(rot, 360) + 360, 360)
    return height * abs(dt) / length, rot, (width if width is not None else 1.0) * length * length / abs(dt)


def warp_object(warp, o, zs):
    """(shape, zs, curves, shapes) for one object under a non-similar warp, or Beyond."""
    k = o["kind"]
    if k == "point":
        return {"kind": "point", "p": pt(warp.f(xy(o["p"]))), **({"z": o["z"]} if o.get("z") is not None else {})}, zs, 0, 0
    if k == "line":
        return {"kind": "line", "a": pt(warp.f(xy(o["a"]))), "b": pt(warp.f(xy(o["b"])))}, zs, 0, 0
    if k in ("polyline", "polygon"):
        closed = k == "polygon"
        rings = [(o["pts"], o.get("bulges"))] + [(h["pts"], h.get("bulges")) for h in o.get("holes") or []]
        out_rings, out_zs, dense = [], [], False
        for i, (pts, bulges) in enumerate(rings):
            p2, z2, d2 = warp_ring(warp, pts, bulges, zs[i] if i < len(zs) else None, closed)
            # A ring keeps its bulges unless its arcs became straight vertices.
            ring = {"pts": p2}
            if not d2 and bulges is not None:
                ring["bulges"] = bulges
            out_rings.append(ring)
            out_zs.append(z2)
            dense = dense or d2
        shape = {"kind": k, **out_rings[0]}
        if len(out_rings) > 1:
            shape["holes"] = out_rings[1:]
        return shape, out_zs, 1 if dense else 0, 0
    if k in ("circle", "arc", "ellipse"):
        if k == "ellipse":
            c = xy(o["c"])
            mv = xy(o["major"])
            nv = (-mv[1] * o["ratio"], mv[0] * o["ratio"])
            t0, t1 = o["t0"], o["t1"]
            full = sweep(t0, t1) >= TAU - 1e-12
            span = (0.0, TAU) if full else (t0, t0 + sweep(t0, t1))
        else:
            c = xy(o["c"])
            mv, nv = (o["r"], 0.0), (0.0, o["r"])
            full = k == "circle"
            span = (0.0, TAU) if full else (o["a0"], o["a0"] + sweep(o["a0"], o["a1"]))

        def point(t):
            return (c[0] + mv[0] * math.cos(t) + nv[0] * math.sin(t), c[1] + mv[1] * math.cos(t) + nv[1] * math.sin(t))

        if warp.kind == "projective":
            ts = densify(warp, point, span[0], span[1])
            pts = [warp.f(point(t)) for t in ts]
            if full:
                pts[-1] = pts[0]
            return {"kind": "polyline", "pts": [pt(p) for p in pts]}, [], 1, 0
        j = warp.jac(c)
        major, ratio, u1, u2, s1, s2 = ellipse_of(j, mv, nv)
        cc = warp.f(c)
        if full:
            t0, t1 = 0.0, TAU
        else:
            start, end = warp.g_of(point(span[0])), warp.g_of(point(span[1]))
            gc = warp.g_of(c)
            qs, qe = (start[0] - gc[0], start[1] - gc[1]), (end[0] - gc[0], end[1] - gc[1])
            ts, te = param_on(qs, u1, u2, s1, s2), param_on(qe, u1, u2, s1, s2)
            t0, t1 = (ts, te) if det(j) > 0 else (te, ts)
        return {"kind": "ellipse", "c": pt(cc), "major": pt(major), "ratio": ratio, "t0": t0, "t1": t1}, [], 0, 0
    if k == "spline":
        return {"kind": "spline", "pts": [pt(warp.f(xy(p))) for p in o["pts"]], "closed": o["closed"]}, [], 0, 0
    if k in ("xline", "ray"):
        p = xy(o["p"])
        d = xy(o["dir"])
        if k == "ray" and warp.kind == "projective":
            c1, c2 = warp.h[6], warp.h[7]
            if c1 * d[0] + c2 * d[1] < 0:
                raise Beyond()
        jd = lin(warp.jac(p), d)
        length = math.hypot(jd[0], jd[1])
        return {"kind": k, "p": pt(warp.f(p)), "dir": pt((jd[0] / length, jd[1] / length))}, [], 0, 0
    if k == "text":
        h, rot, wf = text_rule(warp, xy(o["p"]), o["height"], o["rotation"], o.get("widthFactor"))
        out = {"kind": "text", "p": pt(warp.f(xy(o["p"]))), "text": o["text"], "height": h, "rotation": rot, "widthFactor": wf}
        return out, [], 0, 1
    if k == "leader":
        pts = [xy(p) for p in o["pts"]]
        h, rot, _ = text_rule(warp, pts[-1], o["height"], o["rotation"], None)
        out = {"kind": "leader", "pts": [pt(warp.f(p)) for p in pts], "text": o["text"], "height": h, "rotation": rot}
        return out, [], 0, 1
    if k == "insert":
        p = xy(o["p"])
        j = warp.jac(p)
        s = math.sqrt(abs(det(j)))
        theta = math.atan2(j[1], j[0])
        flip = det(j) < 0
        mirrored = bool(o.get("mirror")) != flip
        rot = norm_angle(theta - o["rotation"] if flip else theta + o["rotation"])
        out = {"kind": "insert", "block": o["block"], "p": pt(warp.f(p)), "scale": o["scale"] * s, "rotation": rot}
        if mirrored:
            out["mirror"] = True
        return out, [], 0, 1
    if k == "dimension":
        a, b = xy(o["a"]), xy(o["b"])
        mid = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
        j = warp.jac(mid)
        s = math.sqrt(abs(det(j)))
        flip = det(j) < 0
        style = o.get("style") or "aligned"
        na, nb = warp.f(a), warp.f(b)
        out = {"kind": "dimension"}
        if style in ("angular", "arcLength"):
            if flip:
                na, nb = nb, na
            offset = o["offset"] * s
        elif style in ("radius", "diameter", "jogged", "ordinate"):
            offset = o["offset"] * s
        else:
            offset = o["offset"] * s * (-1 if flip else 1)
        out.update({"a": pt(na), "b": pt(nb), "offset": offset, "height": o["height"] * s})
        if o.get("style") is not None:
            out["style"] = o["style"]
        if style == "linear":
            rad = (o.get("angle") or 0.0) * math.pi / 180
            dv = lin(j, (math.cos(rad), math.sin(rad)))
            out["angle"] = math.atan2(dv[1], dv[0]) * 180 / math.pi
        elif o.get("angle") is not None:
            out["angle"] = o["angle"]
        if o.get("c") is not None:
            out["c"] = pt(warp.f(xy(o["c"])))
        return out, [], 0, 1
    if k == "hatch":
        ring = [xy(p) for p in o["ring"]]
        mean = (sum(p[0] for p in ring) / len(ring), sum(p[1] for p in ring) / len(ring))
        j = warp.jac(mean)
        pat = dict(o["pattern"])
        rad = pat["angle"] * math.pi / 180
        dv = lin(j, (math.cos(rad), math.sin(rad)))
        pat["angle"] = math.fmod(math.fmod(math.atan2(dv[1], dv[0]) * 180 / math.pi, 180) + 180, 180)
        pat["spacing"] = pat["spacing"] * math.sqrt(abs(det(j)))
        out = {"kind": "hatch", "ring": [pt(warp.f(p)) for p in ring], "pattern": pat}
        if o.get("holes"):
            out["holes"] = [[pt(warp.f(xy(p))) for p in h] for h in o["holes"]]
        return out, [], 0, 1
    raise ValueError(k)


def warp_all(warp, objects, zs):
    """Every object warped, or the first refused one's index."""
    out, out_zs, curves, shapes = [], [], 0, 0
    for i, o in enumerate(objects):
        try:
            shape, z, cu, sh = warp_object(warp, o, zs[i])
        except Beyond:
            return {"error": "beyond_horizon", "at": i}
        out.append(shape)
        out_zs.append(z)
        curves += cu
        shapes += sh
    return {"shapes": out, "zs": out_zs, "curves": curves, "kept": shapes}


def objects():
    """The drawing (local metres around 0, 0) and each object's paths' elevations."""
    o = [
        ({"kind": "point", "p": P(12.5, -4.25), "z": 101.25}, []),
        ({"kind": "line", "a": P(-30, -20), "b": P(40, 15)}, [[100.0, 102.5]]),
        ({"kind": "polyline", "pts": [P(-40, 30), P(-10, 30), P(10, 50)], "bulges": [0.0, 0.5, 0.0]}, [[100.0, 101.0, 103.0]]),
        (
            {"kind": "polygon", "pts": [P(0, 0), P(30, 0), P(30, 20), P(0, 20)], "bulges": [0.0, 0.0, -0.3, 0.0], "holes": [{"pts": [P(5, 5), P(10, 5), P(10, 10), P(5, 10)]}]},
            [[100.0, 100.5, 101.0, None], [None, None, None, None]],
        ),
        ({"kind": "circle", "c": P(-20, -10), "r": 6.0}, []),
        ({"kind": "arc", "c": P(25, -15), "r": 8.0, "a0": math.pi / 6, "a1": 5 * math.pi / 6}, []),
        ({"kind": "ellipse", "c": P(-5, 40), "major": P(9, 3), "ratio": 0.5, "t0": 0.0, "t1": TAU}, []),
        ({"kind": "ellipse", "c": P(35, 35), "major": P(-4, 7), "ratio": 0.6, "t0": 0.4, "t1": 2.6}, []),
        ({"kind": "spline", "pts": [P(-35, -35), P(-25, -28), P(-12, -33), P(0, -26)], "closed": False}, []),
        ({"kind": "xline", "p": P(5, -40), "dir": P(0.6, 0.8)}, []),
        ({"kind": "text", "p": P(15, 25), "text": "Ada 101", "height": 2.0, "rotation": 30.0}, []),
        ({"kind": "insert", "block": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0001", "p": P(-15, 15), "scale": 1.0, "rotation": 0.25}, []),
        ({"kind": "hatch", "ring": [P(40, -40), P(55, -40), P(55, -25), P(40, -25)], "pattern": {"type": "lines", "angle": 45.0, "spacing": 1.5}}, []),
        ({"kind": "dimension", "a": P(-40, -45), "b": P(-10, -45), "offset": 4.0, "height": 2.5}, []),
        ({"kind": "leader", "pts": [P(20, 5), P(28, 12), P(33, 12)], "text": "Sınır taşı", "height": 1.8, "rotation": 0.0}, []),
    ]
    return [x[0] for x in o], [x[1] for x in o]


WARPS = {
    "helmert": {"kind": "similarity", "from": P(0, 0), "to": P(487012.341, 4419876.552), "a": 0.9975670230, "b": 0.0722500300},
    "afin": {"kind": "affine", "from": P(0, 0), "to": P(487100.0, 4420050.0), "m": [1.2, 0.1, 0.3, 0.8]},
    "aynali-afin": {"kind": "affine", "from": P(0, 0), "to": P(487100.0, 4420050.0), "m": [1.1, 0.2, 0.25, -0.9]},
    "projektif": {"kind": "projective", "from": P(0, 0), "to": P(487000.0, 4420000.0), "h": [1.0, 0.05, 2.0, -0.03, 0.95, -1.0, 0.002, -0.001]},
}


def cases():
    objs, zs = objects()
    rows = []

    def add(name, warp, idx, extra=None):
        chosen = [objs[i] for i in idx] + (extra or [])
        chosen_zs = [zs[i] for i in idx] + [[] for _ in extra or []]
        rows.append({"name": name, "warp": warp, "objects": chosen, "zs": chosen_zs, "expected": warp_all(Warp(WARPS[warp]), chosen, chosen_zs)})

    every = list(range(len(objs)))
    add("Afin: her tür; daire ve yaylar elips, yaylı kenarlar köşelere açılır", "afin", every)
    add("Aynalı afin: yaylar ters döner, yazı okunur kalır, blok aynalanır", "aynali-afin", every)
    add("Projektif: eğriler 0,1 mm'lik köşelere açılır, doğrular doğru kalır", "projektif", every)
    add(
        "Projektif: ufka koşan ışın bütün dönüşümü reddeder",
        "projektif",
        [0, 1],
        [{"kind": "ray", "p": P(10, 10), "dir": P(-0.8, 0.6)}],
    )
    add("Projektif: ufuktan uzaklaşan ışın dönüşür", "projektif", [0], [{"kind": "ray", "p": P(10, 10), "dir": P(0.8, -0.6)}])
    add(
        "Projektif: ufkun ötesindeki nokta bütün dönüşümü reddeder",
        "projektif",
        [1],
        [{"kind": "point", "p": P(-800, 0)}],
    )
    return rows


def build():
    return {
        "format": "kentos.fit-warp",
        "version": 1,
        "tolerance": {"metres": 1e-9, "radians": 1e-12, "relative": 1e-12},
        "warps": WARPS,
        "cases": cases(),
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--check", action="store_true", help="compare with the written file instead of writing it")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT} güncel değil: python3 scripts/fixtures/warp_cases.py", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT}: güncel")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    print(f"{OUT}: {len(json.loads(text)['cases'])} durum")


if __name__ == "__main__":
    main()
