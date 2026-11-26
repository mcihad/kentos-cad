"""Writes fixtures/data/v1/sutun.gpkg and fixtures/commands/v1/cad.layers.columns.json: the shared cases of
`cad.layers.columns` v1 (docs/adr/9318). The GeoPackage is written here with sqlite3 (OGC 12-128r19: its tables and the
`GP` blobs with little-endian WKB), and every expectation is worked out here from the contract's rules
(crates/shared/commands/src/gis/cad_layers_columns.rs): the refusals and their order, the columns a table gets and the
rows empty in them (read back with `cad.layers.rows`), the layer's fields after, an undo that gives the layer its fields
back while the column stays in the table, and a refresh that takes the layer's fields to the table's columns. No KentOS
code is used. The desktop (crates/native/application/tests/all/fixtures) and the Python runner run them.

Every case that writes works on its own copy of the GeoPackage (`$tmp:sutun/N.gpkg`) and adds its data layer itself
(`cad.layers.data`), so no case sees another's columns.

    python3 scripts/fixtures/layer_columns_command_cases.py           # writes both files
    python3 scripts/fixtures/layer_columns_command_cases.py --check   # compares the cases with the ones on disk
"""
import json
import sqlite3
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GPKG = ROOT / "fixtures/data/v1/sutun.gpkg"
OUT = ROOT / "fixtures/commands/v1/cad.layers.columns.json"
SOURCE = "$fixture:data/v1/sutun.gpkg"
COMMAND = "cad.layers.columns"
TABLE = "saha_varliklari"
NAME = "Saha varlıkları"

# The table's rows: key, point, durum, no, eski_not (a column the layer does not take at first).
ROWS = [
    {"fid": 1, "x": 600010.0, "y": 4410010.0, "durum": "A", "no": 1, "eski_not": None},
    {"fid": 2, "x": 600020.0, "y": 4410020.0, "durum": "B", "no": 2, "eski_not": "kontrol"},
    {"fid": 3, "x": 600030.0, "y": 4410030.0, "durum": None, "no": 3, "eski_not": None},
]
TABLE_COLUMNS = ["durum", "no", "eski_not"]

# The layer's fields: two of the table's columns; all three; and two with one the table does not have.
LAYER_FIELDS = [{"name": "durum", "kind": "text"}, {"name": "no", "kind": "integer"}]
FULL_FIELDS = LAYER_FIELDS + [{"name": "eski_not", "kind": "text"}]
STALE_FIELDS = LAYER_FIELDS + [{"name": "kayip", "kind": "text"}]
NOTE = {"name": "saha_notu", "kind": "text"}

FOLD = {"I": "ı", "İ": "i"}


def fold(text):
    """The Turkish folding of names: I is ı, İ is i, every other letter its lower case."""
    return "".join(FOLD.get(c) or (c.lower() if len(c.lower()) == 1 else c) for c in text)


def gp_point(x, y):
    """A point as a GeoPackage blob: `GP`, version 0, flags (little-endian, no envelope), SRS 5256, WKB point."""
    return b"GP" + bytes([0, 0b00000001]) + struct.pack("<i", 5256) + struct.pack("<BIdd", 1, 1, x, y)


