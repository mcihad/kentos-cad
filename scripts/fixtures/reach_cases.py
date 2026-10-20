"""Writes fixtures/reach/v1/cases.json: the shared cases of the one resolver of what a caller reads and takes
out (docs/adr/0550 §1), their expectations built here without KentOS code from the ADR's rules:

- a *layer* is bound by its forms: a layer with a published form none of which is withdrawn is **live** (a Veri
  girişi reads the fields the live forms' latest releases show, and the records one of them reaches); a layer with
  published forms that are all withdrawn is **closed** (it reads nothing); a layer no published form binds is free;
- a form's *scope* (docs/adr/0539) is no scope (every record), `Alan = @kullanici_eposta`, `Alan = 'metin'`,
  `Alan > sayı`; one that names a field the layer does not have reaches nothing; an empty value reaches nothing;
- a field is **marked** when a form item for it says `redact`, in any form's draft or latest release (an earlier
  release's mark does not count once a later release drops it; the layer field's name is the attribute, the item
  names its key);
- a field's level: hidden (a Veri girişi on a closed layer, or one the live forms do not show), redacted (marked),
  open; a reader and a form designer read everything;
- what leaves (a file, a copy, a count, a statistic, a chart, an agent's answer) is the open fields; a whole copy
  of the project keeps the marked ones for a form designer; a copy stored as it was written is given only when no
  field is marked or the caller designs forms.

    python3 scripts/fixtures/reach_cases.py           # writes the file
    python3 scripts/fixtures/reach_cases.py --check   # compares it with the one on disk
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/reach/v1/cases.json"
EMAIL = "ali@k.test"

BINA = [{"name": "Sorumlu", "kind": "text"}, {"name": "Malik", "kind": "text"}, {"name": "Kat", "kind": "integer"},
        {"name": "Not", "kind": "text"}]
YOL = [{"name": "Ad", "kind": "text"}, {"name": "Kod", "kind": "text"}]


def form(fid, layer, items, scope=None, published=True, withdrawn=False, draft_items=None, later_items=None):
    f = {"id": fid, "layer": layer, "items": items, "published": published, "withdrawn": withdrawn}
    if later_items is not None:
        f["laterItems"] = later_items
    if scope is not None:
        f["scope"] = scope
    if draft_items is not None:
        f["draftItems"] = draft_items
    return f


def item(field, redact=False):
    return {"field": field, "redact": redact}


def row(layer, **attrs):
    return {"layer": layer, "attrs": {k: str(v) for k, v in attrs.items()}}


def holds(scope, attrs, names):
    """Whether `scope` holds for a record: the three shapes the cases use. None: no answer (does not reach)."""
    if scope is None:
        return True
    if scope == "Yok = 1":
        return False
    field, op, rest = scope.split(" ", 2)
    if field not in names:
        return False
    value = attrs.get(field)
    if value is None or value == "":
        return False
    if op == "=":
        want = EMAIL if rest == "@kullanici_eposta" else rest.strip("'")
        return value == want
    if op == ">":
        return float(value) > float(rest)
    raise ValueError(scope)


def latest(f):
    """The items of a form's latest release."""
    return f.get("laterItems", f["items"])


def expect(case, who):
    through, designs = {"through_forms": (True, False), "reader": (False, False), "whole": (False, True)}[who]
    fields = case["fields"]
    names = {layer: {f["name"] for f in fs} for layer, fs in fields.items()}

    def attr_of(layer, key):
        for f in fields[layer]:
            if f.get("id", f["name"]) == key:
                return f["name"]
        return key

    forms = case["forms"]
    published = [f for f in forms if f["published"]]
    layers = {f["layer"] for f in published}
    live = {l: [f for f in published if f["layer"] == l and not f["withdrawn"]] for l in layers}
    closed = {l for l in layers if not live[l]}
    shown = {l: {attr_of(l, i["field"]) for f in live[l] for i in latest(f) if attr_of(l, i["field"]) in names[l]}
             for l in layers if l not in closed}
    marked = {}
    for f in forms:
        for it in f.get("draftItems", f["items"]) + (latest(f) if f["published"] else []):
            if it["redact"]:
                marked.setdefault(f["layer"], set()).add(attr_of(f["layer"], it["field"]))

    def level(layer, attr):
        if through and (layer in closed or (layer in shown and attr not in shown[layer])):
            return "hidden"
        return "redacted" if attr in marked.get(layer, set()) else "open"

    def sees(layer, attrs):
        if not through:
            return True
        if layer in closed:
            return False
        if layer not in live:
            return True
        return any(holds(f.get("scope"), attrs, names[layer]) for f in live[layer])

    universe = {l: sorted(set(names[l]) | {k for r in case["rows"] if r["layer"] == l for k in r["attrs"]})
                for l in fields}
    levels = {l: {a: level(l, a) for a in universe[l]} for l in fields}

    def kept(layer, attrs, pred):
        return {k: v for k, v in sorted(attrs.items()) if pred(level(layer, k))}

    rows = []
    for r in case["rows"]:
        layer, attrs = r["layer"], r["attrs"]
        rows.append({
            "sees": sees(layer, attrs),
            "readable": kept(layer, attrs, lambda lv: lv != "hidden"),
            "exportable": kept(layer, attrs, lambda lv: lv == "open"),
            "copy": dict(sorted(attrs.items())) if designs else kept(layer, attrs, lambda lv: lv == "open"),
        })
    limited = set()
    if through:
        limited |= closed
        limited |= {l for l in live if not any(f.get("scope") is None for f in live[l])}
    return {
        "levels": levels,
        "rows": rows,
        "limited": sorted(limited),
        "narrowsRows": bool(limited),
        "hasRedacted": bool(marked),
        "wholeCopyOk": designs or not marked,
    }


