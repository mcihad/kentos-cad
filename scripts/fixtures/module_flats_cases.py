#!/usr/bin/env python3
"""Room modules after their first slice (docs/adr/0779, ADV-05): the shared cases of `cad.modules.edit` and
`cad.modules.query` for a flat, written from the ADR's rules without KentOS code.

The drawing: a 6 × 4 m two-room flat on the ground floor (four outer walls and one inner wall 10 cm thick, an entrance
door and an inner door, a living room and a bedroom bound to their walls, both in the building program's flat d1, a
bed, two tags and a dimension on the front wall); outside it a second fixture and a door on the flat's right wall (to
be added), a corridor wall with its own door, a line and a wall on the first floor. The cases define the flat as a
module, place it as other flats (its spaces in their own group), repeat the ground floor on the upper floors in one
step, add members to the definition and take them out, and mark objects changed by hand (drift).

The rules (the ADR's, in its words):

* an instance's matrix: its definition mirrored across its own y axis when asked, turned, moved to its base;
* an instance's group (`unit`): its spaces are in it; a definition whose spaces all share one group leaves it to its
  first instance;
* Modüle ekle: the objects, in the instance's frame, join the definition (keys their ids, numbers unwrapped, level and
  the instance's group taken off); every other linked instance not on a locked layer gets them placed by its matrix,
  the walls first; the version goes one up; a skipped instance takes them on the next source update;
* Modülden çıkar: the keys leave the definition; the named objects stay as ordinary objects; the other instances'
  objects of those keys are deleted, unless a local exception or on a locked layer (released); a member leaning on a
  removed one refuses it;
* drift: an instance's object that differs from the definition placed by the instance's matrix (but for what the
  document writes itself: an opening's jambs, a found space's rings, a tag's text, a broken link; and an annotation's
  place on the view), not a local exception; a source update writes it back and says so;
* Katları tekrarla: every linked instance on the source level placed again on each target level at the same point,
  turn and mirror; its numbering and group the instance's line on the target, else the target's, else its own.

    python3 scripts/fixtures/module_flats_cases.py          # writes the cases
    python3 scripts/fixtures/module_flats_cases.py --check  # compares with the files
"""

import copy
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EDIT = ROOT / "fixtures/commands/v1/cad.modules.edit.flats.json"
QUERY = ROOT / "fixtures/commands/v1/cad.modules.query.drift.json"
# The KCAD sample of schema 144 (docs/adr/0779 §6): the flat as a module, its first instance in group d1.
SAMPLE = ROOT / "fixtures/kcad/v2/module-units.json"

BLOCK = "0192f5a1-7790-7000-8000-0000000000b1"
STEP = "Oda modülü"
ANY = {"status": "completed", "output": "$any", "warnings": "$any"}
THICK = 0.1


def UID(n):
    return f"0192f5a1-7790-7000-8000-{n:012x}"


def P(x, y):
    return {"x": float(x), "y": float(y)}


def layer(id, name):
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


# Walls: id → (start, end, level).
WALLS = {1: ((0, 0), (6, 0), "zemin"), 2: ((6, 0), (6, 4), "zemin"), 3: ((6, 4), (0, 4), "zemin"),
         4: ((0, 4), (0, 0), "zemin"), 5: ((3, 0), (3, 4), "zemin"), 16: ((0, -3), (6, -3), "zemin"),
         19: ((10, 0), (12, 0), "kat1")}
MEMBERS = list(range(1, 14))  # the flat's objects, in the order they are given
NEXT = 20  # the first slot a placement fills
LEVELS = [("zemin", "Zemin kat", 0.0), ("kat1", "1. kat", 3.0), ("kat2", "2. kat", 6.0), ("kat3", "3. kat", 9.0)]
# Doors: id → (wall, at, width, number).
DOORS = {6: (1, 1.5, 0.9, "K1"), 7: (5, 2.0, 0.8, "K2"), 15: (2, 2.0, 0.8, "K3"), 17: (16, 3.0, 1.0, "K9")}


def wall(i):
    a, b, level = WALLS[i]
    return {"kind": "wall", "id": i, "layerId": "duvar", "attrs": {}, "pts": [P(*a), P(*b)], "width": THICK,
            "justify": "center", "arch": {"level": level}}


def jambs(a, b, at, width):
    """An opening's jambs on the straight wall a → b: its start's left and right face points, then its end's."""
    ux, uy = b[0] - a[0], b[1] - a[1]
    n = math.hypot(ux, uy)
    ux, uy = ux / n, uy / n
    lx, ly = -uy, ux
    h = THICK / 2
    out = []
    for s in (at - width / 2, at + width / 2):
        cx, cy = a[0] + ux * s, a[1] + uy * s
        out += [P(cx + lx * h, cy + ly * h), P(cx - lx * h, cy - ly * h)]
    return out


def door(i):
    host, at, width, number = DOORS[i]
    a, b, _ = WALLS[host]
    return {"kind": "opening", "id": i, "layerId": "kapi", "attrs": {}, "host": UID(host), "at": at, "width": width,
            "form": "door", "frame": 0.05, "number": number, "place": jambs(a, b, at, width)}


def ring(x0, y0, x1, y1):
    return {"outer": {"pts": [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]}}


def space(i, sources, part, seed, number, name):
    return {"kind": "space", "id": i, "layerId": "mimari", "attrs": {}, "sources": [UID(s) for s in sources],
            "parts": [part], "seed": P(*seed), "number": number, "name": name, "arch": {"level": "zemin"},
            "unit": "d1"}


def insert(i, p):
    return {"kind": "insert", "id": i, "layerId": "mimari", "attrs": {}, "block": BLOCK, "p": P(*p), "scale": 1.0,
            "rotation": 0.0, "arch": {"level": "zemin"}}


def setup_entities():
    return [
        wall(1), wall(2), wall(3), wall(4), wall(5),
        door(6), door(7),
        space(8, (1, 3, 4, 5), ring(0.05, 0.05, 2.95, 3.95), (1.5, 2.0), "S1", "Salon"),
        space(9, (1, 2, 3, 5), ring(3.05, 0.05, 5.95, 3.95), (4.5, 2.0), "Y1", "Yatak odası"),
        insert(10, (4.5, 3.0)),
        {"kind": "tag", "id": 11, "layerId": "mimari", "attrs": {}, "of": UID(8), "at": P(1.5, 1.5),
         "content": "{number}", "text": "S1"},
        {"kind": "tag", "id": 12, "layerId": "mimari", "attrs": {}, "of": UID(9), "at": P(4.5, 1.5),
         "content": "{number}", "text": "Y1"},
        {"kind": "dimension", "id": 13, "layerId": "olcu", "attrs": {}, "a": P(0, 0), "b": P(6, 0), "height": 0.125,
         "offset": -0.5, "style": "linear", "angle": 0.0,
         "refs": {"a": {"uid": UID(1), "place": {"kind": "vertex", "part": 0, "ring": 0, "index": 0}},
                  "b": {"uid": UID(1), "place": {"kind": "vertex", "part": 0, "ring": 0, "index": 1}}}},
        insert(14, (1.0, 3.0)),
        door(15),
        wall(16),
        door(17),
        {"kind": "line", "id": 18, "layerId": "duvar", "attrs": {}, "a": P(8, 0), "b": P(9, 0)},
        wall(19),
    ]


