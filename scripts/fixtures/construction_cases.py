"""The shared cases of cad.constructions.create (docs/adr/0622 §9): Paralel çizgi, Dik in, Dik çık, İki daireye teğet,
Dördüncü köşe, Menzil halkaları, Ara nokta, Kesişim noktası, Yardımcı çizgi's Açıortay and Paralel.

    python3 scripts/fixtures/construction_cases.py           # writes the file
    python3 scripts/fixtures/construction_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.constructions.create.json. The rules are written here from the ADRs, without KentOS
code; the tangents and the rings are drawing_extras_cases.py's (ADR 0197). In the order of their arithmetic:

- Paralel çizgi: repeated axis points dropped; each leg offset by d along its left normal (−dy/l, dx/l)·d (the left
  side d = left, the right side d = −right); joined at the meeting of the neighbouring offset legs (a miter);
  an open side starts at its first leg's start and ends at its last leg's end; the corridor of an open axis is the
  left side out and the right side back, turned counter-clockwise; of a closed one the larger ring the outline
  (counter-clockwise), the other its hole (clockwise); a side at 0 is none, the corridor's then the axis itself;
- Dik in: the foot of p on a→b is a + u·absis with u = (b − a)/|b − a| and absis = (p − a)·u; on a circle,
  c + (p − c)/|p − c|·r; the line runs from p to its foot;
- Dik çık: foot a + u·along, end a + u·along + (u.y, −u.x)·offset (right plus);
- Dördüncü köşe: d = a + c − b; the parallelogram a, b, c, d;
- Menzil halkaları: circles k·s and rays to M + n·s·(sin θ, cos θ), θ = 2πj/m;
- Ara nokta: a + (b − a)·t, t = k/n, distance/|b − a| or the ratio;
- Kesişim noktası: two circles' meeting points, the nearer to `near` or else the one right of a→b; two bearings
  (from north, clockwise) ahead of both points; two lines' meeting;
- Açıortay: the unit sum of the arms' units (square to opposite arms); Paralel: the edge's line moved `distance`
  to the side the point is on, through the foot of a, or through the point without a distance.

Steps with transcendental numbers carry `within` (1e-8 m).
"""
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from drawing_extras_cases import chosen, rings, tangents  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
POLYGON = json.loads((OUT / "cad.polygon.create.json").read_text(encoding="utf-8"))
X, Y = 487000, 4420000
SETUP = {**{k: v for k, v in POLYGON["setup"].items() if k != "entities"}, "entities": []}
NEXT = 1
UNCHANGED = {"ids": [], "dirty": False, "revision": "same"}
POLY_ERR = {}
for case in POLYGON["cases"]:
    for step in case["steps"]:
        e = step.get("result", {}).get("error")
        if e:
            POLY_ERR.setdefault(e["code"], e)


def num(v):
    v = float(v)
    return int(v) if v.is_integer() else v


def W(p):
    return {"x": num(p[0]), "y": num(p[1])}


def at(x, y):
    return (float(X + x), float(Y + y))


def hypot(a, b):
    """ECMAScript's Math.hypot as the core writes it."""
    m = max(abs(a), abs(b))
    if m == 0.0:
        return 0.0
    total, comp = 0.0, 0.0
    for x in (a, b):
        n = abs(x) / m
        summand = n * n - comp
        pre = total + summand
        comp = (pre - total) - summand
        total = pre
    return math.sqrt(total) * m


# ── The constructions ────────────────────────────────────────────────────────────────────────────────────────────


def line_meet(a, b, c, d):
    """The meeting of the lines a-b and c-d by Cramer's rule; none when parallel."""
    r = (b[0] - a[0], b[1] - a[1])
    s = (d[0] - c[0], d[1] - c[1])
    den = r[0] * s[1] - r[1] * s[0]
    if abs(den) < 1e-12:
        return None
    t = ((c[0] - a[0]) * s[1] - (c[1] - a[1]) * s[0]) / den
    return (a[0] + r[0] * t, a[1] + r[1] * t)


