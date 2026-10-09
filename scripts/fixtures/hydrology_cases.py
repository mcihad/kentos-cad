#!/usr/bin/env python3
"""Hidroloji (docs/adr/0235): the eight tools' rules written from the ADR with Python, without KentOS code, and the
cases the raster core plays (crates/shared/raster/tests/all/hydrology.rs) and the web's WASM module
(apps/web/src/io/raster.wasm.test.ts) bit for bit; only what goes through libm (Çoklu yön's exponent, D∞'s angle,
TWI's logarithm) is held to one unit in the last place of the 32-bit result.

- Çukur doldur: ε = 0 a plain priority flood from the outlets (a cell set when first reached); ε > 0 Dijkstra with the
  candidate max(z, f + ε·d) over every neighbour (the least fixpoint; no Zhou-style tracing here).
- Akış yönü: steepest descent (f − f')/d, the first in the neighbour order on ties; outlets out; flats by Barnes,
  Lehman and Mulla (2014b): low and high edges, labels, the two breadth-first distances, mask 2·l + (U − u).
- Akış birikimi: every cell's sum pulled from its neighbours in their order once their own sums are known (Kahn), D8,
  Çoklu yön (Quinn's contour lengths, a fixed or Qin's adaptive exponent) and D∞ (Tarboton's facets).
- TWI: ln(a / max(tan β, floor)), Horn's gradient through the row's axes.
- Döküm noktası, Noktadan havza, Havzalar (main, sub, route crossings) and Dere ağı (links, Strahler, Shreve, the lines
  and their lengths), their areas with ADR 0234's rings (scripts/fixtures/raster_vector_cases.py).

Cross-checks with GRASS GIS 8.4 (--check, when `grass` is on the path): r.terraflow's filled surface cell for cell on
DEMs without nodata; its single flow direction on inner cells with one steepest neighbour; its D8 accumulation on a
dome without pits or flats; r.water.outlet's basins over our directions (turned into r.watershed's coding).

    python3 scripts/fixtures/hydrology_cases.py          # write fixtures/hydrology/v1/cases.json
    python3 scripts/fixtures/hydrology_cases.py --check  # compare, and cross-check with GRASS
"""

import heapq
import json
import math
import shutil
import struct
import subprocess
import sys
import tempfile
from collections import deque
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import raster_ops_cases as ro  # noqa: E402  (places, samples)
import raster_vector_cases as rv  # noqa: E402  (ADR 0234's rings, Douglas–Peucker, chords)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/hydrology/v1/cases.json"

# §2: the neighbours' order, their ESRI and TauDEM codes, Quinn's contour lengths.
N = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)]
ESRI = [1, 2, 4, 8, 16, 32, 64, 128]
TAUDEM = [1, 8, 7, 6, 5, 4, 3, 2]
CONTOUR = [0.5, 0.354, 0.5, 0.354, 0.5, 0.354, 0.5, 0.354]
# §5: D∞'s facets (e₁ the square neighbour, e₂ the diagonal one), counter-clockwise from east–north-east.
FACETS = [(0, 7), (6, 7), (6, 5), (4, 5), (4, 3), (2, 3), (2, 1), (0, 1)]
NOFLOW = -1
GRS80_A = 6378137.0
GRS80_F = 1.0 / 298.257222101


def f32(x):
    return struct.unpack("<f", struct.pack("<f", float(x)))[0]


# ── The grid (§2) ─────────────────────────────────────────────────────────

def degree_metres(phi):
    """The metres of a degree east and north at latitude phi on GRS80 (ADR 0231 §2)."""
    e2 = GRS80_F * (2.0 - GRS80_F)
    r = phi * math.pi / 180.0
    s = math.sin(r)
    w = 1.0 - e2 * s * s
    n = GRS80_A / math.sqrt(w)
    m = GRS80_A * (1.0 - e2) / (w * math.sqrt(w))
    k = math.pi / 180.0
    return (n * math.cos(r) * k, m * k)


class Grid:
    """A DEM's band 1 as the tools see it: the values (NaN none), the place and each row's axes in metres."""

    def __init__(self, r, geographic=False):
        self.r = r
        self.w, self.h = r.width, r.height
        self.affine = r.affine
        self.geographic = geographic
        self.z = []
        for j in range(self.h):
            for i in range(self.w):
                v = r.get(i, j, 0)
                self.z.append(math.nan if v is None or math.isnan(v) else float(v))
        self.valid = [not math.isnan(v) for v in self.z]
        self.axes = [self.row_axes(j) for j in range(self.h)]

    def row_axes(self, j):
        x0, a, b, y0, c, d = self.affine
        if self.geographic:
            lat = y0 + c * (self.w / 2.0) + d * (j + 0.5)
            ke, kn = degree_metres(lat)
            return [a * ke, b * ke, c * kn, d * kn]
        return [a, b, c, d]

    def step(self, j, di, dj):
        a, b, c, d = self.axes[j] if 0 <= j < self.h else self.row_axes(j)
        x = a * di + b * dj
        y = c * di + d * dj
        return math.sqrt(x * x + y * y)

    def area(self, j):
        a, b, c, d = self.axes[j]
        return abs(a * d - b * c)

    def width(self, j):
        return math.sqrt(self.area(j))

    def inside(self, i, j):
        return 0 <= i < self.w and 0 <= j < self.h

    def ok(self, i, j):
        return self.inside(i, j) and self.valid[j * self.w + i]

    def outlet(self, k):
        j, i = divmod(k, self.w)
        if not self.valid[k]:
            return False
        return any(not self.ok(i + di, j + dj) for di, dj in N)


# ── Çukur doldur (§3) ─────────────────────────────────────────────────────

