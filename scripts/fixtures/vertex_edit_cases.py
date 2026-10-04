#!/usr/bin/env python3
"""Independent reference of Köşe tablosu's writes (docs/adr/0172 §4, §5).

Writes fixtures/vertex-table/v1/edits.json from the rules alone, with Python's standard library, mpmath and the core's
reference rules (vertex_table_cases.py), no KentOS code. Both platforms run every case on their own document through their
own table code (web `ui/bottom/vertexEdit.ts`, desktop `vertices/edit.rs`) and must leave the same drawing, say the same
messages, name the same undo step and keep the cell open alike.

A case's drawing is `objects` (v1 objects) on the layers “Çizim” (cizim) and “Kilitli” (kilitli, locked), in a project of
the case's type (`gis`: Y east and X north; `cad`: X east and Y north), lengths shown with three decimals. Its action is
one of:

- `cell`: the vertex `index` of path `path` (the elevations' order: a line's two ends, a polyline, an area's outer ring,
  its holes, then each other part's ring and holes) of object `id`, its `column` (east, north, z, radius) given `text`;
- `draft`: Satır ekle's row (`east`, `north`, `z`) written after that vertex;
- `remove`: the vertices `at` (each [path, index]) removed.

The rules:

1. Y, X, Z and Yarıçap are read as the point input reads a number: trimmed, the first comma a point, then a whole decimal
   number. An empty Z removes the elevation, an empty Yarıçap (or 0) makes the edge straight; an empty Y or X, or
   anything else, is not a number: “Köşe tablosu: Y bir sayı olmalı.” (the column's name in the project's type), the
   cell stays open, nothing is written.
2. A value the vertex already has writes nothing and says nothing: its coordinate, its elevation (none for none), its
   edge's radius within 1e-9 (relative; straight for an empty or 0 on a straight edge).
3. The core's rules (vertex_table_cases.py) give the new paths, or a refusal said in the table's words (below); the cell
   stays open. The slack of a radius is half a unit of three decimals, 0.0005 m.
4. An object on a locked layer is refused with the edit command's words; the cell closes.
5. A cell written is one undo step, “Köşe düzenle”.
6. A draft needs Y and X (“Köşe tablosu: Y ve X yazılmalı.”, “… Y yazılmalı.”, “… X yazılmalı.”, in the type's names and
   order), each a number (1); an empty Z is none. It is written after its vertex as Köşe ekle writes: “Köşe ekle”; the
   next draft goes after the new vertex.
7. Removed vertices are written as Köşe sil writes: “Köşe sil”.

The table's words for a refusal: ontoNeighbour (a move) “Köşe komşu köşesinin yerine taşınamaz; köşeyi kaldırmak için
satırı silin.”, (a draft) “Yeni köşe komşu köşesinin yerinde olamaz.”; noEdge “Bu köşeden çıkan kenar yok.”; lineArc
“Çizginin kenarı yay olamaz; önce köşe ekleyin.”; noChord “Kenarın iki ucu aynı yerde; yay olamaz.”; radiusBelow
“Yarıçap kirişin yarısından küçük olamaz: en az 20.616 m.” (the least as lengths are shown); lineEnds “Çizginin iki ucu
silinemez.”; pathMin “Çoklu çizgide en az iki köşe kalmalı.”; ringMin “Her halkada en az üç köşe kalmalı.”; missing
“Köşe bulunamadı.”; each after “Köşe tablosu: ”.

What is compared: the messages said, in order; the undo step's name (null: nothing written); whether the cell stays
open; every object after, in the drawing's order, as its kind and paths (vertices, elevations, and bulges as many as
the vertices or null when every edge is straight); for a draft written, where the next one goes.
"""

