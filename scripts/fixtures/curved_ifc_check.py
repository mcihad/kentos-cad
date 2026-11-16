#!/usr/bin/env python3
"""The curved building's IFC 4.3 (docs/adr/8071; plan/bim-plan.md BIM-10) read by IfcOpenShell against the closed forms
curved_building.py works out without KentOS code (fixtures/arch/v1/curved-building.json).

    KENTOS_WRITE_IFC_CURVED=1 cargo test -p kentos-native-application --test all the_curved_building
    uv run --no-project --python 3.11 --with ifcopenshell==0.9.0.post1 --with pytest --with lark \\
        python -I scripts/fixtures/curved_ifc_check.py [FILE]

It checks fixtures/arch/v1/ifc/egri-bina.ifc (Design Transfer: bodies whole, openings 'Body') and egri-bina-referans.ifc
(Reference View: bodies already cut, openings 'Reference', docs/adr/8071 §4), or FILE alone, by its header:
- the header's view definition is what the file holds: every opening's shape 'Body' in a Design Transfer View,
  'Reference' only in a Reference View;
- the schema is IFC 4.3 and IfcOpenShell's validator (attributes, EXPRESS rules) finds nothing;
- every wall's Qto_WallBaseQuantities and every slab's Qto_SlabBaseQuantities is its closed form (1e-9 relative);
- each room's Qto_SpaceBaseQuantities.NetFloorArea is its closed form;
- the spiral stair is an IfcStair SPIRAL_STAIR whose every tread's profile has its two arcs and its post's its two half
  circles;
- the curved walls' and the slabs' profiles are indexed curves with arc segments (IfcArcIndex), the window through each
  curved wall too: arcs written as arcs (a Reference View's cut wall has them in every band);
- IfcOpenShell's own geometry of every wall, slab, room and the stair is a closed, oriented surface (each edge of its triangles
  met as often one way as the other), standing where the closed form puts it (its lowest and highest point over ±0.00), and encloses the closed
  form's volume less what its triangles leave of its arcs (IfcOpenShell meshes them at a 0.005 rad step; within
  0.05 %): a wall's its net volume, the openings taken out by IfcOpenShell's own booleans in a Design Transfer View and
  by KentOS's cut in a Reference View.

And fixtures/arch/v1/ifc/egimli-doseme.ifc (docs/adr/8071 §6; KENTOS_WRITE_IFC_SLOPED=1 cargo test … a_sloped_slab): the
section building with a 4 × 3 m slab 20 cm thick rising 30 cm over 3 m northward from 3.00 m: validator 0 findings; its
Pset_SlabCommon.PitchAngle atan 0.1; its Qto 0.2 m wide, 12 · √1.01 m² as it lies, 12 · 0.2 · √1.01 m³; IfcOpenShell's
own surface of it closed, its volume the same (its faces plane: 1e-9), its lowest point 3.00 − 0.2 · √1.01 (its
underside at its tail), its highest 3.30.

And fixtures/arch/v1/ifc/katmanli-cati.ifc (docs/adr/8071 §8; KENTOS_WRITE_IFC_ROOF_LAYERS=1 cargo test … a_layered_roof):
the section building's hip roof over its 1. kat — its eaves 50 cm out of the walls' outer faces (the 12 × 9 m grid's
25 cm walls: 13.25 × 10.25 m), 33 % on every edge, at +6.00 — with five layers top down: validator 0 findings; the
roof's material one IfcMaterialLayerSet, each layer its name, thickness across its face, material (none where not
given) and role (IfcMaterialLayer.Category's own words); IfcOpenShell's own surface of its body closed, enclosing
13.25 · 10.25 · 0.26 · √(1 + 0.33²) m³ (each face's plates its plan area times their upright thickness), its lowest
point 6.00 − 0.26 · √(1 + 0.33²) (the eaves' underside), its highest 6.00 + 0.33 · 10.25 / 2 (the ridge).

And fixtures/arch/v1/ifc/yerel-sistem.ifc (docs/adr/8072 §3; KENTOS_WRITE_IFC_LOCATION=1 cargo test … ifc_place): the
section building in a local system fitted from three control points onto TUREF / TM36, against
fixtures/arch/v1/ifc/yerel-sistem.json (scripts/fixtures/location_cases.py, no KentOS code): `check_location`.

Every file's placement chain (`chain_faults`): each product placed relative to one of its own — the structure that holds
it, the whole it is part of, the element its opening cuts, the opening it fills — the site at the chain's root, no loop."""
import json
import math
import sys
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FILES = [ROOT / "fixtures/arch/v1/ifc/egri-bina.ifc", ROOT / "fixtures/arch/v1/ifc/egri-bina-referans.ifc"]
SLOPED = ROOT / "fixtures/arch/v1/ifc/egimli-doseme.ifc"
ROOF_LAYERS = ROOT / "fixtures/arch/v1/ifc/katmanli-cati.ifc"
UNDER_ROOF = ROOT / "fixtures/arch/v1/ifc/cati-alti-mahal.ifc"
BLOCKS = (ROOT / "fixtures/arch/v1/ifc/blok-a.ifc", ROOT / "fixtures/arch/v1/ifc/blok-b.ifc")
LOCATION = (ROOT / "fixtures/arch/v1/ifc/yerel-sistem.ifc", ROOT / "fixtures/arch/v1/ifc/yerel-sistem.json")
EXPECTED = ROOT / "fixtures/arch/v1/curved-building.json"


