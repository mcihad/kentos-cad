#!/usr/bin/env python3
"""Independent reference of an insert's own y factor (docs/adr/0315 §3; plan/cad.md CAD-BLOCK-03).

    python3 scripts/fixtures/block_scale_cases.py           # writes the file
    python3 scripts/fixtures/block_scale_cases.py --check   # writes nothing; compares

Writes fixtures/blocks/v1/scales.json with Python's standard library, no KentOS code. The geometry core
(`Blocks::expand`, `Blocks::explode`; crates/shared/geometry-core/tests/all/block_scales.rs) must give the same
geometry, compared as geometry (an ellipse by its centre, semi-axes and axis, an arc by its ends, sense and a
middle point), within 1e-9 m; a path's arc segments become straight vertices on the exact image within 0.1 mm.

The rules (ADR 0315 §3): an insert places its definition by x' = p + R(θ)·S·M·(x − base), S = diag(sx, sy), M
mirroring in the definition's x axis. Straight geometry maps exactly; a circle and an ellipse become the exact
ellipse of their image, an arc an arc of it, its sense kept unless the map mirrors; a text keeps its shape at its
point's derivative J: its run along u = (cos α, sin α) becomes J·u, its height h·|det J|/|J·u|, its width factor
w·|J·u|²/|det J|, its turn the direction of J·u (a half turn more when J mirrors). Nested inserts compose. Patlat
keeps a nested insert as one while its composed axes stay square (a turn, two factors, a mirror), and refuses when
they shear.
"""
import json
import math
import sys

OUT = "fixtures/blocks/v1/scales.json"


def P(x, y):
    return {"x": x, "y": y}


def obj(kind, i, **f):
    return {"kind": kind, "id": i, "layerId": "", "attrs": {}, **f}


DAIRE = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4101"
YAY = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4102"
ODA = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4103"
DIS = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4104"
DUZ = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4105"
YOL = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4106"

DEFS = [
    {"id": DAIRE, "base": P(0, 0), "entities": [obj("circle", 1, c=P(2, 1), r=1)]},
    {"id": YAY, "base": P(0, 0), "entities": [obj("arc", 1, c=P(0, 0), r=1, a0=0, a1=math.pi / 2)]},
    {"id": ODA, "base": P(1, 1), "entities": [
        obj("polygon", 1, pts=[P(1, 1), P(5, 1), P(5, 4), P(1, 4)]),
        obj("line", 2, a=P(1, 1), b=P(5, 4)),
        obj("ellipse", 3, c=P(3, 2.5), major=P(1, 0), ratio=0.5, t0=0, t1=2 * math.pi),
        obj("text", 4, p=P(2, 3), text="ODA", height=0.5, rotation=0),
    ]},
    # A turned door inside: Patlat keeps it only while the composition stays square.
    {"id": DIS, "base": P(0, 0), "entities": [
        {**obj("insert", 1), "block": DAIRE, "p": P(0, 0), "scale": 1, "rotation": math.pi / 4},
    ]},
    {"id": DUZ, "base": P(0, 0), "entities": [
        {**obj("insert", 1), "block": DAIRE, "p": P(1, 0), "scale": 1, "rotation": math.pi / 2},
    ]},
    {"id": YOL, "base": P(0, 0), "entities": [
        obj("polyline", 1, pts=[P(0, 0), P(2, 0), P(4, 0)], bulges=[0.5, 0]),
    ]},
]
BY_ID = {d["id"]: d for d in DEFS}


def insert(block, p, sx, sy=None, rot=0.0, mirror=False):
    e = {"kind": "insert", "id": 1, "layerId": "kat", "attrs": {}, "block": block, "p": p, "scale": sx,
         "rotation": rot}
    if sy is not None:
        e["yScale"] = sy
    if mirror:
        e["mirror"] = True
    return e


# ── The affine maps ──────────────────────────────────────────────────────

def placement(ins):
    sx = ins["scale"]
    sy = ins.get("yScale", sx)
    k = -1.0 if ins.get("mirror") else 1.0
    c, s = math.cos(ins["rotation"]), math.sin(ins["rotation"])
    # x' = p + R·S·M·x: columns R·(sx, 0) and R·(0, k·sy).
    return (sx * c, sx * s, -k * sy * s, k * sy * c, ins["p"]["x"], ins["p"]["y"])


def compose(m, n):
    a, b, c, d, e, f = m
    A, B, C, D, E, G = n
    return (a * A + c * B, b * A + d * B, a * C + c * D, b * C + d * D, a * E + c * G + e, b * E + d * G + f)


def shift(dx, dy):
    return (1.0, 0.0, 0.0, 1.0, dx, dy)


def apply(m, q):
    return (m[0] * q[0] + m[2] * q[1] + m[4], m[1] * q[0] + m[3] * q[1] + m[5])


