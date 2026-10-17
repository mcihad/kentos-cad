#!/usr/bin/env python3
"""The shared cases of constraints in other commands' edits (docs/adr/0337): kept, lost, copied.

    python3 scripts/fixtures/constraint_follow_cases.py           # writes the file
    python3 scripts/fixtures/constraint_follow_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.constraints.edit.follow.json: each case sets constraints with cad.constraints.edit, then
runs another command on the constrained objects. The rules are written here from the ADR, without KentOS code:

- A write that changes constrained objects (and not the constraints) is followed by a solve in the same step: the parts
  of each changed object the write moved are held (a point whose unknowns changed by more than a metre's billionth; a
  circle or an arc's circle whose centre or radius changed), a fix holds its part where it was before the write, objects on locked layers are held. A
  solve that holds writes the objects it moves; one that does not writes nothing (`constraint_conflict` with the
  conflicting constraints, `constraint_singular`, `not_converged`).
- With `relaxConstraints` no solve: the constraints the written shapes do not meet go (`constraints_relaxed`).
- The cutting commands (trim, extend, break, join, divide, explode, fillet, chamfer, cleanup, reverse, units) never
  solve: the constraints they leave unmet go (`constraints_removed`).
- Before either: the constraints of a deleted object, those naming a vertex, an edge or a node of an object whose
  layout (kind, parts, rings, vertex counts, closedness) the step changed, and those naming a part the object no longer
  has, go (`constraints_removed`); a dimension's parameter goes with it unless another parameter reads it.
- A copy (cad.entities.transform's copy, cad.entities.array) copies the constraints among the copied objects to each
  copy: new ids, a dimension's copy a new name (the next free d…) whose value is what the copy measures; a copy that
  does not meet one (a turned copy's horizontal) and one naming an object not copied are not copied
  (`constraints_not_copied`). The clipboard copies none.

The solve is constraint_cases.py's independent reference (sympy and mpmath at 50 digits); its shapes are expected
only where the solution is unique.
"""
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import constraint_cases as cc  # noqa: E402
import constraint_command_cases as ccc  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
FILE = ROOT / "fixtures/commands/v1/cad.constraints.edit.follow.json"

U, pt, seg, circ, c, add = ccc.U, ccc.pt, ccc.seg, ccc.circ, ccc.c, ccc.add
DIMENSIONS = ccc.DIMENSIONS


def P(x, y):
    return {"x": x, "y": y}


# ---- the rules -------------------------------------------------------------------------------------------------------

def layout(s):
    """What a shape's parts are numbered by: its kind and, for a path, each ring's vertex count and closedness."""
    kind, v = next(iter(s.items()))
    if kind in ("polygon", "polyline"):
        return (kind, len(v["pts"]), v.get("closed", kind == "polygon"))
    return (kind,)


def held_parts(before, after):
    """The parts of an object a write moved (docs/adr/0337 §1): each point whose unknowns changed, a circle whose centre or
    radius did; none when its layout changed."""
    kind, _ = next(iter(before.items()))
    a, b = cc.unknowns(before), cc.unknowns(after)
    # A number moved by a metre's billionth or less has not been dragged (an edit writes the others back rounded).
    moved = {j for j, (x, y) in enumerate(zip(a, b)) if abs(x - y) > 1e-9}
    if kind == "line":
        parts = [({"point": "start"}, {0, 1}), ({"point": "end"}, {2, 3})]
    elif kind == "circle":
        parts = [("circle", {0, 1, 2})]
    elif kind == "arc":
        parts = [("circle", {0, 1, 2}), ({"point": "start"}, {0, 1, 2, 3}), ({"point": "end"}, {0, 1, 2, 4})]
    elif kind == "point":
        parts = [({"point": "node"}, {0, 1})]
    else:
        n = len(a) // 2
        parts = [({"vertex": [0, 0, i]}, {2 * i, 2 * i + 1}) for i in range(n)]
    return [el for el, vars_ in parts if vars_ & moved]


