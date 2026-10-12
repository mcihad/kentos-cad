#!/usr/bin/env python3
"""Grafiklerin ve alan istatistiklerinin resimlerinin ve izlerinin çizimi (docs/adr/0227): bir mahallenin parselleri tipli
alanlarıyla (Ada, Parsel, Kullanım değer listesiyle, Kat, Emsal, Ruhsat, Tescil tarihi, m² birim Değer) ve yolları (Ad, Genişlik).
Değer kuzeydoğuya ve kat sayısıyla artar; ticaret parselleri caddeye bakar; birkaç değer boş, biri kurala uymaz (“üç”).

    python3 scripts/fixtures/chart_scene.py          # fixtures/interaction/v1/charts.kcad'i yazar
    python3 scripts/fixtures/chart_scene.py --check  # hiçbir şey yazmaz; karşılaştırır

KentOS kodu kullanılmaz; köşeler yarım metrelik ızgarada, deterministik bir üreteçle.
"""

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "interaction" / "v1" / "charts.kcad"
E, N = 486500, 4421000
COLS, ROWS = 9, 7
USES = [("K", "Konut"), ("T", "Ticaret"), ("İ", "İşyeri"), ("S", "Sanayi"), ("Y", "Yeşil alan"), ("E", "Eğitim")]


class Lcg:
    def __init__(self, seed):
        self.s = seed

    def next(self):
        self.s = (self.s * 6364136223846793005 + 1442695040888963407) % 2 ** 64
        return self.s >> 33

    def unit(self):
        return (self.next() % 1_000_000) / 1_000_000


def h(v):
    """Half a metre."""
    return round(v * 2) / 2


def P(x, y):
    return {"x": E + h(x), "y": N + h(y)}


def style(color, weight=0.25, **more):
    return {"color": color, "lineType": "continuous", "lineWeight": weight, **more}


def layer(i, name, st, fields):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True, "style": st,
            "children": [], "fields": fields}


def drawing():
    rng = Lcg(227)
    entities = []

    def add(e, lid, attrs, label=None):
        entities.append({**e, "id": len(entities) + 1, "layerId": lid, "attrs": attrs, **({"label": label} if label else {})})

    # Columns of uneven widths, rows of uneven depths: the parcels' areas spread.
    xs = [0]
    for i in range(COLS):
        xs.append(xs[-1] + 22 + 4 * (i % 3) + (rng.next() % 9))
    ys = [0]
    for j in range(ROWS):
        ys.append(ys[-1] + 26 + 5 * (j % 2) + (rng.next() % 7))
    for j in range(ROWS):
        for i in range(COLS):
            x0, x1, y0, y1 = xs[i] + 1, xs[i + 1] - 1, ys[j] + 1, ys[j + 1] - 1
            ada = 112 + j // 2
            parsel = 1 + i + (j % 2) * COLS
            # Ticaret on the avenue (the middle row), Sanayi in the south-west, green and schools here and there.
            if j == 3:
                use = "T" if i % 4 else "İ"
            elif j == 0 and i < 3:
                use = "S"
            elif (i, j) in [(4, 5), (7, 1)]:
                use = "Y"
            elif (i, j) in [(2, 5), (6, 2)]:
                use = "E"
            else:
                use = "K" if rng.next() % 6 else "İ"
            floors = {"T": 4, "İ": 3, "S": 1, "Y": 0, "E": 3}.get(use, 2) + rng.next() % 4 - (1 if use in "TKİ" else 0)
            floors = max(0, floors)
            value = 4200 + 260 * i + 180 * j + 380 * floors + (900 if use == "T" else 0) + (rng.next() % 600)
            emsal = (floors * 0.35) if use != "Y" else 0.0
            attrs = {
                "Ada": str(ada),
                "Parsel": str(parsel),
                "Kullanım": use,
                "Kat": str(floors),
                "Emsal": f"{emsal:.2f}",
                "Ruhsat": "true" if rng.next() % 5 else "false",
                "Tescil": f"{2014 + rng.next() % 12}-{1 + rng.next() % 12:02d}-{1 + rng.next() % 28:02d}",
                "Değer": str(value - value % 10),
            }
            if (i, j) in [(8, 6), (0, 4)]:
                del attrs["Kat"]
            if (i, j) == (5, 0):
                attrs["Kat"] = "üç"
            if use == "Y":
                del attrs["Değer"]
            add({"kind": "polygon", "pts": [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]}, "parsel", attrs, f"{ada}/{parsel}")
    # Streets between the blocks.
    width = xs[-1]
    for j, name in [(2, "Lale Sokak"), (4, "Cumhuriyet Caddesi"), (6, "Gül Sokak")]:
        add({"kind": "polyline", "pts": [P(-8, ys[j]), P(width + 8, ys[j])]}, "yol",
            {"Ad": name, "Genişlik": "15" if "Cadde" in name else "7"})
    for i, name in [(3, "Okul Sokak"), (6, "Pazar Sokak")]:
        add({"kind": "polyline", "pts": [P(xs[i], -8), P(xs[i], ys[-1] + 8)]}, "yol", {"Ad": name, "Genişlik": "10"})
    parcel_fields = [
        {"name": "Ada", "kind": "text", "required": True},
        {"name": "Parsel", "kind": "integer", "min": "1"},
        {"name": "Kullanım", "kind": "text", "values": [{"code": c, "label": l} for c, l in USES]},
        {"name": "Kat", "kind": "integer", "min": "0", "max": "20"},
        {"name": "Emsal", "kind": "decimal", "scale": 2},
        {"name": "Ruhsat", "kind": "boolean"},
        {"name": "Tescil", "alias": "Tescil tarihi", "kind": "date"},
        {"name": "Değer", "alias": "Birim değer (TL/m²)", "kind": "decimal", "scale": 0},
    ]
    road_fields = [{"name": "Ad", "kind": "text"}, {"name": "Genişlik", "kind": "decimal", "scale": 1}]
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Grafikler",
        "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                     "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": E, "y": N},
        "layers": [layer("parsel", "Parseller", style("#3E63DD", 0.25, fill="#3E63DD14"), parcel_fields),
                   layer("yol", "Yollar", style("#ADB5BD", 0.5), road_fields)],
        "activeLayer": "parsel",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }


def main():
    text = json.dumps(drawing(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check olmadan çalıştırıp farkı okuyun.")
            return 1
        print(f"{OUT.relative_to(ROOT)}: güncel.")
        return 0
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
