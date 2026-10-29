#!/usr/bin/env python3
"""The shared cases of docs/adr/0733 (ARCH-DOC-02): the fixtures schedule, the room finish schedule, the type filter,
headings and subtotals (`cad.schedules.create`), a cell that is no object's own field and a subtotal's row refused
(`cad.schedules.edit`), and `cad.schedules.query`'s columns and rows; written from the ADR's rules without KentOS's code.

Writes fixtures/commands/v1/cad.schedules.create.extras.json, cad.schedules.edit.extras.json and
cad.schedules.query.json; `--check` compares.

The rules (docs/adr/0733 §2–§6, docs/adr/0344 §3):
- A fixtures schedule lists the block inserts; its type is the block's name, its category the block card's, its space
  the number and name of the space on its level whose ring holds its point (spaces here are rectangles), its level and
  phase its own.
- A space's finishes are its own fields; its baseboard its perimeter less its doorways (none here: its perimeter).
- Rows in the drawing's order, sorted by the keys (numbers by value, words in the natural order, empties last), alike
  rows merged when grouped (summed: count, area, perimeter, baseboard).
- A sort key with `header` puts its value before each group, across the row (merged), bold; with `footer` after it
  “‹value› toplamı” in its column (or the first not summed) and the group's sums, bold; an inner key's footer before
  an outer one's; the total last. Header, footer and total rows have no members.
- A column that is not the objects' own field (a fixture's type is its block's) is grey over each run of data rows.
- `cad.schedules.query`'s roles: heading (row 0), total (the last when the schedule has one), data (with members),
  header (a merged row), footer.
"""

import argparse
import json
import math
import sys
from functools import cmp_to_key
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import arch_command_cases as acc  # noqa: E402  (refusals, numbers as Rust writes them)
import point_editor_cases as pe  # noqa: E402  (the natural order)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
P, refusal, fixed, layer, uid_of = acc.P, acc.refusal, acc.fixed, acc.layer, acc.uid_of
SCALE = 50
GREY = "#7f8790"
CREATE, EDIT, QUERY = "cad.schedules.create", "cad.schedules.edit", "cad.schedules.query"

LEVELS = [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
          {"id": "kat-1", "name": "1. kat", "elevation": 3.0, "height": 3.0}]
PHASES = {"existing": "Mevcut", "demolish": "Yıkılacak", "new": "Yeni", "temporary": "Geçici"}
B_KOLTUK, B_LAVABO, B_ANTET = ("0192f5a1-7330-7000-8000-0000000000b1", "0192f5a1-7330-7000-8000-0000000000b2",
                               "0192f5a1-7330-7000-8000-0000000000b3")


def block(bid, name, category=None):
    b = {"id": bid, "name": name, "base": P(0, 0),
         "entities": [{"kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": P(0, 0), "b": P(0.5, 0)}]}
    if category:
        b["card"] = {"category": category, "version": 1}
    return b


def rect(x0, y0, x1, y1):
    return [{"outer": {"pts": [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]}}]


def space(number, name, usage, level, box, finishes):
    f = {"parts": rect(*box), "number": number, "name": name, "arch": {"level": level}, "finishes": finishes}
    if usage:
        f["usage"] = usage
    return f


def insert(b, at, arch=None, attrs=None):
    f = {"block": b, "p": at, "scale": 1, "rotation": 0}
    if arch:
        f["arch"] = arch
    return f, attrs or {}


