"""Independent reference values for core operations called by name
(CLAUDE.md §23.4, docs/adr/0008): exact rational arithmetic on the
arguments' decimal text (Python fractions) and 60-digit square roots,
never KentOS code. Native Rust, WASM and (while it exists) the TypeScript
reference must stay within each case's bound.

    python3 scripts/fixtures/geometry_call_reference.py

Writes fixtures/geometry/v1/reference-calls.json. Arguments are written as
JSON numbers from their decimal text, so a reader gets the nearest float64,
as it would from a file; expected values are decimal text.
"""
import json
from decimal import Decimal, getcontext
from fractions import Fraction as F

getcontext().prec = 60
PI = Decimal("3.141592653589793238462643383279502884197169399375105820974944")

def D(f):
    """A Fraction as decimal text."""
    return format(Decimal(f.numerator) / Decimal(f.denominator), "f")

def sqrt(f):
    return (Decimal(f.numerator) / Decimal(f.denominator)).sqrt()

def P(x, y):
    """A point given as decimal text: (JSON args, exact value)."""
    return {"x": float(x), "y": float(y)}, (F(Decimal(x)), F(Decimal(y)))

E, N = Decimal("486512.34"), Decimal("4420187.52")
def T(dx, dy):
    return P(format(E + Decimal(dx), "f"), format(N + Decimal(dy), "f"))

cases = []
def case(name, fn, args, expect, bound):
    cases.append({"name": name, "fn": fn, "args": args, "expect": expect, "bound": bound})

# lineLine: exact rational parameters and point (TM coordinates).
(a, A), (b, B), (c, C), (d, Dd) = T("0", "0"), T("40.117", "3.205"), T("12.5", "-20.25"), T("15.875", "30.03")
rx, ry = B[0] - A[0], B[1] - A[1]
sx, sy = Dd[0] - C[0], Dd[1] - C[1]
qx, qy = C[0] - A[0], C[1] - A[1]
den = rx * sy - ry * sx
t = (qx * sy - qy * sx) / den
u = (qx * ry - qy * rx) / den
case("TM doğruları kesişimi", "lineLine", [a, b, c, d],
     {"p": {"x": D(A[0] + t * rx), "y": D(A[1] + t * ry)}, "t": D(t), "u": D(u)},
     # A TM coordinate carries 0.5 ulp (4.7e-10 m) of input rounding.
     "1e-8")

# circleThrough: exact centre, radius from a 60-digit root.
(p1, Q1), (p2, Q2), (p3, Q3) = T("0", "0"), T("30.5", "4.25"), T("11.75", "27.125")
ax, ay = Q2[0] - Q1[0], Q2[1] - Q1[1]
bx, by = Q3[0] - Q1[0], Q3[1] - Q1[1]
dd = 2 * (ax * by - ay * bx)
a2, b2 = ax * ax + ay * ay, bx * bx + by * by
cx, cy = (by * a2 - ay * b2) / dd, (ax * b2 - bx * a2) / dd
case("TM üç noktadan çember", "circleThrough", [p1, p2, p3],
     {"c": {"x": D(Q1[0] + cx), "y": D(Q1[1] + cy)}, "r": format(sqrt(cx * cx + cy * cy), "f")}, "1e-8")

# bulgeArc: a half circle on a TM chord (centre on the chord, a0 = π, sweep = π).
(ha, HA), (hb, HB) = T("0", "0"), T("20", "0")
case("TM yarım daire yayı", "bulgeArc", [ha, hb, 1],
     {"c": {"x": D((HA[0] + HB[0]) / 2), "y": D((HA[1] + HB[1]) / 2)}, "r": "10", "a0": format(PI, "f"), "sweep": format(PI, "f")}, "1e-9")
# segmentMid of the same half circle: 10 m to the right of travel.
case("TM yarım daire ortası", "segmentMid", [ha, hb, 1],
     {"x": D((HA[0] + HB[0]) / 2), "y": D(HA[1] - 10)}, "1e-9")

# distToSegment: perpendicular distance, exact square then a 60-digit root.
(sp, SP), (sa, SA), (sb, SB) = T("7.25", "9.5"), T("0", "0"), T("25", "2.5")
dx, dy = SB[0] - SA[0], SB[1] - SA[1]
tt = ((SP[0] - SA[0]) * dx + (SP[1] - SA[1]) * dy) / (dx * dx + dy * dy)
fx, fy = SP[0] - (SA[0] + tt * dx), SP[1] - (SA[1] + tt * dy)
case("TM noktanın parçaya uzaklığı", "distToSegment", [sp, sa, sb], format(sqrt(fx * fx + fy * fy), "f"), "1e-8")

