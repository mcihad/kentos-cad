"""Kauçuk levha's sheet in IEEE double, in the core's order of operations (ops::rubber, docs/adr/0158 §2).

The command cases (transform_command_cases.py) expect the platforms' geometry bit for bit, and a thin
plate spline's bits are its solution's arithmetic:

- the frame: the sources' centre from differences to the first source, over their largest distance
  from it by V8's Math.hypot (a scaled, compensated sum; `js_hypot` in the core);
- the (n+3)² system [K P; Pᵀ 0] for east and north at once, φ(r²) = ½·r²·ln r²;
- Gaussian elimination with partial pivoting: of equal pivots the last row wins (Rust's `max_by`), a
  pivot at or below 1e-12 of the system's largest number means no single solution;
- back substitution, then d(p) and the derivative as the core sums them;
- libm's log (FreeBSD's e_log, ported below): glibc's `math.log` may differ in the last bit.

The sheet's accuracy is not this file's: rubber_cases.py solves the same equations at 50 digits with
mpmath, and `assert_near_reference` holds this one within its tolerance of that.
"""

import math
import struct
from fractions import Fraction

# ── libm's log (FreeBSD /usr/src/lib/msun/src/e_log.c, as libm 0.2.16 ports it) ──
LN2_HI = 6.93147180369123816490e-01
LN2_LO = 1.90821492927058770002e-10
LG1 = 6.666666666666735130e-01
LG2 = 3.999999999940941908e-01
LG3 = 2.857142874366239149e-01
LG4 = 2.222219843214978396e-01
LG5 = 1.818357216161805012e-01
LG6 = 1.531383769920937332e-01
LG7 = 1.479819860511658591e-01


def _bits(x):
    return struct.unpack("<Q", struct.pack("<d", x))[0]


def _double(u):
    return struct.unpack("<d", struct.pack("<Q", u))[0]


def log(x):
    ui = _bits(x)
    hx = ui >> 32
    k = 0
    if hx < 0x00100000 or hx >> 31:
        if (ui << 1) & 0xFFFFFFFFFFFFFFFF == 0:
            return -math.inf
        if hx >> 31:
            return math.nan
        k -= 54  # a subnormal: scaled up
        x *= _double(0x4350000000000000)
        ui = _bits(x)
        hx = ui >> 32
    elif hx >= 0x7FF00000:
        return x
    elif hx == 0x3FF00000 and ui & 0xFFFFFFFF == 0:
        return 0.0
    # x into [√2/2, √2]
    hx += 0x3FF00000 - 0x3FE6A09E
    k += (hx >> 20) - 0x3FF
    hx = (hx & 0x000FFFFF) + 0x3FE6A09E
    x = _double((hx << 32) | (ui & 0xFFFFFFFF))
    f = x - 1.0
    hfsq = 0.5 * f * f
    s = f / (2.0 + f)
    z = s * s
    w = z * z
    t1 = w * (LG2 + w * (LG4 + w * LG6))
    t2 = z * (LG1 + w * (LG3 + w * (LG5 + w * LG7)))
    r = t2 + t1
    dk = float(k)
    return s * (hfsq + r) + dk * LN2_LO - hfsq + f + dk * LN2_HI


# ── JavaScript's number helpers, as the core keeps them (jsmath.rs) ──


def js_hypot(a, b):
    """V8's Math.hypot: the larger magnitude scales a compensated sum of squares."""
    big, nan = 0.0, False
    for v in (a, b):
        v = abs(v)
        if v != v:
            nan = True
        elif v > big:
            big = v
    if big == math.inf:
        return math.inf
    if nan:
        return math.nan
    if big == 0.0:
        return 0.0
    total, compensation = 0.0, 0.0
    for v in (a, b):
        n = abs(v) / big
        summand = n * n - compensation
        preliminary = total + summand
        compensation = (preliminary - total) - summand
        total = preliminary
    return math.sqrt(total) * big


