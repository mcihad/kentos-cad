"""Raster ve vektör dönüşümü (docs/adr/0234), worked out here from the ADR's
rules with no KentOS code: Rasterleştir, Rasterden alan, Rasterden çizgi,
Rasterden nokta, Çizgi yakala, Alan kapat and Eğrilere kot ver over small
rasters and drawings.

    python3 scripts/fixtures/raster_vector_cases.py           # writes the file
    python3 scripts/fixtures/raster_vector_cases.py --check   # writes nothing; compares

Writes fixtures/raster-vector/v1/cases.json: each case's input (a raster:
place, size, bands, sample type, nodata, values; or the objects and their
value texts), the tool's settings and what it must give: a raster (size,
place, every sample, null where empty; the run's notes), the features (each
one's value, text and tag; the areas' rings; every vertex in the world),
a refusal; and Eğrilere kot ver's elevations.

How it is worked out here (independent of the engine):
- Rasterleştir: a closed shape's cells by ADR 0233 §6's centre rule
  (raster_ops_cases.py: exact rationals on the float64 vertices); an open
  shape's chords (arcs at 0.1 mm by §3's formula, float64) taken to cell
  space by §2's formula, and each chord's cells as the set of half-open
  squares the closed segment meets, decided with exact rationals (Fraction)
  cell by cell over the chord's box; overlaps by their rule, sums exact and
  rounded once.
- Regions by breadth-first floods (4 or 8 neighbours); their rings from
  the graph of their boundary sides (each side directed with the region on
  its left) joined at the corners by §4's rule (at a saddle the side of the
  diagonal cell follows); corners where the direction changes.
- Zhang–Suen over the whole image at once (numpy), not border by border;
  the skeleton's m-adjacency, paths, spurs and Douglas–Peucker as §5 writes
  them.
- Çizgi yakala and Alan kapat over the whole raster, not windows.
- Eğrilere kot ver's crossings with exact rationals.

Cross-checks: GDAL's Polygonize (its 4 and 8 connectedness) gives the same
regions (values, areas, ring counts); GDAL's Rasterize gives the same cells
for the areas (the centre rule) and, with ALL_TOUCHED, for the lines (cases in
general position).
"""
import json
import math
import sys
from collections import deque
from fractions import Fraction
from pathlib import Path

import numpy as np
from osgeo import gdal, ogr

sys.path.insert(0, str(Path(__file__).resolve().parent))
import raster_ops_cases as ro  # noqa: E402  (ADR 0233's centre rule and sample rounding)

gdal.UseExceptions()

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/raster-vector/v1/cases.json"

PLACE = [1000.0, 2.0, 0.0, 2000.0, 0.0, -2.0]
CHORD = 1e-4
TAU = 2 * math.pi


# ── Values and their texts (§2) ───────────────────────────────────────────

def value_text(v, sample):
    """The sample type's shortest decimal that reads back the same number, without an exponent."""
    if v is None or (isinstance(v, float) and math.isnan(v)):
        return ""
    if sample == "f32":
        return np.format_float_positional(np.float32(v), unique=True, trim="-")
    if sample == "f64":
        return np.format_float_positional(np.float64(v), unique=True, trim="-")
    return str(int(v))


def read_number(text):
    """`kentos.statistics/1`'s reading as the cases use it: a decimal point or comma, else none."""
    if text is None:
        return None
    t = text.strip().replace(",", ".")
    try:
        v = float(t)
    except ValueError:
        return None
    return v if math.isfinite(v) else None


# ── Rasterleştir (§3) ─────────────────────────────────────────────────────

def nice_cell(s):
    k = math.floor(math.log10(s))
    for kk in (k + 1, k, k - 1):
        for m in (5.0, 2.5, 2.0, 1.0):
            c = m * 10.0 ** kk if kk >= 0 else m / 10.0 ** (-kk)
            if c <= s:
                return c
    return 10.0 ** (k - 1)


def grid_of_box(b, cell):
    """ADR 0232 §3's grid of a box, in its own float64 order."""
    minx, miny, maxx, maxy = b
    if cell > 0:
        s = cell
    else:
        w, h = maxx - minx, maxy - miny
        short = min(w, h) if w > 0 and h > 0 else max(w, h)
        s = nice_cell(short / 250.0)
    x0 = math.floor(minx / s) * s
    y1 = math.ceil(maxy / s) * s
    width = max(1, math.ceil((maxx - x0) / s))
    height = max(1, math.ceil((y1 - miny) / s))
    return [x0, s, 0.0, y1, 0.0, -s], width, height


def arc_inner(c, r, a0, sweep):
    """§3: n = ⌈|sweep| / (2·acos(1 − 10⁻⁴ / r))⌉ chords at equal angles; the points between the ends."""
    step = 2 * math.acos(1 - CHORD / r) if r > CHORD else math.pi
    n = min(max(math.ceil(abs(sweep) / step), 1), 1 << 16)
    return [(c[0] + r * math.cos(a0 + sweep * k / n), c[1] + r * math.sin(a0 + sweep * k / n)) for k in range(1, n)]


def bulge_arc(a, b, bulge):
    dx, dy = b[0] - a[0], b[1] - a[1]
    chord = math.hypot(dx, dy)
    k = (1 - bulge * bulge) / (4 * bulge)
    c = ((a[0] + b[0]) / 2 - dy * k, (a[1] + b[1]) / 2 + dx * k)
    r = chord * (1 + bulge * bulge) / (4 * abs(bulge))
    return c, r, math.atan2(a[1] - c[1], a[0] - c[0]), 4 * math.atan(bulge)


def path_points(pts, bulges, closed):
    p = [pts[0]]
    n = len(pts)
    for k in range(n if closed else n - 1):
        a, b = pts[k], pts[(k + 1) % n]
        bulge = bulges[k] if bulges and k < len(bulges) else 0.0
        if abs(bulge) > 1e-12:
            c, r, a0, sweep = bulge_arc(a, b, bulge)
            p.extend(arc_inner(c, r, a0, sweep))
        p.append(b)
    return p


def xy(p):
    return (p["x"], p["y"])


def chord_paths(shape):
    """A shape's chord paths in the world: its vertices with its arcs' chord points between."""
    k = shape["kind"]
    if k == "line":
        return [[xy(shape["a"]), xy(shape["b"])]]
    if k == "polyline":
        out = [path_points([xy(p) for p in shape["pts"]], shape.get("bulges"), False)]
        for part in shape.get("parts") or []:
            out.append(path_points([xy(p) for p in part["pts"]], part.get("bulges"), False))
        return out
    if k == "polygon":
        out = []
        def rings(r):
            out.append(path_points([xy(p) for p in r["pts"]], r.get("bulges"), True))
            for h in r.get("holes") or []:
                out.append(path_points([xy(p) for p in h["pts"]], h.get("bulges"), True))
        rings(shape)
        for part in shape.get("parts") or []:
            rings(part)
        return out
    if k == "arc":
        sweep = (shape["a1"] - shape["a0"]) % TAU
        if sweep < 1e-12:
            sweep = TAU
        c, r, a0 = xy(shape["c"]), shape["r"], shape["a0"]
        at = lambda t: (c[0] + r * math.cos(t), c[1] + r * math.sin(t))
        return [[at(a0)] + arc_inner(c, r, a0, sweep) + [at(a0 + sweep)]]
    if k == "circle":
        c, r = xy(shape["c"]), shape["r"]
        at = lambda t: (c[0] + r * math.cos(t), c[1] + r * math.sin(t))
        return [[at(0.0)] + arc_inner(c, r, 0.0, TAU) + [at(TAU)]]
    if k == "point":
        return []
    raise ValueError(k)


