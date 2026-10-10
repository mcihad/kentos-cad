#!/usr/bin/env python3
"""Mekânsal istatistik'in İşlemler durumları (docs/adr/0238): the nine tools run on a drawing, in the processing cases' format
(fixtures/processing/README.md). What each run writes and says comes from the independent reference
(scripts/fixtures/spatial_stats_cases.py: exact places, 40-digit mpmath, no KentOS code).

    python3 scripts/fixtures/spatial_stats_processing_cases.py          # spatial-stats.json ve çizimlerini yazar
    python3 scripts/fixtures/spatial_stats_processing_cases.py --check  # hiçbir şey yazmaz; karşılaştırır

The drawing: Kazalar (three groups of accidents and a few apart; No, Ağırlık with an unreadable and a missing weight, Tür),
Parseller (a 6 × 5 block of 20 m parcels with a Değer rising to the north-east and one unreadable), an empty Çizim layer
(active). A second drawing in WGS 84 for the refusal on geographic coordinates. New objects are compared within 10⁻⁶ m
(their coordinates, radii and axes; the ellipse's ratio within the same), attributes, colours and texts exactly.
"""

import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import spatial_stats_cases as ref  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures" / "processing" / "v1"
CASES = DIR / "spatial-stats.json"
DRAWING = DIR / "spatial-stats.kcad"
GEO = DIR / "spatial-stats-geo.kcad"
E, N = ref.E, ref.N


def style(color, weight=0.25, **more):
    return {"color": color, "lineType": "continuous", "lineWeight": weight, **more}


def layer(i, name, st):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True, "style": st, "children": []}


def layer_id(name):
    """A new layer's id as the runner makes it: “islem-” and the name folded, lower case, other characters as “-”."""
    fold = str.maketrans("çğıöşüÇĞİÖŞÜâîû", "cgiosuCGIOSUaiu")
    out, dash = "", False
    for ch in name.translate(fold).lower():
        if "a" <= ch <= "z" or "0" <= ch <= "9":
            out += ch
            dash = False
        elif not dash:
            out += "-"
            dash = True
    return "islem-" + out


def new_layer(name, color):
    st = {"color": color, "lineType": "continuous", "lineWeight": 0.25, "fill": color + "26"}
    if name in ("Ortalama merkez", "Ortanca merkez"):
        st["point"] = {"symbol": "cross", "size": 12}
    if name == "Sıcak noktalar":
        st["renderer"] = HOT_RENDERER
    if name.startswith("Kümeler"):
        st["renderer"] = CLUSTER_RENDERER
    return {"id": layer_id(name), "name": name, "style": st}


HOT_CLASSES = [("3", "Sıcak nokta, %99 güven", "#B2182B"), ("2", "Sıcak nokta, %95 güven", "#EF8A62"), ("1", "Sıcak nokta, %90 güven", "#FDDBC7"),
               ("0", "Anlamlı değil", "#D9D9D9"), ("-1", "Soğuk nokta, %90 güven", "#D1E5F0"), ("-2", "Soğuk nokta, %95 güven", "#67A9CF"),
               ("-3", "Soğuk nokta, %99 güven", "#2166AC")]
HOT_RENDERER = {"type": "categorized", "expr": "[Güven sınıfı]", "categories": [
    {"value": v, "label": label, "symbols": {
        "fill": {"type": "fill", "layers": [{"id": "f", "type": "simpleFill", "color": c},
                                            {"id": "o", "type": "simpleLine", "color": "#FFFFFF", "width": 0.13, "unit": "mm"}]},
        "line": {"type": "line", "layers": [{"id": "l", "type": "simpleLine", "color": c, "width": 0.6, "unit": "mm", "cap": "round", "join": "round"}]},
        "marker": {"type": "marker", "layers": [{"id": "p", "type": "shape", "shape": "circle", "size": 8, "unit": "px", "fill": c, "stroke": "#FFFFFF",
                                                 "strokeWidth": 0.8}]}}}
    for v, label, c in HOT_CLASSES]}


# ── The drawing ───────────────────────────────────────────────────────

