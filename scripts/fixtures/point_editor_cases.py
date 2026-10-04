#!/usr/bin/env python3
"""Independent reference of the point editor's computations (docs/adr/0153 §2, §3, §5, §6).

Writes fixtures/point-editor/v1/cases.json from the rules alone, with
Python's standard library and no KentOS code. The geometry core
(`text::natural`, `ops::point_editor`; crates/shared/geometry-core/tests/all/point_editor.rs)
and the web through its WASM must give the same: orders and groups exactly,
places and elevations within 1e-9 m.

Natural order:
1. A text is tokens: each run of ASCII digits is a number, every other
   character a token of its own.
2. Token by token: two numbers by value (leading zeros do not count); a
   number before any other character; two characters folded the Turkish way
   (I to ı, İ to i, any other to its lowercase when that is one character):
   first what is not a letter (by code point), then letters in the order
   abcçdefgğhıijklmnoöpqrsştuüvwxyz, then other letters by code point.
3. The text with fewer tokens first; last, the texts as they are, by code point.

Table: a row shows when it is selected (if only selected ones are asked), on
the layer asked (if one is), and when the search (trimmed) is empty or found
in its name or code (trimmed; caseless, folded as above; with a `*` the
whole value matches the pattern, `*` any run; without, it is a part). A sort
by name, code or layer is the natural order, by east, north or z the
numbers'; an empty value is after every present one, whichever the way;
equal values keep the drawing's order.

Duplicates: by name, names trimmed and compared exactly, unnamed ones never;
by place, each point in turn joins the first group whose first point is
within the tolerance (under 1e-9 m: the very same place), else starts one.
Groups of two or more keep their first, their last, or their first at the
members' mean place and the mean of the elevations they have.

Follow: every vertex within 1e-6 m of `from` goes to `to`; with setZ it
takes the elevation `z` (null: none); null when nothing changes.
"""

import argparse
import json
import math
import random
import re
import sys
from fractions import Fraction
from functools import cmp_to_key
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "point-editor" / "v1" / "cases.json"
ALPHABET = "abcçdefgğhıijklmnoöpqrsştuüvwxyz"
ON = 1e-6
E0, N0 = 487000.0, 4420000.0
JS_SPACE = " \t\n\v\f\r                 　﻿"


def cmp(a, b):
    return (a > b) - (a < b)


def fold(c):
    if c == "I":
        return "ı"
    if c == "İ":
        return "i"
    low = c.lower()
    return low if len(low) == 1 else c


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


def trimmed(v):
    if v is None:
        return None
    v = v.strip(JS_SPACE)
    return v or None


def found(value, pattern):
    v = "".join(fold(c) for c in value)
    p = "".join(fold(c) for c in pattern)
    if "*" in p:
        return re.fullmatch(".*".join(re.escape(part) for part in p.split("*")), v, re.S) is not None
    return p in v


def table(rows, q):
    wanted = q["search"].strip(JS_SPACE)
    shown = []
    for i, r in enumerate(rows):
        if q["onlySelected"] and not r["selected"]:
            continue
        if q["layer"] is not None and r["layer"] != q["layer"]:
            continue
        if wanted and not any(trimmed(v) is not None and found(trimmed(v), wanted) for v in (r["name"], r["code"])):
            continue
        shown.append(i)
    col, desc = q["sort"], q["descending"]
    if col not in ("name", "code", "east", "north", "z", "layer"):
        return shown

    def val(r):
        if col in ("name", "code"):
            return trimmed(r[col])
        return r[col]

    def by(a, b):
        x, y = val(rows[a]), val(rows[b])
        if x is not None and y is not None:
            o = natural_cmp(x, y) if col in ("name", "code", "layer") else cmp(x, y)
            o = -o if desc else o
        elif x is not None:
            o = -1
        elif y is not None:
            o = 1
        else:
            o = 0
        return o or cmp(a, b)

    return sorted(shown, key=cmp_to_key(by))


def mean(values):
    return float(sum(Fraction(v) for v in values) / len(values))


