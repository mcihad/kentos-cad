#!/usr/bin/env python3
"""Independent reference for the GeoPackage crate (docs/adr/0502, plan/gis.md CBS-03, CBS-44).

    /usr/bin/python3 scripts/fixtures/geopackage_cases.py --qgis   # QGIS 4.2 writes fixtures/geopackage/v1/qgis.gpkg
    /usr/bin/python3 scripts/fixtures/geopackage_cases.py          # writes expected.json from qgis.gpkg (GDAL's OGR)
    /usr/bin/python3 scripts/fixtures/geopackage_cases.py --check  # compares; reads kentos.gpkg the same and validates it

`qgis.gpkg` is a real GeoPackage written by QGIS 4.2.3 (QgsVectorFileWriter): a parcel layer with Turkish names and
values, NULL beside the empty text, a 64-bit id past 2⁵³, a date, a time stamp and a yes/no, two parts and a hole and
a row without geometry; a road with Z and M; points with Z; curves (a compound curve and a curve polygon); an attribute
table without geometry. `expected.json` is what GDAL's OGR reads of it. The Rust side
(crates/native/geopackage/tests/all.rs) must read the same, and, asked with KENTOS_WRITE_GPKG=1, write every layer again
as `kentos.gpkg`; `--check` reads that with OGR, compares it with the same expectation (fids, values, geometry) and runs
GDAL's validate_gpkg.py over it. No KentOS code here.
"""
import json
import os
import re
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/geopackage/v1"
QGIS_FILE = DIR / "qgis.gpkg"
KENTOS_FILE = DIR / "kentos.gpkg"
EXPECTED = DIR / "expected.json"


def write_with_qgis():
    os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
    sys.path.insert(0, "/usr/share/qgis/python")
    from qgis.core import (QgsApplication, QgsFeature, QgsGeometry, QgsProject, QgsVectorFileWriter,
                           QgsVectorLayer)
    from qgis.PyQt.QtCore import QDate, QDateTime, QTime, Qt

    app = QgsApplication([], False)
    app.setPrefixPath("/usr", True)
    app.initQgis()
    if QGIS_FILE.exists():
        QGIS_FILE.unlink()
    layers = [
        ("parsel", "MultiPolygon?crs=EPSG:5256&field=ada:string(40)&field=parsel_no:integer&field=alan:double"
                   "&field=kimlik:int8&field=tescil:date&field=guncelleme:datetime&field=tapulu:boolean&field=not:string",
         [
             ("MULTIPOLYGON(((487000 4420000,487020 4420000,487020 4420020,487000 4420020,487000 4420000),"
              "(487005 4420005,487005 4420010,487010 4420010,487010 4420005,487005 4420005)),"
              "((487030 4420000,487040 4420000,487040 4420010,487030 4420000)))",
              {"ada": "Çağlayan", "parsel_no": 5, "alan": 1234.56, "kimlik": 9007199254740993,
               "tescil": QDate(2024, 2, 29), "guncelleme": QDateTime(QDate(2024, 3, 1), QTime(10, 20, 30), Qt.TimeSpec.UTC),
               "tapulu": True, "not": ""}),
             ("MULTIPOLYGON(((487100 4420000,487110 4420000,487110 4420010,487100 4420010,487100 4420000)))",
              {"ada": "Işık-Ğüşöç", "parsel_no": None, "alan": None, "kimlik": None, "tescil": None,
               "guncelleme": None, "tapulu": False, "not": None}),
             (None, {"ada": "geometrisiz", "parsel_no": 7}),
         ]),
        ("yol", "LineStringZM?crs=EPSG:5256&field=ad:string",
         [("LINESTRING ZM(487000 4420050 10 0,487050 4420050 12 50,487100 4420060 15 101)", {"ad": "Atatürk Cd."}),
          ("LINESTRING ZM(487000 4420070 0 0,487010 4420070 1 10)", {"ad": None})]),
        ("nokta", "PointZ?crs=EPSG:5256&field=ad:string&field=kot:double",
         [("POINT Z(487001.25 4420001.5 101.75)", {"ad": "N1", "kot": 101.75})]),
        ("egri", "CurvePolygon?crs=EPSG:5256&field=ad:string",
         [("CURVEPOLYGON(COMPOUNDCURVE(CIRCULARSTRING(487200 4420000,487210 4420010,487220 4420000),"
           "(487220 4420000,487200 4420000)))", {"ad": "yay ve kiriş"})]),
        ("egri_yol", "CompoundCurve?crs=EPSG:5256&field=ad:string",
         [("COMPOUNDCURVE((487300 4420000,487310 4420000),CIRCULARSTRING(487310 4420000,487315 4420005,487320 4420000))",
           {"ad": "dönemeç"})]),
        ("malik", "None?field=ad:string&field=pay:string&field=parsel:integer",
         [(None, {"ad": "Ayşe Öz", "pay": "1/2", "parsel": 1}), (None, {"ad": "Ömer Çelik", "pay": "1/2", "parsel": 1})]),
    ]
    for name, uri, feats in layers:
        layer = QgsVectorLayer(uri, name, "memory")
        out = []
        for wkt, attrs in feats:
            f = QgsFeature(layer.fields())
            if wkt:
                f.setGeometry(QgsGeometry.fromWkt(wkt))
            for k, v in attrs.items():
                f[k] = v
            out.append(f)
        assert layer.dataProvider().addFeatures(out)
        o = QgsVectorFileWriter.SaveVectorOptions()
        o.driverName = "GPKG"
        o.layerName = name
        if QGIS_FILE.exists():
            o.actionOnExistingFile = QgsVectorFileWriter.ActionOnExistingFile.CreateOrOverwriteLayer
        err = QgsVectorFileWriter.writeAsVectorFormatV3(layer, str(QGIS_FILE), QgsProject.instance().transformContext(), o)
        assert err[0] == QgsVectorFileWriter.WriterError.NoError, err
    print(f"yazıldı: {QGIS_FILE.relative_to(ROOT)} (QGIS 4.2)")


