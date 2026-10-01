#!/usr/bin/env python3
"""Independent reference of the display rule (docs/adr/0149, display v1).

Writes fixtures/numeric/v1/display.json from the rule alone, with Python's
standard library (`decimal`) and no KentOS code. The geometry core
(`kentos_geometry_core::display::fixed`, crates/shared/geometry-core/tests/display.rs)
and the web (`apps/web/src/core/displayNumber.ts`) must write exactly the
expected text for every case.

The rule, for a float v and d decimals:

1. NaN is "NaN", an infinity "Infinity" or "-Infinity".
2. x, the exact binary value of |v|, is rounded to 7 decimals, an exact
   half away from zero; that decimal is then rounded to d decimals, a half
   away from zero. With d of 7 or more, x is rounded once, to d decimals.
3. The digits are written in plain notation with exactly d decimals (none
   and no point when d is 0); a minus sign comes first when v is negative
   and a digit other than 0 is written.

    python3 scripts/fixtures/numeric_display.py           # write the fixture
    python3 scripts/fixtures/numeric_display.py --check   # compare
"""

import json
import math
import random
import sys
from decimal import ROUND_HALF_UP, Decimal, getcontext
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/numeric/v1/display.json"
NOISE_DECIMALS = 7

getcontext().prec = 200


def shown(v: float, d: int) -> str:
    if math.isnan(v):
        return "NaN"
    if math.isinf(v):
        return "Infinity" if v > 0 else "-Infinity"
    x = Decimal(abs(v))  # exact
    if d >= NOISE_DECIMALS:
        y = x.quantize(Decimal(1).scaleb(-d), rounding=ROUND_HALF_UP)
    else:
        y = x.quantize(Decimal(1).scaleb(-NOISE_DECIMALS), rounding=ROUND_HALF_UP)
        y = y.quantize(Decimal(1).scaleb(-d), rounding=ROUND_HALF_UP)
    text = format(y, "f")
    if v < 0 and any(c in "123456789" for c in text):
        text = "-" + text
    return text


def written(v: float) -> str:
    """The float as text both sides parse back to the same float."""
    if math.isnan(v):
        return "NaN"
    if math.isinf(v):
        return "Infinity" if v > 0 else "-Infinity"
    return repr(v)


def case(v: float, d: int, why: str = "") -> dict:
    out = {"v": written(v), "d": d, "expected": shown(v, d)}
    if why:
        out["why"] = why
    return out


def hand() -> list:
    c = []
    # A half at the digits shown, exact and a hair either side (a measure's noise at TM coordinates).
    for v, why in [
        (12.125, "yazılan yarım"),
        (12.1249999997, "yarımın 3·10⁻¹⁰ altı: gürültü"),
        (12.1250000003, "yarımın 3·10⁻¹⁰ üstü: gürültü"),
        (12.12499996, "yarımın 4·10⁻⁸ altı: hâlâ yarım"),
        (12.12499994, "yarımın 6·10⁻⁸ altı: aşağı"),
        (12.12500004, "yarımın 4·10⁻⁸ üstü: yarım"),
        (12.1249, "yarımdan uzak: aşağı"),
    ]:
        c.append(case(v, 2, why))
    # Typed values whose binary is just under a half: written as typed, rounded as on paper.
    for v, d in [(5.0005, 3), (7.8465, 3), (1.005, 2), (2.675, 2), (0.125, 2), (0.375, 2), (10.0625, 3), (1.0049, 2)]:
        c.append(case(v, d, "yazılan değer"))
    for v in [0.5, 1.5, 2.5, -0.5, -1.5, -2.5]:
        c.append(case(v, 0, "tam sayıya yarım"))
    # A radius drawn with the mouse and its diameter: each rounded from its own value.
    c.append(case(7.8465123, 3, "yarıçap"))
    c.append(case(15.6930246, 3, "çap"))
    # Coordinates.
    for v in [487012.0625, 4420000.0005, 4420000.123456789, 487000.9995, 487000.99949, 4419999.99999996]:
        c.append(case(v, 3, "koordinat"))
    # Signs and zeros.
    for v, d in [(0.0, 3), (-0.0, 3), (-0.0001, 3), (-0.0005, 3), (-0.00049, 3), (-0.0006, 3), (-12.125, 2), (-0.004999997, 2)]:
        c.append(case(v, d, "işaret"))
    # Carries.
    for v, d in [(9.9995, 3), (99.995, 2), (0.9999999, 6), (999999.99995, 4), (399.99996, 4), (99.99995, 4)]:
        c.append(case(v, d, "taşma"))
    # Seven digits or more: once, from the exact value.
    for v, d in [(0.00390625, 7), (1 / 3, 9), (4420000.123456789, 9), (2.675, 8), (1.0, 12), (0.1, 12)]:
        c.append(case(v, d, "ince basamak"))
    for v in [float("nan"), float("inf"), float("-inf")]:
        c.append(case(v, 2, "sonlu değil"))
    # Areas in m², dönüm and ha.
    for v, d in [(1234.5675, 2), (1.2345675, 4), (0.12345675, 4), (99999.995, 2)]:
        c.append(case(v, d, "alan"))
    return c


def drawn() -> list:
    rnd = random.Random(20261001)
    out = []
    # Values of every size the tools show.
    for _ in range(600):
        e = rnd.uniform(-6, 7)
        v = rnd.choice([1, -1]) * 10**e
        out.append(case(v, rnd.randint(0, 12)))
    # Halves at the digits shown, with the noise a measure carries, either side.
    for _ in range(600):
        d = rnd.randint(0, 6)
        k = rnd.randint(0, 5_000_000)
        whole = Decimal(k) / Decimal(10) ** d + Decimal(5) / Decimal(10) ** (d + 1)
        noise = rnd.choice([0.0, 3e-10, -3e-10, 1e-9, -1e-9, 2e-8, -2e-8, 4.9e-8, -4.9e-8, 5.1e-8, -5.1e-8, 1e-6, -1e-6])
        v = float(whole) + noise
        out.append(case(rnd.choice([1, -1]) * v, d))
    # Coordinates at TM sizes.
    for _ in range(300):
        v = rnd.uniform(400_000, 600_000) if rnd.random() < 0.5 else rnd.uniform(4_100_000, 4_700_000)
        out.append(case(v, rnd.randint(0, 6)))
    return out


def document() -> dict:
    return {
        "format": "kentos.display-fixtures",
        "version": 1,
        "rule": "display v1 (docs/adr/0149): 7 decimals, then the digits shown, a half away from zero; no sign on zero",
        "source": "python decimal (independent of KentOS code), scripts/fixtures/numeric_display.py",
        "cases": hand() + drawn(),
    }


def main() -> int:
    doc = document()
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    count = len(doc["cases"])
    if "--check" in sys.argv[1:]:
        if OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} kuraldan yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)")
            return 1
        print(f"{OUT.relative_to(ROOT)}: {count} durum kuralla tutarlı")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({count} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
