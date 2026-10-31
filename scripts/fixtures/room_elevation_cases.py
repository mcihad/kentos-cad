#!/usr/bin/env python3
"""A room's interior elevations, a derived drawing (docs/adr/0799, ADV-06f): the shared cases of `cad.elevations.edit`
and `cad.elevations.query`, written from the ADR's rules without KentOS code.

The drawing: a 5 × 4 m room “101 Salon” on the ground floor (four 20 cm walls about their axes, the level 3 m high)
with a door, a window, a window without a head, a niche seen from the room, a niche on the far face, a flue, a column
and a cupboard (a block) inside; a kitchen on the first floor without walls or heights; a room with one curved side;
a room bounded by its walls' axes (gross); a room with a hole and a stair; and a room whose ring turns clockwise, with
one wall of its own height, a door and an opening that runs past its wall's end.

The rules (docs/adr/0799 §2):

- A side is a straight edge of the space's outer rings, numbered from 0 part by part. Seen from inside, its drawing
  runs from the edge's end to its start when the ring turns counter-clockwise (from its start to its end when
  clockwise): x(p) = (b − p)·u or (p − a)·u, u the edge's unit direction. A curved side is not drawn (curvedSide).
- A wall on the space's level whose straight segment is parallel to the side (|u × v| ≤ 1e-6) and whose left or right
  face (its offset, plus 0, w/2 or w as it is justified left, centre or right; the right face w less) lies on the side
  within 1 mm covers [max(min x, 0), min(max x, L)] of its segment's ends; stretches no face covers are open.
- Its height is the space's; else the lowest of its faces' walls' (a wall's own, else its level's); else unknown (only
  the floor line). A face lower than it draws its top across its stretch.
- An opening of a side's wall: broken → brokenOpening, a flue → flue (neither drawn); else its two jamb points on the
  matching face (place[0], place[2] on the left face, place[1], place[3] on the right, both within 1 mm of the side)
  give [x0, x1] clipped to [0, L]; a niche shows on its own face only; without a head (headUnknown) its jambs are dashed
  up to the side's height, not drawn when that is unknown. Openings are drawn by x0, then id; a numbered one with a
  head gets its number in its middle.
- Each side: its frame (closed, 0..L × 0..H) or floor line, its walls' tops, its openings, their numbers, the title
  “101 Salon — A” 4h below the floor (top centre), the width dimension 2h below and the height dimension 2h right, h =
  2.5 mm × the drawing scale. Sides go left to right from the origin (3 m right of the space's box, at its bottom),
  each L + gap (1 m) after the last.
- The room's holes, and the columns, stairs, ramps and blocks of its level with a point inside its outer rings
  (even-odd), are listed, not drawn.
- The objects go on a new layer “ic-gorunus-k” (the first free k) named “101 Salon iç görünüşü” (made unique as
  layers are: “… 2”) in the top group “İç görünüşler” (“ic-gorunusler”, made when the drawing has none), one undo step
  “Oda iç görünüşü”, the layer's derivation record naming the space, the sides asked, the origin, the gap, each object
  by its role and the drawing's digest. A refresh draws again from the record and keeps the objects of the same roles
  (changed ones written over, deleted ones made again with their ids), deletes those no longer drawn; nothing changed
  is `nothing_to_set`. A release removes the record. A read says current, stale (drawn again it differs, or objects
  are missing) or gone (the space is gone), with the members changed by hand while the drawing is the same.

    python3 scripts/fixtures/room_elevation_cases.py          # writes the cases
    python3 scripts/fixtures/room_elevation_cases.py --check  # compares with the files
"""

import copy
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EDIT = ROOT / "fixtures/commands/v1/cad.elevations.edit.json"
QUERY = ROOT / "fixtures/commands/v1/cad.elevations.query.json"

STEP = "Oda iç görünüşü"
GROUP = "İç görünüşler"
ANY = {"status": "completed", "output": "$any", "warnings": "$any"}
WITHIN = 1e-9
PLOT = 50.0
H = 0.0025 * PLOT
BLOCK = "0192f5a1-7990-7000-8000-0000000000b1"


def UID(n):
    return f"0192f5a1-7990-7000-8000-{n:012x}"


def P(x, y):
    return {"x": float(x), "y": float(y)}


def layer(id, name, locked=False):
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


# ── The drawing ──────────────────────────────────────────────────────────────────────────────────────────────────


def wall(id, a, b, width=0.2, level="zemin", height=None):
    w = {"kind": "wall", "id": id, "layerId": "duvar", "attrs": {}, "pts": [P(*a), P(*b)], "width": width,
         "justify": "center"}
    if height is not None:
        w["height"] = height
    if level:
        w["arch"] = {"level": level}
    return w


def faces(w):
    lf = w.get("offset", 0.0) + {"left": 0.0, "center": w["width"] / 2.0, "right": w["width"]}[w.get("justify", "center")]
    return lf, lf - w["width"]


def place(w, at, width):
    """An opening's jambs from its wall: the start's left and right face points, then the end's."""
    a, b = w["pts"][0], w["pts"][1]
    dx, dy = b["x"] - a["x"], b["y"] - a["y"]
    l = math.sqrt(dx * dx + dy * dy)
    ux, uy = dx / l, dy / l
    nx, ny = -uy, ux
    lf, rf = faces(w)
    out = []
    for s in (at - width / 2.0, at + width / 2.0):
        for d in (lf, rf):
            out.append(P(a["x"] + ux * s + nx * d, a["y"] + uy * s + ny * d))
    return out


def opening(id, w, at, width, form, sill=None, head=None, number=None, side=None, depth=None, broken=None):
    o = {"kind": "opening", "id": id, "layerId": "kapi", "attrs": {}, "host": UID(w["id"]), "at": at, "width": width,
         "form": form}
    if side:
        o["side"] = side
    if sill is not None:
        o["sill"] = sill
    if head is not None:
        o["head"] = head
    if depth is not None:
        o["depth"] = depth
    if number:
        o["number"] = number
    o["place"] = place(w, at, width)
    if broken:
        o["broken"] = broken
    return o


def space(id, ring, number=None, name=None, level=None, height=None, method=None, bulges=None, holes=None):
    outer = {"pts": [P(*p) for p in ring]}
    if bulges:
        outer["bulges"] = bulges
    part = {"outer": outer}
    if holes:
        part["holes"] = [{"pts": [P(*p) for p in h]} for h in holes]
    s = {"kind": "space", "id": id, "layerId": "mahal", "attrs": {}, "parts": [part]}
    if method:
        s["method"] = method
    if number:
        s["number"] = number
    if name:
        s["name"] = name
    if level:
        s["arch"] = {"level": level}
    if height is not None:
        s["height"] = height
    return s


