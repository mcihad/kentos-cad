"""A 3+1 detached house on a 400 m² plot (20 × 20 m), drawn through KentOS's
product commands: the same calls the desktop's tools make.

Run from the repository root:

    .run/py/bin/python samples/konut_3_1.py

It writes ``samples/konut-3-1.kcad`` and ``samples/konut-3-1.dxf``.
Geometry is in metres (the project's drawing unit is cm, 1/100).
"""

from __future__ import annotations

import math
from pathlib import Path

from kentos.cad import Document

HERE = Path(__file__).resolve().parent

# ------------------------------------------------------------------ the plan
# Plot 0..20 × 0..20, the street on the south. Setbacks: front 5, sides 3,
# rear 3. Footprint 13.00 × 10.80 = 140.40 m².
OUT = (3.5, 5.5, 16.5, 16.3)
EXT = 0.25  # exterior walls
INT = 0.10  # partitions

X0, Y0, X1, Y1 = OUT
WALLS = [
    # exterior
    (X0, Y0, X1, Y0 + EXT),
    (X0, Y1 - EXT, X1, Y1),
    (X0, Y0, X0 + EXT, Y1),
    (X1 - EXT, Y0, X1, Y1),
    # south rooms | corridor and WC
    (3.75, 9.85, 11.85, 9.95),
    (7.55, 5.75, 7.65, 9.85),
    (10.35, 5.75, 10.45, 9.95),
    # WC
    (10.45, 8.05, 11.85, 8.15),
    (11.75, 8.05, 11.85, 9.95),
    # antre | kitchen
    (12.95, 5.75, 13.05, 8.00),
    # corridor | north rooms
    (3.75, 11.15, 10.45, 11.25),
    (7.55, 11.25, 7.65, 16.05),
    (10.35, 11.15, 10.45, 16.05),
    # en-suite
    (3.75, 13.05, 6.05, 13.15),
    (5.95, 11.25, 6.05, 13.15),
]

# Openings cut through the walls: (x0, y0, x1, y1).
WINDOWS = {
    # south wall
    "w_yatak": (4.60, Y0, 6.60, Y0 + EXT),
    "w_banyo": (8.60, Y0, 9.40, Y0 + EXT),
    "w_mutfak": (14.00, Y0, 15.60, Y0 + EXT),
    # north wall
    "w_ebeveyn": (4.40, Y1 - EXT, 6.40, Y1),
    "w_cocuk": (8.10, Y1 - EXT, 9.90, Y1),
    "w_salon_k": (14.90, Y1 - EXT, 15.90, Y1),
    # west and east walls
    "w_eb_banyo": (X0, 11.80, X0 + EXT, 12.50),
    "w_salon_d": (X1 - EXT, 12.20, X1, 14.60),
    "w_mutfak_d": (X1 - EXT, 7.00, X1, 8.60),
}
SLIDING = (11.20, Y1 - EXT, 14.20, Y1)  # salon → terrace
ENTRANCE = (11.55, Y0, 12.55, Y0 + EXT)
# Interior door openings: (x0, y0, x1, y1, hinge, swing) — the hinge end of
# the opening ("a" = low end, "b" = high end) and the side the leaf opens to.
DOORS = [
    ((6.45, 9.85, 7.35, 9.95), "b", -1),  # yatak odası, from the corridor
    ((8.60, 9.85, 9.40, 9.95), "a", -1),  # banyo
    ((10.70, 8.05, 11.40, 8.15), "b", +1),  # WC, from the antre
    ((6.45, 11.15, 7.35, 11.25), "a", +1),  # ebeveyn
    ((8.25, 11.15, 9.15, 11.25), "b", +1),  # çocuk odası
    ((5.95, 12.20, 6.05, 12.90), "b", +1),  # ebeveyn banyosu, opening out
]
OPENINGS = list(WINDOWS.values()) + [SLIDING, ENTRANCE] + [d[0] for d in DOORS]

