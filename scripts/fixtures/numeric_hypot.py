#!/usr/bin/env python3
"""The correctly rounded hypot's shared cases (docs/adr/9403 §2), without KentOS code.

hypot(x, y) is the double nearest to √(x² + y²), a tie to the even one (IEEE 754 roundTiesToEven). The decision is
made in exact rational arithmetic: the squares are summed as fractions, and a double c is the answer when the sum
lies between the squares of the two middles around c (the middle of c and the double below it, of c and the double
above it). No square root is taken to decide and no library's hypot is asked: the measurement that started this
(the ADR's table) found three of them missing the nearest double for one input in seven.

The inputs: lengths as a drawing has them (differences at drawing size and at map size with millimetre coordinates),
every range of exponents, exponents 25 to 30 apart (where the smaller side stops mattering), subnormals, values near
the largest double, whole triangles, and ties: triangles 3k, 4k, 5k whose longest side needs 54 bits and is odd.

  python3 scripts/fixtures/numeric_hypot.py          # writes fixtures/numeric/v1/hypot.json
  python3 scripts/fixtures/numeric_hypot.py --check  # fails when the file differs
"""

from __future__ import annotations

import json
import math
import struct
import sys
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/numeric/v1/hypot.json"
LARGEST = sys.float_info.max


def bits(x: float) -> str:
    return f"{struct.unpack('<Q', struct.pack('<d', x))[0]:016x}"


def even(x: float) -> bool:
    return struct.unpack("<Q", struct.pack("<d", x))[0] & 1 == 0


def holds(c: float, s: Fraction) -> bool:
    """Whether the non-negative double c is the nearest to √s, a tie to the even one."""
    if c == 0.0:
        above = Fraction(math.nextafter(0.0, 1.0)) / 2
        return s < above * above or (s == above * above)
    below = (Fraction(c) + Fraction(math.nextafter(c, 0.0))) / 2
    if c == LARGEST:
        # Past the largest double's upper middle the answer is infinite.
        above = Fraction(c) + (Fraction(c) - Fraction(math.nextafter(c, 0.0))) / 2
    else:
        above = (Fraction(c) + Fraction(math.nextafter(c, math.inf))) / 2
    lo, hi = below * below, above * above
    return (lo < s or (lo == s and even(c))) and (s < hi or (s == hi and even(c)))


