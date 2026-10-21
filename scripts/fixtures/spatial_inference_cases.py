#!/usr/bin/env python3
"""Permütasyon çıkarımı, çoklu test düzeltmesi ve deney profili (docs/adr/0520): İşlemler's cases of Moran I, Sıcak nokta
(Gi*) and Yerel Moran with permutations, FDR and Bonferroni, written from the ADR without KentOS code and without
packages beyond Python's own (fractions, decimal, hashlib). En yakın komşu's Monte Carlo test and DBSCAN's and k-means'
stability (docs/adr/0566) are spatial_stability_cases.py's, written into the same file.

    python3 scripts/fixtures/spatial_inference_cases.py          # fixtures/processing/v1/spatial-inference.json'u yazar
    python3 scripts/fixtures/spatial_inference_cases.py --check  # hiçbir şey yazmaz; karşılaştırır

The drawing is Mekânsal istatistik's (fixtures/processing/v1/spatial-stats.kcad: Kazalar' points, Parseller' 20 m squares),
read here as it is. Places, distances, weights (the inverse distance's from a 60-digit square root), deviations and every
statistic, observed and permuted, are exact fractions; square roots and erfc are 60-digit decimals. The random numbers are
the rule's (SplitMix64 and xoshiro256** in Python's integers, Lemire's product for an index): the same permutations and
draws the core makes. A permuted statistic that is not equal to the observed one but within 10⁻⁷ of the statistic's bound
of it, a spread within 10⁻⁷ of the bound, a deviation or a lag within 10⁻⁹ of zero, a text within 10⁻⁷ of a rounding
boundary and a corrected analytic p within 10⁻⁹ of a class's threshold are refused here: a case never depends on where
float64 rounds. The pseudo p values and their corrections are float64 operations the ADR spells out, done here in Python's
floats (the same IEEE double operations) and checked against the exact fractions.

The script checks itself against the theory too (`self_checks`): on five values all 120 permutations give the exact
permutation p, which the Monte Carlo p of 9 999 permutations must meet within its binomial error; the mean and the variance
of 9 999 permuted Moran's I must meet E[I] and V[I] under randomisation (Cliff and Ord), the ADR 0238 formulas.
"""

import hashlib
import json
import sys
from decimal import Decimal, localcontext
from fractions import Fraction as F
from itertools import permutations as all_orders
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from spatial_query_cases import read_number  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures" / "processing" / "v1"
DRAWING = DIR / "spatial-stats.kcad"
OUT = DIR / "spatial-inference.json"

RULE = "kentos.spatial-inference/1"
GENERATOR = "SplitMix64 + xoshiro256**"
NEAR = F(1, 10 ** 7)
TIE = 1e-9
DIGITS = 60
HOT_COLORS = ["#2166AC", "#67A9CF", "#D1E5F0", "#D9D9D9", "#FDDBC7", "#EF8A62", "#B2182B"]
LISA = [("YY", "Yüksek-Yüksek", "#D7191C"), ("DD", "Düşük-Düşük", "#2C7BB6"), ("YD", "Yüksek-Düşük", "#FDAE61"),
        ("DY", "Düşük-Yüksek", "#ABD9E9"), ("", "Anlamlı değil", "#D9D9D9"), ("", "Sınanamadı", "#8C8C8C")]
CORRECTIONS = {"none": "Yok", "fdr": "FDR (Benjamini–Hochberg)", "bonferroni": "Bonferroni"}


class Refused(Exception):
    pass


# ── Numbers ───────────────────────────────────────────────────────────


def dec(x):
    with localcontext() as c:
        c.prec = DIGITS
        return Decimal(x.numerator) / Decimal(x.denominator) if isinstance(x, F) else +Decimal(x)


def frac(d):
    return F(d)


def sqrt(x):
    """√x as a fraction of a 60-digit decimal."""
    with localcontext() as c:
        c.prec = DIGITS
        return F(dec(x).sqrt())


def pi():
    """π to 80 digits (the decimal module's recipe)."""
    with localcontext() as c:
        c.prec = 90
        three = Decimal(3)
        lasts, t, s, n, na, d, da = 0, three, 3, 1, 0, 0, 24
        while s != lasts:
            lasts = s
            n, na = n + na, na + 8
            d, da = d + da, da + 32
            t = (t * n) / d
            s += t
        return s


PI = pi()


def erfc(x):
    """erfc(x) ≥ 0 as a fraction: 1 − erf by its series at 300 digits (|x| ≤ 12), else 0 (below 10⁻⁶³)."""
    x = abs(x)
    if x > 12:
        return F(0)
    with localcontext() as c:
        c.prec = 300
        xd = Decimal(x.numerator) / Decimal(x.denominator)
        x2 = xd * xd
        term, total, n = xd, xd, 0
        while True:
            n += 1
            term = -term * x2 / n
            add = term / (2 * n + 1)
            total += add
            if abs(add) < Decimal(10) ** -280:
                break
        erf = 2 / PI.sqrt() * total
        return F(1 - erf)


def p_value(z):
    return erfc(z / sqrt(2))


def near(a, b, bound, what):
    if a != b and abs(a - b) <= NEAR * bound:
        raise Refused(f"{what}: {float(a)} ile {float(b)} float64'te ayırt edilemeyecek kadar yakın")


def fmt(x, d):
    """`x` at `d` fraction digits, no minus on zero; refused within 10⁻⁷ of a rounding boundary (in its last digit).
    A Python float is a value the core computes by the same double operations: its exact binary value is rounded as
    Rust's formatting rounds it (halves to even), with no margin."""
    double = isinstance(x, float)
    x = F(x)
    scaled = x * 10 ** d
    fl = scaled.numerator // scaled.denominator
    rest = scaled - fl
    if double:
        n = fl + (1 if rest > F(1, 2) or (rest == F(1, 2) and fl % 2) else 0)
    else:
        if abs(rest - F(1, 2)) < NEAR:
            raise Refused(f"{float(x)} yuvarlama sınırına çok yakın ({d} basamak)")
        n = fl + (1 if rest > F(1, 2) else 0)
    s = str(abs(n)).rjust(d + 1, "0")
    body = s if d == 0 else s[:-d] + "." + s[-d:]
    return ("-" + body) if n < 0 else body