def lin(m, v):
    return (m[0] * v[0] + m[2] * v[1], m[1] * v[0] + m[3] * v[1])


def det(m):
    return m[0] * m[3] - m[1] * m[2]


def xy(p):
    return (p["x"], p["y"])


def pieces(ins, m=(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)):
    d = BY_ID[ins["block"]]
    m = compose(m, compose(placement(ins), shift(-d["base"]["x"], -d["base"]["y"])))
    out = []
    for e in d["entities"]:
        if e["kind"] == "insert":
            out += pieces(e, m)
        else:
            out.append((e, m))
    return out


# ── An ellipse as an image ───────────────────────────────────────────────

def ellipse_of(c, u, v, m):
    """The image of x(t) = c + u·cos t + v·sin t: its centre and its semi-axes (major first) with the major's
    direction, from the singular values of [m·u | m·v]."""
    U, V = lin(m, u), lin(m, v)
    # x(t) − c = U cos t + V sin t; |x − c|² extremes at tan 2t = 2 U·V / (U·U − V·V).
    uu, vv, uv = U[0] ** 2 + U[1] ** 2, V[0] ** 2 + V[1] ** 2, U[0] * V[0] + U[1] * V[1]
    t = 0.5 * math.atan2(2 * uv, uu - vv)
    a = (U[0] * math.cos(t) + V[0] * math.sin(t), U[1] * math.cos(t) + V[1] * math.sin(t))
    b = (-U[0] * math.sin(t) + V[0] * math.cos(t), -U[1] * math.sin(t) + V[1] * math.cos(t))
    la, lb = math.hypot(*a), math.hypot(*b)
    if lb > la:
        a, b, la, lb = b, a, lb, la
    return {"center": list(apply(m, c)), "semi": [la, lb], "axis": math.atan2(a[1], a[0]) % math.pi}


def expected(e, m):
    k = e["kind"]
    if k == "line":
        return {"kind": "line", "ends": [list(apply(m, xy(e["a"]))), list(apply(m, xy(e["b"])))]}
    if k == "polygon":
        return {"kind": "polygon", "corners": [list(apply(m, xy(q))) for q in e["pts"]]}
    if k == "circle":
        r = e["r"]
        out = ellipse_of(xy(e["c"]), (r, 0), (0, r), m)
        # A circle stays a circle under a similarity, an ellipse otherwise.
        out["kind"] = "circle" if abs(out["semi"][0] - out["semi"][1]) <= 1e-12 * out["semi"][0] else "ellipse"
        return out
    if k == "ellipse":
        mj = xy(e["major"])
        mn = (-mj[1] * e["ratio"], mj[0] * e["ratio"])
        out = ellipse_of(xy(e["c"]), mj, mn, m)
        out["kind"] = "ellipse"
        return out
    if k == "arc":
        c, r, a0, a1 = xy(e["c"]), e["r"], e["a0"], e["a1"]
        at = lambda t: apply(m, (c[0] + r * math.cos(t), c[1] + r * math.sin(t)))  # noqa: E731
        start, end, mid = at(a0), at(a1), at((a0 + a1) / 2)
        # A mirror runs it the other way: its counter-clockwise start is the image of its end.
        if det(m) < 0:
            start, end = end, start
        out = ellipse_of(c, (r, 0), (0, r), m)
        out.update({"kind": "arc", "start": list(start), "end": list(end), "mid": list(mid)})
        return out
    if k == "text":
        J = m
        al = math.radians(e["rotation"])
        Ju = lin(J, (math.cos(al), math.sin(al)))
        length, d = math.hypot(*Ju), det(J)
        turn = math.degrees(math.atan2(Ju[1], Ju[0])) + (180 if d < 0 else 0)
        return {"kind": "text", "p": list(apply(m, xy(e["p"]))), "height": e["height"] * abs(d) / length,
                "widthFactor": 1.0 * length * length / abs(d), "rotation": turn % 360}
    if k == "polyline":
        # Its arc edges straight within 0.1 mm of the exact image: the ends exact, every vertex on the image.
        p0, p1, p2 = (xy(q) for q in e["pts"])
        # The bulge 0.5 arc from p0 to p1: its centre and radius.
        b = e["bulges"][0]
        chord = math.dist(p0, p1)
        r = chord * (1 + b * b) / (4 * abs(b))
        mx, my = (p0[0] + p1[0]) / 2, (p0[1] + p1[1]) / 2
        h = r - abs(b) * chord / 2
        # Counter-clockwise (b > 0) from p0 to p1 bends right of the chord p0→p1 here (below the x axis).
        n = ((p1[1] - p0[1]) / chord, -(p1[0] - p0[0]) / chord)
        cc = (mx - n[0] * h, my - n[1] * h) if b > 0 else (mx + n[0] * h, my + n[1] * h)
        return {"kind": "polyline", "first": list(apply(m, p0)), "last": list(apply(m, p2)),
                "arc": {"center": list(apply(m, cc)), "u": list(lin(m, (r, 0))), "v": list(lin(m, (0, r)))},
                "joint": list(apply(m, p1))}
    raise ValueError(k)


