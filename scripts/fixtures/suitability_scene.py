#!/usr/bin/env python3
"""Uygunluk analizi'nin resimlerinin çizimi (docs/adr/0237): paylaşılan vadinin yükseklik modeli (fixtures/interaction/v1/
rasters.kcad, rasters/dem.tif), ondan türetilen üç ölçüt (suitability/egim.tif, yol.tif, ortu.tif), vadinin güneyinden geçen
mevcut yol ve dik yamaçlardaki heyelan noktaları.

    python3 scripts/fixtures/suitability_scene.py          # fixtures/interaction/v1/suitability.kcad'i ve rasterlerini yazar
    python3 scripts/fixtures/suitability_scene.py --check  # hiçbir şey yazmaz; karşılaştırır

KentOS kodu kullanılmaz.
- Eğim: yükseklik modelinin σ = 3 hücrelik Gauss süzgeciyle yumuşatılmış hâlinin (değersiz köşe ağırlıksız) merkezi farklarla
  yüzde eğimi (numpy'ın gradient'i; distance_scene.py'deki gibi).
- Yola uzaklık: hücre merkezinin yolun doğru parçalarına en kısa düz uzaklığı, metre.
- Arazi örtüsü (bayt, 0 değersiz): 850 m'nin altı 5 (su), eğimi %35'ten dik 1 (orman), yola 40 m'den yakın 4 (yerleşim),
  1 000 m'nin üstü 2 (mera), kalanı 3 (tarım).
- Heyelan: eğimi %45'ten dik hücrelerden satır satır her 997. hücrenin merkezi (en çok 24 nokta).
Yükseklik modelinin değersiz köşesi üç ölçütte de değersizdir.
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
OUT = ROOT / "fixtures/interaction/v1/suitability.kcad"
DIR = ROOT / "fixtures/interaction/v1/suitability"
LAKE = 850.0
HIGH = 1000.0
# The road along the valley's south (distance_scene.py's).
ROAD = [(487210.0, 4420230.0), (487380.0, 4420210.0), (487560.0, 4420250.0), (487760.0, 4420200.0), (487960.0, 4420240.0)]


def smoothed(z, sigma):
    """A separable Gaussian blur of `z` (its edges repeated), radius 3σ."""
    r = int(3 * sigma)
    k = np.exp(-(np.arange(-r, r + 1) ** 2) / (2.0 * sigma * sigma))
    k /= k.sum()
    p = np.pad(z, r, mode="edge")
    rows = np.apply_along_axis(lambda v: np.convolve(v, k, mode="valid"), 1, p)
    return np.apply_along_axis(lambda v: np.convolve(v, k, mode="valid"), 0, rows)


def criteria():
    """The grid's affine, the empty corner, and the three criteria."""
    ds = gdal.Open(str(DEM))
    band = ds.GetRasterBand(1)
    z = band.ReadAsArray().astype(np.float64)
    nodata = band.GetNoDataValue()
    empty = (z == nodata) if nodata is not None else np.zeros(z.shape, bool)
    affine = ds.GetGeoTransform()
    cell = affine[1]
    w = (~empty).astype(np.float64)
    smooth = smoothed(np.where(empty, 0.0, z), 3.0) / np.maximum(smoothed(w, 3.0), 1e-12)
    gy, gx = np.gradient(np.where(empty, np.nan, smooth), cell)
    slope = np.hypot(gx, gy) * 100.0
    slope[empty | ~np.isfinite(slope)] = np.nan
    h, wd = z.shape
    xs = affine[0] + (np.arange(wd) + 0.5) * affine[1]
    ys = affine[3] + (np.arange(h) + 0.5) * affine[5]
    X, Y = np.meshgrid(xs, ys)
    road = np.full(z.shape, np.inf)
    for (ax, ay), (bx, by) in zip(ROAD, ROAD[1:]):
        dx, dy = bx - ax, by - ay
        t = np.clip(((X - ax) * dx + (Y - ay) * dy) / (dx * dx + dy * dy), 0.0, 1.0)
        road = np.minimum(road, np.hypot(X - (ax + t * dx), Y - (ay + t * dy)))
    road[empty] = np.nan
    cover = np.full(z.shape, 3, np.uint8)
    cover[z > HIGH] = 2
    cover[road < 40.0] = 4
    cover[slope > 35.0] = 1
    cover[z < LAKE] = 5
    cover[empty] = 0
    return affine, empty, slope.astype(np.float32), road.astype(np.float32), cover


