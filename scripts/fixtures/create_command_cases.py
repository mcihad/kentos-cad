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
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
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
    layer, and its colour, attributes (none: empty), label and symbol when given. An
    insert is mirrored or has no `mirror` (docs/adr/0144)."""
    out = json.loads(json.dumps(obj["geometry"]))
    if out["kind"] == "insert" and out.get("mirror") is not True:
        out.pop("mirror", None)
    # A text's defaults are no fields (docs/adr/0145): no mask, a width factor of 1.
    if out["kind"] == "text":
        if out.get("mask") is not True:
            out.pop("mask", None)
        if out.get("widthFactor") == 1:
            out.pop("widthFactor")
        # A multi-line text's (docs/adr/0182 §1): no box, a spacing of 1, no runs.
        if out.get("lineSpacing") == 1:
            out.pop("lineSpacing")
        if out.get("runs") == []:
            out.pop("runs")
    # So is a leader's mask (docs/adr/0146 §1); a filled arrow and no note are the fields' absence too.
    if out["kind"] == "leader" and out.get("mask") is not True:
        out.pop("mask", None)
    # And a dimension's (docs/adr/0147).
    if out["kind"] == "dimension" and out.get("mask") is not True:
        out.pop("mask", None)
    # A picture's (docs/adr/0192 §1): not mirrored is no field.
    if out["kind"] == "image" and out.get("mirror") is not True:
        out.pop("mirror", None)
    # A raster's look (docs/adr/0204 §2): no stretch, not inverted and bilinear are no fields.
    if out["kind"] == "raster":
        st = out["style"]
        if st.get("stretch") == "none":
            st.pop("stretch")
        if st.get("invert") is not True:
            st.pop("invert", None)
        if st.get("resampling") == "bilinear":
            st.pop("resampling")
    # A table's (docs/adr/0184 §1): no ranges, no heading row.
    if out["kind"] == "table":
        if out.get("merges") == []:
            out.pop("merges")
        if out.get("header") is not True:
            out.pop("header", None)
    out["id"] = slot
    out["layerId"] = layer_id
    if "color" in obj:
        out["color"] = obj["color"]
    out["attrs"] = obj.get("attrs", {})
    if "label" in obj:
        out["label"] = obj["label"]
    # An object template's symbol, written as given (docs/adr/0176 §3).
    if "symbol" in obj:
        out["symbol"] = obj["symbol"]
    # A linked text knows its object and its scale (docs/adr/0175 §4).
    if out["kind"] == "text" and "labelOf" in obj:
        out["labelOf"] = obj["labelOf"]
        out["labelScale"] = obj["labelScale"]
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


# A closed area's ring may have 2 corners when one of its two edges is an arc (cad.entities.edit's rule); a hatch's may not.
RING_TOO_FEW = "Kapalı alanın en az 3 köşesi olmalı (kenarlarından biri yaysa 2); {} köşe verildi. Eksik köşeleri ekleyin."
HOLE_TOO_FEW = "{}. deliğin en az 3 köşesi olmalı (kenarlarından biri yaysa 2); {} köşe verildi. Eksik köşeleri ekleyin ya da deliği çıkarın."


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

# An object template's recipe (docs/adr/0176 §3): the symbol, attributes and label as given; the symbol's id is
# the libraries' (the host's), not looked up.
cases.append({
    "name": "nesne şablonu: sembol, öznitelikler ve etiket verildiği gibi yazılır (ADR 0176); sembolün kimliği kitaplıkta aranmaz",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(ELLIPSE, symbol="temel.alan.kenar-ici", attrs={"Tür": "Havuz"}, label="H1"), O(SPLINE, symbol="u-yok")]},
         "result": done([3, 4]),
         "expect": {"ids": IDS + [3, 4], "entities": {"3": made(O(ELLIPSE, symbol="temel.alan.kenar-ici", attrs={"Tür": "Havuz"}, label="H1"), 3), "4": made(O(SPLINE, symbol="u-yok"), 4)}, "revision": "changed"}},
        {"op": "undo", "returns": "Ekle", "expect": {"ids": IDS}},
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

# İçine tıklayarak alan: the region clicked inside a circle, as the region core writes it (two corners, two
# half-circle arcs), and a region with a half-disc island.
DISC = {"kind": "polygon", "pts": [P(487045, 4420000), P(487035, 4420000)], "bulges": [1, 1]}
FRAMED = {"kind": "polygon", "pts": [P(487060, 4420060), P(487080, 4420060), P(487080, 4420080), P(487060, 4420080)],
          "holes": [{"pts": [P(487066, 4420070), P(487074, 4420070)], "bulges": [0, 1]}]}

cases.append({
    "name": "İçine tıklayarak alan: bölge alan olarak tek adımda yazılır, adı “Alan oluştur”; dairenin içi iki köşeli, yaylı halkadır; yarım daire ada da yazılır",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "operation": "boundary", "objects": [O(DISC)]}, "result": done([3]),
         "expect": {"ids": IDS + [3], "entities": {"3": made(O(DISC), 3)}, "revision": "changed"}},
        {"op": "execute", "input": {"layerId": "yapi", "operation": "boundary", "objects": [O(FRAMED, color="#E5484D")]}, "result": done([4]),
         "expect": {"ids": IDS + [3, 4], "entities": {"4": made(O(FRAMED, color="#E5484D"), 4)}, "revision": "changed"}},
        {"op": "undo", "returns": "Alan oluştur"},
        {"op": "undo", "returns": "Alan oluştur", "expect": {"ids": IDS, "canUndo": False}},
    ],
})

# Toplu alan (docs/adr/0151): two regions of a block, the first with its number and its corners' elevations
# carried from the lines, the second with a building as its hole and no label.
PARCEL = {"kind": "polygon", "pts": [P(487100, 4420000), P(487120, 4420000), P(487120, 4420015), P(487100, 4420015)],
          "zs": [101.2, 101.4, None, 101.0]}
COURT = {"kind": "polygon", "pts": [P(487120, 4420000), P(487140, 4420000), P(487140, 4420015), P(487120, 4420015)],
         "holes": [{"pts": [P(487125, 4420005), P(487125, 4420010), P(487130, 4420010), P(487130, 4420005)]}]}

cases.append({
    "name": "Toplu alan: bölgeler öznitelikleri ve kotlarıyla tek adımda yazılır, adı “Toplu alan”",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "operation": "polygonize", "objects": [O(PARCEL, attrs={"Ad": "101/1"}), O(COURT)]}, "result": done([3, 4]),
         "expect": {"ids": IDS + [3, 4], "entities": {"3": made(O(PARCEL, attrs={"Ad": "101/1"}), 3), "4": made(O(COURT), 4)}, "revision": "changed"}},
        {"op": "undo", "returns": "Toplu alan", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}},
        {"op": "redo", "returns": "Toplu alan", "expect": {"ids": IDS + [3, 4]}},
    ],
})

# Köşelere nokta (docs/adr/0152): named points at a parcel's corners, the code as an attribute, the corners'
# elevations; one without an elevation.
VERTEX_POINTS = [
    {"kind": "point", "p": P(487100, 4420040), "z": 101.25},
    {"kind": "point", "p": P(487120, 4420040)},
    {"kind": "point", "p": P(487120, 4420055), "z": 101.8},
]

cases.append({
    "name": "Köşelere nokta: adlı noktalar kodları ve kotlarıyla tek adımda yazılır, adı “Köşelere nokta”",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "operation": "vertexPoints", "objects": [O(g, label=f"10{i + 1}", attrs={"Kod": "SN"}) for i, g in enumerate(VERTEX_POINTS)]}, "result": done([3, 4, 5]),
         "expect": {"ids": IDS + [3, 4, 5], "entities": {str(3 + i): made(O(g, label=f"10{i + 1}", attrs={"Kod": "SN"}), 3 + i) for i, g in enumerate(VERTEX_POINTS)}, "revision": "changed"}},
        {"op": "undo", "returns": "Köşelere nokta", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}},
    ],
})

# Etiketleri yazıya çevir (docs/adr/0175 §2): labels as texts, as the core placed them at 1:1000: a parcel's number
# centred, a block's number from its box's corner with a mask, a street's name along it, turned to read upright.
LABEL_TEXTS = [
    {"kind": "text", "p": P(487110.5, 4420030.25), "text": "101", "height": 2.6458333333333335, "rotation": 0, "align": "middleCenter"},
    {"kind": "text", "p": P(487102.11666666667, 4420045.295833333), "text": "Ada 12", "height": 2.9104166666666667, "rotation": 0, "align": "middleLeft", "mask": True},
    {"kind": "text", "p": P(487130.0, 4420040.0), "text": "Cumhuriyet Cd.", "height": 2.6458333333333335, "rotation": -26.565051177077994, "align": "middleCenter"},
]

cases.append({
    "name": "Etiketleri yazıya çevir: etiketler hizaları, dönüşleri ve zeminleriyle yazı olarak tek adımda yazılır, adı “Etiketleri yazıya çevir”",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "operation": "labels", "objects": [O(g) for g in LABEL_TEXTS]}, "result": done([3, 4, 5]),
         "expect": {"ids": IDS + [3, 4, 5], "entities": {str(3 + i): made(O(g), 3 + i) for i, g in enumerate(LABEL_TEXTS)}, "revision": "changed"}},
        {"op": "undo", "returns": "Etiketleri yazıya çevir", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}},
        {"op": "redo", "returns": "Etiketleri yazıya çevir", "expect": {"ids": IDS + [3, 4, 5]}},
    ],
})

# Nesneye bağlı (docs/adr/0175 §4): the text knows the object whose label it writes (the line in slot 1, by its
# persistent id) and the scale it was written at; the command writes it as given and checks the link.
LINKED_TEXT = {"kind": "text", "p": P(487010, 4420001.5), "text": "Hat 1", "height": 2.6458333333333335, "rotation": 0, "align": "middleCenter"}
LINE = {"kind": "line", "a": P(487000, 4420010), "b": P(487010, 4420010)}
NOT_IN_DRAWING = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4ffe"
NIL = "00000000-0000-0000-0000-000000000000"
BOTH = "Bağlı yazının nesnesi ve ölçeği birlikte verilir. İkisini birden verin ya da hiçbirini vermeyin."
SCALE = "Bağlı yazının ölçeği (1:N'deki N) sonlu ve sıfırdan büyük olmalı. Ölçeği düzeltin."


def not_a_text(n, field):
    return failed("invalid_link", f"Yalnız yazı bir nesnenin etiketine bağlanır; {n}. nesne yazı değil. Bağı kaldırın.", f"objects[{n - 1}].{field}")


def link_not_found(uid_text, i):
    return failed("link_not_found", f"“{uid_text}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Çizimdeki bir nesnenin kimliğini verin.", f"objects[{i}].labelOf")


cases.append({
    "name": "Nesneye bağlı: yazı etiketini yazdığı nesnenin kalıcı kimliğini ve ölçeğini taşır; plan gösterir, yazma yazar, geri alma ve yineleme",
    "note": "ADR 0175 §4. Bağlı yazı 1 yuvasındaki çizginin etiketini 1:1000'de yazar.",
    "steps": [
        {"op": "plan", "input": {"layerId": "yapi", "operation": "labels", "objects": [O(LINKED_TEXT, labelOf=uid(1), labelScale=1000)]},
         "result": {"status": "completed", "output": {"entities": [made(O(LINKED_TEXT, labelOf=uid(1), labelScale=1000), 0)], "revision": "$current"}, "warnings": []}, "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "operation": "labels", "objects": [O(LINKED_TEXT, labelOf=uid(1), labelScale=1000), O(LABEL_TEXTS[0])]}, "result": done([3, 4]),
         "expect": {"ids": IDS + [3, 4], "entities": {"3": made(O(LINKED_TEXT, labelOf=uid(1), labelScale=1000), 3), "4": made(O(LABEL_TEXTS[0]), 4)}, "revision": "changed"}},
        {"op": "undo", "returns": "Etiketleri yazıya çevir", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}},
        {"op": "redo", "returns": "Etiketleri yazıya çevir", "expect": {"ids": IDS + [3, 4], "entities": {"3": made(O(LINKED_TEXT, labelOf=uid(1), labelScale=1000), 3)}}},
    ],
})

cases.append({
    "name": "bağ yalnız yazıda, iki alanıyla, kalıcı kimlik yazımıyla ve sonlu, sıfırdan büyük ölçekle olur: invalid_link, her nesnenin geometrisinden sonra, sürümden önce",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(LINE, labelOf=uid(1), labelScale=1000)], "expectedRevision": "999"}, "result": not_a_text(1, "labelOf"),
         "note": "Çizgi bir etiketi yazmaz; sürüm denetimi daha sonra.", "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(LINE, labelScale=1000)]}, "result": not_a_text(1, "labelScale"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(LINKED_TEXT), O(LINKED_TEXT, labelOf=uid(1))]}, "result": failed("invalid_link", BOTH, "objects[1].labelScale"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(LINKED_TEXT, labelScale=1000)]}, "result": failed("invalid_link", BOTH, "objects[0].labelOf"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(LINKED_TEXT, labelOf="Hat-1", labelScale=1000)]},
         "result": failed("invalid_link", "Bağlı nesnenin kimliği küçük harfli, tireli bir UUID olmalı; “Hat-1” verildi.", "objects[0].labelOf"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(LINKED_TEXT, labelOf=uid(1), labelScale=0)]}, "result": failed("invalid_link", SCALE, "objects[0].labelScale"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(LINKED_TEXT, labelOf=uid(1), labelScale=-500)]}, "result": failed("invalid_link", SCALE, "objects[0].labelScale"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(LINKED_TEXT, labelOf=uid(1), labelScale=1000)]}, "nonFinite": {"objects[0].labelScale": "Infinity"},
         "result": failed("invalid_link", SCALE, "objects[0].labelScale"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**LINKED_TEXT, "text": " "}, labelOf=uid(1), labelScale=0)]},
         "result": failed("empty_text", "Yazının metni boş olamaz; yalnız boşluktan oluşan metin de boştur. Yazıya bir metin verin.", "objects[0].geometry.text"),
         "note": "Geometri önce denetlenir.", "expect": NOTHING},
    ],
})

cases.append({
    "name": "bağlı yazının nesnesi çizimin olmalı: link_not_found, sırayla; katman denetiminden sonra",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(LINKED_TEXT, labelOf=uid(2), labelScale=1000), O(LINKED_TEXT, labelOf=NOT_IN_DRAWING, labelScale=1000)]},
         "result": link_not_found(NOT_IN_DRAWING, 1), "note": "2 yuvasındaki daire çizimde; ikinci yazının nesnesi yok.", "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(LINKED_TEXT, labelOf=NIL, labelScale=1000)]}, "result": link_not_found(NIL, 0), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "kilitli", "objects": [O(LINKED_TEXT, labelOf=NOT_IN_DRAWING, labelScale=1000)]}, "result": locked("Kilitli katman"), "expect": NOTHING},
    ],
})

# Bitişik alan (docs/adr/0162 §3): the region a path closes with its neighbours, as one area: here two parts, the
# first with a neighbour's island as its hole, the second with an arc edge.
ADJOINED = {"kind": "polygon",
            "pts": [P(487100, 4420060), P(487120, 4420060), P(487120, 4420075), P(487100, 4420075)],
            "holes": [{"pts": [P(487105, 4420065), P(487105, 4420070), P(487110, 4420070), P(487110, 4420065)]}],
            "parts": [{"pts": [P(487130, 4420060), P(487140, 4420060), P(487140, 4420075), P(487130, 4420075)],
                       "bulges": [0, 0.25, 0, 0]}]}

cases.append({
    "name": "Bitişik alan: komşularla kapanan bölge parçaları ve delikleriyle tek nesne olarak yazılır, adı “Bitişik alan”",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "operation": "adjoin", "objects": [O(ADJOINED, color="#E5484D")]}, "result": done([3]),
         "expect": {"ids": IDS + [3], "entities": {"3": made(O(ADJOINED, color="#E5484D"), 3)}, "revision": "changed"}},
        {"op": "undo", "returns": "Bitişik alan", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}},
        {"op": "redo", "returns": "Bitişik alan", "expect": {"ids": IDS + [3]}},
    ],
})

cases.append({
    "name": "kapalı alanın halkası 2 köşeli olabilir, iki kenarından biri yaysa; iki kenarı düzse reddedilir, delik de öyle; taramanın halkası en az 3 köşelidir",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**DISC, "bulges": [0, 0]})]}, "result": failed("too_few_corners", RING_TOO_FEW.format(2), "objects[0].geometry.pts"),
         "note": "Yay değerleri 0: iki kenar da düz.", "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**FRAMED, "holes": [{"pts": FRAMED["holes"][0]["pts"]}]})]},
         "result": failed("too_few_corners", HOLE_TOO_FEW.format(1, 2), "objects[0].geometry.holes[0].pts"), "note": "Yay değeri verilmemiş delik düzdür.", "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({"kind": "hatch", "ring": DISC["pts"], "pattern": {"type": "solid", "angle": 0, "spacing": 1}})]},
         "result": failed("too_few_corners", "Taramanın en az 3 köşesi olmalı; 2 köşe verildi. Eksik köşeleri ekleyin.", "objects[0].geometry.ring"), "note": "Taramanın halkasında yay yok.", "expect": NOTHING},
    ],
})

# The Hesap windows' “Çizime ekle”: named points, the name as their label and in their attributes (Ad, Tür, and
# Z (m) when the window found an elevation), one step named after the window.
def survey_point(x, y, name, kind, z=None):
    geometry = {"kind": "point", "p": P(x, y), **({"z": z} if z is not None else {})}
    attrs = {"Ad": name, "Tür": kind, **({"Z (m)": f"{z:.3f}"} if z is not None else {})}
    return O(geometry, attrs=attrs, label=name)


TRAVERSE = [survey_point(487012.304, 4420021.268, "P1", "Poligon noktası"), survey_point(487048.119, 4420040.442, "P2", "Poligon noktası")]
POLAR = [survey_point(487030.5, 4420060.25, "101", "Alım noktası", 12.345)]
FORWARD = [survey_point(487070.125, 4420015.5, "K1", "Kestirme noktası")]
RESECTION = [survey_point(487080.75, 4420035.125, "S1", "Kestirme noktası")]

cases.append({
    "name": "Hesap pencereleri: Poligon hesabı, Kutupsal alım, Önden ve Geriden kestirme noktalarını adlarıyla yazar; her biri tek adımdır, adı hesabın adıdır",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "operation": "traverse", "objects": TRAVERSE}, "result": done([3, 4]),
         "expect": {"ids": IDS + [3, 4], "entities": {"3": made(TRAVERSE[0], 3), "4": made(TRAVERSE[1], 4)}, "uids": {"3": "new", "4": "new"}, "revision": "changed"}},
        {"op": "execute", "input": {"layerId": "yapi", "operation": "polarSurvey", "objects": POLAR}, "result": done([5]),
         "note": "Kotu bulunan nokta: z geometride, “Z (m)” öznitelikte.", "expect": {"entities": {"5": made(POLAR[0], 5)}, "revision": "changed"}},
        {"op": "execute", "input": {"layerId": "yapi", "operation": "forwardIntersection", "objects": FORWARD}, "result": done([6]),
         "expect": {"entities": {"6": made(FORWARD[0], 6)}, "revision": "changed"}},
        {"op": "execute", "input": {"layerId": "yapi", "operation": "resection", "objects": RESECTION}, "result": done([7]),
         "expect": {"ids": IDS + [3, 4, 5, 6, 7], "entities": {"7": made(RESECTION[0], 7)}, "revision": "changed"}},
        {"op": "undo", "returns": "Geriden kestirme", "expect": {"ids": IDS + [3, 4, 5, 6]}},
        {"op": "undo", "returns": "Önden kestirme", "expect": {"ids": IDS + [3, 4, 5]}},
        {"op": "undo", "returns": "Kutupsal alım", "expect": {"ids": IDS + [3, 4]}},
        {"op": "undo", "returns": "Poligon hesabı", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}},
        {"op": "redo", "returns": "Poligon hesabı", "expect": {"ids": IDS + [3, 4]}},
    ],
})

# The drawing tools of ADR 0140 that add through this command name their steps.
BETWEEN = [O(point(487005, 4420000)), O(point(487010, 4420000))]
MEETING = [O(point(487004, 4420003))]
CHAINED = [O({"kind": "dimension", "a": P(487010, 4420000), "b": P(487016, 4420000), "offset": 2, "height": 0.5, "style": "linear", "angle": 0})]
BASED = [O({"kind": "dimension", "a": P(487000, 4420000), "b": P(487018, 4420000), "offset": 3.5, "height": 0.5, "style": "linear", "angle": 0})]
cases.append({
    "name": "Ara nokta, Kesişim noktası, Zincir ölçü ve Baz ölçü (ADR 0140): her biri tek adımdır, adı aracın adıdır",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "operation": "pointsBetween", "objects": BETWEEN}, "result": done([3, 4]),
         "expect": {"ids": IDS + [3, 4], "entities": {"3": made(BETWEEN[0], 3), "4": made(BETWEEN[1], 4)}, "revision": "changed"}},
        {"op": "execute", "input": {"layerId": "yapi", "operation": "intersectPoint", "objects": MEETING}, "result": done([5]),
         "expect": {"entities": {"5": made(MEETING[0], 5)}, "revision": "changed"}},
        {"op": "execute", "input": {"layerId": "yapi", "operation": "dimensionChain", "objects": CHAINED}, "result": done([6]),
         "expect": {"entities": {"6": made(CHAINED[0], 6)}, "revision": "changed"}},
        {"op": "execute", "input": {"layerId": "yapi", "operation": "dimensionBaseline", "objects": BASED}, "result": done([7]),
         "expect": {"ids": IDS + [3, 4, 5, 6, 7], "entities": {"7": made(BASED[0], 7)}, "revision": "changed"}},
        {"op": "undo", "returns": "Baz ölçü", "expect": {"ids": IDS + [3, 4, 5, 6]}},
        {"op": "undo", "returns": "Zincir ölçü", "expect": {"ids": IDS + [3, 4, 5]}},
        {"op": "undo", "returns": "Kesişim noktası", "expect": {"ids": IDS + [3, 4]}},
        {"op": "undo", "returns": "Ara nokta", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}},
    ],
})

cases.append({
    "name": "Hesap penceresi gizli katmana da yazar, uyarıyla",
    "steps": [
        {"op": "execute", "input": {"layerId": "gizli", "operation": "polarSurvey", "objects": POLAR}, "result": done([3], [hidden("Gizli katman")]),
         "expect": {"entities": {"3": made(POLAR[0], 3, "gizli")}, "revision": "changed"}},
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
         "result": failed("too_few_corners", RING_TOO_FEW.format(2), "objects[0].geometry.pts"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**CORRIDOR, "holes": [CORRIDOR["holes"][0], {"pts": [P(487110, 4420110), P(487112, 4420110)]}]})]},
         "result": failed("too_few_corners", HOLE_TOO_FEW.format(2, 2), "objects[0].geometry.holes[1].pts"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**DONUT, "ring": RING[:2]})]},
         "result": failed("too_few_corners", "Taramanın en az 3 köşesi olmalı; 2 köşe verildi. Eksik köşeleri ekleyin.", "objects[0].geometry.ring"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**DONUT, "holes": [HOLE[:2]]})]},
         "result": failed("too_few_corners", "1. deliğin en az 3 köşesi olmalı; 2 köşe verildi. Eksik köşeleri ekleyin ya da deliği çıkarın.", "objects[0].geometry.holes[0]"), "expect": NOTHING},
    ],
})

# docs/adr/0186 §1, §6: a pattern's name, scale and families, a gradient's second colour and a hatch's tie are checked
# as the contract checks them (`invalid_hatch`, at the field, before the ring); a pattern and a gradient are written.
PATTERNED = {**DONUT, "pattern": {"type": "pattern", "angle": 15, "spacing": 1, "name": "ANSI33", "scale": 0.5,
             "lines": [{"angle": 45, "origin": [0, 0], "offset": [0, 6.35]},
                       {"angle": 45, "origin": [4.49, 0], "offset": [0, 6.35], "dashes": [3.175, -1.5875]}]}}
GRADED = {**DONUT, "pattern": {"type": "gradient", "angle": 30, "spacing": 1,
                               "gradient": {"shape": "cylinder", "inverted": True, "color2": "#FFFFFF"}}}


def patterned(**pattern):
    return O({**PATTERNED, "pattern": {**PATTERNED["pattern"], **pattern}})


TWICE = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3301"
cases.append({
    "name": "taramanın deseni ve ilişkisi sözleşmenin kuralıyla denetlenir (invalid_hatch, alanında); desen ve degrade yazılır",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [patterned(lines=[])]},
         "result": failed("invalid_hatch", "Desenin 0 çizgi ailesi var; en az 1, en çok 64 olmalı.", "objects[0].geometry.pattern.lines"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [patterned(name="  ")]},
         "result": failed("invalid_hatch", "Desenin adı boş olamaz ve en çok 64 harf olabilir.", "objects[0].geometry.pattern.name"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [patterned(lines=[{"angle": 0, "origin": [0, 0], "offset": [3, 0]}])]},
         "result": failed("invalid_hatch", "Desenin 1. çizgi ailesinin çizgileri arası 0; aileler sıfırdan büyük aralıklı olmalı.", "objects[0].geometry.pattern.lines"),
         "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**DONUT, "pattern": {"type": "lines", "angle": 45, "spacing": 1, "name": "ANSI31"}})]},
         "result": failed("invalid_hatch", "Yalnız desen türündeki taramanın adı, ölçeği ve çizgi aileleri olur.", "objects[0].geometry.pattern"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**GRADED, "pattern": {**GRADED["pattern"], "gradient": {"shape": "linear", "color2": "beyaz"}}})]},
         "result": failed("invalid_hatch", "Degradenin ikinci rengi “beyaz”; #RRGGBB biçiminde olmalı.", "objects[0].geometry.pattern.gradient.color2"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**PATTERNED, "assoc": {"outer": TWICE, "islands": [TWICE], "seed": P(487001, 4420001)}})]},
         "result": failed("invalid_hatch", "Taramanın ilişkisi bir nesneyi iki kez gösteriyor; her nesne bir kez gösterilmeli.", "objects[0].geometry.assoc"),
         "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "operation": "hatch", "objects": [O(PATTERNED), O(GRADED)]}, "result": done([3, 4]),
         "expect": {"ids": IDS + [3, 4], "entities": {"3": made(O(PATTERNED), 3), "4": made(O(GRADED), 4)}, "revision": "changed"}},
        {"op": "undo", "returns": "Tarama", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}},
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

TEXT = {"kind": "text", "p": P(487060, 4420100), "text": "Ada 101", "height": 2, "rotation": 30}
EMPTY_TEXT = "Yazının metni boş olamaz; yalnız boşluktan oluşan metin de boştur. Yazıya bir metin verin."
cases.append({
    "name": "yazının metni boş olamaz: empty_text; yalnız boşluk da boştur (Unicode White_Space: sekme, satır sonu, U+0085, bölünmez boşluk…); yol nesnenin sırasını verir; sayılardan önce denetlenir",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TEXT, "text": ""})]},
         "result": failed("empty_text", EMPTY_TEXT, "objects[0].geometry.text"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(TEXT), O({**TEXT, "text": " \t\r\n\u0085\u00a0\u2007\u3000"})]},
         "result": failed("empty_text", EMPTY_TEXT, "objects[1].geometry.text"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TEXT, "text": ""})]}, "nonFinite": {"objects[0].geometry.p.x": "NaN"},
         "result": failed("empty_text", EMPTY_TEXT, "objects[0].geometry.text"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "kilitli", "objects": [O({**TEXT, "text": " "})], "expectedRevision": "999"},
         "result": failed("empty_text", EMPTY_TEXT, "objects[0].geometry.text"), "note": "Girdinin hatası: sürümden ve katmandan önce.", "expect": NOTHING},
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

# ── Blocks (docs/adr/0144): Blok ekle writes an insert of a block the drawing defines ──
ROGAR_ID = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"
MISSING_BLOCK = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4fff"
B_SETUP = {**SETUP, "blocks": [{"id": ROGAR_ID, "name": "Rögar", "base": P(0, 0), "entities": [
    {"kind": "circle", "id": 1, "layerId": "0", "attrs": {}, "c": P(0, 0), "r": 1}]}]}
INSERT = {"kind": "insert", "block": ROGAR_ID, "p": P(487080, 4420000), "scale": 2, "rotation": 0.5}

cases.append({
    "name": "Blok ekle: yerleştirme öznitelikleri ve etiketiyle; aynalı olan mirror taşır, false yazılmaz; tek adım “Ekle”",
    "setup": B_SETUP,
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(INSERT, attrs={"NO": "R-9"}, label="R-9"), O({**INSERT, "mirror": True}), O({**INSERT, "mirror": False})]},
         "result": done([3, 4, 5]),
         "expect": {"ids": IDS + [3, 4, 5], "entities": {"3": made(O(INSERT, attrs={"NO": "R-9"}, label="R-9"), 3), "4": made(O({**INSERT, "mirror": True}), 4), "5": made(O(INSERT), 5)},
                    "uids": {"3": "new", "4": "new", "5": "new"}, "canUndo": True, "revision": "changed"}},
        {"op": "undo", "returns": "Ekle", "expect": {"ids": IDS, "canUndo": False}},
    ],
})

cases.append({
    "name": "yerleştirmenin bloğu çizimin olmalı: unknown_block, sırayla; katman denetiminden sonra",
    "setup": B_SETUP,
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(INSERT), O({**INSERT, "block": MISSING_BLOCK})]},
         "result": failed("unknown_block", f"“{MISSING_BLOCK}” kimlikli blok çizimde tanımlı değil: silinmiş ya da başka bir çizimin olabilir. Çizimde tanımlı bir bloğun kimliğini verin.", "objects[1].geometry.block"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "kilitli", "objects": [O({**INSERT, "block": MISSING_BLOCK})]}, "result": locked("Kilitli katman"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "yerleştirmenin ölçeği sıfırdan büyük olmalı: invalid_scale; sonlu değilse not_finite; ikisi de sürümden önce",
    "setup": B_SETUP,
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**INSERT, "scale": 0})], "expectedRevision": "999"},
         "result": failed("invalid_scale", "Blok ölçeği sıfırdan büyük olmalı. Pozitif bir ölçek verin.", "objects[0].geometry.scale"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(INSERT), O({**INSERT, "scale": -1})]},
         "result": failed("invalid_scale", "Blok ölçeği sıfırdan büyük olmalı. Pozitif bir ölçek verin.", "objects[1].geometry.scale"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(INSERT)]}, "nonFinite": {"objects[0].geometry.rotation": "Infinity"}, "result": not_finite(1), "expect": NOTHING},
    ],
})


# ── Yazı ekleri (docs/adr/0145) ────────────────────────────────────────


def width_factor_message(w):
    return f"Yazının genişlik çarpanı 0'dan büyük, en çok 100 olmalı; {w} verildi. Çarpanı bu aralıkta verin ya da alanı kaldırın (1)."


dressed_texts = [
    O({**TEXT, "align": "middleCenter", "widthFactor": 0.8, "mask": True}),
    O({**TEXT, "text": "Ada 102", "align": "topRight"}),
    O({**TEXT, "text": "Ada 103", "widthFactor": 1, "mask": False}),
]
cases.append({
    "name": "yazının hizası, genişlik çarpanı ve zemini yazılır; 1 çarpan ve zeminsizlik alan değildir, yazılmaz (ADR 0145)",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": dressed_texts}, "result": done([3, 4, 5]),
         "expect": {"entities": {str(3 + i): made(o, 3 + i) for i, o in enumerate(dressed_texts)}, "revision": "changed"}},
    ],
})
# Metin dosyası yerleştir (docs/adr/0145 §6): a file's lines, 1.5 heights apart down the texts' up, one step.
file_texts = [
    O({**TEXT, "text": "Ada 101", "rotation": 0, "align": "middleCenter"}),
    O({**TEXT, "text": "Ada 102", "rotation": 0, "align": "middleCenter", "p": P(487060, 4420097)}),
]
cases.append({
    "name": "Metin dosyası yerleştir: satırların yazıları tek adımda, adı “Metin dosyası yerleştir” (ADR 0145 §6)",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "operation": "textFile", "objects": file_texts}, "result": done([3, 4]),
         "expect": {"ids": IDS + [3, 4], "entities": {str(3 + i): made(o, 3 + i) for i, o in enumerate(file_texts)}, "revision": "changed"}},
        {"op": "undo", "returns": "Metin dosyası yerleştir", "expect": {"ids": IDS, "canUndo": False}},
    ],
})

cases.append({
    "name": "yazının genişlik çarpanı 0'dan büyük, en çok 100: invalid_width_factor, yolu nesnenin; sonlu olmayan önce not_finite (ADR 0145)",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(TEXT), O({**TEXT, "widthFactor": 0})]},
         "result": failed("invalid_width_factor", width_factor_message(0), "objects[1].geometry.widthFactor"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TEXT, "widthFactor": 101})]},
         "result": failed("invalid_width_factor", width_factor_message(101), "objects[0].geometry.widthFactor"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TEXT, "widthFactor": 0.8})]}, "nonFinite": {"objects[0].geometry.widthFactor": "Infinity"},
         "result": not_finite(1), "expect": NOTHING},
    ],
})


# ── Çok satırlı yazı (docs/adr/0182 §6) ────────────────────────────────


def box_message(w):
    return f"Çok satırlı yazının kutu genişliği sıfırdan büyük olmalı; {w} verildi. Genişliği metre olarak, pozitif verin ya da alanı kaldırın (satırlar yalnız satır sonlarında biter)."


def spacing_message(s):
    return f"Çok satırlı yazının satır aralığı 0.25 ile 4 arasında olmalı; {s} verildi. Aralığı bu sınırlarda verin ya da alanı kaldırın (1)."


def run_messages(n, start=None, end=None, letters=None):
    return {
        "range": f"{n}. biçim dilimi {start}–{end}: başı sonundan önce olmalı, sonu yazının harf sayısını ({letters}) aşmamalı. Dilimi yazının harfleri içinde verin.",
        "format": f"{n}. biçim diliminin biçimi yok; biçimsiz dilim yazılmaz. Dilime bir biçim verin ya da dilimi çıkarın.",
        "color": f"{n}. biçim diliminin rengi boş. Bir renk verin ya da rengi kaldırın.",
        "order": f"{n}. biçim dilimi öncekiyle örtüşüyor ya da ondan önce başlıyor; dilimler sıralı ve ayrı olmalı. Dilimleri sırayla, örtüşmeden verin.",
        "join": f"{n}. biçim dilimi aynı biçimdeki öncekine bitişik; ikisi tek dilimdir. İki dilimi birleştirin.",
    }


PARAGRAPH = {**TEXT, "text": "Parsel 101\nAlan 450 m2", "align": "topLeft"}
paragraphs = [
    O({**PARAGRAPH, "boxWidth": 20, "lineSpacing": 1.5, "runs": [{"start": 0, "end": 6, "bold": True}, {"start": 21, "end": 22, "script": "super"}]}),
    O({**PARAGRAPH, "text": "Ada 7\nçok satır", "lineSpacing": 1, "runs": []}),
    O({**PARAGRAPH, "text": "Renkli", "runs": [{"start": 0, "end": 3, "italic": True, "underline": True, "color": "#E5484D"}, {"start": 3, "end": 6, "color": "ink"}]}),
]
cases.append({
    "name": "çok satırlı yazı kutusu, satır aralığı ve biçim dilimleriyle yazılır; 1 aralık ve boş dilim listesi alan değildir (ADR 0182 §1, §6)",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": paragraphs}, "result": done([3, 4, 5]),
         "expect": {"entities": {str(3 + i): made(o, 3 + i) for i, o in enumerate(paragraphs)}, "revision": "changed"}},
    ],
})
TWO = {**TEXT, "text": "ab\ncd"}
cases.append({
    "name": "çok satırlı yazının kutusu, aralığı ve dilimleri sınırlarında: invalid_paragraph, yolu alanın; sonlu olmayan önce not_finite (ADR 0182 §6)",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(TEXT), O({**TWO, "boxWidth": 0})]},
         "result": failed("invalid_paragraph", box_message(0), "objects[1].geometry.boxWidth"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TWO, "lineSpacing": 5})]},
         "result": failed("invalid_paragraph", spacing_message(5), "objects[0].geometry.lineSpacing"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TWO, "lineSpacing": 0.2})]},
         "result": failed("invalid_paragraph", spacing_message(0.2), "objects[0].geometry.lineSpacing"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TWO, "runs": [{"start": 2, "end": 6, "bold": True}]})]},
         "result": failed("invalid_paragraph", run_messages(1, 2, 6, 5)["range"], "objects[0].geometry.runs"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TWO, "runs": [{"start": 3, "end": 3, "bold": True}]})]},
         "result": failed("invalid_paragraph", run_messages(1, 3, 3, 5)["range"], "objects[0].geometry.runs"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TWO, "runs": [{"start": 0, "end": 2}]})]},
         "result": failed("invalid_paragraph", run_messages(1)["format"], "objects[0].geometry.runs"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TWO, "runs": [{"start": 0, "end": 2, "color": ""}]})]},
         "result": failed("invalid_paragraph", run_messages(1)["color"], "objects[0].geometry.runs"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TWO, "runs": [{"start": 0, "end": 3, "bold": True}, {"start": 2, "end": 4, "italic": True}]})]},
         "result": failed("invalid_paragraph", run_messages(2)["order"], "objects[0].geometry.runs"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TWO, "runs": [{"start": 0, "end": 2, "bold": True}, {"start": 2, "end": 4, "bold": True}]})]},
         "result": failed("invalid_paragraph", run_messages(2)["join"], "objects[0].geometry.runs"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TWO, "boxWidth": 20})]}, "nonFinite": {"objects[0].geometry.boxWidth": "Infinity"},
         "result": not_finite(1), "expect": NOTHING},
    ],
})


# ── Yazı ve ölçü stilleri (docs/adr/0183 §9) ────────────────────────────

ADA_STYLE = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2101"
MIMARI_STYLE = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2201"
MISSING_STYLE = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2fff"
S_SETUP = {**SETUP, "settings": {**SETUP["settings"],
    "textStyles": [{"id": ADA_STYLE, "name": "Ada no", "font": "arimo", "bold": True, "height": 3.5}],
    "dimensionStyles": [{"id": MIMARI_STYLE, "name": "Mimari", "height": 3.5, "arrow": "closed", "decimals": 1, "unit": "cm", "suffix": " cm"}]}}
DIM = {"kind": "dimension", "a": P(487000, 4420140), "b": P(487024.5, 4420140), "offset": -2, "height": 3.5}


def face_message():
    return "Kalın, eğik ve yatık yazı bir yazı tipiyle olur; yazının yazı tipi yok. Yazıya bir yazı tipi verin ya da bu alanları kaldırın (yazı projenin yazı tipiyle çizilir)."


def oblique_message(o):
    return f"Yazının eğikliği −85 ile 85 derece arasında ve sıfırdan farklı olmalı; {o} verildi. Eğikliği bu aralıkta verin ya da alanı kaldırın (dik)."


def size_message(what, floor, v):
    return f"Ölçünün {what} değer yüksekliğinin katıdır: {floor}, en çok 100 olmalı; {v} verildi. Bu aralıkta verin ya da alanı kaldırın (Standart'ınki)."


def affix_message(what, why):
    return f"Ölçünün {what} yazılamaz: {why}. Tek satır, en çok 32 harf verin ya da alanı kaldırın."


def unknown_style_message(style_id, what):
    return f"“{style_id}” kimlikli {what} projede yok: silinmiş ya da başka bir projenin olabilir. Projenin bir {what}nin kimliğini verin ya da alanı kaldırın (Standart)."


styled = [
    O({**TEXT, "textStyle": ADA_STYLE, "font": "arimo", "bold": True, "height": 3.5}),
    O({**TEXT, "text": "Çınar sokağı", "font": "overpass", "italic": True, "oblique": 15}),
    O({**DIM, "dimStyle": MIMARI_STYLE, "arrow": "closed", "decimals": 1, "unit": "cm", "suffix": " cm"}),
    O({**DIM, "a": P(487000, 4420150), "b": P(487010, 4420150), "height": 2.5, "arrow": "dot", "arrowSize": 0.8, "extOffset": 0,
       "textGap": 0.5, "textPlace": "centre", "prefix": "~", "font": "plex-mono"}),
]
cases.append({
    "name": "yazı stili ve yüzüyle, ölçü stili ve görünüşüyle yazılır; stilsiz yüz ve görünüş de olur (ADR 0183 §9)",
    "setup": S_SETUP,
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": styled}, "result": done([3, 4, 5, 6]),
         "expect": {"entities": {str(3 + i): made(o, 3 + i) for i, o in enumerate(styled)}, "revision": "changed"}},
        {"op": "undo", "returns": "Ekle", "expect": {"ids": IDS, "canUndo": False}},
    ],
})
cases.append({
    "name": "yazının yüzü ve ölçünün görünüşü kurallarında: invalid_style, yolu alanın; sonlu olmayan önce not_finite (ADR 0183 §9)",
    "setup": S_SETUP,
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(TEXT), O({**TEXT, "bold": True})]},
         "result": failed("invalid_style", face_message(), "objects[1].geometry.bold"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TEXT, "oblique": 10})]},
         "result": failed("invalid_style", face_message(), "objects[0].geometry.oblique"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TEXT, "font": "arimo", "oblique": 90})]},
         "result": failed("invalid_style", oblique_message(90), "objects[0].geometry.oblique"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TEXT, "textStyle": ""})]},
         "result": failed("invalid_style", "Yazının stil kimliği boş. Stilin kimliğini verin ya da alanı kaldırın (Standart).", "objects[0].geometry.textStyle"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**DIM, "arrowSize": 0})]},
         "result": failed("invalid_style", size_message("ok boyu", "sıfırdan büyük", 0), "objects[0].geometry.arrowSize"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**DIM, "extOffset": -0.5})]},
         "result": failed("invalid_style", size_message("uzatma çizgisinin boşluğu", "0 ya da büyük", -0.5), "objects[0].geometry.extOffset"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**DIM, "decimals": 9})]},
         "result": failed("invalid_style", "Ölçünün basamak sayısı en çok 8; 9 verildi. Daha az basamak verin ya da alanı kaldırın (projenin basamakları).", "objects[0].geometry.decimals"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**DIM, "prefix": ""})]},
         "result": failed("invalid_style", affix_message("öneki", "boş"), "objects[0].geometry.prefix"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**DIM, "suffix": "m\n2"})]},
         "result": failed("invalid_style", affix_message("soneki", "satır sonu ya da denetim karakteri var"), "objects[0].geometry.suffix"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TEXT, "font": "arimo", "oblique": 10})]}, "nonFinite": {"objects[0].geometry.oblique": "Infinity"},
         "result": not_finite(1), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**DIM, "textGap": 0.5})]}, "nonFinite": {"objects[0].geometry.textGap": "NaN"},
         "result": not_finite(1), "expect": NOTHING},
    ],
})
cases.append({
    "name": "yazının ve ölçünün stili projenin olmalı: unknown_style, sırayla; katman denetiminden sonra (ADR 0183 §9)",
    "setup": S_SETUP,
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TEXT, "textStyle": ADA_STYLE}), O({**TEXT, "textStyle": MISSING_STYLE})]},
         "result": failed("unknown_style", unknown_style_message(MISSING_STYLE, "yazı stili"), "objects[1].geometry.textStyle"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**DIM, "dimStyle": ADA_STYLE})]},
         "result": failed("unknown_style", unknown_style_message(ADA_STYLE, "ölçü stili"), "objects[0].geometry.dimStyle"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "kilitli", "objects": [O({**TEXT, "textStyle": MISSING_STYLE})]}, "result": locked("Kilitli katman"), "expect": NOTHING},
    ],
})


# ── Tablo (docs/adr/0184 §6) ────────────────────────────────────────────


def rs(x):
    """A number as Rust's `{}` writes an f64: a whole one without its point."""
    return str(int(x)) if float(x).is_integer() else repr(float(x))


