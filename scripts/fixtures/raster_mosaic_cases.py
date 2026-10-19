#!/usr/bin/env python3
"""The virtual mosaic (docs/adr/0517; plan/gis.md CBS-28): `cad.rasters.mosaic`'s shared cases and its files, from
GDAL, without KentOS code.

    python3 scripts/fixtures/raster_mosaic_cases.py --files   # (re)write the GeoTIFFs of fixtures/raster/v1/mosaic
    python3 scripts/fixtures/raster_mosaic_cases.py --refs    # (re)write GDAL's own VRTs and their pixels there
    python3 scripts/fixtures/raster_mosaic_cases.py           # write fixtures/commands/v1/cad.rasters.mosaic.json
    python3 scripts/fixtures/raster_mosaic_cases.py --check   # writes nothing; compares all of it

The cases come from the rule of ADR 0517 §3, written here again: each file's header as GDAL reads it (size,
geotransform, bands, data type, nodata, EPSG, colour table, TIFFTAG_DATETIME); the extent their union; the pixel
the finest, coarsest or mean of theirs, each axis on its own (gdalbuildvrt's -resolution), or the one given; the size
`int(0.5 + extent / pixel)`; each file's DstRect `((x₀ − minX)/we, (maxY − y₀)/ns, w·a/we, h·|d|/ns)`; the order the
one given, the finer pixel or the newer date on top. Every completed case is checked against gdalbuildvrt itself
(its raster size, geotransform and DstRects, the files given bottom first) within 1e-9.

`--check` also opens GDAL's VRTs of fixtures/raster/v1/mosaic (gdal-*.vrt, made by gdalbuildvrt with relative
paths) and compares their pixels with the raw files beside them (the KentOS reader is held to the same raw files by
`kentos-formats`' test `raster_mosaic`), and opens the VRTs KentOS's command wrote beside them (kentos-*.vrt,
`KENTOS_WRITE_VRT=1 cargo test -p kentos-native-application --test all mosaic_files`) with GDAL: their pixels must
be GDAL's own VRT's, value for value.
"""
import json
import math
import os
import sys
from pathlib import Path

import numpy as np
from osgeo import gdal, osr

gdal.UseExceptions()

ROOT = Path(__file__).resolve().parents[2]
FIX = ROOT / "fixtures"
MOS = FIX / "raster" / "v1" / "mosaic"
OUT = FIX / "commands" / "v1" / "cad.rasters.mosaic.json"
SRID = 5256
X0, Y0 = 487200.0, 4420600.0
NODATA = -9999.0
SIDE_MOST = 2_000_000_000.0
MOST_SOURCES = 10_000

# ── The files ──────────────────────────────────────────────────────────────────────────────────────────────────


def wkt(srid):
    s = osr.SpatialReference()
    s.ImportFromEPSG(srid)
    return s.ExportToWkt()


def gtiff(name, w, h, bands, dtype, gt=None, srid=SRID, nodata=None, date=None, block=16):
    opts = ["TILED=YES", f"BLOCKXSIZE={block}", f"BLOCKYSIZE={block}", "COMPRESS=DEFLATE"]
    if bands == 3 and dtype == gdal.GDT_Byte:
        opts.append("PHOTOMETRIC=RGB")
    ds = gdal.GetDriverByName("GTiff").Create(str(MOS / name), w, h, bands, dtype, opts)
    if gt is not None:
        ds.SetGeoTransform(gt)
    if srid:
        ds.SetProjection(wkt(srid))
    if date:
        ds.SetMetadataItem("TIFFTAG_DATETIME", date)
    if nodata is not None:
        for b in range(bands):
            ds.GetRasterBand(b + 1).SetNoDataValue(nodata)
    return ds


def dem(name, x0, y0, w, h, px, offset, date, hole=None):
    """A height model: a plane rising east and north from `offset`, so each file is told by its heights; `hole` a
    5 × 5 block of nodata at (column, row)."""
    ds = gtiff(name, w, h, 1, gdal.GDT_Float32, [x0, px, 0, y0, 0, -px], nodata=NODATA, date=date)
    j, i = np.mgrid[0:h, 0:w].astype(np.float64)
    a = (offset + (x0 - X0 + (i + 0.5) * px) / 10.0 + (Y0 - y0 + (j + 0.5) * px) / -20.0).astype(np.float32)
    if hole:
        a[hole[1] : hole[1] + 5, hole[0] : hole[0] + 5] = NODATA
    ds.GetRasterBand(1).WriteArray(a)
    ds = None


