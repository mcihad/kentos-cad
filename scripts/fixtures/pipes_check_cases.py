#!/usr/bin/env python3
"""The shared cases of `cad.pipes.check` (Boru hattı denetimi; plan/map-cad.md MAP-16, docs/adr/1640), without KentOS
code.

    .run/py/bin/python scripts/fixtures/pipes_check_cases.py           # writes fixtures/commands/v1/cad.pipes.check.json
    .run/py/bin/python scripts/fixtures/pipes_check_cases.py --check   # writes nothing; compares

The clearances (docs/adr/2360) are measured with mpmath 1.4.1 at 50 digits, imported only for them: the profile's
generator imports this file with the standard library alone.

The drawing is a Harita project with a ground lattice (corners every 10 m over [0, 200] × [0, 100] from X0, Y0):

    G(x, y) = 100 − 0.02 x + k(x) + 0.01 |y − 50|,   k(x) = max(0, 0.5 − 0.01 |x − 120|)

Every break of G is on a lattice line, so each square is one plane and the answers do not depend on how a square is
cut into triangles: along a straight pipe the ground is linear between the places the pipe crosses a lattice line, and
so is the crown, and the least and greatest cover are at those places or at the pipe's vertices (a curved pipe: at its
vertices only). Two networks:

- Atıksu (sewer): pipes on `atiksu` with their inverts in `Akar baş` / `Akar son` (metres) and `Çap` (mm), manholes on
  `baca`; drawn with the flow. P1–P4 along y = 50, P5 and P6 branches from the north written in centimetres and as a
  depth, P7 a pipe alone, P8 flat, P9 leaving the ground.
- Yağmursuyu (storm): 3D polylines on `yagmur` whose vertices' heights are the inverts, `Çap` in metres, structures on
  `izgara` with `Kapak kotu`; the direction field `Akış` = `ters` flows against the drawing; Q3 is an arc.

The rules, by hand: a pipe's path length (an arc segment c·θ / (2 sin(θ/2)), θ = 4·atan|b|); the invert linear by
length between the ends' attributes or the vertices' heights; upstream the drawn start unless `ters`; slope (up −
down) / length; rising is reverseSlope, zero flat, below the least lowSlope, above the greatest steepSlope; the crown
the invert plus the diameter (none asked: the invert); cover the ground less the crown; below zero aboveGround, below
the least shallow, above the greatest deep; an end's invert that misses the cover limits but fits them read /100 (cm)
or /1000 (mm) invertUnit, as a depth (cover = value − diameter) invertIsDepth; a diameter not in the catalogue that is
there in another unit diameterUnit, else notInCatalog; parts are the pipes joined at shared ends, all but the longest
detached; an end at no structure noStructure; an outlet invert above the lowest inlet invert at a node outletAbove.

İçme suyu (water, docs/adr/2360): 3D polylines on `su`, `Çap` in mm, no structures; W1–W3 cross P2, P3 and P1 (a
clash, the water below, the water above), W4 crosses the arc Q3, W5 passes P3 and W6 passes under Q3 without crossing,
W7 has no heights. The clearances by hand: the crossings of segments and arcs solved in closed form (a line and a
circle: the quadratic; the arc's centre from its included angle θ = 4·atan b and r = c / (2 sin(θ/2)), at r·cos(θ/2)
from the chord's middle, on its left for b > 0); the stations along each pipe; the inverts there (the attributes or
the heights, never one for the other); the upper pipe the one whose bore's middle (invert + D/2) is higher, the
pipe checked when level; the clearance the upper invert less the lower crown; below zero clash, below the least
closeCrossing, the asked pipe below wrongSide. Pipes that do not cross: the least distance at an end of one edge or
where the common normal meets both (checked against a dense sampling), less the two radii; below the least
closeAlongside.

Et kalınlığı (docs/adr/3960; Civil 3D's cover “from the top outside of the pipe to the reference surface”): in its
own drawing the pipes carry `Et` (the wall, in the diameter's unit; P7 none). Asked, the crown is the invert plus the
diameter plus the wall (a pipe without one noWall, its inner crown); the vertical clearance the upper invert less its
wall less the lower crown less its wall, the horizontal the axes' distance less both radii and walls.
"""

import copy
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.pipes.check.json"
X0, Y0 = 487000.0, 4420000.0
W = 1e-6
KINDS = ["detached", "noStructure", "noInvert", "invertUnit", "invertIsDepth", "reverseSlope", "flat", "lowSlope",
         "steepSlope", "outletAbove", "noDiameter", "diameterUnit", "notInCatalog", "noWall", "noGround", "offSurface",
         "aboveGround", "shallow", "deep", "curved", "clash", "closeCrossing", "wrongSide", "closeAlongside"]
UNITS = {"mm": 0.001, "cm": 0.01, "m": 1.0}


def uid(n):
    return f"0192f640-1640-7000-8000-{n:012x}"


def layer(id_, name):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def k_bump(x):
    return max(0.0, 0.5 - 0.01 * abs(x - 120.0))


def ground(x, y):
    """The ground at local (x, y), none off the lattice."""
    if not (0.0 <= x <= 200.0 and 0.0 <= y <= 100.0):
        return None
    return 100.0 - 0.02 * x + k_bump(x) + 0.01 * abs(y - 50.0)


# ── The drawing ───────────────────────────────────────────────────────────────────────────────────────────────────
# Manholes of the sewer (local places).
M = {1: (0, 50), 2: (50, 50), 3: (100, 50), 4: (150, 50), 5: (200, 50), 6: (50, 80), 7: (100, 80), 8: (200, 20),
     9: (230, 20)}
# Sewer pipes: (name, from, to, Akar baş, Akar son, Çap).
SEWER = [
    ("P1", M[1], M[2], "98.0", "97.5", "300"),
    ("P2", M[2], M[3], "97.5", "97.6", "300"),
    ("P3", M[3], M[4], "97.0", "96.6", "300"),
    ("P4", M[4], M[5], "96.7", "92.0", "400"),
    ("P5", M[6], M[2], "9800", "97.6", "0.2"),
    ("P6", M[7], M[3], "1.5", "97.1", "250"),
    ("P7", (180, 90), (200, 90), None, None, "300"),
    ("P8", M[5], M[8], "92.0", "92.0", "300"),
    ("P9", M[8], M[9], "91.9", "91.6", "300"),
]
# Storm structures: (place, Kapak kotu).
S = [((0, 20), "100.3"), ((60, 20), "99.1"), ((100, 20), "98.6"), ((140, 20), None)]
# Storm pipes: (name, points, heights, bulges, Akış, Çap in metres).
STORM = [
    ("Q1", [(0, 20), (30, 20), (60, 20)], [99.0, 98.7, 98.5], None, None, "0.4"),
    ("Q2", [(100, 20), (60, 20)], [98.0, 98.3], None, "ters", "0.4"),
    ("Q3", [(100, 20), (140, 20)], [97.9, 97.5], [0.2], None, "0.5"),
]


