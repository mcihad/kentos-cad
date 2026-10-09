"""Raster işlemleri (docs/adr/0233), worked out here from the ADR's rules
with no KentOS code: Raster hesaplayıcı, Yeniden sınıflandır, Maskeyle kırp,
Mozaik, Yeniden örnekle, Bölgesel istatistik, Histogram, Komşuluk istatistiği
and Hücre istatistiği over small rasters.

    python3 scripts/fixtures/raster_ops_cases.py           # writes the file
    python3 scripts/fixtures/raster_ops_cases.py --check   # writes nothing; compares

Writes fixtures/raster-ops/v1/cases.json: each case's input rasters (place,
size, bands, sample type, nodata, values), the areas (the drawing's JSON),
the tool's settings and what it must give: a raster (size, place and every
sample, null where empty), the zones' figures, a histogram, or a refusal.

How the values are worked out here (independent of the engine):
- a raster's place and a cell's centre are float64 as §2 writes them (the
  rule's own arithmetic: the same formula, so the same floats);
- a calculator's expression is written here as Python over the cells'
  float64 values, by the language's rules (ADR 0100);
- whether a cell's centre is inside an area: its rows and edges in cell
  space (§6) with exact rational arithmetic on the float64 vertices (Fraction);
  an arc's crossing from its circle, exactly where the square root is one;
- sums, means, weighted means and variances are exact (Fraction), the
  square root of a variance to 60 digits (mpmath), then rounded once to
  the result's type;
- bilinear and cubic weights are exact on the float64 place of each centre;
- the order statistics, the least, the largest and the histogram's
  intervals (float64, as §11 writes the formula) are exact as they stand.

A case's "rule" says how the engine's samples meet these: "exact" to the
bit; "sum" (worked out with sums, the engine in double-double or float64):
within one unit in the last place of the result's type, and for a zone's
figure (float64) within two.

Cross-checks against GDAL (gdalwarp: its nearest, bilinear, cubic, average,
min and max resampling; its cutline with the centre rule) are made on the
cases where GDAL's rule is the ADR's, away from the rasters' edges.
"""
import json
import math
import struct
import sys
from fractions import Fraction
from pathlib import Path

import mpmath
import numpy as np
from osgeo import gdal, ogr, osr

gdal.UseExceptions()
mpmath.mp.dps = 60

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/raster-ops/v1/cases.json"

INT_RANGES = {
    "u8": (0, 255), "i8": (-128, 127), "u16": (0, 65535), "i16": (-32768, 32767),
    "u32": (0, 4294967295), "i32": (-2147483648, 2147483647),
}


# ── Numbers as the result's type holds them ─────────────────────────────

def f32(x):
    """x rounded to float32 (to nearest, ties to even), as a float."""
    if x is None or (isinstance(x, float) and math.isnan(x)):
        return math.nan
    return struct.unpack("<f", struct.pack("<f", float(x)))[0]


def round_half_away(x):
    return math.floor(x + 0.5) if x >= 0 else -math.floor(-x + 0.5)


def stored(sample, x):
    """A value as a sample of `sample` holds it (integers: halves away from zero, held in range)."""
    if sample == "f32":
        return f32(x)
    if sample == "f64":
        return float(x)
    lo, hi = INT_RANGES[sample]
    if isinstance(x, float) and math.isnan(x):
        return lo
    return float(min(hi, max(lo, round_half_away(float(x)))))


def exact_to(sample, q):
    """An exact value (Fraction) rounded once to float64, then as the type holds it."""
    return stored(sample, float(q))


def mp(q):
    """An exact rational as mpmath's number (60 digits)."""
    q = Fraction(q)
    return mpmath.mpf(q.numerator) / q.denominator


# ── Places (§2) ──────────────────────────────────────────────────────────

def place_in(affine, x, y):
    x0, a, b, y0, c, d = affine
    dx, dy = x - x0, y - y0
    det = a * d - b * c
    return (d * dx - b * dy) / det, (a * dy - c * dx) / det


def point_of(affine, u, v):
    x0, a, b, y0, c, d = affine
    return x0 + a * u + b * v, y0 + c * u + d * v


def centre(affine, i, j):
    return point_of(affine, i + 0.5, j + 0.5)


class Raster:
    """An input raster: its place, size, bands, type, nodata, alpha and samples (bands interleaved)."""

    def __init__(self, affine, width, height, values, bands=1, sample="f32", nodata="nan", alpha=False):
        self.affine, self.width, self.height = affine, width, height
        self.bands, self.sample, self.nodata, self.alpha = bands, sample, nodata, alpha
        self.values = [stored(sample, v) if not (isinstance(v, float) and math.isnan(v)) else
                       (math.nan if sample in ("f32", "f64") else v) for v in values]

    def value_bands(self):
        return self.bands - (1 if self.alpha else 0)

    def get(self, i, j, b):
        """Band b's value at cell (i, j): None outside, NaN without a value."""
        if not (0 <= i < self.width and 0 <= j < self.height):
            return None
        k = (j * self.width + i) * self.bands
        if self.alpha and self.values[k + self.bands - 1] == 0:
            return math.nan
        v = self.values[k + b]
        if math.isnan(v) or (self.nodata not in (None, "nan") and v == self.nodata):
            return math.nan
        return v

    def at_point(self, x, y, b):
        u, v = place_in(self.affine, x, y)
        return self.get(math.floor(u), math.floor(v), b)

    def json(self):
        return {
            "affine": self.affine, "width": self.width, "height": self.height, "bands": self.bands,
            "sample": self.sample, "nodata": self.nodata, "alpha": self.alpha,
            "values": [None if isinstance(v, float) and math.isnan(v) else v for v in self.values],
        }


def nodata_value(r):
    if r.nodata == "nan":
        return math.nan
    return r.nodata


def empty_of(sample, values, alpha, nodata):
    """§2: what a result without a value holds: (nodata, alpha band)."""
    if nodata is not None:
        return nodata, alpha
    if sample in ("f32", "f64"):
        return "nan", alpha
    if sample == "u8" and (alpha or values == 3):
        return None, True
    return {"u8": 255.0, "u16": 65535.0, "u32": 4294967295.0, "i8": -128.0, "i16": -32768.0,
            "i32": -2147483648.0}[sample], False


