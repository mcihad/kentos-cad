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

# uzunluk(metin): the text's UTF-16 units, as JavaScript counts them.
el.FUNCTIONS["UZUNLUK"] = (1, 1, lambda args: None if args[0] is None else float(el.units(el.text(args[0]))))

JS_SPACE = kc.JS_SPACE
MAX_INTEGER = 9007199254740991
MAX_DIGITS = 30
MOST_NAME = 64


# ── A layer field's values (docs/adr/0199 §1) ───────────────────────────


def key_of(f):
    return f.get("id", f["name"])


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
        if ("min" in f and x < Fraction(f["min"])) or ("max" in f and x > Fraction(f["max"])):
            return None, "range"
    return value, None


# ── Rules ───────────────────────────────────────────────────────────────


def parse(src):
    """The rule's tree, or None when it does not compile."""
    try:
        return el.Parser(el.tokenize(src)).parse()
    except el.Invalid:
        return None


def names(tree):
    """The names a tree reads, in order, each once."""
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


def scope(fields, attrs):
    """The record as a rule reads it: each layer field by its kind, other attributes as text."""
    obj = dict(attrs)
    for f in fields:
        v = attrs.get(f["name"])
        if f["kind"] == "boolean":
            obj[f["name"]] = True if v == "true" else False if v == "false" else None
        elif f["kind"] in ("integer", "decimal"):
            obj[f["name"]] = None if v is None else el.read_number(v)
    return obj


def holds(src, fields, attrs):
    return el.truthy(el.value(parse(src), scope(fields, attrs)))


# ── The form ────────────────────────────────────────────────────────────

FITS = {
    "text": {"text", "textArea", "list", "radio", "segmented"},
    "integer": {"number", "list", "radio", "segmented"},
    "decimal": {"number", "list", "radio", "segmented"},
    "date": {"date"},
    "boolean": {"check", "switch", "radio", "segmented"},
}


def widget_of(item, f):
    if "widget" in item:
        return item["widget"]
    if f.get("values"):
        return "list"
    return {"integer": "number", "decimal": "number", "date": "date", "boolean": "check"}.get(
        f["kind"], "textArea" if f.get("length", 0) > 200 else "text")


def issue(code, error, item=None, field=None, rule=None):
    out = {"code": code, "severity": "error" if error else "warning"}
    for k, v in (("item", item), ("field", field), ("rule", rule)):
        if v is not None:
            out[k] = v
    return out


def preflight(body, fields):
    problem = kc.body_problem(body)
    if problem:
        return [issue("form_body_invalid", True)]
    issues, bound = [], []

    def rule(item, key, itself, at, src):
        tree = parse(src)
        if tree is None:
            issues.append(issue("form_rule_invalid", True, item, key, at))
            return
        for name in names(tree):
            if not any(f["name"] == name for f in fields):
                issues.append(issue("form_rule_unknown_name", False, item, key, at))
            elif name == itself:
                issues.append(issue("form_rule_reads_itself", False, item, key, at))

    def field(f, hideable):
        key = f["field"]
        if any(b[0] == key for b in bound):
            issues.append(issue("form_field_twice", True, f["id"], key))
        always = not hideable and "visibleWhen" not in f and not f.get("readOnly") and "readOnlyWhen" not in f
        bound.append((key, f["id"], always))
        layer = next((x for x in fields if key_of(x) == key), None)
        if layer is None:
            issues.append(issue("form_field_unknown", True, f["id"], key))
            return
        w = widget_of(f, layer)
        if w not in FITS[layer["kind"]]:
            issues.append(issue("form_widget_unfit", True, f["id"], key))
        elif w in ("list", "radio", "segmented") and not layer.get("values") and layer["kind"] != "boolean":
            issues.append(issue("form_widget_needs_values", True, f["id"], key))
        if "default" in f and check_value(layer, f["default"])[1]:
            issues.append(issue("form_default_invalid", True, f["id"], key, "default"))
        if f.get("readOnly") and f.get("required") and "default" not in f:
            issues.append(issue("form_required_read_only", True, f["id"], key))
        for at in ("visibleWhen", "requiredWhen", "readOnlyWhen"):
            if at in f:
                rule(f["id"], key, None if at == "requiredWhen" else layer["name"], at, f[at])
        for i, c in enumerate(f.get("checks", [])):
            rule(f["id"], key, None, f"checks/{i}", c["rule"])

    def items(xs, hideable):
        for x in xs:
            if x["type"] == "tab":
                items(x["items"], hideable)
            elif x["type"] == "group":
                if "visibleWhen" in x:
                    rule(x["id"], None, None, "visibleWhen", x["visibleWhen"])
                items(x["items"], hideable or "visibleWhen" in x)
            elif x["type"] == "field":
                field(x, hideable)

    items(body["items"], False)
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


