#!/usr/bin/env python3
"""A wall changed at a fixed reference and its ends' node decisions (docs/adr/0771 §1, §2), from the ADR's rules, without
KentOS code.

Writes fixtures/commands/v1/cad.walls.edit.parts.json and cad.walls.joins.nodes.json; `--check` compares and writes
nothing. The first's drawing is the space cases' (`space_command_cases.py`: an 8 × 5 building of 20 cm walls, a 15 cm
partition at x = 5 teeing into the long walls, a lone wall far away); the second's the joins' scenarios
(`wall_command_cases.py`'s `joins_setup`, the joins' reference `wall_join_cases.py`). The rules, as the ADR gives them:

- `keep` names the line that stays where it is when the width or the style changes: the path (as before), the body's
  centre, a finished face, or a core face (the boundary of the strongest component, the smallest priority, the first of
  equals, on that side; a wall without components its face). The wall's offset moves by what that line moved. A plan
  says what the change does round it: each space whose area changes (before, after) and each opening on a wall whose
  width changes (its depth before and after). `keep` alone is `nothing_to_set`; with `offset` or `justify`
  `invalid_wall`.
- `startPass` and `endPass` keep, on the wall's end, how many of the host's components each of its components passes
  where it tees into another wall (one number per component, a plain wall one; at most 16); null drops it. On an end
  that is a corner, an inline join or free, or on a curved edge or host, it is `wall_node_unsupported`.
- `cad.walls.joins` with `nodes`: each end of a joint says who decided it (`user` when the end has a join mode or passes,
  or a tee's node is the user's) and a tee's joining end the passes it takes (the user's, else the priorities').
"""

import argparse
import copy
import json
import sys
from fractions import Fraction as F
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import space_command_cases as sc  # noqa: E402
import wall_command_cases as wc  # noqa: E402
import wall_join_cases  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
EDIT, JOINS = "cad.walls.edit", "cad.walls.joins"
P, refusal, uid_of = sc.P, sc.refusal, sc.uid_of
DEFAULT = 3
# KentOS's own wall styles the cases take, from their documented thicknesses and priorities (ADR 0339 §3, 0340 §1):
# (thickness or None for the rest, priority), and the width a wall takes when one takes the rest.
STYLES = {
    "kentos-dis-duvar-25": ([(F(2, 100), 5), (None, 1), (F(5, 100), 3), (F(3, 100), 5)], F(25, 100)),
    "kentos-ic-duvar-15": ([(F(2, 100), 5), (None, 1), (F(2, 100), 5)], F(15, 100)),
}


def faces(w, comps=None):
    """(left face, right face, the components' thicknesses) of a wall's JSON, distances to its path's left."""
    width = F(w["width"])
    left = F(w.get("offset", 0.0)) + {"left": 0, "center": width / 2, "right": width}[w.get("justify", "center")]
    t = None
    if comps:
        fixed = sum(x for x, _ in comps if x is not None)
        t = [x if x is not None else width - fixed for x, _ in comps]
    return left, left - width, t


def reference(w, r, comps=None):
    left, right, t = faces(w, comps)
    if r == "path":
        return F(0)
    if r == "center":
        return (left + right) / 2
    if r == "leftFace":
        return left
    if r == "rightFace":
        return right
    if not comps:
        return left if r == "coreLeft" else right
    core = min(range(len(comps)), key=lambda k: (comps[k][1] if comps[k][1] is not None else DEFAULT, k))
    before = sum(t[:core])
    return left - before if r == "coreLeft" else left - before - t[core]


def changed_wall(d, i, ch):
    """The wall `i` with the change `ch` (width, style, keep) as the ADR keeps it; and its components (or None)."""
    w = copy.deepcopy(d.walls[i])
    old_comps = STYLES[w["style"]][0] if w.get("style") else None
    new = copy.deepcopy(w)
    comps = old_comps
    if "style" in ch:
        new["style"] = ch["style"]
        comps, width = STYLES[ch["style"]]
        if all(x is not None for x, _ in comps):
            new["width"] = float(sum(x for x, _ in comps))
    if "width" in ch:
        new["width"] = ch["width"]
    if "keep" in ch:
        was = reference(w, ch["keep"], old_comps)
        now = reference(new, ch["keep"], comps)
        new["offset"] = float(F(w.get("offset", 0.0)) + was - now)
        if new["offset"] == 0:
            new.pop("offset")
    return new


