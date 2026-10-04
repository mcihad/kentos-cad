#!/usr/bin/env python3
"""Independent reference of coordinate systems read from and written as WKT and PROJ strings (docs/adr/0168 §5).

Writes fixtures/geodesy/v1/text.json, no KentOS code:

- **reads:** WKT 1 (OGC and ESRI), WKT 2 and PROJ strings. PROJ (pyproj) reads each text; its PROJJSON gives the numbers
  (the ellipsoid, the projection's parameters in their units, the seven parameters to WGS 84), the ADR's rules give the
  core's system:
  - a datum is the registry's (TUREF, ED50, WGS 84) when its name says so (the names Shapefile's reader knows), its
    ellipsoid is that datum's, and its seven parameters to WGS 84 are absent or EPSG's (zero for TUREF and WGS 84,
    EPSG:1784's for ED50); any other datum is the text's own, with its name as written (an ESRI “D_” dropped,
    underscores as spaces) and its parameters (position vector in WKT 1 and in PROJ's `+towgs84`);
  - a PROJ string names no datum: `+datum=WGS84` is WGS 84, any other its ellipsoid's own datum (“Bessel 1841
    datumu”); a known ellipsoid takes the table's name, other numbers “a=…, 1/f=…”;
  - only the transverse Mercator (UTM too), latitude and longitude, and a local system derived by an affine transform
    from a projected one; metres and degrees (rotations in arc-seconds, scales in ppm); Greenwich. Anything else says
    what it cannot read: “syntax”, “unsupported”, “unit”, “meridian”, with the word it stopped at.
  - The EPSG code the text names at its root, and the registry's system it is (by the zones' rules), or the one whose
    grid it shares when its datum is the text's own.
- **writes:** a system written as WKT 1 (OGC, TOWGS84 in the position vector convention) and as a PROJ string, a local
  system as WKT 2 (DERIVEDPROJCRS, wrapped in BOUNDCRS when its datum is the project's); numbers in their shortest
  round-trip form. An abridged transformation's scale difference is the ratio 1 + ppm·1e-6, as PROJ writes and reads
  it. PROJ reads every text back to the same parameters, and a local system's points move as its plane
  transform moves them.

The core (crates/shared/geometry-core/tests/all/crs_text.rs) and the web through its WASM must give the same, exactly.
"""

import argparse
import json
import math
import re
import sys
from decimal import Decimal
from pathlib import Path

import pyproj
from pyproj import CRS, Transformer

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "geodesy" / "v1" / "text.json"

DEGREE = 0.0174532925199433
ARC_SECOND = 4.84813681109536e-06

REGISTRY_ELLIPSOIDS = {"TUREF": (6378137.0, 298.257222101), "ED50": (6378388.0, 297.0),
                       "WGS84": (6378137.0, 298.257223563)}
EPSG_TOWGS84 = {"TUREF": ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 0.0), "WGS84": ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 0.0),
                "ED50": ([-84.1, -101.8, -129.7], [0.0, 0.0, 0.468], 1.05)}

# The ellipsoids a PROJ string may name and the core's names for them (and for the same numbers anywhere).
ELLIPSOIDS = {
    "GRS80": ("GRS 1980", 6378137.0, 298.257222101),
    "WGS84": ("WGS 84", 6378137.0, 298.257223563),
    "intl": ("International 1924", 6378388.0, 297.0),
    "bessel": ("Bessel 1841", 6377397.155, 299.1528128),
    "krass": ("Krasovski 1940", 6378245.0, 298.3),
    "clrk80": ("Clarke 1880 (mod.)", 6378249.145, 293.4663),
    "helmert": ("Helmert 1906", 6378200.0, 298.3),
}


def num(x):
    """A number as Rust writes an f64 with `{}`: the shortest round trip, no exponent, no “.0”."""
    s = format(Decimal(repr(float(x))), "f")
    if "." in s:
        s = s.rstrip("0").rstrip(".")
    return s


def key(s):
    return "".join(c.lower() for c in s if c.isascii() and c.isalnum())


def shown(name):
    """A WKT name as the core keeps it: ESRI's “D_” dropped, underscores as spaces."""
    if name.startswith("D_"):
        name = name[2:]
    return name.replace("_", " ")


def registry_datum_named(name):
    k = key(name)
    if any(d in k for d in ["turef", "turkishnationalreferenceframe", "itrf96", "itrf1996",
                            "internationalterrestrialreferenceframe1996"]):
        return "TUREF"
    if any(d in k for d in ["european1950", "europeandatum1950", "ed50"]):
        return "ED50"
    if any(d in k for d in ["wgs1984", "wgs84", "worldgeodeticsystem1984"]):
        return "WGS84"
    return None


def ellipsoid_named(a, rf):
    for name, ea, erf in ELLIPSOIDS.values():
        if (a, rf) == (ea, erf):
            return name
    return f"a={num(a)}, 1/f={num(rf)}"


# ── Reading ───────────────────────────────────────────────────────────────────────────────────────────────────────


class Refused(Exception):
    def __init__(self, kind, detail):
        super().__init__(kind, detail)
        self.kind, self.detail = kind, detail


