#!/usr/bin/env python3
"""Writes fixtures/scene3d/v1/cases.json: the shared cases of KentOS's 3D scene core (crates/shared/scene3d,
plan/3d.md D1, docs/adr/0910 §3), their expectations computed here from the rules alone, without KentOS code:

- the camera: its translation-free rotation and projection (reversed depth, perspective's far plane at infinity), the
  frustum's verdict on boxes (the p-vertex test), a point's pixel and depth, a pixel's ray, a tile origin's offset
  from the eye made float32 (numpy's IEEE rounding);
- the selection: the screen-space error (CesiumJS's: error · height / (distance · 2 tan(fov/2)), orthographic error /
  pixel size; the vertical exaggeration scales it by its largest scale), 3D Tiles 1.1's replacement and additive
  refinement, a parent kept until the children in view can cover, a tile without content always refined, the asks by
  error then key;
- the chunking: the median cut along the longest side of the centroids' box, the leaves within the limits, each
  vertex once as a float32 offset from its leaf's middle, the triangles by part, the lineage, the encoding error.

    uv run --no-project --python 3.11 --with numpy python3 scripts/fixtures/scene3d_cases.py [--check]
"""

import json
import math
import random
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/scene3d/v1/cases.json"


# ── Vectors ──────────────────────────────────────────────────────────────────────────────────────────────────────

def sub(a, b):
    return [a[0] - b[0], a[1] - b[1], a[2] - b[2]]


def add(a, b):
    return [a[0] + b[0], a[1] + b[1], a[2] + b[2]]


def mul(a, k):
    return [a[0] * k, a[1] * k, a[2] * k]


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def cross(a, b):
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]


def unit(a):
    n = math.sqrt(dot(a, a))
    return mul(a, 1 / n) if n > 0 else a


# ── The camera ───────────────────────────────────────────────────────────────────────────────────────────────────

def orbit(target, azimuth, elevation, distance, projection):
    a, e = math.radians(azimuth), math.radians(elevation)
    eye = [target[0] + distance * math.cos(e) * math.cos(a), target[1] + distance * math.cos(e) * math.sin(a),
           target[2] + distance * math.sin(e)]
    return {"eye": eye, "target": list(target), "up": [0.0, 0.0, 1.0], "projection": projection}


def basis(c):
    f = unit(sub(c["target"], c["eye"]))
    s = cross(f, c["up"])
    if math.sqrt(dot(s, s)) < 1e-9:
        s = cross(f, [0.0, 1.0, 0.0])
        if math.sqrt(dot(s, s)) < 1e-9:
            s = [1.0, 0.0, 0.0]
    s = unit(s)
    return s, cross(s, f), f


def aspect(vp):
    return max(vp["width"], 1.0) / max(vp["height"], 1.0)


def matrix(c, asp):
    """P · R, rows; R the view's rotation (rows s, u, −f), P reversed depth."""
    s, u, f = basis(c)
    r = np.array([s + [0.0], u + [0.0], [-f[0], -f[1], -f[2], 0.0], [0.0, 0.0, 0.0, 1.0]])
    p = c["projection"]
    if p["kind"] == "perspective":
        g = 1 / math.tan(p["fovY"] / 2)
        pm = np.array([[g / asp, 0, 0, 0], [0, g, 0, 0], [0, 0, 0, p["near"]], [0, 0, -1, 0]], dtype=float)
    else:
        hh = p["height"] / 2
        hw = hh * asp
        pm = np.array([[1 / hw, 0, 0, 0], [0, 1 / hh, 0, 0], [0, 0, 1 / p["depth"], 1], [0, 0, 0, 1]], dtype=float)
    return pm @ r


def column_major(m):
    return [float(m[r, c]) for c in range(4) for r in range(4)]


def planes(c, asp):
    s, u, f = basis(c)
    p = c["projection"]
    if p["kind"] == "perspective":
        ty = math.tan(p["fovY"] / 2)
        tx = ty * asp
        return [(f, -p["near"]), (add(s, mul(f, tx)), 0.0), (add(mul(s, -1), mul(f, tx)), 0.0),
                (add(u, mul(f, ty)), 0.0), (add(mul(u, -1), mul(f, ty)), 0.0)]
    hh = p["height"] / 2
    hw = hh * asp
    return [(f, 0.0), (mul(f, -1), p["depth"]), (s, hw), (mul(s, -1), hw), (u, hh), (mul(u, -1), hh)]


