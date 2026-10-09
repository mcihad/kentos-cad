#!/usr/bin/env python3
"""İnterpolasyon ve yoğunluğun İşlemler durumları (docs/adr/0232 §13; biçim fixtures/processing/README.md).

    python3 scripts/fixtures/interpolation_processing_cases.py           # çizimi ve durumları yazar
    python3 scripts/fixtures/interpolation_processing_cases.py --check   # hiçbir şey yazmaz; karşılaştırır

KentOS kodu kullanılmaz. Çizim (fixtures/processing/v1/interpolation.kcad) interpolasyonun bağımsız başvurusunun
(fixtures/interpolation/v1/cases.json, scripts/fixtures/interpolation_cases.py) noktalarını, alan değerlerini,
yoğunluk noktalarını ve çizgilerini katman katman taşır; bir de ızgara rasteri (dosyası okunmaz), doğru üzerinde
noktalar ve kotsuz noktalar. Durumlar araçların kuralından yazılır: sonucun adı (girdinin katmanının adı ve aracın
eki), özet (sayılar ADR 0149'un gösterim kuralıyla, scripts/fixtures/numeric_display.py), yeni katmanların adı ve
stili, raster nesnelerinin alanları, çapraz doğrulama tablosu. Yazılan dosyanın değerleri başvurunun adı verilen
durumununkilerdir (`interpolationOf`: dosyanın 0. katı; iki bantta ikincisi durumun `error`'u); yeni katman girdinin
katmanının hemen altındadır (`layerBelow`; Kriging'in hata katmanı tahminin katmanının altında).
"""

import json
import re
import sys
import unicodedata
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
from numeric_display import shown  # noqa: E402  (the display rule's own reference)

DIR = ROOT / "fixtures/processing/v1"
DRAWING = DIR / "interpolation.kcad"
CASES = DIR / "interpolation.json"
REF = json.loads((ROOT / "fixtures/interpolation/v1/cases.json").read_text("utf-8"))
RASTER_LAYER = "#7A6B5B"


def ref(name):
    return next(c for c in REF["cases"] if c["name"] == name)


def words(n):
    return re.sub(r"\B(?=(\d{3})+(?!\d))", ".", str(n))


def layer_id(name):
    folded = name.translate(str.maketrans("çğıöşüÇĞİÖŞÜ", "cgiosuCGIOSU"))
    folded = unicodedata.normalize("NFKD", folded).encode("ascii", "ignore").decode()
    return "islem-" + re.sub(r"[^a-z0-9]+", "-", folded.lower())


def num(v):
    return shown(v, 3)


# ── The drawing ──────────────────────────────────────────────────────

BASE = ref("idw")["sources"]
FIELD = ref("idw-alan-donuk-izgara")["values"]
TURNED = ref("idw-alan-donuk-izgara")["grid"]
FLAT = ref("kriging-otomatik")["sources"]
DENS = ref("cekirdek-quartic")["sources"]
WEIGHTS = ref("cekirdek-otomatik-agirlikli")["values"]
SHAPES = ref("cizgi")["shapes"]
LWEIGHTS = ref("cizgi-agirlikli-otomatik")["values"]