W1 = wall(1, (0, 0), (5, 0))
W2 = wall(2, (5, 0), (5, 4))
W3 = wall(3, (5, 4), (0, 4))
W4 = wall(4, (0, 4), (0, 0))
W6 = wall(21, (39.9, -0.1), (44.1, -0.1), level=None, height=2.7)


def entities(w2_height=None, s1_height=None):
    w2 = copy.deepcopy(W2)
    if w2_height is not None:
        w2["height"] = w2_height
    return [
        W1, w2, W3, W4,
        space(5, [(0.1, 0.1), (4.9, 0.1), (4.9, 3.9), (0.1, 3.9)], "101", "Salon", "zemin", s1_height),
        opening(6, W1, 1.5, 0.9, "door", head=2.1, number="K1"),
        opening(7, w2, 2.0, 1.2, "window", sill=0.9, head=2.2, number="P1"),
        opening(8, W3, 2.5, 1.0, "window", sill=1.0, number="P2"),
        opening(9, W4, 2.0, 0.6, "niche", sill=0.5, head=1.5, depth=0.08),
        opening(10, W4, 1.0, 0.4, "niche", sill=0.3, head=0.9, side="right", depth=0.08),
        opening(11, w2, 3.2, 0.3, "flue", depth=0.1),
        {"kind": "column", "id": 12, "layerId": "mobilya", "attrs": {}, "at": P(2.5, 2.0), "width": 0.3, "depth": 0.3,
         "arch": {"level": "zemin"}},
        {"kind": "insert", "id": 13, "layerId": "mobilya", "attrs": {}, "block": BLOCK, "p": P(1.0, 3.0), "scale": 1.0,
         "rotation": 0.0, "arch": {"level": "zemin"}},
        {"kind": "insert", "id": 14, "layerId": "mobilya", "attrs": {}, "block": BLOCK, "p": P(1.5, 3.0), "scale": 1.0,
         "rotation": 0.0, "arch": {"level": "kat1"}},
        space(15, [(10, 0), (13, 0), (13, 2), (10, 2)], "102", "Mutfak", "kat1"),
        space(16, [(20, 0), (24, 0), (24, 3), (20, 3)], "103", "Kule", height=2.6, bulges=[0.0, 0.2, 0.0, 0.0]),
        space(17, [(25, 0), (28, 0), (28, 3), (25, 3)], "104", "Depo", method="gross"),
        space(18, [(30, 0), (34, 0), (34, 4), (30, 4)], "105", "Hol", height=2.5,
              holes=[[(31.5, 1.5), (32.5, 1.5), (32.5, 2.5), (31.5, 2.5)]]),
        {"kind": "stair", "id": 19, "layerId": "mobilya", "attrs": {}, "pts": [P(30.5, 0.5), P(30.5, 3.5)], "width": 1.0,
         "risers": 16, "riserHeight": 0.175, "tread": 0.25},
        {"kind": "ramp", "id": 20, "layerId": "mobilya", "attrs": {}, "pts": [P(35, 0), P(38, 0)], "width": 1.2,
         "rise": 0.15, "slope": 5.0},
        W6,
        space(22, [(40, 0), (40, 3), (44, 3), (44, 0)], name="Oda"),
        opening(23, W6, 1.0, 0.8, "door", head=2.0, number="K9"),
        opening(24, W6, 3.9, 1.0, "door", head=2.0, broken="offWall"),
    ]


def setup(w2_height=None, s1_height=None, group=None):
    ents = entities(w2_height, s1_height)
    layers = [layer("duvar", "Duvar"), layer("kapi", "Kapı"), layer("mahal", "Mahal"), layer("mobilya", "Mobilya"),
              layer("cizim", "Çizim"), layer("kilitli", "Kilitli", locked=True)]
    if group is not None:
        layers.append(group)
    return {
        "format": "kentos.document", "version": 2, "name": "Oda iç görünüşleri",
        "uids": [UID(e["id"]) for e in ents],
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": PLOT, "workspace": "cad",
                     "arch": {"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
                                         {"id": "kat1", "name": "1. kat", "elevation": 3.0}]}},
        "origin": {"x": 0, "y": 0},
        "layers": layers,
        "activeLayer": "cizim",
        "styles": {"items": [], "categories": []},
        "blocks": [{"id": BLOCK, "name": "Dolap", "base": P(0, 0),
                    "entities": [{"kind": "line", "id": 1, "layerId": "", "attrs": {},
                                  "a": P(0, 0), "b": P(0.6, 0.6)}]}],
        "entities": ents,
    }


# ── The model a case changes ─────────────────────────────────────────────────────────────────────────────────────


class Model:
    def __init__(self, ents, layers):
        self.ents = {e["id"]: copy.deepcopy(e) for e in ents}
        self.uids = {e["id"]: UID(e["id"]) for e in ents}
        self.next = max(self.ents) + 1
        self.layers = copy.deepcopy(layers)  # [(id, name, kind, parent)]
        self.records = {}  # layer id -> record

    def by_uid(self, u):
        for i, x in self.uids.items():
            if x == u and i in self.ents:
                return i
        return None

    def ordered(self):
        return [self.ents[i] for i in sorted(self.ents)]


def letter(k):
    n, out = k + 1, ""
    while n > 0:
        out = chr(ord("A") + (n - 1) % 26) + out
        n = (n - 1) // 26
    return out


def label_of(s):
    words = [w.strip() for w in (s.get("number"), s.get("name")) if w and w.strip()]
    return " ".join(words) if words else "Mahal"


def inside(rings, p):
    odd = False
    for r in rings:
        n = len(r)
        for i in range(n):
            a, b = r[i], r[(i + 1) % n]
            if (a["y"] > p["y"]) != (b["y"] > p["y"]) and \
                    p["x"] < a["x"] + (p["y"] - a["y"]) * (b["x"] - a["x"]) / (b["y"] - a["y"]):
                odd = not odd
    return odd


def ent(layer_id, v):
    out = {"kind": v.pop("kind"), "layerId": layer_id, "attrs": {}}
    out.update(v)
    return out


class Refused(Exception):
    def __init__(self, code, message, path):
        self.code, self.message, self.path = code, message, path


def fail(code, message, path):
    raise Refused(code, message, path)


