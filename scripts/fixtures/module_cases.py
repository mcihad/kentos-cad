#!/usr/bin/env python3
"""Room modules (docs/adr/0775, ADV-05): the shared cases of `cad.modules.edit` and `cad.modules.query`, written from the
ADR's rules without KentOS code.

The drawing: a 2.0 × 2.5 m bathroom on the ground floor (four straight walls 10 cm thick, a 80 cm door, a space bound to
its walls, a basin, a tag of the space, a dimension on its front wall), a corridor wall with its own door, a line, a wall
on the first floor and a curved wall. The cases define the bathroom as a module at the origin, place it again (moved,
mirrored, turned, on another level, numbered), update its source from one instance and give it to the others (one
detached, one keeping an exception, one on a locked layer, one with an object deleted), keep and give back local
exceptions, rename, detach and remove; and read the modules back.

An instance's matrix: its definition mirrored across its own y axis (x → −x) when asked, turned by its rotation, moved to
its base. Its objects' references (an opening's wall, a space's sources, a tag's object, a dimension's points) name the
instance's own objects; a wall, a space and a fixture take the instance's level; an opening's and a space's number take
the instance's prefix and suffix; a mirrored opening opens to the other side (its hinge stays at its wall's start). A
source update takes the changed objects of the instance it comes from back into the definition and gives them to the
other linked instances; a tag keeps its point, a text its point and turn, a dimension its moved text (the view's
placement is the instance's).

    python3 scripts/fixtures/module_cases.py          # writes the cases
    python3 scripts/fixtures/module_cases.py --check  # compares with the files
"""

import copy
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EDIT = ROOT / "fixtures/commands/v1/cad.modules.edit.json"
QUERY = ROOT / "fixtures/commands/v1/cad.modules.query.json"
# The KCAD sample of schema 142 (docs/adr/0775 §7): the bathroom as a module, and a removed one broken.
SAMPLE = ROOT / "fixtures/kcad/v2/modules.json"

BLOCK = "0192f5a1-7750-7000-8000-0000000000b1"
STEP = "Oda modülü"
ANY = {"status": "completed", "output": "$any", "warnings": "$any"}


def UID(n):
    return f"0192f5a1-7750-7000-8000-{n:012x}"


def P(x, y):
    return {"x": float(x), "y": float(y)}


def layer(id, name):
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


# Walls: id → (start, end, level).
WALLS = {1: ((0, 0), (2, 0), "zemin"), 2: ((2, 0), (2, 2.5), "zemin"), 3: ((2, 2.5), (0, 2.5), "zemin"),
         4: ((0, 2.5), (0, 0), "zemin"), 10: ((0, -2), (4, -2), "zemin"), 13: ((0, 5), (2, 5), "kat1"),
         14: ((5, 5), (7, 5), "zemin")}
THICK = 0.1
NEXT = 15  # the first slot a placement fills
MEMBERS = [1, 2, 3, 4, 5, 6, 7, 8, 9]  # the bathroom's objects, in the order they are given


def wall(i):
    a, b, level = WALLS[i]
    e = {"kind": "wall", "id": i, "layerId": "duvar", "attrs": {}, "pts": [P(*a), P(*b)], "width": THICK,
         "justify": "center", "arch": {"level": level}}
    if i == 14:
        e["bulges"] = [0.3, 0.0]
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


def door(i, host, at, width, number):
    a, b, _ = WALLS[host]
    return {"kind": "opening", "id": i, "layerId": "kapi", "attrs": {}, "host": UID(host), "at": at,
            "width": width, "form": "door", "frame": 0.05, "number": number, "place": jambs(a, b, at, width)}


def setup_entities():
    return [
        wall(1), wall(2), wall(3), wall(4),
        door(5, 1, 1.0, 0.8, "K1"),
        {"kind": "space", "id": 6, "layerId": "mimari", "attrs": {}, "sources": [UID(1), UID(2), UID(3), UID(4)],
         "parts": [{"outer": {"pts": [P(0.05, 0.05), P(1.95, 0.05), P(1.95, 2.45), P(0.05, 2.45)]}}],
         "seed": P(1.0, 1.25), "number": "B1", "name": "Banyo", "arch": {"level": "zemin"}},
        {"kind": "insert", "id": 7, "layerId": "mimari", "attrs": {}, "block": BLOCK, "p": P(0.4, 2.1),
         "scale": 1.0, "rotation": 0.0, "arch": {"level": "zemin"}},
        {"kind": "tag", "id": 8, "layerId": "mimari", "attrs": {}, "of": UID(6), "at": P(1.0, 1.0), "content": "{number}",
         "text": "B1"},
        {"kind": "dimension", "id": 9, "layerId": "olcu", "attrs": {}, "a": P(0, 0), "b": P(2, 0), "height": 0.125,
         "offset": -0.5, "style": "linear", "angle": 0.0,
         "refs": {"a": {"uid": UID(1), "place": {"kind": "vertex", "part": 0, "ring": 0, "index": 0}},
                  "b": {"uid": UID(1), "place": {"kind": "vertex", "part": 0, "ring": 0, "index": 1}}}},
        wall(10),
        door(11, 10, 2.0, 0.9, "K9"),
        {"kind": "line", "id": 12, "layerId": "duvar", "attrs": {}, "a": P(5, 0), "b": P(6, 0)},
        wall(13),
        wall(14),
    ]


