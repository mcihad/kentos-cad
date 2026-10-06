"""The shared cases of the hatch patterns (docs/adr/0186): the library's
patterns, a family's dashes as drawn, a pattern's families as hatch paints,
its lines and dots cut to a region, a pattern carried through a similarity,
and a hatch's region.

    python3 scripts/fixtures/hatch_pattern_cases.py           # writes the file
    python3 scripts/fixtures/hatch_pattern_cases.py --check   # writes nothing; compares

Writes fixtures/hatch/v1/cases.json. The rules are written here from the ADR
on their own, not from an implementation's output; the geometry core
(crates/shared/geometry-core, `geom::hatch_pattern` and `ops::hatch_region`,
natively and through WASM: `hatchPatterns`, `hatchPaints`,
`hatchPatternPieces`, `hatchRegion`) is held to them.

- Library (§2): the patterns' families in paper millimetres, unturned; e is
  1/8 inch (3.175 mm); a 45° line through (k·e·√2, 0) lies k·e under the one
  through the origin. ISO 128's line types (W100: d = 1 mm) as horizontal
  families 5 mm apart.
- Dashes (§3): signed lengths (plus drawn, minus a gap, 0 a dot) as runs:
  neighbours of one kind joined; when the last run is of the first's kind it
  joins the first and the pattern starts that much earlier (the phase gains
  it); a gap first then goes to the end and the pattern starts after it (the
  phase loses it); the first eight runs kept. All drawn: whole; all gaps:
  never drawn.
- Paints (§3): a user-defined pattern's lines at its angle (and 90° more for
  a cross) its spacing apart, through the anchor. A pattern's family at its
  angle plus the pattern's; its origin and offset times the scale, the
  origin turned by the pattern's angle; spacing |dy|, offset its origin
  across the lines (−sin·x + cos·y), stagger sign(dy)·dx along them, dashes
  the runs of the scaled dashes, dash offset −(its origin along the lines) +
  the runs' shift. A family whose dy is 0 or whose dashes are all gaps draws
  nothing; a whole one has no dashes, no stagger, dash offset 0.
- Pieces (§8): each family's lines n·p = offset + k·spacing inside the ring
  and its holes (even–odd, a line through a vertex counted once), each span
  cut by the dashes from the phase t = s + dashOffset − k·stagger; a dot is
  a point. Over the budget: none, capped.
- Carried (§8): through a turn θ, a scale s and, first, a reflection in the
  x axis: a pattern's angle θ ± its own (− when reflected; from 0 up to
  360), its scale times s; reflected, each family's angle −angle, its
  origin's and offset's second numbers negated. A gradient's angle the same
  way. A user-defined pattern is the caller's.
- Regions (§5): the outer object's area that holds the seed (else its
  largest), less the islands' areas and the cutouts' boxes that reach into
  it; of what is left, the part that holds the seed, else the largest. The
  cases are axis-aligned rectangles on whole metres, worked on their grid:
  the expected area, the number of holes, which islands and cutouts reach in.
"""
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/hatch/v1/cases.json"

# ── Library ─────────────────────────────────────────────────────────────

E = 3.175
ISO = 5.0


def fam(angle, origin, offset, dashes=()):
    out = {"angle": angle, "origin": list(origin), "offset": list(offset)}
    if dashes:
        out["dashes"] = list(dashes)
    return out


def below(k):
    return (k * E * math.sqrt(2.0), 0.0)


def iso(name, description, dashes):
    return {"name": name, "group": "iso", "description": description, "lines": [fam(0.0, (0.0, 0.0), (0.0, ISO), dashes)]}


