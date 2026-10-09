#!/usr/bin/env python3
"""Independent reference of the vector tile reader (docs/adr/0208 §9).

A tile written by GDAL's MVT driver (points, multi-points, lines, multi-lines, polygons with holes and
multi-polygons, text, whole, decimal and yes/no values, ids) and what GDAL's own MVT reader reads in it,
in tile coordinates (0..extent, Y down) with each ring closed as OGR closes it and the feature's id (GDAL's
`mvt_id` field) apart from its values, no KentOS code:

    fixtures/services/v1/mvt/kizilay.pbf    the tile (z 14, uncompressed)
    fixtures/services/v1/mvt.json           its layers and features as GDAL reads them

`--write` makes the tile again from the features below (GDAL's output may differ between versions: the tile
is kept, not remade); `--check` reads the kept tile with GDAL and compares with mvt.json. The services core
(crates/shared/services/src/mvt.rs's tests) must read the same layers, ids, kinds, values and vertices.

    python3 scripts/fixtures/service_mvt_cases.py [--write | --check]
"""

import argparse
import json
import math
import shutil
import sys
import tempfile
from pathlib import Path

from osgeo import gdal, ogr, osr

gdal.UseExceptions()

ROOT = Path(__file__).resolve().parents[2]
TILE = ROOT / "fixtures/services/v1/mvt/kizilay.pbf"
OUT = ROOT / "fixtures/services/v1/mvt.json"
Z, X, Y = 14, 9687, 6207
EXTENT = 4096
HALF = 20037508.342789244


def tile_box():
    span = 2 * HALF / 2**Z
    x0 = -HALF + X * span
    y0 = HALF - Y * span
    return x0, y0, span


def lonlat(px, py):
    """A point of the tile (0..1 across and down) in degrees."""
    x0, y0, span = tile_box()
    mx = x0 + px * span
    my = y0 - py * span
    lon = mx / HALF * 180
    lat = math.degrees(2 * math.atan(math.exp(my / HALF * math.pi)) - math.pi / 2)
    return [lon, lat]


def ring(points):
    r = [lonlat(*p) for p in points]
    return r + [r[0]]


FEATURES = [
    ("duraklar", 1, {"ad": "Kızılay", "hat": 3, "yuk": 1.5, "acik": True}, {"type": "Point", "coordinates": lonlat(0.31, 0.42)}),
    ("duraklar", 2, {"ad": "Güvenpark", "hat": 12, "yuk": 0.25, "acik": False}, {"type": "MultiPoint", "coordinates": [lonlat(0.5, 0.5), lonlat(0.52, 0.61)]}),
    ("yollar", 3, {"ad": "Atatürk Bulvarı", "serit": 6}, {"type": "LineString", "coordinates": [lonlat(0.1, 0.9), lonlat(0.35, 0.6), lonlat(0.4, 0.2)]}),
    ("yollar", 4, {"ad": "Ziya Gökalp"}, {"type": "MultiLineString", "coordinates": [[lonlat(0.05, 0.3), lonlat(0.45, 0.32)], [lonlat(0.55, 0.33), lonlat(0.95, 0.36)]]}),
    ("yapilar", 5, {"ad": "Avlulu yapı", "kat": 4}, {"type": "Polygon", "coordinates": [
        ring([(0.6, 0.6), (0.8, 0.6), (0.8, 0.8), (0.6, 0.8)]), ring([(0.65, 0.65), (0.65, 0.75), (0.75, 0.75), (0.75, 0.65)])]}),
    ("yapilar", 6, {"ad": "İkiz bloklar", "kat": 9}, {"type": "MultiPolygon", "coordinates": [
        [ring([(0.1, 0.1), (0.2, 0.1), (0.2, 0.2), (0.1, 0.2)])], [ring([(0.25, 0.1), (0.3, 0.1), (0.3, 0.18), (0.25, 0.18)])]]}),
]