def visibility(c, asp, box):
    lo, hi = box
    inside = True
    for n, d in planes(c, asp):
        far = [hi[a] if n[a] >= 0 else lo[a] for a in range(3)]
        near = [lo[a] if n[a] >= 0 else hi[a] for a in range(3)]
        if dot(n, sub(far, c["eye"])) + d < 0:
            return "outside"
        if dot(n, sub(near, c["eye"])) + d < 0:
            inside = False
    return "inside" if inside else "intersecting"


def project(c, vp, p):
    m = matrix(c, aspect(vp))
    q = sub(p, c["eye"])
    clip = m @ np.array(q + [1.0])
    if clip[3] <= 0:
        return None
    x, y, z = clip[0] / clip[3], clip[1] / clip[3], clip[2] / clip[3]
    return [float((x + 1) / 2 * vp["width"]), float((1 - y) / 2 * vp["height"]), float(z)]


def ray(c, vp, x, y):
    s, u, f = basis(c)
    nx = 2 * x / max(vp["width"], 1.0) - 1
    ny = 1 - 2 * y / max(vp["height"], 1.0)
    p = c["projection"]
    if p["kind"] == "perspective":
        ty = math.tan(p["fovY"] / 2)
        tx = ty * aspect(vp)
        return c["eye"], unit(add(f, add(mul(s, nx * tx), mul(u, ny * ty))))
    hh = p["height"] / 2
    hw = hh * aspect(vp)
    return add(c["eye"], add(mul(s, nx * hw), mul(u, ny * hh))), f


def shown(ex, p):
    return [p[0], p[1], ex["datum"] + ex["factor"] * (p[2] - ex["datum"])]


def offset(c, ex, origin):
    return [float(np.float32(v)) for v in sub(shown(ex, origin), c["eye"])]


# ── The selection ────────────────────────────────────────────────────────────────────────────────────────────────

def shown_box(ex, box):
    lo, hi = shown(ex, box[0]), shown(ex, box[1])
    return [[lo[0], lo[1], min(lo[2], hi[2])], [hi[0], hi[1], max(lo[2], hi[2])]]


def distance_to(c, box):
    lo, hi = box
    d = [max(lo[a] - c["eye"][a], c["eye"][a] - hi[a], 0.0) for a in range(3)]
    return math.sqrt(dot(d, d))


def sse(ge, dist, c, vp):
    if ge <= 0:
        return 0.0
    p = c["projection"]
    if p["kind"] == "perspective":
        return ge * vp["height"] / (max(dist, 1e-7) * 2 * math.tan(p["fovY"] / 2))
    return ge / (p["height"] / max(vp["height"], 1.0))


def select(h, c, vp, ex, limit, ready):
    tiles = h["tiles"]
    asp = aspect(vp)
    draw, asks = [], []
    count = {"visited": 0, "culled": 0}
    scale = max(abs(ex["factor"]), 1.0)

    def box(i):
        return shown_box(ex, tiles[i]["bounds"])

    def in_view(i):
        return visibility(c, asp, box(i)) != "outside"

    def error(i):
        return sse(tiles[i]["geometricError"] * scale, distance_to(c, box(i)), c, vp)

    def is_ready(i):
        return tiles[i]["content"] is not None and tiles[i]["content"]["id"] in ready

    def can_cover(i):
        if tiles[i]["content"] is not None:
            return is_ready(i)
        return all(not in_view(k) or can_cover(k) for k in tiles[i]["children"])

    def ask_cover(i):
        if tiles[i]["content"] is not None:
            if not is_ready(i):
                asks.append((error(i), tiles[i]["key"], i))
        else:
            for k in tiles[i]["children"]:
                if in_view(k):
                    ask_cover(k)

    def own(i, e):
        if tiles[i]["content"] is None:
            return
        if is_ready(i):
            draw.append(i)
        else:
            asks.append((e, tiles[i]["key"], i))

    def visit(i):
        count["visited"] += 1
        if not in_view(i):
            count["culled"] += 1
            return
        t = tiles[i]
        e = error(i)
        kids = t["children"]
        refine = bool(kids) and (e > limit or t["content"] is None)
        if not refine:
            own(i, e)
            return
        if t["refine"] == "add":
            own(i, e)
            for k in kids:
                visit(k)
            return
        if t["content"] is None or all(not in_view(k) or can_cover(k) for k in kids):
            for k in kids:
                visit(k)
        else:
            own(i, e)
            for k in kids:
                if in_view(k):
                    ask_cover(k)

    for r in h["roots"]:
        visit(r)
    asks.sort(key=lambda a: (-a[0], a[1]))
    wanted = []
    for _, _, i in asks:
        if i not in wanted:
            wanted.append(i)
    return {"draw": draw, "wanted": wanted, "visited": count["visited"], "culled": count["culled"]}


