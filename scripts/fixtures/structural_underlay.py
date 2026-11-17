#!/usr/bin/env python3
"""The section building's structural model (docs/adr/8102; plan/bim-plan.md BIM-17, B19), written by IfcOpenShell, not by
KentOS: the alignment and clash checks' independent other discipline.

The architecture is fixtures/arch/v1/ifc/kesit-binasi-tam.ifc (KentOS's file of the section building on TUREF / TM33,
±0.00 at 1285.40 m above Antalya height). IfcOpenShell's geometry engine reads its columns, beams and slabs (world
coordinates) and their boxes become the structural model's elements: extruded rectangles, in millimetres (the file says
so, and its map conversion's Scale 0.001 says the same to the map), on the same system and height datum, its storeys
"Zemin Kat" at 0 and "1. Kat" at 3000 mm, its grids A–D and 1–3 where the section building's are (x 0, 4.5, 7.5, 12;
y 0, 4.5, 9; scripts/fixtures/section_building.py). Three files:

- kesit-binasi-statik.ifc: aligned. Every column, beam and slab has the architecture's box.
- kesit-binasi-statik-kaymis.ifc: the ground floor's column on A-1 moved 400 mm east, and "1. Kat" said at 3150 mm.
- kesit-binasi-statik-datum.ifc: aligned, its heights on ITRF96 (ellipsoidal) instead of Antalya height.

And kesit-binasi-statik.json: what the files hold (the moved column's GlobalId, the elements by class). `--check` compares.
Runs under IfcOpenShell, offline:

    uv run --offline --no-project --python 3.11 --with ifcopenshell==0.9.0.post1 \\
        python -I scripts/fixtures/structural_underlay.py [--check]
"""

import json
import sys
import tempfile
from pathlib import Path

import ifcopenshell
import ifcopenshell.api
import ifcopenshell.geom
import ifcopenshell.guid

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "fixtures/arch/v1/ifc/kesit-binasi-tam.ifc"
OUT = ROOT / "fixtures/ifc/v1/statik"
NOTE = OUT / "kesit-binasi-statik.json"
GRIDS_U = [("A", 0.0), ("B", 4.5), ("C", 7.5), ("D", 12.0)]
GRIDS_V = [("1", 0.0), ("2", 4.5), ("3", 9.0)]
STOREYS = [("Zemin Kat", 0.0), ("1. Kat", 3.0)]
CLASSES = ["IfcColumn", "IfcBeam", "IfcSlab"]
MOVED_BY = 0.4
SHIFTED_STOREY = 3.15


def boxes():
    """The architecture's columns, beams and slabs: class, name, storey's elevation, box (metres, world)."""
    src = ifcopenshell.open(str(SOURCE))
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = []
    for cls in CLASSES:
        for e in sorted(src.by_type(cls), key=lambda e: e.GlobalId):
            shape = ifcopenshell.geom.create_shape(settings, e)
            v = shape.geometry.verts
            xs, ys, zs = v[0::3], v[1::3], v[2::3]
            storey = None
            for rel in e.ContainedInStructure or []:
                storey = rel.RelatingStructure
            elevation = storey.Elevation if storey is not None else 0.0
            out.append({"class": cls, "name": e.Name, "elevation": elevation,
                        "box": [round(min(xs), 6), round(min(ys), 6), round(min(zs), 6),
                                round(max(xs), 6), round(max(ys), 6), round(max(zs), 6)]})
    return out


def guid(seed):
    """A GlobalId that is the same each time the file is written."""
    import uuid
    return ifcopenshell.guid.compress(uuid.uuid5(uuid.NAMESPACE_URL, "kentos:statik:" + seed).hex)