def program():
    return {"buildings": [{"id": "a", "name": "A Blok"}],
            "units": [{"id": f"d{n}", "number": str(n), "name": f"Daire {n}", "building": "a"} for n in range(1, 5)]}


def setup():
    ents = setup_entities()
    return {
        "format": "kentos.document", "version": 2, "name": "Daire modülleri",
        "uids": [UID(e["id"]) for e in ents],
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 50, "workspace": "cad",
                     "arch": {"levels": [{"id": i, "name": n, "elevation": z, "height": 3.0} for i, n, z in LEVELS],
                              "program": program()}},
        "origin": {"x": 0, "y": 0},
        "layers": [layer("duvar", "Duvar"), layer("kapi", "Kapı"), layer("mimari", "Mimari"), layer("olcu", "Ölçü")],
        "activeLayer": "duvar",
        "styles": {"items": [], "categories": []},
        "blocks": [{"id": BLOCK, "name": "Yatak", "base": P(0, 0),
                    "entities": [{"kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": P(0, 0), "b": P(1.6, 0)}]}],
        "entities": ents,
    }


ENTS = {e["id"]: e for e in setup_entities()}


# ── The module's definition and instances ─────────────────────────────────────────────────────────────────────────


def definition_entity(i):
    """A member as the definition keeps it: the object at the origin's base, no slot, no level, no group (the flat's
    spaces all share d1, its first instance's)."""
    e = copy.deepcopy(ENTS[i])
    e["id"] = 0
    if "arch" in e:
        e["arch"].pop("level", None)
        if not e["arch"]:
            del e["arch"]
    e.pop("unit", None)
    return e


def member(i):
    return {"key": UID(i), "entity": definition_entity(i)}


def first_instance(keys=MEMBERS, version=1):
    return {"id": "daire-1", "at": P(0, 0), "level": "zemin", "unit": "d1", "version": version,
            "members": [{"key": UID(i), "uid": UID(i)} for i in keys]}


def defined():
    return {"id": "daire", "name": "Daire", "version": 1, "members": [member(i) for i in MEMBERS],
            "instances": [first_instance()]}


def matrix(at, rot, mirror):
    c, s = math.cos(rot), math.sin(rot)

    def f(p):
        x, y = p
        if mirror:
            x = -x
        return (c * x - s * y + at[0], s * x + c * y + at[1])
    return f


class Placed:
    """An instance placed: its record, and its objects' slots by member."""

    def __init__(self, n, first, at, rot=0.0, mirror=False, level=None, numbering=None, unit=None, keys=MEMBERS):
        self.n, self.at, self.rot, self.mirror, self.level = n, at, rot, mirror, level
        self.numbering, self.unit, self.keys = numbering or {}, unit, list(keys)
        self.slots = {m: first + k for k, m in enumerate(self.keys)}
        self.f = matrix(at, rot, mirror)

    def id(self):
        return f"daire-{self.n}"

    def uid(self, m):
        return f"$uidOf:{self.slots[m]}"

    def record(self, version=1, extra=()):
        r = {"id": self.id(), "at": P(*self.at)}
        if self.rot:
            r["rotation"] = self.rot
        if self.mirror:
            r["mirror"] = True
        if self.level:
            r["level"] = self.level
        if self.numbering:
            r["numbering"] = self.numbering
        if self.unit:
            r["unit"] = self.unit
        r["version"] = version
        r["members"] = [{"key": UID(m), "uid": self.uid(m)} for m in self.keys] + list(extra)
        return r

    def num(self, n):
        return self.numbering.get("prefix", "") + n + self.numbering.get("suffix", "")

    def wall_pts(self, i):
        """A wall's path, mirrored: run back, so the wall keeps its faces' sides."""
        a, b, _ = WALLS[i]
        return (self.f(b), self.f(a)) if self.mirror else (self.f(a), self.f(b))

    def arch(self):
        return {"level": self.level} if self.level else None

    def door_fields(self, i, width=None):
        host, at, w, number = DOORS[i]
        w = width if width is not None else w
        a, b = self.wall_pts(host)
        length = math.hypot(b[0] - a[0], b[1] - a[1])
        at = length - at if self.mirror else at
        return {"host": self.uid(host), "at": at, "width": w, "number": self.num(number), "side": None,
                "hand": "end" if self.mirror else None, "place": jambs(a, b, at, w)}

    def fields(self):
        """What the placement writes, object by object (the followers' own fields aside)."""
        out = {}
        for i in (1, 2, 3, 4, 5):
            a, b = self.wall_pts(i)
            out[str(self.slots[i])] = {"pts": [P(*a), P(*b)], "width": THICK, "arch": self.arch()}
        for i in (6, 7):
            out[str(self.slots[i])] = self.door_fields(i)
        for i, src, seed, number, name in ((8, (1, 3, 4, 5), (1.5, 2.0), "S1", "Salon"),
                                           (9, (1, 2, 3, 5), (4.5, 2.0), "Y1", "Yatak odası")):
            out[str(self.slots[i])] = {"sources": [self.uid(s) for s in src], "number": self.num(number),
                                       "name": name, "seed": P(*self.f(seed)), "unit": self.unit or "d1",
                                       "arch": self.arch()}
        out[str(self.slots[10])] = {"p": P(*self.f((4.5, 3.0))), "arch": self.arch()}
        out[str(self.slots[11])] = {"of": self.uid(8), "at": P(*self.f((1.5, 1.5))), "text": self.num("S1")}
        out[str(self.slots[12])] = {"of": self.uid(9), "at": P(*self.f((4.5, 1.5))), "text": self.num("Y1")}
        if not self.mirror:
            out[str(self.slots[13])] = {"a": P(*self.f((0, 0))), "b": P(*self.f((6, 0)))}
        return out


# ── Steps ─────────────────────────────────────────────────────────────────────────────────────────────────────────


