#!/usr/bin/env python3
"""Rasteri yeniden izdüşür (docs/adr/0644 §3; plan/gis.md CBS-31): `cad.rasters.reproject`'in ortak durumları ve
dosyaları, GDAL ve PROJ'la, KentOS kodu olmadan.

    /usr/bin/python3 scripts/fixtures/raster_reproject_cases.py --files    # fixtures/raster/v1/reproject'in GeoTIFF'lerini yazar
    /usr/bin/python3 scripts/fixtures/raster_reproject_cases.py            # fixtures/commands/v1/cad.rasters.reproject.json'u yazar
    /usr/bin/python3 scripts/fixtures/raster_reproject_cases.py --check    # hiçbir şey yazmaz; karşılaştırır
    /usr/bin/python3 scripts/fixtures/raster_reproject_cases.py --written  # KentOS'un yazdığı rasterleri denetler

Kural ADR 0644 §3'ten yeniden yazılır. Kaynağın her pikselinin köşeleri (kenar başına 20 adım) PROJ'la projenin sistemine
taşınır; ızgara onları örten, köşesi piksel boyunun katında, kuzeyi yukarıda ızgaradır (bir pikselin milyonda birinden
ince kenar şeridi satır ya da sütun yapmaz). Piksel boyu verilmezse kaynağın orta pikselinin projede kapladığı alanın
karekökü, altı anlamlı basamağa yuvarlanır. Her çıktı pikselinin merkezi PROJ'la kaynağın sistemine, oradan kaynağın
pikseline döner: en yakın komşu o pikselin değeri; çift doğrusalda çevresindeki dört pikselin ağırlıklı ortalaması
(kaynağın kenarına kıstırılır, değersiz piksel ağırlığıyla dışarıda kalır). Kaynağın dışına düşen piksel değersizdir.
Dönüşüm yolu crs_transform_cases.py'nin kuralıyla (aynı datumda yalnız izdüşüm, TUREF–WGS 84 EPSG:5261 ±1 m, ED50'den
EPSG:1783 + EPSG:5260 ±2,1 m). 8 bitlik ve çok bantlı kaynak değerleriyle kalır (RGBA'ya çevrilmez).

--written KentOS'un yazdığı dosyaları (KENTOS_WRITE_REPROJECT=1 cargo test -p kentos-native-application --test all
reproject_files, fixtures/raster/v1/reproject/out) bu kuralın değerleriyle karşılaştırır: KentOS ters dönüşümü sekiz
pikselde bir tam, aradakileri çift doğrusal hesaplar (en çok 0,02 piksel; ADR 0204 §6), bu yüzden en yakın komşuda
bir piksel kenarına 0,025 pikselden yakın düşen merkezler sayılmaz, çift doğrusalda fark değerin eğimiyle sınırlıdır.
Her durum ayrıca gdalwarp'ın aynı ızgaraya aynı PROJ yoluyla yaptığıyla karşılaştırılır: en yakın komşu piksel piksel
aynıdır (renkli rasterde gdalwarp 0 değerini değersizle karışmasın diye 1 yapar); çift doğrusalda gdalwarp tam sayıya
yuvarlar ve küçülten yönde çekirdeği genişletir, fark bu kadardır.
"""
import json
import math
import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np
from osgeo import gdal, osr

sys.path.insert(0, str(Path(__file__).resolve().parent))
import crs_transform_cases as ct  # noqa: E402

gdal.UseExceptions()

ROOT = Path(__file__).resolve().parents[2]
FIX = ROOT / "fixtures"
DIR = FIX / "raster" / "v1" / "reproject"
OUT = FIX / "commands" / "v1" / "cad.rasters.reproject.json"
WRITTEN = DIR / "out"
REGISTRY = {e["srid"]: e for e in json.loads((FIX / "crs" / "v1" / "registry.json").read_text("utf-8"))["systems"]}
PROJECT = 5256
SLIVER = 1e-6
PER_SIDE = 20
MOST_SIDE = 65536
MOST_CELLS = 1 << 30
NEAR_EDGE = 0.025


