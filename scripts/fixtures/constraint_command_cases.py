#!/usr/bin/env python3
"""The shared cases of the constraint commands cad.constraints.edit and cad.constraints.query (docs/adr/0335).

    python3 scripts/fixtures/constraint_command_cases.py           # writes the files
    python3 scripts/fixtures/constraint_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.constraints.edit.json and cad.constraints.query.json. The command's rules are written
here from the ADR, without KentOS code: the checks and their order, a new constraint's sense read from the drawing
(a tangent's side, a distance's side of its segment, the angle under 180°, a horizontal or vertical distance from the
lower to the higher), a dimension's parameter (the next free name d1, ang1, rad1, dia1; its value an expression in the
drawing's unit and the project's angle unit, or the value the drawing measures written with ten decimals at most), the
solve's diagnosis and the shapes, from the solver's independent reference (constraint_cases.py: the equations in
sympy, ranks and solutions in mpmath at 50 digits), and a solution that turns a shape inside out refused.

A case's solved shapes are expected only where the solution is unique (no freedom left) or the constraints are
linear in the unknowns they move (one least-change step reaches it exactly); elsewhere only what the command says.
"""
import json
import math
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import constraint_cases as cc  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
EDIT = ROOT / "fixtures/commands/v1/cad.constraints.edit.json"
QUERY = ROOT / "fixtures/commands/v1/cad.constraints.query.json"

style = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}


def layer(i, name, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True, "style": style, "children": []}


def P(x, y):
    return {"x": x, "y": y}


def ln(i, a, b, layer_id="cizim"):
    return {"kind": "line", "id": i, "layerId": layer_id, "attrs": {}, "a": P(*a), "b": P(*b)}


ENTITIES = [
    ln(1, (0, 0), (6, 0.3)),
    ln(2, (6, 0.3), (6.2, 4)),
    ln(3, (6.2, 4), (0, 4.1)),
    ln(4, (0, 4.1), (0, 0)),
    {"kind": "circle", "id": 5, "layerId": "cizim", "attrs": {}, "c": P(10, 3), "r": 1.5},
    ln(6, (8, 0), (14, 0)),
    {"kind": "arc", "id": 7, "layerId": "cizim", "attrs": {}, "c": P(3, 9), "r": 2, "a0": 0, "a1": 1.5},
    ln(8, (0, -5), (6, -4.5), "kilitli"),
    {"kind": "polygon", "id": 9, "layerId": "cizim", "attrs": {}, "pts": [P(20, 0), P(24, 0), P(24, 3), P(20, 3)], "bulges": [0, 0.3, 0, 0]},
    {"kind": "point", "id": 10, "layerId": "cizim", "attrs": {}, "p": P(2, 6)},
    {"kind": "point", "id": 11, "layerId": "cizim", "attrs": {}, "p": P(5, 6.5)},
]

SETUP = {
    "format": "kentos.document", "version": 1, "name": "Kısıtlar",
    "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 100,
                 "workspace": "cad"},
    "origin": P(0, 0),
    "layers": [layer("cizim", "Çizim"), layer("kilitli", "Kilitli", locked=True)],
    "activeLayer": "cizim",
    "entities": ENTITIES,
    "styles": {"items": [], "categories": []},
}

RAD_PER_DEG = math.pi / 180
UUID = re.compile(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}")


# ---- the drawing as the reference reads it -------------------------------------------------------------------------

def shape(e):
    """An object as constraint_cases' shape."""
    k = e["kind"]
    if k == "line":
        return {"line": [[e["a"]["x"], e["a"]["y"]], [e["b"]["x"], e["b"]["y"]]]}
    if k == "circle":
        return {"circle": [[e["c"]["x"], e["c"]["y"]], e["r"]]}
    if k == "arc":
        return {"arc": [[e["c"]["x"], e["c"]["y"]], e["r"], e["a0"], e["a1"]]}
    if k == "point":
        return {"point": [e["p"]["x"], e["p"]["y"]]}
    if k in ("polygon", "polyline"):
        out = {"pts": [[p["x"], p["y"]] for p in e["pts"]]}
        if e.get("bulges"):
            out["bulges"] = e["bulges"]
        if k == "polyline":
            out["closed"] = e.get("closed", False)
        return {k: out}
    raise ValueError(k)


def entity(e, s):
    """The object `e` with the solved shape `s`."""
    e = dict(e)
    kind, v = next(iter(s.items()))
    if kind == "line":
        e["a"], e["b"] = P(*v[0]), P(*v[1])
    elif kind == "circle":
        e["c"], e["r"] = P(*v[0]), v[1]
    elif kind == "arc":
        e["c"], e["r"], e["a0"], e["a1"] = P(*v[0]), v[1], v[2], v[3]
    elif kind == "point":
        e["p"] = P(*v)
    else:
        e["pts"] = [P(*q) for q in v["pts"]]
    return e


def element_json(el):
    """A contract element as constraint_cases' element."""
    k = el["kind"]
    if k == "circle":
        return "circle"
    if k == "segment":
        return {"segment": [el["part"], el["ring"], el["index"]]}
    place = el["place"]
    if place["kind"] == "vertex":
        return {"vertex": [place["part"], place["ring"], place["index"]]}
    return {"point": {"start": "start", "end": "end", "center": "center", "node": "node"}[place["kind"]]}


# ---- the parameters' expressions (the ADR's grammar, the part the cases use) ---------------------------------------

UNITS = {"m": 1.0, "cm": 0.01, "mm": 0.001, "km": 1000.0, "deg": RAD_PER_DEG, "rad": 1.0}