def drawing():
    layer = lambda id, name, color: {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
                                     "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25}, "children": []}
    entities = []

    def add(e, layer_id, attrs=None, label=None):
        o = {**e, "id": len(entities) + 1, "layerId": layer_id, "attrs": attrs or {}}
        if label:
            o["label"] = label
        entities.append(o)

    for k, p in enumerate(BASE):
        add(p, "noktalar", {"Değer": FIELD[k]}, f"N{k + 1}")
    for k, p in enumerate(FLAT):
        add(p, "alan", label=f"A{k + 1}")
    for k, p in enumerate(DENS):
        add(p, "olaylar", {"Ağırlık": WEIGHTS[k]})
    for k, s in enumerate(SHAPES):
        add(s, "yollar", {"Ağırlık": LWEIGHTS[k]})
    add({"kind": "raster", "affine": TURNED["affine"], "width": TURNED["width"], "height": TURNED["height"], "bands": 1, "sample": "f32",
         "file": "izgara.tif", "srid": 5254, "style": {"render": "gray", "bands": [1]}}, "izgara")
    for k in range(5):
        add({"kind": "point", "p": {"x": 500000.0 + 10.0 * k, "y": 4420000.0 + 5.0 * k}, "z": 100.0 + k}, "dogru")
    for k in range(3):
        add({"kind": "point", "p": {"x": 500000.0 + 7.0 * k, "y": 4420100.0 + 3.0 * k * k}}, "kotsuz")
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "İşlem durumları: interpolasyon ve yoğunluk",
        "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": 500000, "y": 4420000},
        "layers": [layer("noktalar", "Noktalar", "#1565C0"), layer("alan", "Alan noktaları", "#6A1B9A"), layer("olaylar", "Olaylar", "#C62828"),
                   layer("yollar", "Yollar", "#455A64"), layer("izgara", "Izgara", "#8D6E63"), layer("dogru", "Doğru", "#2E7D32"),
                   layer("kotsuz", "Kotsuz", "#9E9E9E")],
        "activeLayer": "noktalar",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }


# ── The tools' rules ─────────────────────────────────────────────────

LABELS = {
    "interpolation.idw": "Ters uzaklık (IDW)", "interpolation.naturalNeighbor": "Doğal komşu", "interpolation.spline": "Spline",
    "interpolation.kriging": "Kriging", "interpolation.tin": "TIN'den raster", "density.kernel": "Çekirdek yoğunluğu", "density.line": "Çizgi yoğunluğu",
}
SUFFIX = {
    "interpolation.idw": "-idw", "interpolation.naturalNeighbor": "-dogalkomsu", "interpolation.spline": "-spline", "interpolation.kriging": "-kriging",
    "interpolation.tin": "-tin", "density.kernel": "-yogunluk", "density.line": "-cizgiyogunlugu",
}
LAYER = {
    "interpolation.idw": "IDW", "interpolation.naturalNeighbor": "Doğal komşu", "interpolation.spline": "Spline", "interpolation.kriging": "Kriging",
    "interpolation.tin": "TIN", "density.kernel": "Yoğunluk", "density.line": "Çizgi yoğunluğu",
}
ERROR_LAYER = "Kriging standart hatası"


def new_layer(name):
    return {"id": layer_id(name), "name": name, "style": {"color": RASTER_LAYER, "lineType": "continuous", "lineWeight": 0.25}}


def style(ramp, band=1, clear=False):
    s = {"render": "ramp", "bands": [band], "stretch": "minMax", "ramp": ramp}
    if clear:
        s["nodata"] = 0
    return s


def raster_object(layer, r, file, bands, st):
    return {"kind": "raster", "layerId": layer_id(layer), "attrs": {}, "affine": r["expect"]["affine"], "width": r["expect"]["width"],
            "height": r["expect"]["height"], "bands": bands, "sample": "f32", "file": file, "srid": 5254, "style": st}


def cross_table(r, labels, kriging):
    columns = ["Sıra", "Ad", "Y", "X", "Ölçülen", "Tahmin", "Fark"] + (["Standart hata", "Standart fark"] if kriging else [])
    rows = []
    pts = REF_POINTS[r["name"]]
    for i, got in enumerate(r["expect"]["cross"]):
        x, y = pts["xy"][i]
        measured = pts["v"][i]
        pred, err = (got[0], got[1]) if isinstance(got, list) else (got, None)
        row = [str(i + 1), labels[pts["object"][i]], num(x), num(y), num(measured)]
        row += [num(pred), num(pred - measured)] if pred is not None else ["", ""]
        if kriging:
            row += [num(err), num((pred - measured) / err) if err and err > 0 else ""] if pred is not None and err is not None else ["", ""]
        rows.append(row)
    return {"columns": columns, "rows": rows}


def gathered(sources):
    """The points as the reference gathered them (its gather case shows the rule; these sources hold no repeats)."""
    xy = [(s["p"]["x"], s["p"]["y"]) for s in sources]
    assert len(set(xy)) == len(xy)
    return {"xy": xy, "v": [s["z"] for s in sources], "object": list(range(len(sources)))}


