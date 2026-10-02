#!/usr/bin/env python3
"""Independent reference of the point editor's Çift noktaları ayıkla (docs/adr/0153 §5).

Writes fixtures/point-editor/v1/dedupe.json from the rules alone, with
Python's standard library and no KentOS code. Both platforms run every case
on their own document through their own table code (web
`ui/bottom/pointBatch.ts`, desktop `points/batch.rs`) and must find the same
groups, leave the same drawing, show the same summary, say the same messages
and name the same undo step.

A case's drawing is `objects` (v1 objects) on the layers “Çizim” (cizim),
“Kot” (kot) and “Kilitli” (kilitli, locked). Its `targets` are the points
the operation takes, in the table's order; `by` is name or place, `tolerance`
the window's text (metres), `keep` first, last or average, `follow` Bağlı
çizgiler izler.

The rules:

1. The tolerance is read as the point input reads a number (trimmed, the
   first comma a point, then a whole decimal number) and must be zero or
   more: else “Nokta editörü: Tolerans sıfır ya da daha büyük bir sayı
   olmalı.” and nothing is written. Under Aynı ad it is not read.
2. The targets are taken in the drawing's order.
3. Aynı ad: names trimmed (JavaScript's white space), compared exactly; a
   point without a name is in no group. Aynı yer: each point joins the first
   group (in the order they were opened) whose first point is within the
   tolerance of it (the plane distance; under 1e-9 m the very same place),
   else it opens a group. Only groups of two points or more count.
4. Tutulan: İlki or Sonuncusu in the drawing's order, or Ortalaması: the
   first, at the members' mean place and the mean of the elevations they
   have (none of them: none). The others are removed.
5. No group: “Nokta editörü: Çift nokta yok.” and nothing is written.
6. The first point that changes, in the drawing's order (one removed, or the
   kept one when its place or elevation changes), on a locked layer stops it
   all with the edit command's words.
7. With `follow`, the line work with a vertex within 1e-6 m of a kept point's
   place before it moves follows it to its new place, and takes its new
   elevation when that changes (else keeps its own); a line work on a locked
   layer among them stops it all (“… Bağlı çizgilerden biri kilitli katmanda;
   …”).
8. One undo step, “Çift noktaları ayıkla”: the kept points moved group by
   group, then the others removed. Said: “Nokta editörü: 6 grupta 9 nokta
   silindi.”, with “, 2 nokta ortalamaya taşındı” before the full stop when
   some moved.
9. The window's summary: “6 grupta 15 nokta; 9 nokta silinecek.”, with “, 2
   nokta ortalamaya taşınacak” when some would move; “Çift nokta yok.”
   without groups.
10. After İçe aktar from the table (`imports`): the imported points' names,
   trimmed (a point without one is left out), and of them those two points
   or more of the drawing carry. The window's targets are the points that
   carry one of them, in the drawing's order, its header “Aynı adlı 8
   nokta”, its criterion Aynı ad; none: no window opens (targets empty,
   header null). An import case's drawing is `importObjects`, the imported
   points last.

What is compared: the groups (their members' ids, in the drawing's order);
the summary; the messages said; the undo step's name (null: nothing
written); every object after, in the drawing's order, as its kind, layer,
label and attributes, a point's place and elevation, line work's paths
(vertices and elevations). Numbers within 1e-9: the mean is worked out here
with exact fractions.
"""

import argparse
import copy
import json
import math
import re
import sys
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "point-editor" / "v1" / "dedupe.json"
ON = 1e-6
LEAST = 1e-9
JS_SPACE = " \t\n\v\f\r                 　﻿"
LAYERS = {"cizim": "Çizim", "kot": "Kot", "kilitli": "Kilitli"}
LOCKED = {"kilitli"}
PREFIX = "Nokta editörü: "
STEP = "Çift noktaları ayıkla"
FOLLOW_LOCKED = f"{PREFIX}Bağlı çizgilerden biri kilitli katmanda; katmanın kilidini açın ya da Bağlı çizgiler izler'i kapatın."


def trim(t):
    return t.strip(JS_SPACE)


def number(text):
    t = trim(text).replace(",", ".", 1)
    if re.fullmatch(r"[+-]?([0-9]+\.?[0-9]*|\.[0-9]+)([eE][+-]?[0-9]+)?", t):
        return float(t)
    return None


def P(x, y):
    return {"x": float(x), "y": float(y)}


def near(a, b, tol=ON):
    return math.hypot(a["x"] - b["x"], a["y"] - b["y"]) <= tol


