#!/usr/bin/env python3
"""Çok boyutlu veri'nin İşlemler durumları (docs/adr/0243 §8–§10; biçim fixtures/processing/README.md).

    python3 scripts/fixtures/multidim_processing_cases.py           # dosyaları, çizimi ve durumları yazar
    python3 scripts/fixtures/multidim_processing_cases.py --check   # hiçbir şey yazmaz; karşılaştırır

KentOS kodu kullanılmaz. NetCDF dosyaları (fixtures/processing/v1/multidim/*.nc) bağımsız başvurunun
(scripts/fixtures/multidim_cases.py, fixtures/multidim/v1) dosyalarının bayt bayt kopyasıdır; iki küçük GeoTIFF
(üç bantlı ve tek bantlı) GDAL'la yazılır. Çizim (fixtures/processing/v1/multidim.kcad) onları katman katman bağlı
raster olarak, Kesit'in çizgilerini ve Zaman serisi'nin noktalarını taşır. Durumların tabloları, özetleri ve
yazılan dosyaları başvurunun aynı adlı durumlarındandır (Mesh hesaplayıcı'nın dosyası başvurunun UGRID yazıcısının
yazdığıyla bayt bayt aynı: `multidimOf`); sayılar gösterim kuralıyla (scripts/fixtures/numeric_display.py).
"""

import json
import math
import shutil
import sys
from pathlib import Path

import numpy as np
from osgeo import gdal

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import numeric_display as nd  # noqa: E402
import raster_ops_processing_cases as rop  # noqa: E402

gdal.UseExceptions()

DIR = ROOT / "fixtures/processing/v1"
FILES = DIR / "multidim"
DRAWING = DIR / "multidim.kcad"
CASES = DIR / "multidim.json"
SOURCE = ROOT / "fixtures/multidim/v1"
REF = json.loads((SOURCE / "cases.json").read_text("utf-8"))
words = rop.words
layer_id = rop.layer_id
MESH_AFFINE = [500000.0, 0.125, 0.0, 4420040.0, 0.0, -0.125]
MESH_SIZE = (320, 320)


def ref(name):
    return next(c for c in REF["cases"] if c["name"] == name)


def fixed3(v):
    return "" if v is None else nd.shown(v, 3)


# ── The files ────────────────────────────────────────────────────────────

COPIES = ["grid-cdf2.nc", "ugrid.nc", "ugrid-calc.nc"]
# Bantlar: three bands of 4 × 3 cells of 10 m, one value empty (nodata −9999); Tek: one band.
BANDS_AFFINE = [500100.0, 10.0, 0.0, 4420030.0, 0.0, -10.0]


def bands_values(b, j, i):
    return -9999.0 if (b, j, i) == (1, 1, 2) else 10.0 * (b + 1) + j * 4 + i + 0.5


def write_tiffs():
    for name, bands in (("bantlar.tif", 3), ("tek.tif", 1)):
        ds = gdal.GetDriverByName("GTiff").Create(str(FILES / name), 4, 3, bands, gdal.GDT_Float32)
        ds.SetGeoTransform(BANDS_AFFINE)
        for b in range(bands):
            band = ds.GetRasterBand(b + 1)
            band.SetNoDataValue(-9999.0)
            band.WriteArray(np.array([[bands_values(b, j, i) for i in range(4)] for j in range(3)], dtype=np.float32))
        ds.FlushCache()
        ds = None


def tiff_ok(name, bands):
    ds = gdal.Open(str(FILES / name))
    if ds is None or ds.RasterCount != bands or list(ds.GetGeoTransform()) != BANDS_AFFINE:
        return False
    for b in range(bands):
        arr = ds.GetRasterBand(b + 1).ReadAsArray().astype(np.float64)
        want = np.array([[bands_values(b, j, i) for i in range(4)] for j in range(3)], dtype=np.float32).astype(np.float64)
        if not np.array_equal(arr, want):
            return False
    return True


