#!/usr/bin/env python3
"""Paralel kaydır (docs/adr/0191): the shared cases, written from the ADR's
rules without KentOS code, with exact fractions (the target area's root
with mpmath at 50 digits).

- The edge's line moves by d along its normal (an area: away from its
  inside, so a positive d grows it; a polyline: the right of its way), its
  two vertices go to where the moved line meets the neighbouring edges'
  lines; an open polyline's free end moves square to the edge.
- Refused: a curved edge or neighbour, a neighbour parallel to the edge, a
  neighbour or the edge shrinking to nothing or turning round.
- A target area: A(d) = A0 + a1·d + a2·d², the root with the smaller |d|.

The core runs the cases natively and through WASM
(crates/shared/geometry-core/tests/all/edge_shift.rs,
apps/web/src/tools/edgeShift.wasm.test.ts).

    python3 scripts/fixtures/edge_shift_cases.py          # write
    python3 scripts/fixtures/edge_shift_cases.py --check  # compare
"""

import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

import mpmath as mp

mp.mp.dps = 50
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "edge-shift" / "v1" / "cases.json"
SOURCE = "scripts/fixtures/edge_shift_cases.py (docs/adr/0191)"

CURVED = "Kaydırılan kenar ya da komşusu yay; paralel kaydırma düz kenarlarla yapılır."
PARALLEL = "Komşu kenar kaydırılan kenara paralel; köşe bulunamıyor."
PASSED = "Kenar bu uzaklıkta komşusunu aşıyor; daha kısa bir uzaklık yazın."
NO_EDGE = "Bu nesnede böyle bir kenar yok."
NO_AREA = "Bu alana kenarı kaydırarak ulaşılamıyor."
NOT_AREA = "Hedef alan yalnız alanlarda yazılır."
MULTI = "Çok parçalı nesnenin kenarı kaydırılmaz; önce Parçalara ayır."
NOT_PATH = "Paralel kaydır alanın ya da çoklu çizginin düz kenarında çalışır; böyle bir kenara tıklayın."


def pt(x, y):
    return {"x": x, "y": y}


def polygon(pts, holes=None, bulges=None):
    e = {"id": 1, "layerId": "a", "attrs": {}, "kind": "polygon", "pts": [pt(*p) for p in pts]}
    if bulges:
        e["bulges"] = bulges
    if holes:
        e["holes"] = [{"pts": [pt(*p) for p in h]} for h in holes]
    return e


def polyline(pts, bulges=None):
    e = {"id": 1, "layerId": "a", "attrs": {}, "kind": "polyline", "pts": [pt(*p) for p in pts]}
    if bulges:
        e["bulges"] = bulges
    return e


def frac_pts(ps):
    return [(F(p["x"]), F(p["y"])) for p in ps]


def M(x):
    """A fraction at 50 digits."""
    return mp.mpf(x.numerator) / x.denominator if isinstance(x, F) else mp.mpf(x)


def ring_area(pts, bulges=()):
    """The signed area of a ring (counter-clockwise positive): exact for
    straight edges; an arc edge (bulge b, sweep 4·atan b, as DXF's) adds its
    segment r²/2·(sweep − sin sweep), at 50 digits."""
    n = len(pts)
    a = sum(pts[i][0] * pts[(i + 1) % n][1] - pts[(i + 1) % n][0] * pts[i][1] for i in range(n)) / 2
    if not any(bulges):
        return a
    total = M(a)
    for i, b in enumerate(bulges):
        if b == 0:
            continue
        p, q = pts[i], pts[(i + 1) % n]
        chord = mp.sqrt(M((q[0] - p[0]) ** 2 + (q[1] - p[1]) ** 2))
        sweep = 4 * mp.atan(M(F(b)))
        r = chord / (2 * abs(mp.sin(sweep / 2)))
        total += r * r / 2 * (sweep - mp.sin(sweep))
    return total


def area_of(rings, bulge_sets):
    """An area's size: its outer ring's less its holes'."""
    sizes = [abs(ring_area(r, b)) for r, b in zip(rings, bulge_sets)]
    if any(not isinstance(v, F) for v in sizes):
        sizes = [M(v) for v in sizes]
    return sizes[0] - sum(sizes[1:])


def meet(p, d, q, e):
    """Where the line p + t·d meets the line q + u·e; None when parallel."""
    cross = d[0] * e[1] - d[1] * e[0]
    # Parallel: the sine of the angle between them at most 1e-12.
    if cross * cross <= F(1, 10**24) * (d[0] ** 2 + d[1] ** 2) * (e[0] ** 2 + e[1] ** 2):
        return None
    t = ((q[0] - p[0]) * e[1] - (q[1] - p[1]) * e[0]) / cross
    return (p[0] + d[0] * t, p[1] + d[1] * t)


