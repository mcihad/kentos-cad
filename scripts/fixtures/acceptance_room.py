#!/usr/bin/env python3
"""The first end-to-end acceptance drawing (TODOS.md CADP-12, plan/cad.md §23.1 K01–K04): a room in a millimetre CAD
project drawn, corrected, dimensioned and hatched by the catalog's commands, every value it must give worked out here
from the definitions, without KentOS code: axis-parallel lines and their crossings, the offsets' sides, the shoelace
area, the dimensions' places.

The scenario (fixtures/acceptance/v1/room.json) is replayed step by step by every host: Python (kentos.cad), the
headless session, the MCP server and the desktop's commands; the desktop's mouse-and-keyboard trace draws the same
room (fixtures/interaction/v1/acceptance-room.json). Coordinates are metres in the project's plane, as the commands
take them (docs/adr/0245): a 4000 mm wall is 4.0.

Steps:

- ``command``: a catalog command and its input; ``"$name"`` in the input is the persistent id an earlier step bound;
  ``as`` binds the output's ``uid`` (a name) or its ``created`` ids (a list).
- ``undo``: the drawing's undo; the step's name it must say.
- ``reopen``: the drawing written to ``.kcad`` bytes and read back.
- ``expect``: after the step, ``objects`` (named objects' fields, numbers to 1e-9 m), ``measures`` (area and length
  from the source geometry) and ``count`` (objects in the drawing).

    python3 scripts/fixtures/acceptance_room.py          # write
    python3 scripts/fixtures/acceptance_room.py --check  # compare
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/acceptance/v1/room.json"
# The mouse-and-keyboard trace's drawing: the same empty millimetre CAD project (a v1 file, as the traces' scenes).
SCENE = ROOT / "fixtures/interaction/v1/acceptance-room.kcad"

# The room: 4000 × 3000 mm, its corners counter-clockwise from the origin.
W, H = 4.0, 3.0
ROOM = [(0.0, 0.0), (W, 0.0), (W, H), (0.0, H)]
T = 0.2  # the walls' 200 mm offset


def pt(x: float, y: float) -> dict:
    return {"x": x, "y": y}


def area(pts: list[tuple[float, float]]) -> float:
    """The shoelace area."""
    s = 0.0
    for i, (x0, y0) in enumerate(pts):
        x1, y1 = pts[(i + 1) % len(pts)]
        s += x0 * y1 - x1 * y0
    return abs(s) / 2


def perimeter(pts: list[tuple[float, float]]) -> float:
    return sum(((pts[(i + 1) % len(pts)][0] - x) ** 2 + (pts[(i + 1) % len(pts)][1] - y) ** 2) ** 0.5
               for i, (x, y) in enumerate(pts))


def line(a: tuple[float, float], b: tuple[float, float]) -> dict:
    return {"kind": "line", "a": list(a), "b": list(b)}


def polygon(pts: list[tuple[float, float]]) -> dict:
    return {"kind": "polygon", "pts": [list(p) for p in pts]}


# K11, K12: the sheets: id, name, scale. Each is A3 landscape with a map frame (left, top, width,
# height in micrometres from the page's top left) showing CENTER, and a title block.
SHEETS = [("kat-plani", "Kat planı", 50), ("vaziyet", "Vaziyet", 100)]
MAP_FRAME = (20000, 10000, 390000, 230000)
CENTER = (2.0, 1.0)


def stroke(width: int) -> dict:
    return {"color": "#000000", "width": width, "dash": [], "cap": "butt", "join": "miter"}


def text_style(size: int, weight: int) -> dict:
    return {"font": "barlow", "size": size, "weight": weight, "italic": False, "color": "#000000"}


def cell(label: str, value: str) -> dict:
    return {"width": 1, "label": label, "value": value, "align": "left", "valign": "middle",
            "signature": False, "rows": []}


def sheet(sid: str, name: str, scale: int) -> dict:
    """A sheet as the sheet core writes it (kentos.sheet/1): A3 landscape, the map and the antet."""
    left, top, width, height = MAP_FRAME
    return {
        "id": sid, "name": name,
        "page": {"paper": "a3", "orientation": "landscape", "size": {"width": 420000, "height": 297000},
                 "margins": {"top": 10000, "right": 10000, "bottom": 10000, "left": 20000}},
        "items": [
            {"id": f"{sid}-harita", "name": "Harita",
             "frame": {"left": left, "top": top, "width": width, "height": height},
             "kind": {"type": "map", "view": {"type": "fixed", "center": {"x": CENTER[0], "y": CENTER[1]},
                                              "scale": scale, "rotation": 0}}},
            {"id": f"{sid}-antet", "name": "Antet",
             "frame": {"left": 230000, "top": 245000, "width": 180000, "height": 42000},
             "kind": {"type": "titleBlock",
                      "rows": [{"height": 1, "cells": [cell("PROJE", "[% @proje_adi %]"), cell("PAFTA", "[% @pafta_adi %]")]},
                               {"height": 1, "cells": [cell("ÖLÇEK", "[% @olcek %]"), cell("TARİH", "[% @tarih %]")]}],
                      "outer": stroke(350), "inner": stroke(180),
                      "labelStyle": text_style(1500, 500), "valueStyle": text_style(2700, 500),
                      "cellPadding": 1200}},
        ],
    }


def build() -> dict:
    steps: list[dict] = []

    # K01: the room, the walls' inner faces: 12 m² and 14 m, kept by a save and an open.
    room_measure = {"area": area(ROOM), "length": perimeter(ROOM)}
    assert (room_measure["area"], room_measure["length"]) == (12.0, 14.0)
    steps.append({
        "name": "K01 Oda: 4000 × 3000 mm kapalı alan",
        "command": "cad.polygon.create",
        "input": {"layerId": "$layer", "pts": [pt(*p) for p in ROOM]},
        "as": "room",
        "expect": {"objects": {"room": polygon(ROOM)}, "measures": {"room": room_measure}, "count": 1},
    })
    steps.append({
        "name": "K01 Kaydet ve aç: aynı kaynak değerleri",
        "reopen": True,
        "expect": {"objects": {"room": polygon(ROOM)}, "measures": {"room": room_measure}, "count": 1},
    })

    def vertex(i: int) -> dict:
        return {"uid": "$room", "place": {"kind": "vertex", "part": 0, "ring": 0, "index": i}}

    def refs(a: int, b: int) -> dict:
        return {"a": {"place": {"kind": "vertex", "part": 0, "ring": 0, "index": a}},
                "b": {"place": {"kind": "vertex", "part": 0, "ring": 0, "index": b}}}

    # K03: an associative dimension on the room's bottom edge, its ends the edge's vertices; the corner
    # pulled to 4500 mm: the dimension says 4500; undo takes both back to 4000.
    bottom = {"kind": "dimension", "a": [0.0, 0.0], "b": [W, 0.0], "refs": refs(0, 1)}
    steps.append({
        "name": "K03 İlişkili ölçü alt kenarın köşelerine bağlanır",
        "command": "cad.entities.create",
        "input": {"layerId": "$layer", "objects": [{"geometry": {
            "kind": "dimension", "a": pt(0, 0), "b": pt(0, 0), "offset": -0.6, "height": 0.125,
            "refs": {"a": vertex(0), "b": vertex(1)}}}]},
        "as": ["dim1"],
        "expect": {"objects": {"dim1": bottom}, "measures": {"dim1": {"length": W}}, "count": 2},
    })
    pulled = [ROOM[0], (4.5, 0.0), ROOM[2], ROOM[3]]
    steps.append({
        "name": "K03 Köşe 4500 mm'ye çekilir; ölçü izler",
        "command": "cad.entities.edit",
        "input": {"operation": "grip", "changes": [{"kind": "update", "uid": "$room",
                                                     "geometry": {"kind": "polygon", "pts": [pt(*p) for p in pulled]}}]},
        "expect": {"objects": {"room": polygon(pulled), "dim1": {**bottom, "b": [4.5, 0.0]}},
                   "measures": {"dim1": {"length": 4.5}, "room": {"area": area(pulled), "length": perimeter(pulled)}}},
    })
    steps.append({
        "name": "K03 Geri al: oda ve ölçü birlikte 4000 mm",
        "undo": "Tutamaçla düzenle",
        "expect": {"objects": {"room": polygon(ROOM), "dim1": bottom}, "measures": {"dim1": {"length": W}}},
    })

    # K04: a dimension on the top edge, bound to its vertices 2 and 3; a vertex added on that edge: the
    # binding follows the old vertex 3, now 4, never the new one; the new vertex pulled out 300 mm
    # changes the room, not the dimension; both undone.
    top = {"kind": "dimension", "a": [W, H], "b": [0.0, H], "refs": refs(2, 3)}
    steps.append({
        "name": "K04 Üst kenar köşelerinden ölçülür",
        "command": "cad.entities.create",
        "input": {"layerId": "$layer", "objects": [{"geometry": {
            "kind": "dimension", "a": pt(0, 0), "b": pt(0, 0), "offset": -0.6, "height": 0.125,
            "refs": {"a": vertex(2), "b": vertex(3)}}}]},
        "as": ["dim2"],
        "expect": {"objects": {"dim2": top}, "measures": {"dim2": {"length": W}}, "count": 3},
    })
    split = ROOM[:3] + [(W / 2, H)] + ROOM[3:]
    steps.append({
        "name": "K04 Ölçülen kenara köşe eklenir (kenar bölünür)",
        "command": "cad.entities.edit",
        "input": {"operation": "vertexAdd", "changes": [{"kind": "update", "uid": "$room",
                                                          "geometry": {"kind": "polygon", "pts": [pt(*p) for p in split]}}]},
        "expect": {"objects": {"room": polygon(split), "dim2": {**top, "refs": refs(2, 4)}},
                   "measures": {"room": room_measure, "dim2": {"length": W}}},
    })
    bent = split[:3] + [(W / 2, H + 0.3)] + split[4:]
    steps.append({
        "name": "K04 Yeni köşe 300 mm dışa çekilir; ölçü yerinde kalır",
        "command": "cad.entities.edit",
        "input": {"operation": "grip", "changes": [{"kind": "update", "uid": "$room",
                                                     "geometry": {"kind": "polygon", "pts": [pt(*p) for p in bent]}}]},
        "expect": {"objects": {"room": polygon(bent), "dim2": {**top, "refs": refs(2, 4)}},
                   "measures": {"room": {"area": area(bent), "length": perimeter(bent)}, "dim2": {"length": W}}},
    })
    steps.append({"name": "K04 Geri al: çekilen köşe", "undo": "Tutamaçla düzenle",
                  "expect": {"objects": {"room": polygon(split)}}})
    steps.append({"name": "K04 Geri al: eklenen köşe", "undo": "Köşe ekle",
                  "expect": {"objects": {"room": polygon(ROOM), "dim2": top}, "measures": {"room": room_measure}}})

    # The room hatched, the hatch following it: the top edge moved 500 mm up, the hatch is the new room.
    steps.append({
        "name": "Oda ilişkili taramayla taranır",
        "command": "cad.entities.create",
        "input": {"layerId": "$layer", "operation": "hatch", "objects": [{"geometry": {
            "kind": "hatch", "ring": [pt(*p) for p in ROOM], "pattern": {"type": "lines", "angle": 45, "spacing": 0.1},
            "assoc": {"outer": "$room", "seed": pt(W / 2, H / 2)}}}]},
        "as": ["hatch"],
        # A hatch measures its area; it has no length of its own.
        "expect": {"measures": {"hatch": {"area": room_measure["area"]}}, "count": 4},
    })
    taller = ROOM[:2] + [(W, H + 0.5), (0.0, H + 0.5)]
    steps.append({
        "name": "Odanın üst kenarı 500 mm kayar; tarama izler",
        "command": "cad.entities.edit",
        "input": {"operation": "stretch", "changes": [{"kind": "update", "uid": "$room",
                                                        "geometry": {"kind": "polygon", "pts": [pt(*p) for p in taller]}}]},
        "expect": {"measures": {"room": {"area": area(taller), "length": perimeter(taller)}, "hatch": {"area": area(taller)}}},
    })
    steps.append({
        "name": "Geri al: oda ve tarama 12 m²",
        "undo": "Esnet",
        "expect": {"measures": {"room": room_measure, "hatch": {"area": room_measure["area"]}}},
    })

    # K02: 200 mm walls around the room, their axes 100 mm outside it, drawn crossing nothing; each axis
    # offset 100 mm out (the outer face) and in (the inner face, on the room's edge). Each offset's side
    # is a point the user clicks.
    a = T / 2
    axes_pts = [(-a, -a), (W + a, -a), (W + a, H + a), (-a, H + a)]
    axes = [(axes_pts[i], axes_pts[(i + 1) % 4]) for i in range(4)]
    for i, (p0, p1) in enumerate(axes):
        steps.append({
            "name": f"K02 Duvar ekseni {i + 1}",
            "command": "cad.line.create",
            "input": {"layerId": "$layer", "a": pt(*p0), "b": pt(*p1)},
            "as": f"wall{i + 1}",
            "expect": {"objects": {f"wall{i + 1}": line(p0, p1)}, "count": 5 + i},
        })
    outward = [(W / 2, -0.5), (W + 0.5, H / 2), (W / 2, H + 0.5), (-0.5, H / 2)]
    inward = [(W / 2, 1.0), (W - 1.0, H / 2), (W / 2, H - 1.0), (1.0, H / 2)]
    normals = [(0.0, -1.0), (1.0, 0.0), (0.0, 1.0), (-1.0, 0.0)]
    out_lines, in_lines = [], []
    for sign, sides, store, word in ((1, outward, out_lines, "dışa"), (-1, inward, in_lines, "içe")):
        for i, ((p0, p1), side) in enumerate(zip(axes, sides)):
            n = normals[i]
            q0 = (p0[0] + sign * n[0] * a, p0[1] + sign * n[1] * a)
            q1 = (p1[0] + sign * n[0] * a, p1[1] + sign * n[1] * a)
            store.append((q0, q1))
            key = f"{'out' if sign == 1 else 'in'}{i + 1}"
            steps.append({
                "name": f"K02 Duvar {i + 1} {word} 100 mm ötelenir",
                "command": "cad.entities.offset",
                "input": {"uids": [f"$wall{i + 1}"], "by": {"kind": "distance", "distance": a, "side": pt(*side)}},
                "as": [key],
                "expect": {"objects": {key: line(q0, q1)}},
            })

    # The outer faces' corners joined (Köşe yuvarla with radius 0): each side grows to the crossing of
    # the two axis-parallel lines and keeps its direction. The picks are on the sides near the corner.
    out = [list(s) for s in out_lines]
    lo, hi_x, hi_y = -T, W + T, H + T
    corners = [  # (first, second, crossing, first's pick, second's pick)
        (0, 1, (hi_x, lo), (W - 0.1, lo), (hi_x, 0.1)),
        (1, 2, (hi_x, hi_y), (hi_x, H - 0.1), (W - 0.1, hi_y)),
        (2, 3, (lo, hi_y), (0.1, hi_y), (lo, H - 0.1)),
        (3, 0, (lo, lo), (lo, 0.1), (0.1, lo)),
    ]
    for f, s_, x, p1, p2 in corners:
        for k in (f, s_):
            p0, q = out[k]
            # The end nearer the crossing moves to it; the direction stays.
            if (p0[0] - x[0]) ** 2 + (p0[1] - x[1]) ** 2 < (q[0] - x[0]) ** 2 + (q[1] - x[1]) ** 2:
                out[k] = [x, q]
            else:
                out[k] = [p0, x]
        steps.append({
            "name": f"K02 Dış köşe {f + 1}–{s_ + 1} birleşir (yarıçap 0)",
            "command": "cad.entities.fillet",
            "input": {"corner": {"kind": "lines", "first": {"uid": f"$out{f + 1}", "at": pt(*p1)},
                                 "second": {"uid": f"$out{s_ + 1}", "at": pt(*p2)}}, "radius": 0},
            "expect": {"objects": {f"out{f + 1}": line(*out[f]), f"out{s_ + 1}": line(*out[s_])}},
        })
    assert out == [[(lo, lo), (hi_x, lo)], [(hi_x, lo), (hi_x, hi_y)], [(hi_x, hi_y), (lo, hi_y)], [(lo, hi_y), (lo, lo)]]

    # The inner faces overhang the room's corners by 100 mm: Buda, bounded by everything shown (the
    # room's edges and the other faces), takes the overhangs; a pick is 50 mm from each end. The inner
    # faces end on the room's corners.
    picks, trimmed = [], []
    for i, (p0, p1) in enumerate(in_lines):
        d = (p1[0] - p0[0], p1[1] - p0[1])
        n = (d[0] ** 2 + d[1] ** 2) ** 0.5
        u = (d[0] / n, d[1] / n)
        picks.append({"uid": f"$in{i + 1}", "at": pt(p0[0] + u[0] * 0.05, p0[1] + u[1] * 0.05)})
        picks.append({"uid": f"$in{i + 1}", "at": pt(p1[0] - u[0] * 0.05, p1[1] - u[1] * 0.05)})
        cut = ((p0[0] + u[0] * a, p0[1] + u[1] * a), (p1[0] - u[0] * a, p1[1] - u[1] * a))
        # The exact crossings are the room's corners; the sums above round to within an ulp of them.
        exact = (ROOM[i], ROOM[(i + 1) % 4])
        assert all(abs(c[k] - e[k]) < 1e-12 for c, e in zip(cut, exact) for k in (0, 1)), (cut, exact)
        trimmed.append(exact)
    steps.append({
        "name": "K02 İç yüzlerin taşan uçları budanır; iç yüzler odanın kenarlarıdır",
        "command": "cad.entities.trim",
        "input": {"picks": picks, "boundaries": {"kind": "all"}},
        "expect": {"objects": {f"in{i + 1}": line(*trimmed[i]) for i in range(4)}, "count": 16},
    })

    # Saved and opened again: everything as it was.
    final = {
        "room": polygon(ROOM), "dim1": bottom, "dim2": top,
        **{f"wall{i + 1}": line(*axes[i]) for i in range(4)},
        **{f"out{i + 1}": line(*out[i]) for i in range(4)},
        **{f"in{i + 1}": line(*trimmed[i]) for i in range(4)},
    }
    steps.append({
        "name": "Kaydet ve aç: bütün çizim",
        "reopen": True,
        "expect": {"objects": final, "measures": {"room": room_measure, "hatch": {"area": room_measure["area"]},
                                                  "dim1": {"length": W}, "dim2": {"length": W}}, "count": 16},
    })

    # K11: a 5000 mm source line under the room, for the sheet's scale.
    k11 = ((-0.5, -1.5), (4.5, -1.5))
    steps.append({
        "name": "K11 5000 mm kaynak çizgi",
        "command": "cad.line.create",
        "input": {"layerId": "$layer", "a": pt(*k11[0]), "b": pt(*k11[1])},
        "as": "k11",
        "expect": {"objects": {"k11": line(*k11)}, "measures": {"k11": {"length": 5.0}}, "count": 17},
    })

    # K11, K12: two A3 sheets (micrometres on the paper), the drawing at 1:50 and at 1:100 in their
    # map frames, a title block (antet) each; one change of the sheet book.
    sheets = [sheet(*s) for s in SHEETS]
    listed = [{"id": sid, "name": name, "width": 420.0, "height": 297.0, "items": 2,
               "maps": [{"item": f"{sid}-harita", "name": "Harita", "scale": scale}]}
              for sid, name, scale in SHEETS]
    steps.append({
        "name": "K11 İki A3 pafta: 1:50 kat planı ve 1:100 vaziyet, antetleriyle",
        "command": "cad.sheets.edit",
        "input": {"ops": [{"op": "addSheet", "sheet": s} for s in sheets]},
        "expect": {"sheets": listed, "count": 17},
    })

    # The DXF: the drawing in its unit (mm), every object with its layer, as an independent reader
    # reads it (group codes; no KentOS code).
    k = 1000.0
    def mm(p: tuple[float, float]) -> list[float]:
        return [p[0] * k, p[1] * k]
    dxf = {
        "insunits": 4,
        "counts": {"LWPOLYLINE": 1, "DIMENSION": 2, "HATCH": 1, "LINE": 13},
        "polylines": [{"closed": True, "points": [mm(q) for q in ROOM]}],
        "lines": [[mm(a), mm(b)] for a, b in [*axes, *[tuple(x) for x in out], *trimmed, k11]],
        "dimensions": [W * k, W * k],
        "hatches": [{"angle": 45.0, "spacing": T / 2 * k, "boundary": [mm(q) for q in ROOM]}],
        "tolerance": 1e-6,
    }
    steps.append({"name": "DXF: bütün çizim milimetrede", "export": "dxf", "expect": {"dxf": dxf}})

    # The PDF: a page each, A3 landscape; the 5000 mm line drawn where its map puts it, 100 mm long
    # at 1:50 and 50 mm at 1:100 (page millimetres from the bottom left, as PDF measures).
    pages = []
    for sid, name, scale in SHEETS:
        left, top, width, height = MAP_FRAME
        cx, cy = (left + width / 2) / 1000.0, 297.0 - (top + height / 2) / 1000.0
        f = 1000.0 / scale
        seg = [[cx + (q[0] - CENTER[0]) * f, cy + (q[1] - CENTER[1]) * f] for q in k11]
        # The title block's words as poppler's pdftotext reads them: the project, the sheet, the scale.
        pages.append({"width": 420.0, "height": 297.0, "segment": seg, "texts": ["Oda", name, f"1/{scale}"]})
    steps.append({"name": "K11 PDF: A3 sayfalar, 5000 mm çizgi 1:50'de 100 mm", "export": "pdf",
                  "expect": {"pdf": {"pages": pages, "tolerance": 0.01}}})

    # K12: saved and opened elsewhere: the drawing and the book whole.
    steps.append({
        "name": "K12 Kaydet ve başka ortamda aç: çizim ve pafta kitabı eksiksiz",
        "reopen": True,
        "expect": {"objects": {**final, "k11": line(*k11)}, "sheets": listed, "count": 17},
    })

    return {
        "format": "kentos.acceptance",
        "version": 1,
        "id": "room",
        "title": "Oda: çiz, ölç, düzelt ve tara; pafta, PDF ve DXF; başka ortamda aç (K01–K04, K11, K12)",
        "source": "plan/cad.md §23.1 K01–K04, K11, K12; TODOS.md CADP-12",
        "project": {"name": "Oda", "workspace": "cad", "coords": "local", "drawingUnit": "mm", "plotScale": 50},
        "tolerance": 1e-9,
        "steps": steps,
    }


def scene() -> dict:
    """The trace's drawing: a new local CAD project in millimetres at 1:50, one layer, nothing on it."""
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Oda",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 50, "workspace": "cad", "drawingUnit": "mm", "drawingFont": "barlow"},
        "origin": {"x": 0, "y": 0},
        "layers": [{"id": "cizim", "name": "Çizim", "type": "layer", "visible": True, "locked": False, "expanded": True,
                    "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}],
        "activeLayer": "cizim",
        "entities": [],
        "styles": {"items": [], "categories": []},
    }


def main() -> int:
    outputs = {OUT: json.dumps(build(), ensure_ascii=False, indent=1) + "\n",
               SCENE: json.dumps(scene(), ensure_ascii=False, indent=1) + "\n"}
    if "--check" in sys.argv[1:]:
        stale = [p for p, text in outputs.items() if not p.exists() or p.read_text("utf-8") != text]
        for p in stale:
            print(f"{p.relative_to(ROOT)}: yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)")
        if stale:
            return 1
        print(f"Kabul senaryosu tutarlı: {len(build()['steps'])} adım ve izin çizimi.")
        return 0
    for p, text in outputs.items():
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(text, "utf-8")
        print(f"yazıldı: {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
