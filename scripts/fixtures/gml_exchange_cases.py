#!/usr/bin/env python3
"""GML alışverişinin ortak durumları ve bağımsız denetimi (docs/adr/0545), KentOS kodu olmadan.

    <GDAL'lı python> scripts/fixtures/gml_exchange_cases.py --files   # GDAL'ın GML'lerini fixtures/gml/v1'e yazar
    <GDAL'lı python> scripts/fixtures/gml_exchange_cases.py           # fixtures/commands/v1/file.import.gml.json'u yazar
    <GDAL'lı python> scripts/fixtures/gml_exchange_cases.py --check   # durumlar güncel mi; KentOS'un dosyaları OGR'da

GDAL'lı Python: macOS'ta QGIS 4.2'ninki (PYTHONHOME ve PYTHONPATH QGIS'in Resources'ı), betik kendini onunla yeniden açar.

İçe alma: dosyaları GDAL'ın GML sürücüsü QGIS 4.2.3'ün GeoPackage'ından yazar (GML 3.2, 3.1 ve 2; .xsd'li, .xsd'siz,
.gfs'li; iki nesne türü tek dosyada) ve elle yazılmış GML'ler (featureMembers, wfs:member, http adlı sistem, OGC CRS84,
iç içe özellikler). Durumların beklenenleri OGR'ın GML sürücüsünün okuduğundan, GeoPackage alma durumlarının
kurallarıyla (gpkg_import_cases.py) kurulur; nesnenin kalıcı kimliği UUIDv5(gpkg:gml/ad.gml/tür:sıra), sıra türün
nesnelerinde 1'den. OGR dosyaları belleğe kopyalanıp okunur (sürücü .gfs'i dosyanın yanına yazmasın).

OGR'dan ayrılan iki yer, ADR 0545'te: gml:id bir alan olmaz (OGR gml_id alanı açar); boş öğe `<x></x>` metin alanında boş
metindir (OGR boş değer sayar; boşluğu XML'den ElementTree bulur); .xsd ve .gfs'siz dosyada metin alanları uzunluksuzdur
(OGR en uzun değerin baytını genişlik yapar).

Dışa aktarma: KentOS'un yazdığı dosyalar (`KENTOS_WRITE_GML=1 cargo test -p kentos-headless --test headless gmls`
yazar, fixtures/gml/v1/out) OGR'la açılır: geometriler kaynağın GeoPackage'ındakilerle aynı, alanlar türleriyle,
sistem EPSG:5256 ve WGS 84'te enlem boylam sırası. fixtures/gml/v1/geri'deki dosyalar (KentOS'un okuyup yeniden yazdığı
GDAL dosyaları) OGR'da GDAL'ın kendi dosyalarıyla aynı okunur. XYZ dosyaları (fixtures/xyz/v1/out) kaynağın
geometrilerinin köşelerinden bağımsızca üretilip satır satır karşılaştırılır.
"""

import hashlib
import importlib.util
import json
import os
import re
import shutil
import sys
import uuid
import xml.etree.ElementTree as ET
from pathlib import Path

try:
    from osgeo import gdal, ogr
except ImportError:
    qgis = sorted(Path("/Applications").glob("QGIS*.app/Contents"))
    if not qgis or os.environ.get("KENTOS_QGIS_PYTHON"):
        raise
    res = qgis[-1] / "Resources"
    env = dict(os.environ, KENTOS_QGIS_PYTHON="1", PYTHONHOME=str(res),
               PYTHONPATH=os.pathsep.join(str(res / p) for p in ("python3.12", "python3.12/site-packages",
                                                                 "python3.12/lib-dynload")))
    exe = qgis[-1] / "MacOS" / "python3.12"
    os.execve(str(exe), [str(exe), *sys.argv], env)

ogr.UseExceptions()
gdal.UseExceptions()
sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/gml/v1"
OUT = ROOT / "fixtures/commands/v1/file.import.gml.json"
GPKG = ROOT / "fixtures/geopackage/v1/qgis.gpkg"
WGS = ROOT / "fixtures/geopackage/v1/wgs84.gpkg"

spec = importlib.util.spec_from_file_location("gpkg_cases", ROOT / "scripts/fixtures/gpkg_import_cases.py")
G = importlib.util.module_from_spec(spec)
spec.loader.exec_module(G)

NS = ('xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:ogr="http://ogr.maptools.org/" '
      'xmlns:gml="http://www.opengis.net/gml/3.2"')

