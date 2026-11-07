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
    uv run --no-project --with 'laspy==2.7.0' --with numpy python3 scripts/fixtures/road_cloud_terrain_cases.py --heights-las
    uv run --no-project --with 'laspy==2.7.0' --with numpy python3 scripts/fixtures/road_cloud_terrain_cases.py --tm36-las
    uv run --no-project --with 'laspy==2.7.0' --with numpy --with pyproj python3 \\
        scripts/fixtures/road_cloud_terrain_cases.py [--check]
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


# Docs/adr/1029: the same points in two files that name their heights' vertical system, one by a GeoTIFF key
# (VerticalCSTypeGeoKey 4096 = EPSG 5775, Antalya height) and one by a compound WKT (VERT_CS "EGM2008 height", EPSG 3855).
LAS_ANTALYA = ROOT / "fixtures/pointcloud/v1/files/road-ground-antalya.las"
LAS_EGM = ROOT / "fixtures/pointcloud/v1/files/road-ground-egm2008.las"
TM36 = (
    'PROJCS["TUREF / TM36",GEOGCS["TUREF",DATUM["Turkish_National_Reference_Frame",SPHEROID["GRS 1980",6378137,'
    '298.257222101,AUTHORITY["EPSG","7019"]],AUTHORITY["EPSG","1057"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],'
    'UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]],AUTHORITY["EPSG","5252"]],'
    'PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",36],'
    'PARAMETER["scale_factor",1],PARAMETER["false_easting",500000],PARAMETER["false_northing",0],'
    'UNIT["metre",1,AUTHORITY["EPSG","9001"]],AUTHORITY["EPSG","5256"]]'
)
EGM_WKT = (
    f'COMPD_CS["TUREF / TM36 + EGM2008 height",{TM36},VERT_CS["EGM2008 height",VERT_DATUM["EGM2008 geoid",2005,'
    'AUTHORITY["EPSG","1027"]],UNIT["metre",1,AUTHORITY["EPSG","9001"]],AXIS["Gravity-related height",UP],'
    'AUTHORITY["EPSG","3855"]]]'
)


def write_heights_las():
    las = laspy.read(str(LAS))
    keys = np.array([1, 1, 0, 3, 1024, 0, 1, 1, 3072, 0, 1, 5256, 4096, 0, 1, 5775], dtype="<u2").tobytes()
    las.header.vlrs.append(laspy.VLR("LASF_Projection", 34735, "GeoKeyDirectoryTag", keys))
    las.write(str(LAS_ANTALYA))
    las = laspy.read(str(LAS))
    las.header.vlrs.append(laspy.VLR("LASF_Projection", 2112, "OGC Coordinate System WKT", EGM_WKT.encode() + b"\0"))
    las.write(str(LAS_EGM))


# Docs/adr/1032: the same points 453.6 km east and 4286.7 km north, TUREF / TM36 near 38.75° N 35.47° E, inside
# geoid_cases.py's synthetic TG geoid (fixtures/geoid/v1/tg.gtx): the file names no vertical system; the cases say its
# heights are ellipsoidal and turn them by the geoid, H = h − N.
LAS_TM36 = ROOT / "fixtures/pointcloud/v1/files/road-ground-tm36.las"
DX, DY = 453600.0, 4286700.0
TG = "25956004773d9d08ff496a2ee7f27ba02aad4bd00d7d7203afdbe619e95c35f8"
PROJECT_TG = {"kind": "orthometric", "verticalSrid": 5775, "geoid": {"name": "TG-20 (sentetik)", "sha256": TG}}
ELL = {"kind": "ellipsoidal"}


def write_tm36_las():
    src = laspy.read(str(LAS))
    header = laspy.LasHeader(point_format=1, version="1.2")
    header.scales = np.array(src.header.scales)
    header.offsets = np.array([OFFSET[0] + DX, OFFSET[1] + DY, OFFSET[2]])
    header.creation_date = datetime.date(2026, 11, 7)
    las = laspy.LasData(header)
    las.X, las.Y, las.Z = np.asarray(src.X), np.asarray(src.Y), np.asarray(src.Z)
    las.classification = np.asarray(src.classification)
    las.return_number = np.asarray(src.return_number)
    las.number_of_returns = np.asarray(src.number_of_returns)
    las.write(str(LAS_TM36))


def read_las(path=None):
    las = laspy.read(str(path or LAS))
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
ANTALYA, EGM = UID.format(11), UID.format(12)


