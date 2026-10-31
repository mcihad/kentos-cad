#!/usr/bin/env python3
"""Room modules of several storeys and a repeat of a level's objects in no module (docs/adr/0796, ADV-05): the shared
cases of `cad.modules.edit` and `cad.modules.query`, written from the ADR's rules without KentOS code.

The drawing: four levels listed out of their elevations' order (1. kat 3 m, zemin 0 m, 2. kat 6 m, bodrum −3 m); a
duplex: two walls, a door, a drawn living room on the ground floor and two walls, a door, a drawn room, a bed, the room's
tag and a dimension on the first floor; beside it, on the ground floor and in no module, a hall (two walls, a door, a
space bound to the walls and numbered with the ground floor's prefix, its tag, a dimension on a wall, a fixture on a
layer of its own, a column, a dimension from the hall's wall to the duplex's), a space on the ground floor bound to a
first floor wall and its tag; an unlevelled wall, a wall on a level the project lacks and a wall in the basement.

The rules (the ADR's, in its words):

* the storey order: the project's levels by elevation, the list's order between equal ones;
* Modül tanımla: objects on several of the project's levels make a module of as many storeys, its base the lowest; a
  wall's, a space's or a fixture's storey its level's place above the base; an opening, a tag, a dimension storey 0;
  levelled and unlevelled walls, spaces or fixtures together, or a level the project lacks, refuse it;
* Modül yerleştir and Katları tekrarla: a member of storey n stands on the level n places above the instance's base;
  too few levels above, or no base, refuse it (`module_storey_missing`);
* Modüle ekle: an object on the instance's base level or a level above it joins on its storey; one below refuses it;
  another instance with too few levels above refuses it;
* Kaynağı güncelle: a member's storey is the one its source object stands on now;
* Katları tekrarla with `loose`: every object of the source level (a wall's, a space's, a fixture's own; an opening's
  its wall's; a tag's its object's; a dimension's its points' objects' when all on one) in no module is copied to each
  target after the target's instances, in the drawing's order: a wall, a space, a fixture on the target level; its
  references to the other copies and to the repeated instances' objects made theirs; a space's number with the source
  level's prefix taken off and the target's numbering put on (with no numbering the target level's prefix in place of
  the source's), an opening's number with the target's numbering; an object of an instance not repeated, on a locked
  layer, of another kind, or leaning on what is neither copied nor repeated (and so in turn) is left and said.

    python3 scripts/fixtures/module_storeys_cases.py          # writes the cases
    python3 scripts/fixtures/module_storeys_cases.py --check  # compares with the files
"""

import copy
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EDIT = ROOT / "fixtures/commands/v1/cad.modules.edit.storeys.json"
QUERY = ROOT / "fixtures/commands/v1/cad.modules.query.storeys.json"
# The KCAD sample of schema 152 (docs/adr/0796 §3): the duplex as a module of two storeys. The sample
# written in schema 151 is refused as `unknown_field`.
SAMPLE = ROOT / "fixtures/kcad/v2/module-storeys.json"

BLOCK = "0192f5a1-7960-7000-8000-0000000000b1"
STEP = "Oda modülü"
ANY = {"status": "completed", "output": "$any", "warnings": "$any"}
THICK = 0.1
NEXT = 28  # the first slot a write fills

# The levels as the project lists them: (id, name, elevation).
LEVELS = [("kat1", "1. kat", 3.0), ("zemin", "Zemin kat", 0.0), ("kat2", "2. kat", 6.0), ("bodrum", "Bodrum", -3.0)]
# The storey order: by elevation, the list's order between equal ones.
ORDER = [i for _, i in sorted(((z, k), i) for k, (i, _, z) in enumerate(LEVELS))]
assert ORDER == ["bodrum", "zemin", "kat1", "kat2"]
PREFIXES = {"zemin": "Z-", "kat1": "1-"}


def UID(n):
    return f"0192f5a1-7960-7000-8000-{n:012x}"


def P(x, y):
    return {"x": float(x), "y": float(y)}


def up(level, n):
    """The level `n` storeys above `level`; none when the project lacks it."""
    i = ORDER.index(level) + n
    return ORDER[i] if i < len(ORDER) else None


def layer(id, name, locked=False):
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


# Walls: id → (start, end, level or None).
WALLS = {1: ((0, 0), (4, 0), "zemin"), 2: ((4, 0), (4, 3), "zemin"), 3: ((0, 0), (4, 0), "kat1"),
         4: ((4, 0), (4, 3), "kat1"), 12: ((10, 0), (14, 0), "zemin"), 13: ((14, 0), (14, 3), "zemin"),
         21: ((20, 0), (22, 0), "kat1"), 24: ((30, 0), (32, 0), None), 25: ((40, 0), (42, 0), "yok"),
         26: ((0, -5), (4, -5), "bodrum")}
# Doors: id → (wall, at, width, number).
DOORS = {5: (1, 1.0, 0.9, "K1"), 6: (3, 2.0, 0.8, "K2"), 14: (12, 1.0, 0.9, "K5")}
DUPLEX = list(range(1, 12))  # the duplex's objects, in the order they are given
ONE = [1, 2, 5]  # a module of one storey: the ground floor's walls and door


def wall(i):
    a, b, level = WALLS[i]
    e = {"kind": "wall", "id": i, "layerId": "duvar", "attrs": {}, "pts": [P(*a), P(*b)], "width": THICK,
         "justify": "center"}
    if level:
        e["arch"] = {"level": level}
    return e


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


def space(i, part, number, name, level, sources=None, seed=None):
    e = {"kind": "space", "id": i, "layerId": "mimari", "attrs": {}}
    if sources:
        e["sources"] = [UID(s) for s in sources]
    e["parts"] = [part]
    if seed:
        e["seed"] = P(*seed)
    e.update({"number": number, "name": name, "arch": {"level": level}})
    return e


def insert(i, p, level, layer_id="mimari"):
    return {"kind": "insert", "id": i, "layerId": layer_id, "attrs": {}, "block": BLOCK, "p": P(*p), "scale": 1.0,
            "rotation": 0.0, "arch": {"level": level}}


