#!/usr/bin/env python3
"""Independent reference of the snap additions (docs/adr/0163).

Writes fixtures/snap/v1/cases.json from the ADR's rules alone, with Python's
standard library (and mpmath for the arcs), no KentOS code. The geometry
core (`Store::snap_ex`, crates/shared/geometry-core/tests/all/snap.rs) and the
web through its WASM (apps/web/src/viewport/snapExtras.test.ts) must give
the same: the kind and the object exactly, the point within 1e-9 m (a grid
node, a vertex and a centroid of whole numbers bit for bit).

The rules, as the ADR gives them:

- A candidate counts when it lies within the aperture `tol` of the cursor;
  the one of least distance × weight wins: endpoint and node 1, crossing
  1.02, centre and centroid 1.08, midpoint 1.15, extension and parallel 2;
  nearest only when nothing else is; a grid node only when not even that.
- Ağırlık merkezi: a closed area's centroid, holes subtracted, a
  multi-part area's parts weighted by their areas, a bulged edge's circular
  segment exactly.
- Karelaj: the nearest node of the grid of spacings gx, gy, wherever the
  cursor is: round(v / g) · g with JavaScript's rounding (halves up), the
  decimal node's nearest double.
- Uzantı: on an acquired end's line beyond the end (the projection, when it
  lies more than the aperture beyond: nearer, the end itself is the snap),
  on the rest of an arc's circle (as far along it); where an extension
  crosses an edge near the cursor or another extension that far beyond the
  ends, the crossing (weighed as a crossing).
- Paralel: from the last point, on the line through it along an acquired
  direction (the projection).
- A layer's objects take only its own kinds; a crossing counts when either
  object's layer takes crossings.
- The object being drawn is snapped to as one more object; its id is −1, as
  an extension's, a parallel's and a grid node's.

The acquisitions: `extensionsAt` gives, for every edge of the object that
ends at the point, the line beyond the end (a straight edge) or the rest of
the circle (an arc), from the end rested on away from the arc (at the arc's
start it goes the other way round); `directionAt` the direction of the
nearest straight edge within the aperture, from its first point to its
second.

Along an extension (step 3, the snap's tag and a typed distance):
`extensionAlong` is how far a point lies from the end, a line's distance
beyond it or an arc's length around the rest of the circle, when the point
lies on the extension within 1 µm (none behind the end, off the line, or on
the arc itself); `extensionAt` is the point a distance along (none before
the end or past an arc's remainder).
"""

import argparse
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

import mpmath as mp

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "snap" / "v1" / "cases.json"
mp.mp.dps = 50

KINDS = ["endpoint", "midpoint", "center", "node", "quadrant", "intersection", "perpendicular", "tangent", "nearest",
         "centroid", "extension", "parallel", "grid"]
WEIGHT = {"endpoint": 1, "node": 1, "intersection": F(102, 100), "center": F(108, 100), "centroid": F(108, 100),
          "midpoint": F(115, 100), "extension": 2, "parallel": 2}
E0, N0 = 487000, 4420000


def mask(kinds):
    return sum(1 << KINDS.index(k) for k in kinds)


def P(x, y):
    return {"x": float(x), "y": float(y)}


def layer(lid, kinds=None):
    row = {"id": lid, "visible": True, "locked": False, "pickInterior": True}
    if kinds is not None:
        row["snapKinds"] = mask(kinds)
    return row


def line(i, a, b, lid="a"):
    return {"kind": "line", "id": i, "layerId": lid, "attrs": {}, "a": P(*a), "b": P(*b)}


def polygon(i, pts, lid="a", holes=(), parts=(), bulges=None):
    e = {"kind": "polygon", "id": i, "layerId": lid, "attrs": {}, "pts": [P(*p) for p in pts]}
    if bulges is not None:
        e["bulges"] = bulges
    if holes:
        e["holes"] = [{"pts": [P(*p) for p in h]} for h in holes]
    if parts:
        e["parts"] = [{"pts": [P(*p) for p in q]} for q in parts]
    return e


