#!/usr/bin/env python3
"""Atlasın ortak durumları ve bağımsız denetimi (docs/adr/0534), KentOS kodu olmadan.

    /usr/bin/python3 scripts/fixtures/atlas_cases.py           # fixtures/atlas/v1/cases.json'u yazar (pyproj ister)
    /usr/bin/python3 scripts/fixtures/atlas_cases.py --check   # yazılı durumlar güncel mi
    /usr/bin/python3 scripts/fixtures/atlas_cases.py --gpkg    # kaynak durumlarının GeoPackage'ını GDAL'la yazar (ADR 0638 §3)
    python3 scripts/fixtures/atlas_cases.py --files   # KentOS'un yazdığı PDF'i poppler ve GDAL'la denetler

Çizim: 105 parsel (15 × 7 ızgara, 30 × 20 m, aralarında 2 m; sonuncusu 90 × 50 m), beşi iptal ("Durum" = "iptal";
katmanın süzgeci yalnız "aktif"leri geçirir), her birinin malik ("Malik", bir formda `redact` işaretli) ve alanı
tanımlı olmayan bir özniteliği ("Sır") var. Pafta sistemin atlas şablonudur (crates/shared/sheet/templates/gis-atlas.json:
veri olarak okunur); kapsam katmanı parsel, sıralama parsel numarası, sayfa adı "ada/numara".

Beklenen sayfalar buradaki kuraldan hesaplanır: katman süzgecinden geçen parseller, numaraya göre sıralı, sayfa adı
`Ada || '/' || Parsel`, harita merkezi parselin kutusunun ortası, ölçek haritanın içeriğine %10 payla sığan en büyük
standart ölçek (içerik 177 − 2 × 6 = 165 mm × 162 − 2 × 6 = 150 mm; standart ölçekler papers.json'daki seri).

`--files`: KentOS'un yazdığı fixtures/atlas/v1/out/parsel-atlasi.pdf (KENTOS_WRITE_ATLAS=1 cargo test -p kentos-headless
--test headless atlas) poppler'ın pdfinfo ve pdftotext'iyle (sayfa sayısı, Keywords, sayfaların metni: adlar, öznitelikler,
tablo, gizli öznitelik ve iptal parsel yok) ve GDAL'ın gdalinfo'suyla (GeoPDF: koordinat sistemi ve her sayfanın haritasının
yeri ve ölçeği) okunur.
"""

import json
import re
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/atlas/v1/cases.json"
FILES = ROOT / "fixtures/atlas/v1/out"
TEMPLATE = ROOT / "crates/shared/sheet/templates/gis-atlas.json"
PAPERS = ROOT / "crates/shared/sheet/data/papers.json"

COLUMNS, ROWS = 15, 7
CANCELLED = {5, 20, 35, 50, 65}
ORIGIN = (400_000.0, 4_400_000.0)
WIDTH, HEIGHT, GAP = 30.0, 20.0, 2.0
BIG = (90.0, 50.0)
CONTENT_MM = (165.0, 150.0)
MARGIN = 0.10
SCALES = json.loads(PAPERS.read_text())["scales"]
# The first 50 parcels are from 2010, the rest from 2020 (the temporal setup).
OLD = 50


def parcels():
    out = []
    for i in range(COLUMNS * ROWS):
        col, row = i % COLUMNS, i // COLUMNS
        n = i + 1
        w, h = BIG if n == COLUMNS * ROWS else (WIDTH, HEIGHT)
        out.append({
            "n": n, "ada": f"A{row + 1}", "x": ORIGIN[0] + col * (WIDTH + GAP), "y": ORIGIN[1] + row * (HEIGHT + GAP),
            "w": w, "h": h, "durum": "iptal" if n in CANCELLED else "aktif",
            "kurulus": "2010-01-01" if n <= OLD else "2020-06-01",
        })
    return out


def scale_of(w, h):
    need = max(w * (1 + 2 * MARGIN) / (CONTENT_MM[0] / 1000), h * (1 + 2 * MARGIN) / (CONTENT_MM[1] / 1000))
    return next(s for s in SCALES if s >= need)


