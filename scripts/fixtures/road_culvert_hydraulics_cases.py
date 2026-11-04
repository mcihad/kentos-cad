#!/usr/bin/env python3
"""A culvert's headwater and the road's overtopping, shared cases (docs/adr/0947, plan/road.md Y41.1; ROAD-DRAIN-08),
from an independent reference in mpmath at 40 digits: the critical depth by mpmath's findroot (a circle's A³/T = Q²/g
from a bracket), HDS-5's inlet and outlet equations as the ADR writes them, the flow at a headwater and the overtopping
split each solved by findroot's bisection on their own equations; the hazard by AIDR guideline 7-3's table. No KentOS
code.

Checked before writing: a box's critical depth is (q²/g)^(1/3) in closed form; the culvert and the weir together carry
the flow; on the crest depth × speed is the flow per metre; the hazard's classes at their limits.

    uv run --no-project --python 3.11 --with mpmath python3 scripts/fixtures/road_culvert_hydraulics_cases.py
    ... --check   # fails when fixtures/road/v1/culvert_hydraulics.json is not what this writes
"""

import json
import sys
from pathlib import Path

import mpmath as mp

mp.mp.dps = 40
OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "culvert_hydraulics.json"
G = mp.mpf("9.81")
KU = mp.mpf("1.811")

# HDS-5 table A.1 rows (form, K, M, c, Y, Ks), as reproduced in the ADR's source.
INLETS = {
    "pipe-square-headwall": (1, "0.0098", "2.0", "0.0398", "0.67", "-0.5"),
    "pipe-groove-headwall": (1, "0.0018", "2.0", "0.0292", "0.74", "-0.5"),
    "pipe-groove-projecting": (1, "0.0045", "2.0", "0.0317", "0.69", "-0.5"),
    "box-wingwall-30-75": (1, "0.026", "1.0", "0.0347", "0.81", "-0.5"),
    "box-wingwall-90-15": (1, "0.061", "0.75", "0.0400", "0.80", "-0.5"),
    "box-wingwall-0": (1, "0.061", "0.75", "0.0423", "0.82", "-0.5"),
    "box-headwall-chamfer": (2, "0.515", "0.667", "0.0375", "0.79", "-0.5"),
}


def full(sec):
    if sec["kind"] == "circle":
        d = mp.mpf(sec["diameter"])
        return mp.pi * d * d / 4, mp.pi * d, d
    b, h = mp.mpf(sec["width"]), mp.mpf(sec["height"])
    return b * h, 2 * (b + h), h


def wet(sec, y):
    if sec["kind"] == "circle":
        d = mp.mpf(sec["diameter"])
        th = 2 * mp.acos(1 - 2 * y / d)
        return d * d / 8 * (th - mp.sin(th)), d * mp.sin(th / 2)
    b = mp.mpf(sec["width"])
    return b * y, b


def critical(sec, q):
    q = mp.mpf(q)
    if q <= 0:
        return mp.mpf(0)
    if sec["kind"] == "box":
        b, h = mp.mpf(sec["width"]), mp.mpf(sec["height"])
        return min(mp.cbrt((q / b) ** 2 / G), h)
    d = mp.mpf(sec["diameter"])
    # In logarithms, A³/T running from 0 to ∞ over the depth is well scaled.
    f = lambda y: (lambda at: 3 * mp.log(at[0]) - mp.log(at[1]) - mp.log(q * q / G))(wet(sec, y))  # noqa: E731
    y = mp.findroot(f, (d * mp.mpf("1e-9"), d * (1 - mp.mpf("1e-9"))), solver="illinois", tol=mp.mpf("1e-50"),
                    verify=False)
    assert abs(f(y)) < mp.mpf("1e-25"), ("critical depth", sec, q, f(y))
    return y


def critical_head(sec, q):
    y = critical(sec, q)
    a, t = wet(sec, y)
    _, _, d = full(sec)
    if t > 0 and y < d:
        return y + a / (2 * t)
    return y + (q / full(sec)[0]) ** 2 / (2 * G)


def inlet_hw(c, q):
    if q <= 0:
        return mp.mpf(0)
    sec, cells = c["section"], c["cells"]
    qc = mp.mpf(q) / cells
    a, _, d = full(sec)
    form, k, m, cc, yy, ks = INLETS[c["inlet"]]
    k, m, cc, yy, ks = map(mp.mpf, (k, m, cc, yy, ks))
    s = mp.mpf(c["slope"])
    x = KU * qc / (a * mp.sqrt(d))
    un = lambda x: (critical_head(sec, qc) / d + k * x ** m + ks * s) if form == 1 else k * x ** m  # noqa: E731
    sub = lambda x: cc * x * x + yy + ks * s  # noqa: E731
    if x <= mp.mpf("3.5"):
        r = un(x)
    elif x >= 4:
        r = sub(x)
    else:
        u, v = un(mp.mpf("3.5")), sub(mp.mpf(4))
        r = u + (v - u) * (x - mp.mpf("3.5")) / mp.mpf("0.5")
    return d * r


