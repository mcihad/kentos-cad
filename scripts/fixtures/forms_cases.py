"""The forms' shared cases (docs/adr/0401), written here without KentOS code.

    python3 scripts/fixtures/forms_cases.py           # writes the files
    python3 scripts/fixtures/forms_cases.py --check   # writes nothing; compares

Writes
- fixtures/forms/v1/cases.json: the forms core (crates/shared/forms): a form's preflight, a record's items
  and issues, a write through the form, the write profile, the defaults and the form made of a layer's fields;
- fixtures/commands/v1/cad.forms.edit.json, cad.forms.validate.json, cad.forms.submit.json and
  cad.entities.set.forms.json: the commands (crates/native/application).

The rules are the ADR's, written anew here:
- a field item names its layer field by its key: the field's `id`, else its name;
- a rule is the expression language's (scripts/fixtures/expression_language.py evaluates it), over the record's
  attributes; a yes/no field reads true for `true`, false for `false`, empty otherwise; a number field reads
  its number, empty where it is none; the rest as text;
- an item is shown when its rule and every enclosing group's rule hold; a field is required when it, its layer
  field or its `requiredWhen` says so, and only while shown; read-only when it or its `readOnlyWhen` says so;
- a record is valid when every shown field keeps its layer field's rules (ADR 0199's single forms), a shown
  required field is filled and the checks of a shown, filled field hold; a hidden field keeps its value;
- a write through the form changes only the form's fields, none hidden or read-only afterwards;
- the preflight: the fields exist, once each, the widget takes the kind (a list widget a value list), the
  default keeps the field's rules, a required read-only field has a default, the rules compile, a field the
  layer requires is always shown and writable; warnings for a rule reading a name no field has or a field's
  visibility or lock reading the field itself, and for a form without a field.

The issues are compared by their code, weight, item, field and rule; their sentences are the core's.
"""
import copy
import hashlib
import importlib.util
import json
import re
import sys
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


# The language's independent evaluator and the project file's independent reader (the structural rules).
el = load("expression_language", ROOT / "scripts/fixtures/expression_language.py")
kc = load("kcad_reader", ROOT / "tools/kcad/kcad.py")
# The templates' independent splitting (docs/adr/0225 §6): a note's text with `{…}`.
tc = load("template_cases", ROOT / "scripts/fixtures/template_cases.py")
cc = load("chart_cases", ROOT / "scripts/fixtures/chart_cases.py")  # the natural order, ADR 0227
# The spatial relations' independent reference (docs/adr/0200 §1): a spatial list's (docs/adr/0439 §5).
sq = load("spatial_query_cases", ROOT / "scripts/fixtures/spatial_query_cases.py")
# A value as people read it (docs/adr/0199 §4): a list's columns (docs/adr/0439 §2).
lfc = load("layer_field_cases", ROOT / "scripts/fixtures/layer_field_cases.py")
display_value = lfc.display

# uzunluk(metin): the text's UTF-16 units, as JavaScript counts them.
el.FUNCTIONS["UZUNLUK"] = (1, 1, lambda args: None if args[0] is None else float(el.units(el.text(args[0]))))

JS_SPACE = kc.JS_SPACE
MAX_INTEGER = 9007199254740991
MAX_DIGITS = 30
MOST_NAME = 64


# ── A layer field's values (docs/adr/0199 §1) ───────────────────────────


def key_of(f):
    return f.get("id", f["name"])


# docs/adr/0439 §4: the kinds that compare with each other.
FAMILY = {"text": 0, "integer": 1, "decimal": 1, "date": 2, "datetime": 3, "time": 4, "boolean": 5}


def writes_into(src, dst):
    """A value of kind `src` into a field of kind `dst`: the same family (not a decimal into an integer), or
    anything into text."""
    return dst == "text" or (FAMILY[src] == FAMILY[dst] and not (src == "decimal" and dst == "integer"))


def fingerprint(values):
    """The copied values' fingerprint (docs/adr/0439 §3): FNV-1a, 32 bits, of their UTF-8 texts each ended by
    U+001F, an empty one as nothing; eight lower-case hexadecimal digits."""
    h = 0x811C9DC5
    for v in values:
        for b in (v or "").encode() + b"\x1f":
            h ^= b
            h = (h * 0x01000193) & 0xFFFFFFFF
    return f"{h:08x}"


def same_key(numeric, folded, k, w):
    """A list's value against the record's (docs/adr/0439 §4): numbers by value, text exactly or folded."""
    if numeric:
        return Fraction(k) == Fraction(w)
    if folded:
        return fold(k.strip(JS_SPACE)).lower() == fold(w.strip(JS_SPACE)).lower()
    return k == w


def fold(text):
    return kc._fold(text)


def signed(s):
    if s[:1] == "+":
        return False, s[1:]
    if s[:1] == "-":
        return True, s[1:]
    return False, s


def kind_value(f, s):
    """The kind's single form of trimmed `s`, or the refusal's code."""
    kind = f["kind"]
    if kind == "integer":
        minus, rest = signed(s)
        if not rest or not rest.isascii() or not rest.isdigit():
            return None, "type"
        rest = rest.lstrip("0")
        if not rest:
            return "0", None
        if len(rest) > 16 or int(rest) > MAX_INTEGER:
            return None, "magnitude"
        return ("-" if minus else "") + rest, None
    if kind == "decimal":
        minus, rest = signed(s)
        m = re.search(r"[.,]", rest)
        whole, fraction = (rest[: m.start()], rest[m.start() + 1:]) if m else (rest, "")
        ok = lambda t: t == "" or (t.isascii() and t.isdigit())  # noqa: E731
        if not ok(whole) or not ok(fraction) or (not whole and not fraction):
            return None, "type"
        if len(whole) + len(fraction) > MAX_DIGITS:
            return None, "magnitude"
        if "scale" in f and len(fraction) > f["scale"]:
            return None, "scale"
        whole = whole.lstrip("0") or "0"
        zero = whole == "0" and set(fraction) <= {"0"}
        return ("-" if minus and not zero else "") + whole + ("." + fraction if fraction else ""), None
    if kind == "date":
        import calendar

        m = re.fullmatch(r"([0-9]{4})-([0-9]{2})-([0-9]{2})", s)
        if m:
            y, mo, d = (int(x) for x in m.groups())
        else:
            parts = s.split(".")
            if len(parts) != 3 or not all(p.isascii() and p.isdigit() for p in parts):
                return None, "type"
            d_, mo_, y_ = parts
            if not (len(y_) == 4 and 1 <= len(mo_) <= 2 and 1 <= len(d_) <= 2):
                return None, "type"
            y, mo, d = int(y_), int(mo_), int(d_)
        if not (1 <= y <= 9999 and 1 <= mo <= 12 and 1 <= d <= calendar.monthrange(y, mo)[1]):
            return None, "type"
        return f"{y:04}-{mo:02}-{d:02}", None
    if kind == "boolean":
        k = fold(s)
        if k in ("evet", "true", "1"):
            return "true", None
        if k in ("hayır", "false", "0"):
            return "false", None
        return None, "type"
    return s, None


def check_value(f, text):
    """A value written to a field the form asks only while shown: its single form ("" for empty), or the
    refusal's code."""
    s = text.strip(JS_SPACE)
    if not s:
        return "", None
    if f["kind"] == "text" and "length" in f and len(text) > f["length"]:
        return None, "length"
    choices = f.get("values") or None
    if not choices and f.get("multiple") and f["kind"] == "text":
        # docs/adr/0409 §1: free values (ids of a many-to-many reference) in their order once each.
        seen = []
        for part in (p.strip(JS_SPACE) for p in s.split(";")):
            if part and part not in seen:
                seen.append(part)
        return "; ".join(seen), None
    if choices and f.get("multiple") and f["kind"] == "text":
        # docs/adr/0408 §1: labels or codes, in the list's order once each, joined with "; ".
        picked = set()
        for part in (p.strip(JS_SPACE) for p in s.split(";")):
            if not part:
                continue
            at = next((i for i, c in enumerate(choices) if fold(c["label"].strip(JS_SPACE)) == fold(part)), None)
            if at is None:
                at = next((i for i, c in enumerate(choices) if c["code"] == part), None)
            if at is None:
                return None, "choice"
            picked.add(at)
        return "; ".join(c["code"] for i, c in enumerate(choices) if i in picked), None
    if choices:
        wanted = fold(s)
        for c in choices:
            if fold(c["label"].strip(JS_SPACE)) == wanted:
                return c["code"], None
    canonical, why = kind_value(f, s)
    if why:
        return None, why
    value = s if f["kind"] == "text" and choices else canonical
    if choices and value not in [c["code"] for c in choices]:
        return None, "choice"
    if f["kind"] in ("integer", "decimal"):
        x = Fraction(value)
        # docs/adr/0438 §4: an end left out is outside too.
        if ("min" in f and (x < Fraction(f["min"]) or (f.get("minExclusive") and x == Fraction(f["min"])))) or (
                "max" in f and (x > Fraction(f["max"]) or (f.get("maxExclusive") and x == Fraction(f["max"])))):
            return None, "range"
    # docs/adr/0436: a date, a date and time or a time of day by its text, which sorts as written.
    if f["kind"] in ("date", "datetime", "time"):
        if ("min" in f and (value < f["min"] or (f.get("minExclusive") and value == f["min"]))) or (
                "max" in f and (value > f["max"] or (f.get("maxExclusive") and value == f["max"]))):
            return None, "range"
    return value, None


# ── Rules ───────────────────────────────────────────────────────────────

# The `@` values only a host gives (the moment, the user, the coordinate system's name), and of them the moment's and
# the user's: a command gives them; the document's following does not (docs/adr/0402 §5).
MOMENT = ("TARIH", "SIMDI", "KULLANICI")
HOST_ONLY = MOMENT + ("KOORDINAT_SISTEMI",)
CHOICE_CODE, CHOICE_LABEL = "secenek", "secenek_etiketi"


def tokenize(src):
    """The language's tokens with `$name` and `@name` read as names the scope holds (`$…`, `@KEY`)."""
    toks, i = [], 0
    while i < len(src):
        c = src[i]
        if c in "$@":
            j = i + 1
            if not (j < len(src) and (src[j].isalpha() or src[j] == "_")):
                raise el.Invalid(f"lone {c}")
            while j < len(src) and (src[j].isalnum() or src[j] == "_"):
                j += 1
            word = src[i + 1:j]
            toks.append(("field", "$" + el.key(word) if c == "$" else "@" + el.key(word)))
            i = j
            continue
        k, quote, bracket = i, None, False
        while k < len(src):
            ch = src[k]
            if quote:
                if ch == quote:
                    if k + 1 < len(src) and src[k + 1] == quote:
                        k += 1
                    else:
                        quote = None
            elif bracket:
                bracket = ch != "]"
            elif ch in "'\"":
                quote = ch
            elif ch == "[":
                bracket = True
            elif ch in "$@":
                break
            k += 1
        toks.extend(t for t in el.tokenize(src[i:k]) if t[0] != "end")
        i = k
    toks.append(("end", None))
    return toks


def parse(src):
    """The rule's tree, or None when it does not compile."""
    try:
        return el.Parser(tokenize(src)).parse()
    except el.Invalid:
        return None


def names(tree):
    """The names a tree reads, in order, each once (`@KEY` a variable, `$KEY` a geometry value)."""
    out = []

    def walk(n):
        if isinstance(n, tuple):
            if n[0] == "field":
                if n[1] not in out:
                    out.append(n[1])
                return
            for part in n[1:]:
                walk(part)
        elif isinstance(n, list):
            for part in n:
                walk(part)

    walk(tree)
    return out


def fields_read(tree):
    return [n for n in names(tree) if n[:1] not in ("@", "$")]


def variables_read(tree):
    return [n[1:] for n in names(tree) if n[:1] == "@"]


# An object as the cases give it: its measures in metres ($uzunluk, $alan, $köşe).
GEOMETRY = {"UZUNLUK": "length", "CEVRE": "length", "ALAN": "area", "KOSE": "vertices"}


def scope(fields, attrs, ctx=None):
    """The record as a rule reads it: each layer field by its kind, other attributes as text; the `@` values and
    the object's measures the context gives."""
    obj = dict(attrs)
    for f in fields:
        v = attrs.get(f["name"])
        if f["kind"] == "boolean":
            obj[f["name"]] = True if v == "true" else False if v == "false" else None
        elif f["kind"] in ("integer", "decimal"):
            obj[f["name"]] = None if v is None else el.read_number(v)
    ctx = ctx or {}
    for name, value in ctx.get("variables", {}).items():
        obj["@" + el.key(name)] = value
    for word, what in GEOMETRY.items():
        if what in ctx.get("object", {}):
            obj["$" + word] = float(ctx["object"][what])
    return obj


def value_of(src, fields, attrs, ctx=None):
    return el.value(parse(src), scope(fields, attrs, ctx))


def holds(src, fields, attrs, ctx=None):
    return el.truthy(value_of(src, fields, attrs, ctx))


def text_for(v, f):
    """A value written into a field: a yes/no as `true` or `false`, a number as the language writes it, empty as
    None (docs/adr/0402 §3)."""
    if el.is_empty(v):
        return None
    if isinstance(v, bool):
        return "true" if v else "false"
    return el.text(v)


# ── The form ────────────────────────────────────────────────────────────

FITS = {
    "text": {"text", "textArea", "list", "radio", "segmented", "color", "checkList", "search", "chips", "tree"},
    "integer": {"number", "list", "radio", "segmented", "slider", "search", "tree"},
    "decimal": {"number", "list", "radio", "segmented", "slider", "search", "tree"},
    "date": {"date"},
    "boolean": {"check", "switch", "radio", "segmented"},
}


def widget_of(item, f):
    if "widget" in item:
        return item["widget"]
    if f.get("multiple") and (f.get("values") or "lookup" in item):
        return "checkList"
    if f.get("values"):
        return "list"
    return {"integer": "number", "decimal": "number", "date": "date", "boolean": "check"}.get(
        f["kind"], "textArea" if f.get("length", 0) > 200 else "text")


def is_colour(value):
    """docs/adr/0407 §2: `#RRGGBB` or `#RRGGBBAA`."""
    v = value.strip()
    return v.startswith("#") and len(v) in (7, 9) and all(c in "0123456789abcdefABCDEF" for c in v[1:])


def counted(obj, accept):
    """docs/adr/0407 §1: what an attachments item counts of the object's attachments; None where they are not judged."""
    held = (obj or {}).get("attachments")
    if held is None:
        return None
    return {"photo": held["photos"], "document": held["all"] - held["photos"]}.get(accept, held["all"])


def issue(code, error, item=None, field=None, rule=None):
    out = {"code": code, "severity": "error" if error else "warning"}
    for k, v in (("item", item), ("field", field), ("rule", rule)):
        if v is not None:
            out[k] = v
    return out


def known(variables):
    return {el.key(v["name"]) for v in variables}


def template_exprs(text):
    """A note's template: its expressions' sources, or None when it does not split."""
    t = tc.parse(text)
    if "error" in t:
        return None
    return [p["text"] for p in t["pieces"] if p["kind"] == "expr"]


def template_plain(text):
    t = tc.parse(text)
    return all(p["kind"] == "text" for p in t["pieces"])


def fill(text, values):
    out, k = [], 0
    for p in tc.parse(text)["pieces"]:
        if p["kind"] == "text":
            out.append(p["text"])
        elif p["kind"] == "expr":
            v = values[k] if k < len(values) else None
            if v is not None:
                out.append(v)
            k += 1
    return "".join(out)


UID_KEY = "$uid"


def preflight(body, fields, variables=(), world=None):
    """`world`: the other layers by id, each {"name", "fields", "rows": [[uid, attrs], …]} (docs/adr/0404)."""
    world = world or {}
    problem = kc.body_problem(body)
    if problem:
        return [issue("form_body_invalid", True)]
    issues, bound, calcs, mirrors = [], [], [], []
    have = known(variables)

    def rule(item, key, itself, at, src, extra=()):
        tree = parse(src)
        if tree is None:
            issues.append(issue("form_rule_invalid", True, item, key, at))
            return None
        for name in fields_read(tree):
            if not any(f["name"] == name for f in fields):
                issues.append(issue("form_rule_unknown_name", False, item, key, at))
            elif name == itself:
                issues.append(issue("form_rule_reads_itself", False, item, key, at))
        for v in variables_read(tree):
            if v not in have and v not in extra and v not in MOMENT:
                issues.append(issue("form_rule_unknown_variable", False, item, key, at))
        return tree

    WRITES = ("readOnly", "required", "requiredWhen", "readOnlyWhen", "default", "defaultExpr", "calculate", "choiceFilter",
              "unique", "reuseLast", "overridable", "suggest", "lookup", "checks")

    def field(f, hideable):
        key = f["field"]
        # docs/adr/0437: a mirror shows a field the form writes elsewhere; only its label, help, widget and rule.
        if f.get("mirror"):
            mirrors.append((f["id"], key))
            if not any(key_of(x) == key for x in fields):
                issues.append(issue("form_field_unknown", True, f["id"], key))
            if any(f.get(k) for k in WRITES):
                issues.append(issue("form_mirror_rules", True, f["id"], key))
            if "visibleWhen" in f:
                rule(f["id"], key, None, "visibleWhen", f["visibleWhen"])
            return
        if any(b[0] == key for b in bound):
            issues.append(issue("form_field_twice", True, f["id"], key))
        always = not hideable and "visibleWhen" not in f and (
            "calculate" in f or (not f.get("readOnly") and "readOnlyWhen" not in f))
        bound.append((key, f["id"], always))
        layer = next((x for x in fields if key_of(x) == key), None)
        if layer is None:
            issues.append(issue("form_field_unknown", True, f["id"], key))
            return
        w = widget_of(f, layer)
        listed = bool(layer.get("values")) or "lookup" in f
        if layer.get("multiple"):
            many_fits = w in ("checkList", "chips") if listed else w in ("text", "textArea", "checkList", "chips")
        else:
            many_fits = w not in ("checkList", "chips")
        if w not in FITS[layer["kind"]] or not many_fits:
            issues.append(issue("form_widget_unfit", True, f["id"], key))
        elif w == "slider" and ("min" not in layer or "max" not in layer):
            issues.append(issue("form_widget_needs_range", True, f["id"], key))
        elif w in ("list", "radio", "segmented", "checkList", "search", "chips", "tree") and not layer.get("values") \
                and layer["kind"] != "boolean" \
                and "lookup" not in f:
            issues.append(issue("form_widget_needs_values", True, f["id"], key))
        if "default" in f and check_value(layer, f["default"])[1]:
            issues.append(issue("form_default_invalid", True, f["id"], key, "default"))
        elif "default" in f and w == "color" and not is_colour(f["default"]):
            issues.append(issue("form_default_invalid", True, f["id"], key, "default"))
        if f.get("suggest") and (layer["kind"] != "text" or layer.get("values") or "lookup" in f):
            issues.append(issue("form_suggest_unfit", True, f["id"], key, "suggest"))
        if "default" in f and "defaultExpr" in f:
            issues.append(issue("form_rule_conflict", True, f["id"], key, "defaultExpr"))
        if "calculate" in f and ("default" in f or "defaultExpr" in f or "readOnlyWhen" in f):
            issues.append(issue("form_rule_conflict", True, f["id"], key, "calculate"))
        if f.get("overridable") and "calculate" not in f:
            issues.append(issue("form_rule_conflict", True, f["id"], key, "overridable"))
        if "choiceFilter" in f and not layer.get("values"):
            issues.append(issue("form_choice_unlisted", True, f["id"], key, "choiceFilter"))
        if "lookup" in f:
            lookup(f, layer)
        if f.get("readOnly") and f.get("required") and not any(k in f for k in ("default", "defaultExpr", "calculate")):
            issues.append(issue("form_required_read_only", True, f["id"], key))
        for at in ("visibleWhen", "requiredWhen", "readOnlyWhen"):
            if at in f:
                rule(f["id"], key, None if at == "requiredWhen" else layer["name"], at, f[at])
        for i, c in enumerate(f.get("checks", [])):
            rule(f["id"], key, None, f"checks/{i}", c["rule"])
        for at in ("defaultExpr", "calculate"):
            if at not in f:
                continue
            tree = rule(f["id"], key, None, at, f[at])
            if tree is not None and any(v in MOMENT for v in variables_read(tree)):
                issues.append(issue("form_moment_only", False, f["id"], key, at))
            if at == "calculate" and tree is not None:
                calcs.append((f["id"], key, layer["name"], fields_read(tree)))
        if "choiceFilter" in f:
            rule(f["id"], key, None, "choiceFilter", f["choiceFilter"], extra=(el.key(CHOICE_CODE), el.key(CHOICE_LABEL)))

    def lookup(f, layer):
        lk, key = f["lookup"], f["field"]
        if layer.get("values") or "choiceFilter" in f:
            issues.append(issue("form_rule_conflict", True, f["id"], key, "lookup"))
        other = world.get(lk["layer"])
        if other is None:
            issues.append(issue("form_lookup_layer", True, f["id"], key, "lookup"))
            return
        has = lambda k: any(key_of(x) == k for x in other["fields"])  # noqa: E731
        for k in [lk["key"], lk.get("label")]:
            if k is not None and k != UID_KEY and not has(k):
                issues.append(issue("form_lookup_field", True, f["id"], key, "lookup"))
        for m in lk.get("matches", []):
            if not has(m["field"]):
                issues.append(issue("form_lookup_field", True, f["id"], key, "lookup"))
        for m in lk.get("matches", []):
            if not any(key_of(x) == m["record"] for x in fields):
                issues.append(issue("form_lookup_field", True, f["id"], key, "lookup"))
        if "filter" in lk and parse(lk["filter"]) is None:
            issues.append(issue("form_rule_invalid", True, f["id"], key, "lookup.filter"))
        lookup_extras(f, layer, lk, other)

    def lookup_extras(f, layer, lk, other):
        """docs/adr/0439: the order's, columns' and copies' fields exist in the list's layer, the copies' and
        the stamp's in the form's; the kinds compared, written and copied go together; the numbers in bounds."""
        key = f["field"]

        def bad(code):
            issues.append(issue(code, True, f["id"], key, "lookup"))

        theirs = lambda k: next((x for x in other["fields"] if key_of(x) == k), None)  # noqa: E731
        ours = lambda k: next((x for x in fields if key_of(x) == k), None)  # noqa: E731
        read = list(lk.get("columns", [])) + ([lk["sort"]["field"]] if "sort" in lk else []) + \
            [c["from"] for c in lk.get("copy", [])]
        for k in read:
            if k != UID_KEY and theirs(k) is None:
                bad("form_lookup_field")
        for k in [c["to"] for c in lk.get("copy", [])] + ([lk["stamp"]] if "stamp" in lk else []):
            if ours(k) is None:
                bad("form_lookup_field")

        def kind_of(x, k):
            return "text" if k == UID_KEY else (x["kind"] if x else None)

        k = kind_of(theirs(lk["key"]), lk["key"])
        if k is not None and not writes_into(k, layer["kind"]):
            bad("form_lookup_type")
        for m in lk.get("matches", []):
            a, b = theirs(m["field"]), ours(m["record"])
            if a and b and FAMILY[a["kind"]] != FAMILY[b["kind"]]:
                bad("form_lookup_type")
        for c in lk.get("copy", []):
            k, b = kind_of(theirs(c["from"]), c["from"]), ours(c["to"])
            if k is not None and b is not None and not writes_into(k, b["kind"]):
                bad("form_lookup_type")
            if c["to"] == key or c["to"] == lk.get("stamp"):
                bad("form_lookup_type")
        s = ours(lk["stamp"]) if "stamp" in lk else None
        if s is not None and s["kind"] != "text":
            bad("form_lookup_type")
        if len(lk.get("columns", [])) > 4:
            bad("form_lookup_type")
        if "limit" in lk and not 0 < lk["limit"] <= 1000:
            bad("form_lookup_type")
        tol = lk.get("spatial", {}).get("tolerance")
        if tol is not None and tol < 0:
            bad("form_lookup_type")

    def related(x):
        if "visibleWhen" in x:
            rule(x["id"], None, None, "visibleWhen", x["visibleWhen"])
        other = world.get(x["layer"])
        if other is None:
            issues.append(issue("form_lookup_layer", True, x["id"], None, "related"))
        elif not any(key_of(y) == x["field"] for y in other["fields"]):
            issues.append(issue("form_lookup_field", True, x["id"], None, "related"))
        if other is not None:
            # docs/adr/0425: the table's columns are the other layer's fields.
            for c in x.get("columns", []):
                if not any(key_of(y) == c for y in other["fields"]):
                    issues.append(issue("form_lookup_field", True, x["id"], None, "related"))

    def note(x):
        sources = template_exprs(x["text"])
        if sources is None:
            issues.append(issue("form_rule_invalid", True, x["id"], None, "text"))
            return
        for src in sources:
            rule(x["id"], None, None, "text", src)

    def items(xs, hideable):
        for x in xs:
            if x["type"] == "tab":
                # docs/adr/0426: a tab's own visibility rule.
                if "visibleWhen" in x:
                    rule(x["id"], None, None, "visibleWhen", x["visibleWhen"])
                items(x["items"], hideable or "visibleWhen" in x)
            elif x["type"] == "group":
                if "visibleWhen" in x:
                    rule(x["id"], None, None, "visibleWhen", x["visibleWhen"])
                items(x["items"], hideable or "visibleWhen" in x)
            elif x["type"] == "field":
                field(x, hideable)
            elif x["type"] == "related":
                related(x)
            elif x["type"] in ("attachments", "location", "divider", "action"):
                if "visibleWhen" in x:
                    rule(x["id"], None, None, "visibleWhen", x["visibleWhen"])
            else:
                note(x)

    items(body["items"], False)
    for item, key in mirrors:
        if not any(b[0] == key for b in bound):
            issues.append(issue("form_mirror_alone", True, item, key))
    # Calculated fields reading each other in a circle: the first of the circle in the form's order.
    done = [False] * len(calcs)
    while True:
        nxt = next((k for k in range(len(calcs)) if not done[k]
                    and all(done[d] for d in range(len(calcs)) if calcs[d][2] in calcs[k][3])), None)
        if nxt is None:
            break
        done[nxt] = True
    stuck = next((k for k in range(len(calcs)) if not done[k]), None)
    if stuck is not None:
        issues.append(issue("form_calc_cycle", True, calcs[stuck][0], calcs[stuck][1], "calculate"))
    # docs/adr/0442 §5: a step action's step, the workflow's.
    steps = [s["label"] for s in body.get("workflow", {}).get("steps", [])]

    def actions(xs):
        for x in xs:
            if x["type"] == "action" and x.get("kind") == "step" and x.get("step") not in steps:
                issues.append(issue("form_action_step", True, x["id"], None, "step"))
            actions(x.get("items", []))
    actions(body["items"])
    if not bound:
        issues.append(issue("form_empty", False))
    for f in fields:
        if not f.get("required"):
            continue
        reach = next((b for b in bound if b[0] == key_of(f)), None)
        if reach is None:
            issues.append(issue("form_required_hidden", True, None, key_of(f)))
        elif not reach[2]:
            issues.append(issue("form_required_hidden", True, reach[1], key_of(f)))
    return issues


def errors(issues):
    return [i for i in issues if i["severity"] == "error"]


def empty(attrs, name):
    return not attrs.get(name, "").strip(JS_SPACE)


