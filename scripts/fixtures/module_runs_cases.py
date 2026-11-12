#!/usr/bin/env python3
"""Kitchen runs in room modules (docs/adr/8050 §5), from the ADR's rules and the kitchen run's own (`casework_model.py`,
docs/adr/0706, 0727), without KentOS code.

Writes fixtures/commands/v1/cad.modules.edit.runs.json; `--check` compares and writes nothing. The drawing is the kitchen
cases' room (`casework_cases.setup(walls=True)`: the north, west and east walls, their inner faces y = 3.0, x = 0 and
x = 3.6) and its L run along the north wall to the corner and down the west wall (`casework_cases.ell`). The rules:

- a module takes a run whole: its objects are the run's, not the module's members; the definition keeps the run's
  recipe about the module's base (`runs`: its key the run's id), the first instance its run's id; a run with an object
  out of the selection, or standing on a wall out of it, is `module_outside`;
- an instance lays the recipe again with its matrix among its own walls, after them, in the same undo step: its points
  moved, a mirror turning its side over and writing each single door's hand turned over (the run's own rule lays the
  rest: modules numbered from `from`, the sink 5 cm from its module's start), its legs tied to the instance's walls;
- a source update takes the source's run back by its matrix (a mirror turning its side and hands back, the defaults
  left out) and lays the changed recipe in every other linked instance, its run's objects kept by part and number; a
  local exception keeps its run, one changed by hand is written and said, a detached instance's run is its own;
- taking a run out of the module deletes the other instances' runs with their objects and lets the named one go; a wall
  a run stands on is not taken out alone (`module_member_needed`).

    python3 scripts/fixtures/module_runs_cases.py
    python3 scripts/fixtures/module_runs_cases.py --check
"""

import argparse
import copy
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import casework_cases as cc  # noqa: E402  (the kitchen run's drawing and steps)
from casework_model import P  # noqa: E402

M = cc.M
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.modules.edit.runs.json"
ANY = {"status": "completed", "output": "$any", "warnings": "$any"}
STEP = "Oda modülü"
WALLS = [2, 3, 4]


def U(slot):
    return f"$uidOf:{slot}"


def setup():
    return cc.setup(walls=True)


def ell():
    return cc.ell()


def kitchen():
    """The drawing with its L run laid (run 0), and the step that lays it."""
    dr = cc.Drawing(setup())
    step = cc.run_step(dr, "execute", ell(), command="cad.casework.run")
    return dr, step


def run_slots(dr, k):
    return [slot for _, _, slot in dr.runs[k]["members"]]


def define_op(dr, uids=None):
    uids = uids if uids is not None else [U(s) for s in WALLS] + [U(s) for s in run_slots(dr, 0)]
    return {"kind": "define", "name": "Mutfak", "uids": uids, "base": P(0, 0)}


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


