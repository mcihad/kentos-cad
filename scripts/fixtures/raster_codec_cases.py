#!/usr/bin/env python3
"""The raster codec profile (docs/adr/0642 §1): GeoTIFFs in the compressions a COG is written with today, written and read
by GDAL, without KentOS code.

fixtures/raster/v1/codecs/: ZSTD (with the horizontal and the floating point predictor, tiled and in strips), WebP
lossless (RGB and RGBA) and lossy, and the compressions KentOS does not read (LERC, LERC with ZSTD, LZMA).
fixtures/raster/v1/codecs.json: the profile (every TIFF compression GDAL writes, its code, and whether KentOS reads it)
and for each file what the window shows and its level 0 samples as GDAL reads them: an FNV-1a 64 hash of their
little-endian bytes (bands interleaved) for the lossless ones; band means and sampled pixels for lossy WebP (decoders
convert YUV to RGB differently, within a few steps); for the ones not read, the words the refusal says (the
compression's name and the way to convert it).

    /usr/bin/python3 scripts/fixtures/raster_codec_cases.py          # write the files and the cases
    /usr/bin/python3 scripts/fixtures/raster_codec_cases.py --check  # compare with what is on disk
"""

import argparse
import json
import sys
from pathlib import Path

import numpy as np
from osgeo import gdal

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.dont_write_bytecode = True
import raster_cases as R  # noqa: E402

ROOT = HERE.parents[1]
FILES = ROOT / "fixtures/raster/v1/codecs"
OUT = ROOT / "fixtures/raster/v1/codecs.json"

# Every TIFF compression GDAL's GTiff driver writes: its tag value, its name, whether KentOS reads it.
PROFILE = [
    {"code": 1, "name": "NONE", "read": True},
    {"code": 5, "name": "LZW", "read": True},
    {"code": 7, "name": "JPEG", "read": True},
    {"code": 8, "name": "DEFLATE", "read": True},
    {"code": 32773, "name": "PACKBITS", "read": True},
    {"code": 32946, "name": "DEFLATE", "read": True},
    {"code": 50000, "name": "ZSTD", "read": True},
    {"code": 50001, "name": "WEBP", "read": True},
    {"code": 2, "name": "CCITTRLE", "read": False},
    {"code": 3, "name": "CCITTFAX3", "read": False},
    {"code": 4, "name": "CCITTFAX4", "read": False},
    {"code": 34887, "name": "LERC", "read": False},
    {"code": 34925, "name": "LZMA", "read": False},
    {"code": 50002, "name": "JXL", "read": False},
]

# The refusal's words: the compression by name and the way out, a conversion to Deflate.
CONVERT = "gdal_translate -co TILED=YES -co COMPRESS=DEFLATE"


