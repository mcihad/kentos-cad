#!/usr/bin/env python3
"""The terrain surface's shared cases (docs/adr/0900, plan/road.md Y2.1), from an independent reference: Shewchuk's
Triangle (the `triangle` package) triangulates, Python's fractions give the heights; no KentOS code.

    uv run --no-project --python 3.11 --with triangle --with numpy python3 scripts/fixtures/road_surface_cases.py
    ... --check   # fails when fixtures/road/v1/surfaces.json is not what this writes

A case's triangles and constrained edges are written by their corners' coordinates (the indices are each
program's own). Points are in general position (random), so the constrained Delaunay triangulation is unique
and Triangle's and KentOS's are the same set; a regular grid's is not unique, so its case checks the heights on
its plane and the triangle count only.
"""

import json
import random
import sys
from fractions import Fraction
from pathlib import Path

import numpy as np
import triangle

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "surfaces.json"


def pt(p):
    return [float(p[0]), float(p[1])]


def tri_sets(vertices, tris):
    out = []
    for t in tris:
        corners = sorted(pt(vertices[i]) for i in t)
        out.append(corners)
    out.sort()
    return out


def edge_sets(vertices, edges):
    out = []
    for a, b in edges:
        out.append(sorted([pt(vertices[a]), pt(vertices[b])]))
    out.sort()
    return out


def barycentric(xy, z, tris, q):
    """The height at q on the triangles (exact fractions), or None outside them."""
    qx, qy = Fraction(q[0]), Fraction(q[1])
    for t in tris:
        (ax, ay), (bx, by), (cx, cy) = [(Fraction(xy[i][0]), Fraction(xy[i][1])) for i in t]
        d = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax)
        wa = ((bx - qx) * (cy - qy) - (by - qy) * (cx - qx)) / d
        wb = ((cx - qx) * (ay - qy) - (cy - qy) * (ax - qx)) / d
        wc = 1 - wa - wb
        if wa >= 0 and wb >= 0 and wc >= 0:
            return float(wa * Fraction(z[t[0]]) + wb * Fraction(z[t[1]]) + wc * Fraction(z[t[2]]))
    return None


class Case:
    def __init__(self, name, note):
        self.name = name
        self.note = note
        self.points = []  # (x, y, z, source)
        self.breaklines = []  # ([(x, y, z)], closed, source)
        self.boundary = None
        self.voids = []
        self.edge = {"kind": "none"}
        self.queries = []

    def input(self):
        d = {
            "points": [{"x": x, "y": y, "z": z, **({"source": s} if s else {})} for x, y, z, s in self.points],
            "breaklines": [
                {"points": [list(p) for p in pts], "closed": closed, **({"source": s} if s else {})}
                for pts, closed, s in self.breaklines
            ],
            "edge": self.edge,
        }
        if self.boundary:
            d["boundary"] = {"points": [list(p) for p in self.boundary]}
        if self.voids:
            d["voids"] = [{"points": [list(p) for p in r]} for r in self.voids]
        return d

    def corners(self):
        """Each place once, its first height (the cases have no near duplicates)."""
        xy, z, at = [], [], {}
        for x, y, h, _ in self.points:
            if (x, y) not in at:
                at[(x, y)] = len(xy)
                xy.append((x, y))
                z.append(h)
        segs = []
        for pts, closed, _ in self.breaklines:
            ids = []
            for x, y, h in pts:
                if (x, y) not in at:
                    at[(x, y)] = len(xy)
                    xy.append((x, y))
                    z.append(h)
                ids.append(at[(x, y)])
            n = len(ids)
            for k in range(n - 1 + (1 if closed else 0)):
                segs.append((ids[k], ids[(k + 1) % n]))
        return xy, z, at, segs


def triangulate(xy, segs, holes=None, clip=False):
    a = {"vertices": np.array(xy, dtype=np.float64)}
    if segs:
        a["segments"] = np.array(segs, dtype=np.int32)
    if holes:
        a["holes"] = np.array(holes, dtype=np.float64)
    t = triangle.triangulate(a, "p" if clip else "pc")
    assert len(t["vertices"]) == len(xy), "Triangle added points"
    return t["triangles"].tolist()


def inside_point(ring):
    """A point strictly inside a ring, for Triangle's holes: a triangle of an ear's centroid (convex rings here)."""
    (ax, ay), (bx, by), (cx, cy) = ring[0], ring[1], ring[2]
    return [(ax + bx + cx) / 3, (ay + by + cy) / 3]


