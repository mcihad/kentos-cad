#!/usr/bin/env python3
"""Independent reference of the measures (docs/adr/0149 §3).

Writes fixtures/measure/v1/cases.json with 50-digit arithmetic (mpmath) and
no KentOS code: for objects and point pairs at Transverse Mercator
coordinates and near the origin, the exact length, area, distance, bearing,
angle, slope and dimension value, and how the display rule writes each with
0 to 6 decimals. The geometry core (crates/shared/geometry-core/tests/all/measure.rs)
and the web through its WASM (apps/web/src/wasm/measure.wasm.test.ts) call
the same operations by name and must agree within each case's bound and
write the very same text.

The definitions, each from the geometry and not from KentOS's formulas:

- Every input float is taken exactly (mpmath's mpf of a float is exact).
- A straight edge from a to b has length |b − a|.
- An edge with bulge β is an arc of sweep θ = 4·atan(β) (counter-clockwise
  for β > 0). Its centre is the chord's middle plus the chord's left normal
  times (L/2)·cot(θ/2), its radius (L/2)/|sin(θ/2)|, L the chord's length;
  its length is radius·|θ|.
- An area is ½∮(x dy − y dx) over its rings (Green's theorem): a straight
  edge gives ½(x_a·y_b − x_b·y_a); an arc of centre c, radius r, from angle
  φ through θ gives ½(r²θ + r·c_x·(sin(φ+θ) − sin φ) − r·c_y·(cos(φ+θ) − cos φ)).
  A polygon's area is its outer ring's area in size less its holes' in
  size; a multi-part area sums its parts. Its perimeter is the length of
  every ring.
- A circle: 2πr and πr². An arc entity runs counter-clockwise from a0 to
  a1; its sweep is (a1 − a0) mod 2π, a whole turn when that is 0.
- An ellipse: C(t) = c + M·cos t + N·sin t, N the major axis M turned a
  quarter counter-clockwise times the ratio; its length ∫|C′(t)|dt from t0
  over the sweep (t1 − t0 mod 2π, a whole turn when 0); a whole ellipse's
  area π·|M|²·ratio.
- A curve through fit points: the centripetal Catmull-Rom curve (Barry and
  Goldman's pyramid, knots spaced by the square root of the distance), the
  open curve's phantom ends the mirror of the second point through the
  first (and of the second last through the last); its length ∫|C′(t)|dt
  span by span.
- A distance |b − a|; a bearing (semt) clockwise from grid north,
  atan2(Δx, Δy) in grads within [0, 400); a direction angle atan2(Δy, Δx)
  in degrees; a length in space Σ√(plan² + rise²) edge by edge.
- Dimension values: aligned |b − a|; linear |(b − a)·(cos α, sin α)|, α the
  measured direction in degrees; angular the counter-clockwise angle at c
  from the direction of a to that of b; radius |b − a| (a the centre),
  diameter twice that; ordinate a's east (Y) or north (X); arc length the
  radius |a − c| times the counter-clockwise angle at c from a to b; jogged
  radius |b − a|; azimuth the bearing from a to b in radians within
  [0, 2π); slope |zb − za| / |b − a| · 100.

An angle is written as the app writes it: in grads (radians · 200/π).

    python3 scripts/fixtures/measure_cases.py           # write the fixture
    python3 scripts/fixtures/measure_cases.py --check   # compare
"""

import json
import random
import sys
from decimal import ROUND_HALF_UP, Decimal, getcontext
from pathlib import Path

from mpmath import atan, atan2, cos, cot, fabs, mp, mpf, pi, quad, sin, sqrt

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/measure/v1/cases.json"
mp.dps = 50
getcontext().prec = 120
TM = (487000.0, 4420000.0)

# Bounds: what the computed value may differ from the exact one by (abs + rel·|exact|).
LENGTH = (1e-9, 1e-12)
AREA = (1e-6, 1e-12)
ANGLE = (1e-12, 1e-12)
PERCENT = (1e-10, 1e-12)


