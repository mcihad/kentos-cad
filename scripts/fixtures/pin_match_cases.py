#!/usr/bin/env python3
"""Independent reference for matching label pins without a key (docs/adr/0578 §2): written from the ADR's rule,
without KentOS code. It writes `fixtures/labels/v1/pin-match.json`; `cargo test -p kentos-geometry-core --test all
pin_match` reads the cases and runs `ops::pin_match::match_pins` over them.

The rule: an old object's pins go to the one new row of the same kind with exactly the same coordinates; with no such
row, to the nearest new row of the kind (among the rows no object has an exact candidate in) within 0.05 m by the
distance between the middles of the boxes, when the next nearest is farther; a row two objects chose goes to neither;
else lost.

    python3 scripts/fixtures/pin_match_cases.py          # write
    python3 scripts/fixtures/pin_match_cases.py --check  # compare
"""

from __future__ import annotations

import json
import math
import random
import sys
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/labels/v1/pin-match.json"
TOL = 0.05


def feature(kind, coords):
    """A feature: its kind, its vertices as east, north pairs; the box's middle is derived here from the vertices."""
    xs, ys = coords[0::2], coords[1::2]
    return {"kind": kind, "coords": coords, "middle": [(min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2]}


def same(a, b):
    return a["kind"] == b["kind"] and a["coords"] == b["coords"]


def dist(a, b):
    # An exact-enough distance: the squared difference in fractions, rooted in floats.
    dx = Fraction(b["middle"][0]) - Fraction(a["middle"][0])
    dy = Fraction(b["middle"][1]) - Fraction(a["middle"][1])
    return math.sqrt(float(dx * dx + dy * dy))


def match(old, new, tol=TOL):
    # What each object finds, as (how, row, distance).
    found = []
    claimed_by_exact = set()
    for o in old:
        rows = [j for j, n in enumerate(new) if same(o, n)]
        claimed_by_exact.update(rows)
        found.append(("exact", rows))
    result = []
    for o, (how, rows) in zip(old, found):
        if rows:
            result.append(("geometry", rows[0]) if len(rows) == 1 else ("ambiguous", None))
            continue
        pool = sorted(
            (dist(o, n), j) for j, n in enumerate(new) if j not in claimed_by_exact and n["kind"] == o["kind"]
        )
        if not pool or pool[0][0] > tol:
            result.append(("none", None))
        elif len(pool) > 1 and pool[1][0] == pool[0][0]:
            result.append(("ambiguous", None))
        else:
            result.append(("nearest", pool[0][1], pool[0][0]))
    # A row two objects chose goes to neither.
    chose = {}
    for i, r in enumerate(result):
        if r[0] in ("geometry", "nearest"):
            chose.setdefault(r[1], []).append(i)
    for rows in chose.values():
        if len(rows) > 1:
            for i in rows:
                result[i] = ("ambiguous", None)
    out = []
    for r in result:
        if r[0] == "geometry":
            out.append({"found": "geometry", "to": r[1]})
        elif r[0] == "nearest":
            out.append({"found": "nearest", "to": r[1], "distance": r[2]})
        else:
            out.append({"found": r[0]})
    return out


def case(name, old, new, tol=TOL):
    return {"name": name, "tolerance": tol, "old": old, "new": new, "expect": match(old, new, tol)}


def cases():
    p = lambda x, y: feature("point", [x, y])
    poly = lambda x, y, w, h: feature("polygon", [x, y, x + w, y, x + w, y + h, x, y + h])
    out = [
        case("the same coordinates", [p(10.0, 20.0)], [p(10.0, 20.0)]),
        case("the same coordinates win over a nearer row", [p(1.0, 1.0)], [p(1.01, 1.0), p(1.0, 1.0)]),
        case("two rows with the same coordinates are ambiguous", [p(1.0, 1.0)], [p(1.0, 1.0), p(1.0, 1.0)]),
        case("nearest within the tolerance", [p(0.0, 0.0)], [p(0.0, 0.04), p(0.0, 3.0)]),
        case("on the tolerance", [p(0.0, 0.0)], [p(0.0, 0.05)]),
        case("beyond the tolerance", [p(0.0, 0.0)], [p(0.0, 0.0500001)]),
        case("a tie is no guess", [p(0.0, 0.0)], [p(0.02, 0.0), p(-0.02, 0.0)]),
        case("another kind is no row", [p(0.0, 0.0)], [poly(0.0, 0.0, 1.0, 1.0)]),
        case("two objects choose one row", [p(0.0, 0.0), p(0.02, 0.0)], [p(0.01, 0.0)]),
        case("no new rows", [p(0.0, 0.0)], []),
        case("no objects", [], [p(0.0, 0.0)]),
        case("a polygon shifted by a centimetre", [poly(100.0, 200.0, 20.0, 10.0)], [poly(100.01, 200.0, 20.0, 10.0)]),
        case("a polygon shifted by a metre", [poly(100.0, 200.0, 20.0, 10.0)], [poly(101.0, 200.0, 20.0, 10.0)]),
        case("an exact row is not left to a nearer object",
             [p(0.0, 0.0), p(0.03, 0.0)], [p(0.0, 0.0), p(0.04, 0.0)]),
        case("custom tolerance", [p(0.0, 0.0)], [p(0.0, 0.5)], 1.0),
        case("big coordinates", [p(487654.321, 4420123.456)], [p(487654.321, 4420123.456), p(487654.33, 4420123.456)]),
        case("rows in another order",
             [poly(0.0, 0.0, 5.0, 5.0), poly(10.0, 0.0, 5.0, 5.0), poly(20.0, 0.0, 5.0, 5.0)],
             [poly(20.0, 0.0, 5.0, 5.0), poly(0.0, 0.0, 5.0, 5.0), poly(10.0, 0.0, 5.0, 5.0)]),
    ]
    # Seeded random cases: points and squares moved by 0 to 8 cm, some dropped, some added, a few doubled.
    rng = random.Random(20261021)
    for k in range(24):
        old, new = [], []
        n = rng.randint(1, 6)
        for _ in range(n):
            x, y = round(rng.uniform(0, 100), 3), round(rng.uniform(0, 100), 3)
            f = (lambda a, b: p(a, b)) if rng.random() < 0.5 else (lambda a, b: poly(a, b, 4.0, 3.0))
            old.append(f(x, y))
            roll = rng.random()
            if roll < 0.2:
                continue  # the row is gone
            if roll < 0.5:
                new.append(f(x, y))
            else:
                new.append(f(round(x + rng.uniform(-0.08, 0.08), 3), round(y + rng.uniform(-0.08, 0.08), 3)))
            if rng.random() < 0.15:
                new.append(f(x, y))
        for _ in range(rng.randint(0, 2)):
            new.append(p(round(rng.uniform(0, 100), 3), round(rng.uniform(0, 100), 3)))
        rng.shuffle(new)
        out.append(case(f"random {k + 1}", old, new))
    return out


def main():
    text = json.dumps({"version": 1, "tolerance": TOL, "cases": cases()}, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT.relative_to(ROOT)}: güncel.")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(cases())} durum)")


if __name__ == "__main__":
    main()
