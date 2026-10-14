"""AutoCAD's TRIM and EXTEND options in cad.entities.trim and .extend: shared cases (docs/adr/0294).

    python3 scripts/fixtures/trim_modes_cases.py           # writes the files
    python3 scripts/fixtures/trim_modes_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.trim.modes.json and cad.entities.extend.modes.json beside
edge_command_cases.py's files: Uzantılar (`edgeMode: "extend"`, AutoCAD's EDGEMODE 1) with lines, arcs and open
polylines as boundaries, quick mode's erase of what no boundary cuts (`eraseUncut`), Çit (`fence`) and Kesişen
(`crossing`) given to the command, a piece an earlier cut left going whole, locked and hidden objects, refusals.

What the commands do is computed here from the ADR's definitions, without KentOS code:

- A boundary bounds where it is; with Uzantılar a line bounds as its whole line, an arc as its whole circle, an open
  polyline's first segment on back past its start and its last on past its end (a straight one as its half line, an
  arc as its circle). Closed objects bound where they are. Hidden objects bound only when given.
- Buda: the target is a path walked from its start; the boundaries cross it at distances s; a pick takes away the
  part between the crossings around it (or the ends). A later pick of the object cuts the piece it lies on; a piece no
  crossing cuts between its ends goes whole. The first pick on an object no boundary cuts: with `eraseUncut` it goes
  whole, else a warning. A closed path needs two crossings; the part kept runs from the crossing after the pick round
  to the one before.
- Çit: each fence segment's crossings with an object's edges, along the segment, are picks; the objects in the
  drawing's order. Kesişen: Buda takes away every piece between crossings that meets the window on a part of positive
  length; Uzat grows every end inside it.
- Uzat: the end nearer the pick goes on along its line (an arc along its circle) to the nearest boundary.

Crossings with circles need square roots and arcs' angles atan2, so the cases compare within 1e-9 (`within`): the
handlers reach a boundary's extension as a segment 10⁶ m long, as a construction line is.
"""
import copy
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True  # no __pycache__ in the tree

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
WITHIN = 1e-9
TAU = 2 * math.pi


def node(i, name, locked=False, visible=True):
    return {"id": i, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True,
            "style": dict(STYLE), "children": []}


def p(x, y):
    return {"x": x, "y": y}


def line(i, a, b, layer="cizim", **extra):
    return {"kind": "line", "id": i, "layerId": layer, "attrs": {}, **extra, "a": p(*a), "b": p(*b)}


def polyline(i, pts, bulges=None, layer="cizim"):
    e = {"kind": "polyline", "id": i, "layerId": layer, "attrs": {}, "pts": [p(*q) for q in pts]}
    if bulges:
        e["bulges"] = bulges
    return e


def polygon(i, pts, holes=None, layer="cizim"):
    e = {"kind": "polygon", "id": i, "layerId": layer, "attrs": {}, "pts": [p(*q) for q in pts]}
    if holes:
        e["holes"] = [{"pts": [p(*q) for q in h]} for h in holes]
    return e


def arc(i, c, r, a0, a1, layer="cizim"):
    return {"kind": "arc", "id": i, "layerId": layer, "attrs": {}, "c": p(*c), "r": r, "a0": a0, "a1": a1}


def circle(i, c, r, layer="cizim"):
    return {"kind": "circle", "id": i, "layerId": layer, "attrs": {}, "c": p(*c), "r": r}


B45 = math.tan(math.pi / 8)  # a quarter turn's bulge
ENTITIES = [
    # Buda along y = 0: what cuts T where it is, and what cuts it only along its extension.
    line(40, (0, 0), (30, 0), attrs={"Tür": "Duvar"}, color="#E5484D"),
    line(41, (5, 2), (5, 6)),
    line(42, (12, -3), (12, 3)),
    arc(43, (16, 4), 5, 0, math.pi),
    polyline(44, [(28, 8), (26, 6), (26, 2)]),
    polyline(45, [(0, -8), (4, -8), (9, -3)], [0, B45, 0]),
    line(46, (3, -2), (3, 2), layer="gizli"),
    line(47, (60, 30), (70, 30)),
    line(48, (60, 40), (70, 40), layer="kilitli"),
    # Çit and Kesişen.
    line(50, (100, 50), (130, 50), attrs={"Tür": "Bordür"}),
    line(51, (110, 47), (110, 53)),
    line(52, (120, 47), (120, 53)),
    polygon(53, [(100, 60), (110, 60), (110, 70), (100, 70)]),
    line(54, (105, 55), (105, 75)),
    circle(55, (150, 100), 5),
    line(56, (140, 103), (160, 103)),
    polygon(57, [(200, 20), (210, 20), (210, 30), (200, 30)], holes=[[(202, 22), (204, 22), (204, 24), (202, 24)]]),
    # Uzat.
    line(70, (300, 200), (303, 200)),
    line(71, (305, 202), (305, 206)),
    line(72, (300, 210), (303, 210)),
    arc(73, (310, 214), 5, 0, math.pi),
    line(74, (300, 220), (303, 220)),
    polyline(75, [(310, 230), (310, 226), (309, 225)]),
    arc(76, (330, 190), 5, 0, math.pi / 2),
    line(77, (327, 196), (327, 198)),
    line(78, (300, 240), (303, 240)),
    line(79, (306, 238), (306, 246)),
    line(80, (300, 244), (303, 244)),
    line(81, (296, 238), (296, 246)),
    line(82, (300, 260), (303, 260), layer="kilitli"),
    polygon(83, [(320, 240), (324, 240), (324, 244), (320, 244)]),
]
BY_ID = {e["id"]: e for e in ENTITIES}
IDS = [e["id"] for e in ENTITIES]
SHOWN = [e for e in ENTITIES if e["layerId"] != "gizli"]
LOCKED = {"kilitli"}
NEXT = max(IDS) + 1
U = lambda i: f"$uidOf:{i}"  # noqa: E731
UNCHANGED = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}


