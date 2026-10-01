#!/usr/bin/env python3
"""Independent reference of Hızlı ölçü (docs/adr/0147 §7).

Writes fixtures/dimension/v1/quick.json from the rules alone, with Python's
standard library and no KentOS code: for a selection of lines, polylines and
areas, the cursor and an optional typed distance, the dimensions Hızlı ölçü
writes. The geometry core (`geom::dimension::quick::quick_dimensions`) must
give the same within 1e-9 m on both platforms
(crates/shared/geometry-core/tests/dimensions.rs; the web through its WASM).

The rules:

- The objects are taken in the order given. A line is a path of one edge, a
  polyline an open path; an area is rings: its outer ring, then its holes,
  then each further part's outer ring and its holes. Anything else is
  skipped (and counted).
- A path or ring is its edges in order, from each vertex to the next (a ring
  closes back to its first). A bulge β on the edge from vertex i makes it an
  arc of sweep 4·atan(β), counter-clockwise for β > 0; its centre is the
  chord's middle plus the chord's left normal times (L/2)·cot(sweep/2), its
  radius (L/2)/|sin(sweep/2)|, L the chord's length. An edge whose two ends
  are the same point is left out.
- An edge is measured once: one with the same two ends as an edge before it
  (either way round) and, for an arc, the same arc (the same bulge the same
  way round, the opposite bulge the other way) is left out.
- d, the distance: the typed value's size, or else the cursor's distance to
  the nearest edge of the whole selection (a segment: to its nearest point;
  an arc: |cursor − centre| − radius in size when the cursor's direction from
  the centre lies within the arc, else to its nearer end).
- s, the side, 1 left of the way the edges run and −1 right:
  - an area's outer ring: outside it, −1 when it runs counter-clockwise (its
    area, the arcs' segments included, over 0), 1 otherwise;
  - a hole: into it, 1 when it runs counter-clockwise, −1 otherwise;
  - an open path: the cursor's side of the path's nearest edge (the first of
    the nearest): a segment's by the cross product's sign, 0 counted as left;
    an arc's, left being towards its centre when it runs counter-clockwise
    and away from it when clockwise (on its circle counted as left).
- A straight edge from a to b: an aligned dimension from a to b, offset s·d.
- An arc: an arc length dimension about its centre, its ends
  counter-clockwise (the vertex it starts from first when it runs
  counter-clockwise, else the other), offset −s·d when it runs
  counter-clockwise and s·d when clockwise (an arc length's offset is
  outwards from its centre).

Usage:
    python3 scripts/fixtures/quick_dimension_cases.py           # writes the file
    python3 scripts/fixtures/quick_dimension_cases.py --check   # compares with the file
"""

import json
import math
import random
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/dimension/v1/quick.json"

HEIGHT = 2.5


# ── Edges ───────────────────────────────────────────────────────────────────


def arc_of(a, b, bulge):
    """The arc on the chord a→b with this bulge: centre, radius, start angle, signed sweep."""
    sweep = 4.0 * math.atan(bulge)
    dx, dy = b[0] - a[0], b[1] - a[1]
    length = math.hypot(dx, dy)
    nx, ny = -dy / length, dx / length
    k = (length / 2.0) / math.tan(sweep / 2.0)
    c = ((a[0] + b[0]) / 2.0 + nx * k, (a[1] + b[1]) / 2.0 + ny * k)
    r = (length / 2.0) / abs(math.sin(sweep / 2.0))
    a0 = math.atan2(a[1] - c[1], a[0] - c[0])
    return c, r, a0, sweep


def edges_of(pts, bulges, closed):
    """(a, b, bulge) for each edge with two distinct ends, in order."""
    n = len(pts)
    count = n if closed else n - 1
    out = []
    for i in range(max(count, 0)):
        a, b = pts[i], pts[(i + 1) % n]
        if a == b:
            continue
        beta = bulges[i] if bulges and i < len(bulges) else 0.0
        out.append((a, b, beta))
    return out