# ── The chunking ─────────────────────────────────────────────────────────────────────────────────────────────────

def bounds_of(points):
    lo = [min(p[a] for p in points) for a in range(3)]
    hi = [max(p[a] for p in points) for a in range(3)]
    return [lo, hi]


def vertex_normals(m):
    """A smooth mesh's vertex normals: the triangles' cross products summed at their corners in the triangles' order,
    unit, packed signed 8-bit (round half away from zero); none for a flat mesh."""
    if not m.get("smooth"):
        return None
    vs = m["vertices"]
    s = [[0.0, 0.0, 0.0] for _ in vs]
    for t in m["triangles"]:
        if any(k >= len(vs) for k in t):
            continue
        a, b, c = (vs[k] for k in t)
        n = cross(sub(b, a), sub(c, a))
        for k in t:
            s[k] = add(s[k], n)
    out = []
    for v in s:
        length = math.sqrt(dot(v, v))
        if length == 0:
            out.append([0, 0, 0, 0])
            continue
        q = lambda x: int(max(-127.0, min(127.0, math.copysign(math.floor(abs(x / length * 127.0) + 0.5), x))))  # noqa: E731
        out.append([q(v[0]), q(v[1]), q(v[2]), 0])
    return out


def chunk(meshes, limits, first_id):
    normals = [vertex_normals(m) for m in meshes]
    tris = []
    for mi, m in enumerate(meshes):
        for ti, t in enumerate(m["triangles"]):
            if any(k >= len(m["vertices"]) for k in t):
                continue
            a, b, cc = (m["vertices"][k] for k in t)
            tris.append({"mesh": mi, "index": ti, "c": [(a[x] + b[x] + cc[x]) / 3 for x in range(3)]})
    tiles, contents = [], []

    def corners(t):
        m = meshes[t["mesh"]]
        return [m["vertices"][k] for k in m["triangles"][t["index"]]]

    def tile(ts):
        bnd = bounds_of([p for t in ts for p in corners(t)])
        at = len(tiles)
        tiles.append({"key": at, "bounds": bnd, "geometricError": 0.0, "refine": "replace", "content": None,
                      "children": []})
        extent = max(bnd[1][a] - bnd[0][a] for a in range(3))
        small = len(ts) <= limits["maxTriangles"] and extent <= limits["maxExtent"]
        axis = None
        if not small and len(ts) >= 2:
            cl = [min(t["c"][a] for t in ts) for a in range(3)]
            ch = [max(t["c"][a] for t in ts) for a in range(3)]
            sides = [ch[a] - cl[a] for a in range(3)]
            best = 0
            for a in range(3):
                if sides[a] > sides[best]:
                    best = a
            axis = best if sides[best] > 0 else None
        if axis is None:
            content = leaf(ts, bnd)
            tiles[at]["content"] = {"id": content["id"], "bytes": len(content["positions"]) * 20 + len(content["indices"]) * 4}
            contents.append(content)
            return at
        ts = sorted(ts, key=lambda t: (t["c"][axis], t["mesh"], t["index"]))
        mid = len(ts) // 2
        a = tile(ts[:mid])
        b = tile(ts[mid:])
        tiles[at]["children"] = [a, b]
        d = sub(bnd[1], bnd[0])
        tiles[at]["geometricError"] = math.sqrt(dot(d, d))
        return at

    def leaf(ts, bnd):
        origin = [(bnd[0][a] + bnd[1][a]) / 2 for a in range(3)]
        ts = sorted(ts, key=lambda t: (meshes[t["mesh"]]["part"], t["mesh"], t["index"]))
        seen, positions, parts, normal_of, indices, ranges, sources = {}, [], [], [], [], [], []
        err = 0.0
        for t in ts:
            m = meshes[t["mesh"]]
            if not ranges or ranges[-1][0] != m["part"]:
                ranges.append([m["part"], len(indices), len(indices)])
            for k in m["triangles"][t["index"]]:
                if (t["mesh"], k) not in seen:
                    p = m["vertices"][k]
                    d = [p[a] - origin[a] for a in range(3)]
                    f = [float(np.float32(v)) for v in d]
                    err = max(err, max(abs(f[a] - d[a]) for a in range(3)))
                    positions.append(f)
                    parts.append(m["part"])
                    nm = normals[t["mesh"]]
                    normal_of.append(nm[k] if nm is not None else [0, 0, 0, 0])
                    seen[(t["mesh"], k)] = len(positions) - 1
                indices.append(seen[(t["mesh"], k)])
            ranges[-1][2] = len(indices)
            sources.append([m["part"], t["index"]])
        return {"id": first_id + len(contents), "origin": origin, "bounds": bnd, "positions": positions,
                "parts": parts, "normals": normal_of, "indices": indices, "ranges": ranges, "sources": sources, "encodingError": err}

    roots = [tile(tris)] if tris else []
    return {"tiles": tiles, "roots": roots}, contents


