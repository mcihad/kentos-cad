#!/usr/bin/env python3
"""Raster layers' scene (docs/adr/0204): a small, made-up valley for the screenshots and the shared traces.

fixtures/interaction/v1/rasters/:
- dem.tif: a digital elevation model, 480 × 324 pixels of 1.6 m, 32-bit float (heights to a decimetre), tiled 256,
  Deflate with the floating-point predictor, nodata −9999 in a corner; hills, a ridge and a valley with a stream;
- orto.tif: an orthophoto of the same valley, 1280 × 864 pixels of 0.6 m, RGB, JPEG in YCbCr, tiled 256, with its
  overviews; fields, a forest on the steep slopes, the stream, a road and a hamlet, lit by the relief;
- tarama.png and tarama.pgw: a scanned sheet's corner (grey, a grid and a road), placed by its world file.
fixtures/interaction/v1/rasters.kcad: a GIS project in TUREF / TM30 with the three rasters on their layers and parcels
along the road over them.

Everything comes from fixed seeds: running it again writes the same bytes (GDAL's JPEG aside, which is libjpeg's).

    python3 scripts/fixtures/raster_scene.py          # write
    python3 scripts/fixtures/raster_scene.py --check  # compare the drawing
"""

import argparse
import json
import math
import sys
from pathlib import Path

import numpy as np
from osgeo import gdal, osr

gdal.UseExceptions()

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures" / "interaction" / "v1"
OUT = DIR / "rasters"
X0, Y0 = 487200.0, 4420600.0
SRID = 5256


def srs():
    s = osr.SpatialReference()
    s.ImportFromEPSG(SRID)
    return s.ExportToWkt()


def blur(a, r, times=3):
    """A box blur of radius r (pixels) `times` over, both ways: about a Gaussian, no blocks."""
    for _ in range(times):
        for axis in (0, 1):
            pad = [(0, 0), (0, 0)]
            pad[axis] = (r + 1, r)
            c = np.cumsum(np.pad(a, pad, mode="edge"), axis=axis)
            hi = np.take(c, np.arange(2 * r + 1, c.shape[axis]), axis=axis)
            lo = np.take(c, np.arange(0, c.shape[axis] - 2 * r - 1), axis=axis)
            a = (hi - lo) / (2 * r + 1)
    return a


