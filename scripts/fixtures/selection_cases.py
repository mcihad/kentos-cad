#!/usr/bin/env python3
"""Seçim ekleri (docs/adr/0187): the shared cases, written from the ADR's rules
without KentOS code.

- hits.json: every object a click could mean, the most specific first (§1):
  points (within 1.5 × the tolerance), lines and edges by their distance, then
  the areas around the click (a polygon less its holes, a circle) by their size;
  equals in the document's order.
- polygon.json: Çokgenle seç (§2): what lies wholly inside a simple ring (every
  piece of every edge between its meetings with the boundary has its middle
  inside or on it), what touches it (an edge meets the boundary, a point is
  inside, or the ring lies in a polygon's or a circle's area), and every object
  that does not touch it; and the rings that cannot select, in the tools' words.
- similar.json: Benzerini seç (§4): the objects equal to an example in every
  criterion that is on.

Straight edges are decided with exact fractions; circles with floats far from
any edge case. The core runs hits.json and polygon.json natively and through
WASM (crates/shared/geometry-core/tests/all/selection.rs,
apps/web/src/viewport/selection.wasm.test.ts); both platforms run similar.json
(kentos_interaction's select_similar tests, apps/web/src/model/selectSimilar.test.ts).

    python3 scripts/fixtures/selection_cases.py          # write
    python3 scripts/fixtures/selection_cases.py --check  # compare
"""

import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "selection" / "v1"
SOURCE = "scripts/fixtures/selection_cases.py (docs/adr/0187)"


# ── Objects as the geometry store reads them ────────────────────────────────

def pt(x, y):
    return {"x": x, "y": y}


def point(i, x, y):
    return {"id": i, "layerId": "a", "attrs": {}, "kind": "point", "p": pt(x, y)}


def line(i, a, b):
    return {"id": i, "layerId": "a", "attrs": {}, "kind": "line", "a": pt(*a), "b": pt(*b)}


def polyline(i, pts):
    return {"id": i, "layerId": "a", "attrs": {}, "kind": "polyline", "pts": [pt(*p) for p in pts]}


def polygon(i, pts, holes=()):
    o = {"id": i, "layerId": "a", "attrs": {}, "kind": "polygon", "pts": [pt(*p) for p in pts]}
    if holes:
        o["holes"] = [{"pts": [pt(*p) for p in h]} for h in holes]
    return o


def circle(i, c, r):
    return {"id": i, "layerId": "a", "attrs": {}, "kind": "circle", "c": pt(*c), "r": r}


def square(i, x, y, side, holes=()):
    return polygon(i, [(x, y), (x + side, y), (x + side, y + side), (x, y + side)], holes)


# ── Exact plane geometry ────────────────────────────────────────────────────

def q(p):
    return (F(p[0]), F(p[1]))


def cross(o, a, b):
    return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])


def on_segment(p, a, b):
    if cross(a, b, p) != 0:
        return False
    return min(a[0], b[0]) <= p[0] <= max(a[0], b[0]) and min(a[1], b[1]) <= p[1] <= max(a[1], b[1])


def in_ring(p, ring):
    """Even-odd, a point on the boundary either way (the callers ask the boundary first)."""
    inside = False
    n = len(ring)
    for i in range(n):
        a, b = ring[i], ring[(i + 1) % n]
        if (a[1] > p[1]) != (b[1] > p[1]):
            x = a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1])
            if p[0] < x:
                inside = not inside
    return inside


def sides(ring):
    return [(ring[i], ring[(i + 1) % len(ring)]) for i in range(len(ring))]


def on_boundary(p, ring):
    return any(on_segment(p, a, b) for a, b in sides(ring))


def holds(p, ring):
    return on_boundary(p, ring) or in_ring(p, ring)


def seg_params(a, b, c, d):
    """Where segment ab meets segment cd, as parameters along ab (touching counted, overlap by its ends)."""
    r = (b[0] - a[0], b[1] - a[1])
    s = (d[0] - c[0], d[1] - c[1])
    den = r[0] * s[1] - r[1] * s[0]
    if den == 0:
        # Parallel: the other's ends lying on ab.
        out = []
        for e in (c, d):
            if on_segment(e, a, b):
                rr = r[0] * r[0] + r[1] * r[1]
                out.append(((e[0] - a[0]) * r[0] + (e[1] - a[1]) * r[1]) / rr)
        return out
    t = ((c[0] - a[0]) * s[1] - (c[1] - a[1]) * s[0]) / den
    u = ((c[0] - a[0]) * r[1] - (c[1] - a[1]) * r[0]) / den
    return [t] if 0 <= t <= 1 and 0 <= u <= 1 else []


