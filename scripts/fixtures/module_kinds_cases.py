#!/usr/bin/env python3
"""Room modules of more kinds (docs/adr/8050 §1): a curved wall, columns (a rectangle and an L), a slab, a stair and a
ramp in a module, from the ADR's rules and the transforms' own (ADR 0339 §1 for a wall's path, ADR 0725 §2, §3 for a
column, ADR 0709 for a curved stair), without KentOS code.

Writes fixtures/commands/v1/cad.modules.edit.kinds.json; `--check` compares and writes nothing.

An instance places its definition: each point mirrored across the definition's own y axis when asked (x → −x), turned
by the instance's turn, moved to its base. A wall's path mirrored runs back so its faces keep their sides, an arc's
bulge staying (mirrored, then run back); a column as `arch_command_cases.mirrored_column` across the y axis, then
turned (its turn plus the instance's) and moved; a slab's ring point by point; a stair's walking line point by point,
a mirror turning its arc over; a ramp's centre line point by point. Every object goes on the instance's level. A column
tied to grids is refused: it stands where the building's grids cross.

    python3 scripts/fixtures/module_kinds_cases.py
    python3 scripts/fixtures/module_kinds_cases.py --check
"""

import argparse
import copy
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import arch_command_cases as ac  # noqa: E402  (a column mirrored, independently: ADR 0725 §2, §3)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.modules.edit.kinds.json"
ANY = {"status": "completed", "output": "$any", "warnings": "$any"}


def UID(n):
    return f"0192f5a1-8050-7000-8000-{n:012x}"


def P(x, y):
    return {"x": float(x), "y": float(y)}


def layer(id, name):
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def on(level):
    return {"arch": {"level": level}}


def setup_entities():
    return [
        {"kind": "wall", "id": 1, "layerId": "duvar", "attrs": {}, "pts": [P(0, 0), P(4, 0)], "bulges": [0.3, 0.0],
         "width": 0.2, "justify": "center", **on("zemin")},
        {"kind": "column", "id": 2, "layerId": "kolon", "attrs": {}, "at": P(1, 2), "width": 0.4, "depth": 0.3,
         "rotation": 0.2, "anchor": "bottomLeft", **on("zemin")},
        {"kind": "column", "id": 3, "layerId": "kolon", "attrs": {}, "at": P(3, 2), "shape": "lShape", "width": 0.6,
         "depth": 0.5, "legs": [0.2, 0.25], "anchor": "bottomRight", **on("zemin")},
        {"kind": "slab", "id": 4, "layerId": "doseme", "attrs": {},
         "parts": [{"outer": {"pts": [P(0, 0), P(4, 0), P(4, 3), P(0, 3)]}}], "thickness": 0.15, **on("zemin")},
        # A stair whose rise is its level's height (ADR 0724 §1) and a ramp starting at its level's elevation
        # (ADR 0723 §2): the document writes the riser height and the start for each instance's storey.
        {"kind": "stair", "id": 5, "layerId": "mimari", "attrs": {}, "pts": [P(0.5, 0.5), P(0.5, 4.5)],
         "bulges": [0.2, 0.0], "width": 1.0, "risers": 18, "riserHeight": 3.0 / 18, "tread": 0.22,
         "heightFromLevel": True, **on("zemin")},
        {"kind": "ramp", "id": 6, "layerId": "mimari", "attrs": {}, "pts": [P(2, 0.5), P(3.5, 0.5)], "width": 1.2,
         "rise": 0.3, "slope": 20.0, "base": 0.0, "baseOffset": 0.0, **on("zemin")},
        # A column tied to two grids where they cross.
        {"kind": "column", "id": 7, "layerId": "kolon", "attrs": {}, "at": P(6, 0), "width": 0.4,
         "grids": [UID(8), UID(9)], "place": "A-1", **on("zemin")},
        {"kind": "grid", "id": 8, "layerId": "aks", "attrs": {}, "pts": [P(5, 0), P(8, 0)], "name": "A"},
        {"kind": "grid", "id": 9, "layerId": "aks", "attrs": {}, "pts": [P(6, -1), P(6, 2)], "name": "1"},
        # A free column outside the module, for Modüle ekle.
        {"kind": "column", "id": 10, "layerId": "kolon", "attrs": {}, "at": P(2, 2.6), "width": 0.3, **on("zemin")},
    ]