HAND = {
    # GML 3.1: featureMembers holds the features of two types; the namespace without a version; a property that is
    # nested (adres/sokak and adres/no flatten to adres_sokak and adres_no).
    "uyeler.gml": f"""<?xml version="1.0" encoding="utf-8" ?>
<ogr:FeatureCollection xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:ogr="http://ogr.maptools.org/" xmlns:gml="http://www.opengis.net/gml">
  <gml:featureMembers>
    <ogr:bina gml:id="bina.1">
      <ogr:geometryProperty><gml:Polygon srsName="urn:ogc:def:crs:EPSG::5256"><gml:exterior><gml:LinearRing><gml:posList>4420000 487000 4420000 487010 4420010 487010 4420010 487000 4420000 487000</gml:posList></gml:LinearRing></gml:exterior></gml:Polygon></ogr:geometryProperty>
      <ogr:ad>Bina A</ogr:ad>
      <ogr:adres><ogr:sokak>Gül Sk.</ogr:sokak><ogr:no>12</ogr:no></ogr:adres>
      <ogr:kat>4</ogr:kat>
    </ogr:bina>
    <ogr:bina gml:id="bina.2">
      <ogr:geometryProperty><gml:Polygon srsName="urn:ogc:def:crs:EPSG::5256"><gml:exterior><gml:LinearRing><gml:posList>4420020 487020 4420020 487030 4420030 487030 4420020 487020</gml:posList></gml:LinearRing></gml:exterior></gml:Polygon></ogr:geometryProperty>
      <ogr:ad>Bina B</ogr:ad>
      <ogr:adres><ogr:sokak>Lale Sk.</ogr:sokak><ogr:no>3</ogr:no></ogr:adres>
      <ogr:kat>2</ogr:kat>
    </ogr:bina>
    <ogr:kapi gml:id="kapi.1">
      <ogr:geometryProperty><gml:Point srsName="urn:ogc:def:crs:EPSG::5256"><gml:pos>4420005 487005</gml:pos></gml:Point></ogr:geometryProperty>
      <ogr:no>12/A</ogr:no>
    </ogr:kapi>
  </gml:featureMembers>
</ogr:FeatureCollection>
""",
    # GML 3.2 as a WFS answers: wfs:member, the system as an http name (EPSG's axis order: north first), nil values,
    # a MultiPoint and a point with an elevation.
    "wfs.gml": """<?xml version="1.0" encoding="UTF-8"?>
<wfs:FeatureCollection xmlns:wfs="http://www.opengis.net/wfs/2.0" xmlns:gml="http://www.opengis.net/gml/3.2" xmlns:ogr="http://ogr.maptools.org/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" numberMatched="3" numberReturned="3">
  <wfs:member>
    <ogr:nirengi gml:id="nirengi.1">
      <ogr:geom><gml:Point gml:id="p1" srsName="http://www.opengis.net/def/crs/EPSG/0/5256"><gml:pos>4420100.5 487100.25 1001.5</gml:pos></gml:Point></ogr:geom>
      <ogr:no>N-1</ogr:no>
      <ogr:derece>1</ogr:derece>
    </ogr:nirengi>
  </wfs:member>
  <wfs:member>
    <ogr:nirengi gml:id="nirengi.2">
      <ogr:geom><gml:Point gml:id="p2" srsName="http://www.opengis.net/def/crs/EPSG/0/5256"><gml:pos>4420200 487200 1002</gml:pos></gml:Point></ogr:geom>
      <ogr:no xsi:nil="true"/>
      <ogr:derece>2</ogr:derece>
    </ogr:nirengi>
  </wfs:member>
  <wfs:member>
    <ogr:nirengi gml:id="nirengi.3">
      <ogr:geom><gml:MultiPoint gml:id="m3" srsName="http://www.opengis.net/def/crs/EPSG/0/5256"><gml:pointMember><gml:Point gml:id="m3.1"><gml:pos>4420300 487300 1003</gml:pos></gml:Point></gml:pointMember><gml:pointMember><gml:Point gml:id="m3.2"><gml:pos>4420310 487310 1004</gml:pos></gml:Point></gml:pointMember></gml:MultiPoint></ogr:geom>
      <ogr:no>N-3</ogr:no>
      <ogr:derece>3</ogr:derece>
    </ogr:nirengi>
  </wfs:member>
</wfs:FeatureCollection>
""",
    # OGC CRS84 names longitude first, whatever EPSG says of 4326.
    "crs84.gml": f"""<?xml version="1.0" encoding="utf-8" ?>
<ogr:FeatureCollection gml:id="c" {NS}>
  <gml:featureMember>
    <ogr:yer gml:id="yer.0">
      <ogr:geom><gml:Point srsName="urn:ogc:def:crs:OGC:1.3:CRS84" gml:id="g0"><gml:pos>32.8541 39.9208</gml:pos></gml:Point></ogr:geom>
      <ogr:ad>Kızılay</ogr:ad>
    </ogr:yer>
  </gml:featureMember>
</ogr:FeatureCollection>
""".replace("<gml:featureMember>", "<gml:featureMember>"),
    # Lines and a MultiCurve of GML 3.1 with an Arc segment and a compound of three pieces.
    "yaylar.gml": f"""<?xml version="1.0" encoding="utf-8" ?>
<ogr:FeatureCollection gml:id="c" {NS}>
  <gml:featureMember>
    <ogr:kenar gml:id="kenar.0">
      <ogr:geom><gml:Curve srsName="urn:ogc:def:crs:EPSG::5256" gml:id="g0"><gml:segments><gml:LineStringSegment><gml:posList>4420000 487000 4420000 487010</gml:posList></gml:LineStringSegment><gml:Arc><gml:posList>4420000 487010 4420005 487015 4420010 487010</gml:posList></gml:Arc><gml:LineStringSegment><gml:posList>4420010 487010 4420010 487000</gml:posList></gml:LineStringSegment></gml:segments></gml:Curve></ogr:geom>
      <ogr:ad>üç kesim</ogr:ad>
    </ogr:kenar>
  </gml:featureMember>
  <gml:featureMember>
    <ogr:kenar gml:id="kenar.1">
      <ogr:geom><gml:MultiCurve srsName="urn:ogc:def:crs:EPSG::5256" gml:id="g1"><gml:curveMember><gml:LineString gml:id="g1.0"><gml:posList>4420020 487000 4420020 487010</gml:posList></gml:LineString></gml:curveMember><gml:curveMember><gml:Curve gml:id="g1.1"><gml:segments><gml:ArcString><gml:posList>4420030 487000 4420035 487005 4420030 487010</gml:posList></gml:ArcString></gml:segments></gml:Curve></gml:curveMember></gml:MultiCurve></ogr:geom>
      <ogr:ad>çok eğri</ogr:ad>
    </ogr:kenar>
  </gml:featureMember>
</ogr:FeatureCollection>
""",
    # No system named: the numbers are taken as the project's, east first.
    "sistemsiz.gml": f"""<?xml version="1.0" encoding="utf-8" ?>
<ogr:FeatureCollection gml:id="c" {NS}>
  <gml:featureMember>
    <ogr:iz gml:id="iz.0">
      <ogr:geom><gml:LineString gml:id="g0"><gml:posList srsDimension="2">487000 4420000 487010 4420010</gml:posList></gml:LineString></ogr:geom>
      <ogr:ad>sistemsiz</ogr:ad>
    </ogr:iz>
  </gml:featureMember>
</ogr:FeatureCollection>
""",
    # A collection with no member.
    "bos.gml": f"""<?xml version="1.0" encoding="utf-8" ?>
<ogr:FeatureCollection gml:id="c" {NS}>
</ogr:FeatureCollection>
""",
    # A geometry collection has no object of the drawing's: the feature is left out and said.
    "tuhaf.gml": f"""<?xml version="1.0" encoding="utf-8" ?>
<ogr:FeatureCollection gml:id="c" {NS}>
  <gml:featureMember>
    <ogr:tuhaf gml:id="tuhaf.0">
      <ogr:geom><gml:Polygon srsName="urn:ogc:def:crs:EPSG::5256" gml:id="g0"><gml:exterior><gml:LinearRing><gml:posList srsDimension="2">4420000 487000 4420000 487010 4420010 487010 4420000 487000</gml:posList></gml:LinearRing></gml:exterior></gml:Polygon></ogr:geom>
      <ogr:ad>düz</ogr:ad>
    </ogr:tuhaf>
  </gml:featureMember>
  <gml:featureMember>
    <ogr:tuhaf gml:id="tuhaf.1">
      <ogr:geom><gml:MultiGeometry srsName="urn:ogc:def:crs:EPSG::5256" gml:id="g1"><gml:geometryMember><gml:Point gml:id="g1.0"><gml:pos>4420005 487005</gml:pos></gml:Point></gml:geometryMember><gml:geometryMember><gml:LineString gml:id="g1.1"><gml:posList srsDimension="2">4420000 487000 4420010 487010</gml:posList></gml:LineString></gml:geometryMember></gml:MultiGeometry></ogr:geom>
      <ogr:ad>karışık</ogr:ad>
    </ogr:tuhaf>
  </gml:featureMember>
</ogr:FeatureCollection>
""",
    # Not GML: a well-formed XML without a collection, text, and a document cut in the middle of an element.
    "xml-degil.gml": "<?xml version=\"1.0\"?><kitap><baslik>GML değil</baslik></kitap>\n",
    "bozuk.gml": "PK\x03\x04 bir zip, GML degil",
}