def setup():
    return {
        "format": "kentos.document", "version": 1, "name": "Buda ve Uzat kipleri",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 100,
                     "workspace": "cad"},
        "origin": p(150.0, 120.0),
        "layers": [node("cizim", "Çizim"), node("kilitli", "Kilitli", locked=True), node("gizli", "Gizli", visible=False)],
        "activeLayer": "cizim",
        "entities": copy.deepcopy(ENTITIES),
        "styles": {"items": [], "categories": []},
    }


# ── Geometry ───────────────────────────────────────────────────────────────────────────────────────────────────────

def pt(q):
    return (q["x"], q["y"])


def norm_angle(a):
    r = math.fmod(a, TAU)
    return r + TAU if r < 0 else r


def bulge_arc(a, b, bulge):
    """The arc of a bulged segment a→b: centre, radius, start angle, signed sweep."""
    sweep = 4 * math.atan(bulge)
    chord = math.hypot(b[0] - a[0], b[1] - a[1])
    r = chord / (2 * abs(math.sin(sweep / 2)))
    mx, my = (a[0] + b[0]) / 2, (a[1] + b[1]) / 2
    # The centre lies off the chord's middle, left of a→b for a counter-clockwise arc of less than half a turn.
    h = r * math.cos(sweep / 2)
    ux, uy = (b[0] - a[0]) / chord, (b[1] - a[1]) / chord
    side = 1 if sweep > 0 else -1
    c = (mx - uy * h * side, my + ux * h * side)
    return c, r, math.atan2(a[1] - c[1], a[0] - c[0]), sweep


def edges_of(e):
    """An object's edges in their order: ('seg', a, b) and ('arc', c, r, a0, sweep); a circle's from angle 0."""
    k = e["kind"]
    if k == "line":
        return [("seg", pt(e["a"]), pt(e["b"]))]
    if k in ("polyline", "polygon"):
        pts = [pt(q) for q in e["pts"]]
        closed = k == "polygon"
        bulges = e.get("bulges") or [0] * len(pts)
        out = []
        n = len(pts)
        for i in range(n if closed else n - 1):
            a, b = pts[i], pts[(i + 1) % n]
            bl = bulges[i] if i < len(bulges) else 0
            out.append(("arc", *bulge_arc(a, b, bl)) if bl else ("seg", a, b))
        return out
    if k == "arc":
        return [("arc", pt(e["c"]), e["r"], e["a0"], norm_angle(e["a1"] - e["a0"]) or TAU)]
    if k == "circle":
        return [("arc", pt(e["c"]), e["r"], 0.0, TAU)]
    return []


def extended(e):
    """The boundary's edges under Uzantılar: ('line', a, d) a whole line, ('ray', a, d) a half line from a."""
    k = e["kind"]
    if k == "line":
        a, b = pt(e["a"]), pt(e["b"])
        return [("line", a, (b[0] - a[0], b[1] - a[1]))]
    if k == "arc":
        return [("arc", pt(e["c"]), e["r"], 0.0, TAU)]
    if k == "polyline":
        edges = edges_of(e)
        out = list(edges)

        def grown(ed, back):
            if ed[0] == "arc":
                return [("arc", ed[1], ed[2], 0.0, TAU)]
            a, b = ed[1], ed[2]
            d = (b[0] - a[0], b[1] - a[1])
            # The segment and its half line past the end (back: past its start).
            return [ed, ("ray", a, (-d[0], -d[1])) if back else ("ray", b, d)]

        first = grown(edges[0], True)
        if len(edges) == 1:
            return first + grown(edges[0], False)[1:]
        return first + out[1:-1] + grown(edges[-1], False)
    return edges_of(e)


def on_arc(theta, a0, sweep, eps=1e-9):
    if sweep >= TAU - 1e-12:
        return True
    d = norm_angle(theta - a0) if sweep > 0 else norm_angle(a0 - theta)
    s = abs(sweep)
    return d <= s + eps or d >= TAU - eps


def seg_hits(a, b, edge):
    """Parameters t ∈ [0, 1] along a→b where `edge` crosses it."""
    dx, dy = b[0] - a[0], b[1] - a[1]
    kind = edge[0]
    out = []
    if kind in ("seg", "ray", "line"):
        c = edge[1]
        e = (edge[2][0] - c[0], edge[2][1] - c[1]) if kind == "seg" else edge[2]
        den = dx * e[1] - dy * e[0]
        if den == 0:
            return []
        t = ((c[0] - a[0]) * e[1] - (c[1] - a[1]) * e[0]) / den
        u = ((c[0] - a[0]) * dy - (c[1] - a[1]) * dx) / den
        ok_u = (kind == "line") or (u >= -1e-12 and (kind == "ray" or u <= 1 + 1e-12))
        if ok_u and -1e-12 <= t <= 1 + 1e-12:
            out.append(t)
        return out
    c, r, a0, sweep = edge[1], edge[2], edge[3], edge[4]
    fx, fy = a[0] - c[0], a[1] - c[1]
    A = dx * dx + dy * dy
    Bq = 2 * (fx * dx + fy * dy)
    C = fx * fx + fy * fy - r * r
    disc = Bq * Bq - 4 * A * C
    if disc < 0:
        return []
    for t in sorted({(-Bq - math.sqrt(disc)) / (2 * A), (-Bq + math.sqrt(disc)) / (2 * A)}):
        if -1e-12 <= t <= 1 + 1e-12:
            q = (a[0] + dx * t, a[1] + dy * t)
            if on_arc(math.atan2(q[1] - c[1], q[0] - c[0]), a0, sweep):
                out.append(t)
    return out