def read_with_ogr(path):
    """Every layer as OGR reads it: name, kind, srs, geometry type and Z/M, fid column, columns, rows."""
    from osgeo import ogr
    ogr.UseExceptions()
    ds = ogr.Open(str(path))
    out = []
    for i in range(ds.GetLayerCount()):
        lyr = ds.GetLayerByIndex(i)
        defn = lyr.GetLayerDefn()
        srs = lyr.GetSpatialRef()
        cols = []
        for j in range(defn.GetFieldCount()):
            fd = defn.GetFieldDefn(j)
            kind = fd.GetTypeName()
            if fd.GetSubType() == ogr.OFSTBoolean:
                kind = "Boolean"
            cols.append({"name": fd.GetName(), "kind": kind})
        rows = []
        for f in lyr:
            g = f.GetGeometryRef()
            values = {}
            for j, c in enumerate(cols):
                if not f.IsFieldSet(j) or f.IsFieldNull(j):
                    values[c["name"]] = None
                elif c["kind"] == "Integer64":
                    values[c["name"]] = str(f.GetFieldAsInteger64(j))
                elif c["kind"] in ("Integer", "Boolean"):
                    values[c["name"]] = f.GetFieldAsInteger(j)
                elif c["kind"] == "Real":
                    values[c["name"]] = f.GetFieldAsDouble(j)
                elif c["kind"] == "Date":
                    y, mo, d = f.GetFieldAsDateTime(j)[:3]
                    values[c["name"]] = f"{y:04d}-{mo:02d}-{d:02d}"
                elif c["kind"] == "DateTime":
                    values[c["name"]] = f.GetFieldAsISO8601DateTime(j)
                else:
                    values[c["name"]] = f.GetFieldAsString(j)
            rows.append({"fid": f.GetFID(), "wkt": g.ExportToIsoWkt() if g is not None else None, "values": values})
        gtype = defn.GetGeomType()
        out.append({
            "name": lyr.GetName(),
            "geometryColumn": lyr.GetGeometryColumn() or None,
            "geometryType": ogr.GeometryTypeToName(gtype) if gtype != ogr.wkbNone else None,
            "srid": int(srs.GetAuthorityCode(None)) if srs is not None and srs.GetAuthorityCode(None) else None,
            "fid": lyr.GetFIDColumn(),
            "columns": cols,
            "rows": rows,
        })
    return out