# ── The cases ────────────────────────────────────────────────────────────────────────────────────────────────────

def persp(deg=40.0, near=0.5):
    return {"kind": "perspective", "fovY": math.radians(deg), "near": near}


def ortho(height, depth):
    return {"kind": "orthographic", "height": height, "depth": depth}


NONE = {"factor": 1.0, "datum": 0.0}
VP = {"width": 1200.0, "height": 800.0}


def camera_case(name, c, vp=VP, ex=NONE, boxes=(), points=(), pixels=(), origins=()):
    asp = aspect(vp)
    return {"name": name, "camera": c, "viewport": vp, "exaggeration": ex,
            "matrix": column_major(matrix(c, asp)),
            "boxes": [{"box": b, "visibility": visibility(c, asp, b)} for b in boxes],
            "points": [{"point": p, "pixel": project(c, vp, p)} for p in points],
            "rays": [{"pixel": px, "origin": ray(c, vp, *px)[0], "direction": ray(c, vp, *px)[1]} for px in pixels],
            "offsets": [{"origin": o, "offset": offset(c, ex, o)} for o in origins]}


def cameras():
    far = [512345.678, 4401234.567, 1234.5]
    out = [
        camera_case("Kuzeydoğudan perspektif", orbit([0, 0, 0], 45, 30, 500, persp()),
                    boxes=[[[-10, -10, -1], [10, 10, 1]], [[1000, 1000, 0], [1100, 1100, 10]],
                           [[-400, -400, -50], [400, 400, 50]], [[300, 300, 200], [320, 320, 400]]],
                    points=[[0, 0, 0], [50, -20, 3], [400, 400, 300]], pixels=[[600, 400], [0, 0], [1199, 799]],
                    origins=[[0, 0, 0], [123.25, -45.5, 7.75]]),
        camera_case("Göz kutunun içinde", orbit([0, 0, 0], 120, 10, 5, persp(60, 0.1)),
                    boxes=[[[-20, -20, -20], [20, 20, 20]], [[-1, -1, -1], [1, 1, 1]]], points=[[0, 0, 0]]),
        camera_case("Tam üstten (yukarı kuzey)", {"eye": [10, 20, 300], "target": [10, 20, 0], "up": [0.0, 0.0, 1.0],
                                                    "projection": persp(50, 1.0)},
                    boxes=[[[0, 0, 0], [20, 40, 1]], [[500, 0, 0], [520, 10, 1]]], points=[[10, 20, 0], [30, 20, 0]],
                    pixels=[[600, 400]]),
        camera_case("Ortografik üst", {"eye": [0, 0, 1000], "target": [0, 0, 0], "up": [0.0, 0.0, 1.0],
                                        "projection": ortho(200, 2000)},
                    boxes=[[[-50, -50, 0], [50, 50, 10]], [[200, 0, 0], [250, 10, 1]], [[-10, -10, -1500], [10, 10, -1200]]],
                    points=[[0, 0, 0], [40, 30, 5], [0, 0, 1500]], pixels=[[0, 0], [600, 400]]),
        camera_case("Ortografik eğik", orbit([0, 0, 0], 200, 35, 800, ortho(300, 3000)),
                    boxes=[[[-100, -100, 0], [100, 100, 20]], [[500, 500, 0], [600, 600, 5]]], points=[[10, -10, 2]],
                    pixels=[[300, 200]]),
        camera_case("TM koordinatlarında, abartmalı", orbit(far, 315, 25, 750, persp(45, 0.5)),
                    ex={"factor": 3.0, "datum": 1200.0},
                    boxes=[[[512300.0, 4401200.0, 1230.0], [512400.0, 4401300.0, 1300.0]],
                           [[515000.0, 4405000.0, 1000.0], [515100.0, 4405100.0, 1100.0]]],
                    points=[far, [512346.0, 4401235.0, 1240.0]], pixels=[[600, 400], [10, 790]],
                    origins=[[512345.0, 4401234.0, 1234.0], [512845.5, 4401734.25, 1250.125]]),
        camera_case("Dar pencere", orbit([5, 5, 0], 10, 60, 120, persp(35, 0.25)), vp={"width": 400.0, "height": 900.0},
                    boxes=[[[-30, -30, 0], [40, 40, 5]], [[100, -5, 0], [110, 5, 2]]], points=[[5, 5, 0]],
                    pixels=[[200, 450]]),
        camera_case("Yere paralel bakış", {"eye": [0, -100, 2], "target": [0, 0, 2], "up": [0.0, 0.0, 1.0],
                                            "projection": persp()},
                    boxes=[[[-5, -10, 0], [5, 10, 4]], [[-5, -300, 0], [5, -200, 4]]], points=[[0, 0, 2], [0, 50, 10]]),
    ]
    return out


