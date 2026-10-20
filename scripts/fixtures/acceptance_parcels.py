#!/usr/bin/env python3
"""The first end-to-end GIS acceptance project (plan/gis.md CBS-44, docs/adr/0554): a parcel and building project in
TUREF / TM36 made, edited, queried and checked by the catalog's commands, written to GeoPackage, FlatGeobuf, GML,
Shapefile and CSV, and opened elsewhere with nothing lost. Every value it must give is worked out here from the
definitions, without KentOS code: the shoelace areas, the typed fields' canonical text, which building touches which
parcel, what each format can carry.

The scenario (fixtures/acceptance/v1/parcels.json) is replayed by Python (python/tests/test_acceptance_gis.py, the
headless session the MCP server also runs); with ``KENTOS_WRITE_ACCEPTANCE=1`` the files it writes go to
fixtures/acceptance/v1/out, and ``--files`` reads them back with GDAL's OGR (QGIS 4.2's Python on macOS: the script
opens itself with it) against the same numbers.

Steps:

- ``command``: a catalog command and its input; ``"$name"`` is a value an earlier step bound; ``bind`` binds output
  fields (``{"parsel": "layer"}``), ``as`` the output's ``uid``; ``"$out:name"`` is a file in the output folder.
- ``undo``: the drawing's undo; the step's name it must say.
- ``reopen``: the drawing written to ``.kcad`` bytes and read back.
- ``fresh``: the step runs on a new, empty project of the same kind (opening the files elsewhere).
- ``expect``: ``objects`` (named objects' attributes and kind), ``measures`` (area from the source geometry),
  ``count``, ``selected`` (the names the output's ``selected`` holds), ``layers`` (a layer's rows by an attribute: the
  fresh project's).

    python3 scripts/fixtures/acceptance_parcels.py            # write the scenario
    python3 scripts/fixtures/acceptance_parcels.py --check    # compare
    python3 scripts/fixtures/acceptance_parcels.py --files    # read fixtures/acceptance/v1/out with OGR
"""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/acceptance/v1/parcels.json"
FILES = ROOT / "fixtures/acceptance/v1/out"
SRID = 5256  # TUREF / TM36

X0, Y0 = 500000.0, 4400000.0


def rect(x: float, y: float, w: float, h: float) -> list[tuple[float, float]]:
    """A rectangle's corners counter-clockwise from its south-west one."""
    return [(x, y), (x + w, y), (x + w, y + h), (x, y + h)]


def area(pts: list[tuple[float, float]]) -> float:
    """The shoelace area."""
    s = 0.0
    for i, (x0, y0) in enumerate(pts):
        x1, y1 = pts[(i + 1) % len(pts)]
        s += x0 * y1 - x1 * y0
    return abs(s) / 2


# Three parcels side by side, sharing their east and west edges; two buildings, one in the first, one in the third.
PARCELS = {
    "p1": rect(X0, Y0, 20.0, 30.0),
    "p2": rect(X0 + 20.0, Y0, 20.0, 30.0),
    "p3": rect(X0 + 40.0, Y0, 25.0, 30.0),
}
BUILDINGS = {
    "b1": rect(X0 + 5.0, Y0 + 5.0, 10.0, 12.0),
    "b2": rect(X0 + 48.0, Y0 + 8.0, 12.0, 14.0),
}

PARCEL_FIELDS = [
    {"name": "ada", "kind": "integer"},
    {"name": "parsel", "kind": "integer"},
    {"name": "malik", "kind": "text"},
    {"name": "tapu_alani", "kind": "decimal"},
    {"name": "hesap_alani", "kind": "decimal"},
    {"name": "tescil", "kind": "date"},
]
BUILDING_FIELDS = [
    {"name": "kat", "kind": "integer"},
    {"name": "kullanim", "kind": "text"},
]

