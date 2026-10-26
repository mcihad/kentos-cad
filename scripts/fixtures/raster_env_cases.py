#!/usr/bin/env python3
"""Analiz ortamının ortak durumları (docs/adr/0643): alan, hücre boyu, hizalama rasteri ve NoData; blok düzeninden bağımsızlık.

    /usr/bin/python3 scripts/fixtures/raster_env_cases.py           # rasterleri, çizimi ve durumları yazar
    /usr/bin/python3 scripts/fixtures/raster_env_cases.py --check   # hiçbir şey yazmaz; karşılaştırır

KentOS kodu kullanılmaz. Aynı yükseklik modeli GDAL'la dört blok düzeninde yazılır (16 ve 32'lik karolar, birer ve yedişer
satırlık şeritler); bir durum hangi dosyadan okunursa okunsun sonucun değerleri aynıdır, karonun kenarındaki hücrenin
komşuları öbür karodan gelir. Beklenen ızgaralar ADR 0643 §1'in kuralıyla (alan: kaynağın dokunulan hücreleri; hücre boyu ve
hizalama: hizalama çizgileri üstünde alanı örten bütün hücreler), değerler bağımsız başvurularla:

- Raster hesaplayıcı ve Yeniden sınıflandır: hücrenin ortasının düştüğü kaynak hücresi (raster_ops_cases.py'nin `Raster`'ı);
- Komşuluk istatistiği: bütün rasterin 3 × 3 ortalaması, alanın hücrelerinde (raster_ops_cases.py'nin `focal`'ı, kesirlerle);
- Eğim: bütün rasterin Horn eğimi, alanın hücrelerinde (terrain_cases.py'nin `slope_of`'u);
- Çukur doldur: alanın hücrelerinden oluşan yükseklik modelinin doldurulması (hydrology_cases.py'nin `fill`'i): küresel araç
  ortamın alanıyla sınırlıdır.

Ortamı almayan araçlar ve okunamayan ortam söyleyerek reddedilir.
"""

import json
import math
import sys
from fractions import Fraction
from pathlib import Path

import numpy as np
from osgeo import gdal

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import raster_ops_cases as rops  # noqa: E402  (Raster, focal, stored)
import terrain_cases as tc  # noqa: E402  (Horn's slope)
import hydrology_cases as hc  # noqa: E402  (the fill)

DIR = ROOT / "fixtures/raster-env/v1"
CASES = DIR / "cases.json"
COMMANDS = ROOT / "fixtures/commands/v1/cad.processing.run.environment.json"
DRAWING = DIR / "drawing.kcad"
EDGE = 1e-9
SRID = 5254

# The DEM: 40 × 30 cells of 10 m, a pit and four holes.
DEM_AFFINE = [500000.0, 10.0, 0.0, 4420300.0, 0.0, -10.0]
DEM_W, DEM_H = 40, 30
HOLES = {(5, 3), (17, 11), (18, 11), (33, 25)}
PIT = {(20, 14): -9.0, (21, 14): -7.5, (20, 15): -8.0, (21, 15): -6.0, (22, 15): -3.0}


def dem_values():
    out = []
    for j in range(DEM_H):
        for i in range(DEM_W):
            if (i, j) in HOLES:
                out.append(math.nan)
                continue
            v = 100.0 + 13.0 * math.sin(i * 0.31) + 7.0 * math.cos(j * 0.27) + 0.05 * i * j + PIT.get((i, j), 0.0)
            out.append(rops.f32(v))
    return out


# A second raster on other lines: 25 × 20 cells of 10 m, offset by 5 m.
FARK_AFFINE = [500005.0, 10.0, 0.0, 4420295.0, 0.0, -10.0]
FARK_W, FARK_H = 25, 20


def fark_values():
    return [rops.f32(2.0 + math.sin(i * 0.5) * math.cos(j * 0.4)) for j in range(FARK_H) for i in range(FARK_W)]


# The snap raster: 4 m cells on the lines x ≡ 1, y ≡ 3 (mod 4).
HIZA_AFFINE = [500001.0, 4.0, 0.0, 4420299.0, 0.0, -4.0]
HIZA_W, HIZA_H = 6, 6

