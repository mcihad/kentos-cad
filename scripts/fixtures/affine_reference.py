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


def text_turn(rotation, m):
    """A text's turn (degrees) under `m`, from 0 up to 360: its baseline's direction mapped; mirrored, a half
    turn more, so that it stays readable (AutoCAD's MIRRTEXT = 0)."""
    rad = (rotation * math.pi) / 180.0
    d = linear(m, (math.cos(rad), math.sin(rad)))
    rot = (math.atan2(d[1], d[0]) * 180.0) / math.pi
    if reflects(m):
        rot += 180.0
    return math.fmod(math.fmod(rot, 360.0) + 360.0, 360.0)


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
        # The outer ring, an area's holes and its other parts with theirs (docs/adr/0143): every ring by the
        # same map, its arc segments turned the other way under a reflection; elevations stay with their vertices.
        def ring(r):
            r["pts"] = [pt(apply(m, xy(p))) for p in r["pts"]]
            if "bulges" in r and reflects(m):
                r["bulges"] = [-b for b in r["bulges"]]

        ring(e)
        if k == "polygon":
            for h in e.get("holes") or []:
                ring(h)
            for part in e.get("parts") or []:
                ring(part)
                for h in part.get("holes") or []:
                    ring(h)
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
        e["p"] = pt(apply(m, xy(e["p"])))
        e["height"] = e["height"] * s
        e["rotation"] = text_turn(e["rotation"], m)
    elif k == "leader":
        # Its vertices move; its note turns as a text does, readable when mirrored (docs/adr/0146 §4).
        e["pts"] = [pt(apply(m, xy(p))) for p in e["pts"]]
        e["height"] = e["height"] * s
        e["rotation"] = text_turn(e["rotation"], m)
    elif k == "dimension":
        style = e.get("style", "aligned")
        assert style in ("aligned", "ordinate", "arcLength", "jogged", "azimuth", "slope"), style
        e["a"] = pt(apply(m, xy(e["a"])))
        e["b"] = pt(apply(m, xy(e["b"])))
        if "c" in e:
            e["c"] = pt(apply(m, xy(e["c"])))
        # docs/adr/0147 §4: an arc length mirrored measures the same arc, its ends swapped; its offset, a
        # jogged radius's (along the radius) and an ordinate's keep their sign; an ordinate's axis is the
        # world's; a slope keeps its elevations. Aligned, azimuth and slope offsets are sideways: a
        # reflection swaps their sides.
        if style == "arcLength" and reflects(m):
            e["a"], e["b"] = e["b"], e["a"]
        sideways = style in ("aligned", "azimuth", "slope")
        e["offset"] = e["offset"] * s * (-1.0 if sideways and reflects(m) else 1.0)
        e["height"] = e["height"] * s
    elif k == "image":
        # docs/adr/0192 §4: the frame's corners map (lower left, lower right, upper right, upper left, each the lower left
        # plus its sides' shares); under a reflection the mapped upper left is the new lower left and the picture turns
        # over. Its size and turn are the new frame's sides'.
        p, w, h, r = xy(e["p"]), e["width"], e["height"], e["rotation"]
        c, sn = math.cos(r), math.sin(r)
        u, v = (w * c, w * sn), (-h * sn, h * c)
        q = [apply(m, (p[0] + u[0] * a + v[0] * b, p[1] + u[1] * a + v[1] * b)) for a, b in ((0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0))]
        mirrored = e.get("mirror", False)
        if reflects(m):
            corner, along, up, mirrored = q[3], (q[2][0] - q[3][0], q[2][1] - q[3][1]), (q[0][0] - q[3][0], q[0][1] - q[3][1]), not mirrored
        else:
            corner, along, up = q[0], (q[1][0] - q[0][0], q[1][1] - q[0][1]), (q[3][0] - q[0][0], q[3][1] - q[0][1])
        e["p"] = pt(corner)
        e["width"] = js_hypot(*along)
        e["height"] = js_hypot(*up)
        e["rotation"] = math.atan2(along[1], along[0])
        if mirrored:
            e["mirror"] = True
        else:
            e.pop("mirror", None)
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


def js_hypot(a, b):
    """JavaScript's Math.hypot as V8 computes it (the arguments over the largest, the squares summed with Kahan's
    compensation, scaled back): what the drawing's lengths are measured with on both platforms."""
    big = max(abs(a), abs(b))
    if big == 0.0:
        return 0.0
    total, comp = 0.0, 0.0
    for x in (a, b):
        n = abs(x) / big
        summand = n * n - comp
        pre = total + summand
        comp = (pre - total) - summand
        total = pre
    return math.sqrt(total) * big


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
