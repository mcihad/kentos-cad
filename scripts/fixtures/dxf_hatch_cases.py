"""DXF HATCH round trip with independent files (docs/adr/0333 §4).

    python3 scripts/fixtures/dxf_hatch_cases.py           # writes the input files
    python3 scripts/fixtures/dxf_hatch_cases.py --check   # writes nothing; compares the inputs and checks KentOS's output

The input files (fixtures/formats/v1/dxf-hatch/in/*.dxf) are written here from the DXF reference (AutoCAD 2007, AC1021),
without KentOS code: associative hatches with their source objects' handles (97, 330s), the sources' reactors and the
seed points (98); polyline and edge paths (lines, arcs, elliptic arcs, a spline); gradients (LINEAR, CYLINDER,
INVSPHERICAL with true colours); a custom pattern's line families with dashes and a user-defined double hatch.

KentOS reads each file into a new drawing with `file.import.dxf` and writes it with `file.export.dxf`
(`cargo test -p kentos-headless --test headless dxf_hatches_go_round`, KENTOS_WRITE_DXF=1 writes
fixtures/formats/v1/dxf-hatch/out/*.dxf). `--check` reads both files here and checks, hatch by hatch in the files' order:

- the paths draw the same outlines: every path's points (lines, arcs, elliptic arcs, splines evaluated here) within
  1e-6 m of the other file's outline, both ways (KentOS may write lines and arcs as a polyline path with bulges);
- an associative hatch stays associative (71 1): its outer path's sources are objects of the output whose geometry is
  the input's sources', each with the hatch among its reactors; its islands' paths name the islands; its seed point is
  inside its outer path;
- a pattern's line families are the same lines: angle (mod 180°), spacing across the lines, the lines' places across
  (modulo the spacing) and the dashes, within 1e-9 of the size;
- a gradient keeps its name, angle and two true colours.
"""
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/formats/v1/dxf-hatch"
TOL = 1e-6


# ── Writing the inputs ───────────────────────────────────────────────────────────────────────────────────────────

def g(code: int, value) -> str:
    if isinstance(value, float):
        value = repr(value)
    return f"{code:>3}\n{value}\n"


def head(kind: str, handle: str, layer: str, reactors=()) -> str:
    s = g(0, kind) + g(5, handle)
    if reactors:
        s += g(102, "{ACAD_REACTORS")
        for r in reactors:
            s += g(330, r)
        s += g(102, "}")
    return s + g(330, "1F") + g(100, "AcDbEntity") + g(8, layer)


def lwpolyline(handle, pts, layer="SINIR", reactors=()):
    s = head("LWPOLYLINE", handle, layer, reactors) + g(100, "AcDbPolyline") + g(90, len(pts)) + g(70, 1)
    for x, y in pts:
        s += g(10, float(x)) + g(20, float(y))
    return s


def circle(handle, c, r, layer="SINIR", reactors=()):
    return (head("CIRCLE", handle, layer, reactors) + g(100, "AcDbCircle")
            + g(10, float(c[0])) + g(20, float(c[1])) + g(30, 0.0) + g(40, float(r)))


def line(handle, a, b, layer="SINIR", reactors=()):
    return (head("LINE", handle, layer, reactors) + g(100, "AcDbLine")
            + g(10, float(a[0])) + g(20, float(a[1])) + g(30, 0.0) + g(11, float(b[0])) + g(21, float(b[1])) + g(31, 0.0))


def poly_path(pts, external, sources=()):
    s = g(92, 2 | (1 if external else 0)) + g(72, 0) + g(73, 1) + g(93, len(pts))
    for x, y in pts:
        s += g(10, float(x)) + g(20, float(y))
    s += g(97, len(sources))
    for h in sources:
        s += g(330, h)
    return s


