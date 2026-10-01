#!/usr/bin/env python3
"""Independent reference of the point editor's edits (docs/adr/0153 §3, §4).

Writes fixtures/point-editor/v1/edits.json from the rules alone, with
Python's standard library and no KentOS code. Both platforms run every case
on their own document through their own table code (web
`ui/bottom/pointEdit.ts`, desktop `points/edit.rs`) and must leave the same
drawing, say the same messages and name the same undo step.

A case's drawing is `objects` (v1 objects) on the layers “Çizim” (cizim) and
“Kilitli” (kilitli, locked); the active layer is “cizim” unless `active`
says otherwise. Its action is one of:

- `cell`: a point's cell (`id`, `column` name, code, east, north or z) given
  `text`, with Bağlı çizgiler izler on or off (`follow`);
- `draft`: Satır ekle's row written (`name`, `east`, `north`, `z`, `code`).

The rules:

1. Ad and Kod are trimmed (JavaScript's white space); empty removes the
   label or the `Kod` attribute. Y, X and Z are read as the point input reads
   a number: trimmed, the first comma a point, then a whole decimal number;
   an empty Z removes the elevation. Anything else is not a number.
2. A value not a number is said (“Nokta editörü: Y bir sayı olmalı.”) and
   nothing is written; a value the point already has writes nothing.
3. A cell of a point on a locked layer is refused with the command's words.
4. With `follow`, the line work with a vertex within 1e-6 m of the point's
   place before the edit follows it: those vertices move with the point, or
   take its new elevation (none when it loses its own). When any such object
   is on a locked layer (and the point is not), nothing is written and that
   is said: the point's own lock is said first, as the command checks its
   changes in order, the point's first.
5. A written cell is one undo step, “Nokta düzenle”.
6. A name another point has too (trimmed) is written and said.
7. A draft needs Y and X (“Nokta editörü: Y ve X yazılmalı.”, “… Y
   yazılmalı.”, “… X yazılmalı.”); its values are read as in 1; it is
   written on the active layer as the point command writes (“Ekle”; a locked
   layer refuses with the command's words); the next draft's name is the
   written name's Artır (the digits it ends with plus one, as many digits at
   least), empty when it does not end with a digit.

What is compared: the messages said, in order; the undo step's name (null:
nothing written); every object after, in the drawing's order, as its kind,
label, attributes, and for a point its place and elevation, for line work
its paths (vertices and elevations, the outer ring then the holes); for a
draft, the next draft's name.
"""

import argparse
import copy
import json
import math
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "point-editor" / "v1" / "edits.json"
ON = 1e-6
JS_SPACE = " \t\n\v\f\r                 　﻿"
LAYERS = {"cizim": "Çizim", "kilitli": "Kilitli"}
LOCKED = {"kilitli"}
PREFIX = "Nokta editörü: "
WORDS = {"east": "Y", "north": "X", "z": "Z"}
STEP_CELL = "Nokta düzenle"
STEP_DRAFT = "Ekle"


def trim(t):
    return t.strip(JS_SPACE)


def number(text):
    t = trim(text).replace(",", ".", 1)
    if re.fullmatch(r"[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?", t):
        return float(t)
    return None


def increment(name):
    m = re.search(r"(\d+)$", name)
    if not m:
        return None
    digits = m.group(1)
    return name[: m.start()] + str(int(digits) + 1).rjust(len(digits), "0")


def P(x, y):
    return {"x": float(x), "y": float(y)}


def near(a, b):
    return math.hypot(a["x"] - b["x"], a["y"] - b["y"]) <= ON


def paths(o):
    """Line work's paths: [(points, elevations)], the outer ring then the holes."""
    if o["kind"] == "line":
        return [([o["a"], o["b"]], [o.get("za"), o.get("zb")])]
    out = [(o["pts"], list(o.get("zs") or [None] * len(o["pts"])))]
    for h in o.get("holes") or []:
        out.append((h["pts"], list(h.get("zs") or [None] * len(h["pts"]))))
    return out