TABLE = {"kind": "table", "p": P(487000, 4420200), "rotation": 0, "height": 1.25, "rows": [2.5, 2.5, 2.5], "columns": [8, 10],
         "cells": [["Ad", "Alan (m²)"], ["7", "600.00"], ["8", "400.00"]]}
tables = [
    O({**TABLE, "aligns": ["left", "right"], "header": True, "grid": "rows", "frame": 0.25, "font": "arimo", "bold": True,
       "source": {"kind": "areas", "objects": [uid(1), uid(2)]}}),
    O({**TABLE, "p": P(487030, 4420200), "rotation": 30, "cells": [["Başlık", ""], ["a", "b"], ["c", ""]], "merges": [{"row": 0, "col": 0, "rows": 1, "cols": 2}],
       "header": False, "source": {"kind": "file", "name": "noktalar.xlsx", "sheet": "Sayfa1"}}, attrs={"Not": "Excel"}, label="Noktalar"),
]


def table_message(kind, *given):
    return {
        "height": "Tablonun yazı yüksekliği {}; sıfırdan büyük ve sonlu olmalı.",
        "rows": "Tablonun {} satırı var; en az 1, en çok 10000 olmalı.",
        "rowHeight": "Tablonun {}. satırının yüksekliği {}; sıfırdan büyük ve sonlu olmalı.",
        "cells": "Tablonun {} satırı var ama {} satırlık hücre verildi; her satırın hücreleri verilmeli.",
        "row": "Tablonun {}. satırında {} hücre var; sütun sayısı kadar ({}) olmalı.",
        "cell": "{}. satırın {}. hücresi: satır sonu ya da denetim karakteri var; hücre tek satırdır.",
        "merge": "{}. birleşik alan ({}. satır, {}. sütundan {} × {}) tablonun içinde ve birden çok hücre olmalı.",
        "overlap": "{}. birleşik alan, {}. satır {}. sütundaki birleşik alanla örtüşüyor; birleşik alanlar ayrı olmalı.",
        "hidden": "{}. satırın {}. hücresi birleşik bir alanın içinde ama boş değil; birleşik alanın yazısı sol üst hücresindedir.",
        "frame": "Tablonun çerçeve kalınlığı {}; sıfırdan büyük, tablonun eninin ve boyunun yarısından küçük olmalı.",
        "aligns": "Tablonun {} sütunu var ama {} hiza verildi; her sütunun hizası verilmeli.",
        "file": "Tablonun kaynağı olan dosyanın adı boş olamaz.",
        "objects": "Tablonun kaynağı {} nesne gösteriyor; en az 1, en çok 100000 olmalı.",
    }[kind].format(*given)


