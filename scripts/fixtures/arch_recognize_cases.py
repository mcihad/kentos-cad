#!/usr/bin/env python3
"""Planı tanı's shared cases (plan/architecture-advanced.md M48, docs/adr/8048), from the ADR's rules, without KentOS
code.

Each case draws a plan as an office's exploded DWG has it — walls' outlines as lines, a door's leaf and swing, a
window's lines, a column's outline — and says what `cad.arch.recognize` makes of it, as ADR 8048 gives the rules:

- a layer's role is the one given, else its name's (duvar, perde, bölme, wall → a wall's; kapı, door → a door's;
  pencere, window, glaz, cam → a window's; kolon, column → a column's; tarama, ölçü, yazı, aks, merdiven, mobilya …
  ignored); when no layer is a wall's, a layer whose name says nothing ("Çizim") is;
- a wall's two faces at most `maxWidth` apart make it; it runs between its faces' middle, west to east or south to
  north, as wide as its faces are apart, on its first line's layer (or `wallLayerId`); it runs to the walls it meets
  (to their line in a tee, both to their crossing at a corner) and through the columns it stops at;
- a gap in it: a face running on is a joint; an arc about a jamb as wide as the gap a door (Kapı 80 under 0.85 m, else
  Kapı 90; two arcs about both jambs İki kanatlı kapı 140), hinged at that jamb, opening to the arc's side, its swing
  the arc's when not 90°; a window's lines along it a window (Pencere 120); nothing in it a bare opening (Boşluk 100)
  when at least 0.3 m and at most `maxOpening`; each as wide as the gap, its middle the gap's; where the faces run on
  (a block's wipeout masked them), two overlapping window lines between them are a window as long as they are, a door's
  swing about a point of the wall a door from its frame to its frame;
- a column layer's closed outline of four right angles is a rectangular column about its middle (its width the side
  nearer east–west, turned as that side), its circle a round one as wide as it, filled, on its layer; a hatch whose
  middle lies in one goes with it; another outline stays;
- a block is read where it stands, its pieces by their layers (a piece of layer 0 takes the insert's): a door's, a
  window's or a column's block is that door, window or column, and goes when all of it is read;
- the walls in the order of their first line, the openings by wall and along it, the columns in their sources' order;
  walls, then openings, then columns take the new ids;
- a source taken whole goes (unless `keepSources`), the others stay, said by why (ignored layers by name, columns,
  others); one undo step “Planı tanı”.

Writes fixtures/commands/v1/cad.arch.recognize.json; `--check` compares and writes nothing.
"""

import argparse
import copy
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.arch.recognize.json"


def P(x, y):
    return {"x": float(x), "y": float(y)}