SEED = {
    1: ("space", "mahal", space("101", "Salon", "Oturma", "zemin", (0, 0, 5, 4),
                                {"floor": "Parke", "wall": "Boya", "ceiling": "Alçı", "base": "Ahşap"}), {}),
    2: ("space", "mahal", space("102", "Banyo", "Islak", "zemin", (5, 0, 8, 4), {"floor": "Seramik", "wall": "Seramik"}), {}),
    3: ("space", "mahal", space("201", "Oda", "Yatak", "kat-1", (0, 0, 5, 4), {"floor": "Laminat"}), {}),
    4: ("insert", "donati", *insert(B_KOLTUK, P(1, 1), {"level": "zemin"}, {"Renk": "Gri"})),
    5: ("insert", "donati", *insert(B_KOLTUK, P(2, 1), {"level": "zemin"})),
    6: ("insert", "donati", *insert(B_LAVABO, P(6, 1), {"level": "zemin", "phase": "new"})),
    7: ("insert", "donati", *insert(B_KOLTUK, P(1, 1), {"level": "kat-1"})),
    8: ("insert", "donati", *insert(B_ANTET, P(20, 20))),
    9: ("insert", "kilitli", *insert(B_LAVABO, P(6, 2), {"level": "zemin"})),
}
NEW = 10
SETUP = {
    "format": "kentos.document", "version": 1, "name": "Donatı ve kaplama çizelgeleri",
    "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                 "plotScale": SCALE, "workspace": "cad", "arch": {"levels": LEVELS}},
    "origin": {"x": 0, "y": 0},
    "layers": [layer("mahal", "Mahal"), layer("donati", "Donatı"), layer("cizelge", "Çizelge"),
               layer("kilitli", "Kilitli katman", locked=True)],
    "activeLayer": "cizelge",
    "styles": {"items": [], "categories": []},
    "blocks": [block(B_KOLTUK, "Koltuk", "Mobilya/Oturma"), block(B_LAVABO, "Lavabo", "Vitrifiye/Lavabo"),
               block(B_ANTET, "Antet")],
    "entities": [{"kind": k, "id": i, "layerId": ly, "attrs": at, **f} for i, (k, ly, f, at) in sorted(SEED.items())],
}
BLOCKS = {b["id"]: b for b in SETUP["blocks"]}

TITLES = {"type": "Tür", "category": "Kategori", "space": "Mahal", "level": "Kat", "phase": "Evre", "count": "Adet",
          "number": "No", "name": "Ad", "usage": "Kullanım", "floorFinish": "Döşeme", "baseFinish": "Süpürgelik",
          "wallFinish": "Duvar kaplaması", "ceilingFinish": "Tavan", "area": "Alan", "perimeter": "Çevre",
          "baseboard": "Süpürgelik boyu"}
LENGTHS = {"perimeter", "baseboard"}
SUMMED = {"count", "area", "perimeter", "baseboard"}
NUMERIC = LENGTHS | {"count", "area"}
EDITABLE = {"number", "type", "name", "usage", "attr", "floorFinish", "baseFinish", "wallFinish", "ceilingFinish"}
READS = {"fixtures": {"type", "category", "space", "level", "phase", "count", "attr"},
         "spaces": {"number", "name", "usage", "level", "phase", "count", "attr", "area", "perimeter", "floorFinish",
                    "baseFinish", "wallFinish", "ceilingFinish", "baseboard"}}
DEFAULT = {"fixtures": ["type", "category", "space", "level", "count"]}
OF_LABEL = {"fixtures": "Donatı çizelgesi", "spaces": "Mahal çizelgesi"}
KIND = {"fixtures": "insert", "spaces": "space"}


def edits_in(of, field):
    return field in EDITABLE and not (of == "fixtures" and field == "type")


def heading(c):
    if c.get("title"):
        return c["title"]
    f = c["field"]
    if f == "attr":
        return c["attr"]
    if f in LENGTHS:
        return f"{TITLES[f]} (m)"
    if f == "area":
        return "Alan (m²)"
    return TITLES[f]


def level_name(level):
    return next((x["name"] for x in LEVELS if x["id"] == level), level) if level else None


def middle(f, b):
    """Where a fixture stands: the middle of its block's lines, placed as x' = p + R·S·M·x (M mirrors y)."""
    pts = [q for e in b["entities"] for q in (e["a"], e["b"])]
    xs, ys = [q["x"] for q in pts], [q["y"] for q in pts]
    x, y = (min(xs) + max(xs)) / 2 - b["base"]["x"], (min(ys) + max(ys)) / 2 - b["base"]["y"]
    sx = f.get("scale", 1.0)
    sy = f.get("yScale", sx) * (-1.0 if f.get("mirror") else 1.0)
    c, s = math.cos(f.get("rotation", 0.0)), math.sin(f.get("rotation", 0.0))
    return {"x": f["p"]["x"] + sx * c * x - sy * s * y, "y": f["p"]["y"] + sx * s * x + sy * c * y}


def space_at(objs, p, level):
    for i, (k, _, f, _) in sorted(objs.items()):
        if k != "space" or f.get("arch", {}).get("level") != level:
            continue
        pts = f["parts"][0]["outer"]["pts"]
        xs, ys = [q["x"] for q in pts], [q["y"] for q in pts]
        if min(xs) < p["x"] < max(xs) and min(ys) < p["y"] < max(ys):
            words = " ".join(w for w in (f.get("number"), f.get("name")) if w)
            return words or None
    return None


