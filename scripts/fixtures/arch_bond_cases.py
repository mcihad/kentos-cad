#!/usr/bin/env python3
"""Shared cases of the architectural bonds (docs/adr/0770 §4, §5), from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.constraints.edit.arch.json; `--check` compares and writes nothing. A case adds bonds
with cad.constraints.edit (a wall's line: `wall` edge and line; an opening's jamb: `jamb` end), then edits walls, a
parameter or an opening; what the bonds make of it is written here:

- every bonded edge shifts square to itself (`Δ` along its left normal), every bonded opening along its edge (`δ`);
  the rules are linear (the ADR's table); a distance's side `left` is read from the drawing when it is made (two jambs
  in their order along the wall);
- an edit holds the edges whose line it moved or turned and the openings it moved along their wall (their start jamb
  then, from their place, and now, from their wall and `at`); a new bond or a parameter's new value holds the `a` of
  each distance that does not hold now; equal spacing (symmetric) always holds its two ends, its axis follows; a fix whose part an edit moved, lines a
  bond wants parallel that are not, rows left contradicting: refused (`constraint_conflict`), nothing written;
- the rest is the least change: rows in the bonds' order, shifts in their order (edges first as the bonds name them,
  then openings), the first row with a coefficient the pivot, x = Rᵀ (R Rᵀ)⁻¹ d;
- each edge shifted by Kaydır (wall_shift_cases.py's independent rule, joined walls following), in the shifts' order;
  the openings of moved walls read back from their jambs; then each bonded opening's `at` plus its shift.

    python3 scripts/fixtures/arch_bond_cases.py
    python3 scripts/fixtures/arch_bond_cases.py --check
"""

import argparse
import copy
import json
import math
import sys
from fractions import Fraction
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import wall_shift_cases as ws  # noqa: E402  (Kaydır and the openings carried, independently)
import wall_command_cases as wc  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.constraints.edit.arch.json"
TIE = 1e-9
SETTLED = 1e-6
PARALLEL = 1.745_329_251_994_33e-5
CROSSING = 0.5
STEP_ADD = "Kısıt ekle"


def P(x, y):
    return ws.P(x, y)


def door(id_, host, at, width, walls):
    w = walls[host]
    return {"kind": "opening", "id": id_, "layerId": "kapi", "attrs": {}, "host": ws.uid(host), "at": at,
            "width": width, "form": "door", "place": [P(*p) for p in ws.jambs(w, at, width)]}


WALLS = {
    1: ws.wall(1, [(0, 0), (8, 0)]),            # S: south outer wall
    2: ws.wall(2, [(8, 0), (8, 5)]),            # E
    3: ws.wall(3, [(8, 5), (0, 5)]),            # N
    4: ws.wall(4, [(0, 5), (0, 0)]),            # W
    5: ws.wall(5, [(4, 0), (4, 5)], width=0.1),    # P: the partition between rooms A and B
    6: ws.wall(6, [(0, 1.35), (4, 1.35)], width=0.1),  # C: the corridor's wall in room A (corridor 1.20 m)
    10: ws.wall(10, [(0, 20), (4, 20)], bulges=[0.3, 0.0]),   # an arc wall
    11: ws.wall(11, [(0, 22), (4, 22)], layer="kilitli"),     # a wall on a locked layer
    12: ws.wall(12, [(0, 23.2), (4, 23.2)]),
}
ENTITIES = [WALLS[1], WALLS[2], WALLS[3], WALLS[4], WALLS[5], WALLS[6],
            door(7, 1, 2.0, 0.9, WALLS), door(8, 1, 6.0, 0.9, WALLS), door(9, 5, 3.0, 0.8, WALLS),
            WALLS[10], WALLS[11], WALLS[12]]
SETUP = copy.deepcopy(ws.SETUP)
SETUP["name"] = "Mimari bağlar"
SETUP["entities"] = ENTITIES
SETUP["uids"] = [ws.uid(e["id"]) for e in ENTITIES]
IDS = [e["id"] for e in ENTITIES]


def u(i):
    return ws.uid(i)


def W(i, line, edge=0):
    return {"uid": u(i), "element": {"kind": "wall", "edge": edge, "line": line}}


def J(i, end):
    return {"uid": u(i), "element": {"kind": "jamb", "end": end}}


# ── Geometry of the parts ────────────────────────────────────────────────────────────────────────────────────────────

xy, sub, dot, cross, left, unit, along = ws.xy, ws.sub, ws.dot, ws.cross, ws.left, ws.unit, ws.along


def line_offset(w, line):
    lf = ws.left_face(w)
    rf = lf - w["width"]
    # No components here: the core is the faces.
    return {"leftFace": lf, "rightFace": rf, "axis": (lf + rf) / 2, "leftCore": lf, "rightCore": rf}[line]