def first_name(text, word):
    m = re.search(word + r'\[\s*"([^"]*)"', text)
    return m.group(1) if m else None


def root_authority(text):
    t = text.strip()
    m = re.search(r'(?:AUTHORITY\[\s*"EPSG"\s*,\s*"(\d+)"\s*\]|ID\[\s*"EPSG"\s*,\s*(\d+)\s*\])\s*\]\s*$', t)
    return int(m.group(1) or m.group(2)) if m else None


def helmert_from(transformation, text):
    method = transformation["method"]["name"]
    if "Position Vector" in method or "Geocentric translations" in method:
        convention = "positionVector"
    elif "Coordinate Frame" in method:
        convention = "coordinateFrame"
    else:
        raise Refused("unsupported", method)
    values = {p["name"]: p for p in transformation["parameters"]}

    def get(name, unit_factor):
        p = values.get(name)
        if p is None:
            return 0.0
        return float(p["value"])

    t = [get("X-axis translation", 1), get("Y-axis translation", 1), get("Z-axis translation", 1)]
    r = [get("X-axis rotation", 1), get("Y-axis rotation", 1), get("Z-axis rotation", 1)]
    s = get("Scale difference", 1)
    # WKT 2's abridged transformation writes the scale as the ratio 1 + ppm·1e-6; PROJJSON gives the ppm to 15
    # figures only, so the exact value is worked out from the text's number the way PROJ does, and checked against it.
    abridged = re.search(r'ABRIDGEDTRANSFORMATION\[.*PARAMETER\[\s*"Scale difference"\s*,\s*([0-9.eE+-]+)', text)
    if abridged:
        exact = (float(abridged.group(1)) - 1.0) * 1e6
        assert abs(exact - s) < 1e-9, (exact, s)
        s = exact
    return {"translation": t, "rotation": r, "scale": s, "convention": convention}


def datum_of(base, text, towgs84, in_proj):
    """The core's datum for PROJJSON's geographic base: the registry's or the text's own."""
    d = base.get("datum") or base.get("datum_ensemble")
    e = d["ellipsoid"]
    a = float(e["semi_major_axis"])
    rf = float(e["inverse_flattening"]) if "inverse_flattening" in e else a / (a - float(e["semi_minor_axis"]))
    pm = base.get("prime_meridian")
    if pm is not None and float(pm.get("longitude", 0)) != 0.0:
        raise Refused("meridian", pm["name"])
    if in_proj:
        if "+datum=WGS84" in text:
            return "WGS84"
        ell = ellipsoid_named(a, rf)
        datum = {"name": f"{ell} datumu", "ellipsoid": {"name": ell, "semiMajor": a, "inverseFlattening": rf}}
        if towgs84 is not None:
            datum["toWgs84"] = towgs84
        return datum
    written = first_name(text, r"(?:DATUM|ENSEMBLE|TRF|GEODETICDATUM)")
    named = registry_datum_named(written)
    if named is not None and REGISTRY_ELLIPSOIDS[named] == (a, rf):
        if towgs84 is None:
            return named
        pv = towgs84["rotation"] if towgs84["convention"] == "positionVector" else [-r for r in towgs84["rotation"]]
        if (towgs84["translation"], pv, towgs84["scale"]) == EPSG_TOWGS84[named]:
            return named
    spheroid = first_name(text, r"(?:SPHEROID|ELLIPSOID)")
    datum = {"name": shown(written), "ellipsoid": {"name": shown(spheroid), "semiMajor": a, "inverseFlattening": rf}}
    if towgs84 is not None:
        datum["toWgs84"] = towgs84
    return datum


def tm_of(conversion):
    method = conversion["method"]["name"]
    if method != "Transverse Mercator":
        raise Refused("unsupported", method)
    by_code = {p["id"]["code"]: p for p in conversion["parameters"] if "id" in p}
    out = {}
    for code, field in [(8801, "latitudeOfOrigin"), (8802, "centralMeridian"), (8805, "scaleFactor"),
                        (8806, "falseEasting"), (8807, "falseNorthing")]:
        p = by_code.get(code)
        out[field] = float(p["value"]) if p is not None else (1.0 if code == 8805 else 0.0)
    return out


def system_of(d, text, in_proj, towgs84=None):
    kind = d["type"]
    if kind == "BoundCRS":
        target = d["target_crs"]
        tdatum = target.get("datum") or target.get("datum_ensemble")
        if registry_datum_named(tdatum["name"]) != "WGS84":
            raise Refused("unsupported", tdatum["name"])
        return system_of(d["source_crs"], text, in_proj, helmert_from(d["transformation"], text))
    if kind == "GeographicCRS":
        return {"kind": "geographic", "datum": datum_of(d, text, towgs84, in_proj)}
    if kind == "ProjectedCRS":
        tm = tm_of(d["conversion"])
        s = {"kind": "tm", "datum": datum_of(d["base_crs"], text, towgs84, in_proj)}
        if tm["latitudeOfOrigin"] != 0.0:
            s["latitudeOfOrigin"] = tm["latitudeOfOrigin"]
        for f in ["centralMeridian", "scaleFactor", "falseEasting", "falseNorthing"]:
            s[f] = tm[f]
        return s
    if kind == "DerivedProjectedCRS":
        base = system_of(d["base_crs"], text, in_proj, towgs84)
        conv = d["conversion"]
        if conv["method"]["name"] != "Affine parametric transformation":
            raise Refused("unsupported", conv["method"]["name"])
        v = {p["name"]: float(p["value"]) for p in conv["parameters"]}
        return {"kind": "local", "base": base, "plane": plane_from_deriving(v)}
    raise Refused("unsupported", kind)