def written(elements, moved=False, datum="Antalya height (EPSG:5775)", shifted=False):
    f = ifcopenshell.file(schema="IFC4X3_ADD2")
    project = f.createIfcProject(guid("project"), None, "Kesit binası — statik")
    mm = f.createIfcSIUnit(None, "LENGTHUNIT", "MILLI", "METRE")
    area = f.createIfcSIUnit(None, "AREAUNIT", None, "SQUARE_METRE")
    volume = f.createIfcSIUnit(None, "VOLUMEUNIT", None, "CUBIC_METRE")
    radian = f.createIfcSIUnit(None, "PLANEANGLEUNIT", None, "RADIAN")
    project.UnitsInContext = f.createIfcUnitAssignment([mm, area, volume, radian])
    origin = f.createIfcCartesianPoint((0.0, 0.0, 0.0))
    world = f.createIfcAxis2Placement3D(origin, None, None)
    ctx = f.createIfcGeometricRepresentationContext(None, "Model", 3, 1.0e-05, world, None)
    body = f.createIfcGeometricRepresentationSubContext("Body", "Model", None, None, None, None, ctx, None,
                                                        "MODEL_VIEW", None)
    project.RepresentationContexts = [ctx]
    crs = f.createIfcProjectedCRS("EPSG:5255", "TUREF / TM33 (EPSG:5255)", "TUREF", datum,
                                  "Transverse Mercator", "33", None)
    f.createIfcMapConversion(ctx, crs, 0.0, 0.0, 1285.4, 1.0, 0.0, 0.001)

    def place(at, relative=None):
        return f.createIfcLocalPlacement(relative, f.createIfcAxis2Placement3D(f.createIfcCartesianPoint(at), None, None))

    site = f.createIfcSite(guid("site"), None, "Arsa", None, None, place((0.0, 0.0, 0.0)), None, None, "ELEMENT")
    building = f.createIfcBuilding(guid("building"), None, "Kesit binası", None, None, site.ObjectPlacement, None,
                                   None, "ELEMENT")
    f.createIfcRelAggregates(guid("project-site"), None, None, None, project, [site])
    f.createIfcRelAggregates(guid("site-building"), None, None, None, site, [building])
    storeys = []
    for name, elevation in STOREYS:
        said = SHIFTED_STOREY if shifted and name == "1. Kat" else elevation
        s = f.createIfcBuildingStorey(guid("storey:" + name), None, name, None, None,
                                      place((0.0, 0.0, elevation * 1000.0), building.ObjectPlacement), None, name,
                                      "ELEMENT", said * 1000.0)
        storeys.append((elevation, s))
    f.createIfcRelAggregates(guid("building-storeys"), None, None, None, building, [s for _, s in storeys])
    # The grids on the ground floor.
    def axis(tag, a, b):
        line = f.createIfcPolyline([f.createIfcCartesianPoint((a[0] * 1000.0, a[1] * 1000.0)),
                                    f.createIfcCartesianPoint((b[0] * 1000.0, b[1] * 1000.0))])
        return f.createIfcGridAxis(tag, line, True)
    u = [axis(t, (x, -2.0), (x, 11.0)) for t, x in GRIDS_U]
    v = [axis(t, (-2.0, y), (14.0, y)) for t, y in GRIDS_V]
    grid = f.createIfcGrid(guid("grid"), None, "Akslar", None, None, place((0.0, 0.0, 0.0), storeys[0][1].ObjectPlacement),
                           None, u, v, None, None)
    by_storey = {elevation: [] for elevation, _ in storeys}
    by_storey[0.0].append(grid)
    moved_id = None
    made = {cls: 0 for cls in CLASSES}
    for k, e in enumerate(elements):
        x0, y0, z0, x1, y1, z1 = e["box"]
        if moved and e["class"] == "IfcColumn" and moved_id is None and abs(x0 + 0.15) < 1e-6 \
                and abs(y0 + 0.15) < 1e-6 and e["elevation"] == 0.0:
            x0, x1 = x0 + MOVED_BY, x1 + MOVED_BY
            moved_id = guid(f"element:{k}")
        dx, dy, dz = (x1 - x0) * 1000.0, (y1 - y0) * 1000.0, (z1 - z0) * 1000.0
        profile = f.createIfcRectangleProfileDef("AREA", None,
                                                 f.createIfcAxis2Placement2D(f.createIfcCartesianPoint((0.0, 0.0)), None),
                                                 dx, dy)
        solid = f.createIfcExtrudedAreaSolid(profile, f.createIfcAxis2Placement3D(f.createIfcCartesianPoint((0.0, 0.0, 0.0)),
                                                                                 None, None),
                                             f.createIfcDirection((0.0, 0.0, 1.0)), dz)
        rep = f.createIfcShapeRepresentation(body, "Body", "SweptSolid", [solid])
        shape = f.createIfcProductDefinitionShape(None, None, [rep])
        at = place(((x0 + x1) / 2 * 1000.0, (y0 + y1) / 2 * 1000.0, z0 * 1000.0))
        name = f"S-{e['class'][3:].upper()}-{k + 1:02d}"
        product = getattr(f, "create" + e["class"])(guid(f"element:{k}"), None, name, None, None, at, shape, None, None)
        by_storey[e["elevation"]].append(product)
        made[e["class"]] += 1
    for elevation, s in storeys:
        f.createIfcRelContainedInSpatialStructure(guid(f"contains:{elevation}"), None, None, None, by_storey[elevation], s)
    return f, moved_id, made


def text_of(f, name):
    """The file's text, its FILE_NAME fixed so the bytes are the same each time."""
    import re
    with tempfile.TemporaryDirectory() as d:
        p = Path(d) / "x.ifc"
        f.write(str(p))
        text = p.read_text(encoding="utf-8")
    fixed = f"FILE_NAME('{name}','2026-11-17T12:00:00',(''),(''),'IfcOpenShell {ifcopenshell.version}','IfcOpenShell','');"
    return re.sub(r"FILE_NAME\(.*?\);", lambda _: fixed, text, count=1, flags=re.S)


def main():
    check = "--check" in sys.argv
    elements = boxes()
    files = {}
    plain, _, made = written(elements)
    files["kesit-binasi-statik.ifc"] = text_of(plain, "kesit-binasi-statik.ifc")
    moved, moved_id, _ = written(elements, moved=True, shifted=True)
    files["kesit-binasi-statik-kaymis.ifc"] = text_of(moved, "kesit-binasi-statik-kaymis.ifc")
    datum, _, _ = written(elements, datum="ITRF96")
    files["kesit-binasi-statik-datum.ifc"] = text_of(datum, "kesit-binasi-statik-datum.ifc")
    note = {"source": "fixtures/arch/v1/ifc/kesit-binasi-tam.ifc", "writer": "IfcOpenShell " + ifcopenshell.version,
            "elements": made, "moved": {"globalId": moved_id, "by": MOVED_BY, "storey": "Zemin Kat"},
            "shifted": {"storey": "1. Kat", "elevation": SHIFTED_STOREY}, "grids": [t for t, _ in GRIDS_U + GRIDS_V]}
    files[NOTE.name] = json.dumps(note, ensure_ascii=False, indent=2) + "\n"
    OUT.mkdir(parents=True, exist_ok=True)
    stale = []
    for name, text in files.items():
        path = OUT / name
        if check:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                stale.append(name)
        else:
            path.write_text(text, encoding="utf-8")
    if stale:
        print("güncel değil: " + ", ".join(stale), file=sys.stderr)
        return 1
    print(("güncel: " if check else "yazıldı: ") + ", ".join(files))
    return 0


if __name__ == "__main__":
    sys.exit(main())
