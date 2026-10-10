#!/usr/bin/env python3
"""The scene of the thematic renderers' pictures (docs/adr/0213): fixtures/interaction/v1/renderers.kcad.

A GIS project near (487000, 4420000), each renderer in a region of its own (places relative to that corner, metres):
- Sürekli renk (0..900, 600..1200): twelve quarters, coloured by Nüfus.
- İki değişkenli renk (1000..1900, 600..1200): the same quarters by Nüfus and Gelir, 3 × 3.
- Nokta yoğunluğu (0..900, -100..500): the same quarters, a dot per 250 Erkek and Kadın.
- Grafik (1000..1900, -100..500): the same quarters, a pie of Konut, Ticaret and Yeşil.
- Isı haritası (2000..2900, 600..1200): 1 500 incidents round five places, weighted by Önem.
- Kümeleme (2000..2900, -100..500): 800 trees in three groves and scattered.
- Yayma (3000..3600, 600..1200): bus stops, some stopped at the same place two to five times.
- Orantılı sembol (3000..3600, -100..500): schools by Öğrenci, roads by Trafik.
- Ters alan (hidden at first): the study area round the quarters of Sürekli renk.
A text over each region says which it is. Values come from a fixed linear congruential sequence: the file is the same
on every run.

`--check` says whether the file holds what this script writes.
"""
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FILE = ROOT / "fixtures" / "interaction" / "v1" / "renderers.kcad"

X0, Y0 = 487000.0, 4420000.0


class Lcg:
    """A fixed sequence of numbers in [0, 1)."""

    def __init__(self, seed):
        self.s = seed

    def __call__(self):
        self.s = (self.s * 6364136223846793005 + 1442695040888963407) % (1 << 64)
        return (self.s >> 11) / float(1 << 53)

    def normal(self):
        return math.sqrt(-2.0 * math.log(max(self(), 1e-12))) * math.cos(2.0 * math.pi * self())


def p(x, y):
    return {"x": round(X0 + x, 3), "y": round(Y0 + y, 3)}


def layer(id_, name, color, weight=0.25, visible=True, **style):
    return {"id": id_, "name": name, "type": "layer", "visible": visible, "locked": False, "expanded": True,
            "style": {"color": color, "lineType": "continuous", "lineWeight": weight, **style}, "children": []}


def fill(color, edge="#FFFFFF", width=0.3):
    return {"type": "fill", "layers": [{"id": "f", "type": "simpleFill", "color": color},
                                        {"id": "l", "type": "simpleLine", "color": edge, "width": width}]}


def circle(color, size=4.0):
    return {"type": "marker", "layers": [{"id": "c", "type": "shape", "shape": "circle", "size": size, "fill": color,
                                          "stroke": "#FFFFFF", "strokeWidth": 0.2}]}


RAMP = ["#FFF5B8", "#FDB863", "#E66101", "#A50F15"]
HEAT = ["#2B83BA00", "#2B83BA", "#ABDDA4", "#FFFFBF", "#FDAE61", "#D7191C"]
GRID3 = ["#E8E8E8", "#A1D6D6", "#5AC8C8", "#D3A6CA", "#94A3BF", "#4AA0C0", "#BE64AC", "#7D5FA0", "#3B4994"]


def quarters(rng, ox, oy):
    """Twelve quarters (4 × 3) with their numbers: an irregular grid round (ox, oy)."""
    out = []
    xs = [0, 230, 445, 690, 900]
    ys = [0, 190, 410, 600]
    jitter = {(i, j): ((rng() - 0.5) * 40, (rng() - 0.5) * 40) for i in range(1, 4) for j in range(1, 3)}

    def corner(i, j):
        dx, dy = jitter.get((i, j), (0.0, 0.0))
        return p(ox + xs[i] + dx, oy + ys[j] + dy)

    names = ["Cumhuriyet", "Atatürk", "Yenimahalle", "Bahçelievler", "Esentepe", "Kocatepe", "Gültepe", "Yıldızevler",
             "Kavaklıdere", "Çamlık", "Barış", "Sevgi"]
    k = 0
    for j in range(3):
        for i in range(4):
            ring = [corner(i, j), corner(i + 1, j), corner(i + 1, j + 1), corner(i, j + 1)]
            n = int(4000 + rng() * 36000)
            erkek = int(n * (0.47 + rng() * 0.06))
            out.append({"name": names[k], "pts": ring, "attrs": {
                "Ad": names[k], "Nüfus": str(n), "Erkek": str(erkek), "Kadın": str(n - erkek),
                "Gelir": str(int(8000 + rng() * 42000)), "Konut": str(int(20 + rng() * 60)),
                "Ticaret": str(int(5 + rng() * 30)), "Yeşil": str(int(5 + rng() * 25))}})
            k += 1
    return out


