#!/usr/bin/env python3
"""The drawing the CBS-16 derivation cases are made from (docs/adr/0530): an arced parcel with a hole and Z, a straight
one, a record, a well, a road with an arc, a polyline with bulges, Z and M, a closed polyline, ellipses, a fit-point curve,
a NURBS quarter circle, a point, a text, two inserts of a block with attributes, a building footprint and two construction
lines (left out). It is `fixtures/gis-derivation/v1/drawing.json`, a `kentos.document` v1 snapshot every case's `setup`,
the headless tests and `cad_gis_derivation_cases.py` read.

    python3 scripts/fixtures/gis_derivation_drawing.py           # writes the drawing
    python3 scripts/fixtures/gis_derivation_drawing.py --check   # compares
"""
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/gis-derivation/v1/drawing.json"

E, N = 487000.0, 4420000.0
BLOCK = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001"
STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}


def layer(id_, name, fields=None):
    node = {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": STYLE, "children": []}
    if fields:
        node["fields"] = fields
    return node


def pt(x, y):
    return {"x": x, "y": y}


def drawing():
    ents = []

    def add(kind, layer_id, attrs=None, **rest):
        ents.append({"kind": kind, "id": len(ents) + 1, "layerId": layer_id, "attrs": attrs or {}, **rest})

    # The arced parcel: its east edge a quarter-bulge, its hole's last edge bulged, Z at each corner.
    add("polygon", "parsel", {"ada": "101", "parsel_no": "7"},
        pts=[pt(E, N), pt(E + 40, N), pt(E + 40, N + 30), pt(E, N + 30)], bulges=[0, 0.5, 0, 0],
        zs=[10.0, 11.0, 12.0, 13.0],
        holes=[{"pts": [pt(E + 10, N + 10), pt(E + 16, N + 10), pt(E + 16, N + 16), pt(E + 10, N + 16)],
                "bulges": [0, 0, 0, 0.4]}])
    add("polygon", "parsel", {"ada": "101", "parsel_no": "8"},
        pts=[pt(E + 50, N), pt(E + 80, N), pt(E + 80, N + 30), pt(E + 50, N + 30)])
    add("record", "parsel", {"ada": "102"})
    add("circle", "parsel", {"ada": "kuyu"}, c=pt(E + 20, N + 40), r=2.5)
    add("arc", "yol", {"ad": "D-100"}, c=pt(E, N - 50), r=60.0, a0=0.2, a1=1.4)
    add("polyline", "yol", {"ad": "D-200"},
        pts=[pt(E + 100, N - 50), pt(E + 120, N - 50), pt(E + 140, N - 40), pt(E + 160, N - 40)],
        bulges=[0.3, 0, -0.4, 0], zs=[100.0, 101.0, 102.0, 103.0], ms=[0.0, 100.0, 200.0, 300.0])
    add("line", "yol", {"ad": "D-300"}, a=pt(E + 100, N - 70), b=pt(E + 120, N - 60), za=5.0, zb=6.0)
    add("polyline", "yol", {"ad": "D-400"},
        pts=[pt(E + 200, N - 60), pt(E + 220, N - 60), pt(E + 220, N - 40)], bulges=[0.2, 0, 0], closed=True)
    add("ellipse", "cizim", {"ad": "gölet"}, c=pt(E + 200, N), major=pt(12.0, 0.0), ratio=0.5, t0=0, t1=2 * math.pi)
    add("ellipse", "cizim", {"ad": "yay"}, c=pt(E + 240, N), major=pt(10.0, 5.0), ratio=0.4, t0=0.5, t1=2.5)
    add("spline", "cizim", {"ad": "dere"},
        pts=[pt(E + 260, N), pt(E + 265, N + 4), pt(E + 270, N), pt(E + 275, N + 4), pt(E + 280, N)], closed=False)
    r = 8.0
    add("nurbs", "cizim", {"ad": "kıvrım"}, degree=2,
        ctrl=[pt(E + 300 + r, N), pt(E + 300 + r, N + r), pt(E + 300, N + r)], knots=[0, 0, 0, 1, 1, 1],
        weights=[1.0, math.sqrt(0.5), 1.0])
    add("point", "cizim", {"ad": "P-1"}, p=pt(E + 320, N), z=7.5)
    add("text", "yazi", {}, p=pt(E + 5, N + 35), text="Ada 101", height=2.0, rotation=15.0)
    add("insert", "bina", {"NO": "B-1", "KAT": "3"}, block=BLOCK, p=pt(E + 5, N + 20), scale=1, rotation=0)
    add("insert", "bina", {"NO": "B-2"}, block=BLOCK, p=pt(E + 55, N + 5), scale=1, rotation=0.5)
    add("polygon", "bina", {}, pts=[pt(E + 60, N + 15), pt(E + 70, N + 15), pt(E + 70, N + 25), pt(E + 60, N + 25)])
    add("xline", "cizim", {}, p=pt(E, N), dir=pt(1.0, 0.0))
    add("ray", "cizim", {}, p=pt(E, N), dir=pt(0.0, 1.0))
    block = {
        "id": BLOCK, "name": "Bina", "base": pt(0, 0),
        "entities": [{"kind": "polygon", "id": 1, "layerId": "0", "attrs": {},
                      "pts": [pt(0, 0), pt(10, 0), pt(10, 8), pt(0, 8)]}],
        "attributes": [
            {"tag": "NO", "prompt": "Bina no", "value": "B-?", "p": pt(1, 1), "height": 0.5, "rotation": 0},
            {"tag": "KAT", "prompt": "Kat", "value": "1", "p": pt(1, 2), "height": 0.5, "rotation": 0},
        ],
    }
    return {
        "format": "kentos.document", "version": 1, "name": "Ada 101 türevi",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
        "origin": pt(E, N),
        "layers": [
            layer("parsel", "Parsel", [{"name": "ada", "kind": "text"}, {"name": "parsel_no", "kind": "integer"}]),
            layer("bina", "Bina"),
            layer("yol", "Yol", [{"name": "ad", "kind": "text"}]),
            layer("yazi", "Yazı"),
            layer("cizim", "Çizim"),
        ],
        "activeLayer": "parsel",
        "entities": ents,
        "styles": {"items": [], "categories": []},
        "blocks": [block],
    }


def text():
    return json.dumps(drawing(), ensure_ascii=False, indent=1) + "\n"


def main():
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text():
            sys.exit(f"{OUT} is not what the script writes; run it without --check and read the difference")
        print("drawing ok")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text())
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
