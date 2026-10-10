#!/usr/bin/env python3
"""The label engine's cases (docs/adr/0212), from the ADR, not from either platform's code: fixtures/labels/v1/cases.json.

Each case is a drawing (objects as the contract writes them, the layers' labelling, the objects' label texts and pins,
the outlines of the drawing's texts), a window and a scale; what this reference says is placed: each label's frame (its
middle in world coordinates, its angle in degrees, its block's width and height in px, its class and state), its lines
or letters and its callout, in the order the engine draws them, and what a click at a few points picks
(`label_at`). The Rust store (`crates/shared/geometry-core/tests/all/label_engine.rs`) must place the same; the web
and the desktop draw what the store places.

The page is in px, the window's lower left its origin, y up (docs/adr/0212 §3). The letters' advances are the drawing's
typefaces' measured ones (crates/shared/geometry-core/src/text/metrics.rs, measured in Chrome).

`--check` says whether the file holds what this script writes.
"""
import heapq
import json
import math
import re
import sys
from bisect import bisect_right
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/labels/v1/cases.json"
METRICS = ROOT / "crates/shared/geometry-core/src/text/metrics.rs"

# ── The measured advances ───────────────────────────────────────────────


def read_metrics():
    src = METRICS.read_text("utf-8")
    first = int(re.search(r"pub const FIRST: u32 = (\d+);", src).group(1))
    last = int(re.search(r"pub const LAST: u32 = (\d+);", src).group(1))
    fonts = json.loads("[" + re.search(r"pub const FONTS: \[&str; \d+\] = \[(.*?)\];", src).group(1) + "]")

    def table(name):
        body = src[src.index(f"pub const {name}:"):]
        body = body[body.index("= [") + 3:]
        rows, depth, cur = [], 0, []
        for tok in re.finditer(r"\[|\]|\d+|//[^\n]*", body):
            s = tok.group(0)
            if s.startswith("//"):
                continue
            if s == "[":
                depth += 1
                cur = []
            elif s == "]":
                if depth == 0:
                    break
                depth -= 1
                rows.append(cur)
            else:
                cur.append(int(s))
        return rows

    regular, bold = table("ADVANCES"), table("BOLD")
    assert len(regular) == len(fonts) == len(bold)
    return first, last, fonts, regular, bold


FIRST, LAST, FONTS, REGULAR, BOLD = read_metrics()


def advance(font, c, bold):
    row = (BOLD if bold else REGULAR)[FONTS.index(font)]
    code = ord(c)
    if FIRST <= code <= LAST and row[code - FIRST] > 0:
        return row[code - FIRST]
    return sum(row[ord("a") - FIRST:ord("z") - FIRST + 1]) // 26


def line_width(line, font, bold, size):
    """A line's width in px: its letters' advances (thousandths of an em) times the size."""
    if not line:
        return 0.0
    return sum(advance(font, c, bold) for c in line) / 1000.0 * size


def letter_width(c, font, bold, size):
    return advance(font, c, bold) / 1000.0 * size


# ── A class: the style's values, every default resolved (§2, §3.3) ─────

HALO = 1.5          # a label's halo when its style names none, px
PADDING = 2.0       # a background's padding when it names none, px
SPACING = 1.0       # the room between two labels, px
LINE_HEIGHT = 1.2   # a line's height over the size
CONTOUR_REPEAT = 400.0
BOUNDARY_REPEAT = 300.0
CALLOUT_MIN = 6.0
MAX_ANGLE = 25.0
DISTANCE = 2.0

# The old placements as the kinds' modes (§3.3): point, line, area.
OLD = {
    "center": ("center", "horizontal", "horizontal"),
    "corner": ("corner", "corner", "corner"),
    "beside": ("around", "horizontal", "horizontal"),
    "along": ("around", "parallel", "perimeter"),
}


class Class:
    def __init__(self, v, name):
        self.name = name
        point, line, area = OLD[v.get("placement", "center")]
        self.point = v.get("point", point)
        self.line = v.get("line", line)
        self.area = v.get("area", area)
        self.position = v.get("position", "above" if self.area == "boundary" else "on")
        self.size = float(v["size"])
        self.grow = float(v.get("grow", 0.0))
        self.max_size = v.get("maxSize")
        self.bold = v.get("weight", 400) >= 600
        self.min_scale = v.get("minScale")
        self.max_scale = v.get("maxScale")
        self.min_feature_px = v.get("minFeaturePx")
        self.distance = float(v.get("distance", DISTANCE))
        self.repeat = v.get("repeat", CONTOUR_REPEAT if self.line == "contour" else None)
        self.max_angle = math.radians(v.get("maxAngle", MAX_ANGLE))
        self.curved = v.get("curved", False)
        self.merge_lines = v.get("mergeLines", False)
        self.inside = v.get("inside", False) or self.area == "parcel"
        self.outside = v.get("outside", False)
        self.halo = float(v["halo"]["width"]) if "halo" in v else HALO
        self.background = float(v["background"].get("padding", PADDING)) if "background" in v else None
        self.mask = self.line == "contour" and self.background is None
        s = v.get("stack")
        self.stack = None if s is None else {"always": s.get("mode") == "always", "chars": int(s["chars"]),
                                             "at": list(s.get("at", " "))}
        a = v.get("abbreviate")
        self.abbreviate = None if a is None else {"always": a.get("always", False),
                                                  "words": [(w["word"], w["short"]) for w in a["words"]]}
        self.shrink = float(v.get("shrink", 1.0))
        self.priority = int(min(max(v.get("priority", 5), 0), 10))
        self.overlap = v.get("overlap", "never")
        self.duplicates = v.get("duplicates")
        self.callout = None if "callout" not in v else float(v["callout"].get("minLength", CALLOUT_MIN))
        self.align = v.get("align", "center")

    def size_at(self, scale):
        grown = self.size + self.grow * scale
        return self.max_size if self.max_size is not None and self.max_size < grown else grown

    def shown_at(self, scale):
        return not ((self.min_scale is not None and scale < self.min_scale)
                    or (self.max_scale is not None and scale > self.max_scale))

    def pad(self):
        around = self.background if self.background is not None and self.background > self.halo else self.halo
        return around + SPACING

    def outline_repeat(self):
        if self.repeat is not None:
            return self.repeat
        return BOUNDARY_REPEAT if self.area == "boundary" else None


def labelling(row):
    """A layer's labelling: (off, classes, obstacle) or None when it has none."""
    if "label" not in row and "labels" not in row:
        return None
    labels = row.get("labels")
    mode = "single" if labels is None else labels["mode"]
    obstacle = None
    if labels is not None and "obstacle" in labels:
        o = labels["obstacle"]
        obstacle = (int(min(max(o["weight"], 1), 10)), o.get("kind") == "boundary")
    if mode == "off":
        classes = []
    elif mode == "single":
        classes = [Class(row["label"], None)] if "label" in row else []
    else:
        classes = [Class(c["style"], c.get("name", "")) for c in labels["classes"]]
    return {"off": mode == "off", "classes": classes, "obstacle": obstacle}


# ── Words (§3.4) ────────────────────────────────────────────────────────


def lines_of(text):
    return [l.rstrip("\r") for l in text.split("\n")]


def stacked(text, stack):
    """Lines of at most `chars` letters, ended after a stacking letter as late as they can; a space a line ends at
    goes, any other stacking letter stays; a longer piece is a line of its own."""
    out = []
    for own in lines_of(text):
        pieces, cur = [], ""
        for c in own:
            cur += c
            if c in stack["at"]:
                pieces.append(cur)
                cur = ""
        if cur:
            pieces.append(cur)
        line = ""
        for p in pieces:
            if line and len(line) + len(p.rstrip(" ")) > stack["chars"]:
                out.append(line.rstrip(" "))
                line = ""
            line += p
        out.append(line.rstrip(" "))
    return out


def abbreviated(text, a):
    """Each whole word (between spaces and line breaks) the dictionary holds is its short form."""
    out, word = [], ""
    words = a["words"]

    def flush():
        nonlocal word
        if word:
            short = next((s for w, s in words if w == word), None)
            out.append(word if short is None else short)
            word = ""

    for c in text:
        if c in " \n":
            flush()
            out.append(c)
        else:
            word += c
    flush()
    return "".join(out)


# ── The page's geometry ─────────────────────────────────────────────────


def norm(x, y):
    return math.sqrt(x * x + y * y)


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1]


class Obb:
    """A box turned by an angle: middle c, unit direction u of its length, half length a, half height b."""

    def __init__(self, c, angle, a, b, u=None):
        self.c = c
        self.u = u if u is not None else (math.cos(angle), math.sin(angle))
        self.a = a
        self.b = b

    def v(self):
        return (-self.u[1], self.u[0])

    def corners(self):
        u, v, c = self.u, self.v(), self.c
        return [(c[0] + u[0] * s * self.a + v[0] * t * self.b, c[1] + u[1] * s * self.a + v[1] * t * self.b)
                for s, t in ((-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0))]

    def aabb(self):
        v = self.v()
        hx = self.a * abs(self.u[0]) + self.b * abs(v[0])
        hy = self.a * abs(self.u[1]) + self.b * abs(v[1])
        return (self.c[0] - hx, self.c[1] - hy, self.c[0] + hx, self.c[1] + hy)

    def radius(self, l):
        return self.a * abs(dot(self.u, l)) + self.b * abs(dot(self.v(), l))

    def overlaps(self, o):
        """Separating axes: boxes that only touch do not overlap."""
        d = (o.c[0] - self.c[0], o.c[1] - self.c[1])
        for l in (self.u, self.v(), o.u, o.v()):
            if abs(dot(d, l)) >= self.radius(l) + o.radius(l):
                return False
        return True

    def local(self, p):
        d = (p[0] - self.c[0], p[1] - self.c[1])
        return (dot(d, self.u), dot(d, self.v()))

    def contains(self, p):
        q = self.local(p)
        return abs(q[0]) < self.a and abs(q[1]) < self.b

    def hits_segment(self, p, q):
        p, q = self.local(p), self.local(q)
        if max(p[0], q[0]) <= -self.a or min(p[0], q[0]) >= self.a:
            return False
        if max(p[1], q[1]) <= -self.b or min(p[1], q[1]) >= self.b:
            return False
        n = (p[1] - q[1], q[0] - p[0])
        return abs(dot(p, n)) < self.a * abs(n[0]) + self.b * abs(n[1])

    def hits_circle(self, p, r):
        q = self.local(p)
        dx = max(abs(q[0]) - self.a, 0.0)
        dy = max(abs(q[1]) - self.b, 0.0)
        return dx * dx + dy * dy < r * r


