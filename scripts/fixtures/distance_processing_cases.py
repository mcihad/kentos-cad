#!/usr/bin/env python3
"""Uzaklık ve maliyet'in İşlemler durumları (docs/adr/0236; biçim fixtures/processing/README.md).

    python3 scripts/fixtures/distance_processing_cases.py           # rasterleri, çizimi ve durumları yazar
    python3 scripts/fixtures/distance_processing_cases.py --check   # hiçbir şey yazmaz; karşılaştırır

KentOS kodu kullanılmaz. Rasterler (fixtures/processing/v1/distance/*.tif, GDAL'la) bağımsız başvurunun
(fixtures/distance/v1/cases.json, scripts/fixtures/distance_cases.py) girdileridir: maliyet rasteri, engelli maliyet
rasteri, yükseklik modeli, kaynak hücreli raster, ızgarası alınan raster ve sıfır maliyetli raster; çizim
(fixtures/processing/v1/distance.kcad) onları katman katman bağlı raster olarak, kaynakları, varışları ve uçları taşır.
Durumlar araçların kuralından yazılır: yazılan raster dosyasının değerleri başvurunun aynı adlı durumununkilerdir
(`distanceOf`, durumun kuralıyla), yollar başvurunun yollarıdır (köşeler dünyada; öznitelikleri sayılarının ADR 0149'un
gösterim kuralıyla metni: Yol ve Kaynak 0, öbürleri 3 basamak, sondaki sıfırlar atılır); özetlerin sayıları başvurunun
notlarından, sözleri araçların, sayılar binlik noktalı.
"""

import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import distance_cases as dc  # noqa: E402  (the reference's rasters and objects)
import raster_ops_processing_cases as rop  # noqa: E402  (its GeoTIFF writer, layer ids, thousands)
from numeric_display import shown  # noqa: E402  (the display rule's own reference)

DIR = ROOT / "fixtures/processing/v1"
RASTERS = DIR / "distance"
DRAWING = DIR / "distance.kcad"
CASES = DIR / "distance.json"
REF = json.loads((ROOT / "fixtures/distance/v1/cases.json").read_text("utf-8"))
RASTER_LAYER = "#7A6B5B"
PATH_COLOR = "#E5484D"
words = rop.words
layer_id = rop.layer_id


def ref(name):
    return next(c for c in REF["cases"] if c["name"] == name)


def trimmed(v, d):
    t = shown(v, d)
    return t.rstrip("0").rstrip(".") if "." in t else t


# ── The rasters: the reference's inputs, each a file ─────────────────────

GRID_AFFINE = [499990.0, 10.0, 0.0, 4420090.0, 0.0, -6.0]


def grid_raster():
    """The raster whose grid Kapsam takes: the reference's grid, its values not read."""
    return dc.ro.Raster(list(GRID_AFFINE), 16, 17, [0.0] * (16 * 17), 1, "f32", "nan").json()


# File, layer id, layer name, the raster (checked to be the reference's input).
FILES = [
    ("kaynak-raster.tif", "kaynak-raster", "Kaynak raster", lambda: ref("uzaklik-raster-kare")["input"]),
    ("izgara.tif", "izgara", "Izgara", grid_raster),
    ("maliyet.tif", "maliyet", "Maliyet", lambda: ref("maliyet-16")["input"]),
    ("engel.tif", "engel", "Engelli maliyet", lambda: ref("maliyet-engel-16")["input"]),
    ("yukseklik.tif", "yukseklik", "Yükseklik", lambda: ref("maliyet-yukseklik-egim")["surface"]),
    ("sifir.tif", "sifir", "Sıfır maliyet", lambda: ref("ret-maliyet-sifir")["input"]),
]
assert ref("maliyet-16")["input"] == ref("yol-16")["input"] == ref("koridor-percent")["input"]
assert ref("yol-yukseklik")["surface"] == ref("maliyet-yukseklik-egim")["surface"]

