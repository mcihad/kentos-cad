#!/usr/bin/env python3
"""Independent reference of the attribute table's rows (docs/adr/0199 §4):
which objects of a layer the table shows and in what order.

    python3 scripts/fixtures/feature_table_cases.py           # writes the file
    python3 scripts/fixtures/feature_table_cases.py --check   # writes nothing; compares

Writes fixtures/feature-table/v1/cases.json from the rules alone, with
Python's standard library and no KentOS code. The geometry core
(`ops::feature_table`; crates/shared/geometry-core/tests/all/feature_table.rs)
and the web through its WASM (apps/web/src/wasm/featureTable.wasm.test.ts)
give the same rows.

A row is an object of the layer, in the drawing's order: its cells (one per
column: the text it shows and, when its value is there and keeps its field's
rules, its sort key, the value's canonical text), whether it is selected, in
the view, and kept by the expression filter. A column has its order
(`text`, `number`, `date`, `boolean`) and whether the search looks in it.

- Shown: every row (`all`), the selected ones (`selected`) or the ones in the
  view (`inView`); any other word is every row. A row the expression filter
  does not keep is never shown.
- Search: trimmed (JavaScript's white space); empty, it asks nothing. A row
  answers when one of the searched columns' shown texts does: caseless the
  Turkish way (I is ı's, İ is i's, any other letter its lowercase when that
  is one character); with a `*` the pattern matches the whole text (`*` any
  run of characters), without one it is somewhere in it.
- Sort by a column (its index; none, or one the table does not have: the
  drawing's order): numbers by their exact value (canonical decimal text:
  an optional minus, digits, an optional fraction), dates by the calendar
  (YYYY-MM-DD), booleans false before true, texts in the natural order
  (docs/adr/0153 §6). A row without a key (empty, or its value does not keep
  the field's rules) comes after every row with one, whichever the way;
  descending reverses the comparison only; equal ones keep the drawing's
  order.
"""

import json
import random
import re
import sys
from fractions import Fraction
from functools import cmp_to_key
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "feature-table" / "v1" / "cases.json"
ALPHABET = "abcçdefgğhıijklmnoöpqrsştuüvwxyz"
JS_SPACE = " \t\n\v\f\r                 　﻿"


def cmp(a, b):
    return (a > b) - (a < b)


def fold(c):
    if c == "I":
        return "ı"
    if c == "İ":
        return "i"
    low = c.lower()
    return low if len(low) == 1 else c


def folded(s):
    return "".join(fold(c) for c in s)


def trim(s):
    return s.strip(JS_SPACE)


# ── Natural order (docs/adr/0153 §6) ───────────────────────────────────


def tokens(text):
    return re.findall(r"[0-9]+|.", text, re.S)


def rank(c):
    f = fold(c)
    if not f.isalpha():
        return (0, ord(f))
    i = ALPHABET.find(f)
    return (1, i) if i >= 0 else (2, ord(f))


def natural_cmp(a, b):
    ta, tb = tokens(a), tokens(b)
    for x, y in zip(ta, tb):
        xd, yd = x[0].isdigit() and x.isascii(), y[0].isdigit() and y.isascii()
        if xd and yd:
            o = cmp(int(x), int(y))
        elif xd:
            o = -1
        elif yd:
            o = 1
        else:
            o = cmp(rank(x), rank(y))
        if o:
            return o
    return cmp(len(ta), len(tb)) or cmp(a, b)


# ── Search ─────────────────────────────────────────────────────────────


def search(text, pattern):
    t, p = folded(text), folded(pattern)
    if "*" in p:
        return re.fullmatch(".*".join(re.escape(part) for part in p.split("*")), t, re.S) is not None
    return p in t


# ── Sort keys ──────────────────────────────────────────────────────────


def key_cmp(order, a, b):
    if order == "number":
        return cmp(Fraction(a), Fraction(b))
    if order == "date":
        return cmp(a, b)
    if order == "boolean":
        return cmp(a == "true", b == "true")
    return natural_cmp(a, b)


def table(columns, rows, query):
    wanted = trim(query["search"])
    show = query["show"]
    searched = [i for i, c in enumerate(columns) if c["searched"]]
    out = []
    for i, r in enumerate(rows):
        if not r["passes"]:
            continue
        if show == "selected" and not r["selected"]:
            continue
        if show == "inView" and not r["inView"]:
            continue
        if wanted and not any(search(r["cells"][c]["shown"], wanted) for c in searched):
            continue
        out.append(i)
    col = query["sort"]
    if col is None or not 0 <= col < len(columns):
        return out
    order = columns[col]["order"]
    desc = query["descending"]

    def by(a, b):
        ka, kb = rows[a]["cells"][col].get("key"), rows[b]["cells"][col].get("key")
        if ka is not None and kb is not None:
            o = key_cmp(order, ka, kb)
            o = -o if desc else o
        elif ka is not None:
            o = -1
        elif kb is not None:
            o = 1
        else:
            o = 0
        return o or cmp(a, b)

    return sorted(out, key=cmp_to_key(by))