def fill(g, eps):
    """The least surface f ≥ z whose every inner cell has a neighbour n with f(n) + ε·d ≤ f(c)."""
    n = g.w * g.h
    f = list(g.z)
    heap = []
    if eps == 0.0:
        closed = [False] * n
        for k in range(n):
            if g.outlet(k):
                closed[k] = True
                heapq.heappush(heap, (f[k], k))
        while heap:
            v, k = heapq.heappop(heap)
            j, i = divmod(k, g.w)
            for di, dj in N:
                a, b = i + di, j + dj
                if g.ok(a, b) and not closed[b * g.w + a]:
                    m = b * g.w + a
                    closed[m] = True
                    f[m] = max(g.z[m], v)
                    heapq.heappush(heap, (f[m], m))
        return f
    best = [math.inf] * n
    done = [False] * n
    for k in range(n):
        if g.outlet(k):
            best[k] = g.z[k]
            heapq.heappush(heap, (best[k], k))
    while heap:
        v, k = heapq.heappop(heap)
        if done[k] or v != best[k]:
            continue
        done[k] = True
        j, i = divmod(k, g.w)
        for di, dj in N:
            a, b = i + di, j + dj
            if not g.ok(a, b):
                continue
            m = b * g.w + a
            if done[m]:
                continue
            # f(m) ≥ f(k) + ε·d(m, k): the step from m (its row) back to k.
            cand = max(g.z[m], v + eps * g.step(b, -di, -dj))
            if cand < best[m]:
                best[m] = cand
                heapq.heappush(heap, (cand, m))
    return [best[k] if g.valid[k] else math.nan for k in range(n)]


# ── Akış yönü (§4) ────────────────────────────────────────────────────────

def d8(g, f):
    """Each valid cell's direction (0–7), NOFLOW for none; outlets without a lower neighbour flow out."""
    dirs = [NOFLOW] * (g.w * g.h)
    for k in range(g.w * g.h):
        if not g.valid[k]:
            continue
        j, i = divmod(k, g.w)
        best, at = 0.0, NOFLOW
        for q, (di, dj) in enumerate(N):
            if g.ok(i + di, j + dj):
                s = (f[k] - f[(j + dj) * g.w + i + di]) / g.step(j, di, dj)
                if s > best:
                    best, at = s, q
        if at == NOFLOW and g.outlet(k):
            at = next(q for q, (di, dj) in enumerate(N) if not g.ok(i + di, j + dj))
        dirs[k] = at
    return dirs


def bfs(g, seeds, ok):
    """Breadth-first distances (seeds 1) over the cells `ok` lets in."""
    dist = {}
    queue = deque()
    for s in seeds:
        if s not in dist:
            dist[s] = 1
            queue.append(s)
    while queue:
        k = queue.popleft()
        j, i = divmod(k, g.w)
        for di, dj in N:
            a, b = i + di, j + dj
            if g.inside(a, b):
                m = b * g.w + a
                if m not in dist and ok(k, m):
                    dist[m] = dist[k] + 1
                    queue.append(m)
    return dist


def resolve_flats(g, f, dirs):
    """Barnes, Lehman and Mulla (2014b): the flats' cells given directions by the mask; counts the cells resolved."""
    n = g.w * g.h

    def neighbours(k):
        j, i = divmod(k, g.w)
        for q, (di, dj) in enumerate(N):
            if g.ok(i + di, j + dj):
                yield q, (j + dj) * g.w + i + di

    low, high = [], []
    for k in range(n):
        if not g.valid[k]:
            continue
        if dirs[k] != NOFLOW:
            if any(dirs[m] == NOFLOW and f[m] == f[k] for _, m in neighbours(k)):
                low.append(k)
        elif any(f[m] > f[k] for _, m in neighbours(k)):
            high.append(k)
    label = [0] * n
    count = 0
    for s in low:
        if label[s]:
            continue
        count += 1
        label[s] = count
        queue = deque([s])
        while queue:
            k = queue.popleft()
            for _, m in neighbours(k):
                if not label[m] and f[m] == f[s]:
                    label[m] = count
                    queue.append(m)
    high = [k for k in high if label[k]]
    inner = lambda k, m: g.valid[m] and label[m] == label[k] and dirs[m] == NOFLOW
    away = bfs(g, high, inner)
    top = {}
    for k, u in away.items():
        top[label[k]] = max(top.get(label[k], 0), u)
    toward = bfs(g, low, inner)
    mask = {}
    for k, l in toward.items():
        mask[k] = 2 * l + (top[label[k]] - away[k] if k in away else 0)
    resolved = 0
    for k in range(n):
        if not g.valid[k] or dirs[k] != NOFLOW or not label[k]:
            continue
        best, at = mask[k], NOFLOW
        for q, m in neighbours(k):
            if label[m] == label[k] and m in mask and mask[m] < best:
                best, at = mask[m], q
        assert at != NOFLOW, "a drainable flat's cell without a lower mask"
        dirs[k] = at
        resolved += 1
    return resolved


def directions(g, fill_first):
    f = fill(g, 0.0) if fill_first else list(g.z)
    dirs = d8(g, f)
    flats = resolve_flats(g, f, dirs)
    return f, dirs, flats


def receiver(g, dirs, k):
    """The cell the D8 direction leads to; None out of the raster, into no value, or without a direction."""
    if dirs[k] == NOFLOW:
        return None
    j, i = divmod(k, g.w)
    di, dj = N[dirs[k]]
    return (j + dj) * g.w + i + di if g.ok(i + di, j + dj) else None


def coded(g, dirs, coding):
    codes = ESRI if coding == "esri" else TAUDEM
    return [255 if not g.valid[k] else (0 if dirs[k] == NOFLOW else codes[dirs[k]]) for k in range(g.w * g.h)]


# ── Akış birikimi (§5) ────────────────────────────────────────────────────

