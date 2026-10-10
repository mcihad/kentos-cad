#!/usr/bin/env python3
"""Uygunluk analizi (docs/adr/0237): the six tools' rules written from the ADR with Python, without KentOS code, and the cases
the raster core plays (crates/shared/raster/tests/all/suitability.rs) and the web's WASM module (apps/web/src/io/raster.wasm.test.ts).

- Bulanık üyelik: the ADR's formulas in float64 in its order; exp and pow from mpmath at 50 digits rounded once to float64
  (the engine's libm is within one unit of them: those cases' rule is "f32ulp", or "f64ulp" for a 64-bit result).
- Bulanık çakıştırma, Ağırlıklı toplam: float64 in the inputs' order; the inputs read on their common grid (the finest input's
  lattice, the cells whose centres are inside every input's box) at the nearest cell, as the ADR writes it.
- Ağırlıklı çakıştırma: the class tables (ADR 0233 §4's language and `kısıt`), the scale values and the influences as integers;
  the sum exact, rounded half away from zero.
- İkili karşılaştırma: the matrix as exact fractions, its principal eigenvector by power iteration with mpmath at 50 digits to
  10⁻⁴⁵; λ, CI and CR from it ("eig": the engine's float64 weights within 10⁻¹², λ, CI and CR within 10⁻¹⁰).
- ROC ile doğrulama: the sample cells (a point's, an area's centres by raster_ops_cases.py's exact rule), AUC from every
  (presence, background) pair by brute force as an exact fraction, the curve's rows and Youden's best row.

    python3 scripts/fixtures/suitability_cases.py          # write fixtures/suitability/v1/cases.json
    python3 scripts/fixtures/suitability_cases.py --check  # compare
"""

import json
import math
import sys
from fractions import Fraction
from pathlib import Path

import mpmath

sys.path.insert(0, str(Path(__file__).resolve().parent))
import raster_ops_cases as ro  # noqa: E402  (rasters, places, the area cells' rule)

mpmath.mp.dps = 50

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/suitability/v1/cases.json"
NAN = math.nan
PLACE = [500000.0, 10.0, 0.0, 4420000.0, 0.0, -10.0]
RANDOM_INDEX = [0.0, 0.0, 0.58, 0.90, 1.12, 1.24, 1.32, 1.41, 1.45, 1.49, 1.51, 1.48, 1.56, 1.57, 1.59]
MOST_ROWS = 200


def f32(x):
    return ro.f32(x)


def stored(sample, x):
    if isinstance(x, float) and math.isnan(x):
        return NAN
    return f32(x) if sample == "f32" else float(x)


def exp_(x):
    """exp at the float64 x, rounded once to float64."""
    return float(mpmath.exp(mpmath.mpf(x)))


def pow_(x, y):
    return float(mpmath.power(mpmath.mpf(x), mpmath.mpf(y)))


def raster(values, w, h, affine=None, sample="f32"):
    return ro.Raster(list(affine or PLACE), w, h, values, 1, sample, "nan")


def value(r, x, y):
    """The input's band-1 value at a point: its nearest cell, NaN off the raster or without a value."""
    v = r.at_point(x, y, 0)
    return NAN if v is None else v


# ── The common grid (§2) ──────────────────────────────────────────────────

def common_grid(inputs):
    det = lambda r: abs(r.affine[1] * r.affine[5] - r.affine[2] * r.affine[4])
    base = 0
    for k in range(1, len(inputs)):
        if det(inputs[k]) < det(inputs[base]):
            base = k
    g = inputs[base].affine
    u0, u1, v0, v1 = -math.inf, math.inf, -math.inf, math.inf
    for r in inputs:
        us, vs = [], []
        for i, j in ((0.0, 0.0), (float(r.width), 0.0), (0.0, float(r.height)), (float(r.width), float(r.height))):
            x, y = ro.point_of(r.affine, i, j)
            u, v = ro.place_in(g, x, y)
            us.append(u)
            vs.append(v)
        u0, u1 = max(u0, min(us)), min(u1, max(us))
        v0, v1 = max(v0, min(vs)), min(v1, max(vs))
    i0, i1 = math.ceil(u0 - 0.5 - 1e-9), math.floor(u1 - 0.5 + 1e-9)
    j0, j1 = math.ceil(v0 - 0.5 - 1e-9), math.floor(v1 - 0.5 + 1e-9)
    if not (i1 >= i0 and j1 >= j0):
        return None
    x0, y0 = ro.point_of(g, float(i0), float(j0))
    return [x0, g[1], g[2], y0, g[4], g[5]], i1 - i0 + 1, j1 - j0 + 1


