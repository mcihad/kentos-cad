#!/usr/bin/env python3
"""The vertical alignment's shared cases (docs/adr/0902 §2–§4, plan/road.md Y4.2), from an independent reference: a
parabola's elevation and grade from the textbook form in exact fractions (an asymmetric one solved here from its two
conditions, one elevation and one grade at the PVI), a circle's from its centre and the tangent length R·tan(Δ/2) in
mpmath at 40 digits. No KentOS code.

    python3 scripts/fixtures/road_vertical_cases.py           (needs mpmath: uv run --no-project --with mpmath …)
    ... --check   # fails when fixtures/road/v1/verticals.json is not what this writes
"""

import json
import sys
from fractions import Fraction as F
from pathlib import Path

import mpmath as mp

mp.mp.dps = 40
OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "verticals.json"


class Refused(Exception):
    def __init__(self, **kw):
        self.kw = kw


def pvi(id, s, z, curve=None):
    return {"id": id, "station": s, "elevation": z, **({"curve": curve} if curve else {})}


def par(l):
    return {"kind": "parabola", "length": l}


def asym(a, b):
    return {"kind": "asymmetric", "lengthIn": a, "lengthOut": b}


def circ(r):
    return {"kind": "circle", "radius": r}


def solve(pvis):
    """The curves' reaches and an evaluator, or a refusal."""
    if len(pvis) < 2:
        raise Refused(kind="TooFewPoints")
    ids = set()
    for i, p in enumerate(pvis):
        if p["id"] in ids:
            raise Refused(kind="DuplicateId", pvi=p["id"])
        ids.add(p["id"])
        if i and p["station"] <= pvis[i - 1]["station"]:
            raise Refused(kind="NotIncreasing", pvi=p["id"])
        c = p.get("curve")
        if c:
            end = i in (0, len(pvis) - 1)
            vals = [v for k, v in c.items() if k != "kind"]
            if end or any(v <= 0 for v in vals):
                raise Refused(kind="BadCurve", pvi=p["id"])
    S = [F(p["station"]) for p in pvis]
    Z = [F(p["elevation"]) for p in pvis]
    G = [(Z[i + 1] - Z[i]) / (S[i + 1] - S[i]) for i in range(len(pvis) - 1)]
    curves = {}
    for i in range(1, len(pvis) - 1):
        c = pvis[i].get("curve")
        if not c:
            continue
        g1, g2 = G[i - 1], G[i]
        if g1 == g2:
            raise Refused(kind="BadCurve", pvi=pvis[i]["id"])
        s, z = S[i], Z[i]
        if c["kind"] == "parabola":
            L = F(c["length"])
            b, e = s - L / 2, s + L / 2
            r = (g2 - g1) / L
            zb = z - g1 * L / 2
            ev = lambda q, b=b, zb=zb, g1=g1, r=r: (zb + g1 * (q - b) + r * (q - b) ** 2 / 2, g1 + r * (q - b), r)
        elif c["kind"] == "asymmetric":
            l1, l2 = F(c["lengthIn"]), F(c["lengthOut"])
            # Unknowns: the two curvatures. The elevation at s from each side and the grade at s from each side agree.
            # z_b + g1 l1 + r1 l1²/2 = z_e − g2 l2 + r2 l2²/2 (with z_b = z − g1 l1, z_e = z + g2 l2): r1 l1² = r2 l2².
            # g1 + r1 l1 = g2 − r2 l2.  Solved by substitution.
            r1 = (g2 - g1) / (l1 + l1 * l1 / l2)
            r2 = r1 * l1 * l1 / (l2 * l2)
            b, e = s - l1, s + l2
            zb, ze = z - g1 * l1, z + g2 * l2

            def ev(q, b=b, e=e, s=s, zb=zb, ze=ze, g1=g1, g2=g2, r1=r1, r2=r2):
                if q < s:
                    return (zb + g1 * (q - b) + r1 * (q - b) ** 2 / 2, g1 + r1 * (q - b), r1)
                u = e - q
                return (ze - g2 * u + r2 * u * u / 2, g2 - r2 * u, r2)
        else:
            R = mp.mpf(c["radius"])
            t1, t2 = mp.atan(mp.mpf(g1.numerator) / g1.denominator), mp.atan(mp.mpf(g2.numerator) / g2.denominator)
            sag = t2 > t1
            T = R * mp.tan(abs(t2 - t1) / 2)
            sm, zm = mp.mpf(s.numerator) / s.denominator, mp.mpf(z.numerator) / z.denominator
            bs, bz = sm - T * mp.cos(t1), zm - T * mp.sin(t1)
            es = sm + T * mp.cos(t2)
            sign = 1 if sag else -1
            sc, zc = bs - sign * R * mp.sin(t1), bz + sign * R * mp.cos(t1)
            b, e = bs, es

            def ev(q, sc=sc, zc=zc, R=R, sag=sag):
                d = mp.mpf(q) - sc
                h = mp.sqrt(R * R - d * d)
                if sag:
                    return (zc - h, d / h, R * R / h ** 3)
                return (zc + h, -d / h, -R * R / h ** 3)
        curves[i] = (b, e, ev, g1, g2)
    reach, who = S[0], pvis[0]["id"]
    for i in sorted(curves):
        b, e = curves[i][0], curves[i][1]
        if b < reach - mp.mpf("1e-9"):
            raise Refused(kind="Overlap", a=who, b=pvis[i]["id"], excess=float(reach - b))
        reach, who = e, pvis[i]["id"]
    if reach > S[-1] + mp.mpf("1e-9"):
        raise Refused(kind="Overlap", a=who, b=pvis[-1]["id"], excess=float(reach - S[-1]))

    def at(q):
        q = F(q)
        if q < S[0] or q > S[-1]:
            return None
        # A piece's start is its own, an end the next piece's (the alignment's rule); the profile's end its last piece's.
        for i, (b, e, ev, _, _) in curves.items():
            if b <= q < e or (q == S[-1] and e == q):
                return tuple(float(v) for v in ev(q))
        for i in range(len(S) - 1):
            if S[i] <= q < S[i + 1] or (i == len(S) - 2 and q == S[-1]):
                return (float(Z[i] + G[i] * (q - S[i])), float(G[i]), 0.0)
        return None

    return curves, at, S, G


