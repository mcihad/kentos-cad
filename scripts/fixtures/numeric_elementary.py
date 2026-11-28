#!/usr/bin/env python3
"""The correctly rounded elementary functions' shared cases (docs/adr/9403 §2), without KentOS code.

Each case is a function, its argument or arguments and the double nearest to the exact value (a tie to the even one;
IEEE 754 roundTiesToEven). The value is worked out by mpmath at 320 bits and again at 640: the two must round to the
same double, or the case is refused (the hardest cases known of double precision need some 120 bits). An exact value
(a whole power, a tie) is rounded from an exact fraction instead. Neither the platform's C library nor any library
under test is asked.

What needs no arithmetic is written here from the standard's rules (IEEE 754-2019 §9.2.1, C99 Annex F): the zeros'
signs, the infinities, NaN, the arguments outside a function's domain.

The inputs are those a drawing gives and those that are hard: radians of decimal degrees and grads, bearings of many
turns, neighbours of π/2's multiples, arguments from 2⁻¹⁰⁷⁴ to 2¹⁰⁰⁰; differences of drawing and of millimetre
coordinates; results among the subnormals and next to the largest double; whole powers, a negative base's, exact ties.

  scripts/fixtures/py scripts/fixtures/numeric_elementary.py          # writes fixtures/numeric/v1/elementary.json
  scripts/fixtures/py scripts/fixtures/numeric_elementary.py --check  # fails when the file differs
"""

from __future__ import annotations

import json
import math
import struct
import sys
from fractions import Fraction
from pathlib import Path

import mpmath

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/numeric/v1/elementary.json"
LARGEST = sys.float_info.max
INF, NAN = math.inf, math.nan


def bits(x: float) -> str:
    return f"{struct.unpack('<Q', struct.pack('<d', x))[0]:016x}"


def from_bits(b: int) -> float:
    return struct.unpack("<d", struct.pack("<Q", b))[0]


def nearest_fraction(q: Fraction) -> float:
    """The double nearest to an exact fraction (ties to even), an infinity past the largest double's upper middle."""
    top = Fraction(LARGEST) + (Fraction(LARGEST) - Fraction(math.nextafter(LARGEST, 0.0))) / 2
    if abs(q) >= top:
        return math.copysign(INF, q)
    if q == 0:
        return 0.0
    return q.numerator / q.denominator  # Python's division of two integers rounds once, also among the subnormals


def nearest(v) -> float:
    """The double nearest to an mpf."""
    sign, man, exp, _ = v._mpf_
    if not man:
        return -0.0 if sign else 0.0
    q = Fraction(int(man)) * Fraction(2) ** int(exp)
    return nearest_fraction(-q if sign else q)


def _eval(ctx, name: str, x: float, y: float | None):
    X = ctx.mpf(x)
    if name == "atan2":
        return ctx.atan2(X, ctx.mpf(y))
    if name == "pow":
        return ctx.power(X, ctx.mpf(y))
    if name == "exp2":
        return ctx.power(2, X)
    if name == "ln":
        return ctx.log(X)
    if name == "log2":
        return ctx.log(X) / ctx.log(2)
    if name == "log10":
        return ctx.log(X) / ctx.log(10)
    if name == "cbrt":
        return ctx.cbrt(X) if x >= 0 else -ctx.cbrt(-X)
    if name == "lgamma":
        return ctx.log(abs(ctx.gamma(X)))
    return getattr(ctx, name)(X)


_LOW, _HIGH = mpmath.MPContext(), mpmath.MPContext()
_LOW.prec, _HIGH.prec = 320, 640


def rounded(name: str, x: float, y: float | None = None) -> float:
    """f(x) or f(x, y) correctly rounded, for arguments where the value is not exact."""
    a, b = nearest(_eval(_LOW, name, x, y)), nearest(_eval(_HIGH, name, x, y))
    if a != b and not (a != a and b != b):
        raise AssertionError(f"{name}({x!r}, {y!r}): {a!r} at 320 bits, {b!r} at 640: an exact value or a tie?")
    # The sign of a zero result is the exact value's.
    return a