# Water pipes (İçme suyu): (name, points, heights or none, Çap in mm), drawn after the road line.
WATER = [
    ("W1", [(75, 90), (75, 30)], [97.8, 97.6], "100"),
    ("W2", [(125, 90), (125, 30)], [96.2, 96.0], "200"),
    ("W3", [(25, 90), (25, 30)], [98.6, 98.4], "100"),
    ("W4", [(120, 10), (120, 30)], [98.5, 98.3], "100"),
    ("W5", [(105, 53), (145, 54)], [98.0, 97.9], "100"),
    ("W6", [(110, 13), (130, 13)], [98.0, 97.9], "100"),
    ("W7", [(175, 60), (175, 40)], None, "100"),
]


# The walls (docs/adr/3960; the `Et` attribute, in the network's diameter unit): P7 has none.
WALLS = {"P1": "20", "P2": "20", "P3": "25", "P4": "30", "P5": "15", "P6": "18", "P8": "20", "P9": "20",
         "W1": "5", "W2": "8", "W3": "5", "W4": "5", "W5": "5", "W6": "5", "W7": "5"}


def walled(attrs, name, walls):
    if walls and name in WALLS:
        attrs["Et"] = WALLS[name]
    return attrs


def P(p):
    return {"x": X0 + p[0], "y": Y0 + p[1]}


def setup(work="map", water=False, walls=False):
    layers = [layer("0", "0"), layer("zemin", "Zemin"), layer("atiksu", "Atıksu"), layer("baca", "Baca"),
              layer("yagmur", "Yağmursuyu"), layer("izgara", "Izgara"), layer("yol", "Yol"), layer("bos", "Boş")]
    if water:
        layers.append(layer("su", "İçme suyu"))
    ents, uids, n = [], [], 0

    def add(e):
        nonlocal n
        n += 1
        e["id"] = n
        ents.append(e)
        uids.append(uid(n))
        return n

    for j in range(11):
        for i in range(21):
            x, y = 10.0 * i, 10.0 * j
            add({"kind": "point", "layerId": "zemin", "attrs": {}, "p": P((x, y)), "z": ground(x, y)})
    for m in sorted(M):
        add({"kind": "point", "layerId": "baca", "attrs": {"Ad": f"B{m}"}, "p": P(M[m])})
    for name, a, b, i0, i1, d in SEWER:
        attrs = {"Ad": name, "Çap": d}
        if i0 is not None:
            attrs["Akar baş"] = i0
            attrs["Akar son"] = i1
        add({"kind": "line", "layerId": "atiksu", "attrs": walled(attrs, name, walls), "a": P(a), "b": P(b)})
    for k, (p, cover) in enumerate(S):
        attrs = {"Ad": f"I{k + 1}"}
        if cover is not None:
            attrs["Kapak kotu"] = cover
        add({"kind": "point", "layerId": "izgara", "attrs": attrs, "p": P(p)})
    for name, pts, zs, bulges, flow, d in STORM:
        attrs = {"Ad": name, "Çap": d}
        if flow:
            attrs["Akış"] = flow
        e = {"kind": "polyline", "layerId": "yagmur", "attrs": attrs, "pts": [P(p) for p in pts],
             "zs": zs}
        if bulges:
            e["bulges"] = bulges + [0.0]
        add(e)
    add({"kind": "line", "layerId": "yol", "attrs": {}, "a": P((0, 0)), "b": P((50, 0))})
    if water:
        for name, pts, zs, d in WATER:
            e = {"kind": "polyline", "layerId": "su", "attrs": walled({"Ad": name, "Çap": d}, name, walls),
                 "pts": [P(q) for q in pts]}
            if zs:
                e["zs"] = zs
            add(e)
    networks = [
        {"id": "atiksu", "name": "Atıksu", "kind": "utility", "edges": [{"layer": "atiksu"}],
         "junctions": [{"layer": "baca", "role": "junction"}], "connect": "ends", "tolerance": 0.01,
         "direction": {"kind": "digitized"}},
        {"id": "yagmur", "name": "Yağmursuyu", "kind": "utility", "edges": [{"layer": "yagmur"}],
         "junctions": [{"layer": "izgara", "role": "junction"}], "connect": "ends", "tolerance": 0.01,
         "direction": {"kind": "field", "field": "Akış", "forward": [], "backward": ["ters"], "closed": []}},
        {"id": "yollar", "name": "Yollar", "kind": "road", "edges": [{"layer": "yol"}], "connect": "ends",
         "tolerance": 0.01, "direction": {"kind": "both"}},
    ]
    if water:
        networks.append({"id": "su", "name": "İçme suyu", "kind": "utility", "edges": [{"layer": "su"}],
                         "connect": "ends", "tolerance": 0.01, "direction": {"kind": "digitized"}})
    settings = {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                "plotScale": 1000.0, "workspace": "cad", "cadWork": work, "drawingFont": "barlow",
                "networks": networks}
    if work in ("map", "road"):
        settings["road"] = {"surfaces": [{"id": "t-1", "name": "Zemin", "points": {"layers": ["zemin"]}},
                                         {"id": "t-2", "name": "Boş", "points": {"layers": ["bos"]}}]}
    if work == "gis":
        settings["workspace"] = "gis"
        settings.pop("cadWork")
    return {"format": "kentos.document", "version": 2, "name": "Altyapı", "settings": settings,
            "origin": {"x": X0, "y": Y0}, "layers": layers, "activeLayer": "0", "entities": ents, "uids": uids,
            "styles": {"items": [], "categories": []}}


# ── The answers ───────────────────────────────────────────────────────────────────────────────────────────────────
def number(text):
    if text is None:
        return None
    t = text.strip()
    body = t[1:] if t.startswith("-") else t
    whole, sep, frac = body.replace(",", ".").partition(".")
    if not whole.isdigit() or (sep and not frac.isdigit()):
        return None
    return float(t.replace(",", "."))


def arc(chord, bulge):
    if bulge == 0.0:
        return chord
    th = 4.0 * math.atan(abs(bulge))
    return chord * th / (2.0 * math.sin(th / 2.0))