class Form:
    """A form prepared over its layer's fields, with the `@` values given."""

    ROUNDS = 3

    def __init__(self, body, fields, variables=(), world=None, rules=()):
        world = world or {}
        assert not errors(preflight(body, fields, variables, world)), "a form with errors is not prepared"
        self.fields = fields
        self.variables = {v["name"]: v["value"] for v in variables}
        # docs/adr/0438 §1: the layer's record rules, shown on their fields; §5: the day a value's validity is
        # judged on, the rules' @tarih.
        self.rules = list(rules)
        today = self.variables.get("tarih")
        self.today = str(today)[:10] if today not in (None, "") else None
        self.clear_hidden = bool(body.get("clearHidden"))
        # docs/adr/0432: a record in a draft state of the workflow keeps the form's own requirements as warnings.
        self.workflow = body.get("workflow")
        self.nodes = []

        def walk(xs, parent):
            for x in xs:
                at = len(self.nodes)
                self.nodes.append((x, parent))
                walk(x.get("items", []), at)

        walk(body["items"], None)
        # The lists from other layers, made once (docs/adr/0404 §1), and the related items' objects.
        still = {n: v for n, v in self.variables.items() if n not in MOMENT}
        self.unit = 1  # the cases' geometry speaks metres
        self.lists, self.related = {}, {}
        for x, _ in self.nodes:
            if x["type"] == "field" and "lookup" in x:
                self.lists[x["id"]] = self.list_of(x["lookup"], world[x["lookup"]["layer"]], still)
            elif x["type"] == "related":
                other = world[x["layer"]]
                name = next(y["name"] for y in other["fields"] if key_of(y) == x["field"])
                self.related[x["id"]] = [(uid, a[name].strip(JS_SPACE)) for uid, a in other["rows"]
                                         if a.get(name, "").strip(JS_SPACE)]
        # The calculated items in their order: each after the calculated fields it reads, else the form's.
        calcs = [k for k, (x, _) in enumerate(self.nodes) if x["type"] == "field" and "calculate" in x]
        reads = {k: fields_read(parse(self.nodes[k][0]["calculate"])) for k in calcs}
        done, self.calc_order = set(), []
        while len(self.calc_order) < len(calcs):
            k = next(k for k in calcs if k not in done
                     and all(d in done for d in calcs if self.layer(self.nodes[d][0])["name"] in reads[k]))
            done.add(k)
            self.calc_order.append(k)

    def list_of(self, lk, other, still):
        """A list (docs/adr/0404 §1, docs/adr/0439): rows (code, label, keys, columns, copies, shape), sorted; the
        chain's links (record field's name, numeric, folded); the codes more than one object holds."""
        import functools

        def field(k):
            return next((y for y in other["fields"] if key_of(y) == k), None)

        def name(k):
            y = field(k)
            return y["name"] if y else None

        def val(uid, attrs, k):
            v = uid if k == UID_KEY else attrs.get(name(k) or "", "")
            v = v.strip(JS_SPACE)
            return v or None

        def column(uid, attrs, k):
            if k == UID_KEY:
                return uid
            v = val(uid, attrs, k)
            return "" if v is None else display_value(field(k), v)

        # The server's world reads no shapes: a spatial list is not narrowed there.
        shapes = other.get("shapes")
        spatial = lk.get("spatial") if shapes is not None and len(shapes) == len(other["rows"]) else None
        rows, seen, counts = [], set(), {}
        for i, (uid, attrs) in enumerate(other["rows"]):
            code = val(uid, attrs, lk["key"])
            if code is None:
                continue
            if "filter" in lk and not holds(lk["filter"], other["fields"], attrs, {"variables": still, "object": {}}):
                continue
            counts[code] = counts.get(code, 0) + 1
            if code in seen:
                continue
            seen.add(code)
            label = (val(uid, attrs, lk["label"]) if "label" in lk else None) or code
            keys = [val(uid, attrs, m["field"]) for m in lk.get("matches", [])]
            columns = [column(uid, attrs, k) for k in lk.get("columns", [])]
            copies = [val(uid, attrs, c["from"]) for c in lk.get("copy", [])]
            shape = shapes[i] if spatial else None
            by = val(uid, attrs, lk["sort"]["field"]) if "sort" in lk else None
            rows.append(((code, label, keys, columns, copies, shape), by))

        def natural(a, b):
            return cc.natural_cmp(a[1], b[1]) or ((a[0] > b[0]) - (a[0] < b[0]))

        if "sort" in lk:
            sf = field(lk["sort"]["field"])
            numbers = sf is not None and sf["kind"] in ("integer", "decimal")
            plain = sf is not None and sf["kind"] in ("date", "datetime", "time")
            down = bool(lk["sort"].get("descending"))

            def by_field(a, b):
                (ra, va), (rb, vb) = a, b
                if va is None or vb is None:
                    o = (va is None) - (vb is None)
                else:
                    if numbers:
                        x, y = Fraction(va), Fraction(vb)
                    else:
                        x, y = va, vb
                    if numbers or plain:
                        o = (x > y) - (x < y)
                    else:
                        o = cc.natural_cmp(va, vb)
                    o = -o if down else o
                return o or natural(ra, rb)

            rows.sort(key=functools.cmp_to_key(by_field))
        else:
            rows.sort(key=functools.cmp_to_key(lambda a, b: natural(a[0], b[0])))
        chain = []
        for m in lk.get("matches", []):
            ours = next((f for f in self.fields if key_of(f) == m["record"]), None)
            if ours is None:
                continue
            theirs = field(m["field"])
            numeric = ours["kind"] in ("integer", "decimal") and theirs is not None and \
                theirs["kind"] in ("integer", "decimal")
            chain.append((ours["name"], numeric, bool(m.get("fold"))))
        unit = self.unit
        tol = (spatial.get("tolerance") or 0) / unit if spatial else 0
        return {"rows": [r for r, _ in rows], "chain": chain, "dups": {c: n for c, n in counts.items() if n > 1},
                "lookup": lk, "layer": other["name"], "spatial": spatial, "tolerance": tol,
                "copyTo": [next(f["name"] for f in self.fields if key_of(f) == c["to"]) for c in lk.get("copy", [])],
                "stamp": next((f["name"] for f in self.fields if key_of(f) == lk["stamp"]), None) if "stamp" in lk else None,
                "columns": [(field(k)["alias"] if field(k) and field(k).get("alias") else (field(k)["name"] if field(k) else "Kimlik"))
                            for k in lk.get("columns", [])]}

    def offered(self, x, attrs, obj=None):
        lst = self.lists[x["id"]]
        wanted = [attrs.get(n, "").strip(JS_SPACE) or None for n, _, _ in lst["chain"]]
        if any(w is None for w in wanted):
            return []
        rows = [r for r in lst["rows"]
                if all(k is not None and same_key(num, fo, k, w) for k, w, (_, num, fo) in zip(r[2], wanted, lst["chain"]))]
        spatial = lst["spatial"]
        if not spatial:
            return rows
        record = (obj or {}).get("shape")
        if record is None:
            return []
        tol, rel = lst["tolerance"], spatial["relation"]
        if rel == "nearest":
            best = None
            for r in rows:
                if r[5] is None:
                    continue
                d = sq.distance(r[5], record)
                if (tol > 0 and d > tol) or (best is not None and d >= best[0]):
                    continue
                best = (d, r)
            return [best[1]] if best else []
        out = []
        for r in rows:
            s = r[5]
            if s is None:
                continue
            if rel == "contains":
                ok = sq.contains(s, record)
            elif rel == "within":
                ok = sq.contains(record, s)
            elif tol > 0:
                ok = sq.distance(s, record) <= tol
            else:
                ok = sq.intersects(s, record)
            if ok:
                out.append(r)
        return out

    def code_of(self, x, text):
        rows = self.lists[x["id"]]["rows"]
        t = text.strip(JS_SPACE)
        if any(r[0] == t for r in rows):
            return t
        return next((r[0] for r in rows if r[1] == t), t)

    def copied(self, before, attrs):
        """docs/adr/0439 §3: each list whose value was chosen anew (differs from `before`'s; none: a new record), or
        whose stamp names another value, copies the chosen object's fields and stamps the record."""
        out = dict(attrs)
        for x, _ in self.bound():
            lst = self.lists.get(x["id"]) if "lookup" in x else None
            if lst is None or not (lst["copyTo"] or lst["stamp"]):
                continue
            f = self.layer(x)
            if f.get("multiple"):
                continue
            now = attrs.get(f["name"], "").strip(JS_SPACE)
            if not now:
                continue
            was = (before or {}).get(f["name"], "").strip(JS_SPACE)
            row = next((r for r in lst["rows"] if r[0] == now), None)
            stamped = lst["stamp"] is not None and (out.get(lst["stamp"]) is None
                                                     or (out[lst["stamp"]].split(" · ") + [None, None])[1] != now)
            if before is not None and was == now and not stamped:
                continue
            if row is None:
                continue
            for to, v in zip(lst["copyTo"], row[4]):
                if v is None:
                    out.pop(to, None)
                else:
                    out[to] = v
            if lst["stamp"] is not None:
                fp = fingerprint(row[4])
                held = out.get(lst["stamp"])
                parts = held.split(" · ") if held is not None else []
                if not (len(parts) >= 3 and parts[1] == now and parts[2] == "#" + fp):
                    text = f"{lst['layer']} · {now} · #{fp}"
                    if self.today:
                        text += " · " + self.today
                    out[lst["stamp"]] = text
        return out

    def copy_targets(self):
        out = set()
        for x, _ in self.bound():
            lst = self.lists.get(x["id"]) if "lookup" in x else None
            if lst:
                out.update(lst["copyTo"])
                if lst["stamp"]:
                    out.add(lst["stamp"])
        return out

    def ctx(self, obj, extra=None):
        v = dict(self.variables)
        v.update(extra or {})
        return {"variables": v, "object": obj or {}}

    def waits(self, src):
        """Whether an expression reads a value only a host gives, not given."""
        have = known([{"name": n} for n in self.variables])
        return any(v in HOST_ONLY and v not in have for v in variables_read(parse(src)))

    def layer(self, x):
        return next(f for f in self.fields if key_of(f) == x["field"])

    def bound(self):
        return [(x, p) for x, p in self.nodes if x["type"] == "field" and not x.get("mirror")]

    def calculated(self, key):
        """Calculated and locked: an overridable calculation (docs/adr/0417) leaves the field open."""
        return any(x["field"] == key and "calculate" in x and not x.get("overridable") for x, _ in self.bound())

    def states(self, attrs, obj=None):
        shown, out = [], []
        for x, parent in self.nodes:
            rule = x.get("visibleWhen") if x["type"] in ("tab", "group", "field", "related", "attachments", "location",
                                                         "divider", "action") \
                else None
            here = (parent is None or shown[parent]) and (rule is None or holds(rule, self.fields, attrs, self.ctx(obj)))
            shown.append(here)
            state = {"id": x["id"], "shown": here}
            if x["type"] == "field" and not x.get("mirror"):
                f = self.layer(x)
                if here and (x.get("required") or f.get("required")
                             or ("requiredWhen" in x and holds(x["requiredWhen"], self.fields, attrs, self.ctx(obj)))):
                    state["required"] = True
                if x.get("readOnly") or ("calculate" in x and not x.get("overridable")) or (
                        self.workflow and key_of(f) == self.workflow["field"]) or (
                        "readOnlyWhen" in x and holds(x["readOnlyWhen"], self.fields, attrs, self.ctx(obj))):
                    state["readOnly"] = True
            out.append(state)
        return out

    def choices(self, x, attrs, obj=None):
        if "lookup" in x:
            return [r[0] for r in self.offered(x, attrs, obj)]
        if "choiceFilter" not in x:
            return None
        f = self.layer(x)
        value = attrs.get(f["name"])
        held = ([p.strip() for p in value.split(";")] if f.get("multiple") else [value]) if value is not None else []
        # A retired value is offered only to the record that holds it (docs/adr/0418).
        return [c["code"] for c in f.get("values", [])
                if (not withdrawn(c, self.today) or c["code"] in held)
                and holds(x["choiceFilter"], self.fields, attrs,
                          self.ctx(obj, {CHOICE_CODE: c["code"], CHOICE_LABEL: c["label"]}))]

    def described(self, attrs, obj=None):
        states = self.states(attrs, obj)
        for (x, _), state in zip(self.nodes, states):
            if x["type"] == "field" and not x.get("mirror"):
                c = self.choices(x, attrs, obj)
                if c is not None:
                    state["choices"] = c
                if "lookup" in x:
                    # docs/adr/0439: a value's columns its description, a page and the rest's count, the chosen
                    # object's columns.
                    lst = self.lists[x["id"]]
                    options = []
                    for r in self.offered(x, attrs, obj):
                        o = {"code": r[0], "label": r[1]}
                        shown = [c for c in r[3] if c]
                        if shown:
                            o["description"] = " · ".join(shown)
                        options.append(o)
                    limit = lst["lookup"].get("limit")
                    if limit is not None and len(options) > limit:
                        state["more"] = len(options) - limit
                        options = options[:limit]
                    state["options"] = options
                    v = attrs.get(self.layer(x)["name"], "").strip(JS_SPACE)
                    row = next((r for r in lst["rows"] if r[0] == v), None) if v else None
                    if row is not None and lst["columns"]:
                        state["preview"] = [{"label": lab, "value": val} for lab, val in zip(lst["columns"], row[3])]
            elif x["type"] == "attachments":
                n = counted(obj, x.get("accept", "any"))
                if n is not None:
                    state["attachments"] = n
            elif x["type"] == "related" and state["shown"]:
                # docs/adr/0440 §6: a hidden item's related objects are not looked for.
                uid = (obj or {}).get("uid")
                state["related"] = [own for own, held in self.related[x["id"]]
                                    if uid is not None and uid in [h.strip(JS_SPACE) for h in held.split(";")]]
            elif x["type"] == "note" and not template_plain(x["text"]):
                values = [text_or_none(value_of(src, self.fields, attrs, self.ctx(obj)))
                          for src in template_exprs(x["text"])]
                state["text"] = fill(x["text"], values)
        return states

    def complete(self, attrs, obj=None, new=False):
        out, issues = dict(attrs), []
        if new:
            # docs/adr/0428 §2: a new record starts in the workflow's initial state.
            if self.workflow:
                sf = next(f for f in self.fields if key_of(f) == self.workflow["field"])
                if empty(out, sf["name"]):
                    out[sf["name"]] = self.workflow["initial"]
            for x, _ in self.bound():
                f = self.layer(x)
                if "default" in x and empty(out, f["name"]):
                    v, why = check_value(f, x["default"])
                    if not why and v:
                        out[f["name"]] = v
            for x, _ in self.bound():
                if "defaultExpr" not in x:
                    continue
                f = self.layer(x)
                if self.waits(x["defaultExpr"]) or not empty(out, f["name"]):
                    continue
                text = text_for(value_of(x["defaultExpr"], self.fields, out, self.ctx(obj)), f)
                if text is None:
                    continue
                v, why = check_value(f, text)
                if why:
                    issues.append(issue("form_default_invalid", False, x["id"], key_of(f), "defaultExpr"))
                elif v:
                    out[f["name"]] = v
        for rnd in range(self.ROUNDS):
            start, round_issues = dict(out), []
            shown = self.states(out, obj) if self.clear_hidden else None
            for k in self.calc_order:
                x = self.nodes[k][0]
                if shown is not None and not shown[k]["shown"]:
                    continue
                if self.waits(x["calculate"]):
                    continue
                f = self.layer(x)
                # An overridable calculation fills the field only while it is empty.
                if x.get("overridable") and out.get(f["name"]):
                    continue
                text = text_for(value_of(x["calculate"], self.fields, out, self.ctx(obj)), f)
                if text is None:
                    out.pop(f["name"], None)
                    continue
                v, why = check_value(f, text)
                if why:
                    round_issues.append(issue("form_calc_invalid", True, x["id"], key_of(f), "calculate"))
                elif v:
                    out[f["name"]] = v
                else:
                    out.pop(f["name"], None)
            if self.clear_hidden:
                for (x, _), state in zip(self.nodes, self.states(out, obj)):
                    if x["type"] == "field" and not x.get("mirror") and not state["shown"]:
                        out.pop(self.layer(x)["name"], None)
            if out == start or not self.clear_hidden:
                return out, issues + round_issues
            if rnd + 1 == self.ROUNDS:
                return out, issues + round_issues + [issue("form_unstable", True)]
        return out, issues

    def in_draft(self, attrs):
        wf = self.workflow
        if not wf or not wf.get("drafts"):
            return False
        state_field = next(f for f in self.fields if key_of(f) == wf["field"])
        return (attrs.get(state_field["name"], "").strip(JS_SPACE) or wf["initial"]) in wf["drafts"]

    def asked(self, on, moment):
        """docs/adr/0438 §2: a check without a moment always; one kept for saving on saving and on a step; one kept
        for the workflow's step on a step, and on saving when the form has no workflow."""
        if on is None or moment == "submit":
            return True
        if moment == "change":
            return False
        return on == "save" or self.workflow is None

    def validate(self, attrs, obj=None, moment="save"):
        issues = []
        draft = self.in_draft(attrs)
        for (x, _), state in zip(self.nodes, self.states(attrs, obj)):
            if x["type"] == "attachments" and state["shown"]:
                n = counted(obj, x.get("accept", "any"))
                if n is not None and "min" in x and n < x["min"]:
                    issues.append(issue("form_attachments_few", True, x["id"], None, "min"))
                elif n is not None and "max" in x and n > x["max"]:
                    issues.append(issue("form_attachments_many", True, x["id"], None, "max"))
            if x["type"] != "field" or x.get("mirror") or not state["shown"]:
                continue
            f = self.layer(x)
            value = attrs.get(f["name"], "").strip(JS_SPACE)
            if not value:
                if state.get("required"):
                    issues.append(issue("form_required", not draft or bool(f.get("required")), x["id"], key_of(f)))
                continue
            canonical, why = check_value(f, value)
            if why:
                issues.append(issue("form_value_invalid", True, x["id"], key_of(f)))
                continue
            if widget_of(x, f) == "color" and not is_colour(value):
                issues.append(issue("form_color_invalid", True, x["id"], key_of(f)))
                continue
            held = [p.strip(JS_SPACE) for p in canonical.split(";") if p.strip(JS_SPACE)] if f.get("multiple") else [canonical]
            if "lookup" in x and any(h not in [r[0] for r in self.offered(x, attrs, obj)] for h in held):
                issues.append(issue("form_lookup_missing", True, x["id"], key_of(f), "lookup"))
                continue
            if "lookup" in x:
                lst = self.lists[x["id"]]
                policy = lst["lookup"].get("duplicates", "warn")
                dup = next((h for h in held if h in lst["dups"]), None)
                if policy != "first" and dup is not None:
                    issues.append(issue("form_lookup_duplicate", policy == "refuse", x["id"], key_of(f), "lookup"))
                    if policy == "refuse":
                        continue
                # docs/adr/0439 §3: the copy's source changed after the copy.
                if lst["stamp"] is not None and not f.get("multiple") and lst["stamp"] in attrs:
                    parts = attrs[lst["stamp"]].split(" · ")
                    row = next((r for r in lst["rows"] if r[0] == canonical), None)
                    if row is not None and len(parts) >= 3 and parts[1] == canonical \
                            and parts[2] != "#" + fingerprint(row[4]):
                        issues.append(issue("form_lookup_stale", False, x["id"], key_of(f), "lookup.stamp"))
            offered = None if "lookup" in x else self.choices(x, attrs, obj)
            codes = [p.strip(JS_SPACE) for p in canonical.split(";") if p.strip(JS_SPACE)] if f.get("multiple") else [canonical]
            listed = [c["code"] for c in f.get("values", [])]
            if offered is not None and any(c in listed and c not in offered for c in codes):
                issues.append(issue("form_choice_filtered", True, x["id"], key_of(f), "choiceFilter"))
                continue
            for i, c in enumerate(x.get("checks", [])):
                if self.asked(c.get("on"), moment) and not holds(c["rule"], self.fields, attrs, self.ctx(obj)):
                    code = "form_check_warning" if c.get("warning") else "form_check_failed"
                    issues.append(issue(code, not c.get("warning"), x["id"], key_of(f), f"checks/{i}"))
        # docs/adr/0438 §1: a rule of the layer the record breaks, on its field's item.
        for r, f in rules_broken(self.rules, self.fields, None, attrs, self.ctx(obj)):
            item = None if f is None else self.item_of(key_of(f))
            issues.append(issue("form_layer_rule", not r.get("warning"), item, None if f is None else key_of(f),
                                f"rules/{r['id']}"))
        return issues

    def with_defaults(self, attrs, obj=None):
        return self.complete(attrs, obj, True)[0]

    def unique_issues(self, attrs, obj, others):
        out = []
        for (x, _), state in zip(self.nodes, self.states(attrs, obj)):
            f = self.layer(x) if x["type"] == "field" else None
            if x["type"] != "field" or not (x.get("unique") or f.get("unique")) or not state["shown"]:
                continue
            v = attrs.get(f["name"])
            if v is None or not v.strip(JS_SPACE):
                continue
            if any(o.get(f["name"]) == v for o in others):
                out.append(issue("form_unique", True, x["id"], key_of(f), "unique"))
        return out

    def locked(self, changed, after, obj=None):
        issues = []
        for (x, _), state in zip(self.nodes, self.states(after, obj)):
            if x["type"] != "field" or key_of(self.layer(x)) not in changed:
                continue
            if not state["shown"]:
                if self.clear_hidden:
                    continue
                issues.append(issue("form_hidden", True, x["id"], x["field"]))
            elif state.get("readOnly"):
                issues.append(issue("form_read_only", True, x["id"], x["field"]))
        return issues

    def item_of(self, key):
        return next(x["id"] for x, _ in self.bound() if x["field"] == key)

    def assess(self, before, values, obj=None, new=False):
        written, issues, changed, asked = dict(before), [], [], []
        on_form = [x["field"] for x, _ in self.bound()]
        for key, text in values:
            f = next((f for f in self.fields if key_of(f) == key), None)
            if f is None:
                issues.append(issue("form_field_unknown", True, None, key))
                continue
            if key not in on_form:
                issues.append(issue("form_not_on_form", True, None, key))
                continue
            x = next(x for x, _ in self.bound() if x["field"] == key)
            if "lookup" in x:
                text = "; ".join(self.code_of(x, p) for p in text.split(";")) if f.get("multiple") else self.code_of(x, text)
            v, why = check_value(f, text)
            if why:
                issues.append(issue("form_value_invalid", True, self.item_of(key), key))
                continue
            if v:
                written[f["name"]] = v
            else:
                written.pop(f["name"], None)
            if self.calculated(key):
                asked.append((key, f["name"], written.get(f["name"])))
            elif before.get(f["name"]) != written.get(f["name"]):
                changed.append(key)
        written = self.copied(None if new else before, written)
        after, completed = self.complete(written, obj, new)
        issues += self.locked(changed, after, obj)
        issues += self.retired(changed, before, after)
        for key, name, value in asked:
            if after.get(name) != value:
                issues.append(issue("form_read_only", True, self.item_of(key), key))
        issues += completed
        said = [i.get("field") for i in issues]
        issues += [i for i in self.validate(after, obj) if i.get("field") not in said]
        return after, issues

    def enforce(self, before, after, obj=None):
        issues, changed = [], []
        on_form = [x["field"] for x, _ in self.bound()]
        completed, worked = self.complete(after, obj, False)
        copies, targets = self.copied(before, after), self.copy_targets()
        for f in self.fields:
            if before.get(f["name"]) == after.get(f["name"]):
                continue
            if f["name"] in targets and copies.get(f["name"]) == after.get(f["name"]):
                continue
            if self.calculated(key_of(f)):
                if after.get(f["name"]) != completed.get(f["name"]):
                    issues.append(issue("form_read_only", True, self.item_of(key_of(f)), key_of(f)))
                continue
            if key_of(f) in on_form:
                changed.append(key_of(f))
            else:
                issues.append(issue("form_not_on_form", True, None, key_of(f)))
        return issues + self.locked(changed, completed, obj) + self.retired(changed, before, completed) + worked \
            + self.validate(completed, obj)

    def retired(self, changed, before, after):
        """A changed field given a retired value its record did not hold (docs/adr/0418)."""
        issues = []
        for key in changed:
            f = next(x for x in self.fields if key_of(x) == key)
            gone = {c["code"] for c in f.get("values", []) if withdrawn(c, self.today)}

            def parts(v):
                if v is None:
                    return []
                return [p.strip() for p in v.split(";")] if f.get("multiple") else [v]
            held = [p for p in parts(before.get(f["name"])) if p in gone]
            if any(p in gone and p not in held for p in parts(after.get(f["name"]))):
                issues.append(issue("form_value_retired", True, self.item_of(key), key))
        return issues


def withdrawn(c, today):
    """docs/adr/0418, 0438 §5: a listed value a new choice on `today` does not offer: retired, or outside its days."""
    if c.get("retired"):
        return True
    if today is None:
        return False
    return ("from" in c and today < c["from"]) or ("until" in c and today > c["until"])


def rule_breaks(src, fields, attrs, ctx):
    """docs/adr/0438 §1: a rule breaks when its answer is a false one; an empty answer holds."""
    v = value_of(src, fields, attrs, ctx)
    return not el.is_empty(v) and not el.truthy(v)


def rules_broken(rules, fields, before, after, ctx):
    """The rules a write breaks among those it asks (a new record: all; else a rule reading a changed field), each with
    the field it is shown on: its first named field, else the first field it reads."""
    out = []
    for r in rules:
        reads = fields_read(parse(r["rule"]))
        if before is not None and not any(before.get(n) != after.get(n) for n in reads):
            continue
        if not rule_breaks(r["rule"], fields, after, ctx):
            continue
        shown = next((f for k in r.get("fields", [])[:1] for f in fields if key_of(f) == k), None)
        if shown is None:
            shown = next((f for n in reads for f in fields if f["name"] == n), None)
        out.append((r, shown))
    return out


def text_or_none(v):
    return None if el.is_empty(v) else el.text(v)


def item_id_of(name, taken):
    stem = ""
    for c in fold(name):
        if c.isalnum():
            stem += c
        elif not stem.endswith("-"):
            stem += "-"
    stem = stem.strip("-")[: MOST_NAME - 4] or "alan"
    if stem not in taken:
        return stem
    n = 2
    while f"{stem}-{n}" in taken:
        n += 1
    return f"{stem}-{n}"


def generate(fields):
    items, taken = [], []
    for f in fields:
        i = item_id_of(f["name"], taken)
        taken.append(i)
        item = {"type": "field", "id": i, "field": key_of(f)}
        if f.get("required"):
            item["required"] = True
        items.append(item)
    return {"revision": 1, "items": items}


def canonical_json(v):
    return kc.canonical_json(v)


def digest(body):
    return hashlib.sha256(canonical_json(body).encode("utf-8")).hexdigest()[:32]


def snapshot(attrs, skip=(), attachments=(), children=()):
    """A record's snapshot (docs/adr/0440 §4): 16 hex digits of the SHA-256 of the canonical JSON of its attributes
    (`skip`'s left out: a workflow's state and log), its attachments and its children ([relation, uid, attrs] in the
    relations' order, each relation's sorted by uid)."""
    value = {"attrs": {k: v for k, v in attrs.items() if k not in skip}, "attachments": list(attachments),
             "children": [{"relation": r, "uid": u, "attrs": a} for r, u, a in children]}
    return hashlib.sha256(canonical_json(value).encode("utf-8")).hexdigest()[:16]


def snapshot_skip(body, fields):
    """The names a form's workflow keeps out of the snapshot: its state's and its log's."""
    w = body.get("workflow")
    if not w:
        return ()
    keys = [w["field"]] + ([w["log"]] if "log" in w else [])
    return tuple(f["name"] for f in fields if key_of(f) in keys)


def changed_names(a, b):
    return sorted({k for k in list(a) + list(b) if a.get(k) != b.get(k)}, key=lambda s: s.encode("utf-8"))


def submitted(out, slot_uid, before, after, layer, label, snap):
    """The submission's answer's new parts (docs/adr/0440 §4, §5): the records written, the snapshot, the undo step."""
    names = changed_names(before, after)
    if names:
        out["changes"] = [{"uid": slot_uid, "layer": layer, "fields": names}]
        out["undo"] = label
    out["snapshot"] = snap
    return out


# ── The shared layer and form ────────────────────────────────────────────

FIELDS = [
    {"name": "Durum", "kind": "text", "values": [{"code": "iyi", "label": "İyi"}, {"code": "inceleme", "label": "Ek inceleme gerekli"}], "required": True},
    {"id": "kat-sayisi", "name": "Kat", "kind": "integer", "min": "1", "max": "40"},
    {"name": "Açıklama", "kind": "text", "length": 500},
    {"name": "Tarih", "kind": "date"},
    {"name": "Onaylı", "kind": "boolean"},
]

NOT_ITEM = {
    "type": "field", "id": "aciklama", "field": "Açıklama", "widget": "textArea",
    "visibleWhen": "Durum = 'inceleme'", "requiredWhen": "Durum = 'inceleme'",
    "checks": [{"rule": "uzunluk(Açıklama) >= 10", "message": "Açıklama en az 10 karakter olmalı."}],
}
BODY = {"revision": 1, "items": [
    {"type": "tab", "id": "genel", "label": "Genel", "items": [
        {"type": "field", "id": "durum", "field": "Durum", "required": True},
        {"type": "field", "id": "kat", "field": "kat-sayisi", "label": "Kat sayısı", "help": "Zemin kat dahil."},
        NOT_ITEM,
    ]},
    {"type": "tab", "id": "ayrinti", "label": "Ayrıntı", "items": [
        {"type": "group", "id": "denetim", "label": "Denetim", "columns": 2, "visibleWhen": "Kat >= 1", "items": [
            {"type": "field", "id": "tarih", "field": "Tarih", "default": "2026-10-16"},
            {"type": "field", "id": "onay", "field": "Onaylı", "widget": "switch", "readOnlyWhen": "Durum = 'iyi'"},
        ]},
        {"type": "note", "id": "bilgi", "text": "Denetimi yapan kişi kaydeder."},
    ]},
]}
assert kc.body_problem(BODY) is None


def body_with(change):
    b = copy.deepcopy(BODY)
    change(b)
    return b


def fields_with(change):
    f = copy.deepcopy(FIELDS)
    change(f)
    return f


def flat(body):
    return [{"type": "field", "id": f"a{i}", "field": key_of(f)} for i, f in enumerate(body)]


def item(b, iid):
    found = []

    def walk(xs):
        for x in xs:
            if x["id"] == iid:
                found.append(x)
            walk(x.get("items", []))

    walk(b["items"])
    return found[0]


# ── Core cases ───────────────────────────────────────────────────────────


def core():
    pre = []

    def case(name, body=BODY, fields=FIELDS):
        pre.append({"name": name, "fields": fields, "body": body, "issues": preflight(body, fields)})

    case("örnek form: hata yok; Açıklama'nın görünürlüğü Durum'u okur, kendisini değil")
    case("olmayan alan: anahtar katmanda yok; kuralları sorulmaz",
         body_with(lambda b: item(b, "kat").update(field="Kat sayısı", visibleWhen="bilinmez(")))
    case("alan anahtarla bağlanır: kimliği olan alan adıyla bağlanamaz",
         body_with(lambda b: item(b, "kat").update(field="Kat")))
    case("aynı alan iki kez", body_with(lambda b: b["items"][0]["items"].append({"type": "field", "id": "durum-2", "field": "Durum"})))
    case("uymayan öğe: tarih alanına onay kutusu, sayıya tek satır, evet/hayıra sayı",
         body_with(lambda b: (item(b, "tarih").update(widget="check"), item(b, "kat").update(widget="text"), item(b, "onay").update(widget="number"))))
    case("liste öğesi değer listesi ister; evet/hayırın iki değeri liste sayılır",
         body_with(lambda b: (item(b, "aciklama").update(widget="list"), item(b, "onay").update(widget="segmented"), item(b, "kat").update(widget="radio"))))
    case("kaydırıcı: aralığı olan sayı alanına uyar, metne uymaz",
         body_with(lambda b: (item(b, "kat").update(widget="slider"), item(b, "aciklama").update(widget="slider"))))
    case("kaydırıcı en az ve en çok değer ister",
         body_with(lambda b: item(b, "kat").update(widget="slider")),
         [{k: v for k, v in f.items() if k != "min"} if f.get("id") == "kat-sayisi" else f for f in FIELDS])
    case("varsayılan alanın kuralına uymalı: aralık dışı kat, takvimde olmayan gün",
         body_with(lambda b: (item(b, "kat").update(default="41"), item(b, "tarih").update(default="2026-02-30"))))
    case("varsayılan listenin etiketiyle de yazılır; tek biçime çevrilir",
         body_with(lambda b: item(b, "durum").update(default="Ek inceleme gerekli")))
    case("zorunlu ve salt okunur alan varsayılansız doldurulamaz",
         body_with(lambda b: item(b, "kat").update(readOnly=True, required=True)))
    case("okunamayan kural ve denetim; yerleri adıyla",
         body_with(lambda b: (item(b, "aciklama").update(visibleWhen="Durum = ", checks=[{"rule": "uzunluk(Açıklama) >= 10", "message": "x"}, {"rule": "Açıklama gibi", "message": "y"}]))))
    case("bilinmeyen işlev derlenmez", body_with(lambda b: item(b, "denetim").update(visibleWhen="bilinmez(Kat)")))
    case("katmanda olmayan ad uyarıdır; alanın kendini okuması uyarıdır, zorunluluğun okuması değil",
         body_with(lambda b: (item(b, "kat").update(visibleWhen="Kat > 0 ve Kod = 'A'", requiredWhen="Kat > 3"), item(b, "onay").update(readOnlyWhen="Onaylı"))))
    case("katmanın zorunlu alanı formda yok", body_with(lambda b: b["items"][0]["items"].pop(0)))
    case("katmanın zorunlu alanı bir kuralla gizlenebilir ya da kilitlenebilir: grubun kuralı da sayılır",
         body_with(lambda b: (b["items"][0]["items"].pop(0), item(b, "denetim")["items"].append({"type": "field", "id": "durum", "field": "Durum"}))))
    case("katmanın zorunlu alanı salt okunur kuralla", body_with(lambda b: item(b, "durum").update(readOnlyWhen="Kat > 3")))
    case("alansız form uyarıdır; zorunlu alan da eksiktir", {"revision": 1, "items": [{"type": "note", "id": "n", "text": "Boş"}]})
    case("yapısı bozuk form tek sorundur", {"revision": 1, "items": [{"type": "note", "id": "a b", "text": "x"}]})
    case("katman alanları değişince: kimlik ve ad, yeni zorunlu alan",
         fields=fields_with(lambda f: (f[2].update(id="aciklama"), f[3].update(required=True))))

    records = []

    def record(name, attrs, body=BODY, fields=FIELDS):
        form = Form(body, fields)
        records.append({"name": name, "fields": fields, "body": body, "attrs": attrs,
                        "items": form.described(attrs), "issues": form.validate(attrs)})

    record("eksiksiz kayıt: inceleme, notuyla", {"Durum": "inceleme", "Kat": "3", "Açıklama": "Çatı arası ayrıca bakılmalı.", "Tarih": "2026-10-01", "Onaylı": "true"})
    record("boş kayıt: Durum zorunlu; grup Kat'ı okur, gizli", {})
    record("Durum iyi: Açıklama gizli ve sorulmaz, değeri kalır; onay salt okunur", {"Durum": "iyi", "Kat": "2", "Açıklama": "kısa"})
    record("Durum inceleme ve Açıklama boş: koşullu zorunlu", {"Durum": "inceleme", "Kat": "1"})
    record("denetim tutmuyor: Açıklama kısa", {"Durum": "inceleme", "Kat": "1", "Açıklama": "kısa"})
    record("kurala uymayan değerler: aralık dışı kat, listede olmayan durum", {"Durum": "yıkık", "Kat": "50"})
    record("eski değer tek biçimde değil ama alanın değeri: geçerli", {"Durum": "iyi", "Kat": "007", "Onaylı": "evet"})
    record("evet/hayır kurala tipiyle girer: Onaylı yanlışsa grup kuralı gizler",
           {"Durum": "iyi", "Kat": "2", "Onaylı": "false"},
           body=body_with(lambda b: item(b, "denetim").update(visibleWhen="Onaylı = doğru")))
    record("sayı kurala sayı olarak girer: Kat + 1 toplanır",
           {"Durum": "inceleme", "Kat": "9", "Açıklama": "On karakter."},
           body=body_with(lambda b: item(b, "aciklama").update(visibleWhen="Kat + 1 = 10")))

    assess = []

    def write(name, before, values, body=BODY, fields=FIELDS):
        after, issues = Form(body, fields).assess(before, values)
        assess.append({"name": name, "fields": fields, "body": body, "before": before,
                       "values": values, "after": after, "issues": issues})

    base = {"Durum": "inceleme", "Kat": "3", "Açıklama": "Çatı arası ayrıca bakılmalı.", "Kod": "B-12"}
    write("değerler tek biçimle yazılır: listenin etiketi kodu, tarih GG.AA.YYYY", base,
          [["Durum", "İyi"], ["Tarih", "1.10.2026"], ["kat-sayisi", " 04 "]])
    write("boş değer özniteliği kaldırır; alan dışı öznitelik kalır", {**base, "Tarih": "2026-01-01"}, [["Tarih", "  "]])
    write("Durum iyi yapılınca gizlenen açıklamayı silmek de gizli alana yazmaktır", base, [["Açıklama", "  "], ["Durum", "iyi"]])
    write("gizli alana yazmak reddedilir: Durum iyi olunca Açıklama gizlenir", base, [["Durum", "iyi"], ["Açıklama", "Yeni not, uzun."]])
    write("salt okunur alana yazmak reddedilir; aynı değeri yazmak değiştirmez", {**base, "Durum": "iyi", "Onaylı": "true"},
          [["Onaylı", "false"]])
    write("salt okunur alana aynı değer", {**base, "Durum": "iyi", "Onaylı": "true"}, [["Onaylı", "evet"]])
    write("olmayan anahtar ve formda olmayan alan", base, [["Yok", "1"]], body=body_with(lambda b: b["items"][1]["items"].pop(0)))
    write("formda olmayan alan", base, [["Onaylı", "true"]], body=body_with(lambda b: b["items"][1]["items"].pop(0)))
    write("kurala uymayan değer kayda girmez; aynı alan yeniden sorulmaz", base, [["kat-sayisi", "üç"], ["Açıklama", "x" * 501]])
    write("koşullu zorunlu: Durum inceleme yapılınca Açıklama ister", {"Durum": "iyi", "Kat": "2"}, [["Durum", "inceleme"]])

    enforce = []

    def guard(name, before, after, body=BODY, fields=FIELDS):
        enforce.append({"name": name, "fields": fields, "body": body, "before": before, "after": after,
                        "issues": Form(body, fields).enforce(before, after)})

    guard("geçerli yazma", base, {**base, "Açıklama": "Çatı arası ve merdiven."})
    guard("açıklamayı silmek: koşullu zorunlu", base, {k: v for k, v in base.items() if k != "Açıklama"})
    guard("alan dışı öznitelik serbest", base, {**base, "Kod": "B-13"})
    guard("formda olmayan alan yazılamaz", base, {**base, "Onaylı": "true"}, body=body_with(lambda b: b["items"][1]["items"].pop(0)))
    guard("gizli alanı değiştirmek", {**base, "Durum": "iyi"}, {**base, "Durum": "iyi", "Açıklama": "Başka bir not."})
    guard("eski kayıt kurala uymuyorsa her yazma onu da söyler", {"Durum": "inceleme", "Kat": "3"}, {"Durum": "inceleme", "Kat": "4"})

    defaults = []
    for name, attrs in (("boş kayda varsayılan", {}), ("dolu alana varsayılan yazılmaz", {"Tarih": "2025-01-01"}),
                        ("boşluk boş sayılır", {"Tarih": "  "})):
        defaults.append({"name": name, "fields": FIELDS, "body": BODY, "attrs": attrs, "out": Form(BODY, FIELDS).with_defaults(attrs)})

    calc_pre, complete, calc_records, calc_assess, calc_enforce, unique = calc_cases()
    pre.extend(calc_pre)
    records.extend(calc_records)
    assess.extend(calc_assess)
    enforce.extend(calc_enforce)
    look_pre, look_records, look_assess = lookup_cases()
    extra_pre, extra_records = extras_core()
    pre.extend(extra_pre)
    records.extend(extra_records)
    multi_pre, multi_records, multi_assess = multi_core()
    pre.extend(multi_pre)
    records.extend(multi_records)
    assess.extend(multi_assess)
    pre.extend(look_pre)
    records.extend(look_records)
    assess.extend(look_assess)
    src_pre, src_records, src_assess, src_enforce = sources_core()
    pre.extend(src_pre)
    records.extend(src_records)
    assess.extend(src_assess)
    enforce.extend(src_enforce)

    gen = []
    for name, fields in (
        ("katmanın alanlarından: sırası, anahtarı, zorunluluğu", FIELDS),
        ("aynı köke inen adlar numaralanır, Türkçe harfler kalır", [
            {"name": "İl adı", "kind": "text"}, {"name": "il adı ", "kind": "text"}, {"name": "--", "kind": "text"},
            {"name": "Şehir/İlçe", "kind": "text"}, {"name": "Ölçü", "kind": "decimal", "scale": 2}]),
    ):
        gen.append({"name": name, "fields": fields, "body": generate(fields)})

    return {
        "format": "kentos.forms-cases",
        "version": 1,
        "note": "ADR 0401 ve 0402'nin form çekirdeği, KentOS kodu olmadan yazılmış başvurudan "
                "(scripts/fixtures/forms_cases.py): ön denetim, kaydın öğeleri (süzülen seçenekler, notların metni) ve "
                "sorunları, kaydın tamamlanması (varsayılanlar, hesaplar, gizlenenin silinmesi), formla yazma, yazma "
                "profili, tekil alan, varsayılanlar ve katmandan form. Durumun `variables`'ı verilen `@` değerleri, "
                "`object`'i nesnenin ölçüleri (metrede). Sorunlar kodu, ağırlığı, öğesi, alanı ve kuralıyla "
                "karşılaştırılır; cümleleri çekirdeğindir.",
        "preflight": pre,
        "records": records,
        "complete": complete,
        "assess": assess,
        "enforce": enforce,
        "unique": unique,
        "defaults": defaults,
        "generate": gen,
    }


