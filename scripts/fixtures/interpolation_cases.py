"""İnterpolasyon ve yoğunluk's cases (docs/adr/0232), worked out without
KentOS code:

- the points the objects give (§2): vertices and elevations, a field's
  numbers (ADR 0200's decimal rule), equal places joined by their mean;
- the grid's rule (§3): the cell size chosen and its edges on its multiples;
- Ters uzaklık (§5) by brute force in mpmath, cross-checked against GDAL's
  `invdistnn` on the same points;
- TIN'den raster (§7) in qhull's Delaunay triangles (matplotlib) with exact
  rational barycentric weights, cross-checked against GDAL's `linear`;
- Doğal komşu (§6) by Sibson's definition: the Voronoi cells clipped exactly
  with rational arithmetic, the area each neighbour loses over the new cell's;
- Spline (§8) and Kriging (§9) solving each cell's system in mpmath at 40
  digits (K₀ from mpmath's besselk); the variogram's fit by a dense search;
- Çekirdek and Çizgi yoğunluğu (§10, §11) by their definitions in mpmath;
- each interpolation's cross-validation (§12): TIN and Doğal komşu on the
  set without the point (qhull again), the others from the other points.

    python3 scripts/fixtures/interpolation_cases.py           # writes the file
    python3 scripts/fixtures/interpolation_cases.py --check   # writes nothing; compares

Writes fixtures/interpolation/v1/cases.json. Needs numpy, matplotlib, mpmath
and GDAL's Python bindings.
"""
import json
import math
import os
import sys
from fractions import Fraction

import mpmath
import numpy as np
import matplotlib.tri as mtri
from osgeo import gdal, ogr

gdal.UseExceptions()
mpmath.mp.dps = 40

OUT = 'fixtures/interpolation/v1/cases.json'
EULER = mpmath.euler


# ── The objects' points (§2) ─────────────────────────────────────────

def read_number(text):
    """ADR 0200's decimal: sign, ASCII digits, one '.' or ',', no exponent, at most 30 digits, spaces trimmed; else None."""
    if text is None:
        return None
    t = text.strip()
    neg = t.startswith('-')
    body = t[1:] if neg or t.startswith('+') else t
    seps = [k for k, c in enumerate(body) if c in '.,']
    if len(seps) > 1:
        return None
    ip, fp = (body[:seps[0]], body[seps[0] + 1:]) if seps else (body, '')
    if not ip and not fp:
        return None
    if any(c not in '0123456789' for c in ip + fp):
        return None
    if len(ip.lstrip('0')) + len(fp) > 30:
        return None
    return float(('-' if neg else '') + (ip or '0') + '.' + (fp or '0'))


def vertices(src):
    """Each vertex and its elevation in the object's order."""
    k = src['kind']
    if k == 'point':
        yield (src['p']['x'], src['p']['y']), src.get('z')
        for q in src.get('parts') or []:
            yield (q['p']['x'], q['p']['y']), q.get('z')
    elif k == 'line':
        yield (src['a']['x'], src['a']['y']), src.get('za')
        yield (src['b']['x'], src['b']['y']), src.get('zb')
    elif k in ('polyline', 'polygon'):
        def ring(r):
            zs = r.get('zs')
            for i, p in enumerate(r['pts']):
                yield (p['x'], p['y']), (zs[i] if zs and i < len(zs) else None)
        yield from ring(src)
        for h in src.get('holes') or []:
            yield from ring(h)
        for part in src.get('parts') or []:
            yield from ring(part)
            for h in part.get('holes') or []:
                yield from ring(h)


def gather(sources, values):
    raw, unread, no_z = [], 0, 0
    for i, s in enumerate(sources):
        if s['kind'] not in ('point', 'line', 'polyline', 'polygon'):
            continue
        val = None
        if values is not None:
            val = read_number(values[i])
            if val is None:
                unread += 1
                continue
        for p, z in vertices(s):
            v = val if val is not None else z
            if v is None:
                no_z += 1
            else:
                raw.append((p, v, i))
    groups = {}
    order = []
    for k, (p, v, i) in enumerate(raw):
        if p not in groups:
            groups[p] = []
            order.append(p)
        groups[p].append(k)
    xy, vs, obj = [], [], []
    for p in order:
        ks = groups[p]
        if len(ks) == 1:
            v = raw[ks[0]][1]
        else:
            s = 0.0
            for k in ks:
                s += raw[k][1]
            v = s / len(ks)
        xy.append(p)
        vs.append(v)
        obj.append(raw[ks[0]][2])
    return {'xy': xy, 'v': vs, 'object': obj, 'merged': len(raw) - len(xy), 'unread': unread, 'noElevation': no_z}


# ── The grid (§3) ────────────────────────────────────────────────────

def nice_cell(s):
    def val(m, k):
        return m * 10.0 ** k if k >= 0 else m / 10.0 ** (-k)
    k = math.floor(math.log10(s))
    for kk in (k + 1, k, k - 1):
        for m in (5.0, 2.5, 2.0, 1.0):
            c = val(m, kk)
            if c <= s:
                return c
    return val(1.0, k - 1)


def grid_of_box(b, cell):
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


def center(aff, i, j):
    x0, a, b, y0, c, d = aff
    u, v = i + 0.5, j + 0.5
    return (x0 + a * u + b * v, y0 + c * u + d * v)


def bounds(xy):
    return [min(p[0] for p in xy), min(p[1] for p in xy), max(p[0] for p in xy), max(p[1] for p in xy)]


def d2(p, q):
    dx, dy = p[0] - q[0], p[1] - q[1]
    return dx * dx + dy * dy


def nearest(xy, q, k, radius=0.0, skip=None):
    r2 = radius * radius
    c = [(d2(p, q), i) for i, p in enumerate(xy) if i != skip and (radius <= 0 or d2(p, q) <= r2)]
    c.sort()
    return c[:k]