# pathLength: a TM traverse, each leg a 60-digit root.
legs = [T("0", "0"), T("12.345", "6.789"), T("30.1", "2.2"), T("41.004", "19.87")]
total = sum(sqrt((legs[i][1][0] - legs[i - 1][1][0]) ** 2 + (legs[i][1][1] - legs[i - 1][1][1]) ** 2) for i in range(1, len(legs)))
case("TM poligon güzergâhı uzunluğu", "pathLength", [[l[0] for l in legs], False], format(total, "f"), "1e-8")

# circleCircle: two TM circles, both crossings.
(k1, K1), (k2, K2) = T("0", "0"), T("12", "5")
r1, r2 = F(10), F(8)
ddx, ddy = K2[0] - K1[0], K2[1] - K1[1]
d2 = ddx * ddx + ddy * ddy  # 169: d = 13 exactly
d = F(13)
aa = (r1 * r1 - r2 * r2 + d2) / (2 * d)
h = sqrt(r1 * r1 - aa * aa)
mx, my = K1[0] + aa * ddx / d, K1[1] + aa * ddy / d
hx, hy = Decimal(ddy.numerator) / Decimal(ddy.denominator) / 13, Decimal(ddx.numerator) / Decimal(ddx.denominator) / 13
MX, MY = Decimal(mx.numerator) / Decimal(mx.denominator), Decimal(my.numerator) / Decimal(my.denominator)
case("TM iki çember kesişimi", "circleCircle", [k1, 10, k2, 8],
     [{"x": format(MX + h * hx, "f"), "y": format(MY - h * hy, "f")}, {"x": format(MX - h * hx, "f"), "y": format(MY + h * hy, "f")}], "1e-8")

# tangentPoints from an outside point: c + (r²/d²)v ± (r·√(d²−r²)/d²)·perp(v).
(tp, TP), (tc, TC) = T("25", "10"), T("0", "0")
rr = F(6)
vx, vy = TP[0] - TC[0], TP[1] - TC[1]
dd2 = vx * vx + vy * vy
k = rr * rr / dd2
s = Decimal(rr.numerator) * sqrt(dd2 - rr * rr) / (Decimal(dd2.numerator) / Decimal(dd2.denominator))
def dec(f):
    return Decimal(f.numerator) / Decimal(f.denominator)
bxp, byp = dec(TC[0] + k * vx), dec(TC[1] + k * vy)
# Math: base + half comes first (counter-clockwise of the centre→point direction).
case("TM noktadan teğet noktaları", "tangentPoints", [tp, tc, 6],
     [{"x": format(bxp - s * dec(vy), "f"), "y": format(byp + s * dec(vx), "f")},
      {"x": format(bxp + s * dec(vy), "f"), "y": format(byp - s * dec(vx), "f")}], "1e-8")

# Arc length of a quarter circle and a TM arc through three points (radius from a root).
case("çeyrek yay uzunluğu (5π)", "arcLength", [{"c": {"x": 0, "y": 0}, "r": 10, "a0": 0, "a1": float(PI / 2)}], format(5 * PI, "f"), "1e-12")

# offsetPath: an axis-aligned TM square pushed out by 0.5 m — the mitred corners are exact.
sq = [T("0", "0"), T("10", "0"), T("10", "10"), T("0", "10")]
off = F(1, 2)
corners = [(-off, -off), (off, -off), (off, off), (-off, off)]
case("TM karenin dışa ötelemesi", "offsetPath", [[q[0] for q in sq], -0.5, True],
     # Right of travel on a counter-clockwise ring is outside: offset −0.5.
     [{"x": D(q[1][0] + dx), "y": D(q[1][1] + dy)} for q, (dx, dy) in zip(sq, corners)], "1e-9")

# hatchLines at 0°, spacing 1 m on the same square: lines on whole northings, ends on the sides.
from decimal import ROUND_CEILING
first = N.to_integral_value(rounding=ROUND_CEILING)
lines = []
k = first
while k <= N + 10:
    lines.append([{"x": D(F(E)), "y": D(F(k))}, {"x": D(F(E + 10)), "y": D(F(k))}])
    k += 1
case("TM karesine 0° tarama (1 m)", "hatchLines", [[q[0] for q in sq], 0, 1], {"segments": lines, "capped": False}, "1e-9")

# layoutDimension: an aligned dimension measures the exact distance.
(da, DA), (db, DB) = T("0", "0"), T("33.3", "-12.25")
case("TM hizalı ölçü değeri", "layoutDimension", [{"a": da, "b": db, "offset": 2, "height": 1}],
     {"value": format(sqrt((DB[0] - DA[0]) ** 2 + (DB[1] - DA[1]) ** 2), "f")}, "1e-9")