def plane_from_deriving(v):
    """The plane this → base from WKT 2's deriving conversion base → this (X = A0 + A1·x + A2·y, Y = B0 + B1·x + B2·y),
    inverted as the core inverts it."""
    a, b, c, d, e, f = v["A1"], v["A2"], v["A0"], v["B1"], v["B2"], v["B0"]
    det = a * e - b * d
    ia, ib, id_, ie = e / det, -b / det, -d / det, a / det
    return {"kind": "affine", "a": ia, "b": ib, "c": -(ia * c + ib * f), "d": id_, "e": ie, "f": -(id_ * c + ie * f)}


def zone_srid(s):
    """The registry's system with this datum and grid, by the zones' rules (Shapefile's reader's)."""
    if s["kind"] == "geographic":
        return {"WGS84": 4326, "TUREF": 5252}.get(s["datum"]) if isinstance(s["datum"], str) else None
    if s["kind"] != "tm" or s.get("latitudeOfOrigin", 0.0) != 0.0:
        return None
    if (s["falseEasting"], s["falseNorthing"]) != (500000.0, 0.0):
        return None
    cm, k, d = s["centralMeridian"], s["scaleFactor"], s["datum"]
    three, six = [27.0, 30.0, 33.0, 36.0, 39.0, 42.0, 45.0], [27.0, 33.0, 39.0, 45.0]
    if d == "TUREF" and k == 1.0 and cm in three:
        return 5253 + int((cm - 27) / 3)
    if d == "ED50" and k == 1.0 and cm in three:
        return 2319 + int((cm - 27) / 3)
    if d == "ED50" and k == 0.9996 and cm in six:
        return 23035 + int((cm - 27) / 6)
    if d == "WGS84" and k == 0.9996 and cm in six:
        return 32635 + int((cm - 27) / 6)
    return None


def like_registry(s):
    """The datum the registry would give a system of the project's own datum on the same ellipsoid, for the
    suggestion: GRS80 TUREF, International 1924 ED50, WGS 84 WGS 84."""
    if s["kind"] not in ("tm", "geographic") or isinstance(s["datum"], str):
        return None
    e = s["datum"]["ellipsoid"]
    for name, ell in REGISTRY_ELLIPSOIDS.items():
        if (e["semiMajor"], e["inverseFlattening"]) == ell:
            return dict(s, datum=name)
    return None


def refuse_early(text):
    """What the ADR refuses before PROJ is asked (PROJ reads more than KentOS takes)."""
    t = text.strip()
    if t.startswith("+"):
        tokens = t.split()
        for tok in tokens:
            k, _, v = tok[1:].partition("=")
            if k == "proj" and v not in ("tmerc", "etmerc", "utm", "longlat", "latlong", "lonlat", "latlon"):
                raise Refused("unsupported", f"+proj={v}")
            if k == "units" and v != "m":
                raise Refused("unit", f"+units={v}")
            if k in ("nadgrids", "geoidgrids"):
                raise Refused("unsupported", f"+{k}")
            if k == "pm" and v not in ("greenwich", "0"):
                raise Refused("meridian", v)
            if k == "datum" and v != "WGS84":
                raise Refused("unsupported", f"+datum={v}")
            if k == "ellps" and v not in ELLIPSOIDS:
                raise Refused("unsupported", f"+ellps={v}")
        return
    m = re.match(r"\s*([A-Za-z_]+)\s*\[", t)
    if not m:
        raise Refused("syntax", "")
    root = m.group(1).upper()
    if root not in ("PROJCS", "GEOGCS", "PROJCRS", "PROJECTEDCRS", "GEOGCRS", "GEOGRAPHICCRS", "BOUNDCRS",
                    "DERIVEDPROJCRS"):
        raise Refused("unsupported", m.group(1))
    proj = re.search(r'PROJECTION\[\s*"([^"]*)"', t)
    if proj and "transversemercator" not in key(proj.group(1)):
        raise Refused("unsupported", proj.group(1))
    pm = re.search(r'PRIMEM\[\s*"([^"]*)"\s*,\s*([0-9.eE+-]+)', t)
    if pm and float(pm.group(2)) != 0.0:
        raise Refused("meridian", pm.group(1))
    for unit in re.finditer(r'UNIT\[\s*"([^"]*)"\s*,\s*([0-9.eE+-]+)', t):
        name, factor = unit.group(1), float(unit.group(2))
        if factor != 1.0 and abs(factor - DEGREE) > 1e-15 and name.lower() not in ("arc-second", "parts per million"):
            raise Refused("unit", name)


