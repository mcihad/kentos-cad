#!/usr/bin/env python3
"""The shared cases of `cad.rasters.georef`'s targets given in a system that is a definition (docs/adr/0594, plan/gis.md
CBS-29).

    python3 scripts/fixtures/raster_georef_crs_cases.py           # writes fixtures/commands/v1/cad.rasters.georef.crs.json
    python3 scripts/fixtures/raster_georef_crs_cases.py --check   # writes nothing; compares

Without KentOS code: the targets are written in each definition's system from the project's TUREF / TM33 targets by
PROJ (GDAL's osr), rounded to the millimetre, and carried back to TM33 by PROJ; the transforms, residuals, m0, check
errors and grids are raster_georef_command_cases.py's (numpy, held to GDAL there). ADR 0594's rules:

- `crs` is a WKT (1 or 2) or a PROJ string; one that is a registered system exactly is recorded as its EPSG code
  (`srid`), the project's own as nothing; any other is recorded as given (`crs`) and the targets carried from it;
- the WKT definitions here keep TUREF's datum by its name: the carrying is PROJ's transverse Mercator on GRS80 both
  ways, no datum step; the PROJ string's null `+towgs84` puts it on WGS 84, which reaches TUREF by EPSG's null
  geographic transformation (TUREF to WGS 84 (1): the same latitude and longitude, the ellipsoid GRS80's): the
  geocentric step from GRS80 to WGS 84's ellipsoid is written here (PROJ leaves out a null shift's change of ellipsoid),
  the latitude and longitude are then TUREF's: a tenth of a millimetre south of the purely projected point;
- `srid` and `crs` together, an empty definition, one longer than 16 KB are `invalid_input`; a text KentOS does not
  read is `crs_text_unreadable` in the words of the project's Özel koordinat sistemi window; a local project carries
  nothing (`target_not_converted`).
"""
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import raster_georef_command_cases as base  # noqa: E402
from osgeo import osr  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.rasters.georef.crs.json"
ANY = "$any"

GRS80 = "+ellps=GRS80 +units=m +no_defs"
TM33 = f"+proj=tmerc +lat_0=0 +lon_0=33 +k=1 +x_0=500000 +y_0=0 {GRS80}"


def proj(text):
    s = osr.SpatialReference()
    s.ImportFromProj4(text)
    s.SetAxisMappingStrategy(osr.OAMS_TRADITIONAL_GIS_ORDER)
    return s


def tm_wkt1(name, lon0, k, x0):
    """A TM on TUREF's datum as WKT 1, the datum by TUREF's name."""
    return (f'PROJCS["{name}",GEOGCS["TUREF",DATUM["Turkish_National_Reference_Frame",SPHEROID["GRS 1980",6378137,'
            f'298.257222101]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]],'
            f'PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",{lon0}],'
            f'PARAMETER["scale_factor",{k}],PARAMETER["false_easting",{x0}],PARAMETER["false_northing",0],'
            f'UNIT["metre",1]]')


def tm_wkt2(name, lon0, k, x0):
    return (f'PROJCRS["{name}",BASEGEOGCRS["TUREF",DATUM["Turkish National Reference Frame",ELLIPSOID["GRS 1980",'
            f'6378137,298.257222101,LENGTHUNIT["metre",1]]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]]],'
            f'CONVERSION["Transverse Mercator",METHOD["Transverse Mercator"],'
            f'PARAMETER["Latitude of natural origin",0,ANGLEUNIT["degree",0.0174532925199433]],'
            f'PARAMETER["Longitude of natural origin",{lon0},ANGLEUNIT["degree",0.0174532925199433]],'
            f'PARAMETER["Scale factor at natural origin",{k},SCALEUNIT["unity",1]],'
            f'PARAMETER["False easting",{x0},LENGTHUNIT["metre",1]],PARAMETER["False northing",0,LENGTHUNIT["metre",1]]],'
            f'CS[Cartesian,2],AXIS["easting",east,ORDER[1],LENGTHUNIT["metre",1]],'
            f'AXIS["northing",north,ORDER[2],LENGTHUNIT["metre",1]]]')


GEOG_TUREF = ('GEOGCS["TUREF",DATUM["Turkish_National_Reference_Frame",SPHEROID["GRS 1980",6378137,298.257222101]],'
              'PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]]')

