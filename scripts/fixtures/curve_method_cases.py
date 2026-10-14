"""The shared cases of the product command cad.curves.construct (docs/adr/0289).

    python3 scripts/fixtures/curve_method_cases.py           # writes the file
    python3 scripts/fixtures/curve_method_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.curves.construct.json and fixtures/curve-methods/v1/tools.json (Daire's, Yay's and
Elips's methods end to end through the tools, clicked and typed: the object each writes or the warning it says;
crates/native/interaction/tests/all/curve_methods.rs plays them). The checks, their order, codes, paths and messages are
written here by hand from the ADR; the curves are computed from the methods' definitions (AutoCAD's CIRCLE, ARC
and ELLIPSE as docs/adr/0289 states them), without KentOS code, in 70-digit decimals:

- Daire, 2 nokta: the midpoint, half the distance. 3 nokta: the circumcentre from the two perpendicular bisectors
  (Cramer's rule), the distance to it; collinear when |(b − a) × (c − a)| ≤ 10⁻⁹ |b − a| |c − a|.
- Teğet-teğet-yarıçap: every centre at r from both objects (a line's two parallels, a circle's circles of R + r and
  |R − r|), crossed analytically; the one whose touching points are nearest the near points. Two parallel lines 2r
  apart: the centre on the middle line across from the near points' midpoint.
- Teğet-teğet-teğet: every sign of every tangency; three lines as a linear system, a circle with two lines as the
  quadratic in r the lines leave; the one whose touching points are nearest the near points.
- Yay: 3 nokta through the circumcentre, counter-clockwise when the middle point comes first going so; the others
  from the included angle θ: centre = chord's middle + left normal · (chord / 2) / tan(θ / 2), radius
  (chord / 2) / |sin(θ / 2)|; yön's θ is twice the signed angle from the direction to the chord, yarıçap's
  2 asin(chord / 2r) (2π minus it for a negative radius), kiriş's the same round the centre; Devam's direction is
  the object's at its end (a line's way, an arc's counter-clockwise tangent, a path's last edge, a closed path's
  closing edge), backwards at its start. Stored counter-clockwise: from the start when θ > 0, else from the end.
- Elips: the centre is the axis's middle (or given); the major half-axis the longer of the given one and the other
  (the rotation's a · cos(rotation)); an arc's parameters t = atan2(sin φ / ratio, cos φ) of its polar angles φ.

The inputs are doubles; the reference takes them exactly. A value that is a double exactly is compared exactly; a
step with any other (an angle, a square root) carries `within`: 10⁻⁹ m or rad, the core's chain of roundings
against the reference's one (docs/adr/0289 §4).
"""
import json
import math
import sys
from decimal import Decimal as D, getcontext
from pathlib import Path

getcontext().prec = 70
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.curves.construct.json"

E0, N0 = 487000, 4420000
STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}
WITHIN = 1e-9

# ── 70-digit functions, from their series ─────────────────────────────────────
PI = D("3.141592653589793238462643383279502884197169399375105820974944592307816406286208998628034825342117068")
TAU = 2 * PI


def dec(x):
    return x if isinstance(x, D) else D(x)


def sqrt(x):
    return dec(x).sqrt()


def atan(x):
    x = dec(x)
    if x < 0:
        return -atan(-x)
    if x > 1:
        return PI / 2 - atan(1 / x)
    # Halve twice: atan x = 2 atan(x / (1 + √(1 + x²))).
    k = 0
    while x > D("0.1"):
        x = x / (1 + sqrt(1 + x * x))
        k += 1
    s, term, n = D(0), x, 1
    while True:
        add = term / n
        if abs(add) < D("1e-80"):
            break
        s += add
        term = -term * x * x
        n += 2
    return s * (2 ** k)


def atan2(y, x):
    y, x = dec(y), dec(x)
    if x > 0:
        return atan(y / x)
    if x < 0:
        return atan(y / x) + (PI if y >= 0 else -PI)
    if y > 0:
        return PI / 2
    if y < 0:
        return -PI / 2
    return D(0)


def norm(a):
    a = dec(a) % TAU
    return a + TAU if a < 0 else a


def sin(x):
    x = dec(x) % TAU
    s, term, n = D(0), x, 1
    while abs(term) > D("1e-80"):
        s += term
        term = -term * x * x / ((n + 1) * (n + 2))
        n += 2
    return s


def cos(x):
    return sin(dec(x) + PI / 2)


def tan(x):
    return sin(x) / cos(x)


def asin(x):
    x = dec(x)
    return atan2(x, sqrt(1 - x * x))


# ── The drawing ───────────────────────────────────────────────────────────────
def P(x, y):
    return {"x": x, "y": y}


def layer(i, name):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": STYLE, "children": []}


def rel(x, y):
    return P(E0 + x, N0 + y)


ENTITIES = [
    {"kind": "line", "id": 1, "layerId": "yapi", "attrs": {}, "a": rel(-20, 0), "b": rel(20, 0)},
    {"kind": "line", "id": 2, "layerId": "yapi", "attrs": {}, "a": rel(0, -20), "b": rel(0, 20)},
    {"kind": "line", "id": 3, "layerId": "yapi", "attrs": {}, "a": rel(-20, 10), "b": rel(20, 10)},
    {"kind": "circle", "id": 4, "layerId": "yapi", "attrs": {}, "c": rel(30, 0), "r": 5},
    {"kind": "arc", "id": 5, "layerId": "yapi", "attrs": {}, "c": rel(60, 0), "r": 10, "a0": 0, "a1": math.pi / 2},
    {"kind": "polyline", "id": 6, "layerId": "yapi", "attrs": {}, "pts": [rel(0, 30), rel(10, 30), rel(10, 40)]},
    {"kind": "polygon", "id": 7, "layerId": "yapi", "attrs": {},
     "pts": [rel(20, 20), rel(30, 20), rel(30, 30), rel(20, 30)]},
    {"kind": "point", "id": 8, "layerId": "yapi", "attrs": {}, "p": rel(80, 10)},
    {"kind": "polyline", "id": 9, "layerId": "yapi", "attrs": {}, "pts": [rel(40, 20), rel(50, 20), rel(50, 30)],
     "closed": True},
    {"kind": "line", "id": 10, "layerId": "yapi", "attrs": {}, "a": rel(16, -3), "b": rel(-4, 12)},
    {"kind": "ellipse", "id": 11, "layerId": "yapi", "attrs": {}, "c": rel(100, 0), "major": P(8, 0), "ratio": 0.5,
     "t0": 0, "t1": 2 * math.pi},
    {"kind": "line", "id": 12, "layerId": "yapi", "attrs": {}, "a": rel(-20, 20), "b": rel(20, 20)},
]
IDS = [e["id"] for e in ENTITIES]
SETUP = {
    "format": "kentos.document", "version": 1, "name": "Eğri yapımı",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                 "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
    "origin": P(E0, N0), "layers": [layer("yapi", "Yapı")], "activeLayer": "yapi", "entities": ENTITIES,
    "styles": {"items": [], "categories": []},
}
SAME = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
MISSING = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"

# The objects as the reference reads them: lines by their ends, circles and arcs by their circles, paths by edges.
LINES = {1: ((-20, 0), (20, 0)), 2: ((0, -20), (0, 20)), 3: ((-20, 10), (20, 10)), 10: ((16, -3), (-4, 12)),
         12: ((-20, 20), (20, 20))}
CIRCLES = {4: ((30, 0), 5)}


def at(x, y):
    """A point given by coordinates, relative to the drawing's anchor."""
    return {"kind": "at", "at": rel(x, y)}


def place(slot, p):
    return {"kind": "place", "uid": f"$uidOf:{slot}", "place": p}


START, END, CENTER = {"kind": "start"}, {"kind": "end"}, {"kind": "center"}


def vertex(i):
    return {"kind": "vertex", "part": 0, "ring": 0, "index": i}


def near(slot, x, y):
    return {"uid": f"$uidOf:{slot}", "near": rel(x, y)}


def obj_end(slot, p, path=None):
    out = {"uid": f"$uidOf:{slot}", "place": p}
    if path is not None:
        out["path"] = path
    return out