cases.append({
    "name": "Tablo: hücreleri, birleşik alanı, hizaları, başlığı, çizgileri, çerçevesi, yüzü ve kaynağıyla tek adımda yazılır, adı “Tablo”; birleşik alansızlık ve başlıksızlık alan değildir (ADR 0184 §6)",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "operation": "table", "objects": tables}, "result": done([3, 4]),
         "expect": {"ids": IDS + [3, 4], "entities": {str(3 + i): made(o, 3 + i) for i, o in enumerate(tables)}, "uids": {"3": "new", "4": "new"}, "revision": "changed"}},
        {"op": "undo", "returns": "Tablo", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}},
        {"op": "redo", "returns": "Tablo", "expect": {"ids": IDS + [3, 4]}},
    ],
})
cases.append({
    "name": "tablonun kuralları sırayla: invalid_table, yolu alanın; sonlu olmayan önce not_finite; yüz yazınınki gibi invalid_style (ADR 0184 §6)",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(TABLE), O({**TABLE, "height": 0})]},
         "result": failed("invalid_table", table_message("height", 0), "objects[1].geometry.height"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TABLE, "rows": [], "cells": []})]},
         "result": failed("invalid_table", table_message("rows", 0), "objects[0].geometry.rows"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TABLE, "rows": [2.5, -1, 2.5]})]},
         "result": failed("invalid_table", table_message("rowHeight", 2, -1), "objects[0].geometry.rows"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TABLE, "cells": TABLE["cells"][:2]})]},
         "result": failed("invalid_table", table_message("cells", 3, 2), "objects[0].geometry.cells"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TABLE, "cells": [["Ad", "Alan (m²)"], ["7"], ["8", "400.00"]]})]},
         "result": failed("invalid_table", table_message("row", 2, 1, 2), "objects[0].geometry.cells"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TABLE, "cells": [["Ad", "Alan (m²)"], ["7", "600\n00"], ["8", "400.00"]]})]},
         "result": failed("invalid_table", table_message("cell", 2, 2), "objects[0].geometry.cells"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TABLE, "merges": [{"row": 2, "col": 0, "rows": 2, "cols": 1}]})]},
         "result": failed("invalid_table", table_message("merge", 1, 3, 1, 2, 1), "objects[0].geometry.merges"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TABLE, "cells": [["Ad", ""], ["", ""], ["8", "400.00"]],
                                                                     "merges": [{"row": 0, "col": 0, "rows": 2, "cols": 2}, {"row": 1, "col": 1, "rows": 2, "cols": 1}]})]},
         "result": failed("invalid_table", table_message("overlap", 2, 1, 1), "objects[0].geometry.merges"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TABLE, "merges": [{"row": 1, "col": 0, "rows": 1, "cols": 2}]})]},
         "result": failed("invalid_table", table_message("hidden", 2, 2), "objects[0].geometry.merges"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TABLE, "frame": 3.75})]},
         "result": failed("invalid_table", table_message("frame", 3.75), "objects[0].geometry.frame"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TABLE, "aligns": ["left"]})]},
         "result": failed("invalid_table", table_message("aligns", 2, 1), "objects[0].geometry.aligns"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TABLE, "source": {"kind": "file", "name": " "}})]},
         "result": failed("invalid_table", table_message("file"), "objects[0].geometry.source"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TABLE, "source": {"kind": "coordinates", "objects": []}})]},
         "result": failed("invalid_table", table_message("objects", 0), "objects[0].geometry.source"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TABLE, "bold": True})]},
         "result": failed("invalid_style", face_message(), "objects[0].geometry.bold"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**TABLE, "height": 0})]}, "nonFinite": {"objects[0].geometry.columns[1]": "Infinity"},
         "result": not_finite(1), "note": "Sonlu olmayan sayı tablonun kurallarından önce.", "expect": NOTHING},
    ],
})


