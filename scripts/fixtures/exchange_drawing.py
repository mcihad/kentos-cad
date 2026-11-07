#!/usr/bin/env python3
"""The drawing of the exchange round-trip check (`file.exchange.check`, docs/adr/1660; plan/map-cad.md MAP-17).

A small survey drawing at real coordinates (TUREF / TM36, EPSG:5256): spot heights with Z, a route polyline with Z
and M, a road edge with an arc (bulge), a manhole circle, a parcel with a hole, a text, attributes (text and number)
and one object's own colour. Written as a KentOS document (v2, persistent ids) to
fixtures/exchange/v1/devir.json.

    python3 scripts/fixtures/exchange_drawing.py [--check]
"""

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "exchange" / "v1" / "devir.json"
X0, Y0 = 487000.0, 4420000.0


def uid(n):
    return f"0192f660-1660-7000-8000-{n:012x}"


def P(x, y):
    return {"x": X0 + x, "y": Y0 + y}


def layer(id_, name):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


ENTITIES = [
    {"kind": "point", "layerId": "nokta", "attrs": {"Ad": "N1", "Kod": "KOT"}, "p": P(10.0, 10.0), "z": 101.25},
    {"kind": "point", "layerId": "nokta", "attrs": {"Ad": "N2", "Kod": "KOT"}, "p": P(40.0, 12.5), "z": 99.875},
    {"kind": "point", "layerId": "nokta", "attrs": {"Ad": "N3", "Kod": "KOT"}, "p": P(70.0, 30.0), "z": 98.5},
    {"kind": "polyline", "layerId": "guzergah", "attrs": {"Ad": "Hat 1", "Uzunluk": "120.5"},
     "pts": [P(0.0, 0.0), P(50.0, 0.0), P(100.0, 20.0)], "zs": [100.0, 99.5, 98.25], "ms": [0.0, 50.0, 103.85]},
    {"kind": "polyline", "layerId": "yol", "attrs": {"Ad": "Kenar"}, "color": "#E5484D",
     "pts": [P(0.0, 50.0), P(40.0, 50.0), P(60.0, 70.0)], "bulges": [0.0, 0.4142135623730951, 0.0]},
    {"kind": "circle", "layerId": "baca", "attrs": {"Ad": "B1"}, "c": P(80.0, 80.0), "r": 0.6},
    {"kind": "polygon", "layerId": "parsel", "attrs": {"Ada": "101", "Parsel": "7", "Alan": "1875.00"},
     "pts": [P(100.0, 0.0), P(150.0, 0.0), P(150.0, 40.0), P(100.0, 40.0)],
     "holes": [{"pts": [P(120.0, 10.0), P(130.0, 10.0), P(130.0, 20.0), P(120.0, 20.0)]}]},
    {"kind": "text", "layerId": "yazi", "attrs": {}, "p": P(105.0, 45.0), "text": "101/7", "height": 2.0,
     "rotation": 0.0},
]


def document():
    layers = [layer("0", "0"), layer("nokta", "Nokta"), layer("guzergah", "Güzergah"), layer("yol", "Yol"),
              layer("baca", "Baca"), layer("parsel", "Parsel"), layer("yazi", "Yazı")]
    ents = []
    for n, e in enumerate(ENTITIES, 1):
        ents.append({**e, "id": n})
    return {"format": "kentos.document", "version": 2, "name": "Devir", "settings": {
        "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
        "plotScale": 1000.0, "workspace": "cad", "cadWork": "map", "drawingFont": "barlow"},
        "origin": {"x": X0, "y": Y0}, "layers": layers, "activeLayer": "0", "entities": ents,
        "uids": [uid(n) for n in range(1, len(ents) + 1)], "styles": {"items": [], "categories": []}}


def main():
    text = json.dumps(document(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok = OUT.exists() and OUT.read_text("utf-8") == text
        print("Devir çizimi güncel." if ok else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
