"""The shared cases of the product command cad.entities.create (docs/adr/0057).

    python3 scripts/fixtures/create_command_cases.py           # writes the file
    python3 scripts/fixtures/create_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.create.json. The checks, their order,
codes, paths and messages are written here by hand from the ADR, and so is
what an object becomes (`made` below: the contract's rule, not an
implementation's output). The geometry is given in the input; the web's and
the desktop's handlers must write it as given.

`--check` rebuilds the file in memory and compares it with the one on disk.
"""
import json
import math
import sys

style = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}


def layer(i, name, visible=True, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True, "style": style, "children": []}


def group(i, name, children, visible=True, locked=False):
    return {"id": i, "name": name, "type": "group", "visible": visible, "locked": locked, "expanded": True, "style": style, "children": children}


def P(x, y):
    return {"x": x, "y": y}


ENTITIES = [
    {"kind": "line", "id": 1, "layerId": "yapi", "attrs": {}, "a": P(487000, 4420000), "b": P(487020, 4420000)},
    {"kind": "circle", "id": 2, "layerId": "kilitli", "attrs": {}, "c": P(487040, 4420000), "r": 5},
]
IDS = [e["id"] for e in ENTITIES]

SETUP = {
    "format": "kentos.document",
    "version": 1,
    "name": "Ürün komutu",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000, "workspace": "hybrid", "drawingFont": "barlow"},
    "origin": {"x": 487000, "y": 4420000},
    "layers": [
        layer("yapi", "Yapı"),
        layer("kilitli", "Kilitli katman", locked=True),
        layer("gizli", "Gizli katman", visible=False),
        group("plan", "Plan", [layer("ada", "Ada")]),
        group("arsiv", "Arşiv", [layer("eski", "Eski")], locked=True),
        group("sakli", "Saklı grup", [layer("icerde", "İçerideki")], visible=False),
    ],
    "activeLayer": "yapi",
    "entities": ENTITIES,
    "styles": {"items": [], "categories": []},
}

# ── What an object becomes (the contract) ──────────────────────────────


def made(obj, slot, layer_id="yapi"):
    """An object as the document stores it: its geometry, its slot, the input's
    layer, and its colour, attributes (none: empty) and label when given."""
    out = json.loads(json.dumps(obj["geometry"]))
    out["id"] = slot
    out["layerId"] = layer_id
    if "color" in obj:
        out["color"] = obj["color"]
    out["attrs"] = obj.get("attrs", {})
    if "label" in obj:
        out["label"] = obj["label"]
    return out


def O(geometry, **fields):
    return {"geometry": geometry, **fields}


def uid(i):
    return f"$uidOf:{i}"