def main() -> int:
    if len(sys.argv) > 1:
        path = Path(sys.argv[1]).resolve()
        if path.name == SLOPED.name:
            return check_sloped(path)
        if path.name == UNDER_ROOF.name:
            return check_under_roof(path)
        if path.name in (BLOCKS[0].name, BLOCKS[1].name):
            return check_blocks(*BLOCKS)
        if path.name == LOCATION[0].name:
            return check_location(*LOCATION)
        return check_roof_layers(path) if path.name == ROOF_LAYERS.name else check(path)
    return max([check(p) for p in FILES]
               + [check_sloped(SLOPED), check_roof_layers(ROOF_LAYERS), check_under_roof(UNDER_ROOF),
                  check_blocks(*BLOCKS), check_location(*LOCATION)])


def chain_faults(f) -> list[str]:
    """The placement chain (docs/adr/8072 §3; IFC-05: saha → bina → kat → eleman, no loop, no second offset): every
    product placed relative to one of its own — the structure that holds or references it, the whole it is part of,
    the element its opening cuts, the opening it fills, the one it is nested in — up to the site, which stands on
    nothing; no chain loops."""
    owners: dict[int, list] = {}
    for p in f.by_type("IfcProduct"):
        if p.ObjectPlacement is not None:
            owners.setdefault(p.ObjectPlacement.id(), []).append(p)
    parents: dict[int, set] = {}

    def add(child, parent):
        if parent is not None and parent.is_a("IfcProduct"):
            parents.setdefault(child.id(), set()).add(parent.id())

    for r in f.by_type("IfcRelContainedInSpatialStructure"):
        for e in r.RelatedElements:
            add(e, r.RelatingStructure)
    for r in f.by_type("IfcRelReferencedInSpatialStructure"):
        for e in r.RelatedElements:
            add(e, r.RelatingStructure)
    for r in f.by_type("IfcRelAggregates") + f.by_type("IfcRelNests"):
        for e in r.RelatedObjects:
            add(e, r.RelatingObject)
    for r in f.by_type("IfcRelVoidsElement"):
        add(r.RelatedOpeningElement, r.RelatingBuildingElement)
    for r in f.by_type("IfcRelFillsElement"):
        add(r.RelatedBuildingElement, r.RelatingOpeningElement)

    def ancestors(pid: int) -> set:
        out, todo = set(), [pid]
        while todo:
            for q in parents.get(todo.pop(), ()):
                if q not in out:
                    out.add(q)
                    todo.append(q)
        return out

    bad = []
    for p in f.by_type("IfcProduct"):
        place = p.ObjectPlacement
        if place is None:
            continue
        seen, q = set(), place
        while q is not None:
            if q.id() in seen:
                bad.append(f"{p.is_a()} {p.Name}: its placement chain loops")
                break
            seen.add(q.id())
            q = q.PlacementRelTo if q.is_a("IfcLocalPlacement") else None
        rel = place.PlacementRelTo if place.is_a("IfcLocalPlacement") else None
        if p.is_a("IfcSite"):
            if rel is not None:
                bad.append(f"the site {p.Name} is placed relative to something")
            continue
        if rel is None:
            bad.append(f"{p.is_a()} {p.Name}: placed on its own, not relative to what holds it")
            continue
        mine = ancestors(p.id())
        if not any(o.id() in mine for o in owners.get(rel.id(), [])):
            names = [f"{o.is_a()} {o.Name}" for o in owners.get(rel.id(), [])] or ["nothing's"]
            bad.append(f"{p.is_a()} {p.Name}: placed relative to {', '.join(names)}, not one of its own")
    return bad


