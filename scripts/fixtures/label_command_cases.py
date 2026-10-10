"""Writes fixtures/commands/v1/cad.layers.labels.json and cad.labels.pin.json: the shared cases of the label engine's
commands (docs/adr/0212 §5), their expectations built here from the contract's rules (crates/shared/contracts/src/
cad_labels.rs and labels.rs: the checks and their order, the codes, the paths and the words; the steps' names), without
KentOS code. The web (apps/web/src/product/fixtures.test.ts), the desktop (crates/native/application/tests/all/
fixtures.rs) and Python (python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/label_command_cases.py           # writes the files
    python3 scripts/fixtures/label_command_cases.py --check   # compares them with the ones on disk
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"

STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
OSM = {"kind": "xyz", "url": "https://tile.openstreetmap.org/{z}/{x}/{y}.png", "maxZoom": 19}
PARSEL_LABEL = {"placement": "center", "size": 10}
RULES = {"mode": "rules", "classes": [
    {"name": "No", "style": {"placement": "center", "size": 11, "weight": 600, "area": "parcel", "priority": 7}},
    {"name": "Malik", "when": "Malik <> ''", "style": {"placement": "center", "size": 9, "italic": True, "text": "Malik", "area": "free"}},
]}


def node(id_, name, type_="layer", children=None, style=None, **extra):
    return {"id": id_, "name": name, "type": type_, "visible": True, "locked": False, "expanded": True,
            "style": dict(style or STYLE), "children": children or [], **extra}


SETUP_LAYERS = [
    node("parsel", "Parsel", style={**STYLE, "label": dict(PARSEL_LABEL)}),
    node("yol", "Yol"),
    node("kurallar", "Kurallar", style={**STYLE, "labels": copy.deepcopy(RULES)}),
    node("koruma", "Koruma alanı", "group", [node("kilitli", "Sit alanı")], locked=True),
    node("altlik", "OpenStreetMap", service=dict(OSM)),
    node("0", "0"),
]


def rect(x, y, w, h):
    return [{"x": x, "y": y}, {"x": x + w, "y": y}, {"x": x + w, "y": y + h}, {"x": x, "y": y + h}]


X, Y = 500000, 4400000
ENTITIES = [
    {"kind": "polygon", "id": 1, "layerId": "parsel", "attrs": {"Parsel": "12"}, "label": "12", "pts": rect(X, Y, 40, 30)},
    {"kind": "polyline", "id": 2, "layerId": "yol", "attrs": {}, "label": "Ankara Yolu", "pts": [{"x": X, "y": Y + 40}, {"x": X + 90, "y": Y + 45}]},
    {"kind": "polygon", "id": 3, "layerId": "kurallar", "attrs": {"Parsel": "7", "Malik": "Ayşe Demir"}, "pts": rect(X + 40, Y, 20, 20)},
    {"kind": "point", "id": 4, "layerId": "kilitli", "attrs": {}, "label": "P.4", "p": {"x": X + 10, "y": Y + 50}},
    {"kind": "polygon", "id": 5, "layerId": "parsel", "attrs": {}, "label": "13", "pts": rect(X + 60, Y, 30, 30),
     "labelPins": [{"at": {"x": 1.0, "y": 2.0}}]},
]
IDS = [e["id"] for e in ENTITIES]
UID = {e["id"]: f"$uidOf:{e['id']}" for e in ENTITIES}


def setup():
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Etiketler",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                     "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": X, "y": Y},
        "layers": copy.deepcopy(SETUP_LAYERS),
        "activeLayer": "parsel",
        "entities": copy.deepcopy(ENTITIES),
        "styles": {"items": [], "categories": []},
    }


def tree():
    return copy.deepcopy(SETUP_LAYERS)


def get(nodes, id_):
    for n in nodes:
        if n["id"] == id_:
            return n
        hit = get(n["children"], id_)
        if hit:
            return hit
    return None


def styled(layers, id_, **fields):
    """The tree with a layer's style's fields set (a value) or taken away (None)."""
    out = copy.deepcopy(layers)
    n = get(out, id_)
    for k, v in fields.items():
        if v is None:
            n["style"].pop(k, None)
        else:
            n["style"][k] = copy.deepcopy(v)
    return out


def entity(id_, pins):
    e = copy.deepcopy(next(x for x in ENTITIES if x["id"] == id_))
    e.pop("labelPins", None)
    if pins:
        e["labelPins"] = copy.deepcopy(pins)
    return e


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
CONFLICTED = {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}}


def refused(name, input_, code, message, path):
    return {"name": name, "steps": [step("execute", input_, failed(code, message, path), {**UNCHANGED, "layers": tree()})]}


# ── cad.layers.labels ───────────────────────────────────────────────────

LABELS_STEP = "Etiketler"


def labelled(layer, changed=True):
    return completed({"layer": layer, "changed": changed, "revision": "$current"})


def layers_labels_cases():
    out = []
    curved = {"placement": "along", "size": 11, "line": "curved", "mergeLines": True, "repeat": 600.0, "duplicates": 250.0, "priority": 6}
    with_curved = styled(tree(), "yol", label=curved)
    out.append({
        "name": "Tek etiketin stili yazılır; adımı “Etiketler”, geri alınır ve yinelenir; nesneler değişmez",
        "steps": [
            step("execute", {"layer": "yol", "label": curved}, labelled("yol"),
                 {"ids": IDS, "entities": {"2": ENTITIES[1]}, "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed", "layers": with_curved}),
            step("undo", returns=LABELS_STEP, expect={"layers": tree(), "canUndo": False, "canRedo": True}),
            step("redo", returns=LABELS_STEP, expect={"layers": with_curved, "canUndo": True, "canRedo": False}),
        ],
    })
    rules = {"mode": "rules", "classes": [
        {"name": "No", "style": {"placement": "center", "size": 11, "weight": 600, "text": "Parsel", "area": "parcel",
                                 "stack": {"mode": "ifNeeded", "chars": 6}, "outside": True, "callout": {"kind": "straight"}, "priority": 7}},
        {"name": "Malik", "when": "Malik <> ''", "style": {"placement": "center", "size": 9, "italic": True, "text": "Malik", "area": "free", "inside": True,
                                                         "abbreviate": {"words": [{"word": "Arazisi", "short": "Ar."}]}}},
    ], "obstacle": {"weight": 3, "kind": "boundary"}}
    out.append({
        "name": "Kurallı: sınıflar ve engel; tek etiketin stili kalır",
        "steps": [step("execute", {"layer": "parsel", "labels": rules}, labelled("parsel"), {"layers": styled(tree(), "parsel", labels=rules), "canUndo": True})],
    })
    off = {"mode": "off", "obstacle": {"weight": 7}}
    out.append({
        "name": "Yalnız engel: etiketsiz, nesneleri öbür etiketlere engel",
        "steps": [step("execute", {"layer": "0", "labels": off}, labelled("0"), {"layers": styled(tree(), "0", labels=off)})],
    })
    out.append({
        "name": "label null: türün varsayılan etiketine döner; geri almada stil döner",
        "steps": [
            step("execute", {"layer": "parsel", "label": None}, labelled("parsel"), {"layers": styled(tree(), "parsel", label=None), "revision": "changed"}),
            step("undo", returns=LABELS_STEP, expect={"layers": tree()}),
        ],
    })
    out.append({
        "name": "labels null: kurallı katman tek etikete döner",
        "steps": [step("execute", {"layer": "kurallar", "labels": None}, labelled("kurallar"), {"layers": styled(tree(), "kurallar", labels=None)})],
    })
    out.append({
        "name": "İkisi birden, tek adım",
        "steps": [
            step("execute", {"layer": "yol", "label": curved, "labels": off}, labelled("yol"), {"layers": styled(tree(), "yol", label=curved, labels=off)}),
            step("undo", returns=LABELS_STEP, expect={"layers": tree(), "canUndo": False}),
        ],
    })
    out.append({
        "name": "Aynısı: bir şey yazılmaz, changed false",
        "steps": [step("execute", {"layer": "parsel", "label": PARSEL_LABEL}, labelled("parsel", False), {**UNCHANGED, "layers": tree()})],
    })
    out.append({
        "name": "Hiçbiri verilmezse değişmez",
        "steps": [step("execute", {"layer": "parsel"}, labelled("parsel", False), {**UNCHANGED, "layers": tree()})],
    })
    out.append({
        "name": "Kilitli katman: etiketleme katmanın ayarıdır, nesnelere dokunmaz; yazılır",
        "steps": [step("execute", {"layer": "kilitli", "label": {"placement": "beside", "size": 10, "point": "around"}}, labelled("kilitli"),
                       {"layers": styled(tree(), "kilitli", label={"placement": "beside", "size": 10, "point": "around"})})],
    })
    out.append({
        "name": "Servis katmanında null reddedilmez: etiketi yok",
        "steps": [step("execute", {"layer": "altlik", "label": None}, labelled("altlik", False), UNCHANGED)],
    })
    out.append({
        "name": "Doğrula: yazmaz",
        "steps": [step("validate", {"layer": "yol", "label": curved}, {"status": "completed", "output": None, "warnings": []}, {**UNCHANGED, "layers": tree()})],
    })
    out.append({
        "name": "Plan: katmanın yazılacak hâli; yazmaz",
        "steps": [step("plan", {"layer": "yol", "label": curved}, completed({"node": get(with_curved, "yol"), "changed": True, "revision": "$current"}),
                       {**UNCHANGED, "layers": tree()})],
    })
    # The refusals, in the contract's order: the style's and the labelling's rules, their expressions, the revision and the layer.
    for name, input_, message, path in (
        ("boy 0", {"layer": "yol", "label": {"placement": "center", "size": 0}}, "Etiketin stili: boy 1–200 arasında olmalı.", "label"),
        ("küçültme 0,3", {"layer": "yol", "label": {"placement": "center", "size": 10, "shrink": 0.3}}, "Etiketin stili: küçültme 0.5–1 arasında olmalı.", "label"),
        ("öncelik 11", {"layer": "yol", "label": {"placement": "center", "size": 10, "priority": 11}}, "Etiketin stili: öncelik 11: 0–10 arasında olmalı.", "label"),
        ("renk değil", {"layer": "yol", "label": {"placement": "center", "size": 10, "color": "mavi"}},
         "Etiketin stili: yazının rengi “mavi” renk değil (#rrggbb ya da fg, fg-dim, label, ink, paper).", "label"),
        ("yığmanın satırı 1 harf", {"layer": "yol", "label": {"placement": "center", "size": 10, "stack": {"mode": "ifNeeded", "chars": 1}}},
         "Etiketin stili: yığmanın satırı 1 harf: 2–500 olmalı.", "label"),
        ("kurallısı sınıfsız", {"layer": "yol", "labels": {"mode": "rules", "classes": []}}, "Etiketleme: kurallı etiketlemede 1–64 sınıf olmalı.", "labels"),
        ("tekinde sınıf", {"layer": "yol", "labels": {"mode": "single", "classes": [{"name": "A", "style": {"placement": "center", "size": 10}}]}},
         "Etiketleme: sınıflar yalnız kurallı etiketlemede olur.", "labels"),
        ("aynı adlı iki sınıf", {"layer": "yol", "labels": {"mode": "rules", "classes": [{"name": "No", "style": {"placement": "center", "size": 10}},
                                                                                       {"name": "No", "style": {"placement": "center", "size": 9}}]}},
         "Etiketleme: “No” adlı sınıf iki kez var.", "labels"),
        ("sınıfın boyu", {"layer": "yol", "labels": {"mode": "rules", "classes": [{"name": "No", "style": {"placement": "center", "size": 300}}]}},
         "Etiketleme: “No” sınıfı: boy 1–200 arasında olmalı.", "labels"),
        ("engelin ağırlığı 11", {"layer": "yol", "labels": {"mode": "off", "obstacle": {"weight": 11}}}, "Etiketleme: engelin ağırlığı 11: 1–10 olmalı.", "labels"),
    ):
        out.append(refused(f"bozuk etiketleme: {name}", input_, "invalid_labels", message, path))
    out.append(refused("$sıra etiketin metninde", {"layer": "yol", "label": {"placement": "center", "size": 10, "text": "1 + $sıra"}}, "invalid_expression",
                       "Etiketin metni: Etikette $sıra kullanılamaz.", "label/text"))
    out.append(refused("$ölçek sınıfın koşulunda", {"layer": "yol", "labels": {"mode": "rules", "classes": [
        {"name": "No", "when": "500 < $ölçek", "style": {"placement": "center", "size": 10}}]}}, "invalid_expression",
                       "“No” sınıfının koşulu: Etikette $ölçek kullanılamaz.", "labels/classes/0/when"))
    out.append(refused("$sıra sınıfın metninde", {"layer": "yol", "labels": {"mode": "rules", "classes": [
        {"name": "A", "style": {"placement": "center", "size": 10}}, {"name": "B", "style": {"placement": "center", "size": 10, "text": "1 + $sıra"}}]}},
                       "invalid_expression", "“B” sınıfının metni: Etikette $sıra kullanılamaz.", "labels/classes/1/style/text"))
    out.append(refused("bozuk etiketleme sürümden ve katmandan önce", {"layer": "yok", "label": {"placement": "center", "size": 0}, "expectedRevision": "on iki"},
                       "invalid_labels", "Etiketin stili: boy 1–200 arasında olmalı.", "label"))
    out.append(refused("geçersiz beklenen sürüm", {"layer": "yol", "label": curved, "expectedRevision": "on iki"}, "invalid_revision", INVALID_REVISION, "expectedRevision"))
    out.append({
        "name": "sürüm çakışması: plan edilen sürümden sonra çizim değişmişse hiçbir şey yazılmaz",
        "steps": [
            step("captureRevision", **{"as": "once"}),
            step("execute", {"layer": "0", "labels": off}, labelled("0"), {"revision": "changed"}),
            step("execute", {"layer": "yol", "label": curved, "expectedRevision": "$once"}, CONFLICTED,
                 {"revision": "same", "layers": styled(tree(), "0", labels=off)}),
        ],
    })
    out.append(refused("çizimde olmayan katman", {"layer": "yok", "label": curved}, "layer_not_found",
                       "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layer"))
    out.append(refused("grup", {"layer": "koruma", "label": curved}, "not_a_layer",
                       "“Koruma alanı” bir katman grubu; etiketleme yalnız katmanın olur. Grubun bir katmanını verin.", "layer"))
    out.append(refused("servis katmanı", {"layer": "altlik", "label": curved}, "service_layer",
                       "“OpenStreetMap” servisten çizilir ve nesne tutmaz; etiketleme nesneleri olan katmanın olur.", "layer"))
    return out


# ── cad.labels.pin ──────────────────────────────────────────────────────

PIN_STEP = "Etiket"


def pinned(changed):
    return completed({"changed": changed, "revision": "$current"})


def labels_pin_cases():
    out = []
    moved = {"at": {"x": -3.0, "y": 4.0}, "rotation": 15.0}
    out.append({
        "name": "Taşı ve döndür: tek etiket; adımı “Etiket”, geri alınır ve yinelenir",
        "steps": [
            step("execute", {"pins": [{"uid": UID[1], "pin": moved}]}, pinned(1),
                 {"ids": IDS, "entities": {"1": entity(1, [moved])}, "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed", "layers": tree()}),
            step("undo", returns=PIN_STEP, expect={"entities": {"1": entity(1, [])}, "canUndo": False, "canRedo": True}),
            step("redo", returns=PIN_STEP, expect={"entities": {"1": entity(1, [moved])}, "canUndo": True, "canRedo": False}),
        ],
    })
    out.append({
        "name": "Gizle: yer vermeden",
        "steps": [step("execute", {"pins": [{"uid": UID[2], "pin": {"hidden": True}}]}, pinned(1), {"entities": {"2": entity(2, [{"hidden": True}])}})],
    })
    out.append({
        "name": "Kurallı katman: iki sınıfın etiketi, tek nesne, tek adım",
        "steps": [
            step("execute", {"pins": [{"uid": UID[3], "class": "No", "pin": {"at": {"x": 0.0, "y": 5.0}}},
                                      {"uid": UID[3], "class": "Malik", "pin": {"hidden": True}}]}, pinned(1),
                 {"entities": {"3": entity(3, [{"class": "No", "at": {"x": 0.0, "y": 5.0}}, {"class": "Malik", "hidden": True}])}, "canUndo": True}),
            step("undo", returns=PIN_STEP, expect={"entities": {"3": entity(3, [])}, "canUndo": False}),
        ],
    })
    out.append({
        "name": "İğnenin kendi sınıfı değişikliğinkiyle aynı olabilir",
        "steps": [step("execute", {"pins": [{"uid": UID[3], "class": "No", "pin": {"class": "No", "at": {"x": 1.0, "y": 1.0}}}]}, pinned(1),
                       {"entities": {"3": entity(3, [{"class": "No", "at": {"x": 1.0, "y": 1.0}}])}})],
    })
    out.append({
        "name": "Serbest bırak: pin null iğneyi kaldırır",
        "steps": [step("execute", {"pins": [{"uid": UID[5], "pin": None}]}, pinned(1), {"entities": {"5": entity(5, [])}, "revision": "changed"})],
    })
    out.append({
        "name": "Var olan iğne yerinde değişir",
        "steps": [step("execute", {"pins": [{"uid": UID[5], "pin": {"at": {"x": 2.0, "y": 2.0}, "rotation": -30.0}}]}, pinned(1),
                       {"entities": {"5": entity(5, [{"at": {"x": 2.0, "y": 2.0}, "rotation": -30.0}])}})],
    })
    out.append({
        "name": "Aynı iğne: bir şey yazılmaz",
        "steps": [step("execute", {"pins": [{"uid": UID[5], "pin": {"at": {"x": 1.0, "y": 2.0}}}]}, pinned(0), {**UNCHANGED, "entities": {"5": ENTITIES[4]}})],
    })
    out.append({
        "name": "İğnesi olmayan etiketi serbest bırakmak değiştirmez",
        "steps": [step("execute", {"pins": [{"uid": UID[1], "pin": None}]}, pinned(0), UNCHANGED)],
    })
    out.append({
        "name": "Doğrula: yazmaz",
        "steps": [step("validate", {"pins": [{"uid": UID[1], "pin": moved}]}, {"status": "completed", "output": None, "warnings": []}, UNCHANGED)],
    })
    out.append({
        "name": "Plan: değişecek nesnelerin iğneleri; yazmaz",
        "steps": [step("plan", {"pins": [{"uid": UID[1], "pin": moved}, {"uid": UID[5], "pin": {"at": {"x": 1.0, "y": 2.0}}}]},
                       completed({"objects": [{"uid": UID[1], "labelPins": [moved]}], "revision": "$current"}), UNCHANGED)],
    })
    # The refusals, in the contract's order.
    out.append(refused("değişiklik yok", {"pins": []}, "no_entities", "Değişecek etiket yok. En az bir nesnenin etiketini verin.", "pins"))
    out.append(refused("bozuk kimlik", {"pins": [{"uid": "bozuk", "pin": moved}]}, "invalid_uid",
                       "“bozuk” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi).",
                       "pins[0]/uid"))
    out.append(refused("iğnenin sınıfı değişikliğinkinden başka", {"pins": [{"uid": UID[3], "class": "No", "pin": {"class": "Malik", "hidden": True}}]},
                       "invalid_pin", "İğnenin sınıfı değişikliğin sınıfıyla aynı olmalı (ya da verilmemeli).", "pins[0]/pin/class"))
    for name, pin, words in (
        ("yeri olmadan açı", {"rotation": 10.0}, "etiket iğnesinin açısı yerle birlikte verilir"),
        ("ne yer ne gizli", {"hidden": False}, "etiket iğnesi ya bir yer ya da gizli olur"),
        ("hidden false", {"at": {"x": 0.0, "y": 0.0}, "hidden": False}, "etiket iğnesinin hidden'ı yalnız true yazılır"),
        ("açı 400", {"at": {"x": 0.0, "y": 0.0}, "rotation": 400.0}, "etiket iğnesinin açısı −360–360 derece olmalı"),
        ("çok uzak", {"at": {"x": 2e7, "y": 0.0}}, "etiket iğnesinin yeri sonlu ve 10 000 km'den yakın olmalı"),
    ):
        out.append(refused(f"bozuk iğne: {name}", {"pins": [{"uid": UID[1], "pin": pin}]}, "invalid_pin", f"Etiket iğnesi: {words}.", "pins[0]/pin"))
    out.append(refused("aynı etiket iki kez", {"pins": [{"uid": UID[1], "pin": moved}, {"uid": UID[1], "pin": None}]}, "invalid_pin",
                       "Aynı nesnenin aynı etiketi iki kez değişiyor; her etiketi bir kez verin.", "pins[1]"))
    out.append(refused("bozuk iğne sürümden önce", {"pins": [{"uid": UID[1], "pin": {"rotation": 10.0}}], "expectedRevision": "on iki"}, "invalid_pin",
                       "Etiket iğnesi: etiket iğnesinin açısı yerle birlikte verilir.", "pins[0]/pin"))
    out.append(refused("geçersiz beklenen sürüm", {"pins": [{"uid": UID[1], "pin": moved}], "expectedRevision": "on iki"}, "invalid_revision", INVALID_REVISION,
                       "expectedRevision"))
    out.append({
        "name": "sürüm çakışması: plan edilen sürümden sonra çizim değişmişse hiçbir şey yazılmaz",
        "steps": [
            step("captureRevision", **{"as": "once"}),
            step("execute", {"pins": [{"uid": UID[2], "pin": {"hidden": True}}]}, pinned(1), {"revision": "changed"}),
            step("execute", {"pins": [{"uid": UID[1], "pin": moved}], "expectedRevision": "$once"}, CONFLICTED,
                 {"revision": "same", "entities": {"1": entity(1, [])}}),
        ],
    })
    out.append(refused("çizimde olmayan nesne", {"pins": [{"uid": "0192f5a1-7777-7000-8000-00000000ffff", "pin": moved}]}, "entity_not_found",
                       "“0192f5a1-7777-7000-8000-00000000ffff” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.",
                       "pins[0]/uid"))
    out.append(refused("kilitli katman", {"pins": [{"uid": UID[1], "pin": moved}, {"uid": UID[4], "pin": {"hidden": True}}]}, "layer_locked",
                       "“Sit alanı” katmanı kilitli; üzerindeki nesnenin etiketi değişmez. Kilidi Katmanlar panelinden açın.", "pins[1]/uid"))
    out.append(refused("tek etiketli katmanda sınıf", {"pins": [{"uid": UID[1], "class": "No", "pin": moved}]}, "unknown_class",
                       "“Parsel” katmanının tek etiketi var; sınıfı verilmez.", "pins[0]/class"))
    out.append(refused("kurallı katmanda olmayan sınıf", {"pins": [{"uid": UID[3], "class": "Yok", "pin": moved}]}, "unknown_class",
                       "“Kurallar” katmanının “Yok” adlı etiket sınıfı yok.", "pins[0]/class"))
    return out


def build(command, title, cases, note):
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": command,
        "commandVersion": 1,
        "title": title,
        "note": note,
        "setup": setup(),
        "cases": cases,
    }


FILES = [
    ("cad.layers.labels.json", lambda: build("cad.layers.labels", "Etiketler", layers_labels_cases(),
                                             "Written by scripts/fixtures/label_command_cases.py from the contract's rules and docs/adr/0212 §2, §5; do not edit by hand.")),
    ("cad.labels.pin.json", lambda: build("cad.labels.pin", "Etiketi sabitle", labels_pin_cases(),
                                          "Written by scripts/fixtures/label_command_cases.py from the contract's rules and docs/adr/0212 §3.7, §5; do not edit by hand.")),
]


def main() -> int:
    check = "--check" in sys.argv[1:]
    bad = False
    for name, make in FILES:
        doc = make()
        text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
        path = OUT / name
        if check:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                print(f"{path.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
                bad = True
            else:
                print(f"{doc['command']} durumları güncel: {len(doc['cases'])} durum.")
        else:
            path.write_text(text, encoding="utf-8")
            print(f"yazıldı: {path.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
