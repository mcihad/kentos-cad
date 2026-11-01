#!/usr/bin/env python3
"""Combinations of design options (docs/adr/0808, ADV-05): the shared cases of `cad.options.edit`'s Bileşimi kaydet,
Bileşimi uygula and Bileşimi sil, the combinations `cad.options.query` reads back, and a view recipe's combination
(`cad.views.edit` steps), written from the ADR's rules without KentOS code.

The drawing is option_cases.py's flat (its reference runs the sets' own operations here). The cases make two sets,
the kitchen (Mutfak: A, B) and a note (Not: A, B), save combinations of their options, apply and delete them, accept
and delete options under them, and read them back.

The rules (docs/adr/0808 §1–§4):

- A combination is an id, a name and choices, a set and one of its options each, a set once. A set it does not name
  shows its primary. A project holds 50 at most; names are unique, case aside.
- Bileşimi kaydet with no choices records every set's primary now; with choices, each a set and its option by id or
  name. A combination of that name (or of the id given) gets the choices (and the name); none the same refuses as
  nothing to set. A new one's id comes from its name as a set's does (“bilesim” for a name with no letters), numbered
  on when taken. A project without sets refuses.
- Bileşimi uygula makes each named option its set's primary; when they all are already, nothing to set.
- Bileşimi sil removes it; one a view's recipe shows is refused, the views named.
- Accepting a set or deleting an option drops the choices of the gone set or option from the combinations, with a
  warning naming them.
- A read lists every combination (whatever the set or object asked): its choices for every set in the project's order
  (its own, else the primary), whether the drawing shows it now (each choice the set's primary), and the objects,
  spaces' area and walls' length of the plan it shows (every object but the other options' of each set).
- A view recipe names a combination by its id; one the project lacks is refused; a template carries none.

    python3 scripts/fixtures/option_combo_cases.py          # writes the cases
    python3 scripts/fixtures/option_combo_cases.py --check  # compares with the files
"""

import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import option_cases as oc  # noqa: E402

ROOT = oc.ROOT
EDIT = ROOT / "fixtures/commands/v1/cad.options.edit.combos.json"
QUERY = ROOT / "fixtures/commands/v1/cad.options.query.combos.json"
ANY, STEP, UID, P = oc.ANY, oc.STEP, oc.UID, oc.P
Refused = oc.Refused
MAX = 50

KITCHEN = {"kind": "define", "name": "Mutfak", "uids": [UID(i) for i in oc.KITCHEN]}
KITCHEN_B = {"kind": "copy", "set": "mutfak", "option": "a"}
NOTE = {"kind": "define", "name": "Not", "uids": [UID(13)]}
NOTE_B = {"kind": "copy", "set": "not", "option": "a"}


def combo_id(combos, name):
    stem = oc.fold_id(name)
    stem = "bilesim" if stem == "secenek" else stem
    out, n = stem, 2
    while any(c["id"] == out for c in combos):
        out = f"{stem}-{n}"
        n += 1
    return out


def combo_at(combos, key):
    for i, c in enumerate(combos):
        if c["id"] == key:
            return i
    for i, c in enumerate(combos):
        if c["name"].lower() == key.strip().lower():
            return i
    raise Refused("option_combo_not_found", f"Projede “{key}” adlı ya da kimlikli seçenek bileşimi yok.", "operation.combo")


def set_at(sets, key, path):
    try:
        return oc.set_at(sets, key)
    except Refused as r:
        r.error["path"] = path
        raise


class State:
    """option_cases.py's drawing with the project's combinations and its views' recipes."""

    def __init__(self):
        self.doc = oc.Doc()
        self.combos = []
        self.views = {}

    def snapshot(self):
        d = self.doc
        return copy.deepcopy((d.ents, d.uid, d.next, d.sets, self.combos, self.views))

    def restore(self, s):
        d = self.doc
        d.ents, d.uid, d.next, d.sets, self.combos, self.views = copy.deepcopy(s)