def write_gpkg():
    GPKG.parent.mkdir(parents=True, exist_ok=True)
    if GPKG.exists():
        GPKG.unlink()
    db = sqlite3.connect(GPKG)
    db.executescript(
        """
        PRAGMA application_id = 1196444487;
        PRAGMA user_version = 10300;
        CREATE TABLE gpkg_spatial_ref_sys (srs_name TEXT NOT NULL, srs_id INTEGER PRIMARY KEY, organization TEXT NOT NULL,
            organization_coordsys_id INTEGER NOT NULL, definition TEXT NOT NULL, description TEXT);
        INSERT INTO gpkg_spatial_ref_sys VALUES ('Undefined cartesian SRS', -1, 'NONE', -1, 'undefined', NULL),
            ('Undefined geographic SRS', 0, 'NONE', 0, 'undefined', NULL),
            ('WGS 84 geodetic', 4326, 'EPSG', 4326, 'GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]]', NULL),
            ('TUREF / TM36', 5256, 'EPSG', 5256, 'PROJCS["TUREF / TM36",GEOGCS["TUREF",DATUM["Turkish_National_Reference_Frame",SPHEROID["GRS 1980",6378137,298.257222101]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",36],PARAMETER["scale_factor",1],PARAMETER["false_easting",500000],PARAMETER["false_northing",0],UNIT["metre",1]]', NULL);
        CREATE TABLE gpkg_contents (table_name TEXT NOT NULL PRIMARY KEY, data_type TEXT NOT NULL, identifier TEXT UNIQUE,
            description TEXT DEFAULT '', last_change DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            min_x DOUBLE, min_y DOUBLE, max_x DOUBLE, max_y DOUBLE, srs_id INTEGER);
        CREATE TABLE gpkg_geometry_columns (table_name TEXT NOT NULL, column_name TEXT NOT NULL,
            geometry_type_name TEXT NOT NULL, srs_id INTEGER NOT NULL, z TINYINT NOT NULL, m TINYINT NOT NULL,
            CONSTRAINT pk_geom_cols PRIMARY KEY (table_name, column_name));
        CREATE TABLE saha_varliklari (fid INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, geom POINT, durum TEXT,
            no INTEGER, eski_not TEXT);
        """
    )
    for r in ROWS:
        db.execute("INSERT INTO saha_varliklari (fid, geom, durum, no, eski_not) VALUES (?, ?, ?, ?, ?)",
                   (r["fid"], gp_point(r["x"], r["y"]), r["durum"], r["no"], r["eski_not"]))
    xs, ys = [r["x"] for r in ROWS], [r["y"] for r in ROWS]
    db.execute("INSERT INTO gpkg_contents (table_name, data_type, identifier, last_change, min_x, min_y, max_x, max_y, srs_id) "
               "VALUES (?, 'features', ?, '2026-11-26T00:00:00.000Z', ?, ?, ?, ?, 5256)",
               (TABLE, TABLE, min(xs), min(ys), max(xs), max(ys)))
    db.execute("INSERT INTO gpkg_geometry_columns VALUES (?, 'geom', 'POINT', 5256, 0, 0)", (TABLE,))
    db.commit()
    db.close()


def copy(n):
    return {"op": "copyFile", "from": SOURCE, "to": f"$tmp:sutun/{n}.gpkg"}


def data_of(n, table=TABLE, read_only=False):
    data = {"driver": "gpkg", "address": f"$tmp:sutun/{n}.gpkg", "table": table, "key": "fid",
            "geometry": {"column": "geom", "typeName": "POINT", "srid": 5256}}
    if read_only:
        data["readOnly"] = True
    return data


def add_layer(n, fields=LAYER_FIELDS, name=NAME, table=TABLE, read_only=False):
    return {"op": "execute", "command": "cad.layers.data",
            "input": {"operation": "add", "name": name, "data": data_of(n, table, read_only), "fields": fields},
            "result": completed({"layer": "layer-1", "revision": "$current"})}


def completed(output, warnings=None):
    return {"status": "completed", "output": output, "warnings": warnings or []}


def failed(code, message, path=None):
    error = {"code": code, "message": message}
    if path is not None:
        error["path"] = path
    return {"status": "failed", "error": error}


def columns(inp, result, expect=None, op="execute"):
    step = {"op": op, "input": inp, "result": result}
    if expect is not None:
        step["expect"] = expect
    return step


def done(added, fields, removed=(), rows=None, table=TABLE):
    out = {"layer": "layer-1", "table": table, "added": list(added), "removed": list(removed), "fields": fields}
    if rows is not None:
        out["rows"] = rows
    out["revision"] = "$current"
    return out


def add(fields, layer="layer-1"):
    return {"action": "add", "layer": layer, "fields": fields}


def refresh(layer="layer-1"):
    return {"action": "refresh", "layer": layer}


