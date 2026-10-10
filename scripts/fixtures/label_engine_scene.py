#!/usr/bin/env python3
"""The scene of the label engine's pictures and traces (docs/adr/0212): fixtures/interaction/v1/label-engine.kcad.

A GIS project near (487000, 4420000), places relative to it, about 300 × 190 m:
- Mahalle: a quarter's outline, its name repeated inside along it (Sınır).
- Ada: two blocks, their numbers large and level (Yatay, büyüyen).
- Parsel: rule-based classes: the number (Parsel kipi: level, else along its long side, else stacked, smaller, outside
  with a callout) and the owner's name (when there is one), shortened from a dictionary when it does not fit; a thin
  sliver whose number goes outside; one number pinned by hand, turned.
- Bina: buildings, no labels of their own, obstacles to every label (weight 7).
- Yol: road centre lines, a street's name in three pieces meeting end to end (merged, curved), a crossing street.
- Eşyükselti: contours with heights, their labels on the line, their tops uphill (Eş yükselti).
- Nokta: survey points close together, their names around them; one hidden by hand.
- Yazı: the sheet's title, a text the labels keep off.

`--check` says whether the file holds what this script writes.
"""
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FILE = ROOT / "fixtures" / "interaction" / "v1" / "label-engine.kcad"

X0, Y0 = 487000.0, 4420000.0


def p(x, y):
    return {"x": X0 + x, "y": Y0 + y}


def rect(x0, y0, x1, y1):
    return [p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)]


def layer(id_, name, color, weight=0.25, **style):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": color, "lineType": "continuous", "lineWeight": weight, **style}, "children": []}


# The parcels' classes (docs/adr/0212 §2): the number as a land parcel's, the owner's name shortened when needed.
PARSEL = {
    "mode": "rules",
    "classes": [
        {"name": "No", "style": {"placement": "center", "size": 11.0, "weight": 600, "text": "Parsel", "area": "parcel",
                                 "stack": {"mode": "ifNeeded", "chars": 6, "at": " /"}, "shrink": 0.8, "outside": True,
                                 "callout": {"kind": "straight", "color": "fg-dim", "width": 1.0}, "priority": 7}},
        {"name": "Malik", "when": "Malik <> ''", "style": {"placement": "center", "size": 9.0, "italic": True, "text": "Malik",
                                                           "area": "free", "inside": True, "color": "fg-dim", "priority": 3,
                                                           "abbreviate": {"words": [{"word": "Arazisi", "short": "Ar."},
                                                                                    {"word": "Belediyesi", "short": "Bld."}]}}},
    ],
}