# orientation: the exact side (CLAUDE.md §23.3). The predicate is exact on
# the float64 values a reader gets, so the sign is taken on those exact
# values (Fraction of the float), not on decimal text.
import math
import random

def Pf(x, y):
    """A point given as float64: (JSON args, exact value)."""
    return {"x": x, "y": y}, (F(x), F(y))

def side(A, B, C):
    d = (A[0] - C[0]) * (B[1] - C[1]) - (A[1] - C[1]) * (B[0] - C[0])
    return (d > 0) - (d < 0)

def orient_case(name, a, b, c):
    (ja, A), (jb, B), (jc, C) = a, b, c
    case(name, "orientation", [ja, jb, jc], str(side(A, B, C)), "0")

case("sol dönüş", "orientation", [P("0", "0")[0], P("1", "0")[0], P("0", "1")[0]], "1", "0")
case("sağ dönüş", "orientation", [P("0", "0")[0], P("1", "0")[0], P("0", "-1")[0]], "-1", "0")
case("aynı doğru üzerinde", "orientation", [P("0", "0")[0], P("1", "1")[0], P("2", "2")[0]], "0", "0")
case("çakışık iki nokta", "orientation", [P("3", "4")[0], P("3", "4")[0], P("7", "-1")[0]], "0", "0")

# Kettner et al.'s classroom grid: points a few ulps around (0.5, 0.5)
# against the line through (12, 12) and (24, 24); the rounded determinant
# misjudges many of them.
u = 2.0 ** -53
q, r = Pf(12.0, 12.0), Pf(24.0, 24.0)
for i, j in [(0, 0), (1, 0), (0, 1), (1, 1), (2, 1), (1, 2), (3, 5), (5, 3), (7, 7), (8, 9),
             (13, 12), (12, 13), (20, 21), (31, 30), (40, 40), (47, 50), (63, 1), (1, 63)]:
    orient_case(f"sınıf ızgarası p=(0,5+{i}u, 0,5+{j}u)", Pf(0.5 + i * u, 0.5 + j * u), q, r)
    orient_case(f"sınıf ızgarası döndürülmüş ({i}, {j})", q, r, Pf(0.5 + i * u, 0.5 + j * u))

# One ulp off an exactly collinear TM point.
a, b = Pf(486512.5, 4420187.25), Pf(486512.5 + 80.5, 4420187.25 + 60.25)
mx, my = 486512.5 + 40.25, 4420187.25 + 30.125
orient_case("TM doğrusu üzerinde", a, b, Pf(mx, my))
for dx, dy, label in [(0, 1, "yukarı"), (0, -1, "aşağı"), (1, 0, "doğuya"), (-1, 0, "batıya")]:
    x = math.nextafter(mx, math.inf * dx) if dx else mx
    y = math.nextafter(my, math.inf * dy) if dy else my
    orient_case(f"TM doğrusundan bir ulp {label}", a, b, Pf(x, y))

# Random near-collinear TM triples: c on a→b rounded, then nudged by ulps.
rnd = random.Random(20260924)
for n in range(40):
    ax, ay = rnd.uniform(400000, 600000), rnd.uniform(4200000, 4600000)
    reach = 5.0 if n % 3 == 0 else 5000.0
    bx, by = ax + rnd.uniform(-reach, reach), ay + rnd.uniform(-reach, reach)
    t = rnd.uniform(-2, 3)
    cx, cy = ax + t * (bx - ax), ay + t * (by - ay)
    for _ in range(n % 4):
        if rnd.random() < 0.5:
            cx = math.nextafter(cx, math.inf if rnd.random() < 0.5 else -math.inf)
        else:
            cy = math.nextafter(cy, math.inf if rnd.random() < 0.5 else -math.inf)
    orient_case(f"TM neredeyse aynı doğrultu #{n}", Pf(ax, ay), Pf(bx, by), Pf(cx, cy))

# lineLine and segSeg on nearly parallel lines (CLAUDE.md §23.3): exact
# parameters on the float64 values a reader gets. The plain rounded cross
# products put t 3.6e-9 off here (the point 4e-7 m) and miss the touch at
# the shared end (t 1 + 8.4e-9, outside the 1e-9 band).
def cross_hit(a, b, c, d):
    (A0, A1), (B0, B1), (C0, C1), (D0, D1) = [(F(p["x"]), F(p["y"])) for p in (a, b, c, d)]
    rx, ry, sx, sy = B0 - A0, B1 - A1, D0 - C0, D1 - C1
    den = rx * sy - ry * sx
    qx, qy = C0 - A0, C1 - A1
    t, u = (qx * sy - qy * sx) / den, (qx * ry - qy * rx) / den
    return {"p": {"x": D(A0 + t * rx), "y": D(A1 + t * ry)}, "t": D(t), "u": D(u)}