# Each definition: its text, how PROJ sees it, what the record keeps (crs given, an EPSG code, nothing), its name.
DEFS = {
    "tm335": (tm_wkt1("TUREF / TM 33.5 yerel", 33.5, 1, 500000), f"+proj=tmerc +lat_0=0 +lon_0=33.5 +k=1 +x_0=500000 +y_0=0 {GRS80}"),
    "wkt2": (tm_wkt2("Belediye TM", 32.75, 0.9999, 400000), f"+proj=tmerc +lat_0=0 +lon_0=32.75 +k=0.9999 +x_0=400000 +y_0=0 {GRS80}"),
    "proj": ("+proj=tmerc +lat_0=0 +lon_0=34 +k=1 +x_0=500000 +y_0=0 +ellps=GRS80 +towgs84=0,0,0,0,0,0,0 +units=m +no_defs",
             f"+proj=tmerc +lat_0=0 +lon_0=34 +k=1 +x_0=500000 +y_0=0 {GRS80}"),
    "tm30": (tm_wkt1("TUREF / TM30", 30, 1, 500000), f"+proj=tmerc +lat_0=0 +lon_0=30 +k=1 +x_0=500000 +y_0=0 {GRS80}"),
    "geog": (GEOG_TUREF, "+proj=longlat +ellps=GRS80 +no_defs"),
    "tm33": (tm_wkt1("TUREF / TM33", 33, 1, 500000), TM33),
}


def ecef(lon, lat, a, rf):
    f = 1 / rf
    e2 = f * (2 - f)
    lo, la = math.radians(lon), math.radians(lat)
    n = a / math.sqrt(1 - e2 * math.sin(la) ** 2)
    return n * math.cos(la) * math.cos(lo), n * math.cos(la) * math.sin(lo), n * (1 - e2) * math.sin(la)


def geodetic(x, y, z, a, rf):
    f = 1 / rf
    e2 = f * (2 - f)
    p = math.hypot(x, y)
    lat = math.atan2(z, p * (1 - e2))
    for _ in range(30):
        n = a / math.sqrt(1 - e2 * math.sin(lat) ** 2)
        lat = math.atan2(z + e2 * n * math.sin(lat), p)
    return math.degrees(math.atan2(y, x)), math.degrees(lat)


def wgs84_step(key):
    """The PROJ string's way back to TM33: its latitude and longitude on GRS80 (PROJ's inverse TM), the null `+towgs84`
    as the geocentric step to WGS 84's ellipsoid (written here: PROJ leaves a null shift's ellipsoid change out), the
    latitude and longitude taken as TUREF's (EPSG's null transformation), then TM33 (PROJ); none for the WKT ones."""
    if key != "proj":
        return None
    to_geo = osr.CoordinateTransformation(proj(DEFS[key][1]), proj("+proj=longlat +ellps=GRS80 +no_defs"))
    to_tm = osr.CoordinateTransformation(proj("+proj=longlat +ellps=GRS80 +no_defs"), proj(TM33))

    def back(x, y):
        lon, lat, _ = to_geo.TransformPoint(x, y)
        lon, lat = geodetic(*ecef(lon, lat, 6378137.0, 298.257222101), 6378137.0, 298.257223563)
        e, n, _ = to_tm.TransformPoint(lon, lat)
        return e, n
    return back


def given_in(key, points):
    """The points with their targets written in the definition's system (PROJ, rounded as typed: millimetres, a
    geographic one to 10⁻⁹°), and the targets carried back to TM33 by PROJ."""
    to = osr.CoordinateTransformation(proj(TM33), proj(DEFS[key][1]))
    plain = osr.CoordinateTransformation(proj(DEFS[key][1]), proj(TM33))
    back = wgs84_step(key) or (lambda x, y: plain.TransformPoint(x, y)[:2])
    digits = 9 if key == "geog" else 3
    out, carried = [], []
    for p in points:
        x, y, _ = to.TransformPoint(*p["target"])
        t = [round(x, digits), round(y, digits)]
        out.append({**p, "target": t})
        carried.append(tuple(back(*t)))
    return out, carried


def georef_record(method, points, s, source, *, srid=None, crs=None, output=None):
    g = base.record(method, points, s, source, srid=srid, output=output)
    if crs is not None:
        g["crs"] = crs
    return g


cases = []


def add(name, setup, input_, result, expect=None, op="execute", within=None):
    step = {"op": op, "input": input_, "result": result}
    if within is not None:
        step["within"] = within
    if expect is not None:
        step["expect"] = expect
    elif result["status"] != "completed" or op != "execute":
        step["expect"] = {"canUndo": False, "revision": "same"}
    cases.append({"name": name, "setup": setup, "steps": [step]})