def duplicates(points, by, tolerance, keep):
    groups = []
    if by == "name":
        index = {}
        for i, p in enumerate(points):
            n = trimmed(p["name"])
            if n is None:
                continue
            if n in index:
                groups[index[n]].append(i)
            else:
                index[n] = len(groups)
                groups.append([i])
    else:
        exact = not (tolerance >= 1e-9)
        for i, p in enumerate(points):
            home = None
            for g, members in enumerate(groups):
                a = points[members[0]]["p"]
                if exact:
                    near = a["x"] == p["p"]["x"] and a["y"] == p["p"]["y"]
                else:
                    near = math.hypot(a["x"] - p["p"]["x"], a["y"] - p["p"]["y"]) <= tolerance
                if near:
                    home = g
                    break
            if home is None:
                groups.append([i])
            else:
                groups[home].append(i)
    out, removed = [], []
    for members in groups:
        if len(members) < 2:
            continue
        if keep == "last":
            kept = members[-1]
            p, z = points[kept]["p"], points[kept]["z"]
        elif keep == "average":
            kept = members[0]
            p = {"x": mean([points[i]["p"]["x"] for i in members]), "y": mean([points[i]["p"]["y"] for i in members])}
            zs = [points[i]["z"] for i in members if points[i]["z"] is not None]
            z = mean(zs) if zs else None
        else:
            kept = members[0]
            p, z = points[kept]["p"], points[kept]["z"]
        removed.extend(i for i in members if i != kept)
        out.append({"members": members, "kept": kept, "p": p, "z": z})
    return {"groups": out, "removed": sorted(removed)}


def follow(paths, frm, to, set_z, z):
    out = []
    for path in paths:
        pts = [dict(p) for p in path["pts"]]
        zs = list(path["zs"])
        if set_z and len(zs) < len(pts):
            zs += [None] * (len(pts) - len(zs))
        for k, p in enumerate(pts):
            if math.hypot(p["x"] - frm["x"], p["y"] - frm["y"]) <= ON:
                pts[k] = dict(to)
                if set_z:
                    zs[k] = z
        out.append({**path, "pts": pts, "zs": zs})
    return None if out == paths else out


# ── Cases ──────────────────────────────────────────────────────────────

def P(x, y):
    return {"x": x, "y": y}


def natural_cases():
    hand = [
        ("sayılar değerleriyle", ["P10", "P2", "P1", "P02"]),
        ("bölü ve tire", ["101/10", "101/2", "101", "101/1", "101-1"]),
        ("sıfır dolgusu", ["A-10", "A-009", "A-9", "A-0010"]),
        ("Türkçe harfler", ["Kuzey", "köşe", "Kale", "Köşe", "KÖŞE", "çam", "Çam", "can", "dal"]),
        ("ı, i, I, İ", ["i", "I", "ı", "İ", "ii", "Iı"]),
        ("sayı, işaret, harf", ["PA", "P-1", "P1", "P.1", "P 1", "P_1"]),
        ("boş metin", ["a", "", "1", " "]),
        ("büyük sayılar", ["18446744073709551617", "18446744073709551616", "9", "0"]),
        ("başka harfler", ["β", "a", "ω", "z", "ß"]),
    ]
    rnd = random.Random(53)
    for seed in range(1, 11):
        names = [random_name(rnd) or "" for _ in range(rnd.randint(5, 25))]
        hand.append((f"rastgele adlar {seed}", names))
    return [{"name": n, "names": names, "expected": sorted(range(len(names)), key=cmp_to_key(lambda a, b, ns=names: natural_cmp(ns[a], ns[b]) or cmp(a, b)))} for n, names in hand]


WORDS = ["Köşe", "köşe", "Kuzey", "ılık", "İnce", "ince", "çam", "Çam", "şev", "ağaç", "S", "ST"]


