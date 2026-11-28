#!/usr/bin/env python3
"""Bir noktanın halkaların içinde olup olmadığının bağımsız başvurusu (CLAUDE.md §23.3, §23.4; ADR 9401 §9), KentOS kodu
olmadan.

    python3 scripts/fixtures/point_in_rings_cases.py           # fixtures/geometry/v1/point_in_rings.json'u yazar
    python3 scripts/fixtures/point_in_rings_cases.py --check   # yazılı dosya güncel mi

Kural (çift-tek): noktadan sağa giden yarı doğru bir halkanın kenarlarını tek sayıda keserse nokta o halkanın
içindedir; birden çok halkada (delikli alan, çok parçalı alan) içinde olduğu halkaların sayısı tekse içindedir. Bir
kenar, uçlarından biri noktanın yukarısında öbürü yukarısında değilse sayılır (`y > p.y`); kesişimin noktanın sağında
olup olmadığına koordinatların kesin değerleriyle karar verilir (`fractions`), yuvarlanmış bir kesişimle değil.

Bu kuralın sonucu: iki alanın ortak kenarının üstündeki nokta tam olarak birinin içindedir. Kesişimi kayan noktayla
hesaplayıp karşılaştıran yazım (depodaki yerel kopyalar böyleydi) çizim ölçeğindeki koordinatlarda bu noktaların bir
kısmını iki alana birden ya da hiçbirine koyar; her durumun `rounded` alanı o yazımın ne dediğini de kaydeder.

Durumlar: ortak kenarın orta noktası (her zaman tam kenarın üstünde; `fractions` ile denetlenir) çizim ölçeğinde ve
TM ölçeğinde, yatay ve düşey kenarlar, delikli alan, köşeden geçen yarı doğru. Rastgele sayılar sabit tohumdan gelir.
"""

from __future__ import annotations

import json
import random
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/geometry/v1/point_in_rings.json"


def in_ring(ring, p):
    """The rule, on exact values."""
    odd = False
    n = len(ring)
    for i in range(n):
        a, b = ring[i], ring[(i + 1) % n]
        if (a[1] > p[1]) != (b[1] > p[1]):
            side = (F(b[0]) - F(a[0])) * (F(p[1]) - F(a[1])) - (F(b[1]) - F(a[1])) * (F(p[0]) - F(a[0]))
            if (side > 0) if b[1] > a[1] else (side < 0):
                odd = not odd
    return odd


def in_rings(rings, p):
    return sum(1 for r in rings if in_ring(r, p)) % 2 == 1


def rounded_in_ring(ring, p):
    """The writing the rule replaces: the crossing computed in floating point, then compared."""
    odd = False
    n = len(ring)
    for i in range(n):
        a, b = ring[i], ring[(i + 1) % n]
        if (a[1] > p[1]) != (b[1] > p[1]) and p[0] < a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1]):
            odd = not odd
    return odd


def on_segment(a, b, p):
    return (F(p[0]) - F(a[0])) * (F(b[1]) - F(a[1])) == (F(p[1]) - F(a[1])) * (F(b[0]) - F(a[0]))


def pts(ring):
    return [{"x": x, "y": y} for x, y in ring]


def shared(name, a, b, reach=30.0):
    """Two areas on the two sides of a→b and the edge's midpoint."""
    p = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
    if not on_segment(a, b, p):
        return None
    lo, hi = min(a[0], b[0]) - reach, max(a[0], b[0]) + reach
    left = [a, b, (lo, b[1]), (lo, a[1])]
    right = [b, a, (hi, a[1]), (hi, b[1])]
    want = (in_ring(left, p), in_ring(right, p))
    assert want[0] != want[1], (name, want)
    return {
        "name": name, "p": {"x": p[0], "y": p[1]}, "left": pts(left), "right": pts(right),
        "expected": {"left": want[0], "right": want[1]},
        "rounded": {"left": rounded_in_ring(left, p), "right": rounded_in_ring(right, p)},
    }


