#!/usr/bin/env python3
"""The shared cases of `road.terrains.sheetcontours` (Pafta eş yükselti eğrileri; plan/map-cad.md MAP-14,
docs/adr/1820), without KentOS code.

    uv run --no-project --python 3.11 --with triangle --with numpy python3 scripts/fixtures/road_sheet_contour_cases.py
    ... --check   # compares the file with the one on disk

The surface is Shewchuk's Triangle's (road_terrain_command_cases.py's build) of 336 points on a 20 m grid shaken by up
to 5 m: a plain rising 2 cm a metre, a scarp (15 m over a logistic of 4 m) and a hill (30 m, σ 60 m); a building and a
road drawn as areas and a slope boundary as a closed polyline are the details.

The rules, by hand (BÖHHBÜY 2018 m. 78(1)(c)–(g), Ek-3 0001–0005, the package's values):
- a triangle's spacing on the sheet is the interval over its slope (rise over run of its plane, exact fractions), in
  paper millimetres at the scale; under `steepBelow` it draws major lines only, over `auxiliaryAbove` the auxiliary
  line half an interval above each level;
- the level lines over the triangles taken, as road_terrain_command_cases.py's contours (a corner on the level counts
  above, pieces chained by their edges, a line from its fixed place, the lines by their first places); the levels from
  the base by the interval, every `majorEvery`-th major, the auxiliary ones between, all in height order;
- a line's segment split where it crosses a detail ring's edge (p's parameter strictly between 0 and 1, the edge's
  within [0, 1]), the parts whose middles lie inside a detail (even-odd over its rings) left out, their length counted;
  a closed line cut opens, its last part joined to its first through the closing segment;
- along each major part from its first place a height every `labelEvery` paper mm, the first at half that: in the
  middle of its segment's place, turned along it, its top uphill (against the fall direction of the triangle strictly
  around the segment's middle), its text three decimals at most;
- asked for (docs/adr/2320; m. 78(1)(ğ)), the characteristic points: inside each innermost closed major or minor line
  (no other such line's first place inside it, even-odd), the surface's vertex highest above the level, else the one
  lowest below it (the earlier vertex among equals; a vertex at the level is on the line; one inside a detail is no
  ground shown), its height to the decimetre (half up, one decimal) beside a point. A second surface, a bowl of 81 shaken points beside the first, has a pit;
- asked for (docs/adr/3120; m. 78(1)(ğ)'s slopes), every corner of a line or polyline on the slopes' top layers, then on
  their bottom layers, in the drawing's order, once per place, its height the surface's there (Triangle's triangle, its
  plane), written as the summits' are; a corner off the surface is counted. The slope's top and bottom lines run along
  the scarp, the top's last line ending beyond the surface;
- docs/adr/4740 (m. 63(2)): asked for, a height every `heightEvery` paper mm along each road line from its start (the
  first at half of it, once per place), then on the grid of that spacing (multiples of it east and north, south to
  north, west to east) inside the built-up areas, outside the details and at least half the spacing from every height
  placed before; heights the surface's, the places off it counted;
- docs/adr/4740 (m. 91(2), m. 93(1)(c)): the chosen points with their own height on the surface and not one of its
  corners compared: the RMS of own − surface against `factor` · KOH_Z, KOH_Z = `zFactor` · (xy₀·S² + xy₁·S + xy₂) cm;
- docs/adr/4740: the drawing kept and drawn again, writing only what changed: a new line exactly alike an old one not
  taken yet stays (lines are strokes of one look); once a line is written, the heights after it (masked texts) are
  written again; the old ones left removed, the new ones left written at the end with the next numbers.
"""

import copy
import json
import math
import random
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_terrain_command_cases as rt  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/road.terrains.sheetcontours.json"
RULES = json.loads((ROOT / "catalog/v1/rules/esyukselti-bohhbuy-2018.json").read_text("utf-8"))


def U(n):
    return f"0192f5c0-1820-7000-8000-{n:012x}"


def ground(x, y):
    z = 850.0 + 0.02 * x
    z += 30.0 * math.exp(-((x - 250.0) ** 2 + (y - 150.0) ** 2) / (2 * 60.0 ** 2))
    z += 15.0 / (1.0 + math.exp(-(x - 100.0) / 4.0))
    return z


def shaken_points():
    rnd = random.Random(1820)
    out = []
    for j in range(16):
        for i in range(21):
            x = min(400.0, max(0.0, i * 20.0 + rnd.uniform(-5.0, 5.0)))
            y = min(300.0, max(0.0, j * 20.0 + rnd.uniform(-5.0, 5.0)))
            x, y = round(x, 3), round(y, 3)
            out.append((x, y, round(ground(x, y), 3)))
    return out


POINTS = shaken_points()
BUILDING = [(230.0, 130.0), (272.0, 130.0), (272.0, 166.0), (230.0, 166.0)]
ROAD = [(-10.0, 58.5), (410.0, 70.5), (410.0, 82.5), (-10.0, 70.5)]
SLOPE = [(85.0, 205.0), (118.0, 205.0), (118.0, 236.0), (85.0, 236.0)]
TWO = [(500.0, 500.0, 850.0), (510.0, 500.0, 851.0)]
# docs/adr/3120: the scarp's top and bottom lines (its logistic is centred at x = 100 m); the top's last piece a line
# that shares its first corner with the polyline and ends beyond the surface (y 320).
SLOPE_TOP = [(110.0, 30.0), (111.5, 75.0), (109.0, 120.0), (110.5, 165.0)]
SLOPE_TOP_LINE = ((110.5, 165.0), (110.0, 320.0))
SLOPE_BOTTOM = [(90.0, 30.0), (88.5, 75.0), (91.0, 120.0), (89.5, 165.0), (90.0, 190.0)]
# docs/adr/3520: the chosen points (x, y, their own height or none).
CHOSEN = [(100.0, 60.0, 858.62), (95.0, 140.0, None), (130.0, 330.0, 902.4), (150.0, 330.0, None)]
CHOSEN_TWO = [(60.0, 100.0), (95.0, 140.0)]
# docs/adr/4740 (m. 63(2)): a built-up area round the building and the road's centre line along the road area.
BUILT_UP = [(185.0, 105.0), (335.0, 105.0), (335.0, 215.0), (185.0, 215.0)]
ROAD_AXIS = [(-10.0, 64.5), (410.0, 76.5)]
# docs/adr/4740 (m. 93(1)(c)): control points on the smooth ground, their heights the surface's there and an offset
# (within the limit at 1/1000, and over it), and one on a corner of the surface.
CONTROL = [(300.0, 250.0), (340.0, 30.0), (200.0, 260.0), (370.0, 160.0)]
WITHIN = [0.06, -0.09, 0.11, -0.04]
OVER = [0.45, -0.38, 0.52, -0.41]


def bowl_points():
    """A bowl (docs/adr/2320): a 9 × 9 grid of 20 m shaken by up to 4 m, 8 m deep, its bottom 892.3 m."""
    rnd = random.Random(2320)
    out = []
    for j in range(9):
        for i in range(9):
            x = round(600.0 + i * 20.0 + rnd.uniform(-4.0, 4.0), 3)
            y = round(j * 20.0 + rnd.uniform(-4.0, 4.0), 3)
            out.append((x, y, round(900.3 - 8.0 * math.exp(-((x - 680.0) ** 2 + (y - 80.0) ** 2) / (2 * 40.0 ** 2)), 3)))
    return out


BOWL = bowl_points()

LAYERS = [rt.node("0", "0"), rt.node("zemin", "Zemin"), rt.node("bina", "Bina"), rt.node("yol", "Yol"),
          rt.node("sev", "Şev sınırı"), rt.node("sev-ust", "Şev üstü"), rt.node("sev-dip", "Şev dibi"),
          rt.node("secilen", "Seçilen noktalar"), rt.node("yerlesik", "Yerleşik alan"), rt.node("yol-ekseni", "Yol ekseni"),
          rt.node("kontrol", "Kontrol"), rt.node("kontrol-asan", "Kontrol (aşan)"),
          rt.node("az", "Az nokta"), rt.node("cukur", "Çukur"),
          rt.node("esyukselti", "Eş yükselti"),
          rt.node("arazi", "Arazi", [rt.node("ek", "Ek")]), {**rt.node("kilitli", "Kilitli"), "locked": True},
          {**rt.node("gizli", "Gizli"), "visible": False}]