# ── Sampling (§2, §8) ────────────────────────────────────────────────────

def keys(t):
    t = abs(t)
    if t <= 1:
        return (Fraction(3, 2) * t - Fraction(5, 2)) * t * t + 1
    if t < 2:
        return ((Fraction(-1, 2) * t + Fraction(5, 2)) * t - 4) * t + 2
    return Fraction(0)


def sample(r, b, u, v, how):
    """Band b at grid place (u, v) of r: exact weights, cells without a value left out (§8)."""
    if how == "nearest":
        g = r.get(math.floor(u), math.floor(v), b)
        return math.nan if g is None else g
    su, sv = Fraction(u) - Fraction(1, 2), Fraction(v) - Fraction(1, 2)
    i, j = math.floor(su), math.floor(sv)
    fx, fy = su - i, sv - j
    if how == "bilinear":
        taps = [(i, 1 - fx), (i + 1, fx)]
        rows = [(j, 1 - fy), (j + 1, fy)]
    else:
        taps = [(i - 1 + k, keys(fx + 1 - k)) for k in range(4)]
        rows = [(j - 1 + k, keys(fy + 1 - k)) for k in range(4)]
    acc, wsum = Fraction(0), Fraction(0)
    for jj, wy in rows:
        for ii, wx in taps:
            w = wx * wy
            x = r.get(ii, jj, b)
            if w != 0 and x is not None and not math.isnan(x):
                acc += w * Fraction(x)
                wsum += w
    if abs(wsum) < Fraction(1, 10 ** 6):
        return math.nan
    return acc / wsum


# ── Areas onto cells (§6) ────────────────────────────────────────────────

def bulge_arc(a, b, bulge):
    """The arc a → b of `bulge` (KentOS's rule): centre, radius², start, end (counter-clockwise from start to end)."""
    ax, ay = a
    bx, by = b
    dx, dy = bx - ax, by - ay
    k = (1 - bulge * bulge) / (4 * bulge)
    c = ((ax + bx) / 2 - dy * k, (ay + by) / 2 + dx * k)
    s, e = (a, b) if bulge > 0 else (b, a)
    r2 = (s[0] - c[0]) ** 2 + (s[1] - c[1]) ** 2
    return c, r2, s, e


def rings_of(shape):
    """The rings (points, bulges) of an area's JSON; a circle as (centre, radius)."""
    kind = shape["kind"]
    if kind == "circle":
        return [("circle", shape)]
    out = []

    def ring(pts, bulges):
        out.append(("ring", [(p["x"], p["y"]) for p in pts], bulges or []))

    ring(shape["pts"], shape.get("bulges"))
    for h in shape.get("holes") or []:
        ring(h["pts"], h.get("bulges"))
    for part in shape.get("parts") or []:
        ring(part["pts"], part.get("bulges"))
        for h in part.get("holes") or []:
            ring(h["pts"], h.get("bulges"))
    return out


def crossings(shape, affine, vr):
    """The places (u, exact or mpmath) where the area's edges cross row v = vr in cell space, by the half-open rule."""
    def cell(p):
        u, v = place_in(affine, p[0], p[1])
        return (Fraction(u), Fraction(v))

    x0, a, b, y0, c, d = affine
    det = a * d - b * c
    sign = 1 if det > 0 else -1
    side = math.sqrt(abs(det))
    vr = Fraction(vr)
    cuts = []

    def seg(p, q):
        if p[1] == q[1]:
            return
        lo, hi = (p, q) if p[1] < q[1] else (q, p)
        if lo[1] <= vr < hi[1]:
            cuts.append(("seg", lo, hi))

    def arc_pieces(cx, cy, r2, s, e, whole):
        """The arc's pieces that only rise; each that crosses the row adds its crossing (the ends compared exactly)."""
        def ang(p):
            return mpmath.atan2(mp(p[1] - cy), mp(p[0] - cx))
        if whole:
            stops = [(-mpmath.pi / 2, "bottom"), (mpmath.pi / 2, "top"), (3 * mpmath.pi / 2, "bottom")]
        else:
            a_s = ang(s)
            sweep = ang(e) - a_s
            while sweep <= 0:
                sweep += 2 * mpmath.pi
            end_ = a_s + sweep
            stops = [(a_s, ("pt", s))]
            k = mpmath.floor((a_s + mpmath.pi / 2) / mpmath.pi) + 1
            while True:
                t = -mpmath.pi / 2 + k * mpmath.pi
                if t >= end_:
                    break
                stops.append((t, "bottom" if int(k) % 2 == 0 else "top"))
                k += 1
            stops.append((end_, ("pt", e)))

        def above(end_point):
            """The sign of the end's v − vr, exactly (a turning point's v is c.y ± r)."""
            if isinstance(end_point, tuple):
                dv = end_point[1][1] - vr
                return (dv > 0) - (dv < 0)
            if end_point == "top":
                t = vr - cy
                if t < 0:
                    return 1
                return (r2 > t * t) - (r2 < t * t)
            t = cy - vr
            if t < 0:
                return -1
            return (t * t > r2) - (t * t < r2)

        for (t0, p0), (t1, p1) in zip(stops, stops[1:]):
            if t1 - t0 < mpmath.mpf(10) ** -40:
                continue
            right = mpmath.cos((t0 + t1) / 2) > 0
            # Counter-clockwise on the right half rises, on the left half falls.
            lo, hi = (p0, p1) if right else (p1, p0)
            if above(lo) <= 0 < above(hi):
                cuts.append(("arc", (cx, cy, r2), right))

    for item in rings_of(shape):
        if item[0] == "circle":
            sh = item[1]
            cc = cell((sh["c"]["x"], sh["c"]["y"]))
            rr = Fraction(sh["r"]) / Fraction(side)
            arc_pieces(cc[0], cc[1], rr * rr, None, None, True)
            continue
        _, pts, bulges = item
        n = len(pts)
        for i in range(n):
            p, q = pts[i], pts[(i + 1) % n]
            bulge = bulges[i] if i < len(bulges) else 0.0
            if abs(bulge) <= 1e-12:
                seg(cell(p), cell(q))
            else:
                cp, cq = cell(p), cell(q)
                cxy, r2, s, e = bulge_arc(cp, cq, Fraction(sign * bulge))
                arc_pieces(cxy[0], cxy[1], r2, s, e, False)
    return cuts