def over_grid(inputs, cell, sample, rule="exact"):
    """`cell(values)` at every cell of the inputs' common grid; the result's samples."""
    grid = common_grid(inputs)
    if grid is None:
        return {"refused": "Rasterler örtüşmüyor"}
    affine, w, h = grid
    out = []
    for j in range(h):
        for i in range(w):
            x, y = ro.centre(affine, i, j)
            out.append(stored(sample, cell([value(r, x, y) for r in inputs])))
    return ro.result(w, h, 1, affine, out, rule)


# ── Bulanık üyelik (§3) ───────────────────────────────────────────────────

def linear(a, b, x):
    if a < b:
        if x <= a:
            return 0.0
        if x >= b:
            return 1.0
        return (x - a) / (b - a)
    if x <= b:
        return 1.0
    if x >= a:
        return 0.0
    return (a - x) / (a - b)


def membership(t, x):
    fn = t["function"]
    if fn == "linear":
        return linear(t["low"], t["high"], x)
    if fn == "power":
        return pow_(linear(t["low"], t["high"], x), t["exponent"])
    m = t["midpoint"]
    if fn == "gaussian":
        d = x - m
        return exp_(-(t["spread"] * (d * d)))
    if fn == "large":
        return 0.0 if x <= 0 else 1.0 / (1.0 + pow_(x / m, -t["steep"]))
    if fn == "small":
        return 1.0 if x <= 0 else 1.0 / (1.0 + pow_(x / m, t["steep"]))
    if fn == "near":
        d = x - m
        return 1.0 / (1.0 + t["spread"] * (d * d))
    raise ValueError(fn)


def fuzzy_membership(r, t):
    if t["function"] in ("linear", "power") and t["low"] == t["high"]:
        return {"refused": "Alt ve üst değer aynı olamaz"}
    if t["function"] in ("large", "small") and not t["midpoint"] > 0:
        return {"refused": "orta nokta 0'dan büyük olmalı"}
    out = []
    for j in range(r.height):
        for i in range(r.width):
            x = r.get(i, j, 0)
            out.append(NAN if math.isnan(x) else stored(t["sample"], membership(t, x)))
    exact = t["function"] in ("linear", "near")
    rule = "exact" if exact else ("f32ulp" if t["sample"] == "f32" else "f64ulp")
    return ro.result(r.width, r.height, 1, list(r.affine), out, rule)


# ── Bulanık çakıştırma (§4) ───────────────────────────────────────────────

def fuzzy_overlay(inputs, t):
    op, gamma = t["op"], t.get("gamma", 0.0)
    if len(inputs) < 2:
        return {"refused": "en az iki raster"}
    if op == "gamma" and not 0.0 <= gamma <= 1.0:
        return {"refused": "Gamma 0 ile 1 arasında"}
    invalid = 0

    def cell(vals):
        nonlocal invalid
        if any(not math.isnan(v) and not 0.0 <= v <= 1.0 for v in vals):
            invalid += 1
            return NAN
        if any(math.isnan(v) for v in vals):
            return NAN
        if op == "and":
            return min(vals)
        if op == "or":
            return max(vals)
        p = 1.0
        for v in vals:
            p = p * v
        q = 1.0
        for v in vals:
            q = q * (1.0 - v)
        if op == "product":
            return p
        if op == "sum":
            return 1.0 - q
        return pow_(1.0 - q, gamma) * pow_(p, 1.0 - gamma)

    out = over_grid(inputs, cell, t["sample"], "f32ulp" if op == "gamma" else "exact")
    if "refused" not in out:
        out["notes"] = {"invalid": invalid}
    return out


# ── Ağırlıklı toplam (§5) ─────────────────────────────────────────────────

def weighted_sum(inputs, names, t, weights):
    w = []
    for n in names:
        x = weights.get(n)
        if x is None:
            w.append(1.0)
        elif not (math.isfinite(x) and abs(x) <= 1e9):
            return {"refused": "ağırlık −10⁹ ile 10⁹ arasında"}
        else:
            w.append(x)

    def cell(vals):
        if any(math.isnan(v) for v in vals):
            return NAN
        s = 0.0
        for wk, x in zip(w, vals):
            s = s + wk * x
        return s

    return over_grid(inputs, cell, t["sample"], t.get("rule", "exact"))