def evaluate(text, values, kind):
    """A parameter's value in its own unit: numbers with a unit, names, + - * / and parentheses."""
    tokens, i = [], 0
    while i < len(text):
        c = text[i]
        if c.isspace():
            i += 1
        elif c.isdigit() or c == ".":
            j = i
            while j < len(text) and (text[j].isdigit() or text[j] == "."):
                j += 1
            k = j
            while k < len(text) and text[k].isalpha():
                k += 1
            tokens.append(("num", float(text[i:j]), text[j:k] or None))
            i = k
        elif c.isalpha() or c == "_":
            j = i
            while j < len(text) and (text[j].isalnum() or text[j] == "_"):
                j += 1
            tokens.append(("name", text[i:j]))
            i = j
        else:
            tokens.append(("op", c))
            i += 1
    pos = [0]

    def peek():
        return tokens[pos[0]] if pos[0] < len(tokens) else None

    def take():
        pos[0] += 1
        return tokens[pos[0] - 1]

    def atom():
        t = take()
        if t[0] == "num":
            v, unit = t[1], t[2]
            if unit is None:
                return v
            # A unit on a number: in the drawing's metres or the project's degrees.
            base = UNITS[unit]
            return v * base / (RAD_PER_DEG if unit in ("deg", "rad") else 1.0)
        if t[0] == "name":
            return values[t[1]]
        if t == ("op", "("):
            v = expr()
            take()
            return v
        if t == ("op", "-"):
            return -atom()
        raise ValueError(text)

    def term():
        v = atom()
        while peek() in (("op", "*"), ("op", "/")):
            op = take()[1]
            w = atom()
            v = v * w if op == "*" else v / w
        return v

    def expr():
        v = term()
        while peek() in (("op", "+"), ("op", "-")):
            op = take()[1]
            w = term()
            v = v + w if op == "+" else v - w
        return v

    return expr()


FUNCTIONS = {"sin", "cos", "tan", "asin", "acos", "atan", "sqrt", "abs", "round", "floor", "ceil", "min", "max", "pi"}


def names_in(text):
    """The parameter names an expression reads: its words, but not a number's unit nor a function."""
    out, i = [], 0
    while i < len(text):
        ch = text[i]
        if ch.isalpha() or ch == "_":
            j = i
            while j < len(text) and (text[j].isalnum() or text[j] == "_"):
                j += 1
            word = text[i:j]
            if not (i > 0 and (text[i - 1].isdigit() or text[i - 1] == ".")) and word not in FUNCTIONS:
                out.append(word)
            i = j
        else:
            i += 1
    return out


class AnyValue(dict):
    """Every name read as one: an expression checked for its form only."""

    def __missing__(self, key):
        return 1.0


def value_text(v):
    t = f"{v:.12f}".rstrip("0").rstrip(".")
    return "0" if t == "-0" else t


# ---- the command, as the ADR says it --------------------------------------------------------------------------------

POINT_KINDS = ("start", "end", "center", "node", "vertex")


def is_point(el):
    return el["element"]["kind"] == "point"


def is_seg(el):
    return el["element"]["kind"] == "segment"


def is_circle(el):
    return el["element"]["kind"] == "circle"


def fits(rule):
    k = rule["kind"]
    a, b = rule.get("a"), rule.get("b")
    if k == "coincident":
        return is_point(a)
    if k in ("horizontal", "vertical"):
        return is_seg(a) if b is None else is_point(a) and is_point(b)
    if k in ("parallel", "perpendicular", "collinear", "angle"):
        return is_seg(a) and is_seg(b)
    if k == "tangent":
        return (is_seg(a) or is_circle(a)) and is_circle(b)
    if k == "concentric":
        return is_circle(a) and is_circle(b)
    if k == "equal":
        return (is_seg(a) and is_seg(b)) or (is_circle(a) and is_circle(b))
    if k == "symmetric":
        return is_point(a) and is_point(b) and is_seg(rule["axis"])
    if k == "fix":
        return True
    if k == "distance":
        return is_point(a) and (is_point(b) or (is_seg(b) and rule.get("axis", "aligned") == "aligned"))
    return is_circle(a)


LABELS = {"coincident": "Çakışık", "horizontal": "Yatay", "vertical": "Düşey", "parallel": "Paralel", "perpendicular": "Dik",
          "tangent": "Teğet", "concentric": "Eş merkezli", "collinear": "Doğrudaş", "equal": "Eşit", "symmetric": "Simetrik",
          "fix": "Sabit", "distance": "Uzaklık", "angle": "Açı", "radius": "Yarıçap", "diameter": "Çap"}
DIMENSIONS = ("distance", "angle", "radius", "diameter")


def refs(rule):
    out = [rule["a"]]
    if rule.get("b") is not None:
        out.append(rule["b"])
    if isinstance(rule.get("axis"), dict):
        out.append(rule["axis"])
    return out


