#!/usr/bin/env python3
"""Independent reference of the snap inside blocks (docs/adr/0274).

    python3 scripts/fixtures/snap_block_cases.py           # writes the file
    python3 scripts/fixtures/snap_block_cases.py --check   # writes nothing; compares

Writes fixtures/snap/v1/blocks.json with Python's standard library, no KentOS code. The geometry core
(`Store::snap_ex`, crates/shared/geometry-core/tests/all/snap.rs) must give the same: the kind and the object
exactly, the point within 1e-9 m.

The rules (ADR 0274): a block insert's pieces snap as objects of their own, placed in the drawing through every
level's similarity (x' = p + s·R(θ)·M·(x − base), M mirroring in the x axis), with the insert's id; a circle stays
a circle, an arc's ends swap under a mirror, its quadrants are the drawing's east, north, west and south; a closed
piece gives its centroid; a text inside gives its insertion point; the insertion point is the outermost insert's
alone (a nested insert gives its pieces, not its own point). What decides is the insert's layer: hidden, nothing
snaps; locked, everything still snaps (a lock forbids editing, not reading); its own snap kinds apply to the pieces
too. A piece's own layer changes how it looks (docs/adr/0260 §1), not whether it is drawn: it snaps as it shows.

The candidates of the kinds asked are written out here from the definitions (the corners, the edges' middles, the
circles' and arcs' centres and quadrants, the texts' points, the closed pieces' centroids); a case's expectation is
the least weighted distance within the aperture. The crossings, the perpendicular foot, the tangent point and the
nearest point are asked one kind at a time with the cursor where only that point is.
"""

import argparse
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "snap" / "v1" / "blocks.json"

KINDS = ["endpoint", "midpoint", "center", "node", "quadrant", "intersection", "perpendicular", "tangent", "nearest",
         "centroid", "extension", "parallel", "grid", "insertion"]
WEIGHT = {"endpoint": 1.0, "node": 1.0, "insertion": 1.0, "intersection": 1.02, "center": 1.08, "centroid": 1.08,
          "quadrant": 1.1, "midpoint": 1.15, "tangent": 1.2, "perpendicular": 1.25}


def P(x, y):
    return {"x": float(x), "y": float(y)}


def layer(i, visible=True, locked=False, snap=None):
    out = {"id": i, "visible": visible, "locked": locked, "pickInterior": True}
    if snap is not None:
        out["snapKinds"] = sum(1 << KINDS.index(k) for k in snap)
    return out


LAYERS = [layer("a"), layer("gizli", visible=False), layer("kilitli", locked=True),
          layer("kenet", snap=["endpoint"])]

# ── The definitions ─────────────────────────────────────────────────────────

# A door: its leaf and its swing, hinged at the base point.
DOOR = {"id": "kapi", "base": P(0, 0), "entities": [
    {"kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": P(0, 0), "b": P(0, 1)},
    {"kind": "arc", "id": 2, "layerId": "", "attrs": {}, "c": P(0, 0), "r": 1.0, "a0": 0.0, "a1": math.pi / 2},
]}
# A room: its walls, a line across, a column, the door twice (the second turned and mirrored), and a text on a
# hidden layer of its own.
ROOM = {"id": "oda", "base": P(0, 0), "entities": [
    {"kind": "polygon", "id": 1, "layerId": "", "attrs": {}, "pts": [P(0, 0), P(6, 0), P(6, 4), P(0, 4)]},
    {"kind": "line", "id": 2, "layerId": "", "attrs": {}, "a": P(0, 2), "b": P(6, 2)},
    {"kind": "circle", "id": 3, "layerId": "", "attrs": {}, "c": P(4.5, 2), "r": 0.5},
    {"kind": "insert", "id": 4, "layerId": "", "attrs": {}, "block": "kapi", "p": P(1.5, 0), "scale": 1.0,
     "rotation": 0.0},
    {"kind": "insert", "id": 5, "layerId": "", "attrs": {}, "block": "kapi", "p": P(6, 1), "scale": 1.0,
     "rotation": math.pi / 2, "mirror": True},
    {"kind": "text", "id": 6, "layerId": "gizli", "attrs": {}, "p": P(1, 3), "text": "Oda", "height": 0.5,
     "rotation": 0.0},
]}
DEFS = {d["id"]: d for d in (DOOR, ROOM)}