# ── The drawing ──────────────────────────────────────────────────────────

def info(file):
    return ref(f"info {file}")["expect"]


DEM = next(g for g in info("grid-cdf2.nc")["grids"] if g["variable"] == "dem")
MESH = info("ugrid.nc")["meshes"][0]
CALC_MESH_TIMES = ref("series ugrid.nc depth [0]")["expect"]["times"]


def dims_of(dims, index):
    return [{"name": d["name"], "index": index, "values": d["values"], **({"time": True} if d["time"] else {}),
             **({"units": d["units"]} if d.get("units") else {})} for d in dims]


RAMP = {"render": "ramp", "bands": [1], "stretch": "minMax", "ramp": "Spektral"}
MESH_RAMP = {"render": "ramp", "bands": [1], "stretch": "minMax", "ramp": "Viridis"}
GRAY = {"render": "gray", "bands": [1], "stretch": "minMax", "nodata": -9999}
RGB = {"render": "rgb", "bands": [3, 2, 1], "stretch": "minMax", "nodata": -9999}

RASTERS = [
    # Layer id, name, the object's fields.
    ("dem", "DEM", {"affine": DEM["affine"], "width": DEM["width"], "height": DEM["height"], "bands": 1, "sample": "f64",
                    "file": "grid-cdf2.nc", "style": RAMP, "dataset": {"variable": "dem", "dims": dims_of(DEM["dims"], 0)}}),
    ("derinlik", "Derinlik", {"affine": MESH_AFFINE, "width": MESH_SIZE[0], "height": MESH_SIZE[1], "bands": 1, "sample": "f32",
                              "file": "ugrid.nc", "style": MESH_RAMP,
                              "dataset": {"variable": "depth", "mesh": "mesh", "dims": dims_of(MESH["datasets"][0]["dims"], 1)}}),
    ("hesap", "Hesap", {"affine": MESH_AFFINE, "width": MESH_SIZE[0], "height": MESH_SIZE[1], "bands": 1, "sample": "f32",
                        "file": "ugrid-calc.nc", "style": MESH_RAMP,
                        "dataset": {"variable": "depth", "mesh": "mesh",
                                    "dims": [{"name": "time", "index": 0, "values": CALC_MESH_TIMES, "time": True}]}}),
    ("bantlar", "Bantlar", {"affine": BANDS_AFFINE, "width": 4, "height": 3, "bands": 3, "sample": "f32", "file": "bantlar.tif",
                            "style": RGB}),
    ("tek", "Tek bant", {"affine": BANDS_AFFINE, "width": 4, "height": 3, "bands": 1, "sample": "f32", "file": "tek.tif",
                         "style": GRAY}),
]

PROFILE_DEM = ref("profile grid-cdf2.nc dem [0]")
PROFILE_MESH = ref("profile ugrid.nc depth [1]")
SERIES_DEM = ref("series grid-cdf2.nc dem [1]")
SERIES_MESH = ref("series ugrid.nc depth [0]")
BAND_POINTS = [("B1", [(500105.0, 4420025.0)]), ("B2", [(500125.0, 4420015.0), (500135.0, 4420005.0)]), (None, [(500095.0, 4420025.0)])]


def polyline(pts):
    return {"kind": "polyline", "pts": [{"x": x, "y": y} for x, y in pts]}


def point(name, pts):
    e = {"kind": "point", "p": {"x": pts[0][0], "y": pts[0][1]}}
    if len(pts) > 1:
        e["parts"] = [{"p": {"x": x, "y": y}} for x, y in pts[1:]]
    return e, ({"ad": name} if name else {})


