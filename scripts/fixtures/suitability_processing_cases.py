#!/usr/bin/env python3
"""Uygunluk analizi'nin İşlemler durumları (docs/adr/0237; biçim fixtures/processing/README.md).

    python3 scripts/fixtures/suitability_processing_cases.py           # rasterleri, çizimi ve durumları yazar
    python3 scripts/fixtures/suitability_processing_cases.py --check   # hiçbir şey yazmaz; karşılaştırır

KentOS kodu kullanılmaz. Rasterler (fixtures/processing/v1/suitability/*.tif, GDAL'la) bağımsız başvurunun
(fixtures/suitability/v1/cases.json, scripts/fixtures/suitability_cases.py) girdileridir: üç ölçüt, üç sınıflanacak
ölçüt, bir değer rasteri, üç üyelik ve bir duyarlılık rasteri; çizim (fixtures/processing/v1/suitability.kcad) onları
katman katman bağlı raster olarak (Katmanlar panelindeki sırası araçların sırasıdır), varlık ve yokluk nesnelerini taşır.
Durumlar araçların kuralından yazılır: yazılan raster dosyasının değerleri başvurunun aynı adlı durumununkilerdir
(`suitabilityOf`, durumun kuralıyla; tam sayı sonucun nodata'sı değersizdir), ağırlıkların ve ROC'un tabloları ve
özetleri başvurunun sayılarından ADR 0149'un gösterim kuralıyla; sayılar binlik noktalı. Başvurunun durumlarındaki adlar
(ağırlıkların anahtarları) çizimin katmanlarının adlarına sırasıyla çevrilir: değerler addan bağımsızdır.
"""

import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import raster_ops_processing_cases as rop  # noqa: E402  (its GeoTIFF writer and reader, layer ids, thousands)
from numeric_display import shown  # noqa: E402  (the display rule's own reference)

DIR = ROOT / "fixtures/processing/v1"
RASTERS = DIR / "suitability"
DRAWING = DIR / "suitability.kcad"
CASES = DIR / "suitability.json"
REF = json.loads((ROOT / "fixtures/suitability/v1/cases.json").read_text("utf-8"))
RASTER_LAYER = "#7A6B5B"
words = rop.words
layer_id = rop.layer_id


def ref(name):
    return next(c for c in REF["cases"] if c["name"] == name)


def trimmed(v, d):
    t = shown(v, d)
    return t.rstrip("0").rstrip(".") if "." in t else t


# ── The rasters: the reference's inputs, each a file on its own layer (the panel's order, top first) ──

# File, layer id, layer name, (reference case, input index).
FILES = [
    ("egim.tif", "egim", "Eğim", ("agirlikli-toplam", 0)),
    ("yol.tif", "yol", "Yol", ("agirlikli-toplam", 1)),
    ("toprak.tif", "toprak", "Toprak", ("agirlikli-toplam", 2)),
    ("sinif.tif", "sinif", "Sınıf", ("cakistirma-tablolu", 0)),
    ("egim-sinifi.tif", "egim-sinifi", "Eğim sınıfı", ("cakistirma-tablolu", 1)),
    ("ortu.tif", "ortu", "Örtü", ("cakistirma-tablolu", 2)),
    ("deger.tif", "deger", "Değer", ("uyelik-dogrusal", 0)),
    ("uyelik-a.tif", "uyelik-a", "Üyelik A", ("bulanik-and", 0)),
    ("uyelik-b.tif", "uyelik-b", "Üyelik B", ("bulanik-and", 1)),
    ("uyelik-c.tif", "uyelik-c", "Üyelik C", ("bulanik-and", 2)),
    ("duyarlilik.tif", "duyarlilik", "Duyarlılık", ("roc-noktalar", 0)),
]
for _, _, _, (case, k) in FILES:
    assert case in [c["name"] for c in REF["cases"]]
