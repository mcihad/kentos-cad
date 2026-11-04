#!/usr/bin/env python3
"""The shared cases of `road.terrains.cloud` (plan/road.md Y15, docs/adr/0928; a terrain's points from point
clouds).

The cloud is this script's own (`fixtures/pointcloud/v1/files/road-ground.las`, LAS 1.2 point format 1 written with
laspy): a 40 × 30 m terrain z = 100 + 0.02 x′ + 0.01 y′ + 0.25 sin(x′ / 5) sampled on a jittered 0.7 m lattice as ground
(class 2), the last returns under trees (class 4 first returns 3 m up), a building (class 6, no ground under it), a
bridge deck strip (class 17), low vegetation (class 3) and low noise (class 7). What a case expects is read from laspy's
points as ADR 0928 says: the classes and returns kept (first: n ≤ 1, last: n = count or count 0, single: count ≤ 1), the
area's even-odd inside (a point on its edge is inside), the grid's cells ⌊x/c⌋, ⌊y/c⌋ and their nearest point to the
centre (a tie the first read), a steep cell's lowest and highest, every point within the reach of a breakline, the
coverage of the area (cells whose centre is inside) or of the cell box, the density, and the error of the points left out
against a Delaunay triangulation of the kept ones made here (Bowyer–Watson; the lattice's jitter makes it unique). With
breaklines or another surface's points the triangulation is constrained or shared and the error is `$any`. No KentOS
code; the classes, cells and tolerances are the cases' own.

    uv run --no-project --with 'laspy==2.7.0' --with numpy python3 scripts/fixtures/road_cloud_terrain_cases.py --las
    uv run --no-project --with 'laspy==2.7.0' --with numpy python3 scripts/fixtures/road_cloud_terrain_cases.py [--check]
"""

import copy
import datetime
import json
import math
import random
import sys
from pathlib import Path

import laspy
import numpy as np

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
LAS = ROOT / "fixtures/pointcloud/v1/files/road-ground.las"
W = 1e-6
ANY = "$any"
X0, Y0 = 1000.0, 2000.0
SCALE, OFFSET = 0.001, (1000.0, 2000.0, 100.0)


def ground_z(x, y):
    return 100.0 + 0.02 * (x - X0) + 0.01 * (y - Y0) + 0.25 * math.sin((x - X0) / 5.0)


def write_las():
    rnd = random.Random(913)
    pts = []  # (x, y, z, class, return number, number of returns)
    building = lambda x, y: 1025.0 <= x <= 1032.0 and 2005.0 <= y <= 2012.0  # noqa: E731
    for j in range(43):
        for i in range(57):
            x = X0 + 0.35 + 0.7 * i + rnd.uniform(-0.2, 0.2)
            y = Y0 + 0.35 + 0.7 * j + rnd.uniform(-0.2, 0.2)
            if building(x, y):
                pts.append((x, y, 106.0 + rnd.uniform(0, 0.05), 6, 1, 1))
                continue
            z = ground_z(x, y)
            r = rnd.random()
            if r < 0.15:
                pts.append((x + 0.05, y + 0.05, z + 3.0 + rnd.uniform(0, 1), 4, 1, 2))
                pts.append((x, y, z, 2, 2, 2))
            elif r < 0.20:
                pts.append((x, y, z + 0.3, 3, 1, 1))
            else:
                pts.append((x, y, z, 2, 1, 1))
    for k in range(40):
        pts.append((X0 + 1.0 * k, 2020.5, 104.2, 17, 1, 1))
    for k in range(5):
        pts.append((X0 + 3.0 + 7.0 * k, 2002.0, 80.0, 7, 1, 1))
    header = laspy.LasHeader(point_format=1, version="1.2")
    header.scales = np.array([SCALE] * 3)
    header.offsets = np.array(OFFSET)
    header.creation_date = datetime.date(2026, 11, 3)
    las = laspy.LasData(header)
    q = lambda v, o: np.round((np.array(v) - o) / SCALE).astype(np.int32)  # noqa: E731
    las.X = q([p[0] for p in pts], OFFSET[0])
    las.Y = q([p[1] for p in pts], OFFSET[1])
    las.Z = q([p[2] for p in pts], OFFSET[2])
    las.classification = np.array([p[3] for p in pts], dtype=np.uint8)
    las.return_number = np.array([p[4] for p in pts], dtype=np.uint8)
    las.number_of_returns = np.array([p[5] for p in pts], dtype=np.uint8)
    las.write(str(LAS))


