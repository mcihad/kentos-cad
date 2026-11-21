"""Writes fixtures/commands/v1/cad.layers.create.json: the shared cases of `cad.layers.create` v1 (docs/adr/9107 §7).

Every expectation is worked out here from the contract's rules (crates/shared/commands/src/gis/cad_layers_create.rs):
the table's name made from the layer's name when none is given, the OGC geometry type of each kind (multi-part, Z),
the project's system or the one given, the key and geometry columns' names, the refusals and their order. The table
made is read back through `cad.layers.rows` (its columns, no row); a second make of the same table is refused. No
KentOS code is used. The desktop (crates/native/application/tests/all/fixtures) and Python
(python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/layer_create_command_cases.py           # writes the file
    python3 scripts/fixtures/layer_create_command_cases.py --check   # compares the cases with the ones on disk
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.layers.create.json"
STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}

SETUP = {
    "format": "kentos.document",
    "version": 1,
    "name": "Yeni veri katmanı",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                 "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
    "origin": {"x": 600000, "y": 4410000},
    "layers": [
        {"id": "0", "name": "0", "type": "layer", "visible": True, "locked": False, "expanded": True, "style": STYLE,
         "children": []},
        {"id": "grup", "name": "Altyapı", "type": "group", "visible": True, "locked": False, "expanded": True,
         "style": STYLE, "children": []},
    ],
    "activeLayer": "0",
    "entities": [],
    "styles": {"items": [], "categories": []},
}

FIELDS = [{"name": "no", "kind": "integer"}, {"name": "durum", "kind": "text", "length": 20}]


def table_name(name):
    """The layer's name made a table name, as the contract says: Turkish letters as their bare ones, other letters
    lower case, anything else `_` (each one), `_` taken off both ends, `k_` before a name that is empty or starts
    with a digit."""
    bare = {"ç": "c", "Ç": "c", "ğ": "g", "Ğ": "g", "ı": "i", "İ": "i", "I": "i", "ö": "o", "Ö": "o", "ş": "s",
            "Ş": "s", "ü": "u", "Ü": "u"}
    out = []
    for ch in name.strip():
        c = bare.get(ch, ch.lower() if ch.isascii() else ch)
        out.append(c if c.isascii() and c.isalnum() else "_")
    text = "".join(out).strip("_")
    return f"k_{text}" if not text or text[0].isdigit() else text


def tmp(name):
    return f"$tmp:yeni/{name}"


def completed(output, warnings=None):
    return {"status": "completed", "output": output, "warnings": warnings or []}


def refused(code, message, path):
    error = {"code": code, "message": message}
    if path is not None:
        error["path"] = path
    return {"status": "failed", "error": error}


def created(table, address, geometry, srid, layer="layer-1"):
    out = {"layer": layer, "table": table, "address": address}
    if geometry is not None:
        out["geometry"] = geometry
    if srid is not None:
        out["srid"] = srid
    out["revision"] = "$current"
    return completed(out)


def make(inp, result, op="execute"):
    return {"op": op, "input": inp, "result": result}


def rows_step(layer, fields):
    out = {"fields": fields, "rows": [], "offset": 0, "total": 0, "checked": "source"}
    return {"op": "execute", "command": "cad.layers.rows", "input": {"layer": layer, "count": True, "limit": 1000},
            "result": completed(out)}


def case(name, steps):
    return {"name": name, "steps": steps}


def gpkg(name, file, geometry="point", **more):
    inp = {"name": name, "target": "gpkg", "path": tmp(file), "geometry": geometry}
    inp.update(more)
    return inp


def cases():
    names = [f["name"] for f in FIELDS]
    out = [
        case("Nokta katmanı yeni bir GeoPackage'ta; tablo boş okunur", [
            make(gpkg("Direkler", "direk.gpkg", fields=FIELDS),
                 created("direkler", tmp("direk.gpkg"), "POINT", 5256)),
            rows_step("layer-1", names)]),
        case("Çizgi katmanı", [make(gpkg("Yollar", "yol.gpkg", "line"), created("yollar", tmp("yol.gpkg"), "LINESTRING", 5256))]),
        case("Alan katmanı", [make(gpkg("Parseller", "parsel.gpkg", "area", fields=FIELDS),
                                   created("parseller", tmp("parsel.gpkg"), "POLYGON", 5256)),
                              rows_step("layer-1", names)]),
        case("Çok parçalı alan", [make(gpkg("Adalar", "ada.gpkg", "area", multi=True),
                                       created("adalar", tmp("ada.gpkg"), "MULTIPOLYGON", 5256))]),
        case("Çok parçalı nokta", [make(gpkg("Ağaçlar", "agac.gpkg", "point", multi=True),
                                        created("agaclar", tmp("agac.gpkg"), "MULTIPOINT", 5256))]),
        case("Çok parçalı çizgi", [make(gpkg("Hatlar", "hat.gpkg", "line", multi=True),
                                        created("hatlar", tmp("hat.gpkg"), "MULTILINESTRING", 5256))]),
        case("Z ile nokta", [make(gpkg("Kotlu noktalar", "kot.gpkg", "point", z=True),
                                  created("kotlu_noktalar", tmp("kot.gpkg"), "POINT Z", 5256))]),
        case("Geometrisiz kayıt tablosu: sistem yok", [
            make(gpkg("Abone listesi", "abone.gpkg", "none", fields=FIELDS),
                 created("abone_listesi", tmp("abone.gpkg"), None, None)),
            rows_step("layer-1", names)]),
        case("Tablonun adı katmanın Türkçe adından", [
            make(gpkg("Su Hatları (İçme)", "su.gpkg", "line"),
                 created(table_name("Su Hatları (İçme)"), tmp("su.gpkg"), "LINESTRING", 5256))]),
        case("Verilen tablo adı", [make(gpkg("İşletme hattı", "isletme.gpkg", "line", table="isletme_hatti_2026"),
                                        created("isletme_hatti_2026", tmp("isletme.gpkg"), "LINESTRING", 5256))]),
        case("Anahtar ve geometri sütununun adı verilir", [
            make(gpkg("Vanalar", "vana.gpkg", "point", key="kimlik", geometryColumn="sekil",
                      fields=[{"name": "cap", "kind": "integer"}]),
                 created("vanalar", tmp("vana.gpkg"), "POINT", 5256)),
            rows_step("layer-1", ["cap"])]),
        case("Başka sistemde tablo (WGS 84)", [make(gpkg("GPS izi", "gps.gpkg", "line", srid=4326),
                                                    created("gps_izi", tmp("gps.gpkg"), "LINESTRING", 4326))]),
        case("Aynı dosyada iki tablo", [
            make(gpkg("Rögarlar", "kanal.gpkg", "point"), created("rogarlar", tmp("kanal.gpkg"), "POINT", 5256)),
            make(gpkg("Kanal hatları", "kanal.gpkg", "line"),
                 created("kanal_hatlari", tmp("kanal.gpkg"), "LINESTRING", 5256, layer="layer-2"))]),
        case("Grubun içine, ilk sıraya", [make(gpkg("Trafolar", "trafo.gpkg", "point", parent="grup", index=0),
                                               created("trafolar", tmp("trafo.gpkg"), "POINT", 5256))]),
        case("plan hiçbir şey yazmaz: aynı tablo sonra yapılır", [
            make(gpkg("Duraklar", "durak.gpkg", "point"), created("duraklar", tmp("durak.gpkg"), "POINT", 5256, layer=""),
                 op="plan"),
            make(gpkg("Duraklar", "durak.gpkg", "point"), created("duraklar", tmp("durak.gpkg"), "POINT", 5256))]),
        case("Ret: ad boş", [make(gpkg("  ", "bos.gpkg"), refused("empty_name", "Katmanın adı boş olamaz.", "name"))]),
        case("Ret: GeoPackage dosyası yok", [make({"name": "A", "target": "gpkg", "geometry": "point"}, refused(
            "no_target", "Tablonun yapılacağı GeoPackage dosyası verilmedi (path).", "path"))]),
        case("Ret: veritabanı yok", [make({"name": "A", "target": "postgis", "geometry": "point"}, refused(
            "no_target", "Tablonun yapılacağı veritabanı verilmedi (address).", "address"))]),
        case("Ret: kaynağın kendi tablosunun adı", [make(gpkg("A", "ayrik.gpkg", table="gpkg_katman"), refused(
            "invalid_name", "Tablonun adı alınamaz: “gpkg_katman” kaynağın kendi tablolarına ayrılmış bir ad.", "table"))]),
        case("Ret: tablo adı 63 bayttan uzun", [make(gpkg("A", "uzun.gpkg", table="a" * 64), refused(
            "invalid_name", f"Tablonun adı alınamaz: “{'a' * 64}” en çok 63 bayt olabilir.", "table"))]),
        case("Ret: alanın adı anahtar sütununun", [make(gpkg("A", "fid.gpkg", fields=[{"name": "fid", "kind": "integer"}]),
                                                        refused("invalid_name",
                                                                "“fid” anahtar ya da geometri sütununun adı; alana başka bir ad verin.",
                                                                "fields.0.name"))]),
        case("Ret: alanlar kurala uymuyor (aynı ad iki kez)", [
            make(gpkg("A", "iki.gpkg", fields=[{"name": "ad", "kind": "text"}, {"name": "ad", "kind": "text"}]),
                 refused("invalid_fields", "$any", "fields"))]),
        case("Ret: bilinmeyen sistem", [make(gpkg("A", "sistem.gpkg", srid=999999), refused("unknown_srid", "$any", "srid"))]),
        case("Ret: kaynak yapılamıyor (bir dosyanın içinde)", [
            make({"name": "A", "target": "gpkg", "geometry": "point", "path": "$fixture:geopackage/v1/qgis.gpkg/alt.gpkg"},
                 refused("source_unreachable", "$any", None))]),
        case("Ret: tablo var", [
            make(gpkg("Direkler", "var.gpkg"), created("direkler", tmp("var.gpkg"), "POINT", 5256)),
            make(gpkg("Direkler", "var.gpkg"), refused(
                "target_exists",
                "“direkler” tablosu kaynakta var; başka bir ad verin ya da onu Kaynaklar'dan veri katmanı olarak ekleyin.",
                "table"))]),
    ]
    return out


def document():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.layers.create",
        "commandVersion": 1,
        "title": "Yeni veri katmanı: GeoPackage tablosu, nokta, çizgi, alan, çok parçalı, Z, geometrisiz; retler (ADR 9107 §7)",
        "note": "ADR 9107 §7. Beklentiler sözleşmenin kurallarıyla burada hesaplanır; yapılan tablo cad.layers.rows ile "
                "boş okunur, aynı tablonun ikinci kez yapılması reddedilir.",
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
