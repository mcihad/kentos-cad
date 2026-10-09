#!/usr/bin/env python3
"""Uzaklık ve maliyet (docs/adr/0236): the four tools' rules written from the ADR with Python, without KentOS code, and the
cases the raster core plays (crates/shared/raster/tests/all/distance.rs) and the web's WASM module
(apps/web/src/io/raster.wasm.test.ts) bit for bit.

- Uzaklık yüzeyi: by brute force, every cell against every source cell: the least (Δi·sx)² + (Δj·sy)² as an exact fraction
  of the float64 step lengths, then the smaller column, then the smaller row; the distance √(x·x + y·y) in float64.
- Birikimli maliyet: Dijkstra over the cells with a priority queue of (sum, cell); a step's cost the ADR's average times
  its length, every sum in float64 in the ADR's order; a cell's predecessor the least (sum, cell) among the neighbours whose
  sum and step give its sum; the sources carried along in settling order.
- En düşük maliyetli yol: each destination's cells by Rasterleştir's rule (scripts/fixtures/raster_vector_cases.py), its
  least (sum, cell), the predecessors back to a source; Douglas–Peucker in cell space.
- Maliyet koridoru: the two searches' sums, the threshold.

Cross-checks (--check, when `grass` is on the path): r.cost's sums (8 and 16 neighbours; its lengths are in east–west cells,
so ours are divided by the cell size) on cost rasters without corner-joined barriers; r.grow.distance's Euclidean distances
on square and oblong cells; GDAL's Proximity on square cells.

    python3 scripts/fixtures/distance_cases.py          # write fixtures/distance/v1/cases.json
    python3 scripts/fixtures/distance_cases.py --check  # compare, and cross-check with GRASS and GDAL
"""

import heapq
import json
import math
import shutil
import struct
import sys
from fractions import Fraction
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import hydrology_cases as hc  # noqa: E402  (degree_metres, GRASS's runner)
import raster_ops_cases as ro  # noqa: E402  (rasters, places)
import raster_vector_cases as rv  # noqa: E402  (Rasterleştir's cells, the box's grid, Douglas–Peucker)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/distance/v1/cases.json"

# §2: the moves from east clockwise (cell space, v down); the eight's among them.
MOVES = [(1, 0), (2, 1), (1, 1), (1, 2), (0, 1), (-1, 2), (-1, 1), (-2, 1), (-1, 0), (-2, -1), (-1, -1), (-1, -2), (0, -1),
         (1, -2), (1, -1), (2, -1)]
EIGHT = [0, 2, 4, 6, 8, 10, 12, 14]
NAN = math.nan


def f32(x):
    return struct.unpack("<f", struct.pack("<f", float(x)))[0]


def row_axes(affine, w, j, geographic):
    """ADR 0231 §2: the row's pixel axes in metres (a geographic row's at its middle column's centre)."""
    x0, a, b, y0, c, d = affine
    if geographic:
        lat = y0 + c * (w / 2.0) + d * (j + 0.5)
        ke, kn = hc.degree_metres(lat)
        return [a * ke, b * ke, c * kn, d * kn]
    return [a, b, c, d]


# ── Uzaklık yüzeyi (§3) ───────────────────────────────────────────────────

def plane_steps(affine, geographic):
    if geographic:
        return {"refused": "coğrafi ızgarada"}
    _, a, b, _, c, d = affine
    if abs(a * b + c * d) > 1e-12 * (a * a + b * b + c * c + d * d):
        return {"refused": "eğik"}
    return (math.sqrt(a * a + c * c), math.sqrt(b * b + d * d))


