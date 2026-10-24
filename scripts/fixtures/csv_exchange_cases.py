#!/usr/bin/env python3
"""CSV alışverişinin ortak durumları ve bağımsız denetimi (docs/adr/0522), KentOS kodu olmadan.

    python3 scripts/fixtures/csv_exchange_cases.py --files   # GDAL'ın (ogr2ogr) CSV'lerini fixtures/csv/v1'e yazar
    python3 scripts/fixtures/csv_exchange_cases.py           # fixtures/commands/v1/file.import.csv.json'u yazar
    python3 scripts/fixtures/csv_exchange_cases.py --check   # yazılı durumlar güncel mi; KentOS'un CSV'leri OGR'da

İçe alma: CSV'leri GDAL QGIS 4.2.3'ün GeoPackage'ından yazar (WKT ya da X/Y, .csvt ve .prj). Durumların beklenenleri
OGR'ın CSV sürücüsünün okuduğundan, GeoPackage alma durumlarının kurallarıyla (gpkg_import_cases.py: noktalar, yaylar
bulge olarak, delikler, parçalar, alanların değerleri) kurulur; satırın kalıcı kimliği UUIDv5(gpkg:csv/ad.csv:satır).

Dışa aktarma: KentOS'un yazdığı CSV'ler (`KENTOS_WRITE_CSV=1 cargo test -p kentos-headless --test headless csv_tables`
yazar, fixtures/csv/v1/out) OGR'la açılır: geometriler kaynağın GeoPackage'ındakilerle ISO WKT olarak aynı, alanlar
türleriyle, sistem EPSG:5256.
"""

import importlib.util
import json
import sys
import uuid
from pathlib import Path

from osgeo import gdal, ogr

ogr.UseExceptions()
gdal.UseExceptions()
sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/csv/v1"
OUT = ROOT / "fixtures/commands/v1/file.import.csv.json"
GPKG = ROOT / "fixtures/geopackage/v1/qgis.gpkg"

spec = importlib.util.spec_from_file_location("gpkg_cases", ROOT / "scripts/fixtures/gpkg_import_cases.py")
G = importlib.util.module_from_spec(spec)
spec.loader.exec_module(G)


def write_files():
    DIR.mkdir(parents=True, exist_ok=True)
    for name, layer, opts in [
        ("parsel.csv", "parsel", ["GEOMETRY=AS_WKT"]),
        ("yol.csv", "yol", ["GEOMETRY=AS_WKT"]),
        ("egri.csv", "egri", ["GEOMETRY=AS_WKT"]),
        ("malik.csv", "malik", []),
    ]:
        for ext in (".csv", ".csvt", ".prj"):
            (DIR / name.replace(".csv", ext)).unlink(missing_ok=True)
        gdal.VectorTranslate(str(DIR / name), str(GPKG), format="CSV", layers=[layer],
                             layerCreationOptions=opts + ["CREATE_CSVT=YES", "LINEFORMAT=CRLF"])
    # Points as X and Y, with Z.
    for ext in (".csv", ".csvt", ".prj"):
        (DIR / ("nokta" + ext)).unlink(missing_ok=True)
    gdal.VectorTranslate(str(DIR / "nokta.csv"), str(GPKG), format="CSV", layers=["nokta"],
                         layerCreationOptions=["GEOMETRY=AS_XYZ", "CREATE_CSVT=YES", "LINEFORMAT=CRLF"])
    # No system: the road's CSV and types without its .prj; a WGS 84 point; a header alone.
    import shutil
    shutil.copy(DIR / "yol.csv", DIR / "sistemsiz.csv")
    shutil.copy(DIR / "yol.csvt", DIR / "sistemsiz.csvt")
    for ext in (".csv", ".csvt", ".prj"):
        (DIR / ("wgs84" + ext)).unlink(missing_ok=True)
    gdal.VectorTranslate(str(DIR / "wgs84.csv"), str(ROOT / "fixtures/geopackage/v1/wgs84.gpkg"), format="CSV",
                         layerCreationOptions=["GEOMETRY=AS_WKT", "CREATE_CSVT=YES", "LINEFORMAT=CRLF"])
    (DIR / "bos.csv").write_text("WKT,ad\r\n", encoding="utf-8")
    (DIR / "bos.csvt").write_text('"WKT","String"\r\n', encoding="utf-8")
    # Hand-written: semicolons and no .csvt (types worked out), a code with leading zeros, a bad WKT, no system.
    (DIR / "elle.csv").write_text(
        "kod;ad;alan;tarih;WKT\r\n"
        "007;Kuyu;12.5;2026-10-19;POINT (487010 4420010)\r\n"
        "008;Çeşme;3;2026-10-20;POINT Z (487020 4420020 105.25)\r\n"
        "009;Bozuk;4;2026-10-21;POINT (487030)\r\n"
        "010;Boş;5;2026-10-22;\r\n", encoding="utf-8")
    (DIR / "bozuk.csv").write_bytes(b"PK\x03\x04 bir zip, CSV degil")
    print(f"CSV'ler yazıldı: {DIR}")


