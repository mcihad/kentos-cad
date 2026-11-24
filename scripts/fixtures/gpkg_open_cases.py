"""Writes fixtures/data/v1/gpkg-open/: GeoPackages and expected.json, the cases of opening a GeoPackage as a project
(docs/adr/9116). Each file is written here with sqlite3 (OGC 12-128r19: its tables, the `GP` blobs with little-endian
WKB, the R*Tree index; another program's `layer_styles` table; KentOS's `kentos_projects` table as docs/adr/9107 §2
says it), and every expectation is worked out here from the ADR's rules, without KentOS code:

- a file that keeps one project opens it; one that keeps more names them and opens none;
- a file without a project opens as a new project of its tables, written into it under the file's name;
- the project's system is the system most of its geometry tables are in, of the systems a project can have
  (fixtures/crs/v1/registry.json), the first in the file's order on a tie; none of them: no system (0, "Yerel");
- its layers: the tables as data layers, points first, then lines, areas, other geometry, then tables without
  geometry, each group in the file's order, under a Taslak drawing layer; the style table is not a layer;
- a table's default style (`useAsDefault`, else the latest written) is its layer's look;
- its tile tables (docs/adr/9117) are rasters linked to the table at its finest zoom, each on a drawing layer of its
  own named after it, under the other layers; they vote on the system after the geometry tables.

The native tests (crates/native/application/tests/all/gpkg_open.rs) open copies of the files and compare.

    python3 scripts/fixtures/gpkg_open_cases.py           # writes the files
    python3 scripts/fixtures/gpkg_open_cases.py --check   # compares expected.json with the one on disk
"""
import json
import sqlite3
import struct
import sys
import tempfile
import zlib
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/data/v1/gpkg-open"
REGISTRY = ROOT / "fixtures/crs/v1/registry.json"
KCAD = ROOT / "fixtures/kcad/v2/minimal.kcad"
# Two layer styles as the other program wrote them (QML, categories on `ada`; one colour).
QML_CATEGORIZED = OUT / "stil-kategorili.qml"
QML_SINGLE = OUT / "stil-tek-renk.qml"

SYSTEMS = {
    4326: ("WGS 84 geodetic", 'GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]],'
                              'PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]]'),
    5256: ("TUREF / TM36", 'PROJCS["TUREF / TM36",GEOGCS["TUREF",DATUM["Turkish_National_Reference_Frame",'
                           'SPHEROID["GRS 1980",6378137,298.257222101]],PRIMEM["Greenwich",0],'
                           'UNIT["degree",0.0174532925199433]],PROJECTION["Transverse_Mercator"],'
                           'PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",36],'
                           'PARAMETER["scale_factor",1],PARAMETER["false_easting",500000],'
                           'PARAMETER["false_northing",0],UNIT["metre",1]]'),
    2154: ("RGF93 v1 / Lambert-93", 'PROJCS["RGF93 v1 / Lambert-93",GEOGCS["RGF93 v1",DATUM["Reseau_Geodesique_Francais_1993_v1",'
                                    'SPHEROID["GRS 1980",6378137,298.257222101]],PRIMEM["Greenwich",0],'
                                    'UNIT["degree",0.0174532925199433]],PROJECTION["Lambert_Conformal_Conic_2SP"],'
                                    'PARAMETER["standard_parallel_1",49],PARAMETER["standard_parallel_2",44],'
                                    'PARAMETER["latitude_of_origin",46.5],PARAMETER["central_meridian",3],'
                                    'PARAMETER["false_easting",700000],PARAMETER["false_northing",6600000],UNIT["metre",1]]'),
}

POINTS = {"POINT", "MULTIPOINT"}
LINES = {"LINESTRING", "MULTILINESTRING", "CIRCULARSTRING", "COMPOUNDCURVE", "MULTICURVE", "CURVE"}
AREAS = {"POLYGON", "MULTIPOLYGON", "CURVEPOLYGON", "MULTISURFACE", "SURFACE"}
# The table another program keeps its layers' styles in: not a layer (docs/adr/9116 §2).
STYLE_TABLE = "layer_styles"


# ── Geometry blobs ───────────────────────────────────────────────────────