def value(objs, i, field, attr=None):
    k, _, f, at = objs[i]
    if field == "attr":
        v = at.get(attr)
        return ("text", v) if v else None
    if field == "count":
        return ("count", 1)
    arch = f.get("arch", {})
    if field == "level":
        n = level_name(arch.get("level"))
        return ("text", n) if n else None
    if field == "phase":
        return ("text", PHASES[arch["phase"]]) if arch.get("phase") else None
    if k == "insert":
        b = BLOCKS[f["block"]]
        if field == "type":
            return ("text", b["name"])
        if field == "category":
            c = b.get("card", {}).get("category")
            return ("text", c) if c else None
        if field == "space":
            w = space_at(objs, middle(f, b), arch.get("level")) or space_at(objs, f["p"], arch.get("level"))
            return ("text", w) if w else None
        return None
    pts = f["parts"][0]["outer"]["pts"]
    w = max(q["x"] for q in pts) - min(q["x"] for q in pts)
    h = max(q["y"] for q in pts) - min(q["y"] for q in pts)
    if field in ("number", "name", "usage"):
        return ("text", f[field]) if f.get(field) else None
    if field.endswith("Finish"):
        v = f.get("finishes", {}).get(field[: -len("Finish")])
        return ("text", v) if v else None
    if field == "area":
        return ("area", w * h)
    if field in ("perimeter", "baseboard"):
        return ("length", 2 * (w + h))
    return None


def text_of(v):
    if v is None:
        return ""
    kind, x = v
    if kind == "text":
        return x
    if kind == "length":
        return fixed(x, 3)
    if kind == "area":
        return fixed(x, 2)
    return str(x)


def add(a, b):
    if a is None or b is None:
        return a or b
    return (a[0], a[1] + b[1])


def takes(objs, i, flt):
    if not flt:
        return True
    k, ly, f, _ = objs[i]
    arch = f.get("arch", {})
    if "layers" in flt and ly not in flt["layers"]:
        return False
    if "levels" in flt and arch.get("level") not in flt["levels"]:
        return False
    if "phases" in flt and arch.get("phase") not in flt["phases"]:
        return False
    if "types" in flt:
        t = text_of(value(objs, i, "usage" if k == "space" else "type"))
        if not t or t not in flt["types"]:
            return False
    return True