# ── The answers ───────────────────────────────────────────────────────────────
def value(x):
    """A value as the wire carries it, and whether the double is the value exactly."""
    x = dec(x)
    f = float(x)
    return (int(f) if f.is_integer() else f), D(f) == x


class Out:
    def __init__(self):
        self.exact = True

    def v(self, x):
        out, exact = value(x)
        self.exact = self.exact and exact
        return out

    def pt(self, x, y):
        return P(self.v(D(E0) + dec(x)), self.v(D(N0) + dec(y)))


def circle_out(c, r):
    o = Out()
    return {"kind": "circle", "c": o.pt(*c), "r": o.v(r)}, o.exact


def turn(a):
    """An angle in [0, 2π) as the wire carries it: one that rounds to the whole turn's double is 0 there."""
    a = norm(a)
    return D(0) if float(a) >= float(TAU) else a


def arc_out(c, r, a0, a1):
    o = Out()
    return {"kind": "arc", "c": o.pt(*c), "r": o.v(r), "a0": o.v(turn(a0)), "a1": o.v(turn(a1))}, o.exact


def ellipse_out(c, major, ratio, t0, t1):
    o = Out()
    return {"kind": "ellipse", "c": o.pt(*c), "major": P(o.v(major[0]), o.v(major[1])), "ratio": o.v(ratio),
            "t0": o.v(t0), "t1": o.v(t1)}, o.exact


def done(curves, op="execute", iterated=False):
    """A completed step; `iterated` when the core solves it by iteration (Apollonius' circle, Newton's method), so
    that even an exact answer is reached within a few ulps."""
    exact = all(e for _, e in curves) and not iterated
    s = {"op": op, "input": None, "result": {"status": "completed", "output": {"curves": [c for c, _ in curves]},
                                             "warnings": []}, "expect": SAME}
    if not exact:
        s["within"] = WITHIN
    return s


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def step(constructions, result, op="execute", **more):
    if isinstance(result, dict) and "op" in result:  # done(...)
        s = dict(result)
        s["op"] = op
        s["input"] = {"constructions": constructions}
        s.update(more)
        return {k: s[k] for k in ["op", "input", "result", "expect"] + [k for k in s if k not in
                                                                        ("op", "input", "result", "expect")]}
    s = {"op": op, "input": {"constructions": constructions}, "result": result, "expect": SAME}
    s.update(more)
    return s


def case(name, *steps):
    return {"name": name, "steps": list(steps)}


# ── Circles ───────────────────────────────────────────────────────────────────
def circumcentre(a, b, c):
    ax, ay = dec(a[0]), dec(a[1])
    bx, by = dec(b[0]) - ax, dec(b[1]) - ay
    cx, cy = dec(c[0]) - ax, dec(c[1]) - ay
    d = 2 * (bx * cy - by * cx)
    b2, c2 = bx * bx + by * by, cx * cx + cy * cy
    ux, uy = (cy * b2 - by * c2) / d, (bx * c2 - cx * b2) / d
    return (ax + ux, ay + uy), sqrt(ux * ux + uy * uy)


def collinear(a, b, c):
    bx, by = dec(b[0]) - dec(a[0]), dec(b[1]) - dec(a[1])
    cx, cy = dec(c[0]) - dec(a[0]), dec(c[1]) - dec(a[1])
    return abs(bx * cy - by * cx) <= D(1e-9) * sqrt(bx * bx + by * by) * sqrt(cx * cx + cy * cy)


def support(slot):
    """('line', point, unit direction) or ('circle', centre, radius)."""
    if slot in LINES:
        (ax, ay), (bx, by) = LINES[slot]
        dx, dy = dec(bx - ax), dec(by - ay)
        l = sqrt(dx * dx + dy * dy)
        return ("line", (dec(ax), dec(ay)), (dx / l, dy / l))
    if slot in CIRCLES:
        (cx, cy), r = CIRCLES[slot]
        return ("circle", (dec(cx), dec(cy)), dec(r))
    raise KeyError(slot)


def touch(s, c):
    if s[0] == "line":
        (ax, ay), (ux, uy) = s[1], s[2]
        t = (c[0] - ax) * ux + (c[1] - ay) * uy
        return (ax + ux * t, ay + uy * t)
    (kx, ky), r = s[1], s[2]
    d = sqrt((c[0] - kx) ** 2 + (c[1] - ky) ** 2)
    return (kx + (c[0] - kx) / d * r, ky + (c[1] - ky) / d * r)


def dist(p, q):
    return sqrt((dec(p[0]) - dec(q[0])) ** 2 + (dec(p[1]) - dec(q[1])) ** 2)


def offsets(s, r):
    """The curves the centres of the circles of radius r touching s lie on."""
    if s[0] == "line":
        (ax, ay), (ux, uy) = s[1], s[2]
        return [("line", (ax - uy * r * k, ay + ux * r * k), (ux, uy)) for k in (1, -1)]
    out = [("circle", s[1], s[2] + r)]
    if abs(s[2] - r) > D(1e-9):
        out.append(("circle", s[1], abs(s[2] - r)))
    return out


def cross_curves(p, q):
    if p[0] == "line" and q[0] == "line":
        (ax, ay), (ux, uy) = p[1], p[2]
        (bx, by), (vx, vy) = q[1], q[2]
        den = ux * vy - uy * vx
        if den == 0:
            return []
        t = ((bx - ax) * vy - (by - ay) * vx) / den
        return [(ax + ux * t, ay + uy * t)]
    if p[0] == "circle" and q[0] == "line":
        p, q = q, p
    if p[0] == "line":
        (ax, ay), (ux, uy) = p[1], p[2]
        (cx, cy), r = q[1], q[2]
        t0 = (cx - ax) * ux + (cy - ay) * uy
        fx, fy = ax + ux * t0, ay + uy * t0
        h2 = r * r - ((fx - cx) ** 2 + (fy - cy) ** 2)
        if h2 < 0:
            return []
        h = sqrt(h2)
        return [(fx + ux * h, fy + uy * h), (fx - ux * h, fy - uy * h)]
    raise NotImplementedError


def ttr(s1, n1, s2, n2, r):
    r = dec(r)
    if s1[0] == "line" and s2[0] == "line":
        (ux, uy), (vx, vy) = s1[2], s2[2]
        if ux * vy - uy * vx == 0:
            (ax, ay), (bx, by) = s1[1], s2[1]
            gap = abs(ux * (by - ay) - uy * (bx - ax))
            if gap != 2 * r:
                return None
            mx, my = (dec(n1[0]) + dec(n2[0])) / 2, (dec(n1[1]) + dec(n2[1])) / 2
            t = (mx - ax) * ux + (my - ay) * uy
            side = 1 if ux * (by - ay) - uy * (bx - ax) > 0 else -1
            return (ax + ux * t - uy * r * side, ay + uy * t + ux * r * side), r
    best = None
    for o1 in offsets(s1, r):
        for o2 in offsets(s2, r):
            for c in cross_curves(o1, o2):
                score = dist(touch(s1, c), n1) + dist(touch(s2, c), n2)
                if best is None or score < best[0]:
                    best = (score, c)
    return None if best is None else (best[1], r)


