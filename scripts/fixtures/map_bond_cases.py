#!/usr/bin/env python3
"""Paralel kaydır's reference and Bu uzaklığı koru (docs/adr/2580; plan/map-cad.md MAP-06): the shared cases,
written from the ADR's rules without KentOS code, with exact fractions.

- Paralel kaydır (docs/adr/0191): a straight edge's line moves d along its normal (an area's away from its inside,
  a path's to the right of its way); its two vertices go to where the moved line meets the neighbouring edges' lines.
- With a reference (a straight segment of another object): the edge goes `distance` from the reference's line on the
  side it is on now; the edge and the reference are parallel within 0.001° (sine 1.745 329 251 994 33e-5).
- Bu uzaklığı koru: the distance is a bond, a distance constraint from the reference (`segment`) to the edge
  (`boundary`) with its parameter (d1, d2 … the value given, or the distance there is now). When the reference moves,
  the edge moves square to itself by Paralel kaydır's rule until the distance is the parameter's again; a chain's next
  edge follows the edge; a reference that turns, an edge held (on a locked layer, or moved by the write itself) or two
  bonds wanting different shifts refuse the write whole (constraint_conflict, nothing written).

Map coordinates (E ≈ 487 200, N ≈ 4 420 000 m); the drawing's numbers are compared within a micrometre.

    python3 scripts/fixtures/map_bond_cases.py          # write
    python3 scripts/fixtures/map_bond_cases.py --check  # compare
"""

import copy
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.shapes.edit.bond.json"
WITHIN = 1e-6
PARALLEL = 1.745_329_251_994_33e-5

WAY = "Ya da kısıtları gevşeterek düzenleyin."
NOT_PARALLEL = "Bağlı kenar artık referansına paralel değil (referans ya da kenar döndü; bağlı kenar döndürülmez)"
INCONSISTENT = "Bağlar birlikte tutmuyor"


def uid(n):
    return f"0192f5a6-2580-7000-8000-{n:012x}"


def pt(x, y):
    return {"x": float(x), "y": float(y)}


def layer(id, name, locked=False):
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


# The objects: (id, kind, layer, geometry).
P1 = [(487200, 4420010), (487220, 4420010), (487220, 4420040), (487200, 4420040)]
P2 = [(487220, 4420010), (487240, 4420010), (487240, 4420040), (487220, 4420040)]
S = [(487203, 4420015), (487217, 4420015), (487217, 4420037), (487203, 4420037)]
L = [(487250, 4420010), (487270, 4420010), (487270, 4420040), (487250, 4420040)]
C = [(487185, 4420020), (487188, 4420007), (487245, 4420007), (487248, 4420020)]
A = [(487300, 4420010), (487320, 4420010), (487320, 4420040), (487300, 4420040)]
R = ((487190, 4420004), (487280, 4420004))
N = ((487190, 4420060), (487240, 4420062))
R2 = ((487190, 4420001), (487280, 4420001))
IDS = list(range(1, 11))


def polygon(id, layer_id, pts, bulges=None):
    e = {"kind": "polygon", "id": id, "layerId": layer_id, "attrs": {}, "pts": [pt(*p) for p in pts]}
    if bulges:
        e["bulges"] = bulges
    return e


def line(id, layer_id, ab):
    return {"kind": "line", "id": id, "layerId": layer_id, "attrs": {}, "a": pt(*ab[0]), "b": pt(*ab[1])}


def setup(constraints=None):
    d = {
        "format": "kentos.document", "version": 2, "name": "Harita bağı",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 1000, "workspace": "cad"},
        "origin": {"x": 0.0, "y": 0.0},
        "layers": [layer("parsel", "Parsel"), layer("yol", "Yol"), layer("cekme", "Çekme"), layer("bordur", "Bordür"),
                   layer("kilitli", "Kilitli katman", locked=True)],
        "activeLayer": "parsel",
        "styles": {"items": [], "categories": []},
        "entities": [
            polygon(1, "parsel", P1),
            line(2, "yol", R),
            polygon(3, "parsel", P2),
            polygon(4, "cekme", S),
            polygon(5, "kilitli", L),
            {"kind": "polyline", "id": 6, "layerId": "bordur", "attrs": {}, "pts": [pt(*p) for p in C]},
            line(7, "yol", N),
            polygon(8, "parsel", A, bulges=[0.2, 0.0, 0.0, 0.0]),
            {"kind": "circle", "id": 9, "layerId": "yol", "attrs": {}, "c": pt(487350, 4420004), "r": 3.0},
            line(10, "yol", R2),
        ],
        "uids": [uid(i) for i in IDS],
    }
    if constraints:
        d["constraints"] = constraints
    return d


