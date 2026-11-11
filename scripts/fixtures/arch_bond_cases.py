#!/usr/bin/env python3
"""Shared cases of the architectural bonds (docs/adr/0770 §4, §5), from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.constraints.edit.arch.json; `--check` compares and writes nothing. A case adds bonds
with cad.constraints.edit (a wall's line: `wall` edge and line; an opening's jamb: `jamb` end; a straight grid's line:
`grid`, docs/adr/8049 §2), then edits walls, a parameter, an opening or moves a grid; what the bonds make of it is
written here:

- an arc edge's line (ADR 8049 §4) is its circle: its shift to its left changes its radius by −t·Δ (t +1 for an arc
  that turns left); two arcs' lines "run together" when concentric (centres within 1e-6 m), a line R to a's left is
  (r_a − R)·t_a away and a unit shift of b moves it t_a·t_b; a straight line never runs with an arc's, a jamb is not
  bonded to one;
- every bonded edge and grid shifts square to itself (`Δ` along its left normal, a grid's from its first point to its
  second), every bonded opening along its edge (`δ`);
  the rules are linear (the ADR's table); a distance's side `left` is read from the drawing when it is made (two jambs
  in their order along the wall);
- an edit holds the edges and grids whose line it moved or turned and the openings it moved along their wall (their start jamb
  then, from their place, and now, from their wall and `at`); a new bond or a parameter's new value holds the `a` of
  each distance that does not hold now; equal spacing (symmetric) always holds its two ends, its axis follows; a fix whose part an edit moved, lines a
  bond wants parallel that are not, rows left contradicting: refused (`constraint_conflict`), nothing written;
- the rest is the least change: rows in the bonds' order, shifts in their order (edges and grids first as the bonds
  name them, then openings), the first row with a coefficient the pivot, x = Rᵀ (R Rᵀ)⁻¹ d;
- each edge shifted by Kaydır (wall_shift_cases.py's independent rule, joined walls following), in the shifts' order;
  the openings of moved walls read back from their jambs; each grid's two points moved along its left normal; then
  each bonded opening's `at` plus its shift.

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
import column_shift_cases as cs  # noqa: E402  (a column's sides and Mesafe, independently)

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
GRIDS = {
    13: ws.grid(13, (-2, -1.5), (10, -1.5), "A"),   # south of S: S's axis 1.50 m from it
    14: ws.grid(14, (-1, -2), (-1, 7), "1"),        # west of W, square to A
    15: ws.grid(15, (-2, 25), (6, 25), "B", bulges=[0.2, 0.0]),   # an arc grid
}
COLUMNS = {
    # A free 40 x 40 cm column in room B, its lower side 0.20 m from S's inner face.
    16: {"kind": "column", "id": 16, "layerId": "kolon", "attrs": {}, "at": ws.P(6.0, 0.5), "width": 0.4, "depth": 0.4},
    # A column where grids A and 1 cross, tied to them.
    17: {"kind": "column", "id": 17, "layerId": "kolon", "attrs": {}, "at": ws.P(-1.0, -1.5), "width": 0.4, "depth": 0.4,
         "grids": [ws.uid(13), ws.uid(14)], "place": "A-1"},
}
# A curved corridor (ADR 8049 §4): quarter arcs of radius 3 and 4.4 about (20, 10), from 225° to 315°, both turning
# left: 1.20 m clear between the inner's outer face and the outer's inner face.
ARCS = {i: ws.wall(i, [(20 + r * math.cos(math.radians(225)), 10 + r * math.sin(math.radians(225))),
                       (20 + r * math.cos(math.radians(315)), 10 + r * math.sin(math.radians(315)))],
                   bulges=[math.tan(math.pi / 8), 0.0])
        for i, r in ((18, 3.0), (19, 4.4))}
ENTITIES = [WALLS[1], WALLS[2], WALLS[3], WALLS[4], WALLS[5], WALLS[6],
            door(7, 1, 2.0, 0.9, WALLS), door(8, 1, 6.0, 0.9, WALLS), door(9, 5, 3.0, 0.8, WALLS),
            WALLS[10], WALLS[11], WALLS[12], GRIDS[13], GRIDS[14], GRIDS[15], COLUMNS[16], COLUMNS[17],
            ARCS[18], ARCS[19]]
SETUP = copy.deepcopy(ws.SETUP)
SETUP["layers"] = SETUP["layers"] + [wc.layer("kolon", "Kolon")]
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


def G(i):
    return {"uid": u(i), "element": {"kind": "grid"}}


def C(i, side):
    return {"uid": u(i), "element": {"kind": "columnSide", "side": side}}


# ── Geometry of the parts ────────────────────────────────────────────────────────────────────────────────────────────

xy, sub, dot, cross, left, unit, along = ws.xy, ws.sub, ws.dot, ws.cross, ws.left, ws.unit, ws.along


def line_offset(w, line):
    lf = ws.left_face(w)
    rf = lf - w["width"]
    # No components here: the core is the faces.
    return {"leftFace": lf, "rightFace": rf, "axis": (lf + rf) / 2, "leftCore": lf, "rightCore": rf}[line]


def wall_line(state, el):
    """(point on the line, edge direction, its circle or none) of a wall's line: on an arc edge (ADR 8049 §4) the point
    at its start's radius, its way there and (centre, radius moved by the line's offset to its left, way round)."""
    w = state[el_id(el)]
    e = el["element"]
    ed = ws.edge(w, e["edge"])
    if ed is not None:
        a, b = ed
        d = unit(a, b)
        return along(a, left(d), line_offset(w, e["line"])), d, None
    c, r, sweep = ws.arc_of(w, e["edge"])
    turn = 1.0 if sweep > 0 else -1.0
    rl = r - turn * line_offset(w, e["line"])
    a = xy(w["pts"][e["edge"]])
    u0 = unit(c, a)
    return along(c, u0, rl), (-u0[1] * turn, u0[0] * turn), (c, rl, turn)


CONCENTRIC = 1e-6


def together(a, b):
    """Two lines (parts) that run together: straight and parallel, or round and concentric."""
    ra, rb = a[4], b[4]
    if ra is None and rb is None:
        return abs(cross(a[3], b[3])) <= PARALLEL
    if ra is not None and rb is not None:
        return math.hypot(ra[0][0] - rb[0][0], ra[0][1] - rb[0][1]) <= CONCENTRIC
    return False


def offset(a, b):
    """How far line b is to line a's left."""
    if a[4] is not None and b[4] is not None:
        return (a[4][1] - b[4][1]) * a[4][2]
    return dot(left(a[3]), sub(b[2], a[2]))