def wall_line(state, el):
    """(point on the line, edge direction) of a wall's line; none for an arc edge or an edge it has not."""
    w = state[el_id(el)]
    e = el["element"]
    ed = ws.edge(w, e["edge"])
    if ed is None:
        return None
    a, b = ed
    d = unit(a, b)
    return along(a, left(d), line_offset(w, e["line"])), d


def el_id(el):
    return int(el["uid"].rsplit("-", 1)[1], 16)


def host_of(state, o):
    return next(i for i in state if ws.uid(i) == o["host"])


def jamb(state, el):
    """(point, edge direction, host id) of an opening's jamb: the middle of its jamb line, from its wall and `at`."""
    o = state[el_id(el)]
    h = host_of(state, o)
    c = ws.jambs(state[h], o["at"], o["width"])
    a, b = (c[0], c[1]) if el["element"]["end"] == "start" else (c[2], c[3])
    p = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
    ed = ws.edge(state[h], 0)
    return p, unit(*ed), h


# ── The problem ──────────────────────────────────────────────────────────────────────────────────────────────────────

def items(state, rules):
    """The shifts (edges first, in the order the rules name them, then openings) and each rule's parts."""
    var = []

    def v(key):
        if key not in var:
            var.append(key)
        return var.index(key)
    for r in rules:
        for k in ("a", "b", "axis"):
            el = r.get(k)
            if type(el) is dict and el["element"]["kind"] == "wall":
                v(("edge", el_id(el), el["element"]["edge"]))
    parts = []
    for r in rules:
        got = []
        for k in ("a", "b", "axis"):
            el = r.get(k)
            if type(el) is not dict:
                continue
            if el["element"]["kind"] == "wall":
                at, d = wall_line(state, el)
                got.append(("line", v(("edge", el_id(el), el["element"]["edge"])), at, d))
            else:
                p, d, h = jamb(state, el)
                host = ("edge", h, 0)
                got.append(("jamb", v(("opening", el_id(el))), p, d, var.index(host) if host in var else None))
        parts.append(got)
    return var, parts


def rows(nvars, rules, parts, values):
    """The equations (coefficients, right side, the rules made from) or (rule index, why)."""
    out = []
    for k, (r, ps) in enumerate(zip(rules, parts)):
        co = [0.0] * nvars
        kind = r["kind"]
        if kind == "parallel":
            if abs(cross(ps[0][3], ps[1][3])) > PARALLEL:
                return k, "parallel"
            continue
        if kind == "fix":
            co[ps[0][1]] += 1.0
            out.append((co, 0.0, {k}))
            continue
        if kind == "symmetric":
            a, b, ax = ps
            if abs(cross(a[3], ax[3])) > PARALLEL or abs(cross(b[3], ax[3])) > PARALLEL:
                return k, "parallel"
            n = left(ax[3])
            co[a[1]] += dot(n, left(a[3]))
            co[b[1]] += dot(n, left(b[3]))
            co[ax[1]] -= 2.0
            out.append((co, -(dot(n, sub(a[2], ax[2])) + dot(n, sub(b[2], ax[2]))), {k}))
            continue
        a, b = ps
        value = values[r["parameter"]]
        sigma = 1.0 if r.get("left") else -1.0
        if a[0] == "line":
            if abs(cross(a[3], b[3])) > PARALLEL:
                return k, "parallel"
            n = left(a[3])
            co[b[1]] += dot(n, left(b[3]))
            co[a[1]] -= 1.0
            out.append((co, sigma * value - dot(n, sub(b[2], a[2])), {k}))
        elif b[0] == "line":
            n = left(b[3])
            if abs(dot(n, a[3])) < CROSSING:
                return k, "crossing"
            co[a[1]] += dot(n, a[3])
            if a[4] is not None:
                co[a[4]] += dot(n, left(a[3]))
            co[b[1]] -= 1.0
            out.append((co, sigma * value - dot(n, sub(a[2], b[2])), {k}))
        else:
            co[b[1]] += dot(a[3], b[3])
            co[a[1]] -= 1.0
            out.append((co, value - dot(a[3], sub(b[2], a[2])), {k}))
    return out


