"""The shared cases of associative dimensions following their objects through other commands (docs/adr/0320; plan/cad.md
CAD-ANNO-03, CAD-ANNO-07).

    python3 scripts/fixtures/dimension_follow_cases.py           # writes the file
    python3 scripts/fixtures/dimension_follow_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.dimensions.follow.json: each case makes a dimension with cad.dimensions.create, changes
what it measures with another command (cad.entities.transform, .delete, .trim, .extend, cad.dimensions.reassociate)
and says where the dimension is then. Worked out here from the ADR's rules, without KentOS code:

- A point bound to a place follows the place (docs/adr/0249 §2): a moved object's place is where the move took it.
- The rule `curve` (§1): a radius's, a diameter's or a jogged radius's centre `a` is its curve's centre; its point `b`
  is its place's point on the curve when it was there, else as far round from it, about the centre, as it was (so a
  point past an arc's end turns with the arc); a jogged radius's centre shown goes with the centre. An arc length's and
  an arc's angle's centre `c` is their arc's.
- The rule `edges` (§1): the vertex `c` is where the two edges' lines cross; each arm keeps its length and its side of
  the vertex along its edge. Parallel edges cross nowhere: the dimension stays where it was.
- A dimension the step moved by itself, its objects unchanged, lets go of its places (docs/adr/0249 §3).
- Buda (§2): a point on a cut object is looked for, where it is, on the object and the pieces made from it, and
  rebound there; one in the part cut away that was the object's end follows that end to where the cut left it; any
  other in the part cut away breaks (`vertex_gone`). Uzat moves a line's end: the end follows.
- A deleted object breaks its references (`object_gone`); the points stay.
- Merkez işareti (docs/adr/0322) follows by the rule `curve` as a radius: its centre the curve's, its point on the
  curve its place's. Eksen çizgisi by the rule `pair`: from the middle of the two edges' starts to the middle of their
  ends, the second edge turned to run the first's way. Kot's point follows its place; its elevation is the place's
  vertex's while the vertex has one, else it stays.

The coordinates are doubles exactly and the moves whole; where a transcendental value comes in (a place along a
circle, a turned point), the step says `within`.
"""
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True  # no __pycache__ in the tree
from affine_reference import assert_no_negative_zero  # noqa: E402  (beside this file)
from dimension_create_cases import (  # noqa: E402
    P, X, Y, along, dimension, edge, edge_arms, hypot, jog, layer, linear_through, place, radial, refs, signed_offset,
    t_on_arc, t_on_segment, vertex, vertex_arms,
)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
PLOT_SCALE = 500
HEIGHT = 2.5 / 1000.0 * PLOT_SCALE
TAU = 2 * math.pi

CIRCLE_C, ARC_C, R = (X + 10, Y + 10), (X + 30, Y + 10), 5
RECT = [(X + 70, Y), (X + 78, Y), (X + 78, Y + 6), (X + 70, Y + 6)]
ENTITIES = [
    {"kind": "circle", "id": 10, "layerId": "yapi", "attrs": {}, "c": P(*CIRCLE_C), "r": R},
    {"kind": "arc", "id": 11, "layerId": "yapi", "attrs": {}, "c": P(*ARC_C), "r": R, "a0": 0, "a1": math.pi / 2},
    {"kind": "line", "id": 12, "layerId": "yapi", "attrs": {}, "a": P(X, Y + 30), "b": P(X + 20, Y + 30)},
    {"kind": "line", "id": 13, "layerId": "yapi", "attrs": {}, "a": P(X + 5, Y + 25), "b": P(X + 5, Y + 40)},
    {"kind": "line", "id": 14, "layerId": "yapi", "attrs": {}, "a": P(X + 50, Y - 5), "b": P(X + 50, Y + 5)},
    {"kind": "line", "id": 15, "layerId": "yapi", "attrs": {}, "a": P(X + 40, Y), "b": P(X + 60, Y)},
    {"kind": "polygon", "id": 16, "layerId": "yapi", "attrs": {}, "pts": [P(*p) for p in RECT]},
    {"kind": "line", "id": 17, "layerId": "yapi", "attrs": {}, "a": P(X + 90, Y - 5), "b": P(X + 90, Y + 5)},
    {"kind": "line", "id": 18, "layerId": "yapi", "attrs": {}, "a": P(X + 80, Y), "b": P(X + 85, Y)},
    {"kind": "line", "id": 19, "layerId": "yapi", "attrs": {}, "a": P(X + 45, Y - 5), "b": P(X + 45, Y + 5)},
]
IDS = [e["id"] for e in ENTITIES]
NEXT = 20
SETUP = {
    "format": "kentos.document", "version": 1, "name": "İlişkili ölçünün izlemesi",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": PLOT_SCALE},
    "origin": {"x": X, "y": Y},
    "layers": [layer("yapi", "Yapı"), layer("olcu", "Ölçü")],
    "activeLayer": "yapi", "entities": ENTITIES, "styles": {"items": [], "categories": []},
}
UID = lambda i: f"$uidOf:{i}"  # noqa: E731
WITHIN = 1e-9


