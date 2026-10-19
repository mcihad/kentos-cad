"""Writes the GeoPackage files of the many-to-many references' shared cases (docs/adr/0532): the building, parcel and
owner example of the bound layers' relations, in the states a file can hold a reference in.

    python3 scripts/fixtures/source_relations_files.py           # writes fixtures/geopackage/v1/iliskili*.gpkg, malikler.gpkg
    python3 scripts/fixtures/source_relations_files.py --check   # compares them with the ones on disk (rows, not bytes)

Everything is written with Python's sqlite3 from the GeoPackage 1.3 core tables and OGC 18-000's related-tables
extension (gpkgext_relations and a mapping table of base_id and related_id), not with KentOS code. A geometry is a
GeoPackage binary (header, then WKB) built with struct. The lasting ids are in KentOS's `kentos_uids` extension (docs/adr/0502),
fixed here so the cases can name them. The files are:

  iliskili.gpkg                      owners (an attribute table), parcels (polygons; `malikler`: the owners' ids joined with
                                     "; ") and buildings (polygons; `parsel`: the parcel's id): no relation in the file
  iliskili-eslenmis.gpkg             the same with KentOS's relation `kentos_references` between parcels and owners
  iliskili-eslenmis-sutunsuz.gpkg    the same, the parcels without a `malikler` column
  iliskili-yabanci.gpkg              a well-formed relation of another kind (`attributes`) between the same tables
  iliskili-bozuk.gpkg                a `kentos_references` whose mapping table lacks `related_id`
  iliskili-ters.gpkg                 a `kentos_references` the other way round (owners the base)
  malikler.gpkg                      the owners alone, for a relation across two files
"""
import hashlib
import sqlite3
import struct
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/geopackage/v1"
TIME = "2026-10-19T00:00:00.000Z"
SRS = 5256
WKT = ('PROJCS["TUREF / TM36",GEOGCS["TUREF",DATUM["Turkish_National_Reference_Frame",SPHEROID["GRS 1980",6378137,'
       '298.257222101]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]],PROJECTION["Transverse_Mercator"],'
       'PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",36],PARAMETER["scale_factor",1],'
       'PARAMETER["false_easting",500000],PARAMETER["false_northing",0],UNIT["metre",1],AUTHORITY["EPSG","5256"]]')
WGS84 = ('GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563,AUTHORITY["EPSG","7030"]],'
         'AUTHORITY["EPSG","6326"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,'
         'AUTHORITY["EPSG","9122"]],AXIS["Latitude",NORTH],AXIS["Longitude",EAST],AUTHORITY["EPSG","4326"]]')
RELATED = "gpkg_related_tables"
RELATED_DEFINITION = "http://www.opengis.net/doc/IS/gpkg-rte/1.0"

UID = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d{:04d}"
OWNERS = [  # fid, uid, ad, tur
    (1, UID.format(3001), "Ayşe Yıldız", "Gerçek"),
    (2, UID.format(3002), "Mehmet Demir", "Gerçek"),
    (3, UID.format(3003), "Sivas Belediyesi", "Tüzel"),
]
PARCELS = [  # fid, uid, ada, parsel no, the owners' fids
    (1, UID.format(2001), "101", "5", [1, 2]),
    (2, UID.format(2002), "101", "6", [2, 3]),
    (3, UID.format(2003), "102", "1", []),
]
BUILDINGS = [  # fid, uid, the parcel's fid, kat
    (1, UID.format(1001), 1, "2"),
    (2, UID.format(1002), 2, "1"),
]
X0, Y0 = 487000.0, 4420000.0


def owner_uid(fid):
    return next(o[1] for o in OWNERS if o[0] == fid)


def parcel_uid(fid):
    return next(p[1] for p in PARCELS if p[0] == fid)


def square(x, y, side):
    return [(x, y), (x + side, y), (x + side, y + side), (x, y + side), (x, y)]


def blob(ring):
    """A GeoPackage binary of a polygon: 'GP', version 0, flags (little endian, no envelope), the srs id, then WKB."""
    wkb = struct.pack("<BIII", 1, 3, 1, len(ring)) + b"".join(struct.pack("<dd", x, y) for x, y in ring)
    return b"GP" + bytes([0, 1]) + struct.pack("<i", SRS) + wkb


def parcel_ring(fid):
    return square(X0 + 30 * (fid - 1), Y0, 20.0)


def building_ring(fid):
    p = next(b[2] for b in BUILDINGS if b[0] == fid)
    return square(X0 + 30 * (p - 1) + 5, Y0 + 5, 10.0)


def ring_box(ring):
    xs = [p[0] for p in ring]
    ys = [p[1] for p in ring]
    return min(xs), min(ys), max(xs), max(ys)


