#!/usr/bin/env python3
"""Raster ve vektör ile Taranmış harita'nın İşlemler durumları (docs/adr/0234 §10; biçim fixtures/processing/README.md).

    python3 scripts/fixtures/raster_vector_processing_cases.py           # rasterleri, çizimi ve durumları yazar
    python3 scripts/fixtures/raster_vector_processing_cases.py --check   # hiçbir şey yazmaz; karşılaştırır

KentOS kodu kullanılmaz. Rasterler (fixtures/processing/v1/raster-vector/*.tif, GDAL'la) bağımsız başvurunun
(fixtures/raster-vector/v1/cases.json, scripts/fixtures/raster_vector_cases.py) girdileridir; çizim
(fixtures/processing/v1/raster-vector.kcad) onları katman katman bağlı raster olarak, yakılacak nesneleri Değer
alanlarıyla ve kot verilecek eğrileri taşır. Durumlar araçların kuralından yazılır: nesneler başvurunun aynı adlı
durumunun nesneleridir (alanlar, çizgiler, noktalar dünyada; öznitelikleri Değer ve Tür, noktanın kotu değeri), yazılan
raster dosyasının değerleri başvurunun durumununkilerdir (`rasterVectorOf`), eğrilerin köşe kotları başvurunun
kotlarıdır (`elevations`); özetler ve iletiler araçların sözleri, sayılar binlik noktalı.
"""

import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import raster_ops_processing_cases as rop  # noqa: E402  (its GeoTIFF writer, layer ids, thousands)

DIR = ROOT / "fixtures/processing/v1"
RASTERS = DIR / "raster-vector"
DRAWING = DIR / "raster-vector.kcad"
CASES = DIR / "raster-vector.json"
REF = json.loads((ROOT / "fixtures/raster-vector/v1/cases.json").read_text("utf-8"))
RASTER_LAYER = "#7A6B5B"
words = rop.words
layer_id = rop.layer_id


def ref(name):
    return next(c for c in REF["cases"] if c["name"] == name)


def elevations_ref(name):
    return next(c for c in REF["elevations"] if c["name"] == name)


# ── The rasters: the reference's inputs, each a file ─────────────────────

FILES = [
    ("siniflar.tif", "alan-four", "siniflar", "Sınıflar"),
    ("cizgiler.tif", "cizgi-kisa-parcalar", "cizgiler", "Çizgi rasteri"),
    ("yukseklik.tif", "nokta-extrema-1", "yukseklik", "Yükseklik"),
    ("tarama.tif", "yakala-egik", "tarama", "Taranmış pafta"),
    ("pafta.tif", "kapat-degen-delik", "pafta", "Parsel paftası"),
    ("pafta2.tif", "kapat-kesisen", "pafta2", "Parsel paftası 2"),
]


def input_of(file):
    f = next(x for x in FILES if x[0] == file)
    return ref(f[1])["inputs"][0]


# ── The drawing ──────────────────────────────────────────────────────────

BURN = ref("rasterlestir-last")
CURVES = elevations_ref("kot-yukari")
GRID = ref("rasterlestir-raster-izgarasi")["grid"]


