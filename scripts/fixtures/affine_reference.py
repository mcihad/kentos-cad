"""The plane's similarities and what they do to each kind of drawing object,
from their definitions (docs/adr/0037, 0047): the independent side of the
shared cases of cad.entities.transform and cad.entities.array.

The maps are [a, b, c, d, e, f]: x' = a*x + c*y + e, y' = b*x + d*y + f,
computed in IEEE double arithmetic in the same order of operations as
their definitions are written, with Python's own math.cos/sin/atan2/hypot.
Nothing here is copied from an implementation's output; the web's and the
desktop's handlers must meet these values bit for bit.
"""
import json
import math

TAU = math.pi * 2.0


def translation(dx, dy):
    return [1.0, 0.0, 0.0, 1.0, dx, dy]


def rotation(angle, o):
    c = math.cos(angle)
    s = math.sin(angle)
    return [c, s, -s, c, o[0] - c * o[0] + s * o[1], o[1] - s * o[0] - c * o[1]]


def scaling(s, o):
    return [s, 0.0, 0.0, s, o[0] * (1.0 - s), o[1] * (1.0 - s)]


def mirror(p, q):
    dx = q[0] - p[0]
    dy = q[1] - p[1]
    l2 = dx * dx + dy * dy
    a = (dx * dx - dy * dy) / l2
    b = (2.0 * dx * dy) / l2
    return [a, b, b, -a, p[0] - a * p[0] - b * p[1], p[1] - b * p[0] + a * p[1]]


def compose(m2, m1):
    """First m1, then m2: the product of the two maps, each term as a*x + c*y + e writes it."""
    a1, b1, c1, d1, e1, f1 = m1
    a2, b2, c2, d2, e2, f2 = m2
    return [a2 * a1 + c2 * b1, b2 * a1 + d2 * b1, a2 * c1 + c2 * d1, b2 * c1 + d2 * d1, a2 * e1 + c2 * f1 + e2, b2 * e1 + d2 * f1 + f2]


def alignment(s1, d1, s2=None, d2=None, scale=False):
    """s1 onto d1; with a second pair the direction s1→s2 turned onto d1→d2, scaled to fit with `scale`."""
    if s2 is None:
        return translation(d1[0] - s1[0], d1[1] - s1[1])
    ls = math.hypot(s2[0] - s1[0], s2[1] - s1[1])
    ld = math.hypot(d2[0] - d1[0], d2[1] - d1[1])
    turn = math.atan2(d2[1] - d1[1], d2[0] - d1[0]) - math.atan2(s2[1] - s1[1], s2[0] - s1[0])
    k = ld / ls if scale else 1.0
    return compose(translation(d1[0] - s1[0], d1[1] - s1[1]), compose(rotation(turn, s1), scaling(k, s1)))


def apply(m, p):
    return (m[0] * p[0] + m[2] * p[1] + m[4], m[1] * p[0] + m[3] * p[1] + m[5])


def linear(m, v):
    return (m[0] * v[0] + m[2] * v[1], m[1] * v[0] + m[3] * v[1])


def det(m):
    return m[0] * m[3] - m[1] * m[2]


def scale_of(m):
    return math.sqrt(abs(det(m)))


def reflects(m):
    return det(m) < 0.0


def norm_angle(a):
    r = math.fmod(a, TAU)
    return r + TAU if r < 0.0 else r


def xy(p):
    return (p["x"], p["y"])


def pt(t):
    return {"x": t[0], "y": t[1]}


def moved(e, m, new_id=None):
    """`e` under `m`: the geometry by the transforms' definitions, every other field kept."""
    e = json.loads(json.dumps(e))
    if new_id is not None:
        e["id"] = new_id
    k = e["kind"]
    s = scale_of(m)
    if k == "point":
        e["p"] = pt(apply(m, xy(e["p"])))
    elif k == "line":
        e["a"] = pt(apply(m, xy(e["a"])))
        e["b"] = pt(apply(m, xy(e["b"])))
    elif k in ("polyline", "polygon"):
        e["pts"] = [pt(apply(m, xy(p))) for p in e["pts"]]
        if "bulges" in e and reflects(m):
            e["bulges"] = [-b for b in e["bulges"]]
    elif k == "circle":
        e["c"] = pt(apply(m, xy(e["c"])))
        e["r"] = e["r"] * s
    elif k == "arc":
        c, r = xy(e["c"]), e["r"]
        start = (c[0] + math.cos(e["a0"]) * r, c[1] + math.sin(e["a0"]) * r)
        end = (c[0] + math.cos(e["a1"]) * r, c[1] + math.sin(e["a1"]) * r)
        c2 = apply(m, c)
        s2, e2 = apply(m, start), apply(m, end)
        ang = lambda q: norm_angle(math.atan2(q[1] - c2[1], q[0] - c2[0]))
        e["c"] = pt(c2)
        e["r"] = r * s
        if reflects(m):
            e["a0"], e["a1"] = ang(e2), ang(s2)
        else:
            e["a0"], e["a1"] = ang(s2), ang(e2)
    elif k == "ellipse":
        e["c"] = pt(apply(m, xy(e["c"])))
        e["major"] = pt(linear(m, xy(e["major"])))
        if reflects(m):
            e["t0"], e["t1"] = norm_angle(-e["t1"]), norm_angle(-e["t0"])
    elif k == "spline":
        e["pts"] = [pt(apply(m, xy(p))) for p in e["pts"]]
    elif k == "text":
        rad = (e["rotation"] * math.pi) / 180.0
        d = linear(m, (math.cos(rad), math.sin(rad)))
        rot = (math.atan2(d[1], d[0]) * 180.0) / math.pi
        if reflects(m):
            rot += 180.0
        rot = math.fmod(math.fmod(rot, 360.0) + 360.0, 360.0)
        e["p"] = pt(apply(m, xy(e["p"])))
        e["height"] = e["height"] * s
        e["rotation"] = rot
    elif k == "dimension":
        assert e.get("style", "aligned") == "aligned"
        e["a"] = pt(apply(m, xy(e["a"])))
        e["b"] = pt(apply(m, xy(e["b"])))
        e["offset"] = e["offset"] * s * (-1.0 if reflects(m) else 1.0)
        e["height"] = e["height"] * s
    elif k == "hatch":
        # The pattern's lines keep their direction on the object, modulo a half turn; their spacing scales.
        rad = (e["pattern"]["angle"] * math.pi) / 180.0
        d = linear(m, (math.cos(rad), math.sin(rad)))
        angle = math.fmod(math.fmod((math.atan2(d[1], d[0]) * 180.0) / math.pi, 180.0) + 180.0, 180.0)
        e["ring"] = [pt(apply(m, xy(p))) for p in e["ring"]]
        if "holes" in e:
            e["holes"] = [[pt(apply(m, xy(p))) for p in h] for h in e["holes"]]
        e["pattern"]["angle"] = angle
        e["pattern"]["spacing"] = e["pattern"]["spacing"] * s
    else:
        raise ValueError(k)
    return e


def assert_no_negative_zero(v, where):
    """JSON cannot hold what JavaScript writes of −0: the cases keep clear of it."""
    if isinstance(v, float) and v == 0.0 and math.copysign(1.0, v) < 0:
        raise AssertionError(f"−0 at {where}")
    if isinstance(v, dict):
        for k, x in v.items():
            assert_no_negative_zero(x, f"{where}.{k}")
    if isinstance(v, list):
        for i, x in enumerate(v):
            assert_no_negative_zero(x, f"{where}[{i}]")