def solve(held, eqs):
    """The least change, or the rules of a contradicting row."""
    n = len(held)
    rs = [([0.0 if held[j] else c for j, c in enumerate(co)], rhs, set(fr)) for co, rhs, fr in eqs]
    pivots = []
    for col in range(n):
        if held[col]:
            continue
        p = next((i for i, r in enumerate(rs) if abs(r[0][col]) > 1e-9), None)
        if p is None:
            continue
        pv = rs.pop(p)
        nxt = []
        for co, rhs, fr in rs:
            f = co[col] / pv[0][col]
            if f != 0.0:
                co = [x - f * y for x, y in zip(co, pv[0])]
                co[col] = 0.0
                rhs = rhs - f * pv[1]
                fr = fr | pv[2]
            nxt.append((co, rhs, fr))
        rs = nxt
        pivots.append(pv)
    bad = next((r for r in rs if abs(r[1]) > TIE), None)
    if bad is not None:
        return None, sorted(bad[2])
    r = len(pivots)
    m = [[sum(a * b for a, b in zip(pivots[i][0], pivots[j][0])) for j in range(r)] for i in range(r)]
    y = [p[1] for p in pivots]
    # (R Rᵀ) y = d, exactly in fractions: the shifts are rounded once.
    M = [[Fraction(x) for x in row] for row in m]
    Y = [Fraction(v) for v in y]
    for c in range(r):
        p = next(i for i in range(c, r) if M[i][c] != 0)
        M[c], M[p] = M[p], M[c]
        Y[c], Y[p] = Y[p], Y[c]
        for i in range(r):
            if i != c and M[i][c] != 0:
                f = M[i][c] / M[c][c]
                M[i] = [a - f * b for a, b in zip(M[i], M[c])]
                Y[i] -= f * Y[c]
    sol = [Y[c] / M[c][c] for c in range(r)]
    x = [float(sum(Fraction(pivots[i][0][j]) * sol[i] for i in range(r))) for j in range(n)]
    return [0.0 if held[j] or abs(x[j]) <= TIE else x[j] for j in range(n)], None


def measured(state, rule):
    """A distance's clear distance now (along the wall for two jambs) and its side."""
    var, parts = items(state, [rule])
    a, b = parts[0]
    if a[0] == "line":
        g = dot(left(a[3]), sub(b[2], a[2]))
    elif b[0] == "line":
        g = dot(left(b[3]), sub(a[2], b[2]))
    else:
        g = dot(a[3], sub(b[2], a[2]))
        return g, False
    return abs(g), g > 0


def miss(state, rule, values):
    var, parts = items(state, [rule])
    eq = rows(len(var), [rule], parts, values)
    if isinstance(eq, tuple):
        return None
    return eq[0][1] if eq else 0.0


def equations(state, rules, values, edited=None):
    """The bonds' equations solved, nothing written: (shifts, their keys, the rules' parts) or (None, (rule indices,
    words)) when the equations conflict (docs/adr/0780 §1: a bond that frees such a conflict is found here)."""
    var, parts = items(state, rules)
    held = [False] * len(var)
    for k, key in enumerate(var):
        oid = key[1]
        if state[oid]["layerId"] == "kilitli":
            held[k] = True
        if edited and oid in edited:
            before = edited[oid]
            if key[0] == "edge":
                b0 = ws.edge(before, key[2])
                b1 = ws.edge(state[oid], key[2])
                if b0 is None or b1 is None:
                    held[k] = True
                else:
                    u0, u1 = unit(*b0), unit(*b1)
                    across = dot(left(u0), sub(b1[0], b0[0]))
                    held[k] = held[k] or abs(cross(u0, u1)) > PARALLEL or abs(across) > TIE
            else:
                # Moved along its wall in the world: its start jamb then (from its place) and now (wall and at).
                pl = [xy(q) for q in before["place"]]
                then = ((pl[0][0] + pl[1][0]) / 2, (pl[0][1] + pl[1][1]) / 2)
                now_p, h, _ = jamb(state, {"uid": u(oid), "element": {"kind": "jamb", "end": "start"}})
                held[k] = held[k] or abs(dot(h, sub(now_p, then))) > TIE
    # Equal spacing keeps its two ends; its axis follows.
    for i, r in enumerate(rules):
        if r["kind"] == "symmetric":
            held[parts[i][0][1]] = True
            held[parts[i][1][1]] = True
    fixed_moved = [i for i, r in enumerate(rules) if r["kind"] == "fix" and edited
                   and held[parts[i][0][1]] and el_id(r["a"]) in edited]
    if fixed_moved:
        return None, (fixed_moved, "Sabit bağlı bir duvar ya da açıklık oynatılamaz")
    if edited is None:
        for i, r in enumerate(rules):
            if r["kind"] == "distance" and abs(miss(state, r, values)) > SETTLED:
                held[parts[i][0][1]] = True
    eqs = rows(len(var), rules, parts, values)
    if isinstance(eqs, tuple):
        return None, ([eqs[0]], {"parallel": "Bağlı duvar çizgileri artık paralel değil (bir duvar döndü)",
                                  "crossing": "Pervazın bağlandığı çizgi açıklığın duvarını en az 30° ile kesmiyor"}[eqs[1]])
    x, bad = solve(held, eqs)
    if bad is not None:
        return None, (bad, "Bağlar birlikte tutmuyor")
    return (x, var, parts), None


