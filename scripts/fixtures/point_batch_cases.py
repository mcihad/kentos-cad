#!/usr/bin/env python3
"""Independent reference of the point editor's batch operations (docs/adr/0153 §5).

Writes fixtures/point-editor/v1/batch.json from the rules alone, with
Python's standard library and no KentOS code. Both platforms run every case
on their own document through their own table code (web
`ui/bottom/pointBatch.ts`, desktop `points/batch.rs`) and must leave the
same drawing, say the same messages and name the same undo step.

A case's drawing is `objects` (v1 objects) on the layers “Çizim” (cizim),
“Kot” (kot), “Gizli” (gizli, hidden) and “Kilitli” (kilitli, locked). Its
`targets` are the points the operation takes, in the table's order. The
operations:

- `rename`: `mode` add or remove, `prefix`;
- `number`: `start`;
- `layer`: `layer` (an id).

The rules:

1. Names are compared and written trimmed (JavaScript's white space).
2. Yeniden adlandır: the prefix is trimmed; an empty one is said (“Nokta
   editörü: Önek yazılmalı.”) and nothing is written. Önek ekle puts the
   prefix before every name; a point without a name keeps none. Önek kaldır
   takes it from the names that start with it (case counts), the rest
   trimmed; a name that would be left empty does not change.
3. Sıralı numara ver: the start is trimmed and must end with a digit
   (“Nokta editörü: Başlangıç adı sayıyla bitmeli.”; the digits 0–9). The first target takes
   the start, every next one the one before's Artır (the digits it ends with
   plus one, as many digits at least).
4. Only the points whose name changes are written, one by one in the
   targets' order, as one undo step (“Yeniden adlandır”, “Sıralı numara
   ver”). None changing is said (“Nokta editörü: Adı değişen nokta yok.”).
   A changing point on a locked layer stops it all with the set command's
   words. After the write it is said how many names changed, and how many
   of the new names another point has too.
5. Katmana taşı: the points not on the layer already move, with one set
   command (step “Katmana taşı”). None to move is said (“Nokta editörü:
   Taşınacak nokta yok.”). The command's checks answer in its order: a
   moving point on a locked layer, then the layer locked. It is said how
   many moved where, then the command's warning when the layer is hidden.

What is compared: the messages said, in order; the undo step's name (null:
nothing written); every object after, in the drawing's order, as its kind,
layer, label and attributes, and for a point its place and elevation.

`targets` cases: the rows the operations take from the table's rows
(`shown`, in order) and the selection (`selected`): the selected rows in the
table's order, or every row when none of them is selected; with the menu's
header naming them.
"""

import argparse
import copy
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "point-editor" / "v1" / "batch.json"
JS_SPACE = " \t\n\v\f\r                 　﻿"
LAYERS = {"cizim": "Çizim", "kot": "Kot", "gizli": "Gizli", "kilitli": "Kilitli"}
LOCKED = {"kilitli"}
HIDDEN = {"gizli"}
PREFIX = "Nokta editörü: "
STEPS = {"rename": "Yeniden adlandır", "number": "Sıralı numara ver", "layer": "Katmana taşı"}


def trim(t):
    return t.strip(JS_SPACE)


def increment(name):
    m = re.search(r"([0-9]+)$", name)
    if not m:
        return None
    digits = m.group(1)
    return name[: m.start()] + str(int(digits) + 1).rjust(len(digits), "0")


def P(x, y):
    return {"x": float(x), "y": float(y)}


def view(o):
    v = {"kind": o["kind"], "layerId": o["layerId"], "label": o.get("label"), "attrs": o.get("attrs", {})}
    if o["kind"] == "point":
        v["at"] = [o["p"]["x"], o["p"]["y"]]
        v["z"] = o.get("z")
    return v


def edit_refusal(layer):
    return f"“{LAYERS[layer]}” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın."