def drop_gone(sets, combos):
    out = []
    for c in combos:
        keep = [ch for ch in c.get("choices", [])
                if any(s["id"] == ch["set"] and any(o["id"] == ch["option"] for o in s["options"]) for s in sets)]
        if len(keep) != len(c.get("choices", [])):
            out.append(c["name"])
        c["choices"] = keep
    return out


def clean_combo(c):
    return {"id": c["id"], "name": c["name"], **({"choices": c["choices"]} if c.get("choices") else {})}


def save_combo(st, op):
    oc.name_ok(op["name"], "operation.name")
    sets = st.doc.sets
    if not sets:
        raise Refused("option_set_not_found", "Projede tasarım seçeneği kümesi yok; bileşim seçeneklerden kurulur.", "operation")
    if "choices" not in op:
        picked = [{"set": s["id"], "option": s["primary"]} for s in sets]
    else:
        picked = []
        for i, ch in enumerate(op["choices"]):
            si = set_at(sets, ch["set"], f"operation.choices[{i}].set")
            oi = oc.option_at(sets[si], ch["option"], f"operation.choices[{i}].option")
            if any(p["set"] == sets[si]["id"] for p in picked):
                raise Refused("invalid_input", f"“{sets[si]['name']}” kümesi bileşimde iki kez seçilmiş.", f"operation.choices[{i}]")
            picked.append({"set": sets[si]["id"], "option": sets[si]["options"][oi]["id"]})
    name, combos = op["name"], st.combos
    if "id" in op:
        at = next((i for i, c in enumerate(combos) if c["id"] == op["id"]), None)
    else:
        at = next((i for i, c in enumerate(combos) if c["name"].lower() == name.lower()), None)
    if at is not None:
        if any(j != at and c["name"].lower() == name.lower() for j, c in enumerate(combos)):
            raise Refused("invalid_input", f"Projede “{name}” adlı bir seçenek bileşimi var.", "operation.name")
        if combos[at]["name"] == name and combos[at]["choices"] == picked:
            raise Refused("nothing_to_set", f"“{name}” bileşimi zaten bu seçeneklerle kayıtlı.", "operation")
        combos[at]["name"], combos[at]["choices"] = name, picked
        return combos[at]["id"]
    if "id" in op and not (1 <= len(op["id"]) <= 64 and all(c in "abcdefghijklmnopqrstuvwxyz0123456789-" for c in op["id"])):
        raise Refused("invalid_input", f"“{op['id']}” bileşim kimliği olamaz: küçük harf, rakam ve tire, en çok 64 karakter.", "operation.id")
    if any(c["name"].lower() == name.lower() for c in combos):
        raise Refused("invalid_input", f"Projede “{name}” adlı bir seçenek bileşimi var.", "operation.name")
    if len(combos) >= MAX:
        raise Refused("invalid_input", f"Bir projede en çok {MAX} seçenek bileşimi olur.", "operation.name")
    cid = op.get("id") or combo_id(combos, name)
    combos.append({"id": cid, "name": name, "choices": picked})
    return cid


def edit(st, op):
    """One `cad.options.edit` on `st`: (output, warnings), or Refused (nothing changed)."""
    saved = st.snapshot()
    try:
        k = op["kind"]
        if k in ("saveCombo", "applyCombo", "deleteCombo"):
            st.doc.pruned()
            combo = None
            if k == "saveCombo":
                combo = save_combo(st, op)
            elif k == "applyCombo":
                ci = combo_at(st.combos, op["combo"])
                changed = False
                for ch in st.combos[ci]["choices"]:
                    s = next((s for s in st.doc.sets if s["id"] == ch["set"]), None)
                    if s is not None and any(o["id"] == ch["option"] for o in s["options"]) and s["primary"] != ch["option"]:
                        s["primary"] = ch["option"]
                        changed = True
                if not changed:
                    raise Refused("nothing_to_set", f"“{st.combos[ci]['name']}” bileşiminin seçenekleri zaten birincil.", "operation.combo")
                combo = st.combos[ci]["id"]
            else:
                ci = combo_at(st.combos, op["combo"])
                cid = st.combos[ci]["id"]
                views = [n for n, r in st.views.items() if r.get("combo") == cid]
                if views:
                    raise Refused("option_combo_in_use",
                                  f"“{st.combos[ci]['name']}” bileşimini {len(views)} görünümün tarifi gösteriyor "
                                  f"({', '.join(views)}); önce tariflerden kaldırın.", "operation.combo")
                del st.combos[ci]
            out = {}
            c = next((c for c in st.combos if c["id"] == combo), None)
            if c is not None:
                out["combo"] = clean_combo(c)
            out["revision"] = "$current"
        else:
            out = oc.edit(st.doc, op)
        dropped = drop_gone(st.doc.sets, st.combos)
    except Refused:
        st.restore(saved)
        raise
    warnings = [{"code": "option_combo_dropped",
                 "message": f"Kalkan seçenekler {len(dropped)} bileşiminden düştü: {', '.join(dropped)}; o kümelerde birincil gösterilir."}] if dropped else []
    return out, warnings


