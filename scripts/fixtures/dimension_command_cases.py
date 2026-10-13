"""Writes fixtures/commands/v1/cad.dimensions.reassociate.json: the shared cases of Yeniden bağla (docs/adr/0249 §3),
their expectations built here from the contract's rules (crates/shared/contracts/src/cad_dimensions.rs and subref.rs:
the checks and their order, the codes, the paths and the words; the step's name) and from the places' own definitions
(a path's vertex, a line's middle, a circle's centre; a place along an edge or a curve and a piece inside a block,
docs/adr/0263), without KentOS code. The desktop
(crates/native/application/tests/all/fixtures.rs) and Python (python/tests/test_command_cases.py) run them; the web is
paused (docs/adr/0228).

    python3 scripts/fixtures/dimension_command_cases.py           # writes the file
    python3 scripts/fixtures/dimension_command_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.dimensions.reassociate.json"

COMMAND = "cad.dimensions.reassociate"
STEP = "Ölçüyü yeniden bağla"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}


def node(id_, name, type_="layer", children=None, **extra):
    return {"id": id_, "name": name, "type": type_, "visible": True, "locked": False, "expanded": True,
            "style": dict(STYLE), "children": children or [], **extra}


LAYERS = [
    node("duvar", "Duvar"),
    node("olcu", "Ölçü"),
    node("koruma", "Koruma", "group", [node("kilitli", "Eski ölçü")], locked=True),
]

X, Y = 500000, 4400000
WALL = [{"x": X, "y": Y}, {"x": X + 12, "y": Y}, {"x": X + 12, "y": Y + 5}]
CENTRE = {"x": X + 13, "y": Y + 4}
LINE = ({"x": X, "y": Y + 8}, {"x": X + 12, "y": Y + 8})


def dim(id_, layer, a, b):
    return {"kind": "dimension", "id": id_, "layerId": layer, "attrs": {}, "a": dict(a), "b": dict(b),
            "offset": 1.5, "height": 0.25}


ENTITIES = [
    {"kind": "polyline", "id": 1, "layerId": "duvar", "attrs": {}, "pts": copy.deepcopy(WALL)},
    {"kind": "circle", "id": 2, "layerId": "duvar", "attrs": {}, "c": dict(CENTRE), "r": 3},
    dim(3, "olcu", {"x": X, "y": Y}, {"x": X + 10, "y": Y}),
    dim(4, "kilitli", {"x": X, "y": Y + 20}, {"x": X + 10, "y": Y + 20}),
    {"kind": "line", "id": 5, "layerId": "duvar", "attrs": {}, "a": dict(LINE[0]), "b": dict(LINE[1])},
    {"kind": "point", "id": 6, "layerId": "duvar", "attrs": {}, "p": {"x": X + 24, "y": Y + 7.5}},
    {"kind": "insert", "id": 7, "layerId": "duvar", "attrs": {}, "block": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001",
     "p": {"x": X + 30, "y": Y}, "scale": 1, "rotation": 0},
]
IDS = [e["id"] for e in ENTITIES]

# A room block (a floor and a door inside it) and the door (a line); docs/adr/0263 §3.
DOOR_ID, ROOM_ID = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"
BLOCKS = [
    {"id": ROOM_ID, "name": "Oda", "base": {"x": 0, "y": 0}, "entities": [
        {"kind": "polygon", "id": 1, "layerId": "0", "attrs": {},
         "pts": [{"x": 0, "y": 0}, {"x": 4, "y": 0}, {"x": 4, "y": 3}, {"x": 0, "y": 3}]},
        {"kind": "insert", "id": 2, "layerId": "0", "attrs": {}, "block": DOOR_ID, "p": {"x": 1, "y": 0}, "scale": 2,
         "rotation": 0},
    ]},
    {"id": DOOR_ID, "name": "Kapı", "base": {"x": 0, "y": 0}, "entities": [
        {"kind": "line", "id": 1, "layerId": "0", "attrs": {}, "a": {"x": 0, "y": 0}, "b": {"x": 2, "y": 0}},
    ]},
]


def setup():
    return {
        "format": "kentos.document", "version": 1, "name": "Ölçüler",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 100, "workspace": "cad", "drawingFont": "barlow"},
        "origin": {"x": X, "y": Y},
        "layers": copy.deepcopy(LAYERS),
        "activeLayer": "olcu",
        "entities": copy.deepcopy(ENTITIES),
        "styles": {"items": [], "categories": []},
        "blocks": copy.deepcopy(BLOCKS),
    }


# ── The places, from their definitions ──────────────────────────────────

def vertex(index, part=0, ring=0):
    return {"kind": "vertex", "part": part, "ring": ring, "index": index}


def along(index, t, part=0, ring=0):
    return {"kind": "along", "part": part, "ring": ring, "index": index, "t": t}


def piece(object_id, path):
    """The object a block path names in an insert, placed (docs/adr/0263 §3): each number the place in its level's
    definition, the last one not an insert; the inserts here only move and scale. None when there is none."""
    e = next(x for x in ENTITIES if x["id"] == object_id)
    if e["kind"] != "insert" or not path or len(path) > 16:
        return None
    sx, ox, oy, cur = 1.0, 0.0, 0.0, e
    for depth, i in enumerate(path):
        d = next((b for b in BLOCKS if b["id"] == cur["block"]), None)
        if d is None or i >= len(d["entities"]):
            return None
        # x' = o + s·(scale·(x − base) + p)
        ox, oy = ox + sx * (cur["p"]["x"] - cur["scale"] * d["base"]["x"]), oy + sx * (cur["p"]["y"] - cur["scale"] * d["base"]["y"])
        sx *= cur["scale"]
        nxt = d["entities"][i]
        if depth < len(path) - 1:
            if nxt["kind"] != "insert":
                return None
            cur = nxt
        else:
            if nxt["kind"] == "insert":
                return None
            m = lambda q: {"x": ox + sx * q["x"], "y": oy + sx * q["y"]}  # noqa: E731
            out = copy.deepcopy(nxt)
            for k in ("a", "b", "c", "p"):
                if k in out:
                    out[k] = m(out[k])
            if "pts" in out:
                out["pts"] = [m(q) for q in out["pts"]]
            return out
    return None


def place_point(object_id, place, path=None):
    """Where a place on an object is, by the place's definition (docs/adr/0249 §1, 0263 §1); none when the object has
    none."""
    e = next(x for x in ENTITIES if x["id"] == object_id)
    if path:
        e = piece(object_id, path)
        if e is None:
            return None
    kind, where = e["kind"], place["kind"]
    if where == "along":
        t = place["t"]
        if kind == "line" and (place["part"], place["ring"], place["index"]) == (0, 0, 0):
            a, b = e["a"], e["b"]
        elif kind in ("polyline", "polygon") and place["part"] == 0 and place["ring"] == 0:
            n, k = len(e["pts"]), place["index"]
            if k >= n or (kind == "polyline" and k + 1 >= n):
                return None
            a, b = e["pts"][k], e["pts"][(k + 1) % n]
        elif kind == "circle" and (place["part"], place["ring"], place["index"]) == (0, 0, 0):
            return {"x": e["c"]["x"] + math.cos(t * 2 * math.pi) * e["r"], "y": e["c"]["y"] + math.sin(t * 2 * math.pi) * e["r"]}
        else:
            return None
        return {"x": a["x"] + (b["x"] - a["x"]) * t, "y": a["y"] + (b["y"] - a["y"]) * t}
    if kind in ("polyline", "polygon") and where == "vertex" and place["part"] == 0 and place["ring"] == 0:
        return e["pts"][place["index"]] if place["index"] < len(e["pts"]) else None
    if kind == "polyline" and where == "mid" and place["part"] == 0 and place["ring"] == 0:
        k = place["index"]
        if k + 1 >= len(e["pts"]):
            return None
        p, q = e["pts"][k], e["pts"][k + 1]
        return {"x": (p["x"] + q["x"]) / 2, "y": (p["y"] + q["y"]) / 2}
    if kind == "line" and where in ("start", "end"):
        return e["a"] if where == "start" else e["b"]
    if kind == "line" and where == "mid" and place == {"kind": "mid", "part": 0, "ring": 0, "index": 0}:
        return {"x": (e["a"]["x"] + e["b"]["x"]) / 2, "y": (e["a"]["y"] + e["b"]["y"]) / 2}
    if kind == "circle" and where == "center":
        return e["c"]
    if kind == "point" and where == "node" and place["part"] == 0:
        return e["p"]
    return None


def num(v):
    return int(v) if float(v).is_integer() else v


def pt(p):
    return {"x": num(p["x"]), "y": num(p["y"])}


def js_hypot(*xs):
    """JavaScript's Math.hypot as V8 computes it (the rule the shared core keeps on every platform): each term scaled
    by the largest, the squares summed with Kahan's compensation, the root scaled back. It may differ from the correctly
    rounded root by an ulp."""
    big = max(abs(x) for x in xs)
    if big == 0:
        return 0.0
    total, comp = 0.0, 0.0
    for x in xs:
        n = abs(x) / big
        term = n * n - comp
        pre = total + term
        comp = (pre - total) - term
        total = pre
    return math.sqrt(total) * big


def aligned_value(a, b):
    """An aligned dimension measures the distance between its points (docs/adr/0061) as its layout does: the
    difference projected on its own direction, the direction the difference over its length (JavaScript's hypot)."""
    dx, dy = b["x"] - a["x"], b["y"] - a["y"]
    length = js_hypot(dx, dy)
    return abs(dx * (dx / length) + dy * (dy / length))


# ── The answers ─────────────────────────────────────────────────────────

def completed(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def step(op, input_=None, result=None, expect=None, **extra):
    s = {"op": op}
    if input_ is not None:
        s["input"] = input_
    if result is not None:
        s["result"] = result
    s.update(extra)
    if expect is not None:
        s["expect"] = expect
    return s


UNCHANGED = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
INVALID_REVISION = "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."
BAD_UID = "“{}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
MISSING = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"


def refused(name, input_, code, message, path):
    return {"name": name, "steps": [step("execute", input_, failed(code, message, path), UNCHANGED)]}


def bind(point, object_id, place, path=None):
    to = {"uid": f"$uidOf:{object_id}", "place": place}
    if path:
        to["path"] = path
    return {"point": point, "to": to}


def free(point):
    return {"point": point, "to": None}


def bound(dimension_id, bindings):
    """The dimension after the bindings, by the contract's rule: a bound point moves to its place and keeps a live
    reference; a freed one keeps its coordinates and loses its reference; no reference left, no field."""
    d = copy.deepcopy(next(x for x in ENTITIES if x["id"] == dimension_id))
    refs = {}
    for b in bindings:
        if b["to"] is None:
            refs.pop(b["point"], None)
            continue
        object_id = int(b["to"]["uid"].split(":")[1])
        d[b["point"]] = pt(place_point(object_id, b["to"]["place"], b["to"].get("path")))
        refs[b["point"]] = {"uid": b["to"]["uid"], "place": copy.deepcopy(b["to"]["place"])}
        if b["to"].get("path"):
            refs[b["point"]]["path"] = list(b["to"]["path"])
    if refs:
        d["refs"] = {k: refs[k] for k in ("a", "b", "c") if k in refs}
    return d


def cases():
    out = []
    ends = [bind("a", 1, vertex(0)), bind("b", 1, vertex(1))]
    both = bound(3, ends)
    out.append({
        "name": "İki uç duvarın iki köşesine: b köşeye gider, değer 12; adımı “Ölçüyü yeniden bağla”, geri alınır ve yinelenir",
        "steps": [
            step("execute", {"uid": "$uidOf:3", "points": ends},
                 completed({"changed": 2, "value": num(aligned_value(both["a"], both["b"])), "unit": "length", "revision": "$current"}),
                 {"ids": IDS, "entities": {"3": both}, "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}),
            step("undo", returns=STEP, expect={"entities": {"3": ENTITIES[2]}, "canUndo": False, "canRedo": True}),
            step("redo", returns=STEP, expect={"entities": {"3": both}, "canUndo": True, "canRedo": False}),
        ],
    })
    centre = [bind("a", 2, {"kind": "center"})]
    to_centre = bound(3, centre)
    out.append({
        "name": "Bir uç dairenin merkezine: öbür uç serbest kalır; değer 5",
        "steps": [step("execute", {"uid": "$uidOf:3", "points": centre},
                       completed({"changed": 1, "value": num(aligned_value(to_centre["a"], to_centre["b"])), "unit": "length", "revision": "$current"}),
                       {"entities": {"3": to_centre}, "canUndo": True})],
    })
    mid = [bind("b", 5, {"kind": "mid", "part": 0, "ring": 0, "index": 0})]
    to_mid = bound(3, mid)
    out.append({
        "name": "Bir uç çizginin ortasına; değer 10",
        "steps": [step("execute", {"uid": "$uidOf:3", "points": mid},
                       completed({"changed": 1, "value": num(aligned_value(to_mid["a"], to_mid["b"])), "unit": "length", "revision": "$current"}),
                       {"entities": {"3": to_mid}})],
    })
    edge = [bind("a", 1, {"kind": "mid", "part": 0, "ring": 0, "index": 1}), bind("b", 6, {"kind": "node", "part": 0})]
    to_edge = bound(3, edge)
    out.append({
        "name": "Kenar ortası ve nokta: a yolun ikinci kenarının ortasına, b noktaya; değer 13",
        "steps": [step("execute", {"uid": "$uidOf:3", "points": edge},
                       completed({"changed": 2, "value": num(aligned_value(to_edge["a"], to_edge["b"])), "unit": "length", "revision": "$current"}),
                       {"entities": {"3": to_edge}})],
    })
    # Places along an edge or a curve and pieces inside a block (docs/adr/0263).
    for name, bindings in [
        ("Bir uç çizginin çeyreğine (along, t 0,25)", [bind("b", 5, along(0, 0.25))]),
        ("Bir uç yolun ikinci kenarının yarısına (along)", [bind("b", 1, along(1, 0.5))]),
        ("Bir uç dairenin kuzeyine (along, t 0,25)", [bind("b", 2, along(0, 0.25))]),
        ("Bir uç odadaki kapının sonuna (blok yolu [1, 0])", [bind("a", 7, {"kind": "end"}, [1, 0])]),
        ("İki uç bloğun içinde: odanın tabanının köşesi ve kapının yarısı",
         [bind("a", 7, vertex(0), [0]), bind("b", 7, along(0, 0.5), [1, 0])]),
    ]:
        after = bound(3, bindings)
        out.append({"name": name, "steps": [step("execute", {"uid": "$uidOf:3", "points": bindings},
                    completed({"changed": len(bindings), "value": num(aligned_value(after["a"], after["b"])), "unit": "length", "revision": "$current"}),
                    {"entities": {"3": after}, "canUndo": True})]})
    freed = bound(3, ends + [free("b")])
    out.append({
        "name": "Bağla, sonra bir ucu serbest bırak: nokta yerinde kalır, bağı düşer; ikisi de serbestse alan yazılmaz",
        "steps": [
            step("execute", {"uid": "$uidOf:3", "points": ends}, completed({"changed": 2, "value": 12, "unit": "length", "revision": "$current"}), {"entities": {"3": both}}),
            step("execute", {"uid": "$uidOf:3", "points": [free("b")]},
                 completed({"changed": 1, "value": 12, "unit": "length", "revision": "$current"}),
                 {"entities": {"3": freed}}),
            step("execute", {"uid": "$uidOf:3", "points": [free("a")]},
                 completed({"changed": 1, "value": 12, "unit": "length", "revision": "$current"}),
                 {"entities": {"3": {k: v for k, v in freed.items() if k != "refs"}}}),
        ],
    })
    out.append({
        "name": "Değişmeyen bağ hiçbir şey yazmaz: changed 0",
        "steps": [
            step("execute", {"uid": "$uidOf:3", "points": ends}, completed({"changed": 2, "value": 12, "unit": "length", "revision": "$current"}), {"entities": {"3": both}}),
            step("captureRevision", **{"as": "after"}),
            step("execute", {"uid": "$uidOf:3", "points": [bind("a", 1, vertex(0))]},
                 completed({"changed": 0, "value": 12, "unit": "length", "revision": "$after"}),
                 {"entities": {"3": both}, "revision": "same"}),
        ],
    })
    out.append({
        "name": "Serbest noktayı serbest bırakmak hiçbir şey yazmaz",
        "steps": [step("execute", {"uid": "$uidOf:3", "points": [free("a")]},
                       completed({"changed": 0, "value": 10, "unit": "length", "revision": "$current"}), UNCHANGED)],
    })
    plan_both = copy.deepcopy(both)
    out.append({
        "name": "Plan: ölçü olacağı gibi, değeri ve sürüm; hiçbir şey yazılmaz; planın sürümüyle yazma planı yazar",
        "steps": [
            step("plan", {"uid": "$uidOf:3", "points": ends},
                 completed({"dimension": plan_both, "changed": 2, "value": 12, "unit": "length", "revision": "$current"}), UNCHANGED),
            step("execute", {"uid": "$uidOf:3", "points": ends, "expectedRevision": "$current"},
                 completed({"changed": 2, "value": 12, "unit": "length", "revision": "$current"}),
                 {"entities": {"3": both}, "revision": "changed"}),
        ],
    })
    out.append({
        "name": "Beklenen sürüm geçmişte kaldıysa conflict, hiçbir şey yazılmaz",
        "steps": [
            step("captureRevision", **{"as": "old"}),
            step("execute", {"uid": "$uidOf:3", "points": ends}, completed({"changed": 2, "value": 12, "unit": "length", "revision": "$current"}), {"entities": {"3": both}}),
            step("execute", {"uid": "$uidOf:3", "points": [free("a")], "expectedRevision": "$old"},
                 {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}},
                 {"entities": {"3": both}}),
        ],
    })
    # Refusals, in the contract's order.
    out.append(refused("Ölçünün kimliği geçersiz", {"uid": "üç", "points": ends}, "invalid_uid", BAD_UID.format("üç"), "uid"))
    out.append(refused("Yerin nesnesinin kimliği geçersiz: kimlikler boş bağdan önce",
                       {"uid": "$uidOf:3", "points": [{"point": "a", "to": {"uid": "DUVAR", "place": vertex(0)}}]},
                       "invalid_uid", BAD_UID.format("DUVAR"), "points[0].to.uid"))
    out.append(refused("Bağ yok", {"uid": "$uidOf:3", "points": []}, "no_entities",
                       "Bağlanacak nokta yok. Ölçünün en az bir noktasını (a, b ya da c) verin.", "points"))
    out.append(refused("Aynı nokta iki kez", {"uid": "$uidOf:3", "points": [bind("a", 1, vertex(0)), free("a")]}, "invalid_refs",
                       "Ölçünün “a” noktası iki kez veriliyor; her noktayı bir kez verin.", "points[1].point"))
    out.append(refused("Yerin sırası çok büyük", {"uid": "$uidOf:3", "points": [bind("a", 1, vertex(10_000_001))]}, "invalid_refs",
                       "Yerin parçası, halkası ya da sırası çok büyük.", "points[0].to.place"))
    out.append(refused("Yerin t'si 0 ile 1 arasında değil", {"uid": "$uidOf:3", "points": [bind("a", 5, along(0, 1.5))]}, "invalid_refs",
                       "Yerin t'si 1.5: 0 ile 1 arasında olmalı.", "points[0].to.place"))
    out.append(refused("Blok yolu 16 düzeyden derin", {"uid": "$uidOf:3", "points": [bind("a", 7, {"kind": "end"}, [0] * 17)]}, "invalid_refs",
                       "Blok yolu 17 düzey; en çok 16 olur.", "points[0].to.path"))
    out.append(refused("Beklenen sürüm sayı değil", {"uid": "$uidOf:3", "points": ends, "expectedRevision": "on iki"},
                       "invalid_revision", INVALID_REVISION, "expectedRevision"))
    out.append(refused("Ölçü çizimde yok", {"uid": MISSING, "points": ends}, "entity_not_found",
                       f"“{MISSING}” kimlikli ölçü çizimde yok: silinmiş ya da başka bir çizimin olabilir.", "uid"))
    out.append(refused("Yerin nesnesi çizimde yok", {"uid": "$uidOf:3", "points": [free("a"), {"point": "b", "to": {"uid": MISSING, "place": vertex(0)}}]},
                       "entity_not_found",
                       f"“{MISSING}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.",
                       "points[1].to.uid"))
    out.append(refused("Ölçü değil", {"uid": "$uidOf:1", "points": [bind("a", 2, {"kind": "center"})]}, "not_a_dimension",
                       "Verilen nesne bir ölçü değil; Yeniden bağla yalnız ölçülerin noktalarını bağlar.", "uid"))
    out.append(refused("Kilitli grubun altındaki ölçü", {"uid": "$uidOf:4", "points": ends}, "layer_locked",
                       "“Eski ölçü” katmanı kilitli; üzerindeki ölçü değişmez. Kilidi Katmanlar panelinden açın.", "uid"))
    out.append(refused("Ölçünün c noktası yok", {"uid": "$uidOf:3", "points": [bind("c", 2, {"kind": "center"})]}, "invalid_refs",
                       "Bu ölçünün üçüncü noktası (c) yok; yalnız a ve b bağlanır.", "points[0].point"))
    out.append(refused("Ölçü kendisine bağlanamaz", {"uid": "$uidOf:3", "points": [bind("a", 3, {"kind": "start"})]}, "invalid_refs",
                       "Ölçünün noktası ölçünün kendisine bağlanamaz; ölçülen nesnenin bir yerini verin.", "points[0].to.uid"))
    no_place = "Nesnede böyle bir yer yok: köşe sırası halkanın dışında ya da yer bu nesne türünde bulunmuyor."
    assert place_point(1, vertex(3)) is None and place_point(5, {"kind": "center"}) is None
    out.append(refused("Köşe sırası halkanın dışında", {"uid": "$uidOf:3", "points": [bind("a", 1, vertex(3))]}, "no_such_place",
                       no_place, "points[0].to.place"))
    out.append(refused("Çizginin merkezi yok", {"uid": "$uidOf:3", "points": [bind("b", 5, {"kind": "center"})]}, "no_such_place",
                       no_place, "points[0].to.place"))
    out.append(refused("Çizginin ikinci kenarı yok (along)", {"uid": "$uidOf:3", "points": [bind("b", 5, along(1, 0.5))]}, "no_such_place",
                       no_place, "points[0].to.place"))
    no_piece = "Blok yolunda böyle bir parça yok: yolun bir sırası tanımın dışında, yol bir yerleştirmede bitiyor ya da yerleştirme olmayan bir nesneden geçiyor."
    assert piece(5, [0]) is None and piece(7, [9]) is None and piece(7, [1]) is None and piece(7, [0, 0]) is None
    out.append(refused("Blok yolu yerleştirme olmayan nesnede", {"uid": "$uidOf:3", "points": [bind("a", 5, {"kind": "start"}, [0])]}, "piece_gone",
                       no_piece, "points[0].to.path"))
    out.append(refused("Blok yolunun sırası tanımın dışında", {"uid": "$uidOf:3", "points": [bind("a", 7, {"kind": "start"}, [9])]}, "piece_gone",
                       no_piece, "points[0].to.path"))
    out.append(refused("Blok yolu yerleştirmede bitiyor", {"uid": "$uidOf:3", "points": [bind("a", 7, {"kind": "start"}, [1])]}, "piece_gone",
                       no_piece, "points[0].to.path"))
    out.append(refused("Blok yolu tabandan içeri geçemez", {"uid": "$uidOf:3", "points": [bind("a", 7, vertex(0), [0, 0])]}, "piece_gone",
                       no_piece, "points[0].to.path"))
    out.append(refused("Blok parçasında olmayan yer", {"uid": "$uidOf:3", "points": [bind("a", 7, {"kind": "center"}, [1, 0])]}, "no_such_place",
                       no_place, "points[0].to.place"))
    out.append(refused("Açık yolun son köşesinden sonra kenar yok", {"uid": "$uidOf:3", "points": [bind("b", 1, {"kind": "mid", "part": 0, "ring": 0, "index": 2})]},
                       "no_such_place", no_place, "points[0].to.place"))
    return out


def build():
    return {
        "format": "kentos.command-cases", "version": 1, "command": COMMAND, "commandVersion": 1,
        "title": STEP,
        "note": "Written by scripts/fixtures/dimension_command_cases.py from the contract's rules and docs/adr/0249 §1, §3, 0263; do not edit by hand.",
        "setup": setup(),
        "cases": cases(),
    }


def main() -> int:
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check'siz çalıştırın.")
            return 1
        print(f"{OUT.relative_to(ROOT)}: güncel")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı")
    return 0


if __name__ == "__main__":
    sys.exit(main())