def orto(name, x0, y0, w, h, px, tint, seed, date):
    """An orthophoto: fields of a year's colours, a road and a stream, 2 m pixels."""
    rng = np.random.default_rng(seed)
    ds = gtiff(name, w, h, 3, gdal.GDT_Byte, [x0, px, 0, y0, 0, -px], date=date, block=64)
    j, i = np.mgrid[0:h, 0:w].astype(np.float64)
    gx, gy = x0 + (i + 0.5) * px, y0 - (j + 0.5) * px
    # Fields: cells of 40 m, each its own shade.
    cell = (np.floor((gx - X0) / 40.0).astype(int) * 7919 + np.floor((Y0 - gy) / 40.0).astype(int) * 104729) % 97
    shade = rng.uniform(0.75, 1.15, 97)[cell]
    noise = rng.normal(0.0, 0.04, (h, w))
    rgb = np.empty((3, h, w))
    for k in range(3):
        rgb[k] = tint[k] * (shade + noise)
    # A road from the south-west to the north-east and a stream winding east.
    road = np.abs((gy - Y0 + 300.0) - 0.55 * (gx - X0)) < 5.0
    stream = np.abs(gy - (Y0 - 170.0 + 25.0 * np.sin((gx - X0) / 45.0))) < 3.0
    for k, v in enumerate((205, 200, 190)):
        rgb[k][road] = v
    for k, v in enumerate((60, 95, 140)):
        rgb[k][stream] = v
    a = np.clip(rgb, 0, 255).astype(np.uint8)
    for k in range(3):
        ds.GetRasterBand(k + 1).WriteArray(a[k])
    ds = None


def files():
    MOS.mkdir(parents=True, exist_ok=True)
    dem("dem-a.tif", X0, Y0, 40, 30, 10.0, 100.0, "2019:05:01 10:00:00")
    dem("dem-b.tif", X0 + 250, Y0 - 100, 40, 30, 10.0, 150.0, "2023:06:01 10:00:00")
    dem("dem-c.tif", X0 + 200, Y0 - 50, 30, 30, 5.0, 200.0, "2021:07:01 10:00:00", hole=(10, 10))
    dem("dem-d.tif", X0 + 100, Y0 - 200, 20, 20, 10.0, 250.0, None)
    dem("dem-e.tif", X0 + 300, Y0 - 250, 20, 20, 7.5, 300.0, "2020:01:15 08:30:00")
    orto("orto-2019.tif", X0, Y0, 220, 160, 2.0, (95, 135, 70), 2019, "2019:06:10 11:00:00")
    orto("orto-2021.tif", X0 + 260, Y0 - 90, 220, 160, 2.0, (175, 160, 95), 2021, "2021:06:12 11:00:00")
    orto("orto-2023.tif", X0 + 130, Y0 - 200, 220, 160, 2.0, (130, 130, 125), 2023, "2023:06:14 11:00:00")
    # What is refused.
    small = lambda name, **k: gtiff(name, 10, 10, k.pop("bands", 1), k.pop("dtype", gdal.GDT_Float32), **k)  # noqa: E731

    def filled(name, **k):
        ds = small(name, **k)
        for b in range(ds.RasterCount):
            ds.GetRasterBand(b + 1).Fill(b + 1)
        ds = None

    filled("baska-sistem.tif", gt=[X0, 10, 0, Y0, 0, -10], srid=5255)
    filled("donuk.tif", gt=[X0, 8.66, 5, Y0, 5, -8.66])
    filled("iki-bant.tif", gt=[X0, 10, 0, Y0, 0, -10], bands=2)
    filled("tam-sayi.tif", gt=[X0, 10, 0, Y0, 0, -10], dtype=gdal.GDT_Int16)
    filled("yersiz.tif", srid=0)
    ds = small("paletli.tif", gt=[X0, 10, 0, Y0, 0, -10], dtype=gdal.GDT_Byte)
    ct = gdal.ColorTable()
    ct.SetColorEntry(0, (0, 0, 0, 255))
    ct.SetColorEntry(1, (255, 0, 0, 255))
    ds.GetRasterBand(1).SetRasterColorTable(ct)
    ds.GetRasterBand(1).Fill(1)
    ds = None
    for k, x in enumerate((0.0, 100.0)):
        d = small(f"yerel-{'ab'[k]}.tif", gt=[x, 10, 0, 100, 0, -10], srid=0, dtype=gdal.GDT_Byte)
        d.GetRasterBand(1).Fill(10 * (k + 1))
        d = None
    png = gdal.GetDriverByName("MEM").Create("", 4, 4, 1, gdal.GDT_Byte)
    gdal.GetDriverByName("PNG").CreateCopy(str(MOS / "resim.png"), png)
    for aux in MOS.glob("*.aux.xml"):
        aux.unlink()


# ── The rule (ADR 0517 §3) ─────────────────────────────────────────────────────────────────────────────────────


def fx(rel):
    return f"$fixture:raster/v1/mosaic/{rel}"


def tmp(name):
    return f"$tmp:{name}"


def real(text):
    if text.startswith("$fixture:"):
        return str(FIX / text[len("$fixture:") :])
    return text


def name_of(text):
    if text.startswith("$"):
        text = text.split(":", 1)[1]
    return text.rstrip("/").rsplit("/", 1)[-1]


def rust(v):
    """A float as Rust's Display writes it: the shortest decimal, no exponent, no `.0`."""
    if v == int(v) and abs(v) < 1e15:
        return str(int(v))
    return repr(v)


