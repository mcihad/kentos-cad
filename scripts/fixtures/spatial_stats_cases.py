#!/usr/bin/env python3
"""Mekânsal istatistik (docs/adr/0238): the core's cases, written from the ADR without KentOS code.

    python3 scripts/fixtures/spatial_stats_cases.py          # fixtures/spatial-stats/v1/cases.json'u yazar
    python3 scripts/fixtures/spatial_stats_cases.py --check  # hiçbir şey yazmaz; karşılaştırır

Every case is a tool's run on objects: what it writes and says. Places are exact (Fraction): a point, a multi-point's mean,
an area's centroid from its rings' moments, a line's middle, a path's middle vertex, a circle's centre. Comparisons are exact:
distances against a band and ε as squares of fractions, the k nearest by (d², order), k-means' assignments on exact means. The
rest is mpmath at 40 digits: square roots, the geometric median (a data place is the median when its resultant is no more
than its weight, else Weiszfeld's steps to 10⁻³⁰), the ellipse's angle, erfc. Texts are rounded from the 40-digit values; a
value within 10⁻⁷ of a rounding boundary (in its last digit) is refused here, as is a comparison too near to call in float64,
so a case never depends on where float64 rounds.

The Rust test (crates/shared/geometry-core/tests/all/spatial_stats.rs) runs every case through the core's calls as the web
makes them: texts exactly, places and lengths within 10⁻⁶ m, statistics within 10⁻⁹ (relative), p within 10⁻¹².
"""

import json
import sys
from fractions import Fraction as F
from functools import cmp_to_key
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
from point_editor_cases import natural_cmp  # noqa: E402
from spatial_query_cases import JS_SPACE, dec_text, read_number  # noqa: E402

mp.mp.dps = 40

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "spatial-stats" / "v1" / "cases.json"
E, N = 487000, 4420000
MARGIN = mp.mpf("1e-7")
MOST_PAIRS = 50_000_000
EMPTY_GROUP = "(boş)"
HOT_COLORS = ["#2166AC", "#67A9CF", "#D1E5F0", "#D9D9D9", "#FDDBC7", "#EF8A62", "#B2182B"]
CLUSTER_COLORS = ["#1F77B4", "#FF7F0E", "#2CA02C", "#D62728", "#9467BD", "#8C564B", "#E377C2", "#17BECF", "#BCBD22", "#7F7F7F"]
NOISE_COLOR = "#BDBDBD"


# ── Numbers ───────────────────────────────────────────────────────────


def M(v):
    """An mpf of a Fraction, an int or an mpf."""
    if isinstance(v, F):
        return mp.mpf(v.numerator) / v.denominator
    return mp.mpf(v)


def num(v):
    """A JSON number: an int when the fraction is one, else the nearest double."""
    if isinstance(v, F) and v.denominator == 1:
        return int(v)
    return float(M(v)) if not isinstance(v, float) else v


def fmt(x, d):
    """`x` at `d` fraction digits; no minus on zero. An exact fraction (a float64 too: dyadic) rounds its halves to even,
    as Rust's formatting does; a 40-digit value near a rounding boundary is refused."""
    if isinstance(x, F):
        scaled = x * 10 ** d
        fl = scaled.numerator // scaled.denominator
        frac = scaled - fl
        if frac == F(1, 2):
            assert dyadic(x), f"{x}: yarım, ama float64'te kesin değil"
            n = fl + (fl % 2)
        else:
            n = fl + (1 if frac > F(1, 2) else 0)
    else:
        x = M(x)
        scaled = x * mp.mpf(10) ** d
        fl = mp.floor(scaled)
        frac = scaled - fl
        if abs(frac - mp.mpf("0.5")) < MARGIN:
            raise AssertionError(f"{x} yuvarlama sınırına çok yakın ({d} basamak)")
        n = int(fl) + (1 if frac > mp.mpf("0.5") else 0)
    s = str(abs(n)).rjust(d + 1, "0")
    body = s if d == 0 else s[:-d] + "." + s[-d:]
    return ("-" + body) if n < 0 else body


def number_of(t):
    """A value read by `kentos.statistics/1`: (Fraction, mantissa, scale) or None."""
    if t is None:
        return None
    d = read_number(t)
    if d is None:
        return None
    m, s = d
    return F(m, 10 ** s), m, s


def p_value(z):
    return mp.erfc(abs(z) / mp.sqrt(2))


def far_from(a, b, what):
    """Two different exact values a float64 rule compares: refused when they are too near to be told apart there."""
    if a != b and abs(M(a) - M(b)) <= mp.mpf("1e-9") * max(abs(M(a)), abs(M(b)), mp.mpf(1)):
        raise AssertionError(f"{what}: {a} ile {b} float64'te ayırt edilemeyecek kadar yakın")


def dyadic(v):
    return v.denominator & (v.denominator - 1) == 0 and v.denominator <= 2 ** 30


# ── Shapes and places ─────────────────────────────────────────────────


def P(x, y):
    return (F(E) + F(x), F(N) + F(y))


def jp(p):
    return {"x": num(p[0]), "y": num(p[1])}


def ring_moments(ring):
    a = mx = my = F(0)
    for i in range(len(ring)):
        (x0, y0), (x1, y1) = ring[i], ring[(i + 1) % len(ring)]
        c = x0 * y1 - x1 * y0
        a += c / 2
        mx += (x0 + x1) * c / 6
        my += (y0 + y1) * c / 6
    return a, mx, my