def edge_path(edges, external, sources=()):
    """`edges`: ("line", a, b), ("arc", c, r, a0, a1), ("ellipse", c, major, ratio, a0, a1),
    ("spline", degree, knots, ctrl)."""
    s = g(92, 1 if external else 0) + g(93, len(edges))
    for e in edges:
        if e[0] == "line":
            s += g(72, 1) + g(10, float(e[1][0])) + g(20, float(e[1][1])) + g(11, float(e[2][0])) + g(21, float(e[2][1]))
        elif e[0] == "arc":
            s += (g(72, 2) + g(10, float(e[1][0])) + g(20, float(e[1][1])) + g(40, float(e[2]))
                  + g(50, float(e[3])) + g(51, float(e[4])) + g(73, 1))
        elif e[0] == "ellipse":
            s += (g(72, 3) + g(10, float(e[1][0])) + g(20, float(e[1][1])) + g(11, float(e[2][0])) + g(21, float(e[2][1]))
                  + g(40, float(e[3])) + g(50, float(e[4])) + g(51, float(e[5])) + g(73, 1))
        else:
            _, degree, knots, ctrl = e
            s += g(72, 4) + g(94, degree) + g(73, 0) + g(74, 0) + g(95, len(knots)) + g(96, len(ctrl))
            for k in knots:
                s += g(40, float(k))
            for x, y in ctrl:
                s += g(10, float(x)) + g(20, float(y))
    s += g(97, len(sources))
    for h in sources:
        s += g(330, h)
    return s


def hatch(handle, paths, *, name="SOLID", solid=True, assoc=False, kind=1, angle=0.0, scale=1.0, double=False,
          families=(), seeds=(), gradient=None, layer="TARAMA"):
    s = head("HATCH", handle, layer) + g(100, "AcDbHatch") + g(10, 0.0) + g(20, 0.0) + g(30, 0.0)
    s += g(210, 0.0) + g(220, 0.0) + g(230, 1.0) + g(2, name) + g(70, 1 if solid else 0) + g(71, 1 if assoc else 0)
    s += g(91, len(paths)) + "".join(paths) + g(75, 0) + g(76, kind)
    if not solid:
        s += g(52, float(angle)) + g(41, float(scale)) + g(77, 1 if double else 0) + g(78, len(families))
        for f in families:
            s += (g(53, float(f["angle"])) + g(43, float(f["base"][0])) + g(44, float(f["base"][1]))
                  + g(45, float(f["offset"][0])) + g(46, float(f["offset"][1])) + g(79, len(f["dashes"])))
            for d in f["dashes"]:
                s += g(49, float(d))
    s += g(98, len(seeds))
    for x, y in seeds:
        s += g(10, float(x)) + g(20, float(y))
    if gradient:
        s += g(450, 1) + g(451, 0) + g(460, float(gradient["angle"])) + g(461, 0.0) + g(452, 0) + g(462, 1.0) + g(453, 2)
        for k, rgb in enumerate(gradient["colours"]):
            s += g(463, float(k)) + g(63, 5) + g(421, rgb)
        s += g(470, gradient["name"])
    return s


def drawing(entities: str) -> str:
    return (g(0, "SECTION") + g(2, "HEADER") + g(9, "$ACADVER") + g(1, "AC1021") + g(9, "$INSUNITS") + g(70, 6)
            + g(0, "ENDSEC")
            + g(0, "SECTION") + g(2, "TABLES") + g(0, "TABLE") + g(2, "LAYER") + g(70, 2)
            + g(0, "LAYER") + g(2, "SINIR") + g(70, 0) + g(62, 7) + g(6, "CONTINUOUS")
            + g(0, "LAYER") + g(2, "TARAMA") + g(70, 0) + g(62, 5) + g(6, "CONTINUOUS")
            + g(0, "ENDTAB") + g(0, "ENDSEC")
            + g(0, "SECTION") + g(2, "ENTITIES") + entities + g(0, "ENDSEC") + g(0, "EOF"))


def rot(a, v):
    r = math.radians(a)
    return (v[0] * math.cos(r) - v[1] * math.sin(r), v[0] * math.sin(r) + v[1] * math.cos(r))


SQUARE = [(0, 0), (10, 0), (10, 10), (0, 10)]


