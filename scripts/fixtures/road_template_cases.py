#!/usr/bin/env python3
"""A template's sections' shared cases (docs/adr/0903 §3–§5, plan/road.md Y5.2), from an independent reference in exact
fractions: each side built outward from the axis by its components' widths and percent slopes, the hinge's terrain
deciding cut or fill, a ditch then the cut slope, or the fill slope, met with the terrain where the slope's line first
crosses a terrain segment (solved exactly); the cut and fill areas the exact integral of the design line less the
terrain between the outermost points. No KentOS code.

    python3 scripts/fixtures/road_template_cases.py           (--check: fails when the file differs)
"""

import json
import sys
from fractions import Fraction as F
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "templates.json"


def lane(w, s):
    return {"kind": "lane", "width": w, "slope": s}


def shoulder(w, s):
    return {"kind": "shoulder", "width": w, "slope": s}


def side(components, cut=1.0, fill=1.5, ditch=None):
    out = {"components": components, "cut": cut, "fill": fill}
    if ditch:
        out["ditch"] = {"fore": ditch[0], "depth": ditch[1], "bottom": ditch[2]}
    return out


def g(*pts, gaps=()):
    """Terrain places (offset, z or None), left to right; `gaps`: indices whose stretch before is off the surface."""
    return [{"offset": o, "z": z, "gap": i in gaps} for i, (o, z) in enumerate(pts)]


def outward(ground, right):
    if right:
        return [(F(p["offset"]), None if p["z"] is None else F(p["z"]), p["gap"]) for p in ground]
    out = []
    for i in range(len(ground) - 1, -1, -1):
        p = ground[i]
        gap = i + 1 < len(ground) and ground[i + 1]["gap"]
        out.append((-F(p["offset"]), None if p["z"] is None else F(p["z"]), gap))
    return out


def height(t, u):
    for (ua, za, _), (ub, zb, gap) in zip(t, t[1:]):
        if u == ua:
            return za
        if ua < u < ub:
            if gap or za is None or zb is None:
                return None
            return za + (zb - za) * (u - ua) / (ub - ua)
    if t and u == t[-1][0]:
        return t[-1][1]
    return None


def meet(t, u0, z0, m):
    for (ua, za, _), (ub, zb, gap) in zip(t, t[1:]):
        if ub < u0 or gap or za is None or zb is None:
            continue
        a = max(ua, u0)
        ga = za + (zb - za) * (a - ua) / (ub - ua)
        fa = z0 + m * (a - u0) - ga
        fb = z0 + m * (ub - u0) - zb
        if fa == 0:
            return [a, ga]
        if (fa < 0) != (fb < 0) or fb == 0:
            u = a + (ub - a) * fa / (fa - fb)
            return [u, z0 + m * (u - u0)]
    return None


def build_side(s, axis, ground, right):
    t = outward(ground, right)
    pts = [[F(0), F(axis)]]
    u, z = F(0), F(axis)
    for c in s["components"]:
        u += F(c["width"])
        z += F(c["width"]) * F(c["slope"]) / 100
        pts.append([u, z])
    hinge = [u, z]
    gz = height(t, u)
    if gz is None:
        kind, day = "open", None
    elif gz > z:
        bu, bz = u, z
        d = s.get("ditch")
        if d:
            bu += F(d["depth"]) * F(d["fore"])
            bz -= F(d["depth"])
            pts.append([bu, bz])
            if d["bottom"] > 0:
                bu += F(d["bottom"])
                pts.append([bu, bz])
        day = meet(t, bu, bz, 1 / F(s["cut"]))
        kind = "cut" if day else "open"
    else:
        day = meet(t, u, z, -1 / F(s["fill"]))
        kind = "fill" if day else "open"
    if day and pts[-1] != day:
        pts.append(day)
    sign = 1 if right else -1
    flip = lambda p: [sign * p[0], p[1]]
    return {"points": [flip(p) for p in pts], "hinge": flip(hinge), "daylight": flip(day) if day else None,
            "kind": kind}


def areas(design, ground):
    t = [(F(p["offset"]), None if p["z"] is None else F(p["z"]), p["gap"]) for p in ground]
    lo, hi = design[0][0], design[-1][0]
    stops = sorted({p[0] for p in design} | {F(p["offset"]) for p in ground if lo <= F(p["offset"]) <= hi})

    def line(o):
        for a, b in zip(design, design[1:]):
            if a[0] <= o <= b[0]:
                return b[1] if a[0] == b[0] else a[1] + (b[1] - a[1]) * (o - a[0]) / (b[0] - a[0])

    cut = fill = F(0)
    for a, b in zip(stops, stops[1:]):
        if height(t, (a + b) / 2) is None:
            continue
        ga, gb = height(t, a), height(t, b)
        if ga is None or gb is None:
            continue
        fa, fb = line(a) - ga, line(b) - gb
        parts = [(a, b, fa, fb)]
        if (fa < 0 < fb) or (fb < 0 < fa):
            x = a + (b - a) * fa / (fa - fb)
            parts = [(a, x, fa, F(0)), (x, b, F(0), fb)]
        for x0, x1, f0, f1 in parts:
            area = (f0 + f1) / 2 * (x1 - x0)
            if f0 + f1 > 0:
                fill += area
            else:
                cut -= area
    return cut, fill