class Sequence:
    """A fixed sequence (a linear congruential step, Knuth's MMIX constants): the same cases on every run."""

    def __init__(self, seed: int) -> None:
        self.seed = seed

    def next(self) -> int:
        self.seed = (self.seed * 6364136223846793005 + 1442695040888963407) & 0xFFFFFFFFFFFFFFFF
        return self.seed

    def unit(self) -> float:
        return (self.next() >> 11) / float(1 << 53)

    def between(self, a: float, b: float) -> float:
        return a + (b - a) * self.unit()

    def sign(self) -> float:
        return 1.0 if self.unit() < 0.5 else -1.0

    def log_between(self, a: float, b: float) -> float:
        """Between a and b (positive), evenly in the exponent: a power of two times a fraction, no library's exp."""
        lo, hi = math.frexp(a)[1], math.frexp(b)[1]
        e = lo + int(self.unit() * (hi - lo + 1))
        v = math.ldexp(0.5 + self.unit() / 2, e)
        return min(max(v, a), b)

    def subnormal(self) -> float:
        return from_bits((self.next() >> 12) >> int(self.unit() * 40)) * self.sign()


CASES: list[list] = []


def add(kind: str, name: str, x: float, y: float | None = None, expected: float | None = None) -> None:
    if expected is None:
        expected = rounded(name, x, y)
    want = "nan" if expected != expected else bits(expected)
    CASES.append([kind, name, bits(x)] + ([bits(y)] if y is not None else []) + [want])


def angles(q: Sequence) -> list[tuple[str, float]]:
    out: list[tuple[str, float]] = []
    out += [("açı", q.between(-7.0, 7.0)) for _ in range(120)]
    out += [("ondalık derece", round(q.between(-720, 720), 4) * math.pi / 180.0) for _ in range(120)]
    out += [("grad", round(q.between(-800, 800), 4) * math.pi / 200.0) for _ in range(120)]
    out += [("çok tur", q.between(-1e6, 1e6)) for _ in range(80)]
    for k in list(range(-12, 13)) + [57, 355, 1001, 52174, 104348, 5419351, 80143857]:
        b = k * math.pi / 2.0
        out += [("π/2'nin katı", b), ("π/2'nin katı", math.nextafter(b, INF)), ("π/2'nin katı", math.nextafter(b, -INF))]
    out += [("küçük", s * math.ldexp(1.0, e)) for e in range(-300, 0, 12) for s in (1.0, -1.0)]
    out += [("büyük", 1.2345678901234567 * math.ldexp(1.0, e)) for e in range(20, 1010, 30)]
    out += [("subnormal", q.subnormal()) for _ in range(20)]
    return [(kind, x) for kind, x in out if x != 0.0]