def insert(i, block, x, y, scale, rotation, mirror=False, layer_id="a"):
    e = {"kind": "insert", "id": i, "layerId": layer_id, "attrs": {}, "block": block, "p": P(x, y),
         "scale": float(scale), "rotation": float(rotation)}
    if mirror:
        e["mirror"] = True
    return e


I1 = insert(1, "oda", 100, 100, 2, math.radians(30))
I2 = insert(2, "oda", 200, 100, 0.5, 0.0, mirror=True)
I3 = insert(3, "kapi", 300, 100, 1.5, math.radians(-45), mirror=True)
I4 = insert(4, "oda", 400, 100, 1, 0.0, layer_id="gizli")
I5 = insert(5, "oda", 500, 100, 1, 0.0, layer_id="kilitli")
I6 = insert(6, "oda", 600, 100, 1, 0.0, layer_id="kenet")
INSERTS = [I1, I2, I3, I4, I5, I6]
LAYER = {lay["id"]: lay for lay in LAYERS}

# ── Placing ─────────────────────────────────────────────────────────────────


def placement(ins):
    """(a, b, c, d, e, f) with x' = a·x + c·y + e, y' = b·x + d·y + f: p + s·R(θ)·M·x."""
    s, r, k = ins["scale"], ins["rotation"], -1.0 if ins.get("mirror") else 1.0
    co, si = math.cos(r), math.sin(r)
    return (s * co, s * si, -s * k * si, s * k * co, ins["p"]["x"], ins["p"]["y"])


def compose(m, n):
    """m after n."""
    a, b, c, d, e, f = m
    A, B, C, D, E, G = n
    return (a * A + c * B, b * A + d * B, a * C + c * D, b * C + d * D, a * E + c * G + e, b * E + d * G + f)


def apply(m, q):
    a, b, c, d, e, f = m
    return (a * q[0] + c * q[1] + e, b * q[0] + d * q[1] + f)


def xy(p):
    return (p["x"], p["y"])


def pieces(ins, m=(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)):
    """The insert's pieces with the matrix placing each, nested inserts opened where they stand."""
    d = DEFS[ins["block"]]
    m = compose(m, compose(placement(ins), (1.0, 0.0, 0.0, 1.0, -d["base"]["x"], -d["base"]["y"])))
    out = []
    for e in d["entities"]:
        if e["kind"] == "insert":
            out.extend(pieces(e, m))
        else:
            out.append((e, m))
    return out


def scale_of(m):
    return math.sqrt(abs(m[0] * m[3] - m[1] * m[2]))


def angle_of(m, theta):
    q = apply(m, (math.cos(theta), math.sin(theta)))
    o = apply(m, (0.0, 0.0))
    return math.atan2(q[1] - o[1], q[0] - o[0])


def placed_arc(e, m):
    """The arc in the drawing: its centre, radius and the counter-clockwise angles from start to end."""
    c, r = apply(m, xy(e["c"])), e["r"] * scale_of(m)
    a0, a1 = angle_of(m, e["a0"]), angle_of(m, e["a1"])
    if m[0] * m[3] - m[1] * m[2] < 0:
        a0, a1 = a1, a0
    sweep = (a1 - a0) % (2 * math.pi)
    return c, r, a0, sweep


def on_sweep(theta, a0, sweep):
    return (theta - a0) % (2 * math.pi) <= sweep + 1e-12


def quadrants(c, r, a0=0.0, sweep=2 * math.pi):
    q = [(0.0, (c[0] + r, c[1])), (math.pi / 2, (c[0], c[1] + r)), (math.pi, (c[0] - r, c[1])),
         (1.5 * math.pi, (c[0], c[1] - r))]
    return [p for t, p in q if on_sweep(t, a0, sweep)]


