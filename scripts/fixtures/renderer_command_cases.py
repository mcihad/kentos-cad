"""Writes fixtures/commands/v1/cad.layers.renderer.json: the shared cases of Katman stili's command (docs/adr/0213 §5),
their expectations built here from the contract's rules (crates/shared/contracts/src/cad_layers.rs: the checks and their
order, the codes, the paths and the words; the step's name) and the renderer's rules as docs/adr/0213 §5 lists them (what
each kind must have, the ranges, the words of a refusal), without KentOS code. The web (apps/web/src/product/
fixtures.test.ts), the desktop (crates/native/application/tests/all/fixtures.rs) and Python (python/tests/
test_command_cases.py) run them.

    python3 scripts/fixtures/renderer_command_cases.py           # writes the file
    python3 scripts/fixtures/renderer_command_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.layers.renderer.json"

STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
OSM = {"kind": "xyz", "url": "https://tile.openstreetmap.org/{z}/{x}/{y}.png", "maxZoom": 19}
FILL = {"type": "fill", "layers": [{"id": "f", "type": "simpleFill", "color": "#9ECAE1"}]}
MARKER = {"type": "marker", "layers": [{"id": "c", "type": "shape", "shape": "circle", "size": 3, "fill": "#2B83BA"}]}
CATEGORIZED = {"type": "categorized", "expr": "Nitelik", "categories": [
    {"value": "Arsa", "label": "Arsa", "symbols": {"fill": FILL}},
], "other": {"fill": FILL}}


def node(id_, name, type_="layer", children=None, style=None, **extra):
    return {"id": id_, "name": name, "type": type_, "visible": True, "locked": False, "expanded": True,
            "style": dict(style or STYLE), "children": children or [], **extra}


SETUP_LAYERS = [
    node("parsel", "Parsel"),
    node("durak", "Durak"),
    node("yol", "Yol"),
    node("temali", "Nitelik", style={**STYLE, "renderer": copy.deepcopy(CATEGORIZED)}),
    node("koruma", "Koruma alanı", "group", [node("kilitli", "Sit alanı")], locked=True),
    node("altlik", "OpenStreetMap", service=dict(OSM)),
    node("0", "0"),
]


def rect(x, y, w, h):
    return [{"x": x, "y": y}, {"x": x + w, "y": y}, {"x": x + w, "y": y + h}, {"x": x, "y": y + h}]


X, Y = 500000, 4400000
ENTITIES = [
    {"kind": "polygon", "id": 1, "layerId": "parsel", "attrs": {"Nüfus": "120", "Hane": "40"}, "pts": rect(X, Y, 40, 30)},
    {"kind": "polygon", "id": 2, "layerId": "parsel", "attrs": {"Nüfus": "80", "Hane": "25"}, "pts": rect(X + 40, Y, 30, 30)},
    {"kind": "point", "id": 3, "layerId": "durak", "attrs": {"Hat": "12"}, "p": {"x": X + 10, "y": Y + 50}},
    {"kind": "point", "id": 4, "layerId": "durak", "attrs": {"Hat": "14"}, "p": {"x": X + 10.5, "y": Y + 50}},
    {"kind": "polyline", "id": 5, "layerId": "yol", "attrs": {"Trafik": "900"}, "pts": [{"x": X, "y": Y + 40}, {"x": X + 90, "y": Y + 45}]},
    {"kind": "polygon", "id": 6, "layerId": "temali", "attrs": {"Nitelik": "Arsa"}, "pts": rect(X, Y + 60, 20, 20)},
    {"kind": "polygon", "id": 7, "layerId": "kilitli", "attrs": {}, "pts": rect(X + 60, Y + 60, 20, 20)},
]
IDS = [e["id"] for e in ENTITIES]


def setup():
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Katman stili",
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


def with_renderer(layers, id_, renderer):
    """The tree with a layer's renderer set, or taken away (None)."""
    out = copy.deepcopy(layers)
    n = get(out, id_)
    if renderer is None:
        n["style"].pop("renderer", None)
    else:
        n["style"]["renderer"] = copy.deepcopy(renderer)
    return out


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

STEP = "Katman stili"
INNER = ["single", "categorized", "graduated", "rules", "unclassed", "proportional", "bivariate"]


def rendered(layer, changed=True):
    return completed({"layer": layer, "changed": changed, "revision": "$current"})


def refused(name, input_, code, message, path):
    return {"name": name, "steps": [step("execute", input_, failed(code, message, path), {**UNCHANGED, "layers": tree()})]}


