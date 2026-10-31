#!/usr/bin/env python3
"""The building program's units schedule (docs/adr/0795), from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.schedules.create.units.json, cad.schedules.edit.units.json and
cad.schedules.query.units.json; `--check` compares and writes nothing. Run with shapely (the plan and the shares come
from program_share_cases.py and program_cases.py, which need it):
`uv run --no-project --with shapely python3 scripts/fixtures/unit_schedule_cases.py --check`.

The rules, as the ADR gives them:

- A units schedule (`of: units`) lists the program's groups in the program's order, a row each; a row's members are the
  group's spaces in the drawing's order, its `units` the group's id (a group without spaces is still a data row).
- Its cells: the number, the name, the kind's label, the building's and the wing's names, the levels its spaces stand
  on (the project's order, “, ” between), its rooms' area (no voids, no spaces whose brief is outdoor; 0 with spaces
  but no room, empty without spaces), the recorded area, the share weight as written, the share and the gross of
  `kentos.share/1` (an independent unit's alone), the summed range of its briefed rooms (not voids, not outdoor, a brief
  that gives a range; a side any of them leaves open is open) and how far their area lies outside it, its spaces' count.
  Areas with two decimals, a count whole. Headings: the field's title, an area's with “(m²)”, the count “Mahal sayısı”.
- Its filter takes a group with a space on one of its levels, of one of its buildings, of one of its wings, of one of
  its kinds; another schedule's filter does not take buildings, wings or kinds, a units schedule's not phases, forms,
  layers or types. A building or a wing the program lacks is `building_not_found`.
- Sort, grouping (alike in every column not summed), headings and subtotals, total: as every schedule's; a units
  schedule sums its count, area, share, gross and recorded area. Its number, name, recorded area and share weight are
  edited (written to the program, the program's own rule refusing, `invalid_cell_value`), the rest worked out.
- It follows the program and the spaces in the same step.
"""

import argparse
import copy
import json
import sys
from functools import cmp_to_key
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import program_share_cases as ps  # noqa: E402  (the plan, its program, the shares)
import program_cases as pc  # noqa: E402  (setup, refusals, the cases' list)
import point_editor_cases as pe  # noqa: E402  (the natural order)

P, refusal, Cases, LEVELS = pc.P, pc.refusal, pc.Cases, pc.LEVELS
SP = ps.SP
TABLE = max(SP) + 1
GREY = "#7f8790"
SCREATE, SEDIT, SQUERY = "cad.schedules.create", "cad.schedules.edit", "cad.schedules.query"

KIND_LABEL = {"unit": "Bağımsız bölüm", "common": "Ortak alan", "outdoor": "Dış alan"}
TITLES = {"number": "No", "name": "Ad", "unitKind": "Bölüm türü", "building": "Bina", "wing": "Kanat", "level": "Kat",
          "area": "Alan", "recordedArea": "Kayıtlı alan", "shareWeight": "Pay ağırlığı", "share": "Pay",
          "gross": "Brüt", "target": "Tasarım hedefi", "targetDiff": "Hedef farkı", "count": "Adet", "phase": "Evre",
          "usage": "Kullanım"}
AREAS = {"area", "recordedArea", "share", "gross"}
SUMMED = {"count", "area", "share", "gross", "recordedArea"}
NUMERIC = {"count", "area", "share", "gross", "recordedArea"}
EDITED = {"number", "name", "recordedArea", "shareWeight"}
READS = {"number", "name", "unitKind", "building", "wing", "level", "area", "recordedArea", "shareWeight", "share",
         "gross", "target", "targetDiff", "count"}
DEFAULT = ["number", "name", "unitKind", "building", "level", "area", "share", "gross", "recordedArea"]
BRIEFS = [{"usage": "Balkon", "outdoor": True}, {"usage": "Salon", "areaMin": 20.0, "areaMax": 30.0},
          {"usage": "Yatak odası", "areaMin": 12.0, "areaMax": 16.0}, {"usage": "Koridor", "minPassage": 1.2}]


def program_with(rule="equal", briefs=BRIEFS, change=None, weights=None):
    p = ps.program_with(rule, weights=weights, change=change)
    p["briefs"] = copy.deepcopy(briefs)
    return p


BASE = program_with()


def fixed(v, d=2):
    t = f"{abs(v):.{d}f}"
    return ("-" + t) if v < 0 and float(t) != 0 else t


def rust_float(v):
    """A float as Rust's Display writes it: 62.5, 0, -5."""
    t = repr(float(v))
    return t[:-2] if t.endswith(".0") else t


