#!/usr/bin/env python3
"""A multi-gigabyte orthophoto as a COG for the first-view measurement (docs/adr/0642 §5): 65 536 × 65 536 RGB pixels,
JPEG tiles of 256 with overviews, written by GDAL's COG driver into .run/perf/orto-buyuk.tif (made once; kept).
Its pixels are a 1024 × 1024 tile of noise and gradients laid 64 × 64 times through a VRT, so no tile compresses away.

    /usr/bin/python3 scripts/perf/big_cog.py
"""
import sys
from pathlib import Path

import numpy as np
from osgeo import gdal

gdal.UseExceptions()
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / ".run/perf/orto-buyuk.tif"
SIDE, TIMES = 1024, 64


def main():
    if OUT.exists():
        print(f"{OUT} var ({OUT.stat().st_size / 1e9:.2f} GB).")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    tile = OUT.with_name("orto-parca.tif")
    rng = np.random.default_rng(7)
    y, x = np.mgrid[0:SIDE, 0:SIDE]
    ds = gdal.GetDriverByName("GTiff").Create(str(tile), SIDE, SIDE, 3, gdal.GDT_Byte)
    for b in range(3):
        noise = rng.integers(0, 96, (SIDE, SIDE))
        ds.GetRasterBand(b + 1).WriteArray(((x * (b + 1) + y * (3 - b)) // 9 % 160 + noise).astype(np.uint8))
    ds = None
    vrt = OUT.with_name("orto-buyuk.vrt")
    sources = []
    for i in range(TIMES):
        for j in range(TIMES):
            sources.append((i, j))
    bands = []
    for b in range(3):
        items = "".join(
            f'<SimpleSource><SourceFilename relativeToVRT="1">{tile.name}</SourceFilename><SourceBand>{b + 1}</SourceBand>'
            f'<SrcRect xOff="0" yOff="0" xSize="{SIDE}" ySize="{SIDE}"/><DstRect xOff="{j * SIDE}" yOff="{i * SIDE}" xSize="{SIDE}" ySize="{SIDE}"/></SimpleSource>'
            for i, j in sources
        )
        bands.append(f'<VRTRasterBand dataType="Byte" band="{b + 1}">{items}</VRTRasterBand>')
    side = SIDE * TIMES
    vrt.write_text(
        f'<VRTDataset rasterXSize="{side}" rasterYSize="{side}"><SRS>EPSG:5254</SRS>'
        f"<GeoTransform>500000, 0.1, 0, 4500000, 0, -0.1</GeoTransform>{''.join(bands)}</VRTDataset>"
    )
    gdal.SetConfigOption("GDAL_NUM_THREADS", "ALL_CPUS")
    gdal.Translate(str(OUT), str(vrt), format="COG", creationOptions=["COMPRESS=JPEG", "QUALITY=95", "BLOCKSIZE=256", "NUM_THREADS=ALL_CPUS", "BIGTIFF=YES"])
    vrt.unlink()
    tile.unlink()
    print(f"{OUT} yazıldı ({OUT.stat().st_size / 1e9:.2f} GB).")


if __name__ == "__main__":
    sys.exit(main())