def check_location(path: Path, case_path: Path) -> int:
    """The section building in a local system on TUREF / TM36 (docs/adr/8072 §3; IFC-05's first acceptance), against the
    case scripts/fixtures/location_cases.py writes without KentOS code. Read by IfcOpenShell: valid; one system,
    EPSG:5256 named in full in metres; one map conversion, IFC 4.3's scaled one, from the model's context: the fitted
    plane's origin, turn and scale on both axes (the case's, within its tolerances), its Scale 1 (the units the same),
    the heights unscaled, its height ±0.00's; every control point carried into the map by IfcOpenShell's own `xyz2enh`
    lands on its survey within the fit's own residual (to the case's 1e-6 m), and back by `enh2xyz` on its local point;
    eastings the east (TM36's 500 km false easting and a little), northings the north (not swapped); the storeys'
    heights in the map ±0.00's and 3 m over it by their placements' chain; every wall's lowest point on its storey's
    structural slab, under the floor's layers (no second offset); the placement chain whole."""
    import ifcopenshell
    import ifcopenshell.util.geolocation as geo
    import ifcopenshell.util.placement as up
    import ifcopenshell.validate

    case = json.loads(case_path.read_text(encoding="utf-8"))
    tol, plane, datum = case["tolerance"], case["plane"], case["datum"]
    f = ifcopenshell.open(str(path))
    bad: list[str] = []
    log = ifcopenshell.validate.json_logger()
    ifcopenshell.validate.validate(f, log, express_rules=True)
    if log.statements:
        bad.append(f"validator: {[s.get('message', s) for s in log.statements[:5]]}")
    systems, operations = f.by_type("IfcProjectedCRS"), f.by_type("IfcCoordinateOperation")
    if len(systems) != 1 or len(operations) != 1:
        bad.append(f"{len(systems)} systems and {len(operations)} conversions, not one each")
        return report(path, bad, "")
    crs, m = systems[0], operations[0]
    if (crs.Name, crs.GeodeticDatum, crs.MapZone, crs.VerticalDatum) != (
            case["base"]["name"], "TUREF", case["base"]["zone"], case["verticalDatum"]):
        bad.append(f"the system {crs.Name} {crs.GeodeticDatum} {crs.MapZone} {crs.VerticalDatum}")
    if crs.MapUnit is None or crs.MapUnit.Name != "METRE":
        bad.append(f"the map's unit {crs.MapUnit}")
    if not m.is_a("IfcMapConversionScaled") or m.TargetCRS != crs:
        bad.append(f"the conversion is {m.is_a()} to {m.TargetCRS}")
    if not (m.SourceCRS.is_a("IfcGeometricRepresentationContext") and m.SourceCRS.ContextType == "Model"):
        bad.append("the conversion is not the model context's")
    turn = math.degrees(math.atan2(m.XAxisOrdinate, m.XAxisAbscissa))
    if abs(m.Eastings - plane["east"]) > tol["metres"] or abs(m.Northings - plane["north"]) > tol["metres"]:
        bad.append(f"origin {m.Eastings} {m.Northings}, the fit's {plane['east']} {plane['north']}")
    if abs(turn - plane["rotation"]) > tol["degrees"]:
        bad.append(f"turn {turn}°, the fit's {plane['rotation']}°")
    if (m.Scale if m.Scale is not None else 1.0) != 1.0:
        bad.append(f"Scale {m.Scale}: it converts units, and the units are the same")
    for name, factor in (("FactorX", m.FactorX), ("FactorY", m.FactorY)):
        if abs(factor - plane["scale"]) > tol["relative"] * plane["scale"]:
            bad.append(f"{name} {factor}, the fit's scale {plane['scale']}")
    if m.FactorZ != 1.0 or abs(m.OrthogonalHeight - datum) > 1e-9:
        bad.append(f"heights scaled {m.FactorZ} from {m.OrthogonalHeight}, not unscaled from ±0.00's {datum}")
    h = geo.get_helmert_transformation_parameters(f)
    args = (h.e, h.n, h.h, h.xaa, h.xao, h.scale, h.factor_x, h.factor_y, h.factor_z)
    worst = 0.0
    for p in case["points"]:
        (x, y), (e0, n0), res = p["local"], p["map"], p["residual"]
        e, n, z = geo.xyz2enh(x, y, 0.0, *args)
        de, dn = e - e0, n - n0
        worst = max(worst, math.hypot(de, dn))
        if abs(de - res[0]) > tol["metres"] or abs(dn - res[1]) > tol["metres"] or abs(z - datum) > 1e-9:
            bad.append(f"{p['name']} lands {de:+.6f} {dn:+.6f} off its survey, the fit's residual {res[0]:+.6f} {res[1]:+.6f}")
        bx, by, _ = geo.enh2xyz(e0, n0, datum, *args)
        if math.hypot(bx - x, by - y) > res[2] / plane["scale"] + tol["metres"]:
            bad.append(f"{p['name']} back at {bx} {by}, not {x} {y}")
    if not (400_000 < h.e < 700_000 and 4_000_000 < h.n < 4_700_000):
        bad.append(f"eastings {h.e} and northings {h.n}: axes swapped or off TM36")
    want = {s["name"]: datum + s["elevation"] for s in case["storeys"]}
    got = {}
    for storey in f.by_type("IfcBuildingStorey"):
        z = up.get_local_placement(storey.ObjectPlacement)[2][3]
        got[storey.LongName] = geo.xyz2enh(0.0, 0.0, z, *args)[2]
    if set(got) != set(want) or any(abs(got[k] - want[k]) > 1e-9 for k in want):
        bad.append(f"storeys in the map {got}, not {want}")
    walls_from = {s["name"]: s["wallsFrom"] for s in case["storeys"]}
    storey_of = {}
    for r in f.by_type("IfcRelContainedInSpatialStructure"):
        if r.RelatingStructure.is_a("IfcBuildingStorey"):
            for e in r.RelatedElements:
                storey_of[e.id()] = r.RelatingStructure.LongName
    for wall in f.by_type("IfcWall"):
        pts, _, _ = surface(f, wall)
        low, want_low = min(q[2] for q in pts), walls_from.get(storey_of.get(wall.id()))
        if want_low is None or abs(low - want_low) > 1e-6:
            bad.append(f"{wall.Name} stands at {low}, not on its storey's slab at {want_low}")
    bad += chain_faults(f)
    return report(path, bad, f"EPSG:5256 TUREF/TM36 in metres; IfcMapConversionScaled, the fitted plane (turn "
                             f"{turn:.6f}°, scale {m.FactorX:.9f} on both axes, heights unscaled from {datum:.2f}); "
                             f"{len(case['points'])} control points on their surveys within their residuals "
                             f"(at most {worst * 1000:.2f} mm) and back; storeys at {sorted(want.values())} in the map; "
                             f"walls on their storeys' slabs; the placement chain whole")


