#!/usr/bin/env python3
"""Independent reference for the overlay's correctness corpus (docs/adr/0500, plan/gis.md CBS-13, TODOS.md GEO-01).

Each case gives two sets of areas `a` and `b` and what their union, intersection and both differences must measure,
how many areas and holes each result has, and the budget a measure may be off by. Straight-edged cases are measured by
GEOS through GDAL's OGR (`osgeo.ogr`); the arc cases (circles, lenses, a circle in a square) by their closed forms with
mpmath at 50 digits. Neither uses KentOS code. A case with `chain` also unions its sets one after another, each joined
again with the result before it (the shape of GEO-01); the result must measure the same as all at once.

The Rust side is crates/shared/geometry-core/tests/all/overlay_cases.rs: it runs the overlay, compares the measures and
counts, and checks the results are valid, that union is idempotent and that the identities hold.

    python3 scripts/fixtures/overlay_cases.py          # writes fixtures/overlay/v1/cases.json
    python3 scripts/fixtures/overlay_cases.py --check  # compares
"""

import json
import sys
from pathlib import Path

import mpmath as mp
from osgeo import ogr

sys.dont_write_bytecode = True
mp.mp.dps = 50
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/overlay/v1/cases.json"


def pt(x, y):
    return {"x": float(x), "y": float(y)}


def ring(points, bulges=None):
    r = {"pts": [pt(x, y) for x, y in points]}
    if bulges is not None:
        r["bulges"] = [float(b) for b in bulges]
    return r


def area(outer, holes=()):
    """An area: a counter-clockwise outer ring and clockwise holes (points as tuples)."""
    return {"outer": ring(outer), "holes": [ring(list(reversed(h)) if signed(h) > 0 else h) for h in holes]}


def signed(points):
    return sum(points[i][0] * points[(i + 1) % len(points)][1] - points[(i + 1) % len(points)][0] * points[i][1]
               for i in range(len(points))) / 2


def rect(x0, y0, x1, y1):
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]


def circle(cx, cy, r):
    """A whole circle as two half arcs (bulge 1 each, counter-clockwise)."""
    return {"outer": ring([(cx + r, cy), (cx - r, cy)], [1.0, 1.0]), "holes": []}


def shift(areas, dx, dy):
    def move(r):
        out = {"pts": [pt(p["x"] + dx, p["y"] + dy) for p in r["pts"]]}
        if "bulges" in r:
            out["bulges"] = r["bulges"]
        return out
    return [{"outer": move(a["outer"]), "holes": [move(h) for h in a["holes"]]} for a in areas]


# ── GEOS for straight-edged areas ────────────────────────────────────────────

def geos(areas):
    multi = ogr.Geometry(ogr.wkbMultiPolygon)
    for a in areas:
        assert "bulges" not in a["outer"], "GEOS measures straight-edged areas here"
        poly = ogr.Geometry(ogr.wkbPolygon)
        for r in [a["outer"], *a["holes"]]:
            lr = ogr.Geometry(ogr.wkbLinearRing)
            for p in r["pts"] + r["pts"][:1]:
                lr.AddPoint_2D(p["x"], p["y"])
            poly.AddGeometry(lr)
        multi.AddGeometry(poly)
    return multi.UnionCascaded() if multi.GetGeometryCount() > 1 else multi.GetGeometryRef(0).Clone()


def counts(g):
    """Areas and holes of a GEOS result (empty ones none)."""
    if g is None or g.IsEmpty():
        return 0, 0
    polys = []
    name = g.GetGeometryName()
    if name == "POLYGON":
        polys = [g]
    elif name in ("MULTIPOLYGON", "GEOMETRYCOLLECTION"):
        polys = [g.GetGeometryRef(i) for i in range(g.GetGeometryCount())
                 if g.GetGeometryRef(i).GetGeometryName() == "POLYGON" and not g.GetGeometryRef(i).IsEmpty()]
    return len(polys), sum(p.GetGeometryCount() - 1 for p in polys)