READS = [
    ("ESRI WKT 1: TUREF TM36", 'PROJCS["TUREF_TM36",GEOGCS["GCS_TUREF",DATUM["D_Turkish_National_Reference_Frame",'
     'SPHEROID["GRS_1980",6378137.0,298.257222101]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],'
     'PROJECTION["Transverse_Mercator"],PARAMETER["False_Easting",500000.0],PARAMETER["False_Northing",0.0],'
     'PARAMETER["Central_Meridian",36.0],PARAMETER["Scale_Factor",1.0],PARAMETER["Latitude_Of_Origin",0.0],'
     'UNIT["Meter",1.0]]'),
    ("ESRI WKT 1: ED50 UTM 36N", 'PROJCS["ED_1950_UTM_Zone_36N",GEOGCS["GCS_European_1950",DATUM["D_European_1950",'
     'SPHEROID["International_1924",6378388.0,297.0]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],'
     'PROJECTION["Transverse_Mercator"],PARAMETER["False_Easting",500000.0],PARAMETER["False_Northing",0.0],'
     'PARAMETER["Central_Meridian",33.0],PARAMETER["Scale_Factor",0.9996],PARAMETER["Latitude_Of_Origin",0.0],'
     'UNIT["Meter",1.0]]'),
    ("OGC WKT 1: ED50 TM30, EPSG'nin TOWGS84'üyle", 'PROJCS["ED50 / TM30",GEOGCS["ED50",DATUM["European_Datum_1950",'
     'SPHEROID["International 1924",6378388,297],TOWGS84[-84.1,-101.8,-129.7,0,0,0.468,1.05]],PRIMEM["Greenwich",0],'
     'UNIT["degree",0.0174532925199433]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",0],'
     'PARAMETER["central_meridian",30],PARAMETER["scale_factor",1],PARAMETER["false_easting",500000],'
     'PARAMETER["false_northing",0],UNIT["metre",1],AXIS["Easting",EAST],AXIS["Northing",NORTH]]'),
    ("OGC WKT 1: ED50 TM30, başka TOWGS84'le", 'PROJCS["ED50 / TM30",GEOGCS["ED50",DATUM["European_Datum_1950",'
     'SPHEROID["International 1924",6378388,297],TOWGS84[-87,-98,-121,0,0,0,0]],PRIMEM["Greenwich",0],'
     'UNIT["degree",0.0174532925199433]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",0],'
     'PARAMETER["central_meridian",30],PARAMETER["scale_factor",1],PARAMETER["false_easting",500000],'
     'PARAMETER["false_northing",0],UNIT["metre",1]]'),
    ("OGC WKT 1: kodu yazılı TUREF TM30", 'PROJCS["ITRF96 / TM30",GEOGCS["ITRF96",DATUM["International_Terrestrial_'
     'Reference_Frame_1996",SPHEROID["GRS 1980",6378137,298.257222101]],PRIMEM["Greenwich",0],UNIT["degree",'
     '0.0174532925199433]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",0],'
     'PARAMETER["central_meridian",30],PARAMETER["scale_factor",1],PARAMETER["false_easting",500000],'
     'PARAMETER["false_northing",0],UNIT["metre",1,AUTHORITY["EPSG","9001"]],AUTHORITY["EPSG","5254"]]'),
    ("OGC WKT 1: Bessel'li şehir sistemi", 'PROJCS["Şehir sistemi",GEOGCS["Bessel datumu",DATUM["Bessel_datumu",'
     'SPHEROID["Bessel 1841",6377397.155,299.1528128],TOWGS84[598.1,73.7,418.2,0.202,0.045,-2.455,6.7]],'
     'PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]],PROJECTION["Transverse_Mercator"],'
     'PARAMETER["latitude_of_origin",36],PARAMETER["central_meridian",34.5],PARAMETER["scale_factor",0.9999],'
     'PARAMETER["false_easting",200000],PARAMETER["false_northing",100000],UNIT["metre",1]]'),
    ("WKT 1: derecenin uzun yazılışı", 'GEOGCS["TUREF",DATUM["Turkish_National_Reference_Frame",SPHEROID["GRS 1980",'
     '6378137,298.257222101]],PRIMEM["Greenwich",0],UNIT["degree",0.017453292519943295]]'),
    ("ESRI WKT 1: WGS 84 coğrafi", 'GEOGCS["GCS_WGS_1984",DATUM["D_WGS_1984",SPHEROID["WGS_1984",6378137.0,'
     '298.257223563]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]]'),
    ("WKT 2: EPSG:5254", CRS.from_epsg(5254).to_wkt()),
    ("WKT 2: EPSG:4326 (topluluk)", CRS.from_epsg(4326).to_wkt()),
    ("WKT 2: EPSG:23036", CRS.from_epsg(23036).to_wkt()),
    ("WKT 2: Bessel'li TM'nin BOUNDCRS'ı", CRS.from_proj4(
        "+proj=tmerc +lat_0=0 +lon_0=33 +k=1 +x_0=500000 +y_0=0 +ellps=bessel "
        "+towgs84=598.1,73.7,418.2,0.202,0.045,-2.455,6.7 +units=m +no_defs").to_wkt()),
    ("WKT 2: afinle türetilmiş yerel sistem", 'DERIVEDPROJCRS["Şantiye",BASEPROJCRS["TUREF / TM30",'
     'BASEGEOGCRS["TUREF",DATUM["Turkish National Reference Frame",ELLIPSOID["GRS 1980",6378137,298.257222101,'
     'LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]]],CONVERSION["TM30",'
     'METHOD["Transverse Mercator",ID["EPSG",9807]],PARAMETER["Latitude of natural origin",0,ANGLEUNIT["degree",'
     '0.0174532925199433],ID["EPSG",8801]],PARAMETER["Longitude of natural origin",30,ANGLEUNIT["degree",'
     '0.0174532925199433],ID["EPSG",8802]],PARAMETER["Scale factor at natural origin",1,SCALEUNIT["unity",1],'
     'ID["EPSG",8805]],PARAMETER["False easting",500000,LENGTHUNIT["metre",1],ID["EPSG",8806]],'
     'PARAMETER["False northing",0,LENGTHUNIT["metre",1],ID["EPSG",8807]]]],DERIVINGCONVERSION["TM30 to Şantiye",'
     'METHOD["Affine parametric transformation",ID["EPSG",9624]],PARAMETER["A0",-412000,LENGTHUNIT["metre",1],'
     'ID["EPSG",8623]],PARAMETER["A1",0.99,SCALEUNIT["coefficient",1],ID["EPSG",8624]],PARAMETER["A2",0.1,'
     'SCALEUNIT["coefficient",1],ID["EPSG",8625]],PARAMETER["B0",-4512000,LENGTHUNIT["metre",1],ID["EPSG",8639]],'
     'PARAMETER["B1",-0.1,SCALEUNIT["coefficient",1],ID["EPSG",8640]],PARAMETER["B2",0.99,SCALEUNIT["coefficient",'
     '1],ID["EPSG",8641]]],CS[Cartesian,2],AXIS["(E)",east,ORDER[1],LENGTHUNIT["metre",1]],AXIS["(N)",north,'
     'ORDER[2],LENGTHUNIT["metre",1]]]'),
    ("PROJ: Bessel'li TM", "+proj=tmerc +lat_0=0 +lon_0=33 +k=1 +x_0=500000 +y_0=0 +ellps=bessel "
     "+towgs84=598.1,73.7,418.2,0.202,0.045,-2.455,6.7 +units=m +no_defs +type=crs"),
    ("PROJ: UTM 36, International 1924", "+proj=utm +zone=36 +ellps=intl +towgs84=-87,-98,-121,0,0,0,0 +units=m "
     "+no_defs"),
    ("PROJ: güney UTM", "+proj=utm +zone=35 +south +datum=WGS84 +units=m +no_defs"),
    ("PROJ: WGS 84 coğrafi", "+proj=longlat +datum=WGS84 +no_defs"),
    ("PROJ: a ve 1/f'yle GRS80", "+proj=tmerc +lat_0=0 +lon_0=33 +k=1 +x_0=500000 +y_0=0 +a=6378137 "
     "+rf=298.257222101 +towgs84=0,0,0,0,0,0,0 +units=m +no_defs"),
    ("PROJ: a ve 1/f'yle tanınmayan elipsoit", "+proj=longlat +a=6378300 +rf=297.5 +no_defs"),
    ("WKT 1: ABD ayağı", 'PROJCS["x",GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]],'
     'PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]],PROJECTION["Transverse_Mercator"],'
     'PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",33],PARAMETER["scale_factor",0.9996],'
     'PARAMETER["false_easting",500000],PARAMETER["false_northing",0],UNIT["US survey foot",0.304800609601219]]'),
    ("WKT 1: Lambert", 'PROJCS["Lambert",GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,'
     '298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]],'
     'PROJECTION["Lambert_Conformal_Conic_2SP"],PARAMETER["standard_parallel_1",37],'
     'PARAMETER["standard_parallel_2",41],PARAMETER["latitude_of_origin",39],PARAMETER["central_meridian",35],'
     'PARAMETER["false_easting",0],PARAMETER["false_northing",0],UNIT["metre",1]]'),
    ("WKT 1: Paris meridyeni", 'GEOGCS["NTF (Paris)",DATUM["Nouvelle_Triangulation_Francaise_Paris",'
     'SPHEROID["Clarke 1880 (IGN)",6378249.2,293.4660212936269]],PRIMEM["Paris",2.33722917],'
     'UNIT["grad",0.01570796326794897]]'),
    ("WKT 1: yerel sistem tanımı", 'LOCAL_CS["Şantiye",LOCAL_DATUM["x",0],UNIT["metre",1]]'),
    ("PROJ: Lambert", "+proj=lcc +lat_1=37 +lat_2=41 +lat_0=39 +lon_0=35 +datum=WGS84 +units=m +no_defs"),
    ("PROJ: ayak", "+proj=tmerc +lon_0=33 +k=1 +x_0=500000 +datum=WGS84 +units=us-ft +no_defs"),
    ("PROJ: ızgaralı datum", "+proj=longlat +ellps=intl +nadgrids=ed50.gsb +no_defs"),
    ("metin değil", "bir koordinat sistemi değil"),
]