# --- Paralel kaydır, independently (docs/adr/0191) ------------------------------------------------------------------

def sub(p, q):
    return (p[0] - q[0], p[1] - q[1])


def cross(p, q):
    return p[0] * q[1] - p[1] * q[0]


def ring_area(pts):
    o = pts[0]
    s = sum(cross(sub(pts[i], o), sub(pts[(i + 1) % len(pts)], o)) for i in range(len(pts)))
    return s / 2


def normal(pts, closed, i):
    """The edge's normal as a direction (not unit): an area's away from its inside, a path's to the right."""
    a, b = pts[i], pts[(i + 1) % len(pts)]
    t = sub(b, a)
    right = (t[1], -t[0])
    if not closed:
        return right
    return right if ring_area(pts) > 0 else (-right[0], -right[1])


def unit_axis(v):
    """An axis-parallel direction's unit vector, exactly (the cases' edges and references run along the axes)."""
    x, y = F(v[0]), F(v[1])
    assert x == 0 or y == 0, "an axis-parallel direction"
    return (F((x > 0) - (x < 0)), F((y > 0) - (y < 0)))


def meet(p, d, q, e):
    """Where the line through p along d meets the line through q along e."""
    s = cross(sub(q, p), e) / cross(d, e)
    return (p[0] + d[0] * s, p[1] + d[1] * s)


def shifted(pts, closed, i, dist):
    """Paralel kaydır: edge i moved `dist` (metres) along its unit normal; exact while the normal's length is rational
    (the cases' edges run along the axes)."""
    pts = [(F(x), F(y)) for x, y in pts]
    n = len(pts)
    a, b = pts[i], pts[(i + 1) % n]
    nu = unit_axis(normal(pts, closed, i))
    off = (nu[0] * dist, nu[1] * dist)
    a2, b2 = (a[0] + off[0], a[1] + off[1]), (b[0] + off[0], b[1] + off[1])
    t = sub(b, a)
    out = list(pts)
    has_prev = closed or i > 0
    has_next = closed or i + 1 < n - 1
    if has_prev:
        p = pts[(i - 1) % n]
        out[i] = meet(a2, t, p, sub(a, p))
    else:
        out[i] = a2
    if has_next:
        q = pts[(i + 2) % n]
        out[(i + 1) % n] = meet(a2, t, q, sub(q, b))
    else:
        out[(i + 1) % n] = b2
    return out


def length(pts):
    return sum(math.hypot(float(pts[i + 1][0] - pts[i][0]), float(pts[i + 1][1] - pts[i][1]))
               for i in range(len(pts) - 1))


def gap(ref, p):
    """A point's signed distance from a reference's line, its left positive (exact for an axis-parallel reference)."""
    a, b = (F(ref[0][0]), F(ref[0][1])), (F(ref[1][0]), F(ref[1][1]))
    return cross(unit_axis(sub(b, a)), sub((F(p[0]), F(p[1])), a))


def towards(ref, pts, closed, i, distance):
    """The shift along the edge's normal that puts edge i `distance` from the reference on its side, and the side."""
    g0 = gap(ref, pts[i])
    side = 1 if g0 >= 0 else -1
    a, b = (F(ref[0][0]), F(ref[0][1])), (F(ref[1][0]), F(ref[1][1]))
    u = unit_axis(sub(b, a))
    nu = unit_axis(normal([(F(x), F(y)) for x, y in pts], closed, i))
    # The change of the gap per metre of shift: the reference's left normal along the edge's unit normal (±1).
    k = -u[1] * nu[0] + u[0] * nu[1]
    return (side * F(distance) - g0) / k, side


def moved_line(ab, dx, dy):
    return ((ab[0][0] + dx, ab[0][1] + dy), (ab[1][0] + dx, ab[1][1] + dy))


def pts_field(pts):
    return {"pts": [pt(x, y) for x, y in pts]}


def line_field(ab):
    return {"a": pt(*ab[0]), "b": pt(*ab[1])}


def num(v):
    v = float(v)
    return int(v) if v == int(v) and abs(v) < 2 ** 53 else v


# --- The steps -----------------------------------------------------------------------------------------------------

