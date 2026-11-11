#!/usr/bin/env python3
"""The building's IFC 4.3 (docs/adr/8047) read by IfcOpenShell against the section building's own definition
(section_building.py), without KentOS code. Mimarlık ve Mühendislik Projelerinin Dijital Olarak Hazırlanması Hakkında
Yönetmelik (RG 5.8.2026/33331): EK-5 Tablo 5.1's classes and codes, §5.5's names, EK-6's properties.

    KENTOS_WRITE_IFC_BUILDING=1 cargo test -p kentos-native-application --test all arch_ifc
    uv run --no-project --python 3.11 --with ifcopenshell --with pytest --with lark \\
        python3 scripts/fixtures/arch_ifc_check.py

It checks, in fixtures/arch/v1/ifc/kesit-binasi.ifc:
- the schema is IFC 4.3 (EK-5 §5.4) and IfcOpenShell's validator (attributes, EXPRESS rules) finds nothing;
- one project, site and building; the storeys are the definition's levels, at their elevations;
- as many IfcWall, IfcDoor, IfcWindow, IfcSpace, IfcSlab, IfcColumn, IfcBeam and IfcStair as the definition has walls,
  doors, windows, rooms, slabs, columns, beams and stairs;
- every object's name is `MM-<its class's code>[-<description>]`, the description without white space or "-";
- an object's GlobalId is its id in the definition, compressed as IFC compresses a GUID (IfcOpenShell's own);
- each element stands on one storey, each room is part of one; each opening voids a wall and each door and window fills
  an opening of its own wall; doors' and windows' tags are the definition's numbers;
- a wall's Pset_WallCommon.IsExternal is whether it is the definition's outer wall (Dış duvar 25), a door's or window's
  Pset_*Common.IsExternal its wall's;
- a room's Qto_SpaceBaseQuantities.NetFloorArea is its rectangle's area, a storey's its rooms' together, the
  building's all of them; the building's Height the storeys' 3.00 (EK-6 Tablo 6.7, 6.8);
- a column carries (Pset_ColumnCommon.LoadBearing), a terrace slab is outside and a floor slab is not
  (Pset_SlabCommon.IsExternal);
- materials (EK-6): walls their core layer's (Tuğla), slabs theirs (Betonarme), columns and beams Betonarme;
- every element and room gives IfcOpenShell a shape."""
import math
import re
import sys
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))
import section_building as sb  # noqa: E402

FILE = ROOT / "fixtures/arch/v1/ifc/kesit-binasi.ifc"
# EK-5 Tablo 5.1.
CODES = {"IfcWall": "DVR", "IfcSlab": "DSM", "IfcColumn": "KLN", "IfcBeam": "KRS", "IfcRoof": "CAT", "IfcRailing": "KRK",
         "IfcStair": "MRD", "IfcDoor": "KAP", "IfcWindow": "PNC", "IfcSpace": "MHL", "IfcOpeningElement": "ACL",
         "IfcBuildingStorey": "BNK", "IfcBuilding": "BNA", "IfcSite": "SAH"}