# Rooms: name, the rectangles making its floor, where its label goes, wet.
ROOMS = [
    ("SALON + YEMEK", [(10.45, 9.95, 16.25, 16.05)], (11.75, 11.15), False),
    ("MUTFAK", [(13.05, 5.75, 16.25, 9.85)], (14.30, 7.55), True),
    ("ANTRE", [(10.45, 5.75, 12.95, 8.05), (11.85, 8.05, 12.95, 9.85)], (11.75, 7.25), False),
    ("WC", [(10.45, 8.15, 11.75, 9.85)], (11.05, 8.75), True),
    ("YATAK ODASI", [(3.75, 5.75, 7.55, 9.85)], (5.30, 9.15), False),
    ("BANYO", [(7.65, 5.75, 10.35, 9.85)], (9.05, 7.75), True),
    ("KORİDOR", [(3.75, 9.95, 10.35, 11.15)], (5.10, 10.55), False),
    ("EBEVEYN YATAK ODASI", [(3.75, 13.15, 7.55, 16.05), (6.05, 11.25, 7.55, 13.15)], (5.10, 15.55), False),
    ("EBV. BANYO", [(3.75, 11.25, 5.95, 13.05)], (5.35, 12.45), True),
    ("ÇOCUK ODASI", [(7.65, 11.25, 10.35, 16.05)], (8.75, 13.30), False),
]


def area(rects) -> float:
    return sum((x1 - x0) * (y1 - y0) for x0, y0, x1, y1 in rects)


def m2(value: float) -> str:
    return f"{value:.2f}".replace(".", ",") + " m²"


# ------------------------------------------------------------- the walls' rings
def union_rings(rects, holes):
    """The outline of axis-parallel rectangles minus openings: rings with the
    inside on the left (counter-clockwise outers, clockwise holes)."""
    xs = sorted({v for r in rects + holes for v in (r[0], r[2])})
    ys = sorted({v for r in rects + holes for v in (r[1], r[3])})

    def inside(box, cx, cy):
        return box[0] < cx < box[2] and box[1] < cy < box[3]

    filled = set()
    for i in range(len(xs) - 1):
        for j in range(len(ys) - 1):
            cx, cy = (xs[i] + xs[i + 1]) / 2, (ys[j] + ys[j + 1]) / 2
            if any(inside(r, cx, cy) for r in rects) and not any(inside(h, cx, cy) for h in holes):
                filled.add((i, j))
    # Boundary edges with the filled cell on their left, on grid indices.
    edges = {}
    for i, j in filled:
        if (i, j - 1) not in filled:
            edges.setdefault((i, j), []).append((i + 1, j))
        if (i + 1, j) not in filled:
            edges.setdefault((i + 1, j), []).append((i + 1, j + 1))
        if (i, j + 1) not in filled:
            edges.setdefault((i + 1, j + 1), []).append((i, j + 1))
        if (i - 1, j) not in filled:
            edges.setdefault((i, j + 1), []).append((i, j))
    rings = []
    while edges:
        start = next(iter(edges))
        ring, at, came = [start], start, None
        while True:
            outs = edges[at]
            if len(outs) == 1 or came is None:
                nxt = outs[0]
            else:
                # Two ways on (cells touching at a corner): turn right first,
                # so the rings do not cross.
                dx, dy = at[0] - came[0], at[1] - came[1]
                right = (at[0] + dy, at[1] - dx)
                nxt = right if right in outs else outs[0]
            outs.remove(nxt)
            if not outs:
                del edges[at]
            came, at = at, nxt
            if at == start:
                break
            ring.append(at)
        # Drop the corners that are not corners.
        n = len(ring)
        kept = []
        for k in range(n):
            a, b, c = ring[k - 1], ring[k], ring[(k + 1) % n]
            if (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]) != 0:
                kept.append(b)
        rings.append([(xs[i], ys[j]) for i, j in kept])
    return rings


def signed_area(ring) -> float:
    return sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(ring, ring[1:] + ring[:1])) / 2


def contains(ring, p) -> bool:
    hit = False
    for (ax, ay), (bx, by) in zip(ring, ring[1:] + ring[:1]):
        if (ay > p[1]) != (by > p[1]) and p[0] < ax + (p[1] - ay) * (bx - ax) / (by - ay):
            hit = not hit
    return hit