th, ax_, ay_ = 0.3, 486512.34, 4420187.52
pa = {"x": ax_, "y": ay_}
pb = {"x": ax_ + 120 * math.cos(th), "y": ay_ + 120 * math.sin(th)}
xm, ym = ax_ + 0.6 * (pb["x"] - ax_), ay_ + 0.6 * (pb["y"] - ay_)
ph = th + 2e-9
pc = {"x": xm - 80 * math.cos(ph), "y": ym - 80 * math.sin(ph)}
pd = {"x": xm + 90 * math.cos(ph), "y": ym + 90 * math.sin(ph)}
case("TM neredeyse paralel iki doğru (2e-9 rad)", "lineLine", [pa, pb, pc, pd], cross_hit(pa, pb, pc, pd),
     # t and u accurate to ~1e-12; the point then within an ulp of a TM coordinate.
     "1e-9")
ph = th + 3e-9
pc = {"x": pb["x"] - 150 * math.cos(ph), "y": pb["y"] - 150 * math.sin(ph)}
hit = cross_hit(pa, pb, pc, pb)
assert F(hit["t"]) == 1 and F(hit["u"]) == 1
case("TM ortak uçta 3e-9 rad açıyla birleşen iki kenar", "segSeg", [pa, pb, pc, pb], hit, "1e-9")

# ── Surveying computations (Hesap menüsü: crates/shared/geometry-core/src/survey) ──
# Trigonometry by 60-digit series here, independent of KentOS code and of libm;
# the forward intersection solves two rays, the resection runs Newton's method
# on the two measured angles (the core mirrors B in the line of two circle
# centres and uses the sine rule).
TWO_PI = 2 * PI

def pmod(x):
    """x in [0, 2π) (Decimal's % keeps the dividend's sign)."""
    return x - TWO_PI * (x / TWO_PI).to_integral_value(rounding="ROUND_FLOOR")

def dsin(x):
    x = pmod(x)
    term, total, n = x, x, 1
    while abs(term) > Decimal("1e-70"):
        term = -term * x * x / ((2 * n) * (2 * n + 1))
        total += term
        n += 1
    return total

def dcos(x):
    return dsin(x + PI / 2)

def datan(x):
    # atan(x) = 2 atan(x / (1 + sqrt(1 + x²))), halved until the series is quick.
    k = 0
    while abs(x) > Decimal("0.1"):
        x = x / (1 + (1 + x * x).sqrt())
        k += 1
    term, total, n = x, x, 1
    while abs(term) > Decimal("1e-70"):
        term = -term * x * x
        total += term / (2 * n + 1)
        n += 1
    return total * (2 ** k)

def datan2(y, x):
    if x > 0:
        return datan(y / x)
    if x < 0:
        return datan(y / x) + (PI if y >= 0 else -PI)
    return PI / 2 if y > 0 else -PI / 2

def dd(x):
    return x if isinstance(x, Decimal) else Decimal(x.numerator) / Decimal(x.denominator)

def semt(a, b):
    return pmod(datan2(dd(b[0]) - dd(a[0]), dd(b[1]) - dd(a[1])))

def dist(a, b):
    dx, dy = dd(b[0]) - dd(a[0]), dd(b[1]) - dd(a[1])
    return (dx * dx + dy * dy).sqrt()

def signed(r):
    r = pmod(r)
    return r - TWO_PI if r > PI else r

def num(text):
    return float(text), Decimal(text)

def S(x):
    return format(x, "f")

def UNIT(u):
    return (Decimal(400) if u == "grad" else Decimal(360)) / TWO_PI