def mean(values):
    return float(sum(Fraction(v) for v in values) / len(values))


def paths(o):
    if o["kind"] == "line":
        return [([o["a"], o["b"]], [o.get("za"), o.get("zb")])]
    return [(o["pts"], list(o.get("zs") or [None] * len(o["pts"])))]


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
    pts, zs = ps[0]
    o["pts"] = pts
    if any(z is not None for z in zs):
        o["zs"] = zs
    else:
        o.pop("zs", None)


def follow(o, frm, to, set_z, z):
    """The object with its vertices at `frm` moved to `to` (and given `z`), or None when none is there."""
    hit = False
    out = []
    for pts, zs in paths(o):
        pts, zs = [dict(p) for p in pts], list(zs)
        for k, p in enumerate(pts):
            if near(p, frm):
                hit = True
                pts[k] = dict(to)
                if set_z:
                    zs[k] = z
        out.append((pts, zs))
    if not hit:
        return None
    o = copy.deepcopy(o)
    set_paths(o, out)
    return o


def view(o):
    v = {"kind": o["kind"], "layerId": o["layerId"], "label": o.get("label"), "attrs": o.get("attrs", {})}
    if o["kind"] == "point":
        v["at"] = [o["p"]["x"], o["p"]["y"]]
        v["z"] = o.get("z")
    else:
        v["paths"] = [{"pts": [[p["x"], p["y"]] for p in pts], "zs": zs} for pts, zs in paths(o)]
    return v


def groups_of(points, by, tol):
    """The groups of two points or more, each its members in the drawing's order."""
    groups = []
    if by == "name":
        index = {}
        for p in points:
            name = trim(p.get("label") or "")
            if not name:
                continue
            if name in index:
                groups[index[name]].append(p)
            else:
                index[name] = len(groups)
                groups.append([p])
    else:
        exact = tol < LEAST
        for p in points:
            for g in groups:
                a = g[0]["p"]
                if (a == p["p"]) if exact else near(a, p["p"], tol):
                    g.append(p)
                    break
            else:
                groups.append([p])
    return [g for g in groups if len(g) > 1]


def run(objects, ids, by, tolerance, keep, fol):
    objs = copy.deepcopy(objects)
    order = {o["id"]: k for k, o in enumerate(objs)}
    by_id = {o["id"]: o for o in objs}
    tol = 0.0
    if by == "place":
        t = number(tolerance)
        if t is None or t < 0:
            return objs, [], None, [f"{PREFIX}Tolerans sıfır ya da daha büyük bir sayı olmalı."], None
        tol = t
    points = sorted((by_id[i] for i in ids), key=lambda o: order[o["id"]])
    groups = groups_of(points, by, tol)
    ids_of = [[p["id"] for p in g] for g in groups]
    if not groups:
        return objs, ids_of, "Çift nokta yok.", [f"{PREFIX}Çift nokta yok."], None
    # The kept points and where they stand after; the others removed.
    moves, removed = [], []
    for g in groups:
        if keep == "first":
            kept, at, z = g[0], g[0]["p"], g[0].get("z")
        elif keep == "last":
            kept, at, z = g[-1], g[-1]["p"], g[-1].get("z")
        else:
            kept = g[0]
            at = P(mean([p["p"]["x"] for p in g]), mean([p["p"]["y"] for p in g]))
            zs = [p["z"] for p in g if p.get("z") is not None]
            z = mean(zs) if zs else None
        if at != kept["p"] or z != kept.get("z"):
            moves.append((kept, at, z))
        removed.extend(p for p in g if p is not kept)
    total = sum(len(g) for g in groups)
    summary = f"{len(groups)} grupta {total} nokta; {len(removed)} nokta silinecek"
    summary += f", {len(moves)} nokta ortalamaya taşınacak." if moves else "."
    changing = sorted([m[0] for m in moves] + removed, key=lambda o: order[o["id"]])
    for o in changing:
        if o["layerId"] in LOCKED:
            return objs, ids_of, summary, [f"“{LAYERS[o['layerId']]}” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın."], None
    for kept, at, z in moves:
        frm = dict(kept["p"])
        set_z = z != kept.get("z")
        followers = []
        if fol:
            for o in objs:
                if o["kind"] in ("line", "polyline", "polygon"):
                    f = follow(o, frm, at, set_z, z)
                    if f is not None and f != o:
                        if o["layerId"] in LOCKED:
                            return copy.deepcopy(objects), ids_of, summary, [FOLLOW_LOCKED], None
                        followers.append((o, f))
        kept["p"] = dict(at)
        if z is None:
            kept.pop("z", None)
        else:
            kept["z"] = z
        for o, f in followers:
            o.clear()
            o.update(f)
    gone = {o["id"] for o in removed}
    objs = [o for o in objs if o["id"] not in gone]
    said = f"{PREFIX}{len(groups)} grupta {len(removed)} nokta silindi"
    said += f", {len(moves)} nokta ortalamaya taşındı." if moves else "."
    return objs, ids_of, summary, [said], STEP


