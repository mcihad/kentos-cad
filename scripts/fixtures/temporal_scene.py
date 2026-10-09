#!/usr/bin/env python3
"""The scene of the temporal layers' and scenarios' traces and pictures (docs/adr/0210):
fixtures/interaction/v1/temporal.kcad.

A GIS project near (487000, 4420000), places relative to it:
- Parsel (ranged, `gecerlilik_baslangic` – `gecerlilik_bitis`, key `parsel_no`): 101 from 2010-03-05 until its split on
  2018-06-01 into 101/1 and 101/2 (open); 102 from 2012-01-01 (open); 103 from 2015-07-01 until 2021-01-01; 104 from
  2022-04-01 (open).
- Bina (birikimli, `yapim_tarihi`): buildings built in 2013, 2019 and 2023, shown from then on.
- Arıza (anlık, `tarih`): breakdowns on 2016-04-12 and 2020-09-30.
- Yol: a road, no time.
- Yol genişletme A, a hidden scenario: Yol standing for the road (12 m wide) and Yeni park.

`--check` says whether the file holds what this script writes.
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FILE = ROOT / "fixtures" / "interaction" / "v1" / "temporal.kcad"

X0, Y0 = 487000.0, 4420000.0


def p(x, y):
    return {"x": X0 + x, "y": Y0 + y}


def rect(x0, y0, x1, y1):
    return [p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)]


def layer(id_, name, color, weight=0.25, **extra):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": color, "lineType": "continuous", "lineWeight": weight}, "children": [], **extra}


PARSEL = {"start": "gecerlilik_baslangic", "end": "gecerlilik_bitis", "key": "parsel_no"}


def document():
    entities = []

    def add(e):
        entities.append({"id": len(entities) + 1, "attrs": {}, **e})

    def parcel(no, pts, start, end=None):
        attrs = {"parsel_no": no, "gecerlilik_baslangic": start}
        if end:
            attrs["gecerlilik_bitis"] = end
        add({"kind": "polygon", "layerId": "parsel", "attrs": attrs, "pts": pts, "label": no})

    parcel("101", rect(0, 0, 40, 30), "2010-03-05", "2018-06-01")
    parcel("101/1", rect(0, 0, 20, 30), "2018-06-01")
    parcel("101/2", rect(20, 0, 40, 30), "2018-06-01")
    parcel("102", rect(40, 0, 70, 30), "2012-01-01")
    parcel("103", rect(70, 0, 100, 30), "2015-07-01", "2021-01-01")
    parcel("104", rect(70, 0, 100, 30), "2022-04-01")
    for x, year in ((5, "2013-05-20"), (45, "2019-08-01"), (78, "2023-02-15")):
        add({"kind": "polygon", "layerId": "bina", "attrs": {"yapim_tarihi": year}, "pts": rect(x, 8, x + 10, 20)})
    add({"kind": "point", "layerId": "ariza", "attrs": {"tarih": "2016-04-12"}, "p": p(30, 39)})
    add({"kind": "point", "layerId": "ariza", "attrs": {"tarih": "2020-09-30T14:30:00"}, "p": p(60, 39)})
    add({"kind": "line", "layerId": "yol", "attrs": {"genislik": "8", "acilis_tarihi": "2011-09-01"}, "a": p(-10, 36), "b": p(110, 36)})
    add({"kind": "polygon", "layerId": "alt-a-yol", "attrs": {"genislik": "12"}, "pts": rect(-10, 31, 110, 43)})
    add({"kind": "polygon", "layerId": "alt-a-park", "attrs": {"ad": "Yeni park"}, "pts": rect(70, 46, 100, 60)})
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Etkileşim izi: Zaman ve senaryolar",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": X0, "y": Y0},
        "layers": [
            {"id": "alt-a", "name": "Yol genişletme A", "type": "group", "visible": False, "locked": False,
             "expanded": True, "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.18},
             "children": [layer("alt-a-yol", "Yol", "#E5484D", 0.35, replaces="yol"),
                          layer("alt-a-park", "Yeni park", "#30A46C")],
             "scenario": {"note": "12 m'lik yol önerisi"}},
            layer("parsel", "Parsel", "#4F8EF7", 0.35, time=PARSEL),
            layer("bina", "Bina", "#F76B15", time={"start": "yapim_tarihi", "cumulative": True}),
            layer("ariza", "Arıza", "#E5484D", time={"start": "tarih"}),
            layer("yol", "Yol", "#8C9AAA", 0.5),
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
