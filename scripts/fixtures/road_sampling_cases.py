#!/usr/bin/env python3
"""A corridor's adaptive sampling, shared cases (docs/adr/0939, plan/road.md Y33.1), from an independent reference: the
ADR's rules written here without KentOS code, on sections given by closed forms.

A case's sections: an axis along a line (x = s, y = 0) or a left-turning circle of radius R; coded points at offsets
(right positive) on the axis's normal, each present over a range of stations; their height
z(s) = z0 + g·s + c·s²/2 + a·sin(2π·s/λ) + the code's own dz; no section over the ranges given. What is checked:

- the vertical chord height step √(8t/k), and on a parabola the measured height stray c·h²/8 of its middle;
- the stray of a middle section from the straight join of its neighbours: per code the plan distance to the chord
  (its projection kept within the chord) and the height off the chord there, the largest of each and its code;
  "changed" when the three sections do not have the same codes, or one has a section and another has none;
- the refinement: each interval's middle measured, one beyond the tolerances (or changed) halved, both halves
  measured in turn, depth first, until within, or its halves would be shorter than twice the least step (its middle
  is then added and it is left at the least step), or the budget of middles is used (left for the budget); intervals
  shorter than twice the least step are not measured.

    python3 scripts/fixtures/road_sampling_cases.py
    ... --check   # fails when fixtures/road/v1/sampling.json is not what this writes
"""

import json
import math
import sys
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "sampling.json"


# ---- the sections -------------------------------------------------------------------------------------------------

def axis(family, s):
    """The axis's place and its right normal at station s."""
    r = family.get("radius")
    if r is None:
        return (s, 0.0), (0.0, -1.0)
    th = s / r
    return (r * math.sin(th), r * (1 - math.cos(th))), (math.sin(th), -math.cos(th))


def height(family, s, dz):
    z0, g, c = family.get("z0", 100.0), family.get("grade", 0.0), family.get("bend", 0.0)
    a, lam = family.get("amplitude", 0.0), family.get("wave", 1.0)
    return z0 + g * s + c * s * s / 2 + a * math.sin(2 * math.pi * s / lam) + dz


def coded(family, s):
    for lo, hi in family.get("none", []):
        if lo <= s < hi:
            return None
    (ax, ay), (nx, ny) = axis(family, s)
    out = []
    for p in family["points"]:
        if p.get("from", -math.inf) <= s < p.get("to", math.inf):
            o = p["offset"]
            out.append((p["code"], (ax + o * nx, ay + o * ny, height(family, s, p.get("dz", 0.0)))))
    return out


# ---- the stray ----------------------------------------------------------------------------------------------------

def stray(a, m, b):
    if a is None or m is None or b is None:
        return {"plan": 0.0, "planCode": None, "height": 0.0, "heightCode": None,
                "changed": not (a is None and m is None and b is None)}
    codes = lambda v: sorted(p[0] for p in v)  # noqa: E731
    out = {"plan": 0.0, "planCode": None, "height": 0.0, "heightCode": None,
           "changed": codes(a) != codes(m) or codes(m) != codes(b)}
    for code, p in m:
        pa = next((q[1] for q in a if q[0] == code), None)
        pb = next((q[1] for q in b if q[0] == code), None)
        if pa is None or pb is None:
            continue
        dx, dy = pb[0] - pa[0], pb[1] - pa[1]
        l2 = dx * dx + dy * dy
        t = min(1.0, max(0.0, ((p[0] - pa[0]) * dx + (p[1] - pa[1]) * dy) / l2)) if l2 > 0 else 0.0
        cx, cy, cz = pa[0] + t * dx, pa[1] + t * dy, pa[2] + t * (pb[2] - pa[2])
        plan = math.hypot(p[0] - cx, p[1] - cy)
        h = abs(p[2] - cz)
        if plan > out["plan"]:
            out["plan"], out["planCode"] = plan, code
        if h > out["height"]:
            out["height"], out["heightCode"] = h, code
    return out


def within(s, tol):
    return (not s["changed"] and (tol.get("plan") is None or s["plan"] <= tol["plan"])
            and (tol.get("height") is None or s["height"] <= tol["height"]))


# ---- the refinement -----------------------------------------------------------------------------------------------