REF_POINTS = {name: gathered(ref(name)["sources"]) for name in (
    "idw", "idw-yaricap", "tin", "dogal-komsu", "spline", "kriging-kuresel")}


def case(id, title, tool, values, refname, *, input_layer, layer_name, file=None, cross=False, kriging_error=False, density=None, add=True):
    r = ref(refname)
    e = r["expect"]
    stem = {"noktalar": "Noktalar", "alan": "Alan noktaları", "olaylar": "Olaylar", "yollar": "Yollar"}[input_layer]
    out = file or f"{stem}{SUFFIX[tool]}.tif"
    cells = f"{words(e['width'])} × {words(e['height'])} hücre"
    if density == "points":
        summary = f"{words(len(r['sources']))} noktanın yoğunluğu, yarıçap {num(e['radius'])} m; {cells}; “{out}” yazıldı."
    elif density == "lines":
        edges = 0
        for s in r["shapes"]:
            k = s["kind"]
            if k in ("line", "arc", "circle"):
                edges += 1
            else:
                closed = k == "polygon"
                n = len(s["pts"])
                edges += n if closed else n - 1
                for h in s.get("holes") or []:
                    edges += len(h["pts"])
        summary = f"{words(edges)} çizgi parçasının yoğunluğu, yarıçap {num(e['radius'])} m; {cells}; “{out}” yazıldı."
    else:
        n = len(r["sources"])
        summary = f"{words(n)} noktadan {cells}lik raster; “{out}” yazıldı."
    if "variogram" in e:
        g = e["variogram"]
        summary += f" Variogram: Küresel, külçe {num(g['nugget'])}, kısmi eşik {num(g['sill'])}, erim {num(g['range'])} m."
    if tool == "interpolation.kriging" and "variogram" not in e:
        v = r["tool"]["variogram"]
        name = {"spherical": "Küresel", "exponential": "Üstel", "gaussian": "Gauss"}[r["tool"]["model"]]
        summary += f" Variogram: {name}, külçe {num(v['nugget'])}, kısmi eşik {num(v['sill'])}, erim {num(v['range'])} m."
    expect = {"status": "ok", "undo": LABELS[tool] if add else None, "outputs": {"file": out}}
    warnings = []
    empty = sum(1 for v in e["values"] if v is None)
    if empty:
        warnings.append(f"{words(empty)} hücrede değer yok (noktaların kapsamı dışında ya da yeterli komşu yok).")
    if cross:
        s = e["summary"]
        if s["missing"]:
            warnings.append(f"{words(s['missing'])} noktanın çapraz doğrulaması yok (kabuğun üstünde ya da yeterli komşusu yok).")
        summary += (f" Çapraz doğrulama: {words(s['count'])} noktada ortalama fark {num(s['mean'])}, karesel ortalama hata {num(s['rmse'])},"
                    f" ortalama mutlak fark {num(s['mae'])}.")
        if "stdMean" in s:
            summary += f" Standart farkların ortalaması {num(s['stdMean'])}, karesel ortalaması {num(s['stdRmse'])}."
        labels = [f"N{k + 1}" for k in range(len(BASE))]
        expect["outputs"]["table"] = cross_table(r, labels, tool == "interpolation.kriging")
    expect["summary"] = summary
    if warnings:
        expect["log"] = [{"level": "warn", "text": w} for w in warnings]
    bands = 2 if kriging_error else 1
    if add:
        ramp = "Sıcaklık" if density else ("Viridis" if values.get("field") else "Arazi")
        expect["layers"] = [new_layer(layer_name)] + ([new_layer(ERROR_LAYER)] if kriging_error else [])
        expect["added"] = [raster_object(layer_name, r, out, bands, style(ramp, 1, bool(density)))]
        if kriging_error:
            expect["added"].append(raster_object(ERROR_LAYER, r, out, bands, style("Viridis", 2)))
        expect["layerBelow"] = {layer_id(layer_name): input_layer}
        if kriging_error:
            expect["layerBelow"][layer_id(ERROR_LAYER)] = layer_id(layer_name)
    else:
        expect["layers"] = []
        expect["added"] = []
    expect["interpolationOf"] = {out: refname}
    return {"id": id, "title": title, "document": "interpolation.kcad", "selection": [], "run": {"tool": tool},
            "values": {"input": {"scope": "layer", "layerId": input_layer}, **values}, "expect": expect}


