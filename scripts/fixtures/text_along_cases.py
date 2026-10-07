"""The shared cases of a text along a curve (docs/adr/0196): where its
letters stand and turn, its box, its direction, Okunur yap, the transforms,
the piece of a curve a tool gives it, Düzleştir and Doğrultuya döndür.

    python3 scripts/fixtures/text_along_cases.py           # writes the file
    python3 scripts/fixtures/text_along_cases.py --check   # writes nothing; compares

Writes fixtures/text/v1/along.json. The rules are written here from the ADR
on their own, not from an implementation's output; the geometry core
(crates/shared/geometry-core, `text::along`, natively and through WASM:
`textLines`, `textAlongPiece`, `textAlongStraight`, `textAlongTurn`) is
held to them. The letters' advances are the drawing typefaces' measured
widths, read as data from the font recorder's table
(crates/shared/geometry-core/src/text/metrics.rs).

- The path in the world: vertex 0 is p, vertex i is p + R(rotation)·pts[i−1]
  (R turns counter-clockwise by the rotation, degrees); edge i runs from
  vertex i to vertex i + 1 and bends by bulges[i] (DXF's tan(θ/4), counter-
  clockwise positive; absent, straight): an arc of sweep θ = 4·atan(b) whose
  centre is on the chord's perpendicular bisector, chord/(2·tan(θ/2)) to the
  chord's left, radius chord/(2·|sin(θ/2)|), as long as radius·|θ|. A bulge
  of 1e−12 or less, or a chord under 1e−12, is straight. S: the edges' sum.
- A letter's advance: the face's table (the bold table when the text is bold
  with a typeface of its own, or its run is), 0.6 of it raised or lowered;
  metres: /1000 × height × width factor. L: their sum.
- From s0 = a·(S − L) (a: the alignment's share along: left 0, centre ½,
  right 1; none 0), letter i's middle m = s0 + the advances before it + its
  own half. Its point P and unit tangent T: for 0 ≤ m ≤ S on the last edge of
  non-zero length that starts at or before m; under 0 back along the first
  such edge's starting tangent; over S on along the last one's ending
  tangent, straight. A straight edge's tangent is its direction; an arc's
  is square to its radius, turning as its sweep does.
- The letter turns as T (degrees, 0 up to 360); its baseline starts at
  P − T·advance/2 − N·u·height: N the left normal (−T.y, T.x), u the
  alignment's share up (baseline 0, bottom −0.2, middle ½, top 1).
- Its box: each letter's from its baseline start, as long as its advance,
  0.23·height under the baseline to 1.15·height over it, a point `up` over it
  moved up·tan(slant) along it (the slant only with a typeface): the boxes'
  lower corners in order, then their upper corners from the last back.
- Its records: per letter a line (10: its baseline start, turn, height,
  width factor, letters i to i + 1), after its mask (11) when masked: the
  one-line text's mask over the letter's advance.
- Its direction: from the first letter's middle to the last one's (the
  first letter's tangent when they are within 1e−9 m or there is one letter).
  It reads upside down when that is over 90° and at most 270°.
- Turned the other way: the world's vertices backwards (the last the new p),
  the bulges backwards with their signs turned, the rotation half a turn on,
  the share along swapped (left and right). Okunur yap does so to a text
  that reads upside down and swaps its share up too (baseline and bottom to
  top, top to baseline, the middle kept).
- A similarity moves the world's vertices; the rotation turns as the
  baseline's direction; the height scales. Mirrored, the bulges' signs turn,
  then the text is turned the other way, then turned the other way again
  with its share up swapped (Okunur yap's swap) when its curve reads upside
  down: the direction from the curve's first vertex to its last (its first
  edge's starting tangent when they are within 1e−9 m).
- A curve's piece: the curve's paths (a line, an arc open; a circle closed;
  each part of a polyline open; each ring and hole of an area closed); the
  one nearest the click (the first on a tie), the distance s along it of
  its point nearest the click. When the path's tangent there reads upside
  down the path is taken backwards (s from its other end). The piece is L
  long from s − a·L: on an open path its start is held between 0 and S − L
  (the whole path when S < L), on a closed one it runs round past the start
  (once round when L ≥ S). Its first vertex is the text's p, the direction
  from its first vertex to its last its rotation (its first tangent when
  they are within 1e−9 m), its vertices in that frame its pts.
- Düzleştir: the point is the first letter's baseline start, the rotation
  the direction, no alignment, no path. Doğrultuya döndür: a direction's
  angle, half a turn less when it reads upside down.
"""
import json
import math
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/text/v1"
METRICS = ROOT / "crates/shared/geometry-core/src/text/metrics.rs"