ENTS = {e["id"]: e for e in setup_entities()}
# The levels: their elevations and heights (1. kat 3.24 m high, its stair's risers 18 cm).
LEVELS = {"zemin": (0.0, 3.0), "kat1": (3.0, 3.24)}
MEMBERS = [1, 2, 3, 4, 5, 6]
NEXT = 11  # the first slot a placement fills
STEP = "Oda modülü"


def setup():
    ents = setup_entities()
    return {
        "format": "kentos.document", "version": 2, "name": "Modül türleri",
        "uids": [UID(e["id"]) for e in ents],
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 50, "workspace": "cad",
                     "arch": {"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
                                         {"id": "kat1", "name": "1. kat", "elevation": 3.0, "height": 3.24}]}},
        "origin": {"x": 0, "y": 0},
        "layers": [layer("duvar", "Duvar"), layer("kolon", "Kolon"), layer("doseme", "Döşeme"),
                   layer("mimari", "Mimari"), layer("aks", "Aks")],
        "activeLayer": "duvar",
        "styles": {"items": [], "categories": []},
        "entities": ents,
    }


def definition_member(i, ents=None):
    """A member as the definition keeps it: the object at the origin's base, no slot, no level."""
    e = copy.deepcopy((ents or ENTS)[i])
    e["id"] = 0
    e["arch"].pop("level", None)
    if not e["arch"]:
        del e["arch"]
    return {"key": UID(i), "entity": e}


def first_instance(version=1):
    return {"id": "oda-1", "at": P(0, 0), "level": "zemin", "version": version,
            "members": [{"key": UID(i), "uid": UID(i)} for i in MEMBERS]}


def defined(ents=None, version=1):
    return {"id": "oda", "name": "Oda", "version": version,
            "members": [definition_member(i, ents) for i in MEMBERS], "instances": [first_instance(version)]}


def changed_ents(i, **fields):
    """The setup's objects with object `i`'s fields changed, as the source instance's edit leaves them."""
    ents = copy.deepcopy(ENTS)
    ents[i].update(fields)
    return ents


def matrix(at, rot, mirror):
    c, s = math.cos(rot), math.sin(rot)

    def f(p):
        x, y = p
        if mirror:
            x = -x
        return (c * x - s * y + at[0], s * x + c * y + at[1])
    return f


def xy(p):
    return (p["x"], p["y"])


