#!/usr/bin/env python3
"""Writes fixtures/style/v1/template-form.json: the Şablon düzenleyici's form
rules (docs/adr/0176 §4), worked out here from the rule, in plain Python,
without KentOS code.

The form holds texts and choices; the rule makes the library item's fields
(name, category path, description) and the template from them, or says what
is wrong, every problem in the order of the fields:

- Ad is trimmed and needed (“Şablonun adı boş; bir ad yazın.”).
- Kategori and the layer's Gruplar are split on “/”, each part trimmed, the
  empty ones left out.
- Açıklama, Etiket and the point's Ad and Kod are trimmed; empty, they are
  not written.
- Yöntem is written only for a tool that has methods, and only when chosen.
- Katman (its name) is trimmed and needed (“Katmanın adı boş; şablonun
  çizeceği katmanı yazın ya da seçin.”).
- The layer's colour and line type, the colour and the symbol are written
  when chosen; an empty choice is the default (“Katmana göre”).
- A weight is a number with a point or a comma, 0 to 100 mm; empty, not
  written (“Katmanın kalınlığı …” and “Kalınlık 0 ile 100 mm arasında bir
  sayı olmalı.”).
- Öznitelikler: rows with neither a name nor a value are left out; a row
  with a value and no name is said by its place (“3. öznitelik satırının adı
  boş.”), a name written twice by its name (““Tür” özniteliği iki kez
  yazılmış.”); names are trimmed, values kept as typed; written by name.
- A text template's Yükseklik is needed, a number above zero (“Yazının
  yüksekliği sıfırdan büyük bir sayı olmalı.”); its Hiza is written when
  chosen, Zemin when on. A block template's Blok is trimmed and needed
  (“Blok şablonunun bloğu yok; yerleştirilecek bloğu seçin.”). Those of the
  other tools are not written.

And back: a library item's fields and template as the form shows them
(categories and groups joined with “ / ”, numbers as JavaScript writes them,
the attributes by name). The desktop (`kentos_native_style::template_form`)
and the web (`model/templateForm.ts`) both read these cases.

    python3 scripts/fixtures/template_form_cases.py          # writes the cases
    python3 scripts/fixtures/template_form_cases.py --check  # fails when they differ
"""

import json
import re
import sys

PATH = "fixtures/style/v1/template-form.json"

METHODS = {"circle": ["2N", "3N", "TTY", "TTT"]}
NUMBER = re.compile(r"^\d+(?:[.,]\d+)?$")


def parts(text):
    return [p.strip() for p in text.split("/") if p.strip()]


def number(text):
    t = text.strip()
    if not NUMBER.match(t):
        return None
    return float(t.replace(",", "."))


def js(n):
    """A number as JavaScript's String(n) writes it, for these cases' numbers."""
    return str(int(n)) if n == int(n) else repr(n)


def from_form(f):
    issues = []
    name = f["name"].strip()
    if not name:
        issues.append("Şablonun adı boş; bir ad yazın.")
    tool = f["tool"]
    t = {"tool": tool}
    if tool in METHODS and f["method"]:
        t["method"] = f["method"]
    layer_name = f["layerName"].strip()
    if not layer_name:
        issues.append("Katmanın adı boş; şablonun çizeceği katmanı yazın ya da seçin.")
    layer = {"path": parts(f["layerGroups"]), "name": layer_name}
    if f["layerColor"]:
        layer["color"] = f["layerColor"]
    if f["layerLineType"]:
        layer["lineType"] = f["layerLineType"]
    if f["layerWeight"].strip():
        w = number(f["layerWeight"])
        if w is None or w > 100:
            issues.append("Katmanın kalınlığı 0 ile 100 mm arasında bir sayı olmalı.")
        else:
            layer["lineWeight"] = w
    t["layer"] = layer
    if f["color"]:
        t["color"] = f["color"]
    if f["weight"].strip():
        w = number(f["weight"])
        if w is None or w > 100:
            issues.append("Kalınlık 0 ile 100 mm arasında bir sayı olmalı.")
        else:
            t["lineWeight"] = w
    if f["symbol"]:
        t["symbol"] = f["symbol"]
    attrs = {}
    for i, (key, value) in enumerate(f["attrs"]):
        key = key.strip()
        if not key and not value.strip():
            continue
        if not key:
            issues.append(f"{i + 1}. öznitelik satırının adı boş.")
        elif key in attrs:
            issues.append(f"“{key}” özniteliği iki kez yazılmış.")
        else:
            attrs[key] = value
    if attrs:
        t["attrs"] = dict(sorted(attrs.items()))
    label = f["label"].strip()
    if label:
        t["label"] = label
    if tool == "point":
        point = {}
        if f["pointName"].strip():
            point["name"] = f["pointName"].strip()
        if f["pointCode"].strip():
            point["code"] = f["pointCode"].strip()
        if point:
            t["point"] = point
    if tool == "text":
        h = number(f["textHeight"]) if f["textHeight"].strip() else None
        if h is None or h <= 0:
            issues.append("Yazının yüksekliği sıfırdan büyük bir sayı olmalı.")
        else:
            text = {"height": h}
            if f["textAlign"]:
                text["align"] = f["textAlign"]
            if f["textMask"]:
                text["mask"] = True
            t["text"] = text
    if tool == "blockInsert":
        block = f["block"].strip()
        if not block:
            issues.append("Blok şablonunun bloğu yok; yerleştirilecek bloğu seçin.")
        else:
            t["block"] = block
    if issues:
        return {"issues": issues}
    out = {"name": name, "path": parts(f["category"]), "template": t}
    description = f["description"].strip()
    if description:
        out["description"] = description
    return out