def point_list(shape):
    return [xy(shape["p"])] + [xy(q["p"]) for q in shape.get("parts") or []]


def interval(lo, lo_closed, hi, hi_closed):
    return lo < hi or (lo == hi and lo_closed and hi_closed)


def segment_meets_cell(p, q, i, j):
    """Whether the closed segment p–q (Fractions) has a point in [i, i+1) × [j, j+1)."""
    lo, lo_c, hi, hi_c = Fraction(0), True, Fraction(1), True

    def bound(d, s, a, closed_a, b, closed_b):
        """t's range where a ≤ s + t·d ≤ b (each end closed or open)."""
        nonlocal lo, lo_c, hi, hi_c
        if d == 0:
            ok_a = s > a or (closed_a and s == a)
            ok_b = s < b or (closed_b and s == b)
            if not (ok_a and ok_b):
                lo, hi, lo_c, hi_c = Fraction(1), Fraction(0), True, True
            return
        ta, tb = (a - s) / d, (b - s) / d
        if d > 0:
            pairs = ((ta, closed_a, "lo"), (tb, closed_b, "hi"))
        else:
            pairs = ((tb, closed_b, "lo"), (ta, closed_a, "hi"))
        for t, closed, side in pairs:
            if side == "lo":
                if t > lo or (t == lo and not closed):
                    lo, lo_c = t, closed if t > lo else (lo_c and closed)
            else:
                if t < hi or (t == hi and not closed):
                    hi, hi_c = t, closed if t < hi else (hi_c and closed)

    bound(q[0] - p[0], p[0], Fraction(i), True, Fraction(i + 1), False)
    bound(q[1] - p[1], p[1], Fraction(j), True, Fraction(j + 1), False)
    return interval(lo, lo_c, hi, hi_c)


def chord_cells(p, q, w, h):
    """The cells within the grid the closed chord p–q (cell space, float64) meets, exactly."""
    P, Q = (Fraction(p[0]), Fraction(p[1])), (Fraction(q[0]), Fraction(q[1]))
    i0, i1 = math.floor(min(P[0], Q[0])), math.floor(max(P[0], Q[0]))
    j0, j1 = math.floor(min(P[1], Q[1])), math.floor(max(P[1], Q[1]))
    out = set()
    for j in range(max(j0, 0), min(j1, h - 1) + 1):
        for i in range(max(i0, 0), min(i1, w - 1) + 1):
            if segment_meets_cell(P, Q, i, j):
                out.add((i, j))
    return out


def margin_ok(pts, affine):
    """The chord points' cell-space places lie at least 10⁻⁹ from a cell line (no tie the last bit could turn)."""
    for x, y in pts:
        u, v = ro.place_in(affine, x, y)
        for t in (u, v):
            if abs(t - round(t)) < 1e-9 and t != round(t):
                return False
    return True


AREA_KINDS = ("polygon", "circle")
SAMPLES_FIT = {"f32": None, "f64": None, "i32": (-2147483647.0, 2147483647.0), "u8": (0.0, 254.0)}
NODATA = {"f32": None, "f64": None, "i32": -2147483648.0, "u8": 255.0}


def fit(sample, v):
    if sample == "f32":
        return v if math.isfinite(ro.f32(v)) else None
    if sample == "f64":
        return v
    r = ro.round_half_away(v)
    lo, hi = SAMPLES_FIT[sample]
    return float(r) if lo <= r <= hi else None


def rasterize(shapes, texts, tool, cell, grid):
    """Rasterleştir over the objects: the raster, its notes, or a refusal."""
    value, overlap, sample = tool["value"], tool["overlap"], tool["sample"]
    items, unread = [], 0
    for o, s in enumerate(shapes):
        v = value if texts is None else read_number(texts[o])
        if v is None:
            unread += 1
            continue
        items.append((o, s, v))
    if not items:
        return {"refused": "Değeri okunan nesne yok" if unread else "Rasterleştirilecek nesne yok"}
    pts = []
    for _, s, _ in items:
        pts += point_list(s) if s["kind"] == "point" else [p for path in chord_paths(s) for p in path]
    if grid is not None:
        affine, w, h = grid["affine"], grid["width"], grid["height"]
    else:
        b = (min(p[0] for p in pts), min(p[1] for p in pts), max(p[0] for p in pts), max(p[1] for p in pts))
        affine, w, h = grid_of_box(b, cell)
    assert margin_ok(pts, affine), "a chord point on a cell line: choose the case away from ties"
    if overlap != "count":
        for _, _, v in items:
            if fit(sample, v) is None:
                return {"refused": "sonucun türüne sığmıyor"}
    per_cell = {}
    touched = set()
    for o, s, v in items:
        if s["kind"] in AREA_KINDS:
            cells = ro.inside_cells(s, affine, w, h)
        elif s["kind"] == "point":
            cells = set()
            for p in point_list(s):
                u, vv = ro.place_in(affine, p[0], p[1])
                i, j = math.floor(u), math.floor(vv)
                if 0 <= i < w and 0 <= j < h:
                    cells.add((i, j))
        else:
            cells = set()
            for path in chord_paths(s):
                cp = [ro.place_in(affine, x, y) for x, y in path]
                for a, bb in zip(cp, cp[1:]):
                    cells |= chord_cells(a, bb, w, h)
        if cells:
            touched.add(o)
        for c in cells:
            per_cell.setdefault(c, []).append(o)
    values = {o: v for o, _, v in items}
    out, empty = [], 0
    for j in range(h):
        for i in range(w):
            objs = sorted(per_cell.get((i, j), []))
            if not objs:
                empty += 1
                out.append(None if NODATA[sample] is None else NODATA[sample])
                continue
            vs = [values[o] for o in objs]
            r = {"last": vs[-1], "first": vs[0], "max": max(vs), "min": min(vs),
                 "sum": float(sum(Fraction(x) for x in vs)) + 0.0, "count": float(len(vs))}[overlap]
            f = fit(sample, r)
            if f is None:
                return {"refused": "sonucun türüne sığmıyor"}
            out.append(ro.stored(sample, f))
    return {"raster": {"width": w, "height": h, "affine": affine, "sample": sample, "values": out},
            "notes": {"taken": len(items), "unread": unread, "outside": len(items) - len(touched), "empty": empty}}


