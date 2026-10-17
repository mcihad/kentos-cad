"""The shared cases of the product commands cad.entities.boundary and cad.entities.hatch (docs/adr/0254 §6, §7).

    python3 scripts/fixtures/region_command_cases.py           # writes the files
    python3 scripts/fixtures/region_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.{boundary,hatch}.json. The checks, their order, codes, paths and messages
are written here by hand from the ADR. What the commands do is worked out independently, from the operations'
definitions (docs/adr/0062, 0065, 0186, 0254), without KentOS code, on a drawing of axis-parallel line work:

- A region around a point: the smallest face the boundaries' line work closes around it (the shown objects, or
  those meeting the rectangle, or the listed ones; of the boundary layer alone when one is named); the closed
  groups inside it are its holes unless Adalar is off.
- Çizgiler with İlişkili (docs/adr/0328 §3): the hatch names the shown objects of the boundary set and layer along
  whose line work the middle of one of the face's outer edges lies (its bounds), those along its holes' edges (its
  islands), each list in the drawing's order, and the point.
- Kapalı nesne: the smallest closed object around the point; the smaller closed objects inside it are its
  islands, cut out (holes here, each wholly inside); with İlişkili the hatch names the object, the islands that
  reached in and the point.
- Çoklu tara: each closed object by itself from its seed: at mid-height (moved up by 0.0137 of the height while a
  vertex lies there), the middle of the widest span across the rings.
- An area as the commands write it: the outer ring counter-clockwise from its lowest vertex (the leftmost of the
  lowest), the holes clockwise from theirs, ordered by those vertices.
- The pattern: Dolu solid; Çizgili lines at 45° + Açı, 3 mm on paper times Ölçek; a library pattern at Açı with
  its families and Ölçek × the plot scale / 1000 as its scale; a gradient with its shape, İkinci renk and Ters.

`--check` rebuilds the files in memory and compares them with the ones on disk.
"""
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

sys.dont_write_bytecode = True  # no __pycache__ in the tree
from affine_reference import assert_no_negative_zero  # noqa: E402  (beside this file)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
style = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}
X, Y = 487000, 4420000
PLOT_SCALE = 500


def layer(i, name, visible=True, locked=False, kind="layer", children=()):
    return {"id": i, "name": name, "type": kind, "visible": visible, "locked": locked, "expanded": True, "style": style,
            "children": list(children)}


def num(v):
    v = F(v)
    assert v.denominator & (v.denominator - 1) == 0, v  # a dyadic: a double exactly
    f = float(v)
    return int(f) if f.is_integer() else f


def P(x, y):
    return {"x": num(x), "y": num(y)}


def line(i, a, b, layer_id="yapi"):
    return {"kind": "line", "id": i, "layerId": layer_id, "attrs": {}, "a": P(*a), "b": P(*b)}


def rect_lines(first, x0, y0, x1, y1, layer_id="yapi"):
    return [line(first, (x0, y0), (x1, y0), layer_id), line(first + 1, (x1, y0), (x1, y1), layer_id),
            line(first + 2, (x1, y1), (x0, y1), layer_id), line(first + 3, (x0, y1), (x0, y0), layer_id)]


def rect(x0, y0, x1, y1):
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]


def polygon(i, pts, layer_id="yapi", **extra):
    return {"kind": "polygon", "id": i, "layerId": layer_id, "attrs": {}, **extra, "pts": [P(*p) for p in pts]}


ROOM_A = (X, Y, X + 10, Y + 8)
ISLAND = (X + 4, Y + 3, X + 6, Y + 5)
ROOM_B = (X + 20, Y, X + 30, Y + 8)
PARCEL = (X + 40, Y, X + 60, Y + 20)
BUILDING = (X + 44, Y + 4, X + 48, Y + 8)
HIDDEN = (X + 70, Y, X + 80, Y + 8)

