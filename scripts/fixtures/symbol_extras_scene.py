#!/usr/bin/env python3
"""The scene of Sembol katmanı ekleri's pictures and trace (docs/adr/0226): fixtures/interaction/v1/symbol-extras.kcad.

A GIS project near (487000, 4420000), 1:1000, places relative to that corner (metres):
- Arazi (land use): a park, a housing block and a market square, coloured by Kullanım.
- Kabartma: soft grey bands of a relief over the land use, drawn with Çoğalt at 70 % (Katman çizimi).
- Gölet: a pond, its fill with an inner glow.
- Yol: two roads crossing, a pale casing and a white core.
- Bina: buildings with a drop shadow (Gölge) to the lower right.
- Eş yükselti: contour lines, their heights labelled along them; the trace gives their labels a mask over Yol.
- Fotoğraf: photo points; each a dot and, by a geometry generator, the slice of its view from its Yön (60°, 12 mm),
  one without Yön on the fallback (north).
Values are written out: the file is the same on every run. `--check` says whether the file holds what this script
writes.
"""
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FILE = ROOT / "fixtures" / "interaction" / "v1" / "symbol-extras.kcad"

X0, Y0 = 487000.0, 4420000.0


def p(x, y):
    return {"x": round(X0 + x, 3), "y": round(Y0 + y, 3)}


def layer(id_, name, color, weight=0.25, **extra):
    style = {"color": color, "lineType": "continuous", "lineWeight": weight}
    style.update(extra.pop("style", {}))
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": style, "children": [], **extra}


def single(**symbols):
    return {"type": "single", "symbols": symbols}


def base(id_, layer_id, attrs=None, **more):
    return {"id": id_, "layerId": layer_id, "attrs": attrs or {}, **more}


def ring(*pts):
    return [p(x, y) for x, y in pts]


def contour(level, cx, cy, rx, ry, turn, n=48):
    """A closed contour: an ellipse round (cx, cy), wobbled a little, as a polyline."""
    pts = []
    for k in range(n):
        a = 2 * math.pi * k / n
        wobble = 1 + 0.06 * math.sin(3 * a + level / 7)
        x, y = rx * math.cos(a) * wobble, ry * math.sin(a) * wobble
        c, s = math.cos(turn), math.sin(turn)
        pts.append(p(cx + c * x - s * y, cy + s * x + c * y))
    pts.append(pts[0])
    return pts