def ring_area(pts):
    return oc.ring_area(pts)


def combo_rows(st):
    sets, doc = st.doc.sets, st.doc
    out = []
    for c in st.combos:
        choices = []
        for s in sets:
            own = next((ch["option"] for ch in c.get("choices", [])
                        if ch["set"] == s["id"] and any(o["id"] == ch["option"] for o in s["options"])), s["primary"])
            choices.append({"set": s["id"], "option": own})
        current = all(any(s["id"] == ch["set"] and s["primary"] == ch["option"] for s in sets) for ch in choices)
        hidden = {m for s, ch in zip(sets, choices) for o in s["options"] if o["id"] != ch["option"] for m in o.get("members", [])}
        objects, area, length = 0, 0.0, 0.0
        for slot in sorted(doc.ents):
            if doc.uid[slot] in hidden:
                continue
            e = doc.ents[slot]
            objects += 1
            if e["kind"] == "space":
                for part in e["parts"]:
                    area += ring_area(part["outer"]["pts"]) - sum(ring_area(h["pts"]) for h in part.get("holes", []))
            if e["kind"] == "wall":
                pts = e["pts"]
                length += sum(math.hypot(pts[i + 1]["x"] - pts[i]["x"], pts[i + 1]["y"] - pts[i]["y"]) for i in range(len(pts) - 1))
        out.append({"id": c["id"], "name": c["name"], "choices": choices, "current": current, "objects": objects,
                    "spaceArea": area, "wallLength": length})
    return out


class Case:
    def __init__(self, name, other=False):
        self.name, self.other = name, other
        self.st = State()
        self.steps = []

    def _step(self, op, inp, result, expect=None):
        step = {"op": op, "input": inp, "result": result}
        if self.other:
            step = {"op": op, "command": "cad.options.edit", "input": inp, "result": result}
        if expect:
            step["expect"] = expect
        self.steps.append(step)

    def edit(self, op, quiet=False, plan=False, expect=None):
        before = sorted(self.st.doc.ents)
        saved = self.st.snapshot()
        out = {}
        try:
            out, warnings = edit(self.st, op)
            result = ANY if quiet else {"status": "completed", "output": oc.mapped(out), "warnings": warnings}
            if plan:
                self.st.restore(saved)
                expect = {**(expect or {}), "revision": "same"}
        except Refused as r:
            result = {"status": r.status, "error": dict(r.error)}
            expect = {**(expect or {}), "ids": before, "revision": "same"}
        self._step("plan" if plan else "execute", {"operation": op}, result, expect)
        # The copies' ids, captured to name them once deleted.
        if not plan and result.get("status") != "failed":
            for u in oc.mapped(out).get("created", []):
                self.steps.append({"op": "captureUid", "id": int(u.split(":")[1]), "as": "c" + u.split(":")[1]})
        return self

    def setup(self, *ops):
        for op in ops:
            self.edit(op, quiet=True)
        return self

    def view(self, name, recipe, refused=None):
        """Saves a view `name` and gives it `recipe` through `cad.views.edit`; `refused` the recipe's refusal."""
        save = {"edit": {"kind": "save", "name": name, "center": P(3, 2), "width": 8.0, "height": 6.0}}
        self.steps.append({"op": "execute", "command": "cad.views.edit", "input": save, "result": ANY})
        inp = {"edit": {"kind": "recipe", "name": name, "recipe": recipe}}
        if refused is None:
            self.st.views[name] = recipe
            self.steps.append({"op": "execute", "command": "cad.views.edit", "input": inp, "result": ANY})
        else:
            self.steps.append({"op": "execute", "command": "cad.views.edit", "input": inp,
                               "result": {"status": "failed", "error": refused}})
        return self

    def run(self, command, inp, change):
        change(self.st)
        self.steps.append({"op": "execute", "command": command, "input": inp, "result": ANY})
        return self

    def query(self, q=None):
        q = q or {}
        try:
            out, warnings = oc.query(self.st.doc, q)
            rows = combo_rows(self.st)
            if rows:
                out = {k: v for k, v in out.items() if k != "revision"}
                out["combos"] = rows
                out["revision"] = "$current"
            result = {"status": "completed", "output": oc.mapped(out), "warnings": warnings}
        except Refused as r:
            result = {"status": "failed", "error": r.error}
        self.steps.append({"op": "execute", "command": "cad.options.query", "input": q, "result": result, "within": 1e-9})
        return self

    def undo(self, state):
        self.st.restore(state)
        self.steps.append({"op": "undo", "returns": STEP, "expect": {"revision": "changed", "canRedo": True}})
        return self

    def done(self):
        return {"name": self.name, "steps": self.steps}