def refine(family, stations, tol, budget):
    return refine_with(lambda s: coded(family, s), stations, tol, budget)


def refine_with(coded_at, stations, tol, budget):
    """The refinement over sections given by coded_at(station) (a list of (code, (x, y, z)), None: no section)."""
    least = tol["least"]
    added, before, after, left = [], [], [], []
    built = 0
    for s0, s1 in zip(stations, stations[1:]):
        todo = [(s0, s1, True)]
        while todo:
            a, b, first = todo.pop()
            half = (b - a) / 2
            if half < least:
                continue
            if built >= budget:
                left.append({"from": a, "to": b, "why": "budget"})
                continue
            m = a + half
            built += 1
            st = stray(coded_at(a), coded_at(m), coded_at(b))
            ok = within(st, tol)
            measured = {"from": a, "to": b, **st, "within": ok}
            if first:
                before.append(measured)
            if ok:
                after.append(measured)
            elif half / 2 < least:
                added.append(m)
                left.append({**measured, "why": "least"})
            else:
                added.append(m)
                todo.append((m, b, False))
                todo.append((a, m, False))
    return {"added": sorted(added), "before": before, "after": sorted(after, key=lambda x: x["from"]),
            "left": sorted(left, key=lambda x: x["from"])}


# ---- the cases ----------------------------------------------------------------------------------------------------

LANE = [{"code": "axis", "offset": 0.0}, {"code": "c1l", "offset": -3.5, "dz": -0.07},
        {"code": "c1r", "offset": 3.5, "dz": -0.07}]
EDGES = [{"code": "axis", "offset": 0.0}, {"code": "left", "offset": -6.0, "dz": -0.15},
         {"code": "right", "offset": 6.0, "dz": -0.15}]


def case(name, family, stations, tol, budget=1000):
    return {"name": name, "family": family, "stations": stations, "tolerance": tol, "budget": budget,
            "expected": refine(family, stations, tol, budget)}


def strays():
    """Single middles, measured: (name, family, a, m, b) → the stray."""
    out = []
    def one(name, fam, a, m, b):
        out.append({"name": name, "family": fam, "at": [a, m, b],
                    "expected": stray(coded(fam, a), coded(fam, m), coded(fam, b))})
    one("Doğruda düz yol: sapma yok", {"points": EDGES}, 0.0, 10.0, 20.0)
    one("Parabolde kot sapması c·h²/8", {"points": EDGES, "bend": 0.0004}, 0.0, 25.0, 50.0)
    one("Çemberde dış kenarın kirişten uzaklığı", {"points": EDGES, "radius": 120.0}, 0.0, 15.0, 30.0)
    one("Kesitsiz orta: değişti", {"points": EDGES, "none": [[9.0, 11.0]]}, 0.0, 10.0, 20.0)
    one("Üçü de kesitsiz: değişmedi", {"points": EDGES, "none": [[-1.0, 30.0]]}, 0.0, 10.0, 20.0)
    one("Hendek ortada başlıyor: değişti",
        {"points": EDGES + [{"code": "ditch", "offset": 8.0, "dz": -0.8, "from": 7.0}]}, 0.0, 10.0, 20.0)
    one("Ortanın izdüşümü kirişin dışında: uca kırpılır",
        {"points": [{"code": "axis", "offset": 0.0}], "radius": 4.0}, 0.0, 6.0, 12.0)
    return out


def verticals():
    out = []
    for t, k in [(0.05, 0.0004), (0.1, 0.0004), (0.01, 1 / 3000), (0.05, 0.0), (0.02, -0.001)]:
        out.append({"tolerance": t, "bend": k, "expected": None if k == 0 else math.sqrt(8 * t / abs(k))})
    return out