def shapes(rects, holes):
    """Outer rings with their holes."""
    rings = union_rings(rects, holes)
    outers = [r for r in rings if signed_area(r) > 0]
    inner = [r for r in rings if signed_area(r) < 0]
    out = []
    for o in outers:
        mine = [h for h in inner if contains(o, h[0]) or contains(o, ((h[0][0] + h[1][0]) / 2, (h[0][1] + h[1][1]) / 2))]
        out.append((o, mine))
    return out


# ------------------------------------------------------------------ drawing
def pt(x, y):
    return {"x": round(x, 6), "y": round(y, 6)}


def pts(ring):
    return [pt(x, y) for x, y in ring]


def rect(x0, y0, x1, y1):
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]


class Plan:
    def __init__(self) -> None:
        self.doc = Document.new(
            "3+1 Müstakil Konut",
            workspace="cad",
            coords="local",
            drawing_unit="cm",
            plot_scale=100,
        )
        self.layers: dict[str, str] = {}
        self.pending: dict[str, list] = {}

    def run(self, command, data):
        answer = self.doc.run(command, data)
        if answer.get("status") != "completed":
            raise RuntimeError(f"{command}: {answer.get('error')}")
        return answer["output"]

    def layer(self, key, name, color, weight, line_type="continuous"):
        out = self.run(
            "cad.layers.edit",
            {"operation": "add", "name": name, "color": color, "lineWeight": weight, "lineType": line_type},
        )
        self.layers[key] = out["layer"]

    def add(self, layer, geometry, **extra):
        self.pending.setdefault(layer, []).append({"geometry": geometry, **extra})

    def flush(self, step=None):
        for layer, objects in self.pending.items():
            body = {"layerId": self.layers.get(layer, layer), "objects": objects}
            if step:
                body["operation"] = step
            self.run("cad.entities.create", body)
        self.pending.clear()

    # primitives -------------------------------------------------------
    def poly(self, layer, ring, closed=True, holes=()):
        if closed:
            g = {"kind": "polygon", "pts": pts(ring)}
            if holes:
                g["holes"] = [{"pts": pts(h)} for h in holes]
        else:
            g = {"kind": "polyline", "pts": pts(ring)}
        self.add(layer, g)

    def line(self, layer, a, b):
        self.add(layer, {"kind": "line", "a": pt(*a), "b": pt(*b)})

    def circle(self, layer, c, r):
        self.add(layer, {"kind": "circle", "c": pt(*c), "r": r})

    def arc(self, layer, c, r, a0, a1):
        self.add(layer, {"kind": "arc", "c": pt(*c), "r": r, "a0": a0, "a1": a1})

    def ellipse(self, layer, c, major, ratio):
        self.add(layer, {"kind": "ellipse", "c": pt(*c), "major": pt(*major), "ratio": ratio, "t0": 0, "t1": 2 * math.pi})

    def text(self, layer, p, value, height, align="middleCenter", rotation=0.0, **extra):
        self.add(
            layer,
            {"kind": "text", "p": pt(*p), "text": value, "height": height, "rotation": rotation, "align": align, **extra},
        )

    def hatch(self, layer, ring, pattern, holes=(), **extra):
        g = {"kind": "hatch", "ring": pts(ring), "pattern": pattern}
        if holes:
            g["holes"] = [pts(h) for h in holes]
        self.add(layer, g, **extra)

    def dim(self, a, b, offset, style="aligned", angle=0.0):
        self.add(
            "olcu",
            {"kind": "dimension", "a": pt(*a), "b": pt(*b), "offset": offset, "height": 0.2, "style": style, "angle": angle,
             "decimals": 0, "unit": "cm"},
        )

    def rounded(self, layer, x0, y0, x1, y1, r):
        """A rectangle with round corners, as a closed polyline with bulges."""
        b = math.tan(math.pi / 8)
        ring = [
            (x0 + r, y0), (x1 - r, y0), (x1, y0 + r), (x1, y1 - r),
            (x1 - r, y1), (x0 + r, y1), (x0, y1 - r), (x0, y0 + r),
        ]
        bulges = [0, b, 0, b, 0, b, 0, b]
        self.add(layer, {"kind": "polygon", "pts": pts(ring), "bulges": bulges})


