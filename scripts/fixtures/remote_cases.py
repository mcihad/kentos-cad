#!/usr/bin/env python3
"""Uzaktan algılama (docs/adr/0242): the eight tools' rules written from the ADR with Python, without KentOS code, and the
cases the raster core plays (crates/shared/raster/tests/all/remote.rs).

- Bant birleştir, Bantlara ayır: the inputs read on their common grid (suitability_cases.py's, ADR 0237 §2) at the nearest
  cell or with exact bilinear weights (raster_ops_cases.py's rule, ADR 0233 §8); samples as the result's type holds them.
- Spektral indis: the ADR's formulas in float64 in their order, rounded once to float32 ("exact": the engine does the same).
- Denetimli sınıflandırma: the classes from the objects' texts (trimmed, natural order); the training cells (an area's
  centres by raster_ops_cases.py's exact rule; a cell under two areas of a class once); means and covariances as exact
  fractions; the Cholesky factor and the discriminants with mpmath at 50 digits. A cell whose best two discriminants are
  within 10⁻⁹ of each other (relative) would depend on float64, and is refused here: the cases avoid it.
- Denetimsiz sınıflandırma: the ADR's sample, start, rounds and numbering in float64 in the engine's order (the same floats).
- Doğruluk analizi: integer counts; each accuracy and kappa a single division of exact integers.
- Değişim tespiti: float64 in the ADR's order, rounded once to float32; class change in integers.
- Görüntü birleştirme: the multispectral image read on the panchromatic grid with exact bilinear or cubic weights, the ADR's
  formulas in float64, the result's type ("sum": within one unit of it). Weighted Brovey is cross-checked against GDAL's
  pansharpening (a VRTPansharpenedDataset) away from the edges.

The texts: numbers by the display rule (numeric_display.py's `shown`, ADR 0149) of the float the engine works out the same
way; the summaries' counts with a dot between thousands.

    python3 scripts/fixtures/remote_cases.py          # write fixtures/remote/v1/cases.json
    python3 scripts/fixtures/remote_cases.py --check  # compare
"""

import json
import math
import sys
from fractions import Fraction
from functools import cmp_to_key
from pathlib import Path

import mpmath
import numpy as np
from osgeo import gdal

sys.path.insert(0, str(Path(__file__).resolve().parent))
import raster_ops_cases as ro  # noqa: E402
import suitability_cases as su  # noqa: E402
from numeric_display import shown  # noqa: E402
from point_editor_cases import natural_cmp  # noqa: E402
from spatial_query_cases import JS_SPACE, read_number  # noqa: E402

gdal.UseExceptions()
mpmath.mp.dps = 50

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/remote/v1/cases.json"
NAN = math.nan
PLACE = [500000.0, 10.0, 0.0, 4420000.0, 0.0, -10.0]
COARSE = [500000.0, 20.0, 0.0, 4420000.0, 0.0, -20.0]
FINE = [500000.0, 5.0, 0.0, 4420000.0, 0.0, -5.0]
SAMPLE_CELLS = 250_000.0
MOST_VALUE = 2_147_483_647
INDEX_NAME = {"ndvi": "NDVI", "gndvi": "GNDVI", "savi": "SAVI", "evi": "EVI", "ndwi": "NDWI", "mndwi": "MNDWI", "ndbi": "NDBI",
              "ratio": "Oran", "normalized": "Normalize fark"}


def isnan(v):
    return isinstance(v, float) and math.isnan(v)


def fixed(v, d):
    return shown(float(v), d)


def count(n):
    return f"{n:,}".replace(",", ".")


def cell_area(affine):
    """As the engine works it out: |a·d − b·c| in float64."""
    _, a, b, _, c, d = affine
    return abs(a * d - b * c)


def table(columns, rows):
    return {"columns": columns, "rows": rows}


def raster_result(width, height, bands, affine, values, sample, nodata, rule):
    res = ro.result(width, height, bands, affine, values, rule)
    res["raster"]["sample"] = sample
    res["raster"]["nodata"] = "nan" if isnan(nodata) else nodata
    return res


def empty_of(sample, values, alpha, nodata):
    """The engine's rule (raster_ops_cases.py's): what a cell without a value holds, whether an alpha band is added."""
    nd = None if nodata is None or isnan(nodata) or nodata == "nan" else nodata
    empty, alpha = ro.empty_of(sample, values, alpha, nd)
    if empty == "nan":
        return NAN, alpha
    return empty, alpha


def fill_of(nodata):
    return NAN if nodata is None or isnan(nodata) else nodata


def nodata_of(r):
    return NAN if r.nodata == "nan" else r.nodata


# ── Bant birleştir (§3), Bantlara ayır (§4) ──────────────────────────────

def composite(inputs, names, sampling):
    if len(inputs) < 2:
        return {"refused": "Bant birleştir en az iki raster ister."}
    grid = su.common_grid(inputs)
    if grid is None:
        return {"refused": "Rasterler örtüşmüyor: ortak alanlarında hiç hücre merkezi yok."}
    affine, w, h = grid
    same = all(r.sample == inputs[0].sample and r.nodata == inputs[0].nodata for r in inputs)
    sample = inputs[0].sample if same else "f32"
    values = sum(r.value_bands() for r in inputs)
    nodata, alpha = empty_of(sample, values, any(r.alpha for r in inputs), nodata_of(inputs[0]) if same else None)
    fill = 0.0 if nodata is None else fill_of(nodata)
    out = []
    for j in range(h):
        for i in range(w):
            x, y = ro.centre(affine, i, j)
            got = []
            for r in inputs:
                u, v = ro.place_in(r.affine, x, y)
                got.extend(ro.sample(r, b, u, v, sampling) for b in range(r.value_bands()))
            out.extend(fill if isnan(s) else ro.exact_to(sample, s) for s in got)
            if alpha:
                out.append(255.0 if any(not isnan(s) for s in got) else 0.0)
    rows = []
    for r, name in zip(inputs, names):
        for b in range(r.value_bands()):
            rows.append([str(len(rows) + 1), name, str(b + 1)])
    res = raster_result(w, h, values + (1 if alpha else 0), affine, out, sample, nodata,
                        "exact" if sampling == "nearest" else "sum")
    return {**res, "notes": {"table": table(["Bant", "Kaynak", "Kaynağın bandı"], rows), "tail": f"{values} bant.", "warnings": []}}


