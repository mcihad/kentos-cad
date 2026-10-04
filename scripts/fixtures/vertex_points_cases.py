#!/usr/bin/env python3
"""Independent reference of Köşelere nokta (docs/adr/0152 §5).

Writes fixtures/vertex-points/v1/cases.json from the rules alone, with
Python's standard library and no KentOS code. The geometry core
(`ops::vertex_points::vertex_points`, crates/shared/geometry-core/tests/all/vertex_points.rs)
and the web through its WASM must give the same, points within 1e-9 m.

Input: objects in the drawing's order, each its paths ({pts, closed, zs});
the places of the points already there; the first name (null: no names).

1. Every path's vertices, object after object, path after path, in order.
2. A vertex within 1e-6 m of a point already placed is that point: when the
   point has no elevation and the vertex has one, the point takes it.
3. Else a vertex within 1e-6 m of an existing point is passed over; each
   such place is counted once (the first vertex there).
4. Else it is a new point, with the vertex's elevation and the current
   name; the name then becomes the next: the digits it ends with, as a
   decimal number, plus one, at least as many digits as they were (A-009
   → A-010, 99 → 100); a name not ending in a digit stays as it is.
5. `next` is the name after the last point (the first, when none).

When several placed points or existing ones are within reach, the first
placed counts (the order they were placed in).
"""

import argparse
import json
import math
import random
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "vertex-points" / "v1" / "cases.json"
TOUCH = 1e-6
E0, N0 = 487000.0, 4420000.0


def increment(text):
    i = len(text)
    while i > 0 and "0" <= text[i - 1] <= "9":
        i -= 1
    if i == len(text):
        return None
    digits = text[i:]
    return text[:i] + str(int(digits) + 1).rjust(len(digits), "0")


def near(a, b):
    return math.hypot(a["x"] - b["x"], a["y"] - b["y"]) <= TOUCH


def solve(objects, existing, first):
    points = []
    passed = []
    name = first
    for paths in objects:
        for path in paths:
            for k, p in enumerate(path["pts"]):
                z = path["zs"][k]
                hit = next((q for q in points if near(q["p"], p)), None)
                if hit is not None:
                    if hit["z"] is None:
                        hit["z"] = z
                    continue
                if any(near(e, p) for e in existing):
                    if not any(near(q, p) for q in passed):
                        passed.append(p)
                    continue
                points.append({"p": p, "z": z, "name": name})
                if name is not None:
                    nxt = increment(name)
                    name = nxt if nxt is not None else name
    return {"points": points, "skipped": len(passed), "next": name}


def P(x, y):
    return {"x": x, "y": y}


def path(pts, zs=None, closed=False):
    return {"pts": [P(*p) for p in pts], "closed": closed, "zs": zs if zs is not None else [None] * len(pts)}


def hand():
    cases = []
    square = path([(0, 0), (20, 0), (20, 15), (0, 15)], closed=True)
    cases.append(("bir alanın dört köşesi, 101'den", [[square]], [], "101"))
    cases.append(("adsız", [[square]], [], None))
    cases.append(("bölü: 101/12, 101/13 …", [[square]], [], "101/12"))
    cases.append(("dokuzdan ona: P9, P10", [[path([(0, 0), (5, 0), (5, 5)])]], [], "P9"))
    cases.append(("sıfır dolgusu: A-009, A-010", [[path([(0, 0), (5, 0)])]], [], "A-009"))
    cases.append(("sayıyla bitmeyen ad aynı kalır", [[path([(0, 0), (5, 0), (5, 5)])]], [], "Köşe"))
    # Two parcels sharing an edge: the shared corners once.
    left = path([(0, 0), (20, 0), (20, 15), (0, 15)], closed=True)
    right = path([(20, 0), (40, 0), (40, 15), (20, 15)], closed=True)
    cases.append(("iki komşu parsel: ortak köşeler bir kez", [[left], [right]], [], "1"))
    # Existing points at two corners: passed over and counted; the names run on without them.
    cases.append(("var olan noktalar atlanır ve sayılır", [[left], [right]], [P(20, 0), P(0, 15)], "201"))
    # A shared corner on an existing point: counted once.
    cases.append(("var olan noktadaki ortak köşe bir kez sayılır", [[left], [right]], [P(20, 15)], "1"))
    # Elevations: the vertex's; a shared corner takes the first elevation given (the second parcel's).
    lz = path([(0, 0), (20, 0), (20, 15), (0, 15)], zs=[100.0, None, None, 101.5], closed=True)
    rz = path([(20, 0), (40, 0), (40, 15), (20, 15)], zs=[100.4, 100.9, 101.2, None], closed=True)
    cases.append(("kotlar köşelerden; ortak köşe ilk verilen kotu alır", [[lz], [rz]], [], "1"))
    # An area with a hole, then a second part: rings in order.
    outer = path([(0, 0), (30, 0), (30, 30), (0, 30)], closed=True)
    hole = path([(10, 10), (10, 20), (20, 20), (20, 10)], closed=True)
    part = path([(40, 0), (50, 0), (50, 10)], closed=True)
    cases.append(("delik ve parça: halka halka", [[outer, hole, part]], [], "1"))
    # A polyline that ends where it began: its last vertex is its first point.
    cases.append(("başladığı yerde biten çoklu çizgi", [[path([(0, 0), (10, 0), (10, 10), (0, 0)])]], [], "1"))
    # A vertex repeated in a row, and one 0.5 µm off: the same point; one 2 µm off: another.
    cases.append(("1 µm içinde aynı yer, dışında ayrı", [[path([(0, 0), (0, 0), (0.0000005, 0), (0.000002, 0), (5, 0)])]], [], "1"))
    cases.append(("boş girdi", [], [P(0, 0)], "1"))
    return cases