def first_right(cut, vr, width):
    """The first column whose centre is on the crossing or past it (exactly for a straight edge)."""
    if cut[0] == "seg":
        _, lo, hi = cut
        # Centre (k + ½, vr) at or right of the crossing ⟺ orient(lo, hi, C) ≤ 0.
        def right(k):
            cx = Fraction(2 * k + 1, 2)
            return (hi[0] - lo[0]) * (vr - lo[1]) - (hi[1] - lo[1]) * (cx - lo[0]) <= 0
        ux = lo[0] + (hi[0] - lo[0]) * (vr - lo[1]) / (hi[1] - lo[1])
        k = math.ceil(ux - Fraction(1, 2))
        while not right(k):
            k += 1
        while right(k - 1):
            k -= 1
        return min(max(k, 0), width)
    _, (cx, cy, r2), right_side = cut
    dv = Fraction(vr) - cy
    h2 = r2 - dv * dv
    if h2 < 0:
        h2 = Fraction(0)
    # Exactly where the square root is rational.
    num, den = h2.numerator, h2.denominator
    rn, rd = math.isqrt(num), math.isqrt(den)
    if rn * rn == num and rd * rd == den:
        h = Fraction(rn, rd)
        ux = cx + h if right_side else cx - h
        k = math.ceil(ux - Fraction(1, 2))
    else:
        h = mpmath.sqrt(mpmath.mpf(num) / den)
        ux = mpmath.mpf(cx.numerator) / cx.denominator + (h if right_side else -h)
        k = int(mpmath.ceil(ux - mpmath.mpf(1) / 2))
    return min(max(k, 0), width)


def inside_cells(shape, affine, width, height):
    """The set of cells whose centres the area holds (§6)."""
    cells = set()
    for j in range(height):
        vr = Fraction(2 * j + 1, 2)
        ks = sorted(first_right(cut, vr, width) for cut in crossings(shape, affine, vr))
        for a_, b_ in zip(ks[0::2], ks[1::2]):
            for i in range(a_, b_):
                cells.add((i, j))
    return cells


# ── The tools ────────────────────────────────────────────────────────────

def result(width, height, bands, affine, values, rule="exact"):
    return {"raster": {"width": width, "height": height, "bands": bands, "affine": affine,
                       "values": [None if isinstance(v, float) and math.isnan(v) else v for v in values],
                       "rule": rule}}


def calculator(inputs, names, formula, refs, empty, sample, rule="exact"):
    """`formula(values, x, y, area)` over the cells of the first named raster's grid (§3)."""
    g = inputs[refs[0][0]]
    x0, a, b, y0, c, d = g.affine
    area = abs(a * d - b * c)
    out = []
    for j in range(g.height):
        for i in range(g.width):
            x, y = centre(g.affine, i, j)
            vals = []
            for k, band in refs:
                v = inputs[k].at_point(x, y, band)
                vals.append(math.nan if v is None else v)
            if empty == "propagate" and any(math.isnan(v) for v in vals):
                out.append(math.nan)
                continue
            r = formula([None if math.isnan(v) else v for v in vals], x, y, area)
            if r is True or r is False:
                r = 1.0 if r else 0.0
            if r is None or isinstance(r, str) or not math.isfinite(r):
                out.append(math.nan)
            else:
                out.append(stored(sample, r + 0.0))
    return result(g.width, g.height, 1, list(g.affine), out, rule)


def parse_table(text):
    rules = []
    for line in text.replace(";", "\n").split("\n"):
        words = line.split()
        if not words:
            continue
        num = lambda w: None if w == "*" else float(w.replace(",", "."))
        new = lambda w: None if w in ("boş", "bos", "null", "nodata") else float(w.replace(",", "."))
        if len(words) == 2 and words[0] in ("boş", "bos", "null", "nodata"):
            rules.append(("empty", new(words[1])))
        elif len(words) == 2:
            rules.append(("value", float(words[0].replace(",", ".")), new(words[1])))
        else:
            rules.append(("range", num(words[0]), num(words[1]), new(words[2])))
    return rules


def apply_rules(rules, bounds, v):
    empty = math.isnan(v)
    for r in rules:
        if r[0] == "empty" and empty:
            return ("hit", r[1])
        if r[0] == "value" and not empty and v == r[1]:
            return ("hit", r[2])
        if r[0] == "range" and not empty:
            lo, hi = r[1], r[2]
            above = lo is None or (v > lo if bounds == "upperClosed" else v >= lo)
            below = hi is None or (v <= hi if bounds == "upperClosed" else v < hi)
            if above and below:
                return ("hit", r[3])
    return ("miss",)


def reclassify(r, band, table, bounds, unmatched, sample):
    rules = parse_table(table)
    nodata = {"f32": math.nan, "i32": -2147483648.0, "u8": 255.0}[sample]
    out = []
    for j in range(r.height):
        for i in range(r.width):
            v = r.get(i, j, band - 1)
            hit = apply_rules(rules, bounds, v)
            if hit[0] == "hit":
                new = hit[1]
                out.append(nodata if new is None else stored(sample, new))
            elif unmatched == "keep" and not math.isnan(v):
                out.append(stored(sample, v))
            else:
                out.append(nodata)
    return result(r.width, r.height, 1, list(r.affine), out)


def clip(r, shapes, crop):
    cells = set()
    for s in shapes:
        cells |= inside_cells(s, r.affine, r.width, r.height)
    if crop:
        if not cells:
            return {"refused": "kesişmiyor"}
        i0, i1 = min(c[0] for c in cells), max(c[0] for c in cells) + 1
        j0, j1 = min(c[1] for c in cells), max(c[1] for c in cells) + 1
    else:
        i0, i1, j0, j1 = 0, r.width, 0, r.height
    nodata, alpha = empty_of(r.sample, r.value_bands(), r.alpha, None if r.nodata in (None,) else nodata_value(r) if r.nodata != "nan" else None)
    if r.sample in ("f32", "f64") and nodata is None:
        nodata = "nan"
    fill = math.nan if nodata in ("nan", None) and r.sample in ("f32", "f64") else (0.0 if nodata is None else nodata)
    vb = r.value_bands()
    bands = vb + (1 if alpha else 0)
    out = []
    for j in range(j0, j1):
        for i in range(i0, i1):
            inside = (i, j) in cells
            vals = [r.get(i, j, b) if inside else math.nan for b in range(vb)]
            any_value = any(not math.isnan(v) for v in vals)
            for v in vals:
                out.append(fill if math.isnan(v) else v)
            if alpha:
                out.append(255.0 if any_value else 0.0)
    x, y = point_of(r.affine, i0, j0)
    return result(i1 - i0, j1 - j0, bands, [x, r.affine[1], r.affine[2], y, r.affine[4], r.affine[5]], out)