def distance_to(edge, p):
    a, b, beta = edge
    if beta == 0.0:
        dx, dy = b[0] - a[0], b[1] - a[1]
        t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)
        t = min(1.0, max(0.0, t))
        return math.hypot(p[0] - (a[0] + t * dx), p[1] - (a[1] + t * dy))
    c, r, a0, sweep = arc_of(a, b, beta)
    phi = math.atan2(p[1] - c[1], p[0] - c[0])
    turn = (phi - a0) % (2.0 * math.pi) if sweep > 0 else (a0 - phi) % (2.0 * math.pi)
    if turn <= abs(sweep):
        return abs(math.hypot(p[0] - c[0], p[1] - c[1]) - r)
    return min(math.hypot(p[0] - a[0], p[1] - a[1]), math.hypot(p[0] - b[0], p[1] - b[1]))


def cursor_side(edge, p):
    """1 when the cursor is left of the edge's way, −1 right."""
    a, b, beta = edge
    if beta == 0.0:
        cross = (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
        return -1 if cross < 0 else 1
    c, r, _, sweep = arc_of(a, b, beta)
    inside = math.hypot(p[0] - c[0], p[1] - c[1]) <= r
    # Left of a counter-clockwise arc is towards its centre.
    if sweep > 0:
        return 1 if inside else -1
    return -1 if inside else 1


def ring_area(pts, bulges):
    """Signed area, the arcs' segments included (positive: counter-clockwise)."""
    n = len(pts)
    s = 0.0
    for i in range(n):
        a, b = pts[i], pts[(i + 1) % n]
        s += a[0] * b[1] - b[0] * a[1]
    area = s / 2.0
    for a, b, beta in edges_of(pts, bulges, True):
        if beta != 0.0:
            _, r, _, sweep = arc_of(a, b, beta)
            area += r * r / 2.0 * (sweep - math.sin(sweep))
    return area


# ── The selection ───────────────────────────────────────────────────────────


def xy(p):
    return (p["x"], p["y"])


def paths_of(obj):
    """(edges, rule) for each path or ring: rule 'open', 'outer' or 'hole'."""
    kind = obj["kind"]
    if kind == "line":
        return [(edges_of([xy(obj["a"]), xy(obj["b"])], None, False), "open", None)]
    if kind == "polyline":
        pts = [xy(p) for p in obj["pts"]]
        return [(edges_of(pts, obj.get("bulges"), False), "open", None)]
    if kind == "polygon":
        out = []
        rings = [(obj["pts"], obj.get("bulges"), obj.get("holes") or [])]
        rings += [(part["pts"], part.get("bulges"), part.get("holes") or []) for part in obj.get("parts") or []]
        for pts, bulges, holes in rings:
            pts = [xy(p) for p in pts]
            out.append((edges_of(pts, bulges, True), "outer", ring_area(pts, bulges)))
            for h in holes:
                hp = [xy(p) for p in h["pts"]]
                out.append((edges_of(hp, h.get("bulges"), True), "hole", ring_area(hp, h.get("bulges"))))
        return out
    return None


def quick(objects, at, typed):
    paths = []
    skipped = 0
    for obj in objects:
        got = paths_of(obj)
        if got is None:
            skipped += 1
            continue
        paths.extend(got)
    every = [e for edges, _, _ in paths for e in edges]
    if typed is not None:
        d = abs(typed)
    elif every:
        d = min(distance_to(e, at) for e in every)
    else:
        d = 0.0
    seen = []
    out = []
    for edges, rule, area in paths:
        if not edges:
            continue
        if rule == "outer":
            s = -1 if area > 0 else 1
        elif rule == "hole":
            s = 1 if area > 0 else -1
        else:
            nearest = edges[0]
            best = distance_to(nearest, at)
            for e in edges[1:]:
                dist = distance_to(e, at)
                if dist < best:
                    nearest, best = e, dist
            s = cursor_side(nearest, at)
        for a, b, beta in edges:
            if any((a == sa and b == sb and beta == sbeta) or (a == sb and b == sa and beta == -sbeta) for sa, sb, sbeta in seen):
                continue
            seen.append((a, b, beta))
            if beta == 0.0:
                out.append({"a": pt(a), "b": pt(b), "offset": s * d})
                continue
            c, _, _, sweep = arc_of(a, b, beta)
            first, last = (a, b) if sweep > 0 else (b, a)
            offset = -s * d if sweep > 0 else s * d
            out.append({"a": pt(first), "b": pt(last), "offset": offset, "style": "arcLength", "c": pt(c)})
    return {"dimensions": out, "skipped": skipped}


# ── Cases ───────────────────────────────────────────────────────────────────


def pt(p):
    return {"x": p[0], "y": p[1]}


def P(*ps):
    return [pt(p) for p in ps]


QUARTER = math.tan(math.pi / 8)  # a quarter turn's bulge

SQUARE = P((0, 0), (40, 0), (40, 30), (0, 30))

CASES = [
    ("kare alan, saatin tersine: ölçüler dışarıda, imlecin en yakın kenara uzaklığında", [{"kind": "polygon", "pts": SQUARE}], (20, -5), None),
    ("kare alan, saat yönünde: yine dışarıda", [{"kind": "polygon", "pts": list(reversed(SQUARE))}], (20, -5), None),
    ("imleç alanın içinde: uzaklık en yakın kenara, ölçüler yine dışarıda", [{"kind": "polygon", "pts": SQUARE}], (20, 27), None),
    (
        "iki komşu parsel: ortak kenar bir kez",
        [
            {"kind": "polygon", "pts": P((0, 0), (20, 0), (20, 30), (0, 30))},
            {"kind": "polygon", "pts": P((20, 0), (40, 0), (40, 30), (20, 30))},
        ],
        (10, -4),
        None,
    ),
    ("açık çoklu çizgi: imlecin tarafında, en yakın kenarına göre", [{"kind": "polyline", "pts": P((0, 0), (10, 0), (10, 10))}], (5, 3), None),
    ("açık çoklu çizgi, imleç öbür yanda", [{"kind": "polyline", "pts": P((0, 0), (10, 0), (10, 10))}], (13, 6), None),
    ("çizgi, yazılan uzaklık işaretsiz alınır", [{"kind": "line", "a": pt((0, 0)), "b": pt((30, 0))}], (10, -2), -4.0),
    (
        "dışa kabaran yaylı kenar: yay uzunluğu dışarıda",
        [{"kind": "polygon", "pts": P((0, 0), (20, 0), (20, 20), (0, 20)), "bulges": [0, QUARTER, 0, 0]}],
        (10, -3),
        None,
    ),
    (
        "içe çöken yaylı kenar: yay uzunluğu yine dışarıda (merkeze doğru)",
        [{"kind": "polygon", "pts": P((0, 0), (20, 0), (20, 20), (0, 20)), "bulges": [0, -QUARTER, 0, 0]}],
        (10, -3),
        None,
    ),
    (
        "delikli alan: deliğin kenarları deliğin içine",
        [{"kind": "polygon", "pts": P((0, 0), (50, 0), (50, 40), (0, 40)), "holes": [{"pts": P((10, 10), (20, 10), (20, 20), (10, 20))}]}],
        (25, -3),
        None,
    ),
    (
        "saat yönündeki delik de içine",
        [{"kind": "polygon", "pts": P((0, 0), (50, 0), (50, 40), (0, 40)), "holes": [{"pts": P((10, 10), (10, 20), (20, 20), (20, 10))}]}],
        (25, -3),
        None,
    ),
    (
        "çok parçalı alan: her parça kendi dışında",
        [{"kind": "polygon", "pts": P((0, 0), (10, 0), (10, 10), (0, 10)), "parts": [{"pts": P((30, 0), (30, 10), (40, 10), (40, 0))}]}],
        (20, 5),
        2.0,
    ),
    (
        "yaylı açık çoklu çizgi: imleç yayın içinde",
        [{"kind": "polyline", "pts": P((0, 0), (10, 0), (20, 10)), "bulges": [0, QUARTER]}],
        (12, 6),
        None,
    ),
    (
        "yaylı açık çoklu çizgi, saat yönündeki yay, imleç dışında",
        [{"kind": "polyline", "pts": P((0, 0), (10, 0), (20, -10)), "bulges": [0, -QUARTER]}],
        (25, 0),
        None,
    ),
    (
        "iki alanın ortak yayı: ters yönde, karşıt kabarıklıkla, bir kez",
        [
            {"kind": "polygon", "pts": P((0, 0), (20, 0), (20, 20), (0, 20)), "bulges": [0, QUARTER, 0, 0]},
            {"kind": "polygon", "pts": P((20, 0), (40, 0), (40, 20), (20, 20)), "bulges": [0, 0, 0, -QUARTER]},
        ],
        (10, -3),
        None,
    ),
    (
        "aynı kirişin öbür yandaki yayı ayrı ölçülür",
        [
            {"kind": "polygon", "pts": P((0, 0), (20, 0), (20, 20), (0, 20)), "bulges": [0, QUARTER, 0, 0]},
            {"kind": "polygon", "pts": P((20, 0), (40, 0), (40, 20), (20, 20)), "bulges": [0, 0, 0, QUARTER]},
        ],
        (10, -3),
        None,
    ),
    (
        "nokta ve daire atlanır, sayılır; tekrarlanan köşe kenar değildir",
        [
            {"kind": "point", "p": pt((5, 5))},
            {"kind": "circle", "c": pt((50, 50)), "r": 3},
            {"kind": "polyline", "pts": P((0, 0), (0, 0), (10, 0))},
        ],
        (5, 2),
        None,
    ),
    ("imleç kenarın üstünde: uzaklık 0", [{"kind": "line", "a": pt((0, 0)), "b": pt((10, 0))}], (4, 0), None),
    ("ölçülecek bir şey yok", [{"kind": "circle", "c": pt((0, 0)), "r": 3}], (5, 5), None),
]


def random_cases():
    """Random rings and paths about a coordinate far from the origin, as a project's are."""
    rng = random.Random(147)
    E, N = 487000.0, 4420000.0
    out = []
    for i in range(12):
        objects = []
        for _ in range(rng.randint(1, 3)):
            cx, cy = E + rng.uniform(-200, 200), N + rng.uniform(-200, 200)
            n = rng.randint(3, 6)
            # A star-shaped ring: its vertices round its centre in order, one way or the other.
            turns = sorted(rng.uniform(0, 2 * math.pi) for _ in range(n))
            if rng.random() < 0.5:
                turns.reverse()
            pts = [(cx + rng.uniform(20, 60) * math.cos(t), cy + rng.uniform(20, 60) * math.sin(t)) for t in turns]
            bulges = [rng.choice([0.0, 0.0, rng.uniform(-0.6, 0.6)]) for _ in pts]
            kind = rng.choice(["polygon", "polygon", "polyline"])
            obj = {"kind": kind, "pts": P(*pts)}
            if any(bulges):
                obj["bulges"] = bulges if kind == "polygon" else bulges[: len(pts) - 1]
            objects.append(obj)
        at = (E + rng.uniform(-260, 260), N + rng.uniform(-260, 260))
        typed = rng.choice([None, None, rng.uniform(-8, 8)])
        out.append((f"rastgele {i + 1}", objects, at, typed))
    return out


def case(name, objects, at, typed):
    return {
        "name": name,
        "objects": objects,
        "at": pt(at),
        "typed": typed,
        "height": HEIGHT,
        "want": quick(objects, at, typed),
    }


def document():
    cases = CASES + random_cases()
    return {
        "format": "kentos.quick-dimension-cases",
        "version": 1,
        "note": "Written by scripts/fixtures/quick_dimension_cases.py from docs/adr/0147 §7 alone; the geometry core gives the same within 1e-9 m.",
        "cases": [case(*c) for c in cases],
    }


def main():
    doc = document()
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    count = len(doc["cases"])
    if "--check" in sys.argv[1:]:
        if OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} kurallardan yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)")
            return 1
        print(f"{OUT.relative_to(ROOT)}: {count} durum kurallarla tutarlı")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({count} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