# The same DEM in four block layouts.
LAYOUTS = [
    ("dem-k16.tif", "D16", ["TILED=YES", "BLOCKXSIZE=16", "BLOCKYSIZE=16", "COMPRESS=DEFLATE"]),
    ("dem-k32.tif", "D32", ["TILED=YES", "BLOCKXSIZE=32", "BLOCKYSIZE=32"]),
    ("dem-s1.tif", "S1", ["BLOCKYSIZE=1"]),
    ("dem-s7.tif", "S7", ["BLOCKYSIZE=7", "COMPRESS=LZW", "PREDICTOR=3"]),
]
OTHERS = [
    ("fark.tif", "Fark", FARK_AFFINE, FARK_W, FARK_H, fark_values),
    ("hiza.tif", "Hiza", HIZA_AFFINE, HIZA_W, HIZA_H, lambda: [1.0] * (HIZA_W * HIZA_H)),
]

# Entity ids: the rasters first, in this order, then the area polygon and a line.
IDS = {name: k + 1 for k, (_, name, _) in enumerate(LAYOUTS)}
IDS.update({name: len(LAYOUTS) + k + 1 for k, (_, name, *_r) in enumerate(OTHERS)})
POLYGON_ID = len(IDS) + 1
LINE_ID = POLYGON_ID + 1
POLYGON = [(500083.0, 4420079.0), (500171.0, 4420079.0), (500171.0, 4420181.0), (500083.0, 4420181.0)]

# The area most cases take: it crosses the 16 and 32 cell tiles and the 7 row strips.
AREA = [500095.0, 4420105.0, 500342.5, 4420262.0]


def dem():
    return rops.Raster(DEM_AFFINE, DEM_W, DEM_H, dem_values())


def fark():
    return rops.Raster(FARK_AFFINE, FARK_W, FARK_H, fark_values())


def write_tif(path, affine, w, h, values, options):
    ds = gdal.GetDriverByName("GTiff").Create(str(path), w, h, 1, gdal.GDT_Float32, options)
    ds.SetGeoTransform(affine)
    band = ds.GetRasterBand(1)
    band.SetNoDataValue(float("nan"))
    band.WriteArray(np.array(values, dtype=np.float64).reshape(h, w))
    ds.FlushCache()
    ds = None


def read_tif(path):
    ds = gdal.Open(str(path))
    return list(ds.GetGeoTransform()), ds.GetRasterBand(1).ReadAsArray().astype(np.float64).reshape(-1).tolist()


# ── ADR 0643 §1's grids ──────────────────────────────────────────────────

def window(affine, w, h, area):
    """The base's cells the area touches, on its own lattice: (affine, width, height)."""
    x0, a, _, y0, _, d = affine
    us = [(x - x0) / a for x in (area[0], area[2])]
    vs = [(y - y0) / d for y in (area[1], area[3])]
    i0 = max(math.floor(min(us) + EDGE), 0)
    j0 = max(math.floor(min(vs) + EDGE), 0)
    i1 = min(math.ceil(max(us) - EDGE), w)
    j1 = min(math.ceil(max(vs) - EDGE), h)
    assert i1 > i0 and j1 > j0
    return [x0 + a * i0, a, 0.0, y0 + d * j0, 0.0, d], i1 - i0, j1 - j0, (i0, j0)


def lattice(affine, w, h, area, cell, snap):
    """A new lattice: `cell` (else the snap's, else the base's) on the snap's lines (else the base's origin), over the area
    cut to the base's extent (else the extent), whole cells."""
    x0, a, _, y0, _, d = affine
    extent = [x0, y0 + d * h, x0 + a * w, y0]
    if area is not None:
        extent = [max(area[0], extent[0]), max(area[1], extent[1]), min(area[2], extent[2]), min(area[3], extent[3])]
    size = cell if cell is not None else (snap[1] if snap is not None else a)
    ox, oy = (snap[0], snap[3]) if snap is not None else (x0, y0)
    left = ox + math.floor((extent[0] - ox) / size + EDGE) * size
    right = ox + math.ceil((extent[2] - ox) / size - EDGE) * size
    top = oy + math.ceil((extent[3] - oy) / size - EDGE) * size
    bottom = oy + math.floor((extent[1] - oy) / size + EDGE) * size
    return [left, size, 0.0, top, 0.0, -size], round((right - left) / size), round((top - bottom) / size)


def cells(affine, w, h):
    for j in range(h):
        for i in range(w):
            yield rops.centre(affine, i, j)