def vertex(uid, index):
    return {"uid": uid, "place": {"kind": "vertex", "part": 0, "ring": 0, "index": index}}


def dimension(i, a, b, ra, rb):
    return {"kind": "dimension", "id": i, "layerId": "olcu", "attrs": {}, "a": P(*a), "b": P(*b), "height": 0.125,
            "offset": -0.5, "style": "linear", "angle": 0.0, "refs": {"a": ra, "b": rb}}


def tag(i, of, at, text):
    return {"kind": "tag", "id": i, "layerId": "mimari", "attrs": {}, "of": UID(of), "at": P(*at),
            "content": "{number}", "text": text}


def setup_entities():
    return [
        wall(1), wall(2), wall(3), wall(4), door(5), door(6),
        space(7, ring(0.05, 0.05, 3.95, 2.95), "D1", "Salon", "zemin"),
        space(8, ring(0.05, 0.05, 3.95, 2.95), "D2", "Oda", "kat1"),
        insert(9, (2, 2), "kat1"),
        tag(10, 8, (2, 1.5), "D2"),
        dimension(11, (0, 0), (4, 0), vertex(UID(3), 0), vertex(UID(3), 1)),
        wall(12), wall(13), door(14),
        space(15, ring(10.05, 0.05, 13.95, 2.95), "Z-101", "Hol", "zemin", sources=(12, 13), seed=(12, 1.5)),
        tag(16, 15, (12, 1.5), "Z-101"),
        dimension(17, (10, 0), (14, 0), vertex(UID(12), 0), vertex(UID(12), 1)),
        insert(18, (11, 1), "zemin", "mobilya"),
        {"kind": "column", "id": 19, "layerId": "mimari", "attrs": {}, "at": P(16, 1), "width": 0.4, "depth": 0.4,
         "arch": {"level": "zemin"}},
        dimension(20, (10, 0), (4, 0), vertex(UID(12), 0), vertex(UID(1), 1)),
        wall(21),
        space(22, ring(20.05, 0.05, 21.95, 1.95), "Z-102", "Depo", "zemin", sources=(21,), seed=(21, 1)),
        tag(23, 22, (21, 1), "Z-102"),
        wall(24), wall(25), wall(26),
        {"kind": "line", "id": 27, "layerId": "duvar", "attrs": {}, "a": P(50, 0), "b": P(51, 0)},
    ]


ENTS = {e["id"]: e for e in setup_entities()}
IDS = sorted(ENTS)


def setup():
    ents = setup_entities()
    return {
        "format": "kentos.document", "version": 2, "name": "Çok katlı modüller",
        "uids": [UID(e["id"]) for e in ents],
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 50, "workspace": "cad",
                     "arch": {"levels": [{"id": i, "name": n, "elevation": z, "height": 3.0} for i, n, z in LEVELS],
                              "spaceNumberPrefixes": PREFIXES}},
        "origin": {"x": 0, "y": 0},
        "layers": [layer("duvar", "Duvar"), layer("kapi", "Kapı"), layer("mimari", "Mimari"), layer("olcu", "Ölçü"),
                   layer("mobilya", "Mobilya")],
        "activeLayer": "duvar",
        "styles": {"items": [], "categories": []},
        "blocks": [{"id": BLOCK, "name": "Yatak", "base": P(0, 0),
                    "entities": [{"kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": P(0, 0), "b": P(1.6, 0)}]}],
        "entities": ents,
    }


# ── The module's definition ───────────────────────────────────────────────────────────────────────────────────────


def own_level(e):
    return (e.get("arch") or {}).get("level") if e["kind"] in ("wall", "space", "insert") else None


def storey(i, keys):
    """A member's storey: its level's place above the lowest of the definition's levels; 0 but a wall's, a space's,
    a fixture's."""
    levels = [own_level(ENTS[k]) for k in keys if own_level(ENTS[k])]
    low = min(ORDER.index(lv) for lv in levels) if levels else 0
    lv = own_level(ENTS[i])
    return ORDER.index(lv) - low if lv else 0


def definition_entity(i):
    """A member as the definition keeps it: at the base (the origin), no slot, no level."""
    e = copy.deepcopy(ENTS[i])
    e["id"] = 0
    if "arch" in e:
        e["arch"].pop("level", None)
        if not e["arch"]:
            del e["arch"]
    return e


def member(i, keys):
    m = {"key": UID(i), "entity": definition_entity(i)}
    if storey(i, keys):
        m["storey"] = storey(i, keys)
    return m


def defined(keys=DUPLEX, name="Dubleks", mid="dubleks"):
    return {"id": mid, "name": name, "version": 1, "members": [member(i, keys) for i in keys],
            "instances": [{"id": f"{mid}-1", "at": P(0, 0), "level": "zemin", "version": 1,
                           "members": [{"key": UID(i), "uid": UID(i)} for i in keys]}]}