def offset_path(pts, d, closed):
    n = len(pts)
    segs = []
    for i in range(n if closed else n - 1):
        a, b = pts[i], pts[(i + 1) % n]
        l = hypot(b[0] - a[0], b[1] - a[1])
        nx, ny = (-(b[1] - a[1]) / l) * d, ((b[0] - a[0]) / l) * d
        segs.append(((a[0] + nx, a[1] + ny), (b[0] + nx, b[1] + ny)))
    out = []

    def join(s1, s2):
        h = line_meet(s1[0], s1[1], s2[0], s2[1])
        out.append(s2[0] if h is None else h)

    m = len(segs)
    if closed:
        for i in range(m):
            join(segs[(i - 1) % m], segs[i])
    else:
        out.append(segs[0][0])
        for i in range(1, m):
            join(segs[i - 1], segs[i])
        out.append(segs[-1][1])
    return out


def shoelace(r):
    return sum(r[i][0] * r[(i + 1) % len(r)][1] - r[(i + 1) % len(r)][0] * r[i][1] for i in range(len(r))) / 2.0


def path_entity(pts, closed):
    e = {"kind": "polygon" if closed else "polyline", "pts": [W(p) for p in pts]}
    return e


def parallel(axis, left, right, closed=False, with_axis=False, area=False):
    out = []
    if area:
        l = offset_path(axis, left, closed) if left > 0 else list(axis)
        r = offset_path(axis, -right, closed) if right > 0 else list(axis)
        if not closed:
            ring = l + list(reversed(r))
            if shoelace([(p[0] - X, p[1] - Y) for p in ring]) < 0:
                ring.reverse()
            out.append({"kind": "polygon", "pts": [W(p) for p in ring]})
        else:
            rel = lambda ring: [(p[0] - X, p[1] - Y) for p in ring]  # noqa: E731
            outer, hole = (l, r) if abs(shoelace(rel(l))) >= abs(shoelace(rel(r))) else (r, l)
            if shoelace(rel(outer)) < 0:
                outer = list(reversed(outer))
            if shoelace(rel(hole)) > 0:
                hole = list(reversed(hole))
            out.append({"kind": "polygon", "pts": [W(p) for p in outer], "holes": [{"pts": [W(p) for p in hole]}]})
    else:
        if left > 0:
            out.append(path_entity(offset_path(axis, left, closed), closed))
        if right > 0:
            out.append(path_entity(offset_path(axis, -right, closed), closed))
    if with_axis:
        out.append(path_entity(axis, closed))
    return out


def unit(a, b):
    l = hypot(b[0] - a[0], b[1] - a[1])
    return ((b[0] - a[0]) / l, (b[1] - a[1]) / l)


def side_point(a, b, absis, ordinat):
    u = unit(a, b)
    return (a[0] + u[0] * absis + u[1] * ordinat, a[1] + u[1] * absis - u[0] * ordinat)


def foot(a, b, p):
    u = unit(a, b)
    v = (p[0] - a[0], p[1] - a[1])
    return side_point(a, b, v[0] * u[0] + v[1] * u[1], 0.0)


def radial(c, r, p):
    l = hypot(p[0] - c[0], p[1] - c[1])
    return (c[0] + ((p[0] - c[0]) / l) * r, c[1] + ((p[1] - c[1]) / l) * r)


def line(a, b):
    return {"kind": "line", "a": W(a), "b": W(b)}


def point(p):
    return {"kind": "point", "p": W(p)}


def circles_meet(a, da, b, db):
    d = hypot(b[0] - a[0], b[1] - a[1])
    if d > da + db or d < abs(da - db):
        return []
    x = (da * da - db * db + d * d) / (2 * d)
    h = math.sqrt(max(0.0, da * da - x * x))
    u = ((b[0] - a[0]) / d, (b[1] - a[1]) / d)
    m = (a[0] + u[0] * x, a[1] + u[1] * x)
    pts = [(m[0] + u[1] * h, m[1] - u[0] * h), (m[0] - u[1] * h, m[1] + u[0] * h)]  # right of a→b first
    return pts[:1] if h == 0 else pts


def bearings_meet(a, ta, b, tb):
    u, w = (math.sin(ta), math.cos(ta)), (math.sin(tb), math.cos(tb))
    p = line_meet(a, (a[0] + u[0], a[1] + u[1]), b, (b[0] + w[0], b[1] + w[1]))
    if p is None:
        return None
    ahead = lambda o, d: (p[0] - o[0]) * d[0] + (p[1] - o[1]) * d[1] > -1e-9  # noqa: E731
    return p if ahead(a, u) and ahead(b, w) else None