def set_paths(o, ps):
    if o["kind"] == "line":
        (a, b), (za, zb) = ps[0]
        o["a"], o["b"] = a, b
        for k, z in (("za", za), ("zb", zb)):
            if z is None:
                o.pop(k, None)
            else:
                o[k] = z
        return
    def put(target, pts, zs):
        target["pts"] = pts
        if any(z is not None for z in zs):
            target["zs"] = zs
        else:
            target.pop("zs", None)
    put(o, *ps[0])
    for h, (pts, zs) in zip(o.get("holes") or [], ps[1:]):
        put(h, pts, zs)


def follow(o, frm, to, set_z, z):
    """The object with its vertices at `frm` moved to `to` (and given `z`), or None when none is there."""
    ps = paths(o)
    hit = False
    out = []
    for pts, zs in ps:
        pts, zs = [dict(p) for p in pts], list(zs)
        for k, p in enumerate(pts):
            if near(p, frm):
                hit = True
                pts[k] = dict(to)
                if set_z:
                    zs[k] = z
        out.append((pts, zs))
    if not hit or out == [([dict(p) for p in pts], list(zs)) for pts, zs in ps]:
        return None
    o = copy.deepcopy(o)
    set_paths(o, out)
    return o


def view(o):
    v = {"kind": o["kind"], "label": o.get("label"), "attrs": o.get("attrs", {})}
    if o["kind"] == "point":
        v["at"] = [o["p"]["x"], o["p"]["y"]]
        v["z"] = o.get("z")
    else:
        v["paths"] = [{"pts": [[p["x"], p["y"]] for p in pts], "zs": zs} for pts, zs in paths(o)]
    return v


def refusal(layer):
    return f"“{LAYERS[layer]}” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın."


def same_name(objects, name, me=None):
    return any(o["kind"] == "point" and o is not me and trim(o.get("label") or "") == name for o in objects)


def cell(objects, action):
    objs = copy.deepcopy(objects)
    point = next(o for o in objs if o["id"] == action["id"])
    col, text, fol = action["column"], action["text"], action["follow"]
    said = []
    if col in ("name", "code"):
        v = trim(text) or None
        old = trim(point.get("label") or "") or None if col == "name" else point.get("attrs", {}).get("Kod")
        if v == old:
            return objs, said, None
        if point["layerId"] in LOCKED:
            return objs, [refusal(point["layerId"])], None
        if col == "name":
            if v is None:
                point.pop("label", None)
            else:
                point["label"] = v
            if v is not None and same_name(objs, v, point):
                said.append(f"{PREFIX}“{v}” adında başka bir nokta da var.")
        else:
            attrs = dict(point.get("attrs", {}))
            if v is None:
                attrs.pop("Kod", None)
            else:
                attrs["Kod"] = v
            point["attrs"] = attrs
        return objs, said, STEP_CELL
    # A number.
    if col == "z" and trim(text) == "":
        v = None
    else:
        v = number(text)
        if v is None:
            return objs, [f"{PREFIX}{WORDS[col]} bir sayı olmalı."], None
    old_p = dict(point["p"])
    if col == "east":
        new_p, set_z, z = P(v, old_p["y"]), False, None
        changed = v != old_p["x"]
    elif col == "north":
        new_p, set_z, z = P(old_p["x"], v), False, None
        changed = v != old_p["y"]
    else:
        new_p, set_z, z = old_p, True, v
        changed = v != point.get("z")
    if not changed:
        return objs, said, None
    if point["layerId"] in LOCKED:
        return objs, [refusal(point["layerId"])], None
    moved = []
    if fol:
        for o in objs:
            if o["kind"] in ("line", "polyline", "polygon"):
                f = follow(o, old_p, new_p, set_z, z)
                if f is not None:
                    if o["layerId"] in LOCKED:
                        return objects_copy(objects), [f"{PREFIX}Bağlı çizgilerden biri kilitli katmanda; katmanın kilidini açın ya da Bağlı çizgiler izler'i kapatın."], None
                    moved.append((o, f))
    if col == "z":
        if v is None:
            point.pop("z", None)
        else:
            point["z"] = v
    else:
        point["p"] = new_p
    for o, f in moved:
        o.clear()
        o.update(f)
    return objs, said, STEP_CELL