def effects(d, i, new):
    """The spaces whose area the change moves and the openings whose depth it changes (ADR 0770's `WallEffects`)."""
    old = d.walls[i]
    before = {k: sc.area_of(s["parts"]) for k, (_, s, _) in d.spaces.items()}
    d2 = copy.deepcopy(d)
    d2.walls[i] = new
    d2.follow()
    def change(k, s):
        # ADR 0770's `WallEffects` space: its number and name when it has them, `broken` when it no longer closes.
        out = {"uid": uid_of(k)}
        for key in ("number", "name"):
            if d.spaces[k][1].get(key):
                out[key] = d.spaces[k][1][key]
        out.update({"before": float(before[k]), "after": float(sc.area_of(s["parts"]))})
        if s.get("broken"):
            out["broken"] = True
        return out
    spaces = [change(k, s) for k, (_, s, _) in sorted(d2.spaces.items()) if sc.area_of(s["parts"]) != before[k]]
    depths = []
    if F(old["width"]) != F(new["width"]):
        depths = [{"uid": uid_of(k), "before": old["width"], "after": new["width"]}
                  for k, o in sorted(d.openings.items()) if o[0] == i]
    # The lists are written only when not empty.
    return {k: v for k, v in (("spaces", spaces), ("depths", depths)) if v}


def step(op, inp, result, expect, within=None):
    s = {"op": op, "command": EDIT, "input": inp, "result": result, "expect": expect}
    if within:
        s["within"] = within
    return s


def keep_step(d, op, i, ch):
    """cad.walls.edit `set` with `keep` on wall `i`: the plan says the effects, the execute writes the offset."""
    inp = {"uids": [uid_of(i)], "operation": dict({"kind": "set"}, **ch)}
    new = changed_wall(d, i, ch)
    if op == "plan":
        out = {"walls": "$any", "effects": effects(d, i, new), "revision": "$current"}
        if d.openings or d.spaces:
            out["followers"] = "$any"
        if "style" in ch:
            out["stylesAdded"] = [ch["style"]]
        return step("plan", inp, {"status": "completed", "output": out, "warnings": "$any"},
                    {"ids": d.ids(), "revision": "same"}, 1e-9)
    if op == "validate":
        return step("validate", inp, {"status": "completed", "output": None, "warnings": []}, {"ids": d.ids(), "revision": "same"})
    fields = {"width": new["width"], "offset": new.get("offset", 0.0) or None}
    if "style" in new:
        fields["style"] = new["style"]
    d.walls[i] = new
    d.follow()
    return step("execute", inp, {"status": "completed", "output": "$any", "warnings": "$any"},
                {"fields": {str(i): fields}, "revision": "changed"}, 1e-9)


def door(d, host, at):
    d.openings[d.next] = (host, 0.9, 2.1, None, "door", "kentos-kapi-90", None, at)
    d.next += 1
    return {"op": "execute", "command": "cad.openings.create", "input": {"layerId": "kapi", "openings": [{"host": uid_of(host), "at": at}]},
            "result": {"status": "completed", "output": "$any", "warnings": "$any"}, "expect": {"revision": "changed"}}


def room(d, seed):
    return sc.create_step(d, "execute", {"layerId": "mahal", "spaces": [{"seed": seed}]}, command="cad.spaces.create")


def unsupported(why):
    return refusal("wall_node_unsupported",
                   f"Düğüm kararı verilemez: {why} Kararı kaldırın (null) ya da ucu düz bir duvarın içine uzatın.", "uids[0]")


def pass_step(d, i, op, result=None, fields=None):
    inp = {"uids": [uid_of(i)], "operation": dict({"kind": "joins"}, **op)}
    if result is not None:
        return step("execute", inp, result, {"ids": d.ids(), "revision": "same"})
    joins = dict(d.walls[i].get("joins", {}))
    for k in ("startPass", "endPass"):
        if k in op:
            if op[k] is None:
                joins.pop(k, None)
            else:
                joins[k] = op[k]
    if joins:
        d.walls[i]["joins"] = joins
    else:
        d.walls[i].pop("joins", None)
    return step("execute", inp, {"status": "completed", "output": "$any", "warnings": "$any"},
                {"fields": {str(i): {"joins": joins or None}}, "revision": "changed"})