rng = ref.Lcg(4242)
KAZA = []  # (entity, place)
groups = [("Yaya", 20, 30, 5), ("Araç", 110, 40, 6), ("Araç", 60, 120, 4)]
k = 0
for tur, cx, cy, sigma in groups:
    for _ in range(8):
        k += 1
        e, p = ref.pt(cx + rng.normalish(sigma), cy + rng.normalish(sigma))
        w = str(1 + (k * 7) % 4)
        if k == 5:
            w = "bilinmiyor"
        attrs = {"No": f"K{k}", "Ağırlık": w}
        if k != 9:
            attrs["Tür"] = tur
        KAZA.append(({**e, "attrs": attrs}, p))
for x, y in [(170, 10), (5, 150), (150, 150)]:
    k += 1
    e, p = ref.pt(x, y)
    KAZA.append(({**e, "attrs": {"No": f"K{k}", "Ağırlık": "1", "Tür": "Bisiklet"}}, p))
# One weight missing altogether.
KAZA[11][0]["attrs"].pop("Ağırlık")

PARSEL = []
for j in range(5):
    for i in range(6):
        e, p = ref.poly((i * 20, 200 + j * 20), (i * 20 + 20, 200 + j * 20), (i * 20 + 20, 220 + j * 20), (i * 20, 220 + j * 20))
        v = 1000 + 120 * i + 90 * j + ((i * 5 + j * 3) % 7) * 15
        text = f"{v}.00" if (i + j) % 3 else str(v)
        if (i, j) == (2, 1):
            text = "bilinmiyor"
        PARSEL.append(({**e, "attrs": {"Ada": str(100 + j), "Parsel": str(i + 1), "Değer": text}}, p))

ENTITIES = []
for lid, objs in (("kaza", KAZA), ("parsel", PARSEL)):
    for e, _ in objs:
        ENTITIES.append({"kind": e["kind"], "id": len(ENTITIES) + 1, "layerId": lid, "attrs": e["attrs"],
                         **{k2: v for k2, v in e.items() if k2 not in ("kind", "attrs")}})
IDS = {lid: [x["id"] for x in ENTITIES if x["layerId"] == lid] for lid in ("kaza", "parsel")}

SETTINGS = {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
            "workspace": "gis", "drawingFont": "barlow"}
LAYERS = [layer("cizim", "Çizim", style("fg")), layer("kaza", "Kazalar", style("#E5484D", point={"symbol": "ring", "size": 5})),
          layer("parsel", "Parseller", style("#3E63DD", 0.35, fill="#3E63DD1F"))]


def document(name, settings, entities):
    return {"format": "kentos.document", "version": 1, "name": name, "settings": settings, "origin": {"x": E, "y": N},
            "layers": LAYERS, "activeLayer": "cizim", "entities": entities, "styles": {"items": [], "categories": []}}


GEO_ENTITIES = [{"kind": "point", "id": i + 1, "layerId": "kaza", "attrs": {"No": f"G{i + 1}"}, "p": {"x": 32.5 + i * 0.01, "y": 39.9 + i * 0.005}}
                for i in range(4)]


# ── Cases ─────────────────────────────────────────────────────────────

LAY = lambda i: {"scope": "layer", "layerId": i}  # noqa: E731


def objs_of(lid):
    return KAZA if lid == "kaza" else PARSEL


def entity_of(lid, k2):
    return next(x for x in ENTITIES if x["id"] == IDS[lid][k2])


def log_of(run):
    return [{"level": "info", "text": t} for t in run["infos"]] + [{"level": "warn", "text": t} for t in run["warnings"]]


def xy(v):
    return {"x": v[0], "y": v[1]}


def added_objects(run, lname):
    out = []
    for o in run.get("objects", []):
        e = {"kind": o["kind"], "layerId": layer_id(lname), "attrs": dict(o["attrs"])}
        if o["kind"] == "point":
            e["p"] = xy(o["p"])
        elif o["kind"] == "circle":
            e["c"], e["r"] = xy(o["c"]), o["r"]
        else:
            e.update({"c": xy(o["c"]), "major": xy(o["major"]), "ratio": o["ratio"], "t0": 0, "t1": 2 * math.pi})
        out.append(e)
    return out


