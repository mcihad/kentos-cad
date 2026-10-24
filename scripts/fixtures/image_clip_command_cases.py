#!/usr/bin/env python3
"""The shared cases of `cad.images.clip` (Resmi kırp; docs/adr/0192 §5, 0622 §9).

    python3 scripts/fixtures/image_clip_command_cases.py           # writes the file
    python3 scripts/fixtures/image_clip_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.images.clip.json with Python's standard library, no KentOS code, from ADR 0192 §5 and
the command's contract:

- the boundary: a rectangle of two corners along the drawing's axes (a, (b.x, a.y), b, (a.x, b.y)) or a polygon of at
  least three corners, its corners in the picture's own fractions and cut to the unit square as
  `scripts/fixtures/image_cases.py` cuts them (counter-clockwise, from the lowest corner); a boundary leaving nothing
  with an area is `clip_outside`, with the core's words;
- `remove` shows the whole picture again; a picture already showing what is asked is not written (`changed` false);
- the refusals in the contract's order: `not_finite`, `too_few_points`, `invalid_uid`, `entity_not_found`,
  `not_an_image`, `layer_locked`, `clip_outside`, `invalid_revision`, `revision_conflict`.

An unturned picture's fractions are exact here (the frames are chosen so that the core's floats are too); the turned
picture's are compared within 1e-12.
"""
import json
import math
import os
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.images.clip.json"
sys.path.insert(0, str(Path(__file__).resolve().parent))
os.environ["KENTOS_NO_MAIN"] = "1"
from image_cases import CLIP_OUTSIDE, cut, fractions, turned_fractions  # noqa: E402  (ADR 0192 §5's rule)

SETTINGS = {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 50,
            "workspace": "cad"}
TURN = math.pi / 2