def library():
    e = E
    ansi = lambda name, d, lines: {"name": name, "group": "ansi", "description": d, "lines": lines}
    general = lambda name, d, lines: {"name": name, "group": "general", "description": d, "lines": lines}
    return [
        ansi("ANSI31", "Demir, tuğla, taş duvar", [fam(45.0, (0.0, 0.0), (0.0, e))]),
        ansi("ANSI32", "Çelik", [fam(45.0, (0.0, 0.0), (0.0, 3.0 * e)), fam(45.0, below(1.0), (0.0, 3.0 * e))]),
        ansi("ANSI33", "Bronz, pirinç, bakır", [fam(45.0, (0.0, 0.0), (0.0, 2.0 * e)), fam(45.0, below(1.0), (0.0, 2.0 * e), (e, -e / 2.0))]),
        ansi("ANSI34", "Plastik, kauçuk", [fam(45.0, (0.0, 0.0), (0.0, 6.0 * e))] + [fam(45.0, below(float(k)), (0.0, 6.0 * e)) for k in (1, 2, 3)]),
        ansi("ANSI35", "Ateş tuğlası, refrakter malzeme", [fam(45.0, (0.0, 0.0), (0.0, 2.0 * e)), fam(45.0, below(1.0), (0.0, 2.0 * e), (2.5 * e, -e / 2.0, 0.0, -e / 2.0))]),
        ansi("ANSI36", "Mermer, arduvaz, cam", [fam(45.0, (0.0, 0.0), (1.75 * e, e), (2.5 * e, -e / 2.0, 0.0, -e / 2.0))]),
        ansi("ANSI37", "Kurşun, çinko, magnezyum, yalıtım", [fam(45.0, (0.0, 0.0), (0.0, e)), fam(135.0, (0.0, 0.0), (0.0, e))]),
        ansi("ANSI38", "Alüminyum", [fam(45.0, (0.0, 0.0), (0.0, e)), fam(135.0, (0.0, 0.0), (2.0 * e, e), (2.5 * e, -1.5 * e))]),
        iso("ISO02W100", "Kesikli", (12.0, -3.0)),
        iso("ISO03W100", "Aralıklı kesikli", (12.0, -18.0)),
        iso("ISO04W100", "Uzun kesikli noktalı", (24.0, -3.0, 0.5, -3.0)),
        iso("ISO05W100", "Uzun kesikli iki noktalı", (24.0, -3.0, 0.5, -3.0, 0.5, -3.0)),
        iso("ISO06W100", "Uzun kesikli üç noktalı", (24.0, -3.0, 0.5, -3.0, 0.5, -3.0, 0.5, -3.0)),
        iso("ISO07W100", "Noktalı", (0.5, -3.0)),
        iso("ISO08W100", "Uzun ve kısa kesikli", (24.0, -3.0, 6.0, -3.0)),
        iso("ISO09W100", "Uzun ve iki kısa kesikli", (24.0, -3.0, 6.0, -3.0, 6.0, -3.0)),
        iso("ISO10W100", "Kesikli noktalı", (12.0, -3.0, 0.5, -3.0)),
        iso("ISO11W100", "İki kesikli noktalı", (12.0, -3.0, 12.0, -3.0, 0.5, -3.0)),
        iso("ISO12W100", "Kesikli iki noktalı", (12.0, -3.0, 0.5, -3.0, 0.5, -3.0)),
        iso("ISO13W100", "İki kesikli iki noktalı", (12.0, -3.0, 12.0, -3.0, 0.5, -3.0, 0.5, -3.0)),
        iso("ISO14W100", "Kesikli üç noktalı", (12.0, -3.0, 0.5, -3.0, 0.5, -3.0, 0.5, -3.0)),
        general("LINE", "Yatay çizgiler", [fam(0.0, (0.0, 0.0), (0.0, e))]),
        general("NET", "Kare ızgara", [fam(0.0, (0.0, 0.0), (0.0, e)), fam(90.0, (0.0, 0.0), (0.0, e))]),
        general("NET3", "Üçgen ızgara", [fam(a, (0.0, 0.0), (0.0, e)) for a in (0.0, 60.0, 120.0)]),
        general("DASH", "Kesikli çizgiler", [fam(0.0, (0.0, 0.0), (e, e), (e, -e))]),
        general("DOTS", "Noktalar", [fam(0.0, (0.0, 0.0), (e / 4.0, e / 2.0), (0.0, -e / 2.0))]),
        general(
            "BRICK",
            "Tuğla örgüsü",
            [fam(0.0, (0.0, 0.0), (0.0, 2.0 * e)), fam(90.0, (0.0, 0.0), (2.0 * e, 2.0 * e), (2.0 * e, -2.0 * e)), fam(90.0, (2.0 * e, 0.0), (2.0 * e, 2.0 * e), (-2.0 * e, 2.0 * e))],
        ),
        general("CROSS", "Artılar", [fam(0.0, (0.0, 0.0), (2.0 * e, 2.0 * e), (e, -3.0 * e)), fam(90.0, (e / 2.0, -e / 2.0), (2.0 * e, 2.0 * e), (e, -3.0 * e))]),
    ]