class Pipe:
    def __init__(self, n, name, pts, bulges, attrs, heights, layer_):
        self.n, self.name, self.pts, self.attrs, self.heights, self.layer = n, name, pts, attrs, heights, layer_
        self.bulges = bulges or [0.0] * (len(pts) - 1)
        self.curved = any(b != 0.0 for b in self.bulges)
        self.st = [0.0]
        for i in range(len(pts) - 1):
            c = math.hypot(pts[i + 1][0] - pts[i][0], pts[i + 1][1] - pts[i][1])
            self.st.append(self.st[-1] + arc(c, self.bulges[i]))
        self.length = self.st[-1]

    def invert(self, ends, s):
        if ends is not None:
            i0, i1 = ends
            return i0 + (i1 - i0) * s / self.length
        if self.heights is None:
            return None
        for k in range(len(self.st) - 1):
            if s <= self.st[k + 1] or k == len(self.st) - 2:
                s0, s1 = self.st[k], self.st[k + 1]
                h = self.heights
                return h[k] + (h[k + 1] - h[k]) * (s - s0) / (s1 - s0)

    def samples(self):
        """(station, place) at the vertices and, for a straight pipe, every lattice line crossing on the lattice."""
        if self.curved:
            return list(zip(self.st, self.pts))
        out = []
        for i in range(len(self.pts) - 1):
            (ax, ay), (bx, by) = self.pts[i], self.pts[i + 1]
            ts = {0.0, 1.0}
            for v in range(0, 201, 10):
                if ax != bx:
                    t = (v - ax) / (bx - ax)
                    y = ay + t * (by - ay)
                    if 0 < t < 1 and 0 <= y <= 100:
                        ts.add(t)
            for v in range(0, 101, 10):
                if ay != by:
                    t = (v - ay) / (by - ay)
                    x = ax + t * (bx - ax)
                    if 0 < t < 1 and 0 <= x <= 200:
                        ts.add(t)
            for t in sorted(ts):
                if out and t == 0.0:
                    continue
                out.append((self.st[i] + t * (self.st[i + 1] - self.st[i]), (ax + t * (bx - ax), ay + t * (by - ay))))
        return out


def drawing(walls=False):
    """The two networks' pipes and structures, in the drawing's order (with their walls in the walls' drawing)."""
    n = 21 * 11
    baca = {}
    for m in sorted(M):
        n += 1
        baca[M[m]] = n
    sewer = []
    for name, a, b, i0, i1, d in SEWER:
        n += 1
        attrs = {"Ad": name, "Çap": d}
        if i0 is not None:
            attrs.update({"Akar baş": i0, "Akar son": i1})
        sewer.append(Pipe(n, name, [a, b], None, walled(attrs, name, walls), None, "atiksu"))
    izgara = {}
    for p, cover in S:
        n += 1
        izgara[p] = cover
    storm = []
    for name, pts, zs, bulges, flow, d in STORM:
        n += 1
        attrs = {"Ad": name, "Çap": d}
        if flow:
            attrs["Akış"] = flow
        storm.append(Pipe(n, name, pts, bulges, attrs, zs, "yagmur"))
    n += 1  # the road line
    water = []
    for name, pts, zs, d in WATER:
        n += 1
        water.append(Pipe(n, name, pts, None, walled({"Ad": name, "Çap": d}, name, walls), zs, "su"))
    return {"atiksu": (sewer, {p: {} for p in baca}), "yagmur": (storm, {p: {"Kapak kotu": c} for p, c in
                                                                          izgara.items()}), "su": (water, {})}


def wall_of(attrs, diameter):
    """The wall in metres (docs/adr/3960): none asked: None; asked and missing (or below zero): False."""
    if not diameter or not diameter.get("wall"):
        return None
    v = number(attrs.get(diameter["wall"]))
    return v * UNITS[diameter["unit"]] if v is not None and v >= 0 else False


