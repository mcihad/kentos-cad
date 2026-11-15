#!/usr/bin/env python3
"""The architecture drawing of the audit facts' second slice (docs/adr/8057 §39): a small synthetic two-storey building
drawn with KentOS's own commands (``kentos.cad``), saved as fixtures/audit-facts/v1/mimari.kcad. Its facts are measured
by ``kentos_processing::audit_facts`` and kept byte for byte in mimari.json (that test writes them).

- Storeys: Zemin kat (±0.00) and 1. kat (+3.00), 3 m each.
- Program: flats 1 and 2 (Daire), a shop (Dükkân), an office (Büro), flat 1's storage annex (Depo), the common areas;
  the common areas shared by the units' own rooms (net).
- Zemin kat: a corridor along the building with its escape stair's room at the west end and a door to the outside;
  south of it the stair room, the shop and flat 1 (a living room and a bedroom, its door from the corridor); north of
  it flat 1's storage, a lift shaft (a void) and the office.
- 1. kat: the corridor and the stair room again; flat 2's three rooms in a row south of the corridor, its balcony
  outside the façade (an outdoor brief); the office's archive north of it.

Names are generic; no real person, place or address.

``--mid <file>`` writes the mid-size plan the facts' cost is measured on instead (not kept in the repository): eight
storeys of a 60 m corridor, each with its two escape stairs' rooms, two four-room flats and eight offices.

    python3 scripts/fixtures/audit_facts_mimari.py               # with kentos.cad on the path (.run/py)
    python3 scripts/fixtures/audit_facts_mimari.py --mid plan.kcad
"""

from __future__ import annotations

import sys
from pathlib import Path

import kentos.cad as cad

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/audit-facts/v1/mimari.kcad"


def p(x: float, y: float) -> dict:
    return {"x": x, "y": y}


def walls(doc: cad.Document, layer: str, level: str, rows: list[tuple]) -> list[str]:
    made = doc.run("cad.walls.create", {"layerId": layer, "walls": [
        {"pts": [p(*a), p(*b)], "width": w, "arch": {"level": level}} for a, b, w in rows]})
    return [str(u) for u in made["output"]["uids"]]


def rooms(doc: cad.Document, layer: str, level: str, rows: list[dict]) -> None:
    spaces = []
    for r in rows:
        space = {k: v for k, v in r.items() if k != "seed"}
        if "seed" in r:
            space["seed"] = p(*r["seed"])
        space["arch"] = {"level": level}
        spaces.append(space)
    doc.run("cad.spaces.create", {"layerId": layer, "spaces": spaces})


def doors(doc: cad.Document, layer: str, rows: list[tuple]) -> None:
    doc.run("cad.openings.create", {"layerId": layer, "openings": [
        {"host": host, "at": at, "form": "door", "width": width, "head": 2.1} for host, at, width in rows]})


def storey(doc: cad.Document, layer: str, level: str) -> list[str]:
    """The walls every storey has: the 24 × 9 m outline and the corridor's two partitions (y 4.0 and 5.5)."""
    return walls(doc, layer, level, [
        ((0, 0), (24, 0), 0.2), ((24, 0), (24, 9), 0.2), ((24, 9), (0, 9), 0.2), ((0, 9), (0, 0), 0.2),
        ((0, 4), (24, 4), 0.1), ((0, 5.5), (24, 5.5), 0.1),
    ])