def divisions():
    """A profile's pieces divided by the chord height: the fewest equal steps of at most √(8t/k), k the parabola's
    constant second derivative or the circle's at its end farther from its centre (R²/h³, h = √(R² − d²))."""
    def bend(p):
        if p["kind"] == "parabola":
            return abs(p["r"])
        def at(s):
            d = s - p["sc"]
            h = math.sqrt(max(p["radius"] ** 2 - d * d, 0.0))
            return p["radius"] ** 2 / h ** 3
        return max(at(p["s0"]), at(p["s1"]))
    def run(name, pieces, t, region):
        out = []
        for p in pieces:
            if p["kind"] == "grade" or p["s1"] <= p["s0"]:
                continue
            k = bend(p)
            if k == 0:
                continue
            step = math.sqrt(8 * t / k)
            n = max(1, math.ceil((p["s1"] - p["s0"]) / step - 1e-9))
            for i in range(1, n):
                s = p["s0"] + (p["s1"] - p["s0"]) * i / n
                if region[0] < s < region[1]:
                    out.append(s)
        return {"name": name, "pieces": pieces, "tolerance": t, "region": region, "expected": out}
    par = lambda s0, s1, r: {"kind": "parabola", "s0": s0, "s1": s1, "z0": 100.0, "g0": 0.02, "r": r}  # noqa: E731
    gra = lambda s0, s1: {"kind": "grade", "s0": s0, "s1": s1, "z0": 100.0, "g": 0.01}  # noqa: E731
    cir = lambda s0, s1, sc, zc, rr, sag: {"kind": "circle", "s0": s0, "s1": s1, "sc": sc, "zc": zc,  # noqa: E731
                                          "radius": rr, "sag": sag}
    return [
        run("Tepe parabolü L 120, A %6: 0,05 m", [gra(0, 100), par(100, 220, -0.0005), gra(220, 400)], 0.05, [0, 400]),
        run("Aynı kurp, 0,01 m", [par(100, 220, -0.0005)], 0.01, [0, 400]),
        run("Bölge kurbun ortasında biter", [par(100, 220, -0.0005)], 0.01, [0, 160]),
        run("Çok yassı kurp: tek adım yeter", [par(0, 50, 0.00001)], 0.05, [0, 50]),
        run("Asimetrik: iki kol kendi eğriliğiyle", [par(100, 160, 0.0008), par(160, 260, 0.000288)], 0.02, [0, 300]),
        run("Çember R 3000, çukur", [cir(200, 290, 245, 3100.0, 3000.0, True)], 0.02, [0, 400]),
        run("Çember R 1500, tepe", [cir(50, 110, 80, -1400.0, 1500.0, False)], 0.01, [0, 400]),
        run("Yalnız eğimler: bölme yok", [gra(0, 100), gra(100, 300)], 0.01, [0, 300]),
    ]


