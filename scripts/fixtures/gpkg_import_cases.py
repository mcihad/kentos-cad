#!/usr/bin/env python3
"""The shared cases of the product command file.import.gpkg (docs/adr/0502, plan/gis.md CBS-03).

    /usr/bin/python3 scripts/fixtures/gpkg_import_cases.py --files   # GDAL writes the small GeoPackages beside qgis.gpkg
    /usr/bin/python3 scripts/fixtures/gpkg_import_cases.py           # writes fixtures/commands/v1/file.import.gpkg.json
    /usr/bin/python3 scripts/fixtures/gpkg_import_cases.py --check   # compares

Each file is read with GDAL's OGR, not KentOS code, and turned into the objects ADR 0502 says: a point (Z its
elevation), a polyline (each arc of a circular string its edge's bulge tan(θ/4), the sweep found from the arc's three
points here; Z elevations, M measures), an area (the closing vertex dropped; holes; parts), a record for a row without
geometry or with an empty one; NULL no attribute, a boolean `true`/`false`, a real its shortest text, a UTC time stamp
without its zone, an integer column with a value past 2⁵³ − 1 as text. A row's lasting id is `kentos_uids`'s, else
UUIDv5 of `gpkg:<table>:<fid>` in ADR 0502's namespace (checked here by a later step that names it).
"""
import hashlib
import json
import math
import sys
import uuid
from pathlib import Path

from osgeo import ogr, osr

ogr.UseExceptions()
sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/geopackage/v1"
OUT = ROOT / "fixtures/commands/v1/file.import.gpkg.json"
NAMESPACE = uuid.UUID(bytes=bytes([0x6b, 0x1e, 0x4f, 0x0a, 0x93, 0x27, 0x4c, 0x51, 0x8d, 0x36, 0x2f, 0x7a, 0x10, 0xc4,
                                   0x5e, 0x92]))
MAX_INTEGER = 2 ** 53 - 1
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
# A layer the import makes looks as a new layer does (the drawing's colour, a 0.18 mm continuous line).
NEW_LAYER_STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.18}


def write_files():
    """The small GeoPackages: a point in WGS 84, a table without a system, an empty table, a broken file."""
    drv = ogr.GetDriverByName("GPKG")

    def make(name, srs, rows, gtype=ogr.wkbPoint):
        path = DIR / name
        if path.exists():
            path.unlink()
        ds = drv.CreateDataSource(str(path))
        lyr = ds.CreateLayer("yer", srs=srs, geom_type=gtype)
        lyr.CreateField(ogr.FieldDefn("ad", ogr.OFTString))
        for wkt, ad in rows:
            f = ogr.Feature(lyr.GetLayerDefn())
            f.SetGeometry(ogr.CreateGeometryFromWkt(wkt))
            f.SetField("ad", ad)
            lyr.CreateFeature(f)
        ds = None

    wgs = osr.SpatialReference()
    wgs.ImportFromEPSG(4326)
    wgs.SetAxisMappingStrategy(osr.OAMS_TRADITIONAL_GIS_ORDER)
    make("wgs84.gpkg", wgs, [("POINT (32.8541 39.9208)", "Kızılay")])
    make("sistemsiz.gpkg", None, [("POINT (12.5 7.25)", "yerel")])
    make("bos.gpkg", None, [])
    (DIR / "bozuk.gpkg").write_bytes(b"Bu bir GeoPackage degil, duz metin.\n")
    print("yazıldı: wgs84.gpkg, sistemsiz.gpkg, bos.gpkg, bozuk.gpkg")
    profile_files()


# ADR 0627 §6, §7: the relations and styles beside the tables.
RELATION_STANDARD = "https://kentos.app/relation/1"
QGIS_LINUX = Path("/usr/share/qgis/python")
QGIS_MAC = Path("/Applications/QGIS-final-4_2_2.app/Contents")
QGIS_STYLE = """
from qgis.core import QgsApplication, QgsVectorLayer, QgsFillSymbol, QgsSingleSymbolRenderer
QgsApplication.setPrefixPath({prefix!r}, True)
app = QgsApplication([], False); app.initQgis()
l = QgsVectorLayer({path!r} + '|layername=parsel', 'parsel', 'ogr')
s = QgsFillSymbol.createSimple({{'color': '230,80,60,128', 'outline_color': '20,60,160', 'outline_width': '0.5'}})
l.setRenderer(QgsSingleSymbolRenderer(s))
print('ok' if l.saveStyleToDatabaseV2('Parsel', 'QGIS', True, '')[0].value == 0 else 'no')
"""


def qgis_style(path):
    """QGIS 4.2 saves the parcels' default style into the GeoPackage (`layer_styles`)."""
    import os
    import subprocess

    if QGIS_LINUX.is_dir():
        py, prefix, env = "/usr/bin/python3", "/usr", dict(os.environ, PYTHONPATH=str(QGIS_LINUX))
    else:
        py, prefix = str(QGIS_MAC / "MacOS/python3.12"), f"{QGIS_MAC}/MacOS"
        env = dict(os.environ, PYTHONHOME=str(QGIS_MAC / "Frameworks"),
                   PYTHONPATH=f"{QGIS_MAC}/Resources/qgis/python:{QGIS_MAC}/Frameworks/lib/python3.12/site-packages")
    env["QT_QPA_PLATFORM"] = "offscreen"
    r = subprocess.run([py, "-c", QGIS_STYLE.format(prefix=prefix, path=str(path))], env=env, capture_output=True,
                       text=True, timeout=300)
    if "ok" not in r.stdout:
        sys.exit(f"QGIS stili yazamadı: {r.stderr[-600:]}")