def main() -> int:
    import ifcopenshell
    import ifcopenshell.geom
    import ifcopenshell.guid
    import ifcopenshell.util.element as ue
    import ifcopenshell.validate

    f = ifcopenshell.open(str(FILE))
    bad: list[str] = []

    def ensure(ok: bool, what: str) -> None:
        if not ok:
            bad.append(what)

    log = ifcopenshell.validate.json_logger()
    ifcopenshell.validate.validate(f, log, express_rules=True)
    ensure(f.schema_identifier == "IFC4X3_ADD2", f"schema {f.schema_identifier}")
    ensure(not log.statements, f"validator: {[s.get('message', s) for s in log.statements[:5]]}")

    ents = sb.d["entities"]
    ids = {e["id"]: u for e, u in zip(ents, sb.uids)}
    kind = lambda k: [e for e in ents if e["kind"] == k]
    openings = kind("opening")
    want = {"IfcWall": len(kind("wall")), "IfcDoor": sum(o["form"] == "door" for o in openings),
            "IfcWindow": sum(o["form"] == "window" for o in openings), "IfcSpace": len(kind("space")),
            "IfcSlab": len(kind("slab")), "IfcColumn": len(kind("column")), "IfcBeam": len(kind("beam")),
            "IfcStair": len(kind("stair")), "IfcProject": 1, "IfcSite": 1, "IfcBuilding": 1}
    for cls, n in want.items():
        ensure(len(f.by_type(cls)) == n, f"{cls}: {len(f.by_type(cls))}, the drawing {n}")
    levels = sb.d["settings"]["arch"]["levels"]
    storeys = sorted(f.by_type("IfcBuildingStorey"), key=lambda s: s.Elevation)
    ensure([(s.LongName, s.Elevation) for s in storeys] == [(lv["name"], lv["elevation"]) for lv in levels],
           f"storeys {[(s.LongName, s.Elevation) for s in storeys]}")

    for cls, code in CODES.items():
        for o in f.by_type(cls):
            ensure(re.fullmatch(rf"MM-{code}(-[^\s-]+)?", o.Name or "") is not None, f"{cls} named {o.Name!r}")
    # GlobalIds: the definition's ids, compressed.
    by_guid = {ifcopenshell.guid.compress(uuid.UUID(u).hex): e for e, u in zip(ents, sb.uids)}
    for cls in ("IfcWall", "IfcDoor", "IfcWindow", "IfcSpace", "IfcSlab", "IfcColumn", "IfcBeam", "IfcStair"):
        for o in f.by_type(cls):
            ensure(o.GlobalId in by_guid, f"{o.Name}: GlobalId {o.GlobalId} is no id of the drawing")

    # Structure.
    for cls in ("IfcWall", "IfcDoor", "IfcWindow", "IfcSlab", "IfcColumn", "IfcBeam", "IfcStair"):
        for o in f.by_type(cls):
            c = ue.get_container(o)
            ensure(c is not None and c.is_a("IfcBuildingStorey"), f"{o.Name}: on no storey")
    for s in f.by_type("IfcSpace"):
        parent = ue.get_aggregate(s)
        ensure(parent is not None and parent.is_a("IfcBuildingStorey"), f"{s.Name}: part of no storey")
    outer = {by_guid[w.GlobalId]["id"] for w in f.by_type("IfcWall")
             if by_guid[w.GlobalId].get("style") == "kentos-dis-duvar-25"}
    walls_ext = {}
    for w in f.by_type("IfcWall"):
        ext = ue.get_pset(w, "Pset_WallCommon", "IsExternal")
        walls_ext[w.id()] = ext
        ensure(ext == (by_guid[w.GlobalId]["id"] in outer), f"{w.Name} {by_guid[w.GlobalId]['id']}: IsExternal {ext}")
    for o in f.by_type("IfcOpeningElement"):
        voids = o.VoidsElements
        ensure(len(voids) == 1 and voids[0].RelatingBuildingElement.is_a("IfcWall"), f"{o.Name}: voids no wall")
    numbers = {(by_guid[x.GlobalId]["number"]) for x in f.by_type("IfcDoor") + f.by_type("IfcWindow")}
    ensure(numbers == {o["number"] for o in openings}, f"marks {sorted(numbers)}")
    for x in f.by_type("IfcDoor") + f.by_type("IfcWindow"):
        fills = x.FillsVoids
        ensure(len(fills) == 1, f"{x.Name}: fills {len(fills)} openings")
        if not fills:
            continue
        wall = fills[0].RelatingOpeningElement.VoidsElements[0].RelatingBuildingElement
        host = by_guid[x.GlobalId]["host"]
        ensure(ids[by_guid[wall.GlobalId]["id"]] == host, f"{x.Name}: in another wall")
        ensure(x.Tag == by_guid[x.GlobalId]["number"], f"{x.Name}: tag {x.Tag}")
        pset = "Pset_DoorCommon" if x.is_a("IfcDoor") else "Pset_WindowCommon"
        ensure(ue.get_pset(x, pset, "IsExternal") == walls_ext[wall.id()], f"{x.Name}: IsExternal not its wall's")

    # Rooms' areas.
    def area(pts):
        return abs(sum(a["x"] * b["y"] - b["x"] * a["y"] for a, b in zip(pts, pts[1:] + pts[:1]))) / 2.0
    for s in f.by_type("IfcSpace"):
        room = by_guid[s.GlobalId]
        a = ue.get_pset(s, "Qto_SpaceBaseQuantities", "NetFloorArea")
        ensure(a is not None and math.isclose(a, area(room["parts"][0]["outer"]["pts"]), abs_tol=1e-9),
               f"{s.Name}: NetFloorArea {a}")
        ensure(s.LongName == room["name"], f"{s.Name}: long name {s.LongName}")

    # Storeys' and the building's quantities.
    level_of = {lv["name"]: lv["id"] for lv in levels}
    for st in storeys:
        rooms = [e for e in kind("space") if e["arch"]["level"] == level_of[st.LongName]]
        want_net = sum(area(r["parts"][0]["outer"]["pts"]) for r in rooms)
        got = ue.get_pset(st, "Qto_BuildingStoreyBaseQuantities", "NetFloorArea")
        ensure(got is not None and math.isclose(got, want_net, abs_tol=1e-9), f"{st.Name}: NetFloorArea {got}")
    bq = ue.get_pset(f.by_type("IfcBuilding")[0], "Qto_BuildingBaseQuantities") or {}
    all_net = sum(area(r["parts"][0]["outer"]["pts"]) for r in kind("space"))
    ensure(math.isclose(bq.get("NetFloorArea", -1), all_net, abs_tol=1e-9), f"building NetFloorArea {bq}")
    ensure(bq.get("Height") == 3.0, f"building Height {bq}")

    # Columns, slabs, materials.
    for c in f.by_type("IfcColumn"):
        ensure(ue.get_pset(c, "Pset_ColumnCommon", "LoadBearing") is True, f"{c.Name}: LoadBearing")
    for sl in f.by_type("IfcSlab"):
        terrace = by_guid[sl.GlobalId].get("role") == "terrace"
        ensure(ue.get_pset(sl, "Pset_SlabCommon", "IsExternal") is terrace, f"{sl.Name}: IsExternal")

    def made_of(o):
        m = ue.get_material(o)
        return m.Name if m is not None and m.is_a("IfcMaterial") else None
    for cls, want_m in (("IfcWall", None), ("IfcSlab", "Betonarme"), ("IfcColumn", "Betonarme"),
                        ("IfcBeam", "Betonarme")):
        for o in f.by_type(cls):
            if want_m is None:
                core = min(by_guid[o.GlobalId]["components"],
                           key=lambda c: (c.get("priority", 3), "thickness" in c))
                want_m = core["material"]
            ensure(made_of(o) == want_m, f"{o.Name}: material {made_of(o)}, the drawing {want_m}")
            if cls == "IfcWall":
                want_m = None

    # Shapes.
    settings = ifcopenshell.geom.settings()
    shaped = 0
    for cls in ("IfcWall", "IfcDoor", "IfcWindow", "IfcSpace", "IfcSlab", "IfcColumn", "IfcBeam", "IfcStair"):
        for o in f.by_type(cls):
            try:
                ifcopenshell.geom.create_shape(settings, o)
                shaped += 1
            except Exception as e:  # noqa: BLE001
                bad.append(f"{o.Name}: no shape ({e})")
    if bad:
        for b in bad[:30]:
            print("✗", b)
        return 1
    print(f"✓ {FILE.relative_to(ROOT)}: IFC4X3_ADD2, 0 validator findings; "
          + ", ".join(f"{k} {len(f.by_type(k))}" for k in ("IfcBuildingStorey", "IfcWall", "IfcDoor", "IfcWindow",
                                                           "IfcOpeningElement", "IfcSpace", "IfcSlab", "IfcColumn",
                                                           "IfcBeam", "IfcStair"))
          + f"; {shaped} shapes")
    return 0


if __name__ == "__main__":
    sys.exit(main())
