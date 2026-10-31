#!/usr/bin/env python3
"""The 2+1 apartment's acceptance flow (plan/architecture-advanced.md ADV-07, docs/adr/0776): the shell, the
partitions, typed doors and windows, the rooms, a kitchen run, a wall's dimension row, a bound slab, tags, the
schedules, a design change held by a bond, two refusals and the level's sheet, by the catalog's commands; every value
it must give worked out here from the plan's numbers, without KentOS code.

The plan (metres, clear faces): an 11 × 8 m shell of 25 cm walls drawn by its clear faces (İç net); 15 cm
partitions by clear distances from a face; the walls' paths are their reference lines (the shell's its clear face,
a partition's its axis). A measured partition runs along its source edge, in its direction, from one limit's path to
the other's (docs/adr/0657, 0734):

    P1  4.50 from the west face, full height           axis x = 4.575
    P2  4.50 from the south face, P1 → east wall       axis y = 4.575
    P3  3.50 from P1's east face, south wall → P2      axis x = 8.225
    P4  3.00 from the east face, P2 → north wall       axis x = 7.925

The rooms, net (their walls' faces):

    Salon (açık mutfak)  [0, 4.50]   × [0, 8]      36.00 m²
    Yatak 1              [4.65, 8.15] × [0, 4.50]  15.75 m²
    Yatak 2              [8.30, 11]  × [0, 4.50]   12.15 m²
    Koridor              [4.65, 7.85] × [4.65, 8]  10.72 m²
    Banyo                [8.00, 11]  × [4.65, 8]   10.05 m²

The design change: Yatak 1's clear width 3.50 → 3.60 with Bu ölçüyü koru (docs/adr/0770): P3's axis to 8.325,
Yatak 1 16.20 m², Yatak 2 11.70 m²; the bond holds it; the window rows and the slab stay. The slab bound to the
shell's outer faces is 11.5 × 8.5 = 97.75 m². The south wall's dimension row: its clear face's ends and the three
windows' jambs (1.20 m wide at 2.25, 6.40 and 9.70), seven pieces and the overall 11.00 m.

The walls' surface (Metraj): their paths' length × the level's height 3.06 m less the openings' faces: doors
head 2.10 (90 cm ×4, the entrance 1.00), windows sill 0.90/1.50 to head 2.20 (KentOS's window head, the types not
changing it), 1.20 m ×4 and 0.60 m ×1.

Steps:

- ``command``: a catalog command and its input; ``"$name"`` is an earlier step's persistent id; ``as`` binds the
  output's ``uid``, ``set`` or first of ``uids`` (a name) or its ``uids``/``created`` (a list); ``label`` the undo
  step's name: every write step is undone and redone by the runner, the object count and the bound objects back.
- ``refuse``: the command must fail with this code, the drawing unchanged.
- ``reopen``: the drawing written to ``.kcad`` and read back; every bound object the same, by the same id.
- ``expect``: ``count``, ``kinds`` (objects by kind), ``objects`` (bound objects' fields), ``measures`` (area and
  length), ``output`` (fields of the command's answer).

    python3 scripts/fixtures/acceptance_flat.py              # write
    python3 scripts/fixtures/acceptance_flat.py --check      # compare
    python3 scripts/fixtures/acceptance_flat.py --compare D  # the hosts' saved drawings (D/*.kcad) alike
"""

from __future__ import annotations

import json
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/acceptance/v1/flat.json"

W_SHELL, W_PART = F(1, 4), F(3, 20)
IN_X, IN_Y = F(11), F(8)
P1 = F(9, 2) + W_PART / 2
P2 = F(9, 2) + W_PART / 2
P3 = P1 + W_PART / 2 + F(7, 2) + W_PART / 2
P4 = IN_X - 3 - W_PART / 2
LEVEL_HEIGHT = F(306, 100)


def num(v: F) -> float:
    return float(v)


def pt(x: F, y: F) -> dict:
    return {"x": num(x), "y": num(y)}


