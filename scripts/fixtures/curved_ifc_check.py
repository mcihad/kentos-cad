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
- the curved walls' and the slabs' profiles are indexed curves with arc segments (IfcArcIndex), the window through each
  curved wall too: arcs written as arcs (a Reference View's cut wall has them in every band);
- IfcOpenShell's own geometry of every wall, slab and room is a closed, oriented surface (each edge of its triangles
  met as often one way as the other), standing where the closed form puts it (its lowest and highest point over ±0.00), and encloses the closed
  form's volume less what its triangles leave of its arcs (IfcOpenShell meshes them at a 0.005 rad step; within
  0.05 %): a wall's its net volume, the openings taken out by IfcOpenShell's own booleans in a Design Transfer View and
  by KentOS's cut in a Reference View.

And fixtures/arch/v1/ifc/egimli-doseme.ifc (docs/adr/8071 §6; KENTOS_WRITE_IFC_SLOPED=1 cargo test … a_sloped_slab): the
section building with a 4 × 3 m slab 20 cm thick rising 30 cm over 3 m northward from 3.00 m: validator 0 findings; its
Pset_SlabCommon.PitchAngle atan 0.1; its Qto 0.2 m wide, 12 · √1.01 m² as it lies, 12 · 0.2 · √1.01 m³; IfcOpenShell's
own surface of it closed, its volume the same (its faces plane: 1e-9), its lowest point 3.00 − 0.2 · √1.01 (its
underside at its tail), its highest 3.30."""
import json
import sys
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FILES = [ROOT / "fixtures/arch/v1/ifc/egri-bina.ifc", ROOT / "fixtures/arch/v1/ifc/egri-bina-referans.ifc"]
SLOPED = ROOT / "fixtures/arch/v1/ifc/egimli-doseme.ifc"
EXPECTED = ROOT / "fixtures/arch/v1/curved-building.json"


def main() -> int:
    if len(sys.argv) > 1:
        path = Path(sys.argv[1]).resolve()
        return check_sloped(path) if path.name == SLOPED.name else check(path)
    return max([check(p) for p in FILES] + [check_sloped(SLOPED)])


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
    for group, of in (("walls", "NetVolume"), ("slabs", "NetVolume"), ("spaces", None)):
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

    if bad:
        for b in bad[:30]:
            print("✗", b)
        return 1
    shown = path.relative_to(ROOT) if path.is_relative_to(ROOT) else path.name
    print(f"✓ {shown} ({'Reference View' if reference else 'Design Transfer View'}): IFC4X3_ADD2, 0 validator findings; "
          f"{len(want['walls'])} walls, {len(want['slabs'])} slabs and "
          f"{len(want['spaces'])} rooms their closed forms; {checked} closed surfaces of IfcOpenShell's own, their "
          f"volumes the closed forms' within {worst:.2e}.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