# ── The cases ────────────────────────────────────────────────────────────────────────────────────────────────────

OPERATIONS = {"parallel": "Paralel çizgi", "perpendicularIn": "Dik in", "perpendicularOut": "Dik çık",
              "tangentLine": "İki daireye teğet", "fourthCorner": "Dördüncü köşe", "rangeRings": "Menzil halkaları",
              "pointsBetween": "Ara nokta", "intersection": "Kesişim noktası", "bisector": "Ekle", "xlineOffset": "Ekle"}


def inp(construction, **extra):
    return {"layerId": "yapi", "construction": construction, **extra}


def written(name, construction, entities, within=None, extra=None, warnings=(), layer="yapi"):
    ids = list(range(NEXT, NEXT + len(entities)))
    want = {}
    for i, e in zip(ids, entities):
        want[str(i)] = {**e, "id": i, "layerId": layer, "attrs": dict((extra or {}).get("attrs", {})),
                        **{k: v for k, v in (extra or {}).items()
                           # The weight goes only to what is drawn with lines (docs/adr/0139).
                           if k == "color" or (k == "lineWeight" and e["kind"] != "point")}}
    step = {"op": "execute", "input": inp(construction, **(extra or {}))}
    if within:
        step["within"] = within
    step["result"] = {"status": "completed", "output": {"created": "$any", "ids": ids, "revision": "$current"},
                      "warnings": list(warnings)}
    step["expect"] = {"ids": ids, "entities": want, "canUndo": True, "dirty": True}
    undo = OPERATIONS[construction["kind"]]
    return {"name": name, "steps": [step, {"op": "undo", "returns": undo, "expect": {"ids": [], "canRedo": True}}]}


def refused(name, construction, code, message, path, non_finite=None, extra=None):
    step = {"op": "execute", "input": inp(construction, **(extra or {}))}
    if non_finite:
        step["nonFinite"] = non_finite
    step["result"] = {"status": "failed", "error": {"code": code, "message": message, "path": path}}
    step["expect"] = UNCHANGED
    return {"name": name, "steps": [step]}


def not_finite_point(whose, axis):
    name = "doğu (Y)" if axis == "x" else "kuzey (X)"
    return f"{whose} {name} değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin."


def not_finite_number(what):
    return f"{what} sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin."


