#!/usr/bin/env python3
"""Slabs bound to their walls or their space (docs/adr/0771 §4), from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.slabs.bound.json; `--check` compares and writes nothing. The drawing, the outlines found
from the walls, the edge offset and the Metraj are the slab cases' (`slab_cases.py`, `space_command_cases.py`: an 8 × 5
building of 20 cm walls on the ground floor, a 15 cm partition at x = 5). The rules, as the ADR gives them:

- `cad.slabs.create` with `bound: true` and a seed keeps the walls (`bound`: source walls, the seed, the face rule, the
  edge offset); with `space` the slab's outline is that space's rings, bound to it. A bound slab without a seed or a
  space, or a space with an outline drawn or a seed, is `invalid_slab`; the space named by an id that is none,
  `invalid_uid`, `entity_not_found` or `not_a_space`.
- The bound's `uids` are the walls (of the slab's level, every wall when it has none) whose box, its path's grown by
  its width and its offset, meets the outline's box: the walls that bound it when it was last found, in drawing order.
- Whenever a wall changes or the space does, a bound slab's outline is found again in the same step: a wall moved out
  grows it, a partition moved moves a room's; one it cannot be found from (a wall deleted, the space deleted) keeps its
  last outline and is marked broken (`lostEnclosure`, `sourceGone`); an undo takes both back. Its edge's runs follow.
- `cad.slabs.edit`: `bind` binds a slab to walls round a seed or to a space (one of them: `invalid_slab`), found now and
  moved by `offset`; `detach` keeps the outline as its own (`slab_not_bound` for one not bound); `offset` on a bound
  slab adds to its bound's offset; `subtract` and `add` on a bound slab are `slab_bound`.
"""

import argparse
import copy
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import slab_cases as sl  # noqa: E402
import space_command_cases as sc  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.slabs.bound.json"
CREATE, EDIT = "cad.slabs.create", "cad.slabs.edit"
DELETE_STEP = "Sil"
P, refusal, uid_of = sc.P, sc.refusal, sc.uid_of
LEFT, RIGHT, BALCONY, box_json = sl.LEFT, sl.RIGHT, sl.BALCONY, sl.box_json


def box_of(parts):
    xs = [q[0] for o, _ in parts for q in o]
    ys = [q[1] for o, _ in parts for q in o]
    return (min(xs), min(ys), max(xs), max(ys))


def meets(a, b):
    return a[0] <= b[2] and b[0] <= a[2] and a[1] <= b[3] and b[1] <= a[3]


def bounding_walls(d, parts, level):
    """The walls of the level (all when none) whose grown box meets the outline's box (grown 1e-6), in drawing order."""
    ob = box_of(parts)
    eps = F(1, 10 ** 6)
    ob = (ob[0] - eps, ob[1] - eps, ob[2] + eps, ob[3] + eps)
    out = []
    for i, w in sorted(d.walls.items()):
        if level is not None and w.get("arch", {}).get("level") != level:
            continue
        r = F(w["width"]) + abs(F(w.get("offset", 0.0)))
        xs = [F(p["x"]) for p in w["pts"]]
        ys = [F(p["y"]) for p in w["pts"]]
        if meets((min(xs) - r, min(ys) - r, max(xs) + r, max(ys) + r), ob):
            out.append(uid_of(i))
    return out


def bound_json(b):
    out = {"source": b["source"]}
    if b.get("seed") is not None:
        out["seed"] = b["seed"]
    if b.get("face", "outline") != "outline":
        out["face"] = b["face"]
    if b.get("space") is not None:
        out["space"] = b["space"]
    if b.get("offset"):
        out["offset"] = b["offset"]
    if b.get("uids"):
        out["uids"] = b["uids"]
    if b.get("broken"):
        out["broken"] = b["broken"]
    return out


def entity(d, i, layer, s, extra=None):
    e = sl.entity(d, i, layer, s, extra)
    if s.get("bound"):
        e["bound"] = bound_json(s["bound"])
    return e


