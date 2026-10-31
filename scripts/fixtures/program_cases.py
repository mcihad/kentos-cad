#!/usr/bin/env python3
"""The building program (docs/adr/0773), from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.program.edit.json, cad.program.check.json and cad.spaces.edit.decisions.json; `--check`
compares and writes nothing. Run with shapely: `uv run --no-project --with shapely python3 scripts/fixtures/program_cases.py
--check` (the corridor's passage erodes its rings; shapely is GEOS, not KentOS).

The rules, as the ADR gives them:

- A space is in one group of the program or none (`unit`); a group is an independent unit, a common area or an outdoor
  area. A group's totals sum each of its spaces once: its rooms, its outdoor spaces (whose brief says `outdoor`) and its
  voids; the drawing's totals by kind of group, the spaces in no group (or in one the program lacks) apart; `all` is
  every space but the voids. A group's levels are its spaces' in the project's order; its methods its spaces' in their
  order of first use; its recorded area is read, never written, its difference the rooms less it.
- A space's area is its rings' (outer less holes). Its net width is the diameter of the largest circle inside it, its
  `at` that circle's centre (a rectangle's middle).
- Its doors: each door (or a gap without a sill) on a wall whose face toward the space a side of the space runs along,
  at the door's middle on that face; and each other space on its level whose side lies on one of its sides (within
  1 mm, facing, at least 0.3 m along each other: a room separator), at the middle of that stretch. Two spaces open into
  each other through such a stretch or a door whose two faces they lie on. Two spaces are neighbours when their facing
  sides lie at most 0.6 m apart for at least 0.3 m.
- With a brief: the difference from its area range (below minus, above plus, inside 0), its net width against
  `minWidth`, its usages wanted as neighbours (`adjacent`) and opened into (`reach`), each met by the spaces of the
  usage (in the drawing's order). With `minPassage` and two doors or more, its net passage: the widest a body may be and
  still go between all its doors — its rings eroded by half that, every door within half that (and a micron) of one
  piece, the nearest; its own net width when that holds within a tenth of a millimetre. Where it is narrowest: the middle
  of the closest points of the two facing, not touching, straight sides whose distance is nearest the passage.
- The findings (`program`): a group the program lacks (fixed by `cad.spaces.edit` unit null), an area out of range, a
  net width or a passage below the brief by more than a tenth of a millimetre, a neighbour or a reach missing, a room of
  a unit or a common area with no door, a fixture whose clearance leaves its space or covers another fixture; in that
  order, then along x and y. The words name the measured value, the target and the rule with the brief's version.
"""

import argparse
import copy
import json
import math
import sys
from pathlib import Path

try:
    from shapely.geometry import LineString, Polygon, Point
    from shapely.ops import unary_union  # noqa: F401
except ImportError:  # pragma: no cover
    sys.exit("shapely gerekli: uv run --no-project --with shapely python3 scripts/fixtures/program_cases.py")

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
RULE = "kentos.program/1"
VERSION = "Ofis konut hedefleri 2026-1"
PRECISION = 1e-4
OPEN_GAP, ADJ_GAP, OVERLAP = 0.001, 0.6, 0.3
LABEL = {"net": "Net (iç yüzler)", "centerline": "Duvar ekseni", "gross": "Brüt (dış yüzler)"}
LEVELS = [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
          {"id": "kat-1", "name": "1. kat", "elevation": 3.0, "height": 3.0}]


def P(x, y):
    return {"x": float(x), "y": float(y)}


def rect(x0, y0, x1, y1):
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]


def fx(v):
    """Two decimals, as KentOS shows them; −0.00 is 0.00."""
    t = f"{v:.2f}"
    return "0.00" if t == "-0.00" else t


def m2(v):
    return f"{fx(v)} m²"


def m(v):
    return f"{fx(v)} m"


def signed(v):
    return f"+{m2(v)}" if v > 0 else m2(v)