def stored32(v):
    return math.nan if v is None or math.isnan(v) else rops.f32(v)


def written(values, nodata):
    """The file's samples: NaN as the environment's NoData when it gives one."""
    return [None if math.isnan(v) and nodata is None else (nodata if math.isnan(v) else v) for v in values]


# ── The cases ────────────────────────────────────────────────────────────

def raster_case(id, title, tool, inputs, values, env, grid, out, *, rule="exact", nodata=None, same=None):
    affine, w, h = grid
    c = {"id": id, "title": title, "tool": tool, "selection": [IDS[n] for n in inputs], "values": values,
         "environment": env, "expect": {"grid": {"affine": affine, "width": w, "height": h},
                                        "values": written(out, nodata), "rule": rule}}
    if same:
        c["sameAs"] = same
    return c


def refused(id, title, tool, inputs, values, env, text):
    return {"id": id, "title": title, "tool": tool, "selection": [IDS[n] for n in inputs], "values": values,
            "environment": env, "expect": {"refused": text}}


def calc_on(grid, expr):
    """The calculator's result on `grid`: each input at the cell's centre."""
    affine, w, h = grid
    r, f = dem(), fark()
    out = []
    for x, y in cells(affine, w, h):
        v = expr(r.at_point(x, y, 0), f.at_point(x, y, 0))
        out.append(stored32(v))
    return out


def double(z, _):
    return math.nan if z is None or math.isnan(z) else 2.0 * z


def plus(z, f):
    if z is None or f is None or math.isnan(z) or math.isnan(f):
        return math.nan
    return z + f


def cut(values, w, offset, ww, hh):
    i0, j0 = offset
    return [values[(j0 + j) * w + (i0 + i)] for j in range(hh) for i in range(ww)]


def calc(name):
    return {"input": {"scope": "selection"}, "expression": f"[{name}] * 2"}


def focal_values():
    whole = rops.focal(dem(), 1, "mean", True, rops.window("rect", 3, 3))["raster"]["values"]
    return [math.nan if v is None else v for v in whole]


def slope_values():
    arr = np.array(dem_values(), dtype=np.float64).reshape(DEM_H, DEM_W)
    s = tc.slope_of({"array": arr, "gt": DEM_AFFINE, "geographic": False, "nodata": None}, "horn", False, 1.0)
    return [math.nan if math.isnan(v) else tc.f32(v) for v in s.reshape(-1).tolist()]


def fill_values(offset, ww, hh):
    sub = cut(dem_values(), DEM_W, offset, ww, hh)
    r = rops.Raster(window(DEM_AFFINE, DEM_W, DEM_H, AREA)[0], ww, hh, sub)
    return [stored32(v) for v in hc.fill(hc.Grid(r), 0.0)]


def reclass_values(grid):
    affine, w, h = grid
    r = dem()
    out = []
    for x, y in cells(affine, w, h):
        z = r.at_point(x, y, 0)
        out.append(math.nan if z is None or math.isnan(z) else (1.0 if z <= 100.0 else 2.0))
    return out


