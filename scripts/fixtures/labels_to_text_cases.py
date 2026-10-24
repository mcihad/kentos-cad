"""Etiketleri yazıya çevir's shared command cases: `cad.annotations.write`'s `labels` work (docs/adr/0622 §9, 0175,
0212 §4), without KentOS code. Imported by annotations_write_cases.py.

The labels are placed by the label engine's independent reference (`label_engine_cases.py`, through
`labels_report_cases.py`'s drawing and its translation into an engine case): the window the asked objects' bounds, at
1:N (96/0.0254/N pixels a metre). Each placed label of an asked object becomes, by docs/adr/0212 §4's rule:

- one line: a text centred where the engine put its line (middle centre), its size over the pixels a metre its height,
  its turn the line's;
- more lines: a multi-line text whose lines are the label's, 0.72 apart (1.2 sizes in 5/3 of the height), aligned as
  the first line stands in the block (left when it starts at the block's left, right at its right, else centre), at
  the block's middle moved half the block that way;
- curved: a text along the curve through its letters' middles, from half the first letter's advance before it to half
  the last one's after, middle left;
- a callout: a line from by the label to its object, after the texts.

A label with no free place is left and counted unless every label is asked for. The texts and lines go to the asked
layer; the standard text layer `yazi` is opened in the same step when the drawing lacks it. With Nesneye bağlı each
text knows its object (`labelOf`) and the scale (`labelScale`); with Zemin it is masked.
"""
import copy
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import label_engine_cases as engine  # noqa: E402
import labels_report_cases as report  # noqa: E402

PX_PER_M = 96.0 / 0.0254
LINE_SPACING = 1.2 / (5.0 / 3.0)
STEP = "Etiketleri yazıya çevir"


def texts_and_lines(d, ids, scale, every):
    """What the asked objects' labels become at 1:`scale`: texts (as fields, with the object they came from) and
    callout lines, and how many labels were left."""
    by_id = {e["id"]: e for e in d["entities"]}
    boxes = [engine.bounds(by_id[i]) for i in ids]
    window = [min(b[0] for b in boxes), min(b[1] for b in boxes), max(b[2] for b in boxes), max(b[3] for b in boxes)]
    k = PX_PER_M / scale
    case = report.engine_case(d, window, k)
    case["options"] = {"unplaced": True, "hidden": False}
    recs = engine.place_window(case)
    texts, lines, left = [], [], 0
    for r in recs:
        if r["id"] not in ids:
            continue
        if r["state"] & engine.HIDDEN:
            continue
        if r["state"] & engine.UNPLACED and not every:
            left += 1
            continue
        if r["state"] & engine.CURVED and r.get("letters"):
            ls = r["letters"]
            size = ls[0][3]

            def end(l, sign):
                a = math.radians(l[2])
                half = sign * l[5] / 2.0 / k
                return (l[0] + half * math.cos(a), l[1] + half * math.sin(a))

            start = end(ls[0], -1.0)
            pts = [{"x": l[0] - start[0], "y": l[1] - start[1]} for l in ls]
            last = end(ls[-1], 1.0)
            pts.append({"x": last[0] - start[0], "y": last[1] - start[1]})
            texts.append((r["id"], {"kind": "text", "text": r["text"], "p": {"x": start[0], "y": start[1]},
                                    "height": size / k, "rotation": 0.0, "align": "middleLeft", "path": {"pts": pts}}))
        elif len(r.get("lines", [])) == 1:
            x, y, angle, size, text, _ = r["lines"][0]
            texts.append((r["id"], {"kind": "text", "text": text, "p": {"x": x, "y": y}, "height": size / k,
                                    "rotation": angle, "align": "middleCenter"}))
        elif len(r.get("lines", [])) > 1:
            mid, angle, w = r["at"], r["angle"], r["w"]
            c, s = math.cos(math.radians(angle)), math.sin(math.radians(angle))
            x, y, _, size, _, width = r["lines"][0]
            along = ((x - mid[0]) * c + (y - mid[1]) * s) * k
            gap = (w - width) / 2.0
            if gap > 1e-6 and abs(along + gap) < 1e-6:
                align, shift = "middleLeft", -w / 2.0
            elif gap > 1e-6 and abs(along - gap) < 1e-6:
                align, shift = "middleRight", w / 2.0
            else:
                align, shift = "middleCenter", 0.0
            texts.append((r["id"], {"kind": "text", "text": "\n".join(l[4] for l in r["lines"]),
                                    "p": {"x": mid[0] + shift / k * c, "y": mid[1] + shift / k * s},
                                    "height": size / k, "rotation": angle, "align": align,
                                    "lineSpacing": LINE_SPACING}))
        if r.get("callout"):
            a, b = r["callout"]
            lines.append({"kind": "line", "a": {"x": a[0], "y": a[1]}, "b": {"x": b[0], "y": b[1]}})
    return texts, lines, left