class Refused(Exception):
    def __init__(self, code, message, path):
        super().__init__(message)
        self.code, self.message, self.path = code, message, path


def header(text, i):
    """A source as GDAL reads its header; the refusals of ADR 0517 §3 one file at a time."""
    at = f"sources[{i}]"
    path = real(text)
    name = name_of(text)
    if not Path(path).is_file():
        raise Refused("file_not_found", f"{name} bulunamadı. Bakılan yerler: {text}.", at)
    with open(path, "rb") as f:
        head = f.read(4)
    if head[:4] not in (b"II*\0", b"MM\0*", b"II+\0", b"MM\0+"):
        raise Refused("import_unreadable", f"{name} okunamadı (GeoTIFF değil); mozaiğe yalnız GeoTIFF konur.", at)
    ds = gdal.Open(path)
    gt = ds.GetGeoTransform(can_return_null=True)
    if gt is None:
        raise Refused(
            "mosaic_unplaced",
            f"{name} dosyasının yerini GeoTIFF etiketleri söylemiyor; önce Raster oturt ile yerine koyup GeoTIFF olarak yazın.",
            at,
        )
    if gt[2] != 0 or gt[4] != 0 or not gt[1] > 0 or not gt[5] < 0:
        raise Refused(
            "mosaic_rotated",
            f"{name} dönük ya da kuzeyi yukarıda değil; mozaik yalnız kuzeyi yukarıda dosyaları yan yana koyar.",
            at,
        )
    b1 = ds.GetRasterBand(1)
    if b1.GetRasterColorTable() is not None:
        raise Refused("mosaic_mismatch", f"{name} paletli; paletli raster mozaiğe konmaz (önce RGB'ye çevirin).", at)
    srs = ds.GetSpatialRef()
    epsg = int(srs.GetAuthorityCode(None)) if srs is not None and srs.GetAuthorityCode(None) else None
    date = ds.GetMetadataItem("TIFFTAG_DATETIME")
    iso = None
    if date and len(date) == 19:
        iso = f"{date[0:4]}-{date[5:7]}-{date[8:10]}T{date[11:19]}"
    return {
        "text": text,
        "given": i,
        "w": ds.RasterXSize,
        "h": ds.RasterYSize,
        "bands": ds.RasterCount,
        "type": gdal.GetDataTypeName(b1.DataType),
        "epsg": epsg,
        "nodata": b1.GetNoDataValue(),
        "gt": list(gt),
        "date": iso,
        "rgb": ds.RasterCount >= 3 and b1.GetColorInterpretation() == gdal.GCI_RedBand,
    }


