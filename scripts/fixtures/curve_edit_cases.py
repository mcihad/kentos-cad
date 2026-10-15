#!/usr/bin/env python3
"""Eğriyi düzenle: shared cases of cad.curves.edit (docs/adr/0303), from the ADR's rules, without KentOS code.

    python3 scripts/fixtures/curve_edit_cases.py           # writes the file
    python3 scripts/fixtures/curve_edit_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.curves.edit.json. The curves the command writes are computed here twice:

- an IEEE double twin of the ADR's constructions, every operation in the ADR's order: the curve through points
  (nurbs_cases.py's twin, ADR 0259 §3), the curve through points with end tangents (Piegl and Tiller 9.2.2: each
  constrained end's parameter repeated, its derivative row −f, f with f = p / (u[p+1] − u[1]) at the start and
  p / (u[m−1+p] − u[m−1]) at the end, the right side the unit direction times the chord length of the whole, dense
  elimination with partial pivoting, the first largest), the clamped and the wrapped control curve, Boehm's knot
  insertion in homogeneous coordinates (The NURBS Book's A5.1) and the tessellation (each span between distinct
  knots halved, at least twice from degree two, while its middle and quarter points are farther than the tolerance
  from the chord, 18 times at most; de Boor relative to the span's first control point): its numbers are the
  cases' expected numbers, bit for bit;
- in 50-digit arithmetic (mpmath): the control points of every curve through points (and with tangents) solved
  again; an inserted knot's curve evaluated against the curve it came from at 41 parameters; every tessellation
  point evaluated with the basis functions (A2.2) at its parameter, and each chord sampled at 15 parameters within
  the tolerance (and a hair) of the curve.

The rules, as ADR 0303 gives them:
- a polyline: Kapat (needs 3 vertices; the closing edge's bulge is the last of `bulges`), Aç (the closing edge's
  bulge and width become 0), Düzleştir (needs an arc edge; the bulges go), Eğriye çevir (degree 3 when absent,
  1 to 11; through its vertices with them as fit points, or with them as control points, its degree lowered to one
  less than them; its widths go, said as widths_lost); a multi-part one is refused;
- a NURBS curve with fit points: add, move or remove a fit point, or set the end tangents, and the curve is built
  through them again (an open one with its tangents kept, a closed one without); Kapat and Aç build it again closed
  or open through them (Aç keeps none: a closed curve has no tangents);
- every NURBS curve: move a control point or set its weight (a closed curve's wrapped copy too; the fit points and
  tangents go, all weights 1 leaves none), remove one (the curve built again from the rest, clamped and uniform or
  wrapped; at least degree + 1, a closed one max(degree, 3)), insert a knot inside the domain where it is there
  fewer than degree times (open only), Kapat and Aç of a control curve (its drawn control points wrapped or
  clamped), Çoklu çizgiye çevir (a closed curve's last point, its first again, left out);
- the checks in order: the id, the edit's own numbers (finite; a weight and a tolerance above 0), the revision,
  the object (found, not locked), its kind, the edit.
"""
import copy
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))

from mpmath import lu_solve, matrix, mp, mpf, sqrt  # noqa: E402
from nurbs_cases import basis, check_exact, find_span, twin_closed, twin_open  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.curves.edit.json"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
U = lambda i: f"$uidOf:{i}"  # noqa: E731
LABEL = "Eğriyi düzenle"
MAX_DEGREE = 11
DEPTH = 18


def num(v):
    v = float(v)
    return int(v) if v.is_integer() else v


def p(x, y):
    return {"x": num(x), "y": num(y)}


def tup(q):
    return (float(q["x"]), float(q["y"]))


# ── The double twin ────────────────────────────────────────────────────────────────────────────────────────────────


def fit_degree(asked, n):
    return max(1, min(asked, max(n - 1, 0)))


def closed_fit(pts):
    return pts[:-1] if len(pts) > 1 and pts[0] == pts[-1] else pts


class Refused(Exception):
    def __init__(self, code, message, path):
        super().__init__(message)
        self.code, self.message, self.path = code, message, path


def through(pts, degree, closed, path):
    """The curve through `pts` (ADR 0259 §3) checked against its 50-digit solution: (p, ctrl, knots, fit)."""
    q = closed_fit(pts) if closed else pts
    n = len(q)
    edges = 0 if n < 2 else (n if closed else n - 1)
    for i in range(edges):
        if q[i] == q[(i + 1) % n]:
            raise Refused("coincident_points", f"{i + 1}. ve {i + 2}. nokta aynı yerde; eğri her noktadan bir kez geçer. "
                          "Noktalardan birini taşıyın ya da çıkarın.", path)
    if not 1 <= degree <= MAX_DEGREE:
        raise Refused("invalid_nurbs", f"Derece {degree}; 1 ile {MAX_DEGREE} arasında olmalı.", path)
    least = 3 if closed else 2
    if n < least:
        raise Refused("invalid_nurbs", f"{'Kapalı' if closed else 'Açık'} eğri en az {least} geçiş noktasından geçer; {n} verildi.", path)
    k = fit_degree(degree, n)
    ctrl, knots, _ = twin_closed(q, k) if closed else twin_open(q, k)
    check_exact(q, k, closed, ctrl, knots)
    return k, ctrl, knots, q