def place(srid, lon, lat):
    t, _, _ = ct.transformer(REGISTRY[4326], REGISTRY[srid])
    return t.transform(lon, lat)


def snap(v, step):
    return math.floor(v / step) * step


# ── The files ──────────────────────────────────────────────────────────────────────────────────────────────────

def u16_ramp(w, h, holes=()):
    a = np.array([[1000 + 7 * i + 13 * j for i in range(w)] for j in range(h)], dtype=np.float64)
    for (i, j) in holes:
        a[j, i] = 0
    return a


def files():
    """Each file: name, EPSG (None: no system), geotransform, data type, nodata, bands (arrays)."""
    x, y = place(32636, 35.62, 39.48)
    utm = [snap(x, 30), 30.0, 0.0, snap(y, 30), 0.0, -30.0]
    out = [("utm36.tif", 32636, utm, gdal.GDT_UInt16, 0.0, [u16_ramp(40, 30, holes=((3, 4), (20, 10), (39, 29)))])]
    dem = np.array([[900 + 2.5 * i - 1.75 * j + 0.01 * i * j for i in range(50)] for j in range(40)], dtype=np.float64)
    dem[7, 11] = -9999.0
    out.append(("cografi.tif", 4326, [35.61, 0.0003, 0.0, 39.495, 0.0, -0.0003], gdal.GDT_Float32, -9999.0, [dem]))
    x, y = place(5254, 35.63, 39.47)
    out.append(("tm30.tif", 5254, [snap(x, 20), 20.0, 0.0, snap(y, 20), 0.0, -20.0], gdal.GDT_UInt16, None,
                [u16_ramp(30, 20)]))
    x, y = place(23036, 35.62, 39.48)
    i16 = np.array([[-500 + 11 * i - 9 * j for i in range(30)] for j in range(30)], dtype=np.float64)
    out.append(("ed50.tif", 23036, [snap(x, 25), 25.0, 0.0, snap(y, 25), 0.0, -25.0], gdal.GDT_Int16, None, [i16]))
    cls = np.array([[[4, 5, 8, 9, 3, 6, 10, 0, 4, 1, 7, 11][(i + 3 * j) % 12] for i in range(30)] for j in range(20)],
                   dtype=np.float64)
    out.append(("siniflar.tif", 32636, [utm[0], 20.0, 0.0, utm[3], 0.0, -20.0], gdal.GDT_Byte, 0.0, [cls]))
    rgb = [np.array([[(i * 9 + j * 5 + 40 * b) % 256 for i in range(24)] for j in range(16)], dtype=np.float64)
           for b in range(3)]
    out.append(("renkli.tif", 32636, utm, gdal.GDT_Byte, None, rgb))
    out.append(("sistemsiz.tif", None, utm, gdal.GDT_UInt16, None, [u16_ramp(20, 15)]))
    return out


def write_files():
    DIR.mkdir(parents=True, exist_ok=True)
    for name, srid, gt, kind, nodata, bands in files():
        h, w = bands[0].shape
        ds = gdal.GetDriverByName("GTiff").Create(str(DIR / name), w, h, len(bands), kind,
                                                   options=["TILED=NO", "COMPRESS=DEFLATE"])
        ds.SetGeoTransform(gt)
        if srid:
            s = osr.SpatialReference()
            s.ImportFromEPSG(srid)
            ds.SetProjection(s.ExportToWkt())
        for k, a in enumerate(bands):
            b = ds.GetRasterBand(k + 1)
            if nodata is not None:
                b.SetNoDataValue(nodata)
            b.WriteArray(a)
        if len(bands) == 3:
            for k, ci in enumerate((gdal.GCI_RedBand, gdal.GCI_GreenBand, gdal.GCI_BlueBand)):
                ds.GetRasterBand(k + 1).SetColorInterpretation(ci)
        ds = None
    (DIR / "notlar.txt").write_text("Bu bir raster değil.\n", "utf-8")
    print(f"{DIR.relative_to(ROOT)}: dosyalar yazıldı.")


# ── The rule (ADR 0644 §3) ─────────────────────────────────────────────────────────────────────────────────────