# ── The measured advances (data) ────────────────────────────────────────


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

    return first, last, fonts, table("ADVANCES"), table("BOLD")


FIRST, LAST, FONTS, REGULAR, BOLD = read_metrics()


def advance(font, c, bold):
    row = (BOLD if bold else REGULAR)[FONTS.index(font)]
    code = ord(c)
    if FIRST <= code <= LAST and row[code - FIRST] > 0:
        return row[code - FIRST]
    return sum(row[ord("a") - FIRST:ord("z") - FIRST + 1]) // 26


# ── Alignment shares (docs/adr/0145) ────────────────────────────────────

SHARES = {
    None: (0.0, 0.0),
    "baselineCenter": (0.5, 0.0),
    "baselineRight": (1.0, 0.0),
    "bottomLeft": (0.0, -0.2),
    "bottomCenter": (0.5, -0.2),
    "bottomRight": (1.0, -0.2),
    "middleLeft": (0.0, 0.5),
    "middleCenter": (0.5, 0.5),
    "middleRight": (1.0, 0.5),
    "topLeft": (0.0, 1.0),
    "topCenter": (0.5, 1.0),
    "topRight": (1.0, 1.0),
}
NAME = {v: k for k, v in SHARES.items()}


def swapped(align, up_too):
    a, u = SHARES[align]
    a = 1.0 - a
    if up_too:
        u = {0.0: 1.0, -0.2: 1.0, 0.5: 0.5, 1.0: 0.0}[u]
    return NAME[(a, u)]


# ── Geometry ────────────────────────────────────────────────────────────


def deg(rad):
    return math.degrees(rad) % 360.0


def turned(v, degrees):
    r = math.radians(degrees)
    c, s = math.cos(r), math.sin(r)
    return (c * v[0] - s * v[1], s * v[0] + c * v[1])


def edge(a, b, bulge):
    dx, dy = b[0] - a[0], b[1] - a[1]
    chord = math.hypot(dx, dy)
    if abs(bulge) > 1e-12 and chord >= 1e-12:
        theta = 4.0 * math.atan(bulge)
        k = 1.0 / (2.0 * math.tan(theta / 2.0))
        c = ((a[0] + b[0]) / 2.0 - dy * k, (a[1] + b[1]) / 2.0 + dx * k)
        r = chord / (2.0 * abs(math.sin(theta / 2.0)))
        a0 = math.atan2(a[1] - c[1], a[0] - c[0])
        return {"arc": True, "a": a, "b": b, "c": c, "r": r, "a0": a0, "sweep": theta, "len": r * abs(theta)}
    return {"arc": False, "a": a, "b": b, "len": chord}


def point(e, t):
    if e["arc"]:
        f = e["a0"] + e["sweep"] * t
        return (e["c"][0] + e["r"] * math.cos(f), e["c"][1] + e["r"] * math.sin(f))
    return (e["a"][0] + (e["b"][0] - e["a"][0]) * t, e["a"][1] + (e["b"][1] - e["a"][1]) * t)


def tangent(e, t):
    if e["arc"]:
        f = e["a0"] + e["sweep"] * t
        sign = 1.0 if e["sweep"] >= 0 else -1.0
        return (-math.sin(f) * sign, math.cos(f) * sign)
    return ((e["b"][0] - e["a"][0]) / e["len"], (e["b"][1] - e["a"][1]) / e["len"])


