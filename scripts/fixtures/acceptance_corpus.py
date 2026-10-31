#!/usr/bin/env python3
"""The acceptance corpus's other samples (plan/architecture-advanced.md ADV-07, docs/adr/0798): after the 2+1
apartment from an empty project (acceptance_flat.py, docs/adr/0776) a measured change on an existing plan, a
two-storey house, a curved plan with a shaft and a renovation over a DXF underlay. Each is a ``kentos.acceptance``
scenario the Python, MCP and desktop-agent runners replay as the apartment's (same step kinds: ``command``,
``refuse``, ``query``, ``reopen`` and their ``expect``s), a mouse-and-keyboard trace the desktop plays and a drawing
the trace starts from. Every value a scenario expects is worked out here from the plan's numbers, with fractions
and the plane's formulas, without KentOS code.

    python3 scripts/fixtures/acceptance_corpus.py              # write
    python3 scripts/fixtures/acceptance_corpus.py --check      # compare
    python3 scripts/fixtures/acceptance_corpus.py --compare D  # the hosts' saved drawings (D/<id>/*.kcad) alike
"""

from __future__ import annotations

import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from acceptance_flat import num, pt  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
ACCEPTANCE = ROOT / "fixtures/acceptance/v1"
TRACES = ROOT / "fixtures/interaction/v1"
LV = {"level": "zemin"}


def project(name: str) -> dict:
    return {"name": name, "workspace": "cad", "coords": "local", "drawingUnit": "m", "plotScale": 100}


def measures(areas: dict[str, F]) -> dict:
    return {k: {"area": num(v)} for k, v in areas.items()}


def partition(src: str, edge: int, side: str, d: F, width: F, between=None, level: str = "zemin", phase=None) -> dict:
    """``cad.walls.partition``'s input: ``d`` clear from the source's face, from one limit's path to the other's."""
    arch = {"level": level} | ({"phase": phase} if phase else {})
    i = {"source": {"wall": src, "edge": edge}, "side": side, "measure": "clear", "distance": num(d),
         "wall": {"layerId": "$layer", "walls": [{"pts": [], "closed": False, "width": num(width), "arch": arch}]}}
    if between:
        i["extent"] = {"kind": "between", "start": {"wall": between[0][0], "edge": between[0][1]},
                       "end": {"wall": between[1][0], "edge": between[1][1]}}
    return i


# ── 2. Ölçülü değişiklik: the 2+1's plan received and changed by measures ───────────────────────────────────
#
# The same plan as the apartment (acceptance_flat.py): an 11 × 8 m clear shell of 25 cm walls drawn by its clear
# faces, four 15 cm partitions (axes P1 x 4.575, P2 y 4.575, P3 x 8.225, P4 x 7.925), ten openings of KentOS's own
# kinds, five rooms, the south wall's dimension row, the slab bound to the shell's outer faces, tags and the rooms'
# schedule. Then the changes, each by a measure, nothing repaired by hand:
#
#   C1  Yatak 1 3.50 → 3.80 clear, Bu ölçüyü koru: P3's axis 8.525 and the bond P1's east face ↔ P3's west face.
#   C2  Salon 4.50 → 4.80 clear: P1's axis 4.875; the bond moves P3 with it (8.825), P2 starts at the new axis.
#   C3  The shell 25 → 30 cm, its path (the clear face) kept: the rooms stay, the slab 11.6 × 8.6.
#   C4  Yatak 1's door 0.10 from P1's face: its west jamb at 5.05, its centre 5.50 along P2 from 4.875.
#   C5  Yatak 2's window 0.60 from the east wall's clear face: its east jamb 10.40, its centre 9.80 on the south.
#
# Refusals: the same distance again (nothing_to_set), a negative distance on a wall (invalid_wall) and on an
# opening (invalid_opening); each writes nothing.

C_SHELL, C_PART = F(1, 4), F(3, 20)
C_X, C_Y = F(11), F(8)
H = C_PART / 2


def change_rooms(p1: F, p3: F, p2: F = F(9, 2) + H, p4: F = C_X - 3 - H) -> dict[str, F]:
    return {
        "salon": (p1 - H) * C_Y,
        "yatak1": (p3 - H - (p1 + H)) * (p2 - H),
        "yatak2": (C_X - (p3 + H)) * (p2 - H),
        "koridor": (p4 - H - (p1 + H)) * (C_Y - (p2 + H)),
        "banyo": (C_X - (p4 + H)) * (C_Y - (p2 + H)),
    }


CHANGE_SEEDS = {
    "salon": (F(2), F(2), "01", "Salon", "Salon"),
    "yatak1": (F(6), F(2), "02", "Yatak odası 1", "Yatak odası"),
    "yatak2": (F(10), F(2), "03", "Yatak odası 2", "Yatak odası"),
    "koridor": (F(6), F(6), "04", "Koridor", "Koridor"),
    "banyo": (F(19, 2), F(6), "05", "Banyo", "Islak hacim"),
}


def change_marks(windows: list[F]) -> list[F]:
    """The south row's pieces: the clear face's ends and the 1.20 m windows' jambs, west to east."""
    marks = [F(0)] + [c + d for c in sorted(windows) for d in (-F(3, 5), F(3, 5))] + [C_X]
    return [b - a for a, b in zip(marks, marks[1:])]


