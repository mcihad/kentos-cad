#!/usr/bin/env python3
"""Shared cases of the bonds in the way and of Bağı göster / kaldır (docs/adr/0780), from the ADRs' rules, without
KentOS code.

Writes fixtures/commands/v1/cad.walls.edit.bonds.json and cad.constraints.query.arch.json; `--check` compares and
writes nothing. Each case's drawing is arch_bond_cases.py's (two rooms, a corridor, three doors, a locked wall) with its
bonds already in it under fixed ids, so a refusal is written whole:

- a write the bonds cannot follow (an edit, a parameter's new value, a new bond) is refused with `constraint_conflict`
  as ADR 0770 §4 says (arch_bond_cases.follow: the same linear rows, holds and least change, Kaydır by
  wall_shift_cases.py); its `uids` are the conflicting constraints in the set's order and its `bonds` each of them:
  the parts it names (`a`, `b`, `axis`), its label, its parameter, the value it keeps (metres), its clear distance as
  the write leaves the drawing before the bonds follow (along the wall for two jambs), and `frees`: the bonds solve
  the same write with that one taken away alone (docs/adr/0780 §1). The message ends naming the ones that free it;
- `cad.constraints.query`'s `ids` (only these constraints; `constraint_not_found`, `invalid_uid`) and `architectural`
  (only the bonds), each constraint's `kept` (its parameter's value, none for a reference) and an architectural bond's
  `holds` measured on its own (to 1e-6 m), every one that does not hold (docs/adr/0780 §2);
- a bond taken away (`cad.constraints.edit` `remove`, its parameter with it) and the edit then goes through.

    python3 scripts/fixtures/arch_bond_conflict_cases.py
    python3 scripts/fixtures/arch_bond_conflict_cases.py --check
"""

import argparse
import copy
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import arch_bond_cases as ab  # noqa: E402  (the bonds' rules, independently)

ws = ab.ws
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
SETTLED = ab.SETTLED
W, J, u, dist = ab.W, ab.J, ab.u, ab.dist

# A plain line beside the rooms, for a general (not architectural) constraint.
LINE = {"kind": "line", "id": 13, "layerId": "duvar", "attrs": {}, "a": ab.P(0, 30), "b": ab.P(4, 30)}
ENTITIES = ab.ENTITIES + [LINE]
IDS = [e["id"] for e in ENTITIES]

CORRIDOR = ab.CORRIDOR                                       # S's inner face to C's lower face: 1.20
DEPTH = dist(W(6, "leftFace"), W(3, "leftFace"))             # C's upper face to N's inner face
FIX_S = {"kind": "fix", "a": W(1, "axis")}
FIX_S2 = {"kind": "fix", "a": W(1, "leftFace")}
FIX_C = {"kind": "fix", "a": W(6, "axis")}
FIX_N = {"kind": "fix", "a": W(3, "axis")}
FIX_P = {"kind": "fix", "a": W(5, "axis")}
LOCKED = dist(W(11, "leftFace"), W(12, "rightFace"))
SPACING = {"kind": "symmetric", "a": W(4, "leftFace"), "b": W(2, "leftFace"), "axis": W(5, "axis")}
DOOR_WEST = dist(J(7, "start"), W(4, "leftFace"))
FIX_DOOR = {"kind": "fix", "a": J(7, "start")}
DOORS = dist(J(7, "end"), J(8, "start"))
PARALLEL = {"kind": "parallel", "a": W(1, "axis"), "b": W(3, "axis")}
LEVEL = {"kind": "horizontal", "a": {"uid": u(13), "element": {"kind": "segment", "part": 0, "ring": 0, "index": 0}}}


def cid(k):
    """The setup's constraint ids: one per constraint, in order."""
    return f"0192f5a1-7c00-7000-8000-{k:012x}"