def shift(e, ring, edge, dist, check=True):
    """The shape's rings and the ring moved, or a refusal: first what stops
    the edge whatever the distance (its kind, the edge, an arc, a parallel
    neighbour), then what the distance does (a neighbour or the edge passed)."""
    if e.get("parts"):
        return MULTI
    if e["kind"] not in ("polygon", "polyline"):
        return NOT_PATH
    closed = e["kind"] == "polygon"
    if closed:
        rings = [frac_pts(e["pts"])] + [frac_pts(h["pts"]) for h in e.get("holes", [])]
        bulge_sets = [e.get("bulges") or []] + [h.get("bulges") or [] for h in e.get("holes", [])]
    else:
        rings = [frac_pts(e["pts"])]
        bulge_sets = [e.get("bulges") or []]
    if not (0 <= ring < len(rings)):
        return NO_EDGE
    pts, bulges = rings[ring], bulge_sets[ring]
    n = len(pts)
    edges = n if closed else n - 1
    if not (0 <= edge < edges):
        return NO_EDGE
    bulge = lambda i: (bulges[i] if i < len(bulges) else 0)  # noqa: E731
    i, j = edge, (edge + 1) % n
    if bulge(i) != 0:
        return CURVED
    a, b = pts[i], pts[j]
    t = (b[0] - a[0], b[1] - a[1])
    # The normal: an area's away from its inside, a polyline's right.
    right = (t[1], -t[0])
    if closed:
        ccw = ring_area(pts, bulges) > 0
        inside_left = ccw if ring == 0 else not ccw
        n_dir = right if inside_left else (-right[0], -right[1])
    else:
        n_dir = right
    length = t[0] ** 2 + t[1] ** 2
    # n_dir is as long as the edge; the move is dist along the unit normal.
    if length.denominator == 1 and math.isqrt(int(length)) ** 2 == int(length):
        scale = F(dist) / math.isqrt(int(length))
    else:
        # An irrational length: at 50 digits, then back to fractions for the exact steps.
        scale = F(str(mp.mpf(F(dist).numerator) / F(dist).denominator / mp.sqrt(mp.mpf(length.numerator) / length.denominator)))
    move = (n_dir[0] * scale, n_dir[1] * scale)
    a2, b2 = (a[0] + move[0], a[1] + move[1]), (b[0] + move[0], b[1] + move[1])
    # Each end: where the moved line meets its neighbour's line, or a free end square.
    ends = []
    for vertex, moved_to, nb_edge, nb_vertex in ((a, a2, (i - 1) % n, (i - 1) % n), (b, b2, j, (j + 1) % n)):
        free = not closed and ((vertex is a and i == 0) or (vertex is b and j == n - 1))
        if free:
            ends.append((moved_to, None))
            continue
        if bulge(nb_edge) != 0:
            return CURVED
        other = pts[nb_vertex]
        x = meet(moved_to, t, other, (vertex[0] - other[0], vertex[1] - other[1]))
        if x is None:
            return PARALLEL
        ends.append((x, other))
    new = list(pts)
    new[i], new[j] = ends[0][0], ends[1][0]
    if check:
        for (x, other), vertex in zip(ends, (a, b)):
            if other is None:
                continue
            old, now = (vertex[0] - other[0], vertex[1] - other[1]), (x[0] - other[0], x[1] - other[1])
            if old[0] * now[0] + old[1] * now[1] <= 0:
                return PASSED
        moved_edge = (new[j][0] - new[i][0], new[j][1] - new[i][1])
        if moved_edge[0] * t[0] + moved_edge[1] * t[1] <= 0:
            return PASSED
    return rings, bulge_sets, ring, new


def area_after(e, ring, edge, dist):
    rings, bulge_sets, r, new = shift(e, ring, edge, dist, check=False)
    rings = list(rings)
    rings[r] = new
    return area_of(rings, bulge_sets)


def for_area(e, ring, edge, target):
    if e["kind"] != "polygon":
        return NOT_AREA
    got = shift(e, ring, edge, 0, check=False)
    if isinstance(got, str):
        return got
    a0 = area_after(e, ring, edge, 0)
    ap, am = area_after(e, ring, edge, 1), area_after(e, ring, edge, -1)
    # A(d) is quadratic in d: its coefficients from three exact values.
    a0, ap, am = M(a0), M(ap), M(am)
    a2 = (ap + am - 2 * a0) / 2
    a1 = (ap - am) / 2
    c = a0 - M(F(target))
    disc = a1 * a1 - 4 * a2 * c
    if disc < 0 or a1 == 0:
        return NO_AREA
    # The root nearer 0, in the form that keeps its digits when a2 is small.
    d = float(-2 * c / (a1 + mp.sign(a1) * mp.sqrt(disc)))
    # The distance found must be one the edge can move by.
    got = shift(e, ring, edge, d)
    return got if isinstance(got, str) else d