def slides(affine, slope):
    """Every 997th cell steeper than 45 %, row by row: its centre."""
    out = []
    flat = np.argwhere(np.nan_to_num(slope, nan=0.0) > 45.0)
    for j, i in flat[::997][:24]:
        out.append((affine[0] + (i + 0.5) * affine[1], affine[3] + (j + 0.5) * affine[5]))
    return out


def write(path, affine, a, nodata):
    path.parent.mkdir(parents=True, exist_ok=True)
    kind = gdal.GDT_Byte if a.dtype == np.uint8 else gdal.GDT_Float32
    ds = gdal.GetDriverByName("GTiff").Create(str(path), a.shape[1], a.shape[0], 1, kind, ["COMPRESS=DEFLATE", "TILED=YES"])
    ds.SetGeoTransform(affine)
    band = ds.GetRasterBand(1)
    band.SetNoDataValue(nodata)
    band.WriteArray(a)
    ds.FlushCache()
    ds = None


def layer(id, name, color):
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25}, "children": []}


FILES = [("egim.tif", "egim", "Eğim", {"render": "ramp", "bands": [1], "stretch": "percent", "ramp": "Sıcaklık"}),
         ("yol.tif", "yol-uzakligi", "Yola uzaklık", {"render": "ramp", "bands": [1], "stretch": "minMax", "ramp": "Viridis"}),
         ("ortu.tif", "ortu", "Arazi örtüsü",
          {"render": "ramp", "bands": [1], "stretch": "manual", "min": 1.0, "max": 5.0, "ramp": "Spektral", "resampling": "nearest"})]


def drawing(affine, shape, points):
    valley = json.loads(VALLEY.read_text("utf-8"))
    dem = next(e for e in valley["entities"] if e["layerId"] == "dem")
    entities = []

    def add(e, lid, attrs=None):
        entities.append({**e, "id": len(entities) + 1, "layerId": lid, "attrs": attrs or {}})

    for k, (x, y) in enumerate(points):
        add({"kind": "point", "p": {"x": x, "y": y}}, "heyelan", {"Ad": f"H{k + 1}"})
    add({"kind": "polyline", "pts": [{"x": x, "y": y} for x, y in ROAD]}, "yol", {"Ad": "Mevcut yol"})
    sample = {"egim": "f32", "yol-uzakligi": "f32", "ortu": "u8"}
    for file, lid, _, style in FILES:
        add({"kind": "raster", "affine": list(affine), "width": shape[1], "height": shape[0], "bands": 1, "sample": sample[lid],
             "file": f"suitability/{file}", "srid": dem["srid"], "style": style}, lid)
    add({k: v for k, v in dem.items() if k not in ("id", "layerId", "attrs")}, "dem", dem.get("attrs"))
    dem_layer = next(l for l in valley["layers"] if l["id"] == "dem")
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Uygunluk",
        "settings": valley["settings"],
        "origin": valley["origin"],
        "layers": [layer("heyelan", "Heyelan", "#C62828"), layer("yol", "Mevcut yol", "#1F2933")]
        + [layer(lid, name, "#8D6E63") for _, lid, name, _ in FILES] + [dem_layer],
        "activeLayer": "egim",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }


def same(path, affine, a):
    ds = gdal.Open(str(path))
    got = ds.GetRasterBand(1).ReadAsArray()
    if got.shape != a.shape or list(ds.GetGeoTransform()) != list(affine):
        return False
    return all((math.isnan(x) and math.isnan(y)) or x == y for x, y in zip(got.ravel().tolist(), a.ravel().tolist()))


def main():
    affine, empty, slope, road, cover = criteria()
    points = slides(affine, slope)
    text = json.dumps(drawing(affine, slope.shape, points), ensure_ascii=False, indent=1) + "\n"
    rasters = [("egim.tif", slope, float("nan")), ("yol.tif", road, float("nan")), ("ortu.tif", cover, 0.0)]
    if "--check" in sys.argv[1:]:
        problems = []
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            problems.append(f"{OUT.name} güncel değil")
        for name, a, _ in rasters:
            path = DIR / name
            if not path.exists() or not same(path, affine, a):
                problems.append(f"{name} yükseklik modelinin ölçütü değil")
        if problems:
            print("\n".join(problems) + f"\nYeniden yazın: python3 {sys.argv[0]}; farkı okuyun.")
            sys.exit(1)
        print(f"{OUT.name} ve üç ölçüt: güncel.")
        return
    for name, a, nodata in rasters:
        write(DIR / name, affine, a, nodata)
    OUT.write_text(text, "utf-8")
    print(f"{OUT.name} ve üç ölçüt yazıldı; {len(points)} heyelan noktası.")


if __name__ == "__main__":
    main()