def outlet_hw(c, q, tw):
    if q <= 0:
        return max(mp.mpf(tw) - mp.mpf(c["slope"]) * mp.mpf(c["length"]), mp.mpf(0))
    sec, cells = c["section"], c["cells"]
    qc = mp.mpf(q) / cells
    a, p, d = full(sec)
    r = a / p
    v = qc / a
    dc = critical(sec, qc)
    tw_eff = max(mp.mpf(tw), (dc + d) / 2)
    n, length, ke = mp.mpf(c["n"]), mp.mpf(c["length"]), mp.mpf(c["ke"])
    return tw_eff + (1 + ke + 2 * G * n * n * length / r ** (mp.mpf(4) / 3)) * v * v / (2 * G) - mp.mpf(c["slope"]) * length


def headwater(c, q, tw):
    if q <= 0:
        return max(mp.mpf(tw) - mp.mpf(c["slope"]) * mp.mpf(c["length"]), mp.mpf(0)), "outlet"
    i, o = inlet_hw(c, q), outlet_hw(c, q, tw)
    return (i, "inlet") if i >= o else (o, "outlet")


def flow_at(c, hw, tw):
    hw = mp.mpf(hw)
    if hw <= 0:
        return mp.mpf(0)
    hi = mp.mpf(1)
    while headwater(c, hi, tw)[0] < hw:
        hi *= 2
    return mp.findroot(lambda q: headwater(c, q, tw)[0] - hw, (mp.mpf(0), hi), solver="bisect", tol=mp.mpf("1e-30"))


def hazard(depth, speed):
    dv = depth * speed
    for k, (p, d, v) in enumerate([(0.3, 0.3, 2.0), (0.6, 0.5, 2.0), (0.6, 1.2, 2.0), (1.0, 2.0, 2.0), (4.0, 4.0, 4.0)]):
        if dv <= p and depth <= d and speed <= v:
            return k + 1
    return 6


def perform(c, weir, q, tw):
    q = mp.mpf(q)
    hw, control = headwater(c, q, tw)
    crest, length, cd = (mp.mpf(weir[k]) for k in ("crest", "length", "cd"))
    if hw <= crest:
        return {"headwater": hw, "control": control, "inlet": inlet_hw(c, q), "outlet": outlet_hw(c, q, tw), "culvert": q, "overtopping": mp.mpf(0), "head": mp.mpf(0),
                "depth": mp.mpf(0), "speed": mp.mpf(0), "unit": mp.mpf(0), "hazard": 0}
    h = mp.findroot(lambda h: flow_at(c, crest + h, tw) + cd * length * h ** mp.mpf("1.5") - q, (mp.mpf(0), hw - crest),
                    solver="bisect", tol=mp.mpf("1e-25"))
    through = flow_at(c, crest + h, tw)
    over = q - through
    unit = over / length
    depth = mp.cbrt(unit * unit / G)
    speed = mp.sqrt(G * depth)
    assert abs(through + cd * length * h ** mp.mpf("1.5") - q) < mp.mpf("1e-15")
    assert abs(depth * speed - unit) < mp.mpf("1e-25"), "on the crest depth × speed is the flow per metre"
    return {"headwater": crest + h, "control": headwater(c, through, tw)[1], "inlet": inlet_hw(c, through),
            "outlet": outlet_hw(c, through, tw), "culvert": through, "overtopping": over,
            "head": h, "depth": depth, "speed": speed, "unit": unit, "hazard": hazard(float(depth), float(speed))}


def culvert(section, inlet, cells=1, length=30.0, slope=0.01, n=0.013, ke=0.5):
    return {"section": section, "cells": cells, "length": length, "slope": slope, "n": n, "inlet": inlet, "ke": ke}


