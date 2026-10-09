#!/usr/bin/env python3
"""Raster işlemleri ve Raster istatistiği'nin İşlemler durumları (docs/adr/0233 §14; biçim fixtures/processing/README.md).

    python3 scripts/fixtures/raster_ops_processing_cases.py           # rasterleri, çizimi ve durumları yazar
    python3 scripts/fixtures/raster_ops_processing_cases.py --check   # hiçbir şey yazmaz; karşılaştırır

KentOS kodu kullanılmaz. Rasterler (fixtures/processing/v1/raster-ops/*.tif, GDAL'la) raster işlemlerinin bağımsız
başvurusunun (fixtures/raster-ops/v1/cases.json, scripts/fixtures/raster_ops_cases.py) girdileridir; çizim
(fixtures/processing/v1/raster-ops.kcad) onları katman katman bağlı raster olarak, maskeyi ve bölgeleri taşır.
Durumlar araçların kuralından yazılır: sonucun adı (ilk rasterin adı ve aracın eki), özet (sayılar ADR 0149'un
gösterim kuralıyla, scripts/fixtures/numeric_display.py), yeni katmanın adı ve stili, raster nesnesinin alanları,
bölgelerin yazılan alanı ve tablosu, histogramın tablosu. Yazılan dosyanın değerleri başvurunun adı verilen
durumununkilerdir (`rasterOpsOf`: dosyanın 0. katı, durumun kuralıyla); yeni katman ilk rasterin katmanının hemen
üstündedir (`layerAbove`). Bölgelerin ve histogramın sayıları başvurunun kendi işlevleriyle hesaplanır.
"""

import json
import math
import re
import sys
import unicodedata
from fractions import Fraction
from pathlib import Path

import numpy as np
from osgeo import gdal

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
from numeric_display import shown  # noqa: E402  (the display rule's own reference)
import raster_ops_cases as ops  # noqa: E402  (the independent reference's rules)

gdal.UseExceptions()

DIR = ROOT / "fixtures/processing/v1"
RASTERS = DIR / "raster-ops"
DRAWING = DIR / "raster-ops.kcad"
CASES = DIR / "raster-ops.json"
REF = json.loads((ROOT / "fixtures/raster-ops/v1/cases.json").read_text("utf-8"))
RASTER_LAYER = "#7A6B5B"


def ref(name):
    return next(c for c in REF["cases"] if c["name"] == name)


def words(n):
    return re.sub(r"\B(?=(\d{3})+(?!\d))", ".", str(n))


def layer_id(name):
    folded = name.translate(str.maketrans("çğıöşüÇĞİÖŞÜ", "cgiosuCGIOSU"))
    folded = unicodedata.normalize("NFKD", folded).encode("ascii", "ignore").decode()
    return "islem-" + re.sub(r"[^a-z0-9]+", "-", folded.lower())


# ── The rasters: the reference's inputs, each a file ─────────────────────

# File → (reference case, input index), and the layer it is drawn on (the panel's order, top first).
FILES = [
    ("kot.tif", "hesap-iki-raster", 0, "kot", "Kot"),
    ("fark.tif", "hesap-iki-raster", 1, "fark", "Fark"),
    ("egim.tif", "sinif-ust-kapali", 0, "egim", "Eğim"),
    ("buyuk.tif", "kirp-kare", 0, "buyuk", "Büyük"),
    ("pafta1.tif", "mozaik-top", 0, "pafta1", "Pafta 1"),
    ("pafta2.tif", "mozaik-top", 1, "pafta2", "Pafta 2"),
    ("pafta3.tif", "mozaik-top", 2, "pafta3", "Pafta 3"),
    ("dem.tif", "ornekle-mean-3.0", 0, "dem", "DEM"),
    ("bolge.tif", "bolge-mean", 0, "bolge", "Bölge rasteri"),
    ("arazi.tif", "komsu-ortalama-3x3", 0, "arazi", "Arazi"),
    ("yil1.tif", "hucre-mean", 0, "yil1", "Yıl 1"),
    ("yil2.tif", "hucre-mean", 1, "yil2", "Yıl 2"),
    ("yil3.tif", "hucre-mean", 2, "yil3", "Yıl 3"),
]


def input_of(file):
    f = next(x for x in FILES if x[0] == file)
    return ref(f[1])["inputs"][f[2]]


