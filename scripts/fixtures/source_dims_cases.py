#!/usr/bin/env python3
"""The shared cases of Z and M on a write to a bound table (docs/adr/0584 §2, plan/gis.md CBS-11).

    /usr/bin/python3 scripts/fixtures/source_dims_cases.py --files   # GDAL writes fixtures/geopackage/v1/boyut.gpkg
    python3 scripts/fixtures/source_dims_cases.py                    # writes fixtures/commands/v1/cad.sources.sync.dims.json
    python3 scripts/fixtures/source_dims_cases.py --check            # compares

The file's tables are written by GDAL's OGR (2D, Z and M columns; one table's flags set to optional with sqlite3). A
case's verdict follows from the column's Z and M flags as gpkg_geometry_columns says them (read here with sqlite3, not
with KentOS code) and the rule of the ADR, in its order:

- elevations where the column takes none (z = 0): refused, "Z almaz";
- some vertices with an elevation and some without: refused, whatever the column;
- no elevation where the column takes them always (z = 1): refused;
- the same three for measures (M);
- else the write goes through.

A refusal writes nothing: the file keeps its bytes and the drawing its revision. A row's lasting id is the importer's
(uuid5 of the table and the key, gpkg_import_cases.py).
"""
import json
import sqlite3
import sys
import uuid
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import bound_sources_cases as bs  # noqa: E402
import gpkg_import_cases as gi  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
FILE = ROOT / "fixtures/geopackage/v1/boyut.gpkg"
OUT = ROOT / "fixtures/commands/v1/cad.sources.sync.dims.json"
SRID = 5256
ANY = "$any"

SQUARE = [(487000.0, 4420000.0), (487010.0, 4420000.0), (487010.0, 4420010.0), (487000.0, 4420010.0)]
LINE = [(487000.0, 4420100.0), (487010.0, 4420100.0), (487010.0, 4420110.0)]

# Table: (geometry type, Z flag, M flag, the row's elevations, the row's measures) as written.
TABLES = {
    "duz": ("POLYGON", None, None),
    "kotlu": ("POLYGON", [100.0, 100.5, 101.0, 100.5], None),
    "serbest": ("POLYGON", None, None),
    "olculu": ("LINESTRING", None, [0.0, 10.0, 20.0]),
    "serbestcizgi": ("LINESTRING", None, None),
    "cizgi": ("LINESTRING", None, None),
}


def write_files():
    from osgeo import ogr, osr

    if FILE.exists():
        FILE.unlink()
    drv = ogr.GetDriverByName("GPKG")
    ds = drv.CreateDataSource(str(FILE))
    srs = osr.SpatialReference()
    srs.ImportFromEPSG(SRID)
    for name, (kind, zs, ms) in TABLES.items():
        gtype = {
            ("POLYGON", False, False): ogr.wkbPolygon,
            ("POLYGON", True, False): ogr.wkbPolygon25D,
            ("LINESTRING", False, True): ogr.wkbLineStringM,
            ("LINESTRING", False, False): ogr.wkbLineString,
        }[(kind, zs is not None, ms is not None)]
        lyr = ds.CreateLayer(name, srs, gtype, ["FID=fid", "SPATIAL_INDEX=NO"])
        lyr.CreateField(ogr.FieldDefn("ad", ogr.OFTString))
        f = ogr.Feature(lyr.GetLayerDefn())
        f.SetField("ad", name)
        pts = SQUARE if kind == "POLYGON" else LINE
        if kind == "POLYGON":
            ring = ogr.Geometry(ogr.wkbLinearRing)
            for i, (x, y) in enumerate(pts + pts[:1]):
                if zs:
                    ring.AddPoint(x, y, (zs + zs[:1])[i])
                else:
                    ring.AddPoint_2D(x, y)
            g = ogr.Geometry(gtype)
            g.AddGeometry(ring)
        else:
            g = ogr.Geometry(gtype)
            for i, (x, y) in enumerate(pts):
                if ms:
                    g.AddPointM(x, y, ms[i])
                else:
                    g.AddPoint_2D(x, y)
        f.SetGeometry(g)
        lyr.CreateFeature(f)
    ds = None
    # The optional table: Z and M may be present or not (OGC 12-128r19 §2.1.5.1.2).
    with sqlite3.connect(FILE) as db:
        db.execute("UPDATE gpkg_geometry_columns SET z = 2, m = 2 WHERE table_name IN ('serbest', 'serbestcizgi')")


def flags():
    with sqlite3.connect(FILE) as db:
        return {t: (z, m) for t, z, m in db.execute("SELECT table_name, z, m FROM gpkg_geometry_columns")}


