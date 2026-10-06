#!/usr/bin/env python3
"""Independent reference of Veride ara (docs/adr/0178 §1, §2, §4).

Writes fixtures/search/v1/cases.json from the rules alone, with Python's
standard library and no KentOS code. The geometry core
(`text::edit::matches`, `ops::data_search`;
crates/shared/geometry-core/tests/all/data_search.rs), the web through its
WASM (apps/web/src/wasm/dataSearch.wasm.test.ts), and both platforms' record
builders (apps/web/src/model/dataSearch.test.ts,
crates/native/interaction/tests/all/data_search.rs) must give the same.

Match: a word answers a text when, caseless (the Turkish way: I is ı's, İ is
i's, any other letter its lowercase when that is one character; off with
"Büyük küçük harf eşleşsin"), it is somewhere in the text; with a `*` the
pattern matches the whole text (`*` any run of characters, `whole word`
has no say); with "Tam sözcük" the place has no letter, digit or `_` on
either side. The word is trimmed (JavaScript's white space); empty, it asks
nothing.

Search: `drawings` are lists of records, a case asks one of them (by its index). A record answers when one of the fields asked for does: its label,
its text, its block's name, its attributes' values (all, or the one named).
Values are trimmed and a blank one is no field. The row shows the first that
answers, looked at in that order, the attributes by their names (code point
order); `more` counts the others that answer. Rows are in the records'
order, or sorted by `layer`, `kind`, `field` (label, text, block, attribute,
then the attribute's name) or `value`, in the natural order (docs/adr/0153
§6), equal ones in the records' order; the descending sort reverses the
comparison only. `limit` rows at most (0: all); `total` is how many records
answer.

Attribute names: the names the attributes of a drawing's records carry with a
value that is not blank, each once, in the natural order.

Records: an object's record is its kind's Turkish name, its layer's path,
its label as stored, its words (a text's text, a leader's note, a dimension's
own text, a table's cells that are not empty, row by row, one per line), the
name of its insert's block (none for an unknown block) and its attributes as
stored; none when it has no label, text, block name or attribute value that
is not blank.
"""

import argparse
import json
import random
import re
import sys
from functools import cmp_to_key
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "search" / "v1" / "cases.json"
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


# ── Match ──────────────────────────────────────────────────────────────


def word_char(c):
    return c.isalnum() or c == "_"


def matches(text, pattern, caseless, whole_word):
    t = folded(text) if caseless else text
    p = folded(pattern) if caseless else pattern
    if "*" in p:
        return re.fullmatch(".*".join(re.escape(part) for part in p.split("*")), t, re.S) is not None
    start = 0
    while start <= len(t) - len(p):
        i = t.find(p, start)
        if i < 0:
            return False
        end = i + len(p)
        if not whole_word or ((i == 0 or not word_char(t[i - 1])) and (end == len(t) or not word_char(t[end]))):
            return True
        start = i + 1
    return False


# ── Search ─────────────────────────────────────────────────────────────

RANK = {"label": 0, "text": 1, "block": 2, "attr": 3}


def trimmed(v):
    if v is None:
        return None
    v = v.strip(JS_SPACE)
    return v or None


def hits(record, q, wanted):
    def answers(v):
        return matches(v, wanted, not q["matchCase"], q["wholeWord"])

    f = q["fields"]
    out = []
    for field in ("label", "text", "block"):
        v = trimmed(record.get(field))
        if f[field] and v is not None and answers(v):
            out.append((field, None, v))
    if f["attrs"]:
        for name, value in sorted(record["attrs"], key=lambda a: a[0]):
            if f["attrName"] is not None and f["attrName"] != name:
                continue
            v = trimmed(value)
            if v is not None and answers(v):
                out.append(("attr", name, v))
    return out