# Object layers (the panel's order, top first): their objects, as the reference's cases have them.
OBJECTS = [
    ("kaynaklar", "Kaynaklar", ref("maliyet-16")["shapes"], "#C62828"),
    ("baslangic", "Başlangıç", ref("yol-16")["shapes"][:1], "#2E7D32"),
    ("varis", "Varış", ref("yol-16")["shapes"][1:], "#1565C0"),
    ("varis-iki", "Varış (iki)", ref("yol-yukseklik")["shapes"][1:], "#1565C0"),
    ("ikinci", "İkinci uç", ref("koridor-percent")["shapes"][1:], "#6A1B9A"),
    ("engel-kaynak", "Engelin kaynağı", ref("maliyet-engel-16")["shapes"], "#C62828"),
    ("engelde", "Engeldeki kaynak", ref("ret-maliyet-kaynak-engelde")["shapes"], "#C62828"),
    ("nesneler", "Nesneler", ref("uzaklik-nesneler-uzaklik")["shapes"], "#EF6C00"),
]
assert ref("yol-16")["shapes"][:1] == ref("yol-yukseklik")["shapes"][:1] == ref("koridor-percent")["shapes"][:1]


# ── The drawing ──────────────────────────────────────────────────────────

ARAZI = {"render": "ramp", "bands": [1], "stretch": "minMax", "ramp": "Arazi"}


def drawing():
    def layer(id, name, color):
        return {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
                "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25}, "children": []}

    entities = []

    def add(e, lid, attrs=None):
        entities.append({**e, "id": len(entities) + 1, "layerId": lid, "attrs": attrs or {}})

    for file, lid, _, raster in FILES:
        r = raster()
        add({"kind": "raster", "affine": r["affine"], "width": r["width"], "height": r["height"], "bands": 1,
             "sample": r["sample"], "file": file, "srid": 5254, "style": ARAZI}, lid)
    for lid, name, shapes, _ in OBJECTS:
        for k, s in enumerate(shapes):
            add(s, lid, {"Ad": f"{name} {k + 1}"})
    layers = [layer(lid, name, color) for lid, name, _, color in OBJECTS]
    layers += [layer(lid, name, "#8D6E63") for _, lid, name, _ in FILES]
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "İşlem durumları: uzaklık ve maliyet",
        "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                     "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": 1000, "y": 2000},
        "layers": layers,
        "activeLayer": "kaynaklar",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }


# ── The tools' rules ─────────────────────────────────────────────────────

LABELS = {"distance.euclidean": "Uzaklık yüzeyi", "distance.cost": "Birikimli maliyet", "distance.path": "En düşük maliyetli yol",
          "distance.corridor": "Maliyet koridoru"}
SUFFIX = {"distance.euclidean": "-uzaklik", "distance.cost": "-maliyet", "distance.corridor": "-koridor"}
LAYER = {"distance.euclidean": "Uzaklık", "distance.cost": "Birikimli maliyet", "distance.corridor": "Maliyet koridoru"}


def ramp(name, stretch="minMax", **more):
    return {"render": "ramp", "bands": [1], "stretch": stretch, "ramp": name, **more}


VIRIDIS = ramp("Viridis")
COST = ramp("Viridis", "percent")
SOURCES = ramp("Spektral", resampling="nearest")


def new_layer(name):
    return {"id": layer_id(name), "name": name, "style": {"color": RASTER_LAYER, "lineType": "continuous", "lineWeight": 0.25}}


def raster_of(lid):
    return next(f for f in FILES if f[1] == lid)[3]()


def stem(lid):
    return next(f for f in FILES if f[1] == lid)[0][:-4]


def on(lid):
    return {"scope": "layer", "layerId": lid}


def raster_case(id, title, tool, values, refname, style, said, *, input=None, below=None, log=()):
    """A tool that writes a raster: its file, object, layer and summary; the new layer right above the input raster's
    (right below the sources' for Uzaklık yüzeyi from objects)."""
    e = ref(refname)["expect"]
    r = e["raster"]
    notes = e["notes"]
    name = LAYER[tool]
    if below:
        out = f"{next(n for lid, n, _, _ in OBJECTS if lid == below)}{SUFFIX[tool]}.tif"
        affine = r["affine"]
    else:
        out = f"{stem(input)}{SUFFIX[tool]}.tif"
        affine = raster_of(input)["affine"]
    summary = said(r, notes, out)
    expect = {
        "status": "ok",
        "undo": LABELS[tool],
        "summary": summary,
        "outputs": {"file": out},
        "layers": [new_layer(name)],
        "added": [{"kind": "raster", "layerId": layer_id(name), "attrs": {}, "affine": affine, "width": r["width"],
                   "height": r["height"], "bands": 1, "sample": r["sample"], "file": out, "srid": 5254, "style": style}],
        ("layerBelow" if below else "layerAbove"): {layer_id(name): below or input},
        "distanceOf": {out: refname},
    }
    if log:
        expect["log"] = list(log)
    vals = dict(values)
    if input:
        vals = {"input": on(input), **vals}
    return {"id": id, "title": title, "document": "distance.kcad", "selection": [], "run": {"tool": tool},
            "values": vals, "expect": expect}