def core(c):
    c.executescript(f"""
    CREATE TABLE gpkg_spatial_ref_sys (srs_name TEXT NOT NULL, srs_id INTEGER NOT NULL PRIMARY KEY,
      organization TEXT NOT NULL, organization_coordsys_id INTEGER NOT NULL, definition TEXT NOT NULL, description TEXT);
    CREATE TABLE gpkg_contents (table_name TEXT NOT NULL PRIMARY KEY, data_type TEXT NOT NULL, identifier TEXT UNIQUE,
      description TEXT DEFAULT '', last_change DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
      min_x DOUBLE, min_y DOUBLE, max_x DOUBLE, max_y DOUBLE, srs_id INTEGER,
      CONSTRAINT fk_gc_r_srs_id FOREIGN KEY (srs_id) REFERENCES gpkg_spatial_ref_sys(srs_id));
    CREATE TABLE gpkg_geometry_columns (table_name TEXT NOT NULL, column_name TEXT NOT NULL,
      geometry_type_name TEXT NOT NULL, srs_id INTEGER NOT NULL, z TINYINT NOT NULL, m TINYINT NOT NULL,
      CONSTRAINT pk_geom_cols PRIMARY KEY (table_name, column_name), CONSTRAINT uk_gc_table_name UNIQUE (table_name),
      CONSTRAINT fk_gc_tn FOREIGN KEY (table_name) REFERENCES gpkg_contents(table_name),
      CONSTRAINT fk_gc_srs FOREIGN KEY (srs_id) REFERENCES gpkg_spatial_ref_sys (srs_id));
    CREATE TABLE gpkg_extensions (table_name TEXT, column_name TEXT, extension_name TEXT NOT NULL,
      definition TEXT NOT NULL, scope TEXT NOT NULL, CONSTRAINT ge_tce UNIQUE (table_name, column_name, extension_name));
    """)
    c.executemany("INSERT INTO gpkg_spatial_ref_sys VALUES (?,?,?,?,?,?)", [
        ("Undefined cartesian SRS", -1, "NONE", -1, "undefined", "undefined cartesian coordinate reference system"),
        ("Undefined geographic SRS", 0, "NONE", 0, "undefined", "undefined geographic coordinate reference system"),
        ("WGS 84 geodetic", 4326, "EPSG", 4326, WGS84, "longitude/latitude coordinates in decimal degrees on the WGS 84 spheroid"),
        ("TUREF / TM36", SRS, "EPSG", SRS, WKT, None),
    ])


def contents(c, table, kind, box=None):
    c.execute("INSERT INTO gpkg_contents VALUES (?,?,?,?,?,?,?,?,?,?)",
              (table, kind, table, "", TIME, *(box or (None,) * 4), SRS if kind == "features" else None))
    if kind == "features":
        c.execute("INSERT INTO gpkg_geometry_columns VALUES (?,?,?,?,0,0)", (table, "geom", "POLYGON", SRS))


def uids(c, rows):
    """KentOS's lasting ids: (table, fid, uid)."""
    c.execute("CREATE TABLE kentos_uids (table_name TEXT NOT NULL, fid INTEGER NOT NULL, uid TEXT NOT NULL, "
              "PRIMARY KEY (table_name, fid))")
    c.execute("INSERT INTO gpkg_extensions VALUES ('kentos_uids', NULL, 'kentos_uids', "
              "'https://kentos.app/geopackage/uids (docs/adr/0502)', 'read-write')")
    c.executemany("INSERT INTO kentos_uids VALUES (?,?,?)", rows)


def owners_table(c):
    c.execute("CREATE TABLE malik (fid INTEGER PRIMARY KEY AUTOINCREMENT, ad TEXT, tur TEXT)")
    c.executemany("INSERT INTO malik VALUES (?,?,?)", [(f, ad, tur) for f, _, ad, tur in OWNERS])
    contents(c, "malik", "attributes")


def parcels_table(c, with_column=True):
    column = ", malikler TEXT" if with_column else ""
    c.execute(f"CREATE TABLE parsel (fid INTEGER PRIMARY KEY AUTOINCREMENT, geom POLYGON, ada TEXT, parsel_no TEXT{column})")
    for fid, _, ada, no, owners in PARCELS:
        values = [fid, blob(parcel_ring(fid)), ada, no]
        if with_column:
            values.append("; ".join(owner_uid(o) for o in owners) or None)
        c.execute(f"INSERT INTO parsel VALUES ({','.join('?' * len(values))})", values)
    xs = [ring_box(parcel_ring(p[0])) for p in PARCELS]
    contents(c, "parsel", "features", (min(b[0] for b in xs), min(b[1] for b in xs), max(b[2] for b in xs), max(b[3] for b in xs)))


def buildings_table(c):
    c.execute("CREATE TABLE bina (fid INTEGER PRIMARY KEY AUTOINCREMENT, geom POLYGON, parsel TEXT, kat TEXT)")
    for fid, _, parcel, kat in BUILDINGS:
        c.execute("INSERT INTO bina VALUES (?,?,?,?)", (fid, blob(building_ring(fid)), parcel_uid(parcel), kat))
    xs = [ring_box(building_ring(b[0])) for b in BUILDINGS]
    contents(c, "bina", "features", (min(b[0] for b in xs), min(b[1] for b in xs), max(b[2] for b in xs), max(b[3] for b in xs)))


