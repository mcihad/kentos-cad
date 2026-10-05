#!/usr/bin/env python3
"""Writes fixtures/style/v1/template-from-object.json: the object template made
from a drawn object (Seçili nesneden şablon, docs/adr/0176 §4), worked out here
from the rule, in plain Python, without KentOS code.

The rule: the tool is the object's kind's (a point Nokta, a line Çizgi, a
polyline Çoklu çizgi, a polygon Kapalı alan, a circle Daire, a text Yazı, an
insert Blok ekle); any other kind is refused, said by its name. The layer is
the object's: its name, the names of the groups above it from the top, and
its look (colour, line type, weight) as the layer has it. The object's own
colour, line weight and symbol go with it when it has them, and so do its
attributes and its label; a point's label is the template's first name and
its `Kod` attribute its code, not attributes or a label (docs/adr/0152). A
text gives its height, its alignment when it has one and its mask when it is
on; an insert its block, by name. The template is named after its layer and
has no category. The desktop (`kentos_native_style::object_template::from_object`)
and the web (`model/objectTemplate.ts`'s `templateFromObject`) both read these
cases.

    python3 scripts/fixtures/template_from_object_cases.py          # writes the cases
    python3 scripts/fixtures/template_from_object_cases.py --check  # fails when they differ
"""

import json
import sys

PATH = "fixtures/style/v1/template-from-object.json"

TOOLS = {"point": "point", "line": "line", "polyline": "polyline", "polygon": "polygon", "circle": "circle", "text": "text", "insert": "blockInsert"}
KIND_LABEL = {
    "point": "Nokta", "line": "Çizgi", "polyline": "Çoklu çizgi", "polygon": "Kapalı alan", "circle": "Daire", "arc": "Yay",
    "ellipse": "Elips", "spline": "Eğri", "xline": "Yardımcı çizgi", "ray": "Işın", "text": "Yazı", "dimension": "Ölçü",
    "hatch": "Tarama", "insert": "Blok", "leader": "Kılavuz",
}


def node(id_, name, kind="layer", children=None, color="fg", line_type="continuous", weight=0.25, locked=False):
    return {"id": id_, "name": name, "type": kind, "visible": True, "locked": locked, "expanded": True,
            "style": {"color": color, "lineType": line_type, "lineWeight": weight}, "children": children or []}


def find(nodes, id_, groups=()):
    """The node and the names of the groups above it."""
    for n in nodes:
        if n["id"] == id_:
            return n, list(groups)
        if n["type"] == "group":
            found = find(n["children"], id_, groups + (n["name"],))
            if found:
                return found
    return None


def answer(entity, layers, blocks):
    kind = entity["kind"]
    if kind not in TOOLS:
        return {"refused": f"{KIND_LABEL[kind]} nesnesinden şablon yapılamaz: şablon nokta, çizgi, çoklu çizgi, kapalı alan, daire, yazı ya da blok çizer."}
    layer, groups = find(layers, entity["layerId"])
    style = layer["style"]
    t = {"tool": TOOLS[kind], "layer": {"path": groups, "name": layer["name"], "color": style["color"], "lineType": style["lineType"], "lineWeight": style["lineWeight"]}}
    if entity.get("color") is not None:
        t["color"] = entity["color"]
    if entity.get("lineWeight") is not None:
        t["lineWeight"] = entity["lineWeight"]
    if entity.get("symbol") is not None:
        t["symbol"] = entity["symbol"]
    attrs = dict(entity.get("attrs") or {})
    label = entity.get("label")
    if kind == "point":
        point = {}
        if label:
            point["name"] = label
        if attrs.get("Kod"):
            point["code"] = attrs.pop("Kod")
        label = None
        if point:
            t["point"] = point
    if attrs:
        t["attrs"] = dict(sorted(attrs.items()))
    if label:
        t["label"] = label
    if kind == "text":
        text = {"height": entity["height"]}
        if entity.get("align"):
            text["align"] = entity["align"]
        if entity.get("mask") is True:
            text["mask"] = True
        t["text"] = text
    if kind == "insert":
        t["block"] = next(b["name"] for b in blocks if b["id"] == entity["block"])
    return {"name": layer["name"], "template": t}