def band(r, b):
    vb = r.value_bands()
    if b < 1 or b > vb:
        return {"refused": f"Rasterin {vb} bandı var; {b}. bant yok."}
    nodata, alpha = empty_of(r.sample, 1, r.alpha, nodata_of(r))
    fill = 0.0 if nodata is None else fill_of(nodata)
    out = []
    for j in range(r.height):
        for i in range(r.width):
            s = r.get(i, j, b - 1)
            out.append(fill if isnan(s) else ro.exact_to(r.sample, s))
            if alpha:
                out.append(0.0 if isnan(s) else 255.0)
    res = raster_result(r.width, r.height, 1 + (1 if alpha else 0), r.affine, out, r.sample, nodata, "exact")
    return {**res, "notes": {"table": None, "tail": f"{b}. bant.", "warnings": []}}


# ── Spektral indis (§5) ──────────────────────────────────────────────────

USES = {"ndvi": ("red", "nir"), "gndvi": ("green", "nir"), "savi": ("red", "nir"), "evi": ("blue", "red", "nir"),
        "ndwi": ("green", "nir"), "mndwi": ("green", "swir"), "ndbi": ("swir", "nir"), "ratio": ("a", "b"), "normalized": ("a", "b")}


def index_of(kind, v, t):
    """The index from the bands' reflectances (floats), the ADR's order; None where the divisor is 0."""
    if kind in ("ndvi", "gndvi", "ndwi", "mndwi", "ndbi", "normalized"):
        p, q = {"ndvi": ("nir", "red"), "gndvi": ("nir", "green"), "ndwi": ("green", "nir"), "mndwi": ("green", "swir"),
                "ndbi": ("swir", "nir"), "normalized": ("a", "b")}[kind]
        num, den = v[p] - v[q], v[p] + v[q]
    elif kind == "savi":
        num, den = (1.0 + t["saviL"]) * (v["nir"] - v["red"]), v["nir"] + v["red"] + t["saviL"]
    elif kind == "evi":
        num, den = t["g"] * (v["nir"] - v["red"]), v["nir"] + t["c1"] * v["red"] - t["c2"] * v["blue"] + t["eviL"]
    else:
        num, den = v["a"], v["b"]
    if den == 0.0:
        return None
    return num / den


def index(r, t):
    kind = t["index"]
    vb = r.value_bands()
    for name in USES[kind]:
        b = t["bands"][name]
        if b < 1 or b > vb:
            return {"refused": f"Rasterin {vb} bandı var; {b}. bant yok."}
    out, zero = [], 0
    for j in range(r.height):
        for i in range(r.width):
            v, empty = {}, False
            for name in USES[kind]:
                s = r.get(i, j, t["bands"][name] - 1)
                if isnan(s):
                    empty = True
                    break
                v[name] = float(s) * t["scale"] + t["offset"]
            if empty:
                out.append(NAN)
                continue
            x = index_of(kind, v, t)
            if x is None:
                zero += 1
                out.append(NAN)
            else:
                out.append(ro.f32(x))
    have = [x for x in out if not isnan(x)]
    name = INDEX_NAME[kind]
    tail = f"{name} {fixed(min(have), 4)} … {fixed(max(have), 4)}." if have else f"{name}: hiçbir hücrede değer yok."
    warnings = [f"{count(zero)} hücrede bölen sıfır; boş bırakıldı."] if zero else []
    res = raster_result(r.width, r.height, 1, r.affine, out, "f32", NAN, "exact")
    return {**res, "notes": {"table": None, "tail": tail, "warnings": warnings}}


# ── Denetimli sınıflandırma (§6) ─────────────────────────────────────────

def mp(q):
    return mpmath.mpf(q.numerator) / q.denominator


def classes_of(texts):
    """The classes: the trimmed, non-empty texts in their natural order; each object's value (0: none)."""
    trimmed = [t.strip(JS_SPACE) if t is not None else "" for t in texts]
    names = sorted({t for t in trimmed if t}, key=cmp_to_key(natural_cmp))
    of = [names.index(t) + 1 if t else 0 for t in trimmed]
    return names, of


def cholesky(cov, b):
    """The lower factor (mpmath); None when a pivot's rest is not above 10⁻¹² of its variance."""
    low = [[mpmath.mpf(0)] * b for _ in range(b)]
    for j in range(b):
        rest = cov[j][j] - mpmath.fsum(low[j][k] ** 2 for k in range(j))
        if not rest > mpmath.mpf("1e-12") * cov[j][j]:
            return None
        low[j][j] = mpmath.sqrt(rest)
        for i in range(j + 1, b):
            low[i][j] = (cov[i][j] - mpmath.fsum(low[i][k] * low[j][k] for k in range(j))) / low[j][j]
    return low


