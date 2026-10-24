#!/usr/bin/env python3
"""Independent reference of the heights' system a file names on import (docs/adr/0631 §4, plan/gis.md CBS-08).

    .run/geo-py/bin/python scripts/fixtures/vertical_import_cases.py --files   # GDAL writes fixtures/vertical/v1
    .run/geo-py/bin/python scripts/fixtures/vertical_import_cases.py           # writes fixtures/commands/v1/file.import.vertical.json
    .run/geo-py/bin/python scripts/fixtures/vertical_import_cases.py --check   # compares

No KentOS code. GDAL writes a point with a height in TUREF / TM36 under a compound system, the way each format keeps
one: a Shapefile's .prj as OGC's COMPD_CS and as ESRI's PROJCS[…],VERTCS[…] (GDAL's own ESRI export); a GeoJSON's
`crs` member as an OGC compound URN; a GeoPackage's srs under GDAL's own id (organization NONE) with the COMPD_CS
definition. The heights' system each names is read back by GDAL (OSR: the vertical system's name and its EPSG
authority; an ESRI VERTCS by its own name, without an authority): the import takes the horizontal half (TUREF / TM36, the project's: no
move, no warning of the system) and says the vertical one against the project's heights, the heights as they are.
"""

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures" / "vertical" / "v1"
OUT = ROOT / "fixtures" / "commands" / "v1" / "file.import.vertical.json"
REL = "vertical/v1"

CODES = {"antalya": 5775, "egm2008": 3855, "navd88": 5703}
TITLES = {5775: "ortometrik, Antalya height (EPSG:5775)", 3855: "ortometrik, EGM2008 height (EPSG:3855)"}


def gdal():
    from osgeo import gdal as g, ogr, osr
    g.UseExceptions()
    return g, ogr, osr


def write_point(path, driver, sr):
    _, ogr, _ = gdal()
    ds = ogr.GetDriverByName(driver).CreateDataSource(str(path))
    layer = ds.CreateLayer("nokta", sr, ogr.wkbPoint25D)
    f = ogr.Feature(layer.GetLayerDefn())
    f.SetGeometry(ogr.CreateGeometryFromWkt("POINT Z (487200 4420600 1072.4)"))
    layer.CreateFeature(f)
    ds = None


def files():
    _, _, osr = gdal()
    DIR.mkdir(parents=True, exist_ok=True)
    for name, code in CODES.items():
        sr = osr.SpatialReference()
        sr.SetFromUserInput(f"EPSG:5256+{code}")
        # A Shapefile: GDAL's ESRI driver drops the heights' half, so the .prj is written over with each flavour.
        for flavour, wkt in (("ogc", sr.ExportToWkt()), ("esri", sr.ExportToWkt(["FORMAT=WKT1_ESRI"]))):
            shp = DIR / f"{name}-{flavour}.shp"
            for ext in ("shp", "shx", "dbf", "prj", "cpg"):
                shp.with_suffix(f".{ext}").unlink(missing_ok=True)
            write_point(shp, "ESRI Shapefile", sr)
            shp.with_suffix(".prj").write_text(wkt)
        geojson = DIR / f"{name}.geojson"
        geojson.unlink(missing_ok=True)
        write_point(geojson, "GeoJSON", sr)
        gpkg = DIR / f"{name}.gpkg"
        gpkg.unlink(missing_ok=True)
        write_point(gpkg, "GPKG", sr)
    plain = osr.SpatialReference()
    plain.ImportFromEPSG(5256)
    for f in ("duz.gpkg", "duz.geojson"):
        (DIR / f).unlink(missing_ok=True)
    write_point(DIR / "duz.gpkg", "GPKG", plain)
    write_point(DIR / "duz.geojson", "GeoJSON", plain)
    print(f"{DIR.relative_to(ROOT)}: {len(list(DIR.iterdir()))} dosya")


def vertical_of(path):
    """The heights' system the file names, read by GDAL: (name, EPSG code or None)."""
    g, _, osr = gdal()
    if path.suffix == ".shp":
        sr = osr.SpatialReference()
        text = path.with_suffix(".prj").read_text()
        if not text.startswith("COMPD_CS"):
            # ESRI's two definitions: GDAL reads them (ImportFromESRI) but renames the vertical system after EPSG's;
            # the import says the name the file writes, and ESRI's VERTCS gives no authority.
            sr.ImportFromESRI([text])
            assert sr.IsCompound(), path
            return text.split('VERTCS["', 1)[1].split('"', 1)[0], None
        sr.ImportFromWkt(text)
    else:
        ds = g.OpenEx(str(path))
        sr = ds.GetLayer(0).GetSpatialRef()
    if sr is None or not sr.IsCompound():
        return None
    name = sr.GetAttrValue("COMPD_CS|VERT_CS")
    code = sr.GetAuthorityCode("COMPD_CS|VERT_CS")
    # A GeoJSON's URN names only the code: KentOS names it by its own vertical systems (the code GDAL reads).
    return name, int(code) if code else None