def follow(state, rules, values, held_ids=None, edited=None, late=None):
    """What the bonds make of the drawing now (`state` after a write): (changed objects by id) or (rule indices,
    words). `edited`: the objects' state before the write (an edit's follow); none: hold each distance's `a` that does
    not hold now. `late` (a list): told whether a conflict came while the shifts were written."""
    solved, why = equations(state, rules, values, edited)
    if why is not None:
        return None, why
    x, var, parts = solved
    if late is not None:
        late.append(True)
    now = dict(state)
    changed = {}
    moved_walls = {}
    for k, key in enumerate(var):
        if key[0] != "edge" or x[k] == 0.0:
            continue
        try:
            moved = ws.shift(now, key[1], key[2], x[k])
        except ws.Refused as r:
            return None, ([i for i, ps in enumerate(parts) if any(p[1] == k for p in ps)], r.message)
        for j, pts in moved.items():
            if now[j]["layerId"] == "kilitli":
                return None, ([i for i, ps in enumerate(parts) if any(p[1] == k for p in ps)],
                              "Bağ “Kilitli katman” katmanındaki bir duvarı kaydırmak istiyor ve o katman kilitli. Kilidi açın ya da bağı kaldırın.")
            now[j] = dict(now[j], pts=[P(*p) for p in pts])
            changed[j] = now[j]
            moved_walls[j] = pts
    for i, o in ws.carried(state, moved_walls).items():
        now[i] = o
        changed[i] = o
    for k, key in enumerate(var):
        if key[0] != "opening" or x[k] == 0.0:
            continue
        o = copy.deepcopy(now[key[1]])
        o["at"] = o["at"] + x[k]
        h = host_of(now, o)
        o["place"] = [P(*p) for p in ws.jambs(now[h], o["at"], o["width"])]
        now[key[1]] = o
        changed[key[1]] = o
    for i, r in enumerate(rules):
        m = miss(now, r, values)
        if m is None or abs(m) > SETTLED:
            return None, ([i], "Bağlar duvarların izleyebildiği kadar tutamadı (bağlı duvarlar birbirini engelliyor)")
    return changed, None


# ── The bonds in the way (docs/adr/0780 §1) ─────────────────────────────────────────────────────────────────────────

LABELS = {"distance": "Uzaklık", "fix": "Sabit", "symmetric": "Simetrik", "parallel": "Paralel"}


def parts_of(rule):
    return [rule[k] for k in ("a", "b", "axis") if type(rule.get(k)) is dict]


def measured_or_none(state, rule):
    """A distance's clear distance as the drawing is (along the wall for two jambs); none when its parts do not fit."""
    if rule["kind"] != "distance":
        return None
    var, parts = items(state, [rule])
    a, b = parts[0]
    if a[0] == "line" and b[0] == "line" and abs(cross(a[3], b[3])) > PARALLEL:
        return None
    if a[0] == "jamb" and b[0] == "line" and abs(dot(left(b[3]), a[3])) < CROSSING:
        return None
    return measured(state, rule)[0]


def in_the_way(state, rules, values, conflicting, edited, ids=None):
    """Each conflicting rule as a refusal's bond: its parts, its value, what the drawing measures as the write leaves
    it, and whether the bonds solve without it alone (`frees`). `ids`: the rules' constraint ids (else `$any`)."""
    out = []
    # A conflict of the equations is tried again as far as the equations; one that came while the shifts were
    # written, whole (docs/adr/0780 §1).
    late = []
    follow(state, rules, values, edited=edited, late=late)
    for k in conflicting:
        r = rules[k]
        rest = [x for i, x in enumerate(rules) if i != k]
        _, why = (follow if late else equations)(state, rest, values, edited=edited)
        b = {"id": ids[k] if ids else "$any", "label": LABELS[r["kind"]]}
        if r["kind"] == "distance":
            b["parameter"] = r["parameter"]
        b["parts"] = parts_of(r)
        if r["kind"] == "distance" and not r.get("reference"):
            b["kept"] = values[r["parameter"]]
        m = measured_or_none(state, r)
        if m is not None:
            b["measured"] = m
        b["frees"] = why is None
        out.append(b)
    return out