def P(x, y):
    return {"x": float(x), "y": float(y)}


def entities():
    ents = [("point", "zemin", {"p": P(x, y), "z": z}) for x, y, z in POINTS]
    ents.append(("polygon", "bina", {"pts": [P(*p) for p in BUILDING]}))
    ents.append(("polygon", "yol", {"pts": [P(*p) for p in ROAD]}))
    ents.append(("polyline", "sev", {"pts": [P(*p) for p in SLOPE], "closed": True}))
    ents += [("point", "az", {"p": P(x, y), "z": z}) for x, y, z in TWO]
    ents += [("point", "cukur", {"p": P(x, y), "z": z}) for x, y, z in BOWL]
    # docs/adr/3120: after the bowl, so the earlier objects keep their ids.
    ents.append(("polyline", "sev-ust", {"pts": [P(*p) for p in SLOPE_TOP]}))
    ents.append(("line", "sev-ust", {"a": P(*SLOPE_TOP_LINE[0]), "b": P(*SLOPE_TOP_LINE[1])}))
    ents.append(("polyline", "sev-dip", {"pts": [P(*p) for p in SLOPE_BOTTOM]}))
    # docs/adr/3520: the surveyor's chosen points, after the slopes: one measured on the slope, one without a height,
    # a two-part point (the second part a place already taken), one measured off the surface, one off it without.
    for x, y, z in CHOSEN:
        ents.append(("point", "secilen", {"p": P(x, y), **({"z": z} if z is not None else {})}))
    ents.append(("point", "secilen", {"p": P(*CHOSEN_TWO[0]), "parts": [{"p": P(*CHOSEN_TWO[1])}]}))
    # docs/adr/4740: the built-up area and the road's centre line; the control points follow once the surface is made.
    ents.append(("polygon", "yerlesik", {"pts": [P(*p) for p in BUILT_UP]}))
    ents.append(("polyline", "yol-ekseni", {"pts": [P(*p) for p in ROAD_AXIS]}))
    return ents


ENTS = entities()
SURFACES = [{"id": "t-1", "name": "Mevcut zemin", "points": {"layers": ["zemin"]}},
            {"id": "t-9", "name": "Az nokta", "points": {"layers": ["az"]}},
            {"id": "t-3", "name": "Çukur", "points": {"layers": ["cukur"]}}]


def setup(work="map"):
    settings = {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                "plotScale": 1000.0, "workspace": "cad", "cadWork": work, "drawingFont": "barlow"}
    # Surfaces are a Harita or Yol project's (an architecture project's file holds none).
    if work in ("map", "road"):
        settings["road"] = {"surfaces": copy.deepcopy(SURFACES)}
    return {"format": "kentos.document", "version": 2, "name": "Pafta eğrileri", "settings": settings,
            "origin": {"x": 0.0, "y": 0.0}, "layers": copy.deepcopy(LAYERS), "activeLayer": "0",
            "entities": [{"kind": k, "id": i + 1, "layerId": l, "attrs": {}, **copy.deepcopy(g)}
                         for i, (k, l, g) in enumerate(ENTS)],
            "uids": [U(i + 1) for i in range(len(ENTS))], "styles": {"items": [], "categories": []}}


def surface_of(points):
    g = {"points": [(x, y, z, U(i + 1)) for i, (x, y, z) in enumerate(points)], "breaklines": [], "boundary": None,
         "voids": [], "tolerance": None, "maxEdge": None, "edges": [], "removed": []}
    return rt.build(g)


S = surface_of(POINTS)
# The bowl's points come after the first surface's and the details in the drawing: their ids follow.
BOWL_FIRST = next(i for i, e in enumerate(ENTS) if e[1] == "cukur")
S3 = rt.build({"points": [(x, y, z, U(BOWL_FIRST + i + 1)) for i, (x, y, z) in enumerate(BOWL)],
               "breaklines": [], "boundary": None, "voids": [], "tolerance": None, "maxEdge": None, "edges": [],
               "removed": []})


def control_points():
    """docs/adr/4740: on each control layer the four places with the surface's height plus the layer's offsets (three
    decimals), then a ground point's own place (a corner of the surface) a little above it."""
    out = []
    for layer, offsets in (("kontrol", WITHIN), ("kontrol-asan", OVER)):
        for (x, y), dz in zip(CONTROL, offsets):
            h = rt.height(S, (x, y))
            assert h["place"] == "inside", (x, y)
            out.append(("point", layer, {"p": P(x, y), "z": round(h["z"] + dz, 3)}))
        x, y, z = POINTS[150]
        out.append(("point", layer, {"p": P(x, y), "z": round(z + 0.02, 3)}))
    return out


ENTS += control_points()


def ring_holds(ring, p):
    """Even-odd: whether p lies inside the closed ring (the crossing to its right)."""
    holds = False
    n = len(ring)
    for i in range(n):
        a, b = ring[i], ring[(i + 1) % n]
        if (a[1] > p[1]) != (b[1] > p[1]):
            x = a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1])
            if x > p[0]:
                holds = not holds
    return holds


def spots_of(s, lines, dets=()):
    """The characteristic points (docs/adr/2320)."""
    regular = [l for l in lines if l["closed"] and l["kind"] in ("major", "minor") and len(l["points"]) >= 3]
    used = sorted({i for tri in s["tris"] for i in tri})
    out = []
    for k, l in enumerate(regular):
        if any(j != k and ring_holds(l["points"], m["points"][0]) for j, m in enumerate(regular)):
            continue
        held = [i for i in used if s["z"][i] != l["z"] and ring_holds(l["points"], s["xy"][i])
                and not any(inside(d, s["xy"][i]) for d in dets)]
        if not held:
            continue
        top = max(held, key=lambda i: (s["z"][i], -i))
        bottom = min(held, key=lambda i: (s["z"][i], i))
        if s["z"][top] > l["z"]:
            kind, i = "summit", top
        elif s["z"][bottom] < l["z"]:
            kind, i = "pit", bottom
        else:
            continue
        z = s["z"][i]
        out.append({"kind": kind, "x": s["xy"][i][0], "y": s["xy"][i][1], "z": z,
                    "text": f"{math.floor(z * 10.0 + 0.5) / 10.0:.1f}"})
    return out


def slope_spots(s, top=(), bottom=()):
    """The slopes' characteristic points (docs/adr/3120): each corner of a line or polyline on the top layers, then on
    the bottom ones, in the drawing's order, once per place; its height the surface's (none off it: counted)."""
    out, seen, off = [], [], 0
    for kind, layers in (("slopeTop", top), ("slopeBottom", bottom)):
        for k, layer, g in ENTS:
            if layer not in layers:
                continue
            if k == "line":
                corners = [(g["a"]["x"], g["a"]["y"]), (g["b"]["x"], g["b"]["y"])]
            elif k == "polyline":
                corners = [(v["x"], v["y"]) for v in g["pts"]]
            else:
                continue
            for c in corners:
                if c in seen:
                    continue
                seen.append(c)
                h = rt.height(s, c)
                if h["place"] != "inside":
                    off += 1
                    continue
                z = h["z"]
                out.append({"kind": kind, "x": c[0], "y": c[1], "z": z, "text": f"{math.floor(z * 10.0 + 0.5) / 10.0:.1f}"})
    return out, off