def world(t):
    p = (t["p"]["x"], t["p"]["y"])
    vs = [p]
    for q in t["path"]["pts"]:
        d = turned((q["x"], q["y"]), t["rotation"])
        vs.append((p[0] + d[0], p[1] + d[1]))
    bulges = t["path"].get("bulges") or [0.0] * (len(vs) - 1)
    return vs, list(bulges)


def edges_of(vs, bulges):
    return [edge(vs[i], vs[i + 1], bulges[i]) for i in range(len(vs) - 1)]


def locate(es, m):
    total = sum(e["len"] for e in es)
    live = [e for e in es if e["len"] > 0]
    if m < 0:
        e = live[0]
        tg = tangent(e, 0.0)
        return (e["a"][0] + tg[0] * m, e["a"][1] + tg[1] * m), tg
    if m > total:
        e = live[-1]
        tg = tangent(e, 1.0)
        return (e["b"][0] + tg[0] * (m - total), e["b"][1] + tg[1] * (m - total)), tg
    cum, chosen = 0.0, None
    for e in es:
        if e["len"] > 0 and cum <= m:
            chosen = (e, cum)
        cum += e["len"]
    e, start = chosen
    t = min(1.0, (m - start) / e["len"])
    return point(e, t), tangent(e, t)


# ── A text along its path ───────────────────────────────────────────────


def advances(t):
    font = t.get("font") or t.get("drawingFont", "barlow")
    all_bold = bool(t.get("bold")) and t.get("font") is not None
    runs = t.get("runs", [])
    h, w = t["height"], t.get("widthFactor", 1.0)
    out = []
    for i, c in enumerate(t["text"]):
        run = next((r for r in runs if r["start"] <= i < r["end"]), None)
        a = float(advance(font, c, all_bold or bool(run and run.get("bold"))))
        if run and run.get("script"):
            a *= 0.6
        out.append(a / 1000.0 * h * w)
    return out


def lean(t):
    return math.tan(math.radians(t["oblique"])) if t.get("font") and t.get("oblique") else 0.0


def letters(t):
    vs, bulges = world(t)
    es = edges_of(vs, bulges)
    total = sum(e["len"] for e in es)
    adv = advances(t)
    a, u = SHARES[t.get("align")]
    h = t["height"]
    x = a * (total - sum(adv))
    out = []
    for A in adv:
        mid = x + A / 2.0
        P, T = locate(es, mid)
        N = (-T[1], T[0])
        at = (P[0] - T[0] * A / 2.0 - N[0] * u * h, P[1] - T[1] * A / 2.0 - N[1] * u * h)
        out.append({"mid": P, "tangent": T, "at": at, "turn": deg(math.atan2(T[1], T[0])), "advance": A})
        x = x + A
    return out, total


def direction(t, ls):
    if not ls:
        return t["rotation"] % 360.0
    a, b = ls[0]["mid"], ls[-1]["mid"]
    if len(ls) == 1 or math.hypot(b[0] - a[0], b[1] - a[1]) <= 1e-9:
        T = ls[0]["tangent"]
        return deg(math.atan2(T[1], T[0]))
    return deg(math.atan2(b[1] - a[1], b[0] - a[0]))


def upside_down(d):
    return 90.0 < d <= 270.0


def outline(t, ls, margin=0.0):
    h = t["height"]
    k = lean(t)
    y0, y1 = -0.23 * h - margin, 1.15 * h + margin

    def corner(l, x, y):
        T = l["tangent"]
        N = (-T[1], T[0])
        x = x + y * k
        return (l["at"][0] + T[0] * x + N[0] * y, l["at"][1] + T[1] * x + N[1] * y)

    lower, upper = [], []
    for l in ls:
        lower += [corner(l, -margin, y0), corner(l, l["advance"] + margin, y0)]
        upper += [corner(l, l["advance"] + margin, y1), corner(l, -margin, y1)]
    return lower + upper[::-1]


