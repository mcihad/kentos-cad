#!/usr/bin/env python3
"""Uzaktan algılama'nın İşlemler durumları (docs/adr/0242; biçim fixtures/processing/README.md).

    python3 scripts/fixtures/remote_processing_cases.py           # rasterleri, çizimi ve durumları yazar
    python3 scripts/fixtures/remote_processing_cases.py --check   # hiçbir şey yazmaz; karşılaştırır

KentOS kodu kullanılmaz. Rasterler (fixtures/processing/v1/remote/*.tif, GDAL'la; dört bantlı 16 bitlerin ek bantları
ExtraSamples 0, yani veri) bağımsız başvurunun (fixtures/remote/v1/cases.json, scripts/fixtures/remote_cases.py)
girdileridir: dört tek bantlı bant, dört bantlı görüntü ve sonraki tarihi, iki tarihin sınıf rasterleri, sınıflandırılmış
raster, bölenleri sıfır olan küçük raster, çok bantlı ve pankromatik görüntü; çizim (fixtures/processing/v1/remote.kcad)
onları katman katman bağlı raster olarak (Katmanlar panelindeki sırası araçların sırasıdır), eğitim alanlarını ve
referans nesnelerini Sınıf öznitelikleriyle taşır. Durumlar araçların kuralından yazılır: yazılan raster dosyasının
değerleri başvurunun aynı adlı durumununkilerdir (`remoteOf`, durumun kuralıyla: tam sayı sonuçta “sum” bir birim);
tablolar, özetin kuyrukları ve uyarılar başvurunun notlarından; sayılar binlik noktalı.
"""

import json
import math
import sys
from pathlib import Path

import numpy as np
from osgeo import gdal

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import raster_ops_processing_cases as rop  # noqa: E402  (layer ids, thousands)

gdal.UseExceptions()

DIR = ROOT / "fixtures/processing/v1"
RASTERS = DIR / "remote"
DRAWING = DIR / "remote.kcad"
CASES = DIR / "remote.json"
REF = json.loads((ROOT / "fixtures/remote/v1/cases.json").read_text("utf-8"))
RASTER_LAYER = "#7A6B5B"
words = rop.words
layer_id = rop.layer_id


def ref(name):
    return next(c for c in REF["cases"] if c["name"] == name)


# ── The rasters: the reference's inputs, each a file on its own layer (the panel's order, top first) ──

RGB = {"render": "rgb", "bands": [3, 2, 1], "stretch": "percent"}
GRAY = {"render": "gray", "bands": [1], "stretch": "minMax"}
CLASSES = {"render": "ramp", "bands": [1], "stretch": "minMax", "ramp": "Spektral", "resampling": "nearest"}

# File, layer id, layer name, (reference case, input index), look.
FILES = [
    ("mavi.tif", "mavi", "Mavi", ("birlestir-dort-bant", 0), GRAY),
    ("yesil.tif", "yesil", "Yeşil", ("birlestir-dort-bant", 1), GRAY),
    ("kirmizi.tif", "kirmizi", "Kırmızı", ("birlestir-dort-bant", 2), GRAY),
    ("yko.tif", "yko", "YKÖ", ("birlestir-dort-bant", 3), GRAY),
    ("goruntu.tif", "goruntu", "Görüntü", ("indis-ndvi", 0), RGB),
    ("sonraki.tif", "sonraki", "Sonraki", ("degisim-difference", 1), RGB),
    ("once-sinif.tif", "once-sinif", "Önceki sınıflar", ("degisim-siniflar", 0), CLASSES),
    ("sonra-sinif.tif", "sonra-sinif", "Sonraki sınıflar", ("degisim-siniflar", 1), CLASSES),
    ("siniflar.tif", "siniflar", "Sınıflar", ("dogruluk", 0), CLASSES),
    ("sifirli.tif", "sifirli", "Sıfırlı", ("indis-bolen-sifir", 0), GRAY),
    ("cok-bantli.tif", "cok-bantli", "Çok bantlı", ("birlestirme-mean-nearest", 0), RGB),
    ("pankromatik.tif", "pankromatik", "Pankromatik", ("birlestirme-mean-nearest", 1), GRAY),
]
assert ref("indis-ndvi")["inputs"][0] == ref("denetimli-likelihood")["inputs"][0] == ref("degisim-difference")["inputs"][0]
assert ref("ayir-bant-1")["inputs"][0] == ref("indis-ndvi")["inputs"][0]
assert ref("birlestirme-mean-nearest")["inputs"] == ref("birlestirme-brovey-cubic")["inputs"]


