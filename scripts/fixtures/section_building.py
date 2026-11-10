#!/usr/bin/env python3
"""The section's test building (docs/adr/8042): two storeys (Zemin 0.00, 1. kat 3.00, 3.00 m each) on a 12 × 9 m plan,
its walls on the grids (Dış duvar 25 outside, İç duvar 15 inside, with their layers), a stair hall between x 4.5 and 7.5
with a U stair (18 risers of 3.00/18, treads 28 cm, 1.20 m wide, one landing), floor slabs with their layers (the
ground's on the ground, the 1. kat's with a hole over the stair), a terrace slab over the 1. kat, 30/30 columns where
the grids meet the outer walls, 25/50 beams on the outer walls under each slab, windows and doors with their sills and
heads, grids A–D and 1–3, rooms named and numbered on both storeys, and two section marks: A-A across the building
through the stair's flights, B-B along the first flight through the entrance door (PAİY 57(2)(a)(6): one through the
common stair).

Writes fixtures/arch/v1/section-building.kcad with the independent reference writer, without KentOS code; `--check`
compares. The application's section tests and the real-window acceptance read it."""
import argparse
import json
import math
import sys
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))
import kcad_v2_reference as ref  # noqa: E402

OUT = ROOT / "fixtures/arch/v1/section-building.kcad"


def layer(i, name):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


d = {"format": "kentos.document", "version": 2, "name": "Kesit binası",
     "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                  "plotScale": 50.0, "workspace": "cad", "cadWork": "architecture", "arch": {"levels": []}},
     "origin": {"x": 0.0, "y": 0.0},
     "layers": [layer("0", "0"), layer("cizim", "Çizim"), layer("kapi", "Kapı ve pencere"),
                layer("doseme", "Döşeme"), layer("kolon", "Kolon"), layer("kiris", "Kiriş"),
                layer("merdiven", "Merdiven"), layer("aks", "Aks"), layer("isaret", "İşaret")],
     "activeLayer": "cizim", "styles": {"items": [], "categories": []}}