def text_of(v):
    kind, x = v
    if kind == "text":
        return x
    if kind == "area":
        return fixed(x)
    return str(x)


def number_of(v):
    return None if v is None or v[0] == "text" else float(v[1])


def heading(c):
    if c.get("title"):
        return c["title"]
    f = c["field"]
    if f == "count":
        return "Mahal sayısı"
    return f"{TITLES[f]} (m²)" if f in AREAS else TITLES[f]


def groups_of(program, spaces):
    """Each group's spaces in the drawing's order: uids, levels, briefed rooms' ranges and area."""
    ids = {u["id"] for u in program["units"]}
    briefs = {b["usage"]: b for b in program.get("briefs", [])}
    out = {}
    for i, s in sorted(spaces.items()):
        if s["unit"] not in ids:
            continue
        g = out.setdefault(s["unit"], {"uids": [], "levels": [], "lo": 0.0, "hi": 0.0, "open_lo": False,
                                       "open_hi": False, "briefed": 0.0, "briefs": 0})
        g["uids"].append(f"$uidOf:{i}")
        if s["level"] and s["level"] not in g["levels"]:
            g["levels"].append(s["level"])
        b = briefs.get(s["usage"])
        if b and not s["void"] and not b.get("outdoor") and ("areaMin" in b or "areaMax" in b):
            g["briefs"] += 1
            g["briefed"] += pc.area_of(s)
            if "areaMin" in b:
                g["lo"] += b["areaMin"]
            else:
                g["open_lo"] = True
            if "areaMax" in b:
                g["hi"] += b["areaMax"]
            else:
                g["open_hi"] = True
    return out


def unit_value(program, spaces, u, g, field, parts, rooms):
    names = {b["id"]: b for b in program.get("buildings", [])}
    b = names.get(u.get("building"))
    if field == "number":
        return ("text", u["number"])
    if field == "name":
        return ("text", u["name"]) if u.get("name") else None
    if field == "unitKind":
        return ("text", KIND_LABEL[u.get("kind", "unit")])
    if field == "building":
        return ("text", b["name"]) if b else None
    if field == "wing":
        w = next((w for w in (b or {}).get("wings", []) if w["id"] == u.get("wing")), None)
        return ("text", w["name"]) if w else None
    if field == "level":
        if not g:
            return None
        known = [lv["name"] for lv in LEVELS if lv["id"] in g["levels"]]
        known += [lv for lv in g["levels"] if lv not in {x["id"] for x in LEVELS}]
        return ("text", ", ".join(known)) if known else None
    if field == "area":
        return ("area", rooms.get(u["id"], 0.0)) if g else None
    if field == "recordedArea":
        return ("area", u["recordedArea"]) if "recordedArea" in u else None
    if field == "shareWeight":
        return ("text", u["shareWeight"]) if "shareWeight" in u else None
    if field in ("share", "gross"):
        if u.get("kind", "unit") != "unit" or u["id"] not in parts:
            return None
        return ("area", float(parts[u["id"]][field]))
    lo = g["lo"] if g and not g["open_lo"] else None
    hi = g["hi"] if g and not g["open_hi"] else None
    if field == "target":
        if not g or not g["briefs"]:
            return None
        if lo is not None and hi is not None:
            return ("text", f"{fixed(lo)}–{fixed(hi)} m²")
        if lo is not None:
            return ("text", f"≥ {fixed(lo)} m²")
        if hi is not None:
            return ("text", f"≤ {fixed(hi)} m²")
        return None
    if field == "targetDiff":
        if not g or not g["briefs"] or (lo is None and hi is None):
            return None
        a = g["briefed"]
        if lo is not None and a < lo:
            return ("area", a - lo)
        if hi is not None and a > hi:
            return ("area", a - hi)
        return ("area", 0.0)
    if field == "count":
        return ("count", len(g["uids"]) if g else 0)
    raise ValueError(field)


def takes(u, g, flt):
    if not flt:
        return True
    if "levels" in flt and not (g and any(lv in flt["levels"] for lv in g["levels"])):
        return False
    if "buildings" in flt and u.get("building") not in flt["buildings"]:
        return False
    if "wings" in flt and u.get("wing") not in flt["wings"]:
        return False
    if "kinds" in flt and u.get("kind", "unit") not in flt["kinds"]:
        return False
    return True


def add(a, b):
    if a is None or b is None:
        return a or b
    return (a[0], a[1] + b[1])