def bonds_words(bonds):
    """The refusal's last words: the bond (or bonds) whose taking away alone lets the write through, or none."""
    frees = [f"{b['parameter']} ({b['label']})" if "parameter" in b else b["label"] for b in bonds if b["frees"]]
    if not frees:
        return "Tek başına kaldırılınca düzenlemeyi geçiren bağ yok; çelişen bağları kaldırın ya da kısıtları gevşeterek düzenleyin."
    if len(frees) == 1:
        return f"Şu bağ kaldırılınca düzenleme geçer: {frees[0]}. Ya da kısıtları gevşeterek düzenleyin."
    return f"Şu bağlardan biri kaldırılınca düzenleme geçer: {', '.join(frees)}. Ya da kısıtları gevşeterek düzenleyin."


# ── Steps ────────────────────────────────────────────────────────────────────────────────────────────────────────────

class Case:
    def __init__(self, name):
        self.name = name
        self.state = {e["id"]: copy.deepcopy(e) for e in ENTITIES}
        self.rules = []
        self.params = []   # (name, expression)
        self.steps = []

    def values(self):
        return {n: float(e) for n, e in self.params}

    def add(self, *news):
        """cad.constraints.edit add: each rule (or (rule, value)), its side from the drawing, a new parameter d1, d2 …
        valued what the drawing measures unless a value is given."""
        cons, rules, params = [], list(self.rules), list(self.params)
        for item in news:
            r0, value = item if isinstance(item, tuple) else (item, None)
            r = copy.deepcopy(r0)
            entry = {"rule": copy.deepcopy(r0)}
            if r["kind"] == "distance":
                g, side = measured(self.state, r)
                jj = r["a"]["element"]["kind"] == "jamb" and r["b"]["element"]["kind"] == "jamb"
                if jj and g < 0:
                    r["a"], r["b"] = r["b"], r["a"]
                    g = -g
                if side and not jj:
                    r["left"] = True
                name = f"d{1 + sum(1 for n, _ in params if n.startswith('d'))}"
                r["parameter"] = name
                if value is not None:
                    entry["value"] = value
                params.append((name, value if value is not None else wc_value(g)))
            rules.append(r)
            cons.append(entry)
        inp = {"edit": {"kind": "add", "constraints": cons}}
        values = {n: float(e) for n, e in params}
        changed, why = follow(self.state, rules, values)
        step = {"op": "execute", "input": inp}
        if why is not None:
            step["result"] = {"status": "failed", "error": {"code": "constraint_conflict", "message": "$any",
                                                            "path": "edit", "uids": "$any", "bonds": "$any"}}
            step["expect"] = {"ids": IDS, "revision": "same", "constraintKinds": [r["kind"] for r in self.rules]}
            self.steps.append(step)
            return
        self.rules, self.params = rules, params
        self.state.update(changed)
        step["within"] = 1e-9
        step["result"] = {"status": "completed", "output": {
            "added": ["$any"] * len(news), "removed": [], "changed": [u(i) for i in sorted(changed)],
            "diagnosis": "$any", "revision": "$current"}, "warnings": "$any"}
        step["expect"] = {"ids": IDS, "revision": "changed", "constraintKinds": [r["kind"] for r in self.rules],
                          "parameters": [{"name": n, "kind": "length", "expression": e} for n, e in self.params],
                          "entities": {str(i): self.state[i] for i in sorted(changed)}}
        self.steps.append(step)

    def edit(self, command, inp, write, relax=False):
        """Another command's write (`write(state)` gives its changed objects or a refusal), then the bonds."""
        before = {i: copy.deepcopy(e) for i, e in self.state.items()}
        got = write(self.state)
        step = {"op": "execute", "command": command, "input": inp}
        if relax:
            step["relaxConstraints"] = True
        if isinstance(got, dict) and "status" in got:
            step["result"] = got
            step["expect"] = {"ids": IDS, "revision": "same"}
            self.steps.append(step)
            return
        written = dict(self.state)
        written.update(got)
        edited = {i: before[i] for i in got}
        named = {i for r in self.rules for k in ("a", "b", "axis") if type(r.get(k)) is dict for i in [el_id(r[k])]}
        if relax:
            values = self.values()
            gone = [i for i, r in enumerate(self.rules) if (lambda m: m is None or abs(m) > SETTLED)(miss(written, r, values))]
            if gone:
                self.rules = [r for i, r in enumerate(self.rules) if i not in gone]
                used = {r.get("parameter") for r in self.rules}
                self.params = [(n, e) for n, e in self.params if n in used]
            self.state = written
            step["within"] = 1e-9
            step["result"] = "$any"
            step["expect"] = {"ids": IDS, "revision": "changed", "constraintKinds": [r["kind"] for r in self.rules],
                              "entities": {str(i): written[i] for i in sorted(got)}}
            self.steps.append(step)
            return
        if not (named & set(got)):
            changed, why = {}, None
        else:
            changed, why = follow(written, self.rules, self.values(), edited=edited)
        if why is not None:
            bonds = in_the_way(written, self.rules, self.values(), why[0], edited)
            step["result"] = {"status": "failed", "error": {
                "code": "constraint_conflict",
                "message": f"{why[1]}; hiçbir şey yazılmadı. {bonds_words(bonds)}",
                "uids": "$any", "bonds": bonds}}
            step["expect"] = {"ids": IDS, "revision": "same"}
            self.steps.append(step)
            return
        written.update(changed)
        every = dict(got)
        every.update(changed)
        self.state = written
        step["within"] = 1e-9
        step["result"] = "$any"
        step["expect"] = {"ids": IDS, "revision": "changed", "constraintKinds": [r["kind"] for r in self.rules],
                          "entities": {str(i): written[i] for i in sorted(every)}}
        self.steps.append(step)

    def shift(self, i, by, edge=0, relax=False):
        inp = {"uids": [ws.u(i)], "operation": {"kind": "shift", "by": by, **({"edge": edge} if edge else {})}}

        def write(state):
            result, changed = ws.edit(state, inp)
            return result if changed is None else changed
        self.edit("cad.walls.edit", inp, write, relax)

    def keep(self, i, side, distance, edge=0, to=None):
        """cad.walls.edit's Mesafe with `keep` (Bu ölçüyü koru, §3): the edge moved to `distance` as Mesafe moves it
        (none when it is there already), then a distance bond from the facing face across to this edge's face on that
        side, valued what the drawing measures, in the same step."""
        op = {"kind": "distance", "side": side, "distance": distance, "keep": True, **({"edge": edge} if edge else {}),
              **({"to": to} if to else {})}
        inp = {"uids": [ws.u(i)], "operation": op}
        step = {"op": "execute", "command": "cad.walls.edit", "input": inp}
        plain = {"uids": [ws.u(i)], "operation": {k: v for k, v in op.items() if k != "keep"}}
        result, changed = ws.edit(self.state, plain)
        if changed is None and result["error"]["code"] != "nothing_to_set":
            step["result"] = result
            step["expect"] = {"ids": IDS, "revision": "same"}
            self.steps.append(step)
            return
        if to == "grid":
            step["result"] = {"status": "failed", "error": {
                "code": "invalid_wall", "message": "Bu ölçüyü koru yalnız duvar yüzüne olan mesafede olur; aks bağlanmaz.",
                "path": "operation.keep"}}
            step["expect"] = {"ids": IDS, "revision": "same"}
            self.steps.append(step)
            return
        written = dict(self.state)
        written.update(changed or {})
        g = ws.gaps(written, i, edge)[0 if side == "left" else 1]
        rule = dist(W(g["wall"], "leftFace" if g["left"] else "rightFace", g["edge"]),
                    W(i, "leftFace" if side == "left" else "rightFace", edge))
        gap, sense = measured(written, rule)
        if sense:
            rule["left"] = True
        name = f"d{1 + sum(1 for n, _ in self.params if n.startswith('d'))}"
        rule["parameter"] = name
        self.rules.append(rule)
        self.params.append((name, wc_value(gap)))
        self.state = written
        step["within"] = 1e-9
        step["result"] = "$any"
        step["expect"] = {"ids": IDS, "revision": "changed", "constraintKinds": [r["kind"] for r in self.rules],
                          "parameters": [{"name": n, "kind": "length", "expression": e} for n, e in self.params],
                          "entities": {str(k): written[k] for k in sorted(changed or {})}}
        self.steps.append(step)

    def set_parameter(self, name, expression):
        inp = {"edits": [{"kind": "set", "name": name, "expression": expression}]}
        params = [(n, expression if n == name else e) for n, e in self.params]
        values = {n: float(e) for n, e in params}
        changed, why = follow(self.state, self.rules, values)
        step = {"op": "execute", "command": "cad.parameters.edit", "input": inp}
        if why is not None:
            step["result"] = {"status": "failed", "error": {"code": "constraint_conflict", "message": "$any", "path": "edit",
                                                            "uids": "$any", "bonds": "$any"}}
            step["expect"] = {"ids": IDS, "revision": "same"}
            self.steps.append(step)
            return
        self.params = params
        self.state.update(changed)
        step["within"] = 1e-9
        step["result"] = "$any"
        step["expect"] = {"ids": IDS, "revision": "changed",
                          "parameters": [{"name": n, "kind": "length", "expression": e} for n, e in self.params],
                          "entities": {str(i): self.state[i] for i in sorted(changed)}}
        self.steps.append(step)

    def move_opening(self, i, by):
        inp = {"uids": [ws.u(i)], "operation": {"kind": "move", "by": by}}

        def write(state):
            o = copy.deepcopy(state[i])
            o["at"] = o["at"] + by
            o["place"] = [P(*p) for p in ws.jambs(state[host_of(state, o)], o["at"], o["width"])]
            return {i: o}
        self.edit("cad.openings.edit", inp, write)

    def undo(self, returns, entities):
        self.steps.append({"op": "undo", "returns": returns, "within": 1e-9,
                           "expect": {"canRedo": True, "constraintKinds": [r["kind"] for r in self.rules],
                                      "entities": {str(i): e for i, e in entities.items()}}})

    def done(self):
        return {"name": self.name, "steps": self.steps}