def cells_of(r):
    return sum(v is not None for v in r["values"])


def from_objects_says(r, notes, out):
    s = f"{words(notes['taken'])} kaynaktan {words(r['width'])} × {words(r['height'])} hücrelik raster; “{out}” yazıldı."
    if notes["empty"]:
        s += f" En büyük uzaklığın ötesinde {words(notes['empty'])} hücre değersiz."
    return s


def grid_says(r, out):
    return f"{words(r['width'])} × {words(r['height'])} hücrelik raster; “{out}” yazıldı."


def euclid_raster_says(allocation):
    def said(r, notes, out):
        s = grid_says(r, out) + f" {words(notes['sources'])} kaynak hücre"
        if not allocation and notes["cells"]:
            s += f"; en uzak hücre {trimmed(notes['most'], 3)} m"
        return s + "."
    return said


def cost_says(allocation):
    def said(r, notes, out):
        s = grid_says(r, out) + f" {words(notes['sources'])} kaynak hücre"
        if not allocation and notes["cells"]:
            s += f"; en büyük birikimli maliyet {trimmed(notes['most'], 3)}"
        s += "."
        empty = r["width"] * r["height"] - notes["cells"]
        if empty:
            s += f" Değersiz {words(empty)} hücre (engel, erişilemeyen ya da sınırın ötesi)."
        return s
    return said


def corridor_says(r, notes, out):
    least = trimmed(notes["least"], 3)
    if not notes["cells"]:
        return grid_says(r, out) + f" En ucuz yolun maliyeti {least}; eşiğin içinde hücre yok."
    return grid_says(r, out) + f" En ucuz yolun maliyeti {least}; koridorda {words(notes['cells'])} hücre."


def attr_text(field, v):
    return trimmed(v, 0) if field in ("Yol", "Kaynak") else trimmed(v, 3)


def path_case(id, title, values, refname, log=()):
    """En düşük maliyetli yol: the reference's paths as polylines on a new layer right above the cost raster's."""
    e = ref(refname)["expect"]
    f = e["features"]
    name = "En düşük maliyetli yol"
    lid = layer_id(name)
    stride = len(f["fields"])
    added, at = [], 0
    for k, n in enumerate(f["sizes"]):
        pts = [{"x": f["xy"][2 * (at + q)], "y": f["xy"][2 * (at + q) + 1]} for q in range(n)]
        at += n
        attrs = {field: attr_text(field, f["numbers"][k * stride + x]) for x, field in enumerate(f["fields"])}
        added.append({"kind": "polyline", "layerId": lid, "attrs": attrs, "pts": pts})
    expect = {
        "status": "ok",
        "undo": LABELS["distance.path"],
        "summary": f"{words(len(added))} yol yazıldı.",
        "outputs": {"count": len(added)},
        "layers": [{"id": lid, "name": name, "style": {"color": PATH_COLOR, "lineType": "continuous", "lineWeight": 0.25,
                                                         "fill": f"{PATH_COLOR}26"}}],
        "added": added,
        "layerAbove": {lid: "maliyet"},
    }
    unreached = e["notes"]["unreached"]
    log = list(log)
    if unreached:
        log.append({"level": "warn", "text": f"{words(len(unreached))} varışa yol yok (erişilemiyor ya da rasterin değerli hücrelerine "
                                              f"düşmüyor): {', '.join(str(u) for u in unreached)}."})
    if log:
        expect["log"] = log
    return {"id": id, "title": title, "document": "distance.kcad", "selection": [], "run": {"tool": "distance.path"},
            "values": {"input": on("maliyet"), **values}, "expect": expect}


def refused(id, title, tool, values, message):
    return {"id": id, "title": title, "document": "distance.kcad", "selection": [], "run": {"tool": tool}, "values": values,
            "expect": {"status": "error", "message": message}}