def ring_centroid(pts):
    a = mx = my = F(0)
    n = len(pts)
    for i in range(n):
        (x0, y0), (x1, y1) = (F(pts[i][0]), F(pts[i][1])), (F(pts[(i + 1) % n][0]), F(pts[(i + 1) % n][1]))
        cr = x0 * y1 - x1 * y0
        a += cr / 2
        mx += (x0 + x1) * cr / 6
        my += (y0 + y1) * cr / 6
    return (float(mx / a), float(my / a))


def candidates(ins):
    """Every (kind, point) the insert offers of the kinds written out here, in no particular order."""
    out = [("insertion", xy(ins["p"]))]
    for e, m in pieces(ins):
        k = e["kind"]
        if k == "line":
            a, b = apply(m, xy(e["a"])), apply(m, xy(e["b"]))
            out += [("endpoint", a), ("endpoint", b), ("midpoint", ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2))]
        elif k == "polygon":
            local = [xy(q) for q in e["pts"]]
            pts = [apply(m, q) for q in local]
            for i, q in enumerate(pts):
                n = pts[(i + 1) % len(pts)]
                out += [("endpoint", q), ("midpoint", ((q[0] + n[0]) / 2, (q[1] + n[1]) / 2))]
            out.append(("centroid", apply(m, ring_centroid(local))))
        elif k == "circle":
            c, r = apply(m, xy(e["c"])), e["r"] * scale_of(m)
            out += [("center", c)] + [("quadrant", q) for q in quadrants(c, r)]
        elif k == "arc":
            c, r, a0, sw = placed_arc(e, m)
            at = lambda t: (c[0] + r * math.cos(t), c[1] + r * math.sin(t))  # noqa: E731
            out += [("center", c), ("endpoint", at(a0)), ("endpoint", at(a0 + sw)), ("midpoint", at(a0 + sw / 2))]
            out += [("quadrant", q) for q in quadrants(c, r, a0, sw)]
        elif k == "text":
            out.append(("insertion", apply(m, xy(e["p"]))))
    return out


def best(p, kinds, tol):
    """The snap of the kinds written out here: the least weighted distance within the aperture, None when none."""
    found = []
    for ins in INSERTS:
        lay = LAYER[ins["layerId"]]
        if not lay["visible"]:
            continue
        mask = lay.get("snapKinds")
        for kind, q in candidates(ins):
            if kind not in kinds or (mask is not None and not mask & (1 << KINDS.index(kind))):
                continue
            d = math.hypot(q[0] - p[0], q[1] - p[1])
            if d <= tol:
                found.append((WEIGHT[kind] * d, kind, q, ins["id"]))
    if not found:
        return None
    found.sort(key=lambda f: f[0])
    # The winner must be clear of the next other point: no case rests on a tie the order of looking breaks (one
    # point met twice, a leaf's end on its swing's end, is one candidate).
    rest = [f for f in found if math.hypot(f[2][0] - found[0][2][0], f[2][1] - found[0][2][1]) > 1e-9]
    assert not rest or rest[0][0] - found[0][0] > 1e-6, found[:2]
    return found[0][1:]


def tangent_points(f, c, r):
    dx, dy = f[0] - c[0], f[1] - c[1]
    d = math.hypot(dx, dy)
    base = math.atan2(dy, dx)
    off = math.acos(r / d)
    return [(c[0] + r * math.cos(base + s * off), c[1] + r * math.sin(base + s * off)) for s in (1, -1)]


def foot(f, a, b):
    t = ((f[0] - a[0]) * (b[0] - a[0]) + (f[1] - a[1]) * (b[1] - a[1])) / ((b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2)
    return (a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1]))


def line_circle(a, b, c, r):
    """The crossings of the line through a and b with the circle."""
    dx, dy = b[0] - a[0], b[1] - a[1]
    fx, fy = a[0] - c[0], a[1] - c[1]
    A, B, C = dx * dx + dy * dy, 2 * (fx * dx + fy * dy), fx * fx + fy * fy - r * r
    s = math.sqrt(B * B - 4 * A * C)
    return [(a[0] + t * dx, a[1] + t * dy) for t in ((-B - s) / (2 * A), (-B + s) / (2 * A))]


