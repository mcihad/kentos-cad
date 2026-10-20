"""Writes fixtures/commands/v1/cad.styles.impact.json: the shared cases of `cad.styles.impact` v1 (docs/adr/0314 §4).

The expectations are built here from the contract's rules (crates/shared/commands/src/cad_style_impact.rs) and the
look's rules of docs/adr/0312 §4 and docs/adr/0183, without KentOS code:

- a layer: its objects that take at least one of colour, line type (or pattern), weight and transparency from it
  follow; each value of their own is counted by property; a layer's look reaches a locked layer's objects too; a
  block piece on it with a `byLayer` part follows, and the inserts showing its definition (nested ones too) show it;
  a layer state keeping a look other than the layer's own brings it back;
- a line type: the objects drawn in it by their own pattern, and those on a layer drawn in it without a line type of
  their own; a block piece in it by its own pattern or by its `byLayer` line type on such a layer; the layer states
  whose kept looks name it;
- a text (dimension) style: the texts (dimensions) naming it follow, each field differing from the style's is counted;
  those on a locked layer are counted apart (a style's change leaves them);
- KentOS keeps no external reference: always 0. `notes` are the rule's Turkish sentences.

    python3 scripts/fixtures/style_impact_cases.py           # writes the file
    python3 scripts/fixtures/style_impact_cases.py --check   # compares it with the one on disk
"""
import copy
import importlib.util
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.styles.impact.json"


def _load(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(f"{name}.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


A = _load("annotation_style_cases")

SCALE = 1000
X, Y = 500000, 4400000
DUVAR = {"color": "#111111", "lineType": "continuous", "lineWeight": 0.35}
AKS = {"color": "#E5484D", "lineType": "dashed", "lineWeight": 0.18, "linePattern": "EKSEN"}
PLAIN = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
EKSEN = {"name": "EKSEN", "elements": [{"length": 12.0}, {"length": -3.0}], "unit": "drawing"}
BOS = {"name": "BOS", "elements": [{"length": 2.0}, {"length": -1.0}], "unit": "paper"}
KAPI = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001"
CEPHE = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3002"


def node(id_, name, style=PLAIN, locked=False, children=None, type_="layer"):
    return {"id": id_, "name": name, "type": type_, "visible": True, "locked": locked, "expanded": True,
            "style": dict(style), "children": children or []}


LAYERS = [
    node("duvar", "Duvar", DUVAR),
    node("aks", "Aks", AKS),
    node("yazi", "Yazı"),
    node("kilitli", "Kilitli", locked=True),
    node("bos", "Boş"),
]


def line(id_, layer, y=0, **own):
    return {"kind": "line", "id": id_, "layerId": layer, "attrs": {}, "a": {"x": X, "y": Y + y}, "b": {"x": X + 10, "y": Y + y}, **own}


def text(id_, layer, style, x=0, **hand):
    base = {"kind": "text", "id": id_, "layerId": layer, "attrs": {}, "p": {"x": X + x, "y": Y + 30}, "text": "A", "height": 2.0, "rotation": 0}
    out = A.apply_text(style, base, SCALE)
    out.update(hand)
    return out


def dimension(id_, style, **hand):
    base = {"kind": "dimension", "id": id_, "layerId": "yazi", "attrs": {}, "a": {"x": X, "y": Y + 50}, "b": {"x": X + 24.5, "y": Y + 50},
            "offset": -2, "height": 2.0}
    out = A.apply_dimension(style, base, SCALE)
    out.update(hand)
    return out


def insert(id_, layer, block):
    return {"kind": "insert", "id": id_, "layerId": layer, "attrs": {}, "block": block, "p": {"x": X + id_, "y": Y + 70}, "scale": 1, "rotation": 0}


ENTITIES = [
    line(1, "duvar"),
    line(2, "duvar", 1, color="#FF0000"),
    line(3, "duvar", 2, color="#FF0000", lineType="dashed", lineWeight=0.7, transparency=0.5),
    line(4, "duvar", 3, lineType="dashed", linePattern="EKSEN"),
    line(5, "aks", 4),
    line(6, "aks", 5, lineType="dotted"),
    text(7, "yazi", A.ADA),
    text(8, "kilitli", A.ADA, 10),
    text(9, "yazi", A.ADA, 20, bold=False, font="barlow"),
    dimension(10, A.OLCU),
    dimension(11, A.OLCU, decimals=3),
    insert(12, "duvar", KAPI),
    insert(13, "yazi", CEPHE),
]
for e in ENTITIES:
    if e.get("bold") is False:
        e.pop("bold")
IDS = [e["id"] for e in ENTITIES]


def piece(id_, layer, **fields):
    return {"kind": "line", "id": id_, "layerId": layer, "attrs": {}, "a": {"x": 0, "y": 0}, "b": {"x": 1, "y": 0}, **fields}


BLOCKS = [
    {"id": KAPI, "name": "Kapı", "base": {"x": 0, "y": 0}, "entities": [
        piece(1, "duvar", byLayer=["color"]),
        {**text(2, "0", A.ADA), "p": {"x": 0, "y": 1}},
        piece(3, "duvar"),
    ]},
    {"id": CEPHE, "name": "Cephe", "base": {"x": 0, "y": 0}, "entities": [
        {"kind": "insert", "id": 1, "layerId": "0", "attrs": {}, "block": KAPI, "p": {"x": 0, "y": 0}, "scale": 1, "rotation": 0},
        piece(2, "aks", byLayer=["lineType"]),
    ]},
]

STATES = [
    {"id": "s1", "name": "Sunum", "nodes": [{"node": "duvar", "visible": True, "style": {"color": "#000000", "lineType": "continuous", "lineWeight": 0.5}}]},
    {"id": "s2", "name": "Eksenler", "nodes": [{"node": "aks", "visible": True, "style": dict(AKS)}]},
    {"id": "s3", "name": "Aynı", "nodes": [{"node": "duvar", "visible": True, "style": dict(DUVAR)}]},
]


def setup():
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Stil etkisi",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": SCALE,
                     "workspace": "cad", "drawingFont": "barlow", "textStyles": [A.ADA, A.YOL], "dimensionStyles": [A.OLCU],
                     "lineTypes": [EKSEN, BOS], "layerStates": STATES},
        "origin": {"x": X, "y": Y},
        "layers": copy.deepcopy(LAYERS),
        "activeLayer": "yazi",
        "entities": copy.deepcopy(ENTITIES),
        "styles": {"items": [], "categories": []},
        "blocks": copy.deepcopy(BLOCKS),
    }