class Scene:
    """A drawing with its bonds: `bonds` as (rule, value or None, reference) in order; their ids `cid(1)` …"""

    def __init__(self, name, bonds):
        self.name = name
        self.state = {e["id"]: copy.deepcopy(e) for e in ENTITIES}
        self.rules, self.params, self.ids, self.reference = [], [], [], []
        for k, item in enumerate(bonds, start=1):
            rule, value, reference = (item + (None, False))[:3] if isinstance(item, tuple) else (item, None, False)
            r = copy.deepcopy(rule)
            if r["kind"] == "distance":
                g, side = ab.measured(self.state, r)
                jj = r["a"]["element"]["kind"] == "jamb" and r["b"]["element"]["kind"] == "jamb"
                if jj and g < 0:
                    r["a"], r["b"] = r["b"], r["a"]
                    g = -g
                if side and not jj:
                    r["left"] = True
                name = f"d{1 + sum(1 for n, _ in self.params if n.startswith('d'))}"
                r["parameter"] = name
                self.params.append((name, "" if reference else (value if value is not None else ab.wc_value(g))))
            self.rules.append(r)
            self.ids.append(cid(k))
            self.reference.append(reference)
        self.steps = []

    # ── What the drawing holds ───────────────────────────────────────────────────────────────────────────────────

    def arch(self):
        """The architectural rules (their indices in the set) and their constraint ids."""
        keep = [i for i, r in enumerate(self.rules) if r["kind"] != "horizontal"]
        return keep

    def values(self, params=None):
        out = {}
        for i, (n, e) in enumerate(params if params is not None else self.params):
            if e:
                out[n] = float(e)
            else:  # a reference's parameter: what it measures
                r = next(r for r in self.rules if r.get("parameter") == n)
                out[n] = ab.measured(self.state, r)[0]
        return out

    def setup(self):
        s = copy.deepcopy(ab.SETUP)
        s["entities"] = copy.deepcopy(ENTITIES)
        s["uids"] = [ws.uid(e["id"]) for e in ENTITIES]
        cons = []
        for i, r in enumerate(self.rules):
            c = {"id": self.ids[i], "rule": r}
            if self.reference[i]:
                c["reference"] = True
            cons.append(c)
        params = []
        for n, e in self.params:
            p = {"name": n, "kind": "length"}
            if e:
                p["expression"] = e
            params.append(p)
        # An empty list is left out, as the writer leaves it.
        s["constraints"] = {**({"parameters": params} if params else {}), "constraints": cons}
        return s

    def bond_rules(self):
        """The rules the bonds' solver sees: the architectural ones that drive (a reference only measures)."""
        idx = [i for i in self.arch() if not self.reference[i]]
        return idx, [self.rules[i] for i in idx]

    def kinds(self):
        return [r["kind"] for r in self.rules]

    def param_list(self):
        return [{"name": n, "kind": "length", **({"expression": e} if e else {})} for n, e in self.params]

    # ── Steps ────────────────────────────────────────────────────────────────────────────────────────────────────

    def refusal(self, words, conflicting, written, values, edited, idx, rules, path=None, add=None):
        """The refusal: the conflicting rules (indices into `rules`, which are the set's `idx`)."""
        ids = [self.ids[idx[k]] if idx[k] < len(self.ids) else "$any" for k in range(len(idx))]
        bonds = ab.in_the_way(written, rules, values, conflicting, edited, ids=ids)
        for b, k in zip(bonds, conflicting):
            if self.reference[idx[k]] if idx[k] < len(self.reference) else False:
                b.pop("kept", None)
        err = {"code": "constraint_conflict", "message": words(bonds)}
        if path:
            err["path"] = path
        err["uids"] = [b["id"] for b in bonds]
        err["bonds"] = bonds
        return {"status": "failed", "error": err}

    def edit(self, command, inp, write, op="execute"):
        """Another command's write (`write(state)` gives its changed objects or a refusal), then the bonds."""
        before = {i: copy.deepcopy(e) for i, e in self.state.items()}
        got = write(self.state)
        step = {"op": op, "command": command, "input": inp}
        if isinstance(got, dict) and "status" in got:
            step["result"] = got
            step["expect"] = {"ids": IDS, "revision": "same"}
            self.steps.append(step)
            return
        written = dict(self.state)
        written.update(got)
        edited = {i: before[i] for i in got}
        idx, rules = self.bond_rules()
        named = {ab.el_id(r[k]) for r in rules for k in ("a", "b", "axis") if type(r.get(k)) is dict}
        values = self.values()
        if named & set(got):
            changed, why = ab.follow(written, rules, values, edited=edited)
        else:
            changed, why = {}, None
        if why is not None:
            step["within"] = 1e-9
            step["result"] = self.refusal(lambda bonds: f"{why[1]}; hiçbir şey yazılmadı. {ab.bonds_words(bonds)}",
                                          why[0], written, values, edited, idx, rules)
            step["expect"] = {"ids": IDS, "revision": "same"}
            self.steps.append(step)
            return
        if op == "plan":
            step["result"] = "$any"
            step["expect"] = {"ids": IDS, "revision": "same"}
            self.steps.append(step)
            return
        written.update(changed)
        every = dict(got)
        every.update(changed)
        self.state = written
        step["within"] = 1e-9
        step["result"] = "$any"
        step["expect"] = {"ids": IDS, "revision": "changed", "constraintKinds": self.kinds(),
                          "entities": {str(i): written[i] for i in sorted(every)}}
        self.steps.append(step)

    def shift(self, i, by, op="execute"):
        inp = {"uids": [ws.u(i)], "operation": {"kind": "shift", "by": by}}
        self.edit("cad.walls.edit", inp, lambda s: self._walls(s, inp), op)

    def distance(self, i, side, d, op="execute"):
        inp = {"uids": [ws.u(i)], "operation": {"kind": "distance", "side": side, "distance": d}}
        self.edit("cad.walls.edit", inp, lambda s: self._walls(s, inp), op)

    @staticmethod
    def _walls(state, inp):
        result, changed = ws.edit(state, inp)
        return result if changed is None else changed

    def move_opening(self, i, by):
        inp = {"uids": [ws.u(i)], "operation": {"kind": "move", "by": by}}

        def write(state):
            o = copy.deepcopy(state[i])
            o["at"] = o["at"] + by
            o["place"] = [ab.P(*p) for p in ws.jambs(state[ab.host_of(state, o)], o["at"], o["width"])]
            return {i: o}
        self.edit("cad.openings.edit", inp, write)

    def set_parameter(self, name, expression):
        """cad.parameters.edit: the set solved with the new value, each distance not holding keeping its `a`."""
        inp = {"edits": [{"kind": "set", "name": name, "expression": expression}]}
        params = [(n, expression if n == name else e) for n, e in self.params]
        values = self.values(params)
        idx, rules = self.bond_rules()
        changed, why = ab.follow(self.state, rules, values)
        step = {"op": "execute", "command": "cad.parameters.edit", "input": inp}
        if why is not None:
            step["within"] = 1e-9
            step["result"] = self.refusal(self.solve_words, why[0], self.state, values, None, idx, rules, path="edit")
            step["expect"] = {"ids": IDS, "revision": "same"}
            self.steps.append(step)
            return
        self.params = params
        self.state.update(changed)
        step["within"] = 1e-9
        step["result"] = "$any"
        step["expect"] = {"ids": IDS, "revision": "changed", "parameters": self.param_list(),
                          "entities": {str(i): self.state[i] for i in sorted(changed)}}
        self.steps.append(step)

    @staticmethod
    def solve_words(bonds):
        """cad.constraints.edit's and cad.parameters.edit's conflict: the bonds that free it, when one does."""
        if any(b["frees"] for b in bonds):
            return f"Kısıtlar birlikte tutamıyor; hiçbir şey yazılmadı. {ab.bonds_words(bonds)}"
        return ("Kısıtlar birlikte tutamıyor; hiçbir şey yazılmadı. Çelişen kısıtı kaldırın, değerini değiştirin ya "
                "da başvuru yapın.")

    def add(self, rule, value):
        """cad.constraints.edit add of one distance valued `value` that the bonds cannot take."""
        r = copy.deepcopy(rule)
        g, side = ab.measured(self.state, r)
        if side:
            r["left"] = True
        name = f"d{1 + sum(1 for n, _ in self.params if n.startswith('d'))}"
        r["parameter"] = name
        params = self.params + [(name, value)]
        values = {n: float(e) for n, e in params if e}
        idx, rules = self.bond_rules()
        idx, rules = idx + [len(self.rules)], rules + [r]
        changed, why = ab.follow(self.state, rules, values)
        assert why is not None, "the case's new bond is one the drawing cannot take"
        step = {"op": "execute", "command": "cad.constraints.edit",
                "input": {"edit": {"kind": "add", "constraints": [{"rule": rule, "value": value}]}}}
        step["within"] = 1e-9
        step["result"] = self.refusal(self.solve_words, why[0], self.state, values, None, idx, rules, path="edit")
        step["expect"] = {"ids": IDS, "revision": "same", "constraintKinds": self.kinds()}
        self.steps.append(step)

    def remove(self, *ks):
        """cad.constraints.edit remove of constraints `ks` (1-based), their parameters with them."""
        gone = [self.ids[k - 1] for k in ks]
        keep = [i for i in range(len(self.rules)) if i + 1 not in ks]
        self.rules = [self.rules[i] for i in keep]
        self.ids = [self.ids[i] for i in keep]
        self.reference = [self.reference[i] for i in keep]
        used = {r.get("parameter") for r in self.rules}
        self.params = [(n, e) for n, e in self.params if n in used]
        self.steps.append({"op": "execute", "command": "cad.constraints.edit",
                           "input": {"edit": {"kind": "remove", "ids": gone}},
                           "result": {"status": "completed", "output": {
                               "added": [], "removed": gone, "changed": [], "diagnosis": "$any",
                               "revision": "$current"}, "warnings": []},
                           "expect": {"ids": IDS, "revision": "changed", "constraintKinds": self.kinds(),
                                      "parameters": self.param_list()}})

    def query(self, uids=(), ids=(), architectural=False):
        inp = {}
        if uids:
            inp["uids"] = [u(i) for i in uids]
        if ids:
            inp["ids"] = [self.ids[k - 1] for k in ids]
        if architectural:
            inp["architectural"] = True
        values = self.values()
        listed = []
        for i, r in enumerate(self.rules):
            parts = ab.parts_of(r)
            if uids and not any(ab.el_id(p) in uids for p in parts):
                continue
            if ids and i + 1 not in ids:
                continue
            arch = r["kind"] != "horizontal"
            if architectural and not arch:
                continue
            c = {"id": self.ids[i], "rule": r}
            if self.reference[i]:
                c["reference"] = True
            item = {"constraint": c, "label": ab.LABELS.get(r["kind"], "Yatay")}
            if r["kind"] == "distance":
                item["measured"] = ab.measured_or_none(self.state, r)
                if not self.reference[i]:
                    item["kept"] = values[r["parameter"]]
            if arch:
                m = ab.miss(self.state, r, values)
                item["holds"] = m is not None and abs(m) <= SETTLED
            else:
                item["holds"] = True  # the line is level
            listed.append(item)
        self.steps.append({"op": "execute", "command": "cad.constraints.query", "input": inp, "within": 1e-9,
                           "result": {"status": "completed", "warnings": [], "output": {
                               "constraints": listed, "parameters": self.param_list(), "diagnosis": "$any",
                               "revision": "$current"}},
                           "expect": {"ids": IDS, "revision": "same"}})

    def query_refused(self, inp, code, message, path):
        self.steps.append({"op": "execute", "command": "cad.constraints.query", "input": inp,
                           "result": {"status": "failed", "error": {"code": code, "message": message, "path": path}},
                           "expect": {"ids": IDS, "revision": "same"}})

    def done(self):
        return {"name": self.name, "setup": self.setup_at_start, "steps": self.steps}