def relations_table(c):
    c.execute("CREATE TABLE gpkgext_relations (id INTEGER PRIMARY KEY AUTOINCREMENT, base_table_name TEXT NOT NULL, "
              "base_primary_column TEXT NOT NULL DEFAULT 'id', related_table_name TEXT NOT NULL, "
              "related_primary_column TEXT NOT NULL DEFAULT 'id', relation_name TEXT NOT NULL, "
              "mapping_table_name TEXT NOT NULL UNIQUE)")
    c.execute("INSERT INTO gpkg_extensions VALUES ('gpkgext_relations', NULL, ?, ?, 'read-write')", (RELATED, RELATED_DEFINITION))


def relate(c, base, related, name, mapping, pairs, columns=("base_id", "related_id")):
    relations_table_exists = c.execute("SELECT count(*) FROM sqlite_master WHERE name = 'gpkgext_relations'").fetchone()[0]
    if not relations_table_exists:
        relations_table(c)
    c.execute(f"CREATE TABLE {mapping} ({columns[0]} INTEGER NOT NULL, {columns[1]} INTEGER NOT NULL)")
    c.executemany(f"INSERT INTO {mapping} VALUES (?,?)", pairs)
    c.execute("INSERT INTO gpkg_extensions VALUES (?, NULL, ?, ?, 'read-write')", (mapping, RELATED, RELATED_DEFINITION))
    c.execute("INSERT INTO gpkgext_relations (base_table_name, base_primary_column, related_table_name, "
              "related_primary_column, relation_name, mapping_table_name) VALUES (?, 'fid', ?, 'fid', ?, ?)",
              (base, related, name, mapping))


PAIRS = [(p[0], o) for p in PARCELS for o in p[4]]  # parcel fid, owner fid: the child's fid first


def make(name, owners=True, parcels=True, buildings=True, column=True, relation=None):
    path = DIR / name
    if path.exists():
        path.unlink()
    c = sqlite3.connect(path)
    c.execute("PRAGMA application_id = 1196444487")
    c.execute("PRAGMA user_version = 10400")
    core(c)
    if owners:
        owners_table(c)
    if parcels:
        parcels_table(c, column)
    if buildings:
        buildings_table(c)
    rows = []
    if owners:
        rows += [("malik", f, u) for f, u, _, _ in OWNERS]
    if parcels:
        rows += [("parsel", f, u) for f, u, _, _, _ in PARCELS]
    if buildings:
        rows += [("bina", f, u) for f, u, _, _ in BUILDINGS]
    uids(c, rows)
    if relation:
        relation(c)
    c.commit()
    c.execute("VACUUM")
    c.close()


FILES = {
    "iliskili.gpkg": dict(),
    "iliskili-eslenmis.gpkg": dict(
        relation=lambda c: relate(c, "parsel", "malik", "kentos_references", "parsel_malik", PAIRS)),
    "iliskili-eslenmis-sutunsuz.gpkg": dict(
        column=False, relation=lambda c: relate(c, "parsel", "malik", "kentos_references", "parsel_malik", PAIRS)),
    "iliskili-yabanci.gpkg": dict(
        relation=lambda c: relate(c, "parsel", "malik", "attributes", "parsel_malik_ek", PAIRS)),
    "iliskili-bozuk.gpkg": dict(
        relation=lambda c: relate(c, "parsel", "malik", "kentos_references", "parsel_malik", PAIRS,
                                  columns=("base_id", "malik_id"))),
    "iliskili-ters.gpkg": dict(
        relation=lambda c: relate(c, "malik", "parsel", "kentos_references", "malik_parsel",
                                  [(o, p) for p, o in PAIRS])),
    "malikler.gpkg": dict(parcels=False, buildings=False),
}


def sha(name):
    return hashlib.sha256((DIR / name).read_bytes()).hexdigest()


def content(path):
    """What a file holds, as comparable text: its tables' rows."""
    c = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    out = []
    for (t,) in c.execute("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name"):
        out.append((t, c.execute(f'SELECT * FROM "{t}" ORDER BY 1, 2').fetchall()))
    return out


def main():
    check = "--check" in sys.argv
    DIR.mkdir(parents=True, exist_ok=True)
    bad = []
    for name, kw in FILES.items():
        if check:
            tmp = DIR / f".{name}.check"
            real = DIR / name
            real_backup = real.read_bytes() if real.exists() else None
            make(f".{name}.check", **kw)
            same = real_backup is not None and content(tmp) == content(real)
            tmp.unlink()
            if not same:
                bad.append(name)
        else:
            make(name, **kw)
            print(f"{name}: {sha(name)}")
    if check and bad:
        print("differs:", ", ".join(bad))
        sys.exit(1)


if __name__ == "__main__":
    main()
