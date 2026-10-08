#!/usr/bin/env python3
"""Geometri işlemleri (docs/adr/0201): the shared cases, written from the ADR without KentOS code.

Two files come out of it:

- fixtures/geoprocess/v1/cases.json: the core's rules. Tampon's pieces (their areas in closed form: a point's π d², a
  segment's 2 d L + π d², a convex area's A + P d + π d², a turn's sector and its inner kite, an arc's band; places
  inside and outside them at distances worked out here), Kes, the overlays and Birleştir (areas of straight edges as
  exact fractions, lengths, points), the problems Geçerliliği denetle finds and where, what Onar makes of them,
  Sadeleştir's vertices (Douglas–Peucker as ADR 0140 writes it), Koordinat sistemine dönüştür's vertices (PROJ, through
  pyproj, the registry's paths), and the shares Alan oranıyla paylaştır writes (Fraction × the double, half to even).
- fixtures/processing/v1/geometry.json (with geometry.kcad): the eleven tools run on a drawing, in the processing
  cases' format (fixtures/processing/README.md), their objects measured (`addedShapes`).

Closed forms in mpmath at 50 digits; distances of sample places from a shape in mpmath (a point's, a segment's, an
arc's, a ring's boundary; inside by crossing number). A sample place stands at least 1 mm inside or outside.

    python3 scripts/fixtures/geoprocess_cases.py          # write
    python3 scripts/fixtures/geoprocess_cases.py --check  # compare
"""

import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
from crs_transform_cases import REGISTRY, system as crs_system, transformer as crs_transformer  # noqa: E402

mp.mp.dps = 50

ROOT = Path(__file__).resolve().parents[2]
CORE_OUT = ROOT / "fixtures" / "geoprocess" / "v1" / "cases.json"
PROC_DIR = ROOT / "fixtures" / "processing" / "v1"
SOURCE = "scripts/fixtures/geoprocess_cases.py (docs/adr/0201)"

E, N = 487000, 4420000
PI = mp.pi


def P(x, y):
    return (F(E) + F(x), F(N) + F(y))


def num(v):
    """A number as JSON: an integer when it is one, else the nearest double."""
    if isinstance(v, F):
        return int(v) if v.denominator == 1 else float(v)
    if isinstance(v, int):
        return v
    x = float(v)
    return int(x) if x == int(x) and abs(x) < 2**53 else x


def jp(p):
    return {"x": num(p[0]), "y": num(p[1])}


# ── Shapes as the contract writes them ────────────────────────────────

def polygon(pts, holes=(), bulges=None, parts=()):
    out = {"kind": "polygon", "pts": [jp(p) for p in pts]}
    if bulges:
        out["bulges"] = bulges
    if holes:
        out["holes"] = [{"pts": [jp(p) for p in h]} for h in holes]
    if parts:
        out["parts"] = [{"pts": [jp(p) for p in q]} for q in parts]
    return out


def rect(x0, y0, x1, y1):
    return [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]


def line(a, b):
    return {"kind": "line", "a": jp(a), "b": jp(b)}


def polyline(pts, parts=()):
    out = {"kind": "polyline", "pts": [jp(p) for p in pts]}
    if parts:
        out["parts"] = [{"pts": [jp(p) for p in q]} for q in parts]
    return out


def point(p, more=()):
    out = {"kind": "point", "p": jp(p)}
    if more:
        out["parts"] = [{"p": jp(q)} for q in more]
    return out


def circle(c, r):
    return {"kind": "circle", "c": jp(c), "r": num(r)}


def arc(c, r, a0, a1):
    """An arc counter-clockwise from a0 to a1 (radians, as doubles)."""
    return {"kind": "arc", "c": jp(c), "r": num(r), "a0": float(a0), "a1": float(a1)}


# ── Distances (mpmath) ─────────────────────────────────────────────────

def mpp(p):
    return (mp.mpf(p[0].numerator) / p[0].denominator, mp.mpf(p[1].numerator) / p[1].denominator) if isinstance(p[0], F) else (mp.mpf(p[0]), mp.mpf(p[1]))


def d_seg(p, a, b):
    (px, py), (ax, ay), (bx, by) = mpp(p), mpp(a), mpp(b)
    dx, dy = bx - ax, by - ay
    l2 = dx * dx + dy * dy
    t = ((px - ax) * dx + (py - ay) * dy) / l2 if l2 else 0
    t = max(mp.mpf(0), min(mp.mpf(1), t))
    return mp.sqrt((px - ax - t * dx) ** 2 + (py - ay - t * dy) ** 2)


def d_arc(p, c, r, a0, a1):
    (px, py), (cx, cy) = mpp(p), mpp(c)
    ang = mp.atan2(py - cy, px - cx)
    sweep = mp.mpf(a1) - mp.mpf(a0)
    rel = (ang - mp.mpf(a0)) % (2 * PI)
    if rel <= sweep:
        return abs(mp.sqrt((px - cx) ** 2 + (py - cy) ** 2) - r)
    ends = [(cx + r * mp.cos(a), cy + r * mp.sin(a)) for a in (mp.mpf(a0), mp.mpf(a1))]
    return min(mp.sqrt((px - x) ** 2 + (py - y) ** 2) for x, y in ends)


def d_pt(p, q):
    (px, py), (qx, qy) = mpp(p), mpp(q)
    return mp.sqrt((px - qx) ** 2 + (py - qy) ** 2)


def ring_segs(ring):
    return [(ring[i], ring[(i + 1) % len(ring)]) for i in range(len(ring))]


def in_ring(ring, p):
    """Crossing number (the places are never on an edge)."""
    px, py = mpp(p)
    inside = False
    for a, b in ring_segs(ring):
        (ax, ay), (bx, by) = mpp(a), mpp(b)
        if (ay > py) != (by > py):
            x = ax + (py - ay) * (bx - ax) / (by - ay)
            if x > px:
                inside = not inside
    return inside


def d_rings(p, rings):
    return min(d_seg(p, a, b) for r in rings for a, b in ring_segs(r))


def check(cond, what):
    if not cond:
        raise SystemExit(f"başvuru tutarsız: {what}")


# ── Tampon ─────────────────────────────────────────────────────────────

def buffer_case(cid, title, shapes, distances, expect, side="both", rings=1, dissolve=False, unread=(), inward=(), empty=()):
    return {
        "id": cid,
        "title": title,
        "shapes": shapes,
        "distances": distances,
        "side": side,
        "rings": rings,
        "dissolve": dissolve,
        "expect": {"pieces": expect, "unread": list(unread), "inward": list(inward), "empty": list(empty)},
    }


def piece(source, ring, distance, area, inside, outside, parts=1, holes=0, dist=None, lo=0, hi=None):
    """A buffer piece; `dist` the place's distance from the object (mp), checked against the band (lo, hi]."""
    if dist is not None:
        for q in inside:
            d = dist(q)
            check(d > lo + mp.mpf("0.0009") and d < hi - mp.mpf("0.0009"), f"{distance}: {q} içeride değil ({d})")
        for q in outside:
            d = dist(q)
            check(d < lo - mp.mpf("0.0009") or d > hi + mp.mpf("0.0009"), f"{distance}: {q} dışarıda değil ({d})")
    return {
        "source": source,
        "ring": ring,
        "distance": distance,
        "parts": parts,
        "holes": holes,
        "area": num(area),
        "inside": [jp(q) for q in inside],
        "outside": [jp(q) for q in outside],
    }


def signed_left(p, a, b):
    """The place's distance to the left of the line a → b (plus on the left) and its parameter along it."""
    (px, py), (ax, ay), (bx, by) = mpp(p), mpp(a), mpp(b)
    dx, dy = bx - ax, by - ay
    length = mp.sqrt(dx * dx + dy * dy)
    t = ((px - ax) * dx + (py - ay) * dy) / (length * length)
    return (dx * (py - ay) - dy * (px - ax)) / length, t


def in_left_buffer(path, d, p):
    """The left buffer as ADR 0201 §2 builds it: each edge's strip on its left, and where the path turns right
    (away from the left) the disc's wedge between the two edges' left normals."""
    for a, b in zip(path, path[1:]):
        s, t = signed_left(p, a, b)
        if 0 <= t <= 1 and 0 <= s <= d:
            return True
    for a, v, b in zip(path, path[1:], path[2:]):
        (ax, ay), (vx, vy), (bx, by) = mpp(a), mpp(v), mpp(b)
        cross = (vx - ax) * (by - vy) - (vy - ay) * (bx - vx)
        if cross >= 0:
            continue
        if d_pt(p, v) > d:
            continue
        # Between the two left normals: on the left of the first edge's end line and of the second's start line.
        s1, t1 = signed_left(p, a, v)
        s2, t2 = signed_left(p, v, b)
        if t1 >= 1 and t2 <= 0:
            return True
    return False


