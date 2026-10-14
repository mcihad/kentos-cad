"""What Kır, Birleştir and Yönü çevir keep: shared cases of cad.entities.break, .join and .reverse (docs/adr/0296).

    python3 scripts/fixtures/break_join_policy_cases.py           # writes the files
    python3 scripts/fixtures/break_join_policy_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.break.policy.json, cad.entities.join.policy.json and cad.entities.reverse.json
beside reshape_command_cases.py's files. A drawing of objects that carry everything an object can (layer, colour, line
weight, line type and its scale, transparency, symbol, attributes, label, attachments, elevations, measures, widths)
and associative dimensions tied to them; each is broken, joined or turned the other way, and what every piece, joined
object or turned object keeps is the ADR's policy, computed here without KentOS code:

- Kır: the part between the two points goes (a closed object's from the first counter-clockwise to the second); the
  pieces are the object's sub-paths, a line's lines, a polyline's or an area's polylines (two-vertex ones too), an
  arc's and a circle's arcs. Every piece keeps all the object carried; the first keeps its id.
- Birleştir: chains of objects meeting end to end within the tolerance; a chain of lines on one line running one way is
  that line, a chain of arcs of one circle one arc (or the circle when it closes), any other a polyline (a closed one an
  area). The first object's id and all it carried stay; the others' differing attributes and labels are said; what
  was given and joined nothing is said with why.
- Yönü çevir: lines swap their ends, polylines, areas and fit-point curves run backwards, their arcs turning the other
  way; arcs, circles and kinds without a direction are left out with a warning.
- Elevations and measures go with the vertices: a vertex on a source vertex takes its value, one on a source edge the
  edge's linearly by length. Widths go with the edges, linearly along them; a turned edge swaps them.
- An associative dimension's point keeps its coordinates and follows the object (a piece, the joined one, the turned
  one) it lies on, at the place it lies on there: an end (a line's start or end), a vertex; a point on no piece breaks
  (`vertex_gone`).

Arcs need angles and bulges tan(θ/4), so the plans compare within 1e-9 (`within`); the writes compare the dimensions,
whose points do not move, exactly.
"""
import copy
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

sys.dont_write_bytecode = True  # no __pycache__ in the tree

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
WITHIN = 1e-9
TAU = 2 * math.pi


def node(i, name, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": dict(STYLE), "children": []}


def num(v):
    if isinstance(v, F):
        v = float(v)
    return int(v) if float(v).is_integer() else v


def p(x, y):
    return {"x": num(x), "y": num(y)}


RICH = {"color": "#E5484D", "lineWeight": 0.5, "lineType": "dashed", "lineTypeScale": 2, "transparency": 0.3,
        "symbol": "cit", "label": "D1", "attachments": [{"name": "Foto", "file": "foto/d1.jpg"}]}


def base(i, layer="cizim", attrs=None, **extra):
    return {"id": i, "layerId": layer, "attrs": dict(attrs or {}), **extra}


def line(i, a, b, **extra):
    return {"kind": "line", **base(i, **{k: v for k, v in extra.items() if k not in ("za", "zb", "ma", "mb")}),
            "a": p(*a), "b": p(*b), **{k: extra[k] for k in ("za", "zb", "ma", "mb") if k in extra}}


def polyline(i, pts, bulges=None, closed=False, zs=None, ms=None, widths=None, **extra):
    e = {"kind": "polyline", **base(i, **extra), "pts": [p(*q) for q in pts]}
    if bulges:
        e["bulges"] = bulges
    if closed:
        e["closed"] = True
    if zs:
        e["zs"] = zs
    if ms:
        e["ms"] = ms
    if widths:
        e["widths"] = widths
    return e


def ref(of, place, broken=None):
    r = {"uid": f"$uidOf:{of}", "place": place}
    if broken:
        r["broken"] = broken
    return r


REFS = {}


def dimension(i, a, b, refs):
    """A dimension, untied in the drawing: a case ties it first (cad.dimensions.reassociate), as a file cannot name the
    objects' ids before they have them."""
    REFS[i] = refs
    return {"kind": "dimension", **base(i, layer="olcu"), "a": p(*a), "b": p(*b), "offset": 1.5, "height": 0.25}


def tied(d):
    return {**BY_ID[d], "refs": copy.deepcopy(REFS[d])}


def tie_steps(dims):
    """cad.dimensions.reassociate binds each dimension's points to their places before the case's command."""
    return [{"op": "execute", "command": "cad.dimensions.reassociate",
             "input": {"uid": U(d), "points": [{"point": n, "to": REFS[d][n]} for n in ("a", "b")]},
             "result": "$any", "expect": {"entities": {str(d): tied(d)}}} for d in dims]


START, END = {"kind": "start"}, {"kind": "end"}


def vertex(i):
    return {"kind": "vertex", "part": 0, "ring": 0, "index": i}


ENTITIES = [
    line(1, (0, 0), (10, 0), attrs={"Tür": "Duvar"}, za=1, zb=3, ma=0, mb=10, **RICH),
    polyline(2, [(0, 10), (10, 10), (10, 20)], zs=[1, 2, 3], ms=[0, 10, 20], widths=[[0.2, 0.4], [0.4, 0.6], [0, 0]],
             attrs={"Tür": "Bordür"}, symbol="bordur", lineType="dashdot"),
    polyline(3, [(20, 0), (30, 0)], bulges=[1, 0], attrs={"Tür": "Kemer"}),
    {"kind": "polygon", **base(4, attrs={"Ada": "101"}, label="101"),
     "pts": [p(40, 0), p(50, 0), p(50, 10), p(40, 10)]},
    {"kind": "circle", **base(5, attrs={"Tür": "Kuyu"}), "c": p(60, 5), "r": 5},
    {"kind": "arc", **base(6, attrs={"Tür": "Yay"}), "c": p(80, 0), "r": 5, "a0": 0, "a1": math.pi},
    {"kind": "spline", **base(7), "pts": [p(0, 60), p(5, 64), p(10, 60), p(15, 64)], "closed": False},
    line(8, (0, 70), (10, 70), layer="kilitli"),
    dimension(9, (0, 0), (10, 0), {"a": ref(1, START), "b": ref(1, END)}),
    dimension(10, (0, 10), (10, 20), {"a": ref(2, vertex(0)), "b": ref(2, vertex(2))}),
    # Birleştir.
    line(12, (0, 30), (5, 30), attrs={"Tür": "Yol", "No": "1"}, za=0, zb=1, ma=0, mb=5, color="#3E63DD"),
    line(13, (5, 30), (10, 30), attrs={"Tür": "Yol", "No": "2"}, za=1, zb=2, ma=5, mb=10),
    line(14, (10, 30), (10, 40), za=2, zb=5),
    {"kind": "arc", **base(15, attrs={"Tür": "Kavis"}), "c": p(30, 30), "r": 5, "a0": 0, "a1": math.pi / 2},
    {"kind": "arc", **base(16), "c": p(30, 30), "r": 5, "a0": math.pi / 2, "a1": math.pi},
    {"kind": "arc", **base(17, attrs={"Tür": "Havuz"}), "c": p(50, 30), "r": 5, "a0": 0, "a1": math.pi},
    {"kind": "arc", **base(18), "c": p(50, 30), "r": 5, "a0": math.pi, "a1": 0},
    line(19, (60, 30), (65, 30)),
    {"kind": "arc", **base(20), "c": p(65, 35), "r": 5, "a0": 3 * math.pi / 2, "a1": 0},
    {"kind": "point", **base(21), "p": p(80, 30)},
    line(22, (70, 50), (75, 50)),
    dimension(23, (0, 30), (10, 30), {"a": ref(12, START), "b": ref(13, END)}),
    line(24, (0, 50), (5, 50)),
    line(25, (F(50005, 10000), 50), (10, 50)),
    line(26, (0, 80), (10, 80)),
    line(27, (10, 80), (5, 80)),
    line(28, (20, 80), (30, 80), label="Y1"),
    line(29, (30, 80), (30, 90), label="Y2"),
    line(30, (40, 80), (50, 80), attrs={"Ad": "Üçgen"}),
    line(31, (50, 80), (40, 90)),
    line(32, (40, 90), (40, 80)),
    line(33, (60, 80), (70, 80), layer="kilitli"),
    line(34, (70, 80), (80, 80)),
    # Yönü çevir.
    polyline(35, [(0, 100), (10, 100), (10, 110)], closed=True, zs=[1, 2, 3], attrs={"Tür": "Kapalı"}),
]
BY_ID = {e["id"]: e for e in ENTITIES}
IDS = [e["id"] for e in ENTITIES]
NEXT = max(IDS) + 1
LOCKED = {"kilitli"}
U = lambda i: f"$uidOf:{i}"  # noqa: E731
UNCHANGED = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}