# ── Koordinat yaz (docs/adr/0185 §1) ────────────────────────────────────

coordinate_labels = [
    O({"kind": "polyline", "pts": [P(487000, 4420000), P(487003, 4420003), P(487010.5, 4420003)]}),
    O({"kind": "text", "p": P(487003.5, 4420003.4), "text": "487000.000", "height": 1, "rotation": 0}),
    O({"kind": "text", "p": P(487003.5, 4420002.6), "text": "4420000.000", "height": 1, "rotation": 0, "align": "topLeft"}),
    O({"kind": "text", "p": P(486996.5, 4420003.4), "text": "101", "height": 1, "rotation": 0, "align": "baselineRight", "textStyle": ADA_STYLE}),
]
cases.append({
    "name": "Koordinat yaz: kolu ve satırları tek adımda yazılır, adı “Koordinat yaz” (ADR 0185 §1)",
    "setup": S_SETUP,
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "operation": "coordinates", "objects": coordinate_labels}, "result": done([3, 4, 5, 6]),
         "expect": {"ids": IDS + [3, 4, 5, 6], "entities": {str(3 + i): made(o, 3 + i) for i, o in enumerate(coordinate_labels)},
                    "uids": {"3": "new", "4": "new", "5": "new", "6": "new"}, "revision": "changed"}},
        {"op": "undo", "returns": "Koordinat yaz", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}},
        {"op": "redo", "returns": "Koordinat yaz", "expect": {"ids": IDS + [3, 4, 5, 6]}},
    ],
})