def report(path: Path, bad: list[str], ok: str) -> int:
    if bad:
        for b in bad:
            print("✗", path.name, b)
        return 1
    print(f"✓ {path.name}: IFC4X3_ADD2, 0 validator findings; {ok}.")
    return 0


def surface(f, product):
    """IfcOpenShell's own triangles of a product in world coordinates: its points welded at 1e-9 and its triangles."""
    import ifcopenshell.geom

    settings = ifcopenshell.geom.settings()
    for key, value in (("use-world-coords", True), ("weld-vertices", True)):
        try:
            settings.set(key, value)
        except Exception:  # noqa: BLE001
            pass
    g = ifcopenshell.geom.create_shape(settings, product).geometry
    v = g.verts
    pts = [(v[i], v[i + 1], v[i + 2]) for i in range(0, len(v), 3)]
    key = {}
    index = [key.setdefault(tuple(round(c, 9) for c in p), len(key)) for p in pts]
    tris = [(index[g.faces[i]], index[g.faces[i + 1]], index[g.faces[i + 2]]) for i in range(0, len(g.faces), 3)]
    place = {}
    for p, i in zip(pts, index):
        place.setdefault(i, p)
    return pts, place, tris


def closed_volume(place, tris):
    """Whether the triangles close (each directed edge met as often the other way) and the volume they enclose."""
    edges = {}
    for t in tris:
        for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])):
            edges[(a, b)] = edges.get((a, b), 0) + 1
    closed = all(edges.get((b, a), 0) == n for (a, b), n in edges.items())
    vol = abs(sum((a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                   + a[2] * (b[0] * c[1] - b[1] * c[0])) / 6.0
                  for a, b, c in ((place[x], place[y], place[z]) for x, y, z in tris)))
    return closed, vol


def check_blocks(a_path: Path, b_path: Path) -> int:
    """Two blocks, one model each (docs/adr/8072 §2): A Blok over a basement both share, B Blok 20 m east entered 1.20 m
    higher. Read by IfcOpenShell: both valid; one site and one project (the same GlobalIds and names) in both and no
    other GlobalId shared; each its own building; the storeys' heights over ±0.00 by their placements' chain: A's −3.00, ±0.00, +3.00, B's +1.20 (its
    Elevation 0 over its own ±0.00); the basement's slab in A's file only; B's walls standing at +1.20."""
    import ifcopenshell
    import ifcopenshell.util.placement as up
    import ifcopenshell.validate

    bad: list[str] = []
    files = {}
    for tag, path in (("A", a_path), ("B", b_path)):
        f = ifcopenshell.open(str(path))
        log = ifcopenshell.validate.json_logger()
        ifcopenshell.validate.validate(f, log, express_rules=True)
        if log.statements:
            bad.append(f"{tag}: validator: {[s.get('message', s) for s in log.statements[:5]]}")
        files[tag] = f
    one = lambda f, cls: f.by_type(cls)[0]
    for cls in ("IfcSite", "IfcProject"):
        a, b = one(files["A"], cls), one(files["B"], cls)
        if (a.GlobalId, a.Name) != (b.GlobalId, b.Name):
            bad.append(f"{cls} differs: {a.GlobalId} {a.Name} / {b.GlobalId} {b.Name}")
    # Read together, the two files share their project and their site, nothing else (a federated model).
    ids = {tag: {e.GlobalId for e in f.by_type("IfcRoot")} for tag, f in files.items()}
    shared = ids["A"] & ids["B"]
    want = {one(files["A"], "IfcProject").GlobalId, one(files["A"], "IfcSite").GlobalId}
    if shared != want:
        bad.append(f"{len(shared)} GlobalIds in both files, not the project's and the site's alone")
    ba, bb = one(files["A"], "IfcBuilding"), one(files["B"], "IfcBuilding")
    if ba.GlobalId == bb.GlobalId or ba.Name == bb.Name:
        bad.append(f"the buildings are not their own: {ba.Name} / {bb.Name}")
    near = lambda x, y: abs(x - y) <= 1e-9
    heights = {tag: sorted(round(up.get_local_placement(s.ObjectPlacement)[2][3], 9) for s in f.by_type("IfcBuildingStorey"))
               for tag, f in files.items()}
    if heights["A"] != [-3.0, 0.0, 3.0] or heights["B"] != [1.2]:
        bad.append(f"storeys over ±0.00: {heights}")
    b_storey = one(files["B"], "IfcBuildingStorey")
    if not near(b_storey.Elevation, 0.0):
        bad.append(f"B's storey Elevation {b_storey.Elevation}, not 0 over its own ±0.00")
    slabs = {tag: len(f.by_type("IfcSlab")) for tag, f in files.items()}
    if slabs != {"A": 4, "B": 0}:
        bad.append(f"slabs {slabs}: the basement's once, in A")
    for wall in files["B"].by_type("IfcWall"):
        pts, _, _ = surface(files["B"], wall)
        low = min(p[2] for p in pts)
        if not near(low, 1.2):
            bad.append(f"{wall.Name} stands at {low}, not +1.20")
    for tag, f in files.items():
        bad += [f"{tag}: {b}" for b in chain_faults(f)]
    if bad:
        for b in bad:
            print("✗", b)
        return 1
    print(f"✓ {a_path.name} + {b_path.name}: IFC4X3_ADD2, 0 validator findings; one site and one project, two buildings, "
          "storeys at −3.00/±0.00/+3.00 and +1.20 by their placements, the basement once, B's walls at +1.20.")
    return 0