def indexed(ref):
    el = ref["element"]
    return el["kind"] == "segment" or (el["kind"] == "point" and el["place"]["kind"] in ("vertex", "node"))


def read_names(params):
    return {n for p in params for n in ccc.names_in(p.get("expression", ""))}


def take_away(d, gone, measured_from=None):
    """Takes constraints (indices) away with their dimensions' parameters unless another parameter reads them: then the
    parameter keeps a value (a reference's measured)."""
    gone = set(gone)
    dropped = [k for j, k in enumerate(d.constraints) if j in gone]
    d.constraints = [k for j, k in enumerate(d.constraints) if j not in gone]
    read = read_names(d.parameters)
    for k in dropped:
        name = k["rule"].get("parameter")
        if not name:
            continue
        if name not in read:
            d.parameters = [p for p in d.parameters if p["name"] != name]
        else:
            p = next(p for p in d.parameters if p["name"] == name)
            if not p.get("expression"):
                src = measured_from or d
                p["expression"] = ccc.value_text(ccc.from_base(p["kind"], src.measure(k["rule"])))
    return len(dropped)


def values_base(d):
    values, problem = ccc.values_of(d, d.constraints, d.parameters)
    assert problem is None, problem
    return {n: ccc.to_base(next(p["kind"] for p in d.parameters if p["name"] == n), v) for n, v in values.items()}


def points_of(sh):
    kind, v = next(iter(sh.items()))
    if kind == "line":
        return list(v)
    if kind in ("circle", "arc"):
        return [v[0]]
    if kind == "point":
        return [v]
    return v["pts"]


def unmet(d, only=None, fix_from=None):
    """The constraints the drawing as it is does not meet: each one's equations on the shapes, held to the solver's
    tolerance (1e-11 of the extent of the points, at least a metre); a fix at its place in `fix_from` (the objects
    before a write), else where it is. The cases keep clear of the tolerance: what they keep meets it to rounding, what
    they break misses it by millimetres or more."""
    pick = list(range(len(d.constraints))) if only is None else only
    if not pick:
        return []
    chosen = [d.constraints[j] for j in pick]
    case, order = d.cc_case(chosen, values_base(d))
    shapes = case["shapes"]
    fixes = [ccc.shape(fix_from.get(s, d.entities[s])) for s in order] if fix_from else shapes
    pts = [q for sh in shapes for q in points_of(sh)]
    extent = math.hypot(max(q[0] for q in pts) - min(q[0] for q in pts), max(q[1] for q in pts) - min(q[1] for q in pts))
    tol = 1e-11 * max(extent, 1.0)
    xs = [[cc.sp.Float(repr(float(v)), 50) for v in cc.unknowns(sh)] for sh in shapes]
    fix_xs = [[cc.sp.Float(repr(float(v)), 50) for v in cc.unknowns(sh)] for sh in fixes]
    out = []
    for i, rule in enumerate(case["constraints"]):
        if rule.get("reference"):
            continue
        parts = [cc.element(shapes, xs, r) for r in cc.refs_of(rule)]
        if any(q is None for q in parts):
            out.append(pick[i])
            continue
        fixed = [cc.element(fixes, fix_xs, r) for r in cc.refs_of(rule)] if rule["rule"] == "fix" else None
        rows = cc.rows_of(rule, parts, cc.sp.Float(repr(float(rule.get("value", 0.0))), 50), fixed)
        if any(abs(float(cc.sp.N(e, 30))) > tol for e in rows):
            out.append(pick[i])
    return sorted(out)