def ttt(ss, ns):
    """Every circle touching the three supports (lines, at most one circle), by signs; the nearest by its touches."""
    lines = [s for s in ss if s[0] == "line"]
    circles = [s for s in ss if s[0] == "circle"]
    sols = []
    for signs in [(a, b, c) for a in (1, -1) for b in (1, -1) for c in (1, -1)]:
        rows = []
        for s, k in zip(lines, signs):
            (ax, ay), (ux, uy) = s[1], s[2]
            nx, ny = -uy, ux  # n · (c − a) = k r
            rows.append((nx, ny, -k, nx * ax + ny * ay))
        if not circles:
            m = [r_[:3] for r_ in rows]
            rhs = [r_[3] for r_ in rows]
            det = det3(m)
            if det == 0:
                continue
            x = [det3([[rhs[i] if j == col else m[i][j] for j in range(3)] for i in range(3)]) / det for col in range(3)]
            if x[2] > D(1e-9):
                sols.append(((x[0], x[1]), x[2]))
            continue
        # Two lines leave c = p + q r; the circle |c − C| = R + s r (s = 1, −1) or r − R.
        (a1, b1, k1, d1), (a2, b2, k2, d2) = rows[:2]
        den = a1 * b2 - a2 * b1
        if den == 0:
            continue
        px, py = (d1 * b2 - d2 * b1) / den, (a1 * d2 - a2 * d1) / den
        qx, qy = (-k1 * b2 + k2 * b1) / den, (-a1 * k2 + a2 * k1) / den
        (cx, cy), big = circles[0][1], circles[0][2]
        for kind in ("out", "in", "around"):
            # |p + q r − C|² = (R + s r)²: a r² + b r + c = 0.
            ex, ey = px - cx, py - cy
            s_, off = {"out": (1, big), "in": (-1, big), "around": (1, -big)}[kind]
            a = qx * qx + qy * qy - 1
            b = 2 * (ex * qx + ey * qy) - 2 * s_ * off
            c = ex * ex + ey * ey - off * off
            disc = b * b - 4 * a * c
            if a == 0 or disc < 0:
                continue
            for r in ((-b + sqrt(disc)) / (2 * a), (-b - sqrt(disc)) / (2 * a)):
                if r > D(1e-9) and (kind == "out" or (big - r > 0) == (kind == "in")):
                    sols.append(((px + qx * r, py + qy * r), r))
    best = None
    for c, r in sols:
        score = sum(dist(touch(s, c), n) for s, n in zip(ss, ns))
        if best is None or score < best[0]:
            best = (score, c, r)
    return None if best is None else (best[1], best[2])


def det3(m):
    return (m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]))


# ── Arcs ──────────────────────────────────────────────────────────────────────
def ang(c, p):
    return atan2(dec(p[1]) - dec(c[1]), dec(p[0]) - dec(c[0]))


def arc_by_sweep(s, e, theta):
    """From s to e turning theta (signed): the stored counter-clockwise arc."""
    theta = dec(theta)
    sx, sy, ex, ey = map(dec, (s[0], s[1], e[0], e[1]))
    dx, dy = ex - sx, ey - sy
    d = sqrt(dx * dx + dy * dy)
    k = (d / 2) / tan(theta / 2) / d
    c = ((sx + ex) / 2 - dy * k, (sy + ey) / 2 + dx * k)
    r = abs((d / 2) / sin(theta / 2))
    if theta > 0:
        return arc_out(c, r, ang(c, s), ang(c, e))
    return arc_out(c, r, ang(c, e), ang(c, s))


def arc_three(s, m, e):
    c, r = circumcentre(s, m, e)
    a_s, a_m, a_e = ang(c, s), ang(c, m), ang(c, e)
    if norm(a_m - a_s) < norm(a_e - a_s):
        return arc_out(c, r, a_s, a_e)
    return arc_out(c, r, a_e, a_s)


def direction_sweep(s, e, phi):
    """Twice the signed angle from the direction phi to the chord s → e."""
    ux, uy = cos(phi), sin(phi)
    cx, cy = dec(e[0]) - dec(s[0]), dec(e[1]) - dec(s[1])
    return 2 * atan2(ux * cy - uy * cx, ux * cx + uy * cy)


# ── Messages ──────────────────────────────────────────────────────────────────
THREE_APART = "Üç noktadan ikisi aynı yerde; aralarından daire geçmez. Ayrı üç nokta verin."
ENDS_APART = "Başlangıç ve bitiş aynı yerde; aralarında yay yok. Ayrı iki nokta verin."
START_CENTRE = "Başlangıç merkezle aynı yerde; yarıçap sıfır olur. Ayrı noktalar verin."
ON_TANGENT = ("Bitiş noktası başlangıçtaki teğetin doğrusunda; yay değil doğru olur. Doğrunun dışında bir bitiş "
              "verin.")
ARC_ANGLE = ("Yay açısı sıfır ya da tam tur ve daha büyük olamaz; ikisinin arasında bir açı verin (eksi: saat "
             "yönünde).")
NOT_TANGENT = ("Teğet daire yalnız çizgiye, çoklu çizgiye, alana, yaya, daireye, yardımcı çizgiye ya da ışına "
               "değer; bu nesneye değmez.")
NOT_AN_END = ("Devam çizginin, çoklu çizginin ya da yayın bir ucundan sürer: başlangıcı (start) ya da bitişi "
              "(end); kapalı çoklu çizginin yalnız bitişi.")
NO_PIECE = ("Blok yolunda böyle bir parça yok: yolun bir sırası tanımın dışında, yol bir yerleştirmede bitiyor ya da "
            "yerleştirme olmayan bir nesneden geçiyor.")
C0 = "constructions[0]"