def tangent_system(q, degree, s, e, zero, one, root):
    """The rows and right sides of the curve through `q` with end tangents (unit `s`, `e`), in the twin's order."""
    n = len(q)
    d = []
    for k in range(1, n):
        dx, dy = q[k][0] - q[k - 1][0], q[k][1] - q[k - 1][1]
        d.append(root(dx * dx + dy * dy))
    total = zero
    for x in d:
        total += x
    t = [zero] * n
    for k in range(1, n - 1):
        t[k] = t[k - 1] + d[k - 1] / total
    t[n - 1] = one
    ext = [t[0]] + ([t[0]] if s else []) + t[1 : n - 1] + ([t[n - 1]] if e else []) + [t[n - 1]]
    m = len(ext)
    pp = fit_degree(degree, m)
    knots = [zero] * (pp + 1)
    for j in range(1, m - pp):
        acc = zero
        for x in ext[j : j + pp]:
            acc += x
        knots.append(acc / pp)
    knots += [one] * (pp + 1)
    rows, bx, by = [], [], []

    def point_row(k):
        row = [zero] * m
        span = find_span(m - 1, pp, t[k], knots)
        for r, v in enumerate(basis(span, t[k], pp, knots, zero, one)):
            row[span - pp + r] = v
        rows.append(row)
        bx.append(q[k][0])
        by.append(q[k][1])

    point_row(0)
    if s:
        f = pp / (knots[pp + 1] - knots[1])
        row = [zero] * m
        row[0], row[1] = -f, f
        rows.append(row)
        bx.append(s[0] * total)
        by.append(s[1] * total)
    for k in range(1, n - 1):
        point_row(k)
    if e:
        f = pp / (knots[m - 1 + pp] - knots[m - 1])
        row = [zero] * m
        row[m - 2], row[m - 1] = -f, f
        rows.append(row)
        bx.append(e[0] * total)
        by.append(e[1] * total)
    point_row(n - 1)
    return pp, knots, rows, bx, by


def unit(v):
    dx, dy = v
    length = math.sqrt(dx * dx + dy * dy)
    if not (length > 0 and math.isfinite(length)):
        raise Refused("invalid_nurbs", "Teğetin doğrultusu yok (sıfır uzunlukta); bir doğrultu verin ya da teğeti kaldırın.", "edit")
    return (dx / length, dy / length)


def through_tangents(q, degree, start, end, path):
    """The open curve through `q` with its end tangents where given (`start`, `end`: raw directions or None)."""
    if start is None and end is None:
        return through(q, degree, False, path)
    through(q, degree, False, path)
    s = unit(start) if start is not None else None
    e = unit(end) if end is not None else None
    pp, knots, a, bx, by = tangent_system(q, degree, s, e, 0.0, 1.0, math.sqrt)
    m = len(a)
    for k in range(m):
        best = k
        for i in range(k + 1, m):
            if abs(a[i][k]) > abs(a[best][k]):
                best = i
        if best != k:
            a[k], a[best] = a[best], a[k]
            bx[k], bx[best] = bx[best], bx[k]
            by[k], by[best] = by[best], by[k]
        piv = a[k][k]
        for i in range(k + 1, m):
            f = a[i][k] / piv
            for j in range(k, m):
                a[i][j] -= f * a[k][j]
            bx[i] -= f * bx[k]
            by[i] -= f * by[k]
    x, y = [0.0] * m, [0.0] * m
    for i in reversed(range(m)):
        sx, sy = bx[i], by[i]
        for j in range(i + 1, m):
            sx -= a[i][j] * x[j]
            sy -= a[i][j] * y[j]
        x[i] = sx / a[i][i]
        y[i] = sy / a[i][i]
    ctrl = [(x[i], y[i]) for i in range(m)]
    # The same system in 50 digits.
    mp.dps = 50
    qm = [(mpf(u), mpf(v)) for u, v in q]
    sm = (mpf(s[0]), mpf(s[1])) if s else None
    em = (mpf(e[0]), mpf(e[1])) if e else None
    _, wknots, rows, wbx, wby = tangent_system(qm, degree, sm, em, mpf(0), mpf(1), sqrt)
    am = matrix(rows)
    xs, ys = lu_solve(am, matrix(wbx)), lu_solve(am, matrix(wby))
    scale = max(1.0, max(max(abs(u), abs(v)) for u, v in q))
    for i in range(m):
        assert max(abs(mpf(ctrl[i][0]) - xs[i]), abs(mpf(ctrl[i][1]) - ys[i])) <= 1e-11 * scale, f"teğetli eğri, nokta {i}"
    for i, k in enumerate(wknots):
        assert abs(mpf(knots[i]) - k) <= 1e-15, f"teğetli eğri, düğüm {i}"
    return pp, ctrl, knots, q


def clamped(pp, n):
    spans = n - pp
    return [0.0] * (pp + 1) + [float(i) for i in range(1, spans)] + [float(spans)] * (pp + 1)


def by_control(pp, ctrl, weights, closed, path):
    n = len(ctrl)
    if closed:
        if n < 3 or n < pp:
            raise Refused("invalid_nurbs", f"{pp}. dereceden kapalı eğrinin en az {max(pp, 3)} denetim noktası olur; {n} verildi.", path)
        return (pp, [ctrl[i % n] for i in range(n + pp)], [float(i) for i in range(n + 2 * pp + 1)],
                [weights[i % n] for i in range(n + pp)] if weights is not None else None)
    if n < pp + 1:
        raise Refused("invalid_nurbs", f"{pp}. dereceden eğrinin en az {pp + 1} denetim noktası olur; {n} verildi.", path)
    return pp, list(ctrl), clamped(pp, n), list(weights) if weights is not None else None


def span_of(pp, ctrl, k, u):
    """The knot span holding `u`, the domain's ends taken inside (the curve's own rule)."""
    n = len(ctrl) - 1
    if u >= k[n + 1]:
        i = n
        while i > pp and k[i] >= k[n + 1]:
            i -= 1
        return i
    if u <= k[pp]:
        i = pp
        while i < n and k[i + 1] <= k[pp]:
            i += 1
        return i
    lo, hi = pp, n + 1
    while hi - lo > 1:
        mid = (lo + hi) // 2
        if u < k[mid]:
            hi = mid
        else:
            lo = mid
    return lo