def cases():
    whole = (DEM_AFFINE, DEM_W, DEM_H)
    wa, ww, wh, off = window(DEM_AFFINE, DEM_W, DEM_H, AREA)
    win = (wa, ww, wh)
    poly_box = [POLYGON[0][0], POLYGON[0][1], POLYGON[2][0], POLYGON[2][1]]
    pa, pw, ph, _ = window(DEM_AFFINE, DEM_W, DEM_H, poly_box)
    out = []
    # Raster hesaplayıcı: the four layouts give the same file.
    for k, (_, name, _) in enumerate(LAYOUTS):
        out.append(raster_case(f"hesap-{name.lower()}", f"Raster hesaplayıcı, ortamsız: {name} düzenindeki dosyadan", "raster.calculator",
                               [name], calc(name), None, whole, calc_on(whole, double),
                               same=None if k == 0 else "hesap-d16"))
    for k, (_, name, _) in enumerate(LAYOUTS):
        out.append(raster_case(f"hesap-alan-{name.lower()}", f"Raster hesaplayıcı, analiz alanı karoların ve şeritlerin sınırını keser: {name}",
                               "raster.calculator", [name], calc(name), {"area": AREA}, win, calc_on(win, double),
                               same=None if k == 0 else "hesap-alan-d16"))
    out.append(raster_case("hesap-alan-nesne", "Raster hesaplayıcı, analiz alanı bir alan nesnesinin kutusu", "raster.calculator",
                           ["D16"], calc("D16"), {"areaObjects": [f"#{POLYGON_ID}"]}, (pa, pw, ph),
                           calc_on((pa, pw, ph), double)))
    g = lattice(DEM_AFFINE, DEM_W, DEM_H, AREA, 5.0, None)
    out.append(raster_case("hesap-hucre-5", "Raster hesaplayıcı, alan ve 5 m'lik hücre: kaynağın çizgileri üstünde yeni ızgara",
                           "raster.calculator", ["S7"], calc("S7"), {"area": AREA, "cell": 5.0}, g, calc_on(g, double)))
    g = lattice(DEM_AFFINE, DEM_W, DEM_H, None, None, HIZA_AFFINE)
    out.append(raster_case("hesap-hiza", "Raster hesaplayıcı, hizalama rasteri (4 m, kayık çizgiler): bütün raster", "raster.calculator",
                           ["D32"], calc("D32"), {"snap": f"#{IDS['Hiza']}"}, g, calc_on(g, double)))
    g = lattice(DEM_AFFINE, DEM_W, DEM_H, AREA, 25.0, HIZA_AFFINE)
    out.append(raster_case("hesap-hiza-25", "Raster hesaplayıcı, 25 m'lik hücre hizalama rasterinin çizgilerinde, alanla", "raster.calculator",
                           ["S1"], calc("S1"), {"area": AREA, "cell": 25.0, "snap": f"#{IDS['Hiza']}"}, g, calc_on(g, double)))
    out.append(raster_case("hesap-nodata", "Raster hesaplayıcı, NoData −9999: boşluklar −9999 yazılır", "raster.calculator", ["D16"],
                           calc("D16"), {"nodata": -9999.0}, whole, calc_on(whole, double), nodata=-9999.0))
    fark_grid = window(DEM_AFFINE, DEM_W, DEM_H, AREA)
    out.append(raster_case("hesap-iki-raster", "Raster hesaplayıcı, iki raster alanla: ilk adlandırılanın ızgarasında, öbürü ortadan",
                           "raster.calculator", ["D16", "Fark"], {"input": {"scope": "selection"}, "expression": "[D16] + [Fark]"},
                           {"area": AREA}, fark_grid[:3], calc_on(fark_grid[:3], plus)))
    # Komşuluk istatistiği: the window's cells take their neighbours from the source, across tiles and strips.
    focal = focal_values()
    fv = {"input": {"scope": "selection"}, "band": 1, "shape": "rect", "width": 3, "height": 3, "stat": "mean", "ignore": True}
    for k, (_, name, _) in enumerate(LAYOUTS):
        out.append(raster_case(f"komsu-alan-{name.lower()}", f"Komşuluk istatistiği 3 × 3 ortalama, alanla: {name}; komşular kaynaktan",
                               "raster.focalStatistics", [name], fv, {"area": AREA}, win, cut(focal, DEM_W, off, ww, wh),
                               rule="f32", same=None if k == 0 else "komsu-alan-d16"))
    # Eğim: the window's values are the whole raster's.
    slope = slope_values()
    sv = {"input": {"scope": "selection"}, "band": 1, "method": "horn", "unit": "degrees", "zFactor": 1, "area": "raster"}
    out.append(raster_case("egim-alan-d16", "Eğim, ortamın alanıyla: bütün rasterin eğimi alanın hücrelerinde", "surface.slope", ["D16"], sv,
                           {"area": AREA}, win, cut(slope, DEM_W, off, ww, wh), rule="f32"))
    out.append(raster_case("egim-alan-s1", "Eğim, ortamın alanıyla, birer satırlık şeritlerden: aynı dosya", "surface.slope", ["S1"], sv,
                           {"area": AREA}, win, cut(slope, DEM_W, off, ww, wh), rule="f32", same="egim-alan-d16"))
    out.append(raster_case("egim-nodata", "Eğim, NoData −1: değersiz hücreler −1", "surface.slope", ["D32"], sv, {"nodata": -1.0}, whole,
                           slope, rule="f32", nodata=-1.0))
    # Yeniden sınıflandır with the area and NoData.
    rv = {"input": {"scope": "selection"}, "band": 1, "table": "* 100 1; 100 * 2", "bounds": "upperClosed", "unmatched": "keep",
          "sample": "f32"}
    out.append(raster_case("sinif-alan-nodata", "Yeniden sınıflandır, alan ve NoData −1", "raster.reclassify", ["S7"], rv,
                           {"area": AREA, "nodata": -1.0}, win, reclass_values(win), nodata=-1.0))
    # Çukur doldur: a global tool works on the area's cells only.
    hv = {"input": {"scope": "selection"}, "band": 1, "slope": 0, "result": "filled"}
    fill = fill_values(off, ww, wh)
    out.append(raster_case("doldur-alan-k16", "Çukur doldur, alanla: yalnız alanın hücreleri, kenarı çıkış", "hydrology.fill", ["D16"], hv,
                           {"area": AREA}, win, fill, rule="f32"))
    out.append(raster_case("doldur-alan-s7", "Çukur doldur, alanla, yedişer satırlık şeritlerden: aynı dosya", "hydrology.fill", ["S7"], hv,
                           {"area": AREA}, win, fill, rule="f32", same="doldur-alan-k16"))
    # The refusals.
    out.append(refused("ret-komsu-hucre", "Komşuluk istatistiği hücre boyunu almaz", "raster.focalStatistics", ["D16"], fv, {"cell": 5.0},
                       "rasterin kendi hücreleriyle çalışır"))
    out.append(refused("ret-egim-hiza", "Eğim hizalama rasterini almaz", "surface.slope", ["D16"], sv, {"snap": f"#{IDS['Hiza']}"},
                       "Yüzey analizi yükseklik modelinin kendi hücreleriyle çalışır"))
    out.append(refused("ret-ornekle", "Yeniden örnekle ortamın alanını almaz", "raster.resample", ["D16"],
                       {"input": {"scope": "selection"}, "cell": 20, "method": "nearest"}, {"area": AREA}, "analiz ortamının alanını"))
    out.append(refused("ret-alan-disarida", "Analiz alanı rasterin dışında", "raster.calculator", ["D16"], calc("D16"),
                       {"area": [400000.0, 4400000.0, 400010.0, 4400010.0]}, "Analiz alanı rasterle kesişmiyor"))
    out.append(refused("ret-hucre-sifir", "Hücre boyu sıfır", "raster.calculator", ["D16"], calc("D16"), {"cell": 0.0},
                       "sıfırdan büyük bir sayı olmalı"))
    out.append(refused("ret-hiza-raster-degil", "Hizalama rasteri bir çizgi", "raster.calculator", ["D16"], calc("D16"),
                       {"snap": f"#{LINE_ID}"}, "bir raster değil"))
    out.append(refused("ret-alan-ters", "Analiz alanının köşeleri ters", "raster.calculator", ["D16"], calc("D16"),
                       {"area": [500300.0, 4420100.0, 500100.0, 4420200.0]}, "dört sayı olmalı"))
    return out