def drawing():
    def pt(i, label, x, y, layer="cizim", z=None):
        o = {"kind": "point", "id": i, "layerId": layer, "attrs": {}, "p": P(x, y)}
        if label is not None:
            o["label"] = label
        if z is not None:
            o["z"] = z
        return o
    return [
        pt(1, "101", 0, 0, z=100.0),
        pt(2, "102", 10, 0),
        pt(3, "101", 0.0004, -0.0003, "kot", z=100.4),
        pt(4, "103", 20, 0, z=101.0),
        pt(5, None, 20.0006, 0.0002),
        pt(6, "103", 20.5, 0, z=103.0),
        pt(7, "104", 30, 0, "kilitli"),
        pt(8, "104", 30.0002, 0),
        pt(9, " 102 ", 40, 0),
        pt(10, "105", 50, 0),
        pt(11, "105", 50.0009, 0, z=99.0),
        pt(12, "106", 487060.123, 4420000.456),
        pt(13, "106", 487060.1245, 4420000.456),
        {"kind": "line", "id": 14, "layerId": "cizim", "attrs": {}, "a": P(0, 0), "b": P(10, 0), "za": 100.0},
        {"kind": "polyline", "id": 15, "layerId": "cizim", "attrs": {}, "pts": [P(20.0006, 0.0002), P(25, 5), P(30.0002, 0)]},
        {"kind": "line", "id": 16, "layerId": "kilitli", "attrs": {}, "a": P(50, 0), "b": P(55, 5)},
        pt(17, "107", 487070.25, 4420010.75),
        pt(18, "107", 487070.25, 4420010.75, "kot"),
        pt(19, "108", 80, 0, z=100.0),
        pt(20, "108", 80.0004, 0),
        {"kind": "line", "id": 21, "layerId": "cizim", "attrs": {}, "a": P(80, 0), "b": P(85, 0), "za": 95.0},
    ]


ALL = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 17, 18, 19, 20]


def cases():
    objs = drawing()
    rows = [
        ("Aynı ad, ilki: adların boşlukları atılır, kilitli tutulan değişmez", ALL, "name", "", "first", True),
        ("Aynı ad, sonuncusu: kilitli nokta silinecek", ALL, "name", "", "last", True),
        ("Aynı ad, sonuncusu: kilitli nokta tabloda değil", [i for i in ALL if i != 7], "name", "", "last", True),
        ("Aynı ad, ortalaması: kotların ortalaması, bağlı çizgiler izler", [1, 3, 2, 9], "name", "", "average", True),
        ("Aynı ad, ortalaması: bağlı çizgiler izlemez", [1, 3, 2, 9], "name", "", "average", False),
        ("Aynı ad: adsız nokta çift sayılmaz", [4, 5, 6], "name", "", "first", True),
        ("Aynı yer 0.001 m, ilki", ALL, "place", "0.001", "first", True),
        ("Aynı yer, ortalaması: kilitli katmandaki tutulan taşınacak", ALL, "place", "0.001", "average", True),
        ("Aynı yer, ortalaması: bağlı çizgi kilitli katmanda", [10, 11], "place", "0.001", "average", True),
        ("Aynı yer, ortalaması: bağlı çizgiler izlemezse yazılır", [10, 11], "place", "0.001", "average", False),
        ("Aynı yer, ortalaması: kotu olmayan noktanın kotu yalnız ötekinin", [4, 5], "place", "0.001", "average", True),
        ("Aynı yer, sonuncusu: tutulan taşınmaz", [4, 5], "place", "0.001", "last", True),
        ("Aynı ad, ortalaması: tutulanın kotu değişmezse bağlı çizginin kotu kalır", [19, 20], "name", "", "average", True),
        ("Aynı yer, tolerans 0: yalnız tam aynı yer", ALL, "place", "0", "first", True),
        ("Aynı yer, tolerans virgülle 0,002", ALL, "place", " 0,002 ", "first", True),
        ("Aynı yer, büyük koordinatlarda ortalama", [12, 13, 17, 18], "place", "0.002", "average", True),
        ("Tolerans sayı değil", ALL, "place", "abc", "first", True),
        ("Tolerans eksi", ALL, "place", "-0.001", "first", True),
        ("Tablodaki sıra çizim sırasına çevrilir", [3, 1], "name", "", "first", True),
        ("Çift nokta yok", [1, 2, 4], "place", "0.001", "first", True),
    ]
    out = []
    for name, ids, by, tol, keep, fol in rows:
        after, groups, summary, said, step = run(objs, ids, by, tol, keep, fol)
        out.append({
            "name": name, "targets": ids, "by": by, "tolerance": tol, "keep": keep, "follow": fol,
            "expected": {"groups": groups, "summary": summary, "said": said, "step": step, "objects": [view(o) for o in after]},
        })
    return out


