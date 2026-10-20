#!/usr/bin/env python3
"""GDAL's side of the shared cases of file.import.shp, .geojson and .kml (docs/adr/0563), without KentOS code.

    python3 scripts/fixtures/gis_import_ogr.py files                 # GDAL writes fixtures/gis-import/v1
    python3 scripts/fixtures/gis_import_ogr.py vertices REL...       # the vertices OGR finds in each fixture file (JSON)
    python3 scripts/fixtures/gis_import_ogr.py original              # the QGIS GeoPackage's parcels' vertices (JSON)

GDAL'lı Python: macOS'ta QGIS 4.2'ninki; betik kendini onunla yeniden açar (scripts/fixtures/fgb_exchange_cases.py gibi).
`scripts/fixtures/gis_import_cases.py` (pyproj'lu Python) bunu çağırır: dosyaları GDAL yazar, her dosyayı OGR bir daha
okur ve bulduğu köşeler başvuruların okuduklarıyla karşılaştırılır.
"""
import io
import json
import os
import sys
import zipfile
from pathlib import Path

try:
    from osgeo import gdal, ogr
except ImportError:
    qgis = sorted(Path("/Applications").glob("QGIS*.app/Contents"))
    if not qgis or os.environ.get("KENTOS_QGIS_PYTHON"):
        raise
    res = qgis[-1] / "Resources"
    env = dict(os.environ, KENTOS_QGIS_PYTHON="1", PYTHONHOME=str(res), PROJ_DATA=str(res / "qgis/proj"),
               GDAL_DATA=str(res / "qgis/gdal"),
               PYTHONPATH=os.pathsep.join(str(res / p) for p in ("python3.12", "python3.12/site-packages",
                                                                 "python3.12/lib-dynload")))
    exe = qgis[-1] / "MacOS" / "python3.12"
    os.execve(str(exe), [str(exe), *sys.argv], env)

ogr.UseExceptions()
gdal.UseExceptions()
sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/gis-import/v1"
QGIS_GPKG = ROOT / "fixtures/geopackage/v1/qgis.gpkg"


