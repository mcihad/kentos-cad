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

With `--uyeler FILE` it also reads a post type's and a rail type's sections (docs/adr/8170) in FILE, the one
`KENTOS_WRITE_IFC_MEMBER_TYPES=FILE cargo test -p kentos-native-application --test all a_post_type` keeps; the validator
then needs `--with lark --with pytest`.
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
    report += element_types(problems)
    if "--uyeler" in sys.argv:
        report += members(Path(sys.argv[sys.argv.index("--uyeler") + 1]), problems)
    print(json.dumps(report, ensure_ascii=False, indent=2))
    if problems:
        print("uyuşmuyor: " + "; ".join(problems), file=sys.stderr)
        return 1
    print("uyuyor: her stil bir IfcWallType, duvarları ona bağlı, katmanları sırasıyla ve kalınlığıyla; her eleman tipi "
          "sınıfının IFC tipi, elemanları ona bağlı (docs/adr/8106); kolon ve kiriş tipinin kesiti profil takımında, "
          "elemanlar kendi malzemesiyle (docs/adr/8167)"
          + ("; dikme ve kayıt tipinin kesiti rolsüz profil takımında (docs/adr/8170)"
             if "--uyeler" in sys.argv else ""))
    return 0


# Each element type's IFC class and the class of the elements it types (docs/adr/8106).
OCCURRENCE = {"IfcColumnType": "IfcColumn", "IfcMemberType": "IfcMember", "IfcBeamType": "IfcBeam",
              "IfcSlabType": "IfcSlab", "IfcCoveringType": "IfcCovering", "IfcRoofType": "IfcRoof",
              "IfcRailingType": "IfcRailing", "IfcStairType": "IfcStair"}


def element_types(problems):
    """The section building typed by its elements' own values (kesit-binasi-eleman-tipli.ifc): each element type of
    the drawing one IFC type of its class by its name, its elements (their GlobalIds the compressed form of their ids)
    of that class typed by it and by nothing else, every type declared in the project."""
    note_path = DIR / "kesit-binasi-eleman-tipli.json"
    note = json.loads(note_path.read_text(encoding="utf-8"))
    f = ifcopenshell.open(str(DIR / note["file"]))
    out = []
    for t in note["types"]:
        found = [x for x in f.by_type("IfcTypeObject") if x.Name == t["name"] and x.is_a().upper() == t["ifc"]]
        if len(found) != 1:
            problems.append(f"{t['name']}: {t['ifc']} {len(found)} kez")
            continue
        x = found[0]
        occurrence = OCCURRENCE.get(x.is_a())
        want = set()
        for u in t["members"]:
            g = ifcopenshell.guid.compress(uuid.UUID(u).hex)
            try:
                e = f.by_guid(g)
            except RuntimeError:
                continue
            if occurrence and e.is_a(occurrence):
                want.add(g)
        typed = {e.GlobalId for e in f.by_type("IfcProduct") if (ue.get_type(e) or None) and ue.get_type(e).id() == x.id()}
        if typed != want or not typed:
            problems.append(f"{t['name']}: tipli {sorted(typed)}, çizimde {sorted(want)}")
        if not any(x in r.RelatedDefinitions for r in f.by_type("IfcRelDeclares")):
            problems.append(f"{t['name']}: projede bildirilmemiş")
        row = {"type": t["name"], "ifc": x.is_a(), "globalId": x.GlobalId, "predefined": x.PredefinedType,
               "elements": len(typed)}
        if t.get("profile"):
            row["profile"] = section(x, t["profile"], typed, f, problems)
        out.append(row)
    return out


