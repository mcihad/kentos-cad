#!/usr/bin/env python3
"""Where a sample line set takes its sections and where its lines cross, shared cases (docs/adr/0933 §1, plan/road.md
Y29.1), from an independent reference. No KentOS code:

- the simplification is Douglas–Peucker on (station, height) in exact fractions, written from its statement: between
  two kept stations every break's height is within the tolerance of the chord; the farthest break (the first of equal
  ones) is kept while it is not;
- a gap longer than `most` gets, walking from its start, the last break within `most` of the station kept before, or
  the first past it;
- two square lines' meeting is solved at 40 digits on road_alignment_cases.py's alignment (mpmath), each normal from
  the element's own direction; the clip takes the meetings nearest first and shortens both lines where both reach.

    uv run --no-project --python 3.11 --with mpmath python3 scripts/fixtures/road_section_stations_cases.py   (--check)
"""

import json
import random
import sys
from fractions import Fraction as F
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_alignment_cases as al  # noqa: E402

mp.mp.dps = 40
OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "section-stations.json"
TOUCH = mp.mpf("1e-9")


def douglas(profile, tol):
    pts = [(F(s), F(z)) for s, z in profile]
    n = len(pts)
    if n <= 2:
        return list(range(n))
    keep = {0, n - 1}
    tol = F(tol)

    def split(i, j):
        if j <= i + 1:
            return
        (s0, z0), (s1, z1) = pts[i], pts[j]
        worst, at = F(0), None
        for k in range(i + 1, j):
            s, z = pts[k]
            line = z0 + (z1 - z0) * (s - s0) / (s1 - s0) if s1 != s0 else z0
            d = abs(z - line)
            if d > worst:
                worst, at = d, k
        if at is not None and worst > tol:
            keep.add(at)
            split(i, at)
            split(at, j)

    split(0, n - 1)
    return sorted(keep)


def gaps(profile, kept, most):
    st = [F(s) for s, _ in profile]
    most = F(most)
    out = [kept[0]]
    for a, b in zip(kept, kept[1:]):
        last = st[a]
        k = a + 1
        while k < b:
            nxt = st[k + 1] if k + 1 < len(st) else None
            if st[b] - last > most and (nxt is None or nxt - last > most):
                out.append(k)
                last = st[k]
            k += 1
        out.append(b)
    return out


def simplify_case(name, profile, tol, most=None):
    kept = douglas(profile, tol)
    if most:
        kept = gaps(profile, kept, most)
    c = {"name": name, "profile": [[float(s), float(z)] for s, z in profile], "tolerance": tol, "kept": kept}
    if most:
        c["most"] = most
    return c


def normal(els, s):
    e, p = al.place_at(els, mp.mpf(s))
    th = e.theta(mp.mpf(s) - e.station)
    return p, [-mp.sin(th), mp.cos(th)]


def meet(els, stations, widths):
    lines = [normal(els, s) for s in stations]
    out = []
    for i in range(len(stations)):
        p, n = lines[i]
        for j in range(i + 1, len(stations)):
            q, m = lines[j]
            det = n[0] * (-m[1]) - n[1] * (-m[0])
            if abs(det) < mp.mpf("1e-12"):
                continue
            d = [q[0] - p[0], q[1] - p[1]]
            t = (d[0] * (-m[1]) - d[1] * (-m[0])) / det
            u = (n[0] * d[1] - n[1] * d[0]) / det

            def within(x, w):
                # Lines that only touch (a clipped line ends at the meeting) do not cross: 1e-9 m.
                return TOUCH < x < w[0] - TOUCH or (x < -TOUCH and -x < w[1] - TOUCH)

            if t * u > 0 and within(t, widths[i]) and within(u, widths[j]):
                out.append({"first": i, "second": j, "left": bool(t > 0), "atFirst": abs(t), "atSecond": abs(u)})
    return out


def clip(found, widths):
    w = [[mp.mpf(a), mp.mpf(b)] for a, b in widths]
    # Nearest first by the micrometre, then the lines' order (on a circle every pair meets at its centre).
    for c in sorted(found, key=lambda c: (int(mp.nint(min(c["atFirst"], c["atSecond"]) * 10**6)), c["first"], c["second"])):
        k = 0 if c["left"] else 1
        if w[c["first"]][k] > c["atFirst"] + TOUCH and w[c["second"]][k] > c["atSecond"] + TOUCH:
            w[c["first"]][k] = c["atFirst"]
            w[c["second"]][k] = c["atSecond"]
    return w


def cross_case(name, pis, stations, widths):
    els, length, _, _ = al.build(pis)
    found = meet(els, stations, widths)
    clipped = clip(found, widths)
    # The clipped widths meet nowhere: the reference's own check.
    assert not meet(els, stations, [[float(a), float(b)] for a, b in clipped]), name
    return {"name": name, "pis": pis, "stations": stations, "widths": widths,
            "crossings": [{**c, "atFirst": float(c["atFirst"]), "atSecond": float(c["atSecond"])} for c in found],
            "clipped": [[float(a), float(b)] for a, b in clipped]}