def placed(name, key, method, points, *, recorded="crs", op="execute"):
    """A placing method on the gray sheet, targets given in `key`'s system."""
    pts, carried = given_in(key, points)
    e = base.raster(file=base.GRAY)
    s = base.solved(method, pts, carried=carried)
    source = {"width": 333, "height": 257, "file": base.GRAY, "digest": base.sha(base.GRAY)}
    text = DEFS[key][0]
    srid = recorded if isinstance(recorded, int) else None
    crs = text if recorded == "crs" else None
    f = base.fields_of(e)
    f.update({"affine": [base.r9(v) for v in base.affine_of(s["fwd"])], "srid": base.SRID,
              "georef": georef_record(method, pts, s, source, srid=srid, crs=crs)})
    warns = [] if s["checks"] else [base.NO_CHECKS]
    expect = None
    if op == "execute":
        expect = {"canUndo": True, "revision": "changed"}
    add(name, base.drawing([e]), {"uid": "$uidOf:1", "method": method, "points": pts, "crs": text},
        base.completed(f, s, warnings=warns), expect=expect, op=op, within=1e-5)


def resampled(name, key, method, points, *, pixel=None):
    """A resampling method planned, targets given in `key`'s system: the grid from the carried targets."""
    pts, carried = given_in(key, points)
    e = base.raster(file=base.GRAY)
    s = base.solved(method, pts, carried=carried)
    output = base.grid_of(s, pixel)
    source = {"width": 333, "height": 257, "file": base.GRAY, "digest": base.sha(base.GRAY)}
    kept = base.GRAY.rsplit("/", 1)[0] + "/gray-lzw-strips-oturtulmus.tif"
    f = base.fields_of(e)
    f.update({"affine": output["affine"], "width": output["width"], "height": output["height"], "bands": 4,
              "sample": "u8", "file": kept, "srid": base.SRID, "style": {"render": "rgb", "bands": [1, 2, 3, 4]},
              "georef": georef_record(method, pts, s, source, crs=DEFS[key][0], output=output)})
    inp = {"uid": "$uidOf:1", "method": method, "points": pts, "crs": DEFS[key][0]}
    if pixel is not None:
        inp["pixel"] = pixel
    warns = [] if s["checks"] else [base.NO_CHECKS]
    add(name, base.drawing([e]), inp, base.completed(f, s, written=kept, warnings=warns), op="plan",
        within=base.PIXEL_WITHIN if pixel is None else 1e-5)


S = base.STRAIGHT
B = base.BENT
roles = base.with_roles

# Recorded as the definition.
placed("Afin: hedefler TUREF datumlu, 33,5° meridyenli TM tanımında (WKT 1); kayıtta tanım", "tm335", "affine",
       roles(S[:4] + S[9:11], checks=(4, 5), names=True))
placed("Helmert: belediyenin WKT 2 tanımı (32,75°, k 0,9999, 400 000 m)", "wkt2", "helmert", roles(S[:5] + S[10:11], checks=(5,)))
placed("Afin: PROJ dizesi, sıfır +towgs84; denetim noktası yoksa uyarılır", "proj", "affine", roles(S[:6]))
placed("Afin: kullanılmayan nokta tanımın sisteminden de çevrilir, dönüşüme girmez", "wkt2", "affine",
       roles(S[:7], checks=(6,), off=(5,)))
placed("Plan: afin, tanımla, yazmadan", "tm335", "affine", roles(S[:8], checks=(6, 7)), op="plan")
placed("Helmert: yalnız iki kontrol noktası ve bir denetim noktası, PROJ dizesiyle", "proj", "helmert",
       roles(S[:3], checks=(2,)))
# Recorded as the registry's code, or as nothing.
placed("Afin: tanım kayıttaki TUREF / TM30'un kendisi; kayda EPSG:5254 yazılır", "tm30", "affine",
       roles(S[:5], checks=(4,)), recorded=5254)
placed("Afin: tanım kayıttaki TUREF coğrafi sistemi; kayda EPSG:5252 yazılır", "geog", "affine",
       roles(S[:5], checks=(4,)), recorded=5252)
placed("Helmert: tanım projenin kendi sistemi (TUREF / TM33); çevrilmez, kayda yazılmaz", "tm33", "helmert",
       roles(S[:4], checks=(3,)), recorded=None)