def scene(out, name, bonds):
    s = Scene(name, bonds)
    s.setup_at_start = s.setup()
    out.append(s)
    return s


def conflict_cases():
    out = []
    s = scene(out, "Sabit dış duvar ve koridor bağı: koridor duvarını kaydırmak reddedilir; iki bağ da tek başına kaldırılınca geçer",
              [CORRIDOR, FIX_S])
    s.shift(6, 0.3)
    s = scene(out, "Önizleme (plan) aynı reddi bağlarıyla verir", [CORRIDOR, FIX_S])
    s.shift(6, 0.3, op="plan")
    s = scene(out, "Koridor bağı kaldırılınca koridor duvarı kayar, dış duvar sabit kalır", [CORRIDOR, FIX_S])
    s.shift(6, 0.3)
    s.remove(1)
    s.shift(6, 0.3)
    s = scene(out, "Sabit bağ kaldırılınca koridor duvarı kayar, dış duvar bağla izler", [CORRIDOR, FIX_S])
    s.shift(6, 0.3)
    s.remove(2)
    s.shift(6, 0.3)
    s = scene(out, "Sabit duvarı kaydırmak: sabit bağ yolda, kaldırılınca geçer", [FIX_S])
    s.shift(1, -0.5)
    s = scene(out, "Aynı duvara iki sabit bağ: hiçbiri tek başına geçirmez", [FIX_S, FIX_S2])
    s.shift(1, -0.5)
    s = scene(out, "Kilitli katmandaki duvara bağ: Kaydır'ın reddi bağın reddi", [LOCKED])
    s.shift(12, 0.4)
    s = scene(out, "Bağ zinciri sabit kuzey duvarında biter: üç bağ da yolda", [CORRIDOR, DEPTH, FIX_N])
    s.shift(1, -0.5)
    s = scene(out, "Yolda olmayan bağ adlandırılmaz: kapının batı duvarı bağı", [CORRIDOR, FIX_S, DOOR_WEST])
    s.shift(6, 0.3)
    s = scene(out, "Kapının pervazı batı duvarına bağlı ve sabit: batı duvarını kaydırmak reddedilir", [DOOR_WEST, FIX_DOOR])
    s.shift(4, -0.5)
    s = scene(out, "İki kapı arası bağ ve sabit ilk kapı: ikinci kapıyı taşımak reddedilir", [DOORS, FIX_DOOR])
    s.move_opening(8, 0.4)
    s = scene(out, "Eşit aralığın eksenini kaydırmak: simetrik bağ yolda", [SPACING])
    s.shift(5, 0.3)
    s = scene(out, "Eşit aralık ve sabit bölme: doğu duvarını kaydırmak reddedilir", [SPACING, FIX_P])
    s.shift(2, -1.0)
    s = scene(out, "Sabit bölme kaldırılınca doğu duvarı kayar, bölme ortada kalır", [SPACING, FIX_P])
    s.shift(2, -1.0)
    s.remove(2)
    s.shift(2, -1.0)
    s = scene(out, "Mesafe ile koridoru 1,50 m yapmak: koridor bağı ve sabit dış duvar yolda", [CORRIDOR, FIX_S])
    s.distance(6, "right", 1.5)
    s = scene(out, "Mesafe'nin önizlemesi aynı bağları adlandırır", [CORRIDOR, FIX_S])
    s.distance(6, "right", 1.5, op="plan")
    s = scene(out, "Aynı yüzler arasında iki farklı değer: dış duvarı kaydırmak iki bağı da adlandırır",
              [CORRIDOR, (CORRIDOR, "1.5")])
    s.shift(1, -0.5)
    s = scene(out, "Parametrenin yeni değeri sabit iki duvara: bağ ve koridor duvarının sabiti yolda",
              [CORRIDOR, FIX_S, FIX_C])
    s.set_parameter("d1", "1.5")
    s = scene(out, "Yeni bağ sabit iki duvarın arasına başka değerle: yeni bağ ve koridor duvarının sabiti yolda",
              [FIX_S, FIX_C])
    s.add(CORRIDOR, "1.5")
    s = scene(out, "Yolda olmayan duvar bağlarla kaydırılır: kuzey duvarı serbestçe", [CORRIDOR, FIX_S])
    s.shift(3, 0.3)
    s = scene(out, "Paralel bağ kaydırmayı engellemez; sabit ve koridor yine yolda", [PARALLEL, CORRIDOR, FIX_S])
    s.shift(6, 0.3)
    s = scene(out, "Reddedilen kaydırmadan sonra bağ kaldırılıp yeniden denenir, sonra parametre değişir",
              [CORRIDOR, FIX_C])
    s.shift(1, -0.5)
    s.remove(2)
    s.shift(1, -0.5)
    s.set_parameter("d1", "1.4")
    return [x.done() for x in out]