# ── The command's cases (cad.processing.run's `environment`): validated, nothing written ──

def command_cases():
    def step(input, result):
        return {"op": "validate", "input": input, "result": result, "expect": {"canUndo": False, "revision": "same"}}

    def ok(input):
        return step(input, {"status": "completed", "output": None, "warnings": []})

    def bad(input, code, message, path):
        return step(input, {"status": "failed", "error": {"code": code, "message": message, "path": path}})

    calc_in = {"tool": "raster.calculator", "parameters": {"input": {"scope": "selection"}, "expression": "[D16] * 2"},
               "selection": [f"$uidOf:{IDS['D16']}"]}
    fill_in = {"tool": "hydrology.fill", "parameters": {"input": {"scope": "selection"}}, "selection": [f"$uidOf:{IDS['S7']}"]}
    with_env = lambda base, env: {**base, "environment": env}
    nil = "00000000-0000-4000-8000-000000000000"
    cases = [
        ("ortam yok: raster aracı her zamanki gibi", ok(calc_in)),
        ("boş ortam yok sayılır, ortamı almayan araçta da",
         ok({"tool": "attributes.calculate", "parameters": {"field": "Alan", "value": "1"}, "selection": [f"$uidOf:{POLYGON_ID}"],
             "environment": {}})),
        ("alan, hücre boyu, hizalama ve NoData birlikte", ok(with_env(calc_in, {"area": AREA, "cell": 5.0, "snap": f"$uidOf:{IDS['Hiza']}",
                                                                                "nodata": -9999.0}))),
        ("alan bir nesnenin kutusu", ok(with_env(calc_in, {"areaObjects": [f"$uidOf:{POLYGON_ID}"]}))),
        ("alan hem kutu hem nesne: ikisinin kutusu", ok(with_env(calc_in, {"area": AREA, "areaObjects": [f"$uidOf:{POLYGON_ID}"]}))),
        ("küresel araç alanı alır", ok(with_env(fill_in, {"area": AREA}))),
        ("yalnız NoData", ok(with_env(fill_in, {"nodata": -1.0}))),
        ("ortamı almayan araç: söylenir",
         bad({"tool": "attributes.calculate", "parameters": {"field": "Alan", "value": "1"}, "selection": [f"$uidOf:{POLYGON_ID}"],
              "environment": {"cell": 5.0}}, "invalid_parameters",
             "Öznitelik hesapla analiz ortamını almaz: ortam yalnız raster yazan çözümleme araçlarındadır.", "environment")),
        ("ortamı almayan araç, yalnız NoData'yla da",
         bad({"tool": "attributes.calculate", "parameters": {"field": "Alan", "value": "1"}, "selection": [f"$uidOf:{POLYGON_ID}"],
              "environment": {"nodata": 0.0}}, "invalid_parameters",
             "Öznitelik hesapla analiz ortamını almaz: ortam yalnız raster yazan çözümleme araçlarındadır.", "environment")),
        ("alanın köşeleri ters", bad(with_env(calc_in, {"area": [500300.0, 4420100.0, 500100.0, 4420200.0]}), "invalid_parameters",
                                    "Analiz alanı en küçük doğu, kuzey, en büyük doğu, kuzey sırasıyla dört sayı olmalı.",
                                    "environment.area")),
        ("hücre boyu sıfır", bad(with_env(calc_in, {"cell": 0.0}), "invalid_parameters",
                                 "Ortamın hücre boyu sıfırdan büyük bir sayı olmalı.", "environment.cell")),
        ("hücre boyu eksi", bad(with_env(calc_in, {"cell": -2.5}), "invalid_parameters",
                                "Ortamın hücre boyu sıfırdan büyük bir sayı olmalı.", "environment.cell")),
        ("hizalama rasteri bir çizgi", bad(with_env(calc_in, {"snap": f"$uidOf:{LINE_ID}"}), "invalid_parameters", "$any",
                                           "environment.snap")),
        ("hizalama rasteri bir alan", bad(with_env(calc_in, {"snap": f"$uidOf:{POLYGON_ID}"}), "invalid_parameters", "$any",
                                          "environment.snap")),
        ("hizalama rasterinin kimliği UUID değil", bad(with_env(calc_in, {"snap": "hiza"}), "invalid_uid",
                                                       "“hiza” bir nesne kimliği değil (UUID olmalı).", "environment.snap")),
        ("hizalama rasteri çizimde yok", bad(with_env(calc_in, {"snap": nil}), "entity_not_found",
                                             f"{nil} kimlikli nesne çizimde yok.", "environment.snap")),
        ("alanın nesnesinin kimliği UUID değil", bad(with_env(calc_in, {"areaObjects": ["alan"]}), "invalid_uid",
                                                     "“alan” bir nesne kimliği değil (UUID olmalı).", "environment.areaObjects[0]")),
        ("alanın ikinci nesnesi çizimde yok", bad(with_env(calc_in, {"areaObjects": [f"$uidOf:{POLYGON_ID}", nil]}), "entity_not_found",
                                                  f"{nil} kimlikli nesne çizimde yok.", "environment.areaObjects[1]")),
        ("aracın kendi değerinin sorunu ortamdan önce söylenir",
         bad(with_env({"tool": "raster.calculator", "parameters": {"input": {"scope": "selection"}, "expression": "[D16] *"},
                       "selection": [f"$uidOf:{IDS['D16']}"]}, {"cell": 0.0}), "invalid_parameters", "$any", "parameters.expression")),
        ("bilinmeyen araç ortamdan önce söylenir", bad({"tool": "raster.yok", "environment": {"cell": 0.0}}, "unknown_tool",
                                                       "“raster.yok” bir işlem aracı değil. Araçların listesi cad.processing.tools'ta.", "tool")),
    ]
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.processing.run", "commandVersion": 1,
            "title": "İşlem aracını çalıştır: analiz ortamı",
            "note": "Üretici: scripts/fixtures/raster_env_cases.py (docs/adr/0643). Ortamın değerleri ve retleri; değerler ve "
                    "ızgaralar fixtures/raster-env/v1'de.",
            "setup": drawing(), "cases": [{"name": f"Analiz ortamı: {n}", "steps": [st]} for n, st in cases]}