def build() -> None:
    q = Sequence(0x9E3779B97F4A7C15)
    # ── Trigonometry ──────────────────────────────────────────────────────────────────────────────────────────
    for kind, x in angles(q):
        for name in ("sin", "cos", "tan"):
            add(kind, name, x)
    for name in ("asin", "acos"):
        for _ in range(260):
            add("aralık", name, q.between(-1.0, 1.0))
        for k in range(1, 53, 3):
            add("1'e yakın", name, 1.0 - math.ldexp(1.0, -k))
            add("1'e yakın", name, -(1.0 - math.ldexp(1.0, -k)))
        for _ in range(16):
            add("subnormal", name, q.subnormal())
    for _ in range(260):
        add("oran", "atan", q.log_between(1e-6, 1e6) * q.sign())
    for e in range(-1070, 1020, 60):
        add("uç", "atan", 1.75 * math.ldexp(1.0, e))
    for _ in range(220):
        add("çizim", "atan2", q.between(-1e3, 1e3), q.between(-1e3, 1e3))
    for _ in range(220):
        add("harita", "atan2", round(q.between(-5e3, 5e3), 3), round(q.between(-5e3, 5e3), 3))
    for _ in range(40):
        add("subnormal", "atan2", q.subnormal(), q.between(0.5, 8.0))
    for y in (1.0, -1.0, 3.5, -3.5):
        for x in (1.0, -1.0, 3.5, -3.5, 1e-300, -1e-300, 1e300, -1e300):
            add("çeyrek", "atan2", y, x)
    # ── Exponentials and logarithms ───────────────────────────────────────────────────────────────────────────
    for _ in range(260):
        add("üs", "exp", q.between(-30.0, 30.0))
    for _ in range(70):
        add("subnormal sonuç", "exp", q.between(-745.1, -708.4))
    for _ in range(40):
        add("en büyüğe yakın", "exp", q.between(700.0, 709.78))
    for e in range(-1070, -1, 50):
        add("küçük", "exp", 1.5 * math.ldexp(1.0, e))
        add("küçük", "exp", -1.5 * math.ldexp(1.0, e))
    for _ in range(220):
        add("üs", "exp2", q.between(-60.0, 60.0))
    for _ in range(60):
        add("subnormal sonuç", "exp2", q.between(-1074.9, -1022.0))
    for k in list(range(-1074, -1060)) + list(range(-5, 6)) + list(range(1010, 1024)):
        add("tam", "exp2", float(k), expected=nearest_fraction(Fraction(2) ** k))
    for name in ("ln", "log2", "log10"):
        for _ in range(260):
            add("değer", name, q.log_between(1e-9, 1e12))
        for k in range(1, 53, 4):
            add("1'e yakın", name, 1.0 + math.ldexp(1.0, -k))
            add("1'e yakın", name, 1.0 - math.ldexp(1.0, -k))
        for _ in range(30):
            add("subnormal", name, abs(q.subnormal()) or 5e-324)
    for k in (-1074, -1022, -52, -1, 1, 2, 10, 52, 1023):
        add("tam", "log2", math.ldexp(1.0, k), expected=float(k))
    for k in range(1, 23):
        add("tam", "log10", float(10 ** k), expected=float(k))
    for _ in range(260):
        add("değer", "cbrt", q.log_between(1e-9, 1e12) * q.sign())
    for k in (2, 3, 7, 10, 101, 4097, 208063):
        add("tam", "cbrt", float(k ** 3), expected=float(k))
        add("tam", "cbrt", -float(k ** 3), expected=-float(k))
    for _ in range(30):
        add("subnormal", "cbrt", q.subnormal())
    # ── Powers ────────────────────────────────────────────────────────────────────────────────────────────────
    for _ in range(200):
        add("kuvvet", "pow", q.log_between(1e-3, 1e3), (0.5, 1.5, 2.5, 1.0 / 3.0, -0.5, 0.1, 1.2, 7.0 / 3.0)[q.next() % 8])
    for _ in range(200):
        add("kuvvet", "pow", q.log_between(1e-3, 1e3), q.between(-6.0, 6.0))
    for _ in range(80):
        # A whole exponent is exact: rounded from the fraction.
        x, n = round(q.between(-40.0, 40.0), 2) or 1.5, (q.next() % 13) - 6
        if n:
            add("tam üs", "pow", x, float(n), expected=nearest_fraction(Fraction(x) ** n))
    made = 0
    while made < 30:
        # (k²)^1.5 = k³, odd and of 54 bits: the exact value is the middle of two doubles.
        k = (q.next() % 2000) * 2 + 208065
        if (1 << 53) < k ** 3 < (1 << 54):
            add("eşit uzaklık", "pow", float(k * k), 1.5, expected=nearest_fraction(Fraction(k ** 3)))
            made += 1
    # sⁿ odd and of 54 bits: a whole power, the power of a square root and of a fourth root, a negative base, and
    # the same times powers of two (down among the subnormals, where the middle is another bit's).
    for s in range(3, 100, 2):
        n = 2
        while s ** n < (1 << 54):
            if s ** n > (1 << 53):
                exact = Fraction(s ** n)
                add("eşit uzaklık", "pow", float(s), float(n), expected=nearest_fraction(exact))
                if n % 2:
                    add("eşit uzaklık", "pow", -float(s), float(n), expected=nearest_fraction(-exact))
                    add("eşit uzaklık", "pow", float(s * s), n / 2, expected=nearest_fraction(exact))
                    if s ** 4 < (1 << 53):
                        add("eşit uzaklık", "pow", float(s ** 4), n / 4, expected=nearest_fraction(exact))
                for j in (-3, 8, -40):
                    add("eşit uzaklık", "pow", math.ldexp(float(s), j), float(n),
                        expected=nearest_fraction(exact * Fraction(2) ** (j * n)))
            n += 1
    for s, n, j in ((3, 20, -53), (3, 33, -32), (5, 22, -47), (3, 34, -33), (7, 19, -57)):
        # sⁿ·2^(j·n) under the smallest normal double: rounded at the subnormals' last place.
        exact = Fraction(s ** n) * Fraction(2) ** (j * n)
        add("subnormal sonuç", "pow", math.ldexp(float(s), j), float(n), expected=nearest_fraction(exact))
    for _ in range(60):
        # x^y = 2^t for a t among the subnormals' exponents: y = t / log2 x, the logarithm from mpmath (the platform's
        # would give another last place on another system, and another case).
        x = q.between(0.01, 0.99)
        add("subnormal sonuç", "pow", x, q.between(-1074.5, -1022.5) / float(_LOW.log(_LOW.mpf(x)) / _LOW.log(2)))
    for n in (-1076, -1075, -1074, -1073, -1023, -1022, 1023):
        add("tam üs", "pow", 2.0, float(n), expected=nearest_fraction(Fraction(2) ** n))
        add("tam üs", "pow", -2.0, float(n), expected=nearest_fraction(Fraction(-2) ** n))
    for _ in range(30):
        add("en büyüğe yakın", "pow", q.between(1.5, 10.0), q.between(300.0, 308.0) / 0.9542425094393249)
    # ── Hyperbolic functions, the error function, the gamma function ──────────────────────────────────────────
    for name in ("sinh", "cosh", "tanh"):
        for _ in range(180):
            add("aralık", name, q.between(-20.0, 20.0))
        for _ in range(80):
            add("geniş", name, q.log_between(1e-8, 700.0) * q.sign())
        for _ in range(16):
            add("subnormal", name, q.subnormal())
    for _ in range(260):
        add("geniş", "asinh", q.log_between(1e-8, 1e8) * q.sign())
    for _ in range(16):
        add("subnormal", "asinh", q.subnormal())
    for _ in range(260):
        add("aralık", "atanh", q.between(-1.0, 1.0))
    for k in range(1, 53, 4):
        add("1'e yakın", "atanh", 1.0 - math.ldexp(1.0, -k))
    for _ in range(16):
        add("subnormal", "atanh", q.subnormal())
    for _ in range(260):
        add("aralık", "erfc", q.between(-6.0, 26.0))
    for e in range(-1070, -1, 80):
        add("küçük", "erfc", 1.5 * math.ldexp(1.0, e))
    for _ in range(240):
        add("değer", "lgamma", q.log_between(1e-3, 1e4))
    for x in (0.5, 1.5, 2.5, 3.0, 4.0, 10.0, 100.0, 170.5, -0.5, -1.5, -2.5, -100.5):
        add("yarım ve tam", "lgamma", x)
    specials()