def build():
    rnd = random.Random(20261128)
    edges = []
    # Drawing-sized coordinates (an architect's local drawing: metres with centimetres), the edge rising.
    wrong = kept = 0
    while wrong < 120 or kept < 60:
        ax, ay = round(rnd.uniform(0, 40), 2), round(rnd.uniform(0, 40), 2)
        a, b = (ax, ay), (round(ax + rnd.uniform(-9, 9), 2), round(ay + rnd.uniform(0.5, 9), 2))
        c = shared("çizim ölçeği", a, b)
        if c is None:
            continue
        differs = c["rounded"]["left"] == c["rounded"]["right"]
        if differs and wrong < 120:
            wrong += 1
            edges.append(c)
        elif not differs and kept < 60:
            kept += 1
            edges.append(c)
    # TM-sized coordinates.
    n = 0
    while n < 40:
        ax, ay = round(rnd.uniform(486000, 487000), 3), round(rnd.uniform(4420000, 4421000), 3)
        a, b = (ax, ay), (round(ax + rnd.uniform(-90, 90), 3), round(ay + rnd.uniform(5, 90), 3))
        c = shared("TM ölçeği", a, b, reach=300.0)
        if c is not None:
            n += 1
            edges.append(c)
    # A vertical and a horizontal shared edge: the area to the right, and the area above, hold the point.
    vertical = shared("düşey kenar", (5.0, 2.0), (5.0, 8.0))
    edges.append(vertical)
    low = [(0.0, 0.0), (10.0, 0.0), (10.0, 4.0), (0.0, 4.0)]
    high = [(0.0, 4.0), (10.0, 4.0), (10.0, 9.0), (0.0, 9.0)]
    p = (5.0, 4.0)
    want = (in_ring(low, p), in_ring(high, p))
    assert want == (False, True)
    edges.append({"name": "yatay kenar", "p": {"x": p[0], "y": p[1]}, "left": pts(low), "right": pts(high),
                  "expected": {"left": want[0], "right": want[1]},
                  "rounded": {"left": rounded_in_ring(low, p), "right": rounded_in_ring(high, p)}})
    # Rings together: an area with a hole, two parts, a ray through a corner.
    outer = [(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)]
    hole = [(5.0, 5.0), (5.0, 15.0), (15.0, 15.0), (15.0, 5.0)]
    part = [(30.0, 0.0), (40.0, 0.0), (40.0, 10.0), (30.0, 10.0)]
    diamond = [(0.0, 0.0), (5.0, 5.0), (0.0, 10.0), (-5.0, 5.0)]
    many = []
    for name, rings, points in [
        ("delikli alan", [outer, hole], [(2.0, 2.0), (10.0, 10.0), (5.0, 10.0), (15.0, 10.0), (25.0, 10.0), (10.0, 5.0)]),
        ("iki parça", [outer, part], [(10.0, 10.0), (35.0, 5.0), (25.0, 5.0), (20.0, 5.0), (30.0, 5.0)]),
        ("köşeden geçen yarı doğru", [diamond], [(-1.0, 5.0), (-6.0, 5.0), (1.0, 5.0), (0.0, 0.0), (0.0, 10.0), (6.0, 5.0)]),
    ]:
        for p in points:
            many.append({"name": name, "p": {"x": p[0], "y": p[1]}, "rings": [pts(r) for r in rings],
                         "expected": in_rings(rings, p)})
    both = sum(1 for c in edges if c["rounded"] == {"left": True, "right": True})
    neither = sum(1 for c in edges if c["rounded"] == {"left": False, "right": False})
    return {
        "format": "kentos.point-in-rings", "version": 1,
        "rule": "even-odd; an edge counts when one end is above the point and the other is not; whether its crossing "
                "is to the point's right is decided on exact values",
        "note": "Bağımsız başvuru: scripts/fixtures/point_in_rings_cases.py; elle düzenlemeyin. `rounded`: kesişimi "
                f"kayan noktayla hesaplayan yazımın sonucu ({both} durumda iki alan birden, {neither} durumda hiçbiri).",
        "sharedEdges": edges,
        "rings": many,
    }


def main() -> None:
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"{OUT.relative_to(ROOT)} güncel değil")
        data = json.loads(text)
        print(f"{len(data['sharedEdges'])} ortak kenar, {len(data['rings'])} çok halkalı durum güncel.")
        return
    OUT.write_text(text, encoding="utf-8")
    data = json.loads(text)
    print(f"{len(data['sharedEdges'])} ortak kenar, {len(data['rings'])} çok halkalı durum: {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