def pages(keep=lambda p: True):
    """The pages of the atlas: the layer's filter ("aktif"), the page filter `keep`, sorted by the number."""
    out = []
    for p in parcels():
        if p["durum"] != "aktif" or not keep(p):
            continue
        out.append(p)
    out.sort(key=lambda p: p["n"])
    return [{
        "index": k + 1, "name": f"{p['ada']}/{p['n']}", "scale": scale_of(p["w"], p["h"]),
        "center": {"x": p["x"] + p["w"] / 2, "y": p["y"] + p["h"] / 2},
        "bbox": [p["x"], p["y"], p["x"] + p["w"], p["y"] + p["h"]],
    } for k, p in enumerate(out)]


def near(value, within=1e-3):
    """A number the runner takes within `within` of `value`: PROJ's TM and the core's agree to the millimetre."""
    return {"$near": value, "within": within}


def pages_in(target):
    """The pages drawn in another system (docs/adr/0638 §1): each parcel's box moved there with PROJ (21 inner points
    on each edge, as `crs::transform_bounds`), the page's middle the moved box's, its scale fitting the moved box; the
    box told stays the project's."""
    from pyproj import Transformer
    tr = Transformer.from_crs(5253, target, always_xy=True)
    out = []
    for page in pages():
        b = page["bbox"]
        x0, y0, x1, y1 = tr.transform_bounds(b[0], b[1], b[2], b[3], densify_pts=21)
        out.append({**page, "scale": scale_of(x1 - x0, y1 - y0),
                    "center": {"x": near((x0 + x1) / 2), "y": near((y0 + y1) / 2)}})
    return out


def book(atlas=None, with_atlas=True, second=True):
    t = json.loads(TEMPLATE.read_text())
    sheet = t["sheet"]
    sheet["id"] = "atlas-paftasi"
    sheet["name"] = "Parsel atlası"
    sheet["variables"] = [{"name": "kaynak", "label": "Verinin kaynağı"}]
    sheet["atlas"] = atlas if atlas is not None else {
        "layer": "parsel", "filter": "", "sort": [{"expression": "Parsel", "descending": False}],
        "pageName": "Ada || '/' || Parsel",
    }
    if not with_atlas:
        sheet.pop("atlas", None)
    sheet["items"] += [
        {"id": "not", "name": "Nesne notu", "frame": {"left": 19000, "top": 22000, "width": 177000, "height": 7000},
         "kind": {"type": "text",
                  "content": "Ada [% Ada %] parsel [% Parsel %] durum [% Durum %] malik [% Malik %] sır [% Sır %]",
                  "style": {"size": 2600, "weight": 500}, "wrap": False}},
        {"id": "parseller", "name": "Atlas nesnesinin parselleri",
         "frame": {"left": 96000, "top": 130000, "width": 100000, "height": 40000},
         "kind": {"type": "table", "title": "PARSELLER", "header": True,
                  "columns": [{"heading": "Ada", "value": "Ada", "width": 25000},
                              {"heading": "Parsel", "value": "Parsel", "width": 25000},
                              {"heading": "Malik", "value": "Malik"}],
                  "source": {"type": "layer", "layer": "parsel", "atlasFilter": True}}},
    ]
    sheets = [sheet]
    if second:
        # A sheet with no atlas of its own.
        other = {"id": "bos-pafta", "name": "Atlassız pafta", "page": sheet["page"], "variants": [],
                 "items": [{"id": "bos-cerceve", "name": "Sınır çizgisi",
                            "frame": {"left": 15000, "top": 10000, "width": 180000, "height": 277000},
                            "kind": {"type": "border", "style": "single", "stroke": {"width": 350}}}]}
        sheets.append(other)
    return {"schema": "kentos.sheet/1", "sheets": sheets, "masters": [], "assets": [], "variables": []}


def layer(id_, name, color):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": color, "lineType": "continuous", "lineWeight": 0.35}, "children": []}