def uid_of(table):
    return str(uuid.uuid5(gi.NAMESPACE, f"gpkg:{table}:1"))


def P(x, y):
    return gi.P(x, y)


def geometry(table, zs=None, ms=None):
    """The table's row with these elevations and measures (none: without)."""
    kind = TABLES[table][0]
    if kind == "POLYGON":
        g = {"kind": "polygon", "pts": [P(x, y) for x, y in SQUARE]}
    else:
        g = {"kind": "polyline", "pts": [P(x, y) for x, y in LINE]}
    if zs is not None:
        g["zs"] = zs
    if ms is not None:
        g["ms"] = ms
    return g


def problem(table, zs, ms):
    """The ADR's rule: why the column refuses these elevations and measures, none when it takes them."""
    z, m = flags()[table]
    any_z = zs is not None and any(v is not None for v in zs)
    all_z = zs is not None and all(v is not None for v in zs)
    any_m = ms is not None and any(v is not None for v in ms)
    all_m = ms is not None and all(v is not None for v in ms)
    if any_z and z == 0:
        return "nesnenin kotları var, tablonun geometri sütunu Z almaz; kotları silin (Kot ver › Sıfırla) ya da Z'li bir tabloya yazın"
    if any_z and not all_z:
        return "bazı köşelerin kotu var, bazılarının yok; tabloya ya her köşe kotuyla ya hiçbiri kotsuz yazılır (Kot ver)"
    if not any_z and z == 1:
        return "nesnenin kotu yok, tablonun geometri sütunu her köşede Z ister; köşelere kot verin (Kot ver)"
    if any_m and m == 0:
        return "nesnenin ölçüleri (M) var, tablonun geometri sütunu M almaz; M'li bir tabloya yazın"
    if any_m and not all_m:
        return "bazı köşelerin ölçüsü (M) var, bazılarının yok; rotayı kalibre edin"
    if not any_m and m == 1:
        return "nesnenin ölçüsü (M) yok, tablonun geometri sütunu her köşede M ister; rotayı kalibre edin"
    return None


def linked(table, tmp):
    return [
        {"op": "copyFile", "from": "$fixture:geopackage/v1/boyut.gpkg", "to": f"$tmp:{tmp}"},
        bs.step({"edit": {"kind": "link", "path": f"$tmp:{tmp}", "table": table}},
                {"status": "completed", "output": ANY, "warnings": ANY}, {"revision": "changed"},
                command="cad.sources.edit"),
    ]


def reshape(table, zs, ms):
    return bs.step({"operation": "elevation",
                    "changes": [{"kind": "update", "uid": uid_of(table), "geometry": geometry(table, zs, ms)}]},
                   {"status": "completed", "output": ANY, "warnings": ANY}, {"revision": "changed"},
                   command="cad.entities.edit")


def rename(table):
    return bs.step({"uids": [uid_of(table)], "attrs": {"ad": "yeni"}, "operation": "attributes"},
                   {"status": "completed", "output": ANY, "warnings": ANY}, {"revision": "changed"},
                   command="cad.entities.set")


def write(table, zs, ms, op="execute"):
    """The write and its verdict."""
    why = problem(table, zs, ms)
    inp = {"layer": f"import-{table}", "direction": "write"}
    if why:
        return bs.step(inp, bs.failed("geometry_mismatch", f"{uid_of(table)} kimlikli nesne tabloya yazılamaz: {why}.",
                                      "layer"), op=op)
    out = {"added": 0, "modified": 1, "removed": 0, "source": ANY, "revision": "$current"}
    return bs.step(inp, bs.done(out), {"revision": "same" if op == "plan" else "changed"}, op=op)


def row(table):
    """The row as the file holds it."""
    _, zs, ms = TABLES[table]
    return zs, ms


