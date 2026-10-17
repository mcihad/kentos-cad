#!/usr/bin/env python3
"""Shared cases of the constraint solver (docs/adr/0334), with expectations from an independent reference.

The reference knows no KentOS code: each constraint's equations are written here from the ADR's definitions with
sympy, the Jacobian is sympy's, and every number is mpmath's at 50 digits. For each case it

- takes the rows in the constraints' order and keeps a row while the rank grows (an exact SVD rank at 50 digits),
- solves the kept rows to 1e-40 by least-norm Newton steps from the drawing's shape,
- calls a dropped row redundant when it holds there and conflicting when it does not, a row with free unknowns
  and no gradient singular (or redundant when it holds), and a row with no free unknown redundant or conflicting,
- counts the freedoms left (free unknowns minus the rank) and the objects that can still move (an unknown is
  fixed when its unit vector lies in the rows' span),
- and, when every constraint holds and nothing can move, the solved shapes, which are then unique near the start.

A case whose constraints leave freedoms has no unique solution: the Rust test writes what it solved
(`KENTOS_CONSTRAINTS_OUT=FILE`), and `--outputs FILE` checks those shapes with this file's equations.

    python3 scripts/fixtures/constraint_cases.py          # writes fixtures/constraints/v1/cases.json
    python3 scripts/fixtures/constraint_cases.py --check  # recomputes and compares
    python3 scripts/fixtures/constraint_cases.py --outputs FILE
"""

import json
import math
import sys
from pathlib import Path

import mpmath as mp
import sympy as sp

mp.mp.dps = 50
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/constraints/v1/cases.json"
RANK = mp.mpf("1e-25")
HOLDS = mp.mpf("1e-20")


# ---- the unknowns ----------------------------------------------------------------------------------------------

def unknowns(shape):
    """An object's numbers in order, and which of them are arc angles."""
    kind, v = next(iter(shape.items()))
    if kind == "point":
        return [v[0], v[1]]
    if kind == "line":
        return [v[0][0], v[0][1], v[1][0], v[1][1]]
    if kind == "circle":
        return [v[0][0], v[0][1], v[1]]
    if kind == "arc":
        return [v[0][0], v[0][1], v[1], v[2], v[3]]
    if kind in ("polyline", "polygon"):
        return [c for p in v["pts"] for c in p]
    return []


def segment_of(shape, at, n):
    """The vertex indices (i, j) of a path's straight edge `at`, or None."""
    kind, v = next(iter(shape.items()))
    part, ring, i = at
    if part or ring:
        return None
    count = len(v["pts"])
    closed = kind == "polygon" or v.get("closed", False)
    if i + 1 < count:
        j = i + 1
    elif closed and i + 1 == count:
        j = 0
    else:
        return None
    bulges = v.get("bulges") or [0] * count
    if i >= count or abs(bulges[i]) > 1e-12:
        return None
    return i, j


def element(shapes, xs, ref):
    """An element as sympy expressions in its object's unknowns `xs[object]`: ('point', P), ('segment', A, B) or
    ('circle', C, r); None when the object has no such part."""
    obj, el = ref
    if obj >= len(shapes):
        return None
    shape = shapes[obj]
    kind, v = next(iter(shape.items()))
    x = xs[obj]
    if el == "circle":
        if kind in ("circle", "arc"):
            return ("circle", (x[0], x[1]), x[2])
        return None
    if "segment" in el:
        if kind == "line" and el["segment"] == [0, 0, 0]:
            return ("segment", (x[0], x[1]), (x[2], x[3]))
        if kind in ("polyline", "polygon"):
            ij = segment_of(shape, el["segment"], len(v["pts"]))
            if ij is None:
                return None
            i, j = ij
            return ("segment", (x[2 * i], x[2 * i + 1]), (x[2 * j], x[2 * j + 1]))
        return None
    if "vertex" in el:
        part, ring, i = el["vertex"]
        if kind in ("polyline", "polygon") and not part and not ring and i < len(v["pts"]):
            return ("point", (x[2 * i], x[2 * i + 1]))
        return None
    at = el["point"]
    if kind == "point" and at == "node":
        return ("point", (x[0], x[1]))
    if kind == "line" and at in ("start", "end"):
        return ("point", (x[0], x[1]) if at == "start" else (x[2], x[3]))
    if kind in ("circle", "arc") and at == "center":
        return ("point", (x[0], x[1]))
    if kind == "arc" and at in ("start", "end"):
        a = x[3] if at == "start" else x[4]
        return ("point", (x[0] + x[2] * sp.cos(a), x[1] + x[2] * sp.sin(a)))
    if kind == "polyline" and at in ("start", "end"):
        k = 0 if at == "start" else len(v["pts"]) - 1
        return ("point", (x[2 * k], x[2 * k + 1]))
    return None