# ── Ters uzaklık (§5) ────────────────────────────────────────────────

def idw(pts, q, power, k, radius, least, skip=None):
    near = nearest(pts['xy'], q, k, radius, skip)
    if len(near) < max(least, 1):
        return None
    if near[0][0] == 0.0:
        return pts['v'][near[0][1]]
    num = mpmath.mpf(0)
    den = mpmath.mpf(0)
    for dd, i in near:
        w = 1 / mpmath.power(mpmath.mpf(dd), mpmath.mpf(power) / 2)
        num += w * pts['v'][i]
        den += w
    return float(num / den)


# ── Delaunay, TIN and Sibson (§6, §7) ────────────────────────────────

def orient(a, b, c):
    return (Fraction(b[0]) - Fraction(a[0])) * (Fraction(c[1]) - Fraction(a[1])) - (Fraction(b[1]) - Fraction(a[1])) * (Fraction(c[0]) - Fraction(a[0]))


def qhull_triangles(xy):
    t = mtri.Triangulation(np.array([p[0] for p in xy]), np.array([p[1] for p in xy]))
    out = []
    for tri in t.triangles:
        a, b, c = (int(v) for v in tri)
        if orient(xy[a], xy[b], xy[c]) < 0:
            b, c = c, b
        out.append((a, b, c))
    return out


def tin(xy, vs, tris, q):
    """Exact barycentric value in the qhull triangle holding q; None outside."""
    for a, b, c in tris:
        pa, pb, pc = xy[a], xy[b], xy[c]
        if orient(pa, pb, q) >= 0 and orient(pb, pc, q) >= 0 and orient(pc, pa, q) >= 0:
            area = orient(pa, pb, pc)
            la, lb, lc = orient(q, pb, pc) / area, orient(pa, q, pc) / area, orient(pa, pb, q) / area
            return float(la * Fraction(vs[a]) + lb * Fraction(vs[b]) + lc * Fraction(vs[c]))
    return None


def clip(poly, a, b, c):
    """The part of a convex polygon with a·x + b·y ≤ c (exact)."""
    out = []
    n = len(poly)
    for k in range(n):
        p, q = poly[k], poly[(k + 1) % n]
        fp = a * p[0] + b * p[1] - c
        fq = a * q[0] + b * q[1] - c
        if fp <= 0:
            out.append(p)
        if (fp < 0 < fq) or (fq < 0 < fp):
            t = fp / (fp - fq)
            out.append((p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])))
    return out


def area(poly):
    s = Fraction(0)
    n = len(poly)
    for k in range(n):
        p, q = poly[k], poly[(k + 1) % n]
        s += p[0] * q[1] - p[1] * q[0]
    return abs(s) / 2


def voronoi_cell(site, others, box):
    """The points nearer `site` than every other, within `box` (exact)."""
    poly = box
    sx, sy = site
    for ox, oy in others:
        # |x − s|² ≤ |x − o|²  ⟺  2(o − s)·x ≤ |o|² − |s|²
        poly = clip(poly, 2 * (ox - sx), 2 * (oy - sy), ox * ox + oy * oy - sx * sx - sy * sy)
        if not poly:
            break
    return poly


def sibson(xy, vs, q):
    """Sibson's value at q (None outside the hull): each neighbour's lost area over the new cell's."""
    P = [(Fraction(p[0]), Fraction(p[1])) for p in xy]
    Q = (Fraction(q[0]), Fraction(q[1]))
    for i, p in enumerate(P):
        if p == Q:
            return vs[i]
    b = bounds(xy)
    span = Fraction(max(b[2] - b[0], b[3] - b[1]) * 1e6 + 1e6)
    lo_x, lo_y, hi_x, hi_y = Fraction(b[0]) - span, Fraction(b[1]) - span, Fraction(b[2]) + span, Fraction(b[3]) + span
    box = [(lo_x, lo_y), (hi_x, lo_y), (hi_x, hi_y), (lo_x, hi_y)]
    cq = voronoi_cell(Q, P, box)
    if any(v[0] in (lo_x, hi_x) or v[1] in (lo_y, hi_y) for v in cq):
        return None
    total = area(cq)
    s = Fraction(0)
    for j, pj in enumerate(P):
        part = voronoi_cell(pj, [p for k, p in enumerate(P) if k != j], cq)
        if part:
            a = area(part)
            if a:
                s += a * Fraction(vs[j])
    return float(s / total)


# ── Spline (§8) and Kriging (§9) ─────────────────────────────────────

def spline_r(kind, p, r):
    if r == 0:
        return mpmath.mpf(0)
    r = mpmath.mpf(r)
    two_pi = 2 * mpmath.pi
    if kind == 'regularized':
        if p == 0:
            return r * r / 4 * mpmath.log(r) / two_pi
        tau = mpmath.mpf(p)
        return (r * r / 4 * (mpmath.log(r / (2 * tau)) + EULER - 1) + tau * tau * (mpmath.besselk(0, r / tau) + EULER + mpmath.log(r / (2 * tau)))) / two_pi
    phi = mpmath.mpf(p)
    return -(mpmath.besselk(0, r * phi) + EULER + mpmath.log(r * phi / 2)) / (two_pi * phi * phi)