def supervised(r, shapes, texts, method):
    vb = r.value_bands()
    names, of = classes_of(texts)
    k = len(names)
    if k == 0:
        return {"refused": "Eğitim alanı yok: Sınıf alanı dolu en az bir alan seçin."}
    sets = [set() for _ in range(k + 1)]
    for s, c in zip(shapes, of):
        if c:
            sets[c] |= ro.inside_cells(s, r.affine, r.width, r.height)
    vecs = []
    for c in range(1, k + 1):
        vs = []
        for (i, j) in sorted(sets[c], key=lambda t: (t[1], t[0])):
            x = [r.get(i, j, b) for b in range(vb)]
            if not any(isnan(v) for v in x):
                vs.append([Fraction(v) for v in x])
        vecs.append(vs)
    models = []
    for name, vs in zip(names, vecs):
        if method == "likelihood" and len(vs) < vb + 1:
            return {"refused": f"“{name}” sınıfının eğitim hücresi {len(vs)}; en büyük olabilirlik en az {vb + 1} ister."}
        if not vs:
            return {"refused": f"“{name}” sınıfının eğitim hücresi yok."}
        n = len(vs)
        mean = [sum(x[b] for x in vs) / n for b in range(vb)]
        if method == "likelihood":
            cov = [[mp(sum((x[a] - mean[a]) * (x[c] - mean[c]) for x in vs) / (n - 1)) for c in range(vb)] for a in range(vb)]
            low = cholesky(cov, vb)
            if low is None:
                return {"refused": f"“{name}” sınıfının kovaryansı tekil: bantlarından biri sabit ya da bantları doğrusal bağımlı."}
            logdet = 2 * mpmath.fsum(mpmath.log(low[j][j]) for j in range(vb))
            models.append((mean, low, logdet))
        else:
            models.append((mean, None, None))
    out, counts = [], [0] * (k + 1)
    for j in range(r.height):
        for i in range(r.width):
            x = [r.get(i, j, b) for b in range(vb)]
            if any(isnan(v) for v in x):
                out.append(0.0)
                continue
            scores = []
            for (mean, low, logdet) in models:
                d = [mp(Fraction(x[b]) - mean[b]) for b in range(vb)]
                if method == "likelihood":
                    z = []
                    for a in range(vb):
                        z.append((d[a] - mpmath.fsum(low[a][q] * z[q] for q in range(a))) / low[a][a])
                    scores.append(-logdet - mpmath.fsum(v * v for v in z))
                else:
                    scores.append(-mpmath.fsum(v * v for v in d))
            best = max(range(k), key=lambda c: (scores[c], -c))
            for c in range(k):
                if c != best and abs(scores[c] - scores[best]) <= mpmath.mpf("1e-9") * max(abs(scores[best]), 1):
                    raise AssertionError(f"({i}, {j}): two classes too near for float64")
            out.append(float(best + 1))
            counts[best + 1] += 1
    area = cell_area(r.affine)
    rows = [[names[c], str(c + 1), str(len(vecs[c])), str(counts[c + 1]), fixed(float(counts[c + 1]) * area, 2)] for c in range(k)]
    sample = "u8" if k <= 255 else "u16"
    res = raster_result(r.width, r.height, 1, r.affine, out, sample, 0.0, "exact")
    tail = f"{k} sınıf, {count(sum(len(v) for v in vecs))} eğitim hücresi."
    return {**res, "notes": {"table": table(["Sınıf", "Değer", "Eğitim hücresi", "Hücre sayısı", "Alan (m²)"], rows),
                             "tail": tail, "warnings": []}}


# ── Denetimsiz sınıflandırma (§7) ────────────────────────────────────────

def d2(x, c):
    s = 0.0
    for a, b in zip(x, c):
        e = a - b
        s += e * e
    return s


def nearest(x, centers):
    best, bd = 0, math.inf
    for c, ctr in enumerate(centers):
        d = d2(x, ctr)
        if d < bd:
            best, bd = c, d
    return best


def unsupervised(r, k, iterations):
    vb = r.value_bands()
    s = math.ceil(math.sqrt(r.width * r.height / SAMPLE_CELLS))
    s = s if s > 1 else 1
    sample = []
    for j in range(0, r.height, s):
        for i in range(0, r.width, s):
            x = [r.get(i, j, b) for b in range(vb)]
            if not any(isnan(v) for v in x):
                sample.append([float(v) for v in x])
    n = len(sample)
    if n < k:
        return {"refused": f"Örnek hücre sayısı ({n}) küme sayısından ({k}) az."}
    centers = [[0.0] * vb for _ in range(k)]
    for b in range(vb):
        t = 0.0
        for x in sample:
            t += x[b]
        mu = t / n
        q = 0.0
        for x in sample:
            e = x[b] - mu
            q += e * e
        sd = math.sqrt(q / n)
        for c in range(k):
            centers[c][b] = mu + sd * (2.0 * (c + 0.5) / k - 1.0)
    labels = [nearest(x, centers) for x in sample]
    rounds = 0
    while rounds < iterations:
        sums = [[0.0] * vb for _ in range(k)]
        members = [0] * k
        for x, lab in zip(sample, labels):
            for b in range(vb):
                sums[lab][b] += x[b]
            members[lab] += 1
        for c in range(k):
            if members[c]:
                centers[c] = [sums[c][b] / members[c] for b in range(vb)]
        rounds += 1
        new = [nearest(x, centers) for x in sample]
        if new == labels:
            break
        labels = new
    totals = []
    for c in range(k):
        t = 0.0
        for b in range(vb):
            t += centers[c][b]
        totals.append(t)
    order = sorted(range(k), key=lambda c: (totals[c], c))
    ranked = [centers[c] for c in order]
    out, counts = [], [0] * (k + 1)
    for j in range(r.height):
        for i in range(r.width):
            x = [r.get(i, j, b) for b in range(vb)]
            if any(isnan(v) for v in x):
                out.append(0.0)
                continue
            c = nearest([float(v) for v in x], ranked)
            out.append(float(c + 1))
            counts[c + 1] += 1
    area = cell_area(r.affine)
    rows = [[str(c + 1), str(counts[c + 1]), fixed(float(counts[c + 1]) * area, 2)] + [fixed(v, 3) for v in ranked[c]]
            for c in range(k)]
    res = raster_result(r.width, r.height, 1, r.affine, out, "u8", 0.0, "exact")
    tail = f"{k} küme, {rounds} yineleme ({count(n)} örnek hücre)."
    columns = ["Küme", "Hücre sayısı", "Alan (m²)"] + [f"Merkez {b + 1}" for b in range(vb)]
    return {**res, "notes": {"table": table(columns, rows), "tail": tail, "warnings": []}}