def check_under_roof(path: Path) -> int:
    """The section building without its terrace slab, its hip roof over the 1. kat (eaves 50 cm out of the 12 × 9 m
    grid's 25 cm walls at +6.00, 33 %, 25 cm upright): each 1. kat room rises to the roof's underside. Its GrossVolume
    is the underside (the roof's lower envelope, 33 % in from each eaves edge, less 0.25) over its floor integrated here
    on a grid over the room's outline as IfcOpenShell meshes it; NetVolume the same; no Height (not constant);
    IfcOpenShell's own surface of it closed, enclosing its GrossVolume."""
    import ifcopenshell
    import ifcopenshell.util.element as ue
    import ifcopenshell.validate

    f = ifcopenshell.open(str(path))
    bad: list[str] = []
    log = ifcopenshell.validate.json_logger()
    ifcopenshell.validate.validate(f, log, express_rules=True)
    if log.statements:
        bad.append(f"validator: {[s.get('message', s) for s in log.statements[:5]]}")
    near = lambda a, b, rel: a is not None and abs(a - b) <= rel * max(abs(b), 1.0)
    face = 0.25 / 2
    x0, y0, x1, y1 = -face - 0.5, -face - 0.5, 12 + face + 0.5, 9 + face + 0.5
    under = lambda x, y: 6.0 - 0.25 + 0.33 * min(x - x0, x1 - x, y - y0, y1 - y)
    rooms = 0
    for space in f.by_type("IfcSpace"):
        storey = ue.get_aggregate(space)
        if round(storey.Elevation, 6) != 3.0:
            continue
        rooms += 1
        q = ue.get_pset(space, "Qto_SpaceBaseQuantities") or {}
        pts, place, tris = surface(f, space)
        closed, vol = closed_volume(place, tris)
        if not closed:
            bad.append(f"{space.Name}: IfcOpenShell's surface of it is not closed")
        xs, ys, zs = [p[0] for p in pts], [p[1] for p in pts], [p[2] for p in pts]
        floor = min(zs)
        n = 600
        dx, dy = (max(xs) - min(xs)) / n, (max(ys) - min(ys)) / n
        want = sum((under(min(xs) + (i + 0.5) * dx, min(ys) + (j + 0.5) * dy) - floor) * dx * dy
                   for i in range(n) for j in range(n))
        if not near(q.get("GrossVolume"), want, 1e-5):
            bad.append(f"{space.Name}: GrossVolume {q.get('GrossVolume')}, the grid's {want}")
        if q.get("NetVolume") != q.get("GrossVolume"):
            bad.append(f"{space.Name}: NetVolume {q.get('NetVolume')}")
        if "Height" in q:
            bad.append(f"{space.Name}: a Height {q['Height']} for a room whose height is not constant")
        if not near(vol, q.get("GrossVolume") or 0.0, 1e-9):
            bad.append(f"{space.Name}: IfcOpenShell's volume {vol}, its GrossVolume {q.get('GrossVolume')}")
    if rooms != 5:
        bad.append(f"{rooms} rooms on the 1. kat, not 5")
    bad += chain_faults(f)
    if bad:
        for b in bad:
            print("✗", b)
        return 1
    shown = path.relative_to(ROOT) if path.is_relative_to(ROOT) else path.name
    print(f"✓ {shown}: IFC4X3_ADD2, 0 validator findings; the 1. kat's 5 rooms up to the roof, their volumes the grid's "
          "and IfcOpenShell's, no Height.")
    return 0