def document():
    entities = []

    def add(e):
        entities.append({"id": len(entities) + 1, "attrs": {}, **e})

    # Mahalle: its outline, its name inside along it.
    add({"kind": "polygon", "layerId": "mahalle", "label": "Cumhuriyet Mahallesi",
         "pts": [p(-10, -10), p(290, -10), p(290, 175), p(-10, 175)]})
    # Ada: two blocks.
    add({"kind": "polygon", "layerId": "ada", "label": "1244 ada", "pts": rect(10, 10, 130, 80)})
    add({"kind": "polygon", "layerId": "ada", "label": "1245 ada", "pts": rect(150, 10, 270, 80)})
    # Parsel: three a row, two rows a block, a sliver among them.
    owners = {3: "Hazine Arazisi", 5: "Ayşe Demir", 8: "Çankaya Belediyesi", 10: "Ali Kaya"}
    n = 0
    for bx in (10, 150):
        for row in range(2):
            for col in range(3):
                n += 1
                x0, y0 = bx + col * 40, 10 + row * 35
                attrs = {"Parsel": str(n)}
                if n in owners:
                    attrs["Malik"] = owners[n]
                e = {"kind": "polygon", "layerId": "parsel", "attrs": attrs, "pts": rect(x0, y0, x0 + 40, y0 + 35)}
                # Parcel 4's number pinned by hand: moved up and turned (docs/adr/0212 §3.7).
                if n == 4:
                    e["labelPins"] = [{"class": "No", "at": {"x": -8.0, "y": 6.0}, "rotation": 20.0}]
                entities.append({"id": len(entities) + 1, **e})
    # A sliver: too thin for its number inside.
    add({"kind": "polygon", "layerId": "parsel", "attrs": {"Parsel": "13/1"}, "pts": rect(130, 10, 133, 80)})
    # Bina: a building in some parcels, obstacles to the labels.
    for x, y in ((18, 16), (62, 52), (160, 18), (205, 50), (243, 15)):
        add({"kind": "polygon", "layerId": "bina", "pts": rect(x, y, x + 14, y + 11)})
    # Yol: a street in three pieces end to end (merged), a crossing street.
    add({"kind": "polyline", "layerId": "yol", "label": "Atatürk Caddesi", "pts": [p(-10, 92), p(60, 95)]})
    add({"kind": "polyline", "layerId": "yol", "label": "Atatürk Caddesi", "pts": [p(60, 95), p(140, 92), p(180, 96)]})
    add({"kind": "polyline", "layerId": "yol", "label": "Atatürk Caddesi", "pts": [p(180, 96), p(240, 102), p(290, 99)]})
    add({"kind": "polyline", "layerId": "yol", "label": "Gül Sokak", "pts": [p(141, -10), p(141, 45), p(139, 92)]})
    # Eşyükselti: wavy contours rising to the north, a height each.
    for k in range(6):
        z = 1240.0 + 5.0 * k
        pts = [p(x, 110 + 11 * k + 4.0 * math.sin(x / 23.0 + k * 0.4)) for x in range(-10, 291, 15)]
        add({"kind": "polyline", "layerId": "esyukselti", "label": f"{z:g}", "pts": pts, "zs": [z] * len(pts)})
    # Nokta: survey points along the blocks' corners, close together.
    names = 101
    for x, y in ((10, 10), (50, 10), (90, 10), (130, 10), (10, 45), (50, 45), (90, 45), (130, 45), (10, 80), (50, 80), (90, 80),
                 (130, 80), (150, 10), (190, 10), (230, 10), (270, 10), (150, 80), (190, 80), (230, 80), (270, 80), (133, 12), (133, 78)):
        e = {"kind": "point", "layerId": "nokta", "label": f"P.{names}", "p": p(x, y)}
        if names == 105:
            e["labelPins"] = [{"hidden": True}]
        entities.append({"id": len(entities) + 1, "attrs": {}, **e})
        names += 1
    # Yazı: the sheet's title.
    add({"kind": "text", "layerId": "yazi", "p": p(195, 150), "text": "Pafta P-12", "height": 4.0, "rotation": 0.0})
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Etiket motoru",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": X0, "y": Y0},
        "layers": [
            layer("nokta", "Nokta", "#E5C07B", point={"symbol": "cross", "size": 7.0},
                  label={"placement": "beside", "size": 10.0, "point": "around", "distance": 2.0, "priority": 6}),
            layer("yazi", "Yazı", "fg"),
            layer("yol", "Yol", "#8C9AAA", 0.5,
                  label={"placement": "along", "size": 11.0, "weight": 500, "line": "curved", "maxAngle": 30.0,
                         "mergeLines": True, "repeat": 600.0, "duplicates": 250.0, "priority": 6}),
            layer("bina", "Bina", "#F76B15", labels={"mode": "off", "obstacle": {"weight": 7}}),
            layer("parsel", "Parsel", "#4F8EF7", 0.35, labels=PARSEL),
            layer("ada", "Ada", "#3E63DD", 0.5,
                  label={"placement": "center", "size": 13.0, "grow": 1.0, "maxSize": 20.0, "weight": 600, "area": "horizontal",
                         "priority": 9, "halo": {"width": 2.0}}),
            layer("esyukselti", "Eşyükselti", "#C9955F",
                  label={"placement": "along", "size": 9.0, "line": "contour", "repeat": 300.0, "priority": 4}),
            layer("mahalle", "Mahalle", "#8E4EC6", 0.5,
                  label={"placement": "along", "size": 10.0, "weight": 600, "area": "boundary", "repeat": 520.0,
                         "color": "#8E4EC6", "priority": 5}),
        ],
        "activeLayer": "parsel",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }


def main() -> int:
    text = json.dumps(document(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv[1:]:
        if not FILE.exists() or FILE.read_text(encoding="utf-8") != text:
            print(f"{FILE.relative_to(ROOT)} is not what this script writes: run it without --check")
            return 1
        print(f"{FILE.relative_to(ROOT)}: güncel")
        return 0
    FILE.write_text(text, encoding="utf-8")
    print(f"yazıldı: {FILE.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
