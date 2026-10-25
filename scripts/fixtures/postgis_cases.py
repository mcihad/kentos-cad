#!/usr/bin/env python3
"""A bound layer on PostGIS, end to end against a throwaway database (docs/adr/0505, plan/gis.md CBS-04).

    /usr/bin/python3 scripts/fixtures/postgis_cases.py [--screens] [--timing]

1. Starts postgis/postgis:18-3.6 in docker on a free port (never the developer's database on 5432) and loads QGIS's
   GeoPackage (fixtures/geopackage/v1/qgis.gpkg) and the WGS 84 one into the schema `kadastro` with GDAL's ogr2ogr,
   the fids kept as primary keys, as a municipality would load its data.
2. Runs crates/native/postgis's ignored test with KENTOS_PG_URL (no password in it: PGPASSWORD), which binds
   kadastro.parsel through cad.sources.edit, edits it and writes it back, binds it again in a fresh drawing, writes
   from there as another user, and checks the conflict and the reload; then the desktop's PostGIS'e bağlan window
   lists and binds the same database (with --screens its pictures, .run/shots/postgis-*).
3. Checks, with psycopg2 and no KentOS code: the tables KentOS described (names, row counts, geometry types, systems,
   columns) against the catalogue; the objects it took against the rows; and the table after the writes: parcel 1
   renamed with a new `durum` column then deleted by the second drawing, parcel 2 deleted, the triangle inserted with
   its lasting id in kadastro.kentos_uids and an area of 50 m².
4. Stops the container.
"""

import json
import os
import socket
import sqlite3
import subprocess
import sys
import tempfile
import time
from pathlib import Path

import psycopg2
import uuid
from osgeo import gdal

gdal.UseExceptions()
ROOT = Path(__file__).resolve().parents[2]
IMAGE = "postgis/postgis:18-3.6"
PASSWORD = "kentos-gecici"


def free_port():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def wait(port):
    for _ in range(120):
        try:
            c = psycopg2.connect(host="127.0.0.1", port=port, user="postgres", password=PASSWORD, dbname="postgres")
            c.autocommit = True
            c.cursor().execute("SELECT postgis_version()")
            return c
        except Exception:
            time.sleep(1)
    raise SystemExit("geçici PostGIS açılmadı")


def load(port):
    pg = f"PG:host=127.0.0.1 port={port} user=postgres password={PASSWORD} dbname=postgres"
    for name in ("qgis.gpkg", "wgs84.gpkg"):
        gdal.VectorTranslate(pg, str(ROOT / "fixtures/geopackage/v1" / name),
                             options=gdal.VectorTranslateOptions(
                                 layerCreationOptions=["SCHEMA=kadastro", "FID=fid", "GEOMETRY_NAME=geom"],
                                 preserveFID=True))


def catalogue(c):
    cur = c.cursor()
    cur.execute("SELECT f_table_name, type, srid FROM geometry_columns WHERE f_table_schema = 'kadastro'")
    geoms = {r[0]: (r[1], r[2]) for r in cur.fetchall()}
    cur.execute("SELECT table_name FROM information_schema.tables WHERE table_schema = 'kadastro' "
                "AND table_name <> 'kentos_uids'")
    out = {}
    for (t,) in cur.fetchall():
        cur.execute(f'SELECT count(*) FROM kadastro."{t}"')
        rows = cur.fetchone()[0]
        cur.execute("SELECT column_name FROM information_schema.columns WHERE table_schema = 'kadastro' "
                    "AND table_name = %s ORDER BY ordinal_position", (t,))
        cols = [r[0] for r in cur.fetchall() if r[0] not in ("fid", "geom")]
        g = geoms.get(t)
        # The box the statistics estimate (docs/adr/0599 §2): analysed first, read as the describe reads it.
        extent = None
        if g:
            cur.execute(f'ANALYZE kadastro."{t}"')
            cur.execute("SELECT ST_XMin(e), ST_YMin(e), ST_XMax(e), ST_YMax(e) "
                        "FROM (SELECT ST_EstimatedExtent('kadastro', %s, 'geom')::box3d AS e) s", (t,))
            row = cur.fetchone()
            if row and None not in row:
                extent = [float(v) for v in row]
        out[f"kadastro.{t}"] = {"rows": rows, "geometry": g[0] if g else None, "srid": g[1] if g else None,
                                "columns": cols, "extent": extent}
    return out