def main():
    rnd = random.Random(29)
    simplify = []
    flat = [(F(i * 7), F(100)) for i in range(8)]
    simplify.append(simplify_case("Düz arazi: yalnız uçlar", flat, 0.01))
    ridge = [(F(0), F(100)), (F(13), F(101)), (F(27), F(104)), (F(40), F(103)), (F(55), F(100))]
    simplify.append(simplify_case("Sırt, 1 m: tepe kalır", ridge, 1.0))
    simplify.append(simplify_case("Sırt, sıfır tolerans: hepsi kalır", ridge, 0.0))
    simplify.append(simplify_case("Sırt, büyük tolerans: yalnız uçlar", ridge, 10.0))
    line = [(F(i * 5), F(100) + F(i * 5) / 50) for i in range(10)]
    simplify.append(simplify_case("Sabit eğim: yalnız uçlar", line, 0.001))
    simplify.append(simplify_case("Sabit eğim, en çok 12 m aralık", line, 0.001, 12.0))
    simplify.append(simplify_case("Sabit eğim, en çok 5 m: hepsi", line, 0.001, 5.0))
    simplify.append(simplify_case("Tek nokta", [(F(0), F(50))], 0.1))
    simplify.append(simplify_case("İki nokta", [(F(0), F(50)), (F(20), F(51))], 0.1))
    step = [(F(0), F(10)), (F(10), F(10)), (F(10.5), F(12)), (F(30), F(12))]
    simplify.append(simplify_case("Basamak: iki köşesi de kalır", step, 0.05))
    for k, (n, tol) in enumerate([(40, 0.1), (120, 0.05), (300, 0.2), (80, 0.02)]):
        s, z, prof = F(0), F(1278), []
        for _ in range(n):
            prof.append((s, z))
            s += F(rnd.randint(30, 400), 100)
            z += F(rnd.randint(-60, 60), 100)
        simplify.append(simplify_case(f"Rastgele arazi {k + 1}: {n} kırık, tolerans {tol} m", prof, tol))
        if k < 2:
            simplify.append(simplify_case(f"Rastgele arazi {k + 1}, en çok 15 m", prof, tol, 15.0))

    cross = []
    curve = [al.pi("a", 0, 0), al.pi("p", 200, 0, 60, 0, 0), al.pi("b", 200 + 200 * 0.5, 200 * 0.8660254037844386)]
    spiral = [al.pi("a", 0, 0), al.pi("p", 200, 0, 45, 30, 30), al.pi("b", 260, 190)]
    tight = [al.pi("a", 0, 0), al.pi("p", 100, 0, 15, 0, 0), al.pi("b", 100, 100)]
    straight = [al.pi("a", 0, 0), al.pi("b", 300, 40)]
    left = [al.pi("a", 0, 0), al.pi("p", 200, 0, 60, 0, 0), al.pi("b", 300, -170)]
    els, length, reports, _ = al.build(curve)
    ts, st = float(reports[0]["stations"][0]), float(reports[0]["stations"][3])

    def every(a, b, d):
        out, s = [], a
        while s <= b + 1e-9:
            out.append(round(s, 6))
            s += d
        return out

    sts = every(ts - 20, st + 20, 10)
    cross.append(cross_case("Daire R 60: 30 m kol kesişmez", curve, sts, [[30.0, 30.0]] * len(sts)))
    cross.append(cross_case("Daire R 60: 80 m iç kol merkezde kesişir", curve, sts, [[80.0, 20.0]] * len(sts)))
    cross.append(cross_case("Daire R 60: dış kol uzun da olsa kesişmez", curve, sts, [[20.0, 200.0]] * len(sts)))
    cross.append(cross_case("Sağa dönen kurp: iç yan sağda", left, every(150, 300, 15), [[20.0, 90.0]] * 11))
    els2, _, rep2, _ = al.build(spiral)
    a2, b2 = float(rep2[0]["stations"][0]), float(rep2[0]["stations"][3])
    s2 = every(a2 - 10, b2 + 10, 7)
    cross.append(cross_case("Klotoidli kurp: evolütün yakınında", spiral, s2, [[70.0, 15.0]] * len(s2)))
    cross.append(cross_case("Klotoidli kurp, 40 m: yalnız dairede", spiral, s2, [[40.0, 15.0]] * len(s2)))
    els3, _, rep3, _ = al.build(tight)
    a3, b3 = float(rep3[0]["stations"][0]), float(rep3[0]["stations"][3])
    s3 = every(a3 - 5, b3 + 5, 4)
    cross.append(cross_case("Dar kurp R 15, 25 m kollar", tight, s3, [[25.0, 25.0]] * len(s3)))
    w3 = [[10.0 + (i % 3) * 8, 25.0] for i in range(len(s3))]
    cross.append(cross_case("Dar kurp, farklı genişlikler", tight, s3, w3))
    cross.append(cross_case("Düz aks: hiç kesişme yok", straight, every(0, 300, 20), [[100.0, 100.0]] * 16))
    cross.append(cross_case("Kurpta tek kesit", curve, [ts + 10], [[200.0, 200.0]]))
    cross.append(cross_case("Uzak kesitler aynı merkezde", curve, [ts + 5, st - 5], [[100.0, 10.0], [100.0, 10.0]]))
    cross.append(cross_case("Biri kısa: kısa olan merkeze ulaşmaz", curve, [ts + 5, ts + 15, ts + 25],
                            [[100.0, 10.0], [50.0, 10.0], [100.0, 10.0]]))

    text = json.dumps({"simplify": simplify, "crossings": cross}, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            sys.exit(f"{OUT} is not what the reference writes")
        return
    OUT.write_text(text)
    print(f"{OUT}: {len(simplify)} + {len(cross)} durum")


if __name__ == "__main__":
    main()
