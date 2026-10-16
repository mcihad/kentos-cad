#!/usr/bin/env python3
"""The drawing commands write a new object's attributes by its layer's fields (TODOS.md FORM-O, ADR 0199 §2): each
given value in its canonical text (a label its code, Evet true, GG.AA.YYYY a date, a multiple field's codes in the
list's order), an empty one left out, the first that breaks its field's rules refused (`invalid_attribute`, a required
field's empty value `attribute_required`, at `attrs.<name>`, in the names' order), keys no field names kept as given,
and the fields' defaults where none is given.

The values' rules are scripts/fixtures/layer_field_cases.py's (written from ADR 0199 without KentOS code); this script
writes fixtures/commands/v1/<command>.fields.json for every drawing command. `--check` compares instead of writing.
"""

import importlib.util
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("layer_field_cases", ROOT / "scripts/fixtures/layer_field_cases.py")
lf = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(lf)

E0, N0 = 487000, 4420000
STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}
FIELDS = [
    {"name": "Ad", "kind": "text", "required": True},
    {"name": "Tür", "kind": "text", "values": [{"code": "agac", "label": "Ağaç"}, {"code": "cali", "label": "Çalı"}],
     "default": "agac"},
    {"name": "Boy", "kind": "decimal", "scale": 1, "min": "0", "max": "40"},
    {"name": "Dikim", "kind": "date"},
    {"name": "Korunan", "kind": "boolean"},
    {"name": "Bakım", "kind": "text", "multiple": True,
     "values": [{"code": "budama", "label": "Budama"}, {"code": "sulama", "label": "Sulama"}]},
    {"name": "Kod", "kind": "text", "length": 6, "default": "A-1"},
]


def layer(i, name, fields=None, locked=False):
    out = {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True, "style": STYLE,
           "children": []}
    if fields:
        out["fields"] = fields
    return out


SETUP = {"format": "kentos.document", "version": 1, "name": "Alanlar",
         "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                      "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
         "origin": {"x": E0, "y": N0},
         "layers": [layer("agac", "Ağaç", FIELDS), layer("serbest", "Serbest")],
         "activeLayer": "agac", "entities": [], "styles": {"items": [], "categories": []}}


def written(fields, given):
    """The attributes written, or the refusal: (attrs, None) or (None, (code, message, path))."""
    out = {}
    for key in sorted(given):
        f = next((x for x in fields if x["name"] == key), None)
        if f is None:
            out[key] = given[key]
            continue
        r = lf.check(f, given[key])
        if "error" in r:
            code = "attribute_required" if r["error"] == "required" else "invalid_attribute"
            return None, (code, r["message"], f"attrs.{key}")
        if r["value"]:
            out[key] = r["value"]
    for f in fields:
        if "default" in f and f["name"] not in out:
            out[f["name"]] = f["default"]
    return out, None


SHAPES = {
    "cad.point.create": lambda: {"p": {"x": E0 + 10, "y": N0 + 10}},
    "cad.line.create": lambda: {"a": {"x": E0, "y": N0}, "b": {"x": E0 + 20, "y": N0 + 5}},
    "cad.polyline.create": lambda: {"pts": [{"x": E0, "y": N0}, {"x": E0 + 20, "y": N0}, {"x": E0 + 20, "y": N0 + 12.5}]},
    "cad.polygon.create": lambda: {"pts": [{"x": E0, "y": N0}, {"x": E0 + 10, "y": N0}, {"x": E0 + 10, "y": N0 + 8}]},
    "cad.circle.create": lambda: {"c": {"x": E0 + 20, "y": N0 + 15}, "r": 7.5},
    "cad.arc.create": lambda: {"c": {"x": E0 + 20, "y": N0 + 15}, "r": 10, "a0": 0, "a1": 1.5707963267948966},
    "cad.nurbs.create": lambda: {"by": {"kind": "fit", "pts": [{"x": E0, "y": N0}, {"x": E0 + 12, "y": N0 + 7.5},
                                                              {"x": E0 + 25, "y": N0 + 3}, {"x": E0 + 38, "y": N0 + 11}]}},
    "cad.rectangle.create": lambda: {"frame": {"kind": "corners", "a": {"x": E0 + 10, "y": N0 + 10},
                                               "b": {"x": E0 + 30, "y": N0 + 22.5}}},
    "cad.polygon.regular": lambda: {"sides": 6, "frame": {"kind": "inscribed", "center": {"x": E0 + 20, "y": N0 + 20},
                                                          "corner": {"x": E0 + 25, "y": N0 + 20}}},
}