def setup():
    return {
        "format": "kentos.document", "version": 1, "name": "Kır, Birleştir ve Yönü çevir",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 100,
                     "workspace": "cad"},
        "origin": p(40, 50),
        "layers": [node("cizim", "Çizim"), node("olcu", "Ölçü"), node("kilitli", "Kilitli", locked=True)],
        "activeLayer": "cizim",
        "entities": copy.deepcopy(ENTITIES),
        "styles": {"items": [], "categories": []},
    }


# ── Paths ──────────────────────────────────────────────────────────────────────────────────────────────────────────

def xy(q):
    return (q["x"], q["y"])


def norm_angle(a):
    r = math.fmod(a, TAU)
    return r + TAU if r < 0 else r


def bulge_arc(a, b, bulge):
    sweep = 4 * math.atan(bulge)
    chord = math.hypot(b[0] - a[0], b[1] - a[1])
    r = chord / (2 * abs(math.sin(sweep / 2)))
    h = r * math.cos(sweep / 2)
    ux, uy = (b[0] - a[0]) / chord, (b[1] - a[1]) / chord
    side = 1 if sweep > 0 else -1
    c = ((a[0] + b[0]) / 2 - uy * h * side, (a[1] + b[1]) / 2 + ux * h * side)
    return c, r, math.atan2(a[1] - c[1], a[0] - c[0]), sweep


class Path:
    """An object's path by arc length: its edges, ('seg', a, b) or ('arc', c, r, a0, sweep), and vertex values."""

    def __init__(self, e):
        k = e["kind"]
        self.e = e
        if k == "line":
            pts = [xy(e["a"]), xy(e["b"])]
            self.closed = False
            bulges = [0, 0]
        elif k in ("polyline", "polygon"):
            pts = [xy(q) for q in e["pts"]]
            self.closed = k == "polygon" or e.get("closed", False)
            bulges = e.get("bulges") or [0] * len(pts)
        elif k == "circle":
            self.closed = True
            self.edges = [("arc", xy(e["c"]), e["r"], 0.0, TAU)]
            self.pts = []
            self._lengths()
            return
        elif k == "arc":
            self.closed = False
            self.edges = [("arc", xy(e["c"]), e["r"], e["a0"], norm_angle(e["a1"] - e["a0"]) or TAU)]
            self.pts = []
            self._lengths()
            return
        else:
            raise AssertionError(k)
        self.pts = pts
        n = len(pts)
        self.edges = []
        for i in range(n if self.closed else n - 1):
            a, b = pts[i], pts[(i + 1) % n]
            bl = bulges[i] if i < len(bulges) else 0
            self.edges.append(("arc", *bulge_arc(a, b, bl)) if bl else ("seg", a, b))
        self._lengths()

    def _lengths(self):
        self.lens = [length(ed) for ed in self.edges]
        self.cum = [sum(self.lens[:i]) for i in range(len(self.lens))]
        self.length = sum(self.lens)

    def at(self, s):
        if self.closed:
            s = s % self.length
        for i in reversed(range(len(self.edges))):
            if s >= self.cum[i] - 1e-12:
                return point_on(self.edges[i], min(1, (s - self.cum[i]) / self.lens[i]))
        return point_on(self.edges[0], 0)

    def nearest(self, q):
        best = (math.inf, 0)
        for i, ed in enumerate(self.edges):
            d, t = closest(ed, q)
            if d < best[0]:
                best = (d, self.cum[i] + t * self.lens[i])
        return best[1]

    def sub(self, s0, s1):
        """The vertices and bulges of the sub-path s0 → s1 (unwrapped on a closed path)."""
        pts, bulges = [self.at(s0)], []
        rounds = 2 if self.closed else 1
        for rd in range(rounds):
            for i, ed in enumerate(self.edges):
                g0 = self.cum[i] + rd * self.length
                lo, hi = max(s0, g0), min(s1, g0 + self.lens[i])
                if hi - lo <= 1e-9 * max(1, self.length):
                    continue
                t0, t1 = (lo - g0) / self.lens[i], (hi - g0) / self.lens[i]
                bulges.append(math.tan(ed[4] * (t1 - t0) / 4) if ed[0] == "arc" else 0)
                pts.append(point_on(ed, min(1, t1)))
        return pts, bulges