def read_case(name, text):
    case = {"name": name, "text": text}
    in_proj = text.strip().startswith("+")
    try:
        refuse_early(text)
        d = CRS.from_user_input(text).to_json_dict()
        system = system_of(d, text, in_proj)
    except Refused as r:
        case["error"] = {"kind": r.kind, "detail": r.detail}
        return case
    sys_name = "PROJ tanımı" if in_proj else first_name(text, r"[A-Z]+").replace("_", " ")
    case["expect"] = {"name": sys_name, "system": system}
    authority = None if in_proj else root_authority(text)
    if authority is not None:
        case["expect"]["authority"] = authority
    srid = zone_srid(system) if system["kind"] != "local" else None
    if srid is not None:
        case["expect"]["registry"] = {"srid": srid, "exact": True}
    else:
        like = like_registry(system)
        srid = zone_srid(like) if like else None
        if srid is not None:
            case["expect"]["registry"] = {"srid": srid, "exact": False}
    return case


# ── Writing ───────────────────────────────────────────────────────────────────────────────────────────────────────

DATUM_WKT = {"TUREF": ("TUREF", "Turkish_National_Reference_Frame", "GRS 1980"),
             "ED50": ("ED50", "European_Datum_1950", "International 1924"),
             "WGS84": ("WGS 84", "WGS_1984", "WGS 84")}