# ----------------------------------------------------------------- symbols
def window(p: Plan, box):
    x0, y0, x1, y1 = box
    p.poly("pencere", rect(*box))
    if x1 - x0 > y1 - y0:  # in a horizontal wall
        for y in (y0 + (y1 - y0) * 0.4, y0 + (y1 - y0) * 0.6):
            p.line("pencere", (x0, y), (x1, y))
    else:
        for x in (x0 + (x1 - x0) * 0.4, x0 + (x1 - x0) * 0.6):
            p.line("pencere", (x, y0), (x, y1))


def door(p: Plan, box, hinge, swing):
    """A leaf and its swing in an opening of a wall."""
    x0, y0, x1, y1 = box
    horizontal = x1 - x0 > y1 - y0
    if horizontal:
        width = x1 - x0
        face = y1 if swing > 0 else y0
        h = (x0, face) if hinge == "a" else (x1, face)
        closed = (1, 0) if hinge == "a" else (-1, 0)
        open_ = (0, swing)
    else:
        width = y1 - y0
        face = x1 if swing > 0 else x0
        h = (face, y0) if hinge == "a" else (face, y1)
        closed = (0, 1) if hinge == "a" else (0, -1)
        open_ = (swing, 0)
    tip = (h[0] + open_[0] * width, h[1] + open_[1] * width)
    side = (closed[0] * 0.04, closed[1] * 0.04)
    p.poly("kapi", [h, tip, (tip[0] + side[0], tip[1] + side[1]), (h[0] + side[0], h[1] + side[1])])
    a_closed = math.atan2(closed[1], closed[0])
    a_open = math.atan2(open_[1], open_[0])
    sweep = (a_open - a_closed) % (2 * math.pi)
    a0, a1 = (a_closed, a_open) if sweep <= math.pi else (a_open, a_closed)
    p.arc("kapi", h, width, a0 % (2 * math.pi), a1 % (2 * math.pi))


def bed(p: Plan, x0, y0, x1, y1, head):
    """A bed with its pillows at the ``head`` side (w, e, n, s)."""
    p.rounded("mobilya", x0, y0, x1, y1, 0.06)
    double = (x1 - x0 if head in "ns" else y1 - y0) > 1.2
    count = 2 if double else 1
    if head in "we":
        depth = 0.45
        xa = x0 + 0.08 if head == "w" else x1 - 0.08 - depth
        span = (y1 - y0 - 0.16 - 0.1 * (count - 1)) / count
        for k in range(count):
            ya = y0 + 0.08 + k * (span + 0.1)
            p.rounded("mobilya", xa, ya, xa + depth, ya + span, 0.08)
        fold = x0 + 0.62 if head == "w" else x1 - 0.62
        p.line("mobilya", (fold, y0), (fold, y1))
    else:
        depth = 0.45
        ya = y1 - 0.08 - depth if head == "n" else y0 + 0.08
        span = (x1 - x0 - 0.16 - 0.1 * (count - 1)) / count
        for k in range(count):
            xa = x0 + 0.08 + k * (span + 0.1)
            p.rounded("mobilya", xa, ya, xa + span, ya + depth, 0.08)
        fold = y1 - 0.62 if head == "n" else y0 + 0.62
        p.line("mobilya", (x0, fold), (x1, fold))


def wardrobe(p: Plan, x0, y0, x1, y1):
    p.poly("mobilya", rect(x0, y0, x1, y1))
    if x1 - x0 > y1 - y0:
        p.line("mobilya", (x0, (y0 + y1) / 2), (x1, (y0 + y1) / 2))
        n = max(2, round((x1 - x0) / 0.6))
        for k in range(1, n):
            x = x0 + (x1 - x0) * k / n
            p.line("mobilya", (x, y0), (x, y1))
    else:
        p.line("mobilya", ((x0 + x1) / 2, y0), ((x0 + x1) / 2, y1))
        n = max(2, round((y1 - y0) / 0.6))
        for k in range(1, n):
            y = y0 + (y1 - y0) * k / n
            p.line("mobilya", (x0, y), (x1, y))