def length(ed):
    if ed[0] == "seg":
        return math.hypot(ed[2][0] - ed[1][0], ed[2][1] - ed[1][1])
    return abs(ed[4]) * ed[2]


def point_on(ed, t):
    if ed[0] == "seg":
        a, b = ed[1], ed[2]
        return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)
    c, r, a0, sw = ed[1:]
    th = a0 + sw * t
    return (c[0] + r * math.cos(th), c[1] + r * math.sin(th))


def on_arc(th, a0, sw):
    d = norm_angle(th - a0) if sw > 0 else norm_angle(a0 - th)
    return d <= abs(sw) + 1e-12


def closest(ed, q):
    if ed[0] == "seg":
        a, b = ed[1], ed[2]
        dx, dy = b[0] - a[0], b[1] - a[1]
        t = max(0, min(1, ((q[0] - a[0]) * dx + (q[1] - a[1]) * dy) / (dx * dx + dy * dy)))
        x = (a[0] + dx * t, a[1] + dy * t)
        return math.hypot(q[0] - x[0], q[1] - x[1]), t
    c, r, a0, sw = ed[1:]
    th = math.atan2(q[1] - c[1], q[0] - c[0])
    if on_arc(th, a0, sw):
        d = norm_angle(th - a0) if sw > 0 else norm_angle(a0 - th)
        return abs(math.hypot(q[0] - c[0], q[1] - c[1]) - r), d / abs(sw)
    pa, pb = point_on(ed, 0), point_on(ed, 1)
    da, db = math.hypot(q[0] - pa[0], q[1] - pa[1]), math.hypot(q[0] - pb[0], q[1] - pb[1])
    return (da, 0) if da <= db else (db, 1)


def ccw_ring(pts, bulges):
    area = 0
    n = len(pts)
    for i in range(n):
        (x1, y1), (x2, y2) = pts[i], pts[(i + 1) % n]
        area += x1 * y2 - x2 * y1
    return area > 0


# ── Values that go with the vertices and the edges ─────────────────────────────────────────────────────────────────

def vertex_values(e, key):
    """The source's values at its vertices: (point, value) for elevations ('z') or measures ('m')."""
    if e["kind"] == "line":
        a, b = (("za", "zb") if key == "z" else ("ma", "mb"))
        if a not in e:
            return None
        return [(xy(e["a"]), e[a]), (xy(e["b"]), e[b])]
    lst = e.get("zs" if key == "z" else "ms")
    if not lst:
        return None
    return list(zip([xy(q) for q in e["pts"]], lst))


def carried(sources, q, key):
    """A vertex's value from the sources: on a source vertex its value, on a source edge linearly by length."""
    for e in sources:
        vals = vertex_values(e, key)
        if not vals:
            continue
        for (v, z) in vals:
            if math.hypot(v[0] - q[0], v[1] - q[1]) <= 1e-6:
                return z
    for e in sources:
        vals = vertex_values(e, key)
        if not vals:
            continue
        closed = e["kind"] == "polygon" or e.get("closed", False)
        n = len(vals)
        for i in range(n if closed else n - 1):
            (a, za), (b, zb) = vals[i], vals[(i + 1) % n]
            d, t = closest(("seg", a, b), q)
            if d <= 1e-6:
                return za + (zb - za) * t
    return None


def edge_widths(e):
    """The source polyline's edges with their widths: [(a, b, w0, w1)]."""
    if e["kind"] != "polyline" or not e.get("widths"):
        return []
    pts = [xy(q) for q in e["pts"]]
    n = len(pts)
    closed = e.get("closed", False)
    return [(pts[i], pts[(i + 1) % n], *e["widths"][i]) for i in range(n if closed else n - 1)]


def widths_of(sources, pts):
    """A new polyline's widths: each edge's from the source edge it runs along, linearly by length (a turned edge's
    swapped); none when all are 0."""
    out = []
    for i in range(len(pts) - 1):
        a, b = pts[i], pts[i + 1]
        w = [0, 0]
        for e in sources:
            for (sa, sb, w0, w1) in edge_widths(e):
                da, ta = closest(("seg", sa, sb), a)
                db, tb = closest(("seg", sa, sb), b)
                if da <= 1e-6 and db <= 1e-6:
                    w = [w0 + (w1 - w0) * ta, w0 + (w1 - w0) * tb]
                    break
            else:
                continue
            break
        out.append(w)
    out.append([0, 0])
    return out if any(x != 0 for pair in out for x in pair) else None


# ── What the objects keep ──────────────────────────────────────────────────────────────────────────────────────────

CARRIED = ("color", "lineWeight", "lineType", "lineTypeScale", "transparency", "symbol", "label", "attachments")


def keeping(e, eid, data=True):
    out = {"id": eid, "layerId": e["layerId"], "attrs": dict(e["attrs"]) if data else {}}
    for k in CARRIED:
        if k in e and (data or k not in ("label", "attachments")):
            out[k] = copy.deepcopy(e[k])
    return out


