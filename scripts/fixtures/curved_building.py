#!/usr/bin/env python3
"""The curved building (docs/adr/8071; plan/bim-plan.md BIM-10's first acceptance): a 10 × 6 m plan whose north wall is
an arc bowing 2 m out (its line's bulge 0.4: radius 7.25 m round (5, 0.75)), on two storeys of different heights at
different levels (Zemin ±0.00, 3.20 m; 1. kat +3.35, 2.90 m): the ground's walls from the ground slab's structure
(−0.05, under its finish) to the 1. kat slab's underside (+3.20), the 1. kat's from +3.35 to the roof slab's (+6.25). Walls 30 cm, no layers, on their centre lines, joined at
the four corners; a window through the south wall, a door through the east wall and a window through the curved wall,
its jambs square to the arc; slabs along the walls' outer faces, the arc's edge an arc (the ground's with a 5 cm finish
over it, the 1. kat's with a 2 × 1 m hole, the roof's at the top); a room inside each storey's inner faces.

Writes fixtures/arch/v1/curved-building.kcad with the independent reference writer, without KentOS code, and
fixtures/arch/v1/curved-building.json: what the bodies are by closed forms of their own, worked out here (a corner's
mitre through the two faces' meeting points, a ring's piece between two radii (r₁² − r₂²)/2 · θ, a circle's segment
r²/2 · (θ − sin θ)): each wall's and slab's volume gross and net (IFC 4.3's: a wall's gross its own body's without its
joins, its net the joined body's less its openings; a slab's gross its outline's, its net less its hole), its lengths
and areas, each room's floor area, each body's bottom and top over ±0.00. The application's test and `curved_ifc_check.py` compare against it; `--check`
compares both files.

    python3 scripts/fixtures/curved_building.py [--check]"""
import argparse
import json
import math
import sys
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))
import kcad_v2_reference as ref  # noqa: E402

OUT = ROOT / "fixtures/arch/v1/curved-building.kcad"
EXPECTED = ROOT / "fixtures/arch/v1/curved-building.json"