def setup(name="Parsel atlası", *, sheets=True, atlas=None, temporal=False, states=False, form=True, filter_=True,
          parcels_=True, withheld=False):
    entities = []
    for p in (parcels() if parcels_ else []):
        attrs = {"Ada": p["ada"], "Parsel": str(p["n"]), "Durum": p["durum"], "Malik": f"Gizli Kişi {p['n']}",
                 "Sır": f"gizli-not-{p['n']}"}
        if temporal:
            attrs["Kurulus"] = p["kurulus"]
        entities.append({"kind": "polygon", "id": len(entities) + 1, "layerId": "parsel",
                         "pts": [{"x": p["x"], "y": p["y"]}, {"x": p["x"] + p["w"], "y": p["y"]},
                                 {"x": p["x"] + p["w"], "y": p["y"] + p["h"]}, {"x": p["x"], "y": p["y"] + p["h"]}],
                         "attrs": attrs})
    for k in range(4):
        y = ORIGIN[1] - 10.0 + k * 60.0
        entities.append({"kind": "line", "id": len(entities) + 1, "layerId": "yol",
                         "a": {"x": ORIGIN[0] - 20.0, "y": y}, "b": {"x": ORIGIN[0] + 520.0, "y": y}, "attrs": {}})
    parsel = layer("parsel", "Parsel", "#0090FF")
    parsel["fields"] = [{"name": "Ada", "kind": "text"}, {"name": "Parsel", "kind": "integer"},
                        {"name": "Durum", "kind": "text"}, {"name": "Malik", "kind": "text"}]
    if withheld:
        # Dışarı verilmez (docs/adr/0638 §5): the owner kept in the drawing with no form.
        parsel["fields"][3]["withheld"] = True
    if temporal:
        parsel["fields"].append({"name": "Kurulus", "kind": "date"})
        parsel["time"] = {"start": "Kurulus", "cumulative": True}
    if filter_:
        parsel["filter"] = {"expression": "Durum = 'aktif'"}
    settings = {"srid": 5253, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                "plotScale": 1000, "workspace": "gis"}
    if form:
        settings["forms"] = [{"id": "parsel-formu", "name": "Parsel formu", "layer": "parsel",
                              "draft": {"revision": 1, "items": [
                                  {"type": "field", "id": "malik", "field": "Malik", "redact": True}]}}]
    if states:
        settings["layerStates"] = [{"id": "gece", "name": "Gece", "nodes": [
            {"node": "parsel", "visible": True}, {"node": "yol", "visible": False}]}]
    layers = [parsel, layer("yol", "Yol", "#555555")] if parcels_ else [layer("yol", "Yol", "#555555")]
    if not parcels_:
        settings.pop("forms", None)
    drawing = {"format": "kentos.document", "version": 1, "name": name, "settings": settings,
               "origin": {"x": ORIGIN[0], "y": ORIGIN[1]}, "layers": layers,
               "activeLayer": layers[0]["id"], "entities": entities, "styles": {"items": [], "categories": []}}
    # The sheets are added by `cad.sheets.edit`, as a script would (a v1 file has no sheet book).
    return {"drawing": drawing, "book": book(atlas=atlas) if sheets else None}


def done(output=None, warnings=None):
    r = {"status": "completed"}
    if output is not None:
        r["output"] = output
    if warnings is not None:
        r["warnings"] = warnings
    return r


def refused(code, path=None, status="failed"):
    e = {"code": code}
    if path:
        e["path"] = path
    return {"status": status, "error": e}


def plan_output(pages_, **more):
    out = {"sheet": "atlas-paftasi", "pages": pages_, "fields": ["Ada", "Parsel", "Durum"],
           "withheld": ["Malik", "Sır"], "filteredOut": len(CANCELLED),
           "bindings": {"srid": 5253, "crsName": "TUREF / TM27", "geo": True}}
    out.update(more)
    return out