def mosaic(doc, inp):
    """`cad.rasters.mosaic`'s answer: (output, warnings) or Refused."""
    srcs = inp.get("sources", [])
    if not srcs:
        raise Refused("mosaic_empty", "Mozaiğe konacak GeoTIFF dosyası verilmedi.", "sources")
    if len(srcs) > MOST_SOURCES:
        raise Refused("mosaic_too_large", f"{len(srcs)} dosya verildi; mozaik en çok {MOST_SOURCES} dosya alır.", "sources")
    text = inp["path"].strip()
    drawing = inp.get("drawing")
    if not text:
        raise Refused("invalid_file_path", "Mozaiğin dosya yolu boş; bir .vrt dosyasının yolunu verin.", "path")
    if not text.lower().endswith(".vrt"):
        raise Refused(
            "invalid_file_path", f"“{name_of(text)}” bir .vrt dosyası değil; mozaik GDAL'ın VRT dosyası olarak yazılır.", "path"
        )
    absolute = text.startswith("/") or text.startswith("$")
    if absolute:
        path = text
    elif drawing:
        path = drawing.rsplit("/", 1)[0] + "/" + text
    else:
        raise Refused(
            "invalid_file_path",
            f"“{text}” göreli bir yol, ama çizimin dosyası yok; mutlak yolu verin ya da çizimi önce kaydedin.",
            "path",
        )
    folder = path.rsplit("/", 1)[0]
    for i, s in enumerate(srcs):
        t = s.strip()
        if not t:
            raise Refused("invalid_file_path", f"{i + 1}. dosyanın yolu boş.", f"sources[{i}]")
        if not (t.startswith("/") or t.startswith("$")) and not drawing:
            raise Refused(
                "invalid_file_path",
                f"“{t}” göreli bir yol, ama çizimin dosyası yok; mutlak yolu verin ya da çizimi önce kaydedin.",
                f"sources[{i}]",
            )
    p = inp.get("pixelSize")
    if p is not None and not (math.isfinite(p) and p > 0):
        raise Refused("invalid_pixel_size", "Piksel boyu sıfırdan büyük, sonlu bir sayı olmalı.", "pixelSize")
    files = []
    for i, s in enumerate(srcs):
        t = s.strip()
        if not (t.startswith("/") or t.startswith("$")):
            t = drawing.rsplit("/", 1)[0] + "/" + t
        f = header(t, i)
        f["name"] = name_of(s)
        if files:
            first = files[0]
            sysname = lambda e: "sistemsiz" if e is None else f"EPSG:{e}"  # noqa: E731
            why = None
            if f["epsg"] != first["epsg"]:
                why = f"{f['name']} {sysname(f['epsg'])}, {first['name']} {sysname(first['epsg'])}"
            elif f["bands"] != first["bands"]:
                why = f"{f['name']} {f['bands']} bantlı, {first['name']} {first['bands']} bantlı"
            elif f["type"] != first["type"]:
                why = f"{f['name']} {f['type']}, {first['name']} {first['type']}"
            if why:
                raise Refused(
                    "mosaic_mismatch",
                    f"Dosyalar mozaiğe birlikte konmaz: {why}. Hepsi aynı sistemde, bant sayısında ve veri türünde olmalı.",
                    f"sources[{i}]",
                )
        files.append(f)
    min_x = min(f["gt"][0] for f in files)
    max_y = max(f["gt"][3] for f in files)
    max_x = max(f["gt"][0] + f["w"] * f["gt"][1] for f in files)
    min_y = min(f["gt"][3] + f["h"] * f["gt"][5] for f in files)
    res = inp.get("resolution", "highest")
    if p is not None:
        we = ns = p
    elif res == "highest":
        we, ns = min(f["gt"][1] for f in files), min(-f["gt"][5] for f in files)
    elif res == "lowest":
        we, ns = max(f["gt"][1] for f in files), max(-f["gt"][5] for f in files)
    else:
        sw = sn = 0.0
        for f in files:
            sw += f["gt"][1]
            sn += -f["gt"][5]
        we, ns = sw / len(files), sn / len(files)
    wx, wy = 0.5 + (max_x - min_x) / we, 0.5 + (max_y - min_y) / ns
    if not (wx < SIDE_MOST and wy < SIDE_MOST):
        raise Refused(
            "mosaic_too_large",
            f"Mozaik {wx:.0f} × {wy:.0f} piksel olurdu; bir kenarı en çok 2 milyar piksel olabilir. Daha büyük bir piksel boyu verin.",
            "pixelSize" if p is not None else "resolution",
        )
    width, height = max(int(wx), 1), max(int(wy), 1)
    if inp.get("_exists") and inp.get("overwrite") is not True:
        raise Refused("file_exists", f"{path} var; üzerine yazmak için overwrite verin.", "path")
    order = inp.get("order", "given")
    ranked = list(files)
    if order == "finest":
        ranked.sort(key=lambda f: (f["gt"][1] * -f["gt"][5], f["given"]))
    elif order == "newest":
        dated = sorted([f for f in files if f["date"]], key=lambda f: f["given"])
        dated.sort(key=lambda f: f["date"], reverse=True)
        ranked = dated + [f for f in files if not f["date"]]
    bands, typ = files[0]["bands"], files[0]["type"]
    srid = files[0]["epsg"] or 0
    nodata = inp.get("nodata")
    if nodata is None:
        nodata = next((f["nodata"] for f in files if f["nodata"] is not None), None)
    sources = []
    kept = []
    for f in ranked:
        x0, a, _, y0, _, d = f["gt"]
        # gdalbuildvrt's GetSrcDstWin: scaled, cut where the rounded size ends.
        rx, ry = a / we, -d / ns
        sw, sh = float(f["w"]), float(f["h"])
        dx, dy = (x0 - min_x) / we, (max_y - y0) / ns
        dw, dh = sw * rx, sh * ry
        if dx + dw > width:
            dw = width - dx
            sw = dw / rx
        if dy + dh > height:
            dh = height - dy
            sh = dh / ry
        if not (sw > 0 and sh > 0 and dw > 0 and dh > 0):
            continue
        kept.append(f)
        src, dst = [0.0, 0.0, sw, sh], [dx, dy, dw, dh]
        k = len(sources)
        rel = None
        if not path.startswith("$tmp:") and f["text"].startswith(folder + "/"):
            rel = f["text"][len(folder) + 1 :]
        s = {
            "file": rel if rel is not None else f["text"],
            "read": f["text"],
            "rank": k + 1,
            "width": f["w"],
            "height": f["h"],
            "pixel": [a, -d],
            "src": src,
            "dst": dst,
        }
        if f["date"]:
            s["date"] = f["date"]
        if f["nodata"] is not None:
            s["nodata"] = f["nodata"]
        sources.append(dict(sorted(s.items(), key=lambda kv: KEYS.index(kv[0]))))
    warnings = []
    resampled = [f["name"] for f in files if f["gt"][1] != we or -f["gt"][5] != ns]
    if resampled:
        warnings.append(
            {
                "code": "mosaic_resampled",
                "message": f"{', '.join(resampled)} {'dosyasının' if len(resampled) == 1 else 'dosyalarının'} pikseli mozaiğinkinden ({rust(we)} × {rust(ns)}) başka; en yakın pikselden okunur.",
                "path": "sources",
            }
        )
    if order == "newest":
        undated = [f["name"] for f in files if not f["date"]]
        if undated:
            warnings.append(
                {
                    "code": "mosaic_undated",
                    "message": f"{', '.join(undated)} {'dosyasının' if len(undated) == 1 else 'dosyalarının'} tarihi (TIFF DateTime) yok; en alta {'kondu' if len(undated) == 1 else 'kondular'}.",
                    "path": "order",
                }
            )
    layer = inp.get("layer")
    added = False
    if layer is not None:
        node = next((l for l in doc["layers"] if l["id"] == layer), None)
        if node is None:
            raise Refused("layer_not_found", f"“{layer}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layer")
        if node["type"] != "layer":
            raise Refused(
                "not_a_layer",
                f"“{node['name']}” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.",
                "layer",
            )
        if node["locked"]:
            raise Refused(
                "layer_locked",
                f"“{node['name']}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.",
                "layer",
            )
        if not node["visible"]:
            warnings.append(
                {"code": "layer_hidden", "message": f"“{node['name']}” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layer"}
            )
        project = doc["settings"]["srid"]
        if srid != 0 and srid != project:
            raise Refused(
                "crs_mismatch",
                f"Mozaiğin sistemi EPSG:{srid}, projeninki {'yerel' if project == 0 else f'EPSG:{project}'}; raster çevrilmeden eklenmez. Dosyayı yazmak için layer'ı vermeyin.",
                "layer",
            )
        added = True
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
    rgb = files[0]["rgb"] and bands in (3, 4)
    colours = [
        ("Red" if b == 0 else "Green" if b == 1 else "Blue" if b == 2 else "Alpha")
        if rgb
        else ("Gray" if b == 0 else "Undefined")
        for b in range(bands)
    ]
    out = {
        "path": path,
        "width": width,
        "height": height,
        "bands": bands,
        "sample": SAMPLE[typ],
        "affine": [min_x, we, 0.0, max_y, 0.0, -ns],
        "srid": srid,
        "sources": sources,
        # A VRT naming its files by absolute paths is as long as the checkout's path: then any length.
        "bytes": len(vrt_text(width, height, [min_x, we, 0.0, max_y, 0.0, -ns], srid, typ, nodata, colours, kept, sources))
        if all(x["file"] != x["read"] for x in sources)
        else "$any",
        "revision": "$current",
    }
    if nodata is not None:
        out["nodata"] = nodata
    return out, warnings, added, files


