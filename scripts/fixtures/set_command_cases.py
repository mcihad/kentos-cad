"""The shared cases of the product command cad.entities.set: the layer, colour,
symbol, attributes and label of objects (Öznitelikler, Sembol ver, Sembolü kaldır).

    python3 scripts/fixtures/set_command_cases.py           # writes the file
    python3 scripts/fixtures/set_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.set.json. The checks, their order,
codes, paths and messages are written here by hand from the contract
(crates/shared/contracts/src/cad_properties.rs), and so is what an object
becomes (`after` below: the contract's rule, not an implementation's output).

`--check` rebuilds the file in memory and compares it with the one on disk.
"""
import json
import sys

style = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}


def layer(i, name, visible=True, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True, "style": style, "children": []}


def group(i, name, children, visible=True, locked=False):
    return {"id": i, "name": name, "type": "group", "visible": visible, "locked": locked, "expanded": True, "style": style, "children": children}


def P(x, y):
    return {"x": x, "y": y}


ENTITIES = [
    {"kind": "line", "id": 1, "layerId": "yapi", "color": "#E5484D", "attrs": {"Tür": "Duvar"}, "label": "D1", "symbol": "cit", "a": P(487000, 4420000), "b": P(487020, 4420000)},
    {"kind": "polygon", "id": 2, "layerId": "yapi", "attrs": {"Ada": "101", "Parsel": "7"}, "label": "7", "pts": [P(487030, 4420000), P(487050, 4420000), P(487050, 4420020), P(487030, 4420020)]},
    {"kind": "point", "id": 3, "layerId": "yapi", "attrs": {}, "p": P(487060, 4420000)},
    {"kind": "line", "id": 4, "layerId": "yol", "attrs": {}, "a": P(487000, 4420030), "b": P(487020, 4420030)},
    {"kind": "line", "id": 5, "layerId": "kilitli", "attrs": {}, "a": P(487000, 4420035), "b": P(487010, 4420035)},
    {"kind": "line", "id": 6, "layerId": "eski", "attrs": {}, "a": P(487000, 4420040), "b": P(487010, 4420040)},
    {"kind": "line", "id": 7, "layerId": "gizli", "attrs": {}, "a": P(487000, 4420045), "b": P(487010, 4420045)},
    {"kind": "circle", "id": 8, "layerId": "su", "color": "#3E63DD", "attrs": {}, "c": P(487070, 4420010), "r": 3},
]
BY_ID = {e["id"]: e for e in ENTITIES}
IDS = [e["id"] for e in ENTITIES]

SETUP = {
    "format": "kentos.document",
    "version": 1,
    "name": "Ürün komutu",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
    "origin": {"x": 487000, "y": 4420000},
    "layers": [
        layer("yapi", "Yapı"),
        layer("yol", "Yol"),
        layer("kilitli", "Kilitli katman", locked=True),
        layer("gizli", "Gizli katman", visible=False),
        group("altyapi", "Altyapı", [layer("su", "Su")]),
        group("arsiv", "Arşiv", [layer("eski", "Eski")], locked=True),
        group("sakli", "Saklı grup", [layer("icerde", "İçerideki")], visible=False),
    ],
    "activeLayer": "yapi",
    "entities": ENTITIES,
    "styles": {"items": [], "categories": []},
}

# ── What an object becomes (the contract) ──────────────────────────────

ABSENT = object()


def E(i):
    return json.loads(json.dumps(BY_ID[i]))


def after(i, layerId=ABSENT, color=ABSENT, symbol=ABSENT, attrs=None, label=ABSENT):
    """The object with the properties given: a layer moves it; a colour, a symbol
    or a label replaces its own, None takes it away; an attribute is set by its
    name, None removes it, the others stay. Nothing else of it changes."""
    e = E(i)
    if layerId is not ABSENT:
        e["layerId"] = layerId
    for key, value in (("color", color), ("symbol", symbol), ("label", label)):
        if value is ABSENT:
            continue
        if value is None:
            e.pop(key, None)
        else:
            e[key] = value
    for key, value in (attrs or {}).items():
        if value is None:
            e["attrs"].pop(key, None)
        else:
            e["attrs"][key] = value
    return e


def planned(e):
    """An object as the plan shows it: as it would be written, with its slot."""
    return e