def setup():
    ents = setup_entities()
    return {
        "format": "kentos.document", "version": 2, "name": "Oda modülleri",
        "uids": [UID(e["id"]) for e in ents],
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 50, "workspace": "cad",
                     "arch": {"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
                                         {"id": "kat1", "name": "1. kat", "elevation": 3.0, "height": 3.0}]}},
        "origin": {"x": 0, "y": 0},
        "layers": [layer("duvar", "Duvar"), layer("kapi", "Kapı"), layer("mimari", "Mimari"), layer("olcu", "Ölçü")],
        "activeLayer": "duvar",
        "styles": {"items": [], "categories": []},
        "blocks": [{"id": BLOCK, "name": "Lavabo", "base": P(0, 0),
                    "entities": [{"kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": P(0, 0), "b": P(0.5, 0)}]}],
        "entities": ents,
    }


ENTS = {e["id"]: e for e in setup_entities()}


# ── The module's definition and instances ─────────────────────────────────────────────────────────────────────────


def definition_member(i):
    """A member as the definition keeps it: the object at the origin's base, no slot, no level."""
    e = copy.deepcopy(ENTS[i])
    e["id"] = 0
    if "arch" in e:
        e["arch"].pop("level", None)
        if not e["arch"]:
            del e["arch"]
    return {"key": UID(i), "entity": e}


def first_instance():
    return {"id": "banyo-1", "at": P(0, 0), "level": "zemin", "version": 1,
            "members": [{"key": UID(i), "uid": UID(i)} for i in MEMBERS]}


