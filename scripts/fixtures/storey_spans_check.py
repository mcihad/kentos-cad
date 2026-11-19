#!/usr/bin/env python3
"""IFC-04's elements over their storey, read by another program (docs/adr/8168): IfcOpenShell opens the files KentOS
wrote and, from each element's own body as IfcOpenShell builds it in the world and each storey's placement, finds the
higher storeys a wall, a column, a curtain wall, a member, a stair, a ramp, a railing, an elevator or a chimney rises
into, its top more than 1 mm over their floor (the project's spatial tolerance, docs/specs/numeric-policy.md §5). Each
must be referenced (IfcRelReferencedInSpatialStructure) in exactly those storeys; a storey references nothing else but
zones (IfcSpatialZone, IfcZone).

fixtures/arch/v1/ifc/kesit-binasi-kalkan-duvari.ifc is the section building with its east wall drawn once over both
storeys and on over the roof (the application test `a_wall_two_storeys_tall_spans_the_storey_over_its_own`). There the
one element referenced is that wall, in the 1. kat: its GlobalId the section building's definition's id
(section_building.py, not KentOS), its body 6.6 m tall as drawn. IfcOpenShell's validator (attributes, EXPRESS rules)
finds nothing in the file. Prints what it read; exits 1 when something differs.

    uv run --offline --no-project --python 3.11 --with ifcopenshell==0.9.0.post1 --with lark --with pytest \\
        python -I scripts/fixtures/storey_spans_check.py [FILE ...]
"""

import sys
import uuid
from pathlib import Path

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.guid
import ifcopenshell.util.element as ue
import ifcopenshell.util.placement as up
import ifcopenshell.validate

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/arch/v1/ifc"
sys.path.insert(0, str(Path(__file__).resolve().parent))
import section_building as sb  # noqa: E402

#: The files the writer writes again from their drawings (the others' ids are new at each writing).
FILES = ["kesit-binasi.ifc", "kesit-binasi-kompartiman.ifc", "kesit-binasi-kunye.ifc", "kesit-binasi-tipli.ifc",
         "kesit-binasi-tam.ifc", "kesit-binasi-eleman-tipli.ifc", "kesit-binasi-kalkan-duvari.ifc", "egri-bina.ifc",
         "yerel-sistem.ifc"]
#: The vertical elements that may stand in more than one storey.
CLASSES = ("IfcWall", "IfcColumn", "IfcCurtainWall", "IfcMember", "IfcStair", "IfcRamp", "IfcRailing",
           "IfcTransportElement", "IfcChimney")
#: How far a top must rise over a storey's floor to stand in it, metres: the project's spatial tolerance.
SPAN = 0.001
PARTY = "kesit-binasi-kalkan-duvari.ifc"


def check(path: Path, problems: list[str]) -> None:
    f = ifcopenshell.open(str(path))
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    storeys = f.by_type("IfcBuildingStorey")
    floor = {s.id(): float(up.get_local_placement(s.ObjectPlacement)[2][3]) for s in storeys}
    spans: dict[int, set[int]] = {}
    for e in f.by_type("IfcElement"):
        if not any(e.is_a(c) for c in CLASSES) or e.Representation is None:
            continue
        own = ue.get_container(e)
        if own is None or not own.is_a("IfcBuildingStorey"):
            continue
        verts = ifcopenshell.geom.create_shape(settings, e).geometry.verts
        top = max(verts[2::3])
        rises = {s.id() for s in storeys if floor[s.id()] > floor[own.id()] + 1e-9 and top - floor[s.id()] > SPAN}
        referenced = {r.RelatingStructure.id() for r in e.ReferencedInStructures
                      if r.RelatingStructure.is_a("IfcBuildingStorey")}
        if rises != referenced:
            problems.append(f"{path.name}: {e.Name} ({e.GlobalId}) top {top:.3f} m on {own.Name}: rises into "
                            f"{sorted(f.by_id(i).Name for i in rises)}, referenced in "
                            f"{sorted(f.by_id(i).Name for i in referenced)}")
        if rises:
            spans[e.id()] = rises
    found = []
    for r in f.by_type("IfcRelReferencedInSpatialStructure"):
        if not r.RelatingStructure.is_a("IfcBuildingStorey"):
            continue
        for o in r.RelatedElements:
            # Zones are referenced in the storeys they lie in (IFC 4.3: IfcSpatialZone, an IfcZone group).
            if o.is_a("IfcGroup") or o.is_a("IfcSpatialZone"):
                continue
            if r.RelatingStructure.id() not in spans.get(o.id(), set()):
                problems.append(f"{path.name}: {o.Name} ({o.is_a()}) referenced in {r.RelatingStructure.Name}, "
                                "which it does not rise into")
            found.append((o, r.RelatingStructure))
    print(f"{path.name}: {len(storeys)} storeys; "
          + (", ".join(f"{o.Name} {o.GlobalId} → {s.Name}" for o, s in found) or "nothing over its storey"))
    if path.name == PARTY:
        east = next(u for e, u in zip(sb.d["entities"], sb.uids)
                    if e["kind"] == "wall" and e.get("arch", {}).get("level") == "zemin"
                    and [(p["x"], p["y"]) for p in e["pts"]] == [(12.0, 0.0), (12.0, 9.0)])
        guid = ifcopenshell.guid.compress(uuid.UUID(east).hex)
        got = [(o.GlobalId, s.Name) for o, s in found]
        if got != [(guid, "MM-BNK-1._kat")]:
            problems.append(f"{path.name}: references {got}, the east wall {guid} in MM-BNK-1._kat alone")
        # Drawn 6.6 m tall from its base, which stands on the ground slab's structure under the finished floor.
        z = ifcopenshell.geom.create_shape(settings, f.by_guid(guid)).geometry.verts[2::3]
        if abs(max(z) - min(z) - 6.6) > 1e-6:
            problems.append(f"{path.name}: the east wall from {min(z)} to {max(z)}, 6.6 m tall drawn")
        log = ifcopenshell.validate.json_logger()
        ifcopenshell.validate.validate(f, log, express_rules=True)
        if log.statements:
            problems.append(f"{path.name}: validator {[s.get('message', s) for s in log.statements[:5]]}")
        else:
            print(f"{path.name}: 0 validator findings")


def main() -> int:
    paths = [Path(a).resolve() for a in sys.argv[1:]] or [DIR / name for name in FILES]
    problems: list[str] = []
    for path in paths:
        check(path, problems)
    for p in problems:
        print("✗", p)
    if not problems:
        print("✓ every element stands in the storeys it rises into, and in no other")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
