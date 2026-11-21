"""Writes fixtures/commands/v1/cad.layers.export.json: the shared cases of `cad.layers.export` v1 (docs/adr/9106).

Every expectation is worked out here from the contract's rules (crates/shared/commands/src/gis/cad_layers_export.rs):
which objects or rows are written (the selection, the box, the condition), the fields chosen, renamed and given their
labels, the geometry made one kind (an area's outline for a line), multi-part, with Z; the table read back through
`cad.layers.rows` once the written table is added to the map; a CSV's and a GeoJSON's text read back whole; a
Shapefile's `.cpg` and `.prj`; the refusals and their order. No KentOS code is used.

The data layer reads fixtures/data/v1/satirlar.gpkg, which scripts/fixtures/layer_rows_command_cases.py writes; its rows
are that script's own definition. The `.prj` text is fixtures/crs/v1/esri-prj.json's. The desktop
(crates/native/application/tests/all/fixtures) and Python (python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/layer_export_command_cases.py           # writes the file
    python3 scripts/fixtures/layer_export_command_cases.py --check   # compares the cases with the ones on disk
"""
import copy
import importlib.util
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.layers.export.json"
ADDRESS = "$fixture:data/v1/satirlar.gpkg"
PRJ_5256 = json.loads((ROOT / "fixtures/crs/v1/esri-prj.json").read_text("utf-8"))["prj"]["5256"]


def _rows_of_the_table():
    spec = importlib.util.spec_from_file_location("rows_cases", ROOT / "scripts/fixtures/layer_rows_command_cases.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.ROWS


TABLE = _rows_of_the_table()
STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}

# The point layer's objects: use, area, state code.
POINTS = [("Konut", "120", "1"), ("Park", "480.5", "2"), ("Konut", "900", "1"), ("Ticaret", "1500", None),
          ("Eğitim", "60", "2")]
DURUM = [{"code": "1", "label": "Sağlam"}, {"code": "2", "label": "Bakımda"}]


def xy(i):
    return 600000.0 + 10 * i, 4410000.0 + 5 * i


def entities():
    out = []
    for i, (use, area, state) in enumerate(POINTS):
        attrs = {"kullanim": use, "alan": area}
        if state is not None:
            attrs["durum"] = state
        x, y = xy(i)
        out.append({"kind": "point", "id": i + 1, "layerId": "noktalar", "attrs": attrs, "p": {"x": x, "y": y}})
    out.append({"kind": "point", "id": 20, "layerId": "karisik", "attrs": {"ad": "A"}, "p": {"x": 600100.0, "y": 4410100.0}})
    out.append({"kind": "text", "id": 21, "layerId": "karisik", "attrs": {"ad": "B"}, "p": {"x": 600110.0, "y": 4410100.0},
                "text": "Yazı", "height": 2.5, "rotation": 0})
    return out


TABLE_FIELDS = [{"name": "ada", "kind": "integer"}, {"name": "parsel", "kind": "integer"},
                {"name": "mahalle", "kind": "text"}, {"name": "alan", "kind": "decimal"},
                {"name": "kullanim", "kind": "text"}]