def two_sets(c):
    return c.setup(KITCHEN, KITCHEN_B, NOTE, NOTE_B)


def ch(s, o):
    return {"set": s, "option": o}


def edit_cases():
    out = []
    c = two_sets(Case("Kaydet: seçimsiz bileşim kümelerin şimdiki birincillerini tutar"))
    c.edit({"kind": "saveCombo", "name": "Teklif 1"})
    out.append(c.done())

    c = two_sets(Case("Kaydet: seçimler küme ve seçeneğin adıyla verilir"))
    c.edit({"kind": "saveCombo", "name": "Teklif 2", "choices": [ch("Mutfak", "B"), ch("not", "a")]})
    out.append(c.done())

    c = two_sets(Case("Kaydet: kimlik Türkçe harfleri katlanmış addan"))
    c.edit({"kind": "saveCombo", "name": "Öneri Çağrı Şubat", "choices": [ch("mutfak", "b")]})
    out.append(c.done())

    c = two_sets(Case("Kaydet: kimliği alınmış kök numaralanır"))
    c.edit({"kind": "saveCombo", "name": "Teklif 1"}, quiet=True)
    c.edit({"kind": "saveCombo", "name": "Teklif-1", "choices": [ch("not", "b")]})
    out.append(c.done())

    c = two_sets(Case("Kaydet: harfsiz adın kimliği bilesim"))
    c.edit({"kind": "saveCombo", "name": "1+1"})
    c.edit({"kind": "saveCombo", "name": "!!", "choices": [ch("mutfak", "b")]})
    out.append(c.done())

    c = two_sets(Case("Kaydet: kimlik verilir"))
    c.edit({"kind": "saveCombo", "name": "Teklif", "id": "t1", "choices": [ch("mutfak", "b"), ch("not", "b")]})
    out.append(c.done())

    c = two_sets(Case("Kaydet: aynı adla yeniden kaydetmek seçimleri değiştirir"))
    c.edit({"kind": "saveCombo", "name": "Teklif 1"}, quiet=True)
    c.edit({"kind": "saveCombo", "name": "teklif 1", "choices": [ch("mutfak", "b")]})
    out.append(c.done())

    c = two_sets(Case("Kaydet: kimlikle yeniden kaydetmek adı değiştirir"))
    c.edit({"kind": "saveCombo", "name": "Teklif 1"}, quiet=True)
    c.edit({"kind": "saveCombo", "name": "Son teklif", "id": "teklif-1"})
    out.append(c.done())

    c = two_sets(Case("Kaydet'in planı yazmaz"))
    c.edit({"kind": "saveCombo", "name": "Teklif 1"}, plan=True)
    c.query()
    out.append(c.done())

    c = two_sets(Case("Ret: aynı seçimlerle kayıtlı bileşim"))
    c.edit({"kind": "saveCombo", "name": "Teklif 1"}, quiet=True)
    c.edit({"kind": "saveCombo", "name": "Teklif 1"})
    out.append(c.done())

    c = Case("Ret: projede seçenek kümesi yok")
    c.edit({"kind": "saveCombo", "name": "Teklif 1"})
    out.append(c.done())

    c = two_sets(Case("Ret: seçimde olmayan küme"))
    c.edit({"kind": "saveCombo", "name": "Teklif", "choices": [ch("mutfak", "a"), ch("banyo", "a")]})
    out.append(c.done())

    c = two_sets(Case("Ret: seçimde olmayan seçenek"))
    c.edit({"kind": "saveCombo", "name": "Teklif", "choices": [ch("mutfak", "a"), ch("not", "z")]})
    out.append(c.done())

    c = two_sets(Case("Ret: bir küme iki kez"))
    c.edit({"kind": "saveCombo", "name": "Teklif", "choices": [ch("mutfak", "a"), ch("Mutfak", "b")]})
    out.append(c.done())

    c = two_sets(Case("Ret: boş ad"))
    c.edit({"kind": "saveCombo", "name": "  "})
    out.append(c.done())

    c = two_sets(Case("Ret: geçersiz kimlik"))
    c.edit({"kind": "saveCombo", "name": "Teklif", "id": "Teklif 1"})
    out.append(c.done())

    c = two_sets(Case("Ret: başka bileşimin adı"))
    c.edit({"kind": "saveCombo", "name": "Teklif 1"}, quiet=True)
    c.edit({"kind": "saveCombo", "name": "Teklif 2", "choices": [ch("mutfak", "b")]}, quiet=True)
    c.edit({"kind": "saveCombo", "name": "teklif 1", "id": "teklif-2"})
    out.append(c.done())

    c = two_sets(Case("Uygula: bileşimin seçenekleri birincil olur"))
    c.edit({"kind": "saveCombo", "name": "Teklif B", "choices": [ch("mutfak", "b"), ch("not", "b")]}, quiet=True)
    c.edit({"kind": "applyCombo", "combo": "Teklif B"})
    c.query()
    out.append(c.done())

    c = two_sets(Case("Uygula ve geri al: birincil eski seçenekler"))
    c.edit({"kind": "saveCombo", "name": "Teklif B", "choices": [ch("mutfak", "b")]}, quiet=True)
    s = c.st.snapshot()
    c.edit({"kind": "applyCombo", "combo": "teklif-b"})
    c.undo(s)
    c.query()
    out.append(c.done())

    c = two_sets(Case("Ret: bileşimin seçenekleri zaten birincil"))
    c.edit({"kind": "saveCombo", "name": "Teklif 1"}, quiet=True)
    c.edit({"kind": "applyCombo", "combo": "teklif-1"})
    out.append(c.done())

    c = two_sets(Case("Ret: uygulanan bileşim yok"))
    c.edit({"kind": "applyCombo", "combo": "Teklif 9"})
    out.append(c.done())

    c = two_sets(Case("Sil: bileşim kalkar"))
    c.edit({"kind": "saveCombo", "name": "Teklif 1"}, quiet=True)
    c.edit({"kind": "saveCombo", "name": "Teklif 2", "choices": [ch("not", "b")]}, quiet=True)
    c.edit({"kind": "deleteCombo", "combo": "Teklif 1"})
    c.query()
    out.append(c.done())

    c = two_sets(Case("Ret: silinen bileşim yok"))
    c.edit({"kind": "deleteCombo", "combo": "teklif-1"})
    out.append(c.done())

    c = two_sets(Case("Ret: görünümün tarifi gösteren bileşim silinmez"))
    c.edit({"kind": "saveCombo", "name": "Teklif 1"}, quiet=True)
    c.view("Teklif planı", {"combo": "teklif-1"})
    c.view("Teklif kesiti", {"level": {"id": "zemin"}, "combo": "teklif-1"})
    c.edit({"kind": "deleteCombo", "combo": "teklif-1"})
    out.append(c.done())

    c = two_sets(Case("Kabul et: kalkan kümenin seçimi bileşimlerden düşer, uyarılır"))
    c.edit({"kind": "saveCombo", "name": "Teklif B", "choices": [ch("mutfak", "b"), ch("not", "b")]}, quiet=True)
    c.edit({"kind": "saveCombo", "name": "Teklif N", "choices": [ch("not", "b")]}, quiet=True)
    c.edit({"kind": "accept", "set": "mutfak", "option": "a"})
    c.query()
    out.append(c.done())

    c = two_sets(Case("Seçeneği sil: silinen seçeneğin seçimi düşer, uyarılır"))
    c.edit({"kind": "saveCombo", "name": "Teklif B", "choices": [ch("mutfak", "b"), ch("not", "a")]}, quiet=True)
    c.edit({"kind": "delete", "set": "mutfak", "option": "b"})
    c.query()
    out.append(c.done())

    c = two_sets(Case("Yeniden adlandır bileşimleri değiştirmez"))
    c.edit({"kind": "saveCombo", "name": "Teklif B", "choices": [ch("mutfak", "b")]}, quiet=True)
    c.edit({"kind": "rename", "set": "mutfak", "option": "b", "name": "Ada"})
    c.query()
    out.append(c.done())
    return out


