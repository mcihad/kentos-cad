#!/usr/bin/env python3
"""The shared cases of a side's daylight diagnosis (docs/adr/0951, plan/road.md Y45; ROAD-COR-10), in exact fractions,
without KentOS code and by another way: KentOS walks the terrain outward from the slope's start; here the slope's ray
is cut against every terrain stretch on its own (two lines' crossing, or a stretch on the line), the crossings sorted,
and the terrain's continuous reach from the start found apart, the crossings beyond it dropped.

    python3 scripts/fixtures/road_daylight_cases.py [--check]
"""

import json
import sys
from fractions import Fraction as F
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "daylight.json"


def outward(ground, right):
    """(u, z, gap before) outward; the left read leftward, its gaps moved to the stretch they bound."""
    if right:
        return [(F(g["offset"]), None if g["z"] is None else F(g["z"]), g["gap"]) for g in ground]
    out = []
    for i in range(len(ground) - 1, -1, -1):
        g = ground[i]
        gap = i + 1 < len(ground) and ground[i + 1]["gap"]
        out.append((-F(g["offset"]), None if g["z"] is None else F(g["z"]), gap))
    return out


def stretches(t):
    """The terrain's stretches (ua, za, ub, zb) that are on the surface."""
    return [(a[0], a[1], b[0], b[1]) for a, b in zip(t, t[1:]) if not b[2] and a[1] is not None and b[1] is not None]


def height(t, u):
    for ua, za, ub, zb in stretches(t):
        if ua <= u <= ub:
            return za + (zb - za) * (u - ua) / (ub - ua)
    return None


def reach(t, u0):
    """How far outward the terrain lasts without a break from u0, and whether it ended in a break (else the swath)."""
    end = u0
    for ua, za, ub, zb in sorted(stretches(t)):
        if ub <= end:
            continue
        if ua > end:
            return end, True
        end = ub
    last = max(p[0] for p in t)
    # A break within a micrometre of the swath's end is its end (a sliver off the surface, no hole).
    return end, last - end > F(1, 10**6)


def diagnose(side, axis, ground, right):
    t = outward(ground, right)
    u, z = F(0), F(axis)
    for c in side["components"]:
        u += F(c["width"])
        z += F(c["width"]) * F(c["slope"]) / 100
    g = height(t, u)
    if g is None:
        return {"kind": "hinge"}
    cut = g > z
    if cut:
        u0, z0 = u, z
        if side.get("ditch"):
            d = side["ditch"]
            u0 += F(d["depth"]) * F(d["fore"]) + F(d["bottom"])
            z0 -= F(d["depth"])
        m = 1 / F(side["cut"])
    else:
        u0, z0, m = u, z, -1 / F(side["fill"])
    if height(t, u0) is None:
        # A slope starting within a micrometre of the swath's end has no swath left, not a hole.
        if max(p[0] for p in t) - u0 <= F(1, 10**6):
            return {"kind": "swath", "end": float(u0)}
        return {"kind": "gap", "at": float(u0)}
    end, broken = reach(t, u0)
    hits = []
    for ua, za, ub, zb in stretches(t):
        a, b = max(ua, u0), ub
        if b < a or b > end:
            continue
        ga = za + (zb - za) * (a - ua) / (ub - ua)
        fa, fb = z0 + m * (a - u0) - ga, z0 + m * (b - u0) - zb
        slope = (fb - fa) / (b - a) if b > a else None
        if fa == 0:
            hits.append((a, slope))
        if fb == 0:
            hits.append((b, slope))
        if fa != 0 and fb != 0 and (fa < 0) != (fb < 0):
            hits.append((a + (b - a) * fa / (fa - fb), slope))
    # A place on the line belongs to two stretches: once, with the outward stretch's slope (the last one's at the end).
    seen = {}
    for x, sl in sorted(hits, key=lambda h: h[0]):
        if x not in seen or (seen[x] is None):
            seen[x] = sl
    xs = sorted(seen)
    # The outward stretch's slope at a place hit exactly: the stretch starting there, else the one ending there.
    def slope_at(x):
        for ua, za, ub, zb in sorted(stretches(t)):
            a = max(ua, u0)
            if a <= x < ub and ub <= end:
                ga = za + (zb - za) * (a - ua) / (ub - ua)
                return ((z0 + m * (ub - u0) - zb) - (z0 + m * (a - u0) - ga)) / (ub - a)
        for ua, za, ub, zb in sorted(stretches(t)):
            if ub == x:
                a = max(ua, u0)
                ga = za + (zb - za) * (a - ua) / (ub - ua)
                return ((z0 + m * (ub - u0) - zb) - (z0 + m * (a - u0) - ga)) / (ub - a)
        return None
    if xs:
        first = xs[0]
        df = slope_at(first)
        sens = None if not df else float(1 / abs(df))
        out = {"kind": "hit", "offset": float(first), "crossings": len(xs), "cut": cut,
               "sensitivity": sens}
        if len(xs) > 1:
            out["next"] = float(xs[1] - first)
        return out
    if broken:
        return {"kind": "gap", "at": float(end)}
    return {"kind": "swath", "end": float(end)}