def defined():
    return {"id": "banyo", "name": "Banyo", "version": 1, "members": [definition_member(i) for i in MEMBERS],
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

    def __init__(self, n, first, at, rot=0.0, mirror=False, level=None, numbering=None):
        self.n, self.at, self.rot, self.mirror, self.level = n, at, rot, mirror, level
        self.numbering = numbering or {}
        self.slots = {m: first + k for k, m in enumerate(MEMBERS)}
        self.f = matrix(at, rot, mirror)

    def id(self):
        return f"banyo-{self.n}"

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
        if self.numbering:
            r["numbering"] = self.numbering
        r["version"] = version
        r["members"] = [{"key": UID(m), "uid": self.uid(m)} for m in MEMBERS]
        return r

    def num(self, n):
        return self.numbering.get("prefix", "") + n + self.numbering.get("suffix", "")

    def wall_pts(self, i):
        """A wall's path, mirrored: run back, so the wall keeps its faces' sides (its left face stays its left)."""
        a, b, _ = WALLS[i]
        return (self.f(b), self.f(a)) if self.mirror else (self.f(a), self.f(b))

    def fields(self, door_width=0.8):
        """What the placement writes, object by object (the followers' own fields aside)."""
        out = {}
        for i in (1, 2, 3, 4):
            a, b = self.wall_pts(i)
            w = {"pts": [P(*a), P(*b)], "width": THICK}
            w["arch"] = {"level": self.level} if self.level else None
            out[str(self.slots[i])] = w
        # The door along its wall's path: a mirrored path runs back, so the door's distance is from the other end and
        # its hinge at its end; it still opens to its wall's left (into the room).
        a, b = self.wall_pts(1)
        length = math.hypot(b[0] - a[0], b[1] - a[1])
        at = length - 1.0 if self.mirror else 1.0
        d = {"host": self.uid(1), "at": at, "width": door_width, "number": self.num("K1"),
             "side": None, "hand": "end" if self.mirror else None, "place": jambs(a, b, at, door_width)}
        out[str(self.slots[5])] = d
        sp = {"sources": [self.uid(i) for i in (1, 2, 3, 4)], "number": self.num("B1"), "name": "Banyo",
              "seed": P(*self.f((1.0, 1.25))), "arch": {"level": self.level} if self.level else None}
        out[str(self.slots[6])] = sp
        out[str(self.slots[7])] = {"p": P(*self.f((0.4, 2.1))),
                                   "arch": {"level": self.level} if self.level else None}
        out[str(self.slots[8])] = {"of": self.uid(6), "at": P(*self.f((1.0, 1.0))), "text": self.num("B1")}
        out[str(self.slots[9])] = {"a": P(*self.f((0, 0))), "b": P(*self.f((2, 0))),
                                   "refs": {"a": {"uid": self.uid(1), "place": {"kind": "vertex", "part": 0, "ring": 0, "index": 0}},
                                            "b": {"uid": self.uid(1), "place": {"kind": "vertex", "part": 0, "ring": 0, "index": 1}}}}
        return out


# ── Steps ─────────────────────────────────────────────────────────────────────────────────────────────────────────


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


def run(op, result, expect=None, command=None, **extra):
    s = {"op": "execute", "input": {"operation": op}, "result": result}
    if command:
        s["command"] = command
        s["input"] = op
    if expect:
        s["expect"] = expect
    s.update(extra)
    return s


def other(command, inp, expect=None):
    s = {"op": "execute", "command": command, "input": inp, "result": ANY}
    if expect:
        s["expect"] = expect
    return s


def DEFINE(check=None):
    op = {"kind": "define", "name": "Banyo", "uids": [UID(i) for i in MEMBERS], "base": P(0, 0)}
    if check is None:
        return {"op": "execute", "input": {"operation": op}, "result": ANY}
    return run(op, done(edited(module=defined(), instance="banyo-1")), check)


def PLACE(p, version=1):
    op = {"kind": "place", "module": "Banyo", "at": P(*p.at)}
    if p.rot:
        op["rotation"] = p.rot
    if p.mirror:
        op["mirror"] = True
    if p.level:
        op["level"] = p.level
    if p.numbering:
        op["numbering"] = p.numbering
    return op


def placed_module(placements, version=1):
    m = defined()
    m["version"] = version
    m["instances"][0]["version"] = version
    for p in placements:
        m["instances"].append(p.record(version))
    return m


def case(name, steps):
    return {"name": name, "steps": steps}


IDS = list(range(1, 15))


def ids_with(*placements):
    out = list(IDS)
    for p in placements:
        out += sorted(p.slots.values())
    return out


def edit_cases():
    out = []
    # ── Tanımla ─────────────────────────────────────────────────────────────────────────────────────────────────
    out.append(case("Tanımla: banyonun dokuz nesnesi modül olur, yerinde ilk örneği kalır", [
        DEFINE({"ids": IDS, "revision": "changed", "canUndo": True})]))
    out.append(case("Tanımla ve geri al: proje modülsüz kalır", [
        DEFINE(), {"op": "undo", "returns": STEP, "expect": {"ids": IDS, "revision": "changed", "canRedo": True}}]))
    plan = {"op": "plan", "input": {"operation": {"kind": "define", "name": "Banyo", "uids": [UID(i) for i in MEMBERS],
                                                   "base": P(0, 0)}},
            "result": done(edited(module=defined(), instance="banyo-1")), "expect": {"revision": "same"}}
    out.append(case("Tanımla'nın planı yazmaz", [plan]))
    out.append(case("Ret: kapının duvarı seçimin dışında", [run(
        {"kind": "define", "name": "Kapı", "uids": [UID(5)], "base": P(0, 0)},
        refusal("module_outside", f"Seçimdeki açıklık seçimin dışındaki bir nesneye bağlı (duvar, {UID(1)}); onu da seçin ya da bağı kaldırın (kapsam dışı).",
                "operation.uids[0]"), {"revision": "same"})]))
    out.append(case("Ret: etiketin mahali seçimin dışında", [run(
        {"kind": "define", "name": "Etiket", "uids": [UID(8)], "base": P(0, 0)},
        refusal("module_outside", f"Seçimdeki etiket seçimin dışındaki bir nesneye bağlı (mahal, {UID(6)}); onu da seçin ya da bağı kaldırın (kapsam dışı).",
                "operation.uids[0]"))]))
    out.append(case("Ret: ölçünün duvarı seçimin dışında", [run(
        {"kind": "define", "name": "Ölçü", "uids": [UID(9)], "base": P(0, 0)},
        refusal("module_outside", f"Seçimdeki ölçü seçimin dışındaki bir nesneye bağlı (duvar, {UID(1)}); onu da seçin ya da bağı kaldırın (kapsam dışı).",
                "operation.uids[0]"))]))
    out.append(case("Ret: çizgi modüle alınmaz", [run(
        {"kind": "define", "name": "Çizgi", "uids": [UID(10), UID(12)], "base": P(0, 0)},
        refusal("module_kind", "Oda modülünde düz duvar, açıklık, mahal, donatı, ölçü, etiket ve yazı olur.",
                "operation.uids[1]"))]))
    out.append(case("Ret: eğrisel duvar ilk kapsamda yok", [run(
        {"kind": "define", "name": "Eğri", "uids": [UID(14)], "base": P(0, 0)},
        refusal("module_kind", "Eğrisel duvar ilk modül kapsamında değil; düz duvarlı bir oda seçin.", "operation.uids[0]"))]))
    out.append(case("Ret: iki kattaki duvarlar", [run(
        {"kind": "define", "name": "İki kat", "uids": [UID(10), UID(13)], "base": P(0, 0)},
        refusal("module_levels", "Modülün nesneleri tek katta olur; başka kattakileri seçimden çıkarın.", "operation.uids[1]"))]))
    out.append(case("Ret: boş ad", [run(
        {"kind": "define", "name": " ", "uids": [UID(10)], "base": P(0, 0)},
        refusal("invalid_input", "“ ” modül adı olamaz: boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 harf.",
                "operation.name"))]))
    out.append(case("Ret: aynı adlı ikinci modül", [DEFINE(), run(
        {"kind": "define", "name": "banyo", "uids": [UID(10)], "base": P(0, 0)},
        refusal("module_exists", "Projede “banyo” adlı bir modül var.", "operation.name"))]))
    out.append(case("Ret: nesne zaten bir örneğin", [DEFINE(), run(
        {"kind": "define", "name": "Koridor", "uids": [UID(1)], "base": P(0, 0)},
        refusal("module_member_taken", "Bu nesne zaten bir modül örneğinin; önce o örneği bağımsızlaştırın.",
                "operation.uids[0]"))]))
    out.append(case("Ret: kilitli katman", [
        other("cad.layers.set", {"operation": "locked", "layer": "duvar", "value": True}),
        run({"kind": "define", "name": "Koridor", "uids": [UID(10)], "base": P(0, 0)},
            refusal("layer_locked", "“Duvar” katmanı kilitli; modüle alınamaz.", "operation.uids[0]"))]))
    out.append(case("Ret: nesne verilmedi", [run(
        {"kind": "define", "name": "Boş", "uids": [], "base": P(0, 0)},
        refusal("no_entities", "Modüle alınacak nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin.",
                "operation.uids"))]))
    out.append(case("Ret: geçersiz kimlik", [run(
        {"kind": "define", "name": "X", "uids": ["x"], "base": P(0, 0)},
        refusal("invalid_uid", "“x” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.",
                "operation.uids[0]"))]))
    out.append(case("Ret: olmayan nesne", [run(
        {"kind": "define", "name": "X", "uids": [UID(99)], "base": P(0, 0)},
        refusal("entity_not_found", f"“{UID(99)}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.",
                "operation.uids[0]"))]))
    out.append(case("Ret: geçersiz beklenen sürüm", [{
        "op": "execute", "input": {"operation": {"kind": "define", "name": "X", "uids": [UID(10)], "base": P(0, 0)},
                                   "expectedRevision": "x1"},
        "result": refusal("invalid_revision", "Beklenen sürüm “x1” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.",
                          "expectedRevision")}]))
    out.append(case("Kimliği verilen modül", [run(
        {"kind": "define", "name": "Koridor duvarı", "id": "kd", "uids": [UID(10), UID(11)], "base": P(0, -2)},
        done("$any"), {"revision": "changed"})]))
    # ── Yerleştir ───────────────────────────────────────────────────────────────────────────────────────────────
    p2 = Placed(2, NEXT, (10, 0), level="zemin")
    out.append(case("Yerleştir: zemin katta 10 m sağa; bağlar örneğin kendi nesnelerine", [
        DEFINE(), run(PLACE(p2), done(edited(module=placed_module([p2]), instance="banyo-2",
                                             created=[p2.uid(m) for m in MEMBERS])),
                      {"ids": ids_with(p2), "fields": p2.fields()}, within=1e-9)]))
    p3 = Placed(2, NEXT, (20, 0), mirror=True, level="kat1", numbering={"prefix": "D2-"})
    out.append(case("Yerleştir: aynalı, 1. katta, numaralar D2- önekli; kapı öbür yana açılır", [
        DEFINE(), run(PLACE(p3), done(edited(module=placed_module([p3]), instance="banyo-2",
                                             created=[p3.uid(m) for m in MEMBERS])),
                      {"fields": p3.fields()}, within=1e-9)]))
    p4 = Placed(2, NEXT, (10, 10), rot=math.pi / 2, level="kat1")
    out.append(case("Yerleştir: 90° döndürülmüş", [
        DEFINE(), run(PLACE(p4), done(edited(module=placed_module([p4]), instance="banyo-2",
                                             created=[p4.uid(m) for m in MEMBERS])),
                      {"fields": p4.fields()}, within=1e-9)]))
    p5 = Placed(2, NEXT, (10, 0), level="zemin", numbering={"suffix": "-A"})
    out.append(case("Yerleştir: sonek numarası, etiketin yazısı izler", [
        DEFINE(), run(PLACE(p5), done("$any"), {"fields": {str(p5.slots[8]): {"text": "B1-A"},
                                                           str(p5.slots[5]): {"number": "K1-A"}}})]))
    out.append(case("Yerleştir ve geri al: tek adım", [
        DEFINE(), run(PLACE(p2), done("$any")),
        {"op": "undo", "returns": STEP, "expect": {"ids": IDS, "revision": "changed"}}]))
    out.append(case("Ret: olmayan kat", [DEFINE(), run(
        {"kind": "place", "module": "Banyo", "at": P(10, 0), "level": "kat9"},
        refusal("level_not_found", "Projede “kat9” katı yok.", "operation.level"), {"revision": "same"})]))
    out.append(case("Ret: olmayan modül", [run(
        {"kind": "place", "module": "Mutfak", "at": P(10, 0)},
        refusal("module_not_found", "Projede “Mutfak” adlı ya da kimlikli oda modülü yok.", "operation.module"))]))
    s = run({"kind": "place", "module": "Banyo", "at": P(10, 0)},
            refusal("not_finite", "Yer ve dönüş sonlu sayılar olmalı.", "operation.at"))
    s["nonFinite"] = {"operation.at.x": "NaN"}
    out.append(case("Ret: sonlu olmayan yer", [DEFINE(), s]))
    # ── Kaynağı güncelle ────────────────────────────────────────────────────────────────────────────────────────
    pa = Placed(2, NEXT, (10, 0), level="zemin")
    pb = Placed(3, NEXT + 9, (20, 0), mirror=True, level="kat1")
    pc = Placed(4, NEXT + 18, (10, 10), level="kat1")
    m2 = defined()
    m2["version"] = 2
    m2["members"][4]["entity"]["width"] = 0.9
    m2["members"][4]["entity"]["place"] = jambs((0, 0), (2, 0), 1.0, 0.9)
    m2["instances"][0]["version"] = 2
    m2["instances"] += [pa.record(2), pb.record(2)]
    fa, fb, fc = pa.fields(0.9), pb.fields(0.9), pc.fields(0.8)
    out.append(case("Kabul: dört örnek iki katta, biri aynalı; kapı eni 90 cm olur, üç bağlı örnek alır, ayrılan almaz", [
        DEFINE(), run(PLACE(pa), ANY), run(PLACE(pb), ANY), run(PLACE(pc), ANY),
        run({"kind": "detach", "module": "Banyo", "instance": "banyo-4"}, ANY),
        other("cad.openings.edit", {"uids": [UID(5)], "operation": {"kind": "set", "width": 0.9}}),
        run({"kind": "update", "module": "Banyo", "from": "banyo-1"},
            done(edited(module=m2, instance="banyo-1", changed=[pa.uid(5), pb.uid(5)],
                        report={"updated": ["banyo-1", "banyo-2", "banyo-3"], "changedKeys": [UID(5)]})),
            {"fields": {str(pa.slots[5]): fa[str(pa.slots[5])], str(pb.slots[5]): fb[str(pb.slots[5])],
                        str(pc.slots[5]): {"width": 0.8}}}, within=1e-9),
        {"op": "undo", "returns": STEP, "expect": {"fields": {str(pa.slots[5]): {"width": 0.8}}}}]))
    out.append(case("Güncelle: yalnız seçilen örnek", [
        DEFINE(), run(PLACE(pa), ANY), run(PLACE(pb), ANY),
        other("cad.openings.edit", {"uids": [UID(5)], "operation": {"kind": "set", "width": 0.9}}),
        run({"kind": "update", "module": "Banyo", "from": "banyo-1", "instances": ["banyo-3"]},
            done("$any"), {"fields": {str(pa.slots[5]): {"width": 0.8}, str(pb.slots[5]): {"width": 0.9}}})]))
    out.append(case("Güncelle: istisna olan kapı korunur", [
        DEFINE(), run(PLACE(pa), ANY),
        run({"kind": "except", "module": "Banyo", "uids": [pa.uid(5)]}, ANY),
        other("cad.openings.edit", {"uids": [UID(5)], "operation": {"kind": "set", "width": 0.9}}),
        run({"kind": "update", "module": "Banyo", "from": "banyo-1"},
            done("$any", [{"code": "module_exception_kept", "message": "1 nesne yerel istisna olduğu için kaynağın değişikliğini almadı.",
                           "path": "operation"}]),
            {"fields": {str(pa.slots[5]): {"width": 0.8}}})]))
    out.append(case("Güncelle: silinen nesne atlanır, söylenir", [
        DEFINE(), run(PLACE(pa), ANY),
        other("cad.entities.delete", {"uids": [pa.uid(5)]}),
        other("cad.openings.edit", {"uids": [UID(5)], "operation": {"kind": "set", "width": 0.9}}),
        run({"kind": "update", "module": "Banyo", "from": "banyo-1"},
            done("$any", [{"code": "module_member_gone", "message": "1 örnek nesnesi silinmiş; kaynağın değişikliği ona gitmedi.",
                           "path": "operation"}]))]))
    out.append(case("Güncelle: kilitli katmandaki örnek eski sürümde kalır", [
        DEFINE(), run(PLACE(pa), ANY),
        other("cad.openings.edit", {"uids": [UID(5)], "operation": {"kind": "set", "width": 0.9}}),
        other("cad.layers.set", {"operation": "locked", "layer": "mimari", "value": True}),
        run({"kind": "update", "module": "Banyo", "from": "banyo-1"},
            done("$any", [{"code": "module_instance_skipped", "message": "1 örnek kilitli katmandaki nesneleri yüzünden güncellenmedi: banyo-2.",
                           "path": "operation"}]),
            {"fields": {str(pa.slots[5]): {"width": 0.8}}})]))
    out.append(case("Ret: kaynakla aynı örnek", [DEFINE(), run(PLACE(pa), ANY), run(
        {"kind": "update", "module": "Banyo", "from": "banyo-1"},
        refusal("nothing_to_set", "“banyo-1” örneği kaynakla aynı; güncellenecek bir şey yok.", "operation.from"))]))
    out.append(case("Ret: kaynağı verecek örneğin nesnesi silinmiş", [DEFINE(),
        other("cad.entities.delete", {"uids": [UID(8)]}), run(
        {"kind": "update", "module": "Banyo", "from": "banyo-1"},
        refusal("module_member_missing", f"“banyo-1” örneğinin bir nesnesi ({UID(8)}) silinmiş; başka bir örnekten güncelleyin.",
                "operation.from"))]))
    out.append(case("Ret: olmayan örnek", [DEFINE(), run(
        {"kind": "update", "module": "Banyo", "from": "banyo-9"},
        refusal("module_instance_not_found", "“Banyo” modülünde “banyo-9” örneği yok.", "operation.from"))]))
    out.append(case("Güncelle: etiketin görünüşteki yeri örneğindir", [
        DEFINE(), run(PLACE(pa), ANY),
        other("cad.entities.transform", {"uids": [UID(8)], "transform": {"kind": "move", "dx": 0.5, "dy": 0.0}}),
        run({"kind": "update", "module": "Banyo", "from": "banyo-1"},
            done("$any"), {"fields": {str(pa.slots[8]): {"at": P(11.0, 1.0)}}}, within=1e-9)]))
    # ── İstisna, geri döndür, ayır, ad, sil ─────────────────────────────────────────────────────────────────────
    out.append(case("Kaynağa döndür: örneğin kapısı kaynağın enine döner", [
        DEFINE(), run(PLACE(pa), ANY),
        run({"kind": "except", "module": "Banyo", "uids": [pa.uid(5)]}, ANY),
        other("cad.openings.edit", {"uids": [pa.uid(5)], "operation": {"kind": "set", "width": 0.7}}),
        run({"kind": "restore", "module": "Banyo", "uids": [pa.uid(5)]}, done("$any"),
            {"fields": {str(pa.slots[5]): {"width": 0.8}}})]))
    out.append(case("Ret: zaten istisna", [
        DEFINE(), run({"kind": "except", "module": "Banyo", "uids": [UID(5)]}, ANY),
        run({"kind": "except", "module": "Banyo", "uids": [UID(5)]},
            refusal("nothing_to_set", "Nesneler zaten yerel istisna.", "operation.uids"))]))
    out.append(case("Ret: nesne modülün örneğinde değil", [DEFINE(), run(
        {"kind": "except", "module": "Banyo", "uids": [UID(10)]},
        refusal("not_in_module", "Nesne “Banyo” modülünün bir örneğinde değil.", "operation.uids[0]"))]))
    md = defined()
    out.append(case("Bağımsızlaştır: örnek modülden ayrılır, nesneleri kalır", [
        DEFINE(), run(PLACE(pa), ANY),
        run({"kind": "detach", "module": "Banyo", "instance": "banyo-2"},
            done(edited(module=md, instance="banyo-2")), {"ids": ids_with(pa)})]))
    mr = defined()
    mr["name"] = "Ebeveyn banyosu"
    out.append(case("Yeniden adlandır", [DEFINE(), run(
        {"kind": "rename", "module": "banyo", "name": "Ebeveyn banyosu"}, done(edited(module=mr)))]))
    gone = {"id": "banyo", "name": "Banyo", "version": 1, "removed": True,
            "instances": [dict(first_instance(), broken="sourceGone")]}
    out.append(case("Kaynağı sil: örnek son hâliyle kalır, bağı kopuk", [
        DEFINE(), run({"kind": "remove", "module": "Banyo"}, done(edited(module=gone)), {"ids": IDS}),
        run({"kind": "place", "module": "Banyo", "at": P(10, 0)},
            refusal("module_removed", "“Banyo” modülünün kaynağı silinmiş; örnekleri son hâliyle duruyor.", "operation.module"))]))
    out.append(case("Kopuk örnek ayrılınca modül tümüyle gider", [
        DEFINE(), run({"kind": "remove", "module": "Banyo"}, ANY),
        run({"kind": "detach", "module": "Banyo", "instance": "banyo-1"}, done(edited(instance="banyo-1")))]))
    out.append(case("Ret: çizim değişmiş", [DEFINE(), {
        "op": "execute", "input": {"operation": {"kind": "rename", "module": "Banyo", "name": "B"}, "expectedRevision": "999"},
        "result": {"status": "conflict", "error": {"code": "revision_conflict",
                   "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
                   "path": "expectedRevision", "revision": "$current"}}}]))
    return out


def query_cases():
    out = []
    E = "cad.modules.edit"

    def q(inp, result, *before):
        return case(result[0], list(before) + [{"op": "execute", "input": inp, "result": result[1]}])

    def de():
        return dict(DEFINE(), command=E)

    def ed(op):
        return {"op": "execute", "command": E, "input": {"operation": op}, "result": ANY}

    def row(instances, version=1, removed=False, members=9, name="Banyo", id="banyo"):
        return {"id": id, "name": name, "version": version, "members": members, "removed": removed,
                "instances": instances}

    def irow(id, version=1, stale=False, objects=9, exceptions=(), missing=(), broken=None, level="zemin", mirror=False):
        r = {"id": id, "version": version, "stale": stale, "objects": objects}
        if exceptions:
            r["exceptions"] = list(exceptions)
        if missing:
            r["missing"] = list(missing)
        if broken:
            r["broken"] = broken
        if level:
            r["level"] = level
        r["mirror"] = mirror
        return r

    def ans(modules, instance=None, key=None):
        o = {"modules": modules}
        if instance:
            o["instance"] = instance
        if key:
            o["key"] = key
        o["revision"] = "$current"
        return done(o)

    pa = Placed(2, NEXT, (10, 0), level="zemin")
    pb = Placed(2, NEXT, (20, 0), mirror=True, level="kat1")
    out.append(q({}, ("Modülsüz proje", ans([]))))
    out.append(q({}, ("Tanımlanan modül ve ilk örneği", ans([row([irow("banyo-1")])])), de()))
    out.append(q({"module": "banyo"}, ("Kimlikle", ans([row([irow("banyo-1")])])), de()))
    out.append(q({"module": "BANYO"}, ("Adla, büyük harfle", ans([row([irow("banyo-1")])])), de()))
    out.append(q({"module": "Mutfak"}, ("Ret: olmayan modül", refusal(
        "module_not_found", "Projede “Mutfak” adlı ya da kimlikli oda modülü yok.", "module")), de()))
    out.append(q({"uid": UID(5)}, ("Nesnenin örneği ve anahtarı", ans([row([irow("banyo-1")])], "banyo-1", UID(5))), de()))
    out.append(q({"uid": UID(10)}, ("Ret: modülde olmayan nesne", refusal(
        "not_in_module", "Nesne bir oda modülünün örneğinde değil.", "uid")), de()))
    out.append(q({"uid": "x"}, ("Ret: geçersiz kimlik", refusal(
        "invalid_uid", "“x” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.",
        "uid")), de()))
    out.append(q({"uid": UID(99)}, ("Ret: olmayan nesne", refusal(
        "entity_not_found", f"“{UID(99)}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.",
        "uid")), de()))
    out.append(q({}, ("İki örnek", ans([row([irow("banyo-1"), irow("banyo-2")])])), de(), ed(PLACE(pa))))
    out.append(q({}, ("Aynalı örnek 1. katta", ans([row([irow("banyo-1"), irow("banyo-2", level="kat1", mirror=True)])])),
                 de(), ed(PLACE(pb))))
    out.append(q({"uid": pa.uid(6)}, ("Yerleştirilen mahalin örneği", ans([row([irow("banyo-1"), irow("banyo-2")])], "banyo-2", UID(6))),
                 de(), ed(PLACE(pa))))
    out.append(q({}, ("Silinen nesne eksik sayılır", ans([row([irow("banyo-1", objects=8, missing=[UID(8)])])])), de(),
                 other("cad.entities.delete", {"uids": [UID(8)]})))
    out.append(q({}, ("Yerel istisna", ans([row([irow("banyo-1", exceptions=[UID(5)])])])), de(),
                 ed({"kind": "except", "module": "Banyo", "uids": [UID(5)]})))
    out.append(q({}, ("Kilitli örnek eski sürümde", ans([row([irow("banyo-1", version=2), irow("banyo-2", stale=True)], version=2)])),
                 de(), ed(PLACE(pa)),
                 other("cad.openings.edit", {"uids": [UID(5)], "operation": {"kind": "set", "width": 0.9}}),
                 other("cad.layers.set", {"operation": "locked", "layer": "mimari", "value": True}),
                 ed({"kind": "update", "module": "Banyo", "from": "banyo-1"})))
    out.append(q({}, ("Güncellenen örnekler aynı sürümde", ans([row([irow("banyo-1", version=2), irow("banyo-2", version=2)], version=2)])),
                 de(), ed(PLACE(pa)),
                 other("cad.openings.edit", {"uids": [UID(5)], "operation": {"kind": "set", "width": 0.9}}),
                 ed({"kind": "update", "module": "Banyo", "from": "banyo-1"})))
    out.append(q({}, ("Kaynağı silinmiş modül", ans([row([irow("banyo-1", broken="sourceGone")], removed=True, members=0)])),
                 de(), ed({"kind": "remove", "module": "Banyo"})))
    out.append(q({}, ("Ayrılan örnek listede yok", ans([row([irow("banyo-1")])])), de(), ed(PLACE(pa)),
                 ed({"kind": "detach", "module": "Banyo", "instance": "banyo-2"})))
    out.append(q({}, ("Yeniden adlandırılan", ans([row([irow("banyo-1")], name="Ebeveyn banyosu")])), de(),
                 ed({"kind": "rename", "module": "Banyo", "name": "Ebeveyn banyosu"})))
    out.append(q({}, ("İki modül", ans([row([irow("banyo-1")]), {"id": "kd", "name": "Koridor duvarı", "version": 1,
                                                                    "members": 2, "removed": False,
                                                                    "instances": [irow("kd-1", objects=2)]}])),
                 de(), ed({"kind": "define", "name": "Koridor duvarı", "id": "kd", "uids": [UID(10), UID(11)], "base": P(0, -2)})))
    out.append(q({"module": "kd"}, ("İki modülden biri", ans([{"id": "kd", "name": "Koridor duvarı", "version": 1, "members": 2,
                                                                "removed": False, "instances": [irow("kd-1", objects=2)]}])),
                 de(), ed({"kind": "define", "name": "Koridor duvarı", "id": "kd", "uids": [UID(10), UID(11)], "base": P(0, -2)})))
    out.append(q({}, ("Geri alınan yerleştirme", ans([row([irow("banyo-1")])])), de(), ed(PLACE(pa)),
                 {"op": "undo", "returns": STEP}))
    return out


def sample():
    d = setup()
    d["name"] = "Oda modülleri"
    d["settings"]["arch"]["modules"] = [
        defined(),
        {"id": "eski", "name": "Eski banyo", "version": 2, "removed": True,
         "instances": [{"id": "eski-1", "at": P(0, -2), "version": 2,
                        "members": [{"key": UID(10), "uid": UID(10)}, {"key": UID(11), "uid": UID(11)}],
                        "broken": "sourceGone"}]},
    ]
    return d


def write(path, command, title, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "scripts/fixtures/module_cases.py yazar: ADR 0775'in kurallarından, KentOS kodu olmadan. Değiştirmeyin; betiği düzeltin.",
            "setup": setup(), "cases": cases}


def main():
    files = {
        EDIT: write(EDIT, "cad.modules.edit", "Oda modülleri (docs/adr/0775)", edit_cases()),
        QUERY: write(QUERY, "cad.modules.query", "Oda modüllerini oku (docs/adr/0775)", query_cases()),
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
