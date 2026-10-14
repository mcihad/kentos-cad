"""The shared cases of the product command cad.entities.select (docs/adr/0275).

    python3 scripts/fixtures/entities_select_cases.py           # writes the file
    python3 scripts/fixtures/entities_select_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.select.json. The selections follow the rules of docs/adr/0029, 0141, 0187
and 0275, written here by hand with exact fractions, no KentOS code:

- Pencere: the object's box wholly inside the box (edges included). Kesişen: inside, or touching it: a point of its
  outline inside the box, the box's middle inside its area (an area's, not a circle's), or an edge across a side.
- Çit: an edge of the object crossing the path; a point within the tolerance of it.
- Çokgen: İçindekiler every point and every edge's piece inside the ring; Kesişenler inside or touching (an edge
  meeting the ring, a point of it inside, the ring inside its area); Dışındakiler every object shown not touching.
- Daire: wholly inside when its box's corners are; touching when an edge comes within the radius or the centre lies
  in its area.
- İçeren alan: the closed shapes around the point, the smallest area first. A click: the points and edges within
  the tolerance by their distance, then the areas around it by their size.
- Only objects shown count (a hidden layer's never; a locked layer's do); the kinds, then the layers, filter after
  the region, each counting what it leaves out; the objects given that are not shown are counted as hidden.

The drawing is chosen so that no case rests on a point on a boundary or a rounding: every decision is clear of it.
"""
import json
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.entities.select.json"

E0, N0 = 487000, 4420000
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}


def P(x, y):
    return {"x": float(E0 + x), "y": float(N0 + y)}


def layer(i, name, visible=True, locked=False, kind="layer", children=()):
    return {"id": i, "name": name, "type": kind, "visible": visible, "locked": locked, "expanded": True,
            "style": STYLE, "children": list(children)}


def base(kind, i, lay, color=None):
    out = {"kind": kind, "id": i, "layerId": lay, "attrs": {}}
    if color is not None:
        out["color"] = color
    return out


RED = "#E5484D"
# Local coordinates (metres from E0, N0) beside each object, for the reference.
OBJECTS = [
    (1, "polygon", "yapi", None, [(0, 0), (10, 0), (10, 10), (0, 10)]),
    (2, "polygon", "yapi", RED, [(2, 2), (4, 2), (4, 4), (2, 4)]),
    (3, "polygon", "yapi", RED, [(20, 0), (30, 0), (30, 10), (20, 10)]),
    (4, "line", "yol", None, [(-5, 5), (15, 5)]),
    (5, "line", "yol", None, [(40, 0), (50, 0)]),
    (6, "circle", "yapi", None, ((60, 5), 3)),
    (7, "point", "yapi", None, (5, 20)),
    (8, "polygon", "gizli", None, [(0, 30), (10, 30), (10, 40), (0, 40)]),
    (9, "line", "kilitli", None, [(20, 20), (30, 20)]),
    (10, "text", "yapi", None, (40, 30)),
]
SHOWN_LAYERS = {"yapi", "yol", "kilitli"}


def entity(o):
    i, kind, lay, color, g = o
    e = base(kind, i, lay, color)
    if kind == "polygon":
        e["pts"] = [P(*q) for q in g]
    elif kind == "line":
        e["a"], e["b"] = P(*g[0]), P(*g[1])
    elif kind == "circle":
        e["c"], e["r"] = P(*g[0]), float(g[1])
    elif kind == "point":
        e["p"] = P(*g)
    elif kind == "text":
        e.update({"p": P(*g), "text": "Yazı", "height": 1.0, "rotation": 0})
    return e