def report(net, inverts, diameter=None, ground_=None, lo=None, hi=None, smin=None, smax=None, clearance=None,
           walls=False):
    pipes, structures = drawing(walls)[net]
    found, rows = [], []
    key = lambda p: (round(p[0], 6), round(p[1], 6))
    # Parts: pipes joined at shared ends; all but the longest detached.
    parent = list(range(len(pipes)))

    def root(i):
        while parent[i] != i:
            i = parent[i]
        return i

    ends = {}
    for i, p in enumerate(pipes):
        for q in (p.pts[0], p.pts[-1]):
            ends.setdefault(key(q), []).append(i)
    for lst in ends.values():
        for j in lst[1:]:
            a, b = root(lst[0]), root(j)
            if a != b:
                parent[max(a, b)] = min(a, b)
    part_len = {}
    for i, p in enumerate(pipes):
        part_len[root(i)] = part_len.get(root(i), 0.0) + p.length
    longest = max(part_len, key=lambda r: (part_len[r], -r))
    at_node_in, at_node_out = {}, {}

    def mark(i, kind, at, value=None, unit=None):
        f = {"kind": kind, "uid": uid(pipes[i].n), "at": [X0 + at[0], Y0 + at[1]]}
        if value is not None:
            f["value"] = value
        if unit is not None:
            f["unit"] = unit
        found.append((i, KINDS.index(kind), len(found), f))

    for i, p in enumerate(pipes):
        start, end = p.pts[0], p.pts[-1]
        rev = p.attrs.get("Akış") == "ters"
        L = p.length
        if root(i) != longest:
            mark(i, "detached", start, part_len[root(i)])
        for q in (start, end):
            if key(q) not in {key(s) for s in structures}:
                mark(i, "noStructure", q)
        if inverts == "fields":
            a, b = number(p.attrs.get("Akar baş")), number(p.attrs.get("Akar son"))
            ends_ = (a, b) if a is not None and b is not None else None
            known = ends_ is not None
        else:
            ends_ = None
            known = p.heights is not None
        if not known:
            mark(i, "noInvert", start)
        dm = None
        dread = None
        if diameter:
            v = number(p.attrs.get(diameter["field"]))
            dread = v if v is not None and v > 0 else None
            dm = dread * UNITS[diameter["unit"]] if dread is not None else None
        wall = wall_of(p.attrs, diameter)
        if wall is False:
            mark(i, "noWall", start)
        wm = wall if wall not in (None, False) else None
        crown = 0.0 if not diameter else (None if dm is None else dm + (wm or 0.0))

        def g_at(q):
            if ground_ and ground_["from"] == "surface":
                return ground(*q)
            if ground_ and ground_["from"] == "structures":
                return number(structures.get(key(q), {}).get(ground_["field"])) if key(q) in structures else None
            return None

        eg = [g_at(start), g_at(end)]
        if ground_ and ground_["from"] == "surface":
            samples = [(s, q, ground(*q)) for s, q in p.samples()]
        elif ground_:
            samples = [(0.0, start, eg[0]), (L, end, eg[1])]
        else:
            samples = []
        if ends_ and lo is not None and hi is not None and crown is not None:
            for v, g0, q in ((ends_[0], eg[0], start), (ends_[1], eg[1], end)):
                if g0 is None:
                    continue
                fits = lambda c: lo <= c <= hi
                if fits(g0 - (v + crown)):
                    continue
                unit = next((u for kk, u in ((100.0, "cm"), (1000.0, "mm")) if fits(g0 - (v / kk + crown))), None)
                if unit:
                    mark(i, "invertUnit", q, v, unit)
                elif fits(v - crown):
                    mark(i, "invertIsDepth", q, v)
        i0, i1 = p.invert(ends_, 0.0), p.invert(ends_, L)
        up, down = (i1, i0) if rev else (i0, i1)
        slope = (up - down) / L if up is not None and down is not None else None
        if slope is not None:
            if slope < 0:
                mark(i, "reverseSlope", start, slope)
            elif slope == 0:
                mark(i, "flat", start, slope)
            elif smin is not None and slope < smin:
                mark(i, "lowSlope", start, slope)
            if smax is not None and slope > smax:
                mark(i, "steepSlope", start, slope)
        if up is not None and down is not None:
            frm, to = (end, start) if rev else (start, end)
            at_node_out.setdefault(key(frm), []).append((i, up, frm))
            at_node_in.setdefault(key(to), []).append((i, down))
        if diameter:
            if dread is None:
                mark(i, "noDiameter", start)
            elif diameter.get("catalog"):
                cat = diameter["catalog"]
                inc = lambda v: any(abs(v - c) <= 1e-9 * max(abs(c), 1.0) for c in cat)
                if not inc(dread):
                    other = next((u for u in ("mm", "cm", "m") if u != diameter["unit"]
                                  and inc(dread * UNITS[u] / UNITS[diameter["unit"]])), None)
                    if other:
                        mark(i, "diameterUnit", start, dread, other)
                    else:
                        mark(i, "notInCatalog", start, dread)
        mn = mx = None
        if ground_:
            on = [s for s in samples if s[2] is not None]
            if not on:
                mark(i, "noGround", start)
            else:
                off = next((s for s in samples if s[2] is None), None)
                if off:
                    mark(i, "noGround" if ground_["from"] == "structures" else "offSurface", off[1])
            if crown is not None:
                for s, q, gz in on:
                    inv = p.invert(ends_, s)
                    if inv is None:
                        continue
                    c = gz - (inv + crown)
                    if mn is None or c < mn[0]:
                        mn = (c, q)
                    if mx is None or c > mx[0]:
                        mx = (c, q)
            if mn:
                if mn[0] < 0:
                    mark(i, "aboveGround", mn[1], mn[0])
                elif lo is not None and mn[0] < lo:
                    mark(i, "shallow", mn[1], mn[0])
            if mx and hi is not None and mx[0] > hi:
                mark(i, "deep", mx[1], mx[0])
        if p.curved and ground_:
            mark(i, "curved", start)
        row = {"uid": uid(p.n), "layer": p.layer, "length": L}
        if rev:
            row["reversed"] = True
        for k_, v in (("upstream", up), ("downstream", down), ("slope", slope), ("diameter", dm), ("wall", wm)):
            if v is not None:
                row[k_] = v
        if mn:
            row["minCover"] = mn[0]
            row["minCoverAt"] = [X0 + mn[1][0], Y0 + mn[1][1]]
        if mx:
            row["maxCover"] = mx[0]
        rows.append(row)
    for node, outs in at_node_out.items():
        ins = at_node_in.get(node)
        if not ins:
            continue
        low = min(d for _, d in ins)
        for i, o, at in outs:
            if o > low:
                mark(i, "outletAbove", at, o - low)
    # The attributes the report reads the checked pipes' inverts from (above), or their heights.
    asked_inverts = {"from": "fields", "start": "Akar baş", "end": "Akar son"} if inverts == "fields" else \
        {"from": "heights"}
    crossings, near = clearances(pipes, asked_inverts, diameter, clearance, mark, walls) if clearance else ([], [])
    found.sort(key=lambda t: (t[0], t[1], t[2]))
    for row in rows:
        kinds = []
        for i, _, _, f in found:
            if f["uid"] == row["uid"] and f["kind"] not in kinds:
                kinds.append(f["kind"])
        row["findings"] = kinds
    counts = [{"kind": kk, "count": sum(1 for t in found if t[3]["kind"] == kk)} for kk in KINDS]
    cover_to = ("outside" if diameter.get("wall") else "crown") if diameter else "invert"
    out = {"network": net, "coverTo": cover_to, "pipes": rows,
           "findings": [t[3] for t in found], "counts": [c for c in counts if c["count"]],
           "parts": len(part_len), "length": sum(p.length for p in pipes)}
    if crossings:
        out["crossings"] = crossings
    if near:
        out["near"] = near
    return out


# ── The clearances (docs/adr/2360) ────────────────────────────────────────────────────────────────────────────────
class Plan:
    """A pipe as the clearances take it: its edges in mpmath (local coordinates), where its inverts come from and half
    its diameter (0 with no diameter asked, none when asked and missing)."""

    def __init__(self, mp, pipe, inverts, diameter):
        self.mp, self.pipe = mp, pipe
        self.edges = []
        for i in range(len(pipe.pts) - 1):
            (ax, ay), (bx, by) = [tuple(mp.mpf(v) for v in q) for q in (pipe.pts[i], pipe.pts[i + 1])]
            b = pipe.bulges[i]
            s0, s1 = mp.mpf(pipe.st[i]), mp.mpf(pipe.st[i + 1])
            if b == 0.0:
                self.edges.append(("seg", (ax, ay), (bx, by), s0, s1))
                continue
            c = mp.hypot(bx - ax, by - ay)
            th = 4 * mp.atan(mp.mpf(b))
            r = c / (2 * mp.sin(abs(th) / 2))
            ux, uy = (bx - ax) / c, (by - ay) / c
            h = r * mp.cos(th / 2) * (1 if th > 0 else -1)
            cx, cy = (ax + bx) / 2 - uy * h, (ay + by) / 2 + ux * h
            self.edges.append(("arc", (cx, cy), r, mp.atan2(ay - cy, ax - cx), th, s0, s1))
        if inverts["from"] == "fields":
            a, b = number(pipe.attrs.get(inverts["start"])), number(pipe.attrs.get(inverts["end"]))
            self.ends = (a, b) if a is not None and b is not None else None
            self.mode = "fields"
        else:
            self.ends, self.mode = None, "heights"
        if diameter is None:
            self.radius = 0.0
        else:
            v = number(pipe.attrs.get(diameter["field"]))
            self.radius = v * UNITS[diameter["unit"]] / 2.0 if v is not None and v > 0 else None
        # The wall (docs/adr/3960): 0 where none is asked or given.
        w = wall_of(pipe.attrs, diameter)
        self.wall = w if w not in (None, False) else 0.0
        self.length = pipe.length

    def invert(self, s):
        """The invert at station s: only from where the question says, never the heights for missing attributes."""
        if self.mode == "fields":
            return self.pipe.invert(self.ends, s) if self.ends is not None else None
        return self.pipe.invert(None, s) if self.pipe.heights is not None else None


def travelled(mp, a0, sweep, phi):
    """How far along an arc (0..1 of its sweep) the angle phi is; none off the arc."""
    k = 1 if sweep > 0 else -1
    t = mp.fmod(k * (phi - a0), 2 * mp.pi)
    if t < 0:
        t += 2 * mp.pi
    if t > abs(sweep) + mp.mpf(10) ** -30:
        if 2 * mp.pi - t < mp.mpf(10) ** -30:
            return mp.mpf(0)
        return None
    return t / abs(sweep)