def view_cases():
    """A view recipe's combination (docs/adr/0808 §3) through `cad.views.edit`."""
    out = []
    c = two_sets(Case("Görünüş tarifi: bileşimi gösterir"))
    c.edit({"kind": "saveCombo", "name": "Teklif B", "choices": [ch("mutfak", "b")]}, quiet=True)
    c.view("Teklif B", {"combo": "teklif-b"})
    out.append(c.done())

    c = two_sets(Case("Görünüş tarifi: kendi seçeneği ve bileşim birlikte"))
    c.edit({"kind": "saveCombo", "name": "Teklif B", "choices": [ch("mutfak", "b"), ch("not", "b")]}, quiet=True)
    c.view("Teklif B", {"options": [ch("not", "a")], "combo": "teklif-b"})
    out.append(c.done())

    c = two_sets(Case("Ret: tarifte olmayan bileşim"))
    c.view("Teklif", {"combo": "teklif-9"}, {
        "code": "option_combo_not_found",
        "message": "Projede “teklif-9” kimlikli seçenek bileşimi yok. Tasarım seçenekleri › Bileşimler'den birinin kimliğini verin.",
        "path": "edit.recipe.combo"})
    out.append(c.done())

    c = two_sets(Case("Ret: tarifin bileşimi kimlik değil"))
    c.edit({"kind": "saveCombo", "name": "Teklif B", "choices": [ch("mutfak", "b")]}, quiet=True)
    c.view("Teklif", {"combo": "Teklif B"}, {
        "code": "invalid_recipe",
        "message": "Görünüş tarifinin bileşimi bir bileşimin kimliğiyle verilir. Tarifi düzeltip yeniden verin.",
        "path": "edit.recipe"})
    out.append(c.done())

    c = two_sets(Case("Ret: şablon bileşim taşımaz"))
    c.edit({"kind": "saveCombo", "name": "Teklif B", "choices": [ch("mutfak", "b")]}, quiet=True)
    c.steps.append({"op": "execute", "command": "cad.views.edit",
                    "input": {"edit": {"kind": "template", "name": "Uygulama", "recipe": {"scale": 50, "combo": "teklif-b"}}},
                    "result": {"status": "failed", "error": {
                        "code": "invalid_recipe",
                        "message": "Görünüş şablonu tasarım seçeneği taşımaz: hangi seçeneğin gösterileceği görünümündür. Şablonu düzeltip yeniden verin.",
                        "path": "edit.recipe"}}})
    out.append(c.done())

    c = two_sets(Case("Şablona bağlı tarif kendi bileşimini tutar"))
    c.edit({"kind": "saveCombo", "name": "Teklif B", "choices": [ch("mutfak", "b")]}, quiet=True)
    c.steps.append({"op": "execute", "command": "cad.views.edit",
                    "input": {"edit": {"kind": "template", "name": "Uygulama", "recipe": {"scale": 50}}}, "result": ANY})
    c.view("Teklif B", {"template": "Uygulama", "combo": "teklif-b"})
    out.append(c.done())
    return out