# ── Km yaz (docs/adr/0189 §5) ───────────────────────────────────────────

stations = [
    O({"kind": "line", "a": P(487020, 4420001), "b": P(487020, 4419999)}),
    O({"kind": "text", "p": P(487020, 4420002), "text": "0+020", "height": 2, "rotation": 90, "align": "middleLeft"}),
    O({"kind": "line", "a": P(487020, 4420010), "b": P(487020, 4419990)}, attrs={"Km": "0+020"}),
    O({"kind": "point", "p": P(487020, 4419997)}, attrs={"Km": "0+020"}),
]
cases.append({
    "name": "Km yaz: bir istasyonun işareti, yazısı, enkesiti ve noktası tek adımda yazılır, adı “Km yaz” (ADR 0189 §5)",
    "setup": S_SETUP,
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "operation": "stations", "objects": stations}, "result": done([3, 4, 5, 6]),
         "expect": {"ids": IDS + [3, 4, 5, 6], "entities": {str(3 + i): made(o, 3 + i) for i, o in enumerate(stations)},
                    "uids": {"3": "new", "4": "new", "5": "new", "6": "new"}, "revision": "changed"}},
        {"op": "undo", "returns": "Km yaz", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}},
        {"op": "redo", "returns": "Km yaz", "expect": {"ids": IDS + [3, 4, 5, 6]}},
    ],
})


# ── Orta hat (docs/adr/0190 §3) ─────────────────────────────────────────

axis = [O({"kind": "polyline", "pts": [P(487000, 4420003), P(487020, 4420003), P(487030, 4420013)], "bulges": [0, 0.41421356237309503]})]
cases.append({
    "name": "Orta hat: eksen yaylı kenarıyla bir çoklu çizgi olarak tek adımda yazılır, adı “Orta hat” (ADR 0190 §3)",
    "setup": S_SETUP,
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "operation": "centerline", "objects": axis}, "result": done([3]),
         "expect": {"ids": IDS + [3], "entities": {"3": made(axis[0], 3)}, "uids": {"3": "new"}, "revision": "changed"}},
        {"op": "undo", "returns": "Orta hat", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}},
        {"op": "redo", "returns": "Orta hat", "expect": {"ids": IDS + [3]}},
    ],
})


# ── Resim ekle (docs/adr/0192 §6) ───────────────────────────────────────

PICTURE_ID = "resim-0011223344556677"
PICTURE = {"kind": "asset", "id": PICTURE_ID, "name": "logo", "path": ["Resimler"], "format": "png", "data": "data:image/png;base64,iVBORw0KGgo=", "width": 4, "height": 3}
I_SETUP = {**SETUP, "styles": {"items": [PICTURE, {**PICTURE, "id": "cizim-1", "format": "svg", "data": "<svg/>"}], "categories": []}}
IMAGE = {"kind": "image", "p": P(487000, 4420010), "width": 8, "height": 6, "rotation": 0.5235987755982988, "asset": PICTURE_ID}
images = [
    O(IMAGE),
    O({"kind": "image", "p": P(487020, 4420010), "width": 12, "height": 9, "rotation": 0, "mirror": True, "file": "foto/saha.jpg",
       "clip": [P(0.1, 0.1), P(0.9, 0.1), P(0.9, 0.8), P(0.1, 0.8)], "opacity": 0.6}, attrs={"Not": "saha"}),
    O({**IMAGE, "mirror": False}),
]


def image_message(kind, given=None):
    return {
        "size": "Resmin genişliği ve yüksekliği sıfırdan büyük ve en çok 10000000 m olmalı.",
        "both": "Resmin kaynağı ya gömülü varlık (asset) ya bağlı dosya (file) olmalı, ikisi birden değil.",
        "none": "Resmin kaynağı yok: gömülü varlığın kimliğini (asset) ya da bağlı dosyanın yolunu (file) verin.",
        "path": "Bağlı dosyanın yolu en çok 4096 harf olmalı ve denetim karakteri içermemeli.",
        "clip": "Resmin kırpma sınırı en az 3, en çok 10000 köşe olmalı, köşeleri resmin kesirleriyle 0 ile 1 arasında.",
        "opacity": f"Resmin donukluğu 0.1 ile 1 arasında olmalı; {given} verildi.",
    }[kind]


def unknown_asset(asset):
    return f"“{asset}” kimlikli görüntü projenin kitaplığında yok: silinmiş ya da başka bir çizimin olabilir. Projenin kitaplığındaki bir PNG ya da JPEG görüntünün kimliğini verin."