def solve(case):
    xy, z, at, segs = case.corners()
    data = triangulate(xy, segs)
    rings = ([case.boundary] if case.boundary else []) + case.voids
    for ring in rings:
        ids = []
        for p in ring:
            p = tuple(p)
            if p not in at:
                h = barycentric(xy, z, data, p)
                assert h is not None, (case.name, p)
                at[p] = len(xy)
                xy.append(p)
                z.append(h)
            ids.append(at[p])
        for k in range(len(ids)):
            segs.append((ids[k], ids[(k + 1) % len(ids)]))
    tris = triangulate(xy, segs, [inside_point(v) for v in case.voids] or None, clip=bool(case.boundary))
    if not case.boundary and case.voids:
        # Triangle eats a hole only inside segments; with no boundary 'pc' keeps the hull and the holes go.
        pass
    removed = 0
    if case.edge["kind"] == "maxEdge":
        tris, removed = trim(xy, tris, set(map(frozenset, segs)), case.edge["length"])
    pieces, split = split_at_points(xy, segs)
    expect = {
        "triangles": tri_sets(xy, tris),
        "constrained": edge_sets(xy, sorted({tuple(sorted(s)) for s in pieces})),
        "split": split,
        "trimmed": removed,
        "heights": [],
    }
    for q in case.queries:
        h = barycentric(xy, z, tris, q)
        expect["heights"].append({"at": q, "z": h})
    return expect


def split_at_points(xy, segs):
    """Each piece cut at the corners that lie exactly on it, strictly between its ends (fractions)."""
    out, split = [], 0
    for a, b in segs:
        (ax, ay), (bx, by) = [(Fraction(xy[i][0]), Fraction(xy[i][1])) for i in (a, b)]
        on = []
        for i, (x, y) in enumerate(xy):
            if i in (a, b):
                continue
            x, y = Fraction(x), Fraction(y)
            if (bx - ax) * (y - ay) - (by - ay) * (x - ax) != 0:
                continue
            t = ((x - ax) * (bx - ax) + (y - ay) * (by - ay)) / ((bx - ax) ** 2 + (by - ay) ** 2)
            if 0 < t < 1:
                on.append((t, i))
        chain = [a] + [i for _, i in sorted(on)] + [b]
        split += len(on)
        out += list(zip(chain, chain[1:]))
    return out, split


def trim(xy, tris, constrained, length):
    """ADR 0900 §6: an outer triangle whose outer, unconstrained edge is longer than `length` goes, until none is."""
    tris = [tuple(t) for t in tris]
    removed = 0
    while True:
        count = {}
        for t in tris:
            for k in range(3):
                e = frozenset((t[k], t[(k + 1) % 3]))
                count[e] = count.get(e, 0) + 1
        keep, changed = [], False
        for t in tris:
            gone = False
            for k in range(3):
                a, b = t[k], t[(k + 1) % 3]
                e = frozenset((a, b))
                if count[e] == 1 and e not in constrained:
                    (ax, ay), (bx, by) = xy[a], xy[b]
                    if ((bx - ax) ** 2 + (by - ay) ** 2) ** 0.5 > length:
                        gone = True
                        break
            if gone:
                removed += 1
                changed = True
            else:
                keep.append(t)
        tris = keep
        if not changed:
            return [list(t) for t in tris], removed


def ground(x, y):
    return round(100 + 0.05 * x - 0.02 * y + 3.0 * ((x - 50) / 50) ** 2, 4)