def reference(x: float, y: float) -> float:
    """hypot(x, y), correctly rounded, for finite x and y."""
    s = Fraction(x) * Fraction(x) + Fraction(y) * Fraction(y)
    if s == 0:
        return 0.0
    top = Fraction(LARGEST) + (Fraction(LARGEST) - Fraction(math.nextafter(LARGEST, 0.0))) / 2
    if s >= top * top:
        return math.inf
    # A first guess from the integer square root of the sum scaled by an even power of two, then its neighbours.
    shift = 2 * max(0, 70 - (s.numerator.bit_length() - s.denominator.bit_length()) // 2)
    root = math.isqrt((s.numerator << shift) // s.denominator)
    guess = float(Fraction(root, 1 << (shift // 2)))
    for c in (guess, math.nextafter(guess, 0.0), math.nextafter(guess, math.inf)):
        if math.isfinite(c) and holds(c, s):
            return c
    raise AssertionError(f"no double holds for hypot({x!r}, {y!r})")


class Sequence:
    """A fixed sequence (a linear congruential step, Knuth's MMIX constants): the same cases on every run."""

    def __init__(self) -> None:
        self.seed = 0x9E3779B97F4A7C15

    def next(self) -> int:
        self.seed = (self.seed * 6364136223846793005 + 1442695040888963407) & 0xFFFFFFFFFFFFFFFF
        return self.seed

    def unit(self) -> float:
        return (self.next() >> 11) / float(1 << 53)

    def between(self, a: float, b: float) -> float:
        return a + (b - a) * self.unit()


def pairs() -> list[tuple[str, float, float]]:
    q = Sequence()
    out: list[tuple[str, float, float]] = []
    for a, b in ((3.0, 4.0), (5.0, 12.0), (8.0, 15.0), (0.0, 0.0), (0.0, 2.5), (-3.0, -4.0), (1.0, 1.0), (1e-8, 1.0)):
        out.append(("tam", a, b))
    out.append(("ölçümdeki girdi", 896.8095497694007, -294.2548835472934))
    for _ in range(1500):
        out.append(("çizim", q.between(-1e3, 1e3), q.between(-1e3, 1e3)))
    for _ in range(1500):
        # Differences of millimetre coordinates in a 3° Transverse Mercator zone.
        e1, n1 = 486512.34 + round(q.between(0, 1e3), 3), 4420187.52 + round(q.between(0, 1e3), 3)
        e2, n2 = 486512.34 + round(q.between(0, 1e3), 3), 4420187.52 + round(q.between(0, 1e3), 3)
        out.append(("harita", e2 - e1, n2 - n1))
    for _ in range(1200):
        a = struct.unpack("<d", struct.pack("<Q", q.next() & 0x7FEFFFFFFFFFFFFF))[0]
        k = int(q.unit() * 60) - 2
        b = a * 2.0 ** (-k) * q.between(0.5, 1.5)
        if math.isfinite(a) and math.isfinite(b):
            out.append(("her üs", a, b))
    for apart in range(24, 32):
        for _ in range(40):
            a = q.between(1.0, 2.0) * 2.0 ** int(q.between(-200, 200))
            out.append((f"üsler {apart} ayrı", a, a * 2.0 ** (-apart) * q.between(0.5, 2.0)))
    for _ in range(400):
        out.append(("subnormal", struct.unpack("<d", struct.pack("<Q", q.next() >> 12))[0],
                    struct.unpack("<d", struct.pack("<Q", q.next() >> (12 + int(q.unit() * 30))))[0]))
    for _ in range(200):
        out.append(("subnormal ve küçük", struct.unpack("<d", struct.pack("<Q", q.next() >> 12))[0],
                    q.between(1.0, 2.0) * 2.0 ** int(q.between(-1074, -1000))))
    for _ in range(300):
        out.append(("en büyüğe yakın", LARGEST * q.between(0.4, 1.0), LARGEST * q.between(0.0, 1.0)))
    made = 0
    while made < 300:
        # 3k, 4k, 5k with 5k odd and of 54 bits: the exact root is the middle of two doubles.
        k = (q.next() >> 13) | 1
        if 3 * k < (1 << 53) and 4 * k < (1 << 53) and (1 << 53) < 5 * k:
            scale = 2.0 ** int(q.between(-900, 900))
            out.append(("eşit uzaklık", 3.0 * k * scale, 4.0 * k * scale))
            made += 1
    return out


def document() -> dict:
    cases = []
    for kind, x, y in pairs():
        cases.append([kind, bits(x), bits(y), bits(reference(x, y))])
    kinds: dict[str, int] = {}
    for c in cases:
        kinds[c[0]] = kinds.get(c[0], 0) + 1
    return {
        "format": "kentos.numeric-hypot",
        "version": 1,
        "note": "hypot(x, y) = √(x² + y²)'nin en yakın çift duyarlıklı sayısı (eşit uzaklıkta çift olan; IEEE 754). "
                "Üreteç: scripts/fixtures/numeric_hypot.py (docs/adr/9403 §2); karar kesirlerle, karelerin tam "
                "karşılaştırmasıyla verilir, hiçbir kitaplığın hypot'u sorulmaz. Her durum: tür, x, y ve beklenen "
                "değer, IEEE 754 bitleri onaltılık yazıyla.",
        "counts": kinds,
        "cases": cases,
    }


def text() -> str:
    doc = document()
    head = json.dumps({k: v for k, v in doc.items() if k != "cases"}, ensure_ascii=False, indent=1)[:-2]
    rows = ",\n".join("  " + json.dumps(c, ensure_ascii=False) for c in doc["cases"])
    return f'{head},\n "cases": [\n{rows}\n ]\n}}\n'


def main() -> int:
    new = text()
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != new:
            print(f"{OUT.relative_to(ROOT)} güncel değil: python3 {Path(__file__).relative_to(ROOT)} ile yeniden yazın.")
            return 1
        print(f"hypot durumları güncel: {len(document()['cases'])} durum.")
        return 0
    OUT.write_text(new, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(document()['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