def drawing():
    def layer(id, name):
        return {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
                "style": {"color": "#8D6E63", "lineType": "continuous", "lineWeight": 0.25}, "children": []}

    look = {"render": "ramp", "bands": [1], "stretch": "minMax", "ramp": "Arazi"}
    entities, layers = [], []
    for file, name, _ in LAYOUTS:
        layers.append(layer(name.lower(), name))
        entities.append({"kind": "raster", "id": len(entities) + 1, "layerId": name.lower(), "attrs": {}, "affine": DEM_AFFINE,
                         "width": DEM_W, "height": DEM_H, "bands": 1, "sample": "f32", "file": file, "srid": SRID, "style": look})
    for file, name, affine, w, h, _ in OTHERS:
        layers.append(layer(name.lower(), name))
        entities.append({"kind": "raster", "id": len(entities) + 1, "layerId": name.lower(), "attrs": {}, "affine": affine,
                         "width": w, "height": h, "bands": 1, "sample": "f32", "file": file, "srid": SRID, "style": look})
    layers.insert(0, layer("alan", "Alan"))
    entities.append({"kind": "polygon", "id": POLYGON_ID, "layerId": "alan", "attrs": {},
                     "pts": [{"x": x, "y": y} for x, y in POLYGON]})
    entities.append({"kind": "line", "id": LINE_ID, "layerId": "alan", "attrs": {},
                     "a": {"x": 500010.0, "y": 4420010.0}, "b": {"x": 500050.0, "y": 4420050.0}})
    return {
        "format": "kentos.document", "version": 1, "name": "Analiz ortamının durumları",
        "settings": {"srid": SRID, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                     "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": 500000, "y": 4420000}, "layers": layers, "activeLayer": "alan", "entities": entities,
        "styles": {"items": [], "categories": []},
    }