def inside_rings(p, rings):
    inside = False
    for ring in rings:
        n = len(ring)
        if n < 3:
            continue
        j = n - 1
        for i in range(n):
            a, b = ring[i], ring[j]
            if (a[1] > p[1]) != (b[1] > p[1]) and p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]:
                inside = not inside
            j = i
    return inside


def segment_distance2(p, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    x, y = a
    if dx != 0.0 or dy != 0.0:
        t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)
        if t > 1.0:
            x, y = b
        elif t > 0.0:
            x += dx * t
            y += dy * t
    ex, ey = p[0] - x, p[1] - y
    return ex * ex + ey * ey


def signed_distance(p, rings):
    least = math.inf
    for ring in rings:
        n = len(ring)
        if n < 2:
            continue
        j = n - 1
        for i in range(n):
            least = min(least, segment_distance2(p, ring[i], ring[j]))
            j = i
    d = math.sqrt(least)
    return d if inside_rings(p, rings) else -d


def box_inside(b, rings):
    if not all(inside_rings(p, rings) for p in b.corners()):
        return False
    for ring in rings:
        n = len(ring)
        if n < 2:
            continue
        j = n - 1
        for i in range(n):
            if b.hits_segment(ring[j], ring[i]):
                return False
            j = i
    return True


def clip_ring(ring, r):
    """Sutherland and Hodgman: left, right, bottom, top."""
    def cut_x(a, b, x):
        t = (x - a[0]) / (b[0] - a[0])
        return (x, a[1] + (b[1] - a[1]) * t)

    def cut_y(a, b, y):
        t = (y - a[1]) / (b[1] - a[1])
        return (a[0] + (b[0] - a[0]) * t, y)

    out = list(ring)
    for keep, cut, at in ((lambda p, x: p[0] >= x, cut_x, r[0]), (lambda p, x: p[0] <= x, cut_x, r[2]),
                          (lambda p, y: p[1] >= y, cut_y, r[1]), (lambda p, y: p[1] <= y, cut_y, r[3])):
        inp, out = out, []
        if not inp:
            break
        prev = inp[-1]
        for p in inp:
            a, b = keep(p, at), keep(prev, at)
            if a:
                if not b:
                    out.append(cut(prev, p, at))
                out.append(p)
            elif b:
                out.append(cut(prev, p, at))
            prev = p
    return out


def clip_path(pts, r):
    """Liang and Barsky's rule segment by segment: the pieces inside, in the path's order."""
    pieces, open_ = [], False
    for a, b in zip(pts, pts[1:]):
        dx, dy = b[0] - a[0], b[1] - a[1]
        t0, t1, ok = 0.0, 1.0, True
        for p, q in ((-dx, a[0] - r[0]), (dx, r[2] - a[0]), (-dy, a[1] - r[1]), (dy, r[3] - a[1])):
            if p == 0.0:
                if q < 0.0:
                    ok = False
                    break
            else:
                t = q / p
                if p < 0.0:
                    t0 = max(t0, t)
                else:
                    t1 = min(t1, t)
        if not ok or t0 > t1:
            open_ = False
            continue
        s = (a[0] + dx * t0, a[1] + dy * t0)
        e = (a[0] + dx * t1, a[1] + dy * t1)
        if not open_ or t0 > 0.0:
            pieces.append([s])
        pieces[-1].append(e)
        open_ = t1 >= 1.0
    return [p for p in pieces if len(p) >= 2]


class Walk:
    """A path walked by its length (repeated points dropped)."""

    def __init__(self, pts, at=None):
        if at is not None:
            self.pts, self.at = pts, at
            return
        self.pts, self.at, s = [], [], 0.0
        for p in pts:
            if self.pts:
                q = self.pts[-1]
                d = norm(p[0] - q[0], p[1] - q[1])
                if d == 0.0:
                    continue
                s += d
            self.pts.append(p)
            self.at.append(s)

    def length(self):
        return self.at[-1] if self.at else 0.0

    def segment(self, s):
        n = len(self.pts)
        if n < 2:
            return 0
        return min(max(bisect_right(self.at, s), 1), n - 1) - 1

    def point(self, s):
        n = len(self.pts)
        if n == 0:
            return (0.0, 0.0)
        if n == 1:
            return self.pts[0]
        i = self.segment(s)
        a, b = self.pts[i], self.pts[i + 1]
        t = min(max((s - self.at[i]) / (self.at[i + 1] - self.at[i]), 0.0), 1.0)
        return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)

    def angle(self, s):
        if len(self.pts) < 2:
            return 0.0
        i = self.segment(s)
        a, b = self.pts[i], self.pts[i + 1]
        return math.atan2(b[1] - a[1], b[0] - a[0])

    def between(self, s0, s1):
        return [p for p, s in zip(self.pts, self.at) if s0 < s < s1]

    def reversed(self):
        total = self.length()
        return Walk(self.pts[::-1], [total - s for s in self.at[::-1]])


def polylabel_centroid(ring):
    area = x = y = 0.0
    n = len(ring)
    j = n - 1
    for i in range(n):
        a, b = ring[i], ring[j]
        f = a[0] * b[1] - b[0] * a[1]
        x += (a[0] + b[0]) * f
        y += (a[1] + b[1]) * f
        area += f * 3.0
        j = i
    return ring[0] if area == 0.0 else (x / area, y / area)


def pole(rings, precision):
    """The point farthest inside from the edges (Mapbox's polylabel): a queue of cells by the most they can hold,
    of equal ones the first made."""
    outer = rings[0] if rings else []
    if len(outer) < 3:
        return None
    x0, y0 = min(p[0] for p in outer), min(p[1] for p in outer)
    x1, y1 = max(p[0] for p in outer), max(p[1] for p in outer)
    w, h = x1 - x0, y1 - y0
    size = min(w, h)
    if not size > 0.0:
        return None
    seq = 0
    queue = []

    def make(c, half):
        nonlocal seq
        seq += 1
        d = signed_distance(c, rings)
        return (d + half * math.sqrt(2.0), seq, c, half, d)

    def push(cell):
        heapq.heappush(queue, (-cell[0], cell[1], cell))

    half = size / 2.0
    x = x0
    while x < x1:
        y = y0
        while y < y1:
            push(make((x + half, y + half), half))
            y += size
        x += size
    best = make(polylabel_centroid(outer), 0.0)
    middle = make((x0 + w / 2.0, y0 + h / 2.0), 0.0)
    if middle[4] > best[4]:
        best = middle
    while queue:
        cell = heapq.heappop(queue)[2]
        if cell[4] > best[4]:
            best = cell
        if cell[0] - best[4] <= precision:
            continue
        hh = cell[3] / 2.0
        for sx, sy in ((-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)):
            push(make((cell[2][0] + sx * hh, cell[2][1] + sy * hh), hh))
    return (best[2], best[4]) if best[4] > 0.0 else None


def hull(pts):
    p = sorted(set(pts))
    if len(p) < 3:
        return p

    def cross(o, a, b):
        return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])

    lower, upper = [], []
    for q in p:
        while len(lower) >= 2 and cross(lower[-2], lower[-1], q) <= 0.0:
            lower.pop()
        lower.append(q)
    for q in reversed(p):
        while len(upper) >= 2 and cross(upper[-2], upper[-1], q) <= 0.0:
            upper.pop()
        upper.append(q)
    return lower[:-1] + upper[:-1]


def long_axis(pts):
    """The long side's direction of the smallest rectangle round the points (a side on a hull edge; of equal areas
    the first edge)."""
    h = hull(pts)
    if len(h) < 3:
        return None
    best = None
    for i in range(len(h)):
        a, b = h[i], h[(i + 1) % len(h)]
        ln = norm(b[0] - a[0], b[1] - a[1])
        if ln == 0.0:
            continue
        u = ((b[0] - a[0]) / ln, (b[1] - a[1]) / ln)
        v = (-u[1], u[0])
        ss = [dot((q[0] - a[0], q[1] - a[1]), u) for q in h]
        ts = [dot((q[0] - a[0], q[1] - a[1]), v) for q in h]
        s0, s1, t0, t1 = min(ss), max(ss), min(ts), max(ts)
        area = (s1 - s0) * (t1 - t0)
        if best is None or area < best[0]:
            angle = math.atan2(u[1], u[0]) if s1 - s0 >= t1 - t0 else math.atan2(v[1], v[0])
            best = (area, angle)
    return None if best is None else best[1]


def upright(a):
    while a > math.pi / 2:
        a -= math.pi
    while a <= -math.pi / 2:
        a += math.pi
    return a


def wrap(a):
    while a > math.pi:
        a -= 2.0 * math.pi
    while a <= -math.pi:
        a += 2.0 * math.pi
    return a


def edge_toward(c, hw, hh, to):
    """Where the line from a level box's middle toward `to` leaves it, 2 px out."""
    dx, dy = to[0] - c[0], to[1] - c[1]
    t = min((hw + 2.0) / abs(dx) if dx != 0.0 else math.inf, (hh + 2.0) / abs(dy) if dy != 0.0 else math.inf)
    if not math.isfinite(t) or t >= 1.0:
        return c
    return (c[0] + dx * t, c[1] + dy * t)


# ── The engine (§3.2–§3.8) ──────────────────────────────────────────────