def check_roof_layers(path: Path) -> int:
    import math

    import ifcopenshell
    import ifcopenshell.util.element as ue
    import ifcopenshell.validate

    f = ifcopenshell.open(str(path))
    bad: list[str] = []
    log = ifcopenshell.validate.json_logger()
    ifcopenshell.validate.validate(f, log, express_rules=True)
    if log.statements:
        bad.append(f"validator: {[s.get('message', s) for s in log.statements[:5]]}")
    near = lambda a, b, rel=1e-9: a is not None and abs(a - b) <= rel * max(abs(b), 1.0)
    # The section building (section_building.py): a 12 × 9 m grid, its outer walls 25 cm on it; the eaves 50 cm out.
    face = 0.25 / 2
    width, depth = 12 + 2 * (face + 0.5), 9 + 2 * (face + 0.5)
    layers = [("Kaplama", 0.03, "Kiremit", "Outer finish"), ("Lata", 0.03, None, None),
              ("Kaplama tahtası", 0.025, "Çam", None), ("Isı yalıtımı", 0.05, None, "Insulation"),
              ("Mertek", 0.125, "Çam", "LoadBearing")]
    total = sum(t for _, t, _, _ in layers)
    k = math.sqrt(1 + 0.33 ** 2)
    roofs = f.by_type("IfcRoof")
    if len(roofs) != 1:
        bad.append(f"{len(roofs)} roofs")
    for roof in roofs:
        m = ue.get_material(roof)
        if m is None or not m.is_a("IfcMaterialLayerSet"):
            bad.append(f"its material {m}")
        else:
            got = [(l.Name, l.LayerThickness, l.Material.Name if l.Material else None, l.Category)
                   for l in m.MaterialLayers]
            if len(got) != len(layers) or any(g[0] != w[0] or not near(g[1], w[1]) or g[2:] != w[2:]
                                              for g, w in zip(got, layers)):
                bad.append(f"its layers {got}, not {layers}")
        pts, place, tris = surface(f, roof)
        closed, vol = closed_volume(place, tris)
        if not closed:
            bad.append("IfcOpenShell's surface of it is not closed")
        want = width * depth * total * k
        if not near(vol, want):
            bad.append(f"IfcOpenShell's volume {vol}, the closed form {want}")
        zs = [p[2] for p in pts]
        if not (near(min(zs), 6.0 - total * k) and near(max(zs), 6.0 + 0.33 * depth / 2)):
            bad.append(f"{min(zs)} → {max(zs)}")
    bad += chain_faults(f)
    if bad:
        for b in bad:
            print("✗", b)
        return 1
    shown = path.relative_to(ROOT) if path.is_relative_to(ROOT) else path.name
    print(f"✓ {shown}: IFC4X3_ADD2, 0 validator findings; the roof one material layer set of its five layers, "
          "IfcOpenShell's surface closed and its volume the closed form's.")
    return 0


def check_sloped(path: Path) -> int:
    import math

    import ifcopenshell
    import ifcopenshell.geom
    import ifcopenshell.util.element as ue
    import ifcopenshell.validate

    f = ifcopenshell.open(str(path))
    bad: list[str] = []
    log = ifcopenshell.validate.json_logger()
    ifcopenshell.validate.validate(f, log, express_rules=True)
    if log.statements:
        bad.append(f"validator: {[s.get('message', s) for s in log.statements[:5]]}")
    sloped = [s for s in f.by_type("IfcSlab") if (ue.get_pset(s, "Pset_SlabCommon") or {}).get("PitchAngle")]
    if len(sloped) != 1:
        bad.append(f"{len(sloped)} sloped slabs")
    k = math.sqrt(1.01)
    for s in sloped:
        near = lambda a, b: a is not None and abs(a - b) <= 1e-9 * max(abs(b), 1.0)
        pitch = ue.get_pset(s, "Pset_SlabCommon")["PitchAngle"]
        if not near(pitch, math.atan(0.1)):
            bad.append(f"PitchAngle {pitch}")
        q = ue.get_pset(s, "Qto_SlabBaseQuantities") or {}
        for key, want in (("Width", 0.2), ("GrossArea", 12 * k), ("NetArea", 12 * k), ("GrossVolume", 12 * 0.2 * k),
                          ("NetVolume", 12 * 0.2 * k)):
            if not near(q.get(key), want):
                bad.append(f"{key} {q.get(key)}, the closed form {want}")
        settings = ifcopenshell.geom.settings()
        for key, value in (("use-world-coords", True), ("weld-vertices", True)):
            try:
                settings.set(key, value)
            except Exception:  # noqa: BLE001
                pass
        g = ifcopenshell.geom.create_shape(settings, s).geometry
        v = g.verts
        pts = [(v[i], v[i + 1], v[i + 2]) for i in range(0, len(v), 3)]
        key = {}
        index = [key.setdefault(tuple(round(c, 9) for c in p), len(key)) for p in pts]
        tris = [(index[g.faces[i]], index[g.faces[i + 1]], index[g.faces[i + 2]]) for i in range(0, len(g.faces), 3)]
        edges = {}
        for t in tris:
            for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])):
                edges[(a, b)] = edges.get((a, b), 0) + 1
        if not all(edges.get((b, a), 0) == n for (a, b), n in edges.items()):
            bad.append("IfcOpenShell's surface of it is not closed")
        place = {}
        for p, i in zip(pts, index):
            place.setdefault(i, p)
        vol = abs(sum((a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                       + a[2] * (b[0] * c[1] - b[1] * c[0])) / 6.0
                      for a, b, c in ((place[x], place[y], place[z]) for x, y, z in tris)))
        if not near(vol, 12 * 0.2 * k):
            bad.append(f"IfcOpenShell's volume {vol}, the closed form {12 * 0.2 * k}")
        zs = [p[2] for p in pts]
        if not (near(min(zs), 3.0 - 0.2 * k) and near(max(zs), 3.3)):
            bad.append(f"{min(zs)} → {max(zs)}")
    bad += chain_faults(f)
    if bad:
        for b in bad:
            print("✗", b)
        return 1
    shown = path.relative_to(ROOT) if path.is_relative_to(ROOT) else path.name
    print(f"✓ {shown}: IFC4X3_ADD2, 0 validator findings; the sloped slab its closed forms, IfcOpenShell's surface "
          "closed and its volume the same.")
    return 0