def records(t, ls):
    h = t["height"]
    out = []
    for i, l in enumerate(ls):
        T = l["tangent"]
        N = (-T[1], T[0])
        if t.get("mask"):
            hh, mg = h * 1.15, h * 0.1
            y0 = -hh * 0.2 - mg
            x0 = -mg + y0 * lean(t)
            c = (l["at"][0] + T[0] * x0 + N[0] * y0, l["at"][1] + T[1] * x0 + N[1] * y0)
            out.append([0, 11, c[0], c[1], l["turn"], l["advance"] + 2 * mg, hh * 1.2 + 2 * mg, 0, 0])
        out.append([0, 10, l["at"][0], l["at"][1], l["turn"], h, t.get("widthFactor", 1.0), i, i + 1])
    return out


# ── Turning it the other way, Okunur yap, the transforms ───────────────


def framed(vs, bulges, rotation):
    p = vs[0]
    pts = []
    for v in vs[1:]:
        q = turned((v[0] - p[0], v[1] - p[1]), -rotation)
        pts.append({"x": q[0], "y": q[1]})
    path = {"pts": pts}
    if any(b != 0 for b in bulges):
        path["bulges"] = list(bulges)
    return {"p": {"x": p[0], "y": p[1]}, "rotation": rotation % 360.0, "path": path}


def with_place(t, place, align):
    out = dict(t)
    out.update(place)
    if align is None:
        out.pop("align", None)
    else:
        out["align"] = align
    return out


def reversed_text(t, up_too):
    vs, bulges = world(t)
    place = framed(vs[::-1], [-b for b in bulges[::-1]], t["rotation"] + 180.0)
    return with_place(t, place, swapped(t.get("align"), up_too))


def readable(t):
    ls, _ = letters(t)
    if not upside_down(direction(t, ls)):
        return None
    return reversed_text(t, True)


def apply(m, v):
    a, b, c, d, e, f = m
    return (a * v[0] + c * v[1] + e, b * v[0] + d * v[1] + f)


def transformed(t, m):
    a, b, c, d = m[:4]
    det = a * d - b * c
    s = math.sqrt(abs(det))
    vs, bulges = world(t)
    vs = [apply(m, v) for v in vs]
    r = math.radians(t["rotation"])
    u = (a * math.cos(r) + c * math.sin(r), b * math.cos(r) + d * math.sin(r))
    turn = deg(math.atan2(u[1], u[0]))
    out = dict(t)
    out["height"] = t["height"] * s
    if det >= 0:
        return with_place(out, framed(vs, bulges, turn), t.get("align"))
    # Mirrored: the bulges turn; then the other way round; then Okunur yap's swap when the curve reads upside down.
    out = with_place(out, framed(vs, [-x for x in bulges], turn), t.get("align"))
    out = reversed_text(out, False)
    if upside_down(chord(out)):
        out = reversed_text(out, True)
    return out


def chord(t):
    vs, bulges = world(t)
    a, b = vs[0], vs[-1]
    if math.hypot(b[0] - a[0], b[1] - a[1]) <= 1e-9:
        e = next(e for e in edges_of(vs, bulges) if e["len"] > 0)
        T = tangent(e, 0.0)
        return deg(math.atan2(T[1], T[0]))
    return deg(math.atan2(b[1] - a[1], b[0] - a[0]))


# ── A curve's piece ─────────────────────────────────────────────────────


def ring_edges(pts, bulges, closed):
    n = len(pts)
    count = n if closed else n - 1
    out = []
    for i in range(count):
        b = (bulges[i] if bulges and i < len(bulges) else 0.0)
        out.append(edge(pts[i], pts[(i + 1) % n], b))
    return out


def xy(p):
    return (p["x"], p["y"])