def arc_hits(arc_edge, edge):
    """Angles along an arc edge (from its a0, in its direction, as fractions u ∈ [0, 1]) where `edge` crosses it."""
    c, r, a0, sweep = arc_edge[1:]
    out = []
    kind = edge[0]
    if kind in ("seg", "ray", "line"):
        q0 = edge[1]
        d = (edge[2][0] - q0[0], edge[2][1] - q0[1]) if kind == "seg" else edge[2]
        fx, fy = q0[0] - c[0], q0[1] - c[1]
        A = d[0] ** 2 + d[1] ** 2
        Bq = 2 * (fx * d[0] + fy * d[1])
        C = fx * fx + fy * fy - r * r
        disc = Bq * Bq - 4 * A * C
        if disc < 0:
            return []
        for u in {(-Bq - math.sqrt(disc)) / (2 * A), (-Bq + math.sqrt(disc)) / (2 * A)}:
            if kind == "seg" and not (-1e-12 <= u <= 1 + 1e-12):
                continue
            if kind == "ray" and u < -1e-12:
                continue
            q = (q0[0] + d[0] * u, q0[1] + d[1] * u)
            th = math.atan2(q[1] - c[1], q[0] - c[0])
            if on_arc(th, a0, sweep):
                out.append(along_arc(th, a0, sweep))
        return out
    c2, r2, b0, bsweep = edge[1], edge[2], edge[3], edge[4]
    d = math.hypot(c2[0] - c[0], c2[1] - c[1])
    if d == 0 or d > r + r2 or d < abs(r - r2):
        return []
    a = (r * r - r2 * r2 + d * d) / (2 * d)
    h = math.sqrt(max(0.0, r * r - a * a))
    mx, my = c[0] + (c2[0] - c[0]) * a / d, c[1] + (c2[1] - c[1]) * a / d
    for sgn in (1, -1):
        q = (mx - sgn * h * (c2[1] - c[1]) / d, my + sgn * h * (c2[0] - c[0]) / d)
        if on_arc(math.atan2(q[1] - c2[1], q[0] - c2[0]), b0, bsweep):
            th = math.atan2(q[1] - c[1], q[0] - c[0])
            if on_arc(th, a0, sweep):
                out.append(along_arc(th, a0, sweep))
    return out


def along_arc(th, a0, sweep):
    d = norm_angle(th - a0) if sweep > 0 else norm_angle(a0 - th)
    return d / abs(sweep)


class Path:
    """An object as a path parameterised by arc length."""

    def __init__(self, e):
        self.e = e
        self.edges = edges_of(e)
        self.closed = e["kind"] in ("polygon", "circle")
        self.lens = [length_of(ed) for ed in self.edges]
        self.cum = [sum(self.lens[:i]) for i in range(len(self.lens))]
        self.length = sum(self.lens)

    def at(self, s):
        if self.closed:
            s = s % self.length
        s = min(max(s, 0.0), self.length)
        for i in reversed(range(len(self.edges))):
            if s >= self.cum[i] - 1e-12:
                return point_on(self.edges[i], (s - self.cum[i]) / self.lens[i])
        return point_on(self.edges[0], 0)

    def nearest(self, q):
        best = (math.inf, 0.0)
        for i, ed in enumerate(self.edges):
            d, t = closest(ed, q)
            if d < best[0]:
                best = (d, self.cum[i] + t * self.lens[i])
        return best[1]

    def cuts(self, bounds):
        out = []
        for i, ed in enumerate(self.edges):
            for b in bounds:
                ts = seg_hits(ed[1], ed[2], b) if ed[0] == "seg" else arc_hits(ed, b)
                out += [self.cum[i] + t * self.lens[i] for t in ts]
        eps = 1e-7 * max(1.0, self.length)
        if self.closed:
            out = [s % self.length for s in out]
        else:
            out = [s for s in out if eps < s < self.length - eps]
        out.sort()
        kept = []
        for s in out:
            if not kept or s - kept[-1] > eps:
                kept.append(s)
        return kept


def length_of(ed):
    if ed[0] == "seg":
        return math.hypot(ed[2][0] - ed[1][0], ed[2][1] - ed[1][1])
    return abs(ed[4]) * ed[2]


def point_on(ed, t):
    if ed[0] == "seg":
        a, b = ed[1], ed[2]
        return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)
    c, r, a0, sweep = ed[1:]
    th = a0 + sweep * t
    return (c[0] + r * math.cos(th), c[1] + r * math.sin(th))


def closest(ed, q):
    if ed[0] == "seg":
        a, b = ed[1], ed[2]
        dx, dy = b[0] - a[0], b[1] - a[1]
        t = max(0.0, min(1.0, ((q[0] - a[0]) * dx + (q[1] - a[1]) * dy) / (dx * dx + dy * dy)))
        x = (a[0] + dx * t, a[1] + dy * t)
        return math.hypot(q[0] - x[0], q[1] - x[1]), t
    c, r, a0, sweep = ed[1:]
    th = math.atan2(q[1] - c[1], q[0] - c[0])
    if on_arc(th, a0, sweep, 0):
        return abs(math.hypot(q[0] - c[0], q[1] - c[1]) - r), along_arc(th, a0, sweep)
    da = math.hypot(q[0] - point_on(ed, 0)[0], q[1] - point_on(ed, 0)[1])
    db = math.hypot(q[0] - point_on(ed, 1)[0], q[1] - point_on(ed, 1)[1])
    return (da, 0.0) if da <= db else (db, 1.0)