def built_up_spots(s, built, d, dets, placed):
    """docs/adr/4740 (m. 63(2)): the road heights then the built-up grid's (the module's note); and how many along the
    roads, on the grid and off the surface."""
    out, seen, off = [], [], 0

    def add(kind, c):
        nonlocal off
        h = rt.height(s, c)
        if h["place"] != "inside":
            off += 1
            return
        z = h["z"]
        out.append({"kind": kind, "x": c[0], "y": c[1], "z": z, "text": f"{math.floor(z * 10.0 + 0.5) / 10.0:.1f}"})
    for k, layer, g in ENTS:
        if layer not in built.get("roads", ()):
            continue
        if k == "line":
            pieces = [[(g["a"]["x"], g["a"]["y"]), (g["b"]["x"], g["b"]["y"])]]
        elif k == "polyline":
            # Each part of it, a closed one round to its start (a part closes as its polyline does).
            pieces = []
            for part in [g["pts"]] + [q["pts"] for q in g.get("parts", [])]:
                pts = [(v["x"], v["y"]) for v in part]
                pieces.append(pts + [pts[0]] if g.get("closed") and len(pts) > 2 else pts)
        else:
            continue
        for piece in pieces:
            at, walked = d / 2.0, 0.0
            for p, q in zip(piece, piece[1:]):
                ln = math.hypot(q[0] - p[0], q[1] - p[1])
                if ln == 0.0:
                    continue
                while at <= walked + ln:
                    f = (at - walked) / ln
                    c = (p[0] + f * (q[0] - p[0]), p[1] + f * (q[1] - p[1]))
                    if c not in seen:
                        seen.append(c)
                        add("road", c)
                    at += d
                walked += ln
    roads = len(out)
    areas = details_of(built.get("areas", ()))
    corners = [c for a in areas for ring in a for c in ring]
    if corners:
        lo = (min(c[0] for c in corners), min(c[1] for c in corners))
        hi = (max(c[0] for c in corners), max(c[1] for c in corners))
        i0, i1 = math.ceil(lo[0] / d), math.floor(hi[0] / d)
        j0, j1 = math.ceil(lo[1] / d), math.floor(hi[1] / d)

        def near(c, spots):
            return any(math.hypot(sp["x"] - c[0], sp["y"] - c[1]) < d / 2.0 for sp in spots)
        for j in range(j0, j1 + 1):
            for i in range(i0, i1 + 1):
                c = (float(i) * d, float(j) * d)
                if (any(inside(a, c) for a in areas) and not any(inside(x, c) for x in dets)
                        and not near(c, placed) and not near(c, out)):
                    add("builtUp", c)
    return out, roads, len(out) - roads, off


def height_check(s, picked, check, scale):
    """docs/adr/4740: the chosen points with their own height on the surface and not one of its corners, against
    m. 93(1)(c)'s limit; none when there are none."""
    used = {i for t in s["tris"] for i in t}
    corners = {tuple(s["xy"][i]) for i in used}
    measured = [m for m in picked if "surface" in m]
    compared = [m for m in measured if (m["x"], m["y"]) not in corners]
    if not compared:
        return None
    diffs = [m["z"] - m["surface"] for m in compared]
    rms = math.sqrt(sum(v * v for v in diffs) / len(diffs))
    wide = 0
    for k in range(1, len(diffs)):
        if abs(diffs[k]) > abs(diffs[wide]):
            wide = k
    S_ = float(scale)
    koh_z = check["zFactor"] * (check["xy"][0] * S_ * S_ + check["xy"][1] * S_ + check["xy"][2]) / 100.0
    limit = check["factor"] * koh_z
    return {"compared": len(compared), "corners": len(measured) - len(compared), "rms": rms,
            "largest": abs(diffs[wide]), "at": [compared[wide]["x"], compared[wide]["y"]], "kohZ": koh_z,
            "limit": limit, "within": rms < limit}


def level_lines(s, z, keep):
    """road_terrain_command_cases.py's contours for one level, over the triangles `keep` takes."""
    above = lambda i: s["z"][i] >= z  # noqa: E731
    F = rt.F

    def cross(a, b):
        (ax, ay), (bx, by) = s["xy"][a], s["xy"][b]
        t = (F(z) - F(s["z"][a])) / (F(s["z"][b]) - F(s["z"][a]))
        return (float(F(ax) + t * (F(bx) - F(ax))), float(F(ay) + t * (F(by) - F(ay))))
    pieces = []
    for ti, t in enumerate(s["tris"]):
        if not keep(ti):
            continue
        e = [tuple(sorted((a, b))) for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])) if above(a) != above(b)]
        if len(e) == 2:
            pieces.append(e)
    at = {}
    for i, p in enumerate(pieces):
        for e in p:
            at.setdefault(e, []).append(i)
    used = [False] * len(pieces)
    order = sorted(range(len(pieces)), key=lambda i: min(len(at[e]) for e in pieces[i]))
    lines = []
    for start in order:
        if used[start]:
            continue
        e0, e1 = pieces[start]
        frm = e0 if len(at[e0]) == 1 else e1 if len(at[e1]) == 1 else e0
        chain, piece, edge = [frm], start, frm
        while True:
            used[piece] = True
            edge = pieces[piece][1] if pieces[piece][0] == edge else pieces[piece][0]
            chain.append(edge)
            nxt = [i for i in at[edge] if not used[i]]
            if not nxt:
                break
            piece = nxt[0]
        pts = [cross(*e) for e in chain]
        closed = len(chain) > 2 and chain[0] == chain[-1]
        pts = [p for i, p in enumerate(pts) if i == 0 or p != pts[i - 1]]
        if closed and len(pts) > 1 and pts[0] == pts[-1]:
            pts = pts[:-1]
        if len(pts) < (3 if closed else 2):
            continue
        if closed:
            n = len(pts)
            s0 = min(range(n), key=lambda i: pts[i])
            fwd = pts[(s0 + 1) % n] < pts[(s0 - 1) % n]
            pts = [pts[(s0 + j) % n] if fwd else pts[(s0 - j) % n] for j in range(n)]
        elif pts[-1] < pts[0]:
            pts.reverse()
        lines.append(([list(p) for p in pts], closed))
    lines.sort(key=lambda l: tuple(l[0][0]))
    return lines


def inside(rings, p):
    odd = False
    for ring in rings:
        n = len(ring)
        for i in range(n):
            a, b = ring[i], ring[(i + 1) % n]
            if (a[1] > p[1]) != (b[1] > p[1]):
                x = a[0] + (p[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0])
                if p[0] < x:
                    odd = not odd
    return odd


def crossing(p, q, a, b):
    rx, ry = q[0] - p[0], q[1] - p[1]
    sx, sy = b[0] - a[0], b[1] - a[1]
    d = rx * sy - ry * sx
    if d == 0.0:
        return None
    wx, wy = a[0] - p[0], a[1] - p[1]
    t = (wx * sy - wy * sx) / d
    u = (wx * ry - wy * rx) / d
    return t if 0.0 < t < 1.0 and 0.0 <= u <= 1.0 else None


def clip(points, closed, details):
    n = len(points)
    segments = n if closed else n - 1
    parts, current, cut, touched = [], [], 0.0, False
    for i in range(segments):
        p, q = points[i], points[(i + 1) % n]
        ts = [0.0, 1.0]
        for d in details:
            for ring in d:
                m = len(ring)
                for j in range(m):
                    t = crossing(p, q, ring[j], ring[(j + 1) % m])
                    if t is not None:
                        ts.append(t)
        ts = sorted(set(ts))
        for t0, t1 in zip(ts, ts[1:]):
            a = [p[0] + t0 * (q[0] - p[0]), p[1] + t0 * (q[1] - p[1])]
            b = [p[0] + t1 * (q[0] - p[0]), p[1] + t1 * (q[1] - p[1])]
            mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0]
            if any(inside(d, mid) for d in details):
                cut += math.hypot(b[0] - a[0], b[1] - a[1])
                touched = True
                if len(current) >= 2:
                    parts.append(current)
                current = []
            else:
                if not current:
                    current.append(a)
                current.append(b)
    if len(current) >= 2:
        parts.append(current)
    if closed and not touched:
        return [points], 0.0, True
    if closed and len(parts) >= 2 and parts[0][0] == points[0] and parts[-1][-1] == points[0]:
        head = parts.pop(0)
        parts[-1].extend(head[1:])
    return parts, cut, False


def level_text(z):
    r = math.floor(z * 1000.0 + 0.5) / 1000.0
    t = repr(r)
    return t[:-2] if t.endswith(".0") else t