def layer(id, name, kind="layer", visible=True, locked=False, children=None):
    return {"id": id, "name": name, "type": kind, "visible": visible, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": children or []}


LAYERS = {"duvar": "DUVAR", "kapi": "KAPI", "pencere": "PENCERE", "kolon": "KOLON", "aks": "AKS", "cizim": "Çizim",
          "kilitli": "Kilitli katman", "gizli": "Gizli katman"}


def setup(entities, blocks=None):
    doc = {
        "format": "kentos.document", "version": 1, "name": "Ofis planı",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 50, "workspace": "cad",
                     "arch": {"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0}]}},
        "origin": {"x": 0, "y": 0},
        "layers": [layer("duvar", "DUVAR"), layer("kapi", "KAPI"), layer("pencere", "PENCERE"),
                   layer("kolon", "KOLON"), layer("aks", "AKS"), layer("cizim", "Çizim"),
                   layer("kilitli", "Kilitli katman", locked=True), layer("gizli", "Gizli katman", visible=False),
                   layer("pafta", "Pafta", kind="group", children=[layer("sinir", "Sınır")])],
        "activeLayer": "duvar",
        "styles": {"items": [], "categories": []},
        "entities": entities,
    }
    if blocks:
        doc["blocks"] = blocks
    return doc


class Plan:
    """An office's line work, each object with its id in the drawing (1, 2, …)."""

    def __init__(self):
        self.ents = []

    def add(self, kind, layer_id, **fields):
        self.ents.append({"kind": kind, "id": len(self.ents) + 1, "layerId": layer_id, "attrs": {}, **fields})
        return len(self.ents)

    def line(self, a, b, layer_id="duvar"):
        return self.add("line", layer_id, a=P(*a), b=P(*b))

    def arc(self, c, r, a0, sweep, layer_id="kapi"):
        return self.add("arc", layer_id, c=P(*c), r=float(r), a0=math.radians(a0), a1=math.radians(a0 + sweep))

    def square(self, c, size, layer_id="kolon"):
        h = size / 2
        return self.add("polyline", layer_id, pts=[P(c[0] - h, c[1] - h), P(c[0] + h, c[1] - h),
                                                   P(c[0] + h, c[1] + h), P(c[0] - h, c[1] + h)], closed=True)

    def circle(self, c, r, layer_id="kolon"):
        return self.add("circle", layer_id, c=P(*c), r=float(r))

    def outline(self, pts, layer_id="kolon"):
        return self.add("polyline", layer_id, pts=[P(*q) for q in pts], closed=True)

    def solid(self, pts, layer_id="kolon"):
        return self.add("hatch", layer_id, ring=[P(*q) for q in pts], pattern={"type": "solid", "angle": 0.0,
                                                                             "spacing": 1.0})

    def jamb(self, x, y0, y1, layer_id="duvar"):
        return self.line((x, y0), (x, y1), layer_id)

    def text(self, p, words, layer_id="cizim"):
        return self.add("text", layer_id, p=P(*p), text=words, height=0.2, rotation=0.0)

    def along_x(self, y, x0, x1, gaps=(), layer_id="duvar"):
        ids, x = [], x0
        for g0, g1 in gaps:
            ids.append(self.line((x, y), (g0, y), layer_id))
            x = g1
        ids.append(self.line((x, y), (x1, y), layer_id))
        return ids

    def along_y(self, x, y0, y1, gaps=(), layer_id="duvar"):
        ids, y = [], y0
        for g0, g1 in gaps:
            ids.append(self.line((x, y), (x, g0), layer_id))
            y = g1
        ids.append(self.line((x, y), (x, y1), layer_id))
        return ids

    def room(self, south=(), north=(), inner_south=(), inner_north=(), layer_id="duvar"):
        """A room of 4 × 3 m between its walls' centre lines, 0.2 m walls, their faces broken at the gaps: its outer
        and inner faces south, north, west and east, in this order."""
        self.along_x(-0.1, -0.1, 4.1, south, layer_id)
        self.along_x(0.1, 0.1, 3.9, inner_south, layer_id)
        self.along_x(3.1, -0.1, 4.1, north, layer_id)
        self.along_x(2.9, 0.1, 3.9, inner_north, layer_id)
        self.along_y(-0.1, -0.1, 3.1, (), layer_id)
        self.along_y(0.1, 0.1, 2.9, (), layer_id)
        self.along_y(4.1, -0.1, 3.1, (), layer_id)
        self.along_y(3.9, 0.1, 2.9, (), layer_id)


SOUTH, NORTH, WEST, EAST = ((0, 0), (4, 0)), ((0, 3), (4, 3)), ((0, 0), (0, 3)), ((4, 0), (4, 3))
ROOM = [SOUTH, NORTH, WEST, EAST]

# KentOS's opening types (docs/adr/0341, 0772): the fields each gives an opening, as the drawing keeps them.
TYPES = {
    "kentos-kapi-90": {"form": "door", "frame": 0.05, "head": 2.1},
    "kentos-kapi-80": {"form": "door", "frame": 0.05, "head": 2.1},
    "kentos-iki-kanatli-kapi-140": {"form": "door", "leaves": 2, "frame": 0.05, "head": 2.1},
    "kentos-pencere-120": {"form": "window", "leaves": 2, "frame": 0.06, "sill": 0.9, "head": 2.2},
    "kentos-bosluk-100": {"form": "opening", "operation": "fixed", "head": 2.1},
}


def wall(id, ends, width=0.2, layer_id="duvar", arch=None):
    (a, b) = ends
    e = {"kind": "wall", "id": id, "layerId": layer_id, "attrs": {}, "pts": [P(*a), P(*b)], "width": float(width),
         "justify": "center"}
    if arch:
        e["arch"] = arch
    return e


def opening(id, host, ends, t0, t1, kind, width=0.2, layer_id="kapi", hand=None, side=None, swing=None):
    """An opening of `kind` between `t0` and `t1` along its wall from its start, its jambs on both faces."""
    (a, b) = ends
    length = math.hypot(b[0] - a[0], b[1] - a[1])
    u = ((b[0] - a[0]) / length, (b[1] - a[1]) / length)
    n = (-u[1], u[0])
    h = width / 2
    at = lambda t, r: P(a[0] + u[0] * t + n[0] * r, a[1] + u[1] * t + n[1] * r)  # noqa: E731
    e = {"kind": "opening", "id": id, "layerId": layer_id, "attrs": {}, "host": f"$uidOf:{host}",
         "at": (t0 + t1) / 2, "width": t1 - t0, **TYPES[kind], "style": kind,
         "place": [at(t0, h), at(t0, -h), at(t1, h), at(t1, -h)]}
    if hand:
        e["hand"] = hand
    if side:
        e["side"] = side
    if swing is not None:
        e["swing"] = float(swing)
    return e


def column(id, at, width, depth=None, rotation=0.0, shape=None, layer_id="kolon"):
    e = {"kind": "column", "id": id, "layerId": layer_id, "attrs": {}, "at": P(*at), "width": float(width),
         "fill": True}
    if shape:
        e["shape"] = shape
    if depth is not None:
        e["depth"] = float(depth)
    if rotation:
        e["rotation"] = round(rotation * 1e9) / 1e9
    return e


def unread(left, ignored=(), columns=0, other=0):
    """`plan_unrecognized`'s warning: how many objects stayed, by why."""
    parts = []
    if ignored:
        names = sorted({LAYERS[k] for k in ignored}, key=lambda s: s.lower().replace("ç", "c"))
        parts.append(f"{len(ignored)} nesne yok sayılan katmanlarda ({', '.join(names)})")
    if columns:
        parts.append(f"{columns} kolon nesnesi kolona çevrilemedi (dikdörtgen ya da daire değil)")
    if other:
        parts.append(f"{other} nesne duvara, kapıya ya da pencereye ait değil")
    return {"code": "plan_unrecognized", "message": f"{left} nesne tanınmadı ve çizimde kaldı: {'; '.join(parts)}.",
            "path": "uids"}


def refusal(code, message, path, status="failed"):
    error = {"code": code, "message": message, "path": path}
    # A conflict says the drawing's revision now.
    if status == "conflict":
        error["revision"] = "$current"
    return {"status": status, "error": error}


def captures(plan):
    return [{"op": "captureUid", "id": e["id"], "as": f"e{e['id']}"} for e in plan.ents]


def done(plan, new, removed, left=(), warnings=(), keep=False, op="execute", counts=None):
    """A completed step: the new objects (walls, then openings) after the drawing's, `removed` and `left` its ids."""
    first = len(plan.ents) + 1
    walls = [e for e in new if e["kind"] == "wall"]
    openings = [e for e in new if e["kind"] == "opening"]
    columns = [e for e in new if e["kind"] == "column"]
    for k, e in enumerate(walls + openings + columns):
        assert e["id"] == first + k, (e["id"], first + k)
    forms = [e["form"] for e in openings]
    out = {"walls": [f"$uidOf:{e['id']}" for e in walls], "doors": forms.count("door"),
           "windows": forms.count("window"), "bare": forms.count("opening"), "revision": "$current"}
    if openings:
        out["openings"] = [f"$uidOf:{e['id']}" for e in openings]
    if columns:
        out["columns"] = [f"$uidOf:{e['id']}" for e in columns]
    if removed and not keep:
        out["removed"] = [f"$uid:e{i}" for i in removed]
    if left:
        out["left"] = [f"$uid:e{i}" for i in left]
    kept = [e["id"] for e in plan.ents if keep or e["id"] not in removed]
    if op == "plan":
        planned = {"walls": [dict(e, id=0) for e in walls], "revision": "$current"}
        if openings:
            planned["openings"] = [dict(e, id=0, host="$any") for e in openings]
        if columns:
            planned["columns"] = [dict(e, id=0) for e in columns]
        if removed and not keep:
            planned["removed"] = out["removed"]
        if left:
            planned["left"] = out["left"]
        return {"op": "plan", "result": {"status": "completed", "output": planned, "warnings": list(warnings)},
                "expect": {"ids": [e["id"] for e in plan.ents], "canUndo": False, "dirty": False,
                           "revision": "same"}, "within": 1e-9}
    return {"op": "execute", "result": {"status": "completed", "output": out, "warnings": list(warnings)},
            "expect": {"ids": kept + [e["id"] for e in walls + openings + columns],
                       "entities": {str(e["id"]): e for e in walls + openings + columns},
                       "canUndo": True, "dirty": True, "revision": "changed"},
            "within": 1e-9}


def uids_of(plan, ids=None):
    return [f"$uidOf:{i}" for i in (ids or [e["id"] for e in plan.ents])]


def case(name, plan, steps, input_=None, blocks=None):
    inp = {"uids": uids_of(plan)} if input_ is None else input_
    for s in steps:
        if s["op"] in ("execute", "plan", "validate"):
            s.setdefault("input", inp)
    return {"name": name, "setup": setup(plan.ents, blocks), "steps": captures(plan) + steps}


def piece(id, kind, **fields):
    """A block's piece on the block's layer (layer 0)."""
    return {"kind": kind, "id": id, "layerId": "", "attrs": {}, **fields}


DOOR, WINDOW, POST = ("0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4e01", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4e02",
                      "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4e03")
# An office's blocks, in their own axes: along the wall from the start jamb, across to the wall's left.
BLOCKS = [
    {"id": DOOR, "name": "KAPI-90-20", "base": P(0, 0), "entities": [
        piece(1, "line", a=P(0.05, 0.1), b=P(0.05, 0.9)),
        piece(2, "arc", c=P(0.05, 0.1), r=0.8, a0=0.0, a1=math.pi / 2),
        piece(3, "polyline", pts=[P(0, -0.1), P(0.05, -0.1), P(0.05, 0.1), P(0, 0.1)], closed=True),
        piece(4, "polyline", pts=[P(0.85, -0.1), P(0.9, -0.1), P(0.9, 0.1), P(0.85, 0.1)], closed=True)]},
    {"id": WINDOW, "name": "PENCERE-120-20", "base": P(0, 0), "entities": [
        *[piece(k + 1, "line", a=P(0, y), b=P(1.2, y)) for k, y in enumerate((-0.1, -0.02, 0.02, 0.1))],
        piece(5, "line", a=P(-0.05, -0.13), b=P(1.25, -0.13))]},
    {"id": POST, "name": "KOLON-30", "base": P(0, 0), "entities": [
        piece(1, "polyline", pts=[P(-0.15, -0.15), P(0.15, -0.15), P(0.15, 0.15), P(-0.15, 0.15)], closed=True),
        piece(2, "hatch", ring=[P(-0.15, -0.15), P(0.15, -0.15), P(0.15, 0.15), P(-0.15, 0.15)],
              pattern={"type": "solid", "angle": 0.0, "spacing": 1.0})]},
]


def cases():
    out = []
    all_of = lambda p: [e["id"] for e in p.ents]  # noqa: E731

    # A room.
    p = Plan()
    p.room()
    first = len(p.ents) + 1
    walls = [wall(first + k, ends) for k, ends in enumerate(ROOM)]
    step = done(p, walls, all_of(p))
    undo = {"op": "undo", "returns": "Planı tanı",
            "expect": {"ids": all_of(p), "canUndo": False, "canRedo": True, "revision": "changed"}}
    redo = {"op": "redo", "returns": "Planı tanı",
            "expect": dict(step["expect"], canRedo=False), "within": 1e-9}
    out.append(case("oda: dört duvarın dış ve iç çizgileri köşelerinde buluşan dört duvar olur, çizgiler silinir; tek "
                    "adım “Planı tanı”, geri alınır", p, [step, undo, redo]))

    # Doors.
    def door_room(gap, *arcs, leaf=True):
        q = Plan()
        q.room(south=[gap], inner_south=[gap])
        q.line((gap[0], -0.1), (gap[0], 0.1))
        q.line((gap[1], -0.1), (gap[1], 0.1))
        if leaf:
            q.line((gap[0], 0.1), (gap[0], 0.1 + gap[1] - gap[0]), "kapi")
        for c, r, a0, sweep in arcs:
            q.arc(c, r, a0, sweep)
        return q

    p = door_room((1.0, 1.9), ((1.0, 0.1), 0.9, 0.0, 90.0))
    first = len(p.ents) + 1
    walls = [wall(first + k, ends) for k, ends in enumerate(ROOM)]
    door = opening(first + 4, first, SOUTH, 1.0, 1.9, "kentos-kapi-90")
    out.append(case("kapı: pervazının çevresinde boşluk kadar yay Kapı 90 olur; menteşe başlangıçta, kapı yayın yanına "
                    "(sola) açılır; kanat ve yay silinir", p, [done(p, walls + [door], all_of(p))]))

    p = door_room((1.0, 1.9), ((1.9, -0.1), 0.85, 180.0, 100.0), leaf=False)
    first = len(p.ents) + 1
    walls = [wall(first + k, ends) for k, ends in enumerate(ROOM)]
    door = opening(first + 4, first, SOUTH, 1.0, 1.9, "kentos-kapi-90", hand="end", side="right", swing=100)
    out.append(case("kapı: uzak pervazdan dışarı 100° açılan yay: menteşe sonda, sağa açılır, açısı 100°", p,
                    [done(p, walls + [door], all_of(p))]))

    p = door_room((1.0, 2.4), ((1.0, 0.1), 0.7, 0.0, 90.0), ((2.4, 0.1), 0.7, 90.0, 90.0), leaf=False)
    first = len(p.ents) + 1
    walls = [wall(first + k, ends) for k, ends in enumerate(ROOM)]
    door = opening(first + 4, first, SOUTH, 1.0, 2.4, "kentos-iki-kanatli-kapi-140")
    out.append(case("iki kanatlı kapı: iki pervazın çevresinde yarım genişlikte iki yay İki kanatlı kapı 140 olur", p,
                    [done(p, walls + [door], all_of(p))]))

    p = door_room((1.0, 1.8), ((1.0, 0.1), 0.8, 0.0, 90.0))
    first = len(p.ents) + 1
    walls = [wall(first + k, ends) for k, ends in enumerate(ROOM)]
    door = opening(first + 4, first, SOUTH, 1.0, 1.8, "kentos-kapi-80")
    out.append(case("dar kapı: 0,80 m'lik kapı Kapı 80 olur", p, [done(p, walls + [door], all_of(p))]))

    # Blocks: a door's as drawn, a window's turned a half turn and mirrored (its sill outside the north wall), a column's.
    p = Plan()
    p.room(south=[(1.0, 1.9)], inner_south=[(1.0, 1.9)], north=[(2.0, 3.2)], inner_north=[(2.0, 3.2)])
    for x, y0, y1 in ((1.0, -0.1, 0.1), (1.9, -0.1, 0.1), (2.0, 2.9, 3.1), (3.2, 2.9, 3.1)):
        p.jamb(x, y0, y1)
    p.add("insert", "kapi", block=DOOR, p=P(1.0, 0.0), scale=1.0, rotation=0.0)
    p.add("insert", "pencere", block=WINDOW, p=P(3.2, 3.0), scale=1.0, rotation=math.pi, mirror=True)
    p.add("insert", "kolon", block=POST, p=P(2.0, 1.5), scale=1.0, rotation=0.0)
    first = len(p.ents) + 1
    walls = [wall(first + k, ends) for k, ends in enumerate(ROOM)]
    door = opening(first + 4, first, SOUTH, 1.0, 1.9, "kentos-kapi-90")
    window = opening(first + 5, first + 1, NORTH, 2.0, 3.2, "kentos-pencere-120", layer_id="pencere")
    post = column(first + 6, (2.0, 1.5), 0.3)
    out.append(case("bloklar yerinde okunur: kapı bloğu kapı, yarım dönüp aynalanmış pencere bloğu pencere, kolon bloğu "
                    "kolon olur; 0 katmanındaki parçalar bloğun katmanını alır, bloklar silinir", p,
                    [done(p, walls + [door, window, post], all_of(p))], blocks=BLOCKS))

    # A window.
    p = Plan()
    p.room(north=[(2.0, 3.2)], inner_north=[(2.0, 3.2)])
    p.line((2.0, 2.9), (2.0, 3.1))
    p.line((3.2, 2.9), (3.2, 3.1))
    for y in (2.9, 2.98, 3.02, 3.1):
        p.line((2.0, y), (3.2, y), "pencere")
    p.line((1.95, 3.15), (3.25, 3.15), "pencere")
    first = len(p.ents) + 1
    walls = [wall(first + k, ends) for k, ends in enumerate(ROOM)]
    window = opening(first + 4, first + 1, NORTH, 2.0, 3.2, "kentos-pencere-120", layer_id="pencere")
    out.append(case("pencere: boşluk boyunca pencere çizgileri Pencere 120 olur; çizgileri ve denizliği silinir", p,
                    [done(p, walls + [window], all_of(p))]))

    # Drawn over the wall: the faces run on under a window's and a door's blocks.
    p = Plan()
    p.room()
    for y in (2.9, 2.98, 3.02, 3.1):
        p.line((2.0, y), (3.2, y), "pencere")
    p.arc((1.05, 0.1), 0.8, 0.0, 90.0)
    for x0, x1 in ((1.0, 1.05), (1.85, 1.9)):
        p.add("polyline", "kapi", pts=[P(x0, -0.1), P(x1, -0.1), P(x1, 0.1), P(x0, 0.1)], closed=True)
    first = len(p.ents) + 1
    walls = [wall(first + k, ends) for k, ends in enumerate(ROOM)]
    door = opening(first + 4, first, SOUTH, 1.0, 1.9, "kentos-kapi-90")
    window = opening(first + 5, first + 1, NORTH, 2.0, 3.2, "kentos-pencere-120", layer_id="pencere")
    out.append(case("duvarın üstüne çizilmiş: yüzler sürerken iki pencere çizgisi pencere, kasalarıyla bir kapı yayı kapı "
                    "olur", p, [done(p, walls + [door, window], all_of(p))]))

    # A bare opening; a slip of the pen; a wide gap.
    p = Plan()
    p.room(south=[(3.0, 3.5)], inner_south=[(3.0, 3.5)])
    p.line((3.0, -0.1), (3.0, 0.1))
    p.line((3.5, -0.1), (3.5, 0.1))
    first = len(p.ents) + 1
    walls = [wall(first + k, ends) for k, ends in enumerate(ROOM)]
    bare = opening(first + 4, first, SOUTH, 3.0, 3.5, "kentos-bosluk-100", layer_id="duvar")
    out.append(case("boşluk: içi boş 0,5 m'lik aralık Boşluk 100 olur, duvarın katmanında; iki pervazı duvar sayılmaz",
                    p, [done(p, walls + [bare], all_of(p))]))

    p = Plan()
    p.room(south=[(3.0, 3.1)], inner_south=[(3.0, 3.1)])
    first = len(p.ents) + 1
    out.append(case("kalem kayması: 0,1 m'lik boş aralıkta duvar sürer", p,
                    [done(p, [wall(first + k, ends) for k, ends in enumerate(ROOM)], all_of(p))]))

    p = Plan()
    p.along_x(0.0, 0.0, 2.0)
    p.along_x(0.2, 0.0, 2.0)
    p.along_x(0.0, 5.5, 8.0)
    p.along_x(0.2, 5.5, 8.0)
    first = len(p.ents) + 1
    out.append(case("geniş aralık duvarı bitirir: aynı doğrultuda 3,5 m ötedeki duvar ayrı duvardır", p,
                    [done(p, [wall(first, ((0, 0.1), (2, 0.1))), wall(first + 1, ((5.5, 0.1), (8, 0.1)))],
                          all_of(p))]))

    # A partition in a tee at both ends.
    p = Plan()
    p.room(inner_south=[(2.45, 2.55)], inner_north=[(2.45, 2.55)])
    p.along_y(2.45, 0.1, 2.9)
    p.along_y(2.55, 0.1, 2.9)
    first = len(p.ents) + 1
    ends = [SOUTH, NORTH, WEST, EAST, ((2.5, 0), (2.5, 3))]
    walls = [wall(first + k, e, 0.1 if k == 4 else 0.2) for k, e in enumerate(ends)]
    out.append(case("bölme: iki duvara saplanan 0,10 m'lik bölme ikisinin de eksenine uzar; onlar kesintisiz geçer", p,
                    [done(p, walls, all_of(p))]))

    # Columns.
    p = Plan()
    p.along_x(-0.1, 0.15, 3.85, [(1.85, 2.15)])
    p.along_x(0.1, 0.15, 3.85, [(1.85, 2.15)])
    p.along_x(3.1, -0.1, 4.1)
    p.along_x(2.9, 0.1, 3.9)
    p.along_y(-0.1, 0.15, 3.1)
    p.along_y(0.1, 0.15, 2.9)
    p.along_y(4.1, 0.15, 3.1)
    p.along_y(3.9, 0.15, 2.9)
    for c in ((0, 0), (2, 0), (4, 0)):
        p.square(c, 0.3)
    first = len(p.ents) + 1
    walls = [wall(first + k, ends) for k, ends in enumerate(ROOM)]
    cols = [column(first + 4 + k, c, 0.3) for k, c in enumerate(((0, 0), (2, 0), (4, 0)))]
    out.append(case("kolon: kolonda duran duvarlar kolondan geçer ve köşede buluşur; kolon çevresi dolu kolon olur",
                    p, [done(p, walls + cols, all_of(p))]))

    # A round column, a turned one with its hatch, an L that is no column.
    p = Plan()
    p.room()
    p.circle((2.0, 1.5), 0.2)
    p.solid([(1.9, 1.4), (2.1, 1.4), (2.1, 1.6), (1.9, 1.6)])
    turn = math.radians(30.0)
    u, n = (math.cos(turn), math.sin(turn)), (-math.sin(turn), math.cos(turn))
    corner = lambda a, b: (1.0 + u[0] * a + n[0] * b, 1.5 + u[1] * a + n[1] * b)  # noqa: E731
    p.outline([corner(-0.25, -0.15), corner(0.25, -0.15), corner(0.25, 0.15), corner(-0.25, 0.15)])
    ell = p.outline([(3.0, 1.0), (3.4, 1.0), (3.4, 1.15), (3.15, 1.15), (3.15, 1.6), (3.0, 1.6)])
    first = len(p.ents) + 1
    walls = [wall(first + k, ends) for k, ends in enumerate(ROOM)]
    cols = [column(first + 4, (2.0, 1.5), 0.4, shape="round"),
            column(first + 5, (1.0, 1.5), 0.5, depth=0.3, rotation=turn)]
    out.append(case("yuvarlak, dönük ve L kolon: daire yuvarlak kolon, döndürülmüş dikdörtgen dönük kolon olur, "
                    "taraması kolonla gider; L çevre kalır ve söylenir", p,
                    [done(p, walls + cols, [i for i in all_of(p) if i != ell], [ell], [unread(1, columns=1)])]))

    # Components and neighbours.
    p = Plan()
    for y in (0.0, 0.05, 0.3):
        p.along_x(y, 0.0, 5.0)
    out.append(case("yalıtım çizgisi duvarın bileşen çizgisidir: 0,05 m dışındaki çizgiyle tek 0,30 m'lik duvar", p,
                    [done(p, [wall(len(p.ents) + 1, ((0, 0.15), (5, 0.15)), 0.3)], all_of(p))]))

    p = Plan()
    for y in (0.0, 0.15, 0.55, 0.7):
        p.along_x(y, 0.0, 3.0)
    first = len(p.ents) + 1
    out.append(case("şaftın iki yanındaki 0,15 m'lik duvarlar iki ayrı duvardır", p,
                    [done(p, [wall(first, ((0, 0.075), (3, 0.075)), 0.15),
                              wall(first + 1, ((0, 0.625), (3, 0.625)), 0.15)], all_of(p))]))

    p = Plan()
    p.room()
    p.line((-0.1, -0.1), (1.5, -0.1))
    p.line((2.5, -0.1), (1.2, -0.1))
    p.line((2.4, -0.1), (4.1, -0.1))
    first = len(p.ents) + 1
    out.append(case("parça parça, üst üste ve ters çizilmiş yüz bir kez çizilmiş gibi okunur; parçalar da silinir", p,
                    [done(p, [wall(first + k, ends) for k, ends in enumerate(ROOM)], all_of(p))]))

    # Layers' roles.
    p = Plan()
    p.room()
    grids = [p.line((-1, 0), (5, 0), "aks"), p.line((0, -1), (0, 4), "aks")]
    note = p.text((2, 1.5), "Salon")
    first = len(p.ents) + 1
    stay = grids + [note]
    out.append(case("aks çizgileri ve rolü olmayan katmanın yazısı kullanılmaz, kalır ve söylenir", p,
                    [done(p, [wall(first + k, ends) for k, ends in enumerate(ROOM)],
                          [i for i in all_of(p) if i not in stay], stay,
                          [unread(3, ignored=["aks", "aks", "cizim"])])]))

    p = Plan()
    p.room(south=[(1.0, 1.9)], inner_south=[(1.0, 1.9)], layer_id="cizim")
    p.line((1.0, -0.1), (1.0, 0.1), "cizim")
    p.line((1.9, -0.1), (1.9, 0.1), "cizim")
    p.line((1.0, 0.1), (1.0, 1.0), "cizim")
    p.arc((1.0, 0.1), 0.9, 0.0, 90.0, "cizim")
    first = len(p.ents) + 1
    walls = [wall(first + k, ends, layer_id="cizim") for k, ends in enumerate(ROOM)]
    door = opening(first + 4, first, SOUTH, 1.0, 1.9, "kentos-kapi-90", layer_id="cizim")
    out.append(case("tek katmanda çizilmiş plan: hiçbir katman duvar değilken adı bir şey söylemeyen katman duvardır; "
                    "yay yine kapıdır", p, [done(p, walls + [door], all_of(p))]))

    p = Plan()
    p.room(layer_id="pencere")
    first = len(p.ents) + 1
    out.append(case("verilen rol adından önce gelir: PENCERE katmanındaki duvar çizgileri duvar olur", p,
                    [done(p, [wall(first + k, ends, layer_id="pencere") for k, ends in enumerate(ROOM)],
                          all_of(p))],
                    {"uids": uids_of(p), "roles": [{"layerId": "pencere", "role": "wall"}]}))

    # Options.
    p = door_room((1.0, 1.9), ((1.0, 0.1), 0.9, 0.0, 90.0))
    first = len(p.ents) + 1
    walls = [wall(first + k, ends) for k, ends in enumerate(ROOM)]
    door = opening(first + 4, first, SOUTH, 1.0, 1.9, "kentos-kapi-90")
    out.append(case("kaynak korunur: duvarlar ve kapı çizilir, çizgiler kalır", p,
                    [done(p, walls + [door], all_of(p), keep=True)],
                    {"uids": uids_of(p), "keepSources": True}))

    p = Plan()
    p.room()
    first = len(p.ents) + 1
    arch = {"level": "zemin", "phase": "existing"}
    out.append(case("kat ve tadilat durumu: her duvar verilen mimari bilgiyi alır (zemin kat, mevcut)", p,
                    [done(p, [wall(first + k, ends, arch=arch) for k, ends in enumerate(ROOM)], all_of(p))],
                    {"uids": uids_of(p), "arch": arch}))

    p = door_room((1.0, 1.9), ((1.0, 0.1), 0.9, 0.0, 90.0))
    first = len(p.ents) + 1
    walls = [wall(first + k, ends, layer_id="cizim") for k, ends in enumerate(ROOM)]
    door = opening(first + 4, first, SOUTH, 1.0, 1.9, "kentos-kapi-90", layer_id="pencere")
    out.append(case("katmanlar verilir: duvarlar wallLayerId'ye, açıklıklar openingLayerId'ye", p,
                    [done(p, walls + [door], all_of(p))],
                    {"uids": uids_of(p), "wallLayerId": "cizim", "openingLayerId": "pencere"}))

    p = Plan()
    p.room()
    first = len(p.ents) + 1
    hidden = {"code": "layer_hidden", "message": "“Gizli katman” katmanı gizli; çizilen nesne görünmeyecek.",
              "path": "wallLayerId"}
    out.append(case("gizli katmana uyarıyla yazılır", p,
                    [done(p, [wall(first + k, ends, layer_id="gizli") for k, ends in enumerate(ROOM)], all_of(p),
                          warnings=[hidden])],
                    {"uids": uids_of(p), "wallLayerId": "gizli"}))

    p = Plan()
    p.room()
    locked = p.line((5, 0), (6, 0), "kilitli")
    first = len(p.ents) + 1
    skipped = {"code": "layer_locked", "message": "1 nesne kilitli katmanda olduğu için atlandı. Değiştirmek için "
               "katmanın kilidini Katmanlar panelinden açın.", "path": "uids"}
    step = done(p, [wall(first + k, ends) for k, ends in enumerate(ROOM)], [i for i in all_of(p) if i != locked],
                warnings=[skipped])
    out.append(case("kilitli katmandaki kaynak atlanır ve söylenir", p, [step]))

    p = door_room((1.0, 1.9), ((1.0, 0.1), 0.9, 0.0, 90.0))
    first = len(p.ents) + 1
    walls = [wall(first + k, ends) for k, ends in enumerate(ROOM)]
    door = opening(first + 4, first, SOUTH, 1.0, 1.9, "kentos-kapi-90")
    out.append(case("plan yazmadan duvarları, açıklıkları ve silinecek kaynakları gösterir", p,
                    [done(p, walls + [door], all_of(p), op="plan")]))

    # Refusals.
    p = Plan()
    p.room()
    same = {"ids": all_of(p), "canUndo": False, "dirty": False, "revision": "same"}

    def refused(name, plan, input_, result, non_finite=None):
        s = {"op": "execute", "input": input_, "result": result, "expect": dict(same, ids=all_of(plan))}
        if non_finite:
            s["nonFinite"] = non_finite
        out.append(case(name, plan, [s]))

    refused("kimlik verilmezse ret", p, {"uids": []},
            refusal("no_entities", "Tanınacak planın nesneleri verilmedi. En az bir nesnenin kalıcı kimliğini verin.",
                    "uids"))
    refused("geçersiz kimlik reddedilir", p, {"uids": ["duvar"]},
            refusal("invalid_uid", "“duvar” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir "
                    "(01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da "
                    "çizimden alın.", "uids[0]"))
    gone = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"
    refused("çizimde olmayan kimlik reddedilir", p, {"uids": [gone]},
            refusal("entity_not_found", f"“{gone}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir."
                    " Var olan bir nesnenin kimliğini verin.", "uids[0]"))
    q = Plan()
    q.room(layer_id="kilitli")
    refused("yalnız kilitli kaynak: ret", q, {"uids": uids_of(q)},
            refusal("layer_locked", "“Kilitli katman” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar "
                    "panelinden açın.", "uids[0]"))
    refused("rolü verilen katman çizimde yoksa ret", p,
            {"uids": uids_of(p), "roles": [{"layerId": "yok", "role": "wall"}]},
            refusal("layer_not_found", "“yok” kimlikli katman çizimde yok. Rolünü verdiğiniz katman çizimin bir katmanı "
                    "olmalı.", "roles[0].layerId"))
    refused("sonsuz en kalın duvar reddedilir", p, {"uids": uids_of(p), "maxWidth": 0.6},
            refusal("not_finite", "En kalın duvar sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.",
                    "maxWidth"), {"maxWidth": "Infinity"})
    refused("en ince duvar sıfırsa ret", p, {"uids": uids_of(p), "minWidth": 0.0},
            refusal("recognize_option", "En ince duvar sıfırdan kalın olmalı.", "minWidth"))
    refused("en kalın duvar en inceden kalın değilse ret", p, {"uids": uids_of(p), "minWidth": 0.3, "maxWidth": 0.2},
            refusal("recognize_option", "En kalın duvar en ince duvardan kalın olmalı.", "maxWidth"))
    refused("en geniş açıklık sıfırsa ret", p, {"uids": uids_of(p), "maxOpening": 0.0},
            refusal("recognize_option", "En geniş açıklık sıfırdan geniş olmalı.", "maxOpening"))
    refused("açı 45°'den büyükse ret", p, {"uids": uids_of(p), "angle": 50.0},
            refusal("recognize_option", "Açı 0 ile 45° arasında olmalı.", "angle"))
    refused("olmayan kat reddedilir", p, {"uids": uids_of(p), "arch": {"level": "yok"}},
            refusal("level_not_found", "Projede “yok” kimlikli kat yok. Katı Proje ayarları › Mimari'de ekleyin ya da "
                    "var olan bir katı seçin.", "arch.level"))
    q = Plan()
    q.line((-1, 0), (5, 0), "aks")
    q.text((2, 1.5), "Salon")
    refused("duvar tanınmazsa ret: aks ve yazıdan duvar çıkmaz", q, {"uids": uids_of(q)},
            refusal("plan_unrecognized", "Seçimde duvar tanınmadı: duvarın iki yüzü 0.05–0.6 m arayla paralel iki çizgi "
                    "olmalı. Duvar çizgilerinin katmanına Duvar rolünü verin; blokları önce patlatın.", "uids"))
    refused("duvar katmanı yok sayılırsa ret", p,
            {"uids": uids_of(p), "roles": [{"layerId": "duvar", "role": "ignore"}]},
            refusal("plan_unrecognized", "Seçimde duvar tanınmadı: duvarın iki yüzü 0.05–0.6 m arayla paralel iki çizgi "
                    "olmalı. Duvar çizgilerinin katmanına Duvar rolünü verin; blokları önce patlatın.", "uids"))
    refused("kilitli katmana yazılmaz", p, {"uids": uids_of(p), "wallLayerId": "kilitli"},
            refusal("layer_locked", "“Kilitli katman” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir "
                    "katmanı etkinleştirin.", "wallLayerId"))
    refused("katman grubuna yazılmaz", p, {"uids": uids_of(p), "wallLayerId": "pafta"},
            refusal("not_a_layer", "“Pafta” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman "
                    "seçin.", "wallLayerId"))
    refused("olmayan katmana yazılmaz", p, {"uids": uids_of(p), "wallLayerId": "yok"},
            refusal("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.",
                    "wallLayerId"))
    refused("geçersiz beklenen sürüm reddedilir", p, {"uids": uids_of(p), "expectedRevision": "x"},
            refusal("invalid_revision", "Beklenen sürüm “x” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı "
                    "yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"))
    refused("hazırlandığı sürümden sonra değişen çizime yazılmaz: conflict", p,
            {"uids": uids_of(p), "expectedRevision": "7"},
            refusal("revision_conflict", "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu "
                    "çizimin şimdiki hâline göre yeniden hazırlayın.", "expectedRevision", "conflict"))
    return out


def fixture():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.arch.recognize", "commandVersion": 1,
            "title": "Planı tanı",
            "note": "ADR 8048. Beklenenler scripts/fixtures/arch_recognize_cases.py'nin ADR'den yazılmış kurallarından, "
                    "KentOS kodu olmadan: her durum bir ofis planının patlatılmış çizgilerini çizer; duvarlar, açıklıklar, "
                    "silinen ve kalan kaynaklar kuraldan.",
            "setup": setup([]), "cases": cases()}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    text = json.dumps(fixture(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if OUT.read_text() != text:
            sys.exit(f"{OUT} differs: run without --check")
        print(f"{OUT.relative_to(ROOT)}: up to date")
        return
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(fixture()['cases'])} cases")


if __name__ == "__main__":
    main()