def fx(name):
    return f"$fixture:gml/v1/{name}"


def translate(dst, src, layers, fmt="GML3.2", **kw):
    for ext in ("", ".xsd", ".gfs"):
        p = Path(str(dst)[:-4] + ext) if ext else Path(dst)
        p.unlink(missing_ok=True)
    gdal.VectorTranslate(str(dst), str(src), format="GML", layers=layers,
                         datasetCreationOptions=[f"FORMAT={fmt}"], **kw)


def write_files():
    DIR.mkdir(parents=True, exist_ok=True)
    for p in DIR.glob("*"):
        if p.is_file():
            p.unlink()
    for name, layer in [("parsel", "parsel"), ("yol", "yol"), ("nokta", "nokta"), ("egri", "egri"),
                        ("egri_yol", "egri_yol"), ("malik", "malik")]:
        translate(DIR / f"{name}.gml", GPKG, [layer])
    translate(DIR / "yer.gml", WGS, ["yer"])
    translate(DIR / "v31-parsel.gml", GPKG, ["parsel"], "GML3")
    translate(DIR / "v31-egri.gml", GPKG, ["egri"], "GML3")
    translate(DIR / "v2-parsel.gml", GPKG, ["parsel"], "GML2")
    translate(DIR / "v2-egri.gml", GPKG, ["egri"], "GML2")
    # The same GML without its schema: the types come from the values; and with the .gfs OGR writes from them.
    shutil.copy(DIR / "parsel.gml", DIR / "parsel-sema-yok.gml")
    shutil.copy(DIR / "parsel.gml", DIR / "parsel-gfs.gml")
    for name in ("parsel-sema-yok", "parsel-gfs"):
        for ext in (".xsd", ".gfs"):
            (DIR / f"{name}{ext}").unlink(missing_ok=True)
    (DIR / "parsel-sema-yok.xsd").unlink(missing_ok=True)
    mem_read(DIR / "parsel-gfs.gml", keep=True)
    for name, text in HAND.items():
        (DIR / name).write_bytes(text.encode())
    (DIR / "kesik.gml").write_bytes((DIR / "parsel.gml").read_bytes()[:900])
    print(f"GML'ler yazıldı: {DIR}")


def mem_read(path, keep=False):
    """The file's OGR dataset, from a copy in memory (the driver writes a .gfs beside a file it reads without a
    schema); with `keep` the .gfs it wrote is saved beside the file."""
    base = f"/vsimem/{uuid.uuid4().hex}"
    path = Path(path)
    names = []
    for ext in (".gml", ".xsd", ".gfs"):
        src = path.with_suffix(ext)
        if src.exists():
            gdal.FileFromMemBuffer(f"{base}{ext}", src.read_bytes())
            names.append(f"{base}{ext}")
    ds = ogr.Open(f"{base}.gml")
    # Scan the layers whole: the .gfs is written when the scan is done.
    for i in range(ds.GetLayerCount()):
        for _ in ds.GetLayer(i):
            pass
    if keep:
        ds = None
        f = gdal.VSIFOpenL(f"{base}.gfs", "rb")
        if f:
            gdal.VSIFSeekL(f, 0, 2)
            size = gdal.VSIFTellL(f)
            gdal.VSIFSeekL(f, 0, 0)
            data = gdal.VSIFReadL(1, size, f)
            gdal.VSIFCloseL(f)
            path.with_suffix(".gfs").write_bytes(data)
        return None
    return ds, names