def unit_rows(program, spaces, src):
    """The schedule's cells, members, units, aligns and styles."""
    columns = src["columns"]
    got = ps.shares(program, spaces)
    parts = got[1] if got else {}
    rooms = ps.rooms_of(spaces, program)
    groups = groups_of(program, spaces)
    rows = []
    for u in program["units"]:
        g = groups.get(u["id"])
        if not takes(u, g, src.get("filter")):
            continue
        vals = [unit_value(program, spaces, u, g, c["field"], parts, rooms) for c in columns]
        rows.append({"vals": vals, "texts": [text_of(v) if v else "" for v in vals],
                     "uids": list(g["uids"]) if g else [], "units": [u["id"]]})
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
                x, y = number_of(a["vals"][j]), number_of(b["vals"][j])
                o = (x > y) - (x < y) if x is not None and y is not None else pe.natural_cmp(a["texts"][j], b["texts"][j])
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
            m["units"] += r["units"]
        for m in merged:
            for j, s in enumerate(summed):
                if s:
                    m["texts"][j] = text_of(m["vals"][j]) if m["vals"][j] else ""
        rows = merged
    width = len(columns)
    cells, members, units, styles = [[heading(c) for c in columns]], [], [], []

    def bold(at):
        return {"row": at, "col": 0, "rows": 1, "cols": width, "bold": True}
    marked = [k for k in sort if k.get("header") or k.get("footer")]

    def footer(d, sums, value):
        out = [text_of(v) if (s and v) else "" for v, s in zip(sums, summed)]
        for j, c in enumerate(columns):
            if c["field"] == "count" and not out[j]:
                out[j] = "0"
        k = marked[d]["column"]
        label = next((j for j in range(width) if not summed[j]), None) if summed[k] else k
        if label is not None:
            out[label] = f"{value or '(boş)'} toplamı"
        return out
    sums = [None] * width
    gsums = [[None] * width for _ in marked]
    gvals = [""] * len(marked)
    prev = None
    runs = []
    for r in rows:
        keys = [r["texts"][k["column"]] for k in marked]
        frm = 0 if prev is None else next((i for i, (a, b) in enumerate(zip(prev, keys)) if a != b), len(marked))
        if prev is not None:
            for d in reversed(range(frm, len(marked))):
                if marked[d].get("footer"):
                    styles.append(bold(len(cells)))
                    cells.append(footer(d, gsums[d], gvals[d]))
                    members.append([])
                    units.append([])
        for d in range(frm, len(marked)):
            gsums[d] = [None] * width
            gvals[d] = keys[d]
            if marked[d].get("header"):
                at = len(cells)
                styles.append(bold(at))
                cells.append([keys[d] or "(boş)"] + [""] * (width - 1))
                members.append([])
                units.append([])
        for j in range(width):
            if summed[j]:
                sums[j] = add(sums[j], r["vals"][j])
                for gs in gsums:
                    gs[j] = add(gs[j], r["vals"][j])
        at = len(cells)
        if runs and runs[-1][0] + runs[-1][1] == at:
            runs[-1][1] += 1
        else:
            runs.append([at, 1])
        cells.append(r["texts"])
        members.append(r["uids"])
        units.append(r["units"])
        prev = keys
    if prev is not None:
        for d in reversed(range(len(marked))):
            if marked[d].get("footer"):
                styles.append(bold(len(cells)))
                cells.append(footer(d, gsums[d], gvals[d]))
                members.append([])
                units.append([])
    for first, n in runs:
        for j, c in enumerate(columns):
            if c["field"] not in EDITED:
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
    merges = [{"row": i, "col": 0, "rows": 1, "cols": width} for i, row in enumerate(cells)
              if i and width > 1 and members[i - 1:i] == [[]] and units[i - 1:i] == [[]]
              and row[1:] == [""] * (width - 1) and any(k.get("header") for k in marked)
              and not row[0].endswith(" toplamı")]
    return cells, members, units, aligns, styles, merges


def source_of(inp):
    src = {"kind": "schedule", "of": inp["of"],
           "columns": inp["columns"] if "columns" in inp else [{"field": f} for f in DEFAULT]}
    for k in ("filter", "sort"):
        if inp.get(k):
            src[k] = inp[k]
    for k in ("group", "total"):
        if inp.get(k):
            src[k] = True
    return src


def table_fields(program, spaces, src):
    cells, members, units, aligns, styles, _ = unit_rows(program, spaces, src)
    out = copy.deepcopy(src)
    if members:
        out["members"] = members
    if units:
        out["units"] = units
    f = {"cells": cells, "source": out, "aligns": aligns, "header": True}
    f["styles"] = styles if styles else None
    return f


def data_rows(program, spaces, src):
    _, _, units, _, _, _ = unit_rows(program, spaces, src)
    return sum(1 for u in units if u)