SETUP = {
    "format": "kentos.document", "version": 1, "name": "Nesne seç",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                 "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
    "origin": P(0, 0),
    "layers": [layer("yapi", "Yapı"), layer("yol", "Yol"), layer("gizli", "Gizli", visible=False),
               layer("koruma", "Koruma", locked=True, kind="group", children=[layer("kilitli", "Sit alanı")])],
    "activeLayer": "yapi", "entities": [entity(o) for o in OBJECTS], "styles": {"items": [], "categories": []},
}
IDS = [o[0] for o in OBJECTS]
SAME = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
BY_ID = {o[0]: o for o in OBJECTS}

# ── The reference's geometry, exact ─────────────────────────────────────────


def fr(q):
    return (F(q[0]), F(q[1]))


def orient(a, b, c):
    v = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    return (v > 0) - (v < 0)


def on_seg(a, b, p):
    return orient(a, b, p) == 0 and min(a[0], b[0]) <= p[0] <= max(a[0], b[0]) and \
        min(a[1], b[1]) <= p[1] <= max(a[1], b[1])


def cross(a, b, c, d):
    o1, o2, o3, o4 = orient(a, b, c), orient(a, b, d), orient(c, d, a), orient(c, d, b)
    if o1 != o2 and o3 != o4:
        return True
    return on_seg(a, b, c) or on_seg(a, b, d) or on_seg(c, d, a) or on_seg(c, d, b)


def inside_ring(p, ring):
    """Strictly inside (the cases keep points off the boundaries)."""
    n, c = len(ring), False
    for i in range(n):
        a, b = ring[i], ring[(i + 1) % n]
        if (a[1] > p[1]) != (b[1] > p[1]):
            x = a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1])
            if p[0] < x:
                c = not c
    return c


def area(ring):
    return abs(sum(ring[i][0] * ring[(i + 1) % len(ring)][1] - ring[(i + 1) % len(ring)][0] * ring[i][1]
                   for i in range(len(ring)))) / 2


def edges(o):
    _, kind, _, _, g = o
    if kind == "polygon":
        r = [fr(q) for q in g]
        return [(r[i], r[(i + 1) % len(r)]) for i in range(len(r))]
    if kind == "line":
        return [(fr(g[0]), fr(g[1]))]
    return []


def points(o):
    """The object's outline points (a circle's: its four quadrants and none else is needed by the cases)."""
    _, kind, _, _, g = o
    if kind in ("polygon", "line"):
        return [fr(q) for q in g]
    if kind == "point":
        return [fr(g)]
    if kind == "circle":
        (cx, cy), r = g
        return [(F(cx + r), F(cy)), (F(cx), F(cy + r)), (F(cx - r), F(cy)), (F(cx), F(cy - r))]
    raise AssertionError(f"no outline for {kind}")


def box(o):
    _, kind, _, _, g = o
    if kind == "circle":
        (cx, cy), r = g
        return (F(cx - r), F(cy - r), F(cx + r), F(cy + r))
    pts = points(o)
    return (min(p[0] for p in pts), min(p[1] for p in pts), max(p[0] for p in pts), max(p[1] for p in pts))


def ring_of(o):
    return [fr(q) for q in o[4]] if o[1] == "polygon" else None


def shown():
    return [o for o in OBJECTS if o[2] in SHOWN_LAYERS]


def spatial():
    """The objects a region reaches: shown, the text kept out of every region (the cases stay far from it)."""
    return [o for o in shown() if o[1] != "text"]


def rect_sides(r):
    x0, y0, x1, y1 = r
    c = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
    return [(c[i], c[(i + 1) % 4]) for i in range(4)]


def in_rect(r, crossing):
    x0, y0, x1, y1 = r
    out = []
    for o in spatial():
        b = box(o)
        if b[0] >= x0 and b[2] <= x1 and b[1] >= y0 and b[3] <= y1:
            out.append(o[0])
            continue
        if not crossing:
            continue
        inr = any(x0 <= p[0] <= x1 and y0 <= p[1] <= y1 for p in points(o))
        mid = ((x0 + x1) / 2, (y0 + y1) / 2)
        ring = ring_of(o)
        across = any(cross(a, b2, c, d) for a, b2 in edges(o) for c, d in rect_sides(r))
        if o[1] == "circle":
            assert not across and not inr, "a crossing case near the circle"
        if inr or (ring and inside_ring(mid, ring)) or across:
            out.append(o[0])
    return out