def toilet(p: Plan, x, y, facing):
    """A toilet with its cistern against the wall on the far side of ``facing``."""
    dx, dy = facing
    tank = 0.18
    if dx == 0:
        cy = y - dy * 0.27
        p.poly("mobilya", rect(x - 0.22, cy - tank / 2, x + 0.22, cy + tank / 2))
        p.ellipse("mobilya", (x, y + dy * 0.05), (0, 0.24), 0.75)
    else:
        cx = x - dx * 0.27
        p.poly("mobilya", rect(cx - tank / 2, y - 0.22, cx + tank / 2, y + 0.22))
        p.ellipse("mobilya", (x + dx * 0.05, y), (0.24, 0), 0.75)


def basin(p: Plan, x0, y0, x1, y1):
    p.poly("mobilya", rect(x0, y0, x1, y1))
    cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
    w, h = (x1 - x0) * 0.36, (y1 - y0) * 0.36
    if w >= h:
        p.ellipse("mobilya", (cx, cy), (w, 0), h / w)
    else:
        p.ellipse("mobilya", (cx, cy), (0, h), w / h)


def chair(p: Plan, cx, cy, back):
    s = 0.21
    p.rounded("mobilya", cx - s, cy - s, cx + s, cy + s, 0.05)
    bx, by = back
    if bx == 0:
        y = cy + by * (s - 0.05)
        p.line("mobilya", (cx - s, y), (cx + s, y))
    else:
        x = cx + bx * (s - 0.05)
        p.line("mobilya", (x, cy - s), (x, cy + s))


def tree(p: Plan, c, r):
    p.circle("peyzaj", c, r)
    p.circle("peyzaj", c, r * 0.12)
    for k in range(7):
        a = 2 * math.pi * k / 7 + 0.3
        p.line("peyzaj", (c[0] + math.cos(a) * r * 0.2, c[1] + math.sin(a) * r * 0.2),
               (c[0] + math.cos(a) * r * 0.82, c[1] + math.sin(a) * r * 0.82))


def shrub(p: Plan, c, r):
    p.circle("peyzaj", c, r)
    p.circle("peyzaj", c, r * 0.55)


def car(p: Plan, x0, y0, x1, y1):
    """A car seen from above, its nose to the north."""
    p.rounded("mobilya", x0, y0, x1, y1, 0.35)
    w = x1 - x0
    p.rounded("mobilya", x0 + 0.18, y0 + 1.25, x1 - 0.18, y1 - 1.45, 0.25)  # cabin
    p.line("mobilya", (x0 + 0.22, y1 - 1.45), (x1 - 0.22, y1 - 1.45))  # windscreen
    p.line("mobilya", (x0 + 0.25, y0 + 1.25), (x1 - 0.25, y0 + 1.25))
    for side in (x0 - 0.08, x1 - 0.12):
        p.poly("mobilya", rect(side, y1 - 1.25, side + 0.2, y1 - 0.75))
        p.poly("mobilya", rect(side, y0 + 0.65, side + 0.2, y0 + 1.15))
    p.line("mobilya", (x0 + w * 0.3, y1 - 0.12), (x1 - w * 0.3, y1 - 0.12))


