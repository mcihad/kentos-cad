#!/usr/bin/env python3
"""Independent reference of Öznitelik tablosu's Dışa aktar (docs/adr/0223 §3):
the attribute table as a workbook's sheet.

    python3 scripts/fixtures/feature_export_cases.py           # writes the file
    python3 scripts/fixtures/feature_export_cases.py --check   # writes nothing; compares

Writes fixtures/feature-table/v1/export.json from the rule alone, with the
layer fields' rule of layer_field_cases.py (itself written from docs/adr/0199
without KentOS code). The desktop (`kentos_interaction::feature_table::
workbook_sheet`) and the web (`model/featureTable.ts`'s `featureSheet`) give
the same sheet; the workbook writer (docs/adr/0223 §2) then writes it.

- The sheet's name is the layer's; its rows the table's rows shown, in the
  order shown (here the drawing's).
- Its columns: Tür (text), the layer's fields by their names (not their
  aliases), then the keys no field names in the natural order (text). A
  field's column is written by its kind: a whole number a number with no
  fraction digits, a decimal number a number with the field's fraction
  digits (none: as written), a date a date, yes or no a boolean, text text;
  a field with a value list is text (its labels).
- A cell: Tür the object's kind as the table names it; a field's value that
  keeps the field's rules its canonical text (a value list's: its label); a
  value that does not keep them as written; a key no field names as written.
"""
import json
import sys
from functools import cmp_to_key
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import layer_field_cases as lf  # noqa: E402

ROOT = HERE.parents[1]
OUT = ROOT / "fixtures/feature-table/v1/export.json"
KIND = {"point": "Nokta", "polygon": "Kapalı alan"}

FIELDS = [
    {"name": "Ada", "alias": "Ada no", "kind": "integer"},
    {"name": "Alan", "kind": "decimal", "scale": 2},
    {"name": "Tarih", "kind": "date"},
    {"name": "Ruhsat", "kind": "boolean"},
    {"name": "Kullanım", "kind": "text", "values": [{"code": "K", "label": "Konut"}, {"code": "T", "label": "Ticaret"}]},
    {"name": "Malik", "kind": "text"},
]

OBJECTS = [
    {"kind": "point", "attrs": {"Ada": "101", "Alan": "1340.13", "Tarih": "2024-10-01", "Ruhsat": "true", "Kullanım": "K", "Malik": "Ayşe Öztürk", "Not": "köşe"}},
    {"kind": "polygon", "attrs": {"Ada": "12a", "Alan": "12,5", "Ruhsat": "evet", "Kullanım": "T", "Malik": "  Şükrü  "}},
    {"kind": "point", "attrs": {"Tarih": "07.10.2026", "Ruhsat": "hayır", "Kullanım": "Ticaret", "Eski no": "007"}},
    {"kind": "polygon", "attrs": {"Alan": "-1.5", "Kullanım": "X", "Tarih": "2023-02-29", "Not": ""}},
    {"kind": "point", "attrs": {}},
]


def column_kind(f):
    if f.get("values"):
        return {"kind": "text"}
    if f["kind"] == "integer":
        return {"kind": "number", "decimals": 0}
    if f["kind"] == "decimal":
        return {"kind": "number", **({"decimals": f["scale"]} if f.get("scale") is not None else {})}
    return {"kind": {"date": "date", "boolean": "boolean"}.get(f["kind"], "text")}


def sheet(name, fields, objects):
    named = {f["name"] for f in fields}
    others = []
    for o in objects:
        for k in o["attrs"]:
            if k not in named and k not in others:
                others.append(k)
    others = sorted(others, key=cmp_to_key(lf.natural_cmp))
    columns = [{"name": "Tür", "kind": "text"}] + [{"name": f["name"], **column_kind(f)} for f in fields] + [{"name": k, "kind": "text"} for k in others]
    rows = []
    for o in objects:
        row = [KIND[o["kind"]]]
        for f in fields:
            raw = o["attrs"].get(f["name"])
            r = lf.check(f, raw or "")
            if "error" in r:
                row.append(raw or "")
            elif r["value"] == "":
                row.append("")
            elif f.get("values"):
                row.append(lf.display(f, r["value"]))
            else:
                row.append(r["value"])
        row += [o["attrs"].get(k, "") for k in others]
        rows.append(row)
    return {"name": name, "columns": columns, "rows": rows}


def build():
    return {
        "format": "kentos.feature-export-cases",
        "version": 1,
        "generatedBy": "scripts/fixtures/feature_export_cases.py",
        "title": "Öznitelik tablosunun dışa aktarılan sayfası (ADR 0223 §3)",
        "cases": [
            {"title": "Alanlar türleriyle, değer listesi etiketiyle, kuralına uymayan değer yazıldığı gibi, alansız anahtarlar doğal sırayla",
             "layer": "Parseller", "fields": FIELDS, "objects": OBJECTS, "expect": sheet("Parseller", FIELDS, OBJECTS)},
            {"title": "Alanı olmayan katman: Tür ve anahtarlar",
             "layer": "Noktalar", "fields": [], "objects": OBJECTS[:3], "expect": sheet("Noktalar", [], OBJECTS[:3])},
        ],
    }


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)}: diskteki dosya kurallardan üretilenle aynı değil", file=sys.stderr)
            return 1
        print("feature export cases match")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"written: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