def on_edge(mp, e, q):
    """The fraction along edge e of a place q on its line or circle; none off the edge."""
    if e[0] == "seg":
        (ax, ay), (bx, by) = e[1], e[2]
        d2 = (bx - ax) ** 2 + (by - ay) ** 2
        t = ((q[0] - ax) * (bx - ax) + (q[1] - ay) * (by - ay)) / d2
        return t if 0 <= t <= 1 else None
    (cx, cy), _, a0, th = e[1], e[2], e[3], e[4]
    return travelled(mp, a0, th, mp.atan2(q[1] - cy, q[0] - cx))


def hits(mp, e, f):
    """The places two edges cross: (fraction along e, fraction along f, place)."""
    out = []
    if e[0] == "seg" and f[0] == "seg":
        (px, py), (qx, qy) = e[1], f[1]
        rx, ry = e[2][0] - px, e[2][1] - py
        sx, sy = f[2][0] - qx, f[2][1] - qy
        den = rx * sy - ry * sx
        if den == 0:
            return out
        t = ((qx - px) * sy - (qy - py) * sx) / den
        u = ((qx - px) * ry - (qy - py) * rx) / den
        if 0 <= t <= 1 and 0 <= u <= 1:
            out.append((t, u, (px + t * rx, py + t * ry)))
        return out
    if e[0] == "arc" and f[0] == "seg":
        return [(u, t, q) for t, u, q in hits(mp, f, e)]
    if e[0] == "seg":
        (ax, ay), (bx, by) = e[1], e[2]
        (cx, cy), r = f[1], f[2]
        dx, dy, fx, fy = bx - ax, by - ay, ax - cx, ay - cy
        A, B, C = dx * dx + dy * dy, 2 * (fx * dx + fy * dy), fx * fx + fy * fy - r * r
        disc = B * B - 4 * A * C
        if disc < 0:
            return out
        for t in sorted({(-B - mp.sqrt(disc)) / (2 * A), (-B + mp.sqrt(disc)) / (2 * A)}):
            if 0 <= t <= 1:
                q = (ax + t * dx, ay + t * dy)
                u = on_edge(mp, f, q)
                if u is not None:
                    out.append((t, u, q))
        return out
    (c1x, c1y), r1, (c2x, c2y), r2 = e[1], e[2], f[1], f[2]
    d = mp.hypot(c2x - c1x, c2y - c1y)
    if d == 0 or d > r1 + r2 or d < abs(r1 - r2):
        return out
    a = (r1 * r1 - r2 * r2 + d * d) / (2 * d)
    h = mp.sqrt(max(r1 * r1 - a * a, 0))
    mx, my = c1x + a * (c2x - c1x) / d, c1y + a * (c2y - c1y) / d
    for k in (1, -1):
        q = (mx + k * h * (c2y - c1y) / d, my - k * h * (c2x - c1x) / d)
        t, u = on_edge(mp, e, q), on_edge(mp, f, q)
        if t is not None and u is not None:
            out.append((t, u, q))
    return out


def point(mp, e, t):
    if e[0] == "seg":
        return (e[1][0] + t * (e[2][0] - e[1][0]), e[1][1] + t * (e[2][1] - e[1][1]))
    (cx, cy), r, a0, th = e[1], e[2], e[3], e[4]
    return (cx + r * mp.cos(a0 + th * t), cy + r * mp.sin(a0 + th * t))


def nearest_on(mp, e, q):
    """The place of edge e nearest to q, and the distance."""
    if e[0] == "seg":
        (ax, ay), (bx, by) = e[1], e[2]
        d2 = (bx - ax) ** 2 + (by - ay) ** 2
        t = min(max(((q[0] - ax) * (bx - ax) + (q[1] - ay) * (by - ay)) / d2, 0), 1)
        w = (ax + t * (bx - ax), ay + t * (by - ay))
        return mp.hypot(q[0] - w[0], q[1] - w[1]), w
    (cx, cy), r = e[1], e[2]
    if on_edge(mp, e, q) is not None:
        g = mp.hypot(q[0] - cx, q[1] - cy)
        return abs(g - r), (cx + r * (q[0] - cx) / g, cy + r * (q[1] - cy) / g)
    ends = [point(mp, e, 0), point(mp, e, 1)]
    return min((mp.hypot(q[0] - w[0], q[1] - w[1]), w) for w in ends)


def least(mp, e, f):
    """The least distance between two edges that do not cross: (distance, place on e, place on f)."""
    cands = []
    for t in (0, 1):
        q = point(mp, e, t)
        d, w = nearest_on(mp, f, q)
        cands.append((d, q, w))
        q = point(mp, f, t)
        d, w = nearest_on(mp, e, q)
        cands.append((d, w, q))
    kinds = (e[0], f[0])
    if kinds in (("seg", "arc"), ("arc", "seg")):
        sg, ar = (e, f) if e[0] == "seg" else (f, e)
        (ax, ay), (bx, by) = sg[1], sg[2]
        l = mp.hypot(bx - ax, by - ay)
        nx, ny = -(by - ay) / l, (bx - ax) / l
        (cx, cy), r = ar[1], ar[2]
        for k in (1, -1):
            pa = (cx + k * r * nx, cy + k * r * ny)
            if on_edge(mp, ar, pa) is None:
                continue
            t = ((pa[0] - ax) * (bx - ax) + (pa[1] - ay) * (by - ay)) / (l * l)
            if 0 <= t <= 1:
                ps = (ax + t * (bx - ax), ay + t * (by - ay))
                d = mp.hypot(pa[0] - ps[0], pa[1] - ps[1])
                cands.append((d, ps, pa) if e[0] == "seg" else (d, pa, ps))
    if kinds == ("arc", "arc"):
        (c1x, c1y), r1, (c2x, c2y), r2 = e[1], e[2], f[1], f[2]
        d = mp.hypot(c2x - c1x, c2y - c1y)
        if d > 0:
            ux, uy = (c2x - c1x) / d, (c2y - c1y) / d
            for k1 in (1, -1):
                q1 = (c1x + k1 * r1 * ux, c1y + k1 * r1 * uy)
                if on_edge(mp, e, q1) is None:
                    continue
                for k2 in (1, -1):
                    q2 = (c2x + k2 * r2 * ux, c2y + k2 * r2 * uy)
                    if on_edge(mp, f, q2) is not None:
                        cands.append((mp.hypot(q1[0] - q2[0], q1[1] - q2[1]), q1, q2))
    return min(cands, key=lambda x: x[0])