def cases():
    out = []
    P = "$tmp/atlas.pdf"
    every = pages()
    assert len(every) == 100

    def case(name, steps, setup_="main"):
        out.append({"name": name, "setup": setup_, "steps": steps})

    source_cases(case, P)

    case("Plan: yüz sayfa; katman süzgecinden geçmeyen beş parsel sayfa olmaz, sıra numaraya göre, ad ada/numara, "
         "merkez parselin ortası, büyük parsel 1/1000'de", [
             {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi"}, "result": done(plan_output(every))}])
    case("Plan dosya yazmaz; execute yazar: PDF'in sayfa sayısı sayfa sayısıdır", [
        {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi"}, "result": done(), "file": {"exists": False}},
        {"op": "execute", "input": {"path": P, "sheet": "atlas-paftasi"},
         "result": done({"sheet": "atlas-paftasi", "replaced": False, "pages": every}),
         "file": {"exists": True, "pdfPages": 100}}])
    case("Doğrulama dosya yazmaz ve çıktı vermez", [
        {"op": "validate", "input": {"path": P, "sheet": "atlas-paftasi"}, "result": done(), "file": {"exists": False}}])
    case("Var olan dosya overwrite olmadan yazılmaz; overwrite ile yerine konur", [
        {"op": "execute", "input": {"path": P, "sheet": "atlas-paftasi"}, "result": done({"replaced": False}),
         "file": {"exists": True}},
        {"op": "execute", "input": {"path": P, "sheet": "atlas-paftasi"}, "result": refused("file_exists", "path")},
        {"op": "execute", "input": {"path": P, "sheet": "atlas-paftasi", "overwrite": True},
         "result": done({"replaced": True}), "file": {"exists": True, "pdfPages": 100}}])
    case("Göreli yol reddedilir", [
        {"op": "plan", "input": {"path": "atlas.pdf", "sheet": "atlas-paftasi"},
         "result": refused("invalid_file_path", "path")}])
    case("Olmayan pafta", [
        {"op": "plan", "input": {"path": P, "sheet": "yok"}, "result": refused("unknown_sheet", "sheet")}])
    case("Atlası olmayan pafta", [
        {"op": "plan", "input": {"path": P, "sheet": "bos-pafta"}, "result": refused("no_atlas", "sheet")}])
    case("Paftası olmayan proje", [
        {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi"}, "result": refused("no_sheets", "sheet")}],
         "no-sheets")
    case("Kapsam katmanı olmayan atlas", [
        {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi"}, "result": refused("atlas_no_layer", "sheet")}],
         "no-layer")
    case("Hiçbir nesnenin geçmediği süzgeç: sayfa yok", [
        {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi"}, "result": refused("atlas_empty", "sheet")}],
         "empty")
    case("Paftanın kendi süzgeci katman süzgecine eklenir: yalnız 11–20 numaralılar, iptal 20 hariç", [
        {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi"},
         "result": done({"pages": pages(lambda p: 10 < p["n"] <= 20), "filteredOut": 5})}], "page-filter")
    case("Koordinat sistemi: projenin sistemi açıkça kabul edilir ve kayda geçer", [
        {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi", "crs": 5253},
         "result": done({"bindings": {"srid": 5253, "crsName": "TUREF / TM27", "geo": True}})}])
    case("Başka bir sistemde: TUREF / TM30'da sayfaların ortası ve ölçeği o sistemin zemininde, kutuları projenin",
         [{"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi", "crs": 5254},
           "result": done({"pages": pages_in(5254), "bindings": {
               "srid": 5254, "crsName": "TUREF / TM30", "geo": True, "projectSrid": 5253}})}])
    case("Başka bir sistemde: WGS 84 / UTM 35N'e TUREF'ten EPSG'nin sıfır dönüşümüyle; dosya yazılır", [
        {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi", "crs": 32635},
         "result": done({"pages": pages_in(32635), "bindings": {
             "srid": 32635, "crsName": "WGS 84 / UTM 35N", "geo": True, "projectSrid": 5253}})},
        {"op": "execute", "input": {"path": P, "sheet": "atlas-paftasi", "crs": 32635}, "result": done(),
         "file": {"exists": True, "pdfPages": 100}}])
    case("Derece birimli sistem reddedilir: ölçekli harita metre ister", [
        {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi", "crs": 4326},
         "result": refused("crs_unsupported", "crs")}])
    case("Kayıtta olmayan sistem reddedilir", [
        {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi", "crs": 9999},
         "result": refused("crs_unsupported", "crs")}])
    case("Tema: projenin katman durumu atlas haritalarına bağlanır ve kayda geçer", [
        {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi", "theme": "Gece"},
         "result": done({"bindings": {"theme": "Gece"}, "pages": every})}], "states")
    case("Olmayan tema", [
        {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi", "theme": "Gündüz"},
         "result": refused("unknown_theme", "theme")}], "states")
    case("Zaman: 2015'te yalnız 2010'dan kurulu parseller sayfa olur", [
        {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi", "time": "2015-01-01"},
         "result": done({"bindings": {"time": "2015-01-01"}, "pages": pages(lambda p: p["n"] <= OLD)})}], "temporal")
    case("Zaman: 2024'te bütün (iptal olmayan) parseller", [
        {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi", "time": "2024-01-01"},
         "result": done({"pages": pages()})}], "temporal")
    case("Zaman yazılmamışsa kesilmez", [
        {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi"},
         "result": done({"pages": pages()})}], "temporal")
    case("Okunamayan an", [
        {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi", "time": "dün"},
         "result": refused("invalid_time", "time")}], "temporal")
    case("İzinli öznitelikler: yalnız istenen alan taşınır, öbürleri tutulur", [
        {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi", "fields": ["Ada"]},
         "result": done({"fields": ["Ada"], "withheld": ["Durum", "Malik", "Parsel", "Sır"]})}])
    case("Formda redact işaretli alan istense de taşınmaz", [
        {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi", "fields": ["Ada", "Malik", "Parsel"]},
         "result": done({"fields": ["Ada", "Parsel"], "withheld": ["Durum", "Malik", "Sır"]})}])
    case("Olmayan alan istenirse", [
        {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi", "fields": ["Ada", "Yok"]},
         "result": refused("field_not_found", "fields[1]")}])
    case("Formsuz, alanın kendisi “dışarı verilmez”: malik istense de taşınmaz", [
        {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi", "fields": ["Ada", "Malik"]},
         "result": done({"fields": ["Ada"], "withheld": ["Durum", "Malik", "Parsel", "Sır"]})}], "withheld")
    case("redact yoksa malik de taşınır (alanlar tanımlı: yalnız tanımlılar)", [
        {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi"},
         "result": done({"fields": ["Ada", "Parsel", "Durum", "Malik"], "withheld": ["Sır"]})}], "no-form")
    case("Sabitlenen durum: plan'ın pin'i aynı çizimde kabul edilir", [
        {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi"}, "as": "pin", "result": done()},
        {"op": "execute", "input": {"path": P, "sheet": "atlas-paftasi", "pin": "$pin"}, "result": done(),
         "file": {"exists": True, "pdfPages": 100}}])
    case("Plan'dan sonra çizim değişirse pin reddedilir (çatışma) ve dosya yazılmaz", [
        {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi"}, "as": "pin", "result": done()},
        {"command": "cad.polygon.create",
         "input": {"layerId": "parsel", "pts": [{"x": 1.0, "y": 1.0}, {"x": 5.0, "y": 1.0}, {"x": 5.0, "y": 4.0}]},
         "result": done()},
        {"op": "execute", "input": {"path": P, "sheet": "atlas-paftasi", "pin": "$pin"},
         "result": refused("pin_mismatch", "pin", "conflict"), "file": {"exists": False}}])
    case("Değişiklik geri alınınca içerik aynıdır: pin kabul edilir", [
        {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi"}, "as": "pin", "result": done()},
        {"command": "cad.polygon.create",
         "input": {"layerId": "parsel", "pts": [{"x": 1.0, "y": 1.0}, {"x": 5.0, "y": 1.0}, {"x": 5.0, "y": 4.0}]},
         "result": done()},
        {"op": "undo"},
        {"op": "execute", "input": {"path": P, "sheet": "atlas-paftasi", "pin": "$pin"}, "result": done(),
         "file": {"exists": True, "pdfPages": 100}}])
    case("Pin'siz çalıştırma çizimin o anki durumundan yapılır ve pin'ini söyler", [
        {"op": "execute", "input": {"path": P, "sheet": "atlas-paftasi"},
         "result": done({"pin": {"data": [{"layer": "parsel", "role": "coverage", "objects": 105},
                                          {"layer": "yol", "role": "map", "objects": 4}]}}),
         "file": {"exists": True, "pdfPages": 100}}])
    return out


GPKG = ROOT / "fixtures/atlas/v1/atlas.gpkg"
# The part a bound layer holds (docs/adr/0638 §3): the first three columns of parcels and their buildings.
PART = [ORIGIN[0] - 1.0, ORIGIN[1] - 1.0, ORIGIN[0] + 3 * (WIDTH + GAP) - 1.0, ORIGIN[1] + ROWS * (HEIGHT + GAP)]


def building(p):
    """The building on parcel `p`: 10 × 6 m in its middle."""
    cx, cy = p["x"] + p["w"] / 2, p["y"] + p["h"] / 2
    return (cx - 5.0, cy - 3.0, cx + 5.0, cy + 3.0)


def write_gpkg():
    """The source of the coverage cases, written by GDAL: `parsel` (the drawing's parcels and attributes) and
    `bina` (a building in each)."""
    from osgeo import gdal, ogr, osr
    gdal.UseExceptions()
    GPKG.unlink(missing_ok=True)
    ds = ogr.GetDriverByName("GPKG").CreateDataSource(str(GPKG))
    srs = osr.SpatialReference()
    srs.ImportFromEPSG(5253)
    parsel = ds.CreateLayer("parsel", srs, ogr.wkbPolygon)
    for name, kind in [("Ada", ogr.OFTString), ("Parsel", ogr.OFTInteger), ("Durum", ogr.OFTString)]:
        parsel.CreateField(ogr.FieldDefn(name, kind))
    bina = ds.CreateLayer("bina", srs, ogr.wkbPolygon)
    bina.CreateField(ogr.FieldDefn("Parsel", ogr.OFTInteger))

    def ring(x0, y0, x1, y1):
        return ogr.CreateGeometryFromWkt(f"POLYGON(({x0} {y0},{x1} {y0},{x1} {y1},{x0} {y1},{x0} {y0}))")

    for p in parcels():
        f = ogr.Feature(parsel.GetLayerDefn())
        f.SetField("Ada", p["ada"])
        f.SetField("Parsel", p["n"])
        f.SetField("Durum", p["durum"])
        f.SetGeometry(ring(p["x"], p["y"], p["x"] + p["w"], p["y"] + p["h"]))
        parsel.CreateFeature(f)
        g = ogr.Feature(bina.GetLayerDefn())
        g.SetField("Parsel", p["n"])
        g.SetGeometry(ring(*building(p)))
        bina.CreateFeature(g)
    ds = None
    print(f"yazıldı: {GPKG.relative_to(ROOT)}")


def map_box(page):
    """The ground an atlas page's map shows: its middle, the content (165 × 150 mm) at its scale."""
    w, h = CONTENT_MM[0] / 1000 * page["scale"], CONTENT_MM[1] / 1000 * page["scale"]
    c = page["center"]
    return (c["x"] - w / 2, c["y"] - h / 2, c["x"] + w / 2, c["y"] + h / 2)


def meets(a, b):
    return a[0] <= b[2] and a[2] >= b[0] and a[1] <= b[3] and a[3] >= b[1]


def source_cases(case, P):
    """Reading the coverage layer and the maps' parts from their source (docs/adr/0638 §3). The two tables are bound
    onto new layers holding the part `PART`; the atlas's coverage is the parcels' layer."""
    every = [p for p in parcels()]
    whole = sorted(every, key=lambda p: p["n"])
    pages_ = [{"index": k + 1, "name": f"{p['ada']}/{p['n']}", "scale": scale_of(p["w"], p["h"]),
               "center": {"x": p["x"] + p["w"] / 2, "y": p["y"] + p["h"] / 2},
               "bbox": [p["x"], p["y"], p["x"] + p["w"], p["y"] + p["h"]]} for k, p in enumerate(whole)]
    # The buildings each page's map reaches, counted page after page.
    buildings = sum(sum(1 for q in every if meets(building(q), map_box(page))) for page in pages_)
    bind = [
        {"command": "cad.sources.edit", "input": {"edit": {"kind": "link", "path": "$tmp/atlas.gpkg",
                                                           "table": "parsel", "extent": PART}},
         "take": {"parsel": "/link/layer"}, "result": done()},
        {"command": "cad.sources.edit", "input": {"edit": {"kind": "link", "path": "$tmp/atlas.gpkg",
                                                           "table": "bina", "extent": PART}},
         "take": {"bina": "/link/layer"}, "result": done()},
        {"command": "cad.sheets.edit", "input": {"ops": [{"op": "setAtlas", "sheet": "atlas-paftasi", "atlas": {
            "layer": "$parsel", "filter": "", "sort": [{"expression": "Parsel", "descending": False}],
            "pageName": "Ada || '/' || Parsel"}}]}, "result": done()},
    ]
    copy = {"op": "copy", "from": "fixtures/atlas/v1/atlas.gpkg", "to": "$tmp/atlas.gpkg"}
    in_part = [p for p in whole if meets((p["x"], p["y"], p["x"] + p["w"], p["y"] + p["h"]), PART)
               and p["x"] >= PART[0] and p["x"] + p["w"] <= PART[2]]
    case("Kaynaktan: parçayla bağlı katmanın bütün tablosu sayfa olur (105 parsel), okunan satır sayılır", [copy, *bind,
        {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi", "coverage": "source"},
         "result": done({"pages": pages_, "filteredOut": 0,
                         "read": [{"layer": "$parsel", "table": "parsel", "rows": 105},
                                  {"layer": "$bina", "table": "bina", "rows": buildings, "pages": 105}]})}], "source")
    case("Çizimden (varsayılan): parçanın nesneleri sayfa olur", [copy, *bind,
        {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi"},
         "result": done({"pages": [dict(p, index=k + 1) for k, p in enumerate(
             pg for pg in pages_ if any(pg["name"] == f"{q['ada']}/{q['n']}" for q in in_part))]})}], "source")
    case("Kaynaktan: her sayfanın haritası parçalı bina katmanını o sayfanın erişiminde kaynaktan okur", [copy, *bind,
        {"op": "execute", "input": {"path": P, "sheet": "atlas-paftasi", "coverage": "source"},
         "result": done({"read": [{"layer": "$parsel", "table": "parsel", "rows": 105},
                                  {"layer": "$bina", "table": "bina", "rows": buildings, "pages": 105}]}),
         "file": {"exists": True, "pdfPages": 105}}], "source")
    case("Bağlı olmayan kapsam katmanı kaynaktan okunamaz", [
        {"op": "plan", "input": {"path": P, "sheet": "atlas-paftasi", "coverage": "source"},
         "result": refused("coverage_not_bound", "coverage")}])


def document():
    setups = {
        "main": setup(),
        "no-sheets": setup(sheets=False),
        "no-layer": setup(atlas={"layer": "yok", "filter": "", "sort": [], "pageName": "Ada"}),
        "empty": setup(atlas={"layer": "parsel", "filter": "Parsel > 100000", "sort": [], "pageName": "Ada"}),
        "page-filter": setup(atlas={"layer": "parsel", "filter": "Parsel > 10 ve Parsel <= 20",
                                    "sort": [{"expression": "Parsel", "descending": False}],
                                    "pageName": "Ada || '/' || Parsel"}),
        "states": setup(states=True),
        "temporal": setup(temporal=True),
        "no-form": setup(form=False),
        # No parcels: the source's tables are bound onto new layers (docs/adr/0638 §3).
        "source": setup(parcels_=False),
        "withheld": setup(form=False, withheld=True),
    }
    return {
        "format": "kentos.file-command-cases", "version": 1, "command": "file.export.atlas", "commandVersion": 1,
        "title": "Atlası PDF yap: sayfalar, sabitlenen durum, açık bağlar ve izinli öznitelikler",
        "note": ("ADR 0534. Çizimi ve beklenen sayfaları scripts/fixtures/atlas_cases.py KentOS kodu olmadan kurar; "
                 "çıktının PDF'i poppler ve GDAL'la `--files` ile okunur. Koşucu crates/native/headless/tests/headless/"
                 "atlas_cases.rs. Elle düzenlemeyin."),
        "placeholders": {"$tmp": "Her durum için boş bir geçici klasör.", "$pin": "Adı `as` ile verilmiş önceki adımın "
                         "çıktısındaki pin."},
        "setups": setups, "cases": cases(),
    }


def text_of(pdf, first, last):
    return subprocess.run(["pdftotext", "-f", str(first), "-l", str(last), "-layout", str(pdf), "-"],
                          check=True, capture_output=True, text=True).stdout


def info_of(pdf):
    out = subprocess.run(["pdfinfo", str(pdf)], check=True, capture_output=True, text=True).stdout
    return dict(line.split(":", 1) for line in out.splitlines() if ":" in line)


def gdal_page(pdf, page):
    out = subprocess.run(["gdalinfo", "-json", f"PDF:{page}:{pdf}"], check=True, capture_output=True, text=True).stdout
    return json.loads(out)


def check_files():
    pdf = FILES / "parsel-atlasi.pdf"
    if not pdf.exists():
        sys.exit(f"{pdf} yok: KENTOS_WRITE_ATLAS=1 cargo test -p kentos-headless --test headless atlas")
    pin = json.loads((FILES / "parsel-atlasi.pin.json").read_text())
    want = pages()
    problems = []
    info = {k.strip(): v.strip() for k, v in info_of(pdf).items()}
    if info.get("Pages") != str(len(want)):
        problems.append(f"sayfa sayısı {info.get('Pages')} (beklenen {len(want)})")
    keywords = info.get("Keywords", "")
    if "kentos.atlas/1" not in keywords or f"durum={pin['digest']}" not in keywords or "koordinat=EPSG:5253" not in keywords:
        problems.append(f"Keywords: {keywords}")
    if "A4" not in info.get("Page size", ""):
        problems.append(f"sayfa boyu {info.get('Page size')}")
    whole = text_of(pdf, 1, len(want))
    for secret in ("Gizli Kişi", "gizli-not", "durum iptal", "iptal"):
        if secret in whole:
            problems.append(f"PDF'te olmaması gereken metin: {secret!r}")
    sirf = ["‹Malik?›", "‹Sır?›"]
    for missing in sirf:
        if missing not in whole:
            problems.append(f"gizli öznitelik eksik işaretiyle yok: {missing}")
    # The parcels cancelled are on no page: every page is one of the wanted names, in order.
    for k, page in enumerate(want, start=1):
        if k not in (1, 2, 4, 5, 19, 20, 57, 99, 100):
            continue
        t = text_of(pdf, k, k)
        ada, n = page["name"].split("/")
        for needle in (page["name"], f"Ada {ada} parsel {n} durum aktif", f"Sayfa {k} / {len(want)}",
                       f"1/{page['scale']}"):
            if needle not in t:
                problems.append(f"sayfa {k}: {needle!r} yok")
        # The table lists the parcels of the atlas object: its own, once.
        rows = [line for line in t.splitlines() if re.match(r"\s*A\d+\s+\d+\s+‹Malik\?›", line)]
        if len(rows) != 1 or f" {n} " not in rows[0] + " ":
            problems.append(f"sayfa {k}: tablo satırları {rows}")
        g = gdal_page(pdf, k)
        wkt = g.get("coordinateSystem", {}).get("wkt", "")
        if "TUREF / TM27" not in wkt:
            problems.append(f"sayfa {k}: koordinat sistemi {wkt[:40]!r}")
        neat = g.get("metadata", {}).get("", {}).get("NEATLINE", "")
        pts = [tuple(map(float, p.split())) for p in re.findall(r"([-\d.]+ [-\d.]+)", neat)]
        if not pts:
            problems.append(f"sayfa {k}: neatline yok")
            continue
        xs, ys = [p[0] for p in pts], [p[1] for p in pts]
        w = CONTENT_MM[0] * page["scale"] / 1000
        h = CONTENT_MM[1] * page["scale"] / 1000
        got = ((min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2, max(xs) - min(xs), max(ys) - min(ys))
        exp = (page["center"]["x"], page["center"]["y"], w, h)
        if any(abs(a - b) > 0.01 for a, b in zip(got, exp)):
            problems.append(f"sayfa {k}: haritanın yeri {got} (beklenen {exp})")
    if problems:
        print("\n".join(problems))
        sys.exit(1)
    print(f"{pdf.name}: {len(want)} sayfa, poppler ve GDAL'la denetlendi")


def main():
    args = sys.argv[1:]
    if "--files" in args:
        return check_files()
    if "--gpkg" in args:
        return write_gpkg()
    text = json.dumps(document(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in args:
        if not OUT.exists() or OUT.read_text() != text:
            sys.exit(f"{OUT} güncel değil: python3 scripts/fixtures/atlas_cases.py")
        print("güncel")
        return
    OUT.write_text(text)
    print(f"yazıldı: {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