def paths_of(curve):
    k = curve["kind"]
    if k == "line":
        return [{"edges": [edge(xy(curve["a"]), xy(curve["b"]), 0.0)], "closed": False}]
    if k == "arc":
        sw = (curve["a1"] - curve["a0"]) % (2 * math.pi)
        if sw < 1e-12:
            sw = 2 * math.pi
        c, r = xy(curve["c"]), curve["r"]
        a = (c[0] + r * math.cos(curve["a0"]), c[1] + r * math.sin(curve["a0"]))
        b = (c[0] + r * math.cos(curve["a0"] + sw), c[1] + r * math.sin(curve["a0"] + sw))
        e = {"arc": True, "a": a, "b": b, "c": c, "r": r, "a0": curve["a0"], "sweep": sw, "len": r * sw}
        return [{"edges": [e], "closed": False}]
    if k == "circle":
        c, r = xy(curve["c"]), curve["r"]
        e = {"arc": True, "a": (c[0] + r, c[1]), "b": (c[0] + r, c[1]), "c": c, "r": r, "a0": 0.0, "sweep": 2 * math.pi, "len": r * 2 * math.pi}
        return [{"edges": [e], "closed": True}]
    parts = [curve] + curve.get("parts", [])
    out = []
    for part in parts:
        closed = k == "polygon"
        out.append({"edges": ring_edges([xy(p) for p in part["pts"]], part.get("bulges"), closed), "closed": closed})
        for hole in part.get("holes", []) if closed else []:
            out.append({"edges": ring_edges([xy(p) for p in hole["pts"]], hole.get("bulges"), True), "closed": True})
    return out


def nearest_on(e, p):
    if not e["arc"]:
        a, b = e["a"], e["b"]
        dx, dy = b[0] - a[0], b[1] - a[1]
        l2 = dx * dx + dy * dy
        t = 0.0 if l2 < 1e-12 else ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2
        t = max(0.0, min(1.0, t))
        q = point(e, t)
        return t, math.hypot(p[0] - q[0], p[1] - q[1])
    c, r, a0, sw = e["c"], e["r"], e["a0"], e["sweep"]
    f = math.atan2(p[1] - c[1], p[0] - c[0])
    off = ((f - a0) % (2 * math.pi)) if sw >= 0 else ((a0 - f) % (2 * math.pi))
    if abs(sw) >= 2 * math.pi or off <= abs(sw):
        return off / abs(sw), abs(math.hypot(p[0] - c[0], p[1] - c[1]) - r)
    s, f1 = point(e, 0.0), point(e, 1.0)
    ds, df = math.hypot(p[0] - s[0], p[1] - s[1]), math.hypot(p[0] - f1[0], p[1] - f1[1])
    return (0.0, ds) if ds <= df else (1.0, df)


def cums(es):
    out, s = [], 0.0
    for e in es:
        out.append(s)
        s += e["len"]
    return out, s


def at_s(path, s):
    es = path["edges"]
    cum, total = cums(es)
    q = (s % total) if path["closed"] else max(0.0, min(total, s))
    i = 0
    for j in range(len(es) - 1, -1, -1):
        if q >= cum[j] - 1e-12:
            i = j
            break
    e = es[i]
    t = min(1.0, (q - cum[i]) / (e["len"] or 1.0))
    return point(e, t), tangent(e, t)


def backwards(path):
    es = []
    for e in reversed(path["edges"]):
        if e["arc"]:
            es.append({**e, "a": e["b"], "b": e["a"], "a0": e["a0"] + e["sweep"], "sweep": -e["sweep"]})
        else:
            es.append({**e, "a": e["b"], "b": e["a"]})
    return {"edges": es, "closed": path["closed"]}


def stretch(path, s0, s1):
    es = path["edges"]
    cum, total = cums(es)
    vs = [at_s(path, s0)[0]]
    bulges = []
    for rnd in range(2 if path["closed"] else 1):
        for i, e in enumerate(es):
            g0 = cum[i] + rnd * total
            lo, hi = max(s0, g0), min(s1, g0 + e["len"])
            if hi - lo <= 1e-9 * max(1.0, total):
                continue
            t0, t1 = (lo - g0) / (e["len"] or 1.0), (hi - g0) / (e["len"] or 1.0)
            bulges.append(math.tan(e["sweep"] * (t1 - t0) / 4.0) if e["arc"] else 0.0)
            vs.append(point(e, min(1.0, t1)))
    # Vertices within 1e−9 of the one before go, the segment after them keeping its bulge.
    kept_v, kept_b = [vs[0]], []
    for i in range(1, len(vs)):
        if math.hypot(vs[i][0] - kept_v[-1][0], vs[i][1] - kept_v[-1][1]) <= 1e-9:
            continue
        kept_v.append(vs[i])
        kept_b.append(bulges[i - 1])
    return kept_v, kept_b