def done(slots, warnings=()):
    return {"status": "completed", "output": {"created": [uid(i) for i in slots], "ids": list(slots), "revision": "$current"}, "warnings": list(warnings)}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def hidden(name):
    return {"code": "layer_hidden", "message": f"“{name}” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}


def locked(name):
    return failed("layer_locked", f"“{name}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.", "layerId")


def not_finite(n):
    return failed("not_finite", f"{n}. nesnenin geometrisinde sonlu olmayan bir değer var (NaN ya da sonsuz). Geometriyi sonlu sayılarla verin.", f"objects[{n - 1}].geometry")


NOTHING = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."

# The geometry the drawing tools give (their shapes, written as the preview showed them).
ELLIPSE = {"kind": "ellipse", "c": P(487010, 4420020), "major": P(8, 6), "ratio": 0.5, "t0": 0, "t1": 0}
ELLIPTIC_ARC = {"kind": "ellipse", "c": P(487010, 4420020), "major": P(0, 10), "ratio": 0.25, "t0": 0.5, "t1": 2.5}
SPLINE = {"kind": "spline", "pts": [P(487000, 4420030), P(487005, 4420034), P(487010, 4420030), P(487015, 4420034)], "closed": False}
CLOSED_SPLINE = {"kind": "spline", "pts": [P(487020, 4420030), P(487025, 4420036), P(487030, 4420030)], "closed": True}
XLINE = {"kind": "xline", "p": P(487000, 4420040), "dir": P(0.6, 0.8)}
RAY = {"kind": "ray", "p": P(487000, 4420040), "dir": P(-1, 0)}
RING = [P(487050 + 0.5 * math.cos(i * math.pi / 4), 4420050 + 0.5 * math.sin(i * math.pi / 4)) for i in range(8)]
HOLE = [P(487050 + 0.25 * math.cos(i * math.pi / 4), 4420050 + 0.25 * math.sin(i * math.pi / 4)) for i in range(8)]
DONUT = {"kind": "hatch", "ring": RING, "holes": [HOLE], "pattern": {"type": "solid", "angle": 0, "spacing": 1}}
LEFT = {"kind": "polyline", "pts": [P(487000, 4420065), P(487030, 4420065), P(487030, 4420095)]}
RIGHT = {"kind": "polyline", "pts": [P(487000, 4420055), P(487040, 4420055), P(487040, 4420095)]}
AXIS = {"kind": "polyline", "pts": [P(487000, 4420060), P(487035, 4420060), P(487035, 4420095)]}
CORRIDOR = {"kind": "polygon", "pts": [P(487095, 4420095), P(487165, 4420095), P(487165, 4420165), P(487095, 4420165)],
            "holes": [{"pts": [P(487105, 4420105), P(487105, 4420155), P(487155, 4420155), P(487155, 4420105)]}]}
AXIS_RING = {"kind": "polygon", "pts": [P(487100, 4420100), P(487160, 4420100), P(487160, 4420160), P(487100, 4420160)]}
FOOT = {"kind": "line", "a": P(487012, 4420007), "b": P(487012, 4420000)}
OUT = {"kind": "line", "a": P(487005, 4420000), "b": P(487005, 4419996)}


def point(x, y):
    return {"kind": "point", "p": P(x, y)}


# Böl: a 20 m line in five parts.
DIVISION = [point(487004 + i * 4, 4420000) for i in range(4)]

cases = []

# ── Writing ────────────────────────────────────────────────────────────

cases.append({
    "name": "Elips: tek nesne, adı “Ekle” olan tek adım; geri alınır, aynı kimlikle yinelenir",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(ELLIPSE)]}, "result": done([3]),
         "expect": {"ids": IDS + [3], "entities": {"3": made(O(ELLIPSE), 3)}, "uids": {"3": "new"}, "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}},
        {"op": "captureUid", "id": 3, "as": "elips"},
        {"op": "undo", "returns": "Ekle", "note": "Çizim araçlarının hep yazdığı ad.", "expect": {"ids": IDS, "canUndo": False, "canRedo": True, "revision": "changed"}},
        {"op": "redo", "returns": "Ekle", "expect": {"ids": IDS + [3], "entities": {"3": made(O(ELLIPSE), 3)}, "uids": {"3": "elips"}, "canUndo": True, "canRedo": False, "revision": "changed"}},
    ],
})

cases.append({
    "name": "eliptik yay: parametreleri olduğu gibi yazılır",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(ELLIPTIC_ARC)]}, "result": done([3]),
         "expect": {"entities": {"3": made(O(ELLIPTIC_ARC), 3)}, "revision": "changed"}},
    ],
})

cases.append({
    "name": "Eğri: açık ve kapalı iki eğri girdinin sırasıyla yazılır; ikisi tek adımdır",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(SPLINE), O(CLOSED_SPLINE)]}, "result": done([3, 4]),
         "expect": {"ids": IDS + [3, 4], "entities": {"3": made(O(SPLINE), 3), "4": made(O(CLOSED_SPLINE), 4)}, "uids": {"3": "new", "4": "new"}, "revision": "changed"}},
        {"op": "undo", "returns": "Ekle", "expect": {"ids": IDS, "canUndo": False}},
    ],
})

cases.append({
    "name": "Yardımcı çizgi ve Işın: nokta ve yön olduğu gibi yazılır",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(XLINE)]}, "result": done([3]),
         "expect": {"entities": {"3": made(O(XLINE), 3)}, "revision": "changed"}},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(RAY)]}, "result": done([4]),
         "expect": {"entities": {"4": made(O(RAY), 4)}, "revision": "changed"}},
        {"op": "undo", "returns": "Ekle", "note": "Her yazma kendi adımıdır.", "expect": {"ids": IDS + [3]}},
    ],
})

cases.append({
    "name": "Halka: dolu tarama, dış halka ve delik",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(DONUT)]}, "result": done([3]),
         "expect": {"entities": {"3": made(O(DONUT), 3)}, "revision": "changed"}},
    ],
})