NUMBER = re.compile(r"-?\d+(?:\.\d+)?(?:[eE][-+]?\d+)?")


def same_wkt(a, b):
    """Two WKTs the same but for the last bits of their numbers (an arc's middle goes out from its bulge)."""
    if NUMBER.sub("#", a) != NUMBER.sub("#", b):
        return False
    return all(abs(float(x) - float(y)) <= 1e-6 for x, y in zip(NUMBER.findall(a), NUMBER.findall(b)))


def qgis_counts(path, names):
    """How many features QGIS 4.2 reads of each table (its own provider), none for a layer it does not open."""
    os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
    sys.path.insert(0, "/usr/share/qgis/python")
    from qgis.core import QgsApplication, QgsVectorLayer
    app = QgsApplication.instance() or QgsApplication([], False)
    app.setPrefixPath("/usr", True)
    app.initQgis()
    out = {}
    for n in names:
        layer = QgsVectorLayer(f"{path}|layername={n}", n, "ogr")
        out[n] = layer.featureCount() if layer.isValid() else None
    return out


def edited_problems(path):
    """The parcels edited in place by the crate (crates/native/geopackage/tests/all.rs): fid 1 gone, fid 2 renamed and
    moved, fid 4 added with its id, a `durum` column; OGR's spatial filter (the R-tree) finds them where they are."""
    from osgeo import ogr
    ds = ogr.Open(str(path))
    lyr = ds.GetLayerByName("parsel")
    out = []
    names = [lyr.GetLayerDefn().GetFieldDefn(i).GetName() for i in range(lyr.GetLayerDefn().GetFieldCount())]
    if names[-1:] != ["durum"]:
        out.append(f"sütunlar {names}")
    rows = {f.GetFID(): f for f in lyr}
    if sorted(rows) != [2, 3, 4]:
        out.append(f"fid'ler {sorted(rows)}")
    elif rows[2].GetField("ada") != "Yeni Mahalle" or rows[2].GetField("durum") != "onaylı":
        out.append("2. satırın değerleri")
    lyr.SetSpatialFilterRect(487299, 4419999, 487306, 4420006)
    found = [f.GetFID() for f in lyr]
    lyr.SetSpatialFilterRect(486999, 4419999, 487021, 4420021)
    gone = [f.GetFID() for f in lyr]
    if found != [2] or gone:
        out.append(f"mekânsal süzgeç {found} {gone}")
    rs = ds.ExecuteSQL("SELECT uid FROM kentos_uids WHERE table_name = 'parsel' AND fid = 4")
    uid = [r.GetField(0) for r in rs]
    ds.ReleaseResultSet(rs)
    if uid != ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7999"]:
        out.append(f"kentos_uids {uid}")
    return out


