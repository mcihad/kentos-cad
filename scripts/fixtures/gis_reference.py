"""Independent reference writer of the GeoJSON and Shapefile import fixtures (fixtures/formats/v1/gis).

Writes fixtures/formats/v1/gis with Python's standard library only, never with
KentOS code. Every GeoJSON file is written here by hand as text, so its number
lexemes (`1.50`, `1e3`, `-0.0`, `1e999`), escapes, member order and repeated
keys are exactly the ones the fixture means; every Shapefile set is packed byte
by byte with `struct` from the ESRI Shapefile Technical Description (July 1998)
and dBASE III+. `<name>.expected.json` is what tools/formats/gis.py (the
independent reader) reads from the fixture by the shared reading rules; the
Rust reader in crates/shared/formats must read every fixture to the same
objects, float for float, and the same declared SRID and encoding.

The reader's output is not its own proof: SUMMARY below says, derived by hand
from the rules alone, what every fixture must give (each object's kind in
order and most of their coordinates, layers, labels and attributes), and every
Shapefile set is parsed again here on its own (headers, extents, record and
index offsets, the table) and its rings' orientation and holes checked with
exact rational arithmetic.

Three zip archives hold Shapefile sets as portals hand them out (ARCHIVES,
docs/adr/0053), written with Python's zipfile. Deflate's bytes depend on the
zlib that wrote them, so the archives are not compared byte for byte: `--check`
opens each with zipfile (which checks every CRC) and compares its members'
names, methods and bytes with the files built here.

    python3 scripts/fixtures/gis_reference.py           # writes the fixtures
    python3 scripts/fixtures/gis_reference.py --check   # writes nothing; compares and reads

`--check` rebuilds every file in memory and compares it with the one on disk,
then checks the files on disk as above, reads every fixture with the reader
(through its file-path entry, as the CLI does) and compares the outcome with its
expected file and with SUMMARY. It also checks the Rust GeoJSON writer's output
in export/ (each `<name>.geojson` against the `<name>.input.json` it was written
from), reading it back with the same reader; the files there are not written here.
"""
import io
import json
import math
import struct
import sys
import zipfile
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/formats/v1/gis"
sys.dont_write_bytecode = True  # no __pycache__ in the tree
sys.path.insert(0, str(ROOT / "tools/formats"))
import gis  # noqa: E402  (the independent reader)

# ── GeoJSON, written by hand ────────────────────────────────────────────
# `@u` below is a JSON \u escape: the writer puts the backslash in (json_text), because
# some editing tools turn a typed backslash-u escape of a printable character into the
# character itself, which would silently take the escapes out of the fixture. No other
# `@` appears in the texts; every other escape (\n, \", \\, \/, \t) is typed as is.

GEOJSON = {}

# WGS 84 longitude, latitude inside Türkiye; the top-level name (written last) is the default layer.
# The last two members of `features` give nothing: a bare geometry, and a Feature without `geometry`.
GEOJSON["features"] = r'''{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "id": 1,
      "properties": {
        "ad": "@u00c7ankaya @u0130l@u00e7esi @u015eehit @u00d6@u011fretmen I@u015f@u0131k @u00dcst@u00fcn Soka@u011F@u0131",
        "no": 12,
        "oran": 1.50,
        "bin": 1e3,
        "sifir": -0.0,
        "durum": "eski",
        "aktif": true,
        "kapali": false,
        "bos": null,
        "detay": { "kat": 3, "cephe": "g@u00fcney", "alan": 120.750, "isaret": "@ud83d@udccd", "ic": { "x": null, "y": [ true, false ] } },
        "notlar": [ "ilk sat@u0131r\nikinci \"s@u00f6z\"", "a\/b\tc@u0001d\\e", 2, -1.5E-3 ],
        "durum": "yeni"
      },
      "geometry": { "type": "Point", "coordinates": [32.8597, 39.9334, 938.5] }
    },
    {
      "type": "Feature",
      "properties": { "ad": "Kad@u0131k@u00f6y", "bos_metin": "", "bos_nesne": {}, "bos_dizi": [], "eksi_sifir": -0, "buyuk": 123456789012345678901234567890, "pi": 3.14159265358979323846264338327950288 },
      "geometry": { "type": "MultiPoint", "coordinates": [[28.9784, 41.0082, 39.25], [29.0277, 41.0422]] }
    },
    {"type":"Feature","properties":{"tur":"yol"},"geometry":{"type":"LineString","coordinates":[[27.1428,38.4237],[27.2,38.46]]}},
    {
      "geometry": { "coordinates": [[30.7133, 36.8969], [30.72, 36.9, 12.5], [30.7288, 36.8841], [30.74, 36.89]], "type": "LineString" },
      "properties": null,
      "type": "Feature"
    },
    {
      "type": "Feature",
      "properties": { "tur": "dere" },
      "geometry": {
        "type": "MultiLineString",
        "coordinates": [
          [[35.4787, 38.7312], [35.49, 38.74]],
          [[35.5, 38.75], [35.51, 38.76], [35.52, 38.755]]
        ]
      }
    },
    {
      "type": "Feature",
      "properties": { "tur": "park" },
      "geometry": {
        "type": "Polygon",
        "coordinates": [
          [[32.40, 37.80], [32.60, 37.80], [32.60, 37.95], [32.40, 37.95], [32.40, 37.80]],
          [[32.45, 37.85], [32.50, 37.90], [32.55, 37.85], [32.45, 37.85]]
        ]
      }
    },
    {
      "type": "Feature",
      "properties": { "tur": "ada" },
      "geometry": {
        "type": "MultiPolygon",
        "coordinates": [
          [[[39.70, 40.98], [39.74, 40.98], [39.74, 41.02], [39.70, 41.02], [39.70, 40.98]]],
          [[[41.25, 39.88], [41.30, 39.88], [41.32, 39.90], [41.30, 39.92], [41.25, 39.92]]]
        ]
      }
    },
    {
      "type": "Feature",
      "properties": { "tur": "koleksiyon" },
      "geometry": {
        "type": "GeometryCollection",
        "geometries": [
          { "type": "Point", "coordinates": [43.3833, 38.4942] },
          { "type": "LineString", "coordinates": [[43.38, 38.49], [43.40, 38.50], [43.42, 38.505]] }
        ]
      }
    },
    { "type": "Feature", "properties": { "ad": "geometrisiz" }, "geometry": null },
    { "type": "Feature", "geometry": { "type": "LineString", "coordinates": [[34.0, 38.0]] } },
    {
      "type": "Feature",
      "kentos": { "layer": 5, "label": null },
      "properties": { "tur": "dort sayi" },
      "geometry": { "type": "Point", "coordinates": [34.5, 39.1, 1100.5, 7.0] }
    },
    {
      "kentos": { "layer": "Sınır", "label": "P-12" },
      "type": "Feature",
      "properties": { "ada": "12", "parsel": 7 },
      "geometry": { "type": "Polygon", "coordinates": [[[33.0, 39.0], [33.1, 39.0], [33.1, 39.1], [33.0, 39.1], [33.0, 39.0]]] }
    },
    {
      "type": "Feature",
      "kentos": { "label": "K-1", "renk": "#c00" },
      "properties": { "tur": "sadece etiket" },
      "geometry": { "type": "LineString", "coordinates": [[36.0, 37.0], [36.1, 37.05]] }
    },
    {
      "type": "Feature",
      "properties": { "tur": "bilinmeyen" },
      "geometry": { "type": "Circle", "coordinates": [37.0, 38.0], "radius": 50.0 }
    },
    {
      "type": "Feature",
      "properties": { "tur": "ic ice" },
      "geometry": {
        "type": "GeometryCollection",
        "geometries": [
          { "type": "GeometryCollection", "geometries": [ { "type": "Point", "coordinates": [38.0, 37.5, -12.25] } ] },
          { "type": "Circle", "coordinates": [38.1, 37.6], "radius": 5.0 }
        ]
      }
    },
    {
      "type": "Feature",
      "kentos": { "layer": "", "label": "" },
      "properties": { "tur": "bos kentos" },
      "geometry": { "type": "LineString", "coordinates": [[36.5, 37.5], [36.6, 37.55]] }
    },
    { "type": "Point", "coordinates": [37.5, 38.5] },
    { "type": "Feature", "properties": { "tur": "geometri uyesi yok" } }
  ],
  "name": "ornek"
}
'''

# TUREF / TM36 (EPSG:5256) eastings and northings with many decimals, one past float64's precision.
GEOJSON["tm"] = r'''{
  "type": "FeatureCollection",
  "crs": { "type": "name", "properties": { "name": "urn:ogc:def:crs:EPSG::5256" } },
  "features": [
    {
      "type": "Feature",
      "kentos": { "layer": "Parsel", "label": "101/5" },
      "properties": { "ada": "101", "parsel": "5", "alan": 2345.678, "mahalle": "Çamlıbel" },
      "geometry": {
        "type": "Polygon",
        "coordinates": [
          [[486512.3456789, 4420187.12345678], [486562.3456789, 4420187.12345678], [486562.34567891234, 4420237.123456789012], [486512.3456789, 4420237.12345678], [486512.3456789, 4420187.12345678]],
          [[486530.001, 4420200.002], [486530.007, 4420210.008], [486540.005, 4420210.006], [486540.003, 4420200.004], [486530.001, 4420200.002]]
        ]
      }
    },
    {
      "type": "Feature",
      "kentos": { "layer": "Bina", "label": "B-1" },
      "properties": { "kat": 4, "yapi": "betonarme" },
      "geometry": { "type": "Polygon", "coordinates": [[[486545.125, 4420215.25], [486555.875, 4420215.25], [486555.875, 4420228.5], [486545.125, 4420228.5], [486545.125, 4420215.25]]] }
    },
    {
      "type": "Feature",
      "properties": { "tur": "poligon noktas@u0131" },
      "geometry": { "type": "MultiPoint", "coordinates": [[486512.3456789, 4420187.12345678, 1021.345], [486562.3456789, 4420187.12345678, 1019.8765]] }
    },
    {
      "type": "Feature",
      "kentos": { "layer": "Bina" },
      "properties": { "ad": "kap@u0131" },
      "geometry": { "type": "Point", "coordinates": [4.8655e5, 4.4202155e6] }
    }
  ]
}
'''