LIB = {p["name"]: p for p in library()}

# ── Dashes ──────────────────────────────────────────────────────────────


def runs(signed):
    """The ADR's rule, with exact fractions: ("whole",), ("never",) or ("runs", runs, shift)."""
    if not signed:
        return ("whole",)
    rs = []
    for x in signed:
        x = F(x)
        on = x >= 0
        if rs and rs[-1][0] == on:
            rs[-1][1] += abs(x)
        else:
            rs.append([on, abs(x)])
    if all(r[0] for r in rs):
        return ("whole",)
    if not any(r[0] for r in rs):
        return ("never",)
    shift = F(0)
    if len(rs) > 1 and rs[0][0] == rs[-1][0]:
        last = rs.pop()
        rs[0][1] += last[1]
        shift += last[1]
    if not rs[0][0]:
        gap = rs.pop(0)
        shift -= gap[1]
        rs.append(gap)
    return ("runs", [r[1] for r in rs][:8], shift)


def runs_json(signed):
    r = runs(signed)
    if r[0] != "runs":
        return {"kind": r[0]}
    return {"kind": "runs", "runs": [float(x) for x in r[1]], "shift": float(r[2])}


DASH_CASES = [
    ("boş: bütün çizgi", []),
    ("hepsi çizgi: bütün", [2.0, 1.0]),
    ("hepsi boşluk: hiç çizilmez", [-2.0, -1.0]),
    ("kesikli", [12.0, -3.0]),
    ("boşlukla başlayan: boşluk sona, evre geri", [-2.0, 2.0]),
    ("sondaki çizgi baştakine: evre ileri", [5.0, -2.0, 3.0]),
    ("nokta", [0.0, -1.5]),
    ("aynı türden komşular birleşir; nokta çizginin parçası", [3.0, 0.0, -1.0, -1.0, 2.0, -0.5]),
    ("boşlukla başlayıp boşlukla biten: sondaki baştakine, sonra sona", [-1.0, 4.0, -2.0]),
    ("tuğlanın ikinci dikey ailesi", [-6.35, 6.35]),
    ("sekizden çok kesikte ilk sekizi", [12.0, -3.0, 12.0, -3.0, 0.5, -3.0, 0.5, -3.0, 0.5, -3.0]),
]

# ── Paints ──────────────────────────────────────────────────────────────


def paints(p):
    kind = p["type"]
    if kind == "lines" and p["spacing"] > 0:
        return [paint_user(p["angle"], p["spacing"])]
    if kind == "cross" and p["spacing"] > 0:
        return [paint_user(p["angle"], p["spacing"]), paint_user(p["angle"] + 90.0, p["spacing"])]
    if kind != "pattern":
        return []
    scale = p.get("scale") or 0.0
    if not scale > 0:
        return []
    turn = math.radians(p["angle"])
    out = []
    for l in p.get("lines", []):
        dy = l["offset"][1] * scale
        if dy == 0:
            continue
        angle = l["angle"] + p["angle"]
        r = math.radians(angle)
        c, s = math.cos(r), math.sin(r)
        ox, oy = l["origin"][0] * scale, l["origin"][1] * scale
        x, y = ox * math.cos(turn) - oy * math.sin(turn), ox * math.sin(turn) + oy * math.cos(turn)
        offset = -s * x + c * y
        d = runs([d * scale for d in l.get("dashes", [])])
        if d[0] == "never":
            continue
        if d[0] == "whole":
            out.append({"angle": angle, "spacing": abs(dy), "offset": offset, "stagger": 0.0, "dashOffset": 0.0})
        else:
            out.append(
                {
                    "angle": angle,
                    "spacing": abs(dy),
                    "offset": offset,
                    "stagger": math.copysign(1.0, dy) * l["offset"][0] * scale,
                    "dash": [float(v) for v in d[1]],
                    "dashOffset": -(c * x + s * y) + float(d[2]),
                }
            )
    return out