def cases(done, refused, layer_of_text="yazi"):
    """The cases, written with `done(name, input, objects, count, undo, skipped=, setup=)` and
    `refused(name, input, code, message, path, setup=)`."""
    d = report.doc()
    parcels = [1, 2, 3, 4, 5, 6]
    points = list(range(9, 15))
    crowd = list(range(20, 26))

    def labels_work(ids=None, **extra):
        w = {"kind": "labels"}
        if ids is not None:
            w["ids"] = [f"$uidOf:{i}" for i in ids]
        w.update(extra)
        return w

    def objects_of(ids, scale=1000, every=False, mask=False, linked=False, layer=layer_of_text):
        texts, lines, left = texts_and_lines(d, ids, scale, every)
        out = []
        for oid, t in texts:
            t = copy.deepcopy(t)
            t["layerId"] = layer
            if mask:
                t["mask"] = True
            if linked:
                t["labelOf"] = f"$uidOf:{oid}"
                t["labelScale"] = scale
            out.append(t)
        for ln in lines:
            out.append({**ln, "layerId": layer})
        return out, len(texts), left

    def ok(name, ids, scale=1000, every=False, mask=False, linked=False, layer=layer_of_text, given=True, **extra):
        objects, count, left = objects_of(ids, scale, every, mask, linked, layer)
        work = labels_work(ids if given else None, **extra)
        if scale != 1000:
            work["scale"] = scale
        for flag, on in (("every", every), ("mask", mask), ("linked", linked)):
            if on:
                work[flag] = True
        return done(name, {"layerId": layer, "work": work}, objects, count, STEP, skipped=left, setup=d)

    everyone = [e["id"] for e in d["entities"] if e["layerId"] not in ("bina",)]
    return [
        ok("Etiketleri yazıya çevir: parsellerin numaraları, yazı katmanı aynı adımda açılır", parcels),
        ok("Etiketleri yazıya çevir: seçim yoksa görünen bütün etiketler", everyone, given=False),
        ok("Etiketleri yazıya çevir: 1:500'de", parcels, scale=500),
        ok("Etiketleri yazıya çevir: sıkışık noktaların yerleşmeyenleri sayılır", points),
        ok("Etiketleri yazıya çevir: Örtüşenler de", points, every=True),
        ok("Etiketleri yazıya çevir: Zemin", parcels[:2], mask=True),
        ok("Etiketleri yazıya çevir: Nesneye bağlı", parcels[:2], linked=True),
        ok("Etiketleri yazıya çevir: yol adı yol boyunca", [7]),
        ok("Etiketleri yazıya çevir: kalabalık noktalar", crowd),
        ok("Etiketleri yazıya çevir: etkin katmana", parcels[:3], layer="parsel"),
        refused("Etiketleri yazıya çevir: etiketi olmayan nesne", {"layerId": layer_of_text, "work": labels_work([27])},
                "nothing_derived", "Etiketleri yazıya çevir: verilen nesnelerde yazıya çevrilecek etiket yok.",
                "work.ids", setup=d),
        refused("Etiketleri yazıya çevir: ölçek sıfır", {"layerId": layer_of_text, "work": labels_work(parcels, scale=0)},
                "invalid_scale", "Ölçek 1:N'nin N'si, sıfırdan büyük olmalı.", "work.scale", setup=d),
    ]