assert ref("agirlikli-toplam")["inputs"] == ref("ahp-tutarli-raster")["inputs"]
assert ref("cakistirma-tablolu")["inputs"][0] == ref("cakistirma-tablosuz")["inputs"][0]
assert ref("bulanik-and")["inputs"] == ref("bulanik-gamma")["inputs"]
assert ref("uyelik-dogrusal")["inputs"] == ref("uyelik-gauss")["inputs"]


def raster_of(lid):
    case, k = next(f for f in FILES if f[1] == lid)[3]
    return ref(case)["inputs"][k]


def id_of(lid):
    """The raster's object id: rasters come first, in the files' order."""
    return 1 + [f[1] for f in FILES].index(lid)


# Object layers (above the rasters): ROC's presence and absence, as the reference's cases have them.
ROC_POINTS = ref("roc-noktalar")["shapes"]
YOKLUK = ref("roc-yokluk")
OBJECTS = [
    ("heyelan", "Heyelan", ROC_POINTS, "#C62828"),
    ("heyelan-bes", "Heyelan (beş)", YOKLUK["shapes"][:YOKLUK["tool"]["first"]], "#C62828"),
    ("yokluk", "Yokluk", YOKLUK["shapes"][YOKLUK["tool"]["first"]:], "#2E7D32"),
    ("uzak", "Uzak", ref("ret-roc-varlik-disarida")["shapes"], "#6A1B9A"),
]


# ── The drawing ──────────────────────────────────────────────────────────

ARAZI = {"render": "ramp", "bands": [1], "stretch": "minMax", "ramp": "Arazi"}


def drawing():
    def layer(id, name, color):
        return {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
                "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25}, "children": []}

    entities = []

    def add(e, lid, attrs=None):
        entities.append({**e, "id": len(entities) + 1, "layerId": lid, "attrs": attrs or {}})

    for file, lid, _, _ in FILES:
        r = raster_of(lid)
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
        "name": "İşlem durumları: uygunluk analizi",
        "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                     "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": 1000, "y": 2000},
        "layers": layers,
        "activeLayer": "heyelan",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }


# ── The tools' rules ─────────────────────────────────────────────────────

LABELS = {"suitability.fuzzyMembership": "Bulanık üyelik", "suitability.fuzzyOverlay": "Bulanık çakıştırma",
          "suitability.weightedSum": "Ağırlıklı toplam", "suitability.weightedOverlay": "Ağırlıklı çakıştırma",
          "suitability.pairwise": "İkili karşılaştırma (AHP)", "suitability.roc": "ROC ile doğrulama"}
SUFFIX = {"suitability.fuzzyMembership": "-uyelik", "suitability.fuzzyOverlay": "-bulanik", "suitability.weightedSum": "-agirlikli",
          "suitability.weightedOverlay": "-cakistirma", "suitability.pairwise": "-ahp"}
LAYER = {"suitability.fuzzyMembership": "Üyelik", "suitability.fuzzyOverlay": "Bulanık çakıştırma",
         "suitability.weightedSum": "Ağırlıklı toplam", "suitability.weightedOverlay": "Ağırlıklı çakıştırma",
         "suitability.pairwise": "Ağırlıklı toplam (AHP)"}


def ramp(stretch, lo=None, hi=None, resampling=None):
    """The look in the contract's field order (the web compares objects' texts)."""
    s = {"render": "ramp", "bands": [1], "stretch": stretch}
    if lo is not None:
        s["min"], s["max"] = lo, hi
    s["ramp"] = "Spektral"
    if resampling:
        s["resampling"] = resampling
    return s


MEMBERSHIP = ramp("manual", 0.0, 1.0)
SUM = ramp("minMax")


def new_layer(name):
    return {"id": layer_id(name), "name": name, "style": {"color": RASTER_LAYER, "lineType": "continuous", "lineWeight": 0.25}}


def on(lid):
    return {"scope": "layer", "layerId": lid}


SELECTION = {"scope": "selection"}


def renamed(m, names):
    """A reference case's map by its names, keyed by the drawing's names in the same order."""
    ref_names = names[0]
    return {names[1][ref_names.index(k)]: v for k, v in m.items()}