def at(a, b, t):
    return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)


def seg_inside(a, b, ring):
    if not (holds(a, ring) and holds(b, ring)):
        return False
    ts = {F(0), F(1)}
    for c, d in sides(ring):
        ts.update(seg_params(a, b, c, d))
    ts = sorted(ts)
    return all(holds(at(a, b, (t0 + t1) / 2), ring) for t0, t1 in zip(ts, ts[1:]) if t1 > t0)


def seg_meets(a, b, ring):
    return any(seg_params(a, b, c, d) for c, d in sides(ring))


def segments(o):
    k = o["kind"]
    if k == "line":
        return [(q((o["a"]["x"], o["a"]["y"])), q((o["b"]["x"], o["b"]["y"])))]
    if k == "polyline":
        p = [q((v["x"], v["y"])) for v in o["pts"]]
        return list(zip(p, p[1:]))
    if k == "polygon":
        out = []
        for r in [o["pts"]] + [h["pts"] for h in o.get("holes", [])]:
            p = [q((v["x"], v["y"])) for v in r]
            out += sides(p)
        return out
    return []


def polygon_holds(o, p):
    ring = [q((v["x"], v["y"])) for v in o["pts"]]
    holes = [[q((v["x"], v["y"])) for v in h["pts"]] for h in o.get("holes", [])]
    return in_ring(p, ring) and not any(in_ring(p, h) for h in holes)


def shoelace(ring):
    s = 0
    for (x0, y0), (x1, y1) in sides(ring):
        s += x0 * y1 - x1 * y0
    return abs(s) / 2


# ── hits ────────────────────────────────────────────────────────────────────

def seg_dist(p, a, b):
    """Distance from p to segment ab, as a float from exact parts."""
    r = (b[0] - a[0], b[1] - a[1])
    rr = r[0] * r[0] + r[1] * r[1]
    t = ((p[0] - a[0]) * r[0] + (p[1] - a[1]) * r[1]) / rr if rr else F(0)
    t = min(F(1), max(F(0), t))
    c = at(a, b, t)
    return math.sqrt(float((p[0] - c[0]) ** 2 + (p[1] - c[1]) ** 2))


def hits(objects, p, tol):
    p = q(p)
    edges, areas = [], []
    for o in objects:
        k = o["kind"]
        d = None
        area = None
        if k == "point":
            v = q((o["p"]["x"], o["p"]["y"]))
            dist = math.sqrt(float((p[0] - v[0]) ** 2 + (p[1] - v[1]) ** 2))
            d = dist if dist <= 1.5 * tol else None
        elif k == "circle":
            c = q((o["c"]["x"], o["c"]["y"]))
            r = o["r"]
            dc = math.sqrt(float((p[0] - c[0]) ** 2 + (p[1] - c[1]) ** 2))
            d = abs(dc - r) if abs(dc - r) <= tol else None
            area = math.pi * r * r if dc < r else None
        else:
            best = min(seg_dist(p, a, b) for a, b in segments(o))
            d = best if best <= tol else None
            if k == "polygon" and polygon_holds(o, p):
                ring = [q((v["x"], v["y"])) for v in o["pts"]]
                holes = [[q((v["x"], v["y"])) for v in h["pts"]] for h in o.get("holes", [])]
                area = float(shoelace(ring) - sum(shoelace(h) for h in holes))
        if d is not None:
            edges.append((d, o["id"]))
        elif area is not None:
            areas.append((area, o["id"]))
    # Stable: equals keep the document's order.
    edges.sort(key=lambda e: e[0])
    areas.sort(key=lambda e: e[0])
    return [i for _, i in edges] + [i for _, i in areas]


# ── polygon ─────────────────────────────────────────────────────────────────

def circle_parts(o):
    return (float(o["c"]["x"]), float(o["c"]["y"]), float(o["r"]))


def fl(p):
    return (float(p[0]), float(p[1]))


def fseg_dist(p, a, b):
    ax, ay = a
    bx, by = b
    rx, ry = bx - ax, by - ay
    rr = rx * rx + ry * ry
    t = max(0.0, min(1.0, ((p[0] - ax) * rx + (p[1] - ay) * ry) / rr)) if rr else 0.0
    return math.hypot(p[0] - (ax + rx * t), p[1] - (ay + ry * t))


def circle_inside(o, ring):
    cx, cy, r = circle_parts(o)
    if not holds(q((cx, cy)), ring):
        return False
    return all(fseg_dist((cx, cy), fl(a), fl(b)) >= r for a, b in sides(ring))