def rows_of(objs, src):
    """The cells, members, merges, styles and aligns of a schedule (docs/adr/0733 §4)."""
    of, columns = src["of"], src["columns"]
    width = len(columns)
    rows = []
    for i in sorted(objs):
        if objs[i][0] != KIND[of] or not takes(objs, i, src.get("filter")):
            continue
        vals = [value(objs, i, c["field"], c.get("attr")) for c in columns]
        rows.append({"vals": vals, "texts": [text_of(v) for v in vals], "uids": [uid_of(i)]})
    sort = src.get("sort", [])

    def cmp(a, b):
        for s in sort:
            j = s["column"]
            ea, eb = a["texts"][j] == "", b["texts"][j] == ""
            if ea and eb:
                o = 0
            elif ea:
                o = 1
            elif eb:
                o = -1
            else:
                x, y = a["vals"][j], b["vals"][j]
                if x[0] != "text" and y[0] != "text":
                    o = (x[1] > y[1]) - (x[1] < y[1])
                else:
                    o = pe.natural_cmp(a["texts"][j], b["texts"][j])
                if s.get("descending"):
                    o = -o
            if o:
                return o
        return 0
    rows.sort(key=cmp_to_key(cmp))
    summed = [c["field"] in SUMMED for c in columns]
    if src.get("group"):
        merged = []
        for r in rows:
            key = [t for t, s in zip(r["texts"], summed) if not s]
            m = next((m for m in merged if [t for t, s in zip(m["texts"], summed) if not s] == key), None)
            if m is None:
                merged.append(r)
                continue
            for j, s in enumerate(summed):
                if s:
                    m["vals"][j] = add(m["vals"][j], r["vals"][j])
            m["uids"] += r["uids"]
        for m in merged:
            for j, s in enumerate(summed):
                if s:
                    m["texts"][j] = text_of(m["vals"][j])
        rows = merged
    cells = [[heading(c) for c in columns]]
    members, merges, styles, runs = [], [], [], []

    def bold(r):
        return {"row": r, "col": 0, "rows": 1, "cols": width, "bold": True}

    marked = [k for k in sort if k.get("header") or k.get("footer")]

    def footer(d, sums, v):
        out = [text_of(x) if s and x else "" for x, s in zip(sums, summed)]
        for j, c in enumerate(columns):
            if c["field"] == "count" and not out[j]:
                out[j] = "0"
        k = marked[d]["column"]
        label = k if not summed[k] else next((j for j in range(width) if not summed[j]), None)
        if label is not None:
            out[label] = f"{v or '(boş)'} toplamı"
        return out
    sums = [None] * width
    gsums = [[None] * width for _ in marked]
    gvals = [""] * len(marked)
    prev = None
    for r in rows:
        keys = [r["texts"][k["column"]] for k in marked]
        frm = 0 if prev is None else next((d for d, (a, b) in enumerate(zip(prev, keys)) if a != b), len(marked))
        if prev is not None:
            for d in reversed(range(frm, len(marked))):
                if marked[d].get("footer"):
                    styles.append(bold(len(cells)))
                    cells.append(footer(d, gsums[d], gvals[d]))
                    members.append([])
        for d in range(frm, len(marked)):
            gsums[d] = [None] * width
            gvals[d] = keys[d]
            if marked[d].get("header"):
                at = len(cells)
                if width > 1:
                    merges.append({"row": at, "col": 0, "rows": 1, "cols": width})
                styles.append(bold(at))
                cells.append([keys[d] or "(boş)"] + [""] * (width - 1))
                members.append([])
        for j, s in enumerate(summed):
            if s:
                sums[j] = add(sums[j], r["vals"][j])
                for g in gsums:
                    g[j] = add(g[j], r["vals"][j])
        at = len(cells)
        if runs and runs[-1][0] + runs[-1][1] == at:
            runs[-1][1] += 1
        else:
            runs.append([at, 1])
        cells.append(r["texts"])
        members.append(r["uids"])
        prev = keys
    if prev is not None:
        for d in reversed(range(len(marked))):
            if marked[d].get("footer"):
                styles.append(bold(len(cells)))
                cells.append(footer(d, gsums[d], gvals[d]))
                members.append([])
    for first, n in runs:
        for j, c in enumerate(columns):
            if not edits_in(of, c["field"]):
                styles.append({"row": first, "col": j, "rows": n, "cols": 1, "color": GREY})
    if src.get("total"):
        last = [text_of(v) if (s and v) else "" for v, s in zip(sums, summed)]
        for j, c in enumerate(columns):
            if c["field"] == "count" and not last[j]:
                last[j] = "0"
        if not summed[0]:
            last[0] = "Toplam"
        styles.append(bold(len(cells)))
        cells.append(last)
    aligns = ["right" if c["field"] in NUMERIC else "left" for c in columns]
    return cells, members, merges, styles, aligns


def problem(of, columns, sort, flt):
    for i, c in enumerate(columns):
        if c["field"] not in READS[of]:
            return refusal("invalid_schedule", f"“{TITLES[c['field']]}” {OF_LABEL[of].lower()}nde okunmaz.",
                           f"columns[{i}].field")
    for i, s in enumerate(sort):
        if s["column"] >= len(columns):
            return refusal("invalid_schedule", f"Sıralama {s['column'] + 1}. sütunu istiyor; çizelgenin {len(columns)} sütunu var.",
                           f"sort[{i}].column")
    for i, t in enumerate((flt or {}).get("types", [])):
        if not t.strip():
            return refusal("invalid_schedule", "Tür süzgecinin her adı boş olmayan tek satırdır, en çok 200 harftir.",
                           f"filter.types[{i}]")
    return None


def source(inp):
    src = {"kind": "schedule", "of": inp["of"],
           "columns": inp.get("columns") or [{"field": f} for f in DEFAULT[inp["of"]]]}
    for key in ("filter", "sort"):
        if inp.get(key):
            src[key] = inp[key]
    for key in ("group", "total"):
        if inp.get(key):
            src[key] = True
    return src


def table(inp, objs):
    src = source(inp)
    cells, members, merges, styles, aligns = rows_of(objs, src)
    t = {"kind": "table", "id": 0, "layerId": inp["layerId"], "attrs": {}, "p": inp["at"], "rotation": 0.0,
         "height": 2.5 / 1000 * SCALE, "rows": "$any", "columns": "$any", "cells": cells, "header": True,
         "aligns": aligns, "source": src}
    if merges:
        t["merges"] = merges
    if styles:
        t["styles"] = styles
    if members:
        src["members"] = members
    return t