def raster_case(id, title, tool, values, refname, style, tail, *, inputs, sample="f32", log=(), outputs=None):
    """A tool that writes a raster: its file beside the first input, the object on a new layer right above that input's."""
    e = ref(refname)["expect"]
    r = e["raster"]
    first = inputs[0]
    out = f"{first}{SUFFIX[tool]}.tif"
    name = LAYER[tool]
    summary = f"{words(r['width'])} × {words(r['height'])} hücrelik raster; “{out}” yazıldı.{tail}"
    expect = {
        "status": "ok",
        "undo": LABELS[tool],
        "summary": summary,
        "outputs": {"file": out, **(outputs or {})},
        "layers": [new_layer(name)],
        "added": [{"kind": "raster", "layerId": layer_id(name), "attrs": {}, "affine": r["affine"], "width": r["width"],
                   "height": r["height"], "bands": 1, "sample": sample, "file": out, "srid": 5254, "style": style}],
        "layerAbove": {layer_id(name): first},
        "suitabilityOf": {out: refname},
    }
    if log:
        expect["log"] = list(log)
    vals = dict(values)
    selection = []
    if len(inputs) == 1:
        vals = {"input": on(first), **vals}
    else:
        vals = {"input": SELECTION, **vals}
        selection = [id_of(lid) for lid in inputs]
    return {"id": id, "title": title, "document": "suitability.kcad", "selection": selection, "run": {"tool": tool},
            "values": vals, "expect": expect}


def joined(n):
    return f" {words(n)} raster birleşti."


def consistency(p):
    verdict = "tutarsız" if p["cr"] > 0.1 else "tutarlı"
    return f"λ {shown(p['lambda'], 4)}; CI {shown(p['ci'], 4)}; RI {shown(p['ri'], 2)}; CR {shown(p['cr'], 4)} ({verdict})."


def weights_table(names, p):
    return {"columns": ["Ölçüt", "Ağırlık", "Yüzde (%)"],
            "rows": [[n, shown(w, 6), shown(100 * w, 2)] for n, w in zip(names, p["weights"])]}


def inconsistent(p):
    return [{"level": "warn", "text": f"Karşılaştırmalar tutarsız (CR {shown(p['cr'], 2)} > 0.10); en çelişkili çiftleri yeniden "
                                      "gözden geçirin."}] if p["cr"] > 0.1 else []


def pairs_renamed(pairs, names):
    ref_names, ours = names
    return [[ours[ref_names.index(a)], ours[ref_names.index(b)], v] for a, b, v in pairs]


def pairwise_table_case(id, title, refname, inputs):
    """İkili karşılaştırma without its raster: the weights' table; nothing written."""
    c = ref(refname)
    p = c["expect"]["pairwise"]
    names = [next(f[2] for f in FILES if f[1] == lid) for lid in inputs]
    expect = {
        "status": "ok",
        "undo": None,
        "summary": f"{words(len(names))} ölçütün ağırlıkları tabloda; {consistency(p)}",
        "outputs": {"table": weights_table(names, p)},
    }
    log = inconsistent(p)
    if log:
        expect["log"] = log
    return {"id": id, "title": title, "document": "suitability.kcad", "selection": [id_of(lid) for lid in inputs],
            "run": {"tool": "suitability.pairwise"},
            "values": {"input": SELECTION, "comparisons": pairs_renamed(c["tool"]["pairs"], (c["names"], names)), "write": False},
            "expect": expect}


