"""Writes fixtures/style/v1/generator.json: Geometri üretici's steps (docs/adr/0226 §1) on small geometries, computed
from the ADR's definitions without KentOS code. The style core runs every case through `generate::run`
(crates/shared/style-core/tests/all/generator.rs); the web draws the same steps through the same core in WASM
(fixtures/style/v1/batches.json's generator cases).

The definitions, as the ADR writes them: centroids by Green's theorem (holes minus), of paths by their edges'
lengths, of points their mean, exact in fractions; the interior point the centroid of the outer ring when it lies
inside, else the middle of the widest inside stretch of the scan lines at 0.5, 0.35, 0.65, 0.2, 0.8, 0.1 and 0.9 of
the height; a path's middle halfway along its longest path; parallels mitred, bevelled where the mitre reaches
beyond four times the distance; arcs cut into n = clamp(⌈θ / (2·acos(1 − τ/r))⌉, 1, ⌈720·θ/2π⌉) chords, τ = 0,05
paper mm, a full circle into at least 8; a slice of `width` degrees about `direction` (clockwise from north); a
regular polygon's first corner `rotation` degrees counter-clockwise from north; turns and scales about the centroid,
a mirroring scale turning its rings back. A buffer is the core's exact buffer with its arcs chorded: its vertices are
checked to lie at the distance from the input (or on it, for a one-sided buffer's flat side), its chords within τ
of their arcs, its area within its chords' loss of the exact one.

    python3 scripts/fixtures/geometry_generator_cases.py           # writes the file
    python3 scripts/fixtures/geometry_generator_cases.py --check   # compares it with the one on disk
"""
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/style/v1/generator.json"

CHORD_MM = 0.05
MM_PER_PX = 25.4 / 96.0
MITER_LIMIT = 4.0
E0, N0 = 487_012.346, 4_420_187.521


def to_world(v, unit, plot):
    if unit == "m":
        return v
    mm = v * MM_PER_PX if unit == "px" else v
    return mm * plot / 1000.0


def tol_of(plot):
    return CHORD_MM * plot / 1000.0


# ── Definitions ─────────────────────────────────────────────────────────


def centroid(g):
    kind, data = g
    if kind == "points":
        if not data:
            return None
        n = len(data)
        return (float(sum(F(p[0]) for p in data) / n), float(sum(F(p[1]) for p in data) / n))
    if kind == "lines":
        sx = sy = tot = 0.0
        pts_all = [p for path in data for p in path["points"]]
        if not pts_all:
            return None
        for path in data:
            p = path["points"]
            n = len(p)
            count = n if path["closed"] else max(n - 1, 0)
            for i in range(count):
                a, b = p[i], p[(i + 1) % n]
                length = math.hypot(b[0] - a[0], b[1] - a[1])
                sx += length * (a[0] + b[0]) / 2
                sy += length * (a[1] + b[1]) / 2
                tot += length
        return (sx / tot, sy / tot) if tot > 0 else tuple(pts_all[0])
    # Areas: Green's theorem over every ring, in fractions (holes run the other way: minus).
    a2 = cx = cy = F(0)
    for rings in data:
        for r in rings:
            n = len(r)
            for i in range(n):
                p, q = r[i], r[(i + 1) % n]
                px, py, qx, qy = F(p[0]), F(p[1]), F(q[0]), F(q[1])
                c = px * qy - qx * py
                a2 += c
                cx += (px + qx) * c
                cy += (py + qy) * c
    return (float(cx / (3 * a2)), float(cy / (3 * a2)))


def inside(rings, p):
    c = False
    for r in rings:
        n = len(r)
        j = n - 1
        for i in range(n):
            a, b = r[i], r[j]
            if (a[1] > p[1]) != (b[1] > p[1]) and p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]:
                c = not c
            j = i
    return c


def interior_point(rings):
    outer = rings[0]
    c = centroid(("areas", [[outer]]))
    if inside(rings, c):
        return c
    ys = [p[1] for p in outer]
    lo, hi = min(ys), max(ys)
    best = None
    for f in [0.5, 0.35, 0.65, 0.2, 0.8, 0.1, 0.9]:
        y = lo + (hi - lo) * f
        xs = []
        for r in rings:
            n = len(r)
            j = n - 1
            for i in range(n):
                a, b = r[i], r[j]
                if (a[1] > y) != (b[1] > y):
                    xs.append(a[0] + (y - a[1]) * (b[0] - a[0]) / (b[1] - a[1]))
                j = i
        xs.sort()
        for k in range(0, len(xs) - 1, 2):
            w = xs[k + 1] - xs[k]
            if best is None or w > best[2]:
                best = ((xs[k] + xs[k + 1]) / 2, y, w)
    return (best[0], best[1]) if best else c