def rooms(p3: F) -> dict[str, F]:
    """The five rooms' net areas with P3's axis at ``p3``."""
    h = W_PART / 2
    return {
        "salon": (P1 - h) * IN_Y,
        "yatak1": (p3 - h - (P1 + h)) * (P2 - h),
        "yatak2": (IN_X - (p3 + h)) * (P2 - h),
        "koridor": (P4 - h - (P1 + h)) * (IN_Y - (P2 + h)),
        "banyo": (IN_X - (P4 + h)) * (IN_Y - (P2 + h)),
    }


SEEDS = {
    "salon": (F(2), F(2), "01", "Salon", "Salon"),
    "yatak1": (F(6), F(2), "02", "Yatak odası 1", "Yatak odası"),
    "yatak2": (F(19, 2), F(2), "03", "Yatak odası 2", "Yatak odası"),
    "koridor": (F(6), F(6), "04", "Koridor", "Koridor"),
    "banyo": (F(19, 2), F(6), "05", "Banyo", "Islak hacim"),
}
WINDOWS_SOUTH = [F(9, 4), F(32, 5), F(97, 10)]  # centres along the south face, 1.20 m wide
DOORS = [  # (host, along its path, type)
    ("p1", IN_Y - F(63, 10), "İç kapı 90"),  # the corridor's: P1 runs north → south
    ("p2", F(64, 10) - P1, "İç kapı 90"),  # Yatak 1's
    ("p2", F(97, 10) - P1, "İç kapı 90"),  # Yatak 2's
    ("p4", F(63, 10) - P2, "İç kapı 90"),  # Banyo's: P4 runs south → north
    ("shell", IN_X + IN_Y + (IN_X - F(62, 10)), "Giriş kapısı"),  # on the north face, at x 6.20
]
WINDOWS = [("shell", c, "Pencere 120") for c in WINDOWS_SOUTH] + [
    ("shell", IN_X + F(63, 10), "Pencere 60"),  # Banyo's, on the east face at y 6.30
    ("shell", 2 * IN_X + IN_Y + (IN_Y - 4), "Pencere 120"),  # Salon's, on the west face at y 4.00
]


def dimension_row() -> list[F]:
    marks = [F(0)] + [c + d for c in WINDOWS_SOUTH for d in (-F(3, 5), F(3, 5))] + [IN_X]
    return [b - a for a, b in zip(marks, marks[1:])]


def wall_net_area() -> F:
    lengths = 2 * (IN_X + IN_Y) + IN_Y + (IN_X + W_SHELL / 2 - P1) + (P2 + W_SHELL / 2) + (IN_Y + W_SHELL / 2 - P2)
    doors = 4 * F(9, 10) * F(21, 10) + 1 * F(21, 10)
    windows = 4 * F(6, 5) * (F(22, 10) - F(9, 10)) + F(3, 5) * (F(22, 10) - F(3, 2))
    return lengths * LEVEL_HEIGHT - doors - windows


def walls_length() -> F:
    return 2 * (IN_X + IN_Y) + IN_Y + (IN_X + W_SHELL / 2 - P1) + (P2 + W_SHELL / 2) + (IN_Y + W_SHELL / 2 - P2)


