#!/usr/bin/env python3
"""Raster layers (docs/adr/0204): sample files and what reading and drawing them gives, written without KentOS code.

fixtures/raster/v1/files/: GeoTIFFs written by GDAL (tiled and stripped; uncompressed, LZW, Deflate with the horizontal
and the floating point predictor, PackBits, JPEG in YCbCr; 8, 16, 32 bit integers signed and not, 32 and 64 bit floats;
chunky and planar; a palette; big endian; BigTIFF; internal overviews; PixelIsPoint; a rotated and a geographic one) and
PNGs (GDAL's 8 and 16-bit ones with a world file, PIL's palette with transparency, and this script's own coder for an
Adam7-interlaced RGB and a 4-bit grey with every row filter).

fixtures/raster/v1/cases.json: for each file what the window shows (size, bands, type, place, system, nodata,
overviews) and every level's samples as an FNV-1a 64 hash of their little-endian bytes (bands interleaved): the file's
own levels as GDAL reads them, worked-out levels by the ADR's rule (the mean of the 2 × 2 samples below, nodata and NaN
left out, rounded half away from zero for integers; a palette's upper left); the statistics level's band statistics;
and some tiles' colours (258 × 258 premultiplied RGBA) by the ADR's look, hashed. JPEG's levels are given as band means
and sampled pixels, since decoders differ by a step or two.

The look's shaded relief and ramp are checked here against gdaldem hillshade and color-relief (interior pixels within
one step), so the reference's formulas are GDAL's.

    python3 scripts/fixtures/raster_cases.py          # write
    python3 scripts/fixtures/raster_cases.py --check  # compare with what is on disk
"""

import argparse
import io
import json
import math
import struct
import subprocess
import sys
import tempfile
import zlib
from pathlib import Path

import numpy as np
from osgeo import gdal, osr
from PIL import Image

gdal.UseExceptions()

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures" / "raster" / "v1"
FILES = DIR / "files"
OUT = DIR / "cases.json"
TILE = 256
APRON = TILE + 2

DTYPES = {
    "u8": (gdal.GDT_Byte, np.uint8),
    "i8": (gdal.GDT_Int8, np.int8),
    "u16": (gdal.GDT_UInt16, np.uint16),
    "i16": (gdal.GDT_Int16, np.int16),
    "u32": (gdal.GDT_UInt32, np.uint32),
    "i32": (gdal.GDT_Int32, np.int32),
    "f32": (gdal.GDT_Float32, np.float32),
    "f64": (gdal.GDT_Float64, np.float64),
}