def moved(case, de, dn):
    name, objects, existing, first = case
    mv = lambda p: P(p["x"] + de, p["y"] + dn)
    objs = [[{**pa, "pts": [mv(p) for p in pa["pts"]]} for pa in obj] for obj in objects]
    return (name + ", TM koordinatlarında", objs, [mv(p) for p in existing], first)


def network(seed):
    """Parcels of a jittered grid as areas (shared corners), some lines, some existing points, elevations here and there."""
    rnd = random.Random(seed)
    cols, rows = rnd.randint(1, 4), rnd.randint(1, 3)
    w, h = rnd.uniform(10, 25), rnd.uniform(10, 20)
    x0, y0 = E0 + rnd.uniform(-200, 200), N0 + rnd.uniform(-200, 200)
    r3 = lambda v: round(v, 3)
    xs = [r3(x0 + i * w + (rnd.uniform(-2, 2) if 0 < i < cols else 0)) for i in range(cols + 1)]
    ys = [r3(y0 + j * h + (rnd.uniform(-2, 2) if 0 < j < rows else 0)) for j in range(rows + 1)]
    z = lambda: r3(rnd.uniform(100, 120)) if rnd.random() < 0.4 else None
    objects = []
    for i in range(cols):
        for j in range(rows):
            pts = [(xs[i], ys[j]), (xs[i + 1], ys[j]), (xs[i + 1], ys[j + 1]), (xs[i], ys[j + 1])]
            objects.append([path(pts, zs=[z() for _ in pts], closed=True)])
    for _ in range(rnd.randint(0, 3)):
        a = (r3(rnd.uniform(xs[0], xs[-1])), r3(rnd.uniform(ys[0], ys[-1])))
        b = (rnd.choice(xs), rnd.choice(ys))
        objects.append([path([a, b], zs=[z(), z()])])
    rnd.shuffle(objects)
    corners = [P(x, y) for x in xs for y in ys]
    existing = rnd.sample(corners, rnd.randint(0, min(4, len(corners))))
    first = rnd.choice(["1", "101", "101/9", "P099", "S-1", None])
    return (f"rastgele parseller {seed}", objects, existing, first)


def build():
    cases = []
    for c in hand():
        cases.append(c)
        cases.append(moved(c, E0, N0))
    for seed in range(1, 21):
        cases.append(network(seed))
    out = []
    for name, objects, existing, first in cases:
        out.append({"name": name, "objects": objects, "existing": existing, "first": first, "expected": solve(objects, existing, first)})
    return {"format": "kentos.vertex-points-fixtures", "version": 1, "cases": out}


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--check", action="store_true", help="compare with the written file instead of writing it")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if OUT.read_text() != text:
            print(f"{OUT} güncel değil: python3 scripts/fixtures/vertex_points_cases.py", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT}: güncel")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    print(f"{OUT}: {len(json.loads(text)['cases'])} durum")


if __name__ == "__main__":
    main()