# ── Cases ──────────────────────────────────────────────────────────────


def cell(shown, key=None):
    c = {"shown": shown}
    if key is not None:
        c["key"] = key
    return c


def row(cells, selected=False, in_view=True, passes=True):
    return {"cells": cells, "selected": selected, "inView": in_view, "passes": passes}


COLUMNS = [
    {"order": "text", "searched": False},  # Tür
    {"order": "text", "searched": True},  # Ad
    {"order": "number", "searched": True},  # Kat (tam sayı)
    {"order": "number", "searched": True},  # Alan (ondalık)
    {"order": "date", "searched": True},  # Tarih
    {"order": "boolean", "searched": True},  # Ruhsat
    {"order": "text", "searched": True},  # Kullanım (değer listesi: etiketiyle)
]

PARCELS = [
    row([cell("Alan", "Alan"), cell("Ada 10", "Ada 10"), cell("3", "3"), cell("120.50", "120.50"), cell("07.10.2026", "2026-10-07"), cell("Evet", "true"), cell("Konut", "Konut")], selected=True),
    row([cell("Alan", "Alan"), cell("Ada 2", "Ada 2"), cell("12", "12"), cell("98.25", "98.25"), cell("02.01.2026", "2026-01-02"), cell("Hayır", "false"), cell("Ticaret", "Ticaret")]),
    row([cell("Alan", "Alan"), cell("ada 9", "ada 9"), cell(""), cell("120.5", "120.5"), cell("31.12.1999", "1999-12-31"), cell(""), cell("İşyeri", "İşyeri")], in_view=False),
    row([cell("Nokta", "Nokta"), cell("İstasyon", "İstasyon"), cell("3a"), cell("-0.75", "-0.75"), cell("2026-13-01"), cell("Evet", "true"), cell("")], selected=True),
    row([cell("Alan", "Alan"), cell(""), cell("-2", "-2"), cell("0", "0"), cell("07.10.2026", "2026-10-07"), cell("Hayır", "false"), cell("Konut", "Konut")], passes=False),
    row([cell("Çizgi", "Çizgi"), cell("Ilgaz", "Ilgaz"), cell("40", "40"), cell(""), cell("01.01.0999", "0999-01-01"), cell("Evet", "true"), cell("konut ek")], in_view=False),
    row([cell("Alan", "Alan"), cell("Zeytin", "Zeytin"), cell("3", "3"), cell("120.500", "120.500"), cell(""), cell("Hayır", "false"), cell("Ticaret", "Ticaret")], selected=True),
]


def q(search="", show="all", sort=None, descending=False):
    return {"search": search, "show": show, "sort": sort, "descending": descending}


QUERIES = [
    ("çizim-sırası", q()),
    ("ad-artan", q(sort=1)),
    ("ad-azalan", q(sort=1, descending=True)),
    ("kat-artan", q(sort=2)),
    ("kat-azalan", q(sort=2, descending=True)),
    ("alan-eşitler", q(sort=3)),
    ("alan-azalan", q(sort=3, descending=True)),
    ("tarih-artan", q(sort=4)),
    ("tarih-azalan", q(sort=4, descending=True)),
    ("evet-hayır", q(sort=5)),
    ("evet-hayır-azalan", q(sort=5, descending=True)),
    ("liste-etiketle", q(sort=6)),
    ("tür", q(sort=0)),
    ("olmayan-sütun", q(sort=9)),
    ("seçililer", q(show="selected")),
    ("görünümdekiler", q(show="inView", sort=2)),
    ("bilinmeyen-gösterim", q(show="hepsi")),
    ("ara-büyük-harf", q(search="  KONUT ")),
    ("ara-joker", q(search="ada*")),
    ("ara-joker-ortada", q(search="*1*.5*")),
    ("ara-ı", q(search="ılg")),
    ("ara-i", q(search="istasyon")),
    ("ara-türde-aranmaz", q(search="Nokta")),
    ("ara-gösterilende", q(search="07.10")),
    ("ara-sırala", q(search="e", sort=3, descending=True)),
    ("ara-boşluk", q(search="   ")),
]