def search(records, q):
    wanted = q["pattern"].strip(JS_SPACE)
    if not wanted:
        return {"rows": [], "total": 0}
    rows = []
    for i, r in enumerate(records):
        found = hits(r, q, wanted)
        if found:
            field, name, value = found[0]
            rows.append({"record": i, "field": field, "name": name, "value": value, "more": len(found) - 1})
    total = len(rows)
    col, desc = q["sort"], q["descending"]
    if col in ("layer", "kind", "field", "value"):

        def by(a, b):
            ra, rb = records[a["record"]], records[b["record"]]
            if col == "layer":
                o = natural_cmp(ra["layer"], rb["layer"])
            elif col == "kind":
                o = natural_cmp(ra["kind"], rb["kind"])
            elif col == "field":
                o = cmp(RANK[a["field"]], RANK[b["field"]]) or natural_cmp(a["name"] or "", b["name"] or "")
            else:
                o = natural_cmp(a["value"], b["value"])
            return (-o if desc else o) or cmp(a["record"], b["record"])

        rows.sort(key=cmp_to_key(by))
    if q["limit"] > 0:
        rows = rows[: q["limit"]]
    return {"rows": rows, "total": total}


# ── Attribute names ────────────────────────────────────────────────────


def attribute_names(records):
    names = {name for r in records for name, value in r["attrs"] if trimmed(value) is not None}
    return sorted(names, key=cmp_to_key(natural_cmp))


# ── Records ────────────────────────────────────────────────────────────

KIND = {
    "point": "Nokta",
    "line": "Çizgi",
    "polyline": "Çoklu çizgi",
    "polygon": "Kapalı alan",
    "circle": "Daire",
    "arc": "Yay",
    "ellipse": "Elips",
    "spline": "Eğri",
    "xline": "Yardımcı çizgi",
    "ray": "Işın",
    "text": "Yazı",
    "dimension": "Ölçü",
    "hatch": "Tarama",
    "insert": "Blok",
    "leader": "Kılavuz",
    "table": "Tablo",
}


def record_of(e, layers, blocks):
    label = e.get("label")
    text = e.get("text") if e["kind"] in ("text", "leader", "dimension") else None
    if e["kind"] == "table":
        text = "\n".join(w for row in e["cells"] for w in row if w != "")
    block = blocks.get(e["block"]) if e["kind"] == "insert" else None
    attrs = [[k, v] for k, v in e["attrs"].items()]
    blank = lambda v: v is None or not v.strip(JS_SPACE)
    if blank(label) and blank(text) and blank(block) and all(blank(v) for _, v in attrs):
        return None
    return {
        "kind": KIND[e["kind"]],
        "layer": layers.get(e["layerId"], ""),
        "label": label,
        "text": text,
        "block": block,
        "attrs": attrs,
    }


# ── Cases ──────────────────────────────────────────────────────────────

FIELDS_ALL = {"label": True, "text": True, "block": True, "attrs": True, "attrName": None}


def Q(pattern, matchCase=False, wholeWord=False, fields=None, sort=None, descending=False, limit=0):
    return {
        "pattern": pattern,
        "matchCase": matchCase,
        "wholeWord": wholeWord,
        "fields": {**FIELDS_ALL, **(fields or {})},
        "sort": sort,
        "descending": descending,
        "limit": limit,
    }