# A bare geometry: no Feature, no crs, no name. The outline's last position repeats the first's x and y
# with another z; the first hole shrinks to two positions once closed.
GEOJSON["bare-polygon"] = r'''{
  "type": "Polygon",
  "coordinates": [
    [[29.10, 40.20, 100.0], [29.20, 40.20, 101.5], [29.20, 40.30, 102.25], [29.10, 40.30, 100.75], [29.10, 40.20, 99.0]],
    [[29.12, 40.22], [29.13, 40.23], [29.12, 40.22]],
    [[29.15, 40.25], [29.15, 40.28], [29.18, 40.28], [29.18, 40.25], [29.15, 40.25]]
  ]
}
'''

# A single Feature with the legacy {"type": "EPSG"} crs: ED50 / TM30 (EPSG:2320).
GEOJSON["feature-epsg"] = r'''{
  "type": "Feature",
  "crs": { "type": "EPSG", "properties": { "code": 2320 } },
  "properties": { "yol": "D-100", "serit": 3 },
  "geometry": { "type": "LineString", "coordinates": [[415678.901, 4541234.567], [415800.25, 4541300.125], [416012.5, 4541288.75]] }
}
'''

# OGC CRS84 by name; an empty top-level name leaves the default layer to the caller.
GEOJSON["crs84"] = r'''{
  "type": "FeatureCollection",
  "name": "",
  "crs": { "type": "name", "properties": { "name": "urn:ogc:def:crs:OGC:1.3:CRS84" } },
  "features": [
    { "type": "Feature", "properties": { "ad": "An@u0131tkabir" }, "geometry": { "type": "Point", "coordinates": [32.8369, 39.9253] } }
  ]
}
'''

# Numbers past float64 (1e999, -1e999) drop the smallest unit holding them; 1e-999 is a finite 0.0.
# So do positions that are not arrays of at least two numbers, and geometry members that are not
# geometries; a number after the third is ignored even when it is not finite.
GEOJSON["nonfinite"] = r'''{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "properties": { "ad": "coklu nokta", "buyuk": 1e999 },
      "geometry": { "type": "MultiPoint", "coordinates": [[30.0, 40.0], [1e999, 40.1], [30.2, 40.2, 1e-999]] }
    },
    {
      "type": "Feature",
      "properties": { "ad": "cizgi" },
      "geometry": { "type": "LineString", "coordinates": [[30.0, 40.0], [30.1, -1e999], [30.2, 40.2]] }
    },
    {
      "type": "Feature",
      "properties": { "ad": "koleksiyon" },
      "geometry": {
        "type": "GeometryCollection",
        "geometries": [
          { "type": "Point", "coordinates": [31.0, 40.5, 1e999] },
          { "type": "LineString", "coordinates": [[31.0, 40.5], [31.1, 40.6]] }
        ]
      }
    },
    {
      "type": "Feature",
      "properties": { "ad": "coklu cizgi" },
      "geometry": { "type": "MultiLineString", "coordinates": [[[32.0, 41.0], [32.1, 1e999]], [[32.2, 41.2], [32.3, 41.3], [32.4, 41.2]]] }
    },
    {
      "type": "Feature",
      "properties": { "ad": "cokgen" },
      "geometry": {
        "type": "Polygon",
        "coordinates": [
          [[33.0, 40.0], [33.2, 40.0], [33.2, 40.2], [33.0, 40.2], [33.0, 40.0]],
          [[33.05, 40.05], [33.1, -1e999], [33.15, 40.05], [33.05, 40.05]]
        ]
      }
    },
    {
      "type": "Feature",
      "properties": { "ad": "coklu cokgen" },
      "geometry": {
        "type": "MultiPolygon",
        "coordinates": [
          [[[34.0, 40.0], [1e999, 40.0], [34.2, 40.2], [34.0, 40.0]]],
          [[[34.5, 40.5], [34.7, 40.5], [34.7, 40.7], [34.5, 40.5]]]
        ]
      }
    },
    {
      "type": "Feature",
      "properties": { "ad": "nokta" },
      "geometry": { "type": "Point", "coordinates": [35.0, 39.5] }
    },
    {
      "type": "Feature",
      "properties": { "ad": "dorduncu sayi" },
      "geometry": { "type": "Point", "coordinates": [35.5, 39.6, 12.5, 1e999] }
    },
    {
      "type": "Feature",
      "properties": { "ad": "bozuk konumlar" },
      "geometry": { "type": "MultiPoint", "coordinates": [[36.0, 40.0], [true, 40.1], [36.2], [36.3, 40.3, "z"], [36.4, 40.4]] }
    },
    {
      "type": "Feature",
      "properties": { "ad": "bozuk uyeler" },
      "geometry": {
        "type": "GeometryCollection",
        "geometries": [
          5,
          { "type": "Point" },
          { "type": "LineString", "coordinates": "x" },
          { "type": "Point", "coordinates": [37.0, 41.0] }
        ]
      }
    }
  ]
}
'''

# ── .prj texts (WKT 1) ──────────────────────────────────────────────────

TUREF_TM36_ESRI = (
    'PROJCS["TUREF_TM36",GEOGCS["GCS_TUREF",DATUM["D_Turkish_National_Reference_Frame",SPHEROID["GRS_1980",6378137.0,298.257222101]],'
    'PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Transverse_Mercator"],PARAMETER["False_Easting",500000.0],'
    'PARAMETER["False_Northing",0.0],PARAMETER["Central_Meridian",36.0],PARAMETER["Scale_Factor",1.0],PARAMETER["Latitude_Of_Origin",0.0],'
    'UNIT["Meter",1.0]]'
)
ED50_TM30_OGC = (
    'PROJCS["ED50 / TM30",GEOGCS["ED50",DATUM["European_Datum_1950",SPHEROID["International 1924",6378388,297,AUTHORITY["EPSG","7022"]],'
    'TOWGS84[-87,-98,-121,0,0,0,0],AUTHORITY["EPSG","6230"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],'
    'UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]],AUTHORITY["EPSG","4230"]],PROJECTION["Transverse_Mercator"],'
    'PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",30],PARAMETER["scale_factor",1],PARAMETER["false_easting",500000],'
    'PARAMETER["false_northing",0],UNIT["metre",1,AUTHORITY["EPSG","9001"]],AXIS["Easting",EAST],AXIS["Northing",NORTH],AUTHORITY["EPSG","2320"]]'
)
WGS84_ESRI = 'GEOGCS["GCS_WGS_1984",DATUM["D_WGS_1984",SPHEROID["WGS_1984",6378137.0,298.257223563]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]]'
WGS84_UTM36N_ESRI = (
    f'PROJCS["WGS_1984_UTM_Zone_36N",{WGS84_ESRI},PROJECTION["Transverse_Mercator"],PARAMETER["False_Easting",500000.0],'
    'PARAMETER["False_Northing",0.0],PARAMETER["Central_Meridian",33.0],PARAMETER["Scale_Factor",0.9996],PARAMETER["Latitude_Of_Origin",0.0],'
    'UNIT["Meter",1.0]]'
)
# Every parameter of WGS 84 / UTM zone 36N but the projection: only its name keeps the rules from recognising it.
LAMBERT_ESRI = (
    f'PROJCS["WGS_1984_Lambert_Conformal_Conic",{WGS84_ESRI},PROJECTION["Lambert_Conformal_Conic"],PARAMETER["False_Easting",500000.0],'
    'PARAMETER["False_Northing",0.0],PARAMETER["Central_Meridian",33.0],PARAMETER["Standard_Parallel_1",36.0],'
    'PARAMETER["Standard_Parallel_2",42.0],PARAMETER["Scale_Factor",0.9996],PARAMETER["Latitude_Of_Origin",0.0],UNIT["Meter",1.0]]'
)

# ── Shapefile sets ──────────────────────────────────────────────────────
# A point is (x, y) for plain types, (x, y, m) for M types, (x, y, z, m) for Z types (all Z sets here carry M).
# Polygon parts are written with their role, which the exact checks hold them to; the file only gets the ring.


def closed(points):
    return [*points, points[0]]


def outline(*points):
    """A clockwise ring (negative shoelace area): an outline."""
    return ("outline", None, closed(points))


def hole(owner, *points):
    """A counter-clockwise ring: a hole of the record's part number `owner`, or of none (it then stands alone)."""
    return ("hole", owner, closed(points))


def flat(*points):
    """A ring whose shoelace area is exactly zero: it gives nothing."""
    return ("zero", None, closed(points))