def build_buffers():
    out = []
    o = P(0, 0)
    # A point: π d².
    out.append(buffer_case("point", "Noktanın tamponu: yarıçapı d olan daire, π d²", [point(o)], ["2"], [
        piece(0, 1, "2", 4 * PI, [P("1.999", 0), P(0, "-1.999"), P("1.4", "1.4")], [P("2.001", 0), P("1.42", "1.42")],
              dist=lambda q: d_pt(q, o), hi=2)]))
    # A segment: 2 d L + π d².
    a, b = P(0, 0), P(10, 0)
    out.append(buffer_case("segment", "Doğru parçasının tamponu: kapsül, 2dL + πd²", [line(a, b)], ["1"], [
        piece(0, 1, "1", 20 + PI, [P(5, "0.999"), P("-0.7", "0.7"), P("10.7", "-0.7")], [P(5, "1.001"), P("-0.71", "0.71"), P("10.75", "0.75")],
              dist=lambda q: d_seg(q, a, b), hi=1)]))
    # A convex area: A + P d + π d².
    r = rect(0, 0, 10, 6)
    dist_area = lambda q, rings=(r,): 0 if in_ring(rings[0], q) and not any(in_ring(h, q) for h in rings[1:]) else d_rings(q, rings)  # noqa: E731
    out.append(buffer_case("area", "Dışbükey alanın tamponu: A + Pd + πd²", [polygon(r)], ["1"], [
        piece(0, 1, "1", 60 + 32 + PI, [P(5, "-0.999"), P("10.7", "6.7"), P(5, 3)], [P(5, "-1.001"), P("10.75", "6.75"), P("-1.001", 3)],
              dist=dist_area, lo=-1, hi=1)]))
    # Inward: the rectangle shrinks to (w − 2d)(h − 2d), its corners square.
    depth = lambda q: d_rings(q, [r]) if in_ring(r, q) else -d_rings(q, [r])  # noqa: E731
    out.append(buffer_case("inward", "Eksi uzaklık alanı içe küçültür: (w − 2d)(h − 2d), köşeler keskin", [polygon(r)], ["-1"], [
        piece(0, 1, "-1", 32, [P(5, 3), P("1.001", "1.001"), P("8.999", "4.999")], [P("0.999", 3), P(5, "5.001"), P(5, "0.5")])]))
    for q in [P(5, 3), P("1.001", "1.001"), P("8.999", "4.999")]:
        check(depth(q) > 1, f"içe tampon {q}")
    for q in [P("0.999", 3), P(5, "5.001"), P(5, "0.5")]:
        check(depth(q) < 1, f"içe tampon dışı {q}")
    # A square with a hole: its outside grows, its hole shrinks with square corners.
    outer, hole = rect(0, 0, 20, 20), rect(5, 5, 15, 15)
    holed = lambda q: 0 if in_ring(outer, q) and not in_ring(hole, q) else d_rings(q, [outer, hole])  # noqa: E731
    out.append(buffer_case("holed", "Delikli alanın tamponu: dışı büyür, deliği keskin köşelerle küçülür: 480 + π − 64", [polygon(outer, [hole])], ["1"], [
        piece(0, 1, "1", 416 + PI, [P("20.999", 10), P("5.999", 10), P("-0.5", "-0.5")], [P(10, 10), P("6.001", "6.001"), P("21.001", 10), P("-0.71", "-0.71")],
              holes=1, dist=holed, lo=-1, hi=1)]))
    # An L: two legs, a right-angle turn: 2 d (L1 + L2) + π d² + the outer sector (θ d² / 2) − the inner kite (d² tan θ/2).
    l = [P(0, 0), P(20, 0), P(20, 15)]
    d_l = lambda q: min(d_seg(q, l[0], l[1]), d_seg(q, l[1], l[2]))  # noqa: E731
    out.append(buffer_case("turn", "Dönen yolun tamponu: 2d(L1 + L2) + πd² + θd²/2 − d² tan(θ/2)", [polyline(l)], ["2"], [
        piece(0, 1, "2", 2 * 2 * 35 + 4 * PI + PI - 4, [P(10, "1.999"), P("21.999", 7), P("21.4", "-1.4")], [P(10, "2.001"), P("22.001", 7), P("21.42", "-1.42"), P("17.9", "2.1")],
              dist=d_l, hi=2)]))
    # One side of a segment: d L, its ends flat.
    left = [P(0, 0), P(10, 0)]
    out.append(buffer_case("left", "Tek yanlı tampon, Sol: dL, uçları düz", [line(*left)], ["1"], [
        piece(0, 1, "1", 10, [P(5, "0.5"), P("0.001", "0.999")], [P(5, "-0.5"), P("10.5", "0.5"), P("-0.5", "0.5"), P(5, "1.001")])], side="left"))
    # A right turn seen from the left: the outside of the turn, a quarter disc more.
    rt = [P(0, 0), P(10, 0), P(10, -8)]
    out.append(buffer_case("left-outside-turn", "Sol, sağa dönen yol: dönüşün dışında çeyrek daire eklenir, d(L1 + L2) + πd²/4", [polyline(rt)], ["1"], [
        piece(0, 1, "1", 18 + PI / 4, [P("10.5", "0.5"), P("10.5", -4), P(5, "0.5")], [P("10.75", "0.75"), P("9.5", -4), P("10.5", "-8.5")])], side="left"))
    # A left turn seen from the left: the two strips overlap in a d × d square.
    lt = [P(0, 0), P(10, 0), P(10, 8)]
    out.append(buffer_case("left-inside-turn", "Sol, sola dönen yol: şeritler dönüşün içinde örtüşür, d(L1 + L2) − d²", [polyline(lt)], ["1"], [
        piece(0, 1, "1", 17, [P("9.5", 4), P(5, "0.5")], [P("10.5", 4), P("10.5", "0.5"), P(5, "1.5")])], side="left"))
    # The right side of the same left turn is its outside.
    out.append(buffer_case("right-outside-turn", "Sağ, sola dönen yol: dönüşün dışı sağda, d(L1 + L2) + πd²/4", [polyline(lt)], ["1"], [
        piece(0, 1, "1", 18 + PI / 4, [P("10.5", "-0.5"), P("10.5", 4), P(5, "-0.5")], [P("10.75", "-0.75"), P("9.5", 4), P(5, "0.5")])], side="right"))
    # An arc: its band 2 φ r d and a half disc at each end.
    c = P(0, 0)
    arc_d = lambda q: d_arc(q, c, 5, 0, PI / 2)  # noqa: E731
    out.append(buffer_case("arc", "Yayın tamponu: halka dilimi 2φrd ve uçlarında yarım daireler, πd²", [arc(c, 5, 0, math.pi / 2)], ["1"], [
        piece(0, 1, "1", 2 * (PI / 2) * 5 + PI, [P(0, "5.999"), P("5.999", 0), P("-0.999", 5), P("4.2", "4.2")], [P("-1.001", 5), P("4.3", "4.3"), P(5, "-1.001"), P(1, 1)],
              dist=arc_d, hi=1)]))
    # A circle is an area: π (r + d)², inward π (r − d)².
    out.append(buffer_case("circle", "Dairenin tamponu: π(r + d)²", [circle(c, 5)], ["1"], [
        piece(0, 1, "1", 36 * PI, [P("5.999", 0), P(0, 0)], [P("6.001", 0)])]))
    out.append(buffer_case("circle-inward", "Daire içe: π(r − d)²", [circle(c, 5)], ["-1"], [
        piece(0, 1, "-1", 16 * PI, [P("3.999", 0)], [P("4.001", 0)])]))
    # Two points of one object, 4 m apart, d = 3: two discs less their lens.
    lens = 18 * mp.acos(mp.mpf(2) / 3) - 4 * mp.sqrt(5)
    two = lambda q: min(d_pt(q, P(0, 0)), d_pt(q, P(4, 0)))  # noqa: E731
    out.append(buffer_case("multipoint", "Çok noktalı nesne: iki daire birleşir, 18π − mercek", [point(P(0, 0), [P(4, 0)])], ["3"], [
        piece(0, 1, "3", 18 * PI - lens, [P(2, "2.2"), P(-2, 0)], [P(2, "2.3"), P("7.001", 0)], dist=two, hi=3)]))
    # Rings: k d about a point, the band between (k − 1) d and k d: π d² (2k − 1).
    ring_pieces = []
    for k in (1, 2, 3):
        inside = {1: [P(1, 0)], 2: [P(3, 0), P(0, "-3.999")], 3: [P("5.5", 0)]}[k]
        outside = {1: [P("2.001", 0)], 2: [P("1.999", 0), P("4.001", 0)], 3: [P("3.999", 0), P("6.001", 0)]}[k]
        ring_pieces.append(piece(0, k, str(2 * k), 4 * PI * (2 * k - 1), inside, outside, holes=0 if k == 1 else 1,
                                 dist=lambda q: d_pt(q, P(0, 0)), lo=2 * (k - 1) if k > 1 else -1, hi=2 * k))
    out.append(buffer_case("rings", "Halka sayısı 3: k. halka (k − 1)d ile kd arası, “Uzaklık” kd", [point(P(0, 0))], ["2"], ring_pieces, rings=3))
    # A decimal comma and rings: 2,5 → 2.5 and 5.0.
    out.append(buffer_case("rings-decimal", "Virgüllü uzaklık 2,5 ve iki halka: uzaklıklar 2.5 ve 5.0", [point(P(0, 0))], ["2,5"], [
        piece(0, 1, "2.5", PI * mp.mpf("6.25"), [P("2.499", 0)], [P("2.501", 0)]),
        piece(0, 2, "5.0", PI * (25 - mp.mpf("6.25")), [P("4.999", 0)], [P("2.499", 0), P("5.001", 0)], holes=1)], rings=2))
    # Birleştir: two objects' buffers as one; the same distance is written.
    out.append(buffer_case("dissolve", "Birleştir: iki noktanın tamponları tek alan, uzaklık aynıysa yazılır", [point(P(0, 0)), point(P(4, 0))], ["3", "3"], [
        piece(None, 1, "3", 18 * PI - lens, [P(2, "2.2")], [P(2, "2.3")], dist=two, hi=3)], dissolve=True))
    out.append(buffer_case("dissolve-mixed", "Birleştir, farklı uzaklıklar: iki parça, uzaklık yazılmaz", [point(P(0, 0)), point(P(10, 0))], ["1", "2"], [
        piece(None, 1, None, PI + 4 * PI, [P("0.999", 0), P("11.999", 0)], [P("1.001", 0), P("12.001", 0)], parts=2)], dissolve=True))
    # What is not written: a value that is no number, a path at a minus distance, an area shrunk past its middle; a
    # minus distance with more than one ring.
    out.append(buffer_case("left-out", "Okunamayan uzaklık, eksi uzaklıkta çizgi, ortasını aşan içe tampon yazılmaz",
                           [point(P(0, 0)), line(P(0, 0), P(10, 0)), polygon(rect(0, 0, 10, 10)), point(P(20, 0))], ["on", "-1", "-6", "1"], [
                               piece(3, 1, "1", PI, [P(20, "0.999")], [P(20, "1.001")])], unread=[0], empty=[1, 2]))
    out.append(buffer_case("inward-rings", "Eksi uzaklık ve birden çok halka: alınmaz", [polygon(rect(0, 0, 10, 10))], ["-1"], [], rings=2, inward=[0]))
    return out