def cases():
    pipe = {"kind": "circle", "diameter": 1.2}
    box = {"kind": "box", "width": 2.0, "height": 1.5}
    for q in (0.5, 3.0, 9.0):  # a box's critical depth in closed form
        assert abs(critical(box, q) - min(mp.cbrt((mp.mpf(q) / 2) ** 2 / G), mp.mpf("1.5"))) < mp.mpf("1e-30")
    for d, v, k in ((0.3, 1.0, 1), (0.31, 1.0, 2), (0.5, 1.2, 2), (0.5, 1.21, 4), (1.0, 0.5, 3), (1.0, 1.0, 4), (2.0, 2.0, 5), (3.0, 1.5, 6)):
        assert hazard(d, v) == k, (d, v, hazard(d, v))
    out = []

    def case(name, c, weir, q, tw):
        p = perform(c, weir, q, tw)
        out.append({"name": name, "culvert": c, "weir": weir, "flow": q, "tailwater": tw,
                    "expected": {"critical": float(critical(c["section"], mp.mpf(q) / c["cells"])),
                                 **{k: (float(v) if isinstance(v, mp.mpf) else v) for k, v in p.items()}}})

    road = {"crest": 4.0, "length": 60.0, "cd": 1.66}
    case("Boru, küçük debi: giriş kontrollü, batmamış", culvert(pipe, "pipe-square-headwall"), road, 0.8, 0.3)
    case("Boru, orta debi: geçiş bölgesi", culvert(pipe, "pipe-square-headwall"), road, 2.6, 0.5)
    case("Boru, büyük debi: batmış giriş", culvert(pipe, "pipe-square-headwall"), road, 4.0, 0.8)
    case("Boru, yüksek kuyruk suyu: çıkış kontrollü", culvert(pipe, "pipe-square-headwall"), road, 2.0, 2.6)
    case("Oluklu ağızlı boru", culvert(pipe, "pipe-groove-headwall"), road, 2.6, 0.5)
    case("Çıkıntılı oluklu boru", culvert(pipe, "pipe-groove-projecting"), road, 2.6, 0.5)
    case("Kutu menfez 30–75° kanat", culvert(box, "box-wingwall-30-75", ke=0.4), road, 8.0, 0.8)
    case("Kutu menfez 90° ve 15° kanat", culvert(box, "box-wingwall-90-15", ke=0.4), road, 8.0, 0.8)
    case("Kutu menfez kanatsız", culvert(box, "box-wingwall-0", ke=0.5), road, 8.0, 0.8)
    case("Pahlı başlıklı kutu (form 2)", culvert(box, "box-headwall-chamfer", ke=0.5), road, 8.0, 0.8)
    case("İki gözlü kutu", culvert(box, "box-wingwall-30-75", cells=2, ke=0.4), road, 16.0, 0.8)
    case("Üç borulu menfez", culvert(pipe, "pipe-square-headwall", cells=3), road, 7.0, 0.5)
    case("Uzun, pürüzlü, yatık: çıkış kontrollü", culvert(pipe, "pipe-square-headwall", length=120.0, slope=0.002, n=0.024),
         road, 2.5, 0.6)
    low = {"crest": 2.2, "length": 40.0, "cd": 1.66}
    case("Yol taşıyor: az", culvert(pipe, "pipe-square-headwall"), low, 5.0, 0.5)
    case("Yol taşıyor: çok", culvert(pipe, "pipe-square-headwall"), low, 30.0, 0.5)
    case("Kutu menfezde yol taşıyor", culvert(box, "box-wingwall-30-75", ke=0.4), {"crest": 2.6, "length": 80.0, "cd": 1.66},
         40.0, 1.0)
    case("Kısa taşma boyu: tehlikeli", culvert(pipe, "pipe-square-headwall"), {"crest": 2.2, "length": 8.0, "cd": 1.66},
         15.0, 0.5)
    case("Sıfır debi", culvert(pipe, "pipe-square-headwall"), road, 0.0, 0.0)
    return out


def main():
    data = {"inlets": {k: {"form": v[0], "k": float(v[1]), "m": float(v[2]), "c": float(v[3]), "y": float(v[4]),
                           "ks": float(v[5])} for k, v in INLETS.items()}, "cases": cases()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            sys.exit(f"{OUT} is not what scripts/fixtures/road_culvert_hydraulics_cases.py writes")
        print(f"{OUT}: {len(data['cases'])} cases")
        return
    OUT.write_text(text)
    print(f"wrote {OUT}: {len(data['cases'])} cases")
    for c in data["cases"]:
        e = c["expected"]
        print(f"  {c['name'][:42]:42} HW {e['headwater']:.3f} ({e['control']}) giriş {e['inlet']:.3f} çıkış {e['outlet']:.3f} "
              f"taşan {e['overtopping']:.2f} y {e['depth']:.3f} v {e['speed']:.2f} H{e['hazard']}")


if __name__ == "__main__":
    main()