SHAPEFILES = {
    "noktalar": dict(
        type=11,  # PointZ
        shapes=[
            (512345.678, 4423456.789, 105.2, 0.0),
            (512400.125, 4423500.5, 107.85, 1.0),
            (512450.0, 4423400.25, 99.5, 2.0),
        ],
        codec="cp1254",
        ldid=0xCA,
        fields=[("AD", "C", 30, 0), ("KOT", "N", 9, 3), ("ALAN", "F", 19, 11), ("VAR", "L", 1, 0), ("TARIH", "D", 8, 0), ("ACIKLAMA", "C", 20, 0)],
        rows=[
            (False, ["ÇŞĞÜÖİ çşğüöı", "105.200", "1.23456780000e+003", "T", "20260926", ""]),
            (False, ["Işıklı Köprü", "107.850", "-4.50000000000e-001", "F", "", "Sınır taşı"]),
            (False, [" Kuzey Çeşme", "", "0.00000000000e+000", "?", "00000000", ""]),
        ],
        prj=TUREF_TM36_ESRI,
    ),
    "yollar": dict(
        type=3,  # PolyLine
        shapes=[
            [[(415000.5, 4540000.25), (415100.75, 4540050.5)], [(415200.0, 4540100.0), (415250.5, 4540180.25), (415330.125, 4540210.0), (415400.0, 4540300.5)]],
            [[(415500.0, 4540400.0)]],
            [[(415600.0, 4540500.0), (415650.25, 4540575.5), (415700.5, 4540600.0)]],
        ],
        codec="utf-8",
        ldid=0x57,  # the .cpg decides; this byte alone would say Windows-1254
        fields=[("AD", "C", 50, 0), ("ŞERİT", "N", 2, 0), ("KAPLAMA", "C", 20, 0)],
        rows=[
            (False, ["Atatürk Bulvarı", "4", "Asfalt"]),
            (False, ["Çıkmaz Sokak", "1", "Parke taşı"]),
            (False, ["Gökçe Ağaçlı Yolu", "2", "Stabilize"]),
        ],
        cpg="UTF-8",
        prj=ED50_TM30_OGC,
    ),
    "parseller": dict(
        type=5,  # Polygon, WGS 84 longitude, latitude
        shapes=[
            [
                outline((29.0600, 40.1900), (29.0600, 40.1910), (29.0612, 40.1910), (29.0612, 40.1900)),
                hole(0, (29.0603, 40.1903), (29.0609, 40.1903), (29.0609, 40.1907), (29.0603, 40.1907)),
            ],
            [
                outline((29.0620, 40.1900), (29.0620, 40.1908), (29.0628, 40.1908), (29.0628, 40.1900)),
                hole(2, (29.0633, 40.1903), (29.0639, 40.1903), (29.0639, 40.1909), (29.0633, 40.1909)),
                outline((29.0630, 40.1900), (29.0630, 40.1912), (29.0642, 40.1912), (29.0642, 40.1900)),
            ],
            [hole(None, (29.0650, 40.1900), (29.0660, 40.1900), (29.0660, 40.1910), (29.0650, 40.1910))],
            [outline((29.0670, 40.1900), (29.0670, 40.1910), (29.0680, 40.1910), (29.0680, 40.1900))],
            [
                outline((29.0690, 40.1900), (29.0695, 40.1910), (29.0700, 40.1900)),
                flat((29.0703125, 40.1875), (29.0703125, 40.1953125), (29.0703125, 40.203125)),
            ],
        ],
        codec="cp857",
        ldid=0x6B,
        fields=[("ADA", "N", 5, 0), ("PARSEL", "N", 5, 0), ("MALİK", "C", 30, 0), ("NİTELİK", "C", 25, 0)],
        rows=[
            (False, ["101", "5", "Gülşen Çağlar", "Bahçeli kargir ev"]),
            (False, ["101", "6", "Ömer Işık", "Tarla"]),
            (False, ["102", "1", "Şükrü Öğüt", "Bağ"]),
            (True, ["102", "2", "Silinmiş Kayıt", "Arsa"]),
            (False, ["103", "14", "İlkay Ünal", "Zeytinlik"]),
        ],
        prj=WGS84_ESRI,
    ),
    "kuyular": dict(
        type=18,  # MultiPointZ
        shapes=[
            [(452001.25, 4401002.5, 850.75, 10.0), (452010.5, 4401020.0, 851.0, 11.0), (452030.0, 4401005.75, 849.5, 12.0)],
            [(452100.0, 4401100.0, 860.25, 20.0), (452120.5, 4401130.5, 861.5, 21.0)],
        ],
        codec="iso8859_9",
        ldid=0x00,
        fields=[("AD", "C", 25, 0), ("DERINLIK", "N", 8, 2), ("SU", "L", 1, 0), ("OLCUM", "D", 8, 0)],
        rows=[
            (False, ["Şifalı Kuyu 1", "120.50", "Y", "20250314"]),
            (False, ["Işıklı Çeşme Kuyusu", "85.00", "n", " 1.3.25 "]),
        ],
        cpg="ISO-8859-9",
    ),
    "karisik": dict(
        type=21,  # PointM, a Null shape between; the table has one record fewer than the shapes
        shapes=[(500100.0, 4400100.0, 10.0), None, (500200.5, 4400200.5, 20.0), (500300.25, 4400300.75, 30.0)],
        codec="ascii",
        ldid=0x03,  # the unknown .cpg gives Windows-1254; this byte alone would say Windows-1252
        fields=[("AD", "C", 10, 0), ("NO", "N", 3, 0)],
        rows=[(False, ["Bir", "1"]), (False, ["Iki", "2"]), (False, ["Uc", "3"])],
        cpg="KOI8-R",
        prj=LAMBERT_ESRI,
    ),
    "alanlarz": dict(
        type=15,  # PolygonZ
        shapes=[
            [
                outline((487000.0, 4420000.0, 910.0, 0.0), (487000.0, 4420100.0, 912.5, 1.0), (487100.0, 4420100.0, 915.0, 2.0), (487100.0, 4420000.0, 911.25, 3.0)),
                hole(0, (487010.0, 4420010.0, 910.5, 4.0), (487030.0, 4420010.0, 910.75, 5.0), (487030.0, 4420030.0, 911.0, 6.0), (487010.0, 4420030.0, 911.5, 7.0)),
                hole(0, (487060.5, 4420060.5, 913.0, 8.0), (487080.25, 4420060.5, 913.25, 9.0), (487070.125, 4420080.75, 914.0, 10.0)),
            ],
        ],
        codec="cp1252",
        ldid=0x03,
        fields=[("AD", "C", 30, 0), ("KOD", "N", 4, 0)],
        rows=[(False, ["Café Rüzgâr", "17"])],
        prj=WGS84_UTM36N_ESRI,
    ),
}

# Bytes the table must not hold, because Python's codecs (this reference) and a WHATWG decoder (the Rust
# reader) read them differently: the undefined bytes of Windows-1252/1254 and CP857 (Python replaces them,
# WHATWG gives C1 controls), and the C1 range of ISO-8859-9 (WHATWG reads that label as Windows-1254).
UNPORTABLE = {
    "cp1254": frozenset(b"\x81\x8d\x8e\x8f\x90\x9d\x9e"),
    "cp1252": frozenset(b"\x81\x8d\x8e\x8f\x90\x9d\x9e"),
    "cp857": frozenset(b"\xd5\xe7\xf2"),
    "iso8859_9": frozenset(range(0x80, 0xA0)),
}

POINT_TYPES, MULTIPOINT_TYPES, POLYGON_TYPES = (1, 11, 21), (8, 18, 28), (5, 15, 25)
HAS_Z = (11, 13, 15, 18)
HAS_M = (11, 13, 15, 18, 21, 23, 25, 28)


def parts_of(shape_type, shape):
    return [r[2] for r in shape] if shape_type in POLYGON_TYPES else shape


def shape_points(shape_type, shape):
    if shape is None:
        return []
    if shape_type in POINT_TYPES:
        return [shape]
    if shape_type in MULTIPOINT_TYPES:
        return list(shape)
    return [p for part in parts_of(shape_type, shape) for p in part]


def extent(points):
    return (min(p[0] for p in points), min(p[1] for p in points), max(p[0] for p in points), max(p[1] for p in points))


def z_values(shape_type, points):
    return [p[2] for p in points]


def m_values(shape_type, points):
    return [p[3] if shape_type in HAS_Z else p[2] for p in points]


def measures(shape_type, points):
    """The z block (Z types) and the m block after the x, y values: a range, then one value per point."""
    out = b""
    for present, values in ((shape_type in HAS_Z, z_values), (shape_type in HAS_M, m_values)):
        if present:
            v = values(shape_type, points)
            out += struct.pack(f"<2d{len(v)}d", min(v), max(v), *v)
    return out


def xy(points):
    return b"".join(struct.pack("<2d", p[0], p[1]) for p in points)


def content(shape_type, shape):
    """One record's content: the shape type and its data, laid out as the Technical Description's byte tables give them."""
    if shape is None:
        return struct.pack("<i", 0)
    if shape_type in POINT_TYPES:
        return struct.pack(f"<i{len(shape)}d", shape_type, *shape)
    if shape_type in MULTIPOINT_TYPES:
        return struct.pack("<i4di", shape_type, *extent(shape), len(shape)) + xy(shape) + measures(shape_type, shape)
    parts = parts_of(shape_type, shape)
    points = [p for part in parts for p in part]
    starts = [sum(len(q) for q in parts[:i]) for i in range(len(parts))]
    head = struct.pack("<i4d2i", shape_type, *extent(points), len(parts), len(points)) + struct.pack(f"<{len(parts)}i", *starts)
    return head + xy(points) + measures(shape_type, points)