def number(x):
    """A number as the rules write it in a message: a whole one without a point."""
    return str(int(x)) if float(x).is_integer() else repr(float(x))


# The renderers written (docs/adr/0213 §2), each a kind's all fields.
RAMP = ["#FFFFCC", "#FD8D3C", "#800026"]
HEAT_RAMP = ["#2B83BA00", "#2B83BA", "#FFFFBF", "#D7191C"]
UNCLASSED = {"type": "unclassed", "expr": "Nüfus", "min": 0, "max": 200, "ramp": RAMP, "symbols": {"fill": FILL},
             "other": {"fill": FILL}}
PROPORTIONAL = {"type": "proportional", "expr": "Hat", "minValue": 10, "maxValue": 20, "minSize": 2, "maxSize": 8, "unit": "mm",
                "scaling": "flannery", "symbols": {"marker": MARKER}}
BIVARIATE = {"type": "bivariate", "exprX": "Nüfus", "exprY": "Hane", "breaksX": [100], "breaksY": [30],
             "colors": ["#E8E8E8", "#5AC8C8", "#BE64AC", "#3B4994"], "symbols": {"fill": FILL}}
DOTS = {"type": "dotDensity", "fields": [{"expr": "Nüfus", "color": "#E15759", "label": "Nüfus"}, {"expr": "Hane", "color": "#4E79A7"}],
        "dotValue": 10, "dotSize": 1, "unit": "mm", "seed": 7}
PIE = {"type": "chart", "kind": "pie", "fields": [{"expr": "Nüfus", "color": "#E15759"}, {"expr": "Hane", "color": "#4E79A7"}], "size": 8,
       "unit": "mm", "sizeBy": {"minValue": 50, "maxValue": 200, "minSize": 4, "maxSize": 12}, "outline": {"color": "#FFFFFF", "width": 0.2}}
BARS = {"type": "chart", "kind": "bar", "fields": [{"expr": "Nüfus", "color": "#E15759"}], "size": 10, "unit": "px", "maxValue": 200,
        "barWidth": 3}
HEAT = {"type": "heatmap", "radius": 25, "unit": "px", "weight": "Hat", "ramp": HEAT_RAMP, "quality": 2, "opacity": 0.8}
CLUSTER = {"type": "cluster", "distance": 30, "unit": "px", "symbol": MARKER, "count": True, "grow": True,
           "renderer": {"type": "categorized", "expr": "Hat", "categories": [{"value": "12", "label": "12", "symbols": {"marker": MARKER}}]}}
DISPLACEMENT = {"type": "displacement", "tolerance": 4, "unit": "px", "placement": "rings", "spacing": 2, "center": MARKER,
                "circle": {"color": "#7D7D7D", "width": 1}, "renderer": {"type": "single", "symbols": {"marker": MARKER}}}
INVERTED = {"type": "inverted", "symbols": {"fill": FILL}, "merge": True}