def draw(m, uid, sides, at, gap, layer_id):
    """(sides, items [(role, entity)], leftOut, warnings, at, gap, label) as docs/adr/0799 §2 draws them."""
    i = m.by_uid(uid)
    if i is None:
        fail("entity_not_found", f"“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir.",
             "operation.space")
    s = m.ents[i]
    if s["kind"] != "space":
        fail("not_a_space", "Nesne mahal değil; iç görünüş bir mahalden üretilir.", "operation.space")
    if s.get("method", "net") != "net":
        fail("elevation_space_not_net",
             "Mahalin sınırı duvarların iç yüzleri değil (eksen ya da brüt); iç görünüş yalnız net sınırlı mahalden üretilir. Mahalin yöntemini Net yapın.",
             "operation.space")
    level = (s.get("arch") or {}).get("level")
    level_height = next((l.get("height") for l in LEVELS if l["id"] == level), None) if level else None
    edges = []
    for part in s["parts"]:
        r = part["outer"]["pts"]
        n = len(r)
        twice = 0.0
        for k in range(n):
            p, q = r[k], r[(k + 1) % n]
            twice += p["x"] * q["y"] - q["x"] * p["y"]
        bl = part["outer"].get("bulges") or []
        for k in range(n):
            edges.append((r[k], r[(k + 1) % n], bl[k] if k < len(bl) else 0.0, twice > 0.0))
    if not sides:
        chosen = list(range(len(edges)))
    else:
        chosen = []
        for j, k in enumerate(sides):
            if k >= len(edges):
                fail("elevation_no_sides", f"Mahalin {k} numaralı yanı yok; yanlar 0'dan {len(edges) - 1} numaraya kadardır.",
                     f"operation.sides[{j}]")
            if k in chosen:
                fail("invalid_input", f"{k} numaralı yan iki kez verildi; her yanı bir kez verin.", f"operation.sides[{j}]")
            chosen.append(k)
        chosen.sort()
    if all(edges[k][2] != 0.0 for k in chosen):
        fail("elevation_no_sides",
             "Çizilecek düz bir yan yok: istenen yanların hepsi yaylı. Yaylı duvarın iç görünüşü açılmaz.",
             "operation.space" if not sides else "operation.sides")
    if gap is not None and not (0.0 < gap <= 100.0):
        fail("invalid_input", "Yanların arası sıfırdan büyük, en çok 100 m olmalı.", "operation.gap")
    gap = 1.0 if gap is None else gap
    if at is None:
        pts = [p for part in s["parts"] for p in part["outer"]["pts"]]
        at = P(max(p["x"] for p in pts) + 3.0, min(p["y"] for p in pts))
    label = label_of(s)
    walls = [(m.uids[e["id"]], e) for e in m.ordered()
             if e["kind"] == "wall" and (e.get("arch") or {}).get("level") == level]
    openings = [(m.uids[e["id"]], e) for e in m.ordered() if e["kind"] == "opening"]
    out_sides, items, left = [], [], []
    curved = open_ = unknown = headless = 0
    ox, oy = at["x"], at["y"]
    for k in chosen:
        a, b, bulge, ccw = edges[k]
        if bulge != 0.0:
            curved += 1
            left.append({"why": "curvedSide", "side": k})
            continue
        dx, dy = b["x"] - a["x"], b["y"] - a["y"]
        ln = math.sqrt(dx * dx + dy * dy)
        ux, uy = dx / ln, dy / ln

        def xof(p):
            if ccw:
                return (b["x"] - p["x"]) * ux + (b["y"] - p["y"]) * uy
            return (p["x"] - a["x"]) * ux + (p["y"] - a["y"]) * uy

        def w(x, y):
            return P(ox + x, oy + y)

        fs = []
        for wu, wl in walls:
            pts = wl["pts"]
            n = len(pts)
            count = n if wl.get("closed") else n - 1
            lf, rf = faces(wl)
            for j in range(count):
                bl = wl.get("bulges") or []
                if (bl[j] if j < len(bl) else 0.0) != 0.0:
                    continue
                p, q = pts[j], pts[(j + 1) % n]
                vx, vy = q["x"] - p["x"], q["y"] - p["y"]
                vl = math.sqrt(vx * vx + vy * vy)
                vx, vy = vx / vl, vy / vl
                if abs(ux * vy - uy * vx) > 1e-6:
                    continue
                e = vx * (a["y"] - p["y"]) - vy * (a["x"] - p["x"])
                if abs(e - lf) <= 1e-3:
                    lft = True
                elif abs(e - rf) <= 1e-3:
                    lft = False
                else:
                    continue
                x0, x1 = xof(p), xof(q)
                lo, hi = max(min(x0, x1), 0.0), min(max(x0, x1), ln)
                if hi - lo <= 1e-3:
                    continue
                hh = wl.get("height", level_height)
                fs.append({"wall": wu, "seg": j, "left": lft, "lo": lo, "hi": hi, "h": hh})
        side_walls = []
        for f in fs:
            if f["wall"] not in side_walls:
                side_walls.append(f["wall"])
        height = s.get("height")
        if height is None:
            hs = [f["h"] for f in fs if f["h"] is not None]
            height = min(hs) if hs else None
        gaps, reach = [], 0.0
        for lo, hi in sorted((f["lo"], f["hi"]) for f in fs):
            if lo - reach > 1e-3:
                gaps.append([reach, lo])
            reach = max(reach, hi)
        if ln - reach > 1e-3:
            gaps.append([reach, ln])
        if gaps:
            open_ += 1
        its = []
        if height is not None:
            its.append((f"{k}/frame", {"kind": "polyline", "pts": [w(0, 0), w(ln, 0), w(ln, height), w(0, height)],
                                       "closed": True}))
        else:
            unknown += 1
            its.append((f"{k}/floor", {"kind": "line", "a": w(0, 0), "b": w(ln, 0)}))
            pass
        if height is not None:
            for f in fs:
                if f["h"] is not None and f["h"] < height - 1e-9:
                    its.append((f"{k}/top/{f['wall']}/{f['seg']}",
                                {"kind": "line", "a": w(f["lo"], f["h"]), "b": w(f["hi"], f["h"])}))
        seen = []
        for ou, o in openings:
            if o["host"] not in side_walls:
                continue
            if o.get("broken"):
                left.append({"why": "brokenOpening", "side": k, "uid": ou})
                continue
            if o["form"] == "flue":
                left.append({"why": "flue", "side": k, "uid": ou})
                continue
            pl = o["place"]

            def on(p):
                return abs(ux * (p["y"] - a["y"]) - uy * (p["x"] - a["x"])) <= 1e-3

            found = None
            for lft in (True, False):
                if not any(f["wall"] == o["host"] and f["left"] == lft for f in fs):
                    continue
                p, q = (pl[0], pl[2]) if lft else (pl[1], pl[3])
                if on(p) and on(q):
                    found = (lft, p, q)
                    break
            if found is None:
                continue
            lft, p, q = found
            if o["form"] == "niche" and (o.get("side", "left") == "left") != lft:
                continue
            xa, xb = xof(p), xof(q)
            x0, x1 = max(min(xa, xb), 0.0), min(max(xa, xb), ln)
            if x1 - x0 <= 1e-9:
                continue
            if o.get("head") is None:
                headless += 1
                left.append({"why": "headUnknown", "side": k, "uid": ou})
                if height is None:
                    continue
            seen.append({"x0": x0, "x1": x1, "uid": ou, "sill": o.get("sill", 0.0), "head": o.get("head"),
                         "number": o.get("number") if (o.get("number") or "").strip() else None})
        seen.sort(key=lambda o: (o["x0"], o["uid"]))
        for o in seen:
            if o["head"] is not None:
                shape = {"kind": "polyline", "pts": [w(o["x0"], o["sill"]), w(o["x1"], o["sill"]),
                                                     w(o["x1"], o["head"]), w(o["x0"], o["head"])], "closed": True}
            else:
                top = height if height is not None else o["sill"]
                shape = {"kind": "polyline", "lineType": "dashed",
                         "pts": [w(o["x0"], top), w(o["x0"], o["sill"]), w(o["x1"], o["sill"]), w(o["x1"], top)]}
            its.append((f"{k}/opening/{o['uid']}", shape))
        for o in seen:
            if o["number"] and o["head"] is not None:
                its.append((f"{k}/mark/{o['uid']}",
                            {"kind": "text", "p": w((o["x0"] + o["x1"]) / 2.0, (o["sill"] + o["head"]) / 2.0),
                             "text": o["number"], "height": H, "rotation": 0, "align": "middleCenter"}))
        its.append((f"{k}/title", {"kind": "text", "p": w(ln / 2.0, -4.0 * H), "text": f"{label} — {letter(k)}",
                                   "height": H, "rotation": 0, "align": "topCenter"}))
        its.append((f"{k}/width", {"kind": "dimension", "a": w(0, 0), "b": w(ln, 0), "offset": -2.0 * H, "height": H,
                                   "style": "aligned"}))
        if height is not None:
            its.append((f"{k}/height", {"kind": "dimension", "a": w(ln, 0), "b": w(ln, height), "offset": -2.0 * H,
                                        "height": H, "style": "aligned"}))
        side = {"side": k, "letter": letter(k), "length": ln, "origin": P(ox, oy)}
        if height is not None:
            side["height"] = height
        if side_walls:
            side["walls"] = side_walls
        if seen:
            side["openings"] = [o["uid"] for o in seen]
        if gaps:
            side["open"] = gaps
        out_sides.append(side)
        items.extend((role, ent(layer_id, v)) for role, v in its)
        ox = ox + ln + gap
    for part in s["parts"]:
        for _ in part.get("holes", []):
            left.append({"why": "hole"})
    rings = [part["outer"]["pts"] for part in s["parts"]]
    for e in m.ordered():
        kind = e["kind"]
        if kind not in ("column", "stair", "ramp", "insert"):
            continue
        lvl = (e.get("arch") or {}).get("level")
        pts = [e["at"]] if kind == "column" else e["pts"] if kind in ("stair", "ramp") else [e["p"]]
        if lvl != level or not any(inside(rings, p) for p in pts):
            continue
        left.append({"why": {"column": "column", "stair": "stair", "ramp": "ramp", "insert": "fixture"}[kind],
                     "uid": m.uids[e["id"]]})
    warnings = []

    def warn(code, message):
        warnings.append({"code": code, "message": message})

    if curved:
        warn("elevation_curved_side", f"{curved} yaylı yan çizilmedi; yaylı duvarın iç görünüşü açılmaz, yalnız düz yanlar çizildi.")
    if open_:
        warn("elevation_side_open", f"{open_} yanın bir kısmında duvar yüzü yok (ayırıcı ya da boşluk); orada duvar çizilmedi.")
    if unknown:
        warn("elevation_height_unknown",
             f"{unknown} yanın yüksekliği bilinmiyor (mahalin, duvarın ve katın yüksekliği yok); yalnız döşeme çizgisi çizildi.")
    if headless:
        warn("elevation_head_unknown", f"{headless} açıklığın üst kotu yok; pervazları kesikli çizildi, üstleri çizilmedi.")
    nd = sum(1 for x in left if x["why"] not in ("curvedSide", "headUnknown"))
    if nd:
        warn("elevation_not_drawn",
             f"Odadaki {nd} şey çizilmedi (kolon, merdiven, rampa, donatı, baca, boşluk, kopuk açıklık); leftOut listesinde.")
    return {"sides": out_sides, "items": items, "left": left, "warnings": warnings, "at": at, "gap": gap,
            "label": label}