def read_csv(name, geometry=None):
    """A CSV as OGR reads it, in the shape the GeoPackage cases' `read` gives."""
    path = DIR / name
    opts = ["KEEP_GEOM_COLUMNS=NO"]
    if geometry == "xy":
        opts += ["X_POSSIBLE_NAMES=X", "Y_POSSIBLE_NAMES=Y", "Z_POSSIBLE_NAMES=Z"]
    else:
        opts += ["GEOM_POSSIBLE_NAMES=WKT,geom,geometry,the_geom,wkt_geom"]
    if not (DIR / name.replace(".csv", ".csvt")).exists():
        opts += ["AUTODETECT_TYPE=YES", "SEPARATOR=SEMICOLON"]
    ds = gdal.OpenEx(str(path), gdal.OF_VECTOR, open_options=opts)
    lyr = ds.GetLayer(0)
    defn = lyr.GetLayerDefn()
    cols = []
    for j in range(defn.GetFieldCount()):
        fd = defn.GetFieldDefn(j)
        kind = "Boolean" if fd.GetSubType() == ogr.OFSTBoolean else fd.GetTypeName()
        cols.append((fd.GetName(), kind, fd.GetWidth()))
    feats = list(lyr)
    wide = {c[0] for j, c in enumerate(cols) if c[1] in ("Integer", "Integer64")
            and any(f.IsFieldSetAndNotNull(j) and abs(f.GetFieldAsInteger64(j)) > G.MAX_INTEGER for f in feats)}
    rows = []
    for f in feats:
        attrs = {}
        for j, (cname, kind, _) in enumerate(cols):
            if not f.IsFieldSetAndNotNull(j):
                continue
            if kind == "String" and f.GetFieldAsString(j) == "":
                continue
            if kind == "Integer64" or (kind == "Integer" and cname in wide):
                v = f.GetFieldAsInteger64(j)
            elif kind in ("Integer", "Boolean"):
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
            attrs[cname] = G.text(v, kind, cname in wide)
        kind, geo = G.geometry(f.GetGeometryRef())
        line = f.GetFID() + 1
        rows.append({"fid": line, "uid": str(uuid.uuid5(G.NAMESPACE, f"gpkg:csv/{name}:{line}")), "kind": kind,
                     "geometry": geo, "attrs": attrs})
    fields = []
    for cname, kind, width in cols:
        k = {"String": "text", "Integer": "integer", "Integer64": "integer", "Real": "decimal", "Date": "date",
             "DateTime": "datetime", "Boolean": "boolean"}[kind]
        if cname in wide:  # docs/adr/0629 §4
            k = "bigint"
        fd = {"name": cname, "kind": k}
        if kind == "String" and width:
            fd["length"] = width
        fields.append(fd)
    losses = []
    srs = lyr.GetSpatialRef()
    srid = int(srs.GetAuthorityCode(None)) if srs is not None and srs.GetAuthorityName(None) == "EPSG" else None
    data = path.read_bytes()
    import hashlib
    return {"file": name, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest(),
            "tables": [{"name": Path(name).stem, "fields": fields, "rows": rows, "losses": losses, "srid": srid}]}


def imported(name, geometry=None, routes=None, start=2):
    G.read = lambda n: read_csv(n, geometry)
    output, warnings, fields, ids, r = G.imported(name, None, routes, start)
    if "group" in output:
        # The group is named after the file, which is the table's name too: its id is the next free one.
        used = {m["id"] for m in output["layers"]}
        group, k = G.slug(name.removesuffix(".csv")), 2
        base = group
        while group in used:
            group = f"{base}-{k}"
            k += 1
        output["group"] = group
    for w in warnings:
        w["message"] = w["message"]
    return output, warnings, fields, ids


def fx(name):
    return f"$fixture:csv/v1/{name}"


def take(title, name, extra=None, geometry=None, routes=None, op="execute"):
    output, warnings, fields, ids = imported(name, geometry, routes)
    inp = {"path": fx(name), **(extra or {})}
    if op == "plan":
        return G.case(title, G.step(inp, G.done(output, warnings), G.SAME, op="plan"))
    expect = {"ids": [1, *ids], "fields": fields, "revision": "changed", "canUndo": True}
    return G.case(title, G.step(inp, G.done(output, warnings), expect))