def make(measure, at, **extra):
    """Makes the dimension (cad.dimensions.create, this file's command)."""
    return {"op": "execute", "input": {"measure": measure, "at": P(*at), "layerId": "olcu", **extra}, "result": "$any"}


def run(command, inp, expect, within=False, undo=None):
    """Another command's step, then where the dimensions are."""
    step = {"op": "execute", "command": command, "input": inp, "result": "$any", "expect": {"entities": expect}}
    if within:
        step["within"] = WITHIN
    return step


def move(uids, dx, dy):
    return {"uids": [UID(u) for u in uids], "transform": {"kind": "move", "dx": dx, "dy": dy}}


def on_circle(c, r, t):
    return (c[0] + r * math.cos(t * TAU), c[1] + r * math.sin(t * TAU))


def turned(b_was, q_was, centre_was, q, c):
    """The rule `curve`'s point on the curve: the place's point when it was there, else as far round from it."""
    if q_was == b_was:
        return q
    off = math.atan2(b_was[1] - centre_was[1], b_was[0] - centre_was[0]) - math.atan2(q_was[1] - centre_was[1], q_was[0] - centre_was[0])
    at = math.atan2(q[1] - c[1], q[0] - c[0]) + off
    r = hypot(q[0] - c[0], q[1] - c[1])
    return (c[0] + r * math.cos(at), c[1] + r * math.sin(at))