def sampled_least(mp, a, b, n=600):
    """The least distance of two plans by dense sampling, the closed form's check."""
    pa = [point(mp, e, mp.mpf(i) / n) for e in a.edges for i in range(n + 1)]
    pb = [point(mp, e, mp.mpf(i) / n) for e in b.edges for i in range(n + 1)]
    return min(float(mp.hypot(x[0] - y[0], x[1] - y[1])) for x in pa for y in pb)


def clearances(pipes, inverts, diameter, c, mark, walls=False):
    """The checked pipes' crossings with the other network's and their near passes, marking the findings."""
    import mpmath as mp
    mp.mp.dps = 50
    others, _ = drawing(walls)[c["network"]]
    mine = [Plan(mp, p, inverts, diameter) for p in pipes]
    theirs = [Plan(mp, p, c["inverts"], c.get("diameter")) for p in others]
    crossings, near = [], []
    xy = lambda q: [X0 + float(q[0]), Y0 + float(q[1])]
    for i, a in enumerate(mine):
        for b in theirs:
            found = []
            for ea in a.edges:
                for eb in b.edges:
                    for t, u, q in hits(mp, ea, eb):
                        sa = ea[-2] + t * (ea[-1] - ea[-2])
                        sb = eb[-2] + u * (eb[-1] - eb[-2])
                        found.append((sa, sb, q))
            found.sort(key=lambda x: x[0])
            for sa, sb, q in found:
                ia, ib = a.invert(float(sa)), b.invert(float(sb))
                row = {"pipe": uid(a.pipe.n), "other": uid(b.pipe.n), "at": xy(q)}
                if ia is not None:
                    row["invert"] = ia
                if ib is not None:
                    row["otherInvert"] = ib
                if None not in (ia, ib, a.radius, b.radius):
                    up = ia + a.radius >= ib + b.radius
                    gap = (ia - a.wall) - (ib + 2 * b.radius + b.wall) if up else (ib - b.wall) - (ia + 2 * a.radius + a.wall)
                    row["clearance"] = gap
                    row["above"] = "pipe" if up else "other"
                    local = (float(q[0]), float(q[1]))
                    if gap < 0:
                        mark(i, "clash", local, gap)
                    elif c.get("vertical") is not None and gap < c["vertical"]:
                        mark(i, "closeCrossing", local, gap)
                    if c.get("above") is not None and c["above"] != row["above"]:
                        mark(i, "wrongSide", local, gap)
                crossings.append(row)
            if c.get("horizontal") is None or found:
                continue
            d, qa, qb = min((least(mp, ea, eb) for ea in a.edges for eb in b.edges), key=lambda x: x[0])
            got = sampled_least(mp, a, b) if float(d) < 50 else float(d)
            assert -1e-9 <= got - float(d) < 1e-3, (a.pipe.name, b.pipe.name, got, float(d))
            gap = None if None in (a.radius, b.radius) else float(d) - (a.radius + a.wall) - (b.radius + b.wall)
            if (gap if gap is not None else float(d)) < c["horizontal"]:
                mark(i, "closeAlongside", (float(qa[0]), float(qa[1])), gap)
                row = {"pipe": uid(a.pipe.n), "other": uid(b.pipe.n), "at": xy(qa), "otherAt": xy(qb),
                       "distance": float(d)}
                if gap is not None:
                    row["clearance"] = gap
                near.append(row)
    return crossings, near


# ── The cases ─────────────────────────────────────────────────────────────────────────────────────────────────────
SAME = {"revision": "same", "canUndo": False}
SEWER_IN = {"from": "fields", "start": "Akar baş", "end": "Akar son"}
SEWER_D = {"field": "Çap", "unit": "mm", "catalog": [200.0, 300.0, 400.0, 500.0]}
ZEMIN = {"from": "surface", "surface": "Zemin"}
LIMITS = {"minCover": 0.8, "maxCover": 3.0, "minSlope": 0.009, "maxSlope": 0.05}
STORM_D = {"field": "Çap", "unit": "m"}
KAPAK = {"from": "structures", "field": "Kapak kotu"}
NOT_MAP = ("not_a_road_project", "Arazi yüzeyi Harita ve Yol projelerinde olur (KentOS Harita, KentOS Yol); bu çizim "
                                 "ikisinden biri değil.")


def ok(name, input_, out, op="plan", setup_=None):
    step = {"op": op, "input": input_, "result": {"status": "completed", "output": None if op == "validate" else out,
                                                 "warnings": []}, "expect": SAME, "within": W}
    case = {"name": name, "steps": [step]}
    if setup_ is not None:
        case["setup"] = setup_
    return case


def refused(name, input_, code, message, path_=None, setup_=None):
    err = {"code": code, "message": message}
    if path_ is not None:
        err["path"] = path_
    case = {"name": name, "steps": [{"op": "execute", "input": input_, "result": {"status": "failed", "error": err},
                                     "expect": SAME}]}
    if setup_ is not None:
        case["setup"] = setup_
    return case


def sewer_input(**over):
    inp = {"network": "atiksu", "inverts": SEWER_IN, "diameter": SEWER_D, "ground": ZEMIN, **LIMITS}
    for k_, v in over.items():
        if v is None:
            inp.pop(k_, None)
        else:
            inp[k_] = v
    return inp


def sewer(walls=False, **over):
    inp = sewer_input(**over)
    return inp, report("atiksu", "fields", inp.get("diameter"), inp.get("ground"), inp.get("minCover"),
                       inp.get("maxCover"), inp.get("minSlope"), inp.get("maxSlope"), inp.get("clearance"), walls)


def storm(**over):
    inp = {"network": "yagmur", "inverts": {"from": "heights"}, "diameter": STORM_D, "ground": KAPAK, **over}
    inp = {k_: v for k_, v in inp.items() if v is not None}
    inv = inp["inverts"]["from"]
    return inp, report("yagmur", inv, inp.get("diameter"), inp.get("ground"), inp.get("minCover"),
                       inp.get("maxCover"), inp.get("minSlope"), inp.get("maxSlope"), inp.get("clearance"))