def shapefile(shape_type, shapes):
    """The .shp and .shx: a 100-byte header each, the records and their index (offsets in 16-bit words)."""
    body, index, at = b"", b"", 100
    for number, c in enumerate((content(shape_type, s) for s in shapes), 1):
        body += struct.pack(">2i", number, len(c) // 2) + c
        index += struct.pack(">2i", at // 2, len(c) // 2)
        at += 8 + len(c)
    points = [p for s in shapes for p in shape_points(shape_type, s)]
    zr = (min(z_values(shape_type, points)), max(z_values(shape_type, points))) if shape_type in HAS_Z else (0.0, 0.0)
    mr = (min(m_values(shape_type, points)), max(m_values(shape_type, points))) if shape_type in HAS_M else (0.0, 0.0)

    def header(length):
        return struct.pack(">7i", 9994, 0, 0, 0, 0, 0, length // 2) + struct.pack("<2i8d", 1000, shape_type, *extent(points), *zr, *mr)

    return header(100 + len(body)) + body, header(100 + len(index)) + index


def pad(kind, raw, width):
    """A field's bytes: N and F right-aligned, the rest left-aligned, spaces to the width."""
    assert len(raw) <= width, (raw, width)
    return raw.rjust(width, b" ") if kind in "NF" else raw.ljust(width, b" ")


def dbase(spec):
    """A dBASE III table (version 3, no memo), last updated 2026-09-26, ending with 0x1A."""
    fields, rows, codec = spec["fields"], spec["rows"], spec["codec"]
    header_length, record_length = 32 + 32 * len(fields) + 1, 1 + sum(f[2] for f in fields)
    out = struct.pack("<4BI2H", 0x03, 126, 9, 26, len(rows), header_length, record_length) + bytes(17) + bytes([spec["ldid"]]) + bytes(2)
    for name, kind, width, decimals in fields:
        raw = name.encode(codec)
        assert len(raw) <= 10, name  # the 11th byte stays a NUL
        out += raw.ljust(11, b"\0") + kind.encode("ascii") + bytes(4) + bytes([width, decimals]) + bytes(14)
    out += b"\r"
    for deleted, values in rows:
        out += b"*" if deleted else b" "
        for (_, kind, width, _), value in zip(fields, values, strict=True):
            out += pad(kind, value.encode(codec if kind == "C" else "ascii"), width)
    return out + b"\x1a"


def shapefile_set(spec):
    shp, shx = shapefile(spec["type"], spec["shapes"])
    files = {".shp": shp, ".shx": shx, ".dbf": dbase(spec)}
    for ext in (".prj", ".cpg"):
        if ext[1:] in spec:
            files[ext] = spec[ext[1:]].encode("ascii")
    return files


# ── What each fixture must give, by hand from the rules (not from the reader) ─
# Each object lists its kind and the keys checked; None means the key must be absent.

F1_ATTRS = {
    "ad": "Çankaya İlçesi Şehit Öğretmen Işık Üstün Sokağı",
    "no": "12",
    "oran": "1.50",
    "bin": "1e3",
    "sifir": "-0.0",
    "durum": "yeni",
    "aktif": "true",
    "kapali": "false",
    "detay": '{"kat":3,"cephe":"güney","alan":120.750,"isaret":"📍","ic":{"x":null,"y":[true,false]}}',
    "notlar": r'["ilk satır\nikinci \"söz\"","a/b\tc\u0001d\\e",2,-1.5E-3]',
}
F2_ATTRS = {
    "ad": "Kadıköy",
    "bos_metin": "",
    "bos_nesne": "{}",
    "bos_dizi": "[]",
    "eksi_sifir": "-0",
    "buyuk": "123456789012345678901234567890",
    "pi": "3.14159265358979323846264338327950288",
}
NOKTA = [
    {"AD": "ÇŞĞÜÖİ çşğüöı", "KOT": "105.200", "ALAN": "1.23456780000e+003", "VAR": "true", "TARIH": "2026-09-26"},
    {"AD": "Işıklı Köprü", "KOT": "107.850", "ALAN": "-4.50000000000e-001", "VAR": "false", "ACIKLAMA": "Sınır taşı"},
    {"AD": " Kuzey Çeşme", "ALAN": "0.00000000000e+000"},
]
YOL = [{"AD": "Atatürk Bulvarı", "ŞERİT": "4", "KAPLAMA": "Asfalt"}, {"AD": "Gökçe Ağaçlı Yolu", "ŞERİT": "2", "KAPLAMA": "Stabilize"}]
PARSEL = [
    {"ADA": "101", "PARSEL": "5", "MALİK": "Gülşen Çağlar", "NİTELİK": "Bahçeli kargir ev"},
    {"ADA": "101", "PARSEL": "6", "MALİK": "Ömer Işık", "NİTELİK": "Tarla"},
    {"ADA": "102", "PARSEL": "1", "MALİK": "Şükrü Öğüt", "NİTELİK": "Bağ"},
    {"ADA": "103", "PARSEL": "14", "MALİK": "İlkay Ünal", "NİTELİK": "Zeytinlik"},
]
KUYU = [
    {"AD": "Şifalı Kuyu 1", "DERINLIK": "120.50", "SU": "true", "OLCUM": "2025-03-14"},
    {"AD": "Işıklı Çeşme Kuyusu", "DERINLIK": "85.00", "SU": "false", "OLCUM": "1.3.25"},
]

SUMMARY = {
    "features": {
        "declaredSrid": 4326,
        "objects": [
            {"kind": "point", "layer": "ornek", "p": [32.8597, 39.9334], "z": 938.5, "label": None, "attrs": F1_ATTRS},
            {"kind": "point", "layer": "ornek", "p": [28.9784, 41.0082], "z": 39.25, "attrs": F2_ATTRS},
            {"kind": "point", "p": [29.0277, 41.0422], "z": None, "attrs": F2_ATTRS},
            {"kind": "line", "a": [27.1428, 38.4237], "b": [27.2, 38.46], "attrs": {"tur": "yol"}},
            {"kind": "polyline", "pts": [[30.7133, 36.8969], [30.72, 36.9], [30.7288, 36.8841], [30.74, 36.89]], "attrs": {}},
            {"kind": "line", "a": [35.4787, 38.7312], "b": [35.49, 38.74], "attrs": {"tur": "dere"}},
            {"kind": "polyline", "pts": [[35.5, 38.75], [35.51, 38.76], [35.52, 38.755]], "attrs": {"tur": "dere"}},
            {
                "kind": "polygon",
                "pts": [[32.4, 37.8], [32.6, 37.8], [32.6, 37.95], [32.4, 37.95]],
                "holes": [[[32.45, 37.85], [32.5, 37.9], [32.55, 37.85]]],
                "attrs": {"tur": "park"},
            },
            {"kind": "polygon", "pts": [[39.7, 40.98], [39.74, 40.98], [39.74, 41.02], [39.7, 41.02]], "holes": None, "attrs": {"tur": "ada"}},
            {"kind": "polygon", "pts": [[41.25, 39.88], [41.3, 39.88], [41.32, 39.9], [41.3, 39.92], [41.25, 39.92]], "holes": None},
            {"kind": "point", "p": [43.3833, 38.4942], "z": None, "attrs": {"tur": "koleksiyon"}},
            {"kind": "polyline", "pts": [[43.38, 38.49], [43.4, 38.5], [43.42, 38.505]], "attrs": {"tur": "koleksiyon"}},
            {"kind": "point", "layer": "ornek", "p": [34.5, 39.1], "z": 1100.5, "label": None, "attrs": {"tur": "dort sayi"}},
            {"kind": "polygon", "layer": "Sınır", "label": "P-12", "pts": [[33.0, 39.0], [33.1, 39.0], [33.1, 39.1], [33.0, 39.1]], "attrs": {"ada": "12", "parsel": "7"}},
            {"kind": "line", "layer": "ornek", "label": "K-1", "a": [36.0, 37.0], "b": [36.1, 37.05], "attrs": {"tur": "sadece etiket"}},
            {"kind": "point", "p": [38.0, 37.5], "z": -12.25, "attrs": {"tur": "ic ice"}},
            {"kind": "line", "layer": "ornek", "label": None, "a": [36.5, 37.5], "b": [36.6, 37.55], "attrs": {"tur": "bos kentos"}},
        ],
    },
    "tm": {
        "declaredSrid": 5256,
        "objects": [
            {
                "kind": "polygon",
                "layer": "Parsel",
                "label": "101/5",
                "pts": [[486512.3456789, 4420187.12345678], [486562.3456789, 4420187.12345678], [486562.34567891234, 4420237.123456789012], [486512.3456789, 4420237.12345678]],
                "holes": [[[486530.001, 4420200.002], [486530.007, 4420210.008], [486540.005, 4420210.006], [486540.003, 4420200.004]]],
                "attrs": {"ada": "101", "parsel": "5", "alan": "2345.678", "mahalle": "Çamlıbel"},
            },
            {"kind": "polygon", "layer": "Bina", "label": "B-1", "pts": [[486545.125, 4420215.25], [486555.875, 4420215.25], [486555.875, 4420228.5], [486545.125, 4420228.5]], "holes": None, "attrs": {"kat": "4", "yapi": "betonarme"}},
            {"kind": "point", "layer": "tm", "label": None, "p": [486512.3456789, 4420187.12345678], "z": 1021.345, "attrs": {"tur": "poligon noktası"}},
            {"kind": "point", "layer": "tm", "p": [486562.3456789, 4420187.12345678], "z": 1019.8765},
            {"kind": "point", "layer": "Bina", "label": None, "p": [486550.0, 4420215.5], "z": None, "attrs": {"ad": "kapı"}},
        ],
    },
    "bare-polygon": {
        "declaredSrid": 4326,
        "objects": [
            {
                "kind": "polygon",
                "layer": "bare-polygon",
                "label": None,
                "pts": [[29.1, 40.2], [29.2, 40.2], [29.2, 40.3], [29.1, 40.3]],
                "holes": [[[29.15, 40.25], [29.15, 40.28], [29.18, 40.28], [29.18, 40.25]]],
                "attrs": {},
            }
        ],
    },
    "feature-epsg": {
        "declaredSrid": 2320,
        "objects": [{"kind": "polyline", "layer": "feature-epsg", "pts": [[415678.901, 4541234.567], [415800.25, 4541300.125], [416012.5, 4541288.75]], "attrs": {"yol": "D-100", "serit": "3"}}],
    },
    "crs84": {"declaredSrid": 4326, "objects": [{"kind": "point", "layer": "crs84", "p": [32.8369, 39.9253], "z": None, "attrs": {"ad": "Anıtkabir"}}]},
    "nonfinite": {
        "declaredSrid": 4326,
        "objects": [
            {"kind": "point", "p": [30.0, 40.0], "z": None, "attrs": {"ad": "coklu nokta", "buyuk": "1e999"}},
            {"kind": "point", "p": [30.2, 40.2], "z": 0.0, "attrs": {"ad": "coklu nokta", "buyuk": "1e999"}},
            {"kind": "line", "a": [31.0, 40.5], "b": [31.1, 40.6], "attrs": {"ad": "koleksiyon"}},
            {"kind": "polyline", "pts": [[32.2, 41.2], [32.3, 41.3], [32.4, 41.2]], "attrs": {"ad": "coklu cizgi"}},
            {"kind": "polygon", "pts": [[34.5, 40.5], [34.7, 40.5], [34.7, 40.7]], "holes": None, "attrs": {"ad": "coklu cokgen"}},
            {"kind": "point", "p": [35.0, 39.5], "attrs": {"ad": "nokta"}},
            {"kind": "point", "p": [35.5, 39.6], "z": 12.5, "attrs": {"ad": "dorduncu sayi"}},
            {"kind": "point", "p": [36.0, 40.0], "z": None, "attrs": {"ad": "bozuk konumlar"}},
            {"kind": "point", "p": [36.4, 40.4], "z": None, "attrs": {"ad": "bozuk konumlar"}},
            {"kind": "point", "p": [37.0, 41.0], "z": None, "attrs": {"ad": "bozuk uyeler"}},
        ],
    },
    "noktalar": {
        "declaredSrid": 5256,
        "encoding": "Windows-1254",
        "objects": [
            {"kind": "point", "layer": "noktalar", "label": None, "p": [512345.678, 4423456.789], "z": 105.2, "attrs": NOKTA[0]},
            {"kind": "point", "p": [512400.125, 4423500.5], "z": 107.85, "attrs": NOKTA[1]},
            {"kind": "point", "p": [512450.0, 4423400.25], "z": 99.5, "attrs": NOKTA[2]},
        ],
    },
    "yollar": {
        "declaredSrid": 2320,
        "encoding": "UTF-8",
        "objects": [
            {"kind": "line", "layer": "yollar", "a": [415000.5, 4540000.25], "b": [415100.75, 4540050.5], "attrs": YOL[0]},
            {"kind": "polyline", "pts": [[415200.0, 4540100.0], [415250.5, 4540180.25], [415330.125, 4540210.0], [415400.0, 4540300.5]], "attrs": YOL[0]},
            {"kind": "polyline", "pts": [[415600.0, 4540500.0], [415650.25, 4540575.5], [415700.5, 4540600.0]], "attrs": YOL[1]},
        ],
    },
    "parseller": {
        "declaredSrid": 4326,
        "encoding": "CP857",
        "objects": [
            {
                "kind": "polygon",
                "layer": "parseller",
                "pts": [[29.06, 40.19], [29.06, 40.191], [29.0612, 40.191], [29.0612, 40.19]],
                "holes": [[[29.0603, 40.1903], [29.0609, 40.1903], [29.0609, 40.1907], [29.0603, 40.1907]]],
                "attrs": PARSEL[0],
            },
            {"kind": "polygon", "pts": [[29.062, 40.19], [29.062, 40.1908], [29.0628, 40.1908], [29.0628, 40.19]], "holes": None, "attrs": PARSEL[1]},
            {
                "kind": "polygon",
                "pts": [[29.063, 40.19], [29.063, 40.1912], [29.0642, 40.1912], [29.0642, 40.19]],
                "holes": [[[29.0633, 40.1903], [29.0639, 40.1903], [29.0639, 40.1909], [29.0633, 40.1909]]],
                "attrs": PARSEL[1],
            },
            {"kind": "polygon", "pts": [[29.065, 40.19], [29.066, 40.19], [29.066, 40.191], [29.065, 40.191]], "holes": None, "attrs": PARSEL[2]},
            {"kind": "polygon", "pts": [[29.069, 40.19], [29.0695, 40.191], [29.07, 40.19]], "holes": None, "attrs": PARSEL[3]},
        ],
    },
    "kuyular": {
        "declaredSrid": None,
        "encoding": "ISO-8859-9",
        "objects": [
            {"kind": "point", "layer": "kuyular", "p": [452001.25, 4401002.5], "z": 850.75, "attrs": KUYU[0]},
            {"kind": "point", "p": [452010.5, 4401020.0], "z": 851.0, "attrs": KUYU[0]},
            {"kind": "point", "p": [452030.0, 4401005.75], "z": 849.5, "attrs": KUYU[0]},
            {"kind": "point", "p": [452100.0, 4401100.0], "z": 860.25, "attrs": KUYU[1]},
            {"kind": "point", "p": [452120.5, 4401130.5], "z": 861.5, "attrs": KUYU[1]},
        ],
    },
    "karisik": {
        "declaredSrid": None,
        "encoding": "Windows-1254",
        "objects": [
            {"kind": "point", "layer": "karisik", "p": [500100.0, 4400100.0], "z": None, "attrs": {"AD": "Bir", "NO": "1"}},
            {"kind": "point", "p": [500200.5, 4400200.5], "z": None, "attrs": {"AD": "Uc", "NO": "3"}},
            {"kind": "point", "p": [500300.25, 4400300.75], "z": None, "attrs": {}},
        ],
    },
    "alanlarz": {
        "declaredSrid": 32636,
        "encoding": "Windows-1252",
        "objects": [
            {
                "kind": "polygon",
                "layer": "alanlarz",
                "label": None,
                "pts": [[487000.0, 4420000.0], [487000.0, 4420100.0], [487100.0, 4420100.0], [487100.0, 4420000.0]],
                "holes": [
                    [[487010.0, 4420010.0], [487030.0, 4420010.0], [487030.0, 4420030.0], [487010.0, 4420030.0]],
                    [[487060.5, 4420060.5], [487080.25, 4420060.5], [487070.125, 4420080.75]],
                ],
                "attrs": {"AD": "Café Rüzgâr", "KOD": "17"},
            }
        ],
    },
}

KEYS = {
    "point": ("kind", "layer", "p", "z", "label", "attrs"),
    "line": ("kind", "layer", "a", "b", "label", "attrs"),
    "polyline": ("kind", "layer", "pts", "label", "attrs"),
    "polygon": ("kind", "layer", "pts", "holes", "label", "attrs"),
}


def bits(v):
    """A value compared bit for bit: floats by their binary64 bytes (so -0.0 ≠ 0.0, 1 ≠ 1.0)."""
    if type(v) is float:
        return ("f", struct.pack(">d", v))
    if type(v) in (list, tuple):
        return [bits(x) for x in v]
    if type(v) is dict:
        return {k: bits(x) for k, x in v.items()}
    return (type(v).__name__, v)


def against_summary(name, result):
    want, problems = SUMMARY[name], []
    top = ("declaredSrid", "encoding", "objects") if "encoding" in want else ("declaredSrid", "objects")
    if tuple(result) != top:
        problems.append(f"{name}: kök anahtarları {list(result)}, beklenen {list(top)}")
    for key in top[:-1]:
        if bits(result.get(key)) != bits(want[key]):
            problems.append(f"{name}: {key} {result.get(key)!r}, beklenen {want[key]!r}")
    got = result.get("objects", [])
    if [o.get("kind") for o in got] != [o["kind"] for o in want["objects"]]:
        return problems + [f"{name}: nesne türleri {[o.get('kind') for o in got]}, beklenen {[o['kind'] for o in want['objects']]}"]
    for i, (obj, expected) in enumerate(zip(got, want["objects"])):
        keys = KEYS[obj["kind"]]
        if list(obj) != [k for k in keys if k in obj] or not {"kind", "layer", "attrs"} <= set(obj):
            problems.append(f"{name}: {i}. nesnenin anahtarları {list(obj)} kurallardaki sırada ya da tam değil")
        if not all(type(v) is str for v in obj.get("attrs", {}).values()):
            problems.append(f"{name}: {i}. nesnenin öznitelik değerleri metin değil")
        for key, value in expected.items():
            if value is None:
                if key in obj:
                    problems.append(f"{name}: {i}. nesnede {key} olmamalı, var: {obj[key]!r}")
            elif bits(obj.get(key)) != bits(value):
                problems.append(f"{name}: {i}. nesnenin {key} değeri {obj.get(key)!r}, beklenen {value!r}")
    return problems


# ── Checks written apart from the reader ───────────────────────────────


def side(ring, point):
    """+1 strictly inside the ring, -1 strictly outside, 0 on its boundary: exact rational arithmetic."""
    px, py = Fraction(point[0]), Fraction(point[1])
    pts = [(Fraction(p[0]), Fraction(p[1])) for p in ring]
    inside = False
    for i in range(len(pts)):
        (xi, yi), (xj, yj) = pts[i], pts[i - 1]
        if (xj - xi) * (py - yi) == (yj - yi) * (px - xi) and min(xi, xj) <= px <= max(xi, xj) and min(yi, yj) <= py <= max(yi, yj):
            return 0
        if (yi > py) != (yj > py) and px < (xj - xi) * (py - yi) / (yj - yi) + xi:
            inside = not inside
    return 1 if inside else -1


def orientation(a, b, c):
    v = (Fraction(b[0]) - Fraction(a[0])) * (Fraction(c[1]) - Fraction(a[1])) - (Fraction(b[1]) - Fraction(a[1])) * (Fraction(c[0]) - Fraction(a[0]))
    return (v > 0) - (v < 0)


def edges_meet(ring, other):
    """True when an edge of `ring` touches or crosses an edge of `other` (exact)."""
    for i in range(len(ring)):
        a, b = ring[i - 1], ring[i]
        for j in range(len(other)):
            c, d = other[j - 1], other[j]
            o1, o2, o3, o4 = orientation(a, b, c), orientation(a, b, d), orientation(c, d, a), orientation(c, d, b)
            if o1 * o2 <= 0 and o3 * o4 <= 0 and not (o1 == o2 == o3 == o4 == 0 and (max(a[0], b[0]) < min(c[0], d[0]) or max(c[0], d[0]) < min(a[0], b[0]) or max(a[1], b[1]) < min(c[1], d[1]) or max(c[1], d[1]) < min(a[1], b[1]))):
                return True
    return False


def strictly_inside(inner, outer):
    """Every vertex of `inner` strictly inside `outer`, every vertex of `outer` strictly outside `inner`, no edges meeting."""
    return all(side(outer, p) == 1 for p in inner) and all(side(inner, p) == -1 for p in outer) and not edges_meet(inner, outer)


def without_closing(ring):
    return ring[:-1] if ring[-1][:2] == ring[0][:2] else ring


def float_area2(ring):
    """Twice the shoelace area in float64, summed in the rules' order (i to i+1, cyclic)."""
    total = 0.0
    for i in range(len(ring)):
        total += ring[i][0] * ring[(i + 1) % len(ring)][1] - ring[(i + 1) % len(ring)][0] * ring[i][1]
    return total


def polygon_roles(name, spec):
    """Each ring's orientation, exact and in float64, and where each hole must go."""
    problems = []
    for r, shape in enumerate(spec["shapes"] if spec["type"] in POLYGON_TYPES else []):
        rings = [[p[:2] for p in without_closing(part)] for _, _, part in shape]
        outlines = [k for k, (role, _, _) in enumerate(shape) if role == "outline"]
        for k, ((role, owner, _), ring) in enumerate(zip(shape, rings)):
            exact = sum(Fraction(ring[i][0]) * Fraction(ring[(i + 1) % len(ring)][1]) - Fraction(ring[(i + 1) % len(ring)][0]) * Fraction(ring[i][1]) for i in range(len(ring)))
            want = {"outline": -1, "hole": 1, "zero": 0}[role]
            fl = float_area2(ring)
            if (exact > 0) - (exact < 0) != want or (fl > 0) - (fl < 0) != want:
                problems.append(f"{name}: {r + 1}. kaydın {k}. parçası {role} değil (tam alan×2 {float(exact)}, float {fl})")
            if role == "hole":
                if owner is not None and not strictly_inside(ring, rings[owner]):
                    problems.append(f"{name}: {r + 1}. kaydın {k}. parçası {owner}. parçanın tam içinde değil")
                for other in outlines:
                    if other != owner and side(rings[other], ring[0]) != -1:
                        problems.append(f"{name}: {r + 1}. kaydın {k}. parçasının ilk köşesi {other}. parçanın dışında değil")
    return problems


def geojson_holes(name, text):
    """Holes of the GeoJSON polygons (the finite ones) strictly inside their outline."""
    problems = []

    def polygon(rings, where):
        if any(abs(v) == float("inf") for ring in rings for p in ring for v in p):
            return
        outer = without_closing([p[:2] for p in rings[0]])
        for h, ring in enumerate(rings[1:], 1):
            if not strictly_inside(without_closing([p[:2] for p in ring]), outer):
                problems.append(f"{name}: {where} {h}. deliği dış halkanın tam içinde değil")

    def walk(g, where):
        if not isinstance(g, dict):
            return
        if g.get("type") == "Polygon":
            polygon(g["coordinates"], where)
        elif g.get("type") == "MultiPolygon":
            for i, member in enumerate(g["coordinates"]):
                polygon(member, f"{where}[{i}]")
        elif g.get("type") == "GeometryCollection":
            for i, member in enumerate(g["geometries"]):
                walk(member, f"{where}.geometries[{i}]")
        elif g.get("type") == "Feature":
            walk(g.get("geometry"), f"{where}.geometry")
        elif g.get("type") == "FeatureCollection":
            for i, f in enumerate(g["features"]):
                walk(f, f"features[{i}]")

    walk(json.loads(text), "kök")
    return problems


def verify_shapefile(name, spec, files):
    """The set parsed again on its own: headers, extents and ranges, record numbers, index, table."""
    problems = []

    def fail(message):
        problems.append(f"{name}: {message}")

    shape_type, shp, shx, dbf = spec["type"], files[".shp"], files[".shx"], files[".dbf"]
    if struct.unpack_from(">6i", shp, 0) != (9994, 0, 0, 0, 0, 0) or struct.unpack_from(">i", shp, 24)[0] * 2 != len(shp):
        fail(".shp başlığının dosya kodu, boş alanları ya da uzunluğu yanlış")
    if struct.unpack_from("<2i", shp, 28) != (1000, shape_type):
        fail(".shp sürümü ya da şekil türü yanlış")
    decoded, offsets, at = [], [], 100
    while at < len(shp):
        number, words = struct.unpack_from(">2i", shp, at)
        if number != len(decoded) + 1:
            fail(f"{len(decoded) + 1}. kaydın numarası {number}")
        offsets.append((at // 2, words))
        decoded.append(reparse(shape_type, shp[at + 8 : at + 8 + 2 * words], fail))
        at += 8 + 2 * words
    if at != len(shp):
        fail(".shp kayıtları dosyanın sonunda bitmiyor")
    if decoded != [normal(shape_type, s) for s in spec["shapes"]]:
        fail(".shp'den geri okunan şekiller yazılanlar değil")
    points = [p for s in spec["shapes"] for p in shape_points(shape_type, s)]
    zr = (min(z_values(shape_type, points)), max(z_values(shape_type, points))) if shape_type in HAS_Z else (0.0, 0.0)
    mr = (min(m_values(shape_type, points)), max(m_values(shape_type, points))) if shape_type in HAS_M else (0.0, 0.0)
    if struct.unpack_from("<8d", shp, 36) != (*extent(points), *zr, *mr):
        fail(".shp başlığının kapsamı ya da z/m aralıkları noktalarınki değil")
    if len(shx) != 100 + 8 * len(decoded) or shx[:24] != shp[:24] or shx[28:100] != shp[28:100] or struct.unpack_from(">i", shx, 24)[0] * 2 != len(shx):
        fail(".shx başlığı .shp'ninkiyle (uzunluk dışında) aynı değil")
    if [struct.unpack_from(">2i", shx, 100 + 8 * i) for i in range(len(decoded))] != offsets:
        fail(".shx kayıtları .shp'deki yerleri göstermiyor")
    fields, rows, codec = spec["fields"], spec["rows"], spec["codec"]
    count = struct.unpack_from("<I", dbf, 4)[0]
    header_length, record_length = struct.unpack_from("<2H", dbf, 8)
    if dbf[:4] != bytes([3, 126, 9, 26]) or dbf[29] != spec["ldid"] or count != len(rows):
        fail(".dbf sürümü, tarihi, dil sürücüsü ya da kayıt sayısı yanlış")
    if header_length != 32 + 32 * len(fields) + 1 or dbf[header_length - 1] != 0x0D or record_length != 1 + sum(f[2] for f in fields):
        fail(".dbf başlık ya da kayıt uzunluğu yanlış")
    if len(dbf) != header_length + count * record_length + 1 or dbf[-1] != 0x1A:
        fail(".dbf dosya sonu (0x1A) yerinde değil")
    unportable = UNPORTABLE.get(codec, frozenset())
    for i, (fname, kind, width, decimals) in enumerate(fields):
        d = dbf[32 + 32 * i : 64 + 32 * i]
        raw = d[:11].split(b"\0", 1)[0]
        if raw.decode(codec) != fname or d[len(raw) : 11].strip(b"\0") or chr(d[11]) != kind or (d[16], d[17]) != (width, decimals) or d[12:16] + d[18:] != bytes(18):
            fail(f".dbf {i + 1}. alan tanımı yanlış")
        if unportable & set(raw):
            fail(f".dbf {i + 1}. alanın adında okuyuculara göre farklı çözülen bayt var")
    for n, (deleted, values) in enumerate(rows):
        record = dbf[header_length + n * record_length : header_length + (n + 1) * record_length]
        if record[:1] != (b"*" if deleted else b" "):
            fail(f".dbf {n + 1}. kaydın silinme baytı yanlış")
        pos = 1
        for (_, kind, width, _), value in zip(fields, values):
            raw = record[pos : pos + width]
            text = raw.decode(codec if kind == "C" else "ascii")  # strict: an undefined byte stops the check
            left = kind in "CLD"  # left-aligned kinds pad on the right, N and F on the left
            if (text.rstrip(" ") if left else text.lstrip(" ")) != (value.rstrip(" ") if left else value.lstrip(" ")):
                fail(f".dbf {n + 1}. kaydın alanı {text!r}, yazılan {value!r}")
            if unportable & set(raw):
                fail(f".dbf {n + 1}. kaydın {text!r} alanında okuyuculara göre farklı çözülen bayt var")
            pos += width
    if (".prj" in files) != ("prj" in spec) or (".cpg" in files) != ("cpg" in spec):
        fail(".prj ya da .cpg varlığı tanımla uyuşmuyor")
    return problems


def normal(shape_type, shape):
    """A written shape in the form reparse() gives: points as tuples, parts as lists."""
    if shape is None:
        return None
    if shape_type in POINT_TYPES:
        return tuple(shape)
    if shape_type in MULTIPOINT_TYPES:
        return [tuple(p) for p in shape]
    return [[tuple(p) for p in part] for part in parts_of(shape_type, shape)]


def reparse(shape_type, c, fail):
    """One record's content read back by the Technical Description's byte tables, its extent and ranges checked."""
    record_type = struct.unpack_from("<i", c, 0)[0]
    if record_type == 0:
        if len(c) != 4:
            fail("boş şekil 4 bayt değil")
        return None
    if record_type != shape_type:
        fail(f"kaydın türü {record_type}, dosyanınki {shape_type}")
    per_point = 2 + (shape_type in HAS_Z) + (shape_type in HAS_M)
    if shape_type in POINT_TYPES:
        if len(c) != 4 + 8 * per_point:
            fail(f"nokta kaydı {len(c)} bayt")
        return struct.unpack_from(f"<{per_point}d", c, 4)
    box = struct.unpack_from("<4d", c, 4)
    if shape_type in MULTIPOINT_TYPES:
        (n,) = struct.unpack_from("<i", c, 36)
        nparts, starts, at = 0, None, 40
    else:
        nparts, n = struct.unpack_from("<2i", c, 36)
        starts, at = struct.unpack_from(f"<{nparts}i", c, 44), 44 + 4 * nparts
    flat_xy = struct.unpack_from(f"<{2 * n}d", c, at)
    columns, at = [flat_xy[0::2], flat_xy[1::2]], at + 16 * n
    for present in (shape_type in HAS_Z, shape_type in HAS_M):
        if present:
            low, high, *values = struct.unpack_from(f"<{2 + n}d", c, at)
            if (low, high) != (min(values), max(values)):
                fail("kaydın z ya da m aralığı değerlerininki değil")
            columns.append(values)
            at += 16 + 8 * n
    if at != len(c):
        fail(f"kayıt {len(c)} bayt, alanları {at} bayt")
    points = list(zip(*columns))
    if box != extent(points):
        fail("kaydın kapsamı noktalarınki değil")
    if starts is None:
        return points
    if starts[0] != 0 or list(starts) != sorted(starts):
        fail("parça başlangıçları sıralı değil")
    bounds = [*starts, n]
    parts = [points[bounds[i] : bounds[i + 1]] for i in range(nparts)]
    if shape_type in POLYGON_TYPES and any(part[0] != part[-1] or len(part) < 4 for part in parts):
        fail("halka kapalı değil ya da dört noktadan az")
    return parts


# ── The Rust GeoJSON writer's files, read back (export/) ────────────────
# export/<name>.input.json is what the writer was given: {"entities": [KentOS objects in the contract's
# JSON form], "layers": [{"id", "name"}], "srid": N, "name": S}; export/<name>.geojson is what it wrote.
# The GeoJSON is read with the independent reader and held to the writer's rules.

EXPORT = DIR / "export"
UNWRITTEN = ("text", "dimension", "xline", "ray")  # kinds the writer leaves out
ARC = 1e-12  # a bulge above this in size is an arc


def xy_of(p):
    return [float(p["x"]), float(p["y"])]


def has_arcs(bulges):
    return bulges is not None and any(abs(b) > ARC for b in bulges)


def polygon_arcs(entity):
    return has_arcs(entity.get("bulges")) or any(has_arcs(h.get("bulges")) for h in entity.get("holes", []))


def right_hand(ring, sign):
    """The ring the right-hand rule's way (sign +1: a counter-clockwise outline, -1: a clockwise hole): a ring whose
    shoelace area has the other sign is reversed keeping its first vertex; a zero-area ring stays as is."""
    return [ring[0], *ring[:0:-1]] if float_area2(ring) * sign < 0 else ring


def path_of(obj):
    """A read object's points when it is a polyline, or a line (two points); else None."""
    if obj["kind"] == "polyline":
        return obj["pts"]
    if obj["kind"] == "line":
        return [obj["a"], obj["b"]]
    return None


def near(p, q, tol):
    return math.hypot(p[0] - q[0], p[1] - q[1]) <= tol


def on_circle(path, c, r):
    tol = 1e-9 * max(1.0, r)
    return all(abs(math.hypot(x - c[0], y - c[1]) - r) <= tol for x, y in path)


def full_turn(ellipse):
    return abs(float(ellipse["t1"]) - float(ellipse["t0"])) >= 2 * math.pi - 1e-9


def exported(entity, obj):
    """What is wrong with the object read back from one written entity (nothing: an empty list)."""
    kind, path = entity["kind"], path_of(obj)
    if kind == "point":
        wrong = obj["kind"] != "point" or bits(obj["p"]) != bits(xy_of(entity["p"]))
        return ["konum ya da z girdidekiyle aynı değil"] if wrong or bits(obj.get("z")) != bits(float(entity["z"]) if "z" in entity else None) else []
    if kind == "line":
        return [] if obj["kind"] == "line" and bits([obj["a"], obj["b"]]) == bits([xy_of(entity["a"]), xy_of(entity["b"])]) else ["uçlar girdidekiler değil"]
    if kind == "polyline" and not has_arcs(entity.get("bulges")):
        pts = [xy_of(p) for p in entity["pts"]]
        good = path is not None and (obj["kind"] == "line") == (len(pts) == 2) and bits(path) == bits(pts)
        return [] if good else ["noktalar girdidekiler değil (iki noktalı yol çizgi olarak okunur)"]
    if kind == "hatch" or (kind == "polygon" and not polygon_arcs(entity)):
        if kind == "polygon":
            outline, holes = [xy_of(p) for p in entity["pts"]], [[xy_of(p) for p in h["pts"]] for h in entity.get("holes", [])]
        else:
            outline, holes = [xy_of(p) for p in entity["ring"]], [[xy_of(p) for p in h] for h in entity.get("holes", [])]
        want = [right_hand(h, -1) for h in holes if len(h) >= 3]
        good = obj["kind"] == "polygon" and bits(obj["pts"]) == bits(right_hand(outline, 1)) and bits(obj.get("holes", [])) == bits(want)
        return [] if good else ["halkalar girdidekiler değil ya da sağ el kuralına göre (ilk köşe yerinde) döndürülmemiş"]
    if kind == "polygon":  # with arcs
        holes = obj.get("holes", [])
        good = obj["kind"] == "polygon" and len(obj["pts"]) >= 3 and float_area2(obj["pts"]) >= 0 and len(holes) == len(entity.get("holes", []))
        return [] if good and all(len(h) >= 3 and float_area2(h) <= 0 for h in holes) else ["yaylı alan en az üç noktalı, sağ el kuralında ve girdideki kadar delikli değil"]
    if kind == "circle":
        c, r = xy_of(entity["c"]), float(entity["r"])
        good = obj["kind"] == "polyline" and len(path) == 73 and bits(path[0]) == bits(path[-1]) and on_circle(path, c, r)
        return [] if good else ["daire 73 noktalı, kapalı ve çemberin üstünde bir yol değil"]
    if kind == "arc":
        c, r, a0, a1 = xy_of(entity["c"]), float(entity["r"]), float(entity["a0"]), float(entity["a1"])
        tol = 1e-9 * max(1.0, r)
        good = path is not None and len(path) >= 2 and on_circle(path, c, r)
        good = good and near(path[0], (c[0] + r * math.cos(a0), c[1] + r * math.sin(a0)), tol) and near(path[-1], (c[0] + r * math.cos(a1), c[1] + r * math.sin(a1)), tol)
        return [] if good else ["yay çemberin üstünde değil ya da uçları a0 ile a1'de değil"]
    if kind in ("ellipse", "spline", "polyline"):  # the polyline has arcs here
        if path is None or len(path) < 2:
            return ["en az iki noktalı bir yol değil"]
        closed = (kind == "spline" and entity.get("closed")) or (kind == "ellipse" and full_turn(entity))
        if closed and bits(path[0]) != bits(path[-1]):
            return ["kapalı eğrinin ilk ve son noktası aynı değil"]
        if kind == "spline" and not entity.get("closed") and bits([path[0], path[-1]]) != bits([xy_of(entity["pts"][0]), xy_of(entity["pts"][-1])]):
            return ["açık eğri ilk kontrol noktasından başlayıp sonuncusunda bitmiyor"]
        return []
    return [f"bilinmeyen tür {kind!r}"]


def check_export(name, spec, data):
    """One written pair: the document's shape and crs, then every object read back against its entity."""
    try:
        document = json.loads(data.decode("utf-8"))
        result = gis.read_geojson(data, name)
    except (ValueError, gis.GisError) as e:
        return [f"export/{name}.geojson okunamadı: {e}"]
    problems = []
    if not isinstance(document, dict) or document.get("type") != "FeatureCollection" or document.get("name") != spec["name"]:
        problems.append(f"export/{name}: kök, adı girdinin adı ({spec['name']!r}) olan bir FeatureCollection değil")
    srid = spec["srid"]
    if (srid == 4326 and "crs" in document) or (srid != 4326 and document.get("crs") != {"type": "name", "properties": {"name": f"urn:ogc:def:crs:EPSG::{srid}"}}):
        problems.append(f"export/{name}: crs üyesi SRID {srid} için beklenen değil (4326: yok; öbürleri: urn:ogc:def:crs:EPSG::{srid})")
    if result["declaredSrid"] != srid:
        problems.append(f"export/{name}: okunan SRID {result['declaredSrid']}, girdininki {srid}")
    layers = {layer["id"]: layer["name"] for layer in spec["layers"]}
    entities = [e for e in spec["entities"] if e["kind"] not in UNWRITTEN]
    objects = result["objects"]
    if len(objects) != len(entities):
        return problems + [f"export/{name}: {len(objects)} nesne okundu, yazılabilen {len(entities)} nesne var"]
    for i, (entity, obj) in enumerate(zip(entities, objects)):
        where = f"export/{name}: {i}. nesne (kimlik {entity.get('id')}, {entity['kind']})"
        label = entity.get("label") if isinstance(entity.get("label"), str) and entity.get("label") else None
        # A layer missing from the list is not named: the object reads back on the default layer (the document's name).
        layer = layers.get(entity.get("layerId"), spec["name"] or name)
        if obj["layer"] != layer:
            problems.append(f"{where}: katman {obj['layer']!r}, beklenen {layer!r}")
        if obj.get("label") != label:
            problems.append(f"{where}: etiket {obj.get('label')!r}, beklenen {label!r} (None: etiket yok)")
        if obj["attrs"] != entity.get("attrs", {}):
            problems.append(f"{where}: öznitelikler girdidekiler değil")
        problems += [f"{where}: {p}" for p in exported(entity, obj)]
    return problems


def check_exports():
    """(problems, the names of the pairs checked) for export/; nothing to check without the directory."""
    if not EXPORT.is_dir():
        return [], []
    inputs = {p.name[: -len(".input.json")] for p in EXPORT.glob("*.input.json")}
    outputs = {p.name[: -len(".geojson")] for p in EXPORT.glob("*.geojson")}
    problems = [f"export/{name}: .input.json ile .geojson'dan biri eksik" for name in sorted(inputs ^ outputs)]
    names = []
    for name in sorted(inputs & outputs):
        try:
            spec = json.loads((EXPORT / f"{name}.input.json").read_text("utf-8"))
            problems += check_export(name, spec, (EXPORT / f"{name}.geojson").read_bytes())
        except (ValueError, KeyError, TypeError, IndexError) as e:
            problems.append(f"export/{name}: denetlenemedi ({type(e).__name__}: {e})")
        names.append(name)
    return problems, names


# ── Writing and checking ────────────────────────────────────────────────


def json_text(text):
    """A hand-written GeoJSON text with its `@u` marks made JSON \\u escapes."""
    out = text.replace("@u", "\\" + "u")
    assert "@" not in out, "GeoJSON metninde @u dışında @ olmamalı"
    return out


def fixture_sets():
    """{fixture name: {file name: bytes}}, without the expected files."""
    sets = {name: {f"{name}.geojson": json_text(text).encode("utf-8")} for name, text in GEOJSON.items()}
    for name, spec in SHAPEFILES.items():
        sets[name] = {f"{name}{ext}": data for ext, data in shapefile_set(spec).items()}
    return sets


def read_set(name, files):
    """The reader's canonical dict of one fixture, from the bytes given."""
    if name in GEOJSON:
        return gis.read_geojson(files[f"{name}.geojson"], name)
    return gis.read_shapefile_files({ext: files[f"{name}{ext}"] for ext in (".shp", ".shx", ".dbf", ".prj", ".cpg") if f"{name}{ext}" in files}, name)


# ── Zipped Shapefiles (docs/adr/0053) ───────────────────────────────────
# archive → (method, folder inside it, the sets it holds). Each set's files go in
# the order .shp, .shx, .dbf, .prj, .cpg (those it has), at a fixed time, as Unix
# files (0644). The Rust reader must read each set inside exactly as its files
# beside it (crates/shared/formats/tests/gis.rs).
ARCHIVES = {
    "parseller.zip": (zipfile.ZIP_DEFLATED, "", ["parseller"]),
    "katmanlar.zip": (zipfile.ZIP_DEFLATED, "katmanlar/", ["parseller", "yollar"]),
    "kuyular-stored.zip": (zipfile.ZIP_STORED, "", ["kuyular"]),
}
PARTS = (".shp", ".shx", ".dbf", ".prj", ".cpg")


def archive_members(name, files):
    """[(name in the archive, file it holds)] of one archive."""
    _, folder, sets = ARCHIVES[name]
    return [(f"{folder}{s}{ext}", f"{s}{ext}") for s in sets for ext in PARTS if f"{s}{ext}" in files]


def archive(name, files):
    """One archive's bytes, from the files built here."""
    method = ARCHIVES[name][0]
    out = io.BytesIO()
    with zipfile.ZipFile(out, "w") as z:
        for member, source in archive_members(name, files):
            info = zipfile.ZipInfo(member, date_time=(2026, 9, 26, 12, 0, 0))
            info.compress_type = method
            info.create_system = 3
            info.external_attr = 0o644 << 16
            z.writestr(info, files[source])
    return out.getvalue()


def check_archives(files):
    """Each archive on disk opens, its CRCs hold, and its members are the files built here."""
    problems = []
    for name, (method, _, _) in ARCHIVES.items():
        path = DIR / name
        if not path.is_file():
            problems.append(f"{name}: diskte yok (betiği --check olmadan çalıştırın)")
            continue
        try:
            with zipfile.ZipFile(path) as z:
                bad = z.testzip()
                if bad is not None:
                    problems.append(f"{name}: {bad} bozuk (CRC)")
                    continue
                infos = z.infolist()
                want = archive_members(name, files)
                if [i.filename for i in infos] != [m for m, _ in want]:
                    problems.append(f"{name}: içindekiler {[i.filename for i in infos]}, beklenen {[m for m, _ in want]}")
                    continue
                for info, (member, source) in zip(infos, want):
                    if info.compress_type != method:
                        problems.append(f"{name}: {member} yöntemi {info.compress_type}, beklenen {method}")
                    if z.read(info) != files[source]:
                        problems.append(f"{name}: {member} betiğin yazdığı {source} değil")
        except zipfile.BadZipFile as e:
            problems.append(f"{name}: açılamadı: {e}")
    return problems


def build():
    """Every file of the fixture directory but README.md: {file name: bytes}."""
    out = {}
    for name, files in fixture_sets().items():
        out.update(files)
        out[f"{name}.expected.json"] = gis.dumps(read_set(name, files)).encode("utf-8")
    return out


def verify(files):
    """Checks one version of the files (built in memory or read from disk)."""
    problems = []
    for name in SUMMARY:
        try:
            if name in GEOJSON:
                problems += geojson_holes(name, files[f"{name}.geojson"].decode("utf-8"))
            else:
                spec = SHAPEFILES[name]
                problems += verify_shapefile(name, spec, {ext: files[f"{name}{ext}"] for ext in (".shp", ".shx", ".dbf", ".prj", ".cpg") if f"{name}{ext}" in files})
                problems += polygon_roles(name, spec)
            result = read_set(name, files)
        except (gis.GisError, struct.error, ValueError, KeyError) as e:
            problems.append(f"{name}: okunamadı: {e}")
            continue
        problems += against_summary(name, result)
        if gis.dumps(result).encode("utf-8") != files[f"{name}.expected.json"]:
            problems.append(f"{name}: okuyucunun çıktısı {name}.expected.json değil")
    if set(SUMMARY) != set(GEOJSON) | set(SHAPEFILES):
        problems.append("SUMMARY her fixture'ı bir kez anlatmalı")
    return problems


def check(files):
    problems = []
    for rel, data in files.items():
        path = DIR / rel
        if not path.is_file():
            problems.append(f"{rel}: diskte yok (betiği --check olmadan çalıştırın)")
        elif path.read_bytes() != data:
            problems.append(f"{rel}: diskteki dosya yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)")
    if DIR.is_dir():
        for path in sorted(DIR.iterdir()):
            if path.name not in files and path.name not in ARCHIVES and path.name not in ("README.md", EXPORT.name):
                problems.append(f"{path.name}: betiğin yazmadığı dosya")
    problems += check_archives(files)
    if problems:
        return problems, []
    disk = {rel: (DIR / rel).read_bytes() for rel in files}
    problems += verify(disk)
    for name in SUMMARY:
        path = DIR / (f"{name}.geojson" if name in GEOJSON else f"{name}.shp")
        try:
            text = gis.dumps(gis.read_path(path, name))
        except gis.GisError as e:
            problems.append(f"{path.name}: okuyucu dosyadan okuyamadı: {e}")
            continue
        if text.encode("utf-8") != disk[f"{name}.expected.json"]:
            problems.append(f"{path.name}: okuyucunun dosyadan okuduğu {name}.expected.json değil")
    export_problems, pairs = check_exports()
    return problems + export_problems, pairs


def main():
    files = build()
    if "--check" in sys.argv[1:]:
        problems, pairs = check(files)
        for p in problems:
            print(p, file=sys.stderr)
        print(f"export/: {len(pairs)} çift denetlendi" + (f" ({', '.join(pairs)})" if pairs else " (dizin yok ya da boş)"))
        if problems:
            return 1
        print(f"GIS fixture'ları tutarlı: {len(files)} dosya, {len(SUMMARY)} fixture; her biri okuyucuyla expected.json'ı ve SUMMARY'yi veriyor; {len(ARCHIVES)} zip arşivinin içindekiler betiğin dosyaları.")
        return 0
    problems = verify(files)
    for p in problems:
        print(p, file=sys.stderr)
    if problems:
        print("Hiçbir dosya yazılmadı.", file=sys.stderr)
        return 1
    DIR.mkdir(parents=True, exist_ok=True)
    for rel, data in files.items():
        (DIR / rel).write_bytes(data)
    for name in ARCHIVES:
        (DIR / name).write_bytes(archive(name, files))
    print(f"{len(files) + len(ARCHIVES)} dosya yazıldı: {DIR.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