def shift(id, at, distance=None, reference=None, area=None):
    work = {"kind": "edgeShift", "id": f"$uidOf:{id}", "at": pt(*at)}
    if distance is not None:
        work["distance"] = distance
    if area is not None:
        work["area"] = area
    if reference is not None:
        work["reference"] = reference
    return {"work": work}


def ref(id, at, keep=False):
    r = {"id": f"$uidOf:{id}", "at": pt(*at)}
    if keep:
        r["keep"] = True
    return r


def edited(changed, before, after, distance, bonds=0):
    out = {"changed": [f"$uidOf:{c}" for c in changed], "before": num(before), "after": num(after), "objects": 1,
           "skipped": 0, "count": 0, "distance": num(distance), "revision": "$current"}
    if bonds:
        out["bonds"] = ["$any"] * bonds
    return out


def done(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def refused(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def conflict(why, bonds, frees_words):
    return {"status": "failed", "error": {"code": "constraint_conflict",
                                          "message": f"{why}; hiçbir şey yazılmadı. {frees_words}",
                                          "uids": "$any", "bonds": bonds}}


def bond_in_the_way(param, a, b, kept, measured=None):
    out = {"id": "$any", "label": "Uzaklık", "parameter": param, "parts": [a, b], "kept": num(kept)}
    if measured is not None:
        out["measured"] = num(measured)
    out["frees"] = True
    return out


def segment(id, index=0):
    return {"uid": uid(id), "element": {"kind": "segment", "part": 0, "ring": 0, "index": index}}


def boundary(id, index=0):
    return {"uid": uid(id), "element": {"kind": "boundary", "part": 0, "ring": 0, "index": index}}


def params(*pairs):
    return [{"name": n, "kind": "length", "expression": e} for n, e in pairs]


def step(input, result, expect, command=None, **more):
    s = {"op": "execute"}
    if command:
        s["command"] = command
    s["input"] = input
    s.update(more)
    s["within"] = WITHIN
    s["result"] = result
    s["expect"] = {"ids": IDS, **expect}
    return s


def undo(returns, expect):
    return {"op": "undo", "returns": returns, "within": WITHIN, "expect": {"ids": IDS, **expect}}


def move(id, dx, dy):
    return {"uids": [f"$uidOf:{id}"], "transform": {"kind": "move", "dx": dx, "dy": dy}}


def frees_one(param):
    return f"Şu bağ kaldırılınca düzenleme geçer: {param} (Uzaklık). {WAY}"


def case(name, steps, own_setup=None):
    c = {"name": name}
    if own_setup is not None:
        c["setup"] = own_setup
    c["steps"] = steps
    return c


def cases():
    out = []
    front = (487210, 4420010)
    on_road = (487210, 4420004)

    # Bu uzaklığı koru at 5 m: the parcel's front edge from 6 m to 5 m, its bond and d1 = 5 in one step.
    d_keep, _ = towards(R, P1, True, 0, 5)
    p1_kept = shifted(P1, True, 0, d_keep)
    keep5 = step(shift(1, front, 5, ref(2, on_road, keep=True)),
                 done(edited([1], ring_area([(F(x), F(y)) for x, y in P1]), ring_area(p1_kept), d_keep, bonds=1)),
                 {"revision": "changed", "constraintKinds": ["distance"], "parameters": params(("d1", "5")),
                  "fields": {"1": pts_field(p1_kept)}})
    out.append(case("Bu uzaklığı koru: parselin ön kenarı yolun kenarından 5 m'ye, bağ ve d1 = 5 aynı adımda; tek "
                    "adımda geri alınır", [
                        keep5,
                        undo("Paralel kaydır", {"canRedo": True, "constraintKinds": [], "parameters": [],
                                                "fields": {"1": pts_field(P1)}}),
                    ]))

    # Once: 8 m from the road, no bond; the road moving leaves the parcel.
    d8, _ = towards(R, P1, True, 0, 8)
    p1_8 = shifted(P1, True, 0, d8)
    out.append(case("Referansla tek seferlik: kenar yolun kenarından 8 m'ye; bağ yazılmaz, yol kayınca parsel kalır", [
        step(shift(1, front, 8, ref(2, on_road)),
             done(edited([1], 600, ring_area(p1_8), d8)),
             {"revision": "changed", "constraintKinds": [], "fields": {"1": pts_field(p1_8)}}),
        step(move(2, 0, -1), "$any", {"revision": "changed", "constraintKinds": [],
                                      "fields": {"1": pts_field(p1_8), "2": line_field(moved_line(R, 0, -1))}},
             command="cad.entities.transform"),
    ]))

    # Only the bond: the distance there is now (6 m), nothing moves.
    out.append(case("Bu uzaklığı koru şimdiki uzaklıkla: kenar yerinde kalır, yalnız bağ yazılır (d1 = 6)", [
        step(shift(1, front, None, ref(2, on_road, keep=True)),
             done(edited([], 600, 600, 0, bonds=1)),
             {"revision": "changed", "constraintKinds": ["distance"], "parameters": params(("d1", "6")),
              "fields": {"1": pts_field(P1)}}),
    ]))

    # The road 2 m south: the bonded edge follows, the side edges lengthen; one undo step.
    r_down = moved_line(R, 0, -2)
    d_follow, _ = towards(r_down, p1_kept, True, 0, 5)
    p1_follow = shifted(p1_kept, True, 0, d_follow)
    out.append(case("Yol 2 m güneye kayınca bağlı ön kenar da 2 m kayar, yan kenarlar uzar; tek geri alma adımı", [
        keep5,
        step(move(2, 0, -2), "$any", {"revision": "changed", "constraintKinds": ["distance"],
                                      "fields": {"1": pts_field(p1_follow), "2": line_field(r_down)}},
             command="cad.entities.transform"),
        undo("Taşı", {"canRedo": True, "constraintKinds": ["distance"],
                      "fields": {"1": pts_field(p1_kept), "2": line_field(R)}}),
    ]))

    # Along itself: nothing to follow.
    r_east = moved_line(R, 3, 0)
    out.append(case("Yol kendi doğrultusunda 3 m kayınca bağlı kenar yerinde kalır", [
        keep5,
        step(move(2, 3, 0), "$any", {"revision": "changed",
                                     "fields": {"1": pts_field(p1_kept), "2": line_field(r_east)}},
             command="cad.entities.transform"),
    ]))

    # The road turned: the edge would have to turn, the bond refuses the write.
    out.append(case("Yol döndürülünce bağ tutmaz: constraint_conflict, hiçbir şey yazılmaz; bağ kaldırılınca geçer", [
        keep5,
        step({"uids": ["$uidOf:2"], "transform": {"kind": "rotate", "center": pt(*R[0]), "angle": 0.01}},
             conflict(NOT_PARALLEL, [bond_in_the_way("d1", segment(2), boundary(1), 5)], frees_one("d1")),
             {"revision": "same", "fields": {"1": pts_field(p1_kept), "2": line_field(R)}},
             command="cad.entities.transform"),
    ]))

    # A bonded parcel on a locked layer is held: the road cannot move it.
    locked = setup({"parameters": params(("d1", "6")), "constraints": [
        {"id": "0192f5a6-2580-7000-8000-000000000101",
         "rule": {"kind": "distance", "a": segment(2), "b": boundary(5), "left": True, "parameter": "d1"}}]})
    out.append(case("Kilitli katmandaki bağlı parsel tutulur: yol kayınca constraint_conflict, hiçbir şey yazılmaz", [
        step(move(2, 0, -2),
             conflict(INCONSISTENT, [bond_in_the_way("d1", segment(2), boundary(5), 6, 8)], frees_one("d1")),
             {"revision": "same", "fields": {"2": line_field(R), "5": pts_field(L)}},
             command="cad.entities.transform"),
    ], own_setup=locked))

    # The bonded edge moved by hand: refused; relaxed, the bond goes with its parameter.
    at_kept = (487210, 4420009)
    p1_by_hand = shifted(p1_kept, True, 0, 1)
    out.append(case("Bağlı kenar doğrudan kaydırılınca constraint_conflict; gevşeterek düzenlenince bağ kalkar", [
        keep5,
        step(shift(1, at_kept, 1),
             conflict(INCONSISTENT, [bond_in_the_way("d1", segment(2), boundary(1), 5, 4)], frees_one("d1")),
             {"revision": "same", "constraintKinds": ["distance"], "fields": {"1": pts_field(p1_kept)}}),
        step(shift(1, at_kept, 1),
             done(edited([1], ring_area(p1_kept), ring_area(p1_by_hand), 1),
                  [{"code": "constraints_relaxed",
                    "message": "Kısıtlar gevşetildi: düzenlemenin bozduğu 1 kısıt kaldırıldı."}]),
             {"revision": "changed", "constraintKinds": [], "parameters": [], "fields": {"1": pts_field(p1_by_hand)}},
             relaxConstraints=True),
    ]))

    # The parameter's new value: the edge goes to 7 m.
    d7, _ = towards(R, p1_kept, True, 0, 7)
    p1_7 = shifted(p1_kept, True, 0, d7)
    out.append(case("Parametre 7 olunca kenar yolun kenarından 7 m'ye kayar", [
        keep5,
        step({"edits": [{"kind": "set", "name": "d1", "expression": "7"}]}, "$any",
             {"revision": "changed", "constraintKinds": ["distance"], "parameters": params(("d1", "7")),
              "fields": {"1": pts_field(p1_7), "2": line_field(R)}},
             command="cad.parameters.edit"),
    ]))

    # The bond taken away: the road moves alone.
    out.append(case("Bağ kaldırılınca yol yalnız kayar, parsel yerinde kalır", [
        keep5,
        step({"edit": {"kind": "remove", "ids": ["$constraint:0"]}}, "$any",
             {"revision": "changed", "constraintKinds": [], "parameters": []},
             command="cad.constraints.edit"),
        step(move(2, 0, -2), "$any", {"revision": "changed",
                                      "fields": {"1": pts_field(p1_kept), "2": line_field(r_down)}},
             command="cad.entities.transform"),
    ]))

    # A chain: the setback's front edge 5 m from the parcel's front edge, which is 5 m from the road.
    s_ref = (487210, 4420009)
    p1_front = (p1_kept[0], p1_kept[1])
    d_s, _ = towards(p1_front, S, True, 0, 5)
    s_kept = shifted(S, True, 0, d_s)
    d_s_follow, _ = towards(moved_line(p1_front, 0, -2), s_kept, True, 0, 5)
    s_follow = shifted(s_kept, True, 0, d_s_follow)
    out.append(case("Zincir: çekme alanının ön kenarı parselin ön kenarına 5 m'de bağlı; yol 2 m kayınca ikisi "
                    "birden kayar", [
                        keep5,
                        step(shift(4, (487210, 4420015), 5, ref(1, s_ref, keep=True)),
                             done(edited([4], ring_area([(F(x), F(y)) for x, y in S]), ring_area(s_kept), d_s,
                                         bonds=1)),
                             {"revision": "changed", "constraintKinds": ["distance", "distance"],
                              "parameters": params(("d1", "5"), ("d2", "5")), "fields": {"4": pts_field(s_kept)}}),
                        step(move(2, 0, -2), "$any",
                             {"revision": "changed", "constraintKinds": ["distance", "distance"],
                              "fields": {"1": pts_field(p1_follow), "2": line_field(r_down),
                                         "4": pts_field(s_follow)}},
                             command="cad.entities.transform"),
                    ]))

    # Two references that disagree once one moves.
    g_r2 = gap(R2, p1_kept[0])
    out.append(case("İki referans çelişir: kenar iki yola bağlı; yollardan biri kayınca constraint_conflict, iki "
                    "bağdan biri kaldırılınca geçer", [
                        keep5,
                        step(shift(1, at_kept, None, ref(10, (487210, 4420001), keep=True)),
                             done(edited([], ring_area(p1_kept), ring_area(p1_kept), 0, bonds=1)),
                             {"revision": "changed", "constraintKinds": ["distance", "distance"],
                              "parameters": params(("d1", "5"), ("d2", str(num(g_r2))))}),
                        step(move(2, 0, -1),
                             conflict(INCONSISTENT, [
                                 bond_in_the_way("d1", segment(2), boundary(1), 5, 6),
                                 bond_in_the_way("d2", segment(10), boundary(1), g_r2, g_r2),
                             ], f"Şu bağlardan biri kaldırılınca düzenleme geçer: d1 (Uzaklık), d2 (Uzaklık). {WAY}"),
                             {"revision": "same", "fields": {"1": pts_field(p1_kept), "2": line_field(R)}},
                             command="cad.entities.transform"),
                    ]))

    # A path's straight edge: the curb's middle 2 m from the road; it follows, its neighbours lengthen.
    d_c, _ = towards(R, C, False, 1, 2)
    c_kept = shifted(C, False, 1, d_c)
    d_c2, _ = towards(r_down, c_kept, False, 1, 2)
    c_follow = shifted(c_kept, False, 1, d_c2)
    out.append(case("Bordürün (çoklu çizgi) düz kenarı yolun kenarından 2 m'de tutulur; yol kayınca izler, komşu "
                    "kenarlar uzar", [
                        step(shift(6, (487216.5, 4420007), 2, ref(2, (487216.5, 4420004), keep=True)),
                             done(edited([6], length(C), length(c_kept), d_c, bonds=1)),
                             {"revision": "changed", "constraintKinds": ["distance"],
                              "parameters": params(("d1", "2")), "fields": {"6": pts_field(c_kept)}}),
                        step(move(2, 0, -2), "$any",
                             {"revision": "changed", "fields": {"6": pts_field(c_follow), "2": line_field(r_down)}},
                             command="cad.entities.transform"),
                    ]))

    # Refusals: nothing written.
    t = (N[1][0] - N[0][0], N[1][1] - N[0][1])
    sine = abs(t[1]) / math.hypot(*t)
    assert sine > PARALLEL
    degrees = math.degrees(math.asin(min(sine, 1.0)))
    same = {"revision": "same", "dirty": False}
    refusals = [
        ("Referans kenara paralel değil: reddedilir", shift(1, front, 5, ref(7, (487215, 4420061))),
         refused("edge_shift_refused",
                 f"Kenar referansa paralel değil (aralarında {degrees:.4f}° var); uzaklık yalnız 0,001° içinde paralel "
                 "kenarlar arasında tutulur.", "work.reference.at")),
        ("Referans düz bir kenar değil (daire): reddedilir", shift(1, front, 5, ref(9, (487353, 4420004))),
         refused("edge_shift_refused", "Referans düz bir kenar olmalı: bir çizgi ya da bir yolun veya alanın düz "
                 "kenarı.", "work.reference.id")),
        ("Yaylı kenar referansla kaydırılmaz", shift(8, (487310, 4420009), 5, ref(2, (487310, 4420004))),
         refused("edge_shift_refused", "Yaylı kenarın bir doğruya uzaklığı tek bir sayı değil; referansla yalnız düz "
                 "kenar kaydırılır.", "work.at")),
        ("Kenar kendisinin referansı olamaz", shift(1, front, 5, ref(1, front)),
         refused("edge_shift_refused", "Kenar kendisinin referansı olamaz; başka bir kenar seçin.",
                 "work.reference.at")),
        ("Referansla hedef alan verilmez", shift(1, front, None, ref(2, on_road), area=650),
         refused("invalid_input", "Referansla hedef alan verilmez: referansa uzaklığı (distance) verin.",
                 "work.area")),
        ("Referansla ne uzaklık ne Bu uzaklığı koru: reddedilir", shift(1, front, None, ref(2, on_road)),
         refused("invalid_input", "Referansla bir uzaklık (distance) verin ya da şimdiki uzaklığı Bu uzaklığı koru "
                 "(keep) ile bağlayın.", "work.distance")),
        ("Referansa eksi uzaklık reddedilir", shift(1, front, -1, ref(2, on_road)),
         refused("invalid_distance", "Referansa uzaklık eksi olamaz: kenar bulunduğu yanda kalır; sıfır ya da daha "
                 "büyük bir uzaklık yazın.", "work.distance")),
        ("Kenar referansın doğrusu üstünde: hangi yana kayacağı belli değil", shift(1, front, 5, ref(3, (487230, 4420010))),
         refused("edge_shift_refused", "Kenar referansın doğrusu üstünde; hangi yana kayacağı belli değil. Önce "
                 "kenarı Paralel kaydır ile bir yana alın.", "work.distance")),
        ("Kilitli katmandaki kenara yalnız bağ kurulmaz", shift(5, (487260, 4420010), None,
                                                                ref(2, (487260, 4420004), keep=True)),
         refused("layer_locked", "“Kilitli katman” katmanı kilitli; kenara bağ kurulmaz. Kilidi Katmanlar "
                 "panelinden açın.", "work.id")),
    ]
    for name, input, result in refusals:
        out.append(case(name, [step(input, result, same)]))
    return out


def build():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.shapes.edit",
        "commandVersion": 1,
        "title": "Paralel kaydır'ın referansı ve Bu uzaklığı koru: harita bağı (ADR 2580)",
        "note": "Beklenenler scripts/fixtures/map_bond_cases.py'den: ADR 0191'in Paralel kaydır kuralı ve ADR 2580'in "
                "bağı kesirlerle, KentOS kodu kullanılmadı; çizimin sayıları mikrometre içinde.",
        "setup": setup(),
        "cases": cases(),
    }


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"{OUT.relative_to(ROOT)} eski: python3 scripts/fixtures/map_bond_cases.py ile yeniden yazın")
        print(f"tutarlı: {OUT.relative_to(ROOT)}")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(build()['cases'])} durum)")


if __name__ == "__main__":
    main()