def inputs() -> dict[str, str]:
    files = {}
    # An associative ANSI31 at 0.5 inside a closed polyline, its circle an island.
    ansi31 = [{"angle": 45.0, "base": (0.0, 0.0), "offset": rot(45, (0, 3.175 * 0.5)), "dashes": []}]
    files["assoc-polyline.dxf"] = drawing(
        lwpolyline("2A", SQUARE, reactors=["2C"]) + circle("2B", (5, 5), 1.5, reactors=["2C"])
        + hatch("2C", [poly_path(SQUARE, True, ["2A"]),
                       edge_path([("arc", (5, 5), 1.5, 0.0, 360.0)], False, ["2B"])],
                name="ANSI31", solid=False, assoc=True, kind=1, angle=0.0, scale=0.5, families=ansi31,
                seeds=[(1.0, 1.0)]))
    # An associative solid in the face four lines close.
    room = [(20, 0), (40, 0), (40, 8), (20, 8)]
    lines = "".join(line(h, room[i], room[(i + 1) % 4], reactors=["3E"]) for i, h in enumerate(["3A", "3B", "3C", "3D"]))
    files["assoc-lines.dxf"] = drawing(
        lines + hatch("3E", [edge_path([("line", room[i], room[(i + 1) % 4]) for i in range(4)], True,
                                       ["3A", "3B", "3C", "3D"])], assoc=True, seeds=[(22.0, 2.0)]))
    # Edge paths: a line, an arc, a line and an elliptic arc; a spline closed by a line.
    files["edges.dxf"] = drawing(
        hatch("4A", [edge_path([("line", (0, 0), (10, 0)), ("arc", (10, 5), 5.0, -90.0, 90.0),
                                ("line", (10, 10), (0, 10)), ("ellipse", (0, 5), (0, 5), 0.4, 0.0, 180.0)], True)])
        + hatch("4B", [edge_path([("spline", 3, [0, 0, 0, 0, 1, 1, 1, 1], [(20, 0), (25, 6), (30, 6), (35, 0)]),
                                  ("line", (35, 0), (20, 0))], True)]))
    # Gradients with two true colours.
    grads = [("LINEAR", 0.5235987755982988, 0x1F6FEB, 0xFFFFFF, 0), ("CYLINDER", 0.0, 0xE5484D, 0xFFE0A0, 15),
             ("INVSPHERICAL", 0.0, 0x2F9E44, 0xFFFFFF, 30)]
    files["gradients.dxf"] = drawing("".join(
        hatch(f"5{k}", [poly_path([(x + dx, y) for x, y in SQUARE], True)], gradient={
            "name": name, "angle": angle, "colours": [c1, c2]}) for k, (name, angle, c1, c2, dx) in enumerate(grads)))
    # A custom pattern's families with dashes and dots, and a user-defined double hatch.
    karo = [{"angle": 0.0, "base": (0.0, 0.0), "offset": (0.0, 0.6), "dashes": [0.4, -0.2]},
            {"angle": 90.0, "base": (0.1, 0.0), "offset": rot(90, (0.3, 0.6)), "dashes": [0.0, -0.3]}]
    user = [{"angle": 30.0, "base": (0.0, 0.0), "offset": rot(30, (0.0, 0.25)), "dashes": []},
            {"angle": 120.0, "base": (0.0, 0.0), "offset": rot(120, (0.0, 0.25)), "dashes": []}]
    files["custom-pattern.dxf"] = drawing(
        hatch("6A", [poly_path(SQUARE, True)], name="KAROLAJ", solid=False, kind=2, angle=0.0, scale=1.0,
              families=karo)
        + hatch("6B", [poly_path([(x + 15, y) for x, y in SQUARE], True)], name="_USER", solid=False, kind=0,
                angle=30.0, scale=1.0, double=True, families=user))
    return files


# ── Reading ──────────────────────────────────────────────────────────────────────────────────────────────────────