def write_tiff(path, r):
    types = {"f32": gdal.GDT_Float32, "f64": gdal.GDT_Float64, "u8": gdal.GDT_Byte}
    ds = gdal.GetDriverByName("GTiff").Create(str(path), r["width"], r["height"], r["bands"], types[r["sample"]])
    ds.SetGeoTransform(r["affine"])
    arr = np.array([math.nan if v is None else v for v in r["values"]], dtype=np.float64).reshape(r["height"], r["width"], r["bands"])
    for k in range(r["bands"]):
        band = ds.GetRasterBand(k + 1)
        if r["nodata"] == "nan":
            band.SetNoDataValue(float("nan"))
        band.WriteArray(arr[:, :, k])
    ds.FlushCache()
    ds = None


def read_tiff(path):
    ds = gdal.Open(str(path))
    vals = [ds.GetRasterBand(k + 1).ReadAsArray().astype(np.float64) for k in range(ds.RasterCount)]
    return list(ds.GetGeoTransform()), np.stack(vals, axis=-1)


# ── The drawing ──────────────────────────────────────────────────────────

MASK = ref("kirp-kare")["shapes"]
ZONES = ref("bolge-mean")["shapes"]
ZONE_LABELS = ["B1", "B2", "B3", "B4"]
# A mask far from every raster.
FAR = {"kind": "polygon", "pts": [{"x": 501000.0, "y": 4421000.0}, {"x": 501010.0, "y": 4421000.0}, {"x": 501010.0, "y": 4421010.0},
                                  {"x": 501000.0, "y": 4421010.0}]}


def drawing():
    layer = lambda id, name, color: {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
                                     "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25}, "children": []}
    entities = []

    def add(e, layer_id, attrs=None, label=None):
        o = {**e, "id": len(entities) + 1, "layerId": layer_id, "attrs": attrs or {}}
        if label:
            o["label"] = label
        entities.append(o)

    for file, case, k, lid, _ in FILES:
        r = ref(case)["inputs"][k]
        add({"kind": "raster", "affine": r["affine"], "width": r["width"], "height": r["height"], "bands": r["bands"],
             "sample": r["sample"], "file": file, "srid": 5254,
             "style": {"render": "ramp", "bands": [1], "stretch": "minMax", "ramp": "Arazi"}}, lid)
    for s in MASK:
        add(s, "maske")
    for k, s in enumerate(ZONES):
        add(s, "bolgeler", {"Ad": f"Bölge {k + 1}"}, ZONE_LABELS[k])
    add(FAR, "uzak")
    # Two orthophotos on one layer (the expression's names, never run): four bands, then three.
    for file, bands in ORTHOS:
        add({"kind": "raster", "affine": [500100.0, 0.5, 0.0, 4420100.0, 0.0, -0.5], "width": 4, "height": 4, "bands": bands, "sample": "u8",
             "file": file, "srid": 5254, "style": {"render": "rgb", "bands": [1, 2, 3]}}, "orto")
    layers = [layer(lid, name, "#8D6E63") for _, _, _, lid, name in FILES]
    layers += [layer("orto", "Ortofoto", "#8D6E63")]
    layers += [layer("maske", "Maske", "#C62828"), layer("bolgeler", "Bölgeler", "#1565C0"), layer("uzak", "Uzak maske", "#C62828")]
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "İşlem durumları: raster işlemleri",
        "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": 500000, "y": 4420000},
        "layers": layers,
        "activeLayer": "kot",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }


ORTHOS = [("orto1.tif", 4), ("orto2.tif", 3)]


def run_names():
    """Every raster's name in an expression, in the run's order (ADR 0233 §2, §3), with its bands: the layers from the top of
    the panel down, on a layer the later first; a second raster on a layer “Ad (2)”."""
    rasters = [(name, 1) for *_, name in FILES] + [("Ortofoto", bands) for _, bands in reversed(ORTHOS)]
    out, seen = [], {}
    for name, bands in rasters:
        seen[name] = seen.get(name, 0) + 1
        out.append((name if seen[name] == 1 else f"{name} ({seen[name]})", bands))
    return out


def expression_names():
    """The names an expression field offers on every raster: each raster's, and each band after the first with `@`."""
    return sorted([n for n, _ in run_names()] + [f"{n}@{b}" for n, bands in run_names() for b in range(2, bands + 1)])


def ids_of(*lids):
    """The drawing's raster ids on these layers."""
    return [k + 1 for k, f in enumerate(FILES) if f[3] in lids]