def value_text(v):
    """A double's shortest text without an exponent (Rust's Display)."""
    if v == 0:
        return "0"
    t = format(Decimal(repr(v)), "f")
    if "." in t:
        t = t.rstrip("0").rstrip(".")
    return t


# ── The rule's random numbers ─────────────────────────────────────────

MASK = (1 << 64) - 1


class Sequence:
    def __init__(self, seed):
        x = seed & MASK
        self.s = []
        for _ in range(4):
            x = (x + 0x9E3779B97F4A7C15) & MASK
            z = x
            z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & MASK
            z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & MASK
            self.s.append(z ^ (z >> 31))

    def next(self):
        s = self.s
        rotl = lambda v, k: ((v << k) | (v >> (64 - k))) & MASK  # noqa: E731
        result = (rotl((s[1] * 5) & MASK, 7) * 9) & MASK
        t = (s[1] << 17) & MASK
        s[2] ^= s[0]
        s[3] ^= s[1]
        s[1] ^= s[2]
        s[0] ^= s[3]
        s[2] ^= t
        s[3] = rotl(s[3], 45)
        return result

    def below(self, m):
        return (self.next() * m) >> 64


def global_permutations(n, r, seq):
    for _ in range(r):
        a = list(range(n))
        for i in range(n - 1, 0, -1):
            j = seq.below(i + 1)
            a[i], a[j] = a[j], a[i]
        yield a


def conditional_rows(n, kmax, r, seq):
    m = n - 1
    kmax = min(kmax, m)
    out = []
    for _ in range(r):
        a = list(range(m))
        row = []
        for t in range(kmax):
            j = t + seq.below(m - t)
            a[t], a[j] = a[j], a[t]
            row.append(a[t])
        out.append(row)
    return out


def folded(larger, r):
    """The pseudo p as the core divides it, and exactly."""
    l = r - larger if r - larger < larger else larger
    return (l + 1) / (r + 1), F(l + 1, r + 1)


def adjust_float(ps, how):
    """The ADR's float64 operations, in Python's doubles."""
    m = float(sum(1 for p in ps if p is not None))
    if how == "none":
        return list(ps)
    if how == "bonferroni":
        return [None if p is None else min(1.0, p * m) for p in ps]
    order = sorted(((p, i) for i, p in enumerate(ps) if p is not None))
    out = [None] * len(ps)
    least = 1.0
    for rank in range(len(order) - 1, -1, -1):
        p, i = order[rank]
        q = p * m / (rank + 1)
        if q < least:
            least = q
        out[i] = least
    return out


def adjust_exact(ps, how):
    m = sum(1 for p in ps if p is not None)
    if how == "none":
        return list(ps)
    if how == "bonferroni":
        return [None if p is None else min(F(1), p * m) for p in ps]
    order = sorted(((p, i) for i, p in enumerate(ps) if p is not None))
    out = [None] * len(ps)
    least = F(1)
    for rank in range(len(order) - 1, -1, -1):
        p, i = order[rank]
        least = min(least, p * m / (rank + 1))
        out[i] = least
    return out


# ── The drawing ───────────────────────────────────────────────────────

DOC = json.loads(DRAWING.read_text("utf-8"))
LAYER_NAMES = {l["id"]: l["name"] for l in DOC["layers"]}


def objects(lid):
    return [e for e in DOC["entities"] if e["layerId"] == lid]


def place(e):
    if e["kind"] == "point":
        pts = [e["p"]] + [q["p"] for q in e.get("parts", [])]
        return (sum(F(p["x"]) for p in pts) / len(pts), sum(F(p["y"]) for p in pts) / len(pts))
    if e["kind"] == "polygon":
        ring = [(F(p["x"]), F(p["y"])) for p in e["pts"]]
        a = mx = my = F(0)
        for i in range(len(ring)):
            (x0, y0), (x1, y1) = ring[i], ring[(i + 1) % len(ring)]
            c = x0 * y1 - x1 * y0
            a += c / 2
            mx += (x0 + x1) * c / 6
            my += (y0 + y1) * c / 6
        return (mx / a, my / a)
    raise AssertionError(e["kind"])


def number_of(t):
    if t is None:
        return None
    d = read_number(t)
    if d is None:
        return None
    m, s = d
    return F(float(F(m, 10 ** s)))


def valued(objs, field):
    """The objects with a value: their input places, places and values; the unread count."""
    idx, pts, xs, unread = [], [], [], 0
    for i, e in enumerate(objs):
        v = number_of(e["attrs"].get(field))
        if v is None:
            unread += 1
            continue
        idx.append(i)
        pts.append(place(e))
        xs.append(v)
    warnings = [f"{unread} nesnenin değeri “{field}” alanından sayı olarak okunamadı; alınmadı."] if unread else []
    return idx, pts, xs, warnings


def d2(p, q):
    return (p[0] - q[0]) ** 2 + (p[1] - q[1]) ** 2


def neighbours(pts, concept, band, k, own):
    """Each place's neighbours (j, weight), sorted by j; the band² used and k."""
    n = len(pts)
    if concept == "nearest":
        k = min(k, n - 1)
        lists = []
        for i in range(n):
            others = sorted((d2(pts[i], pts[j]), j) for j in range(n) if j != i)
            if k < len(others):
                near(others[k - 1][0], others[k][0], max(others[k][0], 1), "k en yakının sınırı")
            chosen = [(j, F(1)) for _, j in others[:k]]
            if own:
                chosen.append((i, F(1)))
            lists.append(sorted(chosen))
        return lists, None, k
    b2 = F(band) ** 2 if band is not None else max(min(d2(pts[i], pts[j]) for j in range(n) if j != i) for i in range(n))
    lists = []
    for i in range(n):
        row = []
        for j in range(n):
            if j == i:
                if own:
                    row.append((i, F(1)))
                continue
            dd = d2(pts[i], pts[j])
            near(dd, b2, max(b2, 1), "bandın sınırı")
            if dd <= b2:
                w = F(1) if concept == "band" else 1 / max(sqrt(dd), F(1))
                row.append((j, w))
        lists.append(row)
    return lists, b2, k