def uid(i):
    return f"$uidOf:{i}"


def uids(*slots):
    return [uid(i) for i in slots]


def done(slots, warnings=()):
    return {"status": "completed", "output": {"changed": uids(*slots), "ids": list(slots), "revision": "$current"}, "warnings": list(warnings)}


def failed(code, message, path=None):
    error = {"code": code, "message": message}
    if path is not None:
        error["path"] = path
    return {"status": "failed", "error": error}


def hidden(name):
    return {"code": "layer_hidden", "message": f"“{name}” katmanı gizli; taşınan nesneler görünmeyecek.", "path": "layerId"}


def locked_object(name, i):
    return failed("layer_locked", f"“{name}” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın.", f"uids[{i}]")


def locked_target(name):
    return failed("layer_locked", f"“{name}” katmanı kilitli; nesneler ona taşınamaz. Kilidi Katmanlar panelinden açın ya da başka bir katman seçin.", "layerId")


def layer_not_found(i):
    return failed("layer_not_found", f"“{i}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId")


NOTHING = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
NO_ENTITIES = "Özellikleri değişecek nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin."
NOTHING_TO_SET = "Değişecek özellik verilmedi. Katman, renk, kalınlık, sembol, öznitelik ya da etiket verin."
INVALID_ATTRIBUTE = "Öznitelik adı boş olamaz; yalnız boşluktan oluşan ad da boştur. Özniteliğe bir ad verin."
UID_MESSAGE = "“{}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın."
MISSING = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"
MISSING_MESSAGE = f"“{MISSING}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."
GREEN = "#30A46C"
RED = "#E5484D"


def conflict():
    return {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}}


cases = []

# ── Writing ────────────────────────────────────────────────────────────