def strip(r):
    return {k: v for k, v in r.items() if k != "left"}


def wc_value(v):
    t = f"{v:.12f}".rstrip("0").rstrip(".")
    return "0" if t == "-0" else t


def dist(a, b):
    return {"kind": "distance", "a": a, "b": b, "parameter": ""}


CORRIDOR = dist(W(1, "leftFace"), W(6, "rightFace"))   # S's inner face to C's lower face: 1.20


def cases():
    out = []

    def case(name):
        c = Case(name)
        out.append(c)
        return c

    c = case("Koridor bağı: ölçülen 1,20 m değer olur, hiçbir şey kımıldamaz")
    c.add(CORRIDOR)
    c = case("Koridor bağı tutar: güney duvarı 50 cm dışa, koridor duvarı da 50 cm izler")
    c.add(CORRIDOR)
    c.shift(1, -0.5)
    c = case("Bağsız: güney duvarı kayar, koridor duvarı yerinde")
    c.shift(1, -0.5)
    c = case("Koridor duvarını kaydırmak bağın öbür yanını (dış duvar) da kaydırır")
    c.add(CORRIDOR)
    c.shift(6, 0.3)
    c = case("Sabit dış duvar ve koridor bağı: koridor duvarını kaydırmak reddedilir")
    c.add(CORRIDOR, {"kind": "fix", "a": W(1, "axis")})
    c.shift(6, 0.3)
    c = case("Sabit duvarı kaydırmak reddedilir")
    c.add({"kind": "fix", "a": W(1, "axis")})
    c.shift(1, -0.5)
    c = case("Aynı yüzler arasına iki değer: ikisi birlikte eklenmez")
    c.add(CORRIDOR, (CORRIDOR, "1.5"))
    c = case("Parametre 1,50 m: bağın b yanı (koridor duvarı) kayar, dış duvar yerinde")
    c.add(CORRIDOR)
    c.set_parameter("d1", "1.5")
    c = case("Yeni bağ başka değerle: a yerinde, b kayar")
    c.add((CORRIDOR, "1.0"))
    c = case("Eşit aralık: bölme iki yan duvarın ortasında; doğu duvarı 1 m dışa, bölme 50 cm izler")
    c.add({"kind": "symmetric", "a": W(4, "leftFace"), "b": W(2, "leftFace"), "axis": W(5, "axis")})
    c.shift(2, -1.0)
    c = case("Eşit aralığın eksenini (bölmeyi) kaydırmak reddedilir")
    c.add({"kind": "symmetric", "a": W(4, "leftFace"), "b": W(2, "leftFace"), "axis": W(5, "axis")})
    c.shift(5, 0.3)
    c = case("Kapı pervazı batı duvarına 1,45 m: batı duvarı 50 cm dışa, kapı da 50 cm kayar")
    c.add(dist(J(7, "start"), W(4, "leftFace")))
    c.shift(4, -0.5)
    c = case("İki kapı arası 3,10 m: ikinci kapı 40 cm doğuya, ilki izler")
    c.add(dist(J(7, "end"), J(8, "start")))
    c.move_opening(8, 0.4)
    c = case("Bağlar zinciri: koridor ve oda derinliği; güney duvarı dışa, ikisi de izler")
    c.add(CORRIDOR, dist(W(6, "leftFace"), W(3, "leftFace")))
    c.shift(1, -0.5)
    c = case("Gevşeterek düzenleme: koridor bağı kalkar, koridor duvarı yerinde")
    c.add(CORRIDOR)
    c.shift(1, -0.5, relax=True)
    c = case("Kilitli katmandaki duvar bağla kımıldamaz: bağlı duvarı kaydırmak reddedilir")
    c.add(dist(W(11, "leftFace"), W(12, "rightFace")))
    c.shift(12, 0.4)
    c = case("Başvuru (yalnız ölçen) bağ kımıldatmaz")
    rr = dist(W(1, "leftFace"), W(6, "rightFace"))
    c.steps.append({"op": "execute", "input": {"edit": {"kind": "add", "constraints": [{"rule": rr, "reference": True}]}},
                    "result": {"status": "completed", "output": "$any", "warnings": "$any"},
                    "expect": {"ids": IDS, "revision": "changed", "constraintKinds": ["distance"]}})
    c = case("Paralel bağ: iki eksen paralel, kaydırma izin verilir")
    c.add({"kind": "parallel", "a": W(1, "axis"), "b": W(3, "axis")})
    c.shift(1, -0.5)
    c = case("Geri al: bağın kaydırdığı da tek adımda geri gelir")
    c.add(CORRIDOR)
    snap = {i: copy.deepcopy(c.state[i]) for i in (1, 2, 4, 5, 6, 7, 8)}
    c.shift(1, -0.5)
    c.undo("Duvarı düzenle", snap)
    c = case("Kuzey duvarı içe: oda derinliği bağı koridor duvarını iter")
    c.add(dist(W(6, "leftFace"), W(3, "leftFace")))
    c.shift(3, 0.3)
    c = case("Bölmedeki kapının pervazı güney duvarına: güney duvarı dışa, kapı bölme boyunca izler")
    c.add(dist(J(9, "start"), W(1, "leftFace")))
    c.shift(1, -0.4)
    c = case("Bu ölçüyü koru: koridor 1,20 → 1,50 m, bağ aynı adımda; güney duvarı dışa, koridor duvarı izler")
    c.keep(6, "right", 1.5)
    c.shift(1, -0.5)
    c = case("Bu ölçüyü koru, mesafe değişmeden: yalnız bağ yazılır, hiçbir şey kımıldamaz")
    c.keep(6, "right", 1.2)
    c = case("Bu ölçüyü koru aksa: yanında aks yoksa reddedilir")
    c.keep(6, "right", 1.2, to="grid")
    c = case("Bu ölçüyü koru, o yanda paralel duvar yok: reddedilir")
    c.keep(12, "left", 1.0)
    return [x.done() for x in out]


