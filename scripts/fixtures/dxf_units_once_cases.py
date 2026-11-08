#!/usr/bin/env python3
"""The unit applied once (plan/map-cad.md MAP-02, docs/adr/2100): DXF files written in millimetres, centimetres,
inches, feet and metres by ezdxf, and where every point, height and text height lands in metres, as ezdxf itself
places them (its recursive block decomposition) times the unit's length, without KentOS code.

    uv run --with ezdxf==1.4.5 python3 scripts/fixtures/dxf_units_once_cases.py           # writes the files
    uv run --with ezdxf==1.4.5 python3 scripts/fixtures/dxf_units_once_cases.py --check   # compares

Each drawing holds, in its unit: a 3D line, a point with a height, a polyline with an elevation, a circle and an arc
above zero, a text and a multi-line text; a block INNER (a 3D line and a text) inserted in a block OUTER (turned 90°,
scale 3, raised) with a line of its own, OUTER inserted in model space (turned 30°, scale 0.5, raised), and a flat
symbol SEMBOL raised to a point's height. A unit taken
twice (a block's points, a nested insert's offset, a height, a text's size scaled again) puts a value off by the
unit's factor; ezdxf's decomposition is the independent placement.
"""

import json
import math
import sys
from pathlib import Path

import ezdxf
from ezdxf import disassemble  # noqa: F401  (recursive_decompose)
from ezdxf.disassemble import recursive_decompose

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/formats/v1/units-once"
# $INSUNITS code, file stem, metres in one unit (an inch is 25.4 mm, a foot 0.3048 m).
UNITS = ((4, "mm", 0.001), (5, "cm", 0.01), (1, "in", 0.0254), (2, "ft", 0.3048), (6, "m", 1.0))


def drawing(code):
    doc = ezdxf.new("R2018", setup=False)
    doc.header["$INSUNITS"] = code
    doc.header["$MEASUREMENT"] = 1
    msp = doc.modelspace()
    for name in ("CIZGI", "NOKTA", "YAZI", "BLOK"):
        doc.layers.add(name)
    inner = doc.blocks.new("INNER", base_point=(0, 0, 0))
    inner.add_line((0, 0, 10), (100, 0, 20), dxfattribs={"layer": "BLOK"})
    inner.add_text("I", height=50, dxfattribs={"layer": "BLOK", "insert": (10, 10, 5)})
    outer = doc.blocks.new("OUTER", base_point=(0, 0, 0))
    outer.add_blockref("INNER", (1000, 0, 100), dxfattribs={"xscale": 3, "yscale": 3, "zscale": 3, "rotation": 90,
                                                              "layer": "BLOK"})
    outer.add_line((0, 0, 0), (0, 500, 50), dxfattribs={"layer": "BLOK"})
    # A flat symbol (no heights of its own), raised to a point's height as survey drawings place them.
    symbol = doc.blocks.new("SEMBOL", base_point=(0, 0, 0))
    symbol.add_line((-20, 0, 0), (20, 0, 0), dxfattribs={"layer": "NOKTA"})
    symbol.add_circle((0, 0, 0), 10, dxfattribs={"layer": "NOKTA"})
    msp.add_line((0, 0, 1500), (1000, 0, 2500), dxfattribs={"layer": "CIZGI"})
    msp.add_point((500, 500, 1234.5), dxfattribs={"layer": "NOKTA"})
    msp.add_lwpolyline([(0, 2000), (1000, 2000), (1000, 3000)], dxfattribs={"layer": "CIZGI", "elevation": 777})
    msp.add_circle((100, 200, 50), 25, dxfattribs={"layer": "CIZGI"})
    msp.add_arc((300, 200, 60), 40, 0, 90, dxfattribs={"layer": "CIZGI"})
    msp.add_text("T", height=250, dxfattribs={"layer": "YAZI", "insert": (10, 20, 30)})
    msp.add_mtext("M", dxfattribs={"layer": "YAZI", "insert": (40, 50, 0), "char_height": 180})
    msp.add_blockref("OUTER", (5000, 6000, 300), dxfattribs={"xscale": 0.5, "yscale": 0.5, "zscale": 0.5,
                                                             "rotation": 30, "layer": "BLOK"})
    msp.add_blockref("SEMBOL", (2000, 2000, 1234), dxfattribs={"layer": "NOKTA"})
    return doc


def r(v):
    """Rounded to a tenth of a nanometre: what a float carries, the comparison's own tolerance is the test's."""
    return round(v, 10)


def xyz(p, k):
    return [r(p.x * k), r(p.y * k), r(p.z * k)]


def placed(doc, k):
    """Every primitive where ezdxf puts it, in metres: kind, layer, its points (x, y, z) and its sizes."""
    out = []
    for e in recursive_decompose(doc.modelspace()):
        t = e.dxftype()
        layer = e.dxf.layer
        if t == "LINE":
            out.append({"kind": "line", "layer": layer, "points": [xyz(e.dxf.start, k), xyz(e.dxf.end, k)]})
        elif t == "POINT":
            out.append({"kind": "point", "layer": layer, "points": [xyz(e.dxf.location, k)]})
        elif t == "LWPOLYLINE":
            z = e.dxf.elevation
            out.append({"kind": "polyline", "layer": layer,
                        "points": [[r(x * k), r(y * k), r(z * k)] for x, y in e.get_points("xy")]})
        elif t == "CIRCLE":
            out.append({"kind": "circle", "layer": layer, "points": [xyz(e.dxf.center, k)], "radius": r(e.dxf.radius * k)})
        elif t == "ARC":
            out.append({"kind": "arc", "layer": layer, "points": [xyz(e.dxf.center, k)], "radius": r(e.dxf.radius * k),
                        "start": r(e.dxf.start_angle), "end": r(e.dxf.end_angle)})
        elif t == "TEXT":
            out.append({"kind": "text", "layer": layer, "points": [xyz(e.dxf.insert, k)], "height": r(e.dxf.height * k)})
        elif t == "MTEXT":
            out.append({"kind": "mtext", "layer": layer, "points": [xyz(e.dxf.insert, k)],
                        "height": r(e.dxf.char_height * k)})
    return out


def main():
    files, expected = {}, {"note": __doc__.split("\n\n")[0].replace("\n", " "), "ezdxf": ezdxf.__version__, "cases": []}
    for code, stem, k in UNITS:
        doc = drawing(code)
        name = f"{stem}.dxf"
        path = OUT / name
        from io import StringIO
        text = StringIO()
        doc.write(text)
        files[path] = text.getvalue()
        expected["cases"].append({"file": name, "insunits": code, "metres": k, "objects": placed(doc, k)})
    texts = dict(files)
    texts[OUT / "cases.json"] = json.dumps(expected, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        # The DXF texts carry ezdxf's handles and dates: only the expected placements are compared.
        p = OUT / "cases.json"
        if not p.exists() or p.read_text("utf-8") != texts[p]:
            print(f"{p.relative_to(ROOT)} güncel değil; betiği --check olmadan çalıştırın.")
            return 1
        print("Birim bir kez durumları güncel.")
        return 0
    OUT.mkdir(parents=True, exist_ok=True)
    for p, t in texts.items():
        p.write_text(t, "utf-8")
        print(f"yazıldı: {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