def polygonal(g):
    """Only the polygons of a result (a touching pair's intersection also has its points and lines)."""
    if g is None or g.IsEmpty():
        return None
    name = g.GetGeometryName()
    if name in ("POLYGON", "MULTIPOLYGON"):
        return g
    if name == "GEOMETRYCOLLECTION":
        m = ogr.Geometry(ogr.wkbMultiPolygon)
        for i in range(g.GetGeometryCount()):
            p = g.GetGeometryRef(i)
            if p.GetGeometryName() == "POLYGON":
                m.AddGeometry(p)
            elif p.GetGeometryName() == "MULTIPOLYGON":
                for j in range(p.GetGeometryCount()):
                    m.AddGeometry(p.GetGeometryRef(j))
        return m if m.GetGeometryCount() else None
    return None


def result(g):
    g = polygonal(g)
    n, h = counts(g)
    return {"area": 0.0 if g is None else g.GetArea(), "areas": n, "holes": h}


def by_geos(a, b):
    ga, gb = geos(a), geos(b)
    return {
        "union": result(ga.Union(gb)),
        "intersection": result(ga.Intersection(gb)),
        "aMinusB": result(ga.Difference(gb)),
        "bMinusA": result(gb.Difference(ga)),
    }


# ── Closed forms for the arc cases ───────────────────────────────────────────

def lens(r1, r2, d):
    r1, r2, d = mp.mpf(r1), mp.mpf(r2), mp.mpf(d)
    if d >= r1 + r2:
        return mp.mpf(0)
    if d <= abs(r1 - r2):
        return mp.pi * min(r1, r2) ** 2
    t1 = r1 ** 2 * mp.acos((d ** 2 + r1 ** 2 - r2 ** 2) / (2 * d * r1))
    t2 = r2 ** 2 * mp.acos((d ** 2 + r2 ** 2 - r1 ** 2) / (2 * d * r2))
    t3 = mp.sqrt((-d + r1 + r2) * (d + r1 - r2) * (d - r1 + r2) * (d + r1 + r2)) / 2
    return t1 + t2 - t3


def disc(r):
    return mp.pi * mp.mpf(r) ** 2


def m(x, areas, holes):
    return {"area": float(x), "areas": areas, "holes": holes}


# ── The cases ────────────────────────────────────────────────────────────────