def read_las():
    las = laspy.read(str(LAS))
    h = las.header
    X, Y, Z = np.asarray(las.X), np.asarray(las.Y), np.asarray(las.Z)
    out = []
    for k in range(len(X)):
        out.append((float(X[k]) * h.scales[0] + h.offsets[0], float(Y[k]) * h.scales[1] + h.offsets[1],
                    float(Z[k]) * h.scales[2] + h.offsets[2], int(las.classification[k]), int(las.return_number[k]),
                    int(las.number_of_returns[k])))
    mins, maxs = h.mins, h.maxs
    return out, [float(mins[0]), float(mins[1]), float(mins[2]), float(maxs[0]), float(maxs[1]), float(maxs[2])]


# ── The drawing ──────────────────────────────────────────────────────────────────────────────────────────────────
def node(id_, name, **kw):
    n = {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
         "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}
    n.update(kw)
    return n


AREA = [[1004.0, 2003.0], [1030.0, 2004.0], [1034.0, 2018.0], [1018.0, 2027.0], [1003.0, 2019.0]]
BREAK = [[1000.0, 2015.0], [1020.0, 2015.0], [1040.0, 2015.0]]
UID = "0192f5b0-7c3e-7d4a-9b1e-{:012d}"


def setup(work="road"):
    points, bounds = read_las()
    layers = [node("0", "0"), node("arazi", "Arazi"), node("kirik", "Kırık hatlar"), node("alan", "Alan"),
              node("bulut", "Bulut"), node("zemin", "Zemin noktaları"), node("gizli", "Gizli", visible=False),
              node("kilitli", "Kilitli", locked=True),
              node("grup", "Yol", type="group", children=[node("yol-zemin", "Yol zemini")])]
    entities = []

    def add(e):
        e["id"] = len(entities) + 1
        entities.append(e)

    for x, y in ((990.0, 1990.0), (1050.0, 1990.0), (1050.0, 2040.0), (990.0, 2040.0)):
        add({"kind": "point", "layerId": "arazi", "attrs": {}, "p": {"x": x, "y": y}, "z": ground_z(x, y)})
    add({"kind": "polyline", "layerId": "kirik", "attrs": {}, "pts": [{"x": p[0], "y": p[1]} for p in BREAK],
         "zs": [ground_z(*p) for p in BREAK]})
    add({"kind": "polygon", "layerId": "alan", "attrs": {}, "pts": [{"x": p[0], "y": p[1]} for p in AREA]})
    add({"kind": "polyline", "layerId": "alan", "attrs": {}, "pts": [{"x": p[0], "y": p[1]} for p in AREA[:3]]})
    add({"kind": "polygon", "layerId": "alan", "attrs": {}, "pts": [{"x": p[0], "y": p[1]} for p in AREA[:3]],
         "bulges": [0.0, 0.3, 0.0]})
    add({"kind": "pointcloud", "layerId": "bulut", "attrs": {},
         "sources": [{"file": "$fixture:pointcloud/v1/files/road-ground.las", "format": "las", "count": len(points),
                      "bounds": bounds}],
         "bounds": bounds, "count": len(points), "srid": 5254, "style": {"render": "classification", "size": 2}})
    # A cloud whose file is gone: its source is not read, with a warning.
    add({"kind": "pointcloud", "layerId": "bulut", "attrs": {},
         "sources": [{"file": "$fixture:pointcloud/v1/files/yok.las", "format": "las", "count": 10,
                      "bounds": bounds}],
         "bounds": bounds, "count": 10, "srid": 5254, "style": {"render": "classification", "size": 2}})
    settings = {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                "plotScale": 1000.0, "workspace": "cad", "cadWork": work, "drawingFont": "barlow"}
    if work == "road":
        settings["road"] = {"surfaces": [{"id": "t-1", "name": "Mevcut zemin", "points": {"layers": ["arazi"]}}]}
    return {"format": "kentos.document", "version": 2, "name": "Buluttan arazi", "settings": settings,
            "origin": {"x": 0.0, "y": 0.0}, "layers": layers, "activeLayer": "0", "entities": entities,
            "uids": [UID.format(i) for i in range(1, len(entities) + 1)], "styles": {"items": [], "categories": []}}


CLOUD = UID.format(9)
GONE = UID.format(10)
CORNERS = [(x, y, ground_z(x, y)) for x, y in ((990.0, 1990.0), (1050.0, 1990.0), (1050.0, 2040.0), (990.0, 2040.0))]
POINT = UID.format(1)
AREA_UID, OPEN_UID, ARC_UID = UID.format(6), UID.format(7), UID.format(8)


# ── The reference ────────────────────────────────────────────────────────────────────────────────────────────────
def inside_ring(ring, x, y):
    """Even-odd with a point on an edge inside (ADR 0928: the area's test)."""
    n = len(ring)
    for i in range(n):
        (ax, ay), (bx, by) = ring[i], ring[(i + 1) % n]
        cross = (bx - ax) * (y - ay) - (by - ay) * (x - ax)
        if cross == 0.0 and min(ax, bx) <= x <= max(ax, bx) and min(ay, by) <= y <= max(ay, by):
            return True
    odd = False
    for i in range(n):
        (ax, ay), (bx, by) = ring[i], ring[(i + 1) % n]
        if (ay <= y < by) or (by <= y < ay):
            if x < ax + (y - ay) / (by - ay) * (bx - ax):
                odd = not odd
    return odd


def returned(rule, n, c):
    return {"all": True, "first": n <= 1, "last": n == c or c == 0, "single": c <= 1}[rule]


def chosen(points, classes, returns, ring):
    out = []
    for x, y, z, cl, n, c in points:
        if cl not in classes or not returned(returns, n, c):
            continue
        if ring is not None and not inside_ring(ring, x, y):
            continue
        out.append((x, y, z))
    return out


def near_line(lines, reach, x, y):
    r2 = reach * reach
    for line in lines:
        for a, b in zip(line, line[1:]):
            dx, dy = b[0] - a[0], b[1] - a[1]
            len2 = dx * dx + dy * dy
            t = min(max(((x - a[0]) * dx + (y - a[1]) * dy) / len2, 0.0), 1.0) if len2 > 0 else 0.0
            ex, ey = x - (a[0] + t * dx), y - (a[1] + t * dy)
            if ex * ex + ey * ey <= r2:
                return True
    return False


def thin(pts, cell, ztol, near, lines):
    cells = {}
    near_kept = []
    for order, p in enumerate(pts):
        x, y, z = p
        key = (math.floor(x / cell), math.floor(y / cell))
        cx, cy = (key[0] + 0.5) * cell, (key[1] + 0.5) * cell
        d = (x - cx) * (x - cx) + (y - cy) * (y - cy)
        c = cells.get(key)
        if c is None:
            cells[key] = {"best": (order, p), "d": d, "low": (order, p), "high": (order, p)}
        else:
            if d < c["d"]:
                c["best"], c["d"] = (order, p), d
            if z < c["low"][1][2]:
                c["low"] = (order, p)
            if z > c["high"][1][2]:
                c["high"] = (order, p)
        if near is not None and near_line(lines, near, x, y):
            near_kept.append((order, p))
    kept = {}
    for c in cells.values():
        kept.setdefault(c["best"][0], (c["best"][1], 0))
        if ztol is not None and c["high"][1][2] - c["low"][1][2] > ztol:
            kept.setdefault(c["low"][0], (c["low"][1], 1))
            kept.setdefault(c["high"][0], (c["high"][1], 1))
    for order, p in near_kept:
        kept.setdefault(order, (p, 2))
    ordered = sorted(kept.items())
    return {"points": [(o, v[0]) for o, v in ordered], "why": [v[1] for _, v in ordered], "read": len(pts),
            "cells": set(cells)}


def coverage(t, cell, ring):
    if ring is not None:
        xs, ys = [p[0] for p in ring], [p[1] for p in ring]
        cells = occupied = 0
        for i in range(math.floor(min(xs) / cell), math.floor(max(xs) / cell) + 1):
            for j in range(math.floor(min(ys) / cell), math.floor(max(ys) / cell) + 1):
                if inside_ring_centre(ring, (i + 0.5) * cell, (j + 0.5) * cell):
                    cells += 1
                    occupied += (i, j) in t["cells"]
        n = len(ring)
        area = abs(sum(ring[i][0] * ring[(i + 1) % n][1] - ring[(i + 1) % n][0] * ring[i][1] for i in range(n))) / 2.0
    else:
        keys = t["cells"]
        i0, i1 = min(k[0] for k in keys), max(k[0] for k in keys)
        j0, j1 = min(k[1] for k in keys), max(k[1] for k in keys)
        cells = (i1 - i0 + 1) * (j1 - j0 + 1)
        occupied = len(keys)
        area = float(cells) * cell * cell
    return {"cells": cells, "occupied": occupied, "area": area, "share": occupied / cells if cells else 0.0,
            "density": t["read"] / area if area > 0 else 0.0}


def inside_ring_centre(ring, x, y):
    """The coverage's cell test: even-odd alone (`cloud_ground::coverage`)."""
    odd = False
    n = len(ring)
    for i in range(n):
        (ax, ay), (bx, by) = ring[i], ring[(i + 1) % n]
        if (ay > y) != (by > y) and x < ax + (y - ay) / (by - ay) * (bx - ax):
            odd = not odd
    return odd


def delaunay(points):
    """Bowyer–Watson over a super triangle; the triangles of the points' indices."""
    xs, ys = [p[0] for p in points], [p[1] for p in points]
    mx, my = (min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2
    span = max(max(xs) - min(xs), max(ys) - min(ys)) * 1e5 + 1
    pts = [(p[0] - mx, p[1] - my) for p in points] + [(-span, -span), (span, -span), (0.0, span)]
    n = len(points)
    tris = [(n, n + 1, n + 2)]

    def circum(t):
        (ax, ay), (bx, by), (cx, cy) = pts[t[0]], pts[t[1]], pts[t[2]]
        d = 2 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by))
        ux = ((ax * ax + ay * ay) * (by - cy) + (bx * bx + by * by) * (cy - ay) + (cx * cx + cy * cy) * (ay - by)) / d
        uy = ((ax * ax + ay * ay) * (cx - bx) + (bx * bx + by * by) * (ax - cx) + (cx * cx + cy * cy) * (bx - ax)) / d
        return ux, uy, (ax - ux) ** 2 + (ay - uy) ** 2

    circles = {tris[0]: circum(tris[0])}
    for k in range(n):
        px, py = pts[k]
        bad = [t for t in tris if (px - circles[t][0]) ** 2 + (py - circles[t][1]) ** 2 < circles[t][2]]
        edges = {}
        for t in bad:
            for e in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])):
                key = tuple(sorted(e))
                edges[key] = edges.get(key, 0) + 1
        bad_set = set(bad)
        tris = [t for t in tris if t not in bad_set]
        for t in bad:
            del circles[t]
        for (a, b), count in edges.items():
            if count == 1:
                t = (a, b, k)
                tris.append(t)
                circles[t] = circum(t)
    tris = [t for t in tris if max(t) < n]
    # A whole triangulation of n points with h on their hull has 2n − 2 − h triangles: none of the hull's is lost.
    hull = convex_hull(points)
    if len(tris) != 2 * n - 2 - len(hull):
        sys.exit(f"Delaunay {len(tris)} üçgen, {2 * n - 2 - len(hull)} olmalı")
    return tris