# ── The steps ─────────────────────────────────────────────────────────────────────────────────────────────────────


def create_step(program, inp, spaces=None, op="execute"):
    spaces = spaces or SP
    src = source_of(inp)
    step = {"op": op, "command": SCREATE, "input": inp}
    if op == "validate":
        step["result"] = {"status": "completed", "output": None, "warnings": []}
        step["expect"] = {"revision": "same"}
        return step
    if op == "plan":
        step["result"] = {"status": "completed", "output": "$any", "warnings": []}
        step["expect"] = {"revision": "same"}
        return step
    step["result"] = {"status": "completed",
                      "output": {"uid": "$uid", "id": TABLE, "rows": data_rows(program, spaces, src),
                                 "revision": "$current"},
                      "warnings": []}
    step["expect"] = {"revision": "changed", "fields": {str(TABLE): table_fields(program, spaces, src)}}
    return step


def refused(command, inp, code, message, path, op="execute"):
    return {"op": op, "command": command, "input": inp, "result": refusal(code, message, path),
            "expect": {"revision": "same"}}


def follows(program, spaces, src, step):
    """Another command's step after which the schedule reads `program` and `spaces`."""
    step = copy.deepcopy(step)
    step.setdefault("expect", {})["revision"] = "changed"
    step["expect"].setdefault("fields", {})[str(TABLE)] = {"cells": unit_rows(program, spaces, src)[0]}
    return step


def edit_step(program_after, inp, spaces=None, src=None, op="execute", units=()):
    spaces = spaces or SP
    step = {"op": op, "command": SEDIT, "input": inp}
    if op == "plan":
        step["result"] = {"status": "completed", "output": "$any", "warnings": []}
        step["expect"] = {"revision": "same"}
        return step
    step["result"] = {"status": "completed", "output": "$any", "warnings": []}
    step["expect"] = {"revision": "changed",
                      "settings": {"arch": {"levels": LEVELS, "program": program_after}},
                      "fields": {str(TABLE): {"cells": unit_rows(program_after, spaces, src)[0]}}}
    return step


def query_step(program, src, spaces=None):
    spaces = spaces or SP
    cells, members, units, _, _, merges = unit_rows(program, spaces, src)
    heads = {m["row"] for m in merges}
    rows = []
    n = len(cells)
    for i, c in enumerate(cells):
        if i == 0:
            role = "heading"
        elif src.get("total") and i + 1 == n:
            role = "total"
        elif units[i - 1]:
            role = "data"
        elif i in heads:
            role = "header"
        else:
            role = "footer"
        row = {"row": i, "role": role, "cells": c, "members": members[i - 1] if role == "data" else []}
        if role == "data":
            row["units"] = units[i - 1]
        rows.append(row)
    columns = [{"heading": heading(c), "field": c["field"], "editable": c["field"] in EDITED,
                "summed": c["field"] in SUMMED} for c in src["columns"]]
    return {"op": "execute", "command": SQUERY, "input": {"uid": f"$uidOf:{TABLE}"},
            "result": {"status": "completed", "output": {"of": "units", "columns": columns, "rows": rows,
                                                        "revision": "$current"}, "warnings": []},
            "expect": {"revision": "same"}}


def inp_of(of="units", **kw):
    i = {"layerId": "etiket", "at": P(0, -20), "of": of}
    i.update(kw)
    return i


def cols(*fields):
    return [{"field": f} for f in fields]


def setup_with(program):
    return ps.setup_with(program)


# ── cad.schedules.create ──────────────────────────────────────────────────────────────────────────────────────────