class Drawing:
    """The setup's objects by slot, their uids as `$uidOf:k`, the constraints and the parameters."""

    def __init__(self):
        self.entities = {e["id"]: dict(e) for e in ENTITIES}
        self.constraints = []  # each {"id": k (its number), "rule", "reference"}
        self.parameters = []
        self.next_id = 0
        self.last_noop = False

    def slot(self, uid):
        """An object's slot by its `$uidOf:k`; none for an id the drawing does not hold."""
        return int(uid[len("$uidOf:"):]) if uid.startswith("$uidOf:") else None

    def cc_case(self, constraints, values, held=None, held_slots=()):
        """The constraints as constraint_cases' case: objects in the order first named."""
        order = []
        for c in constraints:
            for r in refs(c["rule"]):
                s = self.slot(r["uid"])
                if s not in order:
                    order.append(s)
        index = {s: i for i, s in enumerate(order)}
        out = []
        for c in constraints:
            rule = c["rule"]
            v = {"rule": rule["kind"]}
            for key in ("a", "b"):
                if rule.get(key) is not None:
                    v[key] = [index[self.slot(rule[key]["uid"])], element_json(rule[key]["element"])]
            if isinstance(rule.get("axis"), dict):
                v["axis"] = [index[self.slot(rule["axis"]["uid"])], element_json(rule["axis"]["element"])]
            elif rule.get("axis"):
                v["axis"] = rule["axis"]
            for flag in ("inside", "left", "reverseA", "reverseB"):
                if rule.get(flag):
                    v[flag] = True
            if rule["kind"] in DIMENSIONS:
                v["value"] = values.get(rule["parameter"], 0.0)
                if c.get("reference"):
                    v["reference"] = True
            out.append(v)
        held = list(held or [])
        for s in order:
            if self.entities[s]["layerId"] == "kilitli" or s in held_slots:
                held.append([index[s], None])
        case = {"shapes": [shape(self.entities[s]) for s in order], "constraints": out}
        if held:
            case["held"] = held
        return case, order

    def measure(self, rule):
        """What a dimension measures now, in metres or radians."""
        case, _ = self.cc_case([{"rule": rule}], {rule.get("parameter", ""): 0.0})
        xs = [[cc.sp.Float(repr(float(v)), 50) for v in cc.unknowns(s)] for s in case["shapes"]]
        c = case["constraints"][0]
        parts = [cc.element(case["shapes"], xs, r) for r in cc.refs_of(c)]
        return float(cc.sp.N(cc.measure_of(c, parts), 30))

    def has(self, ref):
        s = self.slot(ref["uid"])
        if s is None or s not in self.entities:
            return False
        return cc.element([shape(self.entities[s])], [[0] * len(cc.unknowns(shape(self.entities[s])))], [0, element_json(ref["element"])]) is not None


def failed(code, path, uids=None):
    """A refusal as the case expects it: its code and path (the message is the command's own words)."""
    error = {"code": code, "message": "$any", "path": path}
    if uids:
        error["uids"] = uids
    return {"status": "failed", "error": error}


NAME_STEMS = {"angle": "ang", "radius": "rad", "diameter": "dia"}
RESERVED = {"pi", "sin", "cos", "tan", "asin", "acos", "atan", "sqrt", "abs", "round", "floor", "ceil", "min", "max"}


def name_ok(name):
    return bool(name) and len(name) <= 64 and (name[0].isalpha()) and all(c.isalnum() or c == "_" for c in name) and name not in RESERVED


def next_name(params, kind):
    stem = NAME_STEMS.get(kind, "d")
    k = 1
    while any(p["name"] == f"{stem}{k}" for p in params):
        k += 1
    return f"{stem}{k}"


def sensed(d, rule):
    """A new constraint's sense from the drawing (docs/adr/0335 §2)."""
    rule = json.loads(json.dumps(rule))
    k = rule["kind"]
    centre = lambda r: {"uid": r["uid"], "element": {"kind": "point", "place": {"kind": "center"}}}
    if k == "tangent":
        if is_seg(rule["a"]):
            side = d.measure({"kind": "distance", "a": centre(rule["b"]), "b": rule["a"], "left": True, "parameter": "x"})
            rule["left"] = side >= 0
            rule.pop("inside", None)
        else:
            dist = d.measure({"kind": "distance", "a": centre(rule["a"]), "b": centre(rule["b"]), "parameter": "x"})
            ra = d.measure({"kind": "radius", "a": rule["a"], "parameter": "x"})
            rb = d.measure({"kind": "radius", "a": rule["b"], "parameter": "x"})
            rule["inside"] = abs(dist - abs(ra - rb)) < abs(dist - (ra + rb))
            rule.pop("left", None)
    elif k == "distance":
        axis = rule.get("axis", "aligned")
        if is_seg(rule["b"]):
            probe = dict(rule, left=True, axis="aligned")
            rule["left"] = d.measure(probe) >= 0
        elif axis in ("horizontal", "vertical"):
            if d.measure(dict(rule, left=False)) < 0:
                rule["a"], rule["b"] = rule["b"], rule["a"]
            rule.pop("left", None)
        else:
            rule.pop("left", None)
    elif k == "angle":
        rule.pop("reverseA", None)
        rule.pop("reverseB", None)
        if d.measure(rule) > math.pi:
            rule["reverseB"] = True
    # The contract writes no flag at its default.
    for flag in ("inside", "left", "reverseA", "reverseB"):
        if rule.get(flag) is False:
            rule.pop(flag)
    return rule


def turned(before, after):
    """Whether a shape is turned inside out (docs/adr/0335 §4)."""
    kb, vb = next(iter(before.items()))
    ka, va = next(iter(after.items()))
    if kb == "line":
        u = (vb[1][0] - vb[0][0], vb[1][1] - vb[0][1])
        w = (va[1][0] - va[0][0], va[1][1] - va[0][1])
        return u[0] * w[0] + u[1] * w[1] < 0
    if kb == "circle":
        return va[1] <= 0
    if kb == "arc":
        return va[1] <= 0 or ((vb[3] - vb[2]) > 0) != ((va[3] - va[2]) > 0)
    if kb == "polygon":
        area = lambda pts: sum(pts[i][0] * pts[(i + 1) % len(pts)][1] - pts[(i + 1) % len(pts)][0] * pts[i][1] for i in range(len(pts)))
        x, y = area(vb["pts"]), area(va["pts"])
        return x != 0 and x * y <= 0
    return False


def kind_of(rule):
    return "angle" if rule["kind"] == "angle" else "length"


def to_base(kind, v):
    return v * RAD_PER_DEG if kind == "angle" else v


def from_base(kind, v):
    return v / RAD_PER_DEG if kind == "angle" else v