def nested_names(path):
    """The names a property nested one level deep gets in KentOS: outer_inner (OGR names it by the inner alone
    when that is free): {(type, inner): outer_inner}."""
    out = {}
    try:
        root = ET.parse(path).getroot()
    except ET.ParseError:
        return out
    local = lambda e: e.tag.rsplit("}", 1)[-1]
    for el in root.iter():
        if local(el) == "featureMembers":
            feats = list(el)
        elif local(el) in ("featureMember", "member"):
            feats = list(el)[:1]
        else:
            continue
        for f in feats:
            for prop in f:
                kids = [k for k in prop if not k.tag.startswith("{http://www.opengis.net/gml")]
                if kids and not any(k.tag.startswith("{http://www.opengis.net/gml") for k in prop):
                    for k in kids:
                        out[(local(f), local(k))] = f"{local(prop)}_{local(k)}"
    return out


def empty_texts(path):
    """The empty property elements of a file, in document order, by feature type: (type, ordinal, property). OGR
    reads an empty element as a null; in a text column it is an empty text (ADR 0545)."""
    out = set()
    try:
        root = ET.parse(path).getroot()
    except ET.ParseError:
        return out
    counts = {}
    for el in root.iter():
        kids = list(el)
        # A feature: a child of featureMember / member, or of featureMembers.
        name = el.tag.rsplit("}", 1)[-1]
        if name in ("featureMember", "member"):
            feats = kids[:1]
        elif name == "featureMembers":
            feats = kids
        else:
            continue
        for f in feats:
            t = f.tag.rsplit("}", 1)[-1]
            counts[t] = counts.get(t, 0) + 1
            for prop in f:
                pn = prop.tag.rsplit("}", 1)[-1]
                nil = prop.attrib.get("{http://www.w3.org/2001/XMLSchema-instance}nil") in ("true", "1")
                if len(prop) == 0 and not nil and (prop.text or "") == "" and pn != "boundedBy":
                    out.add((t, counts[t], pn))
    return out


# OGR gives OGC CRS84 no EPSG code; it is WGS 84 with longitude first, the data axes of EPSG:4326.
SRID = {"crs84.gml": 4326}

XSD_TYPED = {"parsel", "yol", "nokta", "egri", "egri_yol", "malik", "yer", "v31-parsel", "v31-egri",
             "v2-parsel", "v2-egri"}


def read_gml(name):
    """Every feature type of a file as the GeoPackage cases' `read` gives a table: fields as layer fields, rows as
    (fid, uid, kind, geometry fields, attrs), losses."""
    path = DIR / name
    ds, _ = mem_read(path)
    stem = path.stem
    typed = path.with_suffix(".xsd").exists() or path.with_suffix(".gfs").exists()
    empties = empty_texts(path)
    renamed = nested_names(path)
    tables = []
    for i in range(ds.GetLayerCount()):
        lyr = ds.GetLayerByIndex(i)
        defn = lyr.GetLayerDefn()
        cols = []
        for j in range(defn.GetFieldCount()):
            fd = defn.GetFieldDefn(j)
            # gml_id is no field of ours; without a schema OGR makes fields of the envelope's corners too.
            if fd.GetName() in ("gml_id", "lowerCorner", "upperCorner", "fid"):
                continue
            kind = "Boolean" if fd.GetSubType() == ogr.OFSTBoolean else fd.GetTypeName()
            cols.append((j, renamed.get((lyr.GetName(), fd.GetName()), fd.GetName()), kind, fd.GetWidth()))
        feats = list(lyr)
        wide = {c[1] for c in cols if c[2] in ("Integer", "Integer64")
                and any(f.IsFieldSetAndNotNull(c[0]) and abs(f.GetFieldAsInteger64(c[0])) > G.MAX_INTEGER
                        for f in feats)}
        fields = []
        for _, cname, kind, width in cols:
            k = {"String": "text", "Integer": "integer", "Integer64": "integer", "Real": "decimal", "Date": "date",
                 "DateTime": "datetime", "Boolean": "boolean"}[kind]
            if cname in wide:  # docs/adr/0629 §4
                k = "bigint"
            f = {"name": cname, "kind": k}
            if kind == "String" and width and typed:
                f["length"] = width
            fields.append(f)
        losses = []
        rows = []
        skipped = []
        for n, f in enumerate(feats, start=1):
            attrs = {}
            for j, cname, kind, _ in cols:
                if not f.IsFieldSetAndNotNull(j):
                    if kind == "String" and (lyr.GetName(), n, cname) in empties:
                        attrs[cname] = ""
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
            if f.GetGeometryRef() is not None and f.GetGeometryRef().GetGeometryName() == "GEOMETRYCOLLECTION":
                skipped.append((n, "GEOMETRYCOLLECTION geometrisinin çizimde karşılığı yok"))
                continue
            kind, geo = geometry(f.GetGeometryRef())
            uid = str(uuid.uuid5(G.NAMESPACE, f"gpkg:gml/{name}/{lyr.GetName()}:{n}"))
            rows.append({"fid": n, "uid": uid, "kind": kind, "geometry": geo, "attrs": attrs})
        if skipped:
            losses.append(f"{len(skipped)} satır alınmadı ({lyr.GetName()} {skipped[0][0]}: {skipped[0][1]})")
        srs = lyr.GetSpatialRef()
        srid = int(srs.GetAuthorityCode(None)) if srs is not None and srs.GetAuthorityName(None) == "EPSG" else None
        srid = SRID.get(name, srid)
        tables.append({"name": lyr.GetName(), "fields": fields, "rows": rows, "losses": losses, "srid": srid,
                       "skipped": len(skipped)})
    data = path.read_bytes()
    return {"file": name, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest(), "tables": tables}


