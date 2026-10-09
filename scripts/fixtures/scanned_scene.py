#!/usr/bin/env python3
"""A scanned sheet's scene (docs/adr/0234): a made-up topographic sheet for the screenshots of Çizgi yakala, Alan kapat,
Rasterden çizgi and Eğrilere kot ver.

fixtures/interaction/v1/scanned/pafta.tif: 1200 × 900 pixels of 0.5 m, RGB, tiled 256 and Deflate: cream paper with
the scanner's noise, brown contour lines every 5 m (every 25 m thicker) of the valley's terrain, a blue stream, a road
drawn twice and a block of six parcels in black.
fixtures/interaction/v1/scanned.kcad: a GIS project in TUREF / TM33 with the sheet on its layer and empty layers for
what is digitized.

Everything comes from fixed seeds: running it again writes the same bytes.

    python3 scripts/fixtures/scanned_scene.py          # write
    python3 scripts/fixtures/scanned_scene.py --check  # compare the drawing and the sheet's pixels
"""

import argparse
import json
import sys
from pathlib import Path

import numpy as np
from osgeo import gdal, osr

sys.path.insert(0, str(Path(__file__).resolve().parent))
import raster_scene  # noqa: E402  (the valley's terrain)

gdal.UseExceptions()

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures" / "interaction" / "v1"
OUT = DIR / "scanned"
X0, Y0 = 486900.0, 4421300.0
PX = 0.5
W, H = 1200, 900
SRID = 5256

PAPER = (244.0, 238.0, 222.0)
BROWN = (156.0, 92.0, 44.0)
BLUE = (58.0, 112.0, 192.0)
BLACK = (34.0, 32.0, 30.0)

# The parcels' corners, metres from the sheet's upper left (east, south).
PARCELS = [
    [(330, 250), (395, 248), (398, 300), (333, 304)],
    [(395, 248), (452, 246), (456, 296), (398, 300)],
    [(333, 304), (398, 300), (402, 352), (336, 357)],
    [(398, 300), (456, 296), (460, 348), (402, 352)],
    [(452, 246), (520, 243), (526, 293), (456, 296)],
    [(456, 296), (526, 293), (531, 344), (460, 348)],
]


def segment_mask(mask, a, b, width):
    """Marks the pixels whose centres are within width / 2 of the segment a–b (pixels)."""
    r = width / 2.0
    x0, x1 = int(max(min(a[0], b[0]) - r - 1, 0)), int(min(max(a[0], b[0]) + r + 2, W))
    y0, y1 = int(max(min(a[1], b[1]) - r - 1, 0)), int(min(max(a[1], b[1]) + r + 2, H))
    if x0 >= x1 or y0 >= y1:
        return
    yy, xx = np.mgrid[y0:y1, x0:x1].astype(np.float64)
    px, py = xx + 0.5, yy + 0.5
    dx, dy = b[0] - a[0], b[1] - a[1]
    n = dx * dx + dy * dy
    t = np.clip(((px - a[0]) * dx + (py - a[1]) * dy) / n, 0, 1) if n > 0 else 0
    d = np.hypot(px - (a[0] + t * dx), py - (a[1] + t * dy))
    mask[y0:y1, x0:x1] |= d <= r


def polyline_mask(pts, width):
    mask = np.zeros((H, W), dtype=bool)
    for a, b in zip(pts, pts[1:]):
        segment_mask(mask, a, b, width)
    return mask


def dilate(mask, times):
    for _ in range(times):
        m = mask.copy()
        m[1:, :] |= mask[:-1, :]
        m[:-1, :] |= mask[1:, :]
        m[:, 1:] |= mask[:, :-1]
        m[:, :-1] |= mask[:, 1:]
        mask = m
    return mask


