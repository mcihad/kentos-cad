#!/usr/bin/env python3
"""A template's pavement on its sections, shared cases (docs/adr/0975 §1, plan/road.md Y26.1), from an independent
reference in exact fractions: the section as road_template_cases.py builds it; each layer the region between the
finish grade lowered by the depths above and below it (outward from the axis, extended with the last component's
slope past the hinge when the edge is a slope), not above the edge line from the hinge nor the finished line past it,
not past the section's footprint (where its finished line ends);
its area, the pavement's area under and above the ground, by integrating over the offset with every crossing of every
two segments taken as a break (brute force, no envelopes); the earthwork from the formation as the road's cut plus the
pavement under the ground and its fill less the pavement above it. No KentOS code.

    python3 scripts/fixtures/road_pavement_cases.py           (--check: fails when the file differs)
"""

import json
import sys
from fractions import Fraction as F
from itertools import combinations
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_grades  # noqa: E402  (docs/adr/0977: the grades are written as KentOS holds them)
from road_template_cases import areas, bad, build_side, g, lane, shoulder, side  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "pavements.json"


def seg_value(pts, u):
    """A polyline's height at u (u inside its span); None outside."""
    if not pts or u < pts[0][0] or u > pts[-1][0]:
        return None
    for (ua, za), (ub, zb) in zip(pts, pts[1:]):
        if ua <= u <= ub and ub > ua:
            return za + (zb - za) * (u - ua) / (ub - ua)
    return pts[-1][1] if u == pts[-1][0] else None


def finish(components, axis):
    pts, u, z, last = [(F(0), F(axis))], F(0), F(axis), F(0)
    for c in components:
        w, s = F(c["width"]), F(c["slope"]) / 100
        u += w
        z += w * s
        last = s
        pts.append((u, z))
    return pts, last


def ground_runs(ground, right):
    pts = []
    for i, p in enumerate(ground):
        u = F(p["offset"]) if right else -F(p["offset"])
        gap = p["gap"] if right else (i + 1 < len(ground) and ground[i + 1]["gap"])
        pts.append((u, None if p["z"] is None else F(p["z"]), gap))
    if not right:
        pts.reverse()
    runs, run = [], []
    for u, z, gap in pts:
        if z is None:
            if len(run) > 1:
                runs.append(run)
            run = []
        elif gap:
            if len(run) > 1:
                runs.append(run)
            run = [(u, z)]
        else:
            run.append((u, z))
    if len(run) > 1:
        runs.append(run)
    return runs


def segments(pts):
    return [(a, b) for a, b in zip(pts, pts[1:]) if b[0] > a[0]]


def integrate(hi, caps, lo, u0, u1):
    """∫ max(0, min(hi ∪ caps where defined) − max(lo)) over [u0, u1] where hi and lo are all defined."""
    if u1 <= u0:
        return F(0)
    lines = [p for f in hi + caps + lo for p in f]
    stops = {u0, u1} | {p[0] for p in lines if u0 < p[0] < u1}
    segs = [s for f in hi + caps + lo for s in segments(f)]
    for (a, b), (c, d) in combinations(segs, 2):
        m1 = (b[1] - a[1]) / (b[0] - a[0])
        m2 = (d[1] - c[1]) / (d[0] - c[0])
        if m1 == m2:
            continue
        u = (c[1] - m2 * c[0] - a[1] + m1 * a[0]) / (m1 - m2)
        if u0 < u < u1 and a[0] <= u <= b[0] and c[0] <= u <= d[0]:
            stops.add(u)
    stops = sorted(stops)
    total = F(0)
    for a, b in zip(stops, stops[1:]):
        m = (a + b) / 2
        if any(seg_value(f, m) is None for f in hi + lo):
            continue

        def diff(u, m=m):
            # The functions chosen at the middle, read at u (they are linear on the piece).
            top = min(value_on(f, m, u) for f in hi + [c for c in caps if seg_value(c, m) is not None])
            bottom = max(value_on(f, m, u) for f in lo)
            return top - bottom

        f0, f1 = diff(a), diff(b)
        # Zero of the linear difference inside: split.
        if (f0 > 0) != (f1 > 0) and f0 != f1:
            x = a + (b - a) * f0 / (f0 - f1)
            total += part(a, x, f0, F(0)) + part(x, b, F(0), f1)
        else:
            total += part(a, b, f0, f1)
    return total


def value_on(f, m, u):
    """The line of f's segment holding m, read at u."""
    for (ua, za), (ub, zb) in segments(f):
        if ua <= m <= ub:
            return za + (zb - za) * (u - ua) / (ub - ua)
    raise ValueError("not defined")


def part(a, b, f0, f1):
    return (f0 + f1) / 2 * (b - a) if f0 + f1 > 0 else F(0)


def shifted(pts, d):
    return [(u, z - d) for u, z in pts]