def pairs(text: str) -> list[tuple[int, str]]:
    lines = text.splitlines()
    return [(int(lines[i].strip()), lines[i + 1].strip()) for i in range(0, len(lines) - 1, 2)]


def entities(text: str) -> list[list[tuple[int, str]]]:
    p = pairs(text)
    start = next(i for i, (c, v) in enumerate(p) if c == 2 and v == "ENTITIES" and p[i - 1] == (0, "SECTION"))
    out, cur = [], None
    for c, v in p[start + 1:]:
        if c == 0:
            if cur is not None:
                out.append(cur)
            if v == "ENDSEC":
                break
            cur = [(c, v)]
        elif cur is not None:
            cur.append((c, v))
    return out


def first(e, code):
    return next((v for c, v in e if c == code), None)


def subclass(e, name):
    """The groups after the subclass marker `name`."""
    i = next(i for i, (c, v) in enumerate(e) if c == 100 and v == name)
    return e[i + 1:]


def read_hatch(e):
    h = subclass(e, "AcDbHatch")
    i = 0

    def take(code):
        nonlocal i
        assert h[i][0] == code, f"{code} beklenirken {h[i]}"
        i += 1
        return h[i - 1][1]

    out = {"name": None, "solid": None, "assoc": None, "paths": [], "families": [], "seeds": [], "gradient": None,
           "angle": None, "scale": None}
    while i < len(h):
        c = h[i][0]
        if c == 2:
            out["name"] = take(2)
        elif c == 70:
            out["solid"] = int(take(70)) == 1
        elif c == 71:
            out["assoc"] = int(take(71)) != 0
        elif c == 91:
            for _ in range(int(take(91))):
                flags = int(take(92))
                path = {"flags": flags, "edges": [], "sources": []}
                if flags & 2:
                    bulge = int(take(72)) != 0
                    take(73)
                    pts = []
                    for _ in range(int(take(93))):
                        x, y = float(take(10)), float(take(20))
                        b = float(take(42)) if bulge and h[i][0] == 42 else 0.0
                        pts.append((x, y, b))
                    path["poly"] = pts
                else:
                    for _ in range(int(take(93))):
                        t = int(take(72))
                        if t == 1:
                            path["edges"].append(("line", (float(take(10)), float(take(20))), (float(take(11)), float(take(21)))))
                        elif t == 2:
                            path["edges"].append(("arc", (float(take(10)), float(take(20))), float(take(40)),
                                                  float(take(50)), float(take(51)), int(take(73)) != 0))
                        elif t == 3:
                            path["edges"].append(("ellipse", (float(take(10)), float(take(20))), (float(take(11)), float(take(21))),
                                                  float(take(40)), float(take(50)), float(take(51)), int(take(73)) != 0))
                        else:
                            degree = int(take(94))
                            rational = int(take(73)) != 0
                            take(74)
                            nk, nc = int(take(95)), int(take(96))
                            knots = [float(take(40)) for _ in range(nk)]
                            ctrl, weights = [], []
                            for _ in range(nc):
                                ctrl.append((float(take(10)), float(take(20))))
                                if rational and h[i][0] == 42:
                                    weights.append(float(take(42)))
                            if h[i][0] == 97 and h[i + 1][0] == 11:
                                for _ in range(int(take(97))):
                                    take(11)
                                    take(21)
                            for code in (12, 22, 13, 23):
                                if h[i][0] == code:
                                    take(code)
                            path["edges"].append(("spline", degree, knots, ctrl, weights or None))
                if h[i][0] == 97:
                    path["sources"] = [take(330) for _ in range(int(take(97)))]
                out["paths"].append(path)
        elif c == 52:
            out["angle"] = float(take(52))
        elif c == 41:
            out["scale"] = float(take(41))
        elif c == 78:
            for _ in range(int(take(78))):
                f = {"angle": float(take(53)), "base": (float(take(43)), float(take(44))),
                     "offset": (float(take(45)), float(take(46)))}
                f["dashes"] = [float(take(49)) for _ in range(int(take(79)))]
                out["families"].append(f)
        elif c == 98:
            out["seeds"] = [(float(take(10)), float(take(20))) for _ in range(int(take(98)))]
        elif c == 450:
            out["gradient"] = {"colours": []}
            i += 1
        elif c == 460 and out["gradient"] is not None:
            out["gradient"]["angle"] = float(take(460))
        elif c == 421 and out["gradient"] is not None:
            out["gradient"]["colours"].append(int(take(421)))
        elif c == 470 and out["gradient"] is not None:
            out["gradient"]["name"] = take(470)
        else:
            i += 1
    return out


