#!/usr/bin/env python3
"""Independent reference of the snap kinds' coverage (docs/adr/0269).

Writes fixtures/snap/v1/coverage.json from the ADR's matrix alone, with Python's standard library (fractions where
the point is rational, `math` for the circles' angles), no KentOS code. The geometry core
(`Store::snap_ex`, crates/shared/geometry-core/tests/all/snap.rs) must give the same: the kind and the object
exactly, the point within 1e-9 m.

Every case asks for one kind with the cursor near the point the ADR says that kind gives on that object: an
endpoint, a midpoint (a bulged edge's on its arc), a centre (a circle's, an arc's, a bulged edge's), a quadrant
(a circle's four, an arc's and a bulged edge's that the arc passes), a node (a point, a dimension's measured
points, a spline's inner points), an insertion point (a text's, a block insert's, a table's), a crossing (two
lines, a line and a circle), a perpendicular foot and a tangent point from the last point, the nearest point on
an edge, an area's centroid. The cases where the kind gives nothing near (an arc's quadrant outside its sweep, a
text with Nokta alone) expect none. A circle's or an arc's centre also comes through its curve while Merkez is the
only kind asked for (the one-shot Merkez): the cursor on the curve gives the centre; with other kinds too, it does
not (a click on an arc stays on the arc).
"""

import argparse
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "snap" / "v1" / "coverage.json"

KINDS = ["endpoint", "midpoint", "center", "node", "quadrant", "intersection", "perpendicular", "tangent", "nearest",
         "centroid", "extension", "parallel", "grid", "insertion"]
LAYERS = [{"id": "a", "visible": True, "locked": False, "pickInterior": True}]
TOL = 1.0


def P(x, y):
    return {"x": float(x), "y": float(y)}


def base(kind, i):
    return {"kind": kind, "id": i, "layerId": "a", "attrs": {}}


def line(i, a, b):
    return {**base("line", i), "a": P(*a), "b": P(*b)}


def polyline(i, pts, bulges=None):
    e = {**base("polyline", i), "pts": [P(*p) for p in pts]}
    if bulges is not None:
        e["bulges"] = bulges
    return e


def polygon(i, pts):
    return {**base("polygon", i), "pts": [P(*p) for p in pts]}


def circle(i, c, r):
    return {**base("circle", i), "c": P(*c), "r": float(r)}


def arc(i, c, r, a0, a1):
    return {**base("arc", i), "c": P(*c), "r": float(r), "a0": a0, "a1": a1}


def ellipse(i, c, major, ratio, t0, t1):
    return {**base("ellipse", i), "c": P(*c), "major": P(*major), "ratio": ratio, "t0": t0, "t1": t1}


def point(i, p):
    return {**base("point", i), "p": P(*p)}


def text(i, p):
    return {**base("text", i), "p": P(*p), "text": "Bina", "height": 1.0}


def insert(i, p):
    return {**base("insert", i), "block": "vana", "p": P(*p)}


def table(i, p):
    return {**base("table", i), "p": P(*p), "rotation": 0.0, "height": 2.5, "rows": [3.0, 3.0],
            "columns": [10.0, 8.0], "cells": [["Ad", "Alan"], ["101", "450"]]}


def spline(i, pts):
    return {**base("spline", i), "pts": [P(*p) for p in pts], "closed": False}


def dimension(i, a, b, offset):
    return {**base("dimension", i), "a": P(*a), "b": P(*b), "offset": float(offset), "height": 2.5}


# ── The reference's geometry ─────────────────────────────────────────────────


def bulge_arc(a, b, bulge):
    """A bulged edge's circle: centre on the chord's left normal at chord·(1−b²)/(4b), r = chord·(1+b²)/(4|b|);
    the arc from a counter-clockwise by 4·atan(b) (clockwise when negative)."""
    dx, dy = b[0] - a[0], b[1] - a[1]
    chord = math.hypot(dx, dy)
    k = (1 - bulge * bulge) / (4 * bulge)
    c = ((a[0] + b[0]) / 2 - dy * k, (a[1] + b[1]) / 2 + dx * k)
    r = chord * (1 + bulge * bulge) / (4 * abs(bulge))
    return c, r, math.atan2(a[1] - c[1], a[0] - c[0]), 4 * math.atan(bulge)


def on_sweep(theta, a0, sweep):
    """θ on the arc from a0 spanning `sweep` (signed)."""
    if sweep < 0:
        a0, sweep = a0 + sweep, -sweep
    d = (theta - a0) % (2 * math.pi)
    return d <= sweep + 1e-12