def added_copies(run, lid, lname):
    out = []
    for c in run.get("copies", []):
        src = entity_of(lid, c["index"])
        attrs = dict(src["attrs"])
        for kk, v in c["attrs"]:
            attrs[kk] = v
        e = {k2: v for k2, v in src.items() if k2 not in ("id", "layerId", "attrs")}
        e.update({"layerId": layer_id(lname), "attrs": attrs, "color": c["color"]})
        out.append(e)
    return out


def ok_case(cid, title, tool, lid, values, run, lname=None, color=None, undo=None, outputs_extra=()):
    exp = {"status": "ok", "summary": run["summary"], "undo": undo, "log": log_of(run)}
    if lname:
        exp["layers"] = [new_layer(lname, color)]
        exp["layerAbove"] = {layer_id(lname): lid}
        added = added_objects(run, lname) + added_copies(run, lid, lname)
        exp["added"] = added
        exp["outputs"] = {"count": len(added)}
    else:
        exp["outputs"] = {}
    if "table" in run:
        exp["outputs"]["table"] = run["table"]
    for name in outputs_extra:
        exp["outputs"][name] = run["numbers"][name]
    return {"id": cid, "title": title, "document": "spatial-stats.kcad", "run": {"tool": tool}, "values": {"input": LAY(lid), **values},
            "expect": exp}


def attr_list(lid, name):
    return [e["attrs"].get(name) for e, _ in objs_of(lid)]