def js_max(a, b):
    """Math.max(a, b): NaN wins, +0 above −0."""
    if a != a or b != b:
        return math.nan
    if a == b:
        return b if math.copysign(1.0, a) < 0 else a
    return a if a > b else b


# ── The sheet ──

SINGULAR = 1e-12
MAX_LINKS = 1000


def phi(r2):
    return 0.5 * r2 * log(r2) if r2 > 0.0 else 0.0


def collinear(sources):
    """Every source on the line through the first and the next other one (exact orientation)."""
    a = sources[0]
    b = next((p for p in sources if p != a), None)
    if b is None:
        return True
    fa, fb = (Fraction(a[0]), Fraction(a[1])), (Fraction(b[0]), Fraction(b[1]))
    return all((fb[0] - fa[0]) * (Fraction(p[1]) - fa[1]) - (fb[1] - fa[1]) * (Fraction(p[0]) - fa[0]) == 0 for p in sources)


def gauss(a, b):
    m = len(a)
    big = 0.0
    for row in a:
        for v in row:
            big = js_max(big, abs(v))
    least = SINGULAR * big
    for col in range(m):
        pivot = col
        for i in range(col, m):
            if abs(a[i][col]) >= abs(a[pivot][col]):
                pivot = i
        size = abs(a[pivot][col])
        if size != size or size <= least:
            return None
        a[col], a[pivot] = a[pivot], a[col]
        b[col], b[pivot] = b[pivot], b[col]
        for r in range(col + 1, m):
            f = a[r][col] / a[col][col]
            if f != 0.0:
                for k in range(col, m):
                    a[r][k] -= f * a[col][k]
                b[r][0] -= f * b[col][0]
                b[r][1] -= f * b[col][1]
    x = [[0.0, 0.0] for _ in range(m)]
    for i in reversed(range(m)):
        s0, s1 = 0.0, 0.0
        for k in range(i + 1, m):
            s0 += a[i][k] * x[k][0]
            s1 += a[i][k] * x[k][1]
        x[i] = [(b[i][0] - s0) / a[i][i], (b[i][1] - s1) / a[i][i]]
    return x


class Sheet:
    """A solved sheet, or `error` (too_few, too_many, duplicate, collinear, singular)."""

    def __init__(self, links):
        self.error = None
        n = len(links)
        frm = [(l["from"]["x"], l["from"]["y"]) for l in links]
        if n < 3:
            self.error = "too_few"
            return
        if n > MAX_LINKS:
            self.error = "too_many"
            return
        if len(set(frm)) != n:
            self.error = "duplicate"
            return
        if collinear(frm):
            self.error = "collinear"
            return
        fx, fy = frm[0]
        sx, sy = 0.0, 0.0
        for x, y in frm:
            sx += x - fx
            sy += y - fy
        self.cx, self.cy = fx + sx / n, fy + sy / n
        size = 0.0
        for x, y in frm:
            size = js_max(size, js_hypot(x - self.cx, y - self.cy))
        self.size = size
        self.sources = [((x - self.cx) / size, (y - self.cy) / size) for x, y in frm]
        m = n + 3
        a = [[0.0] * m for _ in range(m)]
        b = [[0.0, 0.0] for _ in range(m)]
        src = self.sources
        for i in range(n):
            for j in range(n):
                dx, dy = src[i][0] - src[j][0], src[i][1] - src[j][1]
                a[i][j] = phi(dx * dx + dy * dy)
            for k, v in enumerate((1.0, src[i][0], src[i][1])):
                a[i][n + k] = v
                a[n + k][i] = v
            b[i] = [links[i]["to"]["x"] - links[i]["from"]["x"], links[i]["to"]["y"] - links[i]["from"]["y"]]
        x = gauss(a, b)
        if x is None:
            self.error = "singular"
            return
        self.weights = x[:n]
        self.affine = (x[n], x[n + 1], x[n + 2])

    def frame(self, p):
        return (p["x"] - self.cx) / self.size, (p["y"] - self.cy) / self.size

    def displacement(self, p):
        qx, qy = self.frame(p)
        a0, a1, a2 = self.affine
        d0 = a0[0] + a1[0] * qx + a2[0] * qy
        d1 = a0[1] + a1[1] * qx + a2[1] * qy
        for (sx, sy), w in zip(self.sources, self.weights):
            dx, dy = qx - sx, qy - sy
            f = phi(dx * dx + dy * dy)
            d0 += w[0] * f
            d1 += w[1] * f
        return d0, d1

    def map(self, p):
        d0, d1 = self.displacement(p)
        return {"x": p["x"] + d0, "y": p["y"] + d1}

    def jacobian(self, p):
        """[a, b, c, d]: the images of a metre east (a, b) and of a metre north (c, d)."""
        qx, qy = self.frame(p)
        _, a1, a2 = self.affine
        gx, gy = [a1[0], a1[1]], [a2[0], a2[1]]
        for (sx, sy), w in zip(self.sources, self.weights):
            dx, dy = qx - sx, qy - sy
            r2 = dx * dx + dy * dy
            if r2 > 0.0:
                k = log(r2) + 1.0
                gx[0] += w[0] * dx * k
                gx[1] += w[1] * dx * k
                gy[0] += w[0] * dy * k
                gy[1] += w[1] * dy * k
        s = self.size
        return [1.0 + gx[0] / s, gx[1] / s, gy[0] / s, 1.0 + gy[1] / s]