def described(concept, b2, k):
    if concept == "nearest":
        return f"{k} en yakın komşu"
    band = sqrt(b2)
    return f"{'Sabit uzaklık bandı' if concept == 'band' else 'Ters uzaklık'}, {fmt(band, 3)} m"


def moments(sims):
    r = len(sims)
    mean = sum(sims) / r
    var = sum((s - mean) ** 2 for s in sims) / r
    return mean, sqrt(var) if var > 0 else F(0)


def count_larger(sims, observed, bound):
    larger = 0
    for s in sims:
        near(s, observed, bound, "permütasyonun istatistiği gözlenene")
        if s >= observed:
            larger += 1
    return larger


def profile(tool, permutations, seed, correction, hood, standardize, field, idx, pts, xs, scope, conditional, more=()):
    rows = [
        ("Kural", RULE, "rule"),
        ("Araç", tool, "tool"),
        ("Çıkarım", "Normal dağılım" if not permutations else ("Koşullu permütasyon" if conditional else "Permütasyon"), "inference"),
        ("Permütasyon sayısı", str(permutations), "permutations"),
        ("Tohum", str(seed) if permutations else "—", "seed"),
        ("Sayı üreteci", GENERATOR if permutations else "—", "generator"),
        ("Çoklu test düzeltmesi", "Yok (tek sınama)" if tool == "Moran I" else CORRECTIONS[correction], "correction"),
        ("Komşuluk", hood, "weights"),
        ("Satır standartlaştırma", {True: "Evet", False: "Hayır", None: "Uygulanmaz"}[standardize], "standardize"),
        ("Nesne sayısı", str(len(xs)), "objects"),
        ("Değer alanı", field, "field"),
        *more,
    ]
    text = "".join(f"{i}\t{fmt(p[0], 6)}\t{fmt(p[1], 6)}\t{value_text(float(x))}\n" for i, p, x in zip(idx, pts, xs))
    digest = hashlib.sha256(text.encode("utf-8")).hexdigest()
    rows += [("Kapsam", scope, "scope"), ("Girdi özeti (SHA-256)", digest, "inputs")]
    info = (f"Deney profili ({RULE}): tohum {str(seed) if permutations else '—'}, {permutations} permütasyon, düzeltme "
            f"{'Yok (tek sınama)' if tool == 'Moran I' else CORRECTIONS[correction]}; girdi özeti {digest[:12]}….")
    # The table shows the digest's first 16 digits; the experiment's text holds it whole.
    table = {"columns": ["Alan", "Değer"],
             "rows": [[label, value[:16] + "…" if key == "inputs" else value] for label, value, key in rows]}
    # The same by key, a JSON object's text in the profile's order.
    experiment = "{" + ",".join(json.dumps(key) + ":" + json.dumps(value, ensure_ascii=False) for _, value, key in rows) + "}"
    return {"table": table, "experiment": experiment}, info


def basis(permutations, seed, correction):
    s = f"{permutations} koşullu permütasyonun p'sinden (tohum {seed})" if permutations else "normal dağılımın p'sinden"
    if correction != "none":
        s += f", {CORRECTIONS[correction]} düzeltmesiyle"
    return s


def subset_warning(scope_key, count):
    what = {"selection": "seçili nesneler", "visible": "görünen nesneler"}.get(scope_key)
    if scope_key in ("layer", "all"):
        return []
    what = what or "verilen nesneler"
    return [f"Girdi bir alt küme ({what}): istatistik yalnız bu {count} nesnenin; dışarıda kalan komşular hesaba girmedi."]


def scope_label(scope_key, lid):
    return {"layer": f"Katman: {LAYER_NAMES.get(lid, '')} (bütün nesneleri)", "all": "Bütün nesneler",
            "selection": "Seçili nesneler (alt küme)", "visible": "Görünen nesneler (alt küme)"}.get(scope_key, "Verilen nesneler (alt küme)")


# ── Moran I with permutations (§3) ────────────────────────────────────