def mpf(v):
    return mp.mpf(v.numerator) / v.denominator if isinstance(v, F) else mp.mpf(v)


def report(pvis, curves):
    out = []
    for i in sorted(curves):
        b, e, ev, g1, g2 = curves[i]
        lo, hi = mpf(b), mpf(e)
        exact = isinstance(b, F)
        # A parabola's points exactly (fractions, through a 40-digit decimal); a circle's in mpmath.
        val = lambda q: tuple(mpf(v) for v in ev(F(mp.nstr(q, 40)) if exact else q))
        a = 100 * (g2 - g1)
        ga, gb = val(lo)[1], val(hi)[1]
        extreme = None
        if ga == 0:
            extreme = lo
        elif gb == 0:
            extreme = hi
        elif (ga < 0) != (gb < 0):
            x0, x1 = lo, hi
            for _ in range(140):
                m = (x0 + x1) / 2
                if (val(m)[1] < 0) == (ga < 0):
                    x0 = m
                else:
                    x1 = m
            extreme = (x0 + x1) / 2
        out.append({
            "pvi": pvis[i]["id"], "gIn": float(g1), "gOut": float(g2),
            "start": [float(lo), float(val(lo)[0])], "end": [float(hi), float(val(hi)[0])],
            "a": float(a), "k": float((hi - lo) / abs(mpf(a))), "crest": bool(g2 < g1),
            "extreme": None if extreme is None else [float(extreme), float(val(extreme)[0])],
        })
    return out