def package_problems(c, package):
    """Paketle's database copies (docs/adr/0596 §2) read with OGR against the database: the part's points are the
    database's points in the link's area, the parcels are the database's (one changed after the package was made, so
    compared without its note), each copy keeps its rows' lasting ids, the project file and paket.json are there."""
    from osgeo import ogr

    problems = []
    if not (package / "saha.kcad").is_file() or not (package / "paket.json").is_file():
        return [f"paket {package} yok ya da eksik"]
    cur = c.cursor()
    cur.execute("SELECT ad, ST_X(geom), ST_Y(geom) FROM paket.olay "
                "WHERE geom && ST_MakeEnvelope(486995, 4419995, 487595, 4420015, 5256)")
    # A point without a name (the order cases' empty ones) sorts as an empty name on both sides.
    want = sorted((ad or "", round(x, 6), round(y, 6)) for ad, x, y in cur.fetchall())
    src = ogr.Open(str(package / "veri/paket-olay.gpkg"))
    got = sorted((f.GetField("ad") or "", round(f.GetGeometryRef().GetX(), 6), round(f.GetGeometryRef().GetY(), 6))
                 for f in src.GetLayerByName("olay"))
    if got != want or not got:
        problems.append(f"paketteki olay kopyası {len(got)} nokta, veritabanının alanında {len(want)}")
    points = len(want)
    cur.execute("SELECT kimlik, CASE WHEN geom IS NULL OR ST_IsEmpty(geom) THEN NULL ELSE ST_Area(geom) END FROM paket.parsel")
    # A parcel without geometry or with an empty one (QGIS's file has one) is a record without geometry in KentOS.
    want = sorted((str(k), -1.0 if a is None else round(a, 6)) for k, a in cur.fetchall())
    src = ogr.Open(str(package / "veri/paket-parsel.gpkg"))
    got = sorted((str(f.GetField("kimlik")),
                  -1.0 if f.GetGeometryRef() is None else round(f.GetGeometryRef().GetLinearGeometry().GetArea(), 6))
                 for f in src.GetLayerByName("parsel"))
    if [k for k, _ in got] != [k for k, _ in want] or any(abs(a - b) > 1e-3 for (_, a), (_, b) in zip(got, want)):
        problems.append(f"paketteki parsel kopyası {got}, veritabanı {want}")
    for name, rows in (("paket-olay", points), ("paket-parsel", len(want))):
        copy = ogr.Open(str(package / f"veri/{name}.gpkg"))
        counted = copy.ExecuteSQL("SELECT count(*) FROM kentos_uids")
        n = counted.GetNextFeature().GetField(0)
        copy.ReleaseResultSet(counted)
        if n != rows:
            problems.append(f"{name} kopyasının kalıcı kimlikleri {n}")
    return problems


def lineage_problems(c):
    """The lineage the split test left (docs/adr/0598 §3): one row, its child and its parent rows of soy.parsel, both
    named by soy.kentos_uids, the operation split."""
    cur = c.cursor()
    cur.execute("SELECT table_name, child::text, parent::text, operation, step FROM soy.kentos_lineage")
    rows = cur.fetchall()
    cur.execute("SELECT p.fid, u.uid::text FROM soy.parsel p LEFT JOIN soy.kentos_uids u "
                "ON u.table_name = 'soy.parsel' AND u.pk = p.fid")
    # A row KentOS did not write has the id ADR 0505 gives it: UUID v5 of the database, the table and the key.
    space = uuid.UUID(bytes=bytes([0x6b, 0x1e, 0x4f, 0x0a, 0x93, 0x27, 0x4c, 0x51, 0x8d, 0x36, 0x2f, 0x7a, 0x10, 0xc4,
                                   0x5e, 0x92]))
    named = {u or str(uuid.uuid5(space, f"pg:postgres/soy.parsel:{fid}")) for fid, u in cur.fetchall()}
    out = []
    if len(rows) != 1:
        out.append(f"soy.kentos_lineage: {len(rows)} satır, 1 bekleniyor")
    for table, child, parent, operation, step in rows:
        if table != "soy.parsel" or operation != "split" or step != "Alan böl":
            out.append(f"soy.kentos_lineage: beklenmeyen satır {table} {operation} {step}")
        if child not in named:
            out.append(f"soy.kentos_lineage: {child} tablonun bir satırı değil")
        if parent not in named:
            out.append(f"soy.kentos_lineage: ata {parent} tablonun bir satırı değil")
    return out