def convex_hull(points):
    """The hull's corners (monotone chain), collinear ones left out."""
    pts = sorted(set((p[0], p[1]) for p in points))

    def turn(o, a, b):
        return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])

    lower, upper = [], []
    for p in pts:
        while len(lower) >= 2 and turn(lower[-2], lower[-1], p) <= 0:
            lower.pop()
        lower.append(p)
    for p in reversed(pts):
        while len(upper) >= 2 and turn(upper[-2], upper[-1], p) <= 0:
            upper.pop()
        upper.append(p)
    return lower[:-1] + upper[:-1]


def residuals(kept_pts, left):
    tris = delaunay(kept_pts)
    count = outside = 0
    worst = total = 0.0
    for x, y, z in left:
        h = None
        for a, b, c in tris:
            (ax, ay, az), (bx, by, bz), (cx, cy, cz) = kept_pts[a], kept_pts[b], kept_pts[c]
            d = (by - cy) * (ax - cx) + (cx - bx) * (ay - cy)
            u = ((by - cy) * (x - cx) + (cx - bx) * (y - cy)) / d
            v = ((cy - ay) * (x - cx) + (ax - cx) * (y - cy)) / d
            w = 1 - u - v
            if min(u, v, w) >= -1e-12:
                h = u * az + v * bz + w * cz
                break
        if h is None:
            outside += 1
            continue
        e = abs(z - h)
        count += 1
        worst = max(worst, e)
        total += e * e
    return count, outside, worst, math.sqrt(total / count) if count else 0.0