def heights_setup(heights=None, others=None):
    """Docs/adr/1029: the drawing with the two clouds whose files name a vertical system (uids …11 and …12), the
    project's heights and the existing surface's other cloud records as given."""
    d = setup()
    _, bounds = read_las()
    n = len(read_las()[0])
    for f in ("road-ground-antalya.las", "road-ground-egm2008.las"):
        d["entities"].append({"kind": "pointcloud", "layerId": "bulut", "attrs": {}, "id": len(d["entities"]) + 1,
                              "sources": [{"file": f"$fixture:pointcloud/v1/files/{f}", "format": "las", "count": n,
                                           "bounds": bounds}],
                              "bounds": bounds, "count": n, "srid": 5254,
                              "style": {"render": "classification", "size": 2}})
        d["uids"].append(UID.format(len(d["entities"])))
    if heights is not None:
        d["settings"]["heights"] = heights
    if others is not None:
        d["settings"]["road"]["surfaces"][0]["clouds"] = others
    return d
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


def figures(kept):
    """A new surface's figures (no breakline, no boundary, no repair): its points, its Delaunay triangles, their plan
    area and its lowest and highest height."""
    tris = delaunay(kept)
    area = sum(abs((kept[j][0] - kept[i][0]) * (kept[k][1] - kept[i][1])
                   - (kept[k][0] - kept[i][0]) * (kept[j][1] - kept[i][1])) / 2 for i, j, k in tris)
    zs = [p[2] for p in kept]
    return {"area": area, "constrained": 0, "maxZ": max(zs), "merged": 0, "minZ": min(zs), "split": 0,
            "triangles": len(tris), "trimmed": 0, "vertices": len(kept)}


def geoid_turn(pts):
    """Docs/adr/1032: TUREF / TM36's points' ellipsoidal heights as orthometric, H = h − N, N geoid_cases.py's own
    bilinear reading of the synthetic TG geoid at the point's latitude and longitude (PROJ's inverse TM36)."""
    import geoid_cases as gc
    from pyproj import CRS, Transformer
    t = Transformer.from_crs("EPSG:5256", CRS.from_epsg(5256).geodetic_crs, always_xy=True)
    out = []
    for x, y, z in pts:
        lon, lat = t.transform(x, y)
        out.append((x, y, z - gc.own_n("tg.gtx", lon, lat)))
    return out


def answer(inp, points, measured=True, turn=None):
    """The output `inp` gives, and whether the error is the reference's own (no breakline, no other point). With `turn`
    (docs/adr/1032) the kept and left points' heights are turned after the thinning, and a new surface's figures are
    the reference's own."""
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
    kept_pts = [p for _, p in t["points"]]
    if turn is not None:
        kept_pts, left = turn(kept_pts), turn(left)
    count, outside, worst, rms = residuals(kept_pts + extra, left)
    if count:
        own = not (new and inp.get("breaklines"))
        record["errorMax"], record["errorRms"] = (worst, rms) if own else (ANY, ANY)
    if outside:
        record["outside"] = outside
    pieces = (len(t["points"]) + 9999) // 10000
    surface = ({"id": "t-2", "name": inp["name"], "state": "current", "digest": ANY, "built": ANY, "figures": ANY}
               if new else {"id": "t-1", "name": "Mevcut zemin", "state": "new", "digest": ANY, "figures": ANY})
    surface["algorithm"] = "kentos.cdt/1"  # docs/adr/1036
    if turn is not None and new and not inp.get("breaklines"):
        surface["figures"] = figures(kept_pts)
    scanned = len(points)
    return {"cloud": record, "scanned": scanned, "byCell": t["why"].count(0), "bySlope": t["why"].count(1),
            "byLine": t["why"].count(2), "cells": cov["cells"], "occupied": cov["occupied"], "area": cov["area"],
            "uids": [ANY] * pieces, "surface": surface, "revision": "$current"}, True


TM36 = UID.format(21)


def tm36_setup(heights=PROJECT_TG):
    """Docs/adr/1032: a road project in TUREF / TM36 whose only object is the TM36 cloud; its heights as given."""
    points, bounds = read_las(LAS_TM36)
    d = setup()
    d["entities"] = [{"kind": "pointcloud", "layerId": "bulut", "attrs": {}, "id": 1,
                      "sources": [{"file": "$fixture:pointcloud/v1/files/road-ground-tm36.las", "format": "las",
                                   "count": len(points), "bounds": bounds}],
                      "bounds": bounds, "count": len(points), "srid": 5256,
                      "style": {"render": "classification", "size": 2}}]
    d["uids"] = [TM36]
    d["settings"]["srid"] = 5256
    if heights is not None:
        d["settings"]["heights"] = copy.deepcopy(heights)
    return d