def rows_read(fields):
    """`cad.layers.rows` of these fields: what the table holds in them, a row's empty value left out."""
    rows = []
    for r in ROWS:
        attrs = {}
        for f in fields:
            v = r.get(f)
            if v is not None:
                attrs[f] = v if isinstance(v, str) else str(v)
        rows.append({"key": r["fid"], "attrs": attrs})
    return {"op": "execute", "command": "cad.layers.rows", "input": {"layer": "layer-1", "fields": fields},
            "result": completed({"fields": fields, "rows": rows, "offset": 0, "checked": "source"})}


def refused(name, inp, code, message, path, before=()):
    return {"name": name, "steps": list(before) + [columns(inp, failed(code, message, path), {"revision": "same"})]}


def taken_message(name):
    return f"“{name}” katmanda ya da tablonun anahtarında, geometrisinde var; başka bir ad verin."


def cases():
    out = []
    note = [NOTE]
    out.append(refused("katman verilmedi: no_layer", add(note, layer="  "), "no_layer",
                       "Sütunların ekleneceği ya da şemasının yenileneceği veri katmanının kimliğini (layer) verin.", "layer"))
    out.append(refused("sütun verilmedi: no_columns", add([]), "no_columns",
                       "Tabloya eklenecek sütunları (fields) verin.", "fields"))
    out.append(refused("fields hiç yok: no_columns", {"action": "add", "layer": "layer-1"}, "no_columns",
                       "Tabloya eklenecek sütunları (fields) verin.", "fields"))
    out.append(refused("boş ad: invalid_name", add([{"name": "", "kind": "text"}]), "invalid_name",
                       "Sütunun adı alınamaz: ad boş olamaz.", "fields.0.name"))
    long_name = "a" * 64
    out.append(refused("64 baytlık ad: invalid_name", add([{"name": long_name, "kind": "text"}]), "invalid_name",
                       f"Sütunun adı alınamaz: “{long_name}” en çok 63 bayt olabilir.", "fields.0.name"))
    out.append(refused("kaynağın kendine ayırdığı ad: invalid_name", add([{"name": "rtree_dizin", "kind": "text"}]),
                       "invalid_name", "Sütunun adı alınamaz: “rtree_dizin” kaynağın kendi tablolarına ayrılmış bir ad.",
                       "fields.0.name"))
    assert fold("NOT") == fold("not")
    out.append(refused("iki kez verilen ad: column_exists",
                       add([{"name": "not", "kind": "text"}, {"name": "NOT", "kind": "text"}]), "column_exists",
                       "“NOT” iki kez verildi; her sütuna ayrı bir ad verin.", "fields.1.name"))
    out.append(refused("geçersiz beklenen sürüm: invalid_revision",
                       {**add(note), "expectedRevision": "on iki"}, "invalid_revision",
                       "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.",
                       "expectedRevision"))
    out.append({
        "name": "sürüm çakışması: katman eklendikten sonra eski sürümle hiçbir şey yazılmaz",
        "steps": [
            {"op": "captureRevision", "as": "once"},
            copy(9),
            add_layer(9),
            columns({**add(note), "expectedRevision": "$once"},
                    {"status": "conflict", "error": {"code": "revision_conflict",
                                                     "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
                                                     "path": "expectedRevision", "revision": "$current"}},
                    {"revision": "same"}),
        ],
    })
    out.append(refused("çizimde olmayan katman: layer_not_found", add(note, layer="yok"), "layer_not_found",
                       "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layer"))
    out.append(refused("grup: not_a_layer", add(note, layer="grup"), "not_a_layer",
                       "“Grup” bir katman grubu; sütunlar bir veri katmanının tablosunun olur.", "layer"))
    out.append(refused("çizim katmanı: not_a_data_layer", add(note, layer="cizim"), "not_a_data_layer",
                       "“Çizim” bir veri katmanı değil; nesneleri çizimdedir, sütunu yoktur. Alanlar penceresinde alan ekleyin.",
                       "layer"))
    out.append(refused("çizim katmanının şeması yenilenmez: not_a_data_layer", refresh(layer="cizim"), "not_a_data_layer",
                       "“Çizim” bir veri katmanı değil; nesneleri çizimdedir, sütunu yoktur. Alanlar penceresinde alan ekleyin.",
                       "layer"))
    out.append(refused("salt okunur katman: data_layer_read_only", add(note), "data_layer_read_only",
                       f"“{NAME}” salt okunur tutuluyor; tablosuna sütun eklenmez.", "layer",
                       before=[copy(13), add_layer(13, read_only=True)]))
    assert fold("Durum") == fold("durum")
    out.append(refused("katmanın alanı: column_exists", add([{"name": "Durum", "kind": "text"}]), "column_exists",
                       taken_message("Durum"), "fields.0.name", before=[copy(14), add_layer(14)]))
    assert fold("Fid") == fold("fid") and fold("FID") != fold("fid")
    out.append(refused("tablonun anahtarı: column_exists", add([{"name": "Fid", "kind": "integer"}]), "column_exists",
                       taken_message("Fid"), "fields.0.name", before=[copy(15), add_layer(15)]))
    out.append(refused("tablonun geometrisi: column_exists", add([{"name": "geom", "kind": "text"}]), "column_exists",
                       taken_message("geom"), "fields.0.name", before=[copy(16), add_layer(16)]))
    out.append(refused("tabloda olup katmanda olmayan sütun: column_exists", add([{"name": "eski_not", "kind": "text"}]),
                       "column_exists",
                       f"“{NAME}” katmanının tablosunda “eski_not” sütunu var ama katmanın alanı değil: Kaynak şemasını yenile ile katmana alın.",
                       "fields.0.name", before=[copy(17), add_layer(17)]))
    out.append(refused("alanın kuralı: invalid_fields", add([{"name": "not ", "kind": "text"}]), "invalid_fields",
                       "“not ” alanının adının başında ya da sonunda boşluk var.", "fields",
                       before=[copy(18), add_layer(18)]))
    query = "-- kentos: key=fid geometry=geom\nSELECT fid, geom, durum FROM saha_varliklari"
    out.append(refused("sorgu katmanı: columns_not_permitted", add(note), "columns_not_permitted",
                       "“Saha sorgusu” katmanının tablosuna sütun eklenemez: PostgreSQL'de tablonun sahibi olmak gerekir; salt okunur dosyaya ve sorguya sütun eklenmez. Tablo değişmedi.",
                       "layer",
                       before=[copy(19), add_layer(19, fields=[{"name": "durum", "kind": "text"}], name="Saha sorgusu",
                                                    table=query)]))
    after = LAYER_FIELDS + [NOTE]
    out.append({
        "name": "plan: tablo okunur, sütun eklenmez, satır sayısı söylenir",
        "steps": [copy(20), add_layer(20),
                  columns(add(note), completed(done(["saha_notu"], after, rows=len(ROWS))), {"revision": "same"}, op="plan"),
                  # The plan added nothing: the table still lacks the column.
                  columns(refresh(), completed(done(["eski_not"], FULL_FIELDS)), {"revision": "same"}, op="plan")],
    })
    out.append({
        "name": "bir sütun: tabloya eklenir, var olan satırlarda boş, katmanın alanı olur",
        "steps": [copy(21), add_layer(21),
                  columns(add(note), completed(done(["saha_notu"], after, rows=len(ROWS))), {"revision": "changed"}),
                  rows_read(["durum", "saha_notu"])],
    })
    two = [{"name": "sayac", "kind": "integer"}, {"name": "tarih", "kind": "date"}]
    out.append({
        "name": "iki sütun, türleriyle: ikisi de her satırda boş",
        "steps": [copy(22), add_layer(22),
                  columns(add(two), completed(done(["sayac", "tarih"], LAYER_FIELDS + two, rows=len(ROWS))),
                          {"revision": "changed"}),
                  rows_read(["sayac", "tarih", "no"])],
    })
    out.append({
        "name": "geri alma katmanın alanlarını geri getirir, sütun tabloda kalır: yenile onu yeniden alır",
        "steps": [copy(23), add_layer(23),
                  columns(add(note), completed(done(["saha_notu"], after, rows=len(ROWS))), {"revision": "changed"}),
                  {"op": "undo", "returns": "Kaynağa sütun ekle"},
                  columns(refresh(), completed(done(["eski_not", "saha_notu"], FULL_FIELDS + [NOTE])),
                          {"revision": "changed"})],
    })
    out.append({
        "name": "yenile: başka yerde eklenen sütun alan olur, tabloda olmayan alan çıkar",
        "steps": [copy(24), add_layer(24, fields=STALE_FIELDS),
                  columns(refresh(), completed(done(["eski_not"], FULL_FIELDS, removed=["kayip"])), {"revision": "changed"}),
                  {"op": "undo", "returns": "Kaynak şemasını yenile"}],
    })
    out.append({
        "name": "yenile planı hiçbir şey yazmaz",
        "steps": [copy(25), add_layer(25, fields=STALE_FIELDS),
                  columns(refresh(), completed(done(["eski_not"], FULL_FIELDS, removed=["kayip"])), {"revision": "same"},
                          op="plan")],
    })
    out.append({
        "name": "yenile: değişen yoksa geri alma adımı da yok",
        "steps": [copy(26), add_layer(26, fields=FULL_FIELDS),
                  columns(refresh(), completed(done([], FULL_FIELDS)), {"revision": "same"})],
    })
    ruled = [{"name": "durum", "kind": "text", "alias": "Durumu"}, {"name": "no", "kind": "integer"},
             {"name": "kayip", "kind": "text"}]
    out.append({
        "name": "yenile kalan alanın kurallarını korur",
        "steps": [copy(27), add_layer(27, fields=ruled),
                  columns(refresh(), completed(done(["eski_not"], ruled[:2] + [{"name": "eski_not", "kind": "text"}],
                                                    removed=["kayip"])), {"revision": "changed"})],
    })
    out.append({
        "name": "doğrulama: geçerli girdi bir şey yazmaz",
        "steps": [copy(28), add_layer(28),
                  columns(add(note), {"status": "completed", "output": None, "warnings": []}, {"revision": "same"},
                          op="validate")],
    })
    out.append({
        "name": "eklenen sütuna sonra eklenen sütun: ikisi sırasıyla tabloda",
        "steps": [copy(29), add_layer(29),
                  columns(add(note), completed(done(["saha_notu"], after, rows=len(ROWS))), {"revision": "changed"}),
                  columns(add([{"name": "puan", "kind": "decimal"}]),
                          completed(done(["puan"], after + [{"name": "puan", "kind": "decimal"}], rows=len(ROWS))),
                          {"revision": "changed"}),
                  columns(refresh(), completed(done(["eski_not"], after + [{"name": "puan", "kind": "decimal"},
                                                                          {"name": "eski_not", "kind": "text"}])),
                          {"revision": "changed"})],
    })
    return out


