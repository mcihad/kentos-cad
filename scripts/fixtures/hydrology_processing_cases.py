#!/usr/bin/env python3
"""Hidroloji'nin İşlemler durumları (docs/adr/0235; biçim fixtures/processing/README.md).

    python3 scripts/fixtures/hydrology_processing_cases.py           # rasterleri, çizimi ve durumları yazar
    python3 scripts/fixtures/hydrology_processing_cases.py --check   # hiçbir şey yazmaz; karşılaştırır

KentOS kodu kullanılmaz. Rasterler (fixtures/processing/v1/hydrology/*.tif, GDAL'la) bağımsız başvurunun
(fixtures/hydrology/v1/cases.json, scripts/fixtures/hydrology_cases.py) DEM'leridir: drenaj ağı, vadi, teraslar ve
koni; çizim (fixtures/processing/v1/hydrology.kcad) onları katman katman bağlı raster olarak, döküm noktalarını ve iki
güzergâhı taşır. Durumlar araçların kuralından yazılır: yazılan raster dosyasının değerleri başvurunun aynı adlı
durumununkilerdir (`hydrologyOf`, durumun kuralıyla), nesneler başvurunun nesneleridir (alanlar, çizgiler, noktalar
dünyada; öznitelikleri sayılarının ADR 0149'un gösterim kuralıyla metni: alan, uzunluk, düşü, km ve uzaklık 3, eğim 5,
öbürleri 0 basamak, sondaki sıfırlar atılır); özetlerin en büyük değerleri başvurunun kendi 64 bitlik hesabından,
sözleri araçların, sayılar binlik noktalı.
"""

import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import hydrology_cases as hc  # noqa: E402  (the reference's DEMs and rules)
import raster_ops_processing_cases as rop  # noqa: E402  (its GeoTIFF writer, layer ids, thousands)
from numeric_display import shown  # noqa: E402  (the display rule's own reference)

DIR = ROOT / "fixtures/processing/v1"
RASTERS = DIR / "hydrology"
DRAWING = DIR / "hydrology.kcad"
CASES = DIR / "hydrology.json"
REF = json.loads((ROOT / "fixtures/hydrology/v1/cases.json").read_text("utf-8"))
RASTER_LAYER = "#7A6B5B"
words = rop.words
layer_id = rop.layer_id


def ref(name):
    return next(c for c in REF["cases"] if c["name"] == name)


# ── The rasters: the reference's DEMs, each a file ───────────────────────

# File, the reference case whose input it is, the reference's DEM, its layer (the panel's order, top first).
FILES = [
    ("ag.tif", "ag-dokum-0", lambda: hc.dem(32, 26, hc.network), "ag", "Drenaj ağı"),
    ("vadi.tif", "vadi-doldur", lambda: hc.dem(14, 11, hc.valley), "vadi", "Vadi"),
    ("teras.tif", "teras-doldur", lambda: hc.dem(13, 13, hc.terrace), "teras", "Teraslar"),
    ("koni.tif", "koni-doldur", lambda: hc.dem(11, 11, hc.cone), "koni", "Koni"),
]


def input_of(lid):
    f = next(x for x in FILES if x[3] == lid)
    return ref(f[1])["input"]


def grid_of(lid):
    """The reference's grid of a DEM: its own, checked to be the file's."""
    f = next(x for x in FILES if x[3] == lid)
    r = f[2]()
    assert r.json() == ref(f[1])["input"], f"{lid}: the DEM is not the reference's input"
    return hc.Grid(r)


# The outlets (one outside the raster, two on one cell) and the roads, as the reference's cases have them.
POINTS = ref("ag-havza-25")["shapes"]
ROUTES = ref("ag-guzergah-20000")["shapes"]
assert POINTS == ref("ag-dokum-25")["shapes"] and ROUTES == ref("ag-guzergah-0")["shapes"]


# ── The drawing ──────────────────────────────────────────────────────────

ARAZI = {"render": "ramp", "bands": [1], "stretch": "minMax", "ramp": "Arazi"}