def circle_touches(o, ring):
    cx, cy, r = circle_parts(o)
    for a, b in sides(ring):
        far = max(math.hypot(cx - float(a[0]), cy - float(a[1])), math.hypot(cx - float(b[0]), cy - float(b[1])))
        if fseg_dist((cx, cy), fl(a), fl(b)) <= r <= far:
            return True
    # A point of the curve inside, or the ring in the disc.
    return holds(q((cx + r, cy)), ring) or math.hypot(float(ring[0][0]) - cx, float(ring[0][1]) - cy) <= r


def inside(o, ring):
    k = o["kind"]
    if k == "point":
        return holds(q((o["p"]["x"], o["p"]["y"])), ring)
    if k == "circle":
        return circle_inside(o, ring)
    return all(seg_inside(a, b, ring) for a, b in segments(o))


def touches(o, ring):
    k = o["kind"]
    if k == "point":
        return holds(q((o["p"]["x"], o["p"]["y"])), ring)
    if k == "circle":
        return circle_touches(o, ring)
    for a, b in segments(o):
        if seg_meets(a, b, ring) or holds(a, ring):
            return True
    return k == "polygon" and polygon_holds(o, ring[0])


def ring_problem(ring):
    n = len(ring)
    if n < 3:
        return "Çokgen için en az üç köşe gerekir; sonraki köşeyi gösterin."
    crosses = "Çokgen kendini kesiyor; son köşeyi geri alın (G) ya da kesmeyen bir köşe gösterin."
    no_area = "Çokgenin alanı yok; köşeleri bir doğru üzerinde olmayan noktalara verin."
    others = [p for p in ring if p != ring[0]]
    if others and all(cross(ring[0], others[0], p) == 0 for p in ring):
        return no_area
    s = sides(ring)
    for i in range(n):
        a, b = s[i]
        c = s[(i + 1) % n][1]
        if cross(a, b, c) == 0 and (b[0] - a[0]) * (c[0] - b[0]) + (b[1] - a[1]) * (c[1] - b[1]) < 0:
            return crosses
        for j in range(i + 2, n):
            if i == 0 and j == n - 1:
                continue
            if seg_params(a, b, *s[j]):
                return crosses
    if shoelace(ring) == 0:
        return no_area
    return None


# ── The scenes ──────────────────────────────────────────────────────────────

def hit_scene():
    return [
        square(1, 0, 0, 10),
        square(2, 2, 2, 2),
        line(3, (0, 3), (10, 3)),
        point(4, 3.125, 3.125),
        circle(5, (7, 7), 1),
        polyline(6, [(6, 0), (6, 10)]),
        square(7, 20, 20, 10, holes=[[(22, 22), (28, 22), (28, 28), (22, 28)]]),
        line(8, (0, 15), (10, 15)),
        line(9, (0, 16), (10, 16)),
        point(10, 40, 40),
    ]


HITS = [
    ("a line and a point before the building and the parcel around", (3, 3.0625), 0.25),
    ("a circle by its edge, then the parcel; its own area not again", (7, 7.96875), 0.25),
    ("two lines as near: the document's order", (8, 15.5), 0.625),
    ("in a hole: nothing", (25, 25), 0.25),
    ("in an area with a hole, away from it", (21, 21), 0.25),
    ("a point within one and a half tolerances", (40.25, 40), 0.25),
    ("a point beyond one and a half tolerances", (40.25, 40), 0.125),
    ("nothing near", (100, 100), 0.25),
]


def u_ring():
    return [(0, 0), (30, 0), (30, 20), (20, 20), (20, 10), (10, 10), (10, 20), (0, 20)]


def polygon_scene():
    return [
        line(1, (5, 15), (25, 15)),
        line(2, (2, 2), (8, 18)),
        point(3, 15, 15),
        line(4, (0, 0), (30, 0)),
        line(5, (25, 5), (35, 5)),
        square(6, -10, -10, 50),
        point(7, 100, 100),
        square(8, -50, -50, 130, holes=[[(-20, -20), (50, -20), (50, 50), (-20, 50)]]),
        polyline(9, [(22, 2), (28, 6), (22, 10), (28, 18)]),
        circle(10, (5, 5), 2),
        circle(11, (15, 0), 3),
        circle(12, (15, 15), 2),
        point(13, 30, 10),
        polygon(14, [(12, 2), (18, 2), (15, 8)]),
        polyline(15, [(5, 25), (25, 25)]),
        line(16, (10, 12), (20, 12)),
    ]