def cases():
    out = []
    kz = KAZA
    run = ref.centers(kz, "mean")
    out.append(ok_case("mean", "Ortalama merkez: Kazalar'ın 27 nesnesinin merkezi", "stats.meanCenter", "kaza", {}, run,
                       "Ortalama merkez", "#D6336C", "Ortalama merkez"))
    run = ref.centers(kz, "mean", attr_list("kaza", "Ağırlık"), attr_list("kaza", "Tür"), "Ağırlık")
    out.append(ok_case("mean-weighted-grouped", "Ortalama merkez, Ağırlık ve Tür'e göre: okunamayan ve olmayan ağırlık söylenir, Tür'ü olmayan (boş) grubunda",
                       "stats.meanCenter", "kaza", {"weightField": "Ağırlık", "groupField": "Tür"}, run, "Ortalama merkez", "#D6336C", "Ortalama merkez"))
    run = ref.centers(kz, "median", None, attr_list("kaza", "Tür"))
    out.append(ok_case("median-grouped", "Ortanca merkez, Tür'e göre", "stats.medianCenter", "kaza", {"groupField": "Tür"}, run,
                       "Ortanca merkez", "#7048E8", "Ortanca merkez"))
    run = ref.centers(kz, "distance", k=2)
    out.append(ok_case("distance-two", "Standart uzaklık, 2 standart sapma", "stats.standardDistance", "kaza", {"deviations": "2"}, run,
                       "Standart uzaklık", "#1C7ED6", "Standart uzaklık"))
    run = ref.centers(kz, "ellipse", None, attr_list("kaza", "Tür"))
    out.append(ok_case("ellipse-grouped", "Yön dağılımı, Tür'e göre: Bisiklet'in üç yeri de elips verir, (boş) grubunun tek yeri vermez",
                       "stats.directionalDistribution", "kaza", {"groupField": "Tür"}, run, "Yön dağılımı", "#0CA678", "Yön dağılımı"))
    run = ref.nearest(kz)
    out.append(ok_case("nearest", "En yakın komşu: kümelenmiş, tablo; çizim değişmez", "stats.nearestNeighbor", "kaza", {}, run))
    run = ref.nearest(kz, 250000)
    out.append(ok_case("nearest-area", "En yakın komşu, 250 000 m²'lik alanla", "stats.nearestNeighbor", "kaza", {"area": 250000}, run))
    values = attr_list("parsel", "Değer")
    run = ref.morans_i(PARSEL, values, "Değer")
    out.append(ok_case("moran-band", "Moran I, kendiliğinden bant: kümelenmiş; okunamayan değer söylenir", "stats.moransI", "parsel",
                       {"valueField": "Değer"}, run))
    run = ref.morans_i(PARSEL, values, "Değer", "nearest", None, 4, False)
    out.append(ok_case("moran-nearest", "Moran I, 4 en yakın komşu, satır standartlaştırmasız", "stats.moransI", "parsel",
                       {"valueField": "Değer", "concept": "nearest", "neighbors": 4, "standardize": False}, run))
    run = ref.hot_spots(PARSEL, values, "Değer")
    out.append(ok_case("hot-band", "Sıcak nokta, kendiliğinden bant: parsellerin kopyaları z, p, güven sınıfı ve sınıfın rengiyle",
                       "stats.hotSpot", "parsel", {"valueField": "Değer"}, run, "Sıcak noktalar", "#E03131", "Sıcak nokta (Gi*)",
                       ("hot", "cold")))
    run = ref.hot_spots(kz, attr_list("kaza", "Ağırlık"), "Ağırlık", "nearest", None, 5)
    out.append(ok_case("hot-nearest", "Sıcak nokta, kazaların ağırlıkları, 5 en yakın komşu", "stats.hotSpot", "kaza",
                       {"valueField": "Ağırlık", "concept": "nearest", "neighbors": 5}, run, "Sıcak noktalar", "#E03131", "Sıcak nokta (Gi*)",
                       ("hot", "cold")))
    run = ref.dbscan(kz, F(15, 2), 4)
    out.append(ok_case("dbscan", "DBSCAN, 7,5 m ve 4 nokta: üç küme ve gürültü", "stats.dbscan", "kaza", {"radius": 7.5, "minPoints": 4}, run,
                       "Kümeler (DBSCAN)", "#F08C00", "DBSCAN kümeleme", ("clusters",)))
    run = ref.dbscan(kz, F(15, 2), 4, True)
    out.append(ok_case("dbscan-star", "DBSCAN*: sınır noktaları gürültü", "stats.dbscan", "kaza", {"radius": 7.5, "minPoints": 4, "borderNoise": True},
                       run, "Kümeler (DBSCAN)", "#F08C00", "DBSCAN kümeleme", ("clusters",)))
    run = ref.k_means(kz, 3)
    out.append(ok_case("kmeans", "k-ortalamalar, 3 küme", "stats.kMeans", "kaza", {"clusters": 3}, run, "Kümeler (k-ortalamalar)", "#F08C00",
                       "k-ortalamalar kümeleme", ("clusters",)))
    out.append({"id": "geographic", "title": "Coğrafi koordinatlı projede ret", "document": "spatial-stats-geo.kcad",
                "run": {"tool": "stats.meanCenter"}, "values": {"input": LAY("kaza")},
                "expect": {"status": "error", "message": "Mekânsal istatistik projeksiyonlu koordinat ister: projenin sistemi coğrafi."}})
    sel = IDS["parsel"][:3]
    few = ref.morans_i(PARSEL[:3], values[:3], "Değer")
    out.append({"id": "moran-few", "title": "Moran I, üç parsel: ret", "document": "spatial-stats.kcad", "selection": sel,
                "run": {"tool": "stats.moransI"}, "values": {"input": {"scope": "selection"}, "valueField": "Değer"},
                "expect": {"status": "error", "message": few["refused"]}})
    many = ref.k_means(kz, 50)
    out.append({"id": "kmeans-many", "title": "k-ortalamalar, 50 küme: ret", "document": "spatial-stats.kcad", "run": {"tool": "stats.kMeans"},
                "values": {"input": LAY("kaza"), "clusters": 50}, "expect": {"status": "error", "message": many["refused"]}})
    out.append({"id": "moran-no-field", "title": "Moran I, değer alanı seçilmemiş", "document": "spatial-stats.kcad", "run": {"tool": "stats.moransI"},
                "values": {"input": LAY("parsel")},
                "expect": {"status": "invalid", "issues": [{"param": "valueField", "message": "“Değer alanı”: bir alan adı seçin."}]}})
    return out