def line_middle(paths):
    best = None
    for path in paths:
        p = path["points"]
        n = len(p)
        count = n if path["closed"] else n - 1
        length = sum(math.hypot(p[(i + 1) % n][0] - p[i][0], p[(i + 1) % n][1] - p[i][1]) for i in range(count))
        if best is None or length > best[2]:
            best = (p, path["closed"], length)
    p, closed, total = best
    half = total / 2
    n = len(p)
    count = n if closed else n - 1
    run = 0.0
    for i in range(count):
        a, b = p[i], p[(i + 1) % n]
        length = math.hypot(b[0] - a[0], b[1] - a[1])
        if run + length >= half and length > 0:
            t = (half - run) / length
            return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)
        run += length
    return tuple(p[0])


def open_points(p, closed):
    return p[:-1] if closed and len(p) > 1 and p[0] == p[-1] else p


def vertices(g, which):
    kind, data = g
    paths = []
    if kind == "lines":
        paths = [(open_points(path["points"], path["closed"]), path["closed"]) for path in data]
    elif kind == "areas":
        paths = [(open_points(r, True), True) for rings in data for r in rings]
    out = []
    for p, closed in paths:
        n = len(p)
        if n == 0:
            continue
        if which == "inner":
            if closed:
                out += p
            elif n > 2:
                out += p[1 : n - 1]
        elif which == "first":
            out.append(p[0])
        elif which == "last":
            out.append(p[0] if closed else p[-1])
        elif which == "ends":
            out += [p[0]] if closed or n == 1 else [p[0], p[-1]]
        else:
            out += p
    return ("points", out)


def line_line(a1, a2, b1, b2):
    d1 = (a2[0] - a1[0], a2[1] - a1[1])
    d2 = (b2[0] - b1[0], b2[1] - b1[1])
    den = d1[0] * d2[1] - d1[1] * d2[0]
    if abs(den) < 1e-12 * math.hypot(*d1) * math.hypot(*d2):
        return None
    t = ((b1[0] - a1[0]) * d2[1] - (b1[1] - a1[1]) * d2[0]) / den
    return (a1[0] + d1[0] * t, a1[1] + d1[1] * t)


def offset_path(pts, d, closed):
    n = len(pts)
    count = n if closed else n - 1
    segs = []
    corners = []
    for i in range(count):
        a, b = pts[i], pts[(i + 1) % n]
        length = math.hypot(b[0] - a[0], b[1] - a[1])
        if length < 1e-12:
            continue
        nx, ny = -(b[1] - a[1]) / length * d, (b[0] - a[0]) / length * d
        segs.append(((a[0] + nx, a[1] + ny), (b[0] + nx, b[1] + ny)))
        corners.append(a)
    out = []

    def join(s1, s2, corner):
        h = line_line(s1[0], s1[1], s2[0], s2[1])
        if h is None:
            out.append(s2[0])
        elif math.hypot(h[0] - corner[0], h[1] - corner[1]) > MITER_LIMIT * abs(d):
            out.append(s1[1])
            out.append(s2[0])
        else:
            out.append(h)

    m = len(segs)
    if closed:
        for i in range(m):
            join(segs[(i - 1) % m], segs[i], pts[i])
    else:
        out.append(segs[0][0])
        for i in range(1, m):
            join(segs[i - 1], segs[i], pts[i])
        out.append(segs[m - 1][1])
    return out


def extend(p, a, b):
    p = [list(q) for q in p]
    n = len(p)
    if a > 0:
        k = next(k for k in range(n - 1) if p[k] != p[k + 1])
        q, r = p[k], p[k + 1]
        length = math.hypot(r[0] - q[0], r[1] - q[1])
        p[0] = [p[0][0] - (r[0] - q[0]) / length * a, p[0][1] - (r[1] - q[1]) / length * a]
    if b > 0:
        k = next(k for k in range(n - 1, 0, -1) if p[k] != p[k - 1])
        q, r = p[k - 1], p[k]
        length = math.hypot(r[0] - q[0], r[1] - q[1])
        p[-1] = [p[-1][0] + (r[0] - q[0]) / length * b, p[-1][1] + (r[1] - q[1]) / length * b]
    return p