KEYS = ["file", "read", "rank", "width", "height", "pixel", "date", "src", "dst", "nodata"]
SAMPLE = {"Byte": "u8", "Int8": "i8", "UInt16": "u16", "Int16": "i16", "UInt32": "u32", "Int32": "i32", "Float32": "f32", "Float64": "f64"}


def vnum(v):
    if isinstance(v, float) and math.isnan(v):
        return "nan"
    if v == int(v) and abs(v) < 1e15:
        return str(int(v))
    return repr(v)


def vrt_text(width, height, affine, srid, typ, nodata, colours, ranked, sources):
    """The VRT as ADR 0517 §2 writes it (GDAL's elements; a source with nodata a ComplexSource), for its length."""
    s = f'<VRTDataset rasterXSize="{width}" rasterYSize="{height}">\n'
    if srid:
        s += f"  <SRS>EPSG:{srid}</SRS>\n"
    s += "  <GeoTransform>" + ", ".join(vnum(x) for x in affine) + "</GeoTransform>\n"
    for b, colour in enumerate(colours):
        s += f'  <VRTRasterBand dataType="{typ}" band="{b + 1}">\n'
        if nodata is not None:
            s += f"    <NoDataValue>{vnum(nodata)}</NoDataValue>\n"
        s += f"    <ColorInterp>{colour}</ColorInterp>\n"
        for f, src in reversed(list(zip(ranked, sources))):
            tag = "ComplexSource" if f["nodata"] is not None else "SimpleSource"
            rel = 1 if src["file"] != src["read"] else 0
            fname = esc(src["file"])
            s += f"    <{tag}>\n"
            s += f'      <SourceFilename relativeToVRT="{rel}">{fname}</SourceFilename>\n'
            s += f"      <SourceBand>{b + 1}</SourceBand>\n"
            r = src["src"]
            s += f'      <SrcRect xOff="{vnum(r[0])}" yOff="{vnum(r[1])}" xSize="{vnum(r[2])}" ySize="{vnum(r[3])}" />\n'
            d = src["dst"]
            s += f'      <DstRect xOff="{vnum(d[0])}" yOff="{vnum(d[1])}" xSize="{vnum(d[2])}" ySize="{vnum(d[3])}" />\n'
            if f["nodata"] is not None:
                s += f"      <NODATA>{vnum(f['nodata'])}</NODATA>\n"
            s += f"    </{tag}>\n"
        s += "  </VRTRasterBand>\n"
    s += "</VRTDataset>\n"
    return s.encode("utf-8")


def esc(t):
    return t.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace('"', "&quot;").replace("'", "&apos;")


# ── The cases ──────────────────────────────────────────────────────────────────────────────────────────────────