def match_cases():
    hand = [
        # text, pattern, matchCase, wholeWord
        ("Ada 101", "101", False, False),
        ("Ada 101", "ada", False, False),
        ("Ada 101", "ada", True, False),
        ("Ada 101", "Ada", True, False),
        ("Ada 101", "da 1", False, False),
        ("Ada 101", "102", False, False),
        ("Ada 101", "Ada 101 ", False, False),
        ("KIZ", "kız", False, False),
        ("kız", "KIZ", False, False),
        ("İstasyon", "istasyon", False, False),
        ("istasyon", "İSTASYON", False, False),
        ("Istasyon", "istasyon", False, False),
        ("ISIK", "ışık", False, False),
        ("ışık", "IŞIK", False, False),
        ("Işık", "ışık", True, False),
        ("Işık", "Işık", True, False),
        ("Çamlık", "çam", False, False),
        ("ÇAMLIK", "çam", False, False),
        ("ÇAMLIK", "çam", True, False),
        ("Ağaç", "AĞAÇ", False, False),
        ("Üst", "üst", False, False),
        ("Ödemiş", "ÖDEM", False, False),
        ("Şev", "şev", False, False),
        # Whole word.
        ("Ada 101", "101", False, True),
        ("1010", "101", False, True),
        ("A101", "101", False, True),
        ("Ada-101", "101", False, True),
        ("101/2", "101", False, True),
        ("101_2", "101", False, True),
        ("ev 101 ev", "ev", False, True),
        ("evler", "ev", False, True),
        ("ev1", "ev", False, True),
        ("1010 101", "101", False, True),
        ("Parsel", "Par", False, True),
        ("Çam Ağacı", "ağacı", False, True),
        ("Çam Ağacı", "ağac", False, True),
        ("101", "101", False, True),
        ("101", "10", False, True),
        # Wildcards: the whole text.
        ("Ada 101", "Ada *", False, False),
        ("Ada 101", "ada*", False, False),
        ("Ada 101", "*101", False, False),
        ("Ada 101", "*10", False, False),
        ("Ada 101", "A*1", False, False),
        ("Ada 101", "A*0*1", False, False),
        ("Ada 101", "*", False, False),
        ("Ada 101", "**", False, False),
        ("Ada 101", "*a*", False, False),
        ("Ada 101", "A*A", False, False),
        ("Ada 101", "ada*", True, False),
        ("Ada 101", "Ada*", True, False),
        ("Ada 101", "Ada*", False, True),
        ("a1b2c", "a*b*c", False, False),
        ("abc", "a*b*c", False, False),
        ("ac", "a*b*c", False, False),
        ("abcbc", "a*bc", False, False),
        ("abcbc", "a*b*c", False, False),
        ("İSTASYON 5", "i*5", False, False),
        ("ISTASYON 5", "i*5", False, False),
        ("a.b", "a.*", False, False),
        ("a+b", "a+*", False, False),
        ("(x)", "(*)", False, False),
        # Not whole word, not caseless.
        ("aXbxc", "x", True, False),
        ("aXbxc", "X", True, False),
        # An empty pattern is in every text.
        ("Ada", "", False, False),
        ("", "", False, False),
        ("", "a", False, False),
        ("", "*", False, False),
        # Longer than the text.
        ("ab", "abc", False, False),
        ("ab", "ab", False, False),
    ]
    rnd = random.Random(178)
    words = ["Ada", "ada", "ADA", "Parsel", "Rögar", "RÖGAR", "Vana", "İnce", "ışık", "Işık", "ISIK", "çam", "ÇAM", "101", "1010", "P1", "P10", "A-5", "a_b", "x y"]
    for _ in range(80):
        text = " ".join(rnd.choice(words) for _ in range(rnd.randint(1, 3))) + rnd.choice(["", "", "2", "-7", "/3"])
        pattern = rnd.choice(
            [
                rnd.choice(words),
                rnd.choice(words)[: rnd.randint(1, 3)],
                "*" + rnd.choice(words)[: rnd.randint(1, 3)],
                rnd.choice(words)[: rnd.randint(1, 3)] + "*",
                rnd.choice(words)[:2] + "*" + rnd.choice(words)[-2:],
                text[rnd.randint(0, 2) : rnd.randint(3, max(3, len(text)))] or "a",
            ]
        )
        hand.append((text, pattern, rnd.random() < 0.3, rnd.random() < 0.3))
    return [
        {"name": f"{i + 1}: “{t}” için “{p}”", "text": t, "pattern": p, "matchCase": mc, "wholeWord": ww, "expected": matches(t, p, not mc, ww)}
        for i, (t, p, mc, ww) in enumerate(hand)
    ]


def rec(kind, layer, label=None, text=None, block=None, attrs=()):
    return {"kind": kind, "layer": layer, "label": label, "text": text, "block": block, "attrs": [list(a) for a in attrs]}