def dist2(a, b):
    return (a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2


# ── Exact pieces ──────────────────────────────────────────────────────────────


def ring_moments(pts):
    """A straight ring's signed area and first moments."""
    a = mx = my = F(0)
    n = len(pts)
    for i in range(n):
        (x0, y0), (x1, y1) = pts[i], pts[(i + 1) % n]
        c = x0 * y1 - x1 * y0
        a += c / 2
        mx += (x0 + x1) * c / 6
        my += (y0 + y1) * c / 6
    return a, mx, my


def centroid(rings_per_part):
    """The centroid of parts, each an outer ring and its holes (straight), areas taken positive."""
    total = sx = sy = F(0)
    for outer, holes in rings_per_part:
        a, mx, my = ring_moments(outer)
        s = 1 if a > 0 else -1
        area, cx, cy = a * s, mx * s, my * s
        for h in holes:
            ha, hx, hy = ring_moments(h)
            t = 1 if ha > 0 else -1
            area -= ha * t
            cx -= hx * t
            cy -= hy * t
        total += area
        sx += cx
        sy += cy
    return (sx / total, sy / total)


def js_round(v):
    """JavaScript's Math.round on an exact value: halves go up."""
    return math.floor(v + F(1, 2))


def grid_node(v, g):
    return js_round(F(v) / F(g)) * F(g)


# ── Cases ─────────────────────────────────────────────────────────────────────


def snap_case(name, entities, p, tol, kinds, expect, layers=None, frm=None, extras=None):
    return {
        "name": name,
        "entities": entities,
        "layers": layers or [layer("a"), layer("b")],
        "p": [float(p[0]), float(p[1])],
        "tol": float(tol),
        "kinds": kinds,
        "from": None if frm is None else [float(frm[0]), float(frm[1])],
        "extras": extras or {},
        "expect": expect,
    }


def hit(kind, point, i):
    return {"kind": kind, "point": [float(point[0]), float(point[1])], "id": i}


def centroid_cases(de, dn, suffix):
    m = lambda x, y: (F(x) + de, F(y) + dn)
    out = []
    sq = [m(0, 0), m(10, 0), m(10, 10), m(0, 10)]
    c = centroid([(sq, [])])
    out.append(snap_case("kare: ağırlık merkezi" + suffix, [polygon(1, sq)], (c[0] + F(2, 10), c[1] - F(1, 10)), 1, ["centroid"], hit("centroid", c, 1)))
    ell = [m(0, 0), m(10, 0), m(10, 4), m(4, 4), m(4, 10), m(0, 10)]
    c = centroid([(ell, [])])
    out.append(snap_case("L biçimli alan" + suffix, [polygon(1, ell)], (c[0] + F(3, 10), c[1]), 1, ["centroid"], hit("centroid", c, 1)))
    hole = [m(6, 6), m(6, 9), m(9, 9), m(9, 6)]
    c = centroid([(sq, [hole])])
    out.append(snap_case("delikli alan: delik çıkarılır" + suffix, [polygon(1, sq, holes=[hole])], (c[0], c[1] + F(1, 4)), 1, ["centroid"], hit("centroid", c, 1)))
    a = [m(0, 0), m(4, 0), m(4, 4), m(0, 4)]
    b = [m(10, 0), m(12, 0), m(12, 2), m(10, 2)]
    c = centroid([(a, []), (b, [])])
    out.append(snap_case("iki parçalı alan: parçalar alanlarıyla" + suffix, [polygon(1, a, parts=[b])], (c[0] - F(1, 5), c[1]), 1, ["centroid"], hit("centroid", c, 1)))
    small = [m(0, 0), m(2, 0), m(2, 2), m(0, 2)]
    out.append(snap_case("ağırlık merkezi uçtan önce (uzaklık × ağırlık)" + suffix, [polygon(1, small)], (m(0, 0)[0] + F(9, 10), m(0, 0)[1] + F(9, 10)), 2, ["endpoint", "centroid"], hit("centroid", m(1, 1), 1)))
    out.append(snap_case("açıklığın dışında ağırlık merkezi yok" + suffix, [polygon(1, sq)], (m(0, 0)[0] + 5, m(0, 0)[1] + F(7, 2)), 1, ["centroid"], None))
    # A half disc: (−5, 0) to (5, 0), then the arc back counter-clockwise over the top (bulge 1).
    half = [m(-5, 0), m(5, 0)]
    cy = mp.mpf(20) / (3 * mp.pi)
    target = (m(0, 0)[0], float(to_mp(m(0, 0)[1]) + cy))
    out.append(snap_case("yarım daire: yayın dilimi tam" + suffix, [polygon(1, half, bulges=[0, 1])], (target[0] + 0.25, target[1]), 1, ["centroid"], hit("centroid", target, 1)))
    return out


def to_mp(v):
    return mp.mpf(v.numerator) / v.denominator if isinstance(v, F) else mp.mpf(v)


def grid_cases():
    out = []
    p = (F("12.4"), F("7.6"))
    out.append(snap_case("karelaj 1 × 1 m: en yakın düğüm", [], p, 1, ["grid"], hit("grid", (grid_node(p[0], 1), grid_node(p[1], 1)), -1), extras={"grid": [1, 1]}))
    p = (F("487012.437"), F("4420007.61"))
    out.append(snap_case("karelaj 0,1 × 0,25 m, TM koordinatlarında: ondalık düğüm", [], p, 1, ["grid"], hit("grid", (grid_node(p[0], F("0.1")), grid_node(p[1], F("0.25"))), -1), extras={"grid": [0.1, 0.25]}))
    p = (F("0.31"), F("0.69"))
    out.append(snap_case("karelaj 0,1 m: düğüm ondalık sayının double'ı (3 × 0,1 değil)", [], p, 1, ["grid"], hit("grid", (grid_node(p[0], F("0.1")), grid_node(p[1], F("0.1"))), -1), extras={"grid": [0.1, 0.1]}))
    p = (F("12.4"), F("7.6"))
    out.append(snap_case("karelaj 5 × 2,5 m", [], p, 1, ["grid"], hit("grid", (grid_node(p[0], 5), grid_node(p[1], F("2.5"))), -1), extras={"grid": [5, 2.5]}))
    p = (F("-12.5"), F("7.5"))
    out.append(snap_case("karelaj: yarımlar yukarı (JavaScript'in yuvarlaması)", [], p, 1, ["grid"], hit("grid", (grid_node(p[0], 1), grid_node(p[1], 1)), -1), extras={"grid": [1, 1]}))
    # An object's own point first; the grid only when nothing else, nearest included.
    ln = [line(1, (F("10.3"), 0), (20, 0))]
    out.append(snap_case("karelaj: açıklıktaki uç önce", ln, (F("10.2"), F("0.1")), 1, ["endpoint", "grid"], hit("endpoint", (F("10.3"), 0), 1), extras={"grid": [1, 1]}))
    out.append(snap_case("karelaj: açıklıktaki en yakın önce", ln, (F("15.2"), F("0.3")), 1, ["nearest", "grid"], hit("nearest", (F("15.2"), 0), 1), extras={"grid": [1, 1]}))
    out.append(snap_case("karelaj: açıklıkta bir şey yoksa düğüm", ln, (F("15.2"), F("3.3")), 1, ["nearest", "grid"], hit("grid", (15, 3), -1), extras={"grid": [1, 1]}))
    out.append(snap_case("karelaj kapalıyken düğüm yok", [], (F("12.4"), F("7.6")), 1, ["endpoint"], None, extras={"grid": [1, 1]}))
    return out


def ext_line(end, d):
    return [0, float(end[0]), float(end[1]), float(d[0]), float(d[1])]


def extension_cases():
    out = []
    a = line(1, (0, 0), (10, 0))
    ext = {"extensions": [ext_line((10, 0), (1, 0))]}
    out.append(snap_case("uzantı: uçtan ötede izdüşüm", [a], (15, F("0.3")), 1, ["extension"], hit("extension", (15, 0), -1), extras=ext))
    out.append(snap_case("uzantı: ucun gerisinde yok", [a], (5, F("0.3")), 1, ["extension"], None, extras=ext))
    # Within the aperture of the end the end itself is the snap (resting on it again releases it).
    out.append(snap_case("uzantı: ucun kenet yarıçapında uç kendisidir", [a], (F("10.4"), 0), 1, ["endpoint", "extension"], hit("endpoint", (10, 0), 1), extras=ext))
    out.append(snap_case("uzantı: ucun kenet yarıçapında uzantı yok", [a], (F("10.4"), F("0.1")), 1, ["extension"], None, extras=ext))
    # Just past the aperture: the extension; the end lies out of reach.
    out.append(snap_case("uzantı: kenet yarıçapının ötesinde", [a], (F("11.2"), F("0.1")), 1, ["endpoint", "extension"], hit("extension", (F("11.2"), 0), -1), extras=ext))
    # A crossing with an edge near the cursor wins over the projection when the cursor is near both.
    b = line(2, (14, -5), (14, 5))
    p = (F("14.05"), F("0.3"))
    proj = ((p[0], 0), F("0.3") ** 2 * WEIGHT["extension"] ** 2)
    cross = ((14, 0), dist2(p, (14, 0)) * WEIGHT["intersection"] ** 2)
    best = min([proj, cross], key=lambda c: c[1])
    out.append(snap_case("uzantının bir kenarla kesişimi", [a, b], p, 1, ["extension"], hit("extension", best[0], -1), extras=ext))
    # Two extensions crossing: the cursor on the diagonal near their crossing.
    c = line(3, (20, -10), (20, -2))
    two = {"extensions": [ext_line((10, 0), (1, 0)), ext_line((20, -2), (0, 1))]}
    p = (F("19.9"), F("0.1"))
    cands = [((p[0], 0), F("0.1") ** 2 * 4), ((20, p[1]), F("0.1") ** 2 * 4), ((20, 0), dist2(p, (20, 0)) * WEIGHT["intersection"] ** 2)]
    best = min(cands, key=lambda c: c[1])
    out.append(snap_case("iki uzantının kesişimi", [a, c], p, 1, ["extension"], hit("extension", best[0], -1), extras=two))
    # An endpoint of another object near the extension comes first.
    d = line(4, (F("15.2"), F("0.2")), (16, 5))
    p = (15, F("0.3"))
    cands = [((15, 0), F("0.3") ** 2 * 4), ((F("15.2"), F("0.2")), dist2(p, (F("15.2"), F("0.2"))))]
    best = min(cands, key=lambda c: c[1])
    kind = "endpoint" if best[0] != (15, 0) else "extension"
    out.append(snap_case("uzantının yanındaki uç önce", [a, d], p, 1, ["endpoint", "extension"], hit(kind, best[0], 4 if kind == "endpoint" else -1), extras=ext))
    # An arc's extension: the rest of its circle.
    arc = {"kind": "arc", "id": 1, "layerId": "a", "attrs": {}, "c": P(0, 0), "r": 10.0, "a0": 0.0, "a1": float(mp.pi / 2)}
    rest = [1, 0.0, 0.0, 10.0, float(mp.pi / 2), float(mp.mpf(3) * mp.pi / 2)]
    p = (mp.mpf("-7.2"), mp.mpf("7.0"))
    r = mp.sqrt(p[0] ** 2 + p[1] ** 2)
    q = (float(p[0] * 10 / r), float(p[1] * 10 / r))
    out.append(snap_case("yayın uzantısı: çemberinin kalanı", [arc], (F("-7.2"), F(7)), 1, ["extension"], hit("extension", q, -1), extras={"extensions": [rest]}))
    return out


def parallel_cases():
    out = []
    frm = (0, 0)
    u = (F(3, 5), F(4, 5))
    p = (F("6.1"), F("7.9"))
    t = p[0] * u[0] + p[1] * u[1]
    q = (u[0] * t, u[1] * t)
    out.append(snap_case("paralel: son noktadan geçen doğruya izdüşüm", [], p, 1, ["parallel"], hit("parallel", q, -1), frm=frm, extras={"parallels": [[0.6, 0.8]]}))
    out.append(snap_case("paralel: son nokta yoksa yok", [], p, 1, ["parallel"], None, extras={"parallels": [[0.6, 0.8]]}))
    e = line(1, (F("6.2"), F("8.1")), (12, 20))
    cands = [(q, dist2(p, q) * 4), ((F("6.2"), F("8.1")), dist2(p, (F("6.2"), F("8.1"))))]
    best = min(cands, key=lambda c: c[1])
    kind = "endpoint" if best[0] == (F("6.2"), F("8.1")) else "parallel"
    out.append(snap_case("paralelin yanındaki uç", [e], p, 1, ["endpoint", "parallel"], hit(kind, best[0], 1 if kind == "endpoint" else -1), frm=frm, extras={"parallels": [[0.6, 0.8]]}))
    return out


def layer_cases():
    out = []
    a = line(1, (0, 0), (10, 0), "a")
    b = line(2, (F("10.4"), F("0.3")), (20, 5), "b")
    layers = [layer("a", ["midpoint", "nearest"]), layer("b")]
    out.append(snap_case("katmanın türleri: A'nın ucu alınmaz, B'ninki alınır", [a, b], (F("10.1"), F("0.1")), 1, ["endpoint"], hit("endpoint", (F("10.4"), F("0.3")), 2), layers=layers))
    out.append(snap_case("katmanın türleri: A'nın ortası alınır", [a, b], (5, F("0.2")), 1, ["endpoint", "midpoint"], hit("midpoint", (5, 0), 1), layers=layers))
    c = line(3, (5, -5), (5, 5), "b")
    both_off = [layer("a", ["endpoint"]), layer("b", ["endpoint"])]
    out.append(snap_case("kesişim: iki katman da almıyorsa yok", [a, c], (F("5.1"), F("0.1")), 1, ["intersection"], None, layers=both_off))
    one_on = [layer("a", ["endpoint"]), layer("b", ["intersection"])]
    out.append(snap_case("kesişim: birinin katmanı alıyorsa var", [a, c], (F("5.1"), F("0.1")), 1, ["intersection"], hit("intersection", (5, 0), 1), layers=one_on))
    return out


def draft_cases():
    out = []
    path = {"pts": [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]]}
    out.append(snap_case("çizilmekte olan: köşesi", [], (F("10.2"), F("0.1")), 1, ["endpoint"], hit("endpoint", (10, 0), -1), extras={"draft": path}))
    out.append(snap_case("çizilmekte olan: kenarının ortası", [], (5, F("0.2")), 1, ["midpoint"], hit("midpoint", (5, 0), -1), extras={"draft": path}))
    v = line(1, (5, -5), (5, 5))
    out.append(snap_case("çizilmekte olan: çizimdeki çizgiyle kesişimi", [v], (F("5.1"), F("0.1")), 1, ["intersection"], hit("intersection", (5, 0), 1), extras={"draft": path}))
    out.append(snap_case("çizilmekte olan yoksa köşesi yok", [], (F("10.2"), F("0.1")), 1, ["endpoint"], None))
    return out