cases.append({
    "name": "Paralel çizgi: iki yan ve eksen tek adımda; adım aracın adıdır",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "operation": "parallel", "objects": [O(LEFT), O(RIGHT), O(AXIS)]}, "result": done([3, 4, 5]),
         "expect": {"ids": IDS + [3, 4, 5], "entities": {"3": made(O(LEFT), 3), "4": made(O(RIGHT), 4), "5": made(O(AXIS), 5)}, "revision": "changed"}},
        {"op": "undo", "returns": "Paralel çizgi", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}},
        {"op": "redo", "returns": "Paralel çizgi", "expect": {"ids": IDS + [3, 4, 5]}},
    ],
})

cases.append({
    "name": "Paralel çizgi alan olarak, kapalı eksenle: delikli kapalı alan ve eksen",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "operation": "parallel", "objects": [O(CORRIDOR), O(AXIS_RING)]}, "result": done([3, 4]),
         "expect": {"entities": {"3": made(O(CORRIDOR), 3), "4": made(O(AXIS_RING), 4)}, "revision": "changed"}},
        {"op": "undo", "returns": "Paralel çizgi"},
    ],
})

cases.append({
    "name": "Dik in ve Dik çık: birer çizgi, adımın adı aracın adıdır",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "operation": "perpendicularIn", "objects": [O(FOOT)]}, "result": done([3]),
         "expect": {"entities": {"3": made(O(FOOT), 3)}, "revision": "changed"}},
        {"op": "execute", "input": {"layerId": "yapi", "operation": "perpendicularOut", "objects": [O(OUT)]}, "result": done([4]),
         "expect": {"entities": {"4": made(O(OUT), 4)}, "revision": "changed"}},
        {"op": "undo", "returns": "Dik çık"},
        {"op": "undo", "returns": "Dik in", "expect": {"ids": IDS, "canUndo": False}},
    ],
})

cases.append({
    "name": "Böl: noktalar girdinin sırasıyla, tek adımda; adı “Böl”",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "operation": "divide", "objects": [O(p) for p in DIVISION]}, "result": done([3, 4, 5, 6]),
         "expect": {"ids": IDS + [3, 4, 5, 6], "entities": {str(3 + i): made(O(p), 3 + i) for i, p in enumerate(DIVISION)}, "revision": "changed"}},
        {"op": "undo", "returns": "Böl", "expect": {"ids": IDS, "canUndo": False}},
    ],
})

# Tarama: a parcel with a building left out as an island, lines at 45°, 3 m apart (3 mm at 1:1000).
HATCH = {"kind": "hatch", "ring": [P(487000, 4420120), P(487040, 4420120), P(487040, 4420150), P(487000, 4420150)],
         "holes": [[P(487010, 4420130), P(487020, 4420130), P(487020, 4420138), P(487010, 4420138)]],
         "pattern": {"type": "lines", "angle": 45, "spacing": 3}}

cases.append({
    "name": "Tarama: halka, adası ve deseniyle tek adım; adı “Tarama”",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "operation": "hatch", "objects": [O(HATCH)]}, "result": done([3]),
         "expect": {"ids": IDS + [3], "entities": {"3": made(O(HATCH), 3)}, "revision": "changed"}},
        {"op": "undo", "returns": "Tarama", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}},
        {"op": "redo", "returns": "Tarama", "expect": {"ids": IDS + [3]}},
    ],
})

marked = [
    O({"kind": "point", "p": P(487060, 4420010), "z": 12.5}, color="#E5484D", attrs={"Tür": "Kot noktası", "Z (m)": "12.500"}, label="12.50"),
    O(FOOT),
    O(ELLIPSE, attrs={}),
]
cases.append({
    "name": "renk, öznitelik ve etiket nesne nesne yazılır; verilmeyen yoktur, boş öznitelik boştur",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": marked}, "result": done([3, 4, 5]),
         "expect": {"entities": {"3": made(marked[0], 3), "4": made(marked[1], 4), "5": made(marked[2], 5)}, "revision": "changed"}},
    ],
})

every = [
    O({"kind": "polygon", "pts": [P(487000, 4420100), P(487010, 4420100), P(487010, 4420110)], "bulges": [0, 0.5, 0]}),
    O({"kind": "circle", "c": P(487020, 4420105), "r": 3}),
    O({"kind": "arc", "c": P(487030, 4420105), "r": 3, "a0": 0, "a1": 2}),
    O({"kind": "polyline", "pts": [P(487040, 4420100), P(487050, 4420110)], "bulges": [0.25]}),
    O({"kind": "text", "p": P(487060, 4420100), "text": "Ada 101", "height": 2, "rotation": 30}),
    O({"kind": "dimension", "a": P(487000, 4420120), "b": P(487020, 4420120), "offset": 3, "height": 0.5, "style": "linear", "angle": 0}),
]
cases.append({
    "name": "öbür türler de yazılır: kapalı alan (yay değerleriyle), daire, yay, çoklu çizgi, yazı, ölçü",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": every}, "result": done([3, 4, 5, 6, 7, 8]),
         "expect": {"ids": IDS + [3, 4, 5, 6, 7, 8], "entities": {str(3 + i): made(o, 3 + i) for i, o in enumerate(every)}, "revision": "changed"}},
    ],
})