def radius_cases():
    out = []
    # A radius of the circle, its point at a place along it.
    at = (X + 16, Y + 18)
    b, off = radial(CIRCLE_C, R, at)
    t = t_on_arc(CIRCLE_C, 0.0, TAU, b)
    bond = {"b": along(10, t), "rule": "curve"}
    made = dimension(NEXT, CIRCLE_C, b, off, kind="radius", refs=bond)
    moved_c = (CIRCLE_C[0] + 2, CIRCLE_C[1] + 1)
    q = on_circle(moved_c, R, t)
    out.append({"name": "Yarıçap: daire taşınınca merkez ve nokta onunla gider; tek geri almayla eski yerine döner", "steps": [
        make({"kind": "radius", "of": edge(10, (X + 15, Y + 10))}, at),
        run("cad.entities.transform", move([10], 2, 1),
            {str(NEXT): dimension(NEXT, moved_c, turned(b, on_circle(CIRCLE_C, R, t), CIRCLE_C, q, moved_c), off, kind="radius", refs=bond)},
            within=True),
        {"op": "undo", "returns": "Taşı", "expect": {"entities": {str(NEXT): made}}}]})
    # Scaled about its centre: the point twice as far.
    q2 = on_circle(CIRCLE_C, 2 * R, t)
    out.append({"name": "Yarıçap: daire merkezinden iki kat ölçeklenince nokta yeni çemberde", "steps": [
        make({"kind": "radius", "of": edge(10, (X + 15, Y + 10))}, at),
        run("cad.entities.transform", {"uids": [UID(10)], "transform": {"kind": "scale", "center": P(*CIRCLE_C), "factor": 2}},
            {str(NEXT): dimension(NEXT, CIRCLE_C, turned(b, on_circle(CIRCLE_C, R, t), CIRCLE_C, q2, CIRCLE_C), off, kind="radius", refs=bond)},
            within=True)]})
    # A diameter of the arc, its point past the arc's end: it turns with the arc.
    at = (X + 24, Y + 18)
    b, off = radial(ARC_C, R, at)
    t = t_on_arc(ARC_C, 0.0, math.pi / 2, b)
    bond = {"b": along(11, t), "rule": "curve"}
    q_was = (ARC_C[0] + R * math.cos(t * math.pi / 2), ARC_C[1] + R * math.sin(t * math.pi / 2))
    q = (ARC_C[0] + R * math.cos(math.pi / 2 + t * math.pi / 2), ARC_C[1] + R * math.sin(math.pi / 2 + t * math.pi / 2))
    out.append({"name": "Çap: noktası yayın ötesinde; yay merkezinde 90° dönünce nokta da 90° döner", "steps": [
        make({"kind": "diameter", "of": edge(11, (X + 34, Y + 13))}, at),
        run("cad.entities.transform", {"uids": [UID(11)], "transform": {"kind": "rotate", "center": P(*ARC_C), "angle": math.pi / 2}},
            {str(NEXT): dimension(NEXT, ARC_C, turned(b, q_was, ARC_C, q, ARC_C), off, kind="diameter", refs=bond)}, within=True)]})
    # A jogged radius: the centre shown goes with the centre.
    on, shown, at = (X + 20, Y + 10), (X - 5, Y + 12), (X + 5, Y + 13)
    b = (CIRCLE_C[0] + R, CIRCLE_C[1])
    off = jog(CIRCLE_C, b, shown, at)
    out.append({"name": "Kırıklı yarıçap: daire taşınınca gösterilen merkez de aynı kadar gider", "steps": [
        make({"kind": "jogged", "of": edge(10, (X + 15, Y + 10)), "shownCentre": P(*shown), "on": P(*on)}, at),
        run("cad.entities.transform", move([10], 3, 0),
            {str(NEXT): dimension(NEXT, (CIRCLE_C[0] + 3, CIRCLE_C[1]), (b[0] + 3, b[1]), off, kind="jogged", c=(shown[0] + 3, shown[1]),
                                  refs={"b": along(10, 0.0), "rule": "curve"})})]})
    # Moved by itself: it lets go.
    at = (X + 16, Y + 18)
    b, off = radial(CIRCLE_C, R, at)
    out.append({"name": "Yarıçap ölçüsü kendi başına taşınınca bağını bırakır; kuralıyla birlikte", "steps": [
        make({"kind": "radius", "of": edge(10, (X + 15, Y + 10))}, at),
        run("cad.entities.transform", move([NEXT], 1, 0),
            {str(NEXT): dimension(NEXT, (CIRCLE_C[0] + 1, CIRCLE_C[1]), (b[0] + 1, b[1]), off, kind="radius")})]})
    out.append({"name": "Yarıçap: Yeniden bağla ile serbest bırakılan nokta bağı ve kuralı kaldırır", "steps": [
        make({"kind": "radius", "of": edge(10, (X + 15, Y + 10))}, at),
        run("cad.dimensions.reassociate", {"uid": UID(NEXT), "points": [{"point": "b", "to": None}]},
            {str(NEXT): dimension(NEXT, CIRCLE_C, b, off, kind="radius")})]})
    # The arc deleted: the reference breaks, the points stay.
    at = (X + 24, Y + 18)
    b, off = radial(ARC_C, R, at)
    t = t_on_arc(ARC_C, 0.0, math.pi / 2, b)
    out.append({"name": "Çap: yay silinince bağ kopar (nesne silindi), noktalar yerinde kalır", "steps": [
        {"op": "captureUid", "as": "yay", "id": 11},
        make({"kind": "diameter", "of": edge(11, (X + 34, Y + 13))}, at),
        run("cad.entities.delete", {"uids": [UID(11)]},
            {str(NEXT): dimension(NEXT, ARC_C, b, off, kind="diameter",
                                  refs={"b": {**along(11, t), "uid": "$uid:yay", "broken": "object_gone"}, "rule": "curve"})})]})
    return out


