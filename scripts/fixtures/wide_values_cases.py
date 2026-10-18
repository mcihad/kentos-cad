#!/usr/bin/env python3
"""Values whose meaning a bound layer's round trip keeps (plan/gis.md CBS-06, docs/adr/0512), without KentOS code.

`fixtures/geopackage/v1/genis.gpkg` is written here with GDAL's OGR (`--files`): one table, three points, an integer past
2⁵³ and the smallest 64-bit one, 0 and false, a double with no short decimal, an empty text beside NULL, a time with its
zone (+03:00) and one in UTC, a text with quotes. The application's test
(crates/native/application/tests/all/sources.rs, wide_values_…, with KENTOS_WRITE_GPKG=1) links it, changes the third
row's text and writes it back as `kentos-genis.gpkg`. Here sqlite3 compares the two files cell by cell, each cell's
value and its SQLite type: everything is as it was but that one text.

  /usr/bin/python3 scripts/fixtures/wide_values_cases.py --files   # writes genis.gpkg again
  /usr/bin/python3 scripts/fixtures/wide_values_cases.py --check
"""

import sqlite3
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/geopackage/v1"
COLUMNS = ["buyuk", "sayi", "evet", "ondalik", "metin", "an"]
ROWS = [
    (9007199254740993, 0, 0, 0.1, "", "2026-03-29T00:30:00+03:00"),
    (-9223372036854775807, 7, 1, 123456789.123456789, None, "2026-10-25T03:30:00Z"),
    (None, None, None, None, "ş\"'", None),
]


def write_source():
    from osgeo import ogr, osr
    ogr.UseExceptions()
    path = DIR / "genis.gpkg"
    path.unlink(missing_ok=True)
    ds = ogr.GetDriverByName("GPKG").CreateDataSource(str(path))
    srs = osr.SpatialReference()
    srs.ImportFromEPSG(5256)
    layer = ds.CreateLayer("deger", srs, ogr.wkbPoint)
    layer.CreateField(ogr.FieldDefn("buyuk", ogr.OFTInteger64))
    layer.CreateField(ogr.FieldDefn("sayi", ogr.OFTInteger))
    yes = ogr.FieldDefn("evet", ogr.OFTInteger)
    yes.SetSubType(ogr.OFSTBoolean)
    layer.CreateField(yes)
    layer.CreateField(ogr.FieldDefn("ondalik", ogr.OFTReal))
    layer.CreateField(ogr.FieldDefn("metin", ogr.OFTString))
    layer.CreateField(ogr.FieldDefn("an", ogr.OFTDateTime))
    for i, row in enumerate(ROWS):
        f = ogr.Feature(layer.GetLayerDefn())
        for name, v in zip(COLUMNS, row):
            if v is None:
                f.SetFieldNull(name)
            else:
                f.SetField(name, v)
        g = ogr.Geometry(ogr.wkbPoint)
        g.AddPoint_2D(487000 + i, 4420000)
        f.SetGeometry(g)
        layer.CreateFeature(f)
    ds = None


def cells(name):
    c = sqlite3.connect(DIR / name)
    cols = ", ".join(f"{k}, typeof({k})" for k in COLUMNS)
    return c.execute(f"SELECT fid, {cols} FROM deger ORDER BY fid").fetchall()


def main():
    if "--files" in sys.argv:
        write_source()
    source, written = cells("genis.gpkg"), cells("kentos-genis.gpkg")
    problems = []
    # What OGR wrote is what the cases say (the source is ours, not KentOS's).
    for got, want in zip(source, ROWS):
        values = got[1::2]
        for k, g, w in zip(COLUMNS, values, want):
            if isinstance(w, float) and g is not None:
                ok = g == w
            else:
                ok = g == w
            if not ok:
                problems.append(f"genis.gpkg {got[0]}.{k}: {g!r}, beklenen {w!r}")
    expected = [list(r) for r in source]
    i = 1 + 2 * COLUMNS.index("metin")
    expected[2][i] = "değişti"
    for got, want in zip(written, expected):
        if list(got) != want:
            problems.append(f"kentos-genis.gpkg {got[0]}: {got}, beklenen {tuple(want)}")
    if len(written) != len(source):
        problems.append(f"{len(written)} satır, beklenen {len(source)}")
    if problems:
        print("\n".join(problems))
        return 1
    print("Geniş değerler gidiş-dönüşte aynı: 3 satır, her hücre değeri ve türüyle")
    return 0


if __name__ == "__main__":
    sys.exit(main())