def query_cases():
    out = []
    s = scene(out, "Bütün kısıtlar: mimari bağlar ve genel kısıt, tuttukları ve tutulan değerleri",
              [CORRIDOR, FIX_S, LEVEL])
    s.query()
    s = scene(out, "Yalnız mimari bağlar (Bağı göster)", [CORRIDOR, FIX_S, LEVEL])
    s.query(architectural=True)
    s = scene(out, "Bir duvarın bağları: koridor duvarını ananlar", [CORRIDOR, DEPTH, FIX_S])
    s.query(uids=[6])
    s = scene(out, "Kimlikleriyle: reddin bonds'undaki bağlar", [CORRIDOR, DEPTH, FIX_S])
    s.query(ids=[1, 3])
    s = scene(out, "Kimlik ve nesne birlikte: dış duvarı anan, verilenlerden", [CORRIDOR, DEPTH, FIX_S])
    s.query(uids=[1], ids=[2, 3])
    s = scene(out, "Tutmayan bağ: ölçülen 1,20 m, tutulan 1,50 m", [(CORRIDOR, "1.5")])
    s.query()
    s = scene(out, "İki tutmayan bağ: ikisi de tutmuyor", [(CORRIDOR, "1.5"), (DEPTH, "3.0")])
    s.query(architectural=True)
    s = scene(out, "Başvuru bağı: yalnız ölçer, tuttuğu değer yok", [(CORRIDOR, None, True)])
    s.query()
    s = scene(out, "İki pervaz arası bağ: duvar boyunca ölçülür", [DOORS])
    s.query()
    s = scene(out, "Pervaz ile duvar yüzü arası bağ", [DOOR_WEST])
    s.query()
    s = scene(out, "Eşit aralık tutar", [SPACING])
    s.query()
    s = scene(out, "Sabit ve paralel bağlar ölçü taşımaz", [FIX_S, PARALLEL])
    s.query()
    s = scene(out, "Bağı olmayan çizimde mimari bağ yok", [LEVEL])
    s.query(architectural=True)
    s = scene(out, "Duvar kayınca bağ izler, yine tutar", [CORRIDOR])
    s.shift(1, -0.5)
    s.query(architectural=True)
    s = scene(out, "Bağı kaldır: sorgu artık onu vermez, parametresi de gider", [CORRIDOR, FIX_S])
    s.remove(1)
    s.query(architectural=True)
    s = scene(out, "Bağ kaldırılınca duvar serbestçe kayar ve sorgu kalan bağı verir", [CORRIDOR, DEPTH])
    s.remove(1)
    s.shift(1, -0.5)
    s.query(architectural=True)
    s = scene(out, "Kilitli katmandaki duvara bağ da listelenir", [LOCKED])
    s.query(uids=[12])
    s = scene(out, "Kimliği olmayan kısıt: constraint_not_found", [CORRIDOR])
    s.query_refused({"ids": [cid(99)]}, "constraint_not_found",
                    f"“{cid(99)}” kimlikli kısıt çizimde yok; kaldırılmış olabilir. Kısıtları kimliksiz sorun.", "ids[0]")
    s = scene(out, "Kaldırılmış bağın kimliği sorulursa reddedilir", [CORRIDOR, FIX_S])
    s.remove(2)
    s.query_refused({"ids": [cid(1), cid(2)]}, "constraint_not_found",
                    f"“{cid(2)}” kimlikli kısıt çizimde yok; kaldırılmış olabilir. Kısıtları kimliksiz sorun.", "ids[1]")
    s = scene(out, "Kimlik UUID değil: invalid_uid", [CORRIDOR])
    s.query_refused({"ids": ["d1"]}, "invalid_uid",
                    "“d1” geçerli bir kimlik değil; kimlik küçük harfli, tireli bir UUID'dir.", "ids[0]")
    s = scene(out, "Nesne kimliği UUID değil: invalid_uid", [CORRIDOR])
    s.query_refused({"uids": ["duvar-1"]}, "invalid_uid",
                    "“duvar-1” geçerli bir kimlik değil; kimlik küçük harfli, tireli bir UUID'dir.", "uids[0]")
    s = scene(out, "Parametre değişince bağ izler; sorgu yeni değeri tutulan ve ölçülen verir", [CORRIDOR])
    s.set_parameter("d1", "1.5")
    s.query()
    return [x.done() for x in out]