def write():
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "karolar"
        srs = osr.SpatialReference()
        srs.ImportFromEPSG(4326)
        srs.SetAxisMappingStrategy(osr.OAMS_TRADITIONAL_GIS_ORDER)
        ds = ogr.GetDriverByName("MVT").CreateDataSource(
            str(out), options=["MINZOOM=14", "MAXZOOM=14", f"EXTENT={EXTENT}", "COMPRESS=NO", "FORMAT=DIRECTORY", "MAX_SIZE=500000"]
        )
        layers = {}
        for name, fid, props, geom in FEATURES:
            layers.setdefault(name, []).append((fid, props, geom))
        for name, feats in layers.items():
            lyr = ds.CreateLayer(name, srs=srs, geom_type=ogr.wkbUnknown, options=["MINZOOM=14", "MAXZOOM=14"])
            kinds = {}
            for _, props, _ in feats:
                for k, v in props.items():
                    kinds.setdefault(k, v)
            for k, v in kinds.items():
                d = ogr.FieldDefn(k, ogr.OFTString if isinstance(v, str) else ogr.OFTReal if isinstance(v, float) else ogr.OFTInteger)
                if isinstance(v, bool):
                    d.SetSubType(ogr.OFSTBoolean)
                lyr.CreateField(d)
            for fid, props, geom in feats:
                f = ogr.Feature(lyr.GetLayerDefn())
                f.SetFID(fid)
                for k, v in props.items():
                    f.SetField(k, int(v) if isinstance(v, bool) else v)
                f.SetGeometry(ogr.CreateGeometryFromJson(json.dumps(geom)))
                lyr.CreateFeature(f)
        ds = None
        made = out / str(Z) / str(X) / f"{Y}.pbf"
        if not made.exists():
            raise SystemExit(f"GDAL did not write tile {Z}/{X}/{Y}: {sorted(str(q.relative_to(out)) for q in out.rglob('*.pbf'))}")
        TILE.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(made, TILE)


def value(v):
    if isinstance(v, bool):
        return v
    if isinstance(v, (int, float)):
        return float(v)
    return v


def read():
    """The kept tile as GDAL's MVT reader reads it, in tile coordinates."""
    x0, y0, span = tile_box()

    def tc(x, y):
        return [round((x - x0) / span * EXTENT, 6), round((y0 - y) / span * EXTENT, 6)]

    def ring_of(g):
        return [tc(g.GetX(i), g.GetY(i)) for i in range(g.GetPointCount())]

    path = f"/vsimem/karo/{Z}/{X}/{Y}.pbf"
    gdal.FileFromMemBuffer(path, TILE.read_bytes())
    try:
        ds = gdal.OpenEx(path, gdal.OF_VECTOR, open_options=["CLIP=NO", f"X={X}", f"Y={Y}", f"Z={Z}"])
        layers = []
        for li in range(ds.GetLayerCount()):
            lyr = ds.GetLayer(li)
            feats = []
            for f in lyr:
                g = f.GetGeometryRef()
                t = ogr.GT_Flatten(g.GetGeometryType())
                if t == ogr.wkbPoint:
                    kind, geom = "point", [tc(g.GetX(), g.GetY())]
                elif t == ogr.wkbMultiPoint:
                    kind, geom = "point", [tc(g.GetGeometryRef(i).GetX(), g.GetGeometryRef(i).GetY()) for i in range(g.GetGeometryCount())]
                elif t == ogr.wkbLineString:
                    kind, geom = "line", [ring_of(g)]
                elif t == ogr.wkbMultiLineString:
                    kind, geom = "line", [ring_of(g.GetGeometryRef(i)) for i in range(g.GetGeometryCount())]
                elif t == ogr.wkbPolygon:
                    kind, geom = "polygon", [[ring_of(g.GetGeometryRef(i)) for i in range(g.GetGeometryCount())]]
                elif t == ogr.wkbMultiPolygon:
                    kind, geom = "polygon", [
                        [ring_of(p.GetGeometryRef(i)) for i in range(p.GetGeometryCount())]
                        for p in (g.GetGeometryRef(k) for k in range(g.GetGeometryCount()))
                    ]
                else:
                    raise SystemExit(f"an unexpected geometry: {g.GetGeometryName()}")
                props = {}
                for i in range(f.GetFieldCount()):
                    if f.IsFieldSetAndNotNull(i):
                        d = f.GetFieldDefnRef(i)
                        v = f.GetField(i)
                        if d.GetSubType() == ogr.OFSTBoolean:
                            v = bool(v)
                        props[d.GetName()] = value(v)
                # GDAL's reader gives a feature's own id as the mvt_id field.
                fid = props.pop("mvt_id", None)
                feats.append({"id": None if fid is None else int(fid), "kind": kind, "properties": props, "geometry": geom})
            layers.append({"name": lyr.GetName(), "features": feats})
        return {
            "format": "kentos.service-mvt",
            "version": 1,
            "source": f"GDAL {gdal.__version__}: the MVT driver's reader",
            "tile": [Z, X, Y],
            "extent": EXTENT,
            "layers": sorted(layers, key=lambda layer: layer["name"]),
        }
    finally:
        gdal.Unlink(path)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--write", action="store_true", help="make the tile again with GDAL's writer")
    ap.add_argument("--check", action="store_true", help="compare with the file instead of writing it")
    args = ap.parse_args()
    if args.write or not TILE.exists():
        write()
    data = read()
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
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