def arc_cases():
    out = []
    at = (X + 36, Y + 18)
    a, b = (ARC_C[0] + R, ARC_C[1]), (ARC_C[0] + R * math.cos(math.pi / 2), ARC_C[1] + R * math.sin(math.pi / 2))
    bond = {"a": along(11, 0.0), "b": along(11, 1.0), "rule": "curve"}
    out.append({"name": "Yay uzunluğu: yay taşınınca uçları ve merkezi onunla gider", "steps": [
        make({"kind": "arcLength", "of": edge(11, (X + 34, Y + 13))}, at),
        run("cad.entities.transform", move([11], 0, 2),
            {str(NEXT): dimension(NEXT, (a[0], a[1] + 2), (b[0], b[1] + 2), hypot(6, 8) - R, kind="arcLength", c=(ARC_C[0], ARC_C[1] + 2),
                                  refs=bond)})]})
    out.append({"name": "Açı (Yaydan): yay merkezinden iki kat ölçeklenince uçları yeni yayda", "steps": [
        make({"kind": "angleArc", "of": edge(11, (X + 34, Y + 13))}, at),
        run("cad.entities.transform", {"uids": [UID(11)], "transform": {"kind": "scale", "center": P(*ARC_C), "factor": 2}},
            {str(NEXT): dimension(NEXT, (ARC_C[0] + 2 * R, ARC_C[1]), (ARC_C[0] + 2 * R * math.cos(math.pi / 2), ARC_C[1] + 2 * R),
                                  hypot(6, 8), kind="angular", c=ARC_C, refs=bond)}, within=True)]})
    out.append({"name": "Yay uzunluğu: yay silinince iki ucun bağı kopar, ölçü yerinde kalır", "steps": [
        {"op": "captureUid", "as": "yay", "id": 11},
        make({"kind": "arcLength", "of": edge(11, (X + 34, Y + 13))}, at),
        run("cad.entities.delete", {"uids": [UID(11)]},
            {str(NEXT): dimension(NEXT, a, b, hypot(6, 8) - R, kind="arcLength", c=ARC_C,
                                  refs={"a": {**bond["a"], "uid": "$uid:yay", "broken": "object_gone"},
                                        "b": {**bond["b"], "uid": "$uid:yay", "broken": "object_gone"}, "rule": "curve"})})]})
    return out


def edges_cases():
    out = []
    first, second, at = (X + 15, Y + 30), (X + 5, Y + 38), (X + 8, Y + 33)
    e12, e13 = ((X, Y + 30), (X + 20, Y + 30)), ((X + 5, Y + 25), (X + 5, Y + 40))
    c, a, b = edge_arms((e12[0], e12[1], first), (e13[0], e13[1], second), at)
    off = hypot(at[0] - c[0], at[1] - c[1])
    bond = {"a": along(12, t_on_segment(*e12, first)), "b": along(13, t_on_segment(*e13, second)), "rule": "edges"}
    measure = {"kind": "angle", "first": edge(12, first), "second": edge(13, second)}

    def angle(a, b, c, refs_=bond):
        return dimension(NEXT, a, b, off, kind="angular", c=c, refs=refs_)

    out.append({"name": "Açı (kenarlardan): ikinci kenar 3 m kayınca köşe kesişime gider, kollar boylarıyla kenarlarda", "steps": [
        make(measure, at),
        run("cad.entities.transform", move([13], 3, 0), {str(NEXT): angle((a[0] + 3, a[1]), (b[0] + 3, b[1]), (c[0] + 3, c[1]))}, within=True)]})
    out.append({"name": "Açı (kenarlardan): birinci kenar 2 m yukarı kayınca köşe ve kollar onunla", "steps": [
        make(measure, at),
        run("cad.entities.transform", move([12], 0, 2), {str(NEXT): angle((a[0], a[1] + 2), (b[0], b[1] + 2), (c[0], c[1] + 2))}, within=True)]})
    out.append({"name": "Açı (kenarlardan): kenarlar paralel olunca kesişim yok, ölçü yerinde kalır", "steps": [
        make(measure, at),
        run("cad.entities.transform", {"uids": [UID(12)], "transform": {"kind": "rotate", "center": P(*c), "angle": math.pi / 2}},
            {str(NEXT): angle(a, b, c)}, within=True)]})
    out.append({"name": "Açı (kenarlardan): kenarın biri silinince onun bağı kopar, ölçü yerinde kalır", "steps": [
        {"op": "captureUid", "as": "kenar", "id": 13},
        make(measure, at),
        run("cad.entities.delete", {"uids": [UID(13)]},
            {str(NEXT): angle(a, b, c, {**bond, "b": {**bond["b"], "uid": "$uid:kenar", "broken": "object_gone"}})}, within=True)]})
    # Köşeden by the area's corners.
    at = (X + 72, Y + 2)
    vc, va, vb = vertex_arms(RECT[0], RECT[1], RECT[3], at)
    out.append({"name": "Açı (köşeden): alan taşınınca köşe ve iki kol köşeleriyle gider", "steps": [
        make({"kind": "angleVertex", "c": vertex(16, 0), "a": vertex(16, 1), "b": vertex(16, 3)}, at),
        run("cad.entities.transform", move([16], 1, 2),
            {str(NEXT): dimension(NEXT, (va[0] + 1, va[1] + 2), (vb[0] + 1, vb[1] + 2), hypot(2, 2), kind="angular", c=(vc[0] + 1, vc[1] + 2),
                                  refs={"a": {"uid": UID(16), "place": vertex(16, 1)["place"]},
                                        "b": {"uid": UID(16), "place": vertex(16, 3)["place"]},
                                        "c": {"uid": UID(16), "place": vertex(16, 0)["place"]}})})]})
    return out