# ── The cases ────────────────────────────────────────────────────────────────────────────────────────────────────
def completed(output, warns=()):
    return {"status": "completed", "output": output, "warnings": list(warns)}


def failed(code, message, path=None):
    e = {"code": code, "message": message}
    if path:
        e["path"] = path
    return {"status": "failed", "error": e}


UNCHANGED = {"canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
STEPPED = {"canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}
CONFLICT = {"status": "conflict", "error": {
    "code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu "
    "çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision", "revision": "$current"}}
BAD_REVISION = failed("invalid_revision", "Beklenen sürüm “on” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı "
                      "yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision")
HIDDEN = {"code": "layer_hidden", "message": "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}


def above(classes):
    return {"code": "cloud_class_above_ground", "message": f"Sınıf {', '.join(str(c) for c in classes)} zemin değil "
            "(bitki, bina ya da gürültü); yüzey onların noktalarını zemin sayar.", "path": "classes"}


BASE = {"clouds": [CLOUD], "classes": [2], "cell": 2.0, "layerId": "zemin", "name": "Lidar zemini"}


def answer(inp, points, measured=True):
    """The output `inp` gives, and whether the error is the reference's own (no breakline, no other point)."""
    classes = sorted(set(inp["classes"]))
    ring = AREA if inp.get("area") == AREA_UID else None
    pts = chosen(points, classes, inp.get("returns", "all"), ring)
    lines = []
    if inp.get("near") is not None:
        lines = [BREAK] if inp.get("breaklines") else []
    t = thin(pts, inp["cell"], inp.get("zTolerance"), inp.get("near"), lines)
    cov = coverage(t, inp["cell"], ring)
    new = "name" in inp
    record = {"layer": inp["layerId"], "clouds": list(dict.fromkeys(inp["clouds"])), "classes": classes, "returns": inp.get("returns", "all"),
              "cell": inp["cell"], "read": t["read"], "kept": len(t["points"]), "coverage": cov["share"],
              "density": cov["density"]}
    for k in ("area", "zTolerance", "near"):
        if inp.get(k) is not None:
            record[k] = inp[k]
    # The surface's other points: the existing one's corner points, a new one's breakline vertices. With breaklines
    # the triangulation is constrained: its hull (and so what is outside) is the points', the error is `$any`.
    extra = CORNERS if not new else ([tuple(p) + (ground_z(*p),) for p in BREAK] if inp.get("breaklines") else [])
    kept_set = {o for o, _ in t["points"]}
    left = [p for o, p in enumerate(pts) if o not in kept_set]
    count, outside, worst, rms = residuals([p for _, p in t["points"]] + extra, left)
    if count:
        own = not (new and inp.get("breaklines"))
        record["errorMax"], record["errorRms"] = (worst, rms) if own else (ANY, ANY)
    if outside:
        record["outside"] = outside
    pieces = (len(t["points"]) + 9999) // 10000
    surface = ({"id": "t-2", "name": inp["name"], "state": "current", "digest": ANY, "built": ANY, "figures": ANY}
               if new else {"id": "t-1", "name": "Mevcut zemin", "state": "new", "digest": ANY, "figures": ANY})
    scanned = len(points)
    return {"cloud": record, "scanned": scanned, "byCell": t["why"].count(0), "bySlope": t["why"].count(1),
            "byLine": t["why"].count(2), "cells": cov["cells"], "occupied": cov["occupied"], "area": cov["area"],
            "uids": [ANY] * pieces, "surface": surface, "revision": "$current"}, True


def step(op, inp, result, expect):
    return {"op": op, "input": inp, "result": result, "expect": expect, "within": W}


def cases():
    points, _ = read_las()
    out = []

    def ok(name, inp, warns=(), plan=False):
        output, _ = answer(inp, points)
        steps = [step("plan", inp, completed(output, warns), UNCHANGED)] if plan else []
        if not plan:
            steps.append(step("execute", inp, completed(output, warns), STEPPED))
        out.append({"name": name, "steps": steps})

    def no(name, inp, result, setup_=None):
        c = {"name": name, "steps": [step("execute", inp, result, UNCHANGED)]}
        if setup_ is not None:
            c["setup"] = setup_
        out.append(c)

    ok("Zemin sınıfı, 2 m hücre, yeni yüzey", BASE)
    ok("Yalnız son dönüşler", {**BASE, "returns": "last"})
    ok("Yalnız ilk dönüşler", {**BASE, "returns": "first"})
    ok("Yalnız tek dönüşlüler", {**BASE, "returns": "single"})
    ok("Alçak bitkiyle, uyarıyla", {**BASE, "classes": [3, 2, 2]}, [above([3])])
    ok("Bina ve gürültüyle, uyarıyla", {**BASE, "classes": [2, 6, 7]}, [above([6, 7])])
    ok("Alanın içinde", {**BASE, "area": AREA_UID})
    ok("Dik hücrenin en alçağı ve en yükseği", {**BASE, "cell": 3.0, "zTolerance": 0.1})
    ok("Kırık hattın eriminde her nokta", {**BASE, "near": 0.5, "breaklines": {"layers": ["kirik"]}})
    ok("Kırık hatsız erim hiçbir şey eklemez", {**BASE, "near": 0.5})
    ok("Var olan yüzeye", {k: v for k, v in BASE.items() if k != "name"} | {"surface": "Mevcut zemin"})
    ok("Büyük hücre", {**BASE, "cell": 5.0})
    ok("Küçük hücre", {**BASE, "cell": 0.5})
    ok("Planı yazmaz", BASE, plan=True)
    ok("Gizli katmana uyarıyla", {**BASE, "layerId": "gizli"}, [HIDDEN])
    no("Köprü tabliyesi zemine katılmaz", {**BASE, "classes": [2, 17]},
       failed("cloud_class_not_ground", "Sınıf köprü tabliyesi (17) zemin değil; yer yüzeyiyle birleştirilmez. "
              "Sınıflardan çıkarın.", "classes"))
    no("Üst yapı zemine katılmaz", {**BASE, "classes": [19]},
       failed("cloud_class_not_ground", "Sınıf üst yapı (19) zemin değil; yer yüzeyiyle birleştirilmez. Sınıflardan "
              "çıkarın.", "classes"))
    no("Sınıf yok", {**BASE, "classes": []},
       failed("invalid_cloud_terrain", "En az bir sınıf verin (ASPRS'de 2 zemin).", "classes"))
    no("Bulut yok", {**BASE, "clouds": []}, failed("invalid_cloud_terrain", "En az bir nokta bulutu verin.", "clouds"))
    no("Bulut değil", {**BASE, "clouds": [POINT]},
       failed("not_a_pointcloud", "Verilen nesne bir nokta bulutu değil; bir nokta bulutunun kimliğini verin.",
              "clouds[0]"))
    no("Çizimde olmayan bulut", {**BASE, "clouds": [UID.format(99)]},
       failed("entity_not_found", f"“{UID.format(99)}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin "
              "olabilir. Var olan bir nesnenin kimliğini verin.", "clouds[0]"))
    both = "Ya surface (projedeki yüzey) ya da name (yeni yüzey) verin, ikisini birden değil."
    no("Yüzey ve ad birlikte", {**BASE, "surface": "t-1"}, failed("invalid_cloud_terrain", both, "surface"))
    no("Ne yüzey ne ad", {k: v for k, v in BASE.items() if k != "name"}, failed("invalid_cloud_terrain", both,
                                                                                 "surface"))
    no("Var olan yüzeye kırık hat verilmez", {k: v for k, v in BASE.items() if k != "name"} | {
        "surface": "t-1", "breaklines": {"layers": ["kirik"]}},
       failed("invalid_cloud_terrain", "breaklines ve boundary yalnız yeni yüzeyde verilir; var olan yüzeyin kendi "
              "tanımı kullanılır.", "breaklines"))
    no("Projede olmayan yüzey", {k: v for k, v in BASE.items() if k != "name"} | {"surface": "Tasarım"},
       failed("unknown_terrain", "“Tasarım” kimlikli ya da adlı yüzey projede yok.", "surface"))
    no("Hücre sıfır", {**BASE, "cell": 0.0},
       failed("invalid_cloud_terrain", "Seyreltme: seyreltme hücresi sıfırdan büyük, sonlu bir uzunluk olmalı.",
              "cell"))
    no("Eksi kot toleransı", {**BASE, "zTolerance": -0.1},
       failed("invalid_cloud_terrain", "Seyreltme: kot toleransı sıfırdan büyük, sonlu olmalı.", "zTolerance"))
    no("Sıfır erim", {**BASE, "near": 0.0},
       failed("invalid_cloud_terrain", "Seyreltme: kırık hat erimi sıfırdan büyük, sonlu olmalı.", "near"))
    no("Açık çizgi alan değil", {**BASE, "area": OPEN_UID},
       failed("invalid_cloud_terrain", "Alan kapalı bir çoklu çizgi ya da alan olmalı.", "area"))
    no("Yaylı alan alınmaz", {**BASE, "area": ARC_UID},
       failed("invalid_cloud_terrain", "Alanın kenarları düz olmalı; yaylı kenar alınmaz.", "area"))
    no("Su sınıfının noktası yok", {**BASE, "classes": [9]},
       failed("cloud_ground_empty", "Bulutlarda istenen sınıf ve dönüşten, alanın içinde nokta yok; sınıfları, "
              "dönüşü ya da alanı denetleyin.", "classes"))
    no("Kilitli katman", {**BASE, "layerId": "kilitli"},
       failed("layer_locked", "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı "
                              "etkinleştirin.", "layerId"))
    no("Katman grubu", {**BASE, "layerId": "grup"},
       failed("not_a_layer", "“Yol” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.",
              "layerId"))
    no("Yol projesi değil", BASE, failed("not_a_road_project", "Arazi yüzeyi yalnız yol projesinde (KentOS Yol) olur; "
                                         "bu çizim yol projesi değil."), setup_=None)
    out[-1]["setup"] = "$map"
    ok("Kaynağı okunamayan bulut uyarıyla", {**BASE, "clouds": [CLOUD, GONE]},
       [{"code": "cloud_source_unreadable", "message": ANY, "path": "clouds[1].sources[0]"}])
    no("Katman yok", {**BASE, "layerId": "yok"},
       failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.",
              "layerId"))
    no("Aynı adda yüzey", {**BASE, "name": "mevcut ZEMIN"},
       failed("invalid_terrain", "Yüzey kurallarına uymuyor: bu adda başka bir yüzey var.", "name"))
    no("Tek hücrenin tek noktası yüzey kurmaz", {**BASE, "cell": 100.0},
       failed("terrain_unbuildable", "Kaynaklardan yüzey kurulamıyor: kotlu yer üçten az ya da hepsi bir doğru "
              "üstünde."))
    no("Geçersiz beklenen sürüm", {**BASE, "expectedRevision": "on"}, BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {**BASE, "expectedRevision": "41"}, CONFLICT)
    return out


def document():
    road = setup()
    out = cases()
    for c in out:
        if c.get("setup") == "$map":
            c["setup"] = setup(work="map")
    return {"format": "kentos.command-cases", "version": 1, "command": "road.terrains.cloud", "commandVersion": 1,
            "title": "Buluttan arazi: zemin noktaları, seyreltme, kapsama ve hata",
            "note": "scripts/fixtures/road_cloud_terrain_cases.py'den; bulutun noktaları laspy'nin, seyreltme, kapsama "
                    "ve hata ADR 0928'ün kuralıyla, KentOS kodu olmadan.",
            "setup": road, "cases": out}


def main():
    if "--las" in sys.argv:
        write_las()
        print(LAS)
        return
    doc = document()
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    path = OUT / "road.terrains.cloud.json"
    if "--check" in sys.argv:
        if path.read_text() != text:
            sys.exit(f"{path} güncel değil")
        print(f"{len(doc['cases'])} durum güncel")
        return
    path.write_text(text)
    print(f"{path}: {len(doc['cases'])} durum")


if __name__ == "__main__":
    main()