def drawing():
    def layer(id, name, color):
        return {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
                "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25}, "children": []}

    entities = []

    def add(e, lid, attrs=None):
        entities.append({**e, "id": len(entities) + 1, "layerId": lid, "attrs": attrs or {}})

    for file, _, _, lid, _ in FILES:
        r = input_of(lid)
        add({"kind": "raster", "affine": r["affine"], "width": r["width"], "height": r["height"], "bands": 1,
             "sample": r["sample"], "file": file, "srid": 5254, "style": ARAZI}, lid)
    for k, p in enumerate(POINTS):
        add(p, "noktalar", {"Ad": f"Ç{k + 1}"})
    for k, s in enumerate(ROUTES):
        add(s, "guzergah", {"Ad": f"Eksen {k + 1}"})
    layers = [layer("noktalar", "Çıkış noktaları", "#C62828"), layer("guzergah", "Güzergâh", "#1565C0")]
    layers += [layer(lid, name, "#8D6E63") for *_, lid, name in FILES]
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "İşlem durumları: hidroloji",
        "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                     "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": 1000, "y": 2000},
        "layers": layers,
        "activeLayer": "ag",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }


# ── The tools' rules ─────────────────────────────────────────────────────

LABELS = {
    "hydrology.fill": "Çukur doldur", "hydrology.flowDirection": "Akış yönü", "hydrology.flowAccumulation": "Akış birikimi",
    "hydrology.wetness": "Topografik nemlilik indisi", "hydrology.pourPoint": "Döküm noktası", "hydrology.watershed": "Noktadan havza",
    "hydrology.basins": "Havzalar", "hydrology.streams": "Dere ağı",
}
SUFFIX = {"hydrology.fill": "-dolu", "hydrology.flowDirection": "-yon", "hydrology.flowAccumulation": "-birikim", "hydrology.wetness": "-twi"}
LAYER = {"hydrology.fill": "Doldurulmuş DEM", "hydrology.flowDirection": "Akış yönü", "hydrology.flowAccumulation": "Akış birikimi",
         "hydrology.wetness": "Nemlilik indisi"}
COLORS = {"Döküm noktaları": "#E5484D", "Havzalar": "#30A46C", "Dere ağı": "#0090FF"}


def ramp(name, stretch="minMax", **more):
    return {"render": "ramp", "bands": [1], "stretch": stretch, "ramp": name, **more}


def coded(top):
    """Akış yönü's look: its codes 1…top by the spectral ramp, nearest (a code is not a height)."""
    return {"render": "ramp", "bands": [1], "stretch": "manual", "min": 1.0, "max": float(top), "ramp": "Spektral", "resampling": "nearest"}


ACCUMULATION = ramp("Viridis", "percent")
WETNESS = ramp("Mavi-kırmızı", "percent", invert=True)


def trimmed(v, d):
    t = shown(v, d)
    return t.rstrip("0").rstrip(".") if "." in t else t


def attr_text(field, v):
    if field == "Eğim":
        return trimmed(v, 5)
    if field in ("Alan", "Uzunluk", "Düşü", "Km", "Uzaklık"):
        return trimmed(v, 3)
    return trimmed(v, 0)


def new_layer(name):
    return {"id": layer_id(name), "name": name, "style": {"color": RASTER_LAYER, "lineType": "continuous", "lineWeight": 0.25}}