def trav(unit, start, back, end, fore, angles, dists):
    """A traverse adjusted as in the Hesap dialog: angles equally, coordinates by the compass rule."""
    k = UNIT(unit)
    A = [Decimal(a) / k for a in angles]
    Sd = [Decimal(d) for d in dists]
    t, bk = [], semt(start[1], back[1])
    for i in range(len(Sd)):
        leg = pmod(bk + A[i])
        t.append(leg)
        bk = leg + PI
    out = {}
    corr = Decimal(0)
    if fore is not None:
        f = signed(bk + A[len(Sd)] - semt(end[1], fore[1]))
        corr = -f / len(A)
        out["angleMisclosure"] = S(f * k)
        out["angleCorrection"] = S(corr * k)
    t = [pmod(x + corr * (i + 1)) for i, x in enumerate(t)]
    L = sum(Sd)
    raw = [(s * dsin(x), s * dcos(x)) for x, s in zip(t, Sd)]
    fy = fx = Decimal(0)
    if end is not None:
        fy = sum(r[0] for r in raw) - (dd(end[1][0]) - dd(start[1][0]))
        fx = sum(r[1] for r in raw) - (dd(end[1][1]) - dd(start[1][1]))
        out["fy"], out["fx"] = S(fy), S(fx)
        out["linearMisclosure"] = S((fy * fy + fx * fx).sqrt())
    y, x = dd(start[1][0]), dd(start[1][1])
    pts, legs = [], []
    for (dy, dx), s, b in zip(raw, Sd, t):
        vy, vx = -fy * s / L, -fx * s / L
        y, x = y + dy + vy, x + dx + vx
        pts.append({"x": S(y), "y": S(x)})
        legs.append({"bearing": S(b * k), "distance": S(s), "dy": S(dy + vy), "dx": S(dx + vx), "vy": S(vy), "vx": S(vx)})
    if end is not None:
        pts.pop()
    out["points"], out["legs"], out["length"] = pts, legs, S(L)
    return out

def pt(dx, dy):
    """A TM point as (JSON args, exact Decimal pair)."""
    x, y = format(E + Decimal(dx), "f"), format(N + Decimal(dy), "f")
    return {"x": float(x), "y": float(y)}, (Decimal(x), Decimal(y))

def notes(unit, stations, back, fore, angle_places, errors=()):
    """Field notes of a traverse through designed points: angles and distances rounded as read
    (angle_places decimals, mm), plus deliberate errors (index, amount) in the angles."""
    k = UNIT(unit)
    q = Decimal(1).scaleb(-angle_places)
    angles = []
    for i in range(len(stations) - 1):
        prev = back if i == 0 else stations[i - 1]
        angles.append(pmod(semt(stations[i][1], stations[i + 1][1]) - semt(stations[i][1], prev[1])) * k)
    if fore is not None:
        angles.append(pmod(semt(stations[-1][1], fore[1]) - semt(stations[-1][1], stations[-2][1])) * k)
    angles = [a.quantize(q) for a in angles]
    for i, e in errors:
        angles[i] += Decimal(e)
    dists = [dist(stations[i][1], stations[i + 1][1]).quantize(Decimal("0.001")) for i in range(len(stations) - 1)]
    return [format(a, "f") for a in angles], [format(d, "f") for d in dists]

# Bağlı poligon in grad: rounded field notes (0.1 mgon, mm) and a 2.5 mgon angle error.
A, A0, E_, E0 = pt("0", "0"), pt("-152.318", "211.407"), pt("612.884", "95.271"), pt("790.114", "-60.905")
route = [A, pt("138.214", "35.119"), pt("301.502", "-5.873"), pt("455.931", "16.044"), E_]
angles, dists = notes("grad", route, A0, E0, 4, [(2, "0.0025")])
case("Bağlı poligon (grad, açı ve koordinat dengelemesi)", "surveyTraverse",
     [{"unit": "grad", "start": A[0], "back": A0[0], "end": E_[0], "fore": E0[0], "angles": [float(a) for a in angles], "distances": [float(d) for d in dists]}],
     trav("grad", A, A0, E_, E0, angles, dists), "1e-8")

# Kapalı poligon in degrees: back on its start, oriented on the same point (seconds as 0.0001°).
B0 = pt("-80.5", "40.25")
angles, dists = notes("deg", [A, pt("100.0", "50.0"), pt("200.0", "-50.0"), pt("150.0", "-100.0"), A], B0, B0, 4)
case("Kapalı poligon (derece)", "surveyTraverse",
     [{"unit": "deg", "start": A[0], "back": B0[0], "end": A[0], "fore": B0[0], "angles": [float(a) for a in angles], "distances": [float(d) for d in dists]}],
     trav("deg", A, B0, A, B0, angles, dists), "1e-8")

# Açık poligon: no closure.
angles, dists = notes("grad", [A, pt("60.2", "70.9"), pt("180.4", "95.35")], A0, None, 4)
case("Açık poligon (grad)", "surveyTraverse",
     [{"unit": "grad", "start": A[0], "back": A0[0], "end": None, "fore": None, "angles": [float(a) for a in angles], "distances": [float(d) for d in dists]}],
     trav("grad", A, A0, None, None, angles, dists), "1e-8")