def geometry(g):
    """G.geometry, and the multi points and multi lines (parts of the first)."""
    if g is None or g.IsEmpty():
        return "record", {}
    n = g.GetGeometryName()
    if n == "MULTIPOINT":
        pts = [G.points(g.GetGeometryRef(i))[0] for i in range(g.GetGeometryCount())]

        def one(p):
            o = {"p": G.P(p[0], p[1])}
            if p[2] is not None:
                o["z"] = p[2]
            return o

        out = one(pts[0])
        if len(pts) > 1:
            out["parts"] = [one(p) for p in pts[1:]]
        return "point", out
    if n in ("MULTILINESTRING", "MULTICURVE"):
        parts = [G.fields_of(*G.walk(g.GetGeometryRef(i))[:2], True) for i in range(g.GetGeometryCount())]
        out = dict(parts[0])
        if len(parts) > 1:
            out["parts"] = parts[1:]
        return "polyline", out
    return G.geometry(g)


def imported(name, tables=None, routes=None):
    """The command's answer for a file, GeoPackage import's rules over the rows OGR reads (G.imported)."""
    G.read = read_gml
    output, warnings, fields, ids, _ = G.imported(name, tables, routes)
    r = read_gml(name)
    skipped = {t["name"]: t["skipped"] for t in r["tables"]}
    for t in output["tables"]:
        t["read"] += skipped.get(t["table"], 0)
        t["skipped"] = skipped.get(t["table"], 0)
    if "group" in output:
        # The group is named after the file; when a layer took that id the next free one.
        used = {m["id"] for m in output["layers"]}
        group, k = G.slug(name.removesuffix(".gml")), 2
        base = group
        while group in used:
            group = f"{base}-{k}"
            k += 1
        output["group"] = group
    return output, warnings, fields, ids


def take(title, name, extra=None, routes=None, tables=None, op="execute"):
    output, warnings, fields, ids = imported(name, tables, routes)
    inp = {"path": fx(name), **(extra or {})}
    if op == "plan":
        return G.case(title, G.step(inp, G.done(output, warnings), G.SAME, op="plan"))
    expect = {"ids": [1, *ids], "fields": fields, "revision": "changed", "canUndo": True}
    return G.case(title, G.step(inp, G.done(output, warnings), expect))


def unreadable(name, why):
    return G.case(f"Okunamayan dosya: {name}", G.step({"path": fx(name)}, G.failed(
        "import_unreadable", f"{fx(name)} GML olarak okunamadı: {why}", "path")))


BROKEN = "XML belgesi değil ya da bozuk"
NOT_A_COLLECTION = "GML özellik topluluğu değil (featureMember, featureMembers ya da member yok)"


def well_formed(name):
    try:
        ET.parse(DIR / name)
        return True
    except ET.ParseError:
        return False