class Placed:
    """An instance placed by a translation: its record and its objects' slots by member."""

    def __init__(self, n, first, at, level, numbering=None, keys=DUPLEX, mid="dubleks"):
        self.n, self.at, self.level, self.numbering, self.keys, self.mid = n, at, level, numbering or {}, keys, mid
        self.slots = {m: first + k for k, m in enumerate(keys)}

    def f(self, p):
        return (p[0] + self.at[0], p[1] + self.at[1])

    def id(self):
        return f"{self.mid}-{self.n}"

    def uid(self, m):
        return f"$uidOf:{self.slots[m]}"

    def last(self):
        return max(self.slots.values())

    def record(self, version=1):
        r = {"id": self.id(), "at": P(*self.at), "level": self.level}
        if self.numbering:
            r["numbering"] = self.numbering
        r["version"] = version
        r["members"] = [{"key": UID(m), "uid": self.uid(m)} for m in self.keys]
        return r

    def num(self, n):
        return self.numbering.get("prefix", "") + n + self.numbering.get("suffix", "")

    def level_of(self, i):
        return up(self.level, storey(i, self.keys))

    def fields(self):
        """What the placement writes, object by object (the followers' own fields aside)."""
        out = {}
        for i in self.keys:
            e = ENTS[i]
            s = str(self.slots[i])
            if e["kind"] == "wall":
                a, b, _ = WALLS[i]
                out[s] = {"pts": [P(*self.f(a)), P(*self.f(b))], "arch": {"level": self.level_of(i)}}
            elif e["kind"] == "opening":
                host, at, w, number = DOORS[i]
                a, b, _ = WALLS[host]
                out[s] = {"host": self.uid(host), "at": at, "number": self.num(number),
                          "place": jambs(self.f(a), self.f(b), at, w)}
            elif e["kind"] == "space":
                out[s] = {"number": self.num(e["number"]), "arch": {"level": self.level_of(i)}}
            elif e["kind"] == "insert":
                out[s] = {"p": P(*self.f((e["p"]["x"], e["p"]["y"]))), "arch": {"level": self.level_of(i)}}
            elif e["kind"] == "tag":
                out[s] = {"of": self.uid(int(e["of"][-12:], 16))}
            elif e["kind"] == "dimension":
                out[s] = {"a": P(*self.f((e["a"]["x"], e["a"]["y"]))), "b": P(*self.f((e["b"]["x"], e["b"]["y"]))),
                          "refs": {k: {"uid": self.uid(int(r["uid"][-12:], 16)), "place": r["place"]}
                                   for k, r in e["refs"].items()}}
        return out


# ── The loose objects of a level ──────────────────────────────────────────────────────────────────────────────────


def level_of(e):
    """An object's level as a level view reads it."""
    k = e["kind"]
    if k in ("wall", "space", "insert", "column"):
        return (e.get("arch") or {}).get("level")
    if k == "opening":
        return level_of(ENTS[int(e["host"][-12:], 16)])
    if k == "tag":
        return level_of(ENTS[int(e["of"][-12:], 16)])
    if k == "dimension":
        levels = {level_of(ENTS[int(r["uid"][-12:], 16)]) for r in e["refs"].values()}
        return levels.pop() if len(levels) == 1 else None
    return None


def refs(e):
    """The drawing's objects an object names."""
    out = []
    if e["kind"] == "opening":
        out.append(e["host"])
    if e["kind"] == "space":
        out += e.get("sources", [])
    if e["kind"] == "tag":
        out.append(e["of"])
    if e["kind"] == "dimension":
        out += [r["uid"] for r in e["refs"].values()]
    return [int(u[-12:], 16) for u in out]


def loose_plan(level, in_modules, repeated, locked=(), in_options=()):
    """The ids a repeat with `loose` copies from `level`, in the drawing's order, and those it leaves with why."""
    copy_, left = [], {}
    for i in IDS:
        e = ENTS[i]
        if level_of(e) != level or i in repeated:
            continue
        if i in in_modules:
            left[i] = "module"
        elif i in in_options:
            left[i] = "option"
        elif e["layerId"] in locked:
            left[i] = "locked"
        elif e["kind"] not in ("wall", "opening", "space", "insert", "dimension", "tag"):
            left[i] = "kind"
        else:
            copy_.append(i)
    while True:
        held = set(copy_) | set(repeated)
        out = [i for i in copy_ if any(r not in held for r in refs(ENTS[i]))]
        if not out:
            break
        for i in out:
            copy_.remove(i)
            left[i] = "outside"
    return copy_, [{"uid": UID(i), "why": left[i]} for i in sorted(left)]


def renumbered(n, from_prefix, to_prefix, numbering):
    stripped = n[len(from_prefix):] if from_prefix and n.startswith(from_prefix) else None
    if numbering and (numbering.get("prefix") or numbering.get("suffix")):
        base = stripped if stripped is not None else n
        return numbering.get("prefix", "") + base + numbering.get("suffix", "")
    if stripped is not None and to_prefix:
        return to_prefix + stripped
    return n


class Copies:
    """A repeat's loose copies on one target, from slot `first`: their slots, the map of references."""

    def __init__(self, ids, first, target, numbering=None, instance=None, source="zemin"):
        self.ids, self.target, self.numbering, self.source = ids, target, numbering, source
        self.slots = {i: first + k for k, i in enumerate(ids)}
        self.instance = instance

    def uid_of(self, i):
        if i in self.slots:
            return f"$uidOf:{self.slots[i]}"
        return self.instance.uid(i)

    def copied(self):
        return [{"source": UID(i), "uid": f"$uidOf:{self.slots[i]}", "level": self.target} for i in self.ids]

    def fields(self):
        out = {}
        for i in self.ids:
            e = ENTS[i]
            s = str(self.slots[i])
            k = e["kind"]
            if k == "wall":
                out[s] = {"pts": e["pts"], "arch": {"level": self.target}}
            elif k == "opening":
                out[s] = {"host": self.uid_of(int(e["host"][-12:], 16)), "at": e["at"],
                          "number": renumbered(e["number"], None, None, self.numbering)}
            elif k == "space":
                f = {"arch": {"level": self.target},
                     "number": renumbered(e["number"], PREFIXES.get(self.source), PREFIXES.get(self.target),
                                          self.numbering)}
                if "sources" in e:
                    f["sources"] = [self.uid_of(int(u[-12:], 16)) for u in e["sources"]]
                out[s] = f
            elif k == "insert":
                out[s] = {"p": e["p"], "arch": {"level": self.target}}
            elif k == "tag":
                out[s] = {"of": self.uid_of(int(e["of"][-12:], 16)), "at": e["at"]}
            elif k == "dimension":
                out[s] = {"refs": {n: {"uid": self.uid_of(int(r["uid"][-12:], 16)), "place": r["place"]}
                                   for n, r in e["refs"].items()}}
        return out


# ── Steps ─────────────────────────────────────────────────────────────────────────────────────────────────────────