LEVELS = [{"id": "zemin", "height": 3.0}, {"id": "kat1"}]


def bare(e):
    return {k: v for k, v in e.items() if k != "id"}


# ── The commands as the ADR says ─────────────────────────────────────────────────────────────────────────────────


def unique_name(m, base):
    names = {l[1] for l in m.layers}
    if base not in names:
        return base
    k = 2
    while f"{base} {k}" in names:
        k += 1
    return f"{base} {k}"


def derive(m, space, sides=(), at=None, gap=None, name=None):
    """The output and warnings of a derive, the model written."""
    if name is not None and (not name.strip() or name.strip() != name or len(name) > 200):
        fail("invalid_input",
             "Katmanın adı boş, baş ya da sonda boşluklu, 200 karakterden uzun ya da denetim karakterli olamaz.",
             "operation.name")
    probe = draw(m, space, list(sides), at, gap, "")
    group = next((l for l in m.layers if l[2] == "group" and l[1] == GROUP and l[3] is None), None)
    if group is not None and group[4]:
        fail("layer_locked", f"“{GROUP}” grubu kilitli; iç görünüş katmanı açılamaz. Kilidi Katmanlar panelinden açın.",
             "operation.space")
    ids = {l[0] for l in m.layers}
    layer_id = next(f"ic-gorunus-{k}" for k in range(1, 1000) if f"ic-gorunus-{k}" not in ids)
    lname = unique_name(m, name if name is not None else f"{probe['label']} iç görünüşü")
    d = draw(m, space, list(sides), at, gap, layer_id)
    return group, layer_id, lname, d


def plan_derive(m, space, sides=(), at=None, gap=None, name=None):
    _, layer_id, lname, d = derive(m, space, sides, at, gap, name)
    out = {"layer": layer_id, "name": lname, "sides": d["sides"], "drawn": [dict(e, id=0) for _, e in d["items"]],
           "made": len(d["items"]), "changed": 0, "removed": 0, "revision": "$current"}
    if d["left"]:
        out["leftOut"] = d["left"]
    return out, d["warnings"]


def exec_derive(m, space, sides=(), at=None, gap=None, name=None):
    group, layer_id, lname, d = derive(m, space, sides, at, gap, name)
    if group is None:
        m.layers.append(("ic-gorunusler", GROUP, "group", None, False))
        gid = "ic-gorunusler"
    else:
        gid = group[0]
    m.layers.append((layer_id, lname, "layer", gid, False))
    drawn, members = [], {}
    for role, e in d["items"]:
        i = m.next
        m.next += 1
        e = dict(e, id=i)
        m.ents[i] = e
        m.uids[i] = f"$uidOf:{i}"
        members[role] = i
        drawn.append(e)
    m.records[layer_id] = {"space": space, "sides": list(sides), "at": d["at"], "gap": d["gap"], "members": members,
                           "items": [(r, bare(e)) for r, e in d["items"]], "name": lname}
    out = {"layer": layer_id, "name": lname, "sides": d["sides"], "drawn": drawn, "made": len(drawn), "changed": 0,
           "removed": 0, "revision": "$current"}
    if d["left"]:
        out["leftOut"] = d["left"]
    return out, d["warnings"]