# ── Kes ────────────────────────────────────────────────────────────────

def measure(kind, parts, holes=0, area=None, length=None, points=None):
    out = {"kind": kind, "parts": parts, "holes": holes}
    if area is not None:
        out["area"] = num(area)
    if length is not None:
        out["length"] = num(length)
    if points is not None:
        out["points"] = [jp(p) for p in points]
    return out


def ring_area(ring):
    s = F(0)
    o = ring[0]
    for (ax, ay), (bx, by) in ring_segs(ring):
        s += (ax - o[0]) * (by - o[1]) - (bx - o[0]) * (ay - o[1])
    return abs(s) / 2


def build_clips():
    sq = rect(0, 0, 10, 10)
    return [
        {"id": "area", "title": "Alan kesen alanla kesişir", "shapes": [polygon(sq)], "cut": [polygon(rect(5, -5, 15, 5))],
         "expect": [measure("polygon", 1, area=25)]},
        {"id": "path", "title": "Yol kesen alanların içinde kalan parçalarına bölünür", "shapes": [polyline([P(-5, 5), P(35, 5)])],
         "cut": [polygon(sq), polygon(rect(20, 0, 30, 10))], "expect": [measure("polyline", 2, length=20)]},
        {"id": "path-on-boundary", "title": "Sınırdaki parça içeride sayılır", "shapes": [line(P(-5, 0), P(15, 0))], "cut": [polygon(sq)],
         "expect": [measure("polyline", 1, length=10)]},
        {"id": "path-hole", "title": "Delikli kesen: delikteki parça düşer", "shapes": [line(P(-5, 10), P(25, 10))],
         "cut": [polygon(rect(0, 0, 20, 20), [rect(5, 5, 15, 15)])], "expect": [measure("polyline", 2, length=10)]},
        {"id": "arc", "title": "Yay, kesen alanın içindeki dörtte biri", "shapes": [arc(P(0, 0), 5, 0, math.pi)], "cut": [polygon(sq)],
         "expect": [measure("polyline", 1, length=5 * PI / 2)]},
        {"id": "points", "title": "İçinde ya da sınırında kalan noktalar", "shapes": [point(P(5, 5), [P(15, 5), P(10, 5)])], "cut": [polygon(sq)],
         "expect": [measure("point", 2, points=[P(5, 5), P(10, 5)])]},
        {"id": "outside", "title": "Kesen alanın dışındaki nesneden bir şey kalmaz", "shapes": [polygon(rect(50, 50, 60, 60))], "cut": [polygon(sq)],
         "expect": [None]},
        {"id": "joined", "title": "Kesen alanlar önce birleşir: örtüşen iki alan tek kesen", "shapes": [polygon(rect(0, 0, 30, 10))],
         "cut": [polygon(rect(0, 0, 15, 10)), polygon(rect(10, 0, 20, 10))], "expect": [measure("polygon", 1, area=200)]},
    ]


# ── Kesişim, Fark, Simetrik fark, Birleşim ────────────────────────────

def build_overlays():
    a = [polygon(rect(0, 0, 10, 10)), polygon(rect(20, 0, 30, 10))]
    b = [polygon(rect(5, -5, 25, 5))]
    # Kesişim's pieces: [5,10]×[0,5] and [20,25]×[0,5]; the rest of each A is 75; B less A is [5,25]×[−5,0] and [10,20]×[0,5].
    check(ring_area(rect(5, -5, 25, 5)) - 25 - 25 == 150, "B − A")
    meet = [{"a": 0, "b": 0, "share": 0.25, **measure("polygon", 1, area=25)}, {"a": 1, "b": 0, "share": 0.25, **measure("polygon", 1, area=25)}]
    rest = [{"a": 0, "b": None, "share": 0.75, **measure("polygon", 1, area=75)}, {"a": 1, "b": None, "share": 0.75, **measure("polygon", 1, area=75)}]
    other = [{"a": None, "b": 0, "share": None, **measure("polygon", 1, area=150)}]
    return [
        {"id": "intersection", "title": "Kesişim: her A nesnesi değdiği her B alanıyla bir parça, payı A'nın alanına oranı", "a": a, "b": b,
         "mode": "intersection", "expect": meet},
        {"id": "difference", "title": "Fark: her A nesnesinden B'nin birleşimi çıkar", "a": a, "b": b, "mode": "difference", "expect": rest},
        {"id": "sym-difference", "title": "Simetrik fark: A − B'nin parçaları, sonra B − A'nınkiler", "a": a, "b": b, "mode": "symDifference",
         "expect": rest + other},
        {"id": "union", "title": "Birleşim: kesişimler, sonra iki fark", "a": a, "b": b, "mode": "union", "expect": meet + rest + other},
        {"id": "intersection-path", "title": "Kesişim, yol: payı uzunluğunun oranı", "a": [polyline([P(-5, 2), P(35, 2)])], "b": b,
         "mode": "intersection", "expect": [{"a": 0, "b": 0, "share": 0.5, **measure("polyline", 1, length=20)}]},
        {"id": "intersection-points", "title": "Kesişim, noktalar: payı sayısının oranı", "a": [point(P(6, 2), [P(30, 30)])], "b": b,
         "mode": "intersection", "expect": [{"a": 0, "b": 0, "share": 0.5, **measure("point", 1, points=[P(6, 2)])}]},
        {"id": "intersection-two", "title": "Kesişim, örtüşen iki B alanı: her biriyle ayrı parça", "a": [polygon(rect(0, 0, 10, 10))],
         "b": [polygon(rect(5, 5, 15, 15)), polygon(rect(8, 8, 20, 20))], "mode": "intersection",
         "expect": [{"a": 0, "b": 0, "share": 0.25, **measure("polygon", 1, area=25)}, {"a": 0, "b": 1, "share": 0.04, **measure("polygon", 1, area=4)}]},
        {"id": "difference-covered", "title": "Fark, B'nin örttüğü A'dan parça kalmaz", "a": [polygon(rect(1, 1, 2, 2))], "b": [polygon(rect(0, 0, 10, 10))],
         "mode": "difference", "expect": []},
    ]


# ── Birleştir ──────────────────────────────────────────────────────────