# ── Outlines ─────────────────────────────────────────────────────────────────────────────────────────────────────

def arc_points(c, r, a0, a1, ccw, n=720):
    if ccw:
        span = (a1 - a0) % 360.0 or 360.0
    else:
        a0, a1 = -a0, -a1
        span = -(((a0 - a1) % 360.0) or 360.0)
    return [(c[0] + r * math.cos(math.radians(a0 + span * k / n)), c[1] + r * math.sin(math.radians(a0 + span * k / n)))
            for k in range(n + 1)]


def ellipse_points(c, major, ratio, a0, a1, ccw, n=720):
    minor = (-major[1] * ratio, major[0] * ratio)
    if ccw:
        span = (a1 - a0) % 360.0 or 360.0
    else:
        a0, a1 = -a0, -a1
        span = -(((a0 - a1) % 360.0) or 360.0)
    out = []
    for k in range(n + 1):
        t = math.radians(a0 + span * k / n)
        out.append((c[0] + major[0] * math.cos(t) + minor[0] * math.sin(t),
                    c[1] + major[1] * math.cos(t) + minor[1] * math.sin(t)))
    return out


def de_boor(degree, knots, ctrl, weights, u):
    w = weights or [1.0] * len(ctrl)
    k = max(i for i in range(degree, len(ctrl)) if knots[i] <= u) if u < knots[len(ctrl)] else len(ctrl) - 1
    d = [(ctrl[j][0] * w[j], ctrl[j][1] * w[j], w[j]) for j in range(k - degree, k + 1)]
    for r in range(1, degree + 1):
        for j in range(degree, r - 1, -1):
            i = j + k - degree
            den = knots[i + degree - r + 1] - knots[i]
            a = 0.0 if den == 0 else (u - knots[i]) / den
            d[j] = tuple((1 - a) * d[j - 1][m] + a * d[j][m] for m in range(3))
    return (d[degree][0] / d[degree][2], d[degree][1] / d[degree][2])


def spline_points(degree, knots, ctrl, weights, n=720):
    lo, hi = knots[degree], knots[len(ctrl)]
    return [de_boor(degree, knots, ctrl, weights, lo + (hi - lo) * k / n) for k in range(n + 1)]


def bulge_points(a, b, bulge, n=360):
    if bulge == 0:
        return [a, b]
    chord = math.hypot(b[0] - a[0], b[1] - a[1])
    sweep = 4 * math.atan(bulge)
    r = chord / (2 * math.sin(abs(sweep) / 2))
    mid = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
    d = math.sqrt(max(r * r - (chord / 2) ** 2, 0.0))
    nx, ny = -(b[1] - a[1]) / chord, (b[0] - a[0]) / chord
    side = 1 if (bulge > 0) == (abs(sweep) < math.pi) else -1
    c = (mid[0] + side * d * nx, mid[1] + side * d * ny)
    s = math.atan2(a[1] - c[1], a[0] - c[0])
    return [(c[0] + r * math.cos(s + sweep * k / n), c[1] + r * math.sin(s + sweep * k / n)) for k in range(n + 1)]


def outline(path) -> list[tuple[float, float]]:
    pts = []
    if "poly" in path:
        p = path["poly"]
        for i in range(len(p)):
            a, b = p[i], p[(i + 1) % len(p)]
            pts += bulge_points(a[:2], b[:2], a[2])
        return pts
    for e in path["edges"]:
        if e[0] == "line":
            pts += [e[1], e[2]]
        elif e[0] == "arc":
            pts += arc_points(e[1], e[2], e[3], e[4], e[5])
        elif e[0] == "ellipse":
            pts += ellipse_points(e[1], e[2], e[3], e[4], e[5], e[6])
        else:
            pts += spline_points(e[1], e[2], e[3], e[4])
    return pts