cases.append({
    "name": "Resim ekle: gömülü ve bağlı resim, kırpması, donukluğu ve aynasıyla tek adımda yazılır, adı “Resim ekle”; aynasızlık alan değildir (ADR 0192 §6)",
    "setup": I_SETUP,
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "operation": "image", "objects": images}, "result": done([3, 4, 5]),
         "expect": {"ids": IDS + [3, 4, 5], "entities": {str(3 + i): made(o, 3 + i) for i, o in enumerate(images)}, "uids": {"3": "new", "4": "new", "5": "new"},
                    "revision": "changed"}},
        {"op": "undo", "returns": "Resim ekle", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}},
        {"op": "redo", "returns": "Resim ekle", "expect": {"ids": IDS + [3, 4, 5]}},
    ],
})
cases.append({
    "name": "resmin kuralları sırayla: invalid_image, yolu nesnenin geometrisi; sonlu olmayan önce not_finite; gömülü resmin görüntüsü projenin kitaplığında PNG ya da JPEG olmalı: unknown_asset (ADR 0192 §6)",
    "setup": I_SETUP,
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(IMAGE), O({**IMAGE, "width": 0})]},
         "result": failed("invalid_image", image_message("size"), "objects[1].geometry"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**IMAGE, "file": "logo.png"})]},
         "result": failed("invalid_image", image_message("both"), "objects[0].geometry"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**{k: v for k, v in IMAGE.items() if k != "asset"}})]},
         "result": failed("invalid_image", image_message("none"), "objects[0].geometry"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**IMAGE, "asset": "  "})]},
         "result": failed("invalid_image", image_message("none"), "objects[0].geometry"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**{k: v for k, v in IMAGE.items() if k != "asset"}, "file": "foto/\tsaha.jpg"})]},
         "result": failed("invalid_image", image_message("path"), "objects[0].geometry"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**IMAGE, "clip": [P(0, 0), P(1.5, 0), P(1, 1)]})]},
         "result": failed("invalid_image", image_message("clip"), "objects[0].geometry"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**IMAGE, "clip": [P(0, 0), P(1, 1)]})]},
         "result": failed("invalid_image", image_message("clip"), "objects[0].geometry"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**IMAGE, "opacity": 0.05})]},
         "result": failed("invalid_image", image_message("opacity", 0.05), "objects[0].geometry"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(IMAGE), O({**IMAGE, "asset": "resim-ffffffffffffffff"})]},
         "result": failed("unknown_asset", unknown_asset("resim-ffffffffffffffff"), "objects[1].geometry.asset"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**IMAGE, "asset": "cizim-1"})]},
         "result": failed("unknown_asset", unknown_asset("cizim-1"), "objects[0].geometry.asset"), "expect": NOTHING},
    ],
})


# ── Raster ekle (docs/adr/0204 §9) ──────────────────────────────────────

RASTER_ID = "raster-0011223344556677"
RASTER_ITEM = {"kind": "asset", "id": RASTER_ID, "name": "dem", "path": ["Rasterler"], "format": "tiff", "data": "data:image/tiff;base64,SUkqAA==", "width": 50, "height": 40}
R_SETUP = {**SETUP, "styles": {"items": [RASTER_ITEM, PICTURE, {**PICTURE, "id": "cizim-1", "format": "svg", "data": "<svg/>"}], "categories": []}}
RGB = {"render": "rgb", "bands": [1, 2, 3]}
RASTER = {"kind": "raster", "affine": [487000, 0.5, 0, 4420800, 0, -0.5], "width": 2000, "height": 1500, "bands": 3, "sample": "u8",
          "file": "orto/pafta-12.tif", "srid": 5256, "style": RGB}
rasters = [
    O(RASTER),
    O({"kind": "raster", "affine": [487000, 0.25, 0, 4420100, 0, -0.25], "width": 50, "height": 40, "bands": 1, "sample": "i8", "asset": RASTER_ID,
       "srid": 5256, "style": {"render": "rampShade", "bands": [1], "stretch": "manual", "min": -20, "max": 90, "ramp": "Arazi", "invert": True,
                               "azimuth": 300, "altitude": 40, "zFactor": 2.5, "nodata": -128, "resampling": "nearest"}, "opacity": 0.7},
      attrs={"Kaynak": "HGM"}),
    # The defaults written out: no stretch, not inverted, bilinear; a band of three in grey; the project's system.
    O({**RASTER, "srid": 0, "style": {"render": "gray", "bands": [2], "stretch": "none", "invert": False, "resampling": "bilinear"}}),
]


def raster_message(kind, *given):
    return {
        "affine": "Rasterin dönüşümü tersinmiyor: pikselin iki kenarı aynı doğrultuda ya da sıfır.",
        "size": "Rasterin genişliği ve yüksekliği 1 ile 4000000 piksel arasında olmalı.",
        "bands": "Rasterin 1 ile 255 arasında bandı olmalı.",
        "both": "Rasterin kaynağı ya gömülü varlık (asset) ya bağlı dosya (file) olmalı, ikisi birden değil.",
        "none": "Rasterin kaynağı yok: gömülü varlığın kimliğini (asset) ya da bağlı dosyanın yolunu (file) verin.",
        "path": "Bağlı dosyanın yolu en çok 4096 harf olmalı ve denetim karakteri içermemeli.",
        "rgb": "RGB görünüş üç bant ister (dördüncüsü alfa olabilir).",
        "one": "Bu görünüş tek bant ister.",
        "band": f"Rasterin {given[0] if given else 0} bandı var; {given[1] if len(given) > 1 else 0}. bant gösterilemez.",
        "manual": "Elle gerdirmenin en küçüğü ve en büyüğü sonlu sayılar olmalı, en küçük en büyükten küçük.",
        "ramp": f"“{given[0] if given else ''}” diye bir renk rampası yok; Gri, Arazi, Spektral, Viridis, Mavi-kırmızı, Sıcaklık rampalarından biri seçilmeli.",
        "light": "Gölgeli kabartmanın ışığı 0–360° doğrultudan, 0–90° yükseklikten gelmeli; yükseklik çarpanı sıfırdan büyük olmalı.",
        "opacity": f"Rasterin donukluğu 0.1 ile 1 arasında olmalı; {given[0] if given else 0} verildi.",
    }[kind]


def unknown_raster(asset):
    return f"“{asset}” kimlikli raster projenin kitaplığında yok: silinmiş ya da başka bir çizimin olabilir. Projenin kitaplığındaki bir GeoTIFF, PNG ya da JPEG'in kimliğini verin."


def raster_with(**style):
    return {**RASTER, "style": {**RGB, **style}}


cases.append({
    "name": "Raster ekle: bağlı ve gömülü raster, görünüşü, donukluğu ve sistemiyle tek adımda yazılır, adı “Raster ekle”; görünüşün varsayılanları alan değildir (ADR 0204 §9)",
    "setup": R_SETUP,
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "operation": "raster", "objects": rasters}, "result": done([3, 4, 5]),
         "expect": {"ids": IDS + [3, 4, 5], "entities": {str(3 + i): made(o, 3 + i) for i, o in enumerate(rasters)},
                    "uids": {"3": "new", "4": "new", "5": "new"}, "revision": "changed"}},
        {"op": "undo", "returns": "Raster ekle", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}},
        {"op": "redo", "returns": "Raster ekle", "expect": {"ids": IDS + [3, 4, 5]}},
    ],
})
cases.append({
    "name": "rasterin kuralları sırayla: invalid_raster, yolu nesnenin geometrisi; sonlu olmayan önce not_finite; gömülü rasterin dosyası projenin kitaplığında GeoTIFF, PNG ya da JPEG olmalı: unknown_asset (ADR 0204 §9)",
    "setup": R_SETUP,
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(RASTER), O({**RASTER, "affine": [487000, 0.5, 0.5, 4420800, 0.5, 0.5]})]},
         "result": failed("invalid_raster", raster_message("affine"), "objects[1].geometry"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**RASTER, "width": 0})]},
         "result": failed("invalid_raster", raster_message("size"), "objects[0].geometry"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**RASTER, "bands": 0})]},
         "result": failed("invalid_raster", raster_message("bands"), "objects[0].geometry"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**RASTER, "asset": RASTER_ID})]},
         "result": failed("invalid_raster", raster_message("both"), "objects[0].geometry"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**{k: v for k, v in RASTER.items() if k != "file"}})]},
         "result": failed("invalid_raster", raster_message("none"), "objects[0].geometry"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**RASTER, "file": "orto/\npafta.tif"})]},
         "result": failed("invalid_raster", raster_message("path"), "objects[0].geometry"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(raster_with(bands=[1, 2]))]},
         "result": failed("invalid_raster", raster_message("rgb"), "objects[0].geometry"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**RASTER, "style": {"render": "hillshade", "bands": [1, 2]}})]},
         "result": failed("invalid_raster", raster_message("one"), "objects[0].geometry"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(raster_with(bands=[1, 2, 4]))]},
         "result": failed("invalid_raster", raster_message("band", 3, 4), "objects[0].geometry"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(raster_with(stretch="manual", min=90, max=-20))]},
         "result": failed("invalid_raster", raster_message("manual"), "objects[0].geometry"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**RASTER, "style": {"render": "ramp", "bands": [1], "ramp": "Gökkuşağı"}})]},
         "result": failed("invalid_raster", raster_message("ramp", "Gökkuşağı"), "objects[0].geometry"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**RASTER, "style": {"render": "hillshade", "bands": [1], "altitude": 95}})]},
         "result": failed("invalid_raster", raster_message("light"), "objects[0].geometry"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**RASTER, "opacity": 0.05})]},
         "result": failed("invalid_raster", raster_message("opacity", 0.05), "objects[0].geometry"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(RASTER)]}, "nonFinite": {"objects[0].geometry.affine[3]": "NaN"},
         "result": not_finite(1), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(raster_with(stretch="manual", min=0, max=255))]},
         "nonFinite": {"objects[0].geometry.style.min": "Infinity"}, "result": not_finite(1), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(RASTER), O({**{k: v for k, v in RASTER.items() if k != "file"}, "asset": "raster-ffffffffffffffff"})]},
         "result": failed("unknown_asset", unknown_raster("raster-ffffffffffffffff"), "objects[1].geometry.asset"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**{k: v for k, v in RASTER.items() if k != "file"}, "asset": "cizim-1"})]},
         "result": failed("unknown_asset", unknown_raster("cizim-1"), "objects[0].geometry.asset"), "expect": NOTHING},
    ],
})


# ── Eğri boyunca yazı (docs/adr/0196 §1, §4) ────────────────────────────

CURVED = {**TEXT, "text": "Çamlıca Deresi", "rotation": 12.5, "align": "bottomCenter", "path": {"pts": [P(40, 0)], "bulges": [0.35]}}
curved = [
    O(CURVED),
    O({**TEXT, "text": "Atatürk Bulvarı", "path": {"pts": [P(18, 0), P(30, -6.5)]}, "mask": True}),
]


def path_message(kind, *given):
    return {
        "empty": "Eğri boyunca yazının eğrisinde köşe yok. En az bir köşe verin ya da eğriyi kaldırın (düz yazı).",
        "bulges": f"Eğri boyunca yazının kavis sayısı ({given[0] if given else 0}) köşe sayısından ({given[1] if len(given) > 1 else 0}) farklı. Her kenara bir kavis verin ya da kavisleri kaldırın (düz kenarlar).",
        "zero": "Eğri boyunca yazının eğrisinin uzunluğu sıfır: bütün köşeleri yazının noktasında. Köşeleri yazının noktasından ayırın.",
        "line": "Eğri boyunca yazı tek satırdır: satır sonu, kutu genişliği ve satır aralığı olmaz. Yazıyı tek satır yapın ya da eğriyi kaldırın.",
        "linked": "Nesneye bağlı yazının eğrisi olmaz. Önce bağı koparın ya da eğriyi kaldırın.",
    }[kind]


cases.append({
    "name": "eğri boyunca yazı eğrisiyle yazılır; adım “Eğri boyunca yazı” (ADR 0196 §1, §4)",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "operation": "textAlong", "objects": curved}, "result": done([3, 4]),
         "expect": {"ids": IDS + [3, 4], "entities": {str(3 + i): made(o, 3 + i) for i, o in enumerate(curved)}, "uids": {"3": "new", "4": "new"},
                    "revision": "changed"}},
        {"op": "undo", "returns": "Eğri boyunca yazı", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}},
        {"op": "redo", "returns": "Eğri boyunca yazı", "expect": {"ids": IDS + [3, 4]}},
    ],
})

