#!/usr/bin/env python3
"""Paralel kaydır's shared edge with the vertices inside it (docs/adr/2600; plan/map-cad.md MAP-07): the shared cases,
written from the ADR's rules without KentOS code, with exact fractions.

- Paralel kaydır (docs/adr/0191): the edge's line moves d along its normal (an area's away from its inside); its two
  vertices go to where the moved line meets the neighbouring edges' lines.
- Topolojik düzenleme (docs/adr/0160, 1960): the shown objects with a vertex at a moved corner take its new place.
- docs/adr/2600: a shown neighbour's vertex strictly inside the shared straight edge (a third object's corner on it, a
  T junction; a neighbour's own vertex along it; a hole's vertex; a point only with Noktalar da) moves square to the
  edge by d, so it stays on the moved edge; refused when the shortened edge no longer reaches it, or when the edge is
  an arc with such a vertex on it. A locked neighbour's stay, as its corners do (split and said, or nothing written).

    python3 scripts/fixtures/edge_shift_junction_cases.py          # write
    python3 scripts/fixtures/edge_shift_junction_cases.py --check  # compare
"""

import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.shapes.edit.junction.json"
X0, Y0 = 487000, 4420000
# Map coordinates hold a last bit near a nanometre; a shifted corner off the whole metres carries it: a micrometre.
WITHIN = 1e-6

OFF_EDGE = ("Kenarın ortasındaki ortak köşe kaydırılan kenarın dışına düşüyor: kenar o köşeyi geçecek kadar kısalıyor. "
            "Daha kısa bir uzaklık yazın ya da Topolojik düzenlemeyi kapatın.")
ON_ARC = ("Yaylı ortak kenarın ortasında komşu nesnenin köşesi var; yay kaydırılınca o köşe kenardan ayrılır. "
          "Topolojik düzenlemeyi kapatın ya da o köşeyi kaldırın.")
LOCKED = {"code": "neighbour_locked", "message": "Kilitli katmandaki 1 komşu nesne değişmedi; ortak sınır ayrıldı.",
          "path": "work.topology"}
STOPPED = ("Ortak sınır kilitli katmandaki 1 komşu nesneyle paylaşılıyor; hiçbir şey yazılmadı. Kilidi açın ya da "
           "Topolojik düzenleme › Kilitli komşuda durdur seçeneğini kapatın.")


def uid(n):
    return f"0192f5a6-2600-7000-8000-{n:012x}"


def W(p):
    return {"x": float(X0 + p[0]), "y": float(Y0 + p[1])}


def layer(id, name, locked=False):
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def rect(x0, y0, x1, y1):
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]


# The objects, local metres (east, north from X0, Y0).
A = rect(0, 0, 10, 20)
B1 = [(10, 0), (20, 0), (20, 10), (10, 10)]
B2 = [(10, 10), (20, 10), (20, 20), (10, 20)]
P = (10, 5)
D = rect(40, 0, 50, 20)
E = [(50, 0), (60, 0), (60, 20), (50, 20), (50, 10)]
L = ((50, 10), (60, 10))
FF = rect(80, 0, 90, 20)
G = [(90, 0), (100, 0), (100, 20), (90, 20), (90, 10)]
H = [(120, 0), (130, 4), (130, 16), (120, 20)]
I = [(130, 4), (140, 4), (140, 16), (130, 16), (130, 5)]
M = rect(210, 0, 230, 20)
N_OUT = rect(200, -10, 240, 30)
N_HOLE = [(210, 0), (210, 20), (230, 20), (230, 10), (230, 0)]
J = rect(150, 0, 160, 20)
BULGE = 0.25
# The arc's middle (sagitta bulge · half chord) and each half's bulge, tan(θ/8) for θ = 4·atan(bulge).
MID = (160 + BULGE * 10, 10)
HALF = math.tan(math.atan(BULGE) / 2)
K = [(160, 20), MID, (160, 0), (170, 0), (170, 20)]
# Two T junctions and a path's vertex on R's east edge; ends beyond the edge, a millimetre off it and on a hidden layer.
R = rect(300, 0, 310, 20)
S = rect(310, 0, 320, 20)
T1 = ((310, 5), (320, 5))
T2 = [(305, 12), (310, 12), (315, 14)]
T3 = ((310, 25), (315, 30))
T4 = ((F("310.001"), 16), (320, 16))
T5 = ((310, 18), (320, 18))
# A locked line ending inside the shared edge.
UU = rect(340, 0, 350, 20)
VV = rect(350, 0, 360, 20)
LK = ((350, 10), (360, 10))
# A road's open path and a branch ending on its first edge.
RD = [(400, 0), (400, 20), (410, 30)]
BR = ((400, 10), (390, 10))


