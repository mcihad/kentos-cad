#!/usr/bin/env python3
"""Kesit çıkar's shared cases (docs/adr/8042 §1, §5): `cad.sections.edit` and `cad.sections.query`, written from the
ADR's rules without KentOS code.

The drawing: a 6 × 4 m room “101 Salon” on the ground floor (four 25 cm walls about their axes, the storey 3 m high),
a door in the south wall, a window in the north wall, the floor slab; section marks A (x = 3.2 from the north through
the door, looking east) and B (y = 2 from the west, looking north), a jogged mark C (three points), a mark D shorter
than a millimetre, an elevation mark E (north of the room, looking north: the south facade of what lies past it), a
note far off and a detail mark F. A second drawing adds what a section leaves out.

The rules:

- §1 A section's block id: the first 16 bytes of SHA-256(“kentos.section/” + the mark's id) as a UUID of version 8
  (byte 6's high nibble 8, byte 8's two high bits 10). Its block's name MM-XXXX-KESIT and the mark's letter; its name
  “A-A kesiti” (the mark's words trimmed, twice with a hyphen). An elevation mark's (docs/adr/8043) the same id, its
  name the direction its facade faces (its look turned round, from the plan's north, the nearest of eight):
  “Güney görünüşü”.
- §5 derive, its checks in order: the mark's id a lower-case hyphenated UUID (invalid_uid), in the drawing
  (entity_not_found), a view mark (not_a_section_mark) not a detail mark (not_a_section_mark, its own words), a
  section mark of two points (section_jogged); its section not drawn (section_exists); the origin finite (not_finite); the line at
  least a millimetre long (section_jogged); the expected revision (invalid_revision, revision_conflict). Written: one
  placement, one undo step “Kesit çıkar” (an elevation's “Görünüş çıkar”). Its output: the mark, the name, the block, the placement's id, the objects
  drawn (compared by the core's own cases: tests/all/sections.rs and fixtures/arch/v1/sections.json), changed, what
  is left out and the revision. A plan writes nothing and has no placement; a validation answers nothing.
- refresh, after the mark's checks: its section drawn (section_not_found), the line a millimetre long, the expected
  revision, then a section the model draws as its block holds is nothing_to_set; else written again, one undo step
  “Kesiti yeniden çiz” (“Görünüşü yeniden çiz”), changed, the ids of its placements.
- Left out, the whole drawing's, in this order and then by object id: the roof and the natural ground always; a
  curved wall or slab, a ramp, a wall or column without height, a wall with an opening without head, a stair with no
  slab to take its waist from (none at its top at the storey's height over its start, none at its foot at its floor),
  a wall with a niche or flue.
- query: every section mark in the drawing's order with its section's state (notDrawn without a block; current while
  drawing it again gives its block's objects, stale once it does not) and its placements' count, then each section
  block whose mark is gone (“MM-XXXX-KESITA (işareti yok)”, an elevation's “MM-XXXX-GORUNUSGUNEY (işareti yok)”,
  gone); by a mark (its checks as derive's), or by a
  placement's id (an insert of the drawing, else entity_not_found). A section is stale once an object it cuts or sees
  changes, the words of a room it passes through change, or its mark moves; a note, an object behind or beside its
  line, or its placements moved or deleted leave it current.

    python3 scripts/fixtures/section_command_cases.py          # writes the cases
    python3 scripts/fixtures/section_command_cases.py --check  # compares with the files
"""

import copy
import hashlib
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EDIT = ROOT / "fixtures/commands/v1/cad.sections.edit.json"
QUERY = ROOT / "fixtures/commands/v1/cad.sections.query.json"

DERIVE = "Kesit çıkar"
REFRESH = "Kesiti yeniden çiz"
FACADE_DERIVE = "Görünüş çıkar"
ANY = {"status": "completed", "output": "$any", "warnings": "$any"}
LEFT_ORDER = ["roof", "naturalGround", "curved", "ramp", "noHeight", "headUnknown", "stairWaist", "recess"]


def UID(n):
    return f"0192f5a1-8042-7000-8000-{n:012x}"


def P(x, y):
    return {"x": float(x), "y": float(y)}


def block_id(mark_uid):
    b = bytearray(hashlib.sha256(f"kentos.section/{mark_uid}".encode()).digest()[:16])
    b[6] = (b[6] & 0x0F) | 0x80
    b[8] = (b[8] & 0x3F) | 0x80
    h = b.hex()
    return f"{h[0:8]}-{h[8:12]}-{h[12:16]}-{h[16:20]}-{h[20:32]}"


FACINGS = ["Kuzey", "Kuzeydoğu", "Doğu", "Güneydoğu", "Güney", "Güneybatı", "Batı", "Kuzeybatı"]