CELL = 32.0
MARGIN = 256.0
IMPROVE_TRIES = 32
PATH_CANDIDATES = 21
AREA_STEPS = 8
OVERRUN = 0.5
OUTSIDE = 1.0
SYMBOL = 0.05
PER_WEIGHT = 0.05

PINNED, UNPLACED, HIDDEN, OVERLAPPING, CURVED, OUTSIDE_AREA, MASKED = 1, 2, 4, 8, 16, 32, 64
FIXED = -1


class Form:
    def __init__(self, lines, font, bold, size, outside):
        self.lines = lines
        self.widths = [line_width(l, font, bold, size) for l in lines]
        self.size = size
        self.w = max([0.0] + self.widths)
        self.h = size + (max(len(lines), 1) - 1) * LINE_HEIGHT * size
        self.outside = outside


class Cand:
    def __init__(self, cost, boxes, drawn, callout=None, outside=False):
        self.cost, self.boxes, self.drawn, self.callout, self.outside = cost, boxes, drawn, callout, outside


def forms(u, font, size):
    """The forms a label is tried in (§3.4): as written, stacked, shortened (and stacked), smaller, outside."""
    c = u["cls"]
    base = abbreviated(u["text"], c.abbreviate) if c.abbreviate and c.abbreviate["always"] else u["text"]

    def lay(t):
        return stacked(t, c.stack) if c.stack and c.stack["always"] else lines_of(t)

    texts = [lay(base)]

    def push(l):
        if l not in texts:
            texts.append(l)

    if c.stack and not c.stack["always"]:
        push(stacked(base, c.stack))
    if c.abbreviate and not c.abbreviate["always"]:
        short = abbreviated(base, c.abbreviate)
        push(lay(short))
        if c.stack and not c.stack["always"]:
            push(stacked(short, c.stack))
    out = [Form(l, font, c.bold, size, False) for l in texts]
    if c.shrink < 1.0:
        k = 1
        while True:
            f = 1.0 - 0.1 * k
            if f < c.shrink - 1e-9 or f <= 0.0:
                break
            out.append(Form(texts[-1], font, c.bold, size * f, False))
            k += 1
    if c.outside and u["geo"][0] == "area":
        out.append(Form(texts[0], font, c.bold, size, True))
    return out


def straight(f, u, c, angle, cost):
    cls = u["cls"]
    pad = cls.pad()
    lines = []
    for i, (l, w) in enumerate(zip(f.lines, f.widths)):
        dy = (f.h - f.size) / 2.0 - i * LINE_HEIGHT * f.size
        dx = {"left": -(f.w - w) / 2.0, "right": (f.w - w) / 2.0}.get(cls.align, 0.0)
        lines.append(((dx, dy), l))
    return Cand(cost, [Obb(c, angle, f.w / 2.0 + pad, f.h / 2.0 + pad)], ("straight", c, angle, lines))


def middles(s0, s1, w):
    """The middles along a stretch: its middle, then either side a step at a time (at most 21)."""
    ln = s1 - s0
    if ln < w:
        return []
    mid = (s0 + s1) / 2.0
    room = (ln - w) / 2.0
    step = max(max(w / 4.0, (ln - w) / 20.0), 2.0)
    out, k = [mid], 1.0
    while len(out) < PATH_CANDIDATES and k * step <= room + 1e-9:
        out.append(mid + k * step)
        if len(out) < PATH_CANDIDATES:
            out.append(mid - k * step)
        k += 1.0
    return out


SIDES = {"on": [0.0], "above": [1.0], "below": [-1.0], "sides": [1.0, -1.0]}


def path_cands(f, u, font, walk, s0, s1, interior, out):
    c = u["cls"]
    ln, mid, pad = s1 - s0, (s0 + s1) / 2.0, c.pad()
    mode = ("curved" if c.curved else "parallel") if interior else c.line
    curved = mode in ("curved", "contour") and len(f.lines) == 1

    def place(t):
        return 0.0001 + 0.001 * abs(t - mid) / ln

    if mode == "horizontal":
        for t in middles(s0, s1, f.w):
            out.append(straight(f, u, walk.point(t), 0.0, place(t)))
    elif not curved:
        for t in middles(s0, s1, f.w):
            a, b = t - f.w / 2.0, t + f.w / 2.0
            p0, p1 = walk.point(a), walk.point(b)
            q = norm(p1[0] - p0[0], p1[1] - p0[1]) / f.w
            if not q >= 0.8:
                continue
            e2 = 0.0
            for p in walk.between(a, b):
                e2 = max(e2, segment_distance2(p, p0, p1))
            e = math.sqrt(e2)
            if e > f.size / 2.0:
                continue
            along = math.atan2(p1[1] - p0[1], p1[0] - p0[0])
            theta = upright(along)
            m = ((p0[0] + p1[0]) / 2.0, (p0[1] + p1[1]) / 2.0)
            n = (-math.sin(along), math.cos(along)) if interior else (-math.sin(theta), math.cos(theta))
            base = place(t) + 0.01 * (1.0 - q) + 0.001 * e / f.size
            for side in SIDES[c.position]:
                off = 0.0 if side == 0.0 else side * (c.distance + f.h / 2.0 + e)
                cc = (m[0] + n[0] * off, m[1] + n[1] * off)
                out.append(straight(f, u, cc, theta, base + (0.0005 if side < 0.0 else 0.0)))
    else:
        text = f.lines[0]
        adv = [letter_width(ch, font, c.bold, f.size) for ch in text]
        total = walk.length()

        def letters_on(w, start):
            s, got = start, []
            for a in adv:
                at = s + a / 2.0
                s += a
                got.append((w.point(at), w.angle(at)))
            return got

        for t in middles(s0, s1, f.w):
            fwd = letters_on(walk, t - f.w / 2.0)
            if mode == "contour" and u["uphill"] is not None:
                rev = not u["uphill"]
            else:
                rev = sum(w * math.cos(a) for (_, a), w in zip(fwd, adv)) < 0.0
            letters = letters_on(walk.reversed(), total - t - f.w / 2.0) if rev else fwd
            turn, ok = 0.0, True
            for (_, a0), (_, a1) in zip(letters, letters[1:]):
                d = abs(wrap(a1 - a0))
                if d > c.max_angle:
                    ok = False
                    break
                turn += d
            if not ok:
                continue
            mean = turn / (len(letters) - 1) if len(letters) > 1 else 0.0
            base = place(t) + 0.01 * mean / math.pi
            for side in SIDES["on" if mode == "contour" else c.position]:
                off = side * (c.distance + f.size / 2.0)
                boxes, drawn = [], []
                for i, ((p, a), w) in enumerate(zip(letters, adv)):
                    # An outline's inside is the path's left whichever way the letters read.
                    n = (math.sin(a), -math.cos(a)) if interior and rev else (-math.sin(a), math.cos(a))
                    q = (p[0] + n[0] * off, p[1] + n[1] * off)
                    boxes.append(Obb(q, a, w / 2.0 + pad, f.size / 2.0 + pad))
                    drawn.append((q, a, i))
                out.append(Cand(base + (0.0005 if side < 0.0 else 0.0), boxes, ("curved", text, drawn)))


def area_cands(f, u, rings, pole_, bbox, free, out):
    """A grid round the pole, a quarter of the label's width by half its height a step, 3 to 8 steps either way as
    far as the area's box goes; only the pole when the label is bigger than the box (level)."""
    c = u["cls"]
    sx, sy = max(f.w / 4.0, 2.0), max(f.h / 2.0, 2.0)
    x0, y0, x1, y1 = bbox
    roomy = f.w + 2.0 * c.halo <= x1 - x0 and f.h + 2.0 * c.halo <= y1 - y0

    def reach(near, far, step):
        n = math.ceil(max(near, far) / step)
        return min(max(int(n), 3), AREA_STEPS)

    if roomy or free:
        ni, nj = reach(pole_[0] - x0, x1 - pole_[0], sx), reach(pole_[1] - y0, y1 - pole_[1], sy)
    else:
        ni = nj = 0
    grid = sorted(((i, j) for i in range(-ni, ni + 1) for j in range(-nj, nj + 1)), key=lambda g: (g[0] ** 2 + g[1] ** 2, g[0], g[1]))

    def add(angle, extra):
        ux, uy = math.cos(angle), math.sin(angle)
        for i, j in grid:
            dx, dy = i * sx, j * sy
            cc = (pole_[0] + dx * ux - dy * uy, pole_[1] + dx * uy + dy * ux)
            fits = box_inside(Obb(cc, angle, f.w / 2.0 + c.halo, f.h / 2.0 + c.halo), rings)
            if not fits and c.inside:
                continue
            cost = 0.0001 + 0.0001 * (i * i + j * j) + extra + (0.0 if fits else OVERRUN)
            out.append(straight(f, u, cc, angle, cost))

    add(0.0, 0.0)
    if free:
        a = long_axis(rings[0])
        if a is not None:
            a = upright(a)
            if abs(a) > 1e-3:
                add(a, 0.0005)


def outside_cands(f, u, bbox, pole_, out):
    d = u["cls"].distance
    k = d / math.sqrt(2.0)
    x0, y0, x1, y1 = bbox
    xm, ym = (x0 + x1) / 2.0, (y0 + y1) / 2.0
    hw, hh = f.w / 2.0, f.h / 2.0
    spots = [(x1 + d + hw, ym), (x0 - d - hw, ym), (xm, y1 + d + hh), (xm, y0 - d - hh),
             (x1 + k + hw, y1 + k + hh), (x0 - k - hw, y1 + k + hh), (x1 + k + hw, y0 - k - hh), (x0 - k - hw, y0 - k - hh)]
    for i, cc in enumerate(spots):
        cand = straight(f, u, cc, 0.0, OUTSIDE + 0.001 * i)
        cand.outside = True
        cand.callout = (edge_toward(cc, hw, hh, pole_), pole_)
        out.append(cand)