def standing(m, layer_id):
    r = m.records[layer_id]
    # The record keeps its members by role in the roles' order.
    missing = [i for _, i in sorted(r["members"].items()) if i not in m.ents]
    try:
        d = draw(m, r["space"], r["sides"], r["at"], r["gap"], layer_id)
    except Refused:
        return "gone", missing, [], None
    same = [(ro, bare(e)) for ro, e in d["items"]] == r["items"]
    edited = []
    if same:
        for role, want in d["items"]:
            i = r["members"].get(role)
            if i is None or i not in m.ents:
                continue
            if not close(bare(m.ents[i]), bare(want)):
                edited.append(i)
    state = "current" if same and not missing else "stale"
    return state, missing, edited, d


def close(a, b):
    if isinstance(a, dict) and isinstance(b, dict):
        return a.keys() == b.keys() and all(close(a[k], b[k]) for k in a)
    if isinstance(a, list) and isinstance(b, list):
        return len(a) == len(b) and all(close(x, y) for x, y in zip(a, b))
    if isinstance(a, (int, float)) and isinstance(b, (int, float)) and not isinstance(a, bool):
        return abs(a - b) <= 1e-12
    return a == b


def elevation_layer(m, layer_id):
    node = next((l for l in m.layers if l[0] == layer_id), None)
    if node is None:
        fail("layer_not_found", f"“{layer_id}” kimlikli katman çizimde yok. Bir iç görünüş katmanının kimliğini verin.",
             "operation.layer")
    if layer_id not in m.records:
        fail("elevation_not_found", f"“{node[1]}” katmanı bir oda iç görünüşü değil (türetim kaydı yok).",
             "operation.layer")
    return node


def refresh(m, layer_id, write):
    node = elevation_layer(m, layer_id)
    r = m.records[layer_id]
    try:
        d = draw(m, r["space"], r["sides"], r["at"], r["gap"], layer_id)
    except Refused as e:
        if e.code in ("entity_not_found", "not_a_space"):
            fail("elevation_source_gone",
                 f"“{node[1]}” iç görünüşünün mahali çizimde yok; yeniden üretilemez. Bağı koparıp çizimi koruyabilir ya da katmanı silebilirsiniz.",
                 "operation.layer")
        raise
    if node[4]:
        fail("layer_locked",
             f"“{node[1]}” iç görünüşünün katmanı ya da bir nesnesinin katmanı kilitli. Kilidi Katmanlar panelinden açın.",
             "operation.layer")
    state, missing, edited, _ = standing(m, layer_id)
    add, update, remove = [], [], []
    for role, want in d["items"]:
        i = r["members"].get(role)
        if i is not None and i in m.ents:
            if not close(bare(m.ents[i]), bare(want)):
                update.append((i, want))
        else:
            add.append((role, want, i))
    roles = [ro for ro, _ in d["items"]]
    for role, i in r["members"].items():
        if role not in roles and i in m.ents:
            remove.append(i)
    same = [(ro, bare(e)) for ro, e in d["items"]] == r["items"]
    if same and not add and not update and not remove:
        fail("nothing_to_set", f"“{node[1]}” iç görünüşü güncel; yeniden üretilecek bir şey yok.", "operation.layer")
    warnings = list(d["warnings"])
    if edited:
        warnings.append({"code": "elevation_edited_overwritten",
                         "message": f"İç görünüşün elle değiştirilmiş {len(edited)} nesnesi modelin çizdiğiyle yeniden yazıldı.",
                         "path": "operation.layer"})
    if not write:
        drawn = []
        for role, e in d["items"]:
            i = r["members"].get(role)
            drawn.append(dict(e, id=i if (i is not None and i in m.ents) else 0))
    else:
        for i in remove:
            del m.ents[i]
        for i, want in update:
            m.ents[i] = dict(want, id=i)
        keep = [x for x in add if x[2] is not None]
        fresh = [x for x in add if x[2] is None]
        made = {}
        for role, want, old in keep + fresh:
            i = m.next
            m.next += 1
            m.ents[i] = dict(want, id=i)
            m.uids[i] = m.uids[old] if old is not None else f"$uidOf:{i}"
            made[role] = i
        members = {}
        for role, _ in d["items"]:
            members[role] = made[role] if role in made else r["members"][role]
        r["members"] = members
        r["items"] = [(ro, bare(e)) for ro, e in d["items"]]
        drawn = [m.ents[members[role]] for role, _ in d["items"]]
    out = {"layer": layer_id, "name": node[1], "sides": d["sides"], "drawn": drawn, "made": len(add),
           "changed": len(update), "removed": len(remove), "revision": "$current"}
    if d["left"]:
        out["leftOut"] = d["left"]
    return out, warnings


# ── Steps ────────────────────────────────────────────────────────────────────────────────────────────────────────


def strip(out):
    """An output as the contract writes it: empty lists left out."""
    return {k: v for k, v in out.items() if not (isinstance(v, list) and not v)}


def ok(out, warnings):
    return {"status": "completed", "output": strip(out), "warnings": warnings}


def failed(e):
    return {"status": "failed", "error": {"code": e.code, "message": e.message, "path": e.path}}


def edit_step(op, operation, result, **extra):
    # A copy: the model's objects change after the step and the step says what was then.
    s = {"op": op, "command": "cad.elevations.edit", "input": {"operation": operation},
         "result": copy.deepcopy(result), "within": WITHIN}
    s.update(extra)
    return s


def node_expect(m, layer_id):
    r = m.records[layer_id]
    params = {"space": r["space"], "at": r["at"], "gap": r["gap"], "members": "$any", "digest": "$any"}
    if r["sides"]:
        params["sides"] = r["sides"]
    return {"parent": next(l[3] for l in m.layers if l[0] == layer_id), "type": "layer", "name": r["name"],
            "derivation": {"tool": "cad.elevations.edit", "label": STEP, "toolVersion": 1, "parameters": params,
                           "documentRevision": "$any", "engine": "$any", "at": "$any"}}


BASE_LAYERS = [("duvar", "Duvar", "layer", None, False), ("kapi", "Kapı", "layer", None, False),
               ("mahal", "Mahal", "layer", None, False), ("mobilya", "Mobilya", "layer", None, False),
               ("cizim", "Çizim", "layer", None, False), ("kilitli", "Kilitli", "layer", None, True)]