def synced_problems(path):
    """The bound layer written back (crates/native/application/tests/all/sources.rs): parcel 1 renamed with a new
    `durum` column, parcel 2 deleted, a triangle added with its lasting id, then parcel 3 deleted by another program."""
    from osgeo import ogr
    ds = ogr.Open(str(path))
    lyr = ds.GetLayerByName("parsel")
    out = []
    names = [lyr.GetLayerDefn().GetFieldDefn(i).GetName() for i in range(lyr.GetLayerDefn().GetFieldCount())]
    if names[-1:] != ["durum"]:
        out.append(f"sütunlar {names}")
    rows = {f.GetFID(): f for f in lyr}
    if sorted(rows) != [1, 4]:
        out.append(f"fid'ler {sorted(rows)}")
    else:
        if rows[1].GetField("ada") != "Yeni Mahalle" or rows[1].GetField("durum") != "onaylı":
            out.append("1. satırın değerleri")
        g = rows[4].GetGeometryRef()
        if g is None or abs(g.GetArea() - 50.0) > 1e-9:
            out.append(f"eklenen üçgen {g}")
    lyr.SetSpatialFilterRect(487499, 4419999, 487511, 4420011)
    if [f.GetFID() for f in lyr] != [4]:
        out.append("mekânsal süzgeç eklenen üçgeni bulmuyor")
    rs = ds.ExecuteSQL("SELECT COUNT(*) FROM kentos_uids WHERE table_name = 'parsel' AND fid = 4")
    if [r.GetField(0) for r in rs] != [1]:
        out.append("eklenen üçgenin kalıcı kimliği yok")
    ds.ReleaseResultSet(rs)
    return out


def findings(path):
    """GDAL's validator's findings of a file."""
    from osgeo_utils.samples import validate_gpkg
    try:
        return validate_gpkg.check(str(path), abort_at_first_error=False) or []
    except Exception as e:  # noqa: BLE001  (the validator raises its finding)
        return [str(e)]


def same_meaning(got, want):
    """The drawing's GeoPackage against QGIS's (docs/adr/0502 §5): the same tables, columns, fids, values and
    geometry; an integer column may come back 64-bit and a column past 2⁵³ as text (values compared as text); an empty
    geometry comes back NULL (it was a record)."""
    out = []
    if [l["name"] for l in got] != [l["name"] for l in want]:
        return [f"tablolar {[l['name'] for l in got]} ≠ {[l['name'] for l in want]}"]
    for g, w in zip(got, want):
        if g["geometryType"] != w["geometryType"] or g["srid"] != w["srid"]:
            out.append(f"{w['name']}: {g['geometryType']} {g['srid']} ≠ {w['geometryType']} {w['srid']}")
        if [c["name"] for c in g["columns"]] != [c["name"] for c in w["columns"]]:
            out.append(f"{w['name']}: sütunlar farklı")
        for rg, rw in zip(g["rows"], w["rows"]):
            vg = {k: None if v is None else str(v) for k, v in rg["values"].items()}
            vw = {k: None if v is None else str(v) for k, v in rw["values"].items()}
            if rg["fid"] != rw["fid"] or vg != vw:
                out.append(f"{w['name']} {rw['fid']}: {vg} ≠ {vw}")
            ww = None if rw["wkt"] is not None and rw["wkt"].endswith("EMPTY") else rw["wkt"]
            if (rg["wkt"] is None) != (ww is None) or (ww is not None and not same_wkt(rg["wkt"], ww)):
                out.append(f"{w['name']} {rw['fid']}: {rg['wkt']} ≠ {rw['wkt']}")
        if len(g["rows"]) != len(w["rows"]):
            out.append(f"{w['name']}: {len(g['rows'])} satır ≠ {len(w['rows'])}")
    return out