def moran(objs, field, concept="band", band=None, k=8, standardize=True, permutations=999, seed=1, scope=("layer", None)):
    idx, pts, xs, warnings = valued(objs, field)
    n = len(xs)
    if n < 4:
        return {"refused": f"Moran I en az dört nesne ister; yeri ve değeri olan {n} nesne var."}
    mean = sum(xs) / n
    z = [x - mean for x in xs]
    m2 = sum(v * v for v in z)
    m4 = sum(v ** 4 for v in z)
    lists, b2, kk = neighbours(pts, concept, band, k, False)
    isolated = 0
    rows = [F(0)] * n
    for i, lst in enumerate(lists):
        s = sum(w for _, w in lst)
        if not lst or s <= 0:
            isolated += 1
            lists[i] = []
            continue
        if standardize:
            lists[i] = [(j, w / s) for j, w in lst]
            rows[i] = sum(w for _, w in lists[i])
        else:
            rows[i] = s
    if isolated:
        warnings.append(f"{isolated} nesnenin komşusu yok; satırı sıfır.")
    wmap = [dict(l) for l in lists]
    cols = [F(0)] * n
    s0 = cross = s1 = F(0)
    for i, lst in enumerate(lists):
        lag = F(0)
        for j, w in lst:
            s0 += w
            lag += w * z[j]
            cols[j] += w
            back = wmap[j].get(i, F(0))
            s1 += (w + back) ** 2 / 2 if back > 0 else w * w
        cross += z[i] * lag
    s2 = sum((r + c) ** 2 for r, c in zip(rows, cols))
    i_value = F(n) / s0 * cross / m2
    expected = F(-1, n - 1)
    b2k = n * m4 / (m2 * m2)
    a = n * ((n * n - 3 * n + 3) * s1 - n * s2 + 3 * s0 * s0)
    b = b2k * ((n * n - n) * s1 - 2 * n * s2 + 6 * s0 * s0)
    c = (n - 1) * (n - 2) * (n - 3) * s0 * s0
    variance = (a - b) / c - expected * expected
    zs = (i_value - expected) / sqrt(variance)
    p = p_value(zs)
    hood = described(concept, b2, kk)
    table = [["Nesne sayısı", str(n)], ["Moran I", fmt(i_value, 6)], ["Beklenen I", fmt(expected, 6)], ["Varyans", fmt(variance, 8)],
             ["z", fmt(zs, 4)], ["p", fmt(p, 6)], ["Desen", None], ["Komşuluk", hood], ["Komşusu olmayan", str(isolated)]]
    # The permutations.
    total = sum(abs(w) for l in lists for _, w in l)
    zmax = max(abs(v) for v in z)
    bound = F(n) / s0 * total * zmax * zmax / m2
    seq = Sequence(seed)
    sims = []
    for perm in global_permutations(n, permutations, seq):
        sh = [z[a] for a in perm]
        cr = F(0)
        for i, lst in enumerate(lists):
            cr += sh[i] * sum(w * sh[j] for j, w in lst)
        sims.append(F(n) / s0 * cr / m2)
    smean, sd = moments(sims)
    near(sd, F(0), bound, "permütasyonların yayılımı")
    spread = sd > 0
    if spread:
        larger = count_larger(sims, i_value, bound)
        pf, pe = folded(larger, permutations)
        assert F(pf) - pe == 0 or abs(F(pf) - pe) < F(1, 10 ** 15)
        zsim = (i_value - smean) / sd
        pattern = ("Kümelenmiş", "kümelenmiş") if pf < 0.05 and zsim > 0 else (("Dağınık", "dağınık") if pf < 0.05 else ("Rastgele", "rastgele"))
    else:
        pf = pe = zsim = None
        pattern = ("Rastgele", "rastgele")
        warnings.append("Permütasyonların I'ları yayılmıyor; permütasyon p'si yok, desen rastgele sayıldı.")
    table[6][1] = pattern[0]
    opt = lambda v, d: "" if v is None else fmt(v, d)  # noqa: E731
    table += [["Permütasyon sayısı", str(permutations)], ["Tohum", str(seed)], ["Permütasyon ortalaması", fmt(smean, 6)],
              ["Permütasyon z", opt(zsim, 4)], ["Permütasyon p", opt(pe, 6)],
              ["p'nin standart hatası", opt(None if pe is None else sqrt(pe * (1 - pe) / permutations), 6)]]
    prof, info = profile("Moran I", permutations, seed, "none", hood, standardize, field, idx, pts, xs, scope_label(*scope), False)
    warnings += subset_warning(scope[0], len(objs))
    summary = (f"Moran I {fmt(i_value, 6)} (permütasyon z {opt(zsim, 4)}, p {opt(pe, 6)}; {permutations} permütasyon, tohum {seed}): "
               f"{pattern[1]}.")
    return {"summary": summary, "infos": [info], "warnings": warnings, "table": {"columns": ["Ölçü", "Değer"], "rows": table},
            "profile": prof, "pPermutation": pf}


# ── Local statistics (§4, §6) ─────────────────────────────────────────


def conditional_test(i, others, v, base, factor, vmax, rows):
    """Object i's pseudo p (float, exact) and pseudo z, or None."""
    if not others:
        return None
    observed = base + factor * sum(w * v[j] for j, w in others)
    bound = abs(base) + abs(factor) * sum(abs(w) for _, w in others) * vmax
    sims = []
    for row in rows:
        s = F(0)
        for t, (_, w) in enumerate(others):
            o = row[t]
            o = o + 1 if o >= i else o
            s += w * v[o]
        sims.append(base + factor * s)
    mean, sd = moments(sims)
    near(sd, F(0), bound, f"{i}. nesnenin permütasyonlarının yayılımı")
    if sd == 0:
        return None
    larger = count_larger(sims, observed, bound)
    pf, pe = folded(larger, len(rows))
    return pf, pe, (observed - mean) / sd


def hot_class(z, p):
    level = 3 if p < 0.01 else 2 if p < 0.05 else 1 if p < 0.10 else 0
    return -level if z < 0 else level


def check_threshold(q, what):
    for t in (F(1, 100), F(5, 100), F(10, 100)):
        if q != t and abs(q - t) < F(1, 10 ** 9):
            raise Refused(f"{what}: {float(q)} eşiğe çok yakın")