def turned_cases(points):
    """Docs/adr/1032: heights the call says are the clouds', turned into the project's by its geoid model."""
    tm_points, _ = read_las(LAS_TM36)
    tm = {**BASE, "clouds": [TM36]}
    out = []

    def done(name, inp, frm, turn):
        output, _ = answer(inp, tm_points, turn=turn)
        output["cloud"]["heights"] = {"from": frm, "to": PROJECT_TG}
        out.append({"name": name, "steps": [step("execute", inp, completed(output), STEPPED)], "setup": tm36_setup()})

    def no(name, inp, result, setup_):
        out.append({"name": name, "steps": [step("execute", inp, result, UNCHANGED)], "setup": setup_})

    done("Elipsoit yükseklikli bulut geoitle projenin kotlarına", {**tm, "heights": ELL}, ELL, geoid_turn)
    done("Bulutun kotları projeninki: çevrilmez", {**tm, "heights": PROJECT_TG}, PROJECT_TG, list)
    no("Projenin kotları söylenmemiş: bulut çevrilmez", {**tm, "heights": ELL},
       failed("heights_unknown", "Bulutun kotları projenin kotlarına çevrilir; projenin kotlarının düşey sistemi "
              "söylenmemiş. Proje ayarları › Kotlar'da seçin.", "heights"), tm36_setup(heights=None))
    unsummed = {**PROJECT_TG, "geoid": {"name": "TG-20 (sentetik)"}}
    no("Projenin geoidinin özeti yok: bulut çevrilmez", {**tm, "heights": ELL},
       failed("geoid_required", "Kotlar ortometrik, Antalya height (EPSG:5775), geoit TG-20 (sentetik) ile elipsoit "
              "yüksekliği arasında çevrilmeli: bunun için projenin geoit modeli SHA-256'sıyla gerekir "
              "(heights.geoid.sha256). Kotlar sıfır geoit ayrımıyla çevrilmez: geoit ızgarasını (GTX ya da GeoTIFF) "
              "Proje ayarları › Izgaralar'a ekleyip adını ve özetini verin.", "heights"), tm36_setup(heights=unsummed))
    absent = {**PROJECT_TG, "geoid": {"name": "TG-20 (sentetik)", "sha256": "0" * 64}}
    no("Geoidin ızgarası bu cihazda yok: bulut çevrilmez", {**tm, "heights": ELL},
       failed("geoid_missing", "“TG-20 (sentetik)” geoit modelinin ızgarası (000000000000…) bu cihazın ızgara "
              "kitaplığında yok. Izgarayı Proje ayarları › Izgaralar'a ekleyin; daha düşük doğruluklu bir yola "
              "geçilmez.", "heights"), tm36_setup(heights=absent))
    no("Bulutun kotları kurala uymuyor", {**tm, "heights": {"kind": "ellipsoidal", "verticalSrid": 5775}},
       failed("invalid_heights", "Bulutun kotları: düşey sistem ve geoit modeli yalnız ortometrik yüksekliğin olur; "
              "elipsoit yüksekliğinin elipsoidi yatay sistemindir.", "heights.kind"), tm36_setup())
    no("Geoitsiz başka düşey datum: çevrilmez", {**tm, "heights": {"kind": "orthometric", "verticalSrid": 3855}},
       failed("vertical_unsupported", "Kotlar “ortometrik, EGM2008 height (EPSG:3855)” ile “ortometrik, Antalya height "
              "(EPSG:5775), geoit TG-20 (sentetik)” arasında çevrilmeli: iki ayrı düşey datum. Bunun için iki yanın da "
              "geoit modeli SHA-256'sıyla gerekir (heights.geoid); PROJ'un yaptığı gibi kotları olduğu gibi bırakan "
              "“kaba” bir yol kullanılmaz.", "heights"), tm36_setup())
    no("Dosya başka sistem söylüyor: çağrının kotları alınmaz", {**BASE, "clouds": [EGM], "heights": ELL},
       failed("cloud_heights_differ", "1. bulutun 1. kaynağı kotların EGM2008 height (EPSG:3855) olduğunu söylüyor; "
              "çağrı elipsoit yüksekliği diyor (heights). Dosyanın söylediğini verin ya da heights'ı bırakın.",
              "clouds"), heights_setup(PROJECT_TG))
    kept = answer(BASE, points)[0]["cloud"]["kept"]
    no("Geoit ızgarasının dışındaki bulut: çevrilmez", {**BASE, "heights": ELL},
       failed("invalid_cloud_terrain", f"{kept} noktanın kotu çevrilemiyor: geoit ızgarasının dışında ya da "
              "boşluğunda. Alanı ızgaranın içine alın ya da kapsayan ızgarayı Proje ayarları › Izgaralar'a ekleyin.",
              "heights"), heights_setup(PROJECT_TG))
    return out


