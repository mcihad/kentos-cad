#!/usr/bin/env python3
"""IFC-04's layered slabs, read by another program (docs/adr/8166): IfcOpenShell opens the declared section building
KentOS wrote (fixtures/arch/v1/ifc/kesit-binasi-tipli.ifc) and the layered slabs as the drawing says them
(kesit-binasi-tipli.json's `slabs`, written beside it from the drawing, not from the file). Each slab (its GlobalId the
compressed form of its KentOS id) must be associated with an IfcMaterialLayerSetUsage along its z axis (AXIS3),
downwards (NEGATIVE), its OffsetFromReferenceLine its top over its storey; the usage's set's layers the drawing's from
the top, each its name, its material and its thickness; and the body IfcOpenShell builds must stand from its top down
by the layers' whole thickness. Prints what it read; exits 1 when something differs.

    uv run --offline --no-project --python 3.11 --with ifcopenshell==0.9.0.post1 \\
        python -I scripts/fixtures/slab_layers_check.py
"""

import json
import sys
import uuid
from pathlib import Path

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.guid
import ifcopenshell.util.element as ue

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/arch/v1/ifc"
#: How near two lengths must be, metres: the file writes them as the drawing gives them.
NEAR = 1e-6


def main():
    note = json.loads((DIR / "kesit-binasi-tipli.json").read_text(encoding="utf-8"))
    f = ifcopenshell.open(str(DIR / note["file"]))
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    problems = []
    report = []
    for s in note["slabs"]:
        gid = ifcopenshell.guid.compress(uuid.UUID(s["id"]).hex)
        try:
            slab = f.by_guid(gid)
        except RuntimeError:
            problems.append(f"{s['id']}: IfcSlab yok ({gid})")
            continue
        usage = ue.get_material(slab)
        if usage is None or not usage.is_a("IfcMaterialLayerSetUsage"):
            problems.append(f"{gid}: malzemesi katman takımı kullanımı değil ({usage})")
            continue
        if (usage.LayerSetDirection, usage.DirectionSense) != ("AXIS3", "NEGATIVE"):
            problems.append(f"{gid}: kullanım {usage.LayerSetDirection} {usage.DirectionSense}")
        if abs(usage.OffsetFromReferenceLine - s["top"]) > NEAR:
            problems.append(f"{gid}: taban çizgisi {usage.OffsetFromReferenceLine} ≠ üstü {s['top']}")
        layers = [{"name": l.Name, "material": l.Material.Name if l.Material else None, "thickness": l.LayerThickness}
                  for l in usage.ForLayerSet.MaterialLayers]
        expected = [{"name": l["name"], "material": l["material"], "thickness": l["thickness"]} for l in s["layers"]]
        same = len(layers) == len(expected) and all(
            a["name"] == b["name"] and a["material"] == b["material"] and abs(a["thickness"] - b["thickness"]) <= NEAR
            for a, b in zip(layers, expected))
        if not same:
            problems.append(f"{gid}: katmanlar {layers} ≠ çizimdeki {expected}")
        # The body: from the slab's top over its storey down by the layers' whole thickness.
        storey = ue.get_container(slab)
        floor = storey.Elevation or 0.0
        shape = ifcopenshell.geom.create_shape(settings, slab)
        zs = shape.geometry.verts[2::3]
        top, bottom = max(zs) - floor, min(zs) - floor
        whole = sum(l["thickness"] for l in expected)
        if abs(top - s["top"]) > NEAR or abs((top - bottom) - whole) > NEAR:
            problems.append(f"{gid}: gövde {bottom:.4f}–{top:.4f}, takım {s['top'] - whole:.4f}–{s['top']:.4f}")
        report.append({"globalId": gid, "storey": storey.Name, "top": s["top"], "whole": round(whole, 6),
                       "layers": [f"{l['name']} {l['thickness']}" for l in layers]})
    print(json.dumps(report, ensure_ascii=False, indent=2))
    if problems:
        print("\n".join(problems), file=sys.stderr)
        sys.exit(1)
    print("uyuyor: her katmanlı döşeme üstünden aşağı (AXIS3, NEGATIVE) bir katman takımı kullanımıyla, katmanları "
          "çizimdeki sırasıyla ve kalınlığıyla, gövdesi takımın kalınlığınca (docs/adr/8166)")


if __name__ == "__main__":
    main()
