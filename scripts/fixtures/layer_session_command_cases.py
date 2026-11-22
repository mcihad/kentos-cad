"""Writes fixtures/data/v1/oturum.gpkg and fixtures/commands/v1/cad.layers.session.json: the shared cases of
`cad.layers.session` v1 (docs/adr/9103 §3). The GeoPackage is written here with sqlite3 (OGC 12-128r19: its tables,
the `GP` blobs with little-endian WKB, the R*Tree index and its triggers of §F.3, edition 1.3), and every expectation
is worked out here from the contract's rules (crates/shared/commands/src/gis/cad_layers_session.rs): the refusals and
their order, what a layer would write, what a save writes to the table and takes out of the drawing, letting go,
closing, a row taken as an object of the layer, a table that refuses (a NOT NULL field, a CHECK) and the drawing kept
then. No KentOS code is used. The desktop (crates/native/application/tests/all/fixtures) runs them.

Every case writes to its own copy of the GeoPackage (`$tmp:oturum/N.gpkg`) and adds its data layer itself
(`cad.layers.data`), so no case sees another's writes.

    python3 scripts/fixtures/layer_session_command_cases.py           # writes both files
    python3 scripts/fixtures/layer_session_command_cases.py --check   # compares the cases with the ones on disk
"""
import json
import sqlite3
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GPKG = ROOT / "fixtures/data/v1/oturum.gpkg"
OUT = ROOT / "fixtures/commands/v1/cad.layers.session.json"
SOURCE = "$fixture:data/v1/oturum.gpkg"
COMMAND = "cad.layers.session"

MAHALLE = ["Çınar", "İnönü", "Işıklar", "Şehitler"]
KULLANIM = ["Konut", "Park", "Ticaret"]
X0, Y0, CELL = 600000.0, 4410000.0, 40.0
FIELDS = ["ada", "parsel", "mahalle", "kullanim", "alan"]
LAYER_FIELDS = [
    {"name": "ada", "kind": "integer"},
    {"name": "parsel", "kind": "integer"},
    {"name": "mahalle", "kind": "text"},
    {"name": "kullanim", "kind": "text"},
    {"name": "alan", "kind": "decimal"},
]