def chords(sweep, r, tol):
    sweep = abs(sweep)
    if not r > tol or sweep == 0:
        return 1
    most = max(math.ceil(720 * sweep / (2 * math.pi)), 1)
    n = math.ceil(sweep / (2 * math.acos(1 - tol / r)))
    return min(max(n, 1), most)


def circle(c, r, tol):
    n = max(chords(2 * math.pi, r, tol), 8)
    return [[c[0] + r * math.cos(2 * math.pi * k / n), c[1] + r * math.sin(2 * math.pi * k / n)] for k in range(n)]


def wedge(c, direction, width, r, ri, tol):
    if width >= 360:
        rings = [circle(c, r, tol)]
        if ri > 0:
            rings.append(list(reversed(circle(c, ri, tol))))
        return rings
    sweep = math.radians(width)
    a0 = math.radians(90 - direction) - sweep / 2
    n = chords(sweep, r, tol)
    ring = [[c[0] + r * math.cos(a0 + sweep * k / n), c[1] + r * math.sin(a0 + sweep * k / n)] for k in range(n + 1)]
    if ri > 0:
        m = chords(sweep, ri, tol)
        ring += [[c[0] + ri * math.cos(a0 + sweep - sweep * k / m), c[1] + ri * math.sin(a0 + sweep - sweep * k / m)] for k in range(m + 1)]
    else:
        ring.append(list(c))
    return [ring]


def hull(pts):
    pts = sorted(set(tuple(p) for p in pts))
    if len(pts) < 3:
        return pts

    def cross(o, a, b):
        return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])

    lower, upper = [], []
    for p in pts:
        while len(lower) >= 2 and cross(lower[-2], lower[-1], p) <= 0:
            lower.pop()
        lower.append(p)
    for p in reversed(pts):
        while len(upper) >= 2 and cross(upper[-2], upper[-1], p) <= 0:
            upper.pop()
        upper.append(p)
    return [list(p) for p in lower[:-1] + upper[:-1]]


def coords(g):
    kind, data = g
    if kind == "points":
        return list(data)
    if kind == "lines":
        return [p for path in data for p in path["points"]]
    return [p for rings in data for r in rings for p in r]


def mapped(g, f):
    kind, data = g
    if kind == "points":
        return ("points", [f(p) for p in data])
    if kind == "lines":
        return ("lines", [{"points": [f(p) for p in path["points"]], "closed": path["closed"]} for path in data])
    return ("areas", [[[f(p) for p in r] for r in rings] for rings in data])


def centres(g):
    return [list(p) for p in g[1]] if g[0] == "points" else [list(centroid(g))]


def num(v, attrs):
    """A step's number: fixed, or an expression's value (none here: the fallback)."""
    if v is None:
        return None
    if isinstance(v, dict):
        return v.get("fallback")
    return v