class Source:
    def __init__(self, name, srid=None):
        ds = gdal.Open(str(DIR / name))
        self.name, self.w, self.h, self.gt = name, ds.RasterXSize, ds.RasterYSize, list(ds.GetGeoTransform())
        ref = ds.GetSpatialRef()
        self.declared = int(ref.GetAuthorityCode(None)) if ref is not None and ref.GetAuthorityCode(None) else None
        self.srid = srid if srid is not None else self.declared
        self.bands = [ds.GetRasterBand(k + 1).ReadAsArray().astype(np.float64) for k in range(ds.RasterCount)]
        self.nodata = ds.GetRasterBand(1).GetNoDataValue()
        self.kind = {gdal.GDT_UInt16: "u16", gdal.GDT_Float32: "f32", gdal.GDT_Int16: "i16", gdal.GDT_Byte: "u8"}[
            ds.GetRasterBand(1).DataType]

    def coords(self, u, v):
        x0, a, b, y0, c, d = self.gt
        return x0 + a * u + b * v, y0 + c * u + d * v

    def pixel(self, x, y):
        x0, a, b, y0, c, d = self.gt
        dx, dy = x - x0, y - y0
        det = a * d - b * c
        return (d * dx - b * dy) / det, (a * dy - c * dx) / det


def unofficial(src, dst):
    """An EPSG operation of ED50 stands in the way (its values are not the official ones, ADR 0167 §5)."""
    return "ED50" in (ct.datum(REGISTRY[src]), ct.datum(REGISTRY[dst]))


def way_of(src, dst):
    _, accuracy, via = ct.transformer(REGISTRY[src], REGISTRY[dst])
    words = via if via else "aynı datum"
    if accuracy:
        words += f", ±{accuracy:g} m"
    if unofficial(src, dst):
        words += ", resmî dönüşüm değil"
    return words


def border(w, h, n=PER_SIDE):
    out = []
    for k in range(n):
        t = k / n
        out += [(t * w, 0.0), (w, t * h), (w - t * w, h), (0.0, h - t * h)]
    return out


def sig6(v):
    return float(f"{v:.5e}")


def grid_of(s, cell):
    fwd, _, _ = ct.transformer(REGISTRY[s.srid], REGISTRY[PROJECT])
    if cell is None:
        cu, cv = s.w // 2, s.h // 2
        a = fwd.transform(*s.coords(cu, cv))
        b = fwd.transform(*s.coords(cu + 1, cv))
        c = fwd.transform(*s.coords(cu, cv + 1))
        cell = sig6(math.sqrt(abs((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]))))
    pts = [fwd.transform(*s.coords(u, v)) for (u, v) in border(s.w, s.h)]
    x0, x1 = min(p[0] for p in pts), max(p[0] for p in pts)
    y0, y1 = min(p[1] for p in pts), max(p[1] for p in pts)
    left = math.floor(x0 / cell + SLIVER) * cell
    top = math.ceil(y1 / cell - SLIVER) * cell
    w = max(1, math.ceil((x1 - left) / cell - SLIVER))
    h = max(1, math.ceil((top - y0) / cell - SLIVER))
    return [left, cell, 0.0, top, 0.0, -cell], w, h