def space_parts(d, uid, path):
    """The space's exact rings, or a refusal."""
    if not uid.startswith("$uidOf:"):
        if len(uid) != 36 or any(ch not in sc.UID_TEXT for ch in uid):
            return None, refusal("invalid_uid", f"“{uid}” geçerli bir nesne kimliği değil; mahalin kalıcı kimliğini verin.", path)
        return None, refusal("entity_not_found", f"“{uid}” kimlikli mahal çizimde yok; var olan bir mahalin kimliğini verin.", path)
    i = int(uid[len("$uidOf:"):])
    if i not in d.spaces:
        return None, refusal("not_a_space", "Döşemenin bağlanacağı nesne mahal değil.", path)
    return d.spaces[i][1]["parts"], None


def found_again(d, s):
    """The bound slab as its source gives it now (ADR 0771 §4): the outline and the walls, or the last with its break."""
    b = s["bound"]
    level = s.get("arch", {}).get("level")
    s = copy.deepcopy(s)
    if b["source"] == "walls":
        parts = sl.found(d, b["seed"], b.get("face", "outline") == "room" and "room" or "outline", level)
        why = "lostEnclosure"
    else:
        i = int(b["space"][len("$uidOf:"):])
        parts = d.spaces[i][1]["parts"] if i in d.spaces else None
        why = "sourceGone"
    if parts is not None and b.get("offset"):
        parts, r = sl.offset(parts, b["offset"], "bound")
        if r:
            parts, why = None, "lostEnclosure"
    if parts is None:
        s["bound"]["broken"] = why
        return s
    s.update(parts=parts, json=None)
    s.pop("measure", None)
    s["bound"]["broken"] = None
    s["bound"]["uids"] = bounding_walls(d, parts, level) if b["source"] == "walls" else []
    return s


def follow(d):
    """The document's followers: the spaces found again, then each bound slab."""
    d.follow()
    for i, (ly, s, ex) in list(d.slabs.items()):
        if s.get("bound"):
            d.slabs[i] = (ly, found_again(d, s), ex)

# ── cad.slabs.create with bound and space ───────────────────────────────────────────────────────────────────────


def create(d, inp):
    made = []
    for i, n in enumerate(inp["slabs"]):
        at = f"slabs[{i}]"
        if n.get("bound") and "seed" not in n and "space" not in n:
            return refusal("invalid_slab", "Kaynağa bağlı döşemenin sınırı duvarlardan (seed) ya da bir mahalden (space) bulunur; çizilmiş sınır bağlanmaz.", f"{at}.bound"), None
        if "space" in n and ("parts" in n or "seed" in n):
            return refusal("invalid_slab", "Mahale bağlı döşemenin sınırı mahalin halkalarıdır; sınır (parts) ya da iç nokta (seed) ayrıca verilmez.", f"{at}.space"), None
        arch = n.get("arch", {})
        if "space" in n:
            parts, r = space_parts(d, n["space"], f"{at}.space")
            if r:
                return r, None
            js = None
        else:
            if ("parts" in n) != ("seed" in n):
                r = sl.level_check(arch, f"{at}.arch.level")
                if r:
                    return r, None
            got, r = sl.outline_of(d, n.get("parts"), n.get("seed"), n.get("boundary", "outline"), arch.get("level"), at)
            if r:
                return r, None
            parts, js = got
        if n.get("edgeOffset", 0):
            parts, r = sl.offset(parts, n["edgeOffset"], f"{at}.edgeOffset")
            if r:
                return r, None
            js = None
        s = {"parts": parts, "json": js, "role": n.get("role", "floor"), "thickness": n.get("thickness", 0.15),
             "offset": n.get("offset", 0.0), "arch": arch, "material": n.get("material"), "excluded": False, "attrs": {}}
        if "edge" in n:
            s["edge"] = {"kind": n["edge"]["kind"], "width": n["edge"].get("width", sl.WIDTH[n["edge"]["kind"]])}
        if "space" in n or (n.get("bound") and "seed" in n):
            s["bound"] = {"source": "space" if "space" in n else "walls", "seed": None if "space" in n else n["seed"],
                          "face": "room" if n.get("boundary") == "room" and "space" not in n else "outline",
                          "space": n.get("space"), "offset": n.get("edgeOffset", 0.0)}
            s["bound"]["uids"] = bounding_walls(d, parts, arch.get("level")) if "space" not in n else []
        made.append(s)
    r, warnings = sc.layer_check(inp["layerId"])
    if r:
        return r, None
    return {"status": "completed", "warnings": warnings}, made