def pt(x, y, more=()):
    pts = [P(x, y)] + [P(*q) for q in more]
    place = (sum(p[0] for p in pts) / len(pts), sum(p[1] for p in pts) / len(pts))
    e = {"kind": "point", "p": jp(pts[0])}
    if more:
        e["parts"] = [{"p": jp(q)} for q in pts[1:]]
    return e, place


def poly(*xy):
    ring = [P(x, y) for x, y in xy]
    a, mx, my = ring_moments(ring)
    return {"kind": "polygon", "pts": [jp(p) for p in ring]}, (mx / a, my / a)


def line(a, b):
    a, b = P(*a), P(*b)
    return {"kind": "line", "a": jp(a), "b": jp(b)}, ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)


def pline(*xy):
    pts = [P(x, y) for x, y in xy]
    return {"kind": "polyline", "pts": [jp(p) for p in pts]}, pts[len(pts) // 2]


def circ(x, y, r):
    c = P(x, y)
    return {"kind": "circle", "c": jp(c), "r": num(F(r))}, c


def unplaced():
    return {"kind": "polyline", "pts": []}, None


def placed(objs, keep=lambda i: True):
    """(origin, worked places, their input places, unplaced count)."""
    origin, pts, index, missing = None, [], [], 0
    for i, (_, p) in enumerate(objs):
        if p is None:
            missing += 1
            continue
        if not keep(i):
            continue
        if origin is None:
            origin = p
        pts.append((p[0] - origin[0], p[1] - origin[1]))
        index.append(i)
    return origin, pts, index, missing


def d2(a, b):
    return (a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2


def unplaced_note(n):
    return f"{n} nesnenin yeri bulunamadı; alınmadı."


def unread_note(n, what, field):
    return f"{n} nesnenin {what} “{field}” alanından sayı olarak okunamadı; alınmadı."


def warn_if(out, n, text):
    if n:
        out.append(text(n))


def pair(k, v):
    return [k, v]


# ── Centres (§3–§5) ───────────────────────────────────────────────────


def median_of(pts, ws, start):
    """The geometric median, exact to 40 digits."""
    places = {}
    for p, w in zip(pts, ws):
        if w > 0:
            places[p] = places.get(p, F(0)) + w
    for q, wq in places.items():
        rx = ry = mp.mpf(0)
        for p, w in places.items():
            if p == q:
                continue
            d = mp.sqrt(M(d2(p, q)))
            rx += M(w) * M(p[0] - q[0]) / d
            ry += M(w) * M(p[1] - q[1]) / d
        if mp.sqrt(rx * rx + ry * ry) <= M(wq):
            return (M(q[0]), M(q[1]))
    x, y = M(start[0]), M(start[1])
    for _ in range(200000):
        nx = ny = den = mp.mpf(0)
        for p, w in places.items():
            d = mp.sqrt((M(p[0]) - x) ** 2 + (M(p[1]) - y) ** 2)
            nx += M(w) * M(p[0]) / d
            ny += M(w) * M(p[1]) / d
            den += M(w) / d
        tx, ty = nx / den, ny / den
        step = mp.sqrt((tx - x) ** 2 + (ty - y) ** 2)
        x, y = tx, ty
        if step < mp.mpf("1e-30"):
            return (x, y)
    raise AssertionError("ortanca yakınsamadı")


NOUN = {"mean": "ortalama merkezi", "median": "ortanca merkezi", "distance": "standart uzaklığı", "ellipse": "yön dağılımı elipsi"}


def centers(objs, kind, weights=None, groups=None, wfield="", k=1):
    out = {"warnings": [], "infos": []}
    unread = negative = 0
    wt = {}

    def keep(i):
        nonlocal unread, negative
        if weights is None:
            wt[i] = (F(1), None)
            return True
        r = number_of(weights[i])
        if r is None:
            unread += 1
            return False
        if r[1] < 0:
            negative += 1
            return False
        wt[i] = (r[0], (r[1], r[2]))
        return True

    origin, pts, index, missing = placed(objs, keep)
    warn_if(out["warnings"], missing, unplaced_note)
    warn_if(out["warnings"], unread, lambda n: unread_note(n, "ağırlığı", wfield))
    warn_if(out["warnings"], negative, lambda n: f"{n} nesnenin ağırlığı eksi; alınmadı.")
    if not pts:
        return {"refused": "Yeri ve ağırlığı okunan nesne yok." if weights is not None else "Yeri bulunan nesne yok."}
    names, members, empty = [], [], []
    for p, i in zip(pts, index):
        key = None
        if groups is not None and groups[i] is not None:
            key = groups[i].strip(JS_SPACE) or None
        if groups is not None and key is None:
            empty.append((p, i))
            continue
        name = key if groups is not None else ""
        if name not in names:
            names.append(name)
            members.append([])
        members[names.index(name)].append((p, i))
    order = sorted(range(len(names)), key=cmp_to_key(lambda a, b: natural_cmp(names[a], names[b]) or (a > b) - (a < b)))
    seq = [(names[a] if groups is not None else None, members[a]) for a in order]
    if empty:
        seq.append((EMPTY_GROUP, empty))
    objects = []
    weightless = flat = used = 0
    for name, ms in seq:
        ps = [p for p, _ in ms]
        ws = [wt[i][0] for _, i in ms]
        total = sum(ws, F(0))
        if total <= 0:
            weightless += 1
            continue
        mean = (sum((w * p[0] for p, w in zip(ps, ws)), F(0)) / total, sum((w * p[1] for p, w in zip(ps, ws)), F(0)) / total)
        attrs = []
        if name is not None:
            attrs.append(pair("Grup", name))
        attrs.append(pair("Nesne sayısı", str(len(ps))))
        if weights is not None:
            reads = [wt[i][1] for _, i in ms]
            top = max(s for _, s in reads)
            exact = sum((F(m, 10 ** s) for m, s in reads), F(0))
            attrs.append(pair("Ağırlık toplamı", dec_text(int(exact * 10 ** top), top)))
        at = lambda q: [num(M(origin[0]) + M(q[0])), num(M(origin[1]) + M(q[1]))]
        if kind == "mean":
            obj = {"kind": "point", "p": [num(origin[0] + mean[0]), num(origin[1] + mean[1])]}
        elif kind == "median":
            m = median_of(ps, ws, mean)
            obj = {"kind": "point", "p": at(m)}
        elif kind == "distance":
            ss = sum((w * d2(p, mean) for p, w in zip(ps, ws)), F(0)) / total
            if ss == 0:
                flat += 1
                continue
            sd = mp.sqrt(M(ss))
            attrs.append(pair("Standart uzaklık", fmt(sd, 3)))
            attrs.append(pair("Kat", str(k)))
            obj = {"kind": "circle", "c": [num(origin[0] + mean[0]), num(origin[1] + mean[1])], "r": num(k * sd)}
        else:
            sxx = sum((w * (p[0] - mean[0]) ** 2 for p, w in zip(ps, ws)), F(0))
            syy = sum((w * (p[1] - mean[1]) ** 2 for p, w in zip(ps, ws)), F(0))
            sxy = sum((w * (p[0] - mean[0]) * (p[1] - mean[1]) for p, w in zip(ps, ws)), F(0))
            t, half = (sxx + syy) / 2, (sxx - syy) / 2
            h = mp.sqrt(M(half * half + sxy * sxy))
            l1 = (M(t) + h) / M(total)
            det = sxx * syy - sxy * sxy
            if l1 <= 0 or det <= 0:
                flat += 1
                continue
            l2 = M(det) / (M(total) ** 2 * l1)
            a = k * mp.sqrt(2) * mp.sqrt(l1)
            b = k * mp.sqrt(2) * mp.sqrt(l2)
            phi = mp.atan2(M(2 * sxy), M(sxx - syy)) / 2
            bearing = 90 - phi * 180 / mp.pi
            if bearing >= 180:
                bearing -= 180
            attrs.append(pair("Büyük yarı eksen", fmt(a, 3)))
            attrs.append(pair("Küçük yarı eksen", fmt(b, 3)))
            attrs.append(pair("Doğrultu", fmt(bearing, 2)))
            attrs.append(pair("Kat", str(k)))
            obj = {"kind": "ellipse", "c": [num(origin[0] + mean[0]), num(origin[1] + mean[1])],
                   "major": [num(a * mp.cos(phi)), num(a * mp.sin(phi))], "ratio": num(b / a)}
        obj["attrs"] = attrs
        used += len(ps)
        objects.append(obj)
    warn_if(out["warnings"], weightless, lambda n: f"{n} grubun ağırlıklarının toplamı sıfır; yazılmadı.")
    if kind == "ellipse":
        warn_if(out["warnings"], flat, lambda n: f"{n} grubun yerleri bir doğru üzerinde ya da çakışık; elips yazılmadı.")
    else:
        warn_if(out["warnings"], flat, lambda n: f"{n} grubun standart uzaklığı sıfır (tek yer ya da çakışık yerler); yazılmadı.")
    if not objects:
        return {"refused": {"ellipse": "Yazılacak elips yok: yerler bir doğru üzerinde ya da çakışık.",
                            "distance": "Yazılacak daire yok: yerler çakışık ya da tek."}.get(kind, "Yazılacak merkez yok: ağırlıkların toplamı sıfır.")}
    if groups is not None:
        out["summary"] = f"{len(objects)} grubun {NOUN[kind]} yazıldı ({used} nesne)."
    else:
        a = objects[0]["attrs"]
        tail = ""
        if kind == "distance":
            tail = f": {a[-2][1]} m"
        elif kind == "ellipse":
            tail = f": büyük yarı eksen {a[-4][1]} m, küçük {a[-3][1]} m, doğrultu {a[-2][1]}°"
        out["summary"] = f"{used} nesnenin {NOUN[kind]} yazıldı{tail}."
    out["objects"] = objects
    return out


# ── Nearest neighbour (§6) ────────────────────────────────────────────


def nn_squares(pts):
    """Each place's least squared distance to another (exact)."""
    out = []
    for i, p in enumerate(pts):
        out.append(min(d2(p, q) for j, q in enumerate(pts) if j != i))
    return out


def pattern(z, p, clustered_below):
    if p >= mp.mpf("0.05"):
        return "Rastgele", "rastgele"
    if (z < 0) == clustered_below:
        return "Kümelenmiş", "kümelenmiş"
    return "Dağınık", "dağınık"


def check_p(p, d=6):
    """A p near a class boundary (0.01, 0.05, 0.10) would depend on float64."""
    for b in ("0.01", "0.05", "0.10"):
        if abs(p - mp.mpf(b)) < mp.mpf("1e-9"):
            raise AssertionError(f"p {p} sınıf sınırına çok yakın")
    return fmt(p, d)


def nearest(objs, area=None):
    origin, pts, index, missing = placed(objs)
    out = {"warnings": [], "infos": []}
    warn_if(out["warnings"], missing, unplaced_note)
    n = len(pts)
    if n < 2:
        return {"refused": f"En yakın komşu en az iki nesne ister; {n} nesnenin yeri var."}
    if area is None:
        xs, ys = [p[0] for p in pts], [p[1] for p in pts]
        a = (max(xs) - min(xs)) * (max(ys) - min(ys))
    else:
        a = F(area)
    if a <= 0:
        return {"refused": "Yerlerin kutusunun alanı sıfır; Alan'ı yazın."}
    ds = [mp.sqrt(M(s)) for s in nn_squares(pts)]
    observed = mp.fsum(ds) / n
    expected = mp.mpf("0.5") / mp.sqrt(n / M(a))
    ratio = observed / expected
    se = mp.mpf("0.26136") / mp.sqrt(M(F(n * n)) / M(a))
    z = (observed - expected) / se
    p = p_value(z)
    title, word = pattern(z, p, True)
    rows = [["Nesne sayısı", str(n)], ["Gözlenen ortalama uzaklık (m)", fmt(observed, 3)],
            ["Beklenen ortalama uzaklık (m)", fmt(expected, 3)], ["En yakın komşu oranı", fmt(ratio, 4)],
            ["z", fmt(z, 4)], ["p", check_p(p)], ["Alan (m²)", fmt(a, 2)], ["Desen", title]]
    out["table"] = {"columns": ["Ölçü", "Değer"], "rows": rows}
    out["numbers"] = {"ratio": num(ratio), "z": num(z), "p": num(p)}
    out["summary"] = f"En yakın komşu oranı {fmt(ratio, 4)} (z {fmt(z, 4)}, p {check_p(p)}): {word}."
    return out


# ── Neighbourhoods (§7) ───────────────────────────────────────────────


def band_of(pts, band):
    """The band's square (exact) and its length: given, or the largest nearest-neighbour distance."""
    if band is None:
        sq = max(nn_squares(pts))
        return sq, mp.sqrt(M(sq))
    return F(band) ** 2, M(F(band))


def neighbours(pts, concept, band=None, k=8, own=False):
    """Each place's (order, weight) list, by order; the band used; the k used."""
    n = len(pts)
    lists = []
    if concept == "nearest":
        k = min(k, n - 1)
        for i, p in enumerate(pts):
            order = sorted((d2(p, q), j) for j, q in enumerate(pts) if j != i)
            if k < len(order) and order[k - 1][0] != order[k][0]:
                far_from(order[k - 1][0], order[k][0], "k en yakın")
            chosen = [j for _, j in order[:k]]
            if own:
                chosen.append(i)
            lists.append([(j, mp.mpf(1)) for j in sorted(chosen)])
        return lists, None, k
    sq, length = band_of(pts, band)
    pairs = 0
    for i, p in enumerate(pts):
        found = []
        for j, q in enumerate(pts):
            if j == i:
                continue
            s = d2(p, q)
            if s != sq:
                far_from(s, sq, "bant")
            if s <= sq:
                found.append(j)
        pairs += len(found)
        if pairs > MOST_PAIRS:
            raise AssertionError("çok çift")
        if own:
            found.append(i)
        lst = []
        for j in sorted(found):
            if concept == "inverse" and j != i:
                d = mp.sqrt(M(d2(p, pts[j])))
                if d != 1:
                    far_from(d2(p, pts[j]), 1, "1 m")
                lst.append((j, 1 / max(d, mp.mpf(1))))
            else:
                lst.append((j, mp.mpf(1)))
        lists.append(lst)
    return lists, length, k


def described(concept, length, k):
    if concept == "band":
        return f"Sabit uzaklık bandı, {fmt(length, 3)} m"
    if concept == "inverse":
        return f"Ters uzaklık, {fmt(length, 3)} m"
    return f"{k} en yakın komşu"


def valued(objs, values, field, out):
    unread = 0
    xs = {}

    def keep(i):
        nonlocal unread
        r = number_of(values[i])
        if r is None:
            unread += 1
            return False
        xs[i] = r[0]
        return True

    origin, pts, index, missing = placed(objs, keep)
    warn_if(out["warnings"], missing, unplaced_note)
    warn_if(out["warnings"], unread, lambda n: unread_note(n, "değeri", field))
    return pts, index, [xs[i] for i in index]


def morans_i(objs, values, field, concept="band", band=None, k=8, standardize=True):
    out = {"warnings": [], "infos": []}
    pts, index, xs = valued(objs, values, field, out)
    n = len(xs)
    if n < 4:
        return {"refused": f"Moran I en az dört nesne ister; yeri ve değeri olan {n} nesne var."}
    mean = sum(xs, F(0)) / n
    z = [M(x - mean) for x in xs]
    m2 = mp.fsum(v * v for v in z)
    m4 = mp.fsum(v ** 4 for v in z)
    if sum((x - mean) ** 2 for x in xs) == 0:
        return {"refused": "Değerlerin hepsi aynı; Moran I hesaplanamaz."}
    lists, length, k = neighbours(pts, concept, band, k)
    isolated = 0
    rows = [mp.mpf(0)] * n
    for i, lst in enumerate(lists):
        s = mp.fsum(w for _, w in lst)
        if not lst or s <= 0:
            isolated += 1
            lists[i] = []
            continue
        if standardize:
            lists[i] = [(j, w / s) for j, w in lst]
            rows[i] = mp.mpf(1)
        else:
            rows[i] = s
    warn_if(out["warnings"], isolated, lambda m: f"{m} nesnenin komşusu yok; satırı sıfır.")
    weight = [dict(lst) for lst in lists]
    cols = [mp.mpf(0)] * n
    s0 = cross = s1 = mp.mpf(0)
    for i, lst in enumerate(lists):
        lag = mp.mpf(0)
        for j, w in lst:
            s0 += w
            lag += w * z[j]
            cols[j] += w
            back = weight[j].get(i, mp.mpf(0))
            s1 += (w + back) ** 2 / 2 if back > 0 else w * w
        cross += z[i] * lag
    if s0 <= 0:
        return {"refused": "Hiçbir nesnenin komşusu yok; bandı büyütün."}
    s2 = mp.fsum((r + c) ** 2 for r, c in zip(rows, cols))
    nf = mp.mpf(n)
    iv = nf / s0 * cross / m2
    e = -1 / (nf - 1)
    b2 = nf * m4 / (m2 * m2)
    a = nf * ((nf * nf - 3 * nf + 3) * s1 - nf * s2 + 3 * s0 * s0)
    b = b2 * ((nf * nf - nf) * s1 - 2 * nf * s2 + 6 * s0 * s0)
    c = (nf - 1) * (nf - 2) * (nf - 3) * s0 * s0
    var = (a - b) / c - e * e
    if var <= 0:
        return {"refused": "Moran I'nın varyansı hesaplanamadı; komşuluğu değiştirin."}
    zs = (iv - e) / mp.sqrt(var)
    p = p_value(zs)
    title, word = pattern(zs, p, False)
    out["table"] = {"columns": ["Ölçü", "Değer"], "rows": [
        ["Nesne sayısı", str(n)], ["Moran I", fmt(iv, 6)], ["Beklenen I", fmt(e, 6)], ["Varyans", fmt(var, 8)],
        ["z", fmt(zs, 4)], ["p", check_p(p)], ["Desen", title], ["Komşuluk", described(concept, length, k)],
        ["Komşusu olmayan", str(isolated)]]}
    out["numbers"] = {"moransI": num(iv), "z": num(zs), "p": num(p)}
    out["summary"] = f"Moran I {fmt(iv, 6)} (z {fmt(zs, 4)}, p {check_p(p)}): {word}."
    return out


def hot_class(z, p):
    level = 3 if p < mp.mpf("0.01") else 2 if p < mp.mpf("0.05") else 1 if p < mp.mpf("0.10") else 0
    return -level if z < 0 else level


def hot_spots(objs, values, field, concept="band", band=None, k=8):
    out = {"warnings": [], "infos": []}
    pts, index, xs = valued(objs, values, field, out)
    n = len(xs)
    if n < 3:
        return {"refused": f"Sıcak nokta en az üç nesne ister; yeri ve değeri olan {n} nesne var."}
    mean = sum(xs, F(0)) / n
    dev = [x - mean for x in xs]
    if sum(d * d for d in dev) == 0:
        return {"refused": "Değerlerin hepsi aynı; sıcak nokta aranamaz."}
    s = mp.sqrt(M(sum(d * d for d in dev) / n))
    lists, _, _ = neighbours(pts, concept, band, k, own=True)
    counts = [0] * 7
    undefined = 0
    copies = []
    nf = mp.mpf(n)
    for kk, lst in enumerate(lists):
        sw = mp.fsum(w for _, w in lst)
        sw2 = mp.fsum(w * w for _, w in lst)
        numer = mp.fsum(w * M(dev[j]) for j, w in lst)
        inner = (nf * sw2 - sw * sw) / (nf - 1)
        if inner > mp.mpf("1e-20"):
            zz = numer / (s * mp.sqrt(inner))
            p = p_value(zz)
            check_p(p)
            cls = hot_class(zz, p)
            attrs = [pair("z puanı", fmt(zz, 4)), pair("p değeri", check_p(p)), pair("Güven sınıfı", str(cls))]
        else:
            undefined += 1
            cls = 0
            attrs = [pair("z puanı", ""), pair("p değeri", ""), pair("Güven sınıfı", "0")]
        counts[cls + 3] += 1
        copies.append({"index": index[kk], "attrs": attrs, "color": HOT_COLORS[cls + 3]})
    warn_if(out["warnings"], undefined, lambda m: f"{m} nesnenin bütün nesneler komşusu; z'si hesaplanamadı.")
    hot, cold = sum(counts[4:]), sum(counts[:3])
    out["infos"].append(f"Sıcak: %99 {counts[6]}, %95 {counts[5]}, %90 {counts[4]}; soğuk: %99 {counts[0]}, %95 {counts[1]}, %90 {counts[2]}.")
    out["numbers"] = {"hot": hot, "cold": cold}
    out["summary"] = f"{n} nesne yazıldı: {hot} sıcak, {cold} soğuk nokta (%90 ve üstü güvenle)."
    out["copies"] = copies
    return out


# ── Clusters (§10) ────────────────────────────────────────────────────


def cluster_copies(index, labels):
    sizes = [0] * (max(labels, default=0) + 1)
    for l in labels:
        sizes[l] += 1
    copies = []
    for i, l in zip(index, labels):
        size, color = ("", NOISE_COLOR) if l == 0 else (str(sizes[l]), CLUSTER_COLORS[(l - 1) % 10])
        copies.append({"index": i, "attrs": [pair("Küme", str(l)), pair("Küme boyu", size)], "color": color})
    return copies, sizes


def dbscan(objs, radius, min_points=5, border_noise=False):
    origin, pts, index, missing = placed(objs)
    out = {"warnings": [], "infos": []}
    warn_if(out["warnings"], missing, unplaced_note)
    n = len(pts)
    if n == 0:
        return {"refused": "Yeri bulunan nesne yok."}
    sq = F(radius) ** 2
    lists = []
    for p in pts:
        found = []
        for j, q in enumerate(pts):
            s = d2(p, q)
            if s != sq:
                far_from(s, sq, "ε")
            if s <= sq:
                found.append(j)
        lists.append(found)
    core = [len(l) >= min_points for l in lists]
    labels = [None] * n
    nxt = 1
    for i in range(n):
        if labels[i] is not None or not core[i]:
            continue
        c = nxt
        nxt += 1
        labels[i] = c
        queue = [i]
        while queue:
            p = queue.pop(0)
            for q in lists[p]:
                if labels[q] is not None or (border_noise and not core[q]):
                    continue
                labels[q] = c
                if core[q]:
                    queue.append(q)
    labels = [0 if l is None else l for l in labels]
    out["copies"], sizes = cluster_copies(index, labels)
    clusters, noise = len(sizes) - 1, sizes[0]
    out["numbers"] = {"clusters": clusters}
    out["summary"] = f"Küme bulunamadı; {n} nesnenin hepsi gürültü." if clusters == 0 else f"{n} nesne {clusters} kümeye ayrıldı; {noise} nesne gürültü."
    return out


def nearest_center(p, centers):
    ds = [d2(p, c) for c in centers]
    best = min(range(len(ds)), key=lambda c: (ds[c], c))
    for c, d in enumerate(ds):
        if c != best and d != ds[best]:
            far_from(d, ds[best], "k-ortalamalar")
        if c != best and d == ds[best]:
            assert all(dyadic(v) for v in (*p, *centers[c], *centers[best])), "kesin olmayan eşitlik"
    return best


def k_means(objs, k):
    origin, pts, index, missing = placed(objs)
    out = {"warnings": [], "infos": []}
    warn_if(out["warnings"], missing, unplaced_note)
    n = len(pts)
    if n == 0:
        return {"refused": "Yeri bulunan nesne yok."}
    distinct = len(set(pts))
    if distinct < k:
        return {"refused": f"Farklı yer sayısı ({distinct}) küme sayısından ({k}) az."}
    mean = (sum(p[0] for p in pts) / n, sum(p[1] for p in pts) / n)
    first = nearest_center(mean, pts)
    centers = [pts[first]]
    least = [d2(p, pts[first]) for p in pts]
    while len(centers) < k:
        far = max(range(n), key=lambda i: (least[i], -i))
        for i in range(n):
            if i != far and least[i] != least[far]:
                far_from(least[i], least[far], "en uzak")
        c = pts[far]
        centers.append(c)
        least = [min(least[i], d2(p, c)) for i, p in enumerate(pts)]
    labels = [nearest_center(p, centers) for p in pts]
    rounds, settled = 0, False
    while rounds < 500:
        for c in range(k):
            ms = [p for p, l in zip(pts, labels) if l == c]
            if ms:
                centers[c] = (sum(p[0] for p in ms) / len(ms), sum(p[1] for p in ms) / len(ms))
        rounds += 1
        new = [nearest_center(p, centers) for p in pts]
        if new == labels:
            settled = True
            break
        labels = new
    if not settled:
        out["warnings"].append("Atamalar 500 yinelemede durulmadı; son atamalar yazıldı.")
    out["copies"], sizes = cluster_copies(index, [l + 1 for l in labels])
    clusters = sum(1 for s in sizes[1:] if s > 0)
    out["numbers"] = {"clusters": clusters}
    out["summary"] = f"{n} nesne {clusters} kümeye ayrıldı ({rounds} yineleme)."
    return out


# ── The cases ─────────────────────────────────────────────────────────


class Lcg:
    """A small deterministic generator: places on a quarter-metre lattice."""

    def __init__(self, seed):
        self.s = seed

    def next(self):
        self.s = (self.s * 6364136223846793005 + 1442695040888963407) % 2 ** 64
        return self.s >> 33

    def uniform(self, lo, hi):
        """A multiple of 1/4 in [lo, hi)."""
        span = int((hi - lo) * 4)
        return F(lo) + F(self.next() % span, 4)

    def normalish(self, sigma):
        """The sum of four uniforms, scaled: a quarter-metre multiple."""
        s = sum(self.next() % 1000 for _ in range(4)) - 1998
        return F(round(F(s * sigma, 577) * 4), 4)


def cluster(rng, cx, cy, sigma, count):
    return [pt(cx + rng.normalish(sigma), cy + rng.normalish(sigma)) for _ in range(count)]


def lattice(cols, rows, step):
    return [pt(i * step, j * step) for j in range(rows) for i in range(cols)]


def entities(objs):
    return [e for e, _ in objs]


def case(name, tool, objs, params, expect):
    return {"name": name, "tool": tool, "shapes": entities(objs), "params": params, "expect": expect}


def centers_case(name, kind, objs, weights=None, groups=None, wfield="Ağırlık", k=1):
    params = {"kind": kind, "weights": weights, "groups": groups, "weightField": wfield, "k": k}
    return case(name, "centers", objs, params, centers(objs, kind, weights, groups, wfield, k))


def cases():
    out = []
    rng = Lcg(2026)
    ten = cluster(rng, 40, 25, 12, 10)
    mixed = [poly((0, 0), (20, 0), (20, 10), (0, 10)), line((30, 0), (50, 8)), pline((60, 0), (70, 5), (80, 0)),
             circ(10, 40, 3), pt(40, 40, more=[(44, 40), (42, 46)]), unplaced(), pt(25, 25)]
    weights10 = ["2.5", "1,25", " 3 ", "x", None, "-1", "0", "4", "0.75", "2"]
    for kind in ("mean", "median", "distance", "ellipse"):
        out.append(centers_case(f"{kind}-duz", kind, ten))
        out.append(centers_case(f"{kind}-agirlikli", kind, ten, weights=weights10))
        out.append(centers_case(f"{kind}-turler", kind, mixed))
    groups = ["B", "A", "a10", "a9", None, "  A  ", "", "B", "a10", "a9", "A", "B", "a9", "a10", None]
    big = cluster(Lcg(7), 0, 0, 30, 15)
    for kind in ("mean", "median", "distance", "ellipse"):
        out.append(centers_case(f"{kind}-gruplu", kind, big, groups=groups))
    out.append(centers_case("distance-kat-2", "distance", ten, k=2))
    out.append(centers_case("ellipse-kat-3", "ellipse", ten, k=3))
    # One group's weights all 0; one group a single place (no spread); one group on a line.
    gw = ["0", "0", "1", "2", "1", "1", "1", "1", "1", "1"]
    gg = ["S", "S", "T", "T", "T", "U", "U", "U", "V", "V"]
    shaped = [pt(0, 0), pt(5, 5), pt(10, 0), pt(14, 6), pt(19, 1), pt(30, 30), pt(40, 40), pt(50, 50), pt(60, 0), pt(60, 0)]
    for kind in ("mean", "distance", "ellipse"):
        out.append(centers_case(f"{kind}-atlanan-gruplar", kind, shaped, weights=gw, groups=gg))
    # The median on a data place: the angle at the first is over 120°; and a heavy place.
    out.append(centers_case("median-kose", "median", [pt(0, 0), pt(10, 1), pt(-10, 1)]))
    out.append(centers_case("median-agir", "median", [pt(0, 0), pt(30, 0), pt(0, 40), pt(25, 35)], weights=["10", "1", "1", "1"]))
    out.append(centers_case("median-dogru", "median", [pt(0, 0), pt(7, 0), pt(15, 0), pt(40, 0), pt(41, 0)]))
    # The ellipse: axes along x and y (the bearing 0 and 90), a circle, a line.
    out.append(centers_case("ellipse-kuzey", "ellipse", [pt(0, 20), pt(0, -20), pt(5, 0), pt(-5, 0)]))
    out.append(centers_case("ellipse-dogu", "ellipse", [pt(20, 0), pt(-20, 0), pt(0, 5), pt(0, -5)]))
    out.append(centers_case("ellipse-daire", "ellipse", [pt(10, 0), pt(-10, 0), pt(0, 10), pt(0, -10)]))
    out.append(centers_case("ellipse-dogru", "ellipse", [pt(0, 0), pt(10, 10), pt(20, 20)]))
    out.append(centers_case("distance-cakisik", "distance", [pt(5, 5), pt(5, 5)]))
    out.append(centers_case("mean-yersiz", "mean", [unplaced(), unplaced()]))
    out.append(centers_case("mean-agirliksiz", "mean", ten[:3], weights=["x", None, "-2"]))
    out.append(centers_case("mean-sifir-agirlik", "mean", ten[:2], weights=["0", "0"]))

    # Nearest neighbour.
    tight = cluster(Lcg(11), 0, 0, 4, 12) + cluster(Lcg(12), 200, 150, 4, 12)
    out.append(case("nearest-kumeli", "nearest", tight, {"area": None}, nearest(tight)))
    grid = lattice(6, 5, 20)
    out.append(case("nearest-izgara", "nearest", grid, {"area": None}, nearest(grid)))
    out.append(case("nearest-alan", "nearest", grid, {"area": 40000}, nearest(grid, 40000)))
    rnd = [pt(rng.uniform(0, 300), rng.uniform(0, 200)) for _ in range(40)]
    out.append(case("nearest-rastgele", "nearest", rnd, {"area": None}, nearest(rnd)))
    dup = [pt(0, 0), pt(0, 0), pt(30, 10), pt(12, 40), unplaced()]
    out.append(case("nearest-cakisik", "nearest", dup, {"area": None}, nearest(dup)))
    out.append(case("nearest-tek", "nearest", [pt(1, 1), unplaced()], {"area": None}, nearest([pt(1, 1), unplaced()])))
    flat = [pt(0, 0), pt(10, 0), pt(25, 0)]
    out.append(case("nearest-dogru", "nearest", flat, {"area": None}, nearest(flat)))

    # Moran's I: a trend over a lattice, a checkerboard, k nearest, inverse distance, a band that leaves some alone.
    lat = lattice(6, 5, 10)
    trend = [str(F(3 * (i % 6) + 2 * (i // 6)) + F((i * 7) % 5, 4)) for i in range(30)]
    trend = [dec_text(int(F(v) * 100), 2) for v in trend]
    checker = [("10" if (i % 6 + i // 6) % 2 == 0 else "2") for i in range(30)]
    moran = lambda name, objs, values, **kw: case(name, "moran", objs, {"values": values, "field": "Değer", "concept": kw.get("concept", "band"),
                                                                      "band": kw.get("band"), "k": kw.get("k", 8),
                                                                      "standardize": kw.get("standardize", True)},
                                                  morans_i(objs, values, "Değer", kw.get("concept", "band"), kw.get("band"), kw.get("k", 8),
                                                           kw.get("standardize", True)))
    out.append(moran("moran-egilim", lat, trend))
    out.append(moran("moran-dama", lat, checker))
    out.append(moran("moran-bant", lat, trend, band=15))
    out.append(moran("moran-ters", lat, trend, concept="inverse", band=25))
    out.append(moran("moran-komsu", lat, trend, concept="nearest", k=3))
    out.append(moran("moran-ham", lat, trend, standardize=False, band=15))
    lone = lat + [pt(200, 200), pt(201, 200)]
    lone_values = trend + ["40", "41"]
    out.append(moran("moran-yalniz", lone, lone_values, band=11))
    scattered = [pt(rng.uniform(0, 100), rng.uniform(0, 100)) for _ in range(25)] + [pt(50, 50), pt(50, 50)]
    sv = [str(int(rng.next() % 50)) for _ in range(25)] + ["7", "9"]
    out.append(moran("moran-ters-cakisik", scattered, sv, concept="inverse", band=30))
    out.append(moran("moran-okunamayan", lat[:8] + [unplaced()], ["1", "2", "x", None, "5", "3", "8", "2", "4"]))
    out.append(moran("moran-az", lat[:3], ["1", "2", "3"]))
    out.append(moran("moran-ayni", lat[:6], ["4"] * 6))
    out.append(moran("moran-komsusuz", lat[:6], ["1", "2", "3", "4", "5", "6"], band=5))

    # Gi*: a hot and a cold corner over a lattice.
    hl = lattice(8, 6, 10)
    hv = []
    for i in range(48):
        x, y = i % 8, i // 8
        v = 10 + (i * 13) % 7
        if x >= 5 and y >= 3:
            v += 30
        if x <= 2 and y <= 2:
            v -= 8
        hv.append(str(v))
    hot = lambda name, objs, values, **kw: case(name, "hotSpots", objs, {"values": values, "field": "Değer", "concept": kw.get("concept", "band"),
                                                                       "band": kw.get("band"), "k": kw.get("k", 8)},
                                                hot_spots(objs, values, "Değer", kw.get("concept", "band"), kw.get("band"), kw.get("k", 8)))
    out.append(hot("gi-bant", hl, hv))
    out.append(hot("gi-genis-bant", hl, hv, band=25))
    out.append(hot("gi-komsu", hl, hv, concept="nearest", k=6))
    out.append(hot("gi-hepsi-komsu", lat[:5], ["1", "5", "2", "8", "3"], band=1000))
    out.append(hot("gi-az", lat[:2], ["1", "2"]))
    out.append(hot("gi-ayni", lat[:4], ["3"] * 4))

    # DBSCAN: two clusters, a border place both reach, noise; DBSCAN*; all noise; every place a core.
    a = [pt(0, 0), pt(4, 0), pt(0, 4), pt(4, 4), pt(2, 2)]
    b = [pt(20, 0), pt(24, 0), pt(20, 4), pt(24, 4), pt(22, 2)]
    border = [pt(12, 0)]
    noise = [pt(60, 60), pt(-40, 30)]
    scene = a + border + b + noise + [unplaced()]
    db = lambda name, objs, r, m=5, bn=False: case(name, "dbscan", objs, {"radius": num(F(r)), "minPoints": m, "borderNoise": bn}, dbscan(objs, r, m, bn))
    out.append(db("dbscan-iki-kume", scene, F(17, 2), 5))
    out.append(db("dbscan-yildiz", scene, F(17, 2), 5, True))
    out.append(db("dbscan-gurultu", scene, 1, 3))
    out.append(db("dbscan-hepsi-cekirdek", scene, F(1, 2), 1))
    mixed_c = cluster(Lcg(21), 0, 0, 3, 15) + cluster(Lcg(22), 60, 10, 3, 15) + [pt(rng.uniform(-50, 120), rng.uniform(-40, 60)) for _ in range(8)]
    out.append(db("dbscan-karma", mixed_c, F(9, 2), 4))

    # k-means: three groups; ties at equal distances (exact, on the quarter lattice); fewer places than k.
    three = cluster(Lcg(31), 0, 0, 5, 10) + cluster(Lcg(32), 80, 0, 5, 10) + cluster(Lcg(33), 40, 70, 5, 10)
    km = lambda name, objs, k: case(name, "kMeans", objs, {"k": k}, k_means(objs, k))
    out.append(km("kmeans-uc", three, 3))
    out.append(km("kmeans-bes", three, 5))
    spread = [pt(rng.uniform(0, 200), rng.uniform(0, 120)) for _ in range(40)]
    out.append(km("kmeans-rastgele", spread, 4))
    ties = [pt(0, 0), pt(10, 0), pt(5, 0), pt(5, 4), pt(5, -4), pt(0, 0)]
    out.append(km("kmeans-esit", ties, 2))
    out.append(km("kmeans-az", [pt(0, 0), pt(0, 0), pt(1, 1)], 3))
    out.append(km("kmeans-yersiz", [unplaced()], 2))
    return out


def build():
    return {
        "format": "kentos.spatial-stats-cases",
        "version": 1,
        "source": "scripts/fixtures/spatial_stats_cases.py (docs/adr/0238)",
        "tolerance": {"place": 1e-6, "relative": 1e-9, "p": 1e-12},
        "cases": cases(),
    }


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check olmadan çalıştırıp farkı okuyun.")
            return 1
        print(f"{OUT.relative_to(ROOT)}: güncel.")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(json.loads(text)['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
