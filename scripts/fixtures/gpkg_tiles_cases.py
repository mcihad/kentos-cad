#!/usr/bin/env python3
"""GeoPackage tile pyramids as rasters (docs/adr/0556, 0627; plan/gis.md CBS-03's raster profile): the shared cases of
`cad.rasters.gpkgread` and `cad.rasters.gpkgwrite` and their files, from GDAL's own GeoPackage driver, without KentOS
code.

    python3 scripts/fixtures/gpkg_tiles_cases.py --files   # (re)write the sources of fixtures/gpkg-tiles/v1 (GDAL, sqlite3)
    python3 scripts/fixtures/gpkg_tiles_cases.py           # write fixtures/commands/v1/cad.rasters.gpkg{read,write}.json
    python3 scripts/fixtures/gpkg_tiles_cases.py --check   # writes nothing; compares the cases and the files KentOS wrote

The sources: GeoPackages GDAL wrote (RGBA and grey PNG tiles, JPEG tiles, WebP tiles, PNG and JPEG tiles in one table,
two tile tables in one file, gridded coverages of floats, of 16-bit and signed 16-bit integers and of scaled integers, a
GoogleMapsCompatible pyramid), edited with sqlite3 where a case needs what GDAL does not make (a missing tile, a tile that
is no picture or of another size, a 16-bit picture tile, a broken WebP header, an 8-bit tile in an integer coverage, an
undefined system, a matrix too large to read, a table without tiles), and GeoTIFFs (RGB, RGBA, grey, grey and alpha,
16-bit and signed 16-bit heights with and without nodata, 32 and 64-bit floats, 32-bit integers, a palette, a turned one,
one with nodata, one with no system) for the writer.

The rules written here again from ADR 0556 and 0627: the window `floor((min - origin)/pixel + 1/2)` and `floor((max -
min)/pixel + 1/2)` cut to the grid, pictures side by side in four bands (a grey tile its grey three times, alpha 255; no
tile alpha 0), a coverage's values in one band ((stored × tile scale + tile offset) × scale + offset; the null stored
value no value); a linked window's first pixel moved to a multiple of 2^K (K the coarsest zoom of twice the pixel it draws
from, at most the level of one tile); the pyramid of the writer: the raster's own pixel at its finest zoom with the grid's
origin at its top-left corner, each coarser zoom half the resolution (pictures `(a + b + c + d + 2) // 4` over a canvas
padded with 0; a coverage the mean of the children inside the finer image and not null, an integer one rounded half up),
the matrix set's box the coarsest level's single tile and every level's matrix the whole box (2^k tiles a side; the tiles
that hold none of the image are not written, GDAL's validator holds a pyramid to this).

`--check` also opens the GeoTIFFs and GeoPackages KentOS wrote (`KENTOS_WRITE_GPKGTILES=1 cargo test -p
kentos-native-application --test all gpkg_tiles_files`, fixtures/gpkg-tiles/v1/out) with GDAL: the read rasters hold
GDAL's own pixels of the same zoom and window, a written GeoPackage passes GDAL's validate_gpkg.py and GDAL reads the
finest zoom with the raster's own pixels and every coarser one as the rule's mean.
"""
import json
import math
import os
import shutil
import sqlite3
import struct
import subprocess
import sys
import zlib
from pathlib import Path

import numpy as np
from osgeo import gdal, osr

gdal.UseExceptions()
gdal.SetConfigOption("GDAL_PAM_ENABLED", "NO")

ROOT = Path(__file__).resolve().parents[2]
FIX = ROOT / "fixtures"
DIR = FIX / "gpkg-tiles" / "v1"
OUT_DIR = DIR / "out"
READ = FIX / "commands" / "v1" / "cad.rasters.gpkgread.json"
WRITE = FIX / "commands" / "v1" / "cad.rasters.gpkgwrite.json"
SRID = 5256
X0, Y0 = 487200.0, 4420600.0
MOST_PIXELS = 1 << 32
STRIP_MOST = 256 << 20
SIDE = 256
FLT_MIN = float(np.finfo(np.float32).min)
CHECKS = os.environ.get("KENTOS_GPKG_TILES_CHECKS", "")

# ── The files ──────────────────────────────────────────────────────────────────────────────────────────────────


def wkt(srid):
    s = osr.SpatialReference()
    s.ImportFromEPSG(srid)
    return s.ExportToWkt()