def piece_entity(path, s0, s1, keep, eid):
    """The object a piece from s0 to s1 (unwrapped on a closed path) writes."""
    e = path.e
    out = {"kind": None, "id": eid, "layerId": e["layerId"], "attrs": dict(e["attrs"]) if keep else {}}
    for k in ("color",):
        if k in e:
            out[k] = e[k]
    if e["kind"] in ("arc", "circle"):
        c, r, a0 = path.edges[0][1], path.edges[0][2], path.edges[0][3]
        out.update({"kind": "arc", "c": p(*c), "r": r, "a0": norm_angle(a0 + s0 / r), "a1": norm_angle(a0 + s1 / r)})
        return out
    pts = [path.at(s0)]
    rounds = 2 if path.closed else 1
    for rd in range(rounds):
        for i, ed in enumerate(path.edges):
            g0 = path.cum[i] + rd * path.length
            hi = min(s1, g0 + path.lens[i])
            if hi - max(s0, g0) <= 1e-9 * max(1.0, path.length):
                continue
            pts.append(point_on(ed, min(1.0, (hi - g0) / path.lens[i])))
    if e["kind"] == "line" or len(pts) == 2:
        out.update({"kind": "line", "a": p(*pts[0]), "b": p(*pts[-1])})
        return out
    out.update({"kind": "polyline", "pts": [p(*q) for q in pts]})
    return out


# ── The boundaries ─────────────────────────────────────────────────────────────────────────────────────────────────

def bounds_for(target, boundaries, edge_mode):
    """The boundary edges for `target`: the shown objects (or those given), not itself, along their extensions too."""
    if boundaries is None:
        objs = [e for e in SHOWN if e["id"] != target]
    else:
        objs = [BY_ID[i] for i in boundaries if i != target]
    out = []
    for e in objs:
        out += extended(e) if edge_mode else edges_of(e)
    return out


NO_EDGE = "Tıklanan kısmı kesen bir kenar yok."
TWO_CUTS = "Kapalı nesneyi budamak için en az iki kesişim gerekir."
NO_REACH = "Bu doğrultuda ulaşılacak bir sınır yok."
NO_REACH_ARC = "Bu yönde ulaşılacak bir sınır yok."
LOCKED_SKIP = "{n} nesne kilitli katmanda olduğu için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın."
HOLES = "Adalı alanda budama yapılamaz; iç halkalar kaybolurdu. Önce Patlat ile halkalarına ayırın ya da Alan böl kullanın."


class TrimWork:
    def __init__(self, oid, bounds, erase):
        self.path = Path(BY_ID[oid])
        self.cutsof = self.path.cuts(bounds)
        self.pieces = [(0.0, self.path.length)]
        self.cut = False
        self.erase = erase
        self.done = False

    def piece_at(self, q):
        s = self.path.nearest(q)
        best = None
        for k, (s0, s1) in enumerate(self.pieces):
            for ss in (s, s + self.path.length) if self.path.closed else (s,):
                if s0 - 1e-9 <= ss <= s1 + 1e-9:
                    d = math.hypot(*(a - b for a, b in zip(self.path.at(ss), q)))
                    if d < 1e-6 and (best is None or d < best[1]):
                        best = (k, d, ss)
        return best

    def pick(self, q):
        """A pick: None when it did something or lay on nothing left, else the core's words."""
        L = self.path.length
        eps = 1e-7 * max(1.0, L)
        if not self.cut:
            s = self.path.nearest(q)
            if self.path.closed:
                cuts = self.cutsof
                if len(cuts) < 2:
                    return self.fail(TWO_CUTS)
                lo = max([c for c in cuts if c < s], default=cuts[-1] - L)
                hi = min([c for c in cuts if c > s], default=cuts[0] + L)
                self.pieces = [(hi, lo + L)]
                self.cut = self.done = True
                return None
            k, s0, s1 = 0, 0.0, L
        else:
            found = self.piece_at(q)
            if found is None:
                return None
            k, _, s = found
            s0, s1 = self.pieces[k]
        cuts = [c for c in self.unwrapped() if s0 + eps < c < s1 - eps]
        lo = max([s0] + [c for c in cuts if c < s])
        hi = min([s1] + [c for c in cuts if c > s])
        if lo <= s0 + eps and hi >= s1 - eps:
            if self.cut:  # a piece an earlier cut left: its ends are cuts
                del self.pieces[k]
                self.done = True
                return None
            return self.fail(NO_EDGE)
        now = ([(s0, lo)] if lo > s0 + eps else []) + ([(hi, s1)] if hi < s1 - eps else [])
        self.pieces[k:k + 1] = now
        self.cut = self.done = True
        return None

    def fail(self, why):
        if self.erase:
            self.pieces = []
            self.cut = self.done = True
            return None
        return why

    def unwrapped(self):
        L = self.path.length
        return self.cutsof + [c + L for c in self.cutsof] if self.path.closed else self.cutsof

    def window(self, w):
        """Kesişen: every piece between cuts meeting the window goes; the first that meets it first."""
        for _ in range(1000):
            hit = None
            for k, (s0, s1) in enumerate(self.pieces):
                q = first_inside(self.path, s0, s1, w)
                if q is not None:
                    hit = q
                    break
            if hit is None:
                return None
            why = self.pick(hit)
            if why is not None:
                return why
        raise AssertionError("window did not end")

    def entities(self, oid):
        e = BY_ID[oid]
        keep = len(self.pieces) == 1 and e["kind"] != "polygon"
        ids = [oid] + [NEXT + i for i in range(len(self.pieces) - 1)]
        return [piece_entity(self.path, s0, s1, keep, i) for (s0, s1), i in zip(self.pieces, ids)]


def inside(q, w):
    return w[0][0] <= q[0] <= w[1][0] and w[0][1] <= q[1] <= w[1][1]


def first_inside(path, s0, s1, w):
    """The first point of the path between s0 and s1 on a part of positive length inside the window, sampled finely
    between the window's crossings (exact enough to find the piece: the pieces here are metres long)."""
    n = 4000
    for i in range(n):
        a = s0 + (s1 - s0) * i / n
        b = s0 + (s1 - s0) * (i + 1) / n
        m = path.at((a + b) / 2)
        if inside(m, w) and inside(path.at(a + (b - a) * 0.25), w) and inside(path.at(a + (b - a) * 0.75), w):
            return m
    return None