def refusals():
    """Bonds the drawing cannot take (`invalid_constraint`), nothing written."""
    def refused(name, rule, why):
        return {"name": name, "steps": [{"op": "execute", "input": {"edit": {"kind": "add", "constraints": [{"rule": rule}]}},
                                         "result": {"status": "failed", "error": {"code": "invalid_constraint",
                                                                                  "message": f"Bağ kurulamıyor: {why}.",
                                                                                  "path": "edit.constraints[0].rule",
                                                                                  "uids": "$any"}},
                                         "expect": {"ids": IDS, "revision": "same"}}]}
    return [
        refused("Yay kenara bağ kurulmaz", dist(W(10, "axis"), W(12, "axis")),
                "duvarın 1. kenarının eksen bulunamadı: kenar yay ya da yok, ya da açıklık duvarının dışında"),
        refused("Olmayan kenara bağ kurulmaz", dist(W(1, "axis", edge=3), W(3, "axis")),
                "duvarın 4. kenarının eksen bulunamadı: kenar yay ya da yok, ya da açıklık duvarının dışında"),
        refused("Paralel olmayan duvar çizgileri arasına uzaklık kurulmaz", dist(W(1, "axis"), W(2, "axis")),
                "bağlanan duvar çizgileri paralel olmalı"),
        refused("Pervaz kendi duvarına paralel çizgiye bağlanmaz", dist(J(7, "start"), W(3, "rightFace")),
                "pervazın bağlandığı duvar çizgisi açıklığın duvarını en az 30° ile kesmeli"),
        refused("İki pervaz ayrı duvarlarda olamaz", dist(J(7, "start"), J(9, "start")),
                "iki pervaz aynı duvarın aynı kenarında olmalı"),
        refused("Pervaz bir açıklığın olmalı", {"kind": "distance", "a": {"uid": u(1), "element": {"kind": "jamb", "end": "start"}},
                                               "b": W(4, "leftFace"), "parameter": ""}, "pervaz yalnız bir açıklıkta olur"),
    ]


def document():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.constraints.edit", "commandVersion": 1,
            "title": "Mimari bağlar: duvar yüzü, eksen ve pervaz arasında kalıcı ölçü (ADR 0770)",
            "note": "Beklenenler scripts/fixtures/arch_bond_cases.py'den: ADR 0770 §4'ün doğrusal kuralları, Kaydır ve "
                    "açıklıkların taşınması wall_shift_cases.py'nin bağımsız kuralıyla; KentOS kodu kullanılmadı.",
            "setup": SETUP, "cases": cases() + refusals()}


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
        print(f"cad.constraints.edit (mimari): {len(doc['cases'])} durum bağımsız başvuruyla aynı.")
    else:
        OUT.write_text(encoded)
        print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