cases.append({
    "name": "grubun içindeki katmana yazılır",
    "steps": [
        {"op": "execute", "input": {"layerId": "ada", "objects": [O(SPLINE)]}, "result": done([3]),
         "expect": {"entities": {"3": made(O(SPLINE), 3, "ada")}, "revision": "changed"}},
    ],
})

cases.append({
    "name": "gizli katmana uyarıyla yazılır; nesneler yine yazılır",
    "steps": [
        {"op": "execute", "input": {"layerId": "gizli", "operation": "divide", "objects": [O(p) for p in DIVISION[:2]]}, "result": done([3, 4], [hidden("Gizli katman")]),
         "expect": {"ids": IDS + [3, 4], "entities": {"3": made(O(DIVISION[0]), 3, "gizli"), "4": made(O(DIVISION[1]), 4, "gizli")}, "revision": "changed"}},
    ],
})

cases.append({
    "name": "gizli grubun katmanı da gizlidir; uyarı katmanın adını verir",
    "steps": [
        {"op": "execute", "input": {"layerId": "icerde", "objects": [O(XLINE)]}, "result": done([3], [hidden("İçerideki")]),
         "expect": {"entities": {"3": made(O(XLINE), 3, "icerde")}, "revision": "changed"}},
    ],
})

# ── Refusals, in the contract's order ──────────────────────────────────

cases.append({
    "name": "nesne verilmedi: no_objects",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": []}, "result": failed("no_objects", "Eklenecek nesne verilmedi. En az bir nesne verin.", "objects"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "kilitli", "operation": "divide", "objects": []}, "result": failed("no_objects", "Eklenecek nesne verilmedi. En az bir nesne verin.", "objects"),
         "note": "Boş girdi katmandan önce söylenir.", "expect": NOTHING},
    ],
})

cases.append({
    "name": "çoklu çizginin en az 2 noktası, kapalı alanın, deliğinin ve taramanın en az 3 köşesi olmalı; yol nesnenin sırasını verir",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "operation": "parallel", "objects": [O(LEFT), O({"kind": "polyline", "pts": [P(487000, 4420010)]})]},
         "result": failed("too_few_points", "Çoklu çizginin en az 2 noktası olmalı; 1 nokta verildi. Eksik noktaları ekleyin.", "objects[1].geometry.pts"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({"kind": "polygon", "pts": [P(487030, 4420000), P(487050, 4420000)]})]},
         "result": failed("too_few_corners", "Kapalı alanın en az 3 köşesi olmalı; 2 köşe verildi. Eksik köşeleri ekleyin.", "objects[0].geometry.pts"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**CORRIDOR, "holes": [CORRIDOR["holes"][0], {"pts": [P(487110, 4420110), P(487112, 4420110)]}]})]},
         "result": failed("too_few_corners", "2. deliğin en az 3 köşesi olmalı; 2 köşe verildi. Eksik köşeleri ekleyin ya da deliği çıkarın.", "objects[0].geometry.holes[1].pts"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**DONUT, "ring": RING[:2]})]},
         "result": failed("too_few_corners", "Taramanın en az 3 köşesi olmalı; 2 köşe verildi. Eksik köşeleri ekleyin.", "objects[0].geometry.ring"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**DONUT, "holes": [HOLE[:2]]})]},
         "result": failed("too_few_corners", "1. deliğin en az 3 köşesi olmalı; 2 köşe verildi. Eksik köşeleri ekleyin ya da deliği çıkarın.", "objects[0].geometry.holes[0]"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "NaN ya da sonsuz değer yazılmaz; ileti hangi nesnenin olduğunu söyler, hiçbiri yazılmaz",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(ELLIPSE)]}, "nonFinite": {"objects[0].geometry.ratio": "NaN"}, "result": not_finite(1), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(ELLIPSE), O(SPLINE)]}, "nonFinite": {"objects[1].geometry.pts[2].y": "Infinity"}, "result": not_finite(2), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(XLINE)]}, "nonFinite": {"objects[0].geometry.dir.x": "-Infinity"}, "result": not_finite(1), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(RAY)]}, "nonFinite": {"objects[0].geometry.p.y": "NaN"}, "result": not_finite(1), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(DONUT)]}, "nonFinite": {"objects[0].geometry.ring[3].x": "NaN"}, "result": not_finite(1), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(ELLIPTIC_ARC)]}, "nonFinite": {"objects[0].geometry.t1": "Infinity"}, "result": not_finite(1), "expect": NOTHING},
    ],
})