def fence_hits(e, fence):
    """Where the fence crosses the object, along the fence."""
    out = []
    for a, b in zip(fence, fence[1:]):
        here = []
        for ed in edges_of(e):
            if ed[0] == "seg":
                for t in seg_hits(a, b, ed):
                    here.append((t, (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)))
            else:
                for t in seg_hits(a, b, ed):
                    here.append((t, (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)))
        for _, q in sorted(here):
            if not any(math.hypot(q[0] - r[0], q[1] - r[1]) <= 1e-9 for r in out):
                out.append(q)
    return out


def box_of(pts):
    xs, ys = [q[0] for q in pts], [q[1] for q in pts]
    return (min(xs), min(ys)), (max(xs), max(ys))


def meets_box(e, w):
    """Whether the object's outline meets the rectangle (the store's crossing test), roughly: its edges' samples."""
    path = Path(e) if e["kind"] in ("line", "polyline", "polygon", "arc", "circle") else None
    if path is None:
        return False
    return any(inside(path.at(path.length * i / 2000), w) for i in range(2001)) or any(
        seg_hits(a, b, ed) for ed in path.edges for a, b in rect_sides(w))


def rect_sides(w):
    (x0, y0), (x1, y1) = w
    c = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
    return list(zip(c, c[1:] + c[:1]))


# ── Buda ───────────────────────────────────────────────────────────────────────────────────────────────────────────

def trim(picks=(), fence=None, crossing=None, boundaries=None, edge_mode=False, erase=False):
    """What cad.entities.trim does: (changed ids, created entities, removed ids, entities by id, warnings) or a refusal."""
    work = {}
    order = []
    warnings = []

    def get(oid):
        if oid not in work:
            work[oid] = TrimWork(oid, bounds_for(oid, boundaries, edge_mode), erase)
            order.append(oid)
        return work[oid]

    for n, (oid, q) in enumerate(picks):
        why = get(oid).pick(q)
        if why:
            warnings.append(("trim_failed", why, f"picks[{n}]"))
    locked = 0
    if fence:
        crossed = False
        for e in SHOWN:
            hits = fence_hits(e, fence)
            if not hits:
                continue
            if e["layerId"] in LOCKED:
                locked += 1
                continue
            crossed = True
            if e.get("holes"):
                warnings.append(("trim_failed", HOLES, "fence"))
                continue
            for q in hits:
                why = get(e["id"]).pick(q)
                if why:
                    warnings.append(("trim_failed", why, "fence"))
        if not crossed and not locked:
            warnings.append(("trim_failed", "Çit hiçbir nesneyi kesmiyor; çiti budanacak nesnelerin üstünden geçirin.", "fence"))
    if crossing:
        met = False
        before = locked
        for e in SHOWN:
            if not meets_box(e, crossing):
                continue
            path = Path(e)
            if first_inside(path, 0.0, path.length, crossing) is None:
                continue
            if e["layerId"] in LOCKED:
                locked += 1
                continue
            met = True
            if e.get("holes"):
                warnings.append(("trim_failed", HOLES, "crossing"))
                continue
            why = get(e["id"]).window(crossing)
            if why:
                warnings.append(("trim_failed", why, "crossing"))
        if not met and locked == before:
            warnings.append(("trim_failed", "Pencere budanacak bir parçaya değmiyor; pencereyi nesnelerin üstüne çizin.", "crossing"))
    changed, created, removed, ents = [], [], [], {}
    nxt = NEXT
    for oid in order:
        w = work[oid]
        if not w.done:
            continue
        if not w.pieces:
            removed.append(oid)
            continue
        pieces = w.entities(oid)
        changed.append(oid)
        ents[oid] = pieces[0]
        for e in pieces[1:]:
            e["id"] = nxt
            created.append(e)
            nxt += 1
    path = "picks" if picks else "fence" if fence else "crossing"
    if locked:
        warnings.append(("layer_locked", LOCKED_SKIP.format(n=locked), "fence" if fence else "crossing"))
    if not (changed or removed):
        first = next((w for w in warnings if w[0] == "trim_failed"), None)
        if first is None and locked:
            return ("refused", "layer_locked", LOCKED_SKIP.format(n=locked), path)
        return ("refused", "trim_failed", first[1] if first else "Budanacak bir parça yok; sınırlar bu noktanın iki yanında nesneyi kesmiyor.", path)
    return changed, created, removed, ents, warnings


# ── Uzat ───────────────────────────────────────────────────────────────────────────────────────────────────────────

def ray_reach(tip, d, bounds):
    """The nearest distance t > 1e-7 (in d's units) along the ray where a boundary is."""
    best = math.inf
    far = (tip[0] + d[0] * 1e7, tip[1] + d[1] * 1e7)
    for b in bounds:
        for t in seg_hits(tip, far, b):
            t *= 1e7
            if t > 1e-7:
                best = min(best, t)
    return best


def circle_reach(c, r, frm, ccw, gap, bounds):
    best = math.inf
    full = ("arc", c, r, 0.0, TAU)
    for b in bounds:
        for u in arc_hits(full, b):
            th = u * TAU
            delta = norm_angle(th - frm) if ccw else norm_angle(frm - th)
            if 1e-9 < delta < gap - 1e-9:
                best = min(best, delta)
    return best