def union_grid(inputs):
    """§7: the finest input's lattice grown to hold every input."""
    def cell_area(r):
        x0, a, b, y0, c, d = r.affine
        return abs(a * d - b * c)
    base = min(range(len(inputs)), key=lambda k: (cell_area(inputs[k]), k))
    g = inputs[base]
    us, vs = [], []
    for r in inputs:
        for (i, j) in [(0, 0), (r.width, 0), (0, r.height), (r.width, r.height)]:
            x, y = point_of(r.affine, float(i), float(j))
            u, v = place_in(g.affine, x, y)
            us.append(u)
            vs.append(v)
    i0, i1 = math.floor(min(us) + 1e-9), math.ceil(max(us) - 1e-9)
    j0, j1 = math.floor(min(vs) + 1e-9), math.ceil(max(vs) - 1e-9)
    x, y = point_of(g.affine, float(i0), float(j0))
    return [x, g.affine[1], g.affine[2], y, g.affine[4], g.affine[5]], i1 - i0, j1 - j0


def mosaic(inputs, overlap, sampling_how):
    affine, w, h = union_grid(inputs)
    first = inputs[0]
    vb = first.value_bands()
    any_alpha = any(r.alpha for r in inputs)
    nodata, alpha = empty_of(first.sample, vb, any_alpha, None if first.nodata in (None, "nan") else first.nodata)
    if first.sample in ("f32", "f64") and nodata is None:
        nodata = "nan"
    fill = math.nan if nodata == "nan" else (0.0 if nodata is None else nodata)
    rule = "exact" if sampling_how == "nearest" and overlap != "mean" else "sum"
    out = []
    for j in range(h):
        for i in range(w):
            x, y = centre(affine, i, j)
            got = []
            for b in range(vb):
                vals = []
                for r in inputs:
                    u, v = place_in(r.affine, x, y)
                    s = sample(r, b, u, v, sampling_how)
                    if not (isinstance(s, float) and math.isnan(s)):
                        vals.append(Fraction(s))
                if not vals:
                    got.append(math.nan)
                elif overlap == "top":
                    got.append(vals[0])
                elif overlap == "bottom":
                    got.append(vals[-1])
                elif overlap == "mean":
                    got.append(sum(vals) / len(vals))
                elif overlap == "min":
                    got.append(min(vals))
                else:
                    got.append(max(vals))
            for v in got:
                out.append(fill if isinstance(v, float) and math.isnan(v) else exact_to(first.sample, v))
            if alpha:
                out.append(255.0 if any(not (isinstance(v, float) and math.isnan(v)) for v in got) else 0.0)
    return result(w, h, vb + (1 if alpha else 0), affine, out, rule)


def resample(r, cell, method):
    x0, a, b, y0, c, d = r.affine
    la = abs(a) if c == 0 else math.sqrt(a * a + c * c)
    lb = abs(d) if b == 0 else math.sqrt(b * b + d * d)
    ru, rv = cell / la, cell / lb
    w = max(1, math.ceil(r.width / ru - 1e-9))
    h = max(1, math.ceil(r.height / rv - 1e-9))
    affine = [x0, a * ru, b * rv, y0, c * ru, d * rv]
    vb = r.value_bands()
    nodata, alpha = empty_of(r.sample, vb, r.alpha, None if r.nodata in (None, "nan") else r.nodata)
    if r.sample in ("f32", "f64") and nodata is None:
        nodata = "nan"
    fill = math.nan if nodata == "nan" else (0.0 if nodata is None else nodata)
    point = method in ("nearest", "bilinear", "cubic")
    out = []

    def overlaps(k, rr):
        lo, hi = Fraction(k * rr), Fraction((k + 1) * rr)
        res = []
        i = math.floor(lo)
        while i < hi:
            ov = min(hi, i + 1) - max(lo, i)
            if ov > Fraction(1, 10 ** 9):
                res.append((i, ov))
            i += 1
        return res

    for j in range(h):
        for i in range(w):
            got = []
            for band in range(vb):
                if point:
                    x, y = centre(affine, i, j)
                    u, v = place_in(r.affine, x, y)
                    s = sample(r, band, u, v, method)
                    got.append(s)
                    continue
                cells = []
                for (yy, wy) in overlaps(j, rv):
                    for (xx, wx) in overlaps(i, ru):
                        val = r.get(xx, yy, band)
                        if val is not None and not math.isnan(val):
                            cells.append((Fraction(val), wx * wy))
                if not cells:
                    got.append(math.nan)
                elif method == "mean":
                    got.append(sum(vv * ww for vv, ww in cells) / sum(ww for _, ww in cells))
                elif method == "min":
                    got.append(min(vv for vv, _ in cells))
                elif method == "max":
                    got.append(max(vv for vv, _ in cells))
                else:
                    weights = {}
                    for vv, ww in cells:
                        weights[vv] = weights.get(vv, 0) + ww
                    got.append(min(weights, key=lambda vv: (-weights[vv], vv)))
            for v in got:
                out.append(fill if isinstance(v, float) and math.isnan(v) else exact_to(r.sample, v))
            if alpha:
                out.append(255.0 if any(not (isinstance(v, float) and math.isnan(v)) for v in got) else 0.0)
    rule = "exact" if method in ("nearest", "min", "max", "mode") else "sum"
    return result(w, h, vb + (1 if alpha else 0), affine, out, rule)