ENTITIES = [
    *rect_lines(10, *ROOM_A),
    polygon(14, rect(*ISLAND)),
    *rect_lines(15, *ROOM_B, layer_id="sinir"),
    line(19, (X + 25, Y), (X + 25, Y + 8)),
    polygon(20, rect(*PARCEL), attrs={"Ada": "5"}),
    polygon(21, rect(*BUILDING)),
    *rect_lines(22, *HIDDEN, layer_id="gizli"),
    line(26, (X + 90, Y), (X + 100, Y)),
]
BY_ID = {e["id"]: e for e in ENTITIES}
IDS = [e["id"] for e in ENTITIES]
NEXT = 27

SETUP = {
    "format": "kentos.document", "version": 1, "name": "Bölge komutları",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": PLOT_SCALE},
    "origin": {"x": X, "y": Y},
    "layers": [
        layer("yapi", "Yapı"), layer("sinir", "Sınır"), layer("alan", "Alan"), layer("tarama", "Tarama"),
        layer("kilitli", "Kilitli", locked=True), layer("gizli", "Gizli", visible=False),
        layer("grup", "Grup", kind="group", children=[layer("alt", "Alt")]),
    ],
    "activeLayer": "yapi", "entities": ENTITIES, "styles": {"items": [], "categories": []},
}

UID = lambda i: f"$uidOf:{i}"  # noqa: E731
BAD = "Yapı-01"
MISSING = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"
INVALID_REVISION = "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."
UNCHANGED = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}


def bad_uid(u):
    return f"“{u}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."


def not_found(u):
    return f"“{u}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."


def not_finite_point(whose, axis):
    name = "doğu (Y)" if axis == "x" else "kuzey (X)"
    return f"{whose} {name} değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin."


def completed(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def warning(code, message, path=None):
    w = {"code": code, "message": message}
    if path is not None:
        w["path"] = path
    return w


def refused(name, input_, code, message, path, non_finite=None):
    step = {"op": "execute", "input": input_}
    if non_finite:
        step["nonFinite"] = non_finite
    step["result"] = failed(code, message, path)
    step["expect"] = UNCHANGED
    return {"name": name, "steps": [step]}


# ── Rings as written ──────────────────────────────────────────────────────────────────────────────────────────────

def signed_area(pts):
    return sum(F(a[0]) * F(b[1]) - F(b[0]) * F(a[1]) for a, b in zip(pts, pts[1:] + pts[:1])) / 2


def canonical(pts, ccw):
    """The ring turned the asked way, from its lowest vertex (the leftmost of the lowest)."""
    if (signed_area(pts) > 0) != ccw:
        pts = [pts[0]] + pts[:0:-1]
    k = min(range(len(pts)), key=lambda i: (pts[i][1], pts[i][0]))
    return pts[k:] + pts[:k]


def area(outer, holes=()):
    """An area as the commands write it: outer CCW, holes CW, holes by their lowest vertices."""
    hs = sorted((canonical(rect(*h), False) for h in holes), key=lambda r: (r[0][1], r[0][0]))
    return canonical(rect(*outer), True), hs


# ── Alan oluştur ──────────────────────────────────────────────────────────────────────────────────────────────────

def area_entity(eid, outer, holes=(), layer_id="alan", **look):
    o, hs = area(outer, holes)
    e = {"kind": "polygon", "id": eid, "layerId": layer_id, "attrs": {}, **look, "pts": [P(*p) for p in o]}
    if hs:
        e["holes"] = [{"pts": [P(*p) for p in h]} for h in hs]
    return e


def boundary_case(name, at, outer, holes=(), extra=None, layer_id="alan", look=None, warnings=(), plan=False, undo=False):
    inp = {"at": P(*at), "layerId": layer_id, **(extra or {}), **(look or {})}
    if plan:
        e = area_entity(0, outer, holes, layer_id, **(look or {}))
        return {"name": name, "steps": [{"op": "plan", "input": inp, "result": completed({"entities": [e], "revision": "$current"}, warnings),
                                         "expect": UNCHANGED}]}
    e = area_entity(NEXT, outer, holes, layer_id, **(look or {}))
    steps = [{"op": "execute", "input": inp,
              "result": completed({"created": [UID(NEXT)], "ids": [NEXT], "revision": "$current"}, warnings),
              "expect": {"ids": IDS + [NEXT], "entities": {str(NEXT): e}, "revision": "changed", "canUndo": True, "canRedo": False, "dirty": True}}]
    if undo:
        steps.append({"op": "undo", "returns": "Alan oluştur", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}})
    return {"name": name, "steps": steps}