def tile(key, bounds, ge, content=None, children=(), refine="replace", bytes_=1000):
    return {"key": key, "bounds": bounds, "geometricError": ge, "refine": refine,
            "content": None if content is None else {"id": content, "bytes": bytes_}, "children": list(children)}


def quad_tree(levels, size=1000.0, origin=(0.0, 0.0), z=(0.0, 20.0), refine="replace", empty_root=False):
    """A quadtree of `levels` over a square: every tile a content (the root none if `empty_root`), the error halving."""
    tiles = []

    def make(x0, y0, s, level, ge):
        at = len(tiles)
        has = not (empty_root and level == 0)
        tiles.append(tile(at * 7 + 3, [[x0, y0, z[0]], [x0 + s, y0 + s, z[1]]], ge if level < levels - 1 else 0.0,
                          content=100 + at if has else None, refine=refine))
        if level < levels - 1:
            h = s / 2
            kids = [make(x0, y0, h, level + 1, ge / 2), make(x0 + h, y0, h, level + 1, ge / 2),
                    make(x0, y0 + h, h, level + 1, ge / 2), make(x0 + h, y0 + h, h, level + 1, ge / 2)]
            tiles[at]["children"] = kids
        return at
    make(origin[0], origin[1], size, 0, 64.0)
    return {"tiles": tiles, "roots": [0]}


def contents_of(h):
    return [t["content"]["id"] for t in h["tiles"] if t["content"] is not None]


def selection_case(name, h, c, ready, limit=16.0, vp=VP, ex=NONE):
    return {"name": name, "hierarchy": h, "camera": c, "viewport": vp, "exaggeration": ex, "maxScreenError": limit,
            "ready": sorted(ready), "expect": select(h, c, vp, ex, limit, set(ready))}