def objects_copy(objects):
    return copy.deepcopy(objects)


def draft(objects, action, active):
    objs = copy.deepcopy(objects)
    east, north = trim(action["east"]), trim(action["north"])
    if not east and not north:
        return objs, [f"{PREFIX}Y ve X yazılmalı."], None, None
    if not east:
        return objs, [f"{PREFIX}Y yazılmalı."], None, None
    if not north:
        return objs, [f"{PREFIX}X yazılmalı."], None, None
    x, y = number(east), number(north)
    if x is None:
        return objs, [f"{PREFIX}Y bir sayı olmalı."], None, None
    if y is None:
        return objs, [f"{PREFIX}X bir sayı olmalı."], None, None
    z = None
    if trim(action["z"]):
        z = number(action["z"])
        if z is None:
            return objs, [f"{PREFIX}Z bir sayı olmalı."], None, None
    if active in LOCKED:
        return objs, [f"“{LAYERS[active]}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin."], None, None
    name, code = trim(action["name"]) or None, trim(action["code"]) or None
    o = {"kind": "point", "id": max([e["id"] for e in objs] + [0]) + 1, "layerId": active, "attrs": {"Kod": code} if code else {}, "p": P(x, y)}
    if name:
        o["label"] = name
    if z is not None:
        o["z"] = z
    said = []
    if name and same_name(objs, name):
        said.append(f"{PREFIX}“{name}” adında başka bir nokta da var.")
    objs.append(o)
    nxt = (increment(name) or "") if name else ""
    return objs, said, STEP_DRAFT, nxt


def drawing():
    def pt(i, label, x, y, z=None, kod=None, layer="cizim"):
        o = {"kind": "point", "id": i, "layerId": layer, "attrs": {"Kod": kod} if kod else {}, "p": P(x, y)}
        if label is not None:
            o["label"] = label
        if z is not None:
            o["z"] = z
        return o
    return [
        pt(1, "101", 0, 0, 100.0, "ST"),
        pt(2, "102", 10, 0),
        {"kind": "line", "id": 3, "layerId": "cizim", "attrs": {}, "a": P(0, 0), "b": P(10, 0), "za": 100.0},
        {"kind": "polyline", "id": 4, "layerId": "cizim", "attrs": {}, "pts": [P(0, 0), P(5, 5), P(10, 0.0000004)], "zs": [100.0, None, None]},
        {"kind": "polygon", "id": 5, "layerId": "cizim", "attrs": {"Parsel": "7"}, "pts": [P(0, 0), P(10, 0), P(10, 8), P(0, 8)], "zs": [100.0, None, None, 101.0],
         "holes": [{"pts": [P(2, 2), P(4, 2), P(4, 4), P(2, 4)]}]},
        pt(6, "103", 20, 0, None, None, "kilitli"),
        {"kind": "line", "id": 7, "layerId": "kilitli", "attrs": {}, "a": P(20, 0), "b": P(30, 0)},
        pt(8, "104", 30, 5),
        {"kind": "line", "id": 9, "layerId": "kilitli", "attrs": {}, "a": P(30, 5), "b": P(40, 5)},
        {"kind": "line", "id": 10, "layerId": "cizim", "attrs": {}, "a": P(10, 0.000002), "b": P(15, 0)},
    ]