def cases():
    rnd = random.Random(900)
    out = []

    c = Case("dağınık noktalar", "60 rastgele nokta, kısıtsız: Delaunay'ın kendisi, kotlar düzlemlerden")
    for i in range(60):
        x, y = round(rnd.uniform(0, 100), 3), round(rnd.uniform(0, 100), 3)
        c.points.append((x, y, ground(x, y), f"p{i}"))
    c.queries = [[round(rnd.uniform(10, 90), 3), round(rnd.uniform(10, 90), 3)] for _ in range(8)] + [[150.0, 50.0]]
    out.append(c)

    c = Case("kırık hatlar", "40 nokta ve iki kırık hat (yol kenarı ve dere): parçaları üçgen kenarı")
    for i in range(40):
        x, y = round(rnd.uniform(0, 100), 3), round(rnd.uniform(0, 100), 3)
        c.points.append((x, y, ground(x, y), f"p{i}"))
    c.points += [(-1.0, -1.0, 99.0, "k1"), (101.0, -1.0, 104.0, "k2"), (101.0, 101.0, 102.0, "k3"), (-1.0, 101.0, 97.0, "k4")]
    c.breaklines.append(([(5.137, 20.511, 101.0), (40.271, 25.903, 102.5), (70.719, 21.355, 103.25), (95.33, 30.11, 104.0)], False, "yol-kenari"))
    c.breaklines.append(([(10.555, 80.121, 96.0), (45.377, 60.633, 95.25), (88.919, 70.477, 94.5)], False, "dere"))
    c.queries = [[50.0, 40.0], [40.271, 25.903], [60.0, 62.0], [20.0, 90.0]]
    out.append(c)

    c = Case("sınır ve boşluk", "noktalar ve bir kırık hat; drape sınır ve göl boşluğu, dışı ve içi atılır")
    for i in range(50):
        x, y = round(rnd.uniform(0, 100), 3), round(rnd.uniform(0, 100), 3)
        c.points.append((x, y, ground(x, y), f"p{i}"))
    c.points += [(-2.0, -2.0, 99.0, "k1"), (102.0, -2.0, 104.0, "k2"), (102.0, 102.0, 102.0, "k3"), (-2.0, 102.0, 97.0, "k4")]
    c.breaklines.append(([(12.345, 50.111, 100.0), (88.777, 52.333, 103.0)], False, "sırt"))
    c.boundary = [[5.5, 5.25], [94.75, 6.125], [93.5, 94.875], [6.25, 93.625]]
    c.voids = [[[30.125, 65.375], [45.625, 66.25], [44.875, 80.5], [31.375, 79.75]]]
    c.queries = [[50.0, 30.0], [38.0, 72.0], [2.0, 50.0], [60.0, 51.5]]
    out.append(c)

    c = Case("kırık hat üstünde nokta", "nokta kırık hattın tam üstünde ve kotu hattın kotu: parça orada bölünür")
    c.points = [(0.0, 0.0, 10.0, "a"), (40.0, 0.0, 11.0, "b"), (40.0, 40.0, 12.0, "c"), (0.0, 40.0, 13.0, "d"),
                (20.0, 20.0, 12.0, "orta"), (7.0, 33.0, 12.5, "e"), (33.0, 9.0, 11.25, "f")]
    c.breaklines.append(([(5.0, 20.0, 12.0), (35.0, 20.0, 12.0)], False, "set"))
    c.queries = [[12.5, 20.0], [20.0, 25.0]]
    out.append(c)

    c = Case("en uzun kenar", "L biçimli nokta bulutu: dış kenarı 30 m'den uzun dış üçgenler atılır")
    for i in range(30):
        x, y = round(rnd.uniform(0, 100), 3), round(rnd.uniform(0, 30), 3)
        c.points.append((x, y, ground(x, y), f"a{i}"))
    for i in range(20):
        x, y = round(rnd.uniform(0, 30), 3), round(rnd.uniform(30, 100), 3)
        c.points.append((x, y, ground(x, y), f"b{i}"))
    c.edge = {"kind": "maxEdge", "length": 30.0}
    c.queries = [[70.0, 70.0], [15.0, 60.0], [60.0, 15.0]]
    out.append(c)

    return out


def grid_case():
    """A 6 × 6 grid on a plane: its triangulation is not unique (cocircular squares), its heights are."""
    pts = [{"x": float(i * 10), "y": float(j * 10), "z": 200 + 0.5 * i * 10 - 0.25 * j * 10} for i in range(6) for j in range(6)]
    queries = [[13.0, 27.0], [45.0, 5.0], [33.3, 41.1], [50.0, 50.0], [60.0, 10.0]]
    heights = []
    for x, y in queries:
        inside = 0 <= x <= 50 and 0 <= y <= 50
        heights.append({"at": [x, y], "z": (200 + 0.5 * x - 0.25 * y) if inside else None})
    return {
        "name": "düzlemde ızgara",
        "note": "6 × 6 kare ızgara bir düzlemde: köşegenler eşit, üçgenleme tek değil; kotlar düzlemin ve 50 üçgen",
        "input": {"points": pts},
        "expect": {"triangleCount": 50, "heights": heights},
    }