def cases():
    # The documents the unreadable cases name are as said: broken XML for two, well-formed XML with no collection.
    assert not well_formed("bozuk.gml") and not well_formed("kesik.gml") and well_formed("xml-degil.gml")
    root = ET.parse(DIR / "xml-degil.gml").getroot()
    assert root.tag == "kitap" and len(list(root)) == 1
    out = [
        take("GDAL'ın GML 3.2 parselleri .xsd'siyle: çok parçalı alan, delik, geometrisiz kayıt, alanlar türleriyle "
             "ve uzunluklarıyla, boş öğe boş metin, xsi:nil boş değer", "parsel.gml"),
        take("Plan dosyayı okur, hiçbir şey yazmaz", "parsel.gml", op="plan"),
        take("Şema yok: türler değerlerden (tam sayı, ondalık, tarih, tarih-saat, evet/hayır); metin alanı uzunluksuz",
             "parsel-sema-yok.gml"),
        take("Şema yok, .gfs var: türler ve uzunluklar .gfs'den", "parsel-gfs.gml"),
        take("GML 3.1.1 (geometrilerde gml:id yok)", "v31-parsel.gml"),
        take("GML 2: coordinates ve outerBoundaryIs, EPSG:5256 doğu önce", "v2-parsel.gml"),
        take("GML 2'de yaylar kirişlerle yazılmış: kirişler olarak gelir", "v2-egri.gml"),
        take("Yaylı yol: CompositeCurve, ArcString kesin", "egri_yol.gml"),
        take("Eğri alan: Ring'in Curve'ü ve çizgisi", "egri.gml"),
        take("GML 3.1.1 eğri alan", "v31-egri.gml"),
        take("Kotlu yol: srsDimension 3", "yol.gml"),
        take("Kotlu nokta: pos'ta üç sayı", "nokta.gml"),
        take("Geometrisiz nesne türü: her nesne kayıt", "malik.gml"),
        take("featureMembers'ta iki nesne türü, iç içe özellik outer_inner alan olur", "uyeler.gml"),
        take("Yalnız seçilen nesne türü: tables", "uyeler.gml", {"tables": ["kapi"]}, tables=["kapi"]),
        take("wfs:member, http adlı sistem (EPSG'nin kuzey önce sırası), xsi:nil, çok nokta", "wfs.gml"),
        take("Eğrinin üç kesimi (çizgi, Arc, çizgi) ve çok eğri", "yaylar.gml"),
        take("Çizim biçimi karışık: geometri koleksiyonu çizimde karşılığı olmadığından atlanır, söylenir", "tuhaf.gml"),
        take("Nesneler var olan katmana", "yol.gml", {"layers": {"yol": "cizim"}}, routes={"yol": "cizim"}),
        take("Sistem bildirmeyen dosya olduğu gibi alınır (crs_undeclared)", "sistemsiz.gml"),
    ]
    first = read_gml("yol.gml")["tables"][0]["rows"][0]["uid"]
    w = read_gml("yer.gml")
    out += [
        G.case("Validate yalnız denetler", G.step({"path": fx("parsel.gml")}, G.done(None), op="validate")),
        G.case("Aynı dosya ikinci kez alınmaz",
               take("x", "yol.gml")["steps"][0],
               G.step({"path": fx("yol.gml")},
                      G.failed("entity_exists",
                               f"Dosyadaki bir kaydın kalıcı kimliği ({first}) çizimde zaten bir nesnenin: yol.gml daha "
                               "önce alınmış. Önceki alımı geri alın ya da başka bir çizimde açın.", "path"),
                      {"ids": [1, 2, 3], "revision": "same"})),
        G.case("WGS 84'teki dosya (enlem boylam sırasıyla) çevirme istenmeden alınmaz",
               G.step({"path": fx("yer.gml")},
                      G.failed("crs_mismatch", "yer.gml EPSG:4326 sisteminde, proje EPSG:5256. Koordinatların "
                                               "projenin sistemine çevrilmesi için reproject verin ya da projeyi "
                                               "EPSG:4326'de açın.", "reproject"))),
        G.case("WGS 84'teki dosya reproject ile çevrilir",
               G.step({"path": fx("yer.gml"), "reproject": True},
                      G.done({"file": "yer.gml", "bytes": w["bytes"], "sha256": w["sha256"], "srid": 4326,
                              "moved": True, "objects": 1,
                              "tables": [{"table": "yer", "layer": "import-yer", "read": 1, "objects": 1,
                                          "records": 0, "skipped": 0}],
                              "layers": [{"id": "import-yer", "name": "yer"}], "group": "import-yer-2",
                              "revision": "$current"}),
                      {"ids": [1, 2], "revision": "changed", "canUndo": True,
                       "fields": {"2": {"kind": "point", "layerId": "import-yer", "attrs": {"ad": "Kızılay"}}}})),
        G.case("OGC CRS84 (boylam önce) WGS 84'tür: çevirme istenmeden alınmaz",
               G.step({"path": fx("crs84.gml")},
                      G.failed("crs_mismatch", "crs84.gml EPSG:4326 sisteminde, proje EPSG:5256. Koordinatların "
                                               "projenin sistemine çevrilmesi için reproject verin ya da projeyi "
                                               "EPSG:4326'de açın.", "reproject"))),
        G.case("Nesnesi olmayan koleksiyon",
               G.step({"path": fx("bos.gml")},
                      G.failed("nothing_to_import", "bos.gml dosyasının seçilen tablolarında alınacak kayıt yok.",
                               "path"))),
        G.case("Dosyada olmayan nesne türü", G.step({"path": fx("uyeler.gml"), "tables": ["yok"]}, G.failed(
            "table_not_found", "“yok” tablosu dosyada yok; dosyanın nesne türlerinden birinin adını verin.",
            "tables[0]"))),
        G.case("Hedef katman kilitli",
               G.step({"path": fx("yol.gml"), "layers": {"yol": "kilitli"}},
                      G.failed("layer_locked", "“Kilitli” katmanı kilitli; kilidi Katmanlar panelinden açın ya da "
                                               "başka bir katman verin.", "layers.yol"))),
        G.case("Olmayan dosya", G.step({"path": fx("yok.gml")}, G.failed(
            "file_not_found",
            "$fixture:gml/v1/yok.gml okunamadı: No such file or directory (os error 2). Dosyanın yolunu denetleyin.",
            "path"))),
        G.case("Göreli yol", G.step({"path": "parsel.gml"}, G.failed(
            "invalid_file_path",
            "“parsel.gml” bir dosyanın mutlak yolu değil; okunacak GML dosyasının tam yolunu verin (/klasör/ad.gml).",
            "path"))),
        unreadable("bozuk.gml", BROKEN),
        unreadable("kesik.gml", BROKEN),
        unreadable("xml-degil.gml", NOT_A_COLLECTION),
        G.case("Olmayan katman", G.step({"path": fx("yol.gml"), "layers": {"yol": "yok"}}, G.failed(
            "layer_not_found",
            "“yok” kimlikli katman çizimde yok; “yol” tablosunun kayıtlarını var olan bir katmanın kimliğine yönlendirin.",
            "layers.yol"))),
        G.case("Grup", G.step({"path": fx("yol.gml"), "layers": {"yol": "grup"}}, G.failed(
            "not_a_layer", "“Grup” bir grup; kayıtlar yalnız katmana konur. Grubun altındaki bir katmanı verin.",
            "layers.yol"))),
    ]
    return out


def build():
    return {
        "format": "kentos.command-cases", "version": 1, "command": "file.import.gml", "commandVersion": 1,
        "title": "GML içe aktar: 3.2, 3.1 ve 2, şemalı ve şemasız, yaylar, Z, eksen sırası, sistem",
        "note": "ADR 0545. Dosyaları GDAL'ın GML sürücüsü QGIS 4.2.3'ün GeoPackage'ından yazar, kalanı elle; "
                "beklenenler OGR'ın GML sürücüsünün okuduğundan GeoPackage alma durumlarının kuralıyla "
                "(gpkg_import_cases.py) kurulur. Bağımsız başvuru: scripts/fixtures/gml_exchange_cases.py; "
                "elle düzenlemeyin.",
        "setup": G.SETUP,
        "cases": cases(),
    }


# ── KentOS's files ──────────────────────────────────────────────────────────

def same_geometry(w, g):
    if w is None or g is None:
        return (w is None) == (g is None)
    # A file with a layer of elevations promotes the curves of the others to Z (OGR); the source says.
    if not w.Is3D():
        g = g.Clone()
        g.Set3D(False)
    try:
        if w.Equals(g):
            return True
        lw, lg = w.GetLinearGeometry(0.0001), g.GetLinearGeometry(0.0001)
        return lw.Within(lg.Buffer(1e-9)) and lg.Within(lw.Buffer(1e-9))
    except RuntimeError:
        return False