def create_cases():
    c = Cases()

    def case(name, steps, program=None):
        c.add(name, steps, None if program is None else setup_with(program))
    case("Bölüm çizelgesi programın sırasıyla bölüm başına bir satır yazar", [create_step(BASE, inp_of())])
    case("Bütün alanlar ve toplam", [create_step(BASE, inp_of(columns=cols(
        "number", "name", "unitKind", "building", "wing", "level", "area", "recordedArea", "share", "gross", "count"),
        total=True))])
    case("Katlar projenin sırasıyla; dubleksin iki katı", [create_step(BASE, inp_of(columns=cols("number", "level")))])
    case("Net alan odaları bir kez sayar: şaft ve balkon dışı", [create_step(BASE, inp_of(
        columns=cols("number", "area", "count"), total=True))])
    case("Tasarım hedefi odaların aralıklarının toplamı, fark aralığın dışı", [create_step(BASE, inp_of(
        columns=cols("number", "target", "targetDiff")))])
    opened = program_with(briefs=[b if b["usage"] != "Yatak odası" else {"usage": "Yatak odası", "areaMin": 12.0}
                                  for b in BRIEFS])
    case("Bir odanın üst sınırı yoksa hedef “en az” olur", [create_step(opened, inp_of(
        columns=cols("number", "target", "targetDiff")))], opened)
    weighted = program_with("weight", weights={"d1": "1", "d2": "2", "d3": "1.5", "d4": "3"})
    case("Pay ağırlığı yazıldığı gibi, pay ağırlığa göre", [create_step(weighted, inp_of(
        columns=cols("number", "shareWeight", "share", "gross"), total=True))], weighted)
    noshare = program_with(None)
    case("Pay kuralı yoksa pay ve brüt boş", [create_step(noshare, inp_of(
        columns=cols("number", "area", "share", "gross")))], noshare)
    lonely = program_with(change=lambda p: p["units"].append({"id": "d5", "number": "5", "name": "Daire 5"}))
    case("Mahali olmayan bölüm de bir satırdır: alanı boş, mahal sayısı 0", [create_step(lonely, inp_of(
        columns=cols("number", "level", "area", "share", "gross", "count"), total=True))], lonely)
    case("Kat süzgeci o katta mahali olan bölümleri alır", [create_step(BASE, inp_of(
        columns=cols("number", "level"), filter={"levels": ["kat-1"]}))])
    case("Bina süzgeci", [create_step(BASE, inp_of(columns=cols("number", "building"), filter={"buildings": ["b"]}))])
    case("Kanat süzgeci", [create_step(BASE, inp_of(columns=cols("number", "wing"), filter={"wings": ["a-dogu"]}))])
    case("Tür süzgeci: ortak alanlar", [create_step(BASE, inp_of(
        columns=cols("number", "unitKind", "area"), filter={"kinds": ["common"]}, total=True))])
    case("Süzgeçler birlikte", [create_step(BASE, inp_of(
        columns=cols("number"), filter={"buildings": ["a"], "kinds": ["unit"], "levels": ["zemin"]}))])
    case("Alana göre azalan sıra", [create_step(BASE, inp_of(
        columns=cols("number", "area"), sort=[{"column": 1, "descending": True}]))])
    case("Aynı tür ve binadakiler bir satır, alanları toplanır", [create_step(BASE, inp_of(
        columns=cols("unitKind", "building", "count", "area", "share", "gross"), group=True, total=True))])
    case("Binaya göre başlık ve ara toplam", [create_step(BASE, inp_of(
        columns=cols("building", "number", "area", "gross"), sort=[{"column": 0, "header": True, "footer": True}],
        total=True))])
    case("Plan yazmaz", [create_step(BASE, inp_of(), op="plan")])
    case("Doğrulama yazmaz", [create_step(BASE, inp_of(columns=cols("number")), op="validate")])
    case("Ret: bölüm çizelgesi evre okumaz", [refused(SCREATE, inp_of(columns=cols("number", "phase")),
         "invalid_schedule", "“Evre” bölüm çizelgesinde okunmaz.", "columns[1].field")])
    case("Ret: mahal çizelgesi bölüm türü okumaz", [refused(SCREATE, inp_of("spaces", columns=cols("unitKind")),
         "invalid_schedule", "“Bölüm türü” mahal çizelgesinde okunmaz.", "columns[0].field")])
    case("Ret: bölüm çizelgesinde evre süzgeci", [refused(SCREATE, inp_of(filter={"phases": ["new"]}),
         "invalid_schedule", "Bölüm çizelgesi bölümleri kat, bina, kanat ve türle süzer; evre, biçim, katman ve tür "
         "adı süzgeci nesne çizelgelerindedir.", "filter.phases")])
    case("Ret: mahal çizelgesinde bina süzgeci", [refused(SCREATE, inp_of("spaces", filter={"buildings": ["a"]}),
         "invalid_schedule", "Bina, kanat ve bölüm türü süzgeci yalnız bölüm çizelgesindedir.", "filter.buildings")])
    case("Ret: programda olmayan bina", [refused(SCREATE, inp_of(filter={"buildings": ["a", "z"]}),
         "building_not_found", "Bina programında “z” kimlikli bina yok. Var olan bir binanın kimliğini verin.",
         "filter.buildings[1]")])
    case("Ret: programda olmayan kanat", [refused(SCREATE, inp_of(filter={"wings": ["b"]}),
         "building_not_found", "Bina programında “b” kimlikli kanat yok. Var olan bir kanadın kimliğini verin.",
         "filter.wings[0]")])
    return c.cases


# ── cad.schedules.edit ────────────────────────────────────────────────────────────────────────────────────────────