def values_of(d, constraints, params):
    """Every parameter's value in its own unit: a reference dimension's measured, the others by their expressions in
    the order they read each other; a refusal (code, path) when one cannot be."""
    values, waiting = {}, {}
    names = {p["name"] for p in params}
    for i, p in enumerate(params):
        if p.get("expression"):
            try:
                evaluate(p["expression"], {n: 1.0 for n in names}, p["kind"])
            except (ValueError, IndexError, TypeError):
                return None, ("parameter_expression", f"parameters[{i}]")
            except KeyError:
                return None, ("parameter_missing", f"parameters[{i}]")
            except ZeroDivisionError:
                pass
    for i, p in enumerate(params):
        if not p.get("expression"):
            c = next(c for c in constraints if c["rule"].get("parameter") == p["name"])
            values[p["name"]] = from_base(p["kind"], d.measure(c["rule"]))
        else:
            waiting[p["name"]] = (i, p)
    while waiting:
        progress = False
        for name, (i, p) in sorted(waiting.items()):
            try:
                values[name] = evaluate(p["expression"], values, p["kind"])
            except KeyError:
                continue
            del waiting[name]
            progress = True
        if not progress:
            first = min(i for i, _ in waiting.values())
            return None, ("parameter_cycle", f"parameters[{first}]")
    return values, None


def edit(d, edit_input, allow_flip=False, hold=()):
    """Runs cad.constraints.edit on the drawing `d`: the result, and on success the drawing changed."""
    e = edit_input
    kind = e["kind"]
    # The edit's own shape.
    if kind == "add":
        if not e["constraints"]:
            return failed("no_constraints", "edit.constraints")
        for i, c in enumerate(e["constraints"]):
            at = f"edit.constraints[{i}]"
            if not fits(c["rule"]):
                return failed("invalid_constraint", f"{at}.rule")
            if c.get("reference") and c["rule"]["kind"] not in DIMENSIONS:
                return failed("invalid_constraint", f"{at}.reference")
            name = c["rule"].get("parameter")
            if name and not name_ok(name):
                return failed("invalid_parameter", f"{at}.rule.parameter")
            if "value" in c:
                if c.get("reference") or c["rule"]["kind"] not in DIMENSIONS:
                    return failed("parameter_expression", f"{at}.value")
                try:
                    evaluate(c["value"], {n: 1.0 for n in "abcdefghijklmnopqrstuvwxyz"}, "length")
                except (ValueError, IndexError):
                    return failed("parameter_expression", f"{at}.value")
                except KeyError:
                    pass
    if kind not in ("add", "remove", "mode", "flip", "solve"):
        raise ValueError(kind)
    for i, h in enumerate(hold):
        if not (h.startswith("$uidOf:") or UUID.fullmatch(h)):
            return failed("invalid_uid", f"hold[{i}]")
    if kind == "add":
        pass
    elif kind in ("remove", "mode", "flip"):
        listed = e["ids"] if kind != "flip" else [e["id"]]
        place = "edit.ids" if kind != "flip" else "edit.id"
        if not listed:
            return failed("no_constraints", place)
        for i, text in enumerate(listed):
            if not (text.startswith("$constraint:") or UUID.fullmatch(text)):
                return failed("invalid_uid", f"{place}[{i}]")
    for i, h in enumerate(hold):
        if d.slot(h) is None or d.slot(h) not in d.entities:
            return failed("entity_not_found", f"hold[{i}]", [h])
    constraints = [dict(c) for c in d.constraints]
    params = [dict(p) for p in d.parameters]
    added, removed = [], []
    if kind == "add":
        for i, c in enumerate(e["constraints"]):
            at = f"edit.constraints[{i}]"
            for r in refs(c["rule"]):
                if d.slot(r["uid"]) is None or d.slot(r["uid"]) not in d.entities:
                    return failed("entity_not_found", f"{at}.rule", [r["uid"]])
                if not d.has(r):
                    return failed("element_not_found", f"{at}.rule", [r["uid"]])
            rule = sensed(d, c["rule"]) if c.get("sense", "drawing") == "drawing" else json.loads(json.dumps(c["rule"]))
            if rule["kind"] in DIMENSIONS:
                if not rule.get("parameter"):
                    rule["parameter"] = next_name(params, rule["kind"])
                name = rule["parameter"]
                if any(k["rule"].get("parameter") == name for k in constraints):
                    return failed("parameter_in_use", f"{at}.rule.parameter")
                pk = kind_of(rule)
                if c.get("reference"):
                    expression = ""
                elif "value" in c:
                    expression = c["value"]
                else:
                    expression = value_text(from_base(pk, d.measure(rule)))
                old = next((p for p in params if p["name"] == name), None)
                if old is not None and old["kind"] != pk:
                    return failed("parameter_kind", f"{at}.rule.parameter")
                if old is None:
                    p = {"name": name, "kind": pk}
                    if expression:
                        p["expression"] = expression
                    params.append(p)
                elif c.get("reference"):
                    old.pop("expression", None)
                elif "value" in c:
                    old["expression"] = expression
            new = {"id": d.next_id, "rule": rule}
            d.next_id += 1
            if c.get("reference"):
                new["reference"] = True
            constraints.append(new)
            added.append(new["id"])
    else:
        ids = e.get("ids") or [e.get("id")] if kind in ("remove", "mode", "flip") else []
        place = "edit.id" if kind == "flip" else "edit.ids"
        found = []
        for i, text in enumerate(ids):
            k = next((j for j, c in enumerate(constraints) if f"$constraint:{j}" == text), None)
            if k is None:
                return failed("constraint_not_found", place if kind == "flip" else f"{place}[{i}]")
            found.append(k)
        if kind == "remove":
            gone = [constraints[k] for k in found]
            removed = [f"$constraint:{k}" for k in found]
            constraints = [c for j, c in enumerate(constraints) if j not in found]
            read = set()
            for p in params:
                for tok in ("".join(ch if ch.isalnum() or ch == "_" else " " for ch in p.get("expression", ""))).split():
                    read.add(tok)
            for c in gone:
                name = c["rule"].get("parameter")
                if not name:
                    continue
                if name not in read:
                    params = [p for p in params if p["name"] != name]
                else:
                    p = next(p for p in params if p["name"] == name)
                    if not p.get("expression"):
                        p["expression"] = value_text(from_base(p["kind"], d.measure(c["rule"])))
        elif kind == "mode":
            for i, k in enumerate(found):
                rule = constraints[k]["rule"]
                if rule["kind"] not in DIMENSIONS:
                    return failed("invalid_constraint", f"edit.ids[{i}]")
                if e["reference"]:
                    constraints[k]["reference"] = True
                else:
                    constraints[k].pop("reference", None)
                p = next(p for p in params if p["name"] == rule["parameter"])
                if e["reference"]:
                    p.pop("expression", None)
                elif not p.get("expression"):
                    p["expression"] = value_text(from_base(p["kind"], d.measure(rule)))
        elif kind == "flip":
            k = found[0]
            rule = json.loads(json.dumps(constraints[k]["rule"]))
            rk = rule["kind"]
            if rk == "tangent":
                flag = "left" if is_seg(rule["a"]) else "inside"
                rule[flag] = not rule.get(flag, False)
            elif rk == "distance" and is_seg(rule["b"]):
                rule["left"] = not rule.get("left", False)
            elif rk == "distance" and rule.get("axis", "aligned") != "aligned":
                rule["a"], rule["b"] = rule["b"], rule["a"]
            elif rk == "angle":
                rule["a"], rule["b"] = rule["b"], rule["a"]
                ra, rb = rule.pop("reverseA", False), rule.pop("reverseB", False)
                if rb:
                    rule["reverseA"] = True
                if ra:
                    rule["reverseB"] = True
            else:
                return failed("invalid_constraint", "edit.id")
            for flag in ("inside", "left", "reverseA", "reverseB"):
                if rule.get(flag) is False:
                    rule.pop(flag)
            constraints[k]["rule"] = rule
    values, problem = values_of(d, constraints, params)
    if problem:
        return failed(*problem)
    for k in constraints:
        rule = k["rule"]
        if rule["kind"] in ("radius", "diameter") and not k.get("reference") and values[rule["parameter"]] <= 0:
            return failed("parameter_range", "edit")
    held_slots = [d.slot(h) for h in hold]
    case, order = d.cc_case(constraints, {n: to_base(next(p["kind"] for p in params if p["name"] == n), v) for n, v in values.items()},
                            held_slots=held_slots)
    out = cc.reference(case, all_shapes=True)
    refused_ids = lambda ks: ["$any" if constraints[k]["id"] in added else f"$constraint:{k}" for k in ks]
    if out["status"] == "inconsistent":
        locked = all(
            all(d.entities[d.slot(r["uid"])]["layerId"] == "kilitli" for r in refs(constraints[k]["rule"]))
            for k in out["conflicting"]
        )
        return failed("layer_locked" if locked else "constraint_conflict", "edit", refused_ids(out["conflicting"]))
    if out["status"] == "singular":
        return failed("constraint_singular", "edit", refused_ids(out["singular"]))
    if out["status"] == "nonConvergent":
        return failed("not_converged", "edit")
    repeats = [k for k in out["redundant"] if constraints[k]["id"] in added]
    if repeats:
        return failed("constraint_redundant", "edit.constraints", ["$any"] * len(repeats))
    changed, flipped = [], []
    for i, s in enumerate(order):
        before = shape(d.entities[s])
        after = out["solved"][i]
        if moved(before, after):
            changed.append(s)
            if turned(before, after):
                flipped.append(s)
    if flipped and not allow_flip:
        return failed("shape_flipped", "edit", [f"$uidOf:{s}" for s in flipped])
    # Written; nothing at all changed: no step.
    d.last_noop = not changed and constraints == d.constraints and params == d.parameters
    for i, s in enumerate(order):
        if s in changed:
            d.entities[s] = entity(d.entities[s], out["solved"][i])
    d.constraints = constraints
    d.parameters = params
    diagnosis = {"status": out["status"], "dof": out["dof"]}
    for key in ("redundant", "conflicting", "singular", "broken"):
        if out[key]:
            diagnosis[key] = [f"$constraint:{k}" for k in out[key]]
    if out["free"]:
        diagnosis["free"] = [f"$uidOf:{order[i]}" for i in out["free"]]
    diagnosis["iterations"] = "$any"
    output = {
        "added": ["$any"] * len(added),
        "removed": ["$any"] * len(removed),
        "changed": [f"$uidOf:{s}" for s in changed],
        "diagnosis": diagnosis,
        "revision": "$current",
    }
    return {"status": "completed", "output": output, "warnings": []}, changed