def in_fence(path, tol):
    segs = [(fr(path[i]), fr(path[i + 1])) for i in range(len(path) - 1)]
    out = []
    for o in spatial():
        if o[1] == "point":
            p = fr(o[4])
            d = min(seg_dist(p, a, b) for a, b in segs)
            if d <= tol:
                out.append(o[0])
        elif o[1] == "circle":
            assert all(seg_dist(fr(o[4][0]), a, b) > o[4][1] + 1 for a, b in segs), "a fence near the circle"
        elif any(cross(a, b, c, d) for a, b in edges(o) for c, d in segs):
            out.append(o[0])
    return out


def seg_dist(p, a, b):
    """Distance from p to the segment, as a float (the cases keep it clear of the tolerance)."""
    dx, dy = b[0] - a[0], b[1] - a[1]
    t = max(F(0), min(F(1), ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)))
    q = (a[0] + t * dx, a[1] + t * dy)
    return float(((p[0] - q[0]) ** 2 + (p[1] - q[1]) ** 2)) ** 0.5


def polygon_select(ring_local, mode):
    ring = [fr(q) for q in ring_local]
    sides = [(ring[i], ring[(i + 1) % len(ring)]) for i in range(len(ring))]
    inside, touching = [], []
    for o in spatial():
        if o[1] == "circle":
            c = fr(o[4][0])
            assert not inside_ring(c, ring), "a polygon case around the circle"
            continue
        meets = any(cross(a, b, c, d) for a, b in edges(o) for c, d in sides)
        pts_in = [inside_ring(p, ring) for p in points(o)]
        own = ring_of(o)
        if all(pts_in) and not meets:
            inside.append(o[0])
            touching.append(o[0])
        elif meets or any(pts_in) or (own and inside_ring(ring[0], own)):
            touching.append(o[0])
    if mode == "inside":
        return inside
    if mode == "crossing":
        return touching
    return [o[0] for o in shown() if o[0] not in touching]


def in_circle(c_local, r, crossing):
    c = fr(c_local)
    out = []
    for o in spatial():
        b = box(o)
        corners = [(b[0], b[1]), (b[2], b[1]), (b[2], b[3]), (b[0], b[3])]
        if all((q[0] - c[0]) ** 2 + (q[1] - c[1]) ** 2 <= r * r for q in corners):
            out.append(o[0])
            continue
        if not crossing:
            continue
        ring = ring_of(o)
        near = any(seg_dist(c, a, b2) <= r for a, b2 in edges(o)) or \
            any((p[0] - c[0]) ** 2 + (p[1] - c[1]) ** 2 <= r * r for p in points(o))
        if near or (ring and inside_ring(c, ring)):
            out.append(o[0])
    return out


def containing(p_local):
    p = fr(p_local)
    found = []
    for o in spatial():
        if o[1] == "polygon" and inside_ring(p, ring_of(o)):
            found.append((area(ring_of(o)), o[0]))
        elif o[1] == "circle":
            (cx, cy), r = o[4]
            if (p[0] - cx) ** 2 + (p[1] - cy) ** 2 <= r * r:
                found.append((F(r * r) * F(355, 113), o[0]))
    return [i for _, i in sorted(found, key=lambda f: f[0])]