def selections():
    q3 = quad_tree(3)
    all3 = contents_of(q3)
    near = orbit([200, 200, 10], 225, 35, 120, persp())
    far = orbit([500, 500, 10], 45, 40, 20000, persp())
    mid = orbit([500, 500, 10], 45, 40, 2500, persp())
    root_only = [100]
    level1 = [100, 101, 106, 111, 116]
    out = [
        selection_case("Uzaktan: yalnız kök", q3, far, all3),
        selection_case("Orta uzaklık: birinci düzey", q3, mid, all3),
        selection_case("Yakından: köşede en ince", q3, near, all3),
        selection_case("Hiçbiri hazır değil: kök istenir", q3, near, []),
        selection_case("Yalnız kök hazır: kök kalır, çocuklar istenir", q3, near, root_only),
        selection_case("Birinci düzey hazır: ince olanlar istenir", q3, near, level1),
        selection_case("Bir çocuk eksik: ebeveyn yerinde", q3, near, [c for c in all3 if c != 102]),
        selection_case("Görüş dışı karolar elenir", q3, orbit([-3000, -3000, 0], 45, 20, 200, persp()), all3),
        selection_case("Sınır büyük: kaba kalır", q3, near, all3, limit=500.0),
        selection_case("Sınır küçük: hep ince", q3, near, all3, limit=0.5),
        selection_case("Katkılı inceltme: ebeveyn ve çocuklar", quad_tree(3, refine="add"), near,
                       contents_of(quad_tree(3, refine="add"))),
        selection_case("İçeriksiz kök hep iner", quad_tree(3, empty_root=True), far, contents_of(quad_tree(3, empty_root=True))),
        selection_case("İçeriksiz kök, çocuklar eksik", quad_tree(3, empty_root=True), mid, [101, 106]),
        selection_case("Ortografik üst: piksel boyu", q3, {"eye": [500, 500, 3000], "target": [500, 500, 0],
                                                           "up": [0.0, 0.0, 1.0], "projection": ortho(400, 5000)}, all3),
        selection_case("Ortografik geniş: kaba", q3, {"eye": [500, 500, 3000], "target": [500, 500, 0],
                                                      "up": [0.0, 0.0, 1.0], "projection": ortho(20000, 5000)}, all3),
        selection_case("Abartma hatayı büyütür", q3, mid, all3, ex={"factor": 5.0, "datum": 0.0}),
        selection_case("Dar pencere", q3, near, all3, vp={"width": 360.0, "height": 640.0}),
        selection_case("Dört düzey, kısmi hazır", quad_tree(4), near, [c for c in contents_of(quad_tree(4)) if c % 3]),
        selection_case("Göz karonun içinde", q3, orbit([100, 100, 5], 30, 5, 3, persp(60, 0.1)), all3),
        selection_case("Yere paralel, ufka doğru", q3, {"eye": [500, -50, 15], "target": [500, 500, 15],
                                                         "up": [0.0, 0.0, 1.0], "projection": persp()}, all3),
    ]
    return out


def grid(n, size, origin, z, rng, part):
    """An n × n terrain grid (two triangles a cell, counterclockwise) over a square, heights from `z` and noise."""
    vs = []
    for j in range(n + 1):
        for i in range(n + 1):
            x = origin[0] + size * i / n
            y = origin[1] + size * j / n
            vs.append([x, y, round(z(x, y) + rng.random(), 3)])
    ts = []
    for j in range(n):
        for i in range(n):
            a = j * (n + 1) + i
            ts += [[a, a + 1, a + n + 2], [a, a + n + 2, a + n + 1]]
    return {"part": part, "vertices": vs, "triangles": ts, "smooth": True}


def chunk_case(name, meshes, limits, first_id=1):
    h, contents = chunk(meshes, limits, first_id)
    return {"name": name, "meshes": meshes, "limits": limits, "firstId": first_id,
            "expect": {"hierarchy": h, "contents": contents}}