OBJECTS = [
    ("cizgiler", "Çizgiler", [(polyline(l), {}) for l in PROFILE_DEM["lines"]], "#37474F"),
    ("mesh-cizgiler", "Mesh çizgileri", [(polyline(l), {}) for l in PROFILE_MESH["lines"]], "#37474F"),
    ("noktalar", "Noktalar", [point(n, [(x, y)]) for n, x, y in SERIES_DEM["points"]], "#C62828"),
    ("mesh-noktalar", "Mesh noktaları", [point(n, [(x, y)]) for n, x, y in SERIES_MESH["points"]], "#C62828"),
    ("bant-noktalar", "Bant noktaları", [point(n, pts) for n, pts in BAND_POINTS], "#C62828"),
]


def raster_id(lid):
    return 1 + [r[0] for r in RASTERS].index(lid)


def drawing():
    def layer(id, name, color):
        return {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
                "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25}, "children": []}

    entities = []

    def add(e, lid, attrs=None):
        entities.append({**e, "id": len(entities) + 1, "layerId": lid, "attrs": attrs or {}})

    for lid, _, fields in RASTERS:
        add({"kind": "raster", **fields, "srid": 5254}, lid)
    for lid, _, objs, _ in OBJECTS:
        for e, attrs in objs:
            add(e, lid, attrs)
    layers = [layer(lid, name, color) for lid, name, _, color in OBJECTS]
    layers += [layer(lid, name, "#8D6E63") for lid, name, _ in RASTERS]
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "İşlem durumları: çok boyutlu veri",
        "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                     "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": 500000, "y": 4420000},
        "layers": layers,
        "activeLayer": "cizgiler",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }


# ── The cases ────────────────────────────────────────────────────────────

def on(lid):
    return {"scope": "layer", "layerId": lid}


def new_layer(name, color="#7A6B5B", weight=0.25):
    return {"id": layer_id(name), "name": name, "style": {"color": color, "lineType": "continuous", "lineWeight": weight}}


def profile_table(e):
    rows = [[str(st[0]), fixed3(st[1]), fixed3(st[2]), fixed3(st[3]), fixed3(v)] for st, v in zip(e["stations"], e["values"])]
    return {"columns": ["Çizgi", "Uzaklık (m)", "Y", "X", "Değer"], "rows": rows}


def pieces(e, layer):
    out, run, line = [], [], None

    def close():
        if len(run) >= 2:
            out.append({"kind": "polyline", "layerId": layer, "attrs": {"Çizgi": str(line)},
                        "pts": [{"x": x, "y": y} for x, y, _ in run], "zs": [v for _, _, v in run]})
        run.clear()

    for st, v in zip(e["stations"], e["values"]):
        if st[0] != line:
            close()
            line = st[0]
        if v is None:
            close()
        else:
            run.append((st[2], st[3], v))
    close()
    return out


def profile_case(id, title, case, lid, lines, step, draw):
    e = case["expect"]
    expect = {"status": "ok", "summary": e["summary"], "outputs": {"table": profile_table(e)}}
    values = {"input": on(lid), "lines": on(lines), "step": step, "draw": draw}
    if draw:
        layer = new_layer("Kesit", "#B4572E", 0.35)
        expect.update({"undo": "Kesit", "layers": [layer], "added": pieces(e, layer["id"]), "layerAbove": {layer["id"]: lid}})
        values["layer"] = {"newName": "Kesit"}
    else:
        expect["undo"] = None
    return {"id": id, "title": title, "document": "multidim.kcad", "selection": [], "run": {"tool": "multidim.profile"},
            "values": values, "expect": expect}


def series_table(e):
    rows = [[str(k + 1), e["labels"][k]] + [fixed3(v) for v in row] for k, row in enumerate(e["values"])]
    return {"columns": ["Adım", "Zaman"] + e["names"], "rows": rows}


def series_case(id, title, case, lid, points):
    e = case["expect"]
    return {"id": id, "title": title, "document": "multidim.kcad", "selection": [], "run": {"tool": "multidim.series"},
            "values": {"input": on(lid), "points": on(points)},
            "expect": {"status": "ok", "undo": None, "summary": e["summary"], "outputs": {"table": series_table(e)}}}