def cases():
    out = []
    lane = {"width": 3.5, "slope": -2.5}
    shoulder = {"width": 1.0, "slope": -4.0}
    plain = {"components": [lane, shoulder], "cut": 1.5, "fill": 2.0}
    ditched = {**plain, "ditch": {"fore": 2.0, "depth": 0.6, "bottom": 0.5}}

    def g(pts, gaps=()):
        return [{"offset": o, "z": z, "gap": i in gaps} for i, (o, z) in enumerate(pts)]

    def case(name, left, right, axis, ground):
        tpl = {"left": left, "right": right}
        out.append({"name": name, "template": tpl, "axis": axis, "ground": ground,
                    "expected": [diagnose(left, axis, ground, False), diagnose(right, axis, ground, True)]})

    flat = [(-40.0, 98.0), (-10.0, 98.0), (0.0, 98.0), (10.0, 98.0), (40.0, 98.0)]
    case("Düz arazide iki yan dolgu", plain, plain, 100.0, g(flat))
    rising = [(-40.0, 104.0), (-5.0, 101.5), (0.0, 101.0), (5.0, 101.5), (40.0, 108.0)]
    case("Yükselen arazide hendekli yarma", ditched, ditched, 100.0, g(rising))
    ridge = [(-40.0, 95.0), (0.0, 98.0), (6.0, 98.5), (9.0, 97.0), (12.0, 96.0), (14.0, 97.6), (16.0, 94.0),
             (40.0, 90.0)]
    case("Tümsek: şev araziyi üç kez keser", plain, plain, 100.0, g(ridge))
    # The fill slope falls 1/2; the terrain past the hinge 0.48: they close 2 cm a metre, 50 m of daylight per metre.
    grazing = [(-40.0, 96.0), (0.0, 99.6), (4.5, 99.6), (40.0, 99.6 - 35.5 * 0.48)]
    case("Teğete yakın: arazi şevle neredeyse paralel", plain, plain, 100.0, g(grazing))
    case("Menteşede arazi yok", plain, plain, 100.0,
         g([(-40.0, None), (-3.0, None), (-2.0, 98.0), (40.0, 98.0)]))
    holed = [(-40.0, 98.0), (0.0, 98.0), (6.0, 98.0), (7.0, 98.0), (9.0, 98.0), (40.0, 98.0)]
    case("Şev araziye varmadan boşluğa giriyor", plain, plain, 100.0, g(holed, gaps={4}))
    short = [(-8.0, 98.0), (0.0, 98.0), (6.0, 98.0)]
    case("Kesit kısa: şerit bitiyor", plain, plain, 100.0, g(short))
    # Every number binary-exact so the touch is exact: lanes drop 0.125 m twice, the hinge at 5 m, 99.75; the fill line
    # falls 1/2 to 98.75 at 7 m, where the terrain's corner touches it and falls away faster; it meets it again further.
    dyadic = {"components": [{"width": 4.0, "slope": -3.125}, {"width": 1.0, "slope": -12.5}], "cut": 1.5, "fill": 2.0}
    touch = [(-40.0, 98.0), (0.0, 98.0), (5.0, 98.0), (7.0, 98.75), (9.0, 97.0), (40.0, 85.0)]
    case("Arazi köşesi şeve değip ayrılıyor", dyadic, dyadic, 100.0, g(touch))
    mixed = [(-40.0, 106.0), (0.0, 100.0), (40.0, 94.0)]
    case("Sol yarma, sağ dolgu", ditched, plain, 100.0, g(mixed))
    case("Dar yarma şevi 1/1", {**plain, "cut": 1.0}, plain, 100.0, g(rising))
    return out


def main():
    data = {"cases": cases()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            sys.exit(f"{OUT} is not what {Path(__file__).name} writes")
        print(f"{OUT}: {len(data['cases'])} cases")
        return
    OUT.write_text(text)
    print(f"wrote {OUT}: {len(data['cases'])} cases")
    for c in data["cases"]:
        print(f"  {c['name'][:44]:44} {c['expected']}")


if __name__ == "__main__":
    main()