def paint_user(angle, spacing):
    return {"angle": angle, "spacing": spacing, "offset": 0.0, "stagger": 0.0, "dashOffset": 0.0}


def pattern_of(name, angle=0.0, scale=1.0):
    return {"type": "pattern", "angle": angle, "spacing": 1.0, "name": name, "scale": scale, "lines": LIB[name]["lines"]}


PAINT_CASES = [
    ("çizgili", {"type": "lines", "angle": 30.0, "spacing": 1.5}),
    ("çapraz", {"type": "cross", "angle": 45.0, "spacing": 2.0}),
    ("dolu: boyası yok", {"type": "solid", "angle": 0.0, "spacing": 1.0}),
    ("degrade: boyası yok", {"type": "gradient", "angle": 0.0, "spacing": 1.0, "gradient": {"shape": "linear", "color2": "#FFFFFF"}}),
    ("ölçeksiz desen: boyası yok", {"type": "pattern", "angle": 0.0, "spacing": 1.0, "name": "X", "lines": [fam(0.0, (0.0, 0.0), (0.0, 1.0))]}),
    ("ANSI31, 1:500", pattern_of("ANSI31", 0.0, 0.5)),
    ("ANSI33, 15° döndürülmüş", pattern_of("ANSI33", 15.0, 0.5)),
    ("ANSI36: kaymalı kesikler ve nokta", pattern_of("ANSI36", 0.0, 1.0)),
    ("ANSI38: 135° ailenin kaymasi", pattern_of("ANSI38", 30.0, 2.0)),
    ("tuğla: boşlukla başlayan aile", pattern_of("BRICK", 0.0, 0.25)),
    ("artılar: taban noktası kaymış aile", pattern_of("CROSS", 10.0, 1.0)),
    ("noktalar", pattern_of("DOTS", 0.0, 1.0)),
    ("ISO06: sekiz kesik", pattern_of("ISO06W100", 90.0, 0.2)),
    ("üçgen ızgara", pattern_of("NET3", 0.0, 1.0)),
    (
        "aralıksız aile ve hep boşluk aile çizilmez; eksi aralık",
        {"type": "pattern", "angle": 0.0, "spacing": 1.0, "name": "Ö", "scale": 1.0, "lines": [fam(0.0, (0.0, 0.0), (1.0, 0.0)), fam(0.0, (0.0, 0.0), (0.0, 1.0), (-1.0,)), fam(30.0, (1.0, 2.0), (0.5, -2.0), (1.0, -1.0))]},
    ),
]

# ── Pieces ──────────────────────────────────────────────────────────────


def family_pieces(rings, f, budget, out):
    """Appends one family's segments and dots; False when they pass the budget."""
    if not f["spacing"] > 0:
        return True
    r = math.radians(f["angle"])
    c, s = math.cos(r), math.sin(r)
    loc = [[(x * c + y * s, -x * s + y * c) for x, y in ring] for ring in rings if len(ring) >= 3]
    if not loc:
        return True
    vs = [v for _, v in loc[0]]
    first = math.ceil((min(vs) - f["offset"]) / f["spacing"])
    last = math.floor((max(vs) - f["offset"]) / f["spacing"])
    if last - first + 1 > 20000:
        return False
    dash = f.get("dash")
    period = sum(dash) if dash else 0.0
    world = lambda u, v: [u * c - v * s, u * s + v * c]
    for k in range(first, last + 1):
        v = f["offset"] + k * f["spacing"]
        xs = []
        for ring in loc:
            n = len(ring)
            for i in range(n):
                a, b = ring[i - 1], ring[i]
                if (a[1] <= v) != (b[1] <= v):
                    xs.append(a[0] + (v - a[1]) / (b[1] - a[1]) * (b[0] - a[0]))
        xs.sort()
        for i in range(0, len(xs) - 1, 2):
            u0, u1 = xs[i], xs[i + 1]
            if u1 - u0 <= 1e-9:
                continue
            if not dash or period <= 0:
                out["segments"].append([world(u0, v), world(u1, v)])
                if len(out["segments"]) + len(out["dots"]) > budget:
                    return False
                continue
            shift = f["dashOffset"] - k * f["stagger"]
            j = math.floor((u0 + shift) / period)
            while j * period - shift < u1:
                at = j * period - shift
                for n_, ln in enumerate(dash):
                    if n_ % 2 == 0:
                        a, b = at, at + ln
                        if ln == 0:
                            if u0 <= a <= u1:
                                out["dots"].append(world(a, v))
                        elif b > u0 and a < u1:
                            a, b = max(a, u0), min(b, u1)
                            if b - a > 1e-9:
                                out["segments"].append([world(a, v), world(b, v)])
                        if len(out["segments"]) + len(out["dots"]) > budget:
                            return False
                    at += ln
                j += 1
    return True