def unmatched(want, got):
    """The geometries of `want` no geometry of `got` equals, each matched once."""
    left = list(got)
    missing = []
    for w in want:
        k = next((i for i, g in enumerate(left) if same_geometry(w, g)), None)
        if k is None:
            missing.append(w)
        else:
            left.pop(k)
    return missing, left


def wkt(g):
    return g.ExportToIsoWkt()[:80] if g is not None else None


def check_gml():
    """KentOS's GML read by OGR: the source's geometries, the fields' types, the system and its axis order."""
    out = DIR / "out"
    problems = []
    gdal.PushErrorHandler("CPLQuietErrorHandler")
    sources = {"qgis": ogr.Open(str(GPKG)), "wgs": ogr.Open(str(WGS))}

    def geom(f):
        g = f.GetGeometryRef()
        return None if g is None or g.IsEmpty() else g.Clone()

    layers = ["parsel", "yol", "nokta", "egri", "egri_yol"]
    for name in [*layers, "hepsi", "yer"]:
        path = out / f"kentos-{name}.gml"
        if not path.exists() or not path.with_suffix(".xsd").exists():
            problems.append(f"{path.name} yok: önce KENTOS_WRITE_GML=1 ile KentOS'un testini çalıştırın")
            continue
        ds, _ = mem_read(path)
        src = sources["wgs" if name == "yer" else "qgis"]
        want_layers = ["yer"] if name == "yer" else (layers + ["malik"] if name == "hepsi" else [name])
        if ds.GetLayerCount() != len(want_layers):
            problems.append(f"{path.name}: OGR {ds.GetLayerCount()} katman okudu, {len(want_layers)} olmalı")
            continue
        for lname in want_layers:
            lyr = ds.GetLayerByName(lname)
            if lyr is None:
                problems.append(f"{path.name}: {lname} nesne türü yok")
                continue
            want = [geom(f) for f in src.GetLayerByName(lname)]
            got = [geom(f) for f in lyr]
            missing, extra = unmatched(want, got)
            for w in missing:
                problems.append(f"{path.name}/{lname}: kaynağın geometrisi yok: {wkt(w)}")
            for g in extra:
                problems.append(f"{path.name}/{lname}: fazla geometri: {wkt(g)}")
            srs = lyr.GetSpatialRef()
            code = "4326" if name == "yer" else "5256"
            if lname != "malik" and (srs is None or srs.GetAuthorityCode(None) != code):
                problems.append(f"{path.name}/{lname}: sistem EPSG:{code} değil")
            # Fields and values against the source's, by name.
            sdef, gdef = src.GetLayerByName(lname).GetLayerDefn(), lyr.GetLayerDefn()
            for j in range(sdef.GetFieldCount()):
                sf = sdef.GetFieldDefn(j)
                k = gdef.GetFieldIndex(sf.GetName())
                if k < 0:
                    problems.append(f"{path.name}/{lname}: {sf.GetName()} alanı yok")
                    continue
                kinds = {"Integer": ("Integer", "Integer64"), "Integer64": ("Integer", "Integer64"),
                         "Real": ("Real",), "String": ("String",), "Date": ("Date",), "DateTime": ("DateTime",)}
                gk = gdef.GetFieldDefn(k)
                have = "Boolean" if gk.GetSubType() == ogr.OFSTBoolean else gk.GetTypeName()
                if sf.GetSubType() == ogr.OFSTBoolean:
                    ok = have == "Boolean"
                elif sf.GetName() == "kimlik":
                    ok = have in ("Integer64", "String")
                else:
                    ok = have in kinds.get(sf.GetTypeName(), (sf.GetTypeName(),))
                if not ok:
                    problems.append(f"{path.name}/{lname}: {sf.GetName()} alanı OGR'da {have}, kaynakta {sf.GetTypeName()}")
            for sfeat, gfeat in zip(src.GetLayerByName(lname), lyr):
                for j in range(sdef.GetFieldCount()):
                    fn = sdef.GetFieldDefn(j).GetName()
                    k = gdef.GetFieldIndex(fn)
                    if k < 0:
                        continue
                    a = sfeat.GetField(j) if sfeat.IsFieldSetAndNotNull(j) else None
                    b = gfeat.GetField(k) if gfeat.IsFieldSetAndNotNull(k) else None
                    if isinstance(a, str) and a == "":
                        a = None
                    if str(a) != str(b) and not (a is not None and isinstance(a, (int, float)) and float(a) == float(b)):
                        # Dates and times read in OGR's own text; they are compared as its fields.
                        if sdef.GetFieldDefn(j).GetTypeName() in ("Date", "DateTime"):
                            if sfeat.GetFieldAsString(j).replace("/", "-")[:10] == gfeat.GetFieldAsString(k).replace("/", "-")[:10]:
                                continue
                        problems.append(f"{path.name}/{lname}: {fn} değeri {a!r}, OGR'da {b!r}")
    # The GDAL files KentOS read and wrote again read in OGR as GDAL's own.
    back = DIR / "geri"
    skip = ("bozuk", "xml-degil", "kesik", "crs84", "bos", "tuhaf")  # tuhaf: a feature KentOS leaves out
    for p in sorted(DIR.glob("*.gml")):
        if p.stem in skip:
            continue
        again = back / p.name
        if not again.exists():
            problems.append(f"geri/{p.name} yok: KENTOS_WRITE_GML=1 cargo test -p kentos-headless --test headless gmls")
            continue

        def one_part(g):
            """A multi geometry of one part is that part: KentOS keeps no multi of one."""
            if g is None:
                return ""
            if g.GetGeometryName() in ("MULTIPOLYGON", "MULTISURFACE", "MULTILINESTRING", "MULTICURVE") \
                    and g.GetGeometryCount() == 1:
                g = g.GetGeometryRef(0)
            return g.ExportToIsoWkt()

        def dump(path):
            ds, _ = mem_read(path)
            res = []
            for i in range(ds.GetLayerCount()):
                lyr = ds.GetLayer(i)
                d = lyr.GetLayerDefn()
                keep = [j for j in range(d.GetFieldCount())
                        if d.GetFieldDefn(j).GetName() not in ("gml_id", "fid", "lowerCorner", "upperCorner")]

                def value(f, j):
                    if not f.IsFieldSetAndNotNull(j):
                        return None
                    if d.GetFieldDefn(j).GetTypeName() in ("Date", "DateTime"):
                        return f.GetFieldAsString(j).replace("/", "-")[:19]
                    return str(f.GetField(j))

                rows = sorted((one_part(f.GetGeometryRef()), [value(f, j) for j in keep]) for f in lyr)
                res.append((lyr.GetName(), len(keep), str(rows)))
            return res

        if dump(p) != dump(again):
            problems.append(f"geri/{p.name}: OGR'da GDAL'ın dosyasından farklı")
    return problems