# ── Regions and rings (§4) ────────────────────────────────────────────────

def regions(r, band, eight):
    """Each cell's region (0: none; 1, 2, … by their first cells row by row) and each region's value."""
    w, h = r.width, r.height
    lab = [[0] * w for _ in range(h)]
    vals = []
    steps = [(1, 0), (-1, 0), (0, 1), (0, -1)] + ([(1, 1), (1, -1), (-1, 1), (-1, -1)] if eight else [])
    for j in range(h):
        for i in range(w):
            v = r.get(i, j, band)
            if lab[j][i] or v is None or math.isnan(v):
                continue
            vals.append(v)
            n = len(vals)
            lab[j][i] = n
            queue = deque([(i, j)])
            while queue:
                a, b = queue.popleft()
                for di, dj in steps:
                    x, y = a + di, b + dj
                    if 0 <= x < w and 0 <= y < h and not lab[y][x]:
                        u = r.get(x, y, band)
                        if u is not None and not math.isnan(u) and u == v:
                            lab[y][x] = n
                            queue.append((x, y))
    return lab, vals


def ring_corners(cells):
    """The rings of a set of cells: their boundary sides directed with the cells on the left (top −u, bottom +u,
    left +v, right −v), joined at each corner; at a corner where two sides leave (two diagonal cells of the set), the
    side coming in along one cell goes on along the other's. Each ring's corners from its top-most, left-most one."""
    edges = {}  # start vertex -> list of (end vertex, owner cell)
    def add(s, e, cell):
        edges.setdefault(s, []).append((e, cell))
    for (i, j) in cells:
        if (i, j - 1) not in cells:
            add((i + 1, j), (i, j), (i, j))           # top, west
        if (i, j + 1) not in cells:
            add((i, j + 1), (i + 1, j + 1), (i, j))   # bottom, east
        if (i - 1, j) not in cells:
            add((i, j), (i, j + 1), (i, j))           # left, south
        if (i + 1, j) not in cells:
            add((i + 1, j + 1), (i + 1, j), (i, j))   # right, north
    used = set()
    rings = []
    for s in sorted(edges, key=lambda v: (v[1], v[0])):
        for k in range(len(edges[s])):
            if (s, k) in used:
                continue
            ring = []
            v, kk = s, k
            while (v, kk) not in used:
                used.add((v, kk))
                e, owner = edges[v][kk]
                ring.append((v, e))
                outs = edges[e]
                if len(outs) == 1:
                    nk = 0
                else:
                    # A saddle: the side of the cell diagonal to the owner (not the owner's own).
                    nk = next(x for x, (_, c) in enumerate(outs) if c != owner)
                v, kk = e, nk
            # Corners: where the direction changes.
            corners = []
            n = len(ring)
            for x in range(n):
                (a, b), (c, d) = ring[x - 1], ring[x]
                d1 = (b[0] - a[0], b[1] - a[1])
                d2 = (d[0] - c[0], d[1] - c[1])
                if d1 != d2:
                    corners.append(c)
            m = min(range(len(corners)), key=lambda x: (corners[x][1], corners[x][0]))
            rings.append(corners[m:] + corners[:m])
    return rings


def twice_area(r):
    return sum(r[k][0] * r[(k + 1) % len(r)][1] - r[(k + 1) % len(r)][0] * r[k][1] for k in range(len(r)))


def region_rings(cells):
    rs = ring_corners(cells)
    outline = [r for r in rs if twice_area(r) < 0]
    assert len(outline) == 1, rs
    holes = sorted((r for r in rs if twice_area(r) > 0), key=lambda r: (r[0][1], r[0][0]))
    return outline + holes


def to_world(ring, affine, close=False):
    """Cell-space corners to the world; turned when the affine mirrors cell space (an outline stays counter-clockwise)."""
    x0, a, b, y0, c, d = affine
    pts = [list(p) for p in ring]
    if a * d - b * c > 0 and len(pts) > 1:
        pts = [pts[0]] + pts[1:][::-1]
    return [list(ro.point_of(affine, float(p[0]), float(p[1]))) for p in pts]


def features(kind):
    return {"kind": kind, "values": [], "texts": [], "tags": [], "rings": [], "sizes": [], "xy": []}


def push(f, rings, affine, value, text, tag, cell_space=True):
    f["values"].append(None if value is None or (isinstance(value, float) and math.isnan(value)) else value)
    f["texts"].append(text)
    f["tags"].append(tag)
    if f["kind"] == "polygons":
        f["rings"].append(len(rings))
    for r in rings:
        f["sizes"].append(len(r))
        for p in r:
            f["xy"] += p


def to_polygons(r, band, eight):
    lab, vals = regions(r, band, eight)
    by = {}
    for j in range(r.height):
        for i in range(r.width):
            if lab[j][i]:
                by.setdefault(lab[j][i], set()).add((i, j))
    f = features("polygons")
    for n in range(1, len(vals) + 1):
        rings = [to_world(x, r.affine) for x in region_rings(by[n])]
        push(f, rings, r.affine, vals[n - 1], value_text(vals[n - 1], r.sample), 0)
    return {"features": f}


# ── Thinning, the skeleton and its paths (§5) ─────────────────────────────

NB = [(0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1)]


def thin(mask):
    """Zhang–Suen (1984) with Lü and Wang's (1986) 3 ≤ B ≤ 6, over the whole image at once, each sub-iteration from the
    image as it began; outside is background."""
    m = np.pad(np.array(mask, dtype=np.int32), 1)
    while True:
        changed = False
        for second in (False, True):
            p = [m[0:-2, 1:-1], m[0:-2, 2:], m[1:-1, 2:], m[2:, 2:], m[2:, 1:-1], m[2:, 0:-2], m[1:-1, 0:-2], m[0:-2, 0:-2]]
            centre = m[1:-1, 1:-1]
            b = sum(p)
            a = sum(((p[k] == 0) & (p[(k + 1) % 8] == 1)).astype(np.int32) for k in range(8))
            p2, p4, p6, p8 = p[0], p[2], p[4], p[6]
            if second:
                c = (p2 * p4 * p8 == 0) & (p2 * p6 * p8 == 0)
            else:
                c = (p2 * p4 * p6 == 0) & (p4 * p6 * p8 == 0)
            gone = (centre == 1) & (b >= 3) & (b <= 6) & (a == 1) & c
            if gone.any():
                changed = True
                m[1:-1, 1:-1] = np.where(gone, 0, centre)
        if not changed:
            return m[1:-1, 1:-1].astype(np.uint8)


def m_bits(m, i, j):
    h, w = m.shape
    on = lambda x, y: 0 <= x < w and 0 <= y < h and m[y, x] != 0
    c = [on(i + di, j + dj) for di, dj in NB]
    n, e, s, w_ = c[0], c[2], c[4], c[6]
    return [n, c[1] and not n and not e, e, c[3] and not e and not s, s, c[5] and not s and not w_, w_, c[7] and not w_ and not n]