def values(s, affine, w, h, nearest):
    """Each output band's values (NaN: no value) and the cells whose nearest source pixel is a near thing."""
    inv, _, _ = ct.transformer(REGISTRY[PROJECT], REGISTRY[s.srid])
    left, cell, _, top, _, _ = affine
    out = [np.full((h, w), np.nan) for _ in s.bands]
    unsure = np.zeros((h, w), dtype=bool)

    def get(b, i, j):
        v = s.bands[b][j, i]
        if s.nodata is not None and v == s.nodata:
            return math.nan
        return v

    for j in range(h):
        for i in range(w):
            x, y = left + (i + 0.5) * cell, top - (j + 0.5) * cell
            u, v = s.pixel(*inv.transform(x, y))
            if not (0.0 <= u < s.w and 0.0 <= v < s.h):
                if min(abs(u), abs(v), abs(u - s.w), abs(v - s.h)) < NEAR_EDGE:
                    unsure[j, i] = True
                continue
            if nearest:
                if min(u % 1, 1 - u % 1, v % 1, 1 - v % 1) < NEAR_EDGE:
                    unsure[j, i] = True
                for b in range(len(s.bands)):
                    out[b][j, i] = get(b, math.floor(u), math.floor(v))
                continue
            fx, fy = u - 0.5, v - 0.5
            i0, j0 = math.floor(fx), math.floor(fy)
            dx, dy = fx - i0, fy - j0
            ii = [min(max(i0, 0), s.w - 1), min(max(i0 + 1, 0), s.w - 1)]
            jj = [min(max(j0, 0), s.h - 1), min(max(j0 + 1, 0), s.h - 1)]
            taps = [(ii[0], jj[0], (1 - dx) * (1 - dy)), (ii[1], jj[0], dx * (1 - dy)),
                    (ii[0], jj[1], (1 - dx) * dy), (ii[1], jj[1], dx * dy)]
            for b in range(len(s.bands)):
                acc = wsum = 0.0
                for (p, q, wt) in taps:
                    g = get(b, p, q)
                    if wt > 0 and not math.isnan(g):
                        acc += wt * g
                        wsum += wt
                out[b][j, i] = acc / wsum if wsum > 1e-6 else math.nan
            # A tap of no value near: the weights' share is a near thing.
            if any(math.isnan(get(0, p, q)) for (p, q, _) in taps):
                unsure[j, i] = True
    return out, unsure


# ── The cases ──────────────────────────────────────────────────────────────────────────────────────────────────