def layer(i, name, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


LAYERS = [layer("duvar", "Duvar"), layer("mahal", "Mahal"), layer("kapi", "Kapı"), layer("donati", "Donatı"),
          layer("etiket", "Etiket"), layer("kilitli", "Kilitli", True)]

# ── The acceptance plan: two units on two levels, a shared corridor, an open kitchen, a shaft and a balcony ─────────

WALLS = {  # id: (a, b, level)
    1: ((0, 0), (13, 0), "zemin"), 2: ((13, 0), (13, 8), "zemin"), 3: ((13, 8), (0, 8), "zemin"),
    4: ((0, 8), (0, 0), "zemin"), 5: ((5, 0), (5, 8), "zemin"), 6: ((6.4, 0), (6.4, 8), "zemin"),
    7: ((0, 3.4), (5, 3.4), "zemin"), 8: ((6.4, 4.5), (13, 4.5), "zemin"), 9: ((10, 0), (10, 4.5), "zemin"),
    10: ((5, 7), (6.4, 7), "zemin"),
    11: ((0, 0), (5, 0), "kat-1"), 12: ((5, 0), (5, 4), "kat-1"), 13: ((5, 4), (0, 4), "kat-1"),
    14: ((0, 4), (0, 0), "kat-1"), 15: ((3, 0), (3, 4), "kat-1"),
}
THICK = 0.2


def space(pts, level=None, unit=None, usage=None, number=None, name=None, void=False, holes=(), method=None,
          layer_id="mahal"):
    return {"pts": pts, "holes": [list(h) for h in holes], "level": level, "unit": unit, "usage": usage,
            "number": number, "name": name if name is not None else usage, "void": void, "method": method or "net",
            "layer": layer_id}


SPACES = {
    16: space(rect(0.1, 0.1, 4.9, 3.3), "zemin", "d1", "Yatak odası", "101"),
    17: space(rect(0.1, 3.5, 3.0, 7.9), "zemin", "d1", "Salon", "102"),
    18: space(rect(3.0, 3.5, 4.9, 7.9), "zemin", "d1", "Mutfak", "103"),
    19: space(rect(5.1, 0.1, 6.3, 6.9), "zemin", "ortak", "Koridor", "001"),
    20: space(rect(5.1, 7.1, 6.3, 7.9), "zemin", "ortak", "Şaft", "002", void=True),
    21: space(rect(6.5, 0.1, 9.9, 4.4), "zemin", "d2", "Yatak odası", "201"),
    22: space(rect(10.1, 0.1, 12.9, 4.4), "zemin", "d2", "Banyo", "202"),
    23: space(rect(6.5, 4.6, 12.9, 7.9), "zemin", "d2", "Salon", "203"),
    24: space(rect(0.1, 8.1, 3.0, 9.5), "zemin", "d1", "Balkon", "104"),
    25: space(rect(0.1, 0.1, 2.9, 3.9), "kat-1", "d1", "Yatak odası", "105"),
    26: space(rect(3.1, 0.1, 4.9, 3.9), "kat-1", "d1", "Hol", "106"),
    27: space(rect(14, 0, 18, 8), None, "bahce", "Bahçe", "B1"),
    28: space(rect(14, 10, 16, 12), None, None, "Depo", "D"),
    29: space(rect(20, 0, 22, 2), None, None, "Arşiv", "K", layer_id="kilitli"),
}
DOORS = [(5, 1.5, 0.9), (5, 6.0, 0.9), (6, 2.0, 0.9), (6, 6.0, 0.9), (9, 2.0, 0.9), (1, 5.7, 1.0), (3, 11.5, 0.9),
         (7, 1.5, 0.9), (15, 2.0, 0.9)]
FIRST_DOOR = 30

PROGRAM = {
    "buildings": [{"id": "a", "name": "A Blok", "wings": [{"id": "a-dogu", "name": "Doğu kanadı"}]}],
    "units": [
        {"id": "d1", "number": "1", "name": "Daire 1", "building": "a", "recordedArea": 60.0},
        {"id": "d2", "number": "2", "name": "Daire 2", "building": "a", "wing": "a-dogu"},
        {"id": "ortak", "number": "O", "name": "Ortak alan", "kind": "common", "building": "a"},
        {"id": "bahce", "number": "B", "name": "Bahçe", "kind": "outdoor"},
    ],
    "briefs": [
        {"usage": "Yatak odası", "areaMin": 12.0, "areaMax": 16.0, "minWidth": 2.8, "reach": ["Salon"]},
        {"usage": "Salon", "areaMin": 18.0, "areaMax": 30.0, "adjacent": ["Mutfak"]},
        {"usage": "Mutfak", "areaMin": 6.0, "areaMax": 12.0, "reach": ["Salon"]},
        {"usage": "Koridor", "minWidth": 1.1, "minPassage": 1.2},
        {"usage": "Banyo", "areaMin": 4.0, "areaMax": 8.0},
        {"usage": "Balkon", "outdoor": True},
    ],
    "version": VERSION,
}


def wall_entity(i, a, b, level, width=THICK):
    return {"kind": "wall", "id": i, "layerId": "duvar", "attrs": {}, "pts": [P(*a), P(*b)], "width": width,
            "justify": "center", "arch": {"level": level}}


def space_entity(i, s):
    e = {"kind": "space", "id": i, "layerId": s["layer"], "attrs": {},
         "parts": [{"outer": {"pts": [P(*q) for q in s["pts"]]}}]}
    if s["holes"]:
        e["parts"][0]["holes"] = [{"pts": [P(*q) for q in h]} for h in s["holes"]]
    if s["method"] != "net":
        e["method"] = s["method"]
    for k in ("number", "name", "usage"):
        if s[k] is not None:
            e[k] = s[k]
    if s["level"]:
        e["arch"] = {"level": s["level"]}
    if s["void"]:
        e["void"] = True
    if s["unit"]:
        e["unit"] = s["unit"]
    return e


def setup(walls=WALLS, spaces=SPACES, program=PROGRAM, extra=(), blocks=None, name="Bina programı"):
    arch = {"levels": LEVELS}
    if program:
        arch["program"] = program
    doc = {"format": "kentos.document", "version": 1, "name": name,
           "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                        "plotScale": 50, "workspace": "cad", "arch": arch},
           "origin": {"x": 0, "y": 0}, "layers": copy.deepcopy(LAYERS), "activeLayer": "mahal", "styles": {"items": [], "categories": []},
           "entities": [wall_entity(i, a, b, lv) for i, (a, b, lv) in sorted(walls.items())]
                       + [space_entity(i, s) for i, s in sorted(spaces.items())] + list(extra)}
    if blocks:
        doc["blocks"] = blocks
    return doc


def door_steps(walls=WALLS, doors=DOORS):
    return [{"op": "execute", "command": "cad.openings.create",
             "input": {"layerId": "kapi", "openings": [{"host": f"$uidOf:{w}", "at": at, "form": "door", "width": wd}
                                                        for w, at, wd in doors]},
             "result": {"status": "completed", "output": "$any", "warnings": "$any"},
             "expect": {"revision": "changed"}}] if doors else []


# ── Geometry ─────────────────────────────────────────────────────────────────────────────────────────────────────


def signed_area(pts):
    return sum(pts[i][0] * pts[(i + 1) % len(pts)][1] - pts[(i + 1) % len(pts)][0] * pts[i][1] for i in range(len(pts))) / 2


def area_of(s):
    return abs(signed_area(s["pts"])) - sum(abs(signed_area(h)) for h in s["holes"])


def sides_of(s):
    out = []
    for h, ring in enumerate([s["pts"]] + s["holes"]):
        keep = (signed_area(ring) > 0) == (h == 0)
        for i in range(len(ring)):
            a, b = ring[i], ring[(i + 1) % len(ring)]
            if a != b:
                out.append((a, b) if keep else (b, a))
    return out


def polygon(s):
    return Polygon(s["pts"], s["holes"])


def dist_to_boundary(s, x, y):
    return polygon(s).exterior.distance(Point(x, y)) if not s["holes"] else polygon(s).boundary.distance(Point(x, y))


def width_of(s):
    """The largest circle inside: a rectangle's middle exactly; else polylabel to a micron."""
    pts = s["pts"]
    xs, ys = [p[0] for p in pts], [p[1] for p in pts]
    if len(pts) == 4 and not s["holes"] and len(set(xs)) == 2 and len(set(ys)) == 2:
        return min(max(xs) - min(xs), max(ys) - min(ys)), ((min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2)
    poly = polygon(s)
    from shapely.ops import polylabel
    c = polylabel(poly, tolerance=1e-7)
    return 2 * poly.boundary.distance(c), (c.x, c.y)


def facing(a_sides, b_sides, gap):
    best = None
    for s0, t0 in a_sides:
        dx, dy = t0[0] - s0[0], t0[1] - s0[1]
        ln = math.hypot(dx, dy)
        ux, uy = dx / ln, dy / ln
        ox, oy = uy, -ux
        for p, q in b_sides:
            ex, ey = q[0] - p[0], q[1] - p[1]
            el = math.hypot(ex, ey)
            if abs(ux * ey - uy * ex) / el > 1e-9 or ux * ex + uy * ey >= 0:
                continue
            off = ox * (p[0] - s0[0]) + oy * (p[1] - s0[1])
            if not (-OPEN_GAP <= off <= gap + 1e-9):
                continue
            k0, k1 = ux * (p[0] - s0[0]) + uy * (p[1] - s0[1]), ux * (q[0] - s0[0]) + uy * (q[1] - s0[1])
            lo, hi = max(min(k0, k1), 0), min(max(k0, k1), ln)
            over = hi - lo
            if over + 1e-9 < OVERLAP:
                continue
            def at(k):
                return (s0[0] + ux * k + ox * off / 2, s0[1] + uy * k + oy * off / 2)
            if best is None or over > best[0]:
                best = (over, (at(lo), at(hi)))
    return None if best is None else best[1]


def door_faces(walls, doors):
    """Each door: its id, its level, its two faces (a point on each, the axis' direction, half its span)."""
    out = []
    for k, (w, at, wd) in enumerate(doors):
        a, b, level = walls[w]
        dx, dy = b[0] - a[0], b[1] - a[1]
        ln = math.hypot(dx, dy)
        ux, uy = dx / ln, dy / ln
        c = (a[0] + ux * at, a[1] + uy * at)
        nx, ny = -uy, ux
        faces = [(c[0] + nx * THICK / 2, c[1] + ny * THICK / 2), (c[0] - nx * THICK / 2, c[1] - ny * THICK / 2)]
        out.append((FIRST_DOOR + k, level, faces, (ux, uy), wd / 2))
    return out


def span(face, u, half):
    """A door's span on a face: from jamb to jamb."""
    return ((face[0] - u[0] * half, face[1] - u[1] * half), (face[0] + u[0] * half, face[1] + u[1] * half))


def on_face(s, face, u, half):
    """Whether a side of the space runs along the door's face (within a micron) over its span."""
    for p, q in sides_of(s):
        ex, ey = q[0] - p[0], q[1] - p[1]
        el = math.hypot(ex, ey)
        if abs(u[0] * ey - u[1] * ex) / el > 1e-6:
            continue
        off = abs(u[0] * (p[1] - face[1]) - u[1] * (p[0] - face[0]))
        if off > 1e-6:
            continue
        k0, k1 = u[0] * (p[0] - face[0]) + u[1] * (p[1] - face[1]), u[0] * (q[0] - face[0]) + u[1] * (q[1] - face[1])
        if min(max(k0, k1), half) - max(min(k0, k1), -half) > 0:
            return True
    return False


def reads(level, other):
    return level is None or level == other


def eroded_connected(poly, doors, w):
    r = w / 2
    er = poly.buffer(-r, quad_segs=64) if r > 0 else poly
    if er.is_empty:
        return False
    pieces = list(er.geoms) if er.geom_type == "MultiPolygon" else [er]
    seen = None
    for d in doors:
        pt = LineString(d)
        near = [(pc.distance(pt), i) for i, pc in enumerate(pieces)]
        near = [x for x in near if x[0] <= r + 1e-6]
        if not near:
            return False
        i = min(near)[1]
        if seen is None:
            seen = i
        elif seen != i:
            return False
    return True


def passage_of(s, doors, width):
    poly = polygon(s)
    if eroded_connected(poly, doors, width - PRECISION):
        return width
    lo, hi = 0.0, width
    while hi - lo > 1e-8:
        mid = (lo + hi) / 2
        if eroded_connected(poly, doors, mid):
            lo = mid
        else:
            hi = mid
    return lo


def seg_closest(a, b):
    (ax, ay), (bx, by) = a
    (cx, cy), (ex, ey) = b
    dx, dy, fx_, fy = bx - ax, by - ay, ex - cx, ey - cy
    la, lb = math.hypot(dx, dy), math.hypot(fx_, fy)

    def on(seg, p):
        (x0, y0), (x1, y1) = seg
        vx, vy = x1 - x0, y1 - y0
        t = max(0, min(1, ((p[0] - x0) * vx + (p[1] - y0) * vy) / (vx * vx + vy * vy)))
        q = (x0 + vx * t, y0 + vy * t)
        return math.hypot(p[0] - q[0], p[1] - q[1]), q
    if abs(dx * fy - dy * fx_) <= 1e-12 * la * lb:
        ux, uy = dx / la, dy / la
        k0, k1 = ux * (cx - ax) + uy * (cy - ay), ux * (ex - ax) + uy * (ey - ay)
        lo, hi = max(min(k0, k1), 0), min(max(k0, k1), la)
        if hi >= lo:
            k = (lo + hi) / 2
            p = (ax + ux * k, ay + uy * k)
            d, q = on(b, p)
            return d, p, q
    best = None
    for frm, to, flip in ((a[0], b, False), (a[1], b, False), (b[0], a, True), (b[1], a, True)):
        d, q = on(to, frm)
        if best is None or d < best[0]:
            best = (d, q, frm) if flip else (d, frm, q)
    return best


def pinch(sides, width):
    best = None
    for i, a in enumerate(sides):
        for b in sides[i + 1:]:
            if a[0] in b or a[1] in b:
                continue
            d, p, q = seg_closest(a, b)
            if d <= 1e-9:
                continue

            def left(sg, x):
                return (sg[1][0] - sg[0][0]) * (x[1] - sg[0][1]) - (sg[1][1] - sg[0][1]) * (x[0] - sg[0][0]) > 1e-12
            if not (left(a, q) and left(b, p)):
                continue
            off = abs(d - width)
            if best is None or off < best[0] - 1e-12:
                best = (off, ((p[0] + q[0]) / 2, (p[1] + q[1]) / 2))
    return None if best is None else best[1]


def words(s):
    n, w = s["number"], s["name"] or s["usage"]
    if n and w:
        return f"“{n} {w}”"
    if n:
        return f"“{n}”"
    if w:
        return f"“{w}”"
    return "Mahal"


# ── The check ────────────────────────────────────────────────────────────────────────────────────────────────────


def check(walls, spaces, program, doors=(), fixtures=(), units_filter=None, levels_filter=None, findings_on=True,
          level_order=("zemin", "kat-1")):
    """`cad.program.check`'s output, the drawing's spaces in their ids' order."""
    rule = RULE + (f" · {program['version']}" if program.get("version") else "")
    units = {u["id"]: u for u in program.get("units", [])}
    briefs = {b["usage"]: b for b in program.get("briefs", [])}
    rooms = []
    for i, s in sorted(spaces.items()):
        if units_filter is not None and s["unit"] not in units_filter:
            continue
        if levels_filter is not None and s["level"] not in levels_filter:
            continue
        rooms.append((i, s))
    n = len(rooms)
    faces = door_faces(walls, list(doors))
    dpts = [[] for _ in range(n)]
    by_door = {}
    for ri, (i, s) in enumerate(rooms):
        for k, (did, lvl, fcs, u, half) in enumerate(faces):
            if not reads(s["level"], lvl):
                continue
            for side, f in enumerate(fcs):
                if on_face(s, f, u, half):
                    dpts[ri].append(span(f, u, half))
                    by_door.setdefault(k, []).append((ri, side))
                    break
    opn = [[] for _ in range(n)]
    near = [[] for _ in range(n)]
    conn = [[] for _ in range(n)]
    for a in range(n):
        for b in range(a + 1, n):
            if rooms[a][1]["level"] != rooms[b][1]["level"]:
                continue
            sa, sb = sides_of(rooms[a][1]), sides_of(rooms[b][1])
            mid = facing(sa, sb, OPEN_GAP)
            if mid is not None:
                opn[a].append(mid)
                opn[b].append(mid)
                conn[a].append(b)
                conn[b].append(a)
            if facing(sa, sb, ADJ_GAP) is not None:
                near[a].append(b)
                near[b].append(a)
    for lst in by_door.values():
        for x in range(len(lst)):
            for y in range(x + 1, len(lst)):
                (i, si), (j, sj) = lst[x], lst[y]
                if i != j and si != sj and j not in conn[i]:
                    conn[i].append(j)
                    conn[j].append(i)
    rows, found = [], []
    for ri, (i, s) in enumerate(rooms):
        brief = briefs.get(s["usage"]) if s["usage"] else None
        width, at = width_of(s)
        pts = dpts[ri] + opn[ri]
        passage = at_p = None
        if brief and brief.get("minPassage") is not None and findings_on and len(pts) >= 2 and not s["void"]:
            passage = passage_of(s, pts, width)
            at_p = pinch(sides_of(s), passage)
        area = area_of(s)
        diff = None
        if brief and not s["void"] and (brief.get("areaMin") is not None or brief.get("areaMax") is not None):
            lo, hi = brief.get("areaMin"), brief.get("areaMax")
            diff = area - lo if lo is not None and area < lo else area - hi if hi is not None and area > hi else 0.0

        def links(key, among):
            return [{"usage": u, **({"by": [f"$uidOf:{rooms[j][0]}" for j in sorted(among, key=lambda j: rooms[j][0])
                                           if rooms[j][1]["usage"] == u]}
                                    if any(rooms[j][1]["usage"] == u for j in among) else {})}
                    for u in (brief or {}).get(key, [])]
        adjacent, reach = links("adjacent", near[ri]), links("reach", conn[ri])
        uid = f"$uidOf:{i}"
        wd = words(s)
        if findings_on:
            if s["unit"] and s["unit"] not in units:
                found.append(("program_unit_missing", [uid], at,
                              f"{wd} “{s['unit']}” bölümüne bağlı; bina programında bu bölüm yok.",
                              "Bölümü Bina programına ekleyin ya da mahali bölümden çıkarın.",
                              {"label": "Bölümden çıkar", "step": {"command": "cad.spaces.edit",
                                                                    "input": {"uids": [uid], "operation": {"kind": "set", "unit": None}}}}))
            if brief and diff not in (None, 0.0):
                lo, hi = brief.get("areaMin"), brief.get("areaMax")
                rng = (f"{fx(lo)}–{m2(hi)}" if lo is not None and hi is not None else
                       f"en az {m2(lo)}" if lo is not None else f"en çok {m2(hi)}")
                code = "program_area_below" if diff < 0 else "program_area_above"
                rep = ("Mahali büyütün ya da tasarım hedefini değiştirin." if diff < 0 else
                       "Mahali küçültün ya da tasarım hedefini değiştirin.")
                found.append((code, [uid], at,
                              f"{wd}: {LABEL[s['method']]} alan {m2(area)}; tasarım hedefi {rng}, fark {signed(diff)} ({rule}; yasal sınır değil).",
                              rep, None))
            if brief and brief.get("minWidth") is not None and not s["void"] and width < brief["minWidth"] - PRECISION:
                mw = brief["minWidth"]
                found.append(("program_width_narrow", [uid], at,
                              f"{wd}: net en {m(width)}; tasarım hedefi en az {m(mw)}, fark {m(width - mw)} ({rule}).",
                              "Mahali genişletin ya da tasarım hedefini değiştirin.", None))
            if passage is not None and passage < brief["minPassage"] - PRECISION:
                mp = brief["minPassage"]
                found.append(("program_passage_narrow", [uid], at_p or at,
                              f"{wd}: kapılar arası net geçiş {m(passage)}; tasarım hedefi en az {m(mp)}, fark {m(passage - mp)} ({rule}).",
                              "Dar yeri genişletin (duvarı, kolonu ya da donatıyı kaydırın) ya da tasarım hedefini değiştirin.", None))
            for lk in adjacent:
                if "by" not in lk:
                    found.append(("program_adjacent_missing", [uid], at,
                                  f"{wd}: tasarım hedefi “{lk['usage']}” komşusu istiyor; yanında öyle bir mahal yok ({rule}).",
                                  "Komşu mahali bu mahalin yanına alın ya da tasarım hedefini değiştirin.", None))
            for lk in reach:
                if "by" not in lk:
                    found.append(("program_reach_missing", [uid], at,
                                  f"{wd}: tasarım hedefi “{lk['usage']}” mahaline geçiş istiyor; kapı ya da açık geçiş yok ({rule}).",
                                  "Aradaki duvara kapı koyun ya da mahalleri ayırıcıyla açın.", None))
            grouped = s["unit"] in units and units[s["unit"]].get("kind", "unit") != "outdoor"
            if grouped and not s["void"] and not pts:
                found.append(("program_no_door", [uid], at, f"{wd}: kapısı ya da başka mahale açık geçişi yok ({rule}).",
                              "Mahale kapı koyun ya da komşusuna mahal ayırıcıyla açın.", None))
        row = {"uid": uid}
        for k in ("number", "name", "usage", "unit"):
            if s[k] is not None:
                row[k] = s[k]
        if s["level"]:
            row["level"] = s["level"]
        row["method"] = s["method"]
        if s["void"]:
            row["void"] = True
        if brief and brief.get("outdoor"):
            row["outdoor"] = True
        row.update(area=area, width=width, doors=len(pts))
        if passage is not None:
            row["passage"] = passage
            if at_p is not None:
                row["passageAt"] = P(*at_p)
        if brief:
            for k, key in (("areaMin", "targetMin"), ("areaMax", "targetMax")):
                if brief.get(k) is not None:
                    row[key] = brief[k]
        if diff is not None:
            row["targetDifference"] = diff
        if passage is not None and brief.get("minPassage") is not None:
            row["passageDifference"] = passage - brief["minPassage"]
        if adjacent:
            row["adjacent"] = adjacent
        if reach:
            row["reach"] = reach
        rows.append((i, s, row))
    # Fixtures' clearances.
    if findings_on and program.get("clearances"):
        boxes = []
        for f in fixtures:
            boxes.append(f)
        for k, f in enumerate(boxes):
            if f["zone"] is None:
                continue
            room = next((s for _, s in rooms if s["level"] == f["level"] and not s["void"]
                         and polygon(s).buffer(1e-9).contains(Point(f["middle"]))), None)
            if room is None:
                continue
            zone = Polygon(f["zone"])
            whole = zone.area
            out = max(whole - zone.intersection(polygon(room)).area, 0.0)
            others = [g["uid"] for j, g in enumerate(boxes)
                      if j != k and g["level"] == f["level"] and zone.intersection(Polygon(g["foot"])).area > 1e-6]
            if out > 1e-6 or others:
                bits = []
                if out > 1e-6:
                    bits.append(f"{m2(out)} mahalin dışında")
                if others:
                    bits.append(f"{len(others)} donatıya biniyor")
                cx = sum(p[0] for p in f["zone"]) / 4
                cy = sum(p[1] for p in f["zone"]) / 4
                found.append(("program_fixture_clearance", [f["uid"]] + others, (cx, cy),
                              f"Donatının kullanım alanı ({m2(whole)}) {', '.join(bits)}; {words(room)} içinde ({rule}; tasarım hedefi).",
                              "Donatıyı kaydırın ya da döndürün, önündeki alanı boşaltın.", None))
    # Totals.
    urows = []
    for u in program.get("units", []):
        urows.append({"id": u["id"], "number": u["number"], **({"name": u["name"]} if u.get("name") else {}),
                      "kind": u.get("kind", "unit"),
                      **({"building": u["building"]} if u.get("building") else {}),
                      **({"wing": u["wing"]} if u.get("wing") else {}),
                      "levels": [], "spaces": 0, "rooms": 0.0, "outdoor": 0.0, "voids": 0.0, "methods": [],
                      **({"recordedArea": u["recordedArea"]} if u.get("recordedArea") is not None else {})})
    tot = dict(units=0.0, unitOutdoor=0.0, common=0.0, outdoor=0.0, voids=0.0, unassigned=0.0, all=0.0)
    for i, s, row in rows:
        g = next((u for u in urows if u["id"] == s["unit"]), None)
        a = row["area"]
        if not s["void"]:
            tot["all"] += a
        if g is None:
            if s["void"]:
                tot["voids"] += a
            else:
                tot["unassigned"] += a
            continue
        g["spaces"] += 1
        if s["level"] and s["level"] not in g["levels"]:
            g["levels"].append(s["level"])
        if s["method"] not in g["methods"]:
            g["methods"].append(s["method"])
        outdoor = row.get("outdoor", False)
        if s["void"]:
            g["voids"] += a
            tot["voids"] += a
        elif outdoor:
            g["outdoor"] += a
        else:
            g["rooms"] += a
        if not s["void"]:
            kind = g["kind"]
            key = ("unitOutdoor" if outdoor else "units") if kind == "unit" else kind
            tot[key] += a
    for g in urows:
        g["levels"].sort(key=lambda lv: level_order.index(lv) if lv in level_order else 99)
        if not g["levels"]:
            del g["levels"]
        if "recordedArea" in g:
            g["recordedDifference"] = g["rooms"] - g["recordedArea"]
    if units_filter is not None:
        urows = [g for g in urows if g["id"] in units_filter]
    order = ["program_unit_missing", "program_area_below", "program_area_above", "program_width_narrow",
             "program_passage_narrow", "program_adjacent_missing", "program_reach_missing", "program_no_door",
             "program_fixture_clearance"]
    found.sort(key=lambda f: (order.index(f[0]), f[2][0], f[2][1], f[0] + ":" + ",".join(f[1])))
    findings, seen = [], {}
    for code, uids, at, msg, rep, fix in found:
        key = f"{code}:{','.join(uids)}"
        seen[key] = seen.get(key, 0) + 1
        if seen[key] > 1:
            key = f"{key}#{seen[key]}"
        f = {"key": key, "check": "program", "code": code, "message": msg, "repair": rep, "uids": uids,
             "at": P(cm(at[0]), cm(at[1]))}
        if fix:
            f["fix"] = fix
        findings.append(f)
    out = {"rule": rule, "units": urows, "totals": tot, "spaces": [r for _, _, r in rows]}
    if findings:
        out["findings"] = findings
    out["revision"] = "$current"
    return out


def cm(v):
    """To the centimetre, halves away from zero (a finding's place)."""
    return math.copysign(math.floor(abs(v) * 100 + 0.5) / 100, v)


def check_step(output, inp=None, op="execute"):
    return {"op": op, "command": "cad.program.check", "input": inp or {},
            "result": {"status": "completed", "output": output, "warnings": []},
            "expect": {"revision": "same"}, "within": 1e-3}


def refusal(code, message, path, uids=None):
    err = {"code": code, "message": message, "path": path}
    if uids:
        err["uids"] = uids
    return {"status": "failed", "error": err}


class Cases:
    def __init__(self):
        self.cases = []

    def add(self, name, steps, setup_doc=None):
        c = {"name": name, "steps": steps}
        if setup_doc is not None:
            c["setup"] = setup_doc
        self.cases.append(c)


# ── cad.program.check ────────────────────────────────────────────────────────────────────────────────────────────


def corridor_setup(pts, doors, holes=(), brief=None, unit="ortak", walls=None):
    """A lone corridor drawn as `pts` on the ground level, its doors on walls laid along its sides."""
    sp = {50: space(pts, "zemin", unit, "Koridor", "K1", holes=holes)}
    prog = {"units": [{"id": "ortak", "number": "O", "kind": "common"}],
            "briefs": [brief or {"usage": "Koridor", "minPassage": 1.2}], "version": VERSION}
    return walls, sp, prog


def program_check_cases():
    c = Cases()
    doors = door_steps()
    full = check(WALLS, SPACES, PROGRAM, DOORS)
    c.add("İki daire, ortak koridor, açık mutfak, şaft, balkon ve dubleks: toplamlar, hedefler ve bulgular",
          doors + [check_step(full)])
    c.add("Plan aynı cevabı verir, yazmaz", doors + [check_step(full, op="plan")])
    c.add("Doğrulama yazmaz", doors + [{"op": "validate", "command": "cad.program.check", "input": {},
                                       "result": {"status": "completed", "output": None, "warnings": []},
                                       "expect": {"revision": "same"}}])
    c.add("Kapısız çizim: her oda kapısız; koridorun geçişi ölçülmez",
          [check_step(check(WALLS, SPACES, PROGRAM, ()))])
    c.add("Yalnız 1. daire: dubleksin iki katı, toplamlar yalnız onun",
          doors + [check_step(check(WALLS, SPACES, PROGRAM, DOORS, units_filter=["d1"]), {"units": ["d1"]})])
    c.add("Yalnız 1. kat", doors + [check_step(check(WALLS, SPACES, PROGRAM, DOORS, levels_filter=["kat-1"]),
                                              {"levels": ["kat-1"]})])
    c.add("Bulgusuz: yalnız sayılar, geçiş taranmaz",
          doors + [check_step(check(WALLS, SPACES, PROGRAM, DOORS, findings_on=False), {"findings": False})])
    c.add("Programda olmayan bölüm reddedilir", [{"op": "execute", "command": "cad.program.check",
          "input": {"units": ["d9"]}, "result": refusal("unit_not_found", "“d9” bölümü bina programında yok.", "units[0]"),
          "expect": {"revision": "same"}}])
    c.add("Projede olmayan kat reddedilir", [{"op": "execute", "command": "cad.program.check",
          "input": {"levels": ["kat-9"]}, "result": refusal("level_not_found", "“kat-9” katı projede yok.", "levels[0]"),
          "expect": {"revision": "same"}}])
    # A space in a group the program lacks, its fix.
    sp = copy.deepcopy(SPACES)
    sp[28]["unit"] = "d9"
    c.add("Programda olmayan bölüme bağlı mahal bulgu olur, düzeltmesi bölümden çıkarır",
          doors + [check_step(check(WALLS, sp, PROGRAM, DOORS))], setup(spaces=sp))
    # The recorded area changes nothing but its difference.
    pg = copy.deepcopy(PROGRAM)
    pg["units"][1]["recordedArea"] = 47.78
    c.add("Kayıtlı alan yalnız okunur; farkı yazılır", doors + [check_step(check(WALLS, SPACES, pg, DOORS))],
          setup(program=pg))
    # Mixed methods in a unit.
    sp = copy.deepcopy(SPACES)
    sp[22]["method"] = "centerline"
    c.add("Bölümde iki yöntem: ikisi de söylenir", doors + [check_step(check(WALLS, sp, PROGRAM, DOORS))], setup(spaces=sp))
    # Briefs without a version: the rule alone.
    pg = copy.deepcopy(PROGRAM)
    del pg["version"]
    c.add("Sürümsüz hedefler: bulgular yalnız kuralı söyler", doors + [check_step(check(WALLS, SPACES, pg, DOORS))],
          setup(program=pg))
    # Corridors.
    walls = {1: ((0, 0), (8, 0), "zemin"), 2: ((0, 1.4), (8, 1.4), "zemin"), 3: ((0, 0), (0, 1.4), "zemin"),
             4: ((8, 0), (8, 1.4), "zemin")}
    cdoors = [(3, 0.7, 0.9), (4, 0.7, 0.9)]

    def corridor(name, pts, holes=(), brief=None, dws=cdoors):
        w, sp, prog = corridor_setup(pts, dws, holes, brief)
        res = check(walls, sp, prog, dws)
        c.add(name, door_steps(walls, dws) + [check_step(res)], setup(walls=walls, spaces=sp, program=prog))
    corridor("Düz koridor: geçiş net enine eşit", rect(0.1, 0.1, 7.9, 1.3))
    corridor("Koridorda daralma: geçiş darda, yeri ortası",
             [(0.1, 0.1), (7.9, 0.1), (7.9, 1.3), (4.6, 1.3), (4.6, 1.05), (3.4, 1.05), (3.4, 1.3), (0.1, 1.3)])
    corridor("Koridordaki niş geçişi değiştirmez",
             [(0.1, 0.1), (3.4, 0.1), (3.4, -0.35), (4.0, -0.35), (4.0, 0.1), (7.9, 0.1), (7.9, 1.3), (0.1, 1.3)])
    corridor("Koridordaki kolonun iki yanı: geniş yan geçiş olur", rect(0.1, 0.1, 7.9, 1.3),
             holes=[rect(3.8, 0.3, 4.2, 0.7)[::-1]])
    corridor("Kolon geçişi hedefin altına indirir", rect(0.1, 0.1, 7.9, 1.3),
             holes=[rect(3.8, 0.35, 4.2, 1.05)[::-1]])
    corridor("Tek kapılı koridorun geçişi ölçülmez", rect(0.1, 0.1, 7.9, 1.3), dws=[cdoors[0]])
    corridor("Geçiş hedefi olmayan koridor ölçülmez", rect(0.1, 0.1, 7.9, 1.3), brief={"usage": "Koridor", "minWidth": 1.0})
    # Acceptance: a partition moved; the found spaces follow it.
    awalls = {1: ((0, 0), (9, 0), "zemin"), 2: ((9, 0), (9, 5), "zemin"), 3: ((9, 5), (0, 5), "zemin"),
              4: ((0, 5), (0, 0), "zemin"), 5: ((4, 0), (4, 5), "zemin"), 6: ((5.4, 0), (5.4, 5), "zemin")}
    aprog = {"units": [{"id": "d1", "number": "1"}, {"id": "d2", "number": "2"},
                       {"id": "ortak", "number": "O", "kind": "common"}],
             "briefs": [{"usage": "Oda", "areaMin": 14.0, "areaMax": 16.0},
                        {"usage": "Koridor", "minPassage": 1.2}], "version": VERSION}
    adoors = [(5, 2.5, 0.9), (6, 2.5, 0.9), (1, 4.7, 1.0)]

    def found(seeds):
        """The rooms the walls make round each seed: between the walls' faces either side."""
        out = {}
        for i, (x, y, unit, usage, number) in seeds.items():
            xs = sorted({a[0] for a, b, _ in awalls_now.values() if a[0] == b[0] and min(a[1], b[1]) <= y <= max(a[1], b[1])})
            ys = sorted({a[1] for a, b, _ in awalls_now.values() if a[1] == b[1] and min(a[0], b[0]) <= x <= max(a[0], b[0])})
            x0, x1 = max(v for v in xs if v < x), min(v for v in xs if v > x)
            y0, y1 = max(v for v in ys if v < y), min(v for v in ys if v > y)
            out[i] = space(rect(x0 + 0.1, y0 + 0.1, x1 - 0.1, y1 - 0.1), "zemin", unit, usage, number)
            out[i]["name"] = None
        return out
    seeds = {7: (2, 2.5, "d1", "Oda", "1"), 8: (4.7, 2.5, "ortak", "Koridor", "2"), 9: (7, 2.5, "d2", "Oda", "3")}
    awalls_now = awalls
    make = {"op": "execute", "command": "cad.spaces.create",
            "input": {"layerId": "mahal", "spaces": [{"seed": P(x, y), "usage": u, "number": n, "unit": un,
                                                      "arch": {"level": "zemin"}} for _, (x, y, un, u, n) in sorted(seeds.items())]},
            "result": {"status": "completed", "output": "$any", "warnings": "$any"}, "expect": {"revision": "changed"}}
    before = found(seeds)
    door_ids = door_steps(awalls, adoors)
    global FIRST_DOOR
    keep_first = FIRST_DOOR
    FIRST_DOOR = 10
    res0 = check(awalls, before, aprog, adoors)
    moved = {"op": "execute", "command": "cad.entities.transform",
             "input": {"uids": ["$uidOf:6"], "transform": {"kind": "move", "dx": -0.1, "dy": 0.0}},
             "result": {"status": "completed", "output": "$any", "warnings": "$any"}, "expect": {"revision": "changed"}}
    awalls_now = {**awalls, 6: ((5.3, 0), (5.3, 5), "zemin")}
    after = found(seeds)
    res1 = check(awalls_now, after, aprog, [(5, 2.5, 0.9), (6, 2.5, 0.9), (1, 4.7, 1.0)])
    FIRST_DOOR = keep_first
    asetup = setup(walls=awalls, spaces={}, program=aprog)
    c.add("Kabul: bölme 10 cm kayınca hedef farkı ve kapılar arası geçiş aynı adımda değişir",
          [make] + door_ids + [check_step(res0), moved, check_step(res1)], asetup)
    # Mimari denetim's program group and its exceptions: a millimetre leaves an exception standing, ten centimetres
    # (a change of the words) let it lapse.
    key = "program_area_above:$uidOf:9"

    def audit(findings, excepted, lapsed_key=None):
        listed = []
        for f in findings:
            if f["key"] == key and lapsed_key is None:
                continue
            f = dict(f)
            if f["key"] == lapsed_key:
                f["lapsed"] = True
            listed.append(f)
        warnings = ([{"code": "exception_lapsed",
                      "message": "1 istisnanın bulgusu verildiğinden beri değişti; bulgular yeniden açık. İstisnayı yeniden verin ya da kaldırın."}]
                    if lapsed_key else [])
        return {"op": "execute", "command": "cad.arch.audit", "input": {"checks": ["program"]},
                "result": {"status": "completed", "output": {"findings": listed, "outside": 0, "excepted": excepted,
                                                              "revision": "$current"}, "warnings": warnings},
                "expect": {"revision": "same"}, "within": 1e-3}
    mark = {"op": "execute", "command": "cad.arch.exceptions",
            "input": {"operation": "mark", "keys": [key], "audit": {"checks": ["program"]}},
            "result": {"status": "completed", "output": "$any", "warnings": []}, "expect": {"revision": "changed"}}
    nudge = copy.deepcopy(moved)
    nudge["input"]["transform"]["dx"] = 0.001
    back = copy.deepcopy(moved)
    back["input"]["transform"]["dx"] = -0.101
    c.add("Denetim: program bulgusu istisna olur; 1 mm kayma istisnayı bozmaz, 10 cm kayma bulguyu yeniden açar",
          [make] + door_ids + [mark, audit(res0["findings"], 1), nudge, audit(res0["findings"], 1), back,
                               audit(res1["findings"], 0, key)], asetup)
    # Fixtures' clearances.
    block = {"id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7701", "name": "Klozet", "base": P(0, 0), "entities": [
        {"kind": "polygon", "id": 1, "layerId": "", "attrs": {}, "pts": [P(0, 0), P(0.4, 0), P(0.4, 0.6), P(0, 0.6)]}]}
    block2 = {"id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7702", "name": "Lavabo", "base": P(0, 0), "entities": [
        {"kind": "polygon", "id": 1, "layerId": "", "attrs": {}, "pts": [P(0, 0), P(0.6, 0), P(0.6, 0.45), P(0, 0.45)]}]}
    fprog = {"units": [{"id": "d1", "number": "1"}], "briefs": [], "version": VERSION,
             "clearances": [{"block": "Klozet", "width": 0.8, "depth": 0.6}]}
    fsp = {1: space(rect(0, 0, 2, 2.5), "zemin", "d1", "Banyo", "B")}

    def fixture_case(name, inserts):
        ents, fx_ = [], []
        for k, (blk, x, y, rot) in enumerate(inserts):
            i = 2 + k
            e = {"kind": "insert", "id": i, "layerId": "donati", "attrs": {},
                 "block": blk["id"], "p": P(x, y), "scale": 1, "rotation": rot, "arch": {"level": "zemin"}}
            ents.append(e)
            bx = blk["entities"][0]["pts"]
            x0, x1 = min(p["x"] for p in bx), max(p["x"] for p in bx)
            y0, y1 = min(p["y"] for p in bx), max(p["y"] for p in bx)
            c_, s_ = round(math.cos(rot), 12), round(math.sin(rot), 12)

            def tr(p, x=x, y=y, c_=c_, s_=s_):
                return (x + c_ * p[0] - s_ * p[1], y + s_ * p[0] + c_ * p[1])
            foot = [tr(p) for p in rect(x0, y0, x1, y1)]
            zone = None
            if blk["name"] == "Klozet":
                cx = (x0 + x1) / 2
                zone = [tr(p) for p in rect(cx - 0.4, y1, cx + 0.4, y1 + 0.6)]
            mid = tr(((x0 + x1) / 2, (y0 + y1) / 2))
            fx_.append({"uid": f"$uidOf:{i}", "level": "zemin", "foot": foot, "zone": zone, "middle": mid})
        res = check({}, fsp, fprog, (), fx_)
        doc = setup(walls={}, spaces=fsp, program=fprog, extra=ents, blocks=[block, block2])
        c.add(name, [check_step(res)], doc)
    fixture_case("Önü boş klozet: bulgu yok", [(block, 0.8, 0.0, 0.0)])
    fixture_case("Klozetin kullanım alanı duvardan taşıyor", [(block, 1.6, 1.9, -math.pi / 2)])
    fixture_case("Klozetin önüne lavabo konmuş", [(block, 0.8, 0.0, 0.0), (block2, 0.7, 0.7, 0.0)])
    fixture_case("Döndürülmüş klozetin önü boş", [(block, 0.0, 1.6, -math.pi / 2)])
    return c.cases


# ── cad.program.edit ─────────────────────────────────────────────────────────────────────────────────────────────


def program_edit_cases():
    c = Cases()
    base = copy.deepcopy(PROGRAM)

    def edit(name, inp, out_program=None, assigned=(), fields=None, op="execute", result=None, settings=True, steps=()):
        if result is None:
            out = {"program": out_program if out_program is not None else base}
            if assigned:
                out["assigned"] = list(assigned)
            out["revision"] = "$current"
            result = {"status": "completed", "output": out, "warnings": []}
        if result["status"] == "completed" and op == "execute":
            exp = {"revision": "changed" if (out_program not in (None, base) or assigned) else "same"}
            if settings:
                exp["settings"] = {"arch": {"levels": LEVELS, "program": out_program if out_program is not None else base}}
            if fields:
                exp["fields"] = fields
        else:
            exp = {"revision": "same"}
            if result["status"] == "completed":
                exp["settings"] = {"arch": {"levels": LEVELS, "program": base}}
        st = {"op": op, "command": "cad.program.edit", "input": inp, "result": result, "expect": exp}
        if op == "validate":
            st["result"] = {"status": "completed", "output": None, "warnings": []}
        c.add(name, list(steps) + [st])

    pg = copy.deepcopy(base)
    pg["units"].append({"id": "d3", "number": "3", "name": "Daire 3", "building": "a"})
    edit("Bölüm eklenir", {"program": pg}, pg)
    edit("Plan yazmaz", {"program": pg}, pg, op="plan")
    edit("Doğrulama yazmaz", {"program": pg}, op="validate")
    edit("Mahaller bölüme verilir", {"assign": [{"uids": ["$uidOf:28"], "unit": "d2"}]}, base, ["$uidOf:28"],
         {"28": {"unit": "d2"}})
    edit("Mahal bölümden çıkarılır", {"assign": [{"uids": ["$uidOf:24"], "unit": None}]}, base, ["$uidOf:24"],
         {"24": {"unit": None}})
    edit("Zaten o bölümdeki mahal değişmez", {"assign": [{"uids": ["$uidOf:16"], "unit": "d1"}]}, base, (),
         {"16": {"unit": "d1"}})
    pg2 = copy.deepcopy(pg)
    edit("Program ve atama birlikte, yeni bölüme", {"program": pg2, "assign": [{"uids": ["$uidOf:28", "$uidOf:27"], "unit": "d3"}]},
         pg2, ["$uidOf:27", "$uidOf:28"], {"27": {"unit": "d3"}, "28": {"unit": "d3"}})
    pg3 = copy.deepcopy(base)
    pg3["units"] = [u for u in pg3["units"] if u["id"] != "bahce"]
    edit("Mahalli bölüm kaldırılmaz", {"program": pg3},
         result=refusal("unit_in_use", "Kaldırılan bölümde 1 mahal kalıyor; önce onları başka bölüme verin ya da bölümden çıkarın.",
                        "program.units", ["$uidOf:27"]))
    edit("Mahalleri aynı çağrıda verilen bölüm kaldırılır",
         {"program": pg3, "assign": [{"uids": ["$uidOf:27"], "unit": None}]}, pg3, ["$uidOf:27"], {"27": {"unit": None}})
    edit("Değişiklik verilmezse ret", {},
         result=refusal("nothing_to_set", "Değiştirilecek bir şey verilmedi: programı ya da bölümlere verilecek mahalleri verin.", ""))

    def bad(name, change, path, words):
        p = copy.deepcopy(base)
        change(p)
        edit(name, {"program": p}, result=refusal("invalid_program", words, f"program.{path}"))
    bad("İki bölümün numarası aynı olamaz", lambda p: p["units"][1].update(number="1"), "units[1].number",
        "“1” numarası iki bölümde var; her bölümün numarası ayrı olmalı.")
    bad("Kanat binasının olmalı", lambda p: p["units"][0].update(wing="a-bati"), "units[0].wing",
        "“a-bati” kanadı “A Blok” binasında yok.")
    bad("Binasız kanat", lambda p: p["units"][3].update(wing="a-dogu"), "units[3].wing",
        "Kanat yalnız binası verilen bölümde seçilir.")
    bad("Programda olmayan bina", lambda p: p["units"][3].update(building="b"), "units[3].building",
        "“b” binası programda yok; önce binayı ekleyin.")
    bad("Ters alan aralığı", lambda p: p["briefs"][0].update(areaMin=20.0), "briefs[0].areaMax",
        "En çok alan (16 m²) en az alandan (20 m²) küçük.")
    bad("Aynı kullanımın iki hedefi", lambda p: p["briefs"].append({"usage": "Salon"}), "briefs[6].usage",
        "“Salon” kullanımının hedefi iki kez var.")
    bad("Boş kimlik", lambda p: p["units"][0].update(id=""), "units[0].id", "Bölümün kimliği boş olamaz; bir değer yazın.")
    bad("Sıfır kayıtlı alan", lambda p: p["units"][0].update(recordedArea=0.0), "units[0].recordedArea",
        "Kayıtlı alan 0 m²; sıfırdan büyük, sonlu olmalı.")
    bad("Kullanım iki kez yazılmaz", lambda p: p["briefs"][0].update(reach=["Salon", "Salon"]), "briefs[0].reach[1]",
        "“Salon” iki kez yazıldı.")
    bad("Kullanım alanının eni sıfır olamaz", lambda p: p.update(clearances=[{"block": "Klozet", "width": 0.0, "depth": 0.6}]),
        "clearances[0].width", "Kullanım alanının eni 0 m; sıfırdan büyük, sonlu olmalı.")
    edit("Programda olmayan bölüme verilmez", {"assign": [{"uids": ["$uidOf:28"], "unit": "d9"}]},
         result=refusal("unit_not_found", "“d9” bölümü bina programında yok; önce programa ekleyin.", "assign[0].unit"))
    edit("Mahal olmayan nesne bölüme verilmez", {"assign": [{"uids": ["$uidOf:1"], "unit": "d1"}]},
         result=refusal("not_a_space", "Nesne mahal değil; yalnız mahaller bölüme verilir.", "assign[0].uids[0]"))
    edit("Boş mahal listesi reddedilir", {"assign": [{"uids": [], "unit": "d1"}]},
         result=refusal("no_entities", "Bölüme verilecek mahal verilmedi. En az bir nesnenin kalıcı kimliğini verin.", "assign[0].uids"))
    edit("Geçersiz kimlik reddedilir", {"assign": [{"uids": ["D-1"], "unit": "d1"}]},
         result=refusal("invalid_uid", "“D-1” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.", "assign[0].uids[0]"))
    edit("Çizimde olmayan mahal reddedilir", {"assign": [{"uids": ["0192f5a1-7730-7000-8000-00000000dead"], "unit": "d1"}]},
         result=refusal("entity_not_found", "“0192f5a1-7730-7000-8000-00000000dead” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.", "assign[0].uids[0]"))
    edit("Geçersiz beklenen sürüm reddedilir", {"assign": [{"uids": ["$uidOf:28"], "unit": "d1"}], "expectedRevision": "x1"},
         result=refusal("invalid_revision", "Beklenen sürüm “x1” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"))
    conflict = {"status": "conflict", "error": {"code": "revision_conflict",
                "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
                "path": "expectedRevision", "revision": "$current"}}
    c.add("Çizim değiştiyse hiçbir şey yazılmaz", [
        {"op": "captureRevision", "as": "once"},
        {"op": "execute", "command": "cad.program.edit", "input": {"assign": [{"uids": ["$uidOf:28"], "unit": "d1"}]},
         "result": {"status": "completed", "output": "$any", "warnings": []}, "expect": {"revision": "changed"}},
        {"op": "execute", "command": "cad.program.edit", "input": {"assign": [{"uids": ["$uidOf:28"], "unit": "d2"}], "expectedRevision": "$once"},
         "result": conflict, "expect": {"revision": "same", "fields": {"28": {"unit": "d1"}}}}])
    edit("Kilitli katmandaki mahal bölüme verilmez", {"assign": [{"uids": ["$uidOf:29"], "unit": "d1"}]},
         result=refusal("layer_locked", "“Kilitli” katmanı kilitli; üzerindeki mahal bölüme verilemez. Kilidi Katmanlar panelinden açın.", "assign"))
    # Undo gives both the program and the groups back.
    st = {"op": "execute", "command": "cad.program.edit", "input": {"program": pg2, "assign": [{"uids": ["$uidOf:28"], "unit": "d3"}]},
          "result": {"status": "completed", "output": "$any", "warnings": []}, "expect": {"revision": "changed"}}
    c.add("Geri alma programı ve bölümü birlikte geri verir", [st, {"op": "undo", "returns": "Bina programı"},
          {"op": "execute", "command": "cad.program.check", "input": {"findings": False},
           "result": {"status": "completed", "output": "$any", "warnings": []},
           "expect": {"revision": "same", "settings": {"arch": {"levels": LEVELS, "program": base}},
                      "fields": {"28": {"unit": None}}}}])
    # A tag reading the program follows a renamed unit.
    pg4 = copy.deepcopy(base)
    pg4["units"][0]["name"] = "Dubleks daire"
    pg4["briefs"][0]["areaMax"] = 15.0
    tag = {"op": "execute", "command": "cad.tags.create",
           "input": {"layerId": "etiket", "tags": [{"of": "$uidOf:16", "at": P(2, 2), "content": "{number}\n{unit}\n{target}\n{targetDiff}"}]},
           "result": {"status": "completed", "output": "$any", "warnings": "$any"},
           "expect": {"fields": {"30": {"text": "101\n1 Daire 1\n12.00–16.00 m²\n0.00 m²"}}, "revision": "changed"}}
    st = {"op": "execute", "command": "cad.program.edit", "input": {"program": pg4},
          "result": {"status": "completed", "output": "$any", "warnings": []},
          "expect": {"revision": "changed", "fields": {"30": {"text": "101\n1 Dubleks daire\n12.00–15.00 m²\n0.36 m²"}}}}
    c.add("Programı okuyan etiket bölümün adı ve hedef değişince aynı adımda yenilenir", [tag, st])
    return c.cases


# ── cad.spaces.edit's decisions ──────────────────────────────────────────────────────────────────────────────────


DSP = {
    1: space(rect(0, 0, 6, 4), "zemin", "d1", "Ofis", "1"),
    2: space(rect(6, 0, 9, 4), "zemin", "d2", "Ofis", "2"),
    3: space(rect(0, 4, 6, 6), "zemin", "d1", "Depo", "3"),
}
DSP[1]["finishes"] = {"floor": "Parke"}
DSP[2]["finishes"] = {"floor": "Seramik"}
DPROG = {"units": [{"id": "d1", "number": "1"}, {"id": "d2", "number": "2"}]}
MASA = {"id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7703", "name": "Masa", "base": P(0, 0), "entities": [
    {"kind": "polygon", "id": 1, "layerId": "", "attrs": {}, "pts": [P(-0.4, -0.3), P(0.4, -0.3), P(0.4, 0.3), P(-0.4, 0.3)]}]}
INSERTS = {4: (1.0, 2.0), 5: (5.0, 2.0), 6: (7.5, 2.0)}


def decisions_setup():
    ents = [{"kind": "insert", "id": i, "layerId": "donati", "attrs": {}, "block": MASA["id"], "p": P(x, y),
             "scale": 1, "rotation": 0, "arch": {"level": "zemin"}} for i, (x, y) in INSERTS.items()]
    doc = setup(walls={}, spaces={}, program=DPROG, extra=[], blocks=[MASA])
    ents_sp = []
    for i, s in DSP.items():
        e = space_entity(i, s)
        if s.get("finishes"):
            e["finishes"] = s["finishes"]
        ents_sp.append(e)
    doc["entities"] = ents_sp + ents
    return doc


def inside(pts, p):
    xs, ys = [q[0] for q in pts], [q[1] for q in pts]
    return min(xs) < p[0] < max(xs) and min(ys) < p[1] < max(ys)


def ent_out(i, s, pts, number="keep", finishes="keep"):
    e = {"kind": "space", "id": i, "layerId": "mahal", "attrs": {}, "parts": "$any"}
    num = s["number"] if number == "keep" else number
    if num is not None:
        e["number"] = num
    e["name"] = s["name"]
    e["usage"] = s["usage"]
    e["arch"] = {"level": "zemin"}
    fin = s.get("finishes") if finishes == "keep" else finishes
    if fin:
        e["finishes"] = fin
    e["unit"] = s["unit"]
    return e


def fixtures_in(pts):
    return [f"$uidOf:{i}" for i, (x, y) in sorted(INSERTS.items()) if inside(pts, (x, y))]


def split_case(c, name, i, x, keep=None, numbers=None, finishes=None, op="execute"):
    s = DSP[i]
    xs = [p[0] for p in s["pts"]]
    ys = [p[1] for p in s["pts"]]
    a, b = rect(min(xs), min(ys), x, max(ys)), rect(x, min(ys), max(xs), max(ys))
    pieces = sorted([a, b], key=lambda r: -abs(signed_area(r)))
    by = "largest"
    if keep is not None:
        k = next(j for j, r in enumerate(pieces) if inside(r, keep))
        pieces.insert(0, pieces.pop(k))
        by = "given"
    inp = {"uids": [f"$uidOf:{i}"], "operation": {"kind": "split", "line": [P(x, min(ys) - 1), P(x, max(ys) + 1)]}}
    if keep is not None:
        inp["operation"]["keep"] = P(*keep)
    if numbers is not None:
        inp["operation"]["numbers"] = numbers
    if finishes is not None:
        inp["operation"]["finishes"] = finishes
    new_fin = s.get("finishes") if finishes is not False else None
    new_num = numbers[0] if numbers else None
    decision = {"kept": f"$uidOf:{i}", "by": by, "number": s["number"], "area": abs(signed_area(pieces[0])),
                "unit": s["unit"]}
    fk = fixtures_in(pieces[0])
    if fk:
        decision["fixtures"] = fk
    piece = {"area": abs(signed_area(pieces[1]))}
    if new_num:
        piece["number"] = new_num
    if new_fin:
        piece["finishes"] = new_fin
    fp = fixtures_in(pieces[1])
    if fp:
        piece["fixtures"] = fp
    decision["pieces"] = [piece]
    nid = 7
    out = {"spaces": [ent_out(i, s, pieces[0])], "added": [ent_out(nid if op == "execute" else 0, s, pieces[1], new_num, new_fin)],
           "kept": [f"$uidOf:{i}"], "decisions": [decision], "revision": "$current"}
    step = {"op": op, "command": "cad.spaces.edit", "input": inp,
            "result": {"status": "completed", "output": out, "warnings": []},
            "expect": {"revision": "changed" if op == "execute" else "same"}}
    if op == "execute":
        step["expect"]["fields"] = {str(nid): {"number": new_num, "unit": s["unit"], "finishes": new_fin}}
    c.add(name, [step], decisions_setup())


def merge_case(c, name, uids, keep=None, number="keep", op="execute", tags=False):
    order = list(uids)
    pick = order.index(keep) if keep is not None else 0
    by = "given" if keep is not None else "first"
    k = order[pick]
    s = DSP[k]
    pts_all = [p for u in order for p in DSP[u]["pts"]]
    xs, ys = [p[0] for p in pts_all], [p[1] for p in pts_all]
    merged = rect(min(xs), min(ys), max(xs), max(ys))
    num = s["number"] if number == "keep" else number
    inp = {"uids": [f"$uidOf:{u}" for u in order], "operation": {"kind": "merge"}}
    if keep is not None:
        inp["operation"]["keep"] = f"$uidOf:{keep}"
    if number != "keep":
        inp["operation"]["number"] = number
    pieces = []
    warnings = []
    # A merged space's id is gone once written: it is taken beforehand (`$uid:goneN`).
    ref = {u: (f"$uid:gone{u}" if j != pick else f"$uidOf:{u}") for j, u in enumerate(order)}
    for j, u in enumerate(order):
        if j == pick:
            continue
        o = DSP[u]
        pc = {"uid": ref[u], "number": o["number"], "area": abs(signed_area(o["pts"]))}
        if o.get("finishes"):
            pc["finishes"] = o["finishes"]
        if tags and u == 2:
            pc["tags"] = ["$uidOf:7"]
        pieces.append(pc)
        if o["unit"] != s["unit"]:
            warnings.append({"code": "space_units_merged",
                             "message": f"“{o['number']}” mahalinin bölümü korunan mahalinkinden farklıydı; korunanınki geçerli oldu.",
                             "path": "uids"})
    decision = {"kept": f"$uidOf:{k}", "by": by, "area": abs(signed_area(merged)), "unit": s["unit"]}
    if num is not None:
        decision["number"] = num
    fk = fixtures_in(merged)
    if fk:
        decision["fixtures"] = fk
    decision["pieces"] = pieces
    out = {"spaces": [ent_out(k, s, merged, num)], "kept": [f"$uidOf:{k}"],
           "removed": [ref[u] for j, u in enumerate(order) if j != pick], "decisions": [decision],
           "revision": "$current"}
    steps = [{"op": "captureUid", "id": u, "as": f"gone{u}"} for j, u in enumerate(order) if j != pick]
    if tags:
        steps.append({"op": "execute", "command": "cad.tags.create",
                      "input": {"layerId": "etiket", "tags": [{"of": "$uidOf:2", "at": P(7.5, 3.0)}]},
                      "result": {"status": "completed", "output": "$any", "warnings": "$any"}, "expect": {"revision": "changed"}})
    step = {"op": op, "command": "cad.spaces.edit", "input": inp,
            "result": {"status": "completed", "output": out, "warnings": warnings},
            "expect": {"revision": "changed" if op == "execute" else "same"}}
    if op == "execute":
        step["expect"]["fields"] = {str(k): {"number": num}}
    c.add(name, steps + [step], decisions_setup())


def decisions_cases():
    c = Cases()
    split_case(c, "Böl: büyük parça kimliğini korur; yeni parça numarasız, kaplamalarıyla, donatıları söylenir", 1, 4)
    split_case(c, "Böl: korunacak nokta küçük parçada; kimlik ona geçer", 1, 4, keep=(5, 1))
    split_case(c, "Böl: yeni parçaya numara verilir", 1, 4, numbers=["1b"])
    split_case(c, "Böl: yeni parça kaplamasız", 1, 4, finishes=False)
    split_case(c, "Böl: numara, nokta ve kaplama birlikte", 1, 2, keep=(1, 1), numbers=["1a"], finishes=False)
    split_case(c, "Böl: plan kararı yazmadan söyler", 1, 4, op="plan")
    split_case(c, "Böl: bölüm ve numarası olan mahal; yeni parça bölümde kalır", 2, 8.0, numbers=["2b"])
    merge_case(c, "Birleştir: ilki kimliğini korur; öbürünün numarası ve kaplaması karar olarak söylenir", [1, 3])
    merge_case(c, "Birleştir: verilen mahalin kimliği ve numarası sürer", [1, 3], keep=3)
    merge_case(c, "Birleştir: yeni numara verilir", [1, 3], number="1-3")
    merge_case(c, "Birleştir: numara kaldırılır", [1, 3], number=None)
    merge_case(c, "Birleştir: farklı bölümler uyarıyla, korunanınki geçerli", [1, 2])
    merge_case(c, "Birleştir: giden mahalin etiketi karar olarak söylenir", [1, 2], tags=True)
    merge_case(c, "Birleştir: plan kararı yazmadan söyler", [1, 3], keep=3, op="plan")

    def bad(name, inp, code, msg, path):
        c.add(name, [{"op": "execute", "command": "cad.spaces.edit", "input": inp,
                      "result": refusal(code, msg, path), "expect": {"revision": "same"}}], decisions_setup())
    bad("Birleştir: korunacak mahal birleşenlerden değil", {"uids": ["$uidOf:1", "$uidOf:3"], "operation": {"kind": "merge", "keep": "$uidOf:2"}},
        "invalid_input", "Kimliği korunacak mahal birleştirilen mahallerden biri olmalı.", "operation.keep")
    bad("Böl: korunacak nokta parçalarda değil", {"uids": ["$uidOf:1"], "operation": {"kind": "split", "line": [P(4, -1), P(4, 5)], "keep": P(20, 20)}},
        "invalid_input", "Korunacak nokta bölünen parçaların hiçbirinin içinde değil; kimliği korunacak parçanın içinden bir nokta verin.", "operation.keep")
    bad("Böl: numara sayısı parçalarla uymuyor", {"uids": ["$uidOf:1"], "operation": {"kind": "split", "line": [P(4, -1), P(4, 5)], "numbers": ["a", "b"]}},
        "invalid_input", "Bölme 1 yeni parça yapıyor; 2 numara verildi.", "operation.numbers")
    bad("Böl: numaralar yalnız tek mahalde", {"uids": ["$uidOf:1", "$uidOf:3"], "operation": {"kind": "split", "line": [P(4, -1), P(4, 7)], "numbers": ["a"]}},
        "invalid_input", "Yeni numaralar yalnız tek mahal bölünürken verilir.", "operation.numbers")
    bad("Değiştir: programda olmayan bölüm", {"uids": ["$uidOf:1"], "operation": {"kind": "set", "unit": "d9"}},
        "unit_not_found", "“d9” bölümü bina programında yok; önce Bina programı penceresinden ekleyin.", "operation.unit")
    for name, unit in (("Değiştir: mahal başka bölüme verilir", "d2"), ("Değiştir: mahal bölümden çıkarılır", None)):
        c.add(name, [{"op": "execute", "command": "cad.spaces.edit",
                      "input": {"uids": ["$uidOf:1"], "operation": {"kind": "set", "unit": unit}},
                      "result": {"status": "completed", "output": "$any", "warnings": []},
                      "expect": {"revision": "changed", "fields": {"1": {"unit": unit}}}}], decisions_setup())
    return c.cases


def write(name, title, note, setup_doc, cases, check_only):
    data = {"format": "kentos.command-cases", "version": 1, "command": name.split(".json")[0].replace(".decisions", ""),
            "commandVersion": 1, "title": title, "note": note, "setup": setup_doc, "cases": cases}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    path = OUT / name
    if check_only:
        if not path.exists() or path.read_text() != text:
            print(f"{path.relative_to(ROOT)} güncel değil: --check'siz çalıştırın.")
            return False
        return True
    path.write_text(text)
    print(f"{path.relative_to(ROOT)}: {len(cases)} durum")
    return True


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    note = ("ADR 0773. Beklenenler scripts/fixtures/program_cases.py'nin ADR'den yazılmış kurallarından, KentOS kodu "
            "olmadan; koridorun geçişi shapely (GEOS) ile aşındırılıp ikiye bölünerek.")
    ok = write("cad.program.check.json", "Bina programını denetle: toplamlar, hedefler, geçiş, kapılar, donatı; retler",
               note, setup(), program_check_cases(), args.check)
    ok &= write("cad.program.edit.json", "Bina programı: program, bölüme verme, retler, geri alma, etiket", note, setup(),
                program_edit_cases(), args.check)
    ok &= write("cad.spaces.edit.decisions.json", "Mahali böl ve birleştir: kimlik, numara, kaplama ve donatı kararı",
                note, decisions_setup(), decisions_cases(), args.check)
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