def drawing():
    def layer(id, name, color):
        return {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
                "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25}, "children": []}

    entities = []

    def add(e, layer_id, attrs=None):
        entities.append({**e, "id": len(entities) + 1, "layerId": layer_id, "attrs": attrs or {}})

    for file, _, lid, _ in FILES:
        r = input_of(file)
        style = {"render": "rgb", "bands": [1, 2, 3]} if r["bands"] == 3 else {"render": "ramp", "bands": [1], "stretch": "minMax", "ramp": "Arazi"}
        add({"kind": "raster", "affine": r["affine"], "width": r["width"], "height": r["height"], "bands": r["bands"],
             "sample": r["sample"], "file": file, "srid": 5254, "style": style}, lid)
    # The burnt objects in the reference's order, their values in Değer (a text that is no number among them).
    for s, text in zip(BURN["shapes"], BURN["values"]):
        add(s, "nesneler", {"Değer": text})
    # A raster whose grid Rasterleştir takes (its file is never read).
    add({"kind": "raster", "affine": GRID["affine"], "width": GRID["width"], "height": GRID["height"], "bands": 1, "sample": "f32",
         "file": "izgara.tif", "srid": 5254, "style": {"render": "ramp", "bands": [1], "stretch": "minMax", "ramp": "Arazi"}}, "izgara")
    for k, c in enumerate(CURVES["curves"]):
        add(c, "egriler", {"Ad": f"E{k + 1}"})
    layers = [layer(lid, name, "#8D6E63") for _, _, lid, name in FILES]
    layers += [layer("nesneler", "Nesneler", "#1565C0"), layer("izgara", "Izgara", "#8D6E63"), layer("egriler", "Eğriler", "#A0522D")]
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "İşlem durumları: raster ve vektör",
        "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                     "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": 500000, "y": 4420000},
        "layers": layers,
        "activeLayer": "siniflar",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }


def ids_on(lid):
    return [e["id"] for e in drawing()["entities"] if e["layerId"] == lid]


# ── The tools' rules ─────────────────────────────────────────────────────

LABELS = {
    "raster.rasterize": "Rasterleştir", "raster.toPolygons": "Rasterden alan", "raster.toLines": "Rasterden çizgi",
    "raster.toPoints": "Rasterden nokta", "scan.captureLine": "Çizgi yakala", "scan.closeArea": "Alan kapat",
    "scan.contourElevations": "Eğrilere kot ver",
}
COLORS = {"Bölgeler": "#3E63DD", "Çizgiler": "#E5484D", "Noktalar": "#30A46C", "Yakalanan çizgiler": "#D6409F", "Kapatılan alanlar": "#6E56CF"}