def moved(before, after):
    """Whether the solve moves an object: by more than the solver's tolerance (a value written to twelve decimals
    is within it, and the solver does not step for it)."""
    a = cc.unknowns(before)
    b = cc.unknowns(after)
    return any(abs(x - y) > 1e-10 for x, y in zip(a, b))


# ---- the cases ------------------------------------------------------------------------------------------------------

def U(slot):
    return f"$uidOf:{slot}"


def pt(slot, place, **more):
    return {"uid": U(slot), "element": {"kind": "point", "place": {"kind": place, **more}}}


def seg(slot, index=0):
    return {"uid": U(slot), "element": {"kind": "segment", "part": 0, "ring": 0, "index": index}}


def circ(slot):
    return {"uid": U(slot), "element": {"kind": "circle"}}


def add(*constraints):
    return {"kind": "add", "constraints": list(constraints)}


def c(rule, **more):
    return {"rule": rule, **more}


def rect_rules(width=None, height=None):
    out = [c({"kind": "coincident", "a": pt(i, "end"), "b": pt(i % 4 + 1, "start")}) for i in range(1, 5)]
    out += [
        c({"kind": "horizontal", "a": seg(1)}),
        c({"kind": "parallel", "a": seg(1), "b": seg(3)}),
        c({"kind": "parallel", "a": seg(2), "b": seg(4)}),
        c({"kind": "perpendicular", "a": seg(1), "b": seg(2)}),
        c({"kind": "fix", "a": pt(1, "start")}),
    ]
    if width is not None:
        out.append(c({"kind": "distance", "a": pt(1, "start"), "b": pt(1, "end"), "parameter": ""}, value=width))
    if height is not None:
        out.append(c({"kind": "distance", "a": pt(2, "start"), "b": pt(2, "end"), "parameter": ""}, value=height))
    return out


