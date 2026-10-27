#!/usr/bin/env python3
"""İç ölçü zinciri (docs/adr/0711), from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.dimensions.interior.json; `--check` compares and writes nothing. Kaydır in the follow
cases is scripts/fixtures/wall_shift_cases.py's (ADR 0710), written from its ADR.

The rules, as ADR 0711 gives them:
- a wall's lines are its path moved to its left by each boundary (the left face `offset + w/2` for a centred wall,
  `offset` for a left one, `offset + w` for a right one; between its components' thicknesses; the right face the left
  less the width), joined at the path's corners where the two moved edges' lines meet (a mitre), straight at an open
  path's ends. `outer` takes its first and last line, `skins` every one;
- the cut from `from` to `to` (at least 1 mm) crosses a line's straight edge where the two meet within both, ends
  included; a parallel edge does not cross. A curved edge it crosses is counted, not measured;
- a column: `ends` a rectangular one's sides it crosses (corners (−w/2, −d/2), (w/2, −d/2), (w/2, d/2), (−w/2, d/2)
  turned by its rotation about its centre, the side from corner k to the next is side k), a round one's centre;
  `centers` its centre when the cut crosses it (a side, or the cut starts inside); `none` nothing;
- a crossing more than 1e-9 m inside another wall's body or column is a face no one sees: left out (a column's centre
  is not a face and stays);
- the walls and columns on a hidden or frozen layer are not crossed; with `level`, only those of that level;
- the places sorted along the cut (then the drawing's order, then the line and the edge); of two within 1e-9 m the
  first stays; each consecutive pair a linear dimension: its angle the cut's (degrees from east), `c` the cut's start,
  its line through `at` (absent: the cut's start) along the cut; with `total` (and more than two places) the first to
  the last one row (8 paper mm × the plot scale / 1000) further, on `at`'s side of the cut (left when on it);
- the rule `cross` follows: an edge's point is where its line crosses the cut through `c`, its `t` where that is
  along the edge (when on it); the dimension's line stays (its offset changes by how far its `a` moved across).
"""

import argparse
import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import wall_command_cases as wc  # noqa: E402  (layers, ids, non-finite numbers)
import wall_shift_cases as ws  # noqa: E402  (Kaydır, ADR 0710)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
COMMAND = "cad.dimensions.interior"
STEP = "İç ölçü zinciri"
TIE = 1e-9
SHORTEST = 1e-3
WITHIN = 1e-9
PLOT = 50.0
STANDARD_HEIGHT = 2.5 * PLOT / 1000.0
MIMARI = "1d4f0000-0000-4000-8000-0000000000a1"
MIMARI_HEIGHT = 3.5 * PLOT / 1000.0


def P(x, y):
    return {"x": float(x), "y": float(y)}


def uid(i):
    return f"0192f5a1-7111-7000-8000-{i:012x}"


def wall(id, pts, width=0.2, layer="duvar", **extra):
    return {"kind": "wall", "id": id, "layerId": layer, "attrs": {}, "pts": [P(*p) for p in pts], "width": width,
            "justify": extra.pop("justify", "center"), **extra}


def column(id, at, width, layer="kolon", **extra):
    return {"kind": "column", "id": id, "layerId": layer, "attrs": {}, "at": P(*at), "width": width, **extra}


SKINS = [{"name": "Sıva", "thickness": 0.02, "detail": "standard"}, {"name": "Tuğla", "detail": "standard"},
         {"name": "Sıva", "thickness": 0.02, "detail": "standard"}]
