#!/usr/bin/env python3
"""Formların izinin çizimi (docs/adr/0401): bir sokağın dört binası; Bina katmanının alanları (Durum değer listesiyle ve
zorunlu, Kat kararlı kimlikle, Açıklama, Tarih, Onaylı) ve katmanın “Bina denetimi” formu (taslağı ve ilk yayını:
Genel ve Ayrıntı sekmeleri, açıklamayı inceleme gereken binada gösterip isteyen kural, iki sütunlu Denetim grubu). B-1
incelenecek, açıklaması boş; B-2 iyi durumda.

    python3 scripts/fixtures/forms_scene.py          # fixtures/interaction/v1/forms.kcad'i yazar
    python3 scripts/fixtures/forms_scene.py --check  # hiçbir şey yazmaz; karşılaştırır

KentOS kodu kullanılmaz: form ve alanlar forms_cases.py'nin başvurusundan, yayının özeti kanonik JSON'un SHA-256'sından.
"""

import importlib.util
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "interaction" / "v1" / "forms.kcad"
E, N = 487000, 4420000

spec = importlib.util.spec_from_file_location("forms_cases", ROOT / "scripts/fixtures/forms_cases.py")
fc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fc)


def P(x, y):
    return {"x": E + x, "y": N + y}


def drawing():
    style = {"color": "#3E63DD", "lineType": "continuous", "lineWeight": 0.35}
    bina = {"id": "bina", "name": "Bina", "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": style, "children": [], "fields": fc.FIELDS}
    yol = {"id": "yol", "name": "Yol", "type": "layer", "visible": True, "locked": False, "expanded": True,
           "style": {"color": "#ADB5BD", "lineType": "continuous", "lineWeight": 0.5}, "children": []}
    entities = []
    records = [({"Durum": "inceleme", "Kat": "3", "Kod": "B-12"}, "B-1"), ({"Durum": "iyi", "Kat": "2", "Onaylı": "true"}, "B-2"),
               ({}, None), ({}, None)]
    for k, ((x, y), (attrs, label)) in enumerate(zip([(0, 0), (24, 0), (0, 20), (24, 20)], records)):
        e = {"kind": "polygon", "id": k + 1, "layerId": "bina", "attrs": attrs,
             "pts": [P(x, y), P(x + 18, y), P(x + 18, y + 14), P(x, y + 14)]}
        if label:
            e["label"] = label
        entities.append(e)
    entities.append({"kind": "line", "id": 5, "layerId": "yol", "attrs": {"Ad": "Gül Sokak"}, "a": P(-8, -6), "b": P(50, -6)})
    form = {"id": "bina-denetimi", "name": "Bina denetimi", "layer": "bina", "draft": fc.BODY,
            "releases": [fc.release_of(fc.BODY, 1, "İlk yayın")]}
    assert not fc.errors(fc.preflight(fc.BODY, fc.FIELDS))
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Formlar",
        "settings": {"srid": 5256, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 500, "workspace": "gis", "drawingFont": "barlow", "forms": [form]},
        "origin": {"x": E, "y": N},
        "layers": [bina, yol],
        "activeLayer": "bina",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }


def main():
    text = json.dumps(drawing(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check olmadan çalıştırıp farkı okuyun.")
            return 1
        print(f"{OUT.relative_to(ROOT)}: güncel.")
        return 0
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