def roc_case(id, title, refname, values):
    """ROC ile doğrulama: the curve's table and its summary; nothing written."""
    c = ref(refname)["expect"]["roc"]
    P, N = c["presence"], c["background"]
    fp_label, fp_share = ("Hücre", "Alan oranı (%)") if c["allCells"] else ("Yanlış pozitif", "Yanlış pozitif oranı (%)")
    rows = [[trimmed(t, 6), str(tp), shown(100 * tp / P, 2), str(fp), shown(100 * fp / N, 2)] for t, tp, fp in c["rows"]]
    log = []
    if c["skipped"]:
        log.append({"level": "warn", "text": f"Değersiz hücreye düşen {words(c['skipped'])} örnek atlandı."})
    if c["outside"]:
        log.append({"level": "warn", "text": f"Rasterin dışında kalan {words(c['outside'])} nokta atlandı."})
    if c["both"]:
        log.append({"level": "warn", "text": f"{words(c['both'])} hücre hem varlık hem yokluk; ikisinde de sayıldı."})
    other = "değerli hücre" if c["allCells"] else "yokluk hücresi"
    s = f"AUC {shown(c['auc'], 4)} ({words(P)} varlık hücresi, {words(N)} {other})."
    if c["best"] is not None:
        t, tp, fp = c["rows"][c["best"]]
        share = "alan" if c["allCells"] else "yanlış pozitif"
        s += f" En iyi eşik {trimmed(t, 6)} (doğru pozitif %{shown(100 * tp / P, 1)}, {share} %{shown(100 * fp / N, 1)})."
    expect = {"status": "ok", "undo": None, "summary": s,
              "outputs": {"table": {"columns": ["Eşik", "Doğru pozitif", "Doğru pozitif oranı (%)", fp_label, fp_share], "rows": rows},
                          "auc": c["auc"]}}
    if log:
        expect["log"] = log
    return {"id": id, "title": title, "document": "suitability.kcad", "selection": [], "run": {"tool": "suitability.roc"},
            "values": {"input": on("duyarlilik"), **values}, "expect": expect}


def refused(id, title, tool, values, message, selection=()):
    return {"id": id, "title": title, "document": "suitability.kcad", "selection": list(selection), "run": {"tool": tool},
            "values": values, "expect": {"status": "error", "message": message}}


def invalid_warn(n):
    return {"level": "warn", "text": f"{words(n)} hücrede üyelik 0–1 aralığının dışında; o hücreler değersiz bırakıldı."}