def setup(srid=SRID, workspace="gis"):
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
        "name": "Mozaik",
        "settings": {
            "srid": srid,
            "lengthDecimals": 3,
            "areaDecimals": 2,
            "areaUnit": "m2",
            "angleUnit": "deg",
            "plotScale": 1000,
            "workspace": workspace,
        },
        "origin": {"x": X0, "y": Y0},
        "layers": [
            layer("altlik", "Altlık"),
            layer("gizli", "Gizli", vis=False),
            layer("kilitli", "Kilitli", lock=True),
            dict(layer("grup", "Grup"), type="group"),
        ],
        "activeLayer": "altlik",
        "entities": [],
        "styles": {"items": [], "categories": []},
    }


DEM = [fx("dem-a.tif"), fx("dem-b.tif"), fx("dem-c.tif")]
ORTO = [fx("orto-2019.tif"), fx("orto-2021.tif"), fx("orto-2023.tif")]
DRAWING = fx("proje.kcad")


def step(doc, op, inp, exists=False):
    """One step: the reference's answer, as the harness compares it."""
    probe = dict(inp, _exists=exists)
    try:
        out, warnings, added, _ = mosaic(doc, probe)
    except Refused as r:
        error = {"code": r.code, "message": r.message, "path": r.path}
        status = "failed"
        if r.code == "revision_conflict":
            status, error["revision"] = "conflict", "$current"
        return {"op": op, "input": inp, "result": {"status": status, "error": error}, "expect": {"canUndo": False, "revision": "same"}}
    written = op == "execute" and added
    if written:
        out["uid"] = "$uidOf:1"
        out = {k: out[k] for k in ["path", "width", "height", "bands", "sample", "affine", "srid", "nodata", "sources", "bytes", "uid", "revision"] if k in out}
    expect = {"canUndo": written, "revision": "changed" if written else "same"}
    if written:
        expect["fields"] = {
            "1": {
                "kind": "raster",
                "layerId": inp["layer"],
                "file": inp["path"],
                "width": out["width"],
                "height": out["height"],
                "bands": out["bands"],
                "sample": out["sample"],
                "affine": out["affine"],
                "srid": out["srid"],
            }
        }
    return {"op": op, "input": inp, "result": {"status": "completed", "output": out, "warnings": warnings}, "expect": expect}


cases = []


def case(name, steps, doc=None):
    d = doc or setup()
    built = []
    written = set()
    for op, inp in steps:
        target = inp.get("path", "")
        exists = target in written
        s = step(d, op, inp, exists)
        if op == "execute" and s["result"]["status"] == "completed":
            written.add(target)
        built.append(s)
    cases.append({"name": name, "setup": d, "steps": built})


