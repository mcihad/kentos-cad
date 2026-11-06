#!/usr/bin/env python3
"""A section's cut and fill moments and their volume on a curve, shared cases (docs/adr/0934 §1, plan/road.md Y30.1),
from an independent reference in exact fractions. No KentOS code, and not KentOS's way:

- the sections are road_template_cases.py's (exact); the cut and fill regions are assembled as polygons (the design
  line and the terrain between the points where their order changes) and each polygon's area and first moment about
  the alignment come from the shoelace formulas (`A = ½Σ(xᵢyᵢ₊₁ − xᵢ₊₁yᵢ)`, `∫x dA = ⅙Σ(xᵢ + xᵢ₊₁)(xᵢyᵢ₊₁ − xᵢ₊₁yᵢ)`),
  not by strips; their areas must equal the template reference's (checked here);
- a constant section swept along a circular arc of radius R through θ is the solid of revolution's sector: every point
  at distance r from the centre travels r·θ, so V = θ·∫ r dA with r = R + o on a left turn (centre on the left,
  offsets right positive) and r = R − o on a right one; a section reaching past the centre is no solid of revolution.

    python3 scripts/fixtures/road_curve_volume_cases.py           (--check: fails when the file differs)
"""

import json
import sys
from fractions import Fraction as F
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_template_cases as tref  # noqa: E402
import road_grades  # noqa: E402  (docs/adr/0977: the grades are written as KentOS holds them)

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "curve-volumes.json"


def terrain(ground):
    """The terrain as (offset, z) runs without gaps: a list of polylines."""
    runs, cur = [], []
    for p in ground:
        if p["gap"] and cur:
            runs.append(cur)
            cur = []
        if p["z"] is None:
            if cur:
                runs.append(cur)
            cur = []
            continue
        cur.append((F(p["offset"]), F(p["z"])))
    if cur:
        runs.append(cur)
    return runs


def value(poly, x):
    for (a, za), (b, zb) in zip(poly, poly[1:]):
        if a <= x <= b and b > a:
            return za + (zb - za) * (x - a) / (b - a)
    return None


def polygons(design, ground):
    """The cut and fill regions as polygons: over each terrain run, between the outermost design points, split where
    the design line and the terrain cross."""
    out = {"cut": [], "fill": []}
    lo, hi = design[0][0], design[-1][0]
    for run in terrain(ground):
        a, b = max(lo, run[0][0]), min(hi, run[-1][0])
        if not a < b:
            continue
        xs = sorted({x for x, _ in design if a <= x <= b} | {x for x, _ in run if a <= x <= b} | {a, b})
        # Where the order changes, exactly.
        stops = [xs[0]]
        for x0, x1 in zip(xs, xs[1:]):
            d0 = value(design, x0) - value(run, x0)
            d1 = value(design, x1) - value(run, x1)
            if d0 * d1 < 0:
                stops.append(x0 + (x1 - x0) * d0 / (d0 - d1))
            stops.append(x1)
        # Maximal stretches of one sign, each a polygon: the upper line forward, the lower back.
        i = 0
        while i < len(stops) - 1:
            mid = (stops[i] + stops[i + 1]) / 2
            sign = value(design, mid) - value(run, mid)
            if sign == 0:
                i += 1
                continue
            j = i + 1
            while j < len(stops) - 1:
                m2 = (stops[j] + stops[j + 1]) / 2
                s2 = value(design, m2) - value(run, m2)
                if (s2 > 0) != (sign > 0) or s2 == 0:
                    break
                j += 1
            seg = stops[i:j + 1]
            upper, lower = (design, run) if sign > 0 else (run, design)
            poly = [(x, value(upper, x)) for x in seg] + [(x, value(lower, x)) for x in reversed(seg)]
            out["fill" if sign > 0 else "cut"].append(poly)
            i = j
    return out