def skeleton_paths(m):
    h, w = m.shape
    walked = {}
    deg = lambda i, j: sum(m_bits(m, i, j))
    out = []

    def follow(start, d0):
        path = [start]
        prev, d = start, d0
        while True:
            walked.setdefault(prev, set()).add(d)
            cur = (prev[0] + NB[d][0], prev[1] + NB[d][1])
            walked.setdefault(cur, set()).add((d + 4) % 8)
            path.append(cur)
            if cur == start or deg(*cur) != 2:
                return path
            bits = m_bits(m, *cur)
            nxt = [k for k in range(8) if bits[k] and k not in walked[cur]]
            if not nxt:
                return path
            prev, d = cur, nxt[0]

    for j in range(h):
        for i in range(w):
            if m[j, i] and deg(i, j) not in (0, 2):
                bits = m_bits(m, i, j)
                for d in range(8):
                    if bits[d] and d not in walked.get((i, j), set()):
                        out.append(follow((i, j), d))
    for j in range(h):
        for i in range(w):
            if m[j, i] and deg(i, j) == 2 and not walked.get((i, j)):
                bits = m_bits(m, i, j)
                out.append(follow((i, j), bits.index(True)))
    return out


def clean(m, least):
    found = skeleton_paths(m)
    if least == 0:
        return found
    deg = lambda p: sum(m_bits(m, *p))
    spurs = []
    for p in found:
        a, b = p[0], p[-1]
        if a == b or len(p) - 1 >= least:
            continue
        if deg(a) == 1 and deg(b) >= 3:
            spurs += p[:-1]
        elif deg(b) == 1 and deg(a) >= 3:
            spurs += p[1:]
    m = m.copy()
    for i, j in spurs:
        m[j, i] = 0
    return [p for p in skeleton_paths(m) if len(p) - 1 >= least]


def mark(pts, eps, keep, a, b):
    stack = [(a, b)]
    while stack:
        a, b = stack.pop()
        if b <= a + 1:
            continue
        x0, y0 = pts[a]
        x1, y1 = pts[b]
        dx, dy = x1 - x0, y1 - y0
        length = math.sqrt(dx * dx + dy * dy)
        best, at = -1.0, a
        for i in range(a + 1, b):
            x, y = pts[i]
            d = math.sqrt((x - x0) * (x - x0) + (y - y0) * (y - y0)) if length == 0 else abs(dx * (y - y0) - dy * (x - x0)) / length
            if d > best:
                best, at = d, i
        if best > eps:
            keep[at] = True
            stack.append((a, at))
            stack.append((at, b))


def dp(pts, eps):
    if len(pts) < 3:
        return list(pts)
    keep = [False] * len(pts)
    keep[0] = keep[-1] = True
    mark(pts, eps, keep, 0, len(pts) - 1)
    return [p for p, k in zip(pts, keep) if k]


def dp_closed(pts, eps):
    n = len(pts)
    if n < 4:
        return list(pts)
    x0, y0 = pts[0]
    far, best = 1, -1.0
    for k in range(1, n - 1):
        x, y = pts[k]
        d = (x - x0) * (x - x0) + (y - y0) * (y - y0)
        if d > best:
            best, far = d, k
    keep = [False] * n
    keep[0] = keep[far] = keep[-1] = True
    mark(pts, eps, keep, 0, far)
    mark(pts, eps, keep, far, n - 1)
    return [p for p, k in zip(pts, keep) if k]


def lines_of(paths, off, eps, affine):
    f = features("lines")
    for p in paths:
        pts = [(i + off[0] + 0.5, j + off[1] + 0.5) for i, j in p]
        kept = dp_closed(pts, eps) if len(pts) > 2 and pts[0] == pts[-1] else dp(pts, eps)
        push(f, [[list(ro.point_of(affine, u, v)) for u, v in kept]], affine, math.nan, "", 0)
    return f


def colour(r, i, j):
    """A cell's first three value bands (or its one), none without a value."""
    rgb = r.value_bands() >= 3
    c = [r.get(i, j, k) for k in range(3 if rgb else 1)]
    if any(x is None or math.isnan(x) for x in c):
        return None
    return c + [0.0] * (3 - len(c)), rgb


def near(c, t, tol, rgb):
    if rgb:
        dr, dg, db = c[0] - t[0], c[1] - t[1], c[2] - t[2]
        return dr * dr + dg * dg + db * db <= tol * tol
    return abs(c[0] - t[0]) <= tol


def to_lines(r, band, select, spur, eps):
    mask = np.zeros((r.height, r.width), dtype=np.uint8)
    for j in range(r.height):
        for i in range(r.width):
            if select[0] == "color":
                got = colour(r, i, j)
                on = got is not None and near(got[0], select[1], select[2], True)
            else:
                v = r.get(i, j, band)
                on = v is not None and not math.isnan(v) and (v != 0 if select[0] == "nonZero" else select[1] <= v <= select[2])
            mask[j, i] = 1 if on else 0
    return {"features": lines_of(clean(thin(mask), spur), (0, 0), eps, r.affine)}


# ── Rasterden nokta (§6) ──────────────────────────────────────────────────

def to_points(r, band, mode, k):
    f = features("points")
    for j in range(r.height):
        for i in range(r.width):
            v = r.get(i, j, band)
            if v is None or math.isnan(v):
                continue
            tag = 0
            if mode == "step" and not (i % k == k // 2 and j % k == k // 2):
                continue
            if mode == "extrema":
                around = [r.get(i + di, j + dj, band) for dj in range(-k, k + 1) for di in range(-k, k + 1) if (di, dj) != (0, 0)]
                around = [u for u in around if u is not None and not math.isnan(u)]
                if not around:
                    continue
                if all(u < v for u in around):
                    tag = 1
                elif all(u > v for u in around):
                    tag = 2
                else:
                    continue
            f["values"].append(v)
            f["texts"].append(value_text(v, r.sample))
            f["tags"].append(tag)
            f["sizes"].append(1)
            f["xy"] += list(ro.centre(r.affine, i, j))
    return {"features": f}


# ── Çizgi yakala, Alan kapat (§7, §8) ─────────────────────────────────────

def click_cell(r, x, y):
    u, v = ro.place_in(r.affine, x, y)
    i, j = math.floor(u), math.floor(v)
    if not (0 <= i < r.width and 0 <= j < r.height):
        return None
    return i, j


def flood(r, start, target, tol, rgb, eight):
    steps = [(1, 0), (-1, 0), (0, 1), (0, -1)] + ([(1, 1), (1, -1), (-1, 1), (-1, -1)] if eight else [])
    seen = {start}
    queue = deque([start])
    while queue:
        a, b = queue.popleft()
        for di, dj in steps:
            c = (a + di, b + dj)
            if c in seen or not (0 <= c[0] < r.width and 0 <= c[1] < r.height):
                continue
            got = colour(r, *c)
            if got is not None and near(got[0], target, tol, rgb):
                seen.add(c)
                queue.append(c)
    return seen


def capture_line(r, x, y, tol, spur, eps):
    at = click_cell(r, x, y)
    if at is None:
        return {"refused": "rasterin dışında"}
    best = None
    for j in range(at[1] - 3, at[1] + 4):
        for i in range(at[0] - 3, at[0] + 4):
            if not (0 <= i < r.width and 0 <= j < r.height):
                continue
            got = colour(r, i, j)
            if got is None:
                continue
            c, rgb = got
            lum = 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2] if rgb else c[0]
            d = max(abs(i - at[0]), abs(j - at[1]))
            if best is None or lum < best[0] or (lum == best[0] and d < best[1]):
                best = (lum, d, (i, j), c, rgb)
    if best is None:
        return {"refused": "değeri olan hücre yok"}
    _, _, seed, target, rgb = best
    cells = flood(r, seed, target, tol, rgb, True)
    if len(cells) > 4194304:
        return {"refused": "çizgi değil gibi"}
    mask = np.zeros((r.height, r.width), dtype=np.uint8)
    for i, j in cells:
        mask[j, i] = 1
    return {"features": lines_of(clean(thin(mask), spur), (0, 0), eps, r.affine)}