PROJ_CODES = {(ea, erf): code for code, (_, ea, erf) in ELLIPSOIDS.items()}


def pv(h):
    return h["rotation"] if h["convention"] == "positionVector" else [-r for r in h["rotation"]]


def towgs84_numbers(datum):
    if isinstance(datum, str):
        if datum == "WGS84":
            return None
        t, r, s = EPSG_TOWGS84[datum]
        return t + r + [s]
    h = datum.get("toWgs84")
    return None if h is None else h["translation"] + pv(h) + [h["scale"]]


def ellipsoid_numbers(datum):
    if isinstance(datum, str):
        return REGISTRY_ELLIPSOIDS[datum]
    e = datum["ellipsoid"]
    return e["semiMajor"], e["inverseFlattening"]


def geogcs(datum):
    a, rf = ellipsoid_numbers(datum)
    if isinstance(datum, str):
        geog, dname, ename = DATUM_WKT[datum]
    else:
        geog, dname, ename = datum["name"], datum["name"].replace(" ", "_"), datum["ellipsoid"]["name"]
    towgs = towgs84_numbers(datum)
    tw = f",TOWGS84[{','.join(num(x) for x in towgs)}]" if towgs is not None else ""
    return (f'GEOGCS["{geog}",DATUM["{dname}",SPHEROID["{ename}",{num(a)},{num(rf)}]{tw}],PRIMEM["Greenwich",0],'
            f'UNIT["degree",0.0174532925199433]]')


def tm_wkt1(name, s):
    return (f'PROJCS["{name}",{geogcs(s["datum"])},PROJECTION["Transverse_Mercator"],'
            f'PARAMETER["latitude_of_origin",{num(s.get("latitudeOfOrigin", 0.0))}],'
            f'PARAMETER["central_meridian",{num(s["centralMeridian"])}],'
            f'PARAMETER["scale_factor",{num(s["scaleFactor"])}],PARAMETER["false_easting",{num(s["falseEasting"])}],'
            f'PARAMETER["false_northing",{num(s["falseNorthing"])}],UNIT["metre",1],AXIS["Easting",EAST],'
            f'AXIS["Northing",NORTH]]')


def proj_datum(datum):
    if datum == "WGS84":
        return "+datum=WGS84"
    a, rf = ellipsoid_numbers(datum)
    code = PROJ_CODES.get((a, rf))
    ell = f"+ellps={code}" if code else f"+a={num(a)} +rf={num(rf)}"
    towgs = towgs84_numbers(datum)
    return ell + (f" +towgs84={','.join(num(x) for x in towgs)}" if towgs is not None else "")


def write_proj(s):
    if s["kind"] == "geographic":
        return f"+proj=longlat {proj_datum(s['datum'])} +no_defs +type=crs"
    if s["kind"] == "tm":
        return (f"+proj=tmerc +lat_0={num(s.get('latitudeOfOrigin', 0.0))} +lon_0={num(s['centralMeridian'])} "
                f"+k={num(s['scaleFactor'])} +x_0={num(s['falseEasting'])} +y_0={num(s['falseNorthing'])} "
                f"{proj_datum(s['datum'])} +units=m +no_defs +type=crs")
    return None


def plane_coefficients(plane):
    if plane["kind"] == "affine":
        return [plane[k] for k in "abcdef"]
    th = plane["rotation"] * (math.pi / 180.0)
    c, s = math.cos(th), math.sin(th)
    k = plane["scale"]
    return [k * c, -k * s, plane["east"], k * s, k * c, plane["north"]]