def as_line(e, eid, a, b, sources):
    out = {"kind": "line", **keeping(e, eid), "a": p(*a), "b": p(*b)}
    for key, (ka, kb) in (("z", ("za", "zb")), ("m", ("ma", "mb"))):
        va, vb = carried(sources, a, key), carried(sources, b, key)
        if va is not None and vb is not None:
            out[ka], out[kb] = va, vb
    return out


def as_polyline(e, eid, pts, bulges, sources, kind="polyline", closed=False):
    out = {"kind": kind, **keeping(e, eid), "pts": [p(*q) for q in pts]}
    if any(b != 0 for b in bulges):
        out["bulges"] = bulges + [0] * (len(pts) - len(bulges))
    if closed and kind == "polyline":
        out["closed"] = True
    for key, name in (("z", "zs"), ("m", "ms")):
        vals = [carried(sources, q, key) for q in pts]
        if any(v is not None for v in vals):  # a vertex no rule reaches has none (null)
            out[name] = vals
    if kind == "polyline" and not closed:
        w = widths_of(sources, pts)
        if w:
            out["widths"] = w
    return out


def as_arc(e, eid, c, r, a0, a1):
    return {"kind": "arc", **keeping(e, eid), "c": p(*c), "r": r, "a0": norm_angle(a0), "a1": norm_angle(a1)}


# ── Kır ────────────────────────────────────────────────────────────────────────────────────────────────────────────

def broken(eid, p1, p2):
    """The pieces (each a callable writing the piece with an id) of the object broken between p1 and p2."""
    e = BY_ID[eid]
    path = Path(e)
    L = path.length
    s1, s2 = path.nearest(p1), path.nearest(p2)
    at_point = abs(s1 - s2) <= 1e-7 * max(1, L)
    if not path.closed:
        if at_point:
            ranges = [(0, s1), (s1, L)]
        else:
            lo, hi = min(s1, s2), max(s1, s2)
            ranges = ([(0, lo)] if lo > 1e-9 else []) + ([(hi, L)] if hi < L - 1e-9 else [])
    else:
        if at_point:
            ranges = [(s1, s1 + L)]
        else:
            ccw = e["kind"] == "circle" or ccw_ring([xy(q) for q in e["pts"]], None)
            if not ccw:
                s1, s2 = s2, s1
            ranges = [(s2, s1 if s1 > s2 else s1 + L)]
    pieces = []
    for (a, b) in ranges:
        if e["kind"] in ("arc", "circle"):
            ed = path.edges[0]
            c, r, a0 = ed[1], ed[2], ed[3]
            pieces.append(lambda pid, a=a, b=b, c=c, r=r, a0=a0: as_arc(e, pid, c, r, a0 + a / r, a0 + b / r))
            continue
        pts, bulges = path.sub(a, b)
        if e["kind"] == "line":
            pieces.append(lambda pid, pts=pts: as_line(e, pid, pts[0], pts[-1], [e]))
        else:
            pieces.append(lambda pid, pts=pts, bulges=bulges: as_polyline(e, pid, pts, bulges, [e]))
    return pieces


def relocate(dim_ids, objects):
    """The dimensions' references after the write: each point on an object of `objects` ({old id: [(id, entity)]})
    looked for on them, an end or a vertex where it lies; else broken."""
    out = {}
    for d in dim_ids:
        dim = tied(d)
        for name in ("a", "b"):
            r = dim["refs"].get(name)
            of = int(r["uid"].split(":")[1])
            if of not in objects:
                continue
            q = xy(dim[name])
            found = None
            for oid, ent in objects[of]:
                place = place_on(ent, q)
                if place is not None:
                    found = (oid, place)
                    break
            if found:
                dim["refs"][name] = ref(found[0], found[1])
            else:
                dim["refs"][name] = {**r, "broken": "vertex_gone"}
        out[d] = dim
    return out


def place_on(ent, q, tol=1e-6):
    """Where q lies on the object: a line's start or end, a polyline's vertex, an arc's start or end; None elsewhere
    (these cases' points lie on ends and vertices)."""
    k = ent["kind"]
    if k == "line":
        for place, v in ((START, ent["a"]), (END, ent["b"])):
            if math.hypot(v["x"] - q[0], v["y"] - q[1]) <= tol:
                return place
        return None
    if k in ("polyline", "polygon"):
        for i, v in enumerate(ent["pts"]):
            if math.hypot(v["x"] - q[0], v["y"] - q[1]) <= tol:
                return vertex(i)
        return None
    raise AssertionError(k)


def break_case(name, eid, p1, p2=None, dims=(), plan_only=False):
    pieces = broken(eid, p1, p1 if p2 is None else p2)
    first = pieces[0](eid)
    created_plan = [pc(0) for pc in pieces[1:]]
    created = [pc(NEXT + k) for k, pc in enumerate(pieces[1:])]
    inp = {"uid": U(eid), "first": p(*p1)}
    if p2 is not None:
        inp["second"] = p(*p2)
    plan = {"op": "plan", "input": inp, "within": WITHIN,
            "result": {"status": "completed", "warnings": [],
                       "output": {"changed": [first], "created": created_plan, "removed": [], "revision": "$current"}},
            "expect": {"ids": IDS, "revision": "same"} if dims else UNCHANGED}
    if plan_only:
        return {"name": name, "steps": [plan]}
    objs = {eid: [(eid, first)] + [(c["id"], c) for c in created]}
    after = relocate(dims, objs)
    execute = {"op": "execute", "input": inp,
               "result": {"status": "completed", "warnings": [],
                          "output": {"changed": [U(eid)], "created": [U(c["id"]) for c in created], "removed": [],
                                     "revision": "$current"}},
               "expect": {"ids": IDS + [c["id"] for c in created], "entities": {str(k): v for k, v in after.items()},
                          "revision": "changed", "canUndo": True}}
    undo = {"op": "undo", "returns": "Kır",
            "expect": {"ids": IDS, "entities": {str(d): tied(d) for d in dims}, "canUndo": bool(dims)}}
    return {"name": name, "steps": [*tie_steps(dims), plan, execute, undo]}


