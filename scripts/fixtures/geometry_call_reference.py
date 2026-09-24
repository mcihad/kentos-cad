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
