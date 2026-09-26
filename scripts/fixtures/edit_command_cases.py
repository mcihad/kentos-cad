"""The shared cases of the product command cad.entities.edit (docs/adr/0047).

    python3 scripts/fixtures/edit_command_cases.py           # writes the file
    python3 scripts/fixtures/edit_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.edit.json. The checks, their order,
codes, paths and messages are written here by hand from the ADR, and so is
what each change makes of an object (`updated`, `inherited` below: the
contract's rules, not an implementation's output). The geometry is given in
the input; the web's and the desktop's handlers must write it as given.

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


HALF_PI = math.pi / 2
HOLE = {"pts": [P(487038, 4420008), P(487042, 4420008), P(487042, 4420012), P(487038, 4420012)]}

ENTITIES = [
    {"kind": "line", "id": 1, "layerId": "yapi", "color": "#E5484D", "attrs": {"Tür": "Duvar"}, "label": "D1", "symbol": "cit", "a": P(487000, 4420000), "b": P(487020, 4420000)},
    {"kind": "polyline", "id": 2, "layerId": "yapi", "attrs": {"Ad": "Yol"}, "label": "Y1", "pts": [P(487000, 4420010), P(487010, 4420010), P(487010, 4420020)]},
    {"kind": "polygon", "id": 3, "layerId": "yapi", "attrs": {"Ada": "101"}, "pts": [P(487030, 4420000), P(487050, 4420000), P(487050, 4420020), P(487030, 4420020)], "holes": [HOLE]},
    {"kind": "arc", "id": 4, "layerId": "yapi", "attrs": {}, "c": P(487060, 4420000), "r": 5, "a0": 0, "a1": HALF_PI},
    {"kind": "line", "id": 5, "layerId": "kilitli", "attrs": {}, "a": P(487000, 4420030), "b": P(487010, 4420030)},
    {"kind": "line", "id": 6, "layerId": "eski", "attrs": {}, "a": P(487000, 4420035), "b": P(487010, 4420035)},
    {"kind": "line", "id": 7, "layerId": "gizli", "attrs": {}, "a": P(487000, 4420040), "b": P(487010, 4420040)},
    {"kind": "circle", "id": 8, "layerId": "yapi", "attrs": {}, "c": P(487070, 4420010), "r": 3},
]
BY_ID = {e["id"]: e for e in ENTITIES}
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
        group("arsiv", "Arşiv", [layer("eski", "Eski")], locked=True),
    ],
    "activeLayer": "yapi",
    "entities": ENTITIES,
    "styles": {"items": [], "categories": []},
}

# ── What a change makes of an object (the contract) ────────────────────

# The geometry fields of each kind: what `update` replaces.
GEOMETRY = {
    "point": ["p", "z"],
    "line": ["a", "b"],
    "polyline": ["pts", "bulges", "holes"],
    "polygon": ["pts", "bulges", "holes"],
    "circle": ["c", "r"],
    "arc": ["c", "r", "a0", "a1"],
    "ellipse": ["c", "major", "ratio", "t0", "t1"],
    "spline": ["pts", "closed"],
    "xline": ["p", "dir"],
    "ray": ["p", "dir"],
    "text": ["p", "text", "height", "rotation"],
    "dimension": ["a", "b", "offset", "height", "text", "style", "angle", "c"],
    "hatch": ["ring", "holes", "pattern"],
}


def E(i):
    return json.loads(json.dumps(BY_ID[i]))


def reshaped(e, geometry):
    """`update` of the object `e`: another geometry; every other field of its own stays."""
    out = {k: v for k, v in e.items() if k != "kind" and k not in GEOMETRY[e["kind"]]}
    out.update(json.loads(json.dumps(geometry)))
    return out


def updated(i, geometry):
    """`update`: the object with another geometry; every other field of its own stays."""
    return reshaped(E(i), geometry)


def inherited(i, geometry, keep, slot):
    """`replace` (in slot i) and `add` (in a new slot): the layer and the colour; the
    attributes and the label with keepData; never the symbol."""
    e = E(i)
    out = json.loads(json.dumps(geometry))
    out["id"] = slot
    out["layerId"] = e["layerId"]
    if "color" in e:
        out["color"] = e["color"]
    out["attrs"] = e["attrs"] if keep else {}
    if keep and "label" in e:
        out["label"] = e["label"]
    return out


def line(ax, ay, bx, by):
    return {"kind": "line", "a": P(ax, ay), "b": P(bx, by)}


def uid(i):
    return f"$uidOf:{i}"


def done(changed=(), created=(), removed=()):
    return {"status": "completed", "output": {"changed": list(changed), "created": list(created), "removed": list(removed), "revision": "$current"}, "warnings": []}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def ids_after(removed=(), added=()):
    return [i for i in IDS if i not in removed] + list(added)


NOTHING = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
UID_MESSAGE = "“{}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın."
MISSING = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"
MISSING_MESSAGE = f"“{MISSING}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."


def locked_message(name):
    return f"“{name}” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın."


def not_finite_message(n):
    return f"{n}. değişikliğin geometrisinde sonlu olmayan bir değer var (NaN ya da sonsuz). Geometriyi sonlu sayılarla verin."


cases = []

# ── Writing ────────────────────────────────────────────────────────────

extended = line(487000, 4420000, 487030, 4420000)
cases.append({
    "name": "update (Uzat): yalnız geometri değişir; yuvası, kalıcı kimliği, katmanı, rengi, öznitelikleri, etiketi ve simgesi kalır; tek adımda geri alınır, yinelenir",
    "steps": [
        {"op": "captureUid", "id": 1, "as": "cizgi"},
        {"op": "execute", "input": {"operation": "extend", "changes": [{"kind": "update", "uid": uid(1), "geometry": extended}]}, "result": done(changed=[uid(1)]),
         "expect": {"ids": IDS, "entities": {"1": updated(1, extended)}, "uids": {"1": "cizgi"}, "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}},
        {"op": "undo", "returns": "Uzat", "note": "Uzat aracının adımı.", "expect": {"entities": {"1": E(1)}, "uids": {"1": "cizgi"}, "canUndo": False, "canRedo": True, "revision": "changed"}},
        {"op": "redo", "returns": "Uzat", "expect": {"entities": {"1": updated(1, extended)}, "canUndo": True, "canRedo": False, "revision": "changed"}},
    ],
})

left, right = line(487000, 4420000, 487008, 4420000), line(487012, 4420000, 487020, 4420000)
cases.append({
    "name": "replace ve add (Buda): ilk parça nesnenin kendisidir (yuvası, kalıcı kimliği, katmanı, rengi); keepData yoksa öznitelik ve etiket gelmez; simge hiç gelmez; öbür parça ondan yeni nesnedir",
    "steps": [
        {"op": "captureUid", "id": 1, "as": "cizgi"},
        {"op": "execute", "input": {"operation": "trim", "changes": [{"kind": "replace", "uid": uid(1), "geometry": left}, {"kind": "add", "from": uid(1), "geometry": right}]},
         "result": done(changed=[uid(1)], created=[uid(9)]),
         "expect": {"ids": ids_after(added=[9]), "entities": {"1": inherited(1, left, False, 1), "9": inherited(1, right, False, 9)}, "uids": {"1": "cizgi", "9": "new"}, "canUndo": True, "revision": "changed"}},
        {"op": "undo", "returns": "Buda", "note": "İki parça tek adımda geri alınır; nesne kimliğiyle yerine döner.", "expect": {"ids": IDS, "entities": {"1": E(1)}, "uids": {"1": "cizgi"}, "canUndo": False}},
    ],
})

bent = {"kind": "polyline", "pts": [P(487000, 4420000), P(487010, 4420002), P(487020, 4420000)]}
cases.append({
    "name": "replace keepData ile (Köşe ekle): çizgi yerinde çoklu çizgi olur; öznitelikleri ve etiketi kalır, simgesi gelmez",
    "steps": [
        {"op": "captureUid", "id": 1, "as": "cizgi"},
        {"op": "execute", "input": {"operation": "vertexAdd", "changes": [{"kind": "replace", "uid": uid(1), "geometry": bent, "keepData": True}]}, "result": done(changed=[uid(1)]),
         "expect": {"ids": IDS, "entities": {"1": inherited(1, bent, True, 1)}, "uids": {"1": "cizgi"}, "revision": "changed"}},
        {"op": "undo", "returns": "Köşe ekle", "expect": {"entities": {"1": E(1)}, "uids": {"1": "cizgi"}}},
    ],
})

straight = line(487000, 4420010, 487010, 4420020)
cases.append({
    "name": "update türü de değiştirir (Köşe sil): çoklu çizgi yerinde çizgi olur, öbür alanları kalır",
    "steps": [
        {"op": "captureUid", "id": 2, "as": "yol"},
        {"op": "execute", "input": {"operation": "vertexRemove", "changes": [{"kind": "update", "uid": uid(2), "geometry": straight}]}, "result": done(changed=[uid(2)]),
         "expect": {"entities": {"2": updated(2, straight)}, "uids": {"2": "yol"}, "revision": "changed"}},
        {"op": "undo", "returns": "Köşe sil", "expect": {"entities": {"2": E(2)}, "uids": {"2": "yol"}}},
    ],
})

chain = {"kind": "polyline", "pts": [P(487020, 4420000), P(487000, 4420000), P(487000, 4420010), P(487010, 4420010), P(487010, 4420020)]}
cases.append({
    "name": "Birleştir: zincir ilk nesnesinin yerinde, kimliğiyle ve verisiyle (keepData) yazılır; öbürü silinir; tek adım",
    "steps": [
        {"op": "captureUid", "id": 1, "as": "cizgi"},
        {"op": "captureUid", "id": 2, "as": "yol"},
        {"op": "execute", "input": {"operation": "join", "changes": [{"kind": "replace", "uid": uid(2), "geometry": chain, "keepData": True}, {"kind": "remove", "uid": uid(1)}]},
         "result": done(changed=["$uid:yol"], removed=["$uid:cizgi"]),
         "expect": {"ids": ids_after(removed=[1]), "entities": {"2": inherited(2, chain, True, 2)}, "uids": {"2": "yol"}, "revision": "changed"}},
        {"op": "undo", "returns": "Birleştir", "note": "Silinen nesne de kimliğiyle yerine döner.", "expect": {"ids": IDS, "entities": {"1": E(1), "2": E(2)}, "uids": {"1": "cizgi", "2": "yol"}}},
    ],
})

piece1, piece2 = line(487000, 4420010, 487010, 4420010), line(487010, 4420010, 487010, 4420020)
cases.append({
    "name": "Patlat: nesne silinir, parçaları ondan yeni nesnelerdir (katmanı ve rengi; öznitelik ve etiket gelmez); remove'dan sonra da add aynı nesneden gelebilir",
    "steps": [
        {"op": "captureUid", "id": 2, "as": "yol"},
        {"op": "execute", "input": {"operation": "explode", "changes": [{"kind": "remove", "uid": uid(2)}, {"kind": "add", "from": uid(2), "geometry": piece1}, {"kind": "add", "from": uid(2), "geometry": piece2}]},
         "result": done(created=[uid(9), uid(10)], removed=["$uid:yol"]),
         "expect": {"ids": ids_after(removed=[2], added=[9, 10]), "entities": {"9": inherited(2, piece1, False, 9), "10": inherited(2, piece2, False, 10)}, "uids": {"9": "new", "10": "new"}, "revision": "changed"}},
        {"op": "undo", "returns": "Patlat", "expect": {"ids": IDS, "entities": {"2": E(2)}, "uids": {"2": "yol"}}},
    ],
})

copy = line(487000, 4420001, 487020, 4420001)
cases.append({
    "name": "Ötele: yeni nesne kaynağının katmanını ve rengini alır; kaynak değişmez",
    "steps": [
        {"op": "execute", "input": {"operation": "offset", "changes": [{"kind": "add", "from": uid(1), "geometry": copy}]}, "result": done(created=[uid(9)]),
         "expect": {"ids": ids_after(added=[9]), "entities": {"1": E(1), "9": inherited(1, copy, False, 9)}, "uids": {"9": "new"}, "revision": "changed"}},
        {"op": "undo", "returns": "Ötele", "expect": {"ids": IDS}},
    ],
})

rounded = {"kind": "polygon", "pts": [P(487030, 4420000), P(487048, 4420000), P(487050, 4420002), P(487050, 4420020), P(487030, 4420020)], "bulges": [0, 0.41421356237309503, 0, 0, 0], "holes": [HOLE]}
fillet_arc = {"kind": "arc", "c": P(487010, 4420010), "r": 2, "a0": 0, "a1": HALF_PI}
cases.append({
    "name": "Köşe yuvarla: delikli kapalı alan verilen geometriyle (delikleri dahil) güncellenir; yay başka bir nesneden yeni nesnedir; tek adım",
    "steps": [
        {"op": "execute", "input": {"operation": "fillet", "changes": [{"kind": "update", "uid": uid(3), "geometry": rounded}, {"kind": "add", "from": uid(1), "geometry": fillet_arc}]},
         "result": done(changed=[uid(3)], created=[uid(9)]),
         "expect": {"ids": ids_after(added=[9]), "entities": {"3": updated(3, rounded), "9": inherited(1, fillet_arc, False, 9)}, "revision": "changed"}},
        {"op": "undo", "returns": "Köşe yuvarla", "expect": {"ids": IDS, "entities": {"3": E(3)}}},
    ],
})

cut1, cut2 = line(487000, 4420000, 487018, 4420000), {"kind": "arc", "c": P(487060, 4420000), "r": 5, "a0": 0.25, "a1": HALF_PI}
cases.append({
    "name": "Pah: iki nesne birden güncellenir, girdinin sırasıyla; tek adım",
    "steps": [
        {"op": "execute", "input": {"operation": "chamfer", "changes": [{"kind": "update", "uid": uid(4), "geometry": cut2}, {"kind": "update", "uid": uid(1), "geometry": cut1}]},
         "result": done(changed=[uid(4), uid(1)]),
         "expect": {"ids": IDS, "entities": {"1": updated(1, cut1), "4": updated(4, cut2)}, "revision": "changed"}},
        {"op": "undo", "returns": "Pah", "expect": {"entities": {"1": E(1), "4": E(4)}, "canUndo": False}},
    ],
})

broken = {"kind": "arc", "c": P(487070, 4420010), "r": 3, "a0": 1, "a1": 5}
longer = {"kind": "arc", "c": P(487060, 4420000), "r": 5, "a0": 0, "a1": 2}
stretched = line(487000, 4420000, 487020, 4420004)
cases.append({
    "name": "adımın adı işlemindir: Kır (daire yerinde yay olur), Uzat-kısalt, Esnet",
    "steps": [
        {"op": "execute", "input": {"operation": "break", "changes": [{"kind": "replace", "uid": uid(8), "geometry": broken}]}, "result": done(changed=[uid(8)]),
         "expect": {"entities": {"8": inherited(8, broken, False, 8)}, "revision": "changed"}},
        {"op": "execute", "input": {"operation": "lengthen", "changes": [{"kind": "update", "uid": uid(4), "geometry": longer}]}, "result": done(changed=[uid(4)]),
         "expect": {"entities": {"4": updated(4, longer)}, "revision": "changed"}},
        {"op": "execute", "input": {"operation": "stretch", "changes": [{"kind": "update", "uid": uid(1), "geometry": stretched}]}, "result": done(changed=[uid(1)]),
         "expect": {"entities": {"1": updated(1, stretched)}, "revision": "changed"}},
        {"op": "undo", "returns": "Esnet"},
        {"op": "undo", "returns": "Uzat-kısalt"},
        {"op": "undo", "returns": "Kır", "expect": {"entities": {"1": E(1), "4": E(4), "8": E(8)}, "canUndo": False}},
    ],
})

HATCH = {"kind": "hatch", "ring": [P(487030, 4420000), P(487050, 4420000), P(487050, 4420020), P(487030, 4420020)],
         "holes": [[P(487038, 4420008), P(487042, 4420008), P(487042, 4420012)]], "pattern": {"type": "lines", "angle": 45, "spacing": 1.5}}
DIMENSION = {"kind": "dimension", "a": P(487000, 4420000), "b": P(487020, 4420000), "offset": 3, "height": 0.5, "text": "20,00", "style": "linear", "angle": 0}
HATCH_STRETCHED = {**HATCH, "ring": [P(487030, 4420000), P(487055, 4420000), P(487055, 4420020), P(487030, 4420020)]}
DIMENSION_STRETCHED = {**DIMENSION, "b": P(487025, 4420000)}
hatch_made, dimension_made = inherited(3, HATCH, False, 9), inherited(1, DIMENSION, False, 10)
cases.append({
    "name": "tarama ve ölçü de yazılır: add ile yapılır, Esnet ile köşeleri update edilir; deseni, stili, yazısı kalır",
    "note": "Esnet taramanın ve ölçünün pencerede kalan köşelerini taşır (ADR 0047, 2. kısım).",
    "steps": [
        {"op": "execute", "input": {"operation": "offset", "changes": [{"kind": "add", "from": uid(3), "geometry": HATCH}, {"kind": "add", "from": uid(1), "geometry": DIMENSION}]},
         "result": done(created=[uid(9), uid(10)]),
         "expect": {"ids": ids_after(added=[9, 10]), "entities": {"9": hatch_made, "10": dimension_made}, "revision": "changed"}},
        {"op": "execute", "input": {"operation": "stretch", "changes": [{"kind": "update", "uid": uid(9), "geometry": HATCH_STRETCHED}, {"kind": "update", "uid": uid(10), "geometry": DIMENSION_STRETCHED}]},
         "result": done(changed=[uid(9), uid(10)]),
         "expect": {"entities": {"9": reshaped(hatch_made, HATCH_STRETCHED), "10": reshaped(dimension_made, DIMENSION_STRETCHED)}, "revision": "changed"}},
        {"op": "undo", "returns": "Esnet", "expect": {"entities": {"9": hatch_made, "10": dimension_made}}},
    ],
})

cases.append({
    "name": "taramanın ve deliklerinin en az 3 köşesi olmalı; ölçünün ve taramanın sayıları sonlu olmalı",
    "steps": [
        {"op": "execute", "input": {"operation": "stretch", "changes": [{"kind": "add", "from": uid(3), "geometry": {**HATCH, "ring": HATCH["ring"][:2]}}]},
         "result": failed("too_few_corners", "Taramanın en az 3 köşesi olmalı; 2 köşe verildi. Eksik köşeleri ekleyin.", "changes[0].geometry.ring"), "expect": NOTHING},
        {"op": "execute", "input": {"operation": "stretch", "changes": [{"kind": "add", "from": uid(3), "geometry": {**HATCH, "holes": [HATCH["holes"][0][:2]]}}]},
         "result": failed("too_few_corners", "1. deliğin en az 3 köşesi olmalı; 2 köşe verildi. Eksik köşeleri ekleyin ya da deliği çıkarın.", "changes[0].geometry.holes[0]"), "expect": NOTHING},
        {"op": "execute", "input": {"operation": "stretch", "changes": [{"kind": "add", "from": uid(1), "geometry": DIMENSION}]}, "nonFinite": {"changes[0].geometry.offset": "NaN"},
         "result": failed("not_finite", not_finite_message(1), "changes[0].geometry"), "expect": NOTHING},
        {"op": "execute", "input": {"operation": "stretch", "changes": [{"kind": "add", "from": uid(1), "geometry": DIMENSION}, {"kind": "add", "from": uid(3), "geometry": HATCH}]}, "nonFinite": {"changes[1].geometry.pattern.spacing": "Infinity"},
         "result": failed("not_finite", not_finite_message(2), "changes[1].geometry"), "expect": NOTHING},
        {"op": "execute", "input": {"operation": "stretch", "changes": [{"kind": "add", "from": uid(3), "geometry": HATCH}]}, "nonFinite": {"changes[0].geometry.ring[2].y": "-Infinity"},
         "result": failed("not_finite", not_finite_message(1), "changes[0].geometry"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "Buda, bir parça bile kalmazsa: nesne silinir",
    "steps": [
        {"op": "captureUid", "id": 1, "as": "cizgi"},
        {"op": "execute", "input": {"operation": "trim", "changes": [{"kind": "remove", "uid": uid(1)}]}, "result": done(removed=["$uid:cizgi"]),
         "expect": {"ids": ids_after(removed=[1]), "revision": "changed"}},
        {"op": "undo", "returns": "Buda", "expect": {"ids": IDS, "uids": {"1": "cizgi"}}},
    ],
})

cases.append({
    "name": "geometrisi aynı kalan update adım yazmaz; çıktı onu yine sayar",
    "steps": [
        {"op": "execute", "input": {"operation": "extend", "changes": [{"kind": "update", "uid": uid(4), "geometry": {"kind": "arc", "c": P(487060, 4420000), "r": 5, "a0": 0, "a1": HALF_PI}}]},
         "result": done(changed=[uid(4)]), "expect": NOTHING},
    ],
})

hidden = line(487000, 4420040, 487012, 4420040)
cases.append({
    "name": "gizli katmandaki nesne de düzenlenir; uyarı yok",
    "steps": [
        {"op": "execute", "input": {"operation": "lengthen", "changes": [{"kind": "update", "uid": uid(7), "geometry": hidden}]}, "result": done(changed=[uid(7)]),
         "expect": {"entities": {"7": updated(7, hidden)}, "revision": "changed"}},
    ],
})

# ── Refusals, in the contract's order ──────────────────────────────────

cases.append({
    "name": "değişiklik verilmedi: no_changes",
    "steps": [
        {"op": "execute", "input": {"operation": "trim", "changes": []}, "result": failed("no_changes", "Yapılacak değişiklik verilmedi. En az bir değişiklik verin.", "changes"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "kimlik küçük harfli, tireli bir UUID yazısıdır; ilk bozuk kimlik söylenir, add'de from",
    "steps": [
        {"op": "execute", "input": {"operation": "offset", "changes": [{"kind": "update", "uid": uid(1), "geometry": extended}, {"kind": "add", "from": MISSING.upper(), "geometry": copy}]},
         "result": failed("invalid_uid", UID_MESSAGE.format(MISSING.upper()), "changes[1].from"), "expect": NOTHING},
        {"op": "execute", "input": {"operation": "explode", "changes": [{"kind": "remove", "uid": "12"}]}, "result": failed("invalid_uid", UID_MESSAGE.format("12"), "changes[0].uid"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "çoklu çizginin en az 2 noktası, kapalı alanın ve deliğinin en az 3 köşesi olmalı",
    "steps": [
        {"op": "execute", "input": {"operation": "vertexRemove", "changes": [{"kind": "update", "uid": uid(2), "geometry": {"kind": "polyline", "pts": [P(487000, 4420010)]}}]},
         "result": failed("too_few_points", "Çoklu çizginin en az 2 noktası olmalı; 1 nokta verildi. Eksik noktaları ekleyin.", "changes[0].geometry.pts"), "expect": NOTHING},
        {"op": "execute", "input": {"operation": "vertexRemove", "changes": [{"kind": "update", "uid": uid(3), "geometry": {"kind": "polygon", "pts": [P(487030, 4420000), P(487050, 4420000)]}}]},
         "result": failed("too_few_corners", "Kapalı alanın en az 3 köşesi olmalı; 2 köşe verildi. Eksik köşeleri ekleyin.", "changes[0].geometry.pts"), "expect": NOTHING},
        {"op": "execute", "input": {"operation": "fillet", "changes": [{"kind": "update", "uid": uid(3), "geometry": {"kind": "polygon", "pts": rounded["pts"], "holes": [HOLE, {"pts": [P(487032, 4420015), P(487034, 4420015)]}]}}]},
         "result": failed("too_few_corners", "2. deliğin en az 3 köşesi olmalı; 2 köşe verildi. Eksik köşeleri ekleyin ya da deliği çıkarın.", "changes[0].geometry.holes[1].pts"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "NaN ya da sonsuz değer yazılmaz; ileti hangi değişikliğin olduğunu söyler",
    "steps": [
        {"op": "execute", "input": {"operation": "extend", "changes": [{"kind": "update", "uid": uid(1), "geometry": extended}]}, "nonFinite": {"changes[0].geometry.b.x": "NaN"},
         "result": failed("not_finite", not_finite_message(1), "changes[0].geometry"), "expect": NOTHING},
        {"op": "execute", "input": {"operation": "trim", "changes": [{"kind": "replace", "uid": uid(1), "geometry": left}, {"kind": "add", "from": uid(1), "geometry": bent}]}, "nonFinite": {"changes[1].geometry.pts[1].y": "Infinity"},
         "result": failed("not_finite", not_finite_message(2), "changes[1].geometry"), "expect": NOTHING},
        {"op": "execute", "input": {"operation": "lengthen", "changes": [{"kind": "update", "uid": uid(4), "geometry": longer}]}, "nonFinite": {"changes[0].geometry.a1": "-Infinity"},
         "result": failed("not_finite", not_finite_message(1), "changes[0].geometry"), "expect": NOTHING},
        {"op": "execute", "input": {"operation": "fillet", "changes": [{"kind": "update", "uid": uid(3), "geometry": rounded}]}, "nonFinite": {"changes[0].geometry.bulges[1]": "NaN"},
         "result": failed("not_finite", not_finite_message(1), "changes[0].geometry"), "note": "Yay değeri de geometrinin sayısıdır.", "expect": NOTHING},
    ],
})

cases.append({
    "name": "dairenin ve yayın yarıçapı sıfırdan büyük olmalı",
    "steps": [
        {"op": "execute", "input": {"operation": "offset", "changes": [{"kind": "add", "from": uid(8), "geometry": {"kind": "circle", "c": P(487070, 4420010), "r": 0}}]},
         "result": failed("invalid_radius", "Yarıçap sıfırdan büyük olmalı. Pozitif bir yarıçap verin.", "changes[0].geometry.r"), "expect": NOTHING},
        {"op": "execute", "input": {"operation": "break", "changes": [{"kind": "replace", "uid": uid(8), "geometry": {"kind": "arc", "c": P(487070, 4420010), "r": -3, "a0": 1, "a1": 5}}]},
         "result": failed("invalid_radius", "Yarıçap sıfırdan büyük olmalı. Pozitif bir yarıçap verin.", "changes[0].geometry.r"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "girdinin hatası çizimin durumundan önce gelir: geometri, sürümden ve nesneden önce denetlenir",
    "steps": [
        {"op": "execute", "input": {"operation": "vertexRemove", "changes": [{"kind": "update", "uid": MISSING, "geometry": extended}, {"kind": "update", "uid": uid(2), "geometry": {"kind": "polyline", "pts": [P(487000, 4420010)]}}], "expectedRevision": "999"},
         "result": failed("too_few_points", "Çoklu çizginin en az 2 noktası olmalı; 1 nokta verildi. Eksik noktaları ekleyin.", "changes[1].geometry.pts"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "beklenen sürüm ondalık bir tamsayı yazısıdır",
    "steps": [
        {"op": "execute", "input": {"operation": "extend", "changes": [{"kind": "update", "uid": uid(1), "geometry": extended}], "expectedRevision": "07"},
         "result": failed("invalid_revision", "Beklenen sürüm “07” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "çizim beklenen sürümde değilse hiçbir şey yazılmaz: conflict; çakışma nesneden ve kilitten önce",
    "steps": [
        {"op": "captureRevision", "as": "r0"},
        {"op": "execute", "input": {"operation": "lengthen", "changes": [{"kind": "update", "uid": uid(4), "geometry": longer}]}, "result": done(changed=[uid(4)])},
        {"op": "execute", "input": {"operation": "extend", "changes": [{"kind": "update", "uid": uid(1), "geometry": extended}], "expectedRevision": "$r0"},
         "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}},
         "expect": {"entities": {"1": E(1)}, "revision": "same"}},
        {"op": "execute", "input": {"operation": "trim", "changes": [{"kind": "remove", "uid": uid(5)}, {"kind": "remove", "uid": MISSING}], "expectedRevision": "$r0"},
         "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}},
         "expect": {"revision": "same"}},
    ],
})

cases.append({
    "name": "çizimde olmayan kimlik: entity_not_found; add'in kaynağı da; hiçbir şey yazılmaz",
    "steps": [
        {"op": "execute", "input": {"operation": "offset", "changes": [{"kind": "update", "uid": uid(1), "geometry": extended}, {"kind": "add", "from": MISSING, "geometry": copy}]},
         "result": failed("entity_not_found", MISSING_MESSAGE, "changes[1].from"), "expect": {**NOTHING, "entities": {"1": E(1)}}},
        {"op": "execute", "input": {"operation": "trim", "changes": [{"kind": "remove", "uid": MISSING}]}, "result": failed("entity_not_found", MISSING_MESSAGE, "changes[0].uid"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "bir nesne tek değişiklikle değişir: repeated_entity; add'ler aynı nesneden gelebilir",
    "steps": [
        {"op": "captureUid", "id": 1, "as": "cizgi"},
        {"op": "execute", "input": {"operation": "trim", "changes": [{"kind": "update", "uid": uid(1), "geometry": left}, {"kind": "remove", "uid": uid(1)}]},
         "result": failed("repeated_entity", "“$uid:cizgi” kimlikli nesne birden çok değişiklikte değişiyor. Bir nesneye tek değişiklik verin.", "changes[1].uid"), "expect": NOTHING},
        {"op": "execute", "input": {"operation": "break", "changes": [{"kind": "replace", "uid": uid(1), "geometry": left}, {"kind": "add", "from": uid(1), "geometry": right}, {"kind": "add", "from": uid(1), "geometry": copy}]},
         "result": done(changed=[uid(1)], created=[uid(9), uid(10)]),
         "expect": {"ids": ids_after(added=[9, 10]), "entities": {"9": inherited(1, right, False, 9), "10": inherited(1, copy, False, 10)}, "revision": "changed"}},
    ],
})

cases.append({
    "name": "kilitli katmandaki nesne düzenlenmez: düzenleme bütün yazılır ya da hiç",
    "note": "Parçalar birbirine aittir (budanan nesnenin kalanı, birleşen zincir): bir kısmı yazılmaz.",
    "steps": [
        {"op": "execute", "input": {"operation": "join", "changes": [{"kind": "replace", "uid": uid(1), "geometry": chain, "keepData": True}, {"kind": "remove", "uid": uid(5)}]},
         "result": failed("layer_locked", locked_message("Kilitli katman"), "changes[1].uid"), "expect": {**NOTHING, "entities": {"1": E(1), "5": E(5)}}},
    ],
})

cases.append({
    "name": "kilitli grubun katmanındaki nesneden de nesne yapılmaz",
    "steps": [
        {"op": "execute", "input": {"operation": "offset", "changes": [{"kind": "add", "from": uid(6), "geometry": line(487000, 4420036, 487010, 4420036)}]},
         "result": failed("layer_locked", locked_message("Eski"), "changes[0].from"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "doğrulama hiçbir şey yazmaz; plan yazılacak nesneleri gösterir (yeninin yuvası 0); yazma planın sürümüyle planı yazar",
    "steps": [
        {"op": "validate", "input": {"operation": "trim", "changes": [{"kind": "replace", "uid": uid(1), "geometry": left}, {"kind": "add", "from": uid(1), "geometry": right}]},
         "result": {"status": "completed", "output": None, "warnings": []}, "expect": NOTHING},
        {"op": "plan", "input": {"operation": "trim", "changes": [{"kind": "replace", "uid": uid(1), "geometry": left}, {"kind": "add", "from": uid(1), "geometry": right}]},
         "result": {"status": "completed", "output": {"changed": [inherited(1, left, False, 1)], "created": [inherited(1, right, False, 0)], "removed": [], "revision": "$current"}, "warnings": []}, "expect": NOTHING},
        {"op": "captureRevision", "as": "plan"},
        {"op": "execute", "input": {"operation": "trim", "changes": [{"kind": "replace", "uid": uid(1), "geometry": left}, {"kind": "add", "from": uid(1), "geometry": right}], "expectedRevision": "$plan"},
         "result": done(changed=[uid(1)], created=[uid(9)]), "expect": {"entities": {"1": inherited(1, left, False, 1), "9": inherited(1, right, False, 9)}, "revision": "changed"}},
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
    "cad.entities.edit",
    "Nesneleri düzenle: doğrulama, plan, yazma, geri alma",
    "ADR 0047. Denetim sırası: en az bir değişiklik; her değişikliğin kimliğinin yazımı (add'de from); her geometrinin nokta sayısı, sonlu sayıları ve yarıçapı; beklenen sürümün yazımı, sonra çizimin sürümü; her kimliğin çizimde olması; bir nesnenin tek değişiklikle değişmesi; hiçbir nesnenin kilitli katmanda olmaması (düzenleme bütün yazılır ya da hiç). update yalnız geometriyi değiştirir; replace nesneyi yerinde ve kimliğiyle başka bir nesne yapar, katmanı ve rengi kalır, öznitelikleri ve etiketi keepData ile kalır, simgesi gelmez; add bir nesneden yeni nesne yapar, onun katmanını ve rengini alır. Adım işlemin adıdır: Ötele, Buda, Uzat, Köşe yuvarla, Pah, Kır, Birleştir, Patlat, Uzat-kısalt, Köşe ekle, Köşe sil, Esnet. Kurulumdaki en büyük kimlik 8; yeni nesneler 9'dan başlar. $uidOf:N, N yuvasındaki nesnenin kalıcı kimliğidir.",
    cases,
)
print(f"{len(cases)} cases" + (" match" if "--check" in sys.argv[1:] else " written"))