def point_cands(f, u, p, r, out):
    """Around a point in QGIS's cartographic order: top right, top left, bottom right, bottom left, right, left,
    top, bottom."""
    c = u["cls"]
    hw, hh = f.w / 2.0, f.h / 2.0
    if c.point == "center":
        out.append(straight(f, u, p, 0.0, 0.0001))
    elif c.point == "around":
        dd = r + c.distance
        k = dd / math.sqrt(2.0)
        spots = [(k + hw, k + hh), (-k - hw, k + hh), (k + hw, -k - hh), (-k - hw, -k - hh),
                 (dd + hw, 0.0), (-dd - hw, 0.0), (f.w / 4.0, dd + hh), (-f.w / 4.0, -dd - hh)]
        for i, d in enumerate(spots):
            out.append(straight(f, u, (p[0] + d[0], p[1] + d[1]), 0.0, 0.0001 + 0.001 * i))


def corner(f, u, tl):
    return straight(f, u, (tl[0] + 8.0 + f.w / 2.0, tl[1] - 14.0 - (f.h - f.size) / 2.0), 0.0, 0.0001)


def cands(f, u, font):
    """A unit's candidates in a form, cheapest first, of equal costs as made."""
    out = []
    if u["pin"] is not None:
        return [straight(f, u, u["pin"][0], u["pin"][1], 0.0)]
    geo = u["geo"]
    if geo[0] == "point":
        if u["cls"].point == "corner":
            out.append(corner(f, u, geo[1]))
        else:
            point_cands(f, u, geo[1], geo[2], out)
    elif geo[0] == "corner":
        out.append(corner(f, u, geo[1]))
    elif geo[0] == "path":
        path_cands(f, u, font, geo[1], geo[2], geo[3], geo[4], out)
    else:
        _, rings, pole_, bbox = geo
        if f.outside:
            outside_cands(f, u, bbox, pole_, out)
        else:
            area_cands(f, u, rings, pole_, bbox, u["cls"].area in ("free", "parcel"), out)
    order = sorted(range(len(out)), key=lambda i: (out[i].cost, i))
    return [out[i] for i in order]


class Grid:
    """32 px cells over the window and 256 px round it."""

    def __init__(self, width, height):
        self.x0 = self.y0 = -MARGIN
        self.cols = int(min(max(math.ceil((width + 2 * MARGIN) / CELL), 1), 4096))
        self.rows = int(min(max(math.ceil((height + 2 * MARGIN) / CELL), 1), 4096))
        self.cells = [[] for _ in range(self.cols * self.rows)]

    def col(self, x):
        return int(min(max(math.floor((x - self.x0) / CELL), 0), self.cols - 1))

    def row(self, y):
        return int(min(max(math.floor((y - self.y0) / CELL), 0), self.rows - 1))

    def span(self, box):
        x0, y0, x1, y1 = box
        return [r * self.cols + c for r in range(self.row(y0), self.row(y1) + 1) for c in range(self.col(x0), self.col(x1) + 1)]


class Room(Grid):
    """The boxes placed labels and the drawing's texts take."""

    def __init__(self, width, height):
        super().__init__(width, height)
        self.boxes, self.alive = [], []

    def insert(self, boxes, owner):
        for b in boxes:
            k = len(self.boxes)
            self.boxes.append((b, owner))
            self.alive.append(True)
            for cell in self.span(b.aabb()):
                self.cells[cell].append(k)

    def remove(self, owner):
        for k, (_, o) in enumerate(self.boxes):
            if o == owner:
                self.alive[k] = False

    def blockers(self, boxes):
        out = []
        for b in boxes:
            for cell in self.span(b.aabb()):
                for k in self.cells[cell]:
                    o, owner = self.boxes[k]
                    if self.alive[k] and owner not in out and b.overlaps(o):
                        out.append(owner)
        return out

    def free(self, boxes):
        return not any(self.alive[k] and b.overlaps(self.boxes[k][0]) for b in boxes for cell in self.span(b.aabb())
                       for k in self.cells[cell])


class Obstacles(Grid):
    def __init__(self, width, height, obstacles):
        super().__init__(width, height)
        self.list = obstacles
        for k, ob in enumerate(obstacles):
            shape = ob["shape"]
            if shape[0] == "circle":
                (x, y), r = shape[1], shape[2]
                box = (x - r, y - r, x + r, y + r)
            elif shape[0] == "segment":
                a, b = shape[1], shape[2]
                box = (min(a[0], b[0]), min(a[1], b[1]), max(a[0], b[0]), max(a[1], b[1]))
            else:
                box = shape[2]
            if box[2] < self.x0 or box[3] < self.y0:
                continue
            for cell in self.span(box):
                self.cells[cell].append(k)

    def cost(self, boxes, own, priority):
        """What the boxes cost among the obstacles; None when one they may not cover is under them: a point symbol
        0.05, an obstacle layer's object 0.05 a weight once, none when its weight is over the label's priority."""
        seen, hit, cost = [], [], 0.0
        for b in boxes:
            for cell in self.span(b.aabb()):
                for k in self.cells[cell]:
                    ob = self.list[k]
                    if ob["owner"] == own or k in seen:
                        continue
                    s = ob["shape"]
                    covers = (b.hits_circle(s[1], s[2]) if s[0] == "circle" else b.hits_segment(s[1], s[2])
                              if s[0] == "segment" else inside_rings(b.c, s[1]))
                    if not covers:
                        continue
                    seen.append(k)
                    if ob["weight"] is None:
                        cost += SYMBOL
                    else:
                        if ob["owner"] in hit:
                            continue
                        if priority < ob["weight"]:
                            return None
                        hit.append(ob["owner"])
                        cost += PER_WEIGHT * ob["weight"]
        return cost


def middle_of(c):
    if c.drawn[0] == "straight":
        return c.drawn[1]
    letters = c.drawn[2]
    n = max(len(letters), 1)
    return (sum(p[0] for p, _, _ in letters) / n, sum(p[1] for p, _, _ in letters) / n)


def place(scene):
    """Pinned first; then by priority, the layer's place, the class, the object's place and the piece; each the
    first free candidate of its forms; one label moved for another; those that may cover another; the rest unplaced."""
    units, font = scene["units"], scene["font"]
    room = Room(scene["width"], scene["height"])
    room.insert(scene["fixed"], FIXED)
    obstacles = Obstacles(scene["width"], scene["height"], scene["obstacles"])
    order = sorted((i for i, u in enumerate(units) if not u["hidden"]),
                   key=lambda i: (units[i]["pin"] is None, -units[i]["cls"].priority, units[i]["rank"], units[i]["class"],
                                  units[i]["order"], units[i]["chunk"]))
    forms_of = [None] * len(units)
    placed_at = [None] * len(units)
    forced = [False] * len(units)
    placed = {}
    texts = {}

    def valid(u, f):
        got = []
        for i, c in enumerate(cands(f, u, font)):
            if u["pin"] is None:
                extra = obstacles.cost(c.boxes, u["id"], u["cls"].priority)
                if extra is None:
                    continue
                c.cost += extra
            got.append((c.cost, i, c))
        got.sort(key=lambda g: (g[0], g[1]))
        return [g[2] for g in got]

    def duplicate(u, c, me):
        d = u["cls"].duplicates
        if d is None:
            return False
        m = middle_of(c)
        return any(owner != me and norm(p[0] - m[0], p[1] - m[1]) < d for p, owner in texts.get(u["text"], []))

    def take(i, c, where):
        room.insert(c.boxes, i)
        texts.setdefault(units[i]["text"], []).append((middle_of(c), i))
        placed[i] = c
        placed_at[i] = where

    def give_back(i):
        room.remove(i)
        if units[i]["text"] in texts:
            texts[units[i]["text"]] = [e for e in texts[units[i]["text"]] if e[1] != i]

    for i in order:
        u = units[i]
        if forms_of[i] is None:
            forms_of[i] = forms(u, font, u["cls"].size_at(scene["scale"]))
        done = False
        for fi, f in enumerate(forms_of[i]):
            got = valid(u, f)
            if u["pin"] is not None or u["cls"].overlap == "always":
                if got:
                    take(i, got[0], (fi, 0))
                    forced[i] = u["pin"] is None
                    break
                continue
            for ci, c in enumerate(got):
                if room.free(c.boxes) and not duplicate(u, c, i):
                    take(i, c, (fi, ci))
                    done = True
                    break
            if done:
                break

    # One step of an ejection chain: an unplaced label takes the place of one movable label that moves elsewhere.
    for i in order:
        u = units[i]
        if placed_at[i] is not None or u["pin"] is not None or not forms_of[i]:
            continue
        for tries, c in enumerate(valid(u, forms_of[i][0])):
            if tries >= IMPROVE_TRIES:
                break
            b = room.blockers(c.boxes)
            if len(b) != 1 or b[0] == FIXED:
                continue
            v = b[0]
            other = units[v]
            if other["pin"] is not None or forced[v] or other["cls"].priority > u["cls"].priority or placed_at[v] is None:
                continue
            vf, vc = placed_at[v]
            old = placed[v]
            give_back(v)
            if duplicate(u, c, i):
                take(v, old, (vf, vc))
                continue
            take(i, c, (0, 0))
            moved = False
            for k, cv in enumerate(valid(other, forms_of[v][vf])):
                if k == vc or not room.free(cv.boxes) or duplicate(other, cv, v):
                    continue
                take(v, cv, (vf, k))
                moved = True
                break
            if moved:
                break
            give_back(i)
            del placed[i]
            placed_at[i] = None
            take(v, old, (vf, vc))

    # Those that may cover another, at their first form's best place.
    for i in order:
        u = units[i]
        if placed_at[i] is not None or u["cls"].overlap != "ifNeeded" or not forms_of[i]:
            continue
        got = valid(u, forms_of[i][0])
        if got:
            room.insert(got[0].boxes, i)
            placed[i] = got[0]
            placed_at[i] = (0, 0)
            forced[i] = True

    labels = []
    for i in order:
        if i not in placed:
            continue
        u = units[i]
        state = MASKED if u["cls"].mask else 0
        if u["pin"] is not None:
            state |= PINNED
        if forced[i] and u["cls"].overlap == "ifNeeded":
            state |= OVERLAPPING
        labels.append(label_of(u, forms_of[i][placed_at[i][0]], placed[i], state))
    if scene["unplaced"]:
        for i in order:
            u = units[i]
            if placed_at[i] is None and forms_of[i]:
                got = cands(forms_of[i][0], u, font)
                if got:
                    labels.append(label_of(u, forms_of[i][0], got[0], UNPLACED | (MASKED if u["cls"].mask else 0)))
    if scene["hidden"]:
        for u in units:
            if u["hidden"]:
                fs = forms(u, font, u["cls"].size_at(scene["scale"]))
                got = cands(fs[0], u, font) if fs else []
                if got:
                    labels.append(label_of(u, fs[0], got[0], HIDDEN | (MASKED if u["cls"].mask else 0)))
    return labels