ENTITIES = [
    # A: three rooms in a row, 0..12 × 0..4: four walls across, two along.
    wall(1, [(0, 0), (0, 4)]),
    wall(2, [(4, 0), (4, 4)]),
    wall(3, [(8, 0), (8, 4)]),
    wall(4, [(12, 0), (12, 4)]),
    wall(5, [(0, 0), (12, 0)]),
    wall(6, [(12, 4), (0, 4)]),
    # B: a wall of three components and a plain one.
    wall(7, [(0, 10), (0, 14)], width=0.3, components=SKINS),
    wall(8, [(4, 10), (4, 14)]),
    # C: a rectangular and a round column between two walls.
    wall(9, [(0, 20), (0, 24)]),
    wall(10, [(6, 20), (6, 24)]),
    column(11, (2, 22), 0.4),
    column(12, (4, 22), 0.5, shape="round"),
    # D: a column set in a wall.
    wall(13, [(0, 30), (8, 30)], width=0.4),
    column(14, (4, 30), 0.3),
    # E: a room of four walls meeting at its corners.
    wall(15, [(0, 40), (5, 40)]),
    wall(16, [(5, 40), (5, 44)]),
    wall(17, [(5, 44), (0, 44)]),
    wall(18, [(0, 44), (0, 40)]),
    # F: a curved wall between two straight ones.
    wall(19, [(0, 50), (6, 50)], bulges=[0.3, 0.0]),
    wall(20, [(-2, 48), (-2, 54)]),
    wall(21, [(8, 48), (8, 54)]),
    # G: walls of two levels, overlapping in plan.
    wall(22, [(0, 60), (0, 64)], arch={"level": "zemin"}),
    wall(23, [(4, 60), (4, 64)], arch={"level": "zemin"}),
    wall(24, [(1, 60), (1, 64)], arch={"level": "kat-1"}),
    wall(25, [(5, 60), (5, 64)], arch={"level": "kat-1"}),
    # H: walls on a hidden, a frozen and a locked layer.
    wall(26, [(0, 70), (0, 74)]),
    wall(27, [(2, 70), (2, 74)], layer="gizli"),
    wall(28, [(4, 70), (4, 74)], layer="donuk"),
    wall(29, [(6, 70), (6, 74)], layer="kilitli"),
    # I: a closed rectangular wall.
    wall(30, [(0, 80), (5, 80), (5, 84), (0, 84)], closed=True),
    # K: a turned rectangular column between two walls.
    column(31, (2, 92), 0.6, depth=0.3, rotation=0.5),
    wall(32, [(0, 90), (0, 94)]),
    wall(33, [(4, 90), (4, 94)]),
    # L: a right-justified wall and a centred one; a line, not measured.
    wall(34, [(0, 100), (0, 104)], justify="right"),
    wall(35, [(3, 100), (3, 104)]),
    {"kind": "line", "id": 36, "layerId": "cizim", "attrs": {}, "a": P(1.5, 100), "b": P(1.5, 104)},
]
SETTINGS = {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": PLOT,
            "workspace": "cad",
            "dimensionStyles": [{"id": MIMARI, "name": "Mimari", "height": 3.5}],
            "arch": {"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
                                {"id": "kat-1", "name": "1. kat", "elevation": 3.0, "height": 3.0}]}}
SETUP = {
    "format": "kentos.document", "version": 2, "name": "İç ölçü zinciri", "settings": SETTINGS, "origin": P(0, 0),
    "layers": [wc.layer("duvar", "Duvar"), wc.layer("kolon", "Kolon"), wc.layer("olcu", "Ölçü"), wc.layer("cizim", "Çizim"),
               wc.layer("gizli", "Gizli katman", visible=False), {**wc.layer("donuk", "Dondurulmuş katman"), "frozen": True},
               wc.layer("kilitli", "Kilitli katman", locked=True), wc.layer("grup", "Grup", kind="group", children=[wc.layer("alt", "Alt")])],
    "activeLayer": "duvar", "styles": {"items": [], "categories": []}, "entities": ENTITIES,
    "uids": [uid(e["id"]) for e in ENTITIES],
}
IDS = [e["id"] for e in ENTITIES]
BY_ID = {e["id"]: e for e in ENTITIES}
HIDDEN = {"gizli", "donuk"}


# ── Geometry ─────────────────────────────────────────────────────────────────────────────────────────────────────

def xy(p):
    return (p["x"], p["y"])


def sub(a, b):
    return (a[0] - b[0], a[1] - b[1])


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1]


def cross(a, b):
    return a[0] * b[1] - a[1] * b[0]


def dist(a, b):
    return math.hypot(b[0] - a[0], b[1] - a[1])


def thicknesses(w):
    comps = w.get("components") or []
    fixed = sum(c["thickness"] for c in comps if "thickness" in c)
    rest = [c for c in comps if "thickness" not in c]
    assert len(rest) <= 1
    return [c.get("thickness", w["width"] - fixed) for c in comps]


def boundaries(w):
    left = ws.left_face(w)
    out = [left]
    if w.get("components"):
        at = left
        for t in thicknesses(w)[:-1]:
            at -= t
            out.append(at)
    out.append(left - w["width"])
    return out


def meet(p, d, q, e):
    """Where the line through p along d meets the line through q along e; none when parallel."""
    den = cross(d, e)
    if den == 0:
        return None
    s = cross(sub(q, p), e) / den
    return (p[0] + d[0] * s, p[1] + d[1] * s)