def build_dissolves():
    three = [polygon(rect(0, 0, 10, 10)), polygon(rect(10, 0, 20, 10)), polygon(rect(30, 0, 40, 10))]
    frame = [polygon(rect(0, 0, 30, 10)), polygon(rect(0, 20, 30, 30)), polygon(rect(0, 10, 10, 20)), polygon(rect(20, 10, 30, 20))]
    lines = [line(P(0, 0), P(10, 0)), line(P(10, 0), P(10, 5))]
    return [
        {"id": "one-group", "title": "Tek grup, çok parçalı: ortak kenar kalkar, ayrık alan parça olur", "shapes": three, "groups": [0, 0, 0], "multi": True,
         "expect": [{"group": 0, **measure("polygon", 2, area=300)}]},
        {"id": "single-parts", "title": "Çok parçalı değil: her bağlı parça ayrı nesne, büyükten küçüğe", "shapes": three, "groups": [0, 0, 0], "multi": False,
         "expect": [{"group": 0, **measure("polygon", 1, area=200)}, {"group": 0, **measure("polygon", 1, area=100)}]},
        {"id": "groups", "title": "İki grup, ilk nesnelerinin sırasıyla", "shapes": three, "groups": [0, 1, 0], "multi": True,
         "expect": [{"group": 0, **measure("polygon", 2, area=200)}, {"group": 1, **measure("polygon", 1, area=100)}]},
        {"id": "frame", "title": "Bir boşluğu çevreleyen dört alan: delikli tek alan", "shapes": frame, "groups": [0, 0, 0, 0], "multi": True,
         "expect": [{"group": 0, **measure("polygon", 1, holes=1, area=800)}]},
        {"id": "lines", "title": "Çizgiler grubun çok parçalı çoklu çizgisi olur", "shapes": lines, "groups": [0, 0], "multi": True,
         "expect": [{"group": 0, **measure("polyline", 2, length=15)}]},
        {"id": "lines-single", "title": "Çizgiler, çok parçalı değil: sırasıyla ayrı", "shapes": lines, "groups": [0, 0], "multi": False,
         "expect": [{"group": 0, **measure("polyline", 1, length=10)}, {"group": 0, **measure("polyline", 1, length=5)}]},
    ]


# ── Geçerliliği denetle, Onar ──────────────────────────────────────────

def first_crossing(pts, closed):
    """ADR 0201 §6's first crossing of a ring or a path: the first pair (i < j) of its edges that cross, touch or run
    over each other but for neighbours at their shared vertex, and the place first along edge i. Exact fractions."""
    edges = [(pts[k], pts[(k + 1) % len(pts)]) for k in range(len(pts) if closed else len(pts) - 1)]
    edges = [(a, b) for a, b in edges if a != b]
    n = len(edges)
    for i in range(n):
        for j in range(i + 1, n):
            shared = []
            if j == i + 1:
                shared.append(edges[i][1])
            if closed and i == 0 and j == n - 1:
                shared.append(edges[j][1])
            hits = seg_meet(edges[i], edges[j])
            hits = [(t, p) for t, p in hits if p not in shared]
            if hits:
                return min(hits)[1]
    return None


def seg_meet(e, f):
    """Where segment e meets segment f, as (parameter along e, place): a crossing or touch, or the start along e of
    their overlap when they lie on one line."""
    (a, b), (c, d) = e, f
    rx, ry = b[0] - a[0], b[1] - a[1]
    sx, sy = d[0] - c[0], d[1] - c[1]
    den = rx * sy - ry * sx
    qx, qy = c[0] - a[0], c[1] - a[1]
    if den != 0:
        t = (qx * sy - qy * sx) / den
        u = (qx * ry - qy * rx) / den
        if 0 <= t <= 1 and 0 <= u <= 1:
            return [(t, (a[0] + t * rx, a[1] + t * ry))]
        return []
    if qx * ry - qy * rx != 0:
        return []
    l2 = rx * rx + ry * ry
    tc = (qx * rx + qy * ry) / l2
    td = ((d[0] - a[0]) * rx + (d[1] - a[1]) * ry) / l2
    lo, hi = max(F(0), min(tc, td)), min(F(1), max(tc, td))
    if hi > lo:
        return [(lo, (a[0] + lo * rx, a[1] + lo * ry))]
    if hi == lo:
        return [(lo, (a[0] + lo * rx, a[1] + lo * ry))]
    return []


def problem(kind, at):
    return {"problem": kind, "at": jp(at)}


def build_validity():
    bow = [P(0, 0), P(10, 10), P(10, 0), P(0, 10)]
    check(first_crossing(bow, True) == P(5, 5), "papyon")
    spike = [P(0, 0), P(10, 0), P(10, 10), P(10, 5), P(0, 5)]
    check(first_crossing(spike, True) == P(10, 5), "diken")
    loop = [P(0, 0), P(10, 0), P(10, 10), P(5, -5)]
    at = first_crossing(loop, False)
    check(at == P(F(20, 3), 0), "kendini kesen yol")
    return [
        {"id": "square", "title": "Geçerli alan: sorun yok", "shape": polygon(rect(0, 0, 10, 10)), "expect": []},
        {"id": "circle", "title": "Daire: sorun yok", "shape": circle(P(0, 0), 5), "expect": []},
        {"id": "closed-path", "title": "Başladığı yerde biten yol kendini kesmez", "shape": polyline([P(0, 0), P(10, 0), P(10, 10), P(0, 0)]), "expect": []},
        {"id": "bow-tie", "title": "Papyon: halka kendini (5, 5)'te keser", "shape": polygon(bow), "expect": [problem("ringCrossing", P(5, 5))]},
        {"id": "repeated", "title": "Yinelenen köşe, ilk yerinde", "shape": polygon([P(0, 0), P(10, 0), P(10, 0), P(10, 10), P(0, 10)]),
         "expect": [problem("repeated", P(10, 0))]},
        {"id": "repeated-closing", "title": "Son köşe ilkiyle aynı: kapanışta yinelenen köşe", "shape": polygon([P(0, 0), P(10, 0), P(10, 10), P(0, 0)]),
         "expect": [problem("repeated", P(0, 0))]},
        {"id": "zero-area", "title": "Bir doğru üstündeki köşeler: alanı sıfır olan halka", "shape": polygon([P(0, 0), P(5, 0), P(10, 0)]),
         "expect": [problem("zeroArea", P(0, 0))]},
        {"id": "spike", "title": "Geri dönen kenar: halka kendi üstünden geçer", "shape": polygon(spike), "expect": [problem("ringCrossing", P(10, 5))]},
        {"id": "hole-outside", "title": "Dış halkadan taşan delik, ilk dışarıdaki köşesinde",
         "shape": polygon(rect(0, 0, 10, 10), [[P(8, 2), P(12, 2), P(12, 6), P(8, 6)]]), "expect": [problem("holeOutside", P(12, 2))]},
        {"id": "holes-overlap", "title": "Örtüşen delikler, birincinin kenarının ikinciyi kestiği yerde",
         "shape": polygon(rect(0, 0, 20, 20), [rect(2, 2, 10, 10), rect(6, 6, 14, 14)]), "expect": [problem("holesOverlap", P(10, 6))]},
        {"id": "hole-inside-hole", "title": "Deliğin içindeki delik: köşesi öbürünün içinde",
         "shape": polygon(rect(0, 0, 20, 20), [rect(2, 2, 18, 18), rect(5, 5, 8, 8)]), "expect": [problem("holesOverlap", P(5, 5))]},
        {"id": "path-crossing", "title": "Kendini kesen yol, ilk kenarın üstünde", "shape": polyline(loop), "expect": [problem("pathCrossing", at)]},
        {"id": "path-repeated", "title": "Yolda yinelenen köşe", "shape": polyline([P(0, 0), P(5, 0), P(5, 0), P(10, 0)]), "expect": [problem("repeated", P(5, 0))]},
        {"id": "parts", "title": "Çok parçalı alan: her parça kendi sorunlarıyla, sırasıyla", "shape": polygon(rect(0, 0, 10, 10), parts=[bow_at(20)]),
         "expect": [problem("ringCrossing", P(25, 5))]},
    ]


def bow_at(x):
    return [P(x, 0), P(x + 10, 10), P(x + 10, 0), P(x, 10)]


def build_repairs():
    def rep(cid, title, shape, parts, holes, vertices, area, problems, after):
        return {"id": cid, "title": title, "shape": shape,
                "expect": {"parts": parts, "holes": holes, "vertices": vertices, "area": None if area is None else [num(area[0]), num(area[1])],
                           "problems": problems, "after": after}}
    return [
        rep("valid", "Geçerli alan olduğu gibi kalır", polygon(rect(0, 0, 10, 10)), [1, 1], [0, 0], [4, 4], (100, 100), [],
            measure("polygon", 1, area=100)),
        rep("bow-tie", "Papyon iki üçgene ayrılır: yazıldığı gibi alanı 0, sonra 50", polygon([P(0, 0), P(10, 10), P(10, 0), P(0, 10)]), [1, 2], [0, 0], [4, 6],
            (0, 50), ["ringCrossing"], measure("polygon", 2, area=50)),
        rep("repeated", "Yinelenen köşe düşer", polygon([P(0, 0), P(10, 0), P(10, 0), P(10, 10), P(0, 10)]), [1, 1], [0, 0], [5, 4], (100, 100),
            ["repeated"], measure("polygon", 1, area=100)),
        rep("hole-outside", "Taşan delik dış halkayla kesilir: 84 → 92", polygon(rect(0, 0, 10, 10), [[P(8, 2), P(12, 2), P(12, 6), P(8, 6)]]),
            [1, 1], [1, 0], [8, 8], (84, 92), ["holeOutside"], measure("polygon", 1, area=92)),
        rep("holes-overlap", "Örtüşen delikler tek delik olur: 272 → 288", polygon(rect(0, 0, 20, 20), [rect(2, 2, 10, 10), rect(6, 6, 14, 14)]),
            [1, 1], [2, 1], [12, 12], (272, 288), ["holesOverlap"], measure("polygon", 1, holes=1, area=288)),
        rep("zero-area", "Alanı sıfır olan halkadan bir şey kalmaz", polygon([P(0, 0), P(5, 0), P(10, 0)]), [1, 0], [0, 0], [3, 0], (0, 0),
            ["zeroArea"], None),
        rep("path", "Yolun yinelenen köşesi düşer", polyline([P(0, 0), P(5, 0), P(5, 0), P(10, 0)]), [1, 1], [0, 0], [4, 3], None, ["repeated"],
            measure("polyline", 1, length=10)),
        rep("path-crossing", "Kendini kesen yol onarılmaz, olduğu gibi", polyline([P(0, 0), P(10, 0), P(10, 10), P(5, -5)]), [1, 1], [0, 0], [4, 4], None,
            ["pathCrossing"], measure("polyline", 1, length=10 + 10 + mp.sqrt(250))),
    ]