def step(op, inp, result, expect):
    return {"op": op, "input": inp, "result": result, "expect": expect, "within": W}


def cases():
    points, _ = read_las()
    out = []

    def ok(name, inp, warns=(), plan=False, setup_=None, vertical=None, seen=None):
        output, _ = answer(inp, seen or points)
        if seen:
            output["scanned"] = len(seen)
        if vertical is not None:
            output["cloud"]["vertical"] = vertical
        steps = [step("plan", inp, completed(output, warns), UNCHANGED)] if plan else []
        if not plan:
            steps.append(step("execute", inp, completed(output, warns), STEPPED))
        c = {"name": name, "steps": steps}
        if setup_ is not None:
            c["setup"] = setup_
        out.append(c)

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
    # Docs/adr/1029: the files' vertical system recorded; refused where it differs from the project's, from another
    # source's or from the surface's other clouds'.
    ant, egm = {**BASE, "clouds": [ANTALYA]}, {**BASE, "clouds": [EGM]}
    ortho = lambda srid: {"kind": "orthometric", "verticalSrid": srid}  # noqa: E731
    ok("Dosyanın düşey sistemi kaydedilir (GeoTIFF anahtarı)", ant, setup_=heights_setup(), vertical="EPSG:5775")
    ok("Dosyanın düşey sistemi kaydedilir (WKT)", egm, setup_=heights_setup(), vertical="EGM2008 height")
    ok("Projeninkiyle aynı düşey sistem", egm, setup_=heights_setup(ortho(3855)), vertical="EGM2008 height")
    ok("Biri söylemiyorsa düşey sistem bilinmiyor", {**BASE, "clouds": [CLOUD, ANTALYA]}, setup_=heights_setup(),
       seen=points + points)
    differ = "cloud_heights_differ"
    no("Projeninkinden başka düşey sistem: ret", egm,
       failed(differ, "Bulutun kotları EGM2008 height (EPSG:3855), projenin kotları Antalya height (EPSG:5775): aradaki "
              "ayrım sıfır sayılmaz, bulut noktaları çevrilmeden zemine alınmaz. Bulutu projenin sistemine çevirin ya "
              "da projenin kotlarını Proje ayarlarında düzeltin.", "clouds"), setup_=heights_setup(ortho(5775)))
    no("Elipsoit yüksekliğindeki projeye ortometrik bulut: ret", ant,
       failed(differ, "Bulutun kotları Antalya height (EPSG:5775), projenin kotları elipsoit yüksekliği: aradaki ayrım "
              "sıfır sayılmaz, bulut noktaları çevrilmeden zemine alınmaz. Bulutu projenin sistemine çevirin ya da "
              "projenin kotlarını Proje ayarlarında düzeltin.", "clouds"),
       setup_=heights_setup({"kind": "ellipsoidal"}))
    no("İki kaynak iki düşey sistem: ret", {**BASE, "clouds": [ANTALYA, EGM]},
       failed(differ, "Bulutların kaynakları ayrı düşey sistemlerde: 1. bulutun 1. kaynağı Antalya height (EPSG:5775), "
              "2. bulutun 1. kaynağı EGM2008 height (EPSG:3855). Bir yüzeyin noktaları tek sistemde olmalı; kaynakları "
              "önce aynı sisteme çevirin.", "clouds"), setup_=heights_setup())
    other = [{"layer": "eski", "clouds": [EGM], "classes": [2], "returns": "all", "cell": 2.0, "read": 10, "kept": 5, "coverage": 0.5,
              "density": 1.0, "vertical": "EGM2008 height"}]
    no("Yüzeyin öbür bulutundan başka sistem: ret", {k: v for k, v in ant.items() if k != "name"} | {
        "surface": "Mevcut zemin"},
       failed(differ, "Bulutun kotları Antalya height (EPSG:5775), yüzeyin “eski” katmanındaki bulut noktalarınınki "
              "EGM2008 height (EPSG:3855): bir yüzeyin noktaları tek sistemde olmalı.", "clouds"),
       setup_=heights_setup(others=other))
    out.extend(turned_cases(points))
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
    if "--heights-las" in sys.argv:
        write_heights_las()
        print(LAS_ANTALYA, LAS_EGM)
        return
    if "--tm36-las" in sys.argv:
        write_tm36_las()
        print(LAS_TM36)
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