KNOWN = {5775: "Antalya height", 3855: "EGM2008 height"}


def said(file, vertical, project):
    name, code = vertical
    if file.endswith(".geojson") and code in KNOWN:
        name = KNOWN[code]
    elif file.endswith(".geojson") and code:
        name = f"EPSG:{code}"
    named = f"{name} (EPSG:{code})" if code else name
    if project is not None and project.get("verticalSrid") == code and code is not None:
        message = f"{file} kotların {named} olduğunu söylüyor; projenin kotları da öyle: kotlar olduğu gibi alındı."
    else:
        theirs = "söylenmemiş" if project is None else (
            "elipsoit yüksekliği" if project["kind"] == "ellipsoidal" else TITLES[project["verticalSrid"]])
        message = (f"{file} kotların {named} olduğunu söylüyor; projenin kotları {theirs}: kotlar olduğu gibi alındı, "
                   "çevrilmedi. Çevirmek için Koordinat dönüştür'ü (cad.crs.transform) geoit modeliyle kullanın.")
    return {"code": "vertical_crs", "message": message, "path": "path"}


COMMANDS = {".shp": "file.import.shp", ".geojson": "file.import.geojson", ".gpkg": "file.import.gpkg"}
ANTALYA = {"kind": "orthometric", "verticalSrid": 5775}
EGM = {"kind": "orthometric", "verticalSrid": 3855}
ELL = {"kind": "ellipsoidal"}


def heights_step(h):
    return {"op": "execute", "command": "cad.settings.set", "input": {"heights": h},
            "result": {"status": "completed", "output": "$any", "warnings": []}, "expect": {"revision": "changed"}}


def import_step(file, project):
    path = DIR / file
    v = vertical_of(path)
    warnings = [said(file if path.suffix != ".shp" else file, v, project)] if v else []
    return {"op": "execute", "command": COMMANDS[path.suffix], "input": {"path": f"$fixture:{REL}/{file}"},
            "result": {"status": "completed", "output": "$any", "warnings": warnings},
            "expect": {"revision": "changed", "canUndo": True}}


def case(title, file, project):
    steps = [] if project is None else [heights_step(project)]
    steps.append(import_step(file, project))
    return {"name": title, "steps": steps}


SETUP = {
    "format": "kentos.document", "version": 1, "name": "Düşey sistemli dosyalar",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                 "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
    "origin": {"x": 487200, "y": 4420600},
    "layers": [{"id": "0", "name": "0", "type": "layer", "visible": True, "locked": False, "expanded": True,
                "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}],
    "activeLayer": "0", "entities": [], "styles": {"items": [], "categories": []},
}


def cases():
    out = []
    for name, label in (("antalya", "Antalya"), ("egm2008", "EGM2008"), ("navd88", "NAVD88")):
        out.append(case(f"Shapefile, OGC COMPD_CS (.prj), {label}; projenin kotları söylenmemiş",
                        f"{name}-ogc.shp", None))
        out.append(case(f"Shapefile, OGC COMPD_CS (.prj), {label}; proje Antalya kotunda",
                        f"{name}-ogc.shp", ANTALYA))
        out.append(case(f"Shapefile, ESRI PROJCS ve VERTCS (.prj), {label}; proje Antalya kotunda",
                        f"{name}-esri.shp", ANTALYA))
        out.append(case(f"GeoJSON, OGC bileşik URN, {label}; proje EGM2008 kotunda", f"{name}.geojson", EGM))
        out.append(case(f"GeoPackage, GDAL'ın kendi kimliğiyle COMPD_CS, {label}; proje Antalya kotunda",
                        f"{name}.gpkg", ANTALYA))
        out.append(case(f"GeoPackage, COMPD_CS, {label}; proje elipsoit yüksekliğinde", f"{name}.gpkg", ELL))
    out.append(case("GeoPackage, yalnız TUREF / TM36: düşey sistem söylenmez", "duz.gpkg", ANTALYA))
    out.append(case("GeoJSON, yalnız EPSG:5256: düşey sistem söylenmez", "duz.geojson", None))
    out.append(case("GeoJSON, bileşik URN, projenin kotları söylenmemiş", "antalya.geojson", None))
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "file.import.shp", "commandVersion": 1,
            "title": "İçe aktarmada dosyanın düşey sistemi",
            "note": "ADR 0631 §4. Dosyalar GDAL'ın (fixtures/vertical/v1), düşey sistem GDAL'ın OSR'ıyla okunur; yatay "
                    "yarı projenin sistemi (TUREF / TM36): taşınmaz. Bağımsız başvuru: "
                    "scripts/fixtures/vertical_import_cases.py.",
            "setup": SETUP, "cases": cases()}


def main():
    if "--files" in sys.argv[1:]:
        files()
        return 0
    built = build()
    text = json.dumps(built, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"düşey sistemli içe aktarma durumları tutarlı: {len(built['cases'])} durum")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(built['cases'])} durum")
    return 0


if __name__ == "__main__":
    sys.exit(main())