def segments_cross(a, b, c, d):
    den = (b[0] - a[0]) * (d[1] - c[1]) - (b[1] - a[1]) * (d[0] - c[0])
    t = ((c[0] - a[0]) * (d[1] - c[1]) - (c[1] - a[1]) * (d[0] - c[0])) / den
    return (a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1]))


# ── The cases ───────────────────────────────────────────────────────────────

def at(ins, q):
    """A room or door point of an insert, in the drawing."""
    d = DEFS[ins["block"]]
    m = compose(placement(ins), (1.0, 0.0, 0.0, 1.0, -d["base"]["x"], -d["base"]["y"]))
    return apply(m, q)


def off(q, by):
    return (q[0] + by, q[1] + by * 0.5)


def cases():
    out = []
    # A line across I1's left wall, after the inserts in the drawing's order.
    cross = {"kind": "line", "id": 7, "layerId": "a", "attrs": {},
             "a": P(*at(I1, (-2, 3))), "b": P(*at(I1, (2, 3.5)))}
    entities = INSERTS + [cross]

    def case(name, kinds, p, tol, expect=None, frm=None):
        kinds = kinds if isinstance(kinds, list) else [kinds]
        if expect == "best":
            got = best(p, kinds, tol)
            expect = None if got is None else got
        exp = None if expect is None else {"kind": expect[0], "point": [float(expect[1][0]), float(expect[1][1])],
                                           "id": expect[2]}
        out.append({"name": name, "entities": entities, "layers": LAYERS, "p": [float(p[0]), float(p[1])],
                    "tol": float(tol), "kinds": kinds, "from": None if frm is None else [float(frm[0]), float(frm[1])],
                    "extras": {}, "expect": exp})

    # I1: the room turned 30° and twice as large.
    case("dönük ve büyük odanın köşesi", "endpoint", off(at(I1, (6, 4)), 0.2), 1.0, "best")
    case("dönük odanın duvarının ortası", "midpoint", off(at(I1, (3, 0)), 0.2), 1.0, "best")
    case("odadaki sütunun merkezi", "center", off(at(I1, (4.5, 2)), 0.15), 1.0, "best")
    cw = at(I1, (4.5, 2))
    case("sütunun çeyreği çizimin doğusunda", "quadrant", off((cw[0] + 1, cw[1]), 0.15), 0.6, "best")
    case("iç içe kapının menteşesi: yayın merkezi", "center", off(at(I1, (1.5, 0)), 0.15), 0.6, "best")
    case("iç içe kapının kanadının ucu", "endpoint", off(at(I1, (1.5, 1)), 0.1), 0.5, "best")
    hinge = at(I1, (1.5, 0))
    case("iç içe kapının yayının kuzey çeyreği", "quadrant", off((hinge[0], hinge[1] + 2), 0.1), 0.5, "best")
    m2 = at(I1, (6, 1))
    case("aynalı iç içe kapının yayının ortası", "midpoint",
         off(at(I1, (6 + math.cos(math.pi / 4), 1 + math.sin(math.pi / 4))), 0.1), 0.5, "best")
    case("aynalı iç içe kapının menteşesi", "center", off(m2, 0.1), 0.4, "best")
    case("kendi gizli katmanındaki yazı çizildiği için kenetlenir", "insertion", off(at(I1, (1, 3)), 0.15), 0.8,
         "best")
    case("iç içe kapının ekleme noktası kenetlenmez", "insertion", off(hinge, 0.1), 0.8, "best")
    case("en dıştaki bloğun ekleme noktası", "insertion", off(xy(I1["p"]), 0.1), 0.8, "best")
    case("kapalı parçanın ağırlık merkezi", "centroid", off(at(I1, (3, 2)), 0.2), 1.0, "best")
    # A crossing of two pieces of one insert: the line across and the column.
    a, b = at(I1, (0, 2)), at(I1, (6, 2))
    xs = line_circle(a, b, cw, 1.0)
    east = max(xs, key=lambda q: math.hypot(q[0] - a[0], q[1] - a[1]))
    case("aynı bloğun iki parçasının kesişimi", "intersection", off(east, 0.1), 0.5, ("intersection", east, 1))
    # A crossing of a piece and an object of the drawing: the first of the two in the drawing's order, the insert.
    w = segments_cross(at(I1, (0, 0)), at(I1, (0, 4)), xy(cross["a"]), xy(cross["b"]))
    case("çizimdeki çizgiyle bloğun duvarının kesişimi", "intersection", off(w, 0.1), 0.5, ("intersection", w, 1))
    frm = at(I1, (9, 3))
    ft = foot(frm, at(I1, (6, 0)), at(I1, (6, 4)))
    case("son noktadan dönük duvara dik", "perpendicular", off(ft, 0.1), 0.5, ("perpendicular", ft, 1), frm)
    frm = at(I1, (4.5, 7))
    t = max(tangent_points(frm, cw, 1.0), key=lambda q: q[0])
    case("son noktadan sütuna teğet", "tangent", off(t, 0.05), 0.3, ("tangent", t, 1), frm)
    cur = off(at(I1, (2.2, 4)), 0.1)
    ft = foot(cur, at(I1, (6, 4)), at(I1, (0, 4)))
    case("duvarın üstünde en yakın nokta", "nearest", cur, 0.5, ("nearest", ft, 1))

    # I2: the room mirrored, half as large.
    case("aynalı küçük odanın köşesi", "endpoint", off(at(I2, (6, 4)), 0.05), 0.3, "best")
    case("aynalı odadaki kapının yayının ortası", "midpoint",
         off(at(I2, (1.5 + math.cos(math.pi / 4), math.sin(math.pi / 4))), 0.03), 0.2, "best")
    case("aynalı odadaki aynalı kapının yayının ucu", "endpoint", off(at(I2, (7, 1)), 0.03), 0.2, "best")

    # I3: the door alone, turned −45°, one and a half times as large, mirrored.
    c3 = xy(I3["p"])
    case("aynalı kapının yayının merkezi", "center", off(c3, 0.1), 0.5, "best")
    for name, q in [("ilk ucu", at(I3, (1, 0))), ("öbür ucu", at(I3, (0, 1)))]:
        case(f"aynalı kapının yayının {name}", "endpoint", off(q, 0.05), 0.3, "best")
    case("aynalı kapının yayının ortası", "midpoint",
         off(at(I3, (math.cos(math.pi / 4), math.sin(math.pi / 4))), 0.05), 0.3, "best")
    qs = [q for q in quadrants(c3, 1.5, *placed_arc(DOOR["entities"][1], placement(I3))[2:])]
    assert qs, "the mirrored swing passes a quadrant"
    case("aynalı kapının yayının çeyreği", "quadrant", off(qs[0], 0.05), 0.3, "best")

    # The insert's layer decides.
    case("gizli katmandaki blok kenetlenmez", "endpoint", off(at(I4, (6, 4)), 0.1), 0.5, "best")
    case("kilitli katmandaki blok kenetlenir", "endpoint", off(at(I5, (6, 4)), 0.1), 0.5, "best")
    case("katmanın kendi türleri parçalara da: uç nokta var", "endpoint", off(at(I6, (6, 4)), 0.1), 0.5, "best")
    case("katmanın kendi türleri parçalara da: orta nokta yok", "midpoint", off(at(I6, (3, 0)), 0.1), 0.5, "best")

    for c in out:
        if "yok" in c["name"] or "kenetlenmez" in c["name"]:
            assert c["expect"] is None, c["name"]
        else:
            assert c["expect"] is not None, c["name"]
    return out


def build():
    return {"format": "kentos.snap", "version": 1,
            "source": "scripts/fixtures/snap_block_cases.py (ADR 0274); no KentOS code",
            "kinds": KINDS, "blocks": [DOOR, ROOM], "snap": cases()}


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--check", action="store_true", help="compare with the file instead of writing it")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"blok içi kenet tutarlı: {len(build()['snap'])} durum")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(build()['snap'])} durum")
    return 0


if __name__ == "__main__":
    sys.exit(main())