def quadrants(c, r, a0, sweep):
    q = [(0.0, (c[0] + r, c[1])), (math.pi / 2, (c[0], c[1] + r)), (math.pi, (c[0] - r, c[1])),
         (1.5 * math.pi, (c[0], c[1] - r))]
    return [p for t, p in q if on_sweep(t, a0, sweep)]


def tangent_points(f, c, r):
    """The points where the lines from f touch the circle."""
    dx, dy = f[0] - c[0], f[1] - c[1]
    d = math.hypot(dx, dy)
    base_a = math.atan2(dy, dx)
    off = math.acos(r / d)
    return [(c[0] + r * math.cos(base_a + s * off), c[1] + r * math.sin(base_a + s * off)) for s in (1, -1)]


def foot_on_segment(f, a, b):
    ax, ay, bx, by = map(F, (*a, *b))
    fx, fy = map(F, f)
    t = ((fx - ax) * (bx - ax) + (fy - ay) * (by - ay)) / ((bx - ax) ** 2 + (by - ay) ** 2)
    return (ax + t * (bx - ax), ay + t * (by - ay))


def ring_centroid(pts):
    a = mx = my = F(0)
    n = len(pts)
    for i in range(n):
        (x0, y0), (x1, y1) = map(lambda p: (F(p[0]), F(p[1])), (pts[i], pts[(i + 1) % n]))
        cr = x0 * y1 - x1 * y0
        a += cr / 2
        mx += (x0 + x1) * cr / 6
        my += (y0 + y1) * cr / 6
    return (mx / a, my / a)


def near(p, by=0.3):
    """A cursor `by` metres off a point, towards north-east: inside the aperture, not on the point."""
    return (float(p[0]) + by, float(p[1]) + by)


# ── The matrix ───────────────────────────────────────────────────────────────