def rows():
    """The table's 10 rows: key, ada, parsel, mahalle, kullanim (some NULL), alan, the square's corner and side."""
    out = []
    for i in range(10):
        side = 20.0 + i
        out.append({
            "fid": i + 1,
            "ada": 200 + i // 5,
            "parsel": i % 5 + 1,
            "mahalle": MAHALLE[i % 4],
            "kullanim": KULLANIM[i % 3] if i % 4 != 3 else None,
            "alan": side * side,
            "x": X0 + (i % 5) * CELL,
            "y": Y0 + (i // 5) * CELL,
            "side": side,
        })
    return out


ROWS = rows()
BY_KEY = {r["fid"]: r for r in ROWS}


def corners(x, y, s):
    return [(x, y), (x + s, y), (x + s, y + s), (x, y + s)]


def gp_polygon(ring):
    """A ring as a GeoPackage blob: `GP`, version 0, flags (little-endian, envelope [minx, maxx, miny, maxy]), SRS
    5256, then the little-endian WKB polygon, closed."""
    closed = ring + [ring[0]]
    wkb = struct.pack("<BII", 1, 3, 1) + struct.pack("<I", len(closed)) + b"".join(struct.pack("<dd", *p) for p in closed)
    xs, ys = [p[0] for p in ring], [p[1] for p in ring]
    header = b"GP" + bytes([0, 0b00000011]) + struct.pack("<i", 5256) + struct.pack("<dddd", min(xs), max(xs), min(ys), max(ys))
    return header + wkb


# §F.3's triggers, GeoPackage 1.3: the index follows every write of the geometry column.
TRIGGERS = """
CREATE TRIGGER rtree_parseller_geom_insert AFTER INSERT ON parseller
  WHEN (new.geom NOT NULL AND NOT ST_IsEmpty(NEW.geom))
BEGIN
  INSERT OR REPLACE INTO rtree_parseller_geom VALUES (NEW.fid, ST_MinX(NEW.geom), ST_MaxX(NEW.geom), ST_MinY(NEW.geom), ST_MaxY(NEW.geom));
END;
CREATE TRIGGER rtree_parseller_geom_update1 AFTER UPDATE OF geom ON parseller
  WHEN OLD.fid = NEW.fid AND (NEW.geom NOTNULL AND NOT ST_IsEmpty(NEW.geom))
BEGIN
  INSERT OR REPLACE INTO rtree_parseller_geom VALUES (NEW.fid, ST_MinX(NEW.geom), ST_MaxX(NEW.geom), ST_MinY(NEW.geom), ST_MaxY(NEW.geom));
END;
CREATE TRIGGER rtree_parseller_geom_update2 AFTER UPDATE OF geom ON parseller
  WHEN OLD.fid = NEW.fid AND (NEW.geom ISNULL OR ST_IsEmpty(NEW.geom))
BEGIN
  DELETE FROM rtree_parseller_geom WHERE id = OLD.fid;
END;
CREATE TRIGGER rtree_parseller_geom_update3 AFTER UPDATE ON parseller
  WHEN OLD.fid != NEW.fid AND (NEW.geom NOTNULL AND NOT ST_IsEmpty(NEW.geom))
BEGIN
  DELETE FROM rtree_parseller_geom WHERE id = OLD.fid;
  INSERT OR REPLACE INTO rtree_parseller_geom VALUES (NEW.fid, ST_MinX(NEW.geom), ST_MaxX(NEW.geom), ST_MinY(NEW.geom), ST_MaxY(NEW.geom));
END;
CREATE TRIGGER rtree_parseller_geom_update4 AFTER UPDATE ON parseller
  WHEN OLD.fid != NEW.fid AND (NEW.geom ISNULL OR ST_IsEmpty(NEW.geom))
BEGIN
  DELETE FROM rtree_parseller_geom WHERE id IN (OLD.fid, NEW.fid);
END;
CREATE TRIGGER rtree_parseller_geom_delete AFTER DELETE ON parseller
  WHEN old.geom NOT NULL
BEGIN
  DELETE FROM rtree_parseller_geom WHERE id = OLD.fid;
END;
"""


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
        CREATE TABLE gpkg_extensions (table_name TEXT, column_name TEXT, extension_name TEXT NOT NULL,
            definition TEXT NOT NULL, scope TEXT NOT NULL);
        CREATE TABLE parseller (fid INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, geom POLYGON, ada INTEGER NOT NULL,
            parsel INTEGER, mahalle TEXT, kullanim TEXT, alan DOUBLE CHECK (alan IS NULL OR alan > 0));
        CREATE VIRTUAL TABLE rtree_parseller_geom USING rtree(id, minx, maxx, miny, maxy);
        """
    )
    for r in ROWS:
        ring = corners(r["x"], r["y"], r["side"])
        db.execute(
            "INSERT INTO parseller (fid, geom, ada, parsel, mahalle, kullanim, alan) VALUES (?, ?, ?, ?, ?, ?, ?)",
            (r["fid"], gp_polygon(ring), r["ada"], r["parsel"], r["mahalle"], r["kullanim"], r["alan"]),
        )
        db.execute("INSERT INTO rtree_parseller_geom VALUES (?, ?, ?, ?, ?)",
                   (r["fid"], r["x"], r["x"] + r["side"], r["y"], r["y"] + r["side"]))
    xs = [r["x"] for r in ROWS] + [r["x"] + r["side"] for r in ROWS]
    ys = [r["y"] for r in ROWS] + [r["y"] + r["side"] for r in ROWS]
    db.execute("INSERT INTO gpkg_contents (table_name, data_type, identifier, last_change, min_x, min_y, max_x, max_y, srs_id) "
               "VALUES ('parseller', 'features', 'parseller', '2026-11-21T00:00:00.000Z', ?, ?, ?, ?, 5256)",
               (min(xs), min(ys), max(xs), max(ys)))
    db.execute("INSERT INTO gpkg_geometry_columns VALUES ('parseller', 'geom', 'POLYGON', 5256, 0, 0)")
    db.execute("INSERT INTO gpkg_extensions VALUES ('parseller', 'geom', 'gpkg_rtree_index', "
               "'http://www.geopackage.org/spec120/#extension_rtree', 'write-only')")
    # The triggers after the rows: this script has no ST_ functions; the program that writes the file has them.
    db.executescript(TRIGGERS)
    db.commit()
    db.close()


def number_text(v):
    """A number as the source's value reads: an integer as is, a double in its shortest form without `.0`."""
    if isinstance(v, int):
        return str(v)
    return str(int(v)) if float(v).is_integer() else repr(float(v))


def attrs_of(r):
    """A row's values as the drawing's attributes read them: an empty value left out."""
    out = {}
    for f in FIELDS:
        v = r[f]
        if v is not None:
            out[f] = v if isinstance(v, str) else number_text(v)
    return out


def pts(ring):
    return [{"x": x, "y": y} for x, y in ring]


def object_of(r, layer="layer-1", ring=None):
    """The fields of a taken row's object a case checks: a polygon of the layer, the ring without its closing vertex."""
    return {"kind": "polygon", "layerId": layer, "attrs": attrs_of(r),
            "pts": pts(ring or corners(r["x"], r["y"], r["side"]))}


# ── Steps ───────────────────────────────────────────────────────────────────────────────────────────────────────

def copy(n):
    return {"op": "copyFile", "from": SOURCE, "to": f"$tmp:oturum/{n}.gpkg"}


def add_layer(n, layer="layer-1", name="Parseller", read_only=False):
    data = {"driver": "gpkg", "address": f"$tmp:oturum/{n}.gpkg", "table": "parseller", "key": "fid",
            "geometry": {"column": "geom", "typeName": "POLYGON", "srid": 5256}}
    if read_only:
        data["readOnly"] = True
    return {"op": "execute", "command": "cad.layers.data",
            "input": {"operation": "add", "name": name, "data": data, "fields": LAYER_FIELDS},
            "result": completed({"layer": layer, "revision": "$current"})}


def completed(output, warnings=None):
    return {"status": "completed", "output": output, "warnings": warnings or []}


def failed(code, message, path=None, uids=None):
    error = {"code": code, "message": message}
    if path is not None:
        error["path"] = path
    if uids is not None:
        error["uids"] = uids
    return {"status": "failed", "error": error}


def layer_state(layer="layer-1", editing=True, added=0, changed=0, deleted=0, keys=None, failure=None):
    s = {"layer": layer, "editing": editing, "added": added, "changed": changed, "deleted": deleted}
    if keys:
        s["keys"] = keys
    if failure is not None:
        s["failed"] = failure
    return s


def done(layers, objects=None):
    out = {"layers": layers, "revision": "$current"}
    if objects:
        out["objects"] = objects
    return out


def session(inp, result, expect=None, op="execute"):
    step = {"op": op, "input": inp, "result": result}
    if expect is not None:
        step["expect"] = expect
    return step


def start(layers=("layer-1",)):
    return session({"action": "start", "layers": list(layers)},
                   completed(done([layer_state(l) for l in layers])))


def draw(ring, attrs, slot, layer="layer-1"):
    """A polygon drawn on the layer by `cad.entities.create`: a new row."""
    return {"op": "execute", "command": "cad.entities.create",
            "input": {"layerId": layer, "objects": [{"geometry": {"kind": "polygon", "pts": pts(ring)}, "attrs": attrs}]},
            "result": completed({"created": [f"$uidOf:{slot}"], "ids": [slot], "revision": "$current"})}


def rows_of(keys, count=None):
    """`cad.layers.rows` of these keys: what the table holds after a save."""
    inp = {"layer": "layer-1", "keys": keys}
    return {"op": "execute", "command": "cad.layers.rows", "input": inp, "result": completed(count)}


def rows_answer(rs):
    return {"fields": FIELDS, "rows": [{"key": r["fid"], "attrs": attrs_of(r)} for r in rs], "offset": 0,
            "checked": "source"}


def any_result():
    return {"status": "completed", "output": "$any", "warnings": "$any"}


NEW_RING = corners(600300.0, 4410200.0, 12.0)
NEW_ATTRS = {"ada": "300", "parsel": "1", "mahalle": "Yeni", "kullanim": "Park"}
NEW_ROW = {"fid": 11, "ada": 300, "parsel": 1, "mahalle": "Yeni", "kullanim": "Park", "alan": None}


def cases():
    out = []

    def case(name, steps):
        out.append({"name": name, "steps": steps})

    n = iter(range(1, 100))

    case("Durum: düzenlenen katman yokken boş", [
        session({"action": "status"}, completed(done([]))),
    ])
    case("start katmansız: no_layer", [
        session({"action": "start"}, failed("no_layer", "Düzenleme oturumunun katmanını (layers) verin.", "layers")),
    ])
    case("take katmansız: no_layer", [
        session({"action": "take", "keys": [1]},
                failed("no_layer", "Düzenleme oturumunun katmanını (layers) verin.", "layers")),
    ])
    case("take iki katmanla: one_layer", [
        session({"action": "take", "layers": ["a", "b"], "keys": [1]},
                failed("one_layer", "Satır alma ve silme tek bir katmanla çalışır; 2 katman verildi.", "layers")),
    ])
    case("delete anahtarsız: no_keys", [
        session({"action": "delete", "layers": ["layer-1"]},
                failed("no_keys", "Satırların anahtarlarını (keys) verin.", "keys")),
    ])
    case("delete 10 001 anahtarla: invalid_count", [
        session({"action": "delete", "layers": ["layer-1"], "keys": list(range(1, 10002))},
                failed("invalid_count",
                       "Bir seferde en çok 10000 satır alınır ya da silinir; 10001 anahtar verildi.", "keys")),
    ])
    case("start bilinmeyen katman: layer_not_found", [
        session({"action": "start", "layers": ["yok"]},
                failed("layer_not_found", "“yok” kimlikli katman çizimde yok.", "layers[0]")),
    ])
    case("start çizim katmanı: not_a_data_layer", [
        session({"action": "start", "layers": ["cizim"]},
                failed("not_a_data_layer",
                       "“Çizim” bir veri katmanı değil; düzenleme oturumu yalnız bir tablodan gelen katmanın olur.",
                       "layers[0]")),
    ])
    k = next(n)
    case("start salt okunur katman: data_layer_read_only", [
        copy(k), add_layer(k, read_only=True),
        session({"action": "start", "layers": ["layer-1"]},
                failed("data_layer_read_only", "“Parseller” salt okunur bir veri katmanı; düzenlenemez.", "layers[0]")),
    ])
    k = next(n)
    case("save düzenlenmeyen katman: data_layer_not_editing", [
        copy(k), add_layer(k),
        session({"action": "save", "layers": ["layer-1"]},
                failed("data_layer_not_editing",
                       "“Parseller” düzenlemeye açık değil; önce düzenlemesini başlatın (action: start).", "layers[0]")),
    ])
    k = next(n)
    case("start düzenlemeye açar; durum katmanı söyler", [
        copy(k), add_layer(k),
        start(),
        session({"action": "status"}, completed(done([layer_state()]))),
        session({"action": "start", "layers": ["layer-1"]}, completed(done([layer_state()]))),
    ])
    k = next(n)
    case("Düzenlemeye açık olmayan veri katmanına nesne çizilmez", [
        copy(k), add_layer(k),
        {"op": "execute", "command": "cad.entities.create",
         "input": {"layerId": "layer-1", "objects": [{"geometry": {"kind": "polygon", "pts": pts(NEW_RING)},
                                                       "attrs": NEW_ATTRS}]},
         "result": failed("data_layer_not_editing", "$any", None, "$any"), "expect": {"ids": []}},
    ])
    k = next(n)
    case("Yeni satır çizilir, kaydedilir, çizimden çıkar ve tabloda okunur", [
        copy(k), add_layer(k), start(),
        draw(NEW_RING, NEW_ATTRS, 1),
        session({"action": "status"}, completed(done([layer_state(added=1)]))),
        session({"action": "save"}, completed(done([layer_state(added=1, keys=[11])])), {"ids": [], "canUndo": True}),
        rows_of([11], rows_answer([NEW_ROW])),
        session({"action": "take", "layers": ["layer-1"], "keys": [11]},
                completed(done([layer_state()], ["$uidOf:2"])),
                {"ids": [2], "fields": {"2": object_of(NEW_ROW, ring=NEW_RING)}}),
    ])
    k = next(n)
    case("Satır alınır: düzenleme değildir; tabloda olmayan anahtar boş döner", [
        copy(k), add_layer(k), start(),
        session({"action": "take", "layers": ["layer-1"], "keys": [3, 99]},
                completed(done([layer_state()], ["$uidOf:1", None])),
                {"ids": [1], "fields": {"1": object_of(BY_KEY[3])}}),
        session({"action": "take", "layers": ["layer-1"], "keys": [3]},
                completed(done([layer_state()], ["$uidOf:1"])), {"ids": [1]}),
    ])
    k = next(n)
    changed3 = dict(BY_KEY[3], kullanim="Okul")
    case("Alınan satırın değeri değişir, kaydedilir; tabloda yeni değer okunur", [
        copy(k), add_layer(k), start(),
        session({"action": "take", "layers": ["layer-1"], "keys": [3]},
                completed(done([layer_state()], ["$uidOf:1"]))),
        {"op": "execute", "command": "cad.fields.write",
         "input": {"layer": "layer-1", "write": {"kind": "calculate", "field": "kullanim", "expression": "'Okul'"}},
         "result": any_result()},
        session({"action": "status"}, completed(done([layer_state(changed=1)]))),
        session({"action": "save"}, completed(done([layer_state(changed=1)])), {"ids": []}),
        rows_of([3], rows_answer([changed3])),
    ])
    k = next(n)
    r4 = BY_KEY[4]
    moved4 = [(x + 5.0, y - 2.0) for x, y in corners(r4["x"], r4["y"], r4["side"])]
    case("Alınan satır taşınır, kaydedilir; yeniden alınınca yeni yerinde", [
        copy(k), add_layer(k), start(),
        session({"action": "take", "layers": ["layer-1"], "keys": [4]},
                completed(done([layer_state()], ["$uidOf:1"]))),
        {"op": "execute", "command": "cad.entities.transform",
         "input": {"uids": ["$uidOf:1"], "transform": {"kind": "move", "dx": 5, "dy": -2}}, "result": any_result()},
        session({"action": "save"}, completed(done([layer_state(changed=1)])), {"ids": []}),
        session({"action": "take", "layers": ["layer-1"], "keys": [4]},
                completed(done([layer_state()], ["$uidOf:2"])),
                {"ids": [2], "fields": {"2": object_of(r4, ring=moved4)}}),
    ])
    k = next(n)
    remaining = [r for r in ROWS if r["fid"] not in (5, 6)]
    case("Satırlar anahtarla silinir; geri alınır, yinelenir, kaydedilir", [
        copy(k), add_layer(k), start(),
        session({"action": "delete", "layers": ["layer-1"], "keys": [5, 6, 6]},
                completed(done([layer_state(deleted=2)])), {"canUndo": True}),
        {"op": "undo", "returns": "Satırları sil"},
        session({"action": "status"}, completed(done([layer_state()]))),
        {"op": "redo", "returns": "Satırları sil"},
        session({"action": "status"}, completed(done([layer_state(deleted=2)]))),
        session({"action": "save"}, completed(done([layer_state(deleted=2)])), {"canRedo": False}),
        rows_of(list(range(1, 11)), rows_answer(remaining)),
    ])
    k = next(n)
    case("Alınan satır silinir, kaydedilir; tabloda yok", [
        copy(k), add_layer(k), start(),
        session({"action": "take", "layers": ["layer-1"], "keys": [7]},
                completed(done([layer_state()], ["$uidOf:1"]))),
        {"op": "execute", "command": "cad.entities.delete", "input": {"uids": ["$uidOf:1"]}, "result": any_result()},
        session({"action": "status"}, completed(done([layer_state(deleted=1)]))),
        session({"action": "save"}, completed(done([layer_state(deleted=1)]))),
        rows_of([7], rows_answer([])),
    ])
    k = next(n)
    case("Geri al: değişiklikler bırakılır, katman açık kalır, tablo aynı", [
        copy(k), add_layer(k), start(),
        session({"action": "take", "layers": ["layer-1"], "keys": [3]},
                completed(done([layer_state()], ["$uidOf:1"]))),
        draw(NEW_RING, NEW_ATTRS, 2),
        session({"action": "delete", "layers": ["layer-1"], "keys": [8]}, completed(done([layer_state(added=1, deleted=1)]))),
        session({"action": "rollback"}, completed(done([layer_state()])), {"ids": [], "canRedo": False}),
        rows_of([3, 8], rows_answer([BY_KEY[3], BY_KEY[8]])),
    ])
    k = next(n)
    case("Vazgeç: değişiklikleri bırakır ve katmanı kapatır", [
        copy(k), add_layer(k), start(),
        draw(NEW_RING, NEW_ATTRS, 1),
        session({"action": "cancel", "layers": ["layer-1"]}, completed(done([layer_state(editing=False)])),
                {"ids": []}),
        session({"action": "status"}, completed(done([]))),
    ])
    k = next(n)
    case("Bitir: değişiklik varken reddedilir, yokken kapatır", [
        copy(k), add_layer(k), start(),
        draw(NEW_RING, NEW_ATTRS, 1),
        session({"action": "stop"},
                failed("data_edits_unsaved",
                       "“Parseller” katmanında kaydedilmemiş değişiklik var; düzenlemeyi bitirmeden önce kaydedin (save) ya da bırakın (rollback, cancel).",
                       "layers[0]")),
        session({"action": "rollback"}, completed(done([layer_state()]))),
        session({"action": "stop"}, completed(done([layer_state(editing=False)]))),
    ])
    k = next(n)
    no_ada = {"parsel": "2", "mahalle": "Yeni"}
    case("Boş geçilemeyen alan: hiçbir şey yazılmaz, satır çizimde bekler", [
        copy(k), add_layer(k), start(),
        draw(NEW_RING, no_ada, 1),
        session({"action": "save"},
                failed("data_save_failed",
                       "“Parseller” katmanının değişiklikleri tablosuna yazılamadı: “ada” alanı boş geçilemez.",
                       "layers[0]", ["$uidOf:1"]),
                {"ids": [1]}),
        session({"action": "status"}, completed(done([layer_state(added=1)]))),
        rows_of([11], rows_answer([])),
    ])
    k = next(n)
    case("Tablonun kısıtı reddeder: hiçbir şey yazılmaz, değişiklik bekler", [
        copy(k), add_layer(k), start(),
        session({"action": "take", "layers": ["layer-1"], "keys": [2]},
                completed(done([layer_state()], ["$uidOf:1"]))),
        draw(NEW_RING, NEW_ATTRS, 2),
        {"op": "execute", "command": "cad.fields.write",
         "input": {"layer": "layer-1", "write": {"kind": "calculate", "field": "alan", "expression": "0 - 5"}},
         "result": any_result()},
        session({"action": "save"}, failed("data_save_failed", "$any", "layers[0]"), {"ids": [1, 2]}),
        session({"action": "status"}, completed(done([layer_state(added=1, changed=1)]))),
        rows_of([2, 11], rows_answer([BY_KEY[2]])),
    ])
    a, b = next(n), next(n)
    failure = "“Parseller B” katmanının değişiklikleri tablosuna yazılamadı: “ada” alanı boş geçilemez."
    case("İki katman: yazılamayan uyarıyla söylenir, öteki yazılır", [
        copy(a), add_layer(a), copy(b), add_layer(b, "layer-2", "Parseller B"),
        start(("layer-1", "layer-2")),
        draw(NEW_RING, NEW_ATTRS, 1),
        draw(NEW_RING, no_ada, 2, "layer-2"),
        session({"action": "save"},
                completed(done([layer_state(added=1, keys=[11]),
                                layer_state("layer-2", added=1, failure=failure)]),
                          [{"code": "data_save_failed", "message": failure, "path": "layers[1]"}]),
                {"ids": [2]}),
    ])
    k = next(n)
    case("Proje yeniden açılınca yeni satır bekler, alınan satır tablosunda kalır", [
        copy(k), add_layer(k), start(),
        session({"action": "take", "layers": ["layer-1"], "keys": [3]},
                completed(done([layer_state()], ["$uidOf:1"]))),
        draw(NEW_RING, NEW_ATTRS, 2),
        # The file numbers its objects in their order: the new row is its first.
        {"op": "reopen", "expect": {"ids": [1], "fields": {"1": {"layerId": "layer-1", "attrs": NEW_ATTRS}}}},
        session({"action": "status"}, completed(done([layer_state(added=1)]))),
    ])
    k = next(n)
    case("Kaydedilecek bir şey yokken kaydetmek boş geçer", [
        copy(k), add_layer(k), start(),
        session({"action": "save", "layers": ["layer-1"]}, completed(done([layer_state()]))),
    ])
    k = next(n)
    case("Silinen satır alınmaz", [
        copy(k), add_layer(k), start(),
        session({"action": "delete", "layers": ["layer-1"], "keys": [5]}, completed(done([layer_state(deleted=1)]))),
        session({"action": "take", "layers": ["layer-1"], "keys": [5, 6]},
                completed(done([layer_state(deleted=1)], [None, "$uidOf:1"])), {"ids": [1]}),
    ])
    k = next(n)
    case("Açılamayan tablodan satır alınmaz: source_unreachable", [
        # No copy: the layer names a file that is not there. Opening reads nothing; taking reads the table.
        add_layer(f"yok-{k}"), start(),
        session({"action": "take", "layers": ["layer-1"], "keys": [1]},
                failed("source_unreachable", "$any", "layers"), {"ids": []}),
    ])
    # ── Düzenleme grupları ve çakışma (docs/adr/9104) ────────────────────────────────────────────────────────────
    def grouping(name):
        return {"op": "execute", "command": "cad.settings.set", "input": {"editGrouping": name},
                "result": completed({"changed": ["editGrouping"], "followed": 0, "locked": 0, "revision": "$current"})}

    k = next(n)
    case("Aynı kaynaktakiler birlikte: biri açılınca öteki de açılır, ikisi tek işlemle yazılır", [
        copy(k), add_layer(k), add_layer(k, "layer-2", "Parseller B"), grouping("source"),
        session({"action": "start", "layers": ["layer-1"]},
                completed(done([layer_state(), layer_state("layer-2")]))),
        draw(NEW_RING, NEW_ATTRS, 1),
        draw(NEW_RING, NEW_ATTRS, 2, "layer-2"),
        session({"action": "save", "layers": ["layer-1"]},
                completed(done([layer_state(added=1, keys=[11]), layer_state("layer-2", added=1, keys=[12])])),
                {"ids": []}),
        rows_of([11, 12], rows_answer([NEW_ROW, dict(NEW_ROW, fid=12)])),
    ])
    k = next(n)
    case("Aynı kaynaktakiler birlikte: biri yazılamazsa hiçbiri yazılmaz", [
        copy(k), add_layer(k), add_layer(k, "layer-2", "Parseller B"), grouping("source"),
        session({"action": "start", "layers": ["layer-1"]},
                completed(done([layer_state(), layer_state("layer-2")]))),
        draw(NEW_RING, NEW_ATTRS, 1),
        draw(NEW_RING, no_ada, 2, "layer-2"),
        session({"action": "save"},
                failed("data_save_failed",
                       "“Parseller B” katmanının değişiklikleri tablosuna yazılamadı: “ada” alanı boş geçilemez.",
                       "layers[0]", ["$uidOf:2"]),
                {"ids": [1, 2]}),
        rows_of([11], rows_answer([])),
    ])
    a, b = next(n), next(n)
    case("Bütün katmanlar birlikte: ayrı dosyalardaki katmanlar birlikte açılır ve kapanır", [
        copy(a), add_layer(a), copy(b), add_layer(b, "layer-2", "Parseller B"), grouping("all"),
        session({"action": "start", "layers": ["layer-1"]},
                completed(done([layer_state(), layer_state("layer-2")]))),
        session({"action": "stop", "layers": ["layer-1"]},
                completed(done([layer_state(editing=False), layer_state("layer-2", editing=False)]))),
    ])
    k = next(n)
    AGAIN = "Değişikliği bırakıp satırları yeniden alın (action: rollback, sonra take) ya da üstüne yazın (force: true)."
    # Both sides changed row 3's kullanim to different values: a clash said with its three values (docs/adr/9148).
    first3 = BY_KEY[3]["kullanim"]
    first3 = f"“{first3}”" if first3 else "boş"
    changed3 = ("Başkası aynı satırların aynı alanlarını siz aldıktan sonra değiştirmiş ya da satırı silmiş "
                f"(satır 3: kullanim (ilk {first3}, sizinki “Pazar”, tablodaki “Okul”)); hiçbir şey yazılmadı. " + AGAIN)
    case("Başkasının değiştirdiği satır: çakışma söylenir; üstüne yazmak yalnız değiştirileni yazar", [
        copy(k), add_layer(k), add_layer(k, "layer-2", "Parseller B"),
        start(("layer-1", "layer-2")),
        session({"action": "take", "layers": ["layer-1"], "keys": [3]},
                completed(done([layer_state()], ["$uidOf:1"]))),
        session({"action": "take", "layers": ["layer-2"], "keys": [3]},
                completed(done([layer_state("layer-2")], ["$uidOf:2"]))),
        {"op": "execute", "command": "cad.fields.write",
         "input": {"layer": "layer-2", "write": {"kind": "calculate", "field": "kullanim", "expression": "'Okul'"}},
         "result": any_result()},
        {"op": "execute", "command": "cad.fields.write",
         "input": {"layer": "layer-2", "write": {"kind": "calculate", "field": "mahalle", "expression": "'Yeni'"}},
         "result": any_result()},
        session({"action": "save", "layers": ["layer-2"]}, completed(done([layer_state("layer-2", changed=1)]))),
        {"op": "execute", "command": "cad.fields.write",
         "input": {"layer": "layer-1", "write": {"kind": "calculate", "field": "kullanim", "expression": "'Pazar'"}},
         "result": any_result()},
        session({"action": "save", "layers": ["layer-1"]},
                {"status": "conflict", "error": {"code": "rows_changed", "message": changed3, "path": "layers[0]",
                                                 "uids": ["$uidOf:1"], "retryable": True}},
                {"ids": [1]}),
        rows_of([3], rows_answer([dict(BY_KEY[3], kullanim="Okul", mahalle="Yeni")])),
        session({"action": "save", "layers": ["layer-1"], "force": True},
                completed(done([layer_state(changed=1)])), {"ids": []}),
        rows_of([3], rows_answer([dict(BY_KEY[3], kullanim="Pazar", mahalle="Yeni")])),
    ])
    k = next(n)
    gone4 = ("Başkası aynı satırların aynı alanlarını siz aldıktan sonra değiştirmiş ya da satırı silmiş "
             "(satır 4: tabloda silinmiş, sizde değişmiş); hiçbir şey yazılmadı. " + AGAIN)
    case("Başkasının sildiği satır: çakışma söylenir, hiçbir şey yazılmaz", [
        copy(k), add_layer(k), add_layer(k, "layer-2", "Parseller B"),
        start(("layer-1", "layer-2")),
        session({"action": "take", "layers": ["layer-1"], "keys": [4]},
                completed(done([layer_state()], ["$uidOf:1"]))),
        session({"action": "delete", "layers": ["layer-2"], "keys": [4]},
                completed(done([layer_state("layer-2", deleted=1)]))),
        session({"action": "save", "layers": ["layer-2"]}, completed(done([layer_state("layer-2", deleted=1)]))),
        {"op": "execute", "command": "cad.entities.transform",
         "input": {"uids": ["$uidOf:1"], "transform": {"kind": "move", "dx": 5, "dy": -2}}, "result": any_result()},
        session({"action": "save", "layers": ["layer-1"]},
                {"status": "conflict", "error": {"code": "rows_changed", "message": gone4, "path": "layers[0]",
                                                 "uids": ["$uidOf:1"], "retryable": True}},
                {"ids": [1]}),
    ])
    k = next(n)
    case("plan oturumu söyler, hiçbir şey yazmaz", [
        copy(k), add_layer(k), start(),
        draw(NEW_RING, NEW_ATTRS, 1),
        session({"action": "save"}, completed(done([layer_state(added=1)])), {"ids": [1]}, op="plan"),
        session({"action": "save"}, {"status": "completed", "output": None, "warnings": []}, op="validate"),
    ])
    return out


def setup():
    return {
        "format": "kentos.document", "version": 1, "name": "Veri katmanının düzenleme oturumu",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": 600000, "y": 4410000},
        "layers": [{"id": "cizim", "name": "Çizim", "type": "layer", "visible": True, "locked": False,
                    "expanded": True, "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25},
                    "children": []}],
        "activeLayer": "cizim", "entities": [], "styles": {"items": [], "categories": []},
    }


def document():
    return {
        "format": "kentos.command-cases", "version": 1, "command": COMMAND, "commandVersion": 1,
        "title": "Veri katmanının düzenleme oturumu",
        "note": "ADR 9103. GeoPackage'ı bu betik sqlite3 ile yazar (fixtures/data/v1/oturum.gpkg; R*Tree dizini ve "
                "§F.3 tetikleyicileri); her durum kendi kopyasına yazar ve veri katmanını kendisi ekler. Beklentiler "
                "sözleşmenin kurallarıyla burada hesaplanır: retler ve sırası, katmanın yazacakları, kaydetmenin tabloya "
                "yazdığı ve çizimden çıkardığı, geri alma, bitirme, satır alma, tablonun reddi.",
        "setup": setup(),
        "cases": cases(),
    }


def main():
    text = json.dumps(document(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            print("cad.layers.session.json differs from what the script writes", file=sys.stderr)
            sys.exit(1)
        print(f"{len(document()['cases'])} cases as on disk")
        return
    write_gpkg()
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(document()['cases'])} cases; {GPKG.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