def cases():
    out = []

    def case(name, forms, rows, fields=None):
        c = {"name": name, "email": EMAIL, "fields": fields or {"bina": BINA, "yol": YOL}, "forms": forms,
             "rows": rows}
        c["expect"] = {who: expect(c, who) for who in ("through_forms", "reader", "whole")}
        out.append(c)

    r1 = row("bina", Sorumlu=EMAIL, Malik="Ayşe", Kat=3, Not="n", Ek="x")
    r2 = row("bina", Sorumlu="ayse@k.test", Malik="Veli", Kat=7)
    r3 = row("yol", Ad="D-100", Kod="k")
    rows = [r1, r2, r3]
    case("no forms: everyone reads and takes out everything", [], rows)
    case("a live form shows its fields; a marked one stays on screen and does not leave",
         [form("f1", "bina", [item("Sorumlu"), item("Malik", True)])], rows)
    case("an unmarked live form: a Veri girişi reads its fields only, the others are whole",
         [form("f1", "bina", [item("Sorumlu"), item("Kat")])], rows)
    case("a scope by e-mail reaches the caller's records; a layer no form binds is free",
         [form("f1", "bina", [item("Sorumlu"), item("Kat")], scope="Sorumlu = @kullanici_eposta")], rows)
    case("two live forms: either one's scope reaches",
         [form("f1", "bina", [item("Sorumlu")], scope="Kat > 5"),
          form("f2", "bina", [item("Kat")], scope="Sorumlu = @kullanici_eposta")], rows)
    case("a live form without a scope reaches every record even beside a scoped one",
         [form("f1", "bina", [item("Sorumlu")], scope="Kat > 5"), form("f2", "bina", [item("Kat")])], rows)
    case("a scope that names no field of the layer reaches nothing",
         [form("f1", "bina", [item("Sorumlu")], scope="Yok = 1")], rows)
    case("a scope by a literal",
         [form("f1", "bina", [item("Sorumlu")], scope="Malik = 'Veli'")], rows)
    case("every form withdrawn: the layer is closed to a Veri girişi, whatever its scope was",
         [form("f1", "bina", [item("Sorumlu"), item("Malik", True)], scope="Kat > 5", withdrawn=True)], rows)
    case("a withdrawn form beside a live one: the live one binds",
         [form("f1", "bina", [item("Sorumlu")], withdrawn=True), form("f2", "bina", [item("Kat")], scope="Kat > 5")],
         rows)
    case("a form never published binds nothing but its marks count",
         [form("f1", "bina", [item("Sorumlu")], scope="Kat > 99", published=False, draft_items=[item("Malik", True)])],
         rows)
    case("a mark in the draft only counts too",
         [form("f1", "bina", [item("Sorumlu")], draft_items=[item("Sorumlu"), item("Kat", True)])], rows)
    case("an item names a layer field by its stable id; the attribute is stored under the name",
         [form("f1", "bina", [item("malik-1", True), item("Kat")])], rows,
         fields={"bina": [{"name": "Sorumlu", "kind": "text"}, {"name": "Malik", "id": "malik-1", "kind": "text"},
                          {"name": "Kat", "kind": "integer"}, {"name": "Not", "kind": "text"}],
                 "yol": YOL})
    case("a later release replaces the earlier one's shown fields and marks",
         [form("f1", "bina", [item("Sorumlu"), item("Malik", True)], later_items=[item("Kat"), item("Not", True)])],
         rows)
    case("two layers with their own forms and marks",
         [form("f1", "bina", [item("Sorumlu", True)], scope="Kat > 5"), form("f2", "yol", [item("Kod", True)])], rows)
    case("a record whose scope field is empty is not reached",
         [form("f1", "bina", [item("Sorumlu")], scope="Kat > 1")],
         [row("bina", Sorumlu="a"), row("bina", Sorumlu="b", Kat=2)])
    return out


def build():
    return {"version": 1, "cases": cases()}


def main():
    text = json.dumps(build(), indent=2, ensure_ascii=False) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            print("fixtures/reach/v1/cases.json is out of date; run scripts/fixtures/reach_cases.py")
            return 1
        print(f"ok: {len(build()['cases'])} cases")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    print(f"wrote {OUT.relative_to(ROOT)}: {len(build()['cases'])} cases")
    return 0


if __name__ == "__main__":
    sys.exit(main())