def create(inp, op="plan"):
    s = {"op": op, "command": CREATE, "input": inp}
    src = source(inp)
    r = problem(inp["of"], src["columns"], src.get("sort", []), src.get("filter"))
    if r:
        s["result"] = r
        s["expect"] = {"revision": "same"}
        return s
    t = table(inp, SEED)
    if op == "plan":
        s["result"] = {"status": "completed", "output": {"table": t, "revision": "$current"}, "warnings": []}
        s["within"] = 1e-9
        s["expect"] = {"revision": "same"}
    else:
        rows = sum(1 for m in t["source"].get("members", []) if m)
        s["result"] = {"status": "completed", "output": {"uid": "$uid", "id": NEW, "rows": rows, "revision": "$current"},
                       "warnings": []}
        s["expect"] = {"revision": "changed"}
    return s


def query(inp, objs=SEED, made=NEW):
    """`cad.schedules.query` on the schedule `create(inp)` wrote (slot `made`)."""
    src = source(inp)
    cells, members, merges, _, _ = rows_of(objs, src)
    heads = {m["row"] for m in merges}
    rows = []
    for r, c in enumerate(cells):
        mem = members[r - 1] if 0 < r <= len(members) else []
        if r == 0:
            role = "heading"
        elif src.get("total") and r == len(cells) - 1:
            role = "total"
        elif mem:
            role = "data"
        elif r in heads:
            role = "header"
        else:
            role = "footer"
        rows.append({"row": r, "role": role, "cells": c, "members": mem if role == "data" else []})
    columns = []
    for c in src["columns"]:
        col = {"heading": heading(c), "field": c["field"]}
        if c.get("attr"):
            col["attr"] = c["attr"]
        col["editable"] = edits_in(src["of"], c["field"])
        col["summed"] = c["field"] in SUMMED
        columns.append(col)
    return {"op": "execute", "command": QUERY, "input": {"uid": uid_of(made)},
            "result": {"status": "completed", "output": {"of": src["of"], "columns": columns, "rows": rows,
                                                         "revision": "$current"}, "warnings": []},
            "expect": {"revision": "same"}}


class Cases:
    def __init__(self):
        self.cases = []

    def add(self, name, steps):
        self.cases.append({"name": name, "steps": steps})


FX = {"layerId": "cizelge", "at": P(30, 0), "of": "fixtures"}
FIN = [{"field": f} for f in ("number", "name", "floorFinish", "baseFinish", "wallFinish", "ceilingFinish", "area",
                              "baseboard")]
SP = {"layerId": "cizelge", "at": P(30, 0), "of": "spaces"}
BY_SPACE = [{"field": "space"}, {"field": "type"}, {"field": "count"}]