def cases():
    out = []
    out.append(case("Doğruda düz yol: hiçbir şey eklenmez", {"points": EDGES}, [0.0, 25.0, 50.0],
                    {"plan": 0.02, "height": 0.02, "least": 0.5}))
    out.append(case("Parabol: c·h²/8 toleransın altına inene dek", {"points": EDGES, "bend": 0.0004},
                    [0.0, 50.0, 100.0], {"plan": None, "height": 0.02, "least": 0.5}))
    out.append(case("Parabol: tolerans yalnız planda, kot denetlenmez", {"points": EDGES, "bend": 0.0004},
                    [0.0, 50.0, 100.0], {"plan": 0.02, "height": None, "least": 0.5}))
    out.append(case("Çember R 120: dış kenarın sapması", {"points": EDGES, "radius": 120.0},
                    [0.0, 25.0, 50.0], {"plan": 0.05, "height": None, "least": 0.5}))
    out.append(case("Çember R 120: eksen yalnız", {"points": [{"code": "axis", "offset": 0.0}], "radius": 120.0},
                    [0.0, 25.0, 50.0], {"plan": 0.05, "height": None, "least": 0.5}))
    out.append(case("Çember R 30, dar kurp", {"points": LANE, "radius": 30.0}, [0.0, 20.0, 40.0],
                    {"plan": 0.01, "height": 0.01, "least": 0.25}))
    out.append(case("Dalgalı arazi λ 30: şev ucu", {"points": EDGES + [
                    {"code": "daylight", "offset": 14.0}], "amplitude": 0.6, "wave": 30.0},
                    [0.0, 25.0, 50.0, 75.0], {"plan": None, "height": 0.05, "least": 0.5}))
    out.append(case("Dalgalı arazi: en kısa adım 2 m'de kalır", {"points": EDGES, "amplitude": 0.6, "wave": 7.0},
                    [0.0, 20.0, 40.0], {"plan": None, "height": 0.01, "least": 2.0}))
    out.append(case("Hendek 37,2'de başlıyor: değişim en kısa adımda bulunur", {"points": EDGES + [
                    {"code": "ditch", "offset": 8.0, "dz": -0.8, "from": 37.2}]},
                    [0.0, 25.0, 50.0], {"plan": 0.05, "height": 0.05, "least": 0.5}))
    out.append(case("Kesitsiz aralık 12–14: sınırları bulunur", {"points": EDGES, "none": [[12.0, 14.0]]},
                    [0.0, 25.0], {"plan": 0.05, "height": 0.05, "least": 0.25}))
    out.append(case("Bütçe 5 orta: kalanlar bütçeden", {"points": EDGES, "bend": 0.002},
                    [0.0, 50.0, 100.0], {"plan": None, "height": 0.001, "least": 0.1}, budget=5))
    out.append(case("Bütçe 0: hiçbir şey ölçülmez", {"points": EDGES, "bend": 0.002}, [0.0, 50.0],
                    {"plan": None, "height": 0.001, "least": 0.1}, budget=0))
    out.append(case("İki en kısa adımdan kısa aralık ölçülmez", {"points": EDGES, "bend": 0.002},
                    [0.0, 0.9, 50.0], {"plan": None, "height": 0.001, "least": 0.5}))
    out.append(case("Toleranssız: yalnız biçim değişimi böler", {"points": EDGES + [
                    {"code": "ditch", "offset": 8.0, "to": 31.0}], "bend": 0.002},
                    [0.0, 40.0], {"plan": None, "height": None, "least": 1.0}))
    out.append(case("Eğim ve parabol birlikte: eğim doğruyla birleşir", {"points": EDGES, "grade": 0.04, "bend": -0.0006},
                    [0.0, 40.0, 80.0, 120.0], {"plan": 0.01, "height": 0.03, "least": 0.5}))
    out.append(case("Kurp ve tümsek birlikte", {"points": EDGES + [{"code": "daylight", "offset": 15.0, "dz": -2.0}],
                    "radius": 200.0, "amplitude": 0.3, "wave": 40.0},
                    [0.0, 30.0, 60.0, 90.0], {"plan": 0.05, "height": 0.05, "least": 0.5}))
    out.append(case("Eşit olmayan aralıklar", {"points": EDGES, "bend": 0.001}, [0.0, 7.0, 33.0, 34.0, 80.0],
                    {"plan": None, "height": 0.02, "least": 0.25}))
    out.append(case("Ters kurp gibi: sol çember, sağ yanda iki kod", {"points": LANE + [
                    {"code": "toe", "offset": 11.0, "dz": -1.5, "from": 0.0, "to": 18.0}], "radius": 60.0},
                    [0.0, 15.0, 30.0], {"plan": 0.02, "height": 0.02, "least": 0.5}))
    out.append(case("Aralık tam iki en kısa adım: bir kez ölçülür", {"points": EDGES, "bend": 0.01}, [0.0, 1.0],
                    {"plan": None, "height": 0.0001, "least": 0.5}))
    out.append(case("Gevşek tolerans: ilk ortalar yeter", {"points": EDGES, "radius": 300.0, "bend": 0.0002},
                    [0.0, 20.0, 40.0, 60.0], {"plan": 0.5, "height": 0.5, "least": 0.5}))
    out.append(case("Dar dalga uçlarda ve ortada sıfır: ölçülen orta onu görmez (sınır)", {"points": EDGES, "amplitude": 0.05, "wave": 3.0},
                    [0.0, 12.0], {"plan": None, "height": 0.001, "least": 0.75}))
    return out


def main():
    doc = {"verticals": verticals(), "divisions": divisions(), "strays": strays(), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            sys.exit(f"{OUT} is not what scripts/fixtures/road_sampling_cases.py writes")
        print(f"{OUT}: {len(doc['cases'])} refinements, {len(doc['strays'])} strays, {len(doc['verticals'])} steps")
        return
    OUT.write_text(text)
    print(f"wrote {OUT}: {len(doc['cases'])} refinements")


if __name__ == "__main__":
    main()