d["settings"]["arch"]["levels"] = [
    {"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
    {"id": "k1", "name": "1. kat", "elevation": 3.0, "height": 3.0},
]
ents, uids = [], []


def uid(i):
    return str(uuid.UUID(int=0x0192f5a1_4242_7000_8000_000000000000 + i))


def add(e):
    i = len(ents) + 1
    e["id"] = i
    e.setdefault("layerId", "cizim")
    e.setdefault("attrs", {})
    ents.append(e)
    uids.append(uid(i))
    return uid(i)


def P(x, y):
    return {"x": float(x), "y": float(y)}


def rect(x0, y0, x1, y1):
    return {"outer": {"pts": [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]}}


OUTER = [
    {"name": "İç sıva", "material": "İç sıva", "thickness": 0.02, "detail": "detailed", "priority": 5},
    {"name": "Tuğla", "material": "Tuğla", "hatch": {"pattern": "ANSI31", "scale": 1.0, "angle": 0.0},
     "detail": "standard", "priority": 1},
    {"name": "Isı yalıtımı", "material": "Isı yalıtımı", "thickness": 0.05,
     "hatch": {"pattern": "SN55", "scale": 1.0, "angle": 0.0}, "detail": "standard", "priority": 3},
    {"name": "Dış sıva", "material": "Dış sıva", "thickness": 0.03, "detail": "detailed", "priority": 5},
]
INNER = [
    {"name": "Sıva", "material": "Sıva", "thickness": 0.02, "detail": "detailed", "priority": 5},
    {"name": "Tuğla", "material": "Tuğla", "hatch": {"pattern": "ANSI31", "scale": 1.0, "angle": 0.0},
     "detail": "standard", "priority": 1},
    {"name": "Sıva", "material": "Sıva", "thickness": 0.02, "detail": "detailed", "priority": 5},
]


def wall(a, b, outer, level):
    w = 0.25 if outer else 0.15
    return add({"kind": "wall", "pts": [P(*a), P(*b)], "width": w, "justify": "center",
                "style": "kentos-dis-duvar-25" if outer else "kentos-ic-duvar-15",
                "components": OUTER if outer else INNER,
                "fill": "#5A5A5A", "arch": {"level": level}})


def opening(host, a, b, w_width, at, width, form, sill, head, number, frame):
    """An opening `at` metres from wall a→b's start (its start jamb), its place on both faces."""
    dx, dy = b[0] - a[0], b[1] - a[1]
    L = math.hypot(dx, dy)
    ux, uy = dx / L, dy / L
    nx, ny = -uy, ux
    h = w_width / 2
    s0 = (a[0] + ux * at, a[1] + uy * at)
    s1 = (a[0] + ux * (at + width), a[1] + uy * (at + width))
    place = [P(s0[0] + nx * h, s0[1] + ny * h), P(s0[0] - nx * h, s0[1] - ny * h),
             P(s1[0] + nx * h, s1[1] + ny * h), P(s1[0] - nx * h, s1[1] - ny * h)]
    e = {"kind": "opening", "layerId": "kapi", "host": host, "at": at, "anchor": "start", "width": width,
         "form": form, "frame": frame, "number": number, "place": place, "head": head}
    if sill is not None:
        e["sill"] = sill
    return add(e)


hosts = {}
for level in ("zemin", "k1"):
    south = wall((0, 0), (12, 0), True, level)
    east = wall((12, 0), (12, 9), True, level)
    north = wall((12, 9), (0, 9), True, level)
    west = wall((0, 9), (0, 0), True, level)
    hall_w = wall((4.5, 0), (4.5, 9), False, level)
    hall_e = wall((7.5, 0), (7.5, 9), False, level)
    wall((0, 4.5), (4.5, 4.5), False, level)
    wall((7.5, 4.5), (12, 4.5), False, level)
    hosts[level] = dict(south=south, north=north, hall_w=hall_w, hall_e=hall_e)
    n = "0" if level == "zemin" else "1"
    # South windows: the bedroom's and the bathroom's small one.
    opening(south, (0, 0), (12, 0), 0.25, 1.5, 1.2, "window", 0.9, 2.2, f"P{n}1", 0.06)
    opening(south, (0, 0), (12, 0), 0.25, 9.6, 0.6, "window", 1.5, 2.2, f"P{n}2", 0.06)
    # North windows: the living room's and the kitchen's (the wall runs from x 12 to 0).
    opening(north, (12, 9), (0, 9), 0.25, 9.0, 1.5, "window", 0.9, 2.2, f"P{n}3", 0.06)
    opening(north, (12, 9), (0, 9), 0.25, 1.5, 1.2, "window", 0.9, 2.2, f"P{n}4", 0.06)
    # The hall's doors to the rooms.
    opening(hall_w, (4.5, 0), (4.5, 9), 0.15, 6.0, 0.9, "door", None, 2.1, f"K{n}1", 0.05)
    opening(hall_e, (7.5, 0), (7.5, 9), 0.15, 6.0, 0.9, "door", None, 2.1, f"K{n}2", 0.05)
    if level == "zemin":
        # The entrance on the hall's north wall, x 4.8 to 5.8 (the wall runs from x 12 to 0: 6.2 from its start).
        opening(north, (12, 9), (0, 9), 0.25, 6.2, 1.0, "door", None, 2.1, "GK01", 0.05)
    else:
        opening(north, (12, 9), (0, 9), 0.25, 6.2, 1.0, "window", 0.9, 2.2, "P15", 0.06)
    names = [("Yatak odası", 0.2, 0.2, 4.425, 4.425), ("Salon", 0.2, 4.575, 4.425, 8.8),
             ("Merdiven holü", 4.575, 0.2, 7.425, 8.8), ("Banyo", 7.575, 0.2, 11.8, 4.425),
             ("Mutfak", 7.575, 4.575, 11.8, 8.8)]
    for k, (name, x0, y0, x1, y1) in enumerate(names):
        add({"kind": "space", "parts": [rect(x0, y0, x1, y1)], "number": f"{n}0{k + 1}", "name": name,
             "arch": {"level": level}})

# Slabs: the ground floor's on the ground, the 1. kat's over the stair's hole, the terrace over the 1. kat.
add({"kind": "slab", "layerId": "doseme", "parts": [rect(-0.125, -0.125, 12.125, 9.125)], "thickness": 0.15,
     "material": "Betonarme", "arch": {"level": "zemin"},
     "layers": {"over": [{"kind": "finish", "thickness": 0.02}, {"kind": "screed", "thickness": 0.05},
                         {"kind": "insulation", "thickness": 0.05}, {"kind": "waterproofing", "thickness": 0.01}]}})
k1 = rect(-0.125, -0.125, 12.125, 9.125)
k1["holes"] = [{"pts": [P(4.575, 1.28), P(4.575, 5.22), P(7.425, 5.22), P(7.425, 1.28)]}]
add({"kind": "slab", "layerId": "doseme", "parts": [k1], "thickness": 0.15, "material": "Betonarme",
     "arch": {"level": "k1"},
     "layers": {"over": [{"kind": "finish", "thickness": 0.02}, {"kind": "screed", "thickness": 0.05}],
                "under": [{"kind": "plaster", "thickness": 0.02}]}})
add({"kind": "slab", "layerId": "doseme", "parts": [rect(-0.125, -0.125, 12.125, 9.125)], "thickness": 0.15,
     "material": "Betonarme", "role": "terrace", "offset": 3.0, "arch": {"level": "k1"},
     "layers": {"over": [{"kind": "finish", "thickness": 0.03}, {"kind": "waterproofing", "thickness": 0.01},
                         {"kind": "screed", "thickness": 0.05}, {"kind": "insulation", "thickness": 0.08}],
                "under": [{"kind": "plaster", "thickness": 0.02}]}})

# Columns at the grids' crossings on the outer walls, both storeys.
for level in ("zemin", "k1"):
    for x in (0.0, 4.5, 7.5, 12.0):
        for y in (0.0, 9.0):
            add({"kind": "column", "layerId": "kolon", "at": P(x, y), "width": 0.3, "fill": True,
                 "arch": {"level": level}})
    # Beams on the outer walls under the slab over the storey.
    for a, b in (((0, 0), (12, 0)), ((12, 0), (12, 9)), ((12, 9), (0, 9)), ((0, 9), (0, 0))):
        add({"kind": "beam", "layerId": "kiris", "pts": [P(*a), P(*b)], "width": 0.25, "height": 0.5,
             "arch": {"level": level}})

# The U stair: 9 risers up along +y at x 5.2, a landing, 9 down along −y at x 6.8 (walking line, docs/adr/0704).
add({"kind": "stair", "layerId": "merdiven", "pts": [P(5.2, 1.0), P(5.2, 4.62), P(6.8, 4.62), P(6.8, 1.28)],
     "width": 1.2, "risers": 18, "riserHeight": 3.0 / 18, "tread": 0.28, "landing": 2.2, "cut": 9,
     "arch": {"level": "zemin"}})

# Grids.
for name, a, b in (("A", (0, -2), (0, 11)), ("B", (4.5, -2), (4.5, 11)), ("C", (7.5, -2), (7.5, 11)),
                   ("D", (12, -2), (12, 11)), ("1", (-2, 0), (14, 0)), ("2", (-2, 4.5), (14, 4.5)),
                   ("3", (-2, 9), (14, 9))):
    add({"kind": "grid", "layerId": "aks", "pts": [P(*a), P(*b)], "name": name})

# The section marks: A-A across at y 2.4 looking north; B-B along x 5.2 from the north looking west.
add({"kind": "viewmark", "layerId": "isaret", "form": "section", "points": [P(-1.5, 2.4), P(13.5, 2.4)], "mark": "A",
     "words": {"top": "A", "bottom": "—"}})
add({"kind": "viewmark", "layerId": "isaret", "form": "section", "points": [P(5.2, 10.5), P(5.2, -1.5)], "mark": "B",
     "words": {"top": "B", "bottom": "—"}})

d["entities"], d["uids"] = ents, uids


def main():
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--check", action="store_true")
    a = p.parse_args()
    data = ref.container(ref.document(d))
    if a.check:
        if not OUT.exists() or OUT.read_bytes() != data:
            print(f"{OUT.relative_to(ROOT)} başvuruyla aynı değil", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)}: {len(ents)} nesne, başvuruyla aynı.")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_bytes(data)
    print(f"{OUT.relative_to(ROOT)}: {len(ents)} nesne yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