def specials() -> None:
    """What the standard fixes without arithmetic (IEEE 754-2019 §9.2.1; C99 Annex F.10)."""
    def of_pi(share: Fraction) -> float:
        """The double nearest to a share of π, from the two precisions."""
        a, b = (nearest(c.pi * c.mpf(share.numerator) / share.denominator) for c in (_LOW, _HIGH))
        assert a == b
        return a

    pi, half, quarter, three = (of_pi(Fraction(n, d)) for n, d in ((1, 1), (1, 2), (1, 4), (3, 4)))
    assert pi == math.pi

    def one(name: str, table: list[tuple[float, float]]) -> None:
        # A list, not a dict: 0.0 and −0.0 are one key.
        for x, want in table:
            add("özel", name, x, expected=want)
        add("özel", name, NAN, expected=NAN)

    odd = [(0.0, 0.0), (-0.0, -0.0)]
    one("sin", [*odd, (INF, NAN), (-INF, NAN)])
    one("cos", [(0.0, 1.0), (-0.0, 1.0), (INF, NAN), (-INF, NAN)])
    one("tan", [*odd, (INF, NAN), (-INF, NAN)])
    one("asin", [*odd, (1.0, half), (-1.0, -half), (1.0000000000000002, NAN), (-2.0, NAN), (INF, NAN), (-INF, NAN)])
    one("acos", [(1.0, 0.0), (-1.0, pi), (0.0, half), (-0.0, half), (1.0000000000000002, NAN), (-2.0, NAN), (INF, NAN)])
    one("atan", [*odd, (INF, half), (-INF, -half)])
    one("exp", [(0.0, 1.0), (-0.0, 1.0), (INF, INF), (-INF, 0.0), (710.0, INF), (-746.0, 0.0)])
    one("exp2", [(0.0, 1.0), (-0.0, 1.0), (INF, INF), (-INF, 0.0), (1024.0, INF), (-1076.0, 0.0)])
    for name in ("ln", "log2", "log10"):
        one(name, [(1.0, 0.0), (0.0, -INF), (-0.0, -INF), (-1.0, NAN), (-INF, NAN), (INF, INF)])
    one("cbrt", [*odd, (INF, INF), (-INF, -INF)])
    one("sinh", [*odd, (INF, INF), (-INF, -INF), (711.0, INF), (-711.0, -INF)])
    one("cosh", [(0.0, 1.0), (-0.0, 1.0), (INF, INF), (-INF, INF), (711.0, INF)])
    one("tanh", [*odd, (INF, 1.0), (-INF, -1.0)])
    one("asinh", [*odd, (INF, INF), (-INF, -INF)])
    one("atanh", [*odd, (1.0, INF), (-1.0, -INF), (1.0000000000000002, NAN), (-2.0, NAN), (INF, NAN)])
    one("erfc", [(0.0, 1.0), (-0.0, 1.0), (INF, 0.0), (-INF, 2.0), (28.0, 0.0)])
    one("lgamma", [(1.0, 0.0), (2.0, 0.0), (0.0, INF), (-0.0, INF), (-1.0, INF), (-2.0, INF), (INF, INF), (-INF, INF)])

    def two(name: str, rows: list[tuple[float, float, float]]) -> None:
        for a, b, want in rows:
            add("özel", name, a, b, expected=want)

    # atan2(y, x)
    two("atan2", [
        (0.0, 1.0, 0.0), (-0.0, 1.0, -0.0), (0.0, 0.0, 0.0), (-0.0, 0.0, -0.0), (0.0, -0.0, pi), (-0.0, -0.0, -pi),
        (0.0, -1.0, pi), (-0.0, -1.0, -pi), (1.0, 0.0, half), (-1.0, 0.0, -half), (1.0, -0.0, half),
        (-1.0, -0.0, -half), (1.0, INF, 0.0), (-1.0, INF, -0.0), (1.0, -INF, pi), (-1.0, -INF, -pi),
        (INF, 1.0, half), (-INF, 1.0, -half), (INF, -INF, three), (-INF, -INF, -three),
        (INF, INF, quarter), (-INF, INF, -quarter), (0.0, INF, 0.0), (-0.0, INF, -0.0), (0.0, -INF, pi),
        (NAN, 1.0, NAN), (1.0, NAN, NAN),
        # A quotient under the smallest double: a zero of y's sign.
        (-1e-320, 1e300, -0.0), (1e-320, 1e300, 0.0), (-5e-324, LARGEST, -0.0), (-1e-320, 1e22, -0.0),
    ])
    # pow(x, y)
    big_even, odd3, frac = 1e300, 3.0, math.pi
    two("pow", [
        (NAN, 0.0, 1.0), (NAN, -0.0, 1.0), (INF, 0.0, 1.0), (0.0, 0.0, 1.0), (-2.0, -0.0, 1.0),
        (1.0, NAN, 1.0), (1.0, INF, 1.0), (1.0, -INF, 1.0), (-1.0, INF, 1.0), (-1.0, -INF, 1.0),
        (NAN, 1.0, NAN), (2.0, NAN, NAN), (-1.0, NAN, NAN),
        (0.0, -odd3, INF), (-0.0, -odd3, -INF), (0.0, -2.0, INF), (-0.0, -2.0, INF), (-0.0, -frac, INF),
        (0.0, -INF, INF), (-0.0, -INF, INF), (0.0, INF, 0.0), (-0.0, INF, 0.0),
        (0.0, odd3, 0.0), (-0.0, odd3, -0.0), (-0.0, 2.0, 0.0), (-0.0, frac, 0.0), (-0.0, big_even, 0.0),
        (-0.0, LARGEST, 0.0), (-0.0, 1.0000000000000002, 0.0),
        (0.5, INF, 0.0), (-0.5, INF, 0.0), (2.0, INF, INF), (-2.0, INF, INF), (INF, INF, INF), (-INF, INF, INF),
        (0.5, -INF, INF), (-0.5, -INF, INF), (2.0, -INF, 0.0), (-2.0, -INF, 0.0), (-INF, -INF, 0.0),
        (INF, -1.0, 0.0), (INF, -frac, 0.0), (INF, 1.0, INF), (INF, frac, INF),
        (-INF, -odd3, -0.0), (-INF, odd3, -INF), (-INF, -2.0, 0.0), (-INF, -frac, 0.0), (-INF, 2.0, INF),
        (-INF, frac, INF), (-INF, big_even, INF), (-INF, LARGEST, INF), (-INF, 1.0000000000000002, INF),
        (-8.0, 1.0 / 3.0, NAN), (-2.0, 0.5, NAN), (-2.0, frac, NAN), (-1e-300, 1.5, NAN),
        (-2.0, 3.0, -8.0), (-2.0, 2.0, 4.0), (-2.0, -1.0, -0.5), (-3.0, big_even, INF), (-0.5, big_even, 0.0),
        (-3.0, 9007199254740991.0, -INF), (10.0, 400.0, INF), (10.0, -400.0, 0.0), (-10.0, 401.0, -INF),
        (-10.0, -401.0, -0.0),
    ])


def document() -> dict:
    CASES.clear()
    build()
    counts: dict[str, int] = {}
    for c in CASES:
        counts[c[1]] = counts.get(c[1], 0) + 1
    return {
        "format": "kentos.numeric-elementary",
        "version": 1,
        "note": "Temel işlevlerin en yakın çift duyarlıklı sayıları (eşit uzaklıkta çift olan; IEEE 754). Üreteç: "
                "scripts/fixtures/numeric_elementary.py (docs/adr/9403 §2): değer mpmath ile 320 ve 640 bitte hesaplanır, "
                "ikisi aynı sayıya yuvarlanmalıdır; tam değerler kesirden yuvarlanır; özel değerler standardın "
                "kurallarından yazılır. Her durum: tür, işlev, argüman(lar) ve beklenen değer, IEEE 754 bitleri "
                "onaltılık yazıyla; beklenen NaN ise \"nan\".",
        "counts": counts,
        "cases": CASES,
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
            print(f"{OUT.relative_to(ROOT)} güncel değil: scripts/fixtures/py {Path(__file__).relative_to(ROOT)} ile yeniden yazın.")
            return 1
        print(f"temel işlev durumları güncel: {len(CASES)} durum.")
        return 0
    OUT.write_text(new, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(CASES)} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