def euclid(w, h, own, steps, mx, allocation):
    """Each cell's distance to (or number of) its nearest source cell: least (d², column, row)."""
    sx, sy = steps
    fx, fy = Fraction(sx) ** 2, Fraction(sy) ** 2
    sources = [(k % w, k // w) for k in range(w * h) if not math.isnan(own[k])]
    if not sources:
        return {"refused": "Kaynak hücre yok"}
    out = []
    for j in range(h):
        for i in range(w):
            _, ti, tj = min((fx * (i - a) ** 2 + fy * (j - b) ** 2, a, b) for a, b in sources)
            x = float(i - ti) * sx
            y = float(j - tj) * sy
            d = math.sqrt(x * x + y * y)
            if mx > 0 and d > mx:
                out.append(NAN)
                continue
            out.append(own[tj * w + ti] if allocation else d)
    return out, len(sources)


# ── The cost network (§4) ─────────────────────────────────────────────────

class Net:
    def __init__(self, affine, w, h, geographic, cost, z, neighbours, surface_length, slope):
        self.affine, self.w, self.h = affine, w, h
        self.cost = list(cost)
        self.z = z
        if z is not None:
            self.cost = [NAN if math.isnan(zz) else cc for cc, zz in zip(self.cost, z)]
        self.moves = EIGHT if neighbours == "8" else list(range(16))
        self.surface_length = z is not None and surface_length
        self.slope = slope / 100.0 if z is not None and slope > 0 else math.inf
        self.rows = []
        for j in range(h if geographic else 1):
            a, b, c, d = row_axes(affine, w, j, geographic)
            sq, ln = [], []
            for di, dj in MOVES:
                x = a * di + b * dj
                y = c * di + d * dj
                s = x * x + y * y
                sq.append(s)
                ln.append(math.sqrt(s))
            self.rows.append((sq, ln))
        self.geographic = geographic

    def steps(self, j):
        return self.rows[j if self.geographic else 0]

    def cost_at(self, i, j):
        if not (0 <= i < self.w and 0 <= j < self.h):
            return NAN
        return self.cost[j * self.w + i]

    def step(self, i, j, q):
        """Move q out of cell (i, j): (the cell reached, the step's cost), or None."""
        di, dj = MOVES[q]
        a, b = i + di, j + dj
        if not (0 <= a < self.w and 0 <= b < self.h):
            return None
        m = b * self.w + a
        c1 = self.cost[m]
        if math.isnan(c1):
            return None
        c0 = self.cost[j * self.w + i]
        if abs(di) == 1 and abs(dj) == 1:
            s, t = self.cost_at(i + di, j), self.cost_at(i, j + dj)
            if math.isnan(s) and math.isnan(t):
                return None
            avg = (c0 + c1) / 2.0
        elif abs(di) + abs(dj) == 3:
            if abs(di) == 2:
                s, t = self.cost_at(i + di // 2, j), self.cost_at(i + di // 2, j + dj)
            else:
                s, t = self.cost_at(i, j + dj // 2), self.cost_at(i + di, j + dj // 2)
            if math.isnan(s) or math.isnan(t):
                return None
            avg = (((c0 + c1) + s) + t) / 4.0
        else:
            avg = (c0 + c1) / 2.0
        sq, ln = self.steps(j)
        length = ln[q]
        if self.z is not None:
            dz = self.z[m] - self.z[j * self.w + i]
            if abs(dz) > self.slope * length:
                return None
            if self.surface_length:
                length = math.sqrt(sq[q] + dz * dz)
        return m, avg * length


def search(net, sources, mx=0.0, targets=None):
    """Dijkstra from the source cells: the sums (inf: not reached), the settling order, the last sum settled."""
    n = net.w * net.h
    acc = [math.inf] * n
    heap = []
    for k in sources:
        if not math.isnan(net.cost[k]) and acc[k] != 0.0:
            acc[k] = 0.0
            heapq.heappush(heap, (0.0, k))
    limit = mx if mx > 0 else math.inf
    order = []
    stop = math.inf
    pending = None if targets is None else [t for t in targets if t]
    while heap:
        a, k = heapq.heappop(heap)
        if a != acc[k]:
            continue
        if a > limit or a > stop:
            break
        order.append(k)
        if pending is not None and stop == math.inf:
            # Every destination with a final sum: stop past the largest of their least.
            if all(min(acc[c] for c in t) <= a for t in pending):
                stop = max(min(acc[c] for c in t) for t in pending)
        i, j = k % net.w, k // net.w
        for q in net.moves:
            got = net.step(i, j, q)
            if got is None:
                continue
            m, c = got
            na = a + c
            if na < acc[m]:
                assert na > a, "a step lost against a sum"
                acc[m] = na
                heapq.heappush(heap, (na, m))
    return acc, order, stop


def predecessor(net, acc, c):
    """§4: the least (sum, cell) among the neighbours n with sum(n) + step(n → c) = sum(c)."""
    ac = acc[c]
    if not math.isfinite(ac) or ac == 0.0:
        return None
    i, j = c % net.w, c // net.w
    best = None
    for q in net.moves:
        di, dj = MOVES[q]
        a, b = i - di, j - dj
        if not (0 <= a < net.w and 0 <= b < net.h):
            continue
        n = b * net.w + a
        if not math.isfinite(acc[n]):
            continue
        got = net.step(a, b, q)
        if got is not None and got[0] == c and acc[n] + got[1] == ac:
            if best is None or (acc[n], n) < best[:2]:
                best = (acc[n], n, q)
    return best


def cells_of(shape, affine, w, h):
    """Rasterleştir's cells of one object (docs/adr/0234 §3)."""
    if shape["kind"] in rv.AREA_KINDS:
        return set(ro.inside_cells(shape, affine, w, h))
    cells = set()
    if shape["kind"] == "point":
        for p in rv.point_list(shape):
            u, v = ro.place_in(affine, p[0], p[1])
            i, j = math.floor(u), math.floor(v)
            if 0 <= i < w and 0 <= j < h:
                cells.add((i, j))
        return cells
    for path in rv.chord_paths(shape):
        cp = [ro.place_in(affine, x, y) for x, y in path]
        for a, b in zip(cp, cp[1:]):
            cells |= rv.chord_cells(a, b, w, h)
    return cells


def burnt(shapes, affine, w, h):
    """Each cell the first object's number burning it (NaN: none), and the objects without a cell."""
    own = [NAN] * (w * h)
    outside = 0
    for o in range(len(shapes) - 1, -1, -1):
        cells = cells_of(shapes[o], affine, w, h)
        if not cells:
            outside += 1
        for i, j in cells:
            own[j * w + i] = float(o + 1)
    return own, outside


def source_cells(net, own):
    return [k for k in range(len(own)) if not math.isnan(own[k]) and not math.isnan(net.cost[k])]


def measure(values):
    vs = [v for v in values if not math.isnan(v)]
    return {"least": min(vs) if vs else None, "most": max(vs) if vs else None, "cells": len(vs)}


def read_band(r, b=0):
    out = []
    for j in range(r.height):
        for i in range(r.width):
            v = r.get(i, j, b)
            out.append(NAN if v is None or math.isnan(v) else float(v))
    return out


def bilinear64(r, u, v):
    """ADR 0233 §8's two by two weights in float64, the cells without a value left out (the core's order)."""
    su, sv = u - 0.5, v - 0.5
    i, j = math.floor(su), math.floor(sv)
    fx, fy = su - i, sv - j
    acc, wsum = 0.0, 0.0
    for di, wx in ((0, 1.0 - fx), (1, fx)):
        for dj, wy in ((0, 1.0 - fy), (1, fy)):
            w = wx * wy
            x = r.get(i + di, j + dj, 0)
            if w != 0.0 and x is not None and not math.isnan(x):
                acc += w * x
                wsum += w
    return NAN if abs(wsum) < 1e-6 else acc / wsum


def surface_on(grid_affine, w, h, s):
    """The surface's heights at the cost grid's centres."""
    out = []
    for j in range(h):
        for i in range(w):
            x, y = ro.point_of(grid_affine, float(i) + 0.5, float(j) + 0.5)
            u, v = ro.place_in(s.affine, x, y)
            out.append(bilinear64(s, u, v))
    return out


def network_of(r, surface, geographic, tool):
    cost = read_band(r, tool.get("band", 1) - 1)
    bad = [v for v in cost if not math.isnan(v) and not (v > 0 and math.isfinite(v))]
    if bad:
        return {"refused": "Maliyet rasterinde 0 ya da eksi değer var"}
    slope = tool.get("slope", 0.0)
    if not (0.0 <= slope <= 1000.0):
        return {"refused": "En büyük boyuna eğim"}
    z = surface_on(r.affine, r.width, r.height, surface) if surface is not None else None
    return Net(r.affine, r.width, r.height, geographic, cost, z, tool["neighbours"], tool.get("surfaceLength", False), slope)


def cost_distance(r, surface, geographic, tool, shapes):
    net = network_of(r, surface, geographic, tool)
    if isinstance(net, dict):
        return net
    own, outside = burnt(shapes, r.affine, r.width, r.height)
    cells = source_cells(net, own)
    if not cells:
        return {"refused": "Kaynak hücre yok"}
    acc, order, _ = search(net, cells, tool.get("max", 0.0))
    limit = tool["max"] if tool.get("max", 0.0) > 0 else math.inf
    values = [a if math.isfinite(a) and a <= limit else NAN for a in acc]
    if tool["result"] == "allocation":
        got = list(own)
        for k in order:
            p = predecessor(net, values, k)
            if p is not None:
                got[k] = got[p[1]]
        values = [got[k] if not math.isnan(values[k]) else NAN for k in range(len(values))]
    sample = "f32" if tool["result"] == "allocation" else tool["sample"]
    notes = {"sources": len(cells), "outside": outside, **measure(values)}
    return raster(r, values, sample, notes)


def raster(r, values, sample, notes):
    stored = [None if math.isnan(v) else (f32(v) if sample == "f32" else v) for v in values]
    return {"raster": {"width": r.width, "height": r.height, "sample": sample, "rule": "exact", "values": stored}, "notes": notes}


def cost_path(r, surface, geographic, tool, shapes):
    net = network_of(r, surface, geographic, tool)
    if isinstance(net, dict):
        return net
    first = tool["first"]
    if first == 0:
        return {"refused": "Başlangıç nesnelerini"}
    if first >= len(shapes):
        return {"refused": "Varış nesnelerini"}
    own, outside = burnt(shapes[:first], r.affine, r.width, r.height)
    cells = source_cells(net, own)
    if not cells:
        return {"refused": "Kaynak hücre yok"}
    w = r.width
    targets = []
    unreached = []
    for x, s in enumerate(shapes[first:]):
        t = sorted(j * w + i for i, j in cells_of(s, r.affine, r.width, r.height))
        t = [k for k in t if not math.isnan(net.cost[k])]
        if not t:
            unreached.append(x + 1)
        targets.append(t)
    acc, order, stop = search(net, cells, 0.0, targets) if any(targets) else ([math.inf] * (w * r.height), [], math.inf)
    surface_fields = net.z is not None
    fields = ["Yol", "Kaynak", "Maliyet", "Uzunluk"] + (["Yüzey uzunluğu", "En büyük eğim"] if surface_fields else [])
    fs = {"kind": "lines", "fields": fields, "numbers": [], "rings": [], "sizes": [], "xy": []}
    for x, t in enumerate(targets):
        ends = [k for k in t if math.isfinite(acc[k]) and acc[k] <= stop]
        if not ends:
            if t:
                unreached.append(x + 1)
            continue
        end = min(ends, key=lambda k: (acc[k], k))
        path = [end]
        c = end
        while True:
            p = predecessor(net, acc, c)
            if p is None:
                break
            c = p[1]
            path.append(c)
        path.reverse()
        plan, surf, grade = 0.0, 0.0, 0.0
        for a, b in zip(path, path[1:]):
            q = MOVES.index((b % w - a % w, b // w - a // w))
            j = a // w
            sq, ln = net.steps(j)
            plan += ln[q]
            if net.z is not None:
                dz = net.z[b] - net.z[a]
                surf += math.sqrt(sq[q] + dz * dz)
                grade = max(grade, abs(dz) / ln[q] * 100.0)
        pts = [((k % w) + 0.5, (k // w) + 0.5) for k in path]
        kept = rv.dp(pts, tool["simplify"])
        fs["sizes"].append(len(kept))
        for u, v in kept:
            fs["xy"] += list(ro.point_of(r.affine, u, v))
        fs["numbers"] += [float(x + 1), own[path[0]], acc[end], plan] + ([surf, grade] if surface_fields else [])
    return {"features": fs, "notes": {"sources": len(cells), "outside": outside, "unreached": sorted(set(unreached))}}


def corridor(r, surface, geographic, tool, shapes):
    net = network_of(r, surface, geographic, tool)
    if isinstance(net, dict):
        return net
    first = tool["first"]
    if first == 0:
        return {"refused": "Birinci uçları"}
    if first >= len(shapes):
        return {"refused": "İkinci uçları"}
    a_own, out_a = burnt(shapes[:first], r.affine, r.width, r.height)
    b_own, out_b = burnt(shapes[first:], r.affine, r.width, r.height)
    ca, cb = source_cells(net, a_own), source_cells(net, b_own)
    if not ca or not cb:
        return {"refused": "Kaynak hücre yok"}
    fa, _, _ = search(net, ca)
    fb, _, _ = search(net, cb)
    sums = [x + y if math.isfinite(x) and math.isfinite(y) else NAN for x, y in zip(fa, fb)]
    vs = [s for s in sums if not math.isnan(s)]
    if not vs:
        return {"refused": "Uçlar birbirine erişemiyor"}
    least = min(vs)
    th = tool["threshold"]
    limit = math.inf if th == "none" else least * (1.0 + tool["value"] / 100.0) if th == "percent" else tool["value"]
    sums = [NAN if s > limit else s for s in sums]
    # The least is the least sum before the threshold: the cheapest path's cost.
    notes = {"sources": len(ca) + len(cb), "outside": out_a + out_b, **measure(sums), "least": least}
    return raster(r, sums, tool["sample"], notes)


def euclid_raster(r, geographic, tool):
    steps = plane_steps(r.affine, geographic)
    if isinstance(steps, dict):
        return steps
    own = read_band(r, tool["band"] - 1)
    got = euclid(r.width, r.height, own, steps, tool["max"], tool["result"] == "allocation")
    if isinstance(got, dict):
        return got
    values, n = got
    return raster(r, values, "f32", {"sources": n, **measure(values)})


def euclid_objects(spec, shapes):
    """Uzaklık yüzeyi from objects (the point job): the objects' box with its margin, or a raster's grid."""
    tool = spec["tool"]
    pts = []
    for s in shapes:
        pts += rv.point_list(s) if s["kind"] == "point" else [p for path in rv.chord_paths(s) for p in path]
    if spec.get("grid"):
        g = spec["grid"]
        affine, w, h = g["affine"], g["width"], g["height"]
    else:
        m = tool["margin"]
        b = (min(p[0] for p in pts) - m, min(p[1] for p in pts) - m, max(p[0] for p in pts) + m, max(p[1] for p in pts) + m)
        affine, w, h = rv.grid_of_box(b, spec["cell"])
    assert rv.margin_ok(pts, affine), "a chord point on a cell line: choose the case away from ties"
    steps = plane_steps(affine, False)
    if isinstance(steps, dict):
        return steps
    own, outside = burnt(shapes, affine, w, h)
    got = euclid(w, h, own, steps, tool["max"], tool["result"] == "allocation")
    if isinstance(got, dict):
        return got
    values, n = got
    notes = {"outside": outside, "taken": len(shapes), "empty": sum(math.isnan(v) for v in values)}
    out = raster(ro.Raster(affine, w, h, [0.0] * (w * h)), values, "f32", notes)
    out["raster"]["affine"] = affine
    return out


# ── The cases ─────────────────────────────────────────────────────────────

PLACE = [500000.0, 10.0, 0.0, 4420000.0, 0.0, -10.0]


def noise(i, j, seed):
    x = (i * 0x9E3779B9 ^ j * 0x85EBCA6B ^ seed * 0x2C1B3C6D) & 0xFFFFFFFF
    x ^= x >> 15
    x = (x * 0x2C1B3C6D) & 0xFFFFFFFF
    x ^= x >> 12
    return x


def costs(w, h, seed, holes=(), affine=PLACE, sample="f32"):
    """A cost raster: 1 … 9 in eighths with a cheap valley; `holes` without a value."""
    vals = []
    for j in range(h):
        for i in range(w):
            if (i, j) in holes:
                vals.append(NAN)
                continue
            valley = abs(j - h / 2 - 3 * math.sin(i / 4.0))
            vals.append(1.0 + min(8.0, valley * 0.75) + (noise(i, j, seed) % 9) / 8.0)
    return ro.Raster(list(affine), w, h, vals, 1, sample, "nan")


def heights(w, h, affine=PLACE, seed=3):
    vals = [100.0 + 2.0 * i + 1.5 * j + 3.0 * math.sin(i / 3.0) * math.cos(j / 4.0) + (noise(i, j, seed) % 7) / 10.0
            for j in range(h) for i in range(w)]
    return ro.Raster(list(affine), w, h, vals, 1, "f32", "nan")


def at(affine, u, v):
    """A world point at cell-space place (u, v) (kept off the cell lines)."""
    x, y = ro.point_of(affine, u, v)
    return {"x": x, "y": y}


def point(p):
    return {"kind": "point", "p": p}


def polyline(pts):
    return {"kind": "polyline", "pts": pts}


def polygon(pts):
    return {"kind": "polygon", "pts": pts}


def ops_case(name, r, tool, shapes=(), surface=None, geographic=False):
    kind = tool["kind"]
    if kind == "distance":
        expect = euclid_raster(r, geographic, tool)
    elif kind == "costDistance":
        expect = cost_distance(r, surface, geographic, tool, list(shapes))
    elif kind == "costPath":
        expect = cost_path(r, surface, geographic, tool, list(shapes))
    else:
        expect = corridor(r, surface, geographic, tool, list(shapes))
    if geographic and "raster" in expect:
        # A degree's metres come from sin and cos, which libraries may round apart in the last bit (docs/adr/0235's rule).
        expect["raster"]["rule"] = "f32ulp"
    return {"name": name, "job": "ops", "input": r.json(), "surface": surface.json() if surface else None,
            "geographic": geographic, "tool": tool, "shapes": list(shapes), "expect": expect}


def points_case(name, spec, shapes):
    return {"name": name, "job": "points", "spec": spec, "shapes": list(shapes), "expect": euclid_objects(spec, shapes)}


def tool_of(kind, **kw):
    return {"kind": kind, "band": 1, **kw}


def cases():
    out = []
    # Uzaklık yüzeyi from a raster's cells with values.
    def scattered(w, h, seed, every, affine=PLACE):
        vals = [float(1 + noise(i, j, seed) % 5) if noise(i, j, seed + 1) % every == 0 else NAN
                for j in range(h) for i in range(w)]
        return ro.Raster(list(affine), w, h, vals, 1, "f32", "nan")
    sq = scattered(23, 17, 1, 40)
    out.append(ops_case("uzaklik-raster-kare", sq, tool_of("distance", max=0.0, result="distance")))
    out.append(ops_case("uzaklik-raster-tahsis", sq, tool_of("distance", max=0.0, result="allocation")))
    out.append(ops_case("uzaklik-raster-en-buyuk", sq, tool_of("distance", max=35.0, result="distance")))
    oblong = scattered(21, 19, 2, 35, affine=[500000.0, 10.0, 0.0, 4420000.0, 0.0, -6.0])
    out.append(ops_case("uzaklik-raster-dikdortgen", oblong, tool_of("distance", max=0.0, result="distance")))
    out.append(ops_case("uzaklik-raster-dikdortgen-tahsis", oblong, tool_of("distance", max=0.0, result="allocation")))
    th = math.radians(30.0)
    turned_affine = [500000.0, 5.0 * math.cos(th), 5.0 * math.sin(th), 4420000.0, 5.0 * math.sin(th), -5.0 * math.cos(th)]
    out.append(ops_case("uzaklik-raster-donuk", scattered(17, 15, 3, 30, affine=turned_affine),
                        tool_of("distance", max=0.0, result="distance")))
    ties = ro.Raster(list(PLACE), 11, 11, [float(k % 4 + 1) if k in (0, 10, 110, 120, 60) else NAN for k in range(121)],
                     1, "f32", "nan")
    out.append(ops_case("uzaklik-esitlikler", ties, tool_of("distance", max=0.0, result="allocation")))
    geo = [32.8, 0.0005, 0.0, 39.92, 0.0, -0.0004]
    out.append(ops_case("ret-uzaklik-cografi", scattered(9, 7, 4, 10, affine=geo), tool_of("distance", max=0.0, result="distance"),
                        geographic=True))
    skew = [500000.0, 10.0, 2.0, 4420000.0, 0.0, -10.0]
    out.append(ops_case("ret-uzaklik-egik", scattered(9, 7, 5, 10, affine=skew), tool_of("distance", max=0.0, result="distance")))
    empty = ro.Raster(list(PLACE), 6, 5, [NAN] * 30, 1, "f32", "nan")
    out.append(ops_case("ret-uzaklik-kaynaksiz", empty, tool_of("distance", max=0.0, result="distance")))

    # Uzaklık yüzeyi from objects (the point job).
    objs = [point({"x": 500013.0, "y": 4420047.0}), polyline([{"x": 500031.0, "y": 4420009.0}, {"x": 500078.0, "y": 4420061.0}]),
            polygon([{"x": 500091.0, "y": 4420003.0}, {"x": 500121.0, "y": 4420004.0}, {"x": 500117.0, "y": 4420033.0}])]
    for result in ("distance", "allocation"):
        out.append(points_case(f"uzaklik-nesneler-{'tahsis' if result == 'allocation' else 'uzaklik'}",
                               {"tool": {"kind": "distance", "max": 0.0, "result": result, "margin": 23.0}, "cell": 5.0}, objs))
    out.append(points_case("uzaklik-nesneler-izgara", {"tool": {"kind": "distance", "max": 60.0, "result": "distance", "margin": 0.0},
                                                       "cell": 0.0, "grid": {"affine": [499990.0, 10.0, 0.0, 4420090.0, 0.0, -6.0],
                                                                             "width": 16, "height": 17}}, objs))

    # Birikimli maliyet.
    c = costs(31, 23, 7)
    src = [point(at(PLACE, 2.3, 4.6)), polyline([at(PLACE, 25.2, 18.3), at(PLACE, 28.7, 20.4)])]
    for n in ("16", "8"):
        out.append(ops_case(f"maliyet-{n}", c, tool_of("costDistance", neighbours=n, surfaceLength=False, slope=0.0, max=0.0,
                                                      result="cost", sample="f32"), src))
        out.append(ops_case(f"maliyet-{n}-tahsis", c, tool_of("costDistance", neighbours=n, surfaceLength=False, slope=0.0, max=0.0,
                                                             result="allocation", sample="f32"), src))
    out.append(ops_case("maliyet-en-buyuk", c, tool_of("costDistance", neighbours="16", surfaceLength=False, slope=0.0, max=60.0,
                                                       result="cost", sample="f64"), src))
    # Barriers: blocks and a wall joined only at its corners (its diagonal steps not taken).
    wall = {(10 + k, 3 + k) for k in range(14)}
    blocks = {(i, j) for i in range(18, 22) for j in range(2, 9)} | {(4, 15), (5, 15), (4, 16), (5, 16)}
    cb = costs(31, 23, 8, holes=wall | blocks)
    for n in ("16", "8"):
        out.append(ops_case(f"maliyet-engel-{n}", cb, tool_of("costDistance", neighbours=n, surfaceLength=False, slope=0.0, max=0.0,
                                                              result="cost", sample="f32"), [point(at(PLACE, 12.4, 1.6))]))
    oblong_cost = costs(19, 17, 9, affine=[500000.0, 10.0, 0.0, 4420000.0, 0.0, -6.0])
    out.append(ops_case("maliyet-dikdortgen", oblong_cost, tool_of("costDistance", neighbours="16", surfaceLength=False, slope=0.0,
                                                                   max=0.0, result="cost", sample="f64"),
                        [point(at(oblong_cost.affine, 3.4, 3.6))]))
    gc = costs(17, 13, 10, affine=geo)
    out.append(ops_case("maliyet-cografi", gc, tool_of("costDistance", neighbours="16", surfaceLength=False, slope=0.0, max=0.0,
                                                       result="cost", sample="f32"), [point(at(geo, 8.4, 6.3))], geographic=True))
    dem = heights(31, 23)
    for name, sl, slope in (("maliyet-yukseklik", True, 0.0), ("maliyet-egim", False, 12.0), ("maliyet-yukseklik-egim", True, 12.0)):
        out.append(ops_case(name, c, tool_of("costDistance", neighbours="16", surfaceLength=sl, slope=slope, max=0.0,
                                             result="cost", sample="f64"), src, surface=dem))
    # A surface on another grid: read at the cost cells' centres, two by two.
    coarse = heights(11, 9, affine=[499995.0, 30.0, 0.0, 4420007.0, 0.0, -28.0], seed=4)
    out.append(ops_case("maliyet-orneklenen-yukseklik", c, tool_of("costDistance", neighbours="16", surfaceLength=True, slope=15.0,
                                                                    max=0.0, result="cost", sample="f64"), src, surface=coarse))
    zero = ro.Raster(list(PLACE), 5, 4, [1.0] * 7 + [0.0] + [2.0] * 12, 1, "f32", "nan")
    out.append(ops_case("ret-maliyet-sifir", zero, tool_of("costDistance", neighbours="16", surfaceLength=False, slope=0.0, max=0.0,
                                                           result="cost", sample="f32"), [point(at(PLACE, 1.5, 1.5))]))
    out.append(ops_case("ret-maliyet-kaynak-engelde", cb, tool_of("costDistance", neighbours="16", surfaceLength=False, slope=0.0,
                                                                   max=0.0, result="cost", sample="f32"), [point(at(PLACE, 19.5, 4.5))]))
    out.append(ops_case("ret-maliyet-egim", c, tool_of("costDistance", neighbours="16", surfaceLength=False, slope=2000.0, max=0.0,
                                                       result="cost", sample="f32"), src, surface=dem))

    # En düşük maliyetli yol.
    start = [point(at(PLACE, 1.4, 11.6))]
    ends = [point(at(PLACE, 29.6, 10.3)), point(at(PLACE, 20.3, 1.4)),
            polygon([at(PLACE, 12.3, 19.2), at(PLACE, 16.7, 19.3), at(PLACE, 16.6, 21.8), at(PLACE, 12.2, 21.7)]),
            point(at(PLACE, 40.5, 3.5))]
    for n, simplify in (("16", 0.0), ("8", 1.0)):
        out.append(ops_case(f"yol-{n}", c, tool_of("costPath", neighbours=n, surfaceLength=False, slope=0.0, simplify=simplify,
                                                   first=1), start + ends))
    out.append(ops_case("yol-engel", cb, tool_of("costPath", neighbours="16", surfaceLength=False, slope=0.0, simplify=0.0, first=1),
                        [point(at(PLACE, 12.4, 1.6)), point(at(PLACE, 28.5, 20.5)), point(at(PLACE, 20.5, 5.5))]))
    out.append(ops_case("yol-yukseklik", c, tool_of("costPath", neighbours="16", surfaceLength=True, slope=9.0, simplify=0.0, first=1),
                        start + ends[:2], surface=dem))
    out.append(ops_case("ret-yol-varis", c, tool_of("costPath", neighbours="16", surfaceLength=False, slope=0.0, simplify=0.0,
                                                    first=1), start))

    # Maliyet koridoru.
    a_end, b_end = [point(at(PLACE, 1.4, 11.6))], [point(at(PLACE, 29.6, 10.3))]
    for th_name, value in (("none", 0.0), ("percent", 10.0), ("value", 700.0)):
        out.append(ops_case(f"koridor-{th_name}", c, tool_of("costCorridor", neighbours="16", surfaceLength=False, slope=0.0, first=1,
                                                             threshold=th_name, value=value, sample="f32"), a_end + b_end))
    # A threshold below the cheapest path's cost: no cell, the least still said.
    out.append(ops_case("koridor-esik-alti", c, tool_of("costCorridor", neighbours="8", surfaceLength=False, slope=0.0, first=1,
                                                        threshold="value", value=300.0, sample="f32"), a_end + b_end))
    split = costs(15, 9, 11, holes={(7, j) for j in range(9)})
    out.append(ops_case("ret-koridor-erisilemez", split, tool_of("costCorridor", neighbours="16", surfaceLength=False, slope=0.0,
                                                                  first=1, threshold="none", value=0.0, sample="f32"),
                        [point(at(PLACE, 2.5, 4.5)), point(at(PLACE, 12.5, 4.5))]))
    return out


# ── Cross-checks ──────────────────────────────────────────────────────────

def ulps32(a, b):
    key = lambda v: (lambda i: -0x80000000 - i if i < 0 else i)(struct.unpack("<i", struct.pack("<f", v))[0])
    return abs(key(a) - key(b))


def cross_check():
    if not shutil.which("grass"):
        print("GRASS yok: çapraz denetim atlandı.")
        return
    import numpy as np
    from osgeo import gdal
    gdal.UseExceptions()
    out = "r.out.gdal input={0} output={0}.tif format=GTiff type=Float64 --q -c\n"
    checked = {"cost": 0, "grow": 0, "proximity": 0}
    # r.cost: random costs, two starts, barriers as separate blocks (no corner contacts); ours divided by the cell size.
    for (sx, sy), n in (((10.0, 10.0), "8"), ((10.0, 10.0), "16"), ((10.0, 6.0), "16")):
        holes = {(i, j) for i in range(9, 12) for j in range(4, 15)} | {(25, 20), (26, 20)}
        affine = [500000.0, sx, 0.0, 4420000.0, 0.0, -sy]
        r = costs(37, 29, 13, holes=holes, affine=affine, sample="f64")
        starts = [(3, 3), (30, 24)]
        net = Net(affine, r.width, r.height, False, read_band(r), None, n, False, 0.0)
        acc, _, _ = search(net, [j * r.width + i for i, j in starts])
        coords = ",".join(f"{ro.centre(affine, i, j)[0]},{ro.centre(affine, i, j)[1]}" for i, j in starts)
        flag = " -k" if n == "16" else ""
        folder = hc.grass_run(r, f"r.cost{flag} input=dem output=acc start_coordinates={coords} --q\n" + out.format("acc"))
        theirs = hc.read_tif(folder / "acc.tif")
        for k, (a, b) in enumerate(zip(acc, theirs)):
            mine = a / sx if math.isfinite(a) else NAN
            if math.isnan(mine) != math.isnan(b) or (not math.isnan(b) and abs(mine - b) > 1e-12 * max(1.0, abs(b))):
                raise SystemExit(f"GRASS r.cost farklı ({n} komşu, {sx}×{sy}): hücre {k}: {mine} ≠ {b}")
            checked["cost"] += 1
        shutil.rmtree(folder)
    # r.grow.distance and GDAL's Proximity: scattered sources.
    for sx, sy in ((10.0, 10.0), (10.0, 6.0)):
        affine = [500000.0, sx, 0.0, 4420000.0, 0.0, -sy]
        w, h = 61, 47
        own = [1.0 if noise(i, j, 21) % 90 == 0 else NAN for j in range(h) for i in range(w)]
        mine, _ = euclid(w, h, own, (sx, sy), 0.0, False)
        r = ro.Raster(affine, w, h, own, 1, "f64", "nan")
        folder = hc.grass_run(r, "r.grow.distance input=dem distance=dist metric=euclidean --q\n" + out.format("dist"))
        theirs = hc.read_tif(folder / "dist.tif")
        for k, (a, b) in enumerate(zip(mine, theirs)):
            if abs(a - b) > 1e-9 * max(1.0, a):
                raise SystemExit(f"GRASS r.grow.distance farklı ({sx}×{sy}): hücre {k}: {a} ≠ {b}")
            checked["grow"] += 1
        shutil.rmtree(folder)
        if sx == sy:
            drv = gdal.GetDriverByName("MEM")
            s = drv.Create("", w, h, 1, gdal.GDT_Byte)
            s.SetGeoTransform(affine)
            s.GetRasterBand(1).WriteArray(np.array([0 if math.isnan(v) else 1 for v in own], dtype=np.uint8).reshape(h, w))
            d = drv.Create("", w, h, 1, gdal.GDT_Float64)
            d.SetGeoTransform(affine)
            gdal.ComputeProximity(s.GetRasterBand(1), d.GetRasterBand(1), ["DISTUNITS=GEO", "VALUES=1"])
            g = d.GetRasterBand(1).ReadAsArray().ravel()
            # Proximity works in float32: ours rounded to it, one unit in the last place apart at most.
            for k, (a, b) in enumerate(zip(mine, g)):
                if ulps32(f32(a), float(b)) > 1:
                    raise SystemExit(f"GDAL Proximity farklı: hücre {k}: {a} ≠ {b}")
                checked["proximity"] += 1
    print(f"GRASS ve GDAL ile aynı: {checked['cost']} birikimli maliyet, {checked['grow']} r.grow.distance, "
          f"{checked['proximity']} Proximity hücresi.")


def main():
    check = "--check" in sys.argv
    doc = {
        "format": "kentos.distance-cases",
        "version": 1,
        "note": "ADR 0236'nın araçları, KentOS kodu olmadan bu kurallardan: girdi raster (ve yükseklik modeli, nesneler) ya da "
                "nesnelerin ızgarası, aracın ayarları ve vermesi gereken: raster (her örnek, 32 bitte float32'ye yuvarlanmış; null "
                "değersiz; rule exact bit bit, f32ulp float32'de en çok bir birim) ya da yollar (köşeler dünyada, sayılar), notlar ya "
                "da ret. Üretici scripts/fixtures/distance_cases.py.",
        "cases": cases(),
    }
    text = json.dumps(doc, ensure_ascii=False, separators=(",", ":")) + "\n"
    if check:
        if OUT.read_text("utf-8") != text:
            raise SystemExit(f"{OUT.name} güncel değil. Yeniden yazın: python3 {sys.argv[0]}; farkı okuyun.")
        cross_check()
        print(f"{OUT.name}: {len(doc['cases'])} durum; güncel.")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, "utf-8")
    print(f"{OUT.name}: {len(doc['cases'])} durum yazıldı.")


if __name__ == "__main__":
    main()