def cases():
    full_in, full = sewer()
    kinds = {f["kind"] for f in full["findings"]}
    want = {"detached", "noStructure", "noInvert", "invertUnit", "invertIsDepth", "reverseSlope", "flat", "lowSlope",
            "steepSlope", "outletAbove", "diameterUnit", "notInCatalog", "offSurface", "aboveGround", "shallow", "deep"}
    assert want <= kinds, want - kinds
    st_in, st = storm(minCover=0.5, maxCover=3.0)
    assert not any(f["kind"] == "reverseSlope" for f in st["findings"]), st["findings"]
    assert {"noGround"} <= {f["kind"] for f in st["findings"]}, st["findings"]
    curved_in, curved = storm(ground=ZEMIN)
    assert "curved" in {f["kind"] for f in curved["findings"]}
    out = [
        ok("atıksu: akar kotları öznitelikte, çap mm ve katalog, örtü Zemin yüzeyinden, dört sınır", full_in, full),
        ok("uygula: salt okunur, çizim değişmez", full_in, full, op="execute"),
        ok("doğrulama", full_in, None, op="validate"),
        ok("sınırsız: yalnız her projede yanlış olan (ters eğim, düz, zeminin üstü, bağlantı)", *sewer(
            minCover=None, maxCover=None, minSlope=None, maxSlope=None)),
        ok("çapsız: örtü akar kotuna", *sewer(diameter=None)),
        ok("katalogsuz çap: birim önerisi yok", *sewer(diameter={"field": "Çap", "unit": "mm"})),
        ok("zeminsiz: örtü denetlenmez", *sewer(ground=None)),
        ok("yalnız en az örtü", *sewer(maxCover=None, minSlope=None, maxSlope=None)),
        ok("ağ adıyla, büyük küçük harf ayrımsız", {**full_in, "network": "ATIKSU"}, full),
        ok("yağmursuyu: köşe kotları, Akış = ters akışın yönünü çevirir, örtü ızgaraların kapak kotundan", st_in, st),
        ok("yağmursuyu yüzeyle: yaylı boruda örtü yalnız köşelerde", curved_in, curved),
        ok("yağmursuyu öznitelikten akar kotu yok", *storm(inverts=SEWER_IN)),
        ok("CBS projesinde kapak kotuyla", st_in, st, setup_=setup("gis")),
        ok("Yol projesinde yüzeyle", full_in, full, setup_=setup("road")),
    ]
    bad = "invalid_pipe_check"
    out += [
        refused("boş baş akar kotu özniteliği", sewer_input(inverts={"from": "fields", "start": " ", "end": "x"}),
                bad, "Baş akar kotunun özniteliği boş; özniteliğin adını yazın.", "inverts.start"),
        refused("boş son akar kotu özniteliği", sewer_input(inverts={"from": "fields", "start": "x", "end": ""}),
                bad, "Son akar kotunun özniteliği boş; özniteliğin adını yazın.", "inverts.end"),
        refused("boş çap özniteliği", sewer_input(diameter={"field": "", "unit": "mm"}), bad,
                "Çapın özniteliği boş; özniteliğin adını yazın.", "diameter.field"),
        refused("katalogda sıfır çap", sewer_input(diameter={"field": "Çap", "unit": "mm", "catalog": [200.0, 0.0]}),
                bad, "Katalogdaki çaplar 0'dan büyük olmalı.", "diameter.catalog[1]"),
        refused("201 çaplık katalog", sewer_input(diameter={"field": "Çap", "unit": "mm",
                                                           "catalog": [float(i + 1) for i in range(201)]}),
                bad, "Çap kataloğunda en çok 200 çap olur.", "diameter.catalog"),
        refused("boş yüzey", sewer_input(ground={"from": "surface", "surface": ""}), bad,
                "Zemin yüzeyi boş; yüzeyin adını ya da kimliğini yazın.", "ground.surface"),
        refused("boş kapak kotu özniteliği", sewer_input(ground={"from": "structures", "field": " "}), bad,
                "Kapak kotunun özniteliği boş; özniteliğin adını yazın.", "ground.field"),
        refused("sıfır en az örtü", sewer_input(minCover=0.0), bad,
                "En az örtü 0'dan büyük bir sayı olmalı; denetlenmeyecekse boş bırakın.", "minCover"),
        refused("eksi en çok eğim", sewer_input(maxSlope=-0.01), bad,
                "En çok eğim 0'dan büyük bir sayı olmalı; denetlenmeyecekse boş bırakın.", "maxSlope"),
        refused("en az örtü en çoktan büyük", sewer_input(minCover=3.5), bad,
                "En az örtü en çok örtüden büyük olamaz.", "maxCover"),
        refused("en az eğim en çoktan büyük", sewer_input(minSlope=0.06), bad,
                "En az eğim en çok eğimden büyük olamaz.", "maxSlope"),
        refused("olmayan ağ", sewer_input(network="kanal"), "unknown_network",
                "“kanal” kimlikli ya da adlı ağ projede yok; Ağlar'da tanımlı bir ağı seçin.", "network"),
        refused("yol ağı", sewer_input(network="yollar"), "not_a_pipe_network",
                "“Yollar” bir yol ağı; boru hattı denetimi altyapı ağlarını denetler.", "network"),
        refused("olmayan yüzey", sewer_input(ground={"from": "surface", "surface": "Kazı"}), "unknown_terrain",
                "“Kazı” kimlikli ya da adlı yüzey projede yok.", "ground.surface"),
        refused("kurulamayan yüzey", sewer_input(ground={"from": "surface", "surface": "Boş"}), "terrain_unbuildable",
                "$any", None),
        refused("CBS projesinde yüzey", sewer_input(), NOT_MAP[0], NOT_MAP[1], None, setup_=setup("gis")),
    ]
    out += clearance_cases()
    return out


WATER_ASK = {"network": "su", "inverts": {"from": "heights"}, "diameter": {"field": "Çap", "unit": "mm"}}