# Kutupsal alım (grad): slope distances with zenith angles, heights.
St, Bk = pt("10.5", "-20.25"), pt("250.125", "180.75")
k = UNIT("grad")
back_reading = Decimal("12.3456")
shots = [("48.7612", "84.231", None, None), ("233.0105", "152.608", "98.4410", "1.600"), ("380.5000", "35.114", "101.2050", "2.150")]
sz, ih = Decimal("812.345"), Decimal("1.550")
orient = semt(St[1], Bk[1]) - back_reading / k
exp = []
for r, d, z, th in shots:
    t = pmod(orient + Decimal(r) / k)
    D_ = Decimal(d)
    if z is None:
        h, dz = D_, None
    else:
        zr = Decimal(z) / k
        h, dz = D_ * dsin(zr), D_ * dcos(zr) + ih - Decimal(th)
    e = {"p": {"x": S(St[1][0] + h * dsin(t)), "y": S(St[1][1] + h * dcos(t))}, "bearing": S(t * k), "horizontal": S(h)}
    if dz is not None:
        e["z"], e["dz"] = S(sz + dz), S(dz)
    exp.append(e)
case("Kutupsal alım (grad, eğik uzunluk ve başucu açısı)", "surveyPolar",
     [{"unit": "grad", "station": St[0], "back": Bk[0], "backReading": float(back_reading), "stationZ": float(sz), "instrumentHeight": float(ih),
       "shots": [{"reading": float(r), "distance": float(d), "zenith": None if z is None else float(z), "targetHeight": None if th is None else float(th)} for r, d, z, th in shots]}],
     exp, "1e-8")

# Aplikasyon: bearings, distances and turning angles from the station.
T1, T2 = pt("-40.004", "95.312"), pt("301.77", "-12.5")
exp = []
for T in (T1, T2):
    t = semt(St[1], T[1])
    exp.append({"bearing": S(t * k), "distance": S(dist(St[1], T[1])), "angle": S((pmod(t - semt(St[1], Bk[1]))) * k)})
case("Aplikasyon (grad)", "surveyStakeout", [{"unit": "grad", "station": St[0], "back": Bk[0], "targets": [T1[0], T2[0]]}], exp, "1e-8")

# Önden kestirme: the two rays meet (solved as lines here).
Pa, Pb = pt("0", "0"), pt("412.51", "88.07")
al, be = Decimal("61.2345"), Decimal("48.9012")
ta = pmod(semt(Pa[1], Pb[1]) + al / k)
tb = pmod(semt(Pb[1], Pa[1]) - be / k)
ax0, ay0, bx0, by0 = Pa[1][0], Pa[1][1], Pb[1][0], Pb[1][1]
ua, va, ub, vb = dsin(ta), dcos(ta), dsin(tb), dcos(tb)
den = ua * vb - va * ub
sa = ((bx0 - ax0) * vb - (by0 - ay0) * ub) / den
case("Önden kestirme (grad)", "surveyForward", ["grad", Pa[0], Pb[0], float(al), float(be)],
     {"x": S(ax0 + sa * ua), "y": S(ay0 + sa * va)}, "1e-8")

# Geriden kestirme: Newton's method on the two angles, from the point the angles were read at.
Ra, Rb, Rc = pt("-310.2", "402.75"), pt("205.66", "515.1"), pt("498.3", "-120.45")
al, be = Decimal("83.4417"), Decimal("121.0968")
def ang(P, U, V):
    return pmod(semt(P, V) - semt(P, U))
P = [E + Decimal("20"), N + Decimal("15")]
for _ in range(30):
    f1 = ang(P, Ra[1], Rb[1]) - al / k
    f2 = ang(P, Rb[1], Rc[1]) - be / k
    h = Decimal("1e-25")
    J = []
    for dxy in ((h, 0), (0, h)):
        Q = [P[0] + dxy[0], P[1] + dxy[1]]
        J.append(((ang(Q, Ra[1], Rb[1]) - al / k - f1) / h, (ang(Q, Rb[1], Rc[1]) - be / k - f2) / h))
    a11, a21 = J[0]
    a12, a22 = J[1]
    det = a11 * a22 - a12 * a21
    P = [P[0] - (f1 * a22 - a12 * f2) / det, P[1] - (a11 * f2 - a21 * f1) / det]
assert abs(ang(P, Ra[1], Rb[1]) - al / k) < Decimal("1e-40") and abs(ang(P, Rb[1], Rc[1]) - be / k) < Decimal("1e-40")
case("Geriden kestirme (grad)", "surveyResection", ["grad", Ra[0], Rb[0], Rc[0], float(al), float(be)],
     {"p": {"x": S(P[0]), "y": S(P[1])}}, "1e-7")