def cases():
    out = []

    def case(name, entities, kind, p, expect, frm=None):
        exp = None if expect is None else {"kind": expect[0], "point": [float(expect[1][0]), float(expect[1][1])],
                                           "id": expect[2]}
        out.append({"name": name, "entities": entities, "layers": LAYERS, "p": [float(p[0]), float(p[1])],
                    "tol": TOL, "kinds": kind if isinstance(kind, list) else [kind], "from": None if frm is None else [float(frm[0]), float(frm[1])],
                    "extras": {}, "expect": exp})

    # Lines.
    L = line(1, (0, 0), (20, 10))
    case("çizgi: uç", [L], "endpoint", near((20, 10), 0), ("endpoint", (20, 10), 1))
    case("çizgi: orta", [L], "midpoint", near((10, 5), 0), ("midpoint", (10, 5), 1))
    f = (4, 14)
    case("çizgi: dik ayak", [L], "perpendicular", near(foot_on_segment(f, (0, 0), (20, 10))),
         ("perpendicular", foot_on_segment(f, (0, 0), (20, 10)), 1), f)
    cur = (8.1, 4.4)
    case("çizgi: en yakın", [L], "nearest", cur, ("nearest", foot_on_segment(cur, (0, 0), (20, 10)), 1))

    # A polyline with a straight and a bulged edge.
    pts = [(0, 0), (10, 0), (14, 8)]
    PL = polyline(2, pts, [0, 0.5])
    c, r, a0, sw = bulge_arc((10, 0), (14, 8), 0.5)
    mid_t = a0 + sw / 2
    arc_mid = (c[0] + r * math.cos(mid_t), c[1] + r * math.sin(mid_t))
    case("çoklu çizgi: köşe", [PL], "endpoint", near((10, 0)), ("endpoint", (10, 0), 2))
    case("çoklu çizgi: düz kenarın ortası", [PL], "midpoint", near((5, 0)), ("midpoint", (5, 0), 2))
    case("çoklu çizgi: yaylı kenarın ortası", [PL], "midpoint", near(arc_mid, 0.2), ("midpoint", arc_mid, 2))
    case("çoklu çizgi: yaylı kenarın merkezi", [PL], "center", near(c), ("center", c, 2))
    qs = quadrants(c, r, a0, sw)
    assert qs, "the bulged edge passes a quadrant"
    q = qs[0]
    case("çoklu çizgi: yaylı kenarın çeyreği", [PL], "quadrant", near(q, 0.2), ("quadrant", q, 2))
    f = (20, 10)
    on = [t for t in tangent_points(f, c, r) if on_sweep(math.atan2(t[1] - c[1], t[0] - c[0]), a0, sw)]
    assert len(on) == 1, on
    t = on[0]
    case("çoklu çizgi: yaylı kenara teğet", [PL], "tangent", near(t, 0.2), ("tangent", t, 2), f)

    # An area.
    ring = [(0, 0), (12, 0), (12, 6), (4, 10), (0, 6)]
    A = polygon(3, ring)
    cg = ring_centroid(ring)
    case("alan: köşe", [A], "endpoint", near((12, 6)), ("endpoint", (12, 6), 3))
    case("alan: kenar ortası", [A], "midpoint", near((8, 8)), ("midpoint", (8, 8), 3))
    case("alan: ağırlık merkezi", [A], "centroid", near(cg), ("centroid", cg, 3))
    f = (20, 3)
    case("alan: dik ayak", [A], "perpendicular", near((12, 3)), ("perpendicular", (12, 3), 3), f)

    # A circle.
    C = circle(4, (50, 50), 5)
    case("daire: merkez", [C], "center", near((50, 50)), ("center", (50, 50), 4))
    case("daire: merkez, kenarın üstünden", [C], "center", (54.4, 52.1), ("center", (50, 50), 4))
    for name, qq in [("doğu", (55, 50)), ("kuzey", (50, 55)), ("batı", (45, 50)), ("güney", (50, 45))]:
        case(f"daire: çeyrek {name}", [C], "quadrant", near(qq, 0.2), ("quadrant", qq, 4))
    f = (65, 52)
    t = max(tangent_points(f, (50, 50), 5), key=lambda t: t[1])
    case("daire: teğet", [C], "tangent", near(t, 0.2), ("tangent", t, 4), f)
    cur = (53.1, 53.9)
    d = math.hypot(cur[0] - 50, cur[1] - 50)
    nn = (50 + 5 * (cur[0] - 50) / d, 50 + 5 * (cur[1] - 50) / d)
    case("daire: en yakın", [C], "nearest", cur, ("nearest", nn, 4))

    # An arc: centre, ends, middle, the quadrant inside its sweep, none outside.
    a0, a1 = math.radians(20), math.radians(130)
    R = arc(5, (80, 20), 6, a0, a1)
    pa = (80 + 6 * math.cos(a0), 20 + 6 * math.sin(a0))
    pm = (80 + 6 * math.cos((a0 + a1) / 2), 20 + 6 * math.sin((a0 + a1) / 2))
    # An arc's centre lies outside its box: it comes through the curve under the cursor (§2).
    on_arc = (80 + 6.2 * math.cos(math.radians(75)), 20 + 6.2 * math.sin(math.radians(75)))
    case("yay: merkez, yayın üstünden", [R], "center", on_arc, ("center", (80, 20), 5))
    case("yay: başka türlerle merkez eğrinin üstünden gelmez", [R], ["center", "endpoint"], on_arc, None)
    case("yay: uç", [R], "endpoint", near(pa, 0.2), ("endpoint", pa, 5))
    case("yay: orta", [R], "midpoint", near(pm, 0.2), ("midpoint", pm, 5))
    case("yay: çeyrek, yayın üstünde", [R], "quadrant", near((80, 26), 0.2), ("quadrant", (80, 26), 5))
    case("yay: çeyrek, yayın dışında yok", [R], "quadrant", near((86, 20), 0.2), None)

    # An ellipse: centre, the axes' ends; an elliptical arc's ends.
    E = ellipse(6, (120, 20), (8, 0), 0.5, 0.0, 2 * math.pi)
    case("elips: merkez", [E], "center", near((120, 20)), ("center", (120, 20), 6))
    case("elips: büyük eksenin ucu", [E], "quadrant", near((128, 20), 0.2), ("quadrant", (128, 20), 6))
    case("elips: küçük eksenin ucu", [E], "quadrant", near((120, 24), 0.2), ("quadrant", (120, 24), 6))
    EA = ellipse(7, (150, 20), (8, 0), 0.5, 0.0, math.pi / 2)
    case("eliptik yay: başlangıç ucu", [EA], "endpoint", near((158, 20), 0.2), ("endpoint", (158, 20), 7))

    # Points, texts, inserts, tables, dimensions, splines.
    case("nokta: düğüm", [point(8, (200, 0))], "node", near((200, 0)), ("node", (200, 0), 8))
    case("yazı: ekleme noktası", [text(9, (210, 0))], "insertion", near((210, 0)), ("insertion", (210, 0), 9))
    case("yazı: Nokta türüyle yok", [text(9, (210, 0))], "node", near((210, 0)), None)
    case("blok: ekleme noktası", [insert(10, (220, 0))], "insertion", near((220, 0)), ("insertion", (220, 0), 10))
    case("tablo: ekleme noktası", [table(11, (230, 0))], "insertion", near((230, 0)), ("insertion", (230, 0), 11))
    case("ölçü: ölçülen nokta", [dimension(12, (260, 0), (270, 0), 3)], "node", near((260, 0)),
         ("node", (260, 0), 12))
    S = spline(13, [(300, 0), (305, 4), (310, 0)])
    case("eğri: uç", [S], "endpoint", near((310, 0)), ("endpoint", (310, 0), 13))
    case("eğri: iç nokta", [S], "node", near((305, 4)), ("node", (305, 4), 13))

    # Crossings.
    X1, X2 = line(14, (400, 0), (410, 10)), line(15, (400, 10), (410, 0))
    case("kesişim: iki çizgi", [X1, X2], "intersection", near((405, 5)), ("intersection", (405, 5), 14))
    XC, XL = circle(16, (450, 0), 5), line(17, (440, 3), (460, 3))
    case("kesişim: çizgi ve daire", [XC, XL], "intersection", near((454, 3), 0.2), ("intersection", (454, 3), 16))
    return out