def circle_cases():
    out = []
    out.append(case("2 nokta: 6–8–10 çapının dairesi",
                    step([{"kind": "circleTwoPoints", "a": at(0, 0), "b": at(6, 8)}],
                         done([circle_out((3, 4), 5)]))))
    out.append(case("2 nokta, yerleriyle: çizginin iki ucu",
                    step([{"kind": "circleTwoPoints", "a": place(1, START), "b": place(1, END)}],
                         done([circle_out((0, 0), 20)]))))
    out.append(case("2 nokta aynı yerde: same_points",
                    step([{"kind": "circleTwoPoints", "a": at(0, 0), "b": at(0, 0)}],
                         failed("same_points", "Çapın iki ucu aynı yerde; daire oluşmaz. Ayrı iki nokta verin.",
                                f"{C0}.b"))))
    out.append(case("3 nokta: dik üçgenin çevrel dairesi",
                    step([{"kind": "circleThreePoints", "a": at(0, 0), "b": at(6, 0), "c": at(0, 8)}],
                         done([circle_out((3, 4), 5)]))))
    pts = ((1, 2), (7, 3), (4, 9))
    c, r = circumcentre(*pts)
    out.append(case("3 nokta: genel üçgen, merkez ve yarıçap 70 basamakta",
                    step([{"kind": "circleThreePoints", "a": at(*pts[0]), "b": at(*pts[1]), "c": at(*pts[2])}],
                         done([circle_out(c, r)]))))
    out.append(case("3 noktadan ikisi aynı: same_points",
                    step([{"kind": "circleThreePoints", "a": at(0, 0), "b": at(5, 5), "c": at(5, 5)}],
                         failed("same_points", THREE_APART, f"{C0}.c"))))
    assert collinear((0, 0), (1, 1), (5, 5))
    out.append(case("3 nokta aynı doğruda: collinear_points",
                    step([{"kind": "circleThreePoints", "a": at(0, 0), "b": at(1, 1), "c": at(5, 5)}],
                         failed("collinear_points",
                                "Üç nokta aynı doğru üzerinde; daire geçmez. Doğrunun dışında bir nokta verin.",
                                f"{C0}.c"))))
    far = (2000, 0.000001)
    assert collinear((0, 0), (1000, 0), far)
    out.append(case("3 nokta neredeyse aynı doğruda (açının sinüsü 10⁻⁹'dan küçük): collinear_points",
                    step([{"kind": "circleThreePoints", "a": at(0, 0), "b": at(1000, 0), "c": at(*far)}],
                         failed("collinear_points",
                                "Üç nokta aynı doğru üzerinde; daire geçmez. Doğrunun dışında bir nokta verin.",
                                f"{C0}.c"))))
    # Teğet, teğet, yarıçap.
    got = ttr(support(1), (4, 0.5), support(2), (0.5, 4), 3)
    assert got == ((3, 3), 3), got
    out.append(case("teğet-teğet-yarıçap: iki dik doğru, 3 m, yakın noktaların çeyreğinde",
                    step([{"kind": "circleTangentTangentRadius", "first": near(1, 4, 0.5), "second": near(2, 0.5, 4),
                           "r": 3}], done([circle_out(*got)]))))
    got = ttr(support(1), (-4, -0.5), support(2), (-0.5, -4), 3)
    out.append(case("teğet-teğet-yarıçap: aynı doğrular, öbür çeyrek",
                    step([{"kind": "circleTangentTangentRadius", "first": near(1, -4, -0.5),
                           "second": near(2, -0.5, -4), "r": 3}], done([circle_out(*got)]))))
    got = ttr(support(1), (22, 0.5), support(4), (26, 2), 2.4)
    out.append(case("teğet-teğet-yarıçap: doğru ve daire, dışından",
                    step([{"kind": "circleTangentTangentRadius", "first": near(1, 22, 0.5), "second": near(4, 26, 2),
                           "r": 2.4}], done([circle_out(*got)]))))
    got = ttr(support(4), (30, 4), support(2), (0.5, 8), 20)
    out.append(case("teğet-teğet-yarıçap: daireyi içine alan büyük daire ve doğru",
                    step([{"kind": "circleTangentTangentRadius", "first": near(4, 30, 4), "second": near(2, 0.5, 8),
                           "r": 20}], done([circle_out(*got)]))))
    got = ttr(support(1), (-6, 0.2), support(3), (2, 9.5), 5)
    assert got == ((-2, 5), 5), got
    out.append(case("teğet-teğet-yarıçap: 10 m arayla iki paralel doğru ve 5 m; merkez yakın noktaların ortasının "
                    "karşısında",
                    step([{"kind": "circleTangentTangentRadius", "first": near(1, -6, 0.2), "second": near(3, 2, 9.5),
                           "r": 5}], done([circle_out(*got)]))))
    out.append(case("teğet-teğet-yarıçap: paralel doğrular ve başka yarıçap: no_tangent_circle",
                    step([{"kind": "circleTangentTangentRadius", "first": near(1, -6, 0.2), "second": near(3, 2, 9.5),
                           "r": 3}],
                         failed("no_tangent_circle",
                                "İki doğru paralel; yalnız aralarındaki uzaklığın yarısı olan yarıçapla ikisine birden "
                                "teğet daire olur.", f"{C0}.r"))))
    assert ttr(support(4), (30, 4), support(2), (0.5, 8), 1) is None
    out.append(case("teğet-teğet-yarıçap: daireye ve uzaktaki doğruya küçük yarıçap ulaşmaz: no_tangent_circle",
                    step([{"kind": "circleTangentTangentRadius", "first": near(4, 30, 4), "second": near(2, 0.5, 8),
                           "r": 1}],
                         failed("no_tangent_circle",
                                "Bu yarıçapla iki nesneye birden teğet bir daire yok; yarıçapı değiştirin.",
                                f"{C0}.r"))))
    out.append(case("teğet-teğet-yarıçap: sıfır yarıçap: invalid_radius",
                    step([{"kind": "circleTangentTangentRadius", "first": near(1, 4, 0.5), "second": near(2, 0.5, 4),
                           "r": 0}],
                         failed("invalid_radius", "Yarıçap sıfırdan büyük olmalı. Pozitif bir yarıçap verin.",
                                f"{C0}.r"))))
    out.append(case("teğet-teğet-yarıçap: noktaya teğet olunmaz: not_tangent_kind",
                    step([{"kind": "circleTangentTangentRadius", "first": near(8, 80, 10), "second": near(2, 0.5, 4),
                           "r": 3}], failed("not_tangent_kind", NOT_TANGENT, f"{C0}.first"))))
    # Teğet, teğet, teğet.
    got = ttt([support(1), support(2), support(10)], [(3.5, 0.2), (0.2, 2.5), (5, 5.3)])
    assert got == ((3, 3), 3), got
    out.append(case("teğet-teğet-teğet: 9–12–15 üçgeninin iç teğet dairesi",
                    step([{"kind": "circleTangents", "first": near(1, 3.5, 0.2), "second": near(2, 0.2, 2.5),
                           "third": near(10, 5, 5.3)}], done([circle_out(*got)], iterated=True))))
    got = ttt([support(1), support(2), support(10)], [(17, 0.3), (0.3, 17), (7, 3.8)])
    assert got == ((18, 18), 18), got
    out.append(case("teğet-teğet-teğet: aynı üçgenin hipotenüse dıştan teğet dairesi",
                    step([{"kind": "circleTangents", "first": near(1, 17, 0.3), "second": near(2, 0.3, 17),
                           "third": near(10, 7, 3.8)}], done([circle_out(*got)], iterated=True))))
    got = ttt([support(1), support(3), support(2)], [(5, 0.3), (5, 9.7), (0.2, 5)])
    assert got == ((5, 5), 5), got
    out.append(case("teğet-teğet-teğet: iki paralel doğru ve onları kesen doğru",
                    step([{"kind": "circleTangents", "first": near(1, 5, 0.3), "second": near(3, 5, 9.7),
                           "third": near(2, 0.2, 5)}], done([circle_out(*got)], iterated=True))))
    got = ttt([support(1), support(2), support(4)], [(16, 0.5), (0.5, 16), (26.8, 3.8)])
    out.append(case("teğet-teğet-teğet: iki doğru ve bir daire",
                    step([{"kind": "circleTangents", "first": near(1, 16, 0.5), "second": near(2, 0.5, 16),
                           "third": near(4, 26.8, 3.8)}], done([circle_out(*got)]))))
    assert ttt([support(1), support(3), support(12)], [(0, 0), (0, 10), (0, 20)]) is None
    out.append(case("teğet-teğet-teğet: üç paralel doğru: no_tangent_circle",
                    step([{"kind": "circleTangents", "first": near(1, 0, 0.2), "second": near(3, 0, 9.8),
                           "third": near(12, 0, 19.8)}],
                         failed("no_tangent_circle", "Üç doğru paralel; üçüne birden teğet daire yok.", C0))))
    out.append(case("teğet-teğet-teğet: elipse teğet olunmaz: not_tangent_kind",
                    step([{"kind": "circleTangents", "first": near(1, 3.5, 0.2), "second": near(2, 0.2, 2.5),
                           "third": near(11, 108, 0)}], failed("not_tangent_kind", NOT_TANGENT, f"{C0}.third"))))
    return out


