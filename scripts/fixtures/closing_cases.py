#!/usr/bin/env python3
"""Closings of near blocks (docs/adr/0221 §10): an area buffered out and back in by the same distance, written
without KentOS code.

Alanları kümele closes each cluster this way, and so does a Tampon taken out and back in. Four stepped buildings of a
block, turned a little each way and their corners moved by up to 0.2 m, lie a few metres apart; their buffer's boundary
meets itself in corners and touches a buffer's sides at the buildings' corners, so the circles and lines of the inward
buffer meet at nearly one point in many places. The core once lost whole faces there (a third circle crossing two
nearly identical ones at a shallow angle, a merged piece barely longer than 10⁻⁶ m stepped off on its wrong side);
these blocks hold it to the closing's area.

The closing here does not come from GEOS's buffer, which simplifies its input by a hundredth of the distance and so
differs from the exact closing by up to about 0.006 m² at 4 m. GEOS (shapely) takes the union of the buildings out by
the distance, its arcs 1024 chords a quarter turn (within 10⁻⁶ m of the circle); that outline is then eroded exactly:
less every point within the distance of it, which are each edge's strip on the inner side and a disc (8192 chords) at
each corner where the outline turns inward. A point's nearest place on a polygon's boundary is inside one of its edges
or at one of its corners; inside the polygon, only an inward-turning corner can be the nearest place to a point that
no strip holds. The area is the result's, within 10⁻⁴ m² of the exact closing's.

The cases: the block of Alanları kümele's timing scene (the first one lost), four blocks a run of 300 random ones once
lost, and blocks drawn here at distances of 1, 1.5, 2.5 and 4 m.

    python3 scripts/fixtures/closing_cases.py                 # write fixtures/geoprocess/v1/closings.json
    python3 scripts/fixtures/closing_cases.py --check         # compare
    python3 scripts/fixtures/closing_cases.py --stress FILE   # compare the core's closings of random blocks
                                                              #   (KENTOS_CLOSINGS_OUT=FILE cargo test -p kentos-geometry-core
                                                              #   --test all closings_of_random_blocks -- --ignored)
"""

import json
import math
import random
import sys
from pathlib import Path

from shapely.geometry import Point, Polygon
from shapely.ops import unary_union

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "geoprocess" / "v1" / "closings.json"
SOURCE = "scripts/fixtures/closing_cases.py (docs/adr/0221 §10)"
TOLERANCE = 1e-3

E, N = 487000, 4420000
SHAPE = [(0.0, 0.0), (12.0, 0.0), (12.0, 8.0), (6.0, 8.0), (6.0, 8.8), (0.0, 8.8)]


def ccw(pts):
    """Counter-clockwise, the shoelace taken from the first point (a hole of millimetres stands 4·10⁶ m out)."""
    x0, y0 = pts[0]
    return sum((p[0] - x0) * (q[1] - y0) - (q[0] - x0) * (p[1] - y0) for p, q in zip(pts, pts[1:] + pts[:1])) > 0


def closing(rings, r):
    """The union of the rings' polygons closed by `r`: out by GEOS, back in exactly (see the module's note)."""
    union = unary_union([Polygon(x) for x in rings])
    out = union.buffer(r, quad_segs=1024)
    pieces = []
    for poly in getattr(out, "geoms", [out]):
        for ring, outer in [(poly.exterior, True)] + [(h, False) for h in poly.interiors]:
            pts = list(ring.coords)[:-1]
            # The inside on the left: the outer ring counter-clockwise, the holes clockwise.
            if ccw(pts) != outer:
                pts.reverse()
            n = len(pts)
            for i in range(n):
                p, q, s = pts[i], pts[(i + 1) % n], pts[(i + 2) % n]
                dx, dy = q[0] - p[0], q[1] - p[1]
                length = math.hypot(dx, dy)
                if length == 0:
                    continue
                lx, ly = -dy / length * r, dx / length * r
                pieces.append(Polygon([p, q, (q[0] + lx, q[1] + ly), (p[0] + lx, p[1] + ly)]))
                # Turning right at q (inward): the points nearest to q itself.
                if dx * (s[1] - q[1]) - dy * (s[0] - q[0]) < 0:
                    pieces.append(Point(q).buffer(r, quad_segs=2048))
    return out.difference(unary_union(pieces))


def measured(rings, r):
    """The closing's area and parts; the chords' crumbs at the buildings' corners (below 10⁻⁶ m²) are no parts."""
    c = closing(rings, r)
    parts = [g for g in getattr(c, "geoms", [c]) if g.area > 1e-6]
    return {"area": sum(g.area for g in parts), "parts": len(parts)}


def block(rnd, bx, by):
    """Four stepped buildings of a block, gaps of 2 to 4.5 m, each turned up to 3° either way, corners moved up to 0.2 m."""
    gap_x, gap_y = 2.0 + 2.5 * rnd.random(), 2.0 + 2.5 * rnd.random()
    rings = []
    for ox, oy in [(0.0, 0.0), (12.0 + gap_x, 0.0), (0.0, 8.8 + gap_y), (12.0 + gap_x, 8.8 + gap_y)]:
        turn = (rnd.random() - 0.5) * 6.0 * math.pi / 180.0
        s, c = math.sin(turn), math.cos(turn)
        ring = []
        for x, y in SHAPE:
            x, y = x - 6.0 + (rnd.random() - 0.5) * 0.4, y - 4.4 + (rnd.random() - 0.5) * 0.4
            ring.append([bx + ox + 6.0 + x * c - y * s, by + oy + 4.4 + x * s + y * c])
        rings.append(ring)
    return rings


# Blocks the core once lost, as they were drawn (the doubles matter: the faces went where circles met within 10⁻⁶ m).
FOUND = json.loads((Path(__file__).resolve().parent / "closing_found.json").read_text())


def build():
    cases = []
    for name, about, distance, rings in FOUND:
        cases.append({"name": name, "about": about, "distance": distance, "rings": rings, **measured(rings, distance)})
    rnd = random.Random(221)
    for k, distance in enumerate([1.0, 1.5, 2.5, 4.0] * 10):
        rings = block(rnd, E + (k % 8) * 70.0 + rnd.random(), N + (k // 8) * 60.0 + rnd.random())
        cases.append({
            "name": f"block-{k + 1}",
            "about": f"Dörtlü blok, {distance:g} m",
            "distance": distance,
            "rings": rings,
            **measured(rings, distance),
        })
    return {
        "format": "kentos.closing-cases",
        "version": 1,
        "source": SOURCE,
        "tolerance": {"area": TOLERANCE},
        "cases": cases,
    }


def stress(path):
    """The core's closings of random blocks (each: rings, distance, area, count) against the reference."""
    blocks = json.loads(Path(path).read_text())
    bad, worst = 0, 0.0
    for k, b in enumerate(blocks):
        want = measured(b["rings"], b["distance"])
        diff = abs(want["area"] - b["area"])
        worst = max(worst, diff)
        if diff > TOLERANCE or want["parts"] != b["count"] or "--all" in sys.argv:
            bad += 1
            print(f"blok {k}: çekirdek {b['area']:.6f} m², {b['count']} parça; başvuru {want['area']:.6f} m², {want['parts']} parça")
    print(f"{len(blocks)} blok, {bad} farklı, en büyük alan farkı {worst:.2e} m²")
    return bad == 0


def main():
    if "--stress" in sys.argv[1:]:
        sys.exit(0 if stress(sys.argv[sys.argv.index("--stress") + 1]) else 1)
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT.relative_to(ROOT)} güncel")
        return
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(json.loads(text)['cases'])} durum")


if __name__ == "__main__":
    main()