# ── Ağırlıklı çakıştırma (§6) ─────────────────────────────────────────────

EMPTY_WORDS = ("boş", "Boş", "BOŞ", "bos", "null", "NULL", "nodata")
RESTRICTED = "kısıt"


def number(word):
    return float(word.replace(",", "."))


def parse_classes(text):
    """ADR 0233 §4's table, a new value also `kısıt`: [(kind, …, new)], new a float, None (boş) or RESTRICTED."""
    rules = []
    new_of = lambda w: None if w in EMPTY_WORDS else (RESTRICTED if w in ("kısıt", "kisit") else number(w))
    for line in text.replace(";", "\n").split("\n"):
        words = line.split()
        if not words:
            continue
        if len(words) == 2 and words[0] in EMPTY_WORDS:
            rules.append(("empty", new_of(words[1])))
        elif len(words) == 2:
            rules.append(("value", number(words[0]), new_of(words[1])))
        elif len(words) == 3:
            end = lambda w: None if w == "*" else number(w)
            rules.append(("range", end(words[0]), end(words[1]), new_of(words[2])))
        else:
            raise ValueError(line)
    return rules


def apply_classes(rules, bounds, v):
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


def weighted_overlay(inputs, names, t):
    lo, hi = t["low"], t["high"]
    whole = lambda v: math.isfinite(v) and v == math.floor(v) and abs(v) <= 1e6
    if not (whole(lo) and whole(hi)):
        return {"refused": "tam sayı olmalı"}
    if lo >= hi:
        return {"refused": "alt ucu üst ucundan küçük"}
    lo, hi = int(lo), int(hi)
    if len(inputs) < 2:
        return {"refused": "en az iki raster"}
    influence, classes = t["influence"], t.get("classes", {})
    units = []
    for n in names:
        w = influence.get(n)
        if w is None:
            return {"refused": f"{n}: etkisi yazılmadı"}
        if not (0.0 <= w <= 100.0):
            return {"refused": "etki 0 ile 100 arasında"}
        u = w * 1e4
        if abs(u - round(u)) > 1e-6:
            return {"refused": "en çok dört ondalıklı"}
        units.append(int(round(u)))
    total = sum(units)
    if total != 1_000_000:
        shown = f"{total // 10000}" if total % 10000 == 0 else f"{total // 10000}.{f'{total % 10000:04d}'.rstrip('0')}"
        return {"refused": f"Etkilerin toplamı 100 olmalı; şimdi {shown}."}
    tables = []
    for n in names:
        text = (classes.get(n) or "").strip()
        if not text:
            tables.append(None)
            continue
        rules = parse_classes(text)
        for k, r in enumerate(rules):
            new = r[-1]
            if isinstance(new, float) and (new != math.floor(new) or not lo <= new <= hi):
                return {"refused": f"{n}: {k + 1}. kuralda yeni değer"}
        tables.append(rules)
    bounds = t["bounds"]
    met = {"unmatched": 0, "outside": 0, "restricted": 0}

    def scaled(k, v):
        if tables[k] is not None:
            hit = apply_classes(tables[k], bounds, v)
            if hit[0] == "miss":
                return "unmatched"
            if hit[1] is None:
                return "empty"
            if hit[1] == RESTRICTED:
                return "restricted"
            return int(hit[1])
        if math.isnan(v):
            return "empty"
        if v != math.floor(v) or not lo <= v <= hi:
            return "outside"
        return int(v)

    def cell(vals):
        s, restricted = 0, False
        for k, v in enumerate(vals):
            c = scaled(k, v)
            if c in ("empty", "unmatched", "outside"):
                if c != "empty":
                    met[c] += 1
                return NAN
            if c == "restricted":
                restricted = True
            else:
                s += c * units[k]
        if restricted:
            met["restricted"] += 1
            return float(lo - 1)
        q = (s + 500_000) // 1_000_000 if s >= 0 else -((-s + 500_000) // 1_000_000)
        return float(q)

    out = over_grid(inputs, cell, "f64")
    out["notes"] = dict(met)
    return out


# ── İkili karşılaştırma (§7) ──────────────────────────────────────────────