def create_cases():
    c = Cases()
    c.add("Donatı çizelgesi: blok yerleştirmeleri, tür, kategori, mahal, kat, adet", [create(FX)])
    c.add("Donatı çizelgesi: aynı türler bir satır, genel toplam",
          [create({**FX, "columns": [{"field": "type"}, {"field": "category"}, {"field": "count"}], "group": True, "total": True,
                   "sort": [{"column": 0}]})])
    c.add("Donatı çizelgesi: yalnız zemin kat", [create({**FX, "filter": {"levels": ["zemin"]}})])
    c.add("Donatı çizelgesi: tür süzgeci Koltuk", [create({**FX, "filter": {"types": ["Koltuk"]}})])
    c.add("Donatı çizelgesi: tür süzgeci iki tür", [create({**FX, "filter": {"types": ["Lavabo", "Antet"]}})])
    c.add("Donatı çizelgesi: evre Yeni", [create({**FX, "filter": {"phases": ["new"]}})])
    c.add("Donatı çizelgesi: mahale göre grup başlığı",
          [create({**FX, "columns": BY_SPACE, "sort": [{"column": 0, "header": True}]})])
    c.add("Donatı çizelgesi: mahale göre ara toplam ve genel toplam",
          [create({**FX, "columns": BY_SPACE, "sort": [{"column": 0, "footer": True}], "total": True})])
    c.add("Donatı çizelgesi: başlık ve ara toplam birlikte",
          [create({**FX, "columns": BY_SPACE, "sort": [{"column": 0, "header": True, "footer": True}], "total": True})])
    c.add("Donatı çizelgesi: kata, sonra türe göre iç içe ara toplamlar",
          [create({**FX, "columns": [{"field": "level"}, {"field": "type"}, {"field": "count"}],
                   "sort": [{"column": 0, "footer": True}, {"column": 1, "footer": True}], "total": True})])
    c.add("Donatı çizelgesi: gruplu ve ara toplamlı",
          [create({**FX, "columns": [{"field": "level"}, {"field": "type"}, {"field": "count"}], "group": True,
                   "sort": [{"column": 0, "footer": True}, {"column": 1}], "total": True})])
    c.add("Donatı çizelgesi: azalan sırayla ara toplam",
          [create({**FX, "columns": BY_SPACE, "sort": [{"column": 0, "descending": True, "footer": True}]})])
    c.add("Donatı çizelgesi: blok özniteliği sütunu",
          [create({**FX, "columns": [{"field": "type"}, {"field": "attr", "attr": "Renk"}]})])
    c.add("Donatı çizelgesi: hiç satırı yok", [create({**FX, "filter": {"types": ["Dolap"]}, "total": True})])
    c.add("Donatı çizelgesi: yazılır", [create(FX, "execute")])
    c.add("Mahal kaplama çizelgesi: döşeme, süpürgelik, duvar, tavan, alan, süpürgelik boyu",
          [create({**SP, "columns": FIN, "total": True})])
    c.add("Mahal çizelgesi: kullanım süzgeci", [create({**SP, "columns": FIN, "filter": {"types": ["Islak", "Yatak"]}})])
    c.add("Mahal kaplama çizelgesi: kata göre başlık ve ara toplam",
          [create({**SP, "columns": [{"field": "level"}] + FIN[:3] + [{"field": "area"}],
                   "sort": [{"column": 0, "header": True, "footer": True}], "total": True})])
    c.add("Mahal kaplama çizelgesi: döşemeye göre ara toplam, boş değerin toplamı",
          [create({**SP, "columns": [{"field": "baseFinish"}, {"field": "area"}], "sort": [{"column": 0, "footer": True}]})])
    c.add("Ret: kategori mahal çizelgesinde okunmaz", [create({**SP, "columns": [{"field": "category"}]})])
    c.add("Ret: döşeme kaplaması donatı çizelgesinde okunmaz", [create({**FX, "columns": [{"field": "floorFinish"}]})])
    c.add("Ret: boş tür adı", [create({**FX, "filter": {"types": [" "]}})])
    return c.cases


def made(inp):
    return create(inp, "execute")


def edit(inp, row, column, value, result, then=None):
    s = {"op": "execute", "command": EDIT, "input": {"uid": uid_of(NEW), "operation": {"kind": "cell", "row": row,
                                                                                       "column": column, "value": value}}}
    s["result"] = result
    s["expect"] = {"revision": "changed" if result["status"] == "completed" else "same"}
    steps = [made(inp), s]
    if then:
        steps.append(then)
    return steps