class Form:
    """A form prepared over its layer's fields."""

    def __init__(self, body, fields):
        assert not errors(preflight(body, fields)), "a form with errors is not prepared"
        self.fields = fields
        self.nodes = []

        def walk(xs, parent):
            for x in xs:
                at = len(self.nodes)
                self.nodes.append((x, parent))
                walk(x.get("items", []), at)

        walk(body["items"], None)

    def layer(self, x):
        return next(f for f in self.fields if key_of(f) == x["field"])

    def bound(self):
        return [(x, p) for x, p in self.nodes if x["type"] == "field"]

    def states(self, attrs):
        shown, out = [], []
        for x, parent in self.nodes:
            rule = x.get("visibleWhen") if x["type"] in ("group", "field") else None
            here = (parent is None or shown[parent]) and (rule is None or holds(rule, self.fields, attrs))
            shown.append(here)
            state = {"id": x["id"], "shown": here}
            if x["type"] == "field":
                f = self.layer(x)
                if here and (x.get("required") or f.get("required")
                             or ("requiredWhen" in x and holds(x["requiredWhen"], self.fields, attrs))):
                    state["required"] = True
                if x.get("readOnly") or ("readOnlyWhen" in x and holds(x["readOnlyWhen"], self.fields, attrs)):
                    state["readOnly"] = True
            out.append(state)
        return out

    def validate(self, attrs):
        issues = []
        for (x, _), state in zip(self.nodes, self.states(attrs)):
            if x["type"] != "field" or not state["shown"]:
                continue
            f = self.layer(x)
            value = attrs.get(f["name"], "").strip(JS_SPACE)
            if not value:
                if state.get("required"):
                    issues.append(issue("form_required", True, x["id"], key_of(f)))
                continue
            if check_value(f, value)[1]:
                issues.append(issue("form_value_invalid", True, x["id"], key_of(f)))
                continue
            for i, c in enumerate(x.get("checks", [])):
                if not holds(c["rule"], self.fields, attrs):
                    issues.append(issue("form_check_failed", True, x["id"], key_of(f), f"checks/{i}"))
        return issues

    def with_defaults(self, attrs):
        out = dict(attrs)
        for x, _ in self.bound():
            f = self.layer(x)
            if "default" in x and not out.get(f["name"], "").strip(JS_SPACE):
                v, why = check_value(f, x["default"])
                if not why and v:
                    out[f["name"]] = v
        return out

    def locked(self, changed, after):
        issues = []
        for (x, _), state in zip(self.nodes, self.states(after)):
            if x["type"] != "field" or key_of(self.layer(x)) not in changed:
                continue
            if not state["shown"]:
                issues.append(issue("form_hidden", True, x["id"], x["field"]))
            elif state.get("readOnly"):
                issues.append(issue("form_read_only", True, x["id"], x["field"]))
        return issues

    def assess(self, before, values):
        after, issues, changed = dict(before), [], []
        on_form = [x["field"] for x, _ in self.bound()]
        for key, text in values:
            f = next((f for f in self.fields if key_of(f) == key), None)
            if f is None:
                issues.append(issue("form_field_unknown", True, None, key))
                continue
            if key not in on_form:
                issues.append(issue("form_not_on_form", True, None, key))
                continue
            v, why = check_value(f, text)
            if why:
                item = next(x["id"] for x, _ in self.bound() if x["field"] == key)
                issues.append(issue("form_value_invalid", True, item, key))
                continue
            if v:
                after[f["name"]] = v
            else:
                after.pop(f["name"], None)
            if before.get(f["name"]) != after.get(f["name"]):
                changed.append(key)
        issues += self.locked(changed, after)
        said = [i.get("field") for i in issues]
        issues += [i for i in self.validate(after) if i.get("field") not in said]
        return after, issues

    def enforce(self, before, after):
        issues, changed = [], []
        on_form = [x["field"] for x, _ in self.bound()]
        for f in self.fields:
            if before.get(f["name"]) == after.get(f["name"]):
                continue
            if key_of(f) in on_form:
                changed.append(key_of(f))
            else:
                issues.append(issue("form_not_on_form", True, None, key_of(f)))
        return issues + self.locked(changed, after) + self.validate(after)


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
                        "items": form.states(attrs), "issues": form.validate(attrs)})

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
        "note": "ADR 0401'in form çekirdeği, KentOS kodu olmadan yazılmış başvurudan (scripts/fixtures/forms_cases.py): "
                "ön denetim, kaydın öğeleri ve sorunları, formla yazma, yazma profili, varsayılanlar ve katmandan form. "
                "Sorunlar kodu, ağırlığı, öğesi, alanı ve kuralıyla karşılaştırılır; cümleleri çekirdeğindir.",
        "preflight": pre,
        "records": records,
        "assess": assess,
        "enforce": enforce,
        "defaults": defaults,
        "generate": gen,
    }


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