# --------------------------------------------------------------------- build
def build() -> Plan:
    p = Plan()
    # Layers over the CAD project's own (Çizim, Ölçü, Yazı, Tarama, Yardımcı, Eksen).
    p.layer("parsel", "Parsel", "#C0392B", 0.5)
    p.layer("duvar", "Duvar", "#AEB6C1", 0.5)
    p.layer("kapi", "Kapı", "#C8A165", 0.18)
    p.layer("pencere", "Pencere", "#4AA3DF", 0.18)
    p.layer("mobilya", "Mobilya", "#8A94A6", 0.13)
    p.layer("doseme", "Döşeme", "#6B7685", 0.09)
    p.layer("peyzaj", "Peyzaj", "#5FA35A", 0.13)
    p.layer("cevre", "Yol ve otopark", "#9AA0A6", 0.13)

    # The plot, the setbacks and the street.
    p.poly("parsel", rect(0, 0, 20, 20))
    p.poly("yardimci", rect(3, 5, 17, 17))
    p.text("yardimci", (10, 17.25), "İmar çekme sınırı", 0.18, "bottomCenter")
    for y in (0, -2, -8, -10):  # a 10 m street: sidewalks 2 m, the road 6 m
        p.line("cevre", (-2, y), (22, y))
    p.line("eksen", (-2, -5), (22, -5))
    p.text("yazi", (1, -4.2), "LALE SOKAK (10,00 m)", 0.35, "middleLeft")
    p.text("yazi", (17, -1), "Kaldırım", 0.2, "middleLeft")

    # Walls: one area with its holes, filled.
    for outer, holes in shapes(WALLS, OPENINGS):
        p.poly("duvar", outer, holes=holes)
        p.hatch("duvar", outer, {"type": "solid", "angle": 0, "spacing": 1}, holes=holes)
    p.flush()

    # Windows, doors.
    for box in WINDOWS.values():
        window(p, box)
    sx0, sy0, sx1, sy1 = SLIDING
    p.poly("pencere", rect(*SLIDING))
    mid = (sx0 + sx1) / 2
    p.poly("pencere", rect(sx0, sy0 + 0.06, mid + 0.1, sy0 + 0.12))
    p.poly("pencere", rect(mid - 0.1, sy0 + 0.13, sx1, sy0 + 0.19))
    for box, hinge, swing in DOORS:
        door(p, box, hinge, swing)
    door(p, ENTRANCE, "b", +1)
    p.flush()

    # Floors of the wet rooms, the terrace and the paving.
    for name, rects, _, wet in ROOMS:
        if wet:
            for r in rects:
                p.hatch("doseme", rect(*r), {"type": "cross", "angle": 0, "spacing": 0.3})
    terrace = rect(10.45, 16.3, 16.5, 18.6)
    p.poly("cevre", terrace)
    p.hatch("doseme", terrace, {"type": "lines", "angle": 0, "spacing": 0.2})
    p.text("yazi", (13.475, 17.45), "TERAS", 0.24, mask=True)
    p.text("yazi", (13.475, 17.1), m2(6.05 * 2.3), 0.16, mask=True)
    steps = [(11.45, 4.9, 12.65, 5.5), (11.45, 4.6, 12.65, 4.9)]
    for s in steps:
        p.poly("cevre", rect(*s))
    path = [(11.45, 0), (12.65, 0), (12.65, 4.6), (11.45, 4.6)]
    p.poly("cevre", path)
    p.hatch("doseme", path, {"type": "cross", "angle": 45, "spacing": 0.4})
    park = [(16.9, 0), (19.6, 0), (19.6, 5.6), (16.9, 5.6)]
    p.poly("cevre", park)
    p.hatch("doseme", park, {"type": "cross", "angle": 45, "spacing": 0.5})
    car(p, 17.35, 0.4, 19.15, 5.0)
    p.text("yazi", (18.25, 5.85), "OTOPARK", 0.18, "bottomCenter")
    p.line("cevre", (11.45, 0), (11.45, -2))
    p.line("cevre", (12.65, 0), (12.65, -2))
    p.flush()

    furnish(p)
    landscape(p)
    label(p)
    dimension(p)
    return p