def change() -> dict:
    p1, p2, p4 = F(9, 2) + H, F(9, 2) + H, C_X - 3 - H
    p3 = p1 + H + F(7, 2) + H
    sh = C_SHELL / 2
    p3_c1 = p1 + H + F(19, 5) + H  # Yatak 1 3.80
    p1_c2 = F(24, 5) + H  # Salon 4.80
    p3_c2 = p1_c2 + H + F(19, 5) + H  # the bond holds Yatak 1's 3.80
    rooms0, rooms1, rooms2 = change_rooms(p1, p3), change_rooms(p1, p3_c1), change_rooms(p1_c2, p3_c2)
    windows = [F(9, 4), F(32, 5), F(97, 10)]
    w_c5 = C_X - F(3, 5) - F(3, 5)  # east jamb 0.60 from the clear face, centre 0.60 further
    door_c4 = p1_c2 + H + F(1, 10) + F(9, 20) - p1_c2  # its centre along P2 from P2's start (P1's axis)
    doors = [("p1", C_Y - F(63, 10), "door", F(9, 10)), ("p2", F(64, 10) - p1, "door", F(9, 10)),
             ("p2", F(97, 10) - p1, "door", F(9, 10)), ("p4", F(63, 10) - p2, "door", F(9, 10)),
             ("shell", C_X + C_Y + (C_X - F(62, 10)), "door", F(1))]
    wins = [("shell", c, "window", F(6, 5)) for c in windows] + [
        ("shell", C_X + F(63, 10), "window", F(3, 5)), ("shell", 2 * C_X + C_Y + (C_Y - 4), "window", F(6, 5))]
    names = ["d1", "d2", "d3", "d4", "entrance", "w1", "w2", "w3", "w4", "w5"]
    openings = [{"host": f"${h}", "at": num(a), "form": f, "width": num(w)} | ({"sill": 0.9} if f == "window" else {})
                for h, a, f, w in doors + wins]
    dims = ["dim1", "dim2", "dim3", "dim4", "dim5", "dim6", "dim7", "dimAll"]
    row = lambda ws: {f"dim{i + 1}": {"length": num(v)} for i, v in enumerate(change_marks(ws))} | {"dimAll": {"length": 11.0}}
    vert = lambda x, y0, y1: {"pts": [[num(x), num(y0)], [num(x), num(y1)]]}
    steps = [
        {"name": "Proje ayarları › Mimari: zemin kat, 3.00 m", "command": "cad.settings.set",
         "input": {"arch": {"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0, "height": 3.0}]}},
         "label": None, "expect": {"count": 0}},
        {"name": "Var olan plan: dış kontur İç net 11 × 8 m, 25 cm", "command": "cad.walls.rectangle",
         "input": {"layerId": "$layer", "frame": {"kind": "corners", "a": pt(F(0), F(0)), "b": pt(C_X, C_Y)},
                   "width": num(C_SHELL), "arch": LV}, "as": "shell", "label": "Duvar", "expect": {"count": 1}},
        {"name": "Var olan plan: Bölme 1, batı yüzden 4.50", "command": "cad.walls.partition",
         "input": partition("$shell", 3, "left", F(9, 2), C_PART), "as": "p1", "label": "Duvar",
         "expect": {"objects": {"p1": vert(p1, C_Y, F(0))}}},
        {"name": "Var olan plan: Bölme 2, güney yüzden 4.50", "command": "cad.walls.partition",
         "input": partition("$shell", 0, "left", F(9, 2), C_PART, (("$p1", 0), ("$shell", 1))), "as": "p2", "label": "Duvar",
         "expect": {"objects": {"p2": {"pts": [[num(p1), num(p2)], [num(C_X + sh), num(p2)]]}}}},
        {"name": "Var olan plan: Bölme 3, Bölme 1'den 3.50", "command": "cad.walls.partition",
         "input": partition("$p1", 0, "left", F(7, 2), C_PART, (("$shell", 0), ("$p2", 0))), "as": "p3", "label": "Duvar",
         "expect": {"objects": {"p3": vert(p3, p2, -sh)}}},
        {"name": "Var olan plan: Bölme 4, doğu yüzden 3.00", "command": "cad.walls.partition",
         "input": partition("$shell", 1, "left", F(3), C_PART, (("$p2", 0), ("$shell", 2))), "as": "p4", "label": "Duvar",
         "expect": {"count": 5, "objects": {"p4": vert(p4, p2, C_Y + sh)}}},
        {"name": "Var olan plan: beş kapı, beş pencere", "command": "cad.openings.create",
         "input": {"layerId": "$layer", "openings": openings}, "as": names, "label": "Kapı",
         "expect": {"count": 15, "objects": {n: {"kind": "opening", "at": num(a), "width": num(w), "form": f}
                                             for n, (h, a, f, w) in zip(names, doors + wins)}}},
        {"name": "Var olan plan: beş mahal", "command": "cad.spaces.create",
         "input": {"layerId": "$layer", "spaces": [{"seed": pt(x, y), "number": n, "name": nm, "usage": u, "arch": LV}
                                                    for x, y, n, nm, u in CHANGE_SEEDS.values()]},
         "as": list(CHANGE_SEEDS), "label": "Mahal", "expect": {"count": 20, "measures": measures(rooms0)}},
        {"name": "Var olan plan: güney duvarının ölçü sırası", "command": "cad.dimensionsets.create",
         "input": {"layerId": "$layer", "wall": "$shell", "edge": 0, "at": pt(F(11, 2), -F(3, 2)), "rows": ["openings", "overall"]},
         "as": dims, "label": "Otomatik duvar ölçüleri", "expect": {"count": 28, "measures": row(windows)}},
        {"name": "Var olan plan: dış yüzlere bağlı döşeme", "command": "cad.slabs.create",
         "input": {"layerId": "$layer", "slabs": [{"seed": pt(F(2), F(2)), "bound": True, "arch": LV}]},
         "as": "slab", "label": "Döşeme", "expect": {"count": 29, "measures": {"slab": {"area": num((C_X + 2 * C_SHELL) * (C_Y + 2 * C_SHELL))}}}},
        {"name": "Var olan plan: mahal etiketleri", "command": "cad.tags.create",
         "input": {"layerId": "$layer", "all": {"kinds": ["space"]}}, "label": "Etiket", "expect": {"count": 34}},
        {"name": "Var olan plan: mahal çizelgesi", "command": "cad.schedules.create",
         "input": {"layerId": "$layer", "at": pt(F(14), F(8)), "of": "spaces"}, "as": "schedule",
         "label": "Çizelge", "expect": {"count": 35, "output": {"rows": 5}}},
        {"name": "Değişiklik 1: Yatak 1 net 3.50 → 3.80, Bu ölçüyü koru", "command": "cad.walls.edit",
         "input": {"uids": ["$p3"], "operation": {"kind": "distance", "side": "right", "distance": 3.8, "keep": True}},
         "label": "Duvarı düzenle",
         "expect": {"count": 35, "measures": measures(rooms1), "objects": {"p3": vert(p3_c1, p2, -sh)},
                    "output": {"effects": {"right": {"before": 3.5, "after": 3.8}, "left": {"before": 2.7, "after": 2.4}}}}},
        {"name": "Ret: aynı mesafe yeniden yazılmaz", "command": "cad.walls.edit",
         "input": {"uids": ["$p3"], "operation": {"kind": "distance", "side": "right", "distance": 3.8}},
         "refuse": "nothing_to_set", "expect": {"count": 35}},
        {"name": "Değişiklik 2: Salon net 4.50 → 4.80; bağ Bölme 3'ü birlikte götürür", "command": "cad.walls.edit",
         "input": {"uids": ["$p1"], "operation": {"kind": "distance", "side": "right", "distance": 4.8}},
         "label": "Duvarı düzenle",
         "expect": {"count": 35, "measures": measures(rooms2),
                    "objects": {"p1": vert(p1_c2, C_Y, F(0)), "p3": vert(p3_c2, p2, -sh),
                                "p2": {"pts": [[num(p1_c2), num(p2)], [num(C_X + sh), num(p2)]]}}}},
        {"name": "Ret: eksi mesafe", "command": "cad.walls.edit",
         "input": {"uids": ["$p1"], "operation": {"kind": "distance", "side": "right", "distance": -1.0}},
         "refuse": "invalid_wall", "expect": {"count": 35, "measures": {"salon": {"area": num(rooms2["salon"])}}}},
        {"name": "Değişiklik 3: dış duvar 25 → 30 cm, iç yüz yerinde", "command": "cad.walls.edit",
         "input": {"uids": ["$shell"], "operation": {"kind": "set", "width": 0.3, "keep": "path"}},
         "label": "Duvarı düzenle",
         "expect": {"count": 35, "measures": measures(rooms2) | {"slab": {"area": num((C_X + F(3, 5)) * (C_Y + F(3, 5)))}},
                    "objects": {"shell": {"width": 0.3}}}},
        {"name": "Değişiklik 4: Yatak 1 kapısı Bölme 1'in yüzünden 0.10", "command": "cad.openings.edit",
         "input": {"uids": ["$d2"], "operation": {"kind": "distance", "side": "start", "face": "left", "distance": 0.1}},
         "label": "Açıklığı düzenle", "expect": {"count": 35, "objects": {"d2": {"at": num(door_c4), "host": "$p2"}}}},
        {"name": "Ret: eksi mesafe açıklıkta", "command": "cad.openings.edit",
         "input": {"uids": ["$d2"], "operation": {"kind": "distance", "side": "start", "face": "left", "distance": -0.1}},
         "refuse": "invalid_opening", "expect": {"count": 35, "objects": {"d2": {"at": num(door_c4)}}}},
        {"name": "Değişiklik 5: Yatak 2 penceresi doğu yüzden 0.60", "command": "cad.openings.edit",
         "input": {"uids": ["$w3"], "operation": {"kind": "distance", "side": "end", "face": "left", "distance": 0.6}},
         "label": "Açıklığı düzenle",
         "expect": {"count": 35, "objects": {"w3": {"at": num(w_c5)}}, "measures": row([windows[0], windows[1], w_c5])}},
        {"name": "Metraj: değişikliklerden sonra", "query": "cad.spaces.quantities", "input": {},
         "expect": {"output": {"totals": {"spaceArea": num(sum(rooms2.values())), "openings": 10,
                                          "slabArea": num((C_X + F(3, 5)) * (C_Y + F(3, 5)))}}}},
        {"name": "Kaydet ve aç: aynı kimlikler, aynı nesneler", "reopen": True,
         "expect": {"count": 35, "kinds": {"wall": 5, "opening": 10, "space": 5, "dimension": 8, "slab": 1, "tag": 5, "table": 1},
                    "measures": measures(rooms2) | row([windows[0], windows[1], w_c5])}},
    ]
    return {"format": "kentos.acceptance", "version": 1, "id": "change",
            "title": "Ölçülü değişiklik: var olan 2+1'de net ölçülerle bölme, dış duvar, kapı ve pencere değişiklikleri",
            "source": "plan/architecture-advanced.md ADV-07 (kabul korpusu); docs/adr/0798", "project": project("Değişiklik"),
            "tolerance": 1e-9, "undone": 10, "steps": steps,
            "rooms": {"before": {k: num(v) for k, v in rooms0.items()}, "after": {k: num(v) for k, v in rooms2.items()}}}


# ── 3. İki katlı konut: two levels, a stair bound to the ground floor's height, a stairwell through the slab ─────
#
# Levels: Zemin kat (0, 3.00) and 1. kat (3.00, 3.00). On each level a 10 × 8 m clear shell of 25 cm walls drawn
# by its clear faces and a 15 cm partition 5.00 clear from the west face (axis x 5.075). The ground floor: Salon
# [0, 5] × [0, 8] 40.00 m², Mutfak [5.15, 10] × [0, 8] 38.80 m²; a straight stair along the north face, 1.00 wide,
# 0.28 treads, bound to the level's height: ⌈3.00 / 0.175⌉ = 18 risers of 3.00 / 18, its 17 treads 4.76 long from
# x 0.20. Upstairs a second partition 1.20 clear from the north face, from the first partition to the west wall
# (axis y 6.725): Yatak 1 [0, 5] × [0, 6.65] 33.25 m², the stairwell [0, 5] × [6.80, 8] 6.00 m² a void, Yatak 2
# 38.80 m². Each level's slab bound to its shell's outer faces, 10.5 × 8.5 = 89.25 m²; the upper one detached and
# the stairwell cut from it: 83.25 m². The design change: the ground floor 3.00 → 3.24 high, 1. kat at 3.24: the
# stair keeps its 18 risers, each 0.18. Refusals: cutting a bound slab (slab_bound), a level in use removed
# (level_in_use).

K_SHELL, K_PART = F(1, 4), F(3, 20)
K_X, K_Y = F(10), F(8)


def house() -> dict:
    h = K_PART / 2
    a = F(5) + h
    b = K_Y - F(6, 5) - h
    sh = K_SHELL / 2
    up = {"level": "kat1"}
    levels = lambda z_h, k_e: [{"id": "zemin", "name": "Zemin kat", "elevation": 0, "height": num(z_h)},
                               {"id": "kat1", "name": "1. kat", "elevation": num(k_e), "height": 3.0}]
    ground = {"salon": F(5) * K_Y, "mutfak": (K_X - (a + h)) * K_Y}
    upper = {"yatak1": F(5) * (b - h), "yatak2": (K_X - (a + h)) * K_Y}
    void = F(5) * (K_Y - (b + h))
    slab = (K_X + 2 * K_SHELL) * (K_Y + 2 * K_SHELL)
    risers = math.ceil(F(3) / F(7, 40))
    stair_end = F(1, 5) + (risers - 1) * F(7, 25)
    stair = {"risers": risers, "riserHeight": num(F(3) / risers),
             "pts": [[0.2, 7.4], [num(stair_end), 7.4]]}
    count = 0

    def n(k: int) -> int:
        nonlocal count
        count += k
        return count

    rect = lambda name, lv: {"layerId": "$layer", "frame": {"kind": "corners", "a": pt(F(0), F(0)), "b": pt(K_X, K_Y)},
                             "width": num(K_SHELL), "arch": lv}
    steps = [
        {"name": "Proje ayarları › Mimari: Zemin kat ve 1. kat, 3.00 m", "command": "cad.settings.set",
         "input": {"arch": {"levels": levels(F(3), F(3))}}, "label": None, "expect": {"count": 0}},
        {"name": "Zemin kat: dış kontur İç net 10 × 8 m", "command": "cad.walls.rectangle", "input": rect("shell", LV),
         "as": "shell", "label": "Duvar", "expect": {"count": n(1), "objects": {"shell": {"arch": LV}}}},
        {"name": "Zemin kat: bölme batı yüzden 5.00", "command": "cad.walls.partition",
         "input": partition("$shell", 3, "left", F(5), K_PART), "as": "a", "label": "Duvar",
         "expect": {"count": n(1), "objects": {"a": {"pts": [[num(a), 8.0], [num(a), 0.0]], "arch": LV}}}},
        {"name": "Zemin kat: giriş kapısı, pencere, iç kapı", "command": "cad.openings.create",
         "input": {"layerId": "$layer", "openings": [
             {"host": "$shell", "at": 7.5, "form": "door", "width": 1.0},
             {"host": "$shell", "at": 2.5, "form": "window", "width": 1.2, "sill": 0.9},
             {"host": "$a", "at": 4.0, "form": "door", "width": 0.9}]},
         "as": ["entrance", "w1", "d1"], "label": "Kapı", "expect": {"count": n(3)}},
        {"name": "Zemin kat: kat yüksekliğine bağlı merdiven, 1.00 geniş, 0.28 basamak", "command": "cad.stairs.create",
         "input": {"layerId": "$layer", "stairs": [{"pts": [pt(F(1, 5), F(37, 5)), pt(F(5), F(37, 5))], "width": 1.0,
                                                     "tread": 0.28, "fit": True, "heightFromLevel": True, "arch": LV}]},
         "as": "stair", "label": "Merdiven", "expect": {"count": n(1), "objects": {"stair": stair}}},
        {"name": "Zemin kat: Salon ve Mutfak", "command": "cad.spaces.create",
         "input": {"layerId": "$layer", "spaces": [
             {"seed": pt(F(5, 2), F(3)), "number": "Z01", "name": "Salon", "usage": "Salon", "arch": LV},
             {"seed": pt(F(15, 2), F(4)), "number": "Z02", "name": "Mutfak", "usage": "Mutfak", "arch": LV}]},
         "as": list(ground), "label": "Mahal", "expect": {"count": n(2), "measures": measures(ground)}},
        {"name": "1. kat: dış kontur İç net 10 × 8 m", "command": "cad.walls.rectangle", "input": rect("shell1", up),
         "as": "shell1", "label": "Duvar", "expect": {"count": n(1), "objects": {"shell1": {"arch": up}}}},
        {"name": "1. kat: bölme batı yüzden 5.00", "command": "cad.walls.partition",
         "input": partition("$shell1", 3, "left", F(5), K_PART, level="kat1"), "as": "a1", "label": "Duvar",
         "expect": {"count": n(1), "objects": {"a1": {"pts": [[num(a), 8.0], [num(a), 0.0]], "arch": up}}}},
        {"name": "1. kat: merdiven boşluğunun bölmesi kuzey yüzden 1.20", "command": "cad.walls.partition",
         "input": partition("$shell1", 2, "left", F(6, 5), K_PART, (("$a1", 0), ("$shell1", 3)), level="kat1"),
         "as": "b", "label": "Duvar",
         "expect": {"count": n(1), "objects": {"b": {"pts": [[num(a), num(b)], [num(-sh), num(b)]], "arch": up}}}},
        {"name": "1. kat: iç kapı, iki pencere", "command": "cad.openings.create",
         "input": {"layerId": "$layer", "openings": [
             {"host": "$a1", "at": 5.0, "form": "door", "width": 0.9},
             {"host": "$shell1", "at": 2.5, "form": "window", "width": 1.2, "sill": 0.9},
             {"host": "$shell1", "at": 7.5, "form": "window", "width": 1.2, "sill": 0.9}]},
         "as": ["d2", "w2", "w3"], "label": "Kapı", "expect": {"count": n(3)}},
        {"name": "1. kat: iki yatak odası ve merdiven boşluğu (boşluk)", "command": "cad.spaces.create",
         "input": {"layerId": "$layer", "spaces": [
             {"seed": pt(F(5, 2), F(3)), "number": "101", "name": "Yatak odası 1", "usage": "Yatak odası", "arch": up},
             {"seed": pt(F(5, 2), F(37, 5)), "number": "102", "name": "Merdiven boşluğu", "void": True, "arch": up},
             {"seed": pt(F(15, 2), F(4)), "number": "103", "name": "Yatak odası 2", "usage": "Yatak odası", "arch": up}]},
         "as": ["yatak1", "well", "yatak2"], "label": "Mahal",
         "expect": {"count": n(3), "measures": measures(upper) | {"well": {"area": num(void)}}, "objects": {"well": {"void": True}}}},
        {"name": "Zemin kat: dış yüzlere bağlı döşeme", "command": "cad.slabs.create",
         "input": {"layerId": "$layer", "slabs": [{"seed": pt(F(5, 2), F(3)), "bound": True, "arch": LV}]},
         "as": "slab0", "label": "Döşeme", "expect": {"count": n(1), "measures": {"slab0": {"area": num(slab)}}}},
        {"name": "1. kat: dış yüzlere bağlı döşeme", "command": "cad.slabs.create",
         "input": {"layerId": "$layer", "slabs": [{"seed": pt(F(5, 2), F(3)), "bound": True, "arch": up}]},
         "as": "slab1", "label": "Döşeme", "expect": {"count": n(1), "measures": {"slab1": {"area": num(slab)}}}},
        {"name": "Ret: bağlı döşemeden boşluk kesilmez", "command": "cad.slabs.edit",
         "input": {"uids": ["$slab1"], "operation": {"kind": "subtract", "seed": pt(F(5, 2), F(37, 5))}},
         "refuse": "slab_bound", "expect": {"count": count, "measures": {"slab1": {"area": num(slab)}}}},
        {"name": "1. kat döşemesi kaynaktan ayrılır", "command": "cad.slabs.edit",
         "input": {"uids": ["$slab1"], "operation": {"kind": "detach"}}, "label": "Döşemeyi düzenle",
         "expect": {"count": count, "measures": {"slab1": {"area": num(slab)}}}},
        {"name": "1. kat döşemesinden merdiven boşluğu kesilir", "command": "cad.slabs.edit",
         "input": {"uids": ["$slab1"], "operation": {"kind": "subtract", "seed": pt(F(5, 2), F(37, 5))}},
         "label": "Döşemeyi düzenle", "expect": {"count": count, "measures": {"slab1": {"area": num(slab - void)}}}},
        {"name": "Etiketler: bütün mahaller", "command": "cad.tags.create",
         "input": {"layerId": "$layer", "all": {"kinds": ["space"]}}, "label": "Etiket", "expect": {"count": n(5)}},
        {"name": "Paftalar: her kat için bir plan", "command": "cad.sheets.levels", "input": {}, "label": None,
         "expect": {"count": count, "output": {"sheets": [
             {"name": "A-101 Zemin kat planı", "maps": [{"level": "zemin"}]},
             {"name": "A-102 1. kat planı", "maps": [{"level": "kat1"}]}]}}},
        {"name": "Metraj: iki katın toplamları", "query": "cad.spaces.quantities", "input": {},
         "expect": {"output": {"totals": {"spaceArea": num(sum(ground.values()) + sum(upper.values())), "openings": 6,
                                          "slabArea": num(2 * slab - void)}}}},
        {"name": "Ret: kullanılan kat kaldırılmaz", "command": "cad.settings.set",
         "input": {"arch": {"levels": levels(F(3), F(3))[:1]}}, "refuse": "level_in_use", "expect": {"count": count}},
        {"name": "Tasarım değişikliği: zemin kat 3.00 → 3.24; merdiven 18 rıhtla izler", "command": "cad.settings.set",
         "input": {"arch": {"levels": levels(F(81, 25), F(81, 25))}}, "label": None,
         "expect": {"count": count, "objects": {"stair": {"risers": risers, "riserHeight": num(F(81, 25) / risers),
                                                          "pts": stair["pts"]}}}},
        {"name": "Kaydet ve aç: aynı kimlikler, aynı nesneler", "reopen": True,
         "expect": {"count": count, "kinds": {"wall": 5, "opening": 6, "stair": 1, "space": 5, "slab": 2, "tag": 5},
                    "measures": measures(ground) | measures(upper) | {"slab0": {"area": num(slab)}, "slab1": {"area": num(slab - void)}},
                    "objects": {"stair": {"riserHeight": num(F(81, 25) / risers)}}}},
    ]
    return {"format": "kentos.acceptance", "version": 1, "id": "house",
            "title": "İki katlı konut: iki kat, kata bağlı merdiven, merdiven boşluğu, kat paftaları, kat yüksekliği değişikliği",
            "source": "plan/architecture-advanced.md ADV-07 (kabul korpusu); docs/adr/0798", "project": project("Konut"),
            "tolerance": 1e-9, "undone": 10, "steps": steps,
            "rooms": {"after": {k: num(v) for k, v in (ground | upper).items()}}}


# ── 4. Eğrisel ve şaftlı zor örnek: a semicircular bay, a shaft, a spiral stair ─────────────────────────────
#
# A 10 × 8 m path of 25 cm walls drawn centred, its east edge a semicircle (bulge 1, radius 4 about (10, 4)): the
# inner faces 0.125 in, the bay's inner face of radius 3.875, its outer 4.125. A 15 cm partition 4.00 clear from the
# west face (axis x 4.20); a 15 cm L wall closing a 1 × 1 m clear shaft in the corner of the partition and the north
# wall (axes x 5.35 and y 6.80). The rooms: Yatak [0.125, 4.125] × [0.125, 7.875] 31.00 m²; Salon the rest of the
# rectangle east of the partition, the half disc of radius 3.875 and less the shaft with its walls' bodies (1.15 m
# square); the shaft a void of 1.00 m². The spiral stair in the bay: its walking line of radius 1.50 about (10, 4)
# from (11.5, 4) counter-clockwise, 18 risers of 3.00 / 18 and 17 treads of 0.28: the arc turns 4.76 / 1.5 rad. The
# slab bound to the outer faces: [-0.125, 10] × [-0.125, 8.125] and the half disc of radius 4.125. A window on the
# bay at its middle (10 + 2π along the path). Metraj's slab area is net of the shaft standing on it
# (docs/adr/0722). Refusals: Kaydır and a measured distance on the bay's arc
# (invalid_wall, invalid_opening). The design change: the partition 4.00 → 3.60 from the west face; the shaft's L
# wall's end follows it (the shaft 1.40 × 1.00), the rooms with it. Before it the bay's window moved 1.00 back
# along the arc, the stair narrowed to 0.90 and given a 0.20 centre post, its walking line kept.

Q_W, Q_PART = 0.25, 0.15


def curved() -> dict:
    t = Q_W / 2
    hp = Q_PART / 2
    r_in, r_out = 4 - t, 4 + t

    def rooms(clear_west: float) -> dict[str, float]:
        p1 = t + clear_west + hp
        shaft_x = 5.35 + hp  # the shaft's east body face
        salon = (10 - (p1 + hp)) * (8 - 2 * t) + math.pi * r_in ** 2 / 2 - (shaft_x - (p1 + hp)) * (8 - t - (6.8 - hp))
        return {"yatak": clear_west * (8 - 2 * t), "salon": salon,
                "shaft": ((5.35 - hp) - (p1 + hp)) * ((8 - t) - (6.8 + hp))}

    before, after = rooms(4.0), rooms(3.6)
    slab = (10 + t) * (8 + 2 * t) + math.pi * r_out ** 2 / 2
    theta = 17 * 0.28 / 1.5
    stair = {"risers": 18, "riserHeight": 3.0 / 18,
             "pts": [[11.5, 4.0], [10 + 1.5 * math.cos(theta), 4 + 1.5 * math.sin(theta)]],
             "bulges": [math.tan(theta / 4), 0.0]}
    arc_mid = 10 + 2 * math.pi
    count = 0

    def n(k: int) -> int:
        nonlocal count
        count += k
        return count

    m = lambda a: {k: {"area": v} for k, v in a.items()}
    steps = [
        {"name": "Proje ayarları › Mimari: zemin kat, 3.00 m", "command": "cad.settings.set",
         "input": {"arch": {"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0, "height": 3.0}]}},
         "label": None, "expect": {"count": 0}},
        {"name": "Dış duvar: 10 × 8 m, doğusu yarım daire cumba (büküm 1), ortalı 25 cm", "command": "cad.walls.create",
         "input": {"layerId": "$layer", "walls": [{"pts": [pt(F(0), F(0)), pt(F(10), F(0)), pt(F(10), F(8)), pt(F(0), F(8))],
                                                   "bulges": [0.0, 1.0, 0.0, 0.0], "closed": True, "width": Q_W, "arch": LV}]},
         "as": "shell", "label": "Duvar", "expect": {"count": n(1), "objects": {"shell": {"bulges": [0.0, 1.0, 0.0, 0.0]}}}},
        {"name": "Bölme: batı yüzden 4.00 net", "command": "cad.walls.partition",
         "input": partition("$shell", 3, "left", F(4), F(3, 20)), "as": "p1", "label": "Duvar",
         "expect": {"count": n(1), "objects": {"p1": {"pts": [[4.2, 8.0], [4.2, 0.0]]}}}},
        {"name": "Şaft: bölme ile kuzey duvarı köşesinde 1 × 1 m net, L duvar", "command": "cad.walls.create",
         "input": {"layerId": "$layer", "walls": [{"pts": [pt(F(107, 20), F(8)), pt(F(107, 20), F(34, 5)), pt(F(21, 5), F(34, 5))],
                                                   "closed": False, "width": Q_PART, "arch": LV}]},
         "as": "shaftWall", "label": "Duvar", "expect": {"count": n(1)}},
        {"name": "Açıklıklar: iç kapı, cumbanın ortasında pencere, batı pencere", "command": "cad.openings.create",
         "input": {"layerId": "$layer", "openings": [
             {"host": "$p1", "at": 5.0, "form": "door", "width": 0.9},
             {"host": "$shell", "at": arc_mid, "form": "window", "width": 1.2, "sill": 0.9},
             {"host": "$shell", "at": 24 + 4 * math.pi, "form": "window", "width": 1.2, "sill": 0.9}]},
         "as": ["door", "bay", "west"], "label": "Kapı",
         "expect": {"count": n(3), "objects": {"bay": {"host": "$shell", "at": arc_mid}}}},
        {"name": "Mahaller: Yatak, cumbalı Salon, şaft (boşluk)", "command": "cad.spaces.create",
         "input": {"layerId": "$layer", "spaces": [
             {"seed": pt(F(2), F(4)), "number": "01", "name": "Yatak odası", "usage": "Yatak odası", "arch": LV},
             {"seed": pt(F(8), F(4)), "number": "02", "name": "Salon", "usage": "Salon", "arch": LV},
             {"seed": pt(F(191, 40), F(59, 8)), "number": "03", "name": "Tesisat şaftı", "void": True, "arch": LV}]},
         "as": ["yatak", "salon", "shaft"], "label": "Mahal", "expect": {"count": n(3), "measures": m(before)}},
        {"name": "Döner merdiven: cumbada, kat yüksekliğine bağlı, yürüme çizgisi 1.50", "command": "cad.stairs.create",
         "input": {"layerId": "$layer", "stairs": [{"pts": [pt(F(23, 2), F(4)), pt(F(17, 2), F(4))], "bulges": [1.0, 0.0],
                                                     "width": 1.0, "tread": 0.28, "fit": True, "heightFromLevel": True, "arch": LV}]},
         "as": "stair", "label": "Merdiven", "expect": {"count": n(1), "objects": {"stair": stair}}},
        {"name": "Döşeme: dış yüzlere bağlı, cumbayla", "command": "cad.slabs.create",
         "input": {"layerId": "$layer", "slabs": [{"seed": pt(F(2), F(4)), "bound": True, "arch": LV}]},
         "as": "slab", "label": "Döşeme", "expect": {"count": n(1), "measures": {"slab": {"area": slab}}}},
        {"name": "Etiketler: bütün mahaller", "command": "cad.tags.create",
         "input": {"layerId": "$layer", "all": {"kinds": ["space"]}}, "label": "Etiket", "expect": {"count": n(3)}},
        {"name": "Çizelge: mahal", "command": "cad.schedules.create",
         "input": {"layerId": "$layer", "at": pt(F(17), F(8)), "of": "spaces"}, "as": "schedule",
         "label": "Çizelge", "expect": {"count": n(1), "output": {"rows": 3}}},
        {"name": "Metraj: cumbalı alanlar", "query": "cad.spaces.quantities", "input": {},
         "expect": {"output": {"totals": {"spaceArea": before["yatak"] + before["salon"], "slabArea": slab - before["shaft"], "openings": 3}}}},
        {"name": "Cumbanın penceresi yay boyunca 1.00 geri", "command": "cad.openings.edit",
         "input": {"uids": ["$bay"], "operation": {"kind": "move", "by": -1.0}}, "label": "Açıklığı düzenle",
         "expect": {"count": count, "objects": {"bay": {"at": arc_mid - 1, "host": "$shell"}}}},
        {"name": "Ret: yolun sonunu aşan pencere", "command": "cad.openings.create",
         "input": {"layerId": "$layer", "openings": [{"host": "$shell", "at": 100.0, "form": "window", "width": 1.2}]},
         "refuse": "opening_off_wall", "expect": {"count": count}},
        {"name": "Döner merdiven 0.90 genişliğe", "command": "cad.arch.edit",
         "input": {"uids": ["$stair"], "operation": {"kind": "set", "width": 0.9}}, "label": "Mimari nesneyi düzenle",
         "expect": {"count": count, "objects": {"stair": stair | {"width": 0.9}}}},
        {"name": "Döner merdivene 0.20 merkez direği", "command": "cad.arch.edit",
         "input": {"uids": ["$stair"], "operation": {"kind": "set", "post": 0.2}}, "label": "Mimari nesneyi düzenle",
         "expect": {"count": count, "objects": {"stair": stair | {"width": 0.9, "post": 0.2}}}},
        {"name": "Ret: cumbanın yayı Kaydır'la kaydırılmaz", "command": "cad.walls.edit",
         "input": {"uids": ["$shell"], "operation": {"kind": "shift", "by": 0.5, "edge": 1}},
         "refuse": "invalid_wall", "expect": {"count": count, "measures": m(before)}},
        {"name": "Ret: yaydaki pencereye mesafe verilmez", "command": "cad.openings.edit",
         "input": {"uids": ["$bay"], "operation": {"kind": "distance", "side": "start", "face": "left", "distance": 0.5}},
         "refuse": "invalid_opening", "expect": {"count": count, "objects": {"bay": {"at": arc_mid - 1}}}},
        {"name": "Tasarım değişikliği: bölme batı yüzden 4.00 → 3.60; şaftın duvarı izler", "command": "cad.walls.edit",
         "input": {"uids": ["$p1"], "operation": {"kind": "distance", "side": "right", "distance": 3.6}},
         "label": "Duvarı düzenle",
         "expect": {"count": count, "measures": m(after), "objects": {"p1": {"pts": [[3.8, 8.0], [3.8, 0.0]]}}}},
        {"name": "Metraj: değişiklikten sonra", "query": "cad.spaces.quantities", "input": {},
         "expect": {"output": {"totals": {"spaceArea": after["yatak"] + after["salon"], "slabArea": slab - after["shaft"]}}}},
        {"name": "Kaydet ve aç: aynı kimlikler, aynı nesneler", "reopen": True,
         "expect": {"count": count, "kinds": {"wall": 3, "opening": 3, "space": 3, "stair": 1, "slab": 1, "tag": 3, "table": 1},
                    "measures": m(after) | {"slab": {"area": slab}},
                    "objects": {"stair": stair | {"width": 0.9, "post": 0.2}, "bay": {"at": arc_mid - 1}}}},
    ]
    return {"format": "kentos.acceptance", "version": 1, "id": "curved",
            "title": "Eğrisel ve şaftlı zor örnek: yarım daire cumba, tesisat şaftı, döner merdiven, yaydaki pencere",
            "source": "plan/architecture-advanced.md ADV-07 (kabul korpusu); docs/adr/0798", "project": project("Cumba"),
            "tolerance": 1e-9, "undone": 8, "steps": steps, "rooms": {"after": after}}


# ── 5. Tadilat: a DXF underlay, its wall lines made existing walls, a wall demolished, a new one ───────────────
#
# The underlay (fixtures/acceptance/v1/renovation-altlik.dxf) is a third party's: GDAL 3.13's DXF writer (ogr2ogr
# from scripts/fixtures/acceptance_corpus.py --files), AutoCAD 2004 format, its header saying inches ($INSUNITS 1)
# though the survey is in metres, its lines two-point LWPOLYLINEs on DUVAR and a door leaf on KAPI. The plan: a
# 9 × 7 m clear room of 25 cm walls (their inner and outer faces drawn) split by a 15 cm wall between x 4.00 and
# 4.15. Taken in metres (fileUnit), the underlay layer locked, its ten wall lines made walls by pairs (Çiftler,
# Kaynağı koru) as existing walls: each wall between its pair, along the first line, over their overlap. The
# existing rooms: Salon [0, 4] × [0, 7] 28.00 m², Oda [4.15, 9] × [0, 7] 33.95 m². The renovation: the middle wall
# demolished, a new 15 cm partition 3.00 clear from the west face (axis 3.075) with a door; the new rooms: Yatak
# [0, 3] × [0, 7] 21.00 m², Salon [3.15, 9] × [0, 7] 40.95 m²: the demolished wall bounds no new room, the new
# partition no existing one (docs/adr/0798 §3). Metraj by phase. The design change: the partition 3.00 → 3.30
# clear: Yatak 23.10, Salon 38.85, the existing rooms as they were. Refusals: a relative path (invalid_file_path),
# lines on the locked underlay that would be removed (layer_locked).

R_DXF = "fixtures/acceptance/v1/renovation-altlik.dxf"
R_INNER = [(F(0), F(0)), (F(9), F(0)), (F(9), F(7)), (F(0), F(7))]
R_OUTER = [(F(-1, 4), F(-1, 4)), (F(37, 4), F(-1, 4)), (F(37, 4), F(29, 4)), (F(-1, 4), F(29, 4))]
R_MID = [((F(4), F(7)), (F(4), F(0))), ((F("4.15"), F(7)), (F("4.15"), F(0)))]
R_DOOR = ((F("4.15"), F(3)), (F("5.05"), F(3)))


def underlay_lines() -> list[tuple[str, tuple, tuple]]:
    """The underlay's lines in its order: the inner faces counter-clockwise, the outer ones, the middle wall's two
    faces, the door's leaf."""
    out = [("DUVAR", R_INNER[i], R_INNER[(i + 1) % 4]) for i in range(4)]
    out += [("DUVAR", R_OUTER[i], R_OUTER[(i + 1) % 4]) for i in range(4)]
    out += [("DUVAR", a, b) for a, b in R_MID] + [("KAPI", *R_DOOR)]
    return out


def renovation() -> dict:
    ex, nw = {"level": "zemin", "phase": "existing"}, {"level": "zemin", "phase": "new"}
    lines = underlay_lines()
    pair = lambda k: [[(a + c) / 2 for a, c in zip(p, q)] for p, q in zip(lines[k][1:], lines[k + 4][1:])]
    # Each wall along its first (inner) line over the pair's overlap, half-way between the two lines.
    walls = {}
    for k, name in enumerate(["south", "east", "north", "west"]):
        (a, b), (c, d) = lines[k][1:], lines[k + 4][1:]
        if a[1] == b[1]:  # horizontal
            y = (a[1] + c[1]) / 2
            walls[name] = {"pts": [[num(a[0]), num(y)], [num(b[0]), num(y)]], "width": num(abs(a[1] - c[1]))}
        else:
            x = (a[0] + c[0]) / 2
            walls[name] = {"pts": [[num(x), num(a[1])], [num(x), num(b[1])]], "width": num(abs(a[0] - c[0]))}
    (m1a, m1b), (m2a, _) = R_MID
    walls["middle"] = {"pts": [[num((m1a[0] + m2a[0]) / 2), 7.0], [num((m1a[0] + m2a[0]) / 2), 0.0]],
                       "width": float(m2a[0]) - float(m1a[0])}  # the DXF's two numbers' difference, as doubles
    old = {"salon0": F(4) * 7, "oda0": (9 - F("4.15")) * 7}
    part = lambda clear: F(0) + clear + F(3, 40)

    def new(clear: F) -> dict[str, F]:
        return {"yatak": clear * 7, "salon": (9 - (part(clear) + F(3, 40))) * 7}

    new0, new1 = new(F(3)), new(F(33, 10))
    door_at = 7 - (R_DOOR[0][1] + F(9, 20))  # the leaf's hinge at y 3.00, 0.90 wide, along the wall from y 7
    count = 0

    def n(k: int) -> int:
        nonlocal count
        count += k
        return count

    m = lambda a: {k: {"area": num(v)} for k, v in a.items()}
    lnames = [f"l{i + 1}" for i in range(10)]
    wnames = ["south", "east", "north", "west", "middle"]
    steps = [
        {"name": "Proje ayarları › Mimari: zemin kat, 2.80 m", "command": "cad.settings.set",
         "input": {"arch": {"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0, "height": 2.8}]}},
         "label": None, "expect": {"count": 0}},
        {"name": "Ret: altlığın yolu göreli", "command": "file.import.dxf", "input": {"path": R_DXF, "fileUnit": "m"},
         "refuse": "invalid_file_path", "expect": {"count": 0}},
        {"name": "Altlık: GDAL'ın DXF'i metre olarak (başlığı inç diyor)", "command": "file.import.dxf",
         "input": {"path": "$underlay", "fileUnit": "m"}, "label": "DXF: renovation-altlik.dxf",
         "expect": {"count": n(11), "output": {"objects": 11, "layers": [{"id": "import-duvar", "name": "DUVAR"},
                                                                          {"id": "import-kapi", "name": "KAPI"}],
                                                "unit": {"declared": 1, "factor": 1.0, "source": "chosen"}}}},
        {"name": "Altlığın katmanı kilitlenir", "command": "cad.layers.set",
         "input": {"operation": "locked", "layer": "import-duvar", "value": True}, "label": None, "expect": {"count": count}},
        {"name": "Altlığın duvar çizgileri seçilir", "query": "cad.entities.select",
         "input": {"region": {"kind": "all"}, "kinds": ["polyline"], "layers": ["import-duvar"]}, "as": lnames,
         "expect": {"output": {"uids": ["$" + x for x in lnames]}}},
        {"name": "Ret: kilitli altlığın çizgileri silinerek duvar olmaz", "command": "cad.walls.convert",
         "input": {"uids": ["$" + x for x in lnames], "mode": "pairs", "maxWidth": 0.3, "layerId": "$layer", "arch": ex},
         "refuse": "layer_locked", "expect": {"count": count}},
        {"name": "Var olan duvarlar: Çiftler, Kaynağı koru, kilitli altlıktan", "command": "cad.walls.convert",
         "input": {"uids": ["$" + x for x in lnames], "mode": "pairs", "maxWidth": 0.3, "keepSources": True,
                   "layerId": "$layer", "arch": ex}, "as": wnames, "label": "Çizgiden duvar",
         "expect": {"count": n(5), "objects": {k: v | {"arch": ex} for k, v in walls.items()}}},
        {"name": "Var olan açıklıklar: orta duvarda kapı, güneyde iki pencere", "command": "cad.openings.create",
         "input": {"layerId": "$layer", "openings": [
             {"host": "$middle", "at": num(door_at), "form": "door", "width": 0.9},
             {"host": "$south", "at": 2.0, "form": "window", "width": 1.2, "sill": 0.9},
             {"host": "$south", "at": 6.5, "form": "window", "width": 1.2, "sill": 0.9}]},
         "as": ["oldDoor", "w1", "w2"], "label": "Kapı", "expect": {"count": n(3), "objects": {"oldDoor": {"at": num(door_at)}}}},
        {"name": "Var olan mahaller: Salon ve Oda", "command": "cad.spaces.create",
         "input": {"layerId": "$layer", "spaces": [
             {"seed": pt(F(2), F(3)), "number": "M01", "name": "Salon", "arch": ex},
             {"seed": pt(F(6), F(3)), "number": "M02", "name": "Oda", "arch": ex}]},
         "as": list(old), "label": "Mahal", "expect": {"count": n(2), "measures": m(old)}},
        {"name": "Yıkım: orta duvar", "command": "cad.walls.edit",
         "input": {"uids": ["$middle"], "operation": {"kind": "set", "phase": "demolish"}}, "label": "Duvarı düzenle",
         "expect": {"count": count, "objects": {"middle": {"arch": {"phase": "demolish"}}}, "measures": m(old)}},
        {"name": "Yeni bölme: batı yüzden 3.00 net", "command": "cad.walls.partition",
         "input": partition("$west", 0, "left", F(3), F(3, 20), phase="new"), "as": "partition", "label": "Duvar",
         "expect": {"count": n(1), "objects": {"partition": {"pts": [[num(part(F(3))), 7.0], [num(part(F(3))), 0.0]], "arch": nw}},
                    "measures": m(old)}},
        {"name": "Yeni kapı: bölmede", "command": "cad.openings.create",
         "input": {"layerId": "$layer", "openings": [{"host": "$partition", "at": 3.5, "form": "door", "width": 0.9}]},
         "as": ["newDoor"], "label": "Kapı", "expect": {"count": n(1)}},
        {"name": "Yeni mahaller: yıkılan duvar sınır değil", "command": "cad.spaces.create",
         "input": {"layerId": "$layer", "spaces": [
             {"seed": pt(F(3, 2), F(3)), "number": "Y01", "name": "Yatak odası", "arch": nw},
             {"seed": pt(F(6), F(3)), "number": "Y02", "name": "Salon", "arch": nw}]},
         "as": list(new0), "label": "Mahal", "expect": {"count": n(2), "measures": m(new0) | m(old)}},
        {"name": "Metraj: var olan durum", "query": "cad.spaces.quantities", "input": {"phases": ["existing"]},
         "expect": {"output": {"totals": {"spaceArea": num(sum(old.values())), "wallLength": 32.0}}}},
        {"name": "Metraj: yıkılacaklar", "query": "cad.spaces.quantities", "input": {"phases": ["demolish"]},
         "expect": {"output": {"totals": {"spaceArea": 0.0, "wallLength": 7.0}}}},
        {"name": "Metraj: yeni durum", "query": "cad.spaces.quantities", "input": {"phases": ["new"]},
         "expect": {"output": {"totals": {"spaceArea": num(sum(new0.values())), "wallLength": 7.0}}}},
        {"name": "Evre görünümü: Yeni durum", "command": "cad.settings.set", "input": {"arch": {"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0, "height": 2.8}], "phaseView": "proposed"}},
         "label": None, "expect": {"count": count}},
        {"name": "Tasarım değişikliği: yeni bölme 3.00 → 3.30; var olan mahaller yerinde", "command": "cad.walls.edit",
         "input": {"uids": ["$partition"], "operation": {"kind": "distance", "side": "right", "distance": 3.3}},
         "label": "Duvarı düzenle",
         "expect": {"count": count, "measures": m(new1) | m(old),
                    "objects": {"partition": {"pts": [[num(part(F(33, 10))), 7.0], [num(part(F(33, 10))), 0.0]]}}}},
        {"name": "Kaydet ve aç: aynı kimlikler, aynı nesneler", "reopen": True,
         "expect": {"count": count, "kinds": {"polyline": 11, "wall": 6, "opening": 4, "space": 4},
                    "measures": m(new1) | m(old)}},
        {"name": "Metraj: değişiklikten sonra yeni durum", "query": "cad.spaces.quantities", "input": {"phases": ["new"]},
         "expect": {"output": {"totals": {"spaceArea": num(sum(new1.values()))}}}},
    ]
    return {"format": "kentos.acceptance", "version": 1, "id": "renovation",
            "title": "Tadilat: GDAL'ın yazdığı DXF altlığından var olan duvarlar, yıkım, yeni bölme, evrelere göre mahal ve metraj",
            "source": "plan/architecture-advanced.md ADV-07 (kabul korpusu); docs/adr/0798", "project": project("Tadilat"),
            "files": {"underlay": R_DXF}, "tolerance": 1e-9, "undone": 8, "steps": steps,
            "rooms": {"after": {k: num(v) for k, v in (new1 | old).items()}}}


def write_underlay() -> None:
    """The underlay written by GDAL (ogr2ogr), not by KentOS: the lines as GeoJSON, layer by its ``Layer`` field."""
    import subprocess
    import tempfile
    feats = [{"type": "Feature", "properties": {"Layer": layer},
              "geometry": {"type": "LineString", "coordinates": [[float(a[0]), float(a[1])], [float(b[0]), float(b[1])]]}}
             for layer, a, b in underlay_lines()]
    with tempfile.TemporaryDirectory() as tmp:
        src = Path(tmp) / "altlik.geojson"
        src.write_text(json.dumps({"type": "FeatureCollection", "features": feats}))
        out = ROOT / R_DXF
        out.unlink(missing_ok=True)
        subprocess.run(["ogr2ogr", "-f", "DXF", str(out), str(src)], check=True)
    print(f"{R_DXF} yazıldı (GDAL).")


SCENARIOS = {"change": change, "house": house, "curved": curved, "renovation": renovation}


# ── The mouse-and-keyboard traces (fixtures/interaction/v1/acceptance-<id>.json) ───────────────────────────────
#
# Each sample as a user draws it on the desktop, as acceptance_flat.py's trace (docs/adr/0776 §1): tools from the
# ribbon, values on the command line, walls picked on their bodies, a temporary number clicked where it stands
# and, new here, its padlock (``clickLock``, docs/adr/0770 §3). ``note``s starting “§” open the sections
# scripts/acceptance/trace_metrics.py counts by. Points are the world's, east and north of the view's centre.

import acceptance_flat as flat  # noqa: E402


def drawing(name: str, levels: list[dict], layers: list[dict] | None = None) -> dict:
    """A trace's starting drawing: an empty metre CAD project at 1:100 with its levels (a v1 file, as the scenes)."""
    return {
        "format": "kentos.document", "version": 1, "name": name,
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 100, "workspace": "cad", "drawingUnit": "m", "drawingFont": "barlow",
                     "arch": {"levels": levels}},
        "origin": {"x": 0, "y": 0},
        "layers": [{"id": "cizim", "name": "Çizim", "type": "layer", "visible": True, "locked": False, "expanded": True,
                    "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}] + (layers or []),
        "activeLayer": "cizim", "entities": [], "styles": {"items": [], "categories": []},
    }


def trace_doc(sid: str, title: str, centre: tuple[float, float], steps: list[dict], mpp: float = 0.03) -> dict:
    return {"format": "kentos.interaction-trace", "version": 1, "id": f"acceptance-{sid}", "title": title,
            "source": f"plan/architecture-advanced.md ADV-07; docs/adr/0798 (fixtures/acceptance/v1/{sid}.json'un fareyle ve klavyeyle yolu)",
            "covers": ["ADV-07"], "document": f"acceptance-{sid}.kcad",
            "view": {"center": list(centre), "metresPerPixel": mpp},
            "draft": {"snap": False, "grid": False, "ortho": False, "polar": False, "tracking": False},
            "prefs": {"cursorInput": True}, "clickTolerance": 0.01, "steps": steps}


def change_trace() -> list[dict]:
    """The 2+1's plan drawn as the apartment's trace draws it (acceptance_flat.py, up to its rooms), then changed by
    its temporary numbers: Yatak 1 bound and widened, Salon widened (the bond carries Bölme 3), a door and a window
    placed by their distances."""
    at = flat.at
    p1, p2, p3 = float(flat.P1), float(flat.P2), float(flat.P3)
    head = []
    for st in flat.trace_steps():
        if st.get("note", "").startswith("§ Donatı"):
            break
        st = dict(st)
        if st.get("note", "").startswith("§"):
            st["note"] = "§ Var olan plan — " + st["note"].lstrip("§ ")
        head.append(st)
    p3b, p1b = p3 + 0.3, 4.875
    p3c = p1b + 0.075 + 3.8 + 0.075
    wall = lambda i, a, b: {"id": i, "kind": "wall", "points": [at(*a), at(*b)]}
    s = head
    s += [{"note": "§ Değişiklik 1: Yatak 1 3.50 → 3.80, kilitle (Bu ölçüyü koru)", "click": at(p3, 2), "expect": {"selected": [4]}},
          {"move": at(p3 - 0.3, 2)}, {"clickLock": "west"}, {"clickNumber": "west"}, {"text": "3.8"},
          {"key": "Enter", "expect": {"entities": 20, "objects": [wall(4, (p3b, p2), (p3b, -0.125))]}},
          {"shot": "kilit"}, {"key": "Esc"}]
    s += [{"note": "§ Değişiklik 2: Salon 4.50 → 4.80; bağ Bölme 3'ü götürür", "click": at(p1, 2), "expect": {"selected": [2]}},
          {"move": at(p1 - 0.3, 2)}, {"clickNumber": "west"}, {"text": "4.8"},
          {"key": "Enter", "expect": {"entities": 20, "objects": [wall(2, (p1b, 8), (p1b, 0)), wall(4, (p3c, p2), (p3c, -0.125))]}},
          {"shot": "bag"}, {"key": "Esc"}]
    s += [{"note": "§ Değişiklik 3: Yatak 1 kapısı Bölme 1'in yüzünden 0.10", "click": at(6.4, p2), "expect": {"selected": [7]}},
          {"move": at(6.4, p2 - 0.2)}, {"clickNumber": "west"}, {"text": "0.1"},
          {"key": "Enter", "expect": {"entities": 20}}, {"key": "Esc"}]
    s += [{"note": "§ Değişiklik 4: Yatak 2 penceresi doğu yüzden 0.60", "click": at(9.7, -0.1), "expect": {"selected": [13]}},
          {"move": at(9.7, 0.2)}, {"clickNumber": "east"}, {"text": "0.6"},
          {"key": "Enter", "expect": {"entities": 20}}, {"key": "Esc"}, {"shot": "degisiklikler"}]
    s += [{"note": "§ Kaydet ve aç", "saveAndReopen": True, "expect": {"entities": 20}}]
    return s


def house_trace() -> list[dict]:
    """The house's two levels: the ground floor drawn, Üst kat (Ctrl+↑), the upper floor drawn over it."""
    centre = (5.0, 4.0)
    at = lambda x, y: [round(x - centre[0], 6), round(y - centre[1], 6)]
    opt = flat.option
    a, b = 5.075, 6.725
    s: list[dict] = []
    s += [{"note": "§ Zemin kat: dış kontur", "run": "tool.wallRectangle"}] + opt("E", "0.25")
    s += [{"focus": "commandLine"}, {"text": "0,0"}, {"key": "Enter"}, {"text": "10,8"},
          {"key": "Enter", "expect": {"entities": 1}}, {"key": "Esc"}, {"key": "Esc", "expect": {"tool": "select"}}]
    s += [{"note": "§ Zemin kat: bölme", "run": "tool.wallPartition"}] + opt("E", "0.15") + opt("U", "5")
    s += [{"click": at(-0.1, 2)}, {"move": at(2, 2)}, {"click": at(2, 2), "expect": {"entities": 2}},
          {"key": "Esc"}, {"key": "Esc", "expect": {"tool": "select", "objects": [
              {"id": 2, "kind": "wall", "points": [at(a, 8), at(a, 0)]}]}}]
    s += [{"note": "§ Zemin kat: merdiven kat yüksekliğine", "run": "tool.stair"}] + opt("G", "1") + opt("D", "0.28")
    s += [{"click": at(0.2, 7.4)}, {"move": at(5.0, 7.4)}, {"click": at(5.0, 7.4)}, {"key": "Enter", "expect": {"entities": 3}},
          {"key": "Esc"}, {"key": "Esc", "expect": {"tool": "select"}}, {"shot": "zemin"}]
    s += [{"note": "§ Zemin kat: mahaller", "run": "tool.space"}]
    for x, y, name in [(2.5, 3.0, "Salon"), (7.5, 4.0, "Mutfak")]:
        s += opt("A", name) + [{"click": at(x, y)}]
    s += [{"key": "Esc"}, {"key": "Esc", "expect": {"tool": "select", "entities": 5}}]
    s += [{"note": "§ 1. kat: Üst kat (Ctrl+↑), Yalnız etkin kat, dış kontur", "key": "Ctrl+ArrowUp"},
          {"run": "view.levels.active"}, {"run": "tool.wallRectangle"}] + opt("E", "0.25")
    s += [{"focus": "commandLine"}, {"text": "0,0"}, {"key": "Enter"}, {"text": "10,8"},
          {"key": "Enter", "expect": {"entities": 6}}, {"key": "Esc"}, {"key": "Esc", "expect": {"tool": "select", "objects": [
              {"id": 6, "kind": "wall", "points": [at(0, 0), at(10, 0), at(10, 8), at(0, 8)]}]}}]
    # Duvar genişliği araçlar arasında ortaktır: dikdörtgen 25, bölme 15 yeniden yazılır (docs/adr/0798 §1).
    s += [{"note": "§ 1. kat: bölmeler", "run": "tool.wallPartition"}] + opt("E", "0.15") + opt("U", "5")
    s += [{"click": at(-0.1, 2)}, {"move": at(2, 2)}, {"click": at(2, 2), "expect": {"entities": 7}}]
    s += opt("U", "1.2") + opt("B", "between")
    s += [{"click": at(2.5, 8.1)}, {"click": at(a, 7.5)}, {"click": at(-0.1, 7.5)}, {"move": at(2.5, 7.0)},
          {"click": at(2.5, 7.0), "expect": {"entities": 8}}, {"key": "Esc"},
          {"key": "Esc", "expect": {"tool": "select", "objects": [
              {"id": 7, "kind": "wall", "points": [at(a, 8), at(a, 0)]},
              {"id": 8, "kind": "wall", "points": [at(a, b), at(-0.125, b)]}]}}, {"shot": "ust-kat"}]
    s += [{"note": "§ 1. kat: mahaller, merdiven boşluğu (V)", "run": "tool.space"}]
    s += opt("A", "Yatak 1") + [{"click": at(2.5, 3.0)}]
    s += opt("A", "Merdiven boşluğu") + [{"focus": "commandLine"}, {"text": "V"}, {"key": "Enter"}, {"click": at(2.5, 7.4)}]
    s += [{"focus": "commandLine"}, {"text": "V"}, {"key": "Enter"}] + opt("A", "Yatak 2") + [{"click": at(7.5, 4.0)}]
    s += [{"key": "Esc"}, {"key": "Esc", "expect": {"tool": "select", "entities": 11}}, {"shot": "mahaller"}]
    s += [{"note": "§ Kaydet ve aç", "saveAndReopen": True, "expect": {"entities": 11}}]
    return s


def house_levels() -> list[dict]:
    return [{"id": "zemin", "name": "Zemin kat", "elevation": 0, "height": 3.0},
            {"id": "kat1", "name": "1. kat", "elevation": 3.0, "height": 3.0}]


def typed(*values: str) -> list[dict]:
    """Values on the command line, each with Enter."""
    out: list[dict] = [{"focus": "commandLine"}]
    for v in values:
        out += [{"text": v}, {"key": "Enter"}]
    return out


def curved_trace() -> list[dict]:
    """The bay drawn as a closed polyline with a tangent arc, made a centred wall; the partition, the shaft's L
    wall, the rooms (the shaft a void) and the spiral stair."""
    centre = (6.0, 4.0)
    at = lambda x, y: [round(x - centre[0], 6), round(y - centre[1], 6)]
    opt = flat.option
    s: list[dict] = [{"note": "§ Kontur: Çoklu çizgi, doğuda teğet yay", "run": "tool.polyline"}]
    s += typed("0,0", "10,0", "Y", "10,8", "D", "0,8", "K")
    s += [{"key": "Esc", "expect": {"tool": "select", "entities": 1}}, {"shot": "kontur"}]
    s += [{"note": "§ Çizgiden duvar: ortalı 25 cm", "click": at(5, 0), "expect": {"selected": [1]}},
          {"run": "tool.wallFromLines"}] + opt("E", "0.25") + [{"key": "Enter", "expect": {"entities": 1}},
          {"key": "Esc"}, {"key": "Esc", "expect": {"tool": "select", "objects": [
              {"id": 2, "kind": "wall", "points": [at(0, 0), at(10, 0), at(10, 8), at(0, 8)], "bulges": [0.0, 1.0, 0.0, 0.0]}]}}]
    s += [{"note": "§ Bölme: batı yüzden 4.00", "run": "tool.wallPartition"}] + opt("E", "0.15") + opt("U", "4")
    s += [{"click": at(0, 2)}, {"move": at(2, 2)}, {"click": at(2, 2), "expect": {"entities": 2}},
          {"key": "Esc"}, {"key": "Esc", "expect": {"tool": "select", "objects": [
              {"id": 3, "kind": "wall", "points": [at(4.2, 8), at(4.2, 0)]}]}}]
    s += [{"note": "§ Şaft: L duvar", "run": "tool.wall"}] + opt("E", "0.15")
    s += typed("5.35,8", "5.35,6.8", "4.2,6.8") + [{"key": "Enter", "expect": {"entities": 3}}, {"key": "Esc"},
                                                   {"key": "Esc", "expect": {"tool": "select"}}]
    s += [{"note": "§ Mahaller: Yatak, Salon, şaft boşluk (V)", "run": "tool.space"}]
    s += opt("A", "Yatak") + [{"click": at(2, 4)}] + opt("A", "Salon") + [{"click": at(8, 4)}]
    s += typed("V") + opt("A", "Şaft") + [{"click": at(4.775, 7.375)}]
    s += [{"key": "Esc"}, {"key": "Esc", "expect": {"tool": "select", "entities": 6}}, {"shot": "mahaller"}]
    s += [{"note": "§ Döner merdiven: cumbada, merkez, başlangıç, yön", "run": "tool.stairCurved"}] + opt("G", "1") + opt("D", "0.28")
    s += [{"click": at(10, 4)}, {"click": at(11.5, 4)}, {"move": at(11.06, 5.06)}, {"move": at(10, 5.5)},
          {"move": at(8.94, 5.06)}, {"click": at(8.94, 5.06), "expect": {"entities": 7}},
          {"key": "Esc"}, {"key": "Esc", "expect": {"tool": "select"}}, {"shot": "merdiven"}]
    s += [{"note": "§ Kaydet ve aç", "saveAndReopen": True, "expect": {"entities": 7}}]
    return s


def renovation_trace() -> list[dict]:
    """The underlay's wall lines (its layer locked) made walls by pairs, the rooms, a new partition."""
    centre = (4.5, 3.5)
    at = lambda x, y: [round(x - centre[0], 6), round(y - centre[1], 6)]
    opt = flat.option
    s: list[dict] = [{"note": "§ Çizgiden duvar: Çiftler, Kaynağı koru, kilitli altlık",
                      "drag": [at(-1, -1), at(10, 8)], "expect": {"selected": list(range(1, 12))}},
                     {"run": "tool.wallFromLines"}] + typed("M", "K") + opt("E", "0.3")
    s += [{"key": "Enter", "expect": {"entities": 16}}, {"key": "Esc"}, {"key": "Esc", "expect": {"tool": "select", "objects": [
        {"id": 12, "kind": "wall", "points": [at(0, -0.125), at(9, -0.125)]},
        {"id": 16, "kind": "wall", "points": [at(4.075, 7), at(4.075, 0)]}]}}, {"shot": "duvarlar"}]
    s += [{"note": "§ Mahaller: Salon ve Oda", "run": "tool.space"}]
    s += opt("A", "Salon") + [{"click": at(2, 3)}] + opt("A", "Oda") + [{"click": at(6, 3)}]
    s += [{"key": "Esc"}, {"key": "Esc", "expect": {"tool": "select", "entities": 18}}]
    s += [{"note": "§ Yeni bölme: batı yüzden 3.00", "run": "tool.wallPartition"}] + opt("E", "0.15") + opt("U", "3")
    s += [{"click": at(-0.1, 2)}, {"move": at(1.5, 2)}, {"click": at(1.5, 2), "expect": {"entities": 19}},
          {"key": "Esc"}, {"key": "Esc", "expect": {"tool": "select", "objects": [
              {"id": 19, "kind": "wall", "points": [at(3.075, 7), at(3.075, 0)]}]}}, {"shot": "tadilat"}]
    s += [{"note": "§ Kaydet ve aç", "saveAndReopen": True, "expect": {"entities": 19}}]
    return s


def underlay_scene() -> dict:
    """The renovation trace's drawing: the underlay's lines as GDAL wrote them, on their own locked layer."""
    d = drawing("Tadilat", [{"id": "zemin", "name": "Zemin kat", "elevation": 0, "height": 2.8}],
                [{"id": "duvar", "name": "DUVAR", "type": "layer", "visible": True, "locked": True, "expanded": True,
                  "style": {"color": "#868E96", "lineType": "continuous", "lineWeight": 0.18}, "children": []}])
    d["entities"] = [{"kind": "polyline", "id": i + 1, "layerId": "duvar", "attrs": {},
                      "pts": [{"x": float(a[0]), "y": float(a[1])}, {"x": float(b[0]), "y": float(b[1])}]}
                     for i, (_, a, b) in enumerate(underlay_lines())]
    return d


TRACE_DOCS = {
    "change": lambda: (trace_doc("change", "Ölçülü değişiklik fareyle ve klavyeyle: 2+1'in planı, kilitli net ölçü, bağın götürdüğü bölme, kapı ve pencerenin mesafesi",
                                 flat.CENTRE, change_trace()),
                       drawing("Değişiklik", [{"id": "zemin", "name": "Zemin kat", "elevation": 0, "height": 3.0}])),
    "house": lambda: (trace_doc("house", "İki katlı konut fareyle ve klavyeyle: zemin kat, merdiven, Üst kat, merdiven boşluğu",
                                (5.0, 4.0), house_trace()),
                      drawing("Konut", house_levels())),
    "curved": lambda: (trace_doc("curved", "Eğrisel ve şaftlı örnek fareyle ve klavyeyle: teğet yaylı kontur, çizgiden duvar, şaft, döner merdiven",
                                 (6.0, 4.0), curved_trace()),
                       drawing("Cumba", [{"id": "zemin", "name": "Zemin kat", "elevation": 0, "height": 3.0}])),
    "renovation": lambda: (trace_doc("renovation", "Tadilat fareyle ve klavyeyle: kilitli DXF altlığından Çiftler ile duvar, mahaller, yeni bölme",
                                     (4.5, 3.5), renovation_trace()),
                           underlay_scene()),
}


def ring_area(ring: dict) -> float:
    """A ring's area from its points and bulges (a bulge b on a chord c: the arc's segment, θ = 4 atan b,
    r = c / 2 sin(θ/2), area r²(θ − sin θ)/2 on the bulge's side), without KentOS code."""
    pts = [(p["x"], p["y"]) for p in ring["pts"]]
    bulges = ring.get("bulges") or [0.0] * len(pts)
    total = 0.0
    for i, (a, b) in enumerate(zip(pts, pts[1:] + pts[:1])):
        total += (a[0] * b[1] - b[0] * a[1]) / 2
        g = bulges[i] if i < len(bulges) else 0.0
        if g:
            theta = 4 * math.atan(g)
            c = math.dist(a, b)
            r = c / (2 * math.sin(theta / 2))
            total += r * r * (theta - math.sin(theta)) / 2
    return total


def compare(folder: Path) -> list[str]:
    """Each sample's saved drawings (``folder/<id>/*.kcad``, one per host) read without KentOS code
    (tools/kcad/kcad.py) and held to each other and to the scenario: the same objects by kind, walls, openings and
    rooms, the rooms' areas from their own rings."""
    sys.path.insert(0, str(ROOT / "tools/kcad"))
    import kcad  # noqa: E402

    problems: list[str] = []
    for sid, make in SCENARIOS.items():
        scenario = make()
        numbers = {}
        for step in scenario["steps"]:
            if step.get("command") == "cad.spaces.create":
                keys = step["as"] if isinstance(step["as"], list) else [step["as"]]
                numbers |= dict(zip(keys, (sp["number"] for sp in step["input"]["spaces"])))
        want = {numbers[k]: v for k, v in scenario["rooms"]["after"].items()}
        seen: dict[str, dict] = {}
        for path in sorted((folder / sid).glob("*.kcad")):
            _, doc = kcad.read(path.read_bytes())
            ents = doc["entities"]
            kinds: dict[str, int] = {}
            for e in ents:
                kinds[e["kind"]] = kinds.get(e["kind"], 0) + 1
            walls = sorted((tuple((round(p["x"], 9), round(p["y"], 9)) for p in e["pts"]), round(e["width"], 9))
                           for e in ents if e["kind"] == "wall")
            openings = sorted((round(e["at"], 9), e["width"], e["form"]) for e in ents if e["kind"] == "opening")
            rooms = {}
            for e in ents:
                if e["kind"] == "space":
                    a = 0.0
                    for part in e["parts"]:
                        a += abs(ring_area(part["outer"])) - sum(abs(ring_area(h)) for h in part.get("holes", []))
                    rooms[e["number"]] = round(a, 9)
            seen[path.stem] = {"kinds": kinds, "walls": walls, "openings": openings, "rooms": rooms}
            for number, area in want.items():
                if abs(rooms.get(number, -1) - area) > 1e-9:
                    problems.append(f"{sid}/{path.name}: {number} {rooms.get(number)} m², beklenen {area}")
            if len(set(doc["uids"])) != len(ents):
                problems.append(f"{sid}/{path.name}: kalıcı kimlikler benzersiz değil")
        names = list(seen)
        for other in names[1:]:
            for key in ("kinds", "walls", "openings", "rooms"):
                if seen[other][key] != seen[names[0]][key]:
                    problems.append(f"{sid}: {other} ile {names[0]} farklı: {key}")
        if len(names) < 2:
            problems.append(f"{folder / sid}: karşılaştırılacak en az iki çizim yok ({names})")
        print(sid, json.dumps({n: {"kinds": m["kinds"], "rooms": m["rooms"]} for n, m in seen.items()}, ensure_ascii=False))
    return problems


def main(argv: list[str]) -> int:
    if argv[:1] == ["--compare"]:
        problems = compare(Path(argv[1]))
        print("\n".join(problems) or "Aynı model: bütün örneklerde bütün ev sahipleri.")
        return 1 if problems else 0
    if argv[:1] == ["--files"]:
        write_underlay()
        return 0
    outputs = {ACCEPTANCE / f"{k}.json": f() for k, f in SCENARIOS.items()}
    for k, f in TRACE_DOCS.items():
        t, d = f()
        outputs[TRACES / f"acceptance-{k}.json"] = t
        outputs[TRACES / f"acceptance-{k}.kcad"] = d
    texts = {p: json.dumps(v, ensure_ascii=False, indent=1) + "\n" for p, v in outputs.items()}
    if argv[:1] == ["--check"]:
        stale = [p for p, t in texts.items() if not p.exists() or p.read_text(encoding="utf-8") != t]
        for p in stale:
            print(f"{p.relative_to(ROOT)} güncel değil: python3 {Path(__file__).relative_to(ROOT)}")
        if not stale:
            print("Güncel: " + ", ".join(f"{p.stem} {len(json.loads(t)['steps'])} adım" for p, t in texts.items()
                                         if p.suffix == ".json"))
        return 1 if stale else 0
    for p, t in texts.items():
        p.write_text(t, encoding="utf-8")
        print(f"{p.relative_to(ROOT)} yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