def setup(forms=None):
    settings = dict(SETTINGS)
    if forms:
        settings["forms"] = forms
    return {"format": "kentos.document", "version": 1, "name": "Formlar", "settings": settings, "origin": P(E0, N0),
            "layers": LAYERS, "activeLayer": "bina", "entities": ENTITIES, "styles": {"items": [], "categories": []}}


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


class Project:
    """The forms the edit cases go through, by the ADR's rule."""

    def __init__(self, forms=None):
        self.forms = copy.deepcopy(forms or [])

    def ids(self):
        return [f["id"] for f in self.forms]

    def find(self, i):
        return next(f for f in self.forms if f["id"] == i)

    def step(self, op, **more):
        old = copy.deepcopy(self.forms)
        warnings, kept = [], None
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
            if op["body"]["items"] != f["draft"]["items"]:
                f["draft"] = {"revision": f["draft"]["revision"] + 1, "items": copy.deepcopy(op["body"]["items"])}
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
        changed = self.forms != old
        output = {"forms": [summary(f) for f in self.forms], "changed": changed, "revision": "$current",
                  "issues": shown_issues(preflight(kept["draft"], FIELDS_OF[kept["layer"]])) if kept else []}
        if kept is not None:
            output["form"] = form_json(kept)
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

    return cases


def validate_cases():
    cases = []

    def run(inp, result, forms=None):
        return {"op": "execute", "input": inp, "result": result,
                "expect": {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}}

    def answer(form, body, version, uid_attrs=None, values=(), defaults=False, fields=FIELDS):
        issues = preflight(body, fields)
        out = {"issues": [], "valid": True, "items": []}
        if version is not None:
            out["version"] = version
        if uid_attrs is not None and not errors(issues):
            f = Form(body, fields)
            before = f.with_defaults(uid_attrs) if defaults else dict(uid_attrs)
            after, found = f.assess(before, utf8_sorted(dict(values)))
            out["items"] = f.states(after)
            issues = issues + found
            out["attrs"] = after
        out["issues"] = shown_issues(issues)
        out["valid"] = not errors(issues)
        return {"status": "completed", "output": out, "warnings": []}

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
    return cases


def submit_cases():
    cases = []

    def write(inp, before, values=(), defaults=False, form=PUBLISHED, body=BODY, version=1, undo=0, slot=1):
        f = Form(body, FIELDS)
        start = f.with_defaults(before) if defaults else dict(before)
        after, issues = f.assess(start, utf8_sorted(dict(values)))
        assert not issues, issues
        changed = after != before
        out = {"uid": f"$uidOf:{slot}", "attrs": after, "changed": changed, "revision": "$current"}
        if version is not None:
            out["version"] = version
        expect = {"ids": IDS, "fields": {str(slot): {"attrs": after}}, "canUndo": undo + changed > 0, "canRedo": False}
        expect.update({"dirty": True, "revision": "changed"} if changed else {"revision": "same"})
        return {"op": "execute", "input": inp, "result": {"status": "completed", "output": out, "warnings": []}, "expect": expect}, after

    def refused(inp, result, before, slot=1, undo=0, dirty=False):
        expect = {"ids": IDS, "fields": {str(slot): {"attrs": before}}, "canUndo": undo > 0, "canRedo": False, "revision": "same"}
        if not dirty:
            expect["dirty"] = False
        return {"op": "execute", "input": inp, "result": result, "expect": expect}

    def invalid(inp, before, values, slot=1, undo=0, dirty=False, body=BODY):
        _, issues = Form(body, FIELDS).assess(dict(before), utf8_sorted(dict(values)))
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


def commands():
    d = ROOT / "fixtures/commands/v1"
    return {
        d / "cad.forms.edit.json": command_file(
            "cad.forms.edit", "Formlar",
            "ADR 0401 §9: form oluşturma (katmanın alanlarından ya da verilen gövdeyle), taslağı kaydetme (sürümle), "
            "yeniden adlandırma, silme, yayınlama (ön denetimle, en çok 50 yayın) ve kuralları uygulama; proje ayarıdır, "
            "geri alma adımı değildir.", edit_cases()),
        d / "cad.forms.validate.json": command_file(
            "cad.forms.validate", "Formu denetle",
            "ADR 0401 §9: formun ön denetimi ve bir kaydın öğeleri ve sorunları, değerler yazılmış gibi; hiçbir şey yazılmaz.",
            validate_cases()),
        d / "cad.forms.submit.json": command_file(
            "cad.forms.submit", "Formla kaydet",
            "ADR 0401 §9: kaydın formla yazılması: değerler alan anahtarlarıyla, formun ve alanların kurallarıyla; bütün "
            "kayıt ya da hiçbiri, tek geri alma adımı.", submit_cases()),
        d / "cad.entities.set.forms.json": command_file(
            "cad.entities.set", "Öznitelikler ve formun kuralları",
            "ADR 0401 §8: katmanın kurallarını uyguladığı formun son yayını, öznitelik yazmasının profilidir.", set_cases()),
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