def arc_cases():
    out = []
    out.append(case("3 nokta: doğudan kuzeyden batıya, saat yönünün tersine",
                    step([{"kind": "arcThreePoints", "start": at(10, 0), "mid": at(0, 10), "end": at(-10, 0)}],
                         done([arc_three((10, 0), (0, 10), (-10, 0))]))))
    a = arc_three((-10, 0), (0, 10), (10, 0))
    assert a == arc_three((10, 0), (0, 10), (-10, 0))
    out.append(case("3 nokta saat yönünde verilince aynı yay saklanır",
                    step([{"kind": "arcThreePoints", "start": at(-10, 0), "mid": at(0, 10), "end": at(10, 0)}],
                         done([a]))))
    out.append(case("3 nokta: ortadaki nokta güneyde, alttaki yarım daire",
                    step([{"kind": "arcThreePoints", "start": at(10, 0), "mid": at(0, -10), "end": at(-10, 0)}],
                         done([arc_three((10, 0), (0, -10), (-10, 0))]))))
    out.append(case("3 nokta: genel üç nokta, merkez 70 basamakta",
                    step([{"kind": "arcThreePoints", "start": at(1, 2), "mid": at(4, 9), "end": at(7, 3)}],
                         done([arc_three((1, 2), (4, 9), (7, 3))]))))
    out.append(case("3 nokta aynı doğruda: collinear_points",
                    step([{"kind": "arcThreePoints", "start": at(0, 0), "mid": at(2, 1), "end": at(6, 3)}],
                         failed("collinear_points",
                                "Üç nokta aynı doğru üzerinde; yay geçmez. Doğrunun dışında bir nokta verin.",
                                f"{C0}.end"))))
    out.append(case("başlangıç–merkez–bitiş: çeyrek, bitiş yalnız doğrultuyu verir",
                    step([{"kind": "arcStartCentreEnd", "start": at(10, 0), "centre": at(0, 0), "end": at(0, 5)}],
                         done([arc_out((0, 0), 10, ang((0, 0), (10, 0)), ang((0, 0), (0, 5)))]))))
    out.append(case("başlangıç–merkez–bitiş: yaya devam, yerleriyle (yayın bitişi ve merkezi)",
                    step([{"kind": "arcStartCentreEnd", "start": place(5, END), "centre": place(5, CENTER),
                           "end": at(50, -1)}],
                         done([arc_out((60, 0), 10 * 1, ang((60, 0), (dec(60) + 10 * cos(math.pi / 2), 10 * sin(math.pi / 2))),
                                       ang((60, 0), (50, -1)))]))))
    out.append(case("başlangıç–merkez–bitiş: bitiş başlangıcın doğrultusunda: invalid_angle",
                    step([{"kind": "arcStartCentreEnd", "start": at(10, 0), "centre": at(0, 0), "end": at(20, 0)}],
                         failed("invalid_angle",
                                "Bitiş başlangıçla aynı doğrultuda; yay oluşmaz (tam daire Daire'yle çizilir).",
                                f"{C0}.end"))))
    out.append(case("başlangıç–merkez–bitiş: başlangıç merkezde: same_points",
                    step([{"kind": "arcStartCentreEnd", "start": at(0, 0), "centre": at(0, 0), "end": at(5, 5)}],
                         failed("same_points", START_CENTRE, f"{C0}.centre"))))
    out.append(case("başlangıç–merkez–bitiş: bitiş merkezde: same_points",
                    step([{"kind": "arcStartCentreEnd", "start": at(10, 0), "centre": at(0, 0), "end": at(0, 0)}],
                         failed("same_points",
                                "Bitiş merkezle aynı yerde; bitişin doğrultusu yok. Merkezden ayrı bir bitiş verin.",
                                f"{C0}.end"))))
    half = math.pi / 2
    out.append(case("başlangıç–merkez–açı: π/2 saat yönünün tersine",
                    step([{"kind": "arcStartCentreAngle", "start": at(10, 0), "centre": at(0, 0), "angle": half}],
                         done([arc_out((0, 0), 10, 0, D(half))]))))
    out.append(case("başlangıç–merkez–açı: eksi π/2 saat yönünde, saklanan bitişten başlar",
                    step([{"kind": "arcStartCentreAngle", "start": at(10, 0), "centre": at(0, 0), "angle": -half}],
                         done([arc_out((0, 0), 10, -D(half), 0)]))))
    out.append(case("başlangıç–merkez–açı: tam tur: invalid_angle",
                    step([{"kind": "arcStartCentreAngle", "start": at(10, 0), "centre": at(0, 0),
                           "angle": 2 * math.pi}], failed("invalid_angle", ARC_ANGLE, f"{C0}.angle"))))
    out.append(case("başlangıç–merkez–açı: sıfır: invalid_angle",
                    step([{"kind": "arcStartCentreAngle", "start": at(10, 0), "centre": at(0, 0), "angle": 0}],
                         failed("invalid_angle", ARC_ANGLE, f"{C0}.angle"))))
    sw = 2 * asin(D("0.5"))
    out.append(case("başlangıç–merkez–kiriş: 10 m kiriş yarıçap 10'da 60°",
                    step([{"kind": "arcStartCentreChord", "start": at(10, 0), "centre": at(0, 0), "chord": 10}],
                         done([arc_out((0, 0), 10, 0, sw)]))))
    out.append(case("başlangıç–merkez–kiriş: eksi kiriş büyük yayı verir",
                    step([{"kind": "arcStartCentreChord", "start": at(10, 0), "centre": at(0, 0), "chord": -10}],
                         done([arc_out((0, 0), 10, 0, TAU - sw)]))))
    out.append(case("başlangıç–merkez–kiriş: çaptan uzun: chord_too_long",
                    step([{"kind": "arcStartCentreChord", "start": at(10, 0), "centre": at(0, 0), "chord": 25}],
                         failed("chord_too_long", "Kiriş dairenin çapından uzun; en çok çap kadar bir kiriş verin.",
                                f"{C0}.chord"))))
    out.append(case("başlangıç–merkez–kiriş: sıfır: invalid_distance",
                    step([{"kind": "arcStartCentreChord", "start": at(10, 0), "centre": at(0, 0), "chord": 0}],
                         failed("invalid_distance",
                                "Kiriş sıfır olamaz; sıfırdan farklı bir kiriş verin (eksi: büyük yay).",
                                f"{C0}.chord"))))
    out.append(case("başlangıç–bitiş–açı: π/2, merkez kirişin solunda",
                    step([{"kind": "arcStartEndAngle", "start": at(10, 0), "end": at(0, 10), "angle": half}],
                         done([arc_by_sweep((10, 0), (0, 10), half)]))))
    out.append(case("başlangıç–bitiş–açı: eksi 3π/2, saat yönünde büyük yay",
                    step([{"kind": "arcStartEndAngle", "start": at(10, 0), "end": at(0, 10), "angle": -3 * half}],
                         done([arc_by_sweep((10, 0), (0, 10), -3 * half)]))))
    out.append(case("başlangıç–bitiş–açı: başlangıç ve bitiş aynı: same_points",
                    step([{"kind": "arcStartEndAngle", "start": at(10, 0), "end": at(10, 0), "angle": half}],
                         failed("same_points", ENDS_APART, f"{C0}.end"))))
    out.append(case("başlangıç–bitiş–yön: kuzeye çıkan yarım daire",
                    step([{"kind": "arcStartEndDirection", "start": at(10, 0), "end": at(-10, 0), "direction": half}],
                         done([arc_by_sweep((10, 0), (-10, 0), direction_sweep((10, 0), (-10, 0), half))]))))
    out.append(case("başlangıç–bitiş–yön: kuzeydoğuya çıkan yay",
                    step([{"kind": "arcStartEndDirection", "start": at(0, 0), "end": at(10, 0),
                           "direction": math.pi / 4}],
                         done([arc_by_sweep((0, 0), (10, 0), direction_sweep((0, 0), (10, 0), math.pi / 4))]))))
    out.append(case("başlangıç–bitiş–yön: bitiş teğetin doğrusunda: collinear_points",
                    step([{"kind": "arcStartEndDirection", "start": at(10, 0), "end": at(-10, 0),
                           "direction": math.pi}], failed("collinear_points", ON_TANGENT, f"{C0}.end"))))
    big = D("12.5")
    out.append(case("başlangıç–bitiş–yarıçap: 20 m kiriş, 12,5 m küçük yay",
                    step([{"kind": "arcStartEndRadius", "start": at(10, 0), "end": at(-10, 0), "r": 12.5}],
                         done([arc_by_sweep((10, 0), (-10, 0), 2 * asin(10 / big))]))))
    out.append(case("başlangıç–bitiş–yarıçap: eksi yarıçap büyük yay",
                    step([{"kind": "arcStartEndRadius", "start": at(10, 0), "end": at(-10, 0), "r": -12.5}],
                         done([arc_by_sweep((10, 0), (-10, 0), TAU - 2 * asin(10 / big))]))))
    out.append(case("başlangıç–bitiş–yarıçap: kirişin yarısından küçük: radius_too_small",
                    step([{"kind": "arcStartEndRadius", "start": at(10, 0), "end": at(-10, 0), "r": 5}],
                         failed("radius_too_small",
                                "Yarıçap iki uç arasındaki kirişin yarısından küçük; en az kirişin yarısı kadar bir "
                                "yarıçap verin.", f"{C0}.r"))))
    out.append(case("başlangıç–bitiş–yarıçap: sıfır: invalid_radius",
                    step([{"kind": "arcStartEndRadius", "start": at(10, 0), "end": at(-10, 0), "r": 0}],
                         failed("invalid_radius",
                                "Yarıçap sıfır olamaz; sıfırdan farklı bir yarıçap verin (eksi: büyük yay).",
                                f"{C0}.r"))))
    # Devam.
    out.append(case("Devam: çizginin bitişinden doğuya, sola dönen yay",
                    step([{"kind": "arcContinue", "from": obj_end(1, END), "end": at(30, 10)}],
                         done([arc_by_sweep((20, 0), (30, 10), direction_sweep((20, 0), (30, 10), 0))]))))
    out.append(case("Devam: çizginin başlangıcından geriye, batıya, saat yönünde",
                    step([{"kind": "arcContinue", "from": obj_end(1, START), "end": at(-30, 10)}],
                         done([arc_by_sweep((-20, 0), (-30, 10), direction_sweep((-20, 0), (-30, 10), PI))]))))
    a1 = D(math.pi / 2)
    end5 = (60 + 10 * cos(a1), 10 * sin(a1))
    out.append(case("Devam: yayın bitişinden aynı çember üzerinde",
                    step([{"kind": "arcContinue", "from": obj_end(5, END), "end": at(50, 0)}],
                         done([arc_by_sweep(end5, (50, 0), direction_sweep(end5, (50, 0), a1 + PI / 2))]))))
    out.append(case("Devam: açık çoklu çizginin son köşesinden kuzeye",
                    step([{"kind": "arcContinue", "from": obj_end(6, vertex(2)), "end": at(0, 50)}],
                         done([arc_by_sweep((10, 40), (0, 50), direction_sweep((10, 40), (0, 50), PI / 2))]))))
    out.append(case("Devam: kapalı çoklu çizginin bitişi kapanan kenarın ucu",
                    step([{"kind": "arcContinue", "from": obj_end(9, END), "end": at(30, 20)}],
                         done([arc_by_sweep((40, 20), (30, 20),
                                            direction_sweep((40, 20), (30, 20), atan2(-10, -10)))]))))
    out.append(case("Devam: kapalı çoklu çizginin başlangıcı yok: not_an_end",
                    step([{"kind": "arcContinue", "from": obj_end(9, START), "end": at(30, 20)}],
                         failed("not_an_end", NOT_AN_END, f"{C0}.from.place"))))
    out.append(case("Devam: çoklu çizginin içteki köşesi uç değil: not_an_end",
                    step([{"kind": "arcContinue", "from": obj_end(6, vertex(1)), "end": at(0, 50)}],
                         failed("not_an_end", NOT_AN_END, f"{C0}.from.place"))))
    out.append(case("Devam: daireden sürülmez: not_an_end",
                    step([{"kind": "arcContinue", "from": obj_end(4, END), "end": at(0, 50)}],
                         failed("not_an_end", NOT_AN_END, f"{C0}.from"))))
    out.append(case("Devam: bitiş teğetin doğrusunda: collinear_points",
                    step([{"kind": "arcContinue", "from": obj_end(1, END), "end": at(40, 0)}],
                         failed("collinear_points", ON_TANGENT, f"{C0}.end"))))
    out.append(case("Devam: bitiş sürülen uçta: same_points",
                    step([{"kind": "arcContinue", "from": obj_end(1, END), "end": at(20, 0)}],
                         failed("same_points", ENDS_APART, f"{C0}.end"))))
    out.append(case("Devam: yerleştirme olmayan nesnede blok yolu: piece_gone",
                    step([{"kind": "arcContinue", "from": obj_end(1, END, [0]), "end": at(30, 10)}],
                         failed("piece_gone", NO_PIECE, f"{C0}.from.path"))))
    return out