class Case:
    """A case's steps, each run on the reference drawing as it goes."""

    def __init__(self, name):
        self.name = name
        self.d = Drawing()
        self.steps = []

    def execute(self, edit_input, allow_flip=False, op="execute", within=None, hold=()):
        before = {k: dict(v) for k, v in self.d.entities.items()}
        got = edit(self.d, edit_input, allow_flip, hold)
        if isinstance(got, tuple):
            result, changed = got
        else:
            result, changed = got, []
        step = {"op": op, "input": {"edit": edit_input}}
        if hold:
            step["input"]["hold"] = list(hold)
        if allow_flip:
            step["input"]["allowFlip"] = True
        step["result"] = result
        if result["status"] == "completed" and not self.d.last_noop:
            expect = {"revision": "changed", "canUndo": True}
            if changed:
                expect["entities"] = {str(s): self.d.entities[s] for s in changed}
            if changed:
                step["within"] = within if within is not None else 1e-9
            step["expect"] = expect
        elif result["status"] == "completed":
            step["expect"] = {"revision": "same"}
        else:
            step["expect"] = {"revision": "same"}
            self.d.entities = before
        self.steps.append(step)
        return self

    def undo(self, label="Kısıtlar"):
        self.steps.append({"op": "undo", "returns": label, "expect": {"canRedo": True}})
        return self

    def json(self):
        return {"name": self.name, "steps": self.steps}