def hits(p_local, tol):
    p = fr(p_local)
    near, areas = [], []
    for o in spatial():
        d = min([seg_dist(p, a, b) for a, b in edges(o)] or [float("inf")])
        if o[1] == "point":
            # A point is reached from half as far again (docs/adr/0029).
            d = float((p[0] - o[4][0]) ** 2 + (p[1] - o[4][1]) ** 2) ** 0.5
            reach = tol * 1.5
        else:
            reach = tol
        if d <= reach:
            near.append((d, o[0]))
        elif o[1] == "polygon" and inside_ring(p, ring_of(o)):
            areas.append((area(ring_of(o)), o[0]))
    return [i for _, i in sorted(near, key=lambda f: f[0])] + [i for _, i in sorted(areas, key=lambda f: f[0])]


def similar(examples, crit):
    """Shown objects equal to an example in every criterion that is on (no object here has a symbol)."""
    def k(o):
        _, kind, lay, color, _ = o
        return tuple(v for on, v in ((crit["kind"], kind), (crit["layer"], lay), (crit["color"], color),
                                     (crit["symbol"], None)) if on)
    wanted = {k(BY_ID[i]) for i in examples}
    return [o[0] for o in shown() if k(o) in wanted]


def filtered(ids, kinds=None, layers=None, hidden=0):
    out, lk, ll = [], 0, 0
    for i in ids:
        o = BY_ID[i]
        if kinds is not None and o[1] not in kinds:
            lk += 1
            continue
        if layers is not None and o[2] not in layers:
            ll += 1
            continue
        out.append(i)
    return out, {"kinds": lk, "layers": ll, "hidden": hidden}


# ── The cases ───────────────────────────────────────────────────────────────


def done(ids, left):
    return {"status": "completed",
            "output": {"uids": [f"$uidOf:{i}" for i in ids], "leftOut": left, "revision": "$current"},
            "warnings": []}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def step(inp, result, op="execute", **more):
    s = {"op": op, "input": inp, "result": result, "expect": SAME}
    s.update(more)
    return s


def case(name, *steps):
    return {"name": name, "steps": list(steps)}


def uid(i):
    return f"$uidOf:{i}"


def rect(a, b):
    return (F(min(a[0], b[0])), F(min(a[1], b[1])), F(max(a[0], b[0])), F(max(a[1], b[1])))


def query(name, region, ids, kinds=None, layers=None, hidden=0, op="execute"):
    inp = {"region": region}
    if kinds is not None:
        inp["kinds"] = kinds
    if layers is not None:
        inp["layers"] = layers
    got, left = filtered(ids, kinds, layers, hidden)
    return case(name, step(inp, done(got, left), op=op))


NOT_FINITE = "değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin."