def gp(srs, wkb, box):
    """A GeoPackage blob: `GP`, version 0, flags (little-endian, envelope [minx, maxx, miny, maxy]), the SRS, the WKB."""
    minx, miny, maxx, maxy = box
    return b"GP" + bytes([0, 0b00000011]) + struct.pack("<i", srs) + struct.pack("<dddd", minx, maxx, miny, maxy) + wkb


def point(srs, x, y):
    return gp(srs, struct.pack("<BIdd", 1, 1, x, y), (x, y, x, y))


def line(srs, pts):
    wkb = struct.pack("<BII", 1, 2, len(pts)) + b"".join(struct.pack("<dd", *p) for p in pts)
    xs, ys = [p[0] for p in pts], [p[1] for p in pts]
    return gp(srs, wkb, (min(xs), min(ys), max(xs), max(ys)))


def square_ring(x, y, s):
    return [(x, y), (x + s, y), (x + s, y + s), (x, y + s), (x, y)]


def polygon(srs, x, y, s):
    ring = square_ring(x, y, s)
    wkb = struct.pack("<BII", 1, 3, 1) + struct.pack("<I", len(ring)) + b"".join(struct.pack("<dd", *p) for p in ring)
    return gp(srs, wkb, (x, y, x + s, y + s))


def png(w, h, rgba):
    """A PNG of one colour (8-bit RGBA), written here: signature, IHDR, the rows deflated in IDAT, IEND."""
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    row = b"\x00" + bytes(rgba) * w
    ihdr = struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(row * h)) + chunk(b"IEND", b"")


def box_of(geoms):
    """The union of the rows' boxes: [minx, miny, maxx, maxy]."""
    xs, ys = [], []
    for g in geoms:
        minx, maxx, miny, maxy = struct.unpack("<dddd", g[8:40])
        xs += [minx, maxx]
        ys += [miny, maxy]
    return [min(xs), min(ys), max(xs), max(ys)] if xs else None


# ── A GeoPackage ─────────────────────────────────────────────────────────