class Placed:
    """An instance placed: its record, and its objects' slots by member."""

    def __init__(self, n, at, rot=0.0, mirror=False, level=None, first=NEXT):
        self.n, self.at, self.rot, self.mirror, self.level = n, at, rot, mirror, level
        self.slots = {m: first + k for k, m in enumerate(MEMBERS)}
        self.f = matrix(at, rot, mirror)

    def id(self):
        return f"oda-{self.n}"

    def uid(self, m):
        return f"$uidOf:{self.slots[m]}"

    def record(self, version=1):
        r = {"id": self.id(), "at": P(*self.at)}
        if self.rot:
            r["rotation"] = self.rot
        if self.mirror:
            r["mirror"] = True
        if self.level:
            r["level"] = self.level
        r["version"] = version
        r["members"] = [{"key": UID(m), "uid": self.uid(m)} for m in MEMBERS]
        return r

    def pts(self, e):
        return [P(*self.f(xy(p))) for p in e["pts"]]

    def column(self, e):
        """A column placed: mirrored across the definition's y axis when asked (ADR 0725 §2, §3), then turned and
        moved."""
        g = ac.mirrored_column(e, P(0, 0), P(0, 1)) if self.mirror else copy.deepcopy(e)
        # Turned by the instance: its point (mirrored already) about the origin, then moved; its turn plus the
        # instance's.
        c, s = math.cos(self.rot), math.sin(self.rot)
        x, y = xy(g["at"])
        rot = ac.norm_angle(g.get("rotation", 0.0) + self.rot)
        # No turn is written as none.
        out = {"at": P(c * x - s * y + self.at[0], s * x + c * y + self.at[1]), "rotation": rot or None,
               "anchor": g.get("anchor", "center"), "width": g["width"]}
        if "depth" in g:
            out["depth"] = g["depth"]
        if "legs" in g:
            out["legs"] = g["legs"]
        return out

    def fields(self, ents=None):
        """What the placement writes, object by object, of the definition's objects `ents` (the setup's)."""
        ents = ents or ENTS
        level = {"level": self.level} if self.level else None
        out = {}
        w = ents[1]
        wall_pts = self.pts(w)
        if self.mirror:
            # Mirrored, the path runs back so its faces keep their sides; the arc's bulge stays (mirrored, run back).
            wall_pts = wall_pts[::-1]
        out[str(self.slots[1])] = {"pts": wall_pts, "bulges": w["bulges"], "width": w["width"], "arch": level}
        for i in (2, 3):
            col = self.column(ents[i])
            out[str(self.slots[i])] = {**col, "arch": level}
        out[str(self.slots[4])] = {"parts": [{"outer": {"pts": self.pts(ents[4]["parts"][0]["outer"])}}],
                                   "thickness": ents[4]["thickness"], "arch": level}
        st = ents[5]
        # Mirrored, a stair still climbs its walking line from its first point: its arc turns the other way.
        elevation, height = LEVELS[self.level or "zemin"]
        # Its riser height its storey's height over its risers, its start its storey's elevation: the document's.
        out[str(self.slots[5])] = {"pts": self.pts(st), "bulges": [-b if self.mirror else b for b in st["bulges"]],
                                   "width": st["width"], "risers": st["risers"], "riserHeight": height / st["risers"],
                                   "arch": level}
        out[str(self.slots[6])] = {"pts": self.pts(ents[6]), "width": ents[6]["width"],
                                   "base": elevation + ents[6]["baseOffset"], "arch": level}
        return out

    def field(self, m, ents=None):
        """Member `m`'s object's fields, by its slot."""
        k = str(self.slots[m])
        return {k: self.fields(ents)[k]}