def shown(exact, d: int) -> str:
    """The display rule (docs/adr/0149) on an exact value: 7 decimals, then d, a half away from zero."""
    x = Decimal(mp.nstr(fabs(exact), 45, strip_zeros=False, min_fixed=-mp.inf, max_fixed=mp.inf))
    if d >= 7:
        y = x.quantize(Decimal(1).scaleb(-d), rounding=ROUND_HALF_UP)
    else:
        y = x.quantize(Decimal(1).scaleb(-7), rounding=ROUND_HALF_UP).quantize(Decimal(1).scaleb(-d), rounding=ROUND_HALF_UP)
    text = format(y, "f")
    if exact < 0 and any(c in "123456789" for c in text):
        text = "-" + text
    return text


def P(p):
    return (mpf(p["x"]), mpf(p["y"]))


def v(x, y):
    return {"x": float(x), "y": float(y)}


# --- edges -------------------------------------------------------------------


def arc_of(a, b, beta):
    """An arc edge from a to b with bulge β: centre, radius, start angle, sweep."""
    beta = mpf(beta)
    theta = 4 * atan(beta)
    dx, dy = b[0] - a[0], b[1] - a[1]
    chord = sqrt(dx * dx + dy * dy)
    h = (chord / 2) * cot(theta / 2)
    c = ((a[0] + b[0]) / 2 - dy / chord * h, (a[1] + b[1]) / 2 + dx / chord * h)
    r = (chord / 2) / fabs(sin(theta / 2))
    phi = atan2(a[1] - c[1], a[0] - c[0])
    return c, r, phi, theta


def edges(pts, bulges, closed):
    n = len(pts)
    count = n if closed else n - 1
    out = []
    for i in range(count):
        a, b = P(pts[i]), P(pts[(i + 1) % n])
        beta = (bulges or [0] * n)[i] if bulges else 0
        out.append((a, b, beta))
    return out