def output_layer(name):
    color = COLORS[name]
    return {"id": layer_id(name), "name": name, "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25, "fill": f"{color}26"}}


def raster_layer(name):
    return {"id": layer_id(name), "name": name, "style": {"color": RASTER_LAYER, "lineType": "continuous", "lineWeight": 0.25}}


def objects(f, layer, attrs, z=None):
    """The reference's features as the tool's objects on `layer`."""
    out, at, ring = [], 0, 0

    def path(n):
        nonlocal at
        pts = [{"x": f["xy"][2 * (at + q)], "y": f["xy"][2 * (at + q) + 1]} for q in range(n)]
        at += n
        return pts

    for k in range(len(f["values"])):
        zk = None if z is None else z(k)
        o = {"kind": None, "layerId": layer, "attrs": attrs(k)}
        if f["kind"] == "points":
            o.update(kind="point", p=path(1)[0])
            if zk is not None:
                o["z"] = zk
        elif f["kind"] == "lines":
            pts = path(f["sizes"][k])
            o.update(kind="polyline", pts=pts)
            if zk is not None:
                o["zs"] = [zk] * len(pts)
        else:
            rings = [path(f["sizes"][ring + r]) for r in range(f["rings"][k])]
            ring += f["rings"][k]
            o.update(kind="polygon", pts=rings[0])
            if len(rings) > 1:
                o["holes"] = [{"pts": r} for r in rings[1:]]
        out.append(o)
    return out


def vector_case(id, title, tool, values, refname, layer_name, on, summary, attrs=lambda f, k: {}, z=None, log=None):
    f = ref(refname)["expect"]["features"]
    lid = layer_id(layer_name)
    added = objects(f, lid, lambda k: attrs(f, k), z)
    expect = {
        "status": "ok",
        "undo": LABELS[tool] if added else None,
        "summary": summary(f, added),
        "outputs": {"count": len(added)},
        "layers": [output_layer(layer_name)] if added else [],
        "added": added,
    }
    if added:
        expect["layerAbove"] = {lid: on}
    if log:
        expect["log"] = log
    return {"id": id, "title": title, "document": "raster-vector.kcad", "selection": [], "run": {"tool": tool}, "values": values,
            "expect": expect}


def rasterize_case(id, title, values, refname, log):
    r = ref(refname)
    e, notes = r["expect"]["raster"], r["expect"]["notes"]
    name = "Nesneler-raster.tif"
    summary = f"{words(notes['taken'])} nesneden {words(e['width'])} × {words(e['height'])} hücrelik raster; “{name}” yazıldı."
    if notes["empty"]:
        summary += f" Değersiz {words(notes['empty'])} hücre."
    lid = layer_id("Rasterleştirilmiş")
    expect = {
        "status": "ok",
        "undo": LABELS["raster.rasterize"],
        "summary": summary,
        "log": log,
        "outputs": {"file": name},
        "layers": [raster_layer("Rasterleştirilmiş")],
        "added": [{"kind": "raster", "layerId": lid, "attrs": {}, "affine": e["affine"], "width": e["width"], "height": e["height"],
                   "bands": 1, "sample": e["sample"], "file": name, "srid": 5254,
                   "style": {"render": "ramp", "bands": [1], "stretch": "minMax", "ramp": "Viridis"}}],
        "layerBelow": {lid: "nesneler"},
        "rasterVectorOf": {name: refname},
    }
    return {"id": id, "title": title, "document": "raster-vector.kcad", "selection": [], "run": {"tool": "raster.rasterize"},
            "values": values, "expect": expect}


def short(v):
    t = f"{v:.3f}"
    return t.rstrip("0").rstrip(".") if "." in t else t


def contour_case():
    c = CURVES
    ids = ids_on("egriler")
    crossed = [(ids[k], z, curve) for k, (z, curve) in enumerate(zip(c["expect"], c["curves"])) if z is not None]
    missed = len(ids) - len(crossed)

    def paths(curve, z):
        if curve["kind"] == "line":
            return [[z, z]]
        out = [[z] * len(curve["pts"])]
        for h in curve.get("holes") or []:
            out.append([z] * len(h["pts"]))
        return out

    n = len(crossed)
    expect = {
        "status": "ok",
        "undo": LABELS["scan.contourElevations"],
        "summary": f"{words(n)} eğriye kot verildi: {short(c['first'])} ile {short(c['first'] + (n - 1) * c['step'])} arası.",
        "log": [{"level": "warn", "text": f"{words(missed)} eğri kesen çizgiyle kesişmediği için değişmedi."}] if missed else [],
        "outputs": {"changed": [i for i, _, _ in sorted(crossed)], "count": n},
        "layers": [],
        "added": [],
        "updated": [{"id": i, "attrs": {"Ad": f"E{ids.index(i) + 1}"}} for i, _, _ in sorted(crossed)],
        "elevations": {str(i): paths(curve, z) for i, z, curve in crossed},
    }
    return {"id": "contour-elevations", "title": "Eğrilere kot ver: kesen çizgi altı eğriden beşini keser (biri delikli alan, biri dışarıda); "
            "ilk kot 100, aralık 2,5", "document": "raster-vector.kcad", "selection": [], "run": {"tool": "scan.contourElevations"},
            "values": {"curves": {"scope": "layer", "layerId": "egriler"}, "start": c["start"], "end": c["end"], "first": c["first"],
                       "step": c["step"]},
            "expect": expect}


def click(name):
    t = ref(name)["tool"]
    return {"x": t["x"], "y": t["y"]}


def refused(id, title, tool, values, message):
    return {"id": id, "title": title, "document": "raster-vector.kcad", "selection": [], "run": {"tool": tool}, "values": values,
            "expect": {"status": "error", "message": message}}


def cases():
    distinct = lambda f, added: len(set(f["texts"]))
    out = [
        rasterize_case("rasterize", "Rasterleştir, Alandan, Son çizilen, 0,5 m: parseller (biri delikli), daire, yol, eğri yollar, yay ve "
                       "çok noktalı kuyu; değeri okunmayan nesne söylenir",
                       {"input": {"scope": "layer", "layerId": "nesneler"}, "valueFrom": "field", "field": "Değer", "overlap": "last",
                        "cellSize": 0.5},
                       "rasterlestir-last", [{"level": "warn", "text": "1 nesnenin değeri sayı olarak okunamadığı için alınmadı."}]),
        rasterize_case("rasterize-grid", "Rasterleştir, En büyük, Ondalık 64 bit, rasterin ızgarasında: ızgaranın dışında kalan nesne söylenir",
                       {"input": {"scope": "layer", "layerId": "nesneler"}, "valueFrom": "field", "field": "Değer", "overlap": "max",
                        "sample": "f64", "extent": "raster", "grid": {"scope": "layer", "layerId": "izgara"}},
                       "rasterlestir-raster-izgarasi",
                       [{"level": "warn", "text": "1 nesnenin değeri sayı olarak okunamadığı için alınmadı."},
                        {"level": "warn", "text": "1 nesne ızgaranın dışında kaldı."}]),
        vector_case("polygons", "Rasterden alan, 4 komşu: bölgeler ilk hücrelerinin sırasıyla, delikli olanlar; Değer özniteliği",
                    "raster.toPolygons", {"input": {"scope": "layer", "layerId": "siniflar"}}, "alan-four", "Bölgeler", "siniflar",
                    lambda f, added: f"{words(distinct(f, added))} değerden {words(len(added))} alan yazıldı.",
                    attrs=lambda f, k: {"Değer": f["texts"][k]}),
        vector_case("lines", "Rasterden çizgi, Kısa parçaları at 3, Sadeleştirme 1: inceltilmiş çubuk, çapraz ve dal",
                    "raster.toLines", {"input": {"scope": "layer", "layerId": "cizgiler"}, "spur": 3},
                    "cizgi-kisa-parcalar", "Çizgiler", "cizgiler", lambda f, added: f"{words(len(added))} çizgi yazıldı."),
        vector_case("points", "Rasterden nokta, Tepeler ve çukurlar, pencere 1: kotlu, Değer ve Tür",
                    "raster.toPoints", {"input": {"scope": "layer", "layerId": "yukseklik"}, "mode": "extrema"},
                    "nokta-extrema-1", "Noktalar", "yukseklik",
                    lambda f, added: f"{words(sum(1 for t in f['tags'] if t == 1))} tepe, {words(sum(1 for t in f['tags'] if t == 2))} çukur yazıldı.",
                    attrs=lambda f, k: {"Değer": f["texts"][k], "Tür": "Tepe" if f["tags"][k] == 1 else "Çukur"},
                    z=lambda k: ref("nokta-extrema-1")["expect"]["features"]["values"][k]),
        vector_case("capture", "Çizgi yakala, kot 120: tıklamanın yanındaki eğik çizgi yakalanır, bütün köşeleri 120",
                    "scan.captureLine", {"input": {"scope": "layer", "layerId": "tarama"}, "at": click("yakala-egik"), "z": 120},
                    "yakala-egik", "Yakalanan çizgiler", "tarama", lambda f, added: f"{words(len(added))} çizgi yakalandı.", z=lambda k: 120),
        vector_case("close", "Alan kapat, Delikler Koru: merdivenli duvarın yanındaki parsel, duvara köşeden değen delik",
                    "scan.closeArea", {"input": {"scope": "layer", "layerId": "pafta"}, "at": click("kapat-degen-delik"), "holes": "keep"},
                    "kapat-degen-delik", "Kapatılan alanlar", "pafta",
                    lambda f, added: f"Alan kapatıldı: {words(sum(f['sizes']))} köşe, {words(f['rings'][0] - 1)} delik."),
        vector_case("close-crossing", "Alan kapat, sadeleşen halkalar birbirine değer: alan sadeleştirilmeden yazılır, söylenir",
                    "scan.closeArea", {"input": {"scope": "layer", "layerId": "pafta2"}, "at": click("kapat-kesisen"), "holes": "keep",
                                       "simplify": 1.5},
                    "kapat-kesisen", "Kapatılan alanlar", "pafta2",
                    lambda f, added: f"Alan kapatıldı: {words(sum(f['sizes']))} köşe, {words(f['rings'][0] - 1)} delik.",
                    log=[{"level": "warn", "text": "Sadeleşen halkalar birbirine değdiği için alan sadeleştirilmeden yazıldı."}]),
        contour_case(),
        refused("close-open", "Alan kapat, paftanın dışındaki beyaz: kenara ulaşır", "scan.closeArea",
                {"input": {"scope": "layer", "layerId": "pafta"}, "at": click("kapat-kenara-ulasir")},
                "Alan kapanmıyor: dolgu rasterin kenarına ulaştı. Boşluğu kapatın ya da Renk toleransı'nı küçültün."),
        refused("rasterize-fit", "Rasterleştir, Bayt'a sığmayan sabit değer", "raster.rasterize",
                {"input": {"scope": "layer", "layerId": "nesneler"}, "value": 300, "sample": "u8"},
                "Değer 300 sonucun türüne sığmıyor: Tam sayı 32 bit ±2 147 483 647'yi, Bayt 0–254'ü alır. Türü Ondalık 32 bit ya da Ondalık "
                "64 bit yapın."),
        refused("contour-step", "Eğrilere kot ver, aralık 0", "scan.contourElevations",
                {"curves": {"scope": "layer", "layerId": "egriler"}, "start": CURVES["start"], "end": CURVES["end"], "first": 100, "step": 0},
                "Aralık 0 olamaz: eksi bir aralık kotları azaltır."),
        refused("lines-color", "Rasterden çizgi, Renk tek bantlı rasterde", "raster.toLines",
                {"input": {"scope": "layer", "layerId": "cizgiler"}, "select": "color", "color": "#000000"},
                "Renkle seçmek üç bantlı (RGB) bir raster ister; bu rasterde Değer aralığı'nı kullanın."),
    ]
    return out


def defaults():
    here = lambda lid: {"scope": "layer", "layerId": lid}
    one = {"scope": "selection"}
    return {
        "raster.rasterize": {"input": here("siniflar"), "valueFrom": "constant", "value": 1, "field": "", "overlap": "last", "sample": "f32",
                             "cellSize": 0, "extent": "points", "grid": one, "output": "", "add": True, "layer": {"newName": "Rasterleştirilmiş"}},
        "raster.toPolygons": {"input": one, "band": 1, "connect": "four", "layer": {"newName": "Bölgeler"}},
        "raster.toLines": {"input": one, "band": 1, "select": "nonZero", "min": 1, "max": 1, "color": "#000000", "tolerance": 60, "spur": 0,
                           "simplify": 1, "layer": {"newName": "Çizgiler"}},
        "raster.toPoints": {"input": one, "band": 1, "mode": "step", "step": 10, "radius": 1, "elevation": True, "layer": {"newName": "Noktalar"}},
        "scan.captureLine": {"input": one, "at": None, "tolerance": 60, "spur": 5, "simplify": 1, "z": None,
                             "layer": {"newName": "Yakalanan çizgiler"}},
        "scan.closeArea": {"input": one, "at": None, "tolerance": 60, "holes": "fill", "simplify": 1, "layer": {"newName": "Kapatılan alanlar"}},
        "scan.contourElevations": {"curves": one, "start": None, "end": None, "first": 0, "step": 5},
    }


def build():
    return {
        "format": "kentos.processing-cases",
        "version": 1,
        "tolerance": 1e-9,
        "rasters": {f: f"raster-vector/{f}" for f, *_ in FILES},
        "documents": {
            "raster-vector.kcad": {
                "defaults": {"lengthDecimals": 3, "areaDecimals": 2, "angleUnit": "grad", "plotScale": 1000, "drawingFont": "barlow",
                             "activeLayer": "siniflar", "measureHeightMm": 2},
                "tools": defaults(),
            }
        },
        "cases": cases(),
    }


def main():
    check = "--check" in sys.argv[1:]
    problems = []
    RASTERS.mkdir(parents=True, exist_ok=True)
    for file, *_ in FILES:
        path = RASTERS / file
        r = input_of(file)
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