# ── The rules ────────────────────────────────────────────────────────────────

def layer_of(id_):
    return next(n for n in LAYERS if n["id"] == id_)


def own_values(e):
    out = []
    if "color" in e:
        out.append("color")
    if "lineType" in e or "linePattern" in e:
        out.append("lineType")
    if "lineWeight" in e:
        out.append("lineWeight")
    if "transparency" in e:
        out.append("transparency")
    return out


def counted(names, values):
    return [{"property": n, "count": values.count(n)} for n in names if values.count(n) > 0]


def shown(blocks):
    """The drawing's inserts showing the definitions `blocks` or one holding them, nested."""
    reach = set(blocks)
    while True:
        more = {b["id"] for b in BLOCKS if b["id"] not in reach
                and any(p["kind"] == "insert" and p["block"] in reach for p in b["entities"])}
        if not more:
            break
        reach |= more
    return sum(1 for e in ENTITIES if e["kind"] == "insert" and e["block"] in reach)


WORDS = {"color": "renk", "lineType": "çizgi tipi", "lineWeight": "kalınlık", "transparency": "saydamlık", "font": "yazı tipi",
         "bold": "kalın", "italic": "eğik", "oblique": "yatıklık", "widthFactor": "genişlik çarpanı", "height": "yükseklik"}


def notes(o, layer_target):
    n = [f"{o['follow']} nesne değişikliği alır."]
    if o["own"]:
        n.append("Kendi değerini koruyanlar: " + ", ".join(f"{w['count']} {WORDS.get(w['property'], w['property'])}" for w in o["own"]) + ".")
    if o["locked"] > 0:
        n.append(f"{o['locked']} nesne kilitli katmanda: stil değişince olduğu gibi kalır.")
    if layer_target:
        n.append("Katmanın görünüşü kilitli katmandaki nesnelere de yansır.")
    if o["blockPieces"] > 0:
        n.append(f"{len(o['blocks'])} blokta {o['blockPieces']} parça izler; {o['inserts']} yerleştirmede görünür.")
    if o["layerStates"]:
        n.append("Katman durumları kendi görünüşünü saklıyor ve uygulanınca geri getirir: " + ", ".join(o["layerStates"]) + ".")
    n.append("Dış başvuru (xref) yok: KentOS dış başvuru taşımaz.")
    return n