def document():
    entities = []

    def add(e):
        entities.append({"id": len(entities) + 1, "attrs": {}, **e})

    rng = Lcg(2026)
    base = quarters(rng, 0, 600)
    for q in base:
        add({"kind": "polygon", "layerId": "surekli", "attrs": q["attrs"], "pts": q["pts"]})
    shift = lambda pts, dx, dy: [{"x": round(v["x"] + dx, 3), "y": round(v["y"] + dy, 3)} for v in pts]
    for q in base:
        add({"kind": "polygon", "layerId": "iki", "attrs": q["attrs"], "pts": shift(q["pts"], 1000, 0)})
        add({"kind": "polygon", "layerId": "nokta", "attrs": q["attrs"], "pts": shift(q["pts"], 0, -700)})
        add({"kind": "polygon", "layerId": "grafik", "attrs": q["attrs"], "pts": shift(q["pts"], 1000, -700)})
    # The study area: round the quarters of Sürekli renk, a little in.
    add({"kind": "polygon", "layerId": "calisma",
         "pts": [p(40, 640), p(860, 610), p(880, 1150), p(470, 1180), p(30, 1120)]})
    # Isı haritası: incidents round five places.
    centres = [(2250, 900, 70), (2600, 780, 45), (2450, 1050, 35), (2750, 1100, 55), (2150, 700, 30)]
    for i in range(1500):
        cx, cy, s = centres[i % 5]
        if i % 7 == 0:
            x, y = 2000 + rng() * 900, 600 + rng() * 600
        else:
            x, y = cx + rng.normal() * s, cy + rng.normal() * s
        add({"kind": "point", "layerId": "olay", "attrs": {"Önem": str(1 + int(rng() * 5))}, "p": p(x, y)})
    # Kümeleme: trees in three groves and scattered.
    groves = [(2200, 150, 60), (2550, 300, 90), (2750, 50, 40)]
    for i in range(800):
        if i % 5 == 0:
            x, y = 2000 + rng() * 900, -100 + rng() * 600
        else:
            gx, gy, s = groves[i % 3]
            x, y = gx + rng.normal() * s, gy + rng.normal() * s
        add({"kind": "point", "layerId": "agac", "attrs": {"Tür": ["Çınar", "Meşe", "Çam"][i % 3]}, "p": p(x, y)})
    # Yayma: bus stops, some at one place two to five times (one for each line stopping there).
    lines = ["12", "34", "56", "78", "90"]
    for k in range(40):
        x, y = 3000 + (k % 8) * 75 + rng() * 10, 650 + (k // 8) * 110 + rng() * 10
        for m in range(1 + (k % 5 if k % 3 == 0 else 0)):
            add({"kind": "point", "layerId": "durak", "attrs": {"Hat": lines[m]}, "p": p(x, y)})
    # Orantılı sembol: schools by their pupils, roads by their traffic.
    for k in range(24):
        x, y = 3040 + (k % 6) * 100 + rng() * 30, -60 + (k // 6) * 130 + rng() * 30
        add({"kind": "point", "layerId": "okul", "attrs": {"Öğrenci": str(int(100 + rng() * 1900))}, "p": p(x, y)})
    for k, (a, b, t) in enumerate((((2980, -80), (3620, -60), 4000), ((2980, 120), (3620, 160), 18000),
                                   ((2980, 330), (3620, 300), 9000), ((3300, -100), (3320, 480), 26000))):
        add({"kind": "polyline", "layerId": "yol", "attrs": {"Trafik": str(t)}, "pts": [p(*a), p(*b)]})
    # The regions' names.
    for (x, y), t in (((20, 1230), "Sürekli renk"), ((1020, 1230), "İki değişkenli renk"), ((20, 530), "Nokta yoğunluğu"),
                      ((1020, 530), "Grafik"), ((2020, 1230), "Isı haritası"), ((2020, 530), "Kümeleme"),
                      ((3000, 1230), "Yayma"), ((3000, 530), "Orantılı sembol")):
        add({"kind": "text", "layerId": "yazi", "p": p(x, y), "text": t, "height": 18.0, "rotation": 0.0})
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Ek işleyiciler",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 5000, "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": X0, "y": Y0},
        "layers": [
            layer("yazi", "Yazı", "fg"),
            layer("calisma", "Çalışma alanı", "#8E4EC6", 0.5, visible=False,
                  renderer={"type": "inverted", "symbols": {"fill": {"type": "fill", "layers": [
                      {"id": "f", "type": "simpleFill", "color": "#FFFFFFB3"},
                      {"id": "l", "type": "simpleLine", "color": "#8E4EC6", "width": 0.6}]}}}),
            layer("durak", "Durak", "#30A46C",
                  renderer={"type": "displacement", "tolerance": 4, "unit": "px", "placement": "ring",
                            "circle": {"color": "#7D7D7D", "width": 1},
                            "renderer": {"type": "categorized", "expr": "Hat", "categories": [
                                {"value": h, "label": f"Hat {h}", "symbols": {"marker": circle(c, 3.0)}}
                                for h, c in zip(lines, ["#E15759", "#4E79A7", "#F28E2B", "#59A14F", "#B07AA1"])]}}),
            layer("okul", "Okul", "#3E63DD",
                  renderer={"type": "proportional", "expr": "Öğrenci", "minValue": 100, "maxValue": 2000, "minSize": 2,
                            "maxSize": 9, "unit": "mm", "scaling": "flannery", "symbols": {"marker": circle("#3E63DD")}}),
            layer("yol", "Yol", "#8C9AAA",
                  renderer={"type": "proportional", "expr": "Trafik", "minValue": 0, "maxValue": 30000, "minSize": 0.3,
                            "maxSize": 2.5, "unit": "mm", "scaling": "radius",
                            "symbols": {"line": {"type": "line", "layers": [{"id": "l", "type": "simpleLine", "color": "#8C9AAA",
                                                                             "width": 0.5, "cap": "round"}]}}}),
            layer("agac", "Ağaç", "#30A46C",
                  renderer={"type": "cluster", "distance": 40, "unit": "px", "grow": True,
                            "renderer": {"type": "single", "symbols": {"marker": circle("#30A46C", 2.0)}}}),
            layer("olay", "Olay", "#E5484D",
                  renderer={"type": "heatmap", "radius": 28, "unit": "px", "weight": "Önem", "ramp": HEAT, "quality": 2,
                            "opacity": 0.9}),
            layer("grafik", "Mahalle (grafik)", "#8C9AAA",
                  renderer={"type": "chart", "kind": "pie", "size": 9, "unit": "mm",
                            "fields": [{"expr": "Konut", "color": "#E15759"}, {"expr": "Ticaret", "color": "#4E79A7"},
                                       {"expr": "Yeşil", "color": "#59A14F"}],
                            "outline": {"color": "#FFFFFF", "width": 0.2},
                            "symbols": {"fill": fill("#F2F2F2", "#B0B0B0", 0.25)}}),
            layer("nokta", "Mahalle (nokta)", "#8C9AAA",
                  renderer={"type": "dotDensity", "dotValue": 250, "dotSize": 0.6, "unit": "mm", "seed": 1,
                            "fields": [{"expr": "Erkek", "color": "#4E79A7"}, {"expr": "Kadın", "color": "#E15759"}],
                            "symbols": {"fill": fill("#FAFAFA", "#B0B0B0", 0.25)}}),
            layer("iki", "Mahalle (iki değişkenli)", "#8C9AAA",
                  renderer={"type": "bivariate", "exprX": "Nüfus", "exprY": "Gelir", "breaksX": [16000, 28000],
                            "breaksY": [22000, 36000], "colors": GRID3, "symbols": {"fill": fill("#000000")}}),
            layer("surekli", "Mahalle", "#8C9AAA",
                  renderer={"type": "unclassed", "expr": "Nüfus", "min": 4000, "max": 40000, "ramp": RAMP,
                            "symbols": {"fill": fill("#000000")}}),
        ],
        "activeLayer": "surekli",
        "entities": entities,
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