def assert_near_reference(links, probes, metres=1e-9):
    """This sheet against rubber_cases.py's 50-digit solution of the same links, at `probes`."""
    import rubber_cases

    exact = rubber_cases.solve(links)
    sheet = Sheet(links)
    if "error" in exact or sheet.error:
        assert sheet.error == exact.get("error"), (sheet.error, exact.get("error"))
        return
    for p in probes:
        want, _ = rubber_cases.evaluate(exact, p)
        got = sheet.map(p)
        off = math.hypot(got["x"] - want["x"], got["y"] - want["y"])
        assert off <= metres, f"{p}: {got} is {off} m from the reference {want}"


# ── How far a shape on the sheet lies from its true image (ops::warp::bend_of, docs/adr/0158 §3) ──
# Only for the `rubber_bends` warning: how many shapes bend over 0.1 mm and the largest bend in mm with
# one decimal. Its trigonometry (an arc's points, the nearest similarity) is Python's; the cases assert
# that no count or shown decimal is within reach of a last-bit difference (`margins`).

CHORD = 1e-4
QUARTERS = (0.25, 0.5, 0.75)
TAU = 2 * math.pi


def bulge_arc(a, b, bulge):
    """The arc of a bulged edge (geom::bulge::bulge_arc): centre, radius, start angle, sweep; None when straight."""
    if not abs(bulge) > 1e-12:
        return None
    dx, dy = b["x"] - a["x"], b["y"] - a["y"]
    chord = js_hypot(dx, dy)
    if chord < 1e-12:
        return None
    k = (1.0 - bulge * bulge) / (4.0 * bulge)
    c = ((a["x"] + b["x"]) / 2.0 - dy * k, (a["y"] + b["y"]) / 2.0 + dx * k)
    r = (chord * (1.0 + bulge * bulge)) / (4.0 * abs(bulge))
    return c, r, math.atan2(a["y"] - c[1], a["x"] - c[0]), 4.0 * math.atan(bulge)


