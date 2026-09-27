"""The geometry values expressions read directly from an object
($uzunluk, $alan, $merkez_y/$merkez_x, $min_y … $yükseklik, $köşe;
docs/adr/0100 §3), computed here independently of the engine.

    python3 scripts/fixtures/expression_geometry.py           # writes the file
    python3 scripts/fixtures/expression_geometry.py --check   # writes nothing; compares

Writes fixtures/expression/v2/geometry.json: objects in the web's entity form
and, for each, the values the definitions give:
- area and centroid of straight rings in exact rational arithmetic (the
  shoelace formula and its first moments, holes subtracted), so the
  centroid of a parcel in projected coordinates (millions of metres) is the
  exact one, rounded once;
- a semicircular edge (bulge 1) by the half disk's own formulas (area
  πr²/2, centroid 4r/3π from the chord), not the engine's general segment;
- a circle's area πr² and centre;
- the box from the vertices, the half disk's outermost point included;
- lengths as sums of distances (math.fsum);
- objects without an area: the centroid is the anchor (a line's midpoint,
  a path's middle vertex, a point).

`--check` rebuilds the file in memory and compares it with the one on disk.
Rust compares the engine with this file (crates/shared/expression/tests/typed.rs)
within a stated tolerance.
"""
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "fixtures/expression/v2/geometry.json"

# A parcel's corner in projected coordinates (TM, metres).
X0, Y0 = 487000.25, 4420000.75


def P(x, y):
    return {"x": x, "y": y}


def ring_moments(pts):
    """Exact signed area and first moments of a straight ring (Fractions)."""
    a = mx = my = F(0)
    n = len(pts)
    for i in range(n):
        (x1, y1), (x2, y2) = pts[i], pts[(i + 1) % n]
        x1, y1, x2, y2 = F(x1), F(y1), F(x2), F(y2)
        c = x1 * y2 - x2 * y1
        a += c / 2
        mx += (x1 + x2) * c / 6
        my += (y1 + y2) * c / 6
    return a, mx, my


def area_centroid(outer, holes=()):
    """Area (holes removed) and centroid, exact."""
    a, mx, my = ring_moments(outer)
    s = 1 if a >= 0 else -1
    area, sx, sy = a * s, mx * s, my * s
    for h in holes:
        a, mx, my = ring_moments(h)
        s = 1 if a >= 0 else -1
        area -= a * s
        sx -= mx * s
        sy -= my * s
    return area, (sx / area, sy / area)


def path_length(pts, closed):
    n = len(pts)
    segs = [(pts[i], pts[i + 1]) for i in range(n - 1)]
    if closed and n > 2:
        segs.append((pts[-1], pts[0]))
    return math.fsum(math.hypot(b[0] - a[0], b[1] - a[1]) for a, b in segs)


def box(pts):
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]
    return min(xs), min(ys), max(xs), max(ys)


def values(length, area, centroid, bx, vertices):
    """The expected values, in the expression's names (Y east = x, X north = y)."""
    out = {
        "uzunluk": length,
        "alan": area,
        "merkez_y": None if centroid is None else float(centroid[0]),
        "merkez_x": None if centroid is None else float(centroid[1]),
        "min_y": bx[0],
        "max_y": bx[2],
        "min_x": bx[1],
        "max_x": bx[3],
        "genişlik": bx[2] - bx[0],
        "yükseklik": bx[3] - bx[1],
        "köşe": vertices,
    }
    if area is not None:
        out["alan"] = float(area)
    return out


def square():
    pts = [(X0, Y0), (X0 + 20, Y0), (X0 + 20, Y0 + 30), (X0, Y0 + 30)]
    area, c = area_centroid(pts)
    return (
        "kare, projeksiyon koordinatlarında",
        {"kind": "polygon", "pts": [P(*p) for p in pts]},
        values(path_length(pts, True), area, c, box(pts), 4),
    )


def clockwise_l():
    pts = [(X0, Y0), (X0, Y0 + 40), (X0 + 10, Y0 + 40), (X0 + 10, Y0 + 10), (X0 + 30, Y0 + 10), (X0 + 30, Y0)]
    area, c = area_centroid(pts)
    return (
        "L biçimli, saat yönünde",
        {"kind": "polygon", "pts": [P(*p) for p in pts]},
        values(path_length(pts, True), area, c, box(pts), 6),
    )


def with_hole():
    outer = [(X0, Y0), (X0 + 50, Y0), (X0 + 50, Y0 + 40), (X0, Y0 + 40)]
    hole = [(X0 + 10, Y0 + 10), (X0 + 10, Y0 + 20), (X0 + 25, Y0 + 20), (X0 + 25, Y0 + 10)]
    area, c = area_centroid(outer, [hole])
    return (
        "delikli (delik ters yönde)",
        {"kind": "polygon", "pts": [P(*p) for p in outer], "holes": [{"pts": [P(*p) for p in hole]}]},
        values(path_length(outer, True) + path_length(hole, True), area, c, box(outer), 8),
    )


