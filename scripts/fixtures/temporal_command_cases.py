"""Writes fixtures/commands/v1/cad.layers.time.json and cad.scenarios.edit.json: the shared cases of `cad.layers.time` v1
and `cad.scenarios.edit` v1 (docs/adr/0210 §9, §11), their expectations built here from the contracts' rules
(crates/shared/contracts/src/cad_layers.rs, cad_scenarios.rs, temporal.rs: the checks and their order, the codes, the
paths and the words; where the scenario goes and what its layers keep; the steps' names), without KentOS code. The web
(apps/web/src/product/fixtures.test.ts), the desktop (crates/native/application/tests/all/fixtures.rs) and Python
(python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/temporal_command_cases.py           # writes the files
    python3 scripts/fixtures/temporal_command_cases.py --check   # compares them with the ones on disk
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT_TIME = ROOT / "fixtures/commands/v1/cad.layers.time.json"
OUT_SCENARIO = ROOT / "fixtures/commands/v1/cad.scenarios.edit.json"
RULES = ROOT / "fixtures/temporal/v1/rules.json"

# A new node's look: the layer tree's defaults (web `defaultStyle`, desktop `default_style`).
NEW_STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.18}
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
RED = {"color": "#E5484D", "lineType": "continuous", "lineWeight": 0.35}
GREEN = {"color": "#30A46C", "lineType": "continuous", "lineWeight": 0.25}
BLUE = {"color": "#0090FF", "lineType": "dashed", "lineWeight": 0.25}

OSM = {"kind": "xyz", "url": "https://tile.openstreetmap.org/{z}/{x}/{y}.png", "maxZoom": 19}
PARSEL_TIME = {"start": "gecerlilik_baslangic", "end": "gecerlilik_bitis", "key": "parsel_no"}
PARSEL_SNAP = {"kinds": ["endpoint", "midpoint"]}
YOL_FIELDS = [{"name": "genislik", "kind": "decimal"}]
NOTE = "12 m'lik yol önerisi"

TIME_LABEL = "Zaman ayarları"
CREATE_LABEL = "Senaryo oluştur"
APPLY_LABEL = "Senaryoyu uygula"
NAME_MAX = 80


def node(id_, name, type_="layer", children=None, style=STYLE, **extra):
    return {"id": id_, "name": name, "type": type_, "visible": True, "locked": False, "expanded": True, "style": dict(style), "children": children or [], **extra}


SETUP_LAYERS = [
    node("alt-a", "Yol genişletme A", "group", [
        node("alt-a-yol", "Yol", style=RED, replaces="yol"),
        node("alt-a-park", "Yeni park", style=GREEN),
    ], visible=False, scenario={"note": NOTE}),
    node("yol", "Yol", style=RED, fields=copy.deepcopy(YOL_FIELDS)),
    node("parsel", "Parsel", style=BLUE, time=dict(PARSEL_TIME), snap=copy.deepcopy(PARSEL_SNAP)),
    node("koruma", "Koruma alanı", "group", [node("bina", "Bina")], locked=True),
    node("altlik", "OpenStreetMap", service=dict(OSM)),
    node("0", "0"),
]


def square(x, y, size=20):
    return [{"x": x, "y": y}, {"x": x + size, "y": y}, {"x": x + size, "y": y + size}, {"x": x, "y": y + size}]


ENTITIES = [
    {"kind": "polygon", "id": 1, "layerId": "parsel", "attrs": {"parsel_no": "101", "gecerlilik_baslangic": "2010-03-05", "gecerlilik_bitis": "2018-06-01"}, "pts": square(500000, 4400000, 40)},
    {"kind": "polygon", "id": 2, "layerId": "parsel", "attrs": {"parsel_no": "101", "gecerlilik_baslangic": "2018-06-01"}, "pts": square(500000, 4400000)},
    {"kind": "line", "id": 3, "layerId": "yol", "attrs": {"genislik": "8"}, "a": {"x": 499990, "y": 4400045}, "b": {"x": 500050, "y": 4400045}},
    {"kind": "point", "id": 4, "layerId": "0", "attrs": {}, "p": {"x": 500010, "y": 4400060}},
    {"kind": "polyline", "id": 5, "layerId": "alt-a-yol", "attrs": {"genislik": "12"}, "pts": [{"x": 499990, "y": 4400046}, {"x": 500050, "y": 4400046}]},
    {"kind": "polygon", "id": 6, "layerId": "alt-a-park", "attrs": {}, "pts": square(500060, 4400000)},
    {"kind": "polygon", "id": 7, "layerId": "bina", "attrs": {}, "pts": square(500100, 4400000, 10)},
]
IDS = [e["id"] for e in ENTITIES]


def setup(layers=None, active="0", entities=None):
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Zaman ve senaryolar",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": 500000, "y": 4400000},
        "layers": copy.deepcopy(SETUP_LAYERS if layers is None else layers),
        "activeLayer": active,
        "entities": copy.deepcopy(ENTITIES if entities is None else entities),
        "styles": {"items": [], "categories": []},
    }


def tree():
    return copy.deepcopy(SETUP_LAYERS)


def find(nodes, id_):
    for i, n in enumerate(nodes):
        if n["id"] == id_:
            return nodes, i
        hit = find(n["children"], id_)
        if hit:
            return hit
    return None


def get(nodes, id_):
    siblings, i = find(nodes, id_)
    return siblings[i]


def changed(layers, id_, drop=(), **fields):
    """The tree with node `id_` given `fields` and without the keys in `drop`."""
    out = copy.deepcopy(layers)
    n = get(out, id_)
    n.update(copy.deepcopy(fields))
    for k in drop:
        n.pop(k, None)
    return out


def removed(layers, id_):
    out = copy.deepcopy(layers)
    siblings, i = find(out, id_)
    del siblings[i]
    return out


def entity(id_, **fields):
    e = copy.deepcopy(next(e for e in ENTITIES if e["id"] == id_))
    e.update(fields)
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


def words():
    """The contract's words for the rules' cases (fixtures/temporal/v1/rules.json), by group and name."""
    rules = json.loads(RULES.read_text(encoding="utf-8"))
    return {(g, c["name"]): c["problem"] for g in ("times", "scenarios") for c in rules[g]}


# ── cad.layers.time ─────────────────────────────────────────────────────────


def time_refused(name, input_, code, message, path, layers=None, setup_=None):
    case = {"name": name, "steps": [step("execute", input_, failed(code, message, path), {**UNCHANGED, "layers": layers or tree()})]}
    if setup_ is not None:
        case["setup"] = setup_
    return case


def timed(layer, changed_=True):
    return completed({"layer": layer, "changed": changed_, "revision": "$current"})


def time_cases():
    w = words()
    out = []
    yol_time = {"start": "yapim_tarihi", "end": "yikim_tarihi"}
    with_yol = changed(tree(), "yol", time=yol_time)
    out.append({
        "name": "Aralıklı: başlangıç ve bitiş alanları; adımı “Zaman ayarları”, geri alınır ve yinelenir",
        "steps": [
            step("execute", {"layer": "yol", "time": yol_time}, timed("yol"), {"ids": IDS, "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed", "layers": with_yol}),
            step("undo", returns=TIME_LABEL, expect={"layers": tree(), "canUndo": False, "canRedo": True}),
            step("redo", returns=TIME_LABEL, expect={"layers": with_yol, "canUndo": True, "canRedo": False}),
        ],
    })
    out.append({
        "name": "Anlık: yalnız başlangıç alanı",
        "steps": [step("execute", {"layer": "0", "time": {"start": "tarih"}}, timed("0"), {"layers": changed(tree(), "0", time={"start": "tarih"}), "canUndo": True})],
    })
    cumulative = {"start": "tarih", "key": "ariza_no", "cumulative": True}
    out.append({
        "name": "Birikimli ve kimlikli: alanlar yazıldığı gibi",
        "steps": [step("execute", {"layer": "0", "time": cumulative}, timed("0"), {"layers": changed(tree(), "0", time=cumulative)})],
    })
    out.append({
        "name": "Birikimli false yazılmaz: alan yoksa birikimli değildir",
        "steps": [step("execute", {"layer": "0", "time": {"start": "tarih", "cumulative": False}}, timed("0"), {"layers": changed(tree(), "0", time={"start": "tarih"})})],
    })
    instant = {"start": "gecerlilik_baslangic", "key": "parsel_no"}
    out.append({
        "name": "Değiştir: aralıklı katman anlık olur; nesneleri değişmez",
        "steps": [step("execute", {"layer": "parsel", "time": instant}, timed("parsel"), {"ids": IDS, "entities": {"1": entity(1), "2": entity(2)}, "layers": changed(tree(), "parsel", time=instant)})],
    })
    out.append({
        "name": "Kaldır: time null; geri almada zaman döner",
        "steps": [
            step("execute", {"layer": "parsel", "time": None}, timed("parsel"), {"layers": changed(tree(), "parsel", drop=("time",)), "canUndo": True, "revision": "changed"}),
            step("undo", returns=TIME_LABEL, expect={"layers": tree()}),
        ],
    })
    out.append({
        "name": "Kaldır: time verilmezse de",
        "steps": [step("execute", {"layer": "parsel"}, timed("parsel"), {"layers": changed(tree(), "parsel", drop=("time",))})],
    })
    out.append({
        "name": "Aynı ayar: bir şey yazılmaz, changed false",
        "steps": [step("execute", {"layer": "parsel", "time": PARSEL_TIME}, timed("parsel", False), {**UNCHANGED, "layers": tree()})],
    })
    out.append({
        "name": "Zamanı olmayan katmanın zamanını kaldırmak: changed false",
        "steps": [step("execute", {"layer": "yol", "time": None}, timed("yol", False), UNCHANGED)],
    })
    out.append({
        "name": "Senaryo katmanının da zamanı olur",
        "steps": [step("execute", {"layer": "alt-a-yol", "time": {"start": "acilis"}}, timed("alt-a-yol"), {"layers": changed(tree(), "alt-a-yol", time={"start": "acilis"})})],
    })
    out.append({
        "name": "Kilitli katman: zaman katmanın ayarıdır, nesnelere dokunmaz; yazılır",
        "steps": [step("execute", {"layer": "bina", "time": {"start": "ruhsat_tarihi"}}, timed("bina"), {"ids": IDS, "layers": changed(tree(), "bina", time={"start": "ruhsat_tarihi"})})],
    })
    out.append({
        "name": "Servis katmanından zamanı kaldırmak reddedilmez: zamanı yok",
        "steps": [step("execute", {"layer": "altlik"}, timed("altlik", False), UNCHANGED)],
    })
    out.append({
        "name": "Doğrula: yazmaz",
        "steps": [step("validate", {"layer": "yol", "time": yol_time}, {"status": "completed", "output": None, "warnings": []}, {**UNCHANGED, "layers": tree()})],
    })
    out.append({
        "name": "Plan: katmanın yazılacak hâli; yazmaz",
        "steps": [step("plan", {"layer": "yol", "time": yol_time}, completed({"node": get(with_yol, "yol"), "changed": True, "revision": "$current"}), {**UNCHANGED, "layers": tree()})],
    })
    out.append({
        "name": "Plan: kaldırmada zamansız katman",
        "steps": [step("plan", {"layer": "parsel", "time": None}, completed({"node": get(changed(tree(), "parsel", drop=("time",)), "parsel"), "changed": True, "revision": "$current"}), UNCHANGED)],
    })
    out.append({
        "name": "Plan: aynı ayar, changed false",
        "steps": [step("plan", {"layer": "parsel", "time": PARSEL_TIME}, completed({"node": get(tree(), "parsel"), "changed": False, "revision": "$current"}), UNCHANGED)],
    })
    # The refusals, in the contract's order: the setting's rules first.
    for name, value, rule in (
        ("boş başlangıç", {"start": ""}, "boş başlangıç"),
        ("başlangıçta boşluk", {"start": " baslangic"}, "boşluklu başlangıç"),
        ("boş bitiş", {"start": "baslangic", "end": ""}, "boş bitiş"),
        ("bitiş başlangıçla aynı", {"start": "tarih", "end": "tarih"}, "aynı alan"),
        ("kimlik 65 karakter", {"start": "tarih", "key": "k" * 65}, "kimlik 65 karakter"),
        ("başlangıç 65 karakter", {"start": "b" * 65}, "65 karakter"),
    ):
        problem = w[("times", rule)]
        assert problem, rule
        out.append(time_refused(f"bozuk ayar: {name}", {"layer": "yol", "time": value}, "invalid_time", f"Katmanın zamanı: {problem}.", "time"))
    out.append(time_refused("bozuk ayar sürümden ve katmandan önce", {"layer": "yok", "time": {"start": ""}, "expectedRevision": "on iki"}, "invalid_time", f"Katmanın zamanı: {w[('times', 'boş başlangıç')]}.", "time"))
    out.append(time_refused("geçersiz beklenen sürüm", {"layer": "yol", "time": yol_time, "expectedRevision": "on iki"}, "invalid_revision", INVALID_REVISION, "expectedRevision"))
    out.append({
        "name": "sürüm çakışması: plan edilen sürümden sonra çizim değişmişse hiçbir şey yazılmaz",
        "steps": [
            step("captureRevision", **{"as": "once"}),
            step("execute", {"layer": "0", "time": {"start": "tarih"}}, timed("0"), {"revision": "changed"}),
            step("execute", {"layer": "yol", "time": yol_time, "expectedRevision": "$once"},
                 {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}},
                 {"revision": "same", "layers": changed(tree(), "0", time={"start": "tarih"})}),
        ],
    })
    out.append(time_refused("çizimde olmayan katman", {"layer": "yok", "time": yol_time}, "layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layer"))
    out.append(time_refused("grup", {"layer": "koruma", "time": yol_time}, "not_a_layer", "“Koruma alanı” bir katman grubu; zaman yalnız katmanın olur. Grubun bir katmanını verin.", "layer"))
    out.append(time_refused("senaryo grubu", {"layer": "alt-a", "time": None}, "not_a_layer", "“Yol genişletme A” bir katman grubu; zaman yalnız katmanın olur. Grubun bir katmanını verin.", "layer"))
    out.append(time_refused("servis katmanı", {"layer": "altlik", "time": {"start": "tarih"}}, "service_layer", "“OpenStreetMap” servisten çizilir ve nesne tutmaz; zaman nesneleri olan katmanın olur.", "layer"))
    return out


# ── cad.scenarios.edit ──────────────────────────────────────────────────────


def scenario_group(name, children, note=None, id_="$layer:1"):
    """The group Senaryo oluştur puts on top of the tree: hidden, open, the default look, marked a scenario."""
    return node(id_, name, "group", children, style=NEW_STYLE, visible=False, scenario={"note": note} if note else {})


def copy_of(source_id, id_):
    """A base layer's copy in a scenario: the source's name, look, fields, time and snapping; shown, unlocked, open;
    standing for its source."""
    src = get(tree(), source_id)
    out = node(id_, src["name"], style=src["style"], replaces=source_id)
    for k in ("fields", "time", "snap"):
        if k in src:
            out[k] = copy.deepcopy(src[k])
    return out


def created(scenario, pairs, objects):
    return completed({"scenario": scenario, "layers": [{"base": b, "layer": l} for b, l in pairs], "kept": [], "objects": objects, "removed": 0, "revision": "$current"})


def applied(scenario, pairs, kept, objects, removed_):
    return completed({"scenario": scenario, "layers": [{"base": b, "layer": l} for b, l in pairs], "kept": kept, "objects": objects, "removed": removed_, "revision": "$current"})


def scenario_refused(name, input_, code, message, path, setup_=None, layers=None):
    case = {"name": name, "steps": [step("execute", input_, failed(code, message, path), {**UNCHANGED, "layers": layers or tree()})]}
    if setup_ is not None:
        case["setup"] = setup_
    return case


def scenario_cases():
    w = words()
    out = []
    # Senaryo oluştur.
    one = [scenario_group("Yol genişletme B", [copy_of("yol", "$layer:2")])] + tree()
    copy3 = entity(3, id=8, layerId="$layer:2")
    out.append({
        "name": "Oluştur: bir katmanın kopyası nesneleriyle; grup ağacın başında, gizli; adımı “Senaryo oluştur”, geri alınır ve yinelenir",
        "steps": [
            step("execute", {"operation": "create", "name": "Yol genişletme B", "layers": ["yol"]}, created("$layer:1", [("yol", "$layer:2")], 1),
                 {"ids": IDS + [8], "entities": {"3": entity(3), "8": copy3}, "uids": {"8": "new"}, "layers": one, "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}),
            step("undo", returns=CREATE_LABEL, expect={"ids": IDS, "layers": tree(), "canUndo": False, "canRedo": True}),
            step("redo", returns=CREATE_LABEL, expect={"ids": IDS + [8], "entities": {"8": copy3}, "layers": one, "canUndo": True, "canRedo": False}),
        ],
    })
    two = [scenario_group("Yeni imar", [copy_of("parsel", "$layer:2"), copy_of("yol", "$layer:3")], note=NOTE)] + tree()
    out.append({
        "name": "Oluştur: iki katman verildiği sırayla, notuyla; kopyalar kaynağın görünüşünü, alanlarını, zamanını ve keneti taşır, nesneleri yeni yuvalarda",
        "steps": [
            step("captureUid", id=1, **{"as": "p1"}),
            step("execute", {"operation": "create", "name": "Yeni imar", "layers": ["parsel", "yol"], "note": NOTE},
                 created("$layer:1", [("parsel", "$layer:2"), ("yol", "$layer:3")], 3),
                 {"ids": IDS + [8, 9, 10], "entities": {"1": entity(1), "8": entity(1, id=8, layerId="$layer:2"), "9": entity(2, id=9, layerId="$layer:2"), "10": entity(3, id=10, layerId="$layer:3")},
                  "uids": {"1": "p1", "8": "new", "9": "new", "10": "new"}, "layers": two}),
        ],
    })
    out.append({
        "name": "Oluştur: nesnesiz (copyObjects false); katmanlar boş",
        "steps": [step("execute", {"operation": "create", "name": "Taslak", "layers": ["yol", "parsel"], "copyObjects": False},
                       created("$layer:1", [("yol", "$layer:2"), ("parsel", "$layer:3")], 0),
                       {"ids": IDS, "layers": [scenario_group("Taslak", [copy_of("yol", "$layer:2"), copy_of("parsel", "$layer:3")])] + tree()})],
    })
    out.append({
        "name": "Oluştur: katmansız boş senaryo",
        "steps": [step("execute", {"operation": "create", "name": "Boş öneri"}, created("$layer:1", [], 0), {"ids": IDS, "layers": [scenario_group("Boş öneri", [])] + tree(), "canUndo": True})],
    })
    out.append({
        "name": "Oluştur: boş katman listesi de boş senaryo",
        "steps": [step("execute", {"operation": "create", "name": "Boş öneri", "layers": []}, created("$layer:1", [], 0), {"layers": [scenario_group("Boş öneri", [])] + tree()})],
    })
    out.append({
        "name": "Oluştur: adın başındaki ve sonundaki boşluk atılır",
        "steps": [step("execute", {"operation": "create", "name": "  Öneri 2  "}, created("$layer:1", [], 0), {"layers": [scenario_group("Öneri 2", [])] + tree()})],
    })
    out.append({
        "name": "Oluştur: 80 karakterlik ad",
        "steps": [step("execute", {"operation": "create", "name": "ö" * NAME_MAX}, created("$layer:1", [], 0), {"layers": [scenario_group("ö" * NAME_MAX, [])] + tree()})],
    })
    out.append({
        "name": "Oluştur: kilitli katman nesneleri kopyalanmadan kopyalanabilir",
        "steps": [step("execute", {"operation": "create", "name": "Koruma önerisi", "layers": ["bina"], "copyObjects": False}, created("$layer:1", [("bina", "$layer:2")], 0),
                       {"ids": IDS, "layers": [scenario_group("Koruma önerisi", [copy_of("bina", "$layer:2")])] + tree()})],
    })
    out.append({
        "name": "Oluştur: “0” katmanı da ana katmandır, nesnesiyle kopyalanır",
        "steps": [step("execute", {"operation": "create", "name": "Sıfır", "layers": ["0"]}, created("$layer:1", [("0", "$layer:2")], 1),
                       {"ids": IDS + [8], "entities": {"8": entity(4, id=8, layerId="$layer:2")}, "layers": [scenario_group("Sıfır", [copy_of("0", "$layer:2")])] + tree()})],
    })
    out.append({
        "name": "Plan: verilecek kimlikler ve kopyalanacak nesne sayısı; yazmaz",
        "steps": [step("plan", {"operation": "create", "name": "Yeni imar", "layers": ["parsel", "yol"]}, completed({"scenario": "$layer:1", "layers": [{"base": "parsel", "layer": "$layer:2"}, {"base": "yol", "layer": "$layer:3"}], "kept": [], "objects": 3, "removed": 0, "revision": "$current"}),
                       {**UNCHANGED, "layers": tree()})],
    })
    # Senaryoyu uygula.
    after_apply = changed(removed(tree(), "alt-a-yol"), "alt-a", drop=("scenario",))
    out.append({
        "name": "Uygula: senaryonun yolu ana katmanın nesnelerinin yerini alır (kimlikleri kalır), boşalan katman gider; grup kalan katmanıyla sıradan grup olur; adımı “Senaryoyu uygula”, geri alınır ve yinelenir",
        "steps": [
            step("captureUid", id=5, **{"as": "oneri"}),
            step("execute", {"operation": "apply", "scenario": "alt-a"}, applied("alt-a", [("yol", "alt-a-yol")], ["alt-a-park"], 1, 1),
                 {"ids": [1, 2, 4, 5, 6, 7], "entities": {"5": entity(5, layerId="yol"), "6": entity(6)}, "uids": {"5": "oneri"}, "layers": after_apply, "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}),
            step("undo", returns=APPLY_LABEL, expect={"ids": IDS, "entities": {"3": entity(3), "5": entity(5)}, "uids": {"5": "oneri"}, "layers": tree(), "canUndo": False, "canRedo": True}),
            step("redo", returns=APPLY_LABEL, expect={"ids": [1, 2, 4, 5, 6, 7], "entities": {"5": entity(5, layerId="yol")}, "layers": after_apply, "canUndo": True}),
        ],
    })
    only = [node("alt-b", "Yalnız yol", "group", [node("alt-b-yol", "Yol", style=RED, replaces="yol")], visible=False, scenario={})] + tree()
    only_entities = ENTITIES + [{"kind": "line", "id": 8, "layerId": "alt-b-yol", "attrs": {"genislik": "10"}, "a": {"x": 499990, "y": 4400047}, "b": {"x": 500050, "y": 4400047}}]
    only_ids = IDS + [8]
    out.append({
        "name": "Uygula: yalnız yerine geçen katmanları olan senaryonun grubu da gider",
        "setup": setup(layers=only, entities=only_entities),
        "steps": [step("execute", {"operation": "apply", "scenario": "alt-b"}, applied("alt-b", [("yol", "alt-b-yol")], [], 1, 1),
                       {"ids": [1, 2, 4, 5, 6, 7, 8], "entities": {"8": {**only_entities[-1], "layerId": "yol"}}, "layers": tree()})],
    })
    out.append({
        "name": "Uygula: etkin katman giden senaryo katmanındaysa ana katmana geçer",
        "setup": setup(active="alt-a-yol"),
        "steps": [
            step("execute", {"operation": "apply", "scenario": "alt-a"}, applied("alt-a", [("yol", "alt-a-yol")], ["alt-a-park"], 1, 1), {"layers": after_apply}),
            step("undo", returns=APPLY_LABEL, expect={"layers": tree(), "ids": IDS}),
        ],
    })
    empty_alt = [node("alt-c", "Yolu kaldır", "group", [node("alt-c-yol", "Yol", style=RED, replaces="yol")], visible=False, scenario={})] + tree()
    out.append({
        "name": "Uygula: boş yerine geçen katman ana katmanı boşaltır",
        "setup": setup(layers=empty_alt),
        "steps": [step("execute", {"operation": "apply", "scenario": "alt-c"}, applied("alt-c", [("yol", "alt-c-yol")], [], 0, 1), {"ids": [1, 2, 4, 5, 6, 7], "layers": tree()})],
    })
    stale = [node("alt-d", "Eski öneri", "group", [node("alt-d-yol", "Yol", style=RED, replaces="yol"), node("alt-d-eski", "Eski bina", replaces="yikilan")], visible=False, scenario={})] + tree()
    out.append({
        "name": "Uygula: ana katmanı çizimde olmayan senaryo katmanı kalır, yerine geçtiği bağ düşer",
        "setup": setup(layers=stale),
        "steps": [step("execute", {"operation": "apply", "scenario": "alt-d"}, applied("alt-d", [("yol", "alt-d-yol")], ["alt-d-eski"], 0, 1),
                       {"layers": [node("alt-d", "Eski öneri", "group", [node("alt-d-eski", "Eski bina")], visible=False)] + tree()})],
    })
    shown = changed(tree(), "alt-a", visible=True)
    shown = changed(shown, "yol", visible=False)
    out.append({
        "name": "Uygula: görünürlükler olduğu gibi kalır (görünüm arayüzün işidir)",
        "setup": setup(layers=shown),
        "steps": [step("execute", {"operation": "apply", "scenario": "alt-a"}, applied("alt-a", [("yol", "alt-a-yol")], ["alt-a-park"], 1, 1),
                       {"layers": changed(removed(shown, "alt-a-yol"), "alt-a", drop=("scenario",))})],
    })
    out.append({
        "name": "Plan: uygulamanın taşıyacağı ve sileceği; yazmaz",
        "steps": [step("plan", {"operation": "apply", "scenario": "alt-a"}, completed({"scenario": "alt-a", "layers": [{"base": "yol", "layer": "alt-a-yol"}], "kept": ["alt-a-park"], "objects": 1, "removed": 1, "revision": "$current"}),
                       {**UNCHANGED, "layers": tree()})],
    })
    out.append({
        "name": "Oluştur, sonra uygula: kopyalar ana katmana döner",
        "steps": [
            step("execute", {"operation": "create", "name": "Deneme", "layers": ["yol"]}, created("$layer:1", [("yol", "$layer:2")], 1), {"ids": IDS + [8]}),
            step("execute", {"operation": "apply", "scenario": "$layer:1"}, applied("$layer:1", [("yol", "$layer:2")], [], 1, 1),
                 {"ids": [1, 2, 4, 5, 6, 7, 8], "entities": {"8": entity(3, id=8)}, "layers": tree()}),
            step("undo", returns=APPLY_LABEL, expect={"ids": IDS + [8], "layers": [scenario_group("Deneme", [copy_of("yol", "$layer:2")])] + tree()}),
        ],
    })
    out.append({
        "name": "Doğrula: yazmaz",
        "steps": [step("validate", {"operation": "apply", "scenario": "alt-a"}, {"status": "completed", "output": None, "warnings": []}, {**UNCHANGED, "layers": tree()})],
    })
    # The refusals, in the contract's order.
    out.append(scenario_refused("adsız oluşturma", {"operation": "create", "layers": ["yol"]}, "empty_name", "Senaryonun adı boş olamaz; bir ad verin.", "name"))
    out.append(scenario_refused("yalnız boşluk ad", {"operation": "create", "name": " \t"}, "empty_name", "Senaryonun adı boş olamaz; bir ad verin.", "name"))
    out.append(scenario_refused("81 karakterlik ad", {"operation": "create", "name": "ö" * (NAME_MAX + 1)}, "invalid_name", f"Senaryonun adı {NAME_MAX} karakterden uzun; kısaltın.", "name"))
    out.append(scenario_refused("boş not", {"operation": "create", "name": "Öneri", "note": ""}, "invalid_note", f"Senaryonun notu: {w[('scenarios', 'boş not')]}.", "note"))
    out.append(scenario_refused("501 karakterlik not", {"operation": "create", "name": "Öneri", "note": "n" * 501}, "invalid_note", f"Senaryonun notu: {w[('scenarios', '501 karakter')]}.", "note"))
    out.append(scenario_refused("aynı katman iki kez", {"operation": "create", "name": "Öneri", "layers": ["yol", "parsel", "yol"]}, "duplicate_layer", "“yol” katmanı iki kez verildi; her katman bir kez kopyalanır.", "layers/2"))
    out.append(scenario_refused("uygulanacak senaryo yok", {"operation": "apply"}, "no_scenario", "Uygulanacak senaryonun kimliğini (scenario) verin.", "scenario"))
    out.append(scenario_refused("boş senaryo kimliği", {"operation": "apply", "scenario": ""}, "no_scenario", "Uygulanacak senaryonun kimliğini (scenario) verin.", "scenario"))
    out.append(scenario_refused("geçersiz beklenen sürüm", {"operation": "create", "name": "Öneri", "expectedRevision": "on iki"}, "invalid_revision", INVALID_REVISION, "expectedRevision"))
    out.append({
        "name": "sürüm çakışması: plan edilen sürümden sonra çizim değişmişse hiçbir şey yazılmaz",
        "steps": [
            step("captureRevision", **{"as": "once"}),
            step("execute", {"operation": "create", "name": "Boş öneri"}, created("$layer:1", [], 0), {"revision": "changed"}),
            step("execute", {"operation": "apply", "scenario": "alt-a", "expectedRevision": "$once"},
                 {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}},
                 {"revision": "same", "ids": IDS}),
        ],
    })
    out.append(scenario_refused("çizimde olmayan katman", {"operation": "create", "name": "Öneri", "layers": ["yol", "yok"]}, "layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layers/1"))
    base_only = "bir ana katman değil; senaryo yalnız senaryo dışındaki, nesne tutan katmanları kopyalar."
    out.append(scenario_refused("grup kopyalanmaz", {"operation": "create", "name": "Öneri", "layers": ["koruma"]}, "not_a_base_layer", f"“Koruma alanı” {base_only}", "layers/0"))
    out.append(scenario_refused("senaryonun katmanı kopyalanmaz", {"operation": "create", "name": "Öneri", "layers": ["alt-a-park"]}, "not_a_base_layer", f"“Yeni park” {base_only}", "layers/0"))
    out.append(scenario_refused("servis katmanı kopyalanmaz", {"operation": "create", "name": "Öneri", "layers": ["altlik"]}, "not_a_base_layer", f"“OpenStreetMap” {base_only}", "layers/0"))
    out.append(scenario_refused("kilitli katmanın nesneleri kopyalanmaz (grubundan kilitli)", {"operation": "create", "name": "Öneri", "layers": ["yol", "bina"]}, "layer_locked",
                                "“Bina” katmanı kilitli; nesneleri kopyalanmaz. Kilidi Katmanlar panelinden açın.", "layers/1"))
    out.append(scenario_refused("çizimde olmayan senaryo", {"operation": "apply", "scenario": "yok"}, "scenario_not_found", "“yok” kimlikli senaryo çizimde yok. Bir senaryo grubunun kimliğini verin.", "scenario"))
    out.append(scenario_refused("senaryo olmayan grup", {"operation": "apply", "scenario": "koruma"}, "scenario_not_found", "“koruma” kimlikli senaryo çizimde yok. Bir senaryo grubunun kimliğini verin.", "scenario"))
    out.append(scenario_refused("katman senaryo değildir", {"operation": "apply", "scenario": "alt-a-yol"}, "scenario_not_found", "“alt-a-yol” kimlikli senaryo çizimde yok. Bir senaryo grubunun kimliğini verin.", "scenario"))
    locked_park = changed(tree(), "alt-a-park", locked=True)
    out.append(scenario_refused("senaryonun kilitli katmanı", {"operation": "apply", "scenario": "alt-a"}, "layer_locked", "“Yeni park” katmanı kilitli; senaryo uygulanmadı. Kilidi Katmanlar panelinden açın.", "scenario",
                                setup_=setup(layers=locked_park), layers=locked_park))
    locked_base = changed(tree(), "yol", locked=True)
    out.append(scenario_refused("kilitli ana katman", {"operation": "apply", "scenario": "alt-a"}, "layer_locked", "“Yol” katmanı kilitli; senaryo uygulanmadı. Kilidi Katmanlar panelinden açın.", "scenario",
                                setup_=setup(layers=locked_base), layers=locked_base))
    return out


def build_time():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.layers.time",
        "commandVersion": 1,
        "title": "Katmanın zaman ayarı: yaz, değiştir, kaldır (ADR 0210 §11)",
        "note": "scripts/fixtures/temporal_command_cases.py yazar; beklentiler sözleşmenin kurallarından (crates/shared/contracts/src/cad_layers.rs, temporal.rs) KentOS kodu olmadan kurulur; kuralın sözleri fixtures/temporal/v1/rules.json'dan. `expect.layers` katman ağacının tamamıdır.",
        "setup": setup(),
        "cases": time_cases(),
    }


def build_scenario():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.scenarios.edit",
        "commandVersion": 1,
        "title": "Senaryo: oluştur ve uygula (ADR 0210 §9, §11)",
        "note": "scripts/fixtures/temporal_command_cases.py yazar; beklentiler sözleşmenin kurallarından (crates/shared/contracts/src/cad_scenarios.rs, temporal.rs) KentOS kodu olmadan kurulur. `$layer:K` komutun eklediği (plan: ekleyeceği) K'ıncı katmanın kimliğidir: adımdan önce ağaçta olmayan `layer-N`'ler sayılarına göre sıralı (önce senaryo grubu, sonra kopyalar verildikleri sırayla); `expect.layers` katman ağacının tamamıdır.",
        "setup": setup(),
        "cases": scenario_cases(),
    }


def main():
    outs = ((OUT_TIME, build_time()), (OUT_SCENARIO, build_scenario()))
    if "--check" in sys.argv:
        stale = [p for p, b in outs if not p.exists() or p.read_text(encoding="utf-8") != json.dumps(b, ensure_ascii=False, indent=2) + "\n"]
        if stale:
            print("; ".join(str(p.relative_to(ROOT)) for p in stale) + " güncel değil; betiği --check'siz çalıştırıp farkı okuyun.")
            return 1
        print(f"cad.layers.time ve cad.scenarios.edit durumları güncel: {len(outs[0][1]['cases'])} ve {len(outs[1][1]['cases'])} durum.")
        return 0
    for path, built in outs:
        path.write_text(json.dumps(built, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(f"{path.relative_to(ROOT)} yazıldı: {len(built['cases'])} durum.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