def build():
    cases.clear()
    case("Verilen sıra: ilk dosya üstte, en ince piksel (5 m), VRT yazılır", [("execute", {"sources": DEM, "path": tmp("dem.vrt")})])
    case("Plan: dosya yazılmadan mozaik söylenir", [("plan", {"sources": DEM, "path": tmp("plan.vrt")})])
    case("Önce plan, sonra aynı mozaik yazılır", [("plan", {"sources": DEM, "path": tmp("iki.vrt")}), ("execute", {"sources": DEM, "path": tmp("iki.vrt")})])
    case("İnce çözünürlük üstte: 5 m'lik dosya en üstte", [("execute", {"sources": DEM, "path": tmp("ince.vrt"), "order": "finest"})])
    case("Yeni tarih üstte: 2023, 2021, 2019", [("execute", {"sources": DEM, "path": tmp("yeni.vrt"), "order": "newest"})])
    case(
        "Yeni tarih üstte, tarihsiz dosya en altta ve söylenir",
        [("execute", {"sources": [fx("dem-d.tif")] + DEM, "path": tmp("tarihsiz.vrt"), "order": "newest"})],
    )
    case("En kaba çözünürlük: 10 m", [("execute", {"sources": DEM, "path": tmp("kaba.vrt"), "resolution": "lowest"})])
    case("Ortalama çözünürlük: 10 m ile 5 m'den 7,5 m", [("execute", {"sources": [fx("dem-a.tif"), fx("dem-c.tif")], "path": tmp("ortalama.vrt"), "resolution": "average"})])
    case("Ortalama çözünürlük: 7,5 m'lik dosya da", [("execute", {"sources": [fx("dem-a.tif"), fx("dem-e.tif"), fx("dem-c.tif")], "path": tmp("ortalama3.vrt"), "resolution": "average"})])
    case("Verilen piksel boyu 2,5 m", [("execute", {"sources": DEM, "path": tmp("piksel.vrt"), "pixelSize": 2.5})])
    case("Mozaiğin nodata'sı verilen", [("execute", {"sources": DEM, "path": tmp("nodata.vrt"), "nodata": 0.0})])
    case("Tek dosya: mozaik o dosyanın kendisi", [("execute", {"sources": [fx("dem-b.tif")], "path": tmp("tek.vrt")})])
    case("Ortofotolar: RGB, nodata'sız, aynı piksel", [("execute", {"sources": ORTO, "path": tmp("orto.vrt"), "order": "newest"})])
    case(
        "Katmana eklenir: tek geri alma adımı, rasterin dosyası VRT",
        [("execute", {"sources": ORTO, "path": tmp("katman.vrt"), "order": "newest", "layer": "altlik"})],
    )
    case("Gizli katmana eklenir ve söylenir", [("execute", {"sources": DEM, "path": tmp("gizli.vrt"), "layer": "gizli"})])
    case(
        "Göreli yollar: dosyalar VRT'nin klasöründen adlarıyla (plan)",
        [("plan", {"sources": ["dem-a.tif", "dem-b.tif"], "path": "goreli.vrt", "drawing": DRAWING})],
    )
    case(
        "Sistemsiz dosyalar sistemli projeye sistemsiz eklenir",
        [("execute", {"sources": [fx("yerel-a.tif"), fx("yerel-b.tif")], "path": tmp("yerel.vrt"), "layer": "altlik"})],
    )
    case(
        "Var olan dosya: overwrite'sız reddedilir, overwrite ile yazılır",
        [
            ("execute", {"sources": DEM, "path": tmp("var.vrt")}),
            ("execute", {"sources": DEM, "path": tmp("var.vrt"), "order": "newest"}),
            ("execute", {"sources": DEM, "path": tmp("var.vrt"), "order": "newest", "overwrite": True}),
        ],
    )
    case("Ret: dosya verilmedi", [("execute", {"sources": [], "path": tmp("bos.vrt")})])
    case("Ret: yol .vrt değil", [("execute", {"sources": DEM, "path": tmp("mozaik.tif")})])
    case("Ret: göreli yol, çizimin dosyası yok", [("execute", {"sources": DEM, "path": "mozaik.vrt"})])
    case("Ret: göreli kaynak, çizimin dosyası yok", [("execute", {"sources": ["dem-a.tif"], "path": tmp("k.vrt")})])
    case("Ret: piksel boyu sıfır", [("execute", {"sources": DEM, "path": tmp("p.vrt"), "pixelSize": 0.0})])
    case("Ret: dosya yok", [("execute", {"sources": [fx("dem-a.tif"), fx("yok.tif")], "path": tmp("y.vrt")})])
    case("Ret: PNG GeoTIFF değil", [("execute", {"sources": [fx("dem-a.tif"), fx("resim.png")], "path": tmp("r.vrt")})])
    case("Ret: yeri belli olmayan TIFF", [("execute", {"sources": [fx("yersiz.tif")], "path": tmp("ys.vrt")})])
    case("Ret: dönük dosya", [("execute", {"sources": [fx("dem-a.tif"), fx("donuk.tif")], "path": tmp("d.vrt")})])
    case("Ret: başka koordinat sistemi", [("execute", {"sources": [fx("dem-a.tif"), fx("baska-sistem.tif")], "path": tmp("s.vrt")})])
    case("Ret: başka bant sayısı", [("execute", {"sources": [fx("dem-a.tif"), fx("iki-bant.tif")], "path": tmp("b.vrt")})])
    case("Ret: başka veri türü", [("execute", {"sources": [fx("dem-a.tif"), fx("tam-sayi.tif")], "path": tmp("t.vrt")})])
    case("Ret: paletli dosya", [("execute", {"sources": [fx("paletli.tif")], "path": tmp("pl.vrt")})])
    case("Ret: mozaik çok büyük", [("execute", {"sources": DEM, "path": tmp("buyuk.vrt"), "pixelSize": 3e-7})])
    case("Ret: katman yok", [("execute", {"sources": DEM, "path": tmp("kl.vrt"), "layer": "yok"})])
    case("Ret: katman kilitli", [("execute", {"sources": DEM, "path": tmp("kk.vrt"), "layer": "kilitli"})])
    case("Ret: katman grubu", [("execute", {"sources": DEM, "path": tmp("kg.vrt"), "layer": "grup"})])
    case("Ret: beklenen sürüm sayı değil", [("execute", {"sources": DEM, "path": tmp("rs.vrt"), "expectedRevision": "yedi"})])
    case(
        "Ret: mozaiğin sistemi projeninkinden başka",
        [("execute", {"sources": DEM, "path": tmp("ks.vrt"), "layer": "altlik"})],
        doc=setup(srid=0, workspace="cad"),
    )
    case("Ret: çizim değişti", [("execute", {"sources": DEM, "path": tmp("rv.vrt"), "layer": "altlik", "expectedRevision": "7"})])
    doc = {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.rasters.mosaic",
        "commandVersion": 1,
        "title": "Sanal mozaik",
        "note": "Written by scripts/fixtures/raster_mosaic_cases.py from docs/adr/0517 §3 with GDAL; do not edit by hand.",
        "setup": setup(),
        "cases": cases,
    }
    return json.dumps(doc, ensure_ascii=False, indent=1) + "\n"


# ── GDAL's own VRT for every completed case ────────────────────────────────────────────────────────────────────