def chain_cases():
    out = []
    base_at = (X + 74, Y - 2)
    base = {"a": RECT[0], "b": RECT[1], "offset": signed_offset(RECT[0], RECT[1], base_at), "kind": None, "angle": None}
    base_in = make({"kind": "aligned", "a": vertex(16, 0), "b": vertex(16, 1)}, base_at)
    g = linear_through(base, RECT[1], (X + 82, Y), 0.0)
    out.append({"name": "Zincir ölçü: ilk noktası temelin ucunu izler; alan kayınca o da kayar, serbest ucu kalır", "steps": [
        base_in,
        make({"kind": "continue", "base": UID(NEXT), "to": {"kind": "at", "at": P(X + 82, Y)}}, (X, Y)),
        run("cad.entities.transform", move([16], 0, 1),
            {str(NEXT + 1): dimension(NEXT + 1, (RECT[1][0], RECT[1][1] + 1), (X + 82, Y), g["offset"], kind="linear", angle=g["angle"],
                                      refs=refs(a={"uid": UID(16), "place": vertex(16, 1)["place"]}))})]})
    g = linear_through(base, RECT[0], RECT[2], 1.0 * 3.0 * HEIGHT)
    out.append({"name": "Baz ölçü: ilk noktası temelin ilk noktasını izler; iki ucu da alanla gider", "steps": [
        base_in,
        make({"kind": "baseline", "base": UID(NEXT), "to": vertex(16, 2)}, (X, Y)),
        run("cad.entities.transform", move([16], 1, 2),
            {str(NEXT + 1): dimension(NEXT + 1, (RECT[0][0] + 1, RECT[0][1] + 2), (RECT[2][0] + 1, RECT[2][1] + 2), g["offset"], kind="linear",
                                      angle=g["angle"], refs=refs(a={"uid": UID(16), "place": vertex(16, 0)["place"]},
                                                                  b={"uid": UID(16), "place": vertex(16, 2)["place"]}))})]})
    return out


def trim_cases():
    out = []
    ends = {"kind": "aligned", "a": place(15, "start"), "b": place(15, "end")}
    at = (X + 50, Y + 3)
    bond = {"a": {"uid": UID(15), "place": {"kind": "start"}}, "b": {"uid": UID(15), "place": {"kind": "end"}}}
    out.append({"name": "Buda: kesilen uca bağlı nokta çizginin yeni ucuna gider", "steps": [
        make(ends, at),
        run("cad.entities.trim", {"picks": [{"uid": UID(15), "at": P(X + 55, Y)}], "boundaries": {"kind": "objects", "uids": [UID(14)]}},
            {str(NEXT): dimension(NEXT, (X + 40, Y), (X + 50, Y), 3, refs=bond)})]})
    mid = {"kind": "aligned", "a": place(15, "along", part=0, ring=0, index=0, t=0.35), "b": place(15, "start")}
    out.append({"name": "Buda ortadan: uzak uca bağlı nokta yeni parçaya bağlanır; kesilen kısımdaki yer kopar (köşe yok)", "steps": [
        make(ends, at),
        make(mid, (X + 44, Y + 2)),
        run("cad.entities.trim", {"picks": [{"uid": UID(15), "at": P(X + 47, Y)}],
                                  "boundaries": {"kind": "objects", "uids": [UID(14), UID(19)]}},
            {str(NEXT): dimension(NEXT, (X + 40, Y), (X + 60, Y), 3,
                                  refs={"a": bond["a"], "b": {"uid": UID(NEXT + 2), "place": {"kind": "end"}}}),
             str(NEXT + 1): dimension(NEXT + 1, (X + 47, Y), (X + 40, Y), signed_offset((X + 47, Y), (X + 40, Y), (X + 44, Y + 2)),
                                      refs={"a": {"uid": UID(15), "place": mid["a"]["place"], "broken": "vertex_gone"}, "b": bond["a"]})})]})
    ext = {"kind": "aligned", "a": place(18, "start"), "b": place(18, "end")}
    out.append({"name": "Uzat: uzayan uca bağlı nokta yeni uca gider", "steps": [
        make(ext, (X + 82, Y + 2)),
        run("cad.entities.extend", {"picks": [{"uid": UID(18), "at": P(X + 84, Y)}], "boundaries": {"kind": "objects", "uids": [UID(17)]}},
            {str(NEXT): dimension(NEXT, (X + 80, Y), (X + 90, Y), 2,
                                  refs={"a": {"uid": UID(18), "place": {"kind": "start"}}, "b": {"uid": UID(18), "place": {"kind": "end"}}})})]})
    return out