def shoelace(poly):
    a = q = F(0)
    n = len(poly)
    for k in range(n):
        (x0, y0), (x1, y1) = poly[k], poly[(k + 1) % n]
        cr = x0 * y1 - x1 * y0
        a += cr
        q += (x0 + x1) * cr
    # Clockwise or not, the area is its size and the moment follows its sign.
    if a < 0:
        a, q = -a, -q
    return a / 2, q / 6


def section(c):
    built_l = tref.build_side(c["template"]["left"], c["axis"], c["ground"], False)
    built_r = tref.build_side(c["template"]["right"], c["axis"], c["ground"], True)
    design = [tuple(p) for p in reversed(built_l["points"])] + [tuple(p) for p in built_r["points"][1:]]
    regions = polygons(design, c["ground"])
    out = {}
    for kind in ("cut", "fill"):
        a = q = F(0)
        for poly in regions[kind]:
            pa, pq = shoelace(poly)
            a += pa
            q += pq
        out[kind] = (a, q)
    cut, fill = tref.areas(design, c["ground"])
    assert out["cut"][0] == cut and out["fill"][0] == fill, (c["name"], out, cut, fill)
    return out, (design[0][0], design[-1][0])


def main():
    sections, swept = [], []
    by_name = {}
    for case in tref.CASES:
        if "error" in case or "left" not in case:
            continue
        name = case["name"]
        c = {"name": name, "template": case["template"], "axis": case["axis"], "ground": case["ground"]}
        out, reach = section(c)
        cut, fill = out["cut"], out["fill"]
        c.update({"cut": float(cut[0]), "fill": float(fill[0]), "cutMoment": float(cut[1]), "fillMoment": float(fill[1]),
                  "reach": [float(reach[0]), float(reach[1])]})
        sections.append(c)
        by_name[name] = ((cut, fill), reach, c)
    # Constant sections swept along arcs: left and right turns, small and large radii, one reaching the centre.
    # The sections whose cut or fill sits off the axis (a symmetric one's moment is 0: no correction), and one that is.
    chosen = [n for n, v in by_name.items() if v[0][0][1] != 0 or v[0][1][1] != 0][:8] + [next(iter(by_name))]
    for name in chosen:
        (cut, fill), reach, c = by_name[name]
        for radius, theta, left in ((F(150), F(1, 5), True), (F(150), F(1, 5), False), (F(40), F(1, 2), True),
                                    (F(1000), F(1, 20), False)):
            # r = R + o (left turn: the centre on the left) or R − o.
            inside = reach[0] if left else reach[1]
            if (left and radius + inside <= 0) or (not left and radius - inside <= 0):
                continue
            vol = {k: theta * (radius * a + (q if left else -q)) for k, (a, q) in (("cut", cut), ("fill", fill))}
            swept.append({"name": f"{name}: R {radius} m, {'sola' if left else 'sağa'}, θ {theta}", "section": name,
                          "radius": float(radius), "angle": float(theta), "left": left, "length": float(radius * theta),
                          "cut": float(vol["cut"]), "fill": float(vol["fill"])})
    # Reaching past the centre: no solid of revolution. A left turn whose radius is half the left reach.
    widest = max(by_name.values(), key=lambda v: float(-v[1][0]))
    radius = -widest[1][0] / 2
    swept.append({"name": f"{widest[2]['name']}: sol kol kurbun merkezini geçer", "section": widest[2]["name"],
                  "radius": float(radius), "angle": 0.5, "left": True, "length": float(radius / 2), "past": True})
    # Docs/adr/0977: this reference works in percent, KentOS's templates hold grades.
    sections = [dict(c) for c in sections]
    for c in sections:
        # A fresh copy each, through JSON: the cases share their templates and a template its sides.
        c["template"] = json.loads(json.dumps(c["template"]))
        road_grades.template(c["template"])
    text = json.dumps({"sections": sections, "swept": swept}, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            sys.exit(f"{OUT} is not what the reference writes")
        return
    OUT.write_text(text)
    print(f"{OUT}: {len(sections)} kesit + {len(swept)} süpürme durumu")


if __name__ == "__main__":
    main()