# The attributes as written: the second parcel's owner an empty text, the third parcel's registration absent (NULL).
PARCEL_ATTRS = {
    "p1": {"ada": "101", "parsel": "1", "malik": "Ahmet Yılmaz", "tapu_alani": "600.5", "tescil": "2019-05-14"},
    "p2": {"ada": "101", "parsel": "2", "malik": "", "tapu_alani": "599.75", "tescil": "2020-11-02"},
    "p3": {"ada": "101", "parsel": "3", "malik": "Şükrü Öğüt", "tapu_alani": "750"},
}
BUILDING_ATTRS = {
    "b1": {"kat": "3", "kullanim": "Konut"},
    "b2": {"kat": "1", "kullanim": "Dükkân"},
}


def number_text(v: float) -> str:
    """A number as a decimal field keeps it: the shortest text that reads back as the same double, no exponent for
    these magnitudes, an integer without a point (ADR 0199 §1's one form)."""
    if v == int(v):
        return str(int(v))
    return repr(v)


def computed(name: str) -> dict:
    """A parcel's attributes after Alan hesapla wrote round($alan, 2) into hesap_alani."""
    return dict(PARCEL_ATTRS[name], hesap_alani=number_text(round(area(PARCELS[name]), 2)))


def pt(p: tuple[float, float]) -> dict:
    return {"x": p[0], "y": p[1]}


def touches(a: list[tuple[float, float]], b: list[tuple[float, float]]) -> bool:
    """Whether two axis-parallel rectangles share a point (closed sets)."""
    ax0, ay0 = a[0]
    ax1, ay1 = a[2]
    bx0, by0 = b[0]
    bx1, by1 = b[2]
    return ax0 <= bx1 and bx0 <= ax1 and ay0 <= by1 and by0 <= ay1