def answer(name, follow, own, locked=0, layers=(), blocks=(), pieces=0, states=(), layer_target=False):
    o = {"name": name, "follow": len(follow), "uids": [f"$uidOf:{i}" for i in follow], "own": own, "locked": locked,
         "layers": list(layers), "blocks": list(blocks), "blockPieces": pieces, "inserts": shown(blocks) if blocks else 0,
         "layerStates": list(states), "sheetMaps": [], "externalReferences": 0}
    o["notes"] = notes(o, layer_target)
    o["revision"] = "$current"
    return o


def by_layer(id_):
    layer = layer_of(id_)
    on = [e for e in ENTITIES if e["layerId"] == id_]
    values = [v for e in on for v in own_values(e)]
    follow = [e["id"] for e in on if len(own_values(e)) < 4]
    blocks, pieces = [], 0
    for b in BLOCKS:
        k = sum(1 for p in b["entities"] if p["layerId"] == id_ and p.get("byLayer"))
        if k:
            blocks.append(b["id"])
            pieces += k
    states = [s["name"] for s in STATES if any(n["node"] == id_ and n.get("style") is not None and n["style"] != layer["style"]
                                              for n in s["nodes"])]
    return answer(layer["name"], follow, counted(["color", "lineType", "lineWeight", "transparency"], values),
                  blocks=blocks, pieces=pieces, states=states, layer_target=True)


def by_line_type(name):
    def named(p):
        return p is not None and p.strip().lower() == name.strip().lower()
    layers = [n["id"] for n in LAYERS if named(n["style"].get("linePattern"))]
    follow, values = [], []
    for e in ENTITIES:
        if named(e.get("linePattern")):
            follow.append(e["id"])
        elif e["layerId"] in layers:
            if "lineType" in e or "linePattern" in e:
                values.append("lineType")
            else:
                follow.append(e["id"])
    blocks, pieces = [], 0
    for b in BLOCKS:
        k = sum(1 for p in b["entities"] if named(p.get("linePattern"))
                or (p["layerId"] in layers and "lineType" in p.get("byLayer", [])))
        if k:
            blocks.append(b["id"])
            pieces += k
    states = [s["name"] for s in STATES if any(named((n.get("style") or {}).get("linePattern")) for n in s["nodes"])]
    real = next(t["name"] for t in [EKSEN, BOS] if named(t["name"]))
    return answer(real, follow, counted(["lineType"], values), layers=layers, blocks=blocks, pieces=pieces, states=states)


def by_text_style(style):
    face = A.face_of(style)
    height = A.height_at(style["height"], SCALE) if style.get("height") is not None else None
    names = ["font", "bold", "italic", "oblique", "widthFactor", "height"]
    values, follow, locked = [], [], 0
    for e in ENTITIES:
        if e["kind"] == "text" and e.get("textStyle") == style["id"]:
            if e.get("font") != face.get("font"):
                values.append("font")
            if e.get("bold", False) != style.get("bold", False):
                values.append("bold")
            if e.get("italic", False) != style.get("italic", False):
                values.append("italic")
            if e.get("oblique") != style.get("oblique"):
                values.append("oblique")
            if e.get("widthFactor", 1.0) != style.get("widthFactor", 1.0):
                values.append("widthFactor")
            if height is not None and height != e["height"]:
                values.append("height")
            follow.append(e["id"])
            if layer_of(e["layerId"])["locked"]:
                locked += 1
    blocks, pieces = [], 0
    for b in BLOCKS:
        k = sum(1 for p in b["entities"] if p["kind"] == "text" and p.get("textStyle") == style["id"])
        if k:
            blocks.append(b["id"])
            pieces += k
    return answer(style["name"], follow, counted(names, values), locked=locked, blocks=blocks, pieces=pieces)


def by_dimension_style(style):
    look = A.look_of(style)
    counts, follow, locked = {}, [], 0
    for e in ENTITIES:
        if e["kind"] == "dimension" and e.get("dimStyle") == style["id"]:
            mine = {k: e[k] for k in A.LOOK if k in e}
            for k in set(look) | set(mine):
                if k != "dimStyle" and mine.get(k) != look.get(k):
                    counts[k] = counts.get(k, 0) + 1
            follow.append(e["id"])
            if layer_of(e["layerId"])["locked"]:
                locked += 1
    own = [{"property": k, "count": counts[k]} for k in sorted(counts)]
    return answer(style["name"], follow, own, locked=locked)


# ── The cases ────────────────────────────────────────────────────────────────

UNCHANGED = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}


def completed(output):
    return {"status": "completed", "output": output, "warnings": []}


def failed(code, message):
    return {"status": "failed", "error": {"code": code, "message": message, "path": "id"}}