def pairwise(names, pairs):
    n = len(names)
    if not 2 <= n <= 15:
        return {"refused": "2 ile 15 arasında ölçüt"}
    a = [[Fraction(1)] * n for _ in range(n)]
    done = set()
    for x, y, v in pairs:
        if x not in names or y not in names or x == y:
            continue
        if not (v == math.floor(v) and (v == 1 or 2 <= abs(v) <= 9)):
            return {"refused": "karşılaştırma 1, 2 … 9 ya da −2 … −9 olmalı"}
        i, j, v = names.index(x), names.index(y), int(v)
        if i > j:
            i, j, v = j, i, (1 if v == 1 else -v)
        if (i, j) in done:
            continue
        done.add((i, j))
        aij = Fraction(v) if v >= 1 else Fraction(1, -v)
        a[i][j], a[j][i] = aij, 1 / aij
    m = [[mpmath.mpf(q.numerator) / q.denominator for q in row] for row in a]
    w = [mpmath.mpf(1) / n] * n
    for _ in range(100_000):
        y = [mpmath.fsum(m[i][j] * w[j] for j in range(n)) for i in range(n)]
        s = mpmath.fsum(y)
        nxt = [v / s for v in y]
        moved = max(abs(p - q) for p, q in zip(nxt, w))
        w = nxt
        if moved < mpmath.mpf(10) ** -45:
            break
    lam = mpmath.fsum(mpmath.fsum(m[i][j] * w[j] for j in range(n)) for i in range(n))
    if n <= 2:
        ci = ri = cr = mpmath.mpf(0)
    else:
        ci = (lam - n) / (n - 1)
        ri = mpmath.mpf(RANDOM_INDEX[n - 1])
        cr = ci / ri
    return {"weights": [float(v) for v in w], "lambda": float(lam), "ci": float(ci), "ri": RANDOM_INDEX[n - 1] if n > 2 else 0.0,
            "cr": float(cr), "rule": "eig"}


# ── ROC ile doğrulama (§8) ────────────────────────────────────────────────

def sample_cells(r, shapes):
    """The cells of the objects (§8): a point's (i ≤ u < i + 1), an area's centres; each once; points off the raster."""
    cells, outside = set(), 0
    for s in shapes:
        if s["kind"] == "point":
            for p in [s["p"]] + [q["p"] for q in s.get("parts", [])]:
                u, v = ro.place_in(r.affine, p["x"], p["y"])
                i, j = math.floor(u), math.floor(v)
                if 0 <= i < r.width and 0 <= j < r.height:
                    cells.add((i, j))
                else:
                    outside += 1
        else:
            cells |= ro.inside_cells(s, r.affine, r.width, r.height)
    return cells, outside