def fnv(data: bytes) -> str:
    h = 0xCBF29CE484222325
    for b in data:
        h ^= b
        h = (h * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{h:016x}"


def fnv_array(a: np.ndarray) -> str:
    """FNV-1a 64 of an array's little-endian bytes, row by row, bands interleaved (H × W × B)."""
    return fnv(np.ascontiguousarray(a).astype(a.dtype.newbyteorder("<"), copy=False).tobytes())


def rng(seed):
    """A small deterministic generator (an LCG), no numpy randomness so that the files never change."""
    state = [seed & 0xFFFFFFFF]

    def nxt():
        state[0] = (1664525 * state[0] + 1013904223) & 0xFFFFFFFF
        return state[0] / 2**32

    return nxt


def pattern(h, w, bands, kind, seed=1):
    r = rng(seed)
    yy, xx = np.mgrid[0:h, 0:w].astype(np.float64)
    out = np.zeros((h, w, bands), dtype=np.float64)
    for b in range(bands):
        base = 0.5 + 0.35 * np.sin(xx / (13.0 + 7 * b)) * np.cos(yy / (9.0 + 5 * b)) + 0.15 * (xx + 2 * yy) / (w + 2 * h)
        noise = np.array([[r() for _ in range(w)] for _ in range(h)]) * 0.08 if h * w <= 120000 else 0.0
        out[:, :, b] = np.clip(base + noise, 0.0, 1.0)
    if kind == "u8":
        return np.round(out * 255).astype(np.uint8)
    if kind == "i8":
        return np.round(out * 254 - 127).astype(np.int8)
    if kind == "u16":
        return np.round(out * 65000).astype(np.uint16)
    if kind == "i16":
        return np.round(out * 3000 - 500).astype(np.int16)
    if kind == "u32":
        return np.round(out * 4.0e9).astype(np.uint32)
    if kind == "i32":
        return np.round(out * 2.0e9 - 1.0e9).astype(np.int32)
    if kind == "f32":
        return (out * 1500.0 + 200.0).astype(np.float32)
    return out * 1500.0 + 200.0


def dem(h, w, kind, seed=3):
    """Hills: a sum of gaussians and a tilt, metres."""
    yy, xx = np.mgrid[0:h, 0:w].astype(np.float64)
    z = 850.0 + 0.02 * xx - 0.015 * yy
    r = rng(seed)
    for _ in range(9):
        cx, cy, s, a = r() * w, r() * h, 20 + r() * 60, 20 + r() * 80
        z += a * np.exp(-((xx - cx) ** 2 + (yy - cy) ** 2) / (2 * s * s))
    return z.astype(np.float32 if kind == "f32" else (np.float64 if kind == "f64" else np.int16))


def create(path, data, kind, options, geotransform=None, epsg=5256, nodata=None, overviews=None, color_table=None, point=False):
    """A GeoTIFF of `data` (H × W × B) written by GDAL."""
    h, w, b = data.shape
    drv = gdal.GetDriverByName("GTiff")
    ds = drv.Create(str(path), w, h, b, DTYPES[kind][0], options)
    if geotransform:
        ds.SetGeoTransform(geotransform)
    if epsg:
        sr = osr.SpatialReference()
        sr.ImportFromEPSG(epsg)
        ds.SetProjection(sr.ExportToWkt())
    if point:
        ds.SetMetadataItem("AREA_OR_POINT", "Point")
    for k in range(b):
        band = ds.GetRasterBand(k + 1)
        # The palette decides the photometric tag, which must be set before any pixel is written.
        if color_table is not None:
            band.SetRasterColorTable(color_table)
            band.SetRasterColorInterpretation(gdal.GCI_PaletteIndex)
        if nodata is not None:
            band.SetNoDataValue(nodata)
    for k in range(b):
        ds.GetRasterBand(k + 1).WriteArray(data[:, :, k])
    if overviews:
        ds.BuildOverviews("AVERAGE", overviews)
    ds.FlushCache()
    ds = None


def read_levels(path, case):
    """The file's levels as GDAL reads them: level 0 and each overview, H × W × B."""
    ds = gdal.Open(str(path))
    bands = ds.RasterCount
    out = [np.stack([ds.GetRasterBand(k + 1).ReadAsArray() for k in range(bands)], axis=2)]
    n = ds.GetRasterBand(1).GetOverviewCount()
    for o in range(n):
        out.append(np.stack([ds.GetRasterBand(k + 1).GetOverview(o).ReadAsArray() for k in range(bands)], axis=2))
    return ds, out


def level_sizes(w, h):
    sizes = [(w, h)]
    while w > TILE or h > TILE:
        w, h = (w + 1) // 2, (h + 1) // 2
        sizes.append((w, h))
    return sizes


def round_away(v):
    return np.where(v < 0, -np.floor(-v + 0.5), np.floor(v + 0.5))


def halve(a, nodata, kind, nearest):
    """The ADR's worked-out level: each sample the mean of the up to four below it, nodata and NaN left out."""
    h, w, b = a.shape
    nh, nw = (h + 1) // 2, (w + 1) // 2
    src = a.astype(np.float64)
    out = np.zeros((nh, nw, b), dtype=np.float64)
    for j in range(nh):
        for i in range(nw):
            for c in range(b):
                vals = []
                for dj in (0, 1):
                    for di in (0, 1):
                        y, x = 2 * j + dj, 2 * i + di
                        if y >= h or x >= w:
                            continue
                        v = src[y, x, c]
                        if math.isnan(v) or (nodata is not None and v == nodata):
                            continue
                        vals.append(v)
                        if nearest:
                            break
                    if nearest and vals:
                        break
                if vals:
                    out[j, i, c] = vals[0] if nearest else sum_in_order(vals) / len(vals)
                elif nodata is not None:
                    out[j, i, c] = nodata
                else:
                    out[j, i, c] = math.nan if kind in ("f32", "f64") else 0.0
    return to_kind(out, kind)


def sum_in_order(vals):
    s = 0.0
    for v in vals:
        s += v
    return s


def to_kind(v, kind):
    if kind in ("f32", "f64"):
        return v.astype(DTYPES[kind][1])
    info = np.iinfo(DTYPES[kind][1])
    return np.clip(round_away(v), info.min, info.max).astype(DTYPES[kind][1])


# ── The look (docs/adr/0204 §4), as style.rs works it out ────────────────────────────────────────────────────────

RAMPS = {
    "Gri": [(0, 0, 0), (255, 255, 255)],
    "Arazi": [(0x2E, 0x7D, 0x32), (0x9C, 0xCC, 0x65), (0xFF, 0xF5, 0x9D), (0xA1, 0x88, 0x7F), (0xFA, 0xFA, 0xFA)],
    "Spektral": [(0x2B, 0x83, 0xBA), (0xAB, 0xDD, 0xA4), (0xFF, 0xFF, 0xBF), (0xFD, 0xAE, 0x61), (0xD7, 0x19, 0x1C)],
    "Viridis": [(0x44, 0x01, 0x54), (0x3B, 0x52, 0x8B), (0x21, 0x91, 0x8C), (0x5E, 0xC9, 0x62), (0xFD, 0xE7, 0x25)],
}


def ramp_at(stops, t):
    n = len(stops)
    if n == 1 or not t > 0.0:
        return stops[0]
    if t >= 1.0:
        return stops[-1]
    p = t * (n - 1)
    k = min(int(math.floor(p)), n - 2)
    f = p - k
    a, b = stops[k], stops[k + 1]
    return tuple(int(math.floor(a[c] + (b[c] - a[c]) * f + 0.5)) for c in range(3))


def unit(v, lo, hi):
    if hi > lo:
        return min(max((v - lo) / (hi - lo), 0.0), 1.0)
    return 0.0


def byte(t):
    return int(min(max(math.floor(t * 255.0 + 0.5), 0.0), 255.0))


def stats_of(level, nodata, kind):
    """Each band's count, least, most and 2nd and 98th percentile (⌊p·(n−1) + ½⌋ of the sorted values)."""
    out = []
    h, w, b = level.shape
    for c in range(b):
        v = level[:, :, c].astype(np.float64).ravel()
        v = v[~np.isnan(v)]
        if nodata is not None:
            v = v[v != nodata]
        v = np.sort(v)
        n = len(v)
        if n == 0:
            out.append({"count": 0, "min": 0.0, "max": 0.0, "low": 0.0, "high": 0.0})
            continue
        r = lambda p: int(math.floor(p * (n - 1) + 0.5))
        out.append({"count": n, "min": float(v[0]), "max": float(v[-1]), "low": float(v[r(0.02)]), "high": float(v[r(0.98)])})
    return out


def bounds(style, band, stats, kind):
    i = band - 1
    st = stats[i] if stats and i < len(stats) and stats[i]["count"] > 0 else None
    fallback = {"u8": (0.0, 255.0), "u16": (0.0, 65535.0), "i8": (-128.0, 127.0), "i16": (-32768.0, 32767.0)}.get(kind, (0.0, 1.0))
    s = style.get("stretch", "none")
    if s == "none":
        return None if kind == "u8" else ((st["min"], st["max"]) if st else fallback)
    if s == "minMax":
        return (st["min"], st["max"]) if st else fallback
    if s == "percent":
        return (st["low"], st["high"]) if st else fallback
    return (style.get("min", fallback[0]), style.get("max", fallback[1]))


def channel(v, b):
    if b is None:
        return int(min(max(math.floor(v + 0.5), 0.0), 255.0))
    return byte(unit(v, b[0], b[1]))


def premul(r, g, b, a):
    if a == 255:
        return (r, g, b, 255)
    m = lambda c: (c * a + 127) // 255
    return (m(r), m(g), m(b), a)


def shade_light(style, ewres, nsres):
    az = style.get("azimuth", 315.0)
    alt = style.get("altitude", 45.0)
    z = style.get("zFactor", 1.0)
    rad = math.pi / 180.0
    zs = z / 8.0
    cos_alt_z = math.cos(alt * rad) * zs
    return {
        "s": 254.0 * math.sin(alt * rad),
        "c": 254.0 * math.cos(az * rad) * cos_alt_z,
        "n": 254.0 * math.sin(az * rad) * cos_alt_z,
        "q": zs * zs,
        "ie": 1.0 / ewres,
        "in": 1.0 / nsres,
    }


def shade(L, w):
    x = ((w[0] + w[3] + w[3] + w[6]) - (w[2] + w[5] + w[5] + w[8])) * L["ie"]
    y = ((w[6] + w[7] + w[7] + w[8]) - (w[0] + w[1] + w[1] + w[2])) * L["in"]
    num = L["s"] - (y * L["c"] - x * L["n"])
    c = num / math.sqrt(1.0 + L["q"] * (x * x + y * y))
    return 1.0 if c <= 0.0 else 1.0 + c


def render_tile(level, tx, ty, style, stats, kind, nodata, palette, ewres, nsres):
    """The tile's 258 × 258 premultiplied RGBA, as style.rs makes it, from the level padded by its edges."""
    h, w, b = level.shape
    pad = np.pad(level.astype(np.float64), ((4, TILE + 8), (4, TILE + 8), (0, 0)), mode="edge")
    nd = style.get("nodata", nodata)
    empty = lambda v: math.isnan(v) or (nd is not None and v == nd)

    def at(u, v, band):
        # Output (u, v) is level pixel (tx·256 − 1 + u, ty·256 − 1 + v), clamped by the padding.
        x = min(max(tx * TILE - 1 + u, 0), w - 1)
        y = min(max(ty * TILE - 1 + v, 0), h - 1)
        return float(level[y, x, band])

    out = bytearray(APRON * APRON * 4)
    render = style["render"]
    bands = style["bands"]
    if render == "rgb":
        r, g, bb = bands[0] - 1, bands[1] - 1, bands[2] - 1
        al = bands[3] - 1 if len(bands) > 3 else None
        bd = [bounds(style, bands[k], stats, kind) for k in range(3)]
        for v in range(APRON):
            for u in range(APRON):
                vr, vg, vb = at(u, v, r), at(u, v, g), at(u, v, bb)
                if math.isnan(vr) or math.isnan(vg) or math.isnan(vb):
                    continue
                if nd is not None and vr == nd and vg == nd and vb == nd:
                    continue
                if al is None:
                    a = 255
                else:
                    x = at(u, v, al)
                    a = byte(x / 65535.0) if kind == "u16" else channel(x, None)
                if a == 0:
                    continue
                o = (v * APRON + u) * 4
                out[o : o + 4] = bytes(premul(channel(vr, bd[0]), channel(vg, bd[1]), channel(vb, bd[2]), a))
    elif render in ("gray", "palette", "ramp"):
        k = bands[0] - 1
        bd = bounds(style, bands[0], stats, kind)
        stops = RAMPS[style.get("ramp", "Gri")]
        for v in range(APRON):
            for u in range(APRON):
                x = at(u, v, k)
                if empty(x):
                    continue
                if render == "palette":
                    c = palette[int(x)] if int(x) < len(palette) else (0, 0, 0)
                    c = (c[0] >> 8, c[1] >> 8, c[2] >> 8)
                elif render == "ramp":
                    t = unit(x, *(bd or (0.0, 255.0)))
                    if style.get("invert"):
                        t = 1.0 - t
                    c = ramp_at(stops, t)
                else:
                    gv = channel(x, bd)
                    c = (gv, gv, gv)
                o = (v * APRON + u) * 4
                out[o : o + 4] = bytes((c[0], c[1], c[2], 255))
    else:
        k = bands[0] - 1
        L = shade_light(style, ewres, nsres)
        bd = bounds(style, bands[0], stats, kind)
        stops = RAMPS[style.get("ramp", "Gri")]
        for v in range(APRON):
            for u in range(APRON):
                centre = at(u, v, k)
                if empty(centre):
                    continue
                win = []
                for dj in range(3):
                    for di in range(3):
                        x = at(u - 1 + di, v - 1 + dj, k)
                        win.append(centre if empty(x) else x)
                s = shade(L, win)
                o = (v * APRON + u) * 4
                if render == "hillshade":
                    sv = int(min(max(math.floor(s + 0.5), 0.0), 255.0))
                    out[o : o + 4] = bytes((sv, sv, sv, 255))
                else:
                    t = unit(centre, *(bd or (0.0, 255.0)))
                    if style.get("invert"):
                        t = 1.0 - t
                    c = ramp_at(stops, t)
                    f = 0.4 + 0.6 * s / 255.0
                    m = lambda q: int(min(max(math.floor(q * f + 0.5), 0.0), 255.0))
                    out[o : o + 4] = bytes((m(c[0]), m(c[1]), m(c[2]), 255))
    return bytes(out)


# ── PNG by this script's own coder ────────────────────────────────────────────────────────────────────────────────


def png_chunk(kind, data):
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)