def refused(name, inp, code, message, path, non_finite=None):
    step = {"op": "execute", "input": inp}
    if non_finite:
        step["nonFinite"] = non_finite
    step["result"] = {"status": "failed", "error": {"code": code, "message": message, "path": path}}
    step["expect"] = UNCHANGED
    return {"name": name, "steps": [step]}


LOCKED_MSG = "“Kilitli” katmanı kilitli; üzerindeki nesne değiştirilemez. Kilidi Katmanlar panelinden açın."
MISSING = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"


def not_found(u):
    return f"“{u}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."


def break_cases():
    return [
        break_case("Çizgi iki noktası arasından: iki parça da her şeyi taşır (katman, renk, kalınlık, çizgi tipi ve ölçeği, "
                   "saydamlık, sembol, öznitelik, etiket, ek); kot ve ölçü (M) köşeleriyle", 1, (4, 0), (6, 0), plan_only=True),
        break_case("Çizgi tek noktada: ilişkili ölçünün ucu üstünde durduğu yeni parçayı izler, noktası yerinde kalır", 1, (5, 0),
                   dims=[9]),
        break_case("Çizgi iki noktası arasından: ölçünün sağ ucu yeni parçanın ucunda", 1, (4, 0), (6, 0), dims=[9]),
        break_case("Silinen kısımdaki ölçü noktası kopar (vertex_gone), noktası yerinde kalır", 1, (8, 0), (12, 0), dims=[9]),
        break_case("Çoklu çizgi tek noktada: iki parça da çoklu çizgi kalır, kot, ölçü ve genişlik köşeleri ve kenarlarıyla",
                   2, (5, 10), plan_only=True),
        break_case("Çoklu çizgi tek noktada: ölçünün köşeleri parçalardaki köşelerine geçer", 2, (5, 10), dims=[10]),
        break_case("Çoklu çizgi iki noktası arasından: iki köşeli parça da çoklu çizgi", 2, (5, 10), (10, 15), plan_only=True),
        break_case("Çoklu çizgi köşesinde: parçalar köşede buluşur, ölçünün ucu ikinci parçanın son köşesi", 2, (10, 10), dims=[10]),
        break_case("Yaylı kenarlı çoklu çizgi: yayın iki noktası arası gider, kalan yay parçaları bükümleriyle", 3, (21, -3), (29, -3),
                   plan_only=True),
        break_case("Kapalı alan iki noktası arasından: kalan açık çoklu çizgi alanın verisini ve etiketini taşır", 4, (45, 0), (50, 5),
                   plan_only=True),
        break_case("Kapalı alan tek noktada: halka o noktadan açılır, veriyle", 4, (45, 0), plan_only=True),
        break_case("Daire iki noktası arasından: saat tersi kısım gider, kalan yay dairenin verisini taşır", 5, (63, 9), (57, 9),
                   plan_only=True),
        break_case("Yay iki noktası arasından: iki yay da yayın verisini taşır", 6, (83, 4), (77, 4), plan_only=True),
        break_case("Yay tek noktada: iki yay", 6, (80, 5), plan_only=True),
        break_case("Noktalar ters sırayla: aynı kısım gider", 1, (6, 0), (4, 0), plan_only=True),
        break_case("Nokta çizginin dışındaysa izdüşümü sayılır", 1, (5, 2), plan_only=True),
        refused("Eğri (geçiş noktalı) kırılmaz", {"uid": U(7), "first": p(5, 64)}, "not_breakable",
                "Bu türdeki nesne kırılmaz: çizgi, çoklu çizgi, alan, yay, daire, elips, NURBS eğrisi, yardımcı çizgi ya da ışın olmalı.",
                "uid"),
        refused("Ölçü kırılmaz", {"uid": U(9), "first": p(5, 0)}, "not_breakable",
                "Bu türdeki nesne kırılmaz: çizgi, çoklu çizgi, alan, yay, daire, elips, NURBS eğrisi, yardımcı çizgi ya da ışın olmalı.",
                "uid"),
        refused("Kilitli katmandaki çizgi", {"uid": U(8), "first": p(5, 70)}, "layer_locked", LOCKED_MSG, "uid"),
        refused("Nesne çizimde yok", {"uid": MISSING, "first": p(5, 0)}, "entity_not_found", not_found(MISSING), "uid"),
        refused("Daire tek noktadan kırılmaz", {"uid": U(5), "first": p(65, 5)}, "break_failed",
                "Daire tek noktadan kırılamaz; ikinci bir nokta gösterin.", "first"),
    ]


# ── Birleştir ──────────────────────────────────────────────────────────────────────────────────────────────────────

def ends_chain(e):
    """An object as an open run: (pts, bulges)."""
    if e["kind"] == "line":
        return [xy(e["a"]), xy(e["b"])], [0, 0]
    if e["kind"] == "arc":
        c, r, a0, a1 = xy(e["c"]), e["r"], e["a0"], e["a1"]
        sw = norm_angle(a1 - a0)
        return ([(c[0] + r * math.cos(a0), c[1] + r * math.sin(a0)), (c[0] + r * math.cos(a1), c[1] + r * math.sin(a1))],
                [math.tan(sw / 4), 0])
    pts = [xy(q) for q in e["pts"]]
    return pts, (e.get("bulges") or [0] * len(pts))[:len(pts)]


def reversed_run(run):
    pts, b = run
    n = len(pts)
    rb = [0] * n
    for i in range(n - 1):
        rb[i] = -b[n - 2 - i]
    return pts[::-1], rb


def append(a, b):
    return a[0] + b[0][1:], a[1][:-1] + b[1]


def near(a, b, tol):
    return math.hypot(a[0] - b[0], a[1] - b[1]) <= tol