def cases():
    cell_cases = [
        ("Ad: boşluklar atılır", 2, "name", " 105 ", True),
        ("Ad: başka bir noktanın adı söylenir", 2, "name", "101", True),
        ("Ad: boş, ad kalkar", 1, "name", "", True),
        ("Ad: aynı ad, yazılmaz", 1, "name", "101", True),
        ("Kod: verilir", 2, "code", "SN", True),
        ("Kod: boş, öznitelik kalkar", 1, "code", "  ", True),
        ("Kod: aynı kod, yazılmaz", 1, "code", "ST", True),
        ("Y: virgülle, bağlı çizgiler izler", 1, "east", "1,5", True),
        ("Y: bağlı çizgiler izlemez", 1, "east", "1.5", False),
        ("X: 1 µm içindeki köşe de izler, 2 µm ötedeki kalır", 2, "north", "-2", True),
        ("Y: sayı değil", 2, "east", "abc", True),
        ("X: boş, sayı değil", 2, "north", "", True),
        ("Y: aynı değer, yazılmaz", 2, "east", " 10 ", True),
        ("Z: bağlı köşeler yeni kotu alır", 1, "z", "99.5", True),
        ("Z: boş, kot ve bağlı köşelerinki kalkar", 1, "z", "", True),
        ("Z: bağlı çizgiler izlemez", 2, "z", "7", False),
        ("Z: sayı değil", 2, "z", "x", True),
        ("Z: aynı kot, yazılmaz", 1, "z", "100", True),
        ("kilitli katmandaki noktanın Y'si", 6, "east", "21", False),
        ("kilitli katmandaki noktanın adı", 6, "name", "x", False),
        ("kilitli nokta, bağlı çizgisi de kilitli: noktanınki söylenir", 6, "east", "21", True),
        ("bağlı çizgi kilitli katmanda: yazılmaz", 8, "east", "31", True),
        ("bağlı çizgi kilitli ama izlemez", 8, "east", "31", False),
        ("bağlı çizgi kilitli katmanda: kot da yazılmaz", 8, "z", "5", True),
    ]
    out = []
    objs = drawing()
    for name, i, col, text, fol in cell_cases:
        action = {"id": i, "column": col, "text": text, "follow": fol}
        after, said, step = cell(objs, action)
        out.append({"name": name, "objects": objs, "active": "cizim", "cell": action,
                    "expected": {"said": said, "step": step, "objects": [view(o) for o in after]}})
    draft_cases = [
        ("taslak: ad, Y, X ve kod; sonraki ad", "cizim", {"name": "201", "east": "50", "north": "60", "z": "", "code": " SN "}),
        ("taslak: adsız, virgülle, kotlu", "cizim", {"name": "", "east": "1,5", "north": "2.25", "z": "3", "code": ""}),
        ("taslak: X eksik", "cizim", {"name": "A", "east": "1", "north": "", "z": "", "code": ""}),
        ("taslak: Y eksik", "cizim", {"name": "A", "east": " ", "north": "1", "z": "", "code": ""}),
        ("taslak: Y ve X eksik", "cizim", {"name": "x", "east": "", "north": "", "z": "", "code": ""}),
        ("taslak: Y sayı değil", "cizim", {"name": "", "east": "a", "north": "1", "z": "", "code": ""}),
        ("taslak: Z sayı değil", "cizim", {"name": "", "east": "1", "north": "2", "z": "q", "code": ""}),
        ("taslak: başka bir noktanın adı söylenir", "cizim", {"name": "101", "east": "5", "north": "5", "z": "", "code": ""}),
        ("taslak: P9'dan sonra P10", "cizim", {"name": "P9", "east": "7", "north": "7", "z": "", "code": ""}),
        ("taslak: sayıyla bitmeyen addan sonra boş", "cizim", {"name": "Köşe", "east": "8", "north": "8", "z": "", "code": ""}),
        ("taslak: etkin katman kilitli", "kilitli", {"name": "301", "east": "9", "north": "9", "z": "", "code": ""}),
    ]
    for name, active, action in draft_cases:
        after, said, step, nxt = draft(objs, action, active)
        out.append({"name": name, "objects": objs, "active": active, "draft": action,
                    "expected": {"said": said, "step": step, "next": nxt, "objects": [view(o) for o in after]}})
    return out


def build():
    return {"format": "kentos.point-editor-edits", "version": 1, "layers": [{"id": k, "name": v, "locked": k in LOCKED} for k, v in LAYERS.items()], "cases": cases()}


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--check", action="store_true", help="compare with the written file instead of writing it")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if OUT.read_text() != text:
            print(f"{OUT} güncel değil: python3 scripts/fixtures/point_edit_cases.py", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT}: güncel")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    print(f"{OUT}: {len(json.loads(text)['cases'])} durum")


if __name__ == "__main__":
    main()