def edit_cases():
    c = Cases()
    fin = {**SP, "columns": FIN}
    after = {k: v for k, v in SEED.items()}
    k, ly, f, at = after[2]
    after[2] = (k, ly, {**f, "finishes": {**f["finishes"], "floor": "Granit"}}, at)
    c.add("Kaplama hücresi mahale yazılır", edit(fin, 2, 2, "Granit",
          {"status": "completed", "output": {"table": "$any", "edited": [uid_of(2)], "revision": "$current"}, "warnings": []},
          query(fin, after)))
    after2 = dict(SEED)
    k, ly, f, at = after2[3]
    after2[3] = (k, ly, {**f, "finishes": {**f["finishes"], "base": "PVC"}}, at)
    c.add("Süpürgelik kaplaması boş mahale yazılır", edit(fin, 3, 3, "PVC",
          {"status": "completed", "output": {"table": "$any", "edited": [uid_of(3)], "revision": "$current"}, "warnings": []},
          query(fin, after2)))
    c.add("Ret: donatının türü bloğunundur", edit(FX, 1, 0, "Kanepe", refusal(
        "cell_computed", "“Tür” sütunu çizimden hesaplanır; düzenlenmez. Kaynağın kendi alanını değiştirin.",
        "operation.column")))
    c.add("Ret: mahal sütunu hesaplanır", edit(FX, 1, 2, "102", refusal(
        "cell_computed", "“Mahal” sütunu çizimden hesaplanır; düzenlenmez. Kaynağın kendi alanını değiştirin.",
        "operation.column")))
    # An attribute column: written to its objects, so a heading's and a subtotal's row are what refuse.
    grouped = {**FX, "columns": [{"field": "space"}, {"field": "attr", "attr": "Renk"}, {"field": "count"}],
               "sort": [{"column": 0, "header": True, "footer": True}]}
    c.add("Ret: grup başlığının satırı", edit(grouped, 1, 1, "X", refusal(
        "schedule_summary_row", "Çizelgenin 1. satırı bir grup başlığı ya da ara toplam; düzenlenmez.", "operation.row")))
    c.add("Ret: ara toplamın satırı", edit(grouped, 4, 1, "X", refusal(
        "schedule_summary_row", "Çizelgenin 4. satırı bir grup başlığı ya da ara toplam; düzenlenmez.", "operation.row")))
    after3 = dict(SEED)
    k, ly, f, at = after3[5]
    after3[5] = (k, ly, f, {**at, "Renk": "Mavi"})
    attr = {**FX, "columns": [{"field": "type"}, {"field": "attr", "attr": "Renk"}]}
    c.add("Donatının özniteliği yazılır", edit(attr, 2, 1, "Mavi",
          {"status": "completed", "output": {"table": "$any", "edited": [uid_of(5)], "revision": "$current"}, "warnings": []},
          query(attr, after3)))
    c.add("Ret: kilitli katmandaki donatı", edit(attr, 6, 1, "Beyaz", refusal(
        "layer_locked", "“Kilitli katman” katmanı kilitli; çizelgenin satırı ya da kendisi düzenlenemez. Kilidi Katmanlar panelinden açın.",
        "uid")))
    # Each finish of each space, and a finish cleared with empty words.
    for row, col, word, field in [(1, 2, "Mermer", "floor"), (2, 3, "Seramik", "base"), (1, 4, "Duvar kağıdı", "wall"),
                                  (2, 5, "Asma tavan", "ceiling"), (3, 4, "Boya", "wall"), (3, 5, "Alçıpan", "ceiling"),
                                  (1, 3, "", "base"), (2, 2, "", "floor")]:
        after = dict(SEED)
        k, ly, f, at = after[row]
        fin = dict(f["finishes"])
        if word:
            fin[field] = word
        else:
            fin.pop(field, None)
        after[row] = (k, ly, {**f, "finishes": fin}, at)
        what = f"{word} yazılır" if word else "boşaltılır"
        c.add(f"Mahal {f['number']}: {TITLES[field + 'Finish'].lower()} {what}", edit(fin_spec(), row, col, word,
              {"status": "completed", "output": {"table": "$any", "edited": [uid_of(row)], "revision": "$current"}, "warnings": []},
              query(fin_spec(), after)))
    # Worked-out columns of the finishes schedule and the total's row.
    for col, title in [(6, "Alan (m²)"), (7, "Süpürgelik boyu (m)")]:
        c.add(f"Ret: {title} hesaplanır", edit(fin_spec(), 1, col, "9", refusal(
            "cell_computed", f"“{title}” sütunu çizimden hesaplanır; düzenlenmez. Kaynağın kendi alanını değiştirin.",
            "operation.column")))
    total = {**SP, "columns": FIN, "total": True}
    c.add("Ret: genel toplamın satırı", edit(total, 4, 2, "X", refusal(
        "cell_out_of_range", "Çizelgenin 4. satırı veri satırı değil; veri satırları 1–3 (0 başlık, sonuncusu toplam).",
        "operation.row")))
    for col, title in [(1, "Kategori"), (3, "Kat")]:
        c.add(f"Ret: donatının {title.lower()} sütunu hesaplanır", edit(FX, 2, col, "X", refusal(
            "cell_computed", f"“{title}” sütunu çizimden hesaplanır; düzenlenmez. Kaynağın kendi alanını değiştirin.",
            "operation.column")))
    return c.cases


def fin_spec():
    return {**SP, "columns": FIN}