def chains(ids, tol):
    pool = [i for i in ids if BY_ID[i]["kind"] in ("line", "arc", "polyline") and not BY_ID[i].get("closed")]
    used, groups = set(), []
    for seed in pool:
        if seed in used:
            continue
        used.add(seed)
        run, sources = ends_chain(BY_ID[seed]), [seed]
        grew = True
        while grew:
            grew = False
            for i in pool:
                if i in used:
                    continue
                head, tail = run[0][0], run[0][-1]
                if near(head, tail, tol) and len(run[0]) > 2:
                    break
                c = ends_chain(BY_ID[i])
                if near(tail, c[0][0], tol):
                    run, at_tail = append(run, c), True
                elif near(tail, c[0][-1], tol):
                    run, at_tail = append(run, reversed_run(c)), True
                elif near(head, c[0][-1], tol):
                    run, at_tail = append(c, run), False
                elif near(head, c[0][0], tol):
                    run, at_tail = append(reversed_run(c), run), False
                else:
                    continue
                sources.append(i) if at_tail else sources.insert(0, i)
                used.add(i)
                grew = True
        if len(sources) >= 2:
            groups.append((sources, run))
    return groups


def joined(first, sources, run, tol):
    e = BY_ID[first]
    members = [BY_ID[s] for s in sources]
    pts, bulges = run
    closed = len(pts) > 2 and near(pts[0], pts[-1], tol)
    if closed:
        pts, bulges = pts[:-1], bulges[:-1]
    if all(m["kind"] == "line" for m in members) and not closed:
        a, b = pts[0], pts[-1]
        dx, dy = b[0] - a[0], b[1] - a[1]
        ln = math.hypot(dx, dy)
        ux, uy = dx / ln, dy / ln
        along, ok = 0, True
        for q in pts:
            s = (q[0] - a[0]) * ux + (q[1] - a[1]) * uy
            off = abs((q[0] - a[0]) * uy - (q[1] - a[1]) * ux)
            ok = ok and off <= tol and s >= along - tol
            along = max(along, s)
        if ok:
            return as_line(e, first, a, b, members)
    if all(m["kind"] == "arc" for m in members):
        c, r = xy(members[0]["c"]), members[0]["r"]
        if all(near(xy(m["c"]), c, tol) and abs(m["r"] - r) <= tol for m in members):
            if closed:
                return {"kind": "circle", **keeping(e, first), "c": p(*c), "r": r}
            ccw = all(b > 0 for b in bulges[:len(pts) - 1])
            ang = lambda q: norm_angle(math.atan2(q[1] - c[1], q[0] - c[0]))  # noqa: E731
            a0, a1 = (ang(pts[0]), ang(pts[-1])) if ccw else (ang(pts[-1]), ang(pts[0]))
            return {"kind": "arc", **keeping(e, first), "c": p(*c), "r": r, "a0": a0, "a1": a1}
    if closed:
        out = as_polyline(e, first, pts, bulges, members, kind="polygon")
        return out
    return as_polyline(e, first, pts, bulges, members)


def conflicts(first, sources):
    kept = BY_ID[first]
    keys = {}
    for s in sources:
        if s == first:
            continue
        o = BY_ID[s]
        for k, v in o["attrs"].items():
            if kept["attrs"].get(k) != v:
                vals = keys.setdefault(k, [kept["attrs"].get(k, "yok")])
                if v not in vals:
                    vals.append(v)
        if "label" in o and kept.get("label") != o["label"]:
            vals = keys.setdefault("etiket", [kept.get("label", "yok")])
            if o["label"] not in vals:
                vals.append(o["label"])
    if not keys:
        return None
    listed = ", ".join(f"{k} ({' / '.join(v)})" for k, v in sorted(keys.items()))
    return {"code": "data_conflict", "message": f"Birleşen nesnelerin öznitelikleri ayrışıyor: {listed}; ilk nesnenin değerleri kaldı.",
            "path": "uids"}


def join_case(name, uids, tol=0.001, input_tol=None, dims=(), plan_only=False):
    warnings = []
    targets = []
    for k, i in enumerate(uids):
        if BY_ID[i]["layerId"] in LOCKED:
            warnings.append({"code": "locked_skipped", "message": "Nesne kilitli katmanda olduğu için birleştirilmedi.",
                             "path": f"uids[{k}]"})
        else:
            targets.append(i)
    groups = chains(targets, tol)
    changed, removed, made, objs = [], [], [], {}
    for sources, run in groups:
        first = sources[0]
        ent = joined(first, sources, run, tol)
        changed.append(first)
        removed += [s for s in sources if s != first]
        made.append(ent)
        for s in sources:
            objs[s] = [(first, ent)]
        said = conflicts(first, sources)
        if said:
            warnings.append(said)
    done = [s for g in groups for s in g[0]]
    for k, i in enumerate(uids):
        if i in done or BY_ID[i]["layerId"] in LOCKED:
            continue
        chainable = BY_ID[i]["kind"] in ("line", "arc", "polyline")
        warnings.append({"code": "not_joined", "path": f"uids[{k}]", "message":
                         "Ucu başka bir nesnenin ucuna toleransta değmiyor; birleşmedi." if chainable else
                         "Yalnız çizgi, yay ve açık çoklu çizgi birleşir; bu nesne birleşmedi."})
    inp = {"uids": [U(i) for i in uids]}
    if input_tol is not None:
        inp["tolerance"] = input_tol
    plan = {"op": "plan", "input": inp, "within": WITHIN,
            "result": {"status": "completed", "warnings": warnings,
                       "output": {"changed": made, "created": [], "removed": [U(i) for i in removed], "revision": "$current"}},
            "expect": {"ids": IDS, "revision": "same"} if dims else UNCHANGED}
    if plan_only:
        return {"name": name, "steps": [plan]}
    captures = [{"op": "captureUid", "id": i, "as": f"j{i}"} for i in removed]
    after = relocate(dims, objs)
    execute = {"op": "execute", "input": inp,
               "result": {"status": "completed", "warnings": warnings,
                          "output": {"changed": [U(i) for i in changed], "created": [], "removed": [f"$uid:j{i}" for i in removed],
                                     "revision": "$current"}},
               "expect": {"ids": [i for i in IDS if i not in removed], "entities": {str(k): v for k, v in after.items()},
                          "revision": "changed", "canUndo": True}}
    undo = {"op": "undo", "returns": "Birleştir",
            "expect": {"ids": IDS, "entities": {str(d): tied(d) for d in dims}, "canUndo": bool(dims)}}
    return {"name": name, "steps": [*tie_steps(dims), *captures, plan, execute, undo]}


