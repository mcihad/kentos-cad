#!/usr/bin/env python3
"""A road's earthwork volumes' shared cases (docs/adr/0906, plan/road.md Y8.1), from an independent reference in exact
fractions: the average end area and prismoidal formulas; and between two surfaces given as triangles with rational
corners, each design triangle clipped by each ground triangle (Sutherland–Hodgman in fractions), the difference
d = z_design − z_ground split where it changes sign, each part's integral A·(d₀ + d₁ + d₂)/3 to the fill or the cut,
the design's area outside the ground apart. The plan's control example: a 10 × 10 m square, d = x − 5: 125 m³ cut and
125 m³ fill. No KentOS code.

    python3 scripts/fixtures/road_volume_cases.py
    ... --check   # fails when fixtures/road/v1/volume.json is not what this writes
"""

import json
import random
import sys
from fractions import Fraction as F
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "volume.json"


def area2(a, b, c):
    return (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])


def ccw(t):
    a = area2(t[0], t[1], t[2])
    if a == 0:
        return None
    return t if a > 0 else [t[0], t[2], t[1]]


def z_at(t, p):
    whole = area2(t[0], t[1], t[2])
    wa = area2(p, t[1], t[2]) / whole
    wb = area2(t[0], p, t[2]) / whole
    return wa * t[0][2] + wb * t[1][2] + (1 - wa - wb) * t[2][2]


def clip(poly, a, b):
    out = []
    for i, p in enumerate(poly):
        q = poly[(i + 1) % len(poly)]
        sp, sq = area2(a, b, p), area2(a, b, q)
        if sp >= 0:
            out.append(p)
        if (sp > 0 > sq) or (sp < 0 < sq):
            t = sp / (sp - sq)
            out.append((p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t))
    return out


def split(poly, above):
    keep = (lambda d: d >= 0) if above else (lambda d: d <= 0)
    out = []
    for i, (p, dp) in enumerate(poly):
        q, dq = poly[(i + 1) % len(poly)]
        if keep(dp):
            out.append((p, dp))
        if (dp > 0 > dq) or (dp < 0 < dq):
            t = dp / (dp - dq)
            out.append(((p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t), F(0)))
    return out


def integral(poly):
    a = v = F(0)
    for i in range(1, len(poly) - 1):
        s = area2(poly[0][0], poly[i][0], poly[i + 1][0]) / 2
        a += s
        v += s * (poly[0][1] + poly[i][1] + poly[i + 1][1]) / 3
    return a, v


def between(design, ground):
    ground = [g for g in (ccw([tuple(map(F, p)) for p in t]) for t in ground) if g]
    out = {k: F(0) for k in ("cut", "fill", "cutArea", "fillArea", "covered", "uncovered")}
    for t in design:
        d = ccw([tuple(map(F, p)) for p in t])
        if not d:
            continue
        whole = area2(d[0], d[1], d[2]) / 2
        covered = F(0)
        for g in ground:
            poly = [(p[0], p[1]) for p in d]
            for k in range(3):
                poly = clip(poly, g[k], g[(k + 1) % 3])
                if len(poly) < 3:
                    break
            if len(poly) < 3:
                continue
            w = [(p, z_at(d, p) - z_at(g, p)) for p in poly]
            a, _ = integral(w)
            if a <= 0:
                continue
            covered += a
            fa, fv = integral(split(w, True))
            ca, cv = integral(split(w, False))
            out["fill"] += fv
            out["fillArea"] += fa
            out["cut"] -= cv
            out["cutArea"] += ca
        out["covered"] += covered
        out["uncovered"] += max(whole - covered, F(0))
    return {k: float(v) for k, v in out.items()}


def square(x0, y0, x1, y1, z):
    """A rectangle as two triangles; z(x, y) gives each corner's height."""
    a, b, c, d = (x0, y0), (x1, y0), (x1, y1), (x0, y1)
    p = lambda q: [q[0], q[1], z(q[0], q[1])]  # noqa: E731
    return [[p(a), p(b), p(c)], [p(a), p(c), p(d)]]


def grid(x0, y0, step, n, m, z):
    out = []
    for i in range(n):
        for j in range(m):
            out += square(x0 + i * step, y0 + j * step, x0 + (i + 1) * step, y0 + (j + 1) * step, z)
    return out


def fan(cx, cy, r, cz, rz):
    """A square around a raised centre: four triangles."""
    c = [cx, cy, cz]
    k = [[cx - r, cy - r, rz], [cx + r, cy - r, rz], [cx + r, cy + r, rz], [cx - r, cy + r, rz]]
    return [[c, k[i], k[(i + 1) % 4]] for i in range(4)]