def half_disk_edge():
    # A 20 × 10 rectangle whose top edge, from (20, 10) to (0, 10), is a half
    # circle bulging outward (counter-clockwise ring, bulge 1 on edge 2).
    base = [(X0, Y0), (X0 + 20, Y0), (X0 + 20, Y0 + 10), (X0, Y0 + 10)]
    r = 10.0
    rect_area, rect_c = area_centroid(base)
    half_area = math.pi * r * r / 2
    # The half disk's centroid: 4r/3π above the chord's midpoint.
    hx, hy = X0 + 10, Y0 + 10 + 4 * r / (3 * math.pi)
    rect_area = float(rect_area)
    area = rect_area + half_area
    cx = (rect_area * float(rect_c[0]) + half_area * hx) / area
    cy = (rect_area * float(rect_c[1]) + half_area * hy) / area
    length = 20 + 10 + 10 + math.pi * r
    return (
        "bir kenarı yarım daire (bulge 1)",
        {"kind": "polygon", "pts": [P(*p) for p in base], "bulges": [0, 0, 1, 0]},
        values(length, area, (cx, cy), (X0, Y0, X0 + 20, Y0 + 20), 4),
        # Floats on this one: its tolerance is relative (π, a division).
    )


def hatch_with_hole():
    ring = [(0, 0), (8, 0), (8, 6), (0, 6)]
    hole = [(2, 2), (4, 2), (4, 4), (2, 4)]
    area, c = area_centroid(ring, [hole])
    out = values(None, area, c, box(ring), None)
    return (
        "tarama, delikli",
        {"kind": "hatch", "ring": [P(*p) for p in ring], "holes": [[P(*p) for p in hole]], "pattern": {"type": "solid", "angle": 0, "spacing": 1}},
        out,
    )


def circle():
    c, r = (X0 + 5, Y0 + 7), 3.0
    return (
        "daire",
        {"kind": "circle", "c": P(*c), "r": r},
        values(2 * math.pi * r, math.pi * r * r, c, (c[0] - r, c[1] - r, c[0] + r, c[1] + r), None),
    )


def line():
    a, b = (3.0, 4.0), (9.0, 12.0)
    mid = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
    return ("çizgi", {"kind": "line", "a": P(*a), "b": P(*b)}, values(10.0, None, mid, box([a, b]), 2))


def polyline():
    pts = [(0.0, 0.0), (3.0, 4.0), (3.0, 10.0), (9.0, 10.0)]
    # The anchor of a path: its middle vertex (pts[len / 2]).
    return (
        "çoklu çizgi",
        {"kind": "polyline", "pts": [P(*p) for p in pts]},
        values(path_length(pts, False), None, pts[len(pts) // 2], box(pts), 4),
    )


def point():
    p = (X0 + 1.5, Y0 - 2.5)
    return ("nokta", {"kind": "point", "p": P(*p)}, values(None, None, p, box([p]), 1))


def collinear():
    # No area: the centroid falls back to the anchor, the vertices' average.
    pts = [(0.0, 0.0), (2.0, 0.0), (6.0, 0.0)]
    avg = (sum(p[0] for p in pts) / 3, sum(p[1] for p in pts) / 3)
    return (
        "alansız kapalı alan (doğrusal köşeler)",
        {"kind": "polygon", "pts": [P(*p) for p in pts]},
        values(12.0, 0.0, avg, box(pts), 3),
    )


def build():
    cases = []
    for make in [square, clockwise_l, with_hole, half_disk_edge, hatch_with_hole, circle, line, polyline, point, collinear]:
        name, entity, expected = make()[:3]
        entity = {"id": len(cases) + 1, "layerId": "a", "attrs": {}, **entity}
        cases.append({"name": name, "entity": entity, "values": expected})
    return {
        "format": "kentos.expression-geometry",
        "version": 2,
        "note": "İfadelerin nesneden doğrudan okuduğu geometri değerleri, motordan bağımsız hesaplanmış: düz halkaların alanı ve ağırlık merkezi kesirli sayılarla (delikler düşülür), yarım daire kenar yarım dairenin kendi formülleriyle, daire πr² ve merkezi, kutular köşelerden. Y sağa (kodda x), X yukarı (kodda y). Değer null: nesnenin öyle bir değeri yok. Üretici scripts/fixtures/expression_geometry.py.",
        "cases": cases,
    }


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            sys.exit(f"{OUT} güncel değil: python3 {sys.argv[0]}")
        print(f"{OUT} güncel.")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    print(f"{OUT} yazıldı.")


if __name__ == "__main__":
    main()