NOTHING = "Uçları birleşen çizgi, yay ya da açık çoklu çizgi bulunamadı. Toleransı artırmayı deneyin."


def join_cases():
    return [
        join_case("Aynı doğrudaki iki çizgi tek çizgi olur: ilkinin kimliği ve taşıdıkları; kot ve ölçü uçlarla; ayrışan öznitelik "
                  "data_conflict ile söylenir", [12, 13], plan_only=True),
        join_case("Sıra değişince ilk nesne öbürüdür: onun kimliği ve öznitelikleri kalır", [13, 12], plan_only=True),
        join_case("Çizgiler dönünce çoklu çizgi olur, köşelerin kotları kaynaklardan", [12, 13, 14], plan_only=True),
        join_case("Aynı çemberin iki yayı tek yay", [15, 16], plan_only=True),
        join_case("Aynı çemberin kapanan iki yayı daire olur", [17, 18], plan_only=True),
        join_case("Çizgi ve yay bükümlü çoklu çizgi", [19, 20], plan_only=True),
        join_case("Birleşmeyenler nedenleriyle söylenir: nokta türünden, uzaktaki çizgi ucundan (not_joined)", [12, 13, 21, 22],
                  plan_only=True),
        join_case("İlişkili ölçünün uçları birleşen çizgiyi izler: silinen nesnedeki uç da", [12, 13], dims=[23]),
        join_case("Toleranstaki aralık kapanır: tek çizgi", [24, 25], plan_only=True),
        join_case("Geri dönen aynı doğrudaki çizgiler çizgi olmaz, çoklu çizgi kalır", [26, 27], plan_only=True),
        join_case("Etiketleri ayrışan çizgiler: etiket de söylenir", [28, 29], plan_only=True),
        join_case("Kapanan üç çizgi kapalı alan olur, ilkinin verisiyle", [30, 31, 32], plan_only=True),
        join_case("Kilitli katmandaki nesne locked_skipped ile kalır, öbürleri birleşir", [33, 12, 13], plan_only=True),
        join_case("Yazma: ölçü olmadan da tek adım, geri alınır", [15, 16]),
        join_case("Yazma: kapanan yaylar", [17, 18]),
        refused("Kimlik yok", {"uids": []}, "no_entities", "Birleştirilecek nesne yok. En az bir nesnenin kalıcı kimliğini verin.", "uids"),
        refused("Toleransın altındaki aralık kapanmaz", {"uids": [U(24), U(25)], "tolerance": 0.0001}, "nothing_to_join", NOTHING, "uids"),
        refused("Tolerans sıfır olamaz", {"uids": [U(12), U(13)], "tolerance": 0}, "invalid_distance",
                "Tolerans sıfırdan büyük sonlu bir uzunluk (metre) olmalı.", "tolerance"),
        refused("Nesne çizimde yok", {"uids": [MISSING]}, "entity_not_found", not_found(MISSING), "uids[0]"),
        refused("Yalnız noktalar: birleşecek bir şey yok", {"uids": [U(21), U(22)]}, "nothing_to_join", NOTHING, "uids"),
    ]


# ── Yönü çevir ─────────────────────────────────────────────────────────────────────────────────────────────────────

def turned(eid):
    e = BY_ID[eid]
    k = e["kind"]
    out = copy.deepcopy(e)
    if k == "line":
        out["a"], out["b"] = e["b"], e["a"]
        for x, y in (("za", "zb"), ("ma", "mb")):
            if x in e:
                out[x], out[y] = e[y], e[x]
        return out
    if k == "spline":
        out["pts"] = e["pts"][::-1]
        return out
    pts = e["pts"]
    n = len(pts)
    closed = k == "polygon" or e.get("closed", False)
    out["pts"] = pts[::-1]
    if "bulges" in e:
        b = e["bulges"]
        rb = [0] * n
        if closed:
            for i in range(n):
                rb[i] = -b[(2 * n - 2 - i) % n]
        else:
            for i in range(n - 1):
                rb[i] = -b[n - 2 - i]
        out["bulges"] = rb
    for key in ("zs", "ms"):
        if key in e:
            out[key] = e[key][::-1]
    if "widths" in e:
        w = e["widths"]
        # Edge i of the turned path is edge n-2-i (open) run the other way: its widths swapped.
        ww = [[w[n - 2 - i][1], w[n - 2 - i][0]] for i in range(n - 1)] + [[0, 0]]
        out["widths"] = ww
    return out


CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."
NO_DIRECTION = "Nesnenin kendi yönü yok: yay, daire ve elips hep saat yönünün tersine gider; nokta, yazı ve benzerleri yönsüz."