def to_form(item):
    t = item["template"]
    layer = t["layer"]
    point = t.get("point", {})
    text = t.get("text", {})
    return {
        "name": item["name"],
        "category": " / ".join(item.get("path", [])),
        "description": item.get("description", ""),
        "tool": t["tool"],
        "method": t.get("method", ""),
        "layerGroups": " / ".join(layer["path"]),
        "layerName": layer["name"],
        "layerColor": layer.get("color", ""),
        "layerLineType": layer.get("lineType", ""),
        "layerWeight": js(layer["lineWeight"]) if "lineWeight" in layer else "",
        "color": t.get("color", ""),
        "weight": js(t["lineWeight"]) if "lineWeight" in t else "",
        "symbol": t.get("symbol", ""),
        "attrs": [[k, v] for k, v in sorted(t.get("attrs", {}).items())],
        "label": t.get("label", ""),
        "pointName": point.get("name", ""),
        "pointCode": point.get("code", ""),
        "textHeight": js(text["height"]) if "height" in text else "",
        "textAlign": text.get("align", ""),
        "textMask": bool(text.get("mask", False)),
        "block": t.get("block", ""),
    }


EMPTY = {
    "name": "", "category": "", "description": "", "tool": "polygon", "method": "", "layerGroups": "", "layerName": "",
    "layerColor": "", "layerLineType": "", "layerWeight": "", "color": "", "weight": "", "symbol": "", "attrs": [],
    "label": "", "pointName": "", "pointCode": "", "textHeight": "", "textAlign": "", "textMask": False, "block": "",
}


def form(**fields):
    return {**EMPTY, **fields}


PARCEL = form(name=" Parsel sınırı ", category="Kadastro /  / Sınırlar ", description=" Tescilli parsel ", layerGroups=" Kadastro ",
              layerName=" Parsel ", layerColor="#E5484D", layerWeight="0,35", weight="0.5", symbol="temel.alan.kenar-ici",
              attrs=[["Tür", "Parsel"], ["", ""], [" Ada ", " 101 "]], label=" P ")
FORM_CASES = [
    ("kapalı alan: adlar ve yollar kırpılır, boş kategori parçası düşer, virgüllü ondalık, öznitelikler adlarıyla", PARCEL),
    ("en az: ad ve katman", form(name="Bina", layerName="Bina", tool="rectangle")),
    ("daire yöntemi yazılır", form(name="Ağaç", layerName="Bitki", tool="circle", method="2N")),
    ("yöntemi olmayan araçta yöntem yazılmaz", form(name="Çit", layerName="Çit", tool="polyline", method="2N")),
    ("nokta: ad ve kod; yazı ve blok alanları yazılmaz", form(name="Poligon noktası", layerName="Nokta", tool="point", pointName=" P1 ", pointCode="PN", textHeight="2", block="Rögar")),
    ("adsız, kodsuz nokta", form(name="Nokta", layerName="Nokta", tool="point")),
    ("yazı: yükseklik, hiza ve zemin", form(name="Ada numarası", layerName="Yazı", tool="text", textHeight="2,5", textAlign="middleCenter", textMask=True)),
    ("blok: bloğun adı kırpılır", form(name="Rögar", layerName="Altyapı", tool="blockInsert", block=" Rögar kapağı ")),
    ("katmanın görünüşü: renk, çizgi tipi ve kalınlık", form(name="Yol", layerName="Yol", tool="polyline", layerColor="ink", layerLineType="dashed", layerWeight="0")),
    ("boş ad ve boş katman", form(tool="line")),
    ("yalnız boşluk ad", form(name="   ", layerName="Çizim", tool="line")),
    ("sayı olmayan kalınlıklar", form(name="X", layerName="Y", layerWeight="kalın", weight="0.5mm")),
    ("100 mm'den kalın", form(name="X", layerName="Y", weight="101")),
    ("eksi kalınlık", form(name="X", layerName="Y", weight="-1")),
    ("öznitelik: değeri olan adsız satır ve iki kez yazılan ad", form(name="X", layerName="Y", attrs=[["Tür", "A"], ["", "B"], [" Tür ", "C"]])),
    ("yazının yüksekliği yok", form(name="Not", layerName="Yazı", tool="text")),
    ("yazının yüksekliği sıfır", form(name="Not", layerName="Yazı", tool="text", textHeight="0")),
    ("blok şablonunun bloğu yok", form(name="Rögar", layerName="Altyapı", tool="blockInsert", block="  ")),
    ("her sorun alanların sırasıyla", form(tool="text", weight="x", attrs=[["", "1"]])),
]
ITEMS = [
    ("kapalı alan, bütün alanlarıyla", {"name": "Parsel sınırı", "path": ["Kadastro", "Sınırlar"], "description": "Tescilli parsel", "template": {
        "tool": "polygon", "layer": {"path": ["Kadastro"], "name": "Parsel", "color": "#E5484D", "lineType": "dashed", "lineWeight": 0.35},
        "color": "#7A5C3E", "lineWeight": 0.5, "symbol": "temel.alan.kenar-ici", "attrs": {"Tür": "Parsel", "Ada": "101"}, "label": "P"}}),
    ("en az alanlı daire, yöntemiyle", {"name": "Ağaç", "path": [], "template": {"tool": "circle", "method": "TTY", "layer": {"path": [], "name": "Bitki"}}}),
    ("nokta", {"name": "Poligon noktası", "path": ["Ölçme"], "template": {"tool": "point", "layer": {"path": [], "name": "Nokta"}, "point": {"name": "P1", "code": "PN"}}}),
    ("yazı", {"name": "Ada numarası", "path": [], "template": {"tool": "text", "layer": {"path": [], "name": "Yazı"}, "text": {"height": 2.5, "align": "topLeft", "mask": True}}}),
    ("blok", {"name": "Rögar", "path": [], "template": {"tool": "blockInsert", "layer": {"path": ["Altyapı"], "name": "Rögar"}, "block": "Rögar kapağı", "lineWeight": 0.0}}),
]


