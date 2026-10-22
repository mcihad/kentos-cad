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
        out[f"kadastro.{t}"] = {"rows": rows, "geometry": g[0] if g else None, "srid": g[1] if g else None,
                                "columns": cols}
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
    cur.execute("SELECT data_type FROM information_schema.columns WHERE table_schema = 'kadastro' "
                "AND table_name = 'parsel' AND column_name = 'durum'")
    if [r[0] for r in cur.fetchall()] != ["text"]:
        problems.append("durum sütunu yok ya da metin değil")
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
        # A view keyed by fid and one without a key (docs/adr/0582 §3).
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
        cur.execute("CREATE TABLE iliski2.sahiplik (parsel bigint NOT NULL REFERENCES iliski2.parsel (fid), "
                    "malik bigint NOT NULL REFERENCES iliski2.malik (fid))")
        cur.execute("INSERT INTO iliski2.sahiplik VALUES (1, 1), (1, 2)")
        before = catalogue(c)
        out = Path(tempfile.mkstemp(suffix=".json")[1])
        env = dict(os.environ, KENTOS_PG_URL=f"postgresql://postgres@127.0.0.1:{port}/postgres",
                   PGPASSWORD=PASSWORD, KENTOS_PG_OUT=str(out), CARGO_BUILD_JOBS="2")
        test = subprocess.run(["cargo", "test", "-q", "-p", "kentos-postgis", "--test", "all", "--", "--ignored",
                               "--nocapture"], cwd=ROOT, env=env)
        if test.returncode != 0:
            print("Rust testi düştü", file=sys.stderr)
            return 1
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
        problems = check(c, seen, before)
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
