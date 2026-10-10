#!/usr/bin/env python3
"""Mekânsal istatistik'in resimlerinin çizimi (docs/adr/0238): bir mahallenin yolları, yapı adaları (Değer: m² birim değeri,
kuzeydoğuda yüksek) ve trafik kazaları (Tür'e göre kümeler: yayalar çarşının kavşaklarında, araçlar doğu–batı caddesi boyunca,
bisikletler kuzey–güney yolunda; Ağırlık, Yıl).

    python3 scripts/fixtures/spatial_stats_scene.py          # fixtures/interaction/v1/spatial-stats.kcad'i yazar
    python3 scripts/fixtures/spatial_stats_scene.py --check  # hiçbir şey yazmaz; karşılaştırır

KentOS kodu kullanılmaz; yerler çeyrek metrelik ızgarada, deterministik bir üreteçle.
"""

import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "interaction" / "v1" / "spatial-stats.kcad"
E, N = 487000, 4420000
COLS, ROWS = 6, 6
DX, DY = 200, 150
GAP = 12


class Lcg:
    def __init__(self, seed):
        self.s = seed

    def next(self):
        self.s = (self.s * 6364136223846793005 + 1442695040888963407) % 2 ** 64
        return self.s >> 33

    def unit(self):
        return (self.next() % 1_000_000) / 1_000_000

    def gauss(self):
        return sum(self.unit() for _ in range(6)) - 3.0


def q(v):
    """A quarter of a metre."""
    return round(v * 4) / 4


def P(x, y):
    return {"x": E + q(x), "y": N + q(y)}


def style(color, weight=0.25, **more):
    return {"color": color, "lineType": "continuous", "lineWeight": weight, **more}


def layer(i, name, st):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True, "style": st, "children": []}


def drawing():
    rng = Lcg(2038)
    entities = []

    def add(e, lid, attrs):
        entities.append({**e, "id": len(entities) + 1, "layerId": lid, "attrs": attrs})

    # Streets: avenues east–west, streets north–south.
    for j in range(ROWS + 1):
        add({"kind": "polyline", "pts": [P(-30, j * DY), P(COLS * DX + 30, j * DY)]}, "yol", {"Ad": f"{j + 1}. Cadde"})
    for i in range(COLS + 1):
        add({"kind": "polyline", "pts": [P(i * DX, -30), P(i * DX, ROWS * DY + 30)]}, "yol", {"Ad": f"{i + 1}. Sokak"})
    # Blocks with a value per m²: rising to the north-east, a ripple, a little noise.
    for j in range(ROWS):
        for i in range(COLS):
            x0, y0 = i * DX + GAP, j * DY + GAP
            x1, y1 = (i + 1) * DX - GAP, (j + 1) * DY - GAP
            cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
            v = 7000 + 9000 * (cx / (COLS * DX)) * (cy / (ROWS * DY)) + 1400 * math.sin(cx / 170) * math.cos(cy / 140) + 600 * rng.gauss()
            add({"kind": "polygon", "pts": [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]}, "ada",
                {"Ada": str(101 + j * COLS + i), "Değer": f"{round(v)}"})
    # Accidents: pedestrians at the bazaar's crossings, cars along the 4th avenue, bicycles along the 3rd street.
    k = 0

    def accident(x, y, tur):
        nonlocal k
        k += 1
        add({"kind": "point", "p": P(x, y)}, "kaza",
            {"No": f"K{k:03d}", "Tür": tur, "Ağırlık": str(1 + rng.next() % 3), "Yıl": str(2023 + rng.next() % 3)})

    for (cx, cy) in [(800, 600), (1000, 600), (1000, 750)]:
        for _ in range(22):
            accident(cx + 18 * rng.gauss(), cy + 18 * rng.gauss(), "Yaya")
    for _ in range(45):
        accident(80 + rng.unit() * 1040, 450 + 9 * rng.gauss(), "Araç")
    for _ in range(30):
        accident(400 + 7 * rng.gauss(), 60 + rng.unit() * 780, "Bisiklet")
    for _ in range(14):
        accident(rng.unit() * COLS * DX, rng.unit() * ROWS * DY, "Araç")
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Mekânsal istatistik",
        "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 2000,
                     "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": E, "y": N},
        "layers": [layer("kaza", "Kazalar", style("#4C6EF5", point={"symbol": "ring", "size": 5})),
                   layer("ada", "Yapı adaları", style("#868E96", 0.25, fill="#868E9614")),
                   layer("yol", "Yollar", style("#ADB5BD", 0.5))],
        "activeLayer": "kaza",
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