def roc(r, shapes, first, absence, higher):
    first = min(first, len(shapes))
    if first == 0:
        return {"refused": "Varlık nesnelerini seçin"}
    if absence and first == len(shapes):
        return {"refused": "Yokluk nesnelerini seçin"}
    pres_cells, out_p = sample_cells(r, shapes[:first])
    score = (lambda v: v) if higher else (lambda v: -v)
    skipped = 0
    pres = []
    for i, j in sorted(pres_cells):
        v = r.get(i, j, 0)
        if math.isnan(v):
            skipped += 1
        else:
            pres.append(score(v))
    outside, both = out_p, 0
    if absence:
        abs_cells, out_a = sample_cells(r, shapes[first:])
        outside += out_a
        both = len(pres_cells & abs_cells)
        back = []
        for i, j in sorted(abs_cells):
            v = r.get(i, j, 0)
            if math.isnan(v):
                skipped += 1
            else:
                back.append(score(v))
    else:
        back = [score(r.get(i, j, 0)) for j in range(r.height) for i in range(r.width) if not math.isnan(r.get(i, j, 0))]
    if not pres:
        return {"refused": "düşen varlık yok"}
    if not back:
        return {"refused": "düşen yokluk yok" if absence else "değerli hücresi yok"}
    g = sum(1 for p in pres for q in back if p > q)
    e = sum(1 for p in pres for q in back if p == q)
    P, N = len(pres), len(back)
    auc = float(Fraction(2 * g + e, 2 * P * N))
    distinct = sorted(set(pres), reverse=True)
    m = len(distinct)
    kept = distinct if m <= MOST_ROWS else [distinct[-(-k * m // MOST_ROWS) - 1] for k in range(1, MOST_ROWS + 1)]
    rows, best, best_j = [], None, None
    for k, t in enumerate(kept):
        tp = sum(1 for p in pres if p >= t)
        fp = sum(1 for q in back if q >= t)
        rows.append([t if higher else -t + 0.0, tp, fp])
        jy = tp / P - fp / N
        if best_j is None or jy > best_j:
            best, best_j = k, jy
    return {"roc": {"presence": P, "background": N, "allCells": not absence, "auc": auc, "rows": rows, "best": best,
                    "skipped": skipped, "outside": outside, "both": both}}


# ── The cases ─────────────────────────────────────────────────────────────

def pt(x, y, parts=()):
    s = {"kind": "point", "p": {"x": x, "y": y}}
    if parts:
        s["parts"] = [{"p": {"x": px, "y": py}} for px, py in parts]
    return s


def centre_of(i, j, affine=None):
    x, y = ro.centre(affine or PLACE, i, j)
    return x, y


def values(w, h, f, holes=()):
    return [NAN if k in holes else f32(f(k % w, k // w)) for k in range(w * h)]


# Bulanık üyelik's raster: −4 … 35.7, two holes.
M = raster(values(6, 5, lambda i, j: -4.0 + 1.37 * (j * 6 + i), holes=(7, 19)), 6, 5)
# Memberships: mostly 0–1, one above and one below.
U1 = raster(values(6, 5, lambda i, j: ((j * 6 + i) * 37 % 101) / 100.0, holes=(4,)), 6, 5)
U2 = raster(values(6, 5, lambda i, j: ((j * 6 + i) * 53 % 97) / 96.0), 6, 5)
U2.values[9] = f32(1.25)
U2.values[22] = f32(-0.125)
U3 = raster(values(6, 5, lambda i, j: 1.0 - ((j * 6 + i) * 11 % 31) / 30.0, holes=(28,)), 6, 5)
# A coarser membership, offset half its cell: the common grid is the intersection.
U4 = raster(values(3, 3, lambda i, j: 0.125 * (1 + i + 3 * j) - 0.0625), 3, 3, [500015.0, 20.0, 0.0, 4419995.0, 0.0, -20.0])
# Criteria for the sums.
S1 = raster(values(6, 5, lambda i, j: 12.5 + 3.25 * math.sin(i * 0.9 + j * 0.4) + 0.75 * i * j, holes=(13,)), 6, 5)
S2 = raster(values(6, 5, lambda i, j: 250.0 - 17.0 * i + 9.5 * j), 6, 5)
S3 = raster(values(6, 5, lambda i, j: (i * 7 + j * 3) % 5 + 0.5, holes=(0, 29)), 6, 5)
# A turned criterion (read through the drawing).
S4 = raster(values(5, 5, lambda i, j: 2.0 * i - 1.5 * j), 5, 5, ro.turned(17.0, (499995.0, 4420012.0), 13.0))
# Ağırlıklı çakıştırma's criteria: classes on the scale (with a 0, a 10, a 4.5 and a hole), slopes, land cover codes.
C1 = raster([NAN if k == 3 else float((k * 7) % 11) for k in range(30)], 6, 5)
C1.values[17] = f32(4.5)
C2 = raster(values(6, 5, lambda i, j: 2.0 + 1.6 * (j * 6 + i)), 6, 5)
C3 = raster([float(1 + (k * 3) % 5) for k in range(30)], 6, 5)
# Halves: two criteria on −5 … 5.
H1 = raster([float(v) for v in (-1, 3, 5, -5, 0, 2)], 3, 2)
H2 = raster([float(v) for v in (-2, 4, -4, -4, 1, 3)], 3, 2)
# ROC's raster: ties (whole numbers 0–9), two holes; and a raster of many distinct values.
R = raster([NAN if k in (11, 40) else float((k * 7 + (k // 10) * 3) % 10) for k in range(80)], 10, 8)
D = raster(values(30, 30, lambda i, j: 0.37 * ((i * 13 + j * 29) % 900) + 0.001 * i), 30, 30)


def area(i0, j0, i1, j1, affine=None):
    """A square over cells i0 … i1 − 1, j0 … j1 − 1 of `affine`'s lattice (its edges halfway, the centres inside)."""
    (x0, y0), (x1, y1) = ro.point_of(affine or PLACE, i0 + 0.1, j0 + 0.1), ro.point_of(affine or PLACE, i1 - 0.1, j1 - 0.1)
    return ro.square(min(x0, x1), min(y0, y1), max(x0, x1), max(y0, y1))


def ops_case(name, tool, inputs, names, expect, shapes=()):
    return {"name": name, "job": "ops", "tool": tool, "inputs": [r.json() for r in inputs], "names": names,
            "shapes": list(shapes), "expect": expect}


def cases():
    out = []
    # Bulanık üyelik (§3).
    base = {"kind": "fuzzyMembership", "band": 1, "low": 0.0, "high": 100.0, "exponent": 2.0, "midpoint": 1.0, "spread": 0.1,
            "steep": 5.0, "sample": "f32"}
    for name, more in [
        ("uyelik-dogrusal", {"function": "linear", "low": 0.0, "high": 20.0}),
        ("uyelik-dogrusal-azalan", {"function": "linear", "low": 20.0, "high": 0.0}),
        ("uyelik-uslu", {"function": "power", "low": 0.0, "high": 20.0, "exponent": 2.5}),
        ("uyelik-gauss", {"function": "gaussian", "midpoint": 10.0, "spread": 0.05}),
        ("uyelik-buyuk", {"function": "large", "midpoint": 10.0, "steep": 5.0}),
        ("uyelik-kucuk", {"function": "small", "midpoint": 10.0, "steep": 3.0}),
        ("uyelik-yakin", {"function": "near", "midpoint": 10.0, "spread": 0.1}),
        ("uyelik-gauss-64", {"function": "gaussian", "midpoint": 7.5, "spread": 0.02, "sample": "f64"}),
        ("ret-uyelik-ayni-uclar", {"function": "linear", "low": 5.0, "high": 5.0}),
        ("ret-uyelik-orta-sifir", {"function": "large", "midpoint": 0.0}),
    ]:
        t = {**base, **more}
        out.append(ops_case(name, t, [M], ["Eğim"], fuzzy_membership(M, t)))
    out.append(ops_case("ret-uyelik-iki-raster", {**base, "function": "linear"}, [M, S1], ["Eğim", "Yol"],
                        {"refused": "tek raster"}))
    # Bulanık çakıştırma (§4).
    for op in ("and", "or", "product", "sum", "gamma"):
        t = {"kind": "fuzzyOverlay", "band": 1, "op": op, "gamma": 0.9, "sample": "f32"}
        out.append(ops_case(f"bulanik-{op}", t, [U1, U2, U3], ["A", "B", "C"], fuzzy_overlay([U1, U2, U3], t)))
    t = {"kind": "fuzzyOverlay", "band": 1, "op": "product", "gamma": 0.0, "sample": "f64"}
    out.append(ops_case("bulanik-kesisim", t, [U4, U1], ["Kaba", "İnce"], fuzzy_overlay([U4, U1], t)))
    t = {"kind": "fuzzyOverlay", "band": 1, "op": "gamma", "gamma": 1.5, "sample": "f32"}
    out.append(ops_case("ret-bulanik-gamma", t, [U1, U2], ["A", "B"], fuzzy_overlay([U1, U2], t)))
    t = {"kind": "fuzzyOverlay", "band": 1, "op": "and", "gamma": 0.0, "sample": "f32"}
    out.append(ops_case("ret-bulanik-tek", t, [U1], ["A"], {"refused": "en az iki raster"}))
    # Ağırlıklı toplam (§5).
    names = ["Eğim", "Yol", "Toprak"]
    for name, weights, sample in [
        ("agirlikli-toplam", {"Eğim": 0.5, "Yol": -0.02, "Toprak": 1.25}, "f32"),
        ("agirlikli-toplam-varsayilan", {"Yol": 0.1}, "f32"),
        ("agirlikli-toplam-64", {"Eğim": 0.3, "Yol": 0.001, "Toprak": -2.0}, "f64"),
    ]:
        t = {"kind": "weightedSum", "band": 1, "weights": weights, "sample": sample}
        out.append(ops_case(name, t, [S1, S2, S3], names, weighted_sum([S1, S2, S3], names, t, weights)))
    t = {"kind": "weightedSum", "band": 1, "weights": {"Eğim": 0.75, "Döndürülmüş": 2.0}, "sample": "f32"}
    out.append(ops_case("agirlikli-toplam-donuk", t, [S1, S4], ["Eğim", "Döndürülmüş"],
                        weighted_sum([S1, S4], ["Eğim", "Döndürülmüş"], t, t["weights"])))
    far = raster(values(2, 2, lambda i, j: 1.0), 2, 2, [600000.0, 10.0, 0.0, 4500000.0, 0.0, -10.0])
    t = {"kind": "weightedSum", "band": 1, "weights": {}, "sample": "f32"}
    out.append(ops_case("ret-toplam-ortusmez", t, [S1, far], ["Eğim", "Uzak"], weighted_sum([S1, far], ["Eğim", "Uzak"], t, {})))
    t = {"kind": "weightedSum", "band": 1, "weights": {"Eğim": 2e9}, "sample": "f32"}
    out.append(ops_case("ret-toplam-agirlik", t, [S1, S2], ["Eğim", "Yol"], weighted_sum([S1, S2], ["Eğim", "Yol"], t, t["weights"])))
    # Ağırlıklı çakıştırma (§6).
    cnames = ["Sınıf", "Eğim", "Örtü"]
    tables = {"Eğim": "* 5 9; 5 15 6; 15 30 3; 30 * kısıt", "Örtü": "1 9; 2 7; 3 5; 4 boş"}
    for name, t in [
        ("cakistirma-tablosuz", {"low": 1.0, "high": 9.0, "influence": {"Sınıf": 50.0, "Eğim": 30.0, "Örtü": 20.0}, "classes": {},
                                 "bounds": "upperClosed", "inputs": [C1, C3, C3]}),
        ("cakistirma-tablolu", {"low": 1.0, "high": 9.0, "influence": {"Sınıf": 33.3333, "Eğim": 33.3333, "Örtü": 33.3334},
                                "classes": tables, "bounds": "upperClosed", "inputs": [C1, C2, C3]}),
        ("cakistirma-alt-kapali", {"low": 1.0, "high": 9.0, "influence": {"Sınıf": 10.0, "Eğim": 60.0, "Örtü": 30.0},
                                   "classes": tables, "bounds": "lowerClosed", "inputs": [C1, C2, C3]}),
    ]:
        inputs = t.pop("inputs")
        tool = {"kind": "weightedOverlay", "band": 1, **t}
        out.append(ops_case(name, tool, inputs, cnames, weighted_overlay(inputs, cnames, tool)))
    tool = {"kind": "weightedOverlay", "band": 1, "low": -5.0, "high": 5.0, "influence": {"A": 50.0, "B": 50.0}, "classes": {},
            "bounds": "upperClosed"}
    out.append(ops_case("cakistirma-yarimlar", tool, [H1, H2], ["A", "B"], weighted_overlay([H1, H2], ["A", "B"], tool)))
    for name, more in [
        ("ret-cakistirma-toplam", {"influence": {"Sınıf": 50.0, "Eğim": 30.0, "Örtü": 9.3333}}),
        ("ret-cakistirma-eksik", {"influence": {"Sınıf": 50.0, "Eğim": 50.0}}),
        ("ret-cakistirma-ondalik", {"influence": {"Sınıf": 50.0, "Eğim": 29.99995, "Örtü": 20.00005}}),
        ("ret-cakistirma-kural", {"classes": {"Eğim": "* 5 9; 5 * 12"}}),
        ("ret-cakistirma-olcek", {"low": 9.0, "high": 1.0}),
    ]:
        tool = {"kind": "weightedOverlay", "band": 1, "low": 1.0, "high": 9.0,
                "influence": {"Sınıf": 50.0, "Eğim": 30.0, "Örtü": 20.0}, "classes": {}, "bounds": "upperClosed", **more}
        out.append(ops_case(name, tool, [C1, C2, C3], cnames, weighted_overlay([C1, C2, C3], cnames, tool)))
    # İkili karşılaştırma (§7).
    pnames = ["Eğim", "Yol", "Toprak"]
    pairs = [["Eğim", "Yol", 2.0], ["Eğim", "Toprak", 4.0], ["Toprak", "Yol", -2.0]]
    p = pairwise(pnames, pairs)
    t = {"kind": "pairwise", "band": 1, "pairs": pairs, "write": True, "sample": "f32"}
    e = weighted_sum([S1, S2, S3], pnames, {"sample": "f32", "rule": "f32ulp"}, dict(zip(pnames, p["weights"])))
    e["pairwise"] = p
    out.append(ops_case("ahp-tutarli-raster", t, [S1, S2, S3], pnames, e))
    four = ["A", "B", "C", "D"]
    for name, names_, pairs in [
        ("ahp-dort", four, [["A", "B", 3.0], ["A", "C", 5.0], ["A", "D", 7.0], ["B", "C", 3.0], ["B", "D", 5.0], ["C", "D", 3.0]]),
        ("ahp-tutarsiz", ["A", "B", "C"], [["A", "B", 9.0], ["B", "C", 9.0], ["C", "A", 9.0]]),
        ("ahp-iki", ["A", "B"], [["A", "B", -5.0]]),
        ("ahp-ters-ve-ilk", four, [["B", "A", 3.0], ["A", "B", 7.0], ["D", "C", 1.0], ["C", "B", -6.0], ["X", "A", 9.0]]),
        ("ahp-onbes", [f"Ö{k + 1}" for k in range(15)],
         [[f"Ö{i + 1}", f"Ö{j + 1}", float([-9, -8, -7, -6, -5, -4, -3, -2, 1, 2, 3, 4, 5, 6, 7, 8, 9][(i * 5 + j * 3) % 17])]
          for i in range(15) for j in range(i + 1, 15)]),
        ("ret-ahp-deger", ["A", "B"], [["A", "B", 1.5]]),
        ("ret-ahp-tek", ["A"], []),
    ]:
        inputs = [raster([1.0], 1, 1, [500000.0 + 10 * k, 10.0, 0.0, 4420000.0, 0.0, -10.0]) for k in range(len(names_))]
        t = {"kind": "pairwise", "band": 1, "pairs": pairs, "write": False, "sample": "f32"}
        r = pairwise(names_, pairs)
        out.append(ops_case(name, t, inputs, names_, r if "refused" in r else {"pairwise": r}))
    # ROC ile doğrulama (§8).
    pts = [pt(*centre_of(2, 1)), pt(*centre_of(7, 3)), pt(*centre_of(1, 1)), pt(500021.0, 4419985.0),  # the same cell as the first
           pt(*centre_of(1, 4), parts=[centre_of(8, 6), centre_of(5, 0)]), pt(499990.0, 4420005.0),  # off the raster
           pt(*centre_of(0, 4))]  # cell 40: no value
    for name, shapes, first, absence, higher in [
        ("roc-noktalar", pts, len(pts), False, True),
        ("roc-alanlar", [area(3, 2, 6, 5), pt(*centre_of(4, 3)), pt(*centre_of(9, 7))], 3, False, True),
        ("roc-yokluk", pts[:5] + [area(6, 2, 9, 5), pt(*centre_of(0, 0)), pt(*centre_of(1, 4))], 5, True, True),
        ("roc-dusuk-olasi", pts, len(pts), False, False),
        ("ret-roc-varlik-disarida", [pt(400000.0, 4000000.0)], 1, False, True),
        ("ret-roc-yokluk-secilmedi", pts[:2], 2, True, True),
        ("ret-roc-varlik-secilmedi", pts[:2], 0, True, True),
    ]:
        t = {"kind": "roc", "band": 1, "first": first, "absence": absence, "higher": higher}
        out.append(ops_case(name, t, [R], ["Duyarlılık"], roc(R, shapes, first, absence, higher), shapes))
    shapes = [area(2, 2, 26, 21)]
    t = {"kind": "roc", "band": 1, "first": 1, "absence": False, "higher": True}
    out.append(ops_case("roc-cok-esik", t, [D], ["Duyarlılık"], roc(D, shapes, 1, False, True), shapes))
    return out


def main():
    all_cases = cases()
    doc = {
        "format": "kentos.suitability-cases",
        "version": 1,
        "note": "ADR 0237'nin araçları, KentOS kodu olmadan ADR'nin kurallarından: girdi rasterleri (yer, boy, tür, değerler; null "
                "değersiz) ve adları, nesneler, aracın ayarları ve vermesi gereken: raster (boy, yer, her örnek; null değersiz), "
                "notlar, ağırlıklar ve tutarlılık, ROC'un sayıları ya da ret. rule \"exact\": bit bit; \"f32ulp\" ve "
                "\"f64ulp\": exp ve pow'lu değer sonucun türünde en çok bir son basamak birimi; \"eig\": ağırlıklar 1e-12, λ, CI, "
                "CR 1e-10. Üretici scripts/fixtures/suitability_cases.py.",
        "cases": all_cases,
    }
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT} güncel değil: python3 {sys.argv[0]} ile yeniden yazın ve farkı okuyun.")
            sys.exit(1)
        print(f"{OUT.name}: {len(all_cases)} durum; güncel.")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT} yazıldı: {len(all_cases)} durum.")


if __name__ == "__main__":
    main()
