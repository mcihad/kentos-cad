#!/usr/bin/env python3
"""Yüzey analizinin İşlemler durumları (docs/adr/0231 §10; biçim fixtures/processing/README.md).

    python3 scripts/fixtures/surface_processing_cases.py           # çizimi ve durumları yazar
    python3 scripts/fixtures/surface_processing_cases.py --check   # hiçbir şey yazmaz; karşılaştırır

KentOS kodu kullanılmaz. Çizim (fixtures/processing/v1/surface.kcad) yüzey analizinin örnek DEM'lerini
(fixtures/terrain/v1: tepe ve dönük) bağlı raster olarak taşır, bir de çizgi. Durumlar araçların kuralından
yazılır: sonucun adı (kaynağın adı ve aracın eki, ya da yazılan ad .tif'le), özet, yeni katmanın adı ve
stili, raster nesnesinin alanları (kaynağın yeri ve boyu, sonucun bantları ve örnekleri, aracın görünüşü).
Yazılan dosyanın değerleri yüzey analizinin bağımsız başvurusuna bağlanır (`rasterOf`: dosyanın 0. katı
fixtures/terrain/v1/cases.json'daki adı verilen durumun değerleri), eğriler eğrilerinkine (`contoursOf`:
eklenen çoklu çizgiler fixtures/contours/v1/cases.json'daki durumun eğrileri; köşeleri, kotları, Kot ve
Tür'ü, ana eğrinin kalınlığı). Sonucun yeni katmanı kaynağın katmanının hemen üstündedir (`layerAbove`). İki oynatıcı (apps/web/src/processing/surface.test.ts,
crates/native/processing/tests/surface.rs) çizimi açar, aracı ev sahibinin dosyalarıyla çalıştırır
(rasterler `rasters`'ın dosyalarından okunur, sonuçlar bellekte toplanır) ve sonucu bunlarla karşılaştırır.
"""

import json
import re
import sys
import unicodedata
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/processing/v1"
DRAWING = DIR / "surface.kcad"
CASES = DIR / "surface.json"
TERRAIN = json.loads((ROOT / "fixtures/terrain/v1/cases.json").read_text("utf-8"))
CONTOURS = json.loads((ROOT / "fixtures/contours/v1/cases.json").read_text("utf-8"))

TM30 = {"kind": "tm", "datum": "TUREF", "centralMeridian": 30, "scaleFactor": 1, "falseEasting": 500000, "falseNorthing": 0}
RASTER_LAYER = "#7A6B5B"
CONTOUR_LAYER = "#A0522D"


def dem(name):
    return next(d for d in TERRAIN["dems"] if d["name"] == name)


def drawing():
    tepe, donuk = dem("tepe"), dem("donuk")
    layer = lambda id, name, color: {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True, "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25}, "children": []}

    def raster(id, d):
        return {
            "kind": "raster", "id": id, "layerId": "dem", "attrs": {}, "affine": d["affine"], "width": d["width"], "height": d["height"],
            "bands": 1, "sample": "f32", "file": d["file"], "srid": 5254,
            "style": {"render": "rampShade", "bands": [1], "stretch": "minMax", "ramp": "Arazi"},
        }

    return {
        "format": "kentos.document",
        "version": 1,
        "name": "İşlem durumları: yüzey analizi",
        "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": 500000, "y": 4420000},
        "layers": [layer("dem", "DEM", "#8D6E63"), layer("cizim", "Çizim", "fg")],
        "activeLayer": "dem",
        "entities": [
            raster(1, tepe),
            raster(2, donuk),
            {"kind": "polyline", "id": 3, "layerId": "cizim", "attrs": {}, "pts": [{"x": 500150, "y": 4420150}, {"x": 500300, "y": 4420250}]},
            # tepe placed on a sheared grid (its axes not at right angles), as a raster may be placed by hand.
            {**raster(4, tepe), "affine": [500100.0, 10.0, 3.0, 4420330.0, 0.0, -10.0]},
        ],
        "styles": {"items": [], "categories": []},
    }