cases.append({
    "name": "eğrinin kuralları sırayla: invalid_path, yolu alanın; sonlu olmayan önce not_finite; bağlı yazının eğrisi bağdan sonra (ADR 0196 §1)",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(CURVED), O({**CURVED, "path": {"pts": []}})]},
         "result": failed("invalid_path", path_message("empty"), "objects[1].geometry.path"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**CURVED, "path": {"pts": [P(10, 0), P(20, 2)], "bulges": [0.2]}})]},
         "result": failed("invalid_path", path_message("bulges", 1, 2), "objects[0].geometry.path"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**CURVED, "path": {"pts": [P(0, 0)]}})]},
         "result": failed("invalid_path", path_message("zero"), "objects[0].geometry.path"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**CURVED, "text": "Çamlıca\nDeresi"})]},
         "result": failed("invalid_path", path_message("line"), "objects[0].geometry.path"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**CURVED, "boxWidth": 30})]},
         "result": failed("invalid_path", path_message("line"), "objects[0].geometry.path"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(CURVED, labelOf=uid(1), labelScale=1000)]},
         "result": failed("invalid_path", path_message("linked"), "objects[0].geometry.path"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**CURVED, "path": {"pts": []}})]}, "nonFinite": {"objects[0].geometry.rotation": "NaN"},
         "result": not_finite(1), "note": "Sonlu olmayan sayı eğriden önce.", "expect": NOTHING},
    ],
})


# ── Çizim ekleri (docs/adr/0197 §1–§3) ──────────────────────────────────

extras = [
    ("tangentLine", "İki daireye teğet", [O({"kind": "line", "a": P(487000, 4420004), "b": P(487024, 4420006)})]),
    ("fourthCorner", "Dördüncü köşe", [O({"kind": "polygon", "pts": [P(487000, 4420000), P(487012, 4420000), P(487014, 4420006), P(487002, 4420006)]})]),
    ("rangeRings", "Menzil halkaları", [O({"kind": "circle", "c": P(487050, 4420050), "r": 10}), O({"kind": "circle", "c": P(487050, 4420050), "r": 20}),
                                        O({"kind": "line", "a": P(487050, 4420050), "b": P(487050, 4420070)}),
                                        O({"kind": "line", "a": P(487050, 4420050), "b": P(487050, 4420030)})]),
    # Plan yolu (docs/adr/0198 §2): the road, its carriageway and its axis, their kinds and widths as attributes.
    ("planRoad", "Plan yolu", [O({"kind": "polygon", "pts": [P(487000, 4420095), P(487060, 4420095), P(487060, 4420105), P(487000, 4420105)]},
                                 attrs={"Tür": "Yol", "Genişlik": "10"}),
                               O({"kind": "polygon", "pts": [P(487000, 4420097), P(487060, 4420097), P(487060, 4420103), P(487000, 4420103)]},
                                 attrs={"Tür": "Taşıt yolu", "Genişlik": "6"}),
                               O({"kind": "polyline", "pts": [P(487000, 4420100), P(487060, 4420100)]}, attrs={"Tür": "Yol ekseni"})]),
]
for operation, step, objects in extras:
    new = [3 + i for i in range(len(objects))]
    cases.append({
        "name": f"{step}: nesneleri tek adımda yazılır, adı “{step}” (ADR 0197)",
        "steps": [
            {"op": "execute", "input": {"layerId": "yapi", "operation": operation, "objects": objects}, "result": done(new),
             "expect": {"ids": IDS + new, "entities": {str(i): made(o, i) for i, o in zip(new, objects)}, "uids": {str(i): "new" for i in new},
                        "revision": "changed"}},
            {"op": "undo", "returns": step, "expect": {"ids": IDS, "canUndo": False, "canRedo": True}},
            {"op": "redo", "returns": step, "expect": {"ids": IDS + new}},
        ],
    })

# Yatay ağ dengelemesi's new points (docs/adr/0203 §8): named, their kind “Ağ noktası”, one step named after the window.
network_points = [O({"kind": "point", "p": P(487040.0123, 4420040.0456)}, label="Y2", attrs={"Ad": "Y2", "Tür": "Ağ noktası"}),
                  O({"kind": "point", "p": P(487060.5, 4420020.25)}, label="Y3", attrs={"Ad": "Y3", "Tür": "Ağ noktası"})]
new = [3 + i for i in range(len(network_points))]
cases.append({
    "name": "Yatay ağ dengelemesi: yeni noktalar adlarıyla ve “Ağ noktası” türüyle tek adımda yazılır, adı “Yatay ağ dengelemesi” (ADR 0203 §8)",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "operation": "networkAdjust", "objects": network_points}, "result": done(new),
         "expect": {"ids": IDS + new, "entities": {str(i): made(o, i) for i, o in zip(new, network_points)}, "uids": {str(i): "new" for i in new},
                    "revision": "changed"}},
        {"op": "undo", "returns": "Yatay ağ dengelemesi", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}},
        {"op": "redo", "returns": "Yatay ağ dengelemesi", "expect": {"ids": IDS + new}},
    ],
})


# ── Kılavuz (docs/adr/0146) ─────────────────────────────────────────────

LEADER = {"kind": "leader", "pts": [P(487060, 4420110), P(487066, 4420115)], "text": "Mevcut bina", "height": 2.5, "rotation": 0}
leaders = [
    O(LEADER),
    O({"kind": "leader", "pts": [P(487090, 4420110), P(487086, 4420114), P(487080, 4420116)], "text": "Ø150 PVC", "height": 2,
       "rotation": 0, "arrow": "open", "mask": True}),
    O({"kind": "leader", "pts": [P(487100, 4420110), P(487106, 4420118)], "text": "Ada 101 Parsel 5", "height": 1.5, "rotation": 30,
       "arrow": "dot", "mask": False}, color="#E5484D"),
    O({"kind": "leader", "pts": [P(487060, 4420100), P(487065, 4420104), P(487069, 4420104)], "height": 2.5, "rotation": 0, "arrow": "none"}),
]


def leader_message(kind, given):
    return {
        "few": f"Kılavuzun en az 2 köşesi olmalı; {given} köşe verildi. Okun ucunu ve en az bir köşe daha verin.",
        "empty": "Kılavuzun notu boş olamaz; yalnız boşluktan oluşan not da boştur. Notu yazın ya da notsuz kılavuz için alanı kaldırın.",
        "height": f"Kılavuzun yüksekliği sıfırdan büyük olmalı; {given} verildi. Notun yüksekliğini metre olarak, pozitif verin.",
    }[kind]


cases.append({
    "name": "Kılavuz: köşeleri, notu, yüksekliği, dönüşü, oku ve zemini tek adımda yazılır, adı “Kılavuz”; dolu ok, zeminsizlik ve notsuzluk alan değildir (ADR 0146)",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "operation": "leader", "objects": leaders}, "result": done([3, 4, 5, 6]),
         "expect": {"ids": IDS + [3, 4, 5, 6], "entities": {str(3 + i): made(o, 3 + i) for i, o in enumerate(leaders)}, "uids": {"3": "new", "4": "new", "5": "new", "6": "new"},
                    "revision": "changed"}},
        {"op": "undo", "returns": "Kılavuz", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}},
        {"op": "redo", "returns": "Kılavuz", "expect": {"ids": IDS + [3, 4, 5, 6]}},
    ],
})

cases.append({
    "name": "kılavuzun en az 2 köşesi, boş olmayan notu ve sıfırdan büyük yüksekliği olur; köşe ve not sayılardan önce, yükseklik sonlu sayılardan sonra denetlenir (ADR 0146)",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(LEADER), O({**LEADER, "pts": [P(487060, 4420110)]})]},
         "result": failed("too_few_points", leader_message("few", 1), "objects[1].geometry.pts"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**LEADER, "text": ""})]},
         "result": failed("empty_text", leader_message("empty", None), "objects[0].geometry.text"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**LEADER, "text": " \t\u00a0"})]},
         "result": failed("empty_text", leader_message("empty", None), "objects[0].geometry.text"), "note": "Yalnız boşluk da boştur.", "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**LEADER, "height": 0})]},
         "result": failed("invalid_height", leader_message("height", 0), "objects[0].geometry.height"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**LEADER, "height": -2.5})]},
         "result": failed("invalid_height", leader_message("height", -2.5), "objects[0].geometry.height"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**LEADER, "height": 0})]}, "nonFinite": {"objects[0].geometry.rotation": "NaN"},
         "result": not_finite(1), "note": "Sonlu olmayan sayı yükseklikten önce.", "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**LEADER, "pts": [P(487060, 4420110)]})]}, "nonFinite": {"objects[0].geometry.height": "NaN"},
         "result": failed("too_few_points", leader_message("few", 1), "objects[0].geometry.pts"), "note": "Köşe sayısı sayılardan önce.", "expect": NOTHING},
    ],
})


# ── Yeni ölçü türleri (docs/adr/0147 §6) ────────────────────────────────

ORD_Y = {"kind": "dimension", "a": P(487000, 4420130), "b": P(487006, 4420150), "offset": 0, "height": 2.5, "style": "ordinate", "angle": 0}
ORD_X = {"kind": "dimension", "a": P(487000, 4420130), "b": P(486980, 4420136), "offset": 0, "height": 2.5, "style": "ordinate", "angle": 90, "mask": True}
ARC_LENGTH = {"kind": "dimension", "a": P(487060, 4420130), "b": P(487050, 4420140), "c": P(487050, 4420130), "offset": 2, "height": 2, "style": "arcLength"}
# The radius from (487100, 4419830) to (487100, 4420130): the centre shown 20 m back along it and 4 m aside.
JOGGED = {"kind": "dimension", "a": P(487100, 4419830), "b": P(487100, 4420130), "c": P(487104, 4420110), "offset": 5, "height": 2, "style": "jogged"}
AZIMUTH = {"kind": "dimension", "a": P(487000, 4420160), "b": P(487030, 4420200), "offset": 2, "height": 2.5, "style": "azimuth"}
SLOPE = {"kind": "dimension", "a": P(487040, 4420160), "b": P(487080, 4420160), "offset": 1.5, "height": 2, "style": "slope", "za": 105.25, "zb": 104.75, "mask": True}
PLAIN = {"kind": "dimension", "a": P(487000, 4420210), "b": P(487020, 4420210), "offset": 3, "height": 0.5, "mask": False}
dimensions = [O(ORD_Y), O(ORD_X), O(ARC_LENGTH), O(JOGGED), O(AZIMUTH), O(SLOPE), O(PLAIN)]


def without(g, key):
    return {k: v for k, v in g.items() if k != key}


def dimension_message(kind, given=None):
    return {
        "elevations": "Kot yalnız eğim ölçüsünde olur. Kotları (za, zb) kaldırın ya da ölçünün biçimini eğim yapın.",
        "axis": f"Koordinat ölçüsünün ekseni 0 (Y) ya da 90 (X) olmalı; {given} verildi. Y için 0, X için 90 verin.",
        "ordinateTooShort": "Koordinat ölçüsünün çizgisi noktadan eksene dik yönde yazı yüksekliğinin yarısından uzun olmalı. Çizginin ucunu (b) noktadan daha uzağa verin.",
        "arcNoCentre": "Yay uzunluğu ölçüsünün merkezi (c) verilmeli. Ölçülen yayın merkezini verin.",
        "arcNoRadius": "Yay uzunluğu ölçüsünde yayın başlangıcı (a) merkezde (c); yarıçap sıfır. Başlangıcı yayın üstünde verin.",
        "arcNoSweep": "Yay uzunluğu ölçüsünde yayın iki ucu merkezden aynı doğrultuda; yayın açısı sıfır. Sonu (b) başka bir doğrultuda verin.",
        "arcInside": "Yay uzunluğu ölçüsünün ölçü yayı merkeze ulaşıyor: içe ötelenme yarıçaptan küçük olmalı. Ötelenmeyi büyütün.",
        "joggedNoCentre": "Kırıklı yarıçap ölçüsünün gösterilen merkezi (c) verilmeli. Çizginin başlayacağı noktayı verin.",
        "joggedNoRadius": "Kırıklı yarıçap ölçüsünde yaydaki nokta (b) merkezde (a); yarıçap sıfır. Noktayı yayın üstünde verin.",
        "joggedCentre": "Kırıklı yarıçap ölçüsünde gösterilen merkez (c) yarıçap boyunca yaydaki noktadan (b) geride olmalı; yarıçap çizgisinden uzaklığı bu geriliği aşmamalı. Gösterilen merkezi yayın içinde, yarıçapa yakın verin.",
        "edgeTooShort": f"{given} ölçüsünün iki ucu aynı nokta; ölçülecek kenar yok. Kenarın öbür ucunu (b) verin.",
        "slopeNoElevations": "Eğim ölçüsünün iki ucunun da kotu verilmeli. Eksik kotu (za ya da zb) metre olarak verin.",
    }[kind]