def ellipse(c, v, other):
    """Centre, one half-axis vector, the other half-axis: the stored ellipse's centre, major and ratio."""
    a = sqrt(dec(v[0]) ** 2 + dec(v[1]) ** 2)
    b = abs(dec(other))
    if b <= a:
        return c, (dec(v[0]), dec(v[1])), b / a
    return c, (-dec(v[1]) / a * b, dec(v[0]) / a * b), a / b


def param(ratio, phi):
    return norm(atan2(sin(phi) / ratio, cos(phi)))


def ellipse_cases():
    out = []
    c, m, k = ellipse((10, 0), (10, 0), 4)
    out.append(case("eksen–uç: 20 m eksen ve 4 m diğer yarı eksen",
                    step([{"kind": "ellipseAxis", "a": at(0, 0), "b": at(20, 0), "other": 4}],
                         done([ellipse_out(c, m, k, 0, 0)]))))
    c, m, k = ellipse((10, 0), (10, 0), 15)
    out.append(case("eksen–uç: diğer yarı eksen daha uzun: büyük eksen ona döner",
                    step([{"kind": "ellipseAxis", "a": at(0, 0), "b": at(20, 0), "other": 15}],
                         done([ellipse_out(c, m, k, 0, 0)]))))
    rot = math.pi / 3
    c, m, k = ellipse((0, 0), (0, 10), 10 * cos(rot))
    out.append(case("merkez: döndürme π/3 ile diğer yarı eksen a·cos",
                    step([{"kind": "ellipseCentre", "centre": at(0, 0), "axisEnd": at(0, 10), "rotation": rot}],
                         done([ellipse_out(c, m, k, 0, 0)]))))
    c, m, k = ellipse((10, 0), (10, 0), 4)
    out.append(case("eliptik yay: büyük eksenden 0 ile π/2 arası",
                    step([{"kind": "ellipseAxis", "a": at(0, 0), "b": at(20, 0), "other": 4, "startAngle": 0,
                           "endAngle": math.pi / 2}],
                         done([ellipse_out(c, m, k, param(k, 0), param(k, math.pi / 2))]))))
    c, m, k = ellipse((0, 0), (6, 8), 5)
    out.append(case("eliptik yay: dönük eksen, kutupsal açılar parametreye çevrilir",
                    step([{"kind": "ellipseCentre", "centre": at(0, 0), "axisEnd": at(6, 8), "other": 5,
                           "startAngle": math.pi / 6, "endAngle": 2.5}],
                         done([ellipse_out(c, m, k, param(k, math.pi / 6), param(k, 2.5))]))))
    out.append(case("eliptik yay: başlangıç ve bitiş aynı: invalid_angle",
                    step([{"kind": "ellipseAxis", "a": at(0, 0), "b": at(20, 0), "other": 4, "startAngle": 1,
                           "endAngle": 1}],
                         failed("invalid_angle",
                                "Eliptik yayın bitiş açısı başlangıcıyla aynı yerde olamaz; başka bir bitiş açısı "
                                "verin.", f"{C0}.endAngle"))))
    out.append(case("eliptik yay: yalnız başlangıç: invalid_angle",
                    step([{"kind": "ellipseAxis", "a": at(0, 0), "b": at(20, 0), "other": 4, "startAngle": 1}],
                         failed("invalid_angle",
                                "Eliptik yayın başlangıç ve bitiş açısı birlikte verilir; ikisini de verin ya da "
                                "ikisini de kaldırın.", f"{C0}.endAngle"))))
    out.append(case("diğer yarı eksen ve döndürme birlikte: invalid_half_axis",
                    step([{"kind": "ellipseAxis", "a": at(0, 0), "b": at(20, 0), "other": 4, "rotation": 0.5}],
                         failed("invalid_half_axis",
                                "Diğer yarı eksen ya uzunluğuyla (other) ya döndürme açısıyla (rotation) verilir; "
                                "ikisi birlikte verilmez.", f"{C0}.rotation"))))
    out.append(case("diğer yarı eksen de döndürme de yok: invalid_half_axis",
                    step([{"kind": "ellipseCentre", "centre": at(0, 0), "axisEnd": at(0, 10)}],
                         failed("invalid_half_axis",
                                "Diğer yarı ekseni uzunluğuyla (other) ya da döndürme açısıyla (rotation) verin.",
                                f"{C0}.other"))))
    out.append(case("diğer yarı eksen eksi: invalid_half_axis",
                    step([{"kind": "ellipseAxis", "a": at(0, 0), "b": at(20, 0), "other": -4}],
                         failed("invalid_half_axis", "Diğer yarı eksen sıfırdan büyük olmalı.", f"{C0}.other"))))
    out.append(case("döndürme 89,4°'den büyük: invalid_angle",
                    step([{"kind": "ellipseAxis", "a": at(0, 0), "b": at(20, 0), "rotation": 1.6}],
                         failed("invalid_angle", "Döndürme açısı 0 ile 89,4° (1,5603 radyan) arasında olmalı.",
                                f"{C0}.rotation"))))
    out.append(case("eksenin iki ucu aynı: same_points",
                    step([{"kind": "ellipseAxis", "a": at(0, 0), "b": at(0, 0), "other": 4}],
                         failed("same_points",
                                "Eksenin iki ucu aynı yerde; eksen sıfır uzunlukta. Ayrı iki nokta verin.",
                                f"{C0}.b"))))
    return out