W = 0.30  # the walls' width
H = W / 2
BULGE = 0.4  # the north wall's line: C (10, 6) to D (0, 6), bowing north
C0 = (5.0, 0.75)  # its centre
R = 7.25  # its radius
LEVELS = [
    {"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.20},
    {"id": "k1", "name": "1. kat", "elevation": 3.35, "height": 2.90},
]
SLAB = 0.15  # every slab's structure
FINISH = 0.05  # the finish over the ground slab
ROOF_OFFSET = 3.05  # the roof slab's top over the 1. kat's floor
# Each storey's walls' foot and top over ±0.00 (docs/adr/8040 §1–2): from the structure's top of the floor slab under
# them (the masonry is laid on the concrete, under the screed and the finish) up to the structure's underside of the
# slab over them, never past the storey's top.
WALLS = {
    "zemin": (0.0 - FINISH, min(3.35 - SLAB, 0.0 + 3.20)),
    "k1": (3.35, min(3.35 + ROOF_OFFSET - SLAB, 3.35 + 2.90)),
}


def layer(i, name):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def P(x, y):
    return {"x": float(x), "y": float(y)}


def seg(r, theta):
    """A circle's segment between an arc of `theta` and its chord."""
    return r * r / 2 * (theta - math.sin(theta))


def shoelace(pts):
    return sum(pts[i][0] * pts[(i + 1) % len(pts)][1] - pts[(i + 1) % len(pts)][0] * pts[i][1]
               for i in range(len(pts))) / 2


def on_circle(r, x):
    """The point of the circle of radius `r` round C0 at abscissa `x`, its northern one."""
    return (x, C0[1] + math.sqrt(r * r - (x - C0[0]) ** 2))


def angle(p):
    return math.atan2(p[1] - C0[1], p[0] - C0[0])


# The radius checks the bulge: R = c (1 + b²) / 4b on the 10 m chord, the centre c (1 − b²) / 4b south of its middle.
assert abs(R - 10 * (1 + BULGE ** 2) / (4 * BULGE)) < 1e-12
assert abs(C0[1] - (6 - 10 * (1 - BULGE ** 2) / (4 * BULGE))) < 1e-12
SWEEP = 4 * math.atan(BULGE)  # the north wall's arc, counter-clockwise from C to D

# The corners' mitres (wall_join: every line of both cut at the mitre through the two faces' meeting points). The
# walls run counter-clockwise, A (0, 0) → B (10, 0) → C (10, 6) → D (0, 6) → A: their left faces inside.
A_OUT, A_IN = (-H, -H), (H, H)
B_OUT, B_IN = (10 + H, -H), (10 - H, H)
C_OUT, C_IN = on_circle(R + H, 10 + H), on_circle(R - H, 10 - H)
D_OUT, D_IN = on_circle(R + H, -H), on_circle(R - H, H)
THETA_OUT = angle(D_OUT) - angle(C_OUT)  # the outer face's arc between the mitres
THETA_IN = angle(D_IN) - angle(C_IN)


def body_area(name):
    """A wall's body in plan: its ring between its mitres, the arc's pieces with their segments."""
    if name == "south":
        return abs(shoelace([A_OUT, B_OUT, B_IN, A_IN]))
    if name == "east":
        return abs(shoelace([B_OUT, C_OUT, C_IN, B_IN]))
    if name == "west":
        return abs(shoelace([D_OUT, A_OUT, A_IN, D_IN]))
    # The north wall: its chords' quadrilateral, the outer arc's segment out of it, the inner arc's into it.
    return abs(shoelace([C_OUT, D_OUT, D_IN, C_IN])) + seg(R + H, THETA_OUT) - seg(R - H, THETA_IN)


# The loop's outer face less its inner face: the four bodies together.
OUTER_LOOP = abs(shoelace([A_OUT, B_OUT, C_OUT, D_OUT])) + seg(R + H, THETA_OUT)
INNER_LOOP = abs(shoelace([A_IN, B_IN, C_IN, D_IN])) + seg(R - H, THETA_IN)
assert abs(sum(body_area(n) for n in ("south", "east", "north", "west")) - (OUTER_LOOP - INNER_LOOP)) < 1e-12

d = {"format": "kentos.document", "version": 2, "name": "Eğri cepheli bina",
     "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                  "plotScale": 50.0, "workspace": "cad", "cadWork": "architecture", "arch": {"levels": LEVELS}},
     "origin": {"x": 0.0, "y": 0.0},
     "layers": [layer("0", "0"), layer("cizim", "Çizim"), layer("kapi", "Kapı ve pencere"),
                layer("doseme", "Döşeme")],
     "activeLayer": "cizim", "styles": {"items": [], "categories": []}}
ents, uids = [], []


def uid(i):
    return str(uuid.UUID(int=0x0192f5a1_8071_7000_8000_000000000000 + i))


def add(e):
    i = len(ents) + 1
    e["id"] = i
    e.setdefault("layerId", "cizim")
    e.setdefault("attrs", {})
    ents.append(e)
    uids.append(uid(i))
    return uid(i)


def straight_opening(host, a, b, at, width, form, sill, head, number):
    """An opening `at` metres from wall a→b's start, its jambs square to the wall, its place on both faces."""
    dx, dy = b[0] - a[0], b[1] - a[1]
    L = math.hypot(dx, dy)
    ux, uy = dx / L, dy / L
    nx, ny = -uy, ux
    s0 = (a[0] + ux * at, a[1] + uy * at)
    s1 = (a[0] + ux * (at + width), a[1] + uy * (at + width))
    place = [P(s0[0] + nx * H, s0[1] + ny * H), P(s0[0] - nx * H, s0[1] - ny * H),
             P(s1[0] + nx * H, s1[1] + ny * H), P(s1[0] - nx * H, s1[1] - ny * H)]
    e = {"kind": "opening", "layerId": "kapi", "host": host, "at": at, "anchor": "start", "width": width,
         "form": form, "frame": 0.05, "number": number, "place": place, "head": head}
    if sill is not None:
        e["sill"] = sill
    return add(e)


def arc_opening(host, middle, width, form, sill, head, number):
    """An opening centred `middle` radians round the curved wall, `width` along its line (its centre line), its jambs
    on the radii: its place on the inner (left) and outer faces at both jambs."""
    half = width / R / 2
    t0, t1 = middle - half, middle + half
    at = R * (t0 - math.atan2(6 - C0[1], 10 - C0[0]))
    pt = lambda r, t: P(C0[0] + r * math.cos(t), C0[1] + r * math.sin(t))
    place = [pt(R - H, t0), pt(R + H, t0), pt(R - H, t1), pt(R + H, t1)]
    return add({"kind": "opening", "layerId": "kapi", "host": host, "at": at, "anchor": "start", "width": width,
                "form": form, "frame": 0.05, "number": number, "place": place, "sill": sill, "head": head})


expected = {"walls": {}, "slabs": {}, "spaces": {}, "bodies": {}}
for lv in LEVELS:
    level = lv["id"]
    foot, top = WALLS[level]
    height = top - foot
    n = "0" if level == "zemin" else "1"

    def wall(name, pts, bulges=None):
        e = {"kind": "wall", "pts": [P(*p) for p in pts], "width": W, "justify": "center", "arch": {"level": level}}
        if bulges:
            e["bulges"] = bulges
        return add(e)

    south = wall("south", [(0, 0), (10, 0)])
    east = wall("east", [(10, 0), (10, 6)])
    north = wall("north", [(10, 6), (0, 6)], [BULGE, 0.0])
    west = wall("west", [(0, 6), (0, 0)])
    # Openings: their pieces of their walls, the curved one's a piece of the ring between its faces.
    window_s = (1.5, 0.9, 2.1)  # width, sill, head
    door_e = (1.0, None, 2.1)
    window_n = (1.2, 0.9, 2.1)
    straight_opening(south, (0, 0), (10, 0), 4.0, window_s[0], "window", window_s[1], window_s[2], f"P{n}1")
    straight_opening(east, (10, 0), (10, 6), 2.0, door_e[0], "door", door_e[1], door_e[2], f"K{n}1")
    arc_opening(north, math.pi / 2, window_n[0], "window", window_n[1], window_n[2], f"P{n}2")
    cut = {
        south: window_s[0] * W * (window_s[2] - window_s[1]),
        east: door_e[0] * W * door_e[2],
        north: ((R + H) ** 2 - (R - H) ** 2) / 2 * (window_n[0] / R) * (window_n[2] - window_n[1]),
        west: 0.0,
    }
    # The openings' share of each wall's middle plane: their widths along its centre line by their heights.
    side_cut = {south: window_s[0] * (window_s[2] - window_s[1]), east: door_e[0] * door_e[2],
                north: window_n[0] * (window_n[2] - window_n[1]), west: 0.0}
    length = {south: 10.0, east: 6.0, north: R * SWEEP, west: 6.0}
    # A wall's gross volume its own body's, without its joins (IFC 4.3: "without taking into account the openings and
    # the connection geometry"): its line's length by its width, a curved one's the ring's piece between its faces; its
    # net its body's between its mitres less its openings.
    own = {south: 10.0 * W, east: 6.0 * W, west: 6.0 * W, north: ((R + H) ** 2 - (R - H) ** 2) / 2 * SWEEP}
    for name, u in (("south", south), ("east", east), ("north", north), ("west", west)):
        expected["walls"][u] = {
            "Length": length[u], "Width": W, "Height": height,
            "NetSideArea": length[u] * height - side_cut[u],
            "GrossVolume": own[u] * height, "NetVolume": body_area(name) * height - cut[u],
        }
        expected["bodies"][u] = [foot, top]
    # The room: inside the inner faces, its north edge the inner face's arc.
    room = {"outer": {"pts": [P(*A_IN), P(*B_IN), P(*C_IN), P(*D_IN)],
                      "bulges": [0.0, 0.0, math.tan(THETA_IN / 4), 0.0]}}
    r = add({"kind": "space", "parts": [room], "number": f"{n}01", "name": "Salon", "arch": {"level": level}})
    # Its floor the slab's top under it, its ceiling the underside of the slab over it (ADR 8040 §3, 8071 §7): Zemin
    # 0.00 to 3.20, 1. kat 3.35 to 6.25. Its perimeter its outer ring's, the inner face's arc by its own length.
    ceiling = {"zemin": 3.35 - SLAB, "k1": 3.35 + ROOF_OFFSET - SLAB}[level]
    floor = {"zemin": 0.0, "k1": 3.35}[level]
    rise = ceiling - floor
    perimeter = (B_IN[0] - A_IN[0]) + 2 * (C_IN[1] - B_IN[1]) + (R - H) * THETA_IN
    expected["spaces"][r] = {"NetFloorArea": INNER_LOOP, "GrossFloorArea": INNER_LOOP, "GrossPerimeter": perimeter,
                             "Height": rise, "GrossVolume": INNER_LOOP * rise, "NetVolume": INNER_LOOP * rise}

# Slabs along the outer faces, the north edge the outer face's arc between the mitres.
outline = {"pts": [P(*A_OUT), P(*B_OUT), P(*C_OUT), P(*D_OUT)], "bulges": [0.0, 0.0, math.tan(THETA_OUT / 4), 0.0]}
ground = add({"kind": "slab", "layerId": "doseme", "parts": [{"outer": outline}], "thickness": SLAB,
              "material": "Betonarme", "arch": {"level": "zemin"},
              "layers": {"over": [{"kind": "finish", "thickness": FINISH}]}})
hole = [(2.0, 2.0), (4.0, 2.0), (4.0, 3.0), (2.0, 3.0)]
k1 = add({"kind": "slab", "layerId": "doseme", "parts": [{"outer": outline, "holes": [{"pts": [P(*p) for p in hole]}]}],
          "thickness": SLAB, "material": "Betonarme", "arch": {"level": "k1"}})
roof = add({"kind": "slab", "layerId": "doseme", "parts": [{"outer": outline}], "thickness": SLAB,
            "material": "Betonarme", "role": "terrace", "offset": ROOF_OFFSET, "arch": {"level": "k1"}})
for u, depth, holes, top in ((ground, SLAB + FINISH, 0.0, 0.0), (k1, SLAB, abs(shoelace(hole)), 3.35),
                             (roof, SLAB, 0.0, 3.35 + ROOF_OFFSET)):
    expected["slabs"][u] = {"Width": depth, "GrossArea": OUTER_LOOP, "NetArea": OUTER_LOOP - holes,
                            "GrossVolume": OUTER_LOOP * depth, "NetVolume": (OUTER_LOOP - holes) * depth}
    expected["bodies"][u] = [top - depth, top]

d["entities"], d["uids"] = ents, uids


def main():
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--check", action="store_true")
    a = p.parse_args()
    data = ref.container(ref.document(d))
    text = json.dumps(expected, ensure_ascii=False, indent=1, sort_keys=True) + "\n"
    if a.check:
        bad = [f.relative_to(ROOT) for f, want in ((OUT, data), (EXPECTED, text.encode("utf-8")))
               if not f.exists() or f.read_bytes() != want]
        if bad:
            print(f"{', '.join(map(str, bad))} başvuruyla aynı değil", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)}: {len(ents)} nesne, başvuruyla aynı.")
        return 0
    OUT.write_bytes(data)
    EXPECTED.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)}: {len(ents)} nesne yazıldı; {EXPECTED.relative_to(ROOT)} kapalı formlarıyla.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