def doc(srid=PROJECT, layers=None):
    def layer(lid, name, visible=True, locked=False, kind="layer"):
        return {"id": lid, "name": name, "type": kind, "visible": visible, "locked": locked, "expanded": True,
                "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}
    return {"format": "kentos.document", "version": 1, "name": "Yeniden izdüşüm",
            "settings": {"srid": srid, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                         "plotScale": 1000, "workspace": "gis"},
            "origin": {"x": 0.0, "y": 0.0},
            "layers": layers or [layer("altlik", "Altlık"), layer("kilitli", "Kilitli", locked=True),
                                 layer("gizli", "Gizli", visible=False), layer("grup", "Grup", kind="group")],
            "activeLayer": "altlik", "entities": [], "styles": {"items": [], "categories": []}}


def fix(name):
    return f"$fixture:raster/v1/reproject/{name}"


SAMPLES = {"u16": "u16", "f32": "f32", "i16": "i16", "u8": "u8"}
STYLE_GRAY = {"render": "gray", "bands": [1]}


def completed(name, file, tag, *, cell=None, sample="nearest", srid=None, layer=None, attributes=None,
              output=None, plan=False, warnings=()):
    s = Source(file, srid)
    affine, w, h = grid_of(s, cell)
    inp = {"file": fix(file), "output": output or f"$tmp:izdusum-{tag}.tif"}
    if cell is not None:
        inp["cell"] = cell
    if sample != "nearest":
        inp["sample"] = sample
    if srid is not None:
        inp["srid"] = srid
    if layer is not None:
        inp["layer"] = layer
    if attributes:
        inp["attributes"] = attributes
    out = {"uid": "$uidOf:1", "layer": layer or "$layer", "file": inp["output"], "from": s.srid, "to": PROJECT,
           "affine": affine, "width": w, "height": h, "bands": len(s.bands), "sample": SAMPLES[s.kind],
           "way": way_of(s.srid, PROJECT), "revision": "$current"}
    if plan:
        out["uid"] = None
    result = {"status": "completed", "output": out}
    said = list(warnings)
    if srid is not None and s.declared is not None and srid != s.declared:
        said.append(("crs_overridden", "srid"))
    if unofficial(s.srid, PROJECT):
        said.append(("crs_unofficial", "file"))
    if layer == "gizli":
        said.append(("layer_hidden", "layer"))
    result["warnings"] = [{"code": c, "message": "$any", "path": p} for c, p in said]
    step = {"op": "plan" if plan else "execute", "input": inp, "result": result, "within": 1e-6}
    if plan:
        step["expect"] = {"canUndo": False, "revision": "same"}
    else:
        step["expect"] = {"canUndo": True, "revision": "changed"}
    return {"name": name, "tag": tag, "setup": doc(), "steps": [step]}


def refused(name, inp, code, path, setup=None, message="$any"):
    return {"name": name, "setup": setup or doc(),
            "steps": [{"op": "execute", "input": inp,
                       "result": {"status": "failed", "error": {"code": code, "message": message, "path": path}},
                       "expect": {"canUndo": False, "revision": "same"}}]}


def cases():
    out = [
        completed("UTM 36'dan TM36'ya, en yakın komşu, piksel boyu kaynaktan", "utm36.tif", "utm36"),
        completed("UTM 36'dan TM36'ya, çift doğrusal, 25 m piksel: değersiz pikseller ağırlıkla dışarıda", "utm36.tif",
                  "utm36-cift", cell=25.0, sample="bilinear"),
        completed("Coğrafi DEM'den TM36'ya, çift doğrusal, 30 m", "cografi.tif", "cografi", cell=30.0, sample="bilinear"),
        completed("Coğrafi DEM'den, piksel boyu kaynaktan (altı anlamlı basamak)", "cografi.tif", "cografi-oz"),
        completed("TM30'dan TM36'ya: aynı datum, yalnız izdüşüm", "tm30.tif", "tm30", cell=20.0),
        completed("ED50 UTM 36'dan: datum EPSG:1783 + EPSG:5260 ile", "ed50.tif", "ed50", cell=25.0),
        completed("8 bitlik sınıf rasteri değerleriyle kalır (RGBA'ya çevrilmez)", "siniflar.tif", "siniflar", cell=20.0),
        completed("Üç bantlı renkli raster üç bantla kalır", "renkli.tif", "renkli", cell=30.0),
        completed("Sistem bildirmeyen dosya: sistemi srid ile verilir", "sistemsiz.tif", "sistemsiz", srid=32636,
                  cell=30.0),
        completed("Dosyanın bildirdiğinden başka srid: uyarıyla verilen sistemde sayılır", "tm30.tif", "tm30-baska",
                  srid=32636, cell=20.0),
        completed("Var olan katmana, öznitelikleriyle", "utm36.tif", "utm36-katman", cell=30.0, layer="altlik",
                  attributes={"stac.item": "S2A_36SVK", "kaynak": "uydu"}),
        completed("Plan: hiçbir şey yazılmaz", "utm36.tif", "utm36-plan", cell=30.0, plan=True),
    ]
    out.append(completed("Gizli katmana: uyarıyla", "utm36.tif", "utm36-gizli", cell=30.0, layer="gizli"))
    base = {"file": fix("utm36.tif"), "output": "$tmp:izdusum-ret.tif"}
    out += [
        refused("Dosya yok", {**base, "file": fix("yok.tif")}, "file_not_found", "file"),
        refused("Raster olmayan dosya", {**base, "file": fix("notlar.txt")}, "import_unreadable", "file"),
        refused("Sistemi bilinmeyen dosya, srid yok", {**base, "file": fix("sistemsiz.tif")}, "crs_unknown", "srid"),
        refused("KentOS'ta tanımlı olmayan srid", {**base, "srid": 99999}, "crs_unknown", "srid"),
        refused("Yanlış sistem: UTM koordinatları derece sayılınca projenin sistemine ulaşmaz", {**base, "srid": 4326},
                "crs_unreached", "srid"),
        refused("Kaynak zaten projenin sisteminde", base, "crs_mismatch", "file", setup=doc(32636)),
        refused("Yerel projede çevrilecek sistem yok", base, "crs_mismatch", "file", setup=doc(0)),
        refused("Piksel boyu sıfır", {**base, "cell": 0.0}, "invalid_input", "cell"),
        refused("Piksel boyu çok küçük: ızgara çok büyük", {**base, "cell": 0.001}, "invalid_input", "cell"),
        refused("Çıktı kaynağın kendisi", {**base, "output": fix("utm36.tif")}, "invalid_file_path", "output"),
        refused("Kilitli katman", {**base, "layer": "kilitli"}, "layer_locked", "layer"),
        refused("Olmayan katman", {**base, "layer": "yok"}, "layer_not_found", "layer"),
        refused("Grup katman değil", {**base, "layer": "grup"}, "not_a_layer", "layer"),
        refused("Beklenen sürüm geçersiz", {**base, "expectedRevision": "x"}, "invalid_revision", "expectedRevision"),
        {"name": "Beklenen sürüm tutmuyor: çakışma", "setup": doc(),
         "steps": [{"op": "execute", "input": {**base, "expectedRevision": "7"},
                    "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": "$any",
                                                                "path": "expectedRevision", "revision": "$current"}},
                    "expect": {"canUndo": False, "revision": "same"}}]},
        {"name": "Var olan dosya overwrite olmadan yazılmaz; overwrite ile yazılır", "setup": doc(),
         "steps": [{"op": "copyFile", "from": fix("tm30.tif"), "to": "$tmp:izdusum-var-olan.tif"},
                   {"op": "execute", "input": {**base, "output": "$tmp:izdusum-var-olan.tif"},
                    "result": {"status": "failed", "error": {"code": "file_exists", "message": "$any", "path": "output"}},
                    "expect": {"canUndo": False, "revision": "same"}},
                   {"op": "execute", "input": {**base, "output": "$tmp:izdusum-var-olan.tif", "overwrite": True},
                    "result": {"status": "completed", "output": "$any", "warnings": []},
                    "expect": {"canUndo": True, "revision": "changed"}}]},
    ]
    return out