def bands_case():
    """Zaman serisi over Bantlar's bands: every point of a multi-point object, the unnamed point's column by its place."""
    flat = [(name, x, y) for name, pts in BAND_POINTS for x, y in pts]
    names, seen = [], []
    for k, (name, _, _) in enumerate(flat):
        base = name if name else f"Nokta {k + 1}"
        cand, i = base, 2
        while cand in seen:
            cand = f"{base} ({i})"
            i += 1
        seen.append(cand)
        names.append(cand)
    x0, a, _, y0, _, d = BANDS_AFFINE
    rows = []
    for b in range(3):
        row = [str(b + 1)]
        for _, x, y in flat:
            i, j = math.floor((x - x0) / a), math.floor((y - y0) / d)
            v = bands_values(b, j, i) if 0 <= i < 4 and 0 <= j < 3 else None
            row.append(fixed3(None if v == -9999.0 else (float(np.float32(v)) if v is not None else None)))
        rows.append(row)
    return {"id": "series-bands", "title": "Zaman serisi, zamansız üç bantlı raster: her bant bir satır; çok noktalı nesnenin her "
                                           "noktası, adsız nokta sırasıyla, rasterin dışındaki nokta boş",
            "document": "multidim.kcad", "selection": [], "run": {"tool": "multidim.series"},
            "values": {"input": on("bantlar"), "points": on("bant-noktalar")},
            "expect": {"status": "ok", "undo": None, "summary": f"{len(flat)} nokta, 3 adım.",
                       "outputs": {"table": {"columns": ["Bant"] + names, "rows": rows}}}}


def calc_case(id, title, refname, output, file):
    c = ref(refname)
    e, spec = c["expect"], c["spec"]
    path = output or "ugrid-calc-hesap.nc"
    layer = new_layer("Mesh hesabı")
    dims = [] if spec["summary"] != "none" else [{"name": "time", "index": 0, "values": CALC_MESH_TIMES, "time": True}]
    dataset = {"variable": e["variable"], "mesh": "Mesh2d"}
    if dims:
        dataset["dims"] = dims
    style = {**MESH_RAMP, "stretch": "minMax"}
    added = {"kind": "raster", "layerId": layer["id"], "attrs": {}, "affine": MESH_AFFINE, "width": MESH_SIZE[0],
             "height": MESH_SIZE[1], "bands": 1, "sample": "f32", "file": path, "srid": 5254, "style": style, "dataset": dataset}
    return {"id": id, "title": title, "document": "multidim.kcad", "selection": [], "run": {"tool": "multidim.meshCalculator"},
            "values": {"input": on("hesap"), "expression": spec["expression"], "summary": spec["summary"], "name": spec["name"],
                       "output": output, "add": True, "layer": {"newName": "Mesh hesabı"}},
            "expect": {"status": "ok", "undo": "Mesh hesaplayıcı", "summary": f"{e['summary']} “{path}” yazıldı.",
                       "outputs": {"file": path}, "layers": [layer], "added": [added], "layerAbove": {layer["id"]: "hesap"},
                       "multidimOf": {path: file}}}


def refused(id, title, tool, values, message):
    return {"id": id, "title": title, "document": "multidim.kcad", "selection": [], "run": {"tool": tool}, "values": values,
            "expect": {"status": "error", "message": message}}


