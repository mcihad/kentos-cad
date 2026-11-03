#!/usr/bin/env python3
"""Writes fixtures/road/v1/haul.json: the shared cases of KentOS Yol's mass haul (Brückner) diagram (docs/adr/0907,
plan/road.md Y9), worked out here in exact fractions without KentOS code.

The diagram is the mass ordinate as a polyline (a repeated station a step: a borrow or waste site). Against a balance
line, f = M − b; balance points where f crosses or touches zero; between two of them a loop, forward where f > 0. Read
as layers: at a level y, each stretch where |f| ≥ y carries dy across its width; a stretch reaching an open end of the
diagram is unbalanced (waste forward, borrow backward). Here each layer band between two consecutive vertex levels is
found by solving the crossings at both ends of the band directly (no assumption shared with the core but the
polyline), and checked: a closed loop's moment is ∫|f| ds and its volume the upward variation of |f|; the overhaul by
a fine midpoint sum.

    python3 scripts/fixtures/road_haul_cases.py
    ... --check   # compares the file with the one on disk
"""

import json
import random
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/road/v1/haul.json"


def value(pts, s, after):
    at = [p for p in pts if p[0] == s]
    if at:
        return (at[-1] if after else at[0])[1]
    for p, q in zip(pts, pts[1:]):
        if p[0] < s < q[0]:
            return p[1] + (q[1] - p[1]) * (s - p[0]) / (q[0] - p[0])
    raise ValueError(s)


def crossings(pts, b):
    out = []
    for i, p in enumerate(pts):
        f = p[1] - b
        if f == 0 and 0 < i < len(pts) - 1 and (not out or out[-1] != p[0]):
            out.append(p[0])
        if i + 1 < len(pts):
            g = pts[i + 1][1] - b
            if (f < 0 < g) or (f > 0 > g):
                s = p[0] + (pts[i + 1][0] - p[0]) * f / (f - g)
                if not out or out[-1] != s:
                    out.append(s)
    return out


def intervals(g, y):
    """The stretches of g ≥ y at level y (not a vertex level): (left, right, left is the start, right is the end)."""
    out, start = [], None
    if g[0][1] >= y:
        start = (g[0][0], True)
    for p, q in zip(g, g[1:]):
        if (p[1] < y) != (q[1] < y):
            x = p[0] + (q[0] - p[0]) * (y - p[1]) / (q[1] - p[1])
            if start is None:
                start = (x, False)
            else:
                out.append((start[0], x, start[1], False))
                start = None
    if start is not None:
        out.append((start[0], g[-1][0], start[1], True))
    return out


def layers(g, free, open_start, open_end):
    levels = sorted({max(p[1], F(0)) for p in g} | {F(0)})
    vol = mom = ov = om = opn = F(0)
    for y0, y1 in zip(levels, levels[1:]):
        dy = y1 - y0
        eps = dy / 1000
        lo, hi = intervals(g, y0 + eps), intervals(g, y1 - eps)
        assert len(lo) == len(hi)
        for (a0, c0, s0, e0), (a1, c1, _, _) in zip(lo, hi):
            if (s0 and open_start) or (e0 and open_end):
                opn += dy
                continue
            # The widths at the band's ends, extended linearly from just inside it.
            w_lo, w_hi = c0 - a0, c1 - a1
            slope = (w_hi - w_lo) / (dy - 2 * eps)
            w0, w1 = w_lo - slope * eps, w_hi + slope * eps
            vol += dy
            mom += (w0 + w1) / 2 * dy
            d0, d1 = w0 - free, w1 - free
            if d0 >= 0 and d1 >= 0:
                om += (d0 + d1) / 2 * dy
                if d0 > 0 or d1 > 0:
                    ov += dy
            elif d0 > 0 or d1 > 0:
                h, l = (d0, d1) if d0 > 0 else (d1, d0)
                part = dy * h / (h - l)
                om += h / 2 * part
                ov += part
    return vol, mom, ov, om, opn


def piece(pts, b, sign, a, c):
    out = [(a, sign * (value(pts, a, True) - b))]
    out += [(p[0], sign * (p[1] - b)) for p in pts if a < p[0] < c]
    out.append((c, sign * (value(pts, c, False) - b)))
    return out


def check_closed(g, vol, mom):
    """A closed loop: the moment is ∫ g ds, the volume the upward variation of g."""
    area = sum((p[1] + q[1]) / 2 * (q[0] - p[0]) for p, q in zip(g, g[1:]))
    up = sum(max(F(0), q[1] - p[1]) for p, q in zip(g, g[1:]))
    assert area == mom, (area, mom)
    assert up == vol, (up, vol)


def check_over(g, free, om, n=4000):
    top = max(p[1] for p in g)
    if top <= 0:
        return
    total = 0.0
    for k in range(n):
        y = float(top) * (k + 0.5) / n
        for a, c, _, _ in intervals([(float(s), float(v)) for s, v in g], y):
            total += max(0.0, (c - a) - float(free)) * float(top) / n
    assert abs(total - float(om)) <= 2e-3 * max(1.0, float(om)), (total, float(om))