def profile_files():
    """profil-qgis.gpkg: GDAL's tables and many-to-many relationship (its mapping table, which GDAL also lists in
    gpkg_contents) and QGIS 4.2's default style of the parcels. profil-kentos.gpkg: GDAL's tables, and the relation
    KentOS writes (OGC 18-000 `kentos_references`, the mapping by fid) with KentOS's document of it on the child's
    field column, written here with sqlite3 by the GeoPackage and ADR 0627's tables, not by KentOS."""
    import sqlite3

    from osgeo import gdal

    gdal.UseExceptions()
    srs = osr.SpatialReference()
    srs.ImportFromEPSG(5256)

    def square(x, y, side):
        return f"POLYGON(({x} {y},{x + side} {y},{x + side} {y + side},{x} {y + side},{x} {y}))"

    path = DIR / "profil-qgis.gpkg"
    path.unlink(missing_ok=True)
    ds = gdal.GetDriverByName("GPKG").Create(str(path), 0, 0, 0, gdal.GDT_Unknown)
    p = ds.CreateLayer("parsel", srs, ogr.wkbPolygon)
    p.CreateField(ogr.FieldDefn("ada", ogr.OFTInteger))
    m = ds.CreateLayer("malik", None, ogr.wkbNone)
    m.CreateField(ogr.FieldDefn("ad", ogr.OFTString))
    for i in range(3):
        f = ogr.Feature(p.GetLayerDefn())
        f.SetField("ada", 101 + i)
        f.SetGeometry(ogr.CreateGeometryFromWkt(square(487200 + 20 * i, 4420500, 15)))
        p.CreateFeature(f)
    for name in ["Ayşe Yılmaz", "Mehmet Kaya", "Zeynep Çelik"]:
        f = ogr.Feature(m.GetLayerDefn())
        f.SetField("ad", name)
        m.CreateFeature(f)
    rel = gdal.Relationship("parsel_malik", "parsel", "malik", gdal.GRC_MANY_TO_MANY)
    rel.SetType(gdal.GRT_ASSOCIATION)
    rel.SetLeftTableFields(["fid"])
    rel.SetRightTableFields(["fid"])
    rel.SetLeftMappingTableFields(["base_id"])
    rel.SetRightMappingTableFields(["related_id"])
    rel.SetRelatedTableType("attributes")
    if not ds.AddRelationship(rel):
        sys.exit("GDAL ilişkiyi yazamadı")
    ds = None
    c = sqlite3.connect(path)
    c.executemany("INSERT INTO parsel_malik (base_id, related_id) VALUES (?, ?)", [(1, 1), (1, 2), (2, 2), (3, 3)])
    c.commit()
    c.close()
    qgis_style(path)
    # QGIS opens a GeoPackage in WAL mode: the fixture goes back to a rollback journal, whole in its one file.
    c = sqlite3.connect(path)
    c.execute("PRAGMA journal_mode=DELETE")
    c.close()

    path = DIR / "profil-kentos.gpkg"
    path.unlink(missing_ok=True)
    ds = gdal.GetDriverByName("GPKG").Create(str(path), 0, 0, 0, gdal.GDT_Unknown)
    p = ds.CreateLayer("parsel", srs, ogr.wkbPolygon)
    p.CreateField(ogr.FieldDefn("ada", ogr.OFTInteger))
    b = ds.CreateLayer("bina", srs, ogr.wkbPolygon)
    b.CreateField(ogr.FieldDefn("parsel", ogr.OFTString))
    b.CreateField(ogr.FieldDefn("kat", ogr.OFTInteger))
    for i in range(2):
        f = ogr.Feature(p.GetLayerDefn())
        f.SetField("ada", 201 + i)
        f.SetGeometry(ogr.CreateGeometryFromWkt(square(487300 + 40 * i, 4420500, 30)))
        p.CreateFeature(f)
    owner = {1: 1, 2: 1, 3: 2}
    for k, (x, y) in enumerate([(487302, 4420502), (487315, 4420515), (487345, 4420505)], start=1):
        f = ogr.Feature(b.GetLayerDefn())
        f.SetField("parsel", str(uuid.uuid5(NAMESPACE, f"gpkg:parsel:{owner[k]}")))
        f.SetField("kat", k + 1)
        f.SetGeometry(ogr.CreateGeometryFromWkt(square(x, y, 8)))
        b.CreateFeature(f)
    ds = None
    rte = "http://www.opengis.net/doc/IS/gpkg-rte/1.0"
    meta = "http://www.geopackage.org/spec120/#extension_metadata"
    c = sqlite3.connect(path)
    c.executescript("""
        CREATE TABLE gpkgext_relations (id INTEGER PRIMARY KEY AUTOINCREMENT, base_table_name TEXT NOT NULL,
          base_primary_column TEXT NOT NULL DEFAULT 'id', related_table_name TEXT NOT NULL,
          related_primary_column TEXT NOT NULL DEFAULT 'id', relation_name TEXT NOT NULL,
          mapping_table_name TEXT NOT NULL UNIQUE);
        CREATE TABLE bina_parsel (base_id INTEGER NOT NULL, related_id INTEGER NOT NULL);
        CREATE TABLE gpkg_metadata (id INTEGER CONSTRAINT m_pk PRIMARY KEY ASC NOT NULL,
          md_scope TEXT NOT NULL DEFAULT 'dataset', md_standard_uri TEXT NOT NULL,
          mime_type TEXT NOT NULL DEFAULT 'text/xml', metadata TEXT NOT NULL DEFAULT '');
        CREATE TABLE gpkg_metadata_reference (reference_scope TEXT NOT NULL, table_name TEXT, column_name TEXT,
          row_id_value INTEGER, timestamp DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
          md_file_id INTEGER NOT NULL, md_parent_id INTEGER,
          CONSTRAINT crmr_mfi_fk FOREIGN KEY (md_file_id) REFERENCES gpkg_metadata(id),
          CONSTRAINT crmr_mpi_fk FOREIGN KEY (md_parent_id) REFERENCES gpkg_metadata(id));
    """)
    c.executemany("INSERT INTO gpkg_extensions VALUES (?, ?, ?, ?, 'read-write')", [
        ("gpkgext_relations", None, "gpkg_related_tables", rte),
        ("bina_parsel", None, "gpkg_related_tables", rte),
        ("gpkg_metadata", None, "gpkg_metadata", meta),
        ("gpkg_metadata_reference", None, "gpkg_metadata", meta),
    ])
    c.execute("INSERT INTO gpkgext_relations (base_table_name, base_primary_column, related_table_name, "
              "related_primary_column, relation_name, mapping_table_name) VALUES "
              "('bina', 'fid', 'parsel', 'fid', 'kentos_references', 'bina_parsel')")
    c.executemany("INSERT INTO bina_parsel VALUES (?, ?)", sorted(owner.items()))
    doc = {"mapping": "bina_parsel", "id": "bina-parsel", "name": "Binanın parseli", "field": "parsel",
           "onDelete": "restrict"}
    c.execute("INSERT INTO gpkg_metadata (md_scope, md_standard_uri, mime_type, metadata) VALUES "
              "('dataset', ?, 'application/json', ?)", (RELATION_STANDARD, json.dumps(doc, ensure_ascii=False)))
    c.execute("INSERT INTO gpkg_metadata_reference (reference_scope, table_name, column_name, timestamp, md_file_id) "
              "VALUES ('column', 'bina', 'parsel', '2026-10-24T00:00:00.000Z', 1)")
    c.commit()
    c.close()
    print("yazıldı: profil-qgis.gpkg, profil-kentos.gpkg")