# ── The tools' rules ─────────────────────────────────────────────────────

LABELS = {
    "raster.calculator": "Raster hesaplayıcı", "raster.reclassify": "Yeniden sınıflandır", "raster.clipByMask": "Maskeyle kırp",
    "raster.mosaic": "Mozaik", "raster.resample": "Yeniden örnekle", "raster.zonalStatistics": "Bölgesel istatistik",
    "raster.histogram": "Histogram", "raster.focalStatistics": "Komşuluk istatistiği", "raster.cellStatistics": "Hücre istatistiği",
}
SUFFIX = {
    "raster.calculator": "-hesap", "raster.reclassify": "-sinif", "raster.clipByMask": "-kirpik", "raster.mosaic": "-mozaik",
    "raster.resample": "-ornek", "raster.focalStatistics": "-komsuluk", "raster.cellStatistics": "-hucre",
}
LAYER = {
    "raster.calculator": "Hesap", "raster.reclassify": "Sınıflar", "raster.clipByMask": "Kırpılmış", "raster.mosaic": "Mozaik",
    "raster.resample": "Örneklenmiş", "raster.focalStatistics": "Komşuluk", "raster.cellStatistics": "Hücre istatistiği",
}
ARAZI = {"render": "ramp", "bands": [1], "stretch": "minMax", "ramp": "Arazi"}


def new_layer(name):
    return {"id": layer_id(name), "name": name, "style": {"color": RASTER_LAYER, "lineType": "continuous", "lineWeight": 0.25}}


def ramp(name):
    return {"render": "ramp", "bands": [1], "stretch": "minMax", "ramp": name}


def raster_case(id, title, tool, values, refname, *, first, files, style, extra="", selection=()):
    """A tool that writes a raster: its file, object, layer and summary."""
    e = ref(refname)["expect"]["raster"]
    stem = first[:-4]
    out = f"{stem}{SUFFIX[tool]}.tif"
    summary = f"{words(e['width'])} × {words(e['height'])} hücrelik raster; “{out}” yazıldı.{extra}"
    layer_name = LAYER[tool]
    sample = {"raster.calculator": values.get("sample", "f32"), "raster.reclassify": values.get("sample", "f32"),
              "raster.focalStatistics": "f32", "raster.cellStatistics": "f32"}.get(tool, input_of(first)["sample"])
    expect = {
        "status": "ok",
        "undo": LABELS[tool],
        "summary": summary,
        "outputs": {"file": out},
        "layers": [new_layer(layer_name)],
        "added": [{"kind": "raster", "layerId": layer_id(layer_name), "attrs": {}, "affine": e["affine"], "width": e["width"],
                   "height": e["height"], "bands": e["bands"], "sample": sample, "file": out, "srid": 5254, "style": style}],
        "layerAbove": {layer_id(layer_name): next(f[3] for f in FILES if f[0] == first)},
        "rasterOpsOf": {out: refname},
    }
    return {"id": id, "title": title, "document": "raster-ops.kcad", "selection": list(selection), "run": {"tool": tool},
            "values": values, "expect": expect}


def zone_figures():
    """The zones' sums by the reference's own rules (its cells and statistics)."""
    r_json = ref("bolge-mean")["inputs"][0]
    r = ops.Raster(r_json["affine"], r_json["width"], r_json["height"], [math.nan if v is None else v for v in r_json["values"]],
                   1, r_json["sample"], r_json["nodata"])
    rows = []
    for k, s in enumerate(ZONES):
        cells = ops.inside_cells(s, r.affine, r.width, r.height)
        vals = [Fraction(v) for (i, j) in cells for v in [r.get(i, j, 0)] if v is not None and not math.isnan(v)]
        figure = lambda st: ops.as_float(ops.stat_of(vals, st))
        rows.append({"n": len(vals), "sum": figure("sum"), "mean": figure("mean"), "min": figure("min"),
                     "max": figure("max"), "std": figure("std")})
    return rows


def unmatched(refname):
    """The cells no rule of the reference case's table holds."""
    c = ref(refname)
    t = c["tool"]
    rules = ops.parse_table(t["table"])
    vals = [math.nan if v is None else v for v in c["inputs"][0]["values"]]
    return sum(1 for v in vals if ops.apply_rules(rules, t["bounds"], v)[0] == "miss")


def fig(v, d=3):
    return "" if v is None else shown(v, d)


