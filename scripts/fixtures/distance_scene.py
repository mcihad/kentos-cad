#!/usr/bin/env python3
"""Uzaklık ve maliyet'in resimlerinin çizimi (docs/adr/0236): paylaşılan vadinin yükseklik modeli (fixtures/interaction/v1/
rasters.kcad, rasters/dem.tif), ondan türetilen bir maliyet rasteri (distance/maliyet.tif), beş köy, bir başlangıç, iki varış
ve vadinin güneyinden geçen mevcut yol.

    python3 scripts/fixtures/distance_scene.py          # fixtures/interaction/v1/distance.kcad'i ve distance/maliyet.tif'i yazar
    python3 scripts/fixtures/distance_scene.py --check  # hiçbir şey yazmaz; karşılaştırır

KentOS kodu kullanılmaz. Maliyet metre başına 1 + (e / 20)²'dir, e yükseklik modelinin σ = 3 hücrelik Gauss süzgeciyle
yumuşatılmış hâlinin (değersiz köşe ağırlıksız) merkezi farklarla yüzde eğimi (numpy'ın gradient'i, kenarda tek yanlı;
yükseklik modelinin gürültüsü maliyete geçmesin diye): düzde 1, %40'ta 5, %100'de 26. Yükseklik modelinin değersiz köşesi ve 850 m'nin altında kalan
batıdaki çukur (göl) değersizdir: geçilmez. Köyler, yol ve varışlar elle seçilmiştir.
"""

import json
import math
import sys
from pathlib import Path

import numpy as np
from osgeo import gdal

gdal.UseExceptions()

ROOT = Path(__file__).resolve().parents[2]
VALLEY = ROOT / "fixtures/interaction/v1/rasters.kcad"
DEM = ROOT / "fixtures/interaction/v1/rasters/dem.tif"
OUT = ROOT / "fixtures/interaction/v1/distance.kcad"
COST = ROOT / "fixtures/interaction/v1/distance/maliyet.tif"
LAKE = 850.0

VILLAGES = [("Köy A", 487296.0, 4420150.0), ("Köy B", 487904.0, 4420480.0), ("Köy C", 487600.0, 4420560.0),
            ("Köy D", 487750.0, 4420180.0), ("Köy E", 487380.0, 4420400.0)]
# The road along the valley's south.
ROAD = [(487210.0, 4420230.0), (487380.0, 4420210.0), (487560.0, 4420250.0), (487760.0, 4420200.0), (487960.0, 4420240.0)]


def smoothed(z, sigma):
    """A separable Gaussian blur of `z` (its edges repeated), radius 3σ."""
    r = int(3 * sigma)
    k = np.exp(-(np.arange(-r, r + 1) ** 2) / (2.0 * sigma * sigma))
    k /= k.sum()
    p = np.pad(z, r, mode="edge")
    rows = np.apply_along_axis(lambda v: np.convolve(v, k, mode="valid"), 1, p)
    return np.apply_along_axis(lambda v: np.convolve(v, k, mode="valid"), 0, rows)


def cost():
    """The cost raster's values (NaN: none) on the elevation model's grid."""
    ds = gdal.Open(str(DEM))
    band = ds.GetRasterBand(1)
    z = band.ReadAsArray().astype(np.float64)
    nodata = band.GetNoDataValue()
    empty = (z == nodata) if nodata is not None else np.zeros(z.shape, bool)
    cell = ds.GetGeoTransform()[1]
    # The empty corner weighs nothing in the blur (normalized convolution), emptied again after.
    w = (~empty).astype(np.float64)
    smooth = smoothed(np.where(empty, 0.0, z), 3.0) / np.maximum(smoothed(w, 3.0), 1e-12)
    gy, gx = np.gradient(np.where(empty, np.nan, smooth), cell)
    slope = np.hypot(gx, gy) * 100.0
    c = 1.0 + (slope / 20.0) ** 2
    c[empty | ~np.isfinite(c) | (z < LAKE)] = np.nan
    return ds.GetGeoTransform(), c.astype(np.float32)