def acquisitions():
    ext = []
    ext.append({"name": "çizginin ucu", "entities": [line(1, (0, 0), (10, 0))], "id": 1, "at": [10.0, 0.0], "expect": [0, 10.0, 0.0, 1.0, 0.0]})
    ext.append({"name": "çoklu çizginin köşesi: iki kenarın uzantısı", "entities": [{"kind": "polyline", "id": 1, "layerId": "a", "attrs": {}, "pts": [P(0, 0), P(10, 0), P(10, 10)]}], "id": 1, "at": [10.0, 0.0],
                "expect": [0, 10.0, 0.0, 1.0, 0.0, 0, 10.0, 0.0, 0.0, -1.0]})
    ext.append({"name": "kapalı alanın köşesi", "entities": [polygon(1, [(0, 0), (10, 0), (10, 10), (0, 10)])], "id": 1, "at": [0.0, 0.0],
                "expect": [0, 0.0, 0.0, -1.0, 0.0, 0, 0.0, 0.0, 0.0, -1.0]})
    ext.append({"name": "yayın ucu: çemberin kalanı", "entities": [{"kind": "arc", "id": 1, "layerId": "a", "attrs": {}, "c": P(0, 0), "r": 10.0, "a0": 0.0, "a1": float(mp.pi / 2)}], "id": 1, "at": [0.0, 10.0],
                "expect": [1, 0.0, 0.0, 10.0, float(mp.pi / 2), float(3 * mp.pi / 2)]})
    ext.append({"name": "yayın başı: çemberin kalanı öbür yönde", "entities": [{"kind": "arc", "id": 1, "layerId": "a", "attrs": {}, "c": P(0, 0), "r": 10.0, "a0": 0.0, "a1": float(mp.pi / 2)}], "id": 1, "at": [10.0, 0.0],
                "expect": [1, 0.0, 0.0, 10.0, 0.0, float(-3 * mp.pi / 2)]})
    ext.append({"name": "uç olmayan nokta", "entities": [line(1, (0, 0), (10, 0))], "id": 1, "at": [5.0, 0.0], "expect": []})
    dirs = []
    l = mp.sqrt(125)
    dirs.append({"name": "eğik çizginin doğrultusu", "entities": [line(1, (0, 0), (10, 5))], "p": [5.0, 2.6], "tol": 1.0, "expect": [float(10 / l), float(5 / l)]})
    dirs.append({"name": "yakında kenar yok", "entities": [line(1, (0, 0), (10, 5))], "p": [5.0, 9.0], "tol": 1.0, "expect": None})
    dirs.append({"name": "iki kenardan yakını", "entities": [line(1, (0, 0), (10, 0)), line(2, (0, 1), (10, 3))], "p": [5.0, 0.3], "tol": 2.0, "expect": [1.0, 0.0]})
    return ext, dirs