class Gpkg:
    """A GeoPackage written table by table; `tables` keeps what each one is, in the file's order."""

    def __init__(self, path):
        self.path = path
        if path.exists():
            path.unlink()
        self.db = sqlite3.connect(path)
        self.tables = []
        self.tiles_made = []
        self.db.executescript(
            """
            PRAGMA application_id = 1196444487;
            PRAGMA user_version = 10400;
            CREATE TABLE gpkg_spatial_ref_sys (srs_name TEXT NOT NULL, srs_id INTEGER PRIMARY KEY,
                organization TEXT NOT NULL, organization_coordsys_id INTEGER NOT NULL, definition TEXT NOT NULL,
                description TEXT);
            INSERT INTO gpkg_spatial_ref_sys VALUES ('Undefined cartesian SRS', -1, 'NONE', -1, 'undefined', NULL),
                ('Undefined geographic SRS', 0, 'NONE', 0, 'undefined', NULL);
            CREATE TABLE gpkg_contents (table_name TEXT NOT NULL PRIMARY KEY, data_type TEXT NOT NULL,
                identifier TEXT UNIQUE, description TEXT DEFAULT '',
                last_change DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
                min_x DOUBLE, min_y DOUBLE, max_x DOUBLE, max_y DOUBLE, srs_id INTEGER);
            CREATE TABLE gpkg_geometry_columns (table_name TEXT NOT NULL, column_name TEXT NOT NULL,
                geometry_type_name TEXT NOT NULL, srs_id INTEGER NOT NULL, z TINYINT NOT NULL, m TINYINT NOT NULL,
                CONSTRAINT pk_geom_cols PRIMARY KEY (table_name, column_name));
            CREATE TABLE gpkg_extensions (table_name TEXT, column_name TEXT, extension_name TEXT NOT NULL,
                definition TEXT NOT NULL, scope TEXT NOT NULL,
                CONSTRAINT ge_tce UNIQUE (table_name, column_name, extension_name));
            """
        )

    def system(self, srs):
        if srs in SYSTEMS:
            name, wkt = SYSTEMS[srs]
            self.db.execute("INSERT OR IGNORE INTO gpkg_spatial_ref_sys VALUES (?, ?, 'EPSG', ?, ?, NULL)",
                            (name, srs, srs, wkt))

    def features(self, name, type_name, srs, columns, rows):
        """A feature table: `fid`, `geom`, the columns (name, SQL type); each row its geometry blob and values."""
        self.system(srs)
        cols = ", ".join(f'"{c}" {t}' for c, t in columns)
        self.db.execute(f'CREATE TABLE "{name}" (fid INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, geom {type_name}'
                        + (f", {cols}" if cols else "") + ")")
        self.db.execute(f'CREATE VIRTUAL TABLE "rtree_{name}_geom" USING rtree(id, minx, maxx, miny, maxy)')
        marks = ", ".join("?" for _ in range(len(columns) + 1))
        names = ", ".join(["geom"] + [f'"{c}"' for c, _ in columns])
        for i, (geom, *values) in enumerate(rows, start=1):
            self.db.execute(f'INSERT INTO "{name}" (fid, {names}) VALUES (?, {marks})', (i, geom, *values))
            minx, maxx, miny, maxy = struct.unpack("<dddd", geom[8:40])
            self.db.execute(f'INSERT INTO "rtree_{name}_geom" VALUES (?, ?, ?, ?, ?)', (i, minx, maxx, miny, maxy))
        box = box_of([r[0] for r in rows])
        self.db.execute(
            "INSERT INTO gpkg_contents (table_name, data_type, identifier, last_change, min_x, min_y, max_x, max_y, srs_id) "
            "VALUES (?, 'features', ?, '2026-11-22T00:00:00.000Z', ?, ?, ?, ?, ?)",
            (name, name, *(box or [None] * 4), srs))
        self.db.execute("INSERT INTO gpkg_geometry_columns VALUES (?, 'geom', ?, ?, 0, 0)", (name, type_name, srs))
        self.db.execute("INSERT INTO gpkg_extensions VALUES (?, 'geom', 'gpkg_rtree_index', "
                        "'http://www.geopackage.org/spec120/#extension_rtree', 'write-only')", (name,))
        self.tables.append({"table": name, "type": type_name, "srid": srs, "fields": [c for c, _ in columns],
                            "box": box})

    def attributes(self, name, columns, rows):
        cols = ", ".join(f'"{c}" {t}' for c, t in columns)
        self.db.execute(f'CREATE TABLE "{name}" (fid INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, {cols})')
        marks = ", ".join("?" for _ in columns)
        names = ", ".join(f'"{c}"' for c, _ in columns)
        for i, values in enumerate(rows, start=1):
            self.db.execute(f'INSERT INTO "{name}" (fid, {names}) VALUES (?, {marks})', (i, *values))
        self.db.execute("INSERT INTO gpkg_contents (table_name, data_type, identifier, last_change) "
                        "VALUES (?, 'attributes', ?, '2026-11-22T00:00:00.000Z')", (name, name))
        self.tables.append({"table": name, "type": None, "srid": None, "fields": [c for c, _ in columns],
                            "box": None})

    def tiles(self, name, srs, min_x, max_y, levels):
        """A tile pyramid of pictures (OGC 12-128r19 §2.2): the matrix set from (`min_x`, `max_y`), each level
        (zoom, columns, rows, pixel size) of 256 × 256 PNG tiles of one colour each."""
        self.system(srs)
        self.db.executescript(
            """
            CREATE TABLE IF NOT EXISTS gpkg_tile_matrix_set (table_name TEXT NOT NULL PRIMARY KEY, srs_id INTEGER NOT NULL,
                min_x DOUBLE NOT NULL, min_y DOUBLE NOT NULL, max_x DOUBLE NOT NULL, max_y DOUBLE NOT NULL);
            CREATE TABLE IF NOT EXISTS gpkg_tile_matrix (table_name TEXT NOT NULL, zoom_level INTEGER NOT NULL,
                matrix_width INTEGER NOT NULL, matrix_height INTEGER NOT NULL, tile_width INTEGER NOT NULL,
                tile_height INTEGER NOT NULL, pixel_x_size DOUBLE NOT NULL, pixel_y_size DOUBLE NOT NULL,
                CONSTRAINT pk_ttm PRIMARY KEY (table_name, zoom_level));
            """
        )
        zoom0, cols0, rows0, px0 = levels[0]
        max_x, min_y = min_x + cols0 * 256 * px0, max_y - rows0 * 256 * px0
        self.db.execute(f'CREATE TABLE "{name}" (id INTEGER PRIMARY KEY AUTOINCREMENT, zoom_level INTEGER NOT NULL, '
                        'tile_column INTEGER NOT NULL, tile_row INTEGER NOT NULL, tile_data BLOB NOT NULL, '
                        'UNIQUE (zoom_level, tile_column, tile_row))')
        self.db.execute("INSERT INTO gpkg_tile_matrix_set VALUES (?, ?, ?, ?, ?, ?)", (name, srs, min_x, min_y, max_x, max_y))
        for zoom, cols, rows, px in levels:
            self.db.execute("INSERT INTO gpkg_tile_matrix VALUES (?, ?, ?, ?, 256, 256, ?, ?)", (name, zoom, cols, rows, px, px))
            for c in range(cols):
                for r in range(rows):
                    colour = (40 + 50 * c, 120 + 40 * r, 90 + 30 * zoom, 255)
                    self.db.execute(f'INSERT INTO "{name}" (zoom_level, tile_column, tile_row, tile_data) VALUES (?, ?, ?, ?)',
                                    (zoom, c, r, png(256, 256, colour)))
        self.db.execute("INSERT INTO gpkg_contents (table_name, data_type, identifier, last_change, min_x, min_y, max_x, max_y, srs_id) "
                        "VALUES (?, 'tiles', ?, '2026-11-22T00:00:00.000Z', ?, ?, ?, ?, ?)",
                        (name, name, min_x, min_y, max_x, max_y, srs))
        self.tiles_made.append({"table": name, "srid": srs, "zoom": max(l[0] for l in levels),
                                "box": [min_x, min_y, max_x, max_y]})

    def styles(self, rows):
        """Another program's style table, as it makes it (an attribute table of `gpkg_contents`): each row its table,
        style name, QML, whether it is the default and when it was written."""
        self.db.execute(
            'CREATE TABLE "layer_styles" ( "id" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, "f_table_catalog" TEXT(256), '
            '"f_table_schema" TEXT(256), "f_table_name" TEXT(256), "f_geometry_column" TEXT(256), "styleName" TEXT(30), '
            '"styleQML" TEXT, "styleSLD" TEXT, "useAsDefault" BOOLEAN, "description" TEXT, "owner" TEXT(30), '
            '"ui" TEXT(30), "update_time" DATETIME DEFAULT (strftime(\'%Y-%m-%dT%H:%M:%fZ\',\'now\')))')
        for table, name, qml, default, when in rows:
            self.db.execute(
                "INSERT INTO layer_styles (f_table_catalog, f_table_schema, f_table_name, f_geometry_column, styleName, "
                "styleQML, styleSLD, useAsDefault, description, owner, ui, update_time) "
                "VALUES ('', '', ?, 'geom', ?, ?, NULL, ?, '', '', NULL, ?)",
                (table, name, qml, 1 if default else 0, when))
        self.db.execute("INSERT INTO gpkg_contents (table_name, data_type, identifier, last_change) "
                        "VALUES ('layer_styles', 'attributes', 'layer_styles', '2026-11-22T00:00:00.000Z')")
        self.tables.append({"table": STYLE_TABLE, "type": None, "srid": None, "fields": [], "box": None})

    def projects(self, named):
        """KentOS's project table (docs/adr/9107 §2), each project the same `.kcad` bytes."""
        content = KCAD.read_bytes()
        self.db.executescript(
            """
            CREATE TABLE kentos_projects (name TEXT PRIMARY KEY NOT NULL, metadata TEXT NOT NULL, content BLOB NOT NULL);
            INSERT INTO gpkg_extensions VALUES ('kentos_projects', NULL, 'kentos_projects',
                'https://kentos.app/geopackage/projects (docs/adr/9107)', 'read-write');
            """
        )
        for name in named:
            self.db.execute(
                "INSERT INTO kentos_projects VALUES (?, ?, ?)",
                (name, json.dumps({"modified": "2026-11-21T10:00:00Z", "user": "ayse", "format": "kcad"}), content))

    def close(self):
        self.db.commit()
        self.db.close()