def query_cases():
    c = Cases()
    for name, inp in [
        ("Donatı çizelgesi", FX),
        ("Donatı çizelgesi, gruplu ve toplamlı", {**FX, "columns": [{"field": "type"}, {"field": "count"}], "group": True, "total": True}),
        ("Mahale göre grup başlıkları", {**FX, "columns": BY_SPACE, "sort": [{"column": 0, "header": True}]}),
        ("Mahale göre ara toplamlar", {**FX, "columns": BY_SPACE, "sort": [{"column": 0, "footer": True}]}),
        ("Başlık, ara toplam ve genel toplam", {**FX, "columns": BY_SPACE, "sort": [{"column": 0, "header": True, "footer": True}], "total": True}),
        ("İç içe ara toplamlar", {**FX, "columns": [{"field": "level"}, {"field": "type"}, {"field": "count"}],
                                  "sort": [{"column": 0, "footer": True}, {"column": 1, "footer": True}], "total": True}),
        ("Yalnız zemin kat", {**FX, "filter": {"levels": ["zemin"]}}),
        ("Tür süzgeci", {**FX, "filter": {"types": ["Lavabo"]}}),
        ("Satırsız çizelge", {**FX, "filter": {"types": ["Dolap"]}}),
        ("Satırsız çizelge, toplamlı", {**FX, "filter": {"types": ["Dolap"]}, "total": True}),
        ("Öznitelik sütunu düzenlenir", {**FX, "columns": [{"field": "type"}, {"field": "attr", "attr": "Renk"}]}),
        ("Mahal kaplama çizelgesi", {**SP, "columns": FIN}),
        ("Mahal kaplama çizelgesi, toplamlı", {**SP, "columns": FIN, "total": True}),
        ("Mahal çizelgesi, kata göre başlık ve ara toplam", {**SP, "columns": [{"field": "level"}, {"field": "area"}],
                                                             "sort": [{"column": 0, "header": True, "footer": True}], "total": True}),
        ("Mahal çizelgesi, kullanım süzgeci", {**SP, "columns": FIN, "filter": {"types": ["Oturma"]}}),
        ("Mahal çizelgesi, gruplu kullanım", {**SP, "columns": [{"field": "usage"}, {"field": "count"}, {"field": "area"}], "group": True}),
        ("Başlıklı sütun", {**FX, "columns": [{"field": "type", "title": "Donatı"}, {"field": "count", "title": "Sayı"}]}),
        ("Azalan sıra ve ara toplam", {**FX, "columns": BY_SPACE, "sort": [{"column": 0, "descending": True, "footer": True}]}),
        ("Evre süzgeci", {**FX, "filter": {"phases": ["new"]}}),
        ("Katmana göre süzgeç", {**FX, "filter": {"layers": ["kilitli"]}}),
    ]:
        c.add(f"Oku: {name}", [made(inp), query(inp)])
    c.add("Ret: geçersiz kimlik", [{"op": "execute", "command": QUERY, "input": {"uid": "yok"},
                                    "result": refusal("invalid_uid", "“yok” geçerli bir nesne kimliği değil; küçük harfli, tireli bir UUID verin.", "uid"),
                                    "expect": {"revision": "same"}}])
    c.add("Ret: olmayan nesne", [{"op": "execute", "command": QUERY, "input": {"uid": "0192f5a1-7330-7000-8000-0000000000ff"},
                                  "result": refusal("entity_not_found", "“0192f5a1-7330-7000-8000-0000000000ff” kimlikli nesne çizimde yok. Var olan bir çizelgenin kimliğini verin.", "uid"),
                                  "expect": {"revision": "same"}}])
    c.add("Ret: çizelge olmayan nesne", [{"op": "execute", "command": QUERY, "input": {"uid": uid_of(4)},
                                          "result": refusal("not_schedule", "Nesne bir mimari çizelge değil; Çizelge ile yerleştirilen bir tablonun kimliğini verin.", "uid"),
                                          "expect": {"revision": "same"}}])
    return c.cases


def document(command, title, cases):
    note = ("ADR 0733 §2–§6. Beklenenler scripts/fixtures/schedule_extras_cases.py'nin ADR'den yazılmış kurallarından, KentOS kodu "
            "olmadan; mahaller dikdörtgen, kapısız (süpürgelik boyu çevre). Kurulumda 9 nesne var; yeni nesne 10'dan başlar. "
            "Çizelgenin satır ve sütun boyları yazı tiplerinin ölçüsündendir, burada denetlenmez ($any).")
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": note, "setup": SETUP, "cases": cases}


def files():
    return {
        "cad.schedules.create.extras.json": document(CREATE, "Donatı ve kaplama çizelgesi, tür süzgeci, başlık ve ara toplam; retler", create_cases()),
        "cad.schedules.edit.extras.json": document(EDIT, "Kaplama ve öznitelik hücresi yazılır; hesaplanan, başlık ve ara toplam hücresi, kilitli donatı reddedilir", edit_cases()),
        "cad.schedules.query.json": document(QUERY, "Çizelgeyi oku: sütunların düzenlenebilirliği, satırların rolü ve nesneleri; retler", query_cases()),
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    bad = 0
    for name, doc in files().items():
        path = OUT / name
        text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
        if args.check:
            if not path.exists() or path.read_text() != text:
                print(f"{path.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
                bad += 1
            else:
                print(f"{doc['command']}: {len(doc['cases'])} durum başvuruyla aynı.")
            continue
        path.write_text(text)
        print(f"{path.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