def piece(curve, click, length, share):
    paths = paths_of(curve)
    best = None
    for path in paths:
        cum, _ = cums(path["edges"])
        for i, e in enumerate(path["edges"]):
            t, d = nearest_on(e, click)
            if best is None or d < best[0]:
                best = (d, path, cum[i] + t * e["len"])
    _, path, s = best
    _, total = cums(path["edges"])
    _, T = at_s(path, s)
    if upside_down(deg(math.atan2(T[1], T[0]))):
        path, s = backwards(path), total - s
    start = s - share * length
    if path["closed"]:
        start = start % total
        end = start + min(length, total)
    elif total < length:
        start, end = 0.0, total
    else:
        start = max(0.0, min(total - length, start))
        end = start + length
    vs, bulges = stretch(path, start, end)
    if math.hypot(vs[-1][0] - vs[0][0], vs[-1][1] - vs[0][1]) <= 1e-9:
        T = at_s(path, start)[1]
        rotation = deg(math.atan2(T[1], T[0]))
    else:
        rotation = deg(math.atan2(vs[-1][1] - vs[0][1], vs[-1][0] - vs[0][0]))
    return framed(vs, bulges, rotation)


def straight(t):
    ls, _ = letters(t)
    return {"p": {"x": ls[0]["at"][0], "y": ls[0]["at"][1]}, "rotation": direction(t, ls)}


def turn_of(d):
    a = deg(math.atan2(d["y"], d["x"]))
    if upside_down(a):
        a = (a - 180.0) % 360.0
    return a


# ── The cases ───────────────────────────────────────────────────────────


def P(x, y):
    return {"x": x, "y": y}


def text(name, p, rotation, pts, text_, height=2.0, bulges=None, **more):
    t = {"p": p, "rotation": rotation, "path": {"pts": [P(*q) for q in pts]}, "text": text_, "height": height}
    if bulges is not None:
        t["path"]["bulges"] = bulges
    t.update(more)
    return name, t


LAYOUT = [
    text("düz-sol", P(100, 50), 0.0, [(30, 0)], "Dere"),
    text("düz-orta-orta", P(100, 50), 30.0, [(30, 0)], "Kanal adı", align="middleCenter"),
    text("düz-sağ-alt", P(-20, 7.5), 0.0, [(25, 0)], "Yol", align="bottomRight", height=1.5),
    text("gülümseyen-yay", P(0, 0), 0.0, [(40, 0)], "Çamlıca Deresi", bulges=[0.4], align="bottomCenter"),
    text("tepe-yayı", P(0, 0), 0.0, [(40, 0)], "Atatürk Bulvarı", bulges=[-0.5], align="baselineCenter", height=2.5),
    text("köşe", P(10, 10), 0.0, [(12, 0), (12, -12)], "Köşeden döner", align="middleLeft"),
    text("uzun-orta", P(0, 0), 15.0, [(8, 0)], "Uzun bir yazı", align="topCenter"),
    text("kalın-dar", P(5, 5), 0.0, [(10, 3), (25, 0)], "Kalın x2", bulges=[0.0, 0.2], font="arimo", bold=True, widthFactor=0.8,
         runs=[{"start": 6, "end": 8, "script": "super"}]),
    text("yatık-zeminli", P(3, -2), 350.0, [(30, 0)], "Eğik ve zeminli", bulges=[-0.25], font="overpass", oblique=15.0, mask=True,
         align="middleCenter"),
    text("biçimli-harfler", P(0, 0), 0.0, [(20, 0)], "abcdef", runs=[{"start": 1, "end": 3, "bold": True}, {"start": 4, "end": 5, "script": "sub"}],
         drawingFont="quicksand"),
    text("sıfır-kenar", P(0, 0), 0.0, [(10, 0), (10, 0), (20, 6)], "Tekrar eden köşe"),
    text("uzak-yay", P(487000.25, 4420000.75), 12.0, [(60, 0)], "Uzaktaki dere", bulges=[0.3], align="bottomCenter", height=3.0),
    text("tek-harf", P(2, 2), 0.0, [(5, 0)], "A", bulges=[0.6], align="middleCenter"),
]