# ── The rules (docs/adr/9116) ────────────────────────────────────────────


def listed_systems():
    return {s["srid"] for s in json.loads(REGISTRY.read_text(encoding="utf-8"))["systems"]}


def project_system(tables, tiles=()):
    """The project's system and the notes it brings: the geometry tables' systems in the file's order, then the tile
    tables'."""
    layers = [t for t in tables if t["table"] != STYLE_TABLE]
    if not layers and not tiles:
        return 0, ["no_tables"]
    systems = [t["srid"] for t in layers if t["type"] and t["srid"] and t["srid"] > 0]
    systems += [t["srid"] for t in tiles if t["srid"] and t["srid"] > 0]
    notes = []
    if len(set(systems)) > 1:
        notes.append("mixed_systems")
    listed = [s for s in systems if s in LISTED]
    if listed:
        counts = Counter(listed)
        most = max(counts.values())
        return next(s for s in listed if counts[s] == most), notes
    if systems:
        return 0, notes + ["system_not_listed"]
    return 0, notes + ["no_system"]


def group(t):
    if t["type"] is None:
        return 4
    if t["type"] in POINTS:
        return 0
    if t["type"] in LINES:
        return 1
    if t["type"] in AREAS:
        return 2
    return 3


def layers_of(tables, styled, tiles=()):
    order = sorted((t for t in tables if t["table"] != STYLE_TABLE), key=group)  # stable: the file's order in a group
    out = [{"name": "Taslak", "table": None}]
    rasters = [{"name": t["table"], "table": None, "raster": {"table": t["table"], "zoom": t["zoom"]}} for t in tiles]
    for t in order:
        out.append({
            "name": t["table"],
            "table": t["table"],
            "key": "fid",
            "geometry": None if t["type"] is None else {"column": "geom", "type": t["type"],
                                                         "srid": t["srid"] if t["srid"] and t["srid"] > 0 else None},
            "fields": t["fields"],
            "styled": styled.get(t["table"]),
        })
    return out + rasters