def cases():
    out = [
        take("QGIS GeoPackage'ının parselleri GDAL'ın WKT'li CSV'sinden: delikli ve çok parçalı alanlar, alanlar türleriyle",
             "parsel.csv"),
        take("Yaylı ve kotlu yol: CIRCULARSTRING ve COMPOUNDCURVE kesin bulge'lara, Z ve M", "yol.csv"),
        take("Eğri alan (CURVEPOLYGON)", "egri.csv"),
        take("Geometrisiz tablo: her satır kayıt", "malik.csv"),
        take("X, Y ve Z sütunlarından noktalar", "nokta.csv", {"geometry": {"kind": "xy", "x": "X", "y": "Y", "z": "Z"}},
             geometry="xy"),
        take("Plan dosyayı okur, hiçbir şey yazmaz", "parsel.csv", op="plan"),
        take("Satırlar var olan katmana", "yol.csv", {"layer": "cizim"}, routes={"yol": "cizim"}),
        take("Geometri sütunu adıyla verilir", "parsel.csv", {"geometry": {"kind": "wkt", "column": "geom"}}),
    ]
    first = read_csv("yol.csv")["tables"][0]["rows"][0]["uid"]
    out += [
        G.case("Validate yalnız denetler", G.step({"path": fx("parsel.csv")}, G.done(None), op="validate")),
        G.case("Aynı CSV ikinci kez alınmaz",
               take("x", "yol.csv")["steps"][0],
               G.step({"path": fx("yol.csv")},
                      G.failed("entity_exists",
                               f"Dosyadaki bir kaydın kalıcı kimliği ({first}) çizimde zaten bir nesnenin: yol.csv daha "
                               "önce alınmış. Önceki alımı geri alın ya da başka bir çizimde açın.", "path"),
                      {"ids": [1, 2, 3], "revision": "same"})),
        take("Sistem bildirmeyen CSV olduğu gibi alınır (crs_undeclared)", "sistemsiz.csv"),
        G.case("WGS 84'teki CSV çevirme istenmeden alınmaz",
               G.step({"path": fx("wgs84.csv")},
                      G.failed("crs_mismatch", "wgs84.csv EPSG:4326 sisteminde, proje EPSG:5256. Koordinatların "
                                               "projenin sistemine çevrilmesi için reproject verin ya da projeyi "
                                               "EPSG:4326'de açın.", "reproject"))),
        G.case("WGS 84'teki CSV reproject ile çevrilir",
               G.step({"path": fx("wgs84.csv"), "reproject": True},
                      G.done({"file": "wgs84.csv", "bytes": read_csv("wgs84.csv")["bytes"],
                              "sha256": read_csv("wgs84.csv")["sha256"], "srid": 4326, "moved": True, "objects": 1,
                              "tables": [{"table": "wgs84", "layer": "import-wgs84", "read": 1, "objects": 1,
                                          "records": 0, "skipped": 0}],
                              "layers": [{"id": "import-wgs84", "name": "wgs84"}], "group": "import-wgs84-2",
                              "revision": "$current"}),
                      {"ids": [1, 2], "revision": "changed", "canUndo": True,
                       "fields": {"2": {"kind": "point", "layerId": "import-wgs84", "attrs": {"ad": "Kızılay"}}}})),
        G.case("Başlıktan başka satırı olmayan CSV",
               G.step({"path": fx("bos.csv")},
                      G.failed("nothing_to_import", "bos.csv dosyasının seçilen tablolarında alınacak kayıt yok.",
                               "path"))),
        G.case("Hedef katman kilitli",
               G.step({"path": fx("yol.csv"), "layer": "kilitli"},
                      G.failed("layer_locked", "“Kilitli” katmanı kilitli; kilidi Katmanlar panelinden açın ya da "
                                               "başka bir katman verin.", "layers.yol"))),
        G.case("Olmayan dosya", G.step({"path": fx("yok.csv")}, G.failed(
            "file_not_found",
            "$fixture:csv/v1/yok.csv okunamadı: No such file or directory (os error 2). Dosyanın yolunu denetleyin.",
            "path"))),
        G.case("Göreli yol", G.step({"path": "parsel.csv"}, G.failed(
            "invalid_file_path",
            "“parsel.csv” bir dosyanın mutlak yolu değil; okunacak CSV'nin tam yolunu verin (/klasör/ad.csv).",
            "path"))),
        G.case("CSV olmayan dosya", G.step({"path": fx("bozuk.csv")}, G.failed(
            "import_unreadable",
            "$fixture:csv/v1/bozuk.csv CSV olarak okunamadı: Zip arşivi bozuk: sonundaki dizin kaydı bulunamadı. Arşivi "
            "yeniden oluşturup deneyin.", "path"))),
        G.case("Olmayan geometri sütunu", G.step({"path": fx("parsel.csv"), "geometry": {"kind": "wkt", "column": "sekil"}},
                                                 G.failed("import_unreadable",
                                                          "$fixture:csv/v1/parsel.csv CSV olarak okunamadı: “sekil” "
                                                          "sütunu dosyada yok (sütunlar: geom, ada, parsel_no, alan, "
                                                          "kimlik, tescil, guncelleme, tapulu, not)", "path"))),
        G.case("Olmayan katman", G.step({"path": fx("yol.csv"), "layer": "yok"}, G.failed(
            "layer_not_found",
            "“yok” kimlikli katman çizimde yok; “yol” tablosunun kayıtlarını var olan bir katmanın kimliğine yönlendirin.",
            "layers.yol"))),
        G.case("Grup", G.step({"path": fx("yol.csv"), "layer": "grup"}, G.failed(
            "not_a_layer", "“Grup” bir grup; kayıtlar yalnız katmana konur. Grubun altındaki bir katmanı verin.",
            "layers.yol"))),
    ]
    return out