def to_segment(p, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    n = dx * dx + dy * dy
    t = 0.0 if n == 0 else max(0.0, min(1.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / n))
    return math.hypot(p[0] - a[0] - t * dx, p[1] - a[1] - t * dy)


def near(points, poly) -> float:
    """The farthest of `points` from the closed outline `poly`."""
    return max(min(to_segment(p, poly[i], poly[(i + 1) % len(poly)]) for i in range(len(poly))) for p in points)


def same_outline(a, b) -> bool:
    return near(a[::7], b) <= TOL * 50 and near(b[::7], a) <= TOL * 50


def inside(p, poly) -> bool:
    at, j = False, len(poly) - 1
    for i in range(len(poly)):
        (ax, ay), (bx, by) = poly[i], poly[j]
        if (ay > p[1]) != (by > p[1]) and p[0] < (bx - ax) * (p[1] - ay) / (by - ay) + ax:
            at = not at
        j = i
    return at


# ── Pattern families as lines ────────────────────────────────────────────────────────────────────────────────────

def family_lines(f):
    """Angle mod 180, spacing across, place across mod spacing, dashes."""
    a = math.radians(f["angle"])
    d = (math.cos(a), math.sin(a))
    n = (-d[1], d[0])
    spacing = abs(f["offset"][0] * n[0] + f["offset"][1] * n[1])
    place = (f["base"][0] * n[0] + f["base"][1] * n[1]) % spacing if spacing else 0.0
    return (f["angle"] % 180.0, spacing, place, f["dashes"])


def same_family(a, b) -> bool:
    (aa, sa, pa, da), (ab, sb, pb, db) = family_lines(a), family_lines(b)
    angle = min(abs(aa - ab), 180 - abs(aa - ab)) < 1e-7
    shift = min(abs(pa - pb), sa - abs(pa - pb)) <= 1e-9 * max(sa, 1.0)
    return (angle and abs(sa - sb) <= 1e-9 * max(sa, 1.0) and shift and len(da) == len(db)
            and all(abs(x - y) <= 1e-9 for x, y in zip(da, db)))


# ── Checking ─────────────────────────────────────────────────────────────────────────────────────────────────────

def geometry(e):
    kind = e[0][1]
    if kind == "LWPOLYLINE":
        xs = [float(v) for c, v in e if c == 10]
        ys = [float(v) for c, v in e if c == 20]
        return ("ring", list(zip(xs, ys)))
    if kind == "CIRCLE":
        return ("circle", (float(first(e, 10)), float(first(e, 20)), float(first(e, 40))))
    if kind == "LINE":
        return ("line", (float(first(e, 10)), float(first(e, 20)), float(first(e, 11)), float(first(e, 21))))
    return (kind, None)


def same_geometry(a, b) -> bool:
    if a[0] != b[0]:
        return False
    if a[0] == "ring":
        return same_outline(a[1], b[1])
    if a[0] == "line":
        p, q = a[1], b[1]
        return (all(abs(p[k] - q[k]) <= TOL for k in range(4))
                or all(abs(p[k] - q[(k + 2) % 4]) <= TOL for k in range(4)))
    return all(abs(x - y) <= TOL for x, y in zip(a[1], b[1]))


def reactors(e):
    out, on = [], False
    for c, v in e:
        if c == 102 and v == "{ACAD_REACTORS":
            on = True
        elif c == 102 and v == "}":
            on = False
        elif on and c == 330:
            out.append(v)
    return out


def check(name: str, given: str, written: str) -> list[str]:
    problems = []
    gi, wo = entities(given), entities(written)
    by_handle = {first(e, 5): e for e in wo}
    gh = [e for e in gi if e[0][1] == "HATCH"]
    wh = [e for e in wo if e[0][1] == "HATCH"]
    if len(gh) != len(wh):
        return [f"{name}: {len(gh)} tarama verildi, {len(wh)} yazıldı"]
    gi_by = {first(e, 5): e for e in gi}
    for n, (ge, we) in enumerate(zip(gh, wh), 1):
        a, b = read_hatch(ge), read_hatch(we)
        where = f"{name} tarama {n}"
        # Outlines, path by path in the given order (KentOS writes the outer path first, then its holes).
        ga, wa = [outline(p) for p in a["paths"]], [outline(p) for p in b["paths"]]
        if len(ga) != len(wa):
            problems.append(f"{where}: {len(ga)} yol verildi, {len(wa)} yazıldı")
            continue
        for k, (x, y) in enumerate(zip(ga, wa)):
            if not same_outline(x, y):
                problems.append(f"{where} yol {k + 1}: sınır aynı değil (en uzak {max(near(x[::7], y), near(y[::7], x)):.3g})")
        # Associativity: sources with the same geometry, reactors naming the hatch, the seed inside.
        if a["assoc"]:
            if not b["assoc"]:
                problems.append(f"{where}: ilişkili tarama ilişkisiz yazıldı")
            for k, (pa, pb) in enumerate(zip(a["paths"], b["paths"])):
                want = [geometry(gi_by[h]) for h in pa["sources"]]
                got = [by_handle.get(h) for h in pb["sources"]]
                if None in got:
                    problems.append(f"{where} yol {k + 1}: kaynak tutamaçlarından biri dosyada yok")
                    continue
                got_g = [geometry(e) for e in got]
                if len(want) != len(got_g) or not all(any(same_geometry(w, x) for x in got_g) for w in want):
                    problems.append(f"{where} yol {k + 1}: kaynaklar aynı nesneler değil ({[w[0] for w in want]} / {[x[0] for x in got_g]})")
                for e in got:
                    if first(we, 5) not in reactors(e):
                        problems.append(f"{where} yol {k + 1}: kaynak {first(e, 5)} taramayı reaktörlerinde anmıyor")
            if not b["seeds"] or not inside(b["seeds"][0], wa[0]):
                problems.append(f"{where}: tohum noktası yok ya da bölgesinde değil")
        # Pattern families: the same lines.
        if not a["solid"]:
            if len(a["families"]) != len(b["families"]):
                problems.append(f"{where}: {len(a['families'])} aile verildi, {len(b['families'])} yazıldı")
            for f in a["families"]:
                if not any(same_family(f, x) for x in b["families"]):
                    problems.append(f"{where}: aile {family_lines(f)} yazılanlarda yok: {[family_lines(x) for x in b['families']]}")
        # Gradients.
        if a["gradient"]:
            ag, bg = a["gradient"], b["gradient"] or {}
            if bg.get("name") != ag["name"] or abs(bg.get("angle", 99) - ag["angle"]) > 1e-9 \
                    or bg.get("colours") != ag["colours"]:
                problems.append(f"{where}: degrade {ag} yerine {bg}")
    return problems


def main() -> int:
    do_check = "--check" in sys.argv[1:]
    bad = False
    (DIR / "in").mkdir(parents=True, exist_ok=True)
    for name, text in inputs().items():
        path = DIR / "in" / name
        if do_check:
            if not path.exists() or path.read_text() != text:
                print(f"{path.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
                bad = True
                continue
            out = DIR / "out" / name
            if not out.exists():
                print(f"{out.relative_to(ROOT)} yok: KENTOS_WRITE_DXF=1 cargo test -p kentos-headless --test headless dxf_hatches_go_round",
                      file=sys.stderr)
                bad = True
                continue
            problems = check(name, text, out.read_text())
            for p in problems:
                print(p, file=sys.stderr)
            bad |= bool(problems)
            if not problems:
                print(f"✓ {name}")
        else:
            path.write_text(text)
            print(f"yazıldı: {path.relative_to(ROOT)}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