def cases():
    calc_error = ref("calc refuses 'depth + level' named 'x'")["expect"]["error"]
    return [
        profile_case("profile-dem", "Kesit, NetCDF ızgarası: iki çizgi, 7 m adım; değersiz hücreler ve rasterin dışı boş, kotlu "
                                    "çizgiler orada bölünür", PROFILE_DEM, "dem", "cizgiler", 7.0, True),
        profile_case("profile-mesh", "Kesit, mesh: ağdan enterpolasyon, etkin olmayan yüz ve değersiz düğüm boş", PROFILE_MESH,
                     "derinlik", "mesh-cizgiler", 3.0, False),
        series_case("series-dem", "Zaman serisi, NetCDF ızgarası: iki zaman adımı, üç nokta", SERIES_DEM, "dem", "noktalar"),
        series_case("series-mesh", "Zaman serisi, mesh: üç adım, etkin olmayan yüz ve ağın dışındaki nokta boş", SERIES_MESH,
                    "derinlik", "mesh-noktalar"),
        bands_case(),
        calc_case("calc-max", "Mesh hesaplayıcı, En büyük: zamanlı ve zamansız veri seti, kaynağın yanına", "calc [Su derinliği] + bed max",
                  "", "calc-2.nc"),
        calc_case("calc-steps", "Mesh hesaplayıcı, zaman özeti yok: her adım yazılır, adı verilen dosyaya", "calc depth * 2 + 1 none",
                  "derinlik.nc", "calc-1.nc"),
        refused("calc-places", "Mesh hesaplayıcı, düğüm ve yüz veri setleri birlikte: ret", "multidim.meshCalculator",
                {"input": on("hesap"), "expression": "depth + level", "summary": "none", "name": "x"}, calc_error),
        refused("series-one-band", "Zaman serisi, tek bantlı ve zamansız raster: ret", "multidim.series",
                {"input": on("tek"), "points": on("bant-noktalar")},
                "Raster tek bantlı ve zaman boyutu yok: Zaman serisi zaman boyutlu veri seti ya da çok bantlı raster ister."),
    ]


def defaults():
    one = {"scope": "selection"}
    return {
        "multidim.profile": {"input": one, "lines": one, "step": None, "band": 1, "draw": False, "layer": {"newName": "Kesit"}},
        "multidim.series": {"input": one, "points": one},
        "multidim.meshCalculator": {"input": one, "expression": "", "summary": "none", "name": "Hesap", "output": "", "add": True,
                                    "layer": {"newName": "Mesh hesabı"}},
    }


def build():
    return {
        "format": "kentos.processing-cases",
        "version": 1,
        "tolerance": 1e-9,
        "rasters": {**{f: f"multidim/{f}" for f in COPIES}, "bantlar.tif": "multidim/bantlar.tif", "tek.tif": "multidim/tek.tif"},
        "documents": {
            "multidim.kcad": {
                "defaults": {"lengthDecimals": 3, "areaDecimals": 2, "angleUnit": "grad", "plotScale": 1000, "drawingFont": "barlow",
                             "activeLayer": "cizgiler", "measureHeightMm": 2},
                "tools": defaults(),
            }
        },
        "cases": cases(),
    }


def main():
    check = "--check" in sys.argv[1:]
    problems = []
    FILES.mkdir(parents=True, exist_ok=True)
    for f in COPIES:
        src, dst = SOURCE / "files" / f, FILES / f
        if not check:
            shutil.copyfile(src, dst)
        if not dst.exists() or dst.read_bytes() != src.read_bytes():
            problems.append(f"{dst.relative_to(ROOT)} başvurunun dosyası değil")
    if not check:
        write_tiffs()
    for name, bands in (("bantlar.tif", 3), ("tek.tif", 1)):
        if not (FILES / name).exists() or not tiff_ok(name, bands):
            problems.append(f"{(FILES / name).relative_to(ROOT)} beklenen raster değil")
    texts = {
        DRAWING: json.dumps(drawing(), ensure_ascii=False, indent=1) + "\n",
        CASES: json.dumps(build(), ensure_ascii=False, indent=1) + "\n",
    }
    for path, text in texts.items():
        if check:
            if not path.exists() or path.read_text("utf-8") != text:
                problems.append(f"{path.relative_to(ROOT)} yeniden kurulanla aynı değil (betiği --check olmadan çalıştırın)")
        else:
            path.write_text(text, "utf-8")
    if problems:
        print("\n".join(problems), file=sys.stderr)
        sys.exit(1)
    print(f"{CASES.relative_to(ROOT)}: {len(build()['cases'])} durum; çizim {DRAWING.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