def edge_bend(a, b, bulge, sheet):
    na, nb = sheet.map(a), sheet.map(b)
    arc, kept = bulge_arc(a, b, bulge), bulge_arc(na, nb, bulge)
    worst = 0.0
    for t in QUARTERS:
        if arc and kept:
            u, v = arc[2] + arc[3] * t, kept[2] + kept[3] * t
            real = sheet.map({"x": arc[0][0] + arc[1] * math.cos(u), "y": arc[0][1] + arc[1] * math.sin(u)})
            here = (kept[0][0] + kept[1] * math.cos(v), kept[0][1] + kept[1] * math.sin(v))
        else:
            real = sheet.map({"x": a["x"] + (b["x"] - a["x"]) * t, "y": a["y"] + (b["y"] - a["y"]) * t})
            here = (na["x"] + (nb["x"] - na["x"]) * t, na["y"] + (nb["y"] - na["y"]) * t)
        worst = js_max(worst, js_hypot(real["x"] - here[0], real["y"] - here[1]))
    return worst


def nearest_similarity(j):
    """√|det J| and the turn of J's first column (ops::warp::nearest_similarity), as an affine [a, b, c, d, e, f]."""
    det = j[0] * j[3] - j[1] * j[2]
    s = math.sqrt(abs(det))
    theta = math.atan2(j[1], j[0])
    c, sn = s * math.cos(theta), s * math.sin(theta)
    return [c, sn, sn, -c, 0.0, 0.0] if det < 0.0 else [c, sn, -sn, c, 0.0, 0.0]


def curve_bend(c, m, n, span, sheet):
    sa, sb, sc, sd = nearest_similarity(sheet.jacobian(c))[:4]
    to = sheet.map(c)
    worst = 0.0
    for k in range(8):
        t = span[0] + (span[1] - span[0]) * (k + 0.5) / 8.0
        v = (m[0] * math.cos(t) + n[0] * math.sin(t), m[1] * math.cos(t) + n[1] * math.sin(t))
        real = sheet.map({"x": c["x"] + v[0], "y": c["y"] + v[1]})
        here = (to["x"] + sa * v[0] + sc * v[1], to["y"] + sb * v[0] + sd * v[1])
        worst = js_max(worst, js_hypot(real["x"] - here[0], real["y"] - here[1]))
    return worst


def bend(e, sheet):
    """A drawing object's bend (metres): its edges at the quarters, a circle's or an arc's eight middles; 0 for the rest."""

    def ring(pts, bulges, closed):
        n = len(pts)
        worst = 0.0
        for i in range(n if closed else n - 1):
            worst = js_max(worst, edge_bend(pts[i], pts[(i + 1) % n], (bulges or [])[i] if i < len(bulges or []) else 0.0, sheet))
        return worst

    k = e["kind"]
    if k == "line":
        return edge_bend(e["a"], e["b"], 0.0, sheet)
    if k == "polyline":
        return ring(e["pts"], e.get("bulges"), False)
    if k == "polygon":
        worst = ring(e["pts"], e.get("bulges"), True)
        for h in e.get("holes") or []:
            worst = js_max(worst, ring(h["pts"], h.get("bulges"), True))
        return worst
    if k == "circle":
        return curve_bend(e["c"], (e["r"], 0.0), (0.0, e["r"]), (0.0, TAU), sheet)
    if k == "arc":
        sweep = math.fmod(e["a1"] - e["a0"], TAU)
        sweep = sweep + TAU if sweep < 0.0 else sweep
        sweep = TAU if sweep < 1e-12 else sweep
        return curve_bend(e["c"], (e["r"], 0.0), (0.0, e["r"]), (e["a0"], e["a0"] + sweep), sheet)
    return 0.0


def margins(bends, shown_decimals=1):
    """No bend within 1e-9 m of the 0.1 mm line, the largest in mm not within 1e-9 of a shown decimal's rounding edge."""
    for b in bends:
        assert abs(b - CHORD) > 1e-9, f"a bend of {b} m is at the 0.1 mm line"
    mm = max(bends) * 1000 * 10**shown_decimals
    assert abs(mm - math.floor(mm) - 0.5) > 1e-6, f"the largest bend {max(bends)} m is at a rounding edge"