def facing(e):
    """The direction an elevation mark's facade faces: its look (degrees from east, absent 90) turned round, the
    nearest of eight from the plan's north (the drawing's up: these drawings have no north arrow)."""
    a = math.radians(e.get("angle", 90.0))
    fx, fy = -math.cos(a), -math.sin(a)
    bearing = math.degrees(math.atan2(fx, fy)) % 360.0
    return FACINGS[int((bearing + 22.5) // 45.0) % 8]


def view_name(e):
    return f"{facing(e)} görünüşü" if e["form"] == "elevation" else f"{label(e['mark'])} kesiti"


FOLD = str.maketrans("çğıöşüÇĞİÖŞÜ", "cgiosuCGIOSU")


def block_name(e):
    """Its block's name (EK-4 Tablo 4.4: MM-XXXX-<ad>): the mark's letter, an elevation's direction in capitals."""
    if e["form"] == "elevation":
        return "MM-XXXX-GORUNUS" + facing(e).translate(FOLD).upper()
    return f"MM-XXXX-KESIT{e['mark']}"


def label(words):
    m = words.strip()
    return f"{m}-{m}" if m else "Kesit"


def layer(id, name):
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


# ── The drawing ──────────────────────────────────────────────────────────────────────────────────────────────────


def wall(id, a, b, width=0.25, bulges=None, level="zemin"):
    w = {"kind": "wall", "id": id, "layerId": "duvar", "attrs": {}, "pts": [P(*a), P(*b)], "width": width,
         "justify": "center"}
    if bulges:
        w["bulges"] = bulges
    if level:
        w["arch"] = {"level": level}
    return w


def place(w, at, width):
    """An opening's jambs from its wall, `at` its middle: the start's left and right face points, then the end's."""
    a, b = w["pts"][0], w["pts"][1]
    dx, dy = b["x"] - a["x"], b["y"] - a["y"]
    l = math.hypot(dx, dy)
    ux, uy = dx / l, dy / l
    nx, ny = -uy, ux
    h = w["width"] / 2.0
    out = []
    for s in (at - width / 2.0, at + width / 2.0):
        for d in (h, -h):
            out.append(P(a["x"] + ux * s + nx * d, a["y"] + uy * s + ny * d))
    return out


def opening(id, w, at, width, form, sill=None, head=None, number=None):
    o = {"kind": "opening", "id": id, "layerId": "kapi", "attrs": {}, "host": UID(w["id"]), "at": at,
         "width": width, "form": form}
    if sill is not None:
        o["sill"] = sill
    if head is not None:
        o["head"] = head
    if number:
        o["number"] = number
    o["place"] = place(w, at, width)
    return o


def rect(x0, y0, x1, y1):
    return {"outer": {"pts": [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]}}


def mark(id, form, points, words):
    return {"kind": "viewmark", "id": id, "layerId": "isaret", "attrs": {}, "form": form,
            "points": [P(*p) for p in points], "mark": words, "words": {"top": words, "bottom": "—"}}


# The marks' lines (their first two points) and where they look: the left of first → second.
LINES = {9: ((3.2, 5.5), (3.2, -1.5)), 10: ((-1.5, 2.0), (7.5, 2.0))}


def entities():
    s = wall(1, (0, 0), (6, 0))
    e = wall(2, (6, 0), (6, 4))
    n = wall(3, (6, 4), (0, 4))
    w = wall(4, (0, 4), (0, 0))
    return [
        s, e, n, w,
        opening(5, s, 3.0, 1.0, "door", head=2.1, number="K01"),
        opening(6, n, 4.4, 1.2, "window", sill=0.9, head=2.2, number="P01"),
        {"kind": "slab", "id": 7, "layerId": "doseme", "attrs": {}, "parts": [rect(-0.125, -0.125, 6.125, 4.125)],
         "thickness": 0.15, "material": "Betonarme", "arch": {"level": "zemin"}},
        {"kind": "space", "id": 8, "layerId": "mahal", "attrs": {}, "parts": [rect(0.125, 0.125, 5.875, 3.875)],
         "number": "101", "name": "Salon", "arch": {"level": "zemin"}},
        mark(9, "section", LINES[9], "A"),
        mark(10, "section", LINES[10], "B"),
        mark(11, "section", [(-1.0, -1.0), (3.0, -1.0), (3.0, 5.0)], "C"),
        mark(12, "section", [(9.0, 0.0), (9.0004, 0.0)], "D"),
        mark(13, "elevation", [(3.0, 7.0), (3.0, 6.0)], "E"),
        {"kind": "text", "id": 14, "layerId": "cizim", "attrs": {}, "p": P(30, 30), "text": "Not", "height": 0.25,
         "rotation": 0.0},
        {"kind": "viewmark", "id": 15, "layerId": "isaret", "attrs": {}, "form": "detail", "points": [P(1.0, 1.0)],
         "radius": 0.5, "at": P(-1.5, -1.5), "mark": "F", "words": {"top": "F", "bottom": "—"}},
    ]


def left_out_entities():
    """The drawing with what a section leaves out: a curved wall, a door without head, a niche, a ramp and a stair
    outside the slab."""
    ents = entities()
    curved = wall(21, (8, 0), (10, 0), bulges=[0.3])
    straight = wall(22, (8, 4), (12, 4))
    ents += [
        curved, straight,
        opening(23, straight, 1.0, 0.9, "door", number="K02"),
        opening(24, straight, 3.0, 0.6, "niche", sill=0.5, head=1.5),
        {"kind": "ramp", "id": 25, "layerId": "cizim", "attrs": {}, "pts": [P(14, 0), P(17, 0)], "width": 1.2,
         "rise": 0.15, "slope": 5.0},
        {"kind": "stair", "id": 26, "layerId": "cizim", "attrs": {}, "pts": [P(20, 0), P(20, 3)], "width": 1.0,
         "risers": 16, "riserHeight": 0.1875, "tread": 0.25, "arch": {"level": "zemin"}},
    ]
    return ents


def setup(ents):
    return {
        "format": "kentos.document", "version": 2, "name": "Kesit çıkar",
        "uids": [UID(e["id"]) for e in ents],
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 50.0, "workspace": "cad",
                     "arch": {"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0}]}},
        "origin": {"x": 0, "y": 0},
        "layers": [layer("duvar", "Duvar"), layer("kapi", "Kapı"), layer("doseme", "Döşeme"), layer("mahal", "Mahal"),
                   layer("isaret", "İşaret"), layer("cizim", "Çizim")],
        "activeLayer": "cizim",
        "styles": {"items": [], "categories": []},
        "blocks": [],
        "entities": ents,
    }


# ── The model a case changes ─────────────────────────────────────────────────────────────────────────────────────


class Refused(Exception):
    def __init__(self, code, message, path, status="failed", extra=None):
        super().__init__(message)
        self.code, self.message, self.path, self.status, self.extra = code, message, path, status, extra or {}


class Model:
    def __init__(self, ents):
        self.ents = {e["id"]: copy.deepcopy(e) for e in ents}
        self.next = max(self.ents) + 1
        self.blocks = {}  # block id -> {"mark": slot, "name": its block's name, "stale": bool}
        self.inserts = {}  # slot -> block id

    def by_uid(self, u):
        for i in self.ents:
            if UID(i) == u:
                return i
        return None

    def left_out(self):
        out = {("roof", None), ("naturalGround", None)}
        walls = {UID(i): e for i, e in self.ents.items() if e["kind"] == "wall"}
        for i, e in self.ents.items():
            u = UID(i)
            if e["kind"] == "wall":
                if any(b != 0 for b in e.get("bulges") or []):
                    out.add(("curved", u))
                if not e.get("arch", {}).get("level") and e.get("height") is None:
                    out.add(("noHeight", u))
            elif e["kind"] == "slab":
                if any(any(b != 0 for b in (p["outer"].get("bulges") or [])) for p in e["parts"]):
                    out.add(("curved", u))
            elif e["kind"] == "ramp":
                out.add(("ramp", u))
            elif e["kind"] == "opening" and e["host"] in walls:
                if e["form"] in ("niche", "flue"):
                    out.add(("recess", e["host"]))
                elif e.get("head") is None:
                    out.add(("headUnknown", e["host"]))
            elif e["kind"] == "stair":
                start, end = e["pts"][0], e["pts"][-1]
                top = e["riserHeight"] * e["risers"]
                if not (self.slab_at(start, 0.0) or self.slab_at(end, top)):
                    out.add(("stairWaist", u))
        rows = sorted(out, key=lambda r: (LEFT_ORDER.index(r[0]), r[1] or ""))
        return [{"why": w} if u is None else {"why": w, "uid": u} for w, u in rows]

    def slab_at(self, p, z):
        """A slab whose box holds `p` with its top (the storey's ±0.00 and its offset) within 0.5 m of `z`."""
        for e in self.ents.values():
            if e["kind"] != "slab" or abs(e.get("offset", 0.0) - z) > 0.5:
                continue
            for part in e["parts"]:
                xs = [q["x"] for q in part["outer"]["pts"]]
                ys = [q["y"] for q in part["outer"]["pts"]]
                if min(xs) <= p["x"] <= max(xs) and min(ys) <= p["y"] <= max(ys):
                    return True
        return False

    # The marks' checks (§5).
    def mark_of(self, uid, path):
        if not (len(uid) == 36 and all((c == "-") if k in (8, 13, 18, 23) else c in "0123456789abcdef"
                                       for k, c in enumerate(uid))):
            raise Refused("invalid_uid", f"“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir "
                                         "UUID'dir.", path)
        slot = self.by_uid(uid)
        if slot is None:
            raise Refused("entity_not_found", f"“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin "
                                              "olabilir.", path)
        e = self.ents[slot]
        if e["kind"] != "viewmark":
            raise Refused("not_a_section_mark", "Nesne kesit ya da görünüş işareti değil; kesit ve görünüş, planda "
                                                "çizilmiş işaretinden çıkarılır.", path)
        if e["form"] == "detail":
            raise Refused("not_a_section_mark", "İşaret detay işareti; kesit ve görünüş yalnız kesit ya da görünüş "
                                                "işaretinden çıkarılır.", path)
        if e["form"] == "section" and len(e["points"]) != 2:
            raise Refused("section_jogged", f"Kesit çizgisinin {len(e['points'])} noktası var; kırıklı kesit henüz "
                                            "çıkarılmıyor. İki noktalı bir kesit çizgisi çizin.", path)
        return slot, e

    def short(self, e):
        a, b = e["points"]
        return not math.hypot(b["x"] - a["x"], b["y"] - a["y"]) >= 1e-3

    # What a change of an object does to the drawn sections: stale when the object is cut or seen (§5). The box test
    # over-reaches (an object past the line hidden by what the line cuts is not seen): the cases move objects only
    # where it is exact, across the line or wholly behind or beside it.
    def sees(self, mark_slot, box):
        a, b = (self.ents[mark_slot]["points"][k] for k in (0, 1))
        L = math.hypot(b["x"] - a["x"], b["y"] - a["y"])
        tx, ty = (b["x"] - a["x"]) / L, (b["y"] - a["y"]) / L
        lx, ly = -ty, tx
        (x0, y0), (x1, y1) = box
        corners = [(x, y) for x in (x0, x1) for y in (y0, y1)]
        ts = [(x - a["x"]) * tx + (y - a["y"]) * ty for x, y in corners]
        ds = [(x - a["x"]) * lx + (y - a["y"]) * ly for x, y in corners]
        return max(ts) >= 0 and min(ts) <= L and max(ds) >= 0

    def changed(self, box=None, marks=None, room=False):
        for b in self.blocks.values():
            slot = b["mark"]
            if slot not in self.ents:
                continue
            if room or (marks and slot in marks) or (box and self.sees(slot, box)):
                b["stale"] = True


def strip(out):
    """An output as the contract writes it: empty lists left out."""
    return {k: v for k, v in out.items() if not (isinstance(v, list) and not v)}


def failed(e):
    error = {"code": e.code, "message": e.message, "path": e.path}
    error.update(e.extra)
    return {"status": e.status, "error": error}


def revision(expected):
    if expected is None:
        return
    if not expected.isdigit():
        raise Refused("invalid_revision", f"Beklenen sürüm “{expected}” geçerli bir sürüm değil; sürüm “12” gibi bir "
                                          "tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.",
                      "expectedRevision")
    raise Refused("revision_conflict", "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu "
                                       "çizimin şimdiki hâline göre yeniden hazırlayın.", "expectedRevision",
                  status="conflict", extra={"revision": "$current"})


def edit_step(op, operation, result, expected=None, **extra):
    inp = {"operation": operation}
    if expected is not None:
        inp["expectedRevision"] = expected
    s = {"op": op, "command": "cad.sections.edit", "input": inp, "result": copy.deepcopy(result)}
    s.update(extra)
    return s


def derive(m, slot, op="execute", at=None, expected=None, uid=None, non_finite=None):
    """A derive step of the mark in `slot` (or of the id `uid`), as the model answers."""
    uid = uid or UID(slot)
    operation = {"kind": "derive", "mark": uid}
    if at is not None:
        operation["at"] = P(*at)
    extra = {"nonFinite": non_finite} if non_finite else {}
    try:
        s, e = m.mark_of(uid, "operation.mark")
        bid = block_id(uid)
        if bid in m.blocks:
            raise Refused("section_exists", f"{view_name(e)} zaten çıkarılmış; güncellemek için Yeniden "
                                            "çiz'i kullanın.", "operation.mark")
        if non_finite:
            axis = next(iter(non_finite)).rsplit(".", 1)[1]
            name = "doğu (Y)" if axis == "x" else "kuzey (X)"
            whose = "Görünüşün yerinin" if e["form"] == "elevation" else "Kesitin yerinin"
            raise Refused("not_finite", f"{whose} {name} değeri sonlu bir sayı değil (NaN ya da sonsuz). "
                                        "Koordinatı sonlu bir sayıyla verin.", f"operation.at.{axis}")
        if e["form"] == "section" and m.short(e):
            raise Refused("section_jogged", "Kesit çizgisi bir milimetreden kısa; çizilemez.", "operation.mark")
        revision(expected)
    except Refused as r:
        return edit_step(op, operation, failed(r), expected, expect={"revision": "same"}, **extra)
    out = {"mark": uid, "name": view_name(e), "block": bid, "inserts": [], "drawn": "$any",
           "changed": True, "leftOut": m.left_out(), "revision": "$current"}
    if op == "validate":
        return edit_step(op, operation, {"status": "completed", "output": None, "warnings": []}, expected,
                         expect={"revision": "same"})
    if op == "plan":
        return edit_step(op, operation, {"status": "completed", "output": strip(out), "warnings": []}, expected,
                         expect={"revision": "same"})
    m.blocks[bid] = {"mark": s, "name": block_name(e), "stale": False}
    m.inserts[m.next] = bid
    m.next += 1
    out["inserts"] = ["$any"]
    return edit_step(op, operation, {"status": "completed", "output": strip(out), "warnings": []}, expected,
                     expect={"revision": "changed"})


def refresh(m, slot, op="execute", expected=None):
    uid = UID(slot)
    operation = {"kind": "refresh", "mark": uid}
    try:
        s, e = m.mark_of(uid, "operation.mark")
        bid = block_id(uid)
        if bid not in m.blocks:
            raise Refused("section_not_found", f"{view_name(e)} çıkarılmamış; önce "
                                               f"{'Görünüş çıkar' if e['form'] == 'elevation' else 'Kesit çıkar'}.",
                          "operation.mark")
        if m.short(e):
            raise Refused("section_jogged", "Kesit çizgisi bir milimetreden kısa; çizilemez.", "operation.mark")
        revision(expected)
        if not m.blocks[bid]["stale"]:
            raise Refused("nothing_to_set", f"{view_name(e)} güncel: model çizdiğinden beri değişmedi.",
                          "operation.mark")
    except Refused as r:
        return edit_step(op, operation, failed(r), expected, expect={"revision": "same"})
    n = sum(1 for b in m.inserts.values() if b == bid)
    out = {"mark": uid, "name": view_name(e), "block": bid, "inserts": ["$any"] * n,
           "drawn": "$any", "changed": True, "leftOut": m.left_out(), "revision": "$current"}
    if op == "validate":
        return edit_step(op, operation, {"status": "completed", "output": None, "warnings": []}, expected,
                         expect={"revision": "same"})
    if op == "execute":
        m.blocks[bid]["stale"] = False
    return edit_step(op, operation, {"status": "completed", "output": strip(out), "warnings": []}, expected,
                     expect={"revision": "changed" if op == "execute" else "same"})


def rows(m):
    out = []
    seen = set()
    for i in sorted(m.ents):
        e = m.ents[i]
        if e["kind"] != "viewmark" or e["form"] not in ("section", "elevation"):
            continue
        bid = block_id(UID(i))
        seen.add(bid)
        row = {"mark": UID(i), "name": view_name(e),
               "state": "notDrawn", "inserts": sum(1 for b in m.inserts.values() if b == bid)}
        if bid in m.blocks:
            row["block"] = bid
            row["state"] = "stale" if m.blocks[bid]["stale"] else "current"
        out.append(row)
    for bid, b in m.blocks.items():
        if bid not in seen:
            out.append({"name": f"{b['name']} (işareti yok)", "block": bid, "state": "gone",
                        "inserts": sum(1 for x in m.inserts.values() if x == bid)})
    return out


def query(m, mark_slot=None, uid=None, mark_uid=None, insert=None):
    """A query step: every section, the one of a mark (`mark_slot` or the id `mark_uid`), or the one whose placement
    is the insert in slot `insert` (captured first) or the id `uid`."""
    inp = {}
    steps = []
    try:
        if mark_slot is not None or mark_uid is not None:
            u = mark_uid or UID(mark_slot)
            inp["mark"] = u
            slot, _ = m.mark_of(u, "mark")
            found = [r for r in rows(m) if r.get("mark") == UID(slot)]
        elif insert is not None or uid is not None:
            if insert is not None:
                steps.append({"op": "captureUid", "id": insert, "as": f"n{insert}"})
                inp["uid"] = f"$uid:n{insert}"
                bid = m.inserts[insert]
            else:
                inp["uid"] = uid
                if not (len(uid) == 36 and all((c == "-") if k in (8, 13, 18, 23) else c in "0123456789abcdef"
                                               for k, c in enumerate(uid))):
                    raise Refused("invalid_uid", f"“{uid}” geçerli bir nesne kimliği değil.", "uid")
                raise Refused("entity_not_found", f"“{uid}” kimlikli bir kesit yerleşimi çizimde yok.", "uid")
            found = [r for r in rows(m) if r.get("block") == bid]
        else:
            found = rows(m)
        result = {"status": "completed", "output": {"sections": found, "revision": "$current"}, "warnings": []}
    except Refused as r:
        result = failed(r)
    steps.append({"op": "execute", "command": "cad.sections.query", "input": inp, "result": result,
                  "expect": {"revision": "same"}})
    return steps


# ── Changes a case makes with other commands ─────────────────────────────────────────────────────────────────────


def box_of(e):
    pts = []
    if e["kind"] == "opening":
        pts = e["place"]
    elif "pts" in e:
        pts = e["pts"]
    elif "points" in e:
        pts = e["points"]
    elif "p" in e:
        pts = [e["p"]]
    xs = [p["x"] for p in pts]
    ys = [p["y"] for p in pts]
    return (min(xs), min(ys)), (max(xs), max(ys))


def move_opening(m, slot, at):
    o = m.ents[slot]
    host = m.ents[m.by_uid(o["host"])]
    before = box_of(o)
    o["at"] = at
    o["place"] = place(host, at, o["width"])
    m.changed(box=before)
    m.changed(box=box_of(o))
    return {"op": "execute", "command": "cad.openings.edit",
            "input": {"uids": [UID(slot)], "operation": {"kind": "move", "at": at}}, "result": ANY}


def move(m, slot, dx, dy):
    """An object moved; a mark moved changes its own section, an insert moved none."""
    e = m.ents.get(slot)
    if e is not None:
        before = box_of(e)
        for k in ("p", "a", "b"):
            if k in e:
                e[k] = P(e[k]["x"] + dx, e[k]["y"] + dy)
        for k in ("pts", "points"):
            if k in e:
                e[k] = [P(p["x"] + dx, p["y"] + dy) for p in e[k]]
        if e["kind"] == "viewmark":
            m.changed(marks={slot})
        elif e["kind"] != "text":
            m.changed(box=before)
            m.changed(box=box_of(e))
        uid = UID(slot)
        steps = []
    else:
        steps = [{"op": "captureUid", "id": slot, "as": f"n{slot}"}]
        uid = f"$uid:n{slot}"
    return steps + [{"op": "execute", "command": "cad.entities.transform",
                     "input": {"uids": [uid], "transform": {"kind": "move", "dx": dx, "dy": dy}}, "result": ANY}]


def delete(m, slot):
    if slot in m.ents:
        e = m.ents.pop(slot)
        if e["kind"] not in ("viewmark", "text"):
            m.changed(box=box_of(e))
        return [{"op": "execute", "command": "cad.entities.delete", "input": {"uids": [UID(slot)]}, "result": ANY}]
    del m.inserts[slot]
    return [{"op": "captureUid", "id": slot, "as": f"n{slot}"},
            {"op": "execute", "command": "cad.entities.delete", "input": {"uids": [f"$uid:n{slot}"]},
             "result": ANY}]


def rename_room(m, slot, name):
    m.ents[slot]["name"] = name
    m.changed(room=True)
    return {"op": "execute", "command": "cad.spaces.edit",
            "input": {"uids": [UID(slot)], "operation": {"kind": "set", "name": name}}, "result": ANY}


def undo(m, returns, slot=None, bid=None):
    """An undo of a derive: its block and placement gone."""
    if bid is not None:
        del m.blocks[bid]
        m.inserts = {k: v for k, v in m.inserts.items() if v != bid}
        m.next -= 1
    return {"op": "undo", "returns": returns}


def redo(m, returns):
    return {"op": "redo", "returns": returns}


A, B, C, D, E, NOTE, WALL, DOOR, WINDOW, ROOM, F, INSERT = 9, 10, 11, 12, 13, 14, 1, 5, 6, 8, 15, 16


def case(name, steps, ents=None):
    c = {"name": name, "steps": [s for x in steps for s in (x if isinstance(x, list) else [x])]}
    if ents is not None:
        c["setup"] = setup(ents)
    return c


# ── cad.sections.edit ────────────────────────────────────────────────────────────────────────────────────────────


def edit_cases():
    out = []
    m = Model(entities())
    out.append(case("Doğrulama: A-A çıkarılabilir, çizim değişmez", [derive(m, A, "validate")]))
    m = Model(entities())
    out.append(case("Plan: A-A'nın adı, bloğu, çizilmeyenleri; yerleşim yok, çizim değişmez",
                    [derive(m, A, "plan", at=(20, 0))]))
    m = Model(entities())
    out.append(case("A-A çıkarılır: bloğu ve tek yerleşimi; geri alınır, yinelenir",
                    [derive(m, A, at=(20, 0)), undo(m, DERIVE), redo(m, DERIVE)]))
    m = Model(entities())
    out.append(case("B-B yeri verilmeden çizimin sağına çıkarılır", [derive(m, B)]))
    m = Model(entities())
    out.append(case("İki kesit ayrı bloklarda", [derive(m, A, at=(20, 0)), derive(m, B, at=(20, -10))]))
    m = Model(entities())
    out.append(case("Ret: kesit zaten çıkarılmış", [derive(m, A, at=(20, 0)), derive(m, A, at=(30, 0))]))
    m = Model(entities())
    out.append(case("Ret: doğrulama da çıkarılmış kesiti reddeder",
                    [derive(m, A, at=(20, 0)), derive(m, A, "validate")]))
    m = Model(entities())
    out.append(case("Kapı taşınınca Yeniden çiz kesiti yazar; geri alınır",
                    [derive(m, A, at=(20, 0)), move_opening(m, DOOR, 2.0), refresh(m, A, "plan"), refresh(m, A),
                     undo(m, REFRESH)]))
    m = Model(entities())
    out.append(case("Mahalin adı değişince iki kesit de yeniden çizilir",
                    [derive(m, A, at=(20, 0)), derive(m, B, at=(20, -10)), rename_room(m, ROOM, "Oturma odası"),
                     refresh(m, A), refresh(m, B)]))
    m = Model(entities())
    out.append(case("Ret: model değişmedi, yeniden çizilecek bir şey yok",
                    [derive(m, A, at=(20, 0)), refresh(m, A)]))
    m = Model(entities())
    out.append(case("Ret: pencere A-A'nın arkasında taşındı, A-A güncel",
                    [derive(m, A, at=(20, 0)), move_opening(m, WINDOW, 4.8), refresh(m, A)]))
    m = Model(entities())
    out.append(case("Ret: not taşındı, kesit güncel", [derive(m, A, at=(20, 0)), move(m, NOTE, 5, 0),
                                                      refresh(m, A)]))
    m = Model(entities())
    out.append(case("Ret: çıkarılmamış kesit yeniden çizilmez", [refresh(m, B)]))
    m = Model(entities())
    out.append(case("Ret: kesit işareti değil (duvar)", [derive(m, WALL)]))
    m = Model(entities())
    out.append(case("Görünüş işaretinden görünüşü çıkar: odanın kuzeyinden kuzeye bakan, Güney görünüşü",
                    [derive(m, E, at=(20, 10))]))
    m = Model(entities())
    out.append(case("Görünüşün geri alma adımı: Görünüş çıkar",
                    [derive(m, E, at=(20, 10)), undo(m, FACADE_DERIVE, bid=block_id(UID(E))),
                     redo(m, FACADE_DERIVE)]))
    m = Model(entities())
    out.append(case("Ret: detay işaretinden kesit ya da görünüş çıkmaz", [derive(m, F)]))
    m = Model(entities())
    out.append(case("Ret: kırıklı kesit çizgisi", [derive(m, C)]))
    m = Model(entities())
    out.append(case("Ret: milimetreden kısa kesit çizgisi", [derive(m, D)]))
    m = Model(entities())
    out.append(case("Ret: geçersiz kimlik", [derive(m, None, uid="a")]))
    m = Model(entities())
    out.append(case("Ret: büyük harfli kimlik", [derive(m, None, uid=UID(A).upper())]))
    m = Model(entities())
    out.append(case("Ret: çizimde olmayan işaret", [derive(m, None, uid=UID(99))]))
    m = Model(entities())
    out.append(case("Ret: sonsuz yer", [derive(m, A, at=(1.0, 0.0), non_finite={"operation.at.x": "NaN"})]))
    m = Model(entities())
    out.append(case("Ret: sonsuz yer, kuzeyi", [derive(m, A, at=(0.0, 1.0),
                                                       non_finite={"operation.at.y": "Infinity"})]))
    m = Model(entities())
    out.append(case("Ret: geçersiz beklenen sürüm", [derive(m, A, at=(20, 0), expected="abc")]))
    m = Model(entities())
    out.append(case("Ret: çizim bu arada değişti", [derive(m, A, at=(20, 0), expected="999")]))
    m = Model(entities())
    out.append(case("Ret: önce işaret denetlenir, sonra sürüm", [derive(m, None, uid="a", expected="abc")]))
    m = Model(entities())
    out.append(case("Ret: yeniden çizimde sürüm çakışması",
                    [derive(m, A, at=(20, 0)), move_opening(m, DOOR, 2.0), refresh(m, A, expected="999")]))
    m = Model(left_out_entities())
    out.append(case("Çizilmeyenler: eğri duvar, rampa, lentosuz kapı, plaksız merdiven, niş",
                    [derive(m, A, "plan", at=(30, 0))], left_out_entities()))
    m = Model(entities())
    out.append(case("Yerleşimi silinmiş kesit yine yeniden çizilir; yerleşimsiz",
                    [derive(m, A, at=(20, 0)), delete(m, INSERT), move_opening(m, DOOR, 2.0), refresh(m, A)]))
    return out


# ── cad.sections.query ───────────────────────────────────────────────────────────────────────────────────────────


def query_cases():
    out = []
    m = Model(entities())
    out.append(case("Hiç kesit çıkarılmamış: dört kesit ve bir görünüş işareti, detay işareti okunmaz", [query(m)]))
    m = Model(entities())
    out.append(case("A-A çıkarılınca güncel, bir yerleşim", [derive(m, A, at=(20, 0)), query(m)]))
    m = Model(entities())
    out.append(case("Kapı taşınınca A-A eski, B-B çıkarılmamış",
                    [derive(m, A, at=(20, 0)), move_opening(m, DOOR, 2.0), query(m)]))
    m = Model(entities())
    out.append(case("Yeniden çizilince yine güncel",
                    [derive(m, A, at=(20, 0)), move_opening(m, DOOR, 2.0), refresh(m, A), query(m)]))
    m = Model(entities())
    out.append(case("Pencere taşınınca B-B eski, A-A güncel (pencere A-A'nın arkasında)",
                    [derive(m, A, at=(20, 0)), derive(m, B, at=(20, -10)), move_opening(m, WINDOW, 4.8),
                     query(m)]))
    m = Model(entities())
    out.append(case("Mahalin adı değişince ikisi de eski",
                    [derive(m, A, at=(20, 0)), derive(m, B, at=(20, -10)), rename_room(m, ROOM, "Oturma odası"),
                     query(m)]))
    m = Model(entities())
    out.append(case("Not taşınınca kesit güncel kalır", [derive(m, A, at=(20, 0)), move(m, NOTE, 5, 0), query(m)]))
    m = Model(entities())
    out.append(case("Yerleşim taşınınca kesit güncel kalır",
                    [derive(m, A, at=(20, 0)), move(m, INSERT, 3, 0), query(m)]))
    m = Model(entities())
    out.append(case("Yerleşim silinince kesit güncel, yerleşimsiz",
                    [derive(m, A, at=(20, 0)), delete(m, INSERT), query(m)]))
    m = Model(entities())
    out.append(case("İşaret taşınınca kesiti eski", [derive(m, A, at=(20, 0)), move(m, A, 0.5, 0), query(m)]))
    m = Model(entities())
    out.append(case("Kesitin kestiği duvar silinince eski", [derive(m, A, at=(20, 0)), delete(m, WALL), query(m)]))
    m = Model(entities())
    out.append(case("İşareti silinen kesit: bloğu kalır, kaynağı yok",
                    [derive(m, A, at=(20, 0)), delete(m, A), query(m)]))
    m = Model(entities())
    out.append(case("İşareti silinen görünüş: bloğu kalır, kaynağı yok",
                    [derive(m, E, at=(20, 10)), delete(m, E), query(m)]))
    m = Model(entities())
    out.append(case("Geri alınan kesit yine çıkarılmamış",
                    [derive(m, A, at=(20, 0)), undo(m, DERIVE, bid=block_id(UID(A))), query(m)]))
    m = Model(entities())
    out.append(case("İşaretle sorulur: yalnız A-A", [derive(m, A, at=(20, 0)), query(m, mark_slot=A)]))
    m = Model(entities())
    out.append(case("İşaretle sorulur: çıkarılmamış B-B", [derive(m, A, at=(20, 0)), query(m, mark_slot=B)]))
    m = Model(entities())
    out.append(case("İşaretle sorulur: milimetreden kısa çizgi de listelenir", [query(m, mark_slot=D)]))
    m = Model(entities())
    out.append(case("Yerleşimle sorulur: onun kesiti",
                    [derive(m, A, at=(20, 0)), derive(m, B, at=(20, -10)), query(m, insert=INSERT)]))
    m = Model(entities())
    out.append(case("Ret: kesit işareti değil (duvar)", [query(m, mark_slot=WALL)]))
    m = Model(entities())
    out.append(case("İşaretle sorulur: görünüş işareti de listelenir", [query(m, mark_slot=E)]))
    m = Model(entities())
    out.append(case("Ret: detay işareti", [query(m, mark_slot=F)]))
    m = Model(entities())
    out.append(case("Ret: kırıklı kesit çizgisi", [query(m, mark_slot=C)]))
    m = Model(entities())
    out.append(case("Ret: geçersiz işaret kimliği", [query(m, mark_uid="a")]))
    m = Model(entities())
    out.append(case("Ret: çizimde olmayan işaret", [query(m, mark_uid=UID(99))]))
    m = Model(entities())
    out.append(case("Ret: geçersiz yerleşim kimliği", [query(m, uid="a")]))
    m = Model(entities())
    out.append(case("Ret: yerleşim değil (duvar)", [query(m, uid=UID(WALL))]))
    return out


def document(command, title, cases):
    return {
        "format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
        "note": "scripts/fixtures/section_command_cases.py yazar: ADR 8042'nin kurallarından, KentOS kodu olmadan. "
                "Değiştirmeyin; betiği düzeltin.",
        "setup": setup(entities()),
        "cases": cases,
    }


def main():
    files = {
        EDIT: document("cad.sections.edit", "Kesit çıkar ve yeniden çiz (docs/adr/8042)", edit_cases()),
        QUERY: document("cad.sections.query", "Kesitleri oku: güncel, eski, çıkarılmamış, kaynağı yok (docs/adr/8042)",
                        query_cases()),
    }
    check = "--check" in sys.argv[1:]
    bad = False
    for path, d in files.items():
        text = json.dumps(d, ensure_ascii=False, indent=2) + "\n"
        if check:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                print(f"{path.relative_to(ROOT)}: başvuruyla aynı değil; betiği çalıştırın.")
                bad = True
            else:
                print(f"{path.relative_to(ROOT)}: {len(d['cases'])} durum başvuruyla aynı.")
        else:
            path.write_text(text, encoding="utf-8")
            print(f"{path.relative_to(ROOT)}: {len(d['cases'])} durum yazıldı.")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
