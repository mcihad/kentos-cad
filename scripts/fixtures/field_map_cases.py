#!/usr/bin/env python3
"""Independent reference of the Alan eşleme field (docs/adr/0216 §2), written from the ADR alone, no KentOS code.

Writes fixtures/processing/v1/field-map.json: what the field shows for a map (each row's problem, the lists it offers,
its note, whether Aynı adlarla doldur can add a row), what Aynı adlarla doldur, a row's edit, its removal and Satır ekle
make of a map, and the first problem the dialog's check says. Both platforms play it: the web's
`ui/processing/fieldMap.test.ts`, the desktop's `kentos-processing` `field_map` tests.

The rules:

1. Names are trimmed (JavaScript's white space). A row's problem, the first that holds: its target is empty; an
   earlier row has its target; the source's columns are known and do not hold its source (an empty source is none).
2. The note: the columns are not known (no source read yet); else an empty map says what it does; else none.
3. Aynı adlarla doldur keeps the rows and adds, in the columns' order, a row under its own name for each column that is
   not the key and that no row has as its source or its target (each column once).
4. The check: in the rows' order, the first of: an empty target (“n. satırın hedef alanı boş”), a target given twice,
   then for the target and the source a name longer than 64 UTF-16 units or with a square bracket.
"""

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "processing" / "v1" / "field-map.json"
JS_SPACE = " \t\n\v\f\r                 　﻿"

EMPTY_TARGET = "Hedef alan boş."
NOTE_EMPTY = "Boş: anahtardan başka bütün sütunlar kendi adlarıyla aktarılır."
NOTE_NO_SOURCE = "Kaynağın sütunları kaynak seçilince gelir."


def trim(s):
    return s.strip(JS_SPACE)


def row(target, source="", default=""):
    return {"target": target, "source": source, "default": default}


def fill(value, columns, key):
    out = [dict(r) for r in value]
    taken = set()
    for r in out:
        taken.add(trim(r["target"]))
        taken.add(trim(r["source"]))
    for c in columns:
        name = trim(c)
        if not name or name == trim(key) or name in taken:
            continue
        taken.add(name)
        out.append(row(name, name, ""))
    return out


def view(value, targets, columns, key):
    seen = set()
    rows = []
    for r in value:
        t, s = trim(r["target"]), trim(r["source"])
        if not t:
            problem = EMPTY_TARGET
        elif t in seen:
            problem = f"“{t}” iki satırda hedef."
        elif columns is not None and s and s not in columns:
            problem = f"Kaynakta “{s}” yok."
        else:
            problem = None
        if t:
            seen.add(t)
        rows.append({"target": r["target"], "source": r["source"], "default": r["default"], "problem": problem})
    note = NOTE_NO_SOURCE if columns is None else (None if value else NOTE_EMPTY)
    fills = columns is not None and len(fill(value, columns, key)) > len(value)
    return {"rows": rows, "targets": list(targets or []), "columns": list(columns or []), "note": note, "fills": fills}


def utf16_len(s):
    return len(s.encode("utf-16-le")) // 2


def problem(name, value):
    seen = set()
    for i, r in enumerate(value):
        t = trim(r["target"])
        if not t:
            return f"{name}: {i + 1}. satırın hedef alanı boş."
        if t in seen:
            return f"{name}: “{t}” iki satırda hedef."
        seen.add(t)
        for f in (t, trim(r["source"])):
            if utf16_len(f) > 64:
                return f"{name}: alan adı en çok 64 karakter olabilir."
            if "[" in f or "]" in f:
                return f"{name}: alan adında köşeli parantez kullanılamaz."
    return None


DEED_COLUMNS = ["PARSEL", "ADA", "MALIK_ADI", "HISSE", "YUZOLCUM", "TESCIL_TARIHI"]
PARCEL_FIELDS = ["Parsel", "Ada", "Malik", "Hisse", "Mahalle"]
DEED_MAP = [row("Malik", "MALIK_ADI"), row("Hisse", "HISSE"), row("Alan", "YUZOLCUM"), row("Durum", "", "Tescilli")]