def defaults():
    # A layer scope names the drawing's active layer; an optional objects parameter takes its first scope; an empty field "".
    ends = lambda layer: {"cellSize": 0, "extent": "points", "grid": {"scope": "selection"}, "output": "", "add": True, "layer": {"newName": layer}}
    here = {"scope": "layer", "layerId": "noktalar"}
    interp = lambda layer, extra: {"input": here, "field": "", **extra, **ends(layer), "cross": False}
    return {
        "interpolation.idw": interp("IDW", {"power": 2, "points": 12, "radius": 0, "minPoints": 1}),
        "interpolation.naturalNeighbor": interp("Doğal komşu", {}),
        "interpolation.spline": interp("Spline", {"splineType": "regularized", "weight": 0.1, "points": 12}),
        "interpolation.kriging": {**interp("Kriging", {"model": "spherical", "variogram": "auto", "lags": 12, "nugget": 0, "sill": 1, "range": 100,
                                                      "points": 12, "radius": 0, "errorSurface": False}),
                                  "errorLayer": {"newName": ERROR_LAYER}},
        "interpolation.tin": interp("TIN", {}),
        "density.kernel": {"input": here, "weightField": "", "radius": 0, "kernel": "quartic", "unit": "squareKilometre", **ends("Yoğunluk")},
        "density.line": {"input": here, "weightField": "", "radius": 0, "unit": "kilometrePerSquareKilometre", **ends("Çizgi yoğunluğu")},
    }