def cases():
    M, O, S, W, P = ("suitability.fuzzyMembership", "suitability.fuzzyOverlay", "suitability.weightedSum",
                     "suitability.weightedOverlay", "suitability.pairwise")
    sums = (["Eğim", "Yol", "Toprak"], ["Eğim", "Yol", "Toprak"])
    overlay_names = (ref("cakistirma-tablolu")["names"], ["Sınıf", "Eğim sınıfı", "Örtü"])
    tab = ref("cakistirma-tablolu")["tool"]
    tab_notes = ref("cakistirma-tablolu")["expect"]["notes"]
    overlay_log = []
    if tab_notes["unmatched"]:
        overlay_log.append({"level": "warn", "text": f"Sınıf tablosunun hiçbir kuralının tutmadığı {words(tab_notes['unmatched'])} hücre "
                                                     "değersiz bırakıldı."})
    if tab_notes["outside"]:
        overlay_log.append({"level": "warn", "text": f"Ölçeğin dışında (ya da tam sayı olmayan) değerli {words(tab_notes['outside'])} "
                                                     "hücre değersiz bırakıldı."})
    ahp = ref("ahp-tutarli-raster")
    ahp_p = ahp["expect"]["pairwise"]
    gamma_invalid = ref("bulanik-gamma")["expect"]["notes"]["invalid"]
    return [
        raster_case("membership-linear", "Bulanık üyelik, Doğrusal 0–20; katman rasterin hemen üstünde", M,
                    {"function": "linear", "low": 0, "high": 20}, "uyelik-dogrusal", MEMBERSHIP, "", inputs=["deger"]),
        raster_case("membership-gauss", "Bulanık üyelik, Gauss: orta nokta 10, yayılım 0,05", M,
                    {"function": "gaussian", "midpoint": 10, "spread": 0.05}, "uyelik-gauss", MEMBERSHIP, "", inputs=["deger"]),
        raster_case("membership-large", "Bulanık üyelik, Büyük: orta nokta 10, diklik 5; 0 ve altı 0", M,
                    {"function": "large", "midpoint": 10, "steep": 5}, "uyelik-buyuk", MEMBERSHIP, "", inputs=["deger"]),
        raster_case("fuzzy-and", "Bulanık çakıştırma, Ve: üç üyelik; 0–1 dışındaki üyelik söylenir", O, {"op": "and"},
                    "bulanik-and", MEMBERSHIP, joined(3), inputs=["uyelik-a", "uyelik-b", "uyelik-c"],
                    log=[invalid_warn(ref("bulanik-and")["expect"]["notes"]["invalid"])]),
        raster_case("fuzzy-gamma", "Bulanık çakıştırma, Gamma 0,9", O, {"op": "gamma", "gamma": 0.9}, "bulanik-gamma", MEMBERSHIP,
                    joined(3), inputs=["uyelik-a", "uyelik-b", "uyelik-c"], log=[invalid_warn(gamma_invalid)]),
        raster_case("weighted-sum", "Ağırlıklı toplam: ağırlıklar 0,5, −0,02, 1,25", S,
                    {"weights": renamed(ref("agirlikli-toplam")["tool"]["weights"], sums)}, "agirlikli-toplam", SUM, joined(3),
                    inputs=["egim", "yol", "toprak"]),
        raster_case("weighted-sum-default", "Ağırlıklı toplam: yazılmayan ağırlıklar 1", S,
                    {"weights": renamed(ref("agirlikli-toplam-varsayilan")["tool"]["weights"], sums)}, "agirlikli-toplam-varsayilan",
                    SUM, joined(3), inputs=["egim", "yol", "toprak"]),
        raster_case("weighted-overlay", "Ağırlıklı çakıştırma 1–9: etkiler %33,3333, %33,3333, %33,3334; iki sınıf tablosu, kısıt ve boş",
                    W, {"low": 1, "high": 9, "influence": renamed(tab["influence"], overlay_names),
                        "classes": renamed(tab["classes"], overlay_names), "bounds": tab["bounds"]},
                    "cakistirma-tablolu", ramp("manual", 0.0, 9.0, "nearest"),
                    f"{joined(3)} Kısıtlı {words(tab_notes['restricted'])} hücre.", inputs=["sinif", "egim-sinifi", "ortu"],
                    sample="i32", log=overlay_log),
        raster_case("pairwise-raster", "İkili karşılaştırma: tutarlı üç ölçüt, ağırlıklı toplam yazılır", P,
                    {"comparisons": pairs_renamed(ahp["tool"]["pairs"], (ahp["names"], ["Eğim", "Yol", "Toprak"])), "write": True},
                    "ahp-tutarli-raster", SUM, f" Ağırlıklar tabloda; {consistency(ahp_p)}", inputs=["egim", "yol", "toprak"],
                    outputs={"table": weights_table(["Eğim", "Yol", "Toprak"], ahp_p)}),
        pairwise_table_case("pairwise-four", "İkili karşılaştırma, dört ölçüt, yalnız ağırlıklar", "ahp-dort",
                            ["sinif", "egim-sinifi", "ortu", "deger"]),
        pairwise_table_case("pairwise-inconsistent", "İkili karşılaştırma, döngülü karşılaştırmalar: tutarsız, söylenir", "ahp-tutarsiz",
                            ["sinif", "egim-sinifi", "ortu"]),
        roc_case("roc-cells", "ROC, bütün hücreler: noktalar; değersiz hücredeki ve rasterin dışındaki söylenir", "roc-noktalar",
                 {"presence": on("heyelan")}),
        roc_case("roc-absence", "ROC, yokluk nesneleriyle: hem varlık hem yokluk olan hücreler söylenir", "roc-yokluk",
                 {"presence": on("heyelan-bes"), "background": "absence", "absence": on("yokluk")}),
        roc_case("roc-lower", "ROC, düşük değer daha olası", "roc-dusuk-olasi", {"presence": on("heyelan"), "higher": False}),
        refused("weighted-overlay-sum", "Ağırlıklı çakıştırma, etkilerin toplamı 90: ret", W,
                {"input": SELECTION, "influence": {"Sınıf": 50, "Eğim sınıfı": 30, "Örtü": 10}},
                "Etkilerin toplamı 100 olmalı; şimdi 90.", [id_of("sinif"), id_of("egim-sinifi"), id_of("ortu")]),
        refused("weighted-overlay-missing", "Ağırlıklı çakıştırma, bir rasterin etkisi yazılmamış: ret", W,
                {"input": SELECTION, "influence": {"Sınıf": 50, "Eğim sınıfı": 50}},
                "Örtü: etkisi yazılmadı.", [id_of("sinif"), id_of("egim-sinifi"), id_of("ortu")]),
        refused("pairwise-one", "İkili karşılaştırma, tek raster: ret", P, {"input": on("egim"), "write": False},
                "İkili karşılaştırma 2 ile 15 arasında ölçüt (raster) ister; 1 raster seçili."),
        refused("membership-two", "Bulanık üyelik, iki raster: ret", M, {"input": SELECTION},
                "2 raster seçili; bu araç tek raster ister.", [id_of("egim"), id_of("yol")]),
        refused("fuzzy-one", "Bulanık çakıştırma, tek raster: ret", O, {"input": on("uyelik-a")},
                "Bulanık çakıştırma en az iki raster ister."),
        refused("roc-off", "ROC, varlık rasterin dışında: ret", "suitability.roc", {"input": on("duyarlilik"), "presence": on("uzak")},
                "Rasterin değerli hücrelerine düşen varlık yok: varlık nesneleri rasterin üstünde olmalı."),
        {"id": "weighted-sum-bounds", "title": "Ağırlıklı toplam, sınırın dışında ağırlık: çalışmaz, alanın sorunu söylenir",
         "document": "suitability.kcad", "selection": [id_of("egim"), id_of("yol")], "run": {"tool": S},
         "values": {"input": SELECTION, "weights": {"Eğim": 2e9}},
         "expect": {"status": "invalid", "issues": [{"param": "weights",
                                                      "message": "“Ağırlıklar”: Eğim -1000000000 ile 1000000000 arasında olmalı."}]}},
    ]


