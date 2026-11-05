#!/usr/bin/env python3
"""The corridor volume's sampling convergence corpus (docs/adr/0971, ROAD-QA-03), from an independent reference: no
KentOS code.

A straight road on flat ground at 100 m, a level platform W wide and side slopes m_fill and m_cut (horizontal per one
vertical): where the red line stands H above the ground the fill area is H·(W + m_fill·H), where it is below the cut
area is |H|·(W + m_cut·|H|) (trapezoids). The red line z(s) is the textbook vertical alignment (grades and symmetric
parabolas, exact fractions). So each area is a piecewise polynomial in s, and the exact volume is its integral: each
piece's antiderivative, split at the curves' ends and where H changes sign (a quadratic's root, mpmath at 50 digits).

For each interval h the sections every h metres give the average end area sum Σ L·(A₀ + A₁)/2 and the prismoidal one
Σ L·(A₀ + 4·A_m + A₁)/6 with the middle section. The trapezoidal rule's error is −(b − a)·h²·A″/12, so where A is
smooth the average end area converges as h² and the prismoidal (Simpson's rule) as h⁴, exact for a cubic. Where A
has a kink between stations (a fill that ends on a station-free place: A's slope jumps from W·H′ to 0) both fall to
h². The orders seen are written and checked here.

    uv run --no-project --python 3.11 --with mpmath python3 scripts/fixtures/road_volume_convergence_cases.py
    ... --check   # fails when fixtures/road/v1/volume-convergence.json is not what this writes
"""

import json
import sys
from fractions import Fraction as F
from pathlib import Path

import mpmath as mp

mp.mp.dps = 50
OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "volume-convergence.json"
GROUND = F(100)
STEPS = [F(40), F(20), F(10), F(5), F(5, 2), F(5, 4)]


def pieces(pvis):
    """The red line's polynomial pieces (s0, s1, [c0, c1, c2]) with z = c0 + c1·t + c2·t², t = s − s0."""
    S = [F(p["station"]) for p in pvis]
    Z = [F(p["elevation"]) for p in pvis]
    G = [(Z[i + 1] - Z[i]) / (S[i + 1] - S[i]) for i in range(len(S) - 1)]
    out = []
    for i in range(1, len(S)):
        # The grade between PVI i−1 and i runs from the end of the curve on i−1 (S + L/2) to the start of the curve
        # on i (S − L/2); then the curve on i.
        length = F(pvis[i - 1].get("length", 0)) if i - 1 > 0 else F(0)
        nxt = F(pvis[i].get("length", 0)) if i < len(S) - 1 else F(0)
        start, end = S[i - 1] + length / 2, S[i] - nxt / 2
        z0 = Z[i - 1] + G[i - 1] * (start - S[i - 1])
        out.append((start, end, [z0, G[i - 1], F(0)]))
        if nxt:
            b = S[i] - nxt / 2
            zb = Z[i] - G[i - 1] * nxt / 2
            r = (G[i] - G[i - 1]) / nxt
            out.append((b, b + nxt, [zb, G[i - 1], r / 2]))
    return [p for p in out if p[1] > p[0]]


def red(ps, s):
    for s0, s1, c in ps:
        if s0 <= s <= s1:
            t = s - s0
            return c[0] + c[1] * t + c[2] * t * t
    raise ValueError(s)


def areas(case, h):
    """Cut and fill areas where the red line stands h above the ground."""
    w, mf, mc = F(case["width"]), F(case["fill"]), F(case["cut"])
    if h >= 0:
        return F(0), h * (w + mf * h)
    return -h * (w + mc * -h), F(0)


def exact(case, ps):
    """The exact cut and fill volumes: each piece's H(t) = c0 − 100 + c1·t + c2·t² split at its roots, A(H(t))
    integrated as a polynomial in t."""
    w, mf, mc = (mp.mpf(F(case[k]).numerator) / F(case[k]).denominator for k in ("width", "fill", "cut"))
    cut = fill = mp.mpf(0)
    for s0, s1, c in ps:
        h = [mp.mpf(c[0] - GROUND), mp.mpf(c[1]), mp.mpf(c[2])]
        length = mp.mpf(s1 - s0)
        cuts = [mp.mpf(0), length]
        if h[2] != 0:
            disc = h[1] ** 2 - 4 * h[2] * h[0]
            if disc > 0:
                cuts += [(-h[1] + sg * mp.sqrt(disc)) / (2 * h[2]) for sg in (1, -1)]
        elif h[1] != 0:
            cuts.append(-h[0] / h[1])
        cuts = sorted(x for x in set(cuts) if 0 <= x <= length)
        # A = H·(W + m·H) = W·H + m·H², H a quadratic: integrate the quartic exactly.
        hp = h
        h2 = [hp[0] * hp[0], 2 * hp[0] * hp[1], hp[1] ** 2 + 2 * hp[0] * hp[2], 2 * hp[1] * hp[2], hp[2] ** 2]
        integral = lambda poly, a, b: sum(k / (n + 1) * (b ** (n + 1) - a ** (n + 1)) for n, k in enumerate(poly))
        for a, b in zip(cuts, cuts[1:]):
            mid = (a + b) / 2
            sign = hp[0] + hp[1] * mid + hp[2] * mid * mid
            m = mf if sign > 0 else mc
            v = w * integral(hp, a, b) + m * integral(h2, a, b)
            if sign > 0:
                fill += v
            else:
                cut += -w * integral(hp, a, b) + m * integral(h2, a, b)
    return cut, fill