def edge_length(a, b, beta):
    if beta == 0:
        return sqrt((b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2)
    _, r, _, theta = arc_of(a, b, beta)
    return r * fabs(theta)


def edge_green(a, b, beta):
    """½∮(x dy − y dx) along one edge."""
    if beta == 0:
        return (a[0] * b[1] - b[0] * a[1]) / 2
    c, r, phi, theta = arc_of(a, b, beta)
    return (r * r * theta + r * c[0] * (sin(phi + theta) - sin(phi)) - r * c[1] * (cos(phi + theta) - cos(phi))) / 2


def path_length(pts, bulges, closed):
    return sum((edge_length(*e) for e in edges(pts, bulges, closed)), mpf(0))


def ring_area(pts, bulges):
    return fabs(sum((edge_green(*e) for e in edges(pts, bulges, True)), mpf(0)))


def polygon_area(e):
    def one(part):
        return ring_area(part["pts"], part.get("bulges")) - sum((ring_area(h["pts"], h.get("bulges")) for h in part.get("holes") or []), mpf(0))

    return one(e) + sum((one(p) for p in e.get("parts") or []), mpf(0))


def polygon_perimeter(e):
    def one(part):
        return path_length(part["pts"], part.get("bulges"), True) + sum((path_length(h["pts"], h.get("bulges"), True) for h in part.get("holes") or []), mpf(0))

    return one(e) + sum((one(p) for p in e.get("parts") or []), mpf(0))


# --- curves ------------------------------------------------------------------


def turn(a0, a1):
    s = (mpf(a1) - mpf(a0)) % (2 * pi)
    return 2 * pi if s == 0 else s


def ellipse_length(e):
    c = P(e["c"])
    m = P(e["major"])
    nx, ny = -m[1] * mpf(e["ratio"]), m[0] * mpf(e["ratio"])
    sweep = turn(e["t0"], e["t1"])

    def speed(t):
        return sqrt((-m[0] * sin(t) + nx * cos(t)) ** 2 + (-m[1] * sin(t) + ny * cos(t)) ** 2)

    t0 = mpf(e["t0"])
    cuts = [t0 + sweep * k / 8 for k in range(9)]
    return quad(speed, cuts)


def catmull_spans(pts, closed):
    q = [P(p) for p in pts]
    n = len(q)

    def at(i):
        if closed:
            return q[i % n]
        if i < 0:
            return (2 * q[0][0] - q[1][0], 2 * q[0][1] - q[1][1])
        if i >= n:
            return (2 * q[n - 1][0] - q[n - 2][0], 2 * q[n - 1][1] - q[n - 2][1])
        return q[i]

    spans = n if closed else n - 1
    for i in range(spans):
        yield at(i - 1), at(i), at(i + 1), at(i + 2)


def catmull_length(pts, closed):
    total = mpf(0)
    for p0, p1, p2, p3 in catmull_spans(pts, closed):
        def knot(a, b):
            return sqrt(sqrt((b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2))

        t0 = mpf(0)
        t1 = t0 + knot(p0, p1)
        t2 = t1 + knot(p1, p2)
        t3 = t2 + knot(p2, p3)

        def lerp(a, b, ta, tb, t):
            u, w = (tb - t) / (tb - ta), (t - ta) / (tb - ta)
            return (a[0] * u + b[0] * w, a[1] * u + b[1] * w)

        def dlerp(a, b, da, db, ta, tb, t):
            # d/dt of lerp(A(t), B(t)) with A' = da, B' = db.
            u, w = (tb - t) / (tb - ta), (t - ta) / (tb - ta)
            k = 1 / (tb - ta)
            return (da[0] * u + db[0] * w + (b[0] - a[0]) * k, da[1] * u + db[1] * w + (b[1] - a[1]) * k)

        def speed(t):
            zero = (mpf(0), mpf(0))
            a1, a2, a3 = lerp(p0, p1, t0, t1, t), lerp(p1, p2, t1, t2, t), lerp(p2, p3, t2, t3, t)
            da1, da2, da3 = dlerp(p0, p1, zero, zero, t0, t1, t), dlerp(p1, p2, zero, zero, t1, t2, t), dlerp(p2, p3, zero, zero, t2, t3, t)
            b1, b2 = lerp(a1, a2, t0, t2, t), lerp(a2, a3, t1, t3, t)
            db1, db2 = dlerp(a1, a2, da1, da2, t0, t2, t), dlerp(a2, a3, da2, da3, t1, t3, t)
            d = dlerp(b1, b2, db1, db2, t1, t2, t)
            return sqrt(d[0] ** 2 + d[1] ** 2)

        total += quad(speed, [t1, (t1 + t2) / 2, t2])
    return total


# --- cases -------------------------------------------------------------------


def case(what, op, args, unit, exact, bound, pick=None):
    shown_value = exact * 200 / pi if unit == "angle" else exact
    out = {
        "what": what,
        "op": op,
        "args": args,
        "unit": unit,
        "exact": mp.nstr(exact, 40, strip_zeros=False, min_fixed=-mp.inf, max_fixed=mp.inf),
        "abs": bound[0],
        "rel": bound[1],
        "shown": [shown(shown_value, d) for d in range(7)],
    }
    if pick:
        out["pick"] = pick
    return out


def at(origin, x, y):
    return v(origin[0] + x, origin[1] + y)


def hand(origin, name):
    o = origin
    c = []
    line = {"kind": "line", "a": at(o, 0.1234, 0.5678), "b": at(o, 30.0001, 40.0002)}
    c.append(case(f"{name}: çizginin uzunluğu", "entityLength", [line], "length", sqrt((mpf(line["b"]["x"]) - mpf(line["a"]["x"])) ** 2 + (mpf(line["b"]["y"]) - mpf(line["a"]["y"])) ** 2), LENGTH))
    poly = {"kind": "polyline", "pts": [at(o, 0, 0), at(o, 10, 0), at(o, 10, 10), at(o, 25.5, 12.25)], "bulges": [0, 0.41421356237, -1.0, 0]}
    c.append(case(f"{name}: yaylı çoklu çizginin uzunluğu", "entityLength", [poly], "length", path_length(poly["pts"], poly["bulges"], False), LENGTH))
    flat = {"kind": "polyline", "pts": [at(o, 0, 0), at(o, 100, 0)], "bulges": [1e-7, 0]}
    c.append(case(f"{name}: çok basık yayın uzunluğu", "entityLength", [flat], "length", path_length(flat["pts"], flat["bulges"], False), LENGTH))
    big = {"kind": "polyline", "pts": [at(o, 0, 0), at(o, 1, 0)], "bulges": [8.0, 0]}
    c.append(case(f"{name}: tam çembere yakın yayın uzunluğu", "entityLength", [big], "length", path_length(big["pts"], big["bulges"], False), LENGTH))
    parcel = {
        "kind": "polygon",
        "pts": [at(o, 0, 0), at(o, 40.125, 0), at(o, 40.125, 30.0625), at(o, 20, 35), at(o, 0, 30)],
        "bulges": [0, 0, 0.25, 0, 0],
        "holes": [{"pts": [at(o, 5, 5), at(o, 5, 10), at(o, 10, 10), at(o, 10, 5)]}],
    }
    c.append(case(f"{name}: yaylı, delikli alanın alanı", "entityArea", [parcel], "area", polygon_area(parcel), AREA))
    c.append(case(f"{name}: yaylı, delikli alanın çevresi", "entityLength", [parcel], "length", polygon_perimeter(parcel), LENGTH))
    multi = dict(parcel)
    multi["parts"] = [{"pts": [at(o, 100, 0), at(o, 112.5, 0), at(o, 112.5, 8), at(o, 100, 8)], "bulges": [0, -0.5, 0, 0]}]
    c.append(case(f"{name}: çok parçalı alanın alanı", "entityArea", [multi], "area", polygon_area(multi), AREA))
    c.append(case(f"{name}: çok parçalı alanın çevresi", "entityLength", [multi], "length", polygon_perimeter(multi), LENGTH))
    sliver = {"kind": "polygon", "pts": [at(o, 0, 0), at(o, 1000, 0.001), at(o, 2000, 0)]}
    c.append(case(f"{name}: ince şeridin alanı", "entityArea", [sliver], "area", polygon_area(sliver), AREA))
    for r in [12.125, 5.0005, 7.8465123, 0.001, 2500.0]:
        circle = {"kind": "circle", "c": at(o, 3, 4), "r": r}
        c.append(case(f"{name}: r={r} çemberin çevresi", "entityLength", [circle], "length", 2 * pi * mpf(r), LENGTH))
        c.append(case(f"{name}: r={r} çemberin alanı", "entityArea", [circle], "area", pi * mpf(r) ** 2, AREA))
        dim = {"a": at(o, 3, 4), "b": v(mpf(o[0] + 3) + mpf(r) * cos(mpf(0.7)), mpf(o[1] + 4) + mpf(r) * sin(mpf(0.7))), "offset": 0.0, "height": 2.5, "style": "radius"}
        exact_r = sqrt((mpf(dim["b"]["x"]) - mpf(dim["a"]["x"])) ** 2 + (mpf(dim["b"]["y"]) - mpf(dim["a"]["y"])) ** 2)
        c.append(case(f"{name}: r={r} yarıçap ölçüsü", "layoutDimension", [dim], "length", exact_r, LENGTH, "value"))
        dia = dict(dim, style="diameter")
        c.append(case(f"{name}: r={r} çap ölçüsü", "layoutDimension", [dia], "length", 2 * exact_r, LENGTH, "value"))
    arc = {"kind": "arc", "c": at(o, 0, 0), "r": 15.5, "a0": 5.5, "a1": 0.25}
    c.append(case(f"{name}: sıfırdan geçen yayın uzunluğu", "entityLength", [arc], "length", mpf(15.5) * turn(5.5, 0.25), LENGTH))
    for ratio, t0, t1 in [(1.0, 0.0, 0.0), (0.5, 0.0, 0.0), (0.1, 0.0, 0.0), (0.01, 0.0, 0.0), (0.6, 0.3, 2.1), (0.2, 5.0, 1.0)]:
        e = {"kind": "ellipse", "c": at(o, 1, 2), "major": v(30.0, 17.5), "ratio": ratio, "t0": t0, "t1": t1}
        c.append(case(f"{name}: oranı {ratio}, {t0}–{t1} elipsin uzunluğu", "entityLength", [e], "length", ellipse_length(e), LENGTH))
        if t0 == t1:
            c.append(case(f"{name}: oranı {ratio} elipsin alanı", "entityArea", [e], "area", pi * (mpf(30.0) ** 2 + mpf(17.5) ** 2) * mpf(ratio), AREA))
    for closed in [False, True]:
        s = {"kind": "spline", "pts": [at(o, 0, 0), at(o, 10, 8), at(o, 25, 5), at(o, 32, 20), at(o, 20, 30)], "closed": closed}
        c.append(case(f"{name}: {'kapalı' if closed else 'açık'} eğrinin uzunluğu", "entityLength", [s], "length", catmull_length(s["pts"], closed), LENGTH))
    a, b = at(o, 12.3456, 7.8912), at(o, -45.6789, 123.4567)
    pa, pb = P(a), P(b)
    dx, dy = pb[0] - pa[0], pb[1] - pa[1]
    c.append(case(f"{name}: iki nokta arası uzaklık", "dist", [a, b], "length", sqrt(dx * dx + dy * dy), LENGTH))
    c.append(case(f"{name}: semt (grad)", "bearingGrad", [a, b], "grad", (atan2(dx, dy) * 200 / pi) % 400, ANGLE))
    c.append(case(f"{name}: doğrultu açısı (derece)", "angleDeg", [a, b], "deg", atan2(dy, dx) * 180 / pi, ANGLE))
    zs = [100.125, 101.5, None, 99.75]
    path = [at(o, 0, 0), at(o, 10, 0), at(o, 10, 10), at(o, 0, 10)]
    exact3 = mpf(0)
    plan = [edge_length(*e) for e in edges(path, None, False)]
    full = all(z is not None for z in zs)
    if full:
        exact3 = sum(sqrt(p * p + (mpf(zs[i + 1]) - mpf(zs[i])) ** 2) for i, p in enumerate(plan))
    zs2 = [100.125, 101.5, 101.0, 99.75]
    exact3 = sum((sqrt(p * p + (mpf(zs2[i + 1]) - mpf(zs2[i])) ** 2) for i, p in enumerate(plan)), mpf(0))
    c.append(case(f"{name}: köşe kotlu çoklu çizginin 3B uzunluğu", "length3d", [path, None, False, zs2], "length", exact3, LENGTH))
    # The dimension kinds' values.
    pa, pb = at(o, 2.5, 3.25), at(o, 42.125, 21.0625)
    A, B = P(pa), P(pb)
    dx, dy = B[0] - A[0], B[1] - A[1]
    c.append(case(f"{name}: hizalı ölçü", "layoutDimension", [{"a": pa, "b": pb, "offset": 3.0, "height": 2.5}], "length", sqrt(dx * dx + dy * dy), LENGTH, "value"))
    for alpha in [0.0, 90.0, 30.0]:
        al = mpf(alpha) * pi / 180
        c.append(case(f"{name}: {alpha}° doğrusal ölçü", "layoutDimension", [{"a": pa, "b": pb, "offset": 3.0, "height": 2.5, "style": "linear", "angle": alpha}], "length", fabs(dx * cos(al) + dy * sin(al)), LENGTH, "value"))
    cv = at(o, 0, 0)
    C = P(cv)
    ang = (atan2(B[1] - C[1], B[0] - C[0]) - atan2(A[1] - C[1], A[0] - C[0])) % (2 * pi)
    c.append(case(f"{name}: açı ölçüsü", "layoutDimension", [{"a": pa, "b": pb, "offset": 10.0, "height": 2.5, "style": "angular", "c": cv}], "angle", ang, ANGLE, "value"))
    c.append(case(f"{name}: koordinat ölçüsü (Y)", "layoutDimension", [{"a": pa, "b": at(o, 2.5, 20.0), "offset": 0.0, "height": 2.5, "style": "ordinate", "angle": 0.0}], "length", A[0], LENGTH, "value"))
    c.append(case(f"{name}: koordinat ölçüsü (X)", "layoutDimension", [{"a": pa, "b": at(o, -15.0, 3.25), "offset": 0.0, "height": 2.5, "style": "ordinate", "angle": 90.0}], "length", A[1], LENGTH, "value"))
    ac, a1, a2 = at(o, 50, 50), at(o, 60, 50), at(o, 50, 60)
    AC, A1, A2 = P(ac), P(a1), P(a2)
    rr = sqrt((A1[0] - AC[0]) ** 2 + (A1[1] - AC[1]) ** 2)
    sweep = (atan2(A2[1] - AC[1], A2[0] - AC[0]) - atan2(A1[1] - AC[1], A1[0] - AC[0])) % (2 * pi)
    c.append(case(f"{name}: yay uzunluğu ölçüsü", "layoutDimension", [{"a": a1, "b": a2, "offset": 3.0, "height": 2.5, "style": "arcLength", "c": ac}], "length", rr * sweep, LENGTH, "value"))
    c.append(case(f"{name}: semt ölçüsü", "layoutDimension", [{"a": pa, "b": pb, "offset": 4.0, "height": 2.5, "style": "azimuth"}], "angle", atan2(dx, dy) % (2 * pi), ANGLE, "value"))
    c.append(case(f"{name}: eğim ölçüsü", "layoutDimension", [{"a": pa, "b": pb, "offset": -4.0, "height": 2.5, "style": "slope", "za": 102.4, "zb": 101.15}], "percent", fabs(mpf(101.15) - mpf(102.4)) / sqrt(dx * dx + dy * dy) * 100, PERCENT, "value"))
    return c


def drawn(rnd, origin, name, count):
    out = []
    for i in range(count):
        kind = i % 5
        if kind == 0:
            a = at(origin, rnd.uniform(-2000, 2000), rnd.uniform(-2000, 2000))
            b = at(origin, rnd.uniform(-2000, 2000), rnd.uniform(-2000, 2000))
            pa, pb = P(a), P(b)
            dx, dy = pb[0] - pa[0], pb[1] - pa[1]
            out.append(case(f"{name}: rastgele uzaklık {i}", "dist", [a, b], "length", sqrt(dx * dx + dy * dy), LENGTH))
            out.append(case(f"{name}: rastgele semt {i}", "bearingGrad", [a, b], "grad", (atan2(dx, dy) * 200 / pi) % 400, ANGLE))
        elif kind == 1:
            n = rnd.randint(3, 9)
            base = rnd.uniform(5, 300)
            pts = []
            for k in range(n):
                t = 2 * 3.141592653589793 * k / n + rnd.uniform(-0.2, 0.2)
                rad = base * rnd.uniform(0.6, 1.0)
                pts.append(at(origin, rad * float(cos(t)), rad * float(sin(t))))
            bulges = [rnd.choice([0, 0, 0, rnd.uniform(-0.3, 0.3)]) for _ in range(n)]
            poly = {"kind": "polygon", "pts": pts, "bulges": bulges}
            out.append(case(f"{name}: rastgele alan {i}", "entityArea", [poly], "area", polygon_area(poly), AREA))
            out.append(case(f"{name}: rastgele çevre {i}", "entityLength", [poly], "length", polygon_perimeter(poly), LENGTH))
        elif kind == 2:
            n = rnd.randint(2, 7)
            pts = [at(origin, rnd.uniform(-500, 500), rnd.uniform(-500, 500)) for _ in range(n)]
            bulges = [rnd.choice([0, rnd.uniform(-1.5, 1.5)]) for _ in range(n)]
            line = {"kind": "polyline", "pts": pts, "bulges": bulges}
            out.append(case(f"{name}: rastgele çoklu çizgi {i}", "entityLength", [line], "length", path_length(pts, bulges, False), LENGTH))
        elif kind == 3:
            r = rnd.uniform(0.01, 1000)
            a0, a1 = rnd.uniform(0, 6.28), rnd.uniform(0, 6.28)
            arc = {"kind": "arc", "c": at(origin, rnd.uniform(-100, 100), rnd.uniform(-100, 100)), "r": r, "a0": a0, "a1": a1}
            out.append(case(f"{name}: rastgele yay {i}", "entityLength", [arc], "length", mpf(r) * turn(a0, a1), LENGTH))
        else:
            n = rnd.randint(3, 6)
            pts = [at(origin, rnd.uniform(-200, 200), rnd.uniform(-200, 200)) for _ in range(n)]
            s = {"kind": "spline", "pts": pts, "closed": rnd.random() < 0.3}
            out.append(case(f"{name}: rastgele eğri {i}", "entityLength", [s], "length", catmull_length(pts, s["closed"]), LENGTH))
    return out


def document():
    rnd = random.Random(20261001)
    cases = hand((0.0, 0.0), "başlangıç") + hand(TM, "TM") + drawn(rnd, TM, "TM", 60) + drawn(rnd, (0.0, 0.0), "başlangıç", 20)
    return {
        "format": "kentos.measure-fixtures",
        "version": 1,
        "rule": "display v1 (docs/adr/0149) for the shown texts, 0 to 6 decimals; angles shown in grads",
        "source": "python mpmath, 50 digits (independent of KentOS code), scripts/fixtures/measure_cases.py",
        "cases": cases,
    }


def main() -> int:
    doc = document()
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    count = len(doc["cases"])
    if "--check" in sys.argv[1:]:
        if OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} tanımlardan yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)")
            return 1
        print(f"{OUT.relative_to(ROOT)}: {count} durum tanımlarla tutarlı")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({count} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