# ── Doğruluk analizi (§8) ────────────────────────────────────────────────

def reference_of(text):
    """A reference text as a whole number (kentos.statistics/1) within ±(2³¹ − 1), or None."""
    if text is None:
        return None
    d = read_number(text)
    if d is None:
        return None
    m, s = d
    if m % 10 ** s:
        return None
    v = m // 10 ** s if m >= 0 else -((-m) // 10 ** s)
    return v if abs(v) <= MOST_VALUE else None


def accuracy(r, shapes, refs, band=1):
    pairs, unread, off = {}, 0, 0

    def take(i, j, ref):
        nonlocal off
        g = r.get(i, j, band - 1)
        if g is None or isnan(g):
            off += 1
        else:
            key = (math.trunc(g), ref)
            pairs[key] = pairs.get(key, 0) + 1

    for s, text in zip(shapes, refs):
        ref = reference_of(text)
        if ref is None:
            unread += 1
            continue
        if s["kind"] == "point":
            for p in [s["p"]] + [q["p"] for q in s.get("parts", [])]:
                u, v = ro.place_in(r.affine, p["x"], p["y"])
                take(math.floor(u), math.floor(v), ref)
        else:
            for (i, j) in ro.inside_cells(s, r.affine, r.width, r.height):
                take(i, j, ref)
    n = sum(pairs.values())
    if n == 0:
        return {"refused": "Değerlendirilecek hücre yok: referans nesneleri rasterin değerli hücrelerine düşmüyor."}
    classes = sorted({c for c, _ in pairs} | {x for _, x in pairs})
    at = lambda c, x: pairs.get((c, x), 0)  # noqa: E731
    row_t = {c: sum(at(c, x) for x in classes) for c in classes}
    col_t = {x: sum(at(c, x) for c in classes) for x in classes}

    def pct(part, whole):
        return fixed(Fraction(part * 100, whole), 2) if whole else "—"

    rows = []
    for c in classes:
        rows.append([str(c)] + [str(at(c, x)) for x in classes] + [str(row_t[c]), pct(at(c, c), row_t[c])])
    rows.append(["Toplam"] + [str(col_t[x]) for x in classes] + [str(n), ""])
    rows.append(["Üretici doğruluğu (%)"] + [pct(at(x, x), col_t[x]) for x in classes] + ["", ""])
    diag = sum(at(c, c) for c in classes)
    s_rc = sum(row_t[c] * col_t[c] for c in classes)
    overall = float(Fraction(diag * 100, n))
    kappa = 1.0 if n * n == s_rc else float(Fraction(n * diag - s_rc, n * n - s_rc))
    columns = ["Sınıflandırılan \\ Referans"] + [str(x) for x in classes] + ["Toplam", "Kullanıcı doğruluğu (%)"]
    warnings = []
    if unread:
        warnings.append(f"{count(unread)} nesnenin referansı tam sayı olarak okunamadı; alınmadı.")
    if off:
        warnings.append(f"{count(off)} hücre rasterin dışında ya da boş; alınmadı.")
    summary = f"Genel doğruluk %{fixed(overall, 2)}, kappa {fixed(kappa, 4)} ({count(n)} hücre)."
    return {"accuracy": {"table": table(columns, rows), "summary": summary, "warnings": warnings,
                         "overall": overall, "kappa": kappa}}


# ── Değişim tespiti (§9) ─────────────────────────────────────────────────

def change(before, after, b, method):
    for r in (before, after):
        if b < 1 or b > r.value_bands():
            return {"refused": f"Rasterin {r.value_bands()} bandı var; {b}. bant yok."}
    grid = su.common_grid([before, after])
    if grid is None:
        return {"refused": "Rasterler örtüşmüyor: ortak alanlarında hiç hücre merkezi yok."}
    affine, w, h = grid
    out, inc, dec, same, pairs = [], 0, 0, 0, {}
    for j in range(h):
        for i in range(w):
            x, y = ro.centre(affine, i, j)
            ua, va = ro.place_in(before.affine, x, y)
            uc, vc = ro.place_in(after.affine, x, y)
            a = ro.sample(before, b - 1, ua, va, "nearest")
            c = ro.sample(after, b - 1, uc, vc, "nearest")
            if method == "classes":
                if isnan(a) or isnan(c) or a == 0 or c == 0:
                    out.append(0.0)
                    continue
                key = (math.trunc(a), math.trunc(c))
                pairs[key] = pairs.get(key, 0) + 1
                out.append(float(key[0] * 1000 + key[1]))
                continue
            if isnan(a) or isnan(c):
                out.append(NAN)
                continue
            if c > a:
                inc += 1
            elif c < a:
                dec += 1
            else:
                same += 1
            fa, fc = float(a), float(c)
            if method == "difference":
                out.append(ro.f32(fc - fa))
            elif method == "ratio":
                out.append(NAN if fa == 0.0 else ro.f32(fc / fa))
            else:
                den = fc + fa
                out.append(NAN if den == 0.0 else ro.f32((fc - fa) / den))
    if method == "classes":
        froms = sorted({p for p, _ in pairs})
        tos = sorted({q for _, q in pairs})
        rows = []
        for p in froms:
            row = [pairs.get((p, q), 0) for q in tos]
            rows.append([str(p)] + [str(v) for v in row] + [str(sum(row))])
        total = sum(pairs.values())
        rows.append(["Toplam"] + [str(sum(pairs.get((p, q), 0) for p in froms)) for q in tos] + [str(total)])
        changed = sum(v for (p, q), v in pairs.items() if p != q)
        tail = (f"Değişen hücre {count(changed)} / {count(total)} (%{fixed(Fraction(changed * 100, total), 2)})."
                if total else "Değerli ortak hücre yok.")
        res = raster_result(w, h, 1, affine, out, "i32", 0.0, "exact")
        return {**res, "notes": {"table": table(["Önceki \\ Sonraki"] + [str(q) for q in tos] + ["Toplam"], rows), "tail": tail,
                                 "warnings": []}}
    res = raster_result(w, h, 1, affine, out, "f32", NAN, "exact")
    return {**res, "notes": {"table": None, "tail": f"Artan {count(inc)}, azalan {count(dec)}, değişmeyen {count(same)} hücre.",
                             "warnings": []}}