def cases():
    E, C = "distance.euclidean", "distance.cost"
    return [
        raster_case("euclid-objects", "Uzaklık yüzeyi, nesnelerden: nokta, çizgi ve alanın kutusu 23 m paylı, 5 m hücre; katman nesnelerin "
                    "hemen altında", E, {"sources": on("nesneler"), "margin": 23, "cellSize": 5}, "uzaklik-nesneler-uzaklik", VIRIDIS,
                    from_objects_says, below="nesneler"),
        raster_case("euclid-objects-allocation", "Uzaklık yüzeyi, nesnelerden, En yakın kaynak: nesnenin girdideki sırası", E,
                    {"sources": on("nesneler"), "margin": 23, "cellSize": 5, "result": "allocation"}, "uzaklik-nesneler-tahsis", SOURCES,
                    from_objects_says, below="nesneler"),
        raster_case("euclid-objects-grid", "Uzaklık yüzeyi, rasterin ızgarasında (10 × 6 m hücre), en büyük uzaklık 60 m", E,
                    {"sources": on("nesneler"), "extent": "raster", "grid": on("izgara"), "max": 60}, "uzaklik-nesneler-izgara",
                    VIRIDIS, from_objects_says, below="nesneler"),
        raster_case("euclid-raster", "Uzaklık yüzeyi, rasterden: değerli hücreler kaynak; katman rasterin hemen üstünde", E,
                    {"from": "raster"}, "uzaklik-raster-kare", VIRIDIS, euclid_raster_says(False), input="kaynak-raster"),
        raster_case("euclid-raster-allocation", "Uzaklık yüzeyi, rasterden, En yakın kaynak: hücrenin değeri", E,
                    {"from": "raster", "result": "allocation"}, "uzaklik-raster-tahsis", SOURCES, euclid_raster_says(True),
                    input="kaynak-raster"),
        raster_case("cost", "Birikimli maliyet, 16 komşu: bir nokta ve bir çizgiden", C, {"sources": on("kaynaklar")}, "maliyet-16",
                    COST, cost_says(False), input="maliyet"),
        raster_case("cost-8-allocation", "Birikimli maliyet, 8 komşu, En ucuz kaynak", C,
                    {"sources": on("kaynaklar"), "neighbours": "8", "result": "allocation"}, "maliyet-8-tahsis", SOURCES, cost_says(True),
                    input="maliyet"),
        raster_case("cost-max", "Birikimli maliyet, en büyük maliyet 60, 64 bit: ötesi değersiz", C,
                    {"sources": on("kaynaklar"), "max": 60, "sample": "f64"}, "maliyet-en-buyuk", COST, cost_says(False), input="maliyet"),
        raster_case("cost-barrier", "Birikimli maliyet, engeller: köşeden bağlı duvar geçilmez", C, {"sources": on("engel-kaynak")},
                    "maliyet-engel-16", COST, cost_says(False), input="engel"),
        raster_case("cost-surface", "Birikimli maliyet, yükseklik modeliyle: yüzey uzunluğu ve en büyük boyuna eğim %12", C,
                    {"sources": on("kaynaklar"), "useSurface": True, "surface": on("yukseklik"), "surfaceLength": True, "slope": 12, "sample": "f64"},
                    "maliyet-yukseklik-egim", COST, cost_says(False), input="maliyet"),
        path_case("path", "En düşük maliyetli yol, 16 komşu: üç varış, rasterin dışındaki söylenir",
                  {"sources": on("baslangic"), "targets": on("varis")}, "yol-16"),
        path_case("path-8-simplify", "En düşük maliyetli yol, 8 komşu, sadeleştirme 1 hücre",
                  {"sources": on("baslangic"), "targets": on("varis"), "neighbours": "8", "simplify": 1}, "yol-8"),
        path_case("path-surface", "En düşük maliyetli yol, yükseklik modeliyle: yüzey uzunluğu, en büyük eğim %9",
                  {"sources": on("baslangic"), "targets": on("varis-iki"), "useSurface": True, "surface": on("yukseklik"), "surfaceLength": True,
                   "slope": 9},
                  "yol-yukseklik"),
        raster_case("corridor", "Maliyet koridoru, en küçük toplamın %10 fazlasına kadar", "distance.corridor",
                    {"sources": on("baslangic"), "targets": on("ikinci"), "threshold": "percent"}, "koridor-percent", VIRIDIS,
                    corridor_says, input="maliyet"),
        raster_case("corridor-below", "Maliyet koridoru, 8 komşu, eşik 300: en ucuz yol 300'den pahalı, hücre kalmaz", "distance.corridor",
                    {"sources": on("baslangic"), "targets": on("ikinci"), "neighbours": "8", "threshold": "value", "value": 300},
                    "koridor-esik-alti", VIRIDIS, corridor_says, input="maliyet"),
        refused("cost-zero", "Birikimli maliyet, maliyeti 0 olan hücre: ret", C, {"input": on("sifir"), "sources": on("kaynaklar")},
                "Maliyet rasterinde 0 ya da eksi değer var (en küçüğü 0): maliyet 0'dan büyük olmalı. Geçilmeyecek yerleri değersiz yapın, "
                "çok ucuz yerlere küçük bir artı değer verin."),
        refused("cost-surface-missing", "Birikimli maliyet, Yükseklik modeliyle açık, yükseklik modeli seçilmemiş: ret", C,
                {"input": on("maliyet"), "sources": on("kaynaklar"), "useSurface": True},
                "Yükseklik modelini seçin: Yükseklik modeliyle açık."),
        refused("cost-source-in-barrier", "Birikimli maliyet, kaynak engelin içinde: ret", C,
                {"input": on("engel"), "sources": on("engelde")},
                "Kaynak hücre yok: kaynak nesneleri maliyet rasterinin değerli hücrelerine düşmüyor."),
    ]