def along_cases():
    """How far along an extension a point lies, and the point a distance along (mpmath, 50 digits)."""
    pi = mp.pi
    fwd = [1, 0.0, 0.0, 10.0, float(pi / 2), float(3 * pi / 2)]
    back = [1, 0.0, 0.0, 10.0, 0.0, float(-3 * pi / 2)]
    line_x = [0, 10.0, 0.0, 1.0, 0.0]
    slant = [0, 0.0, 0.0, 0.6, 0.8]
    along = [
        {"name": "çizginin uzantısında 5 m", "ext": line_x, "p": [15.0, 0.0], "expect": 5.0},
        {"name": "uçta 0", "ext": line_x, "p": [10.0, 0.0], "expect": 0.0},
        {"name": "ucun gerisinde değil", "ext": line_x, "p": [5.0, 0.0], "expect": None},
        {"name": "doğrunun yanında değil", "ext": line_x, "p": [15.0, 0.5], "expect": None},
        {"name": "eğik uzantıda 5 m", "ext": slant, "p": [3.0, 4.0], "expect": 5.0},
        {"name": "yayın uzantısında çeyrek çember", "ext": fwd, "p": [-10.0, 0.0], "expect": float(10 * pi / 2)},
        {"name": "yayın uzantısının sonu: yayın başı", "ext": fwd, "p": [10.0, 0.0], "expect": float(10 * 3 * pi / 2)},
        {"name": "yayın kendisinde değil", "ext": fwd, "p": [float(10 * mp.cos(pi / 4)), float(10 * mp.sin(pi / 4))], "expect": None},
        {"name": "geriye dönen uzantıda çeyrek çember", "ext": back, "p": [0.0, -10.0], "expect": float(10 * pi / 2)},
    ]
    at = [
        {"name": "çizginin uzantısında 6 m", "ext": line_x, "d": 6.0, "expect": [16.0, 0.0]},
        {"name": "ucun gerisi yok", "ext": line_x, "d": -1.0, "expect": None},
        {"name": "eğik uzantıda 5 m", "ext": slant, "d": 5.0, "expect": [3.0, 4.0]},
        {"name": "yayın uzantısında çeyrek çember", "ext": fwd, "d": float(10 * pi / 2), "expect": [-10.0, 0.0]},
        {"name": "yayın kalanından öte yok", "ext": fwd, "d": float(16 * pi), "expect": None},
        {"name": "geriye dönen uzantıda çeyrek çember", "ext": back, "d": float(10 * pi / 2), "expect": [0.0, -10.0]},
    ]
    # The arcs' points from mpmath at 50 digits, then to doubles.
    for c in at:
        if c["expect"] is not None and c["ext"][0] == 1:
            _, cx, cy, r, a0, sweep = c["ext"]
            a = mp.mpf(a0) + mp.sign(sweep) * mp.mpf(c["d"]) / r
            c["expect"] = [float(cx + r * mp.cos(a)), float(cy + r * mp.sin(a))]
    return along, at


def build():
    snaps = centroid_cases(0, 0, "") + centroid_cases(E0, N0, ", TM koordinatlarında") + grid_cases() + extension_cases() + parallel_cases() + layer_cases() + draft_cases()
    ext, dirs = acquisitions()
    along, at = along_cases()
    return {
        "format": "kentos.snap",
        "version": 1,
        "source": "scripts/fixtures/snap_cases.py (docs/adr/0163 §1–§4, §7)",
        "kinds": KINDS,
        "snap": snaps,
        "extensionsAt": ext,
        "directionAt": dirs,
        "extensionAlong": along,
        "extensionAt": at,
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--check", action="store_true", help="compare with the written file instead of writing it")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if OUT.read_text() != text:
            print(f"{OUT} güncel değil: python3 scripts/fixtures/snap_cases.py", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT}: güncel")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    d = json.loads(text)
    print(f"{OUT}: {len(d['snap'])} kenet, {len(d['extensionsAt'])} uzantı alma, {len(d['directionAt'])} doğrultu alma, {len(d['extensionAlong'])} + {len(d['extensionAt'])} uzantıda yer")


if __name__ == "__main__":
    main()