P = lambda x, y: {"x": x, "y": y}
LAYERS = [
    node("cizim", "Çizim"),
    node("kadastro", "Kadastro", "group", [
        node("parsel", "Parsel", color="#E5484D", weight=0.35),
        node("tapu", "Tapu", "group", [node("sinir", "Sınır", line_type="dashed", weight=0.5)]),
    ]),
    node("nokta", "Nokta", color="#4F8EF7", weight=0.18),
    node("yazi", "Yazı"),
    node("blok", "Rögar"),
]
BLOCKS = [{"id": "b0c1d2e3-0000-4000-8000-000000000001", "name": "Rögar kapağı"}]
CASES = [
    ("kapalı alan: katmanın yolu ve görünüşü, nesnenin rengi, kalınlığı, sembolü, öznitelikleri ve etiketi",
     {"kind": "polygon", "id": 3, "layerId": "parsel", "color": "#7A5C3E", "lineWeight": 0.5, "symbol": "temel.alan.kenar-ici", "attrs": {"Tür": "Parsel", "Ada": "101"}, "label": "12", "pts": [P(0, 0), P(10, 0), P(10, 10)]}),
    ("katmana göre nesne: rengi, kalınlığı ve sembolü yok; öznitelikleri boşsa yok",
     {"kind": "polygon", "id": 4, "layerId": "cizim", "attrs": {}, "pts": [P(0, 0), P(10, 0), P(10, 10)]}),
    ("iki grup derinde katman: yol en üstten", {"kind": "polyline", "id": 5, "layerId": "sinir", "attrs": {"Tür": "Tapu sınırı"}, "pts": [P(0, 0), P(10, 0)]}),
    ("çizgi", {"kind": "line", "id": 6, "layerId": "cizim", "attrs": {}, "a": P(0, 0), "b": P(5, 5)}),
    ("daire", {"kind": "circle", "id": 7, "layerId": "cizim", "attrs": {"Tür": "Ağaç"}, "c": P(0, 0), "r": 2}),
    ("nokta: etiketi ilk ad, Kod özniteliği kod; ikisi öznitelik ve etiket değil",
     {"kind": "point", "id": 8, "layerId": "nokta", "attrs": {"Kod": "PN", "Tür": "Poligon"}, "label": "P12", "p": P(0, 0)}),
    ("adsız ve kodsuz nokta: nokta bilgisi yok", {"kind": "point", "id": 9, "layerId": "nokta", "attrs": {}, "p": P(0, 0)}),
    ("yalnız kodlu nokta", {"kind": "point", "id": 10, "layerId": "nokta", "attrs": {"Kod": "SN"}, "p": P(0, 0)}),
    ("yazı: yüksekliği, hizası ve zemini", {"kind": "text", "id": 11, "layerId": "yazi", "attrs": {}, "p": P(0, 0), "text": "Ada 101", "height": 2.5, "rotation": 0, "align": "middleCenter", "mask": True}),
    ("hizasız, zeminsiz yazı: yalnız yüksekliği", {"kind": "text", "id": 12, "layerId": "yazi", "attrs": {}, "p": P(0, 0), "text": "Not", "height": 1.8, "rotation": 0}),
    ("blok yerleştirmesi: bloğu adıyla", {"kind": "insert", "id": 13, "layerId": "blok", "attrs": {"NO": "R-9"}, "block": BLOCKS[0]["id"], "p": P(0, 0), "scale": 1, "rotation": 0}),
    ("yay reddedilir", {"kind": "arc", "id": 14, "layerId": "cizim", "attrs": {}, "c": P(0, 0), "r": 5, "a0": 0, "a1": 1}),
    ("tarama reddedilir", {"kind": "hatch", "id": 15, "layerId": "cizim", "attrs": {}, "ring": [P(0, 0), P(4, 0), P(4, 4)], "pattern": {"type": "solid", "angle": 0, "spacing": 1}}),
    ("ölçü reddedilir", {"kind": "dimension", "id": 16, "layerId": "cizim", "attrs": {}, "a": P(0, 0), "b": P(10, 0), "offset": 2, "height": 2.5}),
]


def build():
    cases = [{"name": name, "entity": entity, "result": answer(entity, LAYERS, BLOCKS)} for name, entity in CASES]
    return {
        "format": "kentos.template-from-object-cases",
        "version": 1,
        "note": (
            "ADR 0176 §4: Seçili nesneden şablon. Araç nesnenin türünündür (nokta Nokta, çizgi Çizgi, çoklu çizgi Çoklu çizgi, kapalı "
            "alan Kapalı alan, daire Daire, yazı Yazı, blok yerleştirmesi Blok ekle); başka tür adıyla söylenip reddedilir. Katman nesnenin "
            "katmanıdır: adı, üstündeki grupların adları en üstten ve katmanın görünüşü (renk, çizgi tipi, kalınlık). Nesnenin kendi rengi, "
            "kalınlığı ve sembolü varsa, öznitelikleri ve etiketi de şablona geçer; noktanın etiketi şablonun ilk adı, Kod özniteliği kodudur, "
            "öznitelik ya da etiket değil (ADR 0152). Yazı yüksekliğini, varsa hizasını, açıksa zeminini verir; yerleştirme bloğunu adıyla. "
            "Şablonun adı katmanınkidir, kategorisi yoktur. Öznitelikler adlarının sırasıyladır. Üretici: "
            "scripts/fixtures/template_from_object_cases.py (KentOS kodu olmadan)."
        ),
        "layers": LAYERS,
        "blocks": BLOCKS,
        "cases": cases,
    }


def compact(value):
    return json.dumps(value, ensure_ascii=False, separators=(", ", ": "))


def text_of(doc):
    lines = ["{"]
    for key in ("format", "version", "note", "layers", "blocks"):
        lines.append(f"  {json.dumps(key)}: {compact(doc[key])},")
    lines.append('  "cases": [')
    lines.append(",\n".join(f"    {compact(c)}" for c in doc["cases"]))
    lines.append("  ]")
    lines.append("}")
    return "\n".join(lines) + "\n"


def main():
    text = text_of(build())
    if "--check" in sys.argv[1:]:
        with open(PATH, encoding="utf-8") as f:
            if f.read() != text:
                sys.exit(f"{PATH} is not what this script writes: run it without --check and read the difference.")
        print(f"{len(CASES)} cases match")
        return
    with open(PATH, "w", encoding="utf-8") as f:
        f.write(text)
    print(f"{len(CASES)} cases written")


if __name__ == "__main__":
    main()