import argparse
import copy
import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import vertex_table_cases as vt  # noqa: E402  (the core's rules)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "vertex-table" / "v1" / "edits.json"
JS_SPACE = " \t\n\v\f\r                 　﻿"
LAYERS = {"cizim": "Çizim", "kilitli": "Kilitli"}
LOCKED = {"kilitli"}
PREFIX = "Köşe tablosu: "
STEP_CELL = "Köşe düzenle"
STEP_ADD = "Köşe ekle"
STEP_REMOVE = "Köşe sil"
SLACK = 0.0005
WORDS = {
    "gis": {"east": "Y", "north": "X", "z": "Z", "radius": "Yarıçap"},
    "cad": {"east": "X", "north": "Y", "z": "Z", "radius": "Yarıçap"},
}
REFUSALS = {
    "ontoNeighbour": "Köşe komşu köşesinin yerine taşınamaz; köşeyi kaldırmak için satırı silin.",
    "noEdge": "Bu köşeden çıkan kenar yok.",
    "lineArc": "Çizginin kenarı yay olamaz; önce köşe ekleyin.",
    "noChord": "Kenarın iki ucu aynı yerde; yay olamaz.",
    "lineEnds": "Çizginin iki ucu silinemez.",
    "pathMin": "Çoklu çizgide en az iki köşe kalmalı.",
    "ringMin": "Her halkada en az üç köşe kalmalı.",
    "missing": "Köşe bulunamadı.",
}
ONTO_NEW = "Yeni köşe komşu köşesinin yerinde olamaz."


def trim(t):
    return t.strip(JS_SPACE)


def number(text):
    t = trim(text).replace(",", ".", 1)
    if re.fullmatch(r"[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?", t):
        return float(t)
    return None


def P(x, y):
    return {"x": float(x), "y": float(y)}


def locked_words(o):
    return f"“{LAYERS[o['layerId']]}” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın."


def refusal_words(r, adding=False):
    if r["why"] == "radiusBelow":
        return f"{PREFIX}Yarıçap kirişin yarısından küçük olamaz: en az {r['least']:.3f} m."
    if r["why"] == "ontoNeighbour" and adding:
        return PREFIX + ONTO_NEW
    return PREFIX + REFUSALS[r["why"]]


# ── An object's paths, as the elevations list them ─────────────────────────

def ring(r):
    n = len(r["pts"])
    out = {"pts": [dict(p) for p in r["pts"]], "closed": True, "zs": list(r.get("zs") or [None] * n)}
    if r.get("bulges"):
        out["bulges"] = list(r["bulges"])
    return out


def paths_of(o):
    if o["kind"] == "line":
        return [{"pts": [dict(o["a"]), dict(o["b"])], "closed": False, "zs": [o.get("za"), o.get("zb")]}]
    if o["kind"] == "polyline":
        out = {"pts": [dict(p) for p in o["pts"]], "closed": False, "zs": list(o.get("zs") or [None] * len(o["pts"]))}
        if o.get("bulges"):
            out["bulges"] = list(o["bulges"])
        return [out]
    out = [ring(o)] + [ring(h) for h in o.get("holes") or []]
    for part in o.get("parts") or []:
        out += [ring(part)] + [ring(h) for h in part.get("holes") or []]
    return out


def with_paths(o, kind, ps):
    """The object as the write leaves it: its kind and its paths put back (zs and bulges dropped when empty)."""
    o = copy.deepcopy(o)

    def put(target, p):
        target["pts"] = p["pts"]
        if any(z is not None for z in p["zs"]):
            target["zs"] = p["zs"]
        else:
            target.pop("zs", None)
        if p.get("bulges"):
            target["bulges"] = p["bulges"]
        else:
            target.pop("bulges", None)

    if kind == "line":
        a, b = ps[0]["pts"]
        o["a"], o["b"] = a, b
        for k, z in zip(("za", "zb"), ps[0]["zs"]):
            if z is None:
                o.pop(k, None)
            else:
                o[k] = z
        return o
    if o["kind"] == "line":
        # A line given a vertex is a polyline.
        for k in ("a", "b", "za", "zb"):
            o.pop(k, None)
        o["kind"] = "polyline"
    if kind == "polyline":
        put(o, ps[0])
        return o
    it = iter(ps)
    put(o, next(it))
    for h in o.get("holes") or []:
        put(h, next(it))
    for part in o.get("parts") or []:
        put(part, next(it))
        for h in part.get("holes") or []:
            put(h, next(it))
    return o


def view(o):
    if o["kind"] not in ("line", "polyline", "polygon"):
        return {"kind": o["kind"]}
    out = []
    for p in paths_of(o):
        n = len(p["pts"])
        b = [vt.bulge_at(p, i) for i in range(n)]
        out.append({"pts": [[q["x"], q["y"]] for q in p["pts"]], "zs": p["zs"],
                    "bulges": b if any(abs(x) > 1e-12 for x in b) else None})
    return {"kind": o["kind"], "paths": out}