def names(objs, targets, news, step):
    """The new names written (None: the point keeps its own)."""
    changed = [(o, n) for o, n in zip(targets, news) if n is not None and n != o.get("label")]
    if not changed:
        return [f"{PREFIX}Adı değişen nokta yok."], None
    for o, _ in changed:
        if o["layerId"] in LOCKED:
            return [edit_refusal(o["layerId"])], None
    for o, n in changed:
        o["label"] = n
    shared = sum(
        1
        for o, n in changed
        if any(p is not o and p["kind"] == "point" and trim(p.get("label") or "") == n for p in objs)
    )
    said = f"{PREFIX}{len(changed)} noktanın adı değişti."
    if shared:
        said = f"{PREFIX}{len(changed)} noktanın adı değişti; {shared} ad başka noktalarda da var."
    return [said], step


def run(objects, ids, op):
    objs = copy.deepcopy(objects)
    by_id = {o["id"]: o for o in objs}
    targets = [by_id[i] for i in ids]
    kind = op["kind"]
    if kind == "rename":
        p = trim(op["prefix"])
        if not p:
            return objs, [f"{PREFIX}Önek yazılmalı."], None
        news = []
        for o in targets:
            name = trim(o.get("label") or "")
            if op["mode"] == "add":
                news.append(p + name if name else None)
            else:
                rest = trim(name[len(p):]) if name.startswith(p) else ""
                news.append(rest or None)
        said, step = names(objs, targets, news, STEPS[kind])
        return objs, said, step
    if kind == "number":
        s = trim(op["start"])
        if not re.search(r"[0-9]$", s):
            return objs, [f"{PREFIX}Başlangıç adı sayıyla bitmeli."], None
        news, n = [], s
        for _ in targets:
            news.append(n)
            n = increment(n)
        said, step = names(objs, targets, news, STEPS[kind])
        return objs, said, step
    # Katmana taşı.
    layer = op["layer"]
    moving = [o for o in targets if o["layerId"] != layer]
    if not moving:
        return objs, [f"{PREFIX}Taşınacak nokta yok."], None
    for o in moving:
        if o["layerId"] in LOCKED:
            return objs, [edit_refusal(o["layerId"])], None
    if layer in LOCKED:
        return objs, [f"“{LAYERS[layer]}” katmanı kilitli; nesneler ona taşınamaz. Kilidi Katmanlar panelinden açın ya da başka bir katman seçin."], None
    for o in moving:
        o["layerId"] = layer
    said = [f"{PREFIX}{len(moving)} nokta “{LAYERS[layer]}” katmanına taşındı."]
    if layer in HIDDEN:
        said.append(f"“{LAYERS[layer]}” katmanı gizli; taşınan nesneler görünmeyecek.")
    return objs, said, STEPS[kind]


def drawing():
    def pt(i, label, x, y, layer="cizim", z=None, kod=None):
        o = {"kind": "point", "id": i, "layerId": layer, "attrs": {"Kod": kod} if kod else {}, "p": P(x, y)}
        if label is not None:
            o["label"] = label
        if z is not None:
            o["z"] = z
        return o
    return [
        pt(1, "101", 0, 0, z=100.0, kod="ST"),
        pt(2, "102", 10, 0),
        pt(3, None, 20, 0),
        pt(4, "P.103", 30, 0),
        pt(5, "P.", 40, 0),
        pt(6, " 104 ", 50, 0),
        pt(7, "105", 60, 0, "kot"),
        pt(8, "106", 70, 0, "kilitli"),
        pt(9, "A1", 80, 0),
        {"kind": "line", "id": 10, "layerId": "cizim", "attrs": {}, "a": P(0, 0), "b": P(10, 0)},
        pt(11, "P.101", 90, 0, "kot"),
        pt(12, "1", 100, 0, "kot"),
        pt(13, "P. 107", 110, 0),
    ]


def targets(shown, selected):
    picked = [i for i in shown if i in selected]
    if picked:
        return picked, f"{len(picked)} seçili nokta"
    return list(shown), f"Tablodaki {len(shown)} nokta"