def scenario() -> dict:
    steps: list[dict] = []
    steps.append({
        "name": "Parsel katmanı",
        "command": "cad.layers.edit",
        "input": {"operation": "add", "name": "Parsel", "activate": True},
        "bind": {"parsel": "layer"},
    })
    steps.append({
        "name": "Parsel katmanının tipli alanları",
        "command": "cad.layers.edit",
        "input": {"operation": "fields", "layer": "$parsel", "fields": PARCEL_FIELDS},
    })
    steps.append({
        "name": "Bina katmanı",
        "command": "cad.layers.edit",
        "input": {"operation": "add", "name": "Bina"},
        "bind": {"bina": "layer"},
    })
    steps.append({
        "name": "Bina katmanının tipli alanları",
        "command": "cad.layers.edit",
        "input": {"operation": "fields", "layer": "$bina", "fields": BUILDING_FIELDS},
    })
    count = 0
    for name, ring in PARCELS.items():
        count += 1
        steps.append({
            "name": f"Parsel {name[1]}",
            "command": "cad.polygon.create",
            "input": {"layerId": "$parsel", "pts": [pt(p) for p in ring]},
            "as": name,
            "expect": {"measures": {name: {"area": area(ring)}}, "count": count},
        })
        steps.append({
            "name": f"Parsel {name[1]}'in öznitelikleri",
            "command": "cad.entities.set",
            "input": {"uids": [f"${name}"], "operation": "attributes", "attrs": PARCEL_ATTRS[name]},
            "expect": {"objects": {name: {"kind": "polygon", "attrs": PARCEL_ATTRS[name]}}},
        })
    for name, ring in BUILDINGS.items():
        count += 1
        steps.append({
            "name": f"Bina {name[1]}",
            "command": "cad.polygon.create",
            "input": {"layerId": "$bina", "pts": [pt(p) for p in ring]},
            "as": name,
            "expect": {"measures": {name: {"area": area(ring)}}, "count": count},
        })
        steps.append({
            "name": f"Bina {name[1]}'in öznitelikleri",
            "command": "cad.entities.set",
            "input": {"uids": [f"${name}"], "operation": "attributes", "attrs": BUILDING_ATTRS[name]},
            "expect": {"objects": {name: {"attrs": BUILDING_ATTRS[name]}}},
        })
    steps.append({
        "name": "Bir değer yanlış girilir",
        "command": "cad.entities.set",
        "input": {"uids": ["$p1"], "operation": "attributes", "attrs": {"malik": "Yanlış"}},
        "expect": {"objects": {"p1": {"attrs": dict(PARCEL_ATTRS["p1"], malik="Yanlış")}}},
    })
    steps.append({
        "name": "Geri al: değer eski hâlinde",
        "undo": "Değiştir",  # cad.entities.set's step for attributes
        "expect": {"objects": {"p1": {"attrs": PARCEL_ATTRS["p1"]}}},
    })
    steps.append({
        "name": "Alan hesapla: hesap_alani = round($alan, 2)",
        "command": "cad.fields.write",
        "input": {"layer": "$parsel", "write": {"kind": "calculate", "field": "hesap_alani",
                                               "expression": "round($alan, 2)"}},
        "expect": {"objects": {n: {"attrs": computed(n)} for n in PARCELS}},
    })
    # The building that touches the third parcel: by the rectangles' own corners.
    selected = [b for b, ring in BUILDINGS.items() if touches(ring, PARCELS["p3"])]
    steps.append({
        "name": "Konuma göre seç: üçüncü parseldeki bina",
        "command": "cad.processing.run",
        "input": {"tool": "selection.byLocation", "parameters": {
            "input": {"scope": "layer", "layerId": "$bina"},
            "relation": "intersects",
            "reference": {"scope": "uids", "uids": ["$p3"]},
        }},
        "expect": {"selected": selected, "count": count},
    })
    steps.append({
        "name": "Kaydet ve aç: proje, alanlar ve değerler",
        "reopen": True,
        "expect": {
            "objects": {**{n: {"attrs": computed(n)} for n in PARCELS},
                        **{n: {"attrs": BUILDING_ATTRS[n]} for n in BUILDINGS}},
            "measures": {n: {"area": area(r)} for n, r in {**PARCELS, **BUILDINGS}.items()},
            "count": count,
        },
    })
    for command, file in [
        ("file.export.gpkg", "ada.gpkg"),
        ("file.export.fgb", "ada.fgb"),
        ("file.export.gml", "ada.gml"),
        ("file.export.shp", "ada-shp"),
        ("file.export.csv", "ada.csv"),
    ]:
        steps.append({
            "name": f"Dışa aktar: {file}",
            "command": command,
            "input": {"path": f"$out:{file}"},
        })
    steps.append({
        "name": "Başka ortamda aç: GeoPackage yeni projede, alanlar ve değerler eksiksiz",
        "fresh": True,
        "command": "file.import.gpkg",
        "input": {"path": "$out:ada.gpkg"},
        "expect": {
            "count": count,
            "layers": {
                "Parsel": {"key": "parsel", "rows": {computed(n)["parsel"]: computed(n) for n in PARCELS},
                           "areas": {computed(n)["parsel"]: area(PARCELS[n]) for n in PARCELS}},
                "Bina": {"key": "kullanim", "rows": {a["kullanim"]: a for a in BUILDING_ATTRS.values()},
                         "areas": {BUILDING_ATTRS[n]["kullanim"]: area(r) for n, r in BUILDINGS.items()}},
            },
        },
    })
    return {
        "format": "kentos.acceptance",
        "version": 1,
        "id": "parcels",
        "title": "Ada: parseller ve binalar; tipli alanlar, hesap, sorgu; beş biçim; başka ortamda aç (CBS-44)",
        "source": "plan/gis.md CBS-44; docs/adr/0554",
        "project": {"name": "Ada 101", "workspace": "gis", "srid": SRID},
        "tolerance": 1e-9,
        "steps": steps,
        "files": files_expected(),
    }