def shape(o):
    return {"kind": o["kind"], "paths": paths_of(o)}


# ── The writes ─────────────────────────────────────────────────────────────

def find(objects, i):
    return next(o for o in objects if o["id"] == i)


def replace(objects, o):
    return [o if x["id"] == o["id"] else x for x in objects]


def cell(objects, a, workspace):
    o = find(objects, a["id"])
    k, i, col, text = a["path"], a["index"], a["column"], a["text"]
    sh = shape(o)
    p = sh["paths"][k]
    blank = trim(text) == ""
    v = None if blank and col in ("z", "radius") else number(text)
    if v is None and not (blank and col in ("z", "radius")):
        return objects, [f"{PREFIX}{WORDS[workspace][col]} bir sayı olmalı."], None, True
    if col in ("east", "north"):
        at = p["pts"][i]
        to = {"x": v, "y": at["y"]} if col == "east" else {"x": at["x"], "y": v}
        if to == at:
            return objects, [], None, False
        out = vt.move(sh, k, i, to)
    elif col == "z":
        if v == p["zs"][i]:
            return objects, [], None, False
        out = vt.set_z(sh, k, i, v)
    else:
        j = vt.next_of(p, i)
        now = None
        if j is not None:
            r = vt.radius_of(vt.chord(p["pts"][i], p["pts"][j]), vt.bulge_at(p, i))
            now = None if r is None else float(r)
        # A radius within 1e-9 of the edge's is the edge's (the cell opens with it; the last bit is the platform's).
        same = v is not None and now is not None and abs(v - now) <= 1e-9 * abs(now)
        if (v is None or v == 0) and now is None or same:
            return objects, [], None, False
        out = vt.set_radius(sh, k, i, None if v == 0 else v, SLACK)
    if "refusal" in out:
        return objects, [refusal_words(out["refusal"])], None, True
    if o["layerId"] in LOCKED:
        return objects, [locked_words(o)], None, False
    e = out["edited"]
    return replace(objects, with_paths(o, e["kind"], e["paths"])), [], STEP_CELL, False


def draft(objects, a, workspace):
    o = find(objects, a["id"])
    w = WORDS[workspace]
    east, north = trim(a["east"]), trim(a["north"])
    fail = lambda said: (objects, [said], None, True, None)  # noqa: E731
    if not east and not north:
        return fail(f"{PREFIX}{w['east']} ve {w['north']} yazılmalı.")
    if not east:
        return fail(f"{PREFIX}{w['east']} yazılmalı.")
    if not north:
        return fail(f"{PREFIX}{w['north']} yazılmalı.")
    x = number(east)
    if x is None:
        return fail(f"{PREFIX}{w['east']} bir sayı olmalı.")
    y = number(north)
    if y is None:
        return fail(f"{PREFIX}{w['north']} bir sayı olmalı.")
    z = None
    if trim(a["z"]):
        z = number(a["z"])
        if z is None:
            return fail(f"{PREFIX}Z bir sayı olmalı.")
    out = vt.insert(shape(o), a["path"], a["after"], P(x, y), z)
    if "refusal" in out:
        return fail(refusal_words(out["refusal"], adding=True))
    if o["layerId"] in LOCKED:
        return objects, [locked_words(o)], None, False, None
    e = out["edited"]
    return replace(objects, with_paths(o, e["kind"], e["paths"])), [], STEP_ADD, False, [a["path"], a["after"] + 1]


def remove(objects, a):
    o = find(objects, a["id"])
    out = vt.remove(shape(o), [tuple(x) for x in a["at"]])
    if "refusal" in out:
        return objects, [refusal_words(out["refusal"])], None, False
    if o["layerId"] in LOCKED:
        return objects, [locked_words(o)], None, False
    e = out["edited"]
    return replace(objects, with_paths(o, e["kind"], e["paths"])), [], STEP_REMOVE, False


# ── The drawing and the cases ─────────────────────────────────────────────