cases.append({
    "name": "dairenin ve yayın yarıçapı sıfırdan büyük olmalı",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({"kind": "circle", "c": P(487020, 4420105), "r": 0})]},
         "result": failed("invalid_radius", "Yarıçap sıfırdan büyük olmalı. Pozitif bir yarıçap verin.", "objects[0].geometry.r"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(FOOT), O({"kind": "arc", "c": P(487030, 4420105), "r": -3, "a0": 0, "a1": 2})]},
         "result": failed("invalid_radius", "Yarıçap sıfırdan büyük olmalı. Pozitif bir yarıçap verin.", "objects[1].geometry.r"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "girdinin hatası çizimin durumundan önce gelir: geometri, sürümden ve katmandan önce denetlenir",
    "steps": [
        {"op": "execute", "input": {"layerId": "kilitli", "objects": [O({"kind": "polyline", "pts": [P(487000, 4420010)]})], "expectedRevision": "999"},
         "result": failed("too_few_points", "Çoklu çizginin en az 2 noktası olmalı; 1 nokta verildi. Eksik noktaları ekleyin.", "objects[0].geometry.pts"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "beklenen sürüm ondalık bir tamsayı yazısıdır",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(ELLIPSE)], "expectedRevision": "-1"},
         "result": failed("invalid_revision", "Beklenen sürüm “-1” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(ELLIPSE)], "expectedRevision": "3.0"},
         "result": failed("invalid_revision", "Beklenen sürüm “3.0” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "çizim beklenen sürümde değilse hiçbir şey yazılmaz: conflict; çakışma katmandan önce",
    "steps": [
        {"op": "captureRevision", "as": "r0"},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(XLINE)]}, "result": done([3])},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(RAY)], "expectedRevision": "$r0"},
         "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}},
         "expect": {"ids": IDS + [3], "revision": "same"}},
        {"op": "execute", "input": {"layerId": "kilitli", "objects": [O(RAY)], "expectedRevision": "$r0"},
         "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}},
         "expect": {"ids": IDS + [3], "revision": "same"}},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(RAY)], "expectedRevision": "$current"}, "result": done([4]),
         "note": "Şimdiki sürümle hazırlanan girdi yazılır.", "expect": {"ids": IDS + [3, 4], "revision": "changed"}},
    ],
})