def pave_side(sd, built, axis, depths, edge, right):
    pts, last = finish(sd["components"], axis)
    hinge = pts[-1]
    sign = 1 if right else -1
    outward = [(sign * F(p[0]), F(p[1])) for p in built["points"]]
    outward = [p for p in outward if p[0] >= hinge[0]]
    caps = []
    total = depths[-1]
    if edge is None:
        end = hinge[0]
    else:
        h = F(edge)
        reach = total / (last + 1 / h)
        end = hinge[0] + reach
        pts = pts + [(end, hinge[1] + last * reach)]
        caps.append([hinge, (end, hinge[1] - reach / h)])
        if len(outward) >= 2 and outward[-1][0] > hinge[0]:
            caps.append(outward)
        # Not past the section's footprint: where its finished line ends (the daylight).
        end = min(end, outward[-1][0] if outward else hinge[0])
    return pts, caps, end, hinge


def make(name, template, axis, ground, layers, edge, strip=0):
    out = {"name": name, "template": template, "axis": axis, "ground": ground,
           "pavement": {"layers": layers, "edge": edge}}
    err = bad(template["left"], False) or bad(template["right"], True)
    assert not err, name
    if not layers:
        out["error"] = {"layer": None}
        return out
    for i, t in enumerate(layers):
        if not t > 0:
            out["error"] = {"layer": i}
            return out
    for key, right in (("left", False), ("right", True)):
        if not template[key]["components"]:
            out["error"] = {"noPlatform": right}
            return out
        if edge is not None:
            last = F(template[key]["components"][-1]["slope"]) / 100
            if not edge > 0 or last + 1 / F(edge) <= 0:
                out["error"] = {"edge": right}
                return out
    if strip:
        out["strip"] = strip
    lowered = [dict(p, z=None if p["z"] is None else p["z"] - strip) for p in ground]
    depths, d = [], F(0)
    for t in layers:
        d += F(t)
        depths.append(d)
    built = {k: build_side(template[k], axis, ground, r) for k, r in (("left", False), ("right", True))}
    design = list(reversed(built["left"]["points"])) + built["right"]["points"][1:]
    cut, fill = areas(design, lowered)
    areas_l = [F(0)] * len(depths)
    below = above = F(0)
    for key, right in (("left", False), ("right", True)):
        pts, caps, end, _ = pave_side(template[key], built[key], axis, depths, edge, right)
        top_d = F(0)
        for i, dd in enumerate(depths):
            areas_l[i] += integrate([shifted(pts, top_d)], caps, [shifted(pts, dd)], F(0), end)
            top_d = dd
        for run in ground_runs(lowered, right):
            below += integrate([pts, run], caps, [shifted(pts, depths[-1])], F(0), end)
            above += integrate([pts], caps, [shifted(pts, depths[-1]), run], F(0), end)
    out.update({
        "layers": [float(a) for a in areas_l],
        "below": float(below),
        "above": float(above),
        "road": {"cut": float(cut), "fill": float(fill)},
        "formation": {"cut": float(cut + below), "fill": float(fill - above)},
    })
    return out


def bottom_line(pts, caps, end, hinge, edge, d):
    """A layer's bottom `d` deep, outward to where it meets the edge: the hinge's foot for a vertical edge, the edge line
    for a slope (clamped to the hinge and the pavement's end)."""
    line = shifted(pts, d)
    if edge is None:
        stop = hinge[0]
    else:
        (h0, h1) = caps[0]
        me = (h1[1] - h0[1]) / (h1[0] - h0[0])
        p, q = line[-2], line[-1]
        ml = (q[1] - p[1]) / (q[0] - p[0])
        u = (p[1] - ml * p[0] - h0[1] + me * h0[0]) / (me - ml)
        stop = min(max(u, hinge[0]), end)
    out = [x for x in line if x[0] < stop]
    for a, b in zip(line, line[1:]):
        if a[0] <= stop <= b[0] and b[0] > a[0]:
            out.append((stop, a[1] + (b[1] - a[1]) * (stop - a[0]) / (b[0] - a[0])))
            break
    return out


def section_pavement(template, axis, ground, pavement, built, cut, fill):
    """A section's pavement row (road.sections.query's `pavement`) for a template's `pavement` definition: none where
    it does not build. `built`: the two sides as road_template_cases.py builds them; `cut`, `fill` the road's areas."""
    layers = [F(l["thickness"]) for l in pavement["layers"]]
    edge = pavement["edge"]["h"] if pavement["edge"]["kind"] == "slope" else None
    for key in ("left", "right"):
        if not template[key]["components"]:
            return None
        if edge is not None:
            last = F(template[key]["components"][-1]["slope"]) / 100
            if last + 1 / F(edge) <= 0:
                return None
    depths, d = [], F(0)
    for t in layers:
        d += t
        depths.append(d)
    areas_l = [F(0)] * len(depths)
    below = above = F(0)
    halves = []
    for key, right in (("left", False), ("right", True)):
        pts, caps, end, hinge = pave_side(template[key], built[key], axis, depths, edge, right)
        top_d = F(0)
        for i, dd in enumerate(depths):
            areas_l[i] += integrate([shifted(pts, top_d)], caps, [shifted(pts, dd)], F(0), end)
            top_d = dd
        for run in ground_runs(ground, right):
            below += integrate([pts, run], caps, [shifted(pts, depths[-1])], F(0), end)
            above += integrate([pts], caps, [shifted(pts, depths[-1]), run], F(0), end)
        sign = 1 if right else -1
        halves.append([[(sign * u, z) for u, z in bottom_line(pts, caps, end, hinge, edge, dd)] for dd in depths])
    bottoms = [[[float(u), float(z)] for u, z in list(reversed(halves[0][i])) + halves[1][i][1:]] for i in range(len(depths))]
    return {
        "layers": [{"name": l["name"], "area": float(a)} for l, a in zip(pavement["layers"], areas_l)],
        "below": float(below),
        "above": float(above),
        "formation": bottoms[-1],
        "bottoms": bottoms,
        "formationCut": float(F(cut) + below),
        "formationFill": float(F(fill) - above),
    }