def create_step(d, op, inp):
    result, made = create(d, inp)
    s = {"op": op, "command": CREATE, "input": inp}
    if made is None:
        s["result"] = result
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    if op == "validate":
        s["result"] = {"status": "completed", "output": None, "warnings": result["warnings"]}
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    if op == "plan":
        s["result"] = {"status": "completed", "output": {"slabs": [entity(d, 0, inp["layerId"], m) for m in made],
                                                          "revision": "$current"}, "warnings": result["warnings"]}
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    ids = list(range(d.next, d.next + len(made)))
    d.next += len(made)
    for i, m in zip(ids, made):
        d.slabs[i] = (inp["layerId"], m, {})
    s["result"] = {"status": "completed", "output": {"uids": ["$any"] * len(made), "ids": ids, "revision": "$current"},
                   "warnings": result["warnings"]}
    s["expect"] = {"ids": d.ids(), "entities": {str(i): entity(d, i, inp["layerId"], m) for i, m in zip(ids, made)},
                   "canUndo": True, "dirty": True, "revision": "changed"}
    return s

# ── cad.slabs.edit: bind, detach and the bound's offset ──────────────────────────────────────────────────────────


def edit(d, inp):
    found_ = sl.named(d, inp["uids"])
    if isinstance(found_, dict):
        return found_, None
    for at, i in found_:
        if i not in d.slabs:
            return refusal("not_a_slab", "Nesne döşeme değil; Döşemeyi düzenle yalnız döşemeleri değiştirir.", f"uids[{at}]"), None
    op = inp["operation"]
    kind = op["kind"]
    updates = []
    if kind == "bind":
        if "seed" in op:
            r = sc.point_not_finite(op["seed"], "Döşemenin iç noktasının", "operation.seed")
            if r:
                return r, None
        if "offset" in op and not math.isfinite(op["offset"]):
            return sl.not_finite("Kenar ötelemesi", "operation.offset"), None
        if ("seed" in op) == ("space" in op):
            return refusal("invalid_slab", "Döşeme ya duvarlara (seed: duvarlarla çevrili bir iç nokta) ya da bir mahale (space) bağlanır; birini verin.", "operation"), None
        for at, i in found_:
            s = copy.deepcopy(d.slabs[i][1])
            level = s.get("arch", {}).get("level")
            if "space" in op:
                parts, r = space_parts(d, op["space"], "operation.space")
            else:
                got, r = sl.outline_of(d, None, op["seed"], op.get("boundary", "outline"), level, "operation")
                parts = None if r else got[0]
            if r:
                return r, None
            dist = op.get("offset", 0.0)
            if dist:
                parts, r = sl.offset(parts, dist, "operation.offset")
                if r:
                    return r, None
            s.update(parts=parts, json=None)
            s.pop("measure", None)
            s["bound"] = {"source": "space" if "space" in op else "walls", "seed": op.get("seed"),
                          "face": "room" if op.get("boundary") == "room" and "space" not in op else "outline",
                          "space": op.get("space"), "offset": dist,
                          "uids": bounding_walls(d, parts, level) if "space" not in op else []}
            updates.append((i, s))
    elif kind == "detach":
        for at, i in found_:
            s = copy.deepcopy(d.slabs[i][1])
            if not s.get("bound"):
                return refusal("slab_not_bound", "Döşeme kaynağa bağlı değil; ayrılacak bağ yok.", f"uids[{at}]"), None
            s["bound"] = None
            updates.append((i, s))
    elif kind in ("subtract", "add"):
        for at, i in found_:
            if d.slabs[i][1].get("bound"):
                return refusal("slab_bound", "Döşemenin sınırı kaynağına bağlı; kesmek ya da alan eklemek için önce Ayır ile bağımsız kontur yapın.", f"uids[{at}]"), None
        return sl.edit(d, inp)
    elif kind == "offset":
        result, work = sl.edit(d, inp)
        if work is None:
            return result, None
        for k, (i, s) in enumerate(work):
            if s.get("bound"):
                s["bound"] = dict(s["bound"], offset=s["bound"].get("offset", 0.0) + op["distance"])
                work[k] = (i, found_again(d, s))
        return result, work
    else:
        return sl.edit(d, inp)
    return {"status": "completed"}, updates