def random_name(rnd):
    k = rnd.random()
    n = rnd.randint(0, 120)
    if k < 0.12:
        return None
    if k < 0.35:
        return str(n)
    if k < 0.5:
        return f"P{n}"
    if k < 0.62:
        return f"{100 + rnd.randint(0, 3)}/{n}"
    if k < 0.72:
        return f"A-{n:03d}"
    if k < 0.8:
        return f"  {n} "
    if k < 0.92:
        return rnd.choice(WORDS) + (str(n) if rnd.random() < 0.5 else "")
    return rnd.choice(["p" + str(n), "a-" + str(n), str(n).zfill(4)])


def table_cases():
    rnd = random.Random(153)
    codes = ["SN", "ST", "bk", "BK", "yol", "Yol", "PK", None, "  ", "şev"]
    layers = ["nokta", "Nokta 2", "nokta10", "kot"]
    queries = [
        {"search": "", "layer": None, "onlySelected": False, "sort": None, "descending": False},
        {"search": "", "layer": None, "onlySelected": False, "sort": "name", "descending": False},
        {"search": "", "layer": None, "onlySelected": False, "sort": "name", "descending": True},
        {"search": "", "layer": None, "onlySelected": False, "sort": "code", "descending": False},
        {"search": "", "layer": None, "onlySelected": False, "sort": "east", "descending": True},
        {"search": "", "layer": None, "onlySelected": False, "sort": "north", "descending": False},
        {"search": "", "layer": None, "onlySelected": False, "sort": "z", "descending": False},
        {"search": "", "layer": None, "onlySelected": False, "sort": "z", "descending": True},
        {"search": "", "layer": None, "onlySelected": False, "sort": "layer", "descending": False},
        {"search": "1", "layer": None, "onlySelected": False, "sort": "name", "descending": False},
        {"search": " p* ", "layer": None, "onlySelected": False, "sort": None, "descending": False},
        {"search": "*0", "layer": None, "onlySelected": False, "sort": "name", "descending": False},
        {"search": "KÖŞE", "layer": None, "onlySelected": False, "sort": None, "descending": False},
        {"search": "İNCE", "layer": None, "onlySelected": False, "sort": None, "descending": False},
        {"search": "yol", "layer": None, "onlySelected": False, "sort": "east", "descending": False},
        {"search": "", "layer": "nokta", "onlySelected": False, "sort": "name", "descending": False},
        {"search": "", "layer": None, "onlySelected": True, "sort": None, "descending": False},
        {"search": "*", "layer": "kot", "onlySelected": True, "sort": "z", "descending": False},
        {"search": "", "layer": None, "onlySelected": False, "sort": "sıra", "descending": True},
    ]
    cases = []
    for seed in range(1, 7):
        rows = []
        for _ in range(rnd.randint(12, 30)):
            rows.append({
                "name": random_name(rnd),
                "east": round(E0 + rnd.uniform(-300, 300), 3),
                "north": round(N0 + rnd.uniform(-300, 300), 3),
                "z": round(rnd.uniform(95, 130), 3) if rnd.random() < 0.6 else None,
                "code": rnd.choice(codes),
                "layer": rnd.choice(layers),
                "selected": rnd.random() < 0.3,
            })
        # Ties: two rows with the same east, north and z.
        if len(rows) > 3:
            rows[2] = {**rows[2], "east": rows[0]["east"], "north": rows[0]["north"], "z": rows[0]["z"]}
        for k, q in enumerate(queries):
            cases.append({"name": f"tablo {seed}, sorgu {k + 1}", "rows": rows, "query": q, "expected": table(rows, q)})
    return cases


def pt(x, y, z=None, name=None):
    return {"p": P(x, y), "z": z, "name": name}