def model(**kw):
    return Model(entities(kw.get("w2_height"), kw.get("s1_height")), BASE_LAYERS)


def derive_step(m, op="execute", space=5, **kw):
    operation = {"kind": "derive", "space": UID(space)}
    for k in ("sides", "at", "gap", "name"):
        if kw.get(k) is not None:
            operation[k] = kw[k]
    args = dict(sides=kw.get("sides") or (), at=kw.get("at"), gap=kw.get("gap"), name=kw.get("name"))
    try:
        if op == "execute":
            out, w = exec_derive(m, UID(space), **args)
            step = edit_step("execute", operation, ok(out, w))
            step["expect"] = {"revision": "changed", "nodes": {out["layer"]: node_expect(m, out["layer"])}}
            return step
        out, w = plan_derive(m, UID(space), **args)
        if op == "validate":
            return edit_step("validate", operation, {"status": "completed", "output": None, "warnings": w},
                             expect={"revision": "same"})
        return edit_step("plan", operation, ok(out, w), expect={"revision": "same"})
    except Refused as e:
        return edit_step(op, operation, failed(e), expect={"revision": "same"})


def refresh_step(m, layer_id, op="execute"):
    operation = {"kind": "refresh", "layer": layer_id}
    try:
        out, w = refresh(m, layer_id, op == "execute")
        step = edit_step(op, operation, ok(out, w))
        step["expect"] = {"revision": "changed" if op == "execute" else "same"}
        return step
    except Refused as e:
        return edit_step(op, operation, failed(e), expect={"revision": "same"})


def release_step(m, layer_id):
    operation = {"kind": "release", "layer": layer_id}
    try:
        node = elevation_layer(m, layer_id)
        del m.records[layer_id]
        out = {"layer": layer_id, "name": node[1], "made": 0, "changed": 0, "removed": 0, "revision": "$current"}
        return edit_step("execute", operation, {"status": "completed", "output": out, "warnings": []},
                         expect={"revision": "changed", "nodes": {layer_id: {"derivation": None}}})
    except Refused as e:
        return edit_step("execute", operation, failed(e), expect={"revision": "same"})


def move_opening(m, slot, at):
    o = m.ents[slot]
    host = m.ents[m.by_uid(o["host"])]
    o["at"] = at
    o["place"] = place(host, at, o["width"])
    return {"op": "execute", "command": "cad.openings.edit",
            "input": {"uids": [UID(slot)], "operation": {"kind": "move", "at": at}}, "result": ANY}


def delete(m, slot):
    """The steps deleting an object: an object the case made is captured first, its slot gone after."""
    uid = m.uids[slot]
    steps = []
    if uid.startswith("$uidOf:"):
        steps.append({"op": "captureUid", "id": slot, "as": f"n{slot}"})
        uid = f"$uid:n{slot}"
        m.uids[slot] = uid
    del m.ents[slot]
    return steps + [{"op": "execute", "command": "cad.entities.delete", "input": {"uids": [uid]}, "result": ANY}]


def move_entity(m, slot, dx, dy):
    e = m.ents[slot]

    def shift(p):
        return P(p["x"] + dx, p["y"] + dy)

    for k in ("p", "a", "b"):
        if k in e:
            e[k] = shift(e[k])
    if "pts" in e:
        e["pts"] = [shift(p) for p in e["pts"]]
    return {"op": "execute", "command": "cad.entities.transform",
            "input": {"uids": [m.uids[slot]], "transform": {"kind": "move", "dx": dx, "dy": dy}}, "result": ANY}


def set_height(m, slot, height):
    m.ents[slot]["height"] = height
    return {"op": "execute", "command": "cad.spaces.edit",
            "input": {"uids": [UID(slot)], "operation": {"kind": "set", "height": height}}, "result": ANY}


def lock(m, layer_id):
    m.layers = [(a, b, c, d, True if a == layer_id else e) for a, b, c, d, e in m.layers]
    return {"op": "execute", "command": "cad.layers.set",
            "input": {"operation": "locked", "layer": layer_id, "value": True}, "result": ANY}


def undo(returns=STEP):
    return {"op": "undo", "returns": returns}


# ── cad.elevations.edit ──────────────────────────────────────────────────────────────────────────────────────────


