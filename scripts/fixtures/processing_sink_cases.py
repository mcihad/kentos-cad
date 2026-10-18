#!/usr/bin/env python3
"""cad.processing.run's GeoPackage results (docs/adr/0508 §4), read with GDAL's OGR, without KentOS code.

The application's sink test (crates/native/application/tests/all/processing_sink.rs, with KENTOS_WRITE_GPKG=1)
writes G1's three results to fixtures/geopackage/v1/kentos-run-*.gpkg. They are read here and checked against the
source drawing (fixtures/processing/v1/geometry.kcad) and İşlemler's own independent expectations
(fixtures/processing/v1/geometry.json):

- Kesişim: one row per parcel × zoning piece, each piece's area OGR's intersection of the two sources, its
  attributes those İşlemler's case says (provenance of both sides, apportioned Değer);
- Tampon: one row per road, each a valid surface whose area is OGR's 5 m buffer of the road (1024 segments per
  quarter circle) within 0.05 %, the road's attributes carried;
- Birleştirilen: one row per Ada, its area OGR's union of the group's parcels, sums as İşlemler's case says;
- every table in the project's system (EPSG:5254), every row valid, kentos_uids naming each row;
- its lineage (docs/adr/0511) in GeoPackage's metadata extension, read with sqlite3: the extension registered, one
  dataset document of https://kentos.app/lineage/1 referenced to the table at the time it was made, naming the tool,
  the request as the test gave it, and the layers read with their content's digest (the same parcels' digest in two
  files); GDAL's validate_gpkg.py finds nothing wrong.

  /usr/bin/python3 scripts/fixtures/processing_sink_cases.py --check
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

from osgeo import ogr

ogr.UseExceptions()

ROOT = Path(__file__).resolve().parents[2]
GPKG = ROOT / "fixtures/geopackage/v1"
SOURCE = json.loads((ROOT / "fixtures/processing/v1/geometry.kcad").read_text(encoding="utf-8"))
CASES = {c["id"]: c for c in json.loads((ROOT / "fixtures/processing/v1/geometry.json").read_text(encoding="utf-8"))["cases"]}

problems: list[str] = []


def expect(ok: bool, what: str) -> None:
    if not ok:
        problems.append(what)


def polygon(e: dict) -> ogr.Geometry:
    assert not e.get("bulges") and not e.get("holes"), "straight rings only"
    ring = ogr.Geometry(ogr.wkbLinearRing)
    for p in e["pts"] + [e["pts"][0]]:
        ring.AddPoint_2D(p["x"], p["y"])
    g = ogr.Geometry(ogr.wkbPolygon)
    g.AddGeometry(ring)
    return g


def line(e: dict) -> ogr.Geometry:
    g = ogr.Geometry(ogr.wkbLineString)
    pts = [e["a"], e["b"]] if e["kind"] == "line" else e["pts"]
    assert not e.get("bulges"), "straight paths only"
    for p in pts:
        g.AddPoint_2D(p["x"], p["y"])
    return g


def layer(e_layer: str) -> list[dict]:
    return [e for e in SOURCE["entities"] if e["layerId"] == e_layer]


def rows(name: str, table: str) -> list[tuple[dict, ogr.Geometry]]:
    ds = ogr.Open(str(GPKG / f"kentos-run-{name}.gpkg"))
    names = [ds.GetLayer(i).GetName() for i in range(ds.GetLayerCount())]
    expect(names == [table, "kentos_uids"], f"{name}: tables {names}")
    lyr = ds.GetLayerByName(table)
    srs = lyr.GetSpatialRef()
    expect(srs is not None and srs.GetAuthorityCode(None) == "5254", f"{name}: system {srs and srs.GetAuthorityCode(None)}")
    out = []
    for f in lyr:
        g = f.GetGeometryRef().GetLinearGeometry(0.0005)
        expect(g.IsValid(), f"{name}: row {f.GetFID()} is not valid")
        out.append(({k: f.GetField(k) for k in f.keys()}, g.Clone()))
    uids = ds.GetLayerByName("kentos_uids")
    expect(uids.GetFeatureCount() == len(out), f"{name}: {uids.GetFeatureCount()} ids for {len(out)} rows")
    return out


def close(a: float, b: float, rel: float, absolute: float = 1e-6) -> bool:
    return abs(a - b) <= max(absolute, rel * abs(b))


# Kesişim
got = rows("kesisim", "Kesişim")
want = CASES["intersection"]["expect"]["addedShapes"]
expect(len(got) == len(want), f"kesisim: {len(got)} rows, {len(want)} wanted")
parcels, zones = layer("parsel"), layer("imar")
for (attrs, g), w in zip(got, want):
    expect(attrs == w["attrs"], f"kesisim: attributes {attrs} != {w['attrs']}")
    p = next(e for e in parcels if e["attrs"]["Parsel"] == attrs["Parsel"])
    z = next(e for e in zones if e["attrs"]["Ada"] == attrs["Ada (2)"])
    ref = polygon(p).Intersection(polygon(z)).GetArea()
    expect(close(g.GetArea(), ref, 1e-9), f"kesisim {attrs['Parsel']}: area {g.GetArea()} != {ref}")
pieces = {(e["attrs"]["Parsel"], z["attrs"]["Ada"]) for e in parcels for z in zones if polygon(e).Intersection(polygon(z)).GetArea() > 0}
expect({(a["Parsel"], a["Ada (2)"]) for a, _ in got} == pieces, "kesisim: the pieces are not the sources' intersections")

# Tampon
got = rows("tampon", "Tampon")
roads = layer("yol")
expect(len(got) == len(roads), f"tampon: {len(got)} rows for {len(roads)} roads")
for attrs, g in got:
    road = next(e for e in roads if e["attrs"]["Ad"] == attrs.get("Ad"))
    ref = line(road).Buffer(5, 1024).GetArea()
    expect(close(g.GetArea(), ref, 5e-4), f"tampon {attrs.get('Ad')}: area {g.GetArea()} != {ref}")

# Birleştirilen
got = rows("birlestirilen", "Birleştirilen")
want = CASES["dissolve-ada"]["expect"]["addedShapes"]
expect(len(got) == len(want), f"birlestirilen: {len(got)} rows, {len(want)} wanted")
for (attrs, g), w in zip(got, want):
    expect(attrs == w["attrs"], f"birlestirilen: attributes {attrs} != {w['attrs']}")
    union = None
    for e in parcels:
        if e["attrs"]["Ada"] == attrs["Ada"]:
            union = polygon(e) if union is None else union.Union(polygon(e))
    expect(close(g.GetArea(), union.GetArea(), 1e-9), f"birlestirilen {attrs['Ada']}: area {g.GetArea()} != {union.GetArea()}")

# Lineage (docs/adr/0511).
import re
import sqlite3

REQUESTS = {
    "kesisim": ("Kesişim", "geometry.intersection", {
        "input": {"scope": "layer", "layerId": "parsel"}, "overlay": {"scope": "layer", "layerId": "imar"},
        "prefix": "", "apportion": "Değer"}, ["parsel", "imar"]),
    "tampon": ("Tampon", "geometry.buffer", {"input": {"scope": "layer", "layerId": "yol"}, "distance": 5}, ["yol"]),
    "birlestirilen": ("Birleştirilen", "geometry.dissolve", {
        "input": {"scope": "layer", "layerId": "parsel"}, "group": "Ada", "sums": "Değer"}, ["parsel"]),
}
digests: dict[str, set[str]] = {}
for name, (table, tool, parameters, layers) in REQUESTS.items():
    path = GPKG / f"kentos-run-{name}.gpkg"
    c = sqlite3.connect(path)
    ext = c.execute("SELECT table_name, column_name, extension_name, definition, scope FROM gpkg_extensions "
                    "WHERE extension_name = 'gpkg_metadata' ORDER BY table_name").fetchall()
    want_def = "http://www.geopackage.org/spec120/#extension_metadata"
    expect(ext == [("gpkg_metadata", None, "gpkg_metadata", want_def, "read-write"),
                   ("gpkg_metadata_reference", None, "gpkg_metadata", want_def, "read-write")], f"{name}: extension {ext}")
    rows = c.execute("SELECT m.md_scope, m.md_standard_uri, m.mime_type, m.metadata, r.reference_scope, r.table_name, "
                     "r.column_name, r.row_id_value, r.timestamp FROM gpkg_metadata m JOIN gpkg_metadata_reference r "
                     "ON r.md_file_id = m.id").fetchall()
    expect(len(rows) == 1, f"{name}: {len(rows)} metadata rows")
    scope, uri, mime, text, ref, ref_table, col, row_id, stamp = rows[0]
    expect((scope, uri, mime, ref, ref_table, col, row_id) ==
           ("dataset", "https://kentos.app/lineage/1", "application/json", "table", table, None, None),
           f"{name}: metadata row {rows[0][:3]} {rows[0][4:8]}")
    expect(re.fullmatch(r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{3}Z", stamp) is not None, f"{name}: timestamp {stamp}")
    doc = json.loads(text)
    expect(doc["format"] == "kentos.lineage" and doc["version"] == 1, f"{name}: format")
    expect(doc["table"] == table and doc["made"] == stamp, f"{name}: table and time")
    expect(doc["tool"]["id"] == tool and doc["request"] == {"tool": tool, "parameters": parameters},
           f"{name}: request {doc['request']}")
    expect([i["layer"] for i in doc["inputs"]] == layers, f"{name}: inputs {doc['inputs']}")
    for i in doc["inputs"]:
        expect(re.fullmatch(r"[0-9a-f]{64}", i["digest"]) is not None, f"{name}: digest {i['digest']}")
        digests.setdefault(i["layer"], set()).add(i["digest"])
    expect(doc["engine"].startswith("KentOS "), f"{name}: engine {doc['engine']}")
    try:
        from osgeo_utils.samples import validate_gpkg
        found = validate_gpkg.check(str(path), abort_at_first_error=False) or []
        expect(not found, f"{name}: validate_gpkg {found}")
    except ImportError:
        pass
expect(len(digests["parsel"]) == 1, f"the parcels read unchanged give one digest: {digests['parsel']}")

if problems:
    print("\n".join(problems))
    sys.exit(1)
print("İşlem sonuçlarının GeoPackage'ları tutarlı: 3 dosya")