def refusal(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def done(output, warnings=None):
    return {"status": "completed", "output": output, "warnings": warnings or []}


def warn(code, message):
    return {"code": code, "message": message, "path": "operation"}


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


def case(name, steps, **extra):
    c = {"name": name, "steps": steps}
    c.update(extra)
    return c


def DEFINE(keys=DUPLEX, name="Dubleks"):
    return {"op": "execute", "input": {"operation": {"kind": "define", "name": name, "uids": [UID(i) for i in keys],
                                                     "base": P(0, 0)}},
            "result": ANY}


def PLACE(p, name="Dubleks"):
    op = {"kind": "place", "module": name, "at": P(*p.at), "level": p.level}
    if p.numbering:
        op["numbering"] = p.numbering
    return op


def LOCK(layer_id, value=True):
    return other("cad.layers.set", {"operation": "locked", "layer": layer_id, "value": value})


def REPEAT(to, loose=True, modules=None, source="zemin"):
    op = {"kind": "repeat", "from": source}
    if modules:
        op["modules"] = modules
    op["to"] = to
    if loose:
        op["loose"] = True
    return op


def storey_missing(name, n, base, above):
    base = next(lname for i, lname, _ in LEVELS if i == base)
    return (f"“{name}” modülü {n} katlı; “{base}” katının üstünde {n - 1} kat gerekir, projede {above} var. "
            "Daha alçak bir taban katı seçin ya da projeye kat ekleyin.")


def left_warning(n):
    return warn("module_repeat_left",
                f"Kattaki {n} modülsüz nesne kopyalanmadı (kilitli katman, tekrarlanmayan tür, tekrarlanmayan örnek, "
                "tasarım seçeneği ya da dışarıya bağ); report.left hangilerini ve nedenini söyler.")


# The duplex's objects on the ground floor (repeated with it) and the loose plan of the ground floor with the duplex
# repeated, and without it (no module, or its module removed).
REPEATED = set(DUPLEX)
WITH_DUPLEX = loose_plan("zemin", REPEATED, REPEATED)
NO_MODULE = loose_plan("zemin", set(), set())
REMOVED = loose_plan("zemin", REPEATED, set())


# ── cad.modules.edit ──────────────────────────────────────────────────────────────────────────────────────────────


def define_cases():
    out = []
    out.append(case("Dubleks: iki kattaki nesneler iki katlı modül olur; taban en alçak kat", [
        run({"kind": "define", "name": "Dubleks", "uids": [UID(i) for i in DUPLEX], "base": P(0, 0)},
            done({"module": defined(), "instance": "dubleks-1", "report": {}, "revision": "$current"}),
            {"ids": IDS, "revision": "changed", "canUndo": True})]))
    one = defined(ONE, "Tek", "tek")
    out.append(case("Tek kattaki nesneler tek katlı modül; kat alanı yazılmaz", [
        run({"kind": "define", "name": "Tek", "uids": [UID(i) for i in ONE], "base": P(0, 0)},
            done({"module": one, "instance": "tek-1", "report": {}, "revision": "$current"}))]))
    basement = defined([26, 1, 2], "Bodrumlu", "bodrumlu")
    for m in basement["members"]:
        m.pop("storey", None)
        if m["key"] in (UID(1), UID(2)):
            m["storey"] = 1
    basement["instances"][0]["level"] = "bodrum"
    out.append(case("Bodrum ve zemin: taban bodrum, zemin ikinci kat (katlar kotlarına göre)", [
        run({"kind": "define", "name": "Bodrumlu", "uids": [UID(26), UID(1), UID(2)], "base": P(0, 0)},
            done({"module": basement, "instance": "bodrumlu-1", "report": {}, "revision": "$current"}))]))
    out.append(case("Ret: katlı ve katsız duvar birlikte", [run(
        {"kind": "define", "name": "Karışık", "uids": [UID(1), UID(24)], "base": P(0, 0)},
        refusal("module_levels", "Modülün nesneleri ya hepsi projenin katlarında ya da hepsi katsız olur; katsızlara kat "
                                 "verin ya da onları seçimden çıkarın.", "operation.uids[1]"), {"revision": "same"})]))
    out.append(case("Ret: projede olmayan kattaki duvar", [run(
        {"kind": "define", "name": "Yok", "uids": [UID(1), UID(25)], "base": P(0, 0)},
        refusal("module_levels", "Nesnenin katı “yok” projede yok; önce ona projenin bir katını verin.",
                "operation.uids[1]"))]))
    out.append(case("Katsız tek duvar katsız modül olur", [
        run({"kind": "define", "name": "Katsız", "uids": [UID(24)], "base": P(0, 0)},
            done({"module": "$any", "instance": "katsiz-1", "report": {}, "revision": "$current"}))]))
    return out


def place_cases():
    out = []
    a = Placed(2, NEXT, (20, 10), "zemin")
    out.append(case("Yerleştir: zemin tabanlı dubleksin üst katı 1. kata", [
        DEFINE(), run(PLACE(a), done("$any"), {"ids": IDS + sorted(a.slots.values()), "fields": a.fields()},
                      within=1e-9)]))
    b = Placed(2, NEXT, (20, 10), "kat1", {"prefix": "B-"})
    out.append(case("Yerleştir: 1. kat tabanlı dubleksin üst katı 2. kata, numaralar önekli", [
        DEFINE(), run(PLACE(b), done("$any"), {"fields": b.fields()}, within=1e-9)]))
    c = Placed(2, NEXT, (20, 10), "bodrum")
    out.append(case("Yerleştir: bodrum tabanlı dubleksin üst katı zemine (listenin sırası değil, kotlar)", [
        DEFINE(), run(PLACE(c), done("$any"), {"fields": c.fields()}, within=1e-9)]))
    out.append(case("Ret: en üst kata dubleks sığmaz", [
        DEFINE(), run(PLACE(Placed(2, NEXT, (20, 10), "kat2")),
                      refusal("module_storey_missing", storey_missing("Dubleks", 2, "kat2", 0), "operation.level"),
                      {"revision": "same", "ids": IDS})]))
    out.append(case("Ret: katsız yerleşen dubleks", [
        DEFINE(), run({"kind": "place", "module": "Dubleks", "at": P(20, 10)},
                      refusal("module_storey_missing", "“Dubleks” modülü 2 katlı; taban katı verilmeli. Bir kat seçin.",
                              "operation.level"))]))
    one = Placed(2, NEXT, (20, 10), "kat2", keys=ONE, mid="tek")
    out.append(case("Tek katlı modül en üst kata da yerleşir", [
        DEFINE(ONE, "Tek"), run(PLACE(one, "Tek"), done("$any"), {"fields": one.fields()}, within=1e-9)]))
    out.append(case("Yerleştir ve geri al: tek adım", [
        DEFINE(), run(PLACE(a), ANY), {"op": "undo", "returns": STEP, "expect": {"ids": IDS}}]))
    return out


def edit_cases_storeys():
    out = []
    a = Placed(2, NEXT, (20, 10), "bodrum")
    # The source's first-floor wall moved 0.5 m right; the definition takes it, the instance in the basement follows on
    # its own upper storey (the ground floor).
    out.append(case("Güncelle: üst kattaki duvarın değişikliği öbür örneğin üst katına gider", [
        DEFINE(), run(PLACE(a), ANY),
        other("cad.entities.transform", {"uids": [UID(4)], "transform": {"kind": "move", "dx": 0.5, "dy": 0.0}}),
        run({"kind": "update", "module": "Dubleks", "from": "dubleks-1"},
            done({"module": "$any", "instance": "dubleks-1", "changed": "$any",
                  "report": {"updated": ["dubleks-1", "dubleks-2"], "changedKeys": [UID(4)]}, "revision": "$current"},
                 "$any"),
            {"fields": {str(a.slots[4]): {"pts": [P(24.5, 10), P(24.5, 13)], "arch": {"level": "zemin"}}}},
            within=1e-9)]))
    nb = Placed(2, NEXT, (20, 10), "bodrum", {"prefix": "B-"})
    # The tag's text the document writes with the instance's number (“B-D2”) is no change of the source.
    out.append(case("Güncelle: numaralı yeni örnek kaynakla aynı; etiketin numaralı yazısı değişiklik sayılmaz", [
        DEFINE(), run(PLACE(nb), ANY),
        run({"kind": "update", "module": "Dubleks", "from": "dubleks-2"},
            refusal("nothing_to_set", "“dubleks-2” örneği kaynakla aynı; güncellenecek bir şey yok.", "operation.from"),
            {"revision": "same"})]))
    out.append(case("Güncelle: kaynağın zemindeki duvarı 1. kata alınınca tanımda ikinci kat olur", [
        DEFINE(), run(PLACE(a), ANY),
        other("cad.levels.assign", {"level": "kat1", "uids": [UID(2)]}),
        run({"kind": "update", "module": "Dubleks", "from": "dubleks-1"},
            done({"module": "$any", "instance": "dubleks-1", "changed": "$any",
                  "report": {"updated": ["dubleks-1", "dubleks-2"], "changedKeys": [UID(2)]}, "revision": "$current"},
                 "$any"),
            {"fields": {str(a.slots[2]): {"arch": {"level": "zemin"}}, str(a.slots[1]): {"arch": {"level": "bodrum"}}}})]))
    out.append(case("Kaynağa döndür: elle başka kata alınan üst kat duvarı kendi katına döner", [
        DEFINE(), run(PLACE(a), ANY),
        other("cad.levels.assign", {"level": "kat1", "uids": [a.uid(4)]}),
        run({"kind": "restore", "module": "Dubleks", "uids": [a.uid(4)]}, done("$any"),
            {"fields": {str(a.slots[4]): {"arch": {"level": "zemin"}}}})]))
    out.append(case("Bağımsızlaştır: iki katlı örneğin nesneleri katlarında kalır", [
        DEFINE(), run(PLACE(a), ANY),
        run({"kind": "detach", "module": "Dubleks", "instance": "dubleks-2"}, done("$any"),
            {"fields": {str(a.slots[3]): {"arch": {"level": "zemin"}}, str(a.slots[1]): {"arch": {"level": "bodrum"}}}})]))
    one = defined(ONE, "Tek", "tek")
    one["version"] = 2
    one["members"].append({"key": UID(21), "entity": definition_entity(21), "storey": 1})
    one["instances"][0]["version"] = 2
    one["instances"][0]["members"].append({"key": UID(21), "uid": UID(21)})
    t = Placed(2, NEXT, (20, 10), "bodrum", keys=ONE, mid="tek")
    rec = t.record(2)
    rec["members"].append({"key": UID(21), "uid": f"$uidOf:{t.last() + 1}"})
    one["instances"].append(rec)
    out.append(case("Modüle ekle: üst kattaki duvar ikinci kat olur, bodrumdaki örneğe zemine gelir", [
        DEFINE(ONE, "Tek"), run(PLACE(t, "Tek"), ANY),
        run({"kind": "addMembers", "module": "Tek", "instance": "tek-1", "uids": [UID(21)]},
            done({"module": one, "instance": "tek-1", "created": [f"$uidOf:{t.last() + 1}"],
                  "report": {"updated": ["tek-1", "tek-2"], "addedKeys": [UID(21)]}, "revision": "$current"}),
            {"fields": {str(t.last() + 1): {"pts": [P(40, 10), P(42, 10)], "arch": {"level": "zemin"}}}},
            within=1e-9)]))
    out.append(case("Ret: modüle bodrumdaki duvar eklenmez (taban katının altı)", [
        DEFINE(ONE, "Tek"),
        run({"kind": "addMembers", "module": "Tek", "instance": "tek-1", "uids": [UID(26)]},
            refusal("module_levels", "Eklenen nesne örneğin katlarında değil; örneğin taban katındaki ya da üstündeki "
                                     "katlardaki nesneleri ekleyin.", "operation.uids[0]"))]))
    top = Placed(2, NEXT, (20, 10), "kat2", keys=ONE, mid="tek")
    out.append(case("Ret: üst kat eklenince en üst kattaki örneğin karşılığı yok", [
        DEFINE(ONE, "Tek"), run(PLACE(top, "Tek"), ANY),
        run({"kind": "addMembers", "module": "Tek", "instance": "tek-1", "uids": [UID(21)]},
            refusal("module_storey_missing", storey_missing("Tek", 2, "kat2", 0), "operation.uids"),
            {"revision": "same"})]))
    return out


def repeat_cases():
    out = []
    t1 = {"level": "kat1"}
    a = Placed(2, NEXT, (0, 0), "kat1")
    out.append(case("Katları tekrarla: zemindeki dubleks 1. kata, üst katı 2. kata", [
        DEFINE(), run(REPEAT([t1], loose=False),
                      done({"module": "$any", "created": [a.uid(m) for m in DUPLEX],
                            "placed": [{"module": "dubleks", "instance": "dubleks-2", "level": "kat1"}],
                            "report": {}, "revision": "$current"}),
                      {"fields": a.fields()}, within=1e-9)]))
    out.append(case("Ret: dubleks 2. kata tekrarlanamaz", [
        DEFINE(), run(REPEAT([{"level": "kat2"}], loose=False),
                      refusal("module_storey_missing", storey_missing("Dubleks", 2, "kat2", 0), "operation.to[0].level"),
                      {"revision": "same"})]))
    copies, left = WITH_DUPLEX
    c = Copies(copies, a.last() + 1, "kat1", instance=a)
    out.append(case("Katları tekrarla, modülsüz nesneler de: hol, kapısı, mahali, etiketi, ölçüleri, donatı", [
        DEFINE(), run(REPEAT([t1]),
                      done({"module": "$any", "created": [a.uid(m) for m in DUPLEX] + [f"$uidOf:{c.slots[i]}" for i in copies],
                            "placed": [{"module": "dubleks", "instance": "dubleks-2", "level": "kat1"}],
                            "copied": c.copied(), "report": {"left": left}, "revision": "$current"},
                           [left_warning(len(left))]),
                      {"ids": IDS + list(range(NEXT, c.slots[copies[-1]] + 1)), "fields": c.fields()}, within=1e-9)]))
    out.append(case("Modülsüz kopyalar: dubleksin duvarına bağlı ölçü tekrarlanan örneğin duvarına bağlanır", [
        DEFINE(), run(REPEAT([t1]), done("$any", "$any"), {"fields": {str(c.slots[20]): c.fields()[str(c.slots[20])]}})]))
    n = {"level": "kat1", "numbering": {"prefix": "A1-"}}
    an = Placed(2, NEXT, (0, 0), "kat1", {"prefix": "A1-"})
    cn = Copies(copies, an.last() + 1, "kat1", numbering={"prefix": "A1-"}, instance=an)
    out.append(case("Modülsüz kopyalar: katın öneki numaraların önekinin yerine geçer", [
        DEFINE(), run(REPEAT([n]), done("$any", "$any"),
                      {"fields": {str(cn.slots[15]): {"number": "A1-101"}, str(cn.slots[14]): {"number": "A1-K5"},
                                  str(an.slots[7]): {"number": "A1-D1"}}})]))
    out.append(case("Modülsüz kopyalar: numara verilmezse kat öneki değişir (Z- → 1-), kapınınki kalır", [
        DEFINE(), run(REPEAT([t1]), done("$any", "$any"),
                      {"fields": {str(c.slots[15]): {"number": "1-101"}, str(c.slots[14]): {"number": "K5"}}})]))
    b = Placed(3, a.last() + 1 + len(copies), (0, 0), "bodrum")
    cb = Copies(copies, b.last() + 1, "bodrum", instance=b)
    out.append(case("Katları tekrarla: iki hedef; her biri önce örnekler sonra kopyalar", [
        DEFINE(), run(REPEAT([t1, {"level": "bodrum"}]),
                      done({"module": "$any", "created": "$any",
                            "placed": [{"module": "dubleks", "instance": "dubleks-2", "level": "kat1"},
                                       {"module": "dubleks", "instance": "dubleks-3", "level": "bodrum"}],
                            "copied": c.copied() + cb.copied(), "report": {"left": left}, "revision": "$current"},
                           [left_warning(len(left))]),
                      {"fields": dict(cb.fields(), **{str(b.slots[3]): {"arch": {"level": "zemin"}}}),
                       "ids": IDS + list(range(NEXT, cb.slots[copies[-1]] + 1))}, within=1e-9)]))
    out.append(case("Bodruma kopyalanan hol mahali numarasını korur (bodrumun öneki yok)", [
        DEFINE(), run(REPEAT([{"level": "bodrum"}]), done("$any", "$any"),
                      {"fields": {str(Copies(copies, Placed(2, NEXT, (0, 0), "bodrum").last() + 1, "bodrum").slots[15]):
                                  {"number": "Z-101"}}})]))
    plain, plain_left = NO_MODULE
    cp = Copies(plain, NEXT, "kat2")
    out.append(case("Modülsüz çizimde katın bütün nesneleri kopyalanır", [
        run(REPEAT([{"level": "kat2"}]),
            done({"created": [f"$uidOf:{cp.slots[i]}" for i in plain], "copied": cp.copied(),
                  "report": {"left": plain_left}, "revision": "$current"}, [left_warning(len(plain_left))]),
            {"fields": cp.fields(), "ids": IDS + list(range(NEXT, NEXT + len(plain)))}, within=1e-9)]))
    rm, rm_left = REMOVED
    cr = Copies(rm, NEXT, "kat1")
    out.append(case("Kaynağı silinmiş dubleksin nesneleri kopyalanmaz; ona bağlı ölçü de kalır", [
        DEFINE(), run({"kind": "remove", "module": "Dubleks"}, ANY),
        run(REPEAT([t1]),
            done({"created": [f"$uidOf:{cr.slots[i]}" for i in rm], "copied": cr.copied(),
                  "report": {"left": rm_left}, "revision": "$current"}, [left_warning(len(rm_left))]),
            {"fields": cr.fields()})]))
    lk_copies, lk_left = loose_plan("zemin", REPEATED, REPEATED, locked={"mobilya"})
    out.append(case("Kilitli katmandaki donatı kopyalanmaz, söylenir", [
        DEFINE(), LOCK("mobilya"),
        run(REPEAT([t1]),
            done({"module": "$any", "created": "$any", "placed": "$any",
                  "copied": Copies(lk_copies, a.last() + 1, "kat1", instance=a).copied(),
                  "report": {"left": lk_left}, "revision": "$current"}, [left_warning(len(lk_left))]))]))
    op_copies, op_left = loose_plan("zemin", REPEATED, REPEATED, in_options={18})
    out.append(case("Tasarım seçeneğindeki donatı kopyalanmaz (ana modele katılırdı), söylenir", [
        DEFINE(),
        other("cad.options.edit", {"operation": {"kind": "define", "name": "Hol düzeni", "uids": [UID(18)]}}),
        run(REPEAT([t1]),
            done({"module": "$any", "created": "$any", "placed": "$any",
                  "copied": Copies(op_copies, a.last() + 1, "kat1", instance=a).copied(),
                  "report": {"left": op_left}, "revision": "$current"}, [left_warning(len(op_left))]))]))
    out.append(case("Katları tekrarla'nın planı yazmaz; kopyaları söyler", [
        DEFINE(),
        {"op": "plan", "input": {"operation": REPEAT([t1])},
         # The plan writes nothing: its copies' ids are fresh ones, not the drawing's slots.
         "result": done({"module": "$any", "created": "$any", "placed": "$any", "copied": "$any",
                         "report": {"left": left}, "revision": "$current"}, [left_warning(len(left))]),
         "expect": {"revision": "same", "ids": IDS}}]))
    out.append(case("Modülsüz nesnelerle tekrar ve geri al: tek adım", [
        DEFINE(), run(REPEAT([t1]), ANY), {"op": "undo", "returns": STEP, "expect": {"ids": IDS}}]))
    out.append(case("Ret: kaynak katta ne örnek ne modülsüz nesne var", [
        DEFINE(), run(REPEAT([{"level": "bodrum"}], source="kat2"),
                      refusal("module_repeat_empty",
                              "“kat2” katında tekrarlanacak bağlı modül örneği ya da modülsüz nesne yok.", "operation.from"))]))
    out.append(case("Ret: modülsüz nesne istenmezse modülsüz katta örnek yok", [
        run(REPEAT([{"level": "kat2"}], loose=False),
            refusal("module_repeat_empty", "“zemin” katında tekrarlanacak bağlı modül örneği yok.", "operation.from"))]))
    k1, k1_left = loose_plan("kat1", set(), set())
    ck = Copies(k1, NEXT, "kat2", source="kat1")
    out.append(case("1. kattan 2. kata: modülsüz çizimde üst katın nesneleri", [
        run(REPEAT([{"level": "kat2"}], source="kat1"),
            done({"created": [f"$uidOf:{ck.slots[i]}" for i in k1], "copied": ck.copied(),
                  "report": {"left": k1_left} if k1_left else {}, "revision": "$current"},
                 [left_warning(len(k1_left))] if k1_left else []),
            {"fields": ck.fields()}, within=1e-9)]))
    return out


def edit_cases():
    return define_cases() + place_cases() + edit_cases_storeys() + repeat_cases()


# ── cad.modules.query ─────────────────────────────────────────────────────────────────────────────────────────────


def query_cases():
    out = []
    E = "cad.modules.edit"

    def q(name, inp, result, *before):
        return case(name, list(before) + [{"op": "execute", "input": inp, "result": result}])

    def ed(op):
        return {"op": "execute", "command": E, "input": {"operation": op}, "result": ANY}

    de = dict(DEFINE(), command=E)
    de_one = dict(DEFINE(ONE, "Tek"), command=E)

    def irow(id, level, levels=(), version=1, objects=11, drifted=(), exceptions=(), stale=False, broken=None):
        r = {"id": id, "version": version, "stale": stale, "objects": objects}
        if exceptions:
            r["exceptions"] = list(exceptions)
        if drifted:
            r["drifted"] = list(drifted)
        if broken:
            r["broken"] = broken
        if level:
            r["level"] = level
        if levels:
            r["levels"] = list(levels)
        r["mirror"] = False
        return r

    def row(instances, storeys=2, version=1, members=11, removed=False, mid="dubleks", name="Dubleks"):
        r = {"id": mid, "name": name, "version": version, "members": members}
        if storeys > 1:
            r["storeys"] = storeys
        r["removed"] = removed
        r["instances"] = instances
        return r

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

    def levels(base):
        return [base, up(base, 1)] if up(base, 1) else [base]

    first = irow("dubleks-1", "zemin", levels("zemin"))
    a = Placed(2, NEXT, (20, 10), "bodrum")
    second = irow("dubleks-2", "bodrum", levels("bodrum"))
    move = lambda uid: other("cad.entities.transform",  # noqa: E731
                             {"uids": [uid], "transform": {"kind": "move", "dx": 0.5, "dy": 0.0}})
    out.append(q("İki katlı modül: katları ve örneğin katları", {}, ans([row([first])]), de))
    out.append(q("Tek katlı modülde kat sayısı ve katlar yazılmaz", {},
                 ans([row([irow("tek-1", "zemin", objects=3)], storeys=1, members=3, mid="tek", name="Tek")]), de_one))
    out.append(q("Bodrumda yerleşen örneğin katları bodrum ve zemin", {}, ans([row([first, second])]), de, ed(PLACE(a))))
    k1 = Placed(2, NEXT, (20, 10), "kat1")
    out.append(q("1. katta yerleşen örneğin katları 1. ve 2. kat", {},
                 ans([row([first, irow("dubleks-2", "kat1", levels("kat1"))])]), de, ed(PLACE(k1))))
    out.append(q("Tekrarlanan katın örneği", {}, ans([row([first, irow("dubleks-2", "kat1", levels("kat1"))])]),
                 de, ed(REPEAT([{"level": "kat1"}], loose=False))))
    out.append(q("Modülsüz nesnelerle tekrar örnek sayısını değiştirmez", {},
                 ans([row([first, irow("dubleks-2", "kat1", levels("kat1"))])]), de, ed(REPEAT([{"level": "kat1"}]))))
    out.append(q("Kimlikle tek modül", {"module": "dubleks"}, ans([row([first, second])]), de, ed(PLACE(a))))
    out.append(q("Adla tek modül", {"module": "Dubleks"}, ans([row([first])]), de))
    out.append(q("Üst kattaki duvarın örneği ve anahtarı", {"uid": a.uid(3)},
                 ans([row([first, second])], "dubleks-2", UID(3)), de, ed(PLACE(a))))
    out.append(q("Elle taşınan üst kat duvarı elle değişmiş", {"uid": a.uid(4)},
                 ans([row([first, irow("dubleks-2", "bodrum", levels("bodrum"), drifted=[a.uid(4)])])],
                     "dubleks-2", UID(4), True), de, ed(PLACE(a)), move(a.uid(4))))
    out.append(q("Başka kata alınan üst kat duvarı elle değişmiş", {},
                 ans([row([first, irow("dubleks-2", "bodrum", levels("bodrum"), drifted=[a.uid(4)])])]),
                 de, ed(PLACE(a)), other("cad.levels.assign", {"level": "kat1", "uids": [a.uid(4)]})))
    out.append(q("Başka kata alınıp kaynağa döndürülen artık değişmiş değil", {}, ans([row([first, second])]),
                 de, ed(PLACE(a)), other("cad.levels.assign", {"level": "kat1", "uids": [a.uid(4)]}),
                 ed({"kind": "restore", "module": "Dubleks", "uids": [a.uid(4)]})))
    out.append(q("Yerel istisna yapılan üst kat duvarı değişmiş sayılmaz", {},
                 ans([row([first, irow("dubleks-2", "bodrum", levels("bodrum"), exceptions=[a.uid(4)])])]),
                 de, ed(PLACE(a)), move(a.uid(4)), ed({"kind": "except", "module": "Dubleks", "uids": [a.uid(4)]})))
    out.append(q("Kaynaktan güncellenince üst katta değişen kalmaz", {},
                 ans([row([irow("dubleks-1", "zemin", levels("zemin"), version=2),
                           irow("dubleks-2", "bodrum", levels("bodrum"), version=2)], version=2)]),
                 de, ed(PLACE(a)), move(UID(4)), ed({"kind": "update", "module": "Dubleks", "from": "dubleks-1"})))
    out.append(q("Zemindeki duvar kata alınıp güncellenince modül yine iki katlı", {},
                 ans([row([irow("dubleks-1", "zemin", levels("zemin"), version=2),
                           irow("dubleks-2", "bodrum", levels("bodrum"), version=2)], version=2)]),
                 de, ed(PLACE(a)), other("cad.levels.assign", {"level": "kat1", "uids": [UID(2)]}),
                 ed({"kind": "update", "module": "Dubleks", "from": "dubleks-1"})))
    out.append(q("Tek katlıya üst kat eklenince iki katlı olur", {},
                 ans([row([irow("tek-1", "zemin", levels("zemin"), version=2, objects=4)], version=2, members=4,
                          mid="tek", name="Tek")]),
                 de_one, ed({"kind": "addMembers", "module": "Tek", "instance": "tek-1", "uids": [UID(21)]})))
    upper = [UID(i) for i in (3, 4, 6, 8, 9, 10, 11)]
    out.append(q("Üst katın bütün üyeleri çıkarılınca tek katlı olur", {},
                 ans([row([irow("dubleks-1", "zemin", version=2, objects=4)], storeys=1, version=2, members=4)]),
                 de, ed({"kind": "removeMembers", "module": "Dubleks", "uids": upper})))
    out.append(q("Kaynağı silinen iki katlı modülün kat sayısı yazılmaz", {},
                 ans([row([irow("dubleks-1", "zemin", broken="sourceGone")], storeys=1, members=0, removed=True)]),
                 de, ed({"kind": "remove", "module": "Dubleks"})))
    out.append(q("Bağımsızlaştırılan örnek düşer", {}, ans([row([first])]),
                 de, ed(PLACE(a)), ed({"kind": "detach", "module": "Dubleks", "instance": "dubleks-2"})))
    out.append(q("Modülsüz proje", {}, ans([])))
    out.append(q("Bodrumdan başlayan modül: katları bodrum ve zemin", {},
                 ans([row([irow("bodrumlu-1", "bodrum", levels("bodrum"), objects=3)], members=3, mid="bodrumlu",
                          name="Bodrumlu")]),
                 {"op": "execute", "command": E, "input": {"operation": {
                     "kind": "define", "name": "Bodrumlu", "uids": [UID(26), UID(1), UID(2)], "base": P(0, 0)}},
                  "result": ANY}))
    out.append(q("Geri alınan yerleştirme sayılmaz", {}, ans([row([first])]),
                 de, ed(PLACE(a)), {"op": "undo", "returns": STEP}))
    out.append(q("Zemindeki kapının örneği; elle değişmemiş", {"uid": a.uid(5)},
                 ans([row([first, second])], "dubleks-2", UID(5)), de, ed(PLACE(a))))
    return out


def sample():
    d = setup()
    d["name"] = "Çok katlı modüller"
    d["settings"]["arch"]["modules"] = [defined()]
    return d


def write(command, title, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "scripts/fixtures/module_storeys_cases.py yazar: ADR 0796'nın kurallarından, KentOS kodu olmadan. Değiştirmeyin; betiği düzeltin.",
            "setup": setup(), "cases": cases}


def main():
    files = {
        EDIT: write("cad.modules.edit", "Çok katlı oda modülleri ve modülsüz nesnelerin kat tekrarı (docs/adr/0796)",
                    edit_cases()),
        QUERY: write("cad.modules.query", "Çok katlı oda modüllerinin katları (docs/adr/0796)", query_cases()),
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