def document():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.rasters.reproject", "commandVersion": 1,
            "title": "Rasteri yeniden izdüşür",
            "note": "Written by scripts/fixtures/raster_reproject_cases.py from docs/adr/0644 §3 with GDAL and PROJ; "
                    "do not edit by hand.",
            "setup": doc(), "cases": cases()}


def gdal_operation(src, dst):
    """The same PROJ path as crs_transform_cases.py's, in the systems' own axis order (GDAL's -ct takes it so)."""
    from pyproj import Transformer
    from pyproj.transformer import TransformerGroup
    a, b = ct.datum(REGISTRY[src]), ct.datum(REGISTRY[dst])
    if a == b:
        return Transformer.from_crs(f"EPSG:{src}", f"EPSG:{dst}")
    name = ct.PATHS[frozenset([a, b])][0]
    return next(t for t in TransformerGroup(f"EPSG:{src}", f"EPSG:{dst}").transformers if name in t.description)


def cross_check(case):
    """The grid against gdalwarp's own on the same lattice (same pixels within the near-edge share)."""
    step = case["steps"][0]
    inp, o = step["input"], step["result"]["output"]
    s = Source(inp["file"].split("/")[-1], inp.get("srid"))
    t = gdal_operation(s.srid, PROJECT)
    left, cell, _, top, _, _ = o["affine"]
    with tempfile.TemporaryDirectory() as d:
        dst = Path(d) / "w.tif"
        cmd = ["gdalwarp", "-q", "-of", "GTiff", "-ct", t.to_proj4(),
               "-s_srs", f"EPSG:{s.srid}", "-t_srs", f"EPSG:{PROJECT}",
               "-te", repr(left), repr(top - o["height"] * cell), repr(left + o["width"] * cell), repr(top),
               "-ts", str(o["width"]), str(o["height"]), "-r", "near" if inp.get("sample") != "bilinear" else "bilinear",
               "-dstnodata", "nan" if s.kind == "f32" else str(int(s.nodata) if s.nodata is not None else 0),
               str(DIR / s.name), str(dst)]
        subprocess.run(cmd, check=True)
        wd = gdal.Open(str(dst))
        g = wd.GetRasterBand(1).ReadAsArray().astype(np.float64)
        wd = None
    ours, unsure = values(s, o["affine"], o["width"], o["height"], inp.get("sample") != "bilinear")
    a = ours[0]
    if s.nodata is not None:
        g = np.where(g == s.nodata, np.nan, g)
    both = ~np.isnan(a) & ~np.isnan(g) & ~unsure
    if inp.get("sample") == "bilinear":
        return float(np.max(np.abs(a[both] - g[both]))) if both.any() else 0.0
    return float(np.mean(a[both] != g[both])) if both.any() else 0.0