def files_expected() -> dict:
    """What every written file holds, by the formats' own rules: the layers, their rows by a key, the columns and the
    rings; a format that cannot tell an empty text from NULL (dBASE, CSV) says so."""
    parcels = {computed(n)["parsel"]: {"attrs": computed(n), "ring": PARCELS[n]} for n in PARCELS}
    buildings = {a["kullanim"]: {"attrs": a, "ring": BUILDINGS[n]} for n, a in BUILDING_ATTRS.items()}
    layers = {
        "Parsel": {"key": "parsel", "rows": parcels, "fields": [f["name"] for f in PARCEL_FIELDS]},
        "Bina": {"key": "kullanim", "rows": buildings, "fields": [f["name"] for f in BUILDING_FIELDS]},
    }
    return {
        "srid": SRID,
        "gpkg": {"path": "ada.gpkg", "layers": layers, "nullsKept": True},
        # A FlatGeobuf file is one layer: the layers together with a `katman` column (docs/adr/0544 §2).
        "fgb": {"path": "ada.fgb", "layers": layers, "layerColumn": "katman", "nullsKept": True},
        # OGR reads GML's empty element as NULL; KentOS writes NULL as xsi:nil and reads the two apart (docs/adr/0545).
        "gml": {"path": "ada.gml", "layers": layers, "nullsKept": False},
        # dBASE keeps ten bytes of a column's name (docs/adr/0514): hesap_alani is hesap_alan there.
        "shp": {"path": "ada-shp", "layers": layers, "nullsKept": False, "nameLength": 10},
        "csv": {"path": "ada.csv", "layers": layers, "layerColumn": "katman", "nullsKept": False},
    }


# ── Reading the files back (GDAL's OGR) ──────────────────────────────────────


def ring_of(geom) -> list[tuple[float, float]]:
    ring = geom.GetGeometryRef(0)
    pts = [(ring.GetX(i), ring.GetY(i)) for i in range(ring.GetPointCount())]
    if pts and pts[0] == pts[-1]:
        pts = pts[:-1]
    return pts


def same_ring(a: list[tuple[float, float]], b: list[tuple[float, float]]) -> bool:
    """The same closed ring whatever its first vertex and direction."""
    if len(a) != len(b):
        return False
    for seq in (b, list(reversed(b))):
        for k in range(len(seq)):
            if seq[k:] + seq[:k] == a:
                return True
    return False