def polygon(id, layer_id, pts, bulges=None, holes=None):
    e = {"kind": "polygon", "id": id, "layerId": layer_id, "attrs": {}, "pts": [W(p) for p in pts]}
    if bulges:
        e["bulges"] = bulges
    if holes:
        e["holes"] = [{"pts": [W(p) for p in h]} for h in holes]
    return e


ENTITIES = [
    polygon(1, "parsel", A),
    polygon(2, "parsel", B1),
    polygon(3, "parsel", B2),
    {"kind": "point", "id": 4, "layerId": "parsel", "attrs": {}, "p": W(P)},
    polygon(5, "parsel", D),
    polygon(6, "parsel", E),
    {"kind": "line", "id": 7, "layerId": "yol", "attrs": {}, "a": W(L[0]), "b": W(L[1])},
    polygon(8, "parsel", FF),
    polygon(9, "kilitli", G),
    polygon(10, "parsel", H),
    polygon(11, "parsel", I),
    polygon(12, "parsel", M),
    polygon(13, "parsel", N_OUT, holes=[N_HOLE]),
    polygon(14, "parsel", J, bulges=[0.0, BULGE, 0.0, 0.0]),
    polygon(15, "parsel", K, bulges=[-HALF, -HALF, 0.0, 0.0, 0.0]),
    polygon(16, "parsel", R),
    polygon(17, "parsel", S),
    {"kind": "line", "id": 18, "layerId": "yol", "attrs": {}, "a": W(T1[0]), "b": W(T1[1])},
    {"kind": "polyline", "id": 19, "layerId": "yol", "attrs": {}, "pts": [W(p) for p in T2]},
    {"kind": "line", "id": 20, "layerId": "yol", "attrs": {}, "a": W(T3[0]), "b": W(T3[1])},
    {"kind": "line", "id": 21, "layerId": "yol", "attrs": {}, "a": W(T4[0]), "b": W(T4[1])},
    {"kind": "line", "id": 22, "layerId": "gizli", "attrs": {}, "a": W(T5[0]), "b": W(T5[1])},
    polygon(23, "parsel", UU),
    polygon(24, "parsel", VV),
    {"kind": "line", "id": 25, "layerId": "kilitli", "attrs": {}, "a": W(LK[0]), "b": W(LK[1])},
    {"kind": "polyline", "id": 26, "layerId": "yol", "attrs": {}, "pts": [W(p) for p in RD]},
    {"kind": "line", "id": 27, "layerId": "yol", "attrs": {}, "a": W(BR[0]), "b": W(BR[1])},
]
IDS = [e["id"] for e in ENTITIES]


def setup():
    return {
        "format": "kentos.document", "version": 2, "name": "Ortak kenarın köşeleri",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 500, "workspace": "cad"},
        "origin": {"x": 0.0, "y": 0.0},
        "layers": [layer("parsel", "Parsel"), layer("yol", "Yol"), layer("kilitli", "Kilitli katman", locked=True),
                   {**layer("gizli", "Gizli katman"), "visible": False}],
        "activeLayer": "parsel",
        "styles": {"items": [], "categories": []},
        "entities": ENTITIES,
        "uids": [uid(i) for i in IDS],
    }


# --- Paralel kaydır and the carried vertices, independently --------------------------------------------------------

def sub(p, q):
    return (p[0] - q[0], p[1] - q[1])


def cross(p, q):
    return p[0] * q[1] - p[1] * q[0]


def area(pts):
    o = pts[0]
    return abs(sum(cross(sub(pts[i], o), sub(pts[(i + 1) % len(pts)], o)) for i in range(len(pts)))) / 2


def meet(p, d, q, e):
    s = cross(sub(q, p), e) / cross(d, e)
    return (p[0] + d[0] * s, p[1] + d[1] * s)