def build():
    to_template = [{"name": name, "form": f, "result": from_form(f)} for name, f in FORM_CASES]
    to_fields = []
    for name, item in ITEMS:
        fields = to_form(item)
        # The form gives back what it was made from (attributes by name).
        back = from_form(fields)
        want = {k: v for k, v in item.items() if k in ("name", "path", "description", "template")}
        want["template"] = {**item["template"], **({"attrs": dict(sorted(item["template"]["attrs"].items()))} if "attrs" in item["template"] else {})}
        assert back == want, (name, back, want)
        to_fields.append({"name": name, "item": item, "form": fields})
    return {
        "format": "kentos.template-form-cases",
        "version": 1,
        "note": (
            "ADR 0176 §4: Şablon düzenleyicinin form kuralı. Ad kırpılır ve gerekir; Kategori ve katmanın Grupları “/” ile bölünür, "
            "parçalar kırpılır, boşlar düşer; Açıklama, Etiket, noktanın Ad ve Kod'u kırpılır, boşsa yazılmaz; Yöntem yalnız yöntemi "
            "olan araçta ve seçiliyse; Katman (adı) kırpılır ve gerekir; seçilmeyen renk, çizgi tipi ve sembol yazılmaz; kalınlık noktalı "
            "ya da virgüllü 0–100 mm sayı, boşsa yazılmaz; öznitelik satırlarından adı ve değeri boş olan düşer, değeri olan adsız satır "
            "yerinden, iki kez yazılan ad adıyla söylenir; adlar kırpılır, değerler yazıldığı gibi, adlarının sırasıyla; yazı şablonunda "
            "Yükseklik gerekir, sıfırdan büyük sayı; Hiza seçiliyse, Zemin açıksa; blok şablonunda Blok kırpılır ve gerekir; başka aracın "
            "alanları yazılmaz. Her sorun alanların sırasıyla. Geri yönde kitaplık öğesi forma: yollar “ / ” ile, sayılar JavaScript'in "
            "yazdığı gibi, öznitelikler adlarıyla. Üretici: scripts/fixtures/template_form_cases.py (KentOS kodu olmadan)."
        ),
        "toTemplate": to_template,
        "toForm": to_fields,
    }


def compact(value):
    return json.dumps(value, ensure_ascii=False, separators=(", ", ": "))


def text_of(doc):
    lines = ["{"]
    for key in ("format", "version", "note"):
        lines.append(f"  {json.dumps(key)}: {compact(doc[key])},")
    lines.append('  "toTemplate": [')
    lines.append(",\n".join(f"    {compact(c)}" for c in doc["toTemplate"]))
    lines.append("  ],")
    lines.append('  "toForm": [')
    lines.append(",\n".join(f"    {compact(c)}" for c in doc["toForm"]))
    lines.append("  ]")
    lines.append("}")
    return "\n".join(lines) + "\n"


def main():
    text = text_of(build())
    if "--check" in sys.argv[1:]:
        with open(PATH, encoding="utf-8") as f:
            if f.read() != text:
                sys.exit(f"{PATH} is not what this script writes: run it without --check and read the difference.")
        print(f"{len(FORM_CASES)} + {len(ITEMS)} cases match")
        return
    with open(PATH, "w", encoding="utf-8") as f:
        f.write(text)
    print(f"{len(FORM_CASES)} + {len(ITEMS)} cases written")


if __name__ == "__main__":
    main()