def write_cost(path, affine, c):
    path.parent.mkdir(parents=True, exist_ok=True)
    ds = gdal.GetDriverByName("GTiff").Create(str(path), c.shape[1], c.shape[0], 1, gdal.GDT_Float32, ["COMPRESS=DEFLATE", "TILED=YES"])
    ds.SetGeoTransform(affine)
    band = ds.GetRasterBand(1)
    band.SetNoDataValue(float("nan"))
    band.WriteArray(c)
    ds.FlushCache()
    ds = None


def layer(id, name, color):
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def drawing(affine, shape):
    valley = json.loads(VALLEY.read_text("utf-8"))
    dem = next(e for e in valley["entities"] if e["layerId"] == "dem")
    entities = []

    def add(e, lid, attrs=None):
        entities.append({**e, "id": len(entities) + 1, "layerId": lid, "attrs": attrs or {}})

    point = lambda x, y: {"kind": "point", "p": {"x": x, "y": y}}
    for name, x, y in VILLAGES:
        add(point(x, y), "koyler", {"Ad": name})
    add(point(*VILLAGES[0][1:]), "baslangic", {"Ad": VILLAGES[0][0]})
    for name, x, y in VILLAGES[1:3]:
        add(point(x, y), "varis", {"Ad": name})
    add(point(*VILLAGES[1][1:]), "ikinci", {"Ad": VILLAGES[1][0]})
    add({"kind": "polyline", "pts": [{"x": x, "y": y} for x, y in ROAD]}, "yol", {"Ad": "Mevcut yol"})
    add({"kind": "raster", "affine": list(affine), "width": shape[1], "height": shape[0], "bands": 1, "sample": "f32",
         "file": "distance/maliyet.tif", "srid": dem["srid"],
         "style": {"render": "ramp", "bands": [1], "stretch": "percent", "ramp": "Sıcaklık"}}, "maliyet")
    add({k: v for k, v in dem.items() if k not in ("id", "layerId", "attrs")}, "dem", dem.get("attrs"))
    dem_layer = next(l for l in valley["layers"] if l["id"] == "dem")
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Güzergâh",
        "settings": valley["settings"],
        "origin": valley["origin"],
        "layers": [layer("koyler", "Köyler", "#C62828"), layer("baslangic", "Başlangıç", "#2E7D32"), layer("varis", "Varış", "#1565C0"),
                   layer("ikinci", "Koridorun ucu", "#6A1B9A"), layer("yol", "Mevcut yol", "#1F2933"),
                   layer("maliyet", "Maliyet", "#8D6E63"), dem_layer],
        "activeLayer": "maliyet",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }


def main():
    affine, c = cost()
    text = json.dumps(drawing(affine, c.shape), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        problems = []
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            problems.append(f"{OUT.name} güncel değil")
        if not COST.exists():
            problems.append(f"{COST} yok")
        else:
            ds = gdal.Open(str(COST))
            got = ds.GetRasterBand(1).ReadAsArray()
            same = got.shape == c.shape and all(
                (math.isnan(a) and math.isnan(b)) or a == b for a, b in zip(got.ravel().tolist(), c.ravel().tolist()))
            if list(ds.GetGeoTransform()) != list(affine) or not same:
                problems.append(f"{COST.name} yükseklik modelinin maliyeti değil")
        if problems:
            print("\n".join(problems) + f"\nYeniden yazın: python3 {sys.argv[0]}; farkı okuyun.")
            sys.exit(1)
        print(f"{OUT.name}, {COST.name}: güncel.")
        return
    write_cost(COST, affine, c)
    OUT.write_text(text, "utf-8")
    print(f"{OUT.name} ve {COST.name} yazıldı.")


if __name__ == "__main__":
    main()