def query_cases():
    out = []
    c = two_sets(Case("Bileşimsiz projede okumada bileşim yok", other=True))
    c.query()
    out.append(c.done())

    c = two_sets(Case("Birincillerin bileşimi şimdiki plandır", other=True))
    c.edit({"kind": "saveCombo", "name": "Teklif 1"}, quiet=True)
    c.query()
    out.append(c.done())

    c = two_sets(Case("B seçenekli bileşim şimdiki değil; planın nesneleri, alanı, duvar boyu", other=True))
    c.edit({"kind": "saveCombo", "name": "Teklif B", "choices": [ch("mutfak", "b"), ch("not", "b")]}, quiet=True)
    c.query()
    out.append(c.done())

    c = two_sets(Case("Bir küme seçen bileşim öbür kümede birincili gösterir", other=True))
    c.edit({"kind": "saveCombo", "name": "Teklif N", "choices": [ch("not", "b")]}, quiet=True)
    c.query()
    out.append(c.done())

    c = two_sets(Case("Seçimsiz liste: bütün kümeler birincil", other=True))
    c.edit({"kind": "saveCombo", "name": "Boş", "choices": []}, quiet=True)
    c.query()
    out.append(c.done())

    c = two_sets(Case("Uygulanınca bileşim şimdiki olur, öbürü değil", other=True))
    c.edit({"kind": "saveCombo", "name": "Teklif 1"}, quiet=True)
    c.edit({"kind": "saveCombo", "name": "Teklif B", "choices": [ch("mutfak", "b")]}, quiet=True)
    c.edit({"kind": "applyCombo", "combo": "Teklif B"}, quiet=True)
    c.query()
    out.append(c.done())

    c = two_sets(Case("Bileşimler kaydedildikleri sırada", other=True))
    for n, chs in (("Üç", [ch("not", "b")]), ("Bir", [ch("mutfak", "b")]), ("İki", [ch("mutfak", "b"), ch("not", "b")])):
        c.edit({"kind": "saveCombo", "name": n, "choices": chs}, quiet=True)
    c.query()
    out.append(c.done())

    c = two_sets(Case("Kümeyle süzülen okuma bütün bileşimleri verir", other=True))
    c.edit({"kind": "saveCombo", "name": "Teklif B", "choices": [ch("mutfak", "b")]}, quiet=True)
    c.query({"set": "not"})
    out.append(c.done())

    c = two_sets(Case("Nesneyle okuma bütün bileşimleri verir", other=True))
    c.edit({"kind": "saveCombo", "name": "Teklif B", "choices": [ch("mutfak", "b")]}, quiet=True)
    c.query({"uid": UID(7)})
    out.append(c.done())

    c = two_sets(Case("Karşılaştırmayla bileşimler", other=True))
    c.edit({"kind": "saveCombo", "name": "Teklif B", "choices": [ch("mutfak", "b")]}, quiet=True)
    c.query({"compare": {"set": "mutfak", "a": "a", "b": "b"}})
    out.append(c.done())

    c = two_sets(Case("Kabul edilen kümenin seçimi düşmüş bileşim", other=True))
    c.edit({"kind": "saveCombo", "name": "Teklif B", "choices": [ch("mutfak", "b"), ch("not", "b")]}, quiet=True)
    c.edit({"kind": "accept", "set": "not", "option": "b"}, quiet=True)
    c.query()
    out.append(c.done())

    c = two_sets(Case("Silinen nesne bileşimin planında sayılmaz", other=True))
    c.edit({"kind": "saveCombo", "name": "Teklif B", "choices": [ch("mutfak", "b")]}, quiet=True)

    def gone(st):
        del st.doc.ents[18]
    c.run("cad.entities.delete", {"uids": ["$uidOf:18"]}, gone)
    c.query()
    out.append(c.done())

    c = two_sets(Case("Seçenekten çıkan nesne her bileşimde görünür", other=True))
    c.edit({"kind": "saveCombo", "name": "Teklif B", "choices": [ch("mutfak", "b")]}, quiet=True)
    c.edit({"kind": "removeMembers", "set": "mutfak", "uids": [UID(9)]}, quiet=True)
    c.query()
    out.append(c.done())

    c = two_sets(Case("Silinen bileşim okumada yok", other=True))
    c.edit({"kind": "saveCombo", "name": "Teklif B", "choices": [ch("mutfak", "b")]}, quiet=True)
    c.edit({"kind": "saveCombo", "name": "Teklif N", "choices": [ch("not", "b")]}, quiet=True)
    c.edit({"kind": "deleteCombo", "combo": "teklif-b"}, quiet=True)
    c.query()
    out.append(c.done())

    c = two_sets(Case("Yeniden adlandırılan bileşim", other=True))
    c.edit({"kind": "saveCombo", "name": "Teklif B", "choices": [ch("mutfak", "b")]}, quiet=True)
    c.edit({"kind": "saveCombo", "name": "Ada mutfak", "id": "teklif-b"}, quiet=True)
    c.query()
    out.append(c.done())

    c = two_sets(Case("Üç seçenekli kümede C'yi seçen bileşim", other=True))
    c.edit({"kind": "copy", "set": "mutfak", "option": "a", "name": "Geniş"}, quiet=True)
    c.edit({"kind": "saveCombo", "name": "Teklif C", "choices": [ch("mutfak", "Geniş")]}, quiet=True)
    c.query()
    out.append(c.done())

    c = two_sets(Case("Birincil yapılan seçenek bileşimi şimdiki yapar", other=True))
    c.edit({"kind": "saveCombo", "name": "Teklif B", "choices": [ch("mutfak", "b")]}, quiet=True)
    c.edit({"kind": "primary", "set": "mutfak", "option": "b"}, quiet=True)
    c.query()
    out.append(c.done())

    c = two_sets(Case("Seçeneği silinen bileşim birincili gösterir", other=True))
    c.edit({"kind": "saveCombo", "name": "Teklif N", "choices": [ch("not", "b")]}, quiet=True)
    c.edit({"kind": "delete", "set": "not", "option": "b"}, quiet=True)
    c.query()
    out.append(c.done())

    c = two_sets(Case("Uygulanıp geri alınan bileşim", other=True))
    c.edit({"kind": "saveCombo", "name": "Teklif B", "choices": [ch("mutfak", "b")]}, quiet=True)
    s = c.st.snapshot()
    c.edit({"kind": "applyCombo", "combo": "teklif-b"}, quiet=True)
    c.undo(s)
    c.query()
    out.append(c.done())

    c = two_sets(Case("Ret: okumada olmayan küme", other=True))
    c.edit({"kind": "saveCombo", "name": "Teklif B", "choices": [ch("mutfak", "b")]}, quiet=True)
    c.query({"set": "banyo"})
    out.append(c.done())

    c = Case("Tek kümeli projede bileşim", other=True)
    c.setup(KITCHEN, KITCHEN_B)
    c.edit({"kind": "saveCombo", "name": "Teklif B", "choices": [ch("mutfak", "b")]}, quiet=True)
    c.query()
    out.append(c.done())
    return out


def write(command, title, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "scripts/fixtures/option_combo_cases.py yazar: ADR 0808'in kurallarından, KentOS kodu olmadan. "
                    "Değiştirmeyin; betiği düzeltin.",
            "setup": oc.setup(), "cases": cases}


def main():
    files = {
        EDIT: write("cad.options.edit", "Tasarım seçeneklerinin bileşimleri (docs/adr/0808)", edit_cases() + view_cases()),
        QUERY: write("cad.options.query", "Bileşimleri oku (docs/adr/0808)", query_cases()),
    }
    check = "--check" in sys.argv
    bad = False
    for path, data in files.items():
        assert len(data["cases"]) >= 20, (path, len(data["cases"]))
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