def edit_step(d, op, inp):
    result, work = edit(d, inp)
    s = {"op": op, "command": EDIT, "input": inp}
    if work is None:
        s["result"] = result
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    if op == "validate":
        s["result"] = {"status": "completed", "output": None, "warnings": []}
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    ents = [entity(d, i, d.slabs[i][0], u, d.slabs[i][2]) for i, u in work]
    s["result"] = {"status": "completed", "output": {"slabs": ents, "revision": "$current"}, "warnings": []}
    if op == "plan":
        s["expect"] = {"ids": d.ids(), "revision": "same"}
        return s
    for i, u in work:
        d.slabs[i] = (d.slabs[i][0], u, d.slabs[i][2])
    s["expect"] = {"ids": d.ids(), "entities": {str(i): entity(d, i, d.slabs[i][0], d.slabs[i][1], d.slabs[i][2]) for i, _ in work},
                   "canUndo": True, "dirty": True, "revision": "changed"}
    return s

# ── Steps of other commands, and what follows them ──────────────────────────────────────────────────────────────


def quant(d):
    return sc.quant_step(d, command=sl.QUANT)


def shift_east(d):
    """cad.walls.edit's Kaydır of the east wall (its path north, its left west) 50 cm out: by −0.5; the corners'
    walls' ends slide along their edges with it (ADR 0710)."""
    d.walls[2] = dict(d.walls[2], pts=[P(8.5, 0), P(8.5, 5)])
    d.walls[1] = dict(d.walls[1], pts=[P(0, 0), P(8.5, 0)])
    d.walls[3] = dict(d.walls[3], pts=[P(8.5, 5), P(0, 5)])
    follow(d)
    return {"op": "execute", "command": "cad.walls.edit", "input": {"uids": [uid_of(2)], "operation": {"kind": "shift", "by": -0.5}},
            "result": {"status": "completed", "output": "$any", "warnings": "$any"}, "expect": {"revision": "changed"}}


def move_partition(d, dx):
    d.walls[5] = dict(d.walls[5], pts=[P(5 + dx, 0), P(5 + dx, 5)])
    follow(d)
    return {"op": "execute", "command": "cad.entities.transform",
            "input": {"uids": [uid_of(5)], "transform": {"kind": "move", "dx": float(dx), "dy": 0.0}},
            "result": {"status": "completed", "output": "$any", "warnings": "$any"}, "expect": {"revision": "changed"}}


def delete(d, i, slabs):
    """cad.entities.delete of object `i` (its id taken first, the bound slabs naming it after it is gone); the bound
    slabs' fields as they follow."""
    d.walls.pop(i, None)
    d.spaces.pop(i, None)
    follow(d)
    gone = f"$uid:gone{i}"

    def named(v):
        if isinstance(v, str):
            return gone if v == uid_of(i) else v
        if isinstance(v, list):
            return [named(x) for x in v]
        if isinstance(v, dict):
            return {k: named(x) for k, x in v.items()}
        return v
    return [{"op": "captureUid", "id": i, "as": f"gone{i}"},
            {"op": "execute", "command": "cad.entities.delete", "input": {"uids": [uid_of(i)]},
             "result": {"status": "completed", "output": "$any", "warnings": "$any"},
             "expect": {"fields": {str(k): {"bound": named(bound_json(d.slabs[k][1]["bound"]))} for k in slabs},
                        "revision": "changed"}}]


def space_step(d, seed):
    return sc.create_step(d, "execute", {"layerId": "mahal", "spaces": [{"seed": seed}]}, command="cad.spaces.create")


def one(layer="doseme", **kw):
    return {"layerId": layer, "slabs": [kw]}

# ── The cases ─────────────────────────────────────────────────────────────────────────────────────────────────────


