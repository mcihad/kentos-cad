#!/usr/bin/env python3
"""COG olarak kaydet'in bağımsız denetimi (docs/adr/0515), KentOS kodu olmadan.

    python3 scripts/fixtures/cog_cases.py --files   # kaynak rasterleri GDAL'la ve çizimi yazar (fixtures/cog/v1)
    python3 scripts/fixtures/cog_cases.py --check   # KentOS'un yazdığı COG'ları denetler

KentOS tarafı `KENTOS_WRITE_COG=1 cargo test -p kentos-native-application --test all cog` ile çizimin her rasterini
`cad.processing.run`'ın `raster.cog`'uyla (başsız sunucunun dosyalarıyla) yazar ve fixtures/cog/v1/out'a koyar.
Bu betik onları GDAL'ın kendi COG denetleyicisiyle (validate_cloud_optimized_geotiff.py, hata ve uyarı olmadan) ve
numpy'la denetler:

- 0. kat kaynağın değerleri bit bit, bantlar, örnek türü, değeri olmayan hücre, konum (çizimin affine'i) ve EPSG;
- önizleme katlarının boyları ⌈w / 2ᵏ⌉ × ⌈h / 2ᵏ⌉ (ikisi 256'ya sığana dek) ve değerleri ADR 0231 §2'nin kuralıyla:
  her kat bir üstündekinin yazılmış değerlerinden, 2 × 2 hücrenin ortalaması (değersizler ve NaN dışarıda; hiç
  değeri yoksa değersiz), toplam üst satırın iki hücresi sonra alt satırınkiler sırasıyla, tam sayıda yarımlar
  sıfırdan uzağa yuvarlanıp sınırlanır, float32'de float32'ye çevrilir;
- karolar 256 × 256 ve Deflate; GDAL'ın IMAGE_STRUCTURE'ında LAYOUT=COG.
"""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path

import numpy as np
from osgeo import gdal, osr

gdal.UseExceptions()

from osgeo_utils.samples import validate_cloud_optimized_geotiff as cogval  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/cog/v1"
SRC = DIR / "src"
OUT = DIR / "out"
EPSG = 5256

# Each raster: its file, layer, the drawing's affine, and how GDAL writes it.
RASTERS = [
    {
        "layer": "kot",
        "name": "Kot",
        "file": "kot.tif",
        "affine": [487000.0, 0.5, 0.0, 4421000.0, 0.0, -0.5],
        "size": (520, 300),
        "bands": 1,
        "type": gdal.GDT_Float32,
        "sample": "f32",
        "nodata": -9999.0,
        "options": [],  # şeritli, sıkıştırmasız
    },
    {
        "layer": "sinif",
        "name": "Sınıf",
        "file": "sinif.tif",
        "affine": [487100.0, 0.8, 0.3, 4421000.0, 0.3, -0.8],
        "size": (600, 400),
        "bands": 1,
        "type": gdal.GDT_UInt16,
        "sample": "u16",
        "nodata": 0.0,
        "options": ["TILED=YES", "COMPRESS=DEFLATE"],
        "overviews": True,
    },
    {
        "layer": "orto",
        "name": "Orto",
        "file": "orto.png",
        "affine": [487300.0, 0.25, 0.0, 4421000.0, 0.0, -0.25],
        "size": (400, 300),
        "bands": 3,
        "type": gdal.GDT_Byte,
        "sample": "u8",
        "nodata": None,
        "png": True,
    },
    {
        "layer": "renkli",
        "name": "Saydam",
        "file": "saydam.tif",
        "affine": [487400.0, 1.0, 0.0, 4421000.0, 0.0, -1.0],
        "size": (300, 260),
        "bands": 4,
        "type": gdal.GDT_Byte,
        "sample": "u8",
        "nodata": None,
        "options": ["TILED=YES", "COMPRESS=LZW", "PHOTOMETRIC=RGB", "ALPHA=YES"],
    },
]


def values(r: dict) -> np.ndarray:
    """The raster's samples, bands × rows × columns: smooth fields, a hole where it has nodata."""
    w, h = r["size"]
    y, x = np.mgrid[0:h, 0:w].astype(np.float64)
    out = []
    for b in range(r["bands"]):
        f = 500 + 120 * np.sin(x / 37.0 + b) * np.cos(y / 23.0 - b) + 0.07 * x - 0.11 * y
        if r["sample"] == "u8":
            f = (f - f.min()) / (f.max() - f.min()) * 250
            if b == 3:  # alpha: a transparent band and a half-transparent corner
                f = np.where(x < 40, 0, np.where(y < 30, 128, 255))
            f = np.floor(f)
        elif r["sample"] == "u16":
            f = np.floor(f * 10) % 4000 + 1
        else:
            f = np.round(f, 2)
        out.append(f)
    a = np.array(out)
    if r["nodata"] is not None:
        hole = (x - w * 0.6) ** 2 + (y - h * 0.4) ** 2 < (h * 0.15) ** 2
        a[:, hole] = r["nodata"]
    return a