# ── Calculations, choices, soft checks (docs/adr/0402) ─────────────────

CALC_FIELDS = [
    {"name": "Durum", "kind": "text", "required": True,
     "values": [{"code": "iyi", "label": "İyi"}, {"code": "inceleme", "label": "Ek inceleme gerekli"}]},
    {"id": "kat-sayisi", "name": "Kat", "kind": "integer", "min": "1", "max": "40"},
    {"name": "Yükseklik", "kind": "decimal", "scale": 2},
    {"name": "Taban", "kind": "decimal", "scale": 2},
    {"name": "Riskli", "kind": "boolean"},
    {"name": "Kod", "kind": "text", "length": 20},
    {"name": "Tür", "kind": "text", "values": [{"code": "konut", "label": "Konut"}, {"code": "sanayi", "label": "Sanayi"}]},
    {"name": "Malzeme", "kind": "text", "values": [
        {"code": "ahsap", "label": "Ahşap"}, {"code": "beton", "label": "Betonarme"},
        {"code": "celik", "label": "Çelik"}, {"code": "tas", "label": "Taş"}]},
    {"name": "Açıklama", "kind": "text", "length": 500},
    {"name": "Tarih", "kind": "date"},
    {"name": "Kayıt", "kind": "text", "length": 64},
    {"name": "Puan", "kind": "integer", "min": "0", "max": "10"},
]

CALC_BODY = {"revision": 1, "items": [
    {"type": "group", "id": "genel", "label": "Genel", "columns": 2, "items": [
        {"type": "field", "id": "durum", "field": "Durum", "required": True},
        {"type": "field", "id": "kat", "field": "kat-sayisi"},
        {"type": "field", "id": "kod", "field": "Kod", "unique": True, "defaultExpr": "'B-' || Kat"},
        {"type": "field", "id": "tur", "field": "Tür"},
        {"type": "field", "id": "malzeme", "field": "Malzeme",
         "choiceFilter": "Tür <> 'sanayi' veya @secenek = 'beton' veya @secenek = 'celik'"},
    ]},
    {"type": "group", "id": "hesap", "label": "Hesaplanan", "collapsed": True, "items": [
        {"type": "field", "id": "yukseklik", "field": "Yükseklik", "calculate": "Kat * 3"},
        {"type": "field", "id": "taban", "field": "Taban", "calculate": "$alan"},
        {"type": "field", "id": "riskli", "field": "Riskli", "calculate": "Kat > 5"},
        {"type": "field", "id": "puan", "field": "Puan", "calculate": "Kat - 1"},
        {"type": "note", "id": "toplam", "text": "Toplam alan {Kat * Taban} m², {$köşe} köşe."},
    ]},
    {"type": "field", "id": "aciklama", "field": "Açıklama", "visibleWhen": "Durum = 'inceleme'",
     "checks": [{"rule": "uzunluk(Açıklama) >= 5", "message": "Açıklama çok kısa."},
                {"rule": "uzunluk(Açıklama) >= 20", "message": "Açıklama kısa görünüyor.", "warning": True}]},
    {"type": "field", "id": "tarih", "field": "Tarih", "defaultExpr": "@tarih"},
    {"type": "field", "id": "kayit", "field": "Kayıt", "defaultExpr": "@kullanici"},
]}
assert kc.body_problem(CALC_BODY) is None

GIVEN = [{"name": "tarih", "value": "2026-10-16"}, {"name": "simdi", "value": "2026-10-16T14:30:00"},
         {"name": "kullanici", "value": "Ayşe Yılmaz"}]
SQUARE = {"length": 40, "area": 100, "vertices": 4}


def calc_with(change):
    b = copy.deepcopy(CALC_BODY)
    change(b)
    return b


# ── Lists from other layers and related records (docs/adr/0404) ─────────

def rows(*items):
    return [[f"0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d{n:04x}", a] for n, a in items]