def deriving(plane):
    """WKT 2's deriving conversion base → this: the plane's inverse, as the core inverts it."""
    a, b, c, d, e, f = plane_coefficients(plane)
    det = a * e - b * d
    ia, ib, id_, ie = e / det, -b / det, -d / det, a / det
    return {"A0": -(ia * c + ib * f), "A1": ia, "A2": ib, "B0": -(id_ * c + ie * f), "B1": id_, "B2": ie}


DEG = 'ANGLEUNIT["degree",0.0174532925199433]'
METRE = 'LENGTHUNIT["metre",1]'


def basegeogcrs(datum, word="BASEGEOGCRS"):
    a, rf = ellipsoid_numbers(datum)
    if isinstance(datum, str):
        geog, dname, ename = {"TUREF": ("TUREF", "Turkish National Reference Frame", "GRS 1980"),
                              "ED50": ("ED50", "European Datum 1950", "International 1924"),
                              "WGS84": ("WGS 84", "World Geodetic System 1984", "WGS 84")}[datum]
    else:
        geog, dname, ename = datum["name"], datum["name"], datum["ellipsoid"]["name"]
    return (f'{word}["{geog}",DATUM["{dname}",ELLIPSOID["{ename}",{num(a)},{num(rf)},{METRE}]],'
            f'PRIMEM["Greenwich",0,{DEG}]]')


def write_local(name, s, base_name):
    base = s["base"]
    if base["kind"] != "tm":
        return None
    v = deriving(s["plane"])
    params = []
    for k, eid in [("A0", 8623), ("A1", 8624), ("A2", 8625), ("B0", 8639), ("B1", 8640), ("B2", 8641)]:
        unit = METRE if k.endswith("0") else 'SCALEUNIT["coefficient",1]'
        params.append(f'PARAMETER["{k}",{num(v[k])},{unit},ID["EPSG",{eid}]]')
    text = (f'DERIVEDPROJCRS["{name}",BASEPROJCRS["{base_name}",{basegeogcrs(base["datum"])},'
            f'CONVERSION["{base_name}",METHOD["Transverse Mercator",ID["EPSG",9807]],'
            f'PARAMETER["Latitude of natural origin",{num(base.get("latitudeOfOrigin", 0.0))},{DEG},ID["EPSG",8801]],'
            f'PARAMETER["Longitude of natural origin",{num(base["centralMeridian"])},{DEG},ID["EPSG",8802]],'
            f'PARAMETER["Scale factor at natural origin",{num(base["scaleFactor"])},SCALEUNIT["unity",1],'
            f'ID["EPSG",8805]],PARAMETER["False easting",{num(base["falseEasting"])},{METRE},ID["EPSG",8806]],'
            f'PARAMETER["False northing",{num(base["falseNorthing"])},{METRE},ID["EPSG",8807]]]],'
            f'DERIVINGCONVERSION["{name}",METHOD["Affine parametric transformation",ID["EPSG",9624]],'
            f'{",".join(params)}],CS[Cartesian,2],AXIS["(E)",east,ORDER[1],{METRE}],'
            f'AXIS["(N)",north,ORDER[2],{METRE}]]')
    towgs = towgs84_numbers(base["datum"]) if not isinstance(base["datum"], str) else None
    if towgs is None:
        return text
    names = ["X-axis translation", "Y-axis translation", "Z-axis translation", "X-axis rotation", "Y-axis rotation",
             "Z-axis rotation", "Scale difference"]
    ids = [8605, 8606, 8607, 8608, 8609, 8610, 8611]
    # PROJ writes and reads an abridged transformation's scale difference as the ratio 1 + ppm·1e-6.
    values = towgs[:6] + [1.0 + towgs[6] * 1e-6]
    tparams = ",".join(f'PARAMETER["{n}",{num(x)},ID["EPSG",{i}]]' for n, x, i in zip(names, values, ids))
    wgs84 = basegeogcrs("WGS84", "GEOGCRS")[:-1] + (f',CS[ellipsoidal,2],AXIS["latitude",north,ORDER[1],{DEG}],'
                                                   f'AXIS["longitude",east,ORDER[2],{DEG}]]')
    return (f'BOUNDCRS[SOURCECRS[{text}],TARGETCRS[{wgs84}],'
            f'ABRIDGEDTRANSFORMATION["{name} to WGS 84",METHOD["Position Vector transformation (geog2D domain)",'
            f'ID["EPSG",9606]],{tparams}]]')


BESSEL = {"name": "Bessel datumu", "ellipsoid": {"name": "Bessel 1841", "semiMajor": 6377397.155,
                                                  "inverseFlattening": 299.1528128},
          "toWgs84": {"translation": [598.1, 73.7, 418.2], "rotation": [0.202, 0.045, -2.455], "scale": 6.7,
                      "convention": "positionVector", "accuracy": 1.5}}
BESSEL_CF = dict(BESSEL, toWgs84=dict(BESSEL["toWgs84"], rotation=[-0.202, -0.045, 2.455],
                                      convention="coordinateFrame"))
LONE = {"name": "Bağsız datum", "ellipsoid": {"name": "Özel elipsoit", "semiMajor": 6378300.0,
                                              "inverseFlattening": 297.5}}