CASES = [
    ("İki PVI, yalnız eğim", [pvi("a", 0, 100), pvi("b", 400, 108)], [0, 123.4, 400]),
    ("Simetrik parabol, çukur", [pvi("a", 0, 100), pvi("v1", 300, 94, par(160)), pvi("b", 700, 106)], [0, 220, 260, 300, 350, 380, 700]),
    ("Simetrik parabol, tepe, en yüksek nokta içinde", [pvi("a", 0, 100), pvi("v1", 250, 110, par(200)), pvi("b", 600, 99.5)],
     [150, 200, 250, 300, 350]),
    ("Tepe, iki eğim de artı: en yüksek nokta dışında", [pvi("a", 0, 50), pvi("v1", 200, 60, par(120)), pvi("b", 500, 66)],
     [140, 200, 260]),
    ("Asimetrik parabol", [pvi("a", 0, 100), pvi("v1", 300, 91, asym(80, 200)), pvi("b", 800, 111)],
     [220, 260, 300, 301, 400, 500]),
    ("Asimetrik tepe", [pvi("a", 1000, 845.2), pvi("v1", 1400, 861.2, asym(150, 60)), pvi("b", 1700, 852.2)],
     [1250, 1300, 1400, 1430, 1460]),
    ("Çember, çukur", [pvi("a", 0, 100), pvi("v1", 300, 94, circ(4000)), pvi("b", 700, 106)], [210, 260, 300, 350, 389]),
    ("Çember, tepe", [pvi("a", 0, 100), pvi("v1", 250, 110, circ(3000)), pvi("b", 600, 99.5)], [150, 200, 250, 300, 340]),
    ("Beş PVI, üç kurp türü, yüksek kotlar", [pvi("a", 2000, 1250.4), pvi("v1", 2300, 1262.4, par(180)),
     pvi("v2", 2650, 1251.9, circ(6000)), pvi("v3", 3000, 1265.9, asym(100, 140)), pvi("b", 3400, 1257.9)],
     [2000, 2210, 2300, 2390, 2600, 2650, 2700, 2900, 3000, 3140, 3400]),
    ("Kurplar uç uca", [pvi("a", 0, 100), pvi("v1", 200, 104, par(100)), pvi("v2", 300, 102, par(100)), pvi("b", 500, 106)],
     [150, 250, 300, 350]),
    ("Kırık PVI, kurpsuz", [pvi("a", 0, 100), pvi("v1", 200, 104), pvi("b", 500, 101)], [100, 200, 350]),
    ("Kurplar biniyor", [pvi("a", 0, 100), pvi("v1", 200, 104, par(160)), pvi("v2", 300, 102, par(100)), pvi("b", 500, 106)], []),
    ("Kurp başlangıçtan taşıyor", [pvi("a", 0, 100), pvi("v1", 50, 101, par(160)), pvi("b", 500, 96)], []),
    ("Kurp bitişten taşıyor", [pvi("a", 0, 100), pvi("v1", 450, 109, asym(50, 80)), pvi("b", 500, 106)], []),
    ("Km geri gidiyor", [pvi("a", 0, 100), pvi("v1", 300, 94), pvi("b", 250, 106)], []),
    ("Aynı kimlik", [pvi("a", 0, 100), pvi("a", 300, 94), pvi("b", 700, 106)], []),
    ("Tek PVI", [pvi("a", 0, 100)], []),
    ("Başlangıçta kurp", [pvi("a", 0, 100, par(50)), pvi("b", 300, 94)], []),
    ("Sıfır uzunluk", [pvi("a", 0, 100), pvi("v1", 300, 94, par(0)), pvi("b", 700, 106)], []),
    ("Sıfır yarıçap", [pvi("a", 0, 100), pvi("v1", 300, 94, circ(0)), pvi("b", 700, 106)], []),
    ("Eğim değişmiyor", [pvi("a", 0, 100), pvi("v1", 300, 106, par(100)), pvi("b", 700, 114)], []),
]


def make(name, pvis, stations):
    out = {"name": name, "pvis": pvis}
    try:
        curves, at, S, G = solve(pvis)
    except Refused as r:
        out["error"] = r.kw
        return out
    out["curves"] = report(pvis, curves)
    out["at"] = [{"station": s, "value": list(at(s))} for s in stations]
    return out


def main():
    text = json.dumps({"cases": [make(*c) for c in CASES]}, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            sys.exit(f"{OUT} is not what the reference writes")
        print(f"{OUT.name}: {len(CASES)} cases, the same")
        return
    OUT.write_text(text)
    print(f"wrote {OUT} ({len(CASES)} cases)")


if __name__ == "__main__":
    main()