def pieces(ring, holes, pattern, budget):
    out = {"segments": [], "dots": []}
    for f in paints(pattern):
        if not family_pieces([ring] + holes, f, budget, out):
            return {"segments": [], "dots": [], "capped": True}
    out["capped"] = False
    return out


def square(x0, y0, x1, y1):
    return [[x0, y0], [x1, y0], [x1, y1], [x0, y1]]


PIECE_CASES = [
    ("çizgili kare", square(0.0, 0.0, 10.0, 10.0), [], {"type": "lines", "angle": 0.0, "spacing": 2.5}, 1000),
    ("delikli karede çapraz", square(0.0, 0.0, 10.0, 10.0), [square(3.0, 3.0, 7.0, 7.0)], {"type": "cross", "angle": 45.0, "spacing": 1.5}, 1000),
    ("ANSI33: kesikler", square(2.0, 1.0, 14.0, 9.0), [], pattern_of("ANSI33", 0.0, 0.5), 5000),
    ("tuğla: kaymalı dikey kesikler", square(0.0, 0.0, 20.0, 12.0), [], pattern_of("BRICK", 0.0, 0.5), 5000),
    ("noktalar: noktalar ayrı", square(0.0, 0.0, 4.0, 3.0), [], pattern_of("DOTS", 0.0, 1.0), 5000),
    ("dönük artılar", [[0.0, 0.0], [16.0, 2.0], [12.0, 14.0], [-2.0, 9.0]], [], pattern_of("CROSS", 20.0, 0.5), 5000),
    ("bütçeyi aşan: hiçbiri", square(0.0, 0.0, 10.0, 10.0), [], {"type": "lines", "angle": 0.0, "spacing": 0.1}, 50),
]

# ── Carried ─────────────────────────────────────────────────────────────


def turn_of(a):
    t = math.fmod(a, 360.0)
    return t + 360.0 if t < 0 else t


def carried(p, turn, scale, reflect):
    out = json.loads(json.dumps(p))
    if p["type"] == "pattern":
        out["angle"] = turn_of(turn - p["angle"] if reflect else turn + p["angle"])
        if "scale" in p:
            out["scale"] = p["scale"] * scale
        if reflect:
            lines = []
            for l in p["lines"]:
                m = {"angle": turn_of(-l["angle"]), "origin": [l["origin"][0], -l["origin"][1]], "offset": [l["offset"][0], -l["offset"][1]]}
                if "dashes" in l:
                    m["dashes"] = l["dashes"]
                lines.append(m)
            out["lines"] = lines
    elif p["type"] == "gradient":
        out["angle"] = turn_of(turn - p["angle"] if reflect else turn + p["angle"])
    return out


CARRY_CASES = [
    ("desen döner ve büyür", pattern_of("ANSI33", 15.0, 0.5), 30.0, 2.0, False),
    ("desen yansır: aileler x eksenine göre", pattern_of("CROSS", 10.0, 1.0), 90.0, 1.0, True),
    ("yansıyan desenin açısı 0'dan 360'a", pattern_of("ANSI38", 200.0, 1.0), -45.0, 0.5, True),
    ("degrade döner", {"type": "gradient", "angle": 30.0, "spacing": 1.0, "gradient": {"shape": "linear", "color2": "#FFFFFF"}}, 100.0, 3.0, False),
    ("degrade yansır", {"type": "gradient", "angle": 30.0, "spacing": 1.0, "gradient": {"shape": "cylinder", "inverted": True, "color2": "#00FF00"}}, 0.0, 1.0, True),
    ("çizgili: çağıranındır", {"type": "lines", "angle": 30.0, "spacing": 1.5}, 45.0, 2.0, True),
]