def east_shift(pts, i, d):
    """Paralel kaydır of a counter-clockwise ring's edge i that runs north (x constant): its outward normal is east,
    so the moved line is x + d; the two vertices meet the neighbouring edges' lines there."""
    pts = [(F(x), F(y)) for x, y in pts]
    n = len(pts)
    a, b = pts[i], pts[(i + 1) % n]
    assert a[0] == b[0] and b[1] > a[1], "an edge running north"
    t = (F(0), F(1))
    a2, b2 = (a[0] + d, a[1]), (b[0] + d, b[1])
    p, q = pts[(i - 1) % n], pts[(i + 2) % n]
    out = list(pts)
    out[i] = meet(a2, t, p, sub(a, p))
    out[(i + 1) % n] = meet(a2, t, q, sub(q, b))
    return out


def moved(pts, places):
    """`pts` with each vertex at a place moved to its new place."""
    out = []
    for v in pts:
        v = (F(v[0]), F(v[1]))
        out.append(places.get(v, v))
    return out


def edge_places(old, new, inside, d):
    """The moved corners (old edge's ends to new) and the vertices strictly inside, square to the edge by d."""
    places = {old[0]: new[0], old[1]: new[1]}
    for v in inside:
        places[(F(v[0]), F(v[1]))] = (F(v[0]) + d, F(v[1]))
    return places


def length(pts):
    return sum(math.hypot(float(pts[i + 1][0] - pts[i][0]), float(pts[i + 1][1] - pts[i][1]))
               for i in range(len(pts) - 1))


def pts_field(pts):
    return {"pts": [W(p) for p in pts]}


def num(v):
    v = float(v)
    return int(v) if v == int(v) else v


# --- The steps -----------------------------------------------------------------------------------------------------

def U(i):
    return f"$uidOf:{i}"


def shift(id, at, distance, topology=None):
    work = {"kind": "edgeShift", "id": U(id), "at": W(at), "distance": distance}
    if topology is not None:
        work["topology"] = topology
    return {"work": work}


def output(changed, before, after, distance, neighbours=(), carried=0):
    o = {"changed": [U(i) for i in changed], "before": num(before), "after": num(after), "objects": len(changed),
         "skipped": 0, "count": 0, "distance": num(distance), "revision": "$current"}
    if neighbours:
        o["neighbours"] = [{"uid": U(i), "before": num(b), "after": num(a)} for i, b, a in neighbours]
    if carried:
        o["carried"] = carried
    return o


def done(name, input, out, fields, warnings=()):
    step = {"op": "execute", "input": input, "within": WITHIN,
            "result": {"status": "completed", "output": out, "warnings": list(warnings)},
            "expect": {"ids": IDS, "canUndo": True, "dirty": True, "fields": fields}}
    back = {"op": "undo", "returns": "Paralel kaydır", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}}
    return {"name": name, "steps": [step, back]}


def refused(name, input, code, message, path):
    return {"name": name, "steps": [{
        "op": "execute", "input": input,
        "result": {"status": "failed", "error": {"code": code, "message": message, "path": path}},
        "expect": {"ids": IDS, "dirty": False, "revision": "same"}}]}