def raster_of(lid):
    case, k = next(f for f in FILES if f[1] == lid)[3]
    return ref(case)["inputs"][k]


def id_of(lid):
    """The raster's object id: rasters come first, in the files' order."""
    return 1 + [f[1] for f in FILES].index(lid)


def write_tiff(path, r):
    types = {"f32": gdal.GDT_Float32, "u16": gdal.GDT_UInt16, "u8": gdal.GDT_Byte}
    ds = gdal.GetDriverByName("GTiff").Create(str(path), r["width"], r["height"], r["bands"], types[r["sample"]])
    ds.SetGeoTransform(r["affine"])
    arr = np.array([math.nan if v is None else v for v in r["values"]], dtype=np.float64).reshape(r["height"], r["width"], r["bands"])
    for k in range(r["bands"]):
        band = ds.GetRasterBand(k + 1)
        if r["nodata"] == "nan":
            band.SetNoDataValue(float("nan"))
        elif r["nodata"] is not None:
            band.SetNoDataValue(float(r["nodata"]))
        band.WriteArray(arr[:, :, k])
    ds.FlushCache()
    ds = None


def read_tiff(path):
    ds = gdal.Open(str(path))
    vals = [ds.GetRasterBand(k + 1).ReadAsArray().astype(np.float64) for k in range(ds.RasterCount)]
    last = ds.GetRasterBand(ds.RasterCount).GetColorInterpretation() == gdal.GCI_AlphaBand
    return list(ds.GetGeoTransform()), np.stack(vals, axis=-1), last


# Object layers (above the rasters): the training areas and the reference objects, as the reference's cases have them.
TRAINING = ref("denetimli-likelihood")
REFERENCE = ref("dogruluk")
OFF = ref("ret-dogruluk-hucresiz")["shapes"]


def classed(shapes, texts):
    return [(s, {} if t is None else {"Sınıf": t}) for s, t in zip(shapes, texts)]


OBJECTS = [
    ("egitim", "Eğitim alanları", classed(TRAINING["shapes"], TRAINING["tool"]["texts"]), "#2E7D32"),
    ("referans", "Referans", classed(REFERENCE["shapes"], REFERENCE["tool"]["reference"]), "#C62828"),
    ("uzak", "Uzak", classed(OFF, ["5"]), "#6A1B9A"),
]


def object_id(lid, k):
    first = len(FILES) + 1
    for l, _, objs, _ in OBJECTS:
        if l == lid:
            return first + k
        first += len(objs)
    raise KeyError(lid)


# ── The drawing ──────────────────────────────────────────────────────────

def drawing():
    def layer(id, name, color):
        return {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
                "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25}, "children": []}

    entities = []

    def add(e, lid, attrs=None):
        entities.append({**e, "id": len(entities) + 1, "layerId": lid, "attrs": attrs or {}})

    for file, lid, _, _, style in FILES:
        r = raster_of(lid)
        add({"kind": "raster", "affine": r["affine"], "width": r["width"], "height": r["height"], "bands": r["bands"],
             "sample": r["sample"], "file": file, "srid": 5254, "style": style}, lid)
    for lid, _, objs, _ in OBJECTS:
        for s, attrs in objs:
            add(s, lid, attrs)
    layers = [layer(lid, name, color) for lid, name, _, color in OBJECTS]
    layers += [layer(lid, name, "#8D6E63") for _, lid, name, _, _ in FILES]
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "İşlem durumları: uzaktan algılama",
        "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                     "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": 1000, "y": 2000},
        "layers": layers,
        "activeLayer": "egitim",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }


# ── The tools' rules ─────────────────────────────────────────────────────

LABELS = {"remote.composite": "Bant birleştir", "remote.split": "Bantlara ayır", "remote.index": "Spektral indis",
          "remote.supervised": "Denetimli sınıflandırma", "remote.unsupervised": "Denetimsiz sınıflandırma",
          "remote.accuracy": "Doğruluk analizi", "remote.change": "Değişim tespiti", "remote.pansharpen": "Görüntü birleştirme"}
SUFFIX = {"remote.composite": "-birlesik", "remote.index": "-indis", "remote.supervised": "-siniflar", "remote.unsupervised": "-kumeler",
          "remote.change": "-degisim", "remote.pansharpen": "-birlesim"}
LAYER = {"remote.composite": "Bant birleştir", "remote.split": "Bantlar", "remote.index": "Spektral indis",
         "remote.supervised": "Denetimli sınıflandırma", "remote.unsupervised": "Denetimsiz sınıflandırma", "remote.change": "Değişim",
         "remote.pansharpen": "Görüntü birleştirme"}