def duplicate_cases():
    hand = []
    # Same name: unnamed never; trimmed; three of 101 and two of 102.
    named = [pt(0, 0, 100.0, "101"), pt(5, 0, None, "102"), pt(0.02, 0.01, 100.5, " 101"), pt(9, 9, None, None), pt(9, 9, None, None), pt(5.01, 0, 99.0, "102"), pt(0.01, 0.0, None, "101 ")]
    for keep in ("first", "last", "average"):
        hand.append((f"aynı ad, {keep}", named, "name", 0.0, keep))
    # Same place within 1 mm: anchored at the first, no chains.
    chain = [pt(0, 0, 10.0, "1"), pt(0.0008, 0, 11.0, "2"), pt(0.0016, 0, 12.0, "3"), pt(0.0024, 0, None, "4"), pt(0.0006, 0.0006, None, "5")]
    for keep in ("first", "last", "average"):
        hand.append((f"aynı yer, zincir yok, {keep}", chain, "place", 0.001, keep))
    # The very same place under a nanometre; -0 is 0.
    same = [pt(1, 2, None, "a"), pt(1.0000000001, 2, None, "b"), pt(1, 2, 5.0, "c"), pt(-0.0, 0, None, "d"), pt(0.0, 0, None, "e")]
    hand.append(("tam aynı yer", same, "place", 0.0, "average"))
    hand.append(("tam aynı yer, eksi tolerans", same, "place", -1.0, "first"))
    # On the border: exactly the tolerance along one axis is inside; just over, outside.
    border = [pt(10, 10, None, "a"), pt(10.5, 10, None, "b"), pt(10, 10.5000001, None, "c")]
    hand.append(("sınırda", border, "place", 0.5, "first"))
    hand.append(("çift yok", [pt(0, 0), pt(1, 1), pt(2, 2, None, "x")], "name", 0.0, "first"))
    hand.append(("boş girdi", [], "place", 0.001, "average"))
    cases = []
    for name, pts, by, tol, keep in hand:
        cases.append((name, pts, by, tol, keep))
        moved = [{**p, "p": P(p["p"]["x"] + E0, p["p"]["y"] + N0)} for p in pts]
        cases.append((name + ", TM koordinatlarında", moved, by, tol, keep))
    rnd = random.Random(1530)
    for seed in range(1, 13):
        pts = []
        for i in range(rnd.randint(8, 30)):
            x, y = E0 + rnd.uniform(-50, 50), N0 + rnd.uniform(-50, 50)
            pts.append(pt(round(x, 3), round(y, 3), round(rnd.uniform(100, 120), 3) if rnd.random() < 0.5 else None, random_name(rnd)))
        # Measured again: within a millimetre, or a few millimetres off; same name or another.
        for _ in range(rnd.randint(2, 8)):
            src = rnd.choice(pts)
            dx, dy = rnd.choice([0.0, 0.0003, -0.0006, 0.0004, 0.003, -0.0025]), rnd.choice([0.0, 0.0002, -0.0005, 0.004])
            name = src["name"] if rnd.random() < 0.6 else random_name(rnd)
            pts.insert(rnd.randint(0, len(pts)), pt(src["p"]["x"] + dx, src["p"]["y"] + dy, round(rnd.uniform(100, 120), 3) if rnd.random() < 0.5 else None, name))
        by = rnd.choice(["name", "place"])
        cases.append((f"rastgele ölçüler {seed}", pts, by, rnd.choice([0.001, 0.005]), rnd.choice(["first", "last", "average"])))
    return [{"name": n, "points": pts, "by": by, "tolerance": tol, "keep": keep, "expected": duplicates(pts, by, tol, keep)} for n, pts, by, tol, keep in cases]


def path(pts, zs=None, closed=False, bulges=None):
    return {"pts": [P(*p) for p in pts], "bulges": bulges, "closed": closed, "zs": zs if zs is not None else [None] * len(pts)}