# ── Görüntü birleştirme (§10) ────────────────────────────────────────────

def weights_of(text):
    """Weights typed as numbers (kentos.statistics/1) apart by semicolons or spaces; None when none."""
    out = []
    for tok in [t for t in text.replace(";", " ").split() if t]:
        d = read_number(tok)
        if d is None:
            raise ValueError(f"Ağırlık okunamadı: “{tok}”; sayıları noktalı virgülle ayırın.")
        out.append(float(Fraction(d[0], 10 ** d[1])))
    return out or None


def pansharpen(ms, pan, method, weights, sampling):
    n = ms.value_bands()
    if isinstance(weights, str):
        try:
            weights = weights_of(weights)
        except ValueError as e:
            return {"refused": str(e)}
    if weights is not None and len(weights) != n:
        return {"refused": f"Ağırlıkların sayısı ({len(weights)}) çok bantlının bant sayısı ({n}) değil."}
    w = [float(x) for x in weights] if weights is not None else [1.0 / n] * n
    nodata, alpha = empty_of(ms.sample, n, ms.alpha, nodata_of(ms))
    fill = 0.0 if nodata is None else fill_of(nodata)
    out = []
    for j in range(pan.height):
        for i in range(pan.width):
            p = pan.get(i, j, 0)
            x, y = ro.centre(pan.affine, i, j)
            u, v = ro.place_in(ms.affine, x, y)
            vals = [ro.sample(ms, b, u, v, sampling) for b in range(n)]
            if isnan(p) or any(isnan(s) for s in vals):
                out.extend([fill] * n)
                if alpha:
                    out.append(0.0)
                continue
            fs = [float(s) for s in vals]
            if method == "brovey":
                s = 0.0
                for wk, fk in zip(w, fs):
                    s += wk * fk
                res = [0.0] * n if s <= 0.0 else [fk * float(p) / s for fk in fs]
            else:
                res = [(fk + float(p)) / 2.0 for fk in fs]
            out.extend(ro.stored(ms.sample, q) for q in res)
            if alpha:
                out.append(255.0)
    res = raster_result(pan.width, pan.height, n + (1 if alpha else 0), pan.affine, out, ms.sample, nodata,
                        "exact" if sampling == "nearest" else "sum")
    name = "Brovey" if method == "brovey" else "Basit ortalama"
    return {**res, "notes": {"table": None, "tail": f"{name}, {n} bant.", "warnings": []}}


# ── Rasters and shapes ───────────────────────────────────────────────────

def u16(values, w, h, bands=1, affine=None, nodata=0.0):
    return ro.Raster(list(affine or PLACE), w, h, values, bands, "u16", nodata)


def f32r(values, w, h, bands=1, affine=None):
    return ro.Raster(list(affine or PLACE), w, h, values, bands, "f32", "nan")


def zone(i, j):
    """The land cover of cell (i, j) of the 12 × 10 image: 0 water, 1 forest, 2 soil, 3 built-up."""
    if i < 4:
        return 0
    if i < 8:
        return 1 if j < 6 else 3
    return 2


SPECTRA = [  # blue, green, red, nir: water, forest, soil, built-up
    (900, 800, 500, 300),
    (400, 700, 400, 3200),
    (1100, 1300, 1500, 1900),
    (1600, 1650, 1700, 1800),
]


def image(holes=(17, 66)):
    vals = []
    for k in range(120):
        i, j = k % 12, k // 12
        z = zone(i, j)
        for b in range(4):
            if k in holes:
                vals.append(0.0)
            else:
                noise = ((k * 37 + b * 11) % 61) - 30
                vals.append(float(SPECTRA[z][b] + noise * (b + 1)))
    return u16(vals, 12, 10, 4)