def case(command, name, given, layer_id="agac"):
    fields = FIELDS if layer_id == "agac" else []
    inp = {"layerId": layer_id, **SHAPES[command]()}
    if given is not None:
        inp["attrs"] = given
    attrs, refused = written(fields, given or {})
    if refused:
        code, message, path = refused
        result = {"status": "failed", "error": {"code": code, "message": message, "path": path}}
        expect = {"ids": [], "canUndo": False, "dirty": False, "revision": "same"}
    else:
        result = {"status": "completed", "output": {"uid": "$uid", "id": 1, "revision": "$current"}, "warnings": []}
        expect = {"ids": [1], "fields": {"1": {"attrs": attrs}}, "canUndo": True, "dirty": True, "revision": "changed"}
    return {"name": name, "steps": [{"op": "execute", "input": inp, "result": result, "expect": expect}]}


def cases_of(c):
    return [
        case(c, "değersiz nokta alanların varsayılanlarını alır", None),
        case(c, "listenin etiketi koduna çevrilir", {"Ad": "Çınar", "Tür": "Çalı"}),
        case(c, "kod olduğu gibi kalır", {"Ad": "Çınar", "Tür": "cali"}),
        case(c, "ondalık sayı tek biçime: virgül ve sıfırlar", {"Ad": "Çınar", "Boy": "12,50"}),
        case(c, "tarih GG.AA.YYYY'den", {"Ad": "Çınar", "Dikim": "5.11.2026"}),
        case(c, "evet/hayır Evet'ten", {"Ad": "Çınar", "Korunan": "Evet"}),
        case(c, "çok değerli alan listenin sırasıyla", {"Ad": "Çınar", "Bakım": "Sulama; budama; sulama"}),
        case(c, "boş değer yazılmaz, alanın varsayılanı gelir", {"Ad": "Çınar", "Kod": "  "}),
        case(c, "verilen değer varsayılanın yerine", {"Ad": "Çınar", "Kod": "B-2"}),
        case(c, "alanı olmayan anahtar olduğu gibi", {"Ad": "Çınar", "Not": " serbest "}),
        case(c, "alansız katmanda her şey olduğu gibi, varsayılan yok", {"Tür": "Çalı", "x": "1"}, layer_id="serbest"),
        case(c, "listede olmayan değer reddedilir", {"Ad": "Çınar", "Tür": "Ağaççık"}),
        case(c, "aralık dışı boy reddedilir", {"Ad": "Çınar", "Boy": "41"}),
        case(c, "takvimde olmayan gün reddedilir", {"Ad": "Çınar", "Dikim": "31.04.2026"}),
        case(c, "evet/hayır olmayan değer reddedilir", {"Ad": "Çınar", "Korunan": "belki"}),
        case(c, "uzun kod reddedilir", {"Ad": "Çınar", "Kod": "ABCDEFG"}),
        case(c, "zorunlu alanın boş değeri reddedilir", {"Ad": " "}),
        case(c, "zorunlu alan verilmezse reddedilmez", {"Tür": "agac"}),
        case(c, "çok değerli alanda listede olmayan reddedilir", {"Ad": "Çınar", "Bakım": "budama; gübre"}),
        case(c, "retler adların sırasıyla: Boy, Dikim'den önce", {"Ad": "Çınar", "Dikim": "x", "Boy": "x"}),
        case(c, "ondalık basamak sınırı", {"Ad": "Çınar", "Boy": "1.25"}),
    ]


def command_file(command, cases):
    return {
        "format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1,
        "title": "Yeni nesnenin öznitelikleri katmanın alanlarıyla",
        "note": "TODOS.md FORM-O, ADR 0199 §2: çizim komutu verilen öznitelikleri alanların kuralıyla tek biçime çevirir, "
                "kuralı bozanı reddeder, verilmeyene alanın varsayılanını yazar. Kural ve beklenenler "
                "scripts/fixtures/primitive_fields_cases.py'de, değerlerin kuralı layer_field_cases.py'de KentOS kodu "
                "olmadan yazılmıştır.",
        "setup": SETUP, "cases": cases,
    }


def outputs():
    d = ROOT / "fixtures/commands/v1"
    return {d / f"{c}.fields.json": command_file(c, cases_of(c)) for c in SHAPES}


def main():
    check = "--check" in sys.argv[1:]
    bad = 0
    for path, data in outputs().items():
        text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
        if check:
            if not path.exists() or path.read_text("utf-8") != text:
                print(f"{path.relative_to(ROOT)}: güncel değil", file=sys.stderr)
                bad = 1
            else:
                print(f"{path.relative_to(ROOT)}: {len(data['cases'])} durum, güncel.")
        else:
            path.write_text(text, encoding="utf-8")
            print(f"{path.relative_to(ROOT)}: {len(data['cases'])} durum, yazıldı.")
    return bad


if __name__ == "__main__":
    sys.exit(main())