def follow_cases():
    hand = []
    line = [path([(0, 0), (10, 0)], [100.0, 101.0])]
    hand.append(("çizginin ucu taşınır, kotu kalır", line, P(10, 0), P(12, 1), False, None))
    hand.append(("çizginin ucu yeni kotu alır", line, P(10, 0), P(10, 0), True, 99.5))
    hand.append(("çizginin ucunun kotu kalkar", line, P(10, 0), P(10, 0), True, None))
    hand.append(("aynı kot: değişiklik yok", line, P(10, 0), P(10, 0), True, 101.0))
    hand.append(("yerinde köşe yok", line, P(5, 0), P(6, 0), False, None))
    poly = [path([(0, 0), (5, 0), (5, 5), (0.0000005, 0)], None, False, [0.0, 0.4, 0.0, 0.0])]
    hand.append(("çoklu çizgi: 1 µm içindeki uç da taşınır, büküm kalır", poly, P(0, 0), P(-1, -1), False, None))
    hand.append(("çoklu çizgi: 2 µm ötedeki köşe kalır", [path([(0, 0), (5, 0.000002), (5, 5)])], P(5, 0), P(6, 0), False, None))
    area = [
        path([(0, 0), (20, 0), (20, 15), (0, 15)], [100.0, None, None, 101.0], True),
        path([(5, 5), (5, 10), (10, 10), (10, 5)], None, True),
        path([(20, 15), (30, 15), (30, 20)], None, True),
    ]
    hand.append(("alan: dış halka, delik ve parça", area, P(20, 15), P(21, 16), True, 102.25))
    hand.append(("alan: delikteki köşe", area, P(10, 10), P(11, 11), False, None))
    hand.append(("kotsuz yol kot alır", [path([(0, 0), (1, 1)], [])], P(1, 1), P(1, 1), True, 7.0))
    cases = []
    for name, paths, frm, to, set_z, z in hand:
        cases.append((name, paths, frm, to, set_z, z))
        mv = lambda p: P(p["x"] + E0, p["y"] + N0)
        moved = [{**pa, "pts": [mv(p) for p in pa["pts"]]} for pa in paths]
        cases.append((name + ", TM koordinatlarında", moved, mv(frm), mv(to), set_z, z))
    rnd = random.Random(15300)
    for seed in range(1, 11):
        at = P(round(E0 + rnd.uniform(-20, 20), 3), round(N0 + rnd.uniform(-20, 20), 3))
        paths = []
        for _ in range(rnd.randint(1, 3)):
            pts = []
            for _ in range(rnd.randint(2, 6)):
                r = rnd.random()
                if r < 0.3:
                    q = (at["x"], at["y"])
                elif r < 0.4:
                    q = (at["x"] + 4e-7, at["y"] - 3e-7)
                elif r < 0.5:
                    q = (at["x"] + 2e-6, at["y"])
                else:
                    q = (round(E0 + rnd.uniform(-20, 20), 3), round(N0 + rnd.uniform(-20, 20), 3))
                pts.append(q)
            zs = [round(rnd.uniform(100, 110), 3) if rnd.random() < 0.5 else None for _ in pts]
            paths.append(path(pts, zs, rnd.random() < 0.5))
        to = P(at["x"] + rnd.choice([0.0, 0.25, -1.5]), at["y"] + rnd.choice([0.0, 0.75]))
        set_z = rnd.random() < 0.5
        cases.append((f"rastgele yollar {seed}", paths, at, to, set_z, round(rnd.uniform(90, 100), 3) if rnd.random() < 0.7 else None))
    return [{"name": n, "paths": pa, "from": f, "to": t, "setZ": s, "z": z, "expected": follow(pa, f, t, s, z)} for n, pa, f, t, s, z in cases]


def build():
    return {
        "format": "kentos.point-editor-fixtures",
        "version": 1,
        "natural": natural_cases(),
        "table": table_cases(),
        "duplicates": duplicate_cases(),
        "follow": follow_cases(),
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--check", action="store_true", help="compare with the written file instead of writing it")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if OUT.read_text() != text:
            print(f"{OUT} güncel değil: python3 scripts/fixtures/point_editor_cases.py", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT}: güncel")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    data = json.loads(text)
    print(f"{OUT}: {len(data['natural'])} sıra, {len(data['table'])} tablo, {len(data['duplicates'])} çift, {len(data['follow'])} bağlı köşe durumu")


if __name__ == "__main__":
    main()