def label_of(u, f, c, state):
    if c.drawn[0] == "curved":
        state |= CURVED
    if c.outside:
        state |= OUTSIDE_AREA
    callout = c.callout
    if callout is None and u["cls"].callout is not None and u["target"] is not None and c.drawn[0] == "straight":
        # From the box's edge, in its frame, toward the object.
        m, angle = c.drawn[1], c.drawn[2]
        ux, uy = math.cos(angle), math.sin(angle)
        t = u["target"]
        dx, dy = t[0] - m[0], t[1] - m[1]
        e = edge_toward((0.0, 0.0), f.w / 2.0, f.h / 2.0, (dx * ux + dy * uy, -dx * uy + dy * ux))
        fr = (m[0] + e[0] * ux - e[1] * uy, m[1] + e[0] * uy + e[1] * ux)
        if norm(t[0] - fr[0], t[1] - fr[1]) >= u["cls"].callout and not c.boxes[0].contains(t):
            callout = (fr, t)
    return {"unit": u, "form": f, "cand": c, "state": state, "callout": callout}


# ── The store: a window's units, obstacles and pins (§3.1, §3.3, §3.5, §3.7) ──

PROBE = 6.0
SYMBOL_MIN = 2.0


def xy(p):
    return (float(p["x"]), float(p["y"]))


def kind_of(e):
    return {"point": "point", "insert": "point", "line": "line", "polyline": "line", "arc": "line",
            "polygon": "area", "circle": "area", "hatch": "area"}.get(e["kind"])


def default_slot(e):
    return {"polygon": "polygon", "circle": "circle", "point": "point", "polyline": "polyline", "line": "line"}.get(e["kind"])


def outline(e):
    """A line's points (world)."""
    if e["kind"] == "line":
        return [xy(e["a"]), xy(e["b"])]
    return [xy(p) for p in e["pts"]]


def rings_of(e):
    return [[xy(p) for p in e["pts"]]] + [[xy(p) for p in h["pts"]] for h in e.get("holes", [])]


def bounds(e):
    pts = [xy(e["p"])] if "p" in e else outline(e) if e["kind"] in ("line", "polyline") else rings_of(e)[0]
    return (min(p[0] for p in pts), min(p[1] for p in pts), max(p[0] for p in pts), max(p[1] for p in pts))


def area_centroid(pts):
    """The area's centroid, taken relative to the first point."""
    o = pts[0]
    n = len(pts)
    a = 0.0
    for i in range(n):
        p, q = pts[i], pts[(i + 1) % n]
        a += (p[0] - o[0]) * (q[1] - o[1]) - (q[0] - o[0]) * (p[1] - o[1])
    a /= 2.0
    if abs(a) < 1e-9:
        return (o[0] + sum(p[0] - o[0] for p in pts) / n, o[1] + sum(p[1] - o[1] for p in pts) / n)
    cx = cy = 0.0
    j = n - 1
    for i in range(n):
        xj, yj = pts[j][0] - o[0], pts[j][1] - o[1]
        xi, yi = pts[i][0] - o[0], pts[i][1] - o[1]
        f = xj * yi - xi * yj
        cx += (xj + xi) * f
        cy += (yj + yi) * f
        j = i
    return (o[0] + cx / (6.0 * a), o[1] + cy / (6.0 * a))