# cornersOfRing (docs/adr/0032): the rectangle tool's corner style. A TM
# rectangle rounded by 2 m: every tangent point 2 m from its corner along
# the side, and a quarter arc per corner, bulge tan(22.5°) = √2 − 1. A
# rotated square (3-4-5 sides, clockwise) cut by 1.5 m: every cut point
# 1.5 m along its side, exact. Corners run from the first: its point
# towards the previous corner, then towards the next.
def TMP(dx, dy):
    """A TM point as T gives it (the cases above reuse T and P as names)."""
    x, y = format(E + Decimal(dx), "f"), format(N + Decimal(dy), "f")
    return {"x": float(x), "y": float(y)}, (F(Decimal(x)), F(Decimal(y)))
def PD(x, y):
    return {"x": D(x), "y": D(y)}
rect = [TMP("0", "0"), TMP("20.5", "0"), TMP("20.5", "10.25"), TMP("0", "10.25")]
(x0, y0), (x1, y1) = rect[0][1], rect[2][1]
fr = F(2)
quarter = format(sqrt(F(2)) - 1, "f")
case("TM dikdörtgenin köşeleri 2 m yuvarlanır", "cornersOfRing", [[q[0] for q in rect], {"radius": 2}],
     {"pts": [PD(x0, y0 + fr), PD(x0 + fr, y0), PD(x1 - fr, y0), PD(x1, y0 + fr),
              PD(x1, y1 - fr), PD(x1 - fr, y1), PD(x0 + fr, y1), PD(x0, y1 - fr)],
      "bulges": [quarter, "0", quarter, "0", quarter, "0", quarter, "0"]}, "1e-8")
square = [(F(0), F(0)), (F(6), F(8)), (F(14), F(2)), (F(8), F(-6))]
cut = F(3, 2)
cut_pts = []
for i, (vx, vy) in enumerate(square):
    for (qx, qy) in (square[i - 1], square[(i + 1) % 4]):
        # Every side is 10 m long.
        cut_pts.append(PD(vx + (qx - vx) / 10 * cut, vy + (qy - vy) / 10 * cut))
case("Dönük karenin köşeleri 1,5 m pahlanır", "cornersOfRing",
     [[{"x": float(x), "y": float(y)} for x, y in square], {"d1": 1.5, "d2": 1.5}], {"pts": cut_pts}, "1e-12")
# nearestEdge: the clicked segment of a TM polyline, its own corners exactly.
chain = [TMP("0", "0"), TMP("30.25", "0"), TMP("30.25", "18.5")]
case("TM çoklu çizginin tıklanan kenarı", "nearestEdge",
     [{"kind": "polyline", "pts": [q[0] for q in chain]}, TMP("29", "11")[0]],
     {"kind": "seg", "a": PD(*chain[1][1]), "b": PD(*chain[2][1])}, "0")

# offsetThroughDistance (docs/adr/0047): “Noktadan geç” offsets a TM line by
# the point's distance to it, |AB × AP| / |AB| = |30·5 − 40·20| / 50 = 13.
ta, tb, tp = TMP("0", "0"), TMP("30", "40"), TMP("20", "5")
(ax_, ay_), (bx_, by_), (px_, py_) = ta[1], tb[1], tp[1]
cross_ = (bx_ - ax_) * (py_ - ay_) - (by_ - ay_) * (px_ - ax_)
case("TM çizgiye noktadan geçen öteleme uzaklığı", "offsetThroughDistance",
     [{"kind": "line", "a": ta[0], "b": tb[0]}, tp[0]],
     format(abs(Decimal(cross_.numerator) / Decimal(cross_.denominator)) / sqrt((bx_ - ax_) ** 2 + (by_ - ay_) ** 2), "f"), "1e-8")
# nearHole: a click 1 m from a hole's edge and 9 m from the outer ring is at the hole.
case("Delikli alanda deliğe yakın tıklama", "nearHole",
     [{"kind": "polygon", "pts": [{"x": 0, "y": 0}, {"x": 20, "y": 0}, {"x": 20, "y": 20}, {"x": 0, "y": 20}],
       "holes": [{"pts": [{"x": 8, "y": 8}, {"x": 12, "y": 8}, {"x": 12, "y": 12}, {"x": 8, "y": 12}]}]}, {"x": 10, "y": 9}],
     True, "0")