def defaults():
    one = {"scope": "selection"}
    here = {"scope": "layer", "layerId": "heyelan"}
    visible = {"scope": "visible"}
    raster = lambda name: {"output": "", "add": True, "layer": {"newName": name}}
    return {
        "suitability.fuzzyMembership": {"input": one, "band": 1, "function": "linear", "low": 0, "high": 100, "exponent": 2, "midpoint": 1,
                                        "spread": 0.1, "steep": 5, "sample": "f32", **raster("Üyelik")},
        "suitability.fuzzyOverlay": {"input": visible, "band": 1, "op": "and", "gamma": 0.9, "sample": "f32",
                                     **raster("Bulanık çakıştırma")},
        "suitability.weightedSum": {"input": visible, "band": 1, "weights": {}, "sample": "f32", **raster("Ağırlıklı toplam")},
        "suitability.weightedOverlay": {"input": visible, "band": 1, "low": 1, "high": 9, "influence": {}, "classes": {},
                                        "bounds": "upperClosed", **raster("Ağırlıklı çakıştırma")},
        "suitability.pairwise": {"input": visible, "comparisons": [], "write": True, "band": 1, "sample": "f32",
                                 **raster("Ağırlıklı toplam (AHP)")},
        "suitability.roc": {"input": one, "band": 1, "presence": here, "background": "cells", "absence": here, "higher": True},
    }


def build():
    return {
        "format": "kentos.processing-cases",
        "version": 1,
        "tolerance": 1e-9,
        "rasters": {f: f"suitability/{f}" for f, *_ in FILES},
        "documents": {
            "suitability.kcad": {
                "defaults": {"lengthDecimals": 3, "areaDecimals": 2, "angleUnit": "grad", "plotScale": 1000, "drawingFont": "barlow",
                             "activeLayer": "heyelan", "measureHeightMm": 2},
                "tools": defaults(),
            }
        },
        "cases": cases(),
    }


def main():
    check = "--check" in sys.argv[1:]
    problems = []
    RASTERS.mkdir(parents=True, exist_ok=True)
    for file, lid, _, _ in FILES:
        path = RASTERS / file
        r = raster_of(lid)
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