def crossing(rings):
    edges = [(ri, k, r[k], r[(k + 1) % len(r)]) for ri, r in enumerate(rings) for k in range(len(r))]
    orient = lambda a, b, c: (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    on = lambda a, b, c: min(a[0], b[0]) <= c[0] <= max(a[0], b[0]) and min(a[1], b[1]) <= c[1] <= max(a[1], b[1])

    def meet(a, b, c, d):
        d1, d2, d3, d4 = orient(c, d, a), orient(c, d, b), orient(a, b, c), orient(a, b, d)
        if ((d1 > 0 > d2) or (d1 < 0 < d2)) and ((d3 > 0 > d4) or (d3 < 0 < d4)):
            return True
        return (d1 == 0 and on(c, d, a)) or (d2 == 0 and on(c, d, b)) or (d3 == 0 and on(a, b, c)) or (d4 == 0 and on(a, b, d))

    def folds(p, q, s):
        return orient(p, q, s) == 0 and (q[0] - p[0]) * (s[0] - q[0]) + (q[1] - p[1]) * (s[1] - q[1]) < 0

    for x in range(len(edges)):
        for y in range(x + 1, len(edges)):
            ra, ka, a, b = edges[x]
            rb, kb, c, d = edges[y]
            if ra == rb:
                n = len(rings[ra])
                if (ka + 1) % n == kb or (kb + 1) % n == ka:
                    if ((ka + 1) % n == kb and folds(a, b, d)) or ((kb + 1) % n == ka and folds(c, d, b)):
                        return True
                    continue
            if meet(a, b, c, d):
                return True
    return False


def close_area(r, x, y, tol, holes, eps):
    at = click_cell(r, x, y)
    if at is None:
        return {"refused": "rasterin dışında"}
    got = colour(r, *at)
    if got is None:
        return {"refused": "değeri yok"}
    target, rgb = got
    cells = flood(r, at, target, tol, rgb, False)
    if any(i in (0, r.width - 1) or j in (0, r.height - 1) for i, j in cells):
        return {"refused": "Alan kapanmıyor"}
    rings = region_rings(cells)
    if holes == "fill":
        rings = rings[:1]
    tag = 0
    if eps > 0:
        simple = []
        for ring in rings:
            pts = [(float(u), float(v)) for u, v in ring] + [(float(ring[0][0]), float(ring[0][1]))]
            kept = dp_closed(pts, eps)[:-1]
            simple.append(ring if len(kept) < 3 else [(int(u), int(v)) for u, v in kept])
        if any(twice_area(s) == 0 for s in simple) or crossing(simple):
            tag = 1
        else:
            rings = simple
    f = features("polygons")
    push(f, [to_world(x_, r.affine) for x_ in rings], r.affine, math.nan, "", tag)
    return {"features": f}


# ── Eğrilere kot ver (§9) ─────────────────────────────────────────────────

def curve_segments(shape):
    k = shape["kind"]
    if k == "line":
        return [(xy(shape["a"]), xy(shape["b"]))]
    rings = []
    if k == "polyline":
        rings.append(([xy(p) for p in shape["pts"]], False))
    else:
        rings.append(([xy(p) for p in shape["pts"]], True))
        rings += [([xy(p) for p in h["pts"]], True) for h in shape.get("holes") or []]
    out = []
    for pts, closed in rings:
        n = len(pts)
        out += [(pts[i], pts[(i + 1) % n]) for i in range(n if closed else n - 1)]
    return out


def crossing_t(s0, s1, a, b):
    """The cut's parameter where it meets segment a–b, exactly; none when they do not meet."""
    p, r = (Fraction(s0[0]), Fraction(s0[1])), (Fraction(s1[0]) - Fraction(s0[0]), Fraction(s1[1]) - Fraction(s0[1]))
    q, s = (Fraction(a[0]), Fraction(a[1])), (Fraction(b[0]) - Fraction(a[0]), Fraction(b[1]) - Fraction(a[1]))
    den = r[0] * s[1] - r[1] * s[0]
    if den == 0:
        return None
    qp = (q[0] - p[0], q[1] - p[1])
    t = (qp[0] * s[1] - qp[1] * s[0]) / den
    u = (qp[0] * r[1] - qp[1] * r[0]) / den
    return t if 0 <= t <= 1 and 0 <= u <= 1 else None


def contour_elevations(curves, start, end, first, step):
    places = []
    for c in curves:
        ts = [t for a, b in curve_segments(c) if (t := crossing_t(start, end, a, b)) is not None]
        places.append(min(ts) if ts else None)
    order = sorted((k for k in range(len(curves)) if places[k] is not None), key=lambda k: (places[k], k))
    out = [None] * len(curves)
    for n, k in enumerate(order):
        out[k] = first + n * step
    return out


# ── The cases ─────────────────────────────────────────────────────────────

def poly(pts, holes=None, bulges=None, kind="polygon"):
    s = {"kind": kind, "pts": [{"x": x, "y": y} for x, y in pts]}
    if bulges:
        s["bulges"] = bulges
    if holes:
        s["holes"] = [{"pts": [{"x": x, "y": y} for x, y in h]} for h in holes]
    return s


def line(a, b):
    return {"kind": "line", "a": {"x": a[0], "y": a[1]}, "b": {"x": b[0], "y": b[1]}}


def point(p, parts=()):
    s = {"kind": "point", "p": {"x": p[0], "y": p[1]}}
    if parts:
        s["parts"] = [{"p": {"x": x, "y": y}} for x, y in parts]
    return s


def gray(width, height, rows, affine=PLACE, sample="f32", nodata="nan"):
    vals = [math.nan if v is None else float(v) for row in rows for v in row]
    return ro.Raster(affine, width, height, vals, sample=sample, nodata=nodata)


def sheet(width, height, dark, ink=(30.0, 30.0, 30.0), paper=(255.0, 255.0, 255.0), affine=PLACE):
    vals = []
    for j in range(height):
        for i in range(width):
            vals += list(ink if dark(i, j) else paper)
    return ro.Raster(affine, width, height, vals, bands=3, sample="u8", nodata=None)


def cases():
    out = []

    def case(name, tool, expect, raster=None, burn=None):
        c = {"name": name, "tool": tool}
        if raster is not None:
            c["inputs"] = [raster.json()]
        if burn is not None:
            c.update(burn)
        c["expect"] = expect
        out.append(c)

    # Rasterleştir.
    x0, y0 = 500.0, 300.0
    parcels = [
        poly([(x0 + 0.3, y0 + 0.4), (x0 + 6.7, y0 + 0.9), (x0 + 5.8, y0 + 7.1), (x0 + 0.6, y0 + 5.2)]),
        poly([(x0 + 4.1, y0 + 3.3), (x0 + 11.6, y0 + 2.7), (x0 + 10.9, y0 + 9.4), (x0 + 3.7, y0 + 8.6)],
             holes=[[(x0 + 6.2, y0 + 5.1), (x0 + 8.9, y0 + 5.3), (x0 + 8.3, y0 + 7.4), (x0 + 6.6, y0 + 7.0)]]),
        {"kind": "circle", "c": {"x": x0 + 15.3, "y": y0 + 6.2}, "r": 3.7},
    ]
    roads = [
        line((x0 + 0.37, y0 + 10.83), (x0 + 18.41, y0 + 1.29)),
        {"kind": "polyline", "pts": [{"x": x0 + 1.13, "y": y0 + 1.71}, {"x": x0 + 9.27, "y": y0 + 10.53},
                                     {"x": x0 + 17.71, "y": y0 + 10.19}], "bulges": [0.0, 0.31]},
        {"kind": "arc", "c": {"x": x0 + 13.1, "y": y0 + 4.4}, "r": 2.9, "a0": 0.37, "a1": 2.81},
    ]
    wells = [point((x0 + 2.21, y0 + 9.13), [(x0 + 16.83, y0 + 1.37), (x0 + 16.92, y0 + 1.11)])]
    shapes = parcels + roads + wells
    texts = ["3", "7,5", "-2.25", "11", "4", "yok", "9"]
    for overlap in ("last", "first", "max", "min", "sum", "count"):
        tool = {"kind": "rasterize", "value": 1.0, "overlap": overlap, "sample": "f32"}
        burn = {"shapes": shapes, "values": texts, "cell": 0.5}
        case(f"rasterlestir-{overlap}", tool, rasterize(shapes, texts, tool, 0.5, None), burn=burn)
    tool = {"kind": "rasterize", "value": 1.0, "overlap": "last", "sample": "u8"}
    case("rasterlestir-sabit-bayt", tool, rasterize(shapes, None, tool, 1.0, None),
         burn={"shapes": shapes, "values": None, "cell": 1.0})
    tool = {"kind": "rasterize", "value": 1.0, "overlap": "sum", "sample": "i32"}
    case("rasterlestir-tam-sayi", tool, rasterize(shapes, texts, tool, 0.5, None),
         burn={"shapes": shapes, "values": texts, "cell": 0.5})
    grid = {"affine": [x0 + 2.0, 1.0, 0.0, y0 + 12.0, 0.0, -1.0], "width": 9, "height": 7}
    tool = {"kind": "rasterize", "value": 1.0, "overlap": "max", "sample": "f64"}
    case("rasterlestir-raster-izgarasi", tool, rasterize(shapes, texts, tool, 0.0, grid),
         burn={"shapes": shapes, "values": texts, "cell": 0.0, "grid": grid})
    # A turned grid: the cells' half-open squares in their own space.
    turned = {"affine": ro.turned(25, (x0 - 4.0, y0 + 6.0), 0.9), "width": 26, "height": 20}
    tool = {"kind": "rasterize", "value": 1.0, "overlap": "count", "sample": "f32"}
    case("rasterlestir-donuk-izgara", tool, rasterize(shapes, texts, tool, 0.0, turned),
         burn={"shapes": shapes, "values": texts, "cell": 0.0, "grid": turned})
    tool = {"kind": "rasterize", "value": 300.0, "overlap": "last", "sample": "u8"}
    case("rasterlestir-sigmiyor", tool, {"refused": "sonucun türüne sığmıyor"},
         burn={"shapes": roads, "values": None, "cell": 1.0})
    tool = {"kind": "rasterize", "value": 1.0, "overlap": "last", "sample": "f32"}
    case("rasterlestir-deger-yok", tool, {"refused": "Değeri okunan nesne yok"},
         burn={"shapes": roads, "values": ["a", "b", ""], "cell": 1.0})

    # Rasterden alan.
    n = None
    classes = gray(9, 7, [
        [1, 1, 2, 2, 2, 3, 3, 3, 3],
        [1, 4, 4, 2, 2, 3, n, n, 3],
        [1, 4, 4, 2, 1, 3, n, 5, 3],
        [1, 1, 1, 1, 1, 3, 3, 3, 3],
        [2, 2, 1, 6, 6, 6, 1, 1, 1],
        [2, 1, 2, 6, 7, 6, 1, 8, 1],
        [2, 2, 2, 6, 6, 6, 1, 1, 1],
    ])
    for connect in ("four", "eight"):
        tool = {"kind": "toPolygons", "band": 1, "connect": connect}
        case(f"alan-{connect}", tool, to_polygons(classes, 0, connect == "eight"), raster=classes)
    # A pocket touching the outline at a corner: a hole that meets the outline (4), a diagonal joined (8).
    pocket = gray(6, 6, [
        [9, 9, 9, 9, 9, 9],
        [9, 0, 0, 9, 9, 9],
        [9, 0, 0, 9, 9, 9],
        [9, 9, 9, 0, 9, 9],
        [9, 9, 9, 9, 0, 0],
        [9, 9, 9, 9, 0, 0],
    ])
    for connect in ("four", "eight"):
        tool = {"kind": "toPolygons", "band": 1, "connect": connect}
        case(f"alan-cep-{connect}", tool, to_polygons(pocket, 0, connect == "eight"), raster=pocket)
    mirrored = gray(4, 3, [[1, 1, 2, 2], [1, 3, 3, 2], [1, 1, 2, 2]], affine=[700.0, 1.5, 0.0, 400.0, 0.0, 1.5])
    tool = {"kind": "toPolygons", "band": 1, "connect": "four"}
    case("alan-aynali-afin", tool, to_polygons(mirrored, 0, False), raster=mirrored)
    ints = gray(5, 4, [[-7, -7, 300, 300, 300], [-7, 12, 12, 300, -7], [-7, -7, 12, -7, -7], [0, 0, 0, 0, 0]], sample="i16", nodata=0.0)
    case("alan-tam-sayi-nodata", tool, to_polygons(ints, 0, False), raster=ints)

    # Rasterden çizgi.
    lines_r = gray(16, 12, [[0] * 16 for _ in range(12)])
    def ink(cells, value=1.0):
        for i, j in cells:
            lines_r.values[j * 16 + i] = value
    ink([(i, j) for i in range(1, 15) for j in (2, 3, 4)])               # a thick bar
    ink([(5 + k, 5 + k) for k in range(6)] + [(6 + k, 5 + k) for k in range(5)])  # a diagonal
    ink([(12, j) for j in range(5, 11)] + [(13, j) for j in range(7, 9)])  # a stem with a nub
    ink([(2, 8), (3, 8), (2, 9), (3, 9), (2, 10), (3, 10)], 7.0)         # a blot of another value
    for name, select, spur, eps in [
        ("cizgi-sifir-disi", ("nonZero",), 0, 0.0),
        ("cizgi-kisa-parcalar", ("nonZero",), 3, 1.0),
        ("cizgi-aralik", ("range", 6.5, 7.5), 0, 0.0),
    ]:
        tool = {"kind": "toLines", "band": 1, "select": select[0], "spur": spur, "simplify": eps}
        if select[0] == "range":
            tool.update(min=select[1], max=select[2])
        case(name, tool, to_lines(lines_r, 0, select, spur, eps), raster=lines_r)
    loop = gray(9, 9, [[1 if (abs(i - 4) + abs(j - 4) in (3, 4)) else 0 for i in range(9)] for j in range(9)])
    tool = {"kind": "toLines", "band": 1, "select": "nonZero", "spur": 0, "simplify": 0.6}
    case("cizgi-halka", tool, to_lines(loop, 0, ("nonZero",), 0, 0.6), raster=loop)
    map_rgb = sheet(14, 10, lambda i, j: j == 4 or (i == 9 and j > 4), ink=(200.0, 40.0, 30.0))
    tool = {"kind": "toLines", "band": 1, "select": "color", "color": "#C82A1C", "tolerance": 40.0, "spur": 0, "simplify": 1.0}
    case("cizgi-renk", tool, to_lines(map_rgb, 0, ("color", [200.0, 42.0, 28.0], 40.0), 0, 1.0), raster=map_rgb)

    # Rasterden nokta.
    dem = gray(12, 9, [[round(math.sin(i * 0.7) * 10 + math.cos(j * 0.9) * 7 + i * 0.5, 3) for i in range(12)] for j in range(9)])
    dem.values[40] = math.nan
    for mode, k in [("step", 3), ("all", 0), ("extrema", 1), ("extrema", 2)]:
        tool = {"kind": "toPoints", "band": 1, "mode": mode}
        tool.update({"step": k} if mode == "step" else {"radius": k} if mode == "extrema" else {})
        case(f"nokta-{mode}-{k}", tool, to_points(dem, 0, mode, k), raster=dem)

    # Çizgi yakala.
    scan = sheet(40, 24, lambda i, j: (abs(j - (5 + i * 0.3)) < 1.2) or (abs(i - 30) < 1 and 8 <= j <= 20) or (i, j) in ((10, 18), (11, 18)))
    # The click beside the line: the darkest of the 7 × 7 is on it.
    for name, at, spur in [("yakala-egik", (12, 7), 5), ("yakala-dal", (30, 15), 2), ("yakala-kisa-parca", (10, 20), 0)]:
        x, y = ro.centre(scan.affine, *at)
        tool = {"kind": "captureLine", "x": x, "y": y, "tolerance": 60.0, "spur": spur, "simplify": 1.0}
        case(name, tool, capture_line(scan, x, y, 60.0, spur, 1.0), raster=scan)
    x, y = ro.centre(scan.affine, 3, 22)
    tool = {"kind": "captureLine", "x": x, "y": y, "tolerance": 10.0, "spur": 5, "simplify": 1.0}
    case("yakala-bosluk", tool, capture_line(scan, x, y, 10.0, 5, 1.0), raster=scan)
    tool = {"kind": "captureLine", "x": 1.0, "y": 1.0, "tolerance": 60.0, "spur": 5, "simplify": 1.0}
    case("yakala-disarida", tool, {"refused": "rasterin dışında"}, raster=scan)

    # Alan kapat: a frame, a wall, a stair across the left room, a dot in the right room and one touching the stair.
    def parcel_map(i, j):
        frame = (2 <= i <= 21 and 2 <= j <= 13) and (i in (2, 21) or j in (2, 13))
        wall = i == 11 and 2 <= j <= 13
        stair = i + j == 15 and 3 <= i <= 10
        dots = (i, j) in ((16, 7), (8, 9))
        return frame or wall or stair or dots
    pm = sheet(24, 16, parcel_map)
    for name, at, holes, eps in [("kapat-merdiven", (9, 11), "fill", 1.0), ("kapat-degen-delik", (9, 11), "keep", 1.0),
                                 ("kapat-ucgen", (4, 4), "fill", 1.5), ("kapat-delikli", (14, 10), "keep", 1.0),
                                 ("kapat-sadelesmeden", (14, 10), "keep", 0.0)]:
        x, y = ro.centre(pm.affine, *at)
        tool = {"kind": "closeArea", "x": x, "y": y, "tolerance": 60.0, "holes": holes, "simplify": eps}
        case(name, tool, close_area(pm, x, y, 60.0, holes, eps), raster=pm)
    # A spur of the wall beside a dot: the simplified outline meets the dot, the rings are written as they are (tag 1).
    pm2 = sheet(24, 16, lambda i, j: parcel_map(i, j) and (i, j) != (8, 9) or (i, j) in ((9, 9), (10, 11), (10, 12)))
    x, y = ro.centre(pm2.affine, 7, 12)
    tool = {"kind": "closeArea", "x": x, "y": y, "tolerance": 60.0, "holes": "keep", "simplify": 1.5}
    case("kapat-kesisen", tool, close_area(pm2, x, y, 60.0, "keep", 1.5), raster=pm2)
    x, y = ro.centre(pm.affine, 0, 0)
    tool = {"kind": "closeArea", "x": x, "y": y, "tolerance": 60.0, "holes": "fill", "simplify": 1.0}
    case("kapat-kenara-ulasir", tool, close_area(pm, x, y, 60.0, "fill", 1.0), raster=pm)

    # Eğrilere kot ver.
    elevations = []
    curves = [
        {"kind": "polyline", "pts": [{"x": 0.0, "y": 1.1}, {"x": 4.3, "y": 2.7}, {"x": 9.8, "y": 1.9}]},
        line((0.5, 4.2), (9.1, 5.3)),
        {"kind": "polyline", "pts": [{"x": 0.2, "y": 7.7}, {"x": 5.1, "y": 6.4}, {"x": 9.9, "y": 8.1}]},
        poly([(2.0, 9.0), (8.0, 9.0), (8.0, 11.5), (2.0, 11.5)], holes=[[(4.0, 9.8), (6.0, 9.8), (6.0, 10.6), (4.0, 10.6)]]),
        line((20.0, 0.0), (21.0, 10.0)),
        line((0.0, 4.7), (9.0, 4.7)),
    ]
    for name, start, end, first, step in [("kot-yukari", (3.3, 0.2), (5.9, 12.1), 100.0, 2.5),
                                          ("kot-asagi", (5.9, 12.1), (3.3, 0.2), 120.0, -5.0)]:
        elevations.append({"name": name, "curves": curves, "start": {"x": start[0], "y": start[1]},
                           "end": {"x": end[0], "y": end[1]}, "first": first, "step": step,
                           "expect": contour_elevations(curves, start, end, first, step)})
    return out, elevations


# ── Cross-checks against GDAL ─────────────────────────────────────────────

def mem_raster(r, band=0):
    ds = gdal.GetDriverByName("MEM").Create("", r.width, r.height, 1, gdal.GDT_Float64)
    ds.SetGeoTransform(list(r.affine))
    arr = np.array([r.get(i, j, band) for j in range(r.height) for i in range(r.width)], dtype=np.float64).reshape(r.height, r.width)
    b = ds.GetRasterBand(1)
    b.SetNoDataValue(-99999.0)
    b.WriteArray(np.where(np.isnan(arr), -99999.0, arr))
    return ds


def check_polygonize(all_cases):
    """GDAL's Polygonize: the same regions (value, area, rings) as multisets."""
    checked = 0
    for c in all_cases:
        if c["tool"]["kind"] != "toPolygons":
            continue
        j = c["inputs"][0]
        r = ro.Raster(j["affine"], j["width"], j["height"], [math.nan if v is None else v for v in j["values"]],
                      sample=j["sample"], nodata=j["nodata"])
        ds = mem_raster(r)
        drv = ogr.GetDriverByName("MEM")
        src = drv.CreateDataSource("p")
        layer = src.CreateLayer("p")
        layer.CreateField(ogr.FieldDefn("v", ogr.OFTReal))
        opts = ["8CONNECTED=8"] if c["tool"]["connect"] == "eight" else []
        gdal.FPolygonize(ds.GetRasterBand(1), ds.GetRasterBand(1).GetMaskBand(), layer, 0, opts)
        theirs = sorted((round(f.GetField("v"), 9), round(f.GetGeometryRef().GetArea(), 6), f.GetGeometryRef().GetGeometryCount())
                        for f in layer)
        f = c["expect"]["features"]
        mine, at, ring_at = [], 0, 0
        for k, v in enumerate(f["values"]):
            area, count = 0.0, f["rings"][k]
            for rr in range(count):
                size = f["sizes"][ring_at + rr]
                pts = [(f["xy"][2 * (at + q)], f["xy"][2 * (at + q) + 1]) for q in range(size)]
                a2 = sum(pts[q][0] * pts[(q + 1) % size][1] - pts[(q + 1) % size][0] * pts[q][1] for q in range(size))
                area += a2 / 2
                at += size
            ring_at += count
            mine.append((round(v, 9), round(abs(area), 6), count))
        # GDAL's 8-connected regions joined only at a corner keep one outline touching itself, as here.
        if sorted(mine) != theirs:
            raise SystemExit(f"{c['name']}: GDAL Polygonize {theirs} ≠ {sorted(mine)}")
        checked += 1
    return checked


def check_rasterize(all_cases):
    """GDAL's Rasterize: an area's cells (centre rule), a line's (ALL_TOUCHED) on the cases' grids."""
    checked = 0
    for c in all_cases:
        if c["tool"]["kind"] != "rasterize" or "raster" not in c["expect"] or c["tool"]["overlap"] != "count":
            continue
        exp = c["expect"]["raster"]
        w, h, affine = exp["width"], exp["height"], exp["affine"]
        for shape in c["shapes"]:
            if shape["kind"] not in ("polygon", "line"):
                continue
            ds = gdal.GetDriverByName("MEM").Create("", w, h, 1, gdal.GDT_Byte)
            ds.SetGeoTransform(affine)
            drv = ogr.GetDriverByName("MEM")
            src = drv.CreateDataSource("s")
            layer = src.CreateLayer("s")
            feat = ogr.Feature(layer.GetLayerDefn())
            if shape["kind"] == "line":
                g = ogr.Geometry(ogr.wkbLineString)
                g.AddPoint_2D(shape["a"]["x"], shape["a"]["y"])
                g.AddPoint_2D(shape["b"]["x"], shape["b"]["y"])
                opts = ["ALL_TOUCHED=TRUE"]
                mine = set()
                cp = [ro.place_in(affine, *xy(shape["a"])), ro.place_in(affine, *xy(shape["b"]))]
                mine = chord_cells(cp[0], cp[1], w, h)
            else:
                g = ogr.Geometry(ogr.wkbPolygon)
                for ring in [shape["pts"]] + [hh["pts"] for hh in shape.get("holes") or []]:
                    rg = ogr.Geometry(ogr.wkbLinearRing)
                    for p in ring + [ring[0]]:
                        rg.AddPoint_2D(p["x"], p["y"])
                    g.AddGeometry(rg)
                opts = []
                mine = ro.inside_cells(shape, affine, w, h)
            feat.SetGeometry(g)
            layer.CreateFeature(feat)
            gdal.RasterizeLayer(ds, [1], layer, burn_values=[1], options=opts)
            arr = ds.GetRasterBand(1).ReadAsArray()
            theirs = {(i, j) for j in range(h) for i in range(w) if arr[j, i]}
            if theirs != mine:
                raise SystemExit(f"{c['name']}: GDAL Rasterize {sorted(theirs ^ mine)} differ")
            checked += 1
    return checked


def main():
    all_cases, elevations = cases()
    polys = check_polygonize(all_cases)
    burns = check_rasterize(all_cases)
    doc = {
        "format": "kentos.raster-vector-cases",
        "version": 1,
        "note": "ADR 0234'ün araçları, KentOS kodu olmadan bu kurallardan: girdi (raster ya da nesneler ve değer metinleri), "
                "aracın ayarları ve vermesi gereken: raster (boy, yer, her örnek; null değersiz; notlar), nesneler (değer, "
                "metin, etiket; alanların halkaları; köşeler dünyada) ya da ret; Eğrilere kot ver'in kotları. Hepsi bit bit. "
                f"GDAL'la çapraz denetim: Polygonize {polys} durum, Rasterize {burns} nesne. "
                "Üretici scripts/fixtures/raster_vector_cases.py.",
        "cases": all_cases,
        "elevations": elevations,
    }
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT} güncel değil: python3 {sys.argv[0]} ile yeniden yazın ve farkı okuyun.")
            sys.exit(1)
        print(f"{OUT.name}: {len(all_cases)} durum, {len(elevations)} kot durumu; GDAL'la {polys} + {burns}; güncel.")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT} yazıldı: {len(all_cases)} durum, {len(elevations)} kot durumu; GDAL'la {polys} + {burns}.")


if __name__ == "__main__":
    main()