def extend_one(e, q, bounds):
    """The object grown at its end nearest q, or the core's words."""
    k = e["kind"]
    if k == "line":
        a, b = pt(e["a"]), pt(e["b"])
        at_end = math.hypot(q[0] - b[0], q[1] - b[1]) <= math.hypot(q[0] - a[0], q[1] - a[1])
        tip, prev = (b, a) if at_end else (a, b)
        ln = math.hypot(tip[0] - prev[0], tip[1] - prev[1])
        d = ((tip[0] - prev[0]) / ln, (tip[1] - prev[1]) / ln)
        t = ray_reach(tip, d, bounds)
        if t == math.inf:
            return NO_REACH
        new = (tip[0] + d[0] * t, tip[1] + d[1] * t)
        out = dict(e)
        out["a"], out["b"] = (p(*a), p(*new)) if at_end else (p(*new), p(*b))
        return out
    if k == "arc":
        c, r, a0, a1 = pt(e["c"]), e["r"], e["a0"], e["a1"]
        to_s = math.hypot(q[0] - (c[0] + math.cos(a0) * r), q[1] - (c[1] + math.sin(a0) * r))
        to_e = math.hypot(q[0] - (c[0] + math.cos(a1) * r), q[1] - (c[1] + math.sin(a1) * r))
        at_end = to_e <= to_s
        gap = TAU - norm_angle(a1 - a0)
        best = circle_reach(c, r, a1 if at_end else a0, at_end, gap, bounds)
        if best == math.inf:
            return NO_REACH_ARC
        out = dict(e)
        if at_end:
            out["a1"] = norm_angle(a1 + best)
        else:
            out["a0"] = norm_angle(a0 - best)
        return out
    return "Yalnız açık çizgiler, çoklu çizgiler ve yaylar uzatılabilir."


def ends_of(e):
    path = Path(e)
    return path.at(0.0), path.at(path.length)


def extend(picks=(), fence=None, crossing=None, boundaries=None, edge_mode=False):
    current, used, order, warnings = {}, {}, [], []

    def grow(oid, q, where):
        e = current.get(oid, BY_ID[oid])
        if oid not in current:
            order.append(oid)
            current[oid] = e
            used[oid] = [False, False]
        if e["kind"] not in ("line", "arc", "polyline"):
            warnings.append(("extend_failed", "Yalnız açık çizgiler, çoklu çizgiler ve yaylar uzatılabilir.", where))
            return
        path = Path(e)
        end = 1 if path.nearest(q) > path.length / 2 else 0
        if used[oid][end]:
            return
        used[oid][end] = True
        r = extend_one(e, q, bounds_for(oid, boundaries, edge_mode))
        if isinstance(r, str):
            warnings.append(("extend_failed", r, where))
            return
        current[oid] = r
        current[oid]["_done"] = True

    for n, (oid, q) in enumerate(picks):
        grow(oid, q, f"picks[{n}]")
    locked = 0
    if fence:
        crossed = False
        for e in SHOWN:
            hits = fence_hits(e, fence)
            if not hits:
                continue
            if e["layerId"] in LOCKED:
                locked += 1
                continue
            crossed = True
            for q in hits:
                grow(e["id"], q, "fence")
        if not crossed and not locked:
            warnings.append(("extend_failed", "Çit hiçbir nesneyi kesmiyor; çiti uzatılacak nesnelerin üstünden geçirin.", "fence"))
    if crossing:
        met = False
        before = locked
        for e in SHOWN:
            if not meets_box(e, crossing):
                continue
            if e["kind"] not in ("line", "arc", "polyline"):
                continue
            ends = [q for q in ends_of(current.get(e["id"], e)) if inside(q, crossing)]
            if not ends:
                continue
            if e["layerId"] in LOCKED:
                locked += 1
                continue
            met = True
            for q in ends:
                grow(e["id"], q, "crossing")
        if not met and locked == before:
            warnings.append(("extend_failed", "Pencere uzatılacak bir uca değmiyor; pencereyi nesnelerin üstüne çizin.", "crossing"))
    changed = [oid for oid in order if current[oid].get("_done")]
    ents = {oid: {k: v for k, v in current[oid].items() if k != "_done"} for oid in changed}
    path = "picks" if picks else "fence" if fence else "crossing"
    if locked:
        warnings.append(("layer_locked", LOCKED_SKIP.format(n=locked), "fence" if fence else "crossing"))
    if not changed:
        first = next((w for w in warnings if w[0] == "extend_failed"), None)
        if first is None and locked:
            return ("refused", "layer_locked", LOCKED_SKIP.format(n=locked), path)
        return ("refused", "extend_failed", first[1] if first else "Uzatılacak bir uç yok; uç hiçbir sınıra ulaşmıyor.", path)
    return changed, [], [], ents, warnings


# ── Cases ──────────────────────────────────────────────────────────────────────────────────────────────────────────

def warning(code, message, path):
    return {"code": code, "message": message, "path": path}


def request(picks=(), fence=None, crossing=None, boundaries=None, edge_mode=False, erase=False):
    inp = {}
    if picks:
        inp["picks"] = [{"uid": U(i), "at": p(*q)} for i, q in picks]
    if fence:
        inp["fence"] = [p(*q) for q in fence]
    if crossing:
        inp["crossing"] = {"min": p(*crossing[0]), "max": p(*crossing[1])}
    if boundaries is not None:
        inp["boundaries"] = {"kind": "objects", "uids": [U(i) for i in boundaries]}
    if edge_mode:
        inp["edgeMode"] = "extend"
    if erase:
        inp["eraseUncut"] = True
    return inp