# ── Regions ─────────────────────────────────────────────────────────────


def rect_polygon(x0, y0, x1, y1, holes=()):
    e = {"kind": "polygon", "id": 1, "layerId": "a", "attrs": {}, "pts": [{"x": x, "y": y} for x, y in square(x0, y0, x1, y1)]}
    if holes:
        e["holes"] = [{"pts": [{"x": x, "y": y} for x, y in square(*h)]} for h in holes]
    return e


def region_by_grid(outer, outer_holes, islands, cutouts, seed):
    """The ADR's rule on the grid of the rectangles' whole-metre edges: the seed's part (else the largest) of the
    outer less what reaches into it; its area, its holes and which islands and cutouts reach in."""
    xs = sorted({v for r in [outer] + list(outer_holes) + list(islands) + list(cutouts) for v in (r[0], r[2])})
    ys = sorted({v for r in [outer] + list(outer_holes) + list(islands) + list(cutouts) for v in (r[1], r[3])})
    xs = [xs[0] - 1] + xs + [xs[-1] + 1]
    ys = [ys[0] - 1] + ys + [ys[-1] + 1]
    inside = lambda r, x, y: r[0] < x < r[2] and r[1] < y < r[3]
    nx, ny = len(xs) - 1, len(ys) - 1
    mid = lambda i, j: ((F(xs[i]) + xs[i + 1]) / 2, (F(ys[j]) + ys[j + 1]) / 2)
    base = [[inside(outer, *mid(i, j)) and not any(inside(h, *mid(i, j)) for h in outer_holes) for j in range(ny)] for i in range(nx)]
    if not any(any(c) for c in base):
        return None
    reach = lambda r: any(base[i][j] and inside(r, *mid(i, j)) for i in range(nx) for j in range(ny))
    used_islands = [k for k, r in enumerate(islands) if reach(r)]
    used_cutouts = [k for k, r in enumerate(cutouts) if reach(r)]
    cut = [islands[k] for k in used_islands] + [cutouts[k] for k in used_cutouts]
    left = [[base[i][j] and not any(inside(r, *mid(i, j)) for r in cut) for j in range(ny)] for i in range(nx)]
    seen = [[False] * ny for _ in range(nx)]
    parts = []
    for i in range(nx):
        for j in range(ny):
            if left[i][j] and not seen[i][j]:
                stack, cells = [(i, j)], []
                seen[i][j] = True
                while stack:
                    a, b = stack.pop()
                    cells.append((a, b))
                    for c, d in ((a + 1, b), (a - 1, b), (a, b + 1), (a, b - 1)):
                        if 0 <= c < nx and 0 <= d < ny and left[c][d] and not seen[c][d]:
                            seen[c][d] = True
                            stack.append((c, d))
                parts.append(cells)
    if not parts:
        return None
    area = lambda cells: sum((F(xs[a + 1]) - xs[a]) * (F(ys[b + 1]) - ys[b]) for a, b in cells)
    holding = [p for p in parts if any(xs[a] < seed[0] < xs[a + 1] and ys[b] < seed[1] < ys[b + 1] for a, b in p)]
    part = holding[0] if holding else max(parts, key=area)
    mine = set(part)
    # Holes: the cells not in the part, joined, that do not reach the grid's edge.
    seen = [[(a, b) in mine for b in range(ny)] for a in range(nx)]
    holes = 0
    for i in range(nx):
        for j in range(ny):
            if seen[i][j]:
                continue
            stack, edge = [(i, j)], False
            seen[i][j] = True
            while stack:
                a, b = stack.pop()
                edge |= a in (0, nx - 1) or b in (0, ny - 1)
                for c, d in ((a + 1, b), (a - 1, b), (a, b + 1), (a, b - 1)):
                    if 0 <= c < nx and 0 <= d < ny and not seen[c][d]:
                        seen[c][d] = True
                        stack.append((c, d))
            holes += 0 if edge else 1
    return {"area": float(area(part)), "holes": holes, "islands": used_islands, "cutouts": used_cutouts}


def region_case(note, outer, islands=(), cutouts=(), seed=(1.0, 1.0), outer_holes=()):
    return {
        "note": note,
        "outer": rect_polygon(*outer, holes=outer_holes),
        "islands": [rect_polygon(*r) for r in islands],
        "cutouts": [square(*r) for r in cutouts],
        "seed": {"x": seed[0], "y": seed[1]},
        "expect": region_by_grid(outer, outer_holes, islands, cutouts, seed),
    }