def furnish(p: Plan) -> None:
    # Yatak odası (south-west): double bed against the west wall, wardrobe.
    bed(p, 3.75, 6.75, 5.75, 8.35, "w")
    for y in (6.25, 8.4):
        p.rounded("mobilya", 3.8, y, 4.25, y + 0.45, 0.04)
    wardrobe(p, 6.95, 5.75, 7.55, 8.05)
    p.rounded("mobilya", 5.95, 5.8, 6.85, 6.3, 0.04)  # desk under the window

    # Banyo: bath on the south, WC on the east, basin, washer.
    p.rounded("mobilya", 7.65, 5.75, 9.45, 6.55, 0.06)
    p.rounded("mobilya", 7.8, 5.88, 9.3, 6.42, 0.2)
    toilet(p, 9.98, 7.45, (-1, 0))
    basin(p, 7.65, 7.9, 8.2, 8.75)
    for y in (6.85,):
        p.poly("mobilya", rect(7.65, y, 8.25, y + 0.6))
        p.circle("mobilya", (7.95, y + 0.3), 0.22)
    p.text("mobilya", (7.95, 7.15), "ÇM", 0.12)

    # WC.
    toilet(p, 11.1, 9.55, (0, -1))
    basin(p, 10.45, 8.75, 10.8, 9.2)

    # Antre: coat cupboard and shoe cabinet.
    wardrobe(p, 10.45, 5.75, 11.05, 7.3)
    p.poly("mobilya", rect(12.6, 6.1, 12.95, 7.7))

    # Mutfak: L counter, sink, hob, fridge, island.
    p.poly("mobilya", rect(13.05, 5.75, 15.65, 6.35))
    p.poly("mobilya", rect(15.65, 5.75, 16.25, 8.85))
    p.rounded("mobilya", 14.25, 5.85, 15.15, 6.25, 0.05)  # sink
    p.line("mobilya", (14.7, 5.85), (14.7, 6.25))
    for cx, cy in ((15.82, 7.15), (16.08, 7.15), (15.82, 7.55), (16.08, 7.55)):
        p.circle("mobilya", (cx, cy), 0.09)
    p.poly("mobilya", rect(15.6, 8.95, 16.25, 9.75))
    p.line("mobilya", (15.6, 9.05), (16.25, 9.05))
    p.text("mobilya", (15.92, 9.4), "BZD", 0.12)
    p.rounded("mobilya", 13.55, 8.15, 15.05, 8.95, 0.05)  # island
    for x in (13.85, 14.3, 14.75):
        p.circle("mobilya", (x, 9.2), 0.17)

    # Salon: dining near the kitchen, sitting by the terrace.
    p.rounded("mobilya", 13.2, 10.6, 15.0, 11.5, 0.05)
    for x in (13.6, 14.1, 14.6):
        chair(p, x, 10.3, (0, -1))
        chair(p, x, 11.8, (0, 1))
    chair(p, 12.9, 11.05, (-1, 0))
    chair(p, 15.3, 11.05, (1, 0))
    p.rounded("mobilya", 15.4, 12.25, 16.2, 14.55, 0.12)  # sofa
    p.rounded("mobilya", 15.4, 12.25, 15.62, 14.55, 0.06)
    p.line("mobilya", (15.62, 13.02), (16.2, 13.02))
    p.line("mobilya", (15.62, 13.78), (16.2, 13.78))
    p.rounded("mobilya", 13.9, 12.85, 14.75, 13.95, 0.05)  # coffee table
    for x in (12.85, 13.85):
        p.rounded("mobilya", x, 14.45, x + 0.75, 15.2, 0.12)  # armchairs
        p.rounded("mobilya", x, 15.0, x + 0.75, 15.2, 0.06)
    p.poly("mobilya", rect(10.45, 12.7, 10.9, 14.5))  # TV unit
    p.line("mobilya", (10.95, 13.05), (10.95, 14.15))
    p.add("mobilya", {"kind": "polygon", "pts": pts(rect(11.9, 11.95, 15.25, 14.85))}, lineType="dashed")  # rug
    shrub(p, (16.0, 15.75), 0.22)

    # Ebeveyn: bed against the east partition, wardrobe on the west.
    bed(p, 5.5, 13.55, 7.55, 15.35, "e")
    for y in (13.05, 15.4):
        p.rounded("mobilya", 7.05, y, 7.5, y + 0.45, 0.04)
    wardrobe(p, 3.75, 13.15, 4.35, 15.95)

    # Ebeveyn banyosu: shower, WC, basin.
    p.poly("mobilya", rect(3.75, 12.15, 4.75, 13.05))
    p.line("mobilya", (3.75, 12.15), (4.75, 13.05))
    p.line("mobilya", (3.75, 13.05), (4.75, 12.15))
    toilet(p, 4.3, 11.62, (0, 1))
    basin(p, 5.05, 11.25, 5.65, 11.7)

    # Çocuk odası: single bed, wardrobe, desk.
    bed(p, 9.45, 13.55, 10.35, 15.55, "n")
    wardrobe(p, 7.65, 12.3, 8.25, 14.4)
    p.rounded("mobilya", 7.75, 15.45, 9.25, 16.05, 0.04)
    chair(p, 8.5, 15.1, (0, -1))
    p.flush()


