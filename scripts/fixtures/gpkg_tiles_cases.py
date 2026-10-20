#!/usr/bin/env python3
"""GeoPackage tile pyramids as rasters (docs/adr/0556; plan/gis.md CBS-03's raster profile): the shared cases of
`cad.rasters.gpkgread` and `cad.rasters.gpkgwrite` and their files, from GDAL's own GeoPackage driver, without KentOS
code.

    python3 scripts/fixtures/gpkg_tiles_cases.py --files   # (re)write the sources of fixtures/gpkg-tiles/v1 (GDAL, sqlite3)
    python3 scripts/fixtures/gpkg_tiles_cases.py           # write fixtures/commands/v1/cad.rasters.gpkg{read,write}.json
    python3 scripts/fixtures/gpkg_tiles_cases.py --check   # writes nothing; compares the cases and the files KentOS wrote

The sources: GeoPackages GDAL wrote (RGBA and grey PNG tiles, JPEG tiles, two tile tables in one file), edited with
sqlite3 where a case needs what GDAL does not make (a missing tile, a tile that is no PNG or of another size, a 16-bit
tile, a WebP header, an undefined system, a matrix too large to read, a table without tiles), and GeoTIFFs (RGB, RGBA,
grey, grey and alpha, 16-bit, a palette, a turned one, one with nodata, one with no system) for the writer.

The rules written here again from ADR 0556: the window `floor((min - origin)/pixel + 1/2)` and `floor((max -
min)/pixel + 1/2)` cut to the grid, tiles side by side in four bands (a grey tile its grey three times, alpha 255; no tile
alpha 0); the pyramid of the writer: the raster's own pixel at its finest zoom with the grid's origin at its top-left
corner, each coarser zoom half the resolution (`(a + b + c + d + 2) // 4` over a canvas padded with 0), the matrix
set's box the coarsest level's single tile and every level's matrix the whole box (2^k tiles a side; the tiles that hold none
of the image are not written, GDAL's validator holds a pyramid to this).

`--check` also opens the GeoTIFFs and GeoPackages KentOS wrote (`KENTOS_WRITE_GPKGTILES=1 cargo test -p
kentos-native-application --test all gpkg_tiles_files`, fixtures/gpkg-tiles/v1/out) with GDAL: the read rasters hold
GDAL's own pixels of the same zoom and window, a written GeoPackage passes GDAL's validate_gpkg.py and GDAL reads the
finest zoom with the raster's own pixels and every coarser one as the rule's mean.
"""
import json
import math
import os
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
MOST_PIXELS = 25_000_000
SIDE = 256
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
    import shutil

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
    return "other"