def sheet():
    rng = np.random.default_rng(2342)
    z, _ = raster_scene.terrain(W, H, PX)
    level = np.floor(z / 5.0)
    edge = np.zeros((H, W), dtype=bool)
    edge[:, :-1] |= level[:, :-1] != level[:, 1:]
    edge[:-1, :] |= level[:-1, :] != level[1:, :]
    index = np.zeros((H, W), dtype=bool)
    big = np.floor(z / 25.0)
    index[:, :-1] |= big[:, :-1] != big[:, 1:]
    index[:-1, :] |= big[:-1, :] != big[1:, :]
    contours = dilate(edge, 1) | dilate(index, 2)
    xs = np.arange(0, W + 1, 8, dtype=np.float64)
    stream = polyline_mask([(x, 660 + 70 * np.sin(x / 170.0) + 18 * np.sin(x / 47.0)) for x in xs], 5.0)
    road_mid = [(x, 120 + 40 * np.sin(x / 260.0)) for x in xs]
    road = polyline_mask([(x, y - 6) for x, y in road_mid], 2.6) | polyline_mask([(x, y + 6) for x, y in road_mid], 2.6)
    parcels = np.zeros((H, W), dtype=bool)
    for p in PARCELS:
        ring = [(x / PX, y / PX) for x, y in p] + [(p[0][0] / PX, p[0][1] / PX)]
        parcels |= polyline_mask(ring, 3.0)
    img = np.empty((H, W, 3))
    img[...] = PAPER
    for mask, colour in ((contours, BROWN), (stream, BLUE), (road, BLACK), (parcels, BLACK)):
        img[mask] = colour
    img += rng.normal(0, 3.5, (H, W, 1))
    return np.clip(np.round(img), 0, 255).astype(np.uint8)


def write_sheet(img):
    OUT.mkdir(parents=True, exist_ok=True)
    mem = gdal.GetDriverByName("MEM").Create("", W, H, 3, gdal.GDT_Byte)
    mem.SetGeoTransform([X0, PX, 0.0, Y0, 0.0, -PX])
    s = osr.SpatialReference()
    s.ImportFromEPSG(SRID)
    mem.SetProjection(s.ExportToWkt())
    for k in range(3):
        mem.GetRasterBand(k + 1).WriteArray(img[:, :, k])
    mem.GetRasterBand(1).SetColorInterpretation(gdal.GCI_RedBand)
    mem.GetRasterBand(2).SetColorInterpretation(gdal.GCI_GreenBand)
    mem.GetRasterBand(3).SetColorInterpretation(gdal.GCI_BlueBand)
    gdal.Translate(str(OUT / "pafta.tif"), mem, format="GTiff",
                   creationOptions=["TILED=YES", "BLOCKXSIZE=256", "BLOCKYSIZE=256", "COMPRESS=DEFLATE", "PHOTOMETRIC=RGB"])


def drawing():
    layer = lambda id_, name, color: {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
                                      "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25}, "children": []}
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Taranmış pafta",
        "settings": {"srid": SRID, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000.0,
                     "workspace": "gis"},
        "origin": {"x": X0, "y": Y0 - H * PX},
        "layers": [layer("pafta", "Taranmış pafta", "#6B5B7F"), layer("egriler", "Eğriler", "#A0522D"), layer("0", "0", "#FFFFFF")],
        "activeLayer": "0",
        "entities": [{"kind": "raster", "id": 1, "layerId": "pafta", "attrs": {}, "affine": [X0, PX, 0.0, Y0, 0.0, -PX], "width": W,
                      "height": H, "bands": 3, "sample": "u8", "file": "scanned/pafta.tif", "srid": SRID,
                      "style": {"render": "rgb", "bands": [1, 2, 3]}}],
        "styles": {"items": [], "categories": []},
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    check = ap.parse_args().check
    img = sheet()
    text = json.dumps(drawing(), ensure_ascii=False, indent=1) + "\n"
    path = DIR / "scanned.kcad"
    if check:
        ds = gdal.Open(str(OUT / "pafta.tif"))
        got = np.stack([ds.GetRasterBand(k + 1).ReadAsArray() for k in range(3)], axis=-1)
        if not path.exists() or path.read_text("utf-8") != text or not np.array_equal(got, img):
            sys.exit(f"Taranmış pafta güncel değil: python3 {sys.argv[0]} ile yeniden yazın.")
        print("scanned.kcad ve scanned/pafta.tif güncel.")
        return
    write_sheet(img)
    path.write_text(text, "utf-8")
    print(f"{path} ve {OUT / 'pafta.tif'} yazıldı.")


if __name__ == "__main__":
    main()
