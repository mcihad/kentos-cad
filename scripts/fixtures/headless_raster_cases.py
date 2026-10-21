#!/usr/bin/env python3
"""The headless server's rasters as GDAL reads them (docs/adr/0573): a JPEG picture, a JPEG-compressed TIFF and KentOS's
virtual mosaic (VRT), each one's first band summed up by GDAL, without KentOS code.

    python3 scripts/fixtures/headless_raster_cases.py --files   # write rgb.jpg (GDAL's JPEG of rgb.png) and the summary
    python3 scripts/fixtures/headless_raster_cases.py --check   # the summary is GDAL's still

The summary (fixtures/raster/v1/headless.json) holds, per file, its size and the first band's valid cells: count, mean,
least and most. python/tests/test_headless_rasters.py runs Bölgesel istatistik over each raster headless and holds its
numbers to these. A JPEG's pixels are the decoder's: GDAL's libjpeg and KentOS's zune-jpeg may differ by a unit here
and there, so the test holds the JPEGs' means to half a unit and their counts exactly.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np
from osgeo import gdal

gdal.UseExceptions()

ROOT = Path(__file__).resolve().parents[2]
FILES = ROOT / "fixtures/raster/v1/files"
MOSAIC = ROOT / "fixtures/raster/v1/mosaic"
OUT = ROOT / "fixtures/raster/v1/headless.json"

SOURCES = {
    "rgb.jpg": FILES / "rgb.jpg",
    "rgb-jpeg-ycbcr.tif": FILES / "rgb-jpeg-ycbcr.tif",
    "kentos-dem-verilen.vrt": MOSAIC / "kentos-dem-verilen.vrt",
}


def make_jpeg() -> None:
    gdal.Translate(str(FILES / "rgb.jpg"), str(FILES / "rgb.png"), format="JPEG",
                   creationOptions=["QUALITY=90", "WORLDFILE=NO"])
    aux = FILES / "rgb.jpg.aux.xml"
    if aux.exists():
        aux.unlink()


def summary() -> dict:
    out = {}
    for name, path in SOURCES.items():
        ds = gdal.Open(str(path))
        band = ds.GetRasterBand(1)
        a = band.ReadAsArray().astype(np.float64)
        nodata = band.GetNoDataValue()
        valid = a[a != nodata] if nodata is not None else a.ravel()
        out[name] = {
            "width": ds.RasterXSize,
            "height": ds.RasterYSize,
            "bands": ds.RasterCount,
            "affine": list(ds.GetGeoTransform()),
            "count": int(valid.size),
            "mean": float(valid.mean()),
            "min": float(valid.min()),
            "max": float(valid.max()),
        }
        ds = None
    return out


def main() -> None:
    if "--files" in sys.argv:
        make_jpeg()
    got = {"format": "kentos.headless-rasters", "version": 1, "files": summary()}
    text = json.dumps(got, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"{OUT.relative_to(ROOT)} güncel değil; --files ya da betiği --check'siz çalıştırın")
        print(f"{OUT.relative_to(ROOT)} güncel: {len(SOURCES)} dosya")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
