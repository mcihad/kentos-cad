#!/usr/bin/env python3
"""IFC-03's radial grid, read by another program (docs/adr/8169): IfcOpenShell opens the section building KentOS wrote
with a curved grid system beside it (the application test `a_curved_grid_system_is_a_radial_grid`: radial axes 4 to 7
from the east façade's middle, arcs E and F) and checks it from the file's own entities, without KentOS code.

- IfcOpenShell's validator (attributes, EXPRESS rules) finds nothing.
- One IfcGrid is RADIAL (IFC 4.3 IfcGridTypeEnum): its U axes are straight lines (IfcPolyline) that meet at one point,
  each at or past its end there, listed counter-clockwise round it, the whole fan less than a turn; its V axes are arcs
  (IfcTrimmedCurve on an IfcCircle) about that point, from the centre out; each arc's trimming points lie on its circle
  at the angles its parameters give (radians, the file's plane angle unit), and the arc they bound, in the sense the
  file gives, holds every U axis's direction; each U axis crosses each arc once.
- The other grid is the section building's own (section_building.py), RECTANGULAR, its straight axes polylines.

    KENTOS_WRITE_IFC_RADIAL=/tmp/egrisel-aks.ifc cargo test -p kentos-native-application --test all a_curved_grid
    uv run --offline --no-project --python 3.11 --with ifcopenshell==0.9.0.post1 --with lark --with pytest \\
        python -I scripts/fixtures/radial_grid_check.py /tmp/egrisel-aks.ifc
"""

import math
import sys
from pathlib import Path

import ifcopenshell
import ifcopenshell.validate

sys.path.insert(0, str(Path(__file__).resolve().parent))
import section_building as sb  # noqa: E402

#: How near two points must be to be one, metres: the project's spatial tolerance.
SAME_POINT = 0.001
TAU = 2 * math.pi


def xy(point):
    return (float(point.Coordinates[0]), float(point.Coordinates[1]))


