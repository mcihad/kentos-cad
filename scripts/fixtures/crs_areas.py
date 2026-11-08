"""EPSG's area of use of the CRS registry's systems (docs/adr/2500 §3), read from PROJ's database. No KentOS code.

Writes fixtures/crs/v1/areas.json: for every system of fixtures/crs/v1/registry.json but the local one, EPSG's area of
use as PROJ's database gives it (`pyproj.CRS.area_of_use`): its text and bounding box, degrees west, south, east, north.
IOGP's Guidance Note 7-1 (§5.7) says these boxes are approximate, meant for selecting records, in WGS 84 terms; the
command reads them as such (a point outside is a note to look at, never a refusal). `--check` compares the file.

Run with QGIS's Python and its proj.db (PROJ_DATA), as the other CRS generators are (docs/adr/1230 §Doğrulama).
"""

import json
import sys
from pathlib import Path

from pyproj import CRS, database

ROOT = Path(__file__).resolve().parents[2]
REGISTRY = ROOT / "fixtures/crs/v1/registry.json"
OUT = ROOT / "fixtures/crs/v1/areas.json"


def areas():
    out = []
    for s in json.loads(REGISTRY.read_text("utf-8"))["systems"]:
        if s["kind"] == "local":
            continue
        a = CRS.from_epsg(s["srid"]).area_of_use
        out.append({"srid": s["srid"], "name": a.name, "west": a.west, "south": a.south, "east": a.east,
                    "north": a.north})
    return out


def main():
    data = {"format": "kentos.crs-areas", "version": 1,
            "source": f"EPSG {database.get_database_metadata('EPSG.VERSION')} "
                      f"({database.get_database_metadata('EPSG.DATE')}), PROJ'un veritabanından "
                      "(scripts/fixtures/crs_areas.py); kutular yaklaşıktır, kayıt seçmek içindir (IOGP GN7-1 §5.7)",
            "areas": areas()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text("utf-8") != text:
            print("areas.json güncel değil", file=sys.stderr)
            sys.exit(1)
        print(f"areas.json: {len(data['areas'])} sistem")
        return
    OUT.write_text(text, "utf-8")
    print(f"areas.json: {len(data['areas'])} sistem yazıldı")


if __name__ == "__main__":
    main()