def edit_cases():
    c = sc.Cases()
    door_id = sc.NEW + 1

    def drawing():
        d = sc.Drawing()
        return d, [room(d, sc.RIGHT), door(d, 2, 2.0)]
    for name, keep in [("yol sabit: iki yana kalınlaşır, oda daralır, kapının derinliği artar", "path"),
                       ("iç bitmiş yüz sabit: oda değişmez, duvar dışa kalınlaşır", "leftFace"),
                       ("dış bitmiş yüz sabit: oda 10 cm daralır", "rightFace"),
                       ("eksen sabit: ortalı duvarda yolla aynı", "center"),
                       ("bileşensiz duvarda çekirdek yüzü bitmiş yüzdür", "coreLeft")]:
        d, made = drawing()
        c.add(f"Planda etki: {name}", made + [keep_step(d, "plan", 2, {"width": 0.3, "keep": keep})])
    for name, keep in [("iç yüz sabit kalarak 30 cm", "leftFace"), ("dış yüz sabit kalarak 30 cm", "rightFace")]:
        d = sc.Drawing()
        made = [room(d, sc.RIGHT)]
        c.add(f"Yazılır: {name}; oda aynı adımda izler", made + [keep_step(d, "execute", 2, {"width": 0.3, "keep": keep}),
                                                                 sc.quant_step(d, command="cad.spaces.quantities")])
    d, made = drawing()
    c.add("Stil değişir, dış çekirdek yüzü yerinde kalır: Dış duvar 25'e geçen duvarın ötelemesi",
          made + [keep_step(d, "plan", 2, {"style": "kentos-dis-duvar-25", "keep": "coreRight"})])
    d, made = drawing()
    c.add("Stil ve genişlik birlikte, iç çekirdek yüzü sabit, yazılır",
          made + [keep_step(d, "execute", 2, {"style": "kentos-dis-duvar-25", "width": 0.3, "keep": "coreLeft"})])
    d, made = drawing()
    c.add("Ara duvar iç duvar 15 olur, sol çekirdek yüzü sabit",
          made + [keep_step(d, "execute", 5, {"style": "kentos-ic-duvar-15", "keep": "coreLeft"})])
    d, made = drawing()
    c.add("Doğrulama yazmaz", made + [keep_step(d, "validate", 2, {"width": 0.3, "keep": "leftFace"})])
    d, made = drawing()
    c.add("Tek adım: geri alınca duvar ve oda eski hâline döner",
          made + [keep_step(d, "execute", 2, {"width": 0.3, "keep": "rightFace"}),
                  {"op": "undo", "returns": "Duvarı düzenle", "expect": {"fields": {"2": {"width": 0.2, "offset": None}},
                                                                         "canRedo": True, "revision": "changed"}}])
    for name, op, r in [
        ("Yalnız sabit referans: değiştirilecek alan yok", {"kind": "set", "keep": "leftFace"},
         refusal("nothing_to_set", "Değiştirilecek bir alan verilmedi: genişlik, hiza, öteleme, stil, uçlar, dolgu, yükseklik, kat, evre, durum ya da disiplin verin.", "operation")),
        ("Sabit referans ötelemeyle birlikte verilmez", {"kind": "set", "width": 0.3, "offset": 0.1, "keep": "leftFace"},
         refusal("invalid_wall", "Sabit referans duvarın ötelemesini kendisi bulur; öteleme ya da hiza ile birlikte verilmez.", "operation.keep")),
        ("Sabit referans hizayla birlikte verilmez", {"kind": "set", "width": 0.3, "justify": "left", "keep": "leftFace"},
         refusal("invalid_wall", "Sabit referans duvarın ötelemesini kendisi bulur; öteleme ya da hiza ile birlikte verilmez.", "operation.keep")),
    ]:
        d, made = drawing()
        c.add(name, made + [step("execute", {"uids": [uid_of(2)], "operation": op}, r, {"ids": d.ids(), "revision": "same"})])
    # Node decisions on the partition's ends: its start tees into the south wall, its end into the north one.
    d = sc.Drawing()
    c.add("Düğüm kararı: ara duvarın sonu konağın bileşenini geçer", [pass_step(d, 5, {"endPass": [1]})])
    d = sc.Drawing()
    c.add("Düğüm kararı iki uçta birden", [pass_step(d, 5, {"startPass": [0], "endPass": [1]})])
    d = sc.Drawing()
    c.add("Düğüm kararı kaldırılır (null): yeniden önceliklere göre", [pass_step(d, 5, {"endPass": [1]}), pass_step(d, 5, {"endPass": None})])
    d = sc.Drawing()
    c.add("Köşedeki uca düğüm kararı verilmez", [pass_step(d, 1, {"endPass": [0]}, result=unsupported(
        "bu uç bir köşede; köşede bileşenler gönyeyle birleşir, Alın ya da Geçen kullanın."))])
    d = sc.Drawing()
    c.add("Serbest uca düğüm kararı verilmez", [pass_step(d, 7, {"startPass": [0]}, result=unsupported(
        "bu uç hiçbir duvara birleşmiyor (serbest uç ya da birleşmeyen köşe)."))])
    d = sc.Drawing()
    c.add("Bileşen sayısına uymayan karar reddedilir", [pass_step(d, 5, {"endPass": [0, 1]}, result=refusal(
        "invalid_wall", "Düğüm kararı duvarın her bileşeni için bir sayıdır: 1 bileşen, 2 sayı verildi.", "operation.joins.endPass"))])
    d = sc.Drawing()
    c.add("16'dan büyük geçiş reddedilir", [pass_step(d, 5, {"endPass": [17]}, result=refusal(
        "invalid_wall", "Bir bileşen konağın en çok 16 bileşeninden geçer; 17 verildi.", "operation.joins.endPass[0]"))])
    d = sc.Drawing()
    c.add("Düğüm kararı tek adım, geri alınır", [pass_step(d, 5, {"endPass": [1]}),
          {"op": "undo", "returns": "Duvarı düzenle", "expect": {"fields": {"5": {"joins": None}}, "canRedo": True, "revision": "changed"}}])
    return c.cases