def hot_spots(objs, field, concept="band", band=None, k=8, permutations=999, seed=1, correction="none", scope=("layer", None)):
    idx, pts, xs, warnings = valued(objs, field)
    n = len(xs)
    if n < 3:
        return {"refused": f"Sıcak nokta en az üç nesne ister; yeri ve değeri olan {n} nesne var."}
    mean = sum(xs) / n
    dev = [x - mean for x in xs]
    s = sqrt(sum(d * d for d in dev) / n)
    lists, b2, kk = neighbours(pts, concept, band, k, True)
    vmax = max(abs(d) for d in dev)
    rows = None
    if permutations:
        kmax = max(len(l) - 1 for l in lists)
        rows = conditional_rows(n, kmax, permutations, Sequence(seed))
    analytic, permuted, undefined = [], [], 0
    for kk_, lst in enumerate(lists):
        sw = sum(w for _, w in lst)
        sw2 = sum(w * w for _, w in lst)
        num = sum(w * dev[j] for j, w in lst)
        inner = (n * sw2 - sw * sw) / (n - 1)
        if inner > 0:
            zz = num / (s * sqrt(inner))
            analytic.append((zz, p_value(zz)))
        else:
            undefined += 1
            analytic.append(None)
        if rows is not None:
            own = next(w for j, w in lst if j == kk_)
            others = [(j, w) for j, w in lst if j != kk_]
            permuted.append(conditional_test(kk_, others, dev, own * dev[kk_], F(1), vmax, rows))
        else:
            permuted.append(None)
    if permutations:
        chosen_f = [None if t is None else t[0] for t in permuted]
        chosen_e = [None if t is None else t[1] for t in permuted]
        finals = adjust_float(chosen_f, correction)
        exact = adjust_exact(chosen_e, correction)
        for a_, b_ in zip(finals, exact):
            assert (a_ is None) == (b_ is None) and (a_ is None or abs(F(a_) - b_) < F(1, 10 ** 14))
        final_text = finals
    else:
        chosen_e = [None if t is None else t[1] for t in analytic]
        exact = adjust_exact(chosen_e, correction)
        for q in exact:
            if q is not None:
                check_threshold(q, "düzeltilmiş p")
        finals = exact
        final_text = exact
    counts = [0] * 7
    copies = []
    untested = 0
    opt = lambda v, d: "" if v is None else fmt(v, d)  # noqa: E731
    for kk_ in range(n):
        sign = analytic[kk_][0] if analytic[kk_] is not None else (permuted[kk_][2] if permuted[kk_] is not None else None)
        cls = hot_class(sign, finals[kk_]) if sign is not None and finals[kk_] is not None else 0
        if permutations and permuted[kk_] is None:
            untested += 1
        attrs = [["z puanı", opt(None if analytic[kk_] is None else analytic[kk_][0], 4)],
                 ["p değeri", opt(None if analytic[kk_] is None else analytic[kk_][1], 6)]]
        if permutations:
            attrs += [["Permütasyon z", opt(None if permuted[kk_] is None else permuted[kk_][2], 4)],
                      ["Permütasyon p", opt(None if permuted[kk_] is None else permuted[kk_][1], 6)]]
        if correction != "none":
            attrs.append(["Düzeltilmiş p", opt(final_text[kk_], 6)])
        attrs.append(["Güven sınıfı", str(cls)])
        counts[cls + 3] += 1
        copies.append({"index": idx[kk_], "attrs": attrs, "color": HOT_COLORS[cls + 3]})
    if undefined:
        warnings.append(f"{undefined} nesnenin bütün nesneler komşusu; z'si hesaplanamadı.")
    if untested:
        warnings.append(f"{untested} nesnenin permütasyon p'si yok (kendisinden başka komşusu yok ya da permütasyonları yayılmıyor); sınıfı 0.")
    hot, cold = counts[4] + counts[5] + counts[6], counts[0] + counts[1] + counts[2]
    infos = [f"Sıcak: %99 {counts[6]}, %95 {counts[5]}, %90 {counts[4]}; soğuk: %99 {counts[0]}, %95 {counts[1]}, %90 {counts[2]}.",
             f"Güven sınıfları {basis(permutations, seed, correction)}."]
    prof, info = profile("Sıcak nokta (Gi*)", permutations, seed, correction, described(concept, b2, kk), None, field, idx, pts, xs,
                         scope_label(*scope), True)
    infos.append(info)
    warnings += subset_warning(scope[0], len(objs))
    return {"summary": f"{n} nesne yazıldı: {hot} sıcak, {cold} soğuk nokta (%90 ve üstü güvenle).", "infos": infos,
            "warnings": warnings, "copies": copies, "profile": prof, "hot": hot, "cold": cold}


def local_moran(objs, field, concept="band", band=None, k=8, standardize=True, permutations=999, seed=1, correction="none",
                alpha="0.05", scope=("layer", None)):
    idx, pts, xs, warnings = valued(objs, field)
    n = len(xs)
    if n < 4:
        return {"refused": f"Yerel Moran en az dört nesne ister; yeri ve değeri olan {n} nesne var."}
    mean = sum(xs) / n
    z = [x - mean for x in xs]
    m2 = sum(v * v for v in z) / n
    lists, b2, kk = neighbours(pts, concept, band, k, False)
    isolated = 0
    for i, lst in enumerate(lists):
        s = sum(w for _, w in lst)
        if not lst or s <= 0:
            isolated += 1
            lists[i] = []
        elif standardize:
            lists[i] = [(j, w / s) for j, w in lst]
    zmax = max(abs(v) for v in z)
    kmax = max(len(l) for l in lists)
    rows = conditional_rows(n, kmax, permutations, Sequence(seed))
    local, tests = [], []
    for i, lst in enumerate(lists):
        lag = sum(w * z[j] for j, w in lst)
        factor = z[i] / m2
        if lst:
            for what, v in (("sapma", z[i]), ("gecikme", lag)):
                if abs(v) < F(1, 10 ** 9) * zmax:
                    raise Refused(f"{i}. nesnenin {what}ı sıfıra çok yakın")
        local.append((factor * lag, lag))
        tests.append(conditional_test(i, lst, z, F(0), factor, zmax, rows))
    finals = adjust_float([None if t is None else t[0] for t in tests], correction)
    exact = adjust_exact([None if t is None else t[1] for t in tests], correction)
    for a_, b_ in zip(finals, exact):
        assert (a_ is None) == (b_ is None) and (a_ is None or abs(F(a_) - b_) < F(1, 10 ** 14))
    a = float(alpha)
    counts = [0] * 6
    copies = []
    opt = lambda v, d: "" if v is None else fmt(v, d)  # noqa: E731
    for i in range(n):
        value, lag = local[i]
        quadrant = None if not lists[i] else {(True, True): 0, (False, False): 1, (True, False): 2, (False, True): 3}[(z[i] >= 0, lag >= 0)]
        if quadrant is not None and finals[i] is not None:
            kind = quadrant if finals[i] < a else 4
        else:
            kind = 5
        counts[kind] += 1
        attrs = [["Yerel Moran I", fmt(value, 6) if quadrant is not None else ""], ["Çeyrek", "" if quadrant is None else LISA[quadrant][0]],
                 ["Permütasyon z", opt(None if tests[i] is None else tests[i][2], 4)],
                 ["Permütasyon p", opt(None if tests[i] is None else tests[i][1], 6)]]
        if correction != "none":
            attrs.append(["Düzeltilmiş p", opt(finals[i], 6)])
        attrs.append(["Küme türü", LISA[kind][1]])
        copies.append({"index": idx[i], "attrs": attrs, "color": LISA[kind][2]})
    if isolated:
        warnings.append(f"{isolated} nesnenin komşusu yok; sınanamadı.")
    if counts[5] - isolated:
        warnings.append(f"{counts[5] - isolated} nesnenin permütasyonları yayılmıyor; sınanamadı.")
    at = fmt(F(alpha), 2)
    infos = [f"Küme türleri p < {at} ile, {basis(permutations, seed, correction)}."]
    prof, info = profile("Yerel Moran", permutations, seed, correction, described(concept, b2, kk), standardize, field, idx, pts, xs,
                         scope_label(*scope), True, more=[("Anlamlılık düzeyi", at, "alpha")])
    infos.append(info)
    warnings += subset_warning(scope[0], len(objs))
    return {"summary": f"{n} nesne yazıldı: {counts[0]} Yüksek-Yüksek, {counts[1]} Düşük-Düşük, {counts[2]} Yüksek-Düşük, "
                       f"{counts[3]} Düşük-Yüksek (p < {at}).",
            "infos": infos, "warnings": warnings, "copies": copies, "profile": prof,
            "counts": {"highHigh": counts[0], "lowLow": counts[1], "highLow": counts[2], "lowHigh": counts[3]}}