def terrain(w, h, px):
    """Heights over a w × h grid of px-metre pixels: hills, a ridge, a valley along a winding line."""
    rng = np.random.default_rng(204)
    y, x = np.mgrid[0:h, 0:w].astype(np.float64)
    x *= px
    y *= px
    z = 880.0 + 0.06 * x - 0.04 * y
    for _ in range(9):
        cx, cy = rng.uniform(0, w * px), rng.uniform(0, h * px)
        r, a = rng.uniform(60, 180), rng.uniform(12, 38)
        z += a * np.exp(-((x - cx) ** 2 + (y - cy) ** 2) / (2 * r * r))
    # A ridge from the north-west.
    z += 42.0 * np.exp(-(((x - 0.9 * y) - 160.0) ** 2) / (2 * 70.0**2)) * np.exp(-(y / 420.0))
    # The valley: a winding line across, its floor a stream.
    mid = 0.55 * h * px + 40.0 * np.sin(x / 95.0) + 18.0 * np.sin(x / 31.0)
    d = y - mid
    z -= 34.0 * np.exp(-(d**2) / (2 * 38.0**2))
    # A little roughness of about ten metres, smoothed (the same on every grid: drawn from a 2 m grid).
    step = max(1, int(round(2.0 / px)))
    n = rng.normal(0, 1.0, (h // step + 2, w // step + 2))
    k = blur(np.kron(n, np.ones((step, step)))[:h, :w], max(1, int(round(4.0 / px))))
    z += 9.0 * k
    return z, mid


def hillshade(z, px):
    gy, gx = np.gradient(z, px)
    slope = np.arctan(np.hypot(gx, gy))
    aspect = np.arctan2(-gx, gy)
    az, alt = math.radians(315.0), math.radians(45.0)
    s = np.sin(alt) * np.cos(slope) + np.cos(alt) * np.sin(slope) * np.cos(az - aspect)
    return np.clip(s, 0, 1), slope


def write_dem():
    w, h, px = 480, 324, 1.6
    z, _ = terrain(w, h, px)
    z = (np.round(z * 10.0) / 10.0).astype(np.float32)
    z[:30, :42] = -9999.0
    ds = gdal.GetDriverByName("GTiff").Create(
        str(OUT / "dem.tif"), w, h, 1, gdal.GDT_Float32,
        ["TILED=YES", "BLOCKXSIZE=256", "BLOCKYSIZE=256", "COMPRESS=DEFLATE", "PREDICTOR=3"],
    )
    ds.SetGeoTransform((X0, px, 0.0, Y0, 0.0, -px))
    ds.SetProjection(srs())
    b = ds.GetRasterBand(1)
    b.SetNoDataValue(-9999.0)
    b.WriteArray(z)
    ds = None


def road_mask(x, y, mid):
    """The road: above the stream, along the valley, and a lane up to the hamlet."""
    road = np.abs(y - (mid - 46.0 + 6.0 * np.sin(x / 70.0))) < 3.2
    lane = (np.abs(x - 470.0 - 0.15 * y) < 2.4) & (y < mid - 40.0) & (y > 90.0)
    return road | lane


def write_orto():
    w, h, px = 1280, 864, 0.6
    rng = np.random.default_rng(1204)
    z, mid = terrain(w, h, px)
    shade, slope = hillshade(z, px)
    y, x = np.mgrid[0:h, 0:w].astype(np.float64)
    x *= px
    y *= px
    # Fields: rectangles of crops in rows, turned a little.
    u = (x * 0.98 + y * 0.2) / 46.0
    v = (y * 0.98 - x * 0.2) / 31.0
    cell = (np.floor(u).astype(int) * 7919 + np.floor(v).astype(int) * 104729) % 97
    palette = np.array(
        [[122, 145, 72], [168, 160, 96], [96, 128, 60], [190, 176, 120], [136, 150, 90], [150, 120, 80]],
        dtype=np.float64,
    )
    col = palette[cell % len(palette)]
    furrow = 0.92 + 0.08 * np.sin((u - np.floor(u)) * 40.0)
    col *= furrow[..., None]
    # Forest on the steep slopes and the ridge: crowns of a few pixels, not a pixel's noise.
    forest = (slope > 0.32) | (z > z.mean() + 34.0)
    crowns = rng.normal(0, 1.0, (h // 4 + 1, w // 4 + 1))
    grain = blur(np.kron(crowns, np.ones((4, 4)))[:h, :w], 2, 2)
    grain /= max(grain.std(), 1e-9)
    tree = np.array([58, 84, 48], dtype=np.float64) * (0.9 + 0.1 * grain[..., None])
    col = np.where(forest[..., None], tree, col)
    # The stream and its banks.
    d = np.abs(y - mid)
    col = np.where((d < 3.0)[..., None], np.array([70, 104, 130.0]), col)
    col = np.where(((d >= 3.0) & (d < 7.0))[..., None], np.array([96, 120, 74.0]), col)
    # The road and the lane.
    col = np.where(road_mask(x, y, mid)[..., None], np.array([150, 148, 140.0]), col)
    # A hamlet: roofs beside the lane.
    for k in range(9):
        bx = 455.0 + (k % 3) * 22.0 + rng.uniform(-3, 3)
        by = 140.0 + (k // 3) * 26.0 + rng.uniform(-3, 3)
        roof = (np.abs(x - bx) < 6.5) & (np.abs(y - by) < 4.5)
        tone = np.array([[168, 82, 62], [150, 150, 150], [182, 98, 70]][k % 3], dtype=np.float64)
        col = np.where(roof[..., None], tone * (0.9 + 0.1 * (x > bx)[..., None]), col)
    col *= (0.55 + 0.55 * shade)[..., None]
    rgb = np.clip(col, 0, 255).astype(np.uint8)
    mem = gdal.GetDriverByName("MEM").Create("", w, h, 3, gdal.GDT_Byte)
    mem.SetGeoTransform((X0, px, 0.0, Y0, 0.0, -px))
    mem.SetProjection(srs())
    for i in range(3):
        mem.GetRasterBand(i + 1).WriteArray(rgb[..., i])
    mem.BuildOverviews("AVERAGE", [2, 4])
    gdal.Translate(
        str(OUT / "orto.tif"), mem,
        creationOptions=["TILED=YES", "BLOCKXSIZE=256", "BLOCKYSIZE=256", "COMPRESS=JPEG", "PHOTOMETRIC=YCBCR",
                         "JPEG_QUALITY=85", "COPY_SRC_OVERVIEWS=YES"],
    )


def write_scan():
    """A scanned sheet's corner: paper grey, a 50 m grid, the road drawn, 1.0 m a pixel, west of the valley."""
    w, h = 300, 260
    rng = np.random.default_rng(2041)
    img = np.full((h, w), 228.0) + rng.normal(0, 3.0, (h, w))
    for g in range(0, w, 50):
        img[:, g:g + 2] = 120.0
    for g in range(0, h, 50):
        img[g:g + 2, :] = 120.0
    yy, xx = np.mgrid[0:h, 0:w]
    road = np.abs(yy - (150 + 20 * np.sin(xx / 60.0))) < 2.5
    img[road] = 70.0
    gray = np.clip(img, 0, 255).astype(np.uint8)
    ds = gdal.GetDriverByName("MEM").Create("", w, h, 1, gdal.GDT_Byte)
    ds.GetRasterBand(1).WriteArray(gray)
    gdal.Translate(str(OUT / "tarama.png"), ds, format="PNG")
    # Its world file: the upper left pixel's centre, a metre a pixel (as a world file says it).
    ox, oy = X0 - 330.0, Y0 - 40.0
    (OUT / "tarama.pgw").write_text(f"1.0\n0.0\n0.0\n-1.0\n{ox + 0.5}\n{oy - 0.5}\n")
    aux = OUT / "tarama.png.aux.xml"
    if aux.exists():
        aux.unlink()


def drawing():
    """The scene's GIS project: the three rasters on their layers, parcels along the road over the photograph."""
    layer = lambda id_, name, color: {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False,
                                      "expanded": True, "style": {"color": color, "lineType": "continuous",
                                                                  "lineWeight": 0.25}, "children": []}
    rgb = {"render": "rgb", "bands": [1, 2, 3]}
    entities = [
        {"kind": "raster", "id": 1, "layerId": "dem", "attrs": {}, "affine": [X0, 1.6, 0.0, Y0, 0.0, -1.6],
         "width": 480, "height": 324, "bands": 1, "sample": "f32", "file": "rasters/dem.tif", "srid": SRID,
         "style": {"render": "rampShade", "bands": [1], "stretch": "minMax", "ramp": "Arazi"}},
        {"kind": "raster", "id": 2, "layerId": "orto", "attrs": {}, "affine": [X0, 0.6, 0.0, Y0, 0.0, -0.6],
         "width": 1280, "height": 864, "bands": 3, "sample": "u8", "file": "rasters/orto.tif", "srid": SRID,
         "style": rgb},
        {"kind": "raster", "id": 3, "layerId": "tarama", "attrs": {}, "affine": [X0 - 330.0, 1.0, 0.0, Y0 - 40.0, 0.0, -1.0],
         "width": 300, "height": 260, "bands": 1, "sample": "u8", "file": "rasters/tarama.png", "srid": 0,
         "style": {"render": "gray", "bands": [1]}},
    ]
    # Parcels north of the road, west to east.
    for k in range(5):
        a = X0 + 120.0 + k * 62.0
        b = a + 58.0
        y1, y2 = Y0 - 150.0, Y0 - 228.0
        entities.append({"kind": "polygon", "id": len(entities) + 1, "layerId": "parsel",
                         "attrs": {"Parsel": f"{104 + k}"},
                         "pts": [{"x": a, "y": y1}, {"x": b, "y": y1}, {"x": b + 6.0, "y": y2}, {"x": a + 6.0, "y": y2}]})
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Vadi",
        "settings": {"srid": SRID, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 1000.0, "workspace": "gis"},
        "origin": {"x": X0 + 380.0, "y": Y0 - 260.0},
        # The list's top is drawn last: the parcels over the photograph over the relief.
        "layers": [layer("parsel", "Parsel", "#E8B04A"), layer("orto", "Ortofoto", "#5B6B7F"),
                   layer("dem", "Yükseklik modeli", "#7A6B5B"), layer("tarama", "Taranmış pafta", "#6B5B7F"),
                   layer("0", "0", "fg")],
        "activeLayer": "parsel",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    text = json.dumps(drawing(), ensure_ascii=False, indent=1) + "\n"
    target = DIR / "rasters.kcad"
    if args.check:
        if target.read_text("utf-8") != text:
            print("rasters.kcad farklı; yeniden yazın.", file=sys.stderr)
            sys.exit(1)
        print("rasters.kcad güncel.")
        return
    OUT.mkdir(parents=True, exist_ok=True)
    write_dem()
    write_orto()
    write_scan()
    target.write_text(text, "utf-8")
    sizes = {p.name: p.stat().st_size for p in sorted(OUT.iterdir())}
    print(f"yazıldı: {sizes}")


if __name__ == "__main__":
    main()