# ── Sadeleştir ─────────────────────────────────────────────────────────

def dp(pts, tol, closed):
    """ADR 0140's Sadeleştir on straight edges: Douglas–Peucker between anchors (an open path's ends; a closed ring's
    first vertex and the one farthest from it), a vertex further than the tolerance from the chord kept; the largest
    distance of a removed vertex. Exact fractions, square roots in mpmath."""
    n = len(pts)
    keep = [False] * n
    if not closed:
        keep[0] = keep[-1] = True
    else:
        keep[0] = True
        far = max(range(1, n), key=lambda i: (pts[i][0] - pts[0][0]) ** 2 + (pts[i][1] - pts[0][1]) ** 2)
        keep[far] = True
    worst = [mp.mpf(0)]

    def thin(a, b):
        span = (b - a) % n
        if span < 2:
            return
        best, best_d = None, mp.mpf(-1)
        for k in range(1, span):
            i = (a + k) % n
            d = d_seg(pts[i], pts[a], pts[b])
            if d > best_d:
                best, best_d = i, d
        if best_d > mp.mpf(tol.numerator) / tol.denominator:
            keep[best] = True
            thin(a, best)
            thin(best, b)
        else:
            worst[0] = max(worst[0], best_d)

    anchors = [i for i in range(n) if keep[i]]
    runs = len(anchors) if closed else len(anchors) - 1
    for r in range(runs):
        thin(anchors[r], anchors[(r + 1) % len(anchors)])
    return [pts[i] for i in range(n) if keep[i]], worst[0]


def build_simplify():
    dent = [P(0, 0), P(5, F(2, 100)), P(10, 0), P(10, 10), P(0, 10)]
    kept, dev = dp(dent, F(5, 100), True)
    check(len(kept) == 4, "çentik")
    zig = [P(0, 0), P(5, F(1, 100)), P(10, 0), P(15, 5)]
    kept_z, dev_z = dp(zig, F(5, 100), False)
    check(len(kept_z) == 3, "zikzak")
    return [
        {"id": "dent", "title": "Alanın 2 cm'lik çentiği 5 cm toleransla düşer: 99.9 → 100", "shape": polygon(dent), "tolerance": 0.05,
         "expect": {"changed": True, "vertices": [5, 4], "area": [num(ring_area(dent)), num(ring_area(kept))], "deviation": num(dev),
                    "after": [jp(p) for p in kept]}},
        {"id": "path", "title": "Yolun 1 cm'lik kırığı düşer", "shape": polyline(zig), "tolerance": 0.05,
         "expect": {"changed": True, "vertices": [4, 3], "area": None, "deviation": num(dev_z), "after": [jp(p) for p in kept_z]}},
        {"id": "tight", "title": "Toleranstan büyük sapma kalır: değişmez", "shape": polygon(dent), "tolerance": 0.01,
         "expect": {"changed": False, "vertices": [5, 5], "area": [num(ring_area(dent)), num(ring_area(dent))], "deviation": 0, "after": None}},
    ]


# ── Koordinat sistemine dönüştür ───────────────────────────────────────

SAGITTA = 0.001


def registry():
    return {s["srid"]: s for s in json.loads(REGISTRY.read_text())["systems"]}


def chords(c, r, a0, sweep):
    """An arc's chord ends after its start: n equal steps, n the least whole number with each chord within 1 mm."""
    step = 2 * math.acos(1 - SAGITTA / r)
    n = max(1, math.ceil(abs(sweep) / step))
    return [(c[0] + r * math.cos(a0 + sweep * k / n), c[1] + r * math.sin(a0 + sweep * k / n)) for k in range(1, n + 1)], n


def build_reprojections():
    reg = registry()
    out = []

    def case(cid, title, shape, src, dst, pts, chorded):
        t, _, _ = crs_transformer(reg[src], reg[dst])
        moved = [t.transform(float(x), float(y)) for x, y in pts]
        out.append({"id": cid, "title": title, "shape": shape, "from": crs_system(reg[src]), "to": crs_system(reg[dst]),
                    "expect": {"points": [{"x": x, "y": y} for x, y in moved], "chorded": chorded}})

    # TUREF / TM30 → TUREF / TM33 (one datum: the projections alone).
    sq = [(F(470000), F(4420000)), (F(470100), F(4420000)), (F(470100), F(4420100)), (F(470000), F(4420100))]
    case("tm-zone", "TM30'dan TM33'e: köşeler tek tek", {"kind": "polygon", "pts": [jp(p) for p in sq]}, 5254, 5255, sq, 0)
    # WGS 84 latitude and longitude → TUREF / TM30 (EPSG's TUREF to WGS 84 (1)).
    lonlat = [(F("29.5"), F("39.75"))]
    case("geographic", "WGS 84 enlem ve boylamından TUREF / TM30'a", {"kind": "point", "p": jp(lonlat[0])}, 4326, 5254, lonlat, 0)
    # A circle in ED50 / TM30 → TUREF / TM30: its two half circles as chords within 1 mm.
    c, r = (480000.0, 4400000.0), 20.0
    half1, n1 = chords(c, r, 0.0, math.pi)
    half2, n2 = chords(c, r, math.pi, math.pi)
    ring = [(c[0] + r, c[1])] + half1 + half2[:-1]
    case("circle-datum", "ED50 / TM30'daki daire TUREF / TM30'a: iki yarım daire 1 mm içinde doğru parçaları",
         {"kind": "circle", "c": {"x": c[0], "y": c[1]}, "r": r}, 2320, 5254, ring, 2)
    check(len(ring) == n1 + n2, "dairenin köşeleri")
    return out


def build_reproject_refusals():
    reg = registry()
    t, _, _ = crs_transformer(reg[4326], reg[5254])
    x, y = t.transform(29.0, 95.0)
    check(not (math.isfinite(x) and math.isfinite(y)), "PROJ 95° enlemi dönüştürmemeli")
    return [
        {"id": "outside", "title": "Enlemi 90°'yi aşan köşe dönüştürülemez (PROJ da dönüştürmez)", "shape": {"kind": "point", "p": {"x": 29.0, "y": 95.0}},
         "from": crs_system(reg[4326]), "to": crs_system(reg[5254]), "expect": {"error": "outside"}},
    ]


# ── Alan oranıyla paylaştır ────────────────────────────────────────────

APPORTION = [("1000", 0.25), ("10.5", 0.25), ("7", 1 / 3), ("-3.10", 0.5), ("12,5", 0.4), ("abc", 0.5), ("0.125", 0.5), ("1", 1.0),
             ("999999999999999999.99", 0.1), ("2", 0.0)]


def apportion(value, share):
    t = value.strip().replace(",", ".")
    body = t.lstrip("+-")
    if not body or body.count(".") > 1 or not all(ch.isdigit() or ch == "." for ch in body) or body == ".":
        return None
    scale = len(body.split(".")[1]) if "." in body else 0
    exact = F(t) * F(share)
    q = round(exact * 10 ** (scale + 2))
    sign = "-" if q < 0 else ""
    digits = str(abs(q)).rjust(scale + 3, "0")
    s = scale + 2
    return f"{sign}{digits[:-s]}.{digits[-s:]}" if s else f"{sign}{digits}"


def build_core():
    return {
        "format": "kentos.geoprocess-cases",
        "version": 1,
        "source": SOURCE,
        "tolerance": {"area": 1e-6, "length": 1e-6, "point": 1e-6},
        "buffer": build_buffers(),
        "clip": build_clips(),
        "overlay": build_overlays(),
        "dissolve": build_dissolves(),
        "validity": build_validity(),
        "repair": build_repairs(),
        "simplify": build_simplify(),
        "reproject": build_reprojections() + build_reproject_refusals(),
        "apportion": [{"value": v, "share": s, "expect": apportion(v, s)} for v, s in APPORTION],
    }