def ramp(name, stretch="percent", invert=False, lo=None, hi=None, nearest=False):
    """A look in the contract's field order (the web compares objects' texts)."""
    s = {"render": "ramp", "bands": [1], "stretch": stretch}
    if lo is not None:
        s["min"], s["max"] = lo, hi
    s["ramp"] = name
    if invert:
        s["invert"] = True
    if nearest:
        s["resampling"] = "nearest"
    return s


COMPOSITE = {"render": "rgb", "bands": [1, 2, 3], "stretch": "percent"}
BAND = {"render": "gray", "bands": [1], "stretch": "percent"}
VEGETATION = ramp("Arazi", invert=True)
DIVERGING = ramp("Mavi-kırmızı")


def change_look(refname, centre=0.0):
    """Değişim tespiti's look (§9): Mavi-kırmızı evenly round no change as far as the largest change."""
    vals = [v for v in ref(refname)["expect"]["raster"]["values"] if v is not None]
    reach = max(max(vals) - centre, centre - min(vals))
    return ramp("Mavi-kırmızı", "manual", lo=centre - reach, hi=centre + reach)


def class_look(k):
    return ramp("Spektral", "manual", lo=0.5, hi=k + 0.5, nearest=True)


def new_layer(name):
    return {"id": layer_id(name), "name": name, "style": {"color": RASTER_LAYER, "lineType": "continuous", "lineWeight": 0.25}}


def on(lid):
    return {"scope": "layer", "layerId": lid}


SELECTION = {"scope": "selection"}


def raster_object(r, layer, file, style):
    return {"kind": "raster", "layerId": layer, "attrs": {}, "affine": r["affine"], "width": r["width"], "height": r["height"],
            "bands": r["bands"], "sample": r["sample"], "file": file, "srid": 5254, "style": style}


def log_of(warnings):
    return [{"level": "warn", "text": w} for w in warnings]


def raster_case(id, title, tool, values, refname, style, *, first, selection=()):
    """A tool that writes a raster: its file beside the first input, the object on a new layer right above that input's."""
    e = ref(refname)["expect"]
    r, notes = e["raster"], e["notes"]
    out = f"{first}{SUFFIX[tool]}.tif"
    name = LAYER[tool]
    tail = f" {notes['tail']}" if notes["tail"] else ""
    expect = {
        "status": "ok",
        "undo": LABELS[tool],
        "summary": f"{words(r['width'])} × {words(r['height'])} hücrelik raster; “{out}” yazıldı.{tail}",
        "outputs": {"file": out, **({"table": notes["table"]} if notes["table"] else {})},
        "layers": [new_layer(name)],
        "added": [raster_object(r, layer_id(name), out, style)],
        "layerAbove": {layer_id(name): first},
        "remoteOf": {out: refname},
    }
    if notes["warnings"]:
        expect["log"] = log_of(notes["warnings"])
    vals = {"input": SELECTION if selection else on(first), **values}
    return {"id": id, "title": title, "document": "remote.kcad", "selection": list(selection), "run": {"tool": tool},
            "values": vals, "expect": expect}


def split_case():
    files = [f"goruntu-b{b}.tif" for b in range(1, 5)]
    rs = [ref(f"ayir-bant-{b}")["expect"]["raster"] for b in range(1, 5)]
    name = LAYER["remote.split"]
    expect = {
        "status": "ok",
        "undo": LABELS["remote.split"],
        "summary": f"4 bant ayrı rasterlere yazıldı: “{files[0]}” … “{files[-1]}” (12 × 10 hücre).",
        "outputs": {"table": {"columns": ["Bant", "Dosya"], "rows": [[str(b + 1), f] for b, f in enumerate(files)]}},
        "layers": [new_layer(name)],
        "added": [raster_object(r, layer_id(name), f, BAND) for r, f in zip(rs, files)],
        "layerAbove": {layer_id(name): "goruntu"},
        "remoteOf": {f: f"ayir-bant-{b + 1}" for b, f in enumerate(files)},
    }
    return {"id": "split-four", "title": "Bantlara ayır: dört bantlı 16 bit görüntü (son bandı veri), her bant kendi dosyasında",
            "document": "remote.kcad", "selection": [], "run": {"tool": "remote.split"}, "values": {"input": on("goruntu")},
            "expect": expect}