def bad_dimension(geometry, kind, field, given=None, at=0, before=()):
    objects = [O(g) for g in before] + [O(geometry)]
    return {"op": "execute", "input": {"layerId": "yapi", "objects": objects},
            "result": failed("invalid_dimension", dimension_message(kind, given), f"objects[{at}].geometry.{field}"), "expect": NOTHING}


cases.append({
    "name": "yeni ölçü türleri (ADR 0147): koordinat (Y ve zeminli X), yay uzunluğu, kırıklı yarıçap, semt ve eğim (kotlarıyla, zeminli) verildiği gibi tek adımda yazılır; zeminsizlik alan değildir",
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": dimensions}, "result": done(list(range(3, 10))),
         "expect": {"ids": IDS + list(range(3, 10)), "entities": {str(3 + i): made(o, 3 + i) for i, o in enumerate(dimensions)},
                    "uids": {str(3 + i): "new" for i in range(len(dimensions))}, "revision": "changed"}},
        {"op": "undo", "returns": "Ekle", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}},
    ],
})

cases.append({
    "name": "yeni ölçü türlerinin kuralları (ADR 0147 §6): kot yalnız eğimde, koordinatın ekseni 0 ya da 90, sonra çekirdeğin çizemediği ölçü; her biri invalid_dimension, düzeltilecek alanın yoluyla; sonlu olmayan sayı önce",
    "steps": [
        bad_dimension({**PLAIN, "za": 10}, "elevations", "za"),
        bad_dimension({**PLAIN, "zb": 10}, "elevations", "zb"),
        bad_dimension({**ORD_Y, "angle": 45}, "axis", "angle", 45, at=1, before=[ORD_Y]),
        bad_dimension({**ORD_Y, "b": P(487005, 4420131)}, "ordinateTooShort", "b"),
        bad_dimension(without(ARC_LENGTH, "c"), "arcNoCentre", "c"),
        bad_dimension({**ARC_LENGTH, "a": P(487050, 4420130)}, "arcNoRadius", "a"),
        bad_dimension({**ARC_LENGTH, "b": P(487070, 4420130)}, "arcNoSweep", "b"),
        bad_dimension({**ARC_LENGTH, "offset": -10}, "arcInside", "offset"),
        bad_dimension(without(JOGGED, "c"), "joggedNoCentre", "c"),
        bad_dimension({**JOGGED, "b": P(487100, 4419830)}, "joggedNoRadius", "b"),
        bad_dimension({**JOGGED, "c": P(487104, 4420131)}, "joggedCentre", "c"),
        bad_dimension({**AZIMUTH, "b": P(487000, 4420160)}, "edgeTooShort", "b", "Semt"),
        bad_dimension({**SLOPE, "b": P(487040, 4420160)}, "edgeTooShort", "b", "Eğim"),
        bad_dimension(without(SLOPE, "zb"), "slopeNoElevations", "zb"),
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(SLOPE)]}, "nonFinite": {"objects[0].geometry.za": "NaN"},
         "result": not_finite(1), "note": "Kotlar da sonlu sayıdır.", "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O({**ARC_LENGTH, "offset": -10})]}, "nonFinite": {"objects[0].geometry.c.x": "Infinity"},
         "result": not_finite(1), "note": "Sonlu olmayan sayı ölçünün kurallarından önce.", "expect": NOTHING},
    ],
})


# White space other than the plain space, escaped so a reader sees it (the empty text case).
INVISIBLE = "\u0085\u00a0\u1680\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200a\u2028\u2029\u202f\u205f\u3000\ufeff"


def compact(v):
    text = json.dumps(v, ensure_ascii=False, separators=(", ", ": "))
    return "".join(f"\\u{ord(c):04x}" if c in INVISIBLE else c for c in text)


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
        if "setup" in c:
            lines.append(f'      "setup": {compact(c["setup"])},')
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


# A layer's fields (docs/adr/0199 §2): a value given to a field's key is written in its canonical text, one that does
# not keep its rules refused (`invalid_attribute`; an empty one of a required field `attribute_required`; path
# objects[i].attrs.<name>, objects in order, names in theirs); a field the object does not give takes its default; a
# required field without a value or a default is not refused (the drawing tools know no fields). The rules and their
# words are the independent reference's (scripts/fixtures/layer_field_cases.py).
sys.path.insert(0, str(__import__("pathlib").Path(__file__).resolve().parent))
from layer_field_cases import check as field_check  # noqa: E402

YAPI_FIELDS = [
    {"name": "Ada", "kind": "text", "required": True},
    {"name": "Kat", "kind": "integer", "min": "1", "default": "1"},
    {"name": "Durum", "kind": "text", "values": [{"code": "M", "label": "Mevcut"}, {"code": "P", "label": "Proje"}], "default": "M"},
    {"name": "Tarih", "kind": "date"},
]
F_SETUP = {**SETUP, "layers": [{**SETUP["layers"][0], "fields": YAPI_FIELDS}] + SETUP["layers"][1:]}
FIELD = {f["name"]: f for f in YAPI_FIELDS}
SPOT = point(487070, 4420070)
SPOT2 = point(487072, 4420070)


def refused_at(i, name, text):
    r = field_check(FIELD[name], text)
    assert "error" in r, (name, text)
    return failed("attribute_required" if r["error"] == "required" else "invalid_attribute", r["message"], f"objects[{i}].attrs.{name}")


def with_attrs(obj, attrs):
    return {**obj, "attrs": attrs}


cases.append({
    "name": "Katmanın alanları: verilen değer tek biçimiyle yazılır, verilmeyen alan varsayılanını alır; zorunlu alanın değeri ve varsayılanı yoksa reddedilmez",
    "note": "ADR 0199 §2. Yapı katmanının alanları: Ada (zorunlu, varsayılansız), Kat (en az 1, varsayılan 1), Durum (değer listesi, varsayılan M), Tarih.",
    "setup": F_SETUP,
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(SPOT, attrs={"Kat": "+03", "Tarih": "7.10.2026", "Not": " serbest "}), O(SPOT2)]}, "result": done([3, 4]),
         "expect": {"ids": IDS + [3, 4],
                    "entities": {"3": made(with_attrs(O(SPOT), {"Kat": "3", "Tarih": "2026-10-07", "Not": " serbest ", "Durum": "M"}), 3),
                                 "4": made(with_attrs(O(SPOT2), {"Kat": "1", "Durum": "M"}), 4)},
                    "canUndo": True, "revision": "changed"}},
        {"op": "undo", "returns": "Ekle", "expect": {"ids": IDS}},
        {"op": "plan", "input": {"layerId": "yapi", "objects": [O(SPOT, attrs={"Durum": "proje"})]},
         "result": {"status": "completed", "output": {"entities": [{**made(with_attrs(O(SPOT), {"Durum": "P", "Kat": "1"}), 0)}], "revision": "$current"}, "warnings": []},
         "note": "Etiket kodunu verir; plan da tek biçimi ve varsayılanları gösterir.", "expect": {"revision": "same"}},
        {"op": "execute", "input": {"layerId": "ada", "objects": [O(SPOT, attrs={"Kat": "+03"})]}, "result": done([5]),
         "note": "Alanları olmayan katmanda değer verildiği gibi yazılır.", "expect": {"entities": {"5": made(with_attrs(O(SPOT), {"Kat": "+03"}), 5, "ada")}}},
    ],
})

cases.append({
    "name": "Katmanın alanları: kurala uymayan değer invalid_attribute, zorunlu alana boş attribute_required; nesneler sırayla, adlar kendi sıralarıyla; hiçbir şey yazılmaz",
    "note": "ADR 0199 §2. Yol objects[i].attrs.<ad>.",
    "setup": F_SETUP,
    "steps": [
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(SPOT), O(SPOT2, attrs={"Kat": "0", "Durum": "X"})]}, "result": refused_at(1, "Durum", "X"),
         "note": "İkinci nesnenin iki uymayanından adların sırasıyla ilki (Durum, Kat'tan önce).", "expect": {"ids": IDS, "revision": "same", "canUndo": False}},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(SPOT, attrs={"Kat": "2.5"})]}, "result": refused_at(0, "Kat", "2.5"), "expect": {"revision": "same"}},
        {"op": "validate", "input": {"layerId": "yapi", "objects": [O(SPOT, attrs={"Ada": " "})]}, "result": refused_at(0, "Ada", " ")},
        {"op": "execute", "input": {"layerId": "yapi", "objects": [O(SPOT, attrs={"Tarih": "31.02.2026"})]}, "result": refused_at(0, "Tarih", "31.02.2026"), "expect": {"revision": "same"}},
    ],
})

write(
    "cad.entities.create",
    "Nesneleri ekle: doğrulama, plan, yazma, geri alma",
    "ADR 0057. Denetim sırası: en az bir nesne; her nesnenin geometrisi, sırayla, cad.entities.edit'in kurallarıyla (nokta ve köşe sayısı: kapalı alanın halkası en az 3 köşeli, iki kenarından biri yaysa 2; yazının boş olmayan metni, sonlu sayılar, ölçünün kuralları (ADR 0147: kot yalnız eğimde, koordinatın ekseni 0 ya da 90, sonra çekirdeğin çizebildiği ölçü; invalid_dimension), yarıçap); beklenen sürümün yazımı, sonra çizimin sürümü; katman (var, grup değil, kilitli değil; gizliyse uyarı). Nesne verilen geometrisi, girdinin katmanı ve verildiyse rengi, öznitelikleri (yoksa boş), etiketi ve sembolüyle yazılır (ADR 0176: sembolün kimliği kitaplıkta aranmaz). Adım “Ekle” ya da işlemin adıdır: Paralel çizgi, Dik in, Dik çık, Böl, Tarama, Alan oluştur, Toplu alan, Köşelere nokta, Bitişik alan, Etiketleri yazıya çevir. Blok yerleştirmesi (ADR 0144) çizimde tanımlı bir bloğu adlandırır (unknown_block, katmandan sonra, sırayla), ölçeği sıfırdan büyüktür (invalid_scale); aynalama yalnız true yazılır; blok durumlarının kendi kurulumu vardır. Katmanın alanları (ADR 0199 §2): verilen öznitelikler katmanın alanlarıyla denetlenir ve tek biçimleriyle yazılır, verilmeyen alan varsayılanını alır; kurala uymayan invalid_attribute, zorunlu alana boş attribute_required (yol objects[i].attrs.<ad>; nesneler sırayla, adlar kendi sıralarıyla; bağlı yazının nesnesinden sonra); değeri ve varsayılanı olmayan zorunlu alan reddedilmez. Kurulumdaki en büyük kimlik 2; yeni nesneler 3'ten başlar. $uidOf:N, N yuvasındaki nesnenin kalıcı kimliğidir.",
    cases,
)
print(f"{len(cases)} cases" + (" match" if "--check" in sys.argv[1:] else " written"))