LOOK_WORLD = {
    "il": {"name": "İller", "fields": [{"name": "kod", "kind": "text"}, {"name": "ad", "kind": "text"}],
           "rows": rows((1, {"kod": "06", "ad": "Ankara"}), (2, {"kod": "34", "ad": "İstanbul"}),
                        (3, {"kod": "35", "ad": "İzmir"}), (4, {"ad": "Kodsuz"}))},
    "ilce": {"name": "İlçeler",
             "fields": [{"name": "kod", "kind": "text"}, {"name": "ad", "kind": "text"}, {"name": "il", "kind": "text"}],
             "rows": rows((11, {"kod": "0601", "ad": "Çankaya", "il": "06"}), (12, {"kod": "0602", "ad": "Altındağ", "il": "06"}),
                          (13, {"kod": "3401", "ad": "Kadıköy", "il": "34"}), (14, {"kod": "0601", "ad": "Yineleme", "il": "06"}))},
    "mahalle": {"name": "Mahalleler",
                "fields": [{"name": "kod", "kind": "text"}, {"name": "ad", "kind": "text"}, {"name": "ilce", "kind": "text"},
                           {"name": "durum", "kind": "text"}],
                "rows": rows((21, {"kod": "M10", "ad": "Kızılay", "ilce": "0601", "durum": "etkin"}),
                             (22, {"kod": "M2", "ad": "Bahçelievler", "ilce": "0601", "durum": "etkin"}),
                             (23, {"kod": "M3", "ad": "Eski mahalle", "ilce": "0601", "durum": "kapalı"}),
                             (24, {"kod": "M4", "ad": "Ulus", "ilce": "0602", "durum": "etkin"}))},
    "parsel": {"name": "Parseller", "fields": [{"name": "ada", "kind": "text"}],
               "rows": rows((31, {"ada": "101/5"}), (32, {"ada": "101/12"}), (33, {}))},
    "sayac": {"name": "Sayaçlar", "fields": [{"name": "no", "kind": "text"}, {"name": "bina", "kind": "text"}],
              "rows": rows((41, {"no": "S-1", "bina": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0063"}),
                           (42, {"no": "S-2", "bina": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0063"}),
                           (43, {"no": "S-3", "bina": "başka"}),
                           # docs/adr/0409 §1: a meter shared by two buildings.
                           (44, {"no": "S-4", "bina": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0050; 0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0063"}))},
}
LOOK_FIELDS = [{"name": "İl", "kind": "text"}, {"name": "İlçe", "kind": "text"}, {"name": "Mahalle", "kind": "text"},
               {"name": "Parsel", "kind": "text"}, {"name": "Tür", "kind": "text",
                                                    "values": [{"code": "k", "label": "Konut"}]}]
LOOK_BODY = {"revision": 1, "items": [
    {"type": "field", "id": "il", "field": "İl", "lookup": {"layer": "il", "key": "kod", "label": "ad"}},
    {"type": "field", "id": "ilce", "field": "İlçe",
     "lookup": {"layer": "ilce", "key": "kod", "label": "ad", "matches": [{"field": "il", "record": "İl"}]}},
    {"type": "field", "id": "mahalle", "field": "Mahalle", "widget": "radio",
     "lookup": {"layer": "mahalle", "key": "kod", "label": "ad", "matches": [{"field": "ilce", "record": "İlçe"}],
                "filter": "durum = 'etkin'"}},
    {"type": "field", "id": "parsel", "field": "Parsel", "lookup": {"layer": "parsel", "key": "$uid", "label": "ada"}},
    {"type": "related", "id": "sayaclar", "label": "Sayaçlar", "layer": "sayac", "field": "bina", "visibleWhen": "İl = '06'"},
]}
assert kc.body_problem(LOOK_BODY) is None
# The building whose meters refer to it.
LOOK_OBJECT = {"uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0063"}


def look_with(change):
    b = copy.deepcopy(LOOK_BODY)
    change(b)
    return b


def lookup_cases():
    pre = []

    def case(name, body=LOOK_BODY, fields=LOOK_FIELDS, world=LOOK_WORLD):
        pre.append({"name": name, "fields": fields, "body": body, "variables": [], "world": world,
                    "issues": preflight(body, fields, (), world)})

    case("başka katmandan liste, zincir, süzgeç, nesneye bağ ve ilişkili kayıtlar: sorun yok")
    case("katmanlar bilinmezse listeler ve ilişkili öğe hatadır", world={})
    case("listenin olmayan alanları; kaydın olmayan alanı",
         look_with(lambda b: (item(b, "il")["lookup"].update(label="isim"),
                              item(b, "ilce")["lookup"]["matches"].append({"field": "yok", "record": "Semt"}))))
    case("kendi değer listesi olan alana başka katmandan liste olmaz",
         look_with(lambda b: b["items"].append({"type": "field", "id": "tur", "field": "Tür",
                                                "lookup": {"layer": "il", "key": "kod"}})))
    case("okunamayan süzgeç", look_with(lambda b: item(b, "mahalle")["lookup"].update(filter="durum =")))
    case("ilişkili kaydın alanı yok", look_with(lambda b: item(b, "sayaclar").update(field="yapi")))

    records = []

    def record(name, attrs, obj=LOOK_OBJECT):
        form = Form(LOOK_BODY, LOOK_FIELDS, (), LOOK_WORLD)
        records.append({"name": name, "fields": LOOK_FIELDS, "body": LOOK_BODY, "variables": [], "world": LOOK_WORLD,
                        "object": obj, "attrs": attrs, "items": form.described(attrs, obj),
                        "issues": form.validate(attrs, obj)})

    record("boş kayıt: il listesi doğal sırada, zincir boş; ilişkili sayaçlar gizli", {})
    record("Ankara: ilçeler, yinelenen kod bir kez; Çankaya'nın etkin mahalleleri doğal sırada; sayaçlar",
           {"İl": "06", "İlçe": "0601", "Mahalle": "M2"})
    record("zincirin dışında kalan ilçe ve kapalı mahalle hatadır",
           {"İl": "34", "İlçe": "0601", "Mahalle": "M3"})
    record("parsele nesnenin kimliğiyle bağ; olmayan kimlik hatadır",
           {"İl": "06", "Parsel": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d00ff"})
    record("bağsız nesnenin ilişkili kaydı yok", {"İl": "06"}, obj={})

    assess = []

    def write(name, before, values):
        after, issues = Form(LOOK_BODY, LOOK_FIELDS, (), LOOK_WORLD).assess(before, values, LOOK_OBJECT, False)
        assess.append({"name": name, "fields": LOOK_FIELDS, "body": LOOK_BODY, "variables": [], "world": LOOK_WORLD,
                       "object": LOOK_OBJECT, "before": before, "values": values, "new": False, "after": after,
                       "issues": issues})

    write("etiketler kodlarıyla yazılır", {}, [["İl", "Ankara"], ["İlçe", "Çankaya"], ["Mahalle", "Kızılay"]])
    write("parsel adasıyla seçilir, kimliği yazılır", {"İl": "06"}, [["Parsel", "101/12"]])
    write("listede olmayan değer yazılmaz", {"İl": "06"}, [["İlçe", "Kadıköy"]])
    write("üst alan değişince alt alan zincirin dışında kalır", {"İl": "06", "İlçe": "0601"}, [["İl", "34"]])

    # docs/adr/0409 §1: a many-to-many reference: several parcels by their ids.
    nm_fields = LOOK_FIELDS + [{"name": "Parseller", "kind": "text", "multiple": True}]
    nm_body = look_with(lambda b: b["items"].insert(4, {"type": "field", "id": "parseller", "field": "Parseller",
                                                        "lookup": {"layer": "parsel", "key": "$uid", "label": "ada"}}))
    case("çoktan çoğa: çok değerli alana başka katmandan çoklu seçim", nm_body, nm_fields)
    case("çok değerli başvuruyu tek seçimli liste düzenleyemez",
         look_with(lambda b: b["items"].insert(4, {"type": "field", "id": "parseller", "field": "Parseller", "widget": "list",
                                                   "lookup": {"layer": "parsel", "key": "$uid", "label": "ada"}})), nm_fields)
    p31, p32 = rows((31, {}), (32, {}))[0][0], rows((32, {}),)[0][0]
    for name, attrs in (("iki parselin kimliği geçerli", {"İl": "06", "Parseller": f"{p31}; {p32}"}),
                        ("olmayan parsel hatadır", {"İl": "06", "Parseller": f"{p31}; 0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d00ff"})):
        form = Form(nm_body, nm_fields, (), LOOK_WORLD)
        records.append({"name": f"çoktan çoğa: {name}", "fields": nm_fields, "body": nm_body, "variables": [], "world": LOOK_WORLD,
                        "object": LOOK_OBJECT, "attrs": attrs, "items": form.described(attrs, LOOK_OBJECT),
                        "issues": form.validate(attrs, LOOK_OBJECT)})
    after, issues = Form(nm_body, nm_fields, (), LOOK_WORLD).assess({"İl": "06"}, [["Parseller", "101/12; 101/5; 101/12"]],
                                                                    LOOK_OBJECT, False)
    assess.append({"name": "çoktan çoğa: adalarıyla seçilir, kimlikleri yazılan sırayla bir kez", "fields": nm_fields,
                   "body": nm_body, "variables": [], "world": LOOK_WORLD, "object": LOOK_OBJECT, "before": {"İl": "06"},
                   "values": [["Parseller", "101/12; 101/5; 101/12"]], "new": False, "after": after, "issues": issues})
    return pre, records, assess


# ── Lists from other layers: order, columns, page, chain, duplicates, copies, geometry (docs/adr/0439) ──

def _sq(name, x0, y0, x1, y1):
    return sq.square(name, x0, y0, x1, y1)[1]


SRC_PARCELS = [
    # uid, attrs, shape
    ("p-a", {"ada": "101", "parsel": "5", "mahalle": "İNÖNÜ", "mkod": "06", "alan": "450.50", "malik": "Ayşe Kaya",
             "tarih": "2024-03-01"}, _sq("A", 0, 0, 100, 100)),
    ("p-b", {"ada": "101", "parsel": "12", "mahalle": "Kızılay", "mkod": "7", "alan": "1200", "malik": "Ali Can",
             "tarih": "2023-11-20"}, _sq("B", 100, 0, 200, 100)),
    ("p-c", {"ada": "102", "parsel": "1", "mahalle": "kızılay", "mkod": "7", "alan": "80.25"}, _sq("C", 0, 100, 100, 200)),
    # Ada 101 parsel 12 again (a duplicate code for the ada-parsel key).
    ("p-d", {"ada": "101", "parsel": "12", "mahalle": "Kızılay", "mkod": "7", "alan": "1201", "malik": "Veli Can"},
     _sq("D", 300, 300, 310, 310)),
]
SRC_FIELDS = [{"name": "ada", "kind": "integer"}, {"name": "parsel", "kind": "integer"},
              {"name": "mahalle", "kind": "text"}, {"name": "mkod", "kind": "integer"},
              {"name": "alan", "alias": "Yüzölçümü", "kind": "decimal", "scale": 2}, {"name": "malik", "kind": "text"},
              {"name": "tarih", "kind": "date"}, {"name": "no", "kind": "text"}]
for _uid, _a, _ in SRC_PARCELS:
    _a["no"] = f"{_a['ada']}/{_a['parsel']}"


def src_world(shapes=True):
    """The world as a case writes it (rows with their geometry) and as the twin reads it."""
    json_rows = [[u, a, s["json"]] if shapes else [u, a] for u, a, s in SRC_PARCELS]
    py = {"parsel": {"name": "Parsel", "fields": SRC_FIELDS, "rows": [[u, a] for u, a, _ in SRC_PARCELS]}}
    if shapes:
        py["parsel"]["shapes"] = [s for _, _, s in SRC_PARCELS]
    return {"parsel": {"name": "Parsel", "fields": SRC_FIELDS, "rows": json_rows}}, py


SRC_OWN = [{"name": "Parsel", "kind": "text"}, {"name": "Ada", "kind": "integer"}, {"name": "Alan", "kind": "decimal"},
           {"name": "Malik", "kind": "text"}, {"name": "Kaynak", "kind": "text"}, {"name": "Mahalle", "kind": "text"},
           {"name": "MahKod", "kind": "integer"}, {"name": "Kat", "kind": "integer"}]
SRC_LOOKUP = {"layer": "parsel", "key": "$uid", "label": "no", "columns": ["alan", "malik", "tarih"],
              "sort": {"field": "alan", "descending": True}, "limit": 2,
              "copy": [{"from": "ada", "to": "Ada"}, {"from": "alan", "to": "Alan"}, {"from": "malik", "to": "Malik"}],
              "stamp": "Kaynak"}
SRC_BODY = {"revision": 1, "items": [{"type": "field", "id": "parsel", "field": "Parsel", "lookup": SRC_LOOKUP},
                                     {"type": "field", "id": "kat", "field": "Kat"}]}
assert kc.body_problem(SRC_BODY) is None
SRC_TODAY = [{"name": "tarih", "value": "2026-10-17"}]


def src_body(**lookup):
    b = copy.deepcopy(SRC_BODY)
    b["items"][0]["lookup"].update(lookup)
    for k in [k for k, v in lookup.items() if v is None]:
        del b["items"][0]["lookup"][k]
    return b


def src_object(shape=None):
    """The object as a case writes it and as the twin reads it."""
    j = {"uid": "bina-1"}
    if shape is not None:
        j["entity"] = shape["json"]
    return j, ({**j, "shape": shape} if shape is not None else dict(j))


def sources_core():
    pre, records, assess, enforce = [], [], [], []
    wj, wp = src_world()

    def case(name, body, fields=SRC_OWN):
        assert kc.body_problem(body) is None, name
        pre.append({"name": f"kaynak: {name}", "fields": fields, "body": body, "variables": [], "world": wj,
                    "issues": preflight(body, fields, (), wp)})

    case("sıralama, sütunlar, sayfa, kopya, damga, konum ve yinelenen kuralı: sorun yok",
         src_body(spatial={"relation": "contains", "tolerance": 0.5}, duplicates="refuse"))
    case("listenin ve kaydın olmayan alanları",
         src_body(columns=["alan", "yok"], sort={"field": "eksik"}, copy=[{"from": "hiç", "to": "Ada"},
                                                                         {"from": "alan", "to": "Yok"}], stamp="Damga"))
    case("türü uymayan kopya, damga ve hedef",
         src_body(copy=[{"from": "alan", "to": "Ada"}, {"from": "malik", "to": "Parsel"}], stamp="Kat"))
    case("sayıya metin anahtar yazılamaz",
         {"revision": 1, "items": [{"type": "field", "id": "kat", "field": "Kat",
                                    "lookup": {"layer": "parsel", "key": "mahalle"}}]})
    case("zincir türü uymayan alanları karşılaştıramaz",
         {"revision": 1, "items": [{"type": "field", "id": "parsel", "field": "Parsel",
                                    "lookup": {"layer": "parsel", "key": "$uid",
                                               "matches": [{"field": "mkod", "record": "Mahalle"}]}}]})
    case("tam sayı alanına tam sayı anahtar, metin alanına sayı anahtar yazılır",
         {"revision": 1, "items": [{"type": "field", "id": "kat", "field": "Kat",
                                    "lookup": {"layer": "parsel", "key": "ada"}},
                                   {"type": "field", "id": "mahalle", "field": "Mahalle",
                                    "lookup": {"layer": "parsel", "key": "mkod"}}]})

    def record(name, body, attrs, obj=None, world=(wj, wp), variables=()):
        oj, op = obj if obj is not None else src_object()
        form = Form(body, SRC_OWN, variables, world[1])
        records.append({"name": f"kaynak: {name}", "fields": SRC_OWN, "body": copy.deepcopy(body), "variables": list(variables),
                        "world": world[0], "object": oj, "attrs": attrs, "items": form.described(attrs, op),
                        "issues": form.validate(attrs, op)})

    record("yüzölçümüne göre azalan, sütunlarıyla, ilk iki değer ve kalanın sayısı", SRC_BODY, {})
    record("seçilen parselin sütunları canlı gösterilir", SRC_BODY, {"Parsel": "p-c"})
    record("doğal sıra, sayfasız", src_body(sort=None, limit=None, columns=None), {})
    record("tarihe göre artan: boş tarihler sonda", src_body(sort={"field": "tarih"}, limit=None), {})
    chain = {"revision": 1, "items": [
        {"type": "field", "id": "parsel", "field": "Parsel",
         "lookup": {"layer": "parsel", "key": "$uid", "label": "no", "matches": [{"field": "mkod", "record": "MahKod"}]}},
        {"type": "field", "id": "mah", "field": "Mahalle"}, {"type": "field", "id": "mk", "field": "MahKod"}]}
    record("sayılar değeriyle eşlenir: 6, 06'yı bulur", chain, {"MahKod": "6"})
    folded = copy.deepcopy(chain)
    folded["items"][0]["lookup"]["matches"] = [{"field": "mahalle", "record": "Mahalle"}]
    record("katlanmadan: küçük harfle yazılan mahalle bulunmaz", folded, {"Mahalle": "inönü"})
    folded["items"][0]["lookup"]["matches"][0]["fold"] = True
    record("katlanarak: İNÖNÜ ve inönü, Kızılay ve kızılay aynıdır", folded, {"Mahalle": "KIZILAY"})
    by_no = {"revision": 1, "items": [{"type": "field", "id": "parsel", "field": "Parsel",
                                       "lookup": {"layer": "parsel", "key": "no", "label": "malik"}}]}
    record("yinelenen kod: ilkininki alınır ve uyarılır", by_no, {"Parsel": "101/12"})
    first = copy.deepcopy(by_no)
    first["items"][0]["lookup"]["duplicates"] = "first"
    record("yinelenen kod: ilkini al seçildiyse sessiz", first, {"Parsel": "101/12"})
    refuse = copy.deepcopy(by_no)
    refuse["items"][0]["lookup"]["duplicates"] = "refuse"
    record("yinelenen kod reddedilir; tekil kod geçer", refuse, {"Parsel": "101/12"})
    record("tekil kod reddedilmez", refuse, {"Parsel": "102/1"})
    stale = {"Parsel": "p-b", "Ada": "101", "Alan": "1100", "Malik": "Ali Can",
             "Kaynak": "Parsel · p-b · #00000000 · 2025-01-01"}
    record("kopyanın kaynağı değiştiyse uyarılır", SRC_BODY, stale)
    building = src_object(_sq("bina", 10, 10, 30, 30))
    record("kaydı içeren parsel: yalnız A", src_body(spatial={"relation": "contains"}, limit=None),
           {"Parsel": "p-b"}, building)
    straddle = src_object(_sq("bina", 90, 10, 110, 30))
    record("iki parsele taşan bina: içeren yok, kesişen A ve B",
           src_body(spatial={"relation": "intersects"}, limit=None), {}, straddle)
    record("bir metre toleransla dokunan parsel de kesişir",
           src_body(spatial={"relation": "intersects", "tolerance": 1}, limit=None), {},
           src_object(_sq("bina", 100.5, 100.5, 120, 120)))
    record("kaydın içinde kalan parsel", src_body(spatial={"relation": "within"}, limit=None), {},
           src_object(_sq("ada", -10, 90, 110, 210)))
    near = src_object(sq.point("kuyu", sq.P(150, 150))[1])
    record("en yakın: eşit uzaklıkta listenin ilki", src_body(spatial={"relation": "nearest"}, sort=None, limit=None),
           {}, near)
    record("en yakın, toleransın dışında kalınca hiçbiri",
           src_body(spatial={"relation": "nearest", "tolerance": 40}, sort=None, limit=None), {}, near)
    record("geometrisiz kayda mekânsal liste bir şey sunmaz",
           src_body(spatial={"relation": "contains"}, limit=None), {"Parsel": "p-a"})
    record("şekiller verilmeyen dünyada mekânsal liste daraltmaz",
           src_body(spatial={"relation": "contains"}, limit=None), {"Parsel": "p-b"}, building, world=src_world(False))

    def write(name, before, values, body=SRC_BODY, obj=None, new=False):
        oj, op = obj if obj is not None else src_object()
        after, issues = Form(body, SRC_OWN, SRC_TODAY, wp).assess(before, values, op, new)
        assess.append({"name": f"kaynak: {name}", "fields": SRC_OWN, "body": body, "variables": SRC_TODAY,
                       "world": wj, "object": oj, "before": before, "values": values, "new": new, "after": after,
                       "issues": issues})

    write("etiketle seçilen parselin alanları kopyalanır, damga yazılır", {}, [["Parsel", "101/5"]])
    write("yeni kayıtta da", {}, [["Parsel", "102/1"]], new=True)
    write("damgasız eski değer seçilince kopyalanır", {"Parsel": "p-a"}, [["Kat", "3"]])
    kept = {"Parsel": "p-b", "Ada": "101", "Alan": "999", "Malik": "Ali Can",
            "Kaynak": f"Parsel · p-b · #{fingerprint(['101', '999', 'Ali Can'])} · 2025-01-01"}
    write("değer değişmezse kopya olduğu gibi kalır; kaynağın değiştiği söylenir", kept, [["Kat", "2"]])
    write("değer boşalınca kopyalar kalır", kept, [["Parsel", ""]])
    write("kaynağın boş alanı kopyayı boşaltır", kept, [["Parsel", "102/1"]])

    def guard(name, before, after, body=SRC_BODY):
        oj, op = src_object()
        enforce.append({"name": f"kaynak: {name}", "fields": SRC_OWN, "body": body, "variables": SRC_TODAY,
                        "world": wj, "object": oj, "before": before, "after": after,
                        "issues": Form(body, SRC_OWN, SRC_TODAY, wp).enforce(before, after, op)})

    copied = Form(SRC_BODY, SRC_OWN, SRC_TODAY, wp).copied({}, {"Parsel": "p-a"})
    guard("formun göstermediği kopya hedefleri kopyanın değeriyle yazılabilir", {}, copied)
    guard("kopya hedefine başka değer yazılamaz", {}, {**copied, "Malik": "Başkası"})
    return pre, records, assess, enforce


def calc_cases():
    pre = []

    def case(name, body=CALC_BODY, fields=CALC_FIELDS, variables=()):
        pre.append({"name": name, "fields": fields, "body": body, "variables": list(variables),
                    "issues": preflight(body, fields, variables)})

    case("hesaplı form: anı ve kullanıcıyı okuyan varsayılanlar uyarıdır, hata yok")
    case("anın değerleri verilince de uyarı kalır: izlemede hesaplanmazlar", variables=GIVEN)
    case("değer ve ifade varsayılanı birlikte olmaz",
         calc_with(lambda b: item(b, "kod").update(default="B-0")))
    case("hesaplanan alanın varsayılanı ve kilit kuralı olmaz",
         calc_with(lambda b: (item(b, "yukseklik").update(default="3"), item(b, "riskli").update(readOnlyWhen="Kat > 1"))))
    case("hesaplar birbirini okursa döngüdür: ilki söylenir",
         calc_with(lambda b: (item(b, "yukseklik").update(calculate="Puan + 1"), item(b, "puan").update(calculate="Yükseklik + 1"))))
    case("kendini okuyan hesap da döngüdür", calc_with(lambda b: item(b, "riskli").update(calculate="Riskli")))
    case("listesiz alanın seçenek süzgeci olmaz", calc_with(lambda b: item(b, "kod").update(choiceFilter="@secenek <> ''")))
    case("süzgeç bilinmeyen @ değeri okursa uyarıdır; @secenek bilinir",
         calc_with(lambda b: item(b, "malzeme").update(choiceFilter="@secenek <> @yasak")))
    case("okunamayan hesap ve süzgeç", calc_with(lambda b: (item(b, "taban").update(calculate="$alan *"),
                                                            item(b, "malzeme").update(choiceFilter="@secenek ="))))
    case("notun şablonu açık kalırsa ya da ifadesi okunmazsa hata",
         calc_with(lambda b: item(b, "hesap")["items"].extend([
             {"type": "note", "id": "acik", "text": "Toplam {Kat"}, {"type": "note", "id": "bozuk", "text": "{Kat +}"}])))
    case("zorunlu, salt okunur ve hesaplanan alan doldurulur: sorun değil",
         calc_with(lambda b: item(b, "yukseklik").update(required=True, readOnly=True)))
    case("değiştirilebilir hesap ancak hesapla olur", calc_with(lambda b: item(b, "kod").update(overridable=True)))
    case("değiştirilebilir hesap salt okunur sayılmaz: sorun yok", calc_with(lambda b: item(b, "yukseklik").update(overridable=True)))
    case("katmanın zorunlu alanı hesaplanırsa hep yazılır sayılır",
         calc_with(lambda b: item(b, "kod").update(calculate="'B-' || Kat", defaultExpr=None) or item(b, "kod").pop("defaultExpr")),
         fields=[{**f, "required": True} if f["name"] == "Kod" else f for f in CALC_FIELDS])

    complete = []

    def done(name, attrs, new=False, body=CALC_BODY, variables=GIVEN, obj=SQUARE):
        form = Form(body, CALC_FIELDS, variables)
        out, issues = form.complete(attrs, obj, new)
        complete.append({"name": name, "fields": CALC_FIELDS, "body": body, "variables": list(variables),
                         "object": obj, "attrs": attrs, "new": new, "out": out, "issues": issues})

    done("yeni kayıt: varsayılanlar diğer alanları okur, anın ve kullanıcının değerleri, hesaplar",
         {"Durum": "iyi", "Kat": "3"}, new=True)
    done("yeni kayıt, an ve kullanıcı verilmedi: onları okuyan varsayılanlar boş kalır",
         {"Durum": "iyi", "Kat": "3"}, new=True, variables=())
    done("dolu alana varsayılan yazılmaz", {"Durum": "iyi", "Kat": "3", "Kod": "A-1"}, new=True)
    done("var olan kayıt: varsayılan yok, hesaplar yenilenir",
         {"Durum": "iyi", "Kat": "7", "Yükseklik": "1", "Taban": "7", "Riskli": "false"})
    done("evet/hayır hesabı true ya da false yazılır", {"Durum": "iyi", "Kat": "6"})
    done("alanın kuralına uymayan hesap hatadır; değer değişmez", {"Durum": "iyi", "Kat": "15", "Puan": "3"})
    done("ölçüsüz nesne: $alan boştur, alan boşalır", {"Durum": "iyi", "Kat": "2", "Taban": "5"}, obj={})
    done("uymayan ifade varsayılanı uyarıdır, konmaz",
         {"Durum": "iyi", "Kat": "2"}, new=True, body=calc_with(lambda b: item(b, "tarih").update(defaultExpr="'bugün'")))
    done("gizlenenler silinir: Durum iyi olunca açıklama gider",
         {"Durum": "iyi", "Kat": "2", "Açıklama": "Eski not, uzunca bir not."},
         body=calc_with(lambda b: b.update(clearHidden=True)))
    over = calc_with(lambda b: item(b, "yukseklik").update(overridable=True))
    done("değiştirilebilir hesap: boş alan hesapla dolar", {"Durum": "iyi", "Kat": "3"}, body=over)
    done("değiştirilebilir hesap: yazılmış değer kalır, öbür hesaplar yenilenir",
         {"Durum": "iyi", "Kat": "4", "Yükseklik": "10", "Puan": "1"}, body=over)
    done("gizlenen grubun hesabı yapılmaz ve silinir",
         {"Durum": "iyi", "Kat": "2", "Yükseklik": "6"},
         body=calc_with(lambda b: (b.update(clearHidden=True), item(b, "hesap").update(visibleWhen="Durum = 'inceleme'"))))

    records = []

    def record(name, attrs, body=CALC_BODY, variables=GIVEN, obj=SQUARE):
        form = Form(body, CALC_FIELDS, variables)
        records.append({"name": name, "fields": CALC_FIELDS, "body": body, "variables": list(variables),
                        "object": obj, "attrs": attrs, "items": form.described(attrs, obj),
                        "issues": form.validate(attrs, obj)})

    record("sanayi yapısında süzgeç beton ve çeliği sunar; not değerleriyle yazılır; hesaplananlar salt okunur",
           {"Durum": "iyi", "Kat": "3", "Taban": "100", "Tür": "sanayi", "Malzeme": "beton"})
    record("süzgecin sunmadığı değer hatadır", {"Durum": "iyi", "Kat": "3", "Tür": "sanayi", "Malzeme": "ahsap"})
    record("konut her malzemeyi sunar", {"Durum": "iyi", "Kat": "3", "Tür": "konut", "Malzeme": "ahsap"})
    record("uyarı denetimi kaydı durdurmaz; sert denetim durdurur",
           {"Durum": "inceleme", "Kat": "3", "Açıklama": "Çatı"})
    record("yalnız uyarı", {"Durum": "inceleme", "Kat": "3", "Açıklama": "Çatı arası bakılmalı."})
    old_fields = [dict(f, values=[dict(c, retired=True) if c["code"] == "ahsap" else c for c in f["values"]])
                  if f["name"] == "Malzeme" else f for f in CALC_FIELDS]
    for name, attrs in (("kalkmış değer sunulmaz", {"Durum": "iyi", "Kat": "3", "Tür": "konut", "Malzeme": "beton"}),
                        ("kalkmış değeri tutan kayda o da sunulur, geçerlidir", {"Durum": "iyi", "Kat": "3", "Tür": "konut", "Malzeme": "ahsap"})):
        form = Form(CALC_BODY, old_fields, GIVEN)
        records.append({"name": name, "fields": old_fields, "body": CALC_BODY, "variables": GIVEN, "object": SQUARE,
                        "attrs": attrs, "items": form.described(attrs, SQUARE), "issues": form.validate(attrs, SQUARE)})
    record("değiştirilebilir hesap açıktır; öbür hesaplar salt okunur",
           {"Durum": "iyi", "Kat": "3", "Yükseklik": "10"}, body=calc_with(lambda b: item(b, "yukseklik").update(overridable=True)))

    assess = []

    def write(name, before, values, new=False, body=CALC_BODY, variables=GIVEN, obj=SQUARE):
        after, issues = Form(body, CALC_FIELDS, variables).assess(before, values, obj, new)
        assess.append({"name": name, "fields": CALC_FIELDS, "body": body, "variables": list(variables), "object": obj,
                       "before": before, "values": values, "new": new, "after": after, "issues": issues})

    base = {"Durum": "iyi", "Kat": "3", "Yükseklik": "9", "Taban": "100", "Riskli": "false", "Puan": "2"}
    write("yeni kayıt: verilen değerler önce, sonra varsayılanlar ve hesaplar", {}, [["Durum", "İyi"], ["kat-sayisi", "5"]], new=True)
    write("hesaplanan alana başka değer yazılamaz", base, [["Yükseklik", "12"]])
    write("hesaplanan alana hesabın değeri yazılabilir", base, [["kat-sayisi", "4"], ["Yükseklik", "12"]])
    write("kat değişince hesaplar yenilenir", base, [["kat-sayisi", "8"]])
    write("kat değişince alana uymayan hesap yazmayı durdurur", base, [["kat-sayisi", "12"]])
    write("gizlenenleri silen formda gizli alana yazmak reddedilmez; değer silinir",
          {**base, "Durum": "inceleme", "Açıklama": "Eski not, uzunca bir not."},
          [["Durum", "iyi"], ["Açıklama", "Yeni not, gene uzun bir not."]], body=calc_with(lambda b: b.update(clearHidden=True)))
    write("uyarı yazmayı durdurmaz", {**base, "Durum": "inceleme"}, [["Açıklama", "Kısa not"]])
    over = calc_with(lambda b: item(b, "yukseklik").update(overridable=True))
    write("değiştirilebilir hesaba başka değer yazılır", base, [["Yükseklik", "10"]], body=over)
    write("değiştirilebilir hesap boşaltılınca yeniden hesaplanır", {**base, "Yükseklik": "10"}, [["Yükseklik", ""]], body=over)
    write("kat değişince yazılmış değer kalır", {**base, "Yükseklik": "10"}, [["kat-sayisi", "8"]], body=over)
    old = [dict(f, values=[dict(c, retired=True) if c["code"] == "ahsap" else c for c in f["values"]]) if f["name"] == "Malzeme" else f
           for f in CALC_FIELDS]

    def aged(name, before, values):
        after, issues = Form(CALC_BODY, old, GIVEN).assess(before, values, SQUARE, False)
        assess.append({"name": name, "fields": old, "body": CALC_BODY, "variables": GIVEN, "object": SQUARE,
                       "before": before, "values": values, "new": False, "after": after, "issues": issues})

    aged("kullanımdan kalkmış değer yeni yazılamaz", {**base, "Tür": "konut"}, [["Malzeme", "ahsap"]])
    aged("kayıt tuttuğu kalkmış değeri korur, başka alanı değişir", {**base, "Tür": "konut", "Malzeme": "ahsap"}, [["kat-sayisi", "4"]])
    aged("kalkmış değerden geçerli değere geçilir", {**base, "Tür": "konut", "Malzeme": "ahsap"}, [["Malzeme", "tas"]])

    enforce = []

    def guard(name, before, after, variables=GIVEN, obj=SQUARE, body=CALC_BODY):
        enforce.append({"name": name, "fields": CALC_FIELDS, "body": body, "variables": list(variables), "object": obj,
                        "before": before, "after": after,
                        "issues": Form(body, CALC_FIELDS, variables).enforce(before, after, obj)})

    guard("başka yoldan kat değişir: hesaplar kendiliğinden, sorun yok", base, {**base, "Kat": "4"})
    guard("başka yoldan hesaplanan alan değişmez", base, {**base, "Yükseklik": "99"})
    guard("başka yoldan süzgecin sunmadığı malzeme", {**base, "Tür": "sanayi"}, {**base, "Tür": "sanayi", "Malzeme": "tas"})
    guard("başka yoldan değiştirilebilir hesap değişir: sorun yok", base, {**base, "Yükseklik": "99"}, body=over)

    unique = []
    others = [{"Kod": "B-1"}, {"Kod": "B-2", "Kat": "2"}, {}]
    for name, attrs in (("başka nesnede olan kod", {"Durum": "iyi", "Kod": "B-2"}),
                        ("tekil kod", {"Durum": "iyi", "Kod": "B-3"}),
                        ("boş kod sorulmaz", {"Durum": "iyi"})):
        form = Form(CALC_BODY, CALC_FIELDS, GIVEN)
        unique.append({"name": name, "fields": CALC_FIELDS, "body": CALC_BODY, "variables": GIVEN, "object": SQUARE,
                       "attrs": attrs, "others": others, "issues": form.unique_issues(attrs, SQUARE, others)})

    return pre, complete, records, assess, enforce, unique


def outputs():
    return {ROOT / "fixtures/forms/v1/cases.json": core(), **commands()}


# ── Command cases ────────────────────────────────────────────────────────

STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}


def layer(i, name, fields=None, locked=False, kind="layer", children=None):
    out = {"id": i, "name": name, "type": kind, "visible": True, "locked": locked, "expanded": True, "style": STYLE,
           "children": children or []}
    if fields:
        out["fields"] = fields
    return out


def P(x, y):
    return {"x": x, "y": y}


E0, N0 = 487000, 4420000
YOL_FIELDS = [{"name": "Ad", "kind": "text"}]
LAYERS = [
    layer("bina", "Bina", FIELDS),
    layer("yol", "Yol", YOL_FIELDS),
    layer("kilitli", "Kilitli bina", FIELDS, locked=True),
    layer("grup", "Grup", kind="group", children=[layer("alt", "Alt katman")]),
]
FIELDS_OF = {"bina": FIELDS, "yol": YOL_FIELDS, "kilitli": FIELDS, "alt": []}
FIRST = {"Durum": "inceleme", "Kat": "3", "Açıklama": "Çatı arası ayrıca bakılmalı.", "Kod": "B-12"}


def square(x, y, side=10):
    return [P(E0 + x, N0 + y), P(E0 + x + side, N0 + y), P(E0 + x + side, N0 + y + side), P(E0 + x, N0 + y + side)]


ENTITIES = [
    {"kind": "polygon", "id": 1, "layerId": "bina", "attrs": FIRST, "pts": square(0, 0)},
    {"kind": "polygon", "id": 2, "layerId": "bina", "attrs": {}, "pts": square(20, 0)},
    {"kind": "line", "id": 3, "layerId": "yol", "attrs": {"Ad": "Atatürk Caddesi"}, "a": P(E0 - 10, N0 - 5), "b": P(E0 + 60, N0 - 5)},
    {"kind": "polygon", "id": 4, "layerId": "kilitli", "attrs": {"Durum": "iyi"}, "pts": square(40, 0)},
]
IDS = [e["id"] for e in ENTITIES]
SETTINGS = {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
            "workspace": "gis", "drawingFont": "barlow"}
UNKNOWN_UID = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6dffff"


def setup(forms=None, fields=None, entities=None):
    settings = dict(SETTINGS)
    if forms:
        settings["forms"] = forms
    layers = LAYERS
    if fields is not None:
        layers = copy.deepcopy(LAYERS)
        layers[0]["fields"] = fields
    return {"format": "kentos.document", "version": 1, "name": "Formlar", "settings": settings, "origin": P(E0, N0),
            "layers": layers, "activeLayer": "bina", "entities": entities or ENTITIES, "styles": {"items": [], "categories": []}}


# The objects' measures as the rules see them (a square of 10 m): what `$alan`, `$uzunluk` and `$köşe` read.
OBJECT_OF = {1: SQUARE, 2: SQUARE, 4: SQUARE, 3: {"length": 70, "vertices": 2}}


def failed(code, message, path=None, status="failed"):
    error = {"code": code, "message": message}
    if path is not None:
        error["path"] = path
    if code == "revision_conflict":
        error["revision"] = "$current"
    return {"status": status, "error": error}


ANY = "$any"
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."
REVISION = "Beklenen sürüm “07” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."
NAME_TEXT = "Form adı boş olamaz, en çok 64 karakterdir ve denetim karakteri içermez. Başka bir ad yazın."
NOTE_TEXT = "Yayın notu en çok 1000 karakterdir ve denetim karakteri içermez."
TOO_MANY = "Projede en çok 100 form olur. Kullanmadığınız birini silin."
INVALID_UID = "“{}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
NOT_FOUND = "“{}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."


def unknown_form(i, path):
    return failed("unknown_form", f"“{i}” kimlikli bir form yok. Formlar penceresinden var olan bir formu seçin.", path)


def bad_id(i):
    return failed("invalid_form_id", f"“{i}” bir form kimliği olamaz: 1–64 harf, rakam, -, _ ya da . olmalı.", "edit.id")


def unpublished(name, path):
    text = (f"“{name}” formu hiç yayınlanmadı; kuralları uygulanmadan önce yayınlayın." if path == "edit.on"
            else f"“{name}” formu hiç yayınlanmadı. Önce yayınlayın ya da taslağıyla çalışın.")
    return failed("form_unpublished", text, path)


def mismatch(name):
    return failed("form_layer_mismatch", f"Nesne “{name}” formunun katmanında değil. Formun katmanındaki bir nesneyi seçin.", "uid")


def requirements(body):
    """What a form needs of its host (docs/adr/0441 §4), written from the ADR: the moment and the user its rules read,
    other layers, the record's geometry, attachments, a workflow, then each action's tool once."""
    texts, layers, geometry, attachments, tools = [], False, False, False, []

    def walk(xs):
        nonlocal layers, geometry, attachments
        for x in xs:
            kind = x["type"]
            if kind == "note":
                texts.append(x["text"])
            elif kind == "field":
                for k in ("visibleWhen", "requiredWhen", "readOnlyWhen", "defaultExpr", "calculate", "choiceFilter"):
                    if k in x:
                        texts.append(x[k])
                texts.extend(c["rule"] for c in x.get("checks", []))
                if "lookup" in x:
                    layers = True
                    geometry = geometry or "spatial" in x["lookup"]
                    if "filter" in x["lookup"]:
                        texts.append(x["lookup"]["filter"])
            else:
                if "visibleWhen" in x:
                    texts.append(x["visibleWhen"])
                if kind == "related":
                    layers = True
                elif kind == "attachments":
                    attachments = True
                elif kind == "location":
                    geometry = True
                elif kind == "action":
                    # docs/adr/0442 §5: a tool its id; showing the record its geometry.
                    if "tool" in x and x["tool"] not in tools:
                        tools.append(x["tool"])
                    geometry = geometry or x.get("kind") == "show"
            walk(x.get("items", []))

    walk(body["items"])
    plain = str.maketrans("İIıŞşÇçĞğÖöÜü", "iiissccggoouu")
    folded = [t.translate(plain).lower() for t in texts]
    reads = lambda name: any(name in t for t in folded)
    keys = [k for k, on in [("moment", reads("@tarih") or reads("@simdi")), ("user", reads("@kullanici")),
                            ("layers", layers), ("geometry", geometry), ("attachments", attachments),
                            ("workflow", "workflow" in body)] if on]
    return keys + [f"tool:{x}" for x in tools]


def with_requires(out, body):
    """A validate answer with its form's requirements (docs/adr/0441 §4), written only when there are any."""
    r = requirements(body)
    if r:
        out["requires"] = r
    return out


def shown_issues(issues):
    return [{**i, "message": ANY} for i in issues]


def summary(f):
    out = {"id": f["id"], "name": f["name"], "layer": f["layer"], "revision": f["draft"]["revision"]}
    if f.get("releases"):
        out["version"] = f["releases"][-1]["version"]
    if f.get("enforce"):
        out["enforce"] = True
    return out


def form_json(f):
    out = {k: v for k, v in f.items() if not (k == "releases" and not v) and not (k == "enforce" and not v)}
    return copy.deepcopy(out)


def release_of(body, version, note=None):
    out = {"version": version, "digest": digest(body), "body": copy.deepcopy(body)}
    if note:
        out["note"] = note
    return out


def utf8_sorted(d):
    return sorted(d.items(), key=lambda kv: kv[0].encode("utf-8"))


def record_errors(body, fields, entities, layer="bina", objects=None):
    """Each record's errors by `body`, completed as kept then validated (Kayıtları denetle's judgement, unique
    values aside; docs/adr/0442 §2)."""
    f = Form(body, fields)
    out = []
    for e in entities:
        if e["layerId"] != layer:
            continue
        obj = (objects or OBJECT_OF).get(e["id"], {})
        after, issues = f.complete(e["attrs"], obj, False)
        said = [i.get("field") for i in issues]
        issues = issues + [i for i in f.validate(after, obj) if i.get("field") not in said]
        out.append(errors(issues))
    return out


def breaking_of(old, new, fields, entities, layer="bina", objects=None):
    """The records `new` breaks that `old` held: how many, and how many by each field (keys in bytes' order)."""
    before = record_errors(old, fields, entities, layer, objects)
    after = record_errors(new, fields, entities, layer, objects)
    records, by_field = 0, {}
    for was, now in zip(before, after):
        if not was and now:
            records += 1
            for k in sorted({i["field"] for i in now if i.get("field")}, key=lambda s: s.encode()):
                by_field[k] = by_field.get(k, 0) + 1
    if not records:
        return None
    return {"records": records, "fields": [{"field": k, "records": n}
                                           for k, n in sorted(by_field.items(), key=lambda kv: kv[0].encode())]}


def breaking_warning(name, b):
    parts = ", ".join(f"{f['field']} {f['records']}" for f in b["fields"])
    return {"code": "form_breaking", "path": "edit.id",
            "message": f"“{name}” formunun yeni yayınına {b['records']} kayıt uymuyor (önceki yayına uyuyordu): {parts}. "
                       "Yayından önce kayıtları düzeltin ya da formu gözden geçirin."}


class Project:
    """The forms the edit cases go through, by the ADR's rule."""

    def __init__(self, forms=None, fields_of=None, entities=None, objects=None):
        self.forms = copy.deepcopy(forms or [])
        # The records a publish judges (docs/adr/0442 §2).
        self.fields_of = fields_of or FIELDS_OF
        self.entities = entities or ENTITIES
        self.objects = objects

    def ids(self):
        return [f["id"] for f in self.forms]

    def find(self, i):
        return next(f for f in self.forms if f["id"] == i)

    def step(self, op, **more):
        old = copy.deepcopy(self.forms)
        warnings, kept, more_out = [], None, {}
        k = op["kind"]
        if k == "create":
            name = op["name"].strip()
            if "id" in op:
                i = op["id"]
            else:
                stem = op["name"] if any(c.isalnum() for c in op["name"]) else "form"
                i = item_id_of(stem, self.ids())
            draft = copy.deepcopy(op["body"]) if "body" in op else generate(FIELDS_OF[op["layer"]])
            draft["revision"] = 1
            kept = {"id": i, "name": name, "layer": op["layer"], "draft": draft}
            self.forms.append(kept)
        elif k == "update":
            f = self.find(op["id"])
            if (op["body"]["items"], op["body"].get("clearHidden")) != (f["draft"]["items"], f["draft"].get("clearHidden")):
                f["draft"] = {"revision": f["draft"]["revision"] + 1, "items": copy.deepcopy(op["body"]["items"])}
                if op["body"].get("clearHidden"):
                    f["draft"]["clearHidden"] = True
            kept = f
        elif k == "rename":
            f = self.find(op["id"])
            f["name"] = op["name"].strip()
            kept = f
        elif k == "remove":
            self.forms = [f for f in self.forms if f["id"] != op["id"]]
        elif k == "publish":
            f = self.find(op["id"])
            rel = f.setdefault("releases", [])
            broken = None
            if rel and rel[-1]["body"] != f["draft"]:
                broken = breaking_of(rel[-1]["body"], f["draft"], self.fields_of[f["layer"]], self.entities, f["layer"],
                                     self.objects)
                if broken:
                    warnings.append(breaking_warning(f["name"], broken))
                    more_out["breaking"] = broken
            if not rel or rel[-1]["body"] != f["draft"]:
                note = op.get("note")
                rel.append(release_of(f["draft"], rel[-1]["version"] + 1 if rel else 1, note if note and note.strip() else None))
                if len(rel) > 50:
                    gone = rel.pop(0)
                    warnings.append({"code": "form_release_dropped", "path": "edit.id",
                                     "message": f"“{f['name']}” formu en çok 50 yayın tutar; en eski yayını ({gone['version']}. sürüm) kaldırıldı."})
            if not rel:
                f.pop("releases")
            kept = f
        elif k == "enforce":
            f = self.find(op["id"])
            if op["on"]:
                f["enforce"] = True
            else:
                f.pop("enforce", None)
            kept = f
        elif k == "comment":
            # docs/adr/0441 §5: numbered one past the form's last, its text trimmed.
            f = self.find(op["id"])
            cs = f.setdefault("comments", [])
            c = {"id": max([x["id"] for x in cs], default=0) + 1}
            for key in ("item", "author", "at"):
                if key in op:
                    c[key] = op[key]
            c["text"] = op["text"].strip()
            cs.append(c)
            kept = f
        elif k == "resolve":
            f = self.find(op["id"])
            c = next(x for x in f["comments"] if x["id"] == op["comment"])
            if op["resolved"]:
                c["resolved"] = True
            else:
                c.pop("resolved", None)
            kept = f
        changed = self.forms != old
        output = {"forms": [summary(f) for f in self.forms], "changed": changed, "revision": "$current",
                  "issues": shown_issues(preflight(kept["draft"], self.fields_of[kept["layer"]])) if kept else []}
        if kept is not None:
            output["form"] = form_json(kept)
        output.update(more_out)
        step = {"op": "execute", "input": {"edit": op, **more},
                "result": {"status": "completed", "output": output, "warnings": warnings}}
        expect = {"ids": IDS, "settings": {"forms": [form_json(f) for f in self.forms] or None}, "canUndo": False, "canRedo": False}
        expect.update({"dirty": True, "revision": "changed"} if changed else {"revision": "same"})
        step["expect"] = expect
        return step

    def refused(self, op, result, dirty=False, **more):
        expect = {"ids": IDS, "settings": {"forms": [form_json(f) for f in self.forms] or None}, "canUndo": False, "canRedo": False, "revision": "same"}
        if not dirty:
            expect["dirty"] = False
        return {"op": "execute", "input": {"edit": op, **more}, "result": result, "expect": expect}


def create(name, layer_id="bina", **more):
    return {"kind": "create", "name": name, "layer": layer_id, **more}


FORM = {"id": "denetim", "name": "Bina denetimi", "layer": "bina", "draft": copy.deepcopy(BODY)}
PUBLISHED = {**copy.deepcopy(FORM), "releases": [release_of(BODY, 1, "İlk yayın")]}
ENFORCED = {**copy.deepcopy(PUBLISHED), "enforce": True}
BROKEN = body_with(lambda b: item(b, "kat").update(field="Yok"))
LOCKED_FORM = {"id": "kilitli", "name": "Kilitli bina", "layer": "kilitli", "draft": copy.deepcopy(BODY),
               "releases": [release_of(BODY, 1)]}
# docs/adr/0432: the same form through a workflow whose “inceleme” state is a draft.
DRAFT_BODY = {**copy.deepcopy(BODY), "workflow": {"field": "Durum", "initial": "inceleme", "drafts": ["inceleme"], "steps": [
    {"label": "Onayla", "from": ["inceleme"], "to": "iyi", "valid": True}]}}
DRAFT_FORM = {"id": "taslakli", "name": "Taslaklı denetim", "layer": "bina", "draft": copy.deepcopy(DRAFT_BODY),
              "releases": [release_of(DRAFT_BODY, 1)]}


CALC_FORM = {"id": "hesap", "name": "Bina hesabı", "layer": "bina", "draft": copy.deepcopy(CALC_BODY),
             "releases": [release_of(CALC_BODY, 1)]}
CALC_ENFORCED = {**copy.deepcopy(CALC_FORM), "enforce": True}
CALC_FIRST = {"Durum": "iyi", "Kat": "3", "Kod": "B-3", "Tür": "konut", "Yükseklik": "9", "Taban": "100",
              "Riskli": "false", "Puan": "2"}
CALC_ENTITIES = [{**e, "attrs": CALC_FIRST} if e["id"] == 1 else {**e, "attrs": {}} if e["id"] in (2, 4) else e
                 for e in ENTITIES]
# A command's `@` values: the moment's and the user's when given (the project's own come with every command).
MOMENT_GIVEN = {"moment": "2026-10-16T14:30:00", "user": "Ayşe Yılmaz"}


def calc_setup(form=CALC_FORM, entities=None):
    return setup([form], CALC_FIELDS, entities or CALC_ENTITIES)


def edit_cases():
    cases = []

    p = Project()
    steps = [p.step(create("Bina denetimi")), p.step(create("  Bina denetimi  "))]
    assert p.ids() == ["bina-denetimi", "bina-denetimi-2"]
    cases.append({"name": "Yeni form: katmanın alanlarından üretilir, kimliği adından; aynı adla ikincisi numaralanır, ad kırpılır; proje ayarıdır, geri alma adımı değildir", "steps": steps})

    p = Project()
    steps = [p.step(create("Denetim", id="denetim", body={**copy.deepcopy(BODY), "revision": 7}))]
    assert p.find("denetim")["draft"]["revision"] == 1
    cases.append({"name": "verilen kimlik ve gövdeyle: taslak 1. sürümden başlar", "steps": steps})

    p = Project()
    steps = [p.step(create("---")), p.step(create("Yol formu", "yol")), p.step(create("Boş katman", "alt"))]
    cases.append({"name": "adında harf olmayan formun kimliği “form”; alanı olmayan katmanın formu boştur ve uyarır", "steps": steps})

    p = Project()
    steps = [p.step(create("Bozuk", id="bozuk", body=BROKEN))]
    cases.append({"name": "ön denetimde hata olan taslak saklanır; çıktı sorunları söyler", "steps": steps})

    p = Project()
    cases.append({"name": "geçersiz kimlik, ad, gövde ve not; sıra: kimlik, ad, gövde", "steps": [
        p.refused(create("A", id="a b"), bad_id("a b")),
        p.refused(create("A", id=""), bad_id("")),
        p.refused(create("   ", id="a b"), bad_id("a b")),
        p.refused(create("   "), failed("invalid_form_name", NAME_TEXT, "edit.name")),
        p.refused(create("A\tB"), failed("invalid_form_name", NAME_TEXT, "edit.name")),
        p.refused(create("A", body={"revision": 1, "items": [{"type": "note", "id": "x y", "text": "n"}]}), failed("invalid_form", ANY, "edit.body")),
        # docs/adr/0441 §6: an action names a registered tool by its id; an address is none.
        p.refused(create("A", body={"revision": 1, "items": [{"type": "action", "id": "git", "label": "Git",
                                                              "tool": "https://ornek.org/x"}]}),
                  failed("invalid_form", ANY, "edit.body")),
        # docs/adr/0442 §5: each kind its own target.
        p.refused(create("A", body={"revision": 1, "items": [{"type": "action", "id": "git", "label": "Git", "kind": "show",
                                                              "tool": "attributes.calculate"}]}),
                  failed("invalid_form", ANY, "edit.body")),
        p.refused(create("A", body={"revision": 1, "items": [{"type": "action", "id": "git", "label": "Git", "kind": "step"}]}),
                  failed("invalid_form", ANY, "edit.body")),
        p.refused(create("A", body={"revision": 1, "items": [{"type": "action", "id": "git", "label": "Git",
                                                              "kind": "recompute", "runs": "here"}]}),
                  failed("invalid_form", ANY, "edit.body")),
        p.refused({"kind": "publish", "id": "denetim", "note": "a\u0007"}, failed("invalid_form", NOTE_TEXT, "edit.note")),
    ]})

    p = Project([FORM])
    cases.append({"name": "aynı kimlikle form, olmayan ve grup katman", "setup": setup([FORM]), "steps": [
        p.refused(create("İkinci", id="denetim"), failed("form_exists", "“denetim” kimlikli başka bir form var. Başka bir kimlik verin.", "edit.id")),
        p.refused(create("Yeni", "yok"), failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "edit.layer")),
        p.refused(create("Yeni", "grup"), failed("not_a_layer", "“Grup” bir katman grubu; form yalnız katmanın olur. Grubun içinden bir katman seçin.", "edit.layer")),
    ]})

    many = [{"id": f"f{k}", "name": f"Form {k}", "layer": "bina", "draft": {"revision": 1, "items": []}} for k in range(100)]
    p = Project(many)
    cases.append({"name": "100 form varken yenisi reddedilir; var olan adlandırılır", "setup": setup(many), "steps": [
        p.refused(create("Yeni"), failed("too_many_forms", TOO_MANY, "edit")),
        p.step({"kind": "rename", "id": "f7", "name": "Yedi"}),
    ]})

    p = Project([FORM])
    newer = body_with(lambda b: item(b, "kat").update(help="Zemin ve çatı katı dahil."))
    steps = [
        p.step({"kind": "update", "id": "denetim", "body": newer}),
        p.step({"kind": "update", "id": "denetim", "body": {**newer, "revision": 2}}),
    ]
    stale = {"kind": "update", "id": "denetim", "body": body_with(lambda b: b.update(revision=1))}
    steps.append(p.refused(stale, failed("form_draft_changed", "“Bina denetimi” formunun taslağı 2. sürümde; girdi 1. sürüme göre hazırlanmış. Taslağı yeniden okuyup değişikliği onun üstüne yapın.", "edit.body.revision", "conflict"), dirty=True))
    assert p.find("denetim")["draft"]["revision"] == 2
    cases.append({"name": "Kaydet: taslağın yerine geçer, sürümü bir artar; aynısını kaydetmek değiştirmez; eski sürüme göre hazırlanan reddedilir (conflict)", "setup": setup([FORM]), "steps": steps})

    p = Project([FORM])
    steps = [p.step({"kind": "update", "id": "denetim", "body": BROKEN})]
    cases.append({"name": "Kaydet'te ön denetim engel değildir: sorunlu taslak saklanır", "setup": setup([FORM]), "steps": steps})

    p = Project([FORM])
    steps = [p.step({"kind": "rename", "id": "denetim", "name": "  Yapı denetimi "}),
             p.refused({"kind": "rename", "id": "yok", "name": "X"}, unknown_form("yok", "edit.id"), dirty=True)]
    cases.append({"name": "Yeniden adlandır: ad kırpılır, kimlik kalır; olmayan form", "setup": setup([FORM]), "steps": steps})

    p = Project([FORM, {**copy.deepcopy(FORM), "id": "ikinci", "name": "İkinci"}])
    steps = [p.step({"kind": "remove", "id": "denetim"}),
             p.refused({"kind": "remove", "id": "denetim"}, unknown_form("denetim", "edit.id"), dirty=True),
             p.step({"kind": "remove", "id": "ikinci"})]
    assert p.forms == []
    cases.append({"name": "Sil: öbürlerinin sırası kalır; son form silinince ayar yazılmaz; silinmiş form reddedilir",
                  "setup": setup([FORM, {**copy.deepcopy(FORM), "id": "ikinci", "name": "İkinci"}]), "steps": steps})

    p = Project([FORM])
    steps = [
        p.step({"kind": "publish", "id": "denetim", "note": "İlk yayın"}),
        p.step({"kind": "publish", "id": "denetim", "note": "Değişmeden"}),
        p.step({"kind": "update", "id": "denetim", "body": newer}),
        p.step({"kind": "publish", "id": "denetim", "note": "  "}),
    ]
    assert [r["version"] for r in p.find("denetim")["releases"]] == [1, 2]
    cases.append({"name": "Yayınla: taslak sıradaki yayın olur, özetiyle; değişmemiş taslak yeniden yayınlanmaz; boş not yazılmaz", "setup": setup([FORM]), "steps": steps})

    p = Project([{**copy.deepcopy(FORM), "draft": BROKEN}])
    cases.append({"name": "ön denetimde hata olan taslak yayınlanmaz", "setup": setup([{**copy.deepcopy(FORM), "draft": BROKEN}]), "steps": [
        p.refused({"kind": "publish", "id": "denetim"}, failed("form_preflight", ANY, "edit.id")),
    ]})

    fifty = {**copy.deepcopy(FORM), "draft": copy.deepcopy(newer) | {"revision": 2}, "releases": [release_of(BODY, k) for k in range(1, 51)]}
    p = Project([fifty])
    steps = [p.step({"kind": "publish", "id": "denetim"})]
    assert [r["version"] for r in p.find("denetim")["releases"]][:2] == [2, 3]
    cases.append({"name": "51. yayın en eskisini düşürür, uyarıyla", "setup": setup([fifty]), "steps": steps})

    p = Project([FORM])
    steps = [p.refused({"kind": "enforce", "id": "denetim", "on": True}, unpublished("Bina denetimi", "edit.on")),
             p.step({"kind": "publish", "id": "denetim"}),
             p.step({"kind": "enforce", "id": "denetim", "on": True}),
             p.step({"kind": "enforce", "id": "denetim", "on": True}),
             p.step({"kind": "enforce", "id": "denetim", "on": False})]
    cases.append({"name": "Kuralları uygula yayın ister; açmak, yeniden açmak ve kapatmak", "setup": setup([FORM]), "steps": steps})

    p = Project([FORM])
    steps = [p.refused({"kind": "enforce", "id": "denetim", "on": False}, unknown_form("denetim", "edit.id"))] if False else []
    steps = [p.step({"kind": "enforce", "id": "denetim", "on": False})]
    cases.append({"name": "yayınsız formun kurallarını kapatmak serbesttir, bir şey değiştirmez", "setup": setup([FORM]), "steps": steps})

    p = Project()
    steps = [p.step(create("Kilitli", "kilitli"))]
    cases.append({"name": "kilitli katmanın formu da oluşturulur: form proje ayarıdır, nesneler değişmez", "steps": steps})

    p = Project()
    steps = [p.refused(create("A"), failed("invalid_revision", REVISION, "expectedRevision"), expectedRevision="07"),
             {**p.refused(create("A"), failed("revision_conflict", CONFLICT, "expectedRevision", "conflict"), expectedRevision="999")}]
    steps.append(p.step(create("A"), expectedRevision="$current"))
    cases.append({"name": "beklenen sürüm: geçersiz yazı ve eskisi reddedilir; doğrusu yazar", "steps": steps})

    p = Project()
    long_name = "Ç" * 64
    steps = [p.step(create(long_name, id="x" * 64))]
    cases.append({"name": "sınırlar: 64 karakterlik ad ve kimlik", "steps": steps})

    p = Project([FORM])
    steps = [p.refused({"kind": "update", "id": "yok", "body": BODY}, unknown_form("yok", "edit.id")),
             p.refused({"kind": "publish", "id": "yok"}, unknown_form("yok", "edit.id")),
             p.refused({"kind": "enforce", "id": "yok", "on": True}, unknown_form("yok", "edit.id"))]
    cases.append({"name": "olmayan formu kaydetmek, yayınlamak ve uygulamak: unknown_form", "setup": setup([FORM]), "steps": steps})

    p = Project([FORM])
    added = body_with(lambda b: item(b, "denetim")["items"].append({"type": "field", "id": "yeni", "field": "Yok", "visibleWhen": "Kod = 'A'"}))
    steps = [p.step({"kind": "update", "id": "denetim", "body": added})]
    cases.append({"name": "taslağın sorunları çıktıda: olmayan alan hata, katmanda olmayan ad uyarı", "setup": setup([FORM]), "steps": steps})

    # docs/adr/0441 §5: the reviewers' comments.
    p = Project([FORM])
    steps = [p.step({"kind": "comment", "id": "denetim", "text": "  Kat alanı zorunlu olmalı.  ", "item": "kat",
                     "author": "Ayşe", "at": "2026-10-17 10:00"}),
             p.step({"kind": "comment", "id": "denetim", "text": "Formun adı uygun."}),
             p.step({"kind": "resolve", "id": "denetim", "comment": 1, "resolved": True}),
             p.step({"kind": "resolve", "id": "denetim", "comment": 1, "resolved": True}),
             p.step({"kind": "resolve", "id": "denetim", "comment": 1, "resolved": False})]
    cases.append({"name": "yorumlar: numaralı, öğeye ya da forma; çözülür ve yeniden açılır", "setup": setup([FORM]),
                  "steps": steps})
    p = Project([FORM])
    cases.append({"name": "yorumun öğesi taslakta olmalı, numarası formda", "setup": setup([FORM]), "steps": [
        p.refused({"kind": "comment", "id": "denetim", "text": "x", "item": "yok"},
                  failed("form_item_unknown", "“Bina denetimi” formunun taslağında “yok” öğesi yok.", "edit.item")),
        p.refused({"kind": "resolve", "id": "denetim", "comment": 7, "resolved": True},
                  failed("form_comment_unknown", "“Bina denetimi” formunda 7 numaralı yorum yok.", "edit.comment")),
        p.refused({"kind": "comment", "id": "denetim", "text": "   "},
                  failed("invalid_form", "Yorum boş olamaz, en çok 1000 karakterdir ve denetim karakteri içermez.", "edit.text")),
        p.refused({"kind": "comment", "id": "yok", "text": "x"}, failed("unknown_form", ANY, "edit.id"))]})

    return cases


def validate_cases():
    cases = []

    def run(inp, result, forms=None):
        return {"op": "execute", "input": inp, "result": result,
                "expect": {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}}

    def answer(form, body, version, uid_attrs=None, values=(), defaults=False, fields=FIELDS, variables=(), slot=1,
               others=()):
        issues = preflight(body, fields, variables)
        out = {"issues": [], "valid": True, "items": []}
        if version is not None:
            out["version"] = version
        if uid_attrs is not None and not errors(issues):
            f = Form(body, fields, variables)
            obj = OBJECT_OF.get(slot, {})
            after, found = f.assess(dict(uid_attrs), utf8_sorted(dict(values)), obj, defaults)
            out["items"] = f.described(after, obj)
            issues = issues + found + f.unique_issues(after, obj, others)
            out["attrs"] = after
        out["issues"] = shown_issues(issues)
        out["valid"] = not errors(issues)
        return {"status": "completed", "output": with_requires(out, body), "warnings": []}

    S = setup([PUBLISHED])
    cases.append({"name": "form tek başına: son yayının ön denetimi", "setup": S, "steps": [
        run({"form": "denetim"}, answer(PUBLISHED, BODY, 1))]})
    cases.append({"name": "kayıtla: öğelerin durumu ve sorunlar; değerler yokken kayıt olduğu gibi", "setup": S, "steps": [
        run({"form": "denetim", "uid": "$uidOf:1"}, answer(PUBLISHED, BODY, 1, FIRST)),
        run({"form": "denetim", "uid": "$uidOf:2"}, answer(PUBLISHED, BODY, 1, {}))]})
    cases.append({"name": "değerler yazılmış gibi: Durum iyi olunca açıklama gizlenir, sorulmaz, değeri kalır", "setup": S, "steps": [
        run({"form": "denetim", "uid": "$uidOf:1", "values": {"Durum": "İyi"}}, answer(PUBLISHED, BODY, 1, FIRST, {"Durum": "İyi"}))]})
    cases.append({"name": "gizli ve salt okunur alana yazmak sorundur", "setup": S, "steps": [
        run({"form": "denetim", "uid": "$uidOf:1", "values": {"Durum": "iyi", "Açıklama": "Yeni bir açıklama."}},
            answer(PUBLISHED, BODY, 1, FIRST, {"Durum": "iyi", "Açıklama": "Yeni bir açıklama."})),
        run({"form": "denetim", "uid": "$uidOf:1", "values": {"Durum": "iyi", "Onaylı": "true"}},
            answer(PUBLISHED, BODY, 1, FIRST, {"Durum": "iyi", "Onaylı": "true"}))]})
    cases.append({"name": "kurala uymayan değer kayda girmez; olmayan anahtar sorundur", "setup": S, "steps": [
        run({"form": "denetim", "uid": "$uidOf:1", "values": {"kat-sayisi": "41", "Yok": "1"}},
            answer(PUBLISHED, BODY, 1, FIRST, {"kat-sayisi": "41", "Yok": "1"}))]})
    cases.append({"name": "varsayılanlar boş alanlara önce yazılır (yeni kayıt)", "setup": S, "steps": [
        run({"form": "denetim", "uid": "$uidOf:2", "defaults": True, "values": {"Durum": "iyi", "kat-sayisi": "2"}},
            answer(PUBLISHED, BODY, 1, {}, {"Durum": "iyi", "kat-sayisi": "2"}, defaults=True))]})
    cases.append({"name": "denetim tutmuyor: kısa açıklama", "setup": S, "steps": [
        run({"form": "denetim", "uid": "$uidOf:1", "values": {"Açıklama": "kısa"}}, answer(PUBLISHED, BODY, 1, FIRST, {"Açıklama": "kısa"}))]})
    cases.append({"name": "yayınlanmamış form taslakla sorulur; taslaksız reddedilir", "setup": setup([FORM]), "steps": [
        run({"form": "denetim"}, unpublished("Bina denetimi", "draft")),
        run({"form": "denetim", "draft": True, "uid": "$uidOf:1"}, answer(FORM, BODY, None, FIRST))]})
    changed = {**copy.deepcopy(PUBLISHED), "draft": body_with(lambda b: item(b, "aciklama").pop("visibleWhen"))}
    cases.append({"name": "taslak ile son yayın ayrıdır: varsayılan son yayındır", "setup": setup([changed]), "steps": [
        run({"form": "denetim", "uid": "$uidOf:1", "values": {"Durum": "iyi"}}, answer(changed, BODY, 1, FIRST, {"Durum": "iyi"})),
        run({"form": "denetim", "draft": True, "uid": "$uidOf:1", "values": {"Durum": "iyi"}}, answer(changed, changed["draft"], None, FIRST, {"Durum": "iyi"}))]})
    # docs/adr/0441 §2, §4, §6: a divider and an action shown as items; what the form needs of its host.
    acted = body_with(lambda b: b["items"][0]["items"].extend([
        {"type": "divider", "id": "cizgi", "label": "Onay", "visibleWhen": "@kullanıcı = 'Ayşe'"},
        {"type": "action", "id": "hesapla", "label": "Öznitelik hesapla", "tool": "attributes.calculate"},
        {"type": "location", "id": "yer"}]))
    acting = {**copy.deepcopy(FORM), "draft": acted}
    cases.append({"name": "bölücü ve eylem öğe olarak; formun gerektirdikleri: kullanıcı, şekil, araç", "setup": setup([acting]),
                  "steps": [run({"form": "denetim", "draft": True, "uid": "$uidOf:1"}, answer(acting, acted, None, FIRST))]})
    # docs/adr/0442 §5: the actions' kinds: a step the form has not is an error; showing needs the geometry.
    kinds = body_with(lambda b: b["items"][0]["items"].extend([
        {"type": "action", "id": "goster", "label": "Çizimde göster", "kind": "show"},
        {"type": "action", "id": "yenile", "label": "Hesapları yenile", "kind": "recompute"},
        {"type": "action", "id": "gonder", "label": "Gönder", "kind": "step", "step": "Gönder"}]))
    kinded = {**copy.deepcopy(FORM), "draft": kinds}
    cases.append({"name": "eylem türleri: iş akışında olmayan adım hata, göster kaydın şeklini ister", "setup": setup([kinded]),
                  "steps": [run({"form": "denetim", "draft": True, "uid": "$uidOf:1"}, answer(kinded, kinds, None, FIRST))]})
    shown = body_with(lambda b: b["items"][0]["items"].append(
        {"type": "action", "id": "goster", "label": "Çizimde göster", "kind": "show"}))
    showing = {**copy.deepcopy(FORM), "draft": shown}
    cases.append({"name": "göster eylemi kaydı sorar ve şekli gerektirir", "setup": setup([showing]),
                  "steps": [run({"form": "denetim", "draft": True, "uid": "$uidOf:1"}, answer(showing, shown, None, FIRST))]})
    broken = {**copy.deepcopy(FORM), "draft": BROKEN}
    cases.append({"name": "ön denetimde hata olan form reddedilmez: kaydı sorulmaz", "setup": setup([broken]), "steps": [
        run({"form": "denetim", "draft": True, "uid": "$uidOf:1"}, answer(broken, BROKEN, None, FIRST))]})
    cases.append({"name": "kimlikler: geçersiz, olmayan nesne, başka katmanın nesnesi, olmayan form", "setup": S, "steps": [
        run({"form": "denetim", "uid": "A-1"}, failed("invalid_uid", INVALID_UID.format("A-1"), "uid")),
        run({"form": "denetim", "uid": UNKNOWN_UID}, failed("entity_not_found", NOT_FOUND.format(UNKNOWN_UID), "uid")),
        run({"form": "denetim", "uid": "$uidOf:3"}, mismatch("Bina denetimi")),
        run({"form": "yok"}, unknown_form("yok", "form"))]})
    cases.append({"name": "denetimlerin sırası: kimliğin biçimi formdan önce", "setup": S, "steps": [
        run({"form": "yok", "uid": "A-1"}, failed("invalid_uid", INVALID_UID.format("A-1"), "uid"))]})
    cases.append({"name": "kilitli katmanın kaydı da sorulur (yazmaz)", "setup": setup([LOCKED_FORM]), "steps": [
        run({"form": "kilitli", "uid": "$uidOf:4"}, answer(LOCKED_FORM, BODY, 1, {"Durum": "iyi"}))]})
    cases.append({"name": "koşullu zorunlu: Durum inceleme yapılınca açıklama ister", "setup": S, "steps": [
        run({"form": "denetim", "uid": "$uidOf:2", "values": {"Durum": "inceleme", "kat-sayisi": "1"}},
            answer(PUBLISHED, BODY, 1, {}, {"Durum": "inceleme", "kat-sayisi": "1"}))]})
    D = setup([DRAFT_FORM])
    cases.append({"name": "taslak durumdaki kayıtta formun zorunluluğu uyarıdır (ADR 0432)", "setup": D, "steps": [
        run({"form": "taslakli", "uid": "$uidOf:2", "defaults": True, "values": {"kat-sayisi": "1"}},
            answer(DRAFT_FORM, DRAFT_BODY, 1, {}, {"kat-sayisi": "1"}, defaults=True))]})
    cases.append({"name": "taslakta durum alanı adımlarla değişir: yazmak sorundur", "setup": D, "steps": [
        run({"form": "taslakli", "uid": "$uidOf:1", "values": {"Durum": "iyi"}}, answer(DRAFT_FORM, DRAFT_BODY, 1, FIRST, {"Durum": "iyi"}))]})
    cases.append({"name": "boş değer özniteliği kaldırır: zorunlu Durum boşalınca sorun", "setup": S, "steps": [
        run({"form": "denetim", "uid": "$uidOf:1", "values": {"Durum": ""}}, answer(PUBLISHED, BODY, 1, FIRST, {"Durum": ""}))]})
    for k, (name, values) in enumerate([
        ("evet/hayır: Evet yazılır, onay grupta", {"Durum": "inceleme", "Onaylı": "Evet"}),
        ("tarih GG.AA.YYYY'den", {"Tarih": "5.11.2026"}),
        ("listede olmayan durum", {"Durum": "yıkık"}),
        ("kat 0 aralık dışı, grubu da gizlemez", {"kat-sayisi": "0"}),
        ("boşluklu sayı tek biçime çevrilir", {"kat-sayisi": "  12 "}),
    ]):
        cases.append({"name": f"değerler: {name}", "setup": S, "steps": [
            run({"form": "denetim", "uid": "$uidOf:1", "values": values}, answer(PUBLISHED, BODY, 1, FIRST, values))]})
    C = calc_setup()
    calc = dict(fields=CALC_FIELDS)
    cases.append({"name": "hesaplı form, yeni kayıt, an ve kullanıcı verilir: varsayılanlar, hesaplar, süzülen seçenekler, notun metni",
                  "setup": C, "steps": [
        run({"form": "hesap", "uid": "$uidOf:2", "defaults": True, "values": {"Durum": "iyi", "kat-sayisi": "4", "Tür": "sanayi"},
             **MOMENT_GIVEN},
            answer(CALC_FORM, CALC_BODY, 1, {}, {"Durum": "iyi", "kat-sayisi": "4", "Tür": "sanayi"}, True, variables=GIVEN,
                   slot=2, others=[CALC_FIRST], **calc))]})
    cases.append({"name": "an ve kullanıcı verilmezse onları okuyan varsayılanlar boş kalır; başka nesnenin kodu tekil değildir",
                  "setup": C, "steps": [
        run({"form": "hesap", "uid": "$uidOf:2", "defaults": True, "values": {"Durum": "iyi", "kat-sayisi": "3"}},
            answer(CALC_FORM, CALC_BODY, 1, {}, {"Durum": "iyi", "kat-sayisi": "3"}, True, slot=2, others=[CALC_FIRST], **calc))]})
    cases.append({"name": "geçersiz an ve kullanıcı adı formdan önce söylenir", "setup": C, "steps": [
        run({"form": "hesap", "moment": "dün"},
            failed("invalid_moment", "“dün” bir an değil. Bir tarih (2026-10-16) ya da tarih ve yerel saat (2026-10-16T14:30:00) yazın.", "moment")),
        run({"form": "yok", "user": " Ayşe"},
            failed("invalid_user", "Kullanıcının adı boş olamaz, başında ve sonunda boşluk olmaz, en çok 64 karakterdir.", "user"))]})
    return cases


def submit_cases():
    cases = []

    def write(inp, before, values=(), defaults=False, form=PUBLISHED, body=BODY, version=1, undo=0, slot=1, fields=FIELDS,
              variables=()):
        f = Form(body, fields, variables)
        after, issues = f.assess(dict(before), utf8_sorted(dict(values)), OBJECT_OF.get(slot, {}), defaults)
        assert not errors(issues), issues
        warnings = [{"code": i["code"], "message": ANY, "path": f"values.{i['field']}"} for i in issues
                    if i["severity"] == "warning"]
        changed = after != before
        out = {"uid": f"$uidOf:{slot}", "attrs": after, "changed": changed, "revision": "$current"}
        if version is not None:
            out["version"] = version
        submitted(out, f"$uidOf:{slot}", before, after, form["layer"], f"Form: {form['name']}",
                  snapshot(after, snapshot_skip(body, fields)))
        expect = {"ids": IDS, "fields": {str(slot): {"attrs": after}}, "canUndo": undo + changed > 0, "canRedo": False}
        expect.update({"dirty": True, "revision": "changed"} if changed else {"revision": "same"})
        return {"op": "execute", "input": inp, "result": {"status": "completed", "output": out, "warnings": warnings},
                "expect": expect}, after

    def refused(inp, result, before, slot=1, undo=0, dirty=False):
        expect = {"ids": IDS, "fields": {str(slot): {"attrs": before}}, "canUndo": undo > 0, "canRedo": False, "revision": "same"}
        if not dirty:
            expect["dirty"] = False
        return {"op": "execute", "input": inp, "result": result, "expect": expect}

    def invalid(inp, before, values, slot=1, undo=0, dirty=False, body=BODY, fields=FIELDS, variables=(), others=()):
        f = Form(body, fields, variables)
        after, issues = f.assess(dict(before), utf8_sorted(dict(values)), OBJECT_OF.get(slot, {}), inp.get("defaults", False))
        issues = errors(issues) or f.unique_issues(after, OBJECT_OF.get(slot, {}), others)
        first = issues[0]
        path = f"values.{first['field']}" if first.get("field") in values else "values"
        return refused(inp, failed("form_invalid", ANY, path), before, slot, undo, dirty)

    S = setup([PUBLISHED])
    v = {"Durum": "iyi", "kat-sayisi": "4"}
    one, after = write({"form": "denetim", "uid": "$uidOf:1", "values": v}, FIRST, v)
    two, _ = write({"form": "denetim", "uid": "$uidOf:1", "values": v}, after, v, undo=1)
    cases.append({"name": "Formla kaydet: değerler tek biçimle yazılır, alan dışı öznitelik kalır, gizlenen açıklamanın değeri kalır; tek geri alma adımı; aynısını yazmak değiştirmez",
                  "setup": S, "steps": [one, two]})

    v = {"Durum": "Ek inceleme gerekli", "kat-sayisi": "2", "Açıklama": "Bodrumda nem var.", "Onaylı": "hayır"}
    one, _ = write({"form": "denetim", "uid": "$uidOf:2", "values": v}, {}, v, slot=2)
    cases.append({"name": "yeni kayıt: bütün alanlar, etiketten kod, evet/hayırdan false", "setup": S, "steps": [one]})

    v = {"Durum": "iyi", "kat-sayisi": "1"}
    one, _ = write({"form": "denetim", "uid": "$uidOf:2", "values": v, "defaults": True}, {}, v, defaults=True, slot=2)
    cases.append({"name": "varsayılanlar boş alanlara yazılır (yeni kayıt)", "setup": S, "steps": [one]})
    # docs/adr/0432: a new record in the workflow's draft state is kept incomplete, the form's requirement a warning.
    v = {"kat-sayisi": "1"}
    one, _ = write({"form": "taslakli", "uid": "$uidOf:2", "values": v, "defaults": True}, {}, v, defaults=True, slot=2,
                   form=DRAFT_FORM, body=DRAFT_BODY)
    cases.append({"name": "taslak durumdaki yeni kayıt eksik kaydedilir, zorunluluk uyarıdır", "setup": setup([DRAFT_FORM]),
                  "steps": [one]})

    v = {"Açıklama": ""}
    cases.append({"name": "koşullu zorunlu boşaltılamaz: form_invalid, path değerin", "setup": S, "steps": [
        invalid({"form": "denetim", "uid": "$uidOf:1", "values": v}, FIRST, v)]})
    v = {"Durum": "inceleme", "kat-sayisi": "1"}
    cases.append({"name": "koşullu zorunlu verilmemişse path values", "setup": S, "steps": [
        invalid({"form": "denetim", "uid": "$uidOf:2", "values": v}, {}, v, slot=2)]})
    v = {"Durum": "iyi", "Açıklama": "Başka bir açıklama."}
    cases.append({"name": "gizli alana yazılmaz", "setup": S, "steps": [invalid({"form": "denetim", "uid": "$uidOf:1", "values": v}, FIRST, v)]})
    v = {"Durum": "iyi", "Onaylı": "true"}
    cases.append({"name": "salt okunur alana yazılmaz", "setup": S, "steps": [invalid({"form": "denetim", "uid": "$uidOf:1", "values": v}, FIRST, v)]})
    v = {"kat-sayisi": "kırk bir"}
    cases.append({"name": "alanın kuralına uymayan değer", "setup": S, "steps": [invalid({"form": "denetim", "uid": "$uidOf:1", "values": v}, FIRST, v)]})
    v = {"Yok": "1"}
    cases.append({"name": "olmayan anahtar", "setup": S, "steps": [invalid({"form": "denetim", "uid": "$uidOf:1", "values": v}, FIRST, v)]})
    v = {"Açıklama": "kısa"}
    cases.append({"name": "denetim tutmuyor", "setup": S, "steps": [invalid({"form": "denetim", "uid": "$uidOf:1", "values": v}, FIRST, v)]})

    v = {"Durum": "iyi"}
    one, _ = write({"form": "denetim", "uid": "$uidOf:1", "values": v, "draft": True}, FIRST, v, form=FORM, version=None)
    cases.append({"name": "yayınlanmamış form taslakla yazar; taslaksız reddedilir", "setup": setup([FORM]), "steps": [
        refused({"form": "denetim", "uid": "$uidOf:1", "values": v}, unpublished("Bina denetimi", "draft"), FIRST), one]})

    broken = {**copy.deepcopy(FORM), "draft": BROKEN}
    cases.append({"name": "ön denetimde hata olan formla yazılmaz", "setup": setup([broken]), "steps": [
        refused({"form": "denetim", "uid": "$uidOf:1", "values": {"Durum": "iyi"}, "draft": True}, failed("form_preflight", ANY, "form"), FIRST)]})

    cases.append({"name": "kilitli katmanın kaydı yazılmaz", "setup": setup([LOCKED_FORM]), "steps": [
        refused({"form": "kilitli", "uid": "$uidOf:4", "values": {"kat-sayisi": "2"}},
                failed("layer_locked", "“Kilitli bina” katmanı kilitli; kayıt değiştirilemez. Kilidi Katmanlar panelinden açın.", "uid"),
                {"Durum": "iyi"}, slot=4)]})

    cases.append({"name": "kimlikler: geçersiz, olmayan nesne, başka katmanın nesnesi, olmayan form", "setup": S, "steps": [
        refused({"form": "denetim", "uid": "A-1"}, failed("invalid_uid", INVALID_UID.format("A-1"), "uid"), FIRST),
        refused({"form": "denetim", "uid": UNKNOWN_UID}, failed("entity_not_found", NOT_FOUND.format(UNKNOWN_UID), "uid"), FIRST),
        refused({"form": "denetim", "uid": "$uidOf:3"}, mismatch("Bina denetimi"), FIRST),
        refused({"form": "yok", "uid": "$uidOf:1"}, unknown_form("yok", "form"), FIRST)]})

    v = {"kat-sayisi": "5"}
    one, _ = write({"form": "denetim", "uid": "$uidOf:1", "values": v, "expectedRevision": "$current"}, FIRST, v)
    cases.append({"name": "beklenen sürüm: geçersiz yazı ve eskisi reddedilir; doğrusu yazar", "setup": S, "steps": [
        refused({"form": "denetim", "uid": "$uidOf:1", "values": v, "expectedRevision": "07"}, failed("invalid_revision", REVISION, "expectedRevision"), FIRST),
        refused({"form": "denetim", "uid": "$uidOf:1", "values": v, "expectedRevision": "999"}, failed("revision_conflict", CONFLICT, "expectedRevision", "conflict"), FIRST),
        one]})

    one, after = write({"form": "denetim", "uid": "$uidOf:1", "values": {}}, FIRST)
    cases.append({"name": "değersiz kayıt: geçerliyse bir şey değiştirmez", "setup": S, "steps": [one]})

    v = {"Tarih": "31.12.2026", "Açıklama": "Çatı arası ve merdiven boşluğu."}
    one, after = write({"form": "denetim", "uid": "$uidOf:1", "values": v}, FIRST, v)
    v2 = {"Tarih": ""}
    two, _ = write({"form": "denetim", "uid": "$uidOf:1", "values": v2}, after, v2, undo=1)
    cases.append({"name": "art arda iki kayıt iki geri alma adımı; boş değer tarihi kaldırır", "setup": S, "steps": [one, two]})

    enforced_v = {"Açıklama": "Uzun bir açıklama yazıldı."}
    one, _ = write({"form": "denetim", "uid": "$uidOf:1", "values": enforced_v}, FIRST, enforced_v)
    cases.append({"name": "kuralları uygulanan form da formla yazar", "setup": setup([ENFORCED]), "steps": [one]})

    v = {"kat-sayisi": " 007 ", "Onaylı": "Evet"}
    one, _ = write({"form": "denetim", "uid": "$uidOf:1", "values": v}, FIRST, v)
    cases.append({"name": "değerler kırpılır ve tek biçime çevrilir: 007 7, Evet true", "setup": S, "steps": [one]})

    v = {"Durum": "inceleme", "kat-sayisi": "3", "Açıklama": "Kısa", "Onaylı": "true"}
    cases.append({"name": "birden çok sorunda ilki söylenir", "setup": S, "steps": [
        invalid({"form": "denetim", "uid": "$uidOf:2", "values": v}, {}, v, slot=2)]})

    C = calc_setup()
    calc = dict(form=CALC_FORM, body=CALC_BODY, fields=CALC_FIELDS)
    v = {"Durum": "inceleme", "kat-sayisi": "6", "Açıklama": "Kısa ama yeter."}
    one, _ = write({"form": "hesap", "uid": "$uidOf:2", "values": v, "defaults": True, **MOMENT_GIVEN}, {}, v, defaults=True,
                   slot=2, variables=GIVEN, **calc)
    cases.append({"name": "hesaplı form: yeni kayıt varsayılanları ve hesaplarıyla yazılır; uyarı denetimi yazmayı durdurmaz",
                  "setup": C, "steps": [one]})
    v = {"Yükseklik": "99"}
    cases.append({"name": "hesaplanan alana başka değer yazılamaz", "setup": C, "steps": [
        invalid({"form": "hesap", "uid": "$uidOf:1", "values": v}, CALC_FIRST, v, **{k: calc[k] for k in ("body", "fields")})]})
    cases.append({"name": "geçersiz an ve kullanıcı adı: kimliğin biçiminden sonra, formdan önce", "setup": C, "steps": [
        refused({"form": "yok", "uid": "$uidOf:1", "moment": "2026-13-01"},
                failed("invalid_moment", "“2026-13-01” bir an değil. Bir tarih (2026-10-16) ya da tarih ve yerel saat (2026-10-16T14:30:00) yazın.", "moment"),
                CALC_FIRST),
        refused({"form": "hesap", "uid": "$uidOf:1", "user": ""},
                failed("invalid_user", "Kullanıcının adı boş olamaz, başında ve sonunda boşluk olmaz, en çok 64 karakterdir.", "user"),
                CALC_FIRST)]})
    v = {"Durum": "iyi", "kat-sayisi": "5", "Kod": "B-3"}
    cases.append({"name": "tekil alanın değeri katmanın başka nesnesinde var", "setup": C, "steps": [
        invalid({"form": "hesap", "uid": "$uidOf:2", "values": v}, {}, v, slot=2, others=[CALC_FIRST],
                **{k: calc[k] for k in ("body", "fields")})]})
    return cases


def set_cases():
    """cad.entities.set with a layer's enforcing form (docs/adr/0401 §8)."""
    cases = []

    def set_attrs(slot, attrs, before, ok, undo=0, dirty=False, refusal=None):
        inp = {"uids": [f"$uidOf:{slot}"], "attrs": attrs, "operation": "attributes"}
        after = dict(before)
        for k, v in attrs.items():
            if v is None:
                after.pop(k, None)
            else:
                after[k] = v
        if ok:
            changed = after != before
            result = {"status": "completed", "output": {"changed": [f"$uidOf:{slot}"] if changed else [], "ids": [slot] if changed else [],
                                                         "revision": "$current"}, "warnings": []}
            expect = {"ids": IDS, "fields": {str(slot): {"attrs": after}}, "canUndo": undo + changed > 0, "canRedo": False}
            expect.update({"dirty": True, "revision": "changed"} if changed else {"revision": "same"})
            return {"op": "execute", "input": inp, "result": result, "expect": expect}, after
        expect = {"ids": IDS, "fields": {str(slot): {"attrs": before}}, "canUndo": undo > 0, "canRedo": False, "revision": "same"}
        if not dirty:
            expect["dirty"] = False
        return {"op": "execute", "input": inp, "result": refusal or failed("form_rules", ANY, "attrs"), "expect": expect}, before

    form = Form(BODY, FIELDS)

    def judged(before, attrs):
        """Whether the write keeps the form: a write that changes no layer field is not the form's."""
        after = dict(before)
        for k, v in attrs.items():
            if v is None:
                after.pop(k, None)
            else:
                after[k] = v
        if all(before.get(f["name"]) == after.get(f["name"]) for f in FIELDS):
            return True
        return not form.enforce(before, after)

    E = setup([ENFORCED])
    for name, slot, before, attrs in [
        ("katmanın uyguladığı form koşullu zorunlu açıklamayı boşalttırmaz", 1, FIRST, {"Açıklama": None}),
        ("geçerli değişiklik yazılır", 1, FIRST, {"Açıklama": "Çatı arası ve merdiven boşluğu."}),
        ("alan olmayan öznitelik serbest", 1, FIRST, {"Kod": "B-13"}),
        ("zorunlu Durum'u boş bırakan yeni kayıt reddedilir", 2, {}, {"Kat": "2"}),
        ("gizli alan değiştirilemez", 4, {"Durum": "iyi"}, {"Açıklama": "x"}),
    ]:
        if slot == 4:
            continue
        ok = judged(before, attrs)
        step, _ = set_attrs(slot, attrs, before, ok)
        cases.append({"name": name, "setup": E, "steps": [step]})
    for name, slot, before, attrs in [
        ("Durum iyiyken onay salt okunur: değiştirilemez", 1, {**FIRST, "Durum": "iyi", "Onaylı": "true"}, {"Onaylı": "false"}),
        ("Durum incelemeyken onay yazılır", 1, FIRST, {"Onaylı": "true"}),
        ("yeni kayda Durum ve kat yazılır: açıklama gizli", 2, {}, {"Durum": "iyi", "Kat": "2"}),
        ("kat kaldırılır: grup gizlenir, kayıt geçerli kalır", 1, FIRST, {"Kat": None}),
        ("kısa açıklama denetimi tutmaz", 1, FIRST, {"Açıklama": "kısa"}),
        ("açıklama ve alan dışı öznitelik birlikte", 1, FIRST, {"Açıklama": "Merdiven boşluğu da bakılmalı.", "Kod": "B-14"}),
        ("tarih grupta gösterilirken yazılır", 1, FIRST, {"Tarih": "2026-11-05"}),
        ("Durum iyi yapılır, açıklama yazılmadan kalır", 1, FIRST, {"Durum": "iyi"}),
        ("onay kaldırılır", 1, {**FIRST, "Onaylı": "true"}, {"Onaylı": None}),
        ("kat değişir, açıklama kalır", 1, FIRST, {"Kat": "12"}),
    ]:
        ok = judged(before, attrs)
        e = copy.deepcopy(E)
        e["entities"] = [{**x, "attrs": before} if x["id"] == slot else x for x in e["entities"]]
        step, _ = set_attrs(slot, attrs, before, ok)
        cases.append({"name": name, "setup": e, "steps": [step]})
    step, _ = set_attrs(1, {"Durum": None}, FIRST, False,
                        refusal=failed("attribute_required", "“Durum” alanı zorunlu; boş bırakılamaz.", "attrs.Durum"))
    cases.append({"name": "Durum boşaltılamaz: katmanın kendi kuralı formdan önce söyler", "setup": E, "steps": [step]})
    hidden_before = {"Durum": "iyi", "Kat": "2"}
    ok = judged(hidden_before, {"Açıklama": "Yeni açıklama metni."})
    assert not ok
    s1, mid = set_attrs(1, {"Durum": "iyi", "Açıklama": None}, FIRST, judged(FIRST, {"Durum": "iyi", "Açıklama": None}))
    cases.append({"name": "Durum iyi yapılırken gizlenen açıklamayı silmek de gizli alana yazmaktır", "setup": E, "steps": [s1]})
    s1, mid = set_attrs(1, {"Durum": "iyi"}, FIRST, judged(FIRST, {"Durum": "iyi"}))
    s2, _ = set_attrs(1, {"Açıklama": "Yeni açıklama metni."}, mid, judged(mid, {"Açıklama": "Yeni açıklama metni."}), undo=1, dirty=True)
    cases.append({"name": "Durum iyi olunca açıklama gizlenir; ona yazmak reddedilir", "setup": E, "steps": [s1, s2]})
    step, _ = set_attrs(1, {"Açıklama": None}, FIRST, True)
    cases.append({"name": "kuralları uygulanmayan form yazmayı engellemez", "setup": setup([PUBLISHED]), "steps": [step]})
    stale = {**copy.deepcopy(ENFORCED), "releases": [release_of(BROKEN, 1)]}
    step, _ = set_attrs(1, {"Kod": "B-13"}, FIRST, True)
    cases.append({"name": "alan değişmeyen yazma her zaman serbest", "setup": setup([stale]), "steps": [step]})
    step, _ = set_attrs(1, {"Kat": "4"}, FIRST, False)
    cases.append({"name": "katmanın alanlarına artık uymayan yayın yazmayı engeller: form yeniden yayınlanmalı", "setup": setup([stale]), "steps": [step]})

    # docs/adr/0402 §4: the document completes what a write leaves on the layer.
    calc = Form(CALC_BODY, CALC_FIELDS)

    def calc_set(attrs, before=CALC_FIRST):
        after = dict(before)
        for k, v in attrs.items():
            if v is None:
                after.pop(k, None)
            else:
                after[k] = v
        issues = calc.enforce(before, after, SQUARE)
        inp = {"uids": ["$uidOf:1"], "attrs": attrs, "operation": "attributes"}
        if errors(issues):
            expect = {"ids": IDS, "fields": {"1": {"attrs": before}}, "canUndo": False, "canRedo": False, "revision": "same",
                      "dirty": False}
            return {"op": "execute", "input": inp, "result": failed("form_rules", ANY, "attrs"), "expect": expect}
        done, _ = calc.complete(after, SQUARE, False)
        changed = done != before
        result = {"status": "completed", "output": {"changed": ["$uidOf:1"] if changed else [], "ids": [1] if changed else [],
                                                     "revision": "$current"}, "warnings": []}
        expect = {"ids": IDS, "fields": {"1": {"attrs": done}}, "canUndo": changed, "canRedo": False}
        expect.update({"dirty": True, "revision": "changed"} if changed else {"revision": "same"})
        return {"op": "execute", "input": inp, "result": result, "expect": expect}

    C = calc_setup(CALC_ENFORCED)
    cases.append({"name": "hesaplı form: kat değişince belge hesapları aynı adımda yeniler", "setup": C,
                  "steps": [calc_set({"Kat": "5"})]})
    cases.append({"name": "hesaplı form: hesaplanan alan başka yoldan yazılamaz", "setup": C,
                  "steps": [calc_set({"Yükseklik": "99"})]})
    cases.append({"name": "hesaplı form: süzgecin sunmadığı malzeme yazılamaz", "setup": C,
                  "steps": [calc_set({"Tür": "sanayi", "Malzeme": "ahsap"})]})
    return cases


FILL_ENTITIES = CALC_ENTITIES + [
    {"kind": "polygon", "id": 5, "layerId": "bina", "attrs": {"Durum": "iyi", "Kat": "2", "Kod": "B-2"}, "pts": square(60, 0)},
    {"kind": "polygon", "id": 6, "layerId": "bina",
     "attrs": {"Durum": "inceleme", "Kat": "5", "Açıklama": "Çatı akıyor ve duvarlar çatlak."}, "pts": square(80, 0)},
]
FILL_IDS = [e["id"] for e in FILL_ENTITIES]
NO_RECORDS = "Formla yazılacak kayıt verilmedi. En az bir nesnenin kalıcı kimliğini verin."
UIDS_INVALID = ("“{}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir "
                "(01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.")


def fill_cases():
    """cad.forms.fill (docs/adr/0403 §5): the same values to many records, each checked and completed by itself, every
    record or none; a unique field takes no value twice."""
    cases = []
    attrs_of = {e["id"]: e["attrs"] for e in FILL_ENTITIES}
    layer_of = {e["id"]: e["layerId"] for e in FILL_ENTITIES}

    def slot_of(u):
        return int(u.split(":")[1])

    def fill(inp, form=CALC_FORM, body=CALC_BODY, fields=CALC_FIELDS, variables=(), op="execute", undo=0, before=None):
        """The step and the records afterwards, by the rule; `before` the records' attributes now."""
        now = dict(before or attrs_of)
        f = Form(body, fields, variables)
        values = utf8_sorted(dict(inp.get("values", {})))
        seen, slots = set(), []
        for u in inp["uids"]:
            if u not in seen:
                seen.add(u)
                slots.append(slot_of(u))
        new, warnings = {}, []
        for k in slots:
            after, issues = f.assess(dict(now[k]), values, OBJECT_OF.get(k, SQUARE), False)
            errs = errors(issues)
            if errs:
                return failed_step(inp, errs, k, now, undo)
            for i in issues:
                if i["severity"] == "warning":
                    w = {"code": i["code"], "message": ANY, "path": f"values.{i['field']}"}
                    if not any(x["code"] == w["code"] and x["path"] == w["path"] for x in warnings):
                        warnings.append(w)
            new[k] = after
        for k in slots:
            others = [now[j] for j in FILL_IDS if layer_of[j] == "bina" and j not in slots] + \
                     [new[j] for j in slots if j != k]
            taken = f.unique_issues(new[k], OBJECT_OF.get(k, SQUARE), others)
            if taken:
                return failed_step(inp, taken, k, now, undo)
        changed = [k for k in slots if new[k] != now[k]]
        out = {"changed": [f"$uidOf:{k}" for k in changed], "version": 1 if not inp.get("draft") else None,
               "revision": "$current"}
        if out["version"] is None:
            del out["version"]
        after_all = {**now, **{k: new[k] for k in changed}}
        if op == "plan":
            expect = {"ids": FILL_IDS, "fields": {str(k): {"attrs": now[k]} for k in slots}, "canUndo": undo > 0,
                      "canRedo": False, "revision": "same"}
            return {"op": "plan", "input": inp, "result": {"status": "completed", "output": out, "warnings": warnings},
                    "expect": expect}, now
        expect = {"ids": FILL_IDS, "fields": {str(k): {"attrs": after_all[k]} for k in slots},
                  "canUndo": undo + bool(changed) > 0, "canRedo": False}
        expect.update({"dirty": True, "revision": "changed"} if changed else {"revision": "same"})
        return {"op": "execute", "input": inp, "result": {"status": "completed", "output": out, "warnings": warnings},
                "expect": expect}, after_all

    def failed_step(inp, issues, k, now, undo):
        first = errors(issues)[0]
        path = f"values.{first['field']}" if first.get("field") in inp.get("values", {}) else "values"
        result = failed("form_invalid", ANY, path)
        result["error"]["uids"] = [f"$uidOf:{k}"]
        return refused(inp, result, undo), now

    def refused(inp, result, undo=0):
        expect = {"ids": FILL_IDS, "canUndo": undo > 0, "canRedo": False, "revision": "same"}
        if not undo:
            expect["dirty"] = False
        return {"op": "execute", "input": inp, "result": result, "expect": expect}

    def case(name, steps, forms=None):
        cases.append({"name": name, "setup": setup(forms or [CALC_FORM], CALC_FIELDS, FILL_ENTITIES), "steps": steps})

    v = {"Durum": "iyi", "kat-sayisi": "4"}
    one, _ = fill({"form": "hesap", "uids": ["$uidOf:2", "$uidOf:5"], "values": v})
    case("iki kayda aynı değerler: her biri kendi hesaplarıyla, tek adımda", [one])
    one, after = fill({"form": "hesap", "uids": ["$uidOf:2", "$uidOf:5", "$uidOf:6"], "values": {"Durum": "iyi"}})
    two, _ = fill({"form": "hesap", "uids": ["$uidOf:2", "$uidOf:5", "$uidOf:6"], "values": {"Durum": "iyi"}}, before=after, undo=1)
    case("aynısını yeniden yazmak değiştirmez: changed boş, sürüm aynı", [one, two])
    one, _ = fill({"form": "hesap", "uids": ["$uidOf:5", "$uidOf:2", "$uidOf:5"], "values": {"kat-sayisi": "6", "Durum": "iyi"}})
    case("yinelenen kimlik bir kez yazılır; changed girdinin sırasıyla", [one])
    one, _ = fill({"form": "hesap", "uids": ["$uidOf:2", "$uidOf:5"], "values": {"Durum": "iyi", "kat-sayisi": "3"}}, op="plan")
    case("plan yazmaz, değişecekleri söyler", [one])
    one, _ = fill({"form": "hesap", "uids": ["$uidOf:2", "$uidOf:5"], "values": {"Durum": "iyi", "Kod": "X-1"}})
    case("tekil alana iki kayıtta aynı değer: girdideki ilk kayıt söylenir", [one])
    one, _ = fill({"form": "hesap", "uids": ["$uidOf:2"], "values": {"Durum": "iyi", "Kod": "B-3"}})
    case("tekil alanın değeri katmanın başka nesnesinde", [one])
    one, _ = fill({"form": "hesap", "uids": ["$uidOf:1", "$uidOf:5"], "values": {"Kod": "B-3"}})
    case("tekil alan: bir kaydın kendi değeri de iki kayda yazılamaz", [one])
    one, _ = fill({"form": "hesap", "uids": ["$uidOf:5", "$uidOf:2"], "values": {"kat-sayisi": "3"}})
    case("bir kaydın zorunlu alanı boş kalırsa hiçbiri yazılmaz; path values", [one])
    one, _ = fill({"form": "hesap", "uids": ["$uidOf:2", "$uidOf:5"], "values": {"Durum": "iyi", "kat-sayisi": "kırk"}})
    case("alanın kuralına uymayan değer", [one])
    one, _ = fill({"form": "hesap", "uids": ["$uidOf:1", "$uidOf:5"], "values": {"Yükseklik": "99"}})
    case("hesaplanan alana başka değer yazılamaz", [one])
    one, _ = fill({"form": "hesap", "uids": ["$uidOf:1", "$uidOf:5"], "values": {"Tür": "sanayi", "Malzeme": "ahsap"}})
    case("süzgecin sunmadığı seçenek", [one])
    one, _ = fill({"form": "hesap", "uids": ["$uidOf:1", "$uidOf:6"], "values": {"Durum": "inceleme", "Açıklama": "Kısa ama yeter"}})
    case("uyarı denetimi yazmayı durdurmaz; iki kayıtta bir uyarı", [one])
    v = {"Durum": "iyi", "kat-sayisi": "2"}
    one, _ = fill({"form": "hesap", "uids": ["$uidOf:2"], "values": v, **MOMENT_GIVEN}, variables=GIVEN)
    case("an ve kullanıcı verilir; var olan kayıt varsayılan almaz", [one])
    draft_form = {"id": "taslak", "name": "Taslak", "layer": "bina", "draft": copy.deepcopy(CALC_BODY)}
    one, _ = fill({"form": "taslak", "draft": True, "uids": ["$uidOf:2"], "values": {"Durum": "iyi"}}, form=draft_form)
    case("yayınlanmamış form taslakla yazar; taslaksız reddedilir", [
        refused({"form": "taslak", "uids": ["$uidOf:2"], "values": {"Durum": "iyi"}}, unpublished("Taslak", "draft")), one],
         forms=[draft_form])
    case("kayıt verilmedi", [refused({"form": "hesap", "uids": []}, failed("no_entities", NO_RECORDS, "uids"))])
    case("kimliklerden biri geçersiz: yeri uids[1]", [
        refused({"form": "hesap", "uids": ["$uidOf:2", "A-1"]}, failed("invalid_uid", UIDS_INVALID.format("A-1"), "uids[1]"))])
    case("çizimde olmayan kayıt", [
        refused({"form": "hesap", "uids": ["$uidOf:2", UNKNOWN_UID]}, failed("entity_not_found", NOT_FOUND.format(UNKNOWN_UID), "uids[1]"))])
    case("formun katmanında olmayan kayıt", [
        refused({"form": "hesap", "uids": ["$uidOf:2", "$uidOf:3"]},
                failed("form_layer_mismatch", "Nesne “Bina hesabı” formunun katmanında değil. Formun katmanındaki nesneleri seçin.", "uids[1]"))])
    locked_form = {"id": "kilit", "name": "Kilitli bina", "layer": "kilitli", "draft": copy.deepcopy(BODY),
                   "releases": [release_of(BODY, 1)]}
    case("kilitli katman", [refused({"form": "kilit", "uids": ["$uidOf:4"], "values": {"Durum": "iyi"}},
                                    failed("layer_locked", "“Kilitli bina” katmanı kilitli; kayıtlar değiştirilemez. Kilidi Katmanlar panelinden açın.", "uids"))],
         forms=[CALC_FORM, locked_form])
    case("olmayan form; geçersiz an ve kullanıcı formdan önce", [
        refused({"form": "yok", "uids": ["$uidOf:2"]}, unknown_form("yok", "form")),
        refused({"form": "yok", "uids": ["$uidOf:2"], "moment": "dün"},
                failed("invalid_moment", "“dün” bir an değil. Bir tarih (2026-10-16) ya da tarih ve yerel saat (2026-10-16T14:30:00) yazın.", "moment")),
        refused({"form": "yok", "uids": ["$uidOf:2"], "user": " Ayşe"},
                failed("invalid_user", "Kullanıcının adı boş olamaz, başında ve sonunda boşluk olmaz, en çok 64 karakterdir.", "user"))])
    case("beklenen sürüm: geçersiz ve çakışan", [
        refused({"form": "hesap", "uids": ["$uidOf:2"], "values": {"Durum": "iyi"}, "expectedRevision": "07"},
                failed("invalid_revision", REVISION, "expectedRevision")),
        refused({"form": "hesap", "uids": ["$uidOf:2"], "values": {"Durum": "iyi"}, "expectedRevision": "999"},
                failed("revision_conflict", CONFLICT, "expectedRevision", "conflict"))])
    broken = {**copy.deepcopy(CALC_FORM), "draft": BROKEN, "releases": []}
    case("ön denetimde hata olan formla yazılmaz", [
        refused({"form": "hesap", "draft": True, "uids": ["$uidOf:2"], "values": {"Durum": "iyi"}}, failed("form_preflight", ANY, "form"))],
         forms=[broken])
    return cases


def create_cases():
    """cad.polygon.create on a layer whose form enforces its rules (docs/adr/0402 §4): the document gives the new
    object the form's defaults and calculations in the same step; its missing required values are not refused."""
    cases = []

    def rect(x, y, w, h):
        return [P(E0 + x, N0 + y), P(E0 + x + w, N0 + y), P(E0 + x + w, N0 + y + h), P(E0 + x, N0 + y + h)]

    def made(name, layer_id, w, h, attrs=None, form=CALC_ENFORCED, body=CALC_BODY):
        inp = {"layerId": layer_id, "pts": rect(100, 0, w, h)}
        if attrs:
            inp["attrs"] = attrs
        given = dict(attrs or {})
        enforcing = form is not None and form.get("enforce") and layer_id == "bina"
        if enforcing:
            obj = {"length": 2 * (w + h), "area": w * h, "vertices": 4}
            out, _ = Form(body, CALC_FIELDS).complete(given, obj, True)
        else:
            out = given
        entity = {"kind": "polygon", "id": 5, "layerId": layer_id, "attrs": out, "pts": rect(100, 0, w, h)}
        step = {"op": "execute", "input": inp,
                "result": {"status": "completed", "output": {"uid": "$uid", "id": 5, "revision": "$current"}, "warnings": []},
                "expect": {"ids": IDS + [5], "entities": {"5": entity}, "canUndo": True, "canRedo": False, "dirty": True,
                           "revision": "changed"}}
        forms = [form] if form else []
        cases.append({"name": name, "setup": setup(forms, CALC_FIELDS, CALC_ENTITIES), "steps": [step]})

    made("kuralları uygulanan formun katmanına çizilen alan varsayılanlarını ve hesaplarını alır; zorunlu Durum eksik kalır",
         "bina", 10, 10)
    made("alanın ölçüsü hesaba girer: 20 × 12,5", "bina", 20, 12.5)
    made("verilen öznitelikler varsayılanlara ve hesaplara girer", "bina", 10, 10, {"Durum": "iyi", "Kat": "4"})
    made("yüksek kat riskli sayılır", "bina", 10, 10, {"Durum": "iyi", "Kat": "8"})
    made("alana uymayan hesap yazılmaz, öbürleri yazılır", "bina", 10, 10, {"Durum": "iyi", "Kat": "15"})
    made("verilen yanlış yükseklik hesapla düzelir", "bina", 10, 10, {"Durum": "iyi", "Kat": "2", "Yükseklik": "50"})
    made("verilen kod korunur: varsayılan dolu alana yazılmaz", "bina", 10, 10, {"Kat": "2", "Kod": "Özel-1"})
    made("an ve kullanıcıyı okuyan varsayılanlar çizimde boş kalır", "bina", 10, 10, {"Kat": "1"})
    made("ondalıklı alan iki basamağa sığar", "bina", 3, 2.5)
    made("küçük alan", "bina", 1, 1)
    made("büyük alan", "bina", 100, 40, {"Kat": "6"})
    made("başka katmana çizilen alan formdan etkilenmez", "yol", 10, 10, {"Kat": "4"})
    made("kuralları uygulanmayan form çizimi tamamlamaz", "bina", 10, 10, {"Kat": "4"}, form=CALC_FORM)
    made("formsuz çizim olduğu gibi", "bina", 10, 10, {"Kat": "4"}, form=None)
    hidden = calc_with(lambda b: b.update(clearHidden=True))
    hidden_form = {**copy.deepcopy(CALC_ENFORCED), "draft": hidden, "releases": [release_of(hidden, 1)]}
    made("gizlenenleri silen form: Durum iyiyken açıklama silinir", "bina", 10, 10,
         {"Durum": "iyi", "Kat": "2", "Açıklama": "Not, uzunca bir not."}, form=hidden_form, body=hidden)
    made("gizlenenleri silen form: incelemedeki açıklama kalır", "bina", 10, 10,
         {"Durum": "inceleme", "Kat": "2", "Açıklama": "Not, uzunca bir not."}, form=hidden_form, body=hidden)
    hidden_group = calc_with(lambda b: (b.update(clearHidden=True), item(b, "hesap").update(visibleWhen="Durum = 'inceleme'")))
    group_form = {**copy.deepcopy(CALC_ENFORCED), "draft": hidden_group, "releases": [release_of(hidden_group, 1)]}
    made("gizlenen grubun hesapları yapılmaz", "bina", 10, 10, {"Durum": "iyi", "Kat": "3"}, form=group_form, body=hidden_group)
    made("görünen grubun hesapları yapılır", "bina", 10, 10, {"Durum": "inceleme", "Kat": "3"}, form=group_form, body=hidden_group)
    made("tür ve malzeme olduğu gibi yazılır: süzgeç çizimi reddetmez", "bina", 10, 10, {"Tür": "sanayi", "Malzeme": "ahsap"})
    made("kat sınırda", "bina", 10, 10, {"Kat": "40"})
    return cases


def command_file(command, title, note, cases):
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": command,
        "commandVersion": 1,
        "title": title,
        "note": note + " Kural ve beklenenler scripts/fixtures/forms_cases.py'de KentOS kodu olmadan yazılmıştır; "
                       "sorunların ve formdan gelen retlerin cümleleri çekirdeğindir ($any).",
        "setup": setup(),
        "cases": cases,
    }


# Lists from other layers through the commands (docs/adr/0404): the other layers' rows are the setup's objects, their
# persistent ids `$uidOf:N` as the runner gives them.
LOOK_LAYER_IDS = {"il": [3, 4, 5], "ilce": [6, 7, 8], "mahalle": [9, 10, 11, 12], "parsel": [13, 14], "sayac": [15, 16]}
LOOK_ROW_ATTRS = {
    3: {"kod": "06", "ad": "Ankara"}, 4: {"kod": "34", "ad": "İstanbul"}, 5: {"kod": "35", "ad": "İzmir"},
    6: {"kod": "0601", "ad": "Çankaya", "il": "06"}, 7: {"kod": "0602", "ad": "Altındağ", "il": "06"},
    8: {"kod": "3401", "ad": "Kadıköy", "il": "34"},
    9: {"kod": "M10", "ad": "Kızılay", "ilce": "0601", "durum": "etkin"},
    10: {"kod": "M2", "ad": "Bahçelievler", "ilce": "0601", "durum": "etkin"},
    11: {"kod": "M3", "ad": "Eski mahalle", "ilce": "0601", "durum": "kapalı"},
    12: {"kod": "M4", "ad": "Ulus", "ilce": "0602", "durum": "etkin"},
    13: {"ada": "101/5"}, 14: {"ada": "101/12"}, 15: {"no": "S-1"}, 16: {"no": "S-2"},
}
LOOK_FORM = {"id": "adres", "name": "Bina adresi", "layer": "bina", "draft": copy.deepcopy(LOOK_BODY),
             "releases": [release_of(LOOK_BODY, 1)]}
LOOK_ENFORCED = {**copy.deepcopy(LOOK_FORM), "enforce": True}
LOOK_IDS = [1, 2] + [i for ids in LOOK_LAYER_IDS.values() for i in ids]


def look_setup(form=LOOK_FORM, first=None):
    layers = [layer("bina", "Bina", LOOK_FIELDS)] + [
        layer(i, LOOK_WORLD[i]["name"], LOOK_WORLD[i]["fields"]) for i in LOOK_LAYER_IDS]
    entities = [{"kind": "polygon", "id": 1, "layerId": "bina", "attrs": first or {}, "pts": square(0, 0)},
                {"kind": "polygon", "id": 2, "layerId": "bina", "attrs": {}, "pts": square(20, 0)}]
    for lid, ids in LOOK_LAYER_IDS.items():
        for k, i in enumerate(ids):
            entities.append({"kind": "polygon", "id": i, "layerId": lid, "attrs": LOOK_ROW_ATTRS[i],
                             "pts": square(40 + 12 * k, 20 + 12 * list(LOOK_LAYER_IDS).index(lid))})
    settings = dict(SETTINGS)
    settings["forms"] = [form]
    return {"format": "kentos.document", "version": 1, "name": "Adresler", "settings": settings, "origin": P(E0, N0),
            "layers": layers, "activeLayer": "bina", "entities": entities, "styles": {"items": [], "categories": []}}


def look_world(meters=None):
    """The world the commands see: the setup's rows by their `$uidOf:N`; `meters`: the meters' `bina` written."""
    w = {}
    for lid, ids in LOOK_LAYER_IDS.items():
        rows = []
        for i in ids:
            a = dict(LOOK_ROW_ATTRS[i])
            if lid == "sayac" and meters and i in meters:
                a["bina"] = meters[i]
            rows.append([f"$uidOf:{i}", a])
        w[lid] = {"name": LOOK_WORLD[lid]["name"], "fields": LOOK_WORLD[lid]["fields"], "rows": rows}
    return w


def look_command_cases():
    """cad.forms.validate, .submit, cad.entities.set and cad.forms.fill through a form with lists from other layers and a
    related item (docs/adr/0404)."""
    validate, submit, sets, fills = [], [], [], []
    obj1 = {**SQUARE, "uid": "$uidOf:1"}

    def answer(attrs, values=(), world=None, obj=obj1):
        world = world or look_world()
        f = Form(LOOK_BODY, LOOK_FIELDS, (), world)
        after, found = f.assess(dict(attrs), utf8_sorted(dict(values)), obj, False)
        issues = preflight(LOOK_BODY, LOOK_FIELDS, (), world) + found
        return {"status": "completed", "output": with_requires({"issues": shown_issues(issues), "valid": not errors(issues),
                                                  "items": f.described(after, obj), "version": 1, "attrs": after}, LOOK_BODY),
                "warnings": []}

    def ask(name, attrs, values, steps_before=(), meters=None):
        inp = {"form": "adres", "uid": "$uidOf:1"}
        if values:
            inp["values"] = dict(values)
        validate.append({"name": f"başka katmandan liste: {name}", "setup": look_setup(first=attrs), "steps": [
            *steps_before, {"op": "plan", "input": inp, "result": answer(attrs, values, look_world(meters))}]})

    ask("boş kayıt: iller doğal sırada, zincir boş", {}, ())
    ask("etiketler yazılmış gibi: kodları, zincir ve süzgeç", {}, [("İl", "Ankara"), ("İlçe", "Çankaya")])
    ask("zincirin dışında kalan ilçe", {"İl": "34"}, [("İlçe", "0601")])
    meters_set = {"op": "execute", "command": "cad.entities.set", "input": {"uids": ["$uidOf:15", "$uidOf:16"], "attrs": {"bina": "$uidOf:1"},
                                             "operation": "attributes"},
                  "result": {"status": "completed", "output": {"changed": ["$uidOf:15", "$uidOf:16"], "ids": [15, 16],
                                                                "revision": "$current"}, "warnings": []},
                  "expect": {"ids": LOOK_IDS, "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}}
    ask("ilişkili sayaçlar binanın kimliğini tutar", {"İl": "06"}, (), [meters_set], {15: "$uidOf:1", 16: "$uidOf:1"})

    def write(name, attrs, values, ok=True):
        world = look_world()
        f = Form(LOOK_BODY, LOOK_FIELDS, (), world)
        after, issues = f.assess(dict(attrs), utf8_sorted(dict(values)), obj1, False)
        inp = {"form": "adres", "uid": "$uidOf:1", "values": dict(values)}
        if errors(issues):
            first = errors(issues)[0]
            path = f"values.{first['field']}" if first.get("field") in dict(values) else "values"
            step = {"op": "execute", "input": inp, "result": failed("form_invalid", ANY, path),
                    "expect": {"ids": LOOK_IDS, "fields": {"1": {"attrs": attrs}}, "canUndo": False, "canRedo": False,
                               "revision": "same", "dirty": False}}
        else:
            changed = after != attrs
            step = {"op": "execute", "input": inp,
                    "result": {"status": "completed", "output": submitted(
                        {"uid": "$uidOf:1", "attrs": after, "changed": changed, "revision": "$current", "version": 1},
                        "$uidOf:1", attrs, after, "bina", "Form: Bina adresi", ANY), "warnings": []},
                    "expect": {"ids": LOOK_IDS, "fields": {"1": {"attrs": after}}, "canUndo": changed, "canRedo": False,
                               **({"dirty": True, "revision": "changed"} if changed else {"revision": "same"})}}
        submit.append({"name": f"başka katmandan liste: {name}", "setup": look_setup(first=attrs), "steps": [step]})

    write("etiketlerle il, ilçe ve mahalle kodlarıyla yazılır", {}, [("İl", "Ankara"), ("İlçe", "Çankaya"), ("Mahalle", "Bahçelievler")])
    write("parsel adasıyla seçilir, nesnenin kimliği yazılır", {"İl": "06"}, [("Parsel", "101/12")])
    write("kapalı mahalle yazılmaz", {"İl": "06", "İlçe": "0601"}, [("Mahalle", "M3")])
    write("olmayan parsel kimliği yazılmaz", {"İl": "06"}, [("Parsel", UNKNOWN_UID)])

    def guard(name, before, attrs):
        world = look_world()
        f = Form(LOOK_BODY, LOOK_FIELDS, (), world)
        after = {**before, **attrs}
        issues = f.enforce(before, after, obj1)
        inp = {"uids": ["$uidOf:1"], "attrs": attrs, "operation": "attributes"}
        if errors(issues):
            step = {"op": "execute", "input": inp, "result": failed("form_rules", ANY, "attrs"),
                    "expect": {"ids": LOOK_IDS, "fields": {"1": {"attrs": before}}, "canUndo": False, "canRedo": False,
                               "revision": "same", "dirty": False}}
        else:
            step = {"op": "execute", "input": inp,
                    "result": {"status": "completed", "output": {"changed": ["$uidOf:1"], "ids": [1], "revision": "$current"},
                               "warnings": []},
                    "expect": {"ids": LOOK_IDS, "fields": {"1": {"attrs": after}}, "canUndo": True, "canRedo": False,
                               "dirty": True, "revision": "changed"}}
        sets.append({"name": f"başka katmandan liste: {name}", "setup": look_setup(LOOK_ENFORCED, before), "steps": [step]})

    guard("kuralları uygulanan formda zincire uyan yazılır", {"İl": "06"}, {"İlçe": "0602"})
    guard("kuralları uygulanan formda listede olmayan değer başka yoldan da yazılamaz", {"İl": "06"}, {"İlçe": "3401"})

    v = {"İl": "Ankara", "İlçe": "Altındağ"}
    world = look_world()
    f = Form(LOOK_BODY, LOOK_FIELDS, (), world)
    a1, _ = f.assess({}, utf8_sorted(v), obj1, False)
    a2, _ = f.assess({}, utf8_sorted(v), {**SQUARE, "uid": "$uidOf:2"}, False)
    fills.append({"name": "başka katmandan liste: iki binaya aynı il ve ilçe etiketleriyle", "setup": look_setup(), "steps": [
        {"op": "execute", "input": {"form": "adres", "uids": ["$uidOf:1", "$uidOf:2"], "values": v},
         "result": {"status": "completed", "output": {"changed": ["$uidOf:1", "$uidOf:2"], "version": 1,
                                                      "revision": "$current"}, "warnings": []},
         "expect": {"ids": LOOK_IDS, "fields": {"1": {"attrs": a1}, "2": {"attrs": a2}}, "canUndo": True, "canRedo": False,
                    "dirty": True, "revision": "changed"}}]})
    return validate, submit, sets, fills


# ── A layer's records checked by its form (docs/adr/0406 §1) ────────────

AUDIT_ENTITIES = FILL_ENTITIES + [
    {"kind": "polygon", "id": 7, "layerId": "bina", "attrs": {"Durum": "iyi", "Kat": "0"}, "pts": square(100, 0)},
    {"kind": "polygon", "id": 8, "layerId": "bina", "attrs": {"Durum": "iyi", "Kat": "3", "Kod": "B-3"}, "pts": square(120, 0)},
    {"kind": "polygon", "id": 9, "layerId": "bina", "attrs": {"Durum": "inceleme", "Kat": "2", "Açıklama": "Kısa"},
     "pts": square(140, 0)},
    {"kind": "polygon", "id": 10, "layerId": "bina", "attrs": {"Durum": "inceleme", "Kat": "1", "Açıklama": "Yedi harf"},
     "pts": square(160, 0)},
    {"kind": "polygon", "id": 11, "layerId": "bina", "attrs": {"Durum": "iyi", "Kat": "1", "Tür": "sanayi", "Malzeme": "ahsap"},
     "pts": square(180, 0)},
]


def audit_cases():
    cases = []

    def audited(name, inp, body=CALC_BODY, fields=CALC_FIELDS, entities=AUDIT_ENTITIES, forms=None, variables=(),
                version=1, world=None, objects=None):
        f = Form(body, fields, variables, world)
        rows = [e for e in entities if e["layerId"] == "bina"]
        held = {}
        unique = [fl["name"] for x, _ in f.bound() if x.get("unique") for fl in [f.layer(x)]]
        for e in rows:
            for k, n in enumerate(unique):
                v = e["attrs"].get(n, "").strip(JS_SPACE)
                if v:
                    held[(k, v)] = held.get((k, v), 0) + 1
        out = {"checked": 0, "failing": 0, "warned": 0, "records": []}
        limit = inp.get("limit", 500)
        more = False
        for e in rows:
            out["checked"] += 1
            obj = (objects or {}).get(e["id"], SQUARE)
            after, issues = f.complete(e["attrs"], obj, False)
            said = [i.get("field") for i in issues]
            issues = issues + [i for i in f.validate(after, obj) if i.get("field") not in said]
            if any(held.get((k, after.get(n, "").strip(JS_SPACE)), 0) > 1 for k, n in enumerate(unique)):
                issues += f.unique_issues(after, obj, [x["attrs"] for x in rows if x is not e])
            err = bool(errors(issues))
            if err:
                out["failing"] += 1
            elif issues:
                out["warned"] += 1
            if err or (inp.get("warnings") and issues):
                if len(out["records"]) < limit:
                    out["records"].append({"uid": f"$uidOf:{e['id']}", "issues": shown_issues(issues)})
                else:
                    more = True
        if version is not None:
            out["version"] = version
        if more:
            out["more"] = True
        setup_doc = setup(forms or [CALC_FORM], fields, entities) if world is None else None
        return setup_doc, {"op": "plan", "input": inp, "result": {"status": "completed", "output": out, "warnings": []}}

    def case(name, inp, **more):
        doc, step = audited(name, inp, **more)
        cases.append({"name": name, "setup": more.get("setup_doc") or doc, "steps": [step]})

    case("katmanın kayıtları: hatalılar katmanın sırasıyla; tekil kod iki kayıtta", {"form": "hesap"})
    case("uyarısı olanlar da", {"form": "hesap", "warnings": True})
    case("en çok iki kayıt; fazlası more", {"form": "hesap", "warnings": True, "limit": 2})
    case("tek kayıt", {"form": "hesap", "limit": 1})
    case("an ve kullanıcı verilir: var olan kayıt varsayılan almaz", {"form": "hesap", **MOMENT_GIVEN}, variables=GIVEN)
    hidden = calc_with(lambda b: b.update(clearHidden=True))
    hidden_form = {**copy.deepcopy(CALC_FORM), "draft": hidden, "releases": [release_of(hidden, 1)]}
    case("gizlenenleri silen form: gizlenen açıklama denetlenmez", {"form": "hesap", "warnings": True}, body=hidden, forms=[hidden_form])
    draft_form = {"id": "hesap", "name": "Bina hesabı", "layer": "bina", "draft": copy.deepcopy(CALC_BODY)}
    case("yayınlanmamış form taslağıyla", {"form": "hesap", "draft": True}, forms=[draft_form], version=None)
    case("kuralları uygulanan form da denetler", {"form": "hesap"}, forms=[CALC_ENFORCED])
    no_unique = calc_with(lambda b: item(b, "kod").pop("unique"))
    nu_form = {**copy.deepcopy(CALC_FORM), "draft": no_unique, "releases": [release_of(no_unique, 1)]}
    case("tekil olmayan kod: yinelenen kod sorun değil", {"form": "hesap"}, body=no_unique, forms=[nu_form])
    soft = calc_with(lambda b: item(b, "aciklama")["checks"].__setitem__(0, {**item(b, "aciklama")["checks"][0], "warning": True}))
    soft_form = {**copy.deepcopy(CALC_FORM), "draft": soft, "releases": [release_of(soft, 1)]}
    case("sert denetim yumuşatılınca kayıt yalnız uyarır", {"form": "hesap", "warnings": True}, body=soft, forms=[soft_form])
    case("katmanda kayıt yok: sıfır kayıt denetlenir", {"form": "hesap", "warnings": True},
         entities=[e for e in AUDIT_ENTITIES if e["layerId"] != "bina"])

    def refused(name, inp, result, forms=None):
        cases.append({"name": name, "setup": setup(forms or [CALC_FORM], CALC_FIELDS, AUDIT_ENTITIES), "steps": [
            {"op": "plan", "input": inp, "result": result}]})

    refused("sıfır kayıt listelenmez", {"form": "hesap", "limit": 0},
            failed("invalid_count", "Listelenecek kayıt sayısı 1 ile 10000 arasında olmalı.", "limit"))
    refused("en çok 10000 kayıt listelenir", {"form": "hesap", "limit": 10001},
            failed("invalid_count", "Listelenecek kayıt sayısı 1 ile 10000 arasında olmalı.", "limit"))
    refused("olmayan form", {"form": "yok"}, unknown_form("yok", "form"))
    refused("yayınlanmamış form taslaksız", {"form": "hesap"}, unpublished("Bina hesabı", "draft"), forms=[draft_form])
    broken = {**copy.deepcopy(CALC_FORM), "draft": BROKEN, "releases": []}
    refused("ön denetimde hata olan form", {"form": "hesap", "draft": True}, failed("form_preflight", ANY, "form"), forms=[broken])
    refused("geçersiz an", {"form": "hesap", "moment": "dün"},
            failed("invalid_moment", "“dün” bir an değil. Bir tarih (2026-10-16) ya da tarih ve yerel saat (2026-10-16T14:30:00) yazın.", "moment"))
    refused("geçersiz kullanıcı", {"form": "hesap", "user": " "},
            failed("invalid_user", "Kullanıcının adı boş olamaz, başında ve sonunda boşluk olmaz, en çok 64 karakterdir.", "user"))

    # Lists from other layers: a district outside the chain, a parcel that is not there.
    world = look_world()
    f = Form(LOOK_BODY, LOOK_FIELDS, (), world)
    first = {"İl": "34", "İlçe": "0601"}
    out = {"checked": 2, "failing": 1, "warned": 0, "version": 1,
           "records": [{"uid": "$uidOf:1", "issues": shown_issues(f.complete(first, {**SQUARE, "uid": "$uidOf:1"}, False)[1]
                                                             + f.validate(first, {**SQUARE, "uid": "$uidOf:1"}))}]}
    cases.append({"name": "başka katmandan liste: zincirin dışındaki ilçe", "setup": look_setup(first=first), "steps": [
        {"op": "plan", "input": {"form": "adres"}, "result": {"status": "completed", "output": out, "warnings": []}}]})
    out2 = {"checked": 2, "failing": 0, "warned": 0, "version": 1, "records": []}
    cases.append({"name": "başka katmandan liste: boş kayıtlar sorunsuz", "setup": look_setup(), "steps": [
        {"op": "plan", "input": {"form": "adres", "warnings": True}, "result": {"status": "completed", "output": out2, "warnings": []}}]})
    return cases


# ── Several values of a list (docs/adr/0408) ────────────────────────────

MULTI_FIELDS = [
    {"name": "Ad", "kind": "text"},
    {"name": "Bakım", "kind": "text", "multiple": True,
     "values": [{"code": "budama", "label": "Budama"}, {"code": "ilaclama", "label": "İlaçlama"},
                {"code": "sulama", "label": "Sulama"}, {"code": "kesim", "label": "Kesim"}]},
    {"name": "Durum", "kind": "text", "values": [{"code": "saglikli", "label": "Sağlıklı"}, {"code": "kuru", "label": "Kuru"}]},
]
MULTI_BODY = {"revision": 1, "items": [
    {"type": "field", "id": "ad", "field": "Ad"},
    {"type": "field", "id": "durum", "field": "Durum"},
    {"type": "field", "id": "bakim", "field": "Bakım", "choiceFilter": "Durum = 'kuru' veya @secenek <> 'kesim'"},
]}


def multi_with(change):
    b = copy.deepcopy(MULTI_BODY)
    change(b)
    return b


def multi_core():
    pre, records, assess = [], [], []

    def case(name, body=MULTI_BODY, fields=MULTI_FIELDS):
        pre.append({"name": name, "fields": fields, "body": body, "issues": preflight(body, fields)})

    case("çok değerli alan: çoklu seçimle, süzgeç her koda; hata yok")
    case("çok değerli alanı tek seçimli liste düzenleyemez",
         multi_with(lambda b: item(b, "bakim").update(widget="list")))
    case("çoklu seçim yalnız çok değerli alanda",
         multi_with(lambda b: (item(b, "durum").update(widget="checkList"), item(b, "ad").update(widget="checkList"))))
    case("çok değerli varsayılan listenin sırasıyla", multi_with(lambda b: item(b, "bakim").update(default="Sulama; budama")))
    # docs/adr/0431: chips on a listed multiple field, a searched list on a single-valued listed one.
    case("çok değerli alan çiplerle", multi_with(lambda b: item(b, "bakim").update(widget="chips")))
    case("tek değerli alan aranır listeyle", multi_with(lambda b: item(b, "durum").update(widget="search")))
    case("çipler yalnız çok değerli alanda", multi_with(lambda b: item(b, "durum").update(widget="chips")))
    case("aranır liste değer listesi ister", multi_with(lambda b: item(b, "ad").update(widget="search")))
    case("aranır liste çok değerli alanı düzenleyemez", multi_with(lambda b: item(b, "bakim").update(widget="search")))
    # docs/adr/0437: a tree of the list's labels.
    case("tek değerli alan ağaçla", multi_with(lambda b: item(b, "durum").update(widget="tree")))
    case("ağaç çok değerli alanı düzenleyemez", multi_with(lambda b: item(b, "bakim").update(widget="tree")))
    case("ağaç değer listesi ister", multi_with(lambda b: item(b, "ad").update(widget="tree")))
    # docs/adr/0437: a mirror shows the field again, read-only; its own item writes it.
    mirror = {"type": "field", "id": "durum-ozet", "field": "Durum", "mirror": True, "label": "Durum (özet)",
              "visibleWhen": "Ad <> ''"}
    case("yansıma: aynı alan ikinci kez, salt okunur; sorun yok", multi_with(lambda b: b["items"].append(dict(mirror))))
    case("yansımanın alanı formda yok", multi_with(lambda b: (b["items"].append(dict(mirror)), b["items"].pop(1))))
    case("yansımada yazan kural olmaz", multi_with(lambda b: b["items"].append({**mirror, "required": True})))
    case("yansımasız ikinci öğe yine iki kez sayılır", multi_with(lambda b: b["items"].append({**mirror, "mirror": False})))

    def record(name, attrs, body=MULTI_BODY):
        form = Form(body, MULTI_FIELDS)
        records.append({"name": name, "fields": MULTI_FIELDS, "body": body, "attrs": attrs,
                        "items": form.described(attrs), "issues": form.validate(attrs)})

    record("sağlıklı ağaçta kesim sunulmaz; budama ve sulama geçerli", {"Ad": "Çınar", "Durum": "saglikli", "Bakım": "budama; sulama"})
    record("sağlıklı ağaçta seçilmiş kesim süzgece takılır", {"Ad": "Çınar", "Durum": "saglikli", "Bakım": "budama; kesim"})
    record("kuru ağaçta kesim de sunulur", {"Ad": "Çınar", "Durum": "kuru", "Bakım": "kesim"})
    record("listede olmayan kod geçersiz", {"Ad": "Çınar", "Bakım": "budama; gübre"})
    record("eski sırasız değer geçerli sayılır", {"Ad": "Çınar", "Bakım": "sulama;budama"})
    mirrored = multi_with(lambda b: b["items"].append(dict(mirror)))
    record("yansıma görünür ama zorunlu ya da yazılır değildir", {"Ad": "Çınar", "Durum": "kuru"}, mirrored)
    record("yansımanın koşulu onu gizler", {"Durum": "kuru"}, mirrored)

    def write(name, before, values, body=MULTI_BODY):
        after, issues = Form(body, MULTI_FIELDS).assess(before, values)
        assess.append({"name": name, "fields": MULTI_FIELDS, "body": body, "before": before,
                       "values": values, "after": after, "issues": issues})

    write("etiketlerle yazılır, listenin sırasıyla tek biçime çevrilir", {"Ad": "Çınar"}, [["Bakım", "Sulama; İLAÇLAMA; budama; sulama"]])
    write("boş parçalar düşer; hepsi boşsa öznitelik kalkar", {"Ad": "Çınar", "Bakım": "budama"}, [["Bakım", " ; ; "]])
    write("kesim sağlıklı ağaca yazılmaz", {"Ad": "Çınar", "Durum": "saglikli"}, [["Bakım", "kesim; budama"]])
    return pre, records, assess


# ── Attachments, colours and suggestions (docs/adr/0407) ────────────────

EXTRA_FIELDS = [
    {"name": "Ad", "kind": "text", "required": True},
    {"name": "Renk", "kind": "text"},
    {"name": "Cins", "kind": "text", "values": [{"code": "agac", "label": "Ağaç"}, {"code": "cali", "label": "Çalı"}]},
    {"name": "Boy", "kind": "decimal", "scale": 1, "min": "0", "max": "40"},
]
EXTRA_BODY = {"revision": 1, "items": [
    {"type": "field", "id": "ad", "field": "Ad", "suggest": True},
    {"type": "field", "id": "renk", "field": "Renk", "widget": "color", "default": "#2E7D32"},
    {"type": "field", "id": "cins", "field": "Cins"},
    {"type": "field", "id": "boy", "field": "Boy"},
    {"type": "attachments", "id": "fotograflar", "label": "Fotoğraflar", "accept": "photo", "min": 1, "max": 3},
    {"type": "attachments", "id": "belgeler", "label": "Belgeler", "accept": "document", "visibleWhen": "Boy > 10"},
    # docs/adr/0433: the record's place, shown by its rule.
    {"type": "location", "id": "konum", "label": "Konum", "visibleWhen": "Boy > 5"},
]}


def extra_with(change):
    b = copy.deepcopy(EXTRA_BODY)
    change(b)
    return b


def held(n_photos, n_all):
    return {**SQUARE, "attachments": {"all": n_all, "photos": n_photos}}


def extras_core():
    pre, records = [], []

    def case(name, body=EXTRA_BODY, fields=EXTRA_FIELDS):
        pre.append({"name": name, "fields": fields, "body": body, "issues": preflight(body, fields)})

    case("ağaç formu: öneri, renk ve ekler; hata yok")
    case("renk yalnız metin alanında; varsayılan renk olmalı",
         extra_with(lambda b: (item(b, "boy").update(widget="color"), item(b, "renk").update(default="yeşil"))))
    case("öneri değer listesi olmayan metin alanında olur",
         extra_with(lambda b: (item(b, "cins").update(suggest=True), item(b, "boy").update(suggest=True))))
    case("eklerin görünürlük kuralı okunmazsa hata, bilinmeyen ad uyarı",
         extra_with(lambda b: (item(b, "fotograflar").update(visibleWhen="Boy >"),
                               item(b, "belgeler").update(visibleWhen="Yaş > 3"))))
    case("en az en çoktan büyükse form bozuktur",
         extra_with(lambda b: item(b, "fotograflar").update(min=4, max=2)))
    case("küçük harfli ve saydamlıklı renk varsayılanı geçerli",
         extra_with(lambda b: item(b, "renk").update(default="#2e7d32cc")))

    def record(name, attrs, obj, body=EXTRA_BODY):
        form = Form(body, EXTRA_FIELDS)
        records.append({"name": name, "fields": EXTRA_FIELDS, "body": body, "object": obj, "attrs": attrs,
                        "items": form.described(attrs, obj), "issues": form.validate(attrs, obj)})

    tree = {"Ad": "Çınar", "Renk": "#2E7D32", "Cins": "agac", "Boy": "12"}
    record("iki fotoğraf ve bir belgeli ağaç: sayılar durumda, sorun yok", tree, held(2, 3))
    record("fotoğrafsız ağaç: en az bir fotoğraf ister", tree, held(0, 0))
    record("dört fotoğraf: en çok üç", tree, held(4, 4))
    record("yalnız fotoğraflar sayılır: iki belge fotoğraf değil", tree, held(0, 2))
    record("boy 10 ve altı: belgeler gizli ve sınırsız; bir fotoğraf yeter", {**tree, "Boy": "5"}, held(1, 1))
    record("ekler bilinmeyen kayıt: sayılmaz, sorulmaz", tree, SQUARE)
    record("renk değil: yeşil", {**tree, "Renk": "yeşil"}, held(1, 1))
    record("renk #RGB kısa yazımı kabul edilmez", {**tree, "Renk": "#2E7"}, held(1, 1))
    record("saydamlıklı renk geçerli", {**tree, "Renk": "#2e7d32cc"}, held(1, 1))
    record("ekler gizli öğede: Fotoğraflar kuralla gizlenince eksiği söylenmez", tree, held(0, 0),
           body=extra_with(lambda b: item(b, "fotograflar").update(visibleWhen="Boy > 20")))
    return pre, records


TREES = [
    {"kind": "polygon", "id": 1, "layerId": "bina", "pts": square(0, 0),
     "attrs": {"Ad": "Çınar", "Renk": "#2E7D32", "Cins": "agac", "Boy": "12"},
     "attachments": [{"name": "Gövde", "file": "foto/cinar-1.jpg"}, {"name": "Taç", "file": "foto/cinar-2.JPEG"},
                     {"name": "Rapor", "file": "belge/cinar.pdf"}]},
    {"kind": "polygon", "id": 2, "layerId": "bina", "pts": square(20, 0), "attrs": {"Ad": "Çınar", "Boy": "8"}},
    ENTITIES[2],
    {"kind": "polygon", "id": 4, "layerId": "kilitli", "attrs": {"Durum": "iyi"}, "pts": square(40, 0)},
    {"kind": "polygon", "id": 5, "layerId": "bina", "pts": square(60, 0), "attrs": {"Ad": "Ihlamur", "Boy": "4"},
     "attachments": [{"name": f"Foto {k}", "url": f"https://ornek.gov.tr/agac/{k}.png?boy=b"} for k in range(4)]},
    {"kind": "polygon", "id": 6, "layerId": "bina", "pts": square(80, 0), "attrs": {"Ad": " Ihlamur ", "Renk": "yeşil"},
     "attachments": [{"name": "Foto", "file": "foto/ihlamur.png"}]},
    {"kind": "polygon", "id": 7, "layerId": "bina", "pts": square(100, 0), "attrs": {"Ad": "Meşe", "Boy": "15"},
     "attachments": [{"name": "Foto", "file": "foto/mese.jpg"}, {"name": "Harita", "url": "https://ornek.gov.tr/harita"}]},
]
TREE_IDS = [e["id"] for e in TREES]
TREE_BODY = EXTRA_BODY
TREE_FORM = {"id": "agac", "name": "Ağaç envanteri", "layer": "bina", "draft": copy.deepcopy(TREE_BODY),
             "releases": [release_of(TREE_BODY, 1)]}


def photo_link(link):
    path = link.split("?")[0].split("#")[0].lower()
    return path.endswith((".jpg", ".jpeg", ".png"))


def tree_object(e):
    """The object a command judges: its measures and its attachments counted (docs/adr/0407 §1)."""
    atts = e.get("attachments", [])
    photos = sum(1 for a in atts if "asset" in a or photo_link(a.get("file") or a.get("url") or ""))
    return {**SQUARE, "attachments": {"all": len(atts), "photos": photos}}


def tree_suggestions(name):
    """docs/adr/0407 §3: the layer's values, trimmed, the most used first, a tie by the first held; at most 20."""
    seen = {}
    for e in TREES:
        if e["layerId"] != "bina":
            continue
        v = e["attrs"].get(name, "").strip(JS_SPACE)
        if v:
            seen[v] = seen.get(v, 0) + 1
    order = list(seen)
    return sorted(order, key=lambda v: -seen[v])[:20]


def tree_validate_cases():
    cases = []
    S = setup([TREE_FORM], EXTRA_FIELDS, TREES)
    by_id = {e["id"]: e for e in TREES}

    def run(inp, result):
        return {"op": "execute", "input": inp, "result": result,
                "expect": {"ids": TREE_IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}}

    def answer(slot=None, values=()):
        out = {"version": 1, "issues": [], "valid": True, "items": []}
        issues = preflight(TREE_BODY, EXTRA_FIELDS)
        if slot is not None:
            f = Form(TREE_BODY, EXTRA_FIELDS)
            obj = tree_object(by_id[slot])
            after, found = f.assess(dict(by_id[slot]["attrs"]), utf8_sorted(dict(values)), obj, False)
            items = f.described(after, obj)
            for st in items:
                if st["id"] == "ad":
                    st["suggestions"] = tree_suggestions("Ad")
            out["items"] = items
            issues = issues + found
            out["attrs"] = after
        out["issues"] = shown_issues(issues)
        out["valid"] = not errors(issues)
        return {"status": "completed", "output": with_requires(out, TREE_BODY), "warnings": []}

    for name, slot, values in [
        ("ağaç: öneriler en çok kullanılandan, fotoğraf ve ek sayıları", 1, ()),
        ("fotoğrafsız ağaç en az bir fotoğraf ister", 2, ()),
        ("dört fotoğraf en çok üçü aşar; adresler de fotoğraf sayılır", 5, ()),
        ("renk değil: yeşil; öneri boşlukları kırpar", 6, ()),
        ("belgelerden biri adres: ekler sayılır, fotoğraf yalnız biri", 7, ()),
        ("yazılmış gibi: saydamlıklı renk geçerli", 6, (("Renk", "#2e7d32cc"),)),
        ("yazılmış gibi: renk adı reddedilir", 1, (("Renk", "kırmızı"),)),
        ("yazılmış gibi: boy düşünce belgeler gizlenir", 1, (("Boy", "6"),)),
    ]:
        cases.append({"name": f"ekler, renk ve öneri: {name}", "setup": S, "steps": [
            run({"form": "agac", "uid": f"$uidOf:{slot}", "values": dict(values)}, answer(slot, values))]})
    cases.append({"name": "ekler, renk ve öneri: form tek başına", "setup": S, "steps": [
        run({"form": "agac"}, answer())]})
    return cases


def tree_audit_cases():
    S = setup([TREE_FORM], EXTRA_FIELDS, TREES)
    f = Form(TREE_BODY, EXTRA_FIELDS)
    out = {"version": 1, "checked": 0, "failing": 0, "warned": 0, "records": []}
    for e in TREES:
        if e["layerId"] != "bina":
            continue
        out["checked"] += 1
        obj = tree_object(e)
        after, issues = f.complete(e["attrs"], obj, False)
        said = [i.get("field") for i in issues]
        issues = issues + [i for i in f.validate(after, obj) if i.get("field") not in said]
        if errors(issues):
            out["failing"] += 1
            out["records"].append({"uid": f"$uidOf:{e['id']}", "issues": shown_issues(issues)})
        elif issues:
            out["warned"] += 1
    return [{"name": "ağaçlar: eksik ve fazla fotoğraf, renk olmayan renk", "setup": S, "steps": [
        {"op": "plan", "input": {"form": "agac"}, "result": {"status": "completed", "output": out, "warnings": []}}]}]


# ── Relations (docs/adr/0405) ───────────────────────────────────────────

REL_FIELDS = {"bina": [{"name": "Ad", "kind": "text"}],
              "sayac": [{"name": "no", "kind": "text"}, {"name": "bina", "kind": "text"}],
              "vana": [{"name": "no", "kind": "text"}, {"name": "sayac", "kind": "text"}],
              "abone": [{"name": "ad", "kind": "text"}, {"name": "bina", "kind": "text"}],
              # docs/adr/0409 §1: an owner of several buildings (many-to-many).
              "malik": [{"name": "ad", "kind": "text"}, {"name": "binalar", "kind": "text", "multiple": True}]}
# The objects: two buildings; meters of the first two and of the second one; valves of the meters; a subscriber.
REL_OBJECTS = [(1, "bina", {"Ad": "A"}), (2, "bina", {"Ad": "B"}), (3, "sayac", {"no": "S-1"}), (4, "sayac", {"no": "S-2"}),
               (5, "sayac", {"no": "S-3"}), (6, "vana", {"no": "V-1"}), (7, "vana", {"no": "V-2"}), (8, "abone", {"ad": "Ayşe"}),
               (9, "malik", {"ad": "Mehmet"})]
# Who refers to whom: a child, its field and its parent.
REL_LINKS = [(3, "bina", 1), (4, "bina", 1), (5, "bina", 2), (6, "sayac", 3), (7, "sayac", 5), (8, "bina", 1),
             (9, "binalar", (1, 2))]
REL_IDS = [o[0] for o in REL_OBJECTS]


def parts(value):
    """A reference field's ids: one, or a many-to-many field's several (docs/adr/0409 §1)."""
    return [h.strip(JS_SPACE) for h in value.split(";") if h.strip(JS_SPACE)]


def relation(i, parent, child, field, on_delete=None, name=None):
    r = {"id": i, "name": name or i, "parent": parent, "child": child, "field": field}
    if on_delete and on_delete != "unlink":
        r["onDelete"] = on_delete
    return r


def rel_setup(relations=(), locked=()):
    layers = [layer(i, i.capitalize(), f, locked=i in locked) for i, f in REL_FIELDS.items()] + \
        [layer("grup", "Grup", kind="group", children=[layer("alt", "Alt katman")])]
    entities = [{"kind": "polygon", "id": i, "layerId": l, "attrs": a, "pts": square(12 * k, 0)}
                for k, (i, l, a) in enumerate(REL_OBJECTS)]
    settings = dict(SETTINGS)
    if relations:
        settings["relations"] = list(relations)
    return {"format": "kentos.document", "version": 1, "name": "İlişkiler", "settings": settings, "origin": P(E0, N0),
            "layers": layers, "activeLayer": "bina", "entities": entities, "styles": {"items": [], "categories": []}}


def relations_edit_cases():
    cases = []
    base = [relation("bina-sayac", "bina", "sayac", "bina", name="Binanın sayaçları")]

    def links_of(r):
        """docs/adr/0440 §3: the children whose field holds a value of the relation's."""
        found = [f for f in REL_FIELDS.get(r["child"], []) if key_of(f) == r["field"]]
        if not found:
            return 0
        return sum(1 for _, lay, a in REL_OBJECTS if lay == r["child"] and a.get(found[0]["name"], "").strip())

    def edited(name, edit, before=(), after=None, changed=True, op="execute", **more):
        after = list(before) if after is None else after
        inp = {"edit": edit, **more}
        rel = edit.get("relation") if edit["kind"] == "create" else next(
            (r for r in before if r["id"] == edit.get("id", edit.get("relation", {}).get("id"))), None)
        output = {"relations": after, "changed": changed, "revision": "$current"}
        n = links_of(rel) if rel else 0
        if n:
            output["links"] = n
        result = {"status": "completed", "output": output, "warnings": []}
        expect = {"ids": REL_IDS, "canUndo": False, "canRedo": False, "revision": "changed" if changed and op == "execute" else "same"}
        if changed and op == "execute":
            expect["dirty"] = True
        cases.append({"name": name, "setup": rel_setup(before), "steps": [
            {"op": op, "input": inp, "result": result, "expect": expect}]})

    def refused(name, edit, result, before=(), **more):
        cases.append({"name": name, "setup": rel_setup(before), "steps": [
            {"op": "execute", "input": {"edit": edit, **more}, "result": result,
             "expect": {"ids": REL_IDS, "canUndo": False, "canRedo": False, "revision": "same", "dirty": False}}]})

    r = relation("bina-sayac", "bina", "sayac", "bina", name="Binanın sayaçları")
    edited("ilişki oluşturulur; varsayılan politika unlink yazılmaz", {"kind": "create", "relation": r}, after=[r])
    rc = relation("sayac-vana", "sayac", "vana", "sayac", "cascade", "Sayacın vanaları")
    edited("bileşik ilişki: cascade", {"kind": "create", "relation": rc}, base, base + [rc])
    rr = relation("bina-abone", "bina", "abone", "bina", "restrict", "Aboneler")
    edited("restrict", {"kind": "create", "relation": rr}, base, base + [rr])
    rk = relation("bina-abone", "bina", "abone", "bina", "keep", "Aboneler")
    edited("keep", {"kind": "create", "relation": rk}, base, base + [rk])
    rs = relation("sayac-ust", "sayac", "sayac", "no", name="Ana sayaç")
    edited("katman kendisiyle ilişkilenebilir", {"kind": "create", "relation": rs}, base, base + [rs])
    ru = {**base[0], "onDelete": "cascade", "name": "Sayaçlar"}
    edited("ilişki değiştirilir", {"kind": "update", "relation": ru}, base, [ru])
    edited("aynısıyla değiştirmek bir şey değiştirmez", {"kind": "update", "relation": base[0]}, base, base, False)
    edited("ilişki silinir", {"kind": "remove", "id": "bina-sayac"}, base, [])
    edited("plan yazmaz", {"kind": "create", "relation": rc}, base, base + [rc], op="plan")
    refused("kimliği olan ilişki yeniden oluşturulmaz", {"kind": "create", "relation": base[0]},
            failed("relation_exists", "“bina-sayac” kimlikli başka bir ilişki var. Başka bir kimlik verin.", "edit.relation.id"), base)
    many = [relation(f"r{i}", "bina", "sayac", "bina") for i in range(100)]
    refused("en çok 100 ilişki", {"kind": "create", "relation": rc},
            failed("too_many_relations", "Projede en çok 100 ilişki olur. Kullanmadığınız birini silin.", "edit"), many)
    refused("olmayan ilişki değiştirilmez", {"kind": "update", "relation": rc},
            failed("unknown_relation", "“sayac-vana” kimlikli bir ilişki yok. Var olan bir ilişkinin kimliğini verin.", "edit.relation.id"), base)
    refused("olmayan ilişki silinmez", {"kind": "remove", "id": "yok"},
            failed("unknown_relation", "“yok” kimlikli bir ilişki yok. Var olan bir ilişkinin kimliğini verin.", "edit.id"), base)
    refused("geçersiz kimlik", {"kind": "create", "relation": {**rc, "id": "a b"}}, failed("invalid_relation", ANY, "edit.relation"))
    refused("boş ad", {"kind": "create", "relation": {**rc, "name": " "}}, failed("invalid_relation", ANY, "edit.relation"))
    refused("alanı yazılmamış", {"kind": "create", "relation": {**rc, "field": ""}}, failed("invalid_relation", ANY, "edit.relation"))
    refused("silinecek ilişkinin kimliği geçersiz", {"kind": "remove", "id": "a/b"}, failed("invalid_relation", ANY, "edit.id"))
    refused("üst katman yok", {"kind": "create", "relation": {**rc, "parent": "yok"}},
            failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "edit.relation.parent"))
    refused("alt katman bir grup", {"kind": "create", "relation": {**rc, "child": "grup"}},
            failed("not_a_layer", "“Grup” bir katman grubu; ilişki katmanlar arasında olur. Grubun içinden bir katman seçin.", "edit.relation.child"))
    refused("alan alt katmanda yok", {"kind": "create", "relation": {**rc, "field": "bina"}},
            failed("relation_field_unknown", "“Vana” katmanında “bina” alanı yok. Alt katmanın, üst nesnenin kimliğini tutacak alanını seçin.",
                   "edit.relation.field"))
    # docs/adr/0440 §3: compositions in a circle; a key changed under links; the links counted.
    rb = relation("sayac-bina", "sayac", "bina", "Ad", "cascade", "Sayacın binası")
    rcasc = {**base[0], "onDelete": "cascade"}
    refused("bileşik ilişkiler döngü kuramaz", {"kind": "create", "relation": rb}, failed("relation_cycle", ANY, "edit"), [rcasc])
    edited("biri bileşik değilse döngü yoktur", {"kind": "create", "relation": rb}, base, base + [rb])
    ws = {**relation("sayac-ust", "sayac", "sayac", "no", "cascade", "Ana sayaç")}
    edited("katmanın kendisiyle bileşik ilişkisi döngü sayılmaz", {"kind": "create", "relation": ws}, [rcasc], [rcasc, ws])
    link_step = {"op": "execute", "command": "cad.entities.set",
                 "input": {"uids": ["$uidOf:3", "$uidOf:4"], "attrs": {"bina": "$uidOf:1"}, "operation": "attributes"},
                 "result": {"status": "completed", "output": {"changed": ["$uidOf:3", "$uidOf:4"], "ids": [3, 4],
                                                               "revision": "$current"}, "warnings": []},
                 "expect": {"ids": REL_IDS, "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}}
    moved = {**base[0], "field": "no"}
    cases.append({"name": "bağları olan ilişkinin alanı değişemez", "setup": rel_setup(base), "steps": [
        link_step,
        {"op": "execute", "input": {"edit": {"kind": "update", "relation": moved}},
         "result": failed("relation_in_use", ANY, "edit.relation.field"),
         "expect": {"ids": REL_IDS, "revision": "same"}}]})
    cases.append({"name": "bağları olan ilişkinin silme kuralı değişir; bağlar sayılır", "setup": rel_setup(base), "steps": [
        link_step,
        {"op": "execute", "input": {"edit": {"kind": "update", "relation": rcasc}},
         "result": {"status": "completed", "output": {"relations": [rcasc], "changed": True, "links": 2,
                                                      "revision": "$current"}, "warnings": []},
         "expect": {"ids": REL_IDS, "revision": "changed"}}]})
    cases.append({"name": "silinen ilişkinin bağları kayıtlarda kalır ve sayılır", "setup": rel_setup(base), "steps": [
        link_step,
        {"op": "plan", "input": {"edit": {"kind": "remove", "id": "bina-sayac"}},
         "result": {"status": "completed", "output": {"relations": [], "changed": True, "links": 2, "revision": "$current"},
                    "warnings": []},
         "expect": {"ids": REL_IDS, "revision": "same"}}]})
    refused("beklenen sürüm: geçersiz", {"kind": "create", "relation": rc}, failed("invalid_revision", REVISION, "expectedRevision"),
            expectedRevision="07")
    refused("beklenen sürüm: çakışan", {"kind": "create", "relation": rc},
            failed("revision_conflict", CONFLICT, "expectedRevision", "conflict"), expectedRevision="999")
    return cases


def delete_relation_cases():
    """cad.entities.delete with the project's relations (docs/adr/0405 §2): the reference's own follow."""
    cases = []
    capture = [{"op": "captureUid", "id": i, "as": f"o{i}"} for i in REL_IDS]

    def links(relations):
        steps, state = [], {i: dict(a) for i, _, a in REL_OBJECTS}
        for child, field, parent in REL_LINKS:
            held = "; ".join(f"$uid:o{p}" for p in (parent if isinstance(parent, tuple) else (parent,)))
            state[child][field] = held
            steps.append({"op": "execute", "command": "cad.entities.set",
                          "input": {"uids": [f"$uid:o{child}"], "attrs": {field: held}, "operation": "attributes"},
                          "result": {"status": "completed", "output": {"changed": [f"$uid:o{child}"], "ids": [child],
                                                                        "revision": "$current"}, "warnings": []},
                          "expect": {"ids": REL_IDS, "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}})
        return steps, state

    layer_of = {i: l for i, l, _ in REL_OBJECTS}

    def follow(state, ids, relations, locked, counts=None):
        """The objects left and their attributes after deleting `ids` (each relation's policy); `counts` takes each
        relation's unlinked and deleted children (docs/adr/0440 §1). A relation that would change a locked layer's
        child raises."""
        gone, out = list(ids), {i: dict(a) for i, a in state.items() if i not in ids}
        queue = list(ids)
        counts = {} if counts is None else counts
        while queue:
            parent = queue.pop()
            for r in relations:
                pol = r.get("onDelete", "unlink")
                if r["parent"] != layer_of[parent] or pol in ("keep", "restrict"):
                    continue
                if r["child"] in locked:
                    if [f for f in REL_FIELDS[r["child"]] if f["name"] == r["field"]] and any(
                            layer_of[c] == r["child"] and f"$uid:o{parent}" in parts(out[c].get(r["field"], ""))
                            for c in out):
                        raise LookupError(r["name"])
                    continue
                found = [f for f in REL_FIELDS[r["child"]] if f["name"] == r["field"]]
                if not found:
                    continue
                names = [found[0]["name"]]
                me = f"$uid:o{parent}"
                for c in [c for c in out if layer_of[c] == r["child"] and me in parts(out[c].get(names[0], ""))]:
                    # docs/adr/0409 §2: a many-to-many child keeps its other parents; cascaded only when none is left.
                    rest = [h for h in parts(out[c][names[0]]) if h != me] if found[0].get("multiple") else []
                    n = counts.setdefault(r["id"], [0, 0])
                    if rest:
                        out[c][names[0]] = "; ".join(rest)
                        n[0] += 1
                    elif pol == "cascade":
                        out.pop(c)
                        gone.append(c)
                        queue.append(c)
                        n[1] += 1
                    else:
                        out[c].pop(names[0])
                        n[0] += 1
        return out

    def restricted(state, ids, relations):
        for k, i in enumerate(ids):
            for r in relations:
                if r["parent"] != layer_of[i] or r.get("onDelete") != "restrict":
                    continue
                if any(layer_of[c] == r["child"] and c not in ids and f"$uid:o{i}" in parts(state[c].get(r["field"], ""))
                       for c in state):
                    return k, r["name"]
        return None

    def case(name, relations, ids, locked=(), undo=False, given=None, plan=False):
        steps, state = links(relations)
        # A locked layer is locked after its links are written.
        for lid in locked:
            steps.append({"op": "execute", "command": "cad.layers.set",
                          "input": {"operation": "locked", "layer": lid, "value": True},
                          "result": {"status": "completed", "output": {"changed": True, "active": "bina", "revision": "$current"},
                                     "warnings": []},
                          "expect": {"ids": REL_IDS}})
        given = given or ids
        inp = {"uids": [f"$uid:o{i}" for i in given]}
        stop = restricted(state, ids, relations)
        if stop:
            k, rname = stop
            steps.append({"op": "execute", "input": inp,
                          "result": failed("relation_restricted", f"“{rname}” ilişkisi alt kayıtları olan nesnenin silinmesine izin vermiyor. "
                                                                    "Önce alt kayıtları silin ya da bağlarını koparın.", f"uids[{k}]"),
                          "expect": {"ids": REL_IDS, "revision": "same"}})
        else:
            counts = {}
            try:
                after = follow(state, ids, relations, locked, counts)
            except LookupError:
                # docs/adr/0440 §1: a locked layer's child is not changed behind the user's back.
                steps.append({"op": "execute", "input": inp, "result": failed("relation_locked", ANY, "uids"),
                              "expect": {"ids": REL_IDS, "revision": "same"}})
                cases.append({"name": name, "setup": rel_setup(relations), "steps": capture + steps})
                return
            output = {"removed": [f"$uid:o{i}" for i in ids], "locked": [], "revision": "$current"}
            followed = [{"relation": r["id"], "unlinked": counts[r["id"]][0], "deleted": counts[r["id"]][1]}
                        for r in relations if r["id"] in counts]
            if followed:
                output["followed"] = followed
            if plan:
                steps.append({"op": "plan", "input": inp, "result": {"status": "completed", "output": output,
                                                                     "warnings": []},
                              "expect": {"ids": REL_IDS, "revision": "same"}})
            steps.append({"op": "execute", "input": inp,
                          "result": {"status": "completed", "output": output, "warnings": []},
                          "expect": {"ids": sorted(after), "fields": {str(i): {"attrs": a} for i, a in after.items()},
                                     "canUndo": True, "dirty": True, "revision": "changed"}})
            if undo:
                steps.append({"op": "undo", "expect": {"ids": REL_IDS,
                                                        "fields": {str(i): {"attrs": a} for i, a in state.items()},
                                                        "canRedo": True}})
        cases.append({"name": name, "setup": rel_setup(relations), "steps": capture + steps})

    R = lambda pol: relation("bina-sayac", "bina", "sayac", "bina", pol, "Binanın sayaçları")  # noqa: E731
    V = lambda pol: relation("sayac-vana", "sayac", "vana", "sayac", pol, "Sayacın vanaları")  # noqa: E731
    A = lambda pol: relation("bina-abone", "bina", "abone", "bina", pol, "Binanın aboneleri")  # noqa: E731
    case("ilişkisiz silme yalnız nesneyi siler", [], [1])
    case("unlink: binanın sayaçlarının alanı boşalır", [R("unlink")], [1])
    case("unlink geri alınınca bağlar döner", [R("unlink")], [1], undo=True)
    case("cascade: sayaçlar binayla silinir; vanaların bağı (unlink) boşalır", [R("cascade"), V("unlink")], [1])
    case("cascade zinciri: bina, sayaçlar ve vanaları", [R("cascade"), V("cascade")], [1])
    case("cascade zinciri geri alınınca hepsi döner", [R("cascade"), V("cascade")], [1], undo=True)
    case("restrict: alt kaydı olan bina silinmez", [R("restrict")], [1])
    case("restrict: alt kayıtları da silinen bina silinir", [R("restrict"), V("unlink")], [3, 4, 1])
    case("restrict: girdideki ikinci nesne söylenir", [R("restrict")], [7, 2])
    case("keep: bağ olduğu gibi kalır", [R("keep")], [1])
    case("alt nesneyi silmek üst nesneyi değiştirmez", [R("cascade")], [3])
    case("iki bina birden: bütün sayaçlar bağsız kalır", [R("unlink")], [1, 2])
    case("bir binadan iki ilişki: sayaçlar bağsız, aboneler silinir", [R("unlink"), A("cascade")], [1])
    case("kilitli alt katmandaki kayıtları değiştirecek silme reddedilir", [R("cascade")], [1], locked=("sayac",))
    case("kilitli katmandaki alt kayıtlara dokunmayan ilişki silmeyi durdurmaz", [V("cascade")], [1], locked=("vana",))
    case("plan her ilişkinin bağını koparacağı ve sileceği alt kayıtları sayar", [R("cascade"), V("unlink")], [1], plan=True)
    case("alanı alt katmanda olmayan ilişki bir şey yapmaz", [relation("bina-vana", "bina", "vana", "bina", "cascade")], [1])
    case("restrict başka katmanın silinmesini durdurmaz", [A("restrict")], [3])
    case("sayaç silinince vanası silinir, bina değişmez", [V("cascade")], [3])
    case("cascade ve unlink zinciri geri alınır", [R("cascade"), V("unlink")], [1], undo=True)
    case("ikinci binada restrict, ilk bina unlink ilişkisiyle", [R("restrict")], [2])
    case("restrict ilişkisi zincirin altında: sayaç silinmez", [V("restrict")], [5])
    case("abone cascade, sayaç keep", [R("keep"), A("cascade")], [1], undo=True)
    M = lambda pol: relation("bina-malik", "bina", "malik", "binalar", pol, "Binanın malikleri")  # noqa: E731
    case("çoktan çoğa unlink: malik öbür binasıyla kalır", [M("unlink")], [1])
    case("çoktan çoğa cascade: öbür binası varken malik silinmez", [M("cascade")], [1])
    case("çoktan çoğa cascade: iki binası da gidince malik silinir", [M("cascade")], [1, 2], undo=True)
    case("çoktan çoğa unlink: iki bina gidince alan boşalır", [M("unlink")], [2, 1])
    case("çoktan çoğa restrict: maliki olan bina silinmez", [M("restrict")], [2])
    return cases


CHILD_NEW = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d1001"
CHILD_NEW2 = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d1002"
SAYAC_BODY = {"revision": 1, "items": [{"type": "field", "id": "no", "field": "no", "required": True,
                                        "checks": [{"rule": "uzunluk(no) >= 3", "message": "Numara en az 3 karakter."}]},
                                       {"type": "field", "id": "bina", "field": "bina"}]}
BINA_BODY = {"revision": 1, "items": [{"type": "field", "id": "ad", "field": "Ad"}]}


def submit_children_cases():
    """cad.forms.submit with the record's children (docs/adr/0440 §2), the reference's own rule."""
    cases = []
    bina_form = {"id": "bina", "name": "Bina", "layer": "bina", "draft": copy.deepcopy(BINA_BODY),
                 "releases": [{"version": 1, "digest": digest(BINA_BODY), "body": copy.deepcopy(BINA_BODY)}]}
    sayac_form = {"id": "sayac", "name": "Sayaç", "layer": "sayac", "draft": copy.deepcopy(SAYAC_BODY),
                  "releases": [{"version": 1, "digest": digest(SAYAC_BODY), "body": copy.deepcopy(SAYAC_BODY)}]}
    R = relation("bina-sayac", "bina", "sayac", "bina", name="Binanın sayaçları")
    A = relation("bina-abone", "bina", "abone", "bina", name="Binanın aboneleri")
    M = relation("bina-malik", "bina", "malik", "binalar", name="Binanın malikleri")
    V = relation("sayac-vana", "sayac", "vana", "sayac", name="Sayacın vanaları")
    BAD = relation("bina-vana", "bina", "vana", "bina", name="Binanın vanaları")
    rels = [R, A, M, V, BAD]
    layer_of = {i: l for i, l, _ in REL_OBJECTS}

    def setup_(locked=(), forms=(bina_form, sayac_form)):
        s = copy.deepcopy(rel_setup(rels, locked))
        s["settings"]["forms"] = [copy.deepcopy(f) for f in forms]
        return s

    capture = [{"op": "captureUid", "id": i, "as": f"o{i}"} for i in REL_IDS]

    def link_steps(links):
        """The links written before the submission: a child's field holding its parents' ids."""
        out = []
        for child, field, value in links:
            out.append({"op": "execute", "command": "cad.entities.set",
                        "input": {"uids": [f"$uid:o{child}"], "attrs": {field: value}, "operation": "attributes"},
                        "result": {"status": "completed", "output": {"changed": [f"$uid:o{child}"], "ids": [child],
                                                                      "revision": "$current"}, "warnings": []},
                        "expect": {"ids": REL_IDS, "revision": "changed"}})
        return out

    def children_of(state, parent, new_attrs):
        """The record's children after the submission: (relation, uid, attrs), each relation's sorted by uid."""
        out = []
        for r in rels:
            if r["parent"] != layer_of[parent]:
                continue
            found = [f for f in REL_FIELDS[r["child"]] if key_of(f) == r["field"]]
            if not found:
                continue
            rows = [(f"$uidOf:{i}", a) for i, a in state.items() if i in layer_of and layer_of[i] == r["child"]]
            rows += [(u, a) for u, (lay, a) in new_attrs.items() if lay == r["child"]]
            for u, a in sorted(rows):
                if f"$uid:o{parent}" in parts(a.get(found[0]["name"], "")):
                    out.append((r["id"], u, a))
        return out

    def submit(name, children, values=None, parent=1, op="execute", locked=(), forms=(bina_form, sayac_form), links=(),
               then=None):
        state = {i: dict(a) for i, _, a in REL_OBJECTS}
        for child, field, value in links:
            state[child][field] = value
        inp = {"form": "bina", "uid": f"$uidOf:{parent}"}
        if values:
            inp["values"] = values
        inp["children"] = children
        result, written = answer(inp, state, parent, locked, forms)
        if op == "plan" and result["status"] == "completed":
            result["output"].pop("undo", None)
        pre = link_steps(links)
        expect = {"ids": REL_IDS + [10] if written and any(c.get("created") for c in result["output"]["children"]) else REL_IDS}
        if result["status"] != "completed" or op == "plan" or not written:
            expect.update({"revision": "same"})
        else:
            expect.update({"revision": "changed", "dirty": True, "canUndo": True})
            expect["fields"] = {str(i): {"attrs": a} for i, a in written.items()}
        if pre and "canUndo" in expect:
            expect.pop("canUndo")
        steps = capture + pre + [{"op": op, "input": inp, "result": result, "expect": expect}]
        if then:
            steps += then(inp, written)
        cases.append({"name": name, "setup": setup_(locked, forms), "steps": steps})

    def answer(inp, state, parent, locked, forms):
        """The result and the slots written with their attributes (a new record as slot 10); or a refusal."""
        def stop(code, path, status="failed"):
            return failed(code, ANY, path, status), None
        bina = Form(BINA_BODY, REL_FIELDS["bina"])
        before = state[parent]
        after, issues = bina.assess(dict(before), utf8_sorted(dict(inp.get("values", {}))), {}, False)
        if errors(issues):
            first = errors(issues)[0]
            return stop("form_invalid", f"values.{first['field']}" if first.get("field") in inp.get("values", {}) else "values")
        out_children, written, new_attrs, seen = [], {}, {}, []
        for i, c in enumerate(inp["children"]):
            at = f"children[{i}]"
            uid = c["uid"]
            if not uid.startswith("$uidOf:") and not re.fullmatch(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}", uid):
                return stop("invalid_uid", f"{at}.uid")
            if uid in seen or uid == inp["uid"]:
                return stop("invalid_uid", f"{at}.uid")
            seen.append(uid)
            r = next((r for r in rels if r["id"] == c["relation"]), None)
            if r is None:
                return stop("unknown_relation", f"{at}.relation")
            if r["parent"] != "bina":
                return stop("relation_mismatch", f"{at}.relation")
            link = next((f for f in REL_FIELDS[r["child"]] if key_of(f) == r["field"]), None)
            if link is None:
                return stop("relation_field_unknown", f"{at}.relation")
            slot = int(uid.split(":")[1]) if uid.startswith("$uidOf:") else None
            if slot is not None and layer_of[slot] != r["child"]:
                return stop("form_layer_mismatch", f"{at}.uid")
            if slot is None and c.get("unlink"):
                return stop("entity_not_found", f"{at}.uid")
            if r["child"] in locked:
                return stop("layer_locked", f"{at}.uid")
            base = dict(state[slot]) if slot is not None else {}
            held = parts(base.get(link["name"], ""))
            me = f"$uid:o{parent}"
            if c.get("unlink"):
                nxt = [h for h in held if h != me]
            elif link.get("multiple"):
                nxt = held + ([me] if me not in held else [])
            else:
                nxt = [me]
            attrs = dict(base)
            if nxt:
                attrs[link["name"]] = "; ".join(nxt)
            else:
                attrs.pop(link["name"], None)
            form = next((f for f in forms if f["layer"] == r["child"]), None)
            vals = c.get("values", {})
            if form is not None:
                f = Form(form["releases"][-1]["body"], REL_FIELDS[r["child"]])
                a2, iss = f.assess(attrs, utf8_sorted(dict(vals)), {}, slot is None)
                if errors(iss):
                    first = errors(iss)[0]
                    return stop("form_invalid", f"{at}.values.{first['field']}" if first.get("field") in vals else f"{at}.values")
                attrs = a2
            else:
                for k, v in vals.items():
                    fld = next((f for f in REL_FIELDS[r["child"]] if key_of(f) == k), None)
                    if fld is None:
                        return stop("form_invalid", f"{at}.values.{k}")
                    cv, why = check_value(fld, v)
                    if why:
                        return stop("form_invalid", f"{at}.values.{k}")
                    if cv:
                        attrs[fld["name"]] = cv
                    else:
                        attrs.pop(fld["name"], None)
            moved = slot is None or attrs != base
            out_children.append({"uid": uid, "relation": r["id"], **({"created": True} if slot is None else {}),
                                 "attrs": attrs, "changed": moved, "_before": base, "_slot": slot})
            if slot is None:
                new_attrs[uid] = (r["child"], attrs)
            elif moved:
                written[slot] = attrs
        if after != before:
            written[parent] = after
        for c in out_children:
            if c["_slot"] is None:
                written[10] = c["attrs"]
        changes = []
        names = changed_names(before, after)
        if names:
            changes.append({"uid": inp["uid"], "layer": "bina", "fields": names})
        for c in out_children:
            if c["changed"]:
                changes.append({"uid": c["uid"], "layer": layer_of.get(c["_slot"], next(
                    r["child"] for r in rels if r["id"] == c["relation"])),
                    **({"created": True} if c["_slot"] is None else {}), "fields": changed_names(c["_before"], c["attrs"])})
        # The snapshot: computed where every child's id is known (new records' given ids), else any.
        later = dict(state)
        for c in out_children:
            if c["_slot"] is not None:
                later[c["_slot"]] = c["attrs"]
        kids = children_of(later, parent, new_attrs)
        # A child's attributes hold the record's id, known only when the case runs: then any.
        snap = ANY if kids else snapshot(after)
        output = {"uid": inp["uid"], "version": 1, "attrs": after, "changed": bool(names),
                  "children": [{k: v for k, v in c.items() if not k.startswith("_")} for c in out_children],
                  "snapshot": snap, "revision": "$current"}
        if changes:
            output["changes"] = changes
            output["undo"] = "Form: Bina"
        if not output["children"]:
            del output["children"]
        return {"status": "completed", "output": output, "warnings": []}, written

    submit("bağla: var olan sayaç binaya bağlanır, değeri sayacın formuyla yazılır",
           [{"relation": "bina-sayac", "uid": "$uidOf:3", "values": {"no": "S-10"}}])
    submit("bağı kopar: sayacın alanı boşalır", [{"relation": "bina-sayac", "uid": "$uidOf:3", "unlink": True}],
           links=[(3, "bina", "$uid:o1")])
    submit("yeni kayıt: çağıranın kimliğiyle, sayacın formuyla", [{"relation": "bina-sayac", "uid": CHILD_NEW,
                                                                   "values": {"no": "S-99"}}], parent=2)

    def again(inp, written):
        child = {k: v for k, v in written.items() if k == 10}
        step = {"op": "execute", "input": inp, "result": {"status": "completed", "output": {
            "uid": inp["uid"], "version": 1, "attrs": dict(REL_OBJECTS[1][2]), "changed": False,
            "children": [{"uid": CHILD_NEW, "relation": "bina-sayac", "attrs": child[10], "changed": False}],
            "snapshot": ANY,
            "revision": "$current"}, "warnings": []},
            "expect": {"ids": REL_IDS + [10], "fields": {"10": {"attrs": child[10]}}, "revision": "same"}}
        return [step]
    submit("aynı kimlikle yeniden gönderilen yeni kayıt ikinci kez oluşmaz", [{"relation": "bina-sayac", "uid": CHILD_NEW,
                                                                              "values": {"no": "S-99"}}], parent=2,
           then=again)
    submit("çoktan çoğa: malikin binaları listesine bina eklenir", [{"relation": "bina-malik", "uid": "$uidOf:9"}],
           links=[(9, "binalar", "$uid:o2")])
    submit("çoktan çoğa: bağı kopar yalnız bu binayı çıkarır", [{"relation": "bina-malik", "uid": "$uidOf:9", "unlink": True}],
           links=[(9, "binalar", "$uid:o1; $uid:o2")])
    submit("formu olmayan alt katman: değerler alanların kuralıyla", [{"relation": "bina-abone", "uid": "$uidOf:8",
                                                                       "values": {"ad": "Ayşe Kaya"}}])
    submit("kayıt ve alt kayıtları birlikte: değer, bağ, kopma ve yeni kayıt",
           [{"relation": "bina-sayac", "uid": "$uidOf:3", "unlink": True},
            {"relation": "bina-sayac", "uid": "$uidOf:4", "values": {"no": "S-20"}},
            {"relation": "bina-abone", "uid": CHILD_NEW2, "values": {"ad": "Yeni abone"}}],
           values={"Ad": "A blok"}, links=[(3, "bina", "$uid:o1")])
    submit("plan yazmaz, ne yazılacağını söyler", [{"relation": "bina-sayac", "uid": "$uidOf:3", "values": {"no": "S-10"}}],
           values={"Ad": "A2"}, op="plan")

    def undone(inp, written):
        return [{"op": "undo", "expect": {"ids": REL_IDS, "fields": {"1": {"attrs": dict(REL_OBJECTS[0][2])},
                                                                      "3": {"attrs": dict(REL_OBJECTS[2][2])}},
                                          "canRedo": True}}]
    submit("geri alma kaydı ve alt kayıtlarını birlikte geri alır",
           [{"relation": "bina-sayac", "uid": "$uidOf:3", "values": {"no": "S-10"}}], values={"Ad": "A3"}, then=undone)
    submit("olmayan ilişki", [{"relation": "yok", "uid": "$uidOf:3"}])
    submit("üst katmanı formun katmanı olmayan ilişki", [{"relation": "sayac-vana", "uid": "$uidOf:6"}])
    submit("alanı alt katmanda olmayan ilişki", [{"relation": "bina-vana", "uid": "$uidOf:6"}])
    submit("ilişkinin alt katmanında olmayan nesne", [{"relation": "bina-sayac", "uid": "$uidOf:6"}])
    submit("geçersiz kimlik", [{"relation": "bina-sayac", "uid": "S-1"}])
    submit("aynı alt kayıt iki kez", [{"relation": "bina-sayac", "uid": "$uidOf:3"}, {"relation": "bina-sayac", "uid": "$uidOf:3"}])
    submit("olmayan nesnenin bağı koparılmaz", [{"relation": "bina-sayac", "uid": CHILD_NEW, "unlink": True}])
    submit("alt kaydın formu reddedince hiçbiri yazılmaz", [{"relation": "bina-sayac", "uid": "$uidOf:3", "values": {"no": "S"}}],
           values={"Ad": "A4"})
    submit("yeni alt kaydın zorunlu alanı boşsa reddedilir", [{"relation": "bina-sayac", "uid": CHILD_NEW}])
    submit("formu olmayan alt katmanın olmayan alanı", [{"relation": "bina-abone", "uid": "$uidOf:8", "values": {"yas": "3"}}])
    submit("kilitli alt katman", [{"relation": "bina-sayac", "uid": "$uidOf:3"}], locked=("sayac",))
    return cases


def commands():
    d = ROOT / "fixtures/commands/v1"
    look_v, look_s, look_set, look_fill = look_command_cases()
    return {
        d / "cad.forms.edit.json": command_file(
            "cad.forms.edit", "Formlar",
            "ADR 0401 §9: form oluşturma (katmanın alanlarından ya da verilen gövdeyle), taslağı kaydetme (sürümle), "
            "yeniden adlandırma, silme, yayınlama (ön denetimle, en çok 50 yayın) ve kuralları uygulama; proje ayarıdır, "
            "geri alma adımı değildir.", edit_cases()),
        d / "cad.forms.validate.json": command_file(
            "cad.forms.validate", "Formu denetle",
            "ADR 0401 §9: formun ön denetimi ve bir kaydın öğeleri ve sorunları, değerler yazılmış gibi; hiçbir şey yazılmaz.",
            validate_cases() + look_v + tree_validate_cases()),
        d / "cad.forms.submit.json": command_file(
            "cad.forms.submit", "Formla kaydet",
            "ADR 0401 §9: kaydın formla yazılması: değerler alan anahtarlarıyla, formun ve alanların kurallarıyla; bütün "
            "kayıt ya da hiçbiri, tek geri alma adımı.", submit_cases() + look_s),
        d / "cad.entities.set.forms.json": command_file(
            "cad.entities.set", "Öznitelikler ve formun kuralları",
            "ADR 0401 §8, ADR 0402 §4: katmanın kurallarını uyguladığı formun son yayını, öznitelik yazmasının profilidir; "
            "belge yazılan kaydın hesaplarını aynı adımda yeniler.", set_cases() + look_set),
        d / "cad.forms.fill.json": command_file(
            "cad.forms.fill", "Formla toplu yaz",
            "ADR 0403 §5: aynı değerler birden çok kayda formla yazılır; her kayıt kendi değerleri ve şekliyle denetlenir ve "
            "tamamlanır, biri geçmezse hiçbiri yazılmaz.", fill_cases() + look_fill),
        d / "cad.forms.audit.json": command_file(
            "cad.forms.audit", "Kayıtları formla denetle",
            "ADR 0406 §1: formun katmanındaki bütün kayıtlar formun kurallarıyla denetlenir; hiçbir şey yazılmaz.",
            audit_cases() + tree_audit_cases()),
        d / "cad.relations.edit.json": command_file(
            "cad.relations.edit", "İlişkiler",
            "ADR 0405 §3: projenin ilişkileri; oluşturma, değiştirme, silme. Proje ayarıdır, geri alma adımı değildir.",
            relations_edit_cases()),
        d / "cad.entities.delete.relations.json": command_file(
            "cad.entities.delete", "Silme ve ilişkiler",
            "ADR 0405 §2: üst nesne silinince alt kayıtlar ilişkinin politikasıyla aynı adımda izler (unlink, cascade, keep); "
            "restrict alt kaydı olan üst nesneyi silmez; kilitli katmandaki alt kaydı değiştirecek silme reddedilir (ADR 0440 §1); "
            "plan ve cevap her ilişkinin izlediği alt kayıtları sayar.", delete_relation_cases()),
        d / "cad.forms.submit.children.json": command_file(
            "cad.forms.submit", "Formla kaydet ve alt kayıtlar",
            "ADR 0440 §2: kayıt alt kayıtlarıyla tek adımda yazılır: ilişkisiyle bağlanan, bağı koparılan, değişen ya da "
            "çağıranın verdiği kimlikle oluşan; alt katmanın formu varsa onunla; biri reddedilirse hiçbiri yazılmaz. Cevap "
            "yazılan kayıtları, anlık görüntüyü ve geri alma adımını söyler.", submit_children_cases()),
        d / "cad.polygon.create.forms.json": {**command_file(
            "cad.polygon.create", "Alan çiz ve formun kuralları",
            "ADR 0402 §4: kurallarını uygulayan formun katmanına çizilen alan, aynı adımda formun varsayılanlarını ve "
            "hesaplarını alır; zorunlu alanın eksiği reddedilmez.", create_cases()), "setup": setup([CALC_ENFORCED], CALC_FIELDS, CALC_ENTITIES)},
    }


def main():
    files = outputs()
    stale = []
    for path, doc in files.items():
        text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
        if "--check" in sys.argv:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                stale.append(path)
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
    if stale:
        for p in stale:
            print(f"{p.relative_to(ROOT)} güncel değil: python3 {sys.argv[0]} ile yeniden yazın ve farkı okuyun.")
        sys.exit(1)
    verb = "güncel" if "--check" in sys.argv else "yazıldı"
    for path, doc in files.items():
        n = sum(len(v) for v in doc.values() if isinstance(v, list))
        print(f"{path.relative_to(ROOT)}: {n} durum, {verb}.")


if __name__ == "__main__":
    main()