def zonal_case():
    figs = zone_figures()
    updated = []
    rows = []
    for k, f in enumerate(figs):
        zone_id = len(FILES) + len(MASK) + k + 1
        attrs = {"Ad": f"Bölge {k + 1}"}
        if f["mean"] is not None:
            attrs["Ortalama"] = fig(f["mean"])
        # A zone without a cell keeps its attributes: nothing removed, nothing written.
        if f["mean"] is not None:
            updated.append({"id": zone_id, "attrs": attrs, "label": ZONE_LABELS[k]})
        rows.append([ZONE_LABELS[k], shown(f["n"], 0), fig(f["sum"]), fig(f["mean"]), fig(f["min"]), fig(f["max"]), fig(f["std"])])
    empty = sum(1 for f in figs if f["n"] == 0)
    expect = {
        "status": "ok",
        "undo": LABELS["raster.zonalStatistics"],
        "summary": f"{len(updated)} bölgeye “Ortalama” yazıldı (ortalama).",
        "outputs": {"table": {"columns": ["Nesne", "Sayı", "Toplam", "Ortalama", "En küçük", "En büyük", "Standart sapma"], "rows": rows},
                    "changed": [u["id"] for u in updated], "count": len(updated)},
        "updated": updated,
        "layers": [],
        "added": [],
    }
    if empty:
        expect["log"] = [{"level": "warn", "text": f"{words(empty)} bölgenin içinde değeri olan hücre merkezi yok."}]
    return {"id": "zonal", "title": "Bölgesel istatistik, Ortalama: üç bölgeye yazılır (biri daire), dışarıdaki bölge boş; tablo",
            "document": "raster-ops.kcad", "selection": [], "run": {"tool": "raster.zonalStatistics"},
            "values": {"input": {"scope": "layer", "layerId": "bolge"}, "zones": {"scope": "layer", "layerId": "bolgeler"},
                       "stat": "mean", "output": "Ortalama"},
            "expect": expect}


def histogram_case():
    h = ref("histogram-kendi")["expect"]["histogram"]
    n = len(h["counts"])
    total = max(sum(h["counts"]), 1)
    rows = []
    so_far = 0
    for k, c in enumerate(h["counts"]):
        so_far += c
        a = h["lo"] + (h["hi"] - h["lo"]) * k / n
        b = h["hi"] if k + 1 == n else h["lo"] + (h["hi"] - h["lo"]) * (k + 1) / n
        rows.append([str(k + 1), shown(a, 3), shown(b, 3), str(c), shown(100.0 * c / total, 2), shown(100.0 * so_far / total, 2)])
    summary = f"{words(h['valid'])} hücre, {shown(h['lo'], 3)} ile {shown(h['hi'], 3)} arası {words(n)} aralık."
    if h["empty"]:
        summary += f" Değeri olmayan {words(h['empty'])} hücre."
    return {"id": "histogram", "title": "Histogram, 8 aralık, bandın sınırlarıyla: değersiz hücreler sayılır", "document": "raster-ops.kcad",
            "selection": [], "run": {"tool": "raster.histogram"}, "values": {"input": {"scope": "layer", "layerId": "bolge"}, "bins": 8},
            "expect": {"status": "ok", "undo": None, "summary": summary,
                       "outputs": {"table": {"columns": ["Aralık", "Alt sınır", "Üst sınır", "Sayı", "Oran (%)", "Birikimli (%)"], "rows": rows}},
                       "layers": [], "added": []}}


def defaults():
    ends = lambda layer: {"output": "", "add": True, "layer": {"newName": layer}}
    here = {"scope": "layer", "layerId": "kot"}
    many = lambda first: {"scope": first} if first != "layer" else here
    return {
        "raster.calculator": {"input": {"scope": "visible"}, "expression": "", "empty": "propagate", "sample": "f32", **ends("Hesap")},
        "raster.reclassify": {"input": {"scope": "selection"}, "band": 1, "table": "", "bounds": "upperClosed", "unmatched": "keep", "sample": "f32",
                              **ends("Sınıflar")},
        "raster.clipByMask": {"input": {"scope": "selection"}, "mask": here, "crop": True, **ends("Kırpılmış")},
        "raster.mosaic": {"input": {"scope": "selection"}, "overlap": "top", "sampling": "nearest", **ends("Mozaik")},
        "raster.resample": {"input": {"scope": "selection"}, "cell": 0, "method": "nearest", **ends("Örneklenmiş")},
        "raster.zonalStatistics": {"input": {"scope": "selection"}, "band": 1, "zones": here, "stat": "mean", "output": "Ortalama", "decimals": 3},
        "raster.histogram": {"input": {"scope": "selection"}, "band": 1, "bins": 20, "min": None, "max": None},
        "raster.focalStatistics": {"input": {"scope": "selection"}, "band": 1, "shape": "rect", "width": 3, "height": 3, "radius": 3, "inner": 1,
                                   "stat": "mean", "ignore": True, **ends("Komşuluk")},
        "raster.cellStatistics": {"input": {"scope": "selection"}, "band": 1, "stat": "mean", "ignore": True, **ends("Hücre istatistiği")},
    }