# cornerNear: two TM lines meeting end to end (3-4-5 and 7-24-25 sides); the
# corner is their shared end, each side's unit direction exact, its reach
# the shorter side, the angle atan2(|u1 × u2|, u1 · u2).
c0, c1, c2 = TMP("0", "0"), TMP("30", "40"), TMP("24", "-7")
u1 = (F(3, 5), F(4, 5))
u2 = (F(24, 25), F(-7, 25))
dot_ = u1[0] * u2[0] + u1[1] * u2[1]
crs_ = abs(u1[0] * u2[1] - u1[1] * u2[0])
phi_ = datan2(Decimal(crs_.numerator) / Decimal(crs_.denominator), Decimal(dot_.numerator) / Decimal(dot_.denominator))
case("TM iki çizginin ortak ucu köşedir", "cornerNear",
     [[{"kind": "line", "a": c0[0], "b": c1[0]}, {"kind": "line", "a": c2[0], "b": c0[0]}], TMP("0.3", "0.2")[0], 1.5, 0.25],
     {"site": {"kind": "lines", "first": 0, "pick1": PD(*c1[1]), "second": 1, "pick2": PD(*c2[1])},
      "corner": {"at": PD(*c0[1]), "u1": PD(*u1), "u2": PD(*u2), "reach": "25", "phi": format(phi_, "f")}}, "1e-8")

# gridArrayTransforms (docs/adr/0047): row after row, the copy j columns and i
# rows away moved by (j·dx, i·dy); exact.
gdx, gdy = F(Decimal("12.25")), F(Decimal("-7.5"))
case("Dizi 3 × 2, satır satır", "gridArrayTransforms", [3, 2, 12.25, -7.5],
     [["1", "0", "0", "1", D(j * gdx), D(i * gdy)] for i in range(3) for j in range(2) if i or j], "0")
# shapesMiddle: a TM line and a circle; the box's middle, exact.
(la, LA), (lb, LB), (cc, CC) = TMP("0", "0"), TMP("10", "2"), TMP("20", "10")
mx = (min(LA[0], LB[0], CC[0] - F(5, 2)) + max(LA[0], LB[0], CC[0] + F(5, 2))) / 2
my = (min(LA[1], LB[1], CC[1] - F(5, 2)) + max(LA[1], LB[1], CC[1] + F(5, 2))) / 2
case("TM çizgi ve dairenin kutusunun ortası", "shapesMiddle",
     [[{"kind": "line", "a": la, "b": lb}, {"kind": "circle", "c": cc, "r": 2.5}], "barlow"],
     {"x": D(mx), "y": D(my)}, "1e-9")
# arrayTransforms, polar: 5 items over 180° (45° steps) about a TM centre.
# Turning: the k-th copy is the rotation by 45k° about the centre. Not
# turning: it moves as the middle of the line's box goes round, the middle's
# offset from the centre turned less the offset.
(pc, PC), (qa, QA), (qb, QB) = TMP("0", "0"), TMP("10", "0"), TMP("20", "5")
ox = (min(QA[0], QB[0]) + max(QA[0], QB[0])) / 2 - PC[0]
oy = (min(QA[1], QB[1]) + max(QA[1], QB[1])) / 2 - PC[1]
turns, moves = [], []
for k in range(1, 5):
    a = PI * 45 * k / 180
    cs, sn = dcos(a), dsin(a)
    cx_, cy_ = dd(PC[0]), dd(PC[1])
    turns.append([format(v, "f") for v in (cs, sn, -sn, cs, cx_ - cs * cx_ + sn * cy_, cy_ - sn * cx_ - cs * cy_)])
    moves.append(["1", "0", "0", "1", format(cs * dd(ox) - sn * dd(oy) - dd(ox), "f"), format(sn * dd(ox) + cs * dd(oy) - dd(oy), "f")])
case("TM merkez çevresinde 180° içinde 5 öğe, dönerek", "arrayTransforms", ["polar", [pc["x"], pc["y"], 5, 180, 1], [], "barlow"], turns, "1e-8")
case("TM merkez çevresinde 180° içinde 5 öğe, dönmeden", "arrayTransforms",
     ["polar", [pc["x"], pc["y"], 5, 180, 0], [{"kind": "line", "a": qa, "b": qb}], "barlow"], moves, "1e-9")

doc = {
    "format": "kentos.geometry-call-reference",
    "version": 1,
    "source": "python fractions + 60-digit roots and pi (independent of KentOS code)",
    "note": "Arguments are JSON numbers from decimal text (a reader gets the nearest float64); expected values are decimal text; |actual − expected| ≤ bound for every number.",
    "cases": cases,
}
with open("fixtures/geometry/v1/reference-calls.json", "w") as f:
    f.write(json.dumps(doc, indent=2, ensure_ascii=False) + "\n")
for c in cases:
    print(c["fn"], c["name"])