def check(path: Path) -> int:
    import ifcopenshell
    import ifcopenshell.geom
    import ifcopenshell.guid
    import ifcopenshell.util.element as ue
    import ifcopenshell.validate

    f = ifcopenshell.open(str(path))
    want = json.loads(EXPECTED.read_text(encoding="utf-8"))
    bad: list[str] = []

    def ensure(ok: bool, what: str) -> None:
        if not ok:
            bad.append(what)

    log = ifcopenshell.validate.json_logger()
    ifcopenshell.validate.validate(f, log, express_rules=True)
    ensure(f.schema_identifier == "IFC4X3_ADD2", f"schema {f.schema_identifier}")
    ensure(not log.statements, f"validator: {[s.get('message', s) for s in log.statements[:5]]}")

    # The header's view and the openings' shapes.
    view = " ".join(f.header.file_description.description)
    reference = "ViewDefinition [ReferenceView]" in view
    ensure(reference or "ViewDefinition [DesignTransferView]" in view, f"header: {view}")
    for o in f.by_type("IfcOpeningElement"):
        ids = {r.RepresentationIdentifier for r in (o.Representation.Representations if o.Representation else [])}
        ensure(ids == ({"Reference"} if reference else {"Body"}), f"{o.Name}: shapes {sorted(ids)} in {view}")

    by_id = {}
    for o in f.by_type("IfcProduct"):
        try:
            by_id[str(uuid.UUID(hex=ifcopenshell.guid.expand(o.GlobalId)))] = o
        except ValueError:
            pass

    def near(a, b, rel=1e-9):
        return a is not None and abs(a - b) <= rel * max(abs(b), 1.0)

    # The quantities, as the file holds them.
    for group, qto in (("walls", "Qto_WallBaseQuantities"), ("slabs", "Qto_SlabBaseQuantities"),
                       ("spaces", "Qto_SpaceBaseQuantities")):
        for u, quantities in want[group].items():
            o = by_id.get(u)
            ensure(o is not None, f"{u}: not in the file")
            if o is None:
                continue
            got = ue.get_pset(o, qto) or {}
            for k, v in quantities.items():
                ensure(near(got.get(k), v), f"{o.Name} {qto}.{k}: {got.get(k)}, the closed form {v}")

    # Arcs written as arcs: the curved walls', the slabs' and the curved walls' windows' profiles.
    def arcs(product) -> int:
        n = 0
        for rep in (product.Representation.Representations if product.Representation else []):
            for item in rep.Items:
                if not item.is_a("IfcExtrudedAreaSolid"):
                    continue
                prof = item.SweptArea
                curves = [prof.OuterCurve] + list(getattr(prof, "InnerCurves", None) or [])
                for c in curves:
                    if c.is_a("IfcIndexedPolyCurve"):
                        n += sum(1 for s in (c.Segments or []) if s.is_a("IfcArcIndex"))
        return n

    curved_walls = [by_id[u] for u, q in want["walls"].items() if q["Length"] not in (10.0, 6.0)]
    ensure(len(curved_walls) == 2, f"curved walls {len(curved_walls)}")
    for w in curved_walls:
        n = arcs(w)
        ensure(n >= 2 if reference else n == 2, f"{w.Name}: {n} arc segments, its two faces' arcs")
        for rel in w.HasOpenings:
            o = rel.RelatedOpeningElement
            if o.HasFillings and o.HasFillings[0].RelatedBuildingElement.is_a("IfcWindow"):
                ensure(arcs(o) == 2, f"{o.Name} through {w.Name}: {arcs(o)} arc segments")
    for u in want["slabs"]:
        ensure(arcs(by_id[u]) >= 1, f"{by_id[u].Name}: its arc edge not an arc")
    for u in want["spaces"]:
        ensure(arcs(by_id[u]) == 1, f"{by_id[u].Name}: its arc edge not an arc")
    # The spiral stair (docs/adr/0709, 8071): its type, and each tread's two arcs and the post's two half circles arcs.
    for u, q in want["stairs"].items():
        o = by_id.get(u)
        ensure(o is not None and o.is_a("IfcStair") and o.PredefinedType == "SPIRAL_STAIR",
               f"{u}: {o.is_a() if o else None} {getattr(o, 'PredefinedType', None)}")
        if o is not None:
            n = arcs(o)
            ensure(n == 2 * len(q["treads"]) + 2, f"{o.Name}: {n} arc segments, each tread's two and the post's two")

    # IfcOpenShell's own geometry.
    settings = ifcopenshell.geom.settings()
    for key, value in (("use-world-coords", True), ("weld-vertices", True), ("mesher-angular-deflection", 0.005),
                       ("mesher-linear-deflection", 1e-5)):
        try:
            settings.set(key, value)
        except Exception:  # noqa: BLE001  (an older IfcOpenShell names them otherwise)
            pass

    def mesh(o):
        g = ifcopenshell.geom.create_shape(settings, o).geometry
        v = g.verts
        pts = [(v[i], v[i + 1], v[i + 2]) for i in range(0, len(v), 3)]
        # One vertex for each place (to 1e-9 m): IfcOpenShell's faces may each have their own.
        key = {}
        index = []
        for p in pts:
            k = tuple(round(c, 9) for c in p)
            index.append(key.setdefault(k, len(key)))
        tris = [(index[g.faces[i]], index[g.faces[i + 1]], index[g.faces[i + 2]]) for i in range(0, len(g.faces), 3)]
        tris = [t for t in tris if len(set(t)) == 3]
        return pts, index, tris

    def closed(tris) -> bool:
        # Closed and oriented: each edge met as often one way as the other (a body of two items, a slab and its
        # finish, meets their common face's edges twice each way).
        edges = {}
        for t in tris:
            for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])):
                edges[(a, b)] = edges.get((a, b), 0) + 1
        return all(edges.get((b, a), 0) == n for (a, b), n in edges.items())

    def volume(pts, index, tris) -> float:
        place = {}
        for p, i in zip(pts, index):
            place.setdefault(i, p)
        v = 0.0
        for a, b, c in tris:
            (ax, ay, az), (bx, by, bz), (cx, cy, cz) = place[a], place[b], place[c]
            v += (ax * (by * cz - bz * cy) - ay * (bx * cz - bz * cx) + az * (bx * cy - by * cx)) / 6.0
        return abs(v)

    checked = 0
    worst = 0.0
    for group, of in (("walls", "NetVolume"), ("slabs", "NetVolume"), ("spaces", None), ("stairs", "volume")):
        for u, q in want[group].items():
            o = by_id[u]
            try:
                pts, index, tris = mesh(o)
            except Exception as e:  # noqa: BLE001
                bad.append(f"{o.Name}: no shape ({e})")
                continue
            checked += 1
            ensure(closed(tris), f"{o.Name}: IfcOpenShell's surface of it is not closed")
            if u in want["bodies"]:
                low, high = want["bodies"][u]
                zs = [p[2] for p in pts]
                ensure(near(min(zs), low, 1e-9) and near(max(zs), high, 1e-9),
                       f"{o.Name}: {min(zs)} → {max(zs)}, not {low} → {high}")
            if of is None:
                # A room's volume: its floor area by its storey's height.
                storey = ue.get_aggregate(o)
                height = {0.0: 3.20, 3.35: 2.90}[round(storey.Elevation, 6)]
                expect = q["NetFloorArea"] * height
            else:
                expect = q[of]
            got = volume(pts, index, tris)
            worst = max(worst, abs(got - expect) / expect)
            ensure(abs(got - expect) <= 5e-4 * expect, f"{o.Name}: IfcOpenShell's volume {got}, the closed form {expect}")

    bad += chain_faults(f)
    if bad:
        for b in bad[:30]:
            print("✗", b)
        return 1
    shown = path.relative_to(ROOT) if path.is_relative_to(ROOT) else path.name
    print(f"✓ {shown} ({'Reference View' if reference else 'Design Transfer View'}): IFC4X3_ADD2, 0 validator findings; "
          f"{len(want['walls'])} walls, {len(want['slabs'])} slabs, {len(want['spaces'])} rooms and "
          f"{len(want['stairs'])} spiral stair their closed forms; {checked} closed surfaces of IfcOpenShell's own, their "
          f"volumes the closed forms' within {worst:.2e}.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