IMG = image()
BANDS = [u16([IMG.values[k * 4 + b] for k in range(120)], 12, 10) for b in range(4)]
# A coarse band on 20 m cells (6 × 5) and a float band with an empty cell.
COARSE_BAND = u16([float(1000 + 37 * k) for k in range(30)], 6, 5, affine=COARSE)
FLOAT_BAND = f32r([NAN if k == 5 else ro.f32(0.125 * k - 3.0) for k in range(120)], 12, 10)
# Multispectral on 20 m (6 × 5) and panchromatic on 5 m (24 × 20) over the same 120 m × 100 m.
MS = u16([float(SPECTRA[zone(2 * (k // 4 % 6), 2 * (k // 24))][k % 4] + (k * 13 % 17)) for k in range(120)], 6, 5, 4, COARSE)
PAN = u16([float(800 + ((k % 24) * 31 + (k // 24) * 17) % 400) for k in range(480)], 24, 20, affine=FINE)


def classes_raster(seed, holes=()):
    vals = []
    for k in range(120):
        z = zone(k % 12, k // 12)
        moved = (k * seed // 13) % 2 if k % 7 == 0 else 0
        vals.append(0.0 if k in holes else float(1 + (z + moved) % 4))
    return ro.Raster(list(PLACE), 12, 10, vals, 1, "u8", 0.0)


BEFORE = classes_raster(3, holes=(0, 59))
AFTER = classes_raster(5, holes=(119,))


def cell_square(i0, j0, i1, j1):
    """An area over cells i0…i1 − 1, j0…j1 − 1 (a tenth of a cell inside their edges)."""
    (x0, y0), (x1, y1) = ro.point_of(PLACE, i0 + 0.1, j0 + 0.1), ro.point_of(PLACE, i1 - 0.1, j1 - 0.1)
    return ro.square(min(x0, x1), min(y0, y1), max(x0, x1), max(y0, y1))


def pt_at(i, j, affine=None, parts=()):
    x, y = ro.centre(affine or PLACE, i, j)
    s = {"kind": "point", "p": {"x": x, "y": y}}
    if parts:
        s["parts"] = []
        for (pi, pj) in parts:
            px, py = ro.centre(affine or PLACE, pi, pj)
            s["parts"].append({"p": {"x": px, "y": py}})
    return s


def ops_case(name, tool, inputs, names, expect, shapes=()):
    return {"name": name, "tool": tool, "inputs": [r.json() for r in inputs], "names": names,
            "shapes": list(shapes), "expect": expect}


def cases():
    out = []
    names4 = ["Mavi", "Yeşil", "Kırmızı", "YKÖ"]
    # Bant birleştir.
    t = {"kind": "composite", "sampling": "nearest"}
    out.append(ops_case("birlestir-dort-bant", t, BANDS, names4, composite(BANDS, names4, "nearest")))
    pair = [BANDS[0], FLOAT_BAND]
    out.append(ops_case("birlestir-karisik-tur", t, pair, ["Mavi", "Kayan"], composite(pair, ["Mavi", "Kayan"], "nearest")))
    pair = [BANDS[2], COARSE_BAND]
    out.append(ops_case("birlestir-kaba-en-yakin", t, pair, ["Kırmızı", "Kaba"], composite(pair, ["Kırmızı", "Kaba"], "nearest")))
    tb = {"kind": "composite", "sampling": "bilinear"}
    out.append(ops_case("birlestir-kaba-cift-dogrusal", tb, pair, ["Kırmızı", "Kaba"],
                        composite(pair, ["Kırmızı", "Kaba"], "bilinear")))
    out.append(ops_case("ret-birlestir-tek", t, [BANDS[0]], ["Mavi"], composite([BANDS[0]], ["Mavi"], "nearest")))
    far = u16([1.0] * 4, 2, 2, affine=[600000.0, 10.0, 0.0, 4420000.0, 0.0, -10.0])
    out.append(ops_case("ret-birlestir-ortusmeyen", t, [BANDS[0], far], ["Mavi", "Uzak"],
                        composite([BANDS[0], far], ["Mavi", "Uzak"], "nearest")))
    # Bantlara ayır, one band a run.
    for b in (1, 2, 3, 4):
        out.append(ops_case(f"ayir-bant-{b}", {"kind": "band", "band": b}, [IMG], ["Görüntü"], band(IMG, b)))
    out.append(ops_case("ret-ayir-bant-5", {"kind": "band", "band": 5}, [IMG], ["Görüntü"], band(IMG, 5)))
    # Spektral indis.
    bands = {"blue": 1, "green": 2, "red": 3, "nir": 4, "swir": 1, "a": 4, "b": 3}
    base = {"kind": "index", "bands": bands, "scale": 1.0, "offset": 0.0, "saviL": 0.5, "g": 2.5, "c1": 6.0, "c2": 7.5, "eviL": 1.0}
    for kind in ("ndvi", "gndvi", "savi", "ndwi", "mndwi", "ndbi", "ratio", "normalized"):
        tt = {**base, "index": kind}
        out.append(ops_case(f"indis-{kind}", tt, [IMG], ["Görüntü"], index(IMG, tt)))
    tt = {**base, "index": "evi", "scale": 0.0001, "offset": 0.0}
    out.append(ops_case("indis-evi-yansima", tt, [IMG], ["Görüntü"], index(IMG, tt)))
    tt = {**base, "index": "ndvi", "scale": 0.0000275, "offset": -0.2}
    out.append(ops_case("indis-ndvi-landsat", tt, [IMG], ["Görüntü"], index(IMG, tt)))
    pairs = [(0, 0), (100, 50), (0, 30), (40, 0), (0, 0), (7, 7), (65535, 1), (3, 65535)]
    zero = u16([float(v) for p in pairs for v in p], 4, 2, 2, nodata=None)
    tt = {**base, "index": "normalized", "bands": {**bands, "a": 1, "b": 2}}
    out.append(ops_case("indis-bolen-sifir", tt, [zero], ["Sıfırlı"], index(zero, tt)))
    tt = {**base, "index": "mndwi", "bands": {**bands, "swir": 6}}
    out.append(ops_case("ret-indis-bant", tt, [IMG], ["Görüntü"], index(IMG, tt)))
    # Denetimli sınıflandırma: four classes over the zones, an empty text, a class's two areas overlapping.
    texts = ["Su", "Orman", "Toprak", "Yapı", " Su ", "", "Orman", None]
    squares = [cell_square(0, 0, 3, 4), cell_square(4, 0, 8, 3), cell_square(9, 2, 12, 9), cell_square(4, 7, 8, 10),
               cell_square(1, 3, 4, 10), cell_square(0, 0, 12, 10), cell_square(5, 2, 7, 5), cell_square(0, 0, 12, 10)]
    for method in ("likelihood", "distance"):
        tt = {"kind": "supervised", "method": method, "texts": texts}
        out.append(ops_case(f"denetimli-{method}", tt, [IMG], ["Görüntü"], supervised(IMG, squares, texts, method), squares))
    tiny = [cell_square(0, 0, 1, 2), cell_square(4, 0, 8, 3)]
    tt = {"kind": "supervised", "method": "likelihood", "texts": ["Su", "Orman"]}
    out.append(ops_case("ret-denetimli-az-hucre", tt, [IMG], ["Görüntü"], supervised(IMG, tiny, ["Su", "Orman"], "likelihood"), tiny))
    tt = {"kind": "supervised", "method": "likelihood", "texts": ["", None]}
    out.append(ops_case("ret-denetimli-sinifsiz", tt, [IMG], ["Görüntü"], supervised(IMG, tiny, ["", None], "likelihood"), tiny))
    flat = u16([700.0 if k % 2 else float(500 + (k // 2) % 3) for k in range(240)], 12, 10, 2)
    whole = [cell_square(0, 0, 12, 10)]
    tt = {"kind": "supervised", "method": "likelihood", "texts": ["Düz"]}
    out.append(ops_case("ret-denetimli-tekil", tt, [flat], ["Düz"], supervised(flat, whole, ["Düz"], "likelihood"), whole))
    # Denetimsiz sınıflandırma.
    for k, it in ((4, 20), (3, 1), (6, 50)):
        tt = {"kind": "unsupervised", "clusters": k, "iterations": it}
        out.append(ops_case(f"denetimsiz-{k}-{it}", tt, [IMG], ["Görüntü"], unsupervised(IMG, k, it)))
    small = u16([float(100 + k) for k in range(4)], 2, 2)
    tt = {"kind": "unsupervised", "clusters": 6, "iterations": 20}
    out.append(ops_case("ret-denetimsiz-az-ornek", tt, [small], ["Küçük"], unsupervised(small, 6, 20)))
    # Doğruluk analizi: points on cells (one with a second point), an area, an unreadable reference, a point off the raster.
    cls = ro.Raster(list(PLACE), 12, 10, [0.0 if k == 13 else float(1 + zone(k % 12, k // 12)) for k in range(120)], 1, "u8", 0.0)
    spots = [(0, 0), (1, 1), (5, 1), (6, 2), (9, 4), (10, 8), (5, 8), (6, 9), (2, 5), (7, 4), (1, 1)]
    refs = ["1", "1", "2", "2", "3", "3", "4", "4", "2", "3", "1"]
    shapes = [pt_at(i, j) for (i, j) in spots]
    shapes += [cell_square(4, 0, 6, 2), pt_at(1, 1), {"kind": "point", "p": {"x": 499000.0, "y": 4420050.0}},
               pt_at(9, 9, parts=[(10, 9)])]
    refs += ["2", "x", "1", " 3,0 "]
    tt = {"kind": "accuracy", "band": 1, "reference": refs}
    out.append(ops_case("dogruluk", tt, [cls], ["Sınıflar"], accuracy(cls, shapes, refs), shapes))
    off = [{"kind": "point", "p": {"x": 499000.0, "y": 4420050.0}}]
    tt = {"kind": "accuracy", "band": 1, "reference": ["5"]}
    out.append(ops_case("ret-dogruluk-hucresiz", tt, [cls], ["Sınıflar"], accuracy(cls, off, ["5"]), off))
    # Değişim tespiti.
    later = u16([v + (40.0 if (k // 4) % 3 == 0 else -25.0) if v else 0.0 for k, v in enumerate(IMG.values)], 12, 10, 4)
    for method in ("difference", "ratio", "normalized"):
        tt = {"kind": "change", "band": 4, "method": method}
        out.append(ops_case(f"degisim-{method}", tt, [IMG, later], ["Önceki", "Sonraki"], change(IMG, later, 4, method)))
    tt = {"kind": "change", "band": 1, "method": "classes"}
    out.append(ops_case("degisim-siniflar", tt, [BEFORE, AFTER], ["Önceki", "Sonraki"], change(BEFORE, AFTER, 1, "classes")))
    tt = {"kind": "change", "band": 2, "method": "difference"}
    out.append(ops_case("ret-degisim-bant", tt, [BEFORE, AFTER], ["Önceki", "Sonraki"], change(BEFORE, AFTER, 2, "difference")))
    # Görüntü birleştirme.
    for method, weights, sampling in (("brovey", None, "cubic"), ("brovey", [0.1, 0.3, 0.3, 0.3], "bilinear"),
                                      ("mean", None, "nearest")):
        tt = {"kind": "pansharpen", "method": method, "weights": weights, "sampling": sampling}
        tag = f"{method}-{sampling}" + ("-agirlikli" if weights else "")
        out.append(ops_case(f"birlestirme-{tag}", tt, [MS, PAN], ["Çok bantlı", "Pankromatik"],
                            pansharpen(MS, PAN, method, weights, sampling)))
    tt = {"kind": "pansharpen", "method": "brovey", "weights": "0,1; 0,3 0,3;0,3", "sampling": "bilinear"}
    out.append(ops_case("birlestirme-agirlik-metni", tt, [MS, PAN], ["Çok bantlı", "Pankromatik"],
                        pansharpen(MS, PAN, "brovey", "0,1; 0,3 0,3;0,3", "bilinear")))
    tt = {"kind": "pansharpen", "method": "brovey", "weights": " ", "sampling": "cubic"}
    out.append(ops_case("birlestirme-agirlik-bos", tt, [MS, PAN], ["Çok bantlı", "Pankromatik"],
                        pansharpen(MS, PAN, "brovey", " ", "cubic")))
    tt = {"kind": "pansharpen", "method": "brovey", "weights": [0.5, 0.5], "sampling": "cubic"}
    out.append(ops_case("ret-birlestirme-agirlik", tt, [MS, PAN], ["Çok bantlı", "Pankromatik"],
                        pansharpen(MS, PAN, "brovey", [0.5, 0.5], "cubic")))
    tt = {"kind": "pansharpen", "method": "brovey", "weights": "0,1; x", "sampling": "cubic"}
    out.append(ops_case("ret-birlestirme-agirlik-metni", tt, [MS, PAN], ["Çok bantlı", "Pankromatik"],
                        pansharpen(MS, PAN, "brovey", "0,1; x", "cubic")))
    return out


# ── GDAL's pansharpening ─────────────────────────────────────────────────

def gdal_raster(r, path):
    """`r` as a GeoTIFF GDAL writes."""
    kind = {"u16": gdal.GDT_UInt16, "u8": gdal.GDT_Byte, "f32": gdal.GDT_Float32}[r.sample]
    ds = gdal.GetDriverByName("GTiff").Create(str(path), r.width, r.height, r.bands, kind)
    x0, a, b, y0, c, d = r.affine
    ds.SetGeoTransform([x0, a, b, y0, c, d])
    arr = np.array(r.values, dtype=np.float64).reshape(r.height, r.width, r.bands)
    for k in range(r.bands):
        ds.GetRasterBand(k + 1).WriteArray(arr[:, :, k])
    ds.FlushCache()
    return ds


def cross_check(all_cases):
    """Weighted Brovey with cubic resampling against GDAL's VRTPansharpenedDataset, away from the edges. GDAL rounds the
    resampled spectral bands to their type (half up) before the formula: with that rounding the reference gives GDAL's
    samples exactly; the tool, which does not round them, is within one unit."""
    want = next(c for c in all_cases if c["name"] == "birlestirme-brovey-cubic")["expect"]["raster"]
    tmp = ROOT / ".run" / "remote-cross"
    tmp.mkdir(parents=True, exist_ok=True)
    ms = gdal_raster(MS, tmp / "ms.tif")
    pan = gdal_raster(PAN, tmp / "pan.tif")
    xml = """<VRTDataset subClass="VRTPansharpenedDataset">
  <PansharpeningOptions>
    <Algorithm>WeightedBrovey</Algorithm>
    <AlgorithmOptions><Weights>0.25,0.25,0.25,0.25</Weights></AlgorithmOptions>
    <Resampling>Cubic</Resampling>
    <NumThreads>1</NumThreads>
    <SpatialExtentAdjustment>None</SpatialExtentAdjustment>
    <SpectralBand dstBand="1"/><SpectralBand dstBand="2"/><SpectralBand dstBand="3"/><SpectralBand dstBand="4"/>
  </PansharpeningOptions>
</VRTDataset>"""
    vrt = gdal.CreatePansharpenedVRT(xml, pan.GetRasterBand(1), [ms.GetRasterBand(k + 1) for k in range(4)])
    got = np.stack([vrt.GetRasterBand(k + 1).ReadAsArray().astype(np.float64) for k in range(4)], axis=-1)
    ours = np.array(want["values"], dtype=np.float64).reshape(PAN.height, PAN.width, 4)
    # PAN cells whose 4 × 4 cubic window lies inside the multispectral raster.
    worst = 0.0
    for j in range(6, 14):
        for i in range(6, 18):
            p = PAN.get(i, j, 0)
            x, y = ro.centre(PAN.affine, i, j)
            u, v = ro.place_in(MS.affine, x, y)
            fs = [math.floor(ro.sample(MS, b, u, v, "cubic") + Fraction(1, 2)) for b in range(4)]
            s = 0.0
            for f in fs:
                s += 0.25 * f
            for b in range(4):
                if ro.stored("u16", fs[b] * float(p) / s) != got[j, i, b]:
                    raise SystemExit(f"GDAL'ın Brovey birleştirmesi ({i}, {j}) hücresinin {b + 1}. bandında farklı.")
                worst = max(worst, abs(ours[j, i, b] - got[j, i, b]))
    if worst > 1.0:
        raise SystemExit(f"GDAL'ın Brovey birleştirmesiyle fark {worst} birim (en çok 1 olmalı).")
    return worst


def main():
    all_cases = cases()
    worst = cross_check(all_cases)
    doc = {
        "format": "kentos.remote-cases",
        "version": 1,
        "note": "ADR 0242'nin araçları, KentOS kodu olmadan ADR'nin kurallarından: girdi rasterleri (yer, boy, bant, tür, değerler; null "
                "değersiz) ve adları, nesneler, aracın ayarları ve vermesi gereken: raster (boy, yer, tür, değersiz değeri, her örnek; "
                "null değersiz) ve notları (tablo, özetin kuyruğu, uyarılar), doğruluğun tablosu, özeti ve sayıları ya da ret. rule "
                "\"exact\": bit bit; \"sum\": sonucun türünde en çok bir birim. Üretici scripts/fixtures/remote_cases.py.",
        "cases": all_cases,
    }
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT} güncel değil: python3 {sys.argv[0]} ile yeniden yazın ve farkı okuyun.")
            sys.exit(1)
        print(f"{OUT.name}: {len(all_cases)} durum; güncel. GDAL'ın Brovey'iyle en büyük fark {worst:g} birim.")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT} yazıldı: {len(all_cases)} durum. GDAL'ın Brovey'iyle en büyük fark {worst:g} birim.")


if __name__ == "__main__":
    main()