def shares(g, f, dirs, method, exponent):
    """Each cell's receivers and their shares (only shares above 0)."""
    out = []
    for k in range(g.w * g.h):
        if not g.valid[k]:
            out.append([])
            continue
        j, i = divmod(k, g.w)
        got = []
        if method == "mfd":
            lower = []
            for q, (di, dj) in enumerate(N):
                if g.ok(i + di, j + dj):
                    t = (f[k] - f[(j + dj) * g.w + i + di]) / g.step(j, di, dj)
                    if t > 0.0:
                        lower.append((q, t))
            if lower:
                if exponent > 0.0:
                    p = exponent
                else:
                    e = max(t for _, t in lower)
                    p = 8.9 * min(e, 1.0) + 1.1
                weights = [(q, CONTOUR[q] * t ** p) for q, t in lower]
                total = 0.0
                for _, w in weights:
                    total += w
                for q, w in weights:
                    di, dj = N[q]
                    s = w / total
                    if s > 0.0:
                        got.append(((j + dj) * g.w + i + di, s))
        elif method == "dinf":
            best, chosen = 0.0, None
            for e1, e2 in FACETS:
                (a1, b1), (a2, b2) = N[e1], N[e2]
                if not (g.ok(i + a1, j + b1) and g.ok(i + a2, j + b2)):
                    continue
                f0, fa, fb = f[k], f[(j + b1) * g.w + i + a1], f[(j + b2) * g.w + i + a2]
                d1 = g.step(j, a1, b1)
                d2 = g.step(j, a2 - a1, b2 - b1)
                s1 = (f0 - fa) / d1
                s2 = (fa - fb) / d2
                r = math.atan2(s2, s1)
                s = math.sqrt(s1 * s1 + s2 * s2)
                alpha = math.atan2(d2, d1)
                if r < 0.0:
                    r, s = 0.0, s1
                elif r > alpha:
                    r, s = alpha, (f0 - fb) / math.sqrt(d1 * d1 + d2 * d2)
                if s > best:
                    best, chosen = s, (e1, e2, r, alpha)
            if chosen:
                e1, e2, r, alpha = chosen
                q = r / alpha
                for nb, s in ((e1, 1.0 - q), (e2, q)):
                    if s > 0.0:
                        di, dj = N[nb]
                        got.append(((j + dj) * g.w + i + di, s))
        if method == "d8" or not got:
            to = receiver(g, dirs, k)
            got = [(to, 1.0)] if to is not None else []
        out.append(got)
    return out


def accumulate(g, share, own):
    """A(c) = own(c) + Σ pay(n → c)·A(n), the neighbours in their order, each cell once all its donors are known."""
    n = g.w * g.h
    pay = [dict() for _ in range(n)]  # pay[c][n] = n's share to c
    indeg = [0] * n
    for k in range(n):
        for to, s in share[k]:
            pay[to][k] = s
            indeg[to] += 1
    acc = [math.nan] * n
    queue = deque(k for k in range(n) if g.valid[k] and indeg[k] == 0)
    while queue:
        k = queue.popleft()
        j, i = divmod(k, g.w)
        a = own(k)
        for di, dj in N:
            if g.ok(i + di, j + dj):
                m = (j + dj) * g.w + i + di
                if m in pay[k]:
                    a += pay[k][m] * acc[m]
        acc[k] = a
        for to, _ in share[k]:
            indeg[to] -= 1
            if indeg[to] == 0:
                queue.append(to)
    assert all(not g.valid[k] or not math.isnan(acc[k]) for k in range(n)), "a cycle"
    return acc