def home_of(tables, srid, tiles=()):
    """The project's first view: the union of the boxes of the tables and tile tables in its system (other systems are
    moved by KentOS, not here: such a case says none)."""
    if any(t["type"] and t["srid"] != srid for t in tables if t["table"] != STYLE_TABLE):
        return None
    if any(t["srid"] != srid for t in tiles):
        return None
    boxes = [t["box"] for t in tables if t["box"]] + [t["box"] for t in tiles]
    if not boxes:
        return None
    return [min(b[0] for b in boxes), min(b[1] for b in boxes), max(b[2] for b in boxes), max(b[3] for b in boxes)]


def default_styles(rows, tables):
    """Each table's default style as the other program picks it: marked default first, then the latest written; the
    renderer's type its QML names. A style of a table the file does not have is passed over."""
    have = {t["table"] for t in tables}
    best = {}
    for table, _name, qml, default, when in rows:
        if table not in have:
            continue
        key = (1 if default else 0, when)
        if table not in best or key > best[table][0]:
            best[table] = (key, qml)
    kinds = {"categorizedSymbol": "categorized", "singleSymbol": "single"}
    out = {}
    for table, (_, qml) in best.items():
        kind = qml.split('<renderer-v2 ', 1)[1].split('type="', 1)[1].split('"', 1)[0]
        out[table] = kinds[kind]
    return out


# ── The cases ────────────────────────────────────────────────────────────

LISTED = listed_systems()
X0, Y0 = 640000.0, 4400000.0


def built(name, g, say, styles=None):
    srid, notes = project_system(g.tables, g.tiles_made)
    styled = default_styles(styles or [], g.tables)
    return {
        "file": name, "say": say, "opens": "built", "project": Path(name).stem, "srid": srid, "notes": notes,
        "layers": layers_of(g.tables, styled, g.tiles_made), "home": home_of(g.tables, srid, g.tiles_made),
    }