def bad(s, right):
    for i, c in enumerate(s["components"]):
        if not (c["width"] > 0):
            return {"right": right, "component": i}
    d = s.get("ditch")
    if not (s["cut"] > 0 and s["fill"] > 0) or (d and not (d["fore"] > 0 and d["depth"] > 0 and d["bottom"] >= 0)):
        return {"right": right, "component": None}
    return None


def fl(p):
    return [float(v) for v in p]


def make(name, template, axis, ground):
    out = {"name": name, "template": template, "axis": axis, "ground": ground}
    err = bad(template["left"], False) or bad(template["right"], True)
    if err:
        out["error"] = err
        return out
    left = build_side(template["left"], axis, ground, False)
    right = build_side(template["right"], axis, ground, True)
    design = list(reversed(left["points"])) + right["points"][1:]
    cut, fill = areas(design, ground)
    for s in (left, right):
        s["points"] = [fl(p) for p in s["points"]]
        s["hinge"] = fl(s["hinge"])
        s["daylight"] = fl(s["daylight"]) if s["daylight"] else None
    out.update({"left": left, "right": right, "cut": float(cut), "fill": float(fill)})
    return out


ROAD = [lane(3.5, -2.5), shoulder(1.0, -4)]
SYM = {"left": side(ROAD), "right": side(ROAD)}
DITCH = {"left": side(ROAD, 1.0, 1.5, (2.0, 0.8, 0.5)), "right": side(ROAD, 1.0, 1.5, (2.0, 0.8, 0.5))}
FLAT = g((-40, 100), (40, 100))
CASES = [
    make("Düz arazide dolgu: iki yan dolgu şevi", SYM, 102.0, FLAT),
    make("Düz arazide yarma: iki yan yarma şevi", SYM, 97.0, FLAT),
    make("Hendekli yarma", DITCH, 97.0, FLAT),
    make("V hendek (tabansız)", {"left": side(ROAD, 1.0, 1.5, (1.5, 0.6, 0)), "right": side(ROAD, 1.0, 1.5, (1.5, 0.6, 0))},
         96.0, FLAT),
    make("Yamaç: solda yarma, sağda dolgu", DITCH, 100.0, g((-40, 108), (0, 100), (40, 92))),
    make("Kırıklı arazi: şev ilk buluşmada durur", SYM, 101.0,
         g((-40, 99), (-20, 101.5), (-12, 99.2), (-6, 100.4), (0, 100), (8, 99.6), (15, 98.0), (30, 103), (40, 99))),
    # Exact binary numbers: the hinge (100.25 − 0.125 − 0.0625) is the terrain's height to the bit.
    make("Menteşede arazi aynı kotta: dolgu, daylight menteşede",
         {"left": side([lane(4.0, -3.125), shoulder(1.0, -6.25)]), "right": side([lane(4.0, -3.125), shoulder(1.0, -6.25)])},
         100.25, g((-40, 100.0625), (40, 100.0625))),
    make("Şevler şeride sığmıyor: yanlar açık", SYM, 120.0, g((-12, 100), (12, 100))),
    make("Menteşede arazi yok: açık", SYM, 100.0, g((-3, 99), (3, 99))),
    make("Boşluklu arazi: sağ şev boşluğu atlar", SYM, 102.0,
         g((-40, 100), (0, 100), (6, 100), (10, 95), (40, 95), gaps=(3,))),
    make("Asimetrik tip kesit", {"left": side([lane(3.0, -2.0)], 2.0, 2.0), "right": side([lane(3.5, -2.5), lane(3.5, -2.5),
         shoulder(1.5, -5)], 0.5, 3.0)}, 100.0, g((-50, 101.5), (0, 100.5), (50, 99))),
    make("Bileşensiz yan: menteşe eksende", {"left": side([]), "right": side(ROAD)}, 101.0, FLAT),
    make("Dik yarma şevi 1/2, yatık dolgu 3", {"left": side(ROAD, 0.5, 3.0), "right": side(ROAD, 0.5, 3.0)}, 100.0,
         g((-40, 104), (0, 101), (40, 96))),
    make("Ters eğimli şerit (dever gibi)", {"left": side([lane(3.5, 4.0)]), "right": side([lane(3.5, -4.0)])}, 100.0,
         g((-30, 99), (30, 99))),
    make("Büyük kotlar", DITCH, 1251.37, g((-30, 1252.9), (-5, 1251.0), (5, 1250.6), (30, 1249.2))),
    make("Hendek arazinin altına iniyor", DITCH, 99.5, g((-40, 100.2), (40, 100.2))),
    make("Yalnız sağ yanda arazi", SYM, 102.0, g((0.5, 100), (40, 100))),
    make("Sıfır genişlik: geçersiz", {"left": side([lane(0.0, -2)]), "right": side(ROAD)}, 100.0, FLAT),
    make("Sıfır yarma şevi: geçersiz", {"left": side(ROAD), "right": side(ROAD, 0.0)}, 100.0, FLAT),
    make("Eksi hendek tabanı: geçersiz", {"left": side(ROAD, 1.0, 1.5, (2.0, 0.8, -1)), "right": side(ROAD)}, 100.0, FLAT),
]


def main():
    text = json.dumps({"cases": CASES}, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            sys.exit(f"{OUT} is not what the reference writes")
        print(f"{OUT.name}: {len(CASES)} cases, the same")
        return
    OUT.write_text(text)
    print(f"wrote {OUT} ({len(CASES)} cases)")


if __name__ == "__main__":
    main()