cases.append({
    "name": "bilinmeyen katman: layer_not_found; ad kimlik değildir",
    "steps": [
        {"op": "execute", "input": {"layerId": "Yapı", "objects": [O(ELLIPSE)]},
         "result": failed("layer_not_found", "“Yapı” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "", "objects": [O(ELLIPSE)]},
         "result": failed("layer_not_found", "“” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "grup katman değildir: not_a_layer",
    "steps": [
        {"op": "execute", "input": {"layerId": "plan", "operation": "parallel", "objects": [O(LEFT), O(RIGHT)]},
         "result": failed("not_a_layer", "“Plan” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.", "layerId"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "kilitli katmana hiçbir şey yazılmaz: layer_locked; kilitli grubun katmanına da",
    "note": "Araçların metni: aracın iletisi komuttan gelir (ADR 0022).",
    "steps": [
        {"op": "execute", "input": {"layerId": "kilitli", "operation": "divide", "objects": [O(p) for p in DIVISION]}, "result": locked("Kilitli katman"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "eski", "objects": [O(ELLIPSE)]}, "result": locked("Eski"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "doğrulama hiçbir şey yazmaz; plan yazılacak nesneleri gösterir (yuvaları 0); yazma planın sürümüyle planı yazar; uyarı üç kipte de",
    "steps": [
        {"op": "validate", "input": {"layerId": "gizli", "operation": "parallel", "objects": [O(LEFT), O(AXIS)]},
         "result": {"status": "completed", "output": None, "warnings": [hidden("Gizli katman")]}, "expect": NOTHING},
        {"op": "plan", "input": {"layerId": "gizli", "operation": "parallel", "objects": [O(LEFT), O(AXIS)]},
         "result": {"status": "completed", "output": {"entities": [made(O(LEFT), 0, "gizli"), made(O(AXIS), 0, "gizli")], "revision": "$current"}, "warnings": [hidden("Gizli katman")]}, "expect": NOTHING},
        {"op": "captureRevision", "as": "plan"},
        {"op": "execute", "input": {"layerId": "gizli", "operation": "parallel", "objects": [O(LEFT), O(AXIS)], "expectedRevision": "$plan"},
         "result": done([3, 4], [hidden("Gizli katman")]), "expect": {"entities": {"3": made(O(LEFT), 3, "gizli"), "4": made(O(AXIS), 4, "gizli")}, "revision": "changed"}},
    ],
})

cases.append({
    "name": "doğrulama ve plan geri alma ve yineleme geçmişine dokunmaz",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(XLINE)]}, "result": done([3])},
        {"op": "undo", "returns": "Ekle", "expect": {"canUndo": False, "canRedo": True}},
        {"op": "validate", "input": {"layerId": "yapi", "objects": [O(RAY)]}, "result": {"status": "completed", "output": None, "warnings": []},
         "expect": {"ids": IDS, "canUndo": False, "canRedo": True, "revision": "same"}},
        {"op": "plan", "input": {"layerId": "yapi", "objects": [O(RAY)]}, "result": {"status": "completed", "output": {"entities": [made(O(RAY), 0)], "revision": "$current"}, "warnings": []},
         "expect": {"ids": IDS, "canUndo": False, "canRedo": True, "revision": "same"}},
        {"op": "redo", "returns": "Ekle", "expect": {"ids": IDS + [3]}},
    ],
})


def compact(v):
    return json.dumps(v, ensure_ascii=False, separators=(", ", ": "))


def write(command, title, note, cases):
    fixture = {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title, "note": note, "setup": SETUP, "cases": cases}
    out = ["{"]
    for key in ["format", "version", "command", "commandVersion", "title", "note"]:
        out.append(f'  "{key}": {compact(fixture[key])},')
    s = fixture["setup"]
    out.append('  "setup": {')
    for key in ["format", "version", "name", "settings", "origin"]:
        out.append(f'    "{key}": {compact(s[key])},')
    out.append('    "layers": [')
    out.append(",\n".join(f"      {compact(item)}" for item in s["layers"]))
    out.append("    ],")
    out.append(f'    "activeLayer": {compact(s["activeLayer"])},')
    out.append('    "entities": [')
    out.append(",\n".join(f"      {compact(e)}" for e in s["entities"]))
    out.append("    ],")
    out.append(f'    "styles": {compact(s["styles"])}')
    out.append("  },")
    out.append('  "cases": [')
    blocks = []
    for c in cases:
        lines = ["    {", f'      "name": {compact(c["name"])},']
        if "note" in c:
            lines.append(f'      "note": {compact(c["note"])},')
        lines.append('      "steps": [')
        lines.append(",\n".join(f"        {compact(st)}" for st in c["steps"]))
        lines.append("      ]")
        lines.append("    }")
        blocks.append("\n".join(lines))
    out.append(",\n".join(blocks))
    out.append("  ]")
    out.append("}")
    text = "\n".join(out) + "\n"
    path = f"fixtures/commands/v1/{command}.json"
    if "--check" in sys.argv[1:]:
        with open(path, encoding="utf-8") as f:
            if f.read() != text:
                sys.exit(f"{path} is not what this script writes: run it without --check and read the difference.")
        return
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)


write(
    "cad.entities.create",
    "Nesneleri ekle: doğrulama, plan, yazma, geri alma",
    "ADR 0057. Denetim sırası: en az bir nesne; her nesnenin geometrisi, sırayla, cad.entities.edit'in kurallarıyla (nokta ve köşe sayısı, sonlu sayılar, yarıçap); beklenen sürümün yazımı, sonra çizimin sürümü; katman (var, grup değil, kilitli değil; gizliyse uyarı). Nesne verilen geometrisi, girdinin katmanı ve verildiyse rengi, öznitelikleri (yoksa boş) ve etiketiyle yazılır. Adım “Ekle” ya da işlemin adıdır: Paralel çizgi, Dik in, Dik çık, Böl, Tarama. Kurulumdaki en büyük kimlik 2; yeni nesneler 3'ten başlar. $uidOf:N, N yuvasındaki nesnenin kalıcı kimliğidir.",
    cases,
)
print(f"{len(cases)} cases" + (" match" if "--check" in sys.argv[1:] else " written"))