def katmanlar():
    """Every kind of table, an empty one, a table without geometry and the style table, in a mixed order."""
    g = Gpkg(OUT / "katmanlar.gpkg")
    g.features("binalar", "POLYGON", 5256, [("ada", "TEXT"), ("kat", "INTEGER")],
               [(polygon(5256, X0 + 30 * i, Y0, 20), ada, i + 1) for i, ada in enumerate("ABCD")])
    g.attributes("malikler", [("ad", "TEXT"), ("pay", "TEXT")], [("Ayşe Yılmaz", "1/2"), ("Ali Kaya", "1/2")])
    g.features("kuyular", "POINT", 5256, [("ad", "TEXT"), ("derinlik", "REAL")],
               [(point(5256, X0 + 10 + 25 * i, Y0 - 15), f"K{i + 1}", 40.5 + i) for i in range(4)])
    categorized = QML_CATEGORIZED.read_text(encoding="utf-8")
    single = QML_SINGLE.read_text(encoding="utf-8")
    rows = [
        # The default one is older than the other: the default wins.
        ("binalar", "Adalara göre", categorized, True, "2026-11-20T09:00:00.000Z"),
        ("binalar", "Tek renk", single, False, "2026-11-21T09:00:00.000Z"),
        # A table the file does not have.
        ("yok", "Eski", single, True, "2026-11-19T09:00:00.000Z"),
    ]
    g.styles(rows)
    g.features("bos", "MULTIPOLYGON", 5256, [("ad", "TEXT")], [])
    g.features("yollar", "LINESTRING", 5256, [("ad", "TEXT"), ("genislik", "REAL")],
               [(line(5256, [(X0, Y0 + 40), (X0 + 120, Y0 + 40)]), "Atatürk Cad.", 12.0),
                (line(5256, [(X0 + 60, Y0 - 30), (X0 + 60, Y0 + 60)]), "Sivas Yolu", 8.5)])
    case = built("katmanlar.gpkg", g, "Her türden tablo, boş bir tablo, geometrisiz tablo ve stil tablosu", rows)
    g.close()
    return case


def karisik():
    g = Gpkg(OUT / "karisik.gpkg")
    g.features("a", "POINT", 5256, [], [(point(5256, X0, Y0),)])
    g.features("b", "POLYGON", 4326, [], [(polygon(4326, 37.0, 39.7, 0.01),)])
    g.features("c", "LINESTRING", 5256, [], [(line(5256, [(X0, Y0), (X0 + 50, Y0 + 50)]),)])
    case = built("karisik.gpkg", g, "Tabloların çoğu TM36'da, biri WGS 84'te: proje TM36")
    g.close()
    return case


def esitlik():
    g = Gpkg(OUT / "esitlik.gpkg")
    g.features("d", "POINT", 4326, [], [(point(4326, 37.0, 39.7),)])
    g.features("e", "POINT", 5256, [], [(point(5256, X0, Y0),)])
    case = built("esitlik.gpkg", g, "İki sistem eşit sayıda: dosyada önce gelen")
    g.close()
    return case


def yabanci():
    g = Gpkg(OUT / "yabanci-sistem.gpkg")
    g.features("parseller", "POLYGON", 2154, [("no", "INTEGER")], [(polygon(2154, 652000.0, 6862000.0, 30), 1)])
    case = built("yabanci-sistem.gpkg", g, "Projenin alamayacağı bir sistem: proje sistemsiz")
    g.close()
    return case


def sistemsiz():
    g = Gpkg(OUT / "sistemsiz.gpkg")
    g.attributes("defter", [("ad", "TEXT"), ("tutar", "REAL")], [("Kira", 1500.0), ("Aidat", 300.0)])
    case = built("sistemsiz.gpkg", g, "Yalnız geometrisiz tablo: proje sistemsiz")
    g.close()
    return case


def tanimsiz():
    g = Gpkg(OUT / "tanimsiz.gpkg")
    g.features("noktalar", "POINT", -1, [("ad", "TEXT")], [(point(-1, 10.0, 20.0), "N1")])
    case = built("tanimsiz.gpkg", g, "Tanımsız kartezyen sistem (-1): proje sistemsiz")
    g.close()
    return case


def derece():
    """A neighbourhood in WGS 84 degrees: a 270 m line and 20 m buildings, as a downloaded city extract is. The
    project is in degrees; its home is a few thousandths of a degree across (docs/adr/9117 §5)."""
    g = Gpkg(OUT / "derece.gpkg")
    g.features("teleferik", "LINESTRING", 4326, [("ad", "TEXT")],
               [(line(4326, [(32.8500, 39.9200), (32.8507, 39.9224)]), "Kale hattı")])
    g.features("binalar", "POLYGON", 4326, [("kat", "INTEGER")],
               [(polygon(4326, 32.8510 + 0.0004 * i, 39.9205, 0.0002), i + 1) for i in range(3)])
    case = built("derece.gpkg", g, "WGS 84 derecesinde bir mahalle: proje derecede, ilk görünüm birkaç binde bir derece")
    g.close()
    return case