def edit_cases():
    c = Cases()
    src = source_of(inp_of())
    made = create_step(BASE, inp_of())

    def cell(row, column, value):
        return {"uid": f"$uidOf:{TABLE}", "operation": {"kind": "cell", "row": row, "column": column, "value": value}}

    def changed(f, program=BASE):
        p = copy.deepcopy(program)
        f(p)
        return p

    def unit(p, uid):
        return next(u for u in p["units"] if u["id"] == uid)

    def case(name, steps, program=None):
        c.add(name, steps, None if program is None else setup_with(program))
    num = changed(lambda p: unit(p, "d1").update(number="1A"))
    case("Numara programa yazılır, çizelge aynı adımda izler", [made, edit_step(num, cell(1, 0, "1A"), src=src)])
    name = changed(lambda p: unit(p, "d2").update(name="Dubleks"))
    case("Ad programa yazılır", [made, edit_step(name, cell(2, 1, "Dubleks"), src=src)])
    noname = changed(lambda p: unit(p, "d3").pop("name"))
    case("Boş ad bölümün adını kaldırır", [made, edit_step(noname, cell(3, 1, ""), src=src)])
    rec = changed(lambda p: unit(p, "d2").update(recordedArea=62.5))
    case("Kayıtlı alan virgülle yazılır", [made, edit_step(rec, cell(2, 8, "62,5"), src=src)])
    unrec = changed(lambda p: unit(p, "d1").pop("recordedArea"))
    case("Boş kayıtlı alan kaldırılır", [made, edit_step(unrec, cell(1, 8, ""), src=src)])
    case("Geri alma programı ve çizelgeyi döndürür", [made, edit_step(rec, cell(2, 8, "62,5"), src=src),
         {"op": "undo", "returns": "Çizelgeyi düzenle",
          "expect": {"settings": {"arch": {"levels": LEVELS, "program": BASE}},
                     "fields": {str(TABLE): {"cells": unit_rows(BASE, SP, src)[0]}}}}])
    case("Plan yazmaz", [made, edit_step(rec, cell(2, 8, "62,5"), src=src, op="plan")])
    wsrc = source_of(inp_of(columns=cols("number", "shareWeight", "share", "gross")))
    weighted = program_with("weight", weights={"d1": "1", "d2": "2", "d3": "1", "d4": "1"})
    w2 = changed(lambda p: unit(p, "d1").update(shareWeight="1.5"), weighted)
    case("Pay ağırlığı yazılınca pay ve brüt aynı adımda değişir", [
        create_step(weighted, inp_of(columns=wsrc["columns"])), edit_step(w2, cell(1, 1, "1,5"), src=wsrc)], weighted)
    case("Ret: okunamayan pay ağırlığı", [create_step(weighted, inp_of(columns=wsrc["columns"])), refused(
        SEDIT, cell(1, 1, "1.1234567"), "invalid_cell_value",
        "Pay ağırlığı “1.1234567” okunamadı: noktadan önce en çok 12, sonra en çok 6 basamaklı bir sayı yazın "
        "(120, 1.5).", "operation.value")], weighted)
    case("Ret: iki bölümde aynı numara", [made, refused(SEDIT, cell(1, 0, "2"), "invalid_cell_value",
         "“2” numarası iki bölümde var; her bölümün numarası ayrı olmalı.", "operation.value")])
    case("Ret: boş numara", [made, refused(SEDIT, cell(1, 0, " "), "invalid_cell_value",
         "Bölümün numarası boş olamaz.", "operation.value")])
    case("Ret: alan olmayan kayıtlı alan", [made, refused(SEDIT, cell(1, 8, "elli"), "invalid_cell_value",
         "“elli” bir alan değil; projenin alan biriminde bir sayı yazın.", "operation.value")])
    case("Ret: sıfır kayıtlı alan", [made, refused(SEDIT, cell(1, 8, "0"), "invalid_cell_value",
         f"Kayıtlı alan {rust_float(0.0)} m²; sıfırdan büyük, sonlu olmalı.", "operation.value")])
    case("Ret: hesaplanan net alan", [made, refused(SEDIT, cell(1, 5, "40"), "cell_computed",
         "“Alan (m²)” sütunu çizimden hesaplanır; düzenlenmez. Kaynağın kendi alanını değiştirin.",
         "operation.column")])
    case("Ret: hesaplanan bölüm türü", [made, refused(SEDIT, cell(1, 2, "Ortak alan"), "cell_computed",
         "“Bölüm türü” sütunu çizimden hesaplanır; düzenlenmez. Kaynağın kendi alanını değiştirin.",
         "operation.column")])
    unnamed = changed(lambda p: [unit(p, u).pop("name") for u in ("d2", "d3")])
    gsrc = source_of(inp_of(columns=cols("building", "wing", "name", "count"), group=True))
    grouped = create_step(unnamed, inp_of(columns=gsrc["columns"], group=True))
    both = changed(lambda p: [unit(p, u).update(name="Doğu dairesi") for u in ("d2", "d3")], unnamed)
    case("Gruplu satırın adı iki bölüme birden yazılır", [grouped, edit_step(both, cell(2, 2, "Doğu dairesi"),
         src=gsrc)], unnamed)
    case("Ret: gruplu satırda hesaplanan sütun", [grouped, refused(SEDIT, cell(2, 0, "X"), "cell_computed",
         "“Bina” sütunu çizimden hesaplanır; düzenlenmez. Kaynağın kendi alanını değiştirin.", "operation.column")],
         unnamed)
    nsrc = source_of(inp_of(columns=cols("number", "building"), group=True))
    ngroup = create_step(BASE, inp_of(columns=nsrc["columns"], group=True))
    case("Gruplanmayan numaralı satırlar tek bölümdür", [ngroup, edit_step(
        changed(lambda p: unit(p, "d4").update(number="4B")), cell(4, 0, "4B"), src=nsrc)])
    hsrc = source_of(inp_of(columns=cols("building", "number", "area"), sort=[{"column": 0, "header": True}]))
    headed = create_step(BASE, inp_of(columns=hsrc["columns"], sort=hsrc["sort"]))
    case("Ret: başlık satırı düzenlenmez", [headed, refused(SEDIT, cell(1, 1, "9"), "schedule_summary_row",
         "Çizelgenin 1. satırı bir grup başlığı ya da ara toplam; düzenlenmez.", "operation.row")])
    lonely = program_with(change=lambda p: p["units"].append({"id": "d5", "number": "5", "name": "Daire 5"}))
    lsrc = source_of(inp_of(columns=cols("number", "name", "count")))
    case("Mahali olmayan bölümün adı da yazılır", [
        create_step(lonely, inp_of(columns=lsrc["columns"])),
        edit_step(changed(lambda p: unit(p, "d5").update(name="Dükkân"), lonely), cell(10, 1, "Dükkân"), src=lsrc)],
        lonely)
    # Following the program and the spaces.
    moved = copy.deepcopy(SP)
    moved[15]["unit"] = "d2"
    case("Mahal başka bölüme verilince çizelge izler", [made, follows(BASE, moved, src, {
        "op": "execute", "command": "cad.program.edit", "input": {"assign": [{"uids": ["$uidOf:15"], "unit": "d2"}]},
        "result": {"status": "completed", "output": "$any", "warnings": "$any"}})])
    renamed = changed(lambda p: p["buildings"][1].update(name="B Blok (yeni)"))
    case("Bina adı değişince çizelge izler", [made, follows(renamed, SP, src, {
        "op": "execute", "command": "cad.program.edit", "input": {"program": renamed},
        "result": {"status": "completed", "output": "$any", "warnings": []}})])
    gone = copy.deepcopy(SP)
    del gone[11]
    case("Mahal silinince bölümün alanı ve payı izler", [made, follows(BASE, gone, src, {
        "op": "execute", "command": "cad.entities.delete", "input": {"uids": ["$uidOf:11"]},
        "result": {"status": "completed", "output": "$any", "warnings": "$any"}})])
    case("Ret: ayarda programda olmayan kanat", [made, refused(
        SEDIT, {"uid": f"$uidOf:{TABLE}", "operation": {"kind": "set", "filter": {"wings": ["yok"]}}},
        "building_not_found", "Bina programında “yok” kimlikli kanat yok. Var olan bir kanadın kimliğini verin.",
        "operation.filter.wings[0]")])
    fsrc = dict(src, filter={"kinds": ["common", "outdoor"]})
    case("Ayar süzgeci değiştirir", [made, {
        "op": "execute", "command": SEDIT,
        "input": {"uid": f"$uidOf:{TABLE}", "operation": {"kind": "set", "filter": {"kinds": ["common", "outdoor"]}}},
        "result": {"status": "completed", "output": "$any", "warnings": []},
        "expect": {"revision": "changed", "fields": {str(TABLE): {"cells": unit_rows(BASE, SP, fsrc)[0]}}}}])
    return c.cases