def frac(v):
    return [[[str(F(x)) for x in p] for p in t] for t in v]


def cases():
    out = []
    flat = lambda x, y: 100  # noqa: E731
    out.append(("Kontrol örneği: d = x − 5", square(0, 0, 10, 10, lambda x, y: 100 + x - 5), square(0, 0, 10, 10, flat)))
    out.append(("Düz prizma: 2 m dolgu", square(0, 0, 10, 10, lambda x, y: 102), square(0, 0, 10, 10, flat)))
    out.append(("Düz prizma: 1,5 m kazı", square(0, 0, 10, 10, lambda x, y: 98.5), square(0, 0, 10, 10, flat)))
    out.append(("Eğik düzlem, köşegende sıfır", square(0, 0, 10, 10, lambda x, y: 100), square(0, 0, 10, 10, lambda x, y: 95 + F(x + y, 2))))
    out.append(("Yükseltilmiş merkezli zemin", square(0, 0, 10, 10, lambda x, y: 101), fan(5, 5, 5, 103, 100)))
    out.append(("Zemin yarısını örter", square(0, 0, 10, 10, lambda x, y: 101), square(0, 0, 5, 10, flat)))
    out.append(("Zeminde boşluk", grid(0, 0, 5, 2, 2, lambda x, y: 101), [t for t in grid(0, 0, 2, 5, 5, flat)
                                                                          if not (2 <= t[0][0] < 6 and 2 <= t[0][1] < 6)]))
    out.append(("Kayık iki ağ", grid(F(1, 3), F(1, 7), 3, 4, 4, lambda x, y: 100 + F(x - y, 4)),
                grid(0, 0, 2, 7, 7, lambda x, y: 99 + F(x, 5) + F(y * y, 40))))
    out.append(("Tasarımın dışındaki zemin sayılmaz", square(2, 2, 4, 4, lambda x, y: 100 + x - 3), grid(0, 0, 3, 3, 3, flat)))
    out.append(("Saat yönündeki üçgenler", [[t[0], t[2], t[1]] for t in square(0, 0, 6, 4, lambda x, y: 100 + F(y, 2))],
                [[t[0], t[2], t[1]] for t in square(0, 0, 6, 4, flat)]))
    rng = random.Random(7)
    for k in range(4):
        n = 3 + k
        hz = {(i, j): F(rng.randint(-300, 300), 100) for i in range(n + 1) for j in range(n + 1)}
        def g(x, y, hz=hz, n=n):
            return 100 + hz[(round(x / F(9, n)), round(y / F(9, n)))]
        ground = grid(0, 0, F(9, n), n, n, g)
        design = grid(F(1, 2), F(1, 3), F(8, n + 1), n + 1, n + 1, lambda x, y, k=k: 100 + F(k - 1, 2) + F(x, 9) - F(y, 13))
        out.append((f"Rastgele zemin {k + 1}", design, ground))
    return out


def section_cases():
    rows = [(0, (F(12), F(0))), (20, (F(8), F(3))), (40, None), (60, (F(0), F(9))), (75, (F(5, 2), F(7)))]
    got = []
    for (s0, a0), (s1, a1) in zip(rows, rows[1:]):
        if a0 is None or a1 is None:
            got.append({"from": s0, "to": s1})
        else:
            l = F(s1 - s0)
            got.append({"from": s0, "to": s1, "cut": float(l * (a0[0] + a1[0]) / 2), "fill": float(l * (a0[1] + a1[1]) / 2)})
    prism = [{"a0": 12, "am": 9, "a1": 4, "l": 20, "v": float(F(20) * (12 + 36 + 4) / 6)},
             {"a0": 0, "am": 3, "a1": 0, "l": 10, "v": float(F(10) * 12 / 6)},
             {"a0": 7.5, "am": 7.5, "a1": 7.5, "l": 12.5, "v": float(F(25, 2) * F(15, 2))}]
    return {"rows": [{"station": s, **({"cut": float(a[0]), "fill": float(a[1])} if a else {})} for s, a in rows],
            "between": got, "prismoidal": prism}


def make():
    return {"surfaces": [{"name": n, "design": frac(d), "ground": frac(g), **between(d, g)} for n, d, g in cases()],
            "sections": section_cases()}


def main():
    text = json.dumps(make(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            sys.exit(f"{OUT} is not what {Path(__file__).name} writes")
        print("volume cases are current")
        return
    OUT.write_text(text)
    print(f"written {OUT}")


if __name__ == "__main__":
    main()
