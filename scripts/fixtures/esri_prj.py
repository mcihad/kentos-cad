#!/usr/bin/env python3
"""The .prj a Shapefile of each system of KentOS's registry carries (docs/adr/0514 §4): the ESRI flavour of WKT 1,
as GDAL writes it (`MorphToESRI`) and ArcGIS and QGIS read it back to its EPSG code. Written once from GDAL into
fixtures/crs/v1/esri-prj.json; the writer takes it from there (no GDAL at run time).

    python3 scripts/fixtures/esri_prj.py           # writes fixtures/crs/v1/esri-prj.json
    python3 scripts/fixtures/esri_prj.py --check   # checks it is GDAL's
"""

import json
import sys
from pathlib import Path

from osgeo import gdal, osr

osr.UseExceptions()

ROOT = Path(__file__).resolve().parents[2]
REGISTRY = ROOT / "fixtures/crs/v1/registry.json"
OUT = ROOT / "fixtures/crs/v1/esri-prj.json"


def main():
    systems = json.loads(REGISTRY.read_text())["systems"]
    prj, wkt1 = {}, {}
    for s in systems:
        if s["srid"] == 0:
            continue
        srs = osr.SpatialReference()
        srs.ImportFromEPSG(s["srid"])
        # A CSV's .prj as GDAL's CSV driver writes it and reads it back to its EPSG code (docs/adr/0522).
        wkt1[str(s["srid"])] = srs.ExportToWkt()
        srs.MorphToESRI()
        prj[str(s["srid"])] = srs.ExportToWkt()
    doc = {"format": "kentos.esri-prj/1",
           "note": f"ADR 0514, 0522. GDAL {gdal.__version__}: prj MorphToESRI'nin (Shapefile), wkt1 OGC WKT 1'in "
                   "(CSV); KentOS'un kaydındaki sistemler.",
           "prj": prj, "wkt1": wkt1}
    text = json.dumps(doc, ensure_ascii=False, indent=1, sort_keys=True) + "\n"
    if "--check" in sys.argv:
        old = json.loads(OUT.read_text()) if OUT.exists() else {}
        if old.get("prj") != prj or old.get("wkt1") != wkt1:
            print(f"{OUT.relative_to(ROOT)} GDAL'ınkiyle aynı değil; betiği --check'siz çalıştırıp farkı okuyun")
            sys.exit(1)
        print(f"{len(prj)} sistemin .prj'si GDAL'ınkiyle aynı")
    else:
        OUT.write_text(text)
        print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(prj)} sistem)")


if __name__ == "__main__":
    main()
