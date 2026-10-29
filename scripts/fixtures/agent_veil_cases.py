#!/usr/bin/env python3
"""What an agent reads of an answer about a drawing that hides fields (docs/adr/0738 §1), written from the ADR
without KentOS code: fixtures/agent-veil/v1/cases.json. Each case gives the hidden fields' names, the values they
hold in the drawing, an answer and the answer as the agent reads it.

The rules (ADR 0738 §1):
- an attribute map (an object under `attrs` or `values`): a hidden field's value (anything but null) is written
  ‹gizli›; other values are cut as answers are;
- a join view (an object with `columns` whose items name a `field`): the `cells` of every row and the `sum`,
  `count`, `skipped` of every total under a hidden field's column are written ‹gizli› (a null cell stays);
- a reconciliation's field (an object whose keys are only `name`, `layer`, `table`, its `name` a hidden field):
  `layer` and `table` are written ‹gizli›;
- a statistic (an object with `read` whose `field` is a hidden field): `read`, `skipped`, `sum`, `mean`, `min`,
  `max`, `std` are written ‹gizli›;
- any text: a hidden value it quotes (“…”) is written ‹gizli› between the quotes;
- everything else as it is.

`--check` compares the file with what this writes; without it the file is written.
"""

import json
import sys
from pathlib import Path

HIDDEN = "‹gizli›"
OUT = Path(__file__).resolve().parents[2] / "fixtures/agent-veil/v1/cases.json"


def text(s: str, values: set[str]) -> str:
    if not values or "“" not in s:
        return s
    out = []
    rest = s
    while "“" in rest:
        before, after = rest.split("“", 1)
        out.append(before + "“")
        if "”" not in after:
            rest = after
            break
        inner, tail = after.split("”", 1)
        out.append(HIDDEN if inner in values else inner)
        rest = "”" + tail
    out.append(rest)
    return "".join(out)


def cut(v, names: list[str], values: set[str]):
    if isinstance(v, dict):
        return cut_object(dict(v), names, values)
    if isinstance(v, list):
        return [cut(x, names, values) for x in v]
    if isinstance(v, str):
        return text(v, values)
    return v


def hide(d: dict, key: str) -> None:
    if key in d and d[key] is not None:
        d[key] = HIDDEN


def cut_object(d: dict, names: list[str], values: set[str]) -> dict:
    columns = d.get("columns")
    if isinstance(columns, list):
        hidden = [i for i, c in enumerate(columns) if isinstance(c, dict) and c.get("field") in names]
        if hidden:
            for row in d.get("rows") or []:
                cells = row.get("cells") if isinstance(row, dict) else None
                if isinstance(cells, list):
                    for i in hidden:
                        if i < len(cells) and cells[i] is not None:
                            cells[i] = HIDDEN
            for total in d.get("totals") or []:
                if isinstance(total, dict) and total.get("column") in hidden:
                    for k in ("sum", "count", "skipped"):
                        hide(total, k)
    if set(d) <= {"name", "layer", "table"} and d.get("name") in names:
        for k in ("layer", "table"):
            hide(d, k)
    if "read" in d and d.get("field") in names:
        for k in ("read", "skipped", "sum", "mean", "min", "max", "std"):
            hide(d, k)
    out = {}
    for k, v in d.items():
        if k in ("attrs", "values") and isinstance(v, dict):
            out[k] = {a: (HIDDEN if a in names and x is not None else cut(x, names, values)) for a, x in v.items()}
        else:
            out[k] = cut(v, names, values)
    return out


NAMES = ["Malik", "TC"]
VALUES = ["Ali Yılmaz", "12345678901", "Ayşe Kaya"]


def entity(attrs: dict) -> dict:
    return {"kind": "polygon", "id": 3, "layerId": "bina", "attrs": attrs,
            "pts": [{"x": 0.0, "y": 0.0}, {"x": 10.0, "y": 0.0}, {"x": 10.0, "y": 8.0}]}


