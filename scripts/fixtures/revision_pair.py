#!/usr/bin/env python3
"""The section building's second revision (docs/adr/8103; plan/ifc-uyum.md IFC-10), made by IfcOpenShell, not by KentOS:
the first revision is KentOS's file of the declared section building (fixtures/arch/v1/ifc/kesit-binasi-tam.ifc); the
second is the same file with two changes another program made, its GlobalIds kept:

- the ground floor's interior wall between the rooms on y 4.5 (01alMXGa9m080000000007, no opening) moved 0.50 m north:
  its placement takes a new location, its body follows;
- the ground floor's east wall (01alMXGa9m080000000002) said REI 90 instead of REI 60 (Pset_WallCommon.FireRating; each
  wall has its own set).

Writes fixtures/ifc/v1/revizyon/kesit-binasi-r2.ifc and kesit-binasi-r2.json (the two changes, the moved wall's box in
each revision by IfcOpenShell's geometry). `--check` compares. Runs under IfcOpenShell, offline:

    uv run --offline --no-project --python 3.11 --with ifcopenshell==0.9.0.post1 \\
        python -I scripts/fixtures/revision_pair.py [--check]
"""

import json
import re
import sys
import tempfile
from pathlib import Path

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element as ue

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "fixtures/arch/v1/ifc/kesit-binasi-tam.ifc"
OUT = ROOT / "fixtures/ifc/v1/revizyon"
MOVED = "01alMXGa9m080000000007"
MOVED_BY = (0.0, 0.5, 0.0)
RATED = "01alMXGa9m080000000002"
RATING = ("REI 60", "REI 90")


def box(f, gid):
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    v = ifcopenshell.geom.create_shape(settings, f.by_guid(gid)).geometry.verts
    xs, ys, zs = v[0::3], v[1::3], v[2::3]
    return [round(x, 6) for x in (min(xs), min(ys), min(zs), max(xs), max(ys), max(zs))]


def revised():
    f = ifcopenshell.open(str(SOURCE))
    before = box(f, MOVED)
    wall = f.by_guid(MOVED)
    old = wall.ObjectPlacement
    axis = old.RelativePlacement
    at = axis.Location.Coordinates
    point = f.createIfcCartesianPoint(tuple(a + b for a, b in zip(at, MOVED_BY)))
    wall.ObjectPlacement = f.createIfcLocalPlacement(
        old.PlacementRelTo, f.createIfcAxis2Placement3D(point, axis.Axis, axis.RefDirection))
    f.remove(old)
    rated = f.by_guid(RATED)
    pset = f.by_id(ue.get_psets(rated)["Pset_WallCommon"]["id"])
    prop = next(p for p in pset.HasProperties if p.Name == "FireRating")
    assert prop.NominalValue.wrappedValue == RATING[0], prop.NominalValue
    prop.NominalValue = f.create_entity("IfcLabel", RATING[1])
    after = box(f, MOVED)
    with tempfile.TemporaryDirectory() as d:
        p = Path(d) / "x.ifc"
        f.write(str(p))
        text = p.read_text(encoding="utf-8")
    fixed = (f"FILE_NAME('kesit-binasi-r2.ifc','2026-11-17T12:00:00',(''),(''),'IfcOpenShell {ifcopenshell.version}',"
             f"'IfcOpenShell','');")
    text = re.sub(r"FILE_NAME\(.*?\);", lambda _: fixed, text, count=1, flags=re.S)
    note = {
        "source": "fixtures/arch/v1/ifc/kesit-binasi-tam.ifc",
        "writer": "IfcOpenShell " + ifcopenshell.version,
        "moved": {"globalId": MOVED, "by": list(MOVED_BY), "before": before, "after": after},
        "rated": {"globalId": RATED, "set": "Pset_WallCommon", "name": "FireRating", "old": RATING[0], "new": RATING[1]},
    }
    return text, json.dumps(note, ensure_ascii=False, indent=2) + "\n"


def main():
    check = "--check" in sys.argv
    text, note = revised()
    files = {OUT / "kesit-binasi-r2.ifc": text, OUT / "kesit-binasi-r2.json": note}
    stale = []
    for path, content in files.items():
        if check:
            if not path.exists() or path.read_text(encoding="utf-8") != content:
                stale.append(path.name)
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content, encoding="utf-8")
    if stale:
        print("güncel değil: " + ", ".join(stale), file=sys.stderr)
        return 1
    print(("güncel: " if check else "yazıldı: ") + ", ".join(p.name for p in files))
    return 0


if __name__ == "__main__":
    sys.exit(main())