def files():
    out = {file: (DEM_AFFINE, DEM_W, DEM_H, dem_values(), options) for file, _, options in LAYOUTS}
    for file, _, affine, w, h, values in OTHERS:
        out[file] = (affine, w, h, values(), [])
    return out


def build():
    return {"format": "kentos.raster-environment-cases", "version": 1, "document": "drawing.kcad",
            "rasters": {f: f for f in files()}, "cases": cases()}


def main():
    check = "--check" in sys.argv[1:]
    problems = []
    DIR.mkdir(parents=True, exist_ok=True)
    for file, (affine, w, h, values, options) in files().items():
        path = DIR / file
        if not check:
            write_tif(path, affine, w, h, values, options)
        if not path.exists():
            problems.append(f"{path} yok")
            continue
        got_affine, got = read_tif(path)
        if got_affine != affine or any(not ((math.isnan(a) and math.isnan(b)) or a == b) for a, b in zip(got, values)):
            problems.append(f"{file} başvurunun girdisi değil")
    texts = {DRAWING: json.dumps(drawing(), ensure_ascii=False, indent=1) + "\n",
             CASES: json.dumps(build(), ensure_ascii=False, indent=1) + "\n",
             COMMANDS: json.dumps(command_cases(), ensure_ascii=False, indent=1) + "\n"}
    for path, text in texts.items():
        if check:
            if not path.exists() or path.read_text("utf-8") != text:
                problems.append(f"{path.name} güncel değil")
        else:
            path.write_text(text, "utf-8")
    if problems:
        print("\n".join(problems) + f"\nYeniden yazın: /usr/bin/python3 {sys.argv[0]}; farkı okuyun.")
        sys.exit(1)
    n = len(cases())
    print(f"{CASES.relative_to(ROOT)}: {n} durum, {len(files())} raster" + ("; güncel." if check else " yazıldı."))


if __name__ == "__main__":
    main()