def case(name, input_, result, op="execute"):
    return {"name": name, "steps": [{"op": op, "input": input_, "result": result, "expect": UNCHANGED}]}


def cases():
    out = [
        case("Katman: duvar; kendi değerli nesneler sayılır, blok parçası ve onu gösteren iç içe yerleştirmeler, eski görünüşü saklayan durum",
             {"target": "layer", "id": "duvar"}, completed(by_layer("duvar"))),
        case("Katman: aks; çizgi tipi kendi olan sayılır", {"target": "layer", "id": "aks"}, completed(by_layer("aks"))),
        case("Katman: yazı; yazılar, ölçüler ve yerleştirme izler", {"target": "layer", "id": "yazi"}, completed(by_layer("yazi"))),
        case("Katman: kilitli; katmanın görünüşü kilitli katmana da yansır", {"target": "layer", "id": "kilitli"},
             completed(by_layer("kilitli"))),
        case("Katman: boş katman; hiçbir şeye ulaşmaz", {"target": "layer", "id": "bos"}, completed(by_layer("bos"))),
        case("Katman: plan aynı cevabı verir", {"target": "layer", "id": "duvar"}, completed(by_layer("duvar")), op="plan"),
        case("Katman: doğrula yazmaz", {"target": "layer", "id": "duvar"}, {"status": "completed", "output": None, "warnings": []},
             op="validate"),
        case("Çizgi tipi: EKSEN; kendi deseniyle çizilen ve desenli katmanın nesneleri, katmana göre blok parçası, durum",
             {"target": "lineType", "id": "EKSEN"}, completed(by_line_type("EKSEN"))),
        case("Çizgi tipi: ad harf büyüklüğüne ve kenar boşluklarına bakmaz", {"target": "lineType", "id": " eksen "},
             completed(by_line_type("EKSEN"))),
        case("Çizgi tipi: kullanılmayan desen", {"target": "lineType", "id": "BOS"}, completed(by_line_type("BOS"))),
        case("Yazı stili: Ada no; kendi kalınlığı ve yazı tipi olan, kilitli katmandaki ve blok parçası",
             {"target": "textStyle", "id": A.ADA["id"]}, completed(by_text_style(A.ADA))),
        case("Yazı stili: kullanılmayan", {"target": "textStyle", "id": A.YOL["id"]}, completed(by_text_style(A.YOL))),
        case("Ölçü stili: Mimari; kendi basamağı olan ölçü", {"target": "dimensionStyle", "id": A.OLCU["id"]},
             completed(by_dimension_style(A.OLCU))),
        case("Ölçü stili: plan aynı cevabı verir", {"target": "dimensionStyle", "id": A.OLCU["id"]},
             completed(by_dimension_style(A.OLCU)), op="plan"),
        case("Ret: kimlik yok", {"target": "layer", "id": ""},
             failed("empty_name", "Etkisi sorulan katmanın, stilin ya da çizgi tipinin kimliğini verin (id).")),
        case("Ret: kimlik yalnız boşluk", {"target": "lineType", "id": "  "},
             failed("empty_name", "Etkisi sorulan katmanın, stilin ya da çizgi tipinin kimliğini verin (id).")),
        case("Ret: katman çizimde yok", {"target": "layer", "id": "yok"}, failed("layer_not_found", "“yok” kimlikli katman çizimde yok.")),
        case("Ret: yazı stili projede yok", {"target": "textStyle", "id": "yok"},
             failed("style_not_found", "“yok” kimlikli yazı stili projede yok.")),
        case("Ret: yazı stilinin kimliği ölçü stili diye verilmiş", {"target": "dimensionStyle", "id": A.ADA["id"]},
             failed("style_not_found", f"“{A.ADA['id']}” kimlikli ölçü stili projede yok.")),
        case("Ret: çizgi tipi projede yok", {"target": "lineType", "id": "ZIKZAK"},
             failed("line_type_not_found", "“ZIKZAK” adlı çizgi tipi projede yok.")),
    ]
    return out


def build():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.styles.impact",
        "commandVersion": 1,
        "title": "Stil değişikliğinin etkisi",
        "note": "Written by scripts/fixtures/style_impact_cases.py from the contract's rules and docs/adr/0312 §4, 0183, 0314 §4; "
                "do not edit by hand.",
        "setup": setup(),
        "cases": cases(),
    }


def main() -> int:
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil: python3 {Path(__file__).relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)}: {len(build()['cases'])} durum tutarlı.")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