def cases():
    c = sc.Cases()
    first = sc.NEW
    d = sc.Drawing()
    c.add("duvarların dış yüzlerine bağlı döşeme: kaynağı, iç noktası ve sınırlayan duvarlarıyla", [
        create_step(d, "execute", one(seed=LEFT, bound=True)), quant(d)])
    d = sc.Drawing()
    c.add("odanın iç yüzlerine bağlı döşeme: aynı kattaki dört duvarı", [
        create_step(d, "execute", one(seed=RIGHT, boundary="room", bound=True, arch={"level": "zemin"})), quant(d)])
    d = sc.Drawing()
    c.add("mahale bağlı döşeme: sınırı mahalin halkaları", [space_step(d, RIGHT),
          create_step(d, "execute", one(space=uid_of(first))), quant(d)])
    d = sc.Drawing()
    c.add("kenarı 10 cm dışa ötelenmiş bağlı döşeme ötelemeyi saklar", [
        create_step(d, "execute", one(seed=LEFT, bound=True, edgeOffset=0.1)), quant(d)])
    d = sc.Drawing()
    c.add("bağlı döşemenin planı yazmadan gösterir", [create_step(d, "plan", one(seed=LEFT, bound=True))])
    d = sc.Drawing()
    c.add("bağlı döşemenin doğrulaması çizimi değiştirmez", [create_step(d, "validate", one(seed=LEFT, bound=True))])
    d = sc.Drawing()
    c.add("çizilmiş sınır bağlanmaz: iç nokta ya da mahal ister",
          [create_step(d, "execute", one(parts=[box_json(20, 0, 24, 3)], bound=True))])
    d = sc.Drawing()
    c.add("mahale bağlı döşemeye sınır verilmez", [space_step(d, RIGHT),
          create_step(d, "execute", one(space=uid_of(first), parts=[box_json(20, 0, 24, 3)]))])
    d = sc.Drawing()
    c.add("mahalin geçersiz kimliği reddedilir", [create_step(d, "execute", one(space="mahal-1"))])
    d = sc.Drawing()
    c.add("çizimde olmayan mahal reddedilir", [create_step(d, "execute", one(space=sc.MISSING))])
    d = sc.Drawing()
    c.add("mahal olmayan nesneye bağlanmaz", [create_step(d, "execute", one(space=uid_of(1)))])
    # Kabul (ADR 0771): the insulated outer wall moved 50 cm out; the bound slab grows, the drawn balcony stays.
    d = sc.Drawing()
    made = create_step(d, "execute", {"layerId": "doseme", "slabs": [{"seed": LEFT, "bound": True},
                                                                     {"parts": [BALCONY], "role": "balcony", "edge": {"kind": "railing"},
                                                                      "arch": {"level": "zemin"}}]})
    c.add("dış duvar 50 cm kayınca bağlı döşeme büyür, bağımsız balkon ve korkuluğu yerinde; tek adımda", [
        made, shift_east(d), quant(d)])
    d = sc.Drawing()
    made = create_step(d, "execute", one(seed=LEFT, bound=True))
    before = quant(d)
    step = shift_east(d)
    after = quant(d)
    d2 = sc.Drawing()
    create(d2, one(seed=LEFT, bound=True))
    c.add("duvar kaydırması geri alınınca bağlı döşeme eski sınırına döner", [
        made, before, step, after, {"op": "undo", "returns": "Duvarı düzenle", "expect": {"canRedo": True, "revision": "changed"}},
        dict(before)])
    d = sc.Drawing()
    c.add("ara duvar kayınca odaya bağlı döşeme odayla daralır", [
        create_step(d, "execute", one(seed=RIGHT, boundary="room", bound=True, arch={"level": "zemin"})), move_partition(d, F(1, 2)), quant(d)])
    d = sc.Drawing()
    c.add("ara duvar kayınca mahale bağlı döşeme mahalle aynı adımda büyür", [
        space_step(d, RIGHT), create_step(d, "execute", one(space=uid_of(first))), move_partition(d, F(-1, 2)), quant(d)])
    d = sc.Drawing()
    c.add("çıkma döşemenin serbest kenarlarındaki korkuluk duvarla büyüyen sınırı izler", [
        create_step(d, "execute", one(seed=LEFT, bound=True, edgeOffset=1.0, role="balcony", edge={"kind": "railing"})),
        shift_east(d), quant(d)])
    d = sc.Drawing()
    made = create_step(d, "execute", one(seed=LEFT, bound=True))
    c.add("dış duvar silinince bağlı döşeme son sınırıyla kalır, kopuk: lostEnclosure", [made, *delete(d, 1, [first]), quant(d)])
    d = sc.Drawing()
    made = [space_step(d, RIGHT), create_step(d, "execute", one(space=uid_of(first)))]
    c.add("mahali silinen döşeme son sınırıyla kalır, kopuk: sourceGone", made + delete(d, first, [first + 1]))
    d = sc.Drawing()
    made = create_step(d, "execute", one(seed=LEFT, bound=True))
    whole = bound_json(d.slabs[first][1]["bound"])
    gone = delete(d, 1, [first])
    c.add("silme geri alınınca kopukluk kalkar", [made, *gone, {"op": "undo", "returns": DELETE_STEP,
          "expect": {"fields": {str(first): {"bound": whole}}, "canRedo": True, "revision": "changed"}}])
    # cad.slabs.edit: Kaynağa bağla, Ayır and a bound slab's offset.
    for name, n, op in [
        ("çizilmiş döşeme duvarlara bağlanır: sınırı duvarlardan bulunur", {"parts": [box_json(1, 1, 3, 3)]}, {"kind": "bind", "seed": LEFT}),
        ("çizilmiş döşeme odanın iç yüzlerine 5 cm içe ötelenerek bağlanır", {"parts": [box_json(6, 1, 7, 3)], "arch": {"level": "zemin"}},
         {"kind": "bind", "seed": RIGHT, "boundary": "room", "offset": -0.05}),
        ("bağlı döşeme ayrılır: sınırı kendisinin olur", {"seed": LEFT, "bound": True}, {"kind": "detach"}),
        ("bağlı döşemenin kenarı ötelenince bağın ötelemesi de değişir", {"seed": LEFT, "bound": True}, {"kind": "offset", "distance": 0.25}),
    ]:
        d = sc.Drawing()
        made = create_step(d, "execute", one(**n))
        c.add(name, [made, edit_step(d, "execute", {"uids": [uid_of(first)], "operation": op}), quant(d)])
    d = sc.Drawing()
    made = [space_step(d, RIGHT), create_step(d, "execute", one(parts=[box_json(6, 1, 7, 3)]))]
    c.add("çizilmiş döşeme mahale bağlanır", made + [edit_step(d, "execute", {"uids": [uid_of(first + 1)], "operation": {"kind": "bind", "space": uid_of(first)}}), quant(d)])
    d = sc.Drawing()
    made = create_step(d, "execute", one(parts=[box_json(1, 1, 3, 3)]))
    c.add("bağlamanın planı yazmaz", [made, edit_step(d, "plan", {"uids": [uid_of(first)], "operation": {"kind": "bind", "seed": LEFT}})])
    for name, n, op in [
        ("bağlı olmayan döşeme ayrılmaz", {"parts": [box_json(1, 1, 3, 3)]}, {"kind": "detach"}),
        ("bağlı döşemeden alan kesilmez", {"seed": LEFT, "bound": True}, {"kind": "subtract", "parts": [box_json(1, 1, 2, 2)]}),
        ("bağlı döşemeye alan eklenmez", {"seed": LEFT, "bound": True}, {"kind": "add", "parts": [box_json(8, 1, 9, 2)]}),
        ("hem iç nokta hem mahal verilmez", {"parts": [box_json(1, 1, 3, 3)]}, {"kind": "bind", "seed": LEFT, "space": uid_of(1)}),
        ("ne iç nokta ne mahal: bağlanmaz", {"parts": [box_json(1, 1, 3, 3)]}, {"kind": "bind"}),
        ("duvarların dışındaki noktaya bağlanmaz", {"parts": [box_json(1, 1, 3, 3)]}, {"kind": "bind", "seed": P(40, 40)}),
        ("mahal olmayan nesneye bağlanmaz", {"parts": [box_json(1, 1, 3, 3)]}, {"kind": "bind", "space": uid_of(1)}),
    ]:
        d = sc.Drawing()
        made = create_step(d, "execute", one(**n))
        c.add(name, [made, edit_step(d, "execute", {"uids": [uid_of(first)], "operation": op})])
    return c.cases


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    note = ("ADR 0771 §4. Beklenenler scripts/fixtures/bound_slab_cases.py'nin ADR'den yazılmış kurallarından, KentOS kodu olmadan; "
            "duvarlardan bulunan sınır, kenar ötelemesi ve Metraj slab_cases.py ile space_command_cases.py'nin kesirli kurallarından. "
            "Kurulumdaki çizimde 7 nesne var; yeni nesne 8'den başlar.")
    doc = {"format": "kentos.command-cases", "version": 1, "command": EDIT, "commandVersion": 1,
           "title": "Kaynağa bağlı döşeme: duvarlara ve mahale bağlama, izleme, kopukluk, ayırma; retler", "note": note,
           "setup": sl.SETUP, "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
            return 1
        print(f"{EDIT} (bağlı döşeme): {len(doc['cases'])} durum başvuruyla aynı.")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