def spline_at(pts, q, kind, weight, k, skip=None, cache=None):
    near = nearest(pts['xy'], q, k, 0.0, skip)
    trend = 3 if kind == 'regularized' else 1
    if len(near) < trend:
        return None
    ids = sorted(i for _, i in near)
    p = math.sqrt(weight)
    key = (tuple(ids), skip)
    xy = [(mpmath.mpf(pts['xy'][i][0]), mpmath.mpf(pts['xy'][i][1])) for i in ids]
    if cache is not None and key in cache:
        sol = cache[key]
    else:
        m = len(ids)
        n = m + trend
        A = mpmath.zeros(n, n)
        for a in range(m):
            for b in range(m):
                if a != b:
                    A[a, b] = spline_r(kind, p, mpmath.sqrt((xy[a][0] - xy[b][0]) ** 2 + (xy[a][1] - xy[b][1]) ** 2))
            row = [1, xy[a][0], xy[a][1]]
            for t in range(trend):
                A[a, m + t] = row[t]
                A[m + t, a] = row[t]
        rhs = mpmath.matrix([pts['v'][i] for i in ids] + [0] * trend)
        try:
            sol = mpmath.lu_solve(A, rhs)
        except ZeroDivisionError:
            sol = None
        if cache is not None:
            cache[key] = sol
    if sol is None:
        return None
    m = len(ids)
    x, y = mpmath.mpf(q[0]), mpmath.mpf(q[1])
    s = mpmath.mpf(0)
    for a in range(m):
        s += sol[a] * spline_r(kind, p, mpmath.sqrt((x - xy[a][0]) ** 2 + (y - xy[a][1]) ** 2))
    row = [1, x, y]
    for t in range(trend):
        s += sol[m + t] * row[t]
    return float(s)


def model_f(model, u):
    if model == 'spherical':
        return 1.5 * u - 0.5 * u ** 3 if u < 1 else mpmath.mpf(1)
    if model == 'exponential':
        return 1 - mpmath.exp(-3 * u)
    return 1 - mpmath.exp(-3 * u * u)


def gamma(g, h):
    if h > 0:
        return g['nugget'] + g['sill'] * model_f(g['model'], h / g['range'])
    return mpmath.mpf(0)


def kriging_at(pts, q, g, k, radius, skip=None, cache=None):
    near = nearest(pts['xy'], q, k, radius, skip)
    if not near:
        return None, None
    if near[0][0] == 0.0:
        return pts['v'][near[0][1]], 0.0
    ids = sorted(i for _, i in near)
    xy = [(mpmath.mpf(pts['xy'][i][0]), mpmath.mpf(pts['xy'][i][1])) for i in ids]
    m = len(ids)
    key = (tuple(ids),)
    if cache is not None and key in cache:
        A = cache[key]
    else:
        A = mpmath.zeros(m + 1, m + 1)
        for a in range(m):
            for b in range(m):
                if a != b:
                    A[a, b] = gamma(g, mpmath.sqrt((xy[a][0] - xy[b][0]) ** 2 + (xy[a][1] - xy[b][1]) ** 2))
            A[a, m] = 1
            A[m, a] = 1
        if cache is not None:
            cache[key] = A
    x, y = mpmath.mpf(q[0]), mpmath.mpf(q[1])
    g0 = [gamma(g, mpmath.sqrt((x - p[0]) ** 2 + (y - p[1]) ** 2)) for p in xy]
    sol = mpmath.lu_solve(A, mpmath.matrix(g0 + [1]))
    pred = sum(sol[a] * pts['v'][ids[a]] for a in range(m))
    var = sum(sol[a] * g0[a] for a in range(m)) + sol[m]
    return float(pred), float(mpmath.sqrt(max(var, 0)))