def labels_of(s, points, closed, z, every):
    n = len(points)
    segs = [(points[i], points[(i + 1) % n]) for i in range(n if closed else n - 1)]
    out, s_at, walked = [], every / 2.0, 0.0
    for p, q in segs:
        ln = math.hypot(q[0] - p[0], q[1] - p[1])
        if ln == 0.0:
            continue
        while s_at <= walked + ln:
            f = (s_at - walked) / ln
            at = [p[0] + f * (q[0] - p[0]), p[1] + f * (q[1] - p[1])]
            d = [(q[0] - p[0]) / ln, (q[1] - p[1]) / ln]
            mid = [(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0]
            t = rt.strictly_in(s, mid)
            if t is not None:
                _, aspect = rt.slope_of(s, t)
                if aspect is not None:
                    uphill = [-math.sin(aspect), -math.cos(aspect)]
                    left = [-d[1], d[0]]
                    if left[0] * uphill[0] + left[1] * uphill[1] < 0.0:
                        d = [-d[0], -d[1]]
            out.append({"z": z, "text": level_text(z), "at": at, "rotation": math.degrees(math.atan2(d[1], d[0]))})
            s_at += every
        walked += ln
    return out


def num(v):
    t = repr(float(v))
    return t[:-2] if t.endswith(".0") else t


def details_of(layers):
    out = []
    for kind, layer, g in ENTS:
        if layer not in layers:
            continue
        if kind == "polygon" or (kind == "polyline" and g.get("closed")):
            rings = [[(p["x"], p["y"]) for p in g["pts"]]]
            out.append([r for r in rings if len(r) >= 3])
    return [d for d in out if d]


def chosen_spots(s, layers):
    """docs/adr/3520: each point on the layers (a multi-point's each part), in the drawing's order, once per place; its
    own height when it has one (the surface's beside it when inside), else the surface's (none: counted); and how many
    take their own height."""
    out, seen, off, own_count = [], [], 0, 0
    for k, layer, g in ENTS:
        if layer not in layers or k != "point":
            continue
        places = [((g["p"]["x"], g["p"]["y"]), g.get("z"))] + [((q["p"]["x"], q["p"]["y"]), q.get("z"))
                                                                  for q in g.get("parts", [])]
        for c, own in places:
            if c in seen:
                continue
            seen.append(c)
            h = rt.height(s, c)
            ground = h["z"] if h["place"] == "inside" else None
            z = own if own is not None else ground
            if z is None:
                off += 1
                continue
            own_count += own is not None
            spot = {"kind": "chosen", "x": c[0], "y": c[1], "z": z, "text": f"{math.floor(z * 10.0 + 0.5) / 10.0:.1f}"}
            if own is not None and ground is not None:
                spot["surface"] = ground
            out.append(spot)
    return out, off, own_count


def sheet(scale=1000, interval=None, base=0.0, details=(), steep=True, auxiliary=True, labels=True, rules=None,
          spots=False, surface=None, slopes=None, points=None, built=None, warns=None):
    rules = rules or RULES
    if interval is None:
        interval = next(i["interval"] for i in rules["intervals"] if i["scale"] == scale)
    s = surface or S
    used = {i for t in s["tris"] for i in t}
    lo, hi = min(s["z"][i] for i in used), max(s["z"][i] for i in used)
    first, last = math.ceil((lo - base) / interval), math.floor((hi - base) / interval)
    mm = 1000.0 / scale
    spacing = []
    for t in s["tris"]:
        slope, _ = rt.slope_of(s, t)
        spacing.append(interval / slope * mm if slope > 0.0 else math.inf)
    steep_t = [steep and v < rules["steepBelow"] for v in spacing]
    gentle_t = [auxiliary and v > rules["auxiliaryAbove"] for v in spacing]
    dets = details_of(details)
    lines, cut = [], 0.0

    def add(z, kind, found):
        nonlocal cut
        for points, closed in found:
            parts, gone, still = clip(points, closed, dets)
            cut += gone
            for part in parts:
                lines.append({"z": z, "kind": kind, "closed": still, "points": part})
    for m in range(2 * first - 1, 2 * last + 2):
        half = m / 2.0
        z = base + half * interval
        if m % 2 == 0:
            k = m // 2
            major = rules["majorEvery"] > 0 and k % rules["majorEvery"] == 0
            found = level_lines(s, z, (lambda t: True) if major else (lambda t: not steep_t[t]))
            add(z, "major" if major else "minor", found)
        elif auxiliary:
            add(z, "auxiliary", level_lines(s, z, lambda t: gentle_t[t]))
    every = rules["labelEvery"] / mm
    labs = [lab for l in lines if l["kind"] == "major" for lab in labels_of(s, l["points"], l["closed"], l["z"], every)] \
        if labels else []
    n_steep, n_gentle = sum(steep_t), sum(gentle_t)
    notes = [f"Eğriler {rules['id']} paketine göre: aralık {num(interval)} m (1/{scale}), her {rules['majorEvery']}. "
             f"eğri kalın ({num(rules['majorWeight'])} mm), öbürleri {num(rules['minorWeight'])} mm."]
    if n_steep:
        notes.append(f"Eğri arası paftada {num(rules['steepBelow'])} mm'den az olan {n_steep} üçgende yalnız kalın "
                     "eğriler çizildi.")
    if n_gentle:
        notes.append(f"Eğri arası {num(rules['auxiliaryAbove'])} mm'yi aşan {n_gentle} üçgende yarım aralıkta kesik "
                     "yardımcı eğri çizildi.")
    if labels:
        notes.append(f"Kalın eğrilere her {num(rules['labelEvery'])} mm'de bir yükseklik yazıldı "
                     f"({num(rules['labelHeight'])} mm); yazının kutusu maskelenir, eğri bölünmez.")
    if details:
        notes.append(f"Detayların içinde kalan {cut:.2f} m eğri çizilmedi (m. 78(1)(g)).")
    found = spots_of(s, lines, dets) if spots else []
    if spots:
        notes.append(f"Tepe ve çukurlarda {len(found)} karakteristik nokta kotu dm'ye kadar yazıldı "
                     f"({num(rules['spotHeight'])} mm, m. 78(1)(ğ)): en içteki kapalı eğrinin içindeki en yüksek ya da "
                     "en alçak köşe" + ("." if slopes else "; şev üstü ve dibi işaretlenmedi."))
    if slopes:
        more, off = slope_spots(s, slopes.get("top", ()), slopes.get("bottom", ()))
        tops = sum(1 for m in more if m["kind"] == "slopeTop")
        note = (f"Şevlerin üstünde {tops}, dibinde {len(more) - tops} karakteristik nokta kotu dm'ye kadar yazıldı "
                f"({num(rules['spotHeight'])} mm, m. 78(1)(ğ)): şev çizgilerinin köşeleri, kotları yüzeyden.")
        if off:
            note += f" Yüzeyin dışındaki {off} köşe yazılmadı."
        notes.append(note)
        found = found + more
    check = None
    if points:
        picked, off, own_count = chosen_spots(s, points)
        measured = [m for m in picked if "surface" in m]
        note = (f"Ölçmecinin seçtiği {len(picked)} noktanın kotu dm'ye kadar yazıldı ({num(rules['spotHeight'])} mm, "
                f"m. 78(1)(ğ)): kendi kotu olan {own_count} nokta kendi kotuyla, öbürleri yüzeyden; yalnız kot "
                "yazılır, nokta ölçmecinindir.")
        if measured:
            wide = max(measured, key=lambda m: abs(m["z"] - m["surface"]))
            note += (f" Kendi kotu yüzeyinkinden en çok {abs(wide['z'] - wide['surface']):.3f} m ayrılıyor "
                     f"(Y {wide['x']:.3f}, X {wide['y']:.3f}).")
        if off:
            note += f" Kendi kotu olmayan ve yüzeyin dışında kalan {off} nokta yazılmadı."
        notes.append(note)
        found = found + picked
        if rules.get("sectionCheck"):
            check = height_check(s, picked, rules["sectionCheck"], scale)
    built_found = None
    if built is not None:
        every = built.get("every", rules.get("heightEvery"))
        built_found = built_up_spots(s, built, every / mm, dets, found)
        found = found + built_found[0]
    if check is not None:
        f = rules["sectionCheck"]["factor"]
        note = (f"Seçilen noktalardan kendi kotu olan ve yüzeyin köşesi olmayan {check['compared']} noktada ölçülen kot ile "
                f"yüzeyin kotu arasındaki farkların karesel ortalaması {check['rms']:.3f} m (en büyük "
                f"{check['largest']:.3f} m, Y {check['at'][0]:.3f}, X {check['at'][1]:.3f}); m. 93(1)(c)'nin 1/{scale} için "
                f"sınırı {num(f)} · KOH_Z = {check['limit']:.3f} m (KOH_Z m. 91(2)'den {check['kohZ']:.3f} m): "
                f"{'sınırın altında' if check['within'] else 'sınırı aşıyor'}. Bu bir karşılaştırmadır; m. 93'ün kesit "
                "kontrolü yüksekliği bilinen iki kontrol noktası arasında, en çok 20 m aralıklı ve nivelmanla ölçülmüş "
                "noktalarla yapılır.")
        if check["corners"]:
            note += f" Yüzeyin köşesi olan {check['corners']} nokta karşılaştırılmadı (farkı sıfırdır)."
        notes.append(note)
        if not check["within"] and warns is not None:
            warns.append({"code": "section_check_exceeded",
                          "message": f"Seçilen noktaların ölçülen kotlarıyla yüzeyin kotu arasındaki farkların karesel "
                                     f"ortalaması {check['rms']:.3f} m, m. 93(1)(c)'nin sınırını ({check['limit']:.3f} m) "
                                     "aşıyor; yüzeyin kaynaklarını ve noktaların kotlarını denetleyin.",
                          "path": "points"})
    elif points and rules.get("sectionCheck") and any("surface" in m for m in found if m["kind"] == "chosen"):
        notes.append("Kendi kotu olan seçilen noktaların hepsi yüzeyin köşesi; ölçülen kotla yüzeyin karşılaştırması "
                     "yüzeye girmeyen noktalarla yapılır.")
    if built_found is not None:
        _, roads, grid, off = built_found
        every = built.get("every", rules.get("heightEvery"))
        note = (f"Yollarda {roads}, yerleşik alanlarda {grid} kot dm'ye kadar yazıldı ({num(rules['spotHeight'])} mm, "
                f"m. 63(2)): paftada yaklaşık {num(every)} mm'de bir (arazide {num(every / mm)} m), kotlar yüzeyden; "
                "yerleşik alandaki ızgara noktası detayın içinde ya da başka bir kota aralığın yarısından yakınsa "
                "yazılmaz. Çatı ve terasların uygun köşeleri ölçmecinin seçtiği noktalardır. m. 63 fotogrametrik "
                "üretimin maddesidir; yersel alımda m. 45(1)(c) uygulanır.")
        if off:
            note += f" Yüzeyin dışındaki {off} yer yazılmadı."
        notes.append(note)
    # docs/adr/2720: the auxiliary lines take the package's pattern (Ek-3 0003) as a project line type of its name.
    pattern = None
    if rules.get("auxiliaryPattern") and any(l["kind"] == "auxiliary" for l in lines):
        pattern = pattern_name(rules)
        parts = [f"{num(v)} mm çizgi" if v > 0 else f"{num(-v)} mm boşluk" if v < 0 else "nokta"
                 for v in rules["auxiliaryPattern"]]
        notes.append(f"Yardımcı eğriler “{pattern}” çizgi tipiyle çizilir (kâğıtta {', '.join(parts)}); tip projede "
                     "yoksa eklenir, varsa projedeki kullanılır; tip proje ayarıdır, geri almada kalır.")
    out = {"rules": rules["id"], "source": rules["source"], "scale": scale, "interval": interval, "lines": lines,
           "labels": labs, "steep": n_steep, "gentle": n_gentle, "cut": cut, "revision": "$current", "notes": notes}
    if found:
        out["spots"] = found
    if pattern:
        out["pattern"] = pattern
    if check is not None:
        out["check"] = check
    return out


def pattern_name(rules):
    return f"Yardımcı eğri ({rules['id']})"


def pattern_type(rules):
    """The package's pattern as the project line type the command adds (docs/adr/2720)."""
    return {"name": pattern_name(rules), "description": f"{rules['id']} paketinin yardımcı eğrisi",
            "elements": [{"length": v} for v in rules["auxiliaryPattern"]], "unit": "paper", "source": rules["id"]}


N = len(ENTS)


# ── docs/adr/4740: the drawing kept and drawn again, writing only what changed ────────────────────────────────────
def objects_of(o, rules=RULES):
    """The objects a sheet output writes, in its order: (content, look, places). Lines are strokes of the drawing's one
    look; a spot's point and text and a label's text keep their order (a text's place is its insertion point)."""
    spot = rules["spotHeight"] * o["scale"] / 1000.0
    out = [(("line", l["z"], l["kind"], l["closed"], tuple(tuple(p) for p in l["points"])), "stroke",
            [tuple(p) for p in l["points"]]) for l in o["lines"]]
    for s_ in o.get("spots", []):
        if s_["kind"] != "chosen":
            out.append((("point", s_["x"], s_["y"], s_["z"]), "ordered", [(s_["x"], s_["y"])]))
        out.append((("spot", s_["x"], s_["y"], s_["text"]), "ordered", [(s_["x"] + spot / 2.0, s_["y"])]))
    out += [(("label", lab["text"], tuple(lab["at"]), lab["rotation"]), "ordered", [tuple(lab["at"])])
            for lab in o["labels"]]
    return out


def staying(old, new, looks):
    """docs/adr/4740's walk: a new object takes the first old one alike not taken yet, unless an object before it is
    written, or stays from a later place, and the two are not strokes of one look; then the old one stays free."""
    alike = {}
    for i, c in enumerate(old):
        alike.setdefault(c, []).append(i)
    written, kept, stays = [], {}, [None] * len(new)

    def free(a, b):
        return a != "ordered" and a == b
    for j, c in enumerate(new):
        look = looks[j]
        queue = alike.get(c)
        cand = queue.pop(0) if queue else None
        blocked = cand is not None and (any(not free(w, look) for w in written)
                                        or any(m > cand and not free(k, look) for k, m in kept.items()))
        if cand is not None and not blocked:
            stays[j] = cand
            kept[look] = max(kept.get(look, -1), cand)
        else:
            if cand is not None:
                queue.insert(0, cand)
            if look not in written:
                written.append(look)
    return stays


class Kept:
    """The document's numbers as the steps leave them and the drawing's objects (number, content, look, places)."""

    def __init__(self):
        self.ids, self.next, self.objs = list(range(1, N + 1)), N + 1, []

    def draw(self, o):
        for content, look, places in objects_of(o):
            self.objs.append((self.next, content, look, places))
            self.ids.append(self.next)
            self.next += 1

    def redraw(self, o, regional=True):
        new, old = objects_of(o), self.objs
        stays = staying([x[1] for x in old], [c for c, _, _ in new], [l for _, l, _ in new]) if regional \
            else [None] * len(new)
        taken = {i for i in stays if i is not None}
        gone = [old[i] for i in range(len(old)) if i not in taken]
        gone_ids = {g[0] for g in gone}
        self.ids = [i for i in self.ids if i not in gone_ids]
        objs, written = [], []
        for j, (c, look, places) in enumerate(new):
            if stays[j] is None:
                objs.append((self.next, c, look, places))
                self.ids.append(self.next)
                self.next += 1
                written.append(places)
            else:
                objs.append(old[stays[j]])
        self.objs = objs
        row = {"drawing": "cz-1", "command": "road.terrains.sheetcontours", "removed": len(gone)}
        if regional:
            row["kept"] = len(new) - len(written)
            places = [p for g in gone for p in g[3]] + [p for pl in written for p in pl]
            if places:
                row["region"] = [min(p[0] for p in places), min(p[1] for p in places),
                                 max(p[0] for p in places), max(p[1] for p in places)]
        row["uids"] = "$any"
        return row


def raised(k, dz):
    """The surface with ground point k's height raised by dz (three decimals), and that height."""
    pts = list(POINTS)
    x, y, z = pts[k]
    pts[k] = (x, y, round(z + dz, 3))
    return surface_of(pts), pts[k][2]


def drawn(cases, name, input_, op="execute", layer="esyukselti", warns=(), note=None, **kw):
    more = []
    out = sheet(warns=more, **kw)
    warns = list(warns) + more
    written = op == "execute"
    if written:
        out["uids"] = "$any"
    # A chosen point's height alone is written (docs/adr/3520); every other spot a point and its height.
    count = len(out["lines"]) + len(out["labels"]) + sum(1 if sp["kind"] == "chosen" else 2 for sp in out.get("spots", []))
    expect = ({"canUndo": True, "canRedo": False, "dirty": True, "revision": "changed",
               "ids": list(range(1, N + count + 1)), "road": rt.road_of(SURFACES)}
              if written else {**rt.UNCHANGED, "ids": list(range(1, N + 1))})
    result = rt.completed(out, warns) if op != "validate" else rt.completed(None, warns)
    cases.append({"name": name, **({"note": note} if note else {}),
                  "steps": [rt.step(op, {"surface": "t-1", "layerId": layer, **input_}, result, expect, rt.W)]})
    return out


def refused(cases, name, input_, result, setup_=None):
    c = {"name": name, "steps": [rt.step("execute", {"surface": "t-1", "layerId": "esyukselti", **input_}, result,
                                         {**rt.UNCHANGED, "ids": list(range(1, N + 1))})]}
    if setup_ is not None:
        c["setup"] = setup_
    cases.append(c)


def cases():
    out = []
    all3 = ["bina", "yol", "sev"]
    main = drawn(out, "1/1000: m. 78(1)(c)–(g) ilk değerleriyle, bina, yol ve şev kesilir", {"details": all3},
                 details=all3)
    kinds = {l["kind"] for l in main["lines"]}
    assert kinds == {"major", "minor", "auxiliary"} and main["labels"] and main["cut"] > 0, (kinds, main["cut"])
    assert main["steep"] > 0 and main["gentle"] > 0, (main["steep"], main["gentle"])
    drawn(out, "Plan yazmaz", {"details": all3}, op="plan", details=all3)
    drawn(out, "Doğrulama yazmaz", {}, op="validate")
    drawn(out, "Detaysız: eğriler kesilmez", {})
    drawn(out, "1/2000: aralık 2 m, yazı her 400 m", {"scale": 2000}, scale=2000)
    drawn(out, "1/500: aralık 1 m, yazı her 100 m", {"scale": 500}, scale=500)
    drawn(out, "Kurallar kapalı: yalnız aralık ve kalınlık", {"steep": False, "auxiliary": False, "labels": False},
          steep=False, auxiliary=False, labels=False)
    drawn(out, "Elle aralık 0,5 m", {"interval": 0.5}, interval=0.5)
    drawn(out, "Taban kotu 0,5", {"base": 0.5}, base=0.5)
    drawn(out, "1/1500: paket aralık vermez, elle 1 m", {"scale": 1500, "interval": 1.0}, scale=1500, interval=1.0)
    other = dict(copy.deepcopy(RULES), id="deneme-10-50", source="Deneme paketi (sentetik)", steepBelow=10.0,
                 auxiliaryAbove=50.0)
    drawn(out, "Başka kural paketi: dik 10 mm, yardımcı 50 mm", {"rules": other}, rules=other)
    # docs/adr/2320: the summits' and pits' heights (m. 78(1)(ğ)).
    top = drawn(out, "Tepe kotu: tepenin en içteki kapalı eğrisinde en yüksek köşe dm'ye kadar", {"spots": True},
                spots=True)
    assert [s_["kind"] for s_ in top["spots"]] == ["summit"], top.get("spots")
    pit = drawn(out, "Çukur kotu: çukurun en içteki kapalı eğrisinde en alçak köşe", {"surface": "t-3", "spots": True},
                spots=True, surface=S3)
    assert [s_["kind"] for s_ in pit["spots"]] == ["pit"], pit.get("spots")
    drawn(out, "Tepe kotu planı yazmaz", {"spots": True}, op="plan", spots=True)
    walled = drawn(out, "Tepe kotu detaylarla: binanın içindeki köşe sayılmaz", {"spots": True, "details": all3},
                   spots=True, details=all3)
    assert walled["spots"] and walled["spots"][0]["z"] < top["spots"][0]["z"], walled.get("spots")
    # docs/adr/3120: the slopes' tops and bottoms (m. 78(1)(ğ)).
    both = {"top": ["sev-ust"], "bottom": ["sev-dip"]}
    sloped = drawn(out, "Şev kotları: üst ve dip çizgilerinin köşeleri yüzeyden dm'ye kadar; ortak köşe bir kez, yüzeyin "
                   "dışındaki köşe sayılır", {"slopes": both}, slopes=both)
    kinds = [s_["kind"] for s_ in sloped["spots"]]
    assert kinds == ["slopeTop"] * 4 + ["slopeBottom"] * 5, kinds
    assert all(t["z"] > b["z"] + 5.0 for t, b in zip(sloped["spots"][:4], sloped["spots"][4:])), sloped["spots"]
    # The rest at 1/5000 (5 m apart): fewer lines to carry.
    drawn(out, "Şev kotları planı yazmaz (1/5000)", {"scale": 5000, "slopes": both}, op="plan", scale=5000, slopes=both)
    drawn(out, "Yalnız şev üstü (1/5000)", {"scale": 5000, "slopes": {"top": ["sev-ust"]}}, scale=5000,
          slopes={"top": ["sev-ust"]})
    drawn(out, "Tepe ve şev kotları birlikte: tepe önce, şevler sonra (1/5000)", {"scale": 5000, "spots": True,
          "slopes": both}, scale=5000, spots=True, slopes=both)
    drawn(out, "Boş şev katmanları: şev kotu istenmemiş sayılır (1/5000)", {"scale": 5000, "slopes": {}}, scale=5000)
    # docs/adr/3520: the surveyor's other points (m. 78(1)(ğ)).
    picked = drawn(out, "Seçilen noktalar: kendi kotu olan kendisiyle, olmayan yüzeyden; iki parçalı noktanın aynı yeri bir "
                   "kez, yüzeyin dışında kotsuz nokta sayılır (1/5000)", {"scale": 5000, "points": ["secilen"]},
                   scale=5000, points=["secilen"])
    kinds = [(s_["kind"], "surface" in s_) for s_ in picked["spots"]]
    assert kinds == [("chosen", True), ("chosen", False), ("chosen", False), ("chosen", False)], kinds
    drawn(out, "Seçilen noktalar planı yazmaz (1/5000)", {"scale": 5000, "points": ["secilen"]}, op="plan",
          scale=5000, points=["secilen"])
    drawn(out, "Şevler ve seçilen noktalar birlikte: şevler önce (1/5000)", {"scale": 5000, "slopes": both,
          "points": ["secilen"]}, scale=5000, slopes=both, points=["secilen"])
    # docs/adr/4740 (m. 63(2)): heights along the road and on the built-up area's grid.
    built = {"areas": ["yerlesik"], "roads": ["yol-ekseni"]}
    town = drawn(out, "m. 63(2): yolda ve yerleşik alanda paftada yaklaşık 2 cm'de bir kot (1/1000: 20 m); binanın içi "
                 "atlanır, yolun yüzeyin dışındaki uçları sayılır", {"builtUp": built, "details": ["bina"]}, built=built,
                 details=["bina"])
    kinds = [s_["kind"] for s_ in town["spots"]]
    assert kinds.count("road") >= 15 and kinds.count("builtUp") >= 20, kinds
    drawn(out, "m. 63(2) elle 40 mm (1/1000: 40 m)", {"builtUp": {**built, "every": 40.0}},
          built={**built, "every": 40.0})
    drawn(out, "m. 63(2) planı yazmaz (1/5000)", {"scale": 5000, "builtUp": built}, op="plan", scale=5000, built=built)
    drawn(out, "Yalnız yol (1/5000)", {"scale": 5000, "builtUp": {"roads": ["yol-ekseni"]}}, scale=5000,
          built={"roads": ["yol-ekseni"]})
    drawn(out, "Tepe kotu ve yerleşik alan: tepeye yarım aralıktan yakın ızgara noktası yazılmaz",
          {"spots": True, "builtUp": {"areas": ["yerlesik"]}}, spots=True, built={"areas": ["yerlesik"]})
    # docs/adr/4740 (m. 91(2), m. 93(1)(c)): the control points' measured heights against the surface.
    within = drawn(out, "m. 93(1)(c): kontrol noktalarının ölçülen kotu ile yüzeyin farklarının KOH'u sınırın altında; "
                   "yüzeyin köşesi karşılaştırılmaz (1/1000)", {"points": ["kontrol"]}, points=["kontrol"])
    assert within["check"]["within"] and within["check"]["compared"] == 4 and within["check"]["corners"] == 1, \
        within["check"]
    over = drawn(out, "m. 93(1)(c): farkların KOH'u sınırı aşar, uyarılır (1/1000)", {"points": ["kontrol-asan"]},
                 points=["kontrol-asan"])
    assert not over["check"]["within"], over["check"]
    drawn(out, "m. 93(1)(c) 1/5000: sınır ölçekle büyür", {"scale": 5000, "points": ["kontrol-asan"]}, scale=5000,
          points=["kontrol-asan"])
    no_check = {k: v for k, v in copy.deepcopy(RULES).items() if k != "sectionCheck"}
    drawn(out, "Karşılaştırma sınırı vermeyen paket: karşılaştırma yok", {"points": ["kontrol"], "rules": no_check},
          points=["kontrol"], rules=no_check)
    # docs/adr/4740: the drawing kept with its surface and layers, drawn again writing only what changed.
    all3_in = {"surface": "t-1", "layerId": "esyukselti", "details": all3}
    k = min(range(len(POINTS)), key=lambda i: (POINTS[i][0] - 330.0) ** 2 + (POINTS[i][1] - 250.0) ** 2)
    first = sheet(details=all3)
    s2_, z2 = raised(k, 0.6)
    second = sheet(details=all3, surface=s2_)
    any_ = {"status": "completed", "output": "$any", "warnings": "$any"}
    draw_step = {"op": "execute", "input": all3_in, "result": any_, "within": rt.W}
    x, y, _ = POINTS[k]
    height = {"op": "execute", "command": "cad.entities.edit",
              "input": {"operation": "elevation", "changes": [{"kind": "update", "uid": f"$uidOf:{k + 1}",
                                                                "geometry": {"kind": "point", "p": P(x, y), "z": z2}}]},
              "result": any_, "within": rt.W}

    def rebuilt(rows, input_):
        return {"op": "execute", "command": "road.project.rebuild", "input": input_,
                "result": rt.completed({"rebuilt": rows, "revision": "$current"}), "within": rt.W}

    def changed(st):
        return {"ids": list(st.ids), "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}
    st = Kept()
    st.draw(first)
    row = st.redraw(second)
    lines_alike = row["kept"]
    assert 0 < lines_alike < len(second["lines"]) and row["removed"] > len(first["labels"]), row
    out.append({"name": "Kayıtlı çizim: zeminin bir kotu değişince eskir; yalnız değişeni yazınca aynı çıkan eğriler "
                        "yerinde kalır, eğriden sonraki yazılar yeniden yazılır",
                "note": "ADR 4740: eğriler tek renkte çizgi, sıraları resmi değiştirmez; maskeli yazılar bir eğri "
                        "yazılınca onun üstünde kalsın diye yeniden yazılır",
                "steps": [draw_step, height, {**rebuilt([row], {"regional": True}), "expect": changed(st)}]})
    st = Kept()
    st.draw(first)
    row = st.redraw(second, regional=False)
    out.append({"name": "Aynı değişiklik bütün yeniden üretimle: bütün nesneler yeniden yazılır",
                "steps": [draw_step, height, {**rebuilt([row], {}), "expect": changed(st)}]})
    # The building moved (a detail layer): the drawing goes stale by its layer's stamp.
    b = next(i for i, e in enumerate(ENTS) if e[1] == "bina")
    moved = [(px + 6.0, py) for px, py in BUILDING]
    keep = ENTS[b]
    ENTS[b] = ("polygon", "bina", {"pts": [P(*p) for p in moved]})
    third = sheet(details=all3)
    ENTS[b] = keep
    st = Kept()
    st.draw(first)
    row = st.redraw(third)
    move = {"op": "execute", "command": "cad.entities.edit",
            "input": {"operation": "grip", "changes": [{"kind": "update", "uid": f"$uidOf:{b + 1}",
                                                         "geometry": {"kind": "polygon", "pts": [P(*p) for p in moved]}}]},
            "result": any_, "within": rt.W}
    out.append({"name": "Bina katmanında bina kaydırılınca çizim katmanın damgasıyla eskir; yalnız değişeni yazar",
                "steps": [draw_step, move, {**rebuilt([row], {"regional": True}), "expect": changed(st)}]})
    # A layer the drawing does not read changed: nothing is stale.
    c = next(i for i, e in enumerate(ENTS) if e[1] == "kontrol")
    cx_, cy_ = CONTROL[0]
    nudge = {"op": "execute", "command": "cad.entities.edit",
             "input": {"operation": "grip", "changes": [{"kind": "update", "uid": f"$uidOf:{c + 1}",
                                                          "geometry": {"kind": "point", "p": P(cx_ + 1.0, cy_),
                                                                       "z": ENTS[c][2]["z"]}}]},
             "result": any_, "within": rt.W}
    st = Kept()
    st.draw(first)
    out.append({"name": "Okumadığı bir katman değişince çizim eskimez: yeniden üretilecek çizim yok",
                "steps": [draw_step, nudge, {**rebuilt([], {"regional": True}),
                                             "expect": {"ids": list(st.ids), "revision": "same"}}]})
    # Drawing again by itself (the project's autoRebuild): only what changed, unless the project draws whole.
    auto = setup()
    auto["settings"]["road"]["autoRebuild"] = True
    told = [{"code": "drawings_rebuilt", "message": "Eskiyen yol çizimleri kendiliğinden yeniden üretildi: cz-1."}]
    st = Kept()
    st.draw(first)
    st.redraw(second)
    out.append({"name": "Kendiliğinden yeniden üretim yalnız değişeni yazar: kot değişikliğinin adımında",
                "setup": auto,
                "steps": [draw_step, {**height, "result": {"status": "completed", "output": "$any", "warnings": told},
                                      "expect": changed(st)}]})
    whole = setup()
    whole["settings"]["road"].update(autoRebuild=True, rebuildWhole=True)
    st = Kept()
    st.draw(first)
    st.redraw(second, regional=False)
    out.append({"name": "Proje bütün çizer derse kendiliğinden yeniden üretim bütün yazar",
                "setup": whole,
                "steps": [draw_step, {**height, "result": {"status": "completed", "output": "$any", "warnings": told},
                                      "expect": changed(st)}]})
    # docs/adr/2720: the auxiliary pattern added as a project line type in the same step; the project's own kept.
    plain_out = sheet()
    assert plain_out.get("pattern"), "the default package draws auxiliary lines with its pattern"
    count = len(plain_out["lines"]) + len(plain_out["labels"])
    written = {**plain_out, "uids": "$any"}
    out.append({"name": "Yardımcı eğrinin 5/1 mm deseni (Ek-3 0003) projeye çizgi tipi olarak eklenir; geri alınınca eğriler "
                        "gider, tip proje ayarı olarak kalır (ADR 0313)",
                "steps": [rt.step("execute", {"surface": "t-1", "layerId": "esyukselti"}, rt.completed(written),
                                  {"canUndo": True, "revision": "changed", "ids": list(range(1, N + count + 1)),
                                   "settings": {"lineTypes": [pattern_type(RULES)]}}, rt.W),
                          {"op": "undo", "returns": "Pafta eş yükselti eğrileri",
                           "expect": {"ids": list(range(1, N + 1)), "settings": {"lineTypes": [pattern_type(RULES)]}}}]})
    own = [{"name": pattern_name(RULES), "elements": [{"length": 3.0}, {"length": -1.5}], "unit": "paper"}]
    owned = setup()
    owned["settings"]["lineTypes"] = own
    out.append({"name": "Projede aynı adlı tip varsa projedeki kullanılır, değiştirilmez",
                "setup": owned,
                "steps": [rt.step("execute", {"surface": "t-1", "layerId": "esyukselti"}, rt.completed(written),
                                  {"canUndo": True, "revision": "changed", "settings": {"lineTypes": own}}, rt.W)]})
    no_pattern = {k: v for k, v in copy.deepcopy(RULES).items() if k != "auxiliaryPattern"}
    drawn(out, "Deseni olmayan paket: yardımcı eğri yerleşik kesikli tiple", {"rules": no_pattern}, rules=no_pattern)
    refused(out, "Deseni bozuk paket", {"rules": dict(copy.deepcopy(RULES), auxiliaryPattern=[-1.0])},
            rt.failed("invalid_contours", "Eğri kural paketi eksik: kimliği ve kaynağı yazılı, her değeri sonlu ve "
                      "sıfırdan büyük olmalı.", "rules"))
    drawn(out, "Gizli katmana uyarıyla", {}, layer="gizli",
          warns=[{"code": "layer_hidden", "message": "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.",
                  "path": "layerId"}])
    bad = rt.failed("invalid_contours", "Eş yükselti aralığı 0'dan büyük bir sayı, taban kotu sonlu bir sayı olmalı.",
                    "interval")
    refused(out, "Yol ya da Harita projesi değil", {}, rt.NOT_ROAD, setup(work="architecture"))
    refused(out, "1/1500 aralıksız", {"scale": 1500},
            rt.failed("invalid_contours", "bohhbuy-2018-m78 paketi 1/1500 ölçeği için eğri aralığı vermiyor; aralığı "
                      "yazın.", "interval"))
    refused(out, "Ölçek 0", {"scale": 0}, rt.failed("invalid_contours", "Ölçek paydası 0 olamaz.", "scale"))
    refused(out, "Sıfır aralık", {"interval": 0.0}, bad)
    refused(out, "Kural paketi eksik", {"rules": dict(copy.deepcopy(RULES), id="")},
            rt.failed("invalid_contours", "Eğri kural paketi eksik: kimliği ve kaynağı yazılı, her değeri sonlu ve "
                      "sıfırdan büyük olmalı.", "rules"))
    no_spot = {k: v for k, v in copy.deepcopy(RULES).items() if k != "spotHeight"}
    refused(out, "Kot noktası yazı yüksekliği vermeyen paket", {"spots": True, "rules": no_spot},
            rt.failed("invalid_contours", "bohhbuy-2018-m78 paketi kot noktası yazısının yüksekliğini (spotHeight, "
                      "Ek-3 0004) vermiyor; tepe ve çukur kotları yazılmaz.", "rules"))
    refused(out, "Şev kotları, kot noktası yazı yüksekliği vermeyen paket", {"slopes": both, "rules": no_spot},
            rt.failed("invalid_contours", "bohhbuy-2018-m78 paketi kot noktası yazısının yüksekliğini (spotHeight, "
                      "Ek-3 0004) vermiyor; şev kotları yazılmaz.", "rules"))
    refused(out, "Seçilen noktalar, kot noktası yazı yüksekliği vermeyen paket", {"points": ["secilen"], "rules": no_spot},
            rt.failed("invalid_contours", "bohhbuy-2018-m78 paketi kot noktası yazısının yüksekliğini (spotHeight, "
                      "Ek-3 0004) vermiyor; seçilen noktaların kotları yazılmaz.", "rules"))
    refused(out, "Olmayan seçilen nokta katmanı", {"points": ["yok"]},
            rt.failed("layer_not_found", "“yok” katmanı çizimde yok.", "points"))
    refused(out, "m. 63(2): katman seçilmemiş", {"builtUp": {}},
            rt.failed("invalid_contours", "Yerleşik alan ve yol kotları için yerleşik alan sınırlarının ya da yolların "
                      "katmanını seçin.", "builtUp"))
    refused(out, "m. 63(2): aralık 0", {"builtUp": {"roads": ["yol-ekseni"], "every": 0.0}},
            rt.failed("invalid_contours", "Kotların paftadaki aralığı 0'dan büyük olmalı.", "builtUp.every"))
    no_every = {k: v for k, v in copy.deepcopy(RULES).items() if k != "heightEvery"}
    refused(out, "m. 63(2): aralık vermeyen paket", {"builtUp": {"roads": ["yol-ekseni"]}, "rules": no_every},
            rt.failed("invalid_contours", "bohhbuy-2018-m78 paketi yerleşik alan kotlarının aralığını (heightEvery, "
                      "m. 63(2)) vermiyor; aralığı yazın.", "builtUp.every"))
    refused(out, "m. 63(2), kot noktası yazı yüksekliği vermeyen paket", {"builtUp": {"roads": ["yol-ekseni"]},
                                                                           "rules": no_spot},
            rt.failed("invalid_contours", "bohhbuy-2018-m78 paketi kot noktası yazısının yüksekliğini (spotHeight, "
                      "Ek-3 0004) vermiyor; yerleşik alan ve yol kotları yazılmaz.", "rules"))
    refused(out, "Olmayan yerleşik alan katmanı", {"builtUp": {"areas": ["yok"]}},
            rt.failed("layer_not_found", "“yok” katmanı çizimde yok.", "builtUp.areas"))
    refused(out, "Olmayan yol katmanı", {"builtUp": {"roads": ["yok"]}},
            rt.failed("layer_not_found", "“yok” katmanı çizimde yok.", "builtUp.roads"))
    refused(out, "Bozuk karşılaştırma sınırı", {"rules": dict(copy.deepcopy(RULES), sectionCheck={"xy": [0.0, 0.0, 0.0],
                                                                                                "zFactor": 0.0,
                                                                                                "factor": 1.5})},
            rt.failed("invalid_contours", "Eğri kural paketi eksik: kimliği ve kaynağı yazılı, her değeri sonlu ve "
                      "sıfırdan büyük olmalı.", "rules"))
    refused(out, "Olmayan şev üstü katmanı", {"slopes": {"top": ["yok"]}},
            rt.failed("layer_not_found", "“yok” katmanı çizimde yok.", "slopes.top"))
    refused(out, "Olmayan şev dibi katmanı", {"slopes": {"top": ["sev-ust"], "bottom": ["yok"]}},
            rt.failed("layer_not_found", "“yok” katmanı çizimde yok.", "slopes.bottom"))
    refused(out, "Çok sık düzey", {"interval": 0.001},
            rt.failed("invalid_contours", "Bu aralıkla yüzeyi 10 000'den çok düzey keser; aralığı büyütün.", "interval"))
    refused(out, "Kesen düzey yok", {"interval": 1000.0},
            rt.failed("no_contours", "Bu aralıkla yüzeyi kesen eş yükselti eğrisi yok; aralığı ya da taban kotunu "
                      "değiştirin.", "interval"))
    refused(out, "Olmayan katman", {"layerId": "yok"},
            rt.failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.",
                      "layerId"))
    refused(out, "Grup katman değil", {"layerId": "arazi"},
            rt.failed("not_a_layer", "“Arazi” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir "
                      "katman seçin.", "layerId"))
    refused(out, "Kilitli katman", {"layerId": "kilitli"},
            rt.failed("layer_locked", "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir "
                      "katmanı etkinleştirin.", "layerId"))
    refused(out, "Olmayan detay katmanı", {"details": ["yok"]},
            rt.failed("layer_not_found", "“yok” katmanı çizimde yok.", "details"))
    refused(out, "Olmayan yüzey", {"surface": "yok"},
            rt.failed("unknown_terrain", "“yok” kimlikli ya da adlı yüzey projede yok.", "surface"))
    refused(out, "Kurulamayan yüzey", {"surface": "t-9"},
            rt.failed("terrain_unbuildable", f"Kaynaklardan yüzey kurulamıyor: {rt.PROBLEM['too_few_points']}."))
    refused(out, "Eski sürüme göre hazırlanmış", {"expectedRevision": "41"}, rt.CONFLICT)
    refused(out, "Geçersiz beklenen sürüm", {"expectedRevision": "x"},
            rt.failed("invalid_revision", "Beklenen sürüm “x” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı "
                      "yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"))
    return out


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.terrains.sheetcontours",
           "commandVersion": 1, "title": "Pafta eş yükselti eğrileri (BÖHHBÜY m. 78)",
           "note": "ADR 1820, 2320, 2720, 4740. Bağımsız başvuru scripts/fixtures/road_sheet_contour_cases.py: yüzey Triangle'ın, eğri arası "
                   "kuralı, kırpma ve yazılar elle; KentOS kodu yok. Yazma tek geri alma adımıdır.",
           "setup": setup(), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok = OUT.exists() and OUT.read_text("utf-8") == text
        print("Pafta eğrileri ortak durumları güncel." if ok else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum, {len(text) // 1024} KB)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