def landscape(p: Plan) -> None:
    for c, r in (((1.6, 2.3), 1.3), ((7.6, 2.6), 1.5), ((1.7, 13.0), 1.2),
                 ((1.9, 18.0), 1.4), ((6.6, 18.4), 1.3), ((18.3, 12.5), 1.3), ((18.4, 17.9), 1.2)):
        tree(p, c, r)
    for x in (0.6, 3.6, 4.6, 9.6, 10.6, 13.6, 14.6, 15.6):
        shrub(p, (x, 0.55), 0.35)
    for y in (6.5, 8.0, 9.5):
        shrub(p, (19.4, y), 0.35)
    p.flush()


def label(p: Plan) -> None:
    gross = (X1 - X0) * (Y1 - Y0)
    net = 0.0
    for name, rects, (x, y), _ in ROOMS:
        a = area(rects)
        net += a
        p.text("yazi", (x, y + 0.12), name, 0.22, "bottomCenter", mask=True)
        p.text("yazi", (x, y - 0.06), m2(a), 0.16, "topCenter", mask=True)
    p.text("yazi", (11.6, 3.4), "GİRİŞ", 0.2, "middleLeft", rotation=90.0, mask=True)
    # North arrow.
    nx, ny = 22.4, 17.0
    p.poly("cizim", [(nx, ny + 1.4), (nx + 0.45, ny), (nx, ny + 0.35), (nx - 0.45, ny)])
    p.hatch("cizim", [(nx, ny + 1.4), (nx, ny + 0.35), (nx - 0.45, ny)], {"type": "solid", "angle": 0, "spacing": 1})
    p.circle("cizim", (nx, ny + 0.6), 0.85)
    p.text("yazi", (nx, ny + 1.75), "K", 0.35, "bottomCenter")
    # Title.
    p.text("yazi", (0, -11.2), "3+1 MÜSTAKİL KONUT", 0.6, "topLeft", bold=True, font="barlow")
    p.text("yazi", (0, -12.1), "Zemin kat planı, ölçek 1/100", 0.3, "topLeft")
    facts = (
        f"Parsel 400,00 m²    Taban alanı {m2(gross)}    TAKS {gross / 400:.2f}".replace(".", ",")
        + f"    Net kullanım alanı {m2(net)}"
    )
    p.text("yazi", (0, -12.7), facts, 0.22, "topLeft")
    p.flush()


def dimension(p: Plan) -> None:
    # The plot.
    p.dim((0, 20), (20, 20), 1.0)
    p.dim((0, 0), (0, 20), 1.0)
    # The house: overall and wall to wall, on the south and the west.
    p.dim((X0, Y0), (X1, Y0), -2.1)
    chain = [X0, 7.6, 10.4, 13.0, X1]
    for a, b in zip(chain, chain[1:]):
        p.dim((a, Y0), (b, Y0), -1.4)
    p.dim((X0, Y0), (X0, Y1), 1.9)
    chain = [Y0, 9.9, 11.2, 16.3 - 0, Y1]
    for a, b in zip(chain, chain[1:]):
        if b > a:
            p.dim((X0, a), (X0, b), 1.2)
    # Setbacks.
    p.dim((14.4, 0), (14.4, Y0), 0.0)
    p.dim((X1, 15.2), (20, 15.2), 0.0)
    p.dim((8.6, Y1), (8.6, 20), 0.0)
    p.flush()


def main() -> None:
    p = build()
    p.doc.save(HERE / "konut-3-1.kcad")
    p.doc.export_dxf(HERE / "konut-3-1.dxf")
    print("yazıldı:", HERE / "konut-3-1.kcad")


if __name__ == "__main__":
    main()
