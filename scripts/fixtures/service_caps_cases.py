#!/usr/bin/env python3
"""Independent reference of the capabilities readers (docs/adr/0208 §5, §6, §10).

Reads the sample documents in fixtures/services/v1/caps/ with OWSLib and writes what they say to
fixtures/services/v1/caps.json, no KentOS code:

- WMS 1.3.0 and 1.1.1: the GetMap address and formats; each named layer's title, whether it is queryable,
  its styles, its systems (inherited from the layers round it) and its WGS 84 box (west, south, east, north);
- WMTS 1.0.0: the layers' identifiers, titles, formats, styles, matrix sets and tile templates; each matrix
  set's system and matrices, a matrix's pixel by the standard's rule (its scale denominator × 0.28 mm, over
  the metres a unit is: 2π·6 378 137 / 360 a degree) and its top left corner east and north (the system's
  axis order as PROJ gives it: EPSG:4326 and TUREF's TM zones north first);
- WFS 2.0.0: the GetFeature address and output formats, each type's title and systems.

The services core (crates/shared/services/tests/all/caps.rs) must read the same: names, titles, formats and
systems exactly, pixels within 1e-12 of themselves, corners and boxes within 1e-9.

    python3 scripts/fixtures/service_caps_cases.py [--check]
"""

import argparse
import json
import math
import sys
from pathlib import Path

from owslib.wfs import WebFeatureService
from owslib.wms import WebMapService
from owslib.wmts import WebMapTileService
from pyproj import CRS

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/services/v1/caps"
OUT = ROOT / "fixtures/services/v1/caps.json"


def srid_of(code):
    """An EPSG code from the names services write (EPSG:n, urn:ogc:def:crs:EPSG:[v]:n)."""
    text = str(code).strip()
    if text.upper().startswith("EPSG:"):
        return int(text.split(":")[1])
    if text.lower().startswith("urn:ogc:def:crs:epsg:"):
        return int(text.rsplit(":", 1)[1])
    raise ValueError(text)


def north_first(srid):
    return CRS.from_epsg(srid).axis_info[0].direction in ("north", "south")


def degrees(srid):
    return CRS.from_epsg(srid).is_geographic


def wms(name, version):
    raw = (DIR / name).read_bytes()
    s = WebMapService("https://ornek.invalid/wms", version=version, xml=raw)
    get_map = s.getOperationByName("GetMap")
    layers = []
    for lname in s.contents:
        layer = s.contents[lname]
        crs = sorted({srid_of(c) for c in (layer.crsOptions or [])})
        bbox = layer.boundingBoxWGS84
        layers.append({
            "name": lname,
            "title": layer.title,
            "queryable": bool(int(layer.queryable or 0)),
            "styles": sorted(layer.styles.keys()),
            "srids": crs,
            "wgs84": None if bbox is None else [float(v) for v in bbox[:4]],
        })
    return {
        "file": name,
        "version": version,
        "title": s.identification.title,
        "getMap": get_map.methods[0]["url"],
        "formats": list(get_map.formatOptions),
        "layers": layers,
    }


def wmts():
    raw = (DIR / "wmts.xml").read_bytes()
    s = WebMapTileService("https://ornek.invalid/wmts", xml=raw)
    per_degree = 2 * math.pi * 6378137 / 360
    sets = []
    for sid, ms in s.tilematrixsets.items():
        srid = srid_of(ms.crs)
        per = per_degree if degrees(srid) else 1.0
        matrices = []
        for mid, m in ms.tilematrix.items():
            a, b = (float(v) for v in m.topleftcorner)
            east, north = (b, a) if north_first(srid) else (a, b)
            matrices.append({
                "id": mid,
                "resolution": float(m.scaledenominator) * 0.00028 / per,
                "x0": east,
                "y0": north,
                "tileWidth": int(m.tilewidth),
                "tileHeight": int(m.tileheight),
                "matrixWidth": int(m.matrixwidth),
                "matrixHeight": int(m.matrixheight),
            })
        sets.append({"id": sid, "srid": srid, "matrices": matrices})
    layers = []
    for lid, layer in s.contents.items():
        layers.append({
            "id": lid,
            "title": layer.title,
            "formats": list(layer.formats),
            "styles": sorted(layer.styles.keys()),
            "sets": sorted(layer.tilematrixsetlinks.keys()),
            "templates": [[r["format"], r["template"]] for r in layer.resourceURLs if r.get("resourceType") == "tile"],
            "wgs84": None if getattr(layer, "boundingBoxWGS84", None) is None else [float(v) for v in layer.boundingBoxWGS84[:4]],
        })
    return {"file": "wmts.xml", "title": s.identification.title, "layers": layers, "sets": sets}


def wfs():
    raw = (DIR / "wfs-200.xml").read_bytes()
    s = WebFeatureService("https://ornek.invalid/wfs", version="2.0.0", xml=raw)
    op = s.getOperationByName("GetFeature")
    get = next(m["url"] for m in op.methods if m["type"].lower() == "get")
    formats = op.parameters.get("outputFormat", {}).get("values", [])
    types = []
    for name, t in s.contents.items():
        types.append({"name": name, "title": t.title, "srids": [c.code for c in t.crsOptions]})
    return {"file": "wfs-200.xml", "title": s.identification.title, "getFeature": get, "formats": list(formats), "types": types}


def build():
    import owslib

    return {
        "format": "kentos.service-caps",
        "version": 1,
        "source": f"OWSLib {owslib.__version__}, PROJ through pyproj",
        "wms": [wms("wms-130.xml", "1.3.0"), wms("wms-111.xml", "1.1.1")],
        "wmts": wmts(),
        "wfs": wfs(),
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true", help="compare with the file instead of writing it")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        old = OUT.read_text(encoding="utf-8") if OUT.exists() else ""
        if json.loads(old or "null") != json.loads(text):
            print(f"{OUT.relative_to(ROOT)} is not what this script writes: run it without --check and read the difference.")
            return 1
        print(f"{OUT.relative_to(ROOT)} matches")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} written")
    return 0


if __name__ == "__main__":
    sys.exit(main())