def import_drawing():
    """A drawing that has had a coordinate list imported: the imported points (7–13) are last."""
    def pt(i, label, x, y, layer, z=None):
        o = {"kind": "point", "id": i, "layerId": layer, "attrs": {}, "p": P(x, y)}
        if label is not None:
            o["label"] = label
        if z is not None:
            o["z"] = z
        return o
    return [
        pt(1, "101", 0, 0, "cizim", z=100.0),
        pt(2, "102", 10, 0, "cizim"),
        pt(3, "103", 20, 0, "cizim"),
        pt(4, None, 30, 0, "cizim"),
        pt(5, "104", 40, 0, "kilitli"),
        {"kind": "line", "id": 6, "layerId": "cizim", "attrs": {}, "a": P(0, 0), "b": P(10, 0), "za": 100.0},
        pt(7, "101", 0.02, -0.01, "kot", z=100.3),
        pt(8, " 102 ", 10.01, 0, "kot"),
        pt(9, "105", 50, 0, "kot"),
        pt(10, None, 60, 0, "kot"),
        pt(11, "106", 70, 0, "kot"),
        pt(12, "106", 70.5, 0, "kot"),
        pt(13, "104", 40.01, 0, "kot"),
    ]


def import_targets(objects, imported):
    """Rule 10: the points carrying an imported name that two points or more carry, and the window's header."""
    points = [o for o in objects if o["kind"] == "point"]
    by_id = {o["id"]: o for o in points}
    names = {trim(by_id[i].get("label") or "") for i in imported if i in by_id} - {""}
    carriers = {}
    for o in points:
        name = trim(o.get("label") or "")
        if name in names:
            carriers.setdefault(name, []).append(o["id"])
    twice = {n for n, ids in carriers.items() if len(ids) > 1}
    ids = [o["id"] for o in points if trim(o.get("label") or "") in twice]
    return ids, (f"Aynı adlı {len(ids)} nokta" if ids else None)


def import_cases():
    objs = import_drawing()
    everything = [7, 8, 9, 10, 11, 12, 13]
    rows = [
        ("İlki çizimdekini tutar: dosyanın aynı adlıları silinir, kilitli tutulan değişmez", everything, "first", True),
        ("Sonuncusu dosyadakini tutar: kilitli katmandaki çizim noktası silinecek", everything, "last", True),
        ("Sonuncusu dosyadakini tutar", [7, 8, 9, 10, 11, 12], "last", True),
        ("Ortalaması ikisinin ortalaması; bağlı çizgi izler", [7, 8, 9, 10, 11, 12], "average", True),
        ("Adı başka noktada olmayan içe aktarmada pencere açılmaz", [9, 10], "first", True),
    ]
    out = []
    for name, imported, keep, fol in rows:
        targets, header = import_targets(objs, imported)
        expected = {"targets": targets, "header": header}
        if targets:
            after, groups, summary, said, step = run(objs, targets, "name", "", keep, fol)
            expected.update({"groups": groups, "summary": summary, "said": said, "step": step, "objects": [view(o) for o in after]})
        out.append({"name": name, "imported": imported, "keep": keep, "follow": fol, "expected": expected})
    return out


def build():
    return {
        "format": "kentos.point-editor-dedupe",
        "version": 1,
        "layers": [{"id": k, "name": v, "locked": k in LOCKED, "visible": True} for k, v in LAYERS.items()],
        "objects": drawing(),
        "cases": cases(),
        "importObjects": import_drawing(),
        "imports": import_cases(),
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--check", action="store_true", help="compare with the written file instead of writing it")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT} güncel değil: python3 scripts/fixtures/point_dedupe_cases.py", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT}: güncel")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    print(f"{OUT}: {len(json.loads(text)['cases'])} durum")


if __name__ == "__main__":
    main()