def png_filter(raw_rows, bpp):
    """Each row with a filter in turn (0 to 4), as a coder may choose."""
    out = bytearray()
    prev = bytearray(len(raw_rows[0])) if raw_rows else bytearray()
    for n, row in enumerate(raw_rows):
        kind = n % 5
        f = bytearray(len(row))
        for i in range(len(row)):
            a = row[i - bpp] if i >= bpp else 0
            b = prev[i]
            c = prev[i - bpp] if i >= bpp else 0
            if kind == 0:
                p = 0
            elif kind == 1:
                p = a
            elif kind == 2:
                p = b
            elif kind == 3:
                p = (a + b) // 2
            else:
                pa, pb, pc = abs(b - c), abs(a - c), abs(a + b - 2 * c)
                p = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
            f[i] = (row[i] - p) & 0xFF
        out.append(kind)
        out += f
        prev = bytearray(row)
    return bytes(out)


def png_write(path, pixels, depth, color, interlace):
    """A PNG of `pixels` (H × W × C, values within `depth` bits)."""
    h, w, ch = pixels.shape
    bits = depth * ch

    def rows_of(img):
        rows = []
        for y in range(img.shape[0]):
            if depth == 8:
                rows.append(bytes(img[y].astype(np.uint8).ravel()))
            elif depth == 16:
                rows.append(b"".join(struct.pack(">H", int(v)) for v in img[y].ravel()))
            else:
                acc, n, out = 0, 0, bytearray()
                for v in img[y].ravel():
                    acc = (acc << depth) | int(v)
                    n += depth
                    if n == 8:
                        out.append(acc)
                        acc, n = 0, 0
                if n:
                    out.append(acc << (8 - n))
                rows.append(bytes(out))
        return rows

    bpp = max(1, (bits + 7) // 8)
    if interlace:
        passes = [(0, 0, 8, 8), (4, 0, 8, 8), (0, 4, 4, 8), (2, 0, 4, 4), (0, 2, 2, 4), (1, 0, 2, 2), (0, 1, 1, 2)]
        data = b""
        for x0, y0, dx, dy in passes:
            sub = pixels[y0::dy, x0::dx]
            if sub.shape[0] == 0 or sub.shape[1] == 0:
                continue
            data += png_filter(rows_of(sub), bpp)
    else:
        data = png_filter(rows_of(pixels), bpp)
    ihdr = struct.pack(">IIBBBBB", w, h, depth, color, 0, 0, 1 if interlace else 0)
    blob = b"\x89PNG\r\n\x1a\n" + png_chunk(b"IHDR", ihdr) + png_chunk(b"IDAT", zlib.compress(data, 9)) + png_chunk(b"IEND", b"")
    Path(path).write_bytes(blob)


def world_text(gt):
    """A world file for GDAL's geotransform: A, D, B, E and the centre of the upper left pixel."""
    x0, a, b, y0, c, d = gt
    vals = [a, c, b, d, x0 + 0.5 * a + 0.5 * b, y0 + 0.5 * c + 0.5 * d]
    return "".join(f"{repr(float(v))}\n" for v in vals)


# ── The cases ─────────────────────────────────────────────────────────────────────────────────────────────────────

GT = (487000.0, 0.25, 0.0, 4420100.0, 0.0, -0.25)


def make_files():
    """Writes every sample file; returns the cases' descriptions."""
    FILES.mkdir(parents=True, exist_ok=True)
    cases = []

    def tiff(name, kind, data, options, **kw):
        create(FILES / name, data, kind, options, geotransform=kw.pop("gt", GT), **kw)
        cases.append({"file": name, "kind": kind})

    tiff("rgb-deflate-overviews.tif", "u8", pattern(400, 600, 3, "u8"),
         ["TILED=YES", "BLOCKXSIZE=256", "BLOCKYSIZE=256", "COMPRESS=DEFLATE", "PREDICTOR=2"], overviews=[2, 4])
    tiff("gray-lzw-strips.tif", "u8", pattern(257, 333, 1, "u8", seed=2), ["COMPRESS=LZW", "BLOCKYSIZE=7"])
    tiff("u16-packbits-big-endian.tif", "u16", pattern(300, 300, 1, "u16", seed=4),
         ["COMPRESS=PACKBITS", "ENDIANNESS=BIG"])
    d = dem(300, 400, "i16").astype(np.int16)[:, :, None]
    d[5:9, 10:30, 0] = -32768
    tiff("i16-dem-tiled128.tif", "i16", d,
         ["TILED=YES", "BLOCKXSIZE=128", "BLOCKYSIZE=128", "COMPRESS=DEFLATE", "PREDICTOR=2"], nodata=-32768)
    f = dem(260, 300, "f32")[:, :, None]
    f[100:104, 200:260, 0] = -9999.0
    tiff("f32-dem-predictor3-bigtiff.tif", "f32", f,
         ["TILED=YES", "COMPRESS=DEFLATE", "PREDICTOR=3", "BIGTIFF=YES"], nodata=-9999.0)
    tiff("f64-planar.tif", "f64", pattern(70, 90, 2, "f64", seed=5), ["INTERLEAVE=BAND"])
    ct = gdal.ColorTable()
    for i in range(256):
        ct.SetColorEntry(i, ((i * 7) % 256, (i * 13) % 256, (255 - i) % 256, 255))
    tiff("palette-lzw.tif", "u8", (pattern(260, 260, 1, "u8", seed=6) // 16).astype(np.uint8), ["COMPRESS=LZW"],
         color_table=ct)
    tiff("rgb-jpeg-ycbcr.tif", "u8", pattern(256, 512, 3, "u8", seed=7),
         ["TILED=YES", "COMPRESS=JPEG", "PHOTOMETRIC=YCBCR", "JPEG_QUALITY=90"])
    tiff("pixel-is-point.tif", "u8", pattern(64, 80, 1, "u8", seed=8), [], point=True)
    tiff("rotated.tif", "u16", pattern(90, 120, 1, "u16", seed=9), ["COMPRESS=DEFLATE"],
         gt=(487000.0, 0.5, 0.1, 4420100.0, 0.1, -0.5))
    tiff("geographic.tif", "f32", dem(50, 60, "f32")[:, :, None], [], gt=(33.0, 0.001, 0.0, 39.5, 0.0, -0.001), epsg=4326)
    tiff("i8.tif", "i8", pattern(40, 50, 1, "i8", seed=10), [])
    tiff("u32.tif", "u32", pattern(40, 50, 1, "u32", seed=11), ["COMPRESS=LZW"])
    tiff("i32-two-bands.tif", "i32", pattern(40, 50, 2, "i32", seed=12), ["COMPRESS=DEFLATE", "PREDICTOR=2"])
    # The last band: a mask (RGB with ALPHA=YES: ExtraSamples 2) or data (a 4-band satellite image: ExtraSamples 0).
    tiff("rgba-alpha.tif", "u8", pattern(48, 64, 4, "u8", seed=15), ["PHOTOMETRIC=RGB", "ALPHA=YES"])
    tiff("u16-four-bands.tif", "u16", pattern(48, 64, 4, "u16", seed=16), ["COMPRESS=DEFLATE"])
    tiff("no-place.tif", "u8", pattern(30, 40, 3, "u8", seed=13), [], gt=None, epsg=None)
    # A large one without overviews: its pyramid is worked out (levels 1, 2) from level 0.
    tiff("u8-large-stripped.tif", "u8", pattern(700, 900, 1, "u8", seed=14), ["COMPRESS=DEFLATE", "BLOCKYSIZE=16"])

    # PNGs.
    rgb = pattern(120, 160, 3, "u8", seed=20)
    Image.fromarray(rgb, "RGB").save(FILES / "rgb.png")
    (FILES / "rgb.pgw").write_text(world_text(GT))
    cases.append({"file": "rgb.png", "kind": "u8", "world": "rgb.pgw"})
    g16 = dem(64, 96, "f32").astype(np.uint16)
    png_write(FILES / "gray16.png", g16[:, :, None], 16, 0, False)
    (FILES / "gray16.pgw").write_text(world_text((487000.0, 2.0, 0.0, 4420100.0, 0.0, -2.0)))
    cases.append({"file": "gray16.png", "kind": "u16", "world": "gray16.pgw"})
    inter = pattern(37, 45, 3, "u8", seed=21)
    png_write(FILES / "rgb-interlaced.png", inter, 8, 2, True)
    cases.append({"file": "rgb-interlaced.png", "kind": "u8"})
    g4 = (pattern(33, 29, 1, "u8", seed=22) // 17).astype(np.uint8)
    png_write(FILES / "gray4.png", g4, 4, 0, False)
    cases.append({"file": "gray4.png", "kind": "u8", "gray4": True})
    pal = Image.fromarray((pattern(20, 24, 1, "u8", seed=23)[:, :, 0] // 32).astype(np.uint8), "P")
    pal.putpalette([v for i in range(256) for v in ((i * 40) % 256, (i * 90) % 256, (i * 20) % 256)])
    pal.info["transparency"] = bytes([0 if i == 3 else 255 for i in range(8)])
    pal.save(FILES / "palette-trns.png", transparency=bytes([0 if i == 3 else 255 for i in range(8)]))
    cases.append({"file": "palette-trns.png", "kind": "u8"})
    return cases


def png_samples(path):
    """A PNG's samples as the ADR reads them (expanded palette, scaled low-depth grey), from PIL and raw IHDR facts."""
    im = Image.open(path)
    im.load()
    head = path.read_bytes()[16:29]
    w, h, depth, color = struct.unpack(">IIBB", head[:10])
    if color == 3:
        rgba = np.array(im.convert("RGBA"))
        has_trns = "transparency" in im.info
        return rgba if has_trns else rgba[:, :, :3]
    if color == 0 and depth < 8:
        raw = np.array(im)  # PIL gives the 4-bit values 0–15 for mode L? (it gives scaled 0–255 for 'L')
        return raw[:, :, None]
    if color == 0 and depth == 16:
        return np.array(im).astype(np.uint16)[:, :, None]
    return np.array(im)


def describe(case):
    path = FILES / case["file"]
    kind = case["kind"]
    out = {"file": case["file"]}
    if "world" in case:
        out["world"] = case["world"]
    if path.suffix == ".png":
        data = png_samples(path)
        if case.get("gray4"):
            # PIL keeps 4-bit grey as 0–15 in mode L? normalise to the ADR's 0–255 scale: v·255/15 rounded half up.
            raw = np.array(Image.open(path))
            if raw.max() <= 15:
                data = ((raw.astype(np.uint32) * 255 + 7) // 15).astype(np.uint8)[:, :, None]
        h, w, b = data.shape
        affine = None
        if "world" in case:
            vals = [float(x) for x in (FILES / case["world"]).read_text().split()]
            A, D, B, E, C, F = vals
            affine = [C - 0.5 * A - 0.5 * B, A, B, F - 0.5 * D - 0.5 * E, D, E]
        out["info"] = {"width": w, "height": h, "bands": b, "sample": kind, "affine": affine, "epsg": None, "nodata": None, "overviews": 0}
        levels = [data]
        nodata = None
        palette = None
        nearest = False
    else:
        ds, file_levels = read_levels(path, case)
        band = ds.GetRasterBand(1)
        gt = ds.GetGeoTransform(can_return_null=True)
        srs = ds.GetSpatialRef()
        epsg = None
        geographic = False
        if srs is not None:
            code = srs.GetAuthorityCode(None)
            epsg = int(code) if code else None
            geographic = bool(srs.IsGeographic())
        nodata = band.GetNoDataValue()
        ct = band.GetRasterColorTable()
        palette = None
        if ct is not None:
            # The file's colour map is 16 bits a channel: GDAL scales 8 bits by 257 when it writes.
            palette = [tuple(ct.GetColorEntry(i)[c] * 257 for c in range(3)) for i in range(ct.GetCount())]
        nearest = palette is not None
        h, w, b = file_levels[0].shape
        out["info"] = {
            "width": w,
            "height": h,
            "bands": b,
            "sample": kind,
            "affine": list(gt) if gt else None,
            "epsg": epsg,
            "geographic": geographic,
            "nodata": None if nodata is None else ("nan" if math.isnan(nodata) else nodata),
            "overviews": len(file_levels) - 1,
            # GDAL's reading: the last band a mask (ExtraSamples 1 or 2; an RGB's fourth without the tag).
            "alpha": ds.GetRasterBand(ds.RasterCount).GetColorInterpretation() == gdal.GCI_AlphaBand,
        }
        levels = file_levels
    sizes = level_sizes(out["info"]["width"], out["info"]["height"])
    # Every level: the file's where it has it (matched by size), else worked out from the one before.
    full = []
    by_size = {(lv.shape[1], lv.shape[0]): lv for lv in levels}
    for k, (w, h) in enumerate(sizes):
        if (w, h) in by_size and (k == 0 or path.suffix != ".png"):
            full.append(by_size[(w, h)])
        else:
            full.append(halve(full[-1], nodata, kind, nearest))
    jpeg = case["file"].endswith("jpeg-ycbcr.tif")
    out["levels"] = []
    for k, lv in enumerate(full):
        entry = {"width": int(lv.shape[1]), "height": int(lv.shape[0])}
        if jpeg:
            entry["means"] = [float(np.mean(lv[:, :, c].astype(np.float64))) for c in range(lv.shape[2])]
            pts = []
            for n in range(40):
                y = (n * 37) % lv.shape[0]
                x = (n * 53) % lv.shape[1]
                pts.append([x, y, [int(v) for v in lv[y, x]]])
            entry["points"] = pts
        else:
            entry["fnv"] = fnv_array(lv)
        out["levels"].append(entry)
    stats_level = next(k for k, (w, h) in enumerate(sizes) if w <= 1024 and h <= 1024)
    stats = stats_of(full[stats_level], nodata, kind)
    if not jpeg:
        out["stats"] = {"level": stats_level, "bands": stats}
    # Tiles by the look.
    affine = out["info"]["affine"] or [0.0, 1.0, 0.0, 0.0, 0.0, -1.0]
    tiles = []
    for style, level, tx, ty in looks_for(case, out["info"], palette):
        if jpeg:
            break
        lv = full[level]
        # A tile the level has (a small raster has one column of tiles).
        tx = min(tx, (lv.shape[1] - 1) // TILE)
        ty = min(ty, (lv.shape[0] - 1) // TILE)
        f = float(1 << level)
        a, bb, c, d = affine[1], affine[2], affine[4], affine[5]
        ewres = math.hypot(a, c) * f
        ns = math.hypot(bb, d) * f
        nsres = -ns if d < 0 else ns
        rgba = render_tile(lv, tx, ty, style, stats, kind, nodata, palette, ewres, nsres)
        tiles.append({"style": style, "level": level, "tx": tx, "ty": ty, "fnv": fnv(rgba), "opaque": sum(1 for i in range(3, len(rgba), 4) if rgba[i])})
    if tiles:
        out["tiles"] = tiles
    return out


def looks_for(case, info, palette):
    """The looks each file is drawn with (its default and a few more)."""
    name = case["file"]
    kind = case["kind"]
    b = info["bands"]
    if palette:
        return [({"render": "palette", "bands": [1]}, 0, 0, 0)]
    if name.startswith("rgb") and kind == "u8":
        looks = [({"render": "rgb", "bands": [1, 2, 3]}, 0, 0, 0), ({"render": "rgb", "bands": [3, 2, 1], "stretch": "percent"}, 0, 1, 0)]
        if b >= 4:
            looks.append(({"render": "rgb", "bands": [1, 2, 3, 4]}, 0, 0, 0))
        if name.startswith("rgb-deflate"):
            looks.append(({"render": "rgb", "bands": [1, 2, 3]}, 1, 1, 0))
        return looks
    if name.startswith("palette-trns"):
        return [({"render": "rgb", "bands": [1, 2, 3, 4]}, 0, 0, 0)]
    if b == 1 and kind == "u8":
        return [({"render": "gray", "bands": [1]}, 0, 0, 0), ({"render": "ramp", "bands": [1], "stretch": "minMax", "ramp": "Viridis"}, 0, 1, 0)]
    if "dem" in name or name == "geographic.tif" or name == "gray16.png":
        return [
            ({"render": "rampShade", "bands": [1], "stretch": "minMax", "ramp": "Arazi"}, 0, 0, 0),
            ({"render": "hillshade", "bands": [1], "azimuth": 300.0, "altitude": 40.0, "zFactor": 2.0}, 0, 1, 0),
            ({"render": "ramp", "bands": [1], "stretch": "percent", "ramp": "Spektral", "invert": True}, 0, 0, 0),
            ({"render": "gray", "bands": [1], "stretch": "manual", "min": 900.0, "max": 1000.0}, 0, 0, 0),
        ]
    if b >= 2:
        return [({"render": "gray", "bands": [2], "stretch": "minMax"}, 0, 0, 0)]
    return [({"render": "gray", "bands": [1], "stretch": "minMax"}, 0, 0, 0)]


def gdaldem_check():
    """The look's shaded relief and ramp against gdaldem (interior pixels within one step)."""
    with tempfile.TemporaryDirectory() as tmp:
        src = FILES / "f32-dem-predictor3-bigtiff.tif"
        ds = gdal.Open(str(src))
        z = ds.GetRasterBand(1).ReadAsArray().astype(np.float64)
        gt = ds.GetGeoTransform()
        nodata = ds.GetRasterBand(1).GetNoDataValue()
        hs = Path(tmp) / "hs.tif"
        gdal.DEMProcessing(str(hs), str(src), "hillshade", azimuth=300.0, altitude=40.0, zFactor=2.0)
        hds = gdal.Open(str(hs))
        ref = hds.GetRasterBand(1).ReadAsArray()
        L = shade_light({"azimuth": 300.0, "altitude": 40.0, "zFactor": 2.0}, gt[1], gt[5])
        h, w = z.shape
        worst = 0
        checked = 0
        for y in range(1, h - 1, 3):
            for x in range(1, w - 1, 3):
                win = z[y - 1 : y + 2, x - 1 : x + 2].ravel()
                if np.any(win == nodata):
                    continue
                s = int(min(max(math.floor(shade(L, list(win)) + 0.5), 0.0), 255.0))
                worst = max(worst, abs(s - int(ref[y, x])))
                checked += 1
        if worst > 1:
            raise SystemExit(f"gölgeli kabartma gdaldem'den {worst} adım farklı")
        # The ramp: gdaldem color-relief through the same stops at the band's least and most value.
        valid = z[z != nodata]
        lo, hi = float(valid.min()), float(valid.max())
        stops = RAMPS["Arazi"]
        table = Path(tmp) / "ramp.txt"
        table.write_text("".join(f"{lo + (hi - lo) * i / (len(stops) - 1)!r} {c[0]} {c[1]} {c[2]}\n" for i, c in enumerate(stops)) + f"{nodata} 0 0 0 0\n")
        cr = Path(tmp) / "cr.tif"
        gdal.DEMProcessing(str(cr), str(src), "color-relief", colorFilename=str(table))
        cds = gdal.Open(str(cr))
        rgb = np.stack([cds.GetRasterBand(k + 1).ReadAsArray() for k in range(3)], axis=2)
        worst_ramp = 0
        for y in range(0, h, 5):
            for x in range(0, w, 5):
                if z[y, x] == nodata:
                    continue
                c = ramp_at(stops, unit(z[y, x], lo, hi))
                worst_ramp = max(worst_ramp, max(abs(int(c[k]) - int(rgb[y, x, k])) for k in range(3)))
        if worst_ramp > 1:
            raise SystemExit(f"renk rampası gdaldem'den {worst_ramp} adım farklı")
        return {"hillshadePixels": checked, "hillshadeWorst": worst, "rampWorst": worst_ramp}


def build(write_files):
    cases = make_files() if write_files else json.loads(OUT.read_text())["cases"]
    if not write_files:
        cases = [{"file": c["file"], "kind": c["info"]["sample"], **({"world": c["world"]} if "world" in c else {}), **({"gray4": True} if c["file"] == "gray4.png" else {})} for c in cases]
    described = [describe(c) for c in cases]
    gd = gdaldem_check()
    return {
        "format": "kentos.raster-cases",
        "version": 1,
        "tile": TILE,
        "gdal": gdal.__version__,
        "gdaldem": gd,
        "cases": described,
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
        print(f"{OUT.relative_to(ROOT)} güncel ({len(data['cases'])} dosya; gdaldem: {data['gdaldem']}).")
        return
    data = build(write_files=True)
    OUT.write_text(json.dumps(data, ensure_ascii=False, indent=1) + "\n")
    print(f"{OUT.relative_to(ROOT)} yazıldı ({len(data['cases'])} dosya; gdaldem: {data['gdaldem']}).")


if __name__ == "__main__":
    main()