def cases():
    out = [
        case("idw", "IDW, varsayılanlar, 5 m hücre, çapraz doğrulamalı: Noktalar-idw.tif, Arazi", "interpolation.idw", {"cellSize": 5, "cross": True},
             "idw", input_layer="noktalar", layer_name="IDW", cross=True),
        case("idw-radius", "IDW 8 nokta, 15 m yarıçap, en az 3 nokta: kenarda boş hücreler", "interpolation.idw",
             {"cellSize": 5, "points": 8, "radius": 15, "minPoints": 3, "cross": True}, "idw-yaricap", input_layer="noktalar", layer_name="IDW", cross=True),
        case("idw-field-grid", "IDW Değer alanından, dönük rasterin ızgarasında: Viridis", "interpolation.idw",
             {"field": "Değer", "extent": "raster", "grid": {"scope": "layer", "layerId": "izgara"}}, "idw-alan-donuk-izgara",
             input_layer="noktalar", layer_name="IDW"),
        case("natural", "Doğal komşu, 10 m hücre, çapraz doğrulamalı: kabuğun dışı boş", "interpolation.naturalNeighbor", {"cellSize": 10, "cross": True},
             "dogal-komsu", input_layer="noktalar", layer_name="Doğal komşu", cross=True),
        case("tin", "TIN'den raster, 5 m hücre, çapraz doğrulamalı, adı yazılı", "interpolation.tin", {"cellSize": 5, "cross": True, "output": "yuzey tin.tiff"},
             "tin", input_layer="noktalar", layer_name="TIN", cross=True, file="yuzey tin.tif"),
        case("spline", "Spline düzenlemeli 0,1, 12 nokta, çapraz doğrulamalı", "interpolation.spline", {"cellSize": 5, "cross": True},
             "spline", input_layer="noktalar", layer_name="Spline", cross=True),
        case("kriging-error", "Kriging küresel, elle variogram, hata yüzeyiyle: iki katman, iki bant", "interpolation.kriging",
             {"cellSize": 5, "variogram": "manual", "nugget": 0.5, "sill": 120, "range": 90, "errorSurface": True, "cross": True},
             "kriging-kuresel", input_layer="noktalar", layer_name="Kriging", cross=True, kriging_error=True),
        case("kriging-auto", "Kriging, otomatik variogram (durağan alan): uydurulan değerler özette, çizime eklenmez", "interpolation.kriging",
             {"cellSize": 10, "add": False}, "kriging-otomatik", input_layer="alan", layer_name="Kriging", add=False),
        case("kernel", "Çekirdek yoğunluğu, dörtlü, 30 m, 10 m hücre: km² başına, Sıcaklık, 0'lar boş", "density.kernel", {"radius": 30, "cellSize": 10},
             "cekirdek-quartic", input_layer="olaylar", layer_name="Yoğunluk", density="points"),
        case("kernel-auto", "Çekirdek yoğunluğu Ağırlık alanıyla, yarıçap kendiliğinden: hektar başına", "density.kernel",
             {"weightField": "Ağırlık", "unit": "hectare", "cellSize": 10}, "cekirdek-otomatik-agirlikli", input_layer="olaylar", layer_name="Yoğunluk",
             density="points"),
        case("line", "Çizgi yoğunluğu 12 m, 4 m hücre: çizgi, yaylı çoklu çizgi, delikli alan, yay, daire", "density.line", {"radius": 12, "cellSize": 4},
             "cizgi", input_layer="yollar", layer_name="Çizgi yoğunluğu", density="lines"),
        case("line-auto", "Çizgi yoğunluğu Ağırlık alanıyla, yarıçap kendiliğinden: m/ha", "density.line",
             {"weightField": "Ağırlık", "unit": "metrePerHectare", "cellSize": 4}, "cizgi-agirlikli-otomatik", input_layer="yollar",
             layer_name="Çizgi yoğunluğu", density="lines"),
    ]
    refused = lambda id, title, tool, values, message: {
        "id": id, "title": title, "document": "interpolation.kcad", "selection": [], "run": {"tool": tool}, "values": values,
        "expect": {"status": "error", "message": message},
    }
    out += [
        refused("tin-line", "TIN doğru üzerindeki noktalardan: üçgenleme yok", "interpolation.tin", {"input": {"scope": "layer", "layerId": "dogru"}},
                "Üçgenleme için doğru üzerinde olmayan en az üç nokta gerekir."),
        refused("no-elevation", "Kotsuz noktalardan IDW: kot yok", "interpolation.idw", {"input": {"scope": "layer", "layerId": "kotsuz"}},
                "Kotu olan nokta yok: köşelerin kotu yok. Kotlu nokta seçin ya da Değer alanı'nı seçin."),
        refused("grid-missing", "Rasterin ızgarası seçili ama raster yok", "interpolation.idw",
                {"input": {"scope": "layer", "layerId": "noktalar"}, "extent": "raster", "grid": {"scope": "layer", "layerId": "noktalar"}},
                "Izgara rasterini seçin: Kapsam rasterin ızgarası."),
        refused("auto-few", "Kriging otomatik variogram, beş doğrusal nokta, üç aralık: ikisi dolu", "interpolation.kriging",
                {"input": {"scope": "layer", "layerId": "dogru"}, "lags": 3},
                "Variogram için yeterli nokta çifti yok (üçten az dolu aralık): Variogram'ı elle girin."),
    ]
    return out


def build():
    return {
        "format": "kentos.processing-cases",
        "version": 1,
        "tolerance": 1e-9,
        "documents": {
            "interpolation.kcad": {
                "defaults": {"lengthDecimals": 3, "areaDecimals": 2, "angleUnit": "grad", "plotScale": 1000, "drawingFont": "barlow",
                             "activeLayer": "noktalar", "measureHeightMm": 2},
                "tools": defaults(),
            }
        },
        "cases": cases(),
    }


def main():
    check = "--check" in sys.argv[1:]
    texts = {
        DRAWING: json.dumps(drawing(), ensure_ascii=False, indent=1) + "\n",
        CASES: json.dumps(build(), ensure_ascii=False, indent=1) + "\n",
    }
    problems = []
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