CASES = [
    ("Bir nesnenin öznitelikleri", {"uid": "u1", "entity": entity({"Ada": "101", "Malik": "Ali Yılmaz", "TC": "12345678901"})}),
    ("Nesnelerin sayfası", {"entities": [{"uid": "u1", "entity": entity({"Malik": "Ali Yılmaz"})},
                                         {"uid": "u2", "entity": entity({"Ada": "102"})}], "next": None}),
    ("Gizli alanı olmayan nesne", {"uid": "u2", "entity": entity({"Ada": "102", "Parsel": "5"})}),
    ("Boş değer de gizlenir", {"attrs": {"Malik": "", "Ada": "1"}}),
    ("Null değer kalır", {"attrs": {"Malik": None}}),
    ("Kaynağın satırı", {"rows": [{"uid": "u1", "key": 7, "attrs": {"Malik": "Ayşe Kaya", "Ada": "103"}, "wkt": "POINT (1 2)"}]}),
    ("Formun değerleri", {"uid": "u1", "values": {"Malik": "Ali Yılmaz", "Kat": "3"}, "defaults": True}),
    ("İç içe kayıtlar", {"children": [{"values": {"TC": "12345678901"}}, {"attrs": {"TC": "x"}}]}),
    ("Aynı adlı yapısal anahtar kalır", {"name": "Malik", "kind": "text", "values": ["Ali Yılmaz", "Veli"]}),
    ("Katman adı gizli alanla aynıysa kalır", {"layers": [{"id": "l1", "name": "TC", "type": "layer"}]}),
    ("Tırnak içindeki gizli değer", {"error": {"code": "not_unique",
                                               "message": "“TC” katmanda tekil olmalı; “12345678901” başka bir nesnede var."}}),
    ("Tırnak içindeki açık değer kalır", {"message": "“Ada” için “101” listede yok; listeden seçin."}),
    ("Kapanmayan tırnak", {"message": "“Ali Yılmaz” ve “Ayşe Kaya"}),
    ("Tırnaksız değer kalır", {"message": "Ali Yılmaz kaydı"}),
    ("Uyarı listesi", {"warnings": [{"code": "x", "message": "“Ayşe Kaya” uymuyor"}, {"code": "y", "message": "tamam"}]}),
    ("Birleşik görünüm", {"name": "Görünüm", "columns": [{"name": "Ada", "side": "left", "field": "Ada", "editable": True},
                                                         {"name": "Malik", "side": "right", "field": "Malik", "editable": False}],
                          "rows": [{"id": "a/b", "left": "a", "right": "b", "cells": ["101", "Ali Yılmaz"]},
                                   {"id": "c/", "left": "c", "cells": ["102", None]}],
                          "totals": [{"column": 1, "sum": "3", "count": 2, "skipped": 1}, {"column": 0, "sum": "203", "count": 2}],
                          "report": {"records": 2}, "revision": "4"}),
    ("Gizli sütunu olmayan görünüm", {"columns": [{"name": "Ada", "field": "Ada"}], "rows": [{"cells": ["101"]}]}),
    ("Uzlaştırmanın alanı", {"conflicts": [{"object": "u1", "kind": "changed", "geometry": False,
                                            "fields": [{"name": "Malik", "layer": "Ali Yılmaz", "table": "Ayşe Kaya"},
                                                       {"name": "Ada", "layer": "101", "table": "102"}]}]}),
    ("Uzlaştırmada bir yan yok", {"name": "TC", "table": "12345678901"}),
    ("Gizli alanın istatistiği", {"statistics": [{"field": "TC", "read": 2, "skipped": 0, "sum": "3", "mean": "1.5",
                                                  "min": "1", "max": "2", "std": "0.7"},
                                                 {"field": "Kat", "read": 2, "skipped": 0, "sum": "5"}]}),
    ("Komutun cevabı", {"status": "completed", "output": {"uid": "u1", "attrs": {"Malik": "Ali Yılmaz"}},
                        "summary": {"added": ["u1"]}, "planId": "p"}),
    ("Kaydın girdisi", {"calls": [{"command": "cad.entities.set", "input": {"attrs": {"TC": "12345678901"}}}]}),
    ("Sayılar ve mantıksal değerler", {"attrs": {"TC": 5, "Malik": True, "Kat": 3}}),
]


def build() -> dict:
    values = set(VALUES)
    return {
        "format": "kentos.agent-veil/1",
        "note": "Written by scripts/fixtures/agent_veil_cases.py from docs/adr/0738 §1 without KentOS code.",
        "hidden": HIDDEN,
        "cases": [
            {"name": name, "names": NAMES, "values": VALUES, "answer": answer, "cut": cut(json.loads(json.dumps(answer)), NAMES, values)}
            for name, answer in CASES
        ],
    }


def main() -> int:
    text_out = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text_out:
            print(f"{OUT} güncel değil: python3 {Path(__file__).name} ile yazın.", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(OUT.parents[3])}: {len(CASES)} durum güncel.")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text_out, encoding="utf-8")
    print(f"{OUT}: {len(CASES)} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