CASES = [
    ("Daire, x 2 y 1: yatay büyük eksenli elips", insert(DAIRE, P(10, 10), 2, 1)),
    ("Daire, x 2 y 1, çeyrek dönüş: büyük eksen düşey", insert(DAIRE, P(10, 10), 2, 1, math.pi / 2)),
    ("Daire, x 1 y 3: büyük eksen y boyunca, oran 1/3", insert(DAIRE, P(0, 0), 1, 3)),
    ("Daire, x 2 y 1, aynalı ve 30° dönük", insert(DAIRE, P(5, -5), 2, 1, math.pi / 6, True)),
    ("Daire, eşit ölçek: daire kalır", insert(DAIRE, P(1, 2), 1.5, 1.5)),
    ("Yay, x 2 y 1: eliptik yay, uçlar ve ortası", insert(YAY, P(0, 0), 2, 1)),
    ("Yay, x 2 y 1, aynalı: yön ters, uçlar yer değiştirir", insert(YAY, P(0, 0), 2, 1, 0.0, True)),
    ("Yay, x 0,5 y 3, 120° dönük", insert(YAY, P(3, 4), 0.5, 3, 2 * math.pi / 3)),
    ("Oda, x 1,5 y 0,5: köşeler, çapraz, elips ve yazı", insert(ODA, P(100, 200), 1.5, 0.5)),
    ("Oda, x 1,5 y 0,5, 45° dönük ve aynalı", insert(ODA, P(100, 200), 1.5, 0.5, math.pi / 4, True)),
    ("İç içe: dışta x 2 y 1, içte 45° dönük daire", insert(DIS, P(0, 0), 2, 1)),
    ("İç içe: dışta x 3 y 1 ve 30°, içte çeyrek dönük daire", insert(DUZ, P(1, 1), 3, 1, math.pi / 6)),
    ("Yaylı çoklu çizgi, x 2 y 1: yay kenarı 0,1 mm içinde köşeler", insert(YOL, P(0, 0), 2, 1)),
]


def explode(ins):
    """Patlat one level open: a nested insert stays one while its composed axes are square, else refused."""
    d = BY_ID[ins["block"]]
    m = compose(placement(ins), shift(-d["base"]["x"], -d["base"]["y"]))
    out = []
    for e in d["entities"]:
        if e["kind"] != "insert":
            out.append(expected(e, m))
            continue
        L = compose(m, placement({**e, "p": P(0, 0)}))
        u, v = (L[0], L[1]), (L[2], L[3])
        su, sv = math.hypot(*u), math.hypot(*v)
        if abs(u[0] * v[0] + u[1] * v[1]) > 1e-12 * su * sv:
            return {"refused": True}
        flip = u[0] * v[1] - u[1] * v[0] < 0
        out.append({"kind": "insert", "p": list(apply(m, xy(e["p"]))), "scale": su,
                    "yScale": None if abs(su - sv) <= 1e-15 * su else sv,
                    "rotation": math.atan2(u[1], u[0]), "mirror": flip})
    return {"pieces": out}


EXPLODE_CASES = [
    ("Patlat: dışta x 2 y 1, içte 45° dönük blok eğilir: reddedilir", insert(DIS, P(0, 0), 2, 1)),
    ("Patlat: dışta x 3 y 1, içte çeyrek dönük blok: x 1 y 3, çeyrek dönük", insert(DUZ, P(1, 1), 3, 1)),
    ("Patlat: dışta x 3 y 1 aynalı, içte çeyrek dönük blok", insert(DUZ, P(1, 1), 3, 1, 0.0, True)),
    ("Patlat: oda x 1,5 y 0,5: daire elips, yazı genişler", insert(ODA, P(100, 200), 1.5, 0.5)),
]


def build():
    cases = []
    for name, ins in CASES:
        cases.append({"name": name, "insert": ins, "pieces": [expected(e, m) for e, m in pieces(ins)]})
    explodes = [{"name": n, "insert": ins, "expect": explode(ins)} for n, ins in EXPLODE_CASES]
    return {"format": "kentos.block-scale-cases", "version": 1,
            "note": "Written by scripts/fixtures/block_scale_cases.py from docs/adr/0315 §3; do not edit by hand.",
            "definitions": DEFS, "cases": cases, "explodes": explodes}


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        with open(OUT, encoding="utf-8") as f:
            if f.read() != text:
                sys.exit(f"{OUT} güncel değil")
        print(f"{OUT}: güncel")
        return
    with open(OUT, "w", encoding="utf-8") as f:
        f.write(text)
    print(f"yazıldı: {OUT}")


main()