def accuracy_case():
    a = REFERENCE["expect"]["accuracy"]
    expect = {"status": "ok", "undo": None, "summary": a["summary"],
              "outputs": {"table": a["table"], "overall": a["overall"], "kappa": a["kappa"]}}
    if a["warnings"]:
        expect["log"] = log_of(a["warnings"])
    return {"id": "accuracy", "title": "Doğruluk analizi: noktalar, bir alan ve çok noktalı nesne; okunamayan referans ve rasterin "
                                       "dışındaki nokta söylenir", "document": "remote.kcad", "selection": [],
            "run": {"tool": "remote.accuracy"},
            "values": {"input": on("siniflar"), "reference": on("referans"), "referenceField": "Sınıf"}, "expect": expect}


def refused(id, title, tool, values, message, selection=()):
    return {"id": id, "title": title, "document": "remote.kcad", "selection": list(selection), "run": {"tool": tool},
            "values": values, "expect": {"status": "error", "message": message}}


def cases():
    C, I, S, U, A, D, P = ("remote.composite", "remote.index", "remote.supervised", "remote.unsupervised", "remote.accuracy",
                           "remote.change", "remote.pansharpen")
    bands4 = [id_of(l) for l in ("mavi", "yesil", "kirmizi", "yko")]
    training = {"training": on("egitim"), "classField": "Sınıf"}
    k_classes = len(ref("denetimli-likelihood")["expect"]["notes"]["table"]["rows"])
    return [
        raster_case("composite-four", "Bant birleştir: dört tek bantlı raster panelin sırasıyla; tabloda bantların kaynağı", C, {},
                    "birlestir-dort-bant", COMPOSITE, first="mavi", selection=bands4),
        split_case(),
        raster_case("index-ndvi", "Spektral indis, NDVI: bantlar 3 ve 4; özette en küçük ve en büyük", I, {"index": "ndvi"}, "indis-ndvi",
                    VEGETATION, first="goruntu"),
        raster_case("index-evi", "Spektral indis, EVI: yansıma ölçeği 0,0001", I, {"index": "evi", "scale": 0.0001}, "indis-evi-yansima",
                    VEGETATION, first="goruntu"),
        raster_case("index-zero", "Spektral indis, normalize fark: bölenin sıfır olduğu hücreler boş, söylenir", I,
                    {"index": "normalized", "a": 1, "b": 2}, "indis-bolen-sifir", DIVERGING, first="sifirli"),
        raster_case("supervised-likelihood", "Denetimli sınıflandırma, en büyük olabilirlik: dört sınıf, kırpılan ve boş sınıf adları, "
                                             "örtüşen eğitim alanları", S, {**training, "method": "likelihood"}, "denetimli-likelihood",
                    class_look(k_classes), first="goruntu"),
        raster_case("supervised-distance", "Denetimli sınıflandırma, en yakın ortalama", S, {**training, "method": "distance"},
                    "denetimli-distance", class_look(k_classes), first="goruntu"),
        raster_case("unsupervised-four", "Denetimsiz sınıflandırma: dört küme, en çok 20 yineleme", U, {"clusters": 4, "iterations": 20},
                    "denetimsiz-4-20", class_look(4), first="goruntu"),
        accuracy_case(),
        raster_case("change-difference", "Değişim tespiti, fark: YKÖ bandı; artan, azalan ve değişmeyen hücreler", D,
                    {"after": on("sonraki"), "band": 4, "method": "difference"}, "degisim-difference", change_look("degisim-difference"),
                    first="goruntu"),
        raster_case("change-classes", "Değişim tespiti, sınıf değişimi: neden neye matrisi", D,
                    {"after": on("sonra-sinif"), "method": "classes"}, "degisim-siniflar",
                    {"render": "ramp", "bands": [1], "stretch": "minMax", "ramp": "Spektral", "resampling": "nearest"},
                    first="once-sinif"),
        raster_case("pansharpen-mean", "Görüntü birleştirme, basit ortalama, en yakın: 20 m'lik dört bant 5 m'ye", P,
                    {"pan": on("pankromatik"), "method": "mean", "sampling": "nearest"}, "birlestirme-mean-nearest", RGB,
                    first="cok-bantli"),
        raster_case("pansharpen-brovey", "Görüntü birleştirme, Brovey, kübik, eşit ağırlıklar", P, {"pan": on("pankromatik")},
                    "birlestirme-brovey-cubic", RGB, first="cok-bantli"),
        raster_case("pansharpen-weights", "Görüntü birleştirme, Brovey, çift doğrusal, ağırlıklar noktalı virgülle ve boşlukla", P,
                    {"pan": on("pankromatik"), "weights": "0,1; 0,3 0,3;0,3", "sampling": "bilinear"}, "birlestirme-agirlik-metni",
                    RGB, first="cok-bantli"),
        refused("composite-one", "Bant birleştir, tek raster: ret", C, {"input": on("mavi")}, "Bant birleştir en az iki raster ister."),
        refused("index-band", "Spektral indis, MNDWI'nin kısa dalga kızılötesi bandı 5: dört bantlı görüntüde ret", I,
                {"input": on("goruntu"), "index": "mndwi"}, "Rasterin 4 bandı var; 5. bant yok."),
        refused("supervised-no-class", "Denetimli sınıflandırma, sınıf adları boş: ret", S,
                {"input": on("goruntu"), "training": SELECTION, "classField": "Sınıf"},
                "Eğitim alanı yok: Sınıf alanı dolu en az bir alan seçin.", [object_id("egitim", 5), object_id("egitim", 7)]),
        refused("unsupervised-few", "Denetimsiz sınıflandırma, sekiz hücrede 50 küme: ret", U, {"input": on("sifirli"), "clusters": 50},
                "Örnek hücre sayısı (8) küme sayısından (50) az."),
        refused("accuracy-off", "Doğruluk analizi, referans rasterin dışında: ret", A,
                {"input": on("siniflar"), "reference": on("uzak"), "referenceField": "Sınıf"},
                ref("ret-dogruluk-hucresiz")["expect"]["refused"]),
        {"id": "change-no-after", "title": "Değişim tespiti, sonraki rasterin katmanında raster yok: çalışmaz, alanın sorunu söylenir",
         "document": "remote.kcad", "selection": [], "run": {"tool": D}, "values": {"input": on("goruntu"), "after": on("referans")},
         "expect": {"status": "invalid", "issues": [{"param": "after",
                                                      "message": "“Sonraki raster”: bu katmanda uygun nesne yok. Başka bir katman seçin."}]}},
        refused("pansharpen-weights-count", "Görüntü birleştirme, iki ağırlık dört banda: ret", P,
                {"input": on("cok-bantli"), "pan": on("pankromatik"), "weights": "0,5; 0,5"},
                "Ağırlıkların sayısı (2) çok bantlının bant sayısı (4) değil."),
    ]


