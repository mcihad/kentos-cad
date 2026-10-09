#!/usr/bin/env python3
"""Hidroloji'nin resimlerinin çizimi (docs/adr/0235): paylaşılan vadinin yükseklik modeli (fixtures/interaction/v1/rasters.kcad,
rasters/dem.tif), doğudaki dere sisteminin kollarını kesen bir yol ekseni ve iki dere sisteminin çıkışına yakın iki nokta.

    python3 scripts/fixtures/hydrology_scene.py          # fixtures/interaction/v1/hydrology.kcad'i yazar
    python3 scripts/fixtures/hydrology_scene.py --check  # hiçbir şey yazmaz; karşılaştırır

KentOS kodu kullanılmaz. Yükseklik modelinin nesnesi rasters.kcad'dekinin aynısıdır (dosyası rasters/dem.tif, bu çizimin
yanındaki klasörde); yol ve noktalar elle seçilmiştir: yol doğu sisteminin ana deresini ve dört kolunu keser, noktalar
iki ana derenin rasterin kenarına yakın yerlerindedir (Yaklaştırma uzaklığı 10 m onları dereye taşır).
"""

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
VALLEY = ROOT / "fixtures/interaction/v1/rasters.kcad"
OUT = ROOT / "fixtures/interaction/v1/hydrology.kcad"

# The road's axis: across the eastern streams, a little north of the eastern main stream.
ROAD = [(487640.0, 4420322.0), (487760.0, 4420334.0), (487880.0, 4420326.0), (487960.0, 4420338.0)]
# The outlets: near the western and the eastern main stream's leaving the raster.
OUTLETS = [(487248.0, 4420276.0), (487860.0, 4420272.0)]


def layer(id, name, color):
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def drawing():
    valley = json.loads(VALLEY.read_text("utf-8"))
    dem = next(e for e in valley["entities"] if e["layerId"] == "dem")
    entities = [{**dem, "id": 1}]
    entities.append({"kind": "polyline", "id": 2, "layerId": "yol", "attrs": {"Ad": "Yol ekseni"},
                     "pts": [{"x": x, "y": y} for x, y in ROAD]})
    for k, (x, y) in enumerate(OUTLETS):
        entities.append({"kind": "point", "id": 3 + k, "layerId": "cikis", "attrs": {"Ad": f"Ç{k + 1}"}, "p": {"x": x, "y": y}})
    dem_layer = next(l for l in valley["layers"] if l["id"] == "dem")
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Havza",
        "settings": valley["settings"],
        "origin": valley["origin"],
        "layers": [layer("cikis", "Çıkış noktaları", "#C62828"), layer("yol", "Yol ekseni", "#1F2933"), dem_layer],
        "activeLayer": "dem",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }


def main():
    text = json.dumps(drawing(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.name} güncel değil. Yeniden yazın: python3 {sys.argv[0]}; farkı okuyun.")
            sys.exit(1)
        print(f"{OUT.name}: güncel.")
        return
    OUT.write_text(text, "utf-8")
    print(f"{OUT.name} yazıldı.")


if __name__ == "__main__":
    main()