def cases():
    out = []
    n = [0]

    def c(name, table, *steps):
        n[0] += 1
        out.append(bs.case(name, *linked(table, f"boyut-{n[0]}.gpkg"), *steps))

    full = [100.0, 100.0, 100.0, 100.0]
    none4 = [None, None, None, None]
    part = [100.0, None, 100.0, 100.0]
    line_ms = [0.0, 5.0, 15.0]
    line_part = [0.0, None, 15.0]
    # duz: a 2D polygon column (z = 0, m = 0).
    c("2B sütuna kotlu nesne yazılmaz", "duz", reshape("duz", full, None), write("duz", full, None))
    c("2B sütuna kısmen kotlu nesne de yazılmaz: kotları vardır", "duz", reshape("duz", part, None), write("duz", part, None))
    c("2B sütunda yalnız öznitelik değişince yazılır", "duz", rename("duz"), write("duz", None, None))
    c("plan da kotlu nesneyi 2B sütuna yazmaz", "duz", reshape("duz", full, None), write("duz", full, None, op="plan"))
    # kotlu: a Z column (z = 1).
    zs, _ = row("kotlu")
    c("Z'li sütuna kotları silinmiş nesne yazılmaz", "kotlu", reshape("kotlu", none4, None),
      write("kotlu", none4, None))
    c("Z'li sütuna kısmen kotlu nesne yazılmaz", "kotlu", reshape("kotlu", part, None), write("kotlu", part, None))
    c("Z'li sütuna bütün köşeleri kotlu nesne yazılır", "kotlu", reshape("kotlu", full, None), write("kotlu", full, None))
    c("Z'li sütunda yalnız öznitelik değişince kotlarıyla yazılır", "kotlu", rename("kotlu"), write("kotlu", zs, None))
    c("Z'li sütun: önce reddedilen kısmi kot düzeltilince yazılır", "kotlu",
      reshape("kotlu", part, None), write("kotlu", part, None),
      reshape("kotlu", full, None), write("kotlu", full, None))
    # serbest: Z and M optional (z = 2, m = 2).
    c("isteğe bağlı sütuna kotlu nesne yazılır", "serbest", reshape("serbest", full, None), write("serbest", full, None))
    c("isteğe bağlı sütuna da kısmen kotlu nesne yazılmaz", "serbest", reshape("serbest", part, None),
      write("serbest", part, None))
    c("isteğe bağlı sütunda kotsuz nesne yazılır", "serbest", rename("serbest"), write("serbest", None, None))
    c("isteğe bağlı çizgi sütununa ölçülü (M) çizgi yazılır", "serbestcizgi", reshape("serbestcizgi", None, line_ms),
      write("serbestcizgi", None, line_ms))
    c("isteğe bağlı çizgi sütununa kısmen ölçülü çizgi yazılmaz", "serbestcizgi",
      reshape("serbestcizgi", None, line_part), write("serbestcizgi", None, line_part))
    # olculu: an M line column (m = 1).
    _, ms = row("olculu")
    c("M'li sütuna ölçüleri silinmiş çizgi yazılmaz", "olculu", reshape("olculu", None, [None, None, None]),
      write("olculu", None, [None, None, None]))
    c("M'li sütuna kısmen ölçülü çizgi yazılmaz", "olculu", reshape("olculu", None, line_part),
      write("olculu", None, line_part))
    c("M'li sütuna bütün köşeleri ölçülü çizgi yazılır", "olculu", reshape("olculu", None, line_ms),
      write("olculu", None, line_ms))
    c("M'li ama Z'siz sütuna kotlu çizgi yazılmaz", "olculu", reshape("olculu", [1.0, 2.0, 3.0], line_ms),
      write("olculu", [1.0, 2.0, 3.0], line_ms))
    c("M'li sütunda yalnız öznitelik değişince ölçüleriyle yazılır", "olculu", rename("olculu"),
      write("olculu", None, ms))
    # cizgi: a 2D line column.
    c("2B çizgi sütununa ölçülü çizgi yazılmaz", "cizgi", reshape("cizgi", None, line_ms), write("cizgi", None, line_ms))
    c("2B çizgi sütununa kotlu çizgi yazılmaz", "cizgi", reshape("cizgi", [1.0, 2.0, 3.0], None),
      write("cizgi", [1.0, 2.0, 3.0], None))
    c("2B çizgi sütununda yalnız öznitelik değişince yazılır", "cizgi", rename("cizgi"), write("cizgi", None, None))
    return out


def build():
    return {
        "format": "kentos.command-cases", "version": 1, "command": "cad.sources.sync", "commandVersion": 1,
        "title": "Bağlı tabloya yazmada Z ve M (ADR 0584 §2)",
        "note": "Tablonun Z ve M bayrakları gpkg_geometry_columns'tan sqlite3'le; kural ADR 0584 §2'nin. "
                "scripts/fixtures/source_dims_cases.py yazar.",
        "setup": bs.SETUP,
        "cases": cases(),
    }


def main():
    if "--files" in sys.argv:
        write_files()
        print(f"yazıldı: {FILE.relative_to(ROOT)}")
        return
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            sys.exit(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check'siz çalıştırın")
        print(f"{OUT.name}: {len(build()['cases'])} durum")
        return
    OUT.write_text(text)
    print(f"{OUT.name}: {len(build()['cases'])} durum yazıldı")


if __name__ == "__main__":
    main()