def files():
    """Writes every file; returns the cases' descriptions."""
    FILES.mkdir(parents=True, exist_ok=True)
    cases = []

    def tiff(name, kind, data, options, **kw):
        R.create(FILES / name, data, kind, options, geotransform=kw.pop("gt", R.GT), **kw)
        cases.append({"file": name, "kind": kind, **kw})

    tiled = ["TILED=YES", "BLOCKXSIZE=256", "BLOCKYSIZE=256"]
    tiff("rgb-zstd-predictor2.tif", "u8", R.pattern(300, 420, 3, "u8", seed=31), tiled + ["COMPRESS=ZSTD", "PREDICTOR=2", "ZSTD_LEVEL=9"])
    f = R.dem(260, 300, "f32")[:, :, None]
    f[10:20, 40:90, 0] = -9999.0
    tiff("f32-zstd-predictor3.tif", "f32", f, tiled + ["COMPRESS=ZSTD", "PREDICTOR=3"], nodata=-9999.0)
    tiff("u16-zstd-strips.tif", "u16", R.pattern(201, 233, 1, "u16", seed=32), ["COMPRESS=ZSTD", "BLOCKYSIZE=9", "ZSTD_LEVEL=1"])
    tiff("i16-zstd-planar.tif", "i16", R.pattern(120, 150, 2, "i16", seed=33), tiled + ["COMPRESS=ZSTD", "INTERLEAVE=BAND"])
    tiff("rgb-webp-lossless.tif", "u8", R.pattern(300, 420, 3, "u8", seed=34), tiled + ["COMPRESS=WEBP", "WEBP_LOSSLESS=YES"])
    tiff("rgba-webp-lossless.tif", "u8", R.pattern(260, 260, 4, "u8", seed=35), tiled + ["COMPRESS=WEBP", "WEBP_LOSSLESS=YES", "ALPHA=YES", "PHOTOMETRIC=RGB"])
    tiff("rgb-webp-lossy.tif", "u8", R.pattern(256, 512, 3, "u8", seed=36), tiled + ["COMPRESS=WEBP", "WEBP_LEVEL=90"])
    tiff("u8-lerc.tif", "u8", R.pattern(64, 80, 1, "u8", seed=37), ["COMPRESS=LERC", "MAX_Z_ERROR=0"])
    tiff("f32-lerc-zstd.tif", "f32", R.dem(64, 80, "f32")[:, :, None], ["COMPRESS=LERC_ZSTD", "MAX_Z_ERROR=0.01"])
    tiff("u8-lzma.tif", "u8", R.pattern(64, 80, 1, "u8", seed=38), ["COMPRESS=LZMA"])
    return cases


def describe(case):
    path = FILES / case["file"]
    ds = gdal.Open(str(path))
    comp = ds.GetMetadataItem("COMPRESSION", "IMAGE_STRUCTURE") or "NONE"
    entry = next(p for p in PROFILE if p["name"] == comp.replace("LERC_ZSTD", "LERC").replace("LERC_DEFLATE", "LERC"))
    out = {"file": case["file"], "compression": entry["name"], "code": entry["code"]}
    if not entry["read"]:
        out["refused"] = [entry["name"], f"({entry['code']})", CONVERT]
        return out
    b = ds.RasterCount
    level = np.stack([ds.GetRasterBand(k + 1).ReadAsArray() for k in range(b)], axis=2)
    h, w, _ = level.shape
    out["info"] = {"width": w, "height": h, "bands": b, "sample": case["kind"]}
    if case["file"].endswith("lossy.tif"):
        out["means"] = [float(np.mean(level[:, :, c].astype(np.float64))) for c in range(b)]
        pts = []
        for n in range(40):
            y = (n * 37) % h
            x = (n * 53) % w
            pts.append([x, y, [int(v) for v in level[y, x]]])
        out["points"] = pts
    else:
        out["fnv"] = R.fnv_array(level)
    return out


def build(write_files):
    cases = files() if write_files else [{"file": c["file"], "kind": c.get("info", {}).get("sample")} for c in json.loads(OUT.read_text())["cases"]]
    return {
        "format": "kentos.raster-codecs",
        "version": 1,
        "gdal": gdal.__version__,
        "convert": CONVERT,
        "profile": PROFILE,
        "cases": [describe(c) for c in cases],
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    if args.check:
        data = build(write_files=False)
        on_disk = json.loads(OUT.read_text())
        data["gdal"] = on_disk.get("gdal")
        if json.dumps(data, sort_keys=True) != json.dumps(on_disk, sort_keys=True):
            print(f"{OUT.relative_to(ROOT)} farklı; yeniden yazın.", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT.relative_to(ROOT)} güncel ({len(data['cases'])} dosya, {len(data['profile'])} sıkıştırma).")
        return
    data = build(write_files=True)
    OUT.write_text(json.dumps(data, ensure_ascii=False, indent=1) + "\n")
    print(f"{OUT.relative_to(ROOT)} yazıldı ({len(data['cases'])} dosya).")


if __name__ == "__main__":
    main()