def main() -> int:
    path = Path(sys.argv[1]).resolve()
    f = ifcopenshell.open(str(path))
    bad: list[str] = []

    def ensure(ok, what):
        if not ok:
            bad.append(what)

    log = ifcopenshell.validate.json_logger()
    ifcopenshell.validate.validate(f, log, express_rules=True)
    ensure(not log.statements, f"validator: {[s.get('message', s) for s in log.statements[:5]]}")
    units = [u for u in f.by_type("IfcSIUnit") if u.UnitType == "PLANEANGLEUNIT"]
    ensure([u.Name for u in units] == ["RADIAN"], f"plane angle unit {[u.Name for u in units]}")

    grids = {g.PredefinedType: g for g in f.by_type("IfcGrid")}
    ensure(sorted(grids) == ["RADIAL", "RECTANGULAR"], f"grids {sorted(grids)}")
    radial, own = grids.get("RADIAL"), grids.get("RECTANGULAR")

    if own is not None:
        want = [(e["name"], [(p["x"], p["y"]) for p in e["pts"][:2]]) for e in sb.d["entities"] if e["kind"] == "grid"]
        got = []
        for a in list(own.UAxes) + list(own.VAxes):
            c = a.AxisCurve
            ensure(c.is_a("IfcPolyline") and len(c.Points) == 2, f"{a.AxisTag}: {c.is_a()}")
            if c.is_a("IfcPolyline"):
                got.append((a.AxisTag, [xy(p) for p in c.Points]))
        ensure(sorted(got) == sorted(want), f"the building's axes {got}")

    if radial is not None:
        rays = []
        for a in radial.UAxes:
            c = a.AxisCurve
            ensure(c.is_a("IfcPolyline") and len(c.Points) == 2, f"U {a.AxisTag}: {c.is_a()}")
            if c.is_a("IfcPolyline"):
                rays.append((a.AxisTag, xy(c.Points[0]), xy(c.Points[-1])))
        # The point the first two lines meet at.
        (_, p, q), (_, r, s) = rays[0], rays[1]
        d1, d2 = (q[0] - p[0], q[1] - p[1]), (s[0] - r[0], s[1] - r[1])
        den = d1[0] * d2[1] - d1[1] * d2[0]
        t = ((r[0] - p[0]) * d2[1] - (r[1] - p[1]) * d2[0]) / den
        centre = (p[0] + t * d1[0], p[1] + t * d1[1])
        angles = []
        for tag, p, q in rays:
            n = math.dist(p, q)
            u = ((q[0] - p[0]) / n, (q[1] - p[1]) / n)
            w = (centre[0] - p[0], centre[1] - p[1])
            across, along = abs(u[0] * w[1] - u[1] * w[0]), u[0] * w[0] + u[1] * w[1]
            ensure(across <= SAME_POINT, f"U {tag}: {across} m off the centre")
            ensure(along <= SAME_POINT or along >= n - SAME_POINT, f"U {tag}: the centre inside it")
            far = q if math.dist(q, centre) > math.dist(p, centre) else p
            angles.append(math.atan2(far[1] - centre[1], far[0] - centre[0]))
        turns = [(b - a) % TAU for a, b in zip(angles, angles[1:])]
        ensure(all(x > 0 for x in turns) and sum(turns) < TAU, f"U not counter-clockwise: {angles}")
        radii = []
        for a in radial.VAxes:
            c = a.AxisCurve
            ensure(c.is_a("IfcTrimmedCurve") and c.BasisCurve.is_a("IfcCircle"), f"V {a.AxisTag}: {c.is_a()}")
            if not (c.is_a("IfcTrimmedCurve") and c.BasisCurve.is_a("IfcCircle")):
                continue
            circle = c.BasisCurve
            o, rad = xy(circle.Position.Location), float(circle.Radius)
            ensure(math.dist(o, centre) <= SAME_POINT, f"V {a.AxisTag}: centre {o}, the axes' {centre}")
            ensure(c.MasterRepresentation == "CARTESIAN", f"V {a.AxisTag}: master {c.MasterRepresentation}")
            ends = []
            for trim in (c.Trim1, c.Trim2):
                pts = [x for x in trim if hasattr(x, "is_a") and x.is_a("IfcCartesianPoint")]
                params = [float(x.wrappedValue) for x in trim if hasattr(x, "is_a") and x.is_a("IfcParameterValue")]
                ensure(len(pts) == 1 and len(params) == 1, f"V {a.AxisTag}: trimmed by {trim}")
                if pts and params:
                    pt = xy(pts[0])
                    ensure(abs(math.dist(pt, o) - rad) <= 1e-6, f"V {a.AxisTag}: {pt} off its circle")
                    at = math.atan2(pt[1] - o[1], pt[0] - o[0]) % TAU
                    off = abs((at - params[0] + math.pi) % TAU - math.pi)
                    ensure(off <= 1e-9, f"V {a.AxisTag}: its parameter {params[0]}, its point at {at}")
                    ends.append(at)
            if len(ends) == 2:
                start, end = ends if c.SenseAgreement else ends[::-1]
                span = (end - start) % TAU
                for x in angles:
                    ensure((x - start) % TAU <= span, f"V {a.AxisTag}: the arc misses the U axis at {math.degrees(x)}°")
            for tag, p, q in rays:
                near, far = sorted((math.dist(p, centre), math.dist(q, centre)))
                ensure(near < rad < far, f"U {tag} does not cross V {a.AxisTag}")
            radii.append(rad)
        ensure(radii == sorted(radii) and len(set(radii)) == len(radii), f"V not from the centre out: {radii}")
        print(f"RADIAL: U {[t for t, _, _ in rays]} round ({centre[0]:.3f}, {centre[1]:.3f}) at "
              f"{[round(math.degrees(x), 3) for x in angles]}°; V {[a.AxisTag for a in radial.VAxes]} radii {radii}")

    if bad:
        for b in bad[:30]:
            print("✗", b)
        return 1
    print(f"✓ {path.name}: 0 validator findings; the radial grid and the building's own as drawn")
    return 0


if __name__ == "__main__":
    sys.exit(main())