def cases():
    out = []

    def case(name):
        k = Case(name)
        out.append(k)
        return k

    case("Dikdörtgen: köşeler, yatay, paralel kenarlar, dik köşe, sabit köşe, genişlik 8 ve yükseklik 4; tam kısıtlı").execute(
        add(*rect_rules("8", "4")), within=1e-9).undo()
    case("Uzaklık değersiz: çizimin ölçtüğü parametre olur, hiçbir şey kımıldamaz").execute(
        add(c({"kind": "distance", "a": pt(1, "start"), "b": pt(1, "end"), "parameter": ""})))
    case("İkinci ölçünün değeri birincinin parametresini okur (d2 = d1 / 2)").execute(
        add(*rect_rules("8", "d1 / 2")), within=1e-9)
    case("Başvuru ölçüsü: ifadesi boş, yalnız ölçer").execute(
        add(c({"kind": "distance", "a": pt(2, "start"), "b": pt(2, "end"), "parameter": "yukseklik"}, reference=True)))
    case("Başvuru ölçüsüne değer verilmez").execute(
        add(c({"kind": "distance", "a": pt(2, "start"), "b": pt(2, "end"), "parameter": ""}, reference=True, value="3")))
    case("İki noktayı yatay yap: ikisi de en az kımıldar").execute(
        add(c({"kind": "horizontal", "a": pt(10, "node", part=0), "b": pt(11, "node", part=0)})), within=1e-12)
    case("Doğruya teğet daire: merkezi doğrunun solunda kalır").execute(
        add(c({"kind": "fix", "a": seg(6)}), c({"kind": "tangent", "a": seg(6), "b": circ(5)})), within=1e-12)
    case("Teğet: ilk seçilen doğru tutulur, daire ona uyar").execute(
        add(c({"kind": "tangent", "a": seg(6), "b": circ(5)})), hold=[U(6)], within=1e-12)
    case("Paralel: ilk seçilen üst kenar tutulur, alt kenar döner").execute(
        add(c({"kind": "parallel", "a": seg(3), "b": seg(1)})), hold=[U(3)])
    case("Tutulacak nesne yok").execute(add(c({"kind": "horizontal", "a": seg(1)})), hold=["0192f5a6-9999-7000-8000-000000000003"])
    case("Teğeti çevirmek daireyi sıfır yarıçapa indirir: ters dönme reddedilir").execute(
        add(c({"kind": "fix", "a": seg(6)}), c({"kind": "tangent", "a": seg(6), "b": circ(5)})), within=1e-12).execute(
        {"kind": "flip", "id": "$constraint:1"})
    case("Yatay uzaklık çevrilince noktalar yer değiştirir").execute(
        add(c({"kind": "distance", "a": pt(11, "node", part=0), "b": pt(10, "node", part=0), "axis": "horizontal", "parameter": ""}))).execute(
        {"kind": "flip", "id": "$constraint:0"}, within=1e-12)
    case("Nokta sabit dairenin üstüne iner").execute(
        add(c({"kind": "fix", "a": circ(5)}), c({"kind": "coincident", "a": pt(10, "node", part=0), "b": circ(5)})), within=1e-12)
    case("Açı derecede, 180°'den küçük olanı: sol kenar alt kenarla 60°").execute(
        add(c({"kind": "fix", "a": seg(1)}), c({"kind": "coincident", "a": pt(4, "end"), "b": pt(1, "start")}),
            c({"kind": "distance", "a": pt(4, "start"), "b": pt(4, "end"), "parameter": ""}, value="4"),
            c({"kind": "angle", "a": seg(1), "b": seg(4), "parameter": ""}, value="60")), within=1e-9)
    case("Çelişkili genişlik: hiçbir şey yazılmaz").execute(
        add(*rect_rules("8", "4"), c({"kind": "distance", "a": pt(3, "start"), "b": pt(3, "end"), "parameter": ""}, value="7")))
    case("Aynı genişliği yeniden vermek fazladır, eklenmez").execute(
        add(*rect_rules("8", "4")), within=1e-9).execute(
        add(c({"kind": "distance", "a": pt(3, "start"), "b": pt(3, "end"), "parameter": ""}, value="8")))
    case("Kaldır: ölçü kısıtının parametresi de gider").execute(
        add(*rect_rules("8", "4")), within=1e-9).execute({"kind": "remove", "ids": ["$constraint:9"]})
    case("Kaldır: başka parametrenin okuduğu parametre kalır").execute(
        add(*rect_rules("8", "d1 / 2")), within=1e-9).execute({"kind": "remove", "ids": ["$constraint:9"]})
    case("Başvuru yap ve yeniden sürücü yap").execute(
        add(*rect_rules("8", "4")), within=1e-9).execute(
        {"kind": "mode", "ids": ["$constraint:10"], "reference": True}).execute(
        {"kind": "mode", "ids": ["$constraint:10"], "reference": False})
    case("Geometrik kısıt başvuru olmaz").execute(
        add(c({"kind": "horizontal", "a": seg(1)}))).execute({"kind": "mode", "ids": ["$constraint:0"], "reference": True})
    case("Paralelin tek çözümü var: çevrilmez").execute(
        add(c({"kind": "parallel", "a": seg(1), "b": seg(3)}))).execute({"kind": "flip", "id": "$constraint:0"})
    case("Kilitli katmandaki çizgi kımıldamaz; ona bağlanan çizgi gelir").execute(
        add(c({"kind": "coincident", "a": pt(4, "end"), "b": pt(8, "start")})), within=1e-12)
    case("Kilitli çizgiyi yatay yapmak çelişir").execute(add(c({"kind": "horizontal", "a": seg(8)})))
    case("Yaylı kenar kısıt almaz").execute(add(c({"kind": "horizontal", "a": seg(9, 1)})))
    case("Olmayan nesne").execute(add(c({"kind": "horizontal", "a": {"uid": "0192f5a6-9999-7000-8000-000000000001", "element": {"kind": "segment", "part": 0, "ring": 0, "index": 0}}})))
    case("Parçalar kurala uymuyor: noktaya paralel").execute(add(c({"kind": "parallel", "a": pt(1, "start"), "b": seg(3)})))
    case("Boş ekleme").execute(add())
    case("Olmayan kısıt kaldırılamaz").execute({"kind": "remove", "ids": ["0192f5a6-9999-7000-8000-000000000002"]})
    case("Parametre adı geçersiz").execute(add(c({"kind": "radius", "a": circ(5), "parameter": "2r"}, value="2")))
    case("Parametre başka bir ölçünün").execute(add(
        c({"kind": "radius", "a": circ(5), "parameter": "r"}, value="2"), c({"kind": "diameter", "a": circ(5), "parameter": "r"}, value="4")))
    case("Değer okunamıyor").execute(add(c({"kind": "radius", "a": circ(5), "parameter": ""}, value="2 +")))
    case("Değer olmayan bir adı okuyor").execute(add(c({"kind": "radius", "a": circ(5), "parameter": ""}, value="genislik")))
    case("Parametreler birbirini okuyor").execute(add(
        c({"kind": "radius", "a": circ(5), "parameter": "a"}, value="b"), c({"kind": "distance", "a": pt(1, "start"), "b": pt(1, "end"), "parameter": "b"}, value="a")))
    case("Yalnız çöz: kısıtsız çizim tam, değişmez").execute({"kind": "solve"})
    case("Geçersiz kısıt kimliği").execute({"kind": "remove", "ids": ["kisit-1"]})
    case("Kullanıcı parametresi kalan uzunluğa açı bağlanamaz").execute(
        add(*rect_rules("8", "d1 / 2")), within=1e-9).execute({"kind": "remove", "ids": ["$constraint:9"]}).execute(
        add(c({"kind": "angle", "a": seg(1), "b": seg(2), "parameter": "d1"})))
    k = case("Beklenen sürüm geçersiz")
    k.steps.append({"op": "execute", "input": {"edit": add(c({"kind": "horizontal", "a": seg(1)})), "expectedRevision": "07"},
                    "result": {"status": "failed", "error": {"code": "invalid_revision", "message": "$any", "path": "expectedRevision"}},
                    "expect": {"revision": "same"}})
    k = case("Beklenen sürüm geçmişte kaldı: conflict, hiçbir şey yazılmaz")
    k.steps.append({"op": "execute", "input": {"edit": add(c({"kind": "horizontal", "a": seg(1)})), "expectedRevision": "999"},
                    "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": "$any", "path": "expectedRevision",
                                                               "revision": "$current"}},
                    "expect": {"revision": "same"}})
    case("Yarıçap sıfır olamaz").execute(add(c({"kind": "radius", "a": circ(5), "parameter": ""}, value="0")))
    case("Çizginin iki ucu çakışık ve paralel: tekil").execute(add(
        c({"kind": "coincident", "a": pt(1, "start"), "b": pt(1, "end")}), c({"kind": "parallel", "a": seg(1), "b": seg(3)})))
    return out


def header(command, title, note):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": note, "setup": SETUP}