# ── cad.walls.joins with nodes ──────────────────────────────────────────────────────────────────────────────────


def nodes_answer(setup, ids):
    by = {e["id"]: e for e in setup["entities"]}
    out = wc.joins_answer(setup, ids)
    changed = [(i, wc.body_of(by[i])) for i in ids]
    boxes = [wc.reach_box(w, wc.JOIN_TOLERANCE) for _, w in changed]
    order = list(ids) + [i for i in sorted(by) if i not in ids and by[i]["kind"] == "wall"
                         and any(wc.boxes_meet(b, wc.reach_box(by[i], 0.0)) for b in boxes)]
    walls = [wall_join_cases.from_json(wc.body_of(by[i])) for i in order]
    solver = wall_join_cases.Solver(walls)
    solver.solve()
    nodes = {tuple(n["at"]): n for n in solver.nodes}
    for j in out["joints"]:
        node = nodes.get((j["at"]["x"], j["at"]["y"])) if j["kind"] == "tee" else None
        for m in j["walls"]:
            if "end" not in m:
                continue
            w = walls[order.index(int(m["uid"][len("$uidOf:"):]))]
            e = 0 if m["end"] == "start" else 1
            user = w.joins[e] != "auto" or w.passes[e] is not None or (node is not None and node["user"])
            m["decided"] = "user" if user else "auto"
            if node is not None:
                m["passes"] = node["depth"]
    return out


def nodes_cases():
    setup, groups = wc.joins_setup()
    before = {"ids": [e["id"] for e in setup["entities"]], "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
    c = []
    for name, ids in groups:
        if not any(k in name for k in ("T", "Düğüm", "Köşe: Alın", "Köşe: bir uç", "Düz devam: eşit", "Üç uç")):
            continue
        inp = {"uids": [uid_of(i) for i in ids], "nodes": True}
        c.append({"name": f"{name}: düğümler ve kararları",
                  "steps": [{"op": "execute", "command": JOINS, "input": inp,
                             "result": {"status": "completed", "output": nodes_answer(setup, ids), "warnings": []},
                             "within": 1e-9, "expect": dict(before)}]})
    return setup, c


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    note = ("ADR 0771 §1, §2. Beklenenler scripts/fixtures/wall_parts_cases.py'nin ADR'den yazılmış kurallarından, KentOS kodu olmadan; "
            "odalar space_command_cases.py'nin kesirli yüz bulmasından, düğümler wall_join_cases.py'nin çözücüsünden.")
    setup_n, nodes = nodes_cases()
    docs = {
        "cad.walls.edit.parts.json": {"format": "kentos.command-cases", "version": 1, "command": EDIT, "commandVersion": 1,
                                      "title": "Duvarı sabit referansla değiştir ve düğüm kararları: etkiler, retler, geri alma",
                                      "note": note, "setup": sc.SETUP, "cases": edit_cases()},
        "cad.walls.joins.nodes.json": {"format": "kentos.command-cases", "version": 1, "command": JOINS, "commandVersion": 1,
                                       "title": "Duvar birleşimlerinin düğümleri: kararı kim verdi, bileşenlerin geçişleri",
                                       "note": note, "setup": setup_n, "cases": nodes},
    }
    bad = 0
    for name, doc in docs.items():
        path = OUT / name
        text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
        if args.check:
            if not path.exists() or path.read_text() != text:
                print(f"{path.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
                bad += 1
            else:
                print(f"{doc['command']}: {len(doc['cases'])} durum başvuruyla aynı.")
            continue
        path.write_text(text)
        print(f"{path.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