def case(name, act, **kw):
    """A plan (the objects as they would be written, within 1e-9) and the execute that writes them."""
    run = trim if act == "trim" else extend
    got = run(**kw)
    inp = request(**kw)
    if got[0] == "refused":
        _, code, message, path = got
        return {"name": name, "steps": [{"op": "execute", "input": inp,
                                         "result": {"status": "failed", "error": {"code": code, "message": message, "path": path}},
                                         "expect": UNCHANGED}]}
    changed, created, removed, ents, warns = got
    warnings = [warning(*w) for w in warns]
    plan_created = []
    for e in created:
        e = dict(e)
        e["id"] = 0
        plan_created.append(e)
    plan = {"op": "plan", "input": inp, "within": WITHIN,
            "result": {"status": "completed", "warnings": warnings,
                       "output": {"changed": [ents[i] for i in changed], "created": plan_created,
                                  "removed": [U(i) for i in removed], "revision": "$current"}},
            "expect": UNCHANGED}
    ids = [i for i in IDS if i not in removed] + [e["id"] for e in created]
    execute = {"op": "execute", "input": inp,
               "result": {"status": "completed", "warnings": warnings,
                          "output": {"changed": [U(i) for i in changed], "created": [U(e["id"]) for e in created],
                                     "removed": [f"$uid:gone{i}" for i in removed], "revision": "$current"}},
               "expect": {"ids": ids, "revision": "changed", "canUndo": True, "canRedo": False, "dirty": True}}
    undo = {"op": "undo", "returns": "Buda" if act == "trim" else "Uzat", "expect": {"ids": IDS, "canUndo": False}}
    # A removed object's id, kept before it goes.
    capture = [{"op": "captureUid", "id": i, "as": f"gone{i}"} for i in removed]
    return {"name": name, "steps": [*capture, plan, execute, undo]}


def refused(name, inp, code, message, path, non_finite=None):
    step = {"op": "execute", "input": inp}
    if non_finite:
        step["nonFinite"] = non_finite
    step["result"] = {"status": "failed", "error": {"code": code, "message": message, "path": path}}
    step["expect"] = UNCHANGED
    return {"name": name, "steps": [step]}


def trim_cases():
    T = 40
    out = [
        case("Uzantılar kapalı: 41 T'ye ulaşmıyor, budanacak kısım yok", "trim", picks=[(T, (3, 0))], boundaries=[41]),
        case("Uzantılar açık: 41'in doğrusu T'yi x = 5'te keser; uçla arası gider, tek parça verisini korur", "trim",
             picks=[(T, (3, 0))], boundaries=[41], edge_mode=True),
        case("Uzantılar açık: yay çemberiyle keser (x = 13 ve 19); arası gider, iki parça kalır", "trim",
             picks=[(T, (16, 0))], boundaries=[43], edge_mode=True),
        case("Uzantılar kapalı: yayın kendisi T'ye ulaşmıyor", "trim", picks=[(T, (16, 0))], boundaries=[43]),
        case("Uzantılar açık: çoklu çizginin son kenarı ucunun ötesinde keser (x = 26); ilk kenarı geriye uzar ama T'ye değmez",
             "trim", picks=[(T, (28, 0))], boundaries=[44], edge_mode=True),
        case("Uzantılar açık: çoklu çizginin son kenarı yay: çemberiyle keser (x = 8); başlangıçtaki kesişim T'nin ucunda sayılmaz",
             "trim", picks=[(T, (6, 0))], boundaries=[45], edge_mode=True),
        case("Uzantılar açık, bütün görünen sınırlar: 12 ile 13 arası gider", "trim", picks=[(T, (12.5, 0))], edge_mode=True),
        case("Uzantılar açık, bütün görünen sınırlar: gizli katmanın çizgisi (x = 3) uzantısıyla da sınır değildir", "trim",
             picks=[(T, (2, 0))], edge_mode=True),
        case("Hızlı kip: hiçbir sınırın kesmediği nesne bütünüyle silinir (eraseUncut)", "trim",
             picks=[(47, (65, 30))], erase=True),
        case("eraseUncut olmadan kesilmeyen nesne trim_failed ile kalır", "trim", picks=[(47, (65, 30))]),
        case("eraseUncut: kesilen nesne yalnız tıklanan kısmından budanır", "trim", picks=[(T, (8, 0))], erase=True),
        case("eraseUncut ve Uzantılar: 41'in doğrusu keser, silinmez", "trim", picks=[(T, (3, 0))], boundaries=[41],
             edge_mode=True, erase=True),
        case("Kalan parçayı kesen sınır yoksa parça bütünüyle gider: iki ucu da kesimdir", "trim",
             picks=[(T, (8, 0)), (T, (3, 0))], boundaries=[41, 42], edge_mode=True),
        case("Çit: U ve kapalı alan kesildikleri yerden; alanın ikinci kesişimi gitmiş parçadadır", "trim",
             fence=[(101, 45), (101, 72)]),
        case("Çit ve Uzantılar: 41 ile 42'nin doğruları T'yi keser, çitin geçtiği iki parça gider", "trim",
             fence=[(3, -1), (3, 1), (8, 1), (8, -1)], boundaries=[41, 42], edge_mode=True),
        case("Çit kilitli çizgiyi geçer: sayısıyla atlanır; kesilmeyen öbürü eraseUncut ile silinir", "trim",
             fence=[(65, 45), (65, 25)], erase=True),
        case("Çit hiçbir nesneyi kesmiyor: trim_failed", "trim", fence=[(80, 80), (90, 90)]),
        case("Kesişen: U'nun pencereye değen iki parçası gider (110'a kadar ve 110–120), kalan 120'den sona; 51 iki parçasıyla gider", "trim",
             crossing=((105, 48), (115, 52))),
        case("Kesişen: pencere içindeki iki kesim arası ve ona değen parçalar gider; Uzantılar açık", "trim",
             crossing=((11, -1), (14, 1)), edge_mode=True),
        case("Kesişen: çember iki kesişim arasındaki üst yayından budanır, alt yayı kalır", "trim",
             crossing=((148, 104), (152, 106))),
        case("Kesişen: kapalı alanın pencereye değen kenarı gider; kalan açık çoklu çizgi", "trim",
             crossing=((98, 64), (102, 66))),
        case("Kesişen: hiçbir sınırın kesmediği nesne eraseUncut ile silinir, kilitli olan sayıyla atlanır", "trim",
             crossing=((62, 25), (64, 45)), erase=True),
        case("Kesişen: kesilmeyen nesne eraseUncut olmadan kalır; hiçbir şey yazılmaz, ilk nedenle reddedilir", "trim",
             crossing=((62, 25), (64, 45))),
        case("Kesişen: adalı alan uyarıyla atlanır", "trim", crossing=((199, 25), (201, 26))),
        case("Seçim, çit ve pencere birlikte: önce seçimler, sonra çit, sonra pencere", "trim",
             picks=[(T, (13, 0))], fence=[(101, 45), (101, 55)], crossing=((148, 104), (152, 106))),
    ]
    out += [
        refused("Seçim, çit ve pencere yok", {}, "no_entities", "Budanacak nesne yok. En az bir nesneyi ve tıklandığı yeri verin.", "picks"),
        refused("Çit tek noktalı", {"fence": [p(1, 1)]}, "too_few_points",
                "Çit için en az iki nokta gerekir; çitin noktalarını sırayla verin.", "fence"),
        refused("Çitin noktası sonlu değil", {"fence": [p(1, 1), p(2, 2)]}, "not_finite",
                "Çitin noktasının doğu (Y) değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.",
                "fence[1].x", non_finite={"fence[1].x": "NaN"}),
        refused("Pencerenin köşesi sonlu değil", {"crossing": {"min": p(0, 0), "max": p(1, 1)}}, "not_finite",
                "Pencerenin köşesinin kuzey (X) değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.",
                "crossing.max.y", non_finite={"crossing.max.y": "Infinity"}),
    ]
    return out