def reverse_case(name, uids, dims=(), twice=False):
    warnings = []
    locked = [i for i in dict.fromkeys(uids) if BY_ID[i]["layerId"] in LOCKED]
    done = []
    for k, i in enumerate(uids):
        if i in done or uids.index(i) != k or i in locked:
            continue
        if BY_ID[i]["kind"] in ("line", "polyline", "polygon", "spline"):
            done.append(i)
        else:
            warnings.append({"code": "not_reversible", "message": NO_DIRECTION, "path": f"uids[{k}]"})
    if locked:
        warnings.insert(0, {"code": "layer_locked", "path": "uids", "message":
                            f"{len(locked)} nesne kilitli katmanda olduğu için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın."})
    inp = {"uids": [U(i) for i in uids]}
    ents = {i: turned(i) for i in done}
    plan = {"op": "plan", "input": inp,
            "result": {"status": "completed", "warnings": warnings,
                       "output": {"changed": [ents[i] for i in done], "created": [], "removed": [], "revision": "$current"}},
            "expect": {"ids": IDS, "revision": "same"} if dims else UNCHANGED}
    after = relocate(dims, {i: [(i, ents[i])] for i in done})
    execute = {"op": "execute", "input": inp,
               "result": {"status": "completed", "warnings": warnings,
                          "output": {"changed": [U(i) for i in done], "created": [], "removed": [], "revision": "$current"}},
               "expect": {"entities": {**{str(i): ents[i] for i in done}, **{str(k): v for k, v in after.items()}},
                          "revision": "changed", "canUndo": True}}
    steps = [*tie_steps(dims), plan, execute]
    if twice:
        steps.append({"op": "execute", "input": inp,
                      "result": {"status": "completed", "warnings": warnings,
                                 "output": {"changed": [U(i) for i in done], "created": [], "removed": [], "revision": "$current"}},
                      "expect": {"entities": {str(i): BY_ID[i] for i in done}}})
    steps.append({"op": "undo", "returns": "Yönü çevir", "expect": {"canUndo": twice or bool(dims)}})
    return {"name": name, "steps": steps}


def reverse_cases():
    return [
        reverse_case("Çizgi: uçları, kotları ve ölçüleri (M) yer değiştirir; kimliği ve bütün taşıdıkları kalır; adım “Yönü çevir”", [1]),
        reverse_case("Çizgi: ilişkili ölçünün uçları yerinde kalır, başlangıç ve son yer değiştirir", [1], dims=[9]),
        reverse_case("Çoklu çizgi: köşeler, kotlar ve ölçüler tersine; genişlikler kenarlarıyla döner, başlangıç ve son yer değiştirir", [2]),
        reverse_case("Çoklu çizgi: ölçünün köşeleri ters sıradaki yerlerine geçer", [2], dims=[10]),
        reverse_case("Yaylı kenarlı çoklu çizgi: yay öbür yöne döner (büküm eksi)", [3]),
        reverse_case("Kapalı alan: halka ters sırayla", [4]),
        reverse_case("Kapalı çoklu çizgi: kapalı kalır, kotları tersine", [35]),
        reverse_case("Eğri (geçiş noktalı): noktaları tersine", [7]),
        reverse_case("Yay ve daire not_reversible uyarısıyla kalır, çizgi döner", [6, 1, 5]),
        reverse_case("Kilitli katmandaki çizgi sayılarak atlanır", [8, 1]),
        reverse_case("Aynı kimlik iki kez: bir kez döner", [1, 1]),
        reverse_case("Birkaç nesne birlikte, tek adım", [1, 2, 4]),
        reverse_case("İki kez çevrilen nesne eski hâlindedir", [2], twice=True),
        refused("Kimlik yok", {"uids": []}, "no_entities", "Yönü çevrilecek nesne yok. En az bir nesnenin kalıcı kimliğini verin.", "uids"),
        refused("Kimlik geçersiz", {"uids": ["Yapı-01"]}, "invalid_uid",
                "“Yapı-01” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). "
                "Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.", "uids[0]"),
        refused("Nesne çizimde yok", {"uids": [MISSING]}, "entity_not_found", not_found(MISSING), "uids[0]"),
        refused("Beklenen sürüm sayı değil", {"uids": [U(1)], "expectedRevision": "on iki"}, "invalid_revision",
                "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.",
                "expectedRevision"),
        refused("Yalnız kilitli: reddedilir", {"uids": [U(8)]}, "layer_locked",
                "“Kilitli” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın.", "uids[0]"),
        refused("Yalnız yay ve daire: not_reversible", {"uids": [U(6), U(5)]}, "not_reversible",
                f"Yönü çevrilecek nesne kalmadı. {NO_DIRECTION}", "uids"),
        refused("Ölçü yönsüz: not_reversible", {"uids": [U(9)]}, "not_reversible", f"Yönü çevrilecek nesne kalmadı. {NO_DIRECTION}", "uids"),
        {"name": "Beklenen sürüm geçmişte kaldıysa conflict, hiçbir şey yazılmaz", "steps": [
            {"op": "captureRevision", "as": "old"},
            {"op": "execute", "input": {"uids": [U(2)]}, "result": "$any"},
            {"op": "execute", "input": {"uids": [U(1)], "expectedRevision": "$old"},
             "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT,
                                                        "path": "expectedRevision", "revision": "$current"}}}]},
    ]


# ── Files ──────────────────────────────────────────────────────────────────────────────────────────────────────────

def build(command, cases, title):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "ADR 0296. Nesnelerin taşıdıkları ve ilişkili ölçüler Kır, Birleştir ve Yönü çevir'de. Bağımsız başvuru: "
                    "scripts/fixtures/break_join_policy_cases.py.",
            "setup": setup(), "cases": cases}


def files():
    return {
        OUT / "cad.entities.break.policy.json": build("cad.entities.break", break_cases(), "Kır: parçaların taşıdıkları ve ilişkili ölçüler"),
        OUT / "cad.entities.join.policy.json": build("cad.entities.join", join_cases(), "Birleştir: AutoCAD'in sonuçları, taşınanlar ve söylenenler"),
        OUT / "cad.entities.reverse.json": build("cad.entities.reverse", reverse_cases(), "Yönü çevir"),
    }


def main():
    check = "--check" in sys.argv[1:]
    bad = 0
    for path, data in files().items():
        text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
        if check:
            if not path.exists() or path.read_text() != text:
                print(f"{path.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
                bad = 1
            else:
                print(f"{path.relative_to(ROOT)} tutarlı: {len(data['cases'])} durum")
        else:
            path.write_text(text)
            print(f"yazıldı: {path.relative_to(ROOT)} ({len(data['cases'])} durum)")
    return bad


if __name__ == "__main__":
    sys.exit(main())
