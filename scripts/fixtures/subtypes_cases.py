"""Writes the shared cases of a layer's subtypes (Alt türler, docs/adr/0422) without KentOS code: the rule of the
subtypes themselves (§1) by `cad.layers.subtypes` v1 (§3), and what they do to the records (§2) by `cad.entities.set`
and `cad.polygon.create`:

- fixtures/commands/v1/cad.layers.subtypes.json
- fixtures/commands/v1/cad.entities.set.subtypes.json
- fixtures/commands/v1/cad.polygon.create.subtypes.json

    python3 scripts/fixtures/subtypes_cases.py           # writes the files
    python3 scripts/fixtures/subtypes_cases.py --check   # compares them with the ones on disk
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"

STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
E0, N0 = 487000, 4420000
SETTINGS = {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
            "workspace": "gis", "drawingFont": "barlow"}


def choice(code, label):
    return {"code": code, "label": label}


TUR = {"name": "Tür", "kind": "text", "values": [choice("konut", "Konut"), choice("sanayi", "Sanayi"), choice("depo", "Depo")]}
FIELDS = [
    TUR,
    {"name": "Malzeme", "kind": "text", "values": [choice("ahsap", "Ahşap"), choice("beton", "Betonarme"),
                                                   choice("celik", "Çelik"), choice("tas", "Taş")], "default": "ahsap"},
    {"id": "kat-sayisi", "name": "Kat", "kind": "integer"},
    {"name": "Not", "kind": "text"},
    {"name": "Etiket", "kind": "text", "multiple": True, "values": [choice("a", "A"), choice("b", "B")]},
]
FABRIKA_FIELDS = [{**TUR, "default": "sanayi"}] + copy.deepcopy(FIELDS[1:])
SUBTYPES = {"field": "Tür", "types": [
    {"code": "konut", "fields": [{"field": "kat-sayisi", "default": "2"}]},
    {"code": "sanayi", "label": "Sanayi binası", "fields": [
        {"field": "Malzeme", "values": ["beton", "celik"], "default": "celik", "required": True},
        {"field": "Not", "required": True}]},
    {"code": "depo", "fields": [{"field": "Malzeme", "values": ["beton", "tas"]}]},
]}


def layer(i, name, fields=None, kind="layer", children=None, locked=False, **more):
    out = {"id": i, "name": name, "type": kind, "visible": True, "locked": locked, "expanded": True, "style": STYLE,
           "children": children or []}
    if fields:
        out["fields"] = copy.deepcopy(fields)
    out.update(more)
    return out


def layers(with_subtypes=True):
    bina = layer("bina", "Bina", FIELDS)
    fab = layer("fabrika", "Fabrika", FABRIKA_FIELDS)
    if with_subtypes:
        bina["subtypes"] = copy.deepcopy(SUBTYPES)
        fab["subtypes"] = copy.deepcopy(SUBTYPES)
    return [bina, fab, layer("yol", "Yol"),
            layer("koruma", "Koruma alanı", kind="group", children=[layer("kilitli", "Sit alanı", FIELDS)]),
            layer("altlik", "OpenStreetMap", service={"kind": "xyz", "url": "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
                                                      "maxZoom": 19})]


def square(x):
    return [{"x": E0 + x, "y": N0}, {"x": E0 + x + 10, "y": N0}, {"x": E0 + x + 10, "y": N0 + 10}, {"x": E0 + x, "y": N0 + 10}]


def setup(with_subtypes=True, entities=()):
    return {"format": "kentos.document", "version": 1, "name": "Alt türler", "settings": SETTINGS, "origin": {"x": E0, "y": N0},
            "layers": layers(with_subtypes), "activeLayer": "bina", "entities": list(entities),
            "styles": {"items": [], "categories": []}}


# ---- The rule (§1), written from the ADR ----

def key(f):
    return f.get("id") or f["name"]


def label(f):
    return f.get("alias") or f["name"]


def holds_many(f):
    return bool(f.get("multiple")) and f["kind"] == "text"


def codes_of(f, value):
    if holds_many(f):
        return [c.strip() for c in value.split(";") if c.strip()]
    v = value.strip()
    return [v] if v else []


def bad_label(s):
    return len(s) > 64 or s.strip() != s or any(ord(c) < 32 or 127 <= ord(c) < 160 for c in s)


def rule_of(t, f):
    return next((r for r in t.get("fields", []) if r["field"] == key(f)), None)


def offers(f, t, code):
    r = rule_of(t, f) if t else None
    return r is None or r.get("values") is None or code in r["values"]


def problem(fields, st):
    sorter = next((f for f in fields if key(f) == st["field"]), None)
    if sorter is None:
        return "field", f"Katmanda “{st['field']}” alanı yok; alt tür alanı katmanın bir alanıdır."
    if not sorter.get("values") or holds_many(sorter):
        return "field", (f"“{label(sorter)}” alanının değer listesi yok ya da çok değerli; alt tür alanı tek değerli bir "
                         "liste alanıdır.")
    types = st["types"]
    if not types:
        return "types", "Alt türü olmayan alt tür tanımı olmaz; en az bir alt tür verin."
    if len(types) > 256:
        return "types", f"Bir katmanın en çok 256 alt türü olur; {len(types)} var."
    codes = [c["code"] for c in sorter["values"]]
    for i, t in enumerate(types):
        at = f"types/{i}"
        if t["code"] not in codes:
            return f"{at}/code", f"“{t['code']}”, “{label(sorter)}” alanının listesinde yok; alt tür listenin bir değeridir."
        if any(o["code"] == t["code"] for o in types[:i]):
            return f"{at}/code", f"“{t['code']}” alt türü iki kez var."
        if bad_label(t.get("label", "")):
            return f"{at}/label", f"“{t['label']}” bir alt tür adı olamaz: en çok 64 karakter, başında ve sonunda boşluk olmadan."
        rules = t.get("fields", [])
        for j, r in enumerate(rules):
            at2 = f"{at}/fields/{j}"
            f = next((x for x in fields if key(x) == r["field"]), None)
            if f is None:
                return f"{at2}/field", f"Katmanda “{r['field']}” alanı yok."
            if key(f) == key(sorter):
                return f"{at2}/field", f"Alt tür, alt tür alanının (“{label(f)}”) kendisini değiştiremez."
            if any(o["field"] == r["field"] for o in rules[:j]):
                return f"{at2}/field", f"“{label(f)}” alanı “{t['code']}” alt türünde iki kez var."
            if "values" in r:
                if not f.get("values"):
                    return (f"{at2}/values", f"“{label(f)}” alanının değer listesi yok; alt tür yalnız listeli alanın "
                                             "değerlerini daraltır.")
                if not r["values"]:
                    return (f"{at2}/values", f"“{label(f)}” alanına “{t['code']}” alt türünde hiçbir değer bırakılmamış; en az "
                                             "bir değer seçin.")
                listed = [c["code"] for c in f["values"]]
                for k, c in enumerate(r["values"]):
                    if c not in listed:
                        return f"{at2}/values/{k}", f"“{c}”, “{label(f)}” alanının listesinde yok."
                    if c in r["values"][:k]:
                        return f"{at2}/values/{k}", f"“{c}” değeri iki kez var."
            if "default" in r:
                d = r["default"]
                listed = [c["code"] for c in f.get("values") or []]
                if d == "" or (f.get("values") and any(c not in listed for c in codes_of(f, d))):
                    # Only these two of check_value's refusals are reached here: empty, and a code off the list.
                    if d == "":
                        return (f"{at2}/default", f"“{label(f)}” alanının “{t['code']}” alt türündeki varsayılanı alanın "
                                                  "yazımıyla verilmeli.")
                    raise AssertionError("a default off the field's list is check_value's refusal; not reached here")
                if f.get("values") and any(not offers(f, t, c) for c in codes_of(f, d)):
                    return (f"{at2}/default", f"“{label(f)}” alanının varsayılanı “{t['code']}” alt türünün değerlerinden "
                                              "değil.")
    return None


def sub_of(fields, st, attrs):
    if not st:
        return None
    sorter = next(f for f in fields if key(f) == st["field"])
    v = attrs.get(sorter["name"])
    if v is None:
        return None
    return next((t for t in st["types"] if t["code"] == v.strip()), None)


def sub_label(fields, st, t):
    if t.get("label"):
        return t["label"]
    sorter = next(f for f in fields if key(f) == st["field"])
    return next(c["label"] for c in sorter["values"] if c["code"] == t["code"])


def seen_required(f, t):
    r = rule_of(t, f) if t else None
    return bool(f.get("required")) or bool(r and r.get("required"))


def seen_default(f, t):
    r = rule_of(t, f) if t else None
    if r and "default" in r:
        return r["default"]
    return f.get("default")


def new_defaults(fields, st, out):
    """A new record's empty fields: the subtype field by its own default, then each by its subtype's, else its own,
    when the subtype offers it (§2)."""
    out = dict(out)
    if st:
        sorter = next(f for f in fields if key(f) == st["field"])
        if sorter.get("default"):
            out.setdefault(sorter["name"], sorter["default"])
    t = sub_of(fields, st, out)
    for f in fields:
        d = seen_default(f, t)
        if not d:
            continue
        r = rule_of(t, f) if t else None
        narrowed = bool(f.get("values")) and r is not None and r.get("values") is not None
        if narrowed and not all(offers(f, t, c) for c in codes_of(f, d)):
            continue
        out.setdefault(f["name"], d)
    return out


class Refused(Exception):
    def __init__(self, code, message, path):
        super().__init__(message)
        self.error = {"code": code, "message": message, "path": path}


def fit(fields, st, before, after, at="attrs"):
    """The record against its subtype (§2): a value written anew the subtype does not offer, a required one of the
    subtype written empty, refused; what it kept, a warning."""
    t = sub_of(fields, st, after)
    if t is None:
        return None
    words = sub_label(fields, st, t)

    def anew(n):
        return before is None or before.get(n) != after.get(n)
    kept = []
    for f in fields:
        r = rule_of(t, f)
        if not r or r.get("values") is None or f["name"] not in after:
            continue
        bad = [c for c in codes_of(f, after[f["name"]]) if c not in r["values"]]
        if not bad:
            continue
        if anew(f["name"]):
            raise Refused("invalid_attribute",
                          f"“{label(f)}” alanına “{', '.join(bad)}”, “{words}” alt türünde verilemez: bu alt tür bu değeri "
                          "sunmaz. Alt türün değerlerinden birini seçin.", f"{at}.{f['name']}")
        kept.append(label(f))
    for f in fields:
        empty = not after.get(f["name"], "").strip()
        if seen_required(f, t) and not f.get("required") and empty:
            if before is not None and anew(f["name"]):
                raise Refused("attribute_required", f"“{label(f)}” alanı “{words}” alt türünde zorunlu; boş bırakılamaz.",
                              f"{at}.{f['name']}")
            if before is not None:
                kept.append(label(f))
    if not kept:
        return None
    return {"code": "subtype_misfit", "message": f"“{words}” alt türüne uymayan değerler kaldı: {', '.join(kept)}. Kaydı "
                                                 "formla açıp düzeltin.", "path": at}


# ---- cad.layers.subtypes ----

def tree(st_of):
    def put(nodes):
        for n in nodes:
            if st_of.get(n["id"]) is not None:
                n["subtypes"] = st_of[n["id"]]
            put(n["children"])
    out = layers(False)
    put(out)
    return out


def subtypes_file():
    cases = []
    none = {}

    def ok(name, inp, now=None, undo=False):
        now = dict(now or none)
        fields = FIELDS
        st = inp.get("subtypes")
        assert st is None or problem(fields, st) is None, problem(fields, st)
        changed = now.get(inp["layer"]) != st
        after = {**now, inp["layer"]: st}
        expect = {"canUndo": changed or undo, "canRedo": False, "revision": "changed" if changed else "same",
                  "layers": tree(after)}
        if changed:
            expect["dirty"] = True
        return {"op": "execute", "input": inp, "result": {"status": "completed", "output": {
            "layer": inp["layer"], "changed": changed, "revision": "$current"}, "warnings": []}, "expect": expect}, after

    def refused(name, inp, code, message, path, status="failed"):
        err = {"code": code, "message": message, "path": path}
        if code == "revision_conflict":
            err["retryable"] = True
        cases.append({"name": name, "setup": setup(False), "steps": [
            {"op": "execute", "input": inp, "result": {"status": status, "error": err},
             "expect": {"canUndo": False, "canRedo": False, "dirty": False, "revision": "same", "layers": tree({})}}]})

    def bad(name, st, path, message, layer_id="bina"):
        assert problem(FIELDS, st) == (path, message), (problem(FIELDS, st), path)
        refused(name, {"layer": layer_id, "subtypes": st}, "invalid_subtypes", message, f"subtypes/{path}")

    one, after = ok("", {"layer": "bina", "subtypes": SUBTYPES})
    undo = {"op": "undo", "expect": {"canUndo": False, "canRedo": True, "layers": tree({})}}
    redo = {"op": "redo", "expect": {"canUndo": True, "canRedo": False, "layers": tree(after)}}
    cases.append({"name": "üç alt tür yazılır; adımı “Alt türler”, geri alınır ve yinelenir", "setup": setup(False),
                  "steps": [one, undo, redo]})
    two, _ = ok("", {"layer": "bina", "subtypes": SUBTYPES}, now=after, undo=True)
    cases.append({"name": "aynı alt türler yeniden: değişmez", "setup": setup(False), "steps": [one, two]})
    gone, _ = ok("", {"layer": "bina"}, now=after, undo=True)
    cases.append({"name": "alt türler verilmezse kaldırılır", "setup": setup(False), "steps": [one, gone]})
    three, _ = ok("", {"layer": "yol"})
    cases.append({"name": "alt türü olmayan katmana boş: değişmez", "setup": setup(False), "steps": [three]})
    only = {"field": "Tür", "types": [{"code": "sanayi"}]}
    four, _ = ok("", {"layer": "bina", "subtypes": only})
    cases.append({"name": "kuralsız tek alt tür", "setup": setup(False), "steps": [four]})
    plan_node = copy.deepcopy(layers(False)[0])
    plan_node["subtypes"] = SUBTYPES
    cases.append({"name": "plan katmanı yazılacak hâliyle gösterir, hiçbir şey yazmaz", "setup": setup(False), "steps": [
        {"op": "plan", "input": {"layer": "bina", "subtypes": SUBTYPES}, "result": {"status": "completed", "output": {
            "node": plan_node, "changed": True, "revision": "$current"}, "warnings": []},
         "expect": {"canUndo": False, "dirty": False, "revision": "same", "layers": tree({})}},
        {"op": "validate", "input": {"layer": "bina", "subtypes": SUBTYPES},
         "result": {"status": "completed", "output": None, "warnings": []}, "expect": {"revision": "same"}}]})
    keyed = {"field": "Tür", "types": [{"code": "konut", "label": "Konut binası",
                                        "fields": [{"field": "kat-sayisi", "required": True}]}]}
    five, _ = ok("", {"layer": "kilitli", "subtypes": keyed})
    cases.append({"name": "kilitli grubun katmanı da alır; alan kimliğiyle anılır", "setup": setup(False), "steps": [five]})

    refused("geçersiz beklenen sürüm", {"layer": "bina", "subtypes": SUBTYPES, "expectedRevision": "on iki"},
            "invalid_revision", "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. "
                                "Sürümü belgeden ya da komutun planından alın.", "expectedRevision")
    first, _ = ok("", {"layer": "fabrika", "subtypes": only})
    conflict = {"op": "execute", "input": {"layer": "bina", "subtypes": SUBTYPES, "expectedRevision": "$once"},
                "result": {"status": "conflict", "error": {
                    "code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey "
                    "yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision",
                    "revision": "$current"}},
                "expect": {"revision": "same", "layers": tree({"fabrika": only})}}
    cases.append({"name": "sürüm çakışması: plan edilen sürümden sonra çizim değişmişse hiçbir şey yazılmaz",
                  "setup": setup(False), "steps": [{"op": "captureRevision", "as": "once"}, first, conflict]})
    refused("çizimde olmayan katman", {"layer": "yok", "subtypes": SUBTYPES}, "layer_not_found",
            "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layer")
    refused("grup", {"layer": "koruma", "subtypes": SUBTYPES}, "not_a_layer",
            "“Koruma alanı” bir katman grubu; alt türler yalnız katmanın olur. Grubun bir katmanını verin.", "layer")
    refused("servis katmanı", {"layer": "altlik", "subtypes": SUBTYPES}, "service_layer",
            "“OpenStreetMap” servisten çizilir ve nesne tutmaz; alt türü yoktur.", "layer")
    refused("katmandan önce sürüm", {"layer": "yok", "subtypes": SUBTYPES, "expectedRevision": "x"}, "invalid_revision",
            "Beklenen sürüm “x” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da "
            "komutun planından alın.", "expectedRevision")
    bad("olmayan alt tür alanı", {"field": "Cins", "types": [{"code": "a"}]}, "field",
        "Katmanda “Cins” alanı yok; alt tür alanı katmanın bir alanıdır.")
    bad("listesiz alan alt tür alanı olamaz", {"field": "Not", "types": [{"code": "a"}]}, "field",
        "“Not” alanının değer listesi yok ya da çok değerli; alt tür alanı tek değerli bir liste alanıdır.")
    bad("çok değerli alan alt tür alanı olamaz", {"field": "Etiket", "types": [{"code": "a"}]}, "field",
        "“Etiket” alanının değer listesi yok ya da çok değerli; alt tür alanı tek değerli bir liste alanıdır.")
    bad("alt türsüz tanım", {"field": "Tür", "types": []}, "types",
        "Alt türü olmayan alt tür tanımı olmaz; en az bir alt tür verin.")
    bad("listede olmayan kod", {"field": "Tür", "types": [{"code": "konut"}, {"code": "ofis"}]}, "types/1/code",
        "“ofis”, “Tür” alanının listesinde yok; alt tür listenin bir değeridir.")
    bad("iki kez aynı alt tür", {"field": "Tür", "types": [{"code": "konut"}, {"code": "konut"}]}, "types/1/code",
        "“konut” alt türü iki kez var.")
    bad("boşluklu ad", {"field": "Tür", "types": [{"code": "konut", "label": " Konut"}]}, "types/0/label",
        "“ Konut” bir alt tür adı olamaz: en çok 64 karakter, başında ve sonunda boşluk olmadan.")
    bad("olmayan alanın kuralı", {"field": "Tür", "types": [{"code": "konut", "fields": [{"field": "Renk", "required": True}]}]},
        "types/0/fields/0/field", "Katmanda “Renk” alanı yok.")
    bad("alt tür alanının kendisi", {"field": "Tür", "types": [{"code": "konut", "fields": [{"field": "Tür", "required": True}]}]},
        "types/0/fields/0/field", "Alt tür, alt tür alanının (“Tür”) kendisini değiştiremez.")
    bad("aynı alanın iki kuralı", {"field": "Tür", "types": [{"code": "sanayi", "fields": [
        {"field": "Not", "required": True}, {"field": "Not", "required": True}]}]},
        "types/0/fields/1/field", "“Not” alanı “sanayi” alt türünde iki kez var.")
    bad("listesiz alanın değerleri", {"field": "Tür", "types": [{"code": "sanayi", "fields": [
        {"field": "kat-sayisi", "values": ["1"]}]}]}, "types/0/fields/0/values",
        "“Kat” alanının değer listesi yok; alt tür yalnız listeli alanın değerlerini daraltır.")
    bad("hiç değer bırakılmamış", {"field": "Tür", "types": [{"code": "sanayi", "fields": [{"field": "Malzeme", "values": []}]}]},
        "types/0/fields/0/values", "“Malzeme” alanına “sanayi” alt türünde hiçbir değer bırakılmamış; en az bir değer seçin.")
    bad("listede olmayan değer", {"field": "Tür", "types": [{"code": "sanayi", "fields": [
        {"field": "Malzeme", "values": ["beton", "cam"]}]}]}, "types/0/fields/0/values/1",
        "“cam”, “Malzeme” alanının listesinde yok.")
    bad("iki kez aynı değer", {"field": "Tür", "types": [{"code": "sanayi", "fields": [
        {"field": "Malzeme", "values": ["beton", "beton"]}]}]}, "types/0/fields/0/values/1", "“beton” değeri iki kez var.")
    bad("alt türün sunmadığı varsayılan", {"field": "Tür", "types": [{"code": "sanayi", "fields": [
        {"field": "Malzeme", "values": ["beton", "celik"], "default": "ahsap"}]}]}, "types/0/fields/0/default",
        "“Malzeme” alanının varsayılanı “sanayi” alt türünün değerlerinden değil.")
    bad("boş varsayılan", {"field": "Tür", "types": [{"code": "sanayi", "fields": [{"field": "Not", "default": ""}]}]},
        "types/0/fields/0/default", "“Not” alanının “sanayi” alt türündeki varsayılanı alanın yazımıyla verilmeli.")
    bad("ikinci alt türün sorunu yeriyle", {"field": "Tür", "types": [{"code": "konut"}, {"code": "depo", "fields": [
        {"field": "Malzeme", "values": ["tas", "tas"]}]}]}, "types/1/fields/0/values/1", "“tas” değeri iki kez var.")
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.layers.subtypes", "commandVersion": 1,
            "title": "Alt türler",
            "note": "ADR 0422 §1, §3: katmanın alt türleri ve kuralları. Written by scripts/fixtures/subtypes_cases.py "
                    "without KentOS code; do not edit by hand.",
            "setup": setup(False), "cases": cases}


# ---- cad.entities.set ----

RECORDS = {
    1: {"Tür": "konut", "Malzeme": "ahsap"},
    2: {"Tür": "sanayi", "Malzeme": "beton", "Not": "Fabrika"},
    3: {"Malzeme": "tas"},
    4: {"Tür": "depo", "Malzeme": "ahsap"},
    5: {},
}
ENTITIES = [{"kind": "polygon", "id": i, "layerId": "bina", "attrs": a, "pts": square(20 * i)} for i, a in RECORDS.items()]


def set_file():
    cases = []

    def step(uids, attrs, now=None, undo=False):
        now = copy.deepcopy(now or RECORDS)
        after_all = copy.deepcopy(now)
        warnings = []
        try:
            for u in uids:
                after = {k: v for k, v in now[u].items()}
                for k, v in attrs.items():
                    if v is None:
                        after.pop(k, None)
                    else:
                        f = next((x for x in FIELDS if x["name"] == k), None)
                        after[k] = v.strip() if f and f["kind"] == "text" and not holds_many(f) else v
                if after == now[u]:
                    continue
                w = fit(FIELDS, SUBTYPES, now[u], after)
                if w and not warnings:
                    warnings.append(w)
                after_all[u] = after
        except Refused as r:
            return {"op": "execute", "input": {"uids": [f"$uidOf:{u}" for u in uids], "attrs": attrs, "operation": "attributes"},
                    "result": {"status": "failed", "error": r.error},
                    "expect": {"ids": list(RECORDS), "fields": {str(k): {"attrs": v} for k, v in now.items()},
                               "canUndo": undo, "canRedo": False, "revision": "same", **({} if undo else {"dirty": False})}}, now
        changed = [u for u in uids if after_all[u] != now[u]]
        out = {"changed": [f"$uidOf:{u}" for u in changed], "ids": changed, "revision": "$current"}
        expect = {"ids": list(RECORDS), "fields": {str(k): {"attrs": v} for k, v in after_all.items()},
                  "canUndo": bool(changed) or undo, "canRedo": False}
        expect.update({"dirty": True, "revision": "changed"} if changed else {"revision": "same"})
        return {"op": "execute", "input": {"uids": [f"$uidOf:{u}" for u in uids], "attrs": attrs, "operation": "attributes"},
                "result": {"status": "completed", "output": out, "warnings": warnings}, "expect": expect}, after_all

    def case(name, uids, attrs):
        s, _ = step(uids, attrs)
        cases.append({"name": name, "steps": [s]})

    case("sanayi kaydına alt türün sunduğu değer", [2], {"Malzeme": "celik"})
    case("sanayi kaydına alt türün sunmadığı değer reddedilir", [2], {"Malzeme": "ahsap"})
    case("konut kaydı listenin her değerini alır", [1], {"Malzeme": "tas"})
    case("alt türü olmayan kayıt her değeri alır", [3], {"Malzeme": "ahsap"})
    case("konuttan sanayiye: kalan değerler uyarıyla söylenir", [1], {"Tür": "sanayi"})
    case("konuttan sanayiye, değerleriyle: uyarı yok", [1], {"Tür": "sanayi", "Malzeme": "beton", "Not": "Atölye"})
    case("aynı değeri yeniden yazmak reddedilmez; uyarılır", [1], {"Tür": "sanayi", "Malzeme": "ahsap", "Not": "Atölye"})
    case("sanayide zorunlu not boşaltılamaz", [2], {"Not": None})
    case("sanayide zorunlu malzeme boşaltılamaz", [2], {"Malzeme": None})
    case("iki kayda izinli değer", [1, 2], {"Malzeme": "beton"})
    case("iki kayıttan birine izinsiz değer: hiçbiri yazılmaz", [1, 2], {"Malzeme": "ahsap"})
    case("sanayiden konuta: daraltma kalkar", [2], {"Tür": "konut", "Malzeme": "ahsap"})
    case("depo kaydında başka alan değişir: uymayan malzeme uyarılır", [4], {"Kat": "3"})
    case("depoya taş", [4], {"Malzeme": "tas"})
    case("depoya çelik reddedilir", [4], {"Malzeme": "celik"})
    case("boşluklu kod tek biçime çevrilir ve alt türü bulunur", [1], {"Tür": " sanayi ", "Malzeme": "celik", "Not": "x"})
    case("türü değişen kayda izinsiz yeni değer reddedilir", [1], {"Tür": "sanayi", "Malzeme": "tas"})
    case("iki kayıt sanayiye: tek uyarı, ilkinin", [1, 3], {"Tür": "sanayi"})
    case("alt tür alanı boşaltılınca daraltma kalkar", [2], {"Tür": None, "Malzeme": "ahsap"})
    case("değişmeyen kayıt yazılmaz", [2], {"Malzeme": "beton"})
    one, after = step([1], {"Tür": "sanayi"})
    two, _ = step([1], {"Not": "Depo yanında"}, now=after, undo=True)
    cases.append({"name": "uyarıdan sonra eksik not yazılınca yalnız malzeme kalır", "steps": [one, two]})
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.entities.set", "commandVersion": 1,
            "title": "Öznitelikler ve alt türler",
            "note": "ADR 0422 §2: kaydın alt türüne uymayan yeni değer reddedilir, kalan uyarılır. Written by "
                    "scripts/fixtures/subtypes_cases.py without KentOS code; do not edit by hand.",
            "setup": setup(True, ENTITIES), "cases": cases}


# ---- cad.polygon.create ----

def create_file():
    cases = []

    def made(name, layer_id, attrs=None):
        fields = FABRIKA_FIELDS if layer_id == "fabrika" else FIELDS
        inp = {"layerId": layer_id, "pts": square(0)}
        given = {}
        if attrs:
            inp["attrs"] = attrs
            for k, v in attrs.items():
                f = next((x for x in fields if x["name"] == k), None)
                v = v.strip() if f and f["kind"] == "text" and not holds_many(f) else v
                if v:
                    given[k] = v
        out = new_defaults(fields, SUBTYPES, given)
        try:
            fit(fields, SUBTYPES, None, out)
        except Refused as r:
            cases.append({"name": name, "steps": [{"op": "execute", "input": inp,
                                                   "result": {"status": "failed", "error": r.error},
                                                   "expect": {"ids": [], "canUndo": False, "dirty": False, "revision": "same"}}]})
            return
        cases.append({"name": name, "steps": [{"op": "execute", "input": inp, "result": {
            "status": "completed", "output": {"uid": "$uid", "id": 1, "revision": "$current"}, "warnings": []},
            "expect": {"ids": [1], "fields": {"1": {"attrs": out}}, "canUndo": True, "dirty": True, "revision": "changed"}}]})

    made("değersiz alan: alanın varsayılanı", "bina")
    made("sanayi: alt türün varsayılanı alanınkinin yerine", "bina", {"Tür": "sanayi"})
    made("konut: alt türün kat varsayılanı ve alanın malzemesi", "bina", {"Tür": "konut"})
    made("sanayi, verilen izinli değer", "bina", {"Tür": "sanayi", "Malzeme": "beton"})
    made("sanayi, verilen izinsiz değer reddedilir", "bina", {"Tür": "sanayi", "Malzeme": "ahsap"})
    made("depo: alanın varsayılanını alt tür sunmuyor, yazılmaz", "bina", {"Tür": "depo"})
    made("depo, taş", "bina", {"Tür": "depo", "Malzeme": "tas"})
    made("depo, ahşap reddedilir", "bina", {"Tür": "depo", "Malzeme": "ahsap"})
    made("sanayi: yeni kayıtta zorunlu not reddedilmez", "bina", {"Tür": "sanayi", "Kat": "4"})
    made("sanayi, notuyla", "bina", {"Tür": "sanayi", "Not": "Kalıp atölyesi"})
    made("konut, verilen kat varsayılanın önünde", "bina", {"Tür": "konut", "Kat": "5"})
    made("konut, taş", "bina", {"Tür": "konut", "Malzeme": "tas"})
    made("alt türü olmayan değer: daraltma yok", "bina", {"Malzeme": "celik"})
    made("boşluklu kod: alt türü bulunur", "bina", {"Tür": " sanayi "})
    made("alt tür alanının varsayılanı alt türü verir", "fabrika")
    made("alt tür alanının varsayılanı: izinsiz değer reddedilir", "fabrika", {"Malzeme": "ahsap"})
    made("alt tür alanının varsayılanı verilen türün arkasında kalır", "fabrika", {"Tür": "konut"})
    made("fabrikada depo", "fabrika", {"Tür": "depo"})
    made("fabrikada depo, beton", "fabrika", {"Tür": "depo", "Malzeme": "beton"})
    made("çok değerli alan alt türden etkilenmez", "bina", {"Tür": "sanayi", "Etiket": "a; b"})
    made("depo, çelik reddedilir", "bina", {"Tür": "depo", "Malzeme": "celik"})
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.polygon.create", "commandVersion": 1,
            "title": "Alan çiz ve alt türler",
            "note": "ADR 0422 §2: yeni kayıt alt türünün varsayılanlarını alır, alt türün sunmadığı değer reddedilir. Written "
                    "by scripts/fixtures/subtypes_cases.py without KentOS code; do not edit by hand.",
            "setup": setup(True), "cases": cases}


# ---- Forms over a layer with subtypes (§2), the form twin of forms_cases.py told of them ----

def form_twin():
    sys.path.insert(0, str(ROOT / "scripts/fixtures"))
    import forms_cases as fc  # noqa: E402  (the form twin, written from the ADRs without KentOS code)

    class SubForm(fc.Form):
        """`fc.Form` over a layer with subtypes: a record's kind narrows the choices, gives a new record its
        defaults where the form gives none, makes fields required, and a value it does not offer written
        anew (or left when the kind changed) is the issue `form_value_not_in_subtype`."""

        def __init__(self, body, fields, subtypes, variables=()):
            super().__init__(body, fields, variables)
            self.subtypes = subtypes

        def kind(self, attrs):
            return sub_of(self.fields, self.subtypes, attrs)

        def states(self, attrs, obj=None):
            out = super().states(attrs, obj)
            t = self.kind(attrs)
            for (x, _), state in zip(self.nodes, out):
                if x["type"] == "field" and state["shown"] and t and seen_required(self.layer(x), t):
                    state["required"] = True
            return out

        def choices(self, x, attrs, obj=None):
            if "lookup" in x:
                return super().choices(x, attrs, obj)
            f = self.layer(x)
            t = self.kind(attrs)
            r = rule_of(t, f) if t else None
            narrows = r is not None and r.get("values") is not None
            if "choiceFilter" not in x and not narrows:
                return None
            base = super().choices(x, attrs, obj) if "choiceFilter" in x else [
                c["code"] for c in f.get("values", []) if not c.get("retired")]
            held = codes_of(f, attrs[f["name"]]) if f["name"] in attrs else []
            if "choiceFilter" not in x:
                base = [c["code"] for c in f.get("values", []) if not c.get("retired") or c["code"] in held]
            return [c for c in base if offers(f, t, c) or c in held]

        def layer_defaults(self, out, skip):
            """docs/adr/0752: as a drawing command fills a new record (`new_defaults`), but the fields the form gives
            a default of its own: the subtype field by its own default, then each by its subtype's, else its own,
            when the subtype offers it."""
            sorter = next(f for f in self.fields if key(f) == self.subtypes["field"])
            if key(sorter) not in skip and sorter.get("default") and not out.get(sorter["name"], "").strip():
                out[sorter["name"]] = sorter["default"]
            t = self.kind(out)
            for f in self.fields:
                if key(f) in skip or out.get(f["name"], "").strip():
                    continue
                d = seen_default(f, t)
                if not d:
                    continue
                r = rule_of(t, f) if t else None
                narrowed = bool(f.get("values")) and r is not None and r.get("values") is not None
                if narrowed and not all(offers(f, t, c) for c in codes_of(f, d)):
                    continue
                out[f["name"]] = d

        def retired(self, changed, before, after):
            issues = super().retired(changed, before, after)
            t = self.kind(after)
            if t is None:
                return issues
            sorter = next(f for f in self.fields if key(f) == self.subtypes["field"])
            kind_changed = before.get(sorter["name"]) != after.get(sorter["name"])
            for f in self.fields:
                r = rule_of(t, f)
                if not r or r.get("values") is None or f["name"] not in after:
                    continue
                if not any(c not in r["values"] for c in codes_of(f, after[f["name"]])):
                    continue
                if kind_changed or key(f) in changed:
                    issues.append(fc.issue("form_value_not_in_subtype", True, self.item_of(key(f)), key(f)))
            return issues

    return fc, SubForm


FORM_BODY = {"revision": 1, "items": [
    {"type": "field", "id": "tur", "field": "Tür"},
    {"type": "field", "id": "malzeme", "field": "Malzeme"},
    {"type": "field", "id": "kat", "field": "kat-sayisi"},
    {"type": "field", "id": "not", "field": "Not"},
]}
# docs/adr/0426: a tab of its own for the industrial kind, written as the designer's Alt türlerde göster does.
TABBED_BODY = {"revision": 1, "items": [
    {"type": "tab", "id": "genel", "label": "Genel", "items": [
        {"type": "field", "id": "tur", "field": "Tür"},
        {"type": "field", "id": "malzeme", "field": "Malzeme"},
    ]},
    {"type": "tab", "id": "sanayi", "label": "Sanayi", "visibleWhen": "[Tür] içinde ('sanayi', 'depo')", "items": [
        {"type": "field", "id": "kat", "field": "kat-sayisi"},
        {"type": "field", "id": "not", "field": "Not"},
    ]},
]}
FILTERED_BODY = {"revision": 1, "items": [
    {"type": "field", "id": "tur", "field": "Tür"},
    {"type": "field", "id": "malzeme", "field": "Malzeme", "choiceFilter": "@secenek <> 'celik'"},
    {"type": "field", "id": "kat", "field": "kat-sayisi", "default": "9"},
    {"type": "field", "id": "not", "field": "Not"},
]}


def validate_file():
    fc, SubForm = form_twin()
    form = {"id": "bina", "name": "Bina", "layer": "bina", "draft": FORM_BODY,
            "releases": [{"version": 1, "digest": fc.digest(FORM_BODY), "body": FORM_BODY}]}
    filtered = {"id": "suzgec", "name": "Süzgeçli", "layer": "bina", "draft": FILTERED_BODY,
                "releases": [{"version": 1, "digest": fc.digest(FILTERED_BODY), "body": FILTERED_BODY}]}
    tabbed = {"id": "sekmeli", "name": "Sekmeli", "layer": "bina", "draft": TABBED_BODY,
              "releases": [{"version": 1, "digest": fc.digest(TABBED_BODY), "body": TABBED_BODY}]}
    S = setup(True, ENTITIES)
    S["settings"] = {**SETTINGS, "forms": [form, filtered, tabbed]}
    cases = []

    def run(name, inp, body=FORM_BODY, slot=None):
        before = RECORDS[slot] if slot else {}
        f = SubForm(body, FIELDS, SUBTYPES)
        after, found = f.assess(dict(before), fc.utf8_sorted(dict(inp.get("values", {}))), {}, bool(inp.get("defaults")))
        out = {"issues": fc.shown_issues(found), "valid": not fc.errors(found), "items": f.described(after, {}),
               "version": 1, "attrs": after}
        fc.with_requires(out, body)
        if slot:
            inp = {**inp, "uid": f"$uidOf:{slot}"}
        else:
            inp = {**inp}
        cases.append({"name": name, "steps": [{"op": "execute", "input": inp,
                                               "result": {"status": "completed", "output": out, "warnings": []},
                                               "expect": {"ids": list(RECORDS), "canUndo": False, "canRedo": False,
                                                          "dirty": False, "revision": "same"}}]})

    run("sanayi kaydı: malzeme seçenekleri daraltılır, not zorunlu", {"form": "bina"}, slot=2)
    run("konut kaydı: daraltma yok", {"form": "bina"}, slot=1)
    run("alt türü olmayan kayıt", {"form": "bina"}, slot=3)
    run("depo kaydı: tuttuğu ahşap seçeneklerde kalır, sorun değil", {"form": "bina"}, slot=4)
    run("depo kaydına çelik: alt türde yok", {"form": "bina", "values": {"Malzeme": "celik"}}, slot=4)
    run("depo kaydına taş", {"form": "bina", "values": {"Malzeme": "tas"}}, slot=4)
    run("konuttan sanayiye: kalan ahşap sorun, not zorunlu", {"form": "bina", "values": {"Tür": "sanayi"}}, slot=1)
    run("konuttan sanayiye, değerleriyle: sorunsuz",
        {"form": "bina", "values": {"Tür": "sanayi", "Malzeme": "celik", "Not": "Atölye"}}, slot=1)
    run("sanayiden konuta: daraltma ve zorunluluk kalkar", {"form": "bina", "values": {"Tür": "konut"}}, slot=2)
    run("sanayinin notu boşaltılır: zorunlu", {"form": "bina", "values": {"Not": ""}}, slot=2)
    run("yeni sanayi kaydı: alt türün varsayılanı", {"form": "bina", "values": {"Tür": "sanayi"}, "defaults": True}, slot=5)
    run("yeni konut kaydı: alt türün kat varsayılanı", {"form": "bina", "values": {"Tür": "konut"}, "defaults": True}, slot=5)
    run("yeni kayıt, türsüz: alanın kendi varsayılanı (ahşap)", {"form": "bina", "defaults": True}, slot=5)
    run("yeni depo kaydı: varsayılan yok, seçenekler daraltılır", {"form": "bina", "values": {"Tür": "depo"}, "defaults": True}, slot=5)
    run("yeni sanayi kaydı, verilen malzeme varsayılanın önünde",
        {"form": "bina", "values": {"Tür": "sanayi", "Malzeme": "beton"}, "defaults": True}, slot=5)
    run("süzgeç ve alt tür birlikte: sanayide yalnız beton", {"form": "suzgec"}, body=FILTERED_BODY, slot=2)
    run("süzgeç, konut: çelik dışındakiler", {"form": "suzgec"}, body=FILTERED_BODY, slot=1)
    run("formun varsayılanı alt türünkinin önünde", {"form": "suzgec", "values": {"Tür": "konut"}, "defaults": True},
        body=FILTERED_BODY, slot=5)
    run("alt türü olmayan kayda sanayi ve taş: taş sorun", {"form": "bina", "values": {"Tür": "sanayi"}}, slot=3)
    run("türü değişmeyen kayıtta başka alan: tuttuğu değer sorun değil", {"form": "bina", "values": {"kat-sayisi": "2"}},
        slot=4)
    run("boşluklu tür kodu", {"form": "bina", "values": {"Tür": " depo ", "Malzeme": "beton"}}, slot=1)
    run("sekmenin kuralı: konutta sanayi sekmesi gizli", {"form": "sekmeli"}, body=TABBED_BODY, slot=1)
    run("sekmenin kuralı: sanayide sanayi sekmesi ve zorunlu notu", {"form": "sekmeli"}, body=TABBED_BODY, slot=2)
    run("sekmenin kuralı: konutu depo yapınca sekme görünür", {"form": "sekmeli", "values": {"Tür": "depo"}},
        body=TABBED_BODY, slot=1)
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.forms.validate", "commandVersion": 1,
            "title": "Formu denetle ve alt türler",
            "note": "ADR 0422 §2: alt türü olan katmanın formu: seçenekler, zorunluluk, varsayılan ve alt türün sunmadığı "
                    "değer. Written by scripts/fixtures/subtypes_cases.py with the form twin of forms_cases.py, without "
                    "KentOS code; the issues' sentences are the core's ($any).",
            "setup": S, "cases": cases}


def outputs():
    return {OUT / "cad.layers.subtypes.json": subtypes_file(), OUT / "cad.entities.set.subtypes.json": set_file(),
            OUT / "cad.polygon.create.subtypes.json": create_file(),
            OUT / "cad.forms.validate.subtypes.json": validate_file()}


def main() -> int:
    stale = []
    for path, doc in outputs().items():
        text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
        if "--check" in sys.argv[1:]:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                stale.append(path)
        else:
            path.write_text(text, encoding="utf-8")
        print(f"{path.relative_to(ROOT)}: {len(doc['cases'])} durum")
    if stale:
        for p in stale:
            print(f"{p.relative_to(ROOT)} güncel değil: betiği --check olmadan çalıştırın ve farkı okuyun.", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