RINGS = [
    ("two corners", [(0, 0), (10, 0)]),
    ("three corners in a line", [(0, 0), (5, 0), (10, 0)]),
    ("a bow tie", [(0, 0), (10, 10), (10, 0), (0, 10)]),
    ("a spike turning back", [(0, 0), (10, 0), (5, 0), (5, 5)]),
    ("a corner on a side it does not end", [(0, 0), (10, 0), (10, 10), (5, 0)]),
    ("the U", u_ring()),
    ("a triangle", [(0, 0), (10, 0), (5, 8)]),
]


def similar_scene():
    def f(i, kind, layer, color=None, symbol=None, block=None):
        return {"id": i, "kind": kind, "block": block, "layer": layer, "color": color, "symbol": symbol}

    return [
        f(1, "polygon", "Parsel"),
        f(2, "polygon", "Parsel", color="#E5484D"),
        f(3, "polygon", "Yapı"),
        f(4, "line", "Parsel"),
        f(5, "polygon", "Parsel", symbol="sys/parsel"),
        f(6, "insert", "Donatı", block="0b0a0c3e-2d5b-4c47-9e7e-1d2c3b4a5f60"),
        f(7, "insert", "Donatı", block="5d1f9a20-7c34-4b8e-a1f2-3c4d5e6f7a80"),
        f(8, "insert", "Donatı", block="0b0a0c3e-2d5b-4c47-9e7e-1d2c3b4a5f60"),
        f(9, "point", "Nokta", color="#E5484D"),
        f(10, "polygon", "Parsel"),
    ]


ALL = {"kind": True, "layer": True, "color": True, "symbol": True}
SIMILAR = [
    ("every criterion", [1], ALL),
    ("the colour left out", [1], {**ALL, "color": False}),
    ("the kind only", [1], {"kind": True, "layer": False, "color": False, "symbol": False}),
    ("the layer only", [4], {"kind": False, "layer": True, "color": False, "symbol": False}),
    ("an insert by its block", [6], ALL),
    ("the colour only, by two examples", [2, 9], {"kind": False, "layer": False, "color": True, "symbol": False}),
    ("no criterion: every object", [3], {"kind": False, "layer": False, "color": False, "symbol": False}),
    ("the symbol only", [5], {"kind": False, "layer": False, "color": False, "symbol": True}),
    ("no example: nothing", [], ALL),
]


def key(o, c):
    return (
        (o["kind"], o["block"]) if c["kind"] else None,
        o["layer"] if c["layer"] else None,
        o["color"] if c["color"] else None,
        o["symbol"] if c["symbol"] else None,
    )


def similar(objects, examples, c):
    wanted = {key(o, c) for o in objects if o["id"] in examples}
    return [o["id"] for o in objects if key(o, c) in wanted]


# ── Writing ─────────────────────────────────────────────────────────────────

def files():
    scene = hit_scene()
    hits_file = {
        "format": "kentos.selection-hits",
        "version": 1,
        "source": SOURCE,
        "objects": scene,
        "cases": [{"name": n, "at": list(p), "tol": t, "expect": hits(scene, p, t)} for n, p, t in HITS],
    }
    pscene = polygon_scene()
    ring = [q(p) for p in u_ring()]
    polygon_file = {
        "format": "kentos.selection-polygon",
        "version": 1,
        "source": SOURCE,
        "objects": pscene,
        "cases": [
            {
                "name": "the U",
                "ring": [list(p) for p in u_ring()],
                "inside": [o["id"] for o in pscene if inside(o, ring)],
                "crossing": [o["id"] for o in pscene if touches(o, ring)],
                "outside": [o["id"] for o in pscene if not touches(o, ring)],
            }
        ],
        "rings": [{"name": n, "ring": [list(p) for p in r], "problem": ring_problem([q(p) for p in r])} for n, r in RINGS],
    }
    sscene = similar_scene()
    similar_file = {
        "format": "kentos.selection-similar",
        "version": 1,
        "source": SOURCE,
        "objects": sscene,
        "cases": [{"name": n, "examples": e, "criteria": c, "expect": similar(sscene, e, c)} for n, e, c in SIMILAR],
    }
    return {"hits.json": hits_file, "polygon.json": polygon_file, "similar.json": similar_file}


def main():
    check = "--check" in sys.argv
    stale = []
    for name, data in files().items():
        text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
        path = OUT / name
        if check:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                stale.append(name)
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
    if check and stale:
        sys.exit(f"fixtures/selection/v1 güncel değil: {', '.join(stale)}; yeniden yazmak için --check'siz çalıştırın.")
    print("fixtures/selection/v1 " + ("güncel." if check else "yazıldı."))


if __name__ == "__main__":
    main()