def write_files() -> None:
    SRC.mkdir(parents=True, exist_ok=True)
    srs = osr.SpatialReference()
    srs.ImportFromEPSG(EPSG)
    mem = gdal.GetDriverByName("MEM")
    for r in RASTERS:
        w, h = r["size"]
        a = values(r)
        ds = mem.Create("", w, h, r["bands"], r["type"])
        x0, pa, pb, y0, pc, pd = r["affine"]
        ds.SetGeoTransform([x0, pa, pb, y0, pc, pd])
        ds.SetProjection(srs.ExportToWkt())
        for b in range(r["bands"]):
            band = ds.GetRasterBand(b + 1)
            band.WriteArray(a[b])
            if r["nodata"] is not None:
                band.SetNoDataValue(r["nodata"])
        if r.get("png"):
            gdal.GetDriverByName("PNG").CreateCopy(str(SRC / r["file"]), ds, options=["WORLDFILE=YES"])
            aux = SRC / (r["file"] + ".aux.xml")
            if aux.exists():
                aux.unlink()
        else:
            out = gdal.GetDriverByName("GTiff").CreateCopy(str(SRC / r["file"]), ds, options=r["options"])
            if r.get("overviews"):
                out.BuildOverviews("AVERAGE", [2, 4])
            out = None
    layers = [
        {
            "id": r["layer"], "name": r["name"], "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "#8D6E63", "lineType": "continuous", "lineWeight": 0.25}, "children": [],
        }
        for r in RASTERS
    ]
    entities = []
    for r in RASTERS:
        w, h = r["size"]
        bands = list(range(1, min(r["bands"], 3) + 1))
        style = {"render": "rgb", "bands": bands, "stretch": "none"} if r["bands"] >= 3 else {
            "render": "ramp", "bands": [1], "stretch": "minMax", "ramp": "Arazi"}
        if r["nodata"] is not None:
            style["nodata"] = r["nodata"]
        entities.append({
            "kind": "raster", "affine": r["affine"], "width": w, "height": h, "bands": r["bands"],
            "sample": r["sample"], "file": r["file"], "srid": EPSG, "style": style,
            "id": len(entities) + 1, "layerId": r["layer"], "attrs": {},
        })
    doc = {
        "format": "kentos.document", "version": 1, "name": "COG olarak kaydet",
        "settings": {"srid": EPSG, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": 487000, "y": 4421000}, "layers": layers, "activeLayer": RASTERS[0]["layer"],
        "entities": entities, "styles": {"items": [], "categories": []},
    }
    (DIR / "cog.kcad").write_text(json.dumps(doc, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print(f"{len(RASTERS)} raster ve cog.kcad yazıldı: {DIR}")


def stored(sample: str, v: float) -> float:
    if sample == "f32":
        return float(np.float32(v))
    lo, hi = {"u8": (0, 255), "u16": (0, 65535)}[sample]
    r = -math.floor(-v + 0.5) if v < 0 else math.floor(v + 0.5)
    return float(min(max(r, lo), hi))


def halve(level: np.ndarray, sample: str, nodata: float | None) -> np.ndarray:
    """The next level: bands × ⌈h/2⌉ × ⌈w/2⌉, ADR 0231 §2's mean, summed in the writer's order."""
    b, h, w = level.shape
    nh, nw = (h + 1) // 2, (w + 1) // 2
    fill = nodata if nodata is not None else (math.nan if sample == "f32" else 0.0)
    out = np.empty((b, nh, nw), dtype=np.float64)
    lv = level.tolist()
    for k in range(b):
        for j in range(nh):
            for i in range(nw):
                s, n = 0.0, 0
                for (yy, xx) in ((2 * j, 2 * i), (2 * j, 2 * i + 1), (2 * j + 1, 2 * i), (2 * j + 1, 2 * i + 1)):
                    if yy >= h or xx >= w:
                        continue
                    v = lv[k][yy][xx]
                    if math.isnan(v) or (nodata is not None and v == nodata):
                        continue
                    s += v
                    n += 1
                out[k, j, i] = stored(sample, s / n) if n else stored(sample, fill) if sample != "f32" else fill
    return out


failures: list[str] = []


def check(ok: bool, what: str) -> None:
    if not ok:
        failures.append(what)


def same(a: np.ndarray, b: np.ndarray) -> bool:
    return a.shape == b.shape and bool(np.all((a == b) | (np.isnan(a) & np.isnan(b))))


def check_files() -> dict:
    summary = {}
    for r in RASTERS:
        stem = Path(r["file"]).stem
        path = OUT / f"{stem}-cog.tif"
        if not path.exists():
            failures.append(f"{path.name} yok: önce KENTOS_WRITE_COG=1 ile KentOS'un testini çalıştırın")
            continue
        warnings, errors, _ = cogval.validate(str(path), full_check=True)
        check(not errors, f"{path.name}: COG değil: {errors}")
        check(not warnings, f"{path.name}: COG uyarıları: {warnings}")
        ds = gdal.Open(str(path))
        check(ds.GetMetadataItem("LAYOUT", "IMAGE_STRUCTURE") == "COG", f"{path.name}: GDAL LAYOUT=COG demiyor")
        check(ds.GetMetadataItem("COMPRESSION", "IMAGE_STRUCTURE") == "DEFLATE", f"{path.name}: Deflate değil")
        w, h = r["size"]
        check((ds.RasterXSize, ds.RasterYSize, ds.RasterCount) == (w, h, r["bands"]),
              f"{path.name}: boyu ya da bant sayısı farklı")
        gt = list(ds.GetGeoTransform())
        x0, pa, pb, y0, pc, pd = r["affine"]
        check(gt == [x0, pa, pb, y0, pc, pd], f"{path.name}: konumu {gt}, beklenen {r['affine']}")
        srs = ds.GetSpatialRef()
        check(srs is not None and srs.GetAuthorityCode(None) == str(EPSG), f"{path.name}: EPSG:{EPSG} değil")
        want = values(r)
        if r["sample"] == "f32":
            want = want.astype(np.float32).astype(np.float64)
        got = np.array([ds.GetRasterBand(b + 1).ReadAsArray().astype(np.float64) for b in range(r["bands"])])
        check(same(got, want), f"{path.name}: 0. katın değerleri kaynağınkinden farklı")
        nd = ds.GetRasterBand(1).GetNoDataValue()
        check(nd == r["nodata"], f"{path.name}: değeri olmayan hücre {nd}, beklenen {r['nodata']}")
        check(ds.GetRasterBand(1).GetBlockSize() == [256, 256], f"{path.name}: karo 256 × 256 değil")
        # The overviews: their sizes and values from the level above, as written.
        level = got
        sizes = []
        (lw, lh) = (w, h)
        while lw > 256 or lh > 256:
            lw, lh = (lw + 1) // 2, (lh + 1) // 2
            sizes.append((lw, lh))
        band = ds.GetRasterBand(1)
        check(band.GetOverviewCount() == len(sizes),
              f"{path.name}: {band.GetOverviewCount()} önizleme katı, beklenen {len(sizes)}")
        for k, (ow, oh) in enumerate(sizes):
            if k >= band.GetOverviewCount():
                break
            ov = [ds.GetRasterBand(b + 1).GetOverview(k) for b in range(r["bands"])]
            check((ov[0].XSize, ov[0].YSize) == (ow, oh), f"{path.name}: {k + 1}. katın boyu farklı")
            want_level = halve(level, r["sample"], r["nodata"])
            got_level = np.array([o.ReadAsArray().astype(np.float64) for o in ov])
            check(same(got_level, want_level), f"{path.name}: {k + 1}. önizleme katının değerleri kurala uymuyor")
            level = got_level
        summary[path.name] = {"size": [w, h], "bands": r["bands"], "overviews": sizes,
                              "bytes": path.stat().st_size}
    return summary


def main() -> None:
    if "--files" in sys.argv:
        write_files()
        return
    summary = check_files()
    expected = DIR / "expected.json"
    if "--check" in sys.argv and expected.exists():
        old = json.loads(expected.read_text(encoding="utf-8"))
        check({k: {**v, "bytes": None} for k, v in old.items()} ==
              {k: {**v, "bytes": None} for k, v in json.loads(json.dumps(summary)).items()},
              "expected.json güncel değil")
    elif not failures:
        expected.write_text(json.dumps(summary, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    if failures:
        print("\n".join(failures))
        sys.exit(1)
    print(f"{len(summary)} COG GDAL'ın denetleyicisinden ve numpy'ın kurallarından geçti.")


if __name__ == "__main__":
    main()