def layer(i, name, locked=False, visible=True):
    return {"id": i, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


LAYERS = [layer("cizim", "Çizim"), layer("kilitli", "Kilitli", locked=True), layer("gizli", "Gizli", visible=False)]


def pt(x, y):
    return {"x": float(x), "y": float(y)}


def image(i, p, rotation=0.0, mirror=False, clip=None, layer_id="cizim"):
    e = {"kind": "image", "id": i, "layerId": layer_id, "attrs": {}, "p": pt(*p), "width": 40.0, "height": 20.0,
         "rotation": rotation, "file": "kroki.png"}
    if mirror:
        e["mirror"] = True
    if clip is not None:
        e["clip"] = [pt(*q) for q in clip]
    return e


# The pictures: (frame, rotation) as image_cases.py takes them.
FRAMES = {
    1: (((100.0, 200.0), 40.0, 20.0, False), 0.0),
    2: (((200.0, 200.0), 40.0, 20.0, True), 0.0),
    3: (((300.0, 200.0), 40.0, 20.0, False), TURN),
    4: (((400.0, 200.0), 40.0, 20.0, False), 0.0),
    7: (((600.0, 200.0), 40.0, 20.0, False), 0.0),
}
HALF = [(0, 0), (0.5, 0), (0.5, 0.5), (0, 0.5)]
ENTITIES = [
    image(1, (100, 200)),
    image(2, (200, 200), mirror=True),
    image(3, (300, 200), rotation=TURN),
    image(4, (400, 200), clip=HALF),
    image(5, (500, 200), layer_id="kilitli"),
    {"kind": "polyline", "id": 6, "layerId": "cizim", "attrs": {}, "pts": [pt(0, 0), pt(10, 0)]},
    image(7, (600, 200), layer_id="gizli"),
]
IDS = [e["id"] for e in ENTITIES]
SETUP = {"format": "kentos.document", "version": 1, "name": "Resmi kırp", "settings": SETTINGS,
         "origin": {"x": 0.0, "y": 0.0}, "layers": LAYERS, "activeLayer": "cizim",
         "styles": {"items": [], "categories": []}, "entities": ENTITIES}

UNCHANGED = {"ids": IDS, "dirty": False, "revision": "same"}


def ring_of(clip):
    if clip["kind"] == "rectangle":
        a, b = clip["a"], clip["b"]
        return [(a["x"], a["y"]), (b["x"], a["y"]), (b["x"], b["y"]), (a["x"], b["y"])]
    return [(q["x"], q["y"]) for q in clip["points"]]


def clip_exact(i, clip):
    """The clip in the picture's fractions as fractions, or None when nothing with an area is left."""
    frame, rotation = FRAMES[i]
    world = ring_of(clip)
    if rotation == 0.0:
        ring = [fractions(frame, q) for q in world]
    else:
        ring = [tuple(F(v) for v in turned_fractions(frame, rotation, q)) for q in world]
    return cut(ring)


def clip_of(i, clip):
    got = clip_exact(i, clip)
    return None if got is None else [pt(x, y) for x, y in got]


def dyadic(v):
    """A fraction a float holds exactly (its denominator a power of two)."""
    d = F(v).denominator
    return d & (d - 1) == 0


def rect(a, b):
    return {"kind": "rectangle", "a": pt(*a), "b": pt(*b)}


def poly(*points):
    return {"kind": "polygon", "points": [pt(*q) for q in points]}


def refused(code, message, path, status="failed"):
    return {"status": status, "error": {"code": code, "message": message, "path": path}}


def clipped(name, i, clip, note=None):
    """A clip that writes: the output, the picture's new clip, one undo step."""
    got = clip_of(i, clip)
    assert got is not None, name
    turned = FRAMES[i][1] != 0.0
    # The picture's new clip is compared as written when its fractions are floats exactly; the output always within.
    exact = not turned and all(dyadic(v) for q in clip_exact(i, clip) for v in q)
    step = {"op": "execute", "input": {"uid": f"$uidOf:{i}", "clip": clip}, "within": 1e-12}
    step["result"] = {"status": "completed",
                      "output": {"uid": f"$uidOf:{i}", "clip": got, "changed": True, "revision": "$current"},
                      "warnings": []}
    expect = {"ids": IDS, "canUndo": True, "dirty": True, "revision": "changed"}
    if exact:
        expect["fields"] = {str(i): {"clip": got}}
    step["expect"] = expect
    case = {"name": name, "steps": [step, {"op": "undo", "returns": "Resmi kırp",
                                           "expect": {"ids": IDS, "canUndo": False, "canRedo": True}}]}
    if note:
        case["note"] = note
    return case


def outside(name, i, clip):
    assert clip_of(i, clip) is None, name
    return {"name": name, "steps": [{"op": "execute", "input": {"uid": f"$uidOf:{i}", "clip": clip},
                                     "result": refused("clip_outside", CLIP_OUTSIDE, "clip"), "expect": UNCHANGED}]}


def refusal(name, input, result, non_finite=None):
    step = {"op": "execute", "input": input}
    if non_finite:
        step["nonFinite"] = non_finite
    step["result"] = result
    step["expect"] = UNCHANGED
    return {"name": name, "steps": [step]}


def not_finite(whose, axis, path):
    name = "doğu (Y)" if axis == "x" else "kuzey (X)"
    return refused("not_finite",
                   f"{whose} {name} değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.",
                   path)


UID = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4099"


def build_cases():
    cases = [
        clipped("Dikdörtgen: resmin ortası", 1, rect((110, 205), (130, 215))),
        clipped("Dikdörtgen ters köşelerle: aynı kırpma", 1, rect((130, 215), (110, 205)),
                note="Köşelerin sırası sınırın dönüşünü değiştirir; kırpma saat yönünün tersine ve en alttaki köşeden."),
        clipped("Çokgen saat yönünün tersine", 1, poly((110, 205), (130, 205), (130, 215), (110, 215))),
        clipped("Çokgen saat yönünde verilir, ters döner", 1, poly((110, 205), (110, 215), (130, 215), (130, 205))),
        clipped("Taşan sınır resmin kenarlarında kesilir", 1, rect((90, 190), (120, 230))),
        clipped("Resmi saran sınır bütün resimdir", 1, rect((0, 0), (500, 500))),
        clipped("Köşeyi kesen üçgen", 1, poly((130, 195), (150, 195), (150, 225))),
        clipped("En alttaki köşeden: eşit alttakilerin en solu", 1, poly((130, 205), (110, 215), (110, 205))),
        clipped("Aynalı resim: kesir yukarı ters", 2, rect((200, 200), (240, 205))),
        clipped("Dönük resim", 3, rect((295, 205), (305, 230))),
        clipped("Kırpılmış resim yeniden kırpılır", 4, rect((410, 200), (440, 220))),
        clipped("Gizli katmandaki resim kırpılır", 7, rect((600, 200), (620, 210))),
        outside("Resmin dışında: söylenir", 1, poly((300, 300), (310, 300), (310, 310))),
        outside("Yalnız kenara değen: alansız, söylenir", 1, rect((140, 205), (150, 210))),
        outside("Dönük resmin dışında", 3, rect((310, 205), (330, 230))),
        {"name": "Kaldır: kırpılmış resim bütün görünür",
         "steps": [{"op": "execute", "input": {"uid": "$uidOf:4", "clip": {"kind": "remove"}},
                    "result": {"status": "completed",
                               "output": {"uid": "$uidOf:4", "changed": True, "revision": "$current"},
                               "warnings": []},
                    "expect": {"ids": IDS, "canUndo": True, "dirty": True, "revision": "changed",
                               "fields": {"4": {"clip": None}}}},
                   {"op": "undo", "returns": "Resmi kırp",
                    "expect": {"ids": IDS, "canUndo": False, "canRedo": True,
                               "fields": {"4": {"clip": [pt(*q) for q in HALF]}}}}]},
        {"name": "Kaldır: kırpılmamış resimde hiçbir şey yazılmaz",
         "steps": [{"op": "execute", "input": {"uid": "$uidOf:1", "clip": {"kind": "remove"}},
                    "result": {"status": "completed",
                               "output": {"uid": "$uidOf:1", "changed": False, "revision": "$current"},
                               "warnings": []},
                    "expect": {"ids": IDS, "canUndo": False, "dirty": False, "revision": "same"}}]},
        {"name": "Resmin zaten gösterdiği kırpma: hiçbir şey yazılmaz",
         "steps": [{"op": "execute", "input": {"uid": "$uidOf:4", "clip": rect((400, 200), (420, 210))},
                    "result": {"status": "completed",
                               "output": {"uid": "$uidOf:4", "clip": [pt(*q) for q in HALF], "changed": False,
                                          "revision": "$current"},
                               "warnings": []},
                    "expect": {"ids": IDS, "canUndo": False, "dirty": False, "revision": "same"}}]},
        {"name": "Plan: yazılmaz",
         "steps": [{"op": "plan", "input": {"uid": "$uidOf:1", "clip": rect((110, 205), (130, 215))},
                    "result": {"status": "completed", "output": "$any", "warnings": []}, "expect": UNCHANGED}]},
        {"name": "Doğrulama: yazılmaz",
         "steps": [{"op": "validate", "input": {"uid": "$uidOf:1", "clip": rect((110, 205), (130, 215))},
                    "result": {"status": "completed", "output": None, "warnings": []}, "expect": UNCHANGED}]},
        refusal("Dikdörtgenin köşesi sonlu değil", {"uid": "$uidOf:1", "clip": rect((110, 205), (130, 215))},
                not_finite("Dikdörtgenin köşesi", "y", "clip.b.y"), {"clip.b.y": "NaN"}),
        refusal("Çokgenin köşesi sonlu değil", {"uid": "$uidOf:1", "clip": poly((110, 205), (130, 205), (130, 215))},
                not_finite("Çokgenin köşesi", "x", "clip.points[1].x"), {"clip.points[1].x": "Infinity"}),
        refusal("Çokgenin iki köşesi: az", {"uid": "$uidOf:1", "clip": poly((110, 205), (130, 215))},
                refused("too_few_points", "Kırpma çokgeninin en az üç köşesi olmalı; 2 köşe verildi.", "clip.points")),
        refusal("Kimlik UUID değil", {"uid": "resim-1", "clip": {"kind": "remove"}},
                refused("invalid_uid", "“resim-1” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir "
                        "(01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi).", "uid")),
        refusal("Kimlik çizimde yok", {"uid": UID, "clip": {"kind": "remove"}},
                refused("entity_not_found", f"“{UID}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. "
                        "Var olan bir resmin kimliğini verin.", "uid")),
        refusal("Resim değil", {"uid": "$uidOf:6", "clip": rect((0, -1), (5, 1))},
                refused("not_an_image", "Verilen nesne bir resim değil; yalnız resim kırpılır. Bir resmin kimliğini verin.",
                        "uid")),
        refusal("Kilitli katmandaki resim", {"uid": "$uidOf:5", "clip": rect((500, 200), (520, 210))},
                refused("layer_locked", "“Kilitli” katmanı kilitli; üzerindeki resim kırpılamaz. Kilidi Katmanlar "
                        "panelinden açın.", "uid")),
        refusal("Beklenen sürüm geçersiz", {"uid": "$uidOf:1", "clip": rect((110, 205), (130, 215)),
                                            "expectedRevision": "dün"},
                refused("invalid_revision", "Beklenen sürüm “dün” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı "
                        "yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision")),
        {"name": "Beklenen sürüm eski: çakışma",
         "steps": [{"op": "execute", "input": {"uid": "$uidOf:1", "clip": rect((110, 205), (130, 215)),
                                              "expectedRevision": "999"},
                    "result": {"status": "conflict",
                               "error": {"code": "revision_conflict",
                                         "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. "
                                                    "Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
                                         "path": "expectedRevision", "revision": "$current"}},
                    "expect": UNCHANGED}]},
    ]
    return cases


def compact(v):
    return json.dumps(v, ensure_ascii=False, separators=(", ", ": "))


def build(cases):
    out = ["{", '  "format": "kentos.command-cases",', '  "version": 1,', '  "command": "cad.images.clip",',
           '  "commandVersion": 1,', f'  "title": {compact("Resmi kırp")},',
           f'  "note": {compact("Written by scripts/fixtures/image_clip_command_cases.py from docs/adr/0192 §5 and 0622 §9; do not edit by hand.")},',
           f'  "setup": {compact(SETUP)},', '  "cases": [']
    blocks = []
    for c in cases:
        lines = ["    {", f'      "name": {compact(c["name"])},']
        if "note" in c:
            lines.append(f'      "note": {compact(c["note"])},')
        lines.append('      "steps": [')
        lines.append(",\n".join(f"        {compact(s)}" for s in c["steps"]))
        lines.append("      ]")
        lines.append("    }")
        blocks.append("\n".join(lines))
    out.append(",\n".join(blocks))
    out.append("  ]")
    out.append("}")
    return "\n".join(out) + "\n"


def main():
    cases = build_cases()
    text = build(cases)
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"{OUT.relative_to(ROOT)} güncel değil")
        print(f"{len(cases)} durum tutarlı")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{len(cases)} durum yazıldı")


if __name__ == "__main__":
    main()