def cases():
    out = []
    unclassed = with_renderer(tree(), "parsel", UNCLASSED)
    out.append({
        "name": "Sürekli renk yazılır; adımı “Katman stili”, geri alınır ve yinelenir; nesneler değişmez",
        "steps": [
            step("execute", {"layer": "parsel", "renderer": UNCLASSED}, rendered("parsel"),
                 {"ids": IDS, "entities": {"1": ENTITIES[0]}, "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed",
                  "layers": unclassed}),
            step("undo", returns=STEP, expect={"layers": tree(), "canUndo": False, "canRedo": True}),
            step("redo", returns=STEP, expect={"layers": unclassed, "canUndo": True, "canRedo": False}),
        ],
    })
    for name, layer, r in (
        ("Orantılı sembol: Flannery, mm", "durak", PROPORTIONAL),
        ("İki değişkenli renk: 2 × 2", "parsel", BIVARIATE),
        ("Nokta yoğunluğu: iki değer, tohum", "parsel", DOTS),
        ("Grafik: toplama göre büyüyen pasta, çerçeveli", "parsel", PIE),
        ("Grafik: piksel boyunda çubuk", "parsel", BARS),
        ("Isı haritası: ağırlıklı, sabit olmayan en büyük değer", "durak", HEAT),
        ("Kümeleme: tek noktalar Kategorili", "durak", CLUSTER),
        ("Yayma: iç içe halkalar, tek noktalar Tek sembol", "durak", DISPLACEMENT),
        ("Ters alan: örtüşenler boş", "parsel", INVERTED),
    ):
        out.append({"name": name, "steps": [step("execute", {"layer": layer, "renderer": r}, rendered(layer),
                                                 {"layers": with_renderer(tree(), layer, r), "canUndo": True, "revision": "changed"})]})
    simple = with_renderer(tree(), "temali", None)
    out.append({
        "name": "renderer null: katman basit görünüşüne döner; geri almada işleyici döner",
        "steps": [
            step("execute", {"layer": "temali", "renderer": None}, rendered("temali"), {"layers": simple, "revision": "changed"}),
            step("undo", returns=STEP, expect={"layers": tree()}),
        ],
    })
    out.append({
        "name": "renderer verilmezse de basit görünüşe döner",
        "steps": [step("execute", {"layer": "temali"}, rendered("temali"), {"layers": simple})],
    })
    out.append({
        "name": "Aynısı: bir şey yazılmaz, changed false",
        "steps": [step("execute", {"layer": "temali", "renderer": CATEGORIZED}, rendered("temali", False), {**UNCHANGED, "layers": tree()})],
    })
    out.append({
        "name": "İşleyicisi olmayan katmanda null: değişmez",
        "steps": [step("execute", {"layer": "yol", "renderer": None}, rendered("yol", False), {**UNCHANGED, "layers": tree()})],
    })
    out.append({
        "name": "Kilitli katman: işleyici katmanın görünüşüdür, nesnelere dokunmaz; yazılır",
        "steps": [step("execute", {"layer": "kilitli", "renderer": INVERTED}, rendered("kilitli"),
                       {"layers": with_renderer(tree(), "kilitli", INVERTED), "canUndo": True})],
    })
    out.append({
        "name": "İşleyici değişir: bir öncekinin yerine geçer, tek adım",
        "steps": [
            step("execute", {"layer": "temali", "renderer": UNCLASSED}, rendered("temali"), {"layers": with_renderer(tree(), "temali", UNCLASSED)}),
            step("undo", returns=STEP, expect={"layers": tree(), "canUndo": False}),
        ],
    })
    out.append({
        "name": "Doğrula: yazmaz",
        "steps": [step("validate", {"layer": "durak", "renderer": HEAT}, {"status": "completed", "output": None, "warnings": []},
                       {**UNCHANGED, "layers": tree()})],
    })
    out.append({
        "name": "Plan: katmanın yazılacak hâli; yazmaz",
        "steps": [step("plan", {"layer": "durak", "renderer": HEAT},
                       completed({"node": get(with_renderer(tree(), "durak", HEAT), "durak"), "changed": True, "revision": "$current"}),
                       {**UNCHANGED, "layers": tree()})],
    })
    # The refusals, in the contract's order: the renderer's rules, the revision and the layer.
    bad = lambda name, r, message: refused(f"bozuk işleyici: {name}", {"layer": "parsel", "renderer": r}, "invalid_renderer",
                                           f"İşleyici: {message}", "renderer")
    out.append(bad("bilinmeyen tür", {"type": "pie"}, "“pie” türünde bir işleyici yok."))
    out.append(bad("türü yok", {"symbols": {}}, "İşleyicinin türü (“type”) yazılmamış."))
    out.append(bad("nesne değil", "unclassed", "“renderer” bir nesne değil."))
    out.append(bad("en küçük en büyükten büyük", {**UNCLASSED, "min": 300}, "“min” “max”'dan büyük olamaz."))
    out.append(bad("boş ifade", {**UNCLASSED, "expr": "  "}, "“expr” boş: bir alan adı ya da ifade yazın."))
    out.append(bad("tek renkli rampa", {**UNCLASSED, "ramp": ["#FFFFCC"]}, "Rampada 2 ile 16 arası renk olmalı (1 verildi)."))
    out.append(bad("rampada renk değil", {**UNCLASSED, "ramp": ["#FFFFCC", "kırmızı"]}, "“ramp[1]” bir renk değil: #RRGGBB ya da #RRGGBBAA yazın."))
    out.append(bad("orantılının birimi", {**PROPORTIONAL, "unit": "cm"}, "“unit” şunlardan biri olmalı: mm, px."))
    out.append(bad("orantılının boyu", {**PROPORTIONAL, "maxSize": 300},
                   f"“maxSize” {number(0)}'dan büyük, en çok {number(200)} olmalı ({number(300)} verildi)."))
    out.append(bad("iki eksenin sınırları eşit değil", {**BIVARIATE, "breaksY": [10, 20]}, "İki eksenin sınır sayısı eşit olmalı."))
    out.append(bad("ızgaranın renkleri eksik", {**BIVARIATE, "colors": BIVARIATE["colors"][:3]}, "2 × 2 sınıf için 4 renk olmalı (3 verildi)."))
    out.append(bad("nokta değeri 0", {**DOTS, "dotValue": 0}, f"“dotValue” {number(0)}'dan büyük olmalı ({number(0)} verildi)."))
    out.append(bad("değersiz nokta yoğunluğu", {**DOTS, "fields": []}, "1 ile 12 arası alan olmalı (0 verildi)."))
    out.append(bad("değerin rengi", {**DOTS, "fields": [{"expr": "Nüfus", "color": "mavi"}]},
                   "“fields[0].color” bir renk değil: #RRGGBB ya da #RRGGBBAA yazın."))
    out.append(bad("çubuğun en büyük değeri yok", {k: v for k, v in BARS.items() if k != "maxValue"}, "“maxValue” yazılmamış: bir sayı verin."))
    out.append(bad("ısı haritasının yarıçapı 600 px", {**HEAT, "radius": 600},
                   f"“radius” {number(0)}'dan büyük, en çok {number(500)} olmalı ({number(600)} verildi)."))
    out.append(bad("kalite 2,5", {**HEAT, "quality": 2.5}, "“quality” 1 ile 5 arası bir tam sayı olmalı."))
    out.append(bad("kümenin içinde ısı haritası", {**CLUSTER, "renderer": HEAT}, f"İç işleyici şunlardan biri olmalı: {', '.join(INNER)}."))
    out.append(bad("yaymanın yerleşimi", {**DISPLACEMENT, "placement": "spiral"}, "“placement” şunlardan biri olmalı: ring, rings, grid."))
    out.append(bad("ters alanın birleştirmesi", {**INVERTED, "merge": "evet"}, "“merge” evet ya da hayır (true, false) olmalı."))
    out.append(refused("bozuk işleyici sürümden ve katmandan önce", {"layer": "yok", "renderer": {"type": "pie"}, "expectedRevision": "on iki"},
                       "invalid_renderer", "İşleyici: “pie” türünde bir işleyici yok.", "renderer"))
    out.append(refused("geçersiz beklenen sürüm", {"layer": "parsel", "renderer": UNCLASSED, "expectedRevision": "on iki"}, "invalid_revision",
                       INVALID_REVISION, "expectedRevision"))
    out.append({
        "name": "sürüm çakışması: plan edilen sürümden sonra çizim değişmişse hiçbir şey yazılmaz",
        "steps": [
            step("captureRevision", **{"as": "once"}),
            step("execute", {"layer": "0", "renderer": INVERTED}, rendered("0"), {"revision": "changed"}),
            step("execute", {"layer": "parsel", "renderer": UNCLASSED, "expectedRevision": "$once"}, CONFLICTED,
                 {"revision": "same", "layers": with_renderer(tree(), "0", INVERTED)}),
        ],
    })
    out.append(refused("çizimde olmayan katman", {"layer": "yok", "renderer": UNCLASSED}, "layer_not_found",
                       "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layer"))
    out.append(refused("grup", {"layer": "koruma", "renderer": UNCLASSED}, "not_a_layer",
                       "“Koruma alanı” bir katman grubu; işleyici yalnız katmanın olur. Grubun bir katmanını verin.", "layer"))
    out.append(refused("servis katmanı", {"layer": "altlik", "renderer": HEAT}, "service_layer",
                       "“OpenStreetMap” servisten çizilir ve nesne tutmaz; işleyici nesneleri olan katmanın olur.", "layer"))
    out.append(refused("servis katmanında null da", {"layer": "altlik", "renderer": None}, "service_layer",
                       "“OpenStreetMap” servisten çizilir ve nesne tutmaz; işleyici nesneleri olan katmanın olur.", "layer"))
    return out


def build():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.layers.renderer",
        "commandVersion": 1,
        "title": "Katman stili",
        "note": "Written by scripts/fixtures/renderer_command_cases.py from the contract's rules and docs/adr/0213 §2, §5; do not edit by hand.",
        "setup": setup(),
        "cases": cases(),
    }


def main() -> int:
    doc = build()
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"{doc['command']} durumları güncel: {len(doc['cases'])} durum.")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