def where(e):
    """A part's point: a point's own, a segment's start, a circle's centre."""
    return e[1]


def left_distance(a, b, p):
    """How far `p` lies left of the line from `a` to `b`."""
    dx, dy = b[0] - a[0], b[1] - a[1]
    return (dx * (p[1] - a[1]) - dy * (p[0] - a[0])) / sp.sqrt(dx * dx + dy * dy)


def angle_between(u, w):
    """The angle that turns `u` onto `w`, in (−π, π]."""
    return sp.atan2(u[0] * w[1] - u[1] * w[0], u[0] * w[0] + u[1] * w[1])


def rows_of(rule, parts, value, fixed):
    """A constraint's equations, zero when it holds."""
    kind = rule["rule"]
    if kind == "coincident":
        p, q = parts
        if q[0] == "point":
            return [p[1][0] - q[1][0], p[1][1] - q[1][1]]
        if q[0] == "segment":
            return [left_distance(q[1], q[2], p[1])]
        return [sp.sqrt((p[1][0] - q[1][0]) ** 2 + (p[1][1] - q[1][1]) ** 2) - q[2]]
    if kind in ("horizontal", "vertical"):
        k = 1 if kind == "horizontal" else 0
        if len(parts) == 1:
            return [parts[0][2][k] - parts[0][1][k]]
        return [where(parts[1])[k] - where(parts[0])[k]]
    if kind in ("parallel", "perpendicular"):
        (_, a0, a1), (_, b0, b1) = parts
        u = (a1[0] - a0[0], a1[1] - a0[1])
        w = (b1[0] - b0[0], b1[1] - b0[1])
        # The sine (or cosine) of the angle between them.
        lens = sp.sqrt(u[0] ** 2 + u[1] ** 2) * sp.sqrt(w[0] ** 2 + w[1] ** 2)
        if kind == "parallel":
            return [(u[0] * w[1] - u[1] * w[0]) / lens]
        return [(u[0] * w[0] + u[1] * w[1]) / lens]
    if kind == "tangent":
        a, b = parts
        if a[0] == "segment":
            d = left_distance(a[1], a[2], b[1])
            return [(d if rule.get("left") else -d) - b[2]]
        d = sp.sqrt((a[1][0] - b[1][0]) ** 2 + (a[1][1] - b[1][1]) ** 2)
        if rule.get("inside"):
            return [d - sp.Abs(a[2] - b[2])]
        return [d - a[2] - b[2]]
    if kind == "concentric":
        return [parts[0][1][0] - parts[1][1][0], parts[0][1][1] - parts[1][1][1]]
    if kind == "collinear":
        (_, a0, a1), (_, b0, b1) = parts
        return [left_distance(a0, a1, b0), left_distance(a0, a1, b1)]
    if kind == "equal":
        a, b = parts
        if a[0] == "circle":
            return [a[2] - b[2]]
        la = sp.sqrt((a[2][0] - a[1][0]) ** 2 + (a[2][1] - a[1][1]) ** 2)
        lb = sp.sqrt((b[2][0] - b[1][0]) ** 2 + (b[2][1] - b[1][1]) ** 2)
        return [la - lb]
    if kind == "symmetric":
        p, q, (_, s0, s1) = parts[0][1], parts[1][1], parts[2]
        mid = ((p[0] + q[0]) / 2, (p[1] + q[1]) / 2)
        d = (s1[0] - s0[0], s1[1] - s0[1])
        # The middle on the axis, the joining line square to it.
        return [left_distance(s0, s1, mid), d[0] * (q[0] - p[0]) + d[1] * (q[1] - p[1])]
    if kind == "fix":
        now, then = parts[0], fixed[0]
        if now[0] == "point":
            return [now[1][0] - then[1][0], now[1][1] - then[1][1]]
        if now[0] == "segment":
            return [now[1][0] - then[1][0], now[1][1] - then[1][1], now[2][0] - then[2][0], now[2][1] - then[2][1]]
        return [now[1][0] - then[1][0], now[1][1] - then[1][1], now[2] - then[2]]
    if kind == "distance":
        p, q = parts
        if q[0] == "segment":
            d = left_distance(q[1], q[2], p[1])
            return [(d if rule.get("left") else -d) - value]
        a, b = where(p), where(q)
        axis = rule.get("axis", "aligned")
        if axis == "horizontal":
            return [b[0] - a[0] - value]
        if axis == "vertical":
            return [b[1] - a[1] - value]
        return [sp.sqrt((b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2) - value]
    if kind == "angle":
        (_, a0, a1), (_, b0, b1) = parts
        u = (a1[0] - a0[0], a1[1] - a0[1])
        w = (b1[0] - b0[0], b1[1] - b0[1])
        if rule.get("reverseA"):
            u = (-u[0], -u[1])
        if rule.get("reverseB"):
            w = (-w[0], -w[1])
        # `u` turned by the value lies along `w`.
        c, s = sp.cos(value), sp.sin(value)
        turned = (u[0] * c - u[1] * s, u[0] * s + u[1] * c)
        return [angle_between(turned, w)]
    if kind == "radius":
        return [parts[0][2] - value]
    if kind == "diameter":
        return [2 * parts[0][2] - value]
    raise ValueError(kind)


def measure_of(rule, parts):
    kind = rule["rule"]
    if kind == "angle":
        (_, a0, a1), (_, b0, b1) = parts
        u = (a1[0] - a0[0], a1[1] - a0[1])
        w = (b1[0] - b0[0], b1[1] - b0[1])
        if rule.get("reverseA"):
            u = (-u[0], -u[1])
        if rule.get("reverseB"):
            w = (-w[0], -w[1])
        a = angle_between(u, w)
        return a + 2 * sp.pi if a < 0 else a
    return rows_of(rule, parts, 0, None)[0]


def refs_of(rule):
    out = [rule["a"]]
    if "b" in rule:
        out.append(rule["b"])
    if "axis" in rule and isinstance(rule["axis"], list):
        out.append(rule["axis"])
    return out


# ---- the solve ---------------------------------------------------------------------------------------------------

def reference(case):
    shapes = case["shapes"]
    held = case.get("held", [])
    fix_from = case.get("fixFrom") or shapes
    rules = case["constraints"]
    starts = [[mp.mpf(repr(float(v))) for v in unknowns(s)] for s in shapes]
    symbols = [[sp.Symbol(f"x{i}_{k}", real=True) for k in range(len(u))] for i, u in enumerate(starts)]
    # The unknowns held: an object's every number or a part's.
    held_syms = set()
    for obj, el in held:
        if el is None:
            held_syms.update(symbols[obj])
        else:
            e = element(shapes, symbols, (obj, el))
            for expr in (e[1:] if e else []):
                for c in (expr if isinstance(expr, tuple) else (expr,)):
                    held_syms.update(sp.sympify(c).free_symbols)
    fixed_values = {s: v for ss, vv in zip(symbols, [[mp.mpf(repr(float(v))) for v in unknowns(s)] for s in fix_from]) for s, v in zip(ss, vv)}

    broken, rows, reference_rules, used = [], [], [], set()
    for k, rule in enumerate(rules):
        parts = [element(shapes, symbols, r) for r in refs_of(rule)]
        if any(p is None for p in parts) or not math.isfinite(rule.get("value", 0.0)):
            broken.append(k)
            continue
        for r in refs_of(rule):
            used.add(r[0])
        if rule.get("reference"):
            reference_rules.append((k, rule, parts))
            continue
        fixed = None
        if rule["rule"] == "fix":
            fixed = [subs_part(p, fixed_values) for p in parts]
        value = sp.Float(repr(float(rule.get("value", 0.0))), 50)
        for expr in rows_of(rule, parts, value, fixed):
            rows.append((k, sp.sympify(expr)))
    # Every used object's free unknowns, in order.
    free = [s for i in sorted(used) for s in symbols[i] if s not in held_syms]
    point = {s: v for ss, vv in zip(symbols, starts) for s, v in zip(ss, vv)}
    jac = [[sp.diff(expr, s) for s in free] for _, expr in rows]
    lamb_r = [sp.lambdify(list(point), expr, "mpmath") for _, expr in rows]
    lamb_j = [[sp.lambdify(list(point), d, "mpmath") for d in row] for row in jac]
    names = list(point)

    def safe(f, args):
        # A part that lost its meaning (a segment of no length) gives no number.
        try:
            v = f(*args)
        except ZeroDivisionError:
            return mp.nan
        return mp.mpf(v) if mp.isfinite(v) else mp.nan

    def evaluate(at):
        args = [at[n] for n in names]
        res = [safe(f, args) for f in lamb_r]
        jj = mp.matrix(len(rows), len(free)) if rows and free else None
        for i, row in enumerate(lamb_j):
            for j, f in enumerate(row):
                jj[i, j] = safe(f, args)
        return res, jj

    def independent(res, jj):
        """The rows kept in order while the rank grows; the dropped rows and the rows with no gradient."""
        kept, dropped, zero, basis = [], [], [], []
        for i in range(len(rows)):
            row = [jj[i, j] for j in range(len(free))] if jj is not None else []
            own = mp.sqrt(sum(v * v for v in row)) if row else mp.mpf(0)
            if not mp.isfinite(own) or own == 0:
                zero.append(i)
                continue
            v = row[:]
            for _ in range(2):
                for q in basis:
                    c = sum(a * b for a, b in zip(v, q))
                    v = [a - c * b for a, b in zip(v, q)]
            left = mp.sqrt(sum(a * a for a in v))
            if left <= RANK * own:
                dropped.append(i)
                continue
            basis.append([a / left for a in v])
            kept.append(i)
        return kept, dropped, zero, basis

    at = dict(point)
    res, jj = evaluate(at)
    kept, _, _, _ = independent(res, jj)
    for _ in range(200):
        res, jj = evaluate(at)
        if all(abs(res[i]) < mp.mpf("1e-40") for i in kept):
            break
        # The least-norm step of the kept rows: dx = Jᵀ (J Jᵀ)⁻¹ (−r).
        jk = mp.matrix([[jj[i, j] for j in range(len(free))] for i in kept])
        rk = mp.matrix([-res[i] for i in kept])
        y = mp.lu_solve(jk * jk.T, rk)
        dx = jk.T * y
        for j, s in enumerate(free):
            at[s] += dx[j]
    res, jj = evaluate(at)
    kept, dropped, zero, basis = independent(res, jj)
    kept_hold = all(abs(res[i]) < HOLDS for i in kept)
    redundant, conflicting, singular = set(), set(), set()
    for i in dropped:
        (redundant if abs(res[i]) < HOLDS else conflicting).add(rows[i][0])
    for i in zero:
        k, expr = rows[i]
        has_free = any(s in expr.free_symbols for s in free)
        ok = mp.isfinite(res[i]) and abs(res[i]) < HOLDS
        if has_free:
            (redundant if ok else singular).add(k)
        else:
            (redundant if ok else conflicting).add(k)
    all_hold = kept_hold and all(mp.isfinite(r) and abs(r) < HOLDS for r in res)
    if all_hold:
        conflicting.clear()
    rank = len(basis)
    dof = len(free) - rank
    if conflicting:
        status = "inconsistent"
    elif singular:
        status = "singular"
    elif not all_hold:
        status = "nonConvergent"
    elif redundant:
        status = "over"
    elif dof == 0:
        status = "fully"
    else:
        status = "under"
    # The objects with an unknown not fixed by the rows.
    free_objects = set()
    for j, s in enumerate(free):
        along = sum(q[j] ** 2 for q in basis)
        if along < 1 - mp.mpf("1e-20"):
            free_objects.add(owner_of(symbols, s))
    holds = status in ("fully", "under", "over")
    final = at if holds else point
    measured = []
    for k, rule, parts in reference_rules:
        expr = sp.sympify(measure_of(rule, [subs_part(p, final) for p in parts]))
        measured.append([k, float(mp.mpf(sp.N(expr, 50)))])
    expect = {
        "status": status,
        "dof": dof,
        "redundant": sorted(redundant),
        "conflicting": sorted(conflicting),
        "singular": sorted(singular),
        "broken": sorted(broken),
        "free": sorted(free_objects),
        "measured": measured,
    }
    if status == "fully" or not holds:
        expect["shapes"] = [write_shape(s, [final[x] for x in symbols[i]]) for i, s in enumerate(shapes)]
    return expect


def owner_of(symbols, s):
    return next(i for i, ss in enumerate(symbols) if s in ss)


def subs_part(part, values):
    def sub(e):
        if isinstance(e, tuple):
            return tuple(sub(c) for c in e)
        return sp.Float(mp.nstr(sp.sympify(e).xreplace({k: sp.Float(mp.nstr(v, 50), 50) for k, v in values.items()}).evalf(50), 50), 50) if sp.sympify(e).free_symbols else sp.sympify(e)

    return (part[0],) + tuple(sub(c) for c in part[1:])


def write_shape(shape, values):
    kind, v = next(iter(shape.items()))
    f = [float(x) for x in values]
    if kind == "point":
        return {"point": f}
    if kind == "line":
        return {"line": [f[0:2], f[2:4]]}
    if kind == "circle":
        return {"circle": [f[0:2], f[2]]}
    if kind == "arc":
        return {"arc": [f[0:2], f[2], f[3], f[4]]}
    out = dict(v)
    out["pts"] = [f[2 * i:2 * i + 2] for i in range(len(v["pts"]))]
    return {kind: out}


def residuals(case, shapes):
    """The largest residual of the case's constraints on `shapes` (the Rust solve's)."""
    worst = 0.0
    xs = [[sp.Float(repr(float(v)), 50) for v in unknowns(s)] for s in shapes]
    fix_xs = [[sp.Float(repr(float(v)), 50) for v in unknowns(s)] for s in (case.get("fixFrom") or case["shapes"])]
    for rule in case["constraints"]:
        if rule.get("reference"):
            continue
        parts = [element(shapes, xs, r) for r in refs_of(rule)]
        if any(p is None for p in parts):
            continue
        fixed = [element(shapes, fix_xs, r) for r in refs_of(rule)] if rule["rule"] == "fix" else None
        for e in rows_of(rule, parts, sp.Float(repr(float(rule.get("value", 0.0))), 50), fixed):
            worst = max(worst, abs(float(sp.N(e, 30))))
    return worst


# ---- the cases ---------------------------------------------------------------------------------------------------

def P(name):
    return {"point": name}


SEG = {"segment": [0, 0, 0]}
START, END, CENTER, NODE = P("start"), P("end"), P("center"), P("node")


def ln(ax, ay, bx, by):
    return {"line": [[ax, ay], [bx, by]]}


def cc(x, y, r):
    return {"circle": [[x, y], r]}


def rect_shapes(w=6.0, h=4.0):
    return [ln(0, 0, w, 0), ln(w, 0, w, h), ln(w, h, 0, h), ln(0, h, 0, 0)]


def rect_rules(width=6.0, height=4.0, square=True, dims=True):
    out = [{"rule": "coincident", "a": [i, END], "b": [(i + 1) % 4, START]} for i in range(4)]
    out += [
        {"rule": "horizontal", "a": [0, SEG]},
        {"rule": "parallel", "a": [0, SEG], "b": [2, SEG]},
        {"rule": "parallel", "a": [1, SEG], "b": [3, SEG]},
    ]
    if square:
        out.append({"rule": "perpendicular", "a": [0, SEG], "b": [1, SEG]})
    out.append({"rule": "fix", "a": [0, START]})
    if dims:
        out.append({"rule": "distance", "a": [0, START], "b": [0, END], "value": width})
        out.append({"rule": "distance", "a": [1, START], "b": [1, END], "value": height})
    return out


def cases():
    out = []

    def add(name, shapes, constraints, **extra):
        out.append({"name": name, "shapes": shapes, "constraints": constraints, **extra})

    add("Dikdörtgen genişler: karşı kenarlar paralel, köşeler dik", rect_shapes(), rect_rules(8.5))
    add("Dikdörtgen daralır", rect_shapes(), rect_rules(3.25, 5.5))
    add("Çelişkili genişlik: hiçbir şey yazılmaz", rect_shapes(),
        rect_rules(6.0) + [{"rule": "distance", "a": [2, START], "b": [2, END], "value": 7.0}])
    add("Aynı genişlik iki kez: fazla", rect_shapes(),
        rect_rules(6.0) + [{"rule": "distance", "a": [2, START], "b": [2, END], "value": 6.0}])
    add("Ölçüsüz dikdörtgen: iki serbestlik", rect_shapes(), rect_rules(dims=False))
    add("Dik açısız paralelkenar", [ln(0, 0, 6, 0), ln(6, 0, 7, 4), ln(7, 4, 1, 4), ln(1, 4, 0, 0)],
        rect_rules(square=False, dims=False))
    add("Doğruya teğet daire ve yarıçapı", [ln(0, 0, 10, 0), cc(5, 3, 2)], [
        {"rule": "fix", "a": [0, SEG]},
        {"rule": "tangent", "a": [0, SEG], "b": [1, "circle"], "left": True},
        {"rule": "radius", "a": [1, "circle"], "value": 2.5},
    ])
    add("Doğrunun sağına teğet daire, merkezi sabit doğu", [ln(0, 0, 10, 0), cc(4, -1, 1.5)], [
        {"rule": "fix", "a": [0, SEG]},
        {"rule": "tangent", "a": [0, SEG], "b": [1, "circle"], "left": False},
        {"rule": "distance", "a": [0, START], "b": [1, CENTER], "axis": "horizontal", "value": 4.0},
        {"rule": "diameter", "a": [1, "circle"], "value": 2.0},
    ])
    # Teğet ile değme noktasındaki çakışık birlikte çift kök verir (docs/adr/0334 §3): yayın ucunu doğruya teğet
    # oturtmak için merkez ucun düşeyinde tutulur.
    add("Yayın başı doğrunun ucunda, merkezi düşeyinde", [ln(0, 0, 4, 0), {"arc": [[4.3, 2], 2, -1.4, 1.0]}], [
        {"rule": "fix", "a": [0, SEG]},
        {"rule": "coincident", "a": [1, START], "b": [0, END]},
        {"rule": "vertical", "a": [0, END], "b": [1, CENTER]},
        {"rule": "radius", "a": [1, "circle"], "value": 3.0},
        {"rule": "fix", "a": [1, END]},
    ], fixFrom=[ln(0, 0, 4, 0), {"arc": [[4, 3], 3, -math.pi / 2, 0.4]}])
    add("Eş merkezli iki daire, çap ve eşit", [cc(0, 0, 2), cc(0.3, -0.2, 1.1), cc(5, 5, 1)], [
        {"rule": "fix", "a": [0, CENTER]},
        {"rule": "concentric", "a": [0, "circle"], "b": [1, "circle"]},
        {"rule": "diameter", "a": [0, "circle"], "value": 5.0},
        {"rule": "radius", "a": [1, "circle"], "value": 1.25},
        {"rule": "equal", "a": [1, "circle"], "b": [2, "circle"]},
    ])
    add("Dıştan teğet iki daire", [cc(0, 0, 2), cc(5, 1, 1)], [
        {"rule": "fix", "a": [0, "circle"]},
        {"rule": "tangent", "a": [0, "circle"], "b": [1, "circle"]},
        {"rule": "radius", "a": [1, "circle"], "value": 1.0},
        {"rule": "horizontal", "a": [0, CENTER], "b": [1, CENTER]},
    ])
    add("İçten teğet iki daire", [cc(0, 0, 4), cc(1.5, 0.5, 1)], [
        {"rule": "fix", "a": [0, "circle"]},
        {"rule": "tangent", "a": [0, "circle"], "b": [1, "circle"], "inside": True},
        {"rule": "radius", "a": [1, "circle"], "value": 1.5},
        {"rule": "vertical", "a": [0, CENTER], "b": [1, CENTER]},
    ])
    add("Doğrudaş iki çizgi", [ln(0, 0, 5, 1), ln(7, 1.2, 10, 2.5)], [
        {"rule": "fix", "a": [0, SEG]},
        {"rule": "collinear", "a": [0, SEG], "b": [1, SEG]},
        {"rule": "distance", "a": [1, START], "b": [1, END], "value": 3.0},
        {"rule": "distance", "a": [0, END], "b": [1, START], "value": 2.0},
    ])
    add("Eksene göre simetrik iki nokta", [{"point": [1, 2]}, {"point": [4, 1]}, ln(2, -3, 3, 5)], [
        {"rule": "fix", "a": [2, SEG]},
        {"rule": "fix", "a": [0, NODE]},
        {"rule": "symmetric", "a": [0, NODE], "b": [1, NODE], "axis": [2, SEG]},
    ])
    add("Yatay ve düşey nokta çiftleri", [{"point": [0, 0]}, {"point": [3, 0.4]}, {"point": [2.6, 2]}], [
        {"rule": "fix", "a": [0, NODE]},
        {"rule": "horizontal", "a": [0, NODE], "b": [1, NODE]},
        {"rule": "vertical", "a": [1, NODE], "b": [2, NODE]},
        {"rule": "distance", "a": [0, NODE], "b": [1, NODE], "axis": "horizontal", "value": 3.5},
        {"rule": "distance", "a": [1, NODE], "b": [2, NODE], "axis": "vertical", "value": -1.5},
    ])
    add("Noktanın kenara uzaklığı", [ln(0, 0, 8, 2), {"point": [3, 4]}], [
        {"rule": "fix", "a": [0, SEG]},
        {"rule": "distance", "a": [1, NODE], "b": [0, SEG], "left": True, "value": 2.0},
        {"rule": "distance", "a": [0, START], "b": [1, NODE], "value": 5.0},
    ])
    add("İki çizgi arasında 30 derece", [ln(0, 0, 5, 0), ln(0, 0, 4, 3)], [
        {"rule": "fix", "a": [0, SEG]},
        {"rule": "coincident", "a": [1, START], "b": [0, START]},
        {"rule": "angle", "a": [0, SEG], "b": [1, SEG], "value": math.radians(30)},
        {"rule": "distance", "a": [1, START], "b": [1, END], "value": 5.0},
    ])
    add("Ters yönlerle açı", [ln(0, 0, 5, 0), ln(4, 3, 0, 0)], [
        {"rule": "fix", "a": [0, SEG]},
        {"rule": "coincident", "a": [1, END], "b": [0, START]},
        {"rule": "angle", "a": [0, SEG], "b": [1, SEG], "reverseB": True, "value": math.radians(50)},
        {"rule": "distance", "a": [1, START], "b": [1, END], "value": 5.0},
    ])
    add("Açı ile diklik çelişir", [ln(0, 0, 5, 0), ln(0, 0, 4, 3)], [
        {"rule": "fix", "a": [0, SEG]},
        {"rule": "coincident", "a": [1, START], "b": [0, START]},
        {"rule": "perpendicular", "a": [0, SEG], "b": [1, SEG]},
        {"rule": "angle", "a": [0, SEG], "b": [1, SEG], "value": math.radians(60)},
    ])
    add("Yarıçap ile çap çelişir", [cc(0, 0, 2)], [
        {"rule": "radius", "a": [0, "circle"], "value": 2.0},
        {"rule": "diameter", "a": [0, "circle"], "value": 5.0},
    ])
    square = {"polygon": {"pts": [[0, 0], [5, 0.3], [5.2, 5], [0.1, 4.8]]}}
    edge = lambda i: {"segment": [0, 0, i]}
    add("Kapalı alan kareye döner", [square], [
        {"rule": "fix", "a": [0, {"vertex": [0, 0, 0]}]},
        {"rule": "horizontal", "a": [0, edge(0)]},
        {"rule": "perpendicular", "a": [0, edge(0)], "b": [0, edge(1)]},
        {"rule": "perpendicular", "a": [0, edge(1)], "b": [0, edge(2)]},
        {"rule": "perpendicular", "a": [0, edge(2)], "b": [0, edge(3)]},
        {"rule": "equal", "a": [0, edge(0)], "b": [0, edge(1)]},
        {"rule": "distance", "a": [0, {"vertex": [0, 0, 0]}], "b": [0, {"vertex": [0, 0, 1]}], "value": 5.0},
    ])
    bulged = {"polyline": {"pts": [[0, 0], [4, 0], [6, 2]], "bulges": [0, 0.4, 0], "closed": False}}
    add("Yaylı kenar kısıt almaz", [bulged, ln(0, 1, 4, 1)], [
        {"rule": "horizontal", "a": [0, edge(0)]},
        {"rule": "horizontal", "a": [0, edge(1)]},
        {"rule": "parallel", "a": [0, edge(0)], "b": [1, SEG]},
        {"rule": "coincident", "a": [1, START], "b": [0, START]},
    ])
    add("Olmayan nesne ve parça", [ln(0, 0, 1, 0.2), cc(0, 0, 1)], [
        {"rule": "horizontal", "a": [0, SEG]},
        {"rule": "horizontal", "a": [1, SEG]},
        {"rule": "fix", "a": [5, START]},
        {"rule": "radius", "a": [0, "circle"], "value": 1.0},
    ])
    widened = rect_shapes()
    widened[0] = ln(0, 0, 9, 0)
    add("Tutamaç sürüklenir: genişliği tutamak verir", widened,
        [r for r in rect_rules(6.0) if not (r["rule"] == "distance" and r["a"][0] == 0)],
        held=[[0, END]], fixFrom=rect_shapes())
    add("Sabit daire yerinden çekilir", [cc(1, 1, 2), ln(0, 0, 3, 0)], [
        {"rule": "fix", "a": [0, "circle"]},
        {"rule": "tangent", "a": [1, SEG], "b": [0, "circle"], "left": True},
        {"rule": "fix", "a": [1, START]},
    ], fixFrom=[cc(1, 3, 2), ln(0, 0, 3, 0)])
    add("Sıfır boylu çizgiye paralel: tekil", [ln(1, 1, 1, 1), ln(0, 0, 4, 1)], [
        {"rule": "fix", "a": [0, SEG]},
        {"rule": "parallel", "a": [0, SEG], "b": [1, SEG]},
    ])
    add("Nokta daire üstünde", [cc(2, 2, 3), {"point": [6, 6]}], [
        {"rule": "fix", "a": [0, "circle"]},
        {"rule": "coincident", "a": [1, NODE], "b": [0, "circle"]},
        {"rule": "horizontal", "a": [0, CENTER], "b": [1, NODE]},
    ])
    add("Nokta kenar üstünde", [ln(0, 0, 10, 5), {"point": [3, 4]}], [
        {"rule": "fix", "a": [0, SEG]},
        {"rule": "coincident", "a": [1, NODE], "b": [0, SEG]},
        {"rule": "distance", "a": [0, START], "b": [1, NODE], "axis": "horizontal", "value": 4.0},
    ])
    add("Ölçen boyut genişliği okur", rect_shapes(), rect_rules(7.25) + [
        {"rule": "distance", "a": [2, START], "b": [2, END], "reference": True},
        {"rule": "angle", "a": [0, SEG], "b": [3, SEG], "reference": True},
    ])
    add("Çizgi ile kapalı alanın kenarı eşit", [ln(0, 0, 3, 0), {"polygon": {"pts": [[5, 0], [9, 0], [7, 3]]}}], [
        {"rule": "fix", "a": [1, {"vertex": [0, 0, 0]}]},
        {"rule": "fix", "a": [1, {"vertex": [0, 0, 1]}]},
        {"rule": "fix", "a": [1, {"vertex": [0, 0, 2]}]},
        {"rule": "fix", "a": [0, START]},
        {"rule": "horizontal", "a": [0, SEG]},
        {"rule": "equal", "a": [0, SEG], "b": [1, edge(1)]},
    ])
    add("Uzak koordinatlarda dikdörtgen", [
        ln(512000, 4420000, 512006, 4420000), ln(512006, 4420000, 512006, 4420004),
        ln(512006, 4420004, 512000, 4420004), ln(512000, 4420004, 512000, 4420000),
    ], rect_rules(6.125))
    add("Tutulan nesneye kısıt yalnız denetlenir", [ln(0, 0, 4, 1), ln(0, 2, 4, 2)], [
        {"rule": "parallel", "a": [0, SEG], "b": [1, SEG]},
        {"rule": "horizontal", "a": [1, SEG]},
    ], held=[[0, None], [1, None]])
    add("Çoklu çizginin ucu ve köşesi", [{"polyline": {"pts": [[0, 0], [3, 1], [6, 0]], "closed": False}}], [
        {"rule": "fix", "a": [0, START]},
        {"rule": "horizontal", "a": [0, START], "b": [0, END]},
        {"rule": "distance", "a": [0, START], "b": [0, END], "value": 8.0},
        {"rule": "equal", "a": [0, edge(0)], "b": [0, edge(1)]},
        {"rule": "distance", "a": [0, START], "b": [0, {"vertex": [0, 0, 1]}], "axis": "vertical", "value": 2.0},
    ])
    return out


def build():
    out = []
    for case in cases():
        case = dict(case)
        case["expect"] = reference(case)
        out.append(case)
    return out


def main():
    if len(sys.argv) > 2 and sys.argv[1] == "--outputs":
        data = json.loads(Path(sys.argv[2]).read_text())
        by_name = {c["name"]: c for c in cases()}
        bad = 0
        for item in data:
            case = by_name[item["name"]]
            if item["status"] not in ("fully", "under", "over"):
                continue
            worst = residuals(case, item["shapes"])
            if worst > 1e-9:
                print(f"{item['name']}: artık {worst:.3g}")
                bad += 1
        print(f"{len(data)} çözüm denetlendi, {bad} sorun")
        sys.exit(1 if bad else 0)
    built = build()
    text = json.dumps({"version": 1, "cases": built}, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            print(f"{OUT} güncel değil: python3 {sys.argv[0]}")
            sys.exit(1)
        print(f"{len(built)} durum güncel")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    for c in built:
        e = c["expect"]
        print(f"{c['name']}: {e['status']} dof {e['dof']} fazla {e['redundant']} çelişen {e['conflicting']} tekil {e['singular']} kopuk {e['broken']}")


if __name__ == "__main__":
    main()