def structural(d, before, updated, removed):
    """The constraints a step takes away before anything else (§2): of the deleted objects, of the indexed parts of an
    object whose layout changed, of a part an object no longer has."""
    gone = []
    for j, k in enumerate(d.constraints):
        for r in ccc.refs(k["rule"]):
            s = d.slot(r["uid"])
            if s in removed:
                gone.append(j)
                break
            if s in updated and (
                (indexed(r) and layout(ccc.shape(before[s])) != layout(ccc.shape(d.entities[s]))) or not d.has(r)
            ):
                gone.append(j)
                break
    return gone


def follow(d, before, updated, removed, mode, loose=False):
    """The ADR's follow after a write already applied to `d` (its objects; `before` as they were). Answers
    (warnings, moved slots) or a refusal."""
    warnings = []
    removed_count = take_away(d, structural(d, before, updated, removed), measured_from=None)
    named = {d.slot(r["uid"]) for k in d.constraints for r in ccc.refs(k["rule"])}
    edited = [s for s in updated if s in named]
    if mode in ("cut", "relax") and edited:
        bad = unmet(d, fix_from=before)
        n = take_away(d, bad)
        if mode == "cut":
            removed_count += n
        elif n:
            warnings.append({"code": "constraints_relaxed", "message": "$any"})
    if removed_count:
        warnings.insert(0, {"code": "constraints_removed", "message": "$any"})
    if mode in ("cut", "relax") or not edited:
        return warnings, []
    case, order = d.cc_case(d.constraints, values_base(d))
    held = case.get("held", [])
    for i, s in enumerate(order):
        if s in edited:
            for el in held_parts(ccc.shape(before[s]), ccc.shape(d.entities[s])):
                held.append([i, el])
    if held:
        case["held"] = held
    case["fixFrom"] = [ccc.shape(before.get(s, d.entities[s])) for s in order]
    out = cc.reference(case, all_shapes=True)
    ids = lambda ks: [f"$constraint:{k}" for k in ks]
    if out["status"] == "inconsistent":
        # Which of the over-held rows are the conflicting ones is the solver's order on rounded numbers; a case written
        # on numbers the edit made inexact (a turn) compares only that there are some (`loose`).
        uids = "$any" if loose else ids(out["conflicting"])
        return {"status": "failed", "error": {"code": "constraint_conflict", "message": "$any", "uids": uids}}
    if out["status"] == "singular":
        return {"status": "failed", "error": {"code": "constraint_singular", "message": "$any", "uids": ids(out["singular"])}}
    if out["status"] == "nonConvergent":
        return {"status": "failed", "error": {"code": "not_converged", "message": "$any"}}
    moved = []
    for i, s in enumerate(order):
        if ccc.moved(ccc.shape(d.entities[s]), out["solved"][i]):
            d.entities[s] = ccc.entity(d.entities[s], out["solved"][i])
            moved.append(s)
    return warnings, moved


def copy_constraints(d, groups):
    """Copies the constraints among each group's sources to its copies (§3); answers how many were not copied."""
    original = list(d.constraints)
    candidates = []
    crossing = 0
    for g in groups:
        for k in original:
            slots = [d.slot(r["uid"]) for r in ccc.refs(k["rule"])]
            inside = [s in g for s in slots]
            if not any(inside):
                continue
            if not all(inside):
                crossing += 1
                continue
            rule = json.loads(json.dumps(k["rule"]))
            for key in ("a", "b", "axis"):
                if isinstance(rule.get(key), dict) and "uid" in rule[key]:
                    rule[key]["uid"] = U(g[d.slot(rule[key]["uid"])])
            candidates.append({"rule": rule, "reference": k.get("reference", False)})
    # Each dimension's copy a name and the value its copy measures.
    for k in candidates:
        rule = k["rule"]
        if rule["kind"] in DIMENSIONS:
            name = ccc.next_name(d.parameters, rule["kind"])
            rule["parameter"] = name
            kind = ccc.kind_of(rule)
            p = {"name": name, "kind": kind}
            if not k["reference"]:
                p["expression"] = ccc.value_text(ccc.from_base(kind, d.measure(rule)))
            d.parameters.append(p)
    start = len(d.constraints)
    for k in candidates:
        new = {"id": d.next_id, "rule": k["rule"]}
        d.next_id += 1
        if k["reference"]:
            new["reference"] = True
        d.constraints.append(new)
    bad = unmet(d, list(range(start, len(d.constraints))))
    take_away(d, bad)
    return crossing + len(bad)