def check(c, seen, before):
    problems = []
    described = {t["name"]: t for t in seen["tables"] if t["name"].startswith("kadastro.")}
    for name, want in before.items():
        got = described.get(name)
        if got is None:
            problems.append(f"{name} listelenmedi")
            continue
        if got["rows"] != want["rows"] or got.get("geometry") != want["geometry"] or got.get("srid") != want["srid"]:
            problems.append(f"{name}: {got['rows']}, {got.get('geometry')}, {got.get('srid')} ≠ {want}")
        if got.get("extent") != want["extent"]:
            problems.append(f"{name} kutusu {got.get('extent')} ≠ {want['extent']}")
        if want["geometry"] and want["extent"] is None:
            problems.append(f"{name}: ANALYZE'dan sonra ST_EstimatedExtent kutu vermedi")
        if [col["name"] for col in got["columns"]] != want["columns"]:
            problems.append(f"{name} sütunları {[col['name'] for col in got['columns']]} ≠ {want['columns']}")
    if len(seen["objects"]) != before["kadastro.parsel"]["rows"]:
        problems.append(f"{len(seen['objects'])} nesne, {before['kadastro.parsel']['rows']} satır")
    cur = c.cursor()
    cur.execute("SELECT fid, ada, durum, ST_Area(geom) FROM kadastro.parsel ORDER BY fid")
    rows = cur.fetchall()
    fids = [r[0] for r in rows]
    if 1 in fids or 2 in fids or len(rows) != before["kadastro.parsel"]["rows"] - 1:
        problems.append(f"parsel satırları {fids}")
    new = [r for r in rows if r[0] not in (1, 2, 3)]
    if len(new) != 1 or abs(new[0][3] - 50.0) > 1e-9:
        problems.append(f"eklenen üçgen {new}")
    else:
        cur.execute("SELECT uid::text FROM kadastro.kentos_uids WHERE table_name = 'kadastro.parsel' AND pk = %s",
                    (new[0][0],))
        uid = [r[0] for r in cur.fetchall()]
        if uid != [seen["drawn"]]:
            problems.append(f"üçgenin kalıcı kimliği {uid} ≠ {seen['drawn']}")
    # Written together, then refused together (docs/adr/0584 §1); the Z column kept its elevations (§2).
    cur.execute("SELECT (SELECT ad FROM boyut.duz), (SELECT ad FROM boyut.cizgi), "
                "(SELECT ST_ZMax(geom) FROM boyut.kotlu)")
    together = cur.fetchone()
    if together != ("birlikte", "baskasi", 101.0):
        problems.append(f"birlikte yazılan tablolar {together} ≠ ('birlikte', 'baskasi', 101.0)")
    # The neighbours (docs/adr/0585): B and C followed A's corners in the parcels; row security kept B of the copy.
    cur.execute("SELECT ad, ST_AsText(geom) FROM komsu.parsel ORDER BY fid")
    parcels = dict(cur.fetchall())
    if "487011 4420004" not in parcels.get("B", "") or "487010 4420007" not in parcels.get("C2", ""):
        problems.append(f"komşular izlemedi: {parcels}")
    cur.execute("SELECT ad, ST_AsText(geom) FROM komsu.kisitli ORDER BY fid")
    narrow = dict(cur.fetchall())
    if "487011 4420004" not in narrow.get("A", "") or "487010 4420004" not in narrow.get("B", ""):
        problems.append(f"satır güvenliğinin tablosu: {narrow}")
    # The junction table (docs/adr/0590): its rows in the references' order, its mark, its cascading foreign keys;
    # someone else's junction untouched and the reference beside it written as text.
    cur.execute("SELECT base_id, related_id, position FROM iliski.parsel_malik ORDER BY base_id, position")
    junction = cur.fetchall()
    cur.execute("SELECT fid FROM iliski.malik WHERE ad = 'Yeni Malik'")
    added_owner = [r[0] for r in cur.fetchall()]
    if junction[:3] != [(1, 2, 0), (2, 3, 0), (2, 2, 1)] or len(junction) != 5 or len(added_owner) != 1 \
            or junction[3] != (3, added_owner[0], 0) or junction[4][1:] != (1, 0) or junction[4][0] <= 3:
        problems.append(f"bağlantı tablosu {junction}, yeni malik {added_owner}")
    cur.execute("SELECT obj_description('iliski.parsel_malik'::regclass, 'pg_class')")
    mark = cur.fetchone()[0]
    if mark != "kentos_references base=iliski.parsel related=iliski.malik":
        problems.append(f"bağlantı tablosunun işareti {mark!r}")
    cur.execute("SELECT a.attname, k.confrelid::regclass::text, k.confdeltype FROM pg_constraint k "
                "JOIN pg_attribute a ON a.attrelid = k.conrelid AND a.attnum = k.conkey[1] "
                "WHERE k.conrelid = 'iliski.parsel_malik'::regclass AND k.contype = 'f' ORDER BY 1")
    keys = cur.fetchall()
    if keys != [("base_id", "iliski.parsel", "c"), ("related_id", "iliski.malik", "c")]:
        problems.append(f"bağlantı tablosunun yabancı anahtarları {keys}")
    cur.execute("BEGIN")
    cur.execute("DELETE FROM iliski.malik WHERE fid = 3")
    cur.execute("SELECT count(*) FROM iliski.parsel_malik WHERE related_id = 3")
    if cur.fetchone()[0] != 0:
        problems.append("silinen malikin bağlantı satırları kalmış")
    cur.execute("ROLLBACK")
    cur.execute("SELECT count(*) FROM iliski2.sahiplik")
    if cur.fetchone()[0] != 2:
        problems.append("başkasının bağlantı tablosuna dokunulmuş")
    cur.execute("SELECT malikler FROM iliski2.parsel WHERE fid = 3")
    if cur.fetchone()[0] != "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001":
        problems.append("başka temsilli ilişkide başvuru metin olarak yazılmamış")
    # İşlemler's result (docs/adr/0592 §2): a new schema's table of the squares' centres, its key the fid, the lasting
    # ids beside it, the run's lineage its comment; made again by replaceTables, so one table of 12 rows.
    cur.execute("SELECT format_type(atttypid, atttypmod) FROM pg_attribute WHERE attrelid = 'sonuc.\"Merkezler\"'::regclass "
                "AND attname = 'geom'")
    kind = [r[0] for r in cur.fetchall()]
    if kind != ["geometry(Point,5256)"]:
        problems.append(f"sonucun geometri sütunu {kind}")
    cur.execute('SELECT ad, kat, ST_X(geom), ST_Y(geom) FROM sonuc."Merkezler" ORDER BY fid')
    centres = cur.fetchall()
    sq = sqlite3.connect(f"file:{ROOT / 'fixtures/sources/v1/sorgu.gpkg'}?mode=ro", uri=True)
    squares = {}
    for ad, kat, ring in sq.execute("SELECT ad, kat, geom FROM bina"):
        squares[ad] = kat
    sq.close()
    x0, y0 = 487000.0, 4420000.0
    want = sorted((f"B{k + 1}", str(k % 4 + 1), x0 + k * 30 + (6 + k) / 2, y0 + 600 + (6 + k) / 2) for k in range(12))
    if sorted(centres) != want or any(squares[ad] != int(kat) for ad, kat, _, _ in centres):
        problems.append(f"sonucun noktaları {centres}")
    cur.execute("SELECT count(*) FROM sonuc.kentos_uids WHERE table_name = 'sonuc.Merkezler'")
    if cur.fetchone()[0] != 12:
        problems.append("sonucun kalıcı kimlikleri 12 değil")
    cur.execute("SELECT obj_description('sonuc.\"Merkezler\"'::regclass, 'pg_class')")
    lineage = json.loads(cur.fetchone()[0] or "{}")
    if lineage.get("format") != "kentos.lineage" or lineage.get("table") != "sonuc.Merkezler" \
            or lineage.get("tool", {}).get("id") != "convert.toPoints" or lineage.get("inputs", [{}])[0].get("rows") != 12:
        problems.append(f"sonucun soyu {lineage}")
    # A batch reading that table back, a row for each filter, to a schema of its own (docs/adr/0593 §1): the points
    # where the centres are, split by the filters, their lasting ids, a lineage that is the batch's.
    for table, keep in [("Yüksek", lambda k: k >= 3), ("Alçak", lambda k: k < 3)]:
        cur.execute(f'SELECT ad, kat, ST_X(geom), ST_Y(geom) FROM toplu."{table}" ORDER BY ad')
        got = sorted(cur.fetchall())
        if got != sorted(c_ for c_ in centres if keep(int(c_[1]))) or len(got) != 6:
            problems.append(f"toplu işin {table} tablosu {got}")
        cur.execute(f"SELECT obj_description('toplu.\"{table}\"'::regclass, 'pg_class')")
        lineage = json.loads(cur.fetchone()[0] or "{}")
        if lineage.get("run", {}).get("kind") != "batch" or len(lineage.get("request", {}).get("rows", [])) != 2:
            problems.append(f"toplu işin {table} tablosunun soyu {lineage}")
    cur.execute("SELECT count(*) FROM toplu.kentos_uids")
    if cur.fetchone()[0] != 12:
        problems.append("toplu işin kalıcı kimlikleri 12 değil")
    cur.execute("SELECT data_type FROM information_schema.columns WHERE table_schema = 'kadastro' "
                "AND table_name = 'parsel' AND column_name = 'durum'")
    if [r[0] for r in cur.fetchall()] != ["text"]:
        problems.append("durum sütunu yok ya da metin değil")
    return problems