def section(x, want, typed, f, problems):
    """A column type's or a beam type's section (docs/adr/8167): its material an IfcMaterialProfileSet of one profile,
    LoadBearing (a post type's or a rail type's no role, docs/adr/8170), whose profile is the drawing's (a rectangle its
    XDim and YDim, a circle its Radius, an L an arbitrary closed profile); its elements keep their own material, an
    IfcMaterial."""
    m = ue.get_material(x)
    if m is None or not m.is_a("IfcMaterialProfileSet") or len(m.MaterialProfiles) != 1:
        problems.append(f"{x.Name}: tipin malzemesi tek profilli profil takımı değil ({m.is_a() if m else 'yok'})")
        return None
    mp = m.MaterialProfiles[0]
    p = mp.Profile
    near = lambda a, b: abs(float(a) - float(b)) <= 1e-9
    if want["kind"] == "rectangle":
        same = p.is_a("IfcRectangleProfileDef") and near(p.XDim, want["x"]) and near(p.YDim, want["y"])
        read = f"{p.is_a()} {getattr(p, 'XDim', None)} × {getattr(p, 'YDim', None)}"
    elif want["kind"] == "circle":
        same = p.is_a("IfcCircleProfileDef") and near(p.Radius, want["radius"])
        read = f"{p.is_a()} r {getattr(p, 'Radius', None)}"
    else:
        same = p.is_a("IfcArbitraryClosedProfileDef")
        read = p.is_a()
    if not same:
        problems.append(f"{x.Name}: kesit {read}, çizimde {want}")
    role = None if x.is_a("IfcMemberType") else "LoadBearing"
    if mp.Category != role:
        problems.append(f"{x.Name}: kesitin rolü {mp.Category}, beklenen {role}")
    for g in typed:
        own = ue.get_material(f.by_guid(g), should_inherit=False)
        if own is None or not own.is_a("IfcMaterial"):
            kind = own.is_a() if own else "yok"
            problems.append(f"{x.Name}: {g} elemanının kendi malzemesi IfcMaterial değil ({kind})")
    return {"name": p.ProfileName, "read": read, "material": mp.Material.Name if mp.Material else None}


def members(path, problems):
    """Post and rail types (docs/adr/8170) in the file the application test
    `a_post_type_s_and_a_rail_type_s_section_is_their_profile_set` keeps (`KENTOS_WRITE_IFC_MEMBER_TYPES=<path>`): each
    IfcMemberType's material a profile set of one profile, named by its type, a rectangle whose XDim and YDim are the
    centimetres its ProfileName says ("10×10"), its material its elements', no role; its elements keep their own
    IfcMaterial of that name; IfcOpenShell's validator (attributes, EXPRESS rules) finds nothing."""
    import ifcopenshell.validate

    f = ifcopenshell.open(str(path))
    log = ifcopenshell.validate.json_logger()
    ifcopenshell.validate.validate(f, log, express_rules=True)
    if log.statements:
        problems.append(f"{path.name}: doğrulayıcı {[s.get('message', s) for s in log.statements[:5]]}")
    out = []
    found = f.by_type("IfcMemberType")
    if not found:
        problems.append(f"{path.name}: IfcMemberType yok")
    for x in found:
        m = ue.get_material(x)
        if m is None or not m.is_a("IfcMaterialProfileSet") or len(m.MaterialProfiles) != 1:
            problems.append(f"{x.Name}: tipin malzemesi tek profilli profil takımı değil ({m.is_a() if m else 'yok'})")
            continue
        mp, near = m.MaterialProfiles[0], lambda a, b: abs(float(a) - float(b)) <= 1e-9
        p = mp.Profile
        cm = [int(v) / 100 for v in (p.ProfileName or "").split("×")]
        if not (p.is_a("IfcRectangleProfileDef") and len(cm) == 2 and near(p.XDim, cm[0]) and near(p.YDim, cm[1])):
            dims = f"{getattr(p, 'XDim', None)} × {getattr(p, 'YDim', None)}"
            problems.append(f"{x.Name}: kesit {p.is_a()} {p.ProfileName} {dims}")
        if m.Name != x.Name or mp.Category is not None:
            problems.append(f"{x.Name}: takımın adı {m.Name}, rolü {mp.Category}")
        material = mp.Material.Name if mp.Material else None
        typed = [e for e in f.by_type("IfcMember") if ue.get_type(e) is not None and ue.get_type(e).id() == x.id()]
        for e in typed:
            own = ue.get_material(e, should_inherit=False)
            if own is None or not own.is_a("IfcMaterial") or own.Name != material:
                problems.append(f"{x.Name}: {e.GlobalId} elemanının kendi malzemesi {own}")
        out.append({"type": x.Name, "ifc": x.is_a(), "elements": len(typed),
                    "profile": {"name": p.ProfileName, "x": p.XDim, "y": p.YDim, "material": material}})
    return out


if __name__ == "__main__":
    sys.exit(main())