def point(c, u):
    """De Boor in homogeneous coordinates, relative to the span's first control point."""
    pp, ctrl, k, w = c
    lo, hi = k[pp], k[len(ctrl)]
    u = min(max(u, lo), hi)
    s = span_of(pp, ctrl, k, u)
    o = ctrl[s - pp]
    d = []
    for j in range(pp + 1):
        i = s - pp + j
        wi = w[i] if w is not None else 1.0
        d.append([(ctrl[i][0] - o[0]) * wi, (ctrl[i][1] - o[1]) * wi, wi])
    for r in range(1, pp + 1):
        for j in range(pp, r - 1, -1):
            i = s - pp + j
            den = k[i + pp + 1 - r] - k[i]
            a = 0.0 if den == 0.0 else (u - k[i]) / den
            prev = d[j - 1]
            d[j] = [(1.0 - a) * prev[cc] + a * d[j][cc] for cc in range(3)]
    x, y, ww = d[pp]
    return (x / ww + o[0], y / ww + o[1])


def chord_distance(q, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    l2 = dx * dx + dy * dy
    if l2 == 0.0:
        return math.sqrt((q[0] - a[0]) * (q[0] - a[0]) + (q[1] - a[1]) * (q[1] - a[1]))
    return abs((q[0] - a[0]) * dy - (q[1] - a[1]) * dx) / math.sqrt(l2)


def tessellate(c, tol):
    """The points and their parameters."""
    pp, ctrl, k, _ = c
    breaks = []
    for x in k[pp : len(ctrl) + 1]:
        if not breaks or breaks[-1] != x:
            breaks.append(x)
    out = [point(c, breaks[0])]
    params = [breaks[0]]
    least = 2 if pp >= 2 else 0

    def refine(u0, p0, u1, p1, depth):
        um = 0.5 * (u0 + u1)
        pm = point(c, um)
        q1 = point(c, 0.5 * (u0 + um))
        q3 = point(c, 0.5 * (um + u1))
        far = max(max(chord_distance(pm, p0, p1), chord_distance(q1, p0, p1)), chord_distance(q3, p0, p1))
        if depth < DEPTH and (depth < least or far > tol):
            refine(u0, p0, um, pm, depth + 1)
            refine(um, pm, u1, p1, depth + 1)
        else:
            out.append(p1)
            params.append(u1)

    for u0, u1 in zip(breaks, breaks[1:]):
        refine(u0, out[-1], u1, point(c, u1), 0)
    return out, params


def exact_point(c, u):
    """The point at `u` from the basis functions (A2.2), in 50 digits."""
    mp.dps = 50
    pp, ctrl, k, w = c
    km = [mpf(x) for x in k]
    um = mpf(u)
    s = find_span(len(ctrl) - 1, pp, um, km)
    nb = basis(s, um, pp, km, mpf(0), mpf(1))
    sx = sy = sw = mpf(0)
    for r, v in enumerate(nb):
        i = s - pp + r
        wi = mpf(w[i]) if w is not None else mpf(1)
        sx += v * wi * mpf(ctrl[i][0])
        sy += v * wi * mpf(ctrl[i][1])
        sw += v * wi
    return (sx / sw, sy / sw)


def checked_tessellation(c, tol):
    pts, params = tessellate(c, tol)
    scale = max(1.0, max(max(abs(a), abs(b)) for a, b in c[1]))
    for q, u in zip(pts, params):
        x, y = exact_point(c, u)
        assert max(abs(x - mpf(q[0])), abs(y - mpf(q[1]))) <= 1e-12 * scale, f"çoklu çizginin noktası {u}"
    for (a, b), (ua, ub) in zip(zip(pts, pts[1:]), zip(params, params[1:])):
        for j in range(1, 16):
            x, y = exact_point(c, ua + (ub - ua) * j / 16)
            assert chord_distance((float(x), float(y)), a, b) <= tol * 1.05, f"kiriş {ua}–{ub} eğriden uzak"
    return pts


def insert(c, u):
    """The curve with `u` inserted once (A5.1, in homogeneous coordinates)."""
    pp, ctrl, k, w = c
    np_ = len(ctrl) - 1
    s_idx = span_of(pp, ctrl, k, u)
    s = sum(1 for x in k if x == u)
    r = min(1, max(pp - s, 0))
    pw = [[ctrl[i][0] * (w[i] if w else 1.0), ctrl[i][1] * (w[i] if w else 1.0), w[i] if w else 1.0] for i in range(np_ + 1)]
    assert r == 1
    uq = k[: s_idx + 1] + [u] * r + k[s_idx + 1 :]
    qw = [[0.0, 0.0, 0.0] for _ in range(np_ + r + 1)]
    for i in range(s_idx - pp + 1):
        qw[i] = pw[i]
    for i in range(s_idx - s, np_ + 1):
        qw[i + r] = pw[i]
    rw = [list(pw[s_idx - pp + i]) for i in range(pp - s + 1)]
    ll = 0
    for j in range(1, r + 1):
        ll = s_idx - pp + j
        for i in range(pp - j - s + 1):
            alpha = (u - k[ll + i]) / (k[i + s_idx + 1] - k[ll + i])
            rw[i] = [alpha * rw[i + 1][cc] + (1.0 - alpha) * rw[i][cc] for cc in range(3)]
        qw[ll] = rw[0]
        qw[s_idx + r - j - s] = rw[pp - j - s]
    for i in range(ll + 1, s_idx - s):
        qw[i] = rw[i - ll]
    out = (pp, [(q[0] / q[2], q[1] / q[2]) for q in qw], uq, [q[2] for q in qw] if w else None)
    # The same curve: 41 parameters across the domain, in 50 digits.
    lo, hi = k[pp], k[len(ctrl)]
    for j in range(41):
        t = lo + (hi - lo) * j / 40
        a, b = exact_point(c, t), exact_point(out, t)
        assert max(abs(a[0] - b[0]), abs(a[1] - b[1])) <= 1e-10, f"düğüm eklenen eğri {t}'de ayrılıyor"
    return out


# ── The drawing ────────────────────────────────────────────────────────────────────────────────────────────────────


def node(i, name, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": dict(STYLE), "children": []}


def base(i, layer="cizim", **more):
    return {"id": i, "layerId": layer, "attrs": {}, **more}


def nurbs_entity(i, c, closed=False, fit=None, tangents=None, **more):
    pp, ctrl, knots, w = c
    e = {"kind": "nurbs", **base(i, **more), "degree": pp, "ctrl": [p(*q) for q in ctrl], "knots": [num(k) for k in knots]}
    if w is not None:
        e["weights"] = [num(x) for x in w]
    if closed:
        e["closed"] = True
        e["periodic"] = True
    if fit is not None:
        e["fit"] = [p(*q) for q in fit]
    if tangents is not None:
        e["tangents"] = [p(*t) if t is not None else None for t in tangents]
    return e


RIVER = [(0, 60), (12, 67.5), (25, 63), (38, 71), (52, 66.25)]
POND = [(60, 10), (80, 8), (86, 22), (70, 30), (56, 22)]
COLUMNS = [(0, 40), (8, 46), (16, 38), (24, 47), (32, 41), (40, 44)]
SQUARE = [(60, 40), (70, 40), (70, 50), (60, 50)]
QUARTER = [(10, 0), (10, 10), (0, 10)]

BULGED = {"kind": "polyline", **base(1), "pts": [p(0, 0), p(10, 0), p(20, 5), p(30, 0)], "bulges": [0, 0.4, 0, 0]}
CLOSED = {"kind": "polyline", **base(2, color="#ff0000"), "pts": [p(0, 20), p(10, 20), p(10, 30), p(0, 30)],
          "bulges": [0, 0, 0, -0.3], "closed": True, "widths": [[0.1, 0.1], [0.1, 0.2], [0.2, 0.2], [0.3, 0.3]]}
PARTS = {"kind": "polyline", **base(3), "pts": [p(0, 90), p(5, 90)], "parts": [{"pts": [p(0, 95), p(5, 95)]}]}
_fit = through([tuple(map(float, q)) for q in RIVER], 3, False, "edit")
FIT = nurbs_entity(4, (_fit[0], _fit[1], _fit[2], None), fit=_fit[3], attrs={"Ad": "Dere"})
CTRL = nurbs_entity(5, by_control(3, [tuple(map(float, q)) for q in COLUMNS], None, False, "edit"))
RING = nurbs_entity(6, by_control(2, [tuple(map(float, q)) for q in SQUARE], [1.0, 2.0, 1.0, 0.5], True, "edit"), closed=True)
_pond = through([tuple(map(float, q)) for q in POND], 3, True, "edit")
PONDE = nurbs_entity(7, (_pond[0], _pond[1], _pond[2], None), closed=True, fit=_pond[3])
LINE = {"kind": "line", **base(8), "a": p(40, 80), "b": p(45, 80)}
LOCKED = {"kind": "polyline", **base(9, "kilitli"), "pts": [p(0, 100), p(5, 100), p(5, 105)]}
TWO = {"kind": "polyline", **base(10), "pts": [p(0, 80), p(5, 80)]}
ARC = nurbs_entity(11, (2, [tuple(map(float, q)) for q in QUARTER], [0.0, 0.0, 0.0, 1.0, 1.0, 1.0], [1.0, 0.7071067811865476, 1.0]))
ENTITIES = [BULGED, CLOSED, PARTS, FIT, CTRL, RING, PONDE, LINE, LOCKED, TWO, ARC]
BY_ID = {e["id"]: e for e in ENTITIES}
IDS = [e["id"] for e in ENTITIES]


def setup():
    return {
        "format": "kentos.document", "version": 1, "name": "Eğriyi düzenle",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 100,
                     "workspace": "cad"},
        "origin": p(0, 0),
        "layers": [node("cizim", "Çizim"), node("kilitli", "Kilitli", locked=True)],
        "activeLayer": "cizim",
        "entities": copy.deepcopy(ENTITIES),
        "styles": {"items": [], "categories": []},
    }


# ── The rules ──────────────────────────────────────────────────────────────────────────────────────────────────────

NOT_FIT = "Eğrinin geçiş noktası yok: denetim noktalarıyla çizilmiş ya da denetim noktası düzenlenmiş. Denetim noktası düzenlemelerini kullanın."
NOT_POLYLINE = "Çoklu çizgide Kapat, Aç, Düzleştir ve Eğriye çevir yapılır; nokta ve düğüm düzenlemeleri NURBS eğrisinindir."
NOT_NURBS = "NURBS eğrisinde Düzleştir ve Eğriye çevir yapılmaz; onlar çoklu çizginin."
NOT_KIND = "Eğriyi düzenle çoklu çizgide ve NURBS eğrisinde çalışır. Bir çoklu çizgi ya da NURBS eğrisi verin."
AXIS = {"x": "doğu (Y)", "y": "kuzey (X)"}


def keep(e):
    """The object's own fields an edit keeps: its id, layer, look and data."""
    return {k: v for k, v in e.items() if k in ("id", "layerId", "attrs", "color", "lineWeight", "label", "symbol")}


def look(e):
    """The NURBS curve's numbers for the twin."""
    return (e["degree"], [tup(q) for q in e["ctrl"]], [float(k) for k in e["knots"]],
            [float(w) for w in e["weights"]] if "weights" in e else None)


def drawn(e):
    return len(e["ctrl"]) - e["degree"] if e.get("closed") and e.get("periodic") else len(e["ctrl"])


def as_nurbs(e, c, closed=False, fit=None, tangents=None):
    out = nurbs_entity(e["id"], c, closed, fit, tangents)
    out.update(keep(e))
    return out


def bad_index(index, count, what):
    raise Refused("invalid_index", f"Eğrinin {count} {what} var; {index} numaralısı yok. 0 ile {count - 1} arasında bir numara verin.", "edit.index")


def refit(e, fit, closed, tangents, path):
    if closed:
        pp, ctrl, knots, q = through(fit, e["degree"], True, path)
        return as_nurbs(e, (pp, ctrl, knots, None), True, q)
    start, end = tangents if tangents is not None else (None, None)
    pp, ctrl, knots, q = through_tangents(fit, e["degree"], start, end, path)
    kept = tangents if tangents is not None and (tangents[0] is not None or tangents[1] is not None) else None
    return as_nurbs(e, (pp, ctrl, knots, None), False, q, kept)


def polyline_edit(e, edit):
    if e.get("parts"):
        raise Refused("multi_part", "Çok parçalı çoklu çizgi düzenlenmez; önce Parçalara ayır.", "uid")
    pts = [tup(q) for q in e["pts"]]
    n, kind = len(pts), edit["kind"]
    out = dict(e)
    if kind == "close":
        if e.get("closed"):
            raise Refused("edit_not_applicable", "Çoklu çizgi zaten kapalı.", "edit")
        if n < 3:
            raise Refused("too_few_points", "Kapanan çoklu çizginin en az üç köşesi olmalı.", "uid")
        out["closed"] = True
        return out
    if kind == "open":
        if not e.get("closed"):
            raise Refused("edit_not_applicable", "Çoklu çizgi zaten açık.", "edit")
        out.pop("closed")
        if "bulges" in out:
            out["bulges"] = out["bulges"][: n - 1] + [0]
        if "widths" in out:
            out["widths"] = out["widths"][: n - 1] + [[0, 0]]
        return out
    if kind == "decurve":
        if not any(b != 0 for b in e.get("bulges", [])):
            raise Refused("edit_not_applicable", "Çoklu çizginin yay kenarı yok; zaten düz.", "edit")
        out.pop("bulges")
        return out
    if kind == "toSpline":
        degree = edit.get("degree", 3)
        if not 1 <= degree <= MAX_DEGREE:
            raise Refused("invalid_nurbs", f"Derece {degree}; 1 ile {MAX_DEGREE} arasında olmalı.", "edit.degree")
        closed = bool(e.get("closed"))
        if edit["through"]:
            pp, ctrl, knots, q = through(pts, degree, closed, "edit")
            return as_nurbs(e, (pp, ctrl, knots, None), closed, q)
        return as_nurbs(e, by_control(fit_degree(degree, n), pts, None, closed, "edit"), closed)
    raise Refused("edit_not_applicable", NOT_POLYLINE, "edit")


def nurbs_edit(e, edit):
    pp, kind = e["degree"], edit["kind"]
    closed = bool(e.get("closed"))
    fit = [tup(q) for q in e["fit"]] if "fit" in e else None
    tangents = [tup(t) if t is not None else None for t in e["tangents"]] if "tangents" in e else None

    def fit_of():
        if fit is None:
            raise Refused("no_fit_points", NOT_FIT, "edit")
        return list(fit)

    if kind in ("close", "open"):
        closing = kind == "close"
        if closed == closing:
            raise Refused("edit_not_applicable", "Eğri zaten kapalı." if closing else "Eğri zaten açık.", "edit")
        if fit is not None:
            return refit(e, list(fit), closing, None if closing else tangents, "edit")
        c = look(e)
        count = drawn(e)
        w = c[3][:count] if c[3] is not None else None
        return as_nurbs(e, by_control(pp, c[1][:count], w, closing, "edit"), closing)
    if kind == "addFit":
        f = fit_of()
        if edit["index"] > len(f):
            raise Refused("invalid_index", f"Yeni geçiş noktası 0 ile {len(f)} arasındaki bir yere eklenir; {edit['index']} numaralı yer yok.", "edit.index")
        f.insert(edit["index"], tup(edit["at"]))
        return refit(e, f, closed, tangents, "edit.at")
    if kind == "moveFit":
        f = fit_of()
        if edit["index"] >= len(f):
            bad_index(edit["index"], len(f), "geçiş noktası")
        f[edit["index"]] = tup(edit["at"])
        return refit(e, f, closed, tangents, "edit.at")
    if kind == "removeFit":
        f = fit_of()
        if edit["index"] >= len(f):
            bad_index(edit["index"], len(f), "geçiş noktası")
        f.pop(edit["index"])
        return refit(e, f, closed, tangents, "edit.index")
    if kind == "tangents":
        f = fit_of()
        if closed:
            raise Refused("edit_not_applicable", "Kapalı eğrinin uç teğeti olmaz; önce Aç.", "edit")
        s = tup(edit["start"]) if edit.get("start") is not None else None
        t = tup(edit["end"]) if edit.get("end") is not None else None
        return refit(e, f, False, [s, t], "edit")
    count = drawn(e)
    if kind in ("moveControl", "weight", "removeControl") and edit["index"] >= count:
        bad_index(edit["index"], count, "denetim noktası")
    if kind == "moveControl":
        i = edit["index"]
        out = {k: v for k, v in e.items() if k not in ("fit", "tangents")}
        out["ctrl"] = list(e["ctrl"])
        out["ctrl"][i] = p(*tup(edit["at"]))
        if count < len(out["ctrl"]) and i < len(out["ctrl"]) - count:
            out["ctrl"][i + count] = p(*tup(edit["at"]))
        return out
    if kind == "weight":
        i = edit["index"]
        out = {k: v for k, v in e.items() if k not in ("fit", "tangents", "weights")}
        w = list(e.get("weights", [1] * len(e["ctrl"])))
        w[i] = num(edit["weight"])
        if count < len(w) and i < len(w) - count:
            w[i + count] = num(edit["weight"])
        if any(x != 1 for x in w):
            out["weights"] = w
        return out
    if kind == "removeControl":
        c = look(e)
        i = edit["index"]
        ctrl = c[1][:count]
        ctrl.pop(i)
        w = None
        if c[3] is not None:
            w = c[3][:count]
            w.pop(i)
        least = max(pp, 3) if closed else pp + 1
        if len(ctrl) < least:
            raise Refused("too_few_points", f"{pp}. dereceden {'kapalı' if closed else 'açık'} eğrinin en az {least} denetim noktası olmalı; "
                          f"silinirse {len(ctrl)} kalır.", "edit.index")
        return as_nurbs(e, by_control(pp, ctrl, w, closed, "edit.index"), closed)
    if kind == "insertKnot":
        if closed:
            raise Refused("invalid_knot", "Kapalı eğriye düğüm eklenmez; önce Aç.", "edit.at")
        c = look(e)
        u = float(edit["at"])
        lo, hi = c[2][pp], c[2][len(c[1])]
        if not (lo < u < hi) or sum(1 for k in c[2] if k == u) >= pp:
            raise Refused("invalid_knot", f"Düğüm eğrinin aralığının içinde ({num(lo)} ile {num(hi)} arasında) ve orada {pp} kereden az olmalı.",
                          "edit.at")
        return as_nurbs(e, insert(c, u))
    if kind == "toPolyline":
        pts = checked_tessellation(look(e), float(edit["tolerance"]))
        if closed:
            pts = pts[:-1]
        out = {"kind": "polyline", **keep(e), "pts": [p(*q) for q in pts]}
        if closed:
            out["closed"] = True
        return out
    raise Refused("edit_not_applicable", NOT_NURBS, "edit")


def numbers(edit):
    """The edit's own numbers: finite, a weight and a tolerance above 0."""
    def point_(q, path):
        for axis in ("x", "y"):
            if not math.isfinite(q[axis]):
                raise Refused("not_finite", f"Noktanın {AXIS[axis]} değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.",
                              f"{path}.{axis}")

    def finite(v, what, fix, path):
        if not math.isfinite(v):
            raise Refused("not_finite", f"{what} sonlu bir sayı değil (NaN ya da sonsuz). {fix}", path)

    kind = edit["kind"]
    if kind in ("addFit", "moveFit", "moveControl"):
        point_(edit["at"], "edit.at")
    elif kind == "tangents":
        for side in ("start", "end"):
            if edit.get(side) is not None:
                point_(edit[side], f"edit.{side}")
    elif kind == "weight":
        finite(edit["weight"], "Ağırlık", "Ağırlığı sonlu bir sayıyla verin.", "edit.weight")
        if edit["weight"] <= 0:
            raise Refused("invalid_weight", "Ağırlık sıfırdan büyük olmalı. Başka bir ağırlık verin.", "edit.weight")
    elif kind == "insertKnot":
        finite(edit["at"], "Düğüm", "Düğümü sonlu bir sayıyla verin.", "edit.at")
    elif kind == "toPolyline":
        finite(edit["tolerance"], "Tolerans", "Toleransı sonlu bir sayıyla verin.", "edit.tolerance")
        if edit["tolerance"] <= 0:
            raise Refused("invalid_tolerance", "Tolerans sıfırdan büyük olmalı. Başka bir tolerans verin.", "edit.tolerance")


WIDTHS_LOST = {"code": "widths_lost", "message": "1 çoklu çizginin genişliği bu işlemde korunmadı.", "path": "changes"}


def warnings_of(e, after):
    """A polyline's widths a curve does not carry are said (docs/adr/0258 §2)."""
    return [WIDTHS_LOST] if "widths" in e and "widths" not in after else []


def edited(e, edit):
    """The object after the edit, or the refusal."""
    numbers(edit)
    if e["layerId"] == "kilitli":
        raise Refused("layer_locked", "“Kilitli” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın.", "uid")
    if e["kind"] == "polyline":
        return polyline_edit(e, edit)
    if e["kind"] == "nurbs":
        return nurbs_edit(e, edit)
    raise Refused("not_a_curve", NOT_KIND, "uid")


# ── Cases ──────────────────────────────────────────────────────────────────────────────────────────────────────────

NAN = {"NaN": math.nan, "Infinity": math.inf, "-Infinity": -math.inf}


def with_non_finite(edit, non_finite):
    edit = copy.deepcopy(edit)
    for path, word in (non_finite or {}).items():
        parts = path.split(".")[1:]
        node_ = edit
        for key in parts[:-1]:
            node_ = node_[key]
        node_[parts[-1]] = NAN[word]
    return edit


class Cases:
    def __init__(self):
        self.cases = []

    def run(self, i, edit, op="execute", state=None, non_finite=None, **more):
        """A step editing object `i` (as `state` has it, else as set up): its result and expectation."""
        e = state if state is not None else BY_ID[i]
        step = {"op": op, "input": {"uid": U(i), "edit": edit, **more}}
        if non_finite:
            step["nonFinite"] = non_finite
        try:
            after = edited(e, with_non_finite(edit, non_finite))
        except Refused as r:
            step["result"] = {"status": "failed", "error": {"code": r.code, "message": r.message, "path": r.path}}
            step["expect"] = {"revision": "same"}
            return step, None
        if op == "plan":
            step["result"] = {"status": "completed", "warnings": warnings_of(e, after),
                              "output": {"changed": [after], "created": [], "removed": [], "revision": "$current"}}
            step["expect"] = {"revision": "same"}
        elif op == "validate":
            step["result"] = {"status": "completed", "output": None, "warnings": warnings_of(e, after)}
            step["expect"] = {"revision": "same"}
        else:
            step["result"] = {"status": "completed", "warnings": warnings_of(e, after),
                              "output": {"changed": [U(i)], "created": [], "removed": [], "revision": "$current"}}
            step["expect"] = {"ids": IDS, "entities": {str(i): after}, "revision": "changed", "canUndo": True}
        return step, after

    def one(self, name, i, edit, undo=False, **kw):
        step, _ = self.run(i, edit, **kw)
        steps = [step]
        if undo:
            steps.append({"op": "undo", "returns": LABEL, "expect": {"entities": {str(i): BY_ID[i]}, "canUndo": False}})
        self.cases.append({"name": name, "steps": steps})

    def chain(self, name, i, edits):
        """Edits one after another, each on the last one's result."""
        steps, state = [], None
        for edit in edits:
            step, after = self.run(i, edit, state=state)
            steps.append(step)
            state = after if after is not None else state
        self.cases.append({"name": name, "steps": steps})

    def refused(self, name, items):
        """Refusals, each changing nothing: [(id, edit, extra input, non_finite)]."""
        steps = []
        for i, edit, more, non_finite in items:
            step, after = self.run(i, edit, non_finite=non_finite, **more)
            assert after is None, name
            step["expect"] = {"ids": IDS, "revision": "same", "canUndo": False}
            steps.append(step)
        self.cases.append({"name": name, "steps": steps})


def cases():
    w = Cases()
    # ── polylines ──
    w.one("Kapat: açık çoklu çizgi kapanır; kapanış kenarı düz (son büküm 0); adım “Eğriyi düzenle”, geri alınır", 1, {"kind": "close"}, undo=True)
    w.one("Aç: kapalı çoklu çizginin kapanış kenarının bükümü ve genişliği gider; rengi kalır", 2, {"kind": "open"})
    w.one("Düzleştir: yay kenarları düz olur, bükümler gider", 1, {"kind": "decurve"})
    w.one("Eğriye çevir (denetim): köşeler kübik eğrinin denetim noktaları olur, düğümler uçlara sıkıştırılmış ve eşit aralıklı", 1,
          {"kind": "toSpline", "through": False}, undo=True)
    w.one("Eğriye çevir (geçiş): ikinci dereceden eğri köşelerden geçer; köşeler geçiş noktalarıdır", 1, {"kind": "toSpline", "degree": 2, "through": True})
    w.one("Kapalı çoklu çizgi kapalı ve periyodik eğri olur, köşelerden geçer; rengi kalır, genişliği uyarıyla gider", 2, {"kind": "toSpline", "through": True})
    w.one("Kapalı çoklu çizgi denetim noktalarıyla: ilk derece kadar köşe sona sarılır", 2, {"kind": "toSpline", "degree": 2, "through": False})
    # ── a fit curve ──
    w.one("Geçiş noktası ekle: eğri yeni noktadan da geçer; öznitelikleri kalır", 4, {"kind": "addFit", "index": 2, "at": p(18, 70)}, undo=True)
    w.one("Geçiş noktasını taşı: eğri yeniden kurulur", 4, {"kind": "moveFit", "index": 0, "at": p(-2, 58)})
    w.one("Geçiş noktasını sil: eğri kalan dört noktadan geçer", 4, {"kind": "removeFit", "index": 4})
    w.chain("Uç teğetleri: eğri doğrultularla başlar ve biter; teğetler saklanır, taşımada kalır, kaldırılınca gider", 4, [
        {"kind": "tangents", "start": p(0, 1), "end": p(1, -1)},
        {"kind": "moveFit", "index": 2, "at": p(25, 61)},
        {"kind": "tangents"},
    ])
    w.one("Yalnız başlangıç teğeti: bir denetim noktası eklenir", 4, {"kind": "tangents", "start": p(1, 2)})
    w.chain("Geçiş noktalı eğri kapanır (teğetler gider) ve yeniden açılır", 4, [
        {"kind": "tangents", "end": p(1, 0)},
        {"kind": "close"},
        {"kind": "open"},
    ])
    w.one("Kapalı geçiş noktalı eğri açılır: aynı noktalardan geçen açık eğri", 7, {"kind": "open"})
    w.one("Kapalı eğriye geçiş noktası: kapalı kalır, yeni noktadan da geçer", 7, {"kind": "addFit", "index": 5, "at": p(55, 12)})
    # ── control points ──
    w.one("Denetim noktasını taşı: düğümler kalır", 5, {"kind": "moveControl", "index": 2, "at": p(16, 34)})
    w.one("Kapalı eğrinin denetim noktası: sona sarılan kopyası da taşınır", 6, {"kind": "moveControl", "index": 0, "at": p(58, 38)})
    w.one("Ağırlık: kapalı eğride kopyası da değişir", 6, {"kind": "weight", "index": 1, "weight": 3})
    w.chain("Ağırlık: hepsi 1 olunca ağırlıklar kalkar", 5, [
        {"kind": "weight", "index": 3, "weight": 2},
        {"kind": "weight", "index": 3, "weight": 1},
    ])
    w.one("Geçiş noktalı eğride denetim noktası taşınınca geçiş noktaları ve teğetler gider", 4, {"kind": "moveControl", "index": 1, "at": p(10, 70)})
    w.one("Denetim noktasını sil: kalanlarla uçlara sıkıştırılmış düğümler", 5, {"kind": "removeControl", "index": 1})
    w.one("Kapalı ağırlıklı eğriden denetim noktası silinir: üç nokta kalır, sarılır", 6, {"kind": "removeControl", "index": 3})
    w.one("Düğüm ekle: eğri değişmez, bir denetim noktası eklenir", 5, {"kind": "insertKnot", "at": 1.5}, undo=True)
    w.one("Ağırlıklı eğriye düğüm: çeyrek daire aynı kalır, ağırlıklar homojen koordinatlarla", 11, {"kind": "insertKnot", "at": 0.5})
    w.one("Var olan bir düğüm bir kez daha", 5, {"kind": "insertKnot", "at": 1})
    w.one("Denetim noktalı açık eğri kapanır: ilk derece kadar nokta sarılır", 5, {"kind": "close"})
    w.one("Kapalı denetim noktalı eğri açılır: uçlara sıkıştırılmış, ağırlıklar kalır", 6, {"kind": "open"})
    w.one("Çoklu çizgiye çevir: her kiriş eğriye 1 cm'den yakın", 5, {"kind": "toPolyline", "tolerance": 0.01})
    w.one("Kapalı eğri kapalı çoklu çizgi olur; son nokta (ilkiyle aynı) yazılmaz", 6, {"kind": "toPolyline", "tolerance": 0.05})
    # ── plan and validation ──
    plan, _ = w.run(4, {"kind": "addFit", "index": 2, "at": p(18, 70)}, op="plan")
    valid, _ = w.run(5, {"kind": "insertKnot", "at": 2.5}, op="validate")
    refused_valid, _ = w.run(5, {"kind": "insertKnot", "at": 3}, op="validate")
    w.cases.append({"name": "Plan düzenlenen eğriyi gösterir, doğrulama yazmanın cevabını verir; hiçbir şey yazılmaz",
                    "steps": [plan, valid, refused_valid]})
    # ── refusals ──
    w.refused("Çoklu çizginin retleri: çok parçalı, zaten kapalı ve açık, iki köşe kapanmaz, düz olan düzleşmez, düğüm eklenmez, derece 0", [
        (3, {"kind": "close"}, {}, None),
        (2, {"kind": "close"}, {}, None),
        (1, {"kind": "open"}, {}, None),
        (10, {"kind": "close"}, {}, None),
        (10, {"kind": "decurve"}, {}, None),
        (1, {"kind": "insertKnot", "at": 0.5}, {}, None),
        (1, {"kind": "toSpline", "degree": 0, "through": True}, {}, None),
    ])
    w.refused("Eğrinin retleri: geçiş noktası yok, numara yok, kapalı eğrinin teğeti, düğüm aralık dışında ve kapalıda, çok az denetim noktası", [
        (5, {"kind": "addFit", "index": 0, "at": p(1, 1)}, {}, None),
        (4, {"kind": "moveFit", "index": 5, "at": p(1, 1)}, {}, None),
        (4, {"kind": "addFit", "index": 6, "at": p(1, 1)}, {}, None),
        (6, {"kind": "moveControl", "index": 4, "at": p(1, 1)}, {}, None),
        (7, {"kind": "tangents", "start": p(1, 0)}, {}, None),
        (5, {"kind": "insertKnot", "at": 0}, {}, None),
        (6, {"kind": "insertKnot", "at": 3}, {}, None),
        (11, {"kind": "removeControl", "index": 0}, {}, None),
        (7, {"kind": "close"}, {}, None),
        (5, {"kind": "decurve"}, {}, None),
    ])
    w.refused("Yinelenen geçiş noktası ve doğrultusuz teğet", [
        (4, {"kind": "moveFit", "index": 1, "at": p(*RIVER[0])}, {}, None),
        (4, {"kind": "tangents", "start": p(0, 0)}, {}, None),
    ])
    w.refused("Sayılar: NaN ya da sonsuz, sıfır ağırlık ve tolerans", [
        (5, {"kind": "moveControl", "index": 0, "at": p(1, 1)}, {}, {"edit.at.y": "NaN"}),
        (5, {"kind": "weight", "index": 0, "weight": 2}, {}, {"edit.weight": "Infinity"}),
        (5, {"kind": "weight", "index": 0, "weight": 0}, {}, None),
        (5, {"kind": "insertKnot", "at": 1}, {}, {"edit.at": "-Infinity"}),
        (5, {"kind": "toPolyline", "tolerance": 0}, {}, None),
        (5, {"kind": "toPolyline", "tolerance": 1}, {}, {"edit.tolerance": "NaN"}),
    ])
    w.refused("Nesne: çizgi eğri değil, kilitli katman", [
        (8, {"kind": "close"}, {}, None),
        (9, {"kind": "close"}, {}, None),
    ])
    w.cases.append({"name": "Kimlik UUID değil, nesne yok, sürüm sayı değil; retlerin sırası: kimlik, sayılar, sürüm, nesne", "steps": [
        {"op": "execute", "input": {"uid": "on iki", "edit": {"kind": "close"}},
         "result": {"status": "failed", "error": {"code": "invalid_uid", "path": "uid", "message":
                    "“on iki” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). "
                    "Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın."}},
         "expect": {"ids": IDS, "revision": "same"}},
        {"op": "execute", "input": {"uid": "0192f5a2-0000-7000-8000-00000000ffff", "edit": {"kind": "close"}},
         "result": {"status": "failed", "error": {"code": "entity_not_found", "path": "uid", "message":
                    "“0192f5a2-0000-7000-8000-00000000ffff” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."}},
         "expect": {"ids": IDS, "revision": "same"}},
        {"op": "execute", "input": {"uid": U(1), "edit": {"kind": "close"}, "expectedRevision": "on iki"},
         "result": {"status": "failed", "error": {"code": "invalid_revision", "path": "expectedRevision", "message":
                    "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."}},
         "expect": {"ids": IDS, "revision": "same"}},
        {"op": "execute", "input": {"uid": U(5), "edit": {"kind": "weight", "index": 9, "weight": -1}, "expectedRevision": "on iki"},
         "result": {"status": "failed", "error": {"code": "invalid_weight", "path": "edit.weight", "message":
                    "Ağırlık sıfırdan büyük olmalı. Başka bir ağırlık verin."}},
         "expect": {"ids": IDS, "revision": "same"}},
        {"op": "execute", "input": {"uid": "0192f5a2-0000-7000-8000-00000000ffff", "edit": {"kind": "weight", "index": 0, "weight": 2},
                                    "expectedRevision": "on iki"},
         "result": {"status": "failed", "error": {"code": "invalid_revision", "path": "expectedRevision", "message":
                    "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."}},
         "expect": {"ids": IDS, "revision": "same"}},
    ]})
    w.cases.append({"name": "Beklenen sürüm geçmişte kaldıysa conflict, hiçbir şey yazılmaz", "steps": [
        {"op": "captureRevision", "as": "old"},
        {"op": "execute", "command": "cad.entities.transform", "input": {"uids": [U(8)], "transform": {"kind": "move", "dx": 1, "dy": 0}}, "result": "$any"},
        {"op": "execute", "input": {"uid": U(1), "edit": {"kind": "close"}, "expectedRevision": "$old"},
         "result": {"status": "conflict", "error": {
             "code": "revision_conflict", "path": "expectedRevision", "revision": "$current",
             "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."}},
         "expect": {"revision": "same"}}]})
    return w.cases


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.curves.edit", "commandVersion": 1,
            "title": "Eğriyi düzenle: çoklu çizginin ve NURBS eğrisinin düzenlemeleri, retler",
            "note": "ADR 0303. Bağımsız başvuru: scripts/fixtures/curve_edit_cases.py (yapıların IEEE çift duyarlıklı ikizi, "
                    "50 basamaklı mpmath denetimiyle).",
            "setup": setup(), "cases": cases()}


def main():
    check = "--check" in sys.argv[1:]
    data = build()
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} tutarlı: {len(data['cases'])} durum")
        return 0
    OUT.write_text(text)
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(data['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