# ── Cases ─────────────────────────────────────────────────────────────

LAY = lambda i: {"scope": "layer", "layerId": i}  # noqa: E731
DEFAULTS = {"lengthDecimals": 3, "areaDecimals": 2, "angleUnit": "grad", "plotScale": 1000, "drawingFont": "barlow", "activeLayer": "cizim",
            "measureHeightMm": 2}


def layer_id(name):
    fold = str.maketrans("çğıöşüÇĞİÖŞÜâîû", "cgiosuCGIOSUaiu")
    out, dash = "", False
    for ch in name.translate(fold).lower():
        if "a" <= ch <= "z" or "0" <= ch <= "9":
            out += ch
            dash = False
        elif not dash:
            out += "-"
            dash = True
    return "islem-" + out


def symbols(c, size):
    return {"fill": {"type": "fill", "layers": [{"id": "f", "type": "simpleFill", "color": c},
                                                 {"id": "o", "type": "simpleLine", "color": "#FFFFFF", "width": 0.13, "unit": "mm"}]},
            "line": {"type": "line", "layers": [{"id": "l", "type": "simpleLine", "color": c, "width": 0.6, "unit": "mm", "cap": "round", "join": "round"}]},
            "marker": {"type": "marker", "layers": [{"id": "p", "type": "shape", "shape": "circle", "size": size, "unit": "px", "fill": c,
                                                     "stroke": "#FFFFFF", "strokeWidth": 0.8}]}}


HOT_RENDERER = {"type": "categorized", "expr": "[Güven sınıfı]", "categories": [
    {"value": v, "label": label, "symbols": symbols(c, 8)} for v, label, c in [
        ("3", "Sıcak nokta, %99 güven", "#B2182B"), ("2", "Sıcak nokta, %95 güven", "#EF8A62"), ("1", "Sıcak nokta, %90 güven", "#FDDBC7"),
        ("0", "Anlamlı değil", "#D9D9D9"), ("-1", "Soğuk nokta, %90 güven", "#D1E5F0"), ("-2", "Soğuk nokta, %95 güven", "#67A9CF"),
        ("-3", "Soğuk nokta, %99 güven", "#2166AC")]]}
LISA_RENDERER = {"type": "categorized", "expr": "[Küme türü]", "categories": [
    {"value": name, "label": name, "symbols": symbols(c, 8)} for _, name, c in LISA]}


def new_layer(name, color, renderer):
    return {"id": layer_id(name), "name": name, "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25, "fill": color + "26",
                                                          "renderer": renderer}}


def log_of(run):
    return [{"level": "info", "text": t} for t in run["infos"]] + [{"level": "warn", "text": t} for t in run["warnings"]]


def added(run, lid, lname):
    objs = objects(lid)
    out = []
    for c in run["copies"]:
        src = objs[c["index"]]
        attrs = dict(src["attrs"])
        for k, v in c["attrs"]:
            attrs[k] = v
        e = {k: v for k, v in src.items() if k not in ("id", "layerId", "attrs")}
        e.update({"layerId": layer_id(lname), "attrs": attrs, "color": c["color"]})
        out.append(e)
    return out


def case(cid, title, tool, values, run, lid, selection=None):
    if "refused" in run:
        exp = {"status": "error", "message": run["refused"]}
    else:
        exp = {"status": "ok", "summary": run["summary"], "undo": None, "log": log_of(run), "outputs": {"profile": run["profile"]["table"],
                                                                                           "experiment": run["profile"]["experiment"]}}
        if tool == "stats.moransI":
            exp["outputs"]["table"] = run["table"]
            if run["pPermutation"] is not None:
                exp["outputs"]["pPermutation"] = run["pPermutation"]
        else:
            lname, color, renderer, undo = (("Sıcak noktalar", "#E03131", HOT_RENDERER, "Sıcak nokta (Gi*)") if tool == "stats.hotSpot"
                                            else ("Yerel Moran", "#D7191C", LISA_RENDERER, "Yerel Moran (LISA)"))
            exp["undo"] = undo
            exp["layers"] = [new_layer(lname, color, renderer)]
            exp["layerAbove"] = {layer_id(lname): lid}
            exp["added"] = added(run, lid, lname)
            exp["outputs"]["count"] = len(exp["added"])
            if tool == "stats.hotSpot":
                exp["outputs"]["hot"], exp["outputs"]["cold"] = run["hot"], run["cold"]
            else:
                exp["outputs"].update(run["counts"])
    c = {"id": cid, "title": title, "document": "spatial-stats.kcad", "run": {"tool": tool}, "values": values, "expect": exp}
    if selection is not None:
        c["selection"] = selection
    return c