def scenario() -> dict:
    lv = {"level": "zemin"}
    part_wall = {"layerId": "$layer", "walls": [{"pts": [], "closed": False, "width": num(W_PART), "arch": lv}]}
    before, after = rooms(P3), rooms(P3 + F(1, 10))
    measures = lambda a: {k: {"area": num(v)} for k, v in a.items()}
    kinds_rooms = {"wall": 5, "opening": 10, "space": 5}

    def part(src: str, edge: int, side: str, d: F, between=None) -> dict:
        i = {"source": {"wall": src, "edge": edge}, "side": side, "measure": "clear", "distance": num(d), "wall": part_wall}
        if between:
            i["extent"] = {"kind": "between", "start": {"wall": between[0][0], "edge": between[0][1]},
                           "end": {"wall": between[1][0], "edge": between[1][1]}}
        return i

    sh = W_SHELL / 2
    walls = {
        "shell": {"kind": "wall", "pts": [[0.0, 0.0], [11.0, 0.0], [11.0, 8.0], [0.0, 8.0]], "closed": True, "width": 0.25},
        "p1": {"kind": "wall", "pts": [[num(P1), 8.0], [num(P1), 0.0]], "width": num(W_PART)},
        "p2": {"kind": "wall", "pts": [[num(P1), num(P2)], [num(IN_X + sh), num(P2)]]},
        "p3": {"kind": "wall", "pts": [[num(P3), num(P2)], [num(P3), num(-sh)]]},
        "p4": {"kind": "wall", "pts": [[num(P4), num(P2)], [num(P4), num(IN_Y + sh)]]},
    }
    openings = [{"host": f"${h}", "at": num(a), "type": t} for h, a, t in DOORS + WINDOWS]
    names_d = ["d1", "d2", "d3", "d4", "entrance"]
    names_w = ["w1", "w2", "w3", "w4", "w5"]
    total = sum(before.values())
    steps = [
        {"name": "Proje ayarları › Mimari: zemin kat, 3.06 m",
         "command": "cad.settings.set",
         "input": {"arch": {"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0, "height": num(LEVEL_HEIGHT)}]}},
         "label": None, "expect": {"count": 0}},
        {"name": "Dış kontur: Dikdörtgen duvar, İç net 11 × 8 m, 25 cm",
         "command": "cad.walls.rectangle",
         "input": {"layerId": "$layer", "frame": {"kind": "corners", "a": pt(F(0), F(0)), "b": pt(IN_X, IN_Y)},
                   "width": num(W_SHELL), "arch": lv},
         "as": "shell", "label": "Duvar", "expect": {"count": 1, "objects": {"shell": walls["shell"]}}},
        {"name": "Bölme 1: batı yüzden 4.50 net", "command": "cad.walls.partition", "input": part("$shell", 3, "left", F(9, 2)),
         "as": "p1", "label": "Duvar", "expect": {"objects": {"p1": walls["p1"]}}},
        {"name": "Bölme 2: güney yüzden 4.50 net, Bölme 1 ile doğu duvarı arasında",
         "command": "cad.walls.partition", "input": part("$shell", 0, "left", F(9, 2), (("$p1", 0), ("$shell", 1))),
         "as": "p2", "label": "Duvar", "expect": {"objects": {"p2": walls["p2"]}}},
        {"name": "Bölme 3: Bölme 1'den 3.50 net, güney duvarı ile Bölme 2 arasında",
         "command": "cad.walls.partition", "input": part("$p1", 0, "left", F(7, 2), (("$shell", 0), ("$p2", 0))),
         "as": "p3", "label": "Duvar", "expect": {"objects": {"p3": walls["p3"]}}},
        {"name": "Bölme 4: doğu yüzden 3.00 net, Bölme 2 ile kuzey duvarı arasında",
         "command": "cad.walls.partition", "input": part("$shell", 1, "left", F(3), (("$p2", 0), ("$shell", 2))),
         "as": "p4", "label": "Duvar", "expect": {"objects": {"p4": walls["p4"]}, "count": 5}},
    ]
    for name, form, change in [("İç kapı 90", "door", {"width": 0.9}), ("Giriş kapısı", "door", {"width": 1.0}),
                               ("Pencere 120", "window", {"width": 1.2, "sill": 0.9}),
                               ("Pencere 60", "window", {"width": 0.6, "sill": 1.5})]:
        steps.append({"name": f"Kapı ve pencere tipleri: {name}", "command": "cad.openingtypes.edit",
                      "input": {"operation": {"kind": "define", "name": name, "form": form, "change": change}},
                      "label": "Açıklık tipi", "expect": {"count": 5}})
    steps += [
        {"name": "Kapılar ve pencereler tipleriyle: beş kapı, beş pencere", "command": "cad.openings.create",
         "input": {"layerId": "$layer", "openings": openings}, "as": names_d + names_w, "label": "Kapı",
         "expect": {"count": 15, "objects": {**{n: {"kind": "opening", "host": f"${h}", "at": num(a)}
                                                for n, (h, a, _) in zip(names_d + names_w, DOORS + WINDOWS)},
                                             "entrance": {"form": "door", "width": 1.0}, "w5": {"form": "window", "width": 1.2}}}},
        {"name": "Ret: duvarından taşan kapı", "command": "cad.openings.create",
         "input": {"layerId": "$layer", "openings": [{"host": "$p1", "at": 50.0, "type": "İç kapı 90"}]},
         "refuse": "opening_off_wall", "expect": {"count": 15}},
        {"name": "Mahaller: beş oda tohumlarıyla, numara ve adlarıyla", "command": "cad.spaces.create",
         "input": {"layerId": "$layer", "spaces": [
             {"seed": pt(x, y), "number": n, "name": nm, "usage": u, "arch": lv} for x, y, n, nm, u in SEEDS.values()]},
         "as": list(SEEDS), "label": "Mahal", "expect": {"count": 20, "measures": measures(before)}},
        {"name": "Donatı: kuzey duvarı boyunca 3 m mutfak dizisi, evye 2., ocak 4. modül", "command": "cad.casework.run",
         "input": {"layerId": "$layer", "from": pt(F(0), IN_Y), "to": pt(F(3), IN_Y), "sink": 2, "cooktop": 4},
         "label": "Mutfak dizisi", "expect": {"count": 28, "kinds": {"insert": 6, "polygon": 1, "polyline": 1},
                                             "output": {"filler": 0.3}}},
        {"name": "Ölçü: güney duvarının açıklıklar ve toplam sırası", "command": "cad.dimensionsets.create",
         "input": {"layerId": "$layer", "wall": "$shell", "edge": 0, "at": pt(F(11, 2), -F(3, 2)), "rows": ["openings", "overall"]},
         "as": ["dim1", "dim2", "dim3", "dim4", "dim5", "dim6", "dim7", "dimAll"], "label": "Otomatik duvar ölçüleri",
         "expect": {"count": 36, "measures": {f"dim{i + 1}": {"length": num(v)} for i, v in enumerate(dimension_row())}
                    | {"dimAll": {"length": 11.0}}}},
        {"name": "Döşeme: dış yüzlere bağlı", "command": "cad.slabs.create",
         "input": {"layerId": "$layer", "slabs": [{"seed": pt(F(2), F(2)), "bound": True, "arch": lv}]},
         "as": "slab", "label": "Döşeme", "expect": {"count": 37, "measures": {"slab": {"area": 97.75}}}},
        {"name": "Etiketler: bütün mahaller", "command": "cad.tags.create",
         "input": {"layerId": "$layer", "all": {"kinds": ["space"]}}, "label": "Etiket", "expect": {"count": 42}},
        {"name": "Çizelge: kapı ve pencere", "command": "cad.schedules.create",
         "input": {"layerId": "$layer", "at": pt(F(14), F(8)), "of": "openings"}, "as": "openingSchedule",
         "label": "Çizelge", "expect": {"count": 43, "output": {"rows": 10}}},
        {"name": "Çizelge: mahal", "command": "cad.schedules.create",
         "input": {"layerId": "$layer", "at": pt(F(14), -F(2)), "of": "spaces"}, "as": "spaceSchedule",
         "label": "Çizelge", "expect": {"count": 44, "output": {"rows": 5}}},
        {"name": "Metraj: toplamlar", "query": "cad.spaces.quantities", "input": {},
         "expect": {"output": {"totals": {"spaceArea": num(total), "slabArea": 97.75, "openings": 10,
                                          "wallLength": num(walls_length()), "wallNetArea": num(wall_net_area())}}}},
        {"name": "Tasarım değişikliği: Yatak 1 net 3.50 → 3.60, Bu ölçüyü koru", "command": "cad.walls.edit",
         "input": {"uids": ["$p3"], "operation": {"kind": "distance", "side": "right", "distance": 3.6, "keep": True}},
         "label": "Duvarı düzenle",
         "expect": {"count": 44, "measures": measures(after) | {"dimAll": {"length": 11.0}, "slab": {"area": 97.75}},
                    "objects": {"p3": {"pts": [[num(P3 + F(1, 10)), num(P2)], [num(P3 + F(1, 10)), num(-sh)]]}},
                    "output": {"effects": {"right": {"before": 3.5, "after": 3.6}, "left": {"before": 2.7, "after": 2.6}}}}},
        {"name": "Kilitli katman: Kaydır reddedilir", "command": "cad.layers.set",
         "input": {"operation": "locked", "layer": "$layer", "value": True}, "label": None, "expect": {"count": 44}},
        {"name": "Ret: kilitli katmanda duvar", "command": "cad.walls.edit",
         "input": {"uids": ["$p3"], "operation": {"kind": "shift", "by": 0.1}}, "refuse": "layer_locked",
         "expect": {"count": 44, "measures": {"yatak1": {"area": num(after["yatak1"])}}}},
        {"name": "Kilit açılır", "command": "cad.layers.set",
         "input": {"operation": "locked", "layer": "$layer", "value": False}, "label": None, "expect": {"count": 44}},
        {"name": "Pafta: zemin katın planı", "command": "cad.sheets.levels", "input": {}, "label": None,
         "expect": {"count": 44}},
        {"name": "Kaydet ve aç: aynı kimlikler, aynı nesneler", "reopen": True,
         "expect": {"count": 44, "kinds": kinds_rooms | {"slab": 1, "tag": 5, "table": 2, "dimension": 8, "insert": 6},
                    "measures": measures(after) | {"slab": {"area": 97.75}},
                    "objects": {"p3": {"pts": [[num(P3 + F(1, 10)), num(P2)], [num(P3 + F(1, 10)), num(-sh)]]}}}},
        {"name": "Metraj: değişiklikten sonra", "query": "cad.spaces.quantities", "input": {},
         "expect": {"output": {"totals": {"spaceArea": num(sum(after.values())), "slabArea": 97.75}}}},
    ]
    return {
        "format": "kentos.acceptance", "version": 1, "id": "flat",
        "title": "2+1 daire: kontur, bölmeler, tipli açıklıklar, mahal, donatı, ölçü, döşeme, değişiklik, çizelge, pafta",
        "source": "plan/architecture-advanced.md ADV-07; docs/adr/0776",
        "project": {"name": "Daire", "workspace": "cad", "coords": "local", "drawingUnit": "m", "plotScale": 100},
        "tolerance": 1e-9,
        "steps": steps,
        "rooms": {"before": {k: num(v) for k, v in before.items()}, "after": {k: num(v) for k, v in after.items()}},
    }


# ── The mouse-and-keyboard trace (fixtures/interaction/v1/acceptance-flat.json) ───────────────────────────────
#
# The same apartment as an experienced user draws it on the desktop: tools from the ribbon (``run``) or the command
# line, values typed on the command line, the walls picked by clicks on their bodies. ``note`` starting “§” opens a
# section the metrics (scripts/acceptance/trace_metrics.py) count by. Points are the world's, written east and north
# of the view's centre as the traces take them.

TRACE = ROOT / "fixtures/interaction/v1/acceptance-flat.json"
SCENE = ROOT / "fixtures/interaction/v1/acceptance-flat.kcad"
CENTRE = (5.5, 4.0)


def at(x: float, y: float) -> list[float]:
    return [round(x - CENTRE[0], 6), round(y - CENTRE[1], 6)]


def option(key: str, value: str) -> list[dict]:
    """A tool's option on the command line: its letter, Enter, its value, Enter."""
    return [{"focus": "commandLine"}, {"text": key}, {"key": "Enter"}, {"text": value}, {"key": "Enter"}]


def trace_steps() -> list[dict]:
    p1, p2, p3, p4 = (float(v) for v in (P1, P2, P3, P4))
    s: list[dict] = []
    s += [{"note": "§ Dış kontur: Dikdörtgen duvar, İç net, 25 cm", "run": "tool.wallRectangle"}]
    s += option("E", "0.25")
    # The corners typed: a click lands on a screen pixel (3 cm here), a typed point on the number.
    s += [{"focus": "commandLine"}, {"text": "0,0"}, {"key": "Enter"}, {"text": "11,8"},
          {"key": "Enter", "expect": {"entities": 1, "objects": [
              {"id": 1, "kind": "wall", "points": [at(0, 0), at(11, 0), at(11, 8), at(0, 8)]}]}},
          {"key": "Esc"}, {"key": "Esc", "expect": {"tool": "select"}}]
    s += [{"shot": "kontur"}]
    # The partitions: Ölçülü bölme, the source's face clicked, the side by the pointer, a right click writes.
    s += [{"note": "§ Bölmeler: dört Ölçülü bölme", "run": "tool.wallPartition"}]
    s += option("E", "0.15") + option("U", "4.5")
    s += [{"click": at(-0.1, 2)}, {"move": at(2, 2)}, {"click": at(2, 2), "expect": {"entities": 2}}]
    s += option("B", "between")  # Uzaklık stays 4.50 from the first.
    s += [{"click": at(2.5, -0.1)}, {"click": at(p1, 3)}, {"click": at(11.1, 3)}, {"move": at(6, 2)},
          {"click": at(6, 2), "expect": {"entities": 3}}]
    s += option("U", "3.5")  # Kalınlık and Uçlar stay as set: the tool keeps them between partitions.
    s += [{"click": at(p1 + 0.05, 2)}, {"click": at(6.5, -0.1)}, {"click": at(6.5, p2)}, {"move": at(6, 2)},
          {"click": at(6, 2), "expect": {"entities": 4}}]
    s += option("U", "3")
    s += [{"click": at(11.1, 6)}, {"click": at(9.5, p2)}, {"click": at(9.5, 8.1)}, {"move": at(9.5, 6)},
          {"click": at(9.5, 6), "expect": {"entities": 5}}]
    s += [{"key": "Esc"}, {"key": "Esc", "expect": {"tool": "select", "objects": [
        {"id": 2, "kind": "wall", "points": [at(p1, 8), at(p1, 0)]},
        {"id": 3, "kind": "wall", "points": [at(p1, p2), at(11.125, p2)]},
        {"id": 4, "kind": "wall", "points": [at(p3, p2), at(p3, -0.125)]},
        {"id": 5, "kind": "wall", "points": [at(p4, p2), at(p4, 8.125)]}]}}, {"shot": "bolmeler"}]
    # Doors and windows: Kapı and Pencere, a click on the wall's body at the opening's middle.
    s += [{"note": "§ Kapılar ve pencereler", "run": "tool.door"}]
    for x, y in [(p1, 6.3), (6.4, p2), (9.7, p2), (p4, 6.3)]:
        s += [{"move": at(x, y)}, {"click": at(x, y)}]
    s += option("E", "1.0")
    s += [{"move": at(6.2, 8.1)}, {"click": at(6.2, 8.1), "expect": {"entities": 10}}, {"key": "Esc"}, {"key": "Esc"}]
    s += [{"run": "tool.window"}]
    for x, y in [(2.25, -0.1), (6.4, -0.1), (9.7, -0.1), (11.1, 6.3), (-0.1, 4)]:
        s += [{"move": at(x, y)}, {"click": at(x, y)}]
    s += [{"key": "Esc"}, {"key": "Esc", "expect": {"tool": "select", "entities": 15}}, {"shot": "aciklik"}]
    # The rooms one by one with Mahal: the name typed, a click inside.
    s += [{"note": "§ Mahaller: Mahal, adı yazılıp odaya tıklanarak", "run": "tool.space"}]
    for x, y, number, name, usage in SEEDS.values():
        s += option("A", name) + [{"click": at(float(x), float(y))}]
    s += [{"key": "Esc"}, {"key": "Esc", "expect": {"tool": "select", "entities": 20}}, {"shot": "mahaller"}]
    s += [{"note": "§ Donatı: Mutfak dizisi kuzey duvarı boyunca", "run": "tool.casework"},
          {"click": at(0.05, 7.95)}, {"click": at(3.0, 7.95)}, {"move": at(1.5, 7.0)}, {"click": at(1.5, 7.0)},
          {"key": "Esc"}, {"key": "Esc", "expect": {"tool": "select", "entities": 28}}, {"shot": "mutfak"}]
    s += [{"note": "§ Ölçü: Duvar ölçüleri güney duvarında", "run": "tool.wallDimensions"},
          {"click": at(5.5, -0.1)}, {"move": at(5.5, -0.9)}, {"click": at(5.5, -0.9)},
          {"key": "Esc"}, {"key": "Esc", "expect": {"tool": "select", "entities": 36}}]
    s += [{"note": "§ Döşeme: kaynağa bağlı", "run": "tool.slab"}, {"focus": "commandLine"}, {"text": "B"}, {"key": "Enter"},
        {"click": at(2, 2)}, {"key": "Esc"}, {"key": "Esc", "expect": {"tool": "select", "entities": 37}}]
    s += [{"note": "§ Etiketler: Hepsini etiketle", "run": "tool.tagAll"}, {"focus": "commandLine"}, {"key": "Enter"},
          {"key": "Esc", "expect": {"tool": "select", "entities": 52}}]
    s += [{"note": "§ Çizelgeler", "run": "tool.scheduleOpenings"}, {"click": at(13, 8)},
          {"run": "tool.scheduleSpaces"}, {"click": at(13, -1.5)},
          {"key": "Esc", "expect": {"tool": "select", "entities": 54}}, {"shot": "cizelgeler"}]
    # The design change: Yatak 1's clear width from the wall's temporary distance (docs/adr/0710).
    s += [{"note": "§ Tasarım değişikliği: Yatak 1 net 3.50 → 3.60 geçici ölçüden", "click": at(p3, 2),
           "expect": {"selected": [4]}},
          {"move": at(p3 - 0.3, 2)}, {"shot": "gecici-olcu"},
          {"clickNumber": "west"}, {"text": "3.6"},
          {"key": "Enter", "expect": {"entities": 54, "objects": [
              {"id": 4, "kind": "wall", "points": [at(p3 + 0.1, p2), at(p3 + 0.1, -0.125)]}]}},
          {"key": "Esc"}, {"shot": "degisiklik"}]
    # Çıktı › Kat planları opens the sheet template's questions, a panel the player does not answer (docs/adr/0776 §5);
    # the semantic flow makes the sheet (cad.sheets.levels).
    s += [{"note": "§ Kaydet ve aç", "saveAndReopen": True, "expect": {"entities": 54}}]
    return s


def trace() -> dict:
    return {
        "format": "kentos.interaction-trace", "version": 1, "id": "acceptance-flat",
        "title": "2+1 daire fareyle ve klavyeyle: kontur, bölmeler, açıklıklar, mahaller, donatı, ölçü, döşeme, değişiklik, çizelge, pafta",
        "source": "plan/architecture-advanced.md ADV-07; docs/adr/0776 (fixtures/acceptance/v1/flat.json'un fareyle ve klavyeyle yolu)",
        "covers": ["ADV-07"],
        "document": SCENE.name,
        "view": {"center": list(CENTRE), "metresPerPixel": 0.03},
        "draft": {"snap": False, "grid": False, "ortho": False, "polar": False, "tracking": False},
        "prefs": {"cursorInput": True},
        "clickTolerance": 0.01,
        "steps": trace_steps(),
    }


def scene() -> dict:
    """The trace's drawing: an empty metre CAD project at 1:100 with its ground level (a v1 file, as the scenes)."""
    return {
        "format": "kentos.document", "version": 1, "name": "Daire",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 100, "workspace": "cad", "drawingUnit": "m", "drawingFont": "barlow",
                     "arch": {"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0, "height": num(LEVEL_HEIGHT)}]}},
        "origin": {"x": 0, "y": 0},
        "layers": [{"id": "cizim", "name": "Çizim", "type": "layer", "visible": True, "locked": False, "expanded": True,
                    "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}],
        "activeLayer": "cizim", "entities": [], "styles": {"items": [], "categories": []},
    }


def compare(folder: Path) -> list[str]:
    """The hosts' saved drawings (``folder/*.kcad``, each written at the scenario's end) read without KentOS code
    (tools/kcad/kcad.py) and held to each other and to the plan: the same objects by kind, the same walls and
    openings, the rooms' areas from their own rings."""
    sys.path.insert(0, str(ROOT / "tools/kcad"))
    import kcad  # noqa: E402

    want = scenario()["rooms"]["after"]
    seen: dict[str, dict] = {}
    problems: list[str] = []
    for path in sorted(folder.glob("*.kcad")):
        _, doc = kcad.read(path.read_bytes())
        ents = doc["entities"]
        kinds: dict[str, int] = {}
        for e in ents:
            kinds[e["kind"]] = kinds.get(e["kind"], 0) + 1
        walls = sorted((tuple((round(p["x"], 9), round(p["y"], 9)) for p in e["pts"]), e["width"]) for e in ents if e["kind"] == "wall")
        openings = sorted((round(e["at"], 9), e["width"], e["form"]) for e in ents if e["kind"] == "opening")
        rooms_ = {}
        for e in ents:
            if e["kind"] == "space":
                a = 0.0
                for part in e["parts"]:
                    rings = [part["outer"]] + part.get("holes", [])
                    for k, r in enumerate(rings):
                        p = r["pts"]
                        s = sum(p[i]["x"] * p[(i + 1) % len(p)]["y"] - p[(i + 1) % len(p)]["x"] * p[i]["y"] for i in range(len(p))) / 2
                        a += abs(s) if k == 0 else -abs(s)
                rooms_[e["number"]] = round(a, 9)
        model = {"kinds": kinds, "walls": walls, "openings": openings, "rooms": rooms_, "uids": len(set(doc["uids"]))}
        seen[path.stem] = model
        for (x, y, number, *_), (key, area) in zip(SEEDS.values(), want.items()):
            if abs(rooms_.get(number, -1) - area) > 1e-9:
                problems.append(f"{path.name}: {number} {rooms_.get(number)} m², beklenen {area}")
        if model["uids"] != len(ents):
            problems.append(f"{path.name}: kalıcı kimlikler benzersiz değil")
    names = list(seen)
    for other in names[1:]:
        for key in ("kinds", "walls", "openings", "rooms"):
            if seen[other][key] != seen[names[0]][key]:
                problems.append(f"{other} ile {names[0]} farklı: {key}")
    if len(names) < 2:
        problems.append(f"{folder}: karşılaştırılacak en az iki çizim yok ({names})")
    print(json.dumps({n: {"kinds": m["kinds"], "rooms": m["rooms"]} for n, m in seen.items()}, ensure_ascii=False, indent=1))
    return problems


def main(argv: list[str]) -> int:
    if argv[:1] == ["--compare"]:
        problems = compare(Path(argv[1]))
        print("\n".join(problems) or "Aynı model: bütün ev sahipleri.")
        return 1 if problems else 0
    outputs = {OUT: scenario(), TRACE: trace(), SCENE: scene()}
    texts = {p: json.dumps(v, ensure_ascii=False, indent=1) + "\n" for p, v in outputs.items()}
    if argv[:1] == ["--check"]:
        stale = [p for p, t in texts.items() if not p.exists() or p.read_text(encoding="utf-8") != t]
        for p in stale:
            print(f"{p.relative_to(ROOT)} güncel değil: python3 {Path(__file__).relative_to(ROOT)}")
        if not stale:
            print(f"Güncel: {len(scenario()['steps'])} adımlık senaryo, {len(trace()['steps'])} adımlık iz ve izin çizimi.")
        return 1 if stale else 0
    for p, t in texts.items():
        p.write_text(t, encoding="utf-8")
        print(f"{p.relative_to(ROOT)} yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