def refusal(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def done(output, warnings=None):
    return {"status": "completed", "output": output, "warnings": warnings or []}


def warn(code, message):
    return {"code": code, "message": message, "path": "operation"}


def edited(module=None, instance=None, created=(), changed=(), placed=(), report=None):
    out = {}
    if module is not None:
        out["module"] = module
    if instance is not None:
        out["instance"] = instance
    if created:
        out["created"] = list(created)
    if changed:
        out["changed"] = list(changed)
    if placed:
        out["placed"] = list(placed)
    out["report"] = report or {}
    out["revision"] = "$current"
    return out


def run(op, result, expect=None, **extra):
    s = {"op": "execute", "input": {"operation": op}, "result": result}
    if expect:
        s["expect"] = expect
    s.update(extra)
    return s


def other(command, inp, expect=None):
    s = {"op": "execute", "command": command, "input": inp, "result": ANY}
    if expect:
        s["expect"] = expect
    return s


def case(name, steps):
    return {"name": name, "steps": steps}


DEFINE_OP = {"kind": "define", "name": "Daire", "uids": [UID(i) for i in MEMBERS], "base": P(0, 0)}


def DEFINE():
    return {"op": "execute", "input": {"operation": DEFINE_OP}, "result": ANY}


def PLACE(p):
    op = {"kind": "place", "module": "Daire", "at": P(*p.at)}
    if p.rot:
        op["rotation"] = p.rot
    if p.mirror:
        op["mirror"] = True
    if p.level:
        op["level"] = p.level
    if p.numbering:
        op["numbering"] = p.numbering
    if p.unit:
        op["unit"] = p.unit
    return op


def LOCK(layer_id, value=True):
    return other("cad.layers.set", {"operation": "locked", "layer": layer_id, "value": value})


IDS = list(range(1, 20))


def ids_with(*placements):
    out = list(IDS)
    for p in placements:
        out += sorted(p.slots.values())
    return out


def module_with(placements, version=1, members=None, first_keys=MEMBERS, first_version=None, records=None):
    m = defined()
    m["version"] = version
    if members is not None:
        m["members"] = members
    m["instances"] = [first_instance(first_keys, first_version or version)]
    m["instances"] += records if records is not None else [p.record(version) for p in placements]
    return m


def p2(**kw):
    """The second flat: 10 m right on the first floor, numbered 1-, in flat d2."""
    args = dict(at=(10, 0), level="kat1", numbering={"prefix": "1-"}, unit="d2")
    args.update(kw)
    return Placed(2, NEXT, **args)


# ── cad.modules.edit ──────────────────────────────────────────────────────────────────────────────────────────────


def define_cases():
    out = []
    out.append(case("Daire: iki odalı daire modül olur; mahallerin ortak bölümü ilk örneğin", [
        run(DEFINE_OP, done(edited(module=defined(), instance="daire-1")),
            {"ids": IDS, "revision": "changed", "canUndo": True})]))
    mixed = defined()
    mixed["members"][8]["entity"]["unit"] = "d2"
    mixed["members"][7]["entity"]["unit"] = "d1"
    mixed["instances"][0].pop("unit")
    # The bedroom in flat d2 from the start (the case's own drawing).
    two = setup()
    next(e for e in two["entities"] if e["id"] == 9)["unit"] = "d2"
    out.append(dict(case("Daire: iki bölüme yayılan mahaller bölümlerini tanımda tutar, örneğin bölümü olmaz", [
        run(DEFINE_OP, done(edited(module=mixed, instance="daire-1")))]), setup=two))
    p = p2()
    out.append(case("Daire yerleştir: 1. katta, 1- numaralı, mahalleri d2 bölümünde", [
        DEFINE(), run(PLACE(p), done(edited(module=module_with([p]), instance="daire-2",
                                            created=[p.uid(m) for m in MEMBERS])),
                      {"ids": ids_with(p), "fields": p.fields()}, within=1e-9)]))
    pm = p2(at=(20, 0), mirror=True, numbering={"prefix": "2-"}, unit="d3")
    out.append(case("Daire yerleştir: aynalı; iki kapı aynı yanda, açılışları sonda", [
        DEFINE(), run(PLACE(pm), done("$any"),
                      {"fields": {str(pm.slots[6]): pm.door_fields(6), str(pm.slots[7]): pm.door_fields(7)}},
                      within=1e-9)]))
    out.append(case("Ret: bina programında olmayan bölüm", [DEFINE(), run(
        dict(PLACE(p), unit="d9"),
        refusal("unit_not_found", "“d9” bölümü bina programında yok; önce Bina programı penceresinden ekleyin.",
                "operation.unit"), {"revision": "same"})]))
    pa = p2()
    out.append(case("Güncelle: bölümlü örnekten güncellemek bölümü tanıma taşımaz", [
        DEFINE(), run(PLACE(pa), ANY),
        other("cad.spaces.edit", {"uids": [pa.uid(8)], "operation": {"kind": "set", "name": "Oturma odası"}}),
        run({"kind": "update", "module": "Daire", "from": "daire-2"}, done("$any"),
            {"fields": {"8": {"name": "Oturma odası", "unit": "d1", "number": "S1"},
                        str(pa.slots[8]): {"name": "Oturma odası", "unit": "d2", "number": "1-S1"}}})]))
    return out


def repeat_cases():
    out = []
    t1 = {"level": "kat1", "numbering": {"prefix": "1-"}, "unit": "d2"}
    t2 = {"level": "kat2", "numbering": {"prefix": "2-"}, "unit": "d3"}
    a = Placed(2, NEXT, (0, 0), level="kat1", numbering={"prefix": "1-"}, unit="d2")
    b = Placed(3, NEXT + 13, (0, 0), level="kat2", numbering={"prefix": "2-"}, unit="d3")
    placed = [{"module": "daire", "instance": "daire-2", "level": "kat1"},
              {"module": "daire", "instance": "daire-3", "level": "kat2"}]
    out.append(case("Katları tekrarla: zemin kattaki daire 1. ve 2. kata, kendi numara ve bölümleriyle", [
        DEFINE(),
        run({"kind": "repeat", "from": "zemin", "to": [t1, t2]},
            done(edited(module=module_with([a, b]), created=[a.uid(m) for m in MEMBERS] + [b.uid(m) for m in MEMBERS],
                        placed=placed)),
            {"ids": ids_with(a, b), "fields": dict(a.fields(), **b.fields()), "revision": "changed"}, within=1e-9)]))
    out.append(case("Katları tekrarla'nın planı yazmaz", [
        DEFINE(),
        {"op": "plan", "input": {"operation": {"kind": "repeat", "from": "zemin", "to": [t1, t2]}},
         "result": done({"module": "$any", "created": "$any", "placed": placed, "report": {}, "revision": "$current"}),
         "expect": {"revision": "same", "ids": IDS}}]))
    out.append(case("Katları tekrarla ve geri al: tek adım", [
        DEFINE(), run({"kind": "repeat", "from": "zemin", "to": [t1, t2]}, ANY),
        {"op": "undo", "returns": STEP, "expect": {"ids": IDS}}]))
    c = Placed(2, NEXT, (0, 0), level="kat1", numbering={"prefix": "1A-"}, unit="d4")
    out.append(case("Katları tekrarla: örneğin kendi satırı katın numarasının ve bölümünün yerine geçer", [
        DEFINE(), run({"kind": "repeat", "from": "zemin", "to": [
            dict(t1, instances=[{"module": "Daire", "instance": "daire-1", "numbering": {"prefix": "1A-"},
                                 "unit": "d4"}])]},
            done("$any"), {"fields": {str(c.slots[8]): {"number": "1A-S1", "unit": "d4"},
                                      str(c.slots[6]): {"number": "1A-K1"}}})]))
    d = Placed(2, NEXT, (0, 0), level="kat3")
    out.append(case("Katları tekrarla: numara ve bölüm verilmezse örneğinki kalır", [
        DEFINE(), run({"kind": "repeat", "from": "zemin", "to": [{"level": "kat3"}]}, done("$any"),
                      {"fields": {str(d.slots[8]): {"number": "S1", "unit": "d1", "arch": {"level": "kat3"}},
                                  str(d.slots[1]): {"arch": {"level": "kat3"}}}})]))
    m2 = Placed(2, NEXT, (20, 0), mirror=True, level="zemin")
    # Two instances on the ground floor: the first's copy comes first (daire-3), the mirrored one's second (daire-4).
    m3 = Placed(4, NEXT + 26, (20, 0), mirror=True, level="kat1")
    out.append(case("Katları tekrarla: aynalı örnek aynalı tekrarlanır", [
        DEFINE(), run(PLACE(m2), ANY),
        run({"kind": "repeat", "from": "zemin", "to": [{"level": "kat1"}]}, done("$any"),
            {"fields": {str(m3.slots[6]): m3.door_fields(6), str(m3.slots[7]): m3.door_fields(7)}}, within=1e-9)]))
    out.append(case("Ret: hedef kat yok", [DEFINE(), run(
        {"kind": "repeat", "from": "zemin", "to": []},
        refusal("invalid_input", "Hedef kat verilmedi; en az bir kat seçin.", "operation.to"))]))
    out.append(case("Ret: olmayan kaynak kat", [DEFINE(), run(
        {"kind": "repeat", "from": "kat9", "to": [{"level": "kat1"}]},
        refusal("level_not_found", "Projede “kat9” katı yok.", "operation.from"))]))
    out.append(case("Ret: olmayan hedef kat", [DEFINE(), run(
        {"kind": "repeat", "from": "zemin", "to": [{"level": "kat1"}, {"level": "kat9"}]},
        refusal("level_not_found", "Projede “kat9” katı yok.", "operation.to[1].level"))]))
    out.append(case("Ret: kaynak kat hedeflerde", [DEFINE(), run(
        {"kind": "repeat", "from": "zemin", "to": [{"level": "zemin"}]},
        refusal("invalid_input", "Kaynak kat “zemin” hedeflerde olamaz.", "operation.to[0].level"))]))
    out.append(case("Ret: aynı hedef iki kez", [DEFINE(), run(
        {"kind": "repeat", "from": "zemin", "to": [{"level": "kat1"}, {"level": "kat1"}]},
        refusal("invalid_input", "“kat1” katı hedeflerde iki kez geçiyor.", "operation.to[1].level"))]))
    out.append(case("Ret: kaynak katta bağlı örnek yok", [DEFINE(), run(
        {"kind": "repeat", "from": "kat3", "to": [{"level": "kat1"}]},
        refusal("module_repeat_empty", "“kat3” katında tekrarlanacak bağlı modül örneği yok.", "operation.from"))]))
    out.append(case("Ret: kat zaten tekrarlanmış", [
        DEFINE(), run({"kind": "repeat", "from": "zemin", "to": [{"level": "kat1"}]}, ANY),
        run({"kind": "repeat", "from": "zemin", "to": [{"level": "kat1"}]},
            refusal("module_repeat_taken", "“Daire” modülünün “kat1” katında aynı yerde bir örneği var (daire-2).",
                    "operation.to[0].level"), {"revision": "same"})]))
    out.append(case("Ret: olmayan bölüm", [DEFINE(), run(
        {"kind": "repeat", "from": "zemin", "to": [{"level": "kat1", "unit": "d9"}]},
        refusal("unit_not_found", "“d9” bölümü bina programında yok; önce Bina programı penceresinden ekleyin.",
                "operation.to[0].unit"))]))
    out.append(case("Ret: olmayan modül süzgeci", [DEFINE(), run(
        {"kind": "repeat", "from": "zemin", "modules": ["Mutfak"], "to": [{"level": "kat1"}]},
        refusal("module_not_found", "Projede “Mutfak” adlı ya da kimlikli oda modülü yok.", "operation.modules[0]"))]))
    out.append(case("Ret: satırın örneği kaynak katta değil", [DEFINE(), run(
        {"kind": "repeat", "from": "zemin", "to": [
            {"level": "kat1", "instances": [{"module": "daire", "instance": "daire-7"}]}]},
        refusal("module_instance_not_found", "“daire-7” örneği “zemin” katında tekrarlanacak örnekler arasında yok.",
                "operation.to[0].instances[0].instance"))]))
    out.append(case("Ret: kilitli katman", [DEFINE(), LOCK("duvar"), run(
        {"kind": "repeat", "from": "zemin", "to": [{"level": "kat1"}]},
        refusal("layer_locked", "“Duvar” katmanı kilitli; modül tekrarlanamaz.", "operation.from"))]))
    out.append(case("Ret: kaynağı silinmiş modülün süzgeçte adı", [
        DEFINE(), run({"kind": "remove", "module": "Daire"}, ANY),
        run({"kind": "repeat", "from": "zemin", "modules": ["Daire"], "to": [{"level": "kat1"}]},
            refusal("module_removed", "“Daire” modülünün kaynağı silinmiş; örnekleri son hâliyle duruyor.",
                    "operation.modules[0]"))]))
    out.append(case("Ret: kaynağı silinmiş modülün kopuk örneği tekrarlanmaz", [
        DEFINE(), run({"kind": "remove", "module": "Daire"}, ANY),
        run({"kind": "repeat", "from": "zemin", "to": [{"level": "kat1"}]},
            refusal("module_repeat_empty", "“zemin” katında tekrarlanacak bağlı modül örneği yok.", "operation.from"))]))
    return out


def add_cases():
    out = []
    p = p2()
    bed = definition_entity(14)
    door15 = definition_entity(15)
    v2 = module_with([p], version=2, members=[member(i) for i in MEMBERS] + [{"key": UID(14), "entity": bed}],
                     first_keys=MEMBERS + [14],
                     records=[p.record(2, extra=[{"key": UID(14), "uid": f"$uidOf:{NEXT + 13}"}])])
    out.append(case("Modüle ekle: ikinci yatak tanıma katılır, öbür daireye kendi yerinde gelir", [
        DEFINE(), run(PLACE(p), ANY),
        run({"kind": "addMembers", "module": "Daire", "instance": "daire-1", "uids": [UID(14)]},
            done(edited(module=v2, instance="daire-1", created=[f"$uidOf:{NEXT + 13}"],
                        report={"updated": ["daire-1", "daire-2"], "addedKeys": [UID(14)]})),
            {"ids": ids_with(p) + [NEXT + 13],
             "fields": {str(NEXT + 13): {"p": P(*p.f((1.0, 3.0))), "arch": {"level": "kat1"}, "block": BLOCK}}},
            within=1e-9)]))
    q = Placed(2, NEXT, (10, 0), level="kat1", numbering={"prefix": "1-"}, unit="d2", keys=MEMBERS + [15])
    out.append(case("Modüle ekle: duvardaki yeni kapı öbür dairenin kendi duvarına, kendi numarasıyla", [
        DEFINE(), run(PLACE(p), ANY),
        run({"kind": "addMembers", "module": "Daire", "instance": "daire-1", "uids": [UID(15)]}, done("$any"),
            {"fields": {"15": {"host": UID(2), "number": "K3"},
                        str(NEXT + 13): {"host": q.uid(2), "at": 2.0, "width": 0.8, "number": "1-K3",
                                         "place": jambs(q.f((6, 0)), q.f((6, 4)), 2.0, 0.8)}}}, within=1e-9)]))
    out.append(case("Modüle ekle: tanımın kapısının kaydı kendi çerçevesinde, numarası yalın", [
        DEFINE(), run(PLACE(p), ANY),
        run({"kind": "addMembers", "module": "Daire", "instance": "daire-1", "uids": [UID(15)]},
            done({"module": "$any", "instance": "daire-1", "created": [f"$uidOf:{NEXT + 13}"],
                  "report": {"updated": ["daire-1", "daire-2"], "addedKeys": [UID(15)]}, "revision": "$current"})),
        {"op": "execute", "command": "cad.modules.query", "input": {"uid": UID(15)},
         "result": done({"modules": "$any", "instance": "daire-1", "key": UID(15), "revision": "$current"})}]))
    out.append(case("Modüle ekle ve geri al: tek adım", [
        DEFINE(), run(PLACE(p), ANY),
        run({"kind": "addMembers", "module": "Daire", "instance": "daire-1", "uids": [UID(14), UID(15)]}, ANY),
        {"op": "undo", "returns": STEP, "expect": {"ids": ids_with(p)}}]))
    out.append(case("Modüle ekle: kilitli dairenin örneği atlanır, sonraki güncellemede alır", [
        DEFINE(), run(PLACE(p), ANY), LOCK("olcu"),
        run({"kind": "addMembers", "module": "Daire", "instance": "daire-1", "uids": [UID(15)]},
            done({"module": "$any", "instance": "daire-1", "report": {
                "updated": ["daire-1"], "skipped": [{"instance": "daire-2", "why": "locked"}],
                "addedKeys": [UID(15)]}, "revision": "$current"},
                [warn("module_instance_skipped",
                      "1 örnek kilitli katmandaki nesneleri yüzünden güncellenmedi: daire-2.")]),
            {"ids": ids_with(p)}),
        LOCK("olcu", False),
        run({"kind": "update", "module": "Daire", "from": "daire-1"},
            done({"module": "$any", "instance": "daire-1", "created": [f"$uidOf:{NEXT + 13}"],
                  "report": {"updated": ["daire-1", "daire-2"], "addedKeys": [UID(15)]}, "revision": "$current"}),
            {"ids": ids_with(p) + [NEXT + 13], "fields": {str(NEXT + 13): {"host": p.uid(2), "number": "1-K3"}}})]))
    out.append(case("Ret: eklenen kapının duvarı örneğin dışında", [DEFINE(), run(
        {"kind": "addMembers", "module": "Daire", "instance": "daire-1", "uids": [UID(17)]},
        refusal("module_outside",
                f"Eklenen açıklık örneğin dışındaki bir nesneye bağlı (duvar, {UID(16)}); onu da ekleyin ya da bağı kaldırın (kapsam dışı).",
                "operation.uids[0]"), {"revision": "same"})]))
    out.append(case("Modüle ekle: duvarı ile kapısı birlikte", [DEFINE(), run(
        {"kind": "addMembers", "module": "Daire", "instance": "daire-1", "uids": [UID(16), UID(17)]},
        done({"module": "$any", "instance": "daire-1", "report": {"updated": ["daire-1"],
                                                                   "addedKeys": [UID(16), UID(17)]},
              "revision": "$current"}))]))
    out.append(case("Ret: nesne zaten bir örneğin", [DEFINE(), run(
        {"kind": "addMembers", "module": "Daire", "instance": "daire-1", "uids": [UID(1)]},
        refusal("module_member_taken", "Bu nesne zaten bir modül örneğinin; önce o örneği bağımsızlaştırın.",
                "operation.uids[0]"))]))
    # An upper level's wall is the definition's second storey since docs/adr/0796 §1 (scripts/fixtures/
    # module_storeys_cases.py); the case once refused it.
    out.append(case("Modüle ekle: üst kattaki duvar tanımın ikinci katı olur", [DEFINE(), run(
        {"kind": "addMembers", "module": "Daire", "instance": "daire-1", "uids": [UID(19)]},
        done({"module": "$any", "instance": "daire-1", "report": {"updated": ["daire-1"], "addedKeys": [UID(19)]},
              "revision": "$current"}))]))
    out.append(case("Ret: çizgi modüle eklenmez", [DEFINE(), run(
        {"kind": "addMembers", "module": "Daire", "instance": "daire-1", "uids": [UID(18)]},
        refusal("module_kind", "Oda modülünde düz duvar, açıklık, mahal, donatı, ölçü, etiket ve yazı olur.",
                "operation.uids[0]"))]))
    out.append(case("Ret: olmayan örnek", [DEFINE(), run(
        {"kind": "addMembers", "module": "Daire", "instance": "daire-9", "uids": [UID(14)]},
        refusal("module_instance_not_found", "“Daire” modülünde “daire-9” örneği yok.", "operation.instance"))]))
    out.append(case("Ret: eklenecek nesne yok", [DEFINE(), run(
        {"kind": "addMembers", "module": "Daire", "instance": "daire-1", "uids": []},
        refusal("no_entities", "Modüle eklenecek nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin.",
                "operation.uids"))]))
    out.append(case("Ret: eklenecek nesnenin katmanı kilitli", [DEFINE(), LOCK("kapi"), run(
        {"kind": "addMembers", "module": "Daire", "instance": "daire-1", "uids": [UID(15)]},
        refusal("layer_locked", "“Kapı” katmanı kilitli; modüle eklenemez.", "operation.uids[0]"))]))
    return out


def remove_cases():
    out = []
    p = p2()
    v2 = module_with([p], version=2, members=[member(i) for i in MEMBERS if i != 10],
                     first_keys=[i for i in MEMBERS if i != 10],
                     records=[dict(p.record(2), members=[{"key": UID(m), "uid": p.uid(m)} for m in MEMBERS if m != 10])])
    out.append(case("Modülden çıkar: yatak tanımdan çıkar; ilk dairede kalır, öbüründe silinir", [
        DEFINE(), run(PLACE(p), ANY), {"op": "captureUid", "id": p.slots[10], "as": "yatak2"},
        run({"kind": "removeMembers", "module": "Daire", "uids": [UID(10)]},
            done(edited(module=v2, report={"updated": ["daire-1", "daire-2"], "removedKeys": [UID(10)],
                                           "released": [UID(10)], "deleted": ["$uid:yatak2"]})),
            {"ids": [i for i in ids_with(p) if i != p.slots[10]]})]))
    out.append(case("Modülden çıkar: öbür dairenin nesnesinden de", [
        DEFINE(), run(PLACE(p), ANY),
        run({"kind": "removeMembers", "module": "daire", "uids": [p.uid(10)]},
            done({"module": "$any", "report": {"updated": ["daire-1", "daire-2"], "removedKeys": [UID(10)],
                                               "released": [p.uid(10)], "deleted": [UID(10)]},
                  "revision": "$current"}),
            {"ids": [i for i in ids_with(p) if i != 10]})]))
    out.append(case("Modülden çıkar: yerel istisna olan karşılık kalır, söylenir", [
        DEFINE(), run(PLACE(p), ANY),
        run({"kind": "except", "module": "Daire", "uids": [p.uid(7)]}, ANY),
        run({"kind": "removeMembers", "module": "Daire", "uids": [UID(7)]},
            done({"module": "$any", "report": {
                "updated": ["daire-1", "daire-2"],
                "conflicts": [{"instance": "daire-2", "key": UID(7), "uid": p.uid(7), "why": "exception"}],
                "removedKeys": [UID(7)], "released": [UID(7), p.uid(7)]}, "revision": "$current"},
                [warn("module_member_released",
                      "Tanımdan çıkan nesnenin 1 karşılığı silinmedi (yerel istisna ya da kilitli katman); sıradan nesne olarak kaldı.")]),
            {"ids": ids_with(p)})]))
    out.append(case("Modülden çıkar: kilitli katmandaki karşılık kalır", [
        DEFINE(), run(PLACE(p), ANY), LOCK("mimari"),
        run({"kind": "removeMembers", "module": "Daire", "uids": [UID(10)]},
            done({"module": "$any", "report": {
                "updated": ["daire-1", "daire-2"],
                "conflicts": [{"instance": "daire-2", "key": UID(10), "uid": p.uid(10), "why": "locked"}],
                "removedKeys": [UID(10)], "released": [UID(10), p.uid(10)]}, "revision": "$current"},
                [warn("module_member_released",
                      "Tanımdan çıkan nesnenin 1 karşılığı silinmedi (yerel istisna ya da kilitli katman); sıradan nesne olarak kaldı.")]),
            {"ids": ids_with(p)})]))
    out.append(case("Modülden çıkar ve geri al: tek adım, silinen geri gelir", [
        DEFINE(), run(PLACE(p), ANY),
        run({"kind": "removeMembers", "module": "Daire", "uids": [UID(10), UID(12)]}, ANY),
        {"op": "undo", "returns": STEP, "expect": {"ids": ids_with(p)}}]))
    out.append(case("Ret: kapı ve mahaller çıkarılan duvara bağlı", [DEFINE(), run(
        {"kind": "removeMembers", "module": "Daire", "uids": [UID(5)]},
        refusal("module_member_needed",
                "Modülün bir nesnesi (açıklık) çıkarılan nesneye (duvar) bağlı; onu da birlikte çıkarın ya da bırakın.",
                "operation.uids[0]"), {"revision": "same"})]))
    out.append(case("Ret: etiket çıkarılan mahale bağlı", [DEFINE(), run(
        {"kind": "removeMembers", "module": "Daire", "uids": [UID(10), UID(9)]},
        refusal("module_member_needed",
                "Modülün bir nesnesi (etiket) çıkarılan nesneye (mahal) bağlı; onu da birlikte çıkarın ya da bırakın.",
                "operation.uids[1]"))]))
    out.append(case("Ret: bütün nesneler çıkarılamaz", [DEFINE(), run(
        {"kind": "removeMembers", "module": "Daire", "uids": [UID(i) for i in MEMBERS]},
        refusal("invalid_input", "Modülün bütün nesneleri çıkarılamaz; modülü silmek için Kaynağı sil'i kullanın.",
                "operation.uids"))]))
    out.append(case("Ret: nesne modülün örneğinde değil", [DEFINE(), run(
        {"kind": "removeMembers", "module": "Daire", "uids": [UID(16)]},
        refusal("not_in_module", "Nesne “Daire” modülünün bir örneğinde değil.", "operation.uids[0]"))]))
    out.append(case("Ret: kaynağı silinmiş modül", [
        DEFINE(), run({"kind": "remove", "module": "Daire"}, ANY),
        run({"kind": "removeMembers", "module": "Daire", "uids": [UID(10)]},
            refusal("module_removed", "“Daire” modülünün kaynağı silinmiş; örnekleri son hâliyle duruyor.",
                    "operation.module"))]))
    return out


def drift_cases():
    out = []
    p = p2()
    out.append(case("Elle değişen kapı güncellemede kaynağın enine döner, söylenir", [
        DEFINE(), run(PLACE(p), ANY),
        other("cad.openings.edit", {"uids": [p.uid(6)], "operation": {"kind": "set", "width": 1.0}}),
        other("cad.openings.edit", {"uids": [UID(6)], "operation": {"kind": "set", "width": 0.95}}),
        run({"kind": "update", "module": "Daire", "from": "daire-1"},
            done({"module": "$any", "instance": "daire-1", "changed": [p.uid(6)], "report": {
                "updated": ["daire-1", "daire-2"],
                "conflicts": [{"instance": "daire-2", "key": UID(6), "uid": p.uid(6), "why": "drifted"}],
                "changedKeys": [UID(6)]}, "revision": "$current"},
                [warn("module_drift_overwritten",
                      "Elle değiştirilmiş 1 örnek nesnesi kaynağın değeriyle yazıldı; korumak için önce yerel istisna yapın.")]),
            {"fields": {str(p.slots[6]): {"width": 0.95}}})]))
    out.append(case("Elle değişen ama değişmeyen anahtarın nesnesi güncellemede yerinde kalır", [
        DEFINE(), run(PLACE(p), ANY),
        other("cad.entities.transform", {"uids": [p.uid(10)], "transform": {"kind": "move", "dx": 0.5, "dy": 0.0}}),
        other("cad.openings.edit", {"uids": [UID(6)], "operation": {"kind": "set", "width": 0.95}}),
        run({"kind": "update", "module": "Daire", "from": "daire-1"},
            done({"module": "$any", "instance": "daire-1", "changed": [p.uid(6)], "report": {
                "updated": ["daire-1", "daire-2"], "changedKeys": [UID(6)]}, "revision": "$current"}),
            {"fields": {str(p.slots[10]): {"p": P(15.0, 3.0)}}}, within=1e-9)]))
    out.append(case("Elle değişen kapı yerel istisna yapılınca güncellemede korunur", [
        DEFINE(), run(PLACE(p), ANY),
        other("cad.openings.edit", {"uids": [p.uid(6)], "operation": {"kind": "set", "width": 1.0}}),
        run({"kind": "except", "module": "Daire", "uids": [p.uid(6)]}, ANY),
        other("cad.openings.edit", {"uids": [UID(6)], "operation": {"kind": "set", "width": 0.95}}),
        run({"kind": "update", "module": "Daire", "from": "daire-1"},
            done("$any", [warn("module_exception_kept",
                               "1 nesne yerel istisna olduğu için kaynağın değişikliğini almadı.")]),
            {"fields": {str(p.slots[6]): {"width": 1.0}}})]))
    out.append(case("Elle değişen kapı kaynağa döndürülür", [
        DEFINE(), run(PLACE(p), ANY),
        other("cad.openings.edit", {"uids": [p.uid(6)], "operation": {"kind": "set", "width": 1.0}}),
        run({"kind": "restore", "module": "Daire", "uids": [p.uid(6)]}, done("$any"),
            {"fields": {str(p.slots[6]): {"width": 0.9}}})]))
    return out


def edit_cases():
    return define_cases() + repeat_cases() + add_cases() + remove_cases() + drift_cases()


# ── cad.modules.query ─────────────────────────────────────────────────────────────────────────────────────────────


def query_cases():
    out = []
    E = "cad.modules.edit"

    def q(name, inp, result, *before):
        return case(name, list(before) + [{"op": "execute", "input": inp, "result": result}])

    def ed(op):
        return {"op": "execute", "command": E, "input": {"operation": op}, "result": ANY}

    de = dict(DEFINE(), command=E)

    def irow(id, version=1, objects=13, exceptions=(), missing=(), drifted=(), unit=None, level="zemin",
             mirror=False, stale=False, broken=None):
        r = {"id": id, "version": version, "stale": stale, "objects": objects}
        for k, v in (("exceptions", exceptions), ("missing", missing), ("drifted", drifted)):
            if v:
                r[k] = list(v)
        if broken:
            r["broken"] = broken
        if unit:
            r["unit"] = unit
        if level:
            r["level"] = level
        r["mirror"] = mirror
        return r

    def row(instances, version=1, members=13, removed=False):
        return {"id": "daire", "name": "Daire", "version": version, "members": members, "removed": removed,
                "instances": instances}

    def ans(modules, instance=None, key=None, drifted=False):
        o = {"modules": modules}
        if instance:
            o["instance"] = instance
        if key:
            o["key"] = key
        if drifted:
            o["drifted"] = True
        o["revision"] = "$current"
        return done(o)

    p = p2()
    pm = p2(at=(20, 0), mirror=True, numbering={"prefix": "2-"}, unit="d3")
    first = irow("daire-1", unit="d1")
    second = irow("daire-2", unit="d2", level="kat1")
    move = lambda uid, dx=0.5: {"op": "execute", "command": "cad.entities.transform",
                                "input": {"uids": [uid], "transform": {"kind": "move", "dx": dx, "dy": 0.0}},
                                "result": ANY}
    out.append(q("Yeni yerleşen dairelerde elle değişen yok; bölümleri okunur", {}, ans([row([first, second])]),
                 de, ed(PLACE(p))))
    out.append(q("Aynalı ve numaralı dairede de elle değişen yok", {},
                 ans([row([first, irow("daire-2", unit="d3", level="kat1", mirror=True)])]), de, ed(PLACE(pm))))
    out.append(q("Tekrarlanan katlarda elle değişen yok", {}, ans([row([
        first, irow("daire-2", unit="d2", level="kat1"), irow("daire-3", unit="d3", level="kat2")])]),
        de, ed({"kind": "repeat", "from": "zemin", "to": [{"level": "kat1", "unit": "d2"},
                                                         {"level": "kat2", "unit": "d3"}]})))
    out.append(q("Modülsüz proje", {}, ans([])))
    out.append(q("Kimlikle tek modül", {"module": "daire"}, ans([row([first, second])]), de, ed(PLACE(p))))
    plain = Placed(2, NEXT, (10, 0), level="kat1")
    out.append(q("Bölümsüz yerleşen dairenin satırında bölüm yok", {}, ans([row([first, irow("daire-2", level="kat1")])]),
                 de, ed(PLACE(plain))))
    turned = Placed(2, NEXT, (10, 10), rot=math.pi / 2, level="kat1", unit="d2")
    out.append(q("Döndürülmüş dairede elle değişen yok", {}, ans([row([first, second])]), de, ed(PLACE(turned))))
    out.append(q("Elle taşınıp geri alınan yatak değişmiş değil", {}, ans([row([first, second])]),
                 de, ed(PLACE(p)), move(p.uid(10)), {"op": "undo", "returns": "Taşı"}))
    out.append(q("Eklenen nesnenin örneği ve anahtarı", {"uid": UID(14)},
                 ans([row([irow("daire-1", unit="d1", version=2, objects=14),
                           irow("daire-2", unit="d2", level="kat1", version=2, objects=14)], version=2, members=14)],
                     "daire-1", UID(14)),
                 de, ed(PLACE(p)), ed({"kind": "addMembers", "module": "Daire", "instance": "daire-1",
                                       "uids": [UID(14)]})))
    out.append(q("Elle taşınan yatak elle değişmiş sayılır", {},
                 ans([row([first, irow("daire-2", unit="d2", level="kat1", drifted=[p.uid(10)])])]),
                 de, ed(PLACE(p)), move(p.uid(10))))
    out.append(q("Nesnenin örneği: elle değişmiş", {"uid": p.uid(10)},
                 ans([row([first, irow("daire-2", unit="d2", level="kat1", drifted=[p.uid(10)])])],
                     "daire-2", UID(10), True),
                 de, ed(PLACE(p)), move(p.uid(10))))
    out.append(q("Nesnenin örneği: değişmemiş", {"uid": p.uid(6)},
                 ans([row([first, second])], "daire-2", UID(6)), de, ed(PLACE(p))))
    out.append(q("Elle taşınan etiket görünüşteki yeridir, değişmiş sayılmaz", {}, ans([row([first, second])]),
                 de, ed(PLACE(p)), move(p.uid(11))))
    out.append(q("Elle değişen kapının eni", {}, ans([row([first, irow("daire-2", unit="d2", level="kat1",
                                                                        drifted=[p.uid(6)])])]),
                 de, ed(PLACE(p)),
                 other("cad.openings.edit", {"uids": [p.uid(6)], "operation": {"kind": "set", "width": 1.0}})))
    out.append(q("Yerel istisna elle değişmiş sayılmaz", {}, ans([row([first, irow(
        "daire-2", unit="d2", level="kat1", exceptions=[p.uid(6)])])]),
                 de, ed(PLACE(p)),
                 other("cad.openings.edit", {"uids": [p.uid(6)], "operation": {"kind": "set", "width": 1.0}}),
                 ed({"kind": "except", "module": "Daire", "uids": [p.uid(6)]})))
    out.append(q("Kaynağa döndürülen artık değişmiş değil", {}, ans([row([first, second])]),
                 de, ed(PLACE(p)), move(p.uid(10)), ed({"kind": "restore", "module": "Daire", "uids": [p.uid(10)]})))
    out.append(q("İlk örnekte elle değişen mahal adı", {}, ans([row([irow("daire-1", unit="d1", drifted=[UID(8)]),
                                                                     second])]),
                 de, ed(PLACE(p)),
                 other("cad.spaces.edit", {"uids": [UID(8)], "operation": {"kind": "set", "name": "Oturma odası"}})))
    out.append(q("Kaynağı güncellenince elle değişen kalmaz", {}, ans([row([irow("daire-1", unit="d1", version=2),
                                                                           irow("daire-2", unit="d2", level="kat1",
                                                                                version=2)], version=2)]),
                 de, ed(PLACE(p)),
                 other("cad.spaces.edit", {"uids": [UID(8)], "operation": {"kind": "set", "name": "Oturma odası"}}),
                 ed({"kind": "update", "module": "Daire", "from": "daire-1"})))
    out.append(q("Eski sürümde kalan örnekte elle değişen sayılmaz", {}, ans([row([
        irow("daire-1", unit="d1", version=2), irow("daire-2", unit="d2", level="kat1", stale=True)], version=2)]),
                 de, ed(PLACE(p)),
                 other("cad.spaces.edit", {"uids": [UID(8)], "operation": {"kind": "set", "name": "Oturma odası"}}),
                 LOCK("olcu"), ed({"kind": "update", "module": "Daire", "from": "daire-1"})))
    out.append(q("Kaynağı silinmiş modülde elle değişen sayılmaz", {}, ans([row([
        irow("daire-1", unit="d1", broken="sourceGone")], members=0, removed=True)]),
                 de, other("cad.openings.edit", {"uids": [UID(6)], "operation": {"kind": "set", "width": 1.0}}),
                 ed({"kind": "remove", "module": "Daire"})))
    out.append(q("Eklenen nesne sayılır", {}, ans([row([irow("daire-1", unit="d1", version=2, objects=14),
                                                        irow("daire-2", unit="d2", level="kat1", version=2,
                                                             objects=14)], version=2, members=14)]),
                 de, ed(PLACE(p)), ed({"kind": "addMembers", "module": "Daire", "instance": "daire-1",
                                       "uids": [UID(14)]})))
    out.append(q("Çıkarılan nesne sayılmaz", {}, ans([row([irow("daire-1", unit="d1", version=2, objects=12),
                                                           irow("daire-2", unit="d2", level="kat1", version=2,
                                                                objects=12)], version=2, members=12)]),
                 de, ed(PLACE(p)), ed({"kind": "removeMembers", "module": "Daire", "uids": [UID(10)]})))
    return out


def sample():
    d = setup()
    d["name"] = "Daire modülleri"
    d["settings"]["arch"]["modules"] = [defined()]
    return d


def write(command, title, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "scripts/fixtures/module_flats_cases.py yazar: ADR 0779'un kurallarından, KentOS kodu olmadan. Değiştirmeyin; betiği düzeltin.",
            "setup": setup(), "cases": cases}


def main():
    files = {
        EDIT: write("cad.modules.edit", "Daire modülleri: bölüm, kat tekrarı, üye ekle ve çıkar, elle değişen (docs/adr/0779)",
                    edit_cases()),
        QUERY: write("cad.modules.query", "Daire modüllerinde elle değişenler ve bölümler (docs/adr/0779)",
                     query_cases()),
    }
    check = "--check" in sys.argv
    bad = False
    sample_text = json.dumps(sample(), ensure_ascii=False, indent=2) + "\n"
    if check:
        if not SAMPLE.exists() or SAMPLE.read_text() != sample_text:
            print(f"{SAMPLE.relative_to(ROOT)}: başvurudan farklı", file=sys.stderr)
            bad = True
    else:
        SAMPLE.write_text(sample_text)
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