# ── XYZ ─────────────────────────────────────────────────────────────────────

# What the test writes (crates/native/headless/tests/headless.rs, xyz_files_go_out): layers, vertices, separator,
# decimals, header, without Z.
XYZ = {
    "nokta": (["nokta"], "points", " ", None, False, "zero"),
    "yol": (["yol"], "all", ",", 3, True, "zero"),
    "parsel": (["parsel"], "all", " ", None, False, "zero"),
    "egri": (["egri"], "all", ";", None, False, "zero"),
    "egri_yol": (["egri_yol"], "all", "\t", None, False, "zero"),
    "hepsi": (None, "all", " ", None, False, "zero"),
    "hepsi-kotlu": (None, "all", " ", None, False, "skip"),
}


def xyz_curve(g):
    """The vertices of a curve: a line's, a circular string's end points, a compound's pieces end to end (a piece
    starting where the last ended is not written twice); a closed curve's last vertex is not repeated."""
    n = g.GetGeometryName()
    out = []

    def push(p):
        if out and out[-1][0] == p[0] and out[-1][1] == p[1]:
            return
        out.append(p)

    def add(c):
        k = c.GetGeometryName()
        if k in ("LINESTRING", "LINEARRING"):
            for p in G.points(c):
                push(p)
        elif k == "CIRCULARSTRING":
            for p in G.points(c)[0::2]:
                push(p)
        elif k == "COMPOUNDCURVE":
            for i in range(c.GetGeometryCount()):
                add(c.GetGeometryRef(i))
        else:
            raise ValueError(k)

    add(g)
    if len(out) > 2 and out[0][0] == out[-1][0] and out[0][1] == out[-1][1]:
        out.pop()
    return out, n


def xyz_vertices(g, mode):
    if g is None or g.IsEmpty():
        return None
    n = g.GetGeometryName()
    if n == "POINT":
        return list(G.points(g))
    if n == "MULTIPOINT":
        return [G.points(g.GetGeometryRef(i))[0] for i in range(g.GetGeometryCount())]
    if mode == "points":
        return None
    if n in ("LINESTRING", "CIRCULARSTRING", "COMPOUNDCURVE"):
        return xyz_curve(g)[0]
    if n in ("POLYGON", "CURVEPOLYGON"):
        return [p for i in range(g.GetGeometryCount()) for p in xyz_curve(g.GetGeometryRef(i))[0]]
    if n in ("MULTIPOLYGON", "MULTISURFACE", "MULTILINESTRING", "MULTICURVE"):
        out = []
        for i in range(g.GetGeometryCount()):
            out += xyz_vertices(g.GetGeometryRef(i), mode)
        return out
    raise ValueError(n)


def xyz_num(v, decimals):
    if decimals is None:
        s = repr(float(v))
        if s.endswith(".0"):
            s = s[:-2]
    else:
        s = f"{v:.{decimals}f}"
    if s.startswith("-") and set(s[1:]) <= set("0."):
        s = s[1:]
    return s


def xyz_expected(layers, mode, sep, decimals, header, without_z):
    src = ogr.Open(str(GPKG))
    names = layers or ["parsel", "yol", "nokta", "egri", "egri_yol", "malik"]
    lines = [sep.join("XYZ")] if header else []
    for ln in names:
        for f in src.GetLayerByName(ln):
            vs = xyz_vertices(f.GetGeometryRef(), mode)
            for p in vs or []:
                z = p[2]
                if z is None:
                    if without_z == "skip":
                        continue
                    z = 0.0
                lines.append(sep.join(xyz_num(v, decimals) for v in (p[0], p[1], z)))
    return "\n".join(lines) + "\n"


def check_xyz():
    problems = []
    out = ROOT / "fixtures/xyz/v1/out"
    for name, spec in XYZ.items():
        path = out / f"kentos-{name}.xyz"
        if not path.exists():
            problems.append(f"{path.name} yok: önce KENTOS_WRITE_XYZ=1 ile KentOS'un testini çalıştırın")
            continue
        want = xyz_expected(*spec)
        got = path.read_text(encoding="utf-8")
        if want != got:
            a, b = want.splitlines(), got.splitlines()
            at = next((i for i, (x, y) in enumerate(zip(a, b)) if x != y), min(len(a), len(b)))
            problems.append(f"{path.name}: satır {at + 1}: beklenen {a[at] if at < len(a) else None!r}, "
                            f"yazılan {b[at] if at < len(b) else None!r} (beklenen {len(a)} satır, yazılan {len(b)})")
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
        problems += check_gml()
        problems += check_xyz()
        if problems:
            sys.exit("\n".join(problems))
        print(f"{len(data['cases'])} durum güncel; KentOS'un GML'leri OGR'da kaynağın geometrileriyle, XYZ'leri "
              "bağımsız köşe listesiyle aynı.")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{len(data['cases'])} durum yazıldı: {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