def P(x, y):
    return {"x": float(x), "y": float(y)}


def sweep(a, m, b):
    """The signed sweep from a through m to b (counter-clockwise positive), from the circle through the three."""
    (ax, ay), (mx, my), (bx, by) = a, m, b
    d = 2 * (ax * (my - by) + mx * (by - ay) + bx * (ay - my))
    ux = ((ax * ax + ay * ay) * (my - by) + (mx * mx + my * my) * (by - ay) + (bx * bx + by * by) * (ay - my)) / d
    uy = ((ax * ax + ay * ay) * (bx - mx) + (mx * mx + my * my) * (ax - bx) + (bx * bx + by * by) * (mx - ax)) / d
    t0, t2 = math.atan2(ay - uy, ax - ux), math.atan2(by - uy, bx - ux)
    ccw = (mx - ax) * (by - ay) - (my - ay) * (bx - ax) > 0
    tau = 2 * math.pi
    return (t2 - t0) % tau if ccw else -((t0 - t2) % tau)


def points(g):
    out = []
    for i in range(g.GetPointCount()):
        p = g.GetPoint(i)
        z = p[2] if g.Is3D() else None
        m = g.GetM(i) if g.IsMeasured() else None
        out.append((p[0], p[1], z, m))
    return out


def walk(g):
    """A curve as vertices (x, y, z, m) and each vertex's edge bulge (the last 0)."""
    name = g.GetGeometryName()
    pts, bul = [], []

    def push(p, before=None):
        if before is not None and bul:
            bul[-1] = before
        if pts and pts[-1][0] == p[0] and pts[-1][1] == p[1]:
            return
        pts.append(p)
        bul.append(0.0)

    def add(c):
        n = c.GetGeometryName()
        if n == "LINESTRING" or n == "LINEARRING":
            for p in points(c):
                push(p, 0.0 if pts else None)
        elif n == "CIRCULARSTRING":
            ps = points(c)
            push(ps[0])
            for i in range(0, len(ps) - 2, 2):
                s = sweep(ps[i][:2], ps[i + 1][:2], ps[i + 2][:2])
                push(ps[i + 2], math.tan(s / 4))
        elif n == "COMPOUNDCURVE":
            for k in range(c.GetGeometryCount()):
                add(c.GetGeometryRef(k))
        else:
            raise ValueError(n)

    add(g)
    return pts, bul, name


def fields_of(pts, bul, measures):
    out = {"pts": [P(p[0], p[1]) for p in pts]}
    if any(b != 0 for b in bul):
        out["bulges"] = bul
    if any(p[2] is not None for p in pts):
        out["zs"] = [p[2] for p in pts]
    if measures and any(p[3] is not None for p in pts):
        out["ms"] = [p[3] for p in pts]
    return out


def ring(g):
    pts, bul, _ = walk(g)
    if len(pts) > 1 and pts[0][:2] == pts[-1][:2]:
        pts, bul = pts[:-1], bul[:-1]
    return pts, bul


def surface(g):
    rings = [ring(g.GetGeometryRef(i)) for i in range(g.GetGeometryCount())]
    out = fields_of(*rings[0], False)
    if len(rings) > 1:
        out["holes"] = [fields_of(*r, False) for r in rings[1:]]
    return out


def geometry(g):
    """The object's kind and geometry fields; none for a record."""
    if g is None or g.IsEmpty():
        return "record", {}
    n = g.GetGeometryName()
    if n == "POINT":
        p = points(g)[0]
        out = {"p": P(p[0], p[1])}
        if p[2] is not None:
            out["z"] = p[2]
        return "point", out
    if n in ("LINESTRING", "CIRCULARSTRING", "COMPOUNDCURVE"):
        pts, bul, _ = walk(g)
        return "polyline", fields_of(pts, bul, True)
    if n in ("POLYGON", "CURVEPOLYGON"):
        return "polygon", surface(g)
    if n in ("MULTIPOLYGON", "MULTISURFACE"):
        parts = [surface(g.GetGeometryRef(i)) for i in range(g.GetGeometryCount())]
        out = dict(parts[0])
        if len(parts) > 1:
            out["parts"] = parts[1:]
        return "polygon", out
    raise ValueError(n)


def text(v, kind, wide):
    if v is None:
        return None
    if kind == "Boolean":
        return "true" if v else "false"
    if kind in ("Integer", "Integer64"):
        return str(v)
    if kind == "Real":
        return repr(float(v))
    if kind == "DateTime":
        return v[:19].replace(" ", "T")
    return v