def bins_of(xy, vs, lags):
    n = len(xy)
    step = max(1, -(-n // 4000))
    idx = list(range(0, n, step))
    b = bounds([xy[i] for i in idx])
    diag = math.sqrt((b[2] - b[0]) * (b[2] - b[0]) + (b[3] - b[1]) * (b[3] - b[1]))
    most = diag / 2.0
    w = most / lags
    count, sh, sg = [0.0] * lags, [0.0] * lags, [0.0] * lags
    for a, i in enumerate(idx):
        for j in idx[a + 1:]:
            dx, dy = xy[i][0] - xy[j][0], xy[i][1] - xy[j][1]
            h = math.sqrt(dx * dx + dy * dy)
            if h > most or h == 0.0:
                continue
            k = min(int(math.floor(h / w)), lags - 1)
            d = vs[i] - vs[j]
            count[k] += 1.0
            sh[k] += h
            sg[k] += d * d / 2.0
    out = [(sh[k] / count[k], sg[k] / count[k], count[k]) for k in range(lags) if count[k] > 0]
    return out, w, most


def best_at(bins, model, a):
    """The least weighted squared error over nugget, sill ≥ 0 for range a (mpmath)."""
    f = [model_f(model, mpmath.mpf(h) / a) for h, _, _ in bins]
    n = [mpmath.mpf(c) for _, _, c in bins]
    gm = [mpmath.mpf(gv) for _, gv, _ in bins]

    def err(c0, s):
        return sum(n[k] * (gm[k] - c0 - s * f[k]) ** 2 for k in range(len(f)))
    sn, snf, snff = sum(n), sum(n[k] * f[k] for k in range(len(f))), sum(n[k] * f[k] ** 2 for k in range(len(f)))
    sng, snfg = sum(n[k] * gm[k] for k in range(len(f))), sum(n[k] * f[k] * gm[k] for k in range(len(f)))
    cands = []
    det = sn * snff - snf * snf
    if det > mpmath.mpf('1e-12') * sn * snff:
        c0, s = (snff * sng - snf * snfg) / det, (sn * snfg - snf * sng) / det
        if c0 >= 0 and s >= 0:
            cands.append((err(c0, s), c0, s))
    s1 = max(snfg / snff, 0) if snff > 0 else mpmath.mpf(0)
    c1 = max(sng / sn, 0) if sn > 0 else mpmath.mpf(0)
    cands.append((err(0, s1), mpmath.mpf(0), s1))
    cands.append((err(c1, 0), c1, mpmath.mpf(0)))
    return min(cands, key=lambda t: t[0])


def fit_variogram(xy, vs, model, lags):
    """The best range by a dense search over the ADR's interval, then golden refinement in mpmath."""
    bins, w, most = bins_of(xy, vs, lags)
    lo, hi = math.log(w / 2.0), math.log(2.0 * most)
    best = None
    N = 4000
    for m in range(N):
        a = mpmath.exp(lo + (hi - lo) * m / (N - 1))
        e = best_at(bins, model, a)[0]
        if best is None or e < best[0]:
            best = (e, m)
    m = best[1]
    a = mpmath.exp(lo + (hi - lo) * max(m - 1, 0) / (N - 1))
    c = mpmath.exp(lo + (hi - lo) * min(m + 1, N - 1) / (N - 1))
    r = (mpmath.sqrt(5) - 1) / 2
    for _ in range(200):
        x1, x2 = c - r * (c - a), a + r * (c - a)
        if best_at(bins, model, x1)[0] <= best_at(bins, model, x2)[0]:
            c = x2
        else:
            a = x1
    rng = (a + c) / 2
    e, c0, s = best_at(bins, model, rng)
    return {'model': model, 'nugget': float(c0), 'sill': float(s), 'range': float(rng), 'error': float(e),
            'bins': [[h, gv, cnt] for h, gv, cnt in bins]}


# ── Densities (§10, §11) ─────────────────────────────────────────────

KERNELS = {
    'quartic': lambda u2: 3 / mpmath.pi * (1 - u2) ** 2,
    'triangular': lambda u2: 3 / mpmath.pi * (1 - mpmath.sqrt(u2)),
    'uniform': lambda u2: 1 / mpmath.pi,
    'epanechnikov': lambda u2: 2 / mpmath.pi * (1 - u2),
    'triweight': lambda u2: 4 / mpmath.pi * (1 - u2) ** 3,
}
SCALES = {'squareKilometre': 1e6, 'hectare': 1e4, 'decare': 1e3, 'squareMetre': 1.0,
          'kilometrePerSquareKilometre': 1e3, 'metrePerHectare': 1e4, 'metrePerSquareMetre': 1.0}


def kernel_at(xy, w, q, r, kernel, scale):
    r2 = r * r
    s = mpmath.mpf(0)
    for p, wi in zip(xy, w):
        dd = d2(p, q)
        if dd < r2:
            s += wi * KERNELS[kernel](mpmath.mpf(dd) / r2)
    return float(scale * s / r2)


def silverman(xy, w):
    total = mpmath.fsum(w)
    mx = mpmath.fsum(wi * p[0] for p, wi in zip(xy, w)) / total
    my = mpmath.fsum(wi * p[1] for p, wi in zip(xy, w)) / total
    sd = mpmath.sqrt(mpmath.fsum(wi * ((p[0] - mx) ** 2 + (p[1] - my) ** 2) for p, wi in zip(xy, w)) / total)
    dist = sorted((math.sqrt((p[0] - float(mx)) ** 2 + (p[1] - float(my)) ** 2), i, wi) for i, (p, wi) in enumerate(zip(xy, w)))
    acc, med = 0.0, 0.0
    for dd, _, wi in dist:
        acc += wi
        if acc >= float(total) / 2:
            med = dd
            break
    return float(mpmath.mpf('0.9') * min(sd, mpmath.sqrt(1 / mpmath.log(2)) * med) * mpmath.power(total, mpmath.mpf('-0.2')))


def seg_inside(a, b, q, r):
    ax, ay = mpmath.mpf(a[0]) - q[0], mpmath.mpf(a[1]) - q[1]
    dx, dy = mpmath.mpf(b[0]) - a[0], mpmath.mpf(b[1]) - a[1]
    l2 = dx * dx + dy * dy
    hb = ax * dx + ay * dy
    c = ax * ax + ay * ay - mpmath.mpf(r) ** 2
    disc = hb * hb - l2 * c
    if disc <= 0:
        return mpmath.mpf(0)
    root = mpmath.sqrt(disc)
    t0, t1 = max((-hb - root) / l2, 0), min((-hb + root) / l2, 1)
    return (t1 - t0) * mpmath.sqrt(l2) if t1 > t0 else mpmath.mpf(0)


def arc_inside(c, ra, a0, sweep, q, r):
    """The length of an arc inside a disc, by its angles (mpmath)."""
    dx, dy = mpmath.mpf(q[0]) - c[0], mpmath.mpf(q[1]) - c[1]
    dc = mpmath.sqrt(dx * dx + dy * dy)
    span = min(abs(mpmath.mpf(sweep)), 2 * mpmath.pi)
    if dc >= ra + r:
        return mpmath.mpf(0)
    if dc + ra <= r:
        return ra * span
    if dc + r <= ra or dc == 0:
        return mpmath.mpf(0)
    kappa = max(min((ra * ra + dc * dc - mpmath.mpf(r) ** 2) / (2 * ra * dc), 1), -1)
    alpha = mpmath.acos(kappa)
    phi = mpmath.atan2(dy, dx)
    start = mpmath.mpf(a0) if sweep >= 0 else mpmath.mpf(a0) + sweep
    total = mpmath.mpf(0)
    for k in range(-3, 4):
        s0 = phi - alpha + 2 * mpmath.pi * k
        o0, o1 = max(s0, start), min(s0 + 2 * alpha, start + span)
        if o1 > o0:
            total += o1 - o0
    return ra * min(total, span)


def bulge_arc(a, b, bulge):
    """A bulged segment's circle, start angle and sweep (DXF's rule)."""
    dx, dy = b[0] - a[0], b[1] - a[1]
    chord = mpmath.sqrt(mpmath.mpf(dx) ** 2 + mpmath.mpf(dy) ** 2)
    k = (1 - mpmath.mpf(bulge) ** 2) / (4 * mpmath.mpf(bulge))
    c = ((a[0] + b[0]) / mpmath.mpf(2) - dy * k, (a[1] + b[1]) / mpmath.mpf(2) + dx * k)
    r = chord * (1 + mpmath.mpf(bulge) ** 2) / (4 * abs(mpmath.mpf(bulge)))
    return c, r, mpmath.atan2(a[1] - c[1], a[0] - c[0]), 4 * mpmath.atan(bulge)


def edges_of(shape):
    """A line, polyline, polygon, arc or circle as segments and arcs."""
    k = shape['kind']
    if k == 'line':
        return [('seg', (shape['a']['x'], shape['a']['y']), (shape['b']['x'], shape['b']['y']))]
    if k == 'arc':
        a0, a1 = shape['a0'], shape['a1']
        sw = (a1 - a0) % (2 * math.pi)
        return [('arc', (shape['c']['x'], shape['c']['y']), shape['r'], a0, sw)]
    if k == 'circle':
        return [('arc', (shape['c']['x'], shape['c']['y']), shape['r'], 0.0, 2 * math.pi)]
    out = []

    def ring(pts, bulges, closed):
        n = len(pts)
        m = n if closed else n - 1
        for i in range(m):
            a = (pts[i]['x'], pts[i]['y'])
            b = (pts[(i + 1) % n]['x'], pts[(i + 1) % n]['y'])
            bu = (bulges or [0] * n)[i] if bulges and i < len(bulges) else 0
            if abs(bu) > 1e-12:
                c, r, a0, sw = bulge_arc(a, b, bu)
                out.append(('arc', c, r, a0, sw))
            else:
                out.append(('seg', a, b))
    ring(shape['pts'], shape.get('bulges'), k == 'polygon')
    for h in shape.get('holes') or []:
        ring(h['pts'], h.get('bulges'), True)
    return out


def line_at(edges, ws, q, r, scale):
    s = mpmath.mpf(0)
    for e, w in zip(edges, ws):
        if e[0] == 'seg':
            s += w * seg_inside(e[1], e[2], q, r)
        else:
            s += w * arc_inside(e[1], mpmath.mpf(e[2]), e[3], e[4], q, r)
    return float(scale * s / (mpmath.pi * mpmath.mpf(r) ** 2))


# ── GDAL's cross-checks ──────────────────────────────────────────────

def gdal_grid(pts, aff, width, height, algorithm):
    """GDAL's gdal_grid over the points (their values as Z), Float64."""
    drv = ogr.GetDriverByName('MEM')
    ds = drv.CreateDataSource('pts')
    lyr = ds.CreateLayer('p', geom_type=ogr.wkbPoint25D)
    for (x, y), v in zip(pts['xy'], pts['v']):
        f = ogr.Feature(lyr.GetLayerDefn())
        g = ogr.Geometry(ogr.wkbPoint25D)
        g.AddPoint(x, y, v)
        f.SetGeometry(g)
        lyr.CreateFeature(f)
    x0, s, _, y1, _, ms = aff
    out = gdal.Grid('/vsimem/grid.tif', ds, format='GTiff', outputType=gdal.GDT_Float64, width=width, height=height,
                    outputBounds=[x0, y1, x0 + width * s, y1 + height * ms], algorithm=algorithm)
    arr = out.GetRasterBand(1).ReadAsArray()
    out = None
    gdal.Unlink('/vsimem/grid.tif')
    return arr


# ── The cases ────────────────────────────────────────────────────────

def pt(x, y, z=None, **kw):
    d = {'kind': 'point', 'p': {'x': x, 'y': y}}
    if z is not None:
        d['z'] = z
    d.update(kw)
    return d


def surface(x, y):
    """A smooth hill with a valley, the cases' heights."""
    return 100.0 + 20.0 * math.sin(x / 37.0) * math.cos(y / 23.0) + 0.15 * x - 0.05 * y


def survey(rng, n, w, h, x0=0.0, y0=0.0, digits=3):
    out = []
    for _ in range(n):
        x = round(x0 + float(rng.random()) * w, digits)
        y = round(y0 + float(rng.random()) * h, digits)
        out.append(pt(x, y, round(surface(x - x0, y - y0), 3)))
    return out


def run_grid(aff, width, height, f):
    vals = []
    for j in range(height):
        for i in range(width):
            v = f(center(aff, i, j))
            vals.append(None if v is None else v)
    return vals


def cross_rows(pts, f):
    rows = []
    for i in range(len(pts['xy'])):
        got = f(i)
        rows.append(got)
    return rows


def summary(pts, rows):
    diffs = [(r[0] if isinstance(r, tuple) else r) - pts['v'][i] for i, r in enumerate(rows) if r is not None and (r[0] if isinstance(r, tuple) else r) is not None]
    n = len(diffs)
    out = {'count': n, 'missing': len(rows) - n}
    if n:
        out['mean'] = float(mpmath.fsum(diffs) / n)
        out['rmse'] = float(mpmath.sqrt(mpmath.fsum(d * d for d in diffs) / n))
        out['mae'] = float(mpmath.fsum(abs(d) for d in diffs) / n)
    return out


def build():
    rng = np.random.default_rng(2032)
    cases = []

    # The objects' points: kinds, parts, holes, elevations missing, a field's texts, equal places.
    sources = [
        pt(0.0, 0.0, 10.0),
        pt(5.0, 0.0, 12.0, parts=[{'p': {'x': 5.0, 'y': 4.0}, 'z': 13.0}, {'p': {'x': 6.0, 'y': 4.0}}]),
        {'kind': 'line', 'a': {'x': 0.0, 'y': 4.0}, 'b': {'x': 2.0, 'y': 6.0}, 'za': 11.0},
        {'kind': 'polyline', 'pts': [{'x': 8.0, 'y': 0.0}, {'x': 9.0, 'y': 2.0}, {'x': 5.0, 'y': 4.0}], 'zs': [14.0, None, 15.0]},
        {'kind': 'polygon', 'pts': [{'x': 10.0, 'y': 10.0}, {'x': 20.0, 'y': 10.0}, {'x': 20.0, 'y': 20.0}],
         'zs': [20.0, 21.0, 22.0], 'holes': [{'pts': [{'x': 14.0, 'y': 12.0}, {'x': 16.0, 'y': 12.0}, {'x': 15.0, 'y': 14.0}], 'zs': [18.0, None, 19.0]}],
         'parts': [{'pts': [{'x': 30.0, 'y': 30.0}, {'x': 31.0, 'y': 30.0}, {'x': 30.0, 'y': 31.0}], 'zs': [1.0, 2.0, 3.0]}]},
        {'kind': 'text', 'p': {'x': 3.0, 'y': 3.0}, 'text': 'yok'},
        pt(0.0, 0.0, 16.0),
    ]
    texts = ['1,5', ' 2 ', 'abc', '3.25', '-4', 'yazı', None]
    cases.append({'name': 'noktalar-kot', 'kind': 'gather', 'sources': sources, 'values': None,
                  'expect': gather(sources, None)})
    cases.append({'name': 'noktalar-alan', 'kind': 'gather', 'sources': sources, 'values': texts,
                  'expect': gather(sources, texts)})

    # The grid's rule.
    for name, b, cell in (('izgara-otomatik', [500000.123, 4420000.9, 500100.0, 4420050.0], 0.0),
                          ('izgara-verilen', [12.5, 7.25, 73.0, 41.0], 2.5),
                          ('izgara-dar', [0.0, 0.0, 1000.0, 3.7], 0.0),
                          ('izgara-dogru', [0.0, 5.0, 80.0, 5.0], 0.0)):
        aff, w, h = grid_of_box(b, cell)
        # ADR 0231 §2's limits: 65 536 wide, 2³¹ cells.
        if w > 65536 or w * h > 2 ** 31:
            cases.append({'name': name, 'kind': 'grid', 'box': b, 'cell': cell, 'expect': {'error': 'too_big'}})
        else:
            cases.append({'name': name, 'kind': 'grid', 'box': b, 'cell': cell, 'expect': {'affine': aff, 'width': w, 'height': h}})
    for s in (3.7, 2.5, 0.83, 12.0, 0.24, 0.196, 999.0, 1.0, 0.0499):
        cases.append({'name': f'hucre-{s}', 'kind': 'nice', 'size': s, 'expect': nice_cell(s)})

    # Survey points over 120 × 80 m at TM coordinates, heights of the hill.
    base = survey(rng, 40, 120.0, 80.0, 500000.0, 4420000.0)
    pts = gather(base, None)
    aff, width, height = grid_of_box(bounds(pts['xy']), 5.0)

    def add(name, tool, f, cross=None, cell=5.0, extra=None, src=None, values=None, grid=None, tol=None):
        c = {'name': name, 'kind': 'surface', 'sources': src or base, 'values': values, 'tool': tool, 'cell': cell,
             'expect': {'affine': aff if grid is None else grid['affine'], 'width': width if grid is None else grid['width'],
                        'height': height if grid is None else grid['height'], 'values': f}}
        if grid is not None:
            c['grid'] = grid
        if cross is not None:
            c['expect']['cross'] = cross
            c['expect']['summary'] = summary(pts, cross)
        if extra:
            c['expect'].update(extra)
        if tol:
            c['tolerance'] = tol
        cases.append(c)

    # Ters uzaklık: the defaults; a radius with a least count; a power of 1.5.
    vals = run_grid(aff, width, height, lambda q: idw(pts, q, 2.0, 12, 0.0, 1))
    gd = gdal_grid(pts, aff, width, height, 'invdistnn:power=2.0:smoothing=0.0:radius=1e9:max_points=12:min_points=1')
    for k, v in enumerate(vals):
        j, i = divmod(k, width)
        assert abs(gd[j][i] - v) <= 1e-9 * max(1.0, abs(v)), f'GDAL invdistnn differs at {i},{j}: {gd[j][i]} {v}'
    cross = cross_rows(pts, lambda i: idw(pts, pts['xy'][i], 2.0, 12, 0.0, 1, skip=i))
    add('idw', {'kind': 'idw', 'power': 2.0, 'points': 12}, vals, cross)
    vals = run_grid(aff, width, height, lambda q: idw(pts, q, 2.0, 8, 15.0, 3))
    cross = cross_rows(pts, lambda i: idw(pts, pts['xy'][i], 2.0, 8, 15.0, 3, skip=i))
    add('idw-yaricap', {'kind': 'idw', 'power': 2.0, 'points': 8, 'radius': 15.0, 'minPoints': 3}, vals, cross)
    vals = run_grid(aff, width, height, lambda q: idw(pts, q, 1.5, 6, 0.0, 1))
    add('idw-us', {'kind': 'idw', 'power': 1.5, 'points': 6}, vals)

    # TIN'den raster.
    tris = qhull_triangles(pts['xy'])
    vals = run_grid(aff, width, height, lambda q: tin(pts['xy'], pts['v'], tris, q))
    gd = gdal_grid(pts, aff, width, height, 'linear:radius=0.0:nodata=-9999')
    for k, v in enumerate(vals):
        j, i = divmod(k, width)
        if v is None:
            assert gd[j][i] == -9999, f'GDAL linear has a value outside the hull at {i},{j}'
        else:
            assert abs(gd[j][i] - v) <= 1e-9 * max(1.0, abs(v)), f'GDAL linear differs at {i},{j}: {gd[j][i]} {v}'

    def tin_out(i):
        others = [p for k, p in enumerate(pts['xy']) if k != i]
        ov = [v for k, v in enumerate(pts['v']) if k != i]
        return tin(others, ov, qhull_triangles(others), pts['xy'][i])
    add('tin', {'kind': 'tin'}, vals, cross_rows(pts, tin_out))

    # Doğal komşu: fewer cells (exact clipping is slow), the same points.
    nn_aff, nn_w, nn_h = grid_of_box(bounds(pts['xy']), 10.0)
    vals = run_grid(nn_aff, nn_w, nn_h, lambda q: sibson(pts['xy'], pts['v'], q))

    def nn_out(i):
        others = [p for k, p in enumerate(pts['xy']) if k != i]
        ov = [v for k, v in enumerate(pts['v']) if k != i]
        return sibson(others, ov, pts['xy'][i])
    cases.append({'name': 'dogal-komsu', 'kind': 'surface', 'sources': base, 'values': None, 'tool': {'kind': 'naturalNeighbor'},
                  'cell': 10.0, 'expect': {'affine': nn_aff, 'width': nn_w, 'height': nn_h, 'values': vals,
                                           'cross': (lambda c: c)(cross_rows(pts, nn_out))}})
    cases[-1]['expect']['summary'] = summary(pts, cases[-1]['expect']['cross'])

    # Spline: regularized (0.1), thin plate (0), tension (0.1).
    for name, kind, weight in (('spline', 'regularized', 0.1), ('spline-ince-plaka', 'regularized', 0.0), ('spline-gerilimli', 'tension', 0.1)):
        cache = {}
        vals = run_grid(aff, width, height, lambda q: spline_at(pts, q, kind, weight, 12, cache=cache))
        cross = cross_rows(pts, lambda i: spline_at(pts, pts['xy'][i], kind, weight, 12, skip=i, cache=cache))
        add(name, {'kind': 'spline', 'spline': kind, 'weight': weight, 'points': 12}, vals, cross)

    # Kriging: given variograms (with the error band) and a fitted one.
    for name, model, ng, sl, rg in (('kriging-kuresel', 'spherical', 0.5, 120.0, 90.0),
                                    ('kriging-ustel', 'exponential', 0.0, 100.0, 150.0),
                                    ('kriging-gauss', 'gaussian', 1.0, 80.0, 60.0)):
        g = {'model': model, 'nugget': mpmath.mpf(ng), 'sill': mpmath.mpf(sl), 'range': mpmath.mpf(rg)}
        cache = {}
        res = [kriging_at(pts, center(aff, i, j), g, 12, 0.0, cache=cache) for j in range(height) for i in range(width)]
        cross = [kriging_at(pts, pts['xy'][i], g, 12, 0.0, skip=i) for i in range(len(pts['xy']))]
        tool = {'kind': 'kriging', 'model': model, 'variogram': {'fit': 'manual', 'nugget': ng, 'sill': sl, 'range': rg},
                'points': 12, 'error': True}
        c = {'name': name, 'kind': 'surface', 'sources': base, 'values': None, 'tool': tool, 'cell': 5.0,
             'expect': {'affine': aff, 'width': width, 'height': height, 'values': [r[0] for r in res], 'error': [r[1] for r in res],
                        'cross': [list(r) if r[0] is not None else None for r in cross]}}
        diffs = [(r[0] - pts['v'][i], r[1]) for i, r in enumerate(cross) if r[0] is not None]
        sm = summary(pts, [r[0] for r in cross])
        zs = [d / e for d, e in diffs if e and e > 0]
        if zs:
            sm['stdMean'] = float(mpmath.fsum(zs) / len(zs))
            sm['stdRmse'] = float(mpmath.sqrt(mpmath.fsum(z * z for z in zs) / len(zs)))
        c['expect']['summary'] = sm
        cases.append(c)
    # The fit on the hill's points runs to the search's end (2L): the trend's variogram never levels off.
    fitted = fit_variogram(pts['xy'], pts['v'], 'spherical', 12)
    cases.append({'name': 'variogram-egilimli', 'kind': 'fit', 'sources': base, 'model': 'spherical', 'lags': 12, 'expect': fitted})
    # A stationary field (bumps 15 m wide over 200 m): its variogram levels off inside the search.
    bumps = [(float(x), float(y), float(a)) for x, y, a in zip(rng.random(30) * 200.0, rng.random(30) * 200.0, rng.normal(0.0, 5.0, 30))]

    def field_at(x, y):
        return 50.0 + sum(a * math.exp(-((x - bx) ** 2 + (y - by) ** 2) / (2 * 15.0 ** 2)) for bx, by, a in bumps)
    flat = []
    for x, y in rng.random((120, 2)) * 200.0:
        x, y = round(float(x), 2), round(float(y), 2)
        flat.append(pt(x, y, round(field_at(x, y), 3)))
    fpts0 = gather(flat, None)
    for name, model, lags in (('variogram-kuresel', 'spherical', 12), ('variogram-gauss', 'gaussian', 10), ('variogram-ustel', 'exponential', 15)):
        cases.append({'name': name, 'kind': 'fit', 'sources': flat, 'model': model, 'lags': lags,
                      'expect': fit_variogram(fpts0['xy'], fpts0['v'], model, lags)})
    g = fit_variogram(fpts0['xy'], fpts0['v'], 'spherical', 12)
    gm = {'model': 'spherical', 'nugget': mpmath.mpf(g['nugget']), 'sill': mpmath.mpf(g['sill']), 'range': mpmath.mpf(g['range'])}
    kaff, kw, kh = grid_of_box(bounds(fpts0['xy']), 10.0)
    cache = {}
    res = [kriging_at(fpts0, center(kaff, i, j), gm, 12, 0.0, cache=cache) for j in range(kh) for i in range(kw)]
    cases.append({'name': 'kriging-otomatik', 'kind': 'surface', 'sources': flat, 'values': None,
                  'tool': {'kind': 'kriging', 'model': 'spherical', 'variogram': {'fit': 'auto', 'lags': 12}, 'points': 12},
                  'cell': 10.0, 'expect': {'affine': kaff, 'width': kw, 'height': kh, 'values': [r[0] for r in res],
                                           'variogram': {'nugget': g['nugget'], 'sill': g['sill'], 'range': g['range']}}})

    # A field's values and the grid of a turned raster (IDW).
    field = [str(round(float(rng.random()) * 50.0, 2)).replace('.', ',') for _ in base]
    fpts = gather(base, field)
    turned = {'affine': [500010.0, 4.0, 1.5, 4420075.0, 1.0, -4.5], 'width': 18, 'height': 14}
    vals = [idw(fpts, center(turned['affine'], i, j), 2.0, 12, 0.0, 1) for j in range(turned['height']) for i in range(turned['width'])]
    add('idw-alan-donuk-izgara', {'kind': 'idw', 'power': 2.0, 'points': 12}, vals, values=field, grid=turned)

    # Çekirdek yoğunluğu: five kernels at a radius; the automatic radius with weights.
    dens = [pt(round(500000.0 + float(x), 2), round(4420000.0 + float(y), 2)) for x, y in rng.random((60, 2)) * 200.0]
    dxy = [(d['p']['x'], d['p']['y']) for d in dens]
    for kernel in KERNELS:
        r = 30.0
        b = bounds(dxy)
        daff, dw, dh = grid_of_box([b[0] - r, b[1] - r, b[2] + r, b[3] + r], 10.0)
        vals = run_grid(daff, dw, dh, lambda q: kernel_at(dxy, [1.0] * len(dxy), q, r, kernel, 1e6))
        cases.append({'name': f'cekirdek-{kernel}', 'kind': 'surface', 'sources': dens, 'values': None,
                      'tool': {'kind': 'kernel', 'radius': r, 'kernel': kernel, 'unit': 'squareKilometre'}, 'cell': 10.0,
                      'expect': {'affine': daff, 'width': dw, 'height': dh, 'values': vals, 'radius': r}})
    weights = [str(1 + k % 4) for k in range(len(dens))]
    ww = [float(1 + k % 4) for k in range(len(dens))]
    r = silverman(dxy, ww)
    b = bounds(dxy)
    daff, dw, dh = grid_of_box([b[0] - r, b[1] - r, b[2] + r, b[3] + r], 10.0)
    vals = run_grid(daff, dw, dh, lambda q: kernel_at(dxy, ww, q, r, 'quartic', 1e4))
    cases.append({'name': 'cekirdek-otomatik-agirlikli', 'kind': 'surface', 'sources': dens, 'values': weights,
                  'tool': {'kind': 'kernel', 'radius': 0.0, 'kernel': 'quartic', 'unit': 'hectare'}, 'cell': 10.0,
                  'expect': {'affine': daff, 'width': dw, 'height': dh, 'values': vals, 'radius': r}})

    # Çizgi yoğunluğu: lines, a bulged polyline, an area with a hole, an arc and a circle.
    shapes = [
        {'kind': 'line', 'a': {'x': 0.0, 'y': 0.0}, 'b': {'x': 100.0, 'y': 30.0}},
        {'kind': 'polyline', 'pts': [{'x': 10.0, 'y': 60.0}, {'x': 50.0, 'y': 70.0}, {'x': 90.0, 'y': 55.0}], 'bulges': [0.0, 0.4]},
        {'kind': 'polygon', 'pts': [{'x': 20.0, 'y': 10.0}, {'x': 45.0, 'y': 12.0}, {'x': 40.0, 'y': 40.0}],
         'holes': [{'pts': [{'x': 30.0, 'y': 18.0}, {'x': 36.0, 'y': 20.0}, {'x': 33.0, 'y': 26.0}]}]},
        {'kind': 'arc', 'c': {'x': 70.0, 'y': 20.0}, 'r': 12.0, 'a0': 0.3, 'a1': 2.5},
        {'kind': 'circle', 'c': {'x': 25.0, 'y': 85.0}, 'r': 7.5},
    ]
    edges, ews = [], []
    lweights = ['1', '2', '0,5', '1', '3']
    for s, w in zip(shapes, lweights):
        for e in edges_of(s):
            edges.append(e)
            ews.append(read_number(w))
    for name, radius, weights_used, unit in (('cizgi', 12.0, None, 'kilometrePerSquareKilometre'),
                                             ('cizgi-agirlikli-otomatik', 0.0, lweights, 'metrePerHectare')):
        ws = ews if weights_used else [1.0] * len(edges)
        boxes = []
        for e in edges:
            if e[0] == 'seg':
                boxes.append([min(e[1][0], e[2][0]), min(e[1][1], e[2][1]), max(e[1][0], e[2][0]), max(e[1][1], e[2][1])])
            else:
                c, rr = e[1], float(e[2])
                boxes.append([float(c[0]) - rr, float(c[1]) - rr, float(c[0]) + rr, float(c[1]) + rr])
        b = [min(x[0] for x in boxes), min(x[1] for x in boxes), max(x[2] for x in boxes), max(x[3] for x in boxes)]
        r = radius if radius > 0 else min(b[2] - b[0], b[3] - b[1]) / 30.0
        laff, lw, lh = grid_of_box([b[0] - r, b[1] - r, b[2] + r, b[3] + r], 4.0)
        vals = run_grid(laff, lw, lh, lambda q: line_at(edges, ws, q, r, SCALES[unit]))
        cases.append({'name': name, 'kind': 'lines', 'shapes': shapes, 'values': weights_used,
                      'tool': {'kind': 'lineDensity', 'radius': radius, 'unit': unit}, 'cell': 4.0,
                      'expect': {'affine': laff, 'width': lw, 'height': lh, 'values': vals, 'radius': r}})
    return {'format': 'kentos.interpolation', 'version': 1, 'cases': cases}


def main():
    data = build()
    text = json.dumps(data, ensure_ascii=False, indent=1) + '\n'
    if '--check' in sys.argv:
        with open(OUT, encoding='utf-8') as f:
            if f.read() != text:
                print(f'{OUT} differs from the reference', file=sys.stderr)
                sys.exit(1)
        print(f'{OUT} matches')
        return
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, 'w', encoding='utf-8') as f:
        f.write(text)
    print(f'{OUT} written')


if __name__ == '__main__':
    main()