def sums(case, ps, step):
    s0, s1 = ps[0][0], ps[-1][1]
    n = int((s1 - s0) / step)
    assert s0 + n * step == s1, "the steps divide the range"
    st = [s0 + k * step for k in range(n + 1)]
    a = [areas(case, red(ps, s) - GROUND) for s in st]
    m = [areas(case, red(ps, (x + y) / 2) - GROUND) for x, y in zip(st, st[1:])]
    end = [sum(step * (a[k][j] + a[k + 1][j]) / 2 for k in range(n)) for j in (0, 1)]
    pri = [sum(step * (a[k][j] + 4 * m[k][j] + a[k + 1][j]) / 6 for k in range(n)) for j in (0, 1)]
    return end, pri


def orders(errors):
    return [float(mp.log(errors[i] / errors[i + 1], 2)) if errors[i + 1] > 0 and errors[i] > 0 else None
            for i in range(len(errors) - 1)]


def case(name, pvis, width=7, fill=2, cut=F(3, 2), expect=None):
    c = {"name": name, "pvis": pvis, "width": float(width), "fill": float(fill), "cut": float(cut), "ground": 100.0}
    ps = pieces(pvis)
    vc, vf = exact(c, ps)
    rows = []
    for h in STEPS:
        end, pri = sums(c, ps, h)
        rows.append({"step": float(h), "endAreas": [float(end[0]), float(end[1])],
                     "prismoidal": [float(pri[0]), float(pri[1])]})
    total = vc + vf
    err_end = [abs(mp.mpf(r["endAreas"][0] + r["endAreas"][1]) - total) for r in rows]
    err_pri = [abs(mp.mpf(r["prismoidal"][0] + r["prismoidal"][1]) - total) for r in rows]
    c.update({"exact": [float(vc), float(vf)], "sums": rows,
              "orders": {"endAreas": orders(err_end), "prismoidal": orders(err_pri)}})
    if expect:
        expect(c, err_end, err_pri)
    return c


def near(v, want, tol):
    return v is not None and abs(v - want) <= tol


def main():
    pv = lambda id_, s, z, **k: {"id": id_, "station": float(s), "elevation": float(z), **{a: float(b) for a, b in k.items()}}

    def smooth(c, e, p):
        # Fill throughout, A smooth inside each piece: h² and h⁴ (the curve ends are stations at every h here).
        assert all(near(o, 2, 0.05) for o in c["orders"]["endAreas"]), c["orders"]
        assert all(near(o, 4, 0.05) for o in c["orders"]["prismoidal"] if o is not None), c["orders"]

    def ramp(c, e, p):
        # A single grade crossing the ground at a station: A piecewise quadratic with its kink on a station;
        # the prismoidal formula is exact for a quadratic.
        assert all(near(o, 2, 1e-9) for o in c["orders"]["endAreas"]), c["orders"]
        assert all(x < mp.mpf("1e-30") for x in p), p

    def kink(c, e, p):
        # The fill ends between stations (an irrational root): A's slope jumps there; both converge as h², the
        # prismoidal no better on average.
        os = c["orders"]["prismoidal"]
        assert 1.5 < sum(os) / len(os) < 2.5, os

    cases = [
        case("Dolguda tepe kurbu: düzgün alan", [pv("a", 0, 102), pv("v", 200, 106, length=160), pv("b", 400, 103)],
             expect=smooth),
        case("Rampa zemini km 80'de keser: prizmoidal tam", [pv("a", 0, 98), pv("b", 320, 106)], expect=ramp),
        case("Çukur kurbunda yarma–dolgu: kırık km'ler arasında",
             [pv("a", 0, 103), pv("v", 200, 96, length=200), pv("b", 400, 103.5)], expect=kink),
    ]
    out = {"format": "kentos-cases", "version": 1,
           "note": "Corridor volume by average end areas and the prismoidal formula at shrinking station intervals "
                   "against the exact integral (docs/adr/0971); straight road, flat ground, level platform; no KentOS "
                   "code.", "cases": cases}
    text = json.dumps(out, ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv:
        sys.exit(0 if OUT.read_text("utf-8") == text else "volume-convergence.json eski: yeniden üretin")
    OUT.write_text(text, "utf-8")
    for c in cases:
        print(c["name"], c["exact"], c["orders"])


if __name__ == "__main__":
    main()