# ── cad.schedules.query ───────────────────────────────────────────────────────────────────────────────────────────


def query_cases():
    c = Cases()

    def case(name, inp, program=BASE, spaces=None):
        src = source_of(inp)
        c.add(name, [create_step(program, inp, spaces), query_step(program, src, spaces)],
              None if program is BASE else setup_with(program))
    case("Oku: varsayılan sütunlar, satırların bölümleri", inp_of())
    case("Oku: toplam satırı", inp_of(columns=cols("number", "area", "count"), total=True))
    case("Oku: mahal sayısı başlığı ve toplanan sütunlar", inp_of(columns=cols("number", "count", "share", "gross",
                                                                              "recordedArea")))
    case("Oku: düzenlenen ve hesaplanan sütunlar", inp_of(columns=cols(
        "number", "name", "recordedArea", "shareWeight", "unitKind", "level", "target")))
    case("Oku: kat süzgeci", inp_of(columns=cols("number", "level"), filter={"levels": ["kat-1"]}))
    case("Oku: bina süzgeci", inp_of(columns=cols("number"), filter={"buildings": ["a"]}))
    case("Oku: kanat süzgeci", inp_of(columns=cols("number", "wing"), filter={"wings": ["a-bati"]}))
    case("Oku: tür süzgeci", inp_of(columns=cols("number", "unitKind"), filter={"kinds": ["outdoor"]}))
    case("Oku: gruplu satır iki bölümü tutar", inp_of(columns=cols("unitKind", "building", "count", "area"),
                                                      group=True))
    case("Oku: başlık ve ara toplam satırları", inp_of(columns=cols("building", "number", "area"),
                                                       sort=[{"column": 0, "header": True, "footer": True}], total=True))
    case("Oku: ara toplam", inp_of(columns=cols("unitKind", "number", "gross"),
                                   sort=[{"column": 0, "footer": True}]))
    case("Oku: azalan sıra", inp_of(columns=cols("number", "gross"), sort=[{"column": 1, "descending": True}]))
    case("Oku: ada göre sıra", inp_of(columns=cols("name", "number"), sort=[{"column": 0}]))
    lonely = program_with(change=lambda p: p["units"].append({"id": "d5", "number": "5"}))
    case("Oku: mahali olmayan bölüm veri satırıdır", inp_of(columns=cols("number", "count", "area")), lonely)
    weighted = program_with("weight", weights={"d1": "2", "d2": "1", "d3": "1", "d4": "1"})
    case("Oku: ağırlıkla pay", inp_of(columns=cols("number", "shareWeight", "share", "gross"), total=True), weighted)
    net = program_with("net")
    case("Oku: net alana göre pay", inp_of(columns=cols("number", "area", "share", "gross"), total=True), net)
    case("Oku: hedef ve fark", inp_of(columns=cols("number", "target", "targetDiff")))
    case("Oku: süzgeç sonuçsuz", inp_of(columns=cols("number"), filter={"kinds": ["unit"], "buildings": ["c"]},
                                        total=True))
    case("Oku: bütün katlar ve kayıtlı alan", inp_of(columns=cols("number", "level", "recordedArea"), total=True))
    case("Oku: dış alan ve ortak alan payı yok", inp_of(columns=cols("number", "unitKind", "share", "gross"),
                                                        filter={"kinds": ["common", "outdoor"]}))
    return c.cases