def bos():
    g = Gpkg(OUT / "bos.gpkg")
    case = built("bos.gpkg", g, "Tablosuz GeoPackage: yalnız Taslak katmanıyla açılır")
    g.close()
    return case


def ortofotolu():
    """A table of parcels and an orthophoto's tile table in its system: the raster under the parcels."""
    g = Gpkg(OUT / "ortofotolu.gpkg")
    g.features("parseller", "POLYGON", 5256, [("no", "INTEGER")],
               [(polygon(5256, X0 + 10, Y0 - 100, 40), 1), (polygon(5256, X0 + 60, Y0 - 100, 40), 2)])
    g.tiles("ortofoto", 5256, X0, Y0, [(0, 1, 1, 0.5), (1, 2, 2, 0.25)])
    case = built("ortofotolu.gpkg", g, "Parseller ve ortofotonun döşeme tablosu: raster en altta, tabloya bağlı")
    g.close()
    return case


def yalniz_ortofoto():
    g = Gpkg(OUT / "yalniz-ortofoto.gpkg")
    g.tiles("hava", 5256, X0, Y0, [(0, 1, 1, 1.0)])
    case = built("yalniz-ortofoto.gpkg", g, "Yalnız döşeme tablosu: sistemi onun, Taslak'ın altında raster")
    g.close()
    return case


def projeli():
    g = Gpkg(OUT / "projeli.gpkg")
    g.features("kuyular", "POINT", 5256, [("ad", "TEXT")], [(point(5256, X0, Y0), "K1")])
    g.projects(["Sivas altyapı"])
    g.close()
    return {"file": "projeli.gpkg", "say": "Bir projesi var: o açılır, yenisi kurulmaz", "opens": "kept",
            "project": "Sivas altyapı"}


def iki_projeli():
    g = Gpkg(OUT / "iki-projeli.gpkg")
    g.projects(["Zeytinlik", "Çamlık"])
    g.close()
    return {"file": "iki-projeli.gpkg", "say": "İki projesi var: adları söylenir, biri seçilir",
            "opens": "several", "projects": ["Çamlık", "Zeytinlik"]}


def degil():
    path = OUT / "gpkg-degil.gpkg"
    path.write_text("Bu bir GeoPackage değil.\n", encoding="utf-8")
    return {"file": "gpkg-degil.gpkg", "say": "Uzantısı .gpkg olan metin dosyası", "opens": "refused"}


def cases():
    OUT.mkdir(parents=True, exist_ok=True)
    return [katmanlar(), karisik(), esitlik(), yabanci(), sistemsiz(), tanimsiz(), bos(), ortofotolu(), yalniz_ortofoto(),
            derece(), projeli(), iki_projeli(), degil()]


def document(all_cases):
    return {
        "about": "Opening a GeoPackage as a project (docs/adr/9116), written by scripts/fixtures/gpkg_open_cases.py "
                 "with sqlite3; the expectations follow the ADR's rules, worked out without KentOS code.",
        "cases": all_cases,
    }


def main():
    global OUT
    target = OUT / "expected.json"
    if "--check" in sys.argv:
        # --check writes nothing: the GeoPackages are made again in a temporary folder (their bytes are sqlite's, which
        # differs between Python builds; only expected.json is compared).
        with tempfile.TemporaryDirectory() as tmp:
            OUT = Path(tmp)
            text = json.dumps(document(cases()), ensure_ascii=False, indent=1) + "\n"
        on_disk = target.read_text(encoding="utf-8") if target.exists() else ""
        if on_disk != text:
            sys.exit(f"{target} differs: run python3 scripts/fixtures/gpkg_open_cases.py")
        print("ok")
        return
    text = json.dumps(document(cases()), ensure_ascii=False, indent=1) + "\n"
    target.write_text(text, encoding="utf-8")
    print(f"wrote {target} and {len(list(OUT.glob('*.gpkg')))} GeoPackages")


if __name__ == "__main__":
    main()
