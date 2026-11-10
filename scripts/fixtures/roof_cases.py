#!/usr/bin/env python3
"""A roof's faces and plan lines (docs/adr/8044 §2), from the ADR's rules, in exact fractions, without KentOS code.

Writes fixtures/arch/v1/roofs.json; `--check` compares and writes nothing. The shared core's test
(`kentos-geometry-core`, tests/all/roofs.rs) reads it and must give the same faces, lines and rise (within 1e-7).

The rules, as ADR 8044 §2 gives them:

- Eaves: the points in order, each edge from a point to the next (the last back to the first) with a slope in per
  cent or none (a gable end). Taken counter-clockwise: a clockwise line is reversed, its edges keeping their slopes.
- Plane: a sloping edge's rises from the edge's line into the roof (its left, counter-clockwise) at its slope: z =
  slope / 100 × the distance from the line.
- Face: for each sloping edge, the eaves where its plane is not above any other sloping edge's (≤ every other plane),
  when that has an area; in the edges' order as given. A line turning in anywhere is refused (not yet drawn).
- Lines: each edge as given, an eaves line if it slopes, else a gable end; then each face's edge not on the eaves, once,
  from its higher end: a hip if its lower end is a corner of the eaves (on a convex eaves line), else a ridge (level,
  or sloping between two of the roof's inner points).
- Rise: the highest of the faces' corners over the eaves.

    python3 scripts/fixtures/roof_cases.py          # writes the cases
    python3 scripts/fixtures/roof_cases.py --check  # compares with the file
"""

import argparse
import json
import sys
from fractions import Fraction as F
from math import isqrt
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/arch/v1/roofs.json"


def root(x):
    """The square root of a fraction that is a square of one."""
    n, d = x.numerator, x.denominator
    rn, rd = isqrt(n), isqrt(d)
    assert rn * rn == n and rd * rd == d, f"{x} is not a square"
    return F(rn, rd)


def P(x, y):
    return (F(x), F(y))


def area2(pts):
    n = len(pts)
    return sum(pts[i][0] * pts[(i + 1) % n][1] - pts[(i + 1) % n][0] * pts[i][1] for i in range(n))


def ccw(pts, slopes):
    n = len(pts)
    if area2(pts) >= 0:
        return list(pts), list(enumerate(slopes))
    ring = list(reversed(pts))
    return ring, [((2 * n - 2 - k) % n, slopes[(2 * n - 2 - k) % n]) for k in range(n)]


def plane(a, b, slope):
    dx, dy = b[0] - a[0], b[1] - a[1]
    length = root(dx * dx + dy * dy)
    nx, ny = -dy / length, dx / length
    s = F(slope) / 100
    return (-s * (a[0] * nx + a[1] * ny), s * nx, s * ny)


def at(p, q):
    return p[0] + p[1] * q[0] + p[2] * q[1]


def clip(ring, f):
    out = []
    n = len(ring)
    for i in range(n):
        p, q = ring[i], ring[(i + 1) % n]
        fp, fq = f(p), f(q)
        if fp >= 0:
            out.append(p)
        if (fp > 0 > fq) or (fp < 0 < fq):
            t = fp / (fp - fq)
            out.append((p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t))
    clean = []
    for p in out:
        if not clean or clean[-1] != p:
            clean.append(p)
    while len(clean) > 1 and clean[0] == clean[-1]:
        clean.pop()
    return clean


def convex(ring):
    n = len(ring)
    for i in range(n):
        a, b, c = ring[(i - 1) % n], ring[i], ring[(i + 1) % n]
        if (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]) < 0:
            return False
    return True


def planes_of(pts, slopes):
    ring, edges = ccw(pts, slopes)
    n = len(ring)
    return ring, [(i, s, plane(ring[k], ring[(k + 1) % n], s)) for k, (i, s) in enumerate(edges) if s]


def faces(pts, slopes):
    ring, planes = planes_of(pts, slopes)
    if not convex(ring):
        return None
    out = []
    for i, s, p in planes:
        face = list(ring)
        for j, _, q in planes:
            if j != i:
                face = clip(face, lambda x, q=q, p=p: at(q, x) - at(p, x))
                if len(face) < 3:
                    break
        if len(face) >= 3 and area2(face) > 0:
            out.append({"edge": i, "ring": face, "slope": F(s)})
    return sorted(out, key=lambda f: f["edge"])