READABLE = [
    text("okunur", P(0, 0), 10.0, [(20, 0)], "Okunur yazı"),
    text("ters-düz", P(0, 0), 180.0, [(20, 0)], "Ters yazı"),
    text("ters-yay-alt", P(40, 0), 180.0, [(40, 0)], "Ters yaydaki yazı", bulges=[0.35], align="bottomLeft"),
    text("ters-orta-sağ", P(10, 10), 200.0, [(15, 0), (30, 4)], "Ortada", align="middleRight"),
    text("dikin-berisi", P(0, 0), 89.5, [(10, 0)], "Yukarı"),
    text("dikin-ötesi", P(0, 0), 90.5, [(10, 0)], "Yukarı", align="bottomLeft"),
    text("aşağının-berisi", P(0, 0), 269.5, [(10, 0)], "Aşağı", align="topCenter"),
    text("aşağının-ötesi", P(0, 0), 270.5, [(10, 0)], "Aşağı"),
]

TRANSFORM = [
    ("taşı", LAYOUT[3][1], [1, 0, 0, 1, 12.5, -7.25]),
    ("döndür", LAYOUT[5][1], [0, 1, -1, 0, 3, 4]),
    ("ölçekle", LAYOUT[4][1], [2, 0, 0, 2, -10, 5]),
    ("aynala-dikey", LAYOUT[3][1], [-1, 0, 0, 1, 100, 0]),
    ("aynala-yatay", LAYOUT[4][1], [1, 0, 0, -1, 0, 20]),
    ("aynala-eğik", LAYOUT[0][1], [0, 1, 1, 0, 0, 0]),
]

CIRCLE = {"kind": "circle", "c": P(0, 0), "r": 10.0}
ARC_TOP = {"kind": "arc", "c": P(0, 0), "r": 20.0, "a0": math.radians(20), "a1": math.radians(160)}
PIECE = [
    ("çizgi-orta", {"kind": "line", "a": P(0, 0), "b": P(40, 10)}, P(20, 8), 12.0, 0.5),
    ("çizgi-uca-yakın", {"kind": "line", "a": P(0, 0), "b": P(40, 10)}, P(39, 9), 12.0, 0.5),
    ("çizgi-kısa", {"kind": "line", "a": P(0, 0), "b": P(6, 0)}, P(3, 1), 12.0, 0.0),
    ("çizgi-ters", {"kind": "line", "a": P(40, 0), "b": P(0, 0)}, P(10, -1), 8.0, 0.0),
    ("çizgi-ters-sonu", {"kind": "line", "a": P(40, 0), "b": P(0, 0)}, P(30, 2), 8.0, 1.0),
    ("yay-üst", ARC_TOP, P(3, 22), 15.0, 0.5),
    ("yay-alt", {"kind": "arc", "c": P(0, 0), "r": 20.0, "a0": math.radians(200), "a1": math.radians(340)}, P(-2, -19), 15.0, 0.5),
    ("daire-üst", CIRCLE, P(1, 11), 9.0, 0.5),
    ("daire-alt", CIRCLE, P(-1, -9), 9.0, 0.5),
    ("daire-başlangıçtan-geçen", CIRCLE, P(10.5, -0.5), 9.0, 0.5),
    ("daire-tamamı", CIRCLE, P(0, -10.5), 80.0, 0.5),
    ("çoklu-çizgi-yaylı", {"kind": "polyline", "pts": [P(0, 0), P(20, 0), P(30, 10), P(50, 10)], "bulges": [0, 0.4, 0, 0]}, P(24, 6), 18.0, 0.5),
    ("çoklu-çizgi-ikinci-parça", {"kind": "polyline", "pts": [P(0, 0), P(10, 0)], "parts": [{"pts": [P(0, 20), P(30, 25), P(60, 20)]}]}, P(31, 24), 20.0, 0.5),
    ("alanın-deliği", {"kind": "polygon", "pts": [P(0, 0), P(60, 0), P(60, 40), P(0, 40)],
                       "holes": [{"pts": [P(20, 10), P(40, 10), P(40, 30), P(20, 30)]}]}, P(30, 29), 10.0, 0.5),
    ("alanın-halkası-baştan", {"kind": "polygon", "pts": [P(0, 0), P(60, 0), P(60, 40), P(0, 40)]}, P(2, -1), 10.0, 0.5),
]