CITY = {"kind": "tm", "datum": "TUREF", "latitudeOfOrigin": 36.0, "centralMeridian": 34.5, "scaleFactor": 0.9999,
        "falseEasting": 200000.0, "falseNorthing": 100000.0}
TUREF_TM33 = {"kind": "tm", "datum": "TUREF", "centralMeridian": 33.0, "scaleFactor": 1.0, "falseEasting": 500000.0,
              "falseNorthing": 0.0}
ED50_GK = {"kind": "tm", "datum": "ED50", "centralMeridian": 33.0, "scaleFactor": 1.0, "falseEasting": 400000.0,
           "falseNorthing": 0.0}
BESSEL_TM = {"kind": "tm", "datum": BESSEL, "centralMeridian": 33.0, "scaleFactor": 1.0, "falseEasting": 500000.0,
             "falseNorthing": 0.0}

WRITES = [
    # name, the definition's name, system, the base's name (a local system's)
    ("TUREF'li şehir sistemi", "Şehir sistemi", CITY, None),
    ("ED50'li sistem", "ED50 Gauss-Krüger 33", ED50_GK, None),
    ("Bessel datumlu TM", "Bessel TM33", BESSEL_TM, None),
    ("aynısı, koordinat çerçevesi kuralıyla", "Bessel TM33", dict(BESSEL_TM, datum=BESSEL_CF), None),
    ("WGS 84 coğrafi", "WGS 84", {"kind": "geographic", "datum": "WGS84"}, None),
    ("bağsız datumlu coğrafi", "Bağsız", {"kind": "geographic", "datum": LONE}, None),
    ("TUREF TM33'e bağlı şantiye sistemi", "Şantiye", {"kind": "local", "base": TUREF_TM33, "plane": {
        "kind": "similarity", "east": 492345.678, "north": 4422345.678, "rotation": 12.5, "scale": 1.000012}},
     "TUREF / TM33"),
    ("Bessel datumlu TM'ye bağlı afin sistem", "Afin şantiye", {"kind": "local", "base": BESSEL_TM, "plane": {
        "kind": "affine", "a": 1.00002, "b": -0.0003, "c": 492000.0, "d": 0.00025, "e": 0.99998, "f": 4422500.0}},
     "Bessel TM33"),
]


def check_written(case):
    """PROJ reads the written texts back to the same parameters; a local system's points move as its plane moves
    them."""
    s = case["system"]
    for text in [case["wkt"], case["proj"]]:
        if text is None:
            continue
        d = CRS.from_user_input(text).to_json_dict()
        got = system_of(d, text, text.startswith("+"))
        if s["kind"] == "local":
            assert got["kind"] == "local", case["name"]
            derived = CRS.from_user_input(text)
            if derived.type_name.startswith("Bound"):
                derived = derived.source_crs
            t = Transformer.from_crs(derived, derived.source_crs, always_xy=True)
            for x, y in [(0.0, 0.0), (1234.567, -876.543)]:
                a, b, c, dd, e, f = plane_coefficients(s["plane"])
                want = (a * x + b * y + c, dd * x + e * y + f)
                px, py = t.transform(x, y)
                # WKT 2 gives a derived system's base no axes of its own; PROJ lists that base's northing first.
                if abs(px - want[1]) < 1e-6 and abs(py - want[0]) < 1e-6:
                    px, py = py, px
                assert abs(px - want[0]) < 1e-6 and abs(py - want[1]) < 1e-6, (case["name"], px, py, want)
            continue

        def numbers(sys_):
            dd = sys_["datum"]
            out = dict(sys_)
            out["datum"] = (ellipsoid_numbers(dd), towgs84_numbers(dd))
            out.pop("kind")
            out.setdefault("latitudeOfOrigin", 0.0)
            return out

        assert numbers(got) == numbers(s), (case["name"], text, got)


def write_case(name, def_name, system, base_name):
    case = {"name": name, "definition": {"name": def_name, "system": system}}
    if system["kind"] == "local":
        case["wkt"] = write_local(def_name, system, base_name)
        case["proj"] = None
    else:
        case["wkt"] = tm_wkt1(def_name, system) if system["kind"] == "tm" else geog_named(def_name, system["datum"])
        case["proj"] = write_proj(system)
    check_written(dict(case, system=system))
    return case


def geog_named(name, datum):
    """A geographic system's WKT 1, its own name first."""
    inner = geogcs(datum)
    return f'GEOGCS["{name}"' + inner[inner.index(","):]


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="fail when the fixture is not what PROJ and the rules give")
    args = ap.parse_args()
    data = {"format": "kentos.crs-text", "version": 1, "proj": pyproj.proj_version_str,
            "reads": [read_case(n, t) for n, t in READS],
            "writes": [write_case(*w) for w in WRITES]}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} PROJ'dan ve kurallardan çıkan değil; yeniden yazın: "
                  f"python3 {Path(__file__).relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} PROJ'dan ve kurallardan çıkanla aynı.")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(data['reads'])} okuma, {len(data['writes'])} yazma.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