def refusal(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def done(output, warnings=None):
    return {"status": "completed", "output": output, "warnings": warnings or []}


def edited(module=None, instance=None, created=(), changed=(), report=None):
    out = {}
    if module is not None:
        out["module"] = module
    if instance is not None:
        out["instance"] = instance
    if created:
        out["created"] = list(created)
    if changed:
        out["changed"] = list(changed)
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


def warn(code, message):
    return {"code": code, "message": message, "path": "operation"}


DEFINE_OP = {"kind": "define", "name": "Oda", "uids": [UID(i) for i in MEMBERS], "base": P(0, 0)}


def place_op(p):
    op = {"kind": "place", "module": "Oda", "at": P(*p.at)}
    if p.rot:
        op["rotation"] = p.rot
    if p.mirror:
        op["mirror"] = True
    if p.level:
        op["level"] = p.level
    return op


def placed_module(p):
    m = defined()
    m["instances"].append(p.record())
    return m


def ids_with(*placements):
    return IDS + sorted(s for p in placements for s in p.slots.values())


def DEFINE():
    return run(DEFINE_OP, ANY)


def PLACE(p):
    return run(place_op(p), ANY)


def UPDATE(result, expect=None, **extra):
    return run({"kind": "update", "module": "Oda", "from": "oda-1"}, result, expect, **extra)


def case(name, steps):
    return {"name": name, "steps": steps}


IDS = [e["id"] for e in setup_entities()]


# ── An open plan (docs/adr/8050 §4; ADR 0679's own): a 10 × 8 m room of 20 cm walls, a room separator at x = 5, the
# living room and the kitchen bounded by the walls and the separator (net 4.9 × 7.8 = 38.22 m² each), and a plain
# polyline beside it. ──────────────────────────────────────────────────────────────────────────────────────────────

OPEN_WALLS = [((0, 0), (10, 0)), ((10, 0), (10, 8)), ((10, 8), (0, 8)), ((0, 8), (0, 0))]
OPEN_MEMBERS = [1, 2, 3, 4, 5, 6, 7]
OPEN_SOURCES = [UID(i) for i in range(1, 6)]


def open_entities():
    ents = [{"kind": "wall", "id": i + 1, "layerId": "duvar", "attrs": {}, "pts": [P(*a), P(*b)], "width": 0.2,
             "justify": "center", **on("zemin")} for i, (a, b) in enumerate(OPEN_WALLS)]
    ents.append({"kind": "polyline", "id": 5, "layerId": "mimari", "attrs": {}, "pts": [P(5, 0), P(5, 8)],
                 "spaceSeparator": {"level": "zemin"}})
    for i, (x0, x1, name) in ((6, (0.1, 5.0, "Salon")), (7, (5.0, 9.9, "Mutfak"))):
        ents.append({"kind": "space", "id": i, "layerId": "mimari", "attrs": {}, "sources": OPEN_SOURCES,
                     "parts": [{"outer": {"pts": [P(x0, 0.1), P(x1, 0.1), P(x1, 7.9), P(x0, 7.9)]}}],
                     "seed": P((x0 + x1) / 2, 4), "number": f"S{i - 5}", "name": name, **on("zemin")})
    ents.append({"kind": "polyline", "id": 8, "layerId": "mimari", "attrs": {}, "pts": [P(12, 0), P(12, 8)]})
    return ents


def open_setup():
    s = setup()
    ents = open_entities()
    s["name"] = "Açık plan"
    s["uids"] = [UID(e["id"]) for e in ents]
    s["layers"] = [layer("duvar", "Duvar"), layer("mimari", "Mimari")]
    s["entities"] = ents
    return s


OPEN_NEXT = 9
OPEN_DEFINE = {"kind": "define", "name": "Açık plan", "uids": [UID(i) for i in OPEN_MEMBERS], "base": P(0, 0)}


def open_placed(at, mirror, level):
    """An open plan's instance: its objects' slots by member and its matrix."""
    slots = {m: OPEN_NEXT + k for k, m in enumerate(OPEN_MEMBERS)}
    return slots, matrix(at, 0.0, mirror)


def cases():
    out = []
    # ── Tanımla ve yerleştir ────────────────────────────────────────────────────────────────────────────────────
    out.append(case("Tanımla: eğri duvar, iki kolon, döşeme, eğri merdiven ve rampa modül olur, yerinde ilk örneği kalır", [
        run(DEFINE_OP, done(edited(module=defined(), instance="oda-1")),
            {"ids": IDS, "revision": "changed"})]))
    p2 = Placed(2, (10, 0), level="kat1")
    out.append(case("Yerleştir: 1. katta 10 m sağa; her tür yerini ve katını alır", [
        DEFINE(),
        run(place_op(p2), done(edited(module=placed_module(p2), instance="oda-2",
                                      created=[p2.uid(m) for m in MEMBERS])),
            {"ids": ids_with(p2), "fields": p2.fields()}, within=1e-9)]))
    p3 = Placed(2, (20, 5), rot=math.pi / 2, mirror=True, level="zemin")
    out.append(case("Yerleştir: aynalı ve 90° döndürülmüş; duvarın yolu geri döner, kolonların hizası ve L'nin kolları "
                    "değişir, merdivenin yayı ters döner", [
                        DEFINE(),
                        run(place_op(p3), done(edited(module=placed_module(p3), instance="oda-2",
                                                      created=[p3.uid(m) for m in MEMBERS])),
                            {"fields": p3.fields()}, within=1e-9)]))
    pm = Placed(2, (10, 10), mirror=True, level="kat1")
    out.append(case("Yerleştir: yalnız aynalı", [
        DEFINE(), run(place_op(pm), done("$any"), {"fields": pm.fields()}, within=1e-9)]))
    p45 = Placed(2, (10, -10), rot=math.pi / 4, level="zemin")
    out.append(case("Yerleştir: 45° döndürülmüş; kolonun dönüşüne örneğinki eklenir", [
        DEFINE(), run(place_op(p45), done("$any"), {"fields": p45.fields()}, within=1e-9)]))
    p180 = Placed(2, (0, 20), rot=math.pi, mirror=True, level="kat1")
    out.append(case("Yerleştir: aynalı ve 180° döndürülmüş", [
        DEFINE(), run(place_op(p180), done("$any"), {"fields": p180.fields()}, within=1e-9)]))
    up = Placed(2, (10, 0), level="zemin")
    out.append(case("İki katlı modül: 1. kattaki kolon tanımda üst katta, örnekte örneğin bir üst katında", [
        other("cad.arch.edit", {"uids": [UID(2)], "operation": {"kind": "set", "level": "kat1"}}),
        run(DEFINE_OP, done("$any")),
        run(place_op(up), done("$any"), {"fields": {str(up.slots[2]): {"arch": {"level": "kat1"}},
                                                    str(up.slots[3]): {"arch": {"level": "zemin"}}}})]))
    out.append(case("Yerleştir ve geri al: tek adım", [
        DEFINE(), PLACE(p2),
        {"op": "undo", "returns": STEP, "expect": {"ids": IDS, "revision": "changed"}}]))
    out.append(case("Kaynakla aynı: aynalı ve döndürülmüş örnekten güncellenecek bir şey yok (her tür geri dönüşte "
                    "kaynağına denk)", [
                        DEFINE(), PLACE(p3),
                        run({"kind": "update", "module": "Oda", "from": "oda-2"},
                            refusal("nothing_to_set", "“oda-2” örneği kaynakla aynı; güncellenecek bir şey yok.",
                                    "operation.from"))]))
    out.append(case("Kaynakla aynı: 1. kattaki örneğin merdiveni 18 cm rıhtlı, rampası 3 m'den başlar; yine kaynakla "
                    "aynı", [
                        DEFINE(), run(place_op(p2), done("$any"), {"fields": {**p2.field(5), **p2.field(6)}},
                                      within=1e-9),
                        run({"kind": "update", "module": "Oda", "from": "oda-2"},
                            refusal("nothing_to_set", "“oda-2” örneği kaynakla aynı; güncellenecek bir şey yok.",
                                    "operation.from"))]))
    # ── Kaynağı güncelle ────────────────────────────────────────────────────────────────────────────────────────
    pa = Placed(2, (10, 0), level="kat1")
    pb = Placed(3, (20, 5), rot=math.pi / 2, mirror=True, level="zemin", first=NEXT + len(MEMBERS))
    wide = changed_ents(2, width=0.5)
    m2 = defined(wide, version=2)
    m2["instances"] += [pa.record(2), pb.record(2)]
    out.append(case("Güncelle: kaynağın kolonu 50 cm olur; düz ve aynalı-döndürülmüş örnek alır", [
        DEFINE(), PLACE(pa), PLACE(pb),
        other("cad.arch.edit", {"uids": [UID(2)], "operation": {"kind": "set", "width": 0.5}}),
        UPDATE(done(edited(module=m2, instance="oda-1", changed=[pa.uid(2), pb.uid(2)],
                           report={"updated": ["oda-1", "oda-2", "oda-3"], "changedKeys": [UID(2)]})),
               {"fields": {**pa.field(2, wide), **pb.field(2, wide)}}, within=1e-9)]))
    # In the mirrored instance the L's legs are swapped (ADR 0725 §3): its [0.25, 0.2] made [0.3, 0.2] is the
    # definition's [0.2, 0.3].
    legs = changed_ents(3, legs=[0.2, 0.3])
    out.append(case("Güncelle: aynalı örnekte L kolonun kolu kalınlaşır; kaynağa ve öbür örneğe ters kolda döner", [
        DEFINE(), PLACE(pa), PLACE(pb),
        other("cad.arch.edit", {"uids": [pb.uid(3)], "operation": {"kind": "set", "legs": [0.3, 0.2]}}),
        run({"kind": "update", "module": "Oda", "from": "oda-3"},
            done("$any"), {"fields": {"3": {"legs": [0.2, 0.3], "at": P(3, 2), "anchor": "bottomRight"},
                                      **pa.field(3, legs), **pb.field(3, legs)}}, within=1e-9)]))
    out.append(case("Güncelle: döşemenin kalınlığı", [
        DEFINE(), PLACE(pa),
        other("cad.slabs.edit", {"uids": [UID(4)], "operation": {"kind": "set", "thickness": 0.2}}),
        UPDATE(done("$any"), {"fields": pa.field(4, changed_ents(4, thickness=0.2))}, within=1e-9)]))
    out.append(case("Güncelle: eğri merdivenin basamak sayısı; aynalı örnekte yayı ters kalır", [
        DEFINE(), PLACE(pa), PLACE(pb),
        other("cad.arch.edit", {"uids": [UID(5)], "operation": {"kind": "set", "risers": 12}}),
        UPDATE(done("$any"), {"fields": {**pa.field(5, changed_ents(5, risers=12)),
                                         **pb.field(5, changed_ents(5, risers=12))}}, within=1e-9)]))
    out.append(case("Güncelle: rampanın eni", [
        DEFINE(), PLACE(pa),
        other("cad.arch.edit", {"uids": [UID(6)], "operation": {"kind": "set", "width": 1.5}}),
        UPDATE(done("$any"), {"fields": pa.field(6, changed_ents(6, width=1.5))}, within=1e-9)]))
    moved = changed_ents(1, pts=[P(0.5, 0), P(4.5, 0)])
    out.append(case("Güncelle: eğri duvar 50 cm sağa; döndürülmüş aynalı örnekte kendi yönünde kayar", [
        DEFINE(), PLACE(pa), PLACE(pb),
        other("cad.entities.transform", {"uids": [UID(1)], "transform": {"kind": "move", "dx": 0.5, "dy": 0.0}}),
        UPDATE(done("$any"), {"fields": {**pa.field(1, moved), **pb.field(1, moved)}}, within=1e-9)]))
    turned = changed_ents(2, rotation=0.5)
    out.append(case("Güncelle: kolonun dönüşü; aynalı örnekte aynanın dönüşüyle", [
        DEFINE(), PLACE(pa), PLACE(pb),
        other("cad.arch.edit", {"uids": [UID(2)], "operation": {"kind": "set", "rotation": 0.5}}),
        UPDATE(done("$any"), {"fields": {**pa.field(2, turned), **pb.field(2, turned)}}, within=1e-9)]))
    # ── İstisna, ayır, elle değişen ─────────────────────────────────────────────────────────────────────────────
    out.append(case("İstisna: örneğin kolonu kaynağın değişikliğini almaz", [
        DEFINE(), PLACE(pa),
        run({"kind": "except", "module": "Oda", "uids": [pa.uid(2)]}, ANY),
        other("cad.arch.edit", {"uids": [UID(2)], "operation": {"kind": "set", "width": 0.5}}),
        UPDATE(done("$any", [warn("module_exception_kept",
                                  "1 nesne yerel istisna olduğu için kaynağın değişikliğini almadı.")]),
               {"fields": pa.field(2)}, within=1e-9)]))
    out.append(case("Ayır: ayrılan örneğin döşemesi kaynağı izlemez", [
        DEFINE(), PLACE(pa), PLACE(pb),
        run({"kind": "detach", "module": "Oda", "instance": "oda-2"}, ANY),
        other("cad.slabs.edit", {"uids": [UID(4)], "operation": {"kind": "set", "thickness": 0.2}}),
        UPDATE(done("$any"), {"fields": {**pa.field(4), **pb.field(4, changed_ents(4, thickness=0.2))}},
               within=1e-9)]))
    out.append(case("Elle değişen kolon kaynağın değeriyle yazılır, söylenir", [
        DEFINE(), PLACE(pa),
        other("cad.arch.edit", {"uids": [pa.uid(2)], "operation": {"kind": "set", "width": 0.45}}),
        other("cad.arch.edit", {"uids": [UID(2)], "operation": {"kind": "set", "width": 0.5}}),
        UPDATE(done("$any", [warn("module_drift_overwritten",
                                  "Elle değiştirilmiş 1 örnek nesnesi kaynağın değeriyle yazıldı; korumak için önce "
                                  "yerel istisna yapın.")]),
               {"fields": pa.field(2, wide)}, within=1e-9)]))
    # ── Üye ekle ve çıkar ───────────────────────────────────────────────────────────────────────────────────────
    new = NEXT + len(MEMBERS)
    out.append(case("Modüle ekle: serbest kolon tanıma girer, öbür örnekte yerine yerleşir", [
        DEFINE(), PLACE(pa),
        run({"kind": "addMembers", "module": "Oda", "instance": "oda-1", "uids": [UID(10)]}, done("$any"),
            {"ids": ids_with(pa) + [new],
             "fields": {str(new): {"at": P(*pa.f((2, 2.6))), "width": 0.3, "arch": {"level": "kat1"}}}},
            within=1e-9)]))
    out.append(case("Ret: akslara bağlı kolon modüle eklenmez", [
        DEFINE(),
        run({"kind": "addMembers", "module": "Oda", "instance": "oda-1", "uids": [UID(7)]},
            refusal("module_kind", "Akslara bağlı kolon modüle alınmaz: yeri aksların kesişimidir. Önce Bağı kopar'ı "
                                   "seçin ya da kolonu seçimden çıkarın.", "operation.uids[0]"),
            {"revision": "same"})]))
    out.append(case("Modülden çıkar: rampa ilk örnekte kalır, öbüründe silinir", [
        DEFINE(), PLACE(pa),
        run({"kind": "removeMembers", "module": "Oda", "uids": [UID(6)]}, done("$any"),
            {"ids": [i for i in ids_with(pa) if i != pa.slots[6]]})]))
    # ── Mahal ayırıcı (docs/adr/8050 §4) ────────────────────────────────────────────────────────────────────────
    slots, f = open_placed((20, 0), True, "kat1")
    sep = lambda pts: [P(*f(q)) for q in pts]
    uid = lambda m: f"$uidOf:{slots[m]}"
    srcs = [uid(m) for m in range(1, 6)]
    open_place = {"kind": "place", "module": "Açık plan", "at": P(20, 0), "mirror": True, "level": "kat1"}
    out.append({"name": "Mahal ayırıcı modül olur: açık planın iki mahali ayırıcısıyla ve duvarlarıyla",
                "setup": open_setup(), "steps": [
                    run(OPEN_DEFINE, done("$any"), {"revision": "changed"})]})
    out.append({"name": "Yerleştir: aynalı, 1. katta; ayırıcı örneğin katında, mahaller örneğin duvarlarına ve "
                        "ayırıcısına bağlı", "setup": open_setup(), "steps": [
                    run(OPEN_DEFINE, ANY),
                    run(open_place, done("$any"), {
                        "ids": list(range(1, 9)) + sorted(slots.values()),
                        "fields": {
                            str(slots[5]): {"pts": sep([(5, 0), (5, 8)]), "spaceSeparator": {"level": "kat1"}},
                            str(slots[6]): {"sources": srcs, "seed": P(*f((2.55, 4))), "arch": {"level": "kat1"},
                                            "broken": None},
                            str(slots[7]): {"sources": srcs, "seed": P(*f((7.45, 4))), "arch": {"level": "kat1"},
                                            "broken": None}}}, within=1e-9)]})
    out.append({"name": "Kaynakla aynı: aynalı örneğin mahalleri ve ayırıcısı kaynağına denk", "setup": open_setup(),
                "steps": [run(OPEN_DEFINE, ANY), run(open_place, ANY),
                          run({"kind": "update", "module": "Açık plan", "from": "acik-plan-2"},
                              refusal("nothing_to_set", "“acik-plan-2” örneği kaynakla aynı; güncellenecek bir şey "
                                                        "yok.", "operation.from"))]})
    out.append({"name": "Güncelle: ayırıcı kaynakta 1 m doğuya; aynalı örnekte 1 m batıya kayar", "setup": open_setup(),
                "steps": [run(OPEN_DEFINE, ANY), run(open_place, ANY),
                          other("cad.entities.transform",
                                {"uids": [UID(5)], "transform": {"kind": "move", "dx": 1.0, "dy": 0.0}}),
                          run({"kind": "update", "module": "Açık plan", "from": "acik-plan-1"}, done("$any"),
                              {"fields": {str(slots[5]): {"pts": sep([(6, 0), (6, 8)]),
                                                          "spaceSeparator": {"level": "kat1"}}}}, within=1e-9)]})
    out.append({"name": "Ret: sıradan çoklu çizgi modüle alınmaz, ayırıcı olan alınır", "setup": open_setup(),
                "steps": [run({"kind": "define", "name": "Çizgi", "uids": [UID(8)], "base": P(0, 0)},
                              refusal("module_kind", "Oda modülünde duvar, açıklık, mahal, mahal ayırıcı, donatı, "
                                                     "kolon, merdiven, rampa, döşeme, ölçü, etiket ve yazı olur.",
                                      "operation.uids[0]"))]})
    # ── Retler ──────────────────────────────────────────────────────────────────────────────────────────────────
    out.append(case("Ret: akslara bağlı kolon modüle alınmaz", [run(
        {"kind": "define", "name": "Aks kolonu", "uids": [UID(7)], "base": P(0, 0)},
        refusal("module_kind", "Akslara bağlı kolon modüle alınmaz: yeri aksların kesişimidir. Önce Bağı kopar'ı "
                               "seçin ya da kolonu seçimden çıkarın.", "operation.uids[0]"))]))
    out.append(case("Ret: aks modüle alınmaz", [run(
        {"kind": "define", "name": "Aks", "uids": [UID(8)], "base": P(0, 0)},
        refusal("module_kind", "Oda modülünde duvar, açıklık, mahal, mahal ayırıcı, donatı, kolon, merdiven, rampa, döşeme, ölçü, "
                               "etiket ve yazı olur.", "operation.uids[0]"))]))
    return out


def document():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.modules.edit", "commandVersion": 1,
            "title": "Oda modülünde kolon, merdiven, rampa, döşeme ve eğri duvar (ADR 8050)",
            "note": "Beklenenler scripts/fixtures/module_kinds_cases.py'den: yerleştirmenin dönüşümü ve türlerin "
                    "aynalanma kuralları (kolon arch_command_cases.mirrored_column ile); KentOS kodu kullanılmadı.",
            "setup": setup(), "cases": cases()}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    doc = document()
    encoded = json.dumps(doc, ensure_ascii=False, indent=2, allow_nan=False) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != encoded:
            print(f"{OUT.relative_to(ROOT)} bağımsız başvurudan farklı.")
            return 1
        print(f"cad.modules.edit (türler): {len(doc['cases'])} durum bağımsız başvuruyla aynı.")
    else:
        OUT.write_text(encoded)
        print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