def cases():
    out = []

    def straight(name, a, b, note, chain=None, budget=1e-7):
        c = {"name": name, "note": note, "a": a, "b": b, "budget": budget, "expect": by_geos(a, b)}
        if chain is not None:
            c["chain"] = chain
        out.append(c)

    def exact(name, a, b, note, expect, chain=None, budget=1e-9):
        c = {"name": name, "note": note, "a": a, "b": b, "budget": budget, "expect": expect}
        if chain is not None:
            c["chain"] = chain
        out.append(c)

    sq = area(rect(0, 0, 10, 10))
    straight("iki örtüşen kare", [sq], [area(rect(5, 5, 15, 15))], "the plain case")
    straight("ortak kenarlı kareler", [sq], [area(rect(10, 0, 20, 10))], "they share an edge: one area, no overlap")
    straight("kısmen ortak kenar", [sq], [area(rect(10, 3, 20, 7))], "part of an edge shared")
    straight("köşede değen kareler", [sq], [area(rect(10, 10, 20, 20))], "they touch at a corner: two areas")
    straight("içteki kare", [sq], [area(rect(2, 2, 4, 4))], "b wholly inside a")
    straight("içten kenara değen kare", [sq], [area(rect(0, 2, 4, 4))], "b inside a, on a's edge")
    straight("deliği dolduran kare", [area(rect(0, 0, 10, 10), [rect(3, 3, 7, 7)])], [area(rect(3, 3, 7, 7))],
             "b fills a's hole exactly: one area without a hole")
    straight("deliğe giren kare", [area(rect(0, 0, 10, 10), [rect(3, 3, 7, 7)])], [area(rect(4, 4, 6, 6))],
             "b inside a's hole: two areas, the hole kept")
    straight("deliğe taşan kare", [area(rect(0, 0, 10, 10), [rect(3, 3, 7, 7)])], [area(rect(5, 5, 9, 9))],
             "b across a's hole's edge")
    straight("ince şerit", [sq], [area(rect(9.9999, 0, 20, 10))], "a sliver of 0.1 mm overlap")
    straight("ince boşluk", [sq], [area(rect(10.0001, 0, 20, 10))], "a gap of 0.1 mm: two areas")
    straight("neredeyse paralel", [area([(0, 0), (100, 0), (100, 1), (0, 1)])],
             [area([(0, 0.9999), (100, 1.0001), (100, 3), (0, 3)])], "edges almost along each other, crossing")
    # Under the kernel's tolerance (10⁻⁶ m, `arrangement::TOL`, docs/specs/numeric-policy.md) the two edges are one:
    # the 2.5·10⁻⁵ m² sliver GEOS keeps goes, within TOL × the edge's 100 m (docs/adr/0500 §2).
    exact("toleransta paralel", [area([(0, 0), (100, 0), (100, 1), (0, 1)])],
          [area([(0, 0.999999), (100, 1.000001), (100, 3), (0, 3)])],
          "edges within the kernel's tolerance of each other: one edge, the sliver goes",
          {"union": m(300, 1, 0), "intersection": m(0, 0, 0), "aMinusB": m(100, 1, 0), "bMinusA": m(200, 1, 0)},
          budget=1e-4)
    straight("aynı kare", [sq], [sq], "a with itself: union and intersection are a, differences empty")
    straight("çok parçalı", [area(rect(0, 0, 4, 4)), area(rect(6, 0, 10, 4))], [area(rect(2, 1, 8, 3))],
             "a of two parts, b across both")
    straight("tarak", [area([(0, 0), (10, 0), (10, 10), (8, 10), (8, 2), (6, 2), (6, 10), (4, 10), (4, 2), (2, 2),
                             (2, 10), (0, 10)])], [area(rect(0, 5, 10, 7))], "a comb crossed: b fills its teeth's gaps")
    grid = [area(rect(i, j, i + 1, j + 1)) for i in range(5) for j in range(5)]
    straight("5×5 ızgara", grid[:12], grid[12:], "25 squares sharing edges: one area 5 × 5")
    straight("ızgara deliği", [g for k, g in enumerate(grid) if k != 12], [area(rect(1.5, 1.5, 3.5, 3.5))],
             "a grid without its middle, b over the hole")
    a1, b1 = [area(rect(0, 0, 10, 10))], [area(rect(10, 0, 20, 10))]
    straight("zincir: kenara değen kareler", a1, b1 + [area(rect(20, 0, 30, 10))],
             "joined one after another, each touching the result's edge (GEO-01's shape)",
             chain=[a1, b1, [area(rect(20, 0, 30, 10))]])
    far = (500000.0, 4400000.0)
    straight("büyük koordinatta örtüşen kareler", shift([sq], *far), shift([area(rect(5, 5, 15, 15))], *far),
             "the plain case at TM coordinates", budget=1e-6)
    straight("büyük koordinatta ortak kenar", shift([sq], *far), shift([area(rect(10, 0, 20, 10))], *far),
             "shared edge at TM coordinates", budget=1e-6)
    straight("büyük koordinatta ince şerit", shift([sq], *far), shift([area(rect(9.999, 0, 20, 10))], *far),
             "a 1 mm sliver at TM coordinates", budget=1e-6)

    # Arcs: closed forms.
    r = 5.0
    pi_r2 = disc(r)
    exact("örtüşen iki daire", [circle(0, 0, r)], [circle(6, 0, r)], "a lens",
          {"union": m(2 * pi_r2 - lens(r, r, 6), 1, 0), "intersection": m(lens(r, r, 6), 1, 0),
           "aMinusB": m(pi_r2 - lens(r, r, 6), 1, 0), "bMinusA": m(pi_r2 - lens(r, r, 6), 1, 0)})
    exact("dıştan teğet daireler", [circle(0, 0, r)], [circle(10, 0, r)], "they touch at one point: two areas",
          {"union": m(2 * pi_r2, 2, 0), "intersection": m(0, 0, 0), "aMinusB": m(pi_r2, 1, 0),
           "bMinusA": m(pi_r2, 1, 0)})
    exact("içten teğet daire", [circle(0, 0, r)], [circle(2, 0, 3)],
          "b inside a, touching it at one point: a − b is one area whose hole meets its outer ring there (OGC valid)",
          {"union": m(pi_r2, 1, 0), "intersection": m(disc(3), 1, 0), "aMinusB": m(pi_r2 - disc(3), 1, 1),
           "bMinusA": m(0, 0, 0)})
    exact("karenin iç teğet dairesi", [area(rect(-r, -r, r, r))], [circle(0, 0, r)],
          "the square's inscribed circle: it touches all four sides",
          {"union": m(4 * r * r, 1, 0), "intersection": m(pi_r2, 1, 0), "aMinusB": m(4 * r * r - pi_r2, 4, 0),
           "bMinusA": m(0, 0, 0)})
    exact("farklı yarıçaplı daireler", [circle(0, 0, 4)], [circle(5, 1, 3)], "a lens of unequal circles",
          {"union": m(disc(4) + disc(3) - lens(4, 3, mp.sqrt(26)), 1, 0),
           "intersection": m(lens(4, 3, mp.sqrt(26)), 1, 0),
           "aMinusB": m(disc(4) - lens(4, 3, mp.sqrt(26)), 1, 0),
           "bMinusA": m(disc(3) - lens(4, 3, mp.sqrt(26)), 1, 0)})
    tangent = [circle(10 * i, 0, r) for i in range(4)]
    exact("teğet tampon zinciri", tangent[:2], tangent[2:],
          "four circles in a row, each touching the next: GEO-01's touching buffers",
          {"union": m(4 * pi_r2, 4, 0), "intersection": m(0, 0, 0), "aMinusB": m(2 * pi_r2, 2, 0),
           "bMinusA": m(2 * pi_r2, 2, 0)},
          chain=[[tangent[0]], [tangent[1]], [tangent[2]], [tangent[3]]])
    over = [circle(8 * i, 0, r) for i in range(3)]
    l8 = lens(r, r, 8)
    exact("örtüşen daire zinciri", over[:2], over[2:],
          "three circles, each overlapping the next only, joined one after another",
          {"union": m(3 * pi_r2 - 2 * l8, 1, 0), "intersection": m(l8, 1, 0),
           "aMinusB": m(2 * pi_r2 - l8 - l8, 1, 0), "bMinusA": m(pi_r2 - l8, 1, 0)},
          chain=[[over[0]], [over[1]], [over[2]]])
    exact("büyük koordinatta örtüşen daireler", shift([circle(0, 0, r)], *far), shift([circle(6, 0, r)], *far),
          "a lens at TM coordinates",
          {"union": m(2 * pi_r2 - lens(r, r, 6), 1, 0), "intersection": m(lens(r, r, 6), 1, 0),
           "aMinusB": m(pi_r2 - lens(r, r, 6), 1, 0), "bMinusA": m(pi_r2 - lens(r, r, 6), 1, 0)},
          budget=1e-6)
    return out


def main():
    doc = {
        "format": "kentos.overlay-cases",
        "version": 1,
        "note": "Written by scripts/fixtures/overlay_cases.py: straight-edged cases by GEOS (GDAL's OGR), arc cases by "
                "their closed forms (mpmath, 50 digits). `budget` is how far a measure may be off (m²); `chain` sets are "
                "joined one after another and must measure as the union (docs/adr/0500).",
        "cases": cases(),
    }
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            sys.exit(f"{OUT.relative_to(ROOT)}: güncel değil; betiği --check'siz çalıştırın.")
        print(f"{OUT.relative_to(ROOT)}: güncel ({len(doc['cases'])} durum).")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")


if __name__ == "__main__":
    main()