ROAD = [lane(3.5, -2.5), shoulder(1.0, -4)]
SYM = {"left": side(ROAD), "right": side(ROAD)}
DITCH = {"left": side(ROAD, 1.0, 1.5, (2.0, 0.8, 0.5)), "right": side(ROAD, 1.0, 1.5, (2.0, 0.8, 0.5))}
FLAT = g((-40, 100), (40, 100))
# KGM'nin bitümlü sıcak karışım üstyapısı gibi dört tabaka; kalınlıklar örnektir, standart değildir.
KGM4 = [0.05, 0.08, 0.20, 0.25]
TWO = [0.25, 0.5]

CASES = [
    make("Düz arazide dolgu, dik kenar", SYM, 102.0, FLAT, KGM4, None),
    make("Düz arazide dolgu, kenar dolgu şevinde", SYM, 102.0, FLAT, KGM4, 1.5),
    make("Düz arazide yarma, dik kenar: üstyapı zeminin altında", SYM, 97.0, FLAT, TWO, None),
    make("Düz arazide yarma, şevli kenar", SYM, 97.0, FLAT, TWO, 1.0),
    make("Hendekli yarma, kenar hendekten yatık: hendek keser", DITCH, 97.0, FLAT, TWO, 3.0),
    make("Hendekli yarma, kenar hendekten dik", DITCH, 97.0, FLAT, TWO, 1.0),
    make("Yamaç: arazi üstyapıyı kesiyor", DITCH, 100.0, g((-40, 108), (0, 100), (40, 92)), KGM4, 1.5),
    make("Arazi üstyapının içinde, dik kenar", SYM, 100.3, g((-40, 100.2), (40, 99.9)), KGM4, None),
    make("Ters eğimli şerit (dever gibi)", {"left": side([lane(3.5, 4.0)]), "right": side([lane(3.5, -4.0)])}, 100.0,
         g((-30, 99), (30, 99)), TWO, 2.0),
    make("Boşluklu arazi", SYM, 102.0, g((-40, 100), (0, 100), (6, 100), (10, 95), (40, 95), gaps=(3,)), TWO, 1.5),
    make("Asimetrik tip kesit", {"left": side([lane(3.0, -2.0)], 2.0, 2.0),
                                 "right": side([lane(3.5, -2.5), lane(3.5, -2.5), shoulder(1.5, -5)], 0.5, 3.0)},
         100.0, g((-50, 101.5), (0, 100.5), (50, 99)), KGM4, 2.0),
    make("Büyük kotlar", DITCH, 1251.37, g((-30, 1252.9), (-5, 1251.0), (5, 1250.6), (30, 1249.2)), KGM4, 1.5),
    make("Tek tabaka", SYM, 101.0, FLAT, [0.3], None),
    make("Bitkisel toprak sıyrılmış zeminde (Formasyon–Bitkisel Toprak)", SYM, 100.1, FLAT, KGM4, 1.5, strip=0.25),
    make("Tabakasız: geçersiz", SYM, 101.0, FLAT, [], None),
    make("Sıfır kalınlık: geçersiz", SYM, 101.0, FLAT, [0.05, 0.0], None),
    make("Kenar şevi sıfır: geçersiz", SYM, 101.0, FLAT, TWO, 0.0),
    make("Kenar şevi banketten yatık: tabakalar onu hiç kesmez", SYM, 101.0, FLAT, TWO, 30.0),
    make("Bileşensiz yan: üstyapı kurulmaz", {"left": side([]), "right": side(ROAD)}, 101.0, FLAT, TWO, None),
]


def written(cases):
    """The cases as KentOS reads them (docs/adr/0977): this reference works in percent, KentOS's templates hold grades."""
    out = [dict(c) for c in cases]
    for c in out:
        # A fresh copy each, through JSON: the cases share their templates and a template its sides.
        c["template"] = json.loads(json.dumps(c["template"]))
        road_grades.template(c["template"])
    return out


def main():
    text = json.dumps({"cases": written(CASES)}, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            sys.exit(f"{OUT} is not what the reference writes")
        print(f"{OUT.name}: {len(CASES)} cases, the same")
        return
    OUT.write_text(text)
    print(f"wrote {OUT} ({len(CASES)} cases)")


if __name__ == "__main__":
    main()