def cases():
    out = [
        raster_case("calculator", "Raster hesaplayıcı: [Kot] * 2 + [Fark], iki raster seçili; değersizler değersiz kalır",
                    "raster.calculator", {"input": {"scope": "selection"}, "expression": "[Kot] * 2 + [Fark]"}, "hesap-iki-raster",
                    first="kot.tif", files=["kot.tif", "fark.tif"], style=ramp("Viridis"), selection=ids_of("kot", "fark")),
        raster_case("reclassify", "Yeniden sınıflandır: * 5 1; 5 20 2; 30 boş; boş 99 — tabloda olmayanlar kalır",
                    "raster.reclassify", {"input": {"scope": "layer", "layerId": "egim"}, "table": "* 5 1; 5 20 2; 30 boş; boş 99"},
                    "sinif-ust-kapali", first="egim.tif", files=["egim.tif"], style=ramp("Spektral"),
                    extra=f" Tablonun hiçbir kuralının tutmadığı {words(unmatched('sinif-ust-kapali'))} hücre."),
        raster_case("clip", "Maskeyle kırp: kare maske, kutusuna kırpılır", "raster.clipByMask",
                    {"input": {"scope": "layer", "layerId": "buyuk"}, "mask": {"scope": "layer", "layerId": "maske"}},
                    "kirp-kare", first="buyuk.tif", files=["buyuk.tif"], style=ARAZI, extra=" Maskenin içinde {inside} hücre."),
        raster_case("mosaic", "Mozaik, üç pafta (biri iri hücreli), Üstteki: en ince ızgarada", "raster.mosaic",
                    {"input": {"scope": "selection"}}, "mozaik-top", first="pafta1.tif", files=["pafta1.tif", "pafta2.tif", "pafta3.tif"],
                    style=ARAZI, extra=f" {words(3)} raster birleşti.", selection=ids_of("pafta1", "pafta2", "pafta3")),
        raster_case("resample", "Yeniden örnekle, 3 m Ortalama: örtüşmeyle ağırlıklı", "raster.resample",
                    {"input": {"scope": "layer", "layerId": "dem"}, "cell": 3, "method": "mean"}, "ornekle-mean-3.0",
                    first="dem.tif", files=["dem.tif"], style=ARAZI),
        raster_case("focal", "Komşuluk istatistiği, 3 × 3 Ortalama: değersizler yok sayılır", "raster.focalStatistics",
                    {"input": {"scope": "layer", "layerId": "arazi"}}, "komsu-ortalama-3x3", first="arazi.tif", files=["arazi.tif"],
                    style=ramp("Viridis")),
        raster_case("cells", "Hücre istatistiği, üç yıl Ortalama (biri kaydırılmış): ortak ızgarada", "raster.cellStatistics",
                    {"input": {"scope": "selection"}}, "hucre-mean", first="yil1.tif", files=["yil1.tif", "yil2.tif", "yil3.tif"],
                    style=ramp("Viridis"), extra=f" {words(3)} raster birleşti.", selection=ids_of("yil1", "yil2", "yil3")),
        raster_case("calculator-all", "Raster hesaplayıcı bütün rasterlerde, [Yıl 3] - [Yıl 1]: yalnız andığı iki raster açılır (ortofotoların "
                    "dosyası yok); sonuç ilk andığının ızgarasında, adıyla ve katmanının üstünde", "raster.calculator",
                    {"input": {"scope": "all"}, "expression": "[Yıl 3] - [Yıl 1]"}, "hesap-ilk-anilan", first="yil3.tif",
                    files=["yil3.tif", "yil1.tif"], style=ramp("Viridis")),
        zonal_case(),
        histogram_case(),
    ]
    # Maskenin içindeki hücreler: the reference's cells.
    clip = next(c for c in out if c["id"] == "clip")
    r_json = ref("kirp-kare")["inputs"][0]
    inside = set()
    for s in MASK:
        inside |= ops.inside_cells(s, r_json["affine"], r_json["width"], r_json["height"])
    clip["expect"]["summary"] = clip["expect"]["summary"].replace("{inside}", words(len(inside)))
    refused = lambda id, title, tool, values, message, selection=(): {
        "id": id, "title": title, "document": "raster-ops.kcad", "selection": list(selection), "run": {"tool": tool}, "values": values,
        "expect": {"status": "error", "message": message},
    }
    out += [
        refused("calculator-name", "Raster hesaplayıcı, adı olmayan raster: adlar söylenir", "raster.calculator",
                {"input": {"scope": "selection"}, "expression": "[Yok] + 1"},
                "“Yok” adında raster bandı yok. Rasterler: [Kot], [Fark]; bant @ ile: [Ad@2].", ids_of("kot", "fark")),
        refused("calculator-name-all", "Raster hesaplayıcı bütün rasterlerde, adı olmayan raster: bütün adlar panelin sırasıyla",
                "raster.calculator", {"input": {"scope": "all"}, "expression": "[Yok] + 1"},
                "“Yok” adında raster bandı yok. Rasterler: " + ", ".join(f"[{n}]" for n, _ in run_names()) + "; bant @ ile: [Ad@2]."),
        refused("resample-cell", "Yeniden örnekle, hücre boyu yazılmadı", "raster.resample", {"input": {"scope": "layer", "layerId": "dem"}},
                "Hücre boyunu yazın (0'dan büyük)."),
        refused("mosaic-one", "Mozaik tek rasterle", "raster.mosaic", {"input": {"scope": "layer", "layerId": "pafta1"}},
                "Mozaik en az iki raster ister."),
        refused("clip-outside", "Maskeyle kırp, maske rasterin dışında", "raster.clipByMask",
                {"input": {"scope": "layer", "layerId": "kot"}, "mask": {"scope": "layer", "layerId": "uzak"}},
                "Maske rasterle kesişmiyor: maskenin içinde rasterin hiçbir hücre merkezi yok."),
    ]
    return out