def main():
    if "--qgis" in sys.argv:
        write_with_qgis()
        return 0
    if "--check" in sys.argv:
        want = json.loads(EXPECTED.read_text("utf-8"))
        got = read_with_ogr(QGIS_FILE)
        if got != want["layers"]:
            print(f"{EXPECTED.relative_to(ROOT)}: qgis.gpkg'nin okunuşuyla aynı değil", file=sys.stderr)
            return 1
        if not KENTOS_FILE.exists():
            print(f"{KENTOS_FILE.relative_to(ROOT)} yok: KENTOS_WRITE_GPKG=1 cargo test -p kentos-geopackage", file=sys.stderr)
            return 1
        back = read_with_ogr(KENTOS_FILE)
        if back != want["layers"]:
            for a, b in zip(back, want["layers"]):
                if a != b:
                    print(json.dumps(a, ensure_ascii=False)[:1500], "\n≠\n", json.dumps(b, ensure_ascii=False)[:1500],
                          file=sys.stderr)
                    break
            print(f"{KENTOS_FILE.relative_to(ROOT)}: OGR'ın okuduğu QGIS'in yazdığıyla aynı değil", file=sys.stderr)
            return 1
        from osgeo_utils.samples import validate_gpkg
        # GDAL 3.12's validator reads the empty flag from bit 3 of the flags, the specification's bit 4 (§2.1.3.1.1;
        # GDAL's writer sets bit 4): every empty geometry is its finding 152, QGIS's own file's too. KentOS's file may
        # have no finding QGIS's lacks.
        extra = [f for f in findings(KENTOS_FILE) if f not in findings(QGIS_FILE)]
        if extra:
            print(f"{KENTOS_FILE.relative_to(ROOT)}: GDAL'ın doğrulayıcısı: {extra}", file=sys.stderr)
            return 1
        exported = DIR / "kentos-export.gpkg"
        if not exported.exists():
            print(f"{exported.relative_to(ROOT)} yok: KENTOS_WRITE_GPKG=1 cargo test -p kentos-headless --test headless "
                  "geopackage", file=sys.stderr)
            return 1
        problems = same_meaning(read_with_ogr(exported), want["layers"])
        names = [l["name"] for l in want["layers"]]
        counts = {l["name"]: len(l["rows"]) for l in want["layers"]}
        opened = qgis_counts(exported, names)
        if opened != counts:
            problems.append(f"QGIS 4.2'nin okuduğu {opened} ≠ {counts}")
        extra = [f for f in findings(exported) if f not in findings(QGIS_FILE)]
        if problems or extra:
            print(f"{exported.relative_to(ROOT)}: " + "; ".join(problems + [str(e) for e in extra]), file=sys.stderr)
            return 1
        edited = DIR / "kentos-edited.gpkg"
        if not edited.exists():
            print(f"{edited.relative_to(ROOT)} yok: KENTOS_WRITE_GPKG=1 cargo test -p kentos-geopackage", file=sys.stderr)
            return 1
        problems += edited_problems(edited)
        synced = DIR / "kentos-synced.gpkg"
        if not synced.exists():
            print(f"{synced.relative_to(ROOT)} yok: KENTOS_WRITE_GPKG=1 cargo test -p kentos-native-application "
                  "--test all sources::", file=sys.stderr)
            return 1
        problems += synced_problems(synced)
        problems += [str(e) for e in findings(synced) if e not in findings(QGIS_FILE)]
        problems += [str(e) for e in findings(edited) if e not in findings(QGIS_FILE)]
        if problems:
            print(f"{edited.relative_to(ROOT)}: " + "; ".join(problems), file=sys.stderr)
            return 1
        print(f"GeoPackage durumları tutarlı: {len(want['layers'])} katman; kentos.gpkg QGIS'inkiyle aynı okunuyor, "
              "kentos-export.gpkg (QGIS'inki çizime alınıp yazılmış) aynı anlamda ve QGIS 4.2 onu açıyor; ikisi de GDAL'ın doğrulayıcısından geçiyor; yerinde düzenlenen kentos-edited.gpkg'yi ve bağlı katmanın yazdığı kentos-synced.gpkg'yi OGR doğru okuyor")
        return 0
    doc = {
        "format": "kentos.geopackage-cases",
        "version": 1,
        "note": "qgis.gpkg QGIS 4.2.3'ün yazdığı gerçek GeoPackage; layers GDAL'ın OGR'ının onu okuyuşu (docs/adr/0502). "
                "Rust okuyucusu aynısını okur, kentos.gpkg aynı okunur ve validate_gpkg.py'den geçer.",
        "layers": read_with_ogr(QGIS_FILE),
    }
    EXPECTED.write_text(json.dumps(doc, ensure_ascii=False, indent=1) + "\n", "utf-8")
    print(f"yazıldı: {EXPECTED.relative_to(ROOT)} ({len(doc['layers'])} katman)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