def tile_tables(path):
    c = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    sets = []
    rows = c.execute(
        "select c.table_name, c.identifier, c.min_x, c.min_y, c.max_x, c.max_y, s.srs_id, s.min_x, s.min_y, s.max_x, s.max_y "
        "from gpkg_contents c join gpkg_tile_matrix_set s on s.table_name = c.table_name where c.data_type = 'tiles' order by c.rowid"
    ).fetchall()
    has_ext = c.execute("select count(*) from sqlite_master where name='gpkg_extensions'").fetchone()[0] > 0
    for name, ident, dx0, dy0, dx1, dy1, srs, bx0, by0, bx1, by1 in rows:
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
        if exts:
            t["extensions"] = exts
        sets.append(t)
    c.close()
    return sets


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
    if inp.get("layer") is not None and target is None:
        raise Refused(
            "invalid_file_path",
            "layer verildi ama output yok; rasteri eklemek için yazılacak GeoTIFF'in yolunu (output) da verin.",
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
    w = window(m, t["bounds"], rect)
    if w is None:
        raise Refused(
            "tile_table_empty",
            f"Okunacak alan “{t['table']}” tablosunun {zoom}. düzeyinin ızgarasının dışında; alanı ızgaraya çekin.",
            "area" if area else "table",
        )
    if w["w"] * w["h"] > MOST_PIXELS:
        raise Refused(
            "gpkg_tiles_too_large",
            f"Raster {w['w']} × {w['h']} piksel olurdu; en çok 25 milyon piksel okunur. Daha kaba bir zoom düzeyi ya da dar bir alan verin.",
            "area" if (inp.get("zoom") is not None or area is not None) else "zoom",
        )
    tw, th = m["tileWidth"], m["tileHeight"]
    c0, c1 = w["x"] // tw, (w["x"] + w["w"] - 1) // tw
    r0, r1 = w["y"] // th, (w["y"] + w["h"] - 1) // th
    c = sqlite3.connect(f"file:{real(path)}?mode=ro", uri=True)
    tiles = c.execute(
        f'select tile_column, tile_row, tile_data from "{t["table"]}" where zoom_level=? and tile_column between ? and ? '
        "and tile_row between ? and ? order by tile_row, tile_column",
        (zoom, c0, c1, r0, r1),
    ).fetchall()
    c.close()
    if not tiles:
        raise Refused(
            "tile_table_empty",
            f"“{t['table']}” tablosunun {zoom}. düzeyinde okunan pencerede hiç karo yok.",
            "area" if area else "table",
        )
    expected = (c1 - c0 + 1) * (r1 - r0 + 1)
    srid = t.get("srid", 0)
    warnings = []
    if t.get("srid") is None:
        warnings.append(
            {"code": "crs_undeclared", "message": "Karo tablosu EPSG koordinat sistemi bildirmiyor; raster sistemsiz alındı.", "path": "table"}
        )
    if len(tiles) < expected:
        warnings.append(
            {
                "code": "tiles_missing",
                "message": f"Okunan pencerenin {expected - len(tiles)} karosu tabloda yok; o yerler saydam (alfa 0) kaldı.",
                "path": "table",
            }
        )
    if target is not None:
        from PIL import Image
        import io

        for col, row, data in tiles:
            at = f"{zoom}/{col}/{row}"
            f = sniff(data)
            if f in ("jpeg", "webp"):
                raise Refused(
                    "tile_format_unsupported",
                    f"{at} karosu {f.upper()}; bu okuyucu yalnız 8 bitlik PNG karoları açar (JPEG ve WebP karolar okunmaz).",
                    "table",
                )
            if f != "png":
                raise Refused("tile_unreadable", f"{at} karosu PNG değil; GeoPackage karoları PNG olmalı.", "table")
            im = Image.open(io.BytesIO(data))
            if im.size != (tw, th):
                raise Refused(
                    "tile_unreadable",
                    f"{at} karosu {im.size[0]} × {im.size[1]} piksel, matrisin karo boyu {tw} × {th}.",
                    "table",
                )
            if data[24] > 8:
                raise Refused(
                    "tile_format_unsupported",
                    f"{at} karosu 8 bitten fazla; bu okuyucu yalnız 8 bitlik PNG karoları açar.",
                    "table",
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
    px, py = m["pixel"]
    ox, oy = w["origin"]
    out = {
        "tables": tables,
        "table": t["table"],
        "zoom": zoom,
        "area": [ox, oy - w["h"] * py, ox + w["w"] * px, oy],
        "width": w["w"],
        "height": w["h"],
        "bands": 4,
        "sample": "u8",
        "affine": [ox, px, 0.0, oy, 0.0, -py],
        "srid": srid,
        "tilesRead": len(tiles),
        "tilesMissing": expected - len(tiles),
        "revision": "$current",
    }
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


def write_ref(doc, inp, copied=(), exists=()):
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
    if ent["sample"] != "u8":
        raise unsupported("8 bitlik değil (karolar 8 bitlik PNG'dir)")
    if not 1 <= ent["bands"] <= 4:
        raise unsupported(f"{ent['bands']} bantlı")
    x0, a, b, y0, c, d = ent["affine"]
    if not (b == 0 and c == 0 and a > 0 and d < 0):
        raise Refused(
            "raster_not_north_up",
            "Raster dönük ya da kuzeyi yukarıda değil; karo ızgarası yatay ve düşey olduğu için önce Raster oturt ile yeniden örnekleyin.",
            "uid",
        )
    w, h = ent["width"], ent["height"]
    if w * h > MOST_PIXELS:
        raise Refused("gpkg_tiles_too_large", f"Raster {w} × {h} piksel; karo olarak en çok 25 milyon piksel yazılır.", "uid")
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
    if hd["type"] != "Byte" or hd["bands"] != ent["bands"]:
        raise unsupported("kaynağının bantları ya da veri türü kayıttakinden başka")
    nb = hd["bands"]
    if nb == 1:
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
    if hd["nodata"] is not None:
        warnings.append(
            {
                "code": "nodata_lost",
                "message": "Rasterin nodata değeri karolara yazılamaz (GeoPackage karoları alfa taşır); değer olduğu gibi piksel sayıldı.",
                "path": "uid",
            }
        )
    lv = levels_of(w, h)
    top = len(lv) - 1
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
                "formats": ["png"],
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
    return out, warnings


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
    case("JPEG karolar okunmaz", {"path": P("jpeg.gpkg"), "output": tmp("jpeg.tif")})
    case("JPEG karolar yazmadan sorulursa tabloyu söyler", {"path": P("jpeg.gpkg")})
    case("WebP karolar okunmaz", {"path": P("webp.gpkg"), "output": tmp("webp.tif")})
    case("PNG olmayan karo", {"path": P("bozuk-karo.gpkg"), "output": tmp("bozuk.tif")})
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
    case("Okunamayacak kadar büyük matris", {"path": P("genis.gpkg")})
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
    ]
    # A persistent id the harness hands out in the order of the objects (`$uidOf:slot`).
    global UIDS
    UIDS = {f"$uidOf:{e['id']}": e for e in ents}

    def uid(i):
        return f"$uidOf:{i}"

    def doc_with():
        return setup(entities=ents)

    def ref(inp, copied=None, exists=()):
        def fn():
            return write_ref(None, _real_uid(inp), copied or {}, exists)

        return result_of(None, inp, fn)

    def step(inp, op="execute", copied=None, exists=()):
        res, _ = ref(inp, copied, exists)
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
        ("16 bitlik raster", {"uid": uid(8), "path": tmp("x.gpkg")}),
        ("Adresten okunan raster", {"uid": uid(12), "path": tmp("x.gpkg")}),
        ("Dönük raster", {"uid": uid(10), "path": tmp("x.gpkg")}),
        ("25 milyon pikseli aşan raster", {"uid": uid(11), "path": tmp("x.gpkg")}),
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


def gdal_tile_array(path, zoom, bands, window=None):
    ds = gdal.OpenEx(str(path), open_options=[f"ZOOM_LEVEL={zoom}", f"BAND_COUNT={bands}"])
    arr = ds.ReadAsArray()
    gt = ds.GetGeoTransform()
    size = (ds.RasterXSize, ds.RasterYSize)
    return arr, gt, size


QGIS = Path("/Applications/QGIS-final-4_2_2.app/Contents")


def qgis_opens(items):
    """What QGIS 4.2 reads of the GeoPackages KentOS wrote: a valid raster of the finest zoom's size and EPSG; none
    when QGIS is not installed."""
    py = QGIS / "MacOS/python3.12"
    if not py.exists():
        print("QGIS 4.2 yok; QGIS denetimi atlandı")
        return []
    code = f"""
import json
from qgis.core import QgsApplication, QgsRasterLayer
QgsApplication.setPrefixPath('{QGIS}/MacOS', True)
app = QgsApplication([], False); app.initQgis()
out = {{}}
for path, table in {items!r}:
    l = QgsRasterLayer('GPKG:' + path + ':' + table, table, 'gdal')
    out[path] = [l.isValid(), l.width(), l.height(), l.crs().authid(), l.bandCount()]
print(json.dumps(out))
"""
    env = dict(
        os.environ,
        QT_QPA_PLATFORM="offscreen",
        PYTHONHOME=str(QGIS / "Frameworks"),
        PYTHONPATH=f"{QGIS}/Resources/qgis/python:{QGIS}/Frameworks/lib/python3.12/site-packages",
        PROJ_DATA=str(QGIS / "Resources/qgis/proj"),
        GDAL_PAM_ENABLED="NO",
    )
    r = subprocess.run([str(py), "-c", code], env=env, capture_output=True, text=True, timeout=180)
    line = [x for x in r.stdout.splitlines() if x.startswith("{")]
    if not line:
        return [f"QGIS okuyamadı: {r.stderr[-300:]}"]
    got = json.loads(line[-1])
    bad = []
    for path, w, h, srid in items_meta(items):
        valid, qw, qh, auth, bands = got[path]
        if not valid or (qw, qh) != (w, h) or (srid and auth != f"EPSG:{srid}"):
            bad.append(f"QGIS {Path(path).name}: geçerli={valid} boy={qw}x{qh} sistem={auth}; beklenen {w}x{h} EPSG:{srid}")
    print(f"{len(items)} GeoPackage QGIS 4.2'de açıldı")
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


def check_files():
    failures = []
    if not OUT_DIR.exists() or not any(OUT_DIR.iterdir()):
        print("KentOS'un yazdığı dosyalar yok (KENTOS_WRITE_GPKGTILES=1 cargo test -p kentos-native-application --test all gpkg_tiles_files); dosya denetimi atlandı")
        return failures
    read_report = json.loads((OUT_DIR / "read.json").read_text()) if (OUT_DIR / "read.json").exists() else []
    write_report = json.loads((OUT_DIR / "write.json").read_text()) if (OUT_DIR / "write.json").exists() else []
    # Rasters read from tiles: GDAL's own pixels of the same zoom and window.
    for r in read_report:
        src = DIR / r["source"]
        tif = OUT_DIR / r["output"]
        zoom = r["zoom"]
        table = tile_tables(str(src))[0]
        m = next(z for z in table["zooms"] if z["zoom"] == zoom)
        px, py = m["pixel"]
        tms = table["bounds"]
        full, gt, size = gdal_tile_array(src, zoom, 4)
        ds = gdal.Open(str(tif))
        got = ds.ReadAsArray()
        g = ds.GetGeoTransform()
        # The window in the grid's pixels, from the GeoTIFF's own place; GDAL's array starts at the data box's pixel.
        mx = round((g[0] - tms[0]) / px)
        my = round((tms[3] - g[3]) / py)
        data = table.get("dataBounds") or tms
        gx = math.floor((data[0] - tms[0]) / px + 0.5)
        gy = math.floor((tms[3] - data[3]) / py + 0.5)
        ox, oy = mx - gx, my - gy
        h, w = got.shape[1], got.shape[2]
        if ox >= 0 and oy >= 0 and ox + w <= full.shape[2] and oy + h <= full.shape[1]:
            want = full[:, oy : oy + h, ox : ox + w]
            if not np.array_equal(want, got):
                failures.append(f"{r['output']}: {int((want != got).sum())} örnek GDAL'ınkinden farklı")
        else:
            # The window passes the data box: the part GDAL holds must match, the rest is transparent.
            ix0, iy0 = max(ox, 0), max(oy, 0)
            ix1, iy1 = min(ox + w, full.shape[2]), min(oy + h, full.shape[1])
            if ix1 > ix0 and iy1 > iy0 and not np.array_equal(
                full[:, iy0:iy1, ix0:ix1], got[:, iy0 - oy : iy1 - oy, ix0 - ox : ix1 - ox]
            ):
                failures.append(f"{r['output']}: kesişen yer GDAL'ınkinden farklı")
        if abs(g[1] - px) > 1e-12 or abs(g[5] + py) > 1e-12:
            failures.append(f"{r['output']}: piksel {g[1]}, {g[5]}; tablonun pikseli {px}, {py}")
        if abs(g[0] - r["origin"][0]) > 1e-9 or abs(g[3] - r["origin"][1]) > 1e-9:
            failures.append(f"{r['output']}: yer {g[0]}, {g[3]}; komutun söylediği {r['origin']}")
        if r.get("defaultWindow"):
            # The data box's window: GDAL's size, and its place within half a pixel (KentOS keeps the pixels' own grid).
            if size != (w, h):
                failures.append(f"{r['output']}: GDAL {size} okuyor, KentOS {(w, h)}")
            if abs(gt[0] - g[0]) > px / 2 + 1e-9 or abs(gt[3] - g[3]) > py / 2 + 1e-9:
                failures.append(f"{r['output']}: yer GDAL'ınkinden yarım pikselden fazla ayrı")
        if r.get("srid"):
            srs = ds.GetSpatialRef()
            if srs is None or srs.GetAuthorityCode(None) != str(r["srid"]):
                failures.append(f"{r['output']}: sistem EPSG:{r['srid']} değil")
        if ds.RasterCount != 4:
            failures.append(f"{r['output']}: {ds.RasterCount} bant")
    # GeoPackages written from rasters: validate_gpkg.py, GDAL's pixels, the rule's coarser zooms.
    from osgeo_utils.samples import validate_gpkg

    for r in write_report:
        path = OUT_DIR / r["file"]
        errs = validate_gpkg.check(str(path), abort_at_first_error=False) or []
        if errs:
            failures.append(f"{r['file']}: validate_gpkg: {errs[:3]}")
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
            ds = gdal.OpenEx(str(path), open_options=[f"ZOOM_LEVEL={zoom}", f"BAND_COUNT={sb}", f"TABLE={r['table']}"])
            got = ds.ReadAsArray()
            if got.ndim == 2:
                got = got[None]
            want = canvas[:, :lh, :lw]
            if got.shape != want.shape:
                failures.append(f"{r['file']} zoom {zoom}: GDAL {got.shape}, kural {want.shape}")
            elif not np.array_equal(got, want):
                failures.append(f"{r['file']} zoom {zoom}: {int((got != want).sum())} örnek kuralınkinden farklı")
        gt = gdal.OpenEx(str(path), open_options=[f"ZOOM_LEVEL={top}", f"BAND_COUNT={sb}", f"TABLE={r['table']}"]).GetGeoTransform()
        e = r["affine"]
        if any(abs(gt[k] - e[k]) > 1e-9 for k in range(6)):
            failures.append(f"{r['file']}: GDAL'ın yeri {gt}, rasterin yeri {e}")
        c = sqlite3.connect(path)
        n = c.execute(f'select count(*) from "{r["table"]}"').fetchone()[0]
        c.close()
        if n != r["tiles"]:
            failures.append(f"{r['file']}: {n} karo, {r['tiles']} bekleniyordu")
    failures += qgis_opens([(str(OUT_DIR / r["file"]), r["table"]) for r in write_report])
    print(f"{len(read_report)} okunan raster ve {len(write_report)} yazılan GeoPackage GDAL'la denetlendi")
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