def check_written():
    bad = []
    for c in cases():
        if "tag" not in c or c["steps"][0]["op"] == "plan":
            continue
        path = WRITTEN / c["steps"][0]["input"]["output"].split(":", 1)[1]
        if not path.exists():
            bad.append(f"{path.name} yok: KENTOS_WRITE_REPROJECT=1 ile yazın")
            continue
        step = c["steps"][0]
        inp, o = step["input"], step["result"]["output"]
        ds = gdal.Open(str(path))
        if (ds.RasterXSize, ds.RasterYSize, ds.RasterCount) != (o["width"], o["height"], o["bands"]):
            bad.append(f"{path.name}: boy {ds.RasterXSize}×{ds.RasterYSize}×{ds.RasterCount}")
            continue
        if max(abs(p - q) for p, q in zip(ds.GetGeoTransform(), o["affine"])) > 1e-6:
            bad.append(f"{path.name}: yer {ds.GetGeoTransform()} ≠ {o['affine']}")
        ref = ds.GetSpatialRef()
        if ref is None or ref.GetAuthorityCode(None) != str(PROJECT):
            bad.append(f"{path.name}: sistem EPSG:{PROJECT} değil")
        s = Source(inp["file"].split("/")[-1], inp.get("srid"))
        nearest = inp.get("sample") != "bilinear"
        want, unsure = values(s, o["affine"], o["width"], o["height"], nearest)
        nd = ds.GetRasterBand(1).GetNoDataValue()
        for b in range(ds.RasterCount):
            got = ds.GetRasterBand(b + 1).ReadAsArray().astype(np.float64)
            if nd is not None and not math.isnan(nd):
                got = np.where(got == nd, np.nan, got)
            w = want[b]
            if nd is None and s.kind != "f32":
                # No nodata of the source's: the outside is 0, as gdalwarp writes it.
                w = np.where(np.isnan(w), 0.0, w)
            if s.kind != "f32" and not nearest:
                w = np.where(np.isnan(w), np.nan, np.round(w))
            sure = ~unsure
            empty = np.isnan(w) != np.isnan(got)
            if (empty & sure).any():
                k = np.argwhere(empty & sure)[0]
                bad.append(f"{path.name}: bant {b + 1} piksel {k[::-1].tolist()} değerli/değersiz farklı")
                continue
            both = ~np.isnan(w) & ~np.isnan(got) & sure
            diff = np.abs(w[both] - got[both])
            # Bilinear: the inverse within 0.02 pixel times the steepest step (13 a pixel), and the type's rounding.
            most = 0.0 if nearest else (0.02 * 2 * 13 + (1.0 if s.kind != "f32" else 1e-3))
            if diff.size and diff.max() > most:
                bad.append(f"{path.name}: bant {b + 1} en büyük fark {diff.max():g} > {most:g}")
    return bad


def main():
    args = set(sys.argv[1:])
    if "--files" in args:
        write_files()
        return
    if "--written" in args:
        bad = check_written()
        print("\n".join(bad) if bad else "KentOS'un yazdığı rasterler kuralla aynı.")
        sys.exit(1 if bad else 0)
    doc_ = document()
    for c in doc_["cases"]:
        c.pop("tag", None)
    text = json.dumps(doc_, ensure_ascii=False, indent=1) + "\n"
    worst = []
    for c in cases():
        if "tag" in c and c["steps"][0]["op"] == "execute":
            worst.append((c["tag"], cross_check(c)))
    gdal_words = ", ".join(f"{t} {v:.3g}" for t, v in worst)
    if "--check" in args:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.name} güncel değil")
            sys.exit(1)
        print(f"{OUT.name}: {len(doc_['cases'])} durum; güncel. gdalwarp'la fark (en yakında oran, çift doğrusalda en büyük): "
              f"{gdal_words}")
        return
    OUT.write_text(text, "utf-8")
    print(f"{OUT.relative_to(ROOT)}: {len(doc_['cases'])} durum yazıldı. gdalwarp'la fark: {gdal_words}")


if __name__ == "__main__":
    main()