def derivation_problems(c):
    """The derived dataset the Rust test wrote to the schema turev and then detached (docs/adr/0634 §3): six tables,
    each comment a detach note keeping its derivation record (format, version, table as schema.table, the source drawing),
    no lasting id left; the rows the drawing's (17 of them, the road renamed by the GIS)."""
    problems = []
    cur = c.cursor()
    cur.execute("SELECT c.relname, obj_description(c.oid, 'pg_class') FROM pg_class c JOIN pg_namespace n "
                "ON n.oid = c.relnamespace WHERE n.nspname = 'turev' AND c.relkind = 'r' AND c.relname <> 'kentos_uids' "
                "ORDER BY c.relname")
    tables = cur.fetchall()
    if len(tables) != 6:
        return [f"turev şemasında {len(tables)} tablo var, 6 değil"]
    rows = 0
    for name, comment in tables:
        try:
            note = json.loads(comment or "")
        except json.JSONDecodeError:
            problems.append(f"turev.{name}: yorum JSON değil")
            continue
        rec = note.get("derivation", {})
        if note.get("format") != "kentos.derivation-detached" or rec.get("format") != "kentos.derivation" \
                or rec.get("table") != f"turev.{name}" or rec.get("source", {}).get("name") != "Ada 101 türevi":
            problems.append(f"turev.{name}: not {note}")
        cur.execute(f'SELECT count(*) FROM turev."{name}"')
        rows += cur.fetchone()[0]
    if rows != 17:
        problems.append(f"turev'de {rows} satır, 17 değil")
    cur.execute("SELECT count(*) FROM turev.kentos_uids")
    if cur.fetchone()[0] != 0:
        problems.append("turev.kentos_uids boşalmadı")
    cur.execute('SELECT count(*) FROM turev."Yol" WHERE ad = \'E-200\'')
    if cur.fetchone()[0] != 1:
        problems.append("turev.Yol: GIS'in değişikliği yok")
    return problems


