"""Writes fixtures/commands/v1/cad.layers.migrate.json: the shared cases of `cad.layers.migrate` v1 (docs/adr/9107 §8,
docs/adr/9114 §2).

Every expectation is worked out here from the contract's rules (crates/shared/commands/src/gis/cad_layers_migrate.rs):
the rows a drawing layer's objects make (their attributes as text, their geometry as WKT in the project's system),
the table's name made from the layer's name when none is given, every object moved (those the layer filter hides
too), the layer gone afterwards (a second migration of it is refused as not found), the refusals and their order.
A layer's texts go into a table of their own beside its table (`<table>_yazilar`; a layer of texts only: its table):
a point and `metin`, `yukseklik` (metres), `aci` (degrees), `hiza` (Yazı's names) and `genislik`, read back as a text
data layer's rows; a text's details a table has no column for (here its mask) are said as `text_details_left`.
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
    eid += 1
    # Turned, aligned and narrowed: every part a text table keeps.
    out.append({"kind": "text", "id": eid, "layerId": "yazilar", "attrs": {}, "p": p(20, 10), "text": "Ada 101",
                "height": 1.8, "rotation": 33.5, "align": "middleCenter", "widthFactor": 0.8})
    eid += 1
    out.append({"kind": "point", "id": eid, "layerId": "olculu", "attrs": {}, "p": p(0, 0)})
    eid += 1
    out.append({"kind": "dimension", "id": eid, "layerId": "olculu", "attrs": {}, "a": p(0, 0), "b": p(4, 0),
                "offset": 0.5, "height": 0.25})
    eid += 1
    out.append({"kind": "point", "id": eid, "layerId": "zeminli", "attrs": {}, "p": p(0, 0)})
    eid += 1
    out.append({"kind": "text", "id": eid, "layerId": "zeminli", "attrs": {}, "p": p(1, 1), "text": "Zeminli",
                "height": 2, "rotation": 0, "mask": True})
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
        layer("olculu", "Ölçülü", geometry="point"),
        layer("zeminli", "Zeminli", geometry="point"),
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


def migrated(table, address, written, geometry, layer="layer-1", texts=None, warnings=None):
    out = {"layer": layer, "table": table, "address": address, "written": written, "geometry": geometry}
    if texts is not None:
        out["texts"] = texts
    out["revision"] = "$current"
    return completed(out, warnings)


# The columns a text table is written with, and a text's row as the table gives it back (docs/adr/9114 §2): its
# numbers as the source reads them, an integer without its point; its alignment as Yazı names it.
TEXT_FIELDS = ["metin", "yukseklik", "aci", "hiza", "genislik"]
ALIGN_WORDS = {None: "sol taban", "middleCenter": "orta"}


def text_row(key, text, at, height, rotation, align=None, width=1.0):
    attrs = {"metin": text, "yukseklik": num(height), "aci": num(rotation), "hiza": ALIGN_WORDS[align],
             "genislik": num(width)}
    return {"key": key, "attrs": attrs, "wkt": f"POINT ({coords([at])})"}


def text_rows_step(layer, rows):
    out = {"fields": TEXT_FIELDS, "rows": rows, "offset": 0, "total": len(rows), "checked": "source"}
    return {"op": "execute", "command": "cad.layers.rows", "input": {"layer": layer, "count": True, "limit": 1000,
                                                                      "geometry": True}, "result": completed(out)}


def step(inp, result, op="execute"):
    return {"op": op, "input": inp, "result": result}


def gpkg(layer_id, file, **more):
    inp = {"layer": layer_id, "target": "gpkg", "path": tmp(file)}
    inp.update(more)
    return inp


def rows_step(layer, rows, fields=("no", "durum")):
    out = {"fields": list(fields), "rows": rows, "offset": 0, "total": len(rows), "checked": "source"}
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
        # docs/adr/9114 §2: a layer's texts in a table of their own.
        case("Yalnız yazı tutan katman yazı tablosu olur, yerinde ve adıyla", [
            step(gpkg("yazilar", "yazi.gpkg"), migrated("yazilar", tmp("yazi.gpkg"), 2, "POINT")),
            text_rows_step("layer-1", [text_row(1, "Yalnız yazı", (5, 5), 2.5, 0),
                                       text_row(2, "Ada 101", (20, 10), 1.8, 33.5, "middleCenter", 0.8)]),
            gone("yazilar", "yazi2.gpkg")]),
        # A layer without a declared geometry writes a column of any geometry, as the export does.
        case("Nokta ve yazı: yazılar yanındaki tabloya, katmanı onun üstünde", [
            step(gpkg("yazili", "yazili.gpkg"), migrated(
                "yazili", tmp("yazili.gpkg"), 1, "GEOMETRY",
                texts={"layer": "layer-2", "table": "yazili_yazilar", "written": 1})),
            rows_step("layer-1", [{"key": 1, "attrs": {"no": "1"}, "wkt": f"POINT ({coords([(0, 0)])})"}], ("no",)),
            text_rows_step("layer-2", [text_row(1, "Not", (5, 5), 2.5, 0)])]),
        case("plan yazıları da sayar", [
            step(gpkg("yazili", "yazili-plan.gpkg"), migrated(
                "yazili", tmp("yazili-plan.gpkg"), 1, "GEOMETRY", layer="",
                texts={"layer": "", "table": "yazili_yazilar", "written": 1}), op="plan")]),
        case("Yazının tablonun tutmadığı ayrıntısı söylenir (zemin)", [
            step(gpkg("zeminli", "zeminli.gpkg"), migrated(
                "zeminli", tmp("zeminli.gpkg"), 1, "POINT",
                texts={"layer": "layer-2", "table": "zeminli_yazilar", "written": 1},
                warnings=[{"code": "text_details_left",
                           "message": "1 yazının stili, zemini, eğrisi, ölçekleri, nesne bağı ya da paragraf biçimi yazı "
                                      "tablosuna yazılmadı; metni, noktası, yüksekliği, açısı, hizası ve genişlik çarpanı "
                                      "yazıldı.",
                           "path": "layer"}]))]),
        case("Ret: yazıların tablosu var", [
            step(gpkg("yazilar", "yazivar.gpkg", table="yazili_yazilar"),
                 migrated("yazili_yazilar", tmp("yazivar.gpkg"), 2, "POINT")),
            step(gpkg("yazili", "yazivar.gpkg"), refused(
                "target_exists", "“yazili_yazilar” tablosu kaynakta var; başka bir tablo adı verin (table).", "table"))]),
        case("Ret: tabloya yazılamayan nesne kaybolurdu (ölçü)", [step(gpkg("olculu", "olculu.gpkg"), refused(
            "unwritable_objects",
            "Katmanın 1 nesnesi bir tabloya yazılamaz (ölçü bir geometri sütununa yazılmaz); katman taşınırsa "
            "kaybolurdu, hiçbir şey yazılmadı. Onları başka bir katmana alıp yeniden deneyin.", "layer"))]),
    ]


def document():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.layers.migrate",
        "commandVersion": 1,
        "title": "Katmanı kaynağa taşı: nokta, çizgi, alan katmanı bir GeoPackage tablosuna; süzgeçli katman; yazılar "
                 "kendi tablolarına; retler (ADR 9107 §8, ADR 9114 §2)",
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