def step(s, g, plot, unit):
    tol = tol_of(plot)
    length = lambda key: (lambda v: None if v is None else to_world(v, unit, plot))(num(s.get(key), None))
    value = lambda key: num(s.get(key), None)
    kind, data = g
    k = s["kind"]
    if k == "centroid":
        c = centroid(g)
        return ("points", [list(c)] if c else [])
    if k == "pointOnSurface":
        if kind == "points":
            return g
        if kind == "lines":
            return ("points", [list(line_middle(data))])
        largest = max(data, key=lambda rings: sum(ring_area(r) for r in rings))
        return ("points", [list(interior_point(largest))])
    if k == "vertices":
        return vertices(g, s.get("which", "all"))
    if k == "boundary":
        return ("lines", [{"points": r, "closed": True} for rings in data for r in rings]) if kind == "areas" else ("lines", [])
    if k == "offset":
        d = length("distance")
        if d is None:
            return ("lines", [])
        paths = data if kind == "lines" else [{"points": r, "closed": True} for rings in data for r in rings] if kind == "areas" else []
        out = []
        for path in paths:
            p = path["points"] if d == 0 else offset_path(path["points"], d, path["closed"])
            if len(p) >= 2:
                out.append({"points": [list(q) for q in p], "closed": path["closed"]})
        return ("lines", out)
    if k == "extend":
        if kind != "lines":
            return ("lines", [])
        a = max(length("start") or 0.0, 0.0)
        b = max(length("end") or 0.0, 0.0)
        return ("lines", [path if path["closed"] else {"points": extend(path["points"], a, b), "closed": False} for path in data])
    if k == "envelope":
        c = coords(g)
        x0, y0 = min(p[0] for p in c), min(p[1] for p in c)
        x1, y1 = max(p[0] for p in c), max(p[1] for p in c)
        if not (x1 > x0 and y1 > y0):
            return ("areas", [])
        return ("areas", [[[[x0, y0], [x1, y0], [x1, y1], [x0, y1]]]])
    if k == "hull":
        h = hull(coords(g))
        return ("areas", [[h]]) if len(h) >= 3 else ("areas", [])
    if k == "circle":
        r = length("radius")
        if r is None or r <= 0:
            return ("areas", [])
        return ("areas", [[circle(c, r, tol)] for c in centres(g)])
    if k == "wedge":
        d, w, r = value("direction"), value("width"), length("radius")
        if d is None or w is None or r is None:
            return ("areas", [])
        ri = max(length("inner") or 0.0, 0.0)
        if w <= 0 or r <= 0 or ri >= r:
            return ("areas", [])
        return ("areas", [wedge(c, d, w, r, ri, tol) for c in centres(g)])
    if k == "polygon":
        n, r = value("sides"), length("radius")
        if n is None or r is None:
            return ("areas", [])
        n = math.floor(n + 0.5)
        if n < 3 or r <= 0:
            return ("areas", [])
        turn = value("rotation") or 0.0
        n = min(n, 720)
        return ("areas", [[[[c[0] + r * math.cos(math.radians(90 + turn) + 2 * math.pi * j / n), c[1] + r * math.sin(math.radians(90 + turn) + 2 * math.pi * j / n)] for j in range(n)]] for c in centres(g)])
    if k == "translate":
        x, y = length("dx") or 0.0, length("dy") or 0.0
        return mapped(g, lambda p: [p[0] + x, p[1] + y])
    if k == "rotate":
        a = value("angle")
        c = centroid(g)
        if a is None or c is None:
            return g
        ca, sa = math.cos(math.radians(a)), math.sin(math.radians(a))
        return mapped(g, lambda p: [c[0] + (p[0] - c[0]) * ca - (p[1] - c[1]) * sa, c[1] + (p[0] - c[0]) * sa + (p[1] - c[1]) * ca])
    if k == "scale":
        vx, vy = value("x"), value("y")
        fx = vx if vx is not None else vy if vy is not None else 1.0
        fy = vy if vy is not None else vx if vx is not None else 1.0
        c = centroid(g)
        if c is None:
            return g
        out = mapped(g, lambda p: [c[0] + (p[0] - c[0]) * fx, c[1] + (p[1] - c[1]) * fy])
        if out[0] == "areas" and fx * fy < 0:
            out = ("areas", [[list(reversed(r)) for r in rings] for rings in out[1]])
        return out
    raise ValueError(k)


def ring_area(r):
    n = len(r)
    return sum(r[i][0] * r[(i + 1) % n][1] - r[(i + 1) % n][0] * r[i][1] for i in range(n)) / 2


def is_empty(g):
    return len(g[1]) == 0


def run(steps, g, plot, unit):
    for s in steps:
        g = step(s, g, plot, unit)
        if is_empty(g):
            break
    return g


# ── Buffers: what the result must satisfy ──────────────────────────────