def setup():
    return {
        "format": "kentos.document", "version": 1, "name": "Veri katmanının sütunları",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": 600000, "y": 4410000},
        "layers": [
            {"id": "cizim", "name": "Çizim", "type": "layer", "visible": True, "locked": False, "expanded": True,
             "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []},
            {"id": "grup", "name": "Grup", "type": "group", "visible": True, "locked": False, "expanded": True,
             "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []},
        ],
        "activeLayer": "cizim", "entities": [], "styles": {"items": [], "categories": []},
    }


def document():
    return {
        "format": "kentos.command-cases", "version": 1, "command": COMMAND, "commandVersion": 1,
        "title": "Veri katmanının sütunları",
        "note": "ADR 9318. GeoPackage'ı bu betik sqlite3 ile yazar (fixtures/data/v1/sutun.gpkg); yazan her durum kendi "
                "kopyasında çalışır ve veri katmanını kendisi ekler. Beklentiler sözleşmenin kurallarıyla burada "
                "hesaplanır: retler ve sırası, tabloya eklenen sütunlar ve satırlarda boş okunmaları (cad.layers.rows), "
                "katmanın sonraki alanları, alanları geri getiren ama sütunu tabloda bırakan geri alma, alanları tablonun "
                "sütunlarına getiren yenileme.",
        "setup": setup(),
        "cases": cases(),
    }


def main():
    text = json.dumps(document(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            print("cad.layers.columns.json differs from what the script writes", file=sys.stderr)
            sys.exit(1)
        print(f"{len(document()['cases'])} cases as on disk")
        return
    write_gpkg()
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(document()['cases'])} cases; {GPKG.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