MARK_LOOK = {"extLineType": "dashdot"}


def mark_cases():
    out = []
    # A centre mark of the circle: its point east of the centre, at the place t = 0.
    east = lambda c, r=R: (c[0] + r, c[1])  # noqa: E731
    bond = {"b": along(10, 0), "rule": "curve"}
    moved_c = (CIRCLE_C[0] + 2, CIRCLE_C[1] + 1)
    out.append({"name": "Merkez işareti: daire taşınınca merkez ve işaretin noktası onunla", "steps": [
        make({"kind": "centerMark", "of": edge(10, (X + 15, Y + 10))}, (X, Y)),
        run("cad.entities.transform", move([10], 2, 1),
            {str(NEXT): dimension(NEXT, moved_c, east(moved_c), HEIGHT, kind="centerMark", look=MARK_LOOK, refs=bond)})]})
    out.append({"name": "Merkez işareti: daire iki kat büyüyünce eksen çizgileri yeni çemberden taşar", "steps": [
        make({"kind": "centerMark", "of": edge(10, (X + 15, Y + 10))}, (X, Y)),
        run("cad.entities.transform", {"uids": [UID(10)], "transform": {"kind": "scale", "center": P(*CIRCLE_C), "factor": 2}},
            {str(NEXT): dimension(NEXT, CIRCLE_C, east(CIRCLE_C, 2 * R), HEIGHT, kind="centerMark", look=MARK_LOOK, refs=bond)})]})
    # A centre mark of the arc, its point at the arc's start: the arc turned 90° about its centre turns it.
    out.append({"name": "Merkez işareti: yay merkezinde 90° dönünce işaret de döner", "steps": [
        make({"kind": "centerMark", "of": edge(11, (X + 34, Y + 13))}, (X, Y)),
        run("cad.entities.transform", {"uids": [UID(11)], "transform": {"kind": "rotate", "center": P(*ARC_C), "angle": math.pi / 2}},
            {str(NEXT): dimension(NEXT, ARC_C, (ARC_C[0] + R * math.cos(math.pi / 2), ARC_C[1] + R * math.sin(math.pi / 2)), HEIGHT,
                                  kind="centerMark", look=MARK_LOOK, refs={"b": along(11, 0), "rule": "curve"})}, within=True)]})
    # An axis between the lines 12 and 15 (parallel, the same way).
    first, second = ((X, Y + 30), (X + 20, Y + 30)), ((X + 40, Y), (X + 60, Y))
    pair = {"a": along(12, t_on_segment(*first, (X + 10, Y + 30))), "b": along(15, t_on_segment(*second, (X + 44, Y))), "rule": "pair"}
    mid = lambda p, q: ((p[0] + q[0]) / 2, (p[1] + q[1]) / 2)  # noqa: E731
    out.append({"name": "Eksen çizgisi: ikinci kenar 4 m yukarı kayınca eksen 2 m kayar", "steps": [
        make({"kind": "centerLine", "first": edge(12, (X + 10, Y + 30)), "second": edge(15, (X + 44, Y))}, (X, Y)),
        run("cad.entities.transform", move([15], 0, 4),
            {str(NEXT): dimension(NEXT, mid(first[0], (X + 40, Y + 4)), mid(first[1], (X + 60, Y + 4)), HEIGHT, kind="centerLine",
                                  look=MARK_LOOK, refs=pair)})]})
    out.append({"name": "Eksen çizgisi: kenar ters çevrilince eksen yine uçların ortalarından", "steps": [
        make({"kind": "centerLine", "first": edge(12, (X + 10, Y + 30)), "second": edge(15, (X + 44, Y))}, (X, Y)),
        run("cad.entities.reverse", {"uids": [UID(15)]},
            {str(NEXT): dimension(NEXT, mid(first[0], second[0]), mid(first[1], second[1]), HEIGHT, kind="centerLine",
                                  look=MARK_LOOK, refs={**pair, "b": along(15, 1 - pair["b"]["place"]["t"])})})]})
    out.append({"name": "Eksen çizgisi: kenarın biri silinince bağı kopar, eksen yerinde kalır", "steps": [
        {"op": "captureUid", "as": "kenar", "id": 15},
        make({"kind": "centerLine", "first": edge(12, (X + 10, Y + 30)), "second": edge(15, (X + 44, Y))}, (X, Y)),
        run("cad.entities.delete", {"uids": [UID(15)]},
            {str(NEXT): dimension(NEXT, mid(first[0], second[0]), mid(first[1], second[1]), HEIGHT, kind="centerLine",
                                  look=MARK_LOOK, refs={**pair, "b": {**pair["b"], "uid": "$uid:kenar", "broken": "object_gone"}})})]})
    # A level at the line 12's start, its elevation given: the vertex given one, the level takes it; the line moved, it
    # goes with it and keeps it.
    start = {"uid": UID(12), "place": {"kind": "start"}}
    z_line = lambda zs: {"operation": "elevation", "changes": [{"kind": "update", "uid": UID(12), "geometry": {  # noqa: E731
        "kind": "line", "a": P(*first[0]), "b": P(*first[1]), "zs": zs}}]}
    out.append({"name": "Kot: köşeye kot verilince işaret köşenin kotunu alır; çizgi taşınınca onunla gider", "steps": [
        make({"kind": "elevation", "a": place(12, "start"), "z": 5}, (X - 2, Y + 33)),
        run("cad.entities.edit", z_line([7.5, None]),
            {str(NEXT): dimension(NEXT, first[0], (X - 2, Y + 33), 0, kind="elevation", za=7.5, refs={"a": start})}),
        run("cad.entities.transform", move([12], 1, 2),
            {str(NEXT): dimension(NEXT, (first[0][0] + 1, first[0][1] + 2), (X - 2, Y + 33), 0, kind="elevation", za=7.5,
                                  refs={"a": start})})]})
    out.append({"name": "Kot: koordinat noktası; çizgiler taşınınca kımıldamaz", "steps": [
        make({"kind": "elevation", "a": {"kind": "at", "at": P(X + 100, Y + 50)}, "z": 3.2}, (X + 103, Y + 52)),
        run("cad.entities.transform", move([12], 1, 2),
            {str(NEXT): dimension(NEXT, (X + 100, Y + 50), (X + 103, Y + 52), 0, kind="elevation", za=3.2)})]})
    return out


def build():
    return {
        "format": "kentos.command-cases", "version": 1, "command": "cad.dimensions.create", "commandVersion": 1,
        "title": "İlişkili ölçünün izlemesi",
        "note": "Written by scripts/fixtures/dimension_follow_cases.py from the rules of docs/adr/0249, 0320 and 0322; do not edit by hand.",
        "setup": SETUP, "cases": radius_cases() + arc_cases() + edges_cases() + chain_cases() + trim_cases() + mark_cases(),
    }


def main() -> int:
    doc = build()
    assert_no_negative_zero(doc, "cad.dimensions.follow")
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    path = OUT / "cad.dimensions.follow.json"
    if "--check" in sys.argv[1:]:
        if not path.exists() or path.read_text(encoding="utf-8") != text:
            print(f"{path.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"İlişkili ölçünün izleme durumları güncel: {len(doc['cases'])} durum.")
        return 0
    path.write_text(text, encoding="utf-8")
    print(f"yazıldı: {path.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