def ids(lid):
    return [e["id"] for e in DOC["entities"] if e["layerId"] == lid]


def cases():
    out = []
    kaza, parsel = objects("kaza"), objects("parsel")
    perm = {"inference": "permutation"}
    out.append(case("moran-perm", "Moran I, 999 permütasyon, tohum 1: permütasyon p'si 0,001, tablo ve deney profili", "stats.moransI",
                    {"input": LAY("parsel"), "valueField": "Değer", **perm}, moran(parsel, "Değer", scope=("layer", "parsel")), "parsel"))
    out.append(case("moran-perm-seed", "Moran I, 99 permütasyon, tohum 7, 4 en yakın komşu, standartlaştırmasız", "stats.moransI",
                    {"input": LAY("parsel"), "valueField": "Değer", "concept": "nearest", "neighbors": 4, "standardize": False, **perm,
                     "permutations": 99, "seed": 7},
                    moran(parsel, "Değer", "nearest", None, 4, False, 99, 7, ("layer", "parsel")), "parsel"))
    out.append(case("moran-perm-kaza", "Moran I, kazaların ağırlıkları, ters uzaklık, 199 permütasyon: eşit değerlerin bağları", "stats.moransI",
                    {"input": LAY("kaza"), "valueField": "Ağırlık", "concept": "inverse", **perm, "permutations": 199, "seed": 3},
                    moran(kaza, "Ağırlık", "inverse", None, 8, True, 199, 3, ("layer", "kaza")), "kaza"))
    out.append(case("hot-perm", "Sıcak nokta, 999 koşullu permütasyon: permütasyon z ve p, sınıf permütasyondan", "stats.hotSpot",
                    {"input": LAY("parsel"), "valueField": "Değer", **perm},
                    hot_spots(parsel, "Değer", scope=("layer", "parsel")), "parsel"))
    out.append(case("hot-fdr", "Sıcak nokta, 999 koşullu permütasyon ve FDR: düzeltilmiş p, sınıf ondan", "stats.hotSpot",
                    {"input": LAY("parsel"), "valueField": "Değer", **perm, "correction": "fdr"},
                    hot_spots(parsel, "Değer", correction="fdr", scope=("layer", "parsel")), "parsel"))
    out.append(case("hot-bonferroni", "Sıcak nokta, normal dağılım ve Bonferroni: profil tohumsuz", "stats.hotSpot",
                    {"input": LAY("parsel"), "valueField": "Değer", "correction": "bonferroni"},
                    hot_spots(parsel, "Değer", permutations=0, correction="bonferroni", scope=("layer", "parsel")), "parsel"))
    out.append(case("hot-kaza-fdr", "Sıcak nokta, kazaların ağırlıkları, 5 en yakın komşu, 499 permütasyon, tohum 11, FDR", "stats.hotSpot",
                    {"input": LAY("kaza"), "valueField": "Ağırlık", "concept": "nearest", "neighbors": 5, **perm, "permutations": 499,
                     "seed": 11, "correction": "fdr"},
                    hot_spots(kaza, "Ağırlık", "nearest", None, 5, 499, 11, "fdr", ("layer", "kaza")), "kaza"))
    out.append(case("lisa-band", "Yerel Moran, kendiliğinden bant, 999 permütasyon: küme türleri ve aykırılar", "stats.localMoran",
                    {"input": LAY("parsel"), "valueField": "Değer"},
                    local_moran(parsel, "Değer", scope=("layer", "parsel")), "parsel"))
    out.append(case("lisa-fdr", "Yerel Moran, 4 en yakın komşu, FDR, %10", "stats.localMoran",
                    {"input": LAY("parsel"), "valueField": "Değer", "concept": "nearest", "neighbors": 4, "correction": "fdr",
                     "significance": "0.10"},
                    local_moran(parsel, "Değer", "nearest", None, 4, True, 999, 1, "fdr", "0.10", ("layer", "parsel")), "parsel"))
    out.append(case("lisa-kaza", "Yerel Moran, kazaların ağırlıkları, standartlaştırmasız, Bonferroni, 499 permütasyon, tohum 5",
                    "stats.localMoran",
                    {"input": LAY("kaza"), "valueField": "Ağırlık", "standardize": False, "correction": "bonferroni", "permutations": 499,
                     "seed": 5},
                    local_moran(kaza, "Ağırlık", "band", None, 8, False, 499, 5, "bonferroni", "0.05", ("layer", "kaza")), "kaza"))
    sel = ids("parsel")[:18]
    out.append(case("lisa-subset", "Yerel Moran, seçili 18 parsel: alt küme diye söylenir, profilde Kapsam", "stats.localMoran",
                    {"input": {"scope": "selection"}, "valueField": "Değer"},
                    local_moran(parsel[:18], "Değer", scope=("selection", None)), "parsel", sel))
    out.append(case("lisa-few", "Yerel Moran, üç parsel: ret", "stats.localMoran", {"input": {"scope": "selection"}, "valueField": "Değer"},
                    local_moran(parsel[:3], "Değer", scope=("selection", None)), "parsel", ids("parsel")[:3]))
    out.append({"id": "lisa-few-permutations", "title": "Yerel Moran, 5 permütasyon: alan en az 9 ister", "document": "spatial-stats.kcad",
                "run": {"tool": "stats.localMoran"}, "values": {"input": LAY("parsel"), "valueField": "Değer", "permutations": 5},
                "expect": {"status": "invalid", "issues": [{"param": "permutations", "message": "“Permütasyon sayısı” en az 9 olmalı."}]}})
    # En yakın komşu's Monte Carlo test and the clusters' stability (docs/adr/0566).
    import spatial_stability_cases
    out += spatial_stability_cases.cases(stability_case)
    # The study area and DBSCAN's parameter sweep (docs/adr/0574).
    import spatial_study_cases
    out += spatial_study_cases.cases(study_case)
    return out