def build():
    return {
        "format": "kentos.command-cases", "version": 1, "command": "file.import.csv", "commandVersion": 1,
        "title": "CSV içe aktar: WKT ya da X/Y, .csvt ve .prj",
        "note": "ADR 0522. CSV'leri GDAL QGIS 4.2.3'ün GeoPackage'ından yazar; beklenenler OGR'ın CSV sürücüsünün "
                "okuduğundan GeoPackage alma durumlarının kuralıyla (gpkg_import_cases.py) kurulur. Bağımsız başvuru: "
                "scripts/fixtures/csv_exchange_cases.py; elle düzenlemeyin.",
        "setup": G.SETUP,
        "cases": cases(),
    }


def check_out():
    """KentOS's CSVs read by OGR: the source's geometries as ISO WKT, the fields' types, the system."""
    out = DIR / "out"
    problems = []
    src = ogr.Open(str(GPKG))
    for name, layer in [("kentos-parsel.csv", "parsel"), ("kentos-yol.csv", "yol"), ("kentos-egri.csv", "egri")]:
        path = out / name
        if not path.exists():
            problems.append(f"{name} yok: önce KENTOS_WRITE_CSV=1 ile KentOS'un testini çalıştırın")
            continue
        ds = gdal.OpenEx(str(path), gdal.OF_VECTOR, open_options=["GEOM_POSSIBLE_NAMES=WKT", "KEEP_GEOM_COLUMNS=NO"])
        lyr = ds.GetLayer(0)
        def geom(f):
            # An empty geometry is a record's, as a missing one (KentOS writes a record's WKT empty).
            g = f.GetGeometryRef()
            return None if g is None or g.IsEmpty() else g.Clone()

        key = lambda g: (g is None, g.ExportToIsoWkt() if g is not None else "")
        want = sorted([geom(f) for f in src.GetLayerByName(layer)], key=key)
        got = sorted([geom(f) for f in lyr], key=key)
        if len(want) != len(got):
            problems.append(f"{name}: {len(got)} satır, beklenen {len(want)}")
        for k, (w, g) in enumerate(zip(want, got)):
            if w is None or g is None:
                if (w is None) != (g is None):
                    problems.append(f"{name} {k}: geometri biri boş")
                continue
            lw, lg = w.GetLinearGeometry(0.0001), g.GetLinearGeometry(0.0001)
            if not (w.Equals(g) or (lw.Within(lg.Buffer(1e-9)) and lg.Within(lw.Buffer(1e-9)))):
                problems.append(f"{name} {k}: geometri farklı: {g.ExportToIsoWkt()[:80]} ↔ {w.ExportToIsoWkt()[:80]}")
        srs = lyr.GetSpatialRef()
        if srs is None or srs.GetAuthorityCode(None) != "5256":
            problems.append(f"{name}: sistem EPSG:5256 değil")
        defn = lyr.GetLayerDefn()
        types = {defn.GetFieldDefn(j).GetName(): defn.GetFieldDefn(j).GetTypeName() for j in range(defn.GetFieldCount())}
        if layer == "parsel" and (types.get("alan") != "Real" or types.get("tescil") != "Date"):
            problems.append(f"{name}: alanların türleri OGR'da {types}")
    return problems


def main():
    if "--files" in sys.argv:
        write_files()
        return
    data = build()
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        problems = []
        if OUT.read_text(encoding="utf-8") != text:
            problems.append(f"{OUT.relative_to(ROOT)} güncel değil")
        problems += check_out()
        if problems:
            sys.exit("\n".join(problems))
        print(f"{len(data['cases'])} durum güncel; KentOS'un CSV'leri OGR'da kaynağın geometrileriyle aynı.")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{len(data['cases'])} durum yazıldı: {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
