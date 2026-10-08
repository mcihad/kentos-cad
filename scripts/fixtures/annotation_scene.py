#!/usr/bin/env python3
"""The scene both platforms' pictures of docs/adr/0205 open: fixtures/interaction/v1/annotations.kcad.

A CAD project at 1:1000 (texts 2.5 mm on paper are 2.5 m in the drawing): a parcel with its number, its edges'
dimensions (one plain, one following the style “Renkli çizgiler” with its lines' colours, weights and types, one
with its own dashed red dimension line), and a row of leaders, one for each of AutoCAD's arrowheads KentOS has
(the filled triangle first, none last), two with an arrowhead of their own size. `--check` says whether the file
holds what this script writes.
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FILE = ROOT / "fixtures" / "interaction" / "v1" / "annotations.kcad"

X0, Y0 = 500000.0, 4400000.0
LINES = {"dimLineColor": "#E5484D", "dimLineWeight": 0.35, "dimLineType": "dashed", "extColor": "#4F8EF7",
         "extWeight": 0.18, "extLineType": "dotted", "textColor": "#5FBF77"}
STYLE = "0192f6a0-0000-7000-8000-000000000203"
ARROWS = [(None, "Dolu üçgen"), ("closed", "Boş üçgen"), ("open", "Açık ok"), ("open30", "İnce açık ok"),
          ("open90", "Dik açık ok"), ("dot", "Dolu nokta"), ("dotSmall", "Küçük nokta"), ("dotBlank", "Boş nokta"),
          ("oblique", "Eğik çizgi"), ("archTick", "Mimari çentik"), ("boxFilled", "Dolu kare"), ("boxBlank", "Boş kare"),
          ("datumFilled", "Dayanak üçgeni"), ("none", "Yok")]


def p(x, y):
    return {"x": X0 + x, "y": Y0 + y}


def layer(id_, name, color, weight=0.25):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": color, "lineType": "continuous", "lineWeight": weight}, "children": []}


def document():
    entities = []

    def add(e):
        e = {"id": len(entities) + 1, "attrs": {}, **e}
        entities.append(e)

    parcel = [p(0, 0), p(60, 0), p(64, 40), p(2, 38)]
    add({"kind": "polygon", "layerId": "parsel", "pts": parcel, "label": "101/5"})
    add({"kind": "text", "layerId": "yazi", "p": p(16, 10), "text": "Ada 101 Parsel 5", "height": 2.5, "rotation": 0})
    # The south edge plain, the west one following the style, the north one with its own red dashed line.
    add({"kind": "dimension", "layerId": "olcu", "a": parcel[0], "b": parcel[1], "offset": -6.0, "height": 2.5})
    add({"kind": "dimension", "layerId": "olcu", "a": parcel[3], "b": parcel[0], "offset": -6.0, "height": 2.5,
         "dimStyle": STYLE, "arrow": "closed", **LINES})
    add({"kind": "dimension", "layerId": "olcu", "a": parcel[2], "b": parcel[3], "offset": -6.0, "height": 2.5,
         "dimLineColor": "#E5484D", "dimLineType": "dashed", "dimLineWeight": 0.5})
    # The leaders, two rows of seven over the parcel, each pointing down-left at a mark, far enough apart for their notes.
    for i, (arrow, name) in enumerate(ARROWS):
        col, row = i % 7, i // 7
        tip = p(-40 + col * 26.0, 58 + row * 16.0)
        e = {"kind": "leader", "layerId": "kilavuz", "pts": [tip, {"x": tip["x"] + 4.0, "y": tip["y"] + 5.0}],
             "text": name, "height": 2.5, "rotation": 0}
        if arrow:
            e["arrow"] = arrow
        if arrow in ("open30", "boxBlank"):
            e["arrowSize"] = 1.5
        add(e)
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Açıklamalar: ok uçları, ölçü çizgileri ve yazılar (ADR 0205)",
        "settings": {
            "srid": 5256, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
            "plotScale": 1000.0, "workspace": "cad",
            "dimensionStyles": [{"id": STYLE, "name": "Renkli çizgiler", "height": 2.5, "arrow": "closed", **LINES}],
        },
        "origin": {"x": X0, "y": Y0},
        "layers": [layer("0", "0", "fg"), layer("parsel", "Parsel", "fg", 0.35), layer("yazi", "Yazı", "fg"),
                   layer("olcu", "Ölçü", "#E5A50A", 0.18), layer("kilavuz", "Kılavuz", "#3A7BD5", 0.18)],
        "activeLayer": "kilavuz",
        "entities": entities,
        "styles": {"items": [], "categories": []},
        "blocks": [],
    }


def main() -> int:
    text = json.dumps(document(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv[1:]:
        if FILE.read_text(encoding="utf-8") != text:
            print(f"{FILE.relative_to(ROOT)} is not what this script writes: run it without --check")
            return 1
        print(f"{FILE.relative_to(ROOT)}: güncel")
        return 0
    FILE.write_text(text, encoding="utf-8")
    print(f"yazıldı: {FILE.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