def edit_cases():
    out = []

    def case(name, steps, note=None, setup_=None):
        c = {"name": name}
        if note:
            c["note"] = note
        if setup_:
            c["setup"] = setup_
        c["steps"] = steps
        out.append(c)

    m = model()
    first = derive_step(m)
    first["expect"]["ids"] = sorted(m.ents)
    case("Salonun dört yanı: kapı, pencere, üstü bilinmeyen pencere, odadan görünen niş; baca, kolon ve dolap çizilmez",
         [first, undo(), {"op": "redo", "returns": STEP}])
    m = model()
    case("Plan: yalnız A yanı, verilen başlangıç ve aralık; çizim değişmez",
         [derive_step(m, "plan", sides=[0], at=P(20, -10), gap=0.5)])
    m = model()
    case("Doğrulama: uyarılar planınkiyle aynı", [derive_step(m, "validate")])
    m = model()
    case("Katmanın adı verilir", [derive_step(m, name="Salon görünüşleri")])
    m = model()
    case("İkinci üretim: aynı grupta yeni katman, ad numaralanır", [derive_step(m), derive_step(m, sides=[1, 3])])
    m = model()
    case("Duvarsız, yüksekliksiz mutfak: döşeme çizgileri, açık yanlar, yükseklik bilinmez", [derive_step(m, space=15)])
    m = model()
    case("Yaylı yan çizilmez, düzler çizilir", [derive_step(m, space=16)])
    m = model()
    case("Ret: istenen tek yan yaylı", [derive_step(m, space=16, sides=[1])])
    m = model()
    case("Ret: eksenden sınırlı (brüt) mahal", [derive_step(m, space=17)])
    m = model()
    case("Boşluklu hol ve merdiveni listelenir, dışarıdaki rampa listelenmez", [derive_step(m, space=18)])
    m = model()
    case("Saat yönünde halka: duvarın kendi yüksekliği, kapısı ve duvarı aşan kopuk açıklık",
         [derive_step(m, space=22, sides=[3])])
    m = model()
    case("Saat yönünde halkanın bütün yanları: duvarsız yanların yüksekliği bilinmez", [derive_step(m, space=22)])
    m = model(w2_height=2.5, s1_height=2.8)
    case("Mahalin yüksekliği duvarınkinden yüksek: alçak duvarın üstü çizilir",
         [derive_step(m, sides=[1])], setup_=setup(w2_height=2.5, s1_height=2.8))
    for name, operation, code, message, path in [
        ("Ret: geçersiz kimlik", {"kind": "derive", "space": "mahal-1"}, "invalid_uid",
         "“mahal-1” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir.", "operation.space"),
        ("Ret: çizimde olmayan mahal", {"kind": "derive", "space": UID(99)}, "entity_not_found",
         f"“{UID(99)}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir.", "operation.space"),
        ("Ret: duvar mahal değil", {"kind": "derive", "space": UID(1)}, "not_a_space",
         "Nesne mahal değil; iç görünüş bir mahalden üretilir.", "operation.space"),
        ("Ret: mahalde olmayan yan", {"kind": "derive", "space": UID(5), "sides": [9]}, "elevation_no_sides",
         "Mahalin 9 numaralı yanı yok; yanlar 0'dan 3 numaraya kadardır.", "operation.sides[0]"),
        ("Ret: aynı yan iki kez", {"kind": "derive", "space": UID(5), "sides": [0, 0]}, "invalid_input",
         "0 numaralı yan iki kez verildi; her yanı bir kez verin.", "operation.sides[1]"),
        ("Ret: sıfır aralık", {"kind": "derive", "space": UID(5), "gap": 0}, "invalid_input",
         "Yanların arası sıfırdan büyük, en çok 100 m olmalı.", "operation.gap"),
        ("Ret: boşluklu katman adı", {"kind": "derive", "space": UID(5), "name": " Salon"}, "invalid_input",
         "Katmanın adı boş, baş ya da sonda boşluklu, 200 karakterden uzun ya da denetim karakterli olamaz.",
         "operation.name"),
    ]:
        case(name, [edit_step("execute", operation,
                              {"status": "failed", "error": {"code": code, "message": message, "path": path}},
                              expect={"revision": "same"})])
    case("Ret: sonsuz aralık", [edit_step("execute", {"kind": "derive", "space": UID(5), "gap": 1.0},
                                          {"status": "failed", "error": {"code": "not_finite",
                                                                         "message": "Yanların arası sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.",
                                                                         "path": "operation.gap"}},
                                          nonFinite={"operation.gap": "NaN"}, expect={"revision": "same"})])
    s = edit_step("execute", {"kind": "derive", "space": UID(5)},
                  {"status": "failed", "error": {"code": "invalid_revision",
                                                 "message": "Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.",
                                                 "path": "expectedRevision"}}, expect={"revision": "same"})
    s["input"]["expectedRevision"] = "abc"
    case("Ret: geçersiz beklenen sürüm", [s])
    locked_group = {"id": "ic-gorunusler", "name": GROUP, "type": "group", "visible": True, "locked": True,
                    "expanded": True, "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25},
                    "children": []}
    case("Ret: İç görünüşler grubu kilitli", [edit_step("execute", {"kind": "derive", "space": UID(5)}, {
        "status": "failed", "error": {"code": "layer_locked",
                                      "message": f"“{GROUP}” grubu kilitli; iç görünüş katmanı açılamaz. Kilidi Katmanlar panelinden açın.",
                                      "path": "operation.space"}}, expect={"revision": "same"})],
         setup_=setup(group=locked_group))
    # Refresh.
    m = model()
    steps = [derive_step(m), move_opening(m, 6, 3.0), refresh_step(m, "ic-gorunus-1", "plan"),
             refresh_step(m, "ic-gorunus-1"), undo()]
    case("Kapı taşınınca yeniden üret: kapı ve numarası aynı kimlikle yeniden yazılır", steps)
    m = model()
    case("Değişiklik yoksa yeniden üretilecek bir şey yok", [derive_step(m), refresh_step(m, "ic-gorunus-1")])
    m = model()
    st = [derive_step(m)]
    title = m.records["ic-gorunus-1"]["members"]["0/title"]
    st += delete(m, title) + [refresh_step(m, "ic-gorunus-1")]
    case("Silinen başlık aynı kimlikle yeniden yapılır", st)
    m = model()
    st = [derive_step(m)]
    title = m.records["ic-gorunus-1"]["members"]["1/title"]
    st += [move_entity(m, title, 0.5, 0.0), refresh_step(m, "ic-gorunus-1")]
    case("Elle taşınan başlık yeniden yazılır ve söylenir", st)
    m = model()
    st = [derive_step(m), set_height(m, 5, 2.6), refresh_step(m, "ic-gorunus-1")]
    case("Mahalin yüksekliği değişince çerçeveler, ölçüler ve kesikli pervazlar izler", st)
    m = model()
    st = [derive_step(m)] + delete(m, 6) + [refresh_step(m, "ic-gorunus-1")]
    case("Silinen kapının çizimi ve numarası kalkar", st)
    m = model()
    st = [derive_step(m)] + delete(m, 5) + [refresh_step(m, "ic-gorunus-1")]
    case("Ret: mahali silinmiş iç görünüş yeniden üretilemez", st)
    m = model()
    case("Ret: katman yok", [refresh_step(m, "yok")])
    m = model()
    case("Ret: türetim kaydı olmayan katman", [refresh_step(m, "duvar")])
    m = model()
    st = [derive_step(m), move_opening(m, 6, 3.0), lock(m, "ic-gorunus-1"), refresh_step(m, "ic-gorunus-1")]
    case("Ret: kilitli iç görünüş katmanı", st)
    # Release.
    m = model()
    st = [derive_step(m), release_step(m, "ic-gorunus-1"), undo()]
    case("Bağı kopar: kayıt kalkar, çizim kalır; geri alınır", st)
    m = model()
    case("Ret: bağı koparılacak iç görünüş yok", [release_step(m, "mahal")])
    return out


# ── cad.elevations.query ─────────────────────────────────────────────────────────────────────────────────────────


def query_step(m, input, op="execute"):
    try:
        rows = query(m, input)
        return {"op": op, "command": "cad.elevations.query", "input": input,
                "result": {"status": "completed", "output": {"elevations": rows, "revision": "$current"},
                           "warnings": []}, "expect": {"revision": "same"}}
    except Refused as e:
        return {"op": op, "command": "cad.elevations.query", "input": input, "result": failed(e),
                "expect": {"revision": "same"}}


def row(m, layer_id):
    r = m.records[layer_id]
    state, missing, edited, _ = standing(m, layer_id)
    si = m.by_uid(r["space"])
    lab = None
    if si is not None and m.ents[si]["kind"] == "space":
        x = label_of(m.ents[si])
        lab = x if x != "Mahal" else None
    out = {"layer": layer_id, "name": r["name"], "space": r["space"], "state": state,
           "members": len(r["members"]), "derivedAt": "$any"}
    if lab:
        out["spaceLabel"] = lab
    if r["sides"]:
        out["sides"] = r["sides"]
    if missing:
        out["missing"] = [m.uids[i] for i in missing]
    if edited:
        out["edited"] = [m.uids[i] for i in edited]
    return out


def tree_order(m):
    """Layers in the tree's order: the base layers, then the group and its layers."""
    out = []
    for l in m.layers:
        if l[3] is None:
            out.append(l[0])
            out.extend(c[0] for c in m.layers if c[3] == l[0])
    return out