def cases():
    objs = drawing()
    ops = [
        ("Önek ekle: adlılara; adsız değişmez, başka noktadaki ad söylenir", [1, 2, 3, 6], {"kind": "rename", "mode": "add", "prefix": "P."}),
        ("Önek ekle: önekin boşlukları atılır", [2], {"kind": "rename", "mode": "add", "prefix": " X- "}),
        ("Önek kaldır: taşıyanlardan atılır, kalanın boşlukları da; kalan boşsa ve önek yoksa değişmez", [4, 5, 1, 13], {"kind": "rename", "mode": "remove", "prefix": "P."}),
        ("Önek kaldır: büyük küçük harf ayrı", [4], {"kind": "rename", "mode": "remove", "prefix": "p."}),
        ("Önek boş", [1], {"kind": "rename", "mode": "add", "prefix": "  "}),
        ("Önek ekle: kilitli katmandaki nokta hepsini durdurur", [1, 8], {"kind": "rename", "mode": "add", "prefix": "K"}),
        ("Önek kaldır: kilitli nokta değişmiyorsa durdurmaz", [4, 8], {"kind": "rename", "mode": "remove", "prefix": "P."}),
        ("Sıralı numara: tablonun sırasıyla", [3, 1, 2], {"kind": "number", "start": "201"}),
        ("Sıralı numara: öndeki sıfırlar korunur, basamak artar", [1, 2, 3], {"kind": "number", "start": "P-098"}),
        ("Sıralı numara: 101/1 biçimi", [7, 9], {"kind": "number", "start": " 101/1 "}),
        ("Sıralı numara: başlangıç sayıyla bitmiyor", [1, 2], {"kind": "number", "start": "A"}),
        ("Sıralı numara: boş başlangıç", [1], {"kind": "number", "start": " "}),
        ("Sıralı numara: yalnız 0–9 rakamları sayılır", [1], {"kind": "number", "start": "A٣"}),
        ("Sıralı numara: adlar zaten öyle", [1, 2], {"kind": "number", "start": "101"}),
        ("Sıralı numara: boşluklu ad da yazılır", [6], {"kind": "number", "start": "104"}),
        ("Sıralı numara: başka noktadaki ad söylenir", [9, 3], {"kind": "number", "start": "1"}),
        ("Sıralı numara: kilitli nokta değişirse durdurur", [1, 8], {"kind": "number", "start": "301"}),
        ("Katmana taşı: zaten oradakiler değişmez", [1, 7, 2], {"kind": "layer", "layer": "kot"}),
        ("Katmana taşı: hepsi orada", [7, 11], {"kind": "layer", "layer": "kot"}),
        ("Katmana taşı: gizli katmana, söylenir", [1], {"kind": "layer", "layer": "gizli"}),
        ("Katmana taşı: kilitli katmana", [1], {"kind": "layer", "layer": "kilitli"}),
        ("Katmana taşı: kilitli katmandaki nokta", [2, 8, 1], {"kind": "layer", "layer": "kot"}),
    ]
    out = []
    for name, ids, op in ops:
        after, said, step = run(objs, ids, op)
        out.append({"name": name, "targets": ids, "op": op,
                    "expected": {"said": said, "step": step, "objects": [view(o) for o in after]}})
    rows = []
    for name, shown, selected in [
        ("seçim yok: tablodaki bütün satırlar", [1, 2, 3], []),
        ("seçili satırlar tablonun sırasıyla", [3, 1, 2, 9], [2, 3]),
        ("tabloda olmayan seçili nokta sayılmaz", [3, 1, 2], [2, 7]),
        ("seçili nokta tabloda değil: bütün satırlar", [1, 2], [7]),
        ("boş tablo", [], [1]),
    ]:
        ids, header = targets(shown, selected)
        rows.append({"name": name, "shown": shown, "selected": selected, "expected": {"targets": ids, "header": header}})
    return out, rows


def build():
    ops, rows = cases()
    return {
        "format": "kentos.point-editor-batch",
        "version": 1,
        "layers": [{"id": k, "name": v, "locked": k in LOCKED, "visible": k not in HIDDEN} for k, v in LAYERS.items()],
        "objects": drawing(),
        "cases": ops,
        "targets": rows,
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--check", action="store_true", help="compare with the written file instead of writing it")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT} güncel değil: python3 scripts/fixtures/point_batch_cases.py", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT}: güncel")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    data = json.loads(text)
    print(f"{OUT}: {len(data['cases'])} işlem, {len(data['targets'])} hedef durumu")


if __name__ == "__main__":
    main()