def chunks():
    rng = random.Random(7)
    hill = lambda x, y: 100 + 0.02 * x + 3 * math.sin(x / 40) * math.cos(y / 55)  # noqa: E731
    small = grid(6, 120.0, (0.0, 0.0), hill, rng, 0)
    road = {"part": 1, "vertices": [[10.0, 50.0, 101.0], [110.0, 50.0, 103.0], [110.0, 58.0, 103.0], [10.0, 58.0, 101.0],
                                     [60.0, 50.0, 102.0], [60.0, 58.0, 102.0]],
            "triangles": [[0, 4, 5], [0, 5, 3], [4, 1, 2], [4, 2, 5]]}
    far_rng = random.Random(11)
    tm = grid(8, 2400.0, (512000.0, 4401000.0), lambda x, y: 1200 + 0.01 * (x - 512000), far_rng, 0)
    same = {"part": 2, "vertices": [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0]],
            "triangles": [[0, 1, 2], [0, 1, 2], [0, 1, 2]], "smooth": True}
    broken = {"part": 3, "vertices": [[0.0, 0.0, 0.0], [5.0, 0.0, 1.0], [0.0, 5.0, 2.0]],
              "triangles": [[0, 1, 2], [0, 1, 9]], "smooth": True}
    return [
        chunk_case("Tek karo: sınırlar içinde", [small], {"maxTriangles": 1000, "maxExtent": 1000.0}),
        chunk_case("Üçgen sınırıyla bölünür", [small], {"maxTriangles": 16, "maxExtent": 1000.0}),
        chunk_case("Boy sınırıyla bölünür", [small], {"maxTriangles": 1000, "maxExtent": 50.0}),
        chunk_case("Arazi ve yol: parçalar ayrı aralıklarda", [small, road], {"maxTriangles": 24, "maxExtent": 1000.0}, 40),
        chunk_case("TM koordinatlarında 2,4 km", [tm], {"maxTriangles": 32768, "maxExtent": 1000.0}),
        chunk_case("Aynı ağırlık merkezli üçgenler bölünmez", [same], {"maxTriangles": 1, "maxExtent": 1000.0}),
        chunk_case("Dışarı işaret eden üçgen atlanır", [broken], {"maxTriangles": 10, "maxExtent": 1000.0}),
        chunk_case("Boş", [], {"maxTriangles": 10, "maxExtent": 1000.0}),
    ]


def chunked_selection():
    """A chunked terrain selected from three places: the tiles drawn and asked for, every content ready or none."""
    rng = random.Random(3)
    mesh = grid(32, 1600.0, (0.0, 0.0), lambda x, y: 50 + 0.03 * x + 6 * math.sin(y / 90), rng, 0)
    h, contents = chunk([mesh], {"maxTriangles": 128, "maxExtent": 1000.0}, 1)
    ids = [c["id"] for c in contents]
    return [
        selection_case("Bölünmüş arazi, yakından", h, orbit([300, 300, 60], 200, 25, 150, persp()), ids),
        selection_case("Bölünmüş arazi, hiçbiri hazır değil", h, orbit([300, 300, 60], 200, 25, 150, persp()), []),
        selection_case("Bölünmüş arazi, üstten", h, {"eye": [800, 800, 3000], "target": [800, 800, 0],
                                                     "up": [0.0, 0.0, 1.0], "projection": persp()}, ids),
    ]


def main():
    data = {"format": "kentos.scene3d-cases", "version": 1,
            "note": "scripts/fixtures/scene3d_cases.py yazar; kamera, frustum, ekran uzayı hatası (CesiumJS'in kuralı), "
                    "3D Tiles 1.1'in inceltmesi ve karolara bölme ADR 0910 §3'ün kurallarından, KentOS kodu olmadan.",
            "cameras": cameras(), "selections": selections() + chunked_selection(), "chunks": chunks()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text(encoding="utf-8") != text:
            raise SystemExit(f"{OUT} güncel değil: python3 {Path(__file__).name}")
        print(f"{OUT.name}: güncel")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.name}: {len(data['cameras'])} kamera, {len(data['selections'])} seçim, {len(data['chunks'])} bölme durumu")


if __name__ == "__main__":
    main()