NOT_ENCLOSED = "Nokta kapalı bir bölgenin içinde değil. Bölgeyi saran çizgiler birleşmeli ya da kesişmeli."


def boundary_cases():
    room_b_left = (X + 20, Y, X + 25, Y + 8)
    out = [
        boundary_case("Çizgilerin kapattığı oda alan olur; içindeki kapalı nesne delik; adım “Alan oluştur”", (X + 2, Y + 2), ROOM_A, [ISLAND], undo=True),
        boundary_case("Adalar kapalı: delik yok", (X + 2, Y + 2), ROOM_A, extra={"islands": False}),
        boundary_case("Adanın içi kendi bölgesidir", (X + 5, Y + 4), ISLAND),
        boundary_case("Bütün görünen çizgiler: ara çizgi odayı böler", (X + 22, Y + 4), room_b_left),
        boundary_case("Sınır katmanı: yalnız onun çizgileri sayılır, ara çizgi sayılmaz", (X + 22, Y + 4), ROOM_B, extra={"boundaryLayer": "sinir"}),
        boundary_case("Dikdörtgene değen görünen nesneler (aracın görünümü)", (X + 22, Y + 4), room_b_left,
                      extra={"boundaries": {"kind": "area", "min": P(X + 19, Y - 1), "max": P(X + 26, Y + 9)}}),
        boundary_case("Verilen nesneler, gizli katmanda olsalar da", (X + 75, Y + 4), HIDDEN,
                      extra={"boundaries": {"kind": "objects", "uids": [UID(i) for i in (22, 23, 24, 25)]}}),
        boundary_case("Renk ve kalınlıkla", (X + 2, Y + 2), ROOM_A, [ISLAND], look={"color": "#30A46C", "lineWeight": 0.5}),
        boundary_case("Gizli katmana: layer_hidden uyarısıyla yazılır", (X + 2, Y + 2), ROOM_A, [ISLAND], layer_id="gizli",
                      warnings=[warning("layer_hidden", "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.", "layerId")]),
        boundary_case("Plan: alan yazılacağı gibi, id 0; hiçbir şey yazılmaz", (X + 2, Y + 2), ROOM_A, [ISLAND], plan=True),
    ]
    one = {"at": P(X + 2, Y + 2), "layerId": "alan"}
    out += [
        refused("Nokta sonlu değil", one, "not_finite", not_finite_point("Tıklanan noktanın", "x"), "at.x", non_finite={"at.x": "NaN"}),
        refused("Sınır listesi boş", {**one, "boundaries": {"kind": "objects", "uids": []}}, "no_entities",
                "Sınır nesnesi yok. Sınırları verin ya da bütün görünen nesneler için “all”u kullanın.", "boundaries.uids"),
        refused("Sınırın kimliği geçersiz", {**one, "boundaries": {"kind": "objects", "uids": [BAD]}}, "invalid_uid", bad_uid(BAD), "boundaries.uids[0]"),
        refused("Dikdörtgenin köşesi sonlu değil", {**one, "boundaries": {"kind": "area", "min": P(X, Y), "max": P(X + 10, Y + 10)}}, "not_finite",
                not_finite_point("Dikdörtgenin köşesinin", "y"), "boundaries.max.y", non_finite={"boundaries.max.y": "Infinity"}),
        refused("Beklenen sürüm sayı değil", {**one, "expectedRevision": "on iki"}, "invalid_revision", INVALID_REVISION, "expectedRevision"),
        refused("Sınır çizimde yok", {**one, "boundaries": {"kind": "objects", "uids": [MISSING]}}, "entity_not_found", not_found(MISSING), "boundaries.uids[0]"),
        refused("Sınır katmanı yok", {**one, "boundaryLayer": "yok"}, "layer_not_found",
                "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "boundaryLayer"),
        refused("Hiçbir bölgenin içinde değil", {**one, "at": P(X + 200, Y + 200)}, "not_enclosed", NOT_ENCLOSED, "at"),
        refused("Gizli katmanın çizgileri görünmez: bölge kapanmaz", {**one, "at": P(X + 75, Y + 4)}, "not_enclosed", NOT_ENCLOSED, "at"),
        refused("Dikdörtgen bölgeyi saran çizgileri dışarıda bırakır", {**one, "at": P(X + 22, Y + 4),
                "boundaries": {"kind": "area", "min": P(X + 21, Y - 1), "max": P(X + 24, Y + 9)}}, "not_enclosed", NOT_ENCLOSED, "at"),
        refused("Katman yok", {**one, "layerId": "yok"}, "layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"),
        refused("Grup katman değildir", {**one, "layerId": "grup"}, "not_a_layer",
                "“Grup” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.", "layerId"),
        refused("Kilitli katmana yazılmaz", {**one, "layerId": "kilitli"}, "layer_locked",
                "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.", "layerId"),
        refused("Kalınlık aralıkta değil", {**one, "lineWeight": 200}, "invalid_line_weight",
                "Çizgi kalınlığı 0 ile 100 mm arasında bir sayı olmalı (0 en ince çizgidir). Bir kalınlık ya da “Katmana göre” seçin.", "lineWeight"),
    ]
    out.append({"name": "Beklenen sürüm geçmişte kaldıysa conflict, hiçbir şey yazılmaz", "steps": [
        {"op": "captureRevision", "as": "old"},
        {"op": "execute", "input": {"at": P(X + 5, Y + 4), "layerId": "alan"},
         "result": completed({"created": [UID(NEXT)], "ids": [NEXT], "revision": "$current"})},
        {"op": "execute", "input": {**one, "expectedRevision": "$old"},
         "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}}}]})
    return out


# ── Tarama ────────────────────────────────────────────────────────────────────────────────────────────────────────

E = 3.175  # the ANSI family's spacing, mm (docs/adr/0186's library)


def turn_of(a):
    t = math.fmod(a, 360.0)
    return t + 360.0 if t < 0 else t


def pattern(name, scale=1.0, angle=0.0, second="#FFFFFF", inverted=False):
    metres = scale * PLOT_SCALE / 1000.0
    if name == "Dolu":
        return {"type": "solid", "angle": 0, "spacing": 1}
    if name == "Çizgili":
        return {"type": "lines", "angle": turn_of(45.0 + angle) % 180.0, "spacing": 3.0 * metres}
    if name == "ANSI31":
        return {"type": "pattern", "angle": turn_of(angle), "spacing": 1, "name": "ANSI31", "scale": metres,
                "lines": [{"angle": 45, "origin": [0, 0], "offset": [0, E]}]}
    shape = {"Degrade küre": "spherical"}[name]
    g = {"shape": shape, "color2": second}
    if inverted:
        g["inverted"] = True
    return {"type": "gradient", "angle": turn_of(angle), "spacing": 1, "gradient": g}


def seed_of(outer, holes=()):
    """Çoklu tara's point inside an object's own area (see the module), in doubles as the rule's
    arithmetic goes; of equally wide spans the last."""
    rings = [rect(*outer)] + [rect(*h) for h in holes]
    ys = [float(p[1]) for p in rings[0]]
    lo, hi = min(ys), max(ys)
    y = (lo + hi) / 2.0
    for k in range(16):
        if not any(float(p[1]) == y for r in rings for p in r):
            break
        y = lo + (hi - lo) * (0.5 + float(k + 1) * 0.0137)
    xs = []
    for r in rings:
        for i in range(len(r)):
            p, q = r[i], r[(i + 1) % len(r)]
            py, qy = float(p[1]), float(q[1])
            if (py <= y) != (qy <= y):
                xs.append(float(p[0]) + (y - py) / (qy - py) * (float(q[0]) - float(p[0])))
    xs.sort()
    spans = [(xs[i], xs[i + 1]) for i in range(0, len(xs) - 1, 2)]
    a, b = spans[0]
    for s in spans[1:]:
        if s[1] - s[0] >= b - a:
            a, b = s
    return (a + b) / 2.0, y


def plain(v):
    return int(v) if float(v).is_integer() else v


def hatch_entity(eid, outer, holes, pat, assoc=None, layer_id="tarama", **look):
    o, hs = area(outer, holes)
    e = {"kind": "hatch", "id": eid, "layerId": layer_id, "attrs": {}, **look, "ring": [P(*p) for p in o]}
    if hs:
        e["holes"] = [[P(*p) for p in h] for h in hs]
    e["pattern"] = pat
    if assoc:
        e["assoc"] = assoc
    return e


def tie(outer_id, islands=(), seed=None):
    t = {"outer": UID(outer_id)}
    if islands:
        t["islands"] = [UID(i) for i in islands]
    t["seed"] = {"x": plain(seed[0]), "y": plain(seed[1])}
    return t


def on_segment(m, a, b):
    """Whether the point `m` lies on the segment a–b (exactly: the work is axis-parallel)."""
    (mx, my), (ax, ay), (bx, by) = [tuple(map(F, q)) for q in (m, a, b)]
    if (bx - ax) * (my - ay) != (by - ay) * (mx - ax):
        return False
    return min(ax, bx) <= mx <= max(ax, bx) and min(ay, by) <= my <= max(ay, by)


def segments_of(e):
    if e["kind"] == "line":
        return [((e["a"]["x"], e["a"]["y"]), (e["b"]["x"], e["b"]["y"]))]
    pts = [(p["x"], p["y"]) for p in e["pts"]]
    return list(zip(pts, pts[1:] + pts[:1]))


def along(rect_, layer_id=None):
    """The shown objects (of `layer_id` alone when given) along whose line work the middle of one of the rectangle's
    edges lies, in the drawing's order."""
    corners = rect(*rect_)
    middles = [((F(a[0]) + F(b[0])) / 2, (F(a[1]) + F(b[1])) / 2) for a, b in zip(corners, corners[1:] + corners[:1])]
    hidden = {"gizli"}
    out = []
    for e in ENTITIES:
        if e["layerId"] in hidden or (layer_id is not None and e["layerId"] != layer_id):
            continue
        if any(on_segment(m, a, b) for m in middles for a, b in segments_of(e)):
            out.append(e["id"])
    return out


def face_tie(face, holes, seed, layer_id=None):
    """A face's tie (docs/adr/0328 §3): its bounds, its islands (not already bounds), the point."""
    bounds = along(face, layer_id)
    islands = [i for h in holes for i in along(h, layer_id) if i not in bounds]
    t = {"bounds": [UID(i) for i in bounds]}
    if islands:
        t["islands"] = [UID(i) for i in islands]
    t["seed"] = {"x": plain(seed[0]), "y": plain(seed[1])}
    return t


def hatch_case(name, region, pat_in, made, layer_id="tarama", look=None, warnings=(), plan=False, undo=False):
    """`made`: [(outer, holes, pattern, assoc)] the hatches written, in order."""
    inp = {"region": region, "pattern": pat_in, "layerId": layer_id, **(look or {})}
    if plan:
        ents = [hatch_entity(0, o, h, p, a, layer_id, **(look or {})) for o, h, p, a in made]
        return {"name": name, "steps": [{"op": "plan", "input": inp, "result": completed({"entities": ents, "revision": "$current"}, warnings),
                                         "expect": UNCHANGED}]}
    ids = list(range(NEXT, NEXT + len(made)))
    ents = {str(i): hatch_entity(i, o, h, p, a, layer_id, **(look or {})) for i, (o, h, p, a) in zip(ids, made)}
    steps = [{"op": "execute", "input": inp,
              "result": completed({"created": [UID(i) for i in ids], "ids": ids, "revision": "$current"}, warnings),
              "expect": {"ids": IDS + ids, "entities": ents, "revision": "changed", "canUndo": True, "canRedo": False, "dirty": True}}]
    if undo:
        steps.append({"op": "undo", "returns": "Tarama", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}})
    return {"name": name, "steps": steps}


def at_point(x, y, **options):
    return {"kind": "point", "at": P(x, y), **options}


DOLU = {"name": "Dolu"}
NOT_ENCLOSED_OBJECT = "Noktayı çevreleyen kapalı bir alan, daire ya da kapalı eğri yok. Çizgilerle çevrili yerler için way: lines verin."
NOT_ENCLOSED_LINES = "Nokta çizgilerle kapalı bir bölgenin içinde değil. Bölgeyi saran çizgiler birleşmeli ya da kesişmeli."


def hatch_cases():
    p_in = (X + 42, Y + 2)
    solid = pattern("Dolu")
    out = [
        hatch_case("Kapalı nesne: parselin içine tıklanır, bina ada, tarama nesnelerine bağlı; adım “Tarama”", at_point(*p_in), DOLU,
                   [(PARCEL, [BUILDING], solid, tie(20, [21], p_in))], undo=True),
        hatch_case("Adalar kapalı: bina taranır, ilişkide ada yok", at_point(*p_in, islands=False), DOLU, [(PARCEL, [], solid, tie(20, [], p_in))]),
        hatch_case("İlişkisiz: tarama nesnelerini izlemez", at_point(*p_in, associative=False), DOLU, [(PARCEL, [BUILDING], solid, None)]),
        hatch_case("Binanın içi: noktayı çevreleyen en küçük kapalı nesne", at_point(X + 46, Y + 6), DOLU, [(BUILDING, [], solid, tie(21, [], (X + 46, Y + 6)))]),
        hatch_case("Çizgiler: odanın yüzü, içindeki kapalı nesne delik; tarama yüzü kapatan çizgilere ve adaya bağlı", at_point(X + 2, Y + 2, way="lines"), DOLU,
                   [(ROOM_A, [ISLAND], solid, face_tie(ROOM_A, [ISLAND], (X + 2, Y + 2)))]),
        hatch_case("Çizgiler, ilişkisiz: bir şeye bağlanmaz", at_point(X + 2, Y + 2, way="lines", associative=False), DOLU,
                   [(ROOM_A, [ISLAND], solid, None)]),
        hatch_case("Çizgiler ve sınır katmanı: ara çizgi sayılmaz, bağ da onun çizgileri", at_point(X + 22, Y + 4, way="lines", boundaryLayer="sinir"), DOLU,
                   [(ROOM_B, [], solid, face_tie(ROOM_B, [], (X + 22, Y + 4), "sinir"))]),
        hatch_case("Çizgili: 45° + açı, kâğıtta 3 mm × ölçek × çizim ölçeği", at_point(*p_in, associative=False),
                   {"name": "Çizgili", "scale": 2, "angle": 15}, [(PARCEL, [BUILDING], pattern("Çizgili", 2.0, 15.0), None)]),
        hatch_case("Kitaplık deseni adıyla (büyük küçük harf fark etmez)", at_point(*p_in, associative=False), {"name": "ansi31"},
                   [(PARCEL, [BUILDING], pattern("ANSI31"), None)]),
        hatch_case("Degrade adının son sözcüğüyle; ikinci renk adıyla, ters", at_point(*p_in, associative=False),
                   {"name": "küre", "secondColor": "Kırmızı", "inverted": True},
                   [(PARCEL, [BUILDING], pattern("Degrade küre", second="#E5484D", inverted=True), None)]),
        hatch_case("Çoklu tara: her kapalı nesne kendi içinden, tek adımda", {"kind": "objects", "uids": [UID(20), UID(21)]}, DOLU,
                   [(PARCEL, [BUILDING], solid, tie(20, [21], seed_of(PARCEL))),
                    (BUILDING, [], solid, tie(21, [], seed_of(BUILDING)))]),
        hatch_case("Çoklu tara: kapalı olmayan not_closed uyarısıyla atlanır", {"kind": "objects", "uids": [UID(26), UID(21)], "associative": False}, DOLU,
                   [(BUILDING, [], solid, None)], warnings=[warning("not_closed", "Nesne kapalı değil; taranmadı.", "region.uids[0]")]),
        hatch_case("Renkle", at_point(*p_in, associative=False), DOLU, [(PARCEL, [BUILDING], solid, None)], look={"color": "#30A46C"}),
        hatch_case("Gizli katmana: layer_hidden uyarısıyla yazılır", at_point(*p_in, associative=False), DOLU, [(PARCEL, [BUILDING], solid, None)],
                   layer_id="gizli", warnings=[warning("layer_hidden", "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.", "layerId")]),
        hatch_case("Plan: tarama yazılacağı gibi, id 0; hiçbir şey yazılmaz", at_point(*p_in, associative=False), DOLU,
                   [(PARCEL, [BUILDING], solid, None)], plan=True),
    ]
    one = {"region": at_point(*p_in), "pattern": DOLU, "layerId": "tarama"}
    out += [
        refused("Bilinmeyen desen", {**one, "pattern": {"name": "Yok"}}, "unknown_pattern",
                "“Yok” adında desen yok. Desen kitaplığındaki bir adı verin (Dolu, Çizgili, ANSI31, ISO02W100 …).", "pattern.name"),
        refused("Ölçek sıfır olamaz", {**one, "pattern": {"name": "ANSI31", "scale": 0}}, "invalid_scale",
                "Desenin ölçeği sıfırdan büyük, en çok 10 000 olan sonlu bir sayı olmalı.", "pattern.scale"),
        refused("Ölçek 10 000'i aşamaz", {**one, "pattern": {"name": "ANSI31", "scale": 20000}}, "invalid_scale",
                "Desenin ölçeği sıfırdan büyük, en çok 10 000 olan sonlu bir sayı olmalı.", "pattern.scale"),
        refused("Açı sonlu değil", {**one, "pattern": {"name": "ANSI31", "angle": 0}}, "not_finite",
                "Desenin açısı sonlu bir sayı değil (NaN ya da sonsuz). Açıyı derece olarak sonlu bir sayıyla verin.", "pattern.angle",
                non_finite={"pattern.angle": "NaN"}),
        refused("İkinci renk renk değil", {**one, "pattern": {"name": "küre", "secondColor": "mavimsi"}}, "invalid_color",
                "“mavimsi” bir renk değil: #RRGGBB ya da bir renk adı (Beyaz, Siyah, Kırmızı …) verin.", "pattern.secondColor"),
        refused("Nokta sonlu değil", one, "not_finite", not_finite_point("Tıklanan noktanın", "y"), "region.at.y", non_finite={"region.at.y": "NaN"}),
        refused("Çoklu tara: nesne yok", {**one, "region": {"kind": "objects", "uids": []}}, "no_entities",
                "Taranacak nesne yok. En az bir kapalı nesnenin kalıcı kimliğini verin.", "region.uids"),
        refused("Çoklu tara: kimlik geçersiz", {**one, "region": {"kind": "objects", "uids": [BAD]}}, "invalid_uid", bad_uid(BAD), "region.uids[0]"),
        refused("Beklenen sürüm sayı değil", {**one, "expectedRevision": "on iki"}, "invalid_revision", INVALID_REVISION, "expectedRevision"),
        refused("Çoklu tara: nesne çizimde yok", {**one, "region": {"kind": "objects", "uids": [MISSING]}}, "entity_not_found", not_found(MISSING), "region.uids[0]"),
        refused("Sınır katmanı yok", {**one, "region": at_point(X + 2, Y + 2, way="lines", boundaryLayer="yok")}, "layer_not_found",
                "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "region.boundaryLayer"),
        refused("Kapalı nesne yok", {**one, "region": at_point(X + 200, Y + 200)}, "not_enclosed", NOT_ENCLOSED_OBJECT, "region.at"),
        refused("Çizgilerle kapalı bölge yok", {**one, "region": at_point(X + 200, Y + 200, way="lines")}, "not_enclosed", NOT_ENCLOSED_LINES, "region.at"),
        refused("Çoklu tara: hiçbiri kapalı değil", {**one, "region": {"kind": "objects", "uids": [UID(26)]}}, "not_closed",
                "Nesnelerin hiçbiri kapalı değil: alan, daire, tam elips ya da kapalı eğri taranır.", "region.uids"),
        refused("Desen bölge için çok sık: 20 000'den çok çizgi", {**one, "pattern": {"name": "Çizgili", "scale": 0.0001}}, "pattern_too_dense",
                "Desen bu bölge için çok sık; ölçeği ya da çizim ölçeğini büyütün ya da başka bir desen seçin.", "pattern"),
        refused("Kilitli katmana yazılmaz", {**one, "layerId": "kilitli"}, "layer_locked",
                "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.", "layerId"),
        refused("Grup katman değildir", {**one, "layerId": "grup"}, "not_a_layer",
                "“Grup” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.", "layerId"),
        refused("Katman yok", {**one, "layerId": "yok"}, "layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"),
        refused("Kalınlık aralıkta değil", {**one, "lineWeight": 200}, "invalid_line_weight",
                "Çizgi kalınlığı 0 ile 100 mm arasında bir sayı olmalı (0 en ince çizgidir). Bir kalınlık ya da “Katmana göre” seçin.", "lineWeight"),
    ]
    out.append({"name": "Beklenen sürüm geçmişte kaldıysa conflict, hiçbir şey yazılmaz", "steps": [
        {"op": "captureRevision", "as": "old"},
        {"op": "execute", "input": {"region": at_point(X + 46, Y + 6), "pattern": DOLU, "layerId": "tarama"},
         "result": completed({"created": [UID(NEXT)], "ids": [NEXT], "revision": "$current"})},
        {"op": "execute", "input": {**one, "expectedRevision": "$old"},
         "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}}}]})
    return out


FILES = [
    ("cad.entities.boundary", "İçine tıklayarak alan", boundary_cases),
    ("cad.entities.hatch", "Tara", hatch_cases),
]


def build(command, title, cases):
    return {
        "format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
        "note": "Written by scripts/fixtures/region_command_cases.py from the operations' definitions (docs/adr/0062, 0065, 0186, 0254); do not edit by hand.",
        "setup": SETUP, "cases": cases(),
    }


def main() -> int:
    check = "--check" in sys.argv[1:]
    bad = False
    for command, title, cases in FILES:
        doc = build(command, title, cases)
        assert_no_negative_zero(doc, command)
        text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
        path = OUT / f"{command}.json"
        if check:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                print(f"{path.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
                bad = True
            else:
                print(f"{command} durumları güncel: {len(doc['cases'])} durum.")
        else:
            path.write_text(text, encoding="utf-8")
            print(f"yazıldı: {path.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