# ── The tools' rules ─────────────────────────────────────────────────────

def words(n):
    """A whole number with dots between its thousands."""
    s = str(n)
    return re.sub(r"\B(?=(\d{3})+(?!\d))", ".", s)


def layer_id(name):
    """A new layer's id: “islem-” and the name, Turkish letters folded, lower case, other letters '-'."""
    folded = name.translate(str.maketrans("çğıöşüÇĞİÖŞÜ", "cgiosuCGIOSU"))
    folded = unicodedata.normalize("NFKD", folded).encode("ascii", "ignore").decode()
    return "islem-" + re.sub(r"[^a-z0-9]+", "-", folded.lower())


def new_layer(name, color):
    return {"id": layer_id(name), "name": name, "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25}}


STYLES = {
    "ramp": lambda ramp, stretch="minMax": {"render": "ramp", "bands": [1], "stretch": stretch, "ramp": ramp},
    "gray": {"render": "gray", "bands": [1]},
    "rgb": {"render": "rgb", "bands": [1, 2, 3, 4]},
}


def raster_case(id, title, tool, values, *, suffix, layer, terrain, style, bands=1, sample="f32", name=None, add=True, source="tepe", selection=(1,)):
    d = dem(source)
    stem = Path(d["file"]).stem
    out = name or f"{stem}{suffix}.tif"
    expect = {
        "status": "ok",
        "summary": f"{words(d['width'])} × {words(d['height'])} hücrelik raster; “{out}” yazıldı.",
        "undo": tool_label(tool) if add else None,
        "layers": [new_layer(layer, RASTER_LAYER)] if add else [],
        "added": [
            {
                "kind": "raster", "layerId": layer_id(layer), "attrs": {}, "affine": d["affine"], "width": d["width"], "height": d["height"],
                "bands": bands, "sample": sample, "file": out, "srid": 5254, "style": style,
            }
        ] if add else [],
        "outputs": {"file": out},
        "rasterOf": {out: terrain},
    }
    if add:
        # The result's new layer right above its source's (a raster under it would hide it).
        expect["layerAbove"] = {layer_id(layer): "dem"}
    return {"id": id, "title": title, "document": "surface.kcad", "selection": list(selection), "run": {"tool": tool}, "values": values, "expect": expect}


LABELS = {
    "surface.slope": "Eğim", "surface.aspect": "Bakı", "surface.hillshade": "Gölgeli kabartma", "surface.colorRelief": "Renkli kabartma",
    "surface.curvature": "Eğrilik", "surface.ruggedness": "Pürüzlülük", "surface.insolation": "Güneşlenme", "surface.contours": "Eş yükselti eğrileri",
}


def tool_label(tool):
    return LABELS[tool]


def terrain_case(name):
    """A surface case of tepe by its name (it must exist)."""
    next(c for c in TERRAIN["cases"] if c["dem"] == "tepe" and c["name"] == name)
    return name


def contour_case(k):
    c = CONTOURS["cases"][k]
    lines = c["lines"]
    main = sum(1 for ln in lines if ln["index"])
    vertices = sum(len(ln["pts"]) for ln in lines)
    texts = c["texts"]
    lo, hi = texts[str(lines[0]["k"])], texts[str(lines[-1]["k"])]
    return {
        "summary": f"{words(len(lines))} eğri ({words(main)} ana, {words(len(lines) - main)} ara), {words(vertices)} köşe; kotlar {lo} ile {hi} arası.",
        "lines": len(lines),
    }


def defaults():
    raster_values = lambda layer, suffix_extra: {"input": {"scope": "selection"}, "band": 1, **suffix_extra, "output": "", "add": True, "layer": {"newName": layer}}
    return {
        "surface.slope": raster_values("Eğim", {"method": "horn", "unit": "degrees", "zFactor": 1}),
        "surface.aspect": raster_values("Bakı", {"method": "horn", "zFactor": 1}),
        "surface.hillshade": raster_values("Gölgeli kabartma", {"azimuth": 315, "altitude": 45, "zFactor": 1}),
        "surface.colorRelief": raster_values(
            "Renkli kabartma", {"colors": "ramp", "ramp": "Arazi", "invert": False, "range": "auto", "min": 0, "max": 1000, "table": "", "interp": "linear"}
        ),
        "surface.curvature": raster_values("Eğrilik", {"curvature": "total", "zFactor": 1}),
        "surface.ruggedness": raster_values("Pürüzlülük", {"index": "triRiley"}),
        "surface.insolation": raster_values(
            "Güneşlenme",
            {"period": "year", "startDay": 21, "startMonth": "6", "endDay": 21, "endMonth": "9", "dayStep": 14, "hourStep": "0.5", "transmissivity": 0.5, "latitude": None, "zFactor": 1},
        ),
        "surface.contours": {"input": {"scope": "selection"}, "band": 1, "interval": 5, "base": 0, "indexEvery": 5, "simplify": 0, "layer": {"newName": "Eş yükselti eğrileri"}},
    }


def cases():
    out = [
        raster_case("slope", "Eğim, varsayılanlar: derece, Horn; kaynağın yanına tepe-egim.tif, Spektral'le", "surface.slope", {},
                    suffix="-egim", layer="Eğim", terrain=terrain_case("Eğim (derece, Horn)"), style=STYLES["ramp"]("Spektral")),
        raster_case("slope-percent-named", "Eğim yüzde, Zevenbergen-Thorne, z 1,5; yazılan ad .tif'le, çizime eklenmez", "surface.slope",
                    {"method": "zevenbergenThorne", "unit": "percent", "zFactor": 1.5, "output": "egim haritasi.tiff", "add": False},
                    suffix="-egim", layer="Eğim", terrain=terrain_case("Eğim (yüzde, Zevenbergen-Thorne, z 1,5)"), style=None, name="egim haritasi.tif", add=False),
        raster_case("aspect", "Bakı, Zevenbergen-Thorne", "surface.aspect", {"method": "zevenbergenThorne"},
                    suffix="-baki", layer="Bakı", terrain=terrain_case("Bakı (Zevenbergen-Thorne)"), style=STYLES["ramp"]("Spektral")),
        raster_case("hillshade", "Gölgeli kabartma 135°, 30°, z 2: 8 bit gri", "surface.hillshade", {"azimuth": 135, "altitude": 30, "zFactor": 2},
                    suffix="-golge", layer="Gölgeli kabartma", terrain=terrain_case("Gölgeli kabartma (135, 30, 2)"), style=STYLES["gray"], sample="u8"),
        raster_case("color-relief-table", "Renkli kabartma, renk tablosu (alfalı satır, aynı değerden ilki), doğrusal: RGBA", "surface.colorRelief",
                    {"colors": "table", "table": "180 #A1887F\n100 #2E7D32\n140 #FFF59D80\n140 #000000\n"},
                    suffix="-renkli", layer="Renkli kabartma", terrain=terrain_case("Renkli kabartma (tablo, alfa)"), style=STYLES["rgb"], bands=4, sample="u8"),
        raster_case("color-relief-ramp", "Renkli kabartma, Viridis ters, elle 100–200, en yakın", "surface.colorRelief",
                    {"ramp": "Viridis", "invert": True, "range": "manual", "min": 100, "max": 200, "interp": "nearest"},
                    suffix="-renkli", layer="Renkli kabartma", terrain=terrain_case("Renkli kabartma (Viridis ters, 100–200, en yakın)"), style=STYLES["rgb"], bands=4, sample="u8"),
        raster_case("curvature", "Eğrilik, toplam, z 2: Mavi-kırmızı", "surface.curvature", {"curvature": "total", "zFactor": 2},
                    suffix="-egrilik", layer="Eğrilik", terrain=terrain_case("Eğrilik (toplam, z 2)"), style=STYLES["ramp"]("Mavi-kırmızı", "percent")),
        raster_case("ruggedness", "Pürüzlülük, TPI: Viridis", "surface.ruggedness", {"index": "tpi"},
                    suffix="-puruzluluk", layer="Pürüzlülük", terrain=terrain_case("TPI"), style=STYLES["ramp"]("Viridis")),
        raster_case("insolation-range", "Güneşlenme 1 Haziran–31 Ağustos, 7 gün, 1 saat, geçirgenlik 0,6: Sıcaklık", "surface.insolation",
                    {"period": "range", "startDay": 1, "startMonth": "6", "endDay": 31, "endMonth": "8", "dayStep": 7, "hourStep": "1", "transmissivity": 0.6},
                    suffix="-gunes", layer="Güneşlenme", terrain=terrain_case("Güneşlenme (1 Haziran–31 Ağustos, 7 gün, 1 saat, 0,6)"), style=STYLES["ramp"]("Sıcaklık")),
    ]
    c = contour_case(2)
    out.append({
        "id": "contours", "title": "Eş yükselti eğrileri, tepe, 5 m, her beşincisi ana: kotlu çoklu çizgiler", "document": "surface.kcad", "selection": [1],
        "run": {"tool": "surface.contours"}, "values": {},
        "expect": {
            "status": "ok", "summary": c["summary"], "undo": "Eş yükselti eğrileri", "layers": [new_layer("Eş yükselti eğrileri", CONTOUR_LAYER)],
            "contoursOf": 2, "outputs": {"lines": c["lines"]}, "layerAbove": {layer_id("Eş yükselti eğrileri"): "dem"},
        },
    })
    refused = lambda id, title, tool, values, message, selection=(1,): {
        "id": id, "title": title, "document": "surface.kcad", "selection": list(selection), "run": {"tool": tool}, "values": values,
        "expect": {"status": "error", "message": message},
    }
    out += [
        refused("two-rasters", "İki raster seçili: tek raster ister", "surface.slope", {}, "2 raster seçili; çözümleme tek raster ister.", selection=(1, 2)),
        {
            "id": "only-a-line", "title": "Seçimde raster yok (yalnız çizgi): pencere reddeder", "document": "surface.kcad", "selection": [3],
            "run": {"tool": "surface.aspect"}, "values": {},
            # The runner's rule for an input that resolves to nothing (fixtures/processing/README.md).
            "expect": {"status": "invalid", "issues": [{"param": "input", "message": "“Raster”: seçili nesneler arasında uygun nesne yok. Önce nesneleri seçin ya da kapsamı değiştirin."}]},
        },
        refused("band", "2. bant: rasterin bir bandı var", "surface.hillshade", {"band": 2}, "Rasterin 1 bandı var; 2. bant yok."),
        refused("sheared-curvature", "Eğrilik eğik pikselli rasterde (eksenleri dik değil): dik piksel ister", "surface.curvature", {},
                "Eğrilik dik pikselli raster ister; bu rasterin pikselleri eğik (afinin eksenleri dik değil).", selection=(4,)),
        {
            "id": "february-30", "title": "Güneşlenme 30 Şubat'tan: pencere reddeder", "document": "surface.kcad", "selection": [1],
            "run": {"tool": "surface.insolation"}, "values": {"period": "day", "startDay": 30, "startMonth": "2"},
            "expect": {"status": "invalid", "issues": [{"message": "Şubat ayının günü 1 ile 28 arasında olmalı; 30 verildi."}]},
        },
    ]
    return out


def build():
    return {
        "format": "kentos.processing-cases",
        "version": 1,
        "tolerance": 1e-9,
        "documents": {
            "surface.kcad": {
                "defaults": {"lengthDecimals": 3, "areaDecimals": 2, "angleUnit": "grad", "plotScale": 1000, "drawingFont": "barlow", "activeLayer": "dem", "measureHeightMm": 2},
                "tools": defaults(),
            }
        },
        "rasters": {"tepe.tif": "../../terrain/v1/tepe.tif", "donuk.tif": "../../terrain/v1/donuk.tif"},
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