def hand_records():
    return [
        rec("Kapalı alan", "Kadastro / Parsel", "101", None, None, [("Ada", "1244"), ("Parsel", "101"), ("Tür", "Konut")]),
        rec("Kapalı alan", "Kadastro / Parsel", "102", None, None, [("Ada", "1244"), ("Parsel", "102"), ("Tür", "Ticaret")]),
        rec("Yazı", "Yazılar", None, "Ada 1244", None, []),
        rec("Yazı", "Yazılar", None, "  Parsel 101  ", None, []),
        rec("Nokta", "Nokta", "P10", None, None, [("Kod", "ST")]),
        rec("Nokta", "Nokta", "P2", None, None, [("Kod", "KB"), ("Not", "köşe taşı")]),
        rec("Nokta", "Nokta", "p1", None, None, [("Kod", "st")]),
        rec("Blok", "Altyapı / Rögar", None, None, "Rögar", [("NO", "R-1"), ("KOT", "101.25")]),
        rec("Blok", "Altyapı / Rögar", None, None, "Rögar kapağı", [("NO", "R-12")]),
        rec("Kılavuz", "Kılavuz", None, "Ø150 PVC boru", None, []),
        rec("Ölçü", "Ölçü", None, "12.50", None, []),
        rec("Çizgi", "Yol", "Cumhuriyet Cd.", None, None, [("Tür", "Yol")]),
        rec("Çoklu çizgi", "Yol", "Cumhuriyet Cd.", None, None, []),
        rec("Nokta", "Nokta", "   ", None, None, [("Kod", "   ")]),
        rec("Yazı", "Yazılar", "101", "101 numaralı parsel", None, [("Parsel", "101")]),
        rec("Nokta", "Nokta", "101", None, None, [("Ada", "101"), ("Parsel", "101"), ("Not", "101 nolu")]),
    ]


def search_cases():
    recs = hand_records()
    queries = [
        ("101", Q("101")),
        ("101, tam sözcük", Q("101", wholeWord=True)),
        ("ada", Q("ada")),
        ("ada, büyük küçük harf", Q("ada", matchCase=True)),
        ("Ada, büyük küçük harf", Q("Ada", matchCase=True)),
        ("p*", Q("p*")),
        ("P*, büyük küçük harf", Q("P*", matchCase=True)),
        ("*0", Q("*0")),
        ("rögar", Q("rögar")),
        ("RÖGAR", Q("RÖGAR")),
        ("r-1*", Q("r-1*")),
        ("st", Q("st")),
        ("ST, büyük küçük harf", Q("ST", matchCase=True)),
        ("köşe", Q("köşe")),
        ("KÖŞE", Q("KÖŞE")),
        ("boşluklu ada", Q("  ada 1244  ")),
        ("boş söz", Q("   ")),
        ("yok", Q("yokyokyok")),
        ("yalnız etiket", Q("101", fields={"text": False, "block": False, "attrs": False})),
        ("yalnız yazı", Q("101", fields={"label": False, "block": False, "attrs": False})),
        ("yalnız blok", Q("rögar", fields={"label": False, "text": False, "attrs": False})),
        ("yalnız öznitelik", Q("101", fields={"label": False, "text": False, "block": False})),
        ("yalnız Parsel özniteliği", Q("101", fields={"label": False, "text": False, "block": False, "attrName": "Parsel"})),
        ("Parsel özniteliği, hepsi açık", Q("101", fields={"attrName": "Parsel"})),
        ("olmayan öznitelik", Q("101", fields={"attrName": "Yok"})),
        ("Kod özniteliği", Q("st", fields={"attrName": "Kod"})),
        ("hiçbir alan", Q("101", fields={"label": False, "text": False, "block": False, "attrs": False})),
        ("sıra: katman", Q("1", sort="layer")),
        ("sıra: katman, azalan", Q("1", sort="layer", descending=True)),
        ("sıra: tür", Q("1", sort="kind")),
        ("sıra: tür, azalan", Q("1", sort="kind", descending=True)),
        ("sıra: alan", Q("1", sort="field")),
        ("sıra: alan, azalan", Q("1", sort="field", descending=True)),
        ("sıra: değer", Q("p*", sort="value")),
        ("sıra: değer, azalan", Q("*", sort="value", descending=True)),
        ("sıra: değer, doğal", Q("*", sort="value")),
        ("sıra: bilinmeyen", Q("1", sort="sıra")),
        ("sınır 3", Q("*", limit=3)),
        ("sınır 3, sıralı", Q("*", limit=3, sort="value")),
        ("sınır 0", Q("*", limit=0)),
        ("sınır büyük", Q("*", limit=1000)),
    ]
    drawings = [recs]
    cases = [{"name": n, "drawing": 0, "query": q, "expected": search(recs, q)} for n, q in queries]
    # Random drawings.
    rnd = random.Random(1780)
    layers = ["Parsel", "Kadastro / Parsel", "Yol", "Yazılar", "Altyapı / Rögar", "Nokta 2", "Nokta 10"]
    kinds = ["Nokta", "Kapalı alan", "Yazı", "Blok", "Çizgi", "Kılavuz", "Ölçü"]
    names = ["Ada", "Parsel", "Kod", "NO", "Not", "Tür", "Kat"]
    values = ["101", "1010", "A-5", "P10", "P2", "ST", "st", "Rögar", "İnce", "ince", "ışık", "ISIK", "çam", "Ağaç", "Konut", "12.5", "x y", "  7 ", "", "Ada 101", "Parsel 5/2"]
    for seed in range(1, 11):
        records = []
        for _ in range(rnd.randint(15, 40)):
            attrs = {}
            for _ in range(rnd.randint(0, 3)):
                attrs[rnd.choice(names)] = rnd.choice(values)
            records.append(
                rec(
                    rnd.choice(kinds),
                    rnd.choice(layers),
                    rnd.choice(values) if rnd.random() < 0.5 else None,
                    rnd.choice(values) if rnd.random() < 0.3 else None,
                    rnd.choice(values) if rnd.random() < 0.15 else None,
                    list(attrs.items()),
                )
            )
        for k in range(8):
            pattern = rnd.choice(["1", "10", "101", "p*", "*0", "ışık", "ISIK", "ince", "A-*", "ada*", "x y", "7", "5/2", "*", "st", "ağaç", "AĞAÇ"])
            q = Q(
                pattern,
                matchCase=rnd.random() < 0.3,
                wholeWord=rnd.random() < 0.3,
                fields={
                    "label": rnd.random() < 0.8,
                    "text": rnd.random() < 0.8,
                    "block": rnd.random() < 0.8,
                    "attrs": rnd.random() < 0.8,
                    "attrName": rnd.choice([None, None, "Ada", "Kod", "Parsel"]),
                },
                sort=rnd.choice([None, None, "layer", "kind", "field", "value"]),
                descending=rnd.random() < 0.4,
                limit=rnd.choice([0, 0, 5, 12]),
            )
            cases.append({"name": f"rastgele çizim {seed}, sorgu {k + 1}", "drawing": seed, "query": q, "expected": search(records, q)})
        drawings.append(records)
    return drawings, cases