def read(name):
    """Every table: its name, fields (as layer fields), rows as (fid, uid, kind, fields, attrs), losses."""
    path = DIR / name
    ds = ogr.Open(str(path))
    uids = {}
    if ds.GetLayerByName("kentos_uids") is not None:
        rs = ds.ExecuteSQL("SELECT table_name, fid, uid FROM kentos_uids")
        for r in rs:
            uids[(r.GetField(0), r.GetField(1))] = r.GetField(2)
        ds.ReleaseResultSet(rs)
    tables = []
    for i in range(ds.GetLayerCount()):
        lyr = ds.GetLayerByIndex(i)
        defn = lyr.GetLayerDefn()
        cols = []
        for j in range(defn.GetFieldCount()):
            fd = defn.GetFieldDefn(j)
            kind = "Boolean" if fd.GetSubType() == ogr.OFSTBoolean else fd.GetTypeName()
            cols.append((fd.GetName(), kind, fd.GetWidth()))
        feats = list(lyr)
        wide = {c[0] for j, c in enumerate(cols) if c[1] in ("Integer", "Integer64")
                and any(f.IsFieldSetAndNotNull(j) and abs(f.GetFieldAsInteger64(j)) > MAX_INTEGER for f in feats)}
        fields = []
        for cname, kind, width in cols:
            k = {"String": "text", "Integer": "integer", "Integer64": "integer", "Real": "decimal", "Date": "date",
                 "DateTime": "datetime", "Boolean": "boolean"}[kind]
            if cname in wide:
                k = "text"
            f = {"name": cname, "kind": k}
            if kind == "String" and width:
                f["length"] = width
            fields.append(f)
        losses = [f"“{c}” sütunu 2⁵³'ten büyük sayılar tutuyor: metin alanı olarak, her basamağıyla alındı"
                  for c in sorted(wide)]
        rows = []
        for f in feats:
            attrs = {}
            for j, (cname, kind, _) in enumerate(cols):
                if not f.IsFieldSetAndNotNull(j):
                    continue
                if kind == "Integer64" or (kind == "Integer" and cname in wide):
                    v = f.GetFieldAsInteger64(j)
                elif kind == "Integer" or kind == "Boolean":
                    v = f.GetFieldAsInteger(j)
                elif kind == "Real":
                    v = f.GetFieldAsDouble(j)
                elif kind == "Date":
                    y, mo, d = f.GetFieldAsDateTime(j)[:3]
                    v = f"{y:04d}-{mo:02d}-{d:02d}"
                elif kind == "DateTime":
                    v = f.GetFieldAsISO8601DateTime(j)
                else:
                    v = f.GetFieldAsString(j)
                attrs[cname] = text(v, kind, cname in wide)
            kind, geo = geometry(f.GetGeometryRef())
            fid = f.GetFID()
            uid = uids.get((lyr.GetName(), fid)) or str(uuid.uuid5(NAMESPACE, f"gpkg:{lyr.GetName()}:{fid}"))
            rows.append({"fid": fid, "uid": uid, "kind": kind, "geometry": geo, "attrs": attrs})
        srs = lyr.GetSpatialRef()
        srid = int(srs.GetAuthorityCode(None)) if srs is not None and srs.GetAuthorityName(None) == "EPSG" else None
        tables.append({"name": lyr.GetName(), "fields": fields, "rows": rows, "losses": losses, "srid": srid})
    data = path.read_bytes()
    return {"file": name, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest(), "tables": tables}


def slug(name):
    folded = name.translate(str.maketrans("çğıöşüÇĞİÖŞÜ", "cgiosucgiosu")).lower()
    out = ""
    for c in folded:
        if c.isascii() and (c.islower() or c.isdigit()):
            out += c
        elif not out.endswith("-"):
            out += "-"
    return "import-" + (out.strip("-") or "katman")


# The drawing: TUREF / TM36 (EPSG:5256); a layer named “Yol” (the road table goes onto it), a locked layer, a group.
SETUP = {
    "format": "kentos.document", "version": 1, "name": "GeoPackage alma",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                 "plotScale": 1000, "workspace": "gis"},
    "origin": P(0, 0),
    "layers": [
        {"id": "cizim", "name": "Çizim", "type": "layer", "visible": True, "locked": False, "expanded": True,
         "style": STYLE, "children": []},
        {"id": "kilitli", "name": "Kilitli", "type": "layer", "visible": True, "locked": True, "expanded": True,
         "style": STYLE, "children": []},
        {"id": "grup", "name": "Grup", "type": "group", "visible": True, "locked": False, "expanded": True,
         "style": STYLE, "children": []},
    ],
    "activeLayer": "cizim",
    "entities": [{"kind": "point", "id": 1, "layerId": "cizim", "attrs": {}, "p": P(487500, 4420500)}],
    "styles": {"items": [], "categories": []},
}
SAME = {"ids": [1], "canUndo": False, "dirty": False, "revision": "same"}


