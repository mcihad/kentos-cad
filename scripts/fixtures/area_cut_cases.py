#!/usr/bin/env python3
"""Alanla böl (docs/adr/1069): the shared cases, written from the ADR's rules without KentOS code, at 50 digits with mpmath.

- The area is its outer ring (either way round, a closing repeat dropped). The reference edge runs from vertex i to
  the next; the piece asked for lies beside it, on the area's side.
- The cut is the line parallel to the edge at the distance d into the area where the ring's part on the edge's side of
  it has the target area. That area grows with d; here d is found by bisection on the clipped area itself (200 halvings
  at 50 digits), not by the core's closed form.
- The cut's ends are where that line meets the boundary (a vertex on the line counted once), in the edge's direction.
  More than two meetings would leave more than two pieces: refused.
- Refused also: fewer than three vertices or no area, an edge the ring has not, an edge of no length, a target not
  between zero and the ring's area.

The core runs the cases (crates/shared/geometry-core/tests/all/area_cut.rs).

    python3 scripts/fixtures/area_cut_cases.py          # write
    python3 scripts/fixtures/area_cut_cases.py --check  # compare
"""

import json
import sys
from pathlib import Path

import mpmath as mp

mp.mp.dps = 50
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "area-cut" / "v1" / "cases.json"
SOURCE = "scripts/fixtures/area_cut_cases.py (docs/adr/1069)"

TOO_FEW = "Alanla bölmek için en az üç köşeli, alanı olan kapalı bir alan gerekir."
NO_EDGE = "Bu alanda böyle bir kenar yok."
ZERO_EDGE = "Referans kenarın uzunluğu yok; başka bir kenar seçin."
TARGET = "Ayrılacak alan sıfırdan büyük ve alanın tamamından küçük olmalı."
MORE_PIECES = (
    "Bu alanı veren kesim çizgisi alanı ikiden çok parçaya böler; başka bir kenar seçin ya da Alan böl ile elle kesin."
)


def shoelace(pts):
    n = len(pts)
    return sum(pts[i][0] * pts[(i + 1) % n][1] - pts[(i + 1) % n][0] * pts[i][1] for i in range(n)) / 2


def project(ring, i):
    """The ring in the edge's frame (s along it, h into the area), and the frame."""
    n = len(ring)
    p0, p1 = ring[i], ring[(i + 1) % n]
    dx, dy = p1[0] - p0[0], p1[1] - p0[1]
    length = mp.sqrt(dx * dx + dy * dy)
    ux, uy = dx / length, dy / length
    turn = shoelace(ring)
    nx, ny = (-uy, ux) if turn > 0 else (uy, -ux)
    local = [((x - p0[0]) * ux + (y - p0[1]) * uy, (x - p0[0]) * nx + (y - p0[1]) * ny) for x, y in ring]
    return local, (p0, (ux, uy), (nx, ny))


def clipped_area(local, d):
    """The area of the part with h <= d, the ring walked edge by edge and cut where it crosses h = d."""
    out = []
    n = len(local)
    for k in range(n):
        p, q = local[k], local[(k + 1) % n]
        if p[1] <= d:
            out.append(p)
        if (p[1] <= d) != (q[1] <= d):
            t = (d - p[1]) / (q[1] - p[1])
            out.append((p[0] + t * (q[0] - p[0]), d))
    return abs(shoelace(out)) if len(out) >= 3 else mp.mpf(0)


def meetings(local, d):
    n = len(local)
    at = []
    for k in range(n):
        p, q = local[k], local[(k + 1) % n]
        if (p[1] <= d) != (q[1] <= d):
            t = (d - p[1]) / (q[1] - p[1])
            at.append(p[0] + t * (q[0] - p[0]))
    return sorted(at)


def solve(ring, edge, target):
    ring = [(mp.mpf(x), mp.mpf(y)) for x, y in ring]
    if len(ring) > 1 and ring[0] == ring[-1]:
        ring = ring[:-1]
    if len(ring) < 3 or shoelace(ring) == 0:
        return {"problem": TOO_FEW}
    if not 0 <= edge < len(ring):
        return {"problem": NO_EDGE}
    p0, p1 = ring[edge], ring[(edge + 1) % len(ring)]
    if p0 == p1:
        return {"problem": ZERO_EDGE}
    local, (o, u, nrm) = project(ring, edge)
    total = abs(shoelace(local))
    target = mp.mpf(target)
    if not (0 < target < total):
        return {"problem": TARGET}
    lo = min(h for _, h in local)
    hi = max(h for _, h in local)
    for _ in range(200):
        mid = (lo + hi) / 2
        if clipped_area(local, mid) < target:
            lo = mid
        else:
            hi = mid
    d = (lo + hi) / 2
    at = meetings(local, d)
    if len(at) != 2:
        return {"problem": MORE_PIECES}

    def world(s):
        return [float(o[0] + u[0] * s + nrm[0] * d), float(o[1] + u[1] * s + nrm[1] * d)]

    return {
        "a": world(at[0]),
        "b": world(at[1]),
        "distance": float(d),
        "near": float(target),
        "far": float(total - target),
    }


