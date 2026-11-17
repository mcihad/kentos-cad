#!/usr/bin/env python3
"""IFC-04's first acceptance, read by another program (docs/adr/8105): IfcOpenShell opens the declared section building
KentOS wrote with its types (fixtures/arch/v1/ifc/kesit-binasi-tipli.ifc) and the styles as the drawing says them
(kesit-binasi-tipli.json, written beside it from the drawing's settings, not from the file). Each style must be one
IfcWallType by its name; every wall of the style (its GlobalId the compressed form of its KentOS id) typed by it and by
nothing else; the type's material an IfcMaterialLayerSet whose layers are the style's components in order, each its
name, its material and its thickness; and the wall's own material the same set. Prints what it read; exits 1 when
something differs.

    uv run --offline --no-project --python 3.11 --with ifcopenshell==0.9.0.post1 \\
        python -I scripts/fixtures/ifc_types_check.py
"""

import json
import sys
import uuid
from pathlib import Path

import ifcopenshell
import ifcopenshell.guid
import ifcopenshell.util.element as ue

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/arch/v1/ifc"


def main():
    note = json.loads((DIR / "kesit-binasi-tipli.json").read_text(encoding="utf-8"))
    f = ifcopenshell.open(str(DIR / note["file"]))
    problems = []
    report = []
    types = {t.Name: t for t in f.by_type("IfcWallType")}
    walls = f.by_type("IfcWall")
    typed = {w.GlobalId: ue.get_type(w) for w in walls}
    for s in note["styles"]:
        t = types.get(s["name"])
        if t is None:
            problems.append(f"{s['name']}: IfcWallType yok")
            continue
        material = ue.get_material(t)
        layers = []
        if material is not None and material.is_a("IfcMaterialLayerSet"):
            layers = [{"name": l.Name, "material": l.Material.Name if l.Material else None,
                       "thickness": round(float(l.LayerThickness), 9)} for l in material.MaterialLayers]
        else:
            problems.append(f"{s['name']}: tipin malzemesi katman takımı değil ({material.is_a() if material else 'yok'})")
        want = [{"name": l["name"], "material": l["material"], "thickness": round(float(l["thickness"]), 9)} for l in s["layers"]]
        if layers != want:
            problems.append(f"{s['name']}: katmanlar {layers}, çizimde {want}")
        gids = {ifcopenshell.guid.compress(uuid.UUID(u).hex) for u in s["walls"]}
        mine = {g for g, tt in typed.items() if tt is not None and tt.id() == t.id()}
        if mine != gids:
            problems.append(f"{s['name']}: tipli duvarlar {sorted(mine)}, çizimde {sorted(gids)}")
        for g in gids:
            w = f.by_guid(g)
            own = ue.get_material(w, should_inherit=False)
            if own is None or own.id() != material.id():
                problems.append(f"{s['name']}: {g} duvarının malzemesi tipin takımı değil")
        report.append({"type": t.Name, "globalId": t.GlobalId, "walls": len(mine),
                       "layers": [f"{l['name']} {l['thickness']} m" for l in layers]})
    declared = [r for r in f.by_type("IfcRelDeclares") if any(x.is_a("IfcWallType") for x in r.RelatedDefinitions)]
    if len(declared) != 1:
        problems.append(f"{len(declared)} IfcRelDeclares tipleri anıyor")
    print(json.dumps(report, ensure_ascii=False, indent=2))
    if problems:
        print("uyuşmuyor: " + "; ".join(problems), file=sys.stderr)
        return 1
    print("uyuyor: her stil bir IfcWallType, duvarları ona bağlı, katmanları sırasıyla ve kalınlığıyla")
    return 0


if __name__ == "__main__":
    sys.exit(main())