def output_layer(name):
    color = COLORS[name]
    return {"id": layer_id(name), "name": name, "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25, "fill": f"{color}26"}}


def objects(f, layer):
    """The reference's features as the tool's objects on `layer`, their numbers as attributes."""
    out, at, ring = [], 0, 0
    stride = len(f["fields"])
    count = len(f["numbers"]) // stride

    def path(n):
        nonlocal at
        pts = [{"x": f["xy"][2 * (at + q)], "y": f["xy"][2 * (at + q) + 1]} for q in range(n)]
        at += n
        return pts

    for k in range(count):
        attrs = {name: attr_text(name, f["numbers"][k * stride + x]) for x, name in enumerate(f["fields"])}
        o = {"kind": None, "layerId": layer, "attrs": attrs}
        if f["kind"] == "points":
            o.update(kind="point", p=path(1)[0])
        elif f["kind"] == "lines":
            o.update(kind="polyline", pts=path(f["sizes"][k]))
        else:
            rings = [path(f["sizes"][ring + r]) for r in range(f["rings"][k])]
            ring += f["rings"][k]
            o.update(kind="polygon", pts=rings[0])
            if len(rings) > 1:
                o["holes"] = [{"pts": r} for r in rings[1:]]
        out.append(o)
    return out


def raster_case(id, title, tool, values, refname, dem, style, extra):
    """A tool that writes a raster beside the DEM: its file, object, layer and summary."""
    e = ref(refname)["expect"]["raster"]
    out = f"{dem}{SUFFIX[tool]}.tif"
    layer_name = LAYER[tool]
    expect = {
        "status": "ok",
        "undo": LABELS[tool],
        "summary": f"{words(e['width'])} × {words(e['height'])} hücrelik raster; “{out}” yazıldı.{extra}",
        "outputs": {"file": out},
        "layers": [new_layer(layer_name)],
        "added": [{"kind": "raster", "layerId": layer_id(layer_name), "attrs": {}, "affine": input_of(dem)["affine"], "width": e["width"],
                   "height": e["height"], "bands": 1, "sample": e["sample"], "file": out, "srid": 5254, "style": style}],
        "layerAbove": {layer_id(layer_name): dem},
        "hydrologyOf": {out: refname},
    }
    return {"id": id, "title": title, "document": "hydrology.kcad", "selection": [], "run": {"tool": tool},
            "values": {"input": {"scope": "layer", "layerId": dem}, **values}, "expect": expect}


def object_case(id, title, tool, values, refname, layer_name, summary, log=(), dem="ag"):
    """A tool that writes objects on a new layer right above the DEM's."""
    f = ref(refname)["expect"]["features"]
    lid = layer_id(layer_name)
    added = objects(f, lid)
    expect = {
        "status": "ok",
        "undo": LABELS[tool] if added else None,
        "summary": summary(f, added, ref(refname)["expect"]["notes"]),
        "outputs": {"count": len(added)},
        "layers": [output_layer(layer_name)] if added else [],
        "added": added,
    }
    if added:
        expect["layerAbove"] = {lid: dem}
    if log:
        expect["log"] = list(log)
    return {"id": id, "title": title, "document": "hydrology.kcad", "selection": [], "run": {"tool": tool},
            "values": {"input": {"scope": "layer", "layerId": dem}, **values}, "expect": expect}


def refused(id, title, tool, values, message):
    return {"id": id, "title": title, "document": "hydrology.kcad", "selection": [], "run": {"tool": tool}, "values": values,
            "expect": {"status": "error", "message": message}}


def filled_says(refname, depth):
    """Çukur doldur's words: the cells raised, the deepest fill (the reference's own 64-bit difference)."""
    cells = ref(refname)["expect"]["notes"]["cells"]
    if cells == 0:
        return " Doldurulacak çukur yok."
    s = f" {words(cells)} hücre dolduruldu."
    if depth:
        g = grid_of(refname.split("-")[0])
        f = hc.fill(g, 0.0)
        most = max(f[k] - g.z[k] for k in range(g.w * g.h) if g.valid[k])
        s += f" En derin dolgu {trimmed(most, 3)} m."
    return s


def direction_says(refname):
    notes = ref(refname.replace("-taudem", ""))["expect"]["notes"]
    s = ""
    if notes["cells"]:
        s += f" {words(notes['cells'])} düzlük hücresinin yönü verildi."
    if notes["empty"]:
        s += f" {words(notes['empty'])} hücre yönsüz kaldı: çukurlar ve çıkışsız düzlükler (Çukurları doldur açıkken kalmaz)."
    return s


def accumulation_says(dem, method, exponent, unit):
    """The largest sum, from the reference's own 64-bit sums."""
    g = grid_of(dem)
    _, _, acc = hc.accumulation(g, True, method, exponent, unit)
    most = max([a for a in acc if not math.isnan(a)] + [0.0])
    name = {"cells": "hücre", "area": "m²", "sca": "m"}[unit]
    return f" En büyük birikim {trimmed(most, 3)} {name}."


def skipped_warning(notes):
    return {"level": "warn", "text": f"{words(len(notes['skipped']))} nokta rasterin dışında ya da değersiz hücrede kaldığı için atlandı."}


def cases():
    points = {"points": {"scope": "layer", "layerId": "noktalar"}}
    out = [
        raster_case("fill", "Çukur doldur, en küçük eğim 0: vadinin altı hücrelik çukuru taşma yüksekliğine; DEM'in görünüşü kalır",
                    "hydrology.fill", {}, "vadi-doldur", "vadi", ARAZI, filled_says("vadi-doldur", False)),
        raster_case("fill-depth", "Çukur doldur, Dolgu derinliği: dolgunun derinliği ve en derin dolgu",
                    "hydrology.fill", {"result": "depth"}, "vadi-derinlik", "vadi", ramp("Viridis"), filled_says("vadi-derinlik", True)),
        raster_case("fill-slope", "Çukur doldur, en küçük eğim %0,5: doldurulan yüzey çıkışa doğru alçalır",
                    "hydrology.fill", {"slope": 0.5}, "vadi-doldur-egim", "vadi", ARAZI, filled_says("vadi-doldur-egim", False)),
        raster_case("fill-none", "Çukur doldur, çukursuz koni: hiçbir hücre değişmez",
                    "hydrology.fill", {}, "koni-doldur", "koni", ARAZI, filled_says("koni-doldur", False)),
        raster_case("direction", "Akış yönü, ESRI: vadinin dolgu düzlüğündeki altı hücrenin yönü Barnes'ın gradyanlarıyla",
                    "hydrology.flowDirection", {}, "vadi-yon", "vadi", coded(128), direction_says("vadi-yon")),
        raster_case("direction-unfilled", "Akış yönü, Çukurları doldur kapalı: çukurun dört hücresi yönsüz kalır",
                    "hydrology.flowDirection", {"fill": False}, "vadi-yon-dolgusuz", "vadi", coded(128), direction_says("vadi-yon-dolgusuz")),
        raster_case("direction-taudem", "Akış yönü, 1–8 (TauDEM): terasların 45 düzlük hücresi",
                    "hydrology.flowDirection", {"coding": "taudem"}, "teras-yon-taudem", "teras", coded(8), direction_says("teras-yon-taudem")),
        raster_case("accumulation", "Akış birikimi, D8, Alan: vadinin çıkışında bütün vadi",
                    "hydrology.flowAccumulation", {"unit": "area"}, "vadi-birikim-d8-area-0", "vadi", ACCUMULATION,
                    accumulation_says("vadi", "d8", 0.0, "area")),
        raster_case("accumulation-mfd", "Akış birikimi, Çoklu yön, uyarlanan üs, Hücre sayısı: teraslarda pay komşulara bölünür",
                    "hydrology.flowAccumulation", {"method": "mfd"}, "teras-birikim-mfd-cells-0", "teras", ACCUMULATION,
                    accumulation_says("teras", "mfd", 0.0, "cells")),
        raster_case("accumulation-dinf", "Akış birikimi, D∞, Özgül havza alanı",
                    "hydrology.flowAccumulation", {"method": "dinf", "unit": "sca"}, "vadi-birikim-dinf-sca-0", "vadi", ACCUMULATION,
                    accumulation_says("vadi", "dinf", 0.0, "sca")),
        raster_case("wetness", "Topografik nemlilik indisi, Çoklu yön, en küçük eğim %0,1: terasların düz basamaklarında taban eğim",
                    "hydrology.wetness", {}, "teras-twi-mfd-0.1", "teras", WETNESS,
                    f" {words(ref('teras-twi-mfd-0.1')['expect']['notes']['cells'])} hücrede en küçük eğim kullanıldı."),
        object_case("pour-points", "Döküm noktası, 25 m: noktalar derenin en büyük birikimli hücresine; rasterin dışındaki atlanır",
                    "hydrology.pourPoint", {**points, "snap": 25}, "ag-dokum-25", "Döküm noktaları",
                    lambda f, added, n: f"{words(len(added))} döküm noktası yazıldı.", log=[skipped_warning(ref("ag-dokum-25")["expect"]["notes"])]),
        object_case("watershed", "Noktadan havza, 25 m: aynı hücreye düşen iki noktadan öncekinin havzası boş, söylenir",
                    "hydrology.watershed", {**points, "snap": 25}, "ag-havza-25", "Havzalar",
                    lambda f, added, n: f"{words(len(added))} havza yazıldı.",
                    log=[skipped_warning(ref("ag-havza-25")["expect"]["notes"]),
                         {"level": "warn", "text": f"{words(len(ref('ag-havza-25')['expect']['notes']['empty']))} noktanın hücresini sonraki "
                          "bir nokta aldığı için havzası boş."}]),
        object_case("basins-main", "Havzalar, Ana havzalar, en küçük alan 601 m²: 600 m²'lik havza yazılmaz, kalan yeniden numaralanır",
                    "hydrology.basins", {"least": 601}, "vadi-ana-havzalar-601", "Havzalar",
                    lambda f, added, n: f"{words(len(added))} havza yazıldı. En küçük alandan küçük {words(n['dropped'])} havza yazılmadı.",
                    dem="vadi"),
        object_case("basins-sub", "Havzalar, Alt havzalar, eşik 3000 m²: yedi kolun havzaları, Bağ ve Sıra",
                    "hydrology.basins", {"mode": "sub", "threshold": 3000}, "ag-alt-havzalar-3000", "Havzalar",
                    lambda f, added, n: f"{words(len(added))} havza yazıldı."),
        object_case("basins-route", "Havzalar, Güzergâhı kesen dereler, eşik 3000 m², en küçük alan 20 000 m²: geçişlerin havzaları km'leriyle",
                    "hydrology.basins", {"mode": "route", "threshold": 3000, "routes": {"scope": "layer", "layerId": "guzergah"}, "least": 20000},
                    "ag-guzergah-20000", "Havzalar",
                    lambda f, added, n: f"{words(len(added))} havza yazıldı. En küçük alandan küçük {words(n['dropped'])} havza yazılmadı."),
        object_case("streams", "Dere ağı, eşik 3000 m²: yedi kol, Strahler ve Shreve, uzunluk, düşü, eğim",
                    "hydrology.streams", {"threshold": 3000}, "ag-dere-3000-0", "Dere ağı", streams_says),
        object_case("streams-default", "Dere ağı, eşik 0: en büyük birikimin yüzde biri (832 m²), sadeleştirme 1 hücre",
                    "hydrology.streams", {"simplify": 1}, "ag-dere-0-1", "Dere ağı", streams_says),
        refused("basins-route-missing", "Havzalar, Güzergâhı kesen dereler, güzergâh seçilmemiş", "hydrology.basins",
                {"input": {"scope": "layer", "layerId": "ag"}, "mode": "route", "threshold": 3000},
                "Güzergâhı seçin: çizgi, çoklu çizgi ya da yay."),
        refused("accumulation-exponent", "Akış birikimi, Çoklu yönün üssü 0,05", "hydrology.flowAccumulation",
                {"input": {"scope": "layer", "layerId": "vadi"}, "method": "mfd", "exponent": 0.05},
                "Çoklu yönün üssü 0 (uyarlanan) ya da 0,1 ile 100 arasında olmalı."),
    ]
    return out


def streams_says(f, added, notes):
    stride = len(f["fields"])
    order = f["fields"].index("Sıra")
    top = max(f["numbers"][k * stride + order] for k in range(len(added)))
    return f"{words(len(added))} kol yazıldı; en büyük Strahler sırası {trimmed(top, 0)}, eşik {trimmed(notes['threshold'], 3)} m²."


def defaults():
    one = {"scope": "selection"}
    here = {"scope": "layer", "layerId": "ag"}
    raster = lambda name: {"output": "", "add": True, "layer": {"newName": name}}
    return {
        "hydrology.fill": {"input": one, "band": 1, "slope": 0, "result": "filled", **raster("Doldurulmuş DEM")},
        "hydrology.flowDirection": {"input": one, "band": 1, "fill": True, "coding": "esri", **raster("Akış yönü")},
        "hydrology.flowAccumulation": {"input": one, "band": 1, "fill": True, "method": "d8", "exponent": 0, "unit": "cells",
                                       **raster("Akış birikimi")},
        "hydrology.wetness": {"input": one, "band": 1, "fill": True, "method": "mfd", "exponent": 0, "slope": 0.1, **raster("Nemlilik indisi")},
        "hydrology.pourPoint": {"input": one, "band": 1, "points": here, "fill": True, "snap": 0, "layer": {"newName": "Döküm noktaları"}},
        "hydrology.watershed": {"input": one, "band": 1, "points": here, "fill": True, "snap": 0, "layer": {"newName": "Havzalar"}},
        "hydrology.basins": {"input": one, "band": 1, "fill": True, "mode": "main", "threshold": 0, "routes": one, "least": 0,
                             "layer": {"newName": "Havzalar"}},
        "hydrology.streams": {"input": one, "band": 1, "fill": True, "threshold": 0, "simplify": 0, "layer": {"newName": "Dere ağı"}},
    }


def build():
    return {
        "format": "kentos.processing-cases",
        "version": 1,
        "tolerance": 1e-9,
        "rasters": {f: f"hydrology/{f}" for f, *_ in FILES},
        "documents": {
            "hydrology.kcad": {
                "defaults": {"lengthDecimals": 3, "areaDecimals": 2, "angleUnit": "grad", "plotScale": 1000, "drawingFont": "barlow",
                             "activeLayer": "ag", "measureHeightMm": 2},
                "tools": defaults(),
            }
        },
        "cases": cases(),
    }


def main():
    check = "--check" in sys.argv[1:]
    problems = []
    RASTERS.mkdir(parents=True, exist_ok=True)
    for file, _, _, lid, _ in FILES:
        path = RASTERS / file
        r = input_of(lid)
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
