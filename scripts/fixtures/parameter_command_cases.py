#!/usr/bin/env python3
"""The shared cases of the parameter command cad.parameters.edit (docs/adr/0336 §1).

    python3 scripts/fixtures/parameter_command_cases.py           # writes the file
    python3 scripts/fixtures/parameter_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.parameters.edit.json. The rules are written here from the ADR, without KentOS code:
the edits in order (add a user parameter, set an expression, bounds or a description, rename with the expressions
and the dimension that read it following, remove a user parameter nobody reads), their checks and order; then the
drawing solved with the new values by constraint_command_cases.py's model of cad.constraints.edit, whose solutions
come from the solver's independent reference (constraint_cases.py: sympy, mpmath at 50 digits).
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import constraint_cases as cc  # noqa: E402
import constraint_command_cases as ccc  # noqa: E402
from constraint_command_cases import (  # noqa: E402
    DIMENSIONS, Case, U, add, c, circ, failed, moved, name_ok, pt, rect_rules, refs, seg, to_base, values_of,
)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.parameters.edit.json"


def reads(params):
    out = set()
    for p in params:
        out |= set(ccc.names_in(p.get("expression", "")))
    return out


def rename_in(text, old, new):
    """The ADR's rename: whole names only; a word right after a number is its unit."""
    out, i = [], 0
    while i < len(text):
        ch = text[i]
        if ch.isalpha() or ch == "_":
            j = i
            while j < len(text) and (text[j].isalnum() or text[j] == "_"):
                j += 1
            word = text[i:j]
            unit = i > 0 and (text[i - 1].isdigit() or text[i - 1] == ".")
            out.append(new if word == old and not unit else word)
            i = j
        else:
            out.append(ch)
            i += 1
    return "".join(out)


def parses(text):
    try:
        ccc.evaluate(text, ccc.AnyValue(), "length")
        return True
    except (ValueError, IndexError, TypeError):
        return False
    except ZeroDivisionError:
        return True


def param_edit(d, edits):
    """cad.parameters.edit on the drawing `d`: the result, and on success the drawing changed."""
    for i, e in enumerate(edits):
        at = lambda f: f"edits[{i}].{f}"
        k = e["kind"]
        if k == "add":
            p = e["parameter"]
            if not name_ok(p["name"]):
                return failed("invalid_parameter", at("parameter.name"))
            if not p.get("expression"):
                return failed("parameter_expression", at("parameter.expression"))
            if not parses(p["expression"]):
                return failed("parameter_expression", at("parameter.expression"))
        elif k == "set":
            if not name_ok(e["name"]):
                return failed("invalid_parameter", at("name"))
            if "expression" in e and not parses(e["expression"]):
                return failed("parameter_expression", at("expression"))
        elif k == "rename":
            if not name_ok(e["name"]):
                return failed("invalid_parameter", at("name"))
            if not name_ok(e["to"]):
                return failed("invalid_parameter", at("to"))
        elif k == "remove":
            if not name_ok(e["name"]):
                return failed("invalid_parameter", at("name"))
    constraints = json.loads(json.dumps(d.constraints))
    params = json.loads(json.dumps(d.parameters))
    for i, e in enumerate(edits):
        at = lambda f: f"edits[{i}].{f}"
        k = e["kind"]
        named = lambda n: next((p for p in params if p["name"] == n), None)
        if k == "add":
            if named(e["parameter"]["name"]):
                return failed("parameter_exists", at("parameter.name"))
            params.append(dict(e["parameter"]))
        elif k == "set":
            measured = any(con.get("reference") and con["rule"].get("parameter") == e["name"] for con in constraints)
            if "expression" in e and measured:
                return failed("parameter_measured", at("expression"))
            p = named(e["name"])
            if p is None:
                return failed("parameter_not_found", at("name"))
            if "expression" in e:
                p["expression"] = e["expression"]
            for bound in ("min", "max"):
                if bound in e:
                    if e[bound]["kind"] == "clear":
                        p.pop(bound, None)
                    else:
                        p[bound] = e[bound]["value"]
            if "description" in e:
                if e["description"]:
                    p["description"] = e["description"]
                else:
                    p.pop("description", None)
        elif k == "rename":
            if named(e["name"]) is None:
                return failed("parameter_not_found", at("name"))
            if named(e["to"]):
                return failed("parameter_exists", at("to"))
            for p in params:
                if p["name"] == e["name"]:
                    p["name"] = e["to"]
                if p.get("expression"):
                    p["expression"] = rename_in(p["expression"], e["name"], e["to"])
            for con in constraints:
                if con["rule"].get("parameter") == e["name"]:
                    con["rule"]["parameter"] = e["to"]
        elif k == "remove":
            if named(e["name"]) is None:
                return failed("parameter_not_found", at("name"))
            owned = any(con["rule"].get("parameter") == e["name"] for con in constraints)
            if owned or e["name"] in reads(params):
                return failed("parameter_read", at("name"))
            params = [p for p in params if p["name"] != e["name"]]
    values, problem = values_of(d, constraints, params)
    if problem:
        return failed(*problem)
    for con in constraints:
        rule = con["rule"]
        if rule["kind"] in ("radius", "diameter") and not con.get("reference") and values[rule["parameter"]] <= 0:
            return failed("parameter_range", "edit")
    for i, p in enumerate(params):
        v = values.get(p["name"])
        if v is not None and (("min" in p and v < p["min"]) or ("max" in p and v > p["max"])):
            return failed("parameter_range", f"parameters[{i}]")
    kinds = {p["name"]: p["kind"] for p in params}
    case, order = d.cc_case(constraints, {n: to_base(kinds[n], v) for n, v in values.items()})
    out = cc.reference(case, all_shapes=True)
    if out["status"] == "inconsistent":
        return failed("constraint_conflict", "edit", [f"$constraint:{k}" for k in out["conflicting"]])
    if out["status"] == "singular":
        return failed("constraint_singular", "edit", [f"$constraint:{k}" for k in out["singular"]])
    changed = [s for i, s in enumerate(order) if moved(ccc.shape(d.entities[s]), out["solved"][i])]
    flipped = [s for i, s in enumerate(order) if s in changed and ccc.turned(ccc.shape(d.entities[s]), out["solved"][i])]
    if flipped:
        return failed("shape_flipped", "edit", [f"$uidOf:{s}" for s in flipped])
    for i, s in enumerate(order):
        if s in changed:
            d.entities[s] = ccc.entity(d.entities[s], out["solved"][i])
    d.last_noop = not changed and constraints == d.constraints and params == d.parameters
    d.constraints, d.parameters = constraints, params
    after, _ = values_of(d, d.constraints, d.parameters)
    diagnosis = {"status": out["status"], "dof": out["dof"]}
    for key in ("redundant", "conflicting", "singular", "broken"):
        if out[key]:
            diagnosis[key] = [f"$constraint:{k}" for k in out[key]]
    if out["free"]:
        diagnosis["free"] = [f"$uidOf:{order[i]}" for i in out["free"]]
    diagnosis["iterations"] = "$any"
    output = {
        "values": [{"name": p["name"], "kind": p["kind"], "value": after[p["name"]]} for p in d.parameters if p["name"] in after],
        "changed": [f"$uidOf:{s}" for s in changed],
        "diagnosis": diagnosis,
        "revision": "$current",
    }
    return {"status": "completed", "output": output, "warnings": []}, changed