# ── The processing cases' drawing (fixtures/processing/v1/geometry.kcad) ─────────────────────

def style(color, weight=0.25, **more):
    return {"color": color, "lineType": "continuous", "lineWeight": weight, **more}


def layer(i, name, st, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True, "style": st, "children": []}


LAYERS = [
    layer("cizim", "Çizim", style("fg")),
    layer("parsel", "Parsel", style("#E5484D", 0.35, fill="#E5484D1F")),
    layer("imar", "İmar adaları", style("#3E63DD", 0.25, fill="#3E63DD1F")),
    layer("yol", "Yol", style("fg", 0.5)),
    layer("agac", "Ağaç", style("#30A46C", point={"symbol": "ring", "size": 6})),
    layer("hatali", "Hatalı", style("#FFC53D")),
    layer("sinir", "Sınır", style("#8C9AAA")),
    layer("eski", "Eski pafta (ED50)", style("#AD7F58")),
]


def ent(i, kind, layer_id, attrs, **geom):
    return {"kind": kind, "id": i, "layerId": layer_id, "attrs": attrs, **geom}


def pts(points):
    return [jp(p) for p in points]


# ED50 / TM30 coordinates of the old sheet's objects (not of the drawing's origin).
OLD_SQUARE = [(F(470000), F(4420000)), (F(470100), F(4420000)), (F(470100), F(4420100)), (F(470000), F(4420100))]
OLD_WELL = ((470200.0, 4420200.0), 10.0)

ENTITIES = [
    ent(1, "polygon", "parsel", {"Ada": "101", "Parsel": "1", "Değer": "1000.00", "Tür": "Konut"}, pts=pts(rect(0, 0, 20, 30))),
    ent(2, "polygon", "parsel", {"Ada": "101", "Parsel": "2", "Değer": "1500", "Tür": "Konut"}, pts=pts(rect(20, 0, 45, 30))),
    ent(3, "polygon", "parsel", {"Ada": "101", "Parsel": "3", "Değer": "bilinmiyor", "Tür": "Ticaret"}, pts=pts(rect(45, 0, 60, 30))),
    ent(4, "polygon", "parsel", {"Ada": "102", "Parsel": "4", "Değer": "800,5", "Tür": "Konut"}, pts=pts(rect(0, 40, 30, 60))),
    ent(5, "polygon", "imar", {"Ada": "Y1", "Kullanım": "Yol"}, pts=pts(rect(10, 10, 50, 20))),
    ent(6, "polygon", "imar", {"Ada": "Y2", "Kullanım": "Park"}, pts=pts(rect(25, 45, 35, 70))),
    ent(7, "line", "yol", {"Ad": "Cumhuriyet Caddesi"}, a=jp(P(-10, -5)), b=jp(P(70, -5))),
    ent(8, "polyline", "yol", {"Ad": "Okul Sokağı"}, pts=pts([P(5, -10), P(5, 35), P(65, 35)])),
    ent(9, "point", "agac", {"Tür": "Çam", "Yarıçap": "1,5"}, p=jp(P(5, 25)), z=1203.5),
    ent(10, "point", "agac", {"Tür": "Meşe", "Yarıçap": "geniş"}, p=jp(P(30, 5))),
    ent(11, "point", "agac", {"Tür": "Çınar", "Yarıçap": "3"}, p=jp(P(100, 100))),
    ent(12, "polygon", "hatali", {"Ad": "Papyon"}, pts=pts(bow_at(70))),
    ent(13, "polygon", "hatali", {"Ad": "Yinelenen köşe"}, pts=pts([P(85, 0), P(95, 0), P(95, 0), P(95, 10), P(85, 10)])),
    ent(14, "polygon", "hatali", {"Ad": "Taşan delik"}, pts=pts(rect(100, 0, 110, 10)), holes=[{"pts": pts([P(108, 2), P(112, 2), P(112, 6), P(108, 6)])}]),
    ent(15, "polyline", "hatali", {"Ad": "Yinelenen çizgi"}, pts=pts([P(70, 20), P(75, 20), P(75, 20), P(80, 20)])),
    ent(16, "polyline", "hatali", {"Ad": "Kendini kesen"}, pts=pts([P(85, 20), P(95, 20), P(95, 30), P(90, 15)])),
    ent(17, "polygon", "hatali", {"Ad": "Geçerli"}, pts=pts(rect(120, 0, 130, 10))),
    ent(18, "polygon", "sinir", {"Ad": "Çentikli"}, pts=pts([P(0, 100), P(5, F("100.02")), P(10, 100), P(10, 110), P(0, 110)])),
    ent(19, "polyline", "sinir", {"Ad": "Kırıklı"}, pts=pts([P(20, 100), P(25, F("100.01")), P(30, 100), P(35, 105)])),
    ent(20, "polygon", "eski", {"Ad": "Eski parsel"}, pts=[jp(p) for p in OLD_SQUARE]),
    ent(21, "circle", "eski", {"Ad": "Eski kuyu"}, c={"x": OLD_WELL[0][0], "y": OLD_WELL[0][1]}, r=OLD_WELL[1]),
]

DOCUMENT = {
    "format": "kentos.document",
    "version": 1,
    "name": "İşlem durumları: geometri işlemleri",
    "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                 "workspace": "gis", "drawingFont": "barlow"},
    "origin": {"x": E, "y": N},
    "layers": LAYERS,
    "activeLayer": "cizim",
    "entities": ENTITIES,
    "styles": {"items": [], "categories": []},
}


def layer_id(name):
    """A new layer's id as the runner makes it: “islem-” and the name folded, lower case, other characters as “-”."""
    fold = str.maketrans("çğıöşüÇĞİÖŞÜâîû", "cgiosuCGIOSUaiu")
    out, dash = "", False
    for ch in name.translate(fold).lower():
        if "a" <= ch <= "z" or "0" <= ch <= "9":
            out += ch
            dash = False
        elif not dash:
            out += "-"
            dash = True
    return "islem-" + out