STRAIGHT = [LAYOUT[0], LAYOUT[3], LAYOUT[5], LAYOUT[12]]

TURN = [("doğu", P(3, 0)), ("kuzeybatı", P(-1, 1)), ("batı", P(-2, 0)), ("güney", P(0, -1)), ("güneydoğu", P(2, -1)), ("kuzey", P(0, 4))]


def plain(v):
    """Tuples as {x, y}; everything else as it is."""
    if isinstance(v, tuple):
        return {"x": v[0], "y": v[1]}
    if isinstance(v, list):
        return [plain(x) for x in v]
    if isinstance(v, dict):
        return {k: plain(x) for k, x in v.items()}
    return v


def build():
    layout = []
    for name, t in LAYOUT:
        ls, total = letters(t)
        layout.append({
            "name": name,
            "text": t,
            "want": {
                "pathLength": total,
                "textLength": sum(l["advance"] for l in ls),
                "letters": [{"at": l["at"], "turn": l["turn"], "advance": l["advance"]} for l in ls],
                "direction": direction(t, ls),
                "outline": outline(t, ls),
                "grown": outline(t, ls, t["height"] * 0.25),
                "records": records(t, ls),
            },
        })
    readable_cases = [{"name": n, "text": t, "want": readable(t)} for n, t in READABLE]
    transforms = [{"name": n, "text": t, "affine": m, "want": transformed(t, m)} for n, t, m in TRANSFORM]
    pieces = [{"name": n, "curve": c, "click": k, "length": l, "share": a, "want": piece(c, xy(k), l, a)} for n, c, k, l, a in PIECE]
    straights = [{"name": n, "text": t, "want": straight(t)} for n, t in STRAIGHT]
    turns = [{"name": n, "direction": d, "want": turn_of(d)} for n, d in TURN]
    return {
        "along.json": plain({
            "format": "kentos.text-cases",
            "version": 1,
            "generatedBy": "scripts/fixtures/text_along_cases.py",
            "title": "Eğri boyunca yazı: harfler, kutu, doğrultu, Okunur yap, dönüşümler, eğrinin parçası (ADR 0196)",
            "note": "Eğri yazının çerçevesinde (p başlangıç, x dönüklük doğrultusunda, metre); harf ortası eğri üstünde, dönüklüğü teğeti; taban başlangıcı P − T·A/2 − N·u·h. Kutu harflerin kutularının alt köşeleri sırayla, üst köşeleri tersten; grown 0,25·h genişletilmiş. Kayıtlar textLines'ınki (kimlik 0). Açılar derece, 0'dan 360'a.",
            "layout": layout,
            "readable": readable_cases,
            "transform": transforms,
            "piece": pieces,
            "straight": straights,
            "turn": turns,
        })
    }


def text_of(v):
    return json.dumps(v, ensure_ascii=False, indent=2) + "\n"


def main():
    files = build()
    if "--check" in sys.argv[1:]:
        bad = [name for name, v in files.items() if not (DIR / name).exists() or (DIR / name).read_text("utf-8") != text_of(v)]
        for name in bad:
            print(f"{name}: diskteki dosya kurallardan üretilenle aynı değil", file=sys.stderr)
        if bad:
            return 1
        print(f"text along cases match: {', '.join(files)}")
        return 0
    DIR.mkdir(parents=True, exist_ok=True)
    for name, v in files.items():
        (DIR / name).write_text(text_of(v), encoding="utf-8")
    print(f"written: {', '.join(files)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