def drawing():
    road = vt.ROAD["paths"][0]
    return [
        {"kind": "polyline", "id": 1, "layerId": "cizim", "attrs": {}, "pts": road["pts"], "bulges": road["bulges"],
         "zs": road["zs"]},
        {"kind": "polygon", "id": 2, "layerId": "cizim", "attrs": {"Parsel": "12"},
         "pts": vt.PARCEL["paths"][0]["pts"], "bulges": vt.PARCEL["paths"][0]["bulges"], "zs": vt.PARCEL["paths"][0]["zs"],
         "holes": [{"pts": vt.PARCEL["paths"][1]["pts"]}],
         "parts": [{"pts": vt.PARCEL["paths"][2]["pts"], "zs": vt.PARCEL["paths"][2]["zs"]}]},
        {"kind": "line", "id": 3, "layerId": "cizim", "attrs": {}, "a": vt.LINE["paths"][0]["pts"][0],
         "b": vt.LINE["paths"][0]["pts"][1], "za": 812.4},
        {"kind": "polyline", "id": 4, "layerId": "kilitli", "attrs": {}, "pts": [P(0, 0), P(10, 0), P(10, 10)]},
        {"kind": "polygon", "id": 5, "layerId": "cizim", "attrs": {}, "pts": vt.TRIANGLE["paths"][0]["pts"]},
    ]