def extend_cases():
    out = [
        case("Uzantılar kapalı: 71 E'nin doğrultusunda değil", "extend", picks=[(70, (302, 200))], boundaries=[71]),
        case("Uzantılar açık: uç 71'in doğrusuna (x = 305) uzar", "extend", picks=[(70, (302, 200))], boundaries=[71],
             edge_mode=True),
        case("Uzantılar açık: uç yayın çemberine uzar (x = 307)", "extend", picks=[(72, (302, 210))], boundaries=[73],
             edge_mode=True),
        case("Uzantılar kapalı: yayın kendisine ulaşmıyor", "extend", picks=[(72, (302, 210))], boundaries=[73]),
        case("Uzantılar açık: çoklu çizginin son kenarının ötesine uzar (x = 304)", "extend", picks=[(74, (302, 220))],
             boundaries=[75], edge_mode=True),
        case("Uzantılar kapalı: çoklu çizginin kendisine ulaşmıyor", "extend", picks=[(74, (302, 220))], boundaries=[75]),
        case("Uzantılar açık: yay çemberi boyunca çizginin doğrusuna uzar", "extend", picks=[(76, (330, 195))],
             boundaries=[77], edge_mode=True),
        case("Uzantılar kapalı: yay çizgiye ulaşmıyor", "extend", picks=[(76, (330, 195))], boundaries=[77]),
        case("Uzantılar açık, bütün görünen sınırlar: en yakın uzantı (x = 305)", "extend", picks=[(70, (302, 200))],
             edge_mode=True),
        case("Çit: geçtiği iki çizgi geçtiği yere yakın uçlarından uzar", "extend", fence=[(302, 237), (302, 247)]),
        case("Çit ve Uzantılar: E 71'in doğrusuna uzar", "extend", fence=[(302, 199), (302, 201)], boundaries=[71],
             edge_mode=True),
        case("Çit kilitli çizgiyi de geçer: sayısıyla atlanır", "extend", fence=[(302, 237), (302, 262)]),
        case("Çit hiçbir nesneyi kesmiyor: extend_failed", "extend", fence=[(380, 380), (390, 390)]),
        case("Kesişen: penceredeki uçlar uzar", "extend", crossing=((302, 239), (304, 245))),
        case("Kesişen: iki ucu da penceredeki çizgi iki ucundan uzar", "extend", crossing=((299, 239), (304, 241))),
        case("Kesişen: kapalı alan atlanır, ucu dışarıda kalan nesne de; penceredeki uç uzar", "extend",
             crossing=((302, 239), (321, 241))),
        case("Kesişen: kilitli çizgi sayısıyla atlanır", "extend", crossing=((302, 239), (304, 261))),
        case("Kesişen: yalnız kilitli: layer_locked ile reddedilir", "extend", crossing=((302, 259), (304, 261))),
        case("Kesişen ve Uzantılar: uç 71'in doğrusuna uzar", "extend", crossing=((302, 199), (304, 201)), boundaries=[71],
             edge_mode=True),
        case("Kesişen: ucu pencerede olmayan nesne: extend_failed", "extend", crossing=((350, 350), (360, 360))),
        case("Seçim ve çit birlikte: önce seçim", "extend", picks=[(78, (301, 240))], fence=[(302, 243), (302, 245)]),
    ]
    out += [
        refused("Seçim, çit ve pencere yok", {}, "no_entities", "Uzatılacak nesne yok. En az bir nesneyi ve tıklandığı yeri verin.", "picks"),
        refused("Çit tek noktalı", {"fence": [p(1, 1)]}, "too_few_points",
                "Çit için en az iki nokta gerekir; çitin noktalarını sırayla verin.", "fence"),
    ]
    return out


def build(command, cases, title):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "ADR 0294. AutoCAD'in TRIM ve EXTEND seçenekleri: Uzantılar (edgeMode), hızlı kipin silmesi (eraseUncut), "
                    "Çit (fence) ve Kesişen (crossing) komuta verilir. Bağımsız başvuru: scripts/fixtures/trim_modes_cases.py.",
            "setup": setup(), "cases": cases}


def files():
    return {
        OUT / "cad.entities.trim.modes.json": build("cad.entities.trim", trim_cases(),
                                                    "Buda: Uzantılar, hızlı kipin silmesi, Çit ve Kesişen"),
        OUT / "cad.entities.extend.modes.json": build("cad.entities.extend", extend_cases(),
                                                      "Uzat: Uzantılar, Çit ve Kesişen"),
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