def read_files() -> list[str]:
    try:
        from osgeo import ogr, osr
    except ImportError:
        qgis = sorted(Path("/Applications").glob("QGIS*.app/Contents"))
        if not qgis or os.environ.get("KENTOS_QGIS_PYTHON"):
            raise
        res = qgis[-1] / "Resources"
        env = dict(os.environ, KENTOS_QGIS_PYTHON="1", PYTHONHOME=str(res),
                   PYTHONPATH=os.pathsep.join(str(res / p) for p in ("python3.12", "python3.12/site-packages",
                                                                     "python3.12/lib-dynload")))
        exe = qgis[-1] / "MacOS" / "python3.12"
        os.execve(str(exe), [str(exe), *sys.argv], env)
    ogr.UseExceptions()
    want = files_expected()
    problems: list[str] = []
    for fmt in ("gpkg", "fgb", "gml", "shp", "csv"):
        spec = want[fmt]
        path = FILES / spec["path"]
        if not path.exists():
            problems.append(f"{fmt}: {path} yok (önce KENTOS_WRITE_ACCEPTANCE=1 ile Python testini koşun)")
            continue
        ds = ogr.Open(str(path))
        if ds is None:
            problems.append(f"{fmt}: OGR açamadı")
            continue
        # The features of each KentOS layer: a layer per table, or one table with the layer column.
        found: dict[str, list] = {}
        for i in range(ds.GetLayerCount()):
            layer = ds.GetLayer(i)
            if layer.GetGeomType() == ogr.wkbNone:
                continue  # an attribute table (GeoPackage's kentos_uids): no system to have
            srs = layer.GetSpatialRef()
            if srs is None:
                problems.append(f"{fmt}/{layer.GetName()}: sistem yok")
            else:
                srs.AutoIdentifyEPSG()
                epsg = srs.GetAuthorityCode(None)
                target = osr.SpatialReference()
                target.ImportFromEPSG(want["srid"])
                if epsg != str(want["srid"]) and not srs.IsSame(target):
                    problems.append(f"{fmt}/{layer.GetName()}: sistem {epsg}, beklenen EPSG:{want['srid']}")
            for f in layer:
                name = f.GetField(spec["layerColumn"]) if spec.get("layerColumn") else layer.GetName()
                found.setdefault(name, []).append(f.Clone())
        for lname, lspec in spec["layers"].items():
            feats = found.get(lname)
            if feats is None:
                # Shapefile and GML name a table after the layer, perhaps cased or prefixed.
                match = [k for k in found if k.lower().endswith(lname.lower())]
                feats = found.get(match[0]) if match else None
            if feats is None:
                problems.append(f"{fmt}: {lname} katmanı yok (bulunanlar {sorted(found)})")
                continue
            if len(feats) != len(lspec["rows"]):
                problems.append(f"{fmt}/{lname}: {len(feats)} nesne, beklenen {len(lspec['rows'])}")
            rows = {}
            for f in feats:
                key = f.GetField(lspec["key"])
                rows[str(key) if key is not None else None] = f
            for key, row in lspec["rows"].items():
                f = rows.get(key)
                if f is None:
                    problems.append(f"{fmt}/{lname}: {lspec['key']}={key} yok")
                    continue
                for field in lspec["fields"]:
                    idx = f.GetFieldIndex(field[: spec.get("nameLength", len(field))])
                    if idx < 0:
                        problems.append(f"{fmt}/{lname}: {field} sütunu yok")
                        continue
                    have = None if not f.IsFieldSetAndNotNull(idx) else f.GetField(idx)
                    expected = row["attrs"].get(field)
                    if not agrees(have, expected, f.GetFieldDefnRef(idx).GetType(), spec["nullsKept"], ogr):
                        problems.append(f"{fmt}/{lname}/{key}.{field}: {have!r}, beklenen {expected!r}")
                g = f.GetGeometryRef()
                if g is None:
                    problems.append(f"{fmt}/{lname}/{key}: geometri yok")
                    continue
                if g.GetGeometryType() in (ogr.wkbMultiPolygon, ogr.wkbMultiPolygon25D):
                    g = g.GetGeometryRef(0)
                if not same_ring(ring_of(g), row["ring"]):
                    problems.append(f"{fmt}/{lname}/{key}: halka {ring_of(g)}, beklenen {row['ring']}")
                if abs(g.GetArea() - area(row["ring"])) > 1e-9:
                    problems.append(f"{fmt}/{lname}/{key}: alan {g.GetArea()}, beklenen {area(row['ring'])}")
    return problems


def agrees(have, expected: str | None, kind, nulls_kept: bool, ogr) -> bool:
    """A written value read back against the attribute's text: numbers as numbers, a date as its day, an empty text
    and NULL apart where the format keeps them apart."""
    if expected is None or expected == "":
        if not nulls_kept and have in (None, ""):
            return True
        return have is None if expected is None else have == ""
    if kind in (ogr.OFTInteger, ogr.OFTInteger64, ogr.OFTReal):
        try:
            return have is not None and float(have) == float(expected)
        except (TypeError, ValueError):
            return False
    if kind in (ogr.OFTDate, ogr.OFTDateTime):
        return have is not None and str(have).replace("/", "-")[:10] == expected
    return have == expected


def main() -> None:
    if "--files" in sys.argv:
        problems = read_files()
        if problems:
            sys.exit("\n".join(problems))
        print("Kabul projesinin beş dosyası OGR'da beklendiği gibi.")
        return
    text = json.dumps(scenario(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"{OUT.relative_to(ROOT)} güncel değil: betiği --check'siz çalıştırın.")
        print("Kabul senaryosu güncel.")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