def stat_of(values, stat):
    """§9's statistic of exact values (Fractions), exactly (the standard deviation's root to 60 digits)."""
    n = len(values)
    if stat == "count":
        return Fraction(n)
    if n == 0:
        return None
    if stat == "sum":
        return sum(values)
    if stat == "mean":
        return sum(values) / n
    if stat == "min":
        return min(values)
    if stat == "max":
        return max(values)
    if stat == "range":
        return max(values) - min(values)
    if stat == "std":
        if n < 2:
            return None
        m = sum(values) / n
        var = sum((v - m) ** 2 for v in values) / (n - 1)
        return mpmath.sqrt(mpmath.mpf(var.numerator) / var.denominator)
    s = sorted(values)
    if stat == "median":
        return s[n // 2] if n % 2 else (s[n // 2 - 1] + s[n // 2]) / 2
    counts = {}
    for v in s:
        counts[v] = counts.get(v, 0) + 1
    if stat == "majority":
        return min(counts, key=lambda v: (-counts[v], v))
    if stat == "minority":
        return min(counts, key=lambda v: (counts[v], v))
    if stat == "variety":
        return Fraction(len(counts))
    raise ValueError(stat)


def as_float(q):
    if q is None:
        return None
    if isinstance(q, Fraction):
        return float(q)
    return float(q)


def zonal(r, band, stat, shapes):
    x0, a, b, y0, c, d = r.affine
    cell_area = abs(a * d - b * c)
    zones = []
    for s in shapes:
        cells = inside_cells(s, r.affine, r.width, r.height)
        values = []
        for (i, j) in sorted(cells, key=lambda t: (t[1], t[0])):
            v = r.get(i, j, band - 1)
            if v is not None and not math.isnan(v):
                values.append(Fraction(v))
        if stat == "area":
            value = Fraction(len(values)) * Fraction(cell_area)
        else:
            value = stat_of(values, stat)
        exact = stat in ("count", "min", "max", "median", "majority", "minority", "variety", "area")
        zones.append({"n": len(values), "value": as_float(value),
                      "rule": "exact" if exact else "sum"})
    return {"zones": zones}


def histogram(r, band, bins, lo=None, hi=None):
    vals = []
    empty = 0
    for j in range(r.height):
        for i in range(r.width):
            v = r.get(i, j, band - 1)
            if math.isnan(v):
                empty += 1
            else:
                vals.append(v)
    if lo is None:
        lo, hi = (min(vals), max(vals)) if vals else (math.inf, -math.inf)
    counts = [0] * bins
    below = above = 0
    for v in vals:
        if v < lo:
            below += 1
        elif v > hi:
            above += 1
        else:
            k = 0 if hi <= lo else math.floor((v - lo) / (hi - lo) * bins)
            counts[min(max(k, 0), bins - 1)] += 1
    return {"histogram": {"lo": lo, "hi": hi, "counts": counts, "below": below, "above": above,
                          "valid": len(vals), "empty": empty}}


def window(shape, width=0, height=0, radius=0, inner=0):
    if shape == "rect":
        hx, hy = width // 2, height // 2
        return [(dx, dy) for dy in range(-hy, hy + 1) for dx in range(-hx, hx + 1)]
    if shape == "circle":
        return [(dx, dy) for dy in range(-radius, radius + 1) for dx in range(-radius, radius + 1)
                if dx * dx + dy * dy <= radius * radius]
    return [(dx, dy) for dy in range(-radius, radius + 1) for dx in range(-radius, radius + 1)
            if inner * inner < dx * dx + dy * dy <= radius * radius]


def focal(r, band, stat, ignore, offsets):
    sample = "f64" if r.sample == "f64" else "f32"
    out = []
    for j in range(r.height):
        for i in range(r.width):
            vals, missing = [], False
            for dx, dy in offsets:
                v = r.get(i + dx, j + dy, band - 1)
                if v is None:
                    continue
                if math.isnan(v):
                    missing = True
                else:
                    vals.append(Fraction(v))
            if not vals or (missing and not ignore):
                out.append(math.nan)
                continue
            q = stat_of(vals, stat)
            out.append(math.nan if q is None else stored(sample, as_float(q)))
    exact = stat in ("min", "max", "median", "majority", "minority", "variety", "count")
    return result(r.width, r.height, 1, list(r.affine), out, "exact" if exact else "sum")


def cell_statistics(inputs, band, stat, ignore):
    affine, w, h = union_grid(inputs)
    sample = "f64" if any(r.sample == "f64" for r in inputs) else "f32"
    out = []
    for j in range(h):
        for i in range(w):
            x, y = centre(affine, i, j)
            vals, missing = [], False
            for r in inputs:
                v = r.at_point(x, y, band - 1)
                if v is None or math.isnan(v):
                    missing = True
                else:
                    vals.append(Fraction(v))
            if not vals or (missing and not ignore):
                out.append(math.nan)
                continue
            q = stat_of(vals, stat)
            out.append(math.nan if q is None else stored(sample, as_float(q)))
    exact = stat in ("min", "max", "median", "majority", "minority", "variety", "count")
    return result(w, h, 1, affine, out, "exact" if exact else "sum")


# ── Inputs ───────────────────────────────────────────────────────────────

PLACE = [500000.0, 2.0, 0.0, 4420000.0, 0.0, -2.0]


def field(w, h, seed=0.0, holes=(), scale=1.0):
    vals = []
    for j in range(h):
        for i in range(w):
            k = j * w + i
            if k in holes:
                vals.append(math.nan)
            else:
                vals.append(f32(scale * (100.0 + 13.0 * math.sin(i * 0.7 + seed) + 7.0 * math.cos(j * 0.45 - seed) + 0.25 * i * j)))
    return vals


def classes(w, h, seed=0):
    return [float((i * 7 + j * 3 + seed) % 5) for j in range(h) for i in range(w)]


def square(x0, y0, x1, y1, holes=None, bulges=None):
    s = {"kind": "polygon", "pts": [{"x": x0, "y": y0}, {"x": x1, "y": y0}, {"x": x1, "y": y1}, {"x": x0, "y": y1}]}
    if holes:
        s["holes"] = holes
    if bulges:
        s["bulges"] = bulges
    return s


def turned(theta_deg, origin, size):
    t = math.radians(theta_deg)
    return [origin[0], size * math.cos(t), size * math.sin(t), origin[1], size * math.sin(t), -size * math.cos(t)]


def cases():
    out = []

    def case(name, tool, inputs, expect, shapes=(), names=None):
        out.append({
            "name": name,
            "tool": tool,
            "inputs": [dict(r.json(), name=(names[k] if names else "ABCDEFGH"[k])) for k, r in enumerate(inputs)],
            "shapes": list(shapes),
            "expect": expect,
        })

    a = Raster(PLACE, 9, 7, field(9, 7, 0.0, holes=(10, 31)))
    b = Raster(PLACE, 9, 7, field(9, 7, 1.0, holes=(5,)))
    b_off = Raster([PLACE[0] + 4.0, 2.0, 0.0, PLACE[3] - 2.0, 0.0, -2.0], 9, 7, field(9, 7, 2.0))
    b_half = Raster([PLACE[0] + 1.0, 2.0, 0.0, PLACE[3] - 1.0, 0.0, -2.0], 9, 7, field(9, 7, 3.0))
    b_turn = Raster(turned(30, (PLACE[0] + 3.0, PLACE[3] + 1.0), 1.7), 11, 9, field(11, 9, 4.0))

    def calc(name, expr, inputs, refs, formula, empty="propagate", sample="f32", rule="exact"):
        tool = {"kind": "calculator", "expression": expr, "empty": empty, "sample": sample}
        case(name, tool, inputs, calculator(inputs, None, formula, refs, empty, sample, rule))

    calc("hesap-iki-raster", "[A] * 2 + [B]", [a, b], [(0, 0), (1, 0)], lambda v, x, y, s: v[0] * 2 + v[1])
    calc("hesap-kaydirilmis", "[A] - [B]", [a, b_off], [(0, 0), (1, 0)], lambda v, x, y, s: v[0] - v[1])
    calc("hesap-yarim-hucre", "[A] * 0 + [B]", [a, b_half], [(0, 0), (1, 0)], lambda v, x, y, s: v[0] * 0 + v[1])
    calc("hesap-donuk", "[A] + [B]", [a, b_turn], [(0, 0), (1, 0)], lambda v, x, y, s: v[0] + v[1])
    calc("hesap-kosul", "durum eğer [A] > 105 ise [A] yoksa 0 son", [a], [(0, 0)],
         lambda v, x, y, s: v[0] if v[0] > 105 else 0.0)
    calc("hesap-bos", "durum eğer [A] boş ise -1 yoksa [A] son", [a], [(0, 0)],
         lambda v, x, y, s: -1.0 if v[0] is None else v[0], empty="expression")
    calc("hesap-karsilastirma", "[A] > [B]", [a, b], [(0, 0), (1, 0)], lambda v, x, y, s: v[0] > v[1])
    calc("hesap-merkez", "[A] * 0 + $y / 1000 + $x / 1000000 + $alan", [a], [(0, 0)],
         lambda v, x, y, s: v[0] * 0 + x / 1000 + y / 1000000 + s, sample="f64")
    calc("hesap-islevler", "ln([A] + 1) + kök([A]) + sin([A] / 10)", [a], [(0, 0)],
         lambda v, x, y, s: float(mpmath.log(v[0] + 1)) + math.sqrt(v[0]) + float(mpmath.sin(v[0] / 10)),
         rule="sum")
    rgb = Raster(PLACE, 6, 4, [float((i * 37 + j * 11 + k * 53) % 256) for j in range(4) for i in range(6) for k in range(3)],
                 bands=3, sample="u8", nodata=None)
    calc("hesap-bantlar", "([A@1] - [A@2]) / ([A@1] + [A@2])", [rgb], [(0, 0), (0, 1)],
         lambda v, x, y, s: None if v[0] + v[1] == 0 else (v[0] - v[1]) / (v[0] + v[1]))
    calc("hesap-64", "[A] / 3", [a], [(0, 0)], lambda v, x, y, s: v[0] / 3, sample="f64")
    case("hesap-ad-yok", {"kind": "calculator", "expression": "[X] + 1", "empty": "propagate", "sample": "f32"},
         [a], {"refused": "adında raster bandı yok"})

    # Yeniden sınıflandır.
    cl = Raster(PLACE, 8, 6, [float(v) for v in range(48)], sample="f32")
    cl.values[13] = math.nan
    for name, table, bounds, unmatched, sample in [
        ("sinif-ust-kapali", "* 5 1; 5 20 2; 30 boş; boş 99", "upperClosed", "keep", "f32"),
        ("sinif-alt-kapali", "0 10 1; 10 20 2; 20 30 3; 7 70", "lowerClosed", "empty", "u8"),
        ("sinif-tam-sayi", "* 10 -5; 10 * 1000000", "upperClosed", "keep", "i32"),
    ]:
        case(name, {"kind": "reclassify", "band": 1, "table": table, "bounds": bounds, "unmatched": unmatched, "sample": sample},
             [cl], reclassify(cl, 1, table, bounds, unmatched, sample))
    case("sinif-sigmiyor", {"kind": "reclassify", "band": 1, "table": "0 10 3000000000", "bounds": "upperClosed", "unmatched": "keep", "sample": "i32"},
         [cl], {"refused": "sığmıyor"})

    # Maskeyle kırp.
    big = Raster(PLACE, 14, 11, field(14, 11, 0.5, holes=(40,)))
    x0, y0 = PLACE[0], PLACE[3]
    masks = {
        "kirp-kare": ([square(x0 + 4.3, y0 - 15.7, x0 + 13.9, y0 - 5.1)], True),
        "kirp-sinirda": ([square(x0 + 5.0, y0 - 17.0, x0 + 13.0, y0 - 7.0)], True),
        "kirp-delikli-parcali": ([dict(square(x0 + 1.2, y0 - 20.2, x0 + 20.6, y0 - 1.4,
                                              holes=[{"pts": [{"x": x0 + 7.1, "y": y0 - 13.3}, {"x": x0 + 13.6, "y": y0 - 13.3},
                                                              {"x": x0 + 13.6, "y": y0 - 7.7}, {"x": x0 + 7.1, "y": y0 - 7.7}]}]),
                                       parts=[{"pts": [{"x": x0 + 22.3, "y": y0 - 21.1}, {"x": x0 + 27.4, "y": y0 - 21.1},
                                                       {"x": x0 + 26.2, "y": y0 - 15.8}]}])], False),
        "kirp-daire": ([{"kind": "circle", "c": {"x": x0 + 13.0, "y": y0 - 11.0}, "r": 10.0}], True),
        "kirp-yay": ([square(x0 + 3.3, y0 - 18.4, x0 + 18.9, y0 - 6.2, bulges=[0.0, 0.47, 0.0, -0.31])], True),
    }
    for name, (shapes, crop) in masks.items():
        case(name, {"kind": "clipByMask", "crop": crop}, [big], clip(big, shapes, crop), shapes)
    turned_r = Raster(turned(25, (x0 + 2.0, y0 + 3.0), 2.0), 12, 10, field(12, 10, 1.5))
    sh = [square(x0 + 5.3, y0 - 14.9, x0 + 17.1, y0 - 3.7)]
    case("kirp-donuk-raster", {"kind": "clipByMask", "crop": True}, [turned_r], clip(turned_r, sh, True), sh)
    rgb2 = Raster(PLACE, 10, 8, [float((i * 29 + j * 17 + k * 71) % 256) for j in range(8) for i in range(10) for k in range(3)],
                  bands=3, sample="u8", nodata=None)
    sh = [square(x0 + 3.1, y0 - 12.9, x0 + 15.3, y0 - 2.7)]
    case("kirp-rgb-alfa", {"kind": "clipByMask", "crop": True}, [rgb2], clip(rgb2, sh, True), sh)
    sh = [square(x0 + 500.0, y0 + 500.0, x0 + 510.0, y0 + 510.0)]
    case("kirp-disarida", {"kind": "clipByMask", "crop": True}, [big], {"refused": "kesişmiyor"}, sh)

    # Mozaik.
    m1 = Raster(PLACE, 7, 6, field(7, 6, 0.0, holes=(8,)))
    m2 = Raster([x0 + 6.0, 2.0, 0.0, y0 - 4.0, 0.0, -2.0], 7, 6, field(7, 6, 1.0))
    m3 = Raster([x0 - 3.0, 3.0, 0.0, y0 + 3.0, 0.0, -3.0], 5, 4, field(5, 4, 2.0))
    for overlap in ("top", "bottom", "mean", "min", "max"):
        case(f"mozaik-{overlap}", {"kind": "mosaic", "overlap": overlap, "sampling": "nearest"},
             [m1, m2, m3], mosaic([m1, m2, m3], overlap, "nearest"))
    for how in ("bilinear", "cubic"):
        case(f"mozaik-{how}", {"kind": "mosaic", "overlap": "top", "sampling": how},
             [m1, m3], mosaic([m1, m3], "top", how))
    case("mozaik-turler", {"kind": "mosaic", "overlap": "top", "sampling": "nearest"},
         [m1, rgb], {"refused": "aynı türde"})

    # Yeniden örnekle.
    rs = Raster(PLACE, 10, 9, field(10, 9, 0.7, holes=(23, 24)))
    for cell, method in [(4.0, "nearest"), (3.0, "mean"), (4.0, "mean"), (3.0, "mode"), (5.0, "min"), (5.0, "max"),
                         (1.0, "bilinear"), (1.3, "cubic"), (2.5, "bilinear")]:
        src = Raster(PLACE, 10, 9, classes(10, 9)) if method == "mode" else rs
        case(f"ornekle-{method}-{cell}", {"kind": "resample", "cell": cell, "method": method},
             [src], resample(src, cell, method))
    case("ornekle-rgb", {"kind": "resample", "cell": 3.0, "method": "mean"}, [rgb2], resample(rgb2, 3.0, "mean"))

    # Bölgesel istatistik.
    zr = Raster(PLACE, 12, 10, field(12, 10, 0.3, holes=(30, 31, 55)))
    zones = [
        square(x0 + 1.1, y0 - 13.2, x0 + 11.7, y0 - 2.4),
        square(x0 + 8.4, y0 - 19.6, x0 + 22.9, y0 - 9.1),
        {"kind": "circle", "c": {"x": x0 + 13.0, "y": y0 - 9.0}, "r": 6.0},
        square(x0 + 900.0, y0 + 900.0, x0 + 910.0, y0 + 910.0),
    ]
    for stat in ("count", "sum", "mean", "min", "max", "range", "std", "median", "majority", "minority", "variety", "area"):
        src = Raster(PLACE, 12, 10, classes(12, 10, 1)) if stat in ("majority", "minority", "variety") else zr
        case(f"bolge-{stat}", {"kind": "zonalStatistics", "band": 1, "stat": stat}, [src], zonal(src, 1, stat, zones), zones)

    # Histogram.
    case("histogram-kendi", {"kind": "histogram", "band": 1, "bins": 8}, [zr], histogram(zr, 1, 8))
    case("histogram-sinirli", {"kind": "histogram", "band": 1, "bins": 5, "min": 100.0, "max": 120.0}, [zr],
         histogram(zr, 1, 5, 100.0, 120.0))

    # Komşuluk istatistiği.
    fr = Raster(PLACE, 13, 11, field(13, 11, 0.9, holes=(20, 21, 70)))
    fc = Raster(PLACE, 13, 11, classes(13, 11, 2))
    for name, src, shape, size, stat, ignore in [
        ("komsu-ortalama-3x3", fr, "rect", (3, 3, 0, 0), "mean", True),
        ("komsu-toplam-5x3", fr, "rect", (5, 3, 0, 0), "sum", True),
        ("komsu-sapma-5x5-bos", fr, "rect", (5, 5, 0, 0), "std", False),
        ("komsu-buyuk-daire", fr, "circle", (0, 0, 2, 0), "max", True),
        ("komsu-kucuk-daire", fr, "circle", (0, 0, 3, 0), "min", True),
        ("komsu-aralik-3x3", fr, "rect", (3, 3, 0, 0), "range", False),
        ("komsu-ortanca-halka", fr, "ring", (0, 0, 3, 1), "median", True),
        ("komsu-ortalama-daire", fr, "circle", (0, 0, 2, 0), "mean", True),
        ("komsu-cogunluk", fc, "rect", (3, 3, 0, 0), "majority", True),
        ("komsu-azinlik", fc, "circle", (0, 0, 2, 0), "minority", True),
        ("komsu-cesit", fc, "ring", (0, 0, 2, 1), "variety", True),
    ]:
        w_, h_, rad, inner = size
        tool = {"kind": "focalStatistics", "band": 1, "shape": shape, "width": w_, "height": h_, "radius": rad, "inner": inner,
                "stat": stat, "ignore": ignore}
        case(name, tool, [src], focal(src, 1, stat, ignore, window(shape, w_, h_, rad, inner)))

    # Hücre istatistiği.
    c1 = Raster(PLACE, 8, 7, field(8, 7, 0.0, holes=(9,)))
    c2 = Raster(PLACE, 8, 7, field(8, 7, 1.3))
    c3 = Raster([x0 + 4.0, 2.0, 0.0, y0 - 2.0, 0.0, -2.0], 8, 7, field(8, 7, 2.6))
    for stat, ignore in [("mean", True), ("max", False), ("std", True), ("median", True), ("count", True), ("sum", True)]:
        case(f"hucre-{stat}", {"kind": "cellStatistics", "band": 1, "stat": stat, "ignore": ignore},
             [c1, c2, c3], cell_statistics([c1, c2, c3], 1, stat, ignore))
    # Raster hesaplayıcı's result takes the grid of the raster its expression names first, here the second input (the
    # processing cases run it over every raster of their drawing, the host opening only these two).
    calc("hesap-ilk-anilan", "[B] - [A]", [c1, c3], [(1, 0), (0, 0)], lambda v, x, y, s: v[0] - v[1])
    return out


# ── Cross-checks against GDAL ─────────────────────────────────────────────

def gdal_raster(r, path):
    drv = gdal.GetDriverByName("GTiff")
    types = {"f32": gdal.GDT_Float32, "f64": gdal.GDT_Float64, "u8": gdal.GDT_Byte}
    ds = drv.Create(str(path), r.width, r.height, r.bands, types[r.sample])
    x0, a, b, y0, c, d = r.affine
    ds.SetGeoTransform([x0, a, b, y0, c, d])
    arr = np.array(r.values, dtype=np.float64).reshape(r.height, r.width, r.bands)
    for k in range(r.bands):
        band = ds.GetRasterBand(k + 1)
        if r.nodata == "nan":
            band.SetNoDataValue(float("nan"))
        band.WriteArray(arr[:, :, k])
    ds.FlushCache()
    return ds


def cross_check(all_cases):
    """GDAL's warp agrees with the cases where its rule is the ADR's (interior cells)."""
    import tempfile
    checked = 0
    tmp = Path(tempfile.mkdtemp(prefix="raster-ops-"))
    for c in all_cases:
        t = c["tool"]
        if t["kind"] != "resample" or "raster" not in c["expect"]:
            continue
        if t["method"] not in ("nearest", "bilinear", "cubic", "mean", "min", "max"):
            continue
        r_json = c["inputs"][0]
        if r_json["bands"] != 1:
            continue
        r = Raster(r_json["affine"], r_json["width"], r_json["height"],
                   [math.nan if v is None else v for v in r_json["values"]], 1, r_json["sample"], r_json["nodata"])
        if any(math.isnan(v) for v in r.values):
            # GDAL renormalises nodata a little differently near holes: a field without them.
            r = Raster(r.affine, r.width, r.height, field(r.width, r.height, 0.7), 1, "f32")
        src = gdal_raster(r, tmp / "src.tif")
        exp = c["expect"]["raster"]
        mine = resample(r, t["cell"], t["method"])["raster"]
        alg = {"nearest": "near", "mean": "average"}.get(t["method"], t["method"])
        x0, a, b, y0, cc, d = mine["affine"]
        opts = gdal.WarpOptions(format="MEM", outputBounds=(x0, y0 + d * mine["height"], x0 + a * mine["width"], y0),
                                width=mine["width"], height=mine["height"], resampleAlg=alg,
                                warpOptions=["XSCALE=1", "YSCALE=1"] if t["method"] in ("bilinear", "cubic") else [],
                                dstNodata=float("nan"))
        warped = gdal.Warp("", src, options=opts)
        out = warped.GetRasterBand(1).ReadAsArray().astype(np.float64)
        margin = 2 if t["method"] in ("bilinear", "cubic") else 1
        for j in range(margin, mine["height"] - margin):
            for i in range(margin, mine["width"] - margin):
                # Only cells whose source footprint lies inside the raster.
                cx, cy = centre(mine["affine"], i, j)
                u, v = place_in(r.affine, cx, cy)
                reach = {"nearest": 0.5, "bilinear": 1.0, "cubic": 2.0}.get(t["method"], t["cell"] / 2 / abs(r.affine[1]) + 0.5)
                if not (reach <= u <= r.width - reach and reach <= v <= r.height - reach):
                    continue
                want = mine["values"][j * mine["width"] + i]
                got = out[j, i]
                if want is None or math.isnan(got):
                    continue
                if abs(got - want) > 1e-4 * max(1.0, abs(want)):
                    raise SystemExit(f"{c['name']}: GDAL ({alg}) {got} ≠ {want} at ({i}, {j})")
                checked += 1
    return checked


def main():
    all_cases = cases()
    checked = cross_check(all_cases)
    doc = {
        "format": "kentos.raster-ops-cases",
        "version": 1,
        "note": "ADR 0233'ün araçları, KentOS kodu olmadan bu kurallardan: girdi rasterleri (yer, boy, bant, tür, nodata, "
                "değerler; null değersiz), alanlar (çizimin JSON'u), aracın ayarları ve vermesi gereken: raster (boy, yer, "
                "her örnek; null değersiz), bölgelerin sayıları, histogram ya da ret. rule \"exact\": bit bit; \"sum\": "
                "toplamla bulunan değer, sonucun türünde en çok bir son basamak birimi (bölgenin float64 sayısında iki). "
                f"GDAL'ın gdalwarp'ıyla çapraz denetim: {checked} hücre. Üretici scripts/fixtures/raster_ops_cases.py.",
        "cases": all_cases,
    }
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT} güncel değil: python3 {sys.argv[0]} ile yeniden yazın ve farkı okuyun.")
            sys.exit(1)
        print(f"{OUT.name}: {len(all_cases)} durum; GDAL'la {checked} hücre; güncel.")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT} yazıldı: {len(all_cases)} durum; GDAL'la {checked} hücre.")


if __name__ == "__main__":
    main()