def refusal(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def done(output, warnings=None):
    return {"status": "completed", "output": output, "warnings": warnings or []}


def warn(code, message):
    return {"code": code, "message": message, "path": "operation"}


def capture(name, index):
    return {"op": "captureCasework", "index": index, "as": name}


def wall_member(dr, slot):
    e = copy.deepcopy(dr.setup_walls[slot])
    e["id"] = 0
    return e


def placed_wall(e, f, mirrored):
    """A definition's wall placed: its points by `f`, its path run back in a mirror (ADR 0339 §1)."""
    e = copy.deepcopy(e)
    pts = [P(*f((p["x"], p["y"]))) for p in e["pts"]]
    e["pts"] = pts[::-1] if mirrored else pts
    return e


def matrix(at, mirrored):
    def f(p):
        x, y = p
        return ((-x if mirrored else x) + at[0], y + at[1])
    return f


def placed_recipe(recipe, f, mirrored, modules):
    """The definition's recipe placed (docs/adr/8050 §5): its points by `f`; mirrored, its side turned over and each
    single door's hand turned over and written (`modules`: the run's modules as the definition's recipe lays them)."""
    out = copy.deepcopy(recipe)
    out["from"] = P(*f((recipe["from"]["x"], recipe["from"]["y"])))
    if "through" in recipe:
        out["through"] = [P(*f((p["x"], p["y"]))) for p in recipe["through"]]
    out["to"] = P(*f((recipe["to"]["x"], recipe["to"]["y"])))
    if mirrored:
        out["side"] = "right" if recipe.get("side", "left") == "left" else "left"
        own = {s["module"]: dict(s) for s in recipe.get("perModule", [])}
        for m in modules:
            hand = m.get("hand")
            if hand not in ("left", "right"):
                continue
            s = own.setdefault(m["number"], {"module": m["number"]})
            s["hand"] = "right" if hand == "left" else "left"
        if own:
            out["perModule"] = [own[k] for k in sorted(own)]
    return out


def normalized(r):
    """A recipe without what is its default (a left side, a left door) and its modules' own values in order."""
    out = copy.deepcopy(r)
    if out.get("side") == "left":
        del out["side"]
    own = []
    for e in out.pop("perModule", []):
        e = dict(e)
        if e.get("hand") == "left":
            del e["hand"]
        if "width" in e or "hand" in e:
            own.append(e)
    if own:
        out["perModule"] = sorted(own, key=lambda e: e["module"])
    return out


class Kitchen:
    """The kitchen drawing as the steps leave it: the kitchen cases' drawing and the module's instances."""

    def __init__(self):
        self.dr, self.lay_step = kitchen()
        self.dr.setup_walls = {s: copy.deepcopy(self.dr.objects[s]) for s in WALLS}
        self.recipe = cc.recipe_of(ell())
        b, refused = M.run(self.dr.view(), self.recipe, cc.LIBRARY)
        assert refused is None
        self.modules = b.modules
        self.instances = {"mutfak-1": {"walls": {s: s for s in WALLS}, "run": 0}}

    def place(self, n, at, mirrored=False, recipe=None):
        """Instance `n` placed at `at`: its walls in the slots after the drawing's last, then its run laid among them.
        The slots of its walls and of its run's objects."""
        dr = self.dr
        f = matrix(at, mirrored)
        walls = {}
        for s in WALLS:
            e = placed_wall(wall_member(dr, s), f, mirrored)
            slot = dr.next
            dr.next += 1
            e["id"] = slot
            dr.objects[slot] = e
            walls[s] = slot
        r = placed_recipe(recipe or self.recipe, f, mirrored, self.modules)
        b, refused = M.run(dr.view(), r, [])
        assert refused is None, refused
        slots, _ = cc.lay(dr, None, r, b)
        self.instances[f"mutfak-{n}"] = {"walls": walls, "run": len(dr.runs) - 1, "f": f, "mirrored": mirrored}
        return walls, slots

    def relay(self, name, recipe):
        """The instance's run laid again from the definition's `recipe` (its objects kept by part and number)."""
        inst = self.instances[name]
        f, mirrored = inst.get("f", matrix((0, 0), False)), inst.get("mirrored", False)
        r = placed_recipe(recipe, f, mirrored, self.modules)
        b, refused = M.run(self.dr.view(), r, [])
        assert refused is None, refused
        slots, _ = cc.lay(self.dr, inst["run"], r, b)
        return slots

    def expect(self, slots=(), **extra):
        out = {"ids": self.dr.ids(), "casework": self.dr.records()}
        if slots:
            out["entities"] = {str(s): self.dr.objects[s] for s in slots}
        out.update(extra)
        return out


def definition(k, runs_key="$uid:r0"):
    """The module as `define` writes it: the walls about the base (0, 0), the run's recipe, its first instance."""
    return {"id": "mutfak", "name": "Mutfak", "version": 1,
            "members": [{"key": U(s), "entity": wall_member(k.dr, s)} for s in WALLS],
            "runs": [{"key": runs_key, "recipe": k.recipe}],
            "instances": [{"id": "mutfak-1", "at": P(0, 0), "version": 1,
                           "members": [{"key": U(s), "uid": U(s)} for s in WALLS],
                           "runs": [{"key": runs_key, "uid": runs_key}]}]}


def place_op(at, mirrored=False):
    op = {"kind": "place", "module": "Mutfak", "at": P(*at)}
    if mirrored:
        op["mirror"] = True
    return op


def cases():
    out = []
    # ── Tanımla ─────────────────────────────────────────────────────────────────────────────────────────────────
    k = Kitchen()
    out.append({"name": "Tanımla: duvarlar modülün nesnesi, mutfak dizisi tarifiyle modülün; ilk örnek diziyi tutar",
                "steps": [k.lay_step, capture("r0", 0),
                          run(define_op(k.dr), done({"module": definition(k), "instance": "mutfak-1",
                                                      "report": {}, "revision": "$current"}),
                              k.expect(revision="changed"), within=1e-9)]})
    k = Kitchen()
    slots = run_slots(k.dr, 0)
    out.append({"name": "Ret: dizinin bir kısmı seçili", "steps": [
        k.lay_step,
        run(define_op(k.dr, [U(s) for s in WALLS] + [U(s) for s in slots[:2]]),
            refusal("module_outside", "Mutfak dizisinin bir kısmı seçili: dizinin bütün nesnelerini (dolaplar, tezgâh, "
                                      "dolgular) seçin ya da hiçbirini.", "operation.uids[3]"),
            {"revision": "same"})]})
    out.append({"name": "Ret: dizi seçimin dışındaki duvara dayanıyor", "steps": [
        k.lay_step,
        run(define_op(k.dr, [U(4)] + [U(s) for s in slots]),
            refusal("module_outside", "Mutfak dizisi seçimin dışındaki bir duvara dayanıyor; o duvarı da seçin ya da "
                                      "diziyi seçimden çıkarın.", "operation.uids[1]"),
            {"revision": "same"})]})
    # ── Yerleştir ───────────────────────────────────────────────────────────────────────────────────────────────
    k = Kitchen()
    pre = [k.lay_step, capture("r0", 0), run(define_op(k.dr), ANY)]
    walls, laid = k.place(2, (10, 0))
    out.append({"name": "Yerleştir: 10 m doğuya; dizi örneğin duvarlarına yeniden kurulur, bağları örneğin duvarlarına",
                "steps": pre + [run(place_op((10, 0)), done("$any"),
                                    k.expect([*walls.values(), *laid], revision="changed"), within=1e-9)]})
    k = Kitchen()
    pre = [k.lay_step, capture("r0", 0), run(define_op(k.dr), ANY)]
    walls, laid = k.place(2, (20, 0), mirrored=True)
    out.append({"name": "Yerleştir: aynalı; dizinin yanı ve tek kapakların menteşesi ters döner, evye modülünün "
                        "başından 5 cm'de kalır", "steps": pre + [
                    run(place_op((20, 0), True), done("$any"),
                        k.expect([*walls.values(), *laid], revision="changed"), within=1e-9)]})
    mirrored_place = pre + [run(place_op((20, 0), True), ANY), capture("r1", 1)]
    k_m = k
    k = Kitchen()
    pre = [k.lay_step, capture("r0", 0), run(define_op(k.dr), ANY)]
    before = k.expect()
    k.place(2, (10, 0))
    out.append({"name": "Yerleştir ve geri al: duvarlar ve dizi tek adımda", "steps": pre + [
        run(place_op((10, 0)), ANY),
        {"op": "undo", "returns": STEP, "expect": {**before, "revision": "changed"}}]})
    out.append({"name": "Kaynakla aynı: aynalı örneğin dizisi geri dönüşte tanımın tarifi", "steps": mirrored_place + [
        run({"kind": "update", "module": "Mutfak", "from": "mutfak-2"},
            refusal("nothing_to_set", "“mutfak-2” örneği kaynakla aynı; güncellenecek bir şey yok.",
                    "operation.from"))]})
    # ── Kaynağı güncelle ────────────────────────────────────────────────────────────────────────────────────────
    k = Kitchen()
    pre = [k.lay_step, capture("r0", 0), run(define_op(k.dr), ANY)]
    k.place(2, (10, 0))
    pre += [run(place_op((10, 0)), ANY), capture("r1", 1)]
    edit = cc.edit_step(k.dr, 0, {"kind": "module", "module": 3, "width": 0.45})
    new = copy.deepcopy(k.dr.runs[0]["recipe"])
    laid = k.relay("mutfak-2", new)
    out.append({"name": "Güncelle: kaynağın 3. modülü 45 cm; öbür örneğin dizisi yeniden kurulur", "steps": pre + [
        edit, run({"kind": "update", "module": "Mutfak", "from": "mutfak-1"},
                  done({"module": "$any", "instance": "mutfak-1",
                        "report": {"updated": ["mutfak-1", "mutfak-2"], "changedKeys": ["$uid:r0"]},
                        "revision": "$current"}),
                  k.expect(laid, revision="changed"), within=1e-9)]})
    # The mirrored instance's run edited: its 2nd module's door, given left, is the definition's right.
    k = Kitchen()
    pre = [k.lay_step, capture("r0", 0), run(define_op(k.dr), ANY)]
    k.place(2, (20, 0), mirrored=True)
    pre += [run(place_op((20, 0), True), ANY), capture("r1", 1)]
    edit = cc.edit_step(k.dr, 1, {"kind": "module", "module": 4, "width": 0.45})
    # Taken back by the mirror: the copy's run as its recipe lays it there, each single door's hand turned over (the
    # edit left module 4 and the new module 6 the run's default left door, the source's right).
    mine = k.dr.runs[1]["recipe"]
    b, refused = M.run(k.dr.view(), mine, [])
    assert refused is None, refused
    src = normalized(placed_recipe(mine, matrix((20, 0), True), True, b.modules))
    b, refused = M.run(k.dr.view(), src, [])
    assert refused is None, refused
    laid, _ = cc.lay(k.dr, 0, src, b)
    out.append({"name": "Güncelle: aynalı örnekte 4. modül 45 cm; kaynağa ters yanıyla döner", "steps": pre + [
        edit, run({"kind": "update", "module": "Mutfak", "from": "mutfak-2"}, done("$any"),
                  k.expect(laid, revision="changed"), within=1e-9)]})
    # ── İstisna, kaynağa döndür, elle değişen, ayır ─────────────────────────────────────────────────────────────
    k = Kitchen()
    pre = [k.lay_step, capture("r0", 0), run(define_op(k.dr), ANY)]
    k.place(2, (10, 0))
    pre += [run(place_op((10, 0)), ANY), capture("r1", 1)]
    first = run_slots(k.dr, 1)[0]
    edit = cc.edit_step(k.dr, 0, {"kind": "module", "module": 3, "width": 0.45})
    out.append({"name": "İstisna: örneğin dizisi (bir nesnesiyle) kaynağın değişikliğini almaz", "steps": pre + [
        run({"kind": "except", "module": "Mutfak", "uids": [U(first)]}, ANY), edit,
        run({"kind": "update", "module": "Mutfak", "from": "mutfak-1"},
            done("$any", [warn("module_exception_kept", "1 nesne yerel istisna olduğu için kaynağın değişikliğini "
                                                        "almadı.")]),
            k.expect(revision="changed"), within=1e-9)]})
    k = Kitchen()
    pre = [k.lay_step, capture("r0", 0), run(define_op(k.dr), ANY)]
    k.place(2, (10, 0))
    pre += [run(place_op((10, 0)), ANY), capture("r1", 1)]
    first = run_slots(k.dr, 1)[0]
    own = cc.edit_step(k.dr, 1, {"kind": "module", "module": 3, "width": 0.45})
    laid = k.relay("mutfak-2", k.recipe)
    out.append({"name": "Kaynağa döndür: istisna olan dizi tanımın tarifiyle yeniden kurulur", "steps": pre + [
        run({"kind": "except", "module": "Mutfak", "uids": [U(first)]}, ANY), own,
        run({"kind": "restore", "module": "Mutfak", "uids": [U(first)]}, done("$any"),
            k.expect(laid, revision="changed"), within=1e-9)]})
    k = Kitchen()
    pre = [k.lay_step, capture("r0", 0), run(define_op(k.dr), ANY)]
    k.place(2, (10, 0))
    pre += [run(place_op((10, 0)), ANY), capture("r1", 1)]
    own = cc.edit_step(k.dr, 1, {"kind": "module", "module": 4, "width": 0.45})
    edit = cc.edit_step(k.dr, 0, {"kind": "module", "module": 3, "width": 0.45})
    laid = k.relay("mutfak-2", k.dr.runs[0]["recipe"])
    out.append({"name": "Elle değişen dizi kaynağın tarifiyle yazılır, söylenir", "steps": pre + [
        own, edit,
        run({"kind": "update", "module": "Mutfak", "from": "mutfak-1"},
            done("$any", [warn("module_drift_overwritten", "Elle değiştirilmiş 1 örnek nesnesi kaynağın değeriyle "
                                                           "yazıldı; korumak için önce yerel istisna yapın.")]),
            k.expect(laid, revision="changed"), within=1e-9)]})
    k = Kitchen()
    pre = [k.lay_step, capture("r0", 0), run(define_op(k.dr), ANY)]
    k.place(2, (10, 0))
    k.place(3, (20, 0))
    pre += [run(place_op((10, 0)), ANY), run(place_op((20, 0)), ANY)]
    edit = cc.edit_step(k.dr, 0, {"kind": "module", "module": 3, "width": 0.45})
    laid = k.relay("mutfak-3", k.dr.runs[0]["recipe"])
    out.append({"name": "Ayır: ayrılan örneğin dizisi kendinindir, güncelleme ona gitmez", "steps": pre + [
        run({"kind": "detach", "module": "Mutfak", "instance": "mutfak-2"}, ANY), edit,
        run({"kind": "update", "module": "Mutfak", "from": "mutfak-1"}, done("$any"),
            k.expect(laid, revision="changed"), within=1e-9)]})
    # ── Örneğin dizisi kendi duvarlarını izler ──────────────────────────────────────────────────────────────────
    k = Kitchen()
    pre = [k.lay_step, capture("r0", 0), run(define_op(k.dr), ANY)]
    walls, _ = k.place(2, (10, 0))
    pre += [run(place_op((10, 0)), ANY)]
    move = cc.moved_wall(k.dr, walls[2], 0, -0.2)
    out.append({"name": "Örneğin kuzey duvarı 20 cm güneye: yalnız onun dizisi izler", "steps": pre + [move]})
    # ── Modülden çıkar ve modüle ekle ───────────────────────────────────────────────────────────────────────────
    k = Kitchen()
    pre = [k.lay_step, capture("r0", 0), run(define_op(k.dr), ANY)]
    k.place(2, (10, 0))
    pre += [run(place_op((10, 0)), ANY)]
    first = run_slots(k.dr, 0)[0]
    gone = run_slots(k.dr, 1)
    k.dr.runs.pop(1)
    for s in gone:
        del k.dr.objects[s]
    out.append({"name": "Modülden çıkar: dizi tanımdan çıkar; ilk örnekte kalır, öbüründe nesneleriyle silinir",
                "steps": pre + [run({"kind": "removeMembers", "module": "Mutfak", "uids": [U(first)]}, done("$any"),
                                    k.expect(revision="changed"))]})
    k = Kitchen()
    out.append({"name": "Ret: dizinin dayandığı duvar tek başına modülden çıkmaz", "steps": [
        k.lay_step, run(define_op(k.dr), ANY),
        run({"kind": "removeMembers", "module": "Mutfak", "uids": [U(2)]},
            refusal("module_member_needed", "Modülün mutfak dizisi çıkarılan duvara dayanıyor; diziyi de birlikte "
                                            "çıkarın ya da duvarı bırakın.", "operation.uids"),
            {"revision": "same"})]})
    k = Kitchen()
    pre = [k.lay_step, capture("r0", 0), run(define_op(k.dr, [U(s) for s in WALLS]), ANY)]
    walls_only = copy.deepcopy(k)
    k.place(2, (10, 0))
    # The walls-only module placed: then its run added from the first instance.
    k2 = walls_only
    f = matrix((10, 0), False)
    walls = {}
    for s in WALLS:
        e = placed_wall(wall_member(k2.dr, s), f, False)
        slot = k2.dr.next
        k2.dr.next += 1
        e["id"] = slot
        k2.dr.objects[slot] = e
        walls[s] = slot
    r = placed_recipe(k2.recipe, f, False, k2.modules)
    b, refused = M.run(k2.dr.view(), r, [])
    assert refused is None, refused
    laid, _ = cc.lay(k2.dr, None, r, b)
    out.append({"name": "Modüle ekle: dizi ilk örnekten tanıma girer, öbür örneğe kendi duvarlarıyla kurulur",
                "steps": pre + [run(place_op((10, 0)), ANY),
                                run({"kind": "addMembers", "module": "Mutfak", "instance": "mutfak-1",
                                     "uids": [U(s) for s in run_slots(k2.dr, 0)]}, done("$any"),
                                    k2.expect(laid, revision="changed"), within=1e-9)]})
    # ── Kaynağı sil, sorgu ──────────────────────────────────────────────────────────────────────────────────────
    k = Kitchen()
    pre = [k.lay_step, capture("r0", 0), run(define_op(k.dr), ANY)]
    k.place(2, (10, 0))
    pre += [run(place_op((10, 0)), ANY)]
    out.append({"name": "Kaynağı sil: tanımın dizisi gider, örneklerin dizileri olduğu gibi kalır", "steps": pre + [
        run({"kind": "remove", "module": "Mutfak"}, done("$any"), k.expect(revision="changed"))]})
    k = Kitchen()
    out.append({"name": "Ret: dizi zaten bir modül örneğinin", "steps": [
        k.lay_step, run(define_op(k.dr), ANY),
        run({"kind": "define", "name": "Mutfak 2", "uids": [U(s) for s in WALLS] + [U(s) for s in run_slots(k.dr, 0)],
             "base": P(0, 0)},
            refusal("module_member_taken", "Bu mutfak dizisi zaten bir modül örneğinin; önce o örneği "
                                           "bağımsızlaştırın.", "operation.uids[3]"),
            {"revision": "same"})]})
    k = Kitchen()
    pre = [k.lay_step, capture("r0", 0), run(define_op(k.dr), ANY)]
    k.place(2, (10, 0))
    k.place(3, (20, 0), mirrored=True)
    pre += [run(place_op((10, 0)), ANY), run(place_op((20, 0), True), ANY)]
    out.append({"name": "Üç örnek, biri aynalı: üç dizi, her biri kendi duvarlarına bağlı", "steps": pre[:-1] + [
        run(place_op((20, 0), True), done("$any"), k.expect(revision="changed"), within=1e-9)]})
    k = Kitchen()
    pre = [k.lay_step, capture("r0", 0), run(define_op(k.dr), ANY)]
    k.place(2, (10, 0))
    pre += [run(place_op((10, 0)), ANY)]
    first = run_slots(k.dr, 1)[0]
    out.append({"name": "Ret: örneğin dizisinin nesnesi zaten istisna", "steps": pre + [
        run({"kind": "except", "module": "Mutfak", "uids": [U(first)]}, ANY),
        run({"kind": "except", "module": "Mutfak", "uids": [U(first)]},
            refusal("nothing_to_set", "Nesneler zaten yerel istisna.", "operation.uids"), {"revision": "same"})]})
    return out


def document():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.modules.edit", "commandVersion": 1,
            "title": "Oda modülünde mutfak dizisi (ADR 8050 §5)",
            "note": "Beklenenler scripts/fixtures/module_runs_cases.py'den: dizinin döşenmesi casework_model.py'nin "
                    "(ADR 0706, 0727) kurallarıyla, yerleştirmenin dönüşümü ve aynanın yanı ve menteşesi ADR 8050 "
                    "§5'ten; KentOS kodu kullanılmadı.",
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
        print(f"cad.modules.edit (mutfak dizisi): {len(doc['cases'])} durum bağımsız başvuruyla aynı.")
    else:
        OUT.write_text(encoded)
        print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