def document():
    ents = []
    eid = 1

    def add(kind, layer_id, attrs=None, **geom):
        nonlocal eid
        ents.append({"kind": kind, **base(eid, layer_id, attrs), **geom})
        eid += 1

    # Land use.
    add("polygon", "arazi", {"Kullanım": "Park"}, pts=ring((0, 0), (110, 0), (110, 70), (0, 70)))
    add("polygon", "arazi", {"Kullanım": "Konut"}, pts=ring((120, 0), (240, 0), (240, 70), (120, 70)))
    add("polygon", "arazi", {"Kullanım": "Konut"}, pts=ring((0, 80), (110, 80), (110, 150), (0, 150)))
    add("polygon", "arazi", {"Kullanım": "Ticaret"}, pts=ring((120, 80), (240, 80), (240, 150), (120, 150)))
    # The relief's bands (darker towards the hill's shaded side).
    for k in range(4):
        add("polygon", "kabartma", {"Gölge": str(k)}, pts=ring((150 + k * 22, 0), (240, 0), (240, 150), (150 + k * 22, 150)))
    # The pond.
    add("polygon", "golet", {"Ad": "Gölet"}, pts=[p(55 + 32 * math.cos(2 * math.pi * k / 40) * (1 + 0.1 * math.sin(2 * math.pi * k / 40 * 3)), 35 + 20 * math.sin(2 * math.pi * k / 40)) for k in range(40)])
    # Roads: east–west and north–south.
    add("polyline", "yol", {"Ad": "Atatürk Caddesi"}, pts=ring((-10, 75), (250, 75)))
    add("polyline", "yol", {"Ad": "Okul Sokağı"}, pts=ring((115, -10), (115, 160)))
    # Buildings.
    for x, y, w, h in [(130, 10, 30, 22), (170, 12, 26, 20), (205, 8, 25, 28), (12, 92, 34, 24), (60, 96, 28, 30), (130, 92, 40, 24), (185, 100, 36, 30)]:
        add("polygon", "bina", {"Kat": "3"}, pts=ring((x, y), (x + w, y), (x + w, y + h), (x, y + h)))
    # Contours round a hill to the east, labelled by Kot.
    for level, rx, ry in [(820, 70, 46), (825, 52, 34), (830, 34, 22)]:
        add("polyline", "esyukselti", {"Kot": str(level)}, pts=contour(level, 190, 90, rx, ry, 0.35), label=str(level))
    # Photo points with their Yön; one has none.
    for x, y, yon in [(30, 20, "45"), (90, 120, "200"), (175, 60, "300"), (220, 135, None)]:
        add("point", "foto", {"Yön": yon} if yon else {}, p=p(x, y))

    wedge = {
        "type": "marker",
        "layers": [
            {"id": "g", "type": "geometryGenerator",
             "steps": [{"kind": "wedge", "direction": {"expr": "[Yön]", "fallback": 0}, "width": 60, "radius": 12}],
             "symbol": {"type": "fill", "layers": [{"id": "f", "type": "simpleFill", "color": "#E5484D40"},
                                                   {"id": "l", "type": "simpleLine", "color": "#E5484D", "width": 0.25}]}},
            {"id": "c", "type": "shape", "shape": "circle", "size": 2.2, "fill": "#E5484D", "stroke": "#FFFFFF", "strokeWidth": 0.3},
        ],
    }
    land = {
        "type": "categorized", "expr": "Kullanım",
        "categories": [
            {"value": v, "label": v, "symbols": {"fill": {"type": "fill", "layers": [{"id": "f", "type": "simpleFill", "color": c}]}}}
            for v, c in [("Park", "#A1D99B"), ("Konut", "#FEE391"), ("Ticaret", "#FCBBA1")]
        ],
    }
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Sembol katmanı ekleri",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": X0, "y": Y0},
        "layers": [
            layer("foto", "Fotoğraf", "#E5484D", style={"renderer": single(marker=wedge)}),
            layer("esyukselti", "Eş yükselti", "#8D6E63", 0.35,
                  style={"label": {"placement": "along", "size": 11.0, "weight": 600, "line": "curved", "repeat": 180.0,
                                   "priority": 5, "color": "#6D4C41", "halo": {"width": 1.5}}}),
            layer("bina", "Bina", "#8C7B6B", style={"renderer": single(fill={
                "type": "fill", "layers": [
                    {"id": "f", "type": "simpleFill", "color": "#D9CBBE",
                     "effects": [{"type": "dropShadow", "distance": 1.2, "angle": 135, "blur": 1.5, "color": "#000000", "opacity": 0.45}]},
                    {"id": "l", "type": "simpleLine", "color": "#8C7B6B", "width": 0.3}]})}),
            layer("yol", "Yol", "#B0B0B0", style={"renderer": single(line={
                "type": "line", "layers": [
                    {"id": "k", "type": "simpleLine", "color": "#9E9E9E", "width": 3.2, "cap": "round"},
                    {"id": "o", "type": "simpleLine", "color": "#FFFFFF", "width": 2.4, "cap": "round"}]})}),
            layer("golet", "Gölet", "#4292C6", style={"renderer": single(fill={
                "type": "fill", "layers": [
                    {"id": "f", "type": "simpleFill", "color": "#6BAED6",
                     "effects": [{"type": "innerGlow", "spread": 1, "blur": 2.5, "color": "#FFFFFF", "opacity": 0.8}]},
                    {"id": "l", "type": "simpleLine", "color": "#2171B5", "width": 0.35}]})}),
            layer("kabartma", "Kabartma", "#9A9A9A",
                  style={"renderer": {"type": "categorized", "expr": "Gölge", "categories": [
                      {"value": str(k), "label": f"Bant {k + 1}", "symbols": {"fill": {"type": "fill", "layers": [
                          {"id": "f", "type": "simpleFill", "color": c}]}}}
                      for k, c in enumerate(["#E6E6E6", "#CFCFCF", "#B4B4B4", "#9A9A9A"])]}},
                  compose={"opacity": 0.7, "blend": "multiply"}),
            layer("arazi", "Arazi", "#8C9AAA", style={"renderer": land}),
        ],
        "activeLayer": "esyukselti",
        "entities": ents,
        "styles": {"items": [], "categories": []},
    }


def main() -> int:
    text = json.dumps(document(), ensure_ascii=False, indent=1) + "\n"
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