def defaults():
    one = {"scope": "selection"}
    here = {"scope": "layer", "layerId": "egitim"}
    visible = {"scope": "visible"}
    raster = lambda name: {"output": "", "add": True, "layer": {"newName": name}}
    return {
        "remote.composite": {"input": visible, "sampling": "nearest", **raster("Bant birleştir")},
        "remote.split": {"input": one, **raster("Bantlar")},
        "remote.index": {"input": one, "index": "ndvi", "blue": 1, "green": 2, "red": 3, "nir": 4, "swir": 5, "a": 4, "b": 3, "saviL": 0.5,
                         "eviG": 2.5, "eviC1": 6, "eviC2": 7.5, "eviL": 1, "scale": 1, "offset": 0, **raster("Spektral indis")},
        "remote.supervised": {"input": one, "training": here, "classField": "", "method": "likelihood", **raster("Denetimli sınıflandırma")},
        "remote.unsupervised": {"input": one, "clusters": 8, "iterations": 20, **raster("Denetimsiz sınıflandırma")},
        "remote.accuracy": {"input": one, "band": 1, "reference": here, "referenceField": ""},
        "remote.change": {"input": one, "after": one, "band": 1, "method": "difference", **raster("Değişim")},
        "remote.pansharpen": {"input": one, "pan": one, "method": "brovey", "weights": "", "sampling": "cubic", **raster("Görüntü birleştirme")},
    }


def build():
    return {
        "format": "kentos.processing-cases",
        "version": 1,
        "tolerance": 1e-9,
        "rasters": {f: f"remote/{f}" for f, *_ in FILES},
        "documents": {
            "remote.kcad": {
                "defaults": {"lengthDecimals": 3, "areaDecimals": 2, "angleUnit": "grad", "plotScale": 1000, "drawingFont": "barlow",
                             "activeLayer": "egitim", "measureHeightMm": 2},
                "tools": defaults(),
            }
        },
        "cases": cases(),
    }


def main():
    check = "--check" in sys.argv[1:]
    problems = []
    RASTERS.mkdir(parents=True, exist_ok=True)
    for file, lid, *_ in FILES:
        path = RASTERS / file
        r = raster_of(lid)
        if not check:
            write_tiff(path, r)
        if not path.exists():
            problems.append(f"{path.relative_to(ROOT)} yok")
            continue
        affine, arr, alpha = read_tiff(path)
        want = np.array([math.nan if v is None else v for v in r["values"]], dtype=np.float64).reshape(r["height"], r["width"], r["bands"])
        if affine != r["affine"] or arr.shape != want.shape or not np.array_equal(arr, want, equal_nan=True) or alpha:
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