def unit_shift(a, b):
    """How far a unit shift of b (to its left) moves it to a's left."""
    if a[4] is not None and b[4] is not None:
        return a[4][2] * b[4][2]
    return dot(left(a[3]), left(b[3]))


def el_id(el):
    return int(el["uid"].rsplit("-", 1)[1], 16)


def straight(g):
    return g["kind"] == "grid" and len(g["pts"]) == 2 and all(x == 0.0 for x in (g.get("bulges") or []))


def grid_line(state, el):
    """(its first point, its direction) of a straight grid's line (docs/adr/8049 §2)."""
    g = state[el_id(el)]
    a, b = xy(g["pts"][0]), xy(g["pts"][1])
    return a, unit(a, b)


def column_line(state, el):
    """(its start, the direction whose left normal its shift moves it along, its shift's key) of a column's side
    (docs/adr/8049 §3): the column's own across sides `side % 2` (the direction of side 0 or 1), or for a column tied to
    grids its grid parallel to the side (the direction then the grid's way round)."""
    c = state[el_id(el)]
    s = el["element"]["side"]
    sd = cs.sides(c)
    a = sd[s][0]
    d = unit(*sd[s % 2])
    for g in c.get("grids", []):
        gid = el_id({"uid": g})
        if gid not in state or not straight(state[gid]):
            continue
        gd = grid_line(state, {"uid": g})[1]
        if abs(cross(d, gd)) <= PARALLEL:
            return a, (d if dot(d, gd) > 0 else (-d[0], -d[1])), ("grid", gid)
    return a, d, ("column", el_id(el), s % 2)