# ---- the commands' geometry (only what the cases use) ----------------------------------------------------------------

def move_entity(e, dx, dy):
    return transform_entity(e, lambda x, y: (x + dx, y + dy))


def rotate_entity(e, cx, cy, ang):
    co, si = math.cos(ang), math.sin(ang)
    return transform_entity(e, lambda x, y: (cx + co * (x - cx) - si * (y - cy), cy + si * (x - cx) + co * (y - cy)))


def scale_entity(e, cx, cy, f):
    out = transform_entity(e, lambda x, y: (cx + f * (x - cx), cy + f * (y - cy)))
    if "r" in out:
        out["r"] = e["r"] * f
    return out


def transform_entity(e, f):
    e = json.loads(json.dumps(e))
    for key in ("a", "b", "c", "p"):
        if key in e:
            x, y = f(e[key]["x"], e[key]["y"])
            e[key] = P(x, y)
    if "pts" in e:
        e["pts"] = [P(*f(q["x"], q["y"])) for q in e["pts"]]
    return e


# ---- the cases -------------------------------------------------------------------------------------------------------

KEEP, CUT, RELAX = "keep", "cut", "relax"


class Case(ccc.Case):
    def other(self, command, input_, writes, output, mode=KEEP, relax=False, within=1e-9, created=(), loose=False, capture=()):
        """Runs `command` whose write `writes(d)` applies to the drawing; answers its result as the ADR says."""
        d = self.d
        before = {k: dict(v) for k, v in d.entities.items()}
        state = (json.loads(json.dumps(d.constraints)), json.loads(json.dumps(d.parameters)), d.next_id)
        self.prev = (before, state)
        updated, removed, groups = writes(d)
        got = follow(d, before, updated, removed, RELAX if relax and mode == KEEP else mode, loose)
        # A deleted object's id, taken before the step (`$uid:kN`).
        for slot in capture:
            self.steps.append({"op": "captureUid", "id": slot, "as": f"k{slot}"})
        step = {"op": "execute", "command": command, "input": input_}
        if relax:
            step["relaxConstraints"] = True
        if isinstance(got, dict):
            d.entities = before
            d.constraints, d.parameters, d.next_id = state
            step["result"] = got
            step["expect"] = {"revision": "same"}
        else:
            warnings, moved = got
            if groups:
                if copy_constraints(d, groups):
                    warnings.append({"code": "constraints_not_copied", "message": "$any"})
            step["result"] = {"status": "completed", "output": output, "warnings": warnings}
            shown = sorted(set(updated) | set(moved) | set(created))
            expect = {"revision": "changed", "canUndo": True, "constraintKinds": [k["rule"]["kind"] for k in d.constraints],
                      "parameters": [dict(p) for p in d.parameters]}
            if shown:
                expect["entities"] = {str(s): d.entities[s] for s in shown}
                step["within"] = within
            step["expect"] = expect
        self.steps.append(step)
        return self

    def undo(self, label="Kısıtlar"):
        """Takes the last step back: the drawing as it was before it."""
        entities, (constraints, parameters, next_id) = self.prev
        self.d.entities = {k: dict(v) for k, v in entities.items()}
        self.d.constraints, self.d.parameters, self.d.next_id = constraints, parameters, next_id
        self.steps.append({"op": "undo", "returns": label, "expect": {
            "canRedo": True, "constraintKinds": [k["rule"]["kind"] for k in constraints], "parameters": parameters}})
        return self


RECT = [1, 2, 3, 4]