def stability_case(cid, title, tool, values, run, selection=None):
    """A case of docs/adr/0566 (spatial_stability_cases.py): En yakın komşu's simulation, the clusters' stability."""
    if "invalid" in run:
        exp = {"status": "invalid", "issues": run["invalid"]}
    elif tool == "stats.nearestNeighbor":
        exp = {"status": "ok", "summary": run["summary"], "undo": None, "log": log_of(run),
               "outputs": {"table": run["table"], "profile": run["profile"]["table"], "experiment": run["profile"]["experiment"],
                           "pSimulation": run["pSimulation"]}}
    else:
        # The clusters' layer as ADR 0238's cases write it (spatial_stats_processing_cases.py).
        lname, undo, of = (("Kümeler (DBSCAN)", "DBSCAN kümeleme", "dbscan") if tool == "stats.dbscan"
                           else ("Kümeler (k-ortalamalar)", "k-ortalamalar kümeleme", "kmeans"))
        stats = json.loads((DIR / "spatial-stats.json").read_text("utf-8"))
        layer = next(c for c in stats["cases"] if c["id"] == of)["expect"]["layers"][0]
        added_ = added(run, "kaza", lname)
        exp = {"status": "ok", "summary": run["summary"], "undo": undo, "log": log_of(run),
               "layers": [layer], "layerAbove": {layer_id(lname): "kaza"}, "added": added_,
               "outputs": {"count": len(added_), "table": run["table"], "profile": run["profile"]["table"],
                           "experiment": run["profile"]["experiment"], "clusters": run["clusters"], "stable": run["stable"]}}
    c = {"id": cid, "title": title, "document": "spatial-stats.kcad", "run": {"tool": tool}, "values": values, "expect": exp}
    if selection is not None:
        c["selection"] = selection
    return c


def study_case(cid, title, tool, values, run, selection=None):
    """A case of docs/adr/0574 (spatial_study_cases.py): En yakın komşu over a study area, DBSCAN's sweep."""
    if "invalid" in run:
        exp = {"status": "invalid", "issues": run["invalid"]}
    elif "refused" in run:
        exp = {"status": "error", "message": run["refused"]}
    else:
        exp = {"status": "ok", "summary": run["summary"], "undo": None, "log": log_of(run), "outputs": {"table": run["table"]}}
        if tool == "stats.dbscanSweep":
            for key in ("cells", "suggestedRadius", "suggestedMinPoints", "kneeRadius", "clusters", "noiseShare"):
                exp["outputs"][key] = run[key]
        if "profile" in run:
            exp["outputs"]["profile"] = run["profile"]["table"]
            exp["outputs"]["experiment"] = run["profile"]["experiment"]
        if "pSimulation" in run:
            exp["outputs"]["pSimulation"] = run["pSimulation"]
    c = {"id": cid, "title": title, "document": "spatial-stats.kcad", "run": {"tool": tool}, "values": values, "expect": exp}
    if selection is not None:
        c["selection"] = selection
    return c


def defaults():
    return {"stats.localMoran": {"input": LAY("cizim"), "valueField": "", "concept": "band", "band": None, "neighbors": 8, "standardize": True,
                                 "permutations": 999, "seed": 1, "correction": "none", "significance": "0.05",
                                 "layer": {"newName": "Yerel Moran"}}}


def build():
    return {"format": "kentos.processing-cases", "version": 1, "tolerance": 1e-6,
            "documents": {"spatial-stats.kcad": {"defaults": DEFAULTS, "tools": defaults()}}, "cases": cases()}


# ── The reference checked against the theory ──────────────────────────


def self_checks():
    # Five values on a line, their neighbours in a row: the exact permutation p of all 120 orders against 9 999 draws.
    pts = [(F(i), F(0)) for i in range(5)]
    xs = [F(v) for v in (3, 9, 4, 12, 7)]
    lists, _, _ = neighbours(pts, "band", 1, 8, False)
    mean = sum(xs) / 5
    z = [x - mean for x in xs]

    def stat(vals):
        return sum(vals[i] * sum(w * vals[j] for j, w in l) for i, l in enumerate(lists))

    observed = stat(z)
    every = [stat([z[a] for a in order]) for order in all_orders(range(5))]
    exact_larger = sum(1 for s in every if s >= observed)
    exact = F(exact_larger, len(every))
    sims = [stat([z[a] for a in perm]) for perm in global_permutations(5, 9999, Sequence(1))]
    share = F(sum(1 for s in sims if s >= observed), len(sims))
    error = (exact * (1 - exact) / 9999) ** 0.5
    assert abs(float(share - exact)) < 5 * float(error) + 1e-12, (float(share), float(exact))
    # Moran's I of the parcels: the permutations' mean and variance meet E[I] and V[I] (ADR 0238 §8).
    run = moran(objects("parsel"), "Değer", permutations=9999, seed=2)
    rows = dict(run["table"]["rows"])
    e_i, v_i = float(rows["Beklenen I"]), float(rows["Varyans"])
    m = float(rows["Permütasyon ortalaması"])
    assert abs(m - e_i) < 5 * (v_i / 9999) ** 0.5, (m, e_i)
    # The rule's first word from seed 0: SplitMix64's published 0xE220A8397B1DCDAF spread into xoshiro256**.
    assert Sequence(0).s[0] == 0xE220A8397B1DCDAF
    assert Sequence(0).next() == 0x99EC5F36CB75F2B4


def main():
    self_checks()
    import spatial_stability_cases
    spatial_stability_cases.self_checks()
    import spatial_study_cases
    spatial_study_cases.self_checks()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check olmadan çalıştırıp farkı okuyun.")
            return 1
        print(f"{OUT.relative_to(ROOT)}: güncel.")
        return 0
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
