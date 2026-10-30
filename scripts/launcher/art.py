#!/usr/bin/env python3
"""The KentOS Launcher's sidebar drawing (docs/adr/0803 §4): a fragment of a
zoning plan in hairlines, with its blocks, parcels, one wide curving road, a
roundabout and a few survey points. The launcher tints it, so one file serves
both themes, and fades it into the table at its top and its bottom, whatever
the window's height.

    python3 scripts/launcher/art.py          # writes apps/launcher/assets/art/imar.svg
    python3 scripts/launcher/art.py --check  # says whether the file is the script's

It is drawn from a fixed seed with the standard library alone: the same bytes
on every machine.
"""

from __future__ import annotations

import argparse
import math
import random
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "apps/launcher/assets/art/imar.svg"

# The drawing's size in its own units: the sidebar at the default text size.
WIDTH, HEIGHT = 216, 700
# The street grid before it is bent: spacing across and down, and its turn.
ACROSS, DOWN = 70.0, 92.0
TURN = math.radians(-11.0)
# Half a street's width, and the wide road's.
STREET, ROAD = 3.2, 7.0
# The column whose left edge is the wide road.
ROAD_COLUMN = 2
# Parcels: about this wide along a block's long side.
LOT = 13.0
# Hairlines: a block's edge, a parcel's.
EDGE, LINE = 0.6, 0.4

Point = tuple[float, float]


def node(i: int, j: int) -> Point:
    """A street crossing: the grid bent by two long waves, so no street is straight."""
    x, y = i * ACROSS, j * DOWN
    x, y = x + 15.0 * math.sin(y / 170.0 + 0.6), y + 11.0 * math.sin(x / 130.0 + 1.1)
    c, s = math.cos(TURN), math.sin(TURN)
    return (
        WIDTH / 2 + (x - 1.5 * ACROSS) * c - (y - 4 * DOWN) * s,
        HEIGHT / 2 + (x - 1.5 * ACROSS) * s + (y - 4 * DOWN) * c,
    )


def toward(a: Point, b: Point, t: float) -> Point:
    return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)


def length(a: Point, b: Point) -> float:
    return math.hypot(b[0] - a[0], b[1] - a[1])


def meet(a: Point, b: Point, c: Point, d: Point) -> Point:
    """Where the lines ab and cd cross."""
    x1, y1, x2, y2 = a[0], a[1], b[0], b[1]
    x3, y3, x4, y4 = c[0], c[1], d[0], d[1]
    under = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4)
    t = ((x1 - x3) * (y3 - y4) - (y1 - y3) * (x3 - x4)) / under
    return (x1 + (x2 - x1) * t, y1 + (y2 - y1) * t)


def inset(quad: list[Point], widths: list[float]) -> list[Point]:
    """The quad with each side moved in by its width (clockwise on the screen)."""
    lines = []
    for k, width in enumerate(widths):
        a, b = quad[k], quad[(k + 1) % 4]
        size = length(a, b)
        nx, ny = -(b[1] - a[1]) / size, (b[0] - a[0]) / size
        lines.append(((a[0] + nx * width, a[1] + ny * width), (b[0] + nx * width, b[1] + ny * width)))
    return [meet(*lines[k - 1], *lines[k]) for k in range(4)]


def cuts(rng: random.Random, size: float) -> list[float]:
    """Where a side of `size` is cut into parcels: no two alike, as a plan's are."""
    count = max(2, round(size / LOT))
    steps = [rng.uniform(0.82, 1.18) for _ in range(count)]
    total = sum(steps)
    places, at = [], 0.0
    for step in steps[:-1]:
        at += step
        places.append(at / total)
    return places


def path(points: list[Point], closed: bool = False) -> str:
    words = " ".join(f"{x:.1f} {y:.1f}" for x, y in points)
    return f"M{words}{'Z' if closed else ''}"


def block(rng: random.Random, quad: list[Point], edges: list[str], lots: list[str], points: list[Point]) -> None:
    edges.append(path(quad, closed=True))
    if rng.random() < 0.34:
        points.append(quad[rng.randrange(4)])
    kind = rng.random()
    if kind < 0.1:
        # A park: nothing is parcelled, a path runs round it.
        lots.append(path(inset(quad, [4.0, 4.0, 4.0, 4.0]), closed=True))
        return
    a, b, c, d = quad
    if length(a, b) < length(b, c):
        a, b, c, d = b, c, d, a
    if kind < 0.28:
        # A school's or a market's one large plot, set back from the street.
        lots.append(path(inset([a, b, c, d], [5.0, 5.0, 5.0, 5.0]), closed=True))
        return
    spine_a, spine_b = toward(a, d, 0.5), toward(b, c, 0.5)
    lots.append(path([spine_a, spine_b]))
    for near, far, spine_near, spine_far in ((a, b, spine_a, spine_b), (d, c, spine_a, spine_b)):
        for t in cuts(rng, length(near, far)):
            lots.append(path([toward(near, far, t), toward(spine_near, spine_far, t)]))


def draw() -> str:
    rng = random.Random(1923)
    edges: list[str] = []
    lots: list[str] = []
    points: list[Point] = []
    for j in range(-1, 10):
        for i in range(-1, 5):
            quad = [node(i, j), node(i + 1, j), node(i + 1, j + 1), node(i, j + 1)]
            left = ROAD if i == ROAD_COLUMN else STREET
            right = ROAD if i + 1 == ROAD_COLUMN else STREET
            block(rng, inset(quad, [STREET, right, STREET, left]), edges, lots, points)
    # A roundabout where the wide road meets a street.
    cx, cy = node(ROAD_COLUMN, 5)
    rings = "".join(f'<circle cx="{cx:.1f}" cy="{cy:.1f}" r="{r}"/>' for r in (4.5, 10.5))
    marks = "".join(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="1.3"/>' for x, y in points)
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {WIDTH} {HEIGHT}">'
        '<g fill="none" stroke="#fff">'
        f'<path stroke-width="{LINE}" d="{" ".join(lots)}"/>'
        f'<path stroke-width="{EDGE}" d="{" ".join(edges)}"/>'
        f'<g stroke-width="{EDGE}">{rings}</g>'
        f'<g fill="#fff" stroke="none">{marks}</g>'
        "</g></svg>\n"
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--check", action="store_true", help="say whether the file is the script's")
    args = parser.parse_args()
    drawing = draw()
    if args.check:
        same = OUT.is_file() and OUT.read_text(encoding="utf-8") == drawing
        print("güncel" if same else f"{OUT.relative_to(ROOT)} eski; betiği --check olmadan çalıştırın.")
        return 0 if same else 1
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(drawing, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)}: {len(drawing):,} bayt")
    return 0


if __name__ == "__main__":
    sys.exit(main())