def offset_line(pts, d, closed):
    """A straight path moved `d` to its left, mitred at its corners."""
    n = len(pts)
    count = n if closed else n - 1
    pieces = []
    for i in range(count):
        a, b = pts[i], pts[(i + 1) % n]
        l = dist(a, b)
        nx, ny = -(b[1] - a[1]) / l * d, (b[0] - a[0]) / l * d
        pieces.append(((a[0] + nx, a[1] + ny), (b[0] + nx, b[1] + ny)))
    out = []
    for i in range(n):
        if not closed and i == 0:
            out.append(pieces[0][0])
        elif not closed and i == n - 1:
            out.append(pieces[-1][1])
        else:
            p1, p2 = pieces[(i - 1) % count], pieces[i % count]
            out.append(meet(p1[0], sub(p1[1], p1[0]), p2[0], sub(p2[1], p2[0])))
    return out


def curved(w):
    return any(b != 0.0 for b in (w.get("bulges") or []))


def bulge_arc(a, b, k):
    """An arc edge's centre, radius, start angle and sweep (counter-clockwise positive)."""
    sweep = 4.0 * math.atan(k)
    c = dist(a, b)
    r = c / (2.0 * math.sin(abs(sweep) / 2.0))
    m = ((a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0)
    h = r * math.cos(abs(sweep) / 2.0)
    u = ((b[0] - a[0]) / c, (b[1] - a[1]) / c)
    side = 1.0 if (sweep > 0) == (abs(sweep) < math.pi) else -1.0
    centre = (m[0] - u[1] * h * side, m[1] + u[0] * h * side)
    return centre, r, math.atan2(a[1] - centre[1], a[0] - centre[0]), sweep


def lines(w):
    """The wall's lines: each a list of edges, ('seg', a, b) or ('arc', centre, r, a0, sweep)."""
    pts = [xy(p) for p in w["pts"]]
    closed = w.get("closed", False)
    out = []
    for d in boundaries(w):
        if curved(w):
            # Only a two-point arc wall here: its line is the arc moved `d` to its left.
            assert len(pts) == 2 and not closed
            c, r, a0, sw = bulge_arc(pts[0], pts[1], w["bulges"][0])
            out.append([("arc", c, r + (-d if sw > 0 else d), a0, sw)])
            continue
        line = offset_line(pts, d, closed)
        count = len(pts) if closed else len(pts) - 1
        out.append([("seg", line[i], line[(i + 1) % len(line)]) for i in range(count)])
    return out


def body(w):
    """The wall's body as rings (even-odd), or an arc wall's as a fine polygon."""
    ls = lines(w)
    first, last = ls[0], ls[-1]
    if curved(w):
        def ring(edge, n=256):
            _, c, r, a0, sw = edge
            return [(c[0] + r * math.cos(a0 + sw * i / n), c[1] + r * math.sin(a0 + sw * i / n)) for i in range(n + 1)]
        return [ring(first[0]) + list(reversed(ring(last[0])))]
    if w.get("closed"):
        return [[e[1] for e in first], [e[1] for e in last]]
    left = [first[0][1]] + [e[2] for e in first]
    right = [last[0][1]] + [e[2] for e in last]
    return [left + list(reversed(right))]


def corners(c):
    hw, hd = c["width"] / 2.0, c.get("depth", c["width"]) / 2.0
    rot = c.get("rotation", 0.0)
    s, k = math.sin(rot), math.cos(rot)
    at = xy(c["at"])
    return [(at[0] + x * k - y * s, at[1] + x * s + y * k) for x, y in [(-hw, -hd), (hw, -hd), (hw, hd), (-hw, hd)]]


def round_(c):
    return c.get("shape") == "round"


def off_segment(p, a, b):
    d = sub(b, a)
    l2 = dot(d, d)
    t = min(1.0, max(0.0, dot(sub(p, a), d) / l2)) if l2 > 0 else 0.0
    return dist(p, (a[0] + d[0] * t, a[1] + d[1] * t))


def in_rings(p, rings):
    inside = False
    for ring in rings:
        n = len(ring)
        for i in range(n):
            a, b = ring[i], ring[(i + 1) % n]
            if (a[1] > p[1]) != (b[1] > p[1]):
                x = a[0] + (p[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0])
                if p[0] < x:
                    inside = not inside
    return inside


def hides(e, p):
    if e["kind"] == "column":
        if round_(e):
            return dist(p, xy(e["at"])) < e["width"] / 2.0 - TIE
        rings = [corners(e)]
    else:
        rings = body(e)
    return in_rings(p, rings) and all(off_segment(p, r[i], r[(i + 1) % len(r)]) > TIE for r in rings for i in range(len(r)))


def seg_cross(f, t, a, b):
    """Where the cut f→t crosses the segment a→b, ends included: (point, cut fraction, edge fraction); none if not."""
    r, s = sub(t, f), sub(b, a)
    den = cross(r, s)
    if den == 0:
        return None
    tc = cross(sub(a, f), s) / den
    te = cross(sub(a, f), r) / den
    if 0.0 <= tc <= 1.0 and 0.0 <= te <= 1.0:
        return (f[0] + r[0] * tc, f[1] + r[1] * tc), tc, te
    return None


def line_circle(f, t, c, r):
    d = sub(t, f)
    aa = dot(d, d)
    s = dot(sub(c, f), d) / aa
    foot = (f[0] + d[0] * s - c[0], f[1] + d[1] * s - c[1])
    h2 = r * r - dot(foot, foot)
    if h2 < 0:
        return []
    h = math.sqrt(h2 / aa)
    return [s - h, s + h]


def arc_crossed(f, t, edge):
    _, c, r, a0, sw = edge
    for q in line_circle(f, t, c, r):
        if not 0.0 <= q <= 1.0:
            continue
        p = (f[0] + (t[0] - f[0]) * q, f[1] + (t[1] - f[1]) * q)
        th = math.atan2(p[1] - c[1], p[0] - c[0])
        d = (th - a0) if sw > 0 else (a0 - th)
        d %= 2 * math.pi
        if d <= abs(sw) + 1e-12:
            return True
    return False


# ── The chain ────────────────────────────────────────────────────────────────────────────────────────────────────

class Refused(Exception):
    def __init__(self, code, message, path):
        super().__init__(message)
        self.code, self.message, self.path = code, message, path


def sources(state, inp):
    out = []
    for i in sorted(state):
        e = state[i]
        if e["kind"] not in ("wall", "column") or e["layerId"] in HIDDEN:
            continue
        level = (e.get("arch") or {}).get("level")
        if "level" in inp and level != inp["level"]:
            continue
        out.append(i)
    return out


def references(state, inp):
    f, t = xy(inp["from"]), xy(inp["to"])
    length = dist(f, t)
    if not length >= SHORTEST:
        raise Refused("no_dimension", "Kesen çizgi çok kısa; ölçülecek duvarların içinden geçen bir çizgi çizin.", "to")
    u = ((t[0] - f[0]) / length, (t[1] - f[1]) / length)
    srcs = sources(state, inp)
    faces = inp.get("faces", "outer")
    columns = inp.get("columns", "ends")
    found, arcs = [], 0

    def hidden(i, p):
        return any(j != i and hides(state[j], p) for j in srcs)
    for order, i in enumerate(srcs):
        e = state[i]
        if e["kind"] == "wall":
            ls = lines(e)
            parts = [0, len(ls) - 1] if faces == "outer" else list(range(len(ls)))
            for part in parts:
                for index, edge in enumerate(ls[part]):
                    if edge[0] == "arc":
                        arcs += arc_crossed(f, t, edge)
                        continue
                    hit = seg_cross(f, t, edge[1], edge[2])
                    if hit is None or hidden(i, hit[0]):
                        continue
                    found.append({"source": order, "id": i, "place": {"kind": "along", "part": part, "ring": 0, "index": index,
                                                                       "t": hit[2]}, "at": hit[0], "s": hit[1] * length})
            continue
        if columns == "none":
            continue
        at = xy(e["at"])
        centre = {"source": order, "id": i, "place": {"kind": "center"}, "at": at, "s": dot(sub(at, f), u)}
        if round_(e):
            r = e["width"] / 2.0
            if any(0.0 <= q <= 1.0 for q in line_circle(f, t, at, r)) or dist(f, at) <= r:
                found.append(centre)
            continue
        cs = corners(e)
        sides = []
        for index in range(4):
            hit = seg_cross(f, t, cs[index], cs[(index + 1) % 4])
            if hit is not None:
                sides.append((index, hit))
        if columns == "ends":
            for index, hit in sides:
                if hidden(i, hit[0]):
                    continue
                found.append({"source": order, "id": i, "place": {"kind": "along", "part": 0, "ring": 0, "index": index,
                                                                   "t": hit[2]}, "at": hit[0], "s": hit[1] * length})
        elif sides or in_rings(f, [cs]):
            found.append(centre)

    def key(r):
        p = r["place"]
        return (r["s"], r["source"], p.get("part", 0), p.get("index", 0))
    found.sort(key=key)
    refs = []
    for r in found:
        if not refs or r["s"] - refs[-1]["s"] > TIE:
            refs.append(r)
    return refs, arcs


def chain(state, inp):
    """The dimensions written (each the stored entity but its id), and how many curved faces were left out."""
    for name in ("from", "to") + (("at",) if "at" in inp else ()):
        for axis in ("x", "y"):
            if not math.isfinite(inp[name][axis]):
                whose = {"from": "Kesen çizginin başının", "to": "Kesen çizginin sonunun", "at": "Zincirin yerinin"}[name]
                axis_name = "doğu (Y)" if axis == "x" else "kuzey (X)"
                raise Refused("not_finite", f"{whose} {axis_name} değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.",
                              f"{name}.{axis}")
    if "height" in inp and not (math.isfinite(inp["height"]) and inp["height"] > 0):
        raise Refused("invalid_height", "Ölçü yazısının yüksekliği sıfırdan büyük, sonlu bir sayı olmalı.", "height")
    if "expectedRevision" in inp and not inp["expectedRevision"].isdigit():
        raise Refused("invalid_revision", INVALID_REVISION, "expectedRevision")
    if "level" in inp and inp["level"] not in ("zemin", "kat-1"):
        raise Refused("level_not_found", f"Projede “{inp['level']}” kimlikli kat yok. Katı Proje ayarları › Mimari'de ekleyin ya da var olan bir katı seçin.", "level")
    style = inp.get("dimensionStyle")
    if style is None or style.strip().lower() == "standart":
        look, height = {}, STANDARD_HEIGHT
    elif style.strip().lower() == "mimari":
        look, height = {"dimStyle": MIMARI}, MIMARI_HEIGHT
    else:
        raise Refused("unknown_style", f"“{style.strip()}” adlı ölçü stili yok. Projenin ölçü stillerinden birinin adını verin ya da alanı kaldırın (Standart).",
                      "dimensionStyle")
    height = inp.get("height", height)
    refs, arcs = references(state, inp)
    if len(refs) < 2:
        raise Refused("no_dimension", f"Kesen çizgi ölçülecek {len(refs)} yüz kesiyor; zincir için en az iki gerekir. Duvarların ya da kolonların içinden geçen bir çizgi çizin.", "to")
    f, t = xy(inp["from"]), xy(inp["to"])
    angle = math.atan2(t[1] - f[1], t[0] - f[0]) * 180.0 / math.pi
    turn = angle * math.pi / 180.0
    n = (-math.sin(turn), math.cos(turn))
    through = xy(inp["at"]) if "at" in inp else f

    def across(p):
        return n[0] * (through[0] - p[0]) + n[1] * (through[1] - p[1])

    def ref(r):
        return {"uid": u(r["id"]), "place": r["place"]}

    def dimension(a, b, offset):
        d = {"kind": "dimension", "layerId": inp["layerId"], "attrs": {}, "a": P(*a["at"]), "b": P(*b["at"]), "offset": offset,
             "height": height, "refs": {"a": ref(a), "b": ref(b), "rule": "cross"}, "style": "linear", "angle": angle,
             "c": P(*f)}
        d.update(look)
        if inp.get("mask"):
            d["mask"] = True
        return d
    out = [dimension(a, b, across(a["at"])) for a, b in zip(refs, refs[1:])]
    if inp.get("total") and len(refs) > 2:
        side = 1.0
        if "at" in inp:
            side = -1.0 if n[0] * (through[0] - f[0]) + n[1] * (through[1] - f[1]) < 0 else 1.0
        gap = 8.0 * PLOT / 1000.0
        out.append(dimension(refs[0], refs[-1], across(refs[0]["at"]) + side * gap))
    return out, arcs


# ── Following: the rule `cross` ────────────────────────────────────────────────────────────────────────────────────

def edge_of(state, place_owner, place):
    """The straight edge a place names on its object as it is now: its two ends; none for a point place."""
    e = state[place_owner]
    if place["kind"] != "along":
        return None
    if e["kind"] == "column":
        cs = corners(e)
        k = place["index"]
        return cs[k], cs[(k + 1) % 4]
    edge = lines(e)[place["part"]][place["index"]]
    return (edge[1], edge[2]) if edge[0] == "seg" else None


def follow(state, d, changed):
    """A dimension of the chain after the objects in `changed` changed: a copy, or the same."""
    out = copy.deepcopy(d)
    owners = {name: int(d["refs"][name]["uid"].split(":")[1]) for name in ("a", "b")}
    if not any(o in changed for o in owners.values()):
        return d
    turn = d["angle"] * math.pi / 180.0
    u_, n = (math.cos(turn), math.sin(turn)), (-math.sin(turn), math.cos(turn))
    c = xy(d["c"])
    for name, owner in owners.items():
        r = out["refs"][name]
        if owner not in state:
            # Its object is gone: the expectation names it by its persistent id, no slot has it any more.
            r["broken"] = "object_gone"
            r["uid"] = uid(owner)
            continue
        edge = edge_of(state, owner, r["place"])
        if edge is None:
            out[name] = P(*xy(state[owner]["at"]))
            continue
        p, q = edge
        s = sub(q, p)
        den = cross(s, u_)
        if den == 0:
            r["broken"] = "no_such_place"
            continue
        te = cross(sub(c, p), u_) / den
        out[name] = P(p[0] + s[0] * te, p[1] + s[1] * te)
        if 0.0 <= te <= 1.0:
            r["place"] = dict(r["place"], t=te)
    a0, a1 = xy(d["a"]), xy(out["a"])
    out["offset"] = d["offset"] + n[0] * (a0[0] - a1[0]) + n[1] * (a0[1] - a1[1])
    return out


# ── The steps ────────────────────────────────────────────────────────────────────────────────────────────────────

def u(i):
    return f"$uidOf:{i}"


def refusal(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def warnings_of(arcs):
    if not arcs:
        return []
    return [{"code": "arch_curved_face_skipped",
             "message": f"Kesen çizgi {arcs} yaylı duvar yüzünü kesti; iç ölçü zinciri düz yüzleri ölçer, yaylılar atlandı.",
             "path": "to"}]


INVALID_REVISION = "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."


def layer_refusal(inp):
    if inp["layerId"] == "grup":
        return refusal("not_a_layer", "“Grup” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.", "layerId")
    if not 0 <= inp.get("lineWeight", 0) <= 100:
        return refusal("invalid_line_weight", "Çizgi kalınlığı 0 ile 100 mm arasında bir sayı olmalı (0 en ince çizgidir). Bir kalınlık ya da “Katmana göre” seçin.", "lineWeight")
    if inp["layerId"] == "kilitli":
        return refusal("layer_locked", "“Kilitli katman” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.", "layerId")
    if inp["layerId"] not in ("duvar", "kolon", "olcu", "cizim", "gizli", "donuk", "alt"):
        return refusal("layer_not_found", f"“{inp['layerId']}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId")
    return None


def step(op, inp, state, non_finite=None, ids=None):
    """The step, the state after it and the new dimensions by id."""
    ids = ids if ids is not None else sorted(state)
    s = {"op": op, "input": inp}
    if non_finite:
        s["nonFinite"] = non_finite
    try:
        dims, arcs = chain(state, wc.with_non_finite(inp, non_finite or {}))
    except Refused as r:
        s["result"] = refusal(r.code, r.message, r.path)
        s["expect"] = {"ids": ids, "revision": "same"}
        return s, state, {}
    bad = layer_refusal(inp)
    if bad is not None:
        s["result"] = bad
        s["expect"] = {"ids": ids, "revision": "same"}
        return s, state, {}
    warnings = warnings_of(arcs)
    if inp["layerId"] == "gizli":
        warnings = [{"code": "layer_hidden", "message": "“Gizli katman” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}] + warnings
    s["within"] = WITHIN
    if op == "validate":
        s["result"] = {"status": "completed", "output": None, "warnings": warnings}
        s["expect"] = {"ids": ids, "revision": "same"}
        return s, state, {}
    if op == "plan":
        s["result"] = {"status": "completed", "output": {"entities": [dict(d, id=0) for d in dims], "revision": "$current"},
                       "warnings": warnings}
        s["expect"] = {"ids": ids, "revision": "same"}
        return s, state, {}
    first = max(state) + 1
    new = {first + k: dict(d, id=first + k) for k, d in enumerate(dims)}
    s["result"] = {"status": "completed", "output": {"created": [u(i) for i in new], "ids": list(new), "revision": "$current"},
                   "warnings": warnings}
    after = {**state, **new}
    s["expect"] = {"ids": sorted(after), "entities": {str(i): d for i, d in new.items()}, "revision": "changed",
                   "canUndo": True, "canRedo": False, "dirty": True}
    return s, after, new


def inp(f, t, **extra):
    return {"from": P(*f), "to": P(*t), "layerId": extra.pop("layerId", "olcu"), "height": extra.pop("height", 0.15), **extra}


def initial():
    return {e["id"]: copy.deepcopy(e) for e in ENTITIES}


def cases():
    c = []

    def add(name, steps):
        assert name not in [x["name"] for x in c], name
        c.append({"name": name, "steps": steps})

    def one(name, i, op="execute", non_finite=None):
        s, _, _ = step(op, i, initial(), non_finite)
        add(name, [s])
    # Writing: one step, undone as one.
    s, _, new = step("execute", inp((-1, 2), (13, 2)), initial())
    add("üç oda soldan sağa: duvar kalınlıkları ve net genişlikler, yedi ölçü tek adım, kesen çizginin üstünde", [
        s, {"op": "undo", "returns": STEP, "expect": {"ids": IDS, "canUndo": False, "canRedo": True, "revision": "changed"}}])
    one("sağdan sola kesen çizgi: aynı yüzler ters sırayla, açı 180°", inp((13, 2), (-1, 2)))
    one("eğik kesen çizgi: değerler kesen çizgi boyunca", inp((-1, 1), (13, 3)))
    one("zincir kesen çizgiden 1 m yukarıda (at)", inp((-1, 2), (13, 2), at=P(0, 3)))
    one("toplam ölçü bir sıra ötede, at'nin yanında", inp((-1, 2), (13, 2), at=P(0, 1), total=True))
    one("toplam: at yokken kesen çizginin solunda", inp((-1, 2), (13, 2), total=True))
    one("bileşenli duvar: dış yüzler", inp((-1, 12), (5, 12)))
    one("bileşenli duvar: bütün katmanlar (skins)", inp((-1, 12), (5, 12), faces="skins"))
    one("dikdörtgen kolonun kesilen kenarları, yuvarlak kolonun merkezi (ends)", inp((-1, 22), (7, 22)))
    one("kolonların merkezleri (centers)", inp((-1, 22), (7, 22), columns="centers"))
    one("kolonlar ölçülmez (none)", inp((-1, 22), (7, 22), columns="none"))
    one("döndürülmüş dikdörtgen kolon", inp((-1, 92), (5, 92)))
    one("duvara gömülü kolonun yüzleri görünmez: yalnız duvar", inp((4, 29), (4, 31)))
    one("duvara gömülü kolonun merkezi ölçülür", inp((4, 29), (4, 31), columns="centers"))
    one("köşe bölgesinde içteki yüzler görünmez: dıştan dışa", inp((4.95, 39), (4.95, 45)))
    one("yaylı duvarın yüzleri atlanır ve söylenir; düzler ölçülür", inp((-3, 49.5), (9, 49.5)))
    one("yalnız zemin kat (level)", inp((-1, 62), (6, 62), level="zemin"))
    one("kat verilmezse bütün katların duvarları", inp((-1, 62), (6, 62)))
    one("gizli ve dondurulmuş katmandaki duvarlar kesilmez, kilitlideki ölçülür", inp((-1, 72), (7, 72)))
    one("kapalı dikdörtgen duvar: iki kenarının yüzleri", inp((-1, 82), (6, 82)))
    one("sağa hizalı duvarın yüzleri yolunun solunda", inp((-1, 102), (4, 102)))
    one("ölçü stili Mimari: yüksekliği ve görünüşü stilin", {k: v for k, v in inp((-1, 2), (13, 2), dimensionStyle="Mimari").items() if k != "height"})
    one("yükseklik verilmezse Standart'ın", {k: v for k, v in inp((-1, 2), (5, 2)).items() if k != "height"})
    one("zemin (mask)", inp((-1, 2), (5, 2), mask=True))
    one("plan yazmadan gösterir", inp((-1, 2), (5, 2)), op="plan")
    one("doğrula yazmaz", inp((-1, 2), (5, 2)), op="validate")
    # Refusals.
    one("kısa kesen çizgi reddedilir", inp((2, 2), (2.0005, 2)))
    one("tek yüz kesen çizgi reddedilir", inp((-1, 2), (0, 2)))
    one("hiçbir şey kesmeyen çizgi reddedilir", inp((20, 2), (30, 2)))
    one("bilinmeyen kat reddedilir", inp((-1, 2), (5, 2), level="cati"))
    one("bilinmeyen ölçü stili reddedilir", inp((-1, 2), (5, 2), dimensionStyle="Yok"))
    one("sıfır yükseklik reddedilir", inp((-1, 2), (5, 2), height=0))
    one("sonsuz koordinat reddedilir", inp((-1, 2), (5, 2)), non_finite={"from.x": "NaN"})
    one("olmayan katman reddedilir", inp((-1, 2), (5, 2), layerId="yok"))
    one("kilitli katmana yazılmaz", inp((-1, 2), (5, 2), layerId="kilitli"))
    one("katman grubuna yazılmaz", inp((-1, 2), (5, 2), layerId="grup"))
    one("aralık dışı çizgi kalınlığı reddedilir", inp((-1, 2), (5, 2), lineWeight=200))
    one("gizli katmana yazılır, söylenir", inp((-1, 2), (5, 2), layerId="gizli"))
    one("beklenen sürüm sayı değil", inp((-1, 2), (5, 2), expectedRevision="on iki"))
    s, after, new = step("execute", inp((-1, 2), (5, 2)), initial())
    add("beklenen sürüm geçmişte kaldıysa conflict, hiçbir şey yazılmaz", [
        {"op": "captureRevision", "as": "old"}, s,
        {"op": "execute", "input": {**inp((-1, 2), (5, 2)), "expectedRevision": "$old"},
         "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision",
                                                     "revision": "$current"}},
         "expect": {"ids": sorted(after), "revision": "same"}}])
    # Following: Kaydır (ADR 0710) moves the crossed wall; the chain's line stays.
    add("ortadaki duvar 30 cm doğuya: iki ölçünün değeri değişir, çizgi yerinde", follow_steps(inp((-1, 2), (13, 2)), 2, -0.3))
    add("kesilen duvar boyunca uzayınca noktalar kesen çizgide kalır, t değişir", follow_steps(inp((-1, 2), (13, 2)), 6, -0.5))
    add("eğik zincirde duvar kaydırılınca noktalar kesen çizgide kayar, çizgi yerinde", follow_steps(inp((-1, 1), (13, 3)), 3, 0.4))
    add("kesilen duvar silinince ölçülerinin bağı kopar, yerlerinde kalırlar", delete_steps(inp((-1, 2), (13, 2)), 3))
    return c


def follow_steps(i, wall_id, by):
    state = initial()
    s, state, new = step("execute", i, state)
    moved = ws.shift(state, wall_id, 0, by)
    changed = {j: dict(state[j], pts=[P(*p) for p in pts]) for j, pts in moved.items()}
    after = {**state, **changed}
    followed = {k: follow(after, d, changed) for k, d in new.items()}
    expect = {str(k): d for k, d in followed.items() if d != new[k]}
    expect.update({str(j): e for j, e in changed.items()})
    shift = {"op": "execute", "command": "cad.walls.edit",
             "input": {"uids": [u(wall_id)], "operation": {"kind": "shift", "by": by}},
             "result": {"status": "completed", "output": "$any", "warnings": "$any"},
             "within": WITHIN, "expect": {"ids": sorted(after), "entities": expect, "revision": "changed"}}
    return [s, shift]


def delete_steps(i, wall_id):
    state = initial()
    s, state, new = step("execute", i, state)
    after = {k: v for k, v in state.items() if k != wall_id}
    followed = {k: follow(after, d, {wall_id}) for k, d in new.items()}
    expect = {str(k): d for k, d in followed.items() if d != new[k]}
    gone = {"op": "execute", "command": "cad.entities.delete", "input": {"uids": [u(wall_id)]},
            "result": {"status": "completed", "output": "$any", "warnings": "$any"}, "within": WITHIN, "expect": {"ids": sorted(after), "entities": expect, "revision": "changed"}}
    return [s, gone]


NOTE = ("ADR 0711. Beklenenler scripts/fixtures/interior_dims_cases.py'nin ADR'den yazılmış kurallarından, KentOS kodu olmadan; "
        "izleme adımlarında Kaydır scripts/fixtures/wall_shift_cases.py'nindir (ADR 0710).")


def files():
    return {"cad.dimensions.interior.json": {
        "format": "kentos.command-cases", "version": 1, "command": COMMAND, "commandVersion": 1,
        "title": "İç ölçü zinciri: kesen çizginin kestiği yüzler sırayla, kolonlar, gizli yüzler, katlar ve katmanlar, kesen çizgi kuralıyla izleme; retler",
        "note": NOTE, "setup": SETUP, "cases": cases()}}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    bad = 0
    for name, doc in files().items():
        path = OUT / name
        text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
        if args.check:
            if not path.exists() or path.read_text() != text:
                print(f"{path.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
                bad += 1
            else:
                print(f"{name}: {len(doc['cases'])} durum başvuruyla aynı.")
            continue
        path.write_text(text)
        print(f"{path.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