# Exact numbers a float64 cannot tell apart, and every order.
BIG = [{"order": "number", "searched": True}, {"order": "date", "searched": True}, {"order": "boolean", "searched": False}, {"order": "text", "searched": True}]
BIG_ROWS = [
    row([cell("a", "123456789012345678901234567891"), cell("x", "2026-10-07"), cell("Evet", "true"), cell("a10", "a10")]),
    row([cell("b", "123456789012345678901234567890"), cell("y", "2026-10-06"), cell("Hayır", "false"), cell("a2", "a2")]),
    row([cell("c", "0.10000000000000000000000000001"), cell("z", "9999-12-31"), cell(""), cell("B", "B")]),
    row([cell("d", "0.1"), cell(""), cell("Evet", "true"), cell("ç", "ç")]),
    row([cell("e", "-0.1"), cell("w", "0001-01-01"), cell("Hayır", "false"), cell("C", "C")]),
    row([cell("f", "-123456789012345678901234567890"), cell("v", "2026-10-07"), cell("Evet", "true"), cell("a02", "a02")]),
    row([cell("g", "0"), cell("u", "1999-02-28"), cell("Hayır", "false"), cell("")]),
]
BIG_QUERIES = [("sayı", q(sort=0)), ("sayı-azalan", q(sort=0, descending=True)), ("tarih", q(sort=1)), ("evet-hayır", q(sort=2)), ("doğal", q(sort=3)), ("doğal-azalan", q(sort=3, descending=True))]


def random_cases():
    rnd = random.Random(199)
    columns = [{"order": o, "searched": rnd.random() < 0.8} for o in ("number", "number", "date", "boolean", "text", "text")]
    words = ["Ada", "ada", "İl", "ıl", "Konut", "konut", "Çam", "çam", "a1", "a10", "a2", "B", "b", "Ş", "s", "ü", "u", ""]
    rows = []
    for _ in range(48):
        cells = []
        for c in columns:
            r = rnd.random()
            if r < 0.15:
                cells.append(cell(""))
                continue
            if r < 0.25:
                cells.append(cell(rnd.choice(["?", "x1", "--"])))
                continue
            if c["order"] == "number":
                n = Fraction(rnd.randint(-2000, 2000), rnd.choice([1, 10, 100, 1000]))
                scale = rnd.choice([0, 1, 2, 3])
                text = f"{n:.{scale}f}" if scale else str(round(n))
                if re.fullmatch(r"-0(\.0*)?", text):
                    text = text[1:]
                cells.append(cell(text, text))
            elif c["order"] == "date":
                text = f"{rnd.randint(1990, 2030):04d}-{rnd.randint(1, 12):02d}-{rnd.randint(1, 28):02d}"
                cells.append(cell(f"{text[8:]}.{text[5:7]}.{text[:4]}", text))
            elif c["order"] == "boolean":
                v = rnd.choice(["true", "false"])
                cells.append(cell("Evet" if v == "true" else "Hayır", v))
            else:
                w = rnd.choice(words[:-1]) + rnd.choice(["", " 1", " 2", " 10"])
                cells.append(cell(w, w))
        rows.append(row(cells, selected=rnd.random() < 0.4, in_view=rnd.random() < 0.7, passes=rnd.random() < 0.9))
    queries = []
    for n in range(30):
        queries.append((f"rastgele-{n + 1}", q(
            search=rnd.choice(["", "", "a", "A*", "*1", "konut", "İL", "ı", "evet", "1.", "*0*"]),
            show=rnd.choice(["all", "all", "selected", "inView"]),
            sort=rnd.choice([None, 0, 1, 2, 3, 4, 5]),
            descending=rnd.random() < 0.5,
        )))
    return columns, rows, queries


def build():
    groups = [("parseller", COLUMNS, PARCELS, QUERIES), ("kesin-ve-doğal", BIG, BIG_ROWS, BIG_QUERIES), ("rastgele", *random_cases())]
    return {
        "format": "kentos.feature-table-cases",
        "version": 1,
        "generatedBy": "scripts/fixtures/feature_table_cases.py",
        "title": "Öznitelik tablosu: gösterilen satırlar ve sıraları (ADR 0199 §4)",
        "tables": [
            {
                "name": name,
                "columns": columns,
                "rows": rows,
                "queries": [{"name": n, "query": query, "want": table(columns, rows, query)} for n, query in queries],
            }
            for name, columns, rows, queries in groups
        ],
    }


def text_of(v):
    return json.dumps(v, ensure_ascii=False, indent=1) + "\n"


def main():
    want = text_of(build())
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != want:
            print(f"{OUT.relative_to(ROOT)}: diskteki dosya kurallardan üretilenle aynı değil", file=sys.stderr)
            return 1
        print("feature table cases match")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(want, encoding="utf-8")
    print(f"written: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