def moving(slots, dx, dy):
    def w(d):
        for s in slots:
            d.entities[s] = move_entity(d.entities[s], dx, dy)
        return list(slots), [], []
    return w


def rotating(slots, cx, cy, ang):
    def w(d):
        for s in slots:
            d.entities[s] = rotate_entity(d.entities[s], cx, cy, ang)
        return list(slots), [], []
    return w


def copying(slots, f, first):
    def w(d):
        group = {}
        for k, s in enumerate(slots):
            new = f(d.entities[s])
            new["id"] = first + k
            d.entities[first + k] = new
            group[s] = first + k
        return [], [], [group]
    return w


def transformed(slots, transform, copy=False, created=()):
    out = {"uids": [U(s) for s in slots], "transform": transform}
    if copy:
        out["copy"] = True
    return out, {"changed": [] if copy else [U(s) for s in slots], "created": [U(s) for s in created], "locked": [],
                 "revision": "$current"}


def rect(k, fix=True, width="8", height="4"):
    """The rectangle of lines 1–4: corners joined, the bottom horizontal, opposite edges parallel, a square corner, a
    fixed corner, width and height; written and solved, its fix taken away after when `fix` is false."""
    k.execute(add(*ccc.rect_rules(width, height)), within=1e-9)
    if not fix:
        k.execute({"kind": "remove", "ids": ["$constraint:8"]})
    return k