E, N = 487000, 4420000
SQUARE = [[0, 0], [10, 0], [10, 10], [0, 10]]
CASES = [
    ("Kare, alt kenar, 40 m²", SQUARE, 0, 40),
    ("Kare, sağ kenar, 40 m²", SQUARE, 1, 40),
    ("Kare, üst kenar, 40 m²", SQUARE, 2, 40),
    ("Kare, sol kenar, 40 m²", SQUARE, 3, 40),
    ("Kare saat yönünde, alt kenar", [[0, 0], [0, 10], [10, 10], [10, 0]], 3, 40),
    ("Kare, kapanış tekrarlı", SQUARE + [[0, 0]], 0, 25),
    ("Dikdörtgen 30×20, uzun kenar, 150 m²", [[0, 0], [30, 0], [30, 20], [0, 20]], 0, 150),
    ("Yamuk, geniş tabandan", [[0, 0], [40, 0], [30, 20], [10, 20]], 0, 300),
    ("Yamuk, dar üst kenardan", [[0, 0], [40, 0], [30, 20], [10, 20]], 2, 300),
    ("Üçgen, tabandan yarısı", [[0, 0], [20, 0], [5, 15]], 0, 75),
    ("Üçgen, yan kenardan küçük parça", [[0, 0], [20, 0], [5, 15]], 1, 10),
    ("Dışbükey beşgen, köşeyi aşan kesim", [[0, 0], [24, 3], [30, 18], [12, 28], [-4, 14]], 0, 420),
    ("Eğik dörtgen, eğik kenardan", [[2, 1], [27, 7], [21, 31], [-3, 22]], 1, 333.333),
    ("Kenar doğrultusunda ara köşe", [[0, 0], [5, 0], [10, 0], [10, 10], [0, 10]], 0, 60),
    ("Büyük koordinat, kare", [[E, N], [E + 10, N], [E + 10, N + 10], [E, N + 10]], 0, 40),
    ("Büyük koordinat, eğik alan", [[E + 2.31, N + 1.07], [E + 47.92, N + 9.55], [E + 41.6, N + 38.4], [E - 3.2, N + 30.1]], 0, 812.5),
    ("Çok küçük hedef", SQUARE, 0, 0.001),
    ("Toplama çok yakın hedef", SQUARE, 0, 99.999),
    ("İçbükey L, iki kesişimli", [[0, 0], [20, 0], [20, 10], [10, 10], [10, 20], [0, 20]], 0, 150),
    ("İçbükey U, dört kesişim", [[0, 0], [30, 0], [30, 20], [20, 20], [20, 10], [10, 10], [10, 20], [0, 20]], 0, 450),
    ("İçbükey U, kollara uzanmayan kesim", [[0, 0], [30, 0], [30, 20], [20, 20], [20, 10], [10, 10], [10, 20], [0, 20]], 0, 150),
    ("Hedef toplamdan büyük", SQUARE, 0, 120),
    ("Hedef toplama eşit", SQUARE, 0, 100),
    ("Hedef sıfır", SQUARE, 0, 0),
    ("Eksi hedef", SQUARE, 0, -5),
    ("Olmayan kenar", SQUARE, 4, 40),
    ("İki köşe", [[0, 0], [10, 0]], 0, 1),
    ("Alanı olmayan halka", [[0, 0], [10, 0], [20, 0]], 0, 1),
    ("Uzunluğu olmayan kenar", [[0, 0], [0, 0], [10, 0], [10, 10], [0, 10]], 0, 40),
]


def build():
    return {
        "source": SOURCE,
        "cases": [
            {"name": name, "ring": ring, "edge": edge, "target": target, "expect": solve(ring, edge, target)}
            for name, ring, edge, target in CASES
        ],
    }


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            sys.exit(f"{OUT} is out of date: python3 {Path(__file__).relative_to(ROOT)}")
        print("area cut cases: current")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    print(f"wrote {OUT.relative_to(ROOT)}: {len(CASES)} cases")


if __name__ == "__main__":
    main()