def refusals():
    base = [{"x": 0.0, "y": 0.0, "z": 1.0}, {"x": 10.0, "y": 0.0, "z": 2.0}, {"x": 10.0, "y": 10.0, "z": 3.0}, {"x": 0.0, "y": 10.0, "z": 4.0}]
    return [
        {"name": "aynı yerde iki kot", "note": "tek değerli yüzeyde bir yerin bir kotu olur",
         "input": {"points": base + [{"x": 10.0, "y": 10.0, "z": 3.5, "source": "ikinci"}]}, "expect": {"error": "height_ambiguous"}},
        {"name": "aynı yer, aynı kot", "note": "tolerans içinde iki kayıt tek köşe olur",
         "input": {"points": base + [{"x": 10.0, "y": 10.0, "z": 3.0004}]}, "expect": {"triangleCount": 2, "merged": 1}},
        {"name": "kesişen kırık hatlar", "note": "iki kırık hat içten kesişiyor",
         "input": {"points": base, "breaklines": [{"points": [[1, 1, 1], [9, 9, 3]]}, {"points": [[1, 9, 4], [9, 1, 2]]}]},
         "expect": {"error": "constraints_cross"}},
        {"name": "kırık hatta yanlış kotlu nokta", "note": "nokta hattın üstünde, kotu hattınkinden 1 m farklı",
         "input": {"points": base + [{"x": 5.0, "y": 5.0, "z": 9.0}], "breaklines": [{"points": [[1, 1, 1], [9, 9, 3]]}]},
         "expect": {"error": "height_conflict"}},
        {"name": "dışarıda sınır köşesi", "note": "sınırın köşesi noktaların yüzeyinin dışında",
         "input": {"points": base, "boundary": {"points": [[1, 1], [20, 1], [9, 9]]}}, "expect": {"error": "boundary_outside"}},
        {"name": "az nokta", "note": "iki nokta yüzey olmaz",
         "input": {"points": base[:2]}, "expect": {"error": "too_few_points"}},
        {"name": "sayı olmayan kot", "note": "kot sonlu bir sayı olmalı",
         "input": {"points": base[:3] + [{"x": 0.0, "y": 10.0, "z": None}]}, "expect": {"error": "invalid_input"}},
        {"name": "sıfır boylu parça", "note": "ardışık iki köşe aynı yerde",
         "input": {"points": base, "breaklines": [{"points": [[1, 1, 1], [1, 1, 1], [5, 5, 2]]}]}, "expect": {"error": "invalid_input"}},
    ]


def stress(n, path):
    """`--stress N FILE`: N random cases (points, crossing-free breaklines, sometimes a boundary and a void), for
    `KENTOS_ROAD_STRESS=FILE cargo test -p kentos-road --test all stress -- --ignored` (not kept in the repo)."""
    rnd = random.Random(9000)
    doc = {"cases": []}

    def crosses(p, q, r, s):
        def o(a, b, c):
            v = (Fraction(b[0]) - Fraction(a[0])) * (Fraction(c[1]) - Fraction(a[1])) - (Fraction(b[1]) - Fraction(a[1])) * (Fraction(c[0]) - Fraction(a[0]))
            return (v > 0) - (v < 0)
        return o(p, q, r) * o(p, q, s) <= 0 and o(r, s, p) * o(r, s, q) <= 0

    for k in range(n):
        c = Case(f"zorlama {k}", "")
        m = rnd.randint(10, 120)
        for i in range(m):
            x, y = round(rnd.uniform(0, 100), 4), round(rnd.uniform(0, 100), 4)
            c.points.append((x, y, ground(x, y), None))
        c.points += [(-5.0, -5.0, 99.0, None), (105.0, -5.0, 104.0, None), (105.0, 105.0, 102.0, None), (-5.0, 105.0, 97.0, None)]
        segs = []
        for _ in range(rnd.randint(0, 6)):
            a = (round(rnd.uniform(1, 99), 4), round(rnd.uniform(1, 99), 4))
            b = (round(rnd.uniform(1, 99), 4), round(rnd.uniform(1, 99), 4))
            if any(crosses(a, b, s0, s1) for s0, s1 in segs):
                continue
            segs.append((a, b))
            c.breaklines.append(([(a[0], a[1], ground(*a)), (b[0], b[1], ground(*b))], False, None))
        if k % 3 == 0:
            ring = [(2.5, 2.5), (97.5, 3.5), (96.5, 97.5), (3.5, 96.5)]
            if not any(crosses(ring[i], ring[(i + 1) % 4], s0, s1) for i in range(4) for s0, s1 in segs):
                c.boundary = [list(p) for p in ring]
        c.queries = [[round(rnd.uniform(0, 100), 3), round(rnd.uniform(0, 100), 3)] for _ in range(5)]
        try:
            doc["cases"].append({"name": c.name, "input": c.input(), "expect": solve(c)})
        except AssertionError:
            continue
    Path(path).write_text(json.dumps(doc), encoding="utf-8")
    print(f"{len(doc['cases'])} zorlama durumu: {path}")


def main():
    if "--stress" in sys.argv[1:]:
        i = sys.argv.index("--stress")
        stress(int(sys.argv[i + 1]), sys.argv[i + 2])
        return
    doc = {"format": "kentos.road-surface-cases", "version": 1,
           "note": "ADR 0900: Triangle (Shewchuk) ve kesirlerle yazıldı; KentOS kodu yok.", "cases": []}
    for c in cases():
        doc["cases"].append({"name": c.name, "note": c.note, "input": c.input(), "expect": solve(c)})
    doc["cases"].append(grid_case())
    doc["cases"].extend(refusals())
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"{OUT} güncel değil: betiği --check olmadan çalıştırın")
        print(f"{OUT.name} güncel: {len(doc['cases'])} durum")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT} ({len(doc['cases'])} durum)")


if __name__ == "__main__":
    main()