def picture(w, h, seed, bands=4):
    """A deterministic picture: gradients and noise, `bands` of bytes (the last alpha for 4)."""
    rng = np.random.default_rng(seed)
    j, i = np.mgrid[0:h, 0:w]
    a = [
        (i * 255 // max(w - 1, 1)) ^ np.kron(rng.integers(0, 8, (h // 8 + 1, w // 8 + 1)), np.ones((8, 8), int))[:h, :w],
        (j * 255 // max(h - 1, 1)),
        ((i + j) * 3 % 256),
        np.clip(60 + (i * 195 // max(w - 1, 1)), 0, 255),
    ]
    return [np.clip(x, 0, 255).astype(np.uint8) for x in a[:bands]]


def mem_raster(w, h, bands, gt, srid=SRID, seed=1, dtype=gdal.GDT_Byte):
    ds = gdal.GetDriverByName("MEM").Create("", w, h, bands, dtype)
    ds.SetGeoTransform(gt)
    if srid:
        ds.SetProjection(wkt(srid))
    arrays = picture(w, h, seed, max(bands, 1))
    for b in range(bands):
        arr = arrays[b] if b < len(arrays) else arrays[0]
        if dtype == gdal.GDT_UInt16:
            arr = arr.astype(np.uint16) * 257
        ds.GetRasterBand(b + 1).WriteArray(arr)
    if bands == 4:
        ds.GetRasterBand(4).SetColorInterpretation(gdal.GCI_AlphaBand)
        for k, c in enumerate((gdal.GCI_RedBand, gdal.GCI_GreenBand, gdal.GCI_BlueBand)):
            ds.GetRasterBand(k + 1).SetColorInterpretation(c)
    return ds


def gpkg_from(name, mem, table, tile_format="PNG", band_count=None, append=False, overviews=(2, 4)):
    opts = [f"RASTER_TABLE={table}", f"TILE_FORMAT={tile_format}"]
    if append:
        opts.append("APPEND_SUBDATASET=YES")
    ds = gdal.GetDriverByName("GPKG").CreateCopy(str(DIR / name), mem, options=opts)
    if overviews:
        ds.BuildOverviews("NEAREST", list(overviews))
    ds = None


def dem(w, h, gt, dtype, nodata=None, seed=50, base=850.0):
    """A deterministic height model: waves and a slope from `base`, whole numbers for an integer type, a 64-bit one
    with a fraction a 32-bit float rounds; a block of `nodata` near its top-left corner."""
    rng = np.random.default_rng(seed)
    j, i = np.mgrid[0:h, 0:w]
    v = base + 40.0 * np.sin(i / 37.0) + 25.0 * np.cos(j / 29.0) + (i + j) * 0.05 + rng.random((h, w)) * 0.25
    if dtype in (gdal.GDT_UInt16, gdal.GDT_Int16, gdal.GDT_Int32):
        v = np.round(v)
        if dtype == gdal.GDT_Int32:
            v = v + (i * 7 + j * 13)
    elif dtype == gdal.GDT_Float64:
        v = v + 1e-7 * (i + 1)
    if nodata is not None:
        v[5:12, 7:20] = nodata
    ds = gdal.GetDriverByName("MEM").Create("", w, h, 1, dtype)
    ds.SetGeoTransform(gt)
    ds.SetProjection(wkt(SRID))
    b = ds.GetRasterBand(1)
    b.WriteArray(v)
    if nodata is not None:
        b.SetNoDataValue(nodata)
    return ds


def edit(name, fn):
    c = sqlite3.connect(DIR / name)
    fn(c)
    c.commit()
    c.close()


def png_bytes(w, h, bands, fill=None, depth=8):
    """A PNG without a library: `bands` 1 (grey), 3 or 4 of 8 or 16 bits, flat colour `fill`."""
    ct = {1: 0, 3: 2, 4: 6}[bands]
    px = bytes(fill or [128] * bands)
    if depth == 16:
        px = b"".join(bytes([v, v]) for v in px)
    raw = b"".join(b"\0" + px * w for _ in range(h))

    def chunk(kind, data):
        c = kind + data
        return struct.pack(">I", len(data)) + c + struct.pack(">I", zlib.crc32(c))

    ihdr = struct.pack(">IIBBBBB", w, h, depth, ct, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b"")


def gdal_coverages():
    """ADR 0627 §2: coverages that try GDAL's reading rules — its own 16-bit one with the nodata −32768 (tiles offset
    by 1), a scaled one of precision 0.01 (not rounded), and copies edited to the offset −32767 with the null 65535,
    a 16-bit null below 32768, a null that is no stored value and a float coverage whose scale GDAL does not apply."""
    g1 = [X0, 1.0, 0, Y0, 0, -1.0]
    gpkg_from("kapsam-i16-eksi.gpkg", dem(300, 260, g1, gdal.GDT_Int16, nodata=-32768, seed=55, base=-120.0),
              "dem", tile_format="PNG", overviews=())
    ds = gdal.GetDriverByName("GPKG").CreateCopy(
        str(DIR / "kapsam-hassas.gpkg"), dem(300, 260, g1, gdal.GDT_Float32, seed=56),
        options=["RASTER_TABLE=dem", "TILE_FORMAT=PNG", "PRECISION=0.01"],
    )
    ds = None
    for name, sql in (
        ("kapsam-32767.gpkg", "update gpkg_2d_gridded_coverage_ancillary set offset=-32767, data_null=65535"),
        ("kapsam-kucuk-null.gpkg", "update gpkg_2d_gridded_coverage_ancillary set data_null=100"),
        ("kapsam-gecersiz-null.gpkg", "update gpkg_2d_gridded_coverage_ancillary set data_null=-5"),
    ):
        shutil.copy(DIR / "kapsam-i16.gpkg", DIR / name)
        edit(name, lambda c, sql=sql: c.execute(sql))
    shutil.copy(DIR / "kapsam-float.gpkg", DIR / "kapsam-float-olcek.gpkg")
    edit("kapsam-float-olcek.gpkg", lambda c: c.execute(
        "update gpkg_2d_gridded_coverage_ancillary set scale=2.0, offset=5.0"))


def files():
    DIR.mkdir(parents=True, exist_ok=True)
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    for old in DIR.glob("*.gpkg"):
        old.unlink()
    # Read side ---------------------------------------------------------------------------------------------
    gt = [X0, 2.0, 0, Y0, 0, -2.0]
    orto = mem_raster(600, 420, 4, gt, seed=11)
    gpkg_from("orto.gpkg", orto, "orto")
    gpkg_from("gri.gpkg", mem_raster(300, 200, 1, [X0, 1.0, 0, Y0, 0, -1.0], seed=12), "gri", band_count=1)
    # The same with plain grey PNG tiles (one band, no alpha).

    shutil.copy(DIR / "gri.gpkg", DIR / "gri-saf.gpkg")

    def plain_grey(c):
        import io

        from PIL import Image

        for tid, data in c.execute("select id, tile_data from gri").fetchall():
            buf = io.BytesIO()
            Image.open(io.BytesIO(data)).convert("L").save(buf, "PNG")
            c.execute("update gri set tile_data=? where id=?", (buf.getvalue(), tid))

    edit("gri-saf.gpkg", plain_grey)
    gpkg_from("jpeg.gpkg", mem_raster(300, 200, 3, [X0, 1.0, 0, Y0, 0, -1.0], seed=13), "foto", tile_format="JPEG", overviews=())
    gpkg_from("iki-tablo.gpkg", mem_raster(300, 200, 4, [X0, 1.0, 0, Y0, 0, -1.0], seed=14), "a", overviews=(2,))
    gpkg_from("iki-tablo.gpkg", mem_raster(260, 260, 4, [X0 + 400, 1.0, 0, Y0, 0, -1.0], seed=15), "b", append=True, overviews=(2,))
    # The window of a table whose data box is not on its pixels (GDAL's own rounding).
    gpkg_from("kaymis.gpkg", mem_raster(600, 420, 4, gt, seed=16), "orto")
    edit(
        "kaymis.gpkg",
        lambda c: c.execute(
            "update gpkg_contents set min_x=487203.7, min_y=4419800.9, max_x=488197.1, max_y=4420511.3"
        ),
    )
    # A tile missing.
    gpkg_from("eksik.gpkg", mem_raster(600, 420, 4, gt, seed=17), "orto")
    edit("eksik.gpkg", lambda c: c.execute("delete from orto where zoom_level=2 and tile_column=1 and tile_row=0"))
    # An undefined system.
    gpkg_from("sistemsiz.gpkg", mem_raster(300, 200, 4, [0.0, 1.0, 0, 200.0, 0, -1.0], srid=0, seed=18), "yerel", overviews=())

    def undefine(c):
        c.execute("update gpkg_contents set srs_id=-1")
        c.execute("update gpkg_tile_matrix_set set srs_id=-1")

    edit("sistemsiz.gpkg", undefine)
    # Tiles that are not what the reader takes.
    base = [X0, 1.0, 0, Y0, 0, -1.0]
    for name, replace in (
        ("webp.gpkg", b"RIFF\x24\x00\x00\x00WEBPVP8 \x18\x00\x00\x00"),
        ("bozuk-karo.gpkg", b"GIF89a\x01\x00\x01\x00\x00\x00"),
        ("boy-uyumsuz.gpkg", png_bytes(100, 100, 4)),
        ("png16.gpkg", png_bytes(256, 256, 4, depth=16)),
    ):
        gpkg_from(name, mem_raster(200, 100, 4, base, seed=19), "karo", overviews=())
        edit(name, lambda c, r=replace: c.execute("update karo set tile_data=? where tile_column=0 and tile_row=0", (r,)))
    # WebP is also told by its extension row.
    edit(
        "webp.gpkg",
        lambda c: (
            c.execute(
                "create table if not exists gpkg_extensions (table_name text, column_name text, extension_name text not null, "
                "definition text not null, scope text not null, constraint ge_tce unique (table_name, column_name, extension_name))"
            ),
            c.execute(
                "insert into gpkg_extensions values ('karo','tile_data','gpkg_webp','GeoPackage 1.3 ... http://www.geopackage.org/spec/#extension_tiles_webp','read-write')"
            ),
        ),
    )
    # A table of tiles that holds none at its zoom levels.
    gpkg_from("bos.gpkg", mem_raster(200, 100, 4, base, seed=20), "karo", overviews=())
    edit("bos.gpkg", lambda c: c.execute("delete from karo"))
    # A matrix far too large to read whole: one tile of a 400 × 400 tile grid of 0.5 m pixels (102 400 pixels a side).
    gpkg_from("genis.gpkg", mem_raster(256, 256, 4, [X0, 0.5, 0, Y0, 0, -0.5], seed=21), "genis", overviews=())

    def widen(c):
        c.execute("update gpkg_tile_matrix set matrix_width=400, matrix_height=400")
        c.execute(
            "update gpkg_tile_matrix_set set max_x=min_x+400*256*0.5, min_y=max_y-400*256*0.5"
        )
        c.execute("update gpkg_contents set max_x=min_x+400*256*0.5, min_y=max_y-400*256*0.5")

    edit("genis.gpkg", widen)
    # ADR 0627: WebP pictures (GDAL writes them lossy), PNG and JPEG in one table (the opaque tiles JPEG).
    gpkg_from("webp-gercek.gpkg", mem_raster(300, 200, 3, [X0, 1.0, 0, Y0, 0, -1.0], seed=41), "foto", tile_format="WEBP", overviews=())
    mixed = mem_raster(600, 300, 4, [X0, 1.0, 0, Y0, 0, -1.0], seed=42)
    alpha = mixed.GetRasterBand(4).ReadAsArray()
    alpha[:, :256] = 255
    mixed.GetRasterBand(4).WriteArray(alpha)
    gpkg_from("karisik.gpkg", mixed, "karisik", tile_format="PNG_JPEG", overviews=())
    # Gridded coverages (OGC 17-066r1): floats with nodata in TIFF tiles and their overviews, 16-bit integers with
    # nodata, signed 16-bit integers (offset −32768), floats as scaled integers in PNG tiles (each tile's scale and
    # offset), and an integer coverage with an 8-bit tile.
    gpkg_from("kapsam-float.gpkg", dem(500, 400, [X0, 2.0, 0, Y0, 0, -2.0], gdal.GDT_Float32, nodata=-9999.0, seed=50),
              "dem", tile_format="TIFF", overviews=(2, 4))
    gpkg_from("kapsam-u16.gpkg", dem(300, 260, [X0, 1.0, 0, Y0, 0, -1.0], gdal.GDT_UInt16, nodata=65000, seed=51),
              "dem", tile_format="PNG", overviews=())
    gpkg_from("kapsam-i16.gpkg", dem(300, 260, [X0, 1.0, 0, Y0, 0, -1.0], gdal.GDT_Int16, seed=52, base=-120.0),
              "dem", tile_format="PNG", overviews=())
    gpkg_from("kapsam-olcekli.gpkg", dem(300, 260, [X0, 1.0, 0, Y0, 0, -1.0], gdal.GDT_Float32, seed=53),
              "dem", tile_format="PNG", overviews=())
    shutil.copy(DIR / "kapsam-u16.gpkg", DIR / "kapsam-bozuk.gpkg")
    edit("kapsam-bozuk.gpkg", lambda c: c.execute(
        "update dem set tile_data=? where tile_column=0 and tile_row=0", (png_bytes(256, 256, 1),)))
    gdal_coverages()
    # A pyramid on the GoogleMapsCompatible grid (EPSG:3857): its data far from the grid's origin, so a linked window
    # moves to a multiple of its coarser zooms.
    merc = gdal.GetDriverByName("MEM").Create("", 700, 500, 3, gdal.GDT_Byte)
    merc.SetGeoTransform([3_600_013.7, 4.0, 0, 4_849_978.7, 0, -4.0])
    merc.SetProjection(wkt(3857))
    for b, arr in enumerate(picture(700, 500, 54, 3)):
        merc.GetRasterBand(b + 1).WriteArray(arr)
    ds = gdal.GetDriverByName("GPKG").CreateCopy(
        str(DIR / "merkator.gpkg"), merc,
        options=["RASTER_TABLE=merkator", "TILE_FORMAT=PNG", "TILING_SCHEME=GoogleMapsCompatible", "RESAMPLING=NEAREST"],
    )
    ds.BuildOverviews("NEAREST", [2, 4, 8])
    ds = None
    # A file that is a SQLite database but no GeoPackage, and one that is no database.
    p = DIR / "sqlite.db"
    if p.exists():
        p.unlink()
    c = sqlite3.connect(p)
    c.execute("create table t (a integer)")
    c.commit()
    c.close()
    (DIR / "metin.txt").write_text("bu bir GeoPackage değil\n", encoding="utf-8")
    # Write side: GeoTIFF sources ----------------------------------------------------------------------------
    tifs = DIR / "kaynak"
    tifs.mkdir(exist_ok=True)

    def tif(name, w, h, bands, gt, srid=SRID, dtype=gdal.GDT_Byte, seed=30, nodata=None, alpha=False, palette=False):
        mem = mem_raster(w, h, bands, gt, srid=srid, seed=seed, dtype=dtype)
        opts = ["TILED=YES", "BLOCKXSIZE=64", "BLOCKYSIZE=64", "COMPRESS=DEFLATE"]
        if bands == 2 and alpha:
            mem.GetRasterBand(2).SetColorInterpretation(gdal.GCI_AlphaBand)
            opts.append("ALPHA=YES")
        if nodata is not None:
            mem.GetRasterBand(1).SetNoDataValue(nodata)
        if palette:
            ct = gdal.ColorTable()
            for k in range(256):
                ct.SetColorEntry(k, (k, 255 - k, (k * 7) % 256, 255))
            mem.GetRasterBand(1).SetRasterColorTable(ct)
            mem.GetRasterBand(1).SetColorInterpretation(gdal.GCI_PaletteIndex)
        ds = gdal.GetDriverByName("GTiff").CreateCopy(str(tifs / name), mem, options=opts)
        ds = None

    g15 = [X0, 1.5, 0, Y0, 0, -1.5]
    tif("rgb.tif", 700, 300, 3, g15, seed=31)
    tif("rgba.tif", 300, 200, 4, [X0, 0.5, 0, Y0, 0, -0.5], seed=32)
    tif("gri.tif", 256, 256, 1, [X0, 0.25, 0, Y0, 0, -0.25], seed=33)
    tif("gri-alfa.tif", 130, 90, 2, [X0, 2.0, 0, Y0, 0, -3.0], seed=34, alpha=True)
    tif("nodata.tif", 120, 100, 1, [X0, 1.0, 0, Y0, 0, -1.0], seed=35, nodata=0.0)
    tif("sistemsiz.tif", 64, 64, 3, [0.0, 1.0, 0, 64.0, 0, -1.0], srid=0, seed=36)
    tif("cografi.tif", 100, 100, 3, [32.0, 0.001, 0, 40.0, 0, -0.001], srid=4326, seed=37)
    tif("on-alti-bit.tif", 64, 64, 1, [X0, 1.0, 0, Y0, 0, -1.0], dtype=gdal.GDT_UInt16, seed=38)
    tif("paletli.tif", 64, 64, 1, [X0, 1.0, 0, Y0, 0, -1.0], seed=39, palette=True)
    tif("bos-bir.tif", 5, 3, 3, [X0, 10.0, 0, Y0, 0, -10.0], seed=40)
    # ADR 0627: heights for the gridded coverages written.
    g1 = [X0, 1.0, 0, Y0, 0, -1.0]
    for name, w, h, dtype, nodata, seed, base in (
        ("dem-u16.tif", 300, 280, gdal.GDT_UInt16, 65000, 61, 850.0),
        ("dem-i16.tif", 300, 280, gdal.GDT_Int16, None, 62, -120.0),
        ("dem-i16-nodata.tif", 200, 150, gdal.GDT_Int16, -9999, 63, -120.0),
        ("dem-f32.tif", 520, 300, gdal.GDT_Float32, -9999.0, 64, 850.0),
        ("dem-f64.tif", 100, 80, gdal.GDT_Float64, None, 65, 850.0),
        ("dem-i32.tif", 100, 80, gdal.GDT_Int32, None, 66, 20_000_000.0),
    ):
        gdal.GetDriverByName("GTiff").CreateCopy(
            str(tifs / name), dem(w, h, g1, dtype, nodata=nodata, seed=seed, base=base),
            options=["TILED=YES", "BLOCKXSIZE=64", "BLOCKYSIZE=64", "COMPRESS=DEFLATE"],
        )
    for aux in tifs.glob("*.aux.xml"):
        aux.unlink()
    for aux in DIR.glob("*.aux.xml"):
        aux.unlink()


# ── The rules (ADR 0556) ───────────────────────────────────────────────────────────────────────────────────────


class Refused(Exception):
    def __init__(self, code, message, path):
        super().__init__(message)
        self.code, self.message, self.path = code, message, path


def real(text):
    if text.startswith("$fixture:"):
        return str(FIX / text[len("$fixture:") :])
    return text


def name_of(text):
    if text.startswith("$"):
        text = text.split(":", 1)[1]
    return text.rstrip("/").rsplit("/", 1)[-1]


def fx(rel):
    return f"$fixture:gpkg-tiles/v1/{rel}"


def tmp(name):
    return f"$tmp:{name}"


def given(text, drawing, at):
    t = text.strip()
    if not t:
        raise Refused("invalid_file_path", "Dosya yolu boş; bir dosyanın yolunu verin.", at)
    if t.startswith("/") or t.startswith("$"):
        return t
    if drawing is None:
        raise Refused(
            "invalid_file_path",
            f"“{t}” göreli bir yol, ama çizimin dosyası yok; mutlak yolu verin ya da çizimi önce kaydedin.",
            at,
        )
    folder = drawing.rsplit("/", 1)[0] if "/" in drawing else drawing
    if drawing.startswith("$tmp:") and "/" not in drawing:
        folder = "$tmp:"
    return (folder + "/" + t) if not folder.endswith(":") else folder + t


def is_dir(text):
    """Whether the folder of a path is there: a test run's own folder is; one of fixtures is when it is a folder."""
    if text.startswith("$tmp:"):
        return text == "$tmp:" or text.rstrip("/") == "$tmp:"
    return os.path.isdir(real(text))


def parent_of(text):
    if text.startswith("$tmp:") and "/" not in text:
        return "$tmp:"
    return text.rsplit("/", 1)[0] if "/" in text else ""


def sniff(data):
    if data.startswith(b"\x89PNG\r\n\x1a\n"):
        return "png"
    if data.startswith(b"\xff\xd8\xff"):
        return "jpeg"
    if len(data) >= 12 and data[0:4] == b"RIFF" and data[8:12] == b"WEBP":
        return "webp"
    if data.startswith(b"II*\0") or data.startswith(b"MM\0*"):
        return "tiff"
    return "other"


def tile_tables(path):
    c = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    sets = []
    rows = c.execute(
        "select c.table_name, c.identifier, c.min_x, c.min_y, c.max_x, c.max_y, s.srs_id, s.min_x, s.min_y, s.max_x, s.max_y, "
        "c.data_type from gpkg_contents c join gpkg_tile_matrix_set s on s.table_name = c.table_name "
        "where c.data_type in ('tiles', '2d-gridded-coverage') order by c.rowid"
    ).fetchall()
    has_ext = c.execute("select count(*) from sqlite_master where name='gpkg_extensions'").fetchone()[0] > 0
    has_cov = c.execute("select count(*) from sqlite_master where name='gpkg_2d_gridded_coverage_ancillary'").fetchone()[0] > 0
    for name, ident, dx0, dy0, dx1, dy1, srs, bx0, by0, bx1, by1, kind in rows:
        org = c.execute("select organization, organization_coordsys_id from gpkg_spatial_ref_sys where srs_id=?", (srs,)).fetchone()
        srid = org[1] if srs > 0 and org and org[0].upper() == "EPSG" else None
        zooms = []
        for z, mw, mh, tw, th, px, py in c.execute(
            "select zoom_level, matrix_width, matrix_height, tile_width, tile_height, pixel_x_size, pixel_y_size "
            "from gpkg_tile_matrix where table_name=? order by zoom_level",
            (name,),
        ).fetchall():
            n = c.execute(f'select count(*) from "{name}" where zoom_level=?', (z,)).fetchone()[0]
            heads = [r[0] for r in c.execute(f'select substr(tile_data,1,12) from "{name}" where zoom_level=? limit 64', (z,))]
            formats = sorted({sniff(h) for h in heads})
            zoom = {
                "zoom": z,
                "matrixWidth": mw,
                "matrixHeight": mh,
                "tileWidth": tw,
                "tileHeight": th,
                "pixel": [float(px), float(py)],
                "tiles": n,
            }
            if formats:
                zoom["formats"] = formats
            zooms.append(zoom)
        exts = []
        if has_ext:
            exts = [r[0] for r in c.execute("select extension_name from gpkg_extensions where table_name=? order by rowid", (name,))]
        t = {
            "table": name,
            "srsId": srs,
            "bounds": [float(bx0), float(by0), float(bx1), float(by1)],
            "zooms": zooms,
        }
        if ident is not None:
            t["identifier"] = ident
        if srid is not None:
            t["srid"] = srid
        if None not in (dx0, dy0, dx1, dy1):
            t["dataBounds"] = [float(dx0), float(dy0), float(dx1), float(dy1)]
        if kind == "2d-gridded-coverage":
            row = None
            if has_cov:
                row = c.execute(
                    "select datatype, scale, offset, data_null, uom from gpkg_2d_gridded_coverage_ancillary "
                    "where tile_matrix_set_name=?", (name,)).fetchone()
            datatype, scale, offset, null, uom = row or ("integer", 1.0, 0.0, None, None)
            cov = {"datatype": "float" if datatype.lower() == "float" else "integer", "scale": float(scale), "offset": float(offset)}
            if null is not None:
                cov["dataNull"] = float(null)
            if uom is not None:
                cov["uom"] = uom
            t["coverage"] = cov
        if exts:
            t["extensions"] = exts
        sets.append(t)
    c.close()
    return sets


def content_of(path, t):
    """What a table holds as the reader reads it (ADR 0627 §2): pictures, or a coverage's band as GDAL 3.12's GPKG
    driver reads it (ogrgeopackagedatasource.cpp, gdalgeopackagerasterband.cpp): u16 for an integer coverage of scale
    1 and offset 0 whose tiles are all of scale 1 and offset 0; i16 for one offset by −32768 (its tiles so, or offset
    0 or 1 with the null 65535) or by −32767 with the null 65535; f32 otherwise. An integer null is a stored value, a
    whole number 0…65535 (GDAL ignores another); the nodata is it (u16), it or −32768 above 32767 (i16), the lowest
    float (f32), a float coverage's null as a 32-bit float."""
    cov = t.get("coverage")
    if cov is None:
        return {"kind": "image", "bands": 4, "sample": "u8", "nodata": None}
    integer = cov["datatype"] == "integer"
    c = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    has = c.execute("select count(*) from sqlite_master where name='gpkg_2d_gridded_tile_ancillary'").fetchone()[0]

    def none_but(cond):
        return not has or c.execute(
            f"select count(*) from gpkg_2d_gridded_tile_ancillary where tpudt_name=? and not ({cond})", (t["table"],)
        ).fetchone()[0] == 0

    plain = none_but("offset = 0.0 and scale = 1.0")
    plain_or_one = none_but("(offset = 0.0 or offset = 1.0) and scale = 1.0")
    row = c.execute(
        "select precision from gpkg_2d_gridded_coverage_ancillary where tile_matrix_set_name=?", (t["table"],)
    ).fetchone()
    c.close()
    precision = 1.0 if row is None or row[0] is None else float(row[0])
    null = cov.get("dataNull")
    if integer and null is not None and not (0 <= null <= 65535 and float(null).is_integer()):
        null = None
    high = null == 65535.0
    scale, offset = cov["scale"], cov["offset"]
    if not integer or scale != 1.0:
        sample = "f32"
    elif offset == 0.0 and plain:
        sample = "u16"
    elif offset == -32768.0 and (plain_or_one if high else plain):
        sample = "i16"
    elif offset == -32767.0 and high and plain:
        sample = "i16"
    else:
        sample = "f32"
    nodata = None
    if null is not None:
        if not integer:
            nodata = float(np.float32(null))
        elif sample == "u16":
            nodata = null
        elif sample == "i16":
            nodata = -32768.0 if null > 32767 else null
        else:
            nodata = FLT_MIN
    return {"kind": "coverage", "integer": integer, "scale": scale, "offset": offset, "null": null,
            "precision": precision, "bands": 1, "sample": sample, "nodata": nodata}


def tile_problem(content, data, tw, th):
    """A tile's problem before its pixels are read (ADR 0627 §1): (unsupported, words), or none."""
    import io

    from PIL import Image

    f = sniff(data)
    if f == "other":
        return False, "PNG, JPEG, WebP ya da TIFF değil"
    size = None
    if content["kind"] == "image":
        if f == "png":
            if data[24] > 8:
                return True, "8 bitten fazla bir PNG; resim karoları 8 bitlik PNG, JPEG ya da WebP olur"
            size = struct.unpack(">II", data[16:24])
        elif f == "jpeg":
            try:
                size = Image.open(io.BytesIO(data)).size
            except Exception:  # noqa: BLE001
                return False, "JPEG'in başlığı okunmuyor"
        elif f == "webp":
            try:
                size = Image.open(io.BytesIO(data)).size
            except Exception:  # noqa: BLE001
                return False, "WebP'nin başlığı okunmuyor"
        else:
            return True, "TIFF; TIFF karo yalnız yükseklik (kapsam) tablosunda olur"
    elif content["integer"]:
        if f != "png" or data[24] != 16 or data[25] != 0:
            return True, "tam sayılı kapsamın karosu 16 bitlik gri PNG olur"
        size = struct.unpack(">II", data[16:24])
    else:
        if f != "tiff":
            return True, "kayan noktalı kapsamın karosu 32 bitlik TIFF olur"
        try:
            size = Image.open(io.BytesIO(data)).size
        except Exception:  # noqa: BLE001
            gdal.FileFromMemBuffer("/vsimem/karo.tif", bytes(data))
            ds = gdal.Open("/vsimem/karo.tif")
            size = (ds.RasterXSize, ds.RasterYSize)
            ds = None
            gdal.Unlink("/vsimem/karo.tif")
    if tuple(size) != (tw, th):
        return False, f"{size[0]} × {size[1]} piksel, matrisin karo boyu {tw} × {th}"
    return None


def linked_window(zooms, zoom0, w):
    """A linked raster's window (ADR 0627 §3): the first pixel moved up and left to a multiple of 2^K, K the coarsest
    zoom of twice the pixel before it can draw from (at most the level of one tile)."""
    z0 = next(z for z in zooms if z["zoom"] == zoom0)
    n = len(level_sizes(w["w"], w["h"]))

    def has(f):
        return any(
            abs(z["pixel"][0] - z0["pixel"][0] * f) <= 1e-9 * abs(z0["pixel"][0] * f)
            and abs(z["pixel"][1] - z0["pixel"][1] * f) <= 1e-9 * abs(z0["pixel"][1] * f)
            for z in zooms
        )

    k = 0
    while k + 1 < n and has(2 ** (k + 1)):
        k += 1
    step = 2**k
    ax, ay = w["x"] - w["x"] % step, w["y"] - w["y"] % step
    return {"x": ax, "y": ay, "w": w["w"] + (w["x"] - ax), "h": w["h"] + (w["y"] - ay)}


def level_sizes(w, h):
    """The geometry core's levels of a raster: halved until a side fits a 256-pixel tile."""
    out = [(w, h)]
    while w > 256 or h > 256:
        w, h = -(-w // 2), -(-h // 2)
        out.append((w, h))
    return out


def window(m, tms, rect):
    px, py = m["pixel"]
    x0, y1 = tms[0], tms[3]
    fx_ = math.floor((rect[0] - x0) / px + 0.5)
    fy_ = math.floor((y1 - rect[3]) / py + 0.5)
    wide = math.floor((rect[2] - rect[0]) / px + 0.5)
    high = math.floor((rect[3] - rect[1]) / py + 0.5)
    gw = m["matrixWidth"] * m["tileWidth"]
    gh = m["matrixHeight"] * m["tileHeight"]
    xa, xb = max(fx_, 0), min(fx_ + wide, gw)
    ya, yb = max(fy_, 0), min(fy_ + high, gh)
    if not (xb > xa and yb > ya):
        return None
    return {"x": xa, "y": ya, "w": xb - xa, "h": yb - ya, "origin": [x0 + xa * px, y1 - ya * py]}


def read_layer_checks(doc, layer, at="layer"):
    node = next((l for l in doc["layers"] if l["id"] == layer), None)
    if node is None:
        raise Refused("layer_not_found", f"“{layer}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", at)
    if node["type"] != "layer":
        raise Refused(
            "not_a_layer",
            f"“{node['name']}” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.",
            at,
        )
    if node["locked"]:
        raise Refused(
            "layer_locked",
            f"“{node['name']}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.",
            at,
        )
    if not node["visible"]:
        return [{"code": "layer_hidden", "message": f"“{node['name']}” katmanı gizli; çizilen nesne görünmeyecek.", "path": at}]
    return []


def revision_check(inp):
    exp = inp.get("expectedRevision")
    if exp is not None and not (exp.isdigit() and (exp == "0" or not exp.startswith("0"))):
        raise Refused(
            "invalid_revision",
            f"Beklenen sürüm “{exp}” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.",
            "expectedRevision",
        )
    if exp is not None and exp != "0":
        raise Refused(
            "revision_conflict",
            "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
            "expectedRevision",
        )


def read_ref(doc, inp, exists=()):
    """`cad.rasters.gpkgread`'s answer: (output, warnings, added) or Refused."""
    drawing = inp.get("drawing")
    path = given(inp["path"], drawing, "path")
    target = None
    if inp.get("output") is not None:
        target = given(inp["output"], drawing, "output")
        low = target.lower()
        if not (low.endswith(".tif") or low.endswith(".tiff")):
            raise Refused("invalid_file_path", f"“{name_of(inp['output'])}” bir .tif dosyası değil; raster GeoTIFF olarak yazılır.", "output")
        parent = parent_of(target)
        if not is_dir(parent):
            raise Refused("invalid_file_path", f"“{parent}” klasörü yok; raster var olan bir klasöre yazılır.", "output")
    link = inp.get("link") is True
    if link and (target is not None or inp.get("layer") is None):
        raise Refused(
            "link_unusable",
            "Bağlı okuma (link) GeoTIFF yazmaz: karolar dosyadan okunur; output'u vermeyin."
            if target is not None
            else "Bağlı okuma (link) rasteri bir katmana ekler; layer'ı verin.",
            "link",
        )
    if inp.get("layer") is not None and target is None and not link:
        raise Refused(
            "invalid_file_path",
            "layer verildi ama output yok; rasteri eklemek için yazılacak GeoTIFF'in yolunu (output) verin ya da tabloya bağlı ekleyin (link).",
            "output",
        )
    if not os.path.isfile(real(path)):
        raise Refused("file_not_found", f"{path} bulunamadı; GeoPackage'ın yolunu denetleyin.", "path")
    name = name_of(inp["path"])
    with open(real(path), "rb") as f:
        head = f.read(16)
    if not head.startswith(b"SQLite format 3\0"):
        raise Refused("import_unreadable", f"{name} GeoPackage olarak okunamadı: “{path}” bir SQLite dosyası değil", "path")
    c = sqlite3.connect(f"file:{real(path)}?mode=ro", uri=True)
    has = c.execute("select count(*) from sqlite_master where type='table' and name='gpkg_contents'").fetchone()[0]
    app = c.execute("pragma application_id").fetchone()[0]
    c.close()
    if app not in (0x47504B47, 0x47503130) and has == 0:
        raise Refused(
            "import_unreadable",
            f"{name} GeoPackage olarak okunamadı: “{path}” bir GeoPackage değil: gpkg_contents tablosu ve GPKG kimliği yok",
            "path",
        )
    tables = tile_tables(real(path))
    if not tables:
        raise Refused(
            "no_tile_table",
            f"{name} dosyasında karo (raster) tablosu yok; GeoPackage'ın yalnız vektör ve öznitelik tabloları var.",
            "path",
        )
    names = ", ".join(t["table"] for t in tables)
    if inp.get("table") is not None:
        t = next((t for t in tables if t["table"] == inp["table"]), None)
        if t is None:
            raise Refused("table_not_found", f"“{inp['table']}” dosyada bir karo tablosu değil; karo tabloları: {names}.", "table")
    elif len(tables) == 1:
        t = tables[0]
    else:
        raise Refused("table_not_found", f"Dosyada {len(tables)} karo tablosu var; table ile birini seçin: {names}.", "table")
    zoom = inp["zoom"] if inp.get("zoom") is not None else t["zooms"][-1]["zoom"]
    m = next((z for z in t["zooms"] if z["zoom"] == zoom), None)
    if m is None:
        zs = ", ".join(str(z["zoom"]) for z in t["zooms"])
        raise Refused("zoom_not_found", f"“{t['table']}” tablosunun {zoom}. zoom düzeyi yok; düzeyler: {zs}.", "zoom")
    area = inp.get("area")
    if area is not None and not (all(math.isfinite(v) for v in area) and area[0] < area[2] and area[1] < area[3]):
        raise Refused(
            "invalid_area",
            "Alan [en küçük x, en küçük y, en büyük x, en büyük y] olmalı: sonlu sayılar, en küçükler en büyüklerden küçük.",
            "area",
        )
    rect = area or t.get("dataBounds") or t["bounds"]
    empty_at = "area" if area else "table"
    w = window(m, t["bounds"], rect)
    if w is None:
        raise Refused(
            "tile_table_empty",
            f"Okunacak alan “{t['table']}” tablosunun {zoom}. düzeyinin ızgarasının dışında; alanı ızgaraya çekin.",
            empty_at,
        )
    content = content_of(real(path), t)
    px, py = m["pixel"]
    if link:
        # Only zooms that hold tiles are levels (GDAL's overviews are those too); a zoom without tiles leaves it.
        held = [z for z in t["zooms"] if z["tiles"] > 0]
        if any(z["zoom"] == zoom for z in held):
            w = linked_window(held, zoom, w)
        w["origin"] = [t["bounds"][0] + w["x"] * px, t["bounds"][3] - w["y"] * py]
    too_at = "area" if (inp.get("zoom") is not None or area is not None) else "zoom"
    tw, th = m["tileWidth"], m["tileHeight"]
    c0, c1 = w["x"] // tw, (w["x"] + w["w"] - 1) // tw
    r0, r1 = w["y"] // th, (w["y"] + w["h"] - 1) // th
    expected = (c1 - c0 + 1) * (r1 - r0 + 1)
    if target is not None and w["w"] * w["h"] > MOST_PIXELS:
        raise Refused(
            "gpkg_tiles_too_large",
            f"Raster {w['w']} × {w['h']} piksel olurdu; GeoTIFF olarak en çok 4 milyar piksel okunur. Daha kaba bir zoom düzeyi ya da dar bir alan verin, ya da tabloya bağlı ekleyin (link).",
            too_at,
        )
    if link and expected > 2**32 - 1:
        raise Refused(
            "gpkg_tiles_too_large",
            f"Pencerede {expected} karo var; bir düzeyde en çok 4 milyar karo okunur. Dar bir alan verin.",
            too_at,
        )
    c = sqlite3.connect(f"file:{real(path)}?mode=ro", uri=True)
    read = c.execute(
        f'select count(*) from "{t["table"]}" where zoom_level=? and tile_column between ? and ? and tile_row between ? and ?',
        (zoom, c0, c1, r0, r1),
    ).fetchone()[0]
    sample = c.execute(
        f'select tile_column, tile_row, tile_data from "{t["table"]}" where zoom_level=? and tile_column between ? and ? '
        "and tile_row between ? and ? order by tile_row, tile_column limit 64",
        (zoom, c0, c1, r0, r1),
    ).fetchall()
    c.close()
    if read == 0:
        raise Refused(
            "tile_table_empty",
            f"“{t['table']}” tablosunun {zoom}. düzeyinde okunan pencerede hiç karo yok.",
            empty_at,
        )
    if target is not None or link:
        for col, row, data in sample:
            p = tile_problem(content, data, tw, th)
            if p is not None:
                raise Refused(
                    "tile_format_unsupported" if p[0] else "tile_unreadable",
                    f"{zoom}/{col}/{row} karosu okunmaz: {p[1]}.",
                    "table",
                )
    srid = t.get("srid", 0)
    warnings = []
    if t.get("srid") is None:
        warnings.append(
            {"code": "crs_undeclared", "message": "Karo tablosu EPSG koordinat sistemi bildirmiyor; raster sistemsiz alındı.", "path": "table"}
        )
    if read < expected:
        left = "saydam (alfa 0) kaldı" if content["kind"] == "image" else "boş (nodata) kaldı"
        warnings.append(
            {
                "code": "tiles_missing",
                "message": f"Okunan pencerenin {expected - read} karosu tabloda yok; o yerler {left}.",
                "path": "table",
            }
        )
    if target is not None and target in exists and inp.get("overwrite") is not True:
        raise Refused("file_exists", f"{target} var; üzerine yazmak için overwrite verin.", "output")
    added = False
    if inp.get("layer") is not None:
        warnings += read_layer_checks(doc, inp["layer"])
        project = doc["settings"]["srid"]
        if srid != 0 and srid != project:
            raise Refused(
                "crs_mismatch",
                f"Karo tablosunun sistemi EPSG:{srid}, projeninki {'yerel' if project == 0 else f'EPSG:{project}'}; raster çevrilmeden eklenmez. Dosyayı yazmak için layer'ı vermeyin.",
                "layer",
            )
        added = True
    revision_check(inp)
    ox, oy = w["origin"]
    out = {
        "tables": tables,
        "table": t["table"],
        "zoom": zoom,
        "area": [ox, oy - w["h"] * py, ox + w["w"] * px, oy],
        "width": w["w"],
        "height": w["h"],
        "bands": content["bands"],
        "sample": content["sample"],
        "affine": [ox, px, 0.0, oy, 0.0, -py],
        "srid": srid,
        "tilesRead": read,
        "tilesMissing": expected - read,
        "revision": "$current",
    }
    if content["nodata"] is not None:
        out["nodata"] = content["nodata"]
    if link:
        out["linked"] = True
    if target is not None:
        out["path"] = target
        out["bytes"] = "$any"
    return out, warnings, added


# The writer ---------------------------------------------------------------------------------------------------


def levels_of(w, h):
    out = []
    while True:
        out.append((w, h, -(-w // SIDE), -(-h // SIDE)))
        if w <= SIDE and h <= SIDE:
            return out
        w, h = -(-w // 2), -(-h // 2)


def table_ok(name):
    low = name.lower()
    return (
        bool(name)
        and (name[0].isascii() and (name[0].isalpha() or name[0] == "_"))
        and all(ch.isascii() and (ch.isalnum() or ch == "_") for ch in name)
        and len(name) <= 64
        and not low.startswith(("gpkg_", "gpkgext_", "sqlite_", "rtree_"))
    )


def src_header(path):
    ds = gdal.Open(real(path))
    b1 = ds.GetRasterBand(1)
    alpha = any(ds.GetRasterBand(b + 1).GetColorInterpretation() == gdal.GCI_AlphaBand for b in range(ds.RasterCount))
    return {
        "w": ds.RasterXSize,
        "h": ds.RasterYSize,
        "bands": ds.RasterCount,
        "type": gdal.GetDataTypeName(b1.DataType),
        "palette": b1.GetRasterColorTable() is not None,
        "alpha": alpha,
        "nodata": b1.GetNoDataValue(),
    }


def write_ref(doc, inp, copied=(), exists=(), executed=True):
    """`cad.rasters.gpkgwrite`'s answer: (output, warnings) or Refused. `doc` is the document (entities by id)."""
    uid = inp["uid"]
    if uid.startswith("$uidOf:"):
        ent = UIDS[uid]
    elif not _uid_ok(uid):
        raise Refused("invalid_uid", f"“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir.", "uid")
    else:
        raise Refused("entity_not_found", f"“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir.", "uid")
    if ent.get("kind") != "raster":
        raise Refused("not_a_raster", "Verilen nesne bir raster değil; yalnız raster karo olarak yazılır.", "uid")
    drawing = inp.get("drawing")
    path = given(inp["path"], drawing, "path")
    if not path.lower().endswith(".gpkg"):
        raise Refused("invalid_file_path", f"“{name_of(inp['path'])}” bir .gpkg dosyası değil; karolar GeoPackage olarak yazılır.", "path")
    parent = parent_of(path)
    if not is_dir(parent):
        raise Refused("invalid_file_path", f"“{parent}” klasörü yok; GeoPackage var olan bir klasöre yazılır.", "path")
    table = inp.get("table") or "tiles"
    if not table_ok(table):
        raise Refused(
            "invalid_table_name",
            f"“{table}” tablo adı olmaz: harf ya da alt çizgiyle başlamalı, yalnız harf, rakam ve alt çizgi içermeli, 64 karakteri geçmemeli ve gpkg_, sqlite_ ya da rtree_ ile başlamamalı.",
            "table",
        )

    def unsupported(why):
        return Refused("raster_unsupported", f"Raster karo olarak yazılamaz: {why}.", "uid")

    if ent.get("dataset"):
        raise unsupported("veri seti (NetCDF, mesh) gösteriyor")
    if ent.get("asset") is None and ent.get("file") is None:
        raise unsupported("kaynağı bir adres; dosyasını indirip bağlayın ya da gömün")
    coverage = ent["sample"] != "u8"
    if coverage and ent["bands"] != 1:
        raise unsupported(f"{ent['bands']} bantlı ve 8 bitten fazla (yükseklik karosu tek bantlıdır)")
    if ent["sample"] == "i8":
        raise unsupported("8 bitlik işaretli")
    if not 1 <= ent["bands"] <= 4:
        raise unsupported(f"{ent['bands']} bantlı")
    webp = inp.get("format") == "webp"
    if coverage and webp:
        raise Refused(
            "tile_format_unsupported",
            "Yükseklik (kapsam) karoları WebP yazılmaz: 16 bitlik PNG ya da 32 bitlik TIFF olur; format'ı vermeyin.",
            "format",
        )
    x0, a, b, y0, c, d = ent["affine"]
    if not (b == 0 and c == 0 and a > 0 and d < 0):
        raise Refused(
            "raster_not_north_up",
            "Raster dönük ya da kuzeyi yukarıda değil; karo ızgarası yatay ve düşey olduğu için önce Raster oturt ile yeniden örnekleyin.",
            "uid",
        )
    w, h = ent["width"], ent["height"]
    lv = levels_of(w, h)
    per = (2 if ent["sample"] in ("u16", "i16") else 4) if coverage else 4
    if lv[0][2] * SIDE * SIDE * per > STRIP_MOST:
        raise Refused(
            "gpkg_tiles_too_large",
            f"Raster {w} piksel genişliğinde; karolar 256 satırlık şeritlerle yazılır ve bir şerit en çok 256 MB olur. Rasteri bölün ya da daha kaba yeniden örnekleyin.",
            "uid",
        )
    if ent.get("asset") is not None:
        raise Refused(
            "unknown_asset",
            f"Rasterin kaynağı “{ent['asset']}” projenin kitaplığında yok: silinmiş ya da başka bir çizimin olabilir.",
            "uid",
        )
    src = ent["file"]
    if not os.path.isfile(real(src)):
        raise Refused(
            "file_not_found",
            f"Rasterin kaynağı {name_of(src)} bulunamadı. Bakılan yerler: {src}.",
            "uid",
        )
    hd = src_header(src)
    if hd["w"] != w or hd["h"] != h:
        raise Refused(
            "import_unreadable",
            f"Rasterin kaynağı {hd['w']} × {hd['h']} piksel, kayıttaki {w} × {h}; kaynak değişmiş. Rasteri yeniden ekleyin.",
            "uid",
        )
    if hd["palette"]:
        raise unsupported("paletli (önce RGB'ye çevirin)")
    gdal_type = {"u8": "Byte", "u16": "UInt16", "i16": "Int16", "u32": "UInt32", "i32": "Int32", "f32": "Float32", "f64": "Float64"}
    if hd["type"] != gdal_type.get(ent["sample"]) or hd["bands"] != ent["bands"]:
        raise unsupported("kaynağının bantları ya da veri türü kayıttakinden başka")
    nb = hd["bands"]
    nodata = hd["nodata"]
    if coverage:
        kind = coverage_kind(ent["sample"], nodata)
        tile_bands = 1
    elif nb == 1:
        tile_bands = 1
    elif nb == 2 and hd["alpha"]:
        tile_bands = 2
    elif nb == 3:
        tile_bands = 4
    elif nb == 4 and hd["alpha"]:
        tile_bands = 4
    elif nb == 2:
        raise unsupported("2 bantlı ve alfasız")
    else:
        raise unsupported("4 bantlı ve alfasız (renkli ama alfa bandı değil)")
    mode = inp.get("mode", "new")
    if mode == "new":
        if path in exists and inp.get("overwrite") is not True:
            raise Refused(
                "file_exists",
                f"{path} var; üzerine yazmak için overwrite verin ya da mode add ile tabloyu içine ekleyin.",
                "path",
            )
    else:
        if path.startswith("$tmp:") and path not in copied:
            raise Refused("file_not_found", f"{path} bulunamadı; mode add var olan bir GeoPackage'a ekler.", "path")
        if not path.startswith("$tmp:") and not os.path.isfile(real(path)):
            raise Refused("file_not_found", f"{path} bulunamadı; mode add var olan bir GeoPackage'a ekler.", "path")
        rp = real(path) if not path.startswith("$tmp:") else str(DIR / copied[path])
        with open(rp, "rb") as f:
            head = f.read(16)
        if not head.startswith(b"SQLite format 3\0"):
            raise Refused("import_unreadable", f"{name_of(inp['path'])} GeoPackage olarak okunamadı: “{path}” bir SQLite dosyası değil", "path")
        c_ = sqlite3.connect(f"file:{rp}?mode=ro", uri=True)
        has = c_.execute("select count(*) from sqlite_master where lower(name)=lower(?)", (table,)).fetchone()[0]
        c_.close()
        if has:
            raise Refused("table_exists", f"Dosyada “{table}” adlı tablo var; başka bir ad verin. Dosyaya dokunulmadı.", "table")
    warnings = []
    if nodata is not None and not coverage:
        warnings.append(
            {
                "code": "nodata_lost",
                "message": "Rasterin nodata değeri karolara yazılamaz (GeoPackage karoları alfa taşır); değer olduğu gibi piksel sayıldı.",
                "path": "uid",
            }
        )
    if coverage and kind["datatype"] == "integer" and kind["null"] is None and nodata is not None:
        warnings.append(
            {
                "code": "nodata_lost",
                "message": f"Rasterin nodata değeri ({fmt(nodata)}) 16 bitlik karonun değeri olamaz; boş değer yazılmadı.",
                "path": "uid",
            }
        )
    top = len(lv) - 1
    fmt_name = ("png" if kind["datatype"] == "integer" else "tiff") if coverage else ("webp" if webp else "png")
    zooms = []
    for j, (lw, lh, tx, ty) in enumerate(lv):
        zooms.append(
            {
                "zoom": top - j,
                "matrixWidth": 2 ** (top - j),
                "matrixHeight": 2 ** (top - j),
                "tileWidth": SIDE,
                "tileHeight": SIDE,
                "pixel": [a * 2.0**j, -d * 2.0**j],
                "tiles": tx * ty,
                "formats": [fmt_name],
            }
        )
    zooms.reverse()
    span = float(SIDE) * 2.0**top
    out = {
        "path": path,
        "table": table,
        "mode": mode,
        "srid": ent["srid"],
        "bands": tile_bands,
        "bounds": [x0, y0 - span * -d, x0 + span * a, y0],
        "dataBounds": [x0, y0 + h * d, x0 + w * a, y0],
        "zooms": zooms,
        "tiles": sum(z["tiles"] for z in zooms),
        "bytes": "$any",
        "revision": "$current",
    }
    if coverage:
        cov = {"datatype": kind["datatype"], "scale": 1.0, "offset": float(kind["offset"])}
        if kind["null"] is not None:
            cov["dataNull"] = float(kind["null"])
        out["coverage"] = cov
        if executed and kind["datatype"] == "float":
            src_ds = gdal.Open(real(src))
            values = src_ds.GetRasterBand(1).ReadAsArray().astype(np.float64)
            src_ds = None
            valid = np.isfinite(values)
            if nodata is not None:
                valid &= values != nodata
            if np.any(values[valid].astype(np.float32).astype(np.float64) != values[valid]):
                warnings.append(
                    {
                        "code": "precision_lost",
                        "message": "Rasterin bazı değerleri 32 bitlik kayan noktalı karoya yuvarlanarak yazıldı (karolar 32 bitlik TIFF'tir).",
                        "path": "uid",
                    }
                )
    return out, warnings


def coverage_kind(sample, nodata):
    """How a raster of one band of more than 8 bits is written (ADR 0627 §2): 16-bit PNG integers (a signed raster
    offset by −32768, without nodata), else 32-bit float TIFFs; the null."""
    if sample == "u16":
        null = nodata if nodata is not None and float(nodata).is_integer() and 0 <= nodata <= 65535 else None
        return {"datatype": "integer", "offset": 0, "null": null}
    if sample == "i16" and nodata is None:
        return {"datatype": "integer", "offset": -32768, "null": None}
    return {"datatype": "float", "offset": 0, "null": None if nodata is None else float(np.float32(nodata))}


def fmt(v):
    """A number as Rust's `{}` writes an f64."""
    if float(v).is_integer() and abs(v) < 1e16:
        return str(int(v))
    return repr(float(v))


UIDS = {}


def _uid_ok(text):
    if len(text) != 36:
        return False
    for i, ch in enumerate(text):
        if i in (8, 13, 18, 23):
            if ch != "-":
                return False
        elif not (ch.isdigit() or ch in "abcdef"):
            return False
    return True


# ── The cases ──────────────────────────────────────────────────────────────────────────────────────────────────


def setup(srid=SRID, entities=()):
    layer = lambda i, n, vis=True, lock=False: {  # noqa: E731
        "id": i,
        "name": n,
        "type": "layer",
        "visible": vis,
        "locked": lock,
        "expanded": True,
        "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25},
        "children": [],
    }
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Karolar",
        "settings": {
            "srid": srid,
            "lengthDecimals": 3,
            "areaDecimals": 2,
            "areaUnit": "m2",
            "angleUnit": "deg",
            "plotScale": 1000,
            "workspace": "gis",
        },
        "origin": {"x": X0, "y": Y0},
        "layers": [
            layer("altlik", "Altlık"),
            layer("gizli", "Gizli", vis=False),
            layer("kilitli", "Kilitli", lock=True),
            dict(layer("grup", "Grup"), type="group"),
        ],
        "activeLayer": "altlik",
        "entities": list(entities),
        "styles": {"items": [], "categories": []},
    }


def raster_entity(i, rel, w, h, bands, gt, srid=SRID, sample="u8", **extra):
    e = {
        "kind": "raster",
        "layerId": "altlik",
        "attrs": {},
        "affine": gt,
        "width": w,
        "height": h,
        "bands": bands,
        "sample": sample,
        "srid": srid,
        "style": {"render": "gray", "bands": [1]} if bands == 1 else {"render": "rgb", "bands": [1, 2, 3]},
        "id": i,
    }
    if rel is not None:
        e["file"] = fx(rel)
    e.update(extra)
    return e


def result_of(doc, inp, fn):
    try:
        out, warnings, *rest = fn()
    except Refused as r:
        err = {"code": r.code, "message": r.message}
        if r.path:
            err["path"] = r.path
        if r.code == "revision_conflict":
            err["revision"] = "0"
            return {"status": "conflict", "error": err}, False
        return {"status": "failed", "error": err}, False
    res = {"status": "completed", "output": out, "warnings": warnings}
    return res, (rest[0] if rest else False)


def read_step(doc, inp, exists=(), op="execute"):
    res, added = result_of(doc, inp, lambda: read_ref(doc, inp, exists))
    if op != "execute" and res["status"] == "completed":
        res["output"].pop("bytes", None)
    if added and op == "execute":
        res["output"]["uid"] = "$any"
    step = {"op": op, "input": inp, "result": res}
    if res["status"] == "completed" and op == "execute":
        step["expect"] = {"canUndo": bool(added), "revision": "changed" if added else "same"}
    else:
        step["expect"] = {"canUndo": False, "revision": "same"}
    return step


def read_cases():
    doc = setup()
    cases = []

    def case(name, inp, srid=SRID, exists=(), op="execute", steps=None):
        d = setup(srid)
        cases.append({"name": name, "setup": d, "steps": steps or [read_step(d, inp, exists, op)]})

    P = lambda n: fx(n)  # noqa: E731
    case("Dosyadaki karo tablolarını söyler: yazmadan, en ince düzey", {"path": P("orto.gpkg")})
    case("Plan aynı cevabı verir, yazmadan", {"path": P("orto.gpkg"), "output": tmp("plan.tif")}, op="plan")
    case("En ince düzey GeoTIFF olur (600 × 420 piksel, 2 m)", {"path": P("orto.gpkg"), "output": tmp("orto.tif")})
    case("1. düzey 4 m pikselle", {"path": P("orto.gpkg"), "zoom": 1, "output": tmp("orto1.tif")})
    case("0. düzey 8 m pikselle", {"path": P("orto.gpkg"), "zoom": 0, "output": tmp("orto0.tif")})
    case(
        "Alan pikselin tamına yuvarlanır (kesirli sınırlar)",
        {"path": P("orto.gpkg"), "area": [487201.3, 4420020.6, 487700.9, 4420300.2], "output": tmp("kesir.tif")},
    )
    case(
        "Alan ızgaradan taşar: ızgaraya kesilir",
        {"path": P("orto.gpkg"), "area": [487000.0, 4419000.0, 489900.0, 4420800.0], "output": tmp("tasan.tif")},
    )
    case("Alan ızgaranın dışında: tile_table_empty", {"path": P("orto.gpkg"), "area": [500000.0, 4420000.0, 500100.0, 4420100.0]})
    case("Alan geçersiz: en küçük en büyüğe eşit", {"path": P("orto.gpkg"), "area": [487200.0, 4420000.0, 487200.0, 4420100.0]})
    case("Olmayan zoom düzeyi", {"path": P("orto.gpkg"), "zoom": 9})
    case("Veri kutusu pikselin tamında değil: GDAL'ın yuvarlaması", {"path": P("kaymis.gpkg"), "output": tmp("kaymis.tif")})
    case("Kaymış kutuda kaba düzey", {"path": P("kaymis.gpkg"), "zoom": 0, "output": tmp("kaymis0.tif")})
    case("Gri PNG karolar dört banda açılır", {"path": P("gri.gpkg"), "output": tmp("gri.tif")})
    case("Saf gri (tek bantlı) PNG karolar", {"path": P("gri-saf.gpkg"), "output": tmp("gri-saf.tif")})
    case("JPEG karolar okunur", {"path": P("jpeg.gpkg"), "output": tmp("jpeg.tif")})
    case("JPEG karolar yazmadan sorulursa tabloyu söyler", {"path": P("jpeg.gpkg")})
    case("WebP karolar okunur", {"path": P("webp-gercek.gpkg"), "output": tmp("webp.tif")})
    case("Bir tabloda PNG ve JPEG karolar", {"path": P("karisik.gpkg"), "output": tmp("karisik.tif")})
    case("Başlığı bozuk WebP karo", {"path": P("webp.gpkg"), "output": tmp("webp-bozuk.tif")})
    case("Resim olmayan karo", {"path": P("bozuk-karo.gpkg"), "output": tmp("bozuk.tif")})
    case("Matrisin karo boyunda olmayan karo", {"path": P("boy-uyumsuz.gpkg"), "output": tmp("boy.tif")})
    case("16 bitlik PNG karo okunmaz", {"path": P("png16.gpkg"), "output": tmp("png16.tif")})
    case("Vektör GeoPackage'ında karo tablosu yok", {"path": "$fixture:geopackage/v1/qgis.gpkg"})
    case("İki karo tablosu: table gerekir", {"path": P("iki-tablo.gpkg")})
    case("İki karo tablosundan b seçilir", {"path": P("iki-tablo.gpkg"), "table": "b", "output": tmp("b.tif")})
    case("Karo tablosu olmayan ad", {"path": P("iki-tablo.gpkg"), "table": "yok"})
    case("Tablo adı karo tablosu değil", {"path": "$fixture:geopackage/v1/qgis.gpkg", "table": "parsel"})
    case("Eksik karo saydam kalır", {"path": P("eksik.gpkg"), "output": tmp("eksik.tif")})
    case("Sistem bildirmeyen tablo: srid 0", {"path": P("sistemsiz.gpkg"), "output": tmp("yerel.tif")})
    case("Pencerede hiç karo yok", {"path": P("bos.gpkg")})
    case("Okunamayacak kadar büyük matris", {"path": P("genis.gpkg"), "output": tmp("genis.tif")})
    case("Büyük matrisi yazmadan sormak", {"path": P("genis.gpkg")})
    case("Büyük matriste dar alan okunur", {"path": P("genis.gpkg"), "area": [487200.0, 4420472.0, 487328.0, 4420600.0], "output": tmp("dar.tif")})
    case("Yol göreli ama çizimin dosyası yok", {"path": "orto.gpkg"})
    case("Boş yol", {"path": "  "})
    case("Dosya yok", {"path": P("yok.gpkg")})
    case("SQLite değil", {"path": P("metin.txt")})
    case("SQLite ama GeoPackage değil", {"path": P("sqlite.db")})
    case("Çıktı .tif değil", {"path": P("orto.gpkg"), "output": tmp("orto.png")})
    case("Çıktının klasörü yok", {"path": P("orto.gpkg"), "output": tmp("yok/orto.tif")})
    case("Çıktı olmadan layer", {"path": P("orto.gpkg"), "layer": "altlik"})
    case(
        "Göreli çıktı çizimin klasörüne yazılır",
        {"path": P("orto.gpkg"), "output": "goreli.tif", "drawing": tmp("proje.kcad")},
    )
    var = {"path": P("orto.gpkg"), "output": tmp("var.tif")}
    d = setup()
    cases.append(
        {
            "name": "Var olan çıktı overwrite olmadan reddedilir, overwrite ile yazılır",
            "setup": d,
            "steps": [
                read_step(d, var),
                read_step(d, var, exists=(tmp("var.tif"),)),
                read_step(d, dict(var, overwrite=True), exists=(tmp("var.tif"),)),
            ],
        }
    )
    kat = {"path": P("orto.gpkg"), "output": tmp("katman.tif"), "layer": "altlik"}
    d = setup()
    cases.append(
        {
            "name": "Katmana eklenir ve geri alınır",
            "setup": d,
            "steps": [
                read_step(d, kat),
                {"op": "undo", "returns": "GeoPackage karoları", "expect": {"canUndo": False, "canRedo": True}},
            ],
        }
    )
    case("Gizli katman uyarır", {"path": P("orto.gpkg"), "output": tmp("gizli.tif"), "layer": "gizli"})
    case("Kilitli katman", {"path": P("orto.gpkg"), "output": tmp("kilitli.tif"), "layer": "kilitli"})
    case("Grup katman değil", {"path": P("orto.gpkg"), "output": tmp("grup.tif"), "layer": "grup"})
    case("Olmayan katman", {"path": P("orto.gpkg"), "output": tmp("yok.tif"), "layer": "yok"})
    case("Sistem projeninkinden başka", {"path": P("orto.gpkg"), "output": tmp("sistem.tif"), "layer": "altlik"}, srid=5255)
    case("Sistemsiz tablo her projeye eklenir", {"path": P("sistemsiz.gpkg"), "output": tmp("sistemsiz.tif"), "layer": "altlik"}, srid=5255)
    case("Geçersiz beklenen sürüm", {"path": P("orto.gpkg"), "expectedRevision": "x1"})
    case("Eski beklenen sürüm", {"path": P("orto.gpkg"), "expectedRevision": "7"})
    # ADR 0627 §2: gridded coverages.
    case("Kayan noktalı kapsam tek bant f32, nodata'sıyla", {"path": P("kapsam-float.gpkg"), "output": tmp("kapsam-f.tif")})
    case("Kayan noktalı kapsamın kaba düzeyi", {"path": P("kapsam-float.gpkg"), "zoom": 1, "output": tmp("kapsam-f1.tif")})
    case("16 bitlik kapsam u16, boş değeriyle", {"path": P("kapsam-u16.gpkg"), "output": tmp("kapsam-u.tif")})
    case("İşaretli 16 bitlik kapsam i16", {"path": P("kapsam-i16.gpkg"), "output": tmp("kapsam-i.tif")})
    case("Ölçekli tam sayılı kapsam f32", {"path": P("kapsam-olcekli.gpkg"), "output": tmp("kapsam-o.tif")})
    case("GDAL'ın −32768 nodata'lı i16 kapsamı: karoları 1 ötelenmiş", {"path": P("kapsam-i16-eksi.gpkg"), "output": tmp("kapsam-ie.tif")})
    case("Hassasiyeti 0,01 olan ölçekli kapsam yuvarlanmaz", {"path": P("kapsam-hassas.gpkg"), "output": tmp("kapsam-h.tif")})
    case("Ötelemesi −32767, boşu 65535 olan kapsam i16", {"path": P("kapsam-32767.gpkg"), "output": tmp("kapsam-3.tif")})
    case("i16 kapsamın 32767'den küçük boş değeri olduğu gibi", {"path": P("kapsam-kucuk-null.gpkg"), "output": tmp("kapsam-k.tif")})
    case("Saklanan değer olmayan boş değer yok sayılır", {"path": P("kapsam-gecersiz-null.gpkg"), "output": tmp("kapsam-g.tif")})
    case("Kayan noktalı kapsamın ölçeği uygulanmaz", {"path": P("kapsam-float-olcek.gpkg"), "output": tmp("kapsam-fo.tif")})
    case("Kapsamı yazmadan sormak", {"path": P("kapsam-u16.gpkg")})
    case("Tam sayılı kapsamda 8 bitlik karo", {"path": P("kapsam-bozuk.gpkg"), "output": tmp("kapsam-b.tif")})
    case("Kapsamın alanı", {"path": P("kapsam-float.gpkg"), "area": [487250.0, 4419900.0, 487700.0, 4420500.0], "output": tmp("kapsam-alan.tif")})
    # ADR 0627 §3: a raster linked to its table.
    case("Tabloya bağlı eklenir: GeoTIFF yazılmaz", {"path": P("orto.gpkg"), "layer": "altlik", "link": True})
    case("Bağlı kapsam", {"path": P("kapsam-float.gpkg"), "layer": "altlik", "link": True})
    case("Bağlı okuma alanla: pencere kaba düzeylere hizalanır", {"path": P("orto.gpkg"), "area": [487231.0, 4419900.0, 487700.0, 4420543.0], "layer": "altlik", "link": True})
    case("GoogleMapsCompatible ızgarada bağlı pencere hizalanır", {"path": P("merkator.gpkg"), "layer": "altlik", "link": True}, srid=3857)
    case(
        "Bağlı okumada karosu olmayan zoom düzey sayılmaz: pencere ona hizalanmaz",
        {"path": P("jpeg.gpkg"), "area": [487201.0, 4420401.0, 487499.0, 4420599.0], "layer": "altlik", "link": True},
    )
    case("Bağlı okuma çıktı almaz", {"path": P("orto.gpkg"), "layer": "altlik", "link": True, "output": tmp("bagli.tif")})
    case("Bağlı okuma katman ister", {"path": P("orto.gpkg"), "link": True})
    case("Bağlı okumada 16 bitlik resim karosu", {"path": P("png16.gpkg"), "layer": "altlik", "link": True})
    bag = {"path": P("orto.gpkg"), "layer": "altlik", "link": True}
    d = setup()
    cases.append(
        {
            "name": "Bağlı raster geri alınır",
            "setup": d,
            "steps": [
                read_step(d, bag),
                {"op": "undo", "returns": "GeoPackage karoları", "expect": {"canUndo": False, "canRedo": True}},
            ],
        }
    )
    case("Bağlı okumada gizli katman uyarır", {"path": P("orto.gpkg"), "layer": "gizli", "link": True})
    return cases


def write_cases():
    cases = []
    gt15 = [X0, 1.5, 0.0, Y0, 0.0, -1.5]
    ents = [
        raster_entity(1, "kaynak/rgb.tif", 700, 300, 3, gt15),
        raster_entity(2, "kaynak/rgba.tif", 300, 200, 4, [X0, 0.5, 0.0, Y0, 0.0, -0.5]),
        raster_entity(3, "kaynak/gri.tif", 256, 256, 1, [X0, 0.25, 0.0, Y0, 0.0, -0.25]),
        raster_entity(4, "kaynak/gri-alfa.tif", 130, 90, 2, [X0, 2.0, 0.0, Y0, 0.0, -3.0]),
        raster_entity(5, "kaynak/nodata.tif", 120, 100, 1, [X0, 1.0, 0.0, Y0, 0.0, -1.0]),
        raster_entity(6, "kaynak/sistemsiz.tif", 64, 64, 3, [0.0, 1.0, 0.0, 64.0, 0.0, -1.0], srid=0),
        raster_entity(7, "kaynak/cografi.tif", 100, 100, 3, [32.0, 0.001, 0.0, 40.0, 0.0, -0.001], srid=4326),
        raster_entity(8, "kaynak/on-alti-bit.tif", 64, 64, 1, [X0, 1.0, 0.0, Y0, 0.0, -1.0], sample="u16"),
        raster_entity(9, "kaynak/paletli.tif", 64, 64, 1, [X0, 1.0, 0.0, Y0, 0.0, -1.0]),
        raster_entity(10, "kaynak/rgb.tif", 700, 300, 3, [X0, 1.5, 0.3, Y0, 0.2, -1.5]),
        raster_entity(11, None, 6000, 5000, 3, [X0, 1.0, 0.0, Y0, 0.0, -1.0], file=fx("kaynak/yok.tif")),
        raster_entity(12, None, 100, 100, 3, [X0, 1.0, 0.0, Y0, 0.0, -1.0], url="https://ornek.org/orto.tif"),
        raster_entity(13, "kaynak/yok.tif", 64, 64, 3, [X0, 1.0, 0.0, Y0, 0.0, -1.0]),
        raster_entity(14, None, 64, 64, 3, [X0, 1.0, 0.0, Y0, 0.0, -1.0], asset="kitaplikta-yok"),
        raster_entity(15, "kaynak/rgb.tif", 650, 300, 3, gt15),
        raster_entity(16, "kaynak/bos-bir.tif", 5, 3, 3, [X0, 10.0, 0.0, Y0, 0.0, -10.0]),
        raster_entity(17, "kaynak/rgb.tif", 700, 300, 4, gt15),
        {"kind": "point", "id": 18, "layerId": "altlik", "attrs": {}, "p": {"x": 487001, "y": 4420001}},
        # ADR 0627 §2: heights.
        raster_entity(19, "kaynak/dem-u16.tif", 300, 280, 1, [X0, 1.0, 0.0, Y0, 0.0, -1.0], sample="u16"),
        raster_entity(20, "kaynak/dem-i16.tif", 300, 280, 1, [X0, 1.0, 0.0, Y0, 0.0, -1.0], sample="i16"),
        raster_entity(21, "kaynak/dem-i16-nodata.tif", 200, 150, 1, [X0, 1.0, 0.0, Y0, 0.0, -1.0], sample="i16"),
        raster_entity(22, "kaynak/dem-f32.tif", 520, 300, 1, [X0, 1.0, 0.0, Y0, 0.0, -1.0], sample="f32"),
        raster_entity(23, "kaynak/dem-f64.tif", 100, 80, 1, [X0, 1.0, 0.0, Y0, 0.0, -1.0], sample="f64"),
        raster_entity(24, "kaynak/dem-i32.tif", 100, 80, 1, [X0, 1.0, 0.0, Y0, 0.0, -1.0], sample="i32"),
        raster_entity(25, None, 300_000, 50, 1, [X0, 1.0, 0.0, Y0, 0.0, -1.0], sample="f32", file=fx("kaynak/yok.tif")),
        raster_entity(27, None, 64, 64, 1, [X0, 1.0, 0.0, Y0, 0.0, -1.0], sample="i8", file=fx("kaynak/yok.tif")),
        raster_entity(28, None, 64, 64, 3, [X0, 1.0, 0.0, Y0, 0.0, -1.0], sample="u16", file=fx("kaynak/yok.tif")),
    ]
    # A persistent id the harness hands out in the order of the objects (`$uidOf:slot`).
    global UIDS
    UIDS = {f"$uidOf:{e['id']}": e for e in ents}

    def uid(i):
        return f"$uidOf:{i}"

    def doc_with():
        return setup(entities=ents)

    def ref(inp, copied=None, exists=(), executed=True):
        def fn():
            return write_ref(None, _real_uid(inp), copied or {}, exists, executed)

        return result_of(None, inp, fn)

    def step(inp, op="execute", copied=None, exists=()):
        res, _ = ref(inp, copied, exists, op == "execute")
        if op == "plan" and res["status"] == "completed":
            res["output"]["bytes"] = 0
        s = {"op": op, "input": inp, "result": res, "expect": {"canUndo": False, "revision": "same"}}
        return s

    def case(name, steps):
        cases.append({"name": name, "setup": doc_with(), "steps": steps})

    for i, label in (
        (1, "RGB, 700 × 300: üç düzey, alfa içeride 255"),
        (2, "RGBA, 300 × 200: iki düzey"),
        (3, "Gri 256 × 256: tek karo"),
        (4, "Gri ve alfa 130 × 90, kare olmayan piksel"),
        (6, "Sistemsiz raster: tanımsız kartezyen"),
        (7, "Coğrafi raster (EPSG:4326)"),
        (16, "Üç piksellik raster: tek karo"),
    ):
        case(label, [step({"uid": uid(i), "path": tmp(f"karo{i}.gpkg"), "table": f"karo{i}"})])
    case("nodata uyarır", [step({"uid": uid(5), "path": tmp("nodata.gpkg")})])
    # ADR 0627 §1, §2: lossless WebP pictures and gridded coverages.
    case("RGB WebP karolar", [step({"uid": uid(1), "path": tmp("webp1.gpkg"), "table": "webp1", "format": "webp"})])
    case("RGBA WebP karolar", [step({"uid": uid(2), "path": tmp("webp2.gpkg"), "table": "webp2", "format": "webp"})])
    case("Gri WebP karolar", [step({"uid": uid(3), "path": tmp("webp3.gpkg"), "table": "webp3", "format": "webp"})])
    case("PNG açıkça istenir", [step({"uid": uid(3), "path": tmp("png3.gpkg"), "table": "png3", "format": "png"})])
    case("16 bitlik yükseklik: tam sayılı kapsam, boş değeriyle", [step({"uid": uid(19), "path": tmp("dem-u16.gpkg"), "table": "dem"})])
    case("İşaretli 16 bitlik yükseklik: −32768 ötelemeli tam sayılar", [step({"uid": uid(20), "path": tmp("dem-i16.gpkg"), "table": "dem"})])
    case("İşaretli yükseklik nodata'sıyla kayan noktalı yazılır", [step({"uid": uid(21), "path": tmp("dem-i16n.gpkg"), "table": "dem"})])
    case("32 bitlik kayan noktalı yükseklik, nodata'sıyla", [step({"uid": uid(22), "path": tmp("dem-f32.gpkg"), "table": "dem"})])
    case("64 bitlik yükseklik yuvarlanır", [step({"uid": uid(23), "path": tmp("dem-f64.gpkg"), "table": "dem"})])
    case("32 bitlik tam sayılar yuvarlanır", [step({"uid": uid(24), "path": tmp("dem-i32.gpkg"), "table": "dem"})])
    case("Kapsamın planı yuvarlamayı söylemez", [step({"uid": uid(23), "path": tmp("dem-plan.gpkg"), "table": "dem"}, op="plan")])
    case(
        "Kapsam var olan GeoPackage'a eklenir",
        [
            {"op": "copyFile", "from": fx("orto.gpkg"), "to": tmp("dem-ekle.gpkg")},
            step({"uid": uid(22), "path": tmp("dem-ekle.gpkg"), "table": "dem", "mode": "add"}, copied={tmp("dem-ekle.gpkg"): "orto.gpkg"}),
        ],
    )
    case("Plan yazmadan söyler", [step({"uid": uid(1), "path": tmp("plan.gpkg"), "table": "orto"}, op="plan")])
    case("Ad ve tanımlayıcı verilir", [step({"uid": uid(2), "path": tmp("kimlik.gpkg"), "table": "ortofoto", "identifier": "Ortofoto 2026"})])
    case(
        "Var olan dosya overwrite olmadan reddedilir, overwrite ile yazılır",
        [
            step({"uid": uid(3), "path": tmp("uzerine.gpkg")}),
            step({"uid": uid(3), "path": tmp("uzerine.gpkg")}, exists=(tmp("uzerine.gpkg"),)),
            step({"uid": uid(3), "path": tmp("uzerine.gpkg"), "overwrite": True}, exists=(tmp("uzerine.gpkg"),)),
        ],
    )
    case(
        "Göreli yol çizimin klasörüne yazılır",
        [step({"uid": uid(3), "path": "goreli.gpkg", "drawing": tmp("proje.kcad")})],
    )
    # Mode add: a copy of a GeoPackage that has a table already.
    copy = {"op": "copyFile", "from": fx("orto.gpkg"), "to": tmp("ekle.gpkg")}
    case(
        "Var olan GeoPackage'a tablo eklenir",
        [
            copy,
            step({"uid": uid(3), "path": tmp("ekle.gpkg"), "table": "yeni", "mode": "add"}, copied={tmp("ekle.gpkg"): "orto.gpkg"}),
        ],
    )
    case(
        "Aynı adlı tablo eklenmez",
        [
            copy,
            step({"uid": uid(3), "path": tmp("ekle.gpkg"), "table": "orto", "mode": "add"}, copied={tmp("ekle.gpkg"): "orto.gpkg"}),
        ],
    )
    case("mode add: dosya yok", [step({"uid": uid(3), "path": tmp("yok.gpkg"), "mode": "add"})])
    case(
        "mode add: SQLite değil",
        [
            {"op": "copyFile", "from": fx("metin.txt"), "to": tmp("metin.gpkg")},
            step({"uid": uid(3), "path": tmp("metin.gpkg"), "mode": "add"}, copied={tmp("metin.gpkg"): "metin.txt"}),
        ],
    )
    # Refusals.
    for label, inp in (
        ("Geçersiz kimlik", {"uid": "abc", "path": tmp("x.gpkg")}),
        ("Olmayan nesne", {"uid": "00000000-0000-4000-8000-000000000000", "path": tmp("x.gpkg")}),
        ("Raster olmayan nesne", {"uid": uid(18), "path": tmp("x.gpkg")}),
        ("Yol .gpkg değil", {"uid": uid(1), "path": tmp("x.tif")}),
        ("Yol göreli ama çizimin dosyası yok", {"uid": uid(1), "path": "x.gpkg"}),
        ("Klasör yok", {"uid": uid(1), "path": tmp("yok/x.gpkg")}),
        ("Tablo adı gpkg_ ile başlıyor", {"uid": uid(1), "path": tmp("x.gpkg"), "table": "gpkg_orto"}),
        ("Tablo adı rakamla başlıyor", {"uid": uid(1), "path": tmp("x.gpkg"), "table": "2orto"}),
        ("Paletli raster", {"uid": uid(9), "path": tmp("x.gpkg")}),
        ("16 bitlik raster yükseklik kapsamı olur", {"uid": uid(8), "path": tmp("on-alti.gpkg")}),
        ("Adresten okunan raster", {"uid": uid(12), "path": tmp("x.gpkg")}),
        ("Dönük raster", {"uid": uid(10), "path": tmp("x.gpkg")}),
        ("Eski 25 milyon sınırı kalktı: dosya aranır", {"uid": uid(11), "path": tmp("x.gpkg")}),
        ("Şeridi 256 MB'ı aşan raster", {"uid": uid(25), "path": tmp("x.gpkg")}),
        ("Çok bantlı 16 bitlik raster", {"uid": uid(28), "path": tmp("x.gpkg")}),
        ("8 bitlik işaretli raster", {"uid": uid(27), "path": tmp("x.gpkg")}),
        ("Kapsam WebP yazılmaz", {"uid": uid(19), "path": tmp("x.gpkg"), "format": "webp"}),
        ("Kaynak dosya yok", {"uid": uid(13), "path": tmp("x.gpkg")}),
        ("Kitaplıkta olmayan gömülü kaynak", {"uid": uid(14), "path": tmp("x.gpkg")}),
        ("Kaynağın boyu kayıttakinden başka", {"uid": uid(15), "path": tmp("x.gpkg")}),
        ("Kaynağın bant sayısı kayıttakinden başka", {"uid": uid(17), "path": tmp("x.gpkg")}),
    ):
        case(label, [step(inp)])
    return cases


def _real_uid(inp):
    return inp


# ── The files ──────────────────────────────────────────────────────────────────────────────────────────────────


def doc_of(cases, command, title, note):
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": command,
        "commandVersion": 1,
        "title": title,
        "note": note,
        "setup": cases[0]["setup"],
        "cases": cases,
    }


def build():
    read = doc_of(
        read_cases(),
        "cad.rasters.gpkgread",
        "GeoPackage karolarını oku",
        "Written by scripts/fixtures/gpkg_tiles_cases.py from docs/adr/0556 with GDAL; do not edit by hand.",
    )
    write = doc_of(
        write_cases(),
        "cad.rasters.gpkgwrite",
        "Rasteri GeoPackage karosu olarak yaz",
        "Written by scripts/fixtures/gpkg_tiles_cases.py from docs/adr/0556 with GDAL; do not edit by hand.",
    )
    return read, write


def dump(doc):
    return json.dumps(doc, indent=1, ensure_ascii=True) + "\n"


# ── The checks of the files KentOS wrote ───────────────────────────────────────────────────────────────────────


# JPEG tiles: KentOS decodes them with zune-jpeg, GDAL with libjpeg-turbo; their inverse DCTs and chroma upsampling
# differ by at most 4 levels on these tiles (measured 24 October 2026 with GDAL 3.12.2: 3 and 4 at most, means 0.32
# and 0.10). WebP's lossy and lossless tiles decode alike bit for bit (image-webp and libwebp), PNG and TIFF tiles too.
JPEG_MOST = 4


def gdal_zoom(path, zoom, band_count=None, table=None):
    """GDAL's dataset of a zoom's data box and its samples, bands first (pictures as `band_count` bands, a coverage as
    it is)."""
    opts = [f"ZOOM_LEVEL={zoom}"]
    if band_count:
        opts.append(f"BAND_COUNT={band_count}")
    if table:
        opts.append(f"TABLE={table}")
    ds = gdal.OpenEx(str(path), open_options=opts)
    arr = ds.ReadAsArray()
    return ds, arr[None] if arr.ndim == 2 else arr


def tiff_of(path):
    ds = gdal.Open(str(path))
    arr = ds.ReadAsArray()
    return ds, arr[None] if arr.ndim == 2 else arr


def unlike(want, got, most=0):
    """Why two arrays of samples differ (none when alike): their shapes, types, or samples further apart than `most`."""
    if want.shape != got.shape:
        return f"GDAL {want.shape}, KentOS {got.shape}"
    if want.dtype != got.dtype:
        return f"GDAL {want.dtype}, KentOS {got.dtype}"
    if most:
        d = int(np.abs(want.astype(np.int64) - got.astype(np.int64)).max(initial=0))
        return None if d <= most else f"en büyük fark {d}, en çok {most}"
    same = want == got
    if want.dtype.kind == "f":
        same |= np.isnan(want) & np.isnan(got)
    n = int((~same).sum())
    return None if n == 0 else f"{n} örnek farklı"


def most_of(table, zoom):
    """How far a zoom's samples may be from GDAL's: JPEG's decoders apart, none for the rest."""
    m = next(z for z in table["zooms"] if z["zoom"] == zoom)
    return JPEG_MOST if "jpeg" in m["formats"] else 0


def data_pixel(table, m):
    """GDAL's first pixel of a zoom's data box, in the zoom's matrix pixels."""
    tms = table["bounds"]
    data = table.get("dataBounds") or tms
    px, py = m["pixel"]
    return math.floor((data[0] - tms[0]) / px + 0.5), math.floor((tms[3] - data[3]) / py + 0.5)


def beside_gdal(name, full, got, ox, oy, most):
    """KentOS's window at GDAL's (ox, oy): the samples both hold alike."""
    h, w = got.shape[1], got.shape[2]
    ix0, iy0 = max(ox, 0), max(oy, 0)
    ix1, iy1 = min(ox + w, full.shape[2]), min(oy + h, full.shape[1])
    if ix1 <= ix0 or iy1 <= iy0:
        return [f"{name}: GDAL'ın okuduğu yerle kesişmiyor"]
    why = unlike(full[:, iy0:iy1, ix0:ix1], got[:, iy0 - oy : iy1 - oy, ix0 - ox : ix1 - ox], most)
    return [f"{name}: {why}"] if why else []


def worked_out(finer, nodata):
    """The reader's coarser level worked out from the finer one (docs/adr/0204 §2): each sample the mean of its four
    children in the finer level that are not nodata or NaN, in the children's order; integers rounded half away from
    zero, floats as 32-bit floats; nodata (else NaN for floats, else 0) where none is."""
    b, h, w = finer.shape
    hh, ww = (h + 1) // 2, (w + 1) // 2
    total = np.zeros((b, hh, ww), np.float64)
    n = np.zeros((b, hh, ww), np.int64)
    for di, dj in ((0, 0), (1, 0), (0, 1), (1, 1)):
        part = finer[:, dj::2, di::2].astype(np.float64)
        ph, pw = part.shape[1], part.shape[2]
        ok = ~np.isnan(part)
        if nodata is not None:
            ok &= part != nodata
        total[:, :ph, :pw] += np.where(ok, part, 0.0)
        n[:, :ph, :pw] += ok
    mean = total / np.maximum(n, 1)
    if finer.dtype.kind == "f":
        out = mean.astype(np.float32)
        fill = np.float32(np.nan if nodata is None else nodata)
    else:
        out = np.where(mean >= 0, np.floor(mean + 0.5), -np.floor(-mean + 0.5))
        fill = 0 if nodata is None else nodata
    return np.where(n > 0, out, fill).astype(finer.dtype)


def halve_coverage(finer, lw, lh, integer, null):
    """The writer's coarser coverage image (docs/adr/0627 §5) from the finer one (stored values for an integer
    coverage): each the mean of its children in the finer image and not null, in their order; an integer one's
    rounded half up ((2·sum + n) / 2n), a float one's as a 32-bit float; the null (else 0; NaN for a float coverage
    without a null whose children are NaN) where none is."""
    total = np.zeros((lh, lw), np.int64 if integer else np.float64)
    n = np.zeros((lh, lw), np.int64)
    nan = np.zeros((lh, lw), bool)
    for di, dj in ((0, 0), (1, 0), (0, 1), (1, 1)):
        part = finer[dj::2, di::2]
        ph, pw = part.shape
        if integer:
            ok = np.ones(part.shape, bool) if null is None else part != int(null)
            total[:ph, :pw] += np.where(ok, part, 0)
        else:
            isn = np.isnan(part)
            nan[:ph, :pw] |= isn
            ok = ~isn if null is None else ~isn & (part != np.float32(null))
            total[:ph, :pw] += np.where(ok, part.astype(np.float64), 0.0)
        n[:ph, :pw] += ok
    fill = 0 if null is None else null
    if integer:
        mean = (2 * total + n) // np.maximum(2 * n, 1)
        return np.where(n > 0, mean, int(fill)).astype(np.int64)
    mean = (total / np.maximum(n, 1)).astype(np.float32)
    empty = np.where(nan & (null is None), np.float32(np.nan), np.float32(fill))
    return np.where(n > 0, mean, empty).astype(np.float32)


def tile_figures(values):
    """A coverage tile's least, greatest, mean and population standard deviation of its values; none without values."""
    if values.size == 0:
        return None, None, None, None
    mean = math.fsum(values) / values.size
    var = math.fsum((values - mean) ** 2) / values.size
    return float(values.min()), float(values.max()), mean, math.sqrt(var)


def close(a, b):
    return a is None and b is None or a is not None and b is not None and abs(a - b) <= 1e-9 * max(1.0, abs(b))


def check_coverage(r, path):
    """A coverage KentOS wrote: its rows (coverage, extensions, 3D system), every zoom against the rule from the
    source's values, every tile's statistics."""
    bad = []
    table, cov = r["table"], r["coverage"]
    integer, offset, null = cov["datatype"] == "integer", cov["offset"], cov.get("dataNull")
    src = gdal.Open(str(DIR / r["source"]))
    arr = src.GetRasterBand(1).ReadAsArray()
    w, h = src.RasterXSize, src.RasterYSize
    # The finest image: the source's values (a float coverage's as 32-bit floats; its nodata is then its null).
    finest = arr.astype(np.int64) - int(offset) if integer else arr.astype(np.float64).astype(np.float32)
    sizes = levels_of(w, h)
    images = [finest]
    for lw, lh, _, _ in sizes[1:]:
        images.append(halve_coverage(images[-1], lw, lh, integer, null))
    top = len(sizes) - 1
    kind = (np.uint16 if offset == 0 else np.int16) if integer else np.float32
    for j, img in enumerate(images):
        zoom = top - j
        _, got = gdal_zoom(path, zoom, None, table)
        want = (img + int(offset)).astype(kind) if integer else img
        why = unlike(want[None], got)
        if why:
            bad.append(f"{r['file']} zoom {zoom}: {why}")
    c = sqlite3.connect(path)
    row = c.execute(
        "select datatype, scale, offset, data_null, grid_cell_encoding from gpkg_2d_gridded_coverage_ancillary "
        "where tile_matrix_set_name=?",
        (table,),
    ).fetchone()
    if row != (cov["datatype"], 1.0, float(offset), null, "grid-value-is-center"):
        bad.append(f"{r['file']}: kapsamın satırı {row}")
    ext = c.execute("select table_name, column_name, extension_name from gpkg_extensions order by extension_name, table_name").fetchall()
    want_ext = [
        ("gpkg_2d_gridded_coverage_ancillary", None, "gpkg_2d_gridded_coverage"),
        ("gpkg_2d_gridded_tile_ancillary", None, "gpkg_2d_gridded_coverage"),
        (table, "tile_data", "gpkg_2d_gridded_coverage"),
    ]
    if sorted(ext, key=str) != sorted(want_ext, key=str):
        bad.append(f"{r['file']}: eklentiler {ext}")
    if c.execute("select count(*) from gpkg_spatial_ref_sys where srs_id=4979").fetchone()[0] != 1:
        bad.append(f"{r['file']}: 4979 sistemi yok")
    if c.execute("select data_type from gpkg_contents where table_name=?", (table,)).fetchone()[0] != "2d-gridded-coverage":
        bad.append(f"{r['file']}: tablo 2d-gridded-coverage değil")
    stats = c.execute(
        f'select t.zoom_level, t.tile_column, t.tile_row, a.scale, a.offset, a.min, a.max, a.mean, a.std_dev '
        f'from "{table}" t left join gpkg_2d_gridded_tile_ancillary a on a.tpudt_name=? and a.tpudt_id=t.id',
        (table,),
    ).fetchall()
    c.close()
    for zoom, col, rw, scale, off, mn, mx, mean, sd in stats:
        if (scale, off) != (1.0, 0.0):
            bad.append(f"{r['file']} {zoom}/{col}/{rw}: karonun ölçeği ve ötelemesi {scale}, {off}")
            continue
        img = images[top - zoom]
        cut = img[rw * SIDE : (rw + 1) * SIDE, col * SIDE : (col + 1) * SIDE].ravel()
        if integer:
            vals = cut[cut != int(null)] if null is not None else cut
            vals = (vals + int(offset)).astype(np.float64)
        else:
            vals = cut[~np.isnan(cut)]
            if null is not None:
                vals = vals[vals != np.float32(null)]
            vals = vals.astype(np.float64)
        want = tile_figures(vals)
        if not all(close(g, e) for g, e in zip((mn, mx, mean, sd), want)):
            bad.append(f"{r['file']} {zoom}/{col}/{rw}: istatistik {(mn, mx, mean, sd)}, beklenen {want}")
    return bad


def check_picture(r, path):
    """A picture KentOS wrote: GDAL's pixels of every zoom against the rule from the source's (lossless tiles)."""
    bad = []
    src = gdal.Open(str(DIR / r["source"]))
    sb = r["bands"]
    arr = src.ReadAsArray()
    if arr.ndim == 2:
        arr = arr[None]
    w, h = src.RasterXSize, src.RasterYSize
    if sb == 4 and arr.shape[0] == 3:
        arr = np.concatenate([arr, np.full((1, h, w), 255, np.uint8)])
    lv = levels_of(w, h)
    top = len(lv) - 1
    canvas = np.zeros((arr.shape[0], lv[0][3] * SIDE, lv[0][2] * SIDE), np.uint8)
    canvas[:, :h, :w] = arr
    for j, (lw, lh, tx, ty) in enumerate(lv):
        zoom = top - j
        if j > 0:
            px_, py_ = canvas.shape[2], canvas.shape[1]
            pad = np.zeros((canvas.shape[0], 2 * ty * SIDE, 2 * tx * SIDE), np.uint16)
            hh, ww = min(py_, pad.shape[1]), min(px_, pad.shape[2])
            pad[:, :hh, :ww] = canvas[:, :hh, :ww]
            s = pad[:, 0::2, 0::2] + pad[:, 1::2, 0::2] + pad[:, 0::2, 1::2] + pad[:, 1::2, 1::2]
            canvas = ((s + 2) // 4).astype(np.uint8)
        _, got = gdal_zoom(path, zoom, sb, r["table"])
        why = unlike(canvas[:, :lh, :lw], got)
        if why:
            bad.append(f"{r['file']} zoom {zoom}: {why}")
    c = sqlite3.connect(path)
    ext = c.execute("select table_name, column_name, extension_name from gpkg_extensions").fetchall() if c.execute(
        "select count(*) from sqlite_master where name='gpkg_extensions'"
    ).fetchone()[0] else []
    c.close()
    webp = (r["table"], "tile_data", "gpkg_webp") in ext
    if webp != (r["format"] == "webp"):
        bad.append(f"{r['file']}: gpkg_webp eklentisi {'var' if webp else 'yok'}, karolar {r['format']}")
    return bad


QGIS_MAC = Path("/Applications/QGIS-final-4_2_2.app/Contents")
QGIS_LINUX = Path("/usr/share/qgis/python")


def qgis_python():
    """QGIS 4.2's Python, its prefix and environment: the macOS application's or Linux's packages; none without QGIS."""
    base = dict(os.environ, QT_QPA_PLATFORM="offscreen", GDAL_PAM_ENABLED="NO")
    if (QGIS_MAC / "MacOS/python3.12").exists():
        return (
            str(QGIS_MAC / "MacOS/python3.12"),
            f"{QGIS_MAC}/MacOS",
            dict(
                base,
                PYTHONHOME=str(QGIS_MAC / "Frameworks"),
                PYTHONPATH=f"{QGIS_MAC}/Resources/qgis/python:{QGIS_MAC}/Frameworks/lib/python3.12/site-packages",
                PROJ_DATA=str(QGIS_MAC / "Resources/qgis/proj"),
            ),
        )
    if QGIS_LINUX.is_dir() and Path("/usr/bin/python3").exists():
        return "/usr/bin/python3", "/usr", dict(base, PYTHONPATH=str(QGIS_LINUX))
    return None


def qgis_opens(items):
    """What QGIS 4.2 reads of the GeoPackages KentOS wrote: a valid raster of the finest zoom's size and EPSG, a
    coverage as one band of its type; none when QGIS is not installed."""
    found = qgis_python()
    if found is None:
        print("QGIS 4.2 yok; QGIS denetimi atlandı")
        return []
    py, prefix, env = found
    code = f"""
import json
from qgis.core import Qgis, QgsApplication, QgsRasterLayer
QgsApplication.setPrefixPath({prefix!r}, True)
app = QgsApplication([], False); app.initQgis()
out = {{"version": Qgis.version()}}
for path, table in {[(p, t) for p, t, _ in items]!r}:
    l = QgsRasterLayer('GPKG:' + path + ':' + table, table, 'gdal')
    dt = l.dataProvider().dataType(1) if l.isValid() else None
    out[path] = [l.isValid(), l.width(), l.height(), l.crs().authid(), l.bandCount(), getattr(dt, 'name', str(dt))]
print(json.dumps(out))
"""
    r = subprocess.run([py, "-c", code], env=env, capture_output=True, text=True, timeout=300)
    line = [x for x in r.stdout.splitlines() if x.startswith("{")]
    if not line:
        return [f"QGIS okuyamadı: {r.stderr[-300:]}"]
    got = json.loads(line[-1])
    bad = []
    for (path, w, h, srid), (_, _, coverage) in zip(items_meta([(p, t) for p, t, _ in items]), items):
        valid, qw, qh, auth, bands, dt = got[path]
        if not valid or (qw, qh) != (w, h) or (srid and auth != f"EPSG:{srid}"):
            bad.append(f"QGIS {Path(path).name}: geçerli={valid} boy={qw}x{qh} sistem={auth}; beklenen {w}x{h} EPSG:{srid}")
        if coverage and (bands != 1 or coverage not in dt):
            bad.append(f"QGIS {Path(path).name}: {bands} bant, {dt}; beklenen bir bant, {coverage}")
    print(f"{len(items)} GeoPackage QGIS {got['version']}'de açıldı")
    return bad


def items_meta(items):
    out = []
    for path, table in items:
        c = sqlite3.connect(path)
        z, mw, mh, tw, th, px, py = c.execute(
            "select zoom_level, matrix_width, matrix_height, tile_width, tile_height, pixel_x_size, pixel_y_size "
            "from gpkg_tile_matrix where table_name=? order by zoom_level desc limit 1",
            (table,),
        ).fetchone()
        x0, y0, x1, y1, srs = c.execute(
            "select min_x, min_y, max_x, max_y, srs_id from gpkg_contents where table_name=?", (table,)
        ).fetchone()
        c.close()
        out.append((path, math.floor((x1 - x0) / px + 0.5), math.floor((y1 - y0) / py + 0.5), srs if srs > 0 else 0))
    return out


def check_reads(read_report):
    """Rasters read from tiles: GDAL's own samples of the same zoom and window, its type and nodata."""
    failures = []
    for r in read_report:
        src = DIR / r["source"]
        name = r["output"]
        zoom = r["zoom"]
        table = tile_tables(str(src))[0]
        m = next(z for z in table["zooms"] if z["zoom"] == zoom)
        px, py = m["pixel"]
        tms = table["bounds"]
        coverage = table.get("coverage") is not None
        gds, full = gdal_zoom(src, zoom, None if coverage else 4)
        gt, size = gds.GetGeoTransform(), (gds.RasterXSize, gds.RasterYSize)
        ds, got = tiff_of(OUT_DIR / name)
        g = ds.GetGeoTransform()
        gx, gy = data_pixel(table, m)
        ox, oy = round((g[0] - tms[0]) / px) - gx, round((tms[3] - g[3]) / py) - gy
        failures += beside_gdal(name, full, got, ox, oy, most_of(table, zoom))
        h, w = got.shape[1], got.shape[2]
        if abs(g[1] - px) > 1e-12 or abs(g[5] + py) > 1e-12:
            failures.append(f"{name}: piksel {g[1]}, {g[5]}; tablonun pikseli {px}, {py}")
        if abs(g[0] - r["origin"][0]) > 1e-9 or abs(g[3] - r["origin"][1]) > 1e-9:
            failures.append(f"{name}: yer {g[0]}, {g[3]}; komutun söylediği {r['origin']}")
        if r.get("defaultWindow"):
            # The data box's window: GDAL's size, and its place within half a pixel (KentOS keeps the pixels' own grid).
            if size != (w, h):
                failures.append(f"{name}: GDAL {size} okuyor, KentOS {(w, h)}")
            if abs(gt[0] - g[0]) > px / 2 + 1e-9 or abs(gt[3] - g[3]) > py / 2 + 1e-9:
                failures.append(f"{name}: yer GDAL'ınkinden yarım pikselden fazla ayrı")
        if r.get("srid"):
            srs = ds.GetSpatialRef()
            if srs is None or srs.GetAuthorityCode(None) != str(r["srid"]):
                failures.append(f"{name}: sistem EPSG:{r['srid']} değil")
        if ds.RasterCount != r["bands"] or ds.RasterCount != gds.RasterCount:
            failures.append(f"{name}: {ds.RasterCount} bant, GDAL {gds.RasterCount}, komut {r['bands']}")
        ours, theirs = ds.GetRasterBand(1).GetNoDataValue(), gds.GetRasterBand(1).GetNoDataValue()
        if coverage and (ours != theirs or ours != r.get("nodata")):
            failures.append(f"{name}: nodata {ours}, GDAL {theirs}, komut {r.get('nodata')}")
    return failures


def check_links(link_report):
    """The levels a raster linked to its table reads: a zoom's like GDAL's samples of that zoom at the level's place,
    a level worked out by the reader's rule from the finer one KentOS read."""
    failures = []
    finer = {}
    for r in link_report:
        src = DIR / r["source"]
        name = r["output"]
        ds, got = tiff_of(OUT_DIR / name)
        nodata = ds.GetRasterBand(1).GetNoDataValue()
        if r.get("computed"):
            prev = finer.get((r["source"], r["level"] - 1))
            if prev is None:
                failures.append(f"{name}: {r['level'] - 1}. düzey yok")
            else:
                why = unlike(worked_out(prev, nodata), got)
                if why:
                    failures.append(f"{name}: okuyucunun kuralından {why}")
        else:
            table = next(t for t in tile_tables(str(src)) if t["table"] == r["table"])
            zoom = r["zoom"]
            m = next(z for z in table["zooms"] if z["zoom"] == zoom)
            coverage = table.get("coverage") is not None
            gds, full = gdal_zoom(src, zoom, None if coverage else 4, r["table"])
            gx, gy = data_pixel(table, m)
            failures += beside_gdal(name, full, got, r["x"] - gx, r["y"] - gy, most_of(table, zoom))
            if coverage and nodata != gds.GetRasterBand(1).GetNoDataValue():
                failures.append(f"{name}: nodata {nodata}, GDAL {gds.GetRasterBand(1).GetNoDataValue()}")
        finer[(r["source"], r["level"])] = got
    return failures


def qgis_type(r):
    """The band type QGIS reads of a coverage KentOS wrote (GDAL's convention): none for pictures."""
    cov = r.get("coverage")
    if not cov:
        return None
    if cov["datatype"] == "float":
        return "Float32"
    return "UInt16" if cov["offset"] == 0 else "Int16"


def check_files():
    failures = []
    if not OUT_DIR.exists() or not any(OUT_DIR.iterdir()):
        print("KentOS'un yazdığı dosyalar yok (KENTOS_WRITE_GPKGTILES=1 cargo test -p kentos-native-application --test all gpkg_tiles_files); dosya denetimi atlandı")
        return failures
    report = {}
    for part in ("read", "link", "write"):
        f = OUT_DIR / f"{part}.json"
        report[part] = json.loads(f.read_text()) if f.exists() else []
    gdal.UseExceptions()
    failures += check_reads(report["read"])
    failures += check_links(report["link"])
    # GeoPackages written from rasters: validate_gpkg.py, GDAL's samples of every zoom, the rule's coarser zooms.
    from osgeo_utils.samples import validate_gpkg

    for r in report["write"]:
        path = OUT_DIR / r["file"]
        errs = validate_gpkg.check(str(path), abort_at_first_error=False) or []
        if errs:
            failures.append(f"{r['file']}: validate_gpkg: {errs[:3]}")
        failures += check_coverage(r, path) if r.get("coverage") else check_picture(r, path)
        gt = gdal.OpenEx(str(path), open_options=[f"TABLE={r['table']}"]).GetGeoTransform()
        e = r["affine"]
        if any(abs(gt[k] - e[k]) > 1e-9 for k in range(6)):
            failures.append(f"{r['file']}: GDAL'ın yeri {gt}, rasterin yeri {e}")
        c = sqlite3.connect(path)
        n = c.execute(f'select count(*) from "{r["table"]}"').fetchone()[0]
        c.close()
        if n != r["tiles"]:
            failures.append(f"{r['file']}: {n} karo, {r['tiles']} bekleniyordu")
    failures += qgis_opens([(str(OUT_DIR / r["file"]), r["table"], qgis_type(r)) for r in report["write"]])
    print(
        f"{len(report['read'])} okunan raster, {len(report['link'])} bağlı düzey ve {len(report['write'])} yazılan "
        "GeoPackage GDAL'la denetlendi"
    )
    return failures



def main():
    args = sys.argv[1:]
    if "--files" in args:
        files()
        print(f"{DIR} yazıldı")
        return 0
    read, write = build()
    if "--check" in args:
        bad = []
        for path, doc in ((READ, read), (WRITE, write)):
            if not path.exists() or path.read_text() != dump(doc):
                bad.append(f"{path.relative_to(ROOT)} güncel değil")
        bad += check_files()
        if bad:
            print("\n".join(bad))
            return 1
        print("cad.rasters.gpkgread ve cad.rasters.gpkgwrite durumları güncel")
        return 0
    READ.write_text(dump(read))
    WRITE.write_text(dump(write))
    print(f"{READ.relative_to(ROOT)}: {len(read['cases'])} durum, {WRITE.relative_to(ROOT)}: {len(write['cases'])} durum")
    return 0


if __name__ == "__main__":
    sys.exit(main())