def build():
    return {
        "format": "kentos.processing-cases",
        "version": 1,
        "tolerance": 1e-9,
        "rasters": {f: f"raster-ops/{f}" for f, *_ in FILES},
        "documents": {
            "raster-ops.kcad": {
                "defaults": {"lengthDecimals": 3, "areaDecimals": 2, "angleUnit": "grad", "plotScale": 1000, "drawingFont": "barlow",
                             "activeLayer": "kot", "measureHeightMm": 2},
                "tools": defaults(),
            }
        },
        "cases": cases(),
        # The input's names an expression field offers (its chips and the builder's fields), each on one object.
        "inputFields": [
            {"id": "calculator-names", "title": "Raster hesaplayıcı'nın İfade alanı: bütün rasterlerin adları ve bantları; aynı katmanda sonra "
                                                "eklenen “Ortofoto”, öncekini “Ortofoto (2)”",
             "document": "raster-ops.kcad", "tool": "raster.calculator", "values": {"input": {"scope": "all"}},
             "fields": [[name, 1] for name in expression_names()]},
        ],
    }


def main():
    check = "--check" in sys.argv[1:]
    problems = []
    RASTERS.mkdir(parents=True, exist_ok=True)
    for file, *_ in FILES:
        path = RASTERS / file
        r = input_of(file)
        if not check:
            write_tiff(path, r)
        if not path.exists():
            problems.append(f"{path.relative_to(ROOT)} yok")
            continue
        affine, arr = read_tiff(path)
        want = np.array([math.nan if v is None else v for v in r["values"]], dtype=np.float64).reshape(r["height"], r["width"], r["bands"])
        if affine != r["affine"] or arr.shape != want.shape or not np.array_equal(arr, want, equal_nan=True):
            problems.append(f"{path.relative_to(ROOT)} başvurunun girdisi değil")
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
    print(f"{CASES.relative_to(ROOT)}: {len(build()['cases'])} durum; çizim {DRAWING.relative_to(ROOT)}; {len(FILES)} raster")


if __name__ == "__main__":
    main()