def diagram(pts, b, free):
    pts = [(F(s), F(m)) for s, m in pts]
    b, free = F(b), F(free)
    bal = crossings(pts, b)
    first, last = pts[0][0], pts[-1][0]
    cuts = [first] + [s for s in bal if first < s < last] + [last]
    loops = []
    tot = {"volume": F(0), "moment": F(0), "overVolume": F(0), "overMoment": F(0), "waste": F(0), "borrow": F(0)}
    for k, (a, c) in enumerate(zip(cuts, cuts[1:])):
        if c <= a:
            continue
        probe = piece(pts, b, 1, a, c)
        mid = F(0)
        for _, v in probe:
            if abs(v) > abs(mid):
                mid = v
        if mid == 0:
            continue
        fwd = mid > 0
        g = piece(pts, b, 1 if fwd else -1, a, c)
        os_ = k == 0 and g[0][1] > 0
        oe = k + 2 == len(cuts) and g[-1][1] > 0
        vol, mom, ov, om, opn = layers(g, free, os_, oe)
        if not os_ and not oe:
            check_closed(g, vol, mom)
            check_over(g, free, om)
        tot["volume"] += vol
        tot["moment"] += mom
        tot["overVolume"] += ov
        tot["overMoment"] += om
        tot["waste" if fwd else "borrow"] += opn
        loops.append({"from": float(a), "to": float(c), "forward": fwd, "volume": float(vol), "moment": float(mom),
                      "overVolume": float(ov), "overMoment": float(om), "unbalanced": float(opn)})
    return {"balance": [float(s) for s in bal], "loops": loops, **{k: float(v) for k, v in tot.items()}}


def cases():
    out = []

    def add(name, pts, b=0, free=0):
        out.append({"name": name, "points": [[float(F(s)), float(F(m))] for s, m in pts], "balance": float(F(b)),
                    "free": float(F(free)), "expect": diagram(pts, b, free)})

    tri = [(0, 0), (100, 1000), (200, 0)]
    add("Tek tepe: serbest taşıma yok", tri)
    add("Tek tepe: serbest taşıma 50 m", tri, free=50)
    add("Tek tepe: serbest taşıma tepenin enini aşar", tri, free=300)
    add("Yamuk tepe", [(0, 0), (50, 600), (150, 600), (200, 0)], free=80)
    add("İleri sonra geri", [(0, 0), (100, 800), (200, 0), (300, -500), (400, 0)], free=60)
    add("Bir döngüde iki tepe", [(0, 0), (100, 900), (150, 300), (250, 1200), (400, 0)], free=100)
    add("Sonda fazla kazı: döküm", [(0, 0), (100, 700), (200, 400)], free=40)
    add("Başta eksik: ödünç", [(0, 0), (80, -300), (200, 500), (300, 0)], free=50)
    add("Denge çizgisi kaydırılmış", [(0, 0), (100, 900), (200, 300), (300, 700)], b=300, free=70)
    add("Döküm alanı: basamak", [(0, 0), (100, 800), (100, 200), (200, 0)], free=30)
    add("Ödünç alanı: basamak", [(0, 0), (60, -400), (60, 0), (150, 500), (250, 0)], free=50)
    add("Sıfıra değen köşe", [(0, 0), (100, 500), (150, 0), (250, 600), (350, 0)], free=40)
    add("Kesirli istasyonlar", [("0", "0"), ("12.5", "137.25"), ("40.125", "-20.5"), ("75", "0")], free="7.5")
    add("Düz: hacim yok", [(0, 0), (100, 0), (200, 0)], free=10)
    add("İki nokta", [(0, 0), (50, 200)], free=10)
    rng = random.Random(907)
    for k in range(6):
        s, m, pts = 0, 0, [(0, 0)]
        for _ in range(rng.randint(5, 14)):
            s += rng.randint(5, 40)
            m += rng.randint(-400, 400)
            pts.append((s, m))
            if rng.random() < 0.15:
                m += rng.randint(-200, 200)
                pts.append((s, m))
        add(f"Rastgele {k + 1}", pts, b=rng.choice([0, 0, rng.randint(-200, 200)]), free=rng.choice([0, 25, 60]))
    return out


def main():
    data = {"format": "kentos.road-haul-cases", "version": 1,
            "note": "scripts/fixtures/road_haul_cases.py yazar: kütle eğrisi kesirlerle katman katman okunur; KentOS kodu yok.",
            "cases": cases()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text() != text:
            sys.exit(f"{OUT} is not current")
        print("road haul cases are current")
    else:
        OUT.write_text(text)
        print(f"written {OUT} ({len(data['cases'])} cases)")


if __name__ == "__main__":
    main()