def defaults():
    one = {"scope": "selection"}
    here = {"scope": "layer", "layerId": "kaynaklar"}
    raster = lambda name: {"output": "", "add": True, "layer": {"newName": name}}
    network = {"neighbours": "16", "useSurface": False, "surface": one, "surfaceLength": False, "slope": 0}
    return {
        "distance.euclidean": {"from": "objects", "sources": here, "input": one, "band": 1, "max": 0, "result": "distance", "margin": 100,
                               "cellSize": 0, "extent": "objects", "grid": one, **raster("Uzaklık")},
        "distance.cost": {"input": one, "band": 1, "sources": here, **network, "max": 0, "result": "cost", "sample": "f32",
                          **raster("Birikimli maliyet")},
        "distance.path": {"input": one, "band": 1, "sources": here, "targets": here, **network, "simplify": 0,
                          "layer": {"newName": "En düşük maliyetli yol"}},
        "distance.corridor": {"input": one, "band": 1, "sources": here, "targets": here, **network, "threshold": "none", "percent": 10,
                              "value": 0, "sample": "f32", **raster("Maliyet koridoru")},
    }


def build():
    return {
        "format": "kentos.processing-cases",
        "version": 1,
        "tolerance": 1e-9,
        "rasters": {f: f"distance/{f}" for f, *_ in FILES},
        "documents": {
            "distance.kcad": {
                "defaults": {"lengthDecimals": 3, "areaDecimals": 2, "angleUnit": "grad", "plotScale": 1000, "drawingFont": "barlow",
                             "activeLayer": "kaynaklar", "measureHeightMm": 2},
                "tools": defaults(),
            }
        },
        "cases": cases(),
    }


def main():
    check = "--check" in sys.argv[1:]
    problems = []
    RASTERS.mkdir(parents=True, exist_ok=True)
    for file, _, _, raster in FILES:
        path = RASTERS / file
        r = raster()
        if not check:
            rop.write_tiff(path, r)
        else:
            if not path.exists():
                problems.append(f"{path} yok")
                continue
            affine, vals = rop.read_tiff(path)
            want = [math.nan if v is None else v for v in r["values"]]
            got = vals.reshape(-1).tolist()
            if affine != r["affine"] or len(got) != len(want) or any(
                    not ((math.isnan(a) and math.isnan(b)) or a == b) for a, b in zip(got, want)):
                problems.append(f"{path.name} başvurunun girdisi değil")
    texts = {DRAWING: json.dumps(drawing(), ensure_ascii=False, indent=1) + "\n",
             CASES: json.dumps(build(), ensure_ascii=False, indent=1) + "\n"}
    for path, text in texts.items():
        if check:
            if not path.exists() or path.read_text("utf-8") != text:
                problems.append(f"{path.name} güncel değil")
        else:
            path.write_text(text, "utf-8")
    if problems:
        print("\n".join(problems) + f"\nYeniden yazın: python3 {sys.argv[0]}; farkı okuyun.")
        sys.exit(1)
    print(f"{CASES.name}: {len(cases())} durum, {len(FILES)} raster" + ("; güncel." if check else " yazıldı."))


if __name__ == "__main__":
    main()
