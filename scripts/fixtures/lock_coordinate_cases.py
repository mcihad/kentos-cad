#!/usr/bin/env python3
"""Independent reference of the coordinate locks (docs/adr/0267, plan/cad.md CAD-INPUT-03).

Writes fixtures/locks/v1/coordinates.json from the ADR's rule alone, with Python's standard library (exact fractions;
the square root with `decimal` at 60 digits), no KentOS code. The geometry core
(crates/shared/geometry-core/tests/all/locks.rs) must give the same points within 1e-8 m.

The rule (§1), in the frame the locks are in (the drawing's UCS, else the world), with o the reference and c the cursor:

- X and Y locked: (X, Y), whatever else.
- X locked and a length L: the points of the circle about o of radius L on the line x = X; the one nearer the cursor's
  y (the upper one when the cursor is level with o); none when |X − o.x| > L.
- X locked alone: (X, c.y).
- Y the same way with the axes swapped (the right one nearer the cursor's x).
- A direction lock is never held together with a coordinate lock (locking one drops the other), so it has no case here.
"""

import argparse
import json
import sys
from decimal import Decimal, getcontext
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "locks" / "v1" / "coordinates.json"
getcontext().prec = 60

E0, N0 = 487000, 4420000


def ex(v):
    return F(float(v))


def root(q):
    """√q of an exact non-negative rational, as a Fraction near enough (60 digits)."""
    d = (Decimal(q.numerator) / Decimal(q.denominator)).sqrt()
    return F(d)


def coordinate_point(o, c, x, y, length):
    o = (ex(o[0]), ex(o[1]))
    c = (ex(c[0]), ex(c[1]))
    if x is not None and y is not None:
        return (ex(x), ex(y))
    if x is None and y is None:
        return None
    # Swap so the locked axis is the first.
    swap = x is None
    fixed = ex(y if swap else x)
    oa, ob = (o[1], o[0]) if swap else o
    cb = c[0] if swap else c[1]
    if length is None:
        p = (fixed, cb)
    else:
        L = ex(length)
        d = fixed - oa
        disc = L * L - d * d
        if disc < 0:
            return None
        r = root(disc)
        up, down = ob + r, ob - r
        b = up if abs(cb - up) <= abs(cb - down) else down
        p = (fixed, b)
    return (p[1], p[0]) if swap else p


def fl(p):
    return None if p is None else [float(p[0]), float(p[1])]


def case(name, o, c, x=None, y=None, length=None):
    return {"name": name, "o": list(map(float, o)), "c": list(map(float, c)), "x": x, "y": y, "length": length,
            "expect": fl(coordinate_point(o, c, x, y, length))}


def build():
    cases = [
        case("X ve Y: imleç önemsiz", (0, 0), (50, -3), x=12.5, y=-4.25),
        case("yalnız X: imlecin Y'si", (0, 0), (3, 7.5), x=10),
        case("yalnız Y: imlecin X'i", (0, 0), (3, 7.5), y=-2),
        case("X ve uzunluk: imlece yakın üst kesişim (3-4-5)", (0, 0), (3, 10), x=3, length=5),
        case("X ve uzunluk: imlece yakın alt kesişim", (0, 0), (3, -10), x=3, length=5),
        case("X ve uzunluk: imleç aynı hizada, üst", (0, 0), (3, 0), x=3, length=5),
        case("X ve uzunluk: çember eksene değer", (1, 1), (9, 9), x=6, length=5),
        case("X ve uzunluk: çember ulaşmaz", (0, 0), (9, 9), x=6, length=5),
        case("Y ve uzunluk: imlece yakın sağ kesişim", (0, 0), (10, 4), y=4, length=5),
        case("Y ve uzunluk: imlece yakın sol kesişim", (0, 0), (-10, 4), y=4, length=5),
        case("X ve uzunluk: irrasyonel kök", (2, 3), (4, 9), x=4, length=7),
        case("TM: yalnız X", (E0, N0), (E0 + 5, N0 + 12.5), x=E0 + 20.25),
        case("TM: X ve uzunluk", (E0 + 0.5, N0 + 0.25), (E0 + 6, N0 + 30), x=E0 + 6.5, length=12.5),
        case("TM: X ve Y", (E0, N0), (E0, N0), x=E0 + 1.125, y=N0 - 7.875),
        case("hiçbiri yoksa nokta yok", (0, 0), (1, 1)),
    ]
    return {"format": "kentos.lock-coordinate-cases", "version": 1,
            "source": "scripts/fixtures/lock_coordinate_cases.py (docs/adr/0267 §1)", "cases": cases}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    a = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if a.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil", file=sys.stderr)
            return 1
        print(f"koordinat kilitleri tutarlı: {len(build()['cases'])} durum")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