def ent(kind, layer="0", label=None, attrs=None, **rest):
    e = {"kind": kind, "id": 1, "layerId": layer, "attrs": attrs or {}}
    if label is not None:
        e["label"] = label
    return {**e, **rest}


P0 = {"x": 0.0, "y": 0.0}
P1 = {"x": 1.0, "y": 0.0}
P2 = {"x": 1.0, "y": 1.0}


def record_cases():
    layers = {"0": "Çizim", "parsel": "Kadastro / Parsel", "yazi": "Yazılar", "altyapi": "Altyapı / Rögar"}
    ROGAR, VANA, YOK = ("0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d1001", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d1002", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d1999")
    blocks = {ROGAR: "Rögar", VANA: "Vana"}
    hand = [
        ("etiketli nokta", ent("point", "parsel", "P10", {"Kod": "ST"}, p=P0)),
        ("etiketsiz, öznitelikli nokta", ent("point", "parsel", None, {"Kod": "ST"}, p=P0)),
        ("hiçbir değeri yok", ent("point", "parsel", None, {}, p=P0)),
        ("boş değerler", ent("point", "parsel", "  ", {"Kod": ""}, p=P0)),
        ("yalnız boş değerli öznitelikler", ent("point", "parsel", None, {"Kod": " ", "Not": ""}, p=P0)),
        ("çizgi", ent("line", "0", "Cumhuriyet Cd.", {}, a=P0, b=P1)),
        ("çoklu çizgi", ent("polyline", "0", None, {"Tür": "Yol"}, pts=[P0, P1, P2])),
        ("kapalı alan", ent("polygon", "parsel", "101", {"Ada": "1244", "Parsel": "101"}, pts=[P0, P1, P2])),
        ("daire", ent("circle", "0", "Havuz", {}, c=P0, r=2.0)),
        ("yay", ent("arc", "0", None, {"Not": "dere"}, c=P0, r=2.0, a0=0.0, a1=1.0)),
        ("elips", ent("ellipse", "0", "E1", {}, c=P0, major=P1, ratio=0.5, t0=0.0, t1=0.0)),
        ("eğri", ent("spline", "0", "Kot eğrisi", {}, pts=[P0, P1, P2], closed=False)),
        ("yardımcı çizgi", ent("xline", "0", None, {"Not": "eksen"}, p=P0, dir=P1)),
        ("ışın", ent("ray", "0", "I1", {}, p=P0, dir=P1)),
        ("tarama", ent("hatch", "0", None, {"Desen": "Çapraz"}, ring=[P0, P1, P2], pattern={"type": "lines", "angle": 45.0, "spacing": 1.0})),
        ("yazı", ent("text", "yazi", None, {}, p=P0, text="Ada 1244", height=2.0, rotation=0.0)),
        ("etiketli yazı", ent("text", "yazi", "T1", {"Not": "okunur"}, p=P0, text="Parsel 101", height=2.0, rotation=0.0)),
        ("boş yazı", ent("text", "yazi", None, {}, p=P0, text="   ", height=2.0, rotation=0.0)),
        ("notlu kılavuz", ent("leader", "0", None, {}, pts=[P0, P1], text="Ø150 PVC", height=1.0, rotation=0.0)),
        ("notsuz kılavuz", ent("leader", "0", None, {}, pts=[P0, P1], height=1.0, rotation=0.0)),
        ("kendi yazılı ölçü", ent("dimension", "0", None, {}, a=P0, b=P1, offset=1.0, height=1.0, text="12.50")),
        ("ölçülen değerli ölçü", ent("dimension", "0", None, {}, a=P0, b=P1, offset=1.0, height=1.0)),
        ("blok yerleştirmesi", ent("insert", "altyapi", None, {"NO": "R-1", "KOT": "101.25"}, block=ROGAR, p=P0, scale=1.0, rotation=0.0)),
        ("bilinmeyen blok, öznitelikli", ent("insert", "altyapi", None, {"NO": "R-2"}, block=YOK, p=P0, scale=1.0, rotation=0.0)),
        ("bilinmeyen blok, değersiz", ent("insert", "altyapi", None, {}, block=YOK, p=P0, scale=1.0, rotation=0.0)),
        ("etiketli, aynalı blok", ent("insert", "0", "V1", {}, block=VANA, p=P0, scale=2.0, rotation=1.0, mirror=True)),
        ("bilinmeyen katman", ent("point", "yok", "P1", {}, p=P0)),
        (
            "tablo, boş hücreleri yok sayılır",
            ent("table", "0", None, {}, p=P0, rotation=0.0, height=1.0, rows=[2.0, 2.0], columns=[3.0, 3.0], cells=[["Nokta", "Y"], ["", "487000.125"]]),
        ),
        ("boş tablo", ent("table", "0", None, {}, p=P0, rotation=0.0, height=1.0, rows=[2.0], columns=[3.0], cells=[[""]])),
    ]
    return [{"name": n, "layers": layers, "blocks": blocks, "entity": e, "expected": record_of(e, layers, blocks)} for n, e in hand]


def build():
    drawings, searches = search_cases()
    return {
        "format": "kentos.search-fixtures",
        "version": 1,
        "match": match_cases(),
        "drawings": drawings,
        "search": searches,
        "names": [{"name": f"çizim {i}", "drawing": i, "expected": attribute_names(d)} for i, d in enumerate(drawings)],
        "records": record_cases(),
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--check", action="store_true", help="compare with the written file instead of writing it")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT} güncel değil: python3 scripts/fixtures/data_search_cases.py", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT}: güncel")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    data = json.loads(text)
    print(f"{OUT}: {len(data['match'])} eşleşme, {len(data['search'])} arama, {len(data['records'])} kayıt durumu")


if __name__ == "__main__":
    main()