def write(name, command, title, note, setup_doc, cases, check_only):
    data = {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": note, "setup": setup_doc, "cases": cases}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    path = pc.OUT / name
    if check_only:
        if not path.exists() or path.read_text() != text:
            print(f"{path.relative_to(pc.ROOT)} güncel değil: --check'siz çalıştırın.")
            return False
        return True
    path.write_text(text)
    print(f"{path.relative_to(pc.ROOT)}: {len(cases)} durum")
    return True


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    note = ("ADR 0795. Beklenenler scripts/fixtures/unit_schedule_cases.py'nin ADR'den yazılmış kurallarından, KentOS "
            "kodu olmadan; paylar program_share_cases.py'nin kesirlerinden, plan program_cases.py'den.")
    setup_doc = setup_with(BASE)
    ok = write("cad.schedules.create.units.json", SCREATE, "Bölüm çizelgesi: satırlar, alanlar, pay, hedef, süzgeç, "
               "sıra, grup; retler", note, setup_doc, create_cases(), args.check)
    ok &= write("cad.schedules.edit.units.json", SEDIT, "Bölüm çizelgesinin hücresi programa yazılır; çizelge programı ve "
                "mahalleri izler; retler", note, setup_doc, edit_cases(), args.check)
    ok &= write("cad.schedules.query.units.json", SQUERY, "Bölüm çizelgesini oku: sütunlar, satırların rolleri ve "
                "bölümleri", note, setup_doc, query_cases(), args.check)
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