def zip_of(entries):
    out = io.BytesIO()
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        for name, data in entries:
            info = zipfile.ZipInfo(name, date_time=(2026, 10, 21, 12, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o644 << 16
            z.writestr(info, data)
    return out.getvalue()


def write_files():
    """GDAL's files: the QGIS GeoPackage's tables as a Shapefile, a GeoJSON file and a KML, in TUREF TM36 and WGS 84."""
    DIR.mkdir(parents=True, exist_ok=True)
    for p in DIR.glob("*"):
        if p.is_file():
            p.unlink()
    src = str(QGIS_GPKG)
    for layer in ("parsel", "yol", "nokta"):
        gdal.VectorTranslate(str(DIR / f"{layer}.shp"), src, format="ESRI Shapefile", layers=[layer])
    # The same parcels in WGS 84 (OGR moves them by PROJ), with a UTF-8 code page.
    gdal.VectorTranslate(str(DIR / "parsel-wgs84.shp"), src, format="ESRI Shapefile", layers=["parsel"],
                         dstSRS="EPSG:4326", reproject=True, layerCreationOptions=["ENCODING=UTF-8"])
    gdal.VectorTranslate(str(DIR / "nokta-wgs84.shp"), src, format="ESRI Shapefile", layers=["nokta"],
                         dstSRS="EPSG:4326", reproject=True)
    # No .prj: the file says nothing of its system.
    gdal.VectorTranslate(str(DIR / "sistemsiz.shp"), src, format="ESRI Shapefile", layers=["nokta"])
    (DIR / "sistemsiz.prj").unlink()
    # An empty layer.
    gdal.VectorTranslate(str(DIR / "bos.shp"), src, format="ESRI Shapefile", layers=["nokta"], where="1 = 0")

    def parts(stem, folder=""):
        return [(f"{folder}{stem}{ext}", (DIR / f"{stem}{ext}").read_bytes())
                for ext in (".shp", ".shx", ".dbf", ".prj", ".cpg") if (DIR / f"{stem}{ext}").exists()]

    # Zip archives: one layer, three layers in a folder, two layers of one name.
    (DIR / "parsel.zip").write_bytes(zip_of(parts("parsel")))
    (DIR / "katmanlar.zip").write_bytes(zip_of(parts("parsel", "veri/") + parts("yol", "veri/") + parts("nokta", "veri/")))
    (DIR / "ayni-ad.zip").write_bytes(zip_of(parts("parsel", "a/") + parts("parsel", "b/")))
    (DIR / "bozuk.shp").write_bytes(b"Bu bir Shapefile degil.\n")
    (DIR / "bozuk.zip").write_bytes(b"PK\x03\x04 bu bir zip degil")
    (DIR / "bos.zip").write_bytes(zip_of([("not.txt", b"Shapefile yok")]))
    # GeoJSON: RFC 7946 (WGS 84, moved by OGR) and the `crs` member (TM36).
    gdal.VectorTranslate(str(DIR / "parsel-rfc7946.geojson"), src, format="GeoJSON", layers=["parsel"],
                         layerCreationOptions=["RFC7946=YES"])
    gdal.VectorTranslate(str(DIR / "parsel-tm36.geojson"), src, format="GeoJSON", layers=["parsel"],
                         layerCreationOptions=["RFC7946=NO"])
    gdal.VectorTranslate(str(DIR / "yol-tm36.geojson"), src, format="GeoJSON", layers=["yol"],
                         layerCreationOptions=["RFC7946=NO"])
    gdal.VectorTranslate(str(DIR / "nokta-rfc7946.geojson"), src, format="GeoJSON", layers=["nokta"],
                         layerCreationOptions=["RFC7946=YES"])
    (DIR / "bos.geojson").write_text('{"type": "FeatureCollection", "features": []}\n', encoding="utf-8")
    (DIR / "bozuk.geojson").write_text('{"type": "FeatureCollection", "features": [\n', encoding="utf-8")

    def one_point(crs_name, name):
        return json.dumps({
            "type": "FeatureCollection", "name": name,
            "crs": {"type": "name", "properties": {"name": crs_name}},
            "features": [{"type": "Feature", "properties": {"ad": "a"},
                          "geometry": {"type": "Point", "coordinates": [486512.25, 4420187.5]}}]},
            ensure_ascii=False) + "\n"

    (DIR / "belirsiz-crs.geojson").write_text(one_point("yerel şantiye sistemi", "belirsiz"), encoding="utf-8")
    (DIR / "bilinmeyen-epsg.geojson").write_text(one_point("urn:ogc:def:crs:EPSG::9999", "bilinmeyen"),
                                                 encoding="utf-8")
    # A file that names its layers the way KentOS writes them.
    ring = lambda pts: [[x, y] for x, y in pts + [pts[0]]]
    (DIR / "katmanli.geojson").write_text(json.dumps({
        "type": "FeatureCollection", "name": "katmanli",
        "crs": {"type": "name", "properties": {"name": "urn:ogc:def:crs:EPSG::5256"}},
        "features": [
            {"type": "Feature", "properties": {"ada": 101, "parsel": 5}, "kentos": {"layer": "Parsel", "label": "101/5"},
             "geometry": {"type": "Polygon", "coordinates": [ring([(486512.5, 4420187.25), (486562.5, 4420187.25),
                                                                   (486562.5, 4420237.25), (486512.5, 4420237.25)])]}},
            {"type": "Feature", "properties": {"kat": 4}, "kentos": {"layer": "Bina", "label": "B-1"},
             "geometry": {"type": "Polygon", "coordinates": [ring([(486520.0, 4420195.0), (486530.0, 4420195.0),
                                                                   (486530.0, 4420205.0)])]}},
            {"type": "Feature", "properties": {"ad": "kapı"},
             "geometry": {"type": "Point", "coordinates": [486550.0, 4420215.5, 101.25]}},
        ]}, ensure_ascii=False) + "\n", encoding="utf-8")
    # KML and KMZ by LIBKML (OGR moves the coordinates to WGS 84 itself).
    gdal.VectorTranslate(str(DIR / "parsel.kml"), src, format="LIBKML", layers=["parsel"])
    gdal.VectorTranslate(str(DIR / "nokta.kml"), src, format="LIBKML", layers=["nokta"])
    gdal.VectorTranslate(str(DIR / "yol.kml"), src, format="LIBKML", layers=["yol"])
    # One doc.kml with a folder for each layer (without SINGLE_DOC LIBKML writes NetworkLinks to a file for each layer).
    # The three layers as folders of one doc.kml, which is zipped into the KMZ here.
    out = ogr.GetDriverByName("LIBKML").CreateDataSource(str(DIR / "hepsi.kml"), ["SINGLE_DOC=YES"])
    source = ogr.Open(src)
    for layer in ("parsel", "yol", "nokta"):
        out.CopyLayer(source.GetLayerByName(layer), layer)
    out = None
    source = None
    (DIR / "hepsi.kmz").write_bytes(zip_of([("doc.kml", (DIR / "hepsi.kml").read_bytes())]))
    (DIR / "bos.kml").write_text('<?xml version="1.0" encoding="UTF-8"?>\n<kml xmlns="http://www.opengis.net/kml/2.2">'
                                 '<Document><name>Bos</name></Document></kml>\n', encoding="utf-8")
    print(f"GDAL'ın dosyaları yazıldı: {DIR.relative_to(ROOT)} ({len(list(DIR.iterdir()))} dosya)")


def ogr_vertices(geom, acc):
    if geom is None:
        return
    n = geom.GetGeometryCount()
    if n:
        for i in range(n):
            ogr_vertices(geom.GetGeometryRef(i), acc)
    else:
        for i in range(geom.GetPointCount()):
            p = geom.GetPoint_2D(i)
            acc.add((round(p[0], 9), round(p[1], 9)))


def vertices_of_file(rel):
    """The vertices OGR finds in `fixtures/<rel>`: a zip's every .shp, a KMZ's doc.kml."""
    path = ROOT / "fixtures" / rel
    ext = path.suffix.lower()
    if ext == ".zip":
        sources = [f"/vsizip/{path}/{n}" for n in zipfile.ZipFile(path).namelist() if n.lower().endswith(".shp")]
    elif ext == ".kmz":
        sources = [f"/vsizip/{path}/doc.kml"]
    else:
        sources = [str(path)]
    acc = set()
    for s in sources:
        ds = ogr.Open(s)
        for i in range(ds.GetLayerCount()):
            for f in ds.GetLayerByIndex(i):
                ogr_vertices(f.GetGeometryRef(), acc)
    return sorted(acc)


def main():
    cmd = sys.argv[1] if len(sys.argv) > 1 else ""
    if cmd == "files":
        write_files()
    elif cmd == "vertices":
        out = {}
        for rel in sys.argv[2:]:
            try:
                out[rel] = vertices_of_file(rel)
            except Exception as e:  # noqa: BLE001
                out[rel] = f"hata: {e}"
        print(json.dumps(out))
    elif cmd == "original":
        acc = set()
        ds = ogr.Open(str(QGIS_GPKG))
        for f in ds.GetLayerByName("parsel"):
            g = f.GetGeometryRef()
            if g is not None and not g.IsEmpty():
                ogr_vertices(g, acc)
        print(json.dumps(sorted(acc)))
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