def query(m, input):
    only = None
    if "layer" in input:
        l = next((x for x in m.layers if x[0] == input["layer"]), None)
        if l is None:
            fail("layer_not_found", f"“{input['layer']}” kimlikli katman çizimde yok.", "layer")
        if l[0] not in m.records:
            fail("elevation_not_found", f"“{l[1]}” katmanı bir oda iç görünüşü değil (türetim kaydı yok).", "layer")
        only = l[0]
    if "uid" in input:
        u = input["uid"]
        # A placeholder ($uidOf:30) stands for a valid id of the drawing.
        if not u.startswith("$") and not (len(u) == 36 and all(c in "0123456789abcdef-" for c in u)):
            fail("invalid_uid", f"“{u}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir.", "uid")
        i = m.by_uid(u)
        if i is None:
            fail("entity_not_found", f"“{u}” kimlikli nesne çizimde yok.", "uid")
        owner = next((lid for lid in tree_order(m) if lid in m.records
                      and i in m.records[lid]["members"].values()), None)
        if owner is None:
            fail("elevation_not_found", "Nesne bir oda iç görünüşünün nesnesi değil.", "uid")
        if only is not None and only != owner:
            fail("elevation_not_found", "Nesne verilen katmanın iç görünüşünün nesnesi değil.", "uid")
        only = owner
    if only is not None:
        return [row(m, only)]
    return [row(m, lid) for lid in tree_order(m) if lid in m.records]


def query_cases():
    out = []

    def case(name, steps):
        out.append({"name": name, "steps": steps})

    def made(m, *args, **kw):
        s = derive_step(m, *args, **kw)
        s["result"] = ANY
        s.pop("expect", None)
        return s

    m = model()
    case("İç görünüş yokken liste boş", [query_step(m, {})])
    m = model()
    case("Üretilen iç görünüş güncel", [made(m), query_step(m, {})])
    m = model()
    case("Plan aynı cevabı verir", [made(m), query_step(m, {}, "plan")])
    m = model()
    case("Kapı taşınınca eski", [made(m), move_opening(m, 6, 3.0), query_step(m, {})])
    m = model()
    case("Kapının taşınması geri alınınca yine güncel",
         [made(m), move_opening(m, 6, 3.0), undo("Açıklığı düzenle")] + [query_step(model_after_undo(m), {})])
    m = model()
    st = [made(m)]
    t = m.records["ic-gorunus-1"]["members"]["0/title"]
    st += delete(m, t) + [query_step(m, {})]
    case("Silinen nesne eksik sayılır, iç görünüş eski", st)
    m = model()
    st = [made(m)]
    t = m.records["ic-gorunus-1"]["members"]["2/title"]
    st += [move_entity(m, t, 0.0, 0.3), query_step(m, {})]
    case("Elle taşınan nesne: model aynıyken güncel, elle değişmiş sayılır", st)
    m = model()
    case("Mahal silinince kaynağı yok", [made(m)] + delete(m, 5) + [query_step(m, {})])
    m = model()
    case("Mahalin yüksekliği değişince eski", [made(m), set_height(m, 5, 2.7), query_step(m, {})])
    m = model()
    case("Kolon taşınınca güncel kalır: kolon çizilmez", [made(m), move_entity(m, 12, 0.5, 0.0), query_step(m, {})])
    m = model()
    case("İki iç görünüş ağacın sırasıyla", [made(m), made(m, space=15), query_step(m, {})])
    m = model()
    case("Katmana göre tek iç görünüş", [made(m), made(m, space=22), query_step(m, {"layer": "ic-gorunus-2"})])
    m = model()
    case("Ret: katman yok", [query_step(m, {"layer": "yok"})])
    m = model()
    case("Ret: türetim kaydı olmayan katman", [made(m), query_step(m, {"layer": "ic-gorunusler"})])
    m = model()
    st = [made(m)]
    t = m.records["ic-gorunus-1"]["members"]["1/frame"]
    st += [query_step(m, {"uid": m.uids[t]})]
    case("Nesnenin üyesi olduğu iç görünüş", st)
    m = model()
    case("Ret: iç görünüşün nesnesi değil", [made(m), query_step(m, {"uid": UID(1)})])
    m = model()
    case("Ret: geçersiz kimlik", [query_step(m, {"uid": "duvar-1"})])
    m = model()
    case("Ret: çizimde olmayan nesne", [query_step(m, {"uid": UID(99)})])
    m = model()
    st = [made(m), made(m, space=15)]
    t = m.records["ic-gorunus-1"]["members"]["0/frame"]
    st += [query_step(m, {"layer": "ic-gorunus-2", "uid": m.uids[t]})]
    case("Ret: nesne verilen katmanın iç görünüşünde değil", st)
    m = model()
    st = [made(m)]
    st += [release_step(m, "ic-gorunus-1")]
    st[-1]["result"] = ANY
    st[-1].pop("expect", None)
    st += [query_step(m, {})]
    case("Bağı koparılan iç görünüş listelenmez", st)
    m = model()
    st = [made(m), move_opening(m, 6, 3.0), refresh_step(m, "ic-gorunus-1")]
    st[-1]["result"] = ANY
    st[-1].pop("expect", None)
    st += [query_step(m, {})]
    case("Yeniden üretilen iç görünüş güncel", st)
    m = model()
    case("Duvarsız mutfak: güncel, yalnız istenen yanlar", [made(m, space=15, sides=[0, 2]), query_step(m, {})])
    return out


def model_after_undo(m):
    """The model with the opening's move undone: the door back at 1.5."""
    o = m.ents[6]
    host = m.ents[1]
    o["at"] = 1.5
    o["place"] = place(host, 1.5, o["width"])
    return m


def write(command, title, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "scripts/fixtures/room_elevation_cases.py yazar: ADR 0799'un kurallarından, KentOS kodu olmadan. Değiştirmeyin; betiği düzeltin.",
            "setup": setup(), "cases": cases}


def main():
    files = {
        EDIT: write("cad.elevations.edit", "Oda iç görünüşü (docs/adr/0799)", edit_cases()),
        QUERY: write("cad.elevations.query", "İç görünüşleri oku (docs/adr/0799)", query_cases()),
    }
    check = "--check" in sys.argv
    bad = False
    for path, data in files.items():
        text = json.dumps(data, ensure_ascii=False, indent=2) + "\n"
        if check:
            if not path.exists() or path.read_text() != text:
                print(f"{path.relative_to(ROOT)}: başvurudan farklı", file=sys.stderr)
                bad = True
            else:
                print(f"{data['command']} ({path.name}): {len(data['cases'])} durum başvuruyla aynı.")
        else:
            path.write_text(text)
            print(f"{path.relative_to(ROOT)}: {len(data['cases'])} durum yazıldı.")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