class ParamCase(Case):
    def params(self, edits, within=None):
        before = {k: dict(v) for k, v in self.d.entities.items()}
        got = param_edit(self.d, edits)
        result, changed = got if isinstance(got, tuple) else (got, [])
        step = {"op": "execute", "input": {"edits": edits}, "result": result}
        if result["status"] == "completed" and not self.d.last_noop:
            step["within"] = within or 1e-9
            expect = {"revision": "changed", "canUndo": True}
            if changed:
                expect["entities"] = {str(s): self.d.entities[s] for s in changed}
            step["expect"] = expect
        else:
            if result["status"] == "completed":
                step["within"] = within or 1e-9
            step["expect"] = {"revision": "same"}
            self.d.entities = before
        self.steps.append(step)
        return self

    def setup(self, edit_input, **more):
        self.execute(edit_input, **more)
        self.steps[-1]["command"] = "cad.constraints.edit"
        return self


def W(name, value):
    """The rectangle's width dimension named `name`."""
    return c({"kind": "distance", "a": pt(1, "start"), "b": pt(1, "end"), "parameter": name}, value=value)


def H(name, value):
    return c({"kind": "distance", "a": pt(2, "start"), "b": pt(2, "end"), "parameter": name}, value=value)


def rect(width="8", height="4", wname="genislik", hname="yukseklik"):
    return add(*rect_rules(), W(wname, width), H(hname, height))