# Resampled, planned.
resampled("Plan: projektif, hedefler 33,5° TM tanımında, 0,5 m piksel; ızgara çevrilen hedeflerden", "tm335",
          "projective", roles(B[:8] + B[9:10], checks=(8,)), pixel=0.5)
resampled("Plan: polinom 2, WKT 2 tanımı, 0,6 m piksel", "wkt2", "poly2", roles(B[:9] + B[9:10], checks=(9,)), pixel=0.6)
resampled("Plan: ince plaka, PROJ dizesi; kontrol noktalarından tam geçer", "proj", "thinPlate",
          roles(B[:9] + B[10:12], checks=(9, 10)))

# Refusals.
OK = roles(S[:5], checks=(4,))
grey = base.drawing([base.raster(file=base.GRAY)])


def refusal(name, input_, code, message, path, setup=grey):
    add(name, setup, {"uid": "$uidOf:1", "method": "affine", "points": OK, **input_},
        base.refused(code, message, path))


refusal("Ret: hem EPSG kodu hem tanım", {"srid": 4326, "crs": DEFS["tm335"][0]}, "invalid_input",
        "Hedeflerin sistemi ya EPSG kodu (srid) ya tanım metni (crs) olarak verilir, ikisi birden değil.", "crs")
refusal("Ret: boş tanım", {"crs": "   "}, "invalid_input",
        "Hedeflerin sisteminin tanımı boş olmamalı, en çok 16 KB olmalı.", "crs")
refusal("Ret: 16 KB'tan uzun tanım", {"crs": "x" * (16 * 1024 + 1)}, "invalid_input",
        "Hedeflerin sisteminin tanımı boş olmamalı, en çok 16 KB olmalı.", "crs")
refusal("Ret: WKT de PROJ dizesi de olmayan metin", {"crs": "Türkiye'nin yerel sistemi"}, "crs_text_unreadable",
        "Hedeflerin sisteminin tanımı okunamadı. Metin okunamadı: WKT (PROJCS[…], GEOGCS[…], PROJCRS[…] …) ya da +proj= ile "
        "başlayan bir PROJ dizesi yapıştırın.", "crs")
refusal("Ret: Lambert izdüşümü okunmaz", {"crs": "+proj=lcc +lat_1=36 +lat_2=42 +lat_0=39 +lon_0=35 +x_0=0 +y_0=0 "
                                                 "+ellps=GRS80 +units=m"}, "crs_text_unreadable", ANY, "crs")
refusal("Ret: ayak birimli tanım okunmaz", {"crs": "+proj=tmerc +lat_0=0 +lon_0=33 +k=1 +x_0=500000 +y_0=0 "
                                                   "+ellps=GRS80 +units=us-ft"}, "crs_text_unreadable", ANY, "crs")
refusal("Ret: yerel projede tanımla verilen hedefler", {"crs": DEFS["tm335"][0]}, "target_not_converted",
        "Hedefler “TUREF / TM 33.5 yerel” sisteminde; projenin koordinat sistemi yok (yerel proje). Hedefleri projenin "
        "koordinatlarıyla verin.", "crs", setup=base.drawing([base.raster(file=base.GRAY)], settings={"srid": 0}))
refusal("Ret: tanım kayıttaki bir sistem ama yerel projede", {"crs": DEFS["tm30"][0]}, "target_not_converted",
        "Hedefler EPSG:5254 sisteminde; projenin koordinat sistemi yok (yerel proje). Hedefleri projenin koordinatlarıyla "
        "verin.", "crs", setup=base.drawing([base.raster(file=base.GRAY)], settings={"srid": 0}))


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.rasters.georef", "commandVersion": 1,
            "title": "Raster oturt: hedeflerin sistemi tanımla (WKT, PROJ)",
            "note": "Üretici: scripts/fixtures/raster_georef_crs_cases.py (docs/adr/0594); hedefler PROJ'la, dönüşümler "
                    "raster_georef_command_cases.py'nin numpy hesabıyla, KentOS kodu olmadan.",
            "setup": base.drawing(), "cases": cases}


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or not base.numbers_close(json.loads(OUT.read_text(encoding="utf-8")), json.loads(text)):
            sys.exit(f"{OUT.relative_to(ROOT)} güncel değil")
        print(f"{OUT.relative_to(ROOT)}: {len(cases)} durum tutarlı")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)}: {len(cases)} durum yazıldı")


if __name__ == "__main__":
    main()