def build():
    views = [
        ("empty map, source known", [], PARCEL_FIELDS, DEED_COLUMNS, "PARSEL"),
        ("empty map, no source yet", [], PARCEL_FIELDS, None, ""),
        ("deed map: rows fine, fill adds the unmapped columns", DEED_MAP, PARCEL_FIELDS, DEED_COLUMNS, "PARSEL"),
        ("a row's source the source does not have", [row("Malik", "SAHIBI"), row("Ada", " ADA ")], PARCEL_FIELDS, DEED_COLUMNS, "PARSEL"),
        ("a target empty or blank, then a target twice (trimmed)", [row("  ", "ADA"), row("Malik", "MALIK_ADI"), row(" Malik", "HISSE")], [], DEED_COLUMNS, "PARSEL"),
        ("columns not known: sources are not checked", [row("Malik", "SAHIBI")], PARCEL_FIELDS, None, "PARSEL"),
        ("every column mapped: nothing to fill", [row(c, c) for c in DEED_COLUMNS if c != "PARSEL"], [], DEED_COLUMNS, "PARSEL"),
        ("the key mapped too: fill adds the rest only", [row("PARSEL", "PARSEL")], [], ["PARSEL", "ADA"], ""),
        ("columns known but empty", [], [], [], ""),
    ]
    fills = [
        ("an empty map takes every column but the key", [], DEED_COLUMNS, "PARSEL"),
        ("rows kept, columns no row takes added in order", DEED_MAP, DEED_COLUMNS, "PARSEL"),
        ("a column named as a target is not added again", [row("ADA", "", "101")], DEED_COLUMNS, " PARSEL "),
        ("blank, repeated and key columns left out", [], ["", "Ad", "Ad", " Kod ", "Anahtar"], "Anahtar"),
        ("no key: every column", [], ["A", "B"], ""),
    ]
    problems = [
        ("fine", DEED_MAP),
        ("empty", []),
        ("second row's target empty", [row("Malik", "MALIK_ADI"), row(" ", "HISSE")]),
        ("a target twice, trimmed", [row("Malik", "MALIK_ADI"), row("Malik ", "HISSE")]),
        ("a target 65 characters long", [row("x" * 65, "A")]),
        ("a source with a bracket", [row("Malik", "MALIK[1]")]),
        ("a target of 64 characters is fine", [row("ş" * 64, "")]),
        ("an emoji counts two", [row("😀" * 33, "")]),
    ]
    edits = [
        {"op": "set", "value": DEED_MAP, "index": 3, "patch": {"default": "Beyanlı"}},
        {"op": "set", "value": DEED_MAP, "index": 0, "patch": {"source": ""}},
        {"op": "remove", "value": DEED_MAP, "index": 1},
        {"op": "add", "value": DEED_MAP},
        {"op": "add", "value": []},
    ]

    def edit(e):
        value = [dict(r) for r in e["value"]]
        if e["op"] == "set":
            value[e["index"]] = {**value[e["index"]], **e["patch"]}
            return value
        if e["op"] == "remove":
            return [r for k, r in enumerate(value) if k != e["index"]]
        return value + [row("")]

    return {
        "format": "kentos.field-map-cases",
        "version": 1,
        "views": [{"title": t, "value": v, "targets": tg, "columns": c, "key": k, "expect": view(v, tg, c, k)} for t, v, tg, c, k in views],
        "fills": [{"title": t, "value": v, "columns": c, "key": k, "expect": fill(v, c, k)} for t, v, c, k in fills],
        "problems": [{"title": t, "name": "“Alan eşleme”", "value": v, "expect": problem("“Alan eşleme”", v)} for t, v in problems],
        "edits": [{**e, "expect": edit(e)} for e in edits],
    }


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print("alan eşleme durumları tutarlı: fixtures/processing/v1/field-map.json")
        return 0
    OUT.write_text(text)
    print(f"yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