def cases():
    out = []

    def case(name):
        k = ParamCase(name)
        out.append(k)
        return k

    def setting(name, **fields):
        return {"kind": "set", "name": name, **fields}

    case("Genişliğin ifadesi 10: dikdörtgen genişler").setup(rect()).params([setting("genislik", expression="10")]).undo("Parametreler")
    case("Kullanıcı parametresi eklenir, yükseklik onu okur").setup(rect()).params([
        {"kind": "add", "parameter": {"name": "oran", "kind": "number", "expression": "2"}},
        setting("yukseklik", expression="genislik / oran")])
    case("Birimli değer: 950cm").setup(rect()).params([setting("genislik", expression="950cm")])
    case("Yeniden adlandır: okuyan ifade ve ölçü izler").setup(rect(height="genislik / 2")).params([
        {"kind": "rename", "name": "genislik", "to": "en"}, setting("en", expression="12")])
    case("Okunmayan kullanıcı parametresi kaldırılır").setup(rect()).params([
        {"kind": "add", "parameter": {"name": "bos", "kind": "length", "expression": "3"}}]).params([{"kind": "remove", "name": "bos"}])
    case("Okunan parametre kaldırılamaz").setup(rect(height="genislik / 2")).params([{"kind": "remove", "name": "genislik"}])
    case("Ölçünün parametresi kaldırılamaz").setup(rect()).params([{"kind": "remove", "name": "yukseklik"}])
    case("Olmayan parametre").setup(rect()).params([setting("boy", expression="3")])
    case("Ad zaten var").setup(rect()).params([{"kind": "add", "parameter": {"name": "genislik", "kind": "length", "expression": "3"}}])
    case("Yeni ad zaten var").setup(rect()).params([{"kind": "rename", "name": "genislik", "to": "yukseklik"}])
    case("Yalnız ölçen ölçüye ifade verilmez").setup(add(*rect_rules(), W("genislik", "8"),
        c({"kind": "distance", "a": pt(2, "start"), "b": pt(2, "end"), "parameter": "yukseklik"}, reference=True))).params(
        [setting("yukseklik", expression="5")])
    case("Alt sınırın altında kalan değer").setup(rect()).params([setting("genislik", min={"kind": "to", "value": 9})])
    case("Sınırlar içinde değer, sonra sınır kaldırılır").setup(rect()).params([
        setting("genislik", min={"kind": "to", "value": 5}, max={"kind": "to", "value": 20}, expression="12")]).params(
        [setting("genislik", min={"kind": "clear"}, max={"kind": "clear"})])
    case("Parametreler birbirini okuyor").setup(rect()).params([
        setting("genislik", expression="yukseklik * 2"), setting("yukseklik", expression="genislik / 2")])
    case("İfade olmayan bir adı okuyor").setup(rect()).params([setting("genislik", expression="yok * 2")])
    case("İfade okunamıyor").setup(rect()).params([setting("genislik", expression="2 +")])
    case("Geçersiz ad").setup(rect()).params([{"kind": "add", "parameter": {"name": "2x", "kind": "length", "expression": "1"}}])
    case("İfadesiz kullanıcı parametresi").setup(rect()).params([{"kind": "add", "parameter": {"name": "x", "kind": "length", "expression": ""}}])
    case("Yalnız açıklama: geometri değişmez").setup(rect()).params([setting("genislik", description="Odanın içten genişliği")])
    case("Açı derecede: sol kenar 45°").setup(add(
        c({"kind": "fix", "a": seg(1)}), c({"kind": "coincident", "a": pt(4, "end"), "b": pt(1, "start")}),
        c({"kind": "distance", "a": pt(4, "start"), "b": pt(4, "end"), "parameter": "boy"}, value="4"),
        c({"kind": "angle", "a": seg(1), "b": seg(4), "parameter": "aci"}, value="60"))).params([setting("aci", expression="45")])
    case("Yarıçap sıfır olamaz").setup(add(c({"kind": "fix", "a": pt(5, "center")}),
        c({"kind": "radius", "a": circ(5), "parameter": "r"}, value="2"))).params([setting("r", expression="0")])
    case("Yarıçap 2.5: daire büyür").setup(add(c({"kind": "fix", "a": pt(5, "center")}),
        c({"kind": "radius", "a": circ(5), "parameter": "r"}, value="2"))).params([setting("r", expression="2.5")])
    k = case("Beklenen sürüm geçersiz")
    k.steps.append({"op": "execute", "input": {"edits": [setting("genislik", expression="3")], "expectedRevision": "07"},
                    "result": {"status": "failed", "error": {"code": "invalid_revision", "message": "$any", "path": "expectedRevision"}},
                    "expect": {"revision": "same"}})
    k = case("Beklenen sürüm geçmişte kaldı")
    k.steps.append({"op": "execute", "input": {"edits": [setting("genislik", expression="3")], "expectedRevision": "999"},
                    "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": "$any", "path": "expectedRevision",
                                                               "revision": "$current"}},
                    "expect": {"revision": "same"}})
    return out


def build():
    data = ccc.header("cad.parameters.edit", "Parametreler",
                      "ADR 0336 §1: parametreler adlarıyla eklenir, değiştirilir, yeniden adlandırılır ve kaldırılır; çizim yeni "
                      "değerlerle çözülür ve parametreler, oynayan nesnelerle tek adımda yazılır. Kısıtlar önce cad.constraints.edit "
                      "ile eklenir. Durumlar KentOS kodu olmadan scripts/fixtures/parameter_command_cases.py ile yazılır.")
    data["cases"] = [k.json() for k in cases()]
    return data


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            print(f"{OUT} güncel değil: python3 {sys.argv[0]}")
            sys.exit(1)
        print(f"{len(json.loads(text)['cases'])} durum güncel")
        return
    OUT.write_text(text)
    for k in json.loads(text)["cases"]:
        said = [f"{st['result']['status']} {st['result'].get('error', {}).get('code', '')}".strip() for st in k["steps"] if "result" in st]
        print(f"{k['name']}: {' → '.join(said)}")


if __name__ == "__main__":
    main()