def new_layer(name, color):
    return {"id": layer_id(name), "name": name, "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25, "fill": color + "26"}}


def shape(kind, layer_name, attrs, parts=1, holes=0, area=None, length=None, points=None, vertices=None):
    out = {"kind": kind, "layerId": layer_id(layer_name), "attrs": attrs, "parts": parts, "holes": holes}
    if area is not None:
        out["area"] = num(area)
    if length is not None:
        out["length"] = num(length)
    if points is not None:
        out["points"] = [jp(p) for p in points]
    if vertices is not None:
        out["vertices"] = vertices
    return out


ATTRS = {e["id"]: e["attrs"] for e in ENTITIES}
LAY = lambda i: {"scope": "layer", "layerId": i}  # noqa: E731


def proc_case(cid, title, tool, values, expect, selection=None):
    out = {"id": cid, "title": title, "document": "geometry.kcad", "run": {"tool": tool}, "values": values, "expect": expect}
    if selection is not None:
        out["selection"] = selection
    return out


def ok(summary, undo, layers, shapes, log=(), outputs=None, selection=None):
    out = {"status": "ok", "summary": summary, "undo": undo, "layers": layers, "addedShapes": shapes}
    if log:
        out["log"] = [{"level": "warn", "text": t} for t in log]
    if outputs is not None:
        out["outputs"] = outputs
    if selection is not None:
        out["selection"] = selection
    return out


def share_text(value, area, whole):
    """Alan oranıyla paylaştır's text: the share as the core computes it (the piece's area over the whole's, doubles)."""
    return apportion(value, float(area) / float(whole))


HEIGHTS = "1 nesnenin kotları sonuca taşınmadı."


def build_processing():
    a = ATTRS
    buf, rings = "#0090FF", "Tampon"
    cases = [
        proc_case("buffer-points", "Tampon, noktalar 2 m: üç daire, π d²; kotlu nokta söylenir", "geometry.buffer",
                  {"input": LAY("agac"), "distance": 2},
                  ok("3 nesnenin tamponu yazıldı (3 alan).", "Tampon", [new_layer("Tampon", buf)],
                     [shape("polygon", "Tampon", {**a[i], "Uzaklık": "2"}, area=4 * PI) for i in (9, 10, 11)], [HEIGHTS], {"count": 3})),
        proc_case("buffer-rings", "Tampon, iki halka, birleştirerek: halka başına tek alan, “Uzaklık” ve “Halka”", "geometry.buffer",
                  {"input": LAY("agac"), "distance": 2, "rings": 2, "dissolve": True},
                  ok("3 nesnenin tamponu birleştirilerek yazıldı (2 alan).", "Tampon", [new_layer("Tampon", buf)],
                     [shape("polygon", "Tampon", {"Uzaklık": "2", "Halka": "1"}, parts=3, area=12 * PI),
                      shape("polygon", "Tampon", {"Uzaklık": "4", "Halka": "2"}, parts=3, holes=3, area=36 * PI)], [HEIGHTS], {"count": 2})),
        proc_case("buffer-inward", "Tampon, eksi uzaklık: parseller 1 m içe küçülür", "geometry.buffer",
                  {"input": LAY("parsel"), "distance": -1},
                  ok("4 nesnenin tamponu yazıldı (4 alan).", "Tampon", [new_layer("Tampon", buf)],
                     [shape("polygon", "Tampon", {**a[i], "Uzaklık": "-1"}, area=ar) for i, ar in ((1, 18 * 28), (2, 23 * 28), (3, 13 * 28), (4, 28 * 18))], [],
                     {"count": 4})),
        proc_case("buffer-field", "Tampon, Uzaklık alanından: virgüllü değer okunur, okunamayan söylenir", "geometry.buffer",
                  {"input": LAY("agac"), "distanceField": "Yarıçap"},
                  ok("2 nesnenin tamponu yazıldı (2 alan).", "Tampon", [new_layer("Tampon", buf)],
                     [shape("polygon", "Tampon", {**a[9], "Uzaklık": "1.5"}, area=PI * mp.mpf("2.25")),
                      shape("polygon", "Tampon", {**a[11], "Uzaklık": "3"}, area=9 * PI)],
                     [HEIGHTS, "1 nesnenin uzaklığı “Yarıçap” alanından sayı olarak okunamadı; alınmadı."], {"count": 2})),
        proc_case("buffer-left", "Tampon, Sol: sağa dönen sokağın solu, dönüşte çeyrek daire, uçları düz", "geometry.buffer",
                  {"input": {"scope": "selection"}, "distance": 1, "side": "left"},
                  ok("1 nesnenin tamponu yazıldı (1 alan).", "Tampon", [new_layer("Tampon", buf)],
                     [shape("polygon", "Tampon", {**a[8], "Uzaklık": "1"}, area=45 + 60 + PI / 4)], [], {"count": 1}), selection=[8]),
        proc_case("refuse-buffer-rings", "Eksi uzaklıkla birden çok halka çizilmez", "geometry.buffer",
                  {"input": LAY("parsel"), "distance": -1, "rings": 2},
                  {"status": "invalid", "issues": [{"message": "Halkalar yalnız artı uzaklıkla çizilir; Halka sayısını 1 yapın ya da uzaklığı artı yazın."}]}),
        proc_case("clip-roads", "Kırp, yollar parsellerle: sokağın parseldeki parçası; caddeden bir şey kalmaz", "geometry.clip",
                  {"input": LAY("yol"), "cut": LAY("parsel")},
                  ok("1 nesne kırpılarak yazıldı.", "Kırp", [new_layer("Kırpılan", "#30A46C")],
                     [shape("polyline", "Kırpılan", a[8], length=30)], ["1 nesnenin sonucu boş; yazılmadı."], {"count": 1})),
        proc_case("clip-trees", "Kırp, ağaçlar parsellerle: içerideki iki ağaç, kotu söylenir", "geometry.clip",
                  {"input": LAY("agac"), "cut": LAY("parsel")},
                  ok("2 nesne kırpılarak yazıldı.", "Kırp", [new_layer("Kırpılan", "#30A46C")],
                     [shape("point", "Kırpılan", a[9], parts=1, points=[P(5, 25)]), shape("point", "Kırpılan", a[10], parts=1, points=[P(30, 5)])],
                     [HEIGHTS, "1 nesnenin sonucu boş; yazılmadı."], {"count": 2})),
        proc_case("dissolve-ada", "Gruplayarak birleştir, Ada'ya göre: komşu parseller tek alan, Değer toplanır", "geometry.dissolve",
                  {"input": LAY("parsel"), "group": "Ada", "sums": "Değer"},
                  ok("4 nesne 2 grupta birleştirildi; 2 nesne yazıldı.", "Gruplayarak birleştir", [new_layer("Birleştirilen", "#6E56CF")],
                     [shape("polygon", "Birleştirilen", {"Ada": "101", "Nesne sayısı": "3", "Değer": "2500.00"}, area=1800),
                      shape("polygon", "Birleştirilen", {"Ada": "102", "Nesne sayısı": "1", "Değer": "800.5"}, area=600)],
                     ["1 değer sayı olarak okunamadığı için toplanmadı."], {"count": 2})),
        proc_case("dissolve-parts", "Gruplayarak birleştir, gruplamasız ve tek parçalı: büyükten küçüğe iki nesne", "geometry.dissolve",
                  {"input": LAY("parsel"), "multi": False},
                  ok("4 nesne birleştirildi; 2 nesne yazıldı.", "Gruplayarak birleştir", [new_layer("Birleştirilen", "#6E56CF")],
                     [shape("polygon", "Birleştirilen", {"Nesne sayısı": "4"}, area=1800), shape("polygon", "Birleştirilen", {"Nesne sayısı": "4"}, area=600)],
                     [], {"count": 2})),
    ]
    # Kesişim: each parcel with the zoning area it meets; Değer apportioned by the piece's share of the parcel.
    meets = [(1, 5, 100, 600), (2, 5, 250, 750), (3, 5, 50, 450), (4, 6, 75, 600)]

    def apportioned(i, area, whole):
        attrs = dict(a[i])
        s = share_text(attrs["Değer"], area, whole)
        if s is not None:
            attrs["Değer"] = s
        return attrs

    def second(attrs, j, prefix):
        out = dict(attrs)
        for k, v in a[j].items():
            name = prefix + k
            if name in out:
                n = 2
                while f"{name} ({n})" in out:
                    n += 1
                name = f"{name} ({n})"
            out[name] = v
        return out

    cases.append(proc_case("intersection", "Kesişim, parseller imar adalarıyla: önek boş, aynı ad “Ada (2)”; Değer alan oranıyla", "geometry.intersection",
                           {"input": LAY("parsel"), "overlay": LAY("imar"), "prefix": "", "apportion": "Değer"},
                           ok("4 kesişim parçası yazıldı.", "Kesişim", [new_layer("Kesişim", "#F76B15")],
                              [shape("polygon", "Kesişim", second(apportioned(i, ar, w), j, ""), area=ar) for i, j, ar, w in meets],
                              ["1 değer sayı olarak okunamadığı için paylaştırılmadı."], {"count": 4})))
    rest = [(1, 500, 600, 1), (2, 500, 750, 2), (3, 400, 450, 1), (4, 525, 600, 1)]
    cases.append(proc_case("difference", "Fark, parsellerden imar adaları: ortasından kesilen parsel iki parça", "geometry.difference",
                           {"input": LAY("parsel"), "overlay": LAY("imar")},
                           ok("4 nesnenin farkı yazıldı.", "Fark", [new_layer("Fark", "#E5484D")],
                              [shape("polygon", "Fark", a[i], parts=parts, area=ar) for i, ar, _, parts in rest], [], {"count": 4})))
    cases.append(proc_case("sym-difference", "Simetrik fark: parsellerin adaların dışı, parkın parsel dışı; yol adası tümüyle örtülü", "geometry.symDifference",
                           {"input": LAY("parsel"), "overlay": LAY("imar"), "prefix": "İmar "},
                           ok("5 parça yazıldı: 4 birinci, 1 ikinci alanlardan.", "Simetrik fark", [new_layer("Simetrik fark", "#E93D82")],
                              [shape("polygon", "Simetrik fark", a[i], parts=parts, area=ar) for i, ar, _, parts in rest]
                              + [shape("polygon", "Simetrik fark", second({}, 6, "İmar "), area=250 - 75)],
                              ["1 nesnenin sonucu boş; yazılmadı."], {"count": 5})))
    cases.append(proc_case("union", "Birleşim: ortaklar iki tarafın, kalanlar kendi tarafının öznitelikleriyle; Değer paylaşılır", "geometry.union",
                           {"input": LAY("parsel"), "overlay": LAY("imar"), "prefix": "İmar ", "apportion": "Değer"},
                           ok("9 parça yazıldı: 4 ortak, 4 yalnız birinci, 1 yalnız ikinci alanlarda.", "Birleşim", [new_layer("Birleşim", "#3E63DD")],
                              [shape("polygon", "Birleşim", second(apportioned(i, ar, w), j, "İmar "), area=ar) for i, j, ar, w in meets]
                              + [shape("polygon", "Birleşim", apportioned(i, ar, w), parts=parts, area=ar) for i, ar, w, parts in rest]
                              + [shape("polygon", "Birleşim", second({}, 6, "İmar "), area=250 - 75)],
                              ["2 değer sayı olarak okunamadığı için paylaştırılmadı."], {"count": 9})))
    # Geçerliliği denetle: the problems of the faulty layer and where they first show (the core cases' rules).
    def at(p):
        return [f"{float(p[0]):.3f}", f"{float(p[1]):.3f}"]
    found = [(12, "Kendini kesen halka", P(75, 5)), (13, "Yinelenen köşe", P(95, 0)), (14, "Delik dış halkanın dışına taşıyor", P(112, 2)),
             (15, "Yinelenen köşe", P(75, 20)), (16, "Kendini kesen yol", first_crossing([P(85, 20), P(95, 20), P(95, 30), P(90, 15)], False))]
    check(found[4][2] == P(F(275, 3), 20), "kendini kesen yolun yeri")
    rows = [[f"#{i}", "Hatalı", t, *at(p)] for i, t, p in found]
    cases.append(proc_case("validity", "Geçerliliği denetle: beş nesnede beş sorun, yerleriyle; sorunlular seçilir, çizim değişmez", "geometry.validity",
                           {"input": LAY("hatali")},
                           {"status": "ok", "summary": "6 nesneden 5 nesnede 5 sorun bulundu; sorunlu nesneler seçildi.", "undo": None, "layers": [], "addedShapes": [],
                            "selection": [12, 13, 14, 15, 16],
                            "outputs": {"count": 5, "invalid": [12, 13, 14, 15, 16], "problems": {"columns": ["Nesne", "Katman", "Sorun", "Doğu", "Kuzey"], "rows": rows}}}))
    report = [
        ["#12", "1 parça, 0 delik, 0.00 m²", "2 parça, 0 delik, 50.00 m²", "Kendini kesen halka"],
        ["#13", "1 parça, 0 delik, 100.00 m²", "1 parça, 0 delik, 100.00 m²", "Yinelenen köşe"],
        ["#14", "1 parça, 1 delik, 84.00 m²", "1 parça, 0 delik, 92.00 m²", "Delik dış halkanın dışına taşıyor"],
        ["#15", "4 köşe", "3 köşe", "Yinelenen köşe"],
        ["#16", "4 köşe", "4 köşe", "Kendini kesen yol (onarılmaz)"],
    ]
    cases.append(proc_case("repair", "Onar: papyon iki üçgen, yinelenen köşe düşer, taşan delik kesilir; kendini kesen yol olduğu gibi", "geometry.repair",
                           {"input": LAY("hatali")},
                           ok("6 nesne yazıldı; 4 nesne onarıldı.", "Onar", [new_layer("Onarılan", "#12A594")],
                              [shape("polygon", "Onarılan", a[12], parts=2, area=50), shape("polygon", "Onarılan", a[13], area=100),
                               shape("polygon", "Onarılan", a[14], area=92), shape("polyline", "Onarılan", a[15], length=10),
                               shape("polyline", "Onarılan", a[16], length=10 + 10 + mp.sqrt(250)), shape("polygon", "Onarılan", a[17], area=100)],
                              ["1 kendini kesen yol onarılmadı; olduğu gibi yazıldı."],
                              {"count": 4, "report": {"columns": ["Nesne", "Önce", "Sonra", "Değişiklik"], "rows": report}})))
    # Sadeleştir: the dent and the kink go (the core cases' rule); the area's change and the deviation.
    dent = [P(0, 100), P(5, F("100.02")), P(10, 100), P(10, 110), P(0, 110)]
    kept, dev = dp(dent, F(5, 100), True)
    zig = [P(20, 100), P(25, F("100.01")), P(30, 100), P(35, 105)]
    kept_z, dev_z = dp(zig, F(5, 100), False)
    before, after = ring_area(dent), ring_area(kept)
    change = float(after) - float(before)
    rows = [["#18", "5", "4", f"{change:.2f}", f"{change / float(before) * 100:.2f}", f"{float(dev):.3f}"], ["#19", "4", "3", "", "", f"{float(dev_z):.3f}"]]
    cases.append(proc_case("simplify", "Sadeleştir 5 cm: alanın çentiği ve çizginin kırığı düşer; rapor", "geometry.simplify",
                           {"input": LAY("sinir"), "tolerance": 0.05},
                           ok(f"2 nesne yazıldı; 2 nesnede 2 köşe atıldı, en büyük sapma {float(max(dev, dev_z)):.3f} m.", "Sadeleştir",
                              [new_layer("Sadeleştirilen", "#AD7F58")],
                              [shape("polygon", "Sadeleştirilen", a[18], area=after, vertices=[jp(p) for p in kept]),
                               shape("polyline", "Sadeleştirilen", a[19], length=sum(d_pt(p, q) for p, q in zip(kept_z, kept_z[1:])),
                                     vertices=[jp(p) for p in kept_z])],
                              [], {"count": 2, "report": {"columns": ["Nesne", "Köşe (önce)", "Köşe (sonra)", "Alan değişimi (m²)", "Alan değişimi (%)",
                                                                       "En büyük sapma (m)"], "rows": rows}})))
    # Koordinat sistemine dönüştür: the old sheet's square and well from ED50 / TM30 to the project's TUREF / TM30 (PROJ).
    reg = registry()
    t, _, _ = crs_transformer(reg[2320], reg[5254])
    moved = lambda ps: [{"x": x, "y": y} for x, y in (t.transform(float(px), float(py)) for px, py in ps)]  # noqa: E731
    (cx, cy), r = OLD_WELL
    h1, _ = chords((cx, cy), r, 0.0, math.pi)
    h2, _ = chords((cx, cy), r, math.pi, math.pi)
    well = [(cx + r, cy)] + h1 + h2[:-1]
    sq_moved, well_moved = moved(OLD_SQUARE), moved(well)
    cases.append(proc_case("reproject", "Koordinat sistemine dönüştür, ED50 / TM30'dan: köşeler PROJ'la, kuyu 1 mm içinde doğru parçalarıyla", "geometry.reproject",
                           {"input": LAY("eski"), "source": "2320"},
                           ok("2 nesne dönüştürüldü: EPSG:2320 → EPSG:5254.", "Koordinat sistemine dönüştür", [new_layer("Dönüştürülen", "#AB4ABA")],
                              [shape("polygon", "Dönüştürülen", a[20], vertices=sq_moved), shape("polygon", "Dönüştürülen", a[21], vertices=well_moved)],
                              ["1 nesnenin yayları 1 mm içinde doğru parçalarına çevrildi."], {"count": 2})))
    cases.append(proc_case("refuse-reproject-same", "Kaynak sistem projeninkiyle aynıysa çalışmaz", "geometry.reproject",
                           {"input": LAY("eski"), "source": "5254"},
                           {"status": "error", "message": "Kaynak sistem projenin sistemiyle aynı; dönüştürülecek bir şey yok."}))
    # The defaults the tools take from the drawing (the ADR's parameters): a features input's first scope a layer, the
    # active one, unless the tool says otherwise; a new output layer named for the tool.
    active = {"scope": "layer", "layerId": "cizim"}
    tools = {
        "geometry.buffer": {"input": active, "distance": 5, "side": "both", "rings": 1, "dissolve": False, "distanceField": "",
                            "layer": {"newName": "Tampon"}},
        "geometry.clip": {"input": active, "cut": {"scope": "selection"}, "layer": {"newName": "Kırpılan"}},
        "geometry.dissolve": {"input": active, "group": "", "sums": "", "multi": True, "layer": {"newName": "Birleştirilen"}},
        "geometry.intersection": {"input": active, "overlay": active, "prefix": "", "apportion": "", "layer": {"newName": "Kesişim"}},
        "geometry.difference": {"input": active, "overlay": active, "layer": {"newName": "Fark"}},
        "geometry.symDifference": {"input": active, "overlay": active, "prefix": "", "layer": {"newName": "Simetrik fark"}},
        "geometry.union": {"input": active, "overlay": active, "prefix": "", "apportion": "", "layer": {"newName": "Birleşim"}},
        "geometry.validity": {"input": active},
        "geometry.repair": {"input": active, "layer": {"newName": "Onarılan"}},
        "geometry.simplify": {"input": active, "tolerance": 0.1, "layer": {"newName": "Sadeleştirilen"}},
        "geometry.reproject": {"input": active, "source": "4326", "layer": {"newName": "Dönüştürülen"}},
    }
    defaults = {"lengthDecimals": 3, "areaDecimals": 2, "angleUnit": "grad", "plotScale": 1000, "drawingFont": "barlow", "activeLayer": "cizim", "measureHeightMm": 2}
    return {
        "format": "kentos.processing-cases",
        "version": 1,
        "tolerance": 1e-6,
        "measureTolerance": 1e-6,
        "documents": {"geometry.kcad": {"defaults": defaults, "tools": tools}},
        "cases": cases,
    }


def main():
    core = json.dumps(build_core(), ensure_ascii=False, indent=1) + "\n"
    proc = json.dumps(build_processing(), ensure_ascii=False, indent=1) + "\n"
    doc = json.dumps(DOCUMENT, ensure_ascii=False, indent=1) + "\n"
    outs = [(CORE_OUT, core), (PROC_DIR / "geometry.json", proc), (PROC_DIR / "geometry.kcad", doc)]
    if "--check" in sys.argv[1:]:
        bad = [p for p, t in outs if not p.exists() or p.read_text() != t]
        for p in bad:
            print(f"{p.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
        if bad:
            return 1
        print("geometri işlemlerinin durumları tutarlı: fixtures/geoprocess/v1/cases.json, fixtures/processing/v1/geometry.*")
        return 0
    for p, t in outs:
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(t)
        print(f"yazıldı: {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
