#!/usr/bin/env python3
"""IFC files another program writes (docs/adr/8061 §7 "Açık örnekler"; plan/ifc-uyum.md IFC-07): a small building
authored with IfcOpenShell's own API, not KentOS's writer, once in IFC4 and once in IFC 4.3, so the reading face meets
another writer's spelling: its contexts and units, its wall and slab bodies, an opening with a door in it, a space,
a property set and a quantity set, a type and a material, and Turkish names as that writer escapes them.

Deterministic: every GlobalId is made from the object's class and its place (UUID 5), the time stamp is fixed, so
--check compares byte for byte. Writes fixtures/ifc/v1/acik/ifcopenshell-ifc4.ifc and ifcopenshell-ifc4x3.ifc:

    uv run --offline --no-project --python 3.11 --with ifcopenshell==0.9.0.post1 python -I scripts/fixtures/ifc_other_writer_cases.py
"""

import argparse
import sys
import tempfile
import uuid
from pathlib import Path

import ifcopenshell
import ifcopenshell.api
import ifcopenshell.guid
import numpy as np

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "ifc" / "v1" / "acik"
SCHEMAS = {"ifcopenshell-ifc4.ifc": "IFC4", "ifcopenshell-ifc4x3.ifc": "IFC4X3"}
NAMESPACE = uuid.UUID("8061a0e2-1b6f-4c1e-9a3e-0e2b2f0a1207")


def placed(f, product, x=0.0, y=0.0, z=0.0, turn=0.0):
    m = np.eye(4)
    c, s = np.cos(turn), np.sin(turn)
    m[:3, :3] = [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]]
    m[:3, 3] = [x, y, z]
    ifcopenshell.api.run("geometry.edit_object_placement", f, product=product, matrix=m, is_si=True)


def build(schema):
    run = ifcopenshell.api.run
    f = run("project.create_file", version=schema)
    project = run("root.create_entity", f, ifc_class="IfcProject", name="Örnek yapı")
    units = [run("unit.add_si_unit", f, unit_type=t) for t in ("LENGTHUNIT", "AREAUNIT", "VOLUMEUNIT")]
    run("unit.assign_unit", f, units=units)
    model = run("context.add_context", f, context_type="Model")
    body = run("context.add_context", f, context_type="Model", context_identifier="Body", target_view="MODEL_VIEW", parent=model)
    site = run("root.create_entity", f, ifc_class="IfcSite", name="Arsa")
    building = run("root.create_entity", f, ifc_class="IfcBuilding", name="Bina")
    storey = run("root.create_entity", f, ifc_class="IfcBuildingStorey", name="Zemin kat")
    run("aggregate.assign_object", f, relating_object=project, products=[site])
    run("aggregate.assign_object", f, relating_object=site, products=[building])
    run("aggregate.assign_object", f, relating_object=building, products=[storey])
    for p in (site, building, storey):
        placed(f, p)

    wall_type = run("root.create_entity", f, ifc_class="IfcWallType", name="Tuğla duvar 20")
    brick = run("material.add_material", f, name="Tuğla")
    walls = []
    for i, (x, y, turn, length) in enumerate([(0.0, 0.0, 0.0, 6.0), (6.0, 0.0, np.pi / 2, 4.0), (6.0, 4.0, np.pi, 6.0), (0.0, 4.0, -np.pi / 2, 4.0)]):
        wall = run("root.create_entity", f, ifc_class="IfcWall", name=f"Duvar {i + 1} — dış")
        rep = run("geometry.add_wall_representation", f, context=body, length=length, height=3.0, thickness=0.2)
        run("geometry.assign_representation", f, product=wall, representation=rep)
        placed(f, wall, x, y, 0.0, turn)
        run("spatial.assign_container", f, relating_structure=storey, products=[wall])
        walls.append(wall)
    run("type.assign_type", f, related_objects=walls, relating_type=wall_type)
    run("material.assign_material", f, products=walls, material=brick)
    pset = run("pset.add_pset", f, product=walls[0], name="Pset_WallCommon")
    run("pset.edit_pset", f, pset=pset, properties={"IsExternal": True, "FireRating": "REI 60", "Reference": "Dış duvar"})
    qto = run("pset.add_qto", f, product=walls[0], name="Qto_WallBaseQuantities")
    run("pset.edit_qto", f, qto=qto, properties={"Length": 6.0, "Height": 3.0, "Width": 0.2})

    opening = run("root.create_entity", f, ifc_class="IfcOpeningElement", name="Kapı boşluğu")
    rep = run("geometry.add_wall_representation", f, context=body, length=1.0, height=2.1, thickness=0.4)
    run("geometry.assign_representation", f, product=opening, representation=rep)
    placed(f, opening, 2.0, -0.1, 0.0)
    run("feature.add_feature", f, feature=opening, element=walls[0])
    door = run("root.create_entity", f, ifc_class="IfcDoor", name="Giriş kapısı")
    rep = run("geometry.add_wall_representation", f, context=body, length=1.0, height=2.1, thickness=0.05)
    run("geometry.assign_representation", f, product=door, representation=rep)
    placed(f, door, 2.0, 0.075, 0.0)
    run("feature.add_filling", f, opening=opening, element=door)
    run("spatial.assign_container", f, relating_structure=storey, products=[door])

    slab = run("root.create_entity", f, ifc_class="IfcSlab", name="Döşeme")
    rep = run("geometry.add_slab_representation", f, context=body, depth=0.2, polyline=[(0.0, 0.0), (6.0, 0.0), (6.0, 4.0), (0.0, 4.0)])
    run("geometry.assign_representation", f, product=slab, representation=rep)
    placed(f, slab, 0.0, 0.0, -0.2)
    run("spatial.assign_container", f, relating_structure=storey, products=[slab])

    space = run("root.create_entity", f, ifc_class="IfcSpace", name="101")
    space.LongName = "Oturma odası"
    rep = run("geometry.add_slab_representation", f, context=body, depth=3.0, polyline=[(0.1, 0.1), (5.9, 0.1), (5.9, 3.9), (0.1, 3.9)])
    run("geometry.assign_representation", f, product=space, representation=rep)
    placed(f, space, 0.0, 0.0, 0.0)
    run("aggregate.assign_object", f, relating_object=storey, products=[space])

    for i, e in enumerate(sorted(f.by_type("IfcRoot"), key=lambda e: e.id())):
        e.GlobalId = ifcopenshell.guid.compress(uuid.uuid5(NAMESPACE, f"{schema}/{e.is_a()}/{i}").hex)
    f.header.file_name.time_stamp = "2026-11-16T12:00:00"
    f.header.file_name.name = "Örnek yapı.ifc"
    return f


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="compare with the files instead of writing them")
    args = parser.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    stale = []
    for name, schema in SCHEMAS.items():
        with tempfile.TemporaryDirectory() as d:
            tmp = Path(d) / name
            build(schema).write(str(tmp))
            text = tmp.read_bytes()
        out = OUT / name
        if args.check:
            if not out.exists() or out.read_bytes() != text:
                stale.append(out)
        else:
            out.write_bytes(text)
            print(out.relative_to(ROOT))
    if stale:
        for s in stale:
            print(f"{s.relative_to(ROOT)} IfcOpenShell'in yazdığıyla aynı değil; betiği --check olmadan çalıştırın.")
        sys.exit(1)


if __name__ == "__main__":
    main()