def main():
    port = free_port()
    name = f"kentos-postgis-{os.getpid()}"
    subprocess.run(["docker", "run", "-d", "--rm", "--name", name, "-p", f"127.0.0.1:{port}:5432",
                    "-e", f"POSTGRES_PASSWORD={PASSWORD}", IMAGE], check=True, stdout=subprocess.DEVNULL)
    print(f"geçici veritabanı: docker {name}, 127.0.0.1:{port} (5432'deki veritabanına dokunulmuyor)")
    try:
        c = wait(port)
        c.cursor().execute("CREATE SCHEMA kadastro")
        load(port)
        # The query cases' file (docs/adr/0581): its tables in the schema sorgu; the one text in nokta's number
        # column cannot be a number in PostgreSQL and is made NULL (the shared questions leave it out).
        c.cursor().execute("CREATE SCHEMA sorgu")
        pg = f"PG:host=127.0.0.1 port={port} user=postgres password={PASSWORD} dbname=postgres"
        for table in ("nokta", "bina", "kayit"):
            gdal.VectorTranslate(pg, str(ROOT / "fixtures/sources/v1/sorgu.gpkg"),
                                 options=gdal.VectorTranslateOptions(
                                     layers=[table],
                                     layerCreationOptions=["SCHEMA=sorgu", "FID=fid", "GEOMETRY_NAME=geom"],
                                     preserveFID=True))
        c.cursor().execute("UPDATE sorgu.nokta SET kot = NULL WHERE fid = 60")
        # The order cases' file (docs/adr/0591): its table in the schema sira, the date a date, the true/false a
        # boolean.
        c.cursor().execute("CREATE SCHEMA sira")
        gdal.VectorTranslate(pg, str(ROOT / "fixtures/sources/v1/sira.gpkg"),
                             options=gdal.VectorTranslateOptions(
                                 layers=["olay"],
                                 layerCreationOptions=["SCHEMA=sira", "FID=fid", "GEOMETRY_NAME=geom"],
                                 preserveFID=True))
        # The time cases' file (docs/adr/0595): its tables in the schema zaman with the dates as texts, as written,
        # and `surum` once more with date and timestamptz columns (a text no date can hold made NULL).
        # (GDAL would write a date as text 2021/03/01: the texts are copied as stored, by sqlite3 and psycopg2.)
        c.cursor().execute("CREATE SCHEMA zaman")
        z = sqlite3.connect(f"file:{ROOT / 'fixtures/sources/v1/zaman.gpkg'}?mode=ro", uri=True)
        typed = [r for r in z.execute("SELECT fid, ad, bas, bit, kayit, silindi FROM surum ORDER BY fid")]
        events = [r for r in z.execute("SELECT fid, ad, zaman FROM olay ORDER BY fid")]
        z.close()
        cur = c.cursor()
        cur.execute("CREATE TABLE zaman.surum (fid bigint PRIMARY KEY, ad text, bas text, bit text, kayit text, "
                    "silindi text, geom geometry(Point, 5256))")
        for fid, ad, bas, bit, kayit, silindi in typed:
            cur.execute("INSERT INTO zaman.surum VALUES (%s, %s, %s, %s, %s, %s, ST_SetSRID(ST_MakePoint(%s, %s), 5256))",
                        (fid, ad, bas, bit, kayit, silindi, 487000.0 + fid * 20, 4420000.0))
        cur.execute("CREATE TABLE zaman.olay (fid bigint PRIMARY KEY, ad text, zaman text, geom geometry(Point, 5256))")
        for fid, ad, zaman in events:
            cur.execute("INSERT INTO zaman.olay VALUES (%s, %s, %s, ST_SetSRID(ST_MakePoint(%s, %s), 5256))",
                        (fid, ad, zaman, 487000.0 + fid * 20, 4420100.0))
        cur.execute("CREATE TABLE zaman.surum_tipli (fid bigint PRIMARY KEY, ad text, bas date, bit date, "
                    "kayit timestamptz, silindi timestamptz, geom geometry(Point, 5256))")

        def as_date(v):
            return v if v and len(v) == 10 and v[4] == "-" else None

        for fid, ad, bas, bit, kayit, silindi in typed:
            cur.execute("INSERT INTO zaman.surum_tipli VALUES (%s, %s, %s, %s, %s, %s, "
                        "ST_SetSRID(ST_MakePoint(%s, %s), 5256))",
                        (fid, ad, as_date(bas), as_date(bit), kayit, silindi, 487000.0 + fid * 20, 4420000.0))
        # A view keyed by fid and one without a key (docs/adr/0582 §3).
        # Paketle's copies (docs/adr/0596 §2): the order cases' points and QGIS's parcels in the schema paket, written by
        # the package test only once read (a parcel changed after the package is made).
        c.cursor().execute("CREATE SCHEMA paket")
        for source, table in ((ROOT / "fixtures/sources/v1/sira.gpkg", "olay"), (ROOT / "fixtures/geopackage/v1/qgis.gpkg", "parsel")):
            gdal.VectorTranslate(pg, str(source),
                                 options=gdal.VectorTranslateOptions(
                                     layers=[table],
                                     layerCreationOptions=["SCHEMA=paket", "FID=fid", "GEOMETRY_NAME=geom"],
                                     preserveFID=True))
        # The lineage of a split parcel (docs/adr/0598 §3): QGIS's parcels in the schema soy, written by the lineage test.
        c.cursor().execute("CREATE SCHEMA soy")
        gdal.VectorTranslate(pg, str(ROOT / "fixtures/geopackage/v1/qgis.gpkg"),
                             options=gdal.VectorTranslateOptions(
                                 layers=["parsel"],
                                 layerCreationOptions=["SCHEMA=soy", "FID=fid", "GEOMETRY_NAME=geom"],
                                 preserveFID=True))
        c.cursor().execute("CREATE VIEW sorgu.nokta_v AS SELECT fid, ad, tur, geom FROM sorgu.nokta WHERE fid <= 50")
        c.cursor().execute("CREATE VIEW sorgu.anahtarsiz AS SELECT ad, geom FROM sorgu.nokta")
        # The tables written together and the Z and M columns (docs/adr/0584).
        c.cursor().execute("CREATE SCHEMA boyut")
        for table in ("duz", "cizgi", "kotlu"):
            gdal.VectorTranslate(pg, str(ROOT / "fixtures/geopackage/v1/boyut.gpkg"),
                                 options=gdal.VectorTranslateOptions(
                                     layers=[table],
                                     layerCreationOptions=["SCHEMA=boyut", "FID=fid", "GEOMETRY_NAME=geom"],
                                     preserveFID=True))
        # The neighbours outside a part's area (docs/adr/0585), and a copy whose row security lets the account
        # kisitli change every row but B.
        cur = c.cursor()
        cur.execute("CREATE SCHEMA komsu")
        gdal.VectorTranslate(pg, str(ROOT / "fixtures/geopackage/v1/komsu-gecerlilik.gpkg"),
                             options=gdal.VectorTranslateOptions(
                                 layers=["parsel"],
                                 layerCreationOptions=["SCHEMA=komsu", "FID=fid", "GEOMETRY_NAME=geom"],
                                 preserveFID=True))
        cur.execute("CREATE TABLE komsu.kisitli AS SELECT * FROM komsu.parsel")
        cur.execute("ALTER TABLE komsu.kisitli ADD PRIMARY KEY (fid)")
        cur.execute("CREATE INDEX ON komsu.kisitli USING gist (geom)")
        cur.execute(f"CREATE ROLE kisitli LOGIN PASSWORD '{PASSWORD}'")
        cur.execute("GRANT USAGE ON SCHEMA komsu TO kisitli")
        cur.execute("GRANT SELECT, INSERT, UPDATE, DELETE ON komsu.kisitli TO kisitli")
        cur.execute("ALTER TABLE komsu.kisitli ENABLE ROW LEVEL SECURITY")
        cur.execute("CREATE POLICY gor ON komsu.kisitli FOR SELECT USING (true)")
        cur.execute("CREATE POLICY degistir ON komsu.kisitli FOR UPDATE USING (ad <> 'B')")
        # Many-to-many references (docs/adr/0590): the owners, parcels and buildings of iliskili.gpkg with their lasting
        # ids, and in iliski2 the same owners and parcels joined by someone else's junction table.
        for schema in ("iliski", "iliski2"):
            cur.execute(f"CREATE SCHEMA {schema}")
            for table in ("malik", "parsel", "bina") if schema == "iliski" else ("malik", "parsel"):
                gdal.VectorTranslate(pg, str(ROOT / "fixtures/geopackage/v1/iliskili.gpkg"),
                                     options=gdal.VectorTranslateOptions(
                                         layers=[table],
                                         layerCreationOptions=[f"SCHEMA={schema}", "FID=fid", "GEOMETRY_NAME=geom"],
                                         preserveFID=True))
            cur.execute(f"CREATE TABLE {schema}.kentos_uids (table_name text NOT NULL, pk bigint NOT NULL, "
                        "uid uuid NOT NULL, PRIMARY KEY (table_name, pk))")
            gp = sqlite3.connect(str(ROOT / "fixtures/geopackage/v1/iliskili.gpkg"))
            for table, pk, uid in gp.execute("SELECT table_name, fid, uid FROM kentos_uids"):
                if schema == "iliski" or table != "bina":
                    cur.execute(f"INSERT INTO {schema}.kentos_uids VALUES (%s, %s, %s)", (f"{schema}.{table}", pk, uid))
            gp.close()
        # The field kinds of docs/adr/0629 §4 from their PostgreSQL types.
        cur.execute("CREATE SCHEMA turler")
        cur.execute("CREATE TABLE turler.olcum (fid bigint PRIMARY KEY, kimlik bigint, an timestamptz, cihaz uuid, "
                    "ek jsonb, imza bytea, tutar numeric(40, 10), geom geometry(Point, 5256))")
        cur.execute("INSERT INTO turler.olcum VALUES (1, 9007199254740993, '2024-03-01T10:20:30.5+03:00', "
                    "'0192F5A0-7C3E-7D4A-9B1E-4C2F8A6D0001', '{\"not\": \"ç\", \"kot\": [1.25, 12345678901234567890]}', "
                    "'\\x414243', 12345678901234567890.1234567890, ST_SetSRID(ST_MakePoint(487010, 4420010), 5256)), "
                    "(2, -9223372036854775808, '2024-03-01T07:20:30Z', NULL, '[]', NULL, NULL, "
                    "ST_SetSRID(ST_MakePoint(487020, 4420020), 5256))")
        cur.execute("CREATE TABLE iliski2.sahiplik (parsel bigint NOT NULL REFERENCES iliski2.parsel (fid), "
                    "malik bigint NOT NULL REFERENCES iliski2.malik (fid))")
        cur.execute("INSERT INTO iliski2.sahiplik VALUES (1, 1), (1, 2)")
        before = catalogue(c)
        out = Path(tempfile.mkstemp(suffix=".json")[1])
        package = Path(tempfile.mkdtemp()) / "paket"
        env = dict(os.environ, KENTOS_PG_URL=f"postgresql://postgres@127.0.0.1:{port}/postgres",
                   PGPASSWORD=PASSWORD, KENTOS_PG_OUT=str(out), KENTOS_PG_PACKAGE=str(package), CARGO_BUILD_JOBS="2")
        only = sys.argv[sys.argv.index("--only") + 1] if "--only" in sys.argv else None
        test = subprocess.run(["cargo", "test", "-q", "-p", "kentos-postgis", "--test", "all", *([only] if only else []),
                               "--", "--ignored", "--nocapture"], cwd=ROOT, env=env)
        if test.returncode != 0:
            print("Rust testi düştü", file=sys.stderr)
            return 1
        derived = derivation_problems(c)
        if only:
            # One test by its name (docs/adr/0634 §3: `--only a_drawing_goes_to_postgis`): its own checks alone.
            if derived and "a_drawing_goes" in only:
                print("; ".join(derived), file=sys.stderr)
                return 1
            print(f"PostGIS: yalnız {only} koştu, denetimleri tutarlı")
            return 0
        # The desktop's window on the same database: listing and binding (reads only).
        desk = subprocess.run(["cargo", "test", "-q", "-p", "kentos-desktop", "a_real_database_is_listed_and_bound",
                               "--", "--ignored"], cwd=ROOT, env=env)
        if desk.returncode != 0:
            print("masaüstünün PostGIS penceresi düştü", file=sys.stderr)
            return 1
        if "--timing" in sys.argv:
            # A million parcels for docs/adr/0582 §5, made by the database itself.
            cur = c.cursor()
            cur.execute("CREATE SCHEMA zaman")
            cur.execute("CREATE TABLE zaman.parsel (fid bigint PRIMARY KEY, ada integer, parsel integer, ad text, "
                        "geom geometry(Polygon, 5256))")
            cur.execute("INSERT INTO zaman.parsel SELECT g, g / 1000 + 1, g % 1000 + 1, 'Parsel ' || g, "
                        "ST_MakeEnvelope(500000 + (g % 1000) * 20, 4400000 + (g / 1000) * 20, "
                        "500020 + (g % 1000) * 20, 4400020 + (g / 1000) * 20, 5256) "
                        "FROM generate_series(0, 999999) AS g")
            cur.execute("CREATE INDEX ON zaman.parsel USING gist (geom)")
            cur.execute("ANALYZE zaman.parsel")
            timing = subprocess.run(["cargo", "test", "--release", "-q", "-p", "kentos-postgis", "--test", "all",
                                     "postgis_timing", "--", "--ignored", "--nocapture"],
                                    cwd=ROOT, env=dict(env, KENTOS_PG_TIMING="1"))
            if timing.returncode != 0:
                print("PostGIS ölçümü düştü", file=sys.stderr)
                return 1
        # KentOS and QGIS 4.2 on the same table (docs/adr/0582 §2), with the Python SDK's interpreter.
        qgis = subprocess.run([str(ROOT / ".run/py/bin/python"), str(ROOT / "scripts/fixtures/postgis_qgis_cases.py")],
                              cwd=ROOT, env=dict(env, KENTOS_PG_PORT=str(port)))
        if qgis.returncode != 0:
            print("QGIS ile eşzamanlı düzenleme düştü", file=sys.stderr)
            return 1
        if "--screens" in sys.argv:
            subprocess.run(["cargo", "test", "-q", "-p", "kentos-desktop", "exchange::postgis_tests::screens", "--",
                            "--ignored", "--nocapture"], cwd=ROOT, env=env, check=True)
        seen = json.loads(out.read_text("utf-8"))
        problems = check(c, seen, before) + package_problems(c, package) + lineage_problems(c) + derived
        if problems:
            print("; ".join(problems), file=sys.stderr)
            return 1
        print(f"PostGIS durumları tutarlı: {len(before)} tablo KentOS'un söylediği gibi; parsel yazıldı, "
              "kalıcı kimlik kentos_uids'te, çakışma ve geri okuma doğru")
        return 0
    finally:
        subprocess.run(["docker", "stop", name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


if __name__ == "__main__":
    sys.exit(main())