cases.append({
    "name": "Katman değiştir: nesneler başka katmana taşınır; yuvası, kalıcı kimliği ve öbür alanları kalır; tek adımda geri alınır, yinelenir",
    "steps": [
        {"op": "captureUid", "id": 1, "as": "duvar"},
        {"op": "captureUid", "id": 2, "as": "parsel"},
        {"op": "execute", "input": {"uids": uids(1, 2), "layerId": "yol", "operation": "layer"}, "result": done([1, 2]),
         "expect": {"ids": IDS, "entities": {"1": after(1, layerId="yol"), "2": after(2, layerId="yol")}, "uids": {"1": "duvar", "2": "parsel"}, "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}},
        {"op": "undo", "returns": "Katman değiştir", "note": "Öznitelikler'in Katman ▾ satırının adımı.", "expect": {"entities": {"1": E(1), "2": E(2)}, "canUndo": False, "canRedo": True}},
        {"op": "redo", "returns": "Katman değiştir", "expect": {"entities": {"1": after(1, layerId="yol"), "2": after(2, layerId="yol")}, "uids": {"1": "duvar", "2": "parsel"}}},
    ],
})

cases.append({
    "name": "zaten o katmandaki nesne değişmez ve çıktıda yoktur",
    "steps": [
        {"op": "execute", "input": {"uids": uids(4, 3), "layerId": "yol", "operation": "layer"}, "result": done([3]),
         "expect": {"entities": {"3": after(3, layerId="yol"), "4": E(4)}, "revision": "changed"}},
    ],
})

cases.append({
    "name": "grubun içindeki katmana da taşınır",
    "steps": [
        {"op": "execute", "input": {"uids": uids(3), "layerId": "su", "operation": "layer"}, "result": done([3]),
         "expect": {"entities": {"3": after(3, layerId="su")}, "revision": "changed"}},
    ],
})

cases.append({
    "name": "gizli katmana uyarıyla taşınır; gizli grubun katmanına da, uyarı katmanın adını verir",
    "steps": [
        {"op": "execute", "input": {"uids": uids(3), "layerId": "gizli", "operation": "layer"}, "result": done([3], [hidden("Gizli katman")]),
         "expect": {"entities": {"3": after(3, layerId="gizli")}, "revision": "changed"}},
        {"op": "execute", "input": {"uids": uids(3, 1), "layerId": "icerde", "operation": "layer"}, "result": done([3, 1], [hidden("İçerideki")]),
         "expect": {"entities": {"3": after(3, layerId="icerde"), "1": after(1, layerId="icerde")}, "revision": "changed"}},
    ],
})

cases.append({
    "name": "gizli katmana taşınan olmazsa uyarı da yoktur: gizli katmandaki nesne yerinde kalır, hiçbir şey yazılmaz",
    "steps": [
        {"op": "execute", "input": {"uids": uids(7), "layerId": "gizli", "operation": "layer"}, "result": done([]), "expect": NOTHING},
    ],
})

cases.append({
    "name": "Renk değiştir: renk verilir; null nesneyi katmanının rengine döndürür; her yazma kendi adımıdır",
    "steps": [
        {"op": "execute", "input": {"uids": uids(2, 3), "color": GREEN, "operation": "color"}, "result": done([2, 3]),
         "expect": {"entities": {"2": after(2, color=GREEN), "3": after(3, color=GREEN)}, "revision": "changed"}},
        {"op": "execute", "input": {"uids": uids(1), "color": None, "operation": "color"}, "result": done([1]),
         "expect": {"entities": {"1": after(1, color=None)}, "revision": "changed"}},
        {"op": "undo", "returns": "Renk değiştir", "expect": {"entities": {"1": E(1)}}},
        {"op": "undo", "returns": "Renk değiştir", "expect": {"entities": {"2": E(2), "3": E(3)}, "canUndo": False}},
    ],
})


def weighted(i, weight):
    """The object with its own line weight (docs/adr/0139), written after its other fields."""
    e = E(i)
    e["lineWeight"] = weight
    return e


WEIGHT = "Çizgi kalınlığı 0 ile 100 mm arasında bir sayı olmalı (0 en ince çizgidir). Bir kalınlık ya da “Katmana göre” seçin."

cases.append({
    "name": "Kalınlık değiştir: nesnenin kendi kalınlığı verilir (ADR 0139); aynısı değişiklik değildir; null nesneyi katmanının kalınlığına döndürür; her yazma kendi adımıdır",
    "steps": [
        {"op": "execute", "input": {"uids": uids(2, 3), "lineWeight": 0.35, "operation": "lineWeight"}, "result": done([2, 3]),
         "expect": {"entities": {"2": weighted(2, 0.35), "3": weighted(3, 0.35)}, "revision": "changed"}},
        {"op": "execute", "input": {"uids": uids(2), "lineWeight": 0.35, "operation": "lineWeight"}, "result": done([]), "expect": {"revision": "same"}},
        {"op": "execute", "input": {"uids": uids(3), "lineWeight": None, "operation": "lineWeight"}, "result": done([3]),
         "expect": {"entities": {"3": E(3)}, "revision": "changed"}},
        {"op": "undo", "returns": "Kalınlık değiştir", "note": "Öznitelikler'in Kalınlık ▾ satırının adımı.", "expect": {"entities": {"3": weighted(3, 0.35)}}},
        {"op": "undo", "returns": "Kalınlık değiştir", "expect": {"entities": {"2": E(2), "3": E(3)}, "canUndo": False}},
    ],
})

cases.append({
    "name": "kalınlık 0 ile 100 mm arasında bir sayı olmalı; NaN da değildir: invalid_line_weight (yolu lineWeight); hiçbir şey yazılmaz",
    "steps": [
        {"op": "execute", "input": {"uids": uids(1), "lineWeight": 100.5, "operation": "lineWeight"}, "result": failed("invalid_line_weight", WEIGHT, "lineWeight"), "expect": NOTHING},
        {"op": "execute", "input": {"uids": uids(1), "lineWeight": -0.25, "operation": "lineWeight"}, "result": failed("invalid_line_weight", WEIGHT, "lineWeight"), "expect": NOTHING},
        {"op": "execute", "input": {"uids": uids(1), "lineWeight": 0.35, "operation": "lineWeight"}, "nonFinite": {"lineWeight": "NaN"}, "result": failed("invalid_line_weight", WEIGHT, "lineWeight"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "Sembol ata ve Sembolü kaldır: sembol verilir, null katman stiline döndürür; kimliği kitaplıkta aranmaz",
    "steps": [
        {"op": "execute", "input": {"uids": uids(2, 3), "symbol": "agac", "operation": "symbol"}, "result": done([2, 3]),
         "expect": {"entities": {"2": after(2, symbol="agac"), "3": after(3, symbol="agac")}, "revision": "changed"}},
        {"op": "execute", "input": {"uids": uids(1), "symbol": None, "operation": "symbol"}, "result": done([1]),
         "expect": {"entities": {"1": after(1, symbol=None)}, "revision": "changed"}},
        {"op": "undo", "returns": "Sembolü kaldır", "expect": {"entities": {"1": E(1)}}},
        {"op": "undo", "returns": "Sembol ata", "expect": {"entities": {"2": E(2), "3": E(3)}, "canUndo": False}},
    ],
})

cases.append({
    "name": "Değiştir (öznitelik): değer değişir, yeni ad eklenir, null adı siler; olmayan adı silmek değişiklik değildir; adı geçmeyenler kalır",
    "steps": [
        {"op": "execute", "input": {"uids": uids(1), "attrs": {"Tür": "Çit", "Yükseklik": "2", "Yok": None}, "operation": "attributes"}, "result": done([1]),
         "expect": {"entities": {"1": after(1, attrs={"Tür": "Çit", "Yükseklik": "2"})}, "revision": "changed"}},
        {"op": "undo", "returns": "Değiştir", "note": "Öznitelikler'in öznitelik satırları komuttan önce de böyle yazıyordu.", "expect": {"entities": {"1": E(1)}, "canUndo": False}},
    ],
})

cases.append({
    "name": "öznitelik ve onu gösteren etiket birlikte (Parsel): tek adım, adı “Değiştir”",
    "steps": [
        {"op": "execute", "input": {"uids": uids(2), "attrs": {"Parsel": "8"}, "label": "8", "operation": "attributes"}, "result": done([2]),
         "expect": {"entities": {"2": after(2, attrs={"Parsel": "8"}, label="8")}, "revision": "changed"}},
        {"op": "undo", "returns": "Değiştir", "expect": {"entities": {"2": E(2)}, "canUndo": False}},
    ],
})

cases.append({
    "name": "öznitelik silmek: yalnız o adı taşıyan nesne değişir",
    "steps": [
        {"op": "execute", "input": {"uids": uids(2, 3), "attrs": {"Ada": None}, "operation": "attributes"}, "result": done([2]),
         "expect": {"entities": {"2": after(2, attrs={"Ada": None}), "3": E(3)}, "revision": "changed"}},
    ],
})

cases.append({
    "name": "Etiket değiştir: etiket verilir; null kaldırır",
    "steps": [
        {"op": "execute", "input": {"uids": uids(3), "label": "N1", "operation": "label"}, "result": done([3]),
         "expect": {"entities": {"3": after(3, label="N1")}, "revision": "changed"}},
        {"op": "execute", "input": {"uids": uids(1), "label": None, "operation": "label"}, "result": done([1]),
         "expect": {"entities": {"1": after(1, label=None)}, "revision": "changed"}},
        {"op": "undo", "returns": "Etiket değiştir"},
        {"op": "undo", "returns": "Etiket değiştir", "expect": {"entities": {"1": E(1), "3": E(3)}, "canUndo": False}},
    ],
})

cases.append({
    "name": "birden çok özellik tek adımda; adım işlemin adıdır, işlem neyin değişeceğini sınırlamaz",
    "steps": [
        {"op": "execute", "input": {"uids": uids(3), "layerId": "yol", "color": RED, "symbol": "agac", "attrs": {"Ad": "N1"}, "label": "N1", "operation": "layer"}, "result": done([3]),
         "expect": {"entities": {"3": after(3, layerId="yol", color=RED, symbol="agac", attrs={"Ad": "N1"}, label="N1")}, "revision": "changed"}},
        {"op": "undo", "returns": "Katman değiştir", "expect": {"entities": {"3": E(3)}, "canUndo": False}},
    ],
})

cases.append({
    "name": "aynı kimlik iki kez verilirse nesne bir kez değişir, çıktıda bir kez geçer",
    "steps": [
        {"op": "execute", "input": {"uids": uids(3, 3), "color": RED, "operation": "color"}, "result": done([3]),
         "expect": {"entities": {"3": after(3, color=RED)}, "revision": "changed"}},
    ],
})

cases.append({
    "name": "gizli katmandaki nesnenin özellikleri de değişir; uyarı yok",
    "steps": [
        {"op": "execute", "input": {"uids": uids(7), "color": RED, "operation": "color"}, "result": done([7]),
         "expect": {"entities": {"7": after(7, color=RED)}, "revision": "changed"}},
    ],
})

cases.append({
    "name": "hiçbir nesne değişmezse hiçbir şey yazılmaz: aynı katman, renk, sembol, öznitelik ve etiket; olmayanı kaldırmak da değişiklik değildir",
    "steps": [
        {"op": "execute", "input": {"uids": uids(1), "layerId": "yapi", "operation": "layer"}, "result": done([]), "expect": NOTHING},
        {"op": "execute", "input": {"uids": uids(8), "color": "#3E63DD", "operation": "color"}, "result": done([]), "expect": NOTHING},
        {"op": "execute", "input": {"uids": uids(2, 3), "color": None, "operation": "color"}, "result": done([]), "expect": NOTHING},
        {"op": "execute", "input": {"uids": uids(1), "symbol": "cit", "operation": "symbol"}, "result": done([]), "expect": NOTHING},
        {"op": "execute", "input": {"uids": uids(3), "symbol": None, "operation": "symbol"}, "result": done([]), "expect": NOTHING},
        {"op": "execute", "input": {"uids": uids(1), "attrs": {"Tür": "Duvar", "Yok": None}, "operation": "attributes"}, "result": done([]), "expect": NOTHING},
        {"op": "execute", "input": {"uids": uids(2), "label": "7", "operation": "label"}, "result": done([]), "expect": NOTHING},
    ],
})

# ── Refusals, in the contract's order ──────────────────────────────────

cases.append({
    "name": "nesne verilmedi: no_entities; her şeyden önce",
    "steps": [
        {"op": "execute", "input": {"uids": [], "color": RED, "operation": "color"}, "result": failed("no_entities", NO_ENTITIES, "uids"), "expect": NOTHING},
        {"op": "execute", "input": {"uids": [], "operation": "layer", "layerId": "kilitli"}, "result": failed("no_entities", NO_ENTITIES, "uids"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "kimlik küçük harfli, tireli bir UUID yazısıdır; ilk bozuk kimlik söylenir",
    "steps": [
        {"op": "execute", "input": {"uids": [uid(1), "12"], "color": RED, "operation": "color"}, "result": failed("invalid_uid", UID_MESSAGE.format("12"), "uids[1]"), "expect": NOTHING},
        {"op": "execute", "input": {"uids": [MISSING.upper()], "color": RED, "operation": "color"}, "result": failed("invalid_uid", UID_MESSAGE.format(MISSING.upper()), "uids[0]"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "değişecek özellik verilmedi: nothing_to_set (yolu yok); boş öznitelik tablosu da bir şey vermez",
    "steps": [
        {"op": "execute", "input": {"uids": uids(1), "operation": "color"}, "result": failed("nothing_to_set", NOTHING_TO_SET), "expect": NOTHING},
        {"op": "execute", "input": {"uids": uids(1), "attrs": {}, "operation": "attributes"}, "result": failed("nothing_to_set", NOTHING_TO_SET), "expect": NOTHING},
    ],
})

cases.append({
    "name": "öznitelik adı boş olamaz: invalid_attribute; yalnız boşluk da boştur (Unicode White_Space)",
    "steps": [
        {"op": "execute", "input": {"uids": uids(1), "attrs": {"": "x"}, "operation": "attributes"}, "result": failed("invalid_attribute", INVALID_ATTRIBUTE, "attrs"), "expect": NOTHING},
        {"op": "execute", "input": {"uids": uids(1), "attrs": {"Ad": "y", " \t 　": "x"}, "operation": "attributes"}, "result": failed("invalid_attribute", INVALID_ATTRIBUTE, "attrs"), "expect": NOTHING},
        {"op": "execute", "input": {"uids": uids(1), "attrs": {"\u0085": None}, "operation": "attributes"}, "result": failed("invalid_attribute", INVALID_ATTRIBUTE, "attrs"),
         "note": "U+0085 de boşluktur (JavaScript'in trim'i onu almaz; komut Unicode'unkini kullanır).", "expect": NOTHING},
    ],
})

cases.append({
    "name": "girdinin hatası çizimin durumundan önce gelir: öznitelik adı ve verilmeyen özellik, sürümden, nesneden ve katmandan önce denetlenir",
    "steps": [
        {"op": "execute", "input": {"uids": [MISSING], "layerId": "yok", "attrs": {"": "x"}, "operation": "attributes", "expectedRevision": "999"},
         "result": failed("invalid_attribute", INVALID_ATTRIBUTE, "attrs"), "expect": NOTHING},
        {"op": "execute", "input": {"uids": [MISSING], "operation": "label", "expectedRevision": "999"}, "result": failed("nothing_to_set", NOTHING_TO_SET), "expect": NOTHING},
    ],
})

cases.append({
    "name": "beklenen sürüm ondalık bir tamsayı yazısıdır",
    "steps": [
        {"op": "execute", "input": {"uids": uids(1), "color": RED, "operation": "color", "expectedRevision": "07"},
         "result": failed("invalid_revision", "Beklenen sürüm “07” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "çizim beklenen sürümde değilse hiçbir şey yazılmaz: conflict; çakışma nesneden, katmandan ve kilitten önce",
    "steps": [
        {"op": "captureRevision", "as": "r0"},
        {"op": "execute", "input": {"uids": uids(3), "color": RED, "operation": "color"}, "result": done([3])},
        {"op": "execute", "input": {"uids": uids(1), "color": GREEN, "operation": "color", "expectedRevision": "$r0"}, "result": conflict(),
         "expect": {"entities": {"1": E(1)}, "revision": "same"}},
        {"op": "execute", "input": {"uids": [uid(5), MISSING], "layerId": "yok", "operation": "layer", "expectedRevision": "$r0"}, "result": conflict(), "expect": {"revision": "same"}},
        {"op": "execute", "input": {"uids": uids(1), "color": GREEN, "operation": "color", "expectedRevision": "$current"}, "result": done([1]),
         "note": "Şimdiki sürümle hazırlanan girdi yazılır.", "expect": {"entities": {"1": after(1, color=GREEN)}, "revision": "changed"}},
    ],
})

cases.append({
    "name": "çizimde olmayan kimlik: entity_not_found; hiçbir şey yazılmaz",
    "steps": [
        {"op": "execute", "input": {"uids": [uid(1), MISSING], "color": GREEN, "operation": "color"}, "result": failed("entity_not_found", MISSING_MESSAGE, "uids[1]"),
         "expect": {**NOTHING, "entities": {"1": E(1)}}},
    ],
})

cases.append({
    "name": "bilinmeyen katman: layer_not_found; ad kimlik değildir",
    "steps": [
        {"op": "execute", "input": {"uids": uids(3), "layerId": "Yol", "operation": "layer"}, "result": layer_not_found("Yol"), "expect": NOTHING},
        {"op": "execute", "input": {"uids": uids(3), "layerId": "", "operation": "layer"}, "result": layer_not_found(""), "expect": NOTHING},
    ],
})

cases.append({
    "name": "grup katman değildir: not_a_layer",
    "steps": [
        {"op": "execute", "input": {"uids": uids(3), "layerId": "altyapi", "operation": "layer"},
         "result": failed("not_a_layer", "“Altyapı” bir katman grubu; nesneler yalnız bir katmana taşınır. Grubun içinden bir katman seçin.", "layerId"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "kilitli katmandaki nesne değişmez: layer_locked; biri kilitliyse hiçbiri yazılmaz; kilitli grubun katmanı da kilitlidir",
    "steps": [
        {"op": "execute", "input": {"uids": uids(1, 5), "color": GREEN, "operation": "color"}, "result": locked_object("Kilitli katman", 1),
         "expect": {**NOTHING, "entities": {"1": E(1), "5": E(5)}}},
        {"op": "execute", "input": {"uids": uids(6), "label": "E1", "operation": "label"}, "result": locked_object("Eski", 0), "expect": NOTHING},
    ],
})

cases.append({
    "name": "kilitli katmana nesne taşınmaz: layer_locked; kilitli grubun katmanına da",
    "steps": [
        {"op": "execute", "input": {"uids": uids(3), "layerId": "kilitli", "operation": "layer"}, "result": locked_target("Kilitli katman"), "expect": NOTHING},
        {"op": "execute", "input": {"uids": uids(1, 3), "layerId": "eski", "operation": "layer"}, "result": locked_target("Eski"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "katman sırası: hedefin varlığı nesnenin kilidinden, nesnenin kilidi hedefin kilidinden önce",
    "steps": [
        {"op": "execute", "input": {"uids": uids(5), "layerId": "yok", "operation": "layer"}, "result": layer_not_found("yok"), "expect": NOTHING},
        {"op": "execute", "input": {"uids": uids(6), "layerId": "kilitli", "operation": "layer"}, "result": locked_object("Eski", 0), "expect": NOTHING},
    ],
})

cases.append({
    "name": "doğrulama hiçbir şey yazmaz; plan değişecek nesneleri yuvalarıyla gösterir; yazma planın sürümüyle planı yazar; uyarı üç kipte de",
    "steps": [
        {"op": "validate", "input": {"uids": uids(3, 4), "layerId": "gizli", "operation": "layer"},
         "result": {"status": "completed", "output": None, "warnings": [hidden("Gizli katman")]}, "expect": NOTHING},
        {"op": "plan", "input": {"uids": uids(3, 4), "layerId": "gizli", "operation": "layer"},
         "result": {"status": "completed", "output": {"changed": [planned(after(3, layerId="gizli")), planned(after(4, layerId="gizli"))], "revision": "$current"}, "warnings": [hidden("Gizli katman")]},
         "expect": NOTHING},
        {"op": "captureRevision", "as": "plan"},
        {"op": "execute", "input": {"uids": uids(3, 4), "layerId": "gizli", "operation": "layer", "expectedRevision": "$plan"}, "result": done([3, 4], [hidden("Gizli katman")]),
         "expect": {"entities": {"3": after(3, layerId="gizli"), "4": after(4, layerId="gizli")}, "revision": "changed"}},
    ],
})

cases.append({
    "name": "değişmeyecek nesnenin planı boştur; doğrulama ve plan geçmişe dokunmaz",
    "steps": [
        {"op": "execute", "input": {"uids": uids(3), "label": "N1", "operation": "label"}, "result": done([3])},
        {"op": "undo", "returns": "Etiket değiştir", "expect": {"canUndo": False, "canRedo": True}},
        {"op": "validate", "input": {"uids": uids(4), "layerId": "yol", "operation": "layer"}, "result": {"status": "completed", "output": None, "warnings": []}},
        {"op": "plan", "input": {"uids": uids(4), "layerId": "yol", "operation": "layer"}, "result": {"status": "completed", "output": {"changed": [], "revision": "$current"}, "warnings": []},
         "expect": {"canUndo": False, "canRedo": True, "revision": "same"}},
        {"op": "redo", "returns": "Etiket değiştir", "expect": {"entities": {"3": after(3, label="N1")}}},
    ],
})

# White space other than the plain space, escaped so a reader sees it.
INVISIBLE = "\u0085                 　﻿"


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
    "cad.entities.set",
    "Nesnelerin özelliklerini değiştir: doğrulama, plan, yazma, geri alma",
    "Denetim sırası: en az bir kimlik; her kimliğin yazımı; bir özellik verilmesi (katman, renk, sembol, öznitelik ya da etiket); öznitelik adlarının boş olmaması (yalnız boşluk da boştur, Unicode White_Space); beklenen sürümün yazımı, sonra çizimin sürümü; her kimliğin çizimde olması; verilen katmanın var ve grup değil olması; nesnelerin katmanının, sonra verilen katmanın kilitli olmaması (biri kilitliyse hiçbir şey yazılmaz). Verilmeyen özellik değişmez; renk, sembol ve etiket null ile kaldırılır; öznitelik adıyla yazılır, null ile silinir, adı geçmeyenler kalır. Zaten istendiği gibi olan nesne değişmez ve çıktıda yoktur; hiçbiri değişmezse adım yazılmaz. Gizli katmana taşınan olursa layer_hidden uyarısı. Adım işlemin adıdır: Katman değiştir, Renk değiştir, Sembol ata (sembol null ise Sembolü kaldır), Değiştir (öznitelik), Etiket değiştir. Kurulumdaki en büyük kimlik 8. $uidOf:N, N yuvasındaki nesnenin kalıcı kimliğidir.",
    cases,
)
print(f"{len(cases)} cases" + (" match" if "--check" in sys.argv[1:] else " written"))