def main() -> int:
    doc = cad.Document.new("Denetim olguları — mimari", workspace="cad", cad_work="architecture")
    layer = doc.active_layer
    doc.run("cad.settings.set", {"arch": {"levels": [
        {"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
        {"id": "k1", "name": "1. kat", "elevation": 3.0, "height": 3.0},
    ]}})
    doc.run("cad.program.edit", {"program": {
        "units": [
            {"id": "d1", "number": "1", "nature": "Daire"},
            {"id": "d2", "number": "2", "nature": "Daire"},
            {"id": "dk", "number": "3", "nature": "Dükkân"},
            {"id": "of", "number": "4", "nature": "Büro"},
            {"id": "dp", "number": "1-E", "kind": "annex", "annexOf": "d1", "nature": "Depo"},
            {"id": "ortak", "number": "O", "name": "Ortak alan", "kind": "common"},
        ],
        "briefs": [{"usage": "Balkon", "outdoor": True}],
        "shareRule": "net",
    }})
    # Zemin kat.
    z = storey(doc, layer, "zemin")
    z += walls(doc, layer, "zemin", [
        ((4, 0), (4, 4), 0.1), ((12, 0), (12, 4), 0.1), ((18, 0), (18, 4), 0.1),
        ((6, 5.5), (6, 9), 0.1), ((8, 5.5), (8, 9), 0.1),
    ])
    rooms(doc, layer, "zemin", [
        {"seed": (2, 2), "name": "Merdiven evi", "unit": "ortak"},
        {"seed": (12, 4.75), "name": "Koridor", "unit": "ortak"},
        {"seed": (8, 2), "name": "Dükkân", "number": "Z01", "unit": "dk"},
        {"seed": (15, 2), "name": "Salon", "number": "Z02", "unit": "d1"},
        {"seed": (21, 2), "name": "Yatak odası", "number": "Z03", "unit": "d1"},
        {"seed": (3, 7.25), "name": "Depo", "number": "Z04", "unit": "dp"},
        {"seed": (7, 7.25), "name": "Asansör boşluğu", "unit": "ortak", "void": True},
        {"seed": (16, 7.25), "name": "Ofis", "number": "Z05", "unit": "of"},
    ])
    # The corridor's partitions: the stair room's door, the shop's, flat 1's; the storage's and the office's; flat 1's
    # inner door; the corridor's door to the outside in the west wall (from its north end, 9 − 4.75 m along it).
    doors(doc, layer, [
        (z[4], 2.0, 1.0), (z[4], 8.0, 0.9), (z[4], 15.0, 1.0),
        (z[5], 3.0, 0.9), (z[5], 16.0, 0.9),
        (z[8], 2.0, 0.9),
        (z[3], 4.25, 1.2),
    ])
    doc.run("cad.stairs.create", {"layerId": layer, "stairs": [{
        "pts": [p(0.6, 0.6), p(0.6, 3.4), p(3.4, 3.4)], "width": 0.9, "risers": 18, "riserHeight": 0.1667,
        "tread": 0.27, "railings": "both", "arch": {"level": "zemin"}}]})
    # 1. kat.
    k = storey(doc, layer, "k1")
    k += walls(doc, layer, "k1", [((4, 0), (4, 4), 0.1), ((12, 0), (12, 4), 0.1), ((18, 0), (18, 4), 0.1)])
    rooms(doc, layer, "k1", [
        {"seed": (2, 2), "name": "Merdiven evi", "unit": "ortak"},
        {"seed": (12, 4.75), "name": "Koridor", "unit": "ortak"},
        {"seed": (8, 2), "name": "Salon", "number": "K01", "unit": "d2"},
        {"seed": (15, 2), "name": "Yatak odası 1", "number": "K02", "unit": "d2"},
        {"seed": (21, 2), "name": "Yatak odası 2", "number": "K03", "unit": "d2"},
        {"seed": (12, 7.25), "name": "Arşiv", "number": "K04", "unit": "of"},
        {"parts": [{"outer": {"pts": [p(5, -1.5), p(11, -1.5), p(11, -0.1), p(5, -0.1)]}}], "name": "Balkon",
         "usage": "Balkon", "unit": "d2"},
    ])
    doors(doc, layer, [
        (k[4], 2.0, 1.0), (k[4], 8.0, 1.0), (k[5], 12.0, 0.9),
        (k[7], 2.0, 0.9), (k[8], 2.0, 0.9),
    ])
    saved = doc.save(OUT)
    print(f"{saved.path}: {saved.bytes} bayt")
    return 0


def mid(out: Path) -> int:
    """Eight storeys of 19 rooms: two escape stairs' rooms at the west end of a 60 m corridor, two flats of four rooms
    in a row south of it (their doors from the corridor into the first room, inner doors between), eight offices north
    of it."""
    doc = cad.Document.new("Denetim olguları — orta boy", workspace="cad", cad_work="architecture")
    layer = doc.active_layer
    levels = [{"id": f"k{i}", "name": f"{i}. kat", "elevation": 3.0 * i, "height": 3.0} for i in range(8)]
    doc.run("cad.settings.set", {"arch": {"levels": levels}})
    units = [{"id": "ortak", "number": "O", "name": "Ortak alan", "kind": "common"}]
    for i in range(8):
        units += [{"id": f"d{i}a", "number": f"{i}A", "nature": "Daire"},
                  {"id": f"d{i}b", "number": f"{i}B", "nature": "Daire"},
                  {"id": f"of{i}", "number": f"{i}C", "nature": "Büro"}]
    doc.run("cad.program.edit", {"program": {"units": units, "shareRule": "net"}})
    for i in range(8):
        level = f"k{i}"
        w = walls(doc, layer, level, [
            ((0, 0), (60, 0), 0.2), ((60, 0), (60, 9), 0.2), ((60, 9), (0, 9), 0.2), ((0, 9), (0, 0), 0.2),
            ((0, 4), (60, 4), 0.1), ((0, 5.5), (60, 5.5), 0.1),
        ])
        south = [4 + 6.5 * k for k in range(9)]  # 4 … 56: eight rooms between
        north = [4 + 6.5 * k for k in range(9)]
        w += walls(doc, layer, level, [((x, 0), (x, 4), 0.1) for x in south] + [((x, 5.5), (x, 9), 0.1) for x in north])
        names = ["Salon", "Yatak odası 1", "Yatak odası 2", "Mutfak"]
        spaces = [{"seed": (2, 2), "name": "Merdiven evi", "unit": "ortak"},
                  {"seed": (2, 7.25), "name": "Yangın merdiveni", "unit": "ortak"},
                  {"seed": (30, 4.75), "name": "Koridor", "unit": "ortak"}]
        spaces += [{"seed": (south[k] + 3.25, 2), "name": names[k % 4], "unit": f"d{i}{'ab'[k // 4]}"}
                   for k in range(8)]
        spaces += [{"seed": (north[k] + 3.25, 7.25), "name": f"Ofis {k + 1}", "unit": f"of{i}"} for k in range(8)]
        rooms(doc, layer, level, spaces)
        # Partitions: w[6..15] south (x = 4 … 56), w[15..24] north. Doors: the stairs' rooms, each flat's first room,
        # each office from the corridor; inside the flats between their rooms.
        door_rows = [(w[4], 2.0, 1.0), (w[5], 2.0, 1.0), (w[4], south[0] + 3.25, 1.0), (w[4], south[4] + 3.25, 1.0)]
        door_rows += [(w[5], north[k] + 3.25, 0.9) for k in range(8)]
        door_rows += [(w[6 + k], 2.0, 0.9) for k in (1, 2, 3, 5, 6, 7)]
        doors(doc, layer, door_rows)
        if i < 7:
            doc.run("cad.stairs.create", {"layerId": layer, "stairs": [{
                "pts": [p(0.6, 0.6), p(0.6, 3.4), p(3.4, 3.4)], "width": 0.9, "risers": 18, "riserHeight": 0.1667,
                "tread": 0.27, "railings": "both", "arch": {"level": level}}]})
    saved = doc.save(out)
    print(f"{saved.path}: {saved.bytes} bayt")
    return 0


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--mid":
        sys.exit(mid(Path(sys.argv[2])))
    sys.exit(main())