def cases():
    out = []

    # A T junction: B1 and B2 meet A's east edge at (10, 10); the edge 3 m east carries their shared corner.
    a3 = east_shift(A, 1, 3)
    places = edge_places(((F(10), F(0)), (F(10), F(20))), (a3[1], a3[2]), [(10, 10)], 3)
    b1, b2 = moved(B1, places), moved(B2, places)
    assert area(a3) + area(b1) + area(b2) == area(A) + area(B1) + area(B2)
    out.append(done("Üçlü birleşim: A'nın doğu kenarı 3 m doğuya; B1 ile B2'nin köşeleri ve kenarın ortasındaki ortak "
                    "köşeleri birlikte kayar, alanların toplamı korunur",
                    shift(1, (10, 15), 3, {}),
                    output([1, 2, 3], area(A), area(a3), 3,
                           [(2, area(B1), area(b1)), (3, area(B2), area(b2))], carried=1),
                    {"1": pts_field(a3), "2": pts_field(b1), "3": pts_field(b2), "4": {"p": W(P)}}))

    # Noktalar da: the boundary stone on the edge goes with it too.
    places_p = edge_places(((F(10), F(0)), (F(10), F(20))), (a3[1], a3[2]), [(10, 10), P], 3)
    out.append(done("Noktalar da: kenarın üstündeki sınır taşı da kenarla kayar",
                    shift(1, (10, 15), 3, {"points": True}),
                    output([1, 2, 3, 4], area(A), area(a3), 3,
                           [(2, area(B1), area(b1)), (3, area(B2), area(b2)), (4, 0, 0)], carried=2),
                    {"1": pts_field(a3), "2": pts_field(b1), "3": pts_field(b2),
                     "4": {"p": W(places_p[(F(P[0]), F(P[1]))])}}))

    # A neighbour's own vertex along the edge and a line ending there.
    d2 = east_shift(D, 1, 2)
    places = edge_places(((F(50), F(0)), (F(50), F(20))), (d2[1], d2[2]), [(50, 10)], 2)
    e2 = moved(E, places)
    l2 = moved(list(L), places)
    out.append(done("Çoklu köşe: komşunun kenar üstündeki ara köşesi ve oraya bağlanan çizgi kenarla birlikte kayar",
                    shift(5, (50, 15), 2, {}),
                    output([5, 6, 7], area(D), area(d2), 2,
                           [(6, area(E), area(e2)), (7, 10, l2[1][0] - l2[0][0])], carried=1),
                    {"5": pts_field(d2), "6": pts_field(e2), "7": {"a": W(l2[0]), "b": W(l2[1])}}))

    # A locked neighbour's vertex along the edge stays, as its corners do.
    f2 = east_shift(FF, 1, 2)
    out.append(done("Kilitli komşunun kenar üstündeki köşesi de yerinde kalır, sınırın ayrıldığı söylenir",
                    shift(8, (90, 15), 2, {}),
                    output([8], area(FF), area(f2), 2),
                    {"8": pts_field(f2), "9": pts_field(G)}, warnings=[LOCKED]))
    out.append(refused("Kilitli komşuda durdur: kenar üstündeki köşesi olan kilitli komşuyla hiçbir şey yazılmaz",
                       shift(8, (90, 15), 2, {"stopOnLocked": True}), "shared_boundary_locked", STOPPED,
                       "work.topology"))

    # A shortening edge: 5 m out the edge runs from y 6 to 14, the shared vertex at y 5 falls off it.
    h5 = east_shift(H, 1, 5)
    assert h5[1][1] > 5, "the moved edge no longer reaches y = 5"
    out.append(refused("Kısalan kenar: kenarın ortasındaki ortak köşe kenarın dışına düşer, hiçbir şey yazılmaz",
                       shift(10, (130, 10), 5, {}), "edge_shift_refused", OFF_EDGE, "work.distance"))
    h2 = east_shift(H, 1, 2)
    places = edge_places(((F(130), F(4)), (F(130), F(16))), (h2[1], h2[2]), [(130, 5)], 2)
    i2 = moved(I, places)
    assert h2[1][1] < 5 < h2[2][1]
    out.append(done("Kısalan kenarda içeride kalan ortak köşe kenarla birlikte kayar",
                    shift(10, (130, 10), 2, {}),
                    output([10, 11], area(H), area(h2), 2, [(11, area(I), area(i2))], carried=1),
                    {"10": pts_field(h2), "11": pts_field(i2)}))

    # A hole: the outer parcel's hole runs round M with a vertex on M's east edge.
    m2 = east_shift(M, 1, 2)
    places = edge_places(((F(230), F(0)), (F(230), F(20))), (m2[1], m2[2]), [(230, 10)], 2)
    hole2 = moved(N_HOLE, places)
    n_before = area(N_OUT) - area(N_HOLE)
    n_after = area(N_OUT) - area(hole2)
    assert area(M) + n_before == area(m2) + n_after
    out.append(done("Delik: içteki parselin kenarı kayınca dıştakinin deliğindeki ara köşe de kayar",
                    shift(12, (230, 15), 2, {}),
                    output([12, 13], area(M), area(m2), 2, [(13, n_before, n_after)], carried=1),
                    {"12": pts_field(m2), "13": {"pts": [W(p) for p in N_OUT],
                                                 "holes": [{"pts": [W(p) for p in hole2]}]}}))

    # An arc edge with the neighbour's vertex at its middle.
    out.append(refused("Yaylı ortak kenarın ortasında komşu köşesi: yay kaydırılınca köşe kenardan ayrılır, reddedilir",
                       shift(14, MID, 1, {}), "edge_shift_refused", ON_ARC, "work.at"))

    # Inward: B1 and B2 grow, their shared corner on the edge with it.
    a_in = east_shift(A, 1, -2)
    places = edge_places(((F(10), F(0)), (F(10), F(20))), (a_in[1], a_in[2]), [(10, 10)], -2)
    b1_in, b2_in = moved(B1, places), moved(B2, places)
    out.append(done("Üçlü birleşim içeri: A'nın kenarı 2 m batıya, B1 ile B2 büyür, ortadaki köşe kenarla gelir",
                    shift(1, (10, 15), -2, {}),
                    output([1, 2, 3], area(A), area(a_in), -2,
                           [(2, area(B1), area(b1_in)), (3, area(B2), area(b2_in))], carried=1),
                    {"1": pts_field(a_in), "2": pts_field(b1_in), "3": pts_field(b2_in)}))

    # A target area: 240 m² is 2 m out; the junction goes with it.
    a2 = east_shift(A, 1, 2)
    places = edge_places(((F(10), F(0)), (F(10), F(20))), (a2[1], a2[2]), [(10, 10)], 2)
    b1_2, b2_2 = moved(B1, places), moved(B2, places)
    work = shift(1, (10, 15), 2, {})
    work["work"].pop("distance")
    work["work"]["area"] = 240
    out.append(done("Hedef alan 240 m²: kenar 2 m kayar, ortadaki ortak köşe de",
                    work,
                    output([1, 2, 3], area(A), area(a2), 2,
                           [(2, area(B1), area(b1_2)), (3, area(B2), area(b2_2))], carried=1),
                    {"1": pts_field(a2), "2": pts_field(b1_2), "3": pts_field(b2_2)}))

    # Two T junctions and a path's vertex inside R's east edge go with it; an end on the edge's line beyond it, one a
    # millimetre off it and one on a hidden layer stay.
    r2 = east_shift(R, 1, 2)
    places = edge_places(((F(310), F(0)), (F(310), F(20))), (r2[1], r2[2]), [T1[0], T2[1]], 2)
    s2 = moved(S, places)
    t1 = moved(list(T1), places)
    t2 = moved(T2, places)
    out.append(done("İki T birleşimi ve çoklu çizginin ara köşesi kenarla kayar; kenarın uzantısındaki, 1 mm uzaktaki "
                    "ve gizli katmandaki uçlar yerinde kalır",
                    shift(16, (310, 15), 2, {}),
                    output([16, 17, 18, 19], area(R), area(r2), 2,
                           [(17, area(S), area(s2)), (18, 10, length(t1)), (19, length(T2), length(t2))], carried=2),
                    {"16": pts_field(r2), "17": pts_field(s2), "18": {"a": W(t1[0]), "b": W(t1[1])},
                     "19": pts_field(t2), "20": {"a": W(T3[0]), "b": W(T3[1])},
                     "21": {"a": W(T4[0]), "b": W(T4[1])}, "22": {"a": W(T5[0]), "b": W(T5[1])}}))

    # The shared vertex lands on the shortened edge's very end: refused too.
    h25 = east_shift(H, 1, F(5, 2))
    assert h25[1][1] == 5, "2.5 m out the edge starts at y = 5"
    out.append(refused("Kısalan kenarın ucuna düşen ortak köşe de reddedilir",
                       shift(10, (130, 10), 2.5, {}), "edge_shift_refused", OFF_EDGE, "work.distance"))
    work = shift(10, (130, 10), 5, {})
    work["work"].pop("distance")
    work["work"]["area"] = num(area(h5))
    out.append(refused("Hedef alanla kısalan kenarda da ortak köşe dışarı düşerse reddedilir",
                       work, "edge_shift_refused", OFF_EDGE, "work.distance"))

    # A locked line ending inside the edge: it stays, the junction is said to be split; or nothing is written.
    u2 = east_shift(UU, 1, 2)
    places = edge_places(((F(350), F(0)), (F(350), F(20))), (u2[1], u2[2]), [], 2)
    v2 = moved(VV, places)
    out.append(done("Kilitli çizginin ucu kenarın ortasında: kenar ve komşu kayar, çizgi yerinde kalır, ayrıldığı "
                    "söylenir",
                    shift(23, (350, 15), 2, {}),
                    output([23, 24], area(UU), area(u2), 2, [(24, area(VV), area(v2))]),
                    {"23": pts_field(u2), "24": pts_field(v2), "25": {"a": W(LK[0]), "b": W(LK[1])}},
                    warnings=[LOCKED]))
    out.append(refused("Kilitli çizginin ucu kenarın ortasında, Kilitli komşuda durdur açık: hiçbir şey yazılmaz",
                       shift(23, (350, 15), 2, {"stopOnLocked": True}), "shared_boundary_locked", STOPPED,
                       "work.topology"))

    # A road's open path: its first edge 2 m to the right of its way (east); the free end moves square, the other
    # meets the next edge's line; the branch ending on the edge goes with it.
    rd2 = [(F(402), F(0)), (F(402), F(22)), (F(410), F(30))]
    br2 = [(F(402), F(10)), (F(390), F(10))]
    out.append(done("Yolun çoklu çizgisi: ilk kenar 2 m sağa, kenara bağlanan kol da kayar",
                    shift(26, (400, 15), 2, {}),
                    output([26, 27], length(RD), length(rd2), 2, [(27, 10, length(br2))], carried=1),
                    {"26": pts_field(rd2), "27": {"a": W(br2[0]), "b": W(br2[1])}}))

    # Inward on the neighbour's own vertex and on the hole.
    d_in = east_shift(D, 1, -2)
    places = edge_places(((F(50), F(0)), (F(50), F(20))), (d_in[1], d_in[2]), [(50, 10)], -2)
    e_in, l_in = moved(E, places), moved(list(L), places)
    out.append(done("Çoklu köşe içeri: komşu büyür, ara köşesi ve çizginin ucu kenarla gelir",
                    shift(5, (50, 15), -2, {}),
                    output([5, 6, 7], area(D), area(d_in), -2,
                           [(6, area(E), area(e_in)), (7, 10, l_in[1][0] - l_in[0][0])], carried=1),
                    {"5": pts_field(d_in), "6": pts_field(e_in), "7": {"a": W(l_in[0]), "b": W(l_in[1])}}))
    m_in = east_shift(M, 1, -2)
    places = edge_places(((F(230), F(0)), (F(230), F(20))), (m_in[1], m_in[2]), [(230, 10)], -2)
    hole_in = moved(N_HOLE, places)
    out.append(done("Delik içeri: içteki parsel küçülür, dıştakinin deliğindeki ara köşe kenarla gelir",
                    shift(12, (230, 15), -2, {}),
                    output([12, 13], area(M), area(m_in), -2,
                           [(13, area(N_OUT) - area(N_HOLE), area(N_OUT) - area(hole_in))], carried=1),
                    {"12": pts_field(m_in), "13": {"pts": [W(p) for p in N_OUT],
                                                   "holes": [{"pts": [W(p) for p in hole_in]}]}}))

    # A free edge: nothing shared, nothing carried.
    a_w = [(F(-1), F(0)), (F(10), F(0)), (F(10), F(20)), (F(-1), F(20))]
    out.append(done("Komşusuz kenar: Topolojik düzenleme açıkken de yalnız kenar kayar",
                    shift(1, (0, 15), 1, {}),
                    output([1], area(A), area(a_w), 1),
                    {"1": pts_field(a_w)}))

    # Topolojik düzenleme off: only the edge.
    out.append(done("Topolojik düzenleme kapalı: yalnız kenar kayar, ortadaki ortak köşe yerinde kalır",
                    shift(1, (10, 15), 3),
                    output([1], area(A), area(a3), 3),
                    {"1": pts_field(a3), "2": pts_field(B1), "3": pts_field(B2)}))
    return out


def build():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.shapes.edit",
        "commandVersion": 1,
        "title": "Paralel kaydır'da ortak kenarın ortasındaki köşeler: üçlü birleşim, ara köşe, delik (ADR 2600)",
        "note": "Beklenenler scripts/fixtures/edge_shift_junction_cases.py'den: ADR 0191'in Paralel kaydır kuralı, ADR "
                "1960'ın ortak köşeleri ve ADR 2600'ün kenar içindeki köşeleri kesirlerle; KentOS kodu kullanılmadı.",
        "setup": setup(),
        "cases": cases(),
    }


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"{OUT.relative_to(ROOT)} eski: python3 scripts/fixtures/edge_shift_junction_cases.py ile yeniden yazın")
        print(f"tutarlı: {OUT.relative_to(ROOT)}")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(build()['cases'])} durum)")


if __name__ == "__main__":
    main()