def done(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def fx(name):
    return f"$fixture:geopackage/v1/{name}"


def imported(name, tables=None, routes=None, start=2, taken=(), keep=None):
    """The command's answer and the objects it adds (their fields by id), for `tables` (all when none); `keep`, a
    table's fids, takes only those rows (a part's working copy, docs/adr/0510)."""
    r = read(name)
    if keep is not None:
        r = {**r, "tables": [{**t, "rows": [x for x in t["rows"] if x["fid"] in keep.get(t["name"], ())]}
                             for t in r["tables"]]}
    beside = side_tables(name) if tables is None else set()
    chosen = [t for t in r["tables"] if (tables is None and t["name"] not in beside) or (tables and t["name"] in tables)]
    chosen = [t for t in chosen if t["rows"]]
    out_tables, made, fields, ids, losses = [], [], {}, [], set()
    used = set(taken)
    slot = start
    group = slug(name.removesuffix(".gpkg"))
    for t in chosen:
        losses.update(t["losses"])
        layer = (routes or {}).get(t["name"])
        if layer is None:
            layer = slug(t["name"])
            k = 2
            while layer in used:
                layer = f"{slug(t['name'])}-{k}"
                k += 1
            used.add(layer)
            made.append({"id": layer, "name": t["name"]})
        for row in t["rows"]:
            fields[str(slot)] = {"kind": row["kind"], "layerId": layer, "attrs": row["attrs"], **row["geometry"]}
            ids.append(slot)
            slot += 1
        out_tables.append({"table": t["name"], "layer": layer, "read": len(t["rows"]), "objects": len(t["rows"]),
                           "records": sum(1 for x in t["rows"] if x["kind"] == "record"), "skipped": 0})
    srids = sorted({t["srid"] for t in chosen if t["srid"]})
    output = {"file": r["file"], "bytes": r["bytes"], "sha256": r["sha256"], "objects": len(ids),
              "tables": out_tables, "layers": made, "revision": "$current"}
    if made:
        output["group"] = group
    if srids:
        output["srid"] = srids[0]
    warnings = []
    if not srids:
        warnings.append({"code": "crs_undeclared",
                         "message": f"{r['file']} koordinat sistemi bildirmiyor; koordinatlar projenin sisteminde "
                                    "sayılıp olduğu gibi alındı.", "path": "path"})
    if losses:
        warnings.append({"code": "import_losses",
                         "message": f"{r['file']} dosyasının bazı değerleri ya da satırları olduğu gibi alınamadı: "
                                    + "; ".join(sorted(losses)) + ".", "path": "path"})
    return output, warnings, fields, ids, r


def side_tables(name):
    """QGIS's styles and the related tables' mappings: no data of their own, left out of a whole-file import (ADR 0627
    §6, §7)."""
    import sqlite3

    c = sqlite3.connect(f"file:{DIR / name}?mode=ro", uri=True)
    have = {r[0] for r in c.execute("SELECT name FROM sqlite_master WHERE type = 'table'")}
    out = {"layer_styles"}
    if "gpkgext_relations" in have:
        out |= {r[0] for r in c.execute("SELECT mapping_table_name FROM gpkgext_relations")}
    c.close()
    return out


def file_profile(name):
    """The file's relations (with KentOS's document when it has one) and its tables' default QGIS styles."""
    import sqlite3

    c = sqlite3.connect(f"file:{DIR / name}?mode=ro", uri=True)
    have = {r[0] for r in c.execute("SELECT name FROM sqlite_master WHERE type = 'table'")}
    relations = []
    if "gpkgext_relations" in have:
        for base, related, rname, mapping in c.execute(
                "SELECT base_table_name, related_table_name, relation_name, mapping_table_name FROM gpkgext_relations "
                "ORDER BY id"):
            doc = None
            if rname == "kentos_references" and {"gpkg_metadata", "gpkg_metadata_reference"} <= have:
                for (text,) in c.execute(
                        "SELECT m.metadata FROM gpkg_metadata m JOIN gpkg_metadata_reference r ON r.md_file_id = m.id "
                        "WHERE r.table_name = ? AND m.md_standard_uri = ? ORDER BY m.id", (base, RELATION_STANDARD)):
                    d = json.loads(text)
                    if d.get("mapping") == mapping:
                        doc = d
                        break
            rows = [tuple(r) for r in c.execute(f'SELECT base_id, related_id FROM "{mapping}" ORDER BY rowid')]
            relations.append({"base": base, "related": related, "mapping": mapping, "doc": doc, "rows": rows})
    styles = {}
    if "layer_styles" in have:
        for table, qml in c.execute(
                "SELECT f_table_name, styleQML FROM layer_styles WHERE styleQML IS NOT NULL AND f_table_name IS NOT NULL "
                "ORDER BY f_table_name, CASE WHEN useAsDefault THEN 0 ELSE 1 END, update_time DESC, id DESC"):
            styles.setdefault(table, qml)
    c.close()
    return relations, styles


def profile_take(name, tables=None, routes=None, op="execute"):
    """ADR 0627 §6, §7: the file taken with its relations and styles. Another writer's many-to-many relation whose two
    tables are taken is a multi-valued text field of the base table's layer (named after its mapping table) holding
    the related rows' lasting ids `; ` apart in the mapping's order, and a relation (its id the mapping's name with
    `-`); KentOS's relation comes back from its document; a table's default QGIS style is the look of the layer the
    import makes for it, its renderer as KentOS reads the QML (qml_cases.py's reader)."""
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import qml_cases

    output, warnings, fields, ids, r = imported(name, tables, routes)
    relations, styles = file_profile(name)
    by_table = {t["table"]: t["layer"] for t in output["tables"]}
    made = {l["id"] for l in output["layers"]}
    rows_of = {t["name"]: t["rows"] for t in r["tables"]}
    left = []
    nodes = {}
    layer_fields = {t["name"]: list(t["fields"]) for t in r["tables"]}
    # KentOS's relations: a multi-valued field is one again (its column said only text).
    for rel in relations:
        d = rel["doc"]
        if d is not None and d.get("multiple") and rel["base"] in layer_fields:
            for f in layer_fields[rel["base"]]:
                if f["name"] == d["field"]:
                    f["multiple"] = True
    # Another writer's relations: the base table's reference field, before the tables are taken.
    foreign = {}
    for rel in relations:
        if rel["doc"] is not None:
            continue
        if rel["base"] not in by_table or rel["related"] not in by_table:
            left.append(f"“{rel['mapping']}” ilişkisi alınmadı: “{rel['base']}” ve “{rel['related']}” tablolarının "
                        "ikisi birden alınmıyor")
            continue
        related_uid = {x["fid"]: x["uid"] for x in rows_of[rel["related"]]}
        held = {}
        for b, rf in rel["rows"]:
            ids_ = held.setdefault(b, [])
            if related_uid[rf] not in ids_:
                ids_.append(related_uid[rf])
        key = rel["mapping"]
        layer_fields[rel["base"]].append({"name": key, "kind": "text", "multiple": True})
        foreign[rel["mapping"]] = key
        slot_of = {}
        slot = 2
        for t in output["tables"]:
            for x in rows_of[t["table"]]:
                slot_of[(t["table"], x["fid"])] = slot
                slot += 1
        for fid, ids_ in held.items():
            fields[str(slot_of[(rel["base"], fid)])]["attrs"][key] = "; ".join(ids_)
    out_styles = 0
    for table, qml in sorted(styles.items()):
        layer = by_table.get(table)
        if layer is None or layer not in made:
            continue
        renderer, lost, _ = qml_cases.read(qml)
        nodes.setdefault(layer, {})["style"] = {**NEW_LAYER_STYLE, "renderer": renderer}
        out_styles += 1
        if lost:
            left.append(f"“{table}” tablosunun QGIS stilinden alınamayanlar: {', '.join(lost)}")
    for t in output["tables"]:
        if t["layer"] in made:
            nodes.setdefault(t["layer"], {})["fields"] = layer_fields[t["table"]]
    added = []
    for rel in relations:
        child, parent = by_table.get(rel["base"]), by_table.get(rel["related"])
        if child is None or parent is None:
            if rel["doc"] is not None:
                left.append(f"“{rel['mapping']}” ilişkisi alınmadı: “{rel['base']}” ve “{rel['related']}” tablolarının "
                            "ikisi birden alınmıyor")
            continue
        if rel["doc"] is not None:
            d = rel["doc"]
            made_rel = {"id": d["id"], "name": d["name"], "parent": parent, "child": child, "field": d["field"]}
            if d.get("onDelete", "unlink") != "unlink":
                made_rel["onDelete"] = d["onDelete"]
        elif rel["mapping"] in foreign:
            made_rel = {"id": rel["mapping"].replace("_", "-"), "name": rel["mapping"], "parent": parent,
                        "child": child, "field": foreign[rel["mapping"]]}
        else:
            continue
        added.append(made_rel)
    if added:
        output["relations"] = [x["id"] for x in added]
    if out_styles:
        output["styles"] = out_styles
    if left:
        warnings = [*warnings, {"code": "import_losses",
                                "message": f"{r['file']} dosyasının ilişkilerinin ya da stillerinin bir kısmı alınmadı: "
                                           + "; ".join(left) + ".", "path": "path"}]
    inp = {"path": fx(name)}
    if tables is not None:
        inp["tables"] = tables
    if routes:
        inp["layers"] = routes
    if op == "plan":
        return step(inp, done({**output, "revision": "$current"}, warnings), SAME, op="plan")
    expect = {"ids": [1, *ids], "fields": fields, "revision": "changed", "canUndo": True, "nodes": nodes,
              "settings": {"relations": added or None}}
    return step(inp, done(output, warnings), expect)


def step(inp, result, expect=None, op="execute", command=None):
    s = {"op": op, "input": inp, "result": result, "expect": expect or SAME}
    if command:
        s["command"] = command
    return s


def case(name, *steps):
    return {"name": name, "steps": list(steps)}


def take(name, inp_extra=None, tables=None, routes=None, op="execute", expect_layers=False):
    output, warnings, fields, ids, r = imported(name, tables, routes)
    inp = {"path": fx(name), **(inp_extra or {})}
    if op == "plan":
        output = {**output, "revision": "$current"}
        return step(inp, done(output, warnings), SAME, op="plan")
    expect = {"ids": [1, *ids], "fields": fields, "revision": "changed", "canUndo": True}
    return step(inp, done(output, warnings), expect)


def cases():
    q = "qgis.gpkg"
    parsel = read(q)["tables"][0]["rows"]
    return [
        case("QGIS'in yazdığı bütün tablolar: katmanlar alanlarıyla, kayıtlar, eğriler, Z ve M", take(q)),
        case("yalnız parseller: iki parçalı ve delikli alan, geometrisiz satır kayıt", take(q, {"tables": ["parsel"]},
                                                                                         tables=["parsel"])),
        case("yol tablosu var olan katmana yönlendirilir",
             take(q, {"tables": ["yol"], "layers": {"yol": "cizim"}}, tables=["yol"], routes={"yol": "cizim"})),
        case("eğri alan ve birleşik eğri: yaylar bulge olarak", take(q, {"tables": ["egri", "egri_yol"]},
                                                                      tables=["egri", "egri_yol"])),
        case("Z'li noktalar", take(q, {"tables": ["nokta"]}, tables=["nokta"])),
        case("geometrisiz öznitelik tablosu kayıt olur", take(q, {"tables": ["malik"]}, tables=["malik"])),
        case("plan hiçbir şey yazmaz", take(q, {"tables": ["parsel"]}, tables=["parsel"], op="plan")),
        case("validate yalnız denetler", step({"path": fx(q)}, done(None), op="validate")),
        case("aynı dosya ikinci kez alınmaz",
             take(q, {"tables": ["nokta"]}, tables=["nokta"]),
             step({"path": fx(q), "tables": ["nokta"]},
                  failed("entity_exists",
                         f"Dosyadaki bir kaydın kalıcı kimliği ({read(q)['tables'][2]['rows'][0]['uid']}) çizimde zaten "
                         f"bir nesnenin: {q} daha önce alınmış. Önceki alımı geri alın ya da başka bir çizimde açın.",
                         "path"),
                  {"ids": [1, 2], "revision": "same"})),
        case("satırın kalıcı kimliği tablo ve fid'den: ilişki komutu onu bulur",
             take(q, {"tables": ["parsel"]}, tables=["parsel"]),
             step({"a": [parsel[0]["uid"]], "b": [parsel[1]["uid"]], "predicate": "disjoint"},
                  done({"pairs": [{"a": parsel[0]["uid"], "b": parsel[1]["uid"], "matrix": "FF2FF1212"}],
                        "revision": "$current"}),
                  {"ids": [1, 2, 3, 4], "revision": "same"}, command="cad.entities.relate")),
        case("KentOS'un yazdığı dosyanın kentos_uids kimlikleri korunur",
             take("kentos.gpkg", {"tables": ["parsel"]}, tables=["parsel"]),
             step({"a": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7000"], "b": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7001"],
                   "predicate": "disjoint"},
                  done({"pairs": [{"a": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7000",
                                   "b": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7001", "matrix": "FF2FF1212"}],
                        "revision": "$current"}),
                  {"ids": [1, 2, 3, 4], "revision": "same"}, command="cad.entities.relate")),
        case("sistem bildirmeyen dosya olduğu gibi alınır", take("sistemsiz.gpkg")),
        case("QGIS'in ilişkisi çok değerli başvuru alanı ve ilişki olur, varsayılan stili katmanın görünüşü; stil ve "
             "eşleme tablosu katman olmaz", profile_take("profil-qgis.gpkg")),
        case("KentOS'un ilişkisi belgesinden geri gelir: kimliği, adı, alanı, silme kuralı",
             profile_take("profil-kentos.gpkg")),
        case("ilişkinin bir tablosu alınmayınca ilişki alınmaz ve söylenir; stil yine gelir",
             profile_take("profil-qgis.gpkg", tables=["parsel"])),
        case("var olan katmana giden tablo kendi görünüşünü korur",
             profile_take("profil-qgis.gpkg", tables=["parsel", "malik"], routes={"parsel": "cizim"})),
        case("ilişkili dosyanın planı hiçbir şey yazmaz", profile_take("profil-qgis.gpkg", op="plan")),
        case("WGS 84'teki dosya çevirme istenmeden alınmaz",
             step({"path": fx("wgs84.gpkg")},
                  failed("crs_mismatch", "wgs84.gpkg EPSG:4326 sisteminde, proje EPSG:5256. Koordinatların projenin "
                                         "sistemine çevrilmesi için reproject verin ya da projeyi EPSG:4326'de açın.",
                         "reproject"))),
        case("WGS 84'teki dosya reproject ile projenin sistemine çevrilir",
             step({"path": fx("wgs84.gpkg"), "reproject": True},
                  done({"file": "wgs84.gpkg", "bytes": read("wgs84.gpkg")["bytes"],
                        "sha256": read("wgs84.gpkg")["sha256"], "srid": 4326, "moved": True, "objects": 1,
                        "tables": [{"table": "yer", "layer": "import-yer", "read": 1, "objects": 1, "records": 0,
                                    "skipped": 0}],
                        "layers": [{"id": "import-yer", "name": "yer"}], "group": "import-wgs84",
                        "revision": "$current"}),
                  {"ids": [1, 2], "revision": "changed", "canUndo": True,
                   "fields": {"2": {"kind": "point", "layerId": "import-yer", "attrs": {"ad": "Kızılay"}}}})),
        case("boş tablo: alınacak kayıt yok",
             step({"path": fx("bos.gpkg")},
                  failed("nothing_to_import", "bos.gpkg dosyasının seçilen tablolarında alınacak kayıt yok.", "path"))),
        case("dosyada olmayan tablo",
             step({"path": fx(q), "tables": ["parsel", "bina"]},
                  failed("table_not_found", "“bina” tablosu dosyada yok; dosyanın tablolarından birinin adını verin.",
                         "tables[1]"))),
        case("yönlendirilen katman yok",
             step({"path": fx(q), "tables": ["yol"], "layers": {"yol": "yok"}},
                  failed("layer_not_found", "“yok” kimlikli katman çizimde yok; “yol” tablosunun kayıtlarını var "
                                            "olan bir katmanın kimliğine yönlendirin.", "layers.yol"))),
        case("yönlendirilen düğüm grup",
             step({"path": fx(q), "tables": ["yol"], "layers": {"yol": "grup"}},
                  failed("not_a_layer", "“Grup” bir grup; kayıtlar yalnız katmana konur. Grubun altındaki bir katmanı "
                                        "verin.", "layers.yol"))),
        case("yönlendirilen katman kilitli",
             step({"path": fx(q), "tables": ["yol"], "layers": {"yol": "kilitli"}},
                  failed("layer_locked", "“Kilitli” katmanı kilitli; kilidi Katmanlar panelinden açın ya da başka bir "
                                         "katman verin.", "layers.yol"))),
        case("göreli yol",
             step({"path": "veri/kadastro.gpkg"},
                  failed("invalid_file_path", "“veri/kadastro.gpkg” mutlak bir dosya yolu değil; okunacak dosyanın tam "
                                              "yolunu verin (/klasör/ad.gpkg).", "path"))),
        case("klasör",
             step({"path": "$fixture:geopackage/v1"},
                  failed("invalid_file_path", "$fixture:geopackage/v1 bir klasör; okunacak GeoPackage dosyasının adını "
                                              "da verin.", "path"))),
        case("olmayan dosya",
             step({"path": fx("yok.gpkg")},
                  failed("file_not_found", f"{fx('yok.gpkg')} yok; dosyanın yolunu denetleyin.", "path"))),
        case("GeoPackage olmayan dosya",
             step({"path": fx("bozuk.gpkg")},
                  failed("import_unreadable", f"{fx('bozuk.gpkg')} GeoPackage olarak okunamadı: “{fx('bozuk.gpkg')}” "
                                              "bir SQLite dosyası değil", "path"))),
    ]


QGIS_CHECK = """
import json
from qgis.core import QgsApplication, QgsVectorLayer, QgsProviderRegistry
QgsApplication.setPrefixPath({prefix!r}, True)
app = QgsApplication([], False); app.initQgis()
l = QgsVectorLayer({path!r} + '|layername=parsel', 'parsel', 'ogr')
r = l.renderer()
sym = r.symbol() if hasattr(r, 'symbol') else None
out = {{'valid': l.isValid(), 'renderer': r.type(), 'color': sym.color().name(QColor.NameFormat.HexArgb) if sym else None}}
conn = QgsProviderRegistry.instance().providerMetadata('ogr').createConnection({path!r}, {{}})
out['relations'] = [[w.name(), w.referencingLayerName(), w.referencedLayerName(), w.mappingTableName()]
                    for w in conn.relationships()]
print(json.dumps(out))
"""


def without_ids(v):
    if isinstance(v, dict):
        return {k: without_ids(x) for k, x in v.items() if k != "id"}
    if isinstance(v, list):
        return [without_ids(x) for x in v]
    return v


def check_written():
    """KentOS's GeoPackage of a drawing with QGIS's relation and look taken in (written by `KENTOS_WRITE_GPKG=1 cargo
    test -p kentos-headless --test headless geopackage`): GDAL's validator; the relationship as GDAL reads it and its
    mapping's rows (the source's pairs); KentOS's document on the child's column; the default style's renderer as
    KentOS's reader reads the source's; and QGIS 4.2 opening the parcels with that style and finding the relation."""
    import os
    import sqlite3
    import subprocess

    from osgeo import gdal

    path = DIR / "kentos-profil.gpkg"
    if not path.exists():
        print("kentos-profil.gpkg yok (KENTOS_WRITE_GPKG=1 cargo test -p kentos-headless --test headless geopackage); "
              "denetimi atlandı")
        return []
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import qml_cases
    from osgeo_utils.samples import validate_gpkg

    gdal.UseExceptions()
    bad = []
    errs = validate_gpkg.check(str(path), abort_at_first_error=False) or []
    if errs:
        bad.append(f"kentos-profil.gpkg: validate_gpkg: {errs[:3]}")
    ds = gdal.OpenEx(str(path))
    names = ds.GetRelationshipNames() or []
    if names != ["kentos_references"]:
        bad.append(f"kentos-profil.gpkg: GDAL'ın ilişkileri {names}")
    else:
        r = ds.GetRelationship(names[0])
        got = (r.GetLeftTableName(), r.GetRightTableName(), r.GetMappingTableName(), r.GetCardinality())
        if got != ("parsel", "malik", "parsel_malik", gdal.GRC_MANY_TO_MANY):
            bad.append(f"kentos-profil.gpkg: GDAL'ın ilişkisi {got}")
    ds = None
    c = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    src = sqlite3.connect(f"file:{DIR / 'profil-qgis.gpkg'}?mode=ro", uri=True)
    want = sorted(src.execute("SELECT base_id, related_id FROM parsel_malik"))
    if sorted(c.execute("SELECT base_id, related_id FROM parsel_malik")) != want:
        bad.append("kentos-profil.gpkg: eşlemenin satırları kaynağınkinden farklı")
    docs = [json.loads(t) for (t,) in c.execute(
        "SELECT m.metadata FROM gpkg_metadata m JOIN gpkg_metadata_reference r ON r.md_file_id = m.id "
        "WHERE r.reference_scope = 'column' AND r.table_name = 'parsel' AND r.column_name = 'parsel_malik' "
        "AND m.md_standard_uri = ?", (RELATION_STANDARD,))]
    if docs != [{"mapping": "parsel_malik", "id": "parsel-malik", "name": "parsel_malik", "field": "parsel_malik",
                 "multiple": True, "onDelete": "unlink"}]:
        bad.append(f"kentos-profil.gpkg: KentOS'un belgesi {docs}")
    rows = c.execute("SELECT f_table_name, f_geometry_column, useAsDefault, styleQML, styleSLD FROM layer_styles").fetchall()
    (src_qml,) = src.execute("SELECT styleQML FROM layer_styles WHERE f_table_name = 'parsel'").fetchone()
    c.close()
    src.close()
    if len(rows) != 1 or rows[0][:3] != ("parsel", "geom", 1) or not rows[0][4]:
        bad.append(f"kentos-profil.gpkg: layer_styles {[r[:3] for r in rows]}")
    elif without_ids(qml_cases.read(rows[0][3])[0]) != without_ids(qml_cases.read(src_qml)[0]):
        # The symbol layers' ids are the reader's names: KentOS writes a fill's outline as a line layer of its own.
        bad.append("kentos-profil.gpkg: stilin görünüşü kaynağınkinden farklı")
    if QGIS_LINUX.is_dir():
        py, prefix, env = "/usr/bin/python3", "/usr", dict(os.environ, PYTHONPATH=str(QGIS_LINUX))
    elif (QGIS_MAC / "MacOS/python3.12").exists():
        py, prefix = str(QGIS_MAC / "MacOS/python3.12"), f"{QGIS_MAC}/MacOS"
        env = dict(os.environ, PYTHONHOME=str(QGIS_MAC / "Frameworks"),
                   PYTHONPATH=f"{QGIS_MAC}/Resources/qgis/python:{QGIS_MAC}/Frameworks/lib/python3.12/site-packages")
    else:
        print("QGIS 4.2 yok; QGIS denetimi atlandı")
        return bad
    env["QT_QPA_PLATFORM"] = "offscreen"
    def qgis_reads(p):
        # QGIS opens a GeoPackage for writing (WAL, counters): it reads a copy, and the fixture keeps its bytes (the
        # cases hold its SHA-256).
        import shutil
        import tempfile

        with tempfile.TemporaryDirectory() as d:
            q = Path(d) / Path(p).name
            shutil.copyfile(p, q)
            code = "from qgis.PyQt.QtGui import QColor\n" + QGIS_CHECK.format(prefix=prefix, path=str(q))
            r = subprocess.run([py, "-c", code], env=env, capture_output=True, text=True, timeout=300)
        line = [x for x in r.stdout.splitlines() if x.startswith("{")]
        return json.loads(line[-1]) if line else r.stderr[-400:]

    got, source = qgis_reads(path), qgis_reads(DIR / "profil-qgis.gpkg")
    if isinstance(got, str) or isinstance(source, str):
        return [*bad, f"QGIS okuyamadı: {got if isinstance(got, str) else source}"]
    if not got["valid"] or got["renderer"] != "singleSymbol" or got["color"] != source["color"]:
        bad.append(f"QGIS kentos-profil.gpkg'nin parsellerini {got}, kaynağınkileri {source} okudu")
    # QGIS names a many-to-many relation's tables its own way: the same as in GDAL's file of the same relation.
    if [x[1:] for x in got["relations"]] != [x[1:] for x in source["relations"]]:
        bad.append(f"QGIS kentos-profil.gpkg'de {got['relations']}, kaynakta {source['relations']} ilişkilerini buldu")
    print("kentos-profil.gpkg GDAL'la ve QGIS 4.2'de denetlendi")
    return bad


def build():
    return {
        "format": "kentos.command-cases", "version": 1, "command": "file.import.gpkg", "commandVersion": 1,
        "title": "GeoPackage içe aktar: tablolar katman, satırlar nesne ve kayıt",
        "note": "ADR 0502. Dosyalar GDAL'ın OGR'ıyla okunur (qgis.gpkg QGIS 4.2.3'ün yazdığı); nesneler ADR'nin kuralıyla "
                "burada kurulur: yay bulge'u üç noktasının çemberinden, halkanın kapanan köşesi düşer, NULL öznitelik "
                "yok, 2⁵³'ü aşan tam sayı metin, UTC zaman diliminsiz. Kalıcı kimlik kentos_uids'ten ya da "
                "UUIDv5(gpkg:tablo:fid). Bağımsız başvuru: scripts/fixtures/gpkg_import_cases.py.",
        "setup": SETUP, "cases": cases(),
    }


def main():
    if "--files" in sys.argv:
        write_files()
        return 0
    if "--profile-files" in sys.argv:
        profile_files()
        return 0
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        bad = check_written()
        if bad:
            print("\n".join(bad), file=sys.stderr)
            return 1
        print(f"GeoPackage içe aktarma durumları tutarlı: {len(build()['cases'])} durum")
        return 0
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(build()['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