def seg_distance(p, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    l2 = dx * dx + dy * dy
    t = 0.0 if l2 == 0 else max(0.0, min(1.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2))
    return math.hypot(p[0] - a[0] - t * dx, p[1] - a[1] - t * dy)


def buffer_check(g, d, side, plot):
    """What a buffer's result must satisfy: the distance of its vertices from the input, its area's bounds."""
    tol = tol_of(plot)
    kind, data = g
    if kind == "points":
        n_min = max(chords(2 * math.pi, d, tol), 1)
        exact = math.pi * d * d * len(data)
        # Chords lose at most a circle's segments of sagitta τ: the inscribed polygon of the fewest chords.
        return {"distance": d, "onInput": False, "area": [exact - 2 * math.pi * d * tol * len(data) - 1e-9, exact + 1e-9]}
    if kind == "lines":
        segs = [(p["points"][i], p["points"][i + 1]) for p in data for i in range(len(p["points"]) - 1)]
        total = sum(math.hypot(b[0] - a[0], b[1] - a[1]) for a, b in segs)
        if side == "both":
            exact = 2 * d * total + math.pi * d * d
            loss = 2 * math.pi * d * tol + 1e-9
            return {"distance": d, "onInput": False, "area": [exact - loss, exact + 1e-9]}
        exact = d * total
        return {"distance": d, "onInput": True, "area": [exact - 1e-9 * max(1, exact), exact + 1e-9 * max(1, exact)]}
    raise ValueError("areas are given exactly")


# ── Cases ──────────────────────────────────────────────────────────────


def tm(points):
    return [[E0 + x, N0 + y] for x, y in points]


def case(name, geometry, steps, plot=1000, unit="m"):
    kind = next(iter(geometry))
    g = (kind, geometry[kind])
    out = run(steps, g, plot, unit)
    return {"name": name, "plotScale": plot, "unit": unit, "geometry": geometry, "steps": steps, "expect": {out[0]: out[1]}}


def buffer_case(name, geometry, distance, side="both", plot=1000):
    kind = next(iter(geometry))
    g = (kind, geometry[kind])
    return {
        "name": name,
        "plotScale": plot,
        "unit": "m",
        "geometry": geometry,
        "steps": [{"kind": "buffer", "distance": distance, "side": side}],
        "check": buffer_check(g, distance, side, plot),
    }


SQUARE = [[0, 0], [4, 0], [4, 2], [0, 2]]
L_OUTER = [[0, 0], [10, 0], [10, 4], [4, 4], [4, 10], [0, 10]]
HOLE = [[1, 1], [1, 3], [3, 3], [3, 1]]
U_SHAPE = [[0, 0], [9, 0], [9, 9], [6, 9], [6, 3], [3, 3], [3, 9], [0, 9]]
PATH = [[0, 0], [6, 0], [6, 8], [10, 8]]


def cases():
    out = [
        case("ağırlık merkezi: dikdörtgen", {"areas": [[SQUARE]]}, [{"kind": "centroid"}]),
        case("ağırlık merkezi: delikli L, TM koordinatlarında", {"areas": [[tm(L_OUTER), tm(HOLE)]]}, [{"kind": "centroid"}]),
        case("ağırlık merkezi: çizgi, kenar uzunluklarıyla", {"lines": [{"points": PATH, "closed": False}]}, [{"kind": "centroid"}]),
        case("ağırlık merkezi: noktaların ortalaması", {"points": [[0, 0], [3, 0], [3, 6]]}, [{"kind": "centroid"}]),
        case("iç nokta: dışbükey alanda ağırlık merkezi", {"areas": [[SQUARE]]}, [{"kind": "pointOnSurface"}]),
        case("iç nokta: U biçiminde tarama çizgisiyle", {"areas": [[U_SHAPE]]}, [{"kind": "pointOnSurface"}]),
        case("iç nokta: çizginin en uzun yolunun ortası", {"lines": [{"points": [[0, 0], [1, 0]], "closed": False}, {"points": PATH, "closed": False}]}, [{"kind": "pointOnSurface"}]),
        case("köşeler: hepsi, kapalı halkada son tekrar bir kez", {"lines": [{"points": [[0, 0], [5, 0], [5, 5], [0, 0]], "closed": True}]}, [{"kind": "vertices", "which": "all"}]),
        case("köşeler: uçlar dışındakiler", {"lines": [{"points": PATH, "closed": False}]}, [{"kind": "vertices", "which": "inner"}]),
        case("köşeler: ilk", {"lines": [{"points": PATH, "closed": False}]}, [{"kind": "vertices", "which": "first"}]),
        case("köşeler: son", {"lines": [{"points": PATH, "closed": False}]}, [{"kind": "vertices", "which": "last"}]),
        case("köşeler: iki uç", {"lines": [{"points": PATH, "closed": False}]}, [{"kind": "vertices", "which": "ends"}]),
        case("sınır: delikli alan", {"areas": [[L_OUTER, HOLE]]}, [{"kind": "boundary"}]),
        case("paralel: sola 2 m, gönyeli köşeler", {"lines": [{"points": PATH, "closed": False}]}, [{"kind": "offset", "distance": 2}]),
        case("paralel: alanda içeri 0,5 m", {"areas": [[SQUARE]]}, [{"kind": "offset", "distance": 0.5}]),
        case("paralel: sivri köşe kırpılır", {"lines": [{"points": [[0, 0], [10, 0], [0, 1]], "closed": False}]}, [{"kind": "offset", "distance": 1}]),
        case("uzat: baştan 3, sondan 5 m; tekrarlı ilk köşe", {"lines": [{"points": [[0, 0], [0, 0], [4, 3], [4, 7]], "closed": False}]}, [{"kind": "extend", "start": 3, "end": 5}]),
        case("sınırlayıcı kutu", {"points": [[2, 1], [7, 3], [4, 9]]}, [{"kind": "envelope"}]),
        case("dışbükey örtü: doğrudaş noktalarla", {"points": [[0, 0], [2, 0], [4, 0], [4, 4], [2, 2], [0, 4], [1, 3]]}, [{"kind": "hull"}]),
        case("daire: 10 m, 1:1000", {"points": [[100, 200]]}, [{"kind": "circle", "radius": 10}]),
        case("daire: çok küçük, en az 8 kiriş", {"points": [[0, 0]]}, [{"kind": "circle", "radius": 0.01}]),
        case("daire: 5 mm 1:2000'de 10 m", {"points": [[0, 0]]}, [{"kind": "circle", "radius": 5}], plot=2000, unit="mm"),
        case("daire dilimi: kuzeydoğuya 90°, iç yarıçaplı", {"points": tm([[0, 0]])}, [{"kind": "wedge", "direction": 45, "width": 90, "radius": 20, "inner": 5}]),
        case("daire dilimi: 360° halka", {"points": [[0, 0]]}, [{"kind": "wedge", "direction": 0, "width": 360, "radius": 8, "inner": 3}]),
        case("daire dilimi: yön ifadeden, değeri yoksa yedeği", {"points": [[0, 0]]}, [{"kind": "wedge", "direction": {"expr": "[Yön]", "fallback": 120}, "width": 60, "radius": 15}]),
        case("düzgün altıgen: 15° döndürülmüş", {"points": [[10, 10]]}, [{"kind": "polygon", "sides": 6, "radius": 10, "rotation": 15}]),
        case("öteleme: 96 px 1:1000'de 25,4 m", {"areas": [[SQUARE]]}, [{"kind": "translate", "dx": 96, "dy": -48}], unit="px"),
        case("döndür: ağırlık merkezinde 90°", {"areas": [[SQUARE]]}, [{"kind": "rotate", "angle": 90}]),
        case("ölçekle: y boşsa x", {"areas": [[SQUARE]]}, [{"kind": "scale", "x": 2}]),
        case("ölçekle: aynalayan, halka geri döner", {"areas": [[L_OUTER, HOLE]]}, [{"kind": "scale", "x": -1, "y": 1}]),
        case("zincir: ağırlık merkezi, dilim, döndür", {"areas": [[tm(L_OUTER)]]}, [{"kind": "centroid"}, {"kind": "wedge", "direction": 0, "width": 30, "radius": 25}, {"kind": "rotate", "angle": -10}]),
        case("noktada sınır boş", {"points": [[0, 0]]}, [{"kind": "boundary"}]),
        case("iki kenarlı çokgen boş", {"points": [[0, 0]]}, [{"kind": "polygon", "sides": 2, "radius": 5}]),
        case("genişliği 0 olan dilim boş", {"points": [[0, 0]]}, [{"kind": "wedge", "direction": 10, "width": 0, "radius": 5}]),
        case("iç yarıçapı dışınkine eşit dilim boş", {"points": [[0, 0]]}, [{"kind": "wedge", "direction": 10, "width": 30, "radius": 5, "inner": 5}]),
    ]
    out += [
        buffer_case("tampon: nokta, 10 m", {"points": [[50, 50]]}, 10),
        buffer_case("tampon: çizgi, iki yan", {"lines": [{"points": [[0, 0], [30, 0]], "closed": False}]}, 4),
        buffer_case("tampon: çizgi, sol yan", {"lines": [{"points": [[0, 0], [30, 0]], "closed": False}]}, 4, side="left"),
    ]
    # An area buffered inwards keeps its corners sharp (docs/adr/0201 §2): a rectangle 1 m in, whichever corner the
    # ring starts at (`unordered`: the same ring turned round its corners).
    out.append(
        {
            "name": "tampon: alanda içe 1 m, köşeler keskin",
            "plotScale": 1000,
            "unit": "m",
            "geometry": {"areas": [[[[0, 0], [10, 0], [10, 6], [0, 6]]]]},
            "steps": [{"kind": "buffer", "distance": -1, "side": "both"}],
            "expect": {"areas": [[[[1, 1], [9, 1], [9, 5], [1, 5]]]]},
            "unordered": True,
        }
    )
    return out


def build():
    return {
        "format": "kentos.generator-cases",
        "version": 1,
        "note": "Written by scripts/fixtures/geometry_generator_cases.py from docs/adr/0226 §1; do not edit by hand.",
        "tolerance": 1e-9,
        "cases": cases(),
    }


def main() -> int:
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} güncel")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı ({len(build()['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