def anchor(e):
    """Where an object's label sits when it is not placed (the pins' origin)."""
    k = e["kind"]
    if k in ("point", "insert"):
        return xy(e["p"])
    if k == "line":
        a, b = xy(e["a"]), xy(e["b"])
        return ((a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0)
    if k == "polyline":
        return xy(e["pts"][len(e["pts"]) // 2])
    return area_centroid(rings_of(e)[0])


def signed_area(ring):
    a = 0.0
    for i in range(len(ring)):
        p, q = ring[i], ring[(i + 1) % len(ring)]
        a += p[0] * q[1] - q[0] * p[1]
    return a / 2.0


def merged(pieces):
    """Pieces meeting end to end (exactly), chained while two meet at a point; each chain started by its piece first
    in the document, on from its end, then back from its start."""
    ends = {}
    for k, (_, pts) in enumerate(pieces):
        for p in (pts[0], pts[-1]):
            ends.setdefault(p, []).append(k)
    used = [False] * len(pieces)
    out = []
    for k, (first, pts) in enumerate(pieces):
        if used[k] or len(pts) < 2:
            continue
        used[k] = True
        chain = list(pts)
        for forward in (True, False):
            while True:
                at = chain[-1] if forward else chain[0]
                meeting = ends[at]
                if len(meeting) != 2:
                    break
                nxt = next((m for m in meeting if not used[m]), None)
                if nxt is None:
                    break
                used[nxt] = True
                more = list(pieces[nxt][1])
                if forward:
                    if more[0] != at:
                        more.reverse()
                    chain += more[1:]
                else:
                    if more[-1] != at:
                        more.reverse()
                    chain = more[:-1] + chain
        out.append((first, chain))
    return out


def crossing(p, d, ln, segments, own):
    """The height of the nearest other contour the ray from p along d crosses within ln."""
    q = (p[0] + d[0] * ln, p[1] + d[1] * ln)
    best = None
    for a, b, z, oid in segments:
        if oid == own:
            continue
        rx, ry = q[0] - p[0], q[1] - p[1]
        sx, sy = b[0] - a[0], b[1] - a[1]
        den = rx * sy - ry * sx
        if den == 0.0:
            continue
        t = ((a[0] - p[0]) * sy - (a[1] - p[1]) * sx) / den
        uu = ((a[0] - p[0]) * ry - (a[1] - p[1]) * rx) / den
        if 0.0 < t <= 1.0 and 0.0 <= uu <= 1.0 and (best is None or t < best[0]):
            best = (t, z)
    return None if best is None else best[1]


def uphill(walk, z, own, probe, segments):
    """Whether a contour reads along its path (its labels' tops uphill): at the first of its probes where a
    neighbour says; None when none does."""
    ln = walk.length()
    for frac in (0.5, 0.25, 0.75, 0.125, 0.375, 0.625, 0.875):
        s = ln * frac
        p, a = walk.point(s), walk.angle(s)
        n = (-math.sin(a), math.cos(a))
        plus = crossing(p, n, probe, segments, own)
        minus = crossing(p, (-n[0], -n[1]), probe, segments, own)
        says = None
        if plus is not None and minus is not None:
            if plus != minus:
                says = plus > minus
        elif plus is not None:
            if plus != z:
                says = plus > z
        elif minus is not None:
            if minus != z:
                says = minus < z
        if says is not None:
            return says
    return None


def inside_stretches(walk, rect):
    out = []
    for i in range(len(walk.pts) - 1):
        a, b = walk.pts[i], walk.pts[i + 1]
        ln = walk.at[i + 1] - walk.at[i]
        for piece in clip_path([a, b], rect):
            s, e = piece[0], piece[-1]
            fr = walk.at[i] + norm(s[0] - a[0], s[1] - a[1]) / ln * ln
            to = walk.at[i] + norm(e[0] - a[0], e[1] - a[1]) / ln * ln
            if out and fr <= out[-1][1] + 1e-9:
                out[-1] = (out[-1][0], max(out[-1][1], to))
            else:
                out.append((fr, to))
    return out


def path_units(px, cls, rect, interior, single, unit, out):
    """A path's repeated pieces (those the window shows), or its longest stretch in the window."""
    repeat = cls.outline_repeat() if interior else cls.repeat
    if repeat is not None and repeat > 0.0 and not single:
        walk = Walk(px)
        ln = walk.length()
        chunks = set()
        for fr, to in inside_stretches(walk, rect):
            chunks.update(range(int(math.floor(fr / repeat)), int(math.floor(to / repeat)) + 1))
        for k in sorted(chunks):
            s0 = k * repeat
            s1 = min(s0 + repeat, ln)
            if s1 > s0:
                out.append(unit(("path", walk, s0, s1, interior), k, walk.point((s0 + s1) / 2.0)))
    else:
        best = None
        for piece in clip_path(px, rect):
            w = Walk(piece)
            if best is None or w.length() > best.length():
                best = w
        if best is not None:
            out.append(unit(("path", best, 0.0, best.length(), interior), 0, best.point(best.length() / 2.0)))


def place_window(case):
    """What the store places in the case's window at its scale."""
    font = case.get("font", "barlow")
    x0, y0, x1, y1 = case["window"]
    s = case["scale"]

    def px(p):
        return ((p[0] - x0) * s, (p[1] - y0) * s)

    width, height = (x1 - x0) * s, (y1 - y0) * s
    rect = (-MARGIN, -MARGIN, width + MARGIN, height + MARGIN)
    layers = {}
    for row in case["layers"]:
        layers[row["id"]] = {"rank": row["rank"], "point": float(row.get("point", 0.0)), "labelling": labelling(row)}
    defaults = {k: Class(v, None) for k, v in case.get("defaults", {}).items()}
    texts = {t["id"]: t for t in case.get("texts", [])}
    pins = {pid: plist for pid, plist in case.get("pins", [])}
    objects = case["objects"]

    def layer_of(e):
        return layers.get(e["layerId"])

    def class_of(e, k):
        lay = layer_of(e)
        lab = lay and lay["labelling"]
        if lab is not None and lab["off"]:
            return None
        if lab is not None and lab["classes"]:
            return lab["classes"][k] if k < len(lab["classes"]) else None
        return defaults.get(default_slot(e))

    obstacles, contours = [], {}
    for e in objects:
        lay = layer_of(e)
        point_px = lay["point"] if lay else 0.0
        if e["kind"] == "point":
            obstacles.append({"owner": e["id"], "weight": None, "shape": ("circle", px(xy(e["p"])), max(point_px / 2.0, SYMBOL_MIN))})
        lab = lay and lay["labelling"]
        if lab is not None and lab["obstacle"] is not None:
            w, boundary = lab["obstacle"]
            k = kind_of(e)
            if k == "point":
                obstacles.append({"owner": e["id"], "weight": w, "shape": ("circle", px(xy(e["p"])), max(point_px / 2.0, SYMBOL_MIN))})
            elif k == "line":
                pts = [px(p) for p in outline(e)]
                for a, b in zip(pts, pts[1:]):
                    obstacles.append({"owner": e["id"], "weight": w, "shape": ("segment", a, b)})
            elif k == "area":
                rings = [[px(p) for p in r] for r in rings_of(e)]
                for r in rings:
                    for i in range(len(r)):
                        obstacles.append({"owner": e["id"], "weight": w, "shape": ("segment", r[i], r[(i + 1) % len(r)])})
                if not boundary:
                    every = [p for r in rings for p in r]
                    box = (min(p[0] for p in every), min(p[1] for p in every), max(p[0] for p in every), max(p[1] for p in every))
                    obstacles.append({"owner": e["id"], "weight": w, "shape": ("area", rings, box)})
        t = texts.get(e["id"])
        if t is not None and t.get("z") is not None and kind_of(e) == "line":
            pts = [px(p) for p in outline(e)]
            for a, b in zip(pts, pts[1:]):
                contours.setdefault(e["layerId"], []).append((a, b, float(t["z"]), e["id"]))

    units, to_merge = [], []
    for order, e in enumerate(objects):
        t = texts.get(e["id"])
        k = kind_of(e)
        if t is None or k is None:
            continue
        lay = layer_of(e)
        rank = lay["rank"] if lay else 2 ** 32 - 1
        point_px = lay["point"] if lay else 0.0
        b = bounds(e)
        for j, (ci, text) in enumerate(t["texts"]):
            cls = class_of(e, ci)
            if cls is None or not text or not cls.shown_at(s):
                continue
            if cls.min_feature_px is not None and min(b[2] - b[0], b[3] - b[1]) * s < cls.min_feature_px:
                continue
            pin = next((p for p in pins.get(e["id"], []) if p.get("class") == cls.name or ("class" not in p and j == 0)), None)
            hidden = pin is not None and pin.get("hidden", False)
            pinned = None
            if pin is not None and "at" in pin:
                a = anchor(e)
                pinned = (px((a[0] + pin["at"]["x"], a[1] + pin["at"]["y"])), math.radians(pin.get("rotation", 0.0)))

            def unit(geo, chunk, target, e=e, ci=ci, cls=cls, text=text, rank=rank, pinned=pinned, hidden=hidden, order=order):
                return {"id": e["id"], "order": order, "rank": rank, "class": ci, "cls": cls, "text": text, "chunk": chunk,
                        "geo": geo, "target": target, "pin": pinned, "hidden": hidden, "uphill": None}

            corner_geo = ("corner", px((b[0], b[3])))
            if k == "point":
                p = px(xy(e["p"]))
                if cls.point == "corner":
                    units.append(unit(corner_geo, 0, None))
                else:
                    units.append(unit(("point", p, point_px / 2.0 if e["kind"] == "point" else 0.0), 0, p))
            elif k == "line":
                if cls.line == "corner":
                    units.append(unit(corner_geo, 0, None))
                    continue
                world = outline(e)
                if cls.merge_lines and pinned is None and not hidden:
                    to_merge.append(((e["layerId"], ci, text), e, world, cls, order, rank))
                    continue
                pts = [px(p) for p in world]
                start = len(units)
                path_units(pts, cls, rect, False, pinned is not None or hidden, unit, units)
                if cls.line == "contour":
                    up = uphill(Walk(pts), float(t["z"]), e["id"], PROBE * cls.size_at(s), contours.get(e["layerId"], []))
                    for u in units[start:]:
                        u["uphill"] = up
            else:
                if cls.area == "corner":
                    units.append(unit(corner_geo, 0, None))
                    continue
                rings = [[px(p) for p in r] for r in rings_of(e)]
                if cls.area in ("perimeter", "boundary"):
                    ring = list(rings[0])
                    if signed_area(ring) < 0.0:
                        ring.reverse()
                    ring.append(ring[0])
                    path_units(ring, cls, rect, True, pinned is not None or hidden, unit, units)
                    continue
                clipped = [c for c in (clip_ring(r, rect) for r in rings) if len(c) >= 3]
                if not clipped:
                    continue
                got = pole(clipped, 1.0)
                if got is None:
                    continue
                box = (min(p[0] for p in clipped[0]), min(p[1] for p in clipped[0]),
                       max(p[0] for p in clipped[0]), max(p[1] for p in clipped[0]))
                units.append(unit(("area", clipped, got[0], box), 0, got[0]))
    # Merged lines: a chain is labelled as its first piece.
    groups = []
    for k, (key, e, world, cls, order, rank) in enumerate(to_merge):
        g = next((g for g in groups if g[0] == key), None)
        if g is None:
            groups.append((key, [(k, world)]))
        else:
            g[1].append((k, world))
    for key, pieces in groups:
        for first, chain in merged(pieces):
            _, e, _, cls, order, rank = to_merge[first]
            pts = [px(p) for p in chain]

            def unit(geo, chunk, target, e=e, key=key, cls=cls, order=order, rank=rank):
                return {"id": e["id"], "order": order, "rank": rank, "class": key[1], "cls": cls, "text": key[2], "chunk": chunk,
                        "geo": geo, "target": target, "pin": None, "hidden": False, "uphill": None}

            start = len(units)
            path_units(pts, cls, rect, False, False, unit, units)
            if cls.line == "contour":
                up = uphill(Walk(pts), float(texts[e["id"]]["z"]), e["id"], PROBE * cls.size_at(s), contours.get(e["layerId"], []))
                for u in units[start:]:
                    u["uphill"] = up
    fixed = []
    for o in case.get("fixed", []):
        pts = [px(tuple(p)) for p in o]
        a, b = pts[0], pts[1]
        ln = norm(b[0] - a[0], b[1] - a[1])
        u = ((b[0] - a[0]) / ln, (b[1] - a[1]) / ln)
        v = (-u[1], u[0])
        ss = [(p[0] - a[0]) * u[0] + (p[1] - a[1]) * u[1] for p in pts]
        ts = [(p[0] - a[0]) * v[0] + (p[1] - a[1]) * v[1] for p in pts]
        ms, mt = (min(ss) + max(ss)) / 2.0, (min(ts) + max(ts)) / 2.0
        fixed.append(Obb((a[0] + u[0] * ms + v[0] * mt, a[1] + u[1] * ms + v[1] * mt), 0.0,
                         (max(ss) - min(ss)) / 2.0, (max(ts) - min(ts)) / 2.0, u=u))
    options = case.get("options", {})
    labels = place({"units": units, "font": font, "width": width, "height": height, "scale": s, "obstacles": obstacles,
                    "fixed": fixed, "unplaced": options.get("unplaced", False), "hidden": options.get("hidden", False)})

    def world(p):
        return (x0 + p[0] / s, y0 + p[1] / s)

    out = []
    for l in labels:
        u, f, c = l["unit"], l["form"], l["cand"]
        rec = {"id": u["id"], "class": u["class"], "state": l["state"], "w": f.w, "h": f.h}
        if c.drawn[0] == "straight":
            _, m, angle, lines = c.drawn
            rec["at"], rec["angle"] = world(m), math.degrees(angle)
            ux, uy = math.cos(angle), math.sin(angle)
            rec["lines"] = []
            for (dx, dy), text in lines:
                p = world((m[0] + dx * ux - dy * uy, m[1] + dx * uy + dy * ux))
                rec["lines"].append([p[0], p[1], math.degrees(angle), f.size, text, line_width(text, font, u["cls"].bold, f.size)])
        else:
            _, text, letters = c.drawn
            rec["at"], rec["angle"] = world(middle_of(c)), 0.0
            rec["text"] = text
            rec["letters"] = [[*world(p), math.degrees(a), f.size, i, letter_width(text[i], font, u["cls"].bold, f.size)]
                              for p, a, i in letters]
        if l["callout"] is not None:
            rec["callout"] = [list(world(l["callout"][0])), list(world(l["callout"][1]))]
        out.append(rec)
    return out


def label_at(labels, scale, p, tol, every):
    """The topmost label under p (world) within tol px: a straight label by its frame, a curved one by its letters;
    unplaced and hidden ones only with `every`."""
    hit, pad = None, tol / scale
    for l in labels:
        if not every and l["state"] & (UNPLACED | HIDDEN):
            continue
        if l["state"] & CURVED:
            for x, y, a, size, _, adv in l["letters"]:
                if Obb((x, y), math.radians(a), adv / 2.0 / scale + pad, size / 2.0 / scale + pad).contains(p):
                    hit = l
        elif Obb(tuple(l["at"]), math.radians(l["angle"]), l["w"] / 2.0 / scale + pad, l["h"] / 2.0 / scale + pad).contains(p):
            hit = l
    return None if hit is None else {"id": hit["id"], "class": hit["class"], "state": hit["state"]}


# ── The cases ───────────────────────────────────────────────────────────


def P(x, y):
    return {"x": x, "y": y}


def point(id_, layer, x, y):
    return {"id": id_, "layerId": layer, "kind": "point", "p": P(x, y)}


def polyline(id_, layer, pts):
    return {"id": id_, "layerId": layer, "kind": "polyline", "pts": [P(x, y) for x, y in pts]}


def polygon(id_, layer, pts, holes=None):
    e = {"id": id_, "layerId": layer, "kind": "polygon", "pts": [P(x, y) for x, y in pts]}
    if holes:
        e["holes"] = [{"pts": [P(x, y) for x, y in h]} for h in holes]
    return e


def rect(id_, layer, x0, y0, x1, y1):
    return polygon(id_, layer, [(x0, y0), (x1, y0), (x1, y1), (x0, y1)])


def box(x0, y0, x1, y1):
    """A text's outline (world): a level box."""
    return [[x0, y0], [x1, y0], [x1, y1], [x0, y1]]


def turned(cx, cy, w, h, deg):
    a = math.radians(deg)
    u, v = (math.cos(a), math.sin(a)), (-math.sin(a), math.cos(a))
    return [(cx + u[0] * s * w / 2 + v[0] * t * h / 2, cy + u[1] * s * w / 2 + v[1] * t * h / 2)
            for s, t in ((-1, -1), (1, -1), (1, 1), (-1, 1))]


def labelled(*rows):
    return [{"id": i, "texts": t} for i, t in rows]


def one(i, text, z=None):
    t = {"id": i, "texts": [[0, text]]}
    if z is not None:
        t["z"] = z
    return t


NOKTA = {"placement": "beside", "size": 10, "point": "around"}


def cases():
    out = []

    def case(name, objects, layers, texts, window=(0, 0, 100, 60), scale=4.0, **more):
        out.append({"name": name, "window": list(window), "scale": scale, "layers": layers, "objects": objects,
                    "texts": texts, **more})

    # ── Points (§3.3: around in QGIS's order; on it) ──
    case("Noktanın adı sağ üstte", [point(1, "nokta", 50, 30)],
         [{"id": "nokta", "rank": 0, "point": 6, "label": NOKTA}], [one(1, "P.101")],
         hits=[{"at": [53.5, 32.0], "tol": 2}, {"at": [46.0, 32.0], "tol": 2}])
    case("Sağ üst bir yazıyla doluyken sol üst", [point(1, "nokta", 50, 30)],
         [{"id": "nokta", "rank": 0, "point": 6, "label": NOKTA}], [one(1, "P.101")], fixed=[box(51, 31, 58, 34)])
    case("Üstü doluyken sağ alt", [point(1, "nokta", 50, 30)],
         [{"id": "nokta", "rank": 0, "point": 6, "label": NOKTA}], [one(1, "P.101")], fixed=[box(30, 30.5, 70, 40)])
    case("Üstü ve sağ yanı doluyken sol alt", [point(1, "nokta", 50, 30)],
         [{"id": "nokta", "rank": 0, "point": 6, "label": NOKTA}], [one(1, "P.101")],
         fixed=[box(30, 30.5, 70, 40), box(50.5, 20, 70, 30.5)])
    case("Dört köşesi ve iki yanı doluyken altında", [point(1, "nokta", 50, 30)],
         [{"id": "nokta", "rank": 0, "point": 6, "label": NOKTA}], [one(1, "P.101")],
         fixed=[box(30, 30.5, 70, 40), box(52.75, 20, 70, 30.5), box(30, 20, 43.5, 30.5)])
    case("Üstünde: tek aday", [point(1, "nokta", 50, 30)],
         [{"id": "nokta", "rank": 0, "point": 6, "label": {"placement": "center", "size": 10}}], [one(1, "Kuyu")])
    case("Çok satırlı ve sola hizalı", [point(1, "nokta", 50, 30)],
         [{"id": "nokta", "rank": 0, "point": 6, "label": {**NOKTA, "align": "left"}}], [one(1, "Su deposu\nK-2")])
    case("Büyüyen boy en büyük boyda durur", [point(1, "nokta", 50, 30)],
         [{"id": "nokta", "rank": 0, "label": {**NOKTA, "grow": 2.0, "maxSize": 14}}], [one(1, "P.7")])
    case("Ölçek aralığının dışındaki sınıf yazılmaz", [point(1, "a", 30, 30), point(2, "b", 70, 30)],
         [{"id": "a", "rank": 0, "label": {**NOKTA, "minScale": 5.0}}, {"id": "b", "rank": 1, "label": {**NOKTA, "maxScale": 5.0}}],
         [one(1, "Görünmez"), one(2, "Görünür")])

    # ── Order and collisions (§3.5, §3.6) ──
    case("Yüksek öncelik önce yerleşir", [point(1, "dusuk", 50, 30), point(2, "yuksek", 53.5, 30.5)],
         [{"id": "dusuk", "rank": 0, "point": 6, "label": {**NOKTA, "priority": 2}},
          {"id": "yuksek", "rank": 1, "point": 6, "label": {**NOKTA, "priority": 8}}],
         [one(1, "Düşük"), one(2, "Yüksek")])
    case("Aynı öncelikte üstteki katman önce", [point(1, "alt", 50, 30), point(2, "ust", 53.5, 30.5)],
         [{"id": "ust", "rank": 0, "point": 6, "label": NOKTA}, {"id": "alt", "rank": 1, "point": 6, "label": NOKTA}],
         [one(1, "Alttaki"), one(2, "Üstteki")])
    case("Nokta sembolünü örten aday pahalıdır", [point(1, "nokta", 50, 30), point(2, "nokta", 54, 31.5)],
         [{"id": "nokta", "rank": 0, "point": 6, "label": NOKTA}], [one(1, "Ad")])
    case("Engel katmanı: ağırlık öncelikten büyükse geçilmez",
         [point(1, "nokta", 50, 30), rect(2, "bina", 30, 25, 70, 50)],
         [{"id": "nokta", "rank": 0, "point": 6, "label": NOKTA},
          {"id": "bina", "rank": 1, "labels": {"mode": "off", "obstacle": {"weight": 7}}}],
         [one(1, "İçeride")], options={"unplaced": True})
    case("Engel katmanı: ağırlık öncelikten küçükse maliyettir",
         [point(1, "nokta", 50, 30), rect(2, "bina", 30, 25, 70, 50)],
         [{"id": "nokta", "rank": 0, "point": 6, "label": NOKTA},
          {"id": "bina", "rank": 1, "labels": {"mode": "off", "obstacle": {"weight": 3}}}],
         [one(1, "İçeride")])
    case("Engel katmanının yalnız sınırı: içerisi serbest",
         [point(1, "nokta", 50, 30), rect(2, "bina", 30, 20, 70, 50)],
         [{"id": "nokta", "rank": 0, "point": 6, "label": NOKTA},
          {"id": "bina", "rank": 1, "labels": {"mode": "off", "obstacle": {"weight": 9, "kind": "boundary"}}}],
         [one(1, "İçeride")])
    case("Engel katmanının çizgisi: kesen aday düşer",
         [point(1, "nokta", 50, 30), polyline(2, "dere", [(40, 33), (60, 33)])],
         [{"id": "nokta", "rank": 0, "point": 6, "label": NOKTA},
          {"id": "dere", "rank": 1, "labels": {"mode": "off", "obstacle": {"weight": 8}}}],
         [one(1, "Köprü")])
    case("Yinelenen adlar yakınsa biri yazılır",
         [point(1, "nokta", 30, 30), point(2, "nokta", 36, 30), point(3, "nokta", 80, 30)],
         [{"id": "nokta", "rank": 0, "point": 6, "label": {**NOKTA, "duplicates": 100}}],
         [one(1, "Çeşme"), one(2, "Çeşme"), one(3, "Çeşme")], options={"unplaced": True})
    case("İyileştirme: yerleşen etiket başka yerine geçer, sığmayana yer açılır",
         [point(1, "nokta", 50, 30), point(2, "orta", 57, 32.5)],
         [{"id": "nokta", "rank": 0, "point": 6, "label": NOKTA},
          {"id": "orta", "rank": 1, "label": {"placement": "center", "size": 10}}],
         [one(1, "P.1"), one(2, "Vana")])
    case("Gerekirse çakışır", [point(1, "nokta", 50, 30), point(2, "ikinci", 50, 30)],
         [{"id": "nokta", "rank": 0, "point": 6, "label": NOKTA},
          {"id": "ikinci", "rank": 1, "label": {"placement": "center", "size": 10, "overlap": "ifNeeded"}}],
         [one(1, "Birinci"), one(2, "İkinci")], fixed=[box(30, 33, 70, 40), box(30, 20, 70, 27)])
    case("Her zaman en iyi yerinde", [point(1, "nokta", 50, 30), point(2, "ikinci", 51, 31)],
         [{"id": "nokta", "rank": 0, "point": 6, "label": {**NOKTA, "priority": 9}},
          {"id": "ikinci", "rank": 1, "label": {"placement": "center", "size": 10, "overlap": "always"}}],
         [one(1, "Birinci"), one(2, "İkinci")])

    # ── Lines (§3.3) ──
    case("Çizginin adı paralel, üstünde", [polyline(1, "yol", [(10, 10), (90, 50)])],
         [{"id": "yol", "rank": 0, "label": {"placement": "along", "size": 10, "position": "above"}}], [one(1, "Ankara Yolu")])
    case("Paralel: iki yanı da aday, alttaki biraz pahalı", [polyline(1, "yol", [(10, 30), (90, 30)])],
         [{"id": "yol", "rank": 0, "label": {"placement": "along", "size": 10, "position": "sides"}}], [one(1, "Dere Yolu")],
         fixed=[box(5, 30.2, 95, 40)])
    case("Kıvrık: harfler yolu izler",
         [polyline(1, "yol", [(50 + 40 * math.cos(math.radians(a)), -10 + 40 * math.sin(math.radians(a))) for a in range(150, 29, -10)])],
         [{"id": "yol", "rank": 0, "label": {"placement": "along", "size": 10, "line": "curved"}}], [one(1, "Çevre Yolu")],
         hits=[{"at": [50.0, 30.0], "tol": 2}])
    case("Kıvrık: sağdan sola çizilmiş yol okunur yürünür",
         [polyline(1, "yol", [(50 + 40 * math.cos(math.radians(a)), -10 + 40 * math.sin(math.radians(a))) for a in range(30, 151, 10)])],
         [{"id": "yol", "rank": 0, "label": {"placement": "along", "size": 10, "line": "curved"}}], [one(1, "Çevre Yolu")])
    case("Kıvrık: keskin dönemeçten kaçar",
         [polyline(1, "yol", [(5, 20), (45, 20), (50, 40), (95, 40)])],
         [{"id": "yol", "rank": 0, "label": {"placement": "along", "size": 10, "line": "curved", "maxAngle": 20}}],
         [one(1, "Dönemeç Sokak")])
    case("Yatay: çizginin ortasında düz", [polyline(1, "yol", [(10, 10), (90, 50)])],
         [{"id": "yol", "rank": 0, "label": {"placement": "center", "size": 10}}], [one(1, "Hat 3")])
    case("Yineleme: her parçaya bir etiket", [polyline(1, "yol", [(0, 30), (100, 30)])],
         [{"id": "yol", "rank": 0, "label": {"placement": "along", "size": 10, "repeat": 120}}], [one(1, "Su hattı")])
    case("Birleşik çizgiler tek etiket alır",
         [polyline(1, "yol", [(5, 30), (30, 32)]), polyline(2, "yol", [(30, 32), (60, 30)]), polyline(3, "yol", [(95, 31), (60, 30)]),
          polyline(4, "yol", [(5, 10), (40, 10)])],
         [{"id": "yol", "rank": 0, "label": {"placement": "along", "size": 10, "mergeLines": True}}],
         [one(1, "Atatürk Caddesi"), one(2, "Atatürk Caddesi"), one(3, "Atatürk Caddesi"), one(4, "Atatürk Caddesi")])
    contour_pts = [[(x, 15 + 12 * k + 3 * math.sin(x / 9.0 + k)) for x in range(0, 101, 5)] for k in range(3)]
    contour_pts[1].reverse()
    case("Eş yükselti: yazının üstü yokuş yukarı",
         [polyline(k + 1, "es", contour_pts[k]) for k in range(3)],
         [{"id": "es", "rank": 0, "label": {"placement": "along", "size": 9, "line": "contour", "repeat": 260}}],
         [one(1, "100", z=100.0), one(2, "105", z=105.0), one(3, "110", z=110.0)])

    # ── Areas (§3.3) ──
    case("Alanın adı erişilmezlik kutbunda",
         [polygon(1, "ada", [(10, 5), (90, 5), (90, 20), (30, 20), (30, 55), (10, 55)])],
         [{"id": "ada", "rank": 0, "label": {"placement": "center", "size": 12}}], [one(1, "1244")])
    case("Delikli alanda delikten uzak",
         [polygon(1, "ada", [(10, 5), (90, 5), (90, 55), (10, 55)], holes=[[(25, 15), (75, 15), (75, 45), (25, 45)]])],
         [{"id": "ada", "rank": 0, "label": {"placement": "center", "size": 10}}], [one(1, "Avlulu")])
    case("Eğik: sığmayan yatay yerine uzun kenarına paralel",
         [polygon(1, "parsel", turned(50, 30, 70, 6, 30))],
         [{"id": "parsel", "rank": 0, "label": {"placement": "center", "size": 10, "area": "free", "inside": True}}], [one(1, "Uzun Parsel")])
    case("Çevre boyunca, içeride", [rect(1, "alan", 10, 10, 90, 50)],
         [{"id": "alan", "rank": 0, "label": {"placement": "along", "size": 10, "position": "above"}}], [one(1, "Park")])
    case("Sınır: kenar boyunca yinelenir", [rect(1, "mahalle", 5, 5, 95, 55)],
         [{"id": "mahalle", "rank": 0, "label": {"placement": "center", "size": 10, "area": "boundary", "repeat": 260}}],
         [one(1, "Cumhuriyet Mahallesi")])
    case("Parsel: sığmayınca yığılır", [rect(1, "parsel", 40, 20, 55, 35)],
         [{"id": "parsel", "rank": 0, "label": {"placement": "center", "size": 10, "area": "parcel",
                                                 "stack": {"mode": "ifNeeded", "chars": 8}}}],
         [one(1, "Hazine Arazisi")])
    case("Parsel: sığmayınca kısaltılır", [rect(1, "parsel", 40, 20, 55, 35)],
         [{"id": "parsel", "rank": 0, "label": {"placement": "center", "size": 10, "area": "parcel",
                                                 "abbreviate": {"words": [{"word": "Arazisi", "short": "Ar."}]}}}],
         [one(1, "Hazine Arazisi")])
    case("Parsel: sığmayınca küçülür", [rect(1, "parsel", 40, 20, 55, 35)],
         [{"id": "parsel", "rank": 0, "label": {"placement": "center", "size": 10, "area": "parcel", "shrink": 0.6}}],
         [one(1, "Hazine Arazisi")])
    case("Parsel: hiçbiri sığmazsa dışarıda, çağrı çizgisiyle", [rect(1, "parsel", 48, 10, 50, 50)],
         [{"id": "parsel", "rank": 0, "label": {"placement": "center", "size": 10, "area": "parcel", "outside": True,
                                                 "callout": {"kind": "straight"}}}],
         [one(1, "13/1")])
    case("Köşe: kutunun sol üstünde", [rect(1, "pafta", 10, 10, 90, 50)],
         [{"id": "pafta", "rank": 0, "label": {"placement": "corner", "size": 10}}], [one(1, "G23-c-1")])
    case("En küçük nesneden küçükse yazılmaz", [rect(1, "parsel", 10, 10, 12, 12), rect(2, "parsel", 30, 10, 60, 40)],
         [{"id": "parsel", "rank": 0, "label": {"placement": "center", "size": 10, "minFeaturePx": 20}}],
         [one(1, "Küçük"), one(2, "Büyük")])

    # ── Rules, pins (§2, §3.7) ──
    rules = {"mode": "rules", "classes": [
        {"name": "No", "style": {"placement": "center", "size": 11, "weight": 600, "priority": 7}},
        {"name": "Malik", "when": "Malik <> ''", "style": {"placement": "center", "size": 9, "italic": True, "area": "free", "inside": True,
                                                         "priority": 3}}]}
    case("Kurallı sınıflar: numara ve malik",
         [rect(1, "parsel", 10, 10, 50, 50), rect(2, "parsel", 50, 10, 90, 50)],
         [{"id": "parsel", "rank": 0, "labels": rules}],
         [{"id": 1, "texts": [[0, "12"], [1, "Ayşe Demir"]]}, {"id": 2, "texts": [[0, "13"]]}],
         hits=[{"at": [30.0, 30.0], "tol": 1}])
    case("Sabit etiket: taşınmış, döndürülmüş, çağrı çizgisiyle; gizli etiket",
         [rect(1, "parsel", 10, 10, 50, 50), rect(2, "parsel", 50, 10, 90, 50), point(3, "nokta", 70, 30)],
         [{"id": "parsel", "rank": 1, "label": {"placement": "center", "size": 11, "callout": {"kind": "straight"}}},
          {"id": "nokta", "rank": 0, "point": 6, "label": NOKTA}],
         [one(1, "12"), one(2, "13"), one(3, "P.5")],
         pins=[[1, [{"at": {"x": -10.0, "y": 12.0}, "rotation": 20.0}]], [3, [{"hidden": True}]]],
         options={"hidden": True},
         hits=[{"at": [20.0, 42.0], "tol": 2}, {"at": [73.0, 32.0], "tol": 2}, {"at": [73.0, 32.0], "tol": 2, "all": True}])
    case("Kendi varsayılanı olmayan katman türün varsayılanını alır",
         [rect(1, "tanimsiz", 10, 10, 50, 50), polyline(2, "tanimsiz", [(55, 10), (95, 50)])],
         [{"id": "tanimsiz", "rank": 0}],
         [one(1, "Alan"), one(2, "Hat")],
         defaults={"polygon": {"placement": "center", "size": 10}, "polyline": {"placement": "along", "size": 10}})
    return out


def run(case):
    labels = place_window(case)
    got = {"name": case["name"], **{k: v for k, v in case.items() if k != "name"}}
    got["expect"] = labels
    if "hits" in case:
        got["hits"] = [{**h, "hit": label_at(labels, case["scale"], tuple(h["at"]), h["tol"], h.get("all", False))}
                       for h in case["hits"]]
    return got


def rounded(v):
    if isinstance(v, float):
        r = round(v, 9)
        return 0.0 if r == 0.0 else r
    if isinstance(v, (list, tuple)):
        return [rounded(x) for x in v]
    if isinstance(v, dict):
        return {k: rounded(x) for k, x in v.items()}
    return v


def document():
    return {"format": "kentos.labels-cases", "version": 1,
            "note": "Written by scripts/fixtures/label_engine_cases.py from docs/adr/0212, not from either platform's code. "
                    "Objects are the contract's; the window is [x0, y0, x1, y1] in metres, the scale px per metre. An expected "
                    "label: its object, class and state (1 pinned, 2 unplaced, 4 hidden, 8 overlapping, 16 curved, 32 outside "
                    "its area, 64 masked), its block's middle (world) and angle (degrees), width and height (px); a straight "
                    "label's lines [x, y, angle, size, text, width], a curved one's text and letters [x, y, angle, size, index, "
                    "advance]; its callout [from, to]. In the order the engine draws them.",
            "cases": [rounded(run(c)) for c in cases()]}


def main():
    text = json.dumps(document(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil: python3 scripts/fixtures/label_engine_cases.py ile yeniden yazın")
            return 1
        print(f"Etiket motorunun durumları tutarlı: {OUT.relative_to(ROOT)}")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