def layer(lid, name, **more):
    node = {"id": lid, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": STYLE, "children": []}
    node.update(more)
    return node


SETUP = {
    "format": "kentos.document",
    "version": 1,
    "name": "Katmanı dışa aktar",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                 "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
    "origin": {"x": 600000, "y": 4410000},
    "layers": [
        layer("noktalar", "Noktalar", geometry="point",
              fields=[{"name": "kullanim", "kind": "text"}, {"name": "alan", "kind": "decimal"},
                      {"name": "durum", "kind": "text", "values": DURUM}]),
        layer("karisik", "Karışık", fields=[{"name": "ad", "kind": "text"}]),
        layer("parseller", "Parseller", geometry="area",
              data={"driver": "gpkg", "address": ADDRESS, "table": "parseller", "key": "fid",
                    "geometry": {"column": "geom", "typeName": "POLYGON", "srid": 5256}},
              fields=TABLE_FIELDS),
        {"id": "grup", "name": "Grup", "type": "group", "visible": True, "locked": False, "expanded": True,
         "style": STYLE, "children": []},
        layer("altlik", "OpenStreetMap", service={"kind": "xyz", "url": "https://tile.openstreetmap.org/{z}/{x}/{y}.png", "maxZoom": 19}),
    ],
    "activeLayer": "noktalar",
    "entities": entities(),
    "styles": {"items": [], "categories": []},
}


# ── What is written, worked out here ────────────────────────────────────────────────────────────────────────

def num(v):
    """A number as WKT and a table read it back: an integral one bare, else its shortest form."""
    v = float(v)
    return str(int(v)) if v.is_integer() else repr(v)


def ring(r):
    s = r["side"]
    return [(r["x"], r["y"]), (r["x"] + s, r["y"]), (r["x"] + s, r["y"] + s), (r["x"], r["y"] + s), (r["x"], r["y"])]


def coords(pts, z=False):
    return ", ".join(f"{num(x)} {num(y)}" + (" 0" if z else "") for x, y in pts)


def polygon_wkt(r, kind="POLYGON", z=False):
    dims = " Z" if z else ""
    body = f"(({coords(ring(r), z)}))"
    if kind == "MULTIPOLYGON":
        return f"MULTIPOLYGON{dims} ({body})"
    if kind == "LINESTRING":
        return f"LINESTRING{dims} ({coords(ring(r), z)})"
    return f"POLYGON{dims} {body}"


def table_attrs(r, fields):
    out = {}
    for name, to in fields:
        v = r[name]
        if v is None:
            continue
        out[to] = v if isinstance(v, str) else num(v)
    return out


def completed(output, warnings=None):
    return {"status": "completed", "output": output, "warnings": warnings or []}


def refused(code, message, path):
    error = {"code": code, "message": message}
    if path is not None:
        error["path"] = path
    return {"status": "failed", "error": error}


def exported(fmt, files, table, written, fields, geometry, srid, layer=None, warnings=None):
    out = {"format": fmt, "files": files}
    if table is not None:
        out["table"] = table
    out["written"] = written
    out["fields"] = fields
    if geometry is not None:
        out["geometry"] = geometry
    if srid is not None:
        out["srid"] = srid
    if layer is not None:
        out["layer"] = layer
    out["revision"] = "$current"
    return completed(out, warnings)


def export(inp, result, op="execute"):
    return {"op": op, "input": inp, "result": result}


def rows_step(layer, fields, rows, geometry=False):
    inp = {"layer": layer, "count": True, "limit": 1000}
    if geometry:
        inp["geometry"] = True
    out = {"fields": fields, "rows": rows, "offset": 0, "total": len(rows), "checked": "source"}
    return {"op": "execute", "command": "cad.layers.rows", "input": inp, "result": completed(out)}


def read_file(path, returns, kind="text"):
    return {"op": "readFile", "from": path, "as": kind, "returns": returns}


def case(name, steps, setup=None):
    out = {"name": name, "steps": steps}
    if setup is not None:
        out["setup"] = setup
    return out


def table_rows(rows, fields, geometry=None, kind="POLYGON", z=False):
    out = []
    for i, r in enumerate(rows):
        row = {"key": i + 1, "attrs": table_attrs(r, fields)}
        if geometry:
            row["wkt"] = polygon_wkt(r, kind, z)
        out.append(row)
    return out


ALL = [(f, f) for f in ["ada", "parsel", "mahalle", "alan", "kullanim"]]
SKIPPED_TEXT = [{"code": "export_skipped", "message": "1 nesne yazılamadı; ilki: yazı ya da ölçü bir geometri sütununa yazılmaz."}]


def cases():
    out = []
    konut = [r for r in TABLE if r["kullanim"] == "Konut"]
    tmp = lambda name: f"$tmp:disa/{name}"

    # GeoPackage: the data layer's rows a condition keeps, the table added to the map and read back.
    out.append(case("GeoPackage: süzgecin tuttuğu satırlar yeni tabloya, haritaya eklenip geri okunur", [
        export({"layer": "parseller", "format": "gpkg", "path": tmp("konut.gpkg"), "name": "konut",
                "filter": "kullanim = 'Konut'", "addToMap": True},
               exported("gpkg", [tmp("konut.gpkg")], "konut", len(konut), [f for f, _ in ALL], "POLYGON", 5256,
                        "layer-1")),
        rows_step("layer-1", [f for f, _ in ALL], table_rows(konut, ALL, True), geometry=True),
    ]))
    # Fields chosen, renamed, in their order.
    chosen = [("kullanim", "kullanim"), ("ada", "ada_no")]
    out.append(case("GeoPackage: alanlar seçilir, sırası ve adı verilir", [
        export({"layer": "parseller", "format": "gpkg", "path": tmp("alanlar.gpkg"), "name": "secilen",
                "fields": [{"name": "kullanim"}, {"name": "ada", "to": "ada_no"}], "keys": [1, 2, 3], "addToMap": True},
               exported("gpkg", [tmp("alanlar.gpkg")], "secilen", 3, ["kullanim", "ada_no"], "POLYGON", 5256, "layer-1")),
        rows_step("layer-1", ["kullanim", "ada_no"], table_rows(TABLE[:3], chosen)),
    ]))
    # The geometry made one kind: an area's outline as a line; multi-part; with Z.
    for title, extra, kind, z in [
        ("alanın çevresi çizgi olarak yazılır", {"geometry": "line"}, "LINESTRING", False),
        ("çok parçalıya zorlanır", {"multi": True}, "MULTIPOLYGON", False),
        ("Z boyutu eklenir (kotsuz nesne 0)", {"z": True}, "POLYGON", True),
    ]:
        inp = {"layer": "parseller", "format": "gpkg", "path": tmp(f"geo-{kind}-{z}.gpkg"), "name": "geo",
               "keys": [4, 5], "fields": [{"name": "parsel"}], "addToMap": True, **extra}
        type_name = kind + (" Z" if z else "")
        out.append(case(f"GeoPackage: {title}", [
            export(inp, exported("gpkg", [inp["path"]], "geo", 2, ["parsel"], type_name, 5256, "layer-1")),
            rows_step("layer-1", ["parsel"], table_rows(TABLE[3:5], [("parsel", "parsel")], True, kind, z),
                      geometry=True),
        ]))
    # The selection, the box.
    box = [600040.0, 4410040.0, 600160.0, 4410110.0]
    meets = [r for r in TABLE if r["x"] <= box[2] and r["x"] + r["side"] >= box[0] and r["y"] <= box[3]
             and r["y"] + r["side"] >= box[1]]
    out.append(case("GeoPackage: kutuya değen satırlar", [
        export({"layer": "parseller", "format": "gpkg", "path": tmp("kutu.gpkg"), "name": "kutu", "bbox": box,
                "fields": [{"name": "ada"}, {"name": "parsel"}], "addToMap": True},
               exported("gpkg", [tmp("kutu.gpkg")], "kutu", len(meets), ["ada", "parsel"], "POLYGON", 5256, "layer-1")),
        rows_step("layer-1", ["ada", "parsel"], table_rows(meets, [("ada", "ada"), ("parsel", "parsel")])),
    ]))
    # The target there: refused; written over the table; added to.
    def first_in(file):
        return {"layer": "parseller", "format": "gpkg", "path": tmp(file), "name": "var", "keys": [1, 2],
                "fields": [{"name": "ada"}]}

    first = first_in("var1.gpkg")
    out.append(case("Var olan tablo: verilmezse yazılmaz (target_exists)", [
        export(first, exported("gpkg", [tmp("var1.gpkg")], "var", 2, ["ada"], "POLYGON", 5256)),
        export(first, refused("target_exists", "“var” tablosu GeoPackage kaynağında var. Üstüne yazmayı, katmanı "
                              "değiştirmeyi ya da eklemeyi seçin (existing).", "existing")),
    ]))
    first = first_in("var2.gpkg")
    out.append(case("Var olan tablo: katmanı değiştir (tablo yeniden yapılır)", [
        export(first, exported("gpkg", [tmp("var2.gpkg")], "var", 2, ["ada"], "POLYGON", 5256)),
        export({**first, "keys": [7], "existing": "replaceLayer", "addToMap": True},
               exported("gpkg", [tmp("var2.gpkg")], "var", 1, ["ada"], "POLYGON", 5256, "layer-1")),
        rows_step("layer-1", ["ada"], table_rows([TABLE[6]], [("ada", "ada")])),
    ]))
    first = first_in("var3.gpkg")
    out.append(case("Var olan tablo: ekle; tabloda olmayan alan bırakılır (field_left_out)", [
        export(first, exported("gpkg", [tmp("var3.gpkg")], "var", 2, ["ada"], "POLYGON", 5256)),
        export({**first, "keys": [9], "fields": [{"name": "ada"}, {"name": "mahalle"}], "existing": "append",
                "addToMap": True},
               exported("gpkg", [tmp("var3.gpkg")], "var", 1, ["ada", "mahalle"], "POLYGON", 5256, "layer-1",
                        [{"code": "field_left_out", "message": "Eklenen tabloda “mahalle” alanı yok; değerleri yazılmadı.",
                          "path": "fields"}])),
        # The layer added has the table's fields: the one left out is not among them.
        rows_step("layer-1", ["ada"], [
            {"key": i + 1, "attrs": {"ada": num(r["ada"])}} for i, r in enumerate([TABLE[0], TABLE[1], TABLE[8]])
        ]),
    ]))
    first = first_in("var4.gpkg")
    out.append(case("Var olan dosya: üstüne yaz (dosyanın öbür tabloları gider)", [
        export(first, exported("gpkg", [tmp("var4.gpkg")], "var", 2, ["ada"], "POLYGON", 5256)),
        export({**first, "name": "yeni", "keys": [3], "existing": "overwriteFile", "addToMap": True},
               exported("gpkg", [tmp("var4.gpkg")], "yeni", 1, ["ada"], "POLYGON", 5256, "layer-1")),
        rows_step("layer-1", ["ada"], table_rows([TABLE[2]], [("ada", "ada")])),
    ]))
    # A drawing layer: its points; a text left out of a table with geometry, said.
    pts = [{"key": i + 1, "attrs": {k: v for k, v in [("kullanim", u), ("alan", num(a)), ("durum", s)] if v is not None}}
           for i, (u, a, s) in enumerate(POINTS)]
    out.append(case("Çizim katmanı: noktaları GeoPackage'a", [
        export({"layer": "noktalar", "format": "gpkg", "path": tmp("noktalar.gpkg"), "addToMap": True},
               exported("gpkg", [tmp("noktalar.gpkg")], "noktalar", 5, ["kullanim", "alan", "durum"], "POINT", 5256,
                        "layer-1")),
        rows_step("layer-1", ["kullanim", "alan", "durum"], pts),
    ]))
    out.append(case("Çizim katmanı: yazı geometri sütununa yazılmaz, sayılıp söylenir", [
        export({"layer": "karisik", "format": "gpkg", "path": tmp("karisik.gpkg"), "geometry": "point"},
               exported("gpkg", [tmp("karisik.gpkg")], "karisik", 1, ["ad"], "POINT", 5256, None, SKIPPED_TEXT)),
    ]))
    # GeoJSON: the project's coordinates, typed values, the labels of a listed field.
    features = []
    for i, (u, a, s) in enumerate(POINTS):
        x, y = xy(i)
        props = {"kullanim": u, "alan": float(a)}
        if s is not None:
            props["durum"] = next(d["label"] for d in DURUM if d["code"] == s)
        features.append({"type": "Feature", "properties": props,
                         "geometry": {"type": "Point", "coordinates": [x, y]}, "kentos": {"layer": "Noktalar"}})
    out.append(case("GeoJSON: proje koordinatları, sayılar sayı, listeli alan etiketiyle", [
        export({"layer": "noktalar", "format": "geojson", "path": tmp("noktalar.geojson"),
                "fields": [{"name": "kullanim"}, {"name": "alan"}, {"name": "durum", "labels": True}],
                "options": {"projectCoordinates": True}},
               exported("geojson", [tmp("noktalar.geojson")], None, 5, ["kullanim", "alan", "durum"], "POINT", 5256)),
        read_file(tmp("noktalar.geojson"), {"type": "FeatureCollection", "name": "Noktalar", "crs": "$any",
                                            "features": features}, "json"),
    ]))
    # CSV: WKT, comma; X and Y, a semicolon; Windows-1254.
    head = "kullanim,alan,durum,wkt\r\n"
    lines = "".join(f"{u},{num(a)},{s or ''},POINT ({num(xy(i)[0])} {num(xy(i)[1])})\r\n"
                    for i, (u, a, s) in enumerate(POINTS))
    out.append(case("CSV: alanlar ve WKT, virgülle", [
        export({"layer": "noktalar", "format": "csv", "path": tmp("noktalar.csv")},
               exported("csv", [tmp("noktalar.csv")], None, 5, ["kullanim", "alan", "durum"], "POINT", None)),
        read_file(tmp("noktalar.csv"), head + lines),
    ]))
    head_xy = "kullanim;x;y\r\n"
    lines_xy = "".join(f"{u};{num(xy(i)[0])};{num(xy(i)[1])}\r\n" for i, (u, _, _) in enumerate(POINTS))
    out.append(case("CSV: X ve Y sütunları, noktalı virgül, Windows-1254", [
        export({"layer": "noktalar", "format": "csv", "path": tmp("xy.csv"), "fields": [{"name": "kullanim"}],
                "encoding": "windows1254", "options": {"separator": ";", "xy": True}},
               exported("csv", [tmp("xy.csv")], None, 5, ["kullanim"], "POINT", None)),
        read_file(tmp("xy.csv"), head_xy + lines_xy, "windows1254"),
    ]))
    out.append(case("CSV: değeri ayırıcı ya da tırnak taşıyan hücre tırnağa alınır", [
        export({"layer": "noktalar", "format": "csv", "path": tmp("geometrisiz.csv"), "geometry": "none",
                "fields": [{"name": "kullanim", "to": "kullanım, tür"}], "keys": None},
               exported("csv", [tmp("geometrisiz.csv")], None, 5, ["kullanım, tür"], None, None)),
        read_file(tmp("geometrisiz.csv"), "\"kullanım, tür\"\r\n" + "".join(f"{u}\r\n" for u, _, _ in POINTS)),
    ]))
    # Shapefile: its files, the .cpg naming the encoding, the .prj the system.
    for enc, cpg in [("utf8", "UTF-8"), ("windows1254", "1254")]:
        files = [tmp(f"shp-{enc}/noktalar.{e}") for e in ("shp", "shx", "dbf", "prj", "cpg")]
        out.append(case(f"Shapefile ({cpg}): dosyaları, .cpg kodlamayı, .prj sistemi söyler", [
            export({"layer": "noktalar", "format": "shapefile", "path": tmp(f"shp-{enc}/noktalar.shp"), "encoding": enc},
                   exported("shapefile", files, None, 5, ["kullanim", "alan", "durum"], "POINT", 5256)),
            read_file(tmp(f"shp-{enc}/noktalar.cpg"), cpg),
            read_file(tmp(f"shp-{enc}/noktalar.prj"), PRJ_5256),
        ]))
    # KML: always WGS 84.
    out.append(case("KML: her zaman WGS 84", [
        export({"layer": "noktalar", "format": "kml", "path": tmp("noktalar.kml")},
               exported("kml", [tmp("noktalar.kml")], None, 5, ["kullanim", "alan", "durum"], "POINT", 4326)),
        read_file(tmp("noktalar.kml"), "$any"),
    ]))
    # KML's symbols (docs/adr/9112 §4): the layer's style, its colour, none.
    out += kml_symbol_cases(tmp)
    # Plan: the rows made, nothing written.
    out.append(case("Plan: yazılacak satırları sayar, hiçbir şey yazmaz", [
        export({"layer": "parseller", "format": "gpkg", "path": tmp("plan.gpkg"), "filter": "kullanim = 'Konut'"},
               exported("gpkg", [], "parseller", len(konut), [f for f, _ in ALL], "POLYGON", 5256), op="plan"),
    ]))
    # Refusals, in their order.
    out += [
        case("Ret: katman verilmedi", [export({"layer": " ", "format": "gpkg", "path": tmp("x.gpkg")},
                                              refused("no_layer", "Dışa aktarılacak katmanı (layer) verin.", "layer"))]),
        case("Ret: dosya yolu yok", [export({"layer": "noktalar", "format": "shapefile"},
                                            refused("no_target", "Yazılacak dosyanın yolunu verin (path).", "path"))]),
        case("Ret: PostGIS'in adresi yok", [export({"layer": "noktalar", "format": "postgis"}, refused(
            "no_target", "PostGIS'e yazmak için veritabanının adresini verin (address): "
            "postgresql://kullanıcı@sunucu:port/veritabanı.", "address"))]),
        case("Ret: biçimde olmayan seçenek", [export(
            {"layer": "noktalar", "format": "gpkg", "path": tmp("x.gpkg"), "options": {"separator": ";"}},
            refused("invalid_option", "Ayırıcı bu biçimde yoktur; GeoPackage için verilmez.", "options.separator"))]),
        case("Ret: Shapefile alan adı 10 karakteri geçer", [export(
            {"layer": "noktalar", "format": "shapefile", "path": tmp("x.shp"),
             "fields": [{"name": "kullanim", "to": "kullanim_turu"}]},
            refused("invalid_name", "Alanın yazılacak adı boş ya da 10 karakterden uzun olamaz.", "fields[0].to"))]),
        case("Ret: katman yok", [export({"layer": "yok", "format": "gpkg", "path": tmp("x.gpkg")},
                                        refused("layer_not_found", "“yok” kimlikli katman çizimde yok.", "layer"))]),
        case("Ret: grup", [export({"layer": "grup", "format": "gpkg", "path": tmp("x.gpkg")}, refused(
            "not_a_layer", "“Grup” bir katman grubu; dışa aktarılan bir katmandır. Grubun bir katmanını verin.",
            "layer"))]),
        case("Ret: alan yok", [export({"layer": "noktalar", "format": "gpkg", "path": tmp("x.gpkg"),
                                       "fields": [{"name": "kat"}]},
                                      refused("field_not_found", "“kat” alanı “Noktalar” katmanında yok.",
                                              "fields[0].name"))]),
        case("Ret: süzgeç derlenmiyor", [export({"layer": "noktalar", "format": "gpkg", "path": tmp("x.gpkg"),
                                                 "filter": "kullanim ="},
                                                refused("invalid_expression", "$any", "filter"))]),
        case("Ret: hiçbir şey kalmadı", [export({"layer": "parseller", "format": "gpkg", "path": tmp("bos.gpkg"),
                                                 "filter": "ada > 10000"}, refused(
            "nothing_to_export", "Yazılacak nesne kalmadı: seçim, kutu ve süzgeç hiçbir nesne bırakmadı.", "layer"))]),
    ]
    # A map service's layer (no object, no row), a system the project cannot write in, a target that cannot be made
    # (a path inside a file).
    out.extend([
        case("Ret: harita servisi katmanı", [export({"layer": "altlik", "format": "gpkg", "path": tmp("servis.gpkg")},
                                                    refused("service_layer", "“OpenStreetMap” bir harita servisinden "
                                                            "çizilir; nesnesi ya da satırı yoktur.", "layer"))]),
        case("Ret: bilinmeyen sistem", [export({"layer": "noktalar", "format": "gpkg", "path": tmp("sistem.gpkg"),
                                                "srid": 999999}, refused("unknown_srid", "$any", "srid"))]),
        case("Ret: hedef yapılamıyor (bir dosyanın içinde)", [
            export({"layer": "noktalar", "format": "gpkg", "path": "$fixture:geopackage/v1/qgis.gpkg/alt.gpkg"},
                   refused("source_unreachable", "$any", None))]),
    ])
    # A file there already: refused unless written over.
    out.append(case("Ret: dosya var (CSV)", [
        export({"layer": "noktalar", "format": "csv", "path": tmp("var.csv"), "fields": [{"name": "kullanim"}],
                "geometry": "none"},
               exported("csv", [tmp("var.csv")], None, 5, ["kullanim"], None, None)),
        export({"layer": "noktalar", "format": "csv", "path": tmp("var.csv"), "fields": [{"name": "kullanim"}],
                "geometry": "none"}, refused("target_exists", "$any", "existing")),
    ]))
    # The CSV case without a key list passed `keys: None` to keep the input simple: drop it.
    for c in out:
        for s in c["steps"]:
            if isinstance(s.get("input"), dict) and s["input"].get("keys", 0) is None:
                del s["input"]["keys"]
    return out


# ── KML's symbols (docs/adr/9112 §4) ──────────────────────────────────────────────────────────────────────

MM_PER_PIXEL = 25.4 / 96


def half_up(v):
    """Rounding as the writer rounds: halves away from zero (Rust's f64::round), not to even."""
    return math.floor(v + 0.5)


def kml_color(hex_color, opacity=1.0):
    """A `#rrggbb` colour as KML writes it, `aabbggrr`, its alpha the opacity."""
    a = half_up(max(0.0, min(1.0, opacity)) * 255)
    h = hex_color.lstrip("#").lower()
    return f"{a:02x}{h[4:6]}{h[2:4]}{h[0:2]}"


def two(v):
    """A number with two decimals at most, its trailing zeros left out."""
    s = f"{v:.2f}".rstrip("0").rstrip(".")
    return "0" if s == "-0" else s


def pixels(mm):
    return half_up(mm / MM_PER_PIXEL * 100) / 100


def icon_scale(mm):
    """A marker's size against KML's 32-pixel icon."""
    return half_up(pixels(mm) / 32 * 100) / 100


def marker(color, size, opacity=None):
    layer = {"type": "shape", "shape": "circle", "fill": color, "size": size}
    if opacity is not None:
        layer["opacity"] = opacity
    return {"marker": {"type": "marker", "layers": [layer]}}


def area(fill, outline=None, width=None, opacity=None):
    layers = [{"type": "simpleFill", "color": fill}]
    if opacity is not None:
        layers[0]["opacity"] = opacity
    if outline is not None:
        layers.append({"type": "simpleLine", "color": outline, "width": width})
    return {"fill": {"type": "fill", "layers": layers}}


def kml_symbol_cases(tmp):
    out = []
    # The point layer by its use: Konut and Park each a style, Ticaret switched off, the rest Diğer değerler.
    renderer = {"type": "categorized", "expr": "kullanim", "categories": [
        {"value": "Konut", "label": "Konut", "symbols": marker("#e31a1c", 3)},
        {"value": "Park", "label": "Park", "symbols": marker("#33a02c", 4, 0.5)},
        {"value": "Ticaret", "label": "Ticaret", "symbols": marker("#1f78b4", 3), "enabled": False},
    ], "other": marker("#999999", 2.5)}
    setup = copy.deepcopy(SETUP)
    for node in setup["layers"]:
        if node["id"] == "noktalar":
            node["style"] = dict(STYLE, renderer=renderer)
    styles = {
        "s1": {"icon": {"color": kml_color("#e31a1c"), "scale": two(icon_scale(3))}},
        "s2": {"icon": {"color": kml_color("#33a02c", 0.5), "scale": two(icon_scale(4))}},
        "s3": {"icon": {"color": kml_color("#999999"), "scale": two(icon_scale(2.5))}},
    }
    by_use = {"Konut": "#s1", "Park": "#s2", "Ticaret": None}
    placemarks = [by_use.get(use, "#s3") for use, _, _ in POINTS]
    out.append(case("KML: katmanın stili; her sınıf kendi stili, kapalı sınıfın yer imi stilsiz", [
        export({"layer": "noktalar", "format": "kml", "path": tmp("stil.kml"), "options": {"symbols": "style"}},
               exported("kml", [tmp("stil.kml")], None, 5, ["kullanim", "alan", "durum"], "POINT", 4326)),
        read_file(tmp("stil.kml"), {"styles": styles, "placemarks": placemarks}, "kmlStyles"),
    ], setup))
    # The data layer's areas by their use, the use not written: its rows are read with it all the same.
    uses = []
    for r in TABLE:
        if r["kullanim"] is not None and r["kullanim"] not in uses:
            uses.append(r["kullanim"])
    first, second = uses[0], uses[1]
    renderer = {"type": "categorized", "expr": "kullanim", "categories": [
        {"value": first, "label": first, "symbols": area("#d9c27a", "#333333", 0.5)},
        {"value": second, "label": second, "symbols": area("#2b7a3d")},
    ], "other": area("#cccccc", "#888888", 0.25, 0.5)}
    setup = copy.deepcopy(SETUP)
    for node in setup["layers"]:
        if node["id"] == "parseller":
            node["style"] = dict(STYLE, renderer=renderer)
    styles = {
        "s1": {"line": {"color": kml_color("#333333"), "width": two(pixels(0.5))},
               "poly": {"color": kml_color("#d9c27a"), "fill": "1", "outline": "1"}},
        "s2": {"poly": {"color": kml_color("#2b7a3d"), "fill": "1", "outline": "0"}},
        "s3": {"line": {"color": kml_color("#888888"), "width": two(pixels(0.25))},
               "poly": {"color": kml_color("#cccccc", 0.5), "fill": "1", "outline": "1"}},
    }
    placemarks = ["#s1" if r["kullanim"] == first else "#s2" if r["kullanim"] == second else "#s3" for r in TABLE]
    out.append(case("KML: veri katmanının alanları sınıflarının stiliyle; sınıfın alanı yazılmasa da", [
        export({"layer": "parseller", "format": "kml", "path": tmp("alanlar.kml"), "fields": [{"name": "ada"}],
                "options": {"symbols": "style"}},
               exported("kml", [tmp("alanlar.kml")], None, len(TABLE), ["ada"], "POLYGON", 4326)),
        read_file(tmp("alanlar.kml"), {"styles": styles, "placemarks": placemarks}, "kmlStyles"),
    ], setup))
    # None: no style; the layer's colour (the default): one style, its theme colour black.
    out.append(case("KML: sembolsüz; yer imlerinin stili yok", [
        export({"layer": "noktalar", "format": "kml", "path": tmp("sembolsuz.kml"), "options": {"symbols": "none"}},
               exported("kml", [tmp("sembolsuz.kml")], None, 5, ["kullanim", "alan", "durum"], "POINT", 4326)),
        read_file(tmp("sembolsuz.kml"), {"styles": {}, "placemarks": [None] * len(POINTS)}, "kmlStyles"),
    ]))
    color = {"s1": {"icon": {"color": "ff000000", "scale": None},
                    "line": {"color": "ff000000", "width": two(half_up(0.25 / MM_PER_PIXEL * 100) / 100)}}}
    out.append(case("KML: katmanın rengi; tek stil, tema rengi siyah", [
        export({"layer": "noktalar", "format": "kml", "path": tmp("renk.kml"), "options": {"symbols": "color"}},
               exported("kml", [tmp("renk.kml")], None, 5, ["kullanim", "alan", "durum"], "POINT", 4326)),
        read_file(tmp("renk.kml"), {"styles": color, "placemarks": ["#s1"] * len(POINTS)}, "kmlStyles"),
    ]))
    out.append(case("Ret: semboller yalnız KML'de", [
        export({"layer": "noktalar", "format": "geojson", "path": tmp("semboller.geojson"), "options": {"symbols": "style"}},
               refused("invalid_option", "Semboller bu biçimde yoktur; GeoJSON için verilmez.", "options.symbols")),
    ]))
    return out


def document():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.layers.export",
        "commandVersion": 1,
        "title": "Katmanı dışa aktar: GeoPackage, GeoJSON, CSV, Shapefile, KML; seçim, kutu, süzgeç, alanlar, geometri (ADR 9106)",
        "note": "ADR 9106. Beklentiler sözleşmenin kurallarıyla burada hesaplanır; GeoPackage'a yazılan tablo haritaya "
                "eklenip cad.layers.rows ile, CSV ve GeoJSON dosyası metin olarak geri okunur. Veri katmanı satirlar.gpkg'ı "
                "okur (scripts/fixtures/layer_rows_command_cases.py yazar).",
        "setup": SETUP,
        "cases": cases(),
    }


def main():
    text = json.dumps(document(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        same = OUT.exists() and OUT.read_text() == text
        print("aynı" if same else f"{OUT} farklı")
        sys.exit(0 if same else 1)
    OUT.write_text(text)
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(cases())} durum)")


if __name__ == "__main__":
    main()