def cases():
    out = []
    a, b = (-1, -1), (11, 11)
    out.append(query("pencere: kutunun tamamen içindekiler", {"kind": "window", "a": P(*a), "b": P(*b)},
                     in_rect(rect(a, b), False)))
    out.append(query("kesişen: içindekiler ve kesen çizgi", {"kind": "crossing", "a": P(*a), "b": P(*b)},
                     in_rect(rect(a, b), True)))
    a, b = (5, 6), (6, 7)
    out.append(query("kesişen: kutunun ortası alanın içinde", {"kind": "crossing", "a": P(*a), "b": P(*b)},
                     in_rect(rect(a, b), True)))
    out.append(query("pencere: içinde nesne yok", {"kind": "window", "a": P(*a), "b": P(*b)},
                     in_rect(rect(a, b), False)))
    out.append(query("çit: kestiği iki alan", {"kind": "fence", "points": [P(-2, 8), P(35, 8)], "tolerance": 0},
                     in_fence([(-2, 8), (35, 8)], 0)))
    out.append(query("çit: toleransı içindeki nokta",
                     {"kind": "fence", "points": [P(4, 19.5), P(6, 19.5)], "tolerance": 1},
                     in_fence([(4, F(39, 2)), (6, F(39, 2))], 1)))
    ring = [(1, 1), (12, 1), (12, 25), (1, 25)]
    for mode, name in [("inside", "içindekiler"), ("crossing", "kesişenler"), ("outside", "dışındakiler")]:
        out.append(query(f"çokgen: {name}", {"kind": "polygon", "points": [P(*q) for q in ring], "mode": mode},
                         polygon_select(ring, mode)))
    out.append(query("daire: tamamen içindeki", {"kind": "circle", "center": P(3, 3), "radius": 1.8},
                     in_circle((3, 3), F(18, 10), False)))
    out.append(query("daire: dokunanlar, merkezi alanın içinde olan da",
                     {"kind": "circle", "center": P(3, 3), "radius": 1.8, "crossing": True},
                     in_circle((3, 3), F(18, 10), True)))
    out.append(query("içeren alanlar küçükten büyüğe", {"kind": "containing", "at": P(3, 3)}, containing((3, 3))))
    out.append(query("içeren daire", {"kind": "containing", "at": P(60, 5)}, containing((60, 5))))
    out.append(query("tıklama: önce çizgi, sonra içinde olduğu alan",
                     {"kind": "point", "at": P(5, 5.1), "tolerance": 0.3}, hits((5, F(51, 10)), 0.3)))
    every = [o[0] for o in shown()]
    out.append(query("hepsi: gizli katmanınki yok, kilitlininki var", {"kind": "all"}, every))
    out.append(query("hepsi, yalnız alanlar: süzgeç dışarıda kalanları sayar", {"kind": "all"}, every,
                     kinds=["polygon"]))
    out.append(query("hepsi, yalnız Yol katmanı", {"kind": "all"}, every, layers=["yol"]))
    out.append(query("tür ve katman birlikte: önce tür, sonra katman sayılır", {"kind": "all"}, every,
                     kinds=["line"], layers=["kilitli"]))
    out.append(query("verilenler (önceki seçim): gizli olan sayılır, sıra korunur",
                     {"kind": "objects", "uids": [uid(3), uid(8), uid(1), uid(3)]},
                     [3, 1], hidden=1))
    out.append(query("ters çevir", {"kind": "invert", "uids": [uid(1), uid(2)]},
                     [i for i in every if i not in (1, 2)]))
    crit = {"kind": True, "layer": True, "color": True, "symbol": True}
    out.append(query("benzerleri: tür, katman, renk ve sembol", {"kind": "similar", "uids": [uid(2)]},
                     similar([2], crit)))
    c2 = dict(crit, color=False)
    out.append(query("benzerleri: renk dışında", {"kind": "similar", "uids": [uid(2)], "criteria": c2},
                     similar([2], c2)))
    c3 = {"kind": True, "layer": False, "color": False, "symbol": False}
    out.append(query("benzerleri: yalnız tür, kilitli katmanınki de", {"kind": "similar", "uids": [uid(4)],
                                                                     "criteria": c3}, similar([4], c3)))
    c4 = {"kind": False, "layer": False, "color": False, "symbol": False}
    out.append(query("benzerleri: ölçütsüz, bütün gösterilenler", {"kind": "similar", "uids": [uid(4)],
                                                                  "criteria": c4}, similar([4], c4)))
    a, b = (19, 19), (31, 21)
    out.append(query("kilitli katmandaki çizgi seçilir", {"kind": "window", "a": P(*a), "b": P(*b)},
                     in_rect(rect(a, b), False)))
    out.append(query("gizli katmandaki alan seçilmez", {"kind": "window", "a": P(-1, 29), "b": P(11, 41)}, []))
    out.append(query("plan aynı cevabı verir", {"kind": "window", "a": P(-1, -1), "b": P(11, 11)},
                     in_rect(rect((-1, -1), (11, 11)), False), op="plan"))
    out.append(case("validate yalnız denetler",
                    step({"region": {"kind": "all"}}, {"status": "completed", "output": None, "warnings": []},
                         op="validate")))
    # Refusals.
    out.append(case("kutunun köşesi sonlu olmalı",
                    step({"region": {"kind": "window", "a": P(0, 0), "b": P(1, 1)}},
                         failed("not_finite", f"Kutunun ilk köşesinin doğu (Y) {NOT_FINITE}", "region.a.x"),
                         nonFinite={"region.a.x": "Infinity"})))
    out.append(case("çitin tek noktası olmaz",
                    step({"region": {"kind": "fence", "points": [P(0, 0)], "tolerance": 0}},
                         failed("too_few_points", "Çitin en az iki noktası olmalı.", "region.points"))))
    out.append(case("çokgenin iki köşesi olmaz",
                    step({"region": {"kind": "polygon", "points": [P(0, 0), P(1, 1)], "mode": "inside"}},
                         failed("too_few_corners", "Seçim çokgeninin en az üç köşesi olmalı.", "region.points"))))
    out.append(case("kendini kesen çokgen",
                    step({"region": {"kind": "polygon", "points": [P(0, 0), P(10, 10), P(10, 0), P(0, 10)],
                                     "mode": "crossing"}},
                         failed("invalid_ring",
                                "Çokgen kendini kesiyor; son köşeyi geri alın (G) ya da kesmeyen bir köşe gösterin.",
                                "region.points"))))
    out.append(case("dairenin yarıçapı sıfır olmaz",
                    step({"region": {"kind": "circle", "center": P(0, 0), "radius": 0}},
                         failed("invalid_radius", "Seçim dairesinin yarıçapı sıfırdan büyük olmalı.",
                                "region.radius"))))
    out.append(case("eksi tolerans",
                    step({"region": {"kind": "point", "at": P(0, 0), "tolerance": -1}},
                         failed("invalid_distance",
                                "Tolerans sıfırdan küçük olamaz: tıklamanın ya da çitin yakınında aranacak uzaklığı metre olarak verin.",
                                "region.tolerance"))))
    out.append(case("geçersiz kimlik",
                    step({"region": {"kind": "objects", "uids": ["abc"]}},
                         failed("invalid_uid",
                                "“abc” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir.",
                                "region.uids[0]"))))
    gone = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4999"
    out.append(case("çizimde olmayan nesne",
                    step({"region": {"kind": "invert", "uids": [uid(1), gone]}},
                         failed("entity_not_found",
                                f"“{gone}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir.",
                                "region.uids[1]"))))
    out.append(case("örneksiz benzerleri",
                    step({"region": {"kind": "similar", "uids": []}},
                         failed("no_objects", "Benzerini seç için en az bir örnek nesnenin kimliğini verin.",
                                "region.uids"))))
    out.append(case("olmayan katman",
                    step({"region": {"kind": "all"}, "layers": ["yapi", "yok"]},
                         failed("layer_not_found",
                                "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.",
                                "layers[1]"))))
    out.append(case("katman grubu süzgeç olmaz",
                    step({"region": {"kind": "all"}, "layers": ["koruma"]},
                         failed("not_a_layer",
                                "“Koruma” bir katman grubu; süzgeç katmanları alır. Grubun katmanlarını verin.",
                                "layers[0]"))))
    return out


def build():
    return {
        "format": "kentos.command-cases", "version": 1, "command": "cad.entities.select", "commandVersion": 1,
        "title": "Nesne seç: bölge, liste ya da benzerlik; türler ve katmanlar",
        "note": "ADR 0275. Yalnız gösterilen nesneler; kilitli katmanınkiler seçilir. Süzgeçler bölgeden sonra: önce "
                "türler, sonra katmanlar, dışarıda kalanları sayarak. Çizim değişmez. Bağımsız başvuru: "
                "scripts/fixtures/entities_select_cases.py (kesirlerle, KentOS kodu olmadan).",
        "setup": SETUP, "cases": cases(),
    }


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"nesne seç durumları tutarlı: {len(build()['cases'])} durum")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(build()['cases'])} durum")
    return 0


if __name__ == "__main__":
    sys.exit(main())