def query_answer(d, only=None):
    """What cad.constraints.query answers for the drawing `d` (only the constraints naming slot `only`)."""
    values, _ = values_of(d, d.constraints, d.parameters)
    kinds = {p["name"]: p["kind"] for p in d.parameters}
    case, order = d.cc_case(d.constraints, {n: to_base(kinds[n], v) for n, v in values.items()})
    out = cc.reference(case)
    listed = []
    for i, con in enumerate(d.constraints):
        if only is not None and all(d.slot(r["uid"]) != only for r in refs(con["rule"])):
            continue
        item = {"constraint": {"id": f"$constraint:{i}", "rule": con["rule"]}, "label": LABELS[con["rule"]["kind"]], "holds": True}
        if con.get("reference"):
            item["constraint"]["reference"] = True
        if con["rule"]["kind"] in DIMENSIONS:
            item["measured"] = d.measure(con["rule"])
        listed.append(item)
    diagnosis = {"status": out["status"], "dof": out["dof"]}
    for key in ("redundant", "conflicting", "singular", "broken"):
        if out[key]:
            diagnosis[key] = [f"$constraint:{k}" for k in out[key]]
    if out["free"]:
        diagnosis["free"] = [f"$uidOf:{order[i]}" for i in out["free"]]
    diagnosis["iterations"] = "$any"
    return {"status": "completed", "warnings": [], "output": {
        "constraints": listed, "parameters": d.parameters, "diagnosis": diagnosis, "revision": "$current"}}


def query_cases():
    """cad.constraints.query after each of ten drawings' constraints are added: all of them, then those of one object."""
    scenes = [
        ("Dikdörtgen tam kısıtlı", add(*rect_rules("8", "4")), 2),
        ("Ölçüsüz dikdörtgen: iki serbestlik", add(*rect_rules()), 3),
        ("Başvuru ölçüsüyle dikdörtgen", add(*rect_rules("8", "4"), c({"kind": "distance", "a": pt(3, "start"), "b": pt(3, "end"), "parameter": ""}, reference=True)), 3),
        ("Okuyan parametreler", add(*rect_rules("8", "d1 / 2")), 1),
        ("Teğet daire", add(c({"kind": "fix", "a": seg(6)}), c({"kind": "tangent", "a": seg(6), "b": circ(5)})), 5),
        ("Yatay iki nokta", add(c({"kind": "horizontal", "a": pt(10, "node", part=0), "b": pt(11, "node", part=0)})), 10),
        ("Açı ve uzunluk", add(c({"kind": "fix", "a": seg(1)}), c({"kind": "coincident", "a": pt(4, "end"), "b": pt(1, "start")}),
            c({"kind": "distance", "a": pt(4, "start"), "b": pt(4, "end"), "parameter": ""}, value="4"),
            c({"kind": "angle", "a": seg(1), "b": seg(4), "parameter": ""}, value="60")), 4),
        ("Kilitli çizgiye bağlı uç", add(c({"kind": "coincident", "a": pt(4, "end"), "b": pt(8, "start")})), 8),
        ("Daireye oturan nokta", add(c({"kind": "fix", "a": circ(5)}), c({"kind": "coincident", "a": pt(10, "node", part=0), "b": circ(5)})), 10),
        ("Yarıçap ve sabit merkez", add(c({"kind": "fix", "a": pt(5, "center")}), c({"kind": "radius", "a": circ(5), "parameter": ""}, value="2")), 5),
    ]
    out = []
    for name, edit_input, one in scenes:
        for only in (None, one):
            k = Case(f"{name}: " + ("bütün kısıtlar" if only is None else f"yalnız {one}. nesneyi ananlar"))
            k.execute(edit_input)
            k.steps[0]["command"] = "cad.constraints.edit"
            step = {"op": "execute", "input": {} if only is None else {"uids": [U(only)]}, "result": query_answer(k.d, only),
                    "within": 1e-9, "expect": {"revision": "same"}}
            k.steps.append(step)
            out.append(k)
    k = Case("Kısıtsız çizim: boş liste, tam")
    k.steps.append({"op": "execute", "input": {}, "result": query_answer(k.d), "expect": {"revision": "same"}})
    out.append(k)
    k = Case("Geçersiz nesne kimliği")
    k.steps.append({"op": "execute", "input": {"uids": ["nesne"]}, "result": failed("invalid_uid", "uids[0]"), "expect": {"revision": "same"}})
    out.append(k)
    return out


def build():
    edit_cases = [k.json() for k in cases()]
    edit_file = header(
        "cad.constraints.edit", "Kısıtlar",
        "ADR 0335: kısıtlar parçalarına tıklanarak eklenir, kaldırılır, sürücü ya da başvuru yapılır, öbür çözümüne çevrilir; "
        "çizim çözücüyle (ADR 0334) çözülür ve kısıtlarla birlikte tek adımda yazılır. Yön çizimden okunur; şekli ters çeviren "
        "çözüm reddedilir. Durumlar KentOS kodu olmadan scripts/fixtures/constraint_command_cases.py ile yazılır.")
    edit_file["cases"] = edit_cases
    query_file = header(
        "cad.constraints.query", "Kısıtları oku",
        "ADR 0335 §1: kısıtlar Türkçe adları, ölçü kısıtının bugünkü değeri ve tutup tutmadığıyla, parametreler ve çizimin "
        "teşhisi; hiçbir şey yazılmaz. Kısıtlar önce cad.constraints.edit ile eklenir.")
    query_file["cases"] = [k.json() for k in query_cases()]
    return edit_file, query_file


def main():
    built, query = build()
    files = [(EDIT, built), (QUERY, query)]
    if "--check" in sys.argv:
        for path, data in files:
            if path.read_text() != json.dumps(data, ensure_ascii=False, indent=1) + "\n":
                print(f"{path} güncel değil: python3 {sys.argv[0]}")
                sys.exit(1)
        print(f"{len(built['cases'])} + {len(query['cases'])} durum güncel")
        return
    for path, data in files:
        path.write_text(json.dumps(data, ensure_ascii=False, indent=1) + "\n")
    for k in built["cases"]:
        said = [f"{st['result']['status']} {st['result'].get('error', {}).get('code', '')}".strip() for st in k["steps"] if "result" in st]
        print(f"{k['name']}: {' → '.join(said)}")


if __name__ == "__main__":
    main()