def document(command, title, note, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": note, "setup": cases[0]["setup"], "cases": cases}


NOTE = ("Beklenenler scripts/fixtures/arch_bond_conflict_cases.py'den: ADR 0770 §4'ün doğrusal kuralları ve "
        "arch_bond_cases.py'nin bağımsız çözümü, Kaydır wall_shift_cases.py ile; yoldaki bağın kaldırılınca yazmayı "
        "geçirip geçirmediği aynı çözümle; KentOS kodu kullanılmadı.")


def documents():
    return [
        ("cad.walls.edit.bonds.json", document(
            "cad.walls.edit", "Yoldaki bağlar: çelişkide bağların parçaları, değerleri ve kaldırılınca geçiren (ADR 0780)",
            NOTE, conflict_cases())),
        ("cad.constraints.query.arch.json", document(
            "cad.constraints.query", "Bağı göster ve kaldır: mimari bağlar, kimlikleri, tuttukları ve ölçülenleri (ADR 0780)",
            NOTE, query_cases())),
    ]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    bad = 0
    for name, doc in documents():
        path = OUT / name
        encoded = json.dumps(doc, ensure_ascii=False, indent=2, allow_nan=False) + "\n"
        if args.check:
            if not path.exists() or path.read_text() != encoded:
                print(f"{path.relative_to(ROOT)} bağımsız başvurudan farklı.")
                bad = 1
            else:
                print(f"{doc['command']} ({name}): {len(doc['cases'])} durum bağımsız başvuruyla aynı.")
        else:
            path.write_text(encoded)
            print(f"{path.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return bad


if __name__ == "__main__":
    sys.exit(main())