def cases():
    L = [at(0, 0), at(20, 0), at(20, 10)]
    SQ = [at(0, 0), at(10, 0), at(10, 10), at(0, 10)]
    W3 = 1e-8
    out = [
        written("Paralel çizgi: L ekseni, sol 2 ve sağ 3", {"kind": "parallel", "axis": [W(p) for p in L], "left": 2, "right": 3},
                parallel(L, 2, 3)),
        written("Paralel çizgi: eksen de çizilir", {"kind": "parallel", "axis": [W(p) for p in L], "left": 2, "right": 0,
                                                     "withAxis": True}, parallel(L, 2, 0, with_axis=True)),
        written("Paralel çizgi: kapalı eksen", {"kind": "parallel", "axis": [W(p) for p in SQ], "closed": True, "left": 1,
                                                 "right": 2}, parallel(SQ, 1, 2, closed=True)),
        written("Paralel çizgi: açık eksenin koridoru alan", {"kind": "parallel", "axis": [W(p) for p in L], "left": 2,
                                                               "right": 3, "area": True}, parallel(L, 2, 3, area=True)),
        written("Paralel çizgi: kapalı eksenin koridoru delikli alan", {"kind": "parallel", "axis": [W(p) for p in SQ],
                                                                        "closed": True, "left": 1, "right": 2, "area": True},
                parallel(SQ, 1, 2, closed=True, area=True)),
        written("Paralel çizgi: yinelenen eksen noktası düşer", {"kind": "parallel", "axis": [W(p) for p in [L[0], L[1], L[1], L[2]]],
                                                                  "left": 2, "right": 3}, parallel(L, 2, 3)),
        written("Dik in: iki nokta, hattın uzantısına da", {"kind": "perpendicularIn",
                                                             "reference": {"kind": "line", "a": W(at(0, 0)), "b": W(at(10, 0))},
                                                             "points": [W(at(4, 7)), W(at(15, -3))]},
                [line(at(4, 7), foot(at(0, 0), at(10, 0), at(4, 7))), line(at(15, -3), foot(at(0, 0), at(10, 0), at(15, -3)))]),
        written("Dik in: eğik hat (3-4-5)", {"kind": "perpendicularIn",
                                            "reference": {"kind": "line", "a": W(at(0, 0)), "b": W(at(30, 40))},
                                            "points": [W(at(50, 0))]},
                [line(at(50, 0), foot(at(0, 0), at(30, 40), at(50, 0)))], within=W3),
        written("Dik in: daireye, merkezden geçerek", {"kind": "perpendicularIn",
                                                       "reference": {"kind": "circle", "c": W(at(0, 0)), "r": 5},
                                                       "points": [W(at(12, 0)), W(at(0, -9))]},
                [line(at(12, 0), radial(at(0, 0), 5, at(12, 0))), line(at(0, -9), radial(at(0, 0), 5, at(0, -9)))]),
        written("Dik çık: sağa artı, sola eksi", {"kind": "perpendicularOut", "a": W(at(0, 0)), "b": W(at(10, 0)),
                                                 "stations": [{"along": 5, "offset": 4}, {"along": 12, "offset": -3}]},
                [line(side_point(at(0, 0), at(10, 0), 5, 0), side_point(at(0, 0), at(10, 0), 5, 4)),
                 line(side_point(at(0, 0), at(10, 0), 12, 0), side_point(at(0, 0), at(10, 0), 12, -3))]),
    ]
    c1, c2 = at(0, 0), at(20, 0)
    t = chosen(tangents(c1, 3, c2, 5), at(0, 3), at(20, 5))
    out.append(written("İki daireye teğet: üstteki dış teğet", {"kind": "tangentLine", "first": {"c": W(c1), "r": 3},
                                                                "second": {"c": W(c2), "r": 5}, "nearFirst": W(at(0, 3)),
                                                                "nearSecond": W(at(20, 5))},
                       [line((t["a"]["x"], t["a"]["y"]), (t["b"]["x"], t["b"]["y"]))], within=W3))
    t = chosen(tangents(c1, 3, c2, 5), at(0, 3), at(20, -5))
    out.append(written("İki daireye teğet: iç teğet", {"kind": "tangentLine", "first": {"c": W(c1), "r": 3},
                                                       "second": {"c": W(c2), "r": 5}, "nearFirst": W(at(0, 3)),
                                                       "nearSecond": W(at(20, -5))},
                       [line((t["a"]["x"], t["a"]["y"]), (t["b"]["x"], t["b"]["y"]))], within=W3))
    a, b, c = at(0, 0), at(10, 0), at(14, 6)
    d = (a[0] + c[0] - b[0], a[1] + c[1] - b[1])
    out += [
        written("Dördüncü köşe: nokta", {"kind": "fourthCorner", "a": W(a), "b": W(b), "c": W(c)}, [point(d)]),
        written("Dördüncü köşe: paralelkenar alanı", {"kind": "fourthCorner", "a": W(a), "b": W(b), "c": W(c), "area": True},
                [{"kind": "polygon", "pts": [W(a), W(b), W(c), W(d)]}]),
    ]
    rr = rings(at(0, 0), 5, 3, 4)
    out.append(written("Menzil halkaları: üç halka, dört ışın", {"kind": "rangeRings", "centre": W(at(0, 0)), "spacing": 5,
                                                                  "count": 3, "rays": 4},
                       [{"kind": "circle", "c": W(at(0, 0)), "r": num(r)} for r in rr["radii"]]
                       + [line(at(0, 0), (e["x"], e["y"])) for e in rr["rays"]], within=W3))
    rr = rings(at(0, 0), 2.5, 2, 0)
    out.append(written("Menzil halkaları: ışınsız", {"kind": "rangeRings", "centre": W(at(0, 0)), "spacing": 2.5, "count": 2},
                       [{"kind": "circle", "c": W(at(0, 0)), "r": num(r)} for r in rr["radii"]]))
    a, b = at(0, 0), at(12, 0)
    lerp = lambda t: (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)  # noqa: E731
    l = hypot(b[0] - a[0], b[1] - a[1])
    out += [
        written("Ara nokta: dört eşit parça", {"kind": "pointsBetween", "a": W(a), "b": W(b), "parts": 4},
                [point(lerp(k / 4)) for k in range(1, 4)]),
        written("Ara nokta: uzaklıklarla, uzantıda da", {"kind": "pointsBetween", "a": W(a), "b": W(b), "distances": [3, 15]},
                [point(lerp(d / l)) for d in (3.0, 15.0)]),
        written("Ara nokta: oranlarla", {"kind": "pointsBetween", "a": W(a), "b": W(b), "ratios": [0.25, 0.5]},
                [point(lerp(t)) for t in (0.25, 0.5)]),
    ]
    A, B = at(0, 0), at(10, 0)
    meets = circles_meet(A, 6, B, 8)
    out += [
        written("Kesişim noktası: iki uzaklık, A'dan B'ye sağdaki", {"kind": "intersection", "meeting": {
            "method": "distances", "a": W(A), "da": 6, "b": W(B), "db": 8}}, [point(meets[0])], within=W3),
        written("Kesişim noktası: iki uzaklık, yakındaki", {"kind": "intersection", "meeting": {
            "method": "distances", "a": W(A), "da": 6, "b": W(B), "db": 8, "near": W(at(3, 6))}}, [point(meets[1])], within=W3),
        written("Kesişim noktası: iki doğrultu", {"kind": "intersection", "meeting": {
            "method": "bearings", "a": W(A), "ta": math.pi / 4, "b": W(B), "tb": -math.pi / 4}},
                [point(bearings_meet(A, math.pi / 4, B, -math.pi / 4))], within=W3),
        written("Kesişim noktası: iki doğru, uzantılarında", {"kind": "intersection", "meeting": {
            "method": "lines", "a": W(at(0, 0)), "b": W(at(4, 0)), "c": W(at(10, 5)), "d": W(at(10, 8))}},
                [point(line_meet(at(0, 0), at(4, 0), at(10, 5), at(10, 8)))]),
    ]
    o, p1, p2 = at(0, 0), at(10, 0), at(0, 10)
    s = 1 / math.sqrt(2)
    out += [
        written("Yardımcı çizgi, Açıortay: dik açının ortası", {"kind": "bisector", "vertex": W(o), "p1": W(p1), "p2": W(p2)},
                [{"kind": "xline", "p": W(o), "dir": {"x": s, "y": s}}], within=W3),
        written("Yardımcı çizgi, Açıortay: zıt kollara dik", {"kind": "bisector", "vertex": W(o), "p1": W(p1), "p2": W(at(-5, 0))},
                [{"kind": "xline", "p": W(o), "dir": {"x": 0, "y": 1}}]),
        written("Yardımcı çizgi, Paralel: uzaklıkla, noktanın yanına", {"kind": "xlineOffset", "a": W(at(0, 0)), "b": W(at(10, 0)),
                                                                       "side": W(at(3, -4)), "distance": 2},
                [{"kind": "xline", "p": W(at(0, -2)), "dir": {"x": 1, "y": 0}}]),
        written("Yardımcı çizgi, Paralel: noktadan geçerek", {"kind": "xlineOffset", "a": W(at(0, 0)), "b": W(at(10, 0)),
                                                              "side": W(at(3, 4))},
                [{"kind": "xline", "p": W(at(3, 4)), "dir": {"x": 1, "y": 0}}]),
        written("Renk, kalınlık ve öznitelik", {"kind": "fourthCorner", "a": W(a), "b": W(b), "c": W(at(14, 6))},
                [point((a[0] + 14 + X - b[0], a[1] + 6 + Y - b[1]))],
                extra={"color": "#E5484D", "lineWeight": 0.5, "attrs": {"Tür": "Yapım"}}),
        written("Gizli katmana uyarıyla", {"kind": "fourthCorner", "a": W(a), "b": W(b), "c": W(at(14, 6))},
                [point((a[0] + 14 + X - b[0], a[1] + 6 + Y - b[1]))],
                warnings=[{"code": "layer_hidden", "message": "“Gizli katman” katmanı gizli; çizilen nesne görünmeyecek.",
                           "path": "layerId"}], layer="gizli"),
    ]
    out[-1]["steps"][0]["input"]["layerId"] = "gizli"
    par = {"kind": "parallel", "axis": [W(p) for p in L], "left": 2, "right": 3}
    out += [
        refused("Eksenin köşesi sonlu değil", par, "not_finite", not_finite_point("Eksenin köşesinin", "x"),
                "construction.axis[1].x", non_finite={"construction.axis[1].x": "NaN"}),
        refused("Sol mesafe eksi", {**par, "left": -1}, "invalid_distance", "Sol ve sağ mesafe sıfır ya da daha büyük olmalı.",
                "construction.left"),
        refused("Mesafeler sıfır ve eksen yok", {**par, "left": 0, "right": 0}, "invalid_distance",
                "Sol ve sağ mesafe sıfır ve eksen çizilmiyor: çizilecek bir şey yok.", "construction.left"),
        refused("Eksen tek noktaya iner", {**par, "axis": [W(L[0]), W(L[0])]}, "degenerate_shape",
                "Eksenin en az iki ayrı noktası olmalı.", "construction.axis"),
        refused("Dik in: nokta hattın üzerinde", {"kind": "perpendicularIn", "reference": {"kind": "line", "a": W(at(0, 0)),
                                                                                           "b": W(at(10, 0))},
                                                  "points": [W(at(4, 0))]}, "degenerate_shape", "Nokta zaten hattın üzerinde.",
                "construction.points[0]"),
        refused("Dik in: nokta dairenin merkezinde", {"kind": "perpendicularIn", "reference": {"kind": "circle", "c": W(at(0, 0)),
                                                                                               "r": 5},
                                                      "points": [W(at(0, 0))]}, "degenerate_shape",
                "Bu noktadan dik inilemez: nokta dairenin merkezinde.", "construction.points[0]"),
        refused("Dik çık: dik boy sıfır", {"kind": "perpendicularOut", "a": W(at(0, 0)), "b": W(at(10, 0)),
                                          "stations": [{"along": 5, "offset": 0}]}, "invalid_distance", "Dik boy sıfır olamaz.",
                "construction.stations[0].offset"),
        refused("Dik çık: hattın uzunluğu yok", {"kind": "perpendicularOut", "a": W(at(0, 0)), "b": W(at(0, 0)),
                                                "stations": [{"along": 5, "offset": 1}]}, "degenerate_shape",
                "Çizginin iki noktası çakışıyor; uzunluğu olan bir çizgi verin.", "construction.b"),
        refused("Teğet yok: daire ötekinin içinde", {"kind": "tangentLine", "first": {"c": W(c1), "r": 10},
                                                     "second": {"c": W(at(1, 0)), "r": 2}, "nearFirst": W(c1),
                                                     "nearSecond": W(at(1, 0))}, "no_intersection",
                "İki dairenin ortak teğeti yok: biri ötekinin içinde ya da teğetin noktası yayın dışında.", "construction.second"),
        refused("Dördüncü köşe: üç köşe bir doğruda", {"kind": "fourthCorner", "a": W(at(0, 0)), "b": W(at(5, 0)),
                                                       "c": W(at(10, 0))}, "degenerate_shape",
                "Üç köşe bir doğru üzerinde: paralelkenar olmaz. Başka bir köşe verin.", "construction.c"),
        refused("Menzil halkaları: 101 halka", {"kind": "rangeRings", "centre": W(at(0, 0)), "spacing": 5, "count": 101},
                "invalid_count", "Halka sayısı 1 ile 100 arasında olmalı.", "construction.count"),
        refused("Menzil halkaları: aralık sıfır", {"kind": "rangeRings", "centre": W(at(0, 0)), "spacing": 0, "count": 3},
                "invalid_distance", "Halkaların aralığı sıfırdan büyük olmalı.", "construction.spacing"),
        refused("Ara nokta: iki yöntem birden", {"kind": "pointsBetween", "a": W(a), "b": W(b), "parts": 3, "ratios": [0.5]},
                "invalid_count", "Parça sayısı (parts), uzaklıklar (distances) ya da oranlardan (ratios) yalnız birini verin.",
                "construction.parts"),
        refused("Ara nokta: tek parça", {"kind": "pointsBetween", "a": W(a), "b": W(b), "parts": 1}, "invalid_count",
                "Parça sayısı 2 ile 10 000 arasında olmalı.", "construction.parts"),
        refused("Kesişim: daireler buluşmuyor", {"kind": "intersection", "meeting": {"method": "distances", "a": W(A), "da": 2,
                                                                                     "b": W(B), "db": 3}},
                "no_intersection", "Uzaklıklar kesişmiyor: daireler birbirine ulaşmıyor ya da biri ötekinin içinde.",
                "construction.meeting.db"),
        refused("Kesişim: doğrultular arkada buluşuyor", {"kind": "intersection", "meeting": {
            "method": "bearings", "a": W(A), "ta": -math.pi / 4, "b": W(B), "tb": math.pi / 4}}, "no_intersection",
                "Doğrultular kesişmiyor: paralel ya da kesişim noktalardan birinin arkasında kalıyor.", "construction.meeting.tb"),
        refused("Kesişim: doğrular paralel", {"kind": "intersection", "meeting": {
            "method": "lines", "a": W(at(0, 0)), "b": W(at(4, 0)), "c": W(at(0, 5)), "d": W(at(4, 5))}}, "no_intersection",
                "Doğrular paralel; kesişmiyorlar.", "construction.meeting.d"),
        refused("Paralel yardımcı çizgi: nokta kenarın doğrusunda", {"kind": "xlineOffset", "a": W(at(0, 0)), "b": W(at(10, 0)),
                                                                     "side": W(at(20, 0)), "distance": 2},
                "degenerate_shape", "Yanın noktası kenarın doğrusunun üstünde; bir yan gösterin.", "construction.side"),
        refused("Kilitli katman", {"kind": "fourthCorner", "a": W(a), "b": W(b), "c": W(at(14, 6))}, "layer_locked",
                "“Kilitli katman” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.",
                "layerId", extra={"layerId": "kilitli"}),
        refused("Dik in: nokta yok", {"kind": "perpendicularIn", "reference": {"kind": "line", "a": W(at(0, 0)),
                                                                               "b": W(at(10, 0))}, "points": []},
                "too_few_points", "Dik inilecek en az bir nokta verin.", "construction.points"),
    ]
    # cad.entities.create's own refusals: the construction is good, the layer, the look or the revision is not.
    good = {"kind": "fourthCorner", "a": W(a), "b": W(b), "c": W(at(14, 6)), "area": True}
    e = POLY_ERR
    out += [
        refused("Kalınlık 100 mm'den büyük (paralelkenar alanı)", good, "invalid_line_weight", e["invalid_line_weight"]["message"], "lineWeight",
                extra={"lineWeight": 101}),
        refused("Olmayan katman", good, "layer_not_found", e["layer_not_found"]["message"], "layerId",
                extra={"layerId": "yok"}),
        refused("Katman değil grup", good, "not_a_layer", e["not_a_layer"]["message"], "layerId", extra={"layerId": "pafta"}),
        refused("Beklenen sürüm sayı değil", good, "invalid_revision", e["invalid_revision"]["message"], "expectedRevision",
                extra={"expectedRevision": "012"}),
        {"name": "Beklenen sürüm geçmişte kaldıysa conflict, hiçbir şey yazılmaz", "steps": [
            {"op": "captureRevision", "as": "old"},
            {"op": "execute", "input": inp(good),
             "result": {"status": "completed", "output": {"created": "$any", "ids": [NEXT], "revision": "$current"},
                        "warnings": []}},
            {"op": "execute", "input": inp(good, expectedRevision="$old"),
             "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": e["revision_conflict"]["message"],
                                                        "path": "expectedRevision", "revision": "$current"}}}]},
    ]
    assert bearings_meet(A, -math.pi / 4, B, math.pi / 4) is None
    assert circles_meet(A, 2, B, 3) == []
    return out


def main() -> int:
    check = "--check" in sys.argv[1:]
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.constructions.create", "commandVersion": 1,
           "title": "Yapım yardımcıları",
           "note": "Written by scripts/fixtures/construction_cases.py from the constructions' definitions (docs/adr/0622 §9, "
                   "0197, 0057); do not edit by hand.",
           "setup": SETUP, "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    path = OUT / "cad.constructions.create.json"
    if check:
        if not path.exists() or path.read_text(encoding="utf-8") != text:
            print(f"{path.relative_to(ROOT)} yeniden üretilenle aynı değil", file=sys.stderr)
            return 1
        print(f"cad.constructions.create durumları güncel: {len(doc['cases'])} durum.")
        return 0
    path.write_text(text, encoding="utf-8")
    print(f"yazıldı: {path.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