def common_cases():
    out = []
    out.append(case("birkaç yapım sırasıyla; plan aynı cevabı verir, validate yalnız denetler",
                    step([{"kind": "circleTwoPoints", "a": at(0, 0), "b": at(6, 8)},
                          {"kind": "arcStartCentreEnd", "start": at(10, 0), "centre": at(0, 0), "end": at(0, 5)}],
                         done([circle_out((3, 4), 5),
                               arc_out((0, 0), 10, ang((0, 0), (10, 0)), ang((0, 0), (0, 5)))]), op="plan"),
                    step([{"kind": "circleTwoPoints", "a": at(0, 0), "b": at(6, 8)}],
                         {"status": "completed", "output": None, "warnings": []}, op="validate")))
    out.append(case("ikinci yapım reddedilince hiçbir eğri verilmez; yer ikinci yapımındır",
                    step([{"kind": "circleTwoPoints", "a": at(0, 0), "b": at(6, 8)},
                          {"kind": "arcContinue", "from": obj_end(4, END), "end": at(0, 50)}],
                         failed("not_an_end", NOT_AN_END, "constructions[1].from"))))
    out.append(case("hiç yapım yok: no_constructions",
                    step([], failed("no_constructions", "0 yapım verildi: bir ile 10 000 arasında yapım verin.",
                                    "constructions"))))
    out.append(case("kimliğin yazımı: invalid_uid",
                    step([{"kind": "circleTangents", "first": {"uid": "ABC", "near": rel(0, 0)},
                           "second": near(2, 0, 1), "third": near(10, 5, 5)}],
                         failed("invalid_uid",
                                "“ABC” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir "
                                "(01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi).", f"{C0}.first.uid"))))
    out.append(case("çizimde olmayan nesne: entity_not_found",
                    step([{"kind": "circleTwoPoints", "a": {"kind": "place", "uid": MISSING, "place": END},
                           "b": at(0, 0)}],
                         failed("entity_not_found",
                                f"“{MISSING}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. "
                                "Var olan bir nesnenin kimliğini verin.", f"{C0}.a.uid"))))
    out.append(case("nesnede olmayan yer: no_such_place",
                    step([{"kind": "arcThreePoints", "start": place(1, CENTER), "mid": at(0, 10), "end": at(-10, 0)}],
                         failed("no_such_place",
                                "Nesnede böyle bir yer yok: köşe sırası halkanın dışında ya da yer bu nesne türünde "
                                "bulunmuyor.", f"{C0}.start.place"))))
    out.append(case("sayılar sonlu olmalı: önce noktalar, sonra açı ve yarıçap",
                    step([{"kind": "arcStartCentreAngle", "start": at(10, 0), "centre": at(0, 0), "angle": 1}],
                         failed("not_finite",
                                "Noktanın kuzey (X) değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu "
                                "bir sayıyla verin.", f"{C0}.centre.at.y"),
                         nonFinite={f"{C0}.centre.at.y": "NaN", f"{C0}.angle": "Infinity"}),
                    step([{"kind": "arcStartCentreAngle", "start": at(10, 0), "centre": at(0, 0), "angle": 1}],
                         failed("not_finite",
                                "Açı sonlu bir sayı değil (NaN ya da sonsuz). Radyan olarak bir sayı verin.",
                                f"{C0}.angle"), nonFinite={f"{C0}.angle": "Infinity"}),
                    step([{"kind": "circleTangentTangentRadius", "first": near(1, 4, 0.5), "second": near(2, 0.5, 4),
                           "r": 3}],
                         failed("not_finite",
                                "Yarıçap sonlu bir sayı değil (NaN ya da sonsuz). Metre olarak bir sayı verin.",
                                f"{C0}.r"), nonFinite={f"{C0}.r": "NaN"})))
    return out


# ── The tools, end to end ────────────────────────────────────────────────────
TOOLS = ROOT / "fixtures" / "curve-methods" / "v1" / "tools.json"


def rnum(x):
    f = float(dec(x))
    return int(f) if f.is_integer() else f


def r_circle(c, r):
    return {"circle": {"c": [rnum(c[0]), rnum(c[1])], "r": rnum(r)}}


def r_arc(c, r, a0, a1):
    return {"arc": {"c": [rnum(c[0]), rnum(c[1])], "r": rnum(r), "a0": rnum(turn(a0)), "a1": rnum(turn(a1))}}


def r_arc_of(a):
    return {"arc": {"c": [a["c"]["x"] - E0, a["c"]["y"] - N0], "r": a["r"], "a0": a["a0"], "a1": a["a1"]}}


def r_arc_sweep(s, e, theta):
    return r_arc_of(arc_by_sweep(s, e, theta)[0])


def r_arc_three(s, m, e):
    return r_arc_of(arc_three(s, m, e)[0])


def r_ellipse(c, major, ratio, t0=0, t1=0):
    return {"ellipse": {"c": [rnum(c[0]), rnum(c[1])], "major": [rnum(major[0]), rnum(major[1])], "ratio": rnum(ratio),
                        "t0": rnum(t0), "t1": rnum(t1)}}


def deg(n):
    return dec(n) * PI / 180


def tool_case(name, tool, steps, expect, objects=()):
    return {"name": name, "tool": tool, "objects": list(objects), "steps": steps, "expect": expect}


def C(x, y):
    return {"click": [x, y]}


def T(text):
    return {"type": text}


