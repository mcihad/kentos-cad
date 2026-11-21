"""Writes fixtures/commands/v1/cad.layers.migrate.json: the shared cases of `cad.layers.migrate` v1 (docs/adr/9107 §8).

Every expectation is worked out here from the contract's rules (crates/shared/commands/src/gis/cad_layers_migrate.rs):
the rows a drawing layer's objects make (their attributes as text, their geometry as WKT in the project's system),
the table's name made from the layer's name when none is given, every object moved (those the layer filter hides
too), the layer gone afterwards (a second migration of it is refused as not found), the refusals and their order.
No KentOS code is used. The desktop (crates/native/application/tests/all/fixtures) and Python
(python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/layer_migrate_command_cases.py           # writes the file
    python3 scripts/fixtures/layer_migrate_command_cases.py --check   # compares the cases with the ones on disk
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.layers.migrate.json"
STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}
X0, Y0 = 600000.0, 4410000.0

FIELDS = [{"name": "no", "kind": "integer"}, {"name": "durum", "kind": "text"}]
POLES = [(1, "Sağlam"), (2, "Bakımda"), (3, "Arızalı")]
LINES = [[(0, 0), (100, 0)], [(0, 50), (60, 80), (120, 50)]]
AREAS = [[(0, 0), (40, 0), (40, 30), (0, 30)], [(100, 0), (150, 0), (150, 25), (100, 25)]]


def p(x, y):
    return {"x": X0 + x, "y": Y0 + y}


def num(v):
    v = float(v)
    return str(int(v)) if v.is_integer() else repr(v)


def coords(pts):
    return ", ".join(f"{num(X0 + x)} {num(Y0 + y)}" for x, y in pts)


def layer(lid, name, **more):
    node = {"id": lid, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": STYLE, "children": []}
    node.update(more)
    return node


def entities():
    out, eid = [], 1
    for lid in ("direkler", "suzgecli", "gruptaki", "su_vanalari"):
        for i, (no, durum) in enumerate(POLES):
            out.append({"kind": "point", "id": eid, "layerId": lid, "attrs": {"no": str(no), "durum": durum},
                        "p": p(10 * i, 5 * i)})
            eid += 1
    for i, pts in enumerate(LINES):
        out.append({"kind": "polyline", "id": eid, "layerId": "hatlar", "attrs": {"no": str(i + 1), "durum": "Açık"},
                    "pts": [p(x, y) for x, y in pts]})
        eid += 1
    for i, pts in enumerate(AREAS):
        out.append({"kind": "polygon", "id": eid, "layerId": "alanlar", "attrs": {"no": str(i + 1), "durum": "Konut"},
                    "pts": [p(x, y) for x, y in pts]})
        eid += 1
    out.append({"kind": "point", "id": eid, "layerId": "kilitli", "attrs": {}, "p": p(0, 0)})
    eid += 1
    out.append({"kind": "point", "id": eid, "layerId": "yazili", "attrs": {"no": "1"}, "p": p(0, 0)})
    eid += 1
    out.append({"kind": "text", "id": eid, "layerId": "yazili", "attrs": {}, "p": p(5, 5), "text": "Not", "height": 2.5,
                "rotation": 0})
    eid += 1
    out.append({"kind": "text", "id": eid, "layerId": "yazilar", "attrs": {}, "p": p(5, 5), "text": "Yalnız yazı",
                "height": 2.5, "rotation": 0})
    return out


SETUP = {
    "format": "kentos.document",
    "version": 1,
    "name": "Katmanı kaynağa taşı",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                 "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
    "origin": {"x": X0, "y": Y0},
    "layers": [
        layer("0", "0"),
        layer("direkler", "Direkler", geometry="point", fields=FIELDS),
        layer("hatlar", "Hatlar", geometry="line", fields=FIELDS),
        layer("alanlar", "Alanlar", geometry="area", fields=FIELDS),
        layer("suzgecli", "Süzgeçli", geometry="point", fields=FIELDS, filter={"expression": "no > 1"}),
        layer("su_vanalari", "Su Vanaları", geometry="point", fields=FIELDS),
        layer("kilitli", "Kilitli", geometry="point", locked=True),
        layer("yazili", "Yazılı", fields=[{"name": "no", "kind": "integer"}]),
        layer("yazilar", "Yazılar"),
        layer("bos", "Boş", geometry="point"),
        layer("veri", "Parseller", geometry="area",
              data={"driver": "gpkg", "address": "$fixture:data/v1/satirlar.gpkg", "table": "parseller", "key": "fid",
                    "geometry": {"column": "geom", "typeName": "POLYGON", "srid": 5256}},
              fields=[{"name": "ada", "kind": "integer"}]),
        {"id": "grup", "name": "Grup", "type": "group", "visible": True, "locked": False, "expanded": True,
         "style": STYLE, "children": [layer("gruptaki", "Gruptaki", geometry="point", fields=FIELDS)]},
        layer("altlik", "OpenStreetMap", service={"kind": "xyz", "url": "https://tile.openstreetmap.org/{z}/{x}/{y}.png", "maxZoom": 19}),
    ],
    "activeLayer": "0",
    "entities": entities(),
    "styles": {"items": [], "categories": []},
}


def tmp(name):
    return f"$tmp:tasi/{name}"


def completed(output, warnings=None):
    return {"status": "completed", "output": output, "warnings": warnings or []}


def refused(code, message, path):
    error = {"code": code, "message": message}
    if path is not None:
        error["path"] = path
    return {"status": "failed", "error": error}


def migrated(table, address, written, geometry, layer="layer-1"):
    return completed({"layer": layer, "table": table, "address": address, "written": written, "geometry": geometry,
                      "revision": "$current"})


def step(inp, result, op="execute"):
    return {"op": op, "input": inp, "result": result}


def gpkg(layer_id, file, **more):
    inp = {"layer": layer_id, "target": "gpkg", "path": tmp(file)}
    inp.update(more)
    return inp


def rows_step(layer, rows):
    out = {"fields": ["no", "durum"], "rows": rows, "offset": 0, "total": len(rows), "checked": "source"}
    return {"op": "execute", "command": "cad.layers.rows", "input": {"layer": layer, "count": True, "limit": 1000,
                                                                      "geometry": True}, "result": completed(out)}


def pole_rows():
    return [{"key": i + 1, "attrs": {"no": str(no), "durum": durum}, "wkt": f"POINT ({coords([(10 * i, 5 * i)])})"}
            for i, (no, durum) in enumerate(POLES)]


def line_rows():
    return [{"key": i + 1, "attrs": {"no": str(i + 1), "durum": "Açık"}, "wkt": f"LINESTRING ({coords(pts)})"}
            for i, pts in enumerate(LINES)]


def area_rows():
    return [{"key": i + 1, "attrs": {"no": str(i + 1), "durum": "Konut"}, "wkt": f"POLYGON (({coords(pts + [pts[0]])}))"}
            for i, pts in enumerate(AREAS)]


def gone(layer_id, file):
    return step(gpkg(layer_id, file), refused("layer_not_found", f"“{layer_id}” kimlikli katman çizimde yok.", "layer"))


def case(name, steps):
    return {"name": name, "steps": steps}


def cases():
    return [
        case("Nokta katmanı GeoPackage'a taşınır; satırları okunur, katman gider", [
            step(gpkg("direkler", "direk.gpkg"), migrated("direkler", tmp("direk.gpkg"), 3, "POINT")),
            rows_step("layer-1", pole_rows()),
            gone("direkler", "direk2.gpkg")]),
        case("Çizgi katmanı", [step(gpkg("hatlar", "hat.gpkg"), migrated("hatlar", tmp("hat.gpkg"), 2, "LINESTRING")),
                               rows_step("layer-1", line_rows())]),
        case("Alan katmanı", [step(gpkg("alanlar", "alan.gpkg"), migrated("alanlar", tmp("alan.gpkg"), 2, "POLYGON")),
                              rows_step("layer-1", area_rows())]),
        case("Verilen tablo adı", [step(gpkg("direkler", "ad.gpkg", table="direk_2026"),
                                        migrated("direk_2026", tmp("ad.gpkg"), 3, "POINT"))]),
        case("Türkçe adlı katmanın tablo adı", [step(gpkg("su_vanalari", "vana.gpkg"),
                                                     migrated("su_vanalari", tmp("vana.gpkg"), 3, "POINT"))]),
        case("Süzgecin gizlediği nesneler de taşınır", [
            step(gpkg("suzgecli", "suzgec.gpkg"), migrated("suzgecli", tmp("suzgec.gpkg"), 3, "POINT")),
            rows_step("layer-1", pole_rows())]),
        case("Grubun içindeki katman", [step(gpkg("gruptaki", "grup.gpkg"),
                                             migrated("gruptaki", tmp("grup.gpkg"), 3, "POINT"))]),
        case("Yeni bir klasördeki dosya yapılır", [step(gpkg("alanlar", "yeni/klasor/alan.gpkg"),
                                                        migrated("alanlar", tmp("yeni/klasor/alan.gpkg"), 2, "POLYGON"))]),
        case("Aynı dosyaya iki katman", [
            step(gpkg("direkler", "ortak.gpkg"), migrated("direkler", tmp("ortak.gpkg"), 3, "POINT")),
            step(gpkg("hatlar", "ortak.gpkg"), migrated("hatlar", tmp("ortak.gpkg"), 2, "LINESTRING", layer="layer-2"))]),
        case("plan sayar, yazmaz: aynı taşıma sonra yapılır", [
            step(gpkg("direkler", "plan.gpkg"), migrated("direkler", tmp("plan.gpkg"), 3, "POINT", layer=""), op="plan"),
            step(gpkg("direkler", "plan.gpkg"), migrated("direkler", tmp("plan.gpkg"), 3, "POINT"))]),
        case("Ret: katman verilmedi", [step(gpkg("  ", "x.gpkg"), refused("no_layer", "Taşınacak katman verilmedi (layer).",
                                                                          "layer"))]),
        case("Ret: GeoPackage dosyası yok", [step({"layer": "direkler", "target": "gpkg"}, refused(
            "no_target", "Tablonun yapılacağı GeoPackage dosyası verilmedi (path).", "path"))]),
        case("Ret: veritabanı yok", [step({"layer": "direkler", "target": "postgis"}, refused(
            "no_target", "Tablonun yapılacağı veritabanı verilmedi (address).", "address"))]),
        case("Ret: katman yok", [step(gpkg("yok", "x.gpkg"), refused("layer_not_found", "“yok” kimlikli katman çizimde yok.",
                                                                     "layer"))]),
        case("Ret: grup", [step(gpkg("grup", "x.gpkg"), refused(
            "not_a_layer", "“Grup” bir katman grubu; kaynağa taşınan bir katmandır. Grubun bir katmanını verin.", "layer"))]),
        case("Ret: zaten veri katmanı", [step(gpkg("veri", "x.gpkg"), refused(
            "data_layer", "“Parseller” zaten “parseller” tablosunu okuyor; nesneleri kaynağında.", "layer"))]),
        case("Ret: harita servisi katmanı", [step(gpkg("altlik", "x.gpkg"), refused(
            "service_layer", "“OpenStreetMap” bir harita servisinden çizilir; taşınacak nesnesi yoktur.", "layer"))]),
        case("Ret: tablonun adı 63 karakterden uzun", [step(gpkg("direkler", "uzun.gpkg", table="a" * 64), refused(
            "invalid_name", "Tablonun adı boş ya da 63 karakterden uzun olamaz.", "table"))]),
        case("Ret: hedef yapılamıyor (bir dosyanın içinde)", [
            step({"layer": "direkler", "target": "gpkg", "path": "$fixture:geopackage/v1/qgis.gpkg/alt.gpkg"}, refused("source_unreachable", "$any", None))]),
        case("Ret: kilitli katman", [step(gpkg("kilitli", "x.gpkg"), refused(
            "layer_locked", "“Kilitli” kilitli; nesneleri taşınmaz. Kilidi açıp yeniden deneyin.", "layer"))]),
        case("Ret: tablo var", [
            step(gpkg("direkler", "var.gpkg", table="ortak"), migrated("ortak", tmp("var.gpkg"), 3, "POINT")),
            step(gpkg("hatlar", "var.gpkg", table="ortak"), refused(
                "target_exists", "“ortak” tablosu kaynakta var; başka bir tablo adı verin (table).", "table"))]),
        case("Ret: boş katman", [step(gpkg("bos", "bos.gpkg"), refused(
            "nothing_to_export", "Yazılacak nesne kalmadı: seçim, kutu ve süzgeç hiçbir nesne bırakmadı.", "layer"))]),
        case("Ret: yalnız yazı tutan katman", [step(gpkg("yazilar", "yazi.gpkg"), refused(
            "nothing_to_export",
            "Yazılacak nesne kalmadı: 1 nesne yazılamadı (yazı ya da ölçü bir geometri sütununa yazılmaz).", "layer"))]),
        case("Ret: tabloya yazılamayan nesne kaybolurdu", [step(gpkg("yazili", "yazili.gpkg"), refused(
            "unwritable_objects",
            "Katmanın 1 nesnesi bir tabloya yazılamaz (yazı ya da ölçü bir geometri sütununa yazılmaz); katman taşınırsa "
            "kaybolurdu, hiçbir şey yazılmadı. Onları başka bir katmana alıp yeniden deneyin.", "layer"))]),
    ]


def document():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.layers.migrate",
        "commandVersion": 1,
        "title": "Katmanı kaynağa taşı: nokta, çizgi, alan katmanı bir GeoPackage tablosuna; süzgeçli katman; retler (ADR 9107 §8)",
        "note": "ADR 9107 §8. Beklentiler sözleşmenin kurallarıyla burada hesaplanır; taşınan katmanın tablosu "
                "cad.layers.rows ile okunur, katmanın gittiği ikinci bir taşımanın reddiyle görülür.",
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
