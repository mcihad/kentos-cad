#!/usr/bin/env python3
"""Koordinat sistemine dönüştür from every source system it offers (docs/adr/0201, docs/adr/0741).

    python3 scripts/fixtures/reproject_systems_cases.py           # writes the drawing and the cases
    python3 scripts/fixtures/reproject_systems_cases.py --check   # writes nothing; compares

Writes fixtures/processing/v1/reproject.kcad (a TUREF / TM30 drawing, EPSG:5254) and reproject.json. The drawing has
one layer per system of the registry but the local one and the project's own: a square parcel near Kütahya (TUREF
latitude 39.4, longitude 29.98), its corners put into that system by PROJ and written with fixed decimals (100 m
sides in a plane system, 0.001° in a geographic one). A case converts one layer from its system to the project's:
the corners moved by PROJ along the EPSG path crs_transform_cases.py names, not by KentOS. A parcel's corners stay
four (a polygon's straight edges are not densified), its attributes stay, the result is a new layer Dönüştürülen.

The tolerance is a millimetre: the datum paths are the same EPSG operations, but PROJ and the core compose them in
their own order (crs_sweep.py's known difference is a tenth of a millimetre).
"""
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import crs_transform_cases as crs  # noqa: E402

ROOT = HERE.parents[1]
PROC = ROOT / "fixtures/processing/v1"
TOL = 1e-3
TOOL = "geometry.reproject"
PROJECT_SRID = 5254
PLACE = (39.4, 29.98)
NEW = {"id": "islem-donusturulen", "name": "Dönüştürülen",
       "style": {"color": "#AB4ABA", "lineType": "continuous", "lineWeight": 0.25, "fill": "#AB4ABA26"}}

ALL = {s["srid"]: s for s in json.loads(crs.REGISTRY.read_text(encoding="utf-8"))["systems"]}
SOURCES = [s for s in ALL.values() if s["kind"] != "local" and s["srid"] != PROJECT_SRID]


def geographic(entry):
    return entry["kind"] == "geographic"


def corners(entry):
    """The parcel's four corners in the system's own coordinates, as written in the drawing."""
    x, y = crs.into(entry, *PLACE)
    if geographic(entry):
        x0, y0, d, digits = round(x, 6), round(y, 6), 0.001, 6
    else:
        x0, y0, d, digits = round(x, 3), round(y, 3), 100.0, 3
    return [(round(x0 + dx, digits), round(y0 + dy, digits)) for dx, dy in ((0, 0), (d, 0), (d, d), (0, d))]


def layer_of(entry):
    return f"s{entry['srid']}"


def document():
    style = lambda c: {"color": c, "lineType": "continuous", "lineWeight": 0.25}  # noqa: E731
    layers = [{"id": "cizim", "name": "Çizim", "type": "layer", "visible": True, "locked": False, "expanded": True,
               "style": style("fg"), "children": []}]
    entities = []
    for i, s in enumerate(SOURCES, 1):
        layers.append({"id": layer_of(s), "name": f"{s['name']} (EPSG:{s['srid']})", "type": "layer", "visible": True,
                       "locked": False, "expanded": True, "style": style("#AD7F58"), "children": []})
        entities.append({"kind": "polygon", "id": i, "layerId": layer_of(s), "attrs": {"Ad": f"Parsel {s['srid']}"},
                         "pts": [{"x": x, "y": y} for x, y in corners(s)]})
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "İşlem durumları: Koordinat sistemine dönüştür",
        "settings": {"srid": PROJECT_SRID, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": 499000, "y": 4361000},
        "layers": layers,
        "activeLayer": "cizim",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }


def case(entry):
    t, _, _ = crs.transformer(entry, ALL[PROJECT_SRID])
    moved = [t.transform(x, y) for x, y in corners(entry)]
    return {
        "id": f"source-{entry['srid']}",
        "title": f"Koordinat sistemine dönüştür: {entry['name']} parseli TUREF / TM30'a, köşeleri PROJ'la",
        "document": "reproject.kcad",
        "run": {"tool": TOOL},
        "values": {"input": {"scope": "layer", "layerId": layer_of(entry)}, "source": str(entry["srid"]),
                   "layer": {"newName": NEW["name"]}},
        "expect": {
            "status": "ok",
            "summary": f"1 nesne dönüştürüldü: EPSG:{entry['srid']} → EPSG:{PROJECT_SRID}.",
            "undo": "Koordinat sistemine dönüştür",
            "layers": [NEW],
            "addedShapes": [{"kind": "polygon", "layerId": NEW["id"], "attrs": {"Ad": f"Parsel {entry['srid']}"}, "parts": 1,
                             "holes": 0, "vertices": [{"x": x, "y": y} for x, y in moved]}],
            "outputs": {"count": 1},
        },
    }


def text_of(v):
    return json.dumps(v, ensure_ascii=False, indent=1) + "\n"


def outputs():
    cases = {"format": "kentos.processing-cases", "version": 1, "tolerance": TOL, "measureTolerance": TOL, "documents": {},
             "cases": [case(s) for s in SOURCES]}
    return [(PROC / "reproject.json", text_of(cases).encode("utf-8")), (PROC / "reproject.kcad", text_of(document()).encode("utf-8"))]


def main():
    outs = outputs()
    if "--check" in sys.argv[1:]:
        bad = [p for p, data in outs if not p.exists() or p.read_bytes() != data]
        for p in bad:
            print(f"{p.relative_to(ROOT)}: diskteki dosya kurallardan üretilenle aynı değil", file=sys.stderr)
        if bad:
            return 1
        print("Koordinat sistemine dönüştür'ün sistem durumları tutarlı: fixtures/processing/v1/reproject.*")
        return 0
    for p, data in outs:
        p.write_bytes(data)
    print(f"written: {', '.join(str(p.relative_to(ROOT)) for p, _ in outs)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