def crossing(state, c):
    """Where a tied column's two straight grids cross."""
    (pa, da), (pb, db) = (grid_line(state, {"uid": g}) for g in c["grids"])
    t = cross(sub(pb, pa), db) / cross(da, db)
    return along(pa, da, t)


def settle(state, changed):
    """The columns tied to grids `changed` moved, settled where their grids cross (docs/adr/0343 §3), into both."""
    for i in sorted(state):
        c = state[i]
        if c["kind"] != "column" or "grids" not in c:
            continue
        if not any(el_id({"uid": g}) in changed for g in c["grids"]):
            continue
        p = crossing(state, c)
        if dist2(p, xy(c["at"])) > 0.0:
            moved = dict(c, at=P(*p))
            state[i] = moved
            changed[i] = moved


def dist2(a, b):
    return (a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2


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
            elif type(el) is dict and el["element"]["kind"] == "grid":
                v(("grid", el_id(el)))
            elif type(el) is dict and el["element"]["kind"] == "columnSide":
                v(column_line(state, el)[2])
    parts = []
    for r in rules:
        got = []
        for k in ("a", "b", "axis"):
            el = r.get(k)
            if type(el) is not dict:
                continue
            if el["element"]["kind"] == "wall":
                at, d, rd = wall_line(state, el)
                got.append(("line", v(("edge", el_id(el), el["element"]["edge"])), at, d, rd))
            elif el["element"]["kind"] == "grid":
                at, d = grid_line(state, el)
                got.append(("line", v(("grid", el_id(el))), at, d, None))
            elif el["element"]["kind"] == "columnSide":
                at, d, key = column_line(state, el)
                got.append(("line", v(key), at, d, None))
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
            if not together(ps[0], ps[1]):
                return k, "parallel"
            continue
        if kind == "fix":
            co[ps[0][1]] += 1.0
            out.append((co, 0.0, {k}))
            continue
        if kind == "symmetric":
            a, b, ax = ps
            if not together(a, ax) or not together(b, ax):
                return k, "parallel"
            co[a[1]] += unit_shift(ax, a)
            co[b[1]] += unit_shift(ax, b)
            co[ax[1]] -= 2.0
            out.append((co, -(offset(ax, a) + offset(ax, b)), {k}))
            continue
        a, b = ps
        value = values[r["parameter"]]
        sigma = 1.0 if r.get("left") else -1.0
        if a[0] == "line":
            if not together(a, b):
                return k, "parallel"
            co[b[1]] += unit_shift(a, b)
            co[a[1]] -= 1.0
            out.append((co, sigma * value - offset(a, b), {k}))
        elif b[0] == "line":
            if b[4] is not None:
                return k, "crossing"
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
        g = offset(a, b)
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
        # A tied column whose side no grid is parallel to moves only with its grids.
        if key[0] == "column" and "grids" in state[oid]:
            held[k] = True
        if edited and oid in edited:
            before = edited[oid]
            if key[0] == "edge" and ws.bulge(before, key[2]) != 0.0 and ws.bulge(state[oid], key[2]) != 0.0:
                # An arc moved when its circle did: its centre, its radius or its way round (ADR 8049 §4).
                (c0, r0, s0), (c1, r1, s1) = ws.arc_of(before, key[2]), ws.arc_of(state[oid], key[2])
                held[k] = (held[k] or math.hypot(c1[0] - c0[0], c1[1] - c0[1]) > TIE or abs(r1 - r0) > TIE
                           or (s0 > 0) != (s1 > 0))
            elif key[0] == "edge":
                b0 = ws.edge(before, key[2])
                b1 = ws.edge(state[oid], key[2])
                if b0 is None or b1 is None:
                    held[k] = True
                else:
                    u0, u1 = unit(*b0), unit(*b1)
                    across = dot(left(u0), sub(b1[0], b0[0]))
                    held[k] = held[k] or abs(cross(u0, u1)) > PARALLEL or abs(across) > TIE
            elif key[0] == "column":
                # A column turned or moved across its sides `axis` (either of the two).
                s0, s1 = cs.sides(before), cs.sides(state[oid])
                ax = key[2]
                u0, u1 = unit(*s0[ax]), unit(*s1[ax])
                n = left(u0)
                held[k] = (held[k] or abs(cross(u0, u1)) > PARALLEL
                           or abs(dot(n, sub(s1[ax][0], s0[ax][0]))) > TIE
                           or abs(dot(n, sub(s1[ax + 2][0], s0[ax + 2][0]))) > TIE)
            elif key[0] == "grid":
                # A grid moved across or turned (one lengthened along itself is not).
                a0, b0 = xy(before["pts"][0]), xy(before["pts"][1])
                a1, u1 = grid_line(state, {"uid": u(oid)})
                u0 = unit(a0, b0)
                held[k] = held[k] or abs(cross(u0, u1)) > PARALLEL or abs(dot(left(u0), sub(a1, a0))) > TIE
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
        gridded = any(p[0] == "line" and var[p[1]][0] == "grid" for p in parts[eqs[0]])
        rounded = any(p[0] == "line" and p[4] is not None for p in parts[eqs[0]])
        parallel = ("Bağlı yay çizgileri artık eş merkezli değil (bir yay ya da duvar taşındı)" if rounded
                    else "Bağlı aks ve duvar çizgileri artık paralel değil (bir aks ya da duvar döndü)" if gridded
                    else "Bağlı duvar çizgileri artık paralel değil (bir duvar döndü)")
        return None, ([eqs[0]], {"parallel": parallel,
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
            now[j] = dict(now[j], pts=[P(*p) for p in pts],
                          **({"bulges": moved.bulges[j]} if j in getattr(moved, "bulges", {}) else {}))
            changed[j] = now[j]
            moved_walls[j] = pts
    for i, o in ws.carried(state, moved_walls).items():
        now[i] = o
        changed[i] = o
    # The grids' lines square to themselves (docs/adr/8049 §2).
    for k, key in enumerate(var):
        if key[0] != "grid" or x[k] == 0.0:
            continue
        g = copy.deepcopy(now[key[1]])
        a, d = grid_line(now, {"uid": u(key[1])})
        n = left(d)
        g["pts"] = [P(q["x"] + n[0] * x[k], q["y"] + n[1] * x[k]) for q in g["pts"]]
        now[key[1]] = g
        changed[key[1]] = g
    # The free columns square to their sides (docs/adr/8049 §3).
    for k, key in enumerate(var):
        if key[0] != "column" or x[k] == 0.0:
            continue
        c = copy.deepcopy(now[key[1]])
        n = left(unit(*cs.sides(c)[key[2]]))
        c["at"] = P(c["at"]["x"] + n[0] * x[k], c["at"]["y"] + n[1] * x[k])
        now[key[1]] = c
        changed[key[1]] = c
    # The tied columns the bonds name, settled on their grids as moved.
    named = {el_id(r[k]) for r in rules for k in ("a", "b", "axis")
             if type(r.get(k)) is dict and r[k]["element"]["kind"] == "columnSide"}
    for i in sorted(named):
        c = now[i]
        if "grids" in c and any(el_id({"uid": g}) in changed for g in c["grids"]):
            p = crossing(now, c)
            if dist2(p, xy(c["at"])) > 0.0:
                now[i] = dict(c, at=P(*p))
                changed[i] = now[i]
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
    if a[0] == "line" and b[0] == "line" and not together(a, b):
        return None
    if a[0] == "jamb" and b[0] == "line" and (b[4] is not None or abs(dot(left(b[3]), a[3])) < CROSSING):
        return None
    return measured(state, rule)[0]


SET_TRIES = 48   # sets of two or three bonds tried at most (docs/adr/0790 §2)


def without(state, rules, values, edited, gone, late):
    """The rules without those at `gone` solved again (whole when the conflict came late, else its equations): None
    when the write goes through, else the indices (into `rules`) that still conflict."""
    kept = [i for i in range(len(rules)) if i not in gone]
    _, why = (follow if late else equations)(state, [rules[i] for i in kept], values, edited=edited)
    return None if why is None else [kept[k] for k in why[0]]


def smallest(alone, attempt_without):
    """The smallest set of bonds whose taking away together lets the write through, when none alone does
    (docs/adr/0790 §2): each bond with what still conflicts without it, grown to pairs, then the pairs to triples, in
    the order the conflicts name them; the set in the drawing's order, or [] past SET_TRIES sets tried."""
    tried, pairs = [], []

    def attempt(members):
        key = sorted(members)
        if key in tried or len(tried) >= SET_TRIES:
            return None
        tried.append(key)
        still = attempt_without(key)
        return ("frees", key) if still is None else ("still", still)

    for a, still in alone:
        for b in still:
            if b == a:
                continue
            got = attempt([a, b])
            if got is None:
                continue
            if got[0] == "frees":
                return got[1]
            pairs.append(([a, b], got[1]))
    for pair, still in pairs:
        for c in still:
            if c in pair:
                continue
            got = attempt(pair + [c])
            if got is not None and got[0] == "frees":
                return got[1]
    return []


def in_the_way(state, rules, values, conflicting, edited, ids=None):
    """Each conflicting rule as a refusal's bond: its parts, its value, what the drawing measures as the write leaves
    it, and whether the bonds solve without it alone (`frees`); when none does, the smallest set that does together
    (`freesWith`, docs/adr/0790 §2), its members outside the conflict after them. `ids`: the rules' constraint ids
    (else `$any`)."""
    # A conflict of the equations is tried again as far as the equations; one that came while the shifts were
    # written, whole (docs/adr/0780 §1).
    late = []
    follow(state, rules, values, edited=edited, late=late)
    frees, alone = [], []
    for k in conflicting:
        still = without(state, rules, values, edited, [k], bool(late))
        if still is None:
            frees.append(k)
        else:
            alone.append((k, still))
    together = [] if frees else smallest(alone, lambda gone: without(state, rules, values, edited, gone, bool(late)))
    out = []
    for k in conflicting + [k for k in together if k not in conflicting]:
        r = rules[k]
        b = {"id": ids[k] if ids else "$any", "label": LABELS[r["kind"]]}
        if r["kind"] == "distance":
            b["parameter"] = r["parameter"]
        b["parts"] = parts_of(r)
        if r["kind"] == "distance" and not r.get("reference"):
            b["kept"] = values[r["parameter"]]
        m = measured_or_none(state, r)
        if m is not None:
            b["measured"] = m
        b["frees"] = k in frees
        if k in together:
            b["freesWith"] = [ids[o] if ids else "$any" for o in together if o != k]
        out.append(b)
    return out


def bonds_words(bonds):
    """The refusal's last words: the bond (or bonds) whose taking away alone lets the write through, the set that does
    together, or none."""
    def name(b):
        return f"{b['parameter']} ({b['label']})" if "parameter" in b else b["label"]
    frees = [name(b) for b in bonds if b["frees"]]
    together = [name(b) for b in bonds if b.get("freesWith")]
    if not frees and together:
        n = "iki" if len(together) == 2 else "üç"
        return (f"Tek başına kaldırılınca düzenlemeyi geçiren bağ yok; şu {n} bağ birlikte kaldırılınca geçer: "
                f"{', '.join(together)}. Ya da kısıtları gevşeterek düzenleyin.")
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
        settle(self.state, changed)
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
        # A tied column's side is its grid's line: its grids are named too (docs/adr/8049 §3).
        named |= {el_id({"uid": g}) for r in self.rules for k in ("a", "b", "axis")
                  if type(r.get(k)) is dict and r[k]["element"]["kind"] == "columnSide"
                  for g in before[el_id(r[k])].get("grids", [])}
        if relax:
            settle(written, got)
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
            # The tied columns the write's grids moved, where the bonds read them (the step settles them as it closes).
            pre = dict(got)
            settle(written, pre)
            changed, why = follow(written, self.rules, self.values(), edited=edited)
            for i in pre:
                if i not in got:
                    changed.setdefault(i, written[i])
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
        settle(written, every)
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
        written = dict(self.state)
        written.update(changed or {})
        if to == "grid":
            # The grid beside it on that side, found before the move (docs/adr/8049 §2), to this wall's axis.
            at, dirn = ws.axis_middle(self.state[i], edge)
            g = ws.beside(self.state, ws.level(self.state[i]), at, dirn)[0 if side == "left" else 1]
            rule = dist(G(g["grid"]), W(i, "axis", edge))
        else:
            g = ws.gaps(written, i, edge)[0 if side == "left" else 1]
            rule = dist(W(g["wall"], "leftFace" if g["left"] else "rightFace", g["edge"]),
                        W(i, "leftFace" if side == "left" else "rightFace", edge))
        _, sense = measured(written, rule)
        if sense:
            rule["left"] = True
        name = f"d{1 + sum(1 for n, _ in self.params if n.startswith('d'))}"
        rule["parameter"] = name
        self.rules.append(rule)
        # The distance given, not the gap measured (equal but for rounding far from the origin).
        self.params.append((name, wc_value(distance)))
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
        settle(self.state, changed)
        step["within"] = 1e-9
        step["result"] = "$any"
        step["expect"] = {"ids": IDS, "revision": "changed",
                          "parameters": [{"name": n, "kind": "length", "expression": e} for n, e in self.params],
                          "entities": {str(i): self.state[i] for i in sorted(changed)}}
        self.steps.append(step)

    def move_grid(self, i, dx, dy):
        """cad.entities.transform's move of a grid: its two points moved by (dx, dy)."""
        inp = {"uids": [ws.u(i)], "transform": {"kind": "move", "dx": dx, "dy": dy}}

        def write(state):
            g = copy.deepcopy(state[i])
            g["pts"] = [P(q["x"] + dx, q["y"] + dy) for q in g["pts"]]
            return {i: g}
        self.edit("cad.entities.transform", inp, write)

    def move_column(self, i, dx, dy):
        """cad.entities.transform's move of a free column."""
        inp = {"uids": [ws.u(i)], "transform": {"kind": "move", "dx": dx, "dy": dy}}

        def write(state):
            c = copy.deepcopy(state[i])
            c["at"] = P(c["at"]["x"] + dx, c["at"]["y"] + dy)
            return {i: c}
        self.edit("cad.entities.transform", inp, write)

    def keep_column(self, i, side, distance, to=None):
        """cad.arch.edit's Mesafe with `keep` (docs/adr/8049 §3): the column moved to `distance` as Mesafe moves it
        (column_shift_cases.py's rule), then a distance bond from the face across (read before the move) to this
        column's side, valued `distance`, in the same step."""
        op = {"kind": "distance", "side": side, "distance": distance, **({"to": to} if to else {}), "keep": True}
        inp = {"uids": [ws.u(i)], "operation": op}
        step = {"op": "execute", "command": "cad.arch.edit", "input": inp}
        plain = {"uids": [ws.u(i)], "operation": {k: v for k, v in op.items() if k != "keep"}}
        result, changed = cs.edit(self.state, plain)
        if changed is None and result["error"]["code"] == "nothing_to_set" and to != "grid":
            changed = {}
        if changed is None or to == "grid":
            if changed is not None:
                result = {"status": "failed", "error": {
                    "code": "invalid_column",
                    "message": "Bu ölçüyü koru kolonda yalnız yüze olan mesafede olur; kolonu aksla götürmek için "
                               "Akslara bağla ile aksların kesişimine oturtun.",
                    "path": "operation.keep"}}
            step["result"] = result
            step["expect"] = {"ids": IDS, "revision": "same"}
            self.steps.append(step)
            return
        g = cs.gaps(self.state, i)[side]
        a = (W(g["id"], "leftFace" if g["face"] == "left" else "rightFace", g["edge"]) if g["target"] == "wall"
             else C(g["id"], g["edge"]))
        rule = dist(a, C(i, side))
        written = dict(self.state)
        written.update(changed)
        _, sense = measured(written, rule)
        if sense:
            rule["left"] = True
        name = f"d{1 + sum(1 for n, _ in self.params if n.startswith('d'))}"
        rule["parameter"] = name
        self.rules.append(rule)
        self.params.append((name, wc_value(distance)))
        self.state = written
        step["within"] = 1e-9
        step["result"] = "$any"
        step["expect"] = {"ids": IDS, "revision": "changed", "constraintKinds": [r["kind"] for r in self.rules],
                          "parameters": [{"name": n, "kind": "length", "expression": e} for n, e in self.params],
                          "entities": {str(k): written[k] for k in sorted(changed)}}
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
    c = case("Bu ölçüyü koru aksa: o yanda aks yoksa reddedilir")
    c.keep(6, "left", 1.2, to="grid")
    c = case("Aks bağı: güney duvarının ekseni A aksına 1,50 m ölçülür, hiçbir şey kımıldamaz")
    c.add(dist(G(13), W(1, "axis")))
    c = case("A aksı 50 cm güneye taşınınca bağlı güney duvarı izler, kapıları da")
    c.add(dist(G(13), W(1, "axis")))
    c.move_grid(13, 0.0, -0.5)
    c = case("Bağlı güney duvarı kaydırılınca A aksı da izler")
    c.add(dist(G(13), W(1, "axis")))
    c.shift(1, -0.5)
    c = case("Bağsız: aks taşınır, duvar yerinde")
    c.move_grid(13, 0.0, -0.5)
    c = case("Bu ölçüyü koru aksa: güney duvarı A aksına 2,00 m, bağ aynı adımda; aks taşınınca duvar izler")
    c.keep(1, "right", 2.0, to="grid")
    c.move_grid(13, 0.0, -0.5)
    c = case("Bu ölçüyü koru aksa, mesafe değişmeden: yalnız bağ yazılır")
    c.keep(1, "right", 1.5, to="grid")
    c = case("Sabit aks ve bağlı duvar: duvarı kaydırmak reddedilir")
    c.add(dist(G(13), W(1, "axis")), {"kind": "fix", "a": G(13)})
    c.shift(1, -0.5)
    c = case("Aks ve kuzey duvarı paralel: duvar kayar, aks yerinde")
    c.add({"kind": "parallel", "a": G(13), "b": W(3, "axis")})
    c.shift(3, 0.3)
    c = case("Bağlar zinciri aksla: aks taşınınca güney duvarı ve koridor duvarı izler")
    c.add(dist(G(13), W(1, "axis")), CORRIDOR)
    c.move_grid(13, 0.0, -0.5)
    c = case("Kolon yanı bağı: kolonun güney yanı güney duvarına 0,20 m; duvar dışa kayınca kolon izler")
    c.add(dist(W(1, "leftFace"), C(16, 0)))
    c.shift(1, -0.5)
    c = case("Kolon taşınınca bağlı güney duvarı izler")
    c.add(dist(W(1, "leftFace"), C(16, 0)))
    c.move_column(16, 0.0, 0.3)
    c = case("Bu ölçüyü koru kolonda: güney yanı güney duvarına 0,50 m, bağ aynı adımda; duvar kayınca kolon izler")
    c.keep_column(16, 0, 0.5)
    c.shift(1, -0.5)
    c = case("Bu ölçüyü koru kolonda, mesafe değişmeden: yalnız bağ yazılır")
    c.keep_column(16, 0, 0.2)
    c = case("Bu ölçüyü koru kolonda aksa: reddedilir")
    c.keep_column(16, 0, 1.5, to="grid")
    c = case("Akslara bağlı kolonun yanı batı duvarına: duvar batıya kayınca 1 aksı ve kolon izler")
    c.add(dist(W(4, "rightFace"), C(17, 1)))
    c.shift(4, -0.3)
    c = case("1 aksı taşınınca akslara bağlı kolon ve ona bağlı batı duvarı izler")
    c.add(dist(W(4, "rightFace"), C(17, 1)))
    c.move_grid(14, -0.3, 0.0)
    c = case("Yay bağı: eğri koridor 1,20 m ölçülür, hiçbir şey kımıldamaz")
    c.add(dist(W(18, "rightFace"), W(19, "leftFace")))
    c = case("Yay bağı: iç yay merkezine 0,3 m kayınca dış yay da 0,3 m izler, koridor 1,20 m kalır")
    c.add(dist(W(18, "rightFace"), W(19, "leftFace")))
    c.shift(18, 0.3)
    c = case("Yay bağı: dış yay 0,5 m dışa kayınca iç yay izler")
    c.add(dist(W(18, "rightFace"), W(19, "leftFace")))
    c.shift(19, -0.5)
    c = case("Bu ölçüyü koru yayda: eğri koridor 1,20 → 1,50 m, bağ aynı adımda; dış yay kayınca iç yay izler")
    c.keep(18, "right", 1.5)
    c.shift(19, -0.4)
    c = case("Yaylar eş merkezli (paralel) bağlı: dış yay kayar, iç yay yerinde")
    c.add({"kind": "parallel", "a": W(18, "axis"), "b": W(19, "axis")})
    c.shift(19, -0.3)
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
        refused("Yay kenar düz çizgiye bağlanmaz", dist(W(10, "axis"), W(12, "axis")),
                "bağlanan yay çizgileri eş merkezli olmalı; düz çizgi yaya bağlanmaz"),
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
        refused("Yay aksa bağ kurulmaz", dist(G(15), W(12, "axis")), "yay aks bağlanmaz; aks düz olmalı"),
        refused("Paralel olmayan aks ve duvar çizgisi arasına uzaklık kurulmaz", dist(G(14), W(1, "axis")),
                "bağlanan aks ve duvar çizgileri paralel olmalı"),
        refused("Aks çizgisi bir aksın olmalı", dist({"uid": u(1), "element": {"kind": "grid"}}, W(3, "axis")),
                "aks çizgisi yalnız bir aksta olur"),
        refused("Kolon yanı bir kolonun olmalı", dist(W(3, "axis"), {"uid": u(1), "element": {"kind": "columnSide", "side": 0}}),
                "kolon yanı yalnız bir kolonda olur"),
        refused("Paralel olmayan kolon yanı ve duvar çizgisi arasına uzaklık kurulmaz", dist(W(1, "axis"), C(16, 1)),
                "bağlanan kolon yanı ve duvar çizgileri paralel olmalı"),
        refused("Düz duvar çizgisi yaya bağlanmaz", dist(W(1, "axis"), W(18, "axis")),
                "bağlanan yay çizgileri eş merkezli olmalı; düz çizgi yaya bağlanmaz"),
        refused("Eş merkezli olmayan yaylar bağlanmaz", dist(W(18, "axis"), W(10, "axis")),
                "bağlanan yay çizgileri eş merkezli olmalı; düz çizgi yaya bağlanmaz"),
    ]


def document():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.constraints.edit", "commandVersion": 1,
            "title": "Mimari bağlar: duvar yüzü, eksen, aks, kolon yanı ve pervaz arasında kalıcı ölçü (ADR 0770, 8049)",
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
