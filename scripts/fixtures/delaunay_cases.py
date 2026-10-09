"""Delaunay triangulation's cases (docs/adr/0232 §4), worked out without
KentOS code: point sets in general position triangulated by qhull (through
matplotlib.tri), each triangle then checked with exact rational arithmetic
(no point of the set strictly inside its circumcircle; counter-clockwise);
the hull is the triangulation's edges without a neighbour. Degenerate sets
(a regular grid, points on a circle, collinear points on the hull, equal
points) have no unique triangulation: they carry the properties the core
must keep (point count used, hull points, triangle count by Euler's
formula) and the core's own tests check the empty-circle rule exactly.

    python3 scripts/fixtures/delaunay_cases.py           # writes the file
    python3 scripts/fixtures/delaunay_cases.py --check   # writes nothing; compares

Writes fixtures/delaunay/v1/cases.json.
"""
import json
import sys
from fractions import Fraction

import numpy as np
import matplotlib.tri as mtri

OUT = 'fixtures/delaunay/v1/cases.json'


def orient(a, b, c):
    return (Fraction(b[0]) - Fraction(a[0])) * (Fraction(c[1]) - Fraction(a[1])) - (Fraction(b[1]) - Fraction(a[1])) * (Fraction(c[0]) - Fraction(a[0]))


def incircle(a, b, c, d):
    """Positive when d is strictly inside the circle through a, b, c (counter-clockwise)."""
    ax, ay = Fraction(a[0]) - Fraction(d[0]), Fraction(a[1]) - Fraction(d[1])
    bx, by = Fraction(b[0]) - Fraction(d[0]), Fraction(b[1]) - Fraction(d[1])
    cx, cy = Fraction(c[0]) - Fraction(d[0]), Fraction(c[1]) - Fraction(d[1])
    return (ax * ax + ay * ay) * (bx * cy - cx * by) + (bx * bx + by * by) * (cx * ay - ax * cy) + (cx * cx + cy * cy) * (ax * by - bx * ay)


def qhull(points):
    """qhull's triangles (each counter-clockwise, checked exactly) and hull points."""
    xs = np.array([p[0] for p in points], dtype=float)
    ys = np.array([p[1] for p in points], dtype=float)
    t = mtri.Triangulation(xs, ys)
    tris = []
    for tri in t.triangles:
        a, b, c = (int(v) for v in tri)
        if orient(points[a], points[b], points[c]) < 0:
            b, c = c, b
        assert orient(points[a], points[b], points[c]) > 0, 'a flat triangle from qhull'
        tris.append((a, b, c))
    # Exact: no point strictly inside a circumcircle (a bounding box of each circle prunes the check).
    for a, b, c in tris:
        pa, pb, pc = points[a], points[b], points[c]
        for i, p in enumerate(points):
            if i in (a, b, c):
                continue
            assert incircle(pa, pb, pc, p) <= 0, f'qhull triangle {a, b, c} is not Delaunay: point {i}'
    hull = set()
    for k, tri in enumerate(t.triangles):
        for j in range(3):
            if t.neighbors[k][j] == -1:
                hull.add(int(tri[j]))
                hull.add(int(tri[(j + 1) % 3]))
    return tris, sorted(hull)


def canonical(tris):
    """Each triangle rotated to start at its least point, the list sorted."""
    out = []
    for a, b, c in tris:
        m = min(a, b, c)
        while a != m:
            a, b, c = b, c, a
        out.append([a, b, c])
    return sorted(out)


def general(name, points):
    tris, hull = qhull(points)
    return {'name': name, 'points': points, 'triangles': canonical(tris), 'hull': hull}


def degenerate(name, points, used, hull, note):
    """A set whose triangulation is not unique: the counts the core must give."""
    return {'name': name, 'points': points, 'used': used, 'hullPoints': hull,
            'triangleCount': 2 * used - hull - 2, 'note': note}


def refused(name, points):
    return {'name': name, 'points': points, 'error': 'too_few'}


def build():
    rng = np.random.default_rng(232)
    cases = []
    pts = [[float(x), float(y)] for x, y in rng.random((100, 2)) * 100.0]
    cases.append(general('rastgele-100', pts))
    # TM coordinates: 300 points over a kilometre, on the 2⁻¹⁰ m grid as survey data are.
    tm = rng.random((300, 2)) * 1000.0
    pts = [[round(500000.0 + x, 3), round(4420000.0 + y, 3)] for x, y in tm]
    cases.append(general('tm-300', pts))
    # Clusters: three tight groups and a few points between.
    pts = []
    for cx, cy in ((10.0, 10.0), (60.0, 15.0), (35.0, 70.0)):
        for x, y in rng.normal(0.0, 2.0, (50, 2)):
            pts.append([float(cx + x), float(cy + y)])
    for x, y in rng.random((10, 2)) * 80.0:
        pts.append([float(x), float(y)])
    cases.append(general('kumeler', pts))
    # A thin strip: long, narrow triangles.
    pts = [[float(x), float(y)] for x, y in zip(rng.random(120) * 1000.0, rng.random(120) * 3.0)]
    cases.append(general('serit', pts))

    # Degenerate sets.
    grid = [[float(i), float(j)] for j in range(8) for i in range(10)]
    cases.append(degenerate('izgara-10x8', grid, 80, 2 * (10 + 8) - 4, 'her dört komşu eş çemberli'))
    circle = [[float(x), float(y)] for x, y in ((5, 0), (4, 3), (3, 4), (0, 5), (-3, 4), (-4, 3), (-5, 0), (-4, -3),
                                                (-3, -4), (0, -5), (3, -4), (4, -3))]
    cases.append(degenerate('cember-12', circle, 12, 12, 'bütün noktalar 5 yarıçaplı çemberde'))
    line_hull = [[float(i), 0.0] for i in range(11)] + [[5.0, 6.0], [4.0, 2.0], [6.0, 1.5]]
    cases.append(degenerate('kenarda-dogru', line_hull, 14, 12, 'alt kenarda on bir doğrusal nokta'))
    dup = [[0.0, 0.0], [4.0, 0.0], [4.0, 3.0], [0.0, 3.0], [4.0, 0.0], [2.0, 1.0], [2.0, 1.0]]
    cases.append(degenerate('esit-noktalar', dup, 5, 4, 'iki nokta ikişer kez'))

    cases.append(refused('iki-nokta', [[0.0, 0.0], [1.0, 1.0]]))
    cases.append(refused('dogrusal', [[float(i), float(2 * i)] for i in range(6)]))
    cases.append(refused('hepsi-esit', [[3.0, 3.0]] * 5))
    return {'format': 'kentos.delaunay', 'version': 1, 'cases': cases}


def main():
    data = build()
    text = json.dumps(data, ensure_ascii=False, indent=1) + '\n'
    if '--check' in sys.argv:
        with open(OUT, encoding='utf-8') as f:
            if f.read() != text:
                print(f'{OUT} differs from the reference', file=sys.stderr)
                sys.exit(1)
        print(f'{OUT} matches')
        return
    import os
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, 'w', encoding='utf-8') as f:
        f.write(text)
    print(f'{OUT} written')


if __name__ == '__main__':
    main()