def height(pts, slopes, q):
    _, planes = planes_of(pts, slopes)
    return min((at(p, q) for _, _, p in planes), default=F(0))


def on_ring(ring, p):
    n = len(ring)
    for i in range(n):
        a, b = ring[i], ring[(i + 1) % n]
        dx, dy = b[0] - a[0], b[1] - a[1]
        cross = (p[0] - a[0]) * dy - (p[1] - a[1]) * dx
        if cross != 0:
            continue
        t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)
        if 0 <= t <= 1:
            return True
    return False


def lines(pts, slopes):
    n = len(pts)
    out = [{"kind": "eaves" if slopes[i] else "gable", "a": pts[i], "b": pts[(i + 1) % n]} for i in range(n)]
    ring, _ = ccw(pts, slopes)
    inner = []
    for f in faces(pts, slopes):
        r = f["ring"]
        m = len(r)
        for k in range(m):
            a, b = r[k], r[(k + 1) % m]
            if a == b or on_ring(ring, ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)):
                continue
            if any({l["a"], l["b"]} == {a, b} for l in inner):
                continue
            za, zb = height(pts, slopes, a), height(pts, slopes, b)
            hi, lo = (a, b) if za >= zb else (b, a)
            inner.append({"kind": "hip" if za != zb and lo in ring else "ridge", "a": hi, "b": lo})
    return out + inner


def rise(pts, slopes):
    return max((height(pts, slopes, q) for f in faces(pts, slopes) for q in f["ring"]), default=F(0))


def cases():
    rect = [P(0, 0), P(12, 0), P(12, 9), P(0, 9)]
    out = [
        ("Kırma çatı: dikdörtgende dört yüz, mahya ve dört kırma", rect, [33, 33, 33, 33]),
        ("Beşik çatı: mahya kalkandan kalkana", rect, [40, None, 40, None]),
        ("Tek eğimli çatı: tek yüz, öbür kenarlar kalkan", rect, [10, None, None, None]),
        ("Düz çatı: yüz yok", rect, [None, None, None, None]),
        ("Ön %30, arka %45: mahya dik yüze doğru kayar", rect, [30, 33, 45, 33]),
        ("Saat yönünde verilen beşik: eğimler kenarlarıyla kalır",
         [P(0, 0), P(0, 9), P(12, 9), P(12, 0)], [None, 40, None, 40]),
        ("Kare kırma çatı: mahya yok, tepe noktası",
         [P(0, 0), P(8, 0), P(8, 8), P(0, 8)], [50, 50, 50, 50]),
        # A pentagon with 6-8-10 edges: its normals rational, its inner points apart (one with an incircle would meet
        # every hip at one apex).
        ("Beşgen kırma çatı: iç noktaları birleştiren eğik mahya",
         [P(0, 0), P(12, 0), P(12, 6), P(6, 14), P(0, 6)], [40, 40, 40, 40, 40]),
        ("Üçgen kırma çatı", [P(0, 0), P(8, 0), P(4, 3)], [30, 30, 30]),
    ]
    return out


def num(x):
    return float(x)


def pt(p):
    return [num(p[0]), num(p[1])]


def build():
    result = []
    for name, pts, slopes in cases():
        f = faces(pts, slopes)
        case = {"name": name, "pts": [pt(p) for p in pts], "slopes": [s for s in slopes],
                "expect": {"faces": [{"edge": x["edge"], "ring": [pt(p) for p in x["ring"]], "slope": num(x["slope"])}
                                     for x in f],
                           "lines": [{"kind": l["kind"], "a": pt(l["a"]), "b": pt(l["b"])} for l in lines(pts, slopes)],
                           "rise": num(rise(pts, slopes))}}
        result.append(case)
    return {"format": "kentos.roofs", "version": 1, "cases": result}


def main():
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--check", action="store_true")
    a = p.parse_args()
    doc = build()
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if a.check:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} başvuruyla aynı değil", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum başvuruyla aynı.")
        return 0
    OUT.write_text(text, "utf-8")
    print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