WEIGHT = {"endpoint": F(1), "node": F(1), "insertion": F(1), "intersection": F(102, 100), "center": F(108, 100),
          "centroid": F(108, 100), "quadrant": F(110, 100), "midpoint": F(115, 100), "tangent": F(120, 100),
          "perpendicular": F(125, 100)}


def ranked(p, found, nearest):
    """The candidates best first (docs/adr/0271): weighted distance, then the order met; the nearest point last;
    a point met again once."""
    def d(q):
        return math.hypot(q[0] - p[0], q[1] - p[1])
    order = sorted(enumerate(found), key=lambda iq: (float(WEIGHT[iq[1][0]]) * d(iq[1][1]), iq[0]))
    out = []
    for _, (kind, q, i) in order + ([(None, nearest)] if nearest else []):
        if all(o["point"] != [float(q[0]), float(q[1])] for o in out):
            out.append({"kind": kind, "point": [float(q[0]), float(q[1])], "id": i})
    return out


def candidates():
    """Crowded corners: what the cursor has near it, best first."""
    out = []
    sq = polygon(1, [(0, 0), (10, 0), (10, 10), (0, 10)])
    ln = line(2, (-2, 1), (12, 1))
    p = (0.3, 0.6)
    out.append({"name": "köşe, kesişim ve en yakın: kesişim önde, sonra köşe, en sonda en yakın",
                "entities": [sq, ln], "layers": LAYERS, "p": list(p), "tol": 1.5,
                "kinds": ["endpoint", "intersection", "nearest"], "from": None, "extras": {},
                "expect": ranked(p, [("endpoint", (0, 0), 1), ("intersection", (0, 1), 1)],
                                 ("nearest", (0, 0.6), 1))})
    p = (4.6, 0.4)
    out.append({"name": "kenar ortası ile kesişimin yakını: ağırlıklar sırayı verir",
                "entities": [sq, line(3, (4, -3), (4, 3))], "layers": LAYERS, "p": list(p), "tol": 2.0,
                "kinds": ["endpoint", "midpoint", "intersection"], "from": None, "extras": {},
                "expect": ranked(p, [("midpoint", (5, 0), 1), ("intersection", (4, 0), 1)], None)})
    p = (10.2, 10.1)
    out.append({"name": "iki nesnenin aynı köşesi: aynı nokta bir kez, çizimdeki ilk nesneninki",
                "entities": [sq, polygon(4, [(10, 10), (20, 10), (20, 20), (10, 20)])], "layers": LAYERS,
                "p": list(p), "tol": 1.0, "kinds": ["endpoint"], "from": None, "extras": {},
                "expect": ranked(p, [("endpoint", (10, 10), 1), ("endpoint", (10, 10), 4)], None)})
    return out


def build():
    return {"format": "kentos.snap", "version": 1,
            "source": "scripts/fixtures/snap_coverage_cases.py (ADR 0269, 0271); no KentOS code",
            "kinds": KINDS, "snap": cases(), "candidates": candidates()}


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--check", action="store_true", help="compare with the file instead of writing it")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"kenet kapsamı tutarlı: {len(build()['snap'])} durum")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(build()['snap'])} durum")
    return 0


if __name__ == "__main__":
    sys.exit(main())