def cases():
    objs = drawing()
    road = vt.ROAD["paths"][0]["pts"]
    out = []

    def run_cell(name, i, k, index, col, text, workspace="gis"):
        a = {"id": i, "path": k, "index": index, "column": col, "text": text}
        after, said, step, stay = cell(objs, a, workspace)
        out.append({"name": name, "objects": objs, "workspace": workspace, "cell": a,
                    "expected": {"said": said, "step": step, "stay": stay, "objects": [view(o) for o in after]}})

    def run_draft(name, i, k, after_index, east, north, z="", workspace="gis"):
        a = {"id": i, "path": k, "after": after_index, "east": east, "north": north, "z": z}
        after, said, step, stay, nxt = draft(objs, a, workspace)
        out.append({"name": name, "objects": objs, "workspace": workspace, "draft": a,
                    "expected": {"said": said, "step": step, "stay": stay, "next": nxt,
                                 "objects": [view(o) for o in after]}})

    def run_remove(name, i, at):
        a = {"id": i, "at": [list(x) for x in at]}
        after, said, step, stay = remove(objs, a)
        out.append({"name": name, "objects": objs, "workspace": "gis", "remove": a,
                    "expected": {"said": said, "step": step, "stay": stay, "objects": [view(o) for o in after]}})

    # Cells.
    run_cell("Y: köşe taşınır, yaylar bükümlerini korur", 1, 0, 2, "east", "486580.125")
    run_cell("X: virgülle", 1, 0, 2, "north", "4420155,5")
    run_cell("X: deliğin köşesi", 2, 1, 0, "north", "4420219")
    run_cell("Y: parçanın köşesi", 2, 2, 3, "east", "486801.5")
    run_cell("Y: aynı değer, yazılmaz", 1, 0, 1, "east", " 486540 ")
    run_cell("Y: sayı değil", 1, 0, 1, "east", "abc")
    run_cell("X: boş, sayı değil", 1, 0, 1, "north", "")
    run_cell("CAD: X (doğu) sayı değil", 1, 0, 1, "east", "x1", "cad")
    run_cell("CAD: Y (kuzey) sayı değil", 1, 0, 1, "north", "y", "cad")
    run_cell("Y: komşu köşenin yerine taşınamaz", 2, 1, 1, "east", "486720")
    run_cell("X: kapalı halkada son köşe ilkinin yerine taşınamaz", 5, 0, 2, "north", "0")
    run_cell("Z: kotsuz köşeye kot", 1, 0, 2, "z", "805.125")
    run_cell("Z: boş, kot kalkar", 1, 0, 0, "z", "")
    run_cell("Z: kotsuz köşede boş, yazılmaz", 1, 0, 2, "z", " ")
    run_cell("Z: aynı kot, yazılmaz", 1, 0, 3, "z", "802.5")
    run_cell("Z: sayı değil", 1, 0, 3, "z", "q")
    run_cell("Z: çizginin kotsuz ucu", 3, 0, 1, "z", "813")
    run_cell("Yarıçap: düz kenar yay olur", 1, 0, 0, "radius", "40")
    run_cell("Yarıçap: yay sağa döner", 1, 0, 1, "radius", "-35")
    run_cell("Yarıçap: büyük yay büyük kalır", 1, 0, 3, "radius", "30")
    run_cell("Yarıçap: boş, kenar düzleşir", 1, 0, 1, "radius", "")
    run_cell("Yarıçap: 0, kenar düzleşir", 1, 0, 2, "radius", "0")
    run_cell("Yarıçap: düz kenarda boş, yazılmaz", 1, 0, 0, "radius", "")
    run_cell("Yarıçap: düz kenarda 0, yazılmaz", 1, 0, 0, "radius", "0")
    radius_now = float(vt.radius_of(vt.chord(road[1], road[2]), vt.ROAD["paths"][0]["bulges"][1]))
    run_cell("Yarıçap: aynı değer, yazılmaz", 1, 0, 1, "radius", repr(radius_now))
    run_cell("Yarıçap: sayı değil", 1, 0, 1, "radius", "r")
    run_cell("Yarıçap: kirişin yarısından kısa", 1, 0, 1, "radius", "26")
    run_cell("Yarıçap: gösterilen en az değer yarım daire olur", 1, 0, 1, "radius", "26.834")
    run_cell("Yarıçap: en az değere yukarı yuvarlanan küçük yay olur", 1, 0, 0, "radius", "20.616")
    run_cell("Yarıçap: eksi, kirişin yarısından kısa", 1, 0, 0, "radius", "-12.5")
    run_cell("Yarıçap: kapanan kenar", 2, 0, 4, "radius", "-80")
    run_cell("Yarıçap: çizginin kenarı", 3, 0, 0, "radius", "50")
    run_cell("Yarıçap: çoklu çizginin son köşesi", 1, 0, 4, "radius", "50")
    run_cell("kilitli katmandaki nesnenin köşesi", 4, 0, 1, "east", "11")
    run_cell("kilitli katman: sayı olmayan değer önce söylenir", 4, 0, 1, "east", "a")
    # Drafts.
    run_draft("taslak: yaylı kenara köşe", 1, 0, 1, "486560", "4420125", "810")
    run_draft("taslak: virgülle, kotsuz", 1, 0, 0, "486520,5", "4420098.5")
    run_draft("taslak: kapalı halkanın sonuna", 2, 0, 4, "486694", "4420221", "850.6")
    run_draft("taslak: çizgi çoklu çizgi olur", 3, 0, 0, "486512", "4420125")
    run_draft("taslak: X eksik", 1, 0, 1, "486560", " ")
    run_draft("taslak: Y eksik", 1, 0, 1, "", "4420125")
    run_draft("taslak: Y ve X eksik", 1, 0, 1, "", "")
    run_draft("CAD taslak: X ve Y eksik", 1, 0, 1, "", "", "", "cad")
    run_draft("taslak: Y sayı değil", 1, 0, 1, "y", "4420125")
    run_draft("taslak: Z sayı değil", 1, 0, 1, "486560", "4420125", "z")
    run_draft("taslak: komşunun yerinde", 1, 0, 1, "486575.5", "4420150.25")
    run_draft("taslak: kilitli katman", 4, 0, 0, "5", "1")
    # Removals.
    run_remove("sil: iç köşe", 1, [(0, 2)])
    run_remove("sil: halkalardan birlikte", 2, [(0, 1), (2, 0)])
    run_remove("sil: halkada üç köşe kalmalı", 5, [(0, 0)])
    run_remove("sil: çoklu çizgide iki köşe kalmalı", 1, [(0, 0), (0, 1), (0, 2), (0, 3)])
    run_remove("sil: çizginin ucu", 3, [(0, 1)])
    run_remove("sil: kilitli katman", 4, [(0, 1)])
    return out


def build():
    return {"format": "kentos.vertex-edits", "version": 1,
            "layers": [{"id": k, "name": v, "locked": k in LOCKED} for k, v in LAYERS.items()], "cases": cases()}


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="fail when the fixture is not what the rules give")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} kurallardan çıkan değil; yeniden yazın: "
                  f"python3 {Path(__file__).relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} kurallardan çıkanla aynı.")
        return 0
    OUT.write_text(text, encoding="utf-8")
    d = json.loads(text)
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(d['cases'])} durum.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