def against_gdal():
    """Each completed step's size, geotransform and DstRects are gdalbuildvrt's (files given bottom first)."""
    n = 0
    for c in cases:
        for s in c["steps"]:
            if s["result"]["status"] != "completed":
                continue
            inp, out = s["input"], s["result"]["output"]
            files = [real(x["read"]) for x in reversed(out["sources"])]
            opts = {"resolution": inp.get("resolution", "highest")}
            if inp.get("pixelSize") is not None:
                opts = {"resolution": "user", "xRes": inp["pixelSize"], "yRes": inp["pixelSize"]}
            v = gdal.BuildVRT("/vsimem/m.vrt", files, **opts)
            assert (v.RasterXSize, v.RasterYSize) == (out["width"], out["height"]), (c["name"], v.RasterXSize, out["width"])
            g = v.GetGeoTransform()
            assert all(abs(p - q) <= 1e-9 * max(1.0, abs(q)) for p, q in zip(g, out["affine"])), (c["name"], g)
            v = None
            text = gdal.VSIFOpenL("/vsimem/m.vrt", "rb")
            body = gdal.VSIFReadL(1, 1 << 20, text).decode()
            gdal.VSIFCloseL(text)
            gdal.Unlink("/vsimem/m.vrt")
            for tag, key in (("<SrcRect ", "src"), ("<DstRect ", "dst")):
                rects = []
                for part in body.split(tag)[1:]:
                    head = part.split("/>")[0]
                    vals = dict(kv.split("=") for kv in head.split())
                    rects.append([float(vals[k].strip('"')) for k in ("xOff", "yOff", "xSize", "ySize")])
                want = [x[key] for x in reversed(out["sources"])] * out["bands"]
                assert len(rects) == len(want), c["name"]
                for p, q in zip(rects, want):
                    assert all(abs(a - b) <= 1e-9 * max(1.0, abs(b)) for a, b in zip(p, q)), (c["name"], key, p, q)
            n += 1
    return n


# ── The pixels: GDAL's VRTs, their raw values, and KentOS's VRTs read by GDAL ─────────────────────────────────

REFS = {
    "dem-verilen": ([fx("dem-c.tif"), fx("dem-b.tif"), fx("dem-a.tif")], "highest"),
    "dem-kaba": ([fx("dem-a.tif"), fx("dem-b.tif"), fx("dem-c.tif"), fx("dem-e.tif")], "lowest"),
    "orto": ([fx("orto-2019.tif"), fx("orto-2021.tif"), fx("orto-2023.tif")], "highest"),
}


def refs():
    for name, (srcs, res) in REFS.items():
        cwd = os.getcwd()
        os.chdir(MOS)
        try:
            gdal.BuildVRT(f"gdal-{name}.vrt", [name_of(s) for s in srcs], resolution=res).FlushCache()
        finally:
            os.chdir(cwd)
        a = gdal.Open(str(MOS / f"gdal-{name}.vrt")).ReadAsArray()
        if a.ndim == 3:
            a = np.moveaxis(a, 0, -1)
        (MOS / f"gdal-{name}.raw").write_bytes(np.ascontiguousarray(a).astype(a.dtype.newbyteorder("<")).tobytes())


def pixels():
    n = 0
    for name in REFS:
        a = gdal.Open(str(MOS / f"gdal-{name}.vrt")).ReadAsArray()
        if a.ndim == 3:
            a = np.moveaxis(a, 0, -1)
        raw = np.ascontiguousarray(a).astype(a.dtype.newbyteorder("<")).tobytes()
        assert (MOS / f"gdal-{name}.raw").read_bytes() == raw, f"gdal-{name}.raw güncel değil"
        kentos = MOS / f"kentos-{name}.vrt"
        if not kentos.exists():
            sys.exit(f"{kentos.relative_to(ROOT)} yok: KENTOS_WRITE_VRT=1 cargo test -p kentos-native-application --test all mosaic_files")
        b = gdal.Open(str(kentos)).ReadAsArray()
        if b.ndim == 3:
            b = np.moveaxis(b, 0, -1)
        assert a.shape == b.shape and a.dtype == b.dtype, (name, a.shape, b.shape)
        assert np.array_equal(a, b, equal_nan=True), f"{name}: KentOS'un VRT'si GDAL'ınkinden başka pikseller veriyor"
        n += 1
    return n


def main():
    args = sys.argv[1:]
    if "--files" in args:
        files()
        print("dosyalar yazıldı")
        return
    if "--refs" in args:
        refs()
        print("GDAL'ın VRT'leri yazıldı")
        return
    text = build()
    checked = against_gdal()
    if "--check" in args:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"{OUT.relative_to(ROOT)} güncel değil")
        p = pixels()
        print(f"{len(cases)} durum tutarlı; {checked} adım gdalbuildvrt'ye uyuyor; {p} mozaiğin pikselleri GDAL'ınkiler")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{len(cases)} durum yazıldı; {checked} adım gdalbuildvrt'ye uyuyor")


main()