def region_cases():
    o = (0.0, 0.0, 100.0, 60.0)
    return [
        region_case("adasız: bütün alan", o, seed=(50.0, 30.0)),
        region_case("içteki ada delik olur", o, islands=[(10.0, 10.0, 30.0, 30.0)], seed=(50.0, 30.0)),
        region_case("kenarı aşan ada kenarı oyar, delik değil", o, islands=[(90.0, 20.0, 110.0, 40.0)], seed=(50.0, 30.0)),
        region_case("uzaktaki ada sayılmaz", o, islands=[(200.0, 0.0, 210.0, 10.0), (10.0, 10.0, 20.0, 20.0)], seed=(50.0, 30.0)),
        region_case("yazının kutusu delik olur", o, cutouts=[(40.0, 40.0, 52.0, 44.0)], seed=(50.0, 30.0)),
        region_case("bölen şerit: tohumun yanı", o, islands=[(48.0, -10.0, 52.0, 70.0)], seed=(20.0, 30.0)),
        region_case("bölen şerit: öbür yan", o, islands=[(48.0, -10.0, 52.0, 70.0), (60.0, 10.0, 70.0, 20.0)], seed=(80.0, 30.0)),
        region_case("tohum dışarıda: en büyük parça", o, islands=[(30.0, -10.0, 34.0, 70.0)], seed=(500.0, 500.0)),
        region_case("tohum adanın içinde: en büyük parça", o, islands=[(30.0, -10.0, 34.0, 70.0)], seed=(32.0, 30.0)),
        region_case("alanın kendi deliği kalır", o, islands=[(70.0, 10.0, 80.0, 20.0)], cutouts=[(10.0, 40.0, 20.0, 50.0)], seed=(50.0, 30.0), outer_holes=[(40.0, 20.0, 60.0, 40.0)]),
        region_case("bütününü kaplayan ada: bölge yok", o, islands=[(-5.0, -5.0, 105.0, 65.0)], seed=(50.0, 30.0)),
    ]


def open_region_case():
    line = {"kind": "polyline", "id": 1, "layerId": "a", "attrs": {}, "pts": [{"x": 0.0, "y": 0.0}, {"x": 10.0, "y": 0.0}, {"x": 10.0, "y": 10.0}]}
    return {"note": "açık çizgi kapalı değil: bölge yok", "outer": line, "islands": [], "cutouts": [], "seed": {"x": 5.0, "y": 2.0}, "expect": None}


def build():
    return {
        "format": "kentos.hatch-pattern-cases",
        "version": 1,
        "source": "scripts/fixtures/hatch_pattern_cases.py; docs/adr/0186",
        "library": library(),
        "dashes": [{"note": n, "signed": s, "expect": runs_json(s)} for n, s in DASH_CASES],
        "paints": [{"note": n, "pattern": p, "expect": paints(p)} for n, p in PAINT_CASES],
        "pieces": [{"note": n, "ring": r, "holes": h, "pattern": p, "budget": b, "expect": pieces(r, h, p, b)} for n, r, h, p, b in PIECE_CASES],
        "carried": [{"note": n, "pattern": p, "turn": t, "scale": s, "reflect": f, "expect": carried(p, t, s, f)} for n, p, t, s, f in CARRY_CASES],
        "regions": region_cases() + [open_region_case()],
    }


def text_of(v):
    return json.dumps(v, ensure_ascii=False, indent=1) + "\n"


def main():
    data = build()
    text = text_of(data)
    if "--check" in sys.argv:
        old = OUT.read_text("utf-8") if OUT.exists() else ""
        if old != text:
            print(f"{OUT} güncel değil: python3 {Path(__file__).relative_to(ROOT)} ile yeniden yazın.")
            return 1
        print(f"{OUT.relative_to(ROOT)} güncel.")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, "utf-8")
    counts = ", ".join(f"{len(data[k])} {k}" for k in ("library", "dashes", "paints", "pieces", "carried", "regions"))
    print(f"{OUT.relative_to(ROOT)}: {counts}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