def written(e, ring, edge, dist):
    got = shift(e, ring, edge, dist)
    if isinstance(got, str):
        return {"problem": got}
    rings, bulge_sets, r, new = got
    rings = list(rings)
    rings[r] = new
    out = {"pts": [[float(p[0]), float(p[1])] for p in rings[0]]}
    if len(rings) > 1:
        out["holes"] = [[[float(p[0]), float(p[1])] for p in h] for h in rings[1:]]
    if e.get("bulges"):
        out["bulges"] = e["bulges"]
    if e["kind"] == "polygon":
        out["area"] = float(area_of(rings, bulge_sets))
    return out


SQUARE = polygon([(0, 0), (10, 0), (10, 10), (0, 10)])
CW = polygon([(0, 0), (0, 10), (10, 10), (10, 0)])
TRAPEZE = polygon([(0, 0), (20, 0), (15, 10), (5, 10)])
HOLED = polygon([(0, 0), (20, 0), (20, 20), (0, 20)], holes=[[(5, 5), (15, 5), (15, 15), (5, 15)]])
OPEN = polyline([(0, 0), (10, 0), (10, 10), (20, 10)])
ARCED = polygon([(0, 0), (10, 0), (10, 10), (0, 10)], bulges=[0, 0.5, 0, 0])
FAR = polygon([(487000.25, 4420000.5), (487030.25, 4420000.5), (487030.25, 4420020.5), (487000.25, 4420020.5)])
PARALLEL_NEXT = polygon([(0, 0), (10, 0), (20, 0), (20, 10), (0, 10)])
ARC_TOP = polygon([(0, 0), (10, 0), (10, 10), (0, 10)], bulges=[0, 0, 0.25, 0])
KITE = polygon([(0, 0), (10, 0), (10, 2), (0, 10)])
SLANT = polygon([(0, 0), (12, 0), (14, 4), (0, 4)])
LINE = {"id": 1, "layerId": "a", "attrs": {}, "kind": "line", "a": pt(0, 0), "b": pt(10, 0)}
PARTS = dict(polygon([(0, 0), (10, 0), (10, 10), (0, 10)]), parts=[{"pts": [pt(20, 0), pt(30, 0), pt(30, 10), pt(20, 10)]}])

CASES = [
    ("a square's bottom edge out by 2", SQUARE, 0, 0, 2),
    ("a clockwise square: out is still out", CW, 0, 1, 2),
    ("a trapeze's top edge in by 4: the slanted sides shorten", TRAPEZE, 0, 2, -4),
    ("a hole's edge: out of the area is into the hole", HOLED, 1, 0, 1),
    ("an open polyline's middle edge to its right", OPEN, 0, 1, 3),
    ("an open polyline's first edge: its free end moves square", OPEN, 0, 0, -2),
    ("map coordinates", FAR, 0, 1, 0.75),
    ("a slanted edge of irrational length out by 1", SLANT, 0, 1, 1),
    ("an arc elsewhere on the ring stays", ARC_TOP, 0, 0, 1),
    ("a curved edge is refused", ARCED, 0, 1, 1),
    ("a curved neighbour is refused", ARCED, 0, 0, 1),
    ("a neighbour parallel to the edge is refused", PARALLEL_NEXT, 0, 0, 1),
    ("an edge passing its neighbour is refused", TRAPEZE, 0, 2, -12),
    ("no such edge", SQUARE, 0, 9, 1),
    ("no such ring", SQUARE, 1, 0, 1),
    ("a multi-part area is refused", PARTS, 0, 0, 1),
    ("a line is not an area's or a polyline's edge", LINE, 0, 0, 1),
]

TARGETS = [
    ("a square grown to 120 m²: its edge out by 2", SQUARE, 0, 0, 120),
    ("a trapeze to 180 m² by its top edge: a quadratic", TRAPEZE, 0, 2, 180),
    ("a trapeze to a size no shift of its top edge reaches", TRAPEZE, 0, 2, 100000),
    ("a target reached only past a neighbour is refused", KITE, 0, 0, 30),
    ("a slanted edge to 70 m²", SLANT, 0, 1, 70),
    ("an area with an arc elsewhere to 125 m²", ARC_TOP, 0, 0, 125),
    ("a polyline has no area", OPEN, 0, 1, 50),
]


def cases():
    return {
        "format": "kentos.edge-shift-cases",
        "version": 1,
        "source": SOURCE,
        "shifts": [{"name": n, "shape": e, "ring": r, "edge": k, "distance": d, "expect": written(e, r, k, d)} for n, e, r, k, d in CASES],
        "targets": [
            {"name": n, "shape": e, "ring": r, "edge": k, "target": t, "expect": (lambda v: {"problem": v} if isinstance(v, str) else {"distance": v})(for_area(e, r, k, t))}
            for n, e, r, k, t in TARGETS
        ],
    }


def main():
    text = json.dumps(cases(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            sys.exit("fixtures/edge-shift/v1/cases.json güncel değil; yeniden yazmak için --check'siz çalıştırın.")
        print("fixtures/edge-shift/v1/cases.json güncel.")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print("fixtures/edge-shift/v1/cases.json yazıldı.")


if __name__ == "__main__":
    main()