def defaults():
    here = LAY("cizim")
    center = lambda name: {"input": here, "weightField": "", "groupField": "", "layer": {"newName": name}}  # noqa: E731
    return {
        "stats.meanCenter": center("Ortalama merkez"),
        "stats.medianCenter": center("Ortanca merkez"),
        "stats.standardDistance": {**center("Standart uzaklık"), "deviations": "1"},
        "stats.directionalDistribution": {**center("Yön dağılımı"), "deviations": "1"},
        "stats.nearestNeighbor": {"input": here, "area": None},
        "stats.moransI": {"input": here, "valueField": "", "concept": "band", "band": None, "neighbors": 8, "standardize": True},
        "stats.hotSpot": {"input": here, "valueField": "", "concept": "band", "band": None, "neighbors": 8, "layer": {"newName": "Sıcak noktalar"}},
        "stats.dbscan": {"input": here, "radius": 50, "minPoints": 5, "borderNoise": False, "layer": {"newName": "Kümeler (DBSCAN)"}},
        "stats.kMeans": {"input": here, "clusters": 5, "layer": {"newName": "Kümeler (k-ortalamalar)"}},
    }


DEFAULTS = {"lengthDecimals": 3, "areaDecimals": 2, "angleUnit": "grad", "plotScale": 1000, "drawingFont": "barlow", "activeLayer": "cizim",
            "measureHeightMm": 2}


def build():
    return {
        "format": "kentos.processing-cases",
        "version": 1,
        "tolerance": 1e-6,
        "documents": {
            "spatial-stats.kcad": {"defaults": DEFAULTS, "tools": defaults()},
            "spatial-stats-geo.kcad": {"defaults": DEFAULTS, "tools": {}},
        },
        "cases": cases(),
    }



def cluster_symbols(c):
    return {"fill": {"type": "fill", "layers": [{"id": "f", "type": "simpleFill", "color": c},
                                                 {"id": "o", "type": "simpleLine", "color": "#FFFFFF", "width": 0.13, "unit": "mm"}]},
            "line": {"type": "line", "layers": [{"id": "l", "type": "simpleLine", "color": c, "width": 0.6, "unit": "mm", "cap": "round", "join": "round"}]},
            "marker": {"type": "marker", "layers": [{"id": "p", "type": "shape", "shape": "circle", "size": 7, "unit": "px", "fill": c, "stroke": "#FFFFFF",
                                                     "strokeWidth": 0.8}]}}


CLUSTER_RENDERER = {"type": "categorized", "expr": "([Küme] - 1) % 10", "categories": [
    {"value": str(k), "label": f"Küme {k + 1}, {k + 11}, {k + 21} …", "symbols": cluster_symbols(c)} for k, c in enumerate(ref.CLUSTER_COLORS)]
    + [{"value": "-1", "label": "Gürültü", "symbols": cluster_symbols(ref.NOISE_COLOR)}]}


def main():
    texts = {
        DRAWING: json.dumps(document("İşlem durumları: mekânsal istatistik", SETTINGS, ENTITIES), ensure_ascii=False, indent=1) + "\n",
        GEO: json.dumps(document("İşlem durumları: coğrafi proje", {**SETTINGS, "srid": 4326}, GEO_ENTITIES), ensure_ascii=False, indent=1) + "\n",
        CASES: json.dumps(build(), ensure_ascii=False, indent=1) + "\n",
    }
    if "--check" in sys.argv[1:]:
        bad = [p for p, t in texts.items() if not p.exists() or p.read_text("utf-8") != t]
        for p in bad:
            print(f"{p.relative_to(ROOT)} güncel değil; betiği --check olmadan çalıştırıp farkı okuyun.")
        if bad:
            return 1
        print("fixtures/processing/v1/spatial-stats.*: güncel.")
        return 0
    for p, t in texts.items():
        p.write_text(t, "utf-8")
        print(f"yazıldı: {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