def cases():
    out = []

    def case(name):
        k = Case(name)
        out.append(k)
        return k

    # §1: kept.
    k = rect(case("Sabitsiz dikdörtgenin üst kenarı yukarı taşınır: yükseklik korunur, bütün dikdörtgen gelir"), fix=False)
    i, o = transformed([3], {"kind": "move", "dx": 0, "dy": 1})
    k.other("cad.entities.transform", i, moving([3], 0, 1), o)
    k.undo("Taşı")

    k = rect(case("Sabit köşeli dikdörtgenin üst kenarı taşınamaz: çelişki, hiçbir şey yazılmaz"))
    i, o = transformed([3], {"kind": "move", "dx": 0, "dy": 1})
    k.other("cad.entities.transform", i, moving([3], 0, 1), o)

    k = rect(case("Gevşeterek: üst kenar taşınır, tutmayan iki köşe kısıtı kalkar"))
    i, o = transformed([3], {"kind": "move", "dx": 0, "dy": 1})
    k.other("cad.entities.transform", i, moving([3], 0, 1), o, relax=True)

    k = rect(case("Sabitsiz dikdörtgen bütünüyle taşınır: kısıtlar tutar, başka bir şey kımıldamaz"), fix=False)
    i, o = transformed(RECT, {"kind": "move", "dx": 2, "dy": 3})
    k.other("cad.entities.transform", i, moving(RECT, 2, 3), o)

    k = rect(case("Sabit köşeli dikdörtgen bütünüyle taşınamaz"))
    i, o = transformed(RECT, {"kind": "move", "dx": 2, "dy": 3})
    k.other("cad.entities.transform", i, moving(RECT, 2, 3), o)

    k = rect(case("Gevşeterek bütünüyle taşınır: yalnız sabit kalkar"))
    i, o = transformed(RECT, {"kind": "move", "dx": 2, "dy": 3})
    k.other("cad.entities.transform", i, moving(RECT, 2, 3), o, relax=True)

    ang = math.pi / 6
    k = rect(case("Döndürülen dikdörtgen yatay kalamaz: çelişki"), fix=False)
    i, o = transformed(RECT, {"kind": "rotate", "center": P(0, 0), "angle": ang})
    k.other("cad.entities.transform", i, rotating(RECT, 0, 0, ang), o, loose=True)

    k = rect(case("Gevşeterek döndürülür: yatay kalkar, paralellik ve diklik kalır"), fix=False)
    i, o = transformed(RECT, {"kind": "rotate", "center": P(0, 0), "angle": ang})
    k.other("cad.entities.transform", i, rotating(RECT, 0, 0, ang), o, relax=True)

    k = rect(case("Tutamaçla sağ üst köşe yukarı: yükseklik 4 korunur, dikdörtgen kayar"), fix=False)
    k.other("cad.entities.edit", {"operation": "grip", "changes": [
        {"kind": "update", "uid": U(2), "geometry": {"kind": "line", "a": P(8, 0), "b": P(8, 5)}}]},
        lambda d: (d.entities.__setitem__(2, dict(d.entities[2], b=P(8, 5))) or ([2], [], [])),
        {"changed": [U(2)], "created": [], "removed": [], "revision": "$current"})

    k = rect(case("Tutamaçla genişlik: genişliksiz sabit dikdörtgenin sağ üst köşesi 10'a, karşı kenarlar paralel, köşeler dik"),
             width=None)
    k.other("cad.entities.edit", {"operation": "grip", "changes": [
        {"kind": "update", "uid": U(2), "geometry": {"kind": "line", "a": k.d.entities[2]["a"], "b": P(10, 4)}}]},
        lambda d: (d.entities.__setitem__(2, dict(d.entities[2], b=P(10, 4))) or ([2], [], [])),
        {"changed": [U(2)], "created": [], "removed": [], "revision": "$current"})

    k = rect(case("Kısıtsız daire taşınır: kısıtlara dokunulmaz"))
    i, o = transformed([5], {"kind": "move", "dx": 1, "dy": 1})
    k.other("cad.entities.transform", i, moving([5], 1, 1), o)

    # §2: lost.
    k = rect(case("Üst kenar silinir: onu anan üç kısıt da gider"))
    k.other("cad.entities.delete", {"uids": ["$uid:k3"]},
            lambda d: (d.entities.pop(3) and ([], [3], [])),
            {"removed": ["$uid:k3"], "locked": [], "revision": "$current"}, capture=[3])
    k.undo("Sil")

    k = rect(case("Genişliği okunan kenar silinir: ölçü gider, parametresi kullanıcı parametresi olarak kalır"), height="d1 / 2")
    k.other("cad.entities.delete", {"uids": ["$uid:k1"]},
            lambda d: (d.entities.pop(1) and ([], [1], [])),
            {"removed": ["$uid:k1"], "locked": [], "revision": "$current"}, capture=[1])

    k = case("Köşe silinince sıralı kısıtları gider")
    k.execute(add(c({"kind": "horizontal", "a": seg(9, 0)}), c({"kind": "vertical", "a": seg(9, 3)}),
                  c({"kind": "fix", "a": {"uid": U(9), "element": {"kind": "point", "place": {"kind": "vertex", "part": 0, "ring": 0, "index": 0}}}}),
                  c({"kind": "radius", "a": circ(5), "parameter": ""}, value="1.5")))

    def drop_vertex(d):
        e = dict(d.entities[9])
        e["pts"] = e["pts"][:3]
        e["bulges"] = e["bulges"][:3]
        d.entities[9] = e
        return [9], [], []
    k.other("cad.entities.vertex", {"uid": U(9), "action": "remove", "place": {"kind": "vertex", "part": 0, "ring": 0, "index": 3}},
            drop_vertex, {"changed": [U(9)], "ids": [9], "revision": "$current"})

    k = rect(case("Alt kenar kırılır: köşesi ve genişliği gider, yatay ve diklik kalır"))

    def broken(d):
        d.entities[1] = dict(d.entities[1], b=P(3, 0))
        d.entities[12] = dict(d.entities[1], id=12, a=P(5, 0), b=P(8, 0))
        return [1], [], []
    k.other("cad.entities.break", {"uid": U(1), "first": P(3, 0), "second": P(5, 0)}, broken,
            {"changed": [U(1)], "created": [U(12)], "removed": [], "revision": "$current"}, mode=CUT, created=[12])

    # §3: copied.
    k = rect(case("Kopya: aradaki on kısıt yeni adlı ölçülerle kopyalanır"), fix=False)
    i, o = transformed(RECT, {"kind": "move", "dx": 20, "dy": 0}, copy=True, created=[12, 13, 14, 15])
    k.other("cad.entities.transform", i, copying(RECT, lambda e: move_entity(e, 20, 0), 12), o, created=[12, 13, 14, 15])
    k.undo("Kopyala")

    k = rect(case("Sabit köşeli kopya: sabit kopyayı yerinde tutar"))
    i, o = transformed(RECT, {"kind": "move", "dx": 0, "dy": 10}, copy=True, created=[12, 13, 14, 15])
    k.other("cad.entities.transform", i, copying(RECT, lambda e: move_entity(e, 0, 10), 12), o, created=[12, 13, 14, 15])

    k = rect(case("Döndürülmüş kopya: yatay kopyalanmaz, öbürleri kopyalanır"), fix=False)
    i, o = transformed(RECT, {"kind": "rotate", "center": P(20, 0), "angle": ang}, copy=True, created=[12, 13, 14, 15])
    k.other("cad.entities.transform", i, copying(RECT, lambda e: rotate_entity(e, 20, 0, ang), 12), o, created=[12, 13, 14, 15])

    k = rect(case("Ölçekli kopya: ölçülerin kopyası kopyanın ölçtüğü"), fix=False)
    i, o = transformed(RECT, {"kind": "scale", "center": P(0, 10), "factor": 2}, copy=True, created=[12, 13, 14, 15])
    k.other("cad.entities.transform", i, copying(RECT, lambda e: scale_entity(e, 0, 10, 2), 12), o, created=[12, 13, 14, 15])

    k = rect(case("Yarım kopya: yalnız alt ve sağ kenarın aralarındakiler kopyalanır"), fix=False)
    i, o = transformed([1, 2], {"kind": "move", "dx": 0, "dy": -10}, copy=True, created=[12, 13])
    k.other("cad.entities.transform", i, copying([1, 2], lambda e: move_entity(e, 0, -10), 12), o, created=[12, 13])

    k = rect(case("Dizi: her kopya kısıtlarını alır"), fix=False)

    def arrayed(d):
        groups = []
        for col in (1, 2):
            group = {}
            for j, s in enumerate(RECT):
                slot = 12 + (col - 1) * 4 + j
                d.entities[slot] = dict(move_entity(d.entities[s], 20 * col, 0), id=slot)
                group[s] = slot
            groups.append(group)
        return [], [], groups
    k.other("cad.entities.array", {"uids": [U(s) for s in RECT], "layout": {"kind": "grid", "rows": 1, "cols": 3, "dx": 20, "dy": 0}},
            arrayed, {"created": [U(s) for s in range(12, 20)], "locked": [], "revision": "$current"}, created=list(range(12, 20)))

    k = rect(case("Pano kısıt taşımaz: söylenir"))
    k.steps.append({"op": "execute", "command": "cad.entities.copy", "input": {"uids": [U(1), U(2)]},
                    "result": {"status": "completed", "output": "$any",
                               "warnings": [{"code": "constraints_not_copied", "message": "$any"}]},
                    "expect": {"revision": "same"}})
    return out


def build():
    head = ccc.header("cad.constraints.edit", "Düzenlemelerde kısıtlar",
                      "docs/adr/0337: korunur, kaybı ve kopyası söylenir; scripts/fixtures/constraint_follow_cases.py")
    return dict(head, cases=[k.json() for k in cases()])


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if FILE.read_text() != text:
            print(f"{FILE.relative_to(ROOT)} is not what the rules give; run without --check", file=sys.stderr)
            sys.exit(1)
        print("constraint follow cases: same")
        return
    FILE.write_text(text)
    print(f"wrote {FILE.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