def tool_cases():
    out = []
    # Daire.
    out.append(tool_case("Daire: merkez tıklanır, yarıçap noktayla", "circle", [C(0, 0), C(3, 4)], r_circle((0, 0), 5)))
    out.append(tool_case("Daire: merkez, yazılan yarıçap", "circle", [C(2, 1), T("7.5")], r_circle((2, 1), 7.5)))
    out.append(tool_case("Daire: Çap (Ç) ile yazılan çap", "circle", [C(0, 0), T("Ç"), T("12")], r_circle((0, 0), 6)))
    out.append(tool_case("Daire: şeridin Merkez, çap yöntemi", "circle", [T("MÇ"), C(1, 1), T("10")],
                         r_circle((1, 1), 5)))
    out.append(tool_case("Daire: 2N, tıklanan iki uç", "circle", [T("2N"), C(0, 0), C(6, 8)], r_circle((3, 4), 5)))
    out.append(tool_case("Daire: 2N, yazılan iki uç", "circle", [T("2N"), T(f"{E0},{N0}"), T(f"{E0 + 6},{N0 + 8}")],
                         r_circle((3, 4), 5)))
    c, r = circumcentre((1, 2), (7, 3), (4, 9))
    out.append(tool_case("Daire: 3N, genel üç nokta", "circle", [T("3N"), C(1, 2), C(7, 3), C(4, 9)], r_circle(c, r)))
    out.append(tool_case("Daire: 3N aynı doğruda uyarır", "circle", [T("3N"), C(0, 0), C(1, 1), C(5, 5)],
                         {"warn": "Üç nokta aynı doğru üzerinde; daire geçmez. Doğrunun dışında bir nokta verin."}))
    lines = [{"line": [[-20, 0], [20, 0]]}, {"line": [[0, -20], [0, 20]]}]
    out.append(tool_case("Daire: TTY, iki dik doğru ve yazılan yarıçap", "circle",
                         [T("TTY"), C(4, 0), C(0, 4), T("3")], r_circle((3, 3), 3), lines))
    par = [{"line": [[-20, 0], [20, 0]]}, {"line": [[-20, 10], [20, 10]]}]
    got = ttr(("line", (D(-20), D(0)), (D(1), D(0))), (-6, 0), ("line", (D(-20), D(10)), (D(1), D(0))), (2, 10), 5)
    out.append(tool_case("Daire: TTY, 10 m arayla paralel doğrular ve 5 m", "circle",
                         [T("TTY"), C(-6, 0), C(2, 10), T("5")], r_circle(*got), par))
    out.append(tool_case("Daire: TTY, paralel doğrular ve başka yarıçap uyarır", "circle",
                         [T("TTY"), C(-6, 0), C(2, 10), T("3")],
                         {"warn": "İki doğru paralel; yalnız aralarındaki uzaklığın yarısı olan yarıçapla ikisine "
                                  "birden teğet daire olur."}, par))
    tri = lines + [{"line": [[16, -3], [-4, 12]]}]
    out.append(tool_case("Daire: TTT, üçgenin iç teğet dairesi", "circle", [T("TTT"), C(3.5, 0), C(0, 2.5), C(4.8, 5.4)],
                         r_circle((3, 3), 3), tri))
    # Yay.
    out.append(tool_case("Yay: 3 nokta tıklanır", "arc", [C(10, 0), C(0, 10), C(-10, 0)],
                         r_arc_three((10, 0), (0, 10), (-10, 0))))
    out.append(tool_case("Yay: 3 nokta, genel", "arc", [C(1, 2), C(4, 9), C(7, 3)], r_arc_three((1, 2), (4, 9), (7, 3))))
    out.append(tool_case("Yay: başlangıç, M, merkez, bitiş", "arc", [C(10, 0), T("M"), C(0, 0), C(0, 5)],
                         r_arc((0, 0), 10, 0, PI / 2)))
    out.append(tool_case("Yay: BMS yöntemi", "arc", [T("BMS"), C(10, 0), C(0, 0), C(-5, 0)], r_arc((0, 0), 10, 0, PI)))
    out.append(tool_case("Yay: BMA yöntemi, yazılan 90", "arc", [T("BMA"), C(10, 0), C(0, 0), T("90")],
                         r_arc((0, 0), 10, 0, deg(90))))
    out.append(tool_case("Yay: BMU yöntemi, yazılan kiriş 10", "arc", [T("BMU"), C(10, 0), C(0, 0), T("10")],
                         r_arc((0, 0), 10, 0, 2 * asin(D("0.5")))))
    out.append(tool_case("Yay: BMU, çaptan uzun kiriş uyarır", "arc", [T("BMU"), C(10, 0), C(0, 0), T("25")],
                         {"warn": "Kiriş dairenin çapından uzun; en çok çap kadar bir kiriş verin."}))
    out.append(tool_case("Yay: BSA yöntemi, yazılan 90", "arc", [T("BSA"), C(10, 0), C(0, 10), T("90")],
                         r_arc_sweep((10, 0), (0, 10), deg(90))))
    out.append(tool_case("Yay: BSY yöntemi, yazılan yön 90", "arc", [T("BSY"), C(10, 0), C(-10, 0), T("90")],
                         r_arc_sweep((10, 0), (-10, 0), direction_sweep((10, 0), (-10, 0), deg(90)))))
    out.append(tool_case("Yay: BSY yöntemi, tıklanan yön", "arc", [T("BSY"), C(10, 0), C(-10, 0), C(10, 5)],
                         r_arc_sweep((10, 0), (-10, 0), direction_sweep((10, 0), (-10, 0), PI / 2))))
    out.append(tool_case("Yay: BSR yöntemi, yazılan yarıçap 12,5", "arc", [T("BSR"), C(10, 0), C(-10, 0), T("12.5")],
                         r_arc_sweep((10, 0), (-10, 0), 2 * asin(10 / D("12.5")))))
    out.append(tool_case("Yay: BSR, kirişin yarısından küçük yarıçap uyarır", "arc",
                         [T("BSR"), C(10, 0), C(-10, 0), T("5")],
                         {"warn": "Yarıçap iki uç arasındaki kirişin yarısından küçük; en az kirişin yarısı kadar bir "
                                  "yarıçap verin."}))
    out.append(tool_case("Yay: başlangıç, B, bitiş, M, tıklanan merkez", "arc",
                         [C(10, 0), T("B"), C(0, 10), T("M"), C(0, 0)], r_arc((0, 0), 10, 0, PI / 2)))
    out.append(tool_case("Yay: MBA yöntemi, yazılan eksi 90", "arc", [T("MBA"), C(0, 0), C(10, 0), T("-90")],
                         r_arc((0, 0), 10, -deg(90), 0)))
    out.append(tool_case("Yay: MBU yöntemi, yazılan eksi kiriş", "arc", [T("MBU"), C(0, 0), C(10, 0), T("-10")],
                         r_arc((0, 0), 10, 0, TAU - 2 * asin(D("0.5")))))
    out.append(tool_case("Yay: önce merkez (M), başlangıç, bitiş", "arc", [T("M"), C(0, 0), C(0, 10), C(-3, 0)],
                         r_arc((0, 0), 10, PI / 2, PI)))
    out.append(tool_case("Yay: Devam, çizginin bitişinden", "arc", [T("D"), C(30, 10)],
                         r_arc_sweep((20, 0), (30, 10), direction_sweep((20, 0), (30, 10), 0)),
                         [{"line": [[0, 0], [20, 0]]}]))
    out.append(tool_case("Yay: saat yönünde verilen yaydan Devam, verilen son noktadan", "arc",
                         [C(-10, 0), C(0, 10), C(10, 0), T("D"), C(20, 0)],
                         r_arc_sweep((10, 0), (20, 0), direction_sweep((10, 0), (20, 0), -PI / 2))))
    out.append(tool_case("Yay: Devam, bitiş teğetin doğrusunda uyarır", "arc", [T("D"), C(40, 0)],
                         {"warn": ON_TANGENT}, [{"line": [[0, 0], [20, 0]]}]))
    # Elips.
    c, m, k = ellipse((10, 0), (10, 0), 4)
    out.append(tool_case("Elips: eksen uçları ve yazılan yarı eksen", "ellipse", [C(0, 0), C(20, 0), T("4")],
                         r_ellipse(c, m, k)))
    out.append(tool_case("Elips: eksen uçları ve gösterilen yarı eksen", "ellipse", [C(0, 0), C(20, 0), C(10, 4)],
                         r_ellipse(c, m, k)))
    c, m, k = ellipse((0, 0), (0, 10), 10 * cos(deg(60)))
    out.append(tool_case("Elips: Merkez, eksenin ucu, Döndürme 60", "ellipse",
                         [T("M"), C(0, 0), C(0, 10), T("D"), T("60")], r_ellipse(c, m, k)))
    out.append(tool_case("Elips: Döndürme 95 uyarır", "ellipse", [C(0, 0), C(20, 0), T("D"), T("95")],
                         {"warn": "Döndürme açısı 0 ile 89,4° (1,5603 radyan) arasında olmalı."}))
    c, m, k = ellipse((10, 0), (10, 0), 4)
    out.append(tool_case("Elips: Yay, yazılan açılar 0 ve 90", "ellipse",
                         [T("Y"), C(0, 0), C(20, 0), T("4"), T("0"), T("90")],
                         r_ellipse(c, m, k, param(k, 0), param(k, deg(90)))))
    out.append(tool_case("Elips: Yay, gösterilen açılar", "ellipse",
                         [T("Y"), C(0, 0), C(20, 0), T("4"), C(20, 10), C(0, 10)],
                         r_ellipse(c, m, k, param(k, PI / 4), param(k, PI * 3 / 4))))
    return out


def build_tools():
    return {"format": "kentos.curve-method-tools", "version": 1,
            "note": "ADR 0289. Daire, Yay ve Elips araçlarının yöntemleri baştan sona: tıklanan ve yazılan girdiler, "
                    "beklenen nesne (merkez, eksenler göreli: doğu ve kuzey farkı 487000, 4420000'den) ya da uyarı. "
                    "Sayılar 10⁻⁹ içinde. Bağımsız başvuru: scripts/fixtures/curve_method_cases.py.",
            "within": WITHIN, "cases": tool_cases()}


def build():
    return {
        "format": "kentos.command-cases", "version": 1, "command": "cad.curves.construct", "commandVersion": 1,
        "title": "Eğri yapımı: Daire, Yay ve Elips araçlarının yöntemleri",
        "note": "ADR 0289. Yapım yapım sırayla: sayılar sonlu, nesnelerin kimlikleri, nesneler ve yerleri, sonra türün "
                "istedikleri. Çizim değişmez. within taşıyan adımda sayılar 10⁻⁹ m ya da radyan içinde karşılaştırılır "
                "(başvuru 70 basamaktan bir kez yuvarlar). Bağımsız başvuru: scripts/fixtures/curve_method_cases.py.",
        "setup": SETUP, "cases": circle_cases() + arc_cases() + ellipse_cases() + common_cases(),
    }


def main():
    files = [(OUT, build()), (TOOLS, build_tools())]
    check = "--check" in sys.argv[1:]
    for path, data in files:
        text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
        if check:
            if not path.exists() or path.read_text() != text:
                print(f"{path.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
                return 1
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)
    word = "tutarlı" if check else "yazıldı"
    print(f"eğri yapımı durumları {word}: {len(files[0][1]['cases'])} komut, {len(files[1][1]['cases'])} araç durumu")
    return 0


if __name__ == "__main__":
    sys.exit(main())