def clearance_cases():
    """The clearances to the water network (docs/adr/2360)."""
    base_in, base = sewer(clearance=WATER_ASK)
    by_pipe = {(c["pipe"], c["other"]): c for c in base["crossings"]}
    assert len(base["crossings"]) == 4, base["crossings"]
    w = {name: uid(258 + k) for k, (name, *_rest) in enumerate(WATER)}
    p = {name: uid(240 + k + 1) for k, (name, *_rest) in enumerate(SEWER)}
    assert by_pipe[(p["P2"], w["W1"])]["clearance"] < 0 and by_pipe[(p["P2"], w["W1"])]["above"] == "other"
    assert by_pipe[(p["P3"], w["W2"])]["above"] == "pipe"
    assert "clearance" not in by_pipe[(p["P4"], w["W7"])] and "otherInvert" not in by_pipe[(p["P4"], w["W7"])]
    least_in, least = sewer(clearance={**WATER_ASK, "vertical": 0.5, "above": "other"})
    kinds = {(f["uid"], f["kind"]) for f in least["findings"]}
    assert {(p["P1"], "closeCrossing"), (p["P2"], "clash"), (p["P3"], "wrongSide")} <= kinds, kinds
    side_in, side = sewer(clearance={**WATER_ASK, "horizontal": 3.0})
    assert [(n["pipe"], n["other"]) for n in side["near"]] == [(p["P3"], w["W5"])], side.get("near")
    storm_in, storm_ = storm(clearance={**WATER_ASK, "horizontal": 3.0})
    assert len(storm_["crossings"]) == 1 and len(storm_["near"]) == 1, (storm_.get("crossings"), storm_.get("near"))
    thin_in, thin = sewer(clearance={"network": "su", "inverts": {"from": "heights"}})
    fields_in, fields = sewer(clearance={**WATER_ASK, "inverts": {"from": "fields", "start": "Akar baş",
                                                                  "end": "Akar son"}})
    assert all("otherInvert" not in c for c in fields["crossings"])
    bad = "invalid_pipe_check"
    return [
        ok("açıklık: içme suyu boruları atıksuyu kesiyor; düşey açıklık ve üstteki boru", base_in, base),
        ok("açıklık: en az düşey açıklık ve suyun üstte olması", least_in, least),
        ok("açıklık: en az yatay açıklık; kesişmeden yakın geçen boru", side_in, side),
        ok("açıklık: yağmursuyunun yaylı borusunda kesişim ve yanından geçiş", storm_in, storm_),
        ok("açıklık: su borularının çapı sorulmadan; kalınlıksız sayılır", thin_in, thin),
        ok("açıklık: su borularında akar özniteliği yok; köşe kotları yerine geçmez", fields_in, fields),
        refused("açıklık: denetlenen ağın kendisi", sewer_input(clearance={**WATER_ASK, "network": "atiksu"}), bad,
                "Açıklık başka bir ağın borularına ölçülür; “Atıksu” denetlenen ağın kendisi.", "clearance.network"),
        refused("açıklık: sıfır en az düşey açıklık", sewer_input(clearance={**WATER_ASK, "vertical": 0.0}), bad,
                "En az düşey açıklık 0'dan büyük bir sayı olmalı; yalnız çakışmalar aranacaksa boş bırakın.",
                "clearance.vertical"),
        refused("açıklık: eksi en az yatay açıklık", sewer_input(clearance={**WATER_ASK, "horizontal": -1.0}), bad,
                "En az yatay açıklık 0'dan büyük bir sayı olmalı; yatay açıklık denetlenmeyecekse boş bırakın.",
                "clearance.horizontal"),
        refused("açıklık: boş akar kotu özniteliği", sewer_input(clearance={
            **WATER_ASK, "inverts": {"from": "fields", "start": " ", "end": "x"}}), bad,
                "Baş akar kotunun özniteliği boş; özniteliğin adını yazın.", "clearance.inverts.start"),
        refused("açıklık: katalogda sıfır çap", sewer_input(clearance={
            **WATER_ASK, "diameter": {"field": "Çap", "unit": "mm", "catalog": [0.0]}}), bad,
                "Katalogdaki çaplar 0'dan büyük olmalı.", "clearance.diameter.catalog[0]"),
        refused("açıklık: olmayan ağ", sewer_input(clearance={**WATER_ASK, "network": "gaz"}), "unknown_network",
                "“gaz” kimlikli ya da adlı ağ projede yok; Ağlar'da tanımlı bir ağı seçin.", "clearance.network"),
        refused("açıklık: yol ağı", sewer_input(clearance={**WATER_ASK, "network": "yollar"}), "not_a_pipe_network",
                "“Yollar” bir yol ağı; boru hattı denetimi altyapı ağlarını denetler.", "clearance.network"),
        refused("açıklık: önce denetlenen ağın sorusu", sewer_input(minCover=0.0, clearance={**WATER_ASK,
                                                                                         "vertical": 0.0}), bad,
                "En az örtü 0'dan büyük bir sayı olmalı; denetlenmeyecekse boş bırakın.", "minCover"),
    ]


def wall_cases():
    """Et kalınlığı (docs/adr/3960): the cover over the top outside and the clearances outside to outside, in the walls'
    drawing."""
    walled_d = {**SEWER_D, "wall": "Et"}
    cover_in, cover = sewer(walls=True, diameter=walled_d)
    plain_in, plain = sewer(walls=True)
    assert cover["coverTo"] == "outside" and plain["coverTo"] == "crown"
    p = {name: uid(240 + k + 1) for k, (name, *_rest) in enumerate(SEWER)}
    rows = {r["uid"]: r for r in cover["pipes"]}
    plain_rows = {r["uid"]: r for r in plain["pipes"]}
    # P1's least cover is less by its wall (20 mm) than over its crown; P7 has no wall: said, measured over its crown.
    assert abs(plain_rows[p["P1"]]["minCover"] - rows[p["P1"]]["minCover"] - 0.02) < 1e-9
    assert (p["P7"], "noWall") in {(f["uid"], f["kind"]) for f in cover["findings"]}
    water_walled = {**WATER_ASK, "diameter": {**WATER_ASK["diameter"], "wall": "Et"}}
    clear_in, clear = sewer(walls=True, diameter=walled_d, clearance={**water_walled, "vertical": 0.5, "horizontal": 3.0})
    base_in, base = sewer(walls=True, clearance={**WATER_ASK, "vertical": 0.5, "horizontal": 3.0})
    by = lambda r: {(c["pipe"], c["other"]): c.get("clearance") for c in r["crossings"]}
    for k, v in by(clear).items():
        if v is not None:
            assert by(base)[k] - v > 0.004, (k, by(base)[k], v)
    setup_w = setup(water=True, walls=True)
    bad = "invalid_pipe_check"
    return [
        ok("et kalınlığı: örtü borunun dış tepesinden; et kalınlığı olmayan boru söylenir", cover_in, cover, setup_=setup_w),
        ok("et kalınlığı sorulmadan: örtü iç tepeden", plain_in, plain, setup_=setup_w),
        ok("et kalınlığı: açıklıklar dıştan dışa", clear_in, clear, setup_=setup_w),
        refused("et kalınlığı: boş öznitelik", sewer_input(diameter={**SEWER_D, "wall": " "}), bad,
                "Et kalınlığının özniteliği boş; özniteliğin adını yazın.", "diameter.wall", setup_=setup_w),
        refused("et kalınlığı: diğer ağın boş özniteliği", sewer_input(clearance={
            **WATER_ASK, "diameter": {"field": "Çap", "unit": "mm", "wall": ""}}), bad,
                "Et kalınlığının özniteliği boş; özniteliğin adını yazın.", "clearance.diameter.wall", setup_=setup_w),
    ]


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.pipes.check", "commandVersion": 1,
           "title": "Boru hattı denetimi",
           "note": "ADR 1640. Bağımsız başvuru scripts/fixtures/pipes_check_cases.py: zemin kapalı biçimli kafes "
                   "(her kare tek düzlem), örtü köşelerde ve kafes çizgisi kesişimlerinde, eğim, birim ve bağlantı "
                   "kuralları elle; KentOS kodu yok. Sınırlar durumların kendi değerleridir, yönetmelik değeri değil. "
                   "ADR 2360: içme suyu ağına açıklıklar, kesişim ve en yakın yer mpmath ile kapalı biçimde. ADR 3960: "
                   "et kalınlığıyla örtü dış tepeden, açıklıklar dıştan dışa (kendi çiziminde).",
           "setup": setup(water=True), "cases": cases() + wall_cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok_ = OUT.exists() and OUT.read_text("utf-8") == text
        print("Boru hattı denetimi ortak durumları güncel." if ok_ else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok_ else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