def accumulation(g, fill_first, method, exponent, unit):
    f, dirs, _ = directions(g, fill_first)
    share = shares(g, f, dirs, method, exponent)
    if unit == "cells":
        return f, dirs, accumulate(g, share, lambda k: 1.0)
    acc = accumulate(g, share, lambda k: g.area(k // g.w))
    if unit == "sca":
        acc = [a / g.width(k // g.w) if g.valid[k] else a for k, a in enumerate(acc)]
    return f, dirs, acc


# ── Topografik nemlilik indisi (§6) ───────────────────────────────────────

def horn_tan(g, f, k):
    j, i = divmod(k, g.w)
    c = f[k]
    w = []
    for dj in (-1, 0, 1):
        for di in (-1, 0, 1):
            v = f[(j + dj) * g.w + i + di] if g.ok(i + di, j + dj) else math.nan
            w.append(c if math.isnan(v) else v)
    pi = ((w[2] + 2.0 * w[5] + w[8]) - (w[0] + 2.0 * w[3] + w[6])) / 8.0
    pj = ((w[6] + 2.0 * w[7] + w[8]) - (w[0] + 2.0 * w[1] + w[2])) / 8.0
    a, b, cc, d = g.axes[j]
    det = a * d - b * cc
    k1, k2, k3, k4 = 1.0 * d / det, -1.0 * cc / det, -1.0 * b / det, 1.0 * a / det
    gx = k1 * pi + k2 * pj
    gy = k3 * pi + k4 * pj
    return math.sqrt(gx * gx + gy * gy)


def wetness(g, fill_first, method, exponent, slope):
    f, dirs, area = accumulation(g, fill_first, method, exponent, "area")
    floor = slope / 100.0
    out, floored = [], 0
    for k in range(g.w * g.h):
        if not g.valid[k]:
            out.append(math.nan)
            continue
        t = horn_tan(g, f, k)
        if t < floor:
            t, floored = floor, floored + 1
        out.append(math.log(area[k] / g.width(k // g.w) / t))
    return out, floored


# ── Döküm noktası, havzalar (§7–§9) ───────────────────────────────────────

def metres(g, du, dv, j):
    a, b, c, d = g.axes[j] if 0 <= j < g.h else g.row_axes(j)
    x = a * du + b * dv
    y = c * du + d * dv
    return math.sqrt(x * x + y * y)


def snap(g, cells, x, y, r):
    """§7: the pour cell of a point and its distance; None when no cell takes it."""
    u, v = ro.place_in(g.affine, x, y)
    j0 = math.floor(v)
    if r == 0.0:
        i, j = math.floor(u), j0
        if not g.ok(i, j):
            return None
        return j * g.w + i, metres(g, i + 0.5 - u, j + 0.5 - v, j0)
    best = None
    for j in range(g.h):
        for i in range(g.w):
            if not g.ok(i, j):
                continue
            dist = metres(g, i + 0.5 - u, j + 0.5 - v, j0)
            if dist > r:
                continue
            key = (-cells[j * g.w + i], dist, j, i)
            if best is None or key < best[0]:
                best = (key, j * g.w + i, dist)
    return None if best is None else (best[1], best[2])


def upstream(g, dirs, start, stop=frozenset()):
    """The cells whose D8 path passes through `start` (it included), not entering `stop`."""
    seen = {start}
    queue = deque([start])
    while queue:
        k = queue.popleft()
        j, i = divmod(k, g.w)
        for di, dj in N:
            a, b = i + di, j + dj
            if g.ok(a, b):
                m = b * g.w + a
                if m not in seen and m not in stop and receiver(g, dirs, m) == k:
                    seen.add(m)
                    queue.append(m)
    return seen


def area_of(g, cells):
    total = 0.0
    for k in sorted(cells):
        total += g.area(k // g.w)
    return total


def polygon(g, cells):
    return [rv.to_world(r, g.affine) for r in rv.region_rings({(k % g.w, k // g.w) for k in cells})]


def features(kind, fields):
    return {"kind": kind, "fields": fields, "numbers": [], "rings": [], "sizes": [], "xy": []}


def push(fs, rings, numbers):
    if fs["kind"] == "polygons":
        fs["rings"].append(len(rings))
    for ring in rings:
        fs["sizes"].append(len(ring))
        for p in ring:
            fs["xy"] += list(p)
    fs["numbers"] += numbers


def points_of(shapes):
    out = []
    for s in shapes:
        out += [rv.xy(s["p"])] + [rv.xy(q["p"]) for q in s.get("parts") or []]
    return out


def pour_points(g, fill_first, shapes, r):
    f, dirs, cells = accumulation(g, fill_first, "d8", 0.0, "cells")
    _, _, area = accumulation(g, fill_first, "d8", 0.0, "area")
    fs = features("points", ["Nokta", "Birikim", "Alan", "Uzaklık"])
    skipped = []
    for n, (x, y) in enumerate(points_of(shapes)):
        got = snap(g, cells, x, y, r)
        if got is None:
            skipped.append(n)
            continue
        k, dist = got
        cx, cy = ro.centre(g.affine, k % g.w, k // g.w)
        push(fs, [[[cx, cy]]], [n + 1, cells[k], area[k], dist])
    return {"features": fs, "notes": {"skipped": skipped}}


def watershed(g, fill_first, shapes, r):
    f, dirs, cells = accumulation(g, fill_first, "d8", 0.0, "cells")
    owner, skipped = {}, []
    pts = points_of(shapes)
    for n, (x, y) in enumerate(pts):
        got = snap(g, cells, x, y, r)
        if got is None:
            skipped.append(n)
        else:
            owner[got[0]] = n
    stop = frozenset(owner)
    fs = features("polygons", ["Havza", "Alan"])
    empty = []
    by_point = {n: k for k, n in owner.items()}
    for n in range(len(pts)):
        if n in skipped:
            continue
        if n not in by_point:
            empty.append(n)
            continue
        k = by_point[n]
        basin = upstream(g, dirs, k, stop - {k})
        push(fs, polygon(g, basin), [n + 1, area_of(g, basin)])
    return {"features": fs, "notes": {"skipped": skipped, "empty": empty}}


def streams_of(g, dirs, area, threshold):
    """§10: the stream cells' links: each link's cells, end cell (the junction it runs into), orders."""
    n = g.w * g.h
    if threshold <= 0.0:
        threshold = max(a for k, a in enumerate(area) if g.valid[k]) / 100.0
    stream = [g.valid[k] and area[k] >= threshold for k in range(n)]
    ups = [0] * n
    for k in range(n):
        if stream[k]:
            to = receiver(g, dirs, k)
            if to is not None:
                assert stream[to]
                ups[to] += 1
    starts = [k for k in range(n) if stream[k] and ups[k] != 1]
    number = {k: x + 1 for x, k in enumerate(starts)}
    links = []
    link_of = {}
    for k in starts:
        cells = [k]
        x = k
        end = None
        while True:
            to = receiver(g, dirs, x)
            if to is None:
                break
            if ups[to] != 1:
                end = to
                break
            cells.append(to)
            x = to
        for c in cells:
            link_of[c] = number[k]
        links.append({"cells": cells, "end": end, "down": number[end] if end is not None else 0})
    feeders = {x + 1: [] for x in range(len(links))}
    for x, l in enumerate(links):
        if l["down"]:
            feeders[l["down"]].append(x + 1)
    strahler, shreve = {}, {}

    def order(x):
        if x in strahler:
            return
        up = feeders[x]
        for u in up:
            order(u)
        if not up:
            strahler[x], shreve[x] = 1, 1
            return
        top = max(strahler[u] for u in up)
        strahler[x] = top + 1 if sum(1 for u in up if strahler[u] == top) >= 2 else top
        shreve[x] = sum(shreve[u] for u in up)

    for x in range(1, len(links) + 1):
        order(x)
    return threshold, stream, links, link_of, strahler, shreve


def streams(g, fill_first, threshold, eps):
    f, dirs, area = accumulation(g, fill_first, "d8", 0.0, "area")
    threshold, _, links, _, strahler, shreve = streams_of(g, dirs, area, threshold)
    fs = features("lines", ["Bağ", "Sıra", "Shreve", "Uzunluk", "Düşü", "Eğim", "Alan", "Aşağı"])
    for x, l in enumerate(links):
        path = l["cells"] + ([l["end"]] if l["end"] is not None else [])
        length = 0.0
        for a, b in zip(path, path[1:]):
            ja, ia = divmod(a, g.w)
            jb, ib = divmod(b, g.w)
            length += g.step(ja, ib - ia, jb - ja)
        drop = f[path[0]] - f[path[-1]]
        slope = drop / length if length > 0.0 else 0.0
        pts = [(k % g.w + 0.5, k // g.w + 0.5) for k in path]
        kept = rv.dp(pts, eps)
        line = [list(ro.point_of(g.affine, u, v)) for u, v in kept]
        push(fs, [line], [x + 1, strahler[x + 1], shreve[x + 1], length, drop, slope, area[l["cells"][-1]], l["down"]])
    return {"features": fs, "notes": {"threshold": threshold, "links": len(links)}}


def route_station(shape, p):
    """ADR 0188's reading on a straight route: the distance along it of the point nearest to p."""
    pts = [rv.xy(shape["a"]), rv.xy(shape["b"])] if shape["kind"] == "line" else [rv.xy(q) for q in shape["pts"]]
    best, at, run = None, 0.0, 0.0
    for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
        dx, dy = x1 - x0, y1 - y0
        length = math.hypot(dx, dy)
        t = max(0.0, min(1.0, ((p[0] - x0) * dx + (p[1] - y0) * dy) / (length * length)))
        qx, qy = x0 + t * dx, y0 + t * dy
        d = math.hypot(p[0] - qx, p[1] - qy)
        if best is None or d < best:
            best, at = d, run + t * length
        run += length
    return at


def crossed_cells(g, shape):
    out = set()
    for path in rv.chord_paths(shape):
        cs = [ro.place_in(g.affine, x, y) for x, y in path]
        for p, q in zip(cs, cs[1:]):
            out |= {j * g.w + i for i, j in rv.chord_cells(p, q, g.w, g.h)}
    return out


def basins(g, fill_first, mode, threshold, least, shapes):
    f, dirs, area = accumulation(g, fill_first, "d8", 0.0, "area")
    n = g.w * g.h
    found = []  # (numbers before the count, cells)
    notes = {}
    if mode == "main":
        end = {}
        for k in range(n):
            if not g.valid[k]:
                continue
            x, path = k, []
            while x not in end:
                path.append(x)
                to = receiver(g, dirs, x)
                if to is None:
                    end[x] = x
                    break
                x = to
            for c in path:
                end[c] = end[x]
        groups = {}
        for k, e in end.items():
            groups.setdefault(e, set()).add(k)
        found = [([], groups[e]) for e in sorted(groups)]
        fields = ["Havza", "Alan"]
    else:
        threshold, stream, links, link_of, strahler, _ = streams_of(g, dirs, area, threshold)
        notes["threshold"] = threshold
        if mode == "sub":
            label = dict(link_of)
            queue = deque(sorted(link_of))
            while queue:
                k = queue.popleft()
                j, i = divmod(k, g.w)
                for di, dj in N:
                    a, b = i + di, j + dj
                    if g.ok(a, b):
                        m = b * g.w + a
                        if m not in label and receiver(g, dirs, m) == k:
                            label[m] = label[k]
                            queue.append(m)
            groups = {}
            for k, x in label.items():
                groups.setdefault(x, set()).add(k)
            found = [([x, strahler[x]], groups[x]) for x in sorted(groups)]
            fields = ["Bağ", "Sıra", "Alan"]
        else:
            fields = ["Havza", "Km", "Sıra", "Alan"]
            for shape in shapes:
                crossed = {k for k in crossed_cells(g, shape) if stream[k]}
                here = []
                for k in crossed:
                    to = receiver(g, dirs, k)
                    if to is None or to not in crossed:
                        cx, cy = ro.centre(g.affine, k % g.w, k // g.w)
                        here.append((route_station(shape, (cx, cy)), k // g.w, k % g.w, k))
                for km, _, _, k in sorted(here):
                    found.append(([km, strahler[link_of[k]]], upstream(g, dirs, k)))
    fs = features("polygons", fields)
    dropped = 0
    count = 0
    for numbers, cells in found:
        a = area_of(g, cells)
        if a < least:
            dropped += 1
            continue
        count += 1
        lead = numbers if mode == "sub" else [count] + numbers
        push(fs, polygon(g, cells), lead + [a])
    notes["dropped"] = dropped
    return {"features": fs, "notes": notes}


# ── The cases ─────────────────────────────────────────────────────────────

def noise(i, j, seed):
    """A fixed pseudo-random value in [0, 1) of a cell (integers only: the same everywhere)."""
    h = (i * 73856093 ^ j * 19349663 ^ seed * 83492791) & 0xFFFFFFFF
    h = (h ^ (h >> 13)) * 1274126177 & 0xFFFFFFFF
    return ((h ^ (h >> 16)) & 0xFFFF) / 65536.0


def dem(w, h, height, affine=(1000.0, 10.0, 0.0, 2000.0, 0.0, -10.0), sample="f32", nodata="nan", holes=()):
    """A DEM of `height(i, j)`; `holes` without a value (NaN, or the nodata value in an integer band)."""
    empty = math.nan if nodata == "nan" else nodata
    vals = []
    for j in range(h):
        for i in range(w):
            vals.append(empty if (i, j) in holes else height(i, j))
    return ro.Raster(list(affine), w, h, vals, 1, sample, nodata)


def valley(i, j, seed=1):
    """A valley falling west and south with a pit, a terrace and noise."""
    z = 100.0 + 0.9 * i + 0.6 * j + 3.0 * abs(i - 6.5) * 0.4 + 2.0 * noise(i, j, seed)
    if (i, j) in ((4, 4), (5, 4)):
        z -= 6.0
    return z


def terrace(i, j):
    """Flats on purpose: a plateau draining by one gap, a closed basin (an undrainable flat unfilled), ramps."""
    if 3 <= i <= 9 and 2 <= j <= 6:
        return 50.0
    if (i, j) == (10, 4):
        return 49.0
    if 2 <= i <= 4 and 9 <= j <= 11:
        return 40.0
    return 52.0 + 0.5 * i + 0.25 * j if not (i == 0 or j == 0) else 30.0 + i + j


# The drainage net's channels: (cell-space vertices, elevation at the first, at the last), downstream.
CHANNELS = [
    ([(3.5, 4.5), (12.5, 10.5), (20.5, 15.5), (31.5, 25.5)], 64.0, 6.0),   # the main stream to the south-east corner
    ([(2.5, 21.5), (12.5, 18.5), (20.5, 15.5)], 58.0, 32.6),               # a tributary from the west
    ([(6.5, 25.0), (9.5, 19.5)], 52.0, 44.0),                               # its own tributary from the south
    ([(14.5, 1.5), (13.5, 6.5), (12.5, 10.5)], 70.0, 45.2),                # a tributary from the north
    ([(28.5, 2.5), (25.5, 10.5), (24.4, 18.6)], 66.0, 26.0),               # a tributary from the north-east
]


def channel_height(i, j):
    """The nearest channel point's elevation and the distance to it (cell centres, cells)."""
    p = (i + 0.5, j + 0.5)
    best = None
    for pts, z0, z1 in CHANNELS:
        lengths = [math.hypot(b[0] - a[0], b[1] - a[1]) for a, b in zip(pts, pts[1:])]
        total, run = sum(lengths), 0.0
        for (a, b), length in zip(zip(pts, pts[1:]), lengths):
            dx, dy = b[0] - a[0], b[1] - a[1]
            t = max(0.0, min(1.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (length * length)))
            d = math.hypot(p[0] - a[0] - t * dx, p[1] - a[1] - t * dy)
            z = z0 + (z1 - z0) * (run + t * length) / total
            if best is None or d < best[0]:
                best = (d, z)
            run += length
    return best


def network(i, j):
    """A dendritic net: valleys 1.8 m a cell deep at the sides, a little noise (pits the fill takes)."""
    d, z = channel_height(i, j)
    return z + 1.8 * d + 0.6 * noise(i, j, 7)


def cone(i, j):
    """Ties on purpose: a symmetric cone (equal drops to several neighbours)."""
    return 100.0 - 2.0 * max(abs(i - 5), abs(j - 5)) - (abs(i - 5) + abs(j - 5))


def tool(kind, **kw):
    return {"kind": kind, "band": 1, **kw}


def raster_case(name, r, spec, values, sample, rule="exact", notes=None, geographic=False):
    out = {"raster": {"width": r.width, "height": r.height, "sample": sample, "rule": rule,
                      "values": [None if isinstance(v, float) and math.isnan(v) else v for v in values]}}
    if notes is not None:
        out["notes"] = notes
    return {"name": name, "input": r.json(), "geographic": geographic, "tool": spec, "shapes": [], "expect": out}


def as_sample(sample, values):
    if sample == "f32":
        return [f32(v) if not math.isnan(v) else math.nan for v in values]
    return list(values)


def tool_cases(name, r, geographic=False, shapes=(), skip=()):
    """Every tool over one DEM with the settings worth seeing there."""
    g = Grid(r, geographic)
    filled_sample = "f64" if r.sample == "f64" else "f32"
    cases = []
    add = cases.append
    if "fill" not in skip:
        f = fill(g, 0.0)
        raised = sum(1 for k in range(g.w * g.h) if g.valid[k] and f[k] > g.z[k])
        add(raster_case(f"{name}-doldur", r, tool("fill", slope=0.0, result="filled"), as_sample(filled_sample, f),
                        filled_sample, notes={"cells": raised}, geographic=geographic))
        depth = [f[k] - g.z[k] if g.valid[k] else math.nan for k in range(g.w * g.h)]
        add(raster_case(f"{name}-derinlik", r, tool("fill", slope=0.0, result="depth"), as_sample("f32", depth), "f32",
                        notes={"cells": raised}, geographic=geographic))
        fe = fill(g, 0.5 / 100.0)
        raised = sum(1 for k in range(g.w * g.h) if g.valid[k] and fe[k] > g.z[k])
        add(raster_case(f"{name}-doldur-egim", r, tool("fill", slope=0.5, result="filled"), as_sample(filled_sample, fe),
                        filled_sample, notes={"cells": raised}, geographic=geographic))
    for fill_first in (True, False):
        f, dirs, flats = directions(g, fill_first)
        none = sum(1 for k in range(g.w * g.h) if g.valid[k] and dirs[k] == NOFLOW)
        suffix = "" if fill_first else "-dolgusuz"
        add(raster_case(f"{name}-yon{suffix}", r, tool("flowDirection", fill=fill_first, coding="esri"),
                        [float(v) for v in coded(g, dirs, "esri")], "u8", notes={"cells": flats, "empty": none},
                        geographic=geographic))
    f, dirs, _ = directions(g, True)
    add(raster_case(f"{name}-yon-taudem", r, tool("flowDirection", fill=True, coding="taudem"),
                    [float(v) for v in coded(g, dirs, "taudem")], "u8", geographic=geographic))
    rule_of = {"d8": "exact", "mfd": "f32ulp", "dinf": "f32ulp"}
    for method, exponent, unit in (("d8", 0.0, "cells"), ("d8", 0.0, "area"), ("d8", 0.0, "sca"), ("mfd", 0.0, "cells"),
                                   ("mfd", 1.0, "area"), ("mfd", 1.1, "cells"), ("dinf", 0.0, "cells"), ("dinf", 0.0, "sca")):
        _, _, acc = accumulation(g, True, method, exponent, unit)
        rule = rule_of[method] if not geographic or method != "d8" or unit == "cells" else "f32ulp"
        add(raster_case(f"{name}-birikim-{method}-{unit}-{exponent:g}", r,
                        tool("flowAccumulation", fill=True, method=method, exponent=exponent, unit=unit),
                        as_sample("f32", acc), "f32", rule=rule, geographic=geographic))
    for method, exponent, slope in (("mfd", 0.0, 0.1), ("d8", 0.0, 0.1), ("dinf", 0.0, 1.0)):
        twi, floored = wetness(g, True, method, exponent, slope)
        add(raster_case(f"{name}-twi-{method}-{slope:g}", r,
                        tool("wetness", fill=True, method=method, exponent=exponent, slope=slope),
                        as_sample("f32", twi), "f32", rule="f32ulp", notes={"cells": floored}, geographic=geographic))
    return cases


def feature_case(name, r, spec, shapes, result, geographic=False, rule="exact"):
    return {"name": name, "input": r.json(), "geographic": geographic, "tool": spec, "shapes": list(shapes),
            "expect": {**result, "rule": rule}}


def point(x, y):
    return {"kind": "point", "p": {"x": x, "y": y}}


def line(pts):
    return {"kind": "polyline", "pts": [{"x": x, "y": y} for x, y in pts]}


def cases():
    out = []
    v = dem(14, 11, valley)
    out += tool_cases("vadi", v)
    t = dem(13, 13, terrace)
    out += tool_cases("teras", t)
    holes = {(6, 6), (7, 6), (6, 7), (0, 5), (13, 10)}
    out += tool_cases("degersiz", dem(14, 11, lambda i, j: valley(i, j, 3), holes=holes))
    out += tool_cases("dikdortgen", dem(12, 10, lambda i, j: valley(i, j, 5), affine=(1000.0, 10.0, 0.0, 2000.0, 0.0, -6.0)))
    theta = math.radians(25.0)
    turned = (500.0, 4.0 * math.cos(theta), 4.0 * math.sin(theta), 900.0, 4.0 * math.sin(theta), -4.0 * math.cos(theta))
    out += tool_cases("donuk", dem(12, 10, lambda i, j: valley(i, j, 9), affine=turned))
    geo = (32.8, 0.0005, 0.0, 39.92, 0.0, -0.0004)
    out += tool_cases("cografi", dem(12, 10, lambda i, j: valley(i, j, 11), affine=geo), geographic=True)
    ints = dem(12, 10, lambda i, j: float(round(valley(i, j, 13) * 10.0)), sample="i16", nodata=-32768.0, holes={(3, 3)})
    out += [c for c in tool_cases("tamsayi", ints) if "doldur" in c["name"] or "yon" in c["name"]]
    f64 = dem(10, 8, lambda i, j: valley(i, j, 17) + 1e-9 * i, sample="f64")
    out += [c for c in tool_cases("f64", f64) if "doldur" in c["name"]]
    # No pit at all: the fill leaves every cell (Çukur doldur's “Doldurulacak çukur yok”).
    out += [c for c in tool_cases("koni", dem(11, 11, cone))
            if "yon" in c["name"] or "birikim-d8-cells" in c["name"] or c["name"] == "koni-doldur"]

    # Pour points, watersheds, basins and streams over the drainage net.
    net = dem(32, 26, network)
    g = Grid(net)
    x0, y0 = net.affine[0], net.affine[3]
    at = lambda i, j: (x0 + 10.0 * (i + 0.5), y0 - 10.0 * (j + 0.5))
    pts = [point(*at(30, 24)), point(*at(20, 15)), point(*at(12, 18)), point(x0 - 500.0, y0 + 500.0),
           point(*at(20, 15)), point(*at(9, 3))]
    for snap_r in (0.0, 25.0):
        tag = f"{snap_r:g}"
        out.append(feature_case(f"ag-dokum-{tag}", net, tool("pourPoint", fill=True, snap=snap_r), pts,
                                pour_points(g, True, pts, snap_r)))
        out.append(feature_case(f"ag-havza-{tag}", net, tool("watershed", fill=True, snap=snap_r), pts,
                                watershed(g, True, pts, snap_r)))
    for least in (0.0, 2000.0):
        out.append(feature_case(f"ag-ana-havzalar-{least:g}", net,
                                tool("basins", fill=True, mode="main", threshold=0.0, least=least), [],
                                basins(g, True, "main", 0.0, least, [])))
    for threshold in (0.0, 1000.0, 3000.0):
        out.append(feature_case(f"ag-alt-havzalar-{threshold:g}", net,
                                tool("basins", fill=True, mode="sub", threshold=threshold, least=0.0), [],
                                basins(g, True, "sub", threshold, 0.0, [])))
        for eps in (0.0, 1.0):
            out.append(feature_case(f"ag-dere-{threshold:g}-{eps:g}", net,
                                    tool("streams", fill=True, threshold=threshold, simplify=eps), [],
                                    streams(g, True, threshold, eps)))
    # En küçük alan drops the small sub-basins; the kept keep their link's number.
    out.append(feature_case("ag-alt-havzalar-1000-500", net,
                            tool("basins", fill=True, mode="sub", threshold=1000.0, least=500.0), [],
                            basins(g, True, "sub", 1000.0, 500.0, [])))
    gv = Grid(v)
    # 600 m² is the small basin's area: kept at 600 (not less), dropped at 601, the kept renumbered.
    for least in (0.0, 600.0, 601.0):
        out.append(feature_case(f"vadi-ana-havzalar-{least:g}", v,
                                tool("basins", fill=True, mode="main", threshold=0.0, least=least), [],
                                basins(gv, True, "main", 0.0, least, [])))
    road = [line([(x0 + 3.0, y0 - 133.0), (x0 + 160.0, y0 - 141.0), (x0 + 317.0, y0 - 128.0)]),
            line([(x0 + 223.0, y0 - 3.0), (x0 + 236.0, y0 - 256.0)])]
    for least in (0.0, 3000.0, 20000.0):
        out.append(feature_case(f"ag-guzergah-{least:g}", net,
                                tool("basins", fill=True, mode="route", threshold=3000.0, least=least), road,
                                basins(g, True, "route", 3000.0, least, road), rule="km"))
    out.append(feature_case("ag-dere-dolgusuz", net, tool("streams", fill=False, threshold=2000.0, simplify=0.0), [],
                            streams(g, False, 2000.0, 0.0)))

    # Refusals.
    out.append({"name": "ret-esik", "input": net.json(), "geographic": False, "tool": tool("streams", fill=True,
                threshold=-1.0, simplify=0.0), "shapes": [], "expect": {"refused": "Eşik alanı"}})
    out.append({"name": "ret-us", "input": net.json(), "geographic": False, "tool": tool("flowAccumulation", fill=True,
                method="mfd", exponent=200.0, unit="cells"), "shapes": [], "expect": {"refused": "Çoklu yönün üssü"}})
    out.append({"name": "ret-egim", "input": net.json(), "geographic": False, "tool": tool("fill", slope=150.0,
                result="filled"), "shapes": [], "expect": {"refused": "En küçük eğim"}})
    out.append({"name": "ret-guzergah", "input": net.json(), "geographic": False, "tool": tool("basins", fill=True,
                mode="route", threshold=0.0, least=0.0), "shapes": [], "expect": {"refused": "Güzergâh"}})
    return out


# ── GRASS cross-checks (§13) ──────────────────────────────────────────────

def write_tif(r, path):
    """A raster as a float64 GeoTIFF in TUREF / TM30 (GRASS's project takes its system)."""
    import numpy as np
    from osgeo import gdal, osr
    gdal.UseExceptions()
    d = gdal.GetDriverByName("GTiff").Create(str(path), r.width, r.height, 1, gdal.GDT_Float64)
    d.SetGeoTransform(r.affine)
    s = osr.SpatialReference()
    s.ImportFromEPSG(5254)
    d.SetProjection(s.ExportToWkt())
    d.GetRasterBand(1).WriteArray(np.array([r.values[j * r.width:(j + 1) * r.width] for j in range(r.height)],
                                           dtype=np.float64))
    d = None


def grass_run(r, script, more=()):
    """Runs a GRASS shell script over a DEM written as dem.tif (and `more`: name, raster); gives the folder."""
    folder = Path(tempfile.mkdtemp(prefix="kentos-grass-"))
    write_tif(r, folder / "dem.tif")
    for name, raster in more:
        write_tif(raster, folder / f"{name}.tif")
    (folder / "run.sh").write_text("set -e\nr.in.gdal -o input=dem.tif output=dem --q\ng.region raster=dem\n" + script)
    subprocess.run(["grass", "--tmp-project", "dem.tif", "--exec", "bash", "run.sh"], cwd=folder, check=True,
                   capture_output=True, timeout=600)
    return folder


def read_tif(path):
    from osgeo import gdal
    ds = gdal.Open(str(path))
    a = ds.GetRasterBand(1).ReadAsArray().astype(float)
    nd = ds.GetRasterBand(1).GetNoDataValue()
    out = [float(v) for v in a.ravel()]
    return [math.nan if nd is not None and v == nd else v for v in out]


def noisy(w, h, seed):
    """A noisy DEM of float32 values (r.terraflow keeps heights as float32: these it holds exactly)."""
    return dem(w, h, lambda i, j: 100.0 + 0.3 * j + 0.2 * i + 3.0 * math.sin(i / 4.0) * math.cos(j / 5.0)
               + 3.0 * noise(i, j, seed), affine=(500000.0, 10.0, 0.0, 4420000.0, 0.0, -10.0), sample="f32")


def cross_check():
    if not shutil.which("grass"):
        print("GRASS yok: çapraz denetim atlandı.")
        return
    out = "r.out.gdal input={0} output={0}.tif format=GTiff type=Float64 --q -c\n"
    checked = {"fill": 0, "d8": 0, "acc": 0, "basin": 0}
    for seed in (21, 22, 23):
        r = noisy(60, 45, seed)
        folder = grass_run(r, "r.terraflow elevation=dem filled=filled direction=dir accumulation=acc -s\n"
                           + out.format("filled") + out.format("dir"))
        g = Grid(r)
        f = fill(g, 0.0)
        theirs = read_tif(folder / "filled.tif")
        bad = [k for k in range(len(f)) if f[k] != theirs[k]]
        if bad:
            raise SystemExit(f"GRASS r.terraflow'un doldurduğu yüzey farklı: {len(bad)} hücre (tohum {seed})")
        checked["fill"] += len(f)
        their_dir = read_tif(folder / "dir.tif")
        for k in range(g.w * g.h):
            j, i = divmod(k, g.w)
            if i in (0, g.w - 1) or j in (0, g.h - 1):
                continue
            s = [(f[k] - f[(j + dj) * g.w + i + di]) / g.step(j, di, dj) for di, dj in N]
            top = max(s)
            if top <= 0.0 or s.count(top) > 1:
                continue
            if ESRI[s.index(top)] != int(their_dir[k]):
                raise SystemExit(f"GRASS'ın D8'i farklı: hücre {i}, {j}")
            checked["d8"] += 1
        shutil.rmtree(folder)
    # A surface without pits or flats (filled with a slope, then held as float32): r.watershed's single flow
    # direction and accumulation on the inner cells (r.terraflow crashes on such a surface; GRASS 8.4.2).
    base = noisy(60, 45, 25)
    lifted = fill(Grid(base), 1.0)
    surface = ro.Raster(base.affine, base.width, base.height, lifted, 1, "f32", "nan")
    gs = Grid(surface)
    assert all(d != NOFLOW for d in d8(gs, gs.z)), "the lifted surface has a flat"
    folder = grass_run(surface, "r.watershed -s elevation=dem accumulation=acc --q\n" + out.format("acc"))
    _, dirs, cells = accumulation(gs, True, "d8", 0.0, "cells")
    theirs = read_tif(folder / "acc.tif")
    edge = {k for k in range(gs.w * gs.h) if k % gs.w in (0, gs.w - 1) or k // gs.w in (0, gs.h - 1)}
    for k in range(gs.w * gs.h):
        if k in edge or upstream(gs, dirs, k) & edge:
            continue
        # r.watershed marks a count it may have underestimated negative: its size is the count.
        if cells[k] != abs(theirs[k]):
            raise SystemExit(f"GRASS r.watershed'in birikimi farklı: hücre {k % gs.w}, {k // gs.w}")
        checked["acc"] += 1
    shutil.rmtree(folder)
    # r.water.outlet over our directions (r.watershed's coding: 1 NE … 8 E, counter-clockwise).
    r = noisy(60, 45, 24)
    g = Grid(r)
    _, dirs, cells = accumulation(g, True, "d8", 0.0, "cells")
    grass_code = {0: 8, 1: 7, 2: 6, 3: 5, 4: 4, 5: 3, 6: 2, 7: 1}
    coded_dirs = [float(grass_code[d]) if d != NOFLOW else 0.0 for d in dirs]
    outlets = sorted(range(len(cells)), key=lambda k: -cells[k])[3:6]
    script = "r.in.gdal -o input=dir.tif output=dir --q\n"
    for x, k in enumerate(outlets):
        cx, cy = ro.centre(r.affine, k % g.w, k // g.w)
        script += f"r.water.outlet input=dir output=b{x} coordinates={cx},{cy} --q\n" + out.format(f"b{x}")
    dr = ro.Raster(r.affine, r.width, r.height, coded_dirs, 1, "f64", None)
    folder = grass_run(r, script, more=(("dir", dr),))
    for x, k in enumerate(outlets):
        theirs = read_tif(folder / f"b{x}.tif")
        their = {m for m, v in enumerate(theirs) if not math.isnan(v) and v != 0.0}
        mine = upstream(g, dirs, k)
        if their != mine:
            raise SystemExit(f"GRASS r.water.outlet'in havzası farklı: {len(their ^ mine)} hücre")
        checked["basin"] += len(mine)
    shutil.rmtree(folder)
    print(f"GRASS ile aynı: {checked['fill']} doldurulmuş hücre, {checked['d8']} D8 yönü, {checked['acc']} birikim, "
          f"{checked['basin']} havza hücresi.")


def main():
    check = "--check" in sys.argv
    doc = {
        "format": "kentos.hydrology-cases",
        "version": 1,
        "note": "ADR 0235'in araçları, KentOS kodu olmadan bu kurallardan: girdi DEM (ve noktalar ya da güzergâhlar), aracın "
                "ayarları ve vermesi gereken: raster (her örnek; null değersiz; rule exact bit bit, f32ulp float32'de en çok bir "
                "birim) ya da nesneler (alanların halkaları, çizgiler, noktalar; köşeler dünyada; alanların sayıları), notlar "
                "ya da ret. Üretici scripts/fixtures/hydrology_cases.py.",
        "cases": cases(),
    }
    text = json.dumps(doc, ensure_ascii=False, separators=(",", ":")) + "\n"
    if check:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            sys.exit(f"{OUT.relative_to(ROOT)} güncel değil: python3 {sys.argv[0]} ile yeniden yazın.")
        cross_check()
        print(f"cases.json: {len(doc['cases'])} durum; güncel.")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, "utf-8")
    print(f"cases.json: {len(doc['cases'])} durum yazıldı.")
    cross_check()


if __name__ == "__main__":
    main()
